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
  private temporizador: ReturnType<typeof setInterval> | null = null;

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

  private constructor(sesion: GameSession, opciones: OpcionesRunner) {
    this.sesion = sesion;
    this.directorio = opciones.directorio;
    this.ahora = opciones.ahora ?? (() => new Date().toISOString());
  }

  static crear(gameId: string, config: { seed: number; region?: RegionId }, opciones: OpcionesRunner): RunnerDePartida {
    return new RunnerDePartida(GameSession.crear(gameId, config), opciones);
  }

  /** Cierra el hueco entre "servidor recién arrancado" y "partida en curso": si ya hay un snapshot para
   * `gameId`, lo carga (en continuidad de RNG, ver `persistenciaPartida.ts`); si no, crea una partida nueva
   * con `config`. `config` se ignora si se carga un snapshot existente. */
  static async cargarOCrear(gameId: string, config: { seed: number; region?: RegionId }, opciones: OpcionesRunner): Promise<RunnerDePartida> {
    const existente = await cargarPartida(opciones.directorio, gameId);
    return new RunnerDePartida(existente ?? GameSession.crear(gameId, config), opciones);
  }

  get gameId(): string {
    return this.sesion.gameId;
  }

  getState() {
    return this.sesion.getState();
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
    const operacion = () => this.aplicarYPersistir((sesion) => sesion.ejecutar(manejador, params, { momento: this.ahora(), actor }));
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
    return this.encolar(() =>
      this.aplicarYPersistir((sesion) => {
        const momento = this.ahora();
        const resultado = sesion.avanzarTick(momento);
        if (!resultado.ok) return resultado;
        sesion.avanzarAutoComercio(momento);
        sesion.avanzarFaccionesNpc(momento);
        return resultado;
      })
    );
  }

  /**
   * Programa `avanzarTick()` cada `intervaloMs`, a través de la misma cola serial que los comandos —
   * `setInterval` solo dispara la llamada, el orden real lo sigue decidiendo la cola. Un tick que tarde más
   * que `intervaloMs` no se solapa consigo mismo: la siguiente llamada simplemente espera su turno como
   * cualquier otro trabajo encolado.
   */
  iniciarTicksAutomaticos(intervaloMs: number): void {
    if (this.temporizador) return; // ya en marcha: no duplicar el intervalo
    this.temporizador = setInterval(() => {
      void this.avanzarTick();
    }, intervaloMs);
  }

  detenerTicksAutomaticos(): void {
    if (!this.temporizador) return;
    clearInterval(this.temporizador);
    this.temporizador = null;
  }

  /**
   * Se resuelve cuando la cola queda vacía: todo lo encolado hasta este instante ha terminado, con éxito o
   * con error. `detenerTicksAutomaticos` impide que se ENCOLE trabajo nuevo, pero no cancela el que ya
   * estaba en cola (un tick a medio persistir no se aborta) — esto es para esperar a que ese resto drene.
   * Pensado tanto para un apagado limpio del proceso como para pruebas que necesiten un punto determinista
   * después de parar el scheduler.
   */
  async esperarColaVacia(): Promise<void> {
    await this.cola;
  }

  private encolar<T>(trabajo: () => Promise<T>): Promise<T> {
    const resultado = this.cola.then(trabajo);
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
