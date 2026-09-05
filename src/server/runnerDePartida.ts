// RunnerDePartida (Docs/Arquitectura/7_Diseno_GameSession.md §4, §8.4): la pieza que faltaba entre
// `GameSession` (síncrona, sin E/S, sin cola — doc 7 §2) y un backend real. Aquí, y solo aquí, viven:
//
//  - La COLA SERIAL por partida que exige el doc 2 (principio 5): dos llamadas a `ejecutar`/`avanzarTick`
//    lanzadas sin esperar la primera se aplican en el orden en que llegaron, nunca intercaladas.
//  - El ciclo "aplicar -> persistir -> confirmar" del doc 7 §2(a): si la escritura a disco falla, el comando
//    se descarta — `GameSession` vuelve a como estaba antes de aplicarlo, no se queda a medias.
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
import { GameSession, type PartidaExportada, type ResultadoComando } from '../session/gameSession';
import type { ActorId, ManejadorComando } from '../session/comandos/tipos';
import type { GeometriaAsentamientos } from '../session/estado';
import { calcularPrecioReferencia } from '../engine/market';
import { computeTodasLasZonas, computeZonasFusionadasPorFaccion } from '../engine/zones';
import { trazadoParaAsentamiento } from '../engine/trazado';
import { PRECIO_BASE } from '../constants';
import { cargarPartida, guardarPartida } from './persistenciaPartida';

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
  /** Ticks que ejecutó la última ráfaga de catch-up, y el mayor visto. Ver el comentario de `instrumentos`. */
  ultimaRafagaTicks: number;
  mayorRafagaTicks: number;
  relojDeMundoActivo: boolean;
}

export interface OpcionesRunner {
  /** Directorio donde vive el snapshot de esta partida (`persistenciaPartida.ts`). */
  directorio: string;
  /** Reloj inyectado — igual que en toda `session/`, nunca se lee `Date.now()` sin pasar por aquí. Los tests
   * inyectan uno controlado; por defecto, el reloj real. */
  ahora?: () => string;
}

export class RunnerDePartida {
  private sesion: GameSession;
  private readonly directorio: string;
  private readonly ahora: () => string;

  /**
   * Reloj de mundo (Fase D / D5, doc 10 §2–3), `null` si no está en marcha. `referenciaMs` es el instante de
   * PARED del último tick que este reloj dio por bueno — inicializado al `guardadoEn` del snapshot cargado
   * (para el catch-up tras reinicio) o a "ahora" para una partida nueva; avanza en pasos de `intervaloMs` a
   * medida que se ejecutan ticks, nunca por acumulación de `setInterval` (así el jitter del temporizador no
   * deriva). El reloj de pared solo dice CUÁNTOS ticks faltan; el `instante` de cada uno lo deriva el motor
   * del tick (doc 10 §2), así que la ráfaga de catch-up es determinista.
   */
  private relojDeMundo: { intervaloMs: number; timer: ReturnType<typeof setInterval>; referenciaMs: number } | null = null;

  /** Instante de PARED (ms) del último guardado conocido al construir el runner — `guardadoEn` del snapshot
   * cargado, o "ahora" para una partida nueva. Junto a `tickAlConstruir` fija el punto de anclaje del reloj
   * de mundo: "en `referenciaRelojInicialMs` el mundo estaba en `tickAlConstruir`". */
  private readonly referenciaRelojInicialMs: number;
  private readonly tickAlConstruir: number;

  /** Tope de ticks que una sola pasada de `sincronizarConReloj` ejecuta en ráfaga. Acota la latencia de
   * arranque tras una caída larga (y el daño de un salto de reloj disparatado, p. ej. una corrección NTP);
   * lo que exceda se recupera en las pasadas siguientes del temporizador. 10 080 = una semana de mundo. */
  private static readonly MAX_TICKS_RAFAGA = 10_080;

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
  private readonly instrumentos = { pendientes: 0, ticks: 0, msTotal: 0, msUltimo: 0, msMax: 0, ultimaRafaga: 0, mayorRafaga: 0 };

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

  private constructor(sesion: GameSession, opciones: OpcionesRunner, guardadoEn?: string) {
    this.sesion = sesion;
    this.directorio = opciones.directorio;
    this.ahora = opciones.ahora ?? (() => new Date().toISOString());
    this.referenciaRelojInicialMs = guardadoEn !== undefined ? Date.parse(guardadoEn) : new Date(this.ahora()).getTime();
    this.tickAlConstruir = sesion.getState().tick;
  }

  static crear(gameId: string, config: { seed: number; region?: RegionId }, opciones: OpcionesRunner): RunnerDePartida {
    return new RunnerDePartida(GameSession.crear(gameId, config), opciones);
  }

  /** Como `crear`, pero persiste la partida antes de devolverla (Fase C12, doc 4: "un cliente externo no
   * puede descubrir a qué conectarse"). Sin esto, `listarPartidas` (que lee disco, no memoria) no vería una
   * partida recién creada hasta su primer comando o tick — y si el proceso muriera antes de que alguien
   * ejecutara uno, se perdería del todo pese a que la creación ya había respondido 201/200. Seguro fuera de
   * la cola serial: nadie más tiene todavía una referencia a este `runner`, no hay otro mutador con quien
   * pisarse. Usado por `cargarOCrear` (rama "no existe todavía") y `RegistroDePartidas.descartarYCrear`. */
  static async crearYPersistir(
    gameId: string,
    config: { seed: number; region?: RegionId },
    opciones: OpcionesRunner,
    persistencia: { forzar?: boolean } = {}
  ): Promise<RunnerDePartida> {
    const runner = RunnerDePartida.crear(gameId, config, opciones);
    await guardarPartida(opciones.directorio, runner.sesion, runner.ahora(), persistencia);
    return runner;
  }

  /** Cierra el hueco entre "servidor recién arrancado" y "partida en curso": si ya hay un snapshot para
   * `gameId`, lo carga (en continuidad de RNG, ver `persistenciaPartida.ts`); si no, crea una partida nueva
   * con `config` (y la persiste, ver `crearYPersistir`). `config` se ignora si se carga un snapshot
   * existente. */
  static async cargarOCrear(gameId: string, config: { seed: number; region?: RegionId }, opciones: OpcionesRunner): Promise<RunnerDePartida> {
    const existente = await cargarPartida(opciones.directorio, gameId);
    return existente
      ? new RunnerDePartida(existente.sesion, opciones, existente.guardadoEn)
      : RunnerDePartida.crearYPersistir(gameId, config, opciones);
  }

  get gameId(): string {
    return this.sesion.gameId;
  }

  getState() {
    return this.sesion.getState();
  }

  /** Snapshot exportable de la partida (Fase C12: descarga administrativa). En memoria, siempre al día —
   * `exportarSimulacion` ya no necesita reconstruir un formato aparte (el v2 de `GameStore` quedó retirado
   * junto con `importarSimulacion`, doc 4): el snapshot real que ya se persiste en cada comando ES el
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

  /**
   * Ejecuta un comando de jugador y espera su turno en la cola. Rechaza con lo que lance la persistencia si
   * la escritura falla — el comando en sí no se pierde en silencio, pero tampoco queda aplicado sin estar
   * guardado.
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
  ejecutar<P, R>(manejador: ManejadorComando<P, R>, params: P, actor?: ActorId, idempotencyKey?: string): Promise<ResultadoComando<R>> {
    // Sin `momento`: `GameSession` lo deriva del tick (`instanteDeTick`, Fase D / doc 10). `this.ahora()`
    // —el reloj de pared— se reserva para lo que NO es estado de partida: `guardarPartida` (abajo), el TTL de
    // `preciosReferencia`, y el catch-up del reloj de mundo (`sincronizarConReloj`, D5).
    const operacion = () => this.aplicarYPersistir((sesion) => sesion.ejecutar(manejador, params, { actor }));
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
   * `GameStore.avanzarTick` antes de que existiera este runner — es UN solo persist para las tres, no tres
   * comandos sueltos: si se guardaran por separado, un fallo de escritura a mitad podría dejar el tick
   * aplicado pero el turno NPC no, con la partida y el disco de acuerdo en un estado que nadie pidió.
   */
  avanzarTick(): Promise<ResultadoComando<void>> {
    return this.encolar(() => this.aplicarYPersistir((sesion) => this.unTickCompleto(sesion)));
  }

  /** Un tick "completo" tal y como lo entiende este runner: tick puro + auto-comercio + turno del NPC, un
   * solo persist para los tres. Sin encolar — lo llaman `avanzarTick` (una entrada de cola) y la ráfaga de
   * catch-up (`sincronizarConReloj`, también una sola entrada para toda la ráfaga). */
  private unTickCompleto(sesion: GameSession): ResultadoComando<void> {
    const t0 = performance.now();
    const resultado = sesion.avanzarTick();
    if (!resultado.ok) return resultado;
    sesion.avanzarAutoComercio();
    sesion.avanzarFaccionesNpc();
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
   * `referenciaMs` = "instante de pared en que el mundo llegó al tick ACTUAL". Se parte del anclaje del
   * constructor (`referenciaRelojInicialMs` en `tickAlConstruir`) más un intervalo por cada tick avanzado
   * desde entonces a mano (`POST .../tick`), para que arrancar el reloj después de unos ticks manuales no
   * los cuente dos veces ni pare/reanude re-ejecute la ráfaga.
   *
   * En el primer disparo —y tras cualquier hueco: proceso caído y reabierto, host dormido, GC largo—
   * ejecuta EN RÁFAGA los ticks adeudados (catch-up, doc 10 §2), por la misma cola serial que los comandos.
   * `setInterval` solo dispara la comprobación; cuántos ticks faltan lo decide siempre el reloj de pared
   * contra `referenciaMs`, no un contador que acumule el jitter del temporizador.
   */
  iniciarRelojDeMundo(intervaloMs: number): void {
    if (this.relojDeMundo) return; // ya en marcha: no duplicar el intervalo
    const ticksAvanzadosAMano = this.sesion.getState().tick - this.tickAlConstruir;
    const referenciaMs = this.referenciaRelojInicialMs + ticksAvanzadosAMano * intervaloMs;
    const timer = setInterval(() => void this.sincronizarConReloj(), intervaloMs);
    this.relojDeMundo = { intervaloMs, timer, referenciaMs };
    void this.sincronizarConReloj(); // catch-up inmediato, sin esperar al primer intervalo
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
    const adeudados = Math.min(Math.floor((ahoraMs - reloj.referenciaMs) / reloj.intervaloMs), RunnerDePartida.MAX_TICKS_RAFAGA);
    if (adeudados <= 0) return Promise.resolve();
    reloj.referenciaMs += adeudados * reloj.intervaloMs;
    this.instrumentos.ultimaRafaga = adeudados;
    if (adeudados > this.instrumentos.mayorRafaga) this.instrumentos.mayorRafaga = adeudados;
    return this.encolar(async () => {
      for (let i = 0; i < adeudados && this.relojDeMundo; i++) {
        await this.aplicarYPersistir((sesion) => this.unTickCompleto(sesion));
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
      relojDeMundoActivo: this.relojDeMundo !== null,
    };
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

  /** El ciclo "aplicar -> persistir -> confirmar" del doc 7 §2(a). `GameSession` ya adoptó el estado nuevo
   * dentro de `operacion` cuando esta función se entera de si hubo error — por eso, si la persistencia
   * falla, no basta con "no aplicar": hay que reconstruir `GameSession` desde el snapshot previo a la
   * operación. `previo` es barato de capturar (`exportar()` solo copia referencias, no clona, ver
   * `GameSession.exportar`), así que capturarlo en cada operación no es un coste real. */
  private async aplicarYPersistir<R>(operacion: (sesion: GameSession) => ResultadoComando<R>): Promise<ResultadoComando<R>> {
    const previo: PartidaExportada = this.sesion.exportar();
    const resultado = operacion(this.sesion);

    if (!resultado.ok) return resultado; // rechazado: GameSession no cambió nada, no hay nada que persistir

    try {
      await guardarPartida(this.directorio, this.sesion, this.ahora());
      return resultado;
    } catch (err) {
      this.sesion = GameSession.importar(previo);
      throw err;
    }
  }
}
