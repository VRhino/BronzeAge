// Mantenimiento periódico del directorio de datos (Fase E2): respaldar cada partida, podar los respaldos
// viejos y podar la auditoría vencida. Es la pieza que convierte `respaldos.ts` y `auditoria.ts` de
// herramientas que alguien PODRÍA usar en operación que ocurre sola.
//
// **Opt-in, apagado por defecto** — mismo criterio que `INTERVALO_TICK_MS` y `ADMINISTRADORES`: sin
// configurarlo, ni un test ni un proceso de desarrollo escriben respaldos ni borran nada. Un default que
// borra archivos es exactamente la clase de default que nadie nota hasta que ya borró algo que hacía falta.
//
// Trabaja sobre TODAS las partidas (`listarPartidas`), no sobre el registro de las abiertas: así respalda
// también las que este proceso no tiene abiertas (mismo criterio que Fase C12 — una partida guardada antes
// de un reinicio sigue siendo descubrible) y no depende de que haya un servidor corriendo.
//
// Recibe DOS cosas: el `almacen` (para `listarPartidas` y `podarAuditoria`, que van por el puerto de
// persistencia) y el `directorio` de disco (para los respaldos, que son de archivos — ver la cabecera de
// `respaldos.ts`). Con un almacén remoto, los respaldos los hace el proveedor y basta con no activar el
// mantenimiento.
import type { AlmacenDeObjetos } from './almacen/almacenDeObjetos';
import { listarPartidas } from './persistenciaPartida';
import { podarAuditoria } from './auditoria';
import { podarRespaldos, respaldarPartida } from './respaldos';

export interface ConfiguracionMantenimiento {
  /** Cada cuánto corre la pasada. Es también el intervalo entre respaldos de una misma partida. */
  intervaloMs: number;
  /**
   * Respaldos a conservar por partida. Por CUENTA y no por edad: una partida inactiva se quedaría sin
   * ninguno justo cuando más difícil sería regenerarlo (ver `podarRespaldos`).
   */
  respaldosAConservar?: number;
  /** Días de auditoría que se conservan. La auditoría sí se poda por EDAD: su valor es responder "qué pasó
   * el martes", y una línea de hace un año no responde a nada que nadie vaya a preguntar. */
  retencionAuditoriaDias?: number;
}

const POR_DEFECTO = { respaldosAConservar: 7, retencionAuditoriaDias: 30 };

/** Qué hizo una pasada. Se devuelve en vez de solo registrarse por consola para que los tests puedan
 * comprobarlo, y para que las métricas de E3 tengan de dónde leerlo sin parsear un log. */
export interface ResumenPasada {
  partidas: number;
  respaldadas: number;
  respaldosBorrados: number;
  auditoriaBorrada: number;
  /** Partidas cuya pasada falló, con el porqué. Un fallo en una partida NO aborta las demás: el
   * mantenimiento es una tarea de fondo, y que una partida tenga el disco lleno no es razón para dejar al
   * resto del servidor sin respaldo. */
  fallos: { gameId: string; error: string }[];
}

const MS_POR_DIA = 24 * 60 * 60 * 1000;

export class TareaDeMantenimiento {
  private temporizador?: ReturnType<typeof setInterval>;
  private enCurso = false;

  constructor(
    private readonly almacen: AlmacenDeObjetos,
    private readonly directorio: string,
    private readonly config: ConfiguracionMantenimiento,
    private readonly ahora: () => string = () => new Date().toISOString()
  ) {}

  /**
   * Arranca la pasada periódica. `unref()` para que el temporizador NO mantenga vivo el proceso: mantenimiento
   * es trabajo de fondo, y un servidor que ya ha cerrado sus puertos no debe seguir en pie por esto.
   *
   * Llamar dos veces no duplica el temporizador (fuga que costó un test en el reloj de mundo de D5).
   */
  iniciar(): void {
    if (this.temporizador !== undefined) return;
    this.temporizador = setInterval(() => void this.ejecutarPasada(), this.config.intervaloMs);
    this.temporizador.unref?.();
  }

  detener(): void {
    if (this.temporizador === undefined) return;
    clearInterval(this.temporizador);
    this.temporizador = undefined;
  }

  /**
   * Una pasada completa. Pública para poder ejercitarla sin esperar al temporizador — así los tests prueban
   * lo que hace, no cuándo lo hace.
   *
   * **Reentrada protegida** (`enCurso`): con un directorio grande, una pasada puede durar más que el
   * intervalo. Sin esta guarda, dos pasadas simultáneas competirían por los mismos archivos — una podando lo
   * que la otra acaba de escribir.
   */
  async ejecutarPasada(): Promise<ResumenPasada> {
    const resumen: ResumenPasada = { partidas: 0, respaldadas: 0, respaldosBorrados: 0, auditoriaBorrada: 0, fallos: [] };
    if (this.enCurso) return resumen;
    this.enCurso = true;
    try {
      const momento = this.ahora();
      const limiteAuditoria = new Date(
        new Date(momento).getTime() - (this.config.retencionAuditoriaDias ?? POR_DEFECTO.retencionAuditoriaDias) * MS_POR_DIA
      ).toISOString();

      const partidas = await listarPartidas(this.almacen);
      resumen.partidas = partidas.length;
      for (const { gameId } of partidas) {
        try {
          if (await respaldarPartida(this.directorio, gameId, momento)) resumen.respaldadas++;
          resumen.respaldosBorrados += await podarRespaldos(this.directorio, gameId, this.config.respaldosAConservar ?? POR_DEFECTO.respaldosAConservar);
          resumen.auditoriaBorrada += await podarAuditoria(this.almacen, gameId, limiteAuditoria);
        } catch (err) {
          resumen.fallos.push({ gameId, error: err instanceof Error ? err.message : String(err) });
        }
      }
      if (resumen.fallos.length > 0) {
        console.error(`[mantenimiento] ${resumen.fallos.length} partida(s) fallaron:`, resumen.fallos);
      }
      return resumen;
    } finally {
      this.enCurso = false;
    }
  }
}
