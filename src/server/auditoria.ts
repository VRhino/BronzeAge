// Registro de auditoría de comandos (Fase E2, doc 2: "guardar cada comando/tick aceptado como registro
// auditable"; doc 4 §"Fase E — Operación persistente").
//
// QUÉ responde este archivo, y que `eventosDominio` no responde: `eventosDominio` narra lo que PASÓ en el
// mundo —una cosecha, un asedio, un título que cambia de manos— pero no quién lo pidió, ni desde qué sesión,
// ni qué se intentó y fue rechazado. Un jugador que insiste veinte veces en comandar tropas ajenas no deja
// hoy ni una línea: el 403 sale de la ruta y se pierde. Sin eso no hay moderación posible (E3), porque el
// abuso vive justo en los intentos que NO prosperan.
//
// DÓNDE vive (decisión del usuario, 2026-09-05): un archivo JSONL propio por partida, hermano del snapshot,
// NO un array dentro de `PartidaExportada`. Tres razones, en orden de peso:
//
//   1. `eventosDominio` ya enseñó el problema de meter un historial en el estado: crece sin techo, se
//      reescribe ENTERO en cada guardado y viaja entero en cada lectura (por eso C13 tuvo que añadirle un
//      cursor). Repetirlo con la auditoría sería tropezar dos veces con la misma piedra.
//   2. La retención es distinta. Un snapshot se sustituye; una auditoría se acumula y se poda por edad. Con
//      archivos separados, podar es borrar líneas viejas y no toca al estado de juego para nada.
//   3. Cero migración: no sube `FORMATO_SNAPSHOT_VERSION`, así que una partida guardada por una build
//      anterior se sigue cargando igual — simplemente no tiene auditoría anterior a este cambio.
//
// APPEND, no reescritura: a diferencia de `persistenciaPartida.ts`/`persistenciaIdentidad.ts` —que
// reescriben el archivo entero de forma atómica (`.tmp`+`rename`)— aquí se añade una línea al final. El
// truco del `.tmp` no vale: copiar un registro que crece sin límite para añadirle un renglón es
// exactamente lo que un log append-only existe para evitar. A cambio, el formato tiene que aguantar un
// archivo truncado por un corte de luz, y por eso es JSONL y no un array JSON: si la última línea queda a
// medias, se descarta ESA línea y las anteriores siguen siendo válidas. Un `[...]` truncado es ilegible
// entero.
import { appendFile, mkdir, readFile, rename, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

/** Versión del formato de CADA LÍNEA. Va en la línea, no en una cabecera de archivo: un log al que solo se
 * añade puede contener líneas escritas por builds distintas, y no habría dónde poner una cabecera que las
 * describiera a todas. Un lector filtra por esto en vez de adivinar por la forma del objeto. */
export const FORMATO_AUDITORIA_VERSION = 1;

/** Por qué se rechazó un comando. Catálogo cerrado a propósito: es el eje por el que un moderador filtra
 * ("enséñame los rechazos por autorización de este actor"), así que un string libre lo haría inútil. */
export type CausaRechazo =
  /** No pasó `MATRIZ_AUTORIZACION`: rol, facción, asentamiento o cargo insuficiente. El 403 de la ruta, y
   * la señal de moderación más importante — es el intento de hacer algo sobre lo que no es tuyo. */
  | 'autorizacion'
  /** `params` no cumplió el JSON Schema del comando (Fase C9), o el `tipo` no existe. Cliente con un bug o
   * sondeando la API; no es una acción de juego, pero sí una señal de operación. */
  | 'esquema'
  /** El comando se ejecutó y el DOMINIO dijo que no (recursos insuficientes, plaza ya fundada...). No es
   * abuso: es el juego funcionando. Se registra porque el rechazo de dominio es justo lo que explica una
   * queja de "no me deja construir". */
  | 'dominio'
  /** Falló la persistencia tras aplicar el comando (el 409). Ni permiso ni reglas: infraestructura. */
  | 'persistencia';

/**
 * Una línea del registro. Deliberadamente plana y sin anidar: un JSONL se lee con `grep` antes que con un
 * parser, y eso solo funciona si cada campo está en el primer nivel.
 */
export interface EntradaAuditoria {
  formatoVersion: number;
  /**
   * Reloj de PARED en ISO 8601: cuándo se PIDIÓ el comando en tiempo real. No es tiempo de mundo, y es a
   * propósito — una auditoría responde "qué pasó el martes a las 3", no "en qué tick". El tiempo de mundo va
   * aparte, en `instante`, para quien quiera cruzar el registro con los eventos de la partida.
   */
  momento: string;
  /** Instante de MUNDO tras el comando (ms). Ausente si el comando no llegó a ejecutarse (rechazo por
   * autorización o esquema): no hay un instante de mundo que atribuirle. */
  instante?: number;
  gameId: string;
  /**
   * Quién lo pidió: el `jugadorId` de la membresía, o el id derivado del usuario si actúa como
   * administrador. **Resuelto siempre en el servidor** desde la sesión, nunca de lo que el cuerpo afirme
   * (doc 2, principio 3) — una auditoría que confiara en un id enviado por el cliente no auditaría nada.
   */
  actor: string;
  /** `TipoComando`. `string` y no la unión: el registro tiene que poder anotar un `tipo` INVÁLIDO, que es
   * justo uno de los casos que interesa auditar. */
  comando: string;
  resultado: 'aceptado' | 'rechazado';
  causa?: CausaRechazo;
  /** Código estable de `codigosDeError.ts` para `causa: 'dominio'`; texto del motivo para las demás. */
  detalle?: string;
  /** Versión de la partida TRAS el comando. Junto con `instante` es lo que permite cruzar una línea de
   * auditoría con el evento de dominio que produjo (`EventoDominioConVersion.version`, Fase C13). */
  version?: number;
}

/** Nombre del archivo de una partida. Sufijo propio y no una extensión suelta para que un `readdir` del
 * directorio de datos distinga de un vistazo snapshots (`<gameId>.json`) de auditorías. */
export function rutaDeAuditoria(directorio: string, gameId: string): string {
  return join(directorio, `${gameId}.auditoria.jsonl`);
}

/**
 * Escritor del registro, uno por proceso.
 *
 * **Las escrituras se encadenan** (misma técnica que la cola serial de `RunnerDePartida` y que la cola del
 * repositorio de identidad): `appendFile` en paralelo sobre el mismo archivo puede entrelazar dos líneas y
 * dejar ambas ilegibles. Encadenar cuesta una promesa y elimina la clase de bug entera.
 *
 * **Un fallo de escritura NO tumba el comando.** Es una decisión con filo, así que conviene decir el porqué:
 * si el disco se llena, la alternativa —devolver un error al jugador— convierte un problema de operación en
 * una caída de juego para todo el mundo. Se elige seguir jugando y gritar por `stderr`, y se lleva la cuenta
 * en `fallos` para que las métricas de E3 puedan exponerla: una auditoría que falla en silencio y sin
 * contador sí sería inaceptable, porque nadie se enteraría de que lleva un mes sin registrar nada.
 */
export class RegistroDeAuditoria {
  private cola: Promise<unknown> = Promise.resolve();
  private _fallos = 0;
  /**
   * Recuento por resultado desde que arrancó el proceso (Fase E3, "tasa de errores"). Se lleva AQUÍ y no en
   * un colector aparte porque este ya es el punto por el que pasan todos los comandos, aceptados y
   * rechazados: un segundo contador en otro sitio solo podría divergir de este.
   *
   * Es un recuento, no una tasa: dividir entre qué ventana de tiempo es decisión de quien lo consume.
   */
  private readonly _recuento = { aceptados: 0, autorizacion: 0, esquema: 0, dominio: 0, persistencia: 0 };

  constructor(
    private readonly directorio: string,
    private readonly ahora: () => string = () => new Date().toISOString()
  ) {}

  /** Cuántas escrituras se han perdido desde que arrancó el proceso. Lo lee E3 (métricas). */
  get fallos(): number {
    return this._fallos;
  }

  /** Comandos por resultado desde el arranque. Copia: un lector de métricas no debe poder alterarlos. */
  get recuento(): RecuentoDeComandos {
    return { ...this._recuento };
  }

  /**
   * Añade una línea. NO se espera (`void`): auditar no debe añadir latencia a la respuesta del comando, y el
   * orden entre líneas ya lo garantiza la cola, no el llamador. Para los tests —y para el apagado limpio—
   * está `drenar()`.
   */
  registrar(entrada: Omit<EntradaAuditoria, 'formatoVersion' | 'momento'>): void {
    const linea: EntradaAuditoria = { formatoVersion: FORMATO_AUDITORIA_VERSION, momento: this.ahora(), ...entrada };
    // Se cuenta ANTES de encolar la escritura, y a propósito: el recuento describe lo que el servidor
    // DECIDIÓ, y esa decisión ya está tomada aunque el disco falle después. Contar solo lo que llegó a
    // escribirse mezclaría dos cosas distintas — cuántos comandos hubo y cuántos se pudieron registrar — y la
    // segunda ya la responde `fallos`.
    if (entrada.resultado === 'aceptado') this._recuento.aceptados++;
    else if (entrada.causa !== undefined) this._recuento[entrada.causa]++;
    this.cola = this.cola.then(async () => {
      try {
        await mkdir(this.directorio, { recursive: true });
        await appendFile(rutaDeAuditoria(this.directorio, entrada.gameId), `${JSON.stringify(linea)}\n`, 'utf-8');
      } catch (err) {
        this._fallos++;
        console.error(`[auditoria] no se pudo registrar ${entrada.comando} de ${entrada.actor}:`, err);
      }
    });
  }

  /** Espera a que la cola se vacíe. Lo llama el apagado del servidor y todo test que quiera leer lo escrito. */
  async drenar(): Promise<void> {
    await this.cola;
  }

  /**
   * Lee el registro de una partida. Drena antes: `registrar` escribe sin esperar, así que sin esto el
   * comando recién ejecutado podría no estar todavía en el archivo — justo el que se va a consultar.
   *
   * Existe como método, y no solo como la función suelta `leerAuditoria`, para que quien lee no tenga que
   * saber en qué directorio escribe el registro. Sin esto, la ruta HTTP tendría que sacar esa ruta de algún
   * otro sitio (el `directorio` privado del `RunnerDePartida`, por ejemplo) y las dos podrían divergir.
   */
  async leer(gameId: string, filtro?: FiltroAuditoria): Promise<{ entradas: EntradaAuditoria[]; corruptas: number }> {
    await this.drenar();
    return leerAuditoria(this.directorio, gameId, filtro);
  }
}

/** Comandos por resultado desde que arrancó el proceso. Las cuatro causas de rechazo son las de
 * `CausaRechazo`, una a una: agregarlas en un solo "rechazados" perdería justo la distinción que hace útil el
 * número (un pico de `autorizacion` es moderación; uno de `esquema` es un cliente roto; uno de `persistencia`
 * es el disco). */
export interface RecuentoDeComandos {
  aceptados: number;
  autorizacion: number;
  esquema: number;
  dominio: number;
  persistencia: number;
}

/** Ejes por los que se filtra el registro. Son los tres que una consulta de moderación cruza de verdad:
 * "los rechazos de este actor desde el lunes". */
export interface FiltroAuditoria {
  /** ISO 8601 de reloj de PARED; descarta lo anterior. */
  desde?: string;
  actor?: string;
  soloRechazos?: boolean;
}

/**
 * Lee el registro de una partida. Devuelve `[]` si no existe todavía — es "esta partida aún no tiene
 * auditoría", no un error.
 *
 * **Descarta las líneas ilegibles en vez de lanzar.** Es la contrapartida de elegir JSONL: una línea a
 * medias por un corte de luz no puede inutilizar el resto del registro, que es justo el escenario en que una
 * auditoría hace falta. Cuántas se descartaron sale en `corruptas`, para que quien lea sepa que el registro
 * tiene un agujero en vez de creer que está completo.
 */
export async function leerAuditoria(
  directorio: string,
  gameId: string,
  filtro?: FiltroAuditoria
): Promise<{ entradas: EntradaAuditoria[]; corruptas: number }> {
  let contenido: string;
  try {
    contenido = await readFile(rutaDeAuditoria(directorio, gameId), 'utf-8');
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === 'ENOENT') return { entradas: [], corruptas: 0 };
    throw err;
  }

  const entradas: EntradaAuditoria[] = [];
  let corruptas = 0;
  for (const linea of contenido.split('\n')) {
    if (linea.trim() === '') continue;
    let entrada: EntradaAuditoria;
    try {
      entrada = JSON.parse(linea) as EntradaAuditoria;
    } catch {
      corruptas++;
      continue;
    }
    if (filtro?.desde !== undefined && entrada.momento < filtro.desde) continue;
    if (filtro?.actor !== undefined && entrada.actor !== filtro.actor) continue;
    if (filtro?.soloRechazos === true && entrada.resultado !== 'rechazado') continue;
    entradas.push(entrada);
  }
  return { entradas, corruptas };
}

/**
 * Poda el registro de una partida: conserva solo las entradas con `momento >= limite` y devuelve cuántas se
 * descartaron.
 *
 * Reescribe con `.tmp`+`rename` —aquí SÍ, a diferencia del append— porque podar es por definición reescribir
 * el archivo entero, y hacerlo en el sitio dejaría el registro a medias si el proceso muere a mitad. Las
 * líneas corruptas se pierden en la poda: no se pueden fechar, así que no hay forma de decidir si entran o
 * salen, y conservarlas para siempre convertiría el agujero en permanente.
 */
export async function podarAuditoria(directorio: string, gameId: string, limite: string): Promise<number> {
  const { entradas, corruptas } = await leerAuditoria(directorio, gameId);
  const conservadas = entradas.filter((e) => e.momento >= limite);
  const descartadas = entradas.length - conservadas.length + corruptas;
  if (descartadas === 0) return 0;

  const ruta = rutaDeAuditoria(directorio, gameId);
  const temporal = `${ruta}.tmp`;
  await writeFile(temporal, conservadas.map((e) => `${JSON.stringify(e)}\n`).join(''), 'utf-8');
  await rename(temporal, ruta);
  return descartadas;
}
