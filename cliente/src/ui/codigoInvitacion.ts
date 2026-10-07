// Tarjeta del código de invitación del registro de jugadores: el valor real del servidor (`GET/PUT /v1/admin/registro/codigo`),
// editable. La usan la pestaña Mundo y la pantalla «sin partida».
import { cambiarCodigoRegistro, leerCodigoRegistro } from '../app/apiCliente';

export function montarCodigoInvitacion(cont: HTMLElement): { cargar(): Promise<void> } {
  cont.innerHTML = `
    <h2>Código de invitación</h2>
    <p class="legend-note">Lo que piden a un jugador nuevo para registrarse. Es el valor real del servidor; se guarda solo en memoria, así que un reinicio vuelve al <code>CODIGO_REGISTRO</code> del entorno.</p>
    <div class="kv-row"><span>Vigente</span><span data-codigo="vigente">…</span></div>
    <label>Nuevo código <input data-codigo="input" maxlength="100" autocomplete="off" spellcheck="false" placeholder="vacío = registro abierto" /></label>
    <div class="controls-row">
      <button type="button" data-codigo="guardar">Guardar código</button>
      <button type="button" data-codigo="abrir">Dejar el registro abierto</button>
    </div>
    <p class="legend-note" data-codigo="estado" aria-live="polite" hidden></p>`;
  const $ = <T extends HTMLElement>(k: string) => cont.querySelector<T>(`[data-codigo="${k}"]`)!;
  const vigente = $('vigente');
  const input = $<HTMLInputElement>('input');
  const estado = $('estado');

  const aviso = (ok: boolean, texto: string) => {
    estado.hidden = false;
    estado.textContent = `${ok ? '✓' : '✗'} ${texto}`;
  };
  const mostrar = (codigo: string | null) => {
    vigente.textContent = codigo ?? 'ninguno: registro abierto';
    input.placeholder = codigo ? 'vacío = registro abierto' : 'sin código: escribe uno para cerrarlo';
  };
  const guardar = async (codigo: string | null) => {
    try {
      const { codigo: nuevo } = await cambiarCodigoRegistro(codigo);
      mostrar(nuevo);
      input.value = '';
      aviso(true, nuevo ? `Código cambiado a «${nuevo}»: lo piden los registros nuevos.` : 'Registro abierto: ya no se pide código.');
    } catch (err) {
      aviso(false, err instanceof Error ? err.message : String(err));
    }
  };

  $('guardar').addEventListener('click', () => {
    const nuevo = input.value.trim();
    if (!nuevo) return aviso(false, 'Escribe un código, o usa «Dejar el registro abierto».');
    void guardar(nuevo);
  });
  $('abrir').addEventListener('click', () => void guardar(null));

  return {
    async cargar() {
      try {
        mostrar((await leerCodigoRegistro()).codigo);
      } catch (err) {
        vigente.textContent = 'no se pudo leer';
        aviso(false, err instanceof Error ? err.message : String(err));
      }
    },
  };
}
