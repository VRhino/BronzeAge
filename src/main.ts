// Capa de interfaz (DOM/canvas). Regla de frontera: este archivo — y cualquier otro bajo `ui/` —
// solo puede importar de `./app/gameStore` (acciones + estado + suscripción) y de `./ui/canvas`
// (dibujo/color, puramente presentacional). Nunca importa nada de `./engine/*` ni captura errores
// de dominio: eso es responsabilidad exclusiva de `gameStore`. Los tipos de `./domain/types` se
// importan solo como `type` para tipar lo que se lee — no acoplan a ninguna lógica.
import type { Asentamiento, CargoTipo, Faccion } from './domain/types';
import { CATALOGOS, gameStore, type GameState } from './app/gameStore';
import { draw, faccionColor, RECURSO_COLOR, RECURSOS_EN_MAPA, EDIFICIO_COLOR, FACCION_COLORES, type DrawState } from './ui/canvas';

const CANVAS_SIZE = 800;

const RECURSO_NOMBRE: Record<string, string> = {
  madera: 'Madera',
  piedra: 'Piedra',
  trigo: 'Trigo',
  cobre: 'Cobre',
  estano: 'Estaño',
  oro: 'Oro',
  livestock: 'Livestock',
};

const EDIFICIO_NOMBRE: Record<string, string> = {
  vivienda: 'Vivienda',
  granja: 'Granja',
  cantera: 'Cantera',
  lenera: 'Leñera',
  almacen: 'Almacén',
  taller: 'Taller',
  mina: 'Mina de oro',
  minaCobre: 'Mina de cobre',
  fundicion: 'Fundición',
  granFundicion: 'Gran Fundición',
};

// --- Estado de vista (qué se muestra, no simulación): vive solo aquí, nunca en el store. ---
let tabActivo: 'acciones' | 'asentamientos' | 'jugadores' = 'acciones';
let asentamientoSeleccionadoId: string | null = null;
let jugadorSeleccionadoId: string | null = null;

const app = document.getElementById('app')!;
app.innerHTML = `
  <div class="controls-panel">
    <h1>Bronze Age Collapse — Fase 0</h1>
    <div class="tabs" id="main-tabs">
      <button class="tab-btn" data-tab="acciones">Acciones</button>
      <button class="tab-btn" data-tab="asentamientos">Asentamientos</button>
      <button class="tab-btn" data-tab="jugadores">Jugadores</button>
    </div>

    <div class="tab-panel" id="tab-acciones">
    <div class="controls-grid">
      <div class="controls">
        <h2>Mundo</h2>
        <label>
          Facción activa (clic en el mapa funda aquí)
          <select id="faccion-select"></select>
        </label>
        <label>
          Jugadores fundadores (fundación grupal, 1-5)
          <input id="jugadores-input" type="number" value="1" min="1" max="5" />
        </label>
        <label>
          Seed del mundo
          <input id="seed-input" type="number" value="1" />
        </label>
        <button id="regenerar-btn">Regenerar mundo</button>
        <button id="tick-btn">Avanzar tick</button>
        <div id="tick-counter">Tick: 0</div>
      </div>

      <div class="controls">
        <h2>Cargos (Doc 2.2)</h2>
        <label>Facción <select id="cargo-faccion"></select></label>
        <label>Jugador (id) <input id="cargo-jugador" type="text" placeholder="jugador-faccion-1-1" /></label>
        <button id="rey-btn">Asignar Rey</button>
        <button id="embajador-btn">Asignar Embajador</button>
        <label>Asentamiento <select id="cargo-asentamiento"></select></label>
        <label>Cargo local <select id="cargo-tipo"></select></label>
        <button id="cargo-local-btn">Asignar cargo local</button>
      </div>

      <div class="controls">
        <h2>Ciudadanía (Doc 2.5)</h2>
        <label>Asentamiento <select id="casa-asentamiento"></select></label>
        <label>Jugador (id) <input id="casa-jugador" type="text" placeholder="jugador-nuevo-1" /></label>
        <button id="casa-btn">Comprar casa</button>
      </div>

      <div class="controls">
        <h2>Políticas (Doc 4.4)</h2>
        <label>Asentamiento <select id="politica-asentamiento"></select></label>
        <label>Cargo que la activa <select id="politica-cargo"></select></label>
        <label>Política <select id="politica-id"></select></label>
        <button id="politica-btn">Activar política</button>
      </div>

      <div class="controls">
        <h2>Diplomacia (Doc 2.3/2.4)</h2>
        <label>Facción A (señora si es vasallaje) <select id="diplo-a"></select></label>
        <label>Facción B (vasalla si es vasallaje) <select id="diplo-b"></select></label>
        <label>Tipo <select id="diplo-tipo"><option value="alianza">Alianza</option><option value="vasallaje">Vasallaje</option></select></label>
        <label>Tributo: recurso <select id="diplo-tributo-recurso"></select></label>
        <label>Tributo: cantidad/tick <input id="diplo-tributo-cantidad" type="number" value="2" min="0" /></label>
        <button id="diplo-proponer-btn">Proponer relación</button>
        <label>Relación activa <select id="diplo-relacion"></select></label>
        <button id="diplo-romper-btn">Romper (voluntario)</button>
        <button id="diplo-rebelion-btn">Rebelión de vasallo</button>
      </div>

      <div class="controls">
        <h2>Fusión / Anexión (Doc 2.6)</h2>
        <label>Facción A <select id="fusion-a"></select></label>
        <label>Facción B <select id="fusion-b"></select></label>
        <button id="anexion-btn">A anexiona a B</button>
        <label>Nuevo nombre (fusión) <input id="fusion-nombre" type="text" value="Liga Nueva" /></label>
        <label>Nuevo Rey (id jugador, fusión) <input id="fusion-rey" type="text" /></label>
        <button id="fusion-btn">Fusionar en Facción nueva</button>
      </div>

      <div class="controls">
        <h2>Trueque (Doc 3.2)</h2>
        <label>Asentamiento A <select id="trueque-a"></select></label>
        <label>Recurso que entrega A <select id="trueque-recurso-a"></select></label>
        <label>Cantidad de A <input id="trueque-cantidad-a" type="number" value="50" min="1" /></label>
        <label>Asentamiento B <select id="trueque-b"></select></label>
        <label>Recurso que entrega B <select id="trueque-recurso-b"></select></label>
        <label>Cantidad de B <input id="trueque-cantidad-b" type="number" value="50" min="1" /></label>
        <button id="trueque-btn">Proponer trueque</button>
      </div>

      <div class="controls">
        <h2>Orden de Mercado (Doc 3.3)</h2>
        <label>Asentamiento <select id="mercado-asentamiento"></select></label>
        <label>Tipo
          <select id="mercado-tipo"><option value="venta">Venta</option><option value="compra">Compra</option></select>
        </label>
        <label>Recurso <select id="mercado-recurso"></select></label>
        <label>Cantidad <input id="mercado-cantidad" type="number" value="30" min="1" /></label>
        <label>Precio unitario (vacío = precio de referencia) <input id="mercado-precio" type="number" min="0" step="0.1" /></label>
        <button id="mercado-btn">Colocar orden</button>
      </div>

      <div class="controls">
        <h2>Guerra (Doc 5)</h2>
        <label>Asentamiento <select id="guerra-asentamiento"></select></label>
        <label>Reclutar de <select id="reclutar-origen"></select></label>
        <label>Cantidad <input id="reclutar-cantidad" type="number" value="10" min="1" /></label>
        <button id="reclutar-btn">Reclutar</button>
        <button id="fundicion-btn">Construir Fundición</button>
        <button id="gran-fundicion-btn">Construir Gran Fundición</button>

        <label>Escuadrones propios (ids separados por coma) <input id="guerra-escuadrones" type="text" placeholder="escuadron-..." /></label>
        <label>Asentamiento objetivo/rival <select id="guerra-objetivo"></select></label>
        <label>Escuadrones del objetivo (solo campo abierto) <input id="guerra-escuadrones-objetivo" type="text" /></label>
        <button id="asedio-btn">Iniciar asedio</button>
        <button id="campo-abierto-btn">Combate en campo abierto</button>
        <label>Caravana a interceptar <select id="guerra-caravana"></select></label>
        <button id="interceptar-btn">Interceptar caravana</button>
      </div>
    </div>
    </div>

    <div class="tab-panel" id="tab-asentamientos" hidden>
      <div id="asentamientos-tab"></div>
    </div>

    <div class="tab-panel" id="tab-jugadores" hidden>
      <div id="jugadores-tab"></div>
    </div>
  </div>

  <div class="map-column">
    <div class="map-panel">
      <canvas id="world-canvas" width="${CANVAS_SIZE}" height="${CANVAS_SIZE}"></canvas>
      <div class="legend" id="legend">
        <div class="legend-header" id="legend-toggle">Leyenda ▾</div>
        <div class="legend-body" id="legend-body"></div>
      </div>
    </div>

    <div class="section-title">Estado de la simulación</div>
    <div class="logs-grid">
      <div class="log-card">
        <h2>Asentamientos</h2>
        <div class="log-panel" id="asentamientos-panel"></div>
      </div>
      <div class="log-card">
        <h2>Política</h2>
        <div class="log-panel" id="politica-panel"></div>
      </div>
      <div class="log-card">
        <h2>Progresión (Doc 2.9)</h2>
        <div class="log-panel" id="progresion-panel"></div>
      </div>
      <div class="log-card">
        <h2>Militar</h2>
        <div class="log-panel" id="militar-panel"></div>
      </div>
      <div class="log-card">
        <h2>Economía</h2>
        <div class="log-panel" id="economia-panel"></div>
      </div>
      <div class="log-card">
        <h2>Registro</h2>
        <div class="log-panel" id="log"></div>
      </div>
    </div>
  </div>
`;

const canvas = document.getElementById('world-canvas') as HTMLCanvasElement;
const ctx = canvas.getContext('2d')!;
const legendEl = document.getElementById('legend')!;
const legendBodyEl = document.getElementById('legend-body')!;
const logEl = document.getElementById('log')!;
const asentamientosPanelEl = document.getElementById('asentamientos-panel')!;
const politicaPanelEl = document.getElementById('politica-panel')!;
const economiaPanelEl = document.getElementById('economia-panel')!;
const tickCounterEl = document.getElementById('tick-counter')!;
const faccionSelect = document.getElementById('faccion-select') as HTMLSelectElement;
const seedInput = document.getElementById('seed-input') as HTMLInputElement;
const jugadoresInput = document.getElementById('jugadores-input') as HTMLInputElement;

const truequeASelect = document.getElementById('trueque-a') as HTMLSelectElement;
const truequeBSelect = document.getElementById('trueque-b') as HTMLSelectElement;
const truequeRecursoASelect = document.getElementById('trueque-recurso-a') as HTMLSelectElement;
const truequeRecursoBSelect = document.getElementById('trueque-recurso-b') as HTMLSelectElement;
const truequeCantidadAInput = document.getElementById('trueque-cantidad-a') as HTMLInputElement;
const truequeCantidadBInput = document.getElementById('trueque-cantidad-b') as HTMLInputElement;

const mercadoAsentamientoSelect = document.getElementById('mercado-asentamiento') as HTMLSelectElement;
const mercadoTipoSelect = document.getElementById('mercado-tipo') as HTMLSelectElement;
const mercadoRecursoSelect = document.getElementById('mercado-recurso') as HTMLSelectElement;
const mercadoCantidadInput = document.getElementById('mercado-cantidad') as HTMLInputElement;
const mercadoPrecioInput = document.getElementById('mercado-precio') as HTMLInputElement;

const cargoFaccionSelect = document.getElementById('cargo-faccion') as HTMLSelectElement;
const cargoJugadorInput = document.getElementById('cargo-jugador') as HTMLInputElement;
const cargoAsentamientoSelect = document.getElementById('cargo-asentamiento') as HTMLSelectElement;
const cargoTipoSelect = document.getElementById('cargo-tipo') as HTMLSelectElement;

const casaAsentamientoSelect = document.getElementById('casa-asentamiento') as HTMLSelectElement;
const casaJugadorInput = document.getElementById('casa-jugador') as HTMLInputElement;

const politicaAsentamientoSelect = document.getElementById('politica-asentamiento') as HTMLSelectElement;
const politicaCargoSelect = document.getElementById('politica-cargo') as HTMLSelectElement;
const politicaIdSelect = document.getElementById('politica-id') as HTMLSelectElement;

const diploASelect = document.getElementById('diplo-a') as HTMLSelectElement;
const diploBSelect = document.getElementById('diplo-b') as HTMLSelectElement;
const diploTipoSelect = document.getElementById('diplo-tipo') as HTMLSelectElement;
const diploTributoRecursoSelect = document.getElementById('diplo-tributo-recurso') as HTMLSelectElement;
const diploTributoCantidadInput = document.getElementById('diplo-tributo-cantidad') as HTMLInputElement;
const diploRelacionSelect = document.getElementById('diplo-relacion') as HTMLSelectElement;

const fusionASelect = document.getElementById('fusion-a') as HTMLSelectElement;
const fusionBSelect = document.getElementById('fusion-b') as HTMLSelectElement;
const fusionNombreInput = document.getElementById('fusion-nombre') as HTMLInputElement;
const fusionReyInput = document.getElementById('fusion-rey') as HTMLInputElement;

const guerraAsentamientoSelect = document.getElementById('guerra-asentamiento') as HTMLSelectElement;
const reclutarOrigenSelect = document.getElementById('reclutar-origen') as HTMLSelectElement;
const reclutarCantidadInput = document.getElementById('reclutar-cantidad') as HTMLInputElement;
const guerraEscuadronesInput = document.getElementById('guerra-escuadrones') as HTMLInputElement;
const guerraObjetivoSelect = document.getElementById('guerra-objetivo') as HTMLSelectElement;
const guerraEscuadronesObjetivoInput = document.getElementById('guerra-escuadrones-objetivo') as HTMLInputElement;
const guerraCaravanaSelect = document.getElementById('guerra-caravana') as HTMLSelectElement;
const militarPanelEl = document.getElementById('militar-panel')!;
const progresionPanelEl = document.getElementById('progresion-panel')!;

cargoTipoSelect.innerHTML = CATALOGOS.cargos.map((c) => `<option value="${c}">${c}</option>`).join('');
politicaCargoSelect.innerHTML = CATALOGOS.cargos.map((c) => `<option value="${c}">${c}</option>`).join('');
politicaIdSelect.innerHTML = CATALOGOS.politicas.map((p) => `<option value="${p.id}">${p.nombre} (${p.cargo})</option>`).join('');
diploTributoRecursoSelect.innerHTML = CATALOGOS.recursosTrueque.map((r) => `<option value="${r}">${r}</option>`).join('');
truequeRecursoASelect.innerHTML = CATALOGOS.recursosTrueque.map((r) => `<option value="${r}">${r}</option>`).join('');
truequeRecursoBSelect.innerHTML = CATALOGOS.recursosTrueque.map((r) => `<option value="${r}">${r}</option>`).join('');
mercadoRecursoSelect.innerHTML = CATALOGOS.recursosMercado.map((r) => `<option value="${r}">${r}</option>`).join('');
reclutarOrigenSelect.innerHTML = CATALOGOS.origenesTropa.map((o) => `<option value="${o}">${o}</option>`).join('');

function etiquetaAsentamiento(a: Asentamiento, facciones: Faccion[]): string {
  const nombreFaccion = facciones.find((f) => f.id === a.faccionId)?.nombre ?? a.faccionId;
  return `${a.id} (${nombreFaccion})`;
}

function actualizarSelects(state: GameState): void {
  const opcionesAsentamientos = state.asentamientos.map((a) => `<option value="${a.id}">${etiquetaAsentamiento(a, state.facciones)}</option>`).join('');
  for (const select of [
    truequeASelect,
    truequeBSelect,
    mercadoAsentamientoSelect,
    cargoAsentamientoSelect,
    casaAsentamientoSelect,
    politicaAsentamientoSelect,
    guerraAsentamientoSelect,
    guerraObjetivoSelect,
  ]) {
    const seleccionPrevia = select.value;
    select.innerHTML = opcionesAsentamientos;
    if (state.asentamientos.some((a) => a.id === seleccionPrevia)) select.value = seleccionPrevia;
  }

  const opcionesCaravanas = state.caravanas
    .map((c) => `<option value="${c.id}">${c.id} (${c.origenAsentamientoId} → ${c.destinoAsentamientoId})</option>`)
    .join('');
  const caravanaPrevia = guerraCaravanaSelect.value;
  guerraCaravanaSelect.innerHTML = opcionesCaravanas;
  if (state.caravanas.some((c) => c.id === caravanaPrevia)) guerraCaravanaSelect.value = caravanaPrevia;

  const opcionesFacciones = state.facciones.map((f) => `<option value="${f.id}">${f.nombre}</option>`).join('');
  for (const select of [cargoFaccionSelect, diploASelect, diploBSelect, fusionASelect, fusionBSelect]) {
    const seleccionPrevia = select.value;
    select.innerHTML = opcionesFacciones;
    if (state.facciones.some((f) => f.id === seleccionPrevia)) select.value = seleccionPrevia;
  }

  const seleccionPreviaFaccionActiva = faccionSelect.value;
  faccionSelect.innerHTML = state.facciones
    .map((f) => `<option value="${f.id}" style="color:${faccionColor(f.id, state.facciones)}">${f.nombre}</option>`)
    .join('');
  if (state.facciones.some((f) => f.id === seleccionPreviaFaccionActiva)) faccionSelect.value = seleccionPreviaFaccionActiva;

  const relacionesActivas = state.relaciones.filter((r) => r.estado === 'activa');
  const seleccionPrevia = diploRelacionSelect.value;
  diploRelacionSelect.innerHTML = relacionesActivas
    .map((r) => {
      const a = state.facciones.find((f) => f.id === r.faccionAId)?.nombre ?? r.faccionAId;
      const b = state.facciones.find((f) => f.id === r.faccionBId)?.nombre ?? r.faccionBId;
      return `<option value="${r.id}">${r.tipo}: ${a} → ${b}</option>`;
    })
    .join('');
  if (relacionesActivas.some((r) => r.id === seleccionPrevia)) diploRelacionSelect.value = seleccionPrevia;
}

function renderPanelAsentamientos(state: GameState): void {
  asentamientosPanelEl.innerHTML = state.asentamientos
    .map((a) => {
      const { pesants, artesanos, nobleza } = a.poblacion;
      const recursosClave = ['madera', 'piedra', 'trigo', 'oro']
        .map((r) => `${r}: ${Math.floor(a.almacen[r]?.cantidad ?? 0)}/${a.almacen[r]?.capacidad ?? 0}`)
        .join(' · ');
      const activos = a.edificios.filter((e) => e.estado === 'activo').length;
      const enCurso = a.edificios.length - activos;
      const cargosTxt = CATALOGOS.cargos.map((c) => `${c}: ${a.cargos[`${c}Id` as keyof typeof a.cargos] ?? '—'}`).join(' · ');
      const otrasCasas = a.casasCompradas.filter((id) => !a.jugadoresFundadoresIds.includes(id));
      return `<div>
        <strong>${etiquetaAsentamiento(a, state.facciones)}</strong> (nivel ${a.nivel}, radio ${Math.round(a.radioPotencial)}) · Mantenimiento: ${a.medidorMantenimiento.toFixed(0)}/100<br/>
        Fundadores (con casa): ${a.jugadoresFundadoresIds.join(', ')} · Otras casas compradas: ${otrasCasas.join(', ') || '—'}<br/>
        Cargos — ${cargosTxt}<br/>
        Población — Pesants: ${pesants} · Artesanos: ${artesanos} · Nobleza: ${nobleza}<br/>
        Almacén — ${recursosClave}<br/>
        Edificios — activos: ${activos}, en curso/cola: ${enCurso} · Políticas activas: ${a.politicasActivas.length}
      </div>`;
    })
    .join('');
}

function renderDetalleAsentamiento(a: Asentamiento, state: GameState): string {
  const faccion = state.facciones.find((f) => f.id === a.faccionId);
  const { pesants, artesanos, nobleza } = a.poblacion;
  const totalPoblacion = pesants + artesanos + nobleza;
  const otrasCasas = a.casasCompradas.filter((id) => !a.jugadoresFundadoresIds.includes(id));

  const almacenHtml = CATALOGOS.recursosTrueque
    .map((r) => {
      const info = a.almacen[r];
      return `<div class="kv-row"><span>${RECURSO_NOMBRE[r] ?? r}</span><span>${Math.floor(info?.cantidad ?? 0)}/${info?.capacidad ?? 0}</span></div>`;
    })
    .join('');

  const cargosHtml = CATALOGOS.cargos
    .map((c) => {
      const id = a.cargos[`${c}Id` as keyof typeof a.cargos];
      return `<div class="kv-row"><span>${c}</span><span>${id ?? '—'}</span></div>`;
    })
    .join('');

  const edificiosPorTipo = new Map<string, { activo: number; en_construccion: number; en_cola: number }>();
  for (const e of a.edificios) {
    const entry = edificiosPorTipo.get(e.tipo) ?? { activo: 0, en_construccion: 0, en_cola: 0 };
    entry[e.estado] += 1;
    edificiosPorTipo.set(e.tipo, entry);
  }
  const edificiosHtml = edificiosPorTipo.size
    ? `<table class="mini-table">
        <thead><tr><th>Edificio</th><th>Activos</th><th>En construcción</th><th>En cola</th></tr></thead>
        <tbody>
          ${Array.from(edificiosPorTipo.entries())
            .map(
              ([tipo, e]) =>
                `<tr><td>${EDIFICIO_NOMBRE[tipo] ?? tipo}</td><td>${e.activo}</td><td>${e.en_construccion}</td><td>${e.en_cola}</td></tr>`
            )
            .join('')}
        </tbody>
      </table>`
    : '<p class="legend-note">Sin edificios.</p>';

  const politicasHtml = a.politicasActivas.length
    ? `<div class="chip-row">${a.politicasActivas
        .map((p) => `<span class="chip">${CATALOGOS.politicas.find((c) => c.id === p.politicaId)?.nombre ?? p.politicaId} (hasta t${p.expiraEnTick})</span>`)
        .join('')}</div>`
    : '<p class="legend-note">Sin políticas activas.</p>';

  const escuadronesHtml = a.escuadrones.length
    ? `<table class="mini-table">
        <thead><tr><th>Escuadrón</th><th>Origen</th><th>Tier</th><th>Cantidad</th><th>Veteranía</th><th>Moral</th></tr></thead>
        <tbody>
          ${a.escuadrones
            .map(
              (e) =>
                `<tr><td>${e.nombre}${e.heridoHastaTick ? ' (herido)' : ''}</td><td>${e.origen}</td><td>${e.tier}</td><td>${e.cantidad}</td><td>${e.veterania.toFixed(1)}</td><td>${e.moral.toFixed(0)}</td></tr>`
            )
            .join('')}
        </tbody>
      </table>`
    : '<p class="legend-note">Sin escuadrones.</p>';

  return `
    <div class="settlement-detail">
      <div class="detail-section">
        <h3>${a.id}</h3>
        <div class="kv-grid">
          <div class="kv-row"><span>Facción</span><span>${faccion?.nombre ?? a.faccionId}</span></div>
          <div class="kv-row"><span>Nivel</span><span>${a.nivel}</span></div>
          <div class="kv-row"><span>Radio potencial</span><span>${Math.round(a.radioPotencial)}</span></div>
          <div class="kv-row"><span>Fundado en tick</span><span>${a.fundadoEnTick}</span></div>
        </div>
        <div class="kv-row" style="margin-top:6px"><span>Mantenimiento</span><span>${a.medidorMantenimiento.toFixed(0)}/100</span></div>
        <div class="mantenimiento-bar"><div class="mantenimiento-fill" style="width:${Math.max(0, Math.min(100, a.medidorMantenimiento))}%"></div></div>
      </div>

      <div class="detail-section">
        <h3>Fundadores y ciudadanía</h3>
        <div class="kv-row"><span>Fundadores (con casa)</span><span>${a.jugadoresFundadoresIds.join(', ') || '—'}</span></div>
        <div class="kv-row"><span>Otras casas compradas</span><span>${otrasCasas.join(', ') || '—'}</span></div>
        <div class="kv-row"><span>Cupo de vivienda</span><span>${a.casasCompradas.length}/${gameStore.cupoVivienda(a)}</span></div>
      </div>

      <div class="detail-section">
        <h3>Cargos</h3>
        <div class="kv-grid">${cargosHtml}</div>
      </div>

      <div class="detail-section">
        <h3>Población (total ${totalPoblacion})</h3>
        <div class="kv-grid">
          <div class="kv-row"><span>Pesants</span><span>${pesants}</span></div>
          <div class="kv-row"><span>Artesanos</span><span>${artesanos}</span></div>
          <div class="kv-row"><span>Nobleza</span><span>${nobleza}</span></div>
        </div>
      </div>

      <div class="detail-section">
        <h3>Almacén</h3>
        <div class="kv-grid">${almacenHtml}</div>
      </div>

      <div class="detail-section">
        <h3>Edificios</h3>
        ${edificiosHtml}
      </div>

      <div class="detail-section">
        <h3>Políticas activas</h3>
        ${politicasHtml}
      </div>

      <div class="detail-section">
        <h3>Escuadrones</h3>
        ${escuadronesHtml}
      </div>
    </div>
  `;
}

function renderAsentamientosTab(state: GameState): void {
  const cont = document.getElementById('asentamientos-tab')!;

  if (state.asentamientos.length === 0) {
    asentamientoSeleccionadoId = null;
    cont.innerHTML = '<p class="legend-note">Aún no hay asentamientos fundados.</p>';
    return;
  }

  if (!asentamientoSeleccionadoId || !state.asentamientos.some((a) => a.id === asentamientoSeleccionadoId)) {
    asentamientoSeleccionadoId = state.asentamientos[0]!.id;
  }

  const gruposHtml = state.facciones
    .map((f) => {
      const propios = state.asentamientos.filter((a) => a.faccionId === f.id);
      if (propios.length === 0) return '';
      const botones = propios
        .map(
          (a) =>
            `<button type="button" class="settlement-tab-btn${a.id === asentamientoSeleccionadoId ? ' active' : ''}" data-settlement="${a.id}">${a.id}</button>`
        )
        .join('');
      return `<div class="faccion-group">
        <div class="faccion-group-header"><span class="swatch" style="background:${faccionColor(f.id, state.facciones)}"></span>${f.nombre}</div>
        <div class="settlement-tab-row">${botones}</div>
      </div>`;
    })
    .join('');

  const seleccionado = state.asentamientos.find((a) => a.id === asentamientoSeleccionadoId)!;
  cont.innerHTML = `${gruposHtml}${renderDetalleAsentamiento(seleccionado, state)}`;

  cont.querySelectorAll('.settlement-tab-btn').forEach((btn) => {
    btn.addEventListener('click', () => {
      asentamientoSeleccionadoId = (btn as HTMLElement).dataset.settlement!;
      renderAsentamientosTab(gameStore.getState());
    });
  });
}

function renderDetalleJugador(jugadorId: string, state: GameState): string {
  const faccion = state.facciones.find((f) => f.ciudadanosIds.includes(jugadorId));
  const esRey = faccion?.reyId === jugadorId;
  const esEmbajador = faccion?.embajadorId === jugadorId;

  const cargosFaccionHtml =
    esRey || esEmbajador
      ? `<div class="chip-row">${esRey ? '<span class="chip">Rey</span>' : ''}${esEmbajador ? '<span class="chip">Embajador</span>' : ''}</div>`
      : '<p class="legend-note">Sin cargos de Facción.</p>';

  const cargosLocales: { asentamientoId: string; cargo: CargoTipo }[] = [];
  for (const a of state.asentamientos) {
    for (const c of CATALOGOS.cargos) {
      if (a.cargos[`${c}Id` as keyof typeof a.cargos] === jugadorId) {
        cargosLocales.push({ asentamientoId: a.id, cargo: c });
      }
    }
  }
  const cargosLocalesHtml = cargosLocales.length
    ? `<table class="mini-table">
        <thead><tr><th>Asentamiento</th><th>Cargo</th></tr></thead>
        <tbody>${cargosLocales.map((c) => `<tr><td>${c.asentamientoId}</td><td>${c.cargo}</td></tr>`).join('')}</tbody>
      </table>`
    : '<p class="legend-note">Sin cargos locales.</p>';

  const residencias = state.asentamientos.filter((a) => a.casasCompradas.includes(jugadorId));
  const residenciasHtml = residencias.length
    ? `<div class="chip-row">${residencias
        .map((a) => `<span class="chip">${a.id}${a.jugadoresFundadoresIds.includes(jugadorId) ? ' (fundador)' : ''}</span>`)
        .join('')}</div>`
    : '<p class="legend-note">Sin residencias.</p>';

  const historial = state.historialJugadores[jugadorId] ?? [];
  const historialHtml = historial.length
    ? `<div class="log-panel">${historial.map((e) => `<div>[t${e.tick}] ${e.mensaje}</div>`).join('')}</div>`
    : '<p class="legend-note">Sin actividad registrada todavía.</p>';

  return `
    <div class="settlement-detail">
      <div class="detail-section">
        <h3>${jugadorId}</h3>
        <div class="kv-row"><span>Facción</span><span>${faccion?.nombre ?? '—'}</span></div>
      </div>

      <div class="detail-section">
        <h3>Cargos de Facción</h3>
        ${cargosFaccionHtml}
      </div>

      <div class="detail-section">
        <h3>Cargos locales</h3>
        ${cargosLocalesHtml}
      </div>

      <div class="detail-section">
        <h3>Residencias</h3>
        ${residenciasHtml}
      </div>

      <div class="detail-section">
        <h3>Historial de actividad</h3>
        ${historialHtml}
      </div>
    </div>
  `;
}

function renderJugadoresTab(state: GameState): void {
  const cont = document.getElementById('jugadores-tab')!;
  const todosLosIds = state.facciones.flatMap((f) => f.ciudadanosIds);

  if (todosLosIds.length === 0) {
    jugadorSeleccionadoId = null;
    cont.innerHTML = '<p class="legend-note">Aún no hay jugadores con ciudadanía.</p>';
    return;
  }

  if (!jugadorSeleccionadoId || !todosLosIds.includes(jugadorSeleccionadoId)) {
    jugadorSeleccionadoId = todosLosIds[0]!;
  }

  const gruposHtml = state.facciones
    .map((f) => {
      if (f.ciudadanosIds.length === 0) return '';
      const botones = f.ciudadanosIds
        .map(
          (id) =>
            `<button type="button" class="settlement-tab-btn${id === jugadorSeleccionadoId ? ' active' : ''}" data-jugador="${id}">${id}</button>`
        )
        .join('');
      return `<div class="faccion-group">
        <div class="faccion-group-header"><span class="swatch" style="background:${faccionColor(f.id, state.facciones)}"></span>${f.nombre}</div>
        <div class="settlement-tab-row">${botones}</div>
      </div>`;
    })
    .join('');

  cont.innerHTML = `${gruposHtml}${renderDetalleJugador(jugadorSeleccionadoId!, state)}`;

  cont.querySelectorAll('.settlement-tab-btn').forEach((btn) => {
    btn.addEventListener('click', () => {
      jugadorSeleccionadoId = (btn as HTMLElement).dataset.jugador!;
      renderJugadoresTab(gameStore.getState());
    });
  });
}

function renderPanelPolitica(state: GameState): void {
  const facH = state.facciones
    .map((f) => {
      const cap = gameStore.capFundacion(f.nivel);
      const propios = state.asentamientos.filter((a) => a.faccionId === f.id).length;
      return `<div><strong>${f.nombre}</strong> — nivel ${f.nivel}, cap fundación ${propios}/${cap} · Rey: ${f.reyId ?? '—'} · Embajador: ${f.embajadorId ?? '—'} · Ciudadanos: ${f.ciudadanosIds.length} · Reputación: ${f.reputacion.toFixed(0)}</div>`;
    })
    .join('');
  const relH = state.relaciones
    .map((r) => {
      const a = state.facciones.find((f) => f.id === r.faccionAId)?.nombre ?? r.faccionAId;
      const b = state.facciones.find((f) => f.id === r.faccionBId)?.nombre ?? r.faccionBId;
      const trib = r.tributo ? ` (tributo ${r.tributo.cantidadPorTick}/tick ${r.tributo.recurso})` : '';
      return `<div>${r.tipo} [${r.estado}]: ${a} → ${b}${trib}</div>`;
    })
    .join('');
  const ligas = gameStore.getLigas();
  const ligasH = ligas
    .map((liga, i) => {
      const nombres = liga.miembrosFaccionIds.map((id) => state.facciones.find((f) => f.id === id)?.nombre ?? id).join(', ');
      const granRey = liga.granReyFaccionId ? state.facciones.find((f) => f.id === liga.granReyFaccionId)?.reyId ?? '—' : '—';
      return `<div>Liga ${i + 1}: ${nombres}${liga.tieneVasallaje ? ` — Gran Rey: ${granRey}` : ''}</div>`;
    })
    .join('');
  politicaPanelEl.innerHTML = `${facH}${relH}${ligasH || '<div>Sin Ligas formadas.</div>'}`;
}

function renderPanelMilitar(state: GameState): void {
  militarPanelEl.innerHTML = state.asentamientos
    .map((a) => {
      const tieneFundicion = a.edificios.some((e) => e.tipo === 'fundicion' && e.estado === 'activo');
      const tieneGranFundicion = a.edificios.some((e) => e.tipo === 'granFundicion' && e.estado === 'activo');
      const escuadronesHtml =
        a.escuadrones
          .map(
            (e) =>
              `<div>${e.id} — ${e.nombre} (Tier ${e.tier}, ${e.origen}) · cantidad ${e.cantidad} · veterania ${e.veterania.toFixed(1)} · moral ${e.moral.toFixed(0)}${e.heridoHastaTick ? ` · herido hasta t${e.heridoHastaTick}` : ''}</div>`
          )
          .join('') || '<div>Sin escuadrones.</div>';
      return `<div><strong>${etiquetaAsentamiento(a, state.facciones)}</strong> — Fundición: ${tieneFundicion ? 'sí' : 'no'} · Gran Fundición: ${tieneGranFundicion ? 'sí' : 'no'}${escuadronesHtml}</div>`;
    })
    .join('');
}

function renderPanelProgresion(state: GameState): void {
  progresionPanelEl.innerHTML =
    state.titulos
      .map((t) => `<div><strong>${t.nombre}</strong> — ${state.facciones.find((f) => f.id === t.poseedorId)?.nombre ?? t.poseedorId} (${t.valorMetrica.toFixed(0)})</div>`)
      .join('') || '<div>Sin títulos calculados todavía (avanza un tick).</div>';
}

function renderPanelEconomia(state: GameState): void {
  const preciosHtml = CATALOGOS.recursosMercado.map((r) => `${r}: ${gameStore.precioReferencia(r).toFixed(2)}`).join(' · ');
  const acuerdosHtml = state.acuerdos
    .map(
      (t) =>
        `<div>Trueque ${t.id} [${t.estado}] — A entrega ${t.cantidadEntregadaA.toFixed(0)}/${t.cantidadTotalA} ${t.recursoA}, B entrega ${t.cantidadEntregadaB.toFixed(0)}/${t.cantidadTotalB} ${t.recursoB}</div>`
    )
    .join('');
  const ordenesHtml = state.ordenes
    .map(
      (o) =>
        `<div>Orden ${o.id} [${o.estado}] — ${o.tipo} ${o.cantidadCumplida.toFixed(0)}/${o.cantidad} ${o.recurso} @ ${o.precioUnitario.toFixed(2)} oro</div>`
    )
    .join('');
  economiaPanelEl.innerHTML = `<div><strong>Precios de referencia</strong> — ${preciosHtml}</div>
    <div><strong>Caravanas en tránsito:</strong> ${state.caravanas.length}</div>
    ${acuerdosHtml}
    ${ordenesHtml}`;
}

function renderLeyenda(state: GameState): void {
  // Solo los recursos que realmente se dibujan como punto en el mapa (madera/trigo no tienen nodo propio, Doc 1.4).
  const recursosHtml = RECURSOS_EN_MAPA.map(
    (tipo) => `<div class="legend-row"><span class="swatch" style="background:${RECURSO_COLOR[tipo]}"></span>${RECURSO_NOMBRE[tipo] ?? tipo}</div>`
  ).join('');

  const edificiosHtml = Object.entries(EDIFICIO_COLOR)
    .map(([tipo, color]) => `<div class="legend-row"><span class="swatch-square" style="background:${color}"></span>${EDIFICIO_NOMBRE[tipo] ?? tipo}</div>`)
    .join('');

  const faccionesHtml =
    state.facciones
      .map((f, i) => `<div class="legend-row"><span class="swatch" style="background:${FACCION_COLORES[i % FACCION_COLORES.length]}"></span>${f.nombre}</div>`)
      .join('') || '<div class="legend-note">Sin Facciones.</div>';

  legendBodyEl.innerHTML = `
    <div class="legend-group">
      <h3>Recursos</h3>
      ${recursosHtml}
      <div class="legend-row"><span class="swatch-poly" style="background:rgba(63,125,58,0.4)"></span>Bosque (madera)</div>
    </div>
    <div class="legend-group">
      <h3>Edificios</h3>
      ${edificiosHtml}
      <div class="legend-note">Relleno = activo · mitad transparente = en construcción · solo contorno = en cola</div>
    </div>
    <div class="legend-group">
      <h3>Facciones (asentamiento y zona de influencia)</h3>
      ${faccionesHtml}
    </div>
    <div class="legend-group">
      <h3>Otros</h3>
      <div class="legend-row"><span class="swatch" style="background:#f1e6c8"></span>Caravana en tránsito</div>
    </div>
  `;
}

function renderRegistro(state: GameState): void {
  logEl.innerHTML = state.log.map((e) => `<div>[t${e.tick}] ${e.mensaje}</div>`).join('');
}

function actualizarTabs(): void {
  document.querySelectorAll<HTMLButtonElement>('#main-tabs .tab-btn').forEach((btn) => {
    btn.classList.toggle('active', btn.dataset.tab === tabActivo);
  });
  document.getElementById('tab-acciones')!.hidden = tabActivo !== 'acciones';
  document.getElementById('tab-asentamientos')!.hidden = tabActivo !== 'asentamientos';
  document.getElementById('tab-jugadores')!.hidden = tabActivo !== 'jugadores';
}

document.getElementById('main-tabs')!.addEventListener('click', (ev) => {
  const btn = (ev.target as HTMLElement).closest('.tab-btn') as HTMLButtonElement | null;
  if (!btn) return;
  tabActivo = btn.dataset.tab as 'acciones' | 'asentamientos' | 'jugadores';
  actualizarTabs();
});
actualizarTabs();

document.getElementById('legend-toggle')!.addEventListener('click', () => {
  legendEl.classList.toggle('collapsed');
  const toggle = document.getElementById('legend-toggle')!;
  toggle.textContent = legendEl.classList.contains('collapsed') ? 'Leyenda ▸' : 'Leyenda ▾';
});

function render(): void {
  const state = gameStore.getState();
  const drawState: DrawState = { world: state.world, asentamientos: state.asentamientos, zonas: gameStore.getZonas(), facciones: state.facciones, caravanas: state.caravanas };
  draw(ctx, canvas, drawState);
  tickCounterEl.textContent = `Tick: ${state.tick}`;
  actualizarSelects(state);
  renderPanelAsentamientos(state);
  renderAsentamientosTab(state);
  renderJugadoresTab(state);
  renderPanelPolitica(state);
  renderPanelProgresion(state);
  renderPanelMilitar(state);
  renderPanelEconomia(state);
  renderLeyenda(state);
  renderRegistro(state);
}

// Única suscripción: cualquier acción del store dispara un re-render. La interfaz nunca
// vuelve a llamar `render()` manualmente tras una acción — eso sería recrear el acoplamiento.
gameStore.subscribe(render);

function idsDeInput(input: HTMLInputElement): string {
  return input.value;
}

canvas.addEventListener('click', (ev) => {
  const state = gameStore.getState();
  const rect = canvas.getBoundingClientRect();
  const scale = state.world.config.ancho / canvas.width;
  const worldX = (ev.clientX - rect.left) * scale;
  const worldY = (ev.clientY - rect.top) * scale;
  gameStore.fundarAsentamiento(faccionSelect.value, { x: worldX, y: worldY }, Number(jugadoresInput.value) || 1);
});

document.getElementById('rey-btn')!.addEventListener('click', () => {
  gameStore.asignarRey(cargoFaccionSelect.value, cargoJugadorInput.value.trim());
});

document.getElementById('embajador-btn')!.addEventListener('click', () => {
  gameStore.asignarEmbajador(cargoFaccionSelect.value, cargoJugadorInput.value.trim());
});

document.getElementById('cargo-local-btn')!.addEventListener('click', () => {
  gameStore.asignarCargoLocal(cargoAsentamientoSelect.value, cargoTipoSelect.value as CargoTipo, cargoJugadorInput.value.trim());
});

document.getElementById('casa-btn')!.addEventListener('click', () => {
  gameStore.comprarCasa(casaAsentamientoSelect.value, casaJugadorInput.value.trim());
});

document.getElementById('politica-btn')!.addEventListener('click', () => {
  gameStore.activarPolitica(politicaAsentamientoSelect.value, politicaCargoSelect.value as CargoTipo, politicaIdSelect.value);
});

document.getElementById('diplo-proponer-btn')!.addEventListener('click', () => {
  gameStore.proponerRelacion(
    diploTipoSelect.value as 'vasallaje' | 'alianza',
    diploASelect.value,
    diploBSelect.value,
    diploTributoRecursoSelect.value,
    Number(diploTributoCantidadInput.value) || 0
  );
});

document.getElementById('diplo-romper-btn')!.addEventListener('click', () => {
  gameStore.romperRelacion(diploRelacionSelect.value, diploASelect.value);
});

document.getElementById('diplo-rebelion-btn')!.addEventListener('click', () => {
  gameStore.rebelionVasallo(diploRelacionSelect.value);
});

document.getElementById('anexion-btn')!.addEventListener('click', () => {
  gameStore.anexionar(fusionASelect.value, fusionBSelect.value);
});

document.getElementById('fusion-btn')!.addEventListener('click', () => {
  gameStore.fusionar(fusionASelect.value, fusionBSelect.value, fusionNombreInput.value.trim(), fusionReyInput.value.trim());
});

document.getElementById('trueque-btn')!.addEventListener('click', () => {
  gameStore.proponerTrueque(
    truequeASelect.value,
    truequeRecursoASelect.value,
    Number(truequeCantidadAInput.value) || 0,
    truequeBSelect.value,
    truequeRecursoBSelect.value,
    Number(truequeCantidadBInput.value) || 0
  );
});

document.getElementById('mercado-btn')!.addEventListener('click', () => {
  const precio = mercadoPrecioInput.value.trim() === '' ? undefined : Number(mercadoPrecioInput.value);
  gameStore.colocarOrdenMercado(
    mercadoAsentamientoSelect.value,
    mercadoTipoSelect.value as 'compra' | 'venta',
    mercadoRecursoSelect.value,
    Number(mercadoCantidadInput.value) || 0,
    precio
  );
});

document.getElementById('reclutar-btn')!.addEventListener('click', () => {
  gameStore.reclutar(guerraAsentamientoSelect.value, reclutarOrigenSelect.value as 'pesants' | 'artesanos' | 'nobleza', Number(reclutarCantidadInput.value) || 0);
});

document.getElementById('fundicion-btn')!.addEventListener('click', () => {
  gameStore.construirManualmente(guerraAsentamientoSelect.value, 'fundicion');
});
document.getElementById('gran-fundicion-btn')!.addEventListener('click', () => {
  gameStore.construirManualmente(guerraAsentamientoSelect.value, 'granFundicion');
});

document.getElementById('asedio-btn')!.addEventListener('click', () => {
  gameStore.iniciarAsedio(guerraAsentamientoSelect.value, guerraObjetivoSelect.value, idsDeInput(guerraEscuadronesInput));
});

document.getElementById('campo-abierto-btn')!.addEventListener('click', () => {
  gameStore.combateCampoAbierto(
    guerraAsentamientoSelect.value,
    idsDeInput(guerraEscuadronesInput),
    guerraObjetivoSelect.value,
    idsDeInput(guerraEscuadronesObjetivoInput)
  );
});

document.getElementById('interceptar-btn')!.addEventListener('click', () => {
  gameStore.interceptarCaravana(guerraAsentamientoSelect.value, idsDeInput(guerraEscuadronesInput), guerraCaravanaSelect.value);
});

document.getElementById('tick-btn')!.addEventListener('click', () => {
  gameStore.avanzarTick();
});

document.getElementById('regenerar-btn')!.addEventListener('click', () => {
  gameStore.regenerarMundo(Number(seedInput.value) || 0);
});

render();
