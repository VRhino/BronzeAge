// Tarjeta de respaldos (`/v1/admin/respaldos`): verlos por partida (también los de las ya borradas), hacer uno ahora de la conectada,
// restaurar y borrar. Restaurar y borrar piden dos clics, como el resto de acciones destructivas de la consola. La usan la pestaña
// Mundo y la pantalla «sin partida».
import { ApiError, borrarRespaldo, listarRespaldos, respaldarAhora, restaurarRespaldo, type RespaldoListado } from '../app/apiCliente';
import { esc } from './html';

const ARMADO_MS = 5000;

export interface OpcionesRespaldos {
  /** La partida conectada, para «Respaldar ahora»; `null` sin partida. */
  actual: string | null;
  /** Tras restaurar: quien monta decide (conectar la consola a esa partida). */
  alRestaurar(gameId: string): void;
}

export function montarPanelRespaldos(cont: HTMLElement, opciones: OpcionesRespaldos): { cargar(): Promise<void> } {
  let respaldos: RespaldoListado[] = [];
  let armado: { clave: string; timer: ReturnType<typeof setTimeout> } | undefined;
  let aviso: { ok: boolean; texto: string } | undefined;
  let error: string | undefined;

  const fecha = (iso: string) => new Date(iso).toLocaleString('es-ES', { dateStyle: 'short', timeStyle: 'medium' });
  const tamano = (bytes: number) => (bytes < 1024 * 1024 ? `${Math.round(bytes / 1024)} KB` : `${(bytes / 1024 / 1024).toFixed(1)} MB`);
  const boton = (clave: string, texto: string, confirmar: string) =>
    `<button type="button" data-respaldo-accion="${clave}"${armado?.clave === clave ? ' class="btn-armado"' : ''}>${armado?.clave === clave ? confirmar : texto}</button>`;

  function pintar(): void {
    const porPartida = new Map<string, RespaldoListado[]>();
    for (const r of respaldos) porPartida.set(r.gameId, [...(porPartida.get(r.gameId) ?? []), r]);
    const grupos = [...porPartida]
      .map(([gameId, lista]) => {
        const estado = gameId === opciones.actual ? '<span class="badge">conectada</span>' : lista[0]!.partidaExiste ? '' : '<span class="badge badge-modified">borrada</span>';
        return `<div class="detail-sub"><span class="bots-sub">${esc(gameId)} ${estado} <small class="legend-note">${lista.length} respaldo${lista.length === 1 ? '' : 's'}</small></span>
          <table class="mini-table"><thead><tr><th>Hecho</th><th>Tamaño</th><th></th></tr></thead><tbody>${lista
            .map(
              (r) =>
                `<tr><td>${fecha(r.momento)}</td><td>${tamano(r.bytes)}</td><td class="respaldo-acciones">${boton(`restaurar:${r.archivo}`, 'Restaurar', '⚠ Confirmar: sustituye la partida')}${boton(`borrar:${r.archivo}`, 'Borrar', '⚠ Confirmar borrado')}</td></tr>`
            )
            .join('')}</tbody></table></div>`;
      })
      .join('');
    cont.innerHTML = `
      <h2>Respaldos</h2>
      <p class="legend-note">Copias fechadas de cada partida. Borrar una partida no borra sus respaldos: restaurar uno la devuelve (y a sus jugadores) y la abre.</p>
      ${opciones.actual ? `<div class="controls-row"><button type="button" data-respaldo-accion="ahora">Respaldar «${esc(opciones.actual)}» ahora</button><button type="button" data-respaldo-accion="recargar">Actualizar lista</button></div>` : '<div class="controls-row"><button type="button" data-respaldo-accion="recargar">Actualizar lista</button></div>'}
      ${aviso ? `<p class="legend-note" aria-live="polite">${aviso.ok ? '✓' : '✗'} ${esc(aviso.texto)}</p>` : ''}
      ${error ? `<p class="legend-note">${esc(error)}</p>` : grupos || '<p class="legend-note">No hay ningún respaldo todavía.</p>'}`;
  }

  async function cargar(): Promise<void> {
    try {
      respaldos = (await listarRespaldos()).respaldos;
      error = undefined;
    } catch (err) {
      error = err instanceof ApiError && err.status === 501 ? 'Este servidor no guarda respaldos en disco: con un almacén remoto los hace el proveedor.' : `No se pudo leer la lista: ${err instanceof Error ? err.message : err}`;
    }
    pintar();
  }

  async function hacer(accion: string): Promise<void> {
    const [tipo, archivo] = accion.split(/:(.*)/s) as [string, string | undefined];
    try {
      if (tipo === 'ahora' && opciones.actual) {
        const r = await respaldarAhora(opciones.actual);
        aviso = { ok: true, texto: `Respaldo hecho (${tamano(r.bytes)}).` };
      } else if (tipo === 'borrar' && archivo) {
        await borrarRespaldo(archivo);
        aviso = { ok: true, texto: 'Respaldo borrado.' };
      } else if (tipo === 'restaurar' && archivo) {
        const r = await restaurarRespaldo(archivo);
        aviso = { ok: true, texto: `«${r.gameId}» restaurada (v${r.version}, ${r.membresiasReactivadas} membresías devueltas).` };
        pintar();
        return opciones.alRestaurar(r.gameId);
      }
    } catch (err) {
      aviso = { ok: false, texto: err instanceof Error ? err.message : String(err) };
    }
    await cargar();
  }

  cont.addEventListener('click', (ev) => {
    const accion = (ev.target as HTMLElement).closest<HTMLElement>('[data-respaldo-accion]')?.dataset.respaldoAccion;
    if (!accion) return;
    if (accion === 'recargar') return void cargar();
    if (accion === 'ahora') return void hacer(accion);
    // Restaurar y borrar: el primer clic arma, el segundo hace.
    if (armado?.clave !== accion) {
      if (armado) clearTimeout(armado.timer);
      armado = { clave: accion, timer: setTimeout(() => ((armado = undefined), pintar()), ARMADO_MS) };
      aviso = undefined;
      return pintar();
    }
    clearTimeout(armado.timer);
    armado = undefined;
    void hacer(accion);
  });

  pintar();
  return { cargar };
}
