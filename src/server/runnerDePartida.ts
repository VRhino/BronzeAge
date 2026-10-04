// RunnerDePartida (Docs/Arquitectura/7_Diseno_GameSession.md §4, §8.4): la pieza que faltaba entre
// `GameSession` (síncrona, sin E/S, sin cola — doc 7 §2) y un backend real. Aquí, y solo aquí, viven:
//
//  - La COLA SERIAL por partida que exige el doc 2 (principio 5): dos llamadas a `ejecutar`/`avanzarTick`
//    lanzadas sin esperar la primera se aplican en el orden en que llegaron, nunca intercaladas.
//  - El ciclo "aplicar -> anotar en el diario -> confirmar" (doc 12 §5.1, antes doc 7 §2(a) con un guardado
//    completo por comando): si la línea del diario no se escribe, el comando se descarta — `GameSession` vuelve
//    a como estaba antes de aplicarlo, no se queda a medias. La partida entera se guarda una vez por tick.
//  - El scheduler de ticks automáticos (opcional: nada obliga a usarlo, `avanzarTick()` sigue invocable a
//    mano igual que hoy).
//
// Lo que NO hace todavía, a propósito: difusión a clientes (necesita el protocolo de suscripciones de
// Fase C, que no existe) y nada de HTTP/WebSocket (es la capa de encima, no esta).
//
// Contrato ASÍNCRONO desde el principio (doc 7 §8.4), aunque hoy por dentro no espere nada más que la propia
// escritura a disco: migrar a cesión cooperativa del event loop o a un worker (si algún día hiciera falta,
// ver doc 7 §8.3) es, por diseño, un cambio DENTRO de este archivo — invisible para quien lo llama.
import type { Asentamiento, RegionId } from '../domain/types';
import { GameSession, type OpcionesSesion, type PartidaExportada, type ResultadoComando } from '../session/gameSession';
import { ACTOR_SISTEMA, type ActorId, type ManejadorComando } from '../session/comandos/tipos';
import { REGISTRO_DIARIO, type DatosDeDiario, type ParamsDeDiario, type TipoDiario } from '../session/comandos/registro';
import { eventosDesde, eventosRecortados, instanteDeTick, versionMasVieja, type EventoDominioConVersion, type GeometriaAsentamientos } from '../session/estado';
import { calcularPrecioReferencia } from '../engine/market';
import { computeTodasLasZonas, computeZonasFusionadasPorFaccion } from '../engine/zones';
import { trazadoParaAsentamiento } from '../engine/trazado';
import { produccionPorMinuto, type ProduccionItem } from '../engine/asentamientoQuery';
import { evaluarAscenso, type EvaluacionAscenso } from '../engine/ascenso';
import { PRECIO_BASE } from '../constants';
import type { AlmacenDeObjetos } from './almacen/almacenDeObjetos';
import { cargarPartida, guardarPartida } from './persistenciaPartida';
import { anexarEventos, claveDeEventos, leerEventos } from './eventosDePartida';
import { anexarAlDiario, vaciarDiario, type LineaDiario } from './diarioDePartida';
import { tecnologiasDe } from '../engine/tecnologia';

/**
 * Instrumentación de UNA partida (Fase E3). Números crudos, sin interpretar: quien los lee decide si 400 ms
 * de tick son mucho o poco. Deliberadamente plano y todo numérico salvo el `gameId` — es lo que hace que
 * sirva igual para una respuesta JSON que para un exportador de métricas futuro.
 */
export interface MetricasDePartida {
  gameId: string;
  /** Paso de integración interno del motor. Aquí SÍ (a diferencia del contrato de juego, del que la Fase D lo
   * retiró): una métrica de operación mide el motor, y el tick es su unidad real de trabajo. */
  tick: number;
  version: number;
  /** Entradas encoladas y aún sin resolver, incluida la que corre. > 0 sostenido = la cola no drena. */
  colaPendiente: number;
  ticksEjecutados: number;
  tickMsUltimo: number;
  tickMsMedio: number;
  tickMsMaximo: number;
  /** Ticks que ejecutó la última pasada del reloj, y el mayor visto. Con el mundo congelado durante las
   * caídas (2026-09-05) esto ya no mide un catch-up tras reinicio: mide la recuperación de la deriva del
   * temporizador, y en un servidor sano vale 1. */
  ultimaRafagaTicks: number;
  mayorRafagaTicks: number;
  /** Tiempo de mundo DESCARTADO desde que arrancó el proceso, en ticks: los atrasos que se dieron por
   * "el servidor no estaba sirviendo" en vez de ejecutarlos. Es la consecuencia observable de que el mundo no
   * avance durante las caídas — un valor alto tras un incidente es lo esperado; uno que crece con el servidor
   * sano significa que `MAX_TICKS_POR_PASADA` se quedó corto. */
  ticksOmitidos: number;
  relojDeMundoActivo: boolean;
}

export interface OpcionesRunner {
  /** Almacén donde viven el snapshot y el historial de eventos de esta partida (`server/almacen/`). */
  almacen: AlmacenDeObjetos;
  /** Reloj inyectado — igual que en toda `session/`, nunca se lee `Date.now()` sin pasar por aquí. Los tests
   * inyectan uno controlado; por defecto, el reloj real. */
  ahora?: () => string;
  /** Configuración del proceso con que corre la partida (`OpcionesSesion`). Se mantiene al reconstruirla. */
  sesion?: OpcionesSesion;
  /**
   * A quién avisar de los eventos de cada tick del RELOJ de mundo, una vez persistido (en `api.ts`, el hub de WebSocket).
   * Los comandos de jugador ya difunden los suyos desde sus rutas; el reloj no tenía quién lo hiciera y los clientes
   * conectados solo se enteraban de lo que pasaba en el mundo al volver a preguntar.
   */
  alEmitir?: (gameId: string, eventos: readonly EventoDominioConVersion[]) => void;
}

export class RunnerDePartida {
  private sesion: GameSession;
  private readonly almacen: AlmacenDeObjetos;
  private readonly ahora: () => string;
  private readonly opcionesSesion: OpcionesSesion;
  private readonly alEmitir?: OpcionesRunner['alEmitir'];
  /** Los eventos del último tick completo (tick + auto-comercio + NPC), para difundirlos tras persistir. */
  private eventosDelUltimoTick: EventoDominioConVersion[] = [];

  /**
   * Reloj de mundo (Fase D / D5, doc 10 §2–3), `null` si no está en marcha. `referenciaMs` es el instante de
   * PARED del último tick que este reloj dio por bueno; avanza en pasos de `intervaloMs` a medida que se
   * ejecutan ticks, nunca por acumulación de `setInterval` (así el jitter del temporizador no deriva). El
   * reloj de pared solo dice CUÁNTOS ticks faltan; el `instante` de cada uno lo deriva el motor del tick
   * (doc 10 §2), así que lo que se ejecuta es determinista.
   *
   * **El mundo NO avanza mientras el servidor está caído** (decisión del usuario, 2026-09-05): la referencia
   * se ancla a "ahora" al construir el runner, no al `guardadoEn` del snapshot. Reabrir una partida no
   * ejecuta ni un tick atrasado. Ver `sincronizarConReloj` para lo que eso implica y para el único caso que
   * sí se recupera.
   */
  private relojDeMundo: { intervaloMs: number; timer: ReturnType<typeof setInterval>; referenciaMs: number } | null = null;

  /**
   * Cuántos ticks atrasados acepta ejecutar de golpe una pasada de `sincronizarConReloj`. Por encima de eso,
   * el tiempo pendiente **se descarta** en vez de ejecutarse (ver `sincronizarConReloj`).
   *
   * **No es un tope de rendimiento, es dónde se traza la frontera** entre las dos cosas que producen un
   * atraso, que se parecen mucho vistas desde aquí:
   *
   *  - **Deriva del temporizador**, que SÍ hay que recuperar: `setInterval` no dispara exacto, y unas décimas
   *    por disparo se acumulan hasta valer un tick entero cada ~10 minutos. Si no se recuperaran, el mundo
   *    correría más lento que el tiempo real y "1 tick = 1 minuto" dejaría de ser cierto. Esto produce 2
   *    ticks de atraso como mucho.
   *  - **El proceso no estuvo sirviendo** (host suspendido, event loop bloqueado un minuto largo, salto de
   *    reloj por una corrección NTP), que NO hay que recuperar por la misma razón por la que no se recupera
   *    un reinicio. Esto produce decenas o cientos.
   *
   * 5 separa las dos con holgura por los dos lados. Es el único número elegido a ojo de todo esto; súbelo si
   * alguna vez se ven `ticksOmitidos` con el servidor sano.
   */
  private static readonly MAX_TICKS_POR_PASADA = 5;

  /**
   * Instrumentación de la partida (Fase E3). Vive AQUÍ y no en un colector global porque estos tres números
   * solo los conoce el runner: cuánta cola tiene pendiente, cuánto tarda su tick y cuántos ticks ejecutó la
   * última ráfaga de catch-up.
   *
   * La ráfaga se mide a propósito: la re-medición de escala del 2026-09-05 (doc 6 §1) concluyó que un tick
   * suelto ya no bloquea la cola de forma preocupante (~0,5 s a 100 asentamientos) pero **una ráfaga de
   * catch-up sí** — ponerse al día de una semana caída son ~79 minutos con la cola parada. Sin esta métrica,
   * eso solo se ve desde fuera como "el servidor no responde".
   */
  private readonly instrumentos = { pendientes: 0, ticks: 0, msTotal: 0, msUltimo: 0, msMax: 0, ultimaRafaga: 0, mayorRafaga: 0, omitidos: 0 };

  /**
   * Cadena de la cola serial. INVARIANTE: siempre es una promesa que RESUELVE (nunca rechaza) — cada
   * `encolar` la reemplaza por una versión saneada del trabajo que acaba de encadenar. Así un trabajo que
   * falla no "envenena" la cadena para los que vengan después; el rechazo real se lleva por separado en la
   * promesa que se devuelve al llamador de ESE trabajo.
   */
  private cola: Promise<void> = Promise.resolve();

  /**
   * Idempotencia de comandos (Fase C5, doc 4: "reconexión de cliente sin duplicar comandos"). Clave
   * `actor:idempotencyKey` -> la MISMA promesa que ya está en curso o ya resolvió para ese intento. Guardar
   * la promesa (no solo el resultado) cubre dos casos con el mismo mecanismo: un reintento mientras el
   * primero sigue en la cola (dedup en curso) y un reintento después de que ya resolvió (dedup en caché) —
   * ambos devuelven exactamente lo mismo, sin volver a tocar el estado ni la versión.
   *
   * Se borra la entrada si la operación termina en EXCEPCIÓN (fallo de persistencia): eso no se persistió,
   * así que un reintento legítimo debe poder intentarlo de nuevo, no quedarse pegado a un fallo pasado.
   */
  private readonly idempotencia = new Map<string, Promise<ResultadoComando<unknown>>>();
  private static readonly LIMITE_IDEMPOTENCIA = 500;

  /**
   * Caché de precios de referencia (a petición del usuario, tras la auditoría de doc 9: `calcularPrecioReferencia`
   * es una regla de entrada PRIVILEGIADA —suma el stock de TODOS los asentamientos del mundo—, así que un
   * cliente sin motor no puede calcularla; el servidor la calcula y el cliente solo lee el resultado.
   *
   * TTL perezoso, no un `setInterval`: se recalcula la primera vez que alguien lo PIDE después de que pasó un
   * minuto real desde el último cálculo, nunca antes. Con esto se consigue el mismo contrato observable que
   * "se actualiza cada minuto en el servidor" (el cliente nunca ve un valor de más de ~60 s) sin sumar un
   * temporizador de fondo por partida que limpiar en cada `app.close()` ni ruido en los 80 archivos de test
   * que crean un servidor — no hay ninguna diferencia visible entre "recalculado por un timer" y "recalculado
   * en la próxima lectura tras vencer el TTL" cuando nadie mira el valor entre medias.
   *
   * NO se persiste: es una vista DERIVADA de `asentamientos` (barata, O(n) por recurso), no una fuente de
   * verdad — perderla al reiniciar el proceso no pierde nada, se recalcula sola en la siguiente lectura.
   */
  private cachePrecios: { calculadoEnMs: number; precios: Record<string, number> } | null = null;
  private static readonly TTL_PRECIOS_MS = 60_000;

  /**
   * Caché de la geometría por frame (Fase C10, doc 9 T2b: `computeTodasLasZonas`/`computeZonasFusionadasPorFaccion`
   * miran TODOS los asentamientos —entrada privilegiada—, y `render()` las pedía en cada `mousemove` del
   * cliente de antes; ahora viajan precalculadas dentro de la proyección en vez de ser un endpoint).
   *
   * A diferencia de `cachePrecios`, sin TTL: no hay ninguna razón de diseño para que este valor "se sienta"
   * desactualizado un rato — es puro, el resultado correcto es siempre el de AHORA MISMO. Lo que evita
   * recalcular es una clave por IDENTIDAD de referencia de `asentamientos`: ese array es el mismo objeto
   * mientras ningún comando lo toque (`GameSessionState` se reconstruye por spread, así que un comando que no
   * muta asentamientos deja la referencia intacta), así que memorizar por `===` es tan preciso como un TTL de
   * cero milisegundos, sin inventar un reloj que vigilar.
   */
  private cacheGeometria: { sobre: readonly Asentamiento[]; valor: GeometriaAsentamientos } | null = null;

  /**
   * Última `version` cuyos eventos ya están en `<gameId>.eventos.jsonl` (`eventosDePartida.ts`). Para una
   * partida en disco lo calcula `cargarPartida` (el diario repasado puede haber anexado ya parte de los suyos);
   * para una nueva, es 0. Solo avanza cuando un `anexarEventos` termina bien — si falla, el próximo comando
   * reintenta ese tramo (los eventos siguen en memoria hasta un reinicio).
   */
  private versionEventosAnexados: number;

  private constructor(sesion: GameSession, opciones: OpcionesRunner, versionEventosAnexados = sesion.getState().version) {
    this.sesion = sesion;
    this.almacen = opciones.almacen;
    this.ahora = opciones.ahora ?? (() => new Date().toISOString());
    this.opcionesSesion = opciones.sesion ?? {};
    this.alEmitir = opciones.alEmitir;
    this.versionEventosAnexados = versionEventosAnexados;
  }

  static crear(gameId: string, config: { seed: number; region?: RegionId }, opciones: OpcionesRunner): RunnerDePartida {
    return new RunnerDePartida(GameSession.crear(gameId, config, opciones.sesion), opciones);
  }

  /** Como `crear`, pero persiste la partida antes de devolverla (Fase C12, doc 4: "un cliente externo no
   * puede descubrir a qué conectarse"). Sin esto, `listarPartidas` (que lee disco, no memoria) no vería una
   * partida recién creada hasta su primer comando o tick — y si el proceso muriera antes de que alguien
   * ejecutara uno, se perdería del todo pese a que la creación ya había respondido 201/200. Seguro fuera de
   * la cola serial: nadie más tiene todavía una referencia a este `runner`, no hay otro mutador con quien
   * pisarse. Usado por `cargarOCrear` (rama "no existe todavía") y `RegistroDePartidas.descartarYCrear`.
   *
   * Vacía también el diario y el historial de eventos: lo que haya bajo este `gameId` es de una partida
   * descartada, y sus líneas se repasarían (o sus eventos se leerían) sobre la nueva. */
  static async crearYPersistir(
    gameId: string,
    config: { seed: number; region?: RegionId },
    opciones: OpcionesRunner,
    persistencia: { forzar?: boolean } = {}
  ): Promise<RunnerDePartida> {
    const runner = RunnerDePartida.crear(gameId, config, opciones);
    await guardarPartida(opciones.almacen, runner.sesion, runner.ahora(), persistencia);
    await vaciarDiario(opciones.almacen, gameId);
    await opciones.almacen.escribir(claveDeEventos(gameId), '');
    return runner;
  }

  /** Cierra el hueco entre "servidor recién arrancado" y "partida en curso": si ya hay un snapshot para
   * `gameId`, lo carga (en continuidad de RNG, ver `persistenciaPartida.ts`); si no, crea una partida nueva
   * con `config` (y la persiste, ver `crearYPersistir`). `config` se ignora si se carga un snapshot
   * existente. */
  static async cargarOCrear(gameId: string, config: { seed: number; region?: RegionId }, opciones: OpcionesRunner): Promise<RunnerDePartida> {
    const existente = await cargarPartida(opciones.almacen, gameId, opciones.sesion);
    return existente
      ? new RunnerDePartida(existente.sesion, opciones, existente.versionEventosAnexados)
      : RunnerDePartida.crearYPersistir(gameId, config, opciones);
  }

  get gameId(): string {
    return this.sesion.gameId;
  }

  /**
   * Eventos con `version` mayor que `desde`, en orden cronológico. El estado solo guarda los últimos
   * (`MAX_EVENTOS_EN_MEMORIA`): si el cursor es anterior a lo que hay en memoria, el tramo viejo se lee del JSONL.
   */
  async eventosDesde(desde: number): Promise<EventoDominioConVersion[]> {
    const estado = this.sesion.getState();
    const masVieja = versionMasVieja(estado);
    if (!eventosRecortados(estado) || desde >= masVieja) return eventosDesde(estado, desde);
    const delDisco = (await leerEventos(this.almacen, this.gameId, masVieja - 1)).filter((e) => e.version > desde).reverse();
    return [...delDisco, ...eventosDesde(estado, desde)];
  }

  getState() {
    return this.sesion.getState();
  }

  /** Snapshot exportable de la partida (Fase C12: descarga administrativa). En memoria, siempre al día —
   * `exportarSimulacion` ya no necesita reconstruir un formato aparte (el v2 de `GameStore` quedó retirado
   * junto con `importarSimulacion`, doc 4): el snapshot real que se persiste ES el
   * formato de exportación. */
  exportar(): PartidaExportada {
    return this.sesion.exportar();
  }

  /** Precio de referencia por recurso, calculado en el servidor y cacheado con TTL de un minuto real — ver el
   * comentario de `cachePrecios`. Nunca lo calcula el cliente: necesitaría el almacén de todos los
   * asentamientos del mundo, no solo el propio. */
  preciosReferencia(): Record<string, number> {
    const ahoraMs = new Date(this.ahora()).getTime();
    if (!this.cachePrecios || ahoraMs - this.cachePrecios.calculadoEnMs >= RunnerDePartida.TTL_PRECIOS_MS) {
      const asentamientos = this.sesion.getState().asentamientos;
      const precios = Object.fromEntries(Object.keys(PRECIO_BASE).map((recurso) => [recurso, calcularPrecioReferencia(recurso, asentamientos)]));
      this.cachePrecios = { calculadoEnMs: ahoraMs, precios };
    }
    return this.cachePrecios.precios;
  }

  /** Zonas de influencia, su fusión por facción, y el trazado urbano de cada asentamiento — ver el comentario
   * de `cacheGeometria`. Nunca lo calcula un cliente: las zonas necesitan la posición de TODOS los
   * asentamientos del mundo, no solo el propio. */
  geometriaAsentamientos(): GeometriaAsentamientos {
    const asentamientos = this.sesion.getState().asentamientos;
    if (this.cacheGeometria?.sobre !== asentamientos) {
      const zonas = computeTodasLasZonas(asentamientos);
      const valor: GeometriaAsentamientos = {
        zonas,
        zonasFusionadas: computeZonasFusionadasPorFaccion(zonas, asentamientos),
        trazadoPorAsentamiento: Object.fromEntries(asentamientos.map((a) => [a.id, trazadoParaAsentamiento(a)])),
      };
      this.cacheGeometria = { sobre: asentamientos, valor };
    }
    return this.cacheGeometria.valor;
  }

  /** Producción por minuto de mundo de cada edificio productor/transformador de un asentamiento — la muestra
   * el cliente de jugador en la pantalla de asentamiento. No la calcula un cliente: `produccionPorMinuto`
   * necesita la fachada `Mapa` (bosques, stock de yacimientos) y el polígono de zona, entrada privilegiada.
   * Barata: solo lectura sobre estado ya en memoria, sin caché propia — se pide como mucho una vez por
   * proyección y solo para la plaza que el jugador pisa. */
  produccionDeAsentamiento(asentamientoId: string): ProduccionItem[] {
    const asentamiento = this.sesion.getState().asentamientos.find((a) => a.id === asentamientoId);
    if (!asentamiento) return [];
    const zona = this.geometriaAsentamientos().zonas.find((z) => z.asentamientoId === asentamientoId)?.poligono ?? [];
    const adoptadas = tecnologiasDe(this.sesion.getState().tecnologia, asentamiento.faccionId).adoptadas;
    return produccionPorMinuto(asentamiento, this.sesion.getMapa(), zona, adoptadas);
  }

  /** Evaluación de la subida de nivel de un asentamiento (`evaluarAscenso`, engine/ascenso.ts) — la muestra el
   * cliente de jugador en la pantalla de asentamiento. Mismo motivo que `produccionDeAsentamiento` para calcularla
   * aquí: la solvencia lee producción, y la producción necesita la fachada `Mapa`. Solo lectura, sin caché: se pide
   * como mucho una vez por proyección y solo para la plaza que el jugador pisa. */
  ascensoDeAsentamiento(asentamientoId: string): EvaluacionAscenso | undefined {
    const estado = this.sesion.getState();
    const asentamiento = estado.asentamientos.find((a) => a.id === asentamientoId);
    if (!asentamiento) return undefined;
    return evaluarAscenso(
      asentamiento,
      estado.asentamientos,
      estado.facciones,
      this.sesion.getMapa(),
      instanteDeTick(estado.tick),
      tecnologiasDe(estado.tecnologia, asentamiento.faccionId).adoptadas
    );
  }

  /**
   * Ejecuta un comando por su NOMBRE (`REGISTRO_DIARIO`: el nombre es lo que se anota en el diario) y espera su
   * turno en la cola. Rechaza con lo que lance el diario si la escritura falla — el comando en sí no se pierde
   * en silencio, pero tampoco queda aplicado sin estar anotado.
   *
   * `idempotencyKey` (Fase C5): si se pasa, un segundo `ejecutar` con la misma clave para el mismo `actor` —
   * ya esté el primero en curso o ya haya resuelto — devuelve el MISMO resultado sin volver a aplicar el
   * comando. Es lo que hace segura la reconexión de cliente: reintentar tras una respuesta perdida no duplica
   * la acción, aunque el reintento llegue antes de que el primer intento haya terminado.
   *
   * NO comprueba que `manejador`/`params` coincidan con los del primer uso de esa clave — confía en que el
   * cliente no reutiliza una `idempotencyKey` para dos comandos distintos. Validarlo exigiría comparar
   * `params` por igualdad profunda (`JSON.stringify` no sirve: el orden de claves de un objeto puede variar
   * entre dos llamadas equivalentes y daría falsos positivos), y no es lo que este mecanismo existe para
   * resolver — es protección contra la reconexión, no contra un cliente que genera mal sus claves.
   */
  ejecutar<T extends TipoDiario>(
    tipo: T,
    params: ParamsDeDiario<T>,
    actor?: ActorId,
    idempotencyKey?: string
  ): Promise<ResultadoComando<DatosDeDiario<T>>> {
    // Sin `momento`: `GameSession` lo deriva del tick (`instanteDeTick`, Fase D / doc 10). `this.ahora()`
    // —el reloj de pared— se reserva para lo que NO es estado de partida: `guardarPartida` (abajo), el TTL de
    // `preciosReferencia`, y el catch-up del reloj de mundo (`sincronizarConReloj`, D5).
    type R = DatosDeDiario<T>;
    const operacion = () => this.aplicarYAnotar((lineas) => this.aplicar<R>(lineas, tipo, params, actor), false);
    if (idempotencyKey === undefined) return this.encolar(operacion);

    const clave = `${actor ?? ''}:${idempotencyKey}`;
    const enCurso = this.idempotencia.get(clave);
    if (enCurso) return enCurso as Promise<ResultadoComando<R>>;

    const promesa = this.encolar(operacion);
    this.idempotencia.set(clave, promesa);
    if (this.idempotencia.size > RunnerDePartida.LIMITE_IDEMPOTENCIA) {
      const masAntigua = this.idempotencia.keys().next().value;
      if (masAntigua !== undefined) this.idempotencia.delete(masAntigua);
    }
    promesa.catch(() => this.idempotencia.delete(clave));
    return promesa;
  }

  /**
   * Avanza un tick, en la misma cola que los comandos de jugador — comparten la misma restricción (doc 7
   * §8.1: "el estado solo admite un mutador a la vez"), así que comparten la misma cola.
   *
   * Encadena tick → auto-comercio (apagado por defecto) → turno del NPC de gobernanza, mismo orden que tenía
   * `GameStore.avanzarTick` antes de que existiera este runner. Las tres líneas van al diario en UNA escritura,
   * y después se guarda la partida entera y se vacía el diario (doc 12 §5.1).
   */
  avanzarTick(): Promise<ResultadoComando<void>> {
    return this.encolar(() => this.aplicarYAnotar((lineas) => this.unTickCompleto(lineas), true));
  }

  /** Un tick "completo" tal y como lo entiende este runner: tick puro + auto-comercio + turno del NPC. Sin
   * encolar — lo llaman `avanzarTick` (una entrada de cola) y el reloj de mundo (`sincronizarConReloj`, también
   * una sola entrada por pasada). */
  private unTickCompleto(lineas: LineaDiario[]): ResultadoComando<void> {
    const t0 = performance.now();
    const resultado = this.aplicar<void>(lineas, 'avanzarTick', undefined, ACTOR_SISTEMA);
    if (!resultado.ok) return resultado;
    const trasAuto = this.aplicar<void>(lineas, 'avanzarAutoComercio', undefined, ACTOR_SISTEMA);
    const trasNpc = this.aplicar<void>(lineas, 'avanzarFaccionesNpc', undefined, ACTOR_SISTEMA);
    this.eventosDelUltimoTick = [...resultado.eventos, ...trasAuto.eventos, ...trasNpc.eventos];
    // Se cronometra el tick COMPLETO (puro + auto-comercio + turno NPC), que es la unidad que ocupa la cola,
    // no el `avanzarSimulacion` puro que mide `scripts/medicion-escala.ts`. Los dos números no son
    // comparables a ciegas, y es correcto: aquí interesa lo que bloquea a un jugador.
    const ms = performance.now() - t0;
    this.instrumentos.ticks++;
    this.instrumentos.msTotal += ms;
    this.instrumentos.msUltimo = ms;
    if (ms > this.instrumentos.msMax) this.instrumentos.msMax = ms;
    return resultado;
  }

  /**
   * Arranca el RELOJ DE MUNDO (Fase D / D5): mantiene `estado.tick` sincronizado con el tiempo real,
   * ejecutando un tick por cada `intervaloMs` de reloj de pared transcurrido. Con "mundo = tiempo real"
   * (doc 10 §2), `intervaloMs` = `SIMULACION.duracionTickMs` = 60 000: un tick por minuto real.
   *
   * **El mundo avanza solo mientras este reloj está en marcha** (decisión del usuario, 2026-09-05: "el mundo
   * no avanza mientras el servidor está caído"). De ahí que `referenciaMs` se ancle a **"ahora"** al
   * arrancar: da igual cuándo se guardó el snapshot, cuánto llevara el proceso levantado o cuántos ticks se
   * dieran a mano — al arrancar el reloj no se debe ni un tick. Reabrir una partida de hace un mes la
   * reanuda en el tick en el que se quedó.
   *
   * Es también la versión más simple de lo que había: la anterior anclaba al momento de CONSTRUIR el runner
   * y luego descontaba un intervalo por cada tick manual dado desde entonces, para que el catch-up saliera
   * bien. Sin catch-up entre reinicios, esa contabilidad no tiene nada que corregir.
   *
   * En el primer disparo —y tras cualquier hueco: proceso caído y reabierto, host dormido, GC largo—
   * ejecuta EN RÁFAGA los ticks adeudados (catch-up, doc 10 §2), por la misma cola serial que los comandos.
   * `setInterval` solo dispara la comprobación; cuántos ticks faltan lo decide siempre el reloj de pared
   * contra `referenciaMs`, no un contador que acumule el jitter del temporizador.
   */
  iniciarRelojDeMundo(intervaloMs: number): void {
    if (this.relojDeMundo) return; // ya en marcha: no duplicar el intervalo
    const timer = setInterval(() => void this.sincronizarConReloj(), intervaloMs);
    this.relojDeMundo = { intervaloMs, timer, referenciaMs: new Date(this.ahora()).getTime() };
  }

  detenerRelojDeMundo(): void {
    if (!this.relojDeMundo) return;
    clearInterval(this.relojDeMundo.timer);
    this.relojDeMundo = null;
  }

  /**
   * Ejecuta los ticks que el reloj de pared dice que se adeudan desde `referenciaMs`, y adelanta la
   * referencia EXACTAMENTE ese número de intervalos (nunca a "ahora": así un resto sub-intervalo no se
   * pierde). Adelantar la referencia ANTES de encolar la ráfaga hace que un segundo disparo del `setInterval`
   * durante una ráfaga larga vea 0 adeudados y no duplique trabajo.
   *
   * Toda la ráfaga es UNA sola entrada de la cola serial: un comando de jugador que llegue a mitad del
   * catch-up espera a que el mundo termine de ponerse al día (correcto — no se puede actuar "ahora" hasta
   * que el mundo esté en "ahora"), y `esperarColaVacia` cubre la ráfaga entera. Si el reloj se detiene a
   * mitad (`detenerRelojDeMundo`, apagado del proceso), la ráfaga para donde va.
   */
  private sincronizarConReloj(): Promise<void> {
    const reloj = this.relojDeMundo;
    if (!reloj) return Promise.resolve();
    const ahoraMs = new Date(this.ahora()).getTime();
    const adeudados = Math.floor((ahoraMs - reloj.referenciaMs) / reloj.intervaloMs);
    if (adeudados <= 0) return Promise.resolve();

    // Atraso grande = el proceso no estuvo sirviendo. Se DESCARTA el tiempo pendiente y se vuelve a anclar a
    // "ahora": el mundo se queda donde estaba, que es la decisión, y no hay ráfaga que ocupe la cola. Se
    // cuenta en `omitidos` porque es la consecuencia observable de esa decisión y quien opera debe verla —
    // si aparece con el servidor sano, el que está mal es el umbral.
    if (adeudados > RunnerDePartida.MAX_TICKS_POR_PASADA) {
      reloj.referenciaMs = ahoraMs;
      this.instrumentos.omitidos += adeudados;
      this.instrumentos.ultimaRafaga = 0;
      return Promise.resolve();
    }

    reloj.referenciaMs += adeudados * reloj.intervaloMs;
    this.instrumentos.ultimaRafaga = adeudados;
    if (adeudados > this.instrumentos.mayorRafaga) this.instrumentos.mayorRafaga = adeudados;
    return this.encolar(async () => {
      for (let i = 0; i < adeudados && this.relojDeMundo; i++) {
        const r = await this.aplicarYAnotar((lineas) => this.unTickCompleto(lineas), true);
        if (r.ok && this.alEmitir) this.alEmitir(this.gameId, this.eventosDelUltimoTick);
      }
    });
  }

  /**
   * Se resuelve cuando la cola queda vacía: todo lo encolado hasta este instante ha terminado, con éxito o
   * con error. `detenerRelojDeMundo` impide que se ENCOLE trabajo nuevo, pero no cancela el que ya estaba en
   * cola (un tick a medio persistir no se aborta) — esto es para esperar a que ese resto drene. Pensado
   * tanto para un apagado limpio del proceso como para pruebas que necesiten un punto determinista después
   * de parar el reloj.
   */
  async esperarColaVacia(): Promise<void> {
    await this.cola;
  }

  /**
   * Instantánea de instrumentación de esta partida (Fase E3). Copia, no la referencia interna: quien lee
   * métricas no debe poder tocarlas.
   *
   * `tickMsMedio` es la media desde que arrancó el proceso, no una ventana móvil: para "¿este servidor va
   * sobrado o justo?" la media larga es la respuesta honesta, y `tickMsMaximo` recoge el pico que una media
   * esconde.
   */
  metricas(): MetricasDePartida {
    const i = this.instrumentos;
    return {
      gameId: this.gameId,
      tick: this.sesion.getState().tick,
      version: this.sesion.getState().version,
      colaPendiente: i.pendientes,
      ticksEjecutados: i.ticks,
      tickMsUltimo: i.msUltimo,
      tickMsMedio: i.ticks === 0 ? 0 : i.msTotal / i.ticks,
      tickMsMaximo: i.msMax,
      ultimaRafagaTicks: i.ultimaRafaga,
      mayorRafagaTicks: i.mayorRafaga,
      ticksOmitidos: i.omitidos,
      relojDeMundoActivo: this.relojDeMundo !== null,
    };
  }

  /** ms de reloj de PARED entre ticks para esta partida, o `null` si su reloj de mundo está parado (solo
   * avanza con `POST .../tick`). Lo fija `INTERVALO_TICK_MS` del proceso, o el override que reciba
   * `RegistroDePartidas.descartarYCrear` al regenerar. La consola de administración lo muestra en la
   * pestaña "Mundo". */
  intervaloRelojDeMundoMs(): number | null {
    return this.relojDeMundo?.intervaloMs ?? null;
  }

  private encolar<T>(trabajo: () => Promise<T>): Promise<T> {
    // `pendientes` cuenta lo ENCOLADO y aún sin resolver, incluida la entrada en curso. Es la señal que
    // delata un tick largo bloqueando comandos: si crece y no baja, la cola no está drenando.
    this.instrumentos.pendientes++;
    const resultado = this.cola.then(trabajo);
    void resultado.then(
      () => this.instrumentos.pendientes--,
      () => this.instrumentos.pendientes--
    );
    this.cola = resultado.then(
      () => undefined,
      () => undefined
    );
    return resultado;
  }

  /** Aplica una mutación por nombre y, si subió la versión, apunta su línea en `lineas` (aún sin escribir). */
  private aplicar<R>(lineas: LineaDiario[], tipo: TipoDiario, params: unknown, actor: ActorId | undefined): ResultadoComando<R> {
    const a = actor ?? ACTOR_SISTEMA;
    const manejador = REGISTRO_DIARIO[tipo] as ManejadorComando<unknown, R>;
    const versionAntes = this.sesion.getState().version;
    const resultado = this.sesion.ejecutar(manejador, params, { actor: a });
    if (resultado.version !== versionAntes) lineas.push({ v: resultado.version, t: tipo, a, ...(params === undefined ? {} : { p: params }) });
    return resultado;
  }

  /**
   * El ciclo "aplicar -> anotar -> confirmar" (doc 12 §5.1). Lo aceptado es durable en cuanto su línea está en el
   * diario; si esa escritura falla, se reconstruye `GameSession` desde `previo` (barato: `exportar()` solo copia
   * referencias) y se lanza. Con `guardar` (los ticks), después se guarda la partida entera y se vacía el diario.
   */
  private async aplicarYAnotar<R>(operacion: (lineas: LineaDiario[]) => ResultadoComando<R>, guardar: boolean): Promise<ResultadoComando<R>> {
    const previo: PartidaExportada = this.sesion.exportar();
    const lineas: LineaDiario[] = [];
    const resultado = operacion(lineas);
    if (lineas.length === 0) return resultado; // rechazado o sin cambios: no hay nada que anotar

    try {
      await anexarAlDiario(this.almacen, this.gameId, lineas);
    } catch (err) {
      this.sesion = GameSession.importar(previo, this.opcionesSesion);
      throw err;
    }

    await this.anexarEventosNuevos();
    if (guardar) await this.guardarYVaciarDiario();
    return resultado;
  }

  /**
   * Guarda la partida entera y vacía el diario. Su fallo NO revierte nada: lo que hay en memoria ya está en el
   * diario, así que se grita y el diario sigue creciendo hasta que un guardado posterior salga bien. Un corte
   * entre las dos escrituras deja en el diario líneas que el guardado ya contiene; `repasarDiario` las salta.
   */
  private async guardarYVaciarDiario(): Promise<void> {
    try {
      await guardarPartida(this.almacen, this.sesion, this.ahora());
      await vaciarDiario(this.almacen, this.gameId);
    } catch (err) {
      console.error(`[diario] no se pudo guardar '${this.gameId}' (versión ${this.sesion.getState().version}); el diario la conserva:`, err);
    }
  }

  /** Guarda la partida y vacía el diario por la cola serial — para el apagado limpio (`RegistroDePartidas.cerrar`),
   * así un despliegue normal arranca sin nada que repasar. */
  guardar(): Promise<void> {
    return this.encolar(() => this.guardarYVaciarDiario());
  }

  /**
   * Anexa al JSONL de eventos (`eventosDePartida.ts`) lo emitido desde el último anexado. El corte se hace
   * por `version` y no por `ResultadoComando.eventos` porque un tick completo son hasta tres mutaciones (tick
   * + auto-comercio + turno del NPC) y solo la primera vuelve en el resultado — `eventosDesde` las recoge las
   * tres.
   *
   * Va DESPUÉS del diario, y su fallo NO revierte el comando: diario y snapshot son la fuente de verdad del
   * estado, este archivo es el historial derivado (mismo trato que `auditoria.ts`). Si el append falla se
   * grita y el cursor NO avanza, así que el próximo comando reintenta ese tramo — los eventos siguen en
   * memoria hasta un reinicio. Va por comando y no por tick porque la memoria solo guarda los últimos
   * `MAX_EVENTOS_EN_MEMORIA`: un tick con muchos comandos podría recortarlos antes de llegar aquí.
   */
  private async anexarEventosNuevos(): Promise<void> {
    const estado = this.sesion.getState();
    if (estado.version === this.versionEventosAnexados) return;
    try {
      await anexarEventos(this.almacen, this.gameId, eventosDesde(estado, this.versionEventosAnexados));
      this.versionEventosAnexados = estado.version;
    } catch (err) {
      console.error(
        `[eventos] no se pudieron anexar los de '${this.gameId}' (versiones ${this.versionEventosAnexados + 1}–${estado.version}):`,
        err
      );
    }
  }
}
