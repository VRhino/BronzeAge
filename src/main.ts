// Capa de interfaz (DOM/canvas). Regla de frontera: este archivo — y cualquier otro bajo `ui/` —
// solo puede importar de `./app/gameStore` (acciones + estado + suscripción) y de `./ui/canvas`
// (dibujo/color, puramente presentacional). Nunca importa nada de `./engine/*` ni captura errores
// de dominio: eso es responsabilidad exclusiva de `gameStore`. Los tipos de `./domain/types` se
// importan solo como `type` para tipar lo que se lee — no acoplan a ninguna lógica.
import type { Asentamiento, CargoTipo, Faccion } from './domain/types';
import { CATALOGOS, gameStore, type GameState, type CampoBalance } from './app/gameStore';
import { draw, drawFiltroFertilidad, faccionColor, RECURSO_COLOR, RECURSOS_EN_MAPA, EDIFICIO_COLOR, FACCION_COLORES, type DrawState } from './ui/canvas';

const CANVAS_SIZE = 800;

const RECURSO_NOMBRE: Record<string, string> = {
  madera: 'Madera',
  piedra: 'Piedra',
  trigo: 'Trigo',
  cobre: 'Cobre',
  estano: 'Estaño',
  oro: 'Oro',
  livestock: 'Livestock',
  lingoteCobre: 'Lingote de Cobre',
  lingoteEstano: 'Lingote de Estaño',
  lingoteBronce: 'Lingote de Bronce',
  cuero: 'Cuero',
  cueroCurtido: 'Cuero Curtido',
  cueroCalidad: 'Cuero de Calidad',
  armaCobre: 'Arma de Cobre',
  armaBronce: 'Arma de Bronce',
  armaBronceCalidad: 'Arma de Bronce de Calidad',
  armaduraBasica: 'Armadura Básica',
  armaduraIntermedia: 'Armadura Intermedia',
  armaduraBronce: 'Armadura de Bronce',
};

const EDIFICIO_NOMBRE: Record<string, string> = {
  centroUrbano: 'Centro Urbano',
  vivienda: 'Vivienda',
  granja: 'Granja',
  cantera: 'Cantera',
  lenera: 'Leñera',
  almacen: 'Almacén',
  mina: 'Mina de oro',
  minaCobre: 'Mina de cobre',
  minaEstano: 'Mina de estaño',
  corral: 'Corral',
  fundicion: 'Fundición',
  curtiduria: 'Curtiduría',
  armeria: 'Armería',
  carpinteria: 'Carpintería',
  barracon: 'Barracón',
  galeriaDeTiro: 'Galería de Tiro',
  palacio: 'Palacio',
  granFundicion: 'Gran Fundición',
};

const EDIFICIO_FUNCION: Record<string, string> = {
  centroUrbano: 'Marca el centro fundacional del asentamiento. Único: solo se obtiene al fundar, nunca se puede construir después.',
  vivienda: 'Amplía la capacidad de población (15 habitantes c/u).',
  granja: 'Produce trigo (comida) según la fertilidad del suelo donde se ubica.',
  cantera: 'Extrae piedra de un yacimiento cercano hasta agotarlo.',
  lenera: 'Extrae madera de un bosque cercano (no se agota).',
  almacen: 'Amplía la capacidad de almacenamiento de todos los recursos.',
  mina: 'Extrae oro de un yacimiento cercano hasta agotarlo.',
  minaCobre: 'Extrae cobre de un yacimiento cercano hasta agotarlo.',
  minaEstano: 'Extrae estaño de un yacimiento cercano hasta agotarlo.',
  corral: 'Extrae livestock de una manada cercana hasta agotarla.',
  fundicion: 'Fabrica lingotes de cobre/estaño/bronce a partir de mineral. Dispara la aparición de Artesanos si es el primero de su tipo.',
  curtiduria: 'Trata cuero a partir de livestock. Dispara la aparición de Artesanos si es el primero de su tipo.',
  armeria: 'Fabrica armas y armaduras a partir de lingotes y cuero. Dispara la aparición de Artesanos si es el primero de su tipo.',
  carpinteria: 'Recluta armas de asedio y habilita mejoras de otros edificios (Armería/Barracón/Galería de tiro nivel 2, Palacio).',
  barracon: 'Reclutamiento de tropas cuerpo a cuerpo. Solo se construye mientras la política del General esté activa.',
  galeriaDeTiro: 'Reclutamiento de tropas a distancia. Solo se construye mientras la política del General esté activa.',
  palacio: 'Desbloquea la aparición de Nobleza. Solo se construye mientras la política del Gobernador esté activa.',
  granFundicion: 'Edificio militar de élite (colocación manual) — requiere nivel de Facción alto; habilita tropas de Nobleza.',
};

/** Campos de factor que puede traer una política del catálogo (ver CATALOGOS.politicas), con etiqueta legible. */
const FACTOR_LABEL: Record<string, string> = {
  factorConsumoComida: 'Consumo de comida',
  factorCrecimientoNobleza: 'Crecimiento de Nobleza',
  factorTiempoConstruccion: 'Tiempo de construcción',
  factorComisionExterna: 'Comisión de comercio externo',
  factorCostoReclutamiento: 'Costo de reclutamiento',
};

/** Describe en una línea qué mueve una política del catálogo (multiplicador sobre el factor correspondiente,
 * o el efecto especial de campos no multiplicativos como `minimoLenerasPrioritario`). */
function efectoPolitica(politica: (typeof CATALOGOS.politicas)[number]): string {
  const registro = politica as unknown as Record<string, unknown>;
  const efectos: string[] = [];
  if (typeof registro.minimoLenerasPrioritario === 'number') {
    efectos.push(`Prioriza Leñeras: bloquea el resto de auto-construcción hasta tener ${registro.minimoLenerasPrioritario} (activas o en curso)`);
  }
  efectos.push(
    ...Object.keys(FACTOR_LABEL)
      .filter((campo) => typeof registro[campo] === 'number')
      .map((campo) => `${FACTOR_LABEL[campo]} ×${registro[campo]}`)
  );
  return efectos.length ? efectos.join(', ') : 'Sin efecto mecánico modelado todavía (solo flag de postura).';
}

// --- Estado de vista (qué se muestra, no simulación): vive solo aquí, nunca en el store. ---
let tabActivo: 'acciones' | 'asentamientos' | 'jugadores' | 'politicas' | 'balance' = 'acciones';
let asentamientoSeleccionadoId: string | null = null;
let jugadorSeleccionadoId: string | null = null;
/** Tick que el slider de línea de tiempo está mostrando. Sigue al tick en vivo salvo que el usuario arrastre hacia atrás. */
let viewedTick = 0;
let ultimoTickEnVivo = 0;
let mostrarFiltroFertilidad = false;
/** Grupos de la pestaña "Valores de simulación" que el usuario dejó expandidos (persiste solo en esta vista). */
const balanceGruposAbiertos = new Set<string>();
let balanceFiltro = '';

const app = document.getElementById('app')!;
app.innerHTML = `
  <div class="controls-panel">
    <h1>Bronze Age Collapse — Fase 0</h1>
    <div class="tabs" id="main-tabs">
      <button class="tab-btn" data-tab="acciones">Acciones</button>
      <button class="tab-btn" data-tab="asentamientos">Asentamientos</button>
      <button class="tab-btn" data-tab="jugadores">Jugadores</button>
      <button class="tab-btn" data-tab="politicas">Políticas</button>
      <button class="tab-btn" data-tab="balance">Valores de simulación</button>
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
      </div>

      <div class="controls">
        <h2>Cargos (Doc 2.2)</h2>
        <label>Facción <select id="cargo-faccion"></select></label>
        <label>Jugador (ciudadano de la facción) <select id="cargo-jugador"></select></label>
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
        <label>Reclutar tropa (Barracón/Galería de tiro) <select id="reclutar-tropa"></select></label>
        <label>Cantidad de unidades <input id="reclutar-tropa-cantidad" type="number" value="10" min="1" /></label>
        <button id="reclutar-tropa-btn">Reclutar tropa</button>
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

    <div class="tab-panel" id="tab-politicas" hidden>
      <div class="section-title">Catálogo de políticas (Doc 4.4)</div>
      <p class="legend-note">El Gobernador puede activar cualquier política del catálogo completo; el resto de cargos solo las de su propio pool. Duración fija de ${CATALOGOS.duracionPoliticaTicks} ticks, sin cancelación anticipada.</p>
      <div id="politicas-tab" class="controls-grid"></div>
    </div>

    <div class="tab-panel" id="tab-balance" hidden>
      <div class="section-title-row">
        <div class="section-title">Valores de simulación</div>
        <button type="button" id="restaurar-balance-btn">Restaurar valores de fábrica</button>
      </div>
      <p class="legend-note">Placeholders de balance (Doc — ver Consideraciones/Preguntas_Abiertas.md). Se editan en caliente: afectan de inmediato a la próxima acción o tick, sin necesidad de regenerar el mundo (salvo el grupo "Mundo"/generación, que solo se aplica al fundar un mundo nuevo).</p>
      <div class="balance-toolbar">
        <input type="search" id="balance-search" placeholder="Buscar campo o grupo…" />
        <span class="balance-count" id="balance-count"></span>
      </div>
      <div id="balance-tab" class="balance-groups"></div>
    </div>
  </div>

  <div class="map-column">
    <div class="map-panel">
      <canvas id="world-canvas" width="${CANVAS_SIZE}" height="${CANVAS_SIZE}"></canvas>
      <label class="fertilidad-toggle"><input type="checkbox" id="fertilidad-checkbox" /> Filtro de fertilidad</label>
      <div class="legend" id="legend">
        <div class="legend-header" id="legend-toggle">Leyenda ▾</div>
        <div class="legend-body" id="legend-body"></div>
      </div>
    </div>

    <div class="section-title-row">
      <div class="section-title">Estado de la simulación</div>
      <div class="time-travel">
        <button type="button" id="regenerar-btn">Regenerar mundo</button>
        <button type="button" id="tick-btn">Avanzar tick</button>
        <span class="time-travel-divider"></span>
        <span id="tick-slider-label">Tick: 0</span>
        <input type="range" id="tick-slider" min="0" max="0" value="0" step="1" disabled />
        <button type="button" id="volver-presente-btn" hidden>Volver al presente</button>
        <span class="time-travel-divider"></span>
        <button type="button" id="exportar-btn">Exportar</button>
        <button type="button" id="importar-btn">Importar</button>
        <input type="file" id="importar-input" accept="application/json,.json" hidden />
      </div>
    </div>
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
const tickSliderEl = document.getElementById('tick-slider') as HTMLInputElement;
const tickSliderLabelEl = document.getElementById('tick-slider-label')!;
const volverPresenteBtn = document.getElementById('volver-presente-btn') as HTMLButtonElement;
const controlsPanelEl = document.querySelector('.controls-panel')!;
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
const cargoJugadorSelect = document.getElementById('cargo-jugador') as HTMLSelectElement;
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
const reclutarTropaSelect = document.getElementById('reclutar-tropa') as HTMLSelectElement;
const reclutarTropaCantidadInput = document.getElementById('reclutar-tropa-cantidad') as HTMLInputElement;
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
reclutarTropaSelect.innerHTML = CATALOGOS.tropasReclutables
  .map((t) => {
    const costoTxt = Object.entries(t.costoEquipo)
      .map(([r, c]) => `${c} ${RECURSO_NOMBRE[r] ?? r}`)
      .join(' + ');
    return `<option value="${t.id}">${t.nombre} — ${EDIFICIO_NOMBRE[t.edificio] ?? t.edificio} nivel ${t.nivelRequerido} (${costoTxt})</option>`;
  })
  .join('');

function etiquetaAsentamiento(a: Asentamiento, facciones: Faccion[]): string {
  const nombreFaccion = facciones.find((f) => f.id === a.faccionId)?.nombre ?? a.faccionId;
  return `${a.id} (${nombreFaccion})`;
}

/** El combo "Jugador" de Cargos solo ofrece ciudadanos de la Facción elegida en el combo de al lado. */
function actualizarCargoJugadorSelect(state: GameState): void {
  const faccion = state.facciones.find((f) => f.id === cargoFaccionSelect.value);
  const ciudadanos = faccion?.ciudadanosIds ?? [];
  const seleccionPrevia = cargoJugadorSelect.value;
  cargoJugadorSelect.innerHTML = ciudadanos.length
    ? ciudadanos.map((id) => `<option value="${id}">${id}</option>`).join('')
    : '<option value="">Sin ciudadanos en esta facción</option>';
  if (ciudadanos.includes(seleccionPrevia)) cargoJugadorSelect.value = seleccionPrevia;
}

cargoFaccionSelect.addEventListener('change', () => actualizarCargoJugadorSelect(gameStore.getState()));

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

  actualizarCargoJugadorSelect(state);

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

  // Muestra TODOS los recursos presentes en el almacén (incluidos los intermedios de crafting del rediseño
  // de progreso, Doc 4.2.1) — no solo los tradeables de CATALOGOS.recursosTrueque, que se quedan cortos aquí
  // adrede (esos intermedios no son comerciables en Fase 0).
  const almacenHtml = Object.keys(a.almacen)
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

  // La cola de construcción es única y compartida por TODO el asentamiento (no una por tipo de edificio):
  // el motor evalúa todos los "en_cola" en el mismo orden del array cada tick (ver Doc engine/construction.ts).
  const colaGlobal = a.edificios.filter((e) => e.estado === 'en_cola');
  const posicionEnCola = new Map(colaGlobal.map((e, i) => [e.id, i + 1]));

  const edificiosPorTipo = new Map<string, { activos: number; enConstruccion: number[]; enCola: number[] }>();
  for (const e of a.edificios) {
    const entry = edificiosPorTipo.get(e.tipo) ?? { activos: 0, enConstruccion: [], enCola: [] };
    if (e.estado === 'activo') entry.activos += 1;
    else if (e.estado === 'en_construccion') entry.enConstruccion.push(e.ticksRestantes);
    else entry.enCola.push(posicionEnCola.get(e.id)!);
    edificiosPorTipo.set(e.tipo, entry);
  }
  const edificiosHtml = edificiosPorTipo.size
    ? `<table class="mini-table">
        <thead><tr><th>Edificio</th><th>Función</th><th>Activos</th><th>En construcción</th><th>En cola (${colaGlobal.length}/${CATALOGOS.maximoEdificiosEnCola})</th></tr></thead>
        <tbody>
          ${Array.from(edificiosPorTipo.entries())
            .map(([tipo, e]) => {
              const construccionTxt = e.enConstruccion.length
                ? e.enConstruccion.map((t) => `${t}t`).join(', ')
                : '—';
              const colaTxt = e.enCola.length ? e.enCola.map((p) => `#${p} de ${colaGlobal.length}`).join(', ') : '—';
              return `<tr><td>${EDIFICIO_NOMBRE[tipo] ?? tipo}</td><td>${EDIFICIO_FUNCION[tipo] ?? '—'}</td><td>${e.activos}</td><td>${construccionTxt}</td><td>${colaTxt}</td></tr>`;
            })
            .join('')}
        </tbody>
      </table>`
    : '<p class="legend-note">Sin edificios.</p>';

  const produccion = gameStore.produccionInfo(a);
  const manoObra = gameStore.manoObraInfo(a);
  const produccionHtml = produccion.length
    ? `<table class="mini-table">
        <thead><tr><th>Edificio</th><th>Activos</th><th>Recurso</th><th>Producción/tick</th></tr></thead>
        <tbody>
          ${produccion
            .map(
              (p) =>
                `<tr><td>${EDIFICIO_NOMBRE[p.tipo] ?? p.tipo}</td><td>${p.activos}</td><td>${RECURSO_NOMBRE[p.recurso] ?? p.recurso}</td><td>${p.cantidadPorTick.toFixed(1)}</td></tr>`
            )
            .join('')}
        </tbody>
      </table>`
    : '<p class="legend-note">Sin edificios productores activos.</p>';

  const politicasActivasHtml = a.politicasActivas.length
    ? `<table class="mini-table">
        <thead><tr><th>Política</th><th>Cargo</th><th>Efecto</th><th>Expira</th></tr></thead>
        <tbody>
          ${a.politicasActivas
            .map((p) => {
              const def = CATALOGOS.politicas.find((c) => c.id === p.politicaId);
              return `<tr><td>${def?.nombre ?? p.politicaId}</td><td>${p.cargo}</td><td>${def ? efectoPolitica(def) : '—'}</td><td>t${p.expiraEnTick}</td></tr>`;
            })
            .join('')}
        </tbody>
      </table>`
    : '<p class="legend-note">Sin políticas activas.</p>';

  const slotsHtml = CATALOGOS.cargos
    .map((c) => {
      const activasDeCargo = a.politicasActivas.filter((p) => p.cargo === c).length;
      const limite = gameStore.slotsPoliticaDisponibles(c, faccion?.nivel ?? 1);
      return `<div class="kv-row"><span>${c}</span><span>${activasDeCargo}/${limite}</span></div>`;
    })
    .join('');
  const politicasHtml = `${politicasActivasHtml}<div class="kv-grid" style="margin-top:8px">${slotsHtml}</div>`;

  const nivelInfo = gameStore.nivelAsentamientoInfo(a);
  let nivelPorcentaje = 100;
  let nivelTexto = 'Nivel máximo';
  if (!nivelInfo.esMaximo && nivelInfo.siguiente) {
    const s = nivelInfo.siguiente;
    const ratioPesants = Math.min(1, s.pesants.actual / s.pesants.requerido);
    const ratioArtesanos = Math.min(1, s.artesanos.actual / s.artesanos.requerido);
    const edificiosListos = s.edificiosRequeridos - s.edificiosFaltantes.length;
    const ratioEdificios = s.edificiosRequeridos > 0 ? edificiosListos / s.edificiosRequeridos : 1;
    nivelPorcentaje = Math.round(((ratioPesants + ratioArtesanos + ratioEdificios) / 3) * 100);
    const faltantesTexto = s.edificiosFaltantes.length
      ? `; faltan: ${s.edificiosFaltantes.map((tipo) => EDIFICIO_NOMBRE[tipo] ?? tipo).join(', ')}`
      : '';
    nivelTexto = `Nivel ${s.nivelObjetivo}: ${s.pesants.actual}/${s.pesants.requerido} pesants, ${s.artesanos.actual}/${s.artesanos.requerido} artesanos${faltantesTexto}`;
  }

  const mantenimiento = gameStore.mantenimientoInfo(a);
  const mantenimientoHtml = mantenimiento.enGracia
    ? `<p class="legend-note">En periodo de gracia (recién fundado): sin coste todavía — ${mantenimiento.ticksParaFinGracia} ticks restantes.</p>`
    : mantenimiento.items.length
      ? `<table class="mini-table">
          <thead><tr><th>Recurso</th><th>Coste/tick</th><th>Disponible</th></tr></thead>
          <tbody>
            ${mantenimiento.items
              .map(
                (i) =>
                  `<tr class="${i.cubierto ? '' : 'fila-deficit'}"><td>${RECURSO_NOMBRE[i.recurso] ?? i.recurso}</td><td>${i.costoPorTick.toFixed(1)}</td><td>${i.disponible.toFixed(0)}</td></tr>`
              )
              .join('')}
          </tbody>
        </table>`
      : '<p class="legend-note">Sin coste de mantenimiento.</p>';

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
        <div class="kv-row" style="margin-top:6px"><span>Progreso de nivel</span><span>${nivelTexto}</span></div>
        <div class="mantenimiento-bar"><div class="mantenimiento-fill" style="width:${nivelPorcentaje}%"></div></div>
        <div class="kv-row" style="margin-top:6px"><span>Mantenimiento</span><span>${a.medidorMantenimiento.toFixed(0)}/100</span></div>
        <div class="mantenimiento-bar"><div class="mantenimiento-fill" style="width:${Math.max(0, Math.min(100, a.medidorMantenimiento))}%"></div></div>
      </div>

      <div class="detail-section">
        <h3>Mantenimiento — consumo por tick</h3>
        ${mantenimientoHtml}
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
        <h3>Mano de obra en producción primaria</h3>
        <div class="kv-grid">
          <div class="kv-row"><span>Puestos requeridos</span><span>${manoObra.trabajadoresRequeridos}</span></div>
          <div class="kv-row"><span>Pesants ocupados</span><span>${manoObra.ocupados.toFixed(0)}</span></div>
          <div class="kv-row"><span>Pesants excedentes</span><span>${manoObra.excedente.toFixed(0)}</span></div>
          <div class="kv-row"><span>Ratio de mano de obra</span><span>${Math.round(manoObra.ratioMano * 100)}%</span></div>
        </div>
        ${
          manoObra.ratioMano < 1
            ? `<p class="legend-note">Faltan pesants: la producción de granja/cantera/leñera/minas se reduce a un ${Math.round(manoObra.ratioMano * 100)}% del máximo.</p>`
            : manoObra.excedente > 0
              ? `<p class="legend-note">Pool de pesants para reclutamiento: ${manoObra.excedente.toFixed(0)}.</p>`
              : ''
        }
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
        <h3>Producción — por tick</h3>
        ${produccionHtml}
      </div>

      <div class="detail-section">
        <h3>Políticas activas y slots por cargo</h3>
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
      render();
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
      render();
    });
  });
}

function renderPoliticasTab(): void {
  const cont = document.getElementById('politicas-tab')!;
  const gobernadorSlots = CATALOGOS.slotsPorCargoBase.gobernador;
  const notaGobernador = `<div class="controls">
    <h2>Gobernador</h2>
    <p class="legend-note">Pool completa: puede activar cualquier política del catálogo, no solo las de su propio cargo.</p>
    <div class="kv-row"><span>Slots simultáneos</span><span>${gobernadorSlots.base}–${gobernadorSlots.maximo} (escala +1 cada ${CATALOGOS.nivelFaccionPorSlotExtraGobernador} niveles de Facción)</span></div>
  </div>`;

  const tarjetasPolitica = CATALOGOS.politicas
    .map((p) => {
      const slots = CATALOGOS.slotsPorCargoBase[p.cargo as keyof typeof CATALOGOS.slotsPorCargoBase];
      return `<div class="controls">
        <h2>${p.nombre}</h2>
        <div class="kv-row"><span>Cargo</span><span>${p.cargo}</span></div>
        <div class="kv-row"><span>Slots simultáneos (${p.cargo})</span><span>${slots.base} (fijo)</span></div>
        <div class="kv-row"><span>Duración</span><span>${CATALOGOS.duracionPoliticaTicks} ticks</span></div>
        <p class="legend-note">${efectoPolitica(p)}</p>
      </div>`;
    })
    .join('');

  cont.innerHTML = notaGobernador + tarjetasPolitica;
}

/** Agrupa los campos de balance tal como los expone el store, preservando su orden de aparición. */
function agruparPorGrupo(campos: CampoBalance[]): Map<string, CampoBalance[]> {
  const grupos = new Map<string, CampoBalance[]>();
  for (const campo of campos) {
    const lista = grupos.get(campo.grupo) ?? [];
    lista.push(campo);
    grupos.set(campo.grupo, lista);
  }
  return grupos;
}

function renderBalanceTab(): void {
  const cont = document.getElementById('balance-tab')!;
  const countEl = document.getElementById('balance-count')!;
  const todosLosCampos = gameStore.getBalance();
  const filtro = balanceFiltro.trim().toLowerCase();
  const grupos = agruparPorGrupo(todosLosCampos);

  let camposCoincidentes = 0;
  const gruposHtml = Array.from(grupos.entries())
    .map(([grupo, campos]) => {
      const camposFiltrados = filtro
        ? campos.filter((c) => c.etiqueta.toLowerCase().includes(filtro) || grupo.toLowerCase().includes(filtro))
        : campos;
      if (camposFiltrados.length === 0) return '';
      camposCoincidentes += camposFiltrados.length;

      const modificados = campos.filter((c) => c.valor !== c.defecto).length;
      const abierto = filtro !== '' || balanceGruposAbiertos.has(grupo);

      const camposHtml = camposFiltrados
        .map((c) => {
          const modificado = c.valor !== c.defecto;
          return `<div class="balance-field${modificado ? ' is-modified' : ''}">
            <label>
              <span class="balance-field-label">${c.etiqueta}</span>
              <span class="balance-field-row">
                <input type="number" step="any" data-path="${c.path}" value="${c.valor}" title="Valor de fábrica: ${c.defecto}" />
                ${modificado ? `<button type="button" class="balance-field-reset" data-reset-path="${c.path}" data-default="${c.defecto}" title="Restaurar a ${c.defecto}">↺</button>` : ''}
              </span>
            </label>
          </div>`;
        })
        .join('');

      return `<details class="balance-group" data-grupo="${grupo}"${abierto ? ' open' : ''}>
        <summary>
          <span class="grupo-nombre">${grupo}</span>
          <span class="badge">${campos.length}</span>
          ${modificados ? `<span class="badge badge-modified">${modificados} modificado${modificados > 1 ? 's' : ''}</span>` : ''}
        </summary>
        <div class="balance-fields">${camposHtml}</div>
      </details>`;
    })
    .join('');

  cont.innerHTML = gruposHtml || '<p class="legend-note">Sin coincidencias para el filtro actual.</p>';
  countEl.textContent = filtro
    ? `${camposCoincidentes} de ${todosLosCampos.length} campos coinciden`
    : `${todosLosCampos.length} campos en ${grupos.size} grupos`;

  cont.querySelectorAll<HTMLDetailsElement>('.balance-group').forEach((el) => {
    el.addEventListener('toggle', () => {
      const grupo = el.dataset.grupo!;
      if (el.open) balanceGruposAbiertos.add(grupo);
      else balanceGruposAbiertos.delete(grupo);
    });
  });
}

document.getElementById('balance-tab')!.addEventListener('change', (ev) => {
  const input = ev.target as HTMLInputElement;
  const path = input.dataset.path;
  if (!path) return;
  gameStore.actualizarBalance(path, Number(input.value));
  renderBalanceTab();
});

document.getElementById('balance-tab')!.addEventListener('click', (ev) => {
  const btn = (ev.target as HTMLElement).closest('.balance-field-reset') as HTMLButtonElement | null;
  if (!btn) return;
  const path = btn.dataset.resetPath;
  const defecto = btn.dataset.default;
  if (!path || defecto === undefined) return;
  gameStore.actualizarBalance(path, Number(defecto));
  renderBalanceTab();
});

const balanceSearchInput = document.getElementById('balance-search') as HTMLInputElement;
balanceSearchInput.addEventListener('input', () => {
  balanceFiltro = balanceSearchInput.value;
  renderBalanceTab();
});

document.getElementById('restaurar-balance-btn')!.addEventListener('click', () => {
  gameStore.restaurarBalance();
  renderBalanceTab();
});

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
  const ligas = gameStore.getLigas(state.relaciones, state.facciones);
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
  const preciosHtml = CATALOGOS.recursosMercado.map((r) => `${r}: ${gameStore.precioReferencia(r, state.asentamientos).toFixed(2)}`).join(' · ');
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
  document.getElementById('tab-politicas')!.hidden = tabActivo !== 'politicas';
  document.getElementById('tab-balance')!.hidden = tabActivo !== 'balance';
  if (tabActivo === 'balance') renderBalanceTab();
  if (tabActivo === 'politicas') renderPoliticasTab();
}

document.getElementById('main-tabs')!.addEventListener('click', (ev) => {
  const btn = (ev.target as HTMLElement).closest('.tab-btn') as HTMLButtonElement | null;
  if (!btn) return;
  tabActivo = btn.dataset.tab as 'acciones' | 'asentamientos' | 'jugadores' | 'politicas' | 'balance';
  actualizarTabs();
});
actualizarTabs();

document.getElementById('legend-toggle')!.addEventListener('click', () => {
  legendEl.classList.toggle('collapsed');
  const toggle = document.getElementById('legend-toggle')!;
  toggle.textContent = legendEl.classList.contains('collapsed') ? 'Leyenda ▸' : 'Leyenda ▾';
});

document.getElementById('fertilidad-checkbox')!.addEventListener('change', (ev) => {
  mostrarFiltroFertilidad = (ev.target as HTMLInputElement).checked;
  render();
});

function render(): void {
  const liveState = gameStore.getState();

  // El tick en vivo solo puede avanzar mientras se está viendo el presente (las acciones se
  // deshabilitan en el pasado, ver más abajo), así que si cambió, el slider lo sigue automáticamente.
  if (liveState.tick !== ultimoTickEnVivo) {
    ultimoTickEnVivo = liveState.tick;
    viewedTick = liveState.tick;
  }

  const viendoPasado = viewedTick !== liveState.tick;
  const state = viendoPasado ? (gameStore.getSnapshot(viewedTick) ?? liveState) : liveState;

  const rangoTicks = gameStore.getTickRange();
  tickSliderEl.min = String(rangoTicks.min);
  tickSliderEl.max = String(rangoTicks.max);
  tickSliderEl.value = String(viewedTick);
  tickSliderEl.disabled = rangoTicks.min === rangoTicks.max;
  tickSliderLabelEl.textContent = viendoPasado ? `Viendo tick ${viewedTick} de ${liveState.tick}` : `Tick: ${liveState.tick}`;
  volverPresenteBtn.hidden = !viendoPasado;
  controlsPanelEl.classList.toggle('viendo-pasado', viendoPasado);

  const drawState: DrawState = { world: state.world, asentamientos: state.asentamientos, zonas: gameStore.getZonas(state.asentamientos), facciones: state.facciones, caravanas: state.caravanas };
  draw(ctx, canvas, drawState);
  if (mostrarFiltroFertilidad) drawFiltroFertilidad(ctx, canvas, state.world);
  actualizarSelects(liveState);
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

tickSliderEl.addEventListener('input', () => {
  viewedTick = Number(tickSliderEl.value);
  render();
});

volverPresenteBtn.addEventListener('click', () => {
  viewedTick = gameStore.getState().tick;
  render();
});

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
  gameStore.asignarRey(cargoFaccionSelect.value, cargoJugadorSelect.value);
});

document.getElementById('embajador-btn')!.addEventListener('click', () => {
  gameStore.asignarEmbajador(cargoFaccionSelect.value, cargoJugadorSelect.value);
});

document.getElementById('cargo-local-btn')!.addEventListener('click', () => {
  gameStore.asignarCargoLocal(cargoAsentamientoSelect.value, cargoTipoSelect.value as CargoTipo, cargoJugadorSelect.value);
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
  gameStore.reclutar(guerraAsentamientoSelect.value, reclutarOrigenSelect.value as 'artesanos' | 'nobleza', Number(reclutarCantidadInput.value) || 0);
});

document.getElementById('reclutar-tropa-btn')!.addEventListener('click', () => {
  gameStore.reclutarTropa(guerraAsentamientoSelect.value, reclutarTropaSelect.value, Number(reclutarTropaCantidadInput.value) || 0);
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

document.getElementById('exportar-btn')!.addEventListener('click', () => {
  const json = gameStore.exportarSimulacion();
  const blob = new Blob([json], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const enlace = document.createElement('a');
  enlace.href = url;
  enlace.download = `bronze-age-sim-tick${gameStore.getState().tick}.json`;
  enlace.click();
  URL.revokeObjectURL(url);
});

const importarInput = document.getElementById('importar-input') as HTMLInputElement;
document.getElementById('importar-btn')!.addEventListener('click', () => {
  importarInput.click();
});
importarInput.addEventListener('change', () => {
  const archivo = importarInput.files?.[0];
  importarInput.value = '';
  if (!archivo) return;
  const lector = new FileReader();
  lector.onload = () => {
    gameStore.importarSimulacion(String(lector.result));
  };
  lector.readAsText(archivo);
});

render();
