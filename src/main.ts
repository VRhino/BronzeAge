import type { AcuerdoTrueque, Asentamiento, CargoTipo, Caravana, Faccion, OrdenMercado, RelacionPolitica, Titulo } from './domain/types';
import { POLITICA_CATALOGO, WORLD_DEFAULT } from './constants';
import { generateWorld } from './engine/world';
import { fundarAsentamiento, FundacionInvalidaError } from './engine/settlement';
import { computeTodasLasZonas } from './engine/zones';
import { avanzarSimulacion } from './engine/simulation';
import { proponerTrueque, TruequeInvalidoError } from './engine/trade';
import { colocarOrdenMercado, calcularPrecioReferencia, OrdenInvalidaError } from './engine/market';
import { crearFaccion, comprarCasa, calcularCapFundacion, FaccionInvalidaError } from './engine/faccion';
import { asignarRey, asignarEmbajador, asignarCargoLocal, CargoInvalidoError } from './engine/cargos';
import { activarPolitica, PoliticaInvalidaError } from './engine/politicas';
import { proponerVasallaje, proponerAlianza, romperRelacion, rebelionVasallo, DiplomaciaInvalidaError } from './engine/diplomacia';
import { computeLigas } from './engine/liga';
import { anexionar, fusionar, FusionInvalidaError } from './engine/fusion';
import { reclutar, ReclutamientoInvalidoError } from './engine/tropas';
import { construirManualmente, ConstruccionManualInvalidaError } from './engine/construction';
import { iniciarAsedio, combateCampoAbierto, interceptarCaravana, CombateInvalidoError } from './engine/combate';
import { draw, faccionColor, type DrawState } from './ui/canvas';

const CANVAS_SIZE = 800;
const RECURSOS_TRUEQUE = ['madera', 'piedra', 'trigo', 'cobre', 'estano', 'oro', 'livestock'];
const RECURSOS_MERCADO = ['madera', 'piedra', 'trigo', 'cobre', 'estano', 'livestock']; // oro es la moneda, no cotiza consigo mismo
const CARGOS: CargoTipo[] = ['gobernador', 'tesorero', 'general', 'maestroObras', 'sacerdote'];
const ORIGENES_TROPA = ['pesants', 'artesanos', 'nobleza'] as const;

let world = generateWorld({ ...WORLD_DEFAULT, seed: 1 });
let asentamientos: Asentamiento[] = [];
let facciones: Faccion[] = [crearFaccion('faccion-1', 'Micenas'), crearFaccion('faccion-2', 'Troya'), crearFaccion('faccion-3', 'Ugarit')];
let caravanas: Caravana[] = [];
let acuerdos: AcuerdoTrueque[] = [];
let ordenes: OrdenMercado[] = [];
let relaciones: RelacionPolitica[] = [];
let titulos: Titulo[] = [];
let tick = 0;
let contadorAcciones = 0;

const app = document.getElementById('app')!;
app.innerHTML = `
  <div class="map-panel">
    <canvas id="world-canvas" width="${CANVAS_SIZE}" height="${CANVAS_SIZE}"></canvas>
  </div>
  <div class="side-panel">
    <div class="controls">
      <h1>Bronze Age Collapse — Fase 0</h1>
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

    <h2>Asentamientos</h2>
    <div class="log-panel" id="asentamientos-panel"></div>
    <h2>Política</h2>
    <div class="log-panel" id="politica-panel"></div>
    <h2>Progresión (Doc 2.9)</h2>
    <div class="log-panel" id="progresion-panel"></div>
    <h2>Militar</h2>
    <div class="log-panel" id="militar-panel"></div>
    <h2>Economía</h2>
    <div class="log-panel" id="economia-panel"></div>
    <h2>Registro</h2>
    <div class="log-panel" id="log"></div>
  </div>
`;

const canvas = document.getElementById('world-canvas') as HTMLCanvasElement;
const ctx = canvas.getContext('2d')!;
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

faccionSelect.innerHTML = facciones
  .map((f) => `<option value="${f.id}" style="color:${faccionColor(f.id, facciones)}">${f.nombre}</option>`)
  .join('');
cargoTipoSelect.innerHTML = CARGOS.map((c) => `<option value="${c}">${c}</option>`).join('');
politicaCargoSelect.innerHTML = CARGOS.map((c) => `<option value="${c}">${c}</option>`).join('');
politicaIdSelect.innerHTML = POLITICA_CATALOGO.map((p) => `<option value="${p.id}">${p.nombre} (${p.cargo})</option>`).join('');
diploTributoRecursoSelect.innerHTML = RECURSOS_TRUEQUE.map((r) => `<option value="${r}">${r}</option>`).join('');
truequeRecursoASelect.innerHTML = RECURSOS_TRUEQUE.map((r) => `<option value="${r}">${r}</option>`).join('');
truequeRecursoBSelect.innerHTML = RECURSOS_TRUEQUE.map((r) => `<option value="${r}">${r}</option>`).join('');
mercadoRecursoSelect.innerHTML = RECURSOS_MERCADO.map((r) => `<option value="${r}">${r}</option>`).join('');
reclutarOrigenSelect.innerHTML = ORIGENES_TROPA.map((o) => `<option value="${o}">${o}</option>`).join('');

function log(mensaje: string): void {
  const entry = document.createElement('div');
  entry.textContent = `[t${tick}] ${mensaje}`;
  logEl.prepend(entry);
}

function etiquetaAsentamiento(a: Asentamiento): string {
  const nombreFaccion = facciones.find((f) => f.id === a.faccionId)?.nombre ?? a.faccionId;
  return `${a.id} (${nombreFaccion})`;
}

function actualizarSelects(): void {
  const opcionesAsentamientos = asentamientos.map((a) => `<option value="${a.id}">${etiquetaAsentamiento(a)}</option>`).join('');
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
    if (asentamientos.some((a) => a.id === seleccionPrevia)) select.value = seleccionPrevia;
  }

  const opcionesCaravanas = caravanas
    .map((c) => `<option value="${c.id}">${c.id} (${c.origenAsentamientoId} → ${c.destinoAsentamientoId})</option>`)
    .join('');
  const caravanaPrevia = guerraCaravanaSelect.value;
  guerraCaravanaSelect.innerHTML = opcionesCaravanas;
  if (caravanas.some((c) => c.id === caravanaPrevia)) guerraCaravanaSelect.value = caravanaPrevia;

  const opcionesFacciones = facciones.map((f) => `<option value="${f.id}">${f.nombre}</option>`).join('');
  for (const select of [cargoFaccionSelect, diploASelect, diploBSelect, fusionASelect, fusionBSelect]) {
    const seleccionPrevia = select.value;
    select.innerHTML = opcionesFacciones;
    if (facciones.some((f) => f.id === seleccionPrevia)) select.value = seleccionPrevia;
  }

  const relacionesActivas = relaciones.filter((r) => r.estado === 'activa');
  const seleccionPrevia = diploRelacionSelect.value;
  diploRelacionSelect.innerHTML = relacionesActivas
    .map((r) => {
      const a = facciones.find((f) => f.id === r.faccionAId)?.nombre ?? r.faccionAId;
      const b = facciones.find((f) => f.id === r.faccionBId)?.nombre ?? r.faccionBId;
      return `<option value="${r.id}">${r.tipo}: ${a} → ${b}</option>`;
    })
    .join('');
  if (relacionesActivas.some((r) => r.id === seleccionPrevia)) diploRelacionSelect.value = seleccionPrevia;
}

function renderPanelAsentamientos(): void {
  asentamientosPanelEl.innerHTML = asentamientos
    .map((a) => {
      const { pesants, artesanos, nobleza } = a.poblacion;
      const recursosClave = ['madera', 'piedra', 'trigo', 'oro']
        .map((r) => `${r}: ${Math.floor(a.almacen[r]?.cantidad ?? 0)}/${a.almacen[r]?.capacidad ?? 0}`)
        .join(' · ');
      const activos = a.edificios.filter((e) => e.estado === 'activo').length;
      const enCurso = a.edificios.length - activos;
      const cargosTxt = CARGOS.map((c) => `${c}: ${a.cargos[`${c}Id` as keyof typeof a.cargos] ?? '—'}`).join(' · ');
      return `<div>
        <strong>${etiquetaAsentamiento(a)}</strong> (nivel ${a.nivel}, radio ${Math.round(a.radioPotencial)}) · Mantenimiento: ${a.medidorMantenimiento.toFixed(0)}/100<br/>
        Fundadores: ${a.jugadoresFundadoresIds.join(', ')} · Casas: ${a.casasCompradas.join(', ') || '—'}<br/>
        Cargos — ${cargosTxt}<br/>
        Población — Pesants: ${pesants} · Artesanos: ${artesanos} · Nobleza: ${nobleza}<br/>
        Almacén — ${recursosClave}<br/>
        Edificios — activos: ${activos}, en curso/cola: ${enCurso} · Políticas activas: ${a.politicasActivas.length}
      </div>`;
    })
    .join('');
}

function renderPanelPolitica(): void {
  const facH = facciones
    .map((f) => {
      const cap = calcularCapFundacion(f.nivel);
      const propios = asentamientos.filter((a) => a.faccionId === f.id).length;
      return `<div><strong>${f.nombre}</strong> — nivel ${f.nivel}, cap fundación ${propios}/${cap} · Rey: ${f.reyId ?? '—'} · Embajador: ${f.embajadorId ?? '—'} · Ciudadanos: ${f.ciudadanosIds.length} · Reputación: ${f.reputacion.toFixed(0)}</div>`;
    })
    .join('');
  const relH = relaciones
    .map((r) => {
      const a = facciones.find((f) => f.id === r.faccionAId)?.nombre ?? r.faccionAId;
      const b = facciones.find((f) => f.id === r.faccionBId)?.nombre ?? r.faccionBId;
      const trib = r.tributo ? ` (tributo ${r.tributo.cantidadPorTick}/tick ${r.tributo.recurso})` : '';
      return `<div>${r.tipo} [${r.estado}]: ${a} → ${b}${trib}</div>`;
    })
    .join('');
  const ligas = computeLigas(relaciones, facciones);
  const ligasH = ligas
    .map((liga, i) => {
      const nombres = liga.miembrosFaccionIds.map((id) => facciones.find((f) => f.id === id)?.nombre ?? id).join(', ');
      const granRey = liga.granReyFaccionId ? facciones.find((f) => f.id === liga.granReyFaccionId)?.reyId ?? '—' : '—';
      return `<div>Liga ${i + 1}: ${nombres}${liga.tieneVasallaje ? ` — Gran Rey: ${granRey}` : ''}</div>`;
    })
    .join('');
  politicaPanelEl.innerHTML = `${facH}${relH}${ligasH || '<div>Sin Ligas formadas.</div>'}`;
}

function renderPanelMilitar(): void {
  militarPanelEl.innerHTML = asentamientos
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
      return `<div><strong>${etiquetaAsentamiento(a)}</strong> — Fundición: ${tieneFundicion ? 'sí' : 'no'} · Gran Fundición: ${tieneGranFundicion ? 'sí' : 'no'}${escuadronesHtml}</div>`;
    })
    .join('');
}

function renderPanelProgresion(): void {
  progresionPanelEl.innerHTML =
    titulos
      .map((t) => `<div><strong>${t.nombre}</strong> — ${facciones.find((f) => f.id === t.poseedorId)?.nombre ?? t.poseedorId} (${t.valorMetrica.toFixed(0)})</div>`)
      .join('') || '<div>Sin títulos calculados todavía (avanza un tick).</div>';
}

function renderPanelEconomia(): void {
  const preciosHtml = RECURSOS_MERCADO.map((r) => `${r}: ${calcularPrecioReferencia(r, asentamientos).toFixed(2)}`).join(' · ');
  const acuerdosHtml = acuerdos
    .map(
      (t) =>
        `<div>Trueque ${t.id} [${t.estado}] — A entrega ${t.cantidadEntregadaA.toFixed(0)}/${t.cantidadTotalA} ${t.recursoA}, B entrega ${t.cantidadEntregadaB.toFixed(0)}/${t.cantidadTotalB} ${t.recursoB}</div>`
    )
    .join('');
  const ordenesHtml = ordenes
    .map(
      (o) =>
        `<div>Orden ${o.id} [${o.estado}] — ${o.tipo} ${o.cantidadCumplida.toFixed(0)}/${o.cantidad} ${o.recurso} @ ${o.precioUnitario.toFixed(2)} oro</div>`
    )
    .join('');
  economiaPanelEl.innerHTML = `<div><strong>Precios de referencia</strong> — ${preciosHtml}</div>
    <div><strong>Caravanas en tránsito:</strong> ${caravanas.length}</div>
    ${acuerdosHtml}
    ${ordenesHtml}`;
}

function render(): void {
  const zonas = computeTodasLasZonas(asentamientos);
  const state: DrawState = { world, asentamientos, zonas, facciones, caravanas };
  draw(ctx, canvas, state);
  tickCounterEl.textContent = `Tick: ${tick}`;
  actualizarSelects();
  renderPanelAsentamientos();
  renderPanelPolitica();
  renderPanelProgresion();
  renderPanelMilitar();
  renderPanelEconomia();
}

canvas.addEventListener('click', (ev) => {
  const rect = canvas.getBoundingClientRect();
  const scale = world.config.ancho / canvas.width;
  const worldX = (ev.clientX - rect.left) * scale;
  const worldY = (ev.clientY - rect.top) * scale;
  const faccionId = faccionSelect.value;

  const numJugadores = Math.min(5, Math.max(1, Number(jugadoresInput.value) || 1));
  const jugadoresIds = Array.from({ length: numJugadores }, (_, i) => `jugador-${faccionId}-${i + 1}`);

  try {
    const resultado = fundarAsentamiento(world, facciones, faccionId, { x: worldX, y: worldY }, jugadoresIds, asentamientos, tick);
    asentamientos = [...asentamientos, resultado.asentamiento];
    facciones = resultado.facciones;
    const nombreFaccion = facciones.find((f) => f.id === faccionId)?.nombre ?? faccionId;
    log(`${nombreFaccion} funda asentamiento en (${Math.round(worldX)}, ${Math.round(worldY)}).`);
    render();
  } catch (err) {
    if (err instanceof FundacionInvalidaError) {
      log(`Fundación rechazada: ${err.message}`);
    } else {
      throw err;
    }
  }
});

document.getElementById('rey-btn')!.addEventListener('click', () => {
  try {
    const faccion = facciones.find((f) => f.id === cargoFaccionSelect.value)!;
    facciones = facciones.map((f) => (f.id === faccion.id ? asignarRey(f, cargoJugadorInput.value.trim()) : f));
    log(`${faccion.nombre}: ${cargoJugadorInput.value.trim()} es el nuevo Rey.`);
    render();
  } catch (err) {
    if (err instanceof CargoInvalidoError) log(`Rey rechazado: ${err.message}`);
    else throw err;
  }
});

document.getElementById('embajador-btn')!.addEventListener('click', () => {
  try {
    const faccion = facciones.find((f) => f.id === cargoFaccionSelect.value)!;
    facciones = facciones.map((f) => (f.id === faccion.id ? asignarEmbajador(f, cargoJugadorInput.value.trim()) : f));
    log(`${faccion.nombre}: ${cargoJugadorInput.value.trim()} es el nuevo Embajador.`);
    render();
  } catch (err) {
    if (err instanceof CargoInvalidoError) log(`Embajador rechazado: ${err.message}`);
    else throw err;
  }
});

document.getElementById('cargo-local-btn')!.addEventListener('click', () => {
  try {
    const asentamiento = asentamientos.find((a) => a.id === cargoAsentamientoSelect.value)!;
    const faccion = facciones.find((f) => f.id === asentamiento.faccionId)!;
    const cargo = cargoTipoSelect.value as CargoTipo;
    const actualizado = asignarCargoLocal(asentamiento, faccion, cargo, cargoJugadorInput.value.trim());
    asentamientos = asentamientos.map((a) => (a.id === actualizado.id ? actualizado : a));
    log(`${asentamiento.id}: ${cargoJugadorInput.value.trim()} asignado como ${cargo}.`);
    render();
  } catch (err) {
    if (err instanceof CargoInvalidoError) log(`Cargo rechazado: ${err.message}`);
    else throw err;
  }
});

document.getElementById('casa-btn')!.addEventListener('click', () => {
  try {
    const asentamiento = asentamientos.find((a) => a.id === casaAsentamientoSelect.value)!;
    const resultado = comprarCasa(facciones, asentamiento, casaJugadorInput.value.trim());
    facciones = resultado.facciones;
    asentamientos = asentamientos.map((a) => (a.id === asentamiento.id ? resultado.asentamiento : a));
    log(`${casaJugadorInput.value.trim()} compra casa en ${asentamiento.id} y obtiene ciudadanía.`);
    render();
  } catch (err) {
    if (err instanceof FaccionInvalidaError) log(`Compra de casa rechazada: ${err.message}`);
    else throw err;
  }
});

document.getElementById('politica-btn')!.addEventListener('click', () => {
  try {
    const asentamiento = asentamientos.find((a) => a.id === politicaAsentamientoSelect.value)!;
    const faccion = facciones.find((f) => f.id === asentamiento.faccionId)!;
    const cargo = politicaCargoSelect.value as CargoTipo;
    const actualizado = activarPolitica(asentamiento, faccion, cargo, politicaIdSelect.value, tick, contadorAcciones++);
    asentamientos = asentamientos.map((a) => (a.id === actualizado.id ? actualizado : a));
    log(`${asentamiento.id}: política "${politicaIdSelect.value}" activada por ${cargo}.`);
    render();
  } catch (err) {
    if (err instanceof PoliticaInvalidaError) log(`Política rechazada: ${err.message}`);
    else throw err;
  }
});

document.getElementById('diplo-proponer-btn')!.addEventListener('click', () => {
  try {
    const tipo = diploTipoSelect.value as 'vasallaje' | 'alianza';
    const nueva =
      tipo === 'vasallaje'
        ? proponerVasallaje(
            facciones,
            relaciones,
            diploASelect.value,
            diploBSelect.value,
            diploTributoRecursoSelect.value,
            Number(diploTributoCantidadInput.value) || 0,
            tick,
            contadorAcciones++
          )
        : proponerAlianza(facciones, relaciones, diploASelect.value, diploBSelect.value, tick, contadorAcciones++);
    relaciones = [...relaciones, nueva];
    log(`Relación propuesta: ${nueva.id}.`);
    render();
  } catch (err) {
    if (err instanceof DiplomaciaInvalidaError) log(`Relación rechazada: ${err.message}`);
    else throw err;
  }
});

document.getElementById('diplo-romper-btn')!.addEventListener('click', () => {
  if (!diploRelacionSelect.value) return;
  const resultado = romperRelacion(facciones, relaciones, diploRelacionSelect.value, diploASelect.value);
  facciones = resultado.facciones;
  relaciones = resultado.relaciones;
  log(`Relación ${diploRelacionSelect.value} rota voluntariamente.`);
  render();
});

document.getElementById('diplo-rebelion-btn')!.addEventListener('click', () => {
  try {
    if (!diploRelacionSelect.value) return;
    const resultado = rebelionVasallo(facciones, relaciones, acuerdos, asentamientos, diploRelacionSelect.value);
    facciones = resultado.facciones;
    relaciones = resultado.relaciones;
    acuerdos = resultado.acuerdos;
    for (const e of resultado.eventos) log(e);
    render();
  } catch (err) {
    if (err instanceof DiplomaciaInvalidaError) log(`Rebelión rechazada: ${err.message}`);
    else throw err;
  }
});

document.getElementById('anexion-btn')!.addEventListener('click', () => {
  try {
    const resultado = anexionar(facciones, asentamientos, fusionASelect.value, fusionBSelect.value);
    facciones = resultado.facciones;
    asentamientos = resultado.asentamientos;
    for (const e of resultado.eventos) log(e);
    render();
  } catch (err) {
    if (err instanceof FusionInvalidaError) log(`Anexión rechazada: ${err.message}`);
    else throw err;
  }
});

document.getElementById('fusion-btn')!.addEventListener('click', () => {
  try {
    const resultado = fusionar(
      facciones,
      asentamientos,
      fusionASelect.value,
      fusionBSelect.value,
      fusionNombreInput.value.trim() || 'Facción Fusionada',
      fusionReyInput.value.trim(),
      tick
    );
    facciones = resultado.facciones;
    asentamientos = resultado.asentamientos;
    for (const e of resultado.eventos) log(e);
    render();
  } catch (err) {
    if (err instanceof FusionInvalidaError) log(`Fusión rechazada: ${err.message}`);
    else throw err;
  }
});

document.getElementById('trueque-btn')!.addEventListener('click', () => {
  try {
    const nuevo = proponerTrueque(
      asentamientos,
      truequeASelect.value,
      truequeBSelect.value,
      truequeRecursoASelect.value,
      truequeRecursoBSelect.value,
      Number(truequeCantidadAInput.value) || 0,
      Number(truequeCantidadBInput.value) || 0,
      tick,
      contadorAcciones++
    );
    acuerdos = [...acuerdos, nuevo];
    log(`Trueque propuesto: ${nuevo.id}.`);
    render();
  } catch (err) {
    if (err instanceof TruequeInvalidoError) {
      log(`Trueque rechazado: ${err.message}`);
    } else {
      throw err;
    }
  }
});

document.getElementById('mercado-btn')!.addEventListener('click', () => {
  try {
    const precio = mercadoPrecioInput.value.trim() === '' ? undefined : Number(mercadoPrecioInput.value);
    const nueva = colocarOrdenMercado(
      asentamientos,
      mercadoAsentamientoSelect.value,
      mercadoTipoSelect.value as 'compra' | 'venta',
      mercadoRecursoSelect.value,
      Number(mercadoCantidadInput.value) || 0,
      tick,
      precio,
      contadorAcciones++
    );
    ordenes = [...ordenes, nueva];
    log(`Orden de mercado colocada: ${nueva.id} (${nueva.tipo} ${nueva.cantidad} ${nueva.recurso} @ ${nueva.precioUnitario.toFixed(2)}).`);
    render();
  } catch (err) {
    if (err instanceof OrdenInvalidaError) {
      log(`Orden rechazada: ${err.message}`);
    } else {
      throw err;
    }
  }
});

function idsDeInput(input: HTMLInputElement): string[] {
  return input.value
    .split(',')
    .map((s) => s.trim())
    .filter((s) => s.length > 0);
}

document.getElementById('reclutar-btn')!.addEventListener('click', () => {
  try {
    const asentamiento = asentamientos.find((a) => a.id === guerraAsentamientoSelect.value)!;
    const actualizado = reclutar(
      asentamiento,
      reclutarOrigenSelect.value as 'pesants' | 'artesanos' | 'nobleza',
      Number(reclutarCantidadInput.value) || 0,
      tick,
      contadorAcciones++
    );
    asentamientos = asentamientos.map((a) => (a.id === actualizado.id ? actualizado : a));
    log(`${asentamiento.id}: recluta ${reclutarCantidadInput.value} de ${reclutarOrigenSelect.value}.`);
    render();
  } catch (err) {
    if (err instanceof ReclutamientoInvalidoError) log(`Reclutamiento rechazado: ${err.message}`);
    else throw err;
  }
});

function construirEdificioMilitar(tipo: 'fundicion' | 'granFundicion'): void {
  try {
    const asentamiento = asentamientos.find((a) => a.id === guerraAsentamientoSelect.value)!;
    const faccion = facciones.find((f) => f.id === asentamiento.faccionId)!;
    const zona = computeTodasLasZonas(asentamientos).find((z) => z.asentamientoId === asentamiento.id);
    const actualizado = construirManualmente(asentamiento, faccion, zona?.poligono ?? [], tipo, contadorAcciones++);
    asentamientos = asentamientos.map((a) => (a.id === actualizado.id ? actualizado : a));
    log(`${asentamiento.id}: se encola ${tipo} (construcción manual).`);
    render();
  } catch (err) {
    if (err instanceof ConstruccionManualInvalidaError) log(`Construcción rechazada: ${err.message}`);
    else throw err;
  }
}
document.getElementById('fundicion-btn')!.addEventListener('click', () => construirEdificioMilitar('fundicion'));
document.getElementById('gran-fundicion-btn')!.addEventListener('click', () => construirEdificioMilitar('granFundicion'));

document.getElementById('asedio-btn')!.addEventListener('click', () => {
  try {
    const atacante = asentamientos.find((a) => a.id === guerraAsentamientoSelect.value)!;
    const defensor = asentamientos.find((a) => a.id === guerraObjetivoSelect.value)!;
    const resultado = iniciarAsedio(atacante, defensor, idsDeInput(guerraEscuadronesInput), facciones, relaciones, tick);
    asentamientos = asentamientos.map((a) => {
      if (a.id === resultado.atacante.id) return resultado.atacante;
      if (a.id === resultado.defensor.id) return resultado.defensor;
      return a;
    });
    facciones = resultado.facciones;
    for (const e of resultado.eventos) log(e);
    render();
  } catch (err) {
    if (err instanceof CombateInvalidoError) log(`Asedio rechazado: ${err.message}`);
    else throw err;
  }
});

document.getElementById('campo-abierto-btn')!.addEventListener('click', () => {
  try {
    const asentamientoA = asentamientos.find((a) => a.id === guerraAsentamientoSelect.value)!;
    const asentamientoB = asentamientos.find((a) => a.id === guerraObjetivoSelect.value)!;
    const resultado = combateCampoAbierto(
      asentamientoA,
      idsDeInput(guerraEscuadronesInput),
      asentamientoB,
      idsDeInput(guerraEscuadronesObjetivoInput),
      facciones,
      relaciones,
      tick
    );
    asentamientos = asentamientos.map((a) => {
      if (a.id === resultado.asentamientoA.id) return resultado.asentamientoA;
      if (a.id === resultado.asentamientoB.id) return resultado.asentamientoB;
      return a;
    });
    facciones = resultado.facciones;
    for (const e of resultado.eventos) log(e);
    render();
  } catch (err) {
    if (err instanceof CombateInvalidoError) log(`Combate rechazado: ${err.message}`);
    else throw err;
  }
});

document.getElementById('interceptar-btn')!.addEventListener('click', () => {
  try {
    const atacante = asentamientos.find((a) => a.id === guerraAsentamientoSelect.value)!;
    const caravana = caravanas.find((c) => c.id === guerraCaravanaSelect.value)!;
    const resultado = interceptarCaravana(atacante, idsDeInput(guerraEscuadronesInput), caravana, tick);
    asentamientos = asentamientos.map((a) => (a.id === resultado.atacante.id ? resultado.atacante : a));
    if (resultado.caravanaCapturada) caravanas = caravanas.filter((c) => c.id !== caravana.id);
    for (const e of resultado.eventos) log(e);
    render();
  } catch (err) {
    if (err instanceof CombateInvalidoError) log(`Intercepción rechazada: ${err.message}`);
    else throw err;
  }
});

document.getElementById('tick-btn')!.addEventListener('click', () => {
  tick += 1;
  const resultado = avanzarSimulacion({ asentamientos, facciones, caravanas, acuerdos, ordenes, relaciones, titulos }, world, tick);
  asentamientos = resultado.asentamientos;
  facciones = resultado.facciones;
  caravanas = resultado.caravanas;
  acuerdos = resultado.acuerdos;
  ordenes = resultado.ordenes;
  relaciones = resultado.relaciones;
  titulos = resultado.titulos;
  for (const evento of resultado.eventos) log(evento);
  render();
});

document.getElementById('regenerar-btn')!.addEventListener('click', () => {
  const seed = Number(seedInput.value) || 0;
  world = generateWorld({ ...WORLD_DEFAULT, seed });
  asentamientos = [];
  facciones = [crearFaccion('faccion-1', 'Micenas'), crearFaccion('faccion-2', 'Troya'), crearFaccion('faccion-3', 'Ugarit')];
  caravanas = [];
  acuerdos = [];
  ordenes = [];
  relaciones = [];
  titulos = [];
  tick = 0;
  logEl.innerHTML = '';
  log(`Mundo regenerado con seed ${seed}.`);
  render();
});

log('Mundo generado. Selecciona una facción y haz clic en el mapa para fundar.');
render();
