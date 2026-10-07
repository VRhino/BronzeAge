// Pantalla «sin partida»: la consola no está conectada a ninguna (se borró la última, o se eligió no tener ninguna). Es un estado
// válido: desde aquí se abre una de las que hay, se crea una nueva con control total sobre cómo nace, se gestiona el código de
// invitación y se restauran respaldos. No crea nada por su cuenta. Abrir o crear guarda la elección y recarga: arranca la consola.
import type { RegionId } from '@motor/domain/types';
import { crearOResumirPartida, listarPartidas } from './app/apiCliente';
import { elegirGameId } from './app/gameStore';
import { montarCodigoInvitacion } from './ui/codigoInvitacion';
import { esc } from './ui/html';
import { htmlListaPartidas, ID_PARTIDA_VALIDO, OPCIONES_REGION, OPCIONES_TICK } from './ui/opcionesPartida';
import { montarPanelRespaldos } from './ui/panelRespaldos';

function conectar(gameId: string): void {
  elegirGameId(gameId);
  location.reload();
}

export async function montarSinPartida(aviso?: string): Promise<void> {
  const app = document.getElementById('app')!;
  app.classList.add('sin-partida');
  app.innerHTML = `
    <div class="controls-panel">
      <div class="admin-titlebar"><h1>Bronze Age Collapse — Fase 0</h1></div>
      <div class="section-title registros-heading">Sin partida</div>
      <p class="legend-note registros-intro">${aviso ? `${esc(aviso)} ` : ''}Esta consola no está conectada a ninguna partida. Abre una de las que hay o crea una nueva.</p>
      <div class="controls-grid world-generation-grid">
        <div class="controls">
          <h2>Partidas en el servidor</h2>
          <div class="partidas-lista" id="sp-partidas" role="list"><p class="legend-note">Cargando…</p></div>
        </div>
        <div class="controls">
          <h2>Crear una partida</h2>
          <label>ID de la partida <input id="sp-id" autocomplete="off" spellcheck="false" /></label>
          <label>Seed del mundo <input id="sp-seed" type="number" value="1" /></label>
          <label>Región geográfica (Fase 0.2) <select id="sp-region">${OPCIONES_REGION}</select></label>
          <label>Velocidad de tick <select id="sp-tick">${OPCIONES_TICK}</select></label>
          <button type="button" id="sp-crear" disabled>Crear</button>
          <p class="legend-note" id="sp-estado" aria-live="polite"></p>
        </div>
        <div class="controls" id="sp-codigo"></div>
        <div class="controls" id="sp-respaldos"></div>
      </div>
    </div>`;

  const lista = document.getElementById('sp-partidas')!;
  const idInput = document.getElementById('sp-id') as HTMLInputElement;
  const crearBtn = document.getElementById('sp-crear') as HTMLButtonElement;
  const estado = document.getElementById('sp-estado')!;
  let existentes: string[] = [];

  const actualizarCrear = () => {
    const id = idInput.value.trim();
    const valido = ID_PARTIDA_VALIDO.test(id);
    crearBtn.disabled = !valido;
    crearBtn.textContent = existentes.includes(id) ? `Abrir «${id}»` : valido ? `Crear «${id}»` : 'Crear';
    estado.textContent = id && !valido ? 'El ID solo admite letras, números, guion, guion bajo y punto.' : existentes.includes(id) ? 'Ya existe: se abre tal como está.' : '';
  };

  const cargarPartidas = async () => {
    try {
      const { partidas } = await listarPartidas();
      existentes = partidas.map((p) => p.gameId);
      lista.innerHTML = htmlListaPartidas(partidas, null);
    } catch (err) {
      lista.innerHTML = `<p class="legend-note">No se pudo leer la lista: ${esc(err instanceof Error ? err.message : err)}</p>`;
    }
    actualizarCrear();
  };

  lista.addEventListener('click', (ev) => {
    const id = (ev.target as HTMLElement).closest<HTMLElement>('[data-partida]')?.dataset.partida;
    if (id) conectar(id);
  });
  idInput.addEventListener('input', actualizarCrear);
  crearBtn.addEventListener('click', async () => {
    const id = idInput.value.trim();
    if (existentes.includes(id)) return conectar(id);
    crearBtn.disabled = true;
    estado.textContent = 'Creando partida…';
    try {
      const region = (document.getElementById('sp-region') as HTMLSelectElement).value as RegionId | '';
      const seed = Number((document.getElementById('sp-seed') as HTMLInputElement).value) || 0;
      const tick = Number((document.getElementById('sp-tick') as HTMLSelectElement).value) || undefined;
      await crearOResumirPartida(id, seed, region || undefined, true, tick);
      conectar(id);
    } catch (err) {
      estado.textContent = `✗ ${err instanceof Error ? err.message : err}`;
      crearBtn.disabled = false;
    }
  });

  const codigo = montarCodigoInvitacion(document.getElementById('sp-codigo')!);
  const respaldos = montarPanelRespaldos(document.getElementById('sp-respaldos')!, { actual: null, alRestaurar: conectar });
  await Promise.all([cargarPartidas(), codigo.cargar(), respaldos.cargar()]);
  // Otra consola puede crear o abrir partidas mientras tanto.
  setInterval(() => {
    if (!document.hidden) void cargarPartidas();
  }, 5000);
}
