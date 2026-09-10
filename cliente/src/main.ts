// Capa de interfaz (DOM/canvas). Regla de frontera: este archivo — y cualquier otro bajo `ui/` —
// solo puede importar de `./app/gameStore` (acciones + estado + suscripción) y de `./ui/canvas`
// (dibujo/color, puramente presentacional). Nunca importa nada de `./engine/*` ni captura errores
// de dominio: eso es responsabilidad exclusiva de `gameStore`. Los tipos de `./domain/types` se
// importan solo como `type` para tipar lo que se lee — no acoplan a ninguna lógica.
import type { Asentamiento, BiomaTipo, CargoTipo, Edificio, Faccion, RegionId } from '@motor/domain/types';
import { CATALOGOS, crearGameStore, fmtTiempoMundo, type GameState, type GameStore, type EstadoMejoraEdificio } from './app/gameStore';
import { draw, drawAsentamiento, drawFiltroFertilidad, drawTerreno, faccionColor, BIOMA_COLOR, BIOMA_COLOR_SIMPLE, RECURSO_COLOR, RECURSOS_EN_MAPA, EDIFICIO_COLOR, FACCION_COLORES, type DrawState } from './ui/canvas';

// Subido de 800 a 900 junto con el mapa 2000x2000 (Fase 0.1): el mundo más grande necesitaba algo más de
// resolución física para que la capa de terreno/ríos no perdiera nitidez.
const CANVAS_SIZE = 900;

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
  armaMadera: 'Arma de Madera',
  armaCobre: 'Arma de Cobre',
  armaBronce: 'Arma de Bronce',
  armaBronceCalidad: 'Arma de Bronce de Calidad',
  armaduraBasica: 'Armadura Básica',
  armaduraIntermedia: 'Armadura Intermedia',
  armaduraBronce: 'Armadura de Bronce',
};

/** Regiones geográficas disponibles (Fase 0.2, ver `worldgen/regiones.ts`) — nombre para el selector del
 * mundo. Mantenido a mano, igual que `BIOMA_NOMBRE`/`EDIFICIO_NOMBRE`: es presentación pura, no se deriva de
 * `worldgen/` (este archivo no puede importar de ahí, ver la nota de frontera arriba). */
const REGION_NOMBRE: Record<RegionId, string> = {
  greciaContinental: 'Grecia continental',
  anatolia: 'Anatolia',
  egeo: 'Egeo (archipiélago)',
  nilo: 'Nilo',
  mesopotamia: 'Mesopotamia',
};

/** Biomas (Fase 0.1) en orden de elevación creciente — así la leyenda se lee como una escala de altura.
 * Usado con el toggle "Detalle de biomas" activado (`mostrarDetalleBiomas`, ver `BIOMA_COLOR`). */
const BIOMA_NOMBRE: [BiomaTipo, string][] = [
  ['agua', 'Agua (lagos y cauces)'],
  ['costa', 'Ribera'],
  ['estepa', 'Estepa (llano seco)'],
  ['llanuraFertil', 'Llanura fértil'],
  ['colina', 'Colina'],
  ['montana', 'Montaña'],
  ['cima', 'Cima (inhabitable)'],
];

/** Leyenda con el toggle de detalle DESACTIVADO (por defecto): solo las tres bandas que cambian qué se
 * puede hacer en el terreno; costa/estepa/llanuraFertil/colina se resumen en una única fila (ver
 * `BIOMA_COLOR_SIMPLE`). */
const BIOMA_NOMBRE_SIMPLE: [BiomaTipo, string][] = [
  ['agua', 'Agua (lagos y cauces)'],
  ['costa', 'Tierra habitable'],
  ['montana', 'Montaña'],
  ['cima', 'Cima (inhabitable)'],
];

const RECURSO_ICONO: Record<string, string> = {
  madera: '🪵', piedra: '🪨', trigo: '🌾', cobre: '🟠', estano: '⚙️', oro: '🪙', livestock: '🐄',
  lingoteCobre: '🔶', lingoteEstano: '🔩', lingoteBronce: '🟫', cuero: '🟤', cueroCurtido: '🧵',
  cueroCalidad: '✨', armaMadera: '🏹', armaCobre: '🗡️', armaBronce: '⚔️', armaBronceCalidad: '🛡️',
  armaduraBasica: '🥋', armaduraIntermedia: '🛡️', armaduraBronce: '🛡️',
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
  mercado: 'Mercado',
  puestoMercado: 'Puesto de mercado',
  maravilla: 'Maravilla',
  muralla: 'Muralla',
  plaza: 'Plaza',
  plazaDeArmas: 'Plaza de Armas',
  patioDeGremios: 'Patio de Gremios',
  tallerCarpinteria: 'Taller de carpintería',
  pozo: 'Pozo',
  parque: 'Parque',
};

const EDIFICIO_FUNCION: Record<string, string> = {
  centroUrbano: 'Marca el centro fundacional del asentamiento. Único: solo se obtiene al fundar, nunca se puede construir después.',
  vivienda: 'Amplía la capacidad de población: 15 cupos de Pesants + 5 de Artesanos c/u (cupos separados).',
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
  armeria: 'Fabrica armas y armaduras a partir de lingotes y cuero, además de armas de madera en bruto (escalón de entrada, sin metalurgia). Dispara la aparición de Artesanos si es el primero de su tipo.',
  carpinteria: 'Recluta armas de asedio y habilita mejoras de otros edificios (Armería/Barracón/Galería de tiro nivel 2, Palacio).',
  barracon: 'Reclutamiento de tropas cuerpo a cuerpo. Solo se añade a la cola manualmente (Gobernador/Maestro de Obras).',
  galeriaDeTiro: 'Reclutamiento de tropas a distancia. Solo se añade a la cola manualmente (Gobernador/Maestro de Obras).',
  palacio: 'Desbloquea la aparición de Nobleza. Solo se añade a la cola manualmente (Gobernador/Maestro de Obras).',
  granFundicion: 'Edificio militar de élite — requiere nivel de Facción alto; habilita tropas de Nobleza. Solo se añade a la cola manualmente (Gobernador/Maestro de Obras).',
  mercado: 'Exige poder colocar órdenes de mercado y construir caravanas comerciales propias. Su nivel interno fija el cupo de flota. Solo se añade a la cola manualmente (Gobernador/Maestro de Obras).',
  puestoMercado: 'Pieza de la zona de Mercado: no se construye ni cuesta nada, aparece sola al completarse el Mercado y al subir cada nivel interno. Solo ocupa suelo — el cupo de flota lo fija la pieza principal.',
  maravilla: 'Edificio trofeo de coste extremo — requiere asentamiento en nivel máximo (5). Solo se añade a la cola manualmente (Gobernador/Maestro de Obras). El ciclo de servidor que se cerraría al completarla no está implementado todavía.',
  plaza: 'Ancla de saturación del núcleo residencial: no se construye ni cuesta nada, aparece sola cuando el núcleo de Vivienda alrededor del Centro Urbano se llena. Solo ocupa suelo.',
  plazaDeArmas: 'Ancla del núcleo militar: no se construye ni cuesta nada, aparece sola frente al primer edificio militar (Barracón/Galería de tiro/Carpintería) que se construye. Solo ocupa suelo.',
  patioDeGremios: 'Ancla del núcleo de industria: no se construye ni cuesta nada, aparece sola frente al primer edificio de transformación (Fundición/Curtiduría/Armería/Gran Fundición/Maravilla) que se construye. Solo ocupa suelo.',
  tallerCarpinteria: 'Pieza de la zona de Carpintería: no se construye ni cuesta nada, aparecen dos al completarse la Carpintería. Solo ocupa suelo.',
  pozo: 'Ancla de saturación del núcleo residencial (una de tres posibles, sorteada al azar): no se construye ni cuesta nada. Solo ocupa suelo.',
  parque: 'Ancla de saturación del núcleo residencial (una de tres posibles, sorteada al azar): no se construye ni cuesta nada. Solo ocupa suelo.',
};

/** Campos de factor que puede traer una política del catálogo (ver CATALOGOS.politicas), con etiqueta legible. */
const FACTOR_LABEL: Record<string, string> = {
  factorConsumoComida: 'Consumo de comida',
  factorCrecimientoNobleza: 'Crecimiento de Nobleza',
  factorTiempoConstruccion: 'Tiempo de construcción',
  factorComisionExterna: 'Comisión de comercio externo',
  factorCostoReclutamiento: 'Costo de reclutamiento',
  factorProduccionTrigo: 'Producción de trigo',
  factorCapacidadCaravana: 'Capacidad de carga de caravanas',
  factorVelocidadCaravana: 'Velocidad de caravanas',
};

/** Describe en una línea qué mueve una política del catálogo (multiplicador sobre el factor correspondiente,
 * o el efecto especial de campos no multiplicativos como `cupoCaravanaExtra`). */
function efectoPolitica(politica: (typeof CATALOGOS.politicas)[number]): string {
  const registro = politica as unknown as Record<string, unknown>;
  const efectos: string[] = [];
  if (typeof registro.lineasProduccionPriorizadas === 'boolean' && registro.lineasProduccionPriorizadas) {
    efectos.push('Sitúa Fundición/Curtiduría/Armería nuevas cerca de la fuente de sus insumos, no en el primer hueco libre');
  }
  if (typeof registro.cupoCaravanaExtra === 'number') {
    efectos.push(`+${registro.cupoCaravanaExtra} cupo de caravanas`);
  }
  efectos.push(
    ...Object.keys(FACTOR_LABEL)
      .filter((campo) => typeof registro[campo] === 'number')
      .map((campo) => `${FACTOR_LABEL[campo]} ×${registro[campo]}`)
  );
  return efectos.length ? efectos.join(', ') : 'Sin efecto mecánico modelado todavía (solo flag de postura).';
}

// --- Estado de vista (qué se muestra, no simulación): vive solo aquí, nunca en el store. ---
let tabActivo: 'guerra' | 'comercio' | 'asentamientos' | 'facciones' | 'jugadores' | 'politicas' | 'registros' | 'generacionMundo' = 'asentamientos';
let comercioDetalleTab: 'acciones' | 'info' = 'acciones';
let asentamientoSeleccionadoId: string | null = null;
let asentamientoDetalleTab: 'general' | 'edificios' | 'produccion' | 'militar' = 'general';
let reservaProtegidaAbierta = false;
let faccionSeleccionadaId: string | null = null;
let jugadorSeleccionadoId: string | null = null;
/** Filtro de Facción (a petición del usuario): con muchas Facciones, listar el grupo de cada una a la vez
 * dejaba de caber en pantalla — las pestañas Asentamientos/Jugadores filtran a una Facción por combobox. */
let asentamientosFaccionFiltroId: string | null = null;
let jugadoresFaccionFiltroId: string | null = null;
let mostrarFiltroFertilidad = false;
/** Vista del canvas principal (a petición del usuario): 'mundo' = mapa general de siempre; 'asentamiento' =
 * espacio plano local del asentamiento seleccionado (`asentamientoSeleccionadoId`), donde se ve con detalle la
 * ubicación de construcción. Es estado de vista puro, nunca de simulación. */
let vistaMapa: 'mundo' | 'asentamiento' = 'mundo';
/** `false` = paleta de biomas simplificada (agua/montaña/cima distintas, resto de tierra en un solo tono —
 * ver `BIOMA_COLOR_SIMPLE`); `true` = paleta completa de siempre. Por defecto simplificada. */
let mostrarDetalleBiomas = false;
/** Pestaña activa dentro del segmento de leyenda: "mundo" (terreno/recursos/edificios/otros) o "facciones". */
let legendTabActivo: 'mundo' | 'facciones' = 'mundo';
/** Cache de la capa de terreno (Fase 0.1): `drawTerreno` es cara (~900 muestras de bioma) para llamarla en
 * cada `render()` (dispara en cada mousemove sobre el canvas), así que se pinta una vez a un canvas
 * offscreen y se reusa mientras no cambien seed/tamaño/detalle de biomas — es un artefacto de render, no
 * dato de juego. */
let terrenoCache: { key: string; canvas: HTMLCanvasElement } | null = null;

const app = document.getElementById('app')!;
app.innerHTML = `
  <div class="controls-panel">
    <div class="admin-titlebar">
      <h1>Bronze Age Collapse — Fase 0</h1>
      <div class="admin-title-actions" aria-label="Controles de simulación">
        <span id="reloj-mundo" class="reloj-mundo" title="Tiempo de mundo. El servidor avanza solo (1 minuto real = 1 tick); la interfaz se refresca sola cada 5 s.">—</span>
        <button type="button" id="exportar-btn">Exportar</button>
        <button type="button" id="refrescar-btn">Refrescar</button>
      </div>
    </div>
    <div class="tabs" id="main-tabs">
      <button class="tab-btn" data-tab="guerra">Guerra</button>
      <button class="tab-btn" data-tab="comercio">Comercio</button>
      <button class="tab-btn" data-tab="asentamientos">Asentamientos</button>
      <button class="tab-btn" data-tab="facciones">Facción</button>
      <button class="tab-btn" data-tab="jugadores">Jugadores</button>
      <button class="tab-btn" data-tab="politicas">Políticas</button>
      <button class="tab-btn" data-tab="registros">Registros</button>
      <button class="tab-btn" data-tab="generacionMundo">Mundo</button>
    </div>


    <div class="tab-panel" id="tab-guerra" hidden>
    <div class="controls-grid">
      <div class="controls">
        <h2>Catálogo de reclutamiento (Doc 5.7/5.8)</h2>
        <p class="legend-note">Solo consulta — reclutar es del jugador residente, no de administración.</p>
        <label>Tropa (Centro Urbano/Barracón/Galería de tiro) <select id="reclutar-tropa"></select></label>
        <div class="tropa-info" id="reclutar-tropa-info"></div>
      </div>
    </div>

    <div class="detail-section roster-section">
      <h3>Roster de tropas (Doc 5.8)</h3>
      <div id="roster-tropas" class="table-scroll"></div>
    </div>
    </div>

    <div class="tab-panel" id="tab-comercio" hidden>
    <div class="commerce-detail-tabs" role="tablist" aria-label="Secciones de comercio">
      <button type="button" class="commerce-detail-tab active" data-commerce-detail-tab="acciones" role="tab" aria-selected="true">🔍 Detalle</button>
      <button type="button" class="commerce-detail-tab" data-commerce-detail-tab="info" role="tab" aria-selected="false">📈 Información</button>
    </div>

    <div class="commerce-detail-panel active" data-commerce-detail-panel="acciones" role="tabpanel">
    <div class="controls-grid commerce-actions-grid">
      <div class="controls trade-barter-card">
        <h2>Trueque (Doc 3.2)</h2>
        <p class="trade-card-intro">Materiales comerciables de cada asentamiento — solo consulta.</p>
        <div class="trade-side trade-side-a">
          <div class="trade-side-heading"><span>🟦</span><strong>Asentamiento A</strong></div>
        <label>Asentamiento A <select id="trueque-a"></select></label>
        <div class="trade-settlement-info" id="trueque-a-info"></div>
        </div>
        <div class="trade-exchange-mark" aria-hidden="true">⇄</div>
        <div class="trade-side trade-side-b">
          <div class="trade-side-heading"><span>🟧</span><strong>Asentamiento B</strong></div>
        <label>Asentamiento B <select id="trueque-b"></select></label>
        <div class="trade-settlement-info" id="trueque-b-info"></div>
        </div>
      </div>

      <div class="controls trade-card trade-fleet-card">
        <h2>Flota de Caravanas (Doc 3.2, ampliación de comercio)</h2>
        <p class="trade-card-intro">Capacidad logística de la flota propia — solo consulta.</p>
        <label>Asentamiento <select id="flota-asentamiento"></select></label>
        <div class="tropa-info" id="flota-info"></div>
      </div>
    </div>
    </div>

    <div class="commerce-detail-panel" data-commerce-detail-panel="info" role="tabpanel" hidden>
    <div class="detail-section trade-overview">
      <h3>Info de comercio (Doc 3.2/3.3)</h3>
      <div id="economia-panel" class="log-panel"></div>
    </div>
    </div>
    </div>

    <div class="tab-panel" id="tab-asentamientos" hidden>
      <div id="asentamientos-tab"></div>
    </div>

    <div class="tab-panel" id="tab-facciones" hidden>
      <div id="facciones-tab"></div>
    </div>

    <div class="tab-panel" id="tab-jugadores" hidden>
      <div id="jugadores-tab"></div>
    </div>

    <div class="tab-panel" id="tab-politicas" hidden>
      <div class="section-title">Catálogo de políticas (Doc 4.4)</div>
      <p class="legend-note">El Gobernador puede activar cualquier política del catálogo completo; el resto de cargos solo las de su propio pool. Duración fija de ${CATALOGOS.duracionPoliticaMinutos} min de mundo, sin cancelación anticipada.</p>
      <div id="politicas-tab" class="controls-grid"></div>
    </div>

    <div class="tab-panel" id="tab-generacionMundo" hidden>
      <div class="section-title registros-heading">Mundo</div>
      <p class="legend-note registros-intro">Datos de la partida conectada y regeneración del mapa procedural.</p>
      <div class="controls-grid world-generation-grid">
        <div class="controls">
          <h2>Datos de la partida</h2>
          <div class="kv-grid" id="info-partida"></div>
        </div>
        <div class="controls">
          <h2>Regenerar mundo</h2>
          <p class="legend-note">Descarta la partida actual y crea una nueva con la seed y región indicadas — acción destructiva, pide confirmación.</p>
          <label>Seed del mundo <input id="seed-input" type="number" value="1" /></label>
          <label>Región geográfica (Fase 0.2)
            <select id="region-select">
              <option value="">Libre (procedural, sin sesgo)</option>
              ${Object.entries(REGION_NOMBRE)
                .map(([id, nombre]) => `<option value="${id}">${nombre}</option>`)
                .join('')}
            </select>
          </label>
          <button type="button" id="regenerar-btn">Regenerar mundo</button>
        </div>
      </div>
    </div>

    <div class="tab-panel" id="tab-registros" hidden>
      <div class="section-title registros-heading">Registros de la simulación</div>
      <p class="legend-note registros-intro">Información de solo lectura sobre el estado actual del mundo.</p>
      <div class="logs-grid registros-grid">
        <div class="log-card registros-asentamientos-card">
          <h2>Asentamientos</h2>
          <div class="log-panel" id="asentamientos-panel"></div>
        </div>
        <div class="log-card registros-politica-card">
          <h2>Política</h2>
          <div class="log-panel" id="politica-panel"></div>
        </div>
        <div class="log-card registros-progresion-card">
          <h2>Progresión (Doc 2.9)</h2>
          <div class="log-panel" id="progresion-panel"></div>
        </div>
        <div class="log-card registros-militar-card">
          <h2>Militar</h2>
          <div class="log-panel" id="militar-panel"></div>
        </div>
      </div>
    </div>
  </div>

  <div class="map-column">
    <div class="log-card log-card-registro">
      <h2>Registro</h2>
      <div class="log-panel" id="log"></div>
    </div>

    <div class="map-panel">
      <canvas id="world-canvas" width="${CANVAS_SIZE}" height="${CANVAS_SIZE}"></canvas>
      <div id="settlement-building-tooltip" class="settlement-building-tooltip" hidden></div>
      <div class="map-toggles">
        <div class="vista-mapa-toggle" id="vista-mapa-toggle">
          <button type="button" class="vista-btn active" data-vista="mundo">Mundo</button>
          <button type="button" class="vista-btn" data-vista="asentamiento">Asentamiento</button>
        </div>
        <select id="vista-asentamiento-select" hidden></select>
        <label class="fertilidad-toggle vista-mundo-only"><input type="checkbox" id="fertilidad-checkbox" /> Filtro de fertilidad</label>
        <label class="fertilidad-toggle vista-mundo-only"><input type="checkbox" id="biomas-checkbox" /> Detalle de biomas</label>
      </div>
      <div class="legend" id="legend" style="--legend-max-height:${CANVAS_SIZE}px">
        <div class="legend-header" id="legend-toggle">Leyenda ▾</div>
        <div class="legend-tabs" id="legend-tabs">
          <button type="button" class="legend-tab-btn active" data-legend-tab="mundo">Mundo</button>
          <button type="button" class="legend-tab-btn" data-legend-tab="facciones">Facciones</button>
        </div>
        <div class="legend-body" id="legend-body"></div>
      </div>
    </div>

  </div>
`;

const canvas = document.getElementById('world-canvas') as HTMLCanvasElement;
const ctx = canvas.getContext('2d')!;
const mapPanelEl = document.querySelector('.map-panel') as HTMLElement;
const settlementBuildingTooltipEl = document.getElementById('settlement-building-tooltip') as HTMLDivElement;
const legendEl = document.getElementById('legend')!;
const legendBodyEl = document.getElementById('legend-body')!;
const logEl = document.getElementById('log')!;
const relojMundoEl = document.getElementById('reloj-mundo')!;
const asentamientosPanelEl = document.getElementById('asentamientos-panel')!;
const politicaPanelEl = document.getElementById('politica-panel')!;
const economiaPanelEl = document.getElementById('economia-panel')!;
const seedInput = document.getElementById('seed-input') as HTMLInputElement;
const regionSelect = document.getElementById('region-select') as HTMLSelectElement;
const infoPartidaEl = document.getElementById('info-partida')!;

/** Pestaña "Mundo": identifica a qué partida está conectada esta consola. `gameId` sale del estado; el resto
 * lo inyecta `vite.config.ts` desde el entorno del servidor de dev (ver `vite-env.d.ts`). "Local" vs "En la
 * nube" se deduce de si el backend proxeado es localhost. */
function renderInfoPartida(state: GameState): void {
  const backendUrl = import.meta.env.VITE_BACKEND_URL ?? '';
  const esLocal = /^https?:\/\/(localhost|127\.0\.0\.1|\[::1\])(:\d+)?\/?$/i.test(backendUrl);
  const codigo = import.meta.env.VITE_CODIGO_INVITACION ?? '';
  const filas: [string, string][] = [
    ['Nombre', state.gameId],
    ['Ubicación', backendUrl ? (esLocal ? 'Local' : 'En la nube') : '—'],
    ['Backend', backendUrl || '—'],
    ['Proveedor', import.meta.env.VITE_PROVEEDOR_AUTH || 'dev'],
    ['Código de invitación', codigo || '— (registro abierto)'],
  ];
  infoPartidaEl.innerHTML = filas
    .map(([k, v]) => `<div class="kv-row"><span>${k}</span><span>${v}</span></div>`)
    .join('');
}

// Fase C8: el administrador observa, no interactúa como jugador — los selects de abajo solo alimentan
// paneles de SOLO CONSULTA (materiales comerciables, estado de flota, catálogo de reclutamiento). Los
// campos y botones que componían los 27 comandos de rol `jugador` (fundar, comprar casa, diplomacia,
// órdenes de mercado, combate...) se retiraron de la interfaz — no le corresponden a un administrador, ver
// Docs/Arquitectura/5_Contratos_Identidad_Permisos.md "Diferencia entre rol técnico y cargo de juego".
const truequeASelect = document.getElementById('trueque-a') as HTMLSelectElement;
const truequeBSelect = document.getElementById('trueque-b') as HTMLSelectElement;
const truequeAInfoEl = document.getElementById('trueque-a-info')!;
const truequeBInfoEl = document.getElementById('trueque-b-info')!;

const reclutarTropaSelect = document.getElementById('reclutar-tropa') as HTMLSelectElement;
const reclutarTropaInfoEl = document.getElementById('reclutar-tropa-info')!;
const rosterTropasEl = document.getElementById('roster-tropas')!;
const militarPanelEl = document.getElementById('militar-panel')!;
const flotaAsentamientoSelect = document.getElementById('flota-asentamiento') as HTMLSelectElement;
const flotaInfoEl = document.getElementById('flota-info')!;
const progresionPanelEl = document.getElementById('progresion-panel')!;

// Docs/Arquitectura/4_Plan_Evolucion_Tareas.md, Fase B3 — migración de `main.ts`: el cliente de jugador ya no
// posee una `GameSession` en memoria del navegador, se conecta al backend real (`npm run server`). El shell
// estático de arriba (estructura de pestañas, refs de elementos) no depende de esto y ya está en el DOM; lo
// único que espera es la conexión inicial, con `await` de nivel de módulo (soportado por el target ES2022 de
// este proyecto) en vez de envolver el resto del archivo en una función — el resto del archivo sigue leyendo
// `gameStore` tal cual, sin reindentar miles de líneas.
//
// `crearGameStore` crea la partida si todavía no existe para este `gameId` (arranque limpio, cómodo para
// desarrollo) o se conecta a la que ya haya — no es destructivo. DESCARTAR una partida en curso y empezar de
// cero sí lo es (`gameStore.regenerarMundo`, botón "Regenerar mundo" en la pestaña Generación de mundo) — no
// hay separación real todavía entre cliente de jugador y herramienta de administración (una sola interfaz
// sirve a los dos propósitos por ahora, a propósito), así que vive aquí mismo, no en un panel aparte.
logEl.textContent = 'Conectando con el servidor...';
let gameStore: GameStore;
try {
  gameStore = await crearGameStore();
} catch (err) {
  logEl.textContent =
    err instanceof Error ? `No se pudo conectar con el servidor: ${err.message}` : 'No se pudo conectar con el servidor.';
  throw err;
}

reclutarTropaSelect.innerHTML = CATALOGOS.tropasReclutables
  .map((t) => {
    const costoTxt = Object.entries(t.costoEquipo)
      .map(([r, c]) => `${c} ${RECURSO_NOMBRE[r] ?? r}`)
      .join(' + ');
    return `<option value="${t.id}">${t.nombre} — ${edificioRequeridoTxt(t)} (${costoTxt})</option>`;
  })
  .join('');

/** Centro Urbano no tiene nivel interno (nace `activo` con el asentamiento, sin cola ni política, Doc 4.2.1)
 * — a diferencia de Barracón/Galería de tiro, mostrar "nivel 1" ahí sería ruido sin significado real. */
function edificioRequeridoTxt(tropa: (typeof CATALOGOS.tropasReclutables)[number]): string {
  const nombreEdificio = EDIFICIO_NOMBRE[tropa.edificio] ?? tropa.edificio;
  return tropa.edificio === 'centroUrbano' ? nombreEdificio : `${nombreEdificio} nivel ${tropa.nivelRequerido}`;
}

/** Nivel de la tropa reclutada por un escuadrón (Doc 5.8, fix de tropas a petición del usuario): un escuadrón
 * ya no tiene "Tier" propio — nunca cambia de `tropaId` al ganar veteranía (solo gana poder), así que el nivel
 * que importa es el del catálogo de la tropa (`nivelRequerido`, TROPAS_RECLUTABLES), no un valor que evolucione. */
function nivelTropaTxt(tropaId: string): string {
  const tropa = CATALOGOS.tropasReclutables.find((t) => t.id === tropaId);
  return tropa ? `Nivel ${tropa.nivelRequerido}` : '—';
}

/** Desglose de costo de una tropa, por soldado y para el escuadrón completo (`unidadesPorDefecto`, tamaño fijo).
 * Incluye el oro por escalón (Doc 5.8, "economía del oro") — salvo la Milicia del Centro Urbano, exenta. */
function costoTropaTxt(tropa: (typeof CATALOGOS.tropasReclutables)[number], porSoldado: boolean): string {
  const mult = porSoldado ? 1 : tropa.unidadesPorDefecto;
  const partes = Object.entries(tropa.costoEquipo).map(([r, c]) => `${(c ?? 0) * mult} ${RECURSO_NOMBRE[r] ?? r}`);
  const oro = gameStore.costoOroReclutamientoPorSoldado(tropa) * mult;
  if (oro > 0) partes.push(`${oro} oro`);
  return partes.length ? partes.join(' + ') : '—';
}

/** Segmento "Info:" bajo el combo de reclutamiento — catálogo de solo consulta (Fase C8): edificio/nivel
 * exigido, tamaño fijo del escuadrón, costo por soldado y total, poder base. Reclutar de verdad (con
 * jugador y asentamiento reales) sigue siendo un comando de jugador, no algo que esta consola ejecute. */
function actualizarInfoTropa(): void {
  const tropa = CATALOGOS.tropasReclutables.find((t) => t.id === reclutarTropaSelect.value);
  if (!tropa) {
    reclutarTropaInfoEl.innerHTML = '';
    return;
  }
  reclutarTropaInfoEl.innerHTML = `
    <div class="kv-row"><span>Info:</span><span>${tropa.nombre}</span></div>
    <div class="kv-row"><span>Edificio requerido</span><span>${edificioRequeridoTxt(tropa)}</span></div>
    <div class="kv-row"><span>Unidades por escuadrón</span><span>${tropa.unidadesPorDefecto} (tamaño fijo)</span></div>
    <div class="kv-row"><span>Costo por soldado</span><span>${costoTropaTxt(tropa, true)}</span></div>
    <div class="kv-row"><span>Costo del escuadrón completo</span><span>${costoTropaTxt(tropa, false)}</span></div>
    <div class="kv-row"><span>Poder base (por soldado)</span><span>${tropa.poderBase}</span></div>
  `;
}
reclutarTropaSelect.addEventListener('change', actualizarInfoTropa);
actualizarInfoTropa();

/** Info de flota (ampliación de comercio, pestaña Acciones): mercado activo, cupo y cuántas caravanas propias
 * hay disponibles/en tránsito. Se refresca al cambiar de asentamiento y en cada `render()` (los conteos
 * cambian tick a tick, no solo con una acción del jugador). */
function actualizarInfoFlota(state: GameState): void {
  const asentamiento = state.asentamientos.find((a) => a.id === flotaAsentamientoSelect.value);
  if (!asentamiento) {
    flotaInfoEl.innerHTML = '<div class="kv-row"><span>Info:</span><span>Selecciona un asentamiento.</span></div>';
    return;
  }
  const info = gameStore.caravanasInfo(asentamiento);
  flotaInfoEl.innerHTML = `
    <div class="kv-row"><span>Mercado activo</span><span>${info.mercadoActivo ? 'Sí' : 'No'}</span></div>
    <div class="kv-row"><span>Cupo de flota</span><span>${info.cupo}</span></div>
    <div class="kv-row"><span>Disponibles</span><span>${info.disponibles}</span></div>
    <div class="kv-row"><span>En tránsito</span><span>${info.enTransito}</span></div>
    <div class="kv-row"><span>Volviendo</span><span>${info.retornando}</span></div>
    <div class="kv-row"><span>Cooldown de creación</span><span>${info.cooldownCreacionRestanteMin > 0 ? `${info.cooldownCreacionRestanteMin} min` : 'Listo'}</span></div>
  `;
}
flotaAsentamientoSelect.addEventListener('change', () => actualizarInfoFlota(gameStore.getState()));

/** Materiales comerciables disponibles de cada asentamiento seleccionado en el formulario de trueque. */
function actualizarInfoTruequeAsentamientos(state: GameState): void {
  const renderInfo = (select: HTMLSelectElement): string => {
    const asentamiento = state.asentamientos.find((a) => a.id === select.value);
    if (!asentamiento) return '<div class="legend-note">Selecciona un asentamiento.</div>';
    const recursos = CATALOGOS.recursosTrueque
      .map((recurso) => ({ recurso, cantidad: asentamiento.almacen[recurso]?.cantidad ?? 0 }))
      .filter((item) => item.cantidad > 0);
    if (recursos.length === 0) return '<div class="legend-note">Sin materiales comerciables disponibles.</div>';
    return `<div class="trade-settlement-info-title">Materiales disponibles</div><div class="trade-material-list">${recursos
      .map((item) => `<span class="trade-material-pill"><span aria-hidden="true">${RECURSO_ICONO[item.recurso] ?? '📦'}</span>${RECURSO_NOMBRE[item.recurso] ?? item.recurso}<strong>${item.cantidad.toFixed(0)}</strong></span>`)
      .join('')}</div>`;
  };
  truequeAInfoEl.innerHTML = renderInfo(truequeASelect);
  truequeBInfoEl.innerHTML = renderInfo(truequeBSelect);
}

truequeASelect.addEventListener('change', () => actualizarInfoTruequeAsentamientos(gameStore.getState()));
truequeBSelect.addEventListener('change', () => actualizarInfoTruequeAsentamientos(gameStore.getState()));

/** Secciones del roster completo (Doc 5.8), agrupadas por edificio de reclutamiento — la pestaña Guerra las
 * muestra siempre visibles, a diferencia del segmento "Info:" que solo detalla la tropa seleccionada. */
const ROSTER_SECCIONES: { edificio: 'centroUrbano' | 'barracon' | 'galeriaDeTiro'; titulo: string }[] = [
  { edificio: 'centroUrbano', titulo: 'Centro Urbano (defensa mínima, sin edificio dedicado)' },
  { edificio: 'barracon', titulo: 'Barracón (cuerpo a cuerpo)' },
  { edificio: 'galeriaDeTiro', titulo: 'Galería de tiro (a distancia)' },
];

function renderRosterTropas(): void {
  const filas = (edificio: (typeof ROSTER_SECCIONES)[number]['edificio']) =>
    CATALOGOS.tropasReclutables
      .filter((t) => t.edificio === edificio)
      .map(
        (t) =>
          `<tr><td>${t.nivelRequerido}</td><td>${t.nombre}</td><td>${costoTropaTxt(t, true)}</td><td>${t.poderBase}</td><td>${t.unidadesPorDefecto}</td></tr>`
      )
      .join('');
  const tabla = (titulo: string, edificio: (typeof ROSTER_SECCIONES)[number]['edificio']) => {
    const cuerpo = filas(edificio);
    if (!cuerpo) return '';
    return `
    <h3>${titulo}</h3>
    <table class="mini-table">
      <thead><tr><th>Nvl</th><th>Tropa</th><th>Costo/soldado</th><th>Poder</th><th>Uds.</th></tr></thead>
      <tbody>${cuerpo}</tbody>
    </table>`;
  };
  rosterTropasEl.innerHTML = ROSTER_SECCIONES.map(({ edificio, titulo }) => tabla(titulo, edificio)).join('');
}

function etiquetaAsentamiento(a: Asentamiento, facciones: Faccion[]): string {
  const nombreFaccion = facciones.find((f) => f.id === a.faccionId)?.nombre ?? a.faccionId;
  return `${a.nombre ?? a.id} (${nombreFaccion})`;
}

function actualizarSelects(state: GameState): void {
  const opcionesAsentamientos = state.asentamientos.map((a) => `<option value="${a.id}">${etiquetaAsentamiento(a, state.facciones)}</option>`).join('');
  for (const select of [truequeASelect, truequeBSelect, flotaAsentamientoSelect]) {
    const seleccionPrevia = select.value;
    select.innerHTML = opcionesAsentamientos;
    if (state.asentamientos.some((a) => a.id === seleccionPrevia)) select.value = seleccionPrevia;
  }
}

function renderPanelAsentamientos(state: GameState): void {
  asentamientosPanelEl.innerHTML = state.asentamientos
    .map((a) => {
      const { pesants, artesanos, nobleza } = a.poblacion;
      const faccion = state.facciones.find((f) => f.id === a.faccionId);
      const recursosClave = ['madera', 'piedra', 'trigo', 'oro'];
      const activos = a.edificios.filter((e) => e.estado === 'activo').length;
      const enCurso = a.edificios.length - activos;
      const mantenimiento = Math.max(0, Math.min(100, a.medidorMantenimiento));
      const otrasCasas = a.casasCompradas.filter((id) => !a.jugadoresFundadoresIds.includes(id));
      const recursosHtml = recursosClave
        .map((r) => {
          const recurso = a.almacen[r];
          return `<div class="registro-resource"><span>${RECURSO_ICONO[r] ?? '📦'} ${RECURSO_NOMBRE[r] ?? r}</span><strong>${Math.floor(recurso?.cantidad ?? 0)}<small>/${recurso?.capacidad ?? 0}</small></strong></div>`;
        })
        .join('');
      const cargosHtml = CATALOGOS.cargos
        .map((c) => `<span class="registro-role"><b>${c}</b>${a.cargos[`${c}Id` as keyof typeof a.cargos] ?? '—'}</span>`)
        .join('');
      const ocupada = gameStore.ocupacionInfo(a);
      return `<article class="registro-asentamiento">
        <header class="registro-asentamiento-header">
          <div>
            <h3>${a.nombre ?? a.id}${ocupada ? ' <span class="chip" title="Ocupación militar reciente (Doc 5.12.9)">⚔ ocupada</span>' : ''}</h3>
            <p>${faccion?.nombre ?? a.faccionId} <span aria-hidden="true">·</span> ${a.id}</p>
          </div>
          <span class="registro-level">Nivel ${a.nivel}</span>
        </header>
        <div class="registro-asentamiento-summary">
          <div class="registro-summary-item"><span>Población</span><strong>${pesants + artesanos + nobleza}</strong><small>${pesants} pesants · ${artesanos} artesanos · ${nobleza} nobleza</small></div>
          <div class="registro-summary-item"><span>Edificios</span><strong>${activos}</strong><small>${enCurso} en curso o en cola</small></div>
          <div class="registro-summary-item"><span>Radio potencial</span><strong>${Math.round(a.radioPotencial)}</strong><small>Fundado ${fmtTiempoMundo(a.fundadoEn)}</small></div>
          <div class="registro-summary-item"><span>Viviendas</span><strong>${a.casasCompradas.length}</strong><small>${otrasCasas.length} adquiridas después</small></div>
        </div>
        <div class="registro-asentamiento-health">
          <div class="registro-health-label"><span>Mantenimiento</span><strong>${a.medidorMantenimiento.toFixed(0)}/100</strong></div>
          <div class="registro-health-bar"><span style="width:${mantenimiento}%"></span></div>
        </div>
        <div class="registro-asentamiento-columns">
          <section><h4>Almacén</h4><div class="registro-resource-grid">${recursosHtml}</div></section>
          <section><h4>Cargos</h4><div class="registro-role-list">${cargosHtml}</div></section>
        </div>
        <footer class="registro-asentamiento-footer"><span>Políticas activas: <strong>${a.politicasActivas.length}</strong></span><span>Fundadores: <strong>${a.jugadoresFundadoresIds.length}</strong></span></footer>
      </article>`;
    })
    .join('') || '<p class="legend-note registro-empty">Aún no hay asentamientos fundados.</p>';
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
    .filter((r) => (a.almacen[r]?.cantidad ?? 0) > 0)
    .map((r) => {
      const info = a.almacen[r];
      const cantidad = Math.floor(info?.cantidad ?? 0);
      const capacidad = info?.capacidad ?? 0;
      const porcentaje = capacidad > 0 ? Math.min(100, Math.round((cantidad / capacidad) * 100)) : 0;
      const nombre = RECURSO_NOMBRE[r] ?? r;
      return `<div class="storage-resource-card" title="${nombre}">
        <div class="storage-resource-heading">
          <span class="storage-resource-icon" aria-hidden="true">${RECURSO_ICONO[r] ?? '📦'}</span>
          <span class="storage-resource-name">${nombre}</span>
          <strong>${cantidad}<small> / ${capacidad}</small></strong>
        </div>
        <div class="storage-capacity-track" aria-label="${porcentaje}% de capacidad ocupada"><span style="width:${porcentaje}%"></span></div>
      </div>`;
    })
    .join('') || '<p class="legend-note">Almacén vacío.</p>';

  // Reserva manual por recurso (ver Asentamiento.reservaManual): tope 0-999 que la auto-construcción no puede
  // tocar (construcción manual exenta). Fase C8: solo consulta para el administrador — calibrarla es del
  // Tesorero (jugador), no de la consola de administración.
  const reservaHtml = a.cargos.tesoreroId
    ? `<div class="kv-grid">${CATALOGOS.recursosTrueque
        .map((r) => `<div class="kv-row"><span>${RECURSO_NOMBRE[r] ?? r}</span><span>${a.reservaManual?.[r] ?? 0}</span></div>`)
        .join('')}</div>
      <p class="legend-note">La auto-construcción nunca gasta por debajo de esta reserva (la construcción manual queda exenta).</p>`
    : '<p class="legend-note">Requiere un Tesorero asignado.</p>';

  const cargosHtml = CATALOGOS.cargos
    .map((c) => {
      const id = a.cargos[`${c}Id` as keyof typeof a.cargos];
      return `<div class="kv-row"><span>${c}</span><span>${id ?? '—'}</span></div>`;
    })
    .join('');

  // La cola de construcción es única y compartida por TODO el asentamiento (no una por tipo de edificio).
  // Overhaul de auto-construcción: los "en_cola" ya están PAGADOS (el pago ocurre al comprometerse, no al
  // arrancar obra) y compiten por hueco de obra por `prioridad` (score de necesidad), mayor primero.
  //
  // El orden se calcula AQUÍ y no viene dado por el array: el motor devuelve sus edificios en orden de
  // crecimiento, que es historial y no cola (permutarlo movía las calles — ver `edificiosOrdenados` en
  // engine/construction.ts). Es el mismo criterio que usa el motor para decidir quién arranca.
  const colaGlobal = a.edificios.filter((e) => e.estado === 'en_cola').sort((x, y) => (y.prioridad ?? 0) - (x.prioridad ?? 0));
  const posicionEnCola = new Map(colaGlobal.map((e, i) => [e.id, i + 1]));
  const enConstruccionCount = a.edificios.filter((e) => e.estado === 'en_construccion').length;

  const edificiosPorTipo = new Map<string, { activos: number; enConstruccion: number[]; enCola: number[]; danados: number }>();
  for (const e of a.edificios) {
    const entry = edificiosPorTipo.get(e.tipo) ?? { activos: 0, enConstruccion: [], enCola: [], danados: 0 };
    if (e.estado === 'activo') entry.activos += 1;
    // Fase D: la obra ya no lleva un contador `ticksRestantes` sino la fecha absoluta `completaEn` (Instante
    // de mundo) — la interfaz muestra los minutos de mundo que faltan contra `state.instante`.
    else if (e.estado === 'en_construccion')
      entry.enConstruccion.push(e.completaEn !== undefined ? Math.max(0, Math.round((e.completaEn - state.instante) / 60_000)) : 0);
    else entry.enCola.push(posicionEnCola.get(e.id)!);
    // Ocupación post-conquista (Doc 5.12.9): un edificio dañado por el saqueo está en cola con `danado`; se
    // reconstruye pagando solo una fracción del costo.
    if (e.danado) entry.danados += 1;
    edificiosPorTipo.set(e.tipo, entry);
  }
  const totalDanados = a.edificios.filter((e) => e.danado).length;
  const edificiosHtml = edificiosPorTipo.size
    ? `<table class="mini-table">
        <thead><tr><th>Edificio</th><th>Función</th><th>Activos</th><th>En construcción</th><th>En cola (${colaGlobal.length}/${CATALOGOS.maximoEdificiosEnCola})</th></tr></thead>
        <tbody>
          ${Array.from(edificiosPorTipo.entries())
            .map(([tipo, e]) => {
              const construccionTxt = e.enConstruccion.length
                ? e.enConstruccion.map((m) => `${m} min`).join(', ')
                : '—';
              const colaTxt = e.enCola.length
                ? `${e.enCola.map((p) => `#${p} de ${colaGlobal.length}`).join(', ')}${e.danados > 0 ? ` <span class="legend-note">(${e.danados} dañado${e.danados > 1 ? 's' : ''})</span>` : ''}`
                : '—';
              return `<tr><td>${EDIFICIO_NOMBRE[tipo] ?? tipo}</td><td>${EDIFICIO_FUNCION[tipo] ?? '—'}</td><td>${e.activos}</td><td>${construccionTxt}</td><td>${colaTxt}</td></tr>`;
            })
            .join('')}
        </tbody>
      </table>${totalDanados > 0 ? `<p class="legend-note">${totalDanados} edificio(s) dañado(s) por un saqueo de conquista — se reconstruyen a coste/tiempo reducido (Doc 5.12.9).</p>` : ''}`
    : '<p class="legend-note">Sin edificios.</p>';

  // Mejora manual de un edificio individual (Doc 4.2, a petición del usuario): la mejora automática de
  // `avanzarMejoras` sigue corriendo cada tick — esto solo deja adelantar la de un edificio elegido. Solo
  // aparecen los edificios activos con una mejora posible (ver `gameStore.infoMejoraEdificio`).
  const mejorasDisponibles: { edificio: Edificio; info: EstadoMejoraEdificio }[] = [];
  for (const e of a.edificios) {
    if (e.estado !== 'activo') continue;
    const info = gameStore.infoMejoraEdificio(a, e.id);
    if (info) mejorasDisponibles.push({ edificio: e, info });
  }
  const mejorasHtml = mejorasDisponibles.length
    ? `<table class="mini-table">
        <thead><tr><th>Edificio</th><th>Nivel</th><th>Costo</th></tr></thead>
        <tbody>
          ${mejorasDisponibles
            .map(({ edificio, info }) => {
              const costoTxt =
                Object.entries(info.costo)
                  .map(([r, c]) => `${RECURSO_NOMBRE[r] ?? r}: ${c}`)
                  .join(', ') || '—';
              return `<tr>
                <td>${EDIFICIO_NOMBRE[edificio.tipo] ?? edificio.tipo}</td>
                <td>${info.nivelActual} &rarr; ${info.nivelSiguiente}</td>
                <td>${costoTxt}</td>
              </tr>`;
            })
            .join('')}
        </tbody>
      </table>`
    : '<p class="legend-note">Sin mejoras disponibles ahora mismo.</p>';

  const produccion = gameStore.produccionInfo(a);
  const consumoPorTipoYRecurso = new Map<string, { tipo: string; recurso: string; activos: number; cantidadPorMinuto: number }>();
  for (const edificio of a.edificios) {
    if (edificio.estado !== 'activo') continue;
    const economia = gameStore.edificioEconomiaInfo(a, edificio);
    for (const item of economia.consumoTotal) {
      const clave = `${edificio.tipo}:${item.recurso}`;
      const anterior = consumoPorTipoYRecurso.get(clave);
      if (anterior) anterior.cantidadPorMinuto += item.cantidadPorMinuto;
      else {
        consumoPorTipoYRecurso.set(clave, {
          tipo: edificio.tipo,
          recurso: item.recurso,
          activos: a.edificios.filter((e) => e.estado === 'activo' && e.tipo === edificio.tipo).length,
          cantidadPorMinuto: item.cantidadPorMinuto,
        });
      }
    }
  }
  const consumo = [...consumoPorTipoYRecurso.values()];
  const manoObra = gameStore.manoObraInfo(a);
  const poblacion = gameStore.poblacionInfo(a);
  const produccionHtml = produccion.length
    ? `<table class="mini-table">
        <thead><tr><th>Edificio</th><th>Activos</th><th>Recurso</th><th>Producción/min</th></tr></thead>
        <tbody>
          ${produccion
            .map(
              (p) =>
                `<tr><td>${EDIFICIO_NOMBRE[p.tipo] ?? p.tipo}</td><td>${p.activos}</td><td><span class="resource-inline-icon" aria-hidden="true">${RECURSO_ICONO[p.recurso] ?? '📦'}</span>${RECURSO_NOMBRE[p.recurso] ?? p.recurso}</td><td>${Math.floor(p.cantidadPorMinuto)}</td></tr>`
            )
            .join('')}
        </tbody>
      </table>`
    : '<p class="legend-note">Sin edificios productores activos.</p>';

  const consumoHtml = consumo.length
    ? `<table class="mini-table">
        <thead><tr><th>Edificio</th><th>Activos</th><th>Recurso</th><th>Consumo/total</th></tr></thead>
        <tbody>
          ${consumo
            .map(
              (c) => {
                const producidoEsteTick = produccion
                  .filter((item) => item.recurso === c.recurso)
                  .reduce((total, item) => total + item.cantidadPorMinuto, 0);
                const disponibleParaConsumo = a.almacen[c.recurso]?.cantidad ?? 0;
                const consumosAnteriores = consumo
                  .slice(0, consumo.indexOf(c))
                  .filter((item) => item.recurso === c.recurso)
                  .reduce((total, item) => total + item.cantidadPorMinuto, 0);
                const cubierto = Math.min(
                  c.cantidadPorMinuto,
                  Math.max(0, disponibleParaConsumo + producidoEsteTick - consumosAnteriores)
                );
                const formatearCantidad = (cantidad: number) => Math.floor(cantidad).toString();
                return `<tr><td>${EDIFICIO_NOMBRE[c.tipo] ?? c.tipo}</td><td>${c.activos}</td><td><span class="resource-inline-icon" aria-hidden="true">${RECURSO_ICONO[c.recurso] ?? '📦'}</span>${RECURSO_NOMBRE[c.recurso] ?? c.recurso}</td><td>${formatearCantidad(cubierto)}/${formatearCantidad(c.cantidadPorMinuto)}</td></tr>`;
              }
            )
            .join('')}
        </tbody>
      </table>`
    : '<p class="legend-note">Sin edificios consumidores activos.</p>';

  const politicasActivasHtml = a.politicasActivas.length
    ? `<table class="mini-table">
        <thead><tr><th>Política</th><th>Cargo</th><th>Efecto</th><th>Expira</th></tr></thead>
        <tbody>
          ${a.politicasActivas
            .map((p) => {
              const def = CATALOGOS.politicas.find((c) => c.id === p.politicaId);
              return `<tr><td>${def?.nombre ?? p.politicaId}</td><td>${p.cargo}</td><td>${def ? efectoPolitica(def) : '—'}</td><td>${fmtTiempoMundo(p.expiraEn)}</td></tr>`;
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
    // s.edificiosRequeridos puede ser MENOR que la cantidad de candidatos en s.edificiosFaltantes (gate tipo
    // "al menos N de M", Doc Fase_0_6 nivel 2: 3 de 6 edificios de extracción) — el ratio va sobre cuántos
    // YA están construidos, no sobre cuántos faltan de la lista completa de candidatos.
    const ratioEdificios = s.edificiosRequeridos > 0 ? Math.min(1, s.edificiosConstruidos / s.edificiosRequeridos) : 1;
    nivelPorcentaje = Math.round(((ratioPesants + ratioArtesanos + ratioEdificios) / 3) * 100);
    const faltantesTexto =
      s.edificiosConstruidos < s.edificiosRequeridos && s.edificiosFaltantes.length
        ? `; ${s.edificiosConstruidos}/${s.edificiosRequeridos} edificios, de: ${s.edificiosFaltantes.map((tipo) => EDIFICIO_NOMBRE[tipo] ?? tipo).join(', ')}`
        : '';
    nivelTexto = `Nivel ${s.nivelObjetivo}: ${s.pesants.actual}/${s.pesants.requerido} pesants, ${s.artesanos.actual}/${s.artesanos.requerido} artesanos${faltantesTexto}`;
  }
  // Cumple gates pero no sube: el cupo de nivel de su Facción está lleno (Doc Fase_0_5 §5) — se queda
  // "elegible, esperando cupo" hasta que la Facción suba de nivel (combate/conquista/construcción) o libere
  // un cupo. Sin este aviso el jugador solo ve "100%" y ningún ascenso, sin saber por qué.
  const cupoBloqueo = gameStore.cupoNivelInfo(a);
  const cupoBloqueoHtml = cupoBloqueo
    ? `<div class="fundacion-viabilidad aviso" style="margin-top:6px">Cumple los requisitos para nivel ${cupoBloqueo.nivelObjetivo}, pero el cupo de la Facción para ese nivel está lleno (${cupoBloqueo.ocupados}/${cupoBloqueo.cupoTotal}) — sube cuando la Facción alcance un nivel mayor (combate, conquista o edificios nuevos) o se libere un cupo.</div>`
    : '';

  const mantenimiento = gameStore.mantenimientoInfo(a);
  const recaudacion = gameStore.recaudacionInfo(a);
  const ocupacion = gameStore.ocupacionInfo(a);
  const produccionPorRecurso = new Map<string, number>();
  for (const item of gameStore.produccionInfo(a)) {
    produccionPorRecurso.set(item.recurso, (produccionPorRecurso.get(item.recurso) ?? 0) + item.cantidadPorMinuto);
  }
  const mantenimientoHtml = mantenimiento.congeladoPorOcupacion
    ? `<p class="legend-note">Ocupación reciente (Doc 5.12.9): la degradación de mantenimiento está suspendida — ${ocupacion?.minutosRestantes ?? 0} min restantes.</p>`
    : mantenimiento.enGracia
    ? `<p class="legend-note">En periodo de gracia (recién fundado): sin coste todavía — ${mantenimiento.minutosParaFinGracia} min restantes.</p>`
    : mantenimiento.items.length
      ? `<table class="mini-table">
          <thead><tr><th>Recurso</th><th>Disponible</th><th>Producción/min</th><th>Coste/min</th><th>Valor (producción-coste)</th></tr></thead>
          <tbody>
            ${mantenimiento.items
              .map(
                (i) => {
                  const produccion = produccionPorRecurso.get(i.recurso) ?? 0;
                  const valor = produccion - i.costoPorMinuto;
                  const nombre = RECURSO_NOMBRE[i.recurso] ?? i.recurso;
                  return `<tr class="${i.cubierto ? '' : 'fila-deficit'}"><td><span class="resource-inline-icon" aria-hidden="true">${RECURSO_ICONO[i.recurso] ?? '📦'}</span>${nombre}</td><td>${i.disponible.toFixed(0)}</td><td>${produccion.toFixed(1)}</td><td>${i.costoPorMinuto.toFixed(1)}</td><td class="${valor < 0 ? 'valor-negativo' : ''}">${valor.toFixed(1)}</td></tr>`;
                }
              )
              .join('')}
          </tbody>
        </table>`
      : '<p class="legend-note">Sin coste de mantenimiento.</p>';

  const poderMilitar = gameStore.poderMilitarInfo(a);
  const escuadronesHtml = a.escuadrones.length
    ? `<table class="mini-table">
        <thead><tr><th>Escuadrón</th><th>Jugador</th><th>Origen</th><th>Nivel</th><th>Cantidad</th><th>Veteranía</th><th>Moral</th></tr></thead>
        <tbody>
          ${a.escuadrones
            .map(
              (e) =>
                `<tr><td>${e.nombre}${e.heridoHasta ? ' (herido)' : ''}</td><td>${e.jugadorId}</td><td>${e.origen}</td><td>${nivelTropaTxt(e.tropaId)}</td><td>${e.cantidad}</td><td>${e.veterania.toFixed(1)}</td><td>${e.moral.toFixed(0)}</td></tr>`
            )
            .join('')}
        </tbody>
      </table>
      <div class="military-summary">
        <span>Total de soldados: <strong>${poderMilitar.soldados}</strong></span>
        <span>Nivel de poder: <strong>${poderMilitar.poder.toFixed(1)}</strong></span>
      </div>`
    : '<p class="legend-note">Sin escuadrones.</p>';

  const truequesActivos = state.acuerdos.filter(
    (acuerdo) => acuerdo.estado === 'activo' && (acuerdo.asentamientoAId === a.id || acuerdo.asentamientoBId === a.id)
  );
  const truequesHtml = truequesActivos.length
    ? `<table class="mini-table">
        <thead><tr><th>Intercambio</th><th>CompletaciÃ³n</th><th>Caravanas asignadas</th><th>Expira</th></tr></thead>
        <tbody>
          ${truequesActivos
            .map((acuerdo) => {
              const asentamientoOtroId = acuerdo.asentamientoAId === a.id ? acuerdo.asentamientoBId : acuerdo.asentamientoAId;
              const otro = state.asentamientos.find((asentamiento) => asentamiento.id === asentamientoOtroId);
              const progresoA = acuerdo.cantidadTotalA > 0 ? Math.min(1, acuerdo.cantidadEntregadaA / acuerdo.cantidadTotalA) : 1;
              const progresoB = acuerdo.cantidadTotalB > 0 ? Math.min(1, acuerdo.cantidadEntregadaB / acuerdo.cantidadTotalB) : 1;
              const porcentaje = Math.round(((progresoA + progresoB) / 2) * 100);
              const caravanasAsignadas = state.caravanas.filter(
                (caravana) => caravana.tipo === 'comercial' && caravana.origenAcuerdoId === acuerdo.id
              ).length;
              return `<tr>
                <td>${RECURSO_NOMBRE[acuerdo.recursoA] ?? acuerdo.recursoA} ↔ ${RECURSO_NOMBRE[acuerdo.recursoB] ?? acuerdo.recursoB}<br/><span class="legend-note">con ${otro?.nombre ?? otro?.id ?? asentamientoOtroId}</span></td>
                <td>${porcentaje}%<br/><span class="legend-note">A: ${acuerdo.cantidadEntregadaA}/${acuerdo.cantidadTotalA} · B: ${acuerdo.cantidadEntregadaB}/${acuerdo.cantidadTotalB}</span></td>
                <td>${caravanasAsignadas}</td>
                <td>${fmtTiempoMundo(acuerdo.expiraEn)}</td>
              </tr>`;
            })
            .join('')}
        </tbody>
      </table>`
    : '<p class="legend-note">Sin trueques activos.</p>';

  return `
    <div class="settlement-detail">
      <div class="detail-section settlement-storage-summary">
        <div class="storage-section-heading">
          <span class="storage-section-icon" aria-hidden="true">📦</span>
          <div><h3>AlmacÃ©n</h3><p>Existencias actuales y capacidad disponible</p></div>
        </div>
        <div class="storage-resource-grid">${almacenHtml}</div>
        <details class="storage-reserve-section">
          <summary class="storage-section-heading storage-reserve-heading">
            <span class="storage-section-icon" aria-hidden="true">🔒</span>
            <div><h3>Reserva protegida</h3><p>Material que la auto-construcciÃ³n no puede gastar</p></div>
            <span class="storage-reserve-chevron" aria-hidden="true">▸</span>
          </summary>
          <div class="storage-reserve-content">${reservaHtml}</div>
        </details>
      </div>
      <div class="settlement-detail-tabs" role="tablist" aria-label="Información del asentamiento">
        <button type="button" class="settlement-detail-tab${asentamientoDetalleTab === 'general' ? ' active' : ''}" data-settlement-detail-tab="general" role="tab" aria-selected="${asentamientoDetalleTab === 'general'}">General</button>
        <button type="button" class="settlement-detail-tab${asentamientoDetalleTab === 'edificios' ? ' active' : ''}" data-settlement-detail-tab="edificios" role="tab" aria-selected="${asentamientoDetalleTab === 'edificios'}">Edificios</button>
        <button type="button" class="settlement-detail-tab${asentamientoDetalleTab === 'produccion' ? ' active' : ''}" data-settlement-detail-tab="produccion" role="tab" aria-selected="${asentamientoDetalleTab === 'produccion'}">Producción</button>
        <button type="button" class="settlement-detail-tab${asentamientoDetalleTab === 'militar' ? ' active' : ''}" data-settlement-detail-tab="militar" role="tab" aria-selected="${asentamientoDetalleTab === 'militar'}">Militar</button>
      </div>

      <div class="settlement-detail-panel${asentamientoDetalleTab === 'general' ? ' active' : ''}" data-settlement-detail-panel="general" role="tabpanel">
      <div class="detail-section">
        <h3>${a.nombre ?? a.id}${a.nombre ? ` <span class="legend-note" style="font-weight:normal">(${a.id})</span>` : ''}</h3>
        <div class="kv-grid">
          <div class="kv-row"><span>Facción</span><span>${faccion?.nombre ?? a.faccionId}</span></div>
          <div class="kv-row"><span>Nivel</span><span>${a.nivel}</span></div>
          <div class="kv-row"><span>Radio potencial</span><span>${Math.round(a.radioPotencial)}</span></div>
          <div class="kv-row"><span>Fundado</span><span>${fmtTiempoMundo(a.fundadoEn)}</span></div>
        </div>
        <div class="kv-row" style="margin-top:6px"><span>Progreso de nivel</span><span>${nivelTexto}</span></div>
        <div class="mantenimiento-bar"><div class="mantenimiento-fill" style="width:${nivelPorcentaje}%"></div></div>
        ${cupoBloqueoHtml}
        ${
          ocupacion
            ? `<div class="fundacion-viabilidad aviso" style="margin-top:6px">⚔ Bajo ocupación militar (Doc 5.12.9) — ${ocupacion.minutosRestantes} min restantes. Inmune a un nuevo asedio; recaudación ×${ocupacion.factorRecaudacion} y crecimiento ×${ocupacion.factorCrecimiento}; mantenimiento congelado. La guarnición son escuadrones del conquistador (ver pestaña Militar).</div>`
            : ''
        }
        <div class="kv-row" style="margin-top:6px"><span>Mantenimiento</span><span>${a.medidorMantenimiento.toFixed(0)}/100</span></div>
        <div class="mantenimiento-bar"><div class="mantenimiento-fill" style="width:${Math.max(0, Math.min(100, a.medidorMantenimiento))}%"></div></div>
        <div class="kv-row" style="margin-top:6px"><span>Nutrición</span><span>${(a.nutricionPoblacion ?? 100).toFixed(0)}/100</span></div>
        <div class="mantenimiento-bar"><div class="mantenimiento-fill" style="width:${Math.max(0, Math.min(100, a.nutricionPoblacion ?? 100))}%"></div></div>
      </div>

      <div class="detail-section">
        <h3>Mantenimiento — consumo por minuto</h3>
        ${mantenimientoHtml}
        <div class="kv-row" style="margin-top:6px"><span>Recaudación de oro</span><span>+${recaudacion.total.toFixed(2)}/min${recaudacion.reducidaPorOcupacion ? ' <span class="legend-note">(reducida por ocupación)</span>' : ''} (P ${recaudacion.pesants.toFixed(2)} · A ${recaudacion.artesanos.toFixed(2)} · N ${recaudacion.nobleza.toFixed(2)})</span></div>
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
          <div class="kv-row"><span>Pesants</span><span>${poblacion.pesants.actual}/${poblacion.pesants.limite}</span></div>
          <div class="kv-row"><span>Artesanos</span><span>${poblacion.artesanos.actual}/${poblacion.artesanos.limite}</span></div>
          <div class="kv-row"><span>Nobleza</span><span>${poblacion.nobleza.actual}/${poblacion.nobleza.limite}</span></div>
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
        <h3>Trueques activos</h3>
        ${truequesHtml}
      </div>

      <div class="detail-section">
        <h3>Políticas activas y slots por cargo</h3>
        ${politicasHtml}
      </div>

      </div>

      <div class="settlement-detail-panel${asentamientoDetalleTab === 'edificios' ? ' active' : ''}" data-settlement-detail-panel="edificios" role="tabpanel">
      <div class="detail-section">
        <h3>Edificios</h3>
        <div class="kv-row">
          <span>Auto-construcción</span><span>${a.autoConstruccionPausada ? 'pausada' : 'activa'}</span>
        </div>
        <p class="legend-note">
          ${a.autoConstruccionPausada
            ? 'No se detectan nuevas necesidades. Lo ya pagado (en cola/en construcción) sigue avanzando.'
            : `Cupo de obras activas simultáneas: ${enConstruccionCount}/${CATALOGOS.maximoEnConstruccionSimultanea}.`}
        </p>
        ${edificiosHtml}
      </div>

      <div class="detail-section">
        <h3>Mejoras de edificios disponibles</h3>
        <p class="legend-note">La mejora automática evalúa cada edificio activo en cada tick del servidor — control manual de jugador (Gobernador/Maestro de Obras), no de administración.</p>
        ${mejorasHtml}
      </div>

      <div class="detail-section">
        <h3>Cola de construcción</h3>
        ${
          colaGlobal.length
            ? `<table class="mini-table">
                <thead><tr><th>#</th><th>Edificio</th></tr></thead>
                <tbody>
                  ${colaGlobal
                    .map((e, i) => `<tr><td>${i + 1}</td><td>${EDIFICIO_NOMBRE[e.tipo] ?? e.tipo}${e.danado ? ' <span class="legend-note">(dañado — reconstrucción)</span>' : ''}</td></tr>`)
                    .join('')}
                </tbody>
              </table>`
            : '<p class="legend-note">Cola vacía.</p>'
        }
      </div>

      </div>

      <div class="settlement-detail-panel${asentamientoDetalleTab === 'produccion' ? ' active' : ''}" data-settlement-detail-panel="produccion" role="tabpanel">
      <div class="detail-section">
        <h3>Producción — por minuto</h3>
        ${produccionHtml}
      </div>

      <div class="detail-section">
        <h3>Consumo — consumo/total</h3>
        ${consumoHtml}
      </div>
      </div>

      <div class="settlement-detail-panel${asentamientoDetalleTab === 'militar' ? ' active' : ''}" data-settlement-detail-panel="militar" role="tabpanel">
      <div class="detail-section">
        <h3>Escuadrones</h3>
        ${
          ocupacion
            ? `<p class="legend-note">Ocupación reciente (Doc 5.12.9): esta guarnición son los escuadrones del ejército conquistador, propiedad de jugadores que NO residen aquí. Defienden y su dueño los repone/re-moviliza. Inmune a un nuevo asedio ${ocupacion.minutosRestantes} min más.</p>`
            : ''
        }
        ${escuadronesHtml}
      </div>
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

  // Combo de Facción (a petición del usuario): filtra a UNA Facción a la vez en vez de listar el grupo de
  // todas simultáneamente — evita que la lista deje de caber en pantalla con muchas Facciones. Por defecto
  // sigue a la Facción dueña del asentamiento ya seleccionado.
  const faccionesConAsentamientos = state.facciones.filter((f) => state.asentamientos.some((a) => a.faccionId === f.id));
  const faccionDelSeleccionado = state.asentamientos.find((a) => a.id === asentamientoSeleccionadoId)!.faccionId;
  if (!asentamientosFaccionFiltroId || !faccionesConAsentamientos.some((f) => f.id === asentamientosFaccionFiltroId)) {
    asentamientosFaccionFiltroId = faccionDelSeleccionado;
  }
  const faccionFiltroId = asentamientosFaccionFiltroId!;

  const faccionSelectHtml = `<select id="asentamientos-faccion-select">
    ${faccionesConAsentamientos
      .map((f) => `<option value="${f.id}"${f.id === faccionFiltroId ? ' selected' : ''}>${f.nombre}</option>`)
      .join('')}
  </select>`;
  const botones = state.asentamientos
    .filter((a) => a.faccionId === faccionFiltroId)
    .map(
      (a) => {
        const mantenimiento = gameStore.mantenimientoInfo(a);
        const mantenimientoIncumplido = !mantenimiento.enGracia && mantenimiento.items.some((item) => !item.cubierto);
        const avisoMantenimiento = mantenimientoIncumplido
          ? '<span class="settlement-warning-icon" title="Mantenimiento no cubierto" aria-label="Mantenimiento no cubierto">⚠</span>'
          : '';
        return `<button type="button" class="settlement-tab-btn${a.id === asentamientoSeleccionadoId ? ' active' : ''}" data-settlement="${a.id}">${a.nombre ?? a.id}${avisoMantenimiento}</button>`;
      }
    )
    .join('');
  const gruposHtml = `<div class="faccion-group">
    <div class="faccion-group-header"><span class="swatch" style="background:${faccionColor(faccionFiltroId, state.facciones)}"></span>${faccionSelectHtml}</div>
    <div class="settlement-tab-row">${botones}</div>
  </div>`;

  const seleccionado = state.asentamientos.find((a) => a.id === asentamientoSeleccionadoId)!;
  const reservaActual = cont.querySelector<HTMLDetailsElement>('.storage-reserve-section');
  if (reservaActual) reservaProtegidaAbierta = reservaActual.open;
  cont.innerHTML = `${gruposHtml}${renderDetalleAsentamiento(seleccionado, state)}`;

  const reservaNueva = cont.querySelector<HTMLDetailsElement>('.storage-reserve-section');
  if (reservaNueva) {
    reservaNueva.open = reservaProtegidaAbierta;
    reservaNueva.addEventListener('toggle', () => {
      reservaProtegidaAbierta = reservaNueva.open;
    });
  }

  document.getElementById('asentamientos-faccion-select')?.addEventListener('change', (ev) => {
    asentamientosFaccionFiltroId = (ev.target as HTMLSelectElement).value;
    const primero = state.asentamientos.find((a) => a.faccionId === asentamientosFaccionFiltroId);
    if (primero) asentamientoSeleccionadoId = primero.id;
    render();
  });

  cont.querySelectorAll('.settlement-tab-btn').forEach((btn) => {
    btn.addEventListener('click', () => {
      asentamientoSeleccionadoId = (btn as HTMLElement).dataset.settlement!;
      render();
    });
  });

  cont.querySelectorAll<HTMLButtonElement>('[data-settlement-detail-tab]').forEach((btn) => {
    btn.addEventListener('click', () => {
      asentamientoDetalleTab = btn.dataset.settlementDetailTab as 'general' | 'edificios' | 'produccion' | 'militar';
      cont.querySelectorAll<HTMLButtonElement>('[data-settlement-detail-tab]').forEach((tab) => {
        const activa = tab === btn;
        tab.classList.toggle('active', activa);
        tab.setAttribute('aria-selected', String(activa));
      });
      cont.querySelectorAll<HTMLElement>('[data-settlement-detail-panel]').forEach((panel) => {
        panel.classList.toggle('active', panel.dataset.settlementDetailPanel === asentamientoDetalleTab);
      });
    });
  });

}

function renderDetalleFaccion(faccion: Faccion, state: GameState): string {
  const nivelFaccion = gameStore.nivelFaccionInfo(faccion);
  const cupo = gameStore.cupoAsentamientosFaccion(faccion);
  const cap = gameStore.capFundacion(faccion.nivel);
  const propios = state.asentamientos.filter((a) => a.faccionId === faccion.id);

  const xpTexto = nivelFaccion.esMaximo
    ? 'Nivel máximo'
    : `${nivelFaccion.experiencia}/${nivelFaccion.umbralSiguiente} XP (desde ${nivelFaccion.umbralActual})`;
  const xpPorcentaje = nivelFaccion.esMaximo
    ? 100
    : Math.round(((nivelFaccion.experiencia - nivelFaccion.umbralActual) / (nivelFaccion.umbralSiguiente! - nivelFaccion.umbralActual)) * 100);

  const asentamientosHtml = propios.length
    ? `<table class="mini-table">
        <thead><tr><th>Asentamiento</th><th>Nivel</th><th>Nivel operativo</th><th>Población</th></tr></thead>
        <tbody>
          ${propios
            .map((a) => {
              const totalPob = a.poblacion.pesants + a.poblacion.artesanos + a.poblacion.nobleza;
              return `<tr><td>${a.nombre ?? a.id}</td><td>${a.nivel}</td><td>${a.nivelActual}</td><td>${totalPob}</td></tr>`;
            })
            .join('')}
        </tbody>
      </table>`
    : '<p class="legend-note">Sin asentamientos.</p>';

  const relacionesDeLaFaccion = state.relaciones.filter((r) => r.faccionAId === faccion.id || r.faccionBId === faccion.id);
  const relacionesHtml = relacionesDeLaFaccion.length
    ? `<table class="mini-table">
        <thead><tr><th>Tipo</th><th>Con</th><th>Rol</th><th>Estado</th><th>Tributo</th></tr></thead>
        <tbody>
          ${relacionesDeLaFaccion
            .map((r) => {
              const esA = r.faccionAId === faccion.id;
              const otraId = esA ? r.faccionBId : r.faccionAId;
              const otraNombre = state.facciones.find((f) => f.id === otraId)?.nombre ?? otraId;
              const rol = r.tipo === 'vasallaje' ? (esA ? 'Señora' : 'Vasalla') : '—';
              const tributo = r.tributo ? `${r.tributo.cantidadPorMinuto}/min ${RECURSO_NOMBRE[r.tributo.recurso] ?? r.tributo.recurso}` : '—';
              return `<tr><td>${r.tipo}</td><td>${otraNombre}</td><td>${rol}</td><td>${r.estado}</td><td>${tributo}</td></tr>`;
            })
            .join('')}
        </tbody>
      </table>`
    : '<p class="legend-note">Sin relaciones diplomáticas.</p>';

  const ligas = gameStore.getLigas(state.relaciones, state.facciones);
  const ligaDeLaFaccion = ligas.find((l) => l.miembrosFaccionIds.includes(faccion.id));
  const ligaHtml = ligaDeLaFaccion
    ? `<div class="kv-row"><span>Miembros</span><span>${ligaDeLaFaccion.miembrosFaccionIds.map((id) => state.facciones.find((f) => f.id === id)?.nombre ?? id).join(', ')}</span></div>
       ${
         ligaDeLaFaccion.tieneVasallaje
           ? `<div class="kv-row"><span>Gran Rey</span><span>${state.facciones.find((f) => f.id === ligaDeLaFaccion.granReyFaccionId)?.reyId ?? '—'}</span></div>`
           : ''
       }`
    : '<p class="legend-note">No pertenece a ninguna Liga.</p>';

  const titulosDeLaFaccion = state.titulos.filter((t) => t.poseedorId === faccion.id);
  const titulosHtml = titulosDeLaFaccion.length
    ? `<div class="chip-row">${titulosDeLaFaccion.map((t) => `<span class="chip">${t.nombre}</span>`).join('')}</div>`
    : '<p class="legend-note">Sin títulos.</p>';

  // Ceder la Facción al NPC de gobernanza (`session/npcGobernanza.ts`).
  const esNpc = state.faccionesNpcIds.includes(faccion.id);
  const controlHtml = `
      <div class="detail-section">
        <h3>Control</h3>
        <label class="npc-toggle">
          <input type="checkbox" id="faccion-npc-toggle" data-faccion="${faccion.id}" ${esNpc ? 'checked' : ''} />
          Controlada por NPC (juega sola)
        </label>
        <p class="legend-note">
          El NPC asume el papel de Gobernador/Tesorero/Rey de esta Facción en el próximo avance del mundo: si todavía
          no tiene ningún asentamiento se funda uno solo (5 fundadores propios); luego cargos y reserva de
          madera, Mercado y caravana propia, trueques de supervivencia (solo con otras Facciones NPC),
          reclutamiento, ataque a campamentos de bandidos y expansión con Caravanas de Fundación. Se puede
          retomar el control manual en cualquier momento. El trueque de especialización entre asentamientos
          propios requiere además <code>SIMULACION_AUTO_COMERCIO.activo = 1</code> en la pestaña "Valores de
          simulación".
        </p>
      </div>`;

  return `
    <div class="settlement-detail">
      ${controlHtml}
      <div class="detail-section">
        <h3>${faccion.nombre}${esNpc ? ' <span class="chip">NPC</span>' : ''}</h3>
        <div class="kv-grid">
          <div class="kv-row"><span>Nivel</span><span>${faccion.nivel}</span></div>
          <div class="kv-row"><span>Rey</span><span>${faccion.reyId ?? '—'}</span></div>
          <div class="kv-row"><span>Embajador</span><span>${faccion.embajadorId ?? '—'}</span></div>
          <div class="kv-row"><span>Ciudadanos</span><span>${faccion.ciudadanosIds.length}</span></div>
          <div class="kv-row"><span>Reputación</span><span>${faccion.reputacion.toFixed(0)}</span></div>
        </div>
        <div class="kv-row" style="margin-top:6px"><span>Progreso de nivel de Facción</span><span>${xpTexto}</span></div>
        <div class="mantenimiento-bar"><div class="mantenimiento-fill" style="width:${xpPorcentaje}%"></div></div>
      </div>

      <div class="detail-section">
        <h3>Cupo de expansión (Doc 1.7/Fase_0_5 §5)</h3>
        <div class="kv-grid">
          <div class="kv-row"><span>Cap de fundación</span><span>${propios.length}/${cap}</span></div>
          <div class="kv-row"><span>Cupo asentamientos nivel 2</span><span>${cupo.nivel2.ocupados}/${cupo.nivel2.total}</span></div>
          <div class="kv-row"><span>Cupo asentamientos nivel 3</span><span>${cupo.nivel3.ocupados}/${cupo.nivel3.total}</span></div>
        </div>
        <p class="legend-note">El cupo de nivel 2/3 sube con el nivel de Facción (combate, conquista o edificios nuevos completados) — un asentamiento propio que ya cumple los gates pero no tiene cupo libre se queda "elegible" hasta que la Facción suba de nivel o se libere uno.</p>
      </div>

      <div class="detail-section">
        <h3>Asentamientos (${propios.length})</h3>
        ${asentamientosHtml}
      </div>

      <div class="detail-section">
        <h3>Relaciones diplomáticas</h3>
        ${relacionesHtml}
      </div>

      <div class="detail-section">
        <h3>Liga</h3>
        ${ligaHtml}
      </div>

      <div class="detail-section">
        <h3>Títulos (Doc 2.9)</h3>
        ${titulosHtml}
      </div>
    </div>
  `;
}

function renderFaccionesTab(state: GameState): void {
  const cont = document.getElementById('facciones-tab')!;

  if (state.facciones.length === 0) {
    faccionSeleccionadaId = null;
    cont.innerHTML = '<p class="legend-note">Aún no hay Facciones fundadas.</p>';
    return;
  }

  if (!faccionSeleccionadaId || !state.facciones.some((f) => f.id === faccionSeleccionadaId)) {
    faccionSeleccionadaId = state.facciones[0]!.id;
  }

  const botones = state.facciones
    .map(
      (f) =>
        `<button type="button" class="settlement-tab-btn${f.id === faccionSeleccionadaId ? ' active' : ''}" data-faccion="${f.id}">${f.nombre}${
          state.faccionesNpcIds.includes(f.id) ? ' · NPC' : ''
        }</button>`
    )
    .join('');
  const seleccionada = state.facciones.find((f) => f.id === faccionSeleccionadaId)!;
  cont.innerHTML = `<div class="settlement-tab-row">${botones}</div>${renderDetalleFaccion(seleccionada, state)}`;

  cont.querySelectorAll('.settlement-tab-btn').forEach((btn) => {
    btn.addEventListener('click', () => {
      faccionSeleccionadaId = (btn as HTMLElement).dataset.faccion!;
      render();
    });
  });

  const npcToggle = cont.querySelector('#faccion-npc-toggle') as HTMLInputElement | null;
  npcToggle?.addEventListener('change', async () => {
    await gameStore.alternarFaccionNpc(npcToggle.dataset.faccion!, npcToggle.checked);
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
    ? `<div class="log-panel">${historial.map((e) => `<div>[${fmtTiempoMundo(e.momento)}] ${e.mensaje}</div>`).join('')}</div>`
    : '<p class="legend-note">Sin actividad registrada todavía.</p>';

  // Escuadrones reclutados por este jugador (Doc 2.5): cada uno vive en el `asentamiento.escuadrones` donde
  // fue reclutado, así que hay que recorrer TODOS los asentamientos y filtrar por `jugadorId` — un jugador
  // puede tener escuadrones en más de una residencia.
  const escuadronesJugador = state.asentamientos.flatMap((a) =>
    a.escuadrones.filter((e) => e.jugadorId === jugadorId).map((e) => ({ asentamientoId: a.id, escuadron: e }))
  );
  const escuadronesJugadorHtml = escuadronesJugador.length
    ? `<table class="mini-table">
        <thead><tr><th>Escuadrón</th><th>Asentamiento</th><th>Origen</th><th>Nivel</th><th>Cantidad</th><th>Veteranía</th><th>Moral</th></tr></thead>
        <tbody>
          ${escuadronesJugador
            .map(
              ({ asentamientoId, escuadron: e }) =>
                `<tr><td>${e.nombre}${e.heridoHasta ? ' (herido)' : ''}</td><td>${asentamientoId}</td><td>${e.origen}</td><td>${nivelTropaTxt(e.tropaId)}</td><td>${e.cantidad}</td><td>${e.veterania.toFixed(1)}</td><td>${e.moral.toFixed(0)}</td></tr>`
            )
            .join('')}
        </tbody>
      </table>`
    : '<p class="legend-note">Sin escuadrones reclutados.</p>';

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
        <h3>Escuadrones</h3>
        ${escuadronesJugadorHtml}
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

  // Combo de Facción (a petición del usuario, mismo patrón que Asentamientos): filtra a UNA Facción a la vez.
  const faccionesConCiudadanos = state.facciones.filter((f) => f.ciudadanosIds.length > 0);
  const faccionDelSeleccionado = state.facciones.find((f) => f.ciudadanosIds.includes(jugadorSeleccionadoId!))!.id;
  if (!jugadoresFaccionFiltroId || !faccionesConCiudadanos.some((f) => f.id === jugadoresFaccionFiltroId)) {
    jugadoresFaccionFiltroId = faccionDelSeleccionado;
  }
  const faccionFiltroId = jugadoresFaccionFiltroId!;

  const faccionSelectHtml = `<select id="jugadores-faccion-select">
    ${faccionesConCiudadanos
      .map((f) => `<option value="${f.id}"${f.id === faccionFiltroId ? ' selected' : ''}>${f.nombre}</option>`)
      .join('')}
  </select>`;
  const faccionFiltro = faccionesConCiudadanos.find((f) => f.id === faccionFiltroId)!;
  const botones = faccionFiltro.ciudadanosIds
    .map(
      (id) =>
        `<button type="button" class="settlement-tab-btn${id === jugadorSeleccionadoId ? ' active' : ''}" data-jugador="${id}">${id}</button>`
    )
    .join('');
  const gruposHtml = `<div class="faccion-group">
    <div class="faccion-group-header"><span class="swatch" style="background:${faccionColor(faccionFiltroId, state.facciones)}"></span>${faccionSelectHtml}</div>
    <div class="settlement-tab-row">${botones}</div>
  </div>`;

  cont.innerHTML = `${gruposHtml}${renderDetalleJugador(jugadorSeleccionadoId!, state)}`;

  document.getElementById('jugadores-faccion-select')?.addEventListener('change', (ev) => {
    jugadoresFaccionFiltroId = (ev.target as HTMLSelectElement).value;
    const faccion = state.facciones.find((f) => f.id === jugadoresFaccionFiltroId);
    if (faccion && faccion.ciudadanosIds.length > 0) jugadorSeleccionadoId = faccion.ciudadanosIds[0]!;
    render();
  });

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
        <div class="kv-row"><span>Duración</span><span>${CATALOGOS.duracionPoliticaMinutos} min</span></div>
        <p class="legend-note">${efectoPolitica(p)}</p>
      </div>`;
    })
    .join('');

  cont.innerHTML = notaGobernador + tarjetasPolitica;
}

function renderPanelPolitica(state: GameState): void {
  const facH = state.facciones
    .map((f) => {
      const cap = gameStore.capFundacion(f.nivel);
      const propios = state.asentamientos.filter((a) => a.faccionId === f.id).length;
      return `<article class="registro-politica-faccion">
        <header><strong>${f.nombre}</strong><span class="registro-level">Nivel ${f.nivel}</span></header>
        <div class="registro-mini-stats"><span>Asentamientos <b>${propios}/${cap}</b></span><span>Ciudadanos <b>${f.ciudadanosIds.length}</b></span><span>Reputación <b>${f.reputacion.toFixed(0)}</b></span></div>
        <div class="registro-politica-leaders"><span>Rey <b>${f.reyId ?? '—'}</b></span><span>Embajador <b>${f.embajadorId ?? '—'}</b></span></div>
      </article>`;
    })
    .join('');
  const relH = state.relaciones
    .map((r) => {
      const a = state.facciones.find((f) => f.id === r.faccionAId)?.nombre ?? r.faccionAId;
      const b = state.facciones.find((f) => f.id === r.faccionBId)?.nombre ?? r.faccionBId;
      const trib = r.tributo ? ` (tributo ${r.tributo.cantidadPorMinuto}/min ${r.tributo.recurso})` : '';
      return `<div class="registro-politica-relation"><span class="registro-relation-type">${r.tipo}</span><span class="registro-relation-route">${a} <b aria-hidden="true">→</b> ${b}</span><span class="registro-relation-status">${r.estado}${trib}</span></div>`;
    })
    .join('');
  const ligas = gameStore.getLigas(state.relaciones, state.facciones);
  const ligasH = ligas
    .map((liga, i) => {
      const nombres = liga.miembrosFaccionIds.map((id) => state.facciones.find((f) => f.id === id)?.nombre ?? id).join(', ');
      const granRey = liga.granReyFaccionId ? state.facciones.find((f) => f.id === liga.granReyFaccionId)?.reyId ?? '—' : '—';
      return `<div class="registro-liga"><strong>Liga ${i + 1}</strong><span>${nombres}</span>${liga.tieneVasallaje ? `<small>Gran Rey: ${granRey}</small>` : '<small>Sin vasallaje</small>'}</div>`;
    })
    .join('');
  politicaPanelEl.innerHTML = `<section class="registro-section-block"><h3>Facciones</h3><div class="registro-politica-facciones">${facH || '<p class="legend-note">Sin facciones.</p>'}</div></section>
    <section class="registro-section-block"><h3>Relaciones diplomáticas</h3><div class="registro-politica-relations">${relH || '<p class="legend-note">Sin relaciones registradas.</p>'}</div></section>
    <section class="registro-section-block"><h3>Ligas</h3><div class="registro-politica-ligas">${ligasH || '<p class="legend-note">Sin Ligas formadas.</p>'}</div></section>`;
}

function renderPanelMilitar(state: GameState): void {
  militarPanelEl.innerHTML = state.asentamientos
    .map((a) => {
      const tieneFundicion = a.edificios.some((e) => e.tipo === 'fundicion' && e.estado === 'activo');
      const tieneGranFundicion = a.edificios.some((e) => e.tipo === 'granFundicion' && e.estado === 'activo');
      const poder = gameStore.poderMilitarInfo(a);
      const escuadronesHtml =
        a.escuadrones
          .map(
            (e) =>
              `<div class="registro-squad"><div><strong>${e.nombre}</strong><small>${e.id} · ${e.jugadorId}</small></div><span>${e.cantidad} soldados</span><span>${nivelTropaTxt(e.tropaId)}</span><span>Moral ${e.moral.toFixed(0)}</span>${e.heridoHasta ? `<em>Herido hasta ${fmtTiempoMundo(e.heridoHasta)}</em>` : ''}</div>`
          )
          .join('') || '<div>Sin escuadrones.</div>';
      return `<article class="registro-militar-settlement"><header><div><h3>${a.nombre ?? a.id}</h3><p>${state.facciones.find((f) => f.id === a.faccionId)?.nombre ?? a.faccionId}</p></div><div class="registro-military-power"><strong>${poder.soldados}</strong><small>soldados · poder ${poder.poder.toFixed(1)}</small></div></header><div class="registro-military-buildings"><span class="${tieneFundicion ? 'is-active' : ''}">Fundición ${tieneFundicion ? 'activa' : 'inactiva'}</span><span class="${tieneGranFundicion ? 'is-active' : ''}">Gran Fundición ${tieneGranFundicion ? 'activa' : 'inactiva'}</span></div><div class="registro-squad-list">${escuadronesHtml}</div></article>`;
    })
    .join('');

  // Campamentos de bandidos (Doc 1.9): amenaza global del mundo, no de un asentamiento — se muestra aparte.
  const campamentosHtml = state.campamentosBandidos
    .map((c) => `<div class="registro-bandit"><strong>${c.id}</strong><span>Posición ${Math.round(c.posicion.x)}, ${Math.round(c.posicion.y)}</span><b>Poder ${c.poder}</b></div>`)
    .join('');
  militarPanelEl.innerHTML += `<section class="registro-section-block"><h3>Campamentos de bandidos</h3><div class="registro-bandit-list">${campamentosHtml || '<p class="legend-note">Ninguno activo.</p>'}</div></section>`;
}

function renderPanelProgresion(state: GameState): void {
  progresionPanelEl.innerHTML = state.titulos.length
    ? `<div class="registro-title-grid">${state.titulos
        .map((t) => `<article class="registro-title-card"><span class="registro-title-mark">✦</span><div><h3>${t.nombre}</h3><p>${state.facciones.find((f) => f.id === t.poseedorId)?.nombre ?? t.poseedorId}</p></div><strong>${t.valorMetrica.toFixed(0)}<small>métrica</small></strong></article>`)
        .join('')}</div>`
    : '<p class="legend-note registro-empty">Sin títulos calculados todavía (aparecen cuando el mundo avanza).</p>';
}

/** Caravanas realmente en movimiento (a petición del usuario: punto de partida, destino, carga, % de viaje
 * completado y ahora también coordenadas exactas de `posicionActual`, para ubicarlas en el mapa sin ambigüedad
 * — ver el marcador triangular por Facción en `ui/canvas.ts`) — excluye las 'disponibles' paradas en su
 * asentamiento (nada que mostrar de un viaje) y las Caravanas de Fundación (`destinoPosicion`, no
 * `destinoAsentamientoId`: no son parte del comercio, ya tienen su propio bloque en la pestaña Acciones). No
 * incluye columna de "escolta": Fase 0 no modela escolta de jugadores en caravanas todavía (ver
 * `engine/bandidos.ts`) — se avisa en la nota al pie en vez de inventar un dato. */
function caravanasEnRutaHtml(state: GameState): string {
  const enRuta = state.caravanas.filter((c) => c.destinoAsentamientoId);
  if (enRuta.length === 0) return '<p class="legend-note">Ninguna caravana en ruta.</p>';
  const nombreAsentamiento = (id: string) => state.asentamientos.find((a) => a.id === id)?.nombre ?? id;
  const filas = enRuta
    .map((c) => {
      const cargaTxt =
        Object.entries(c.contenido)
          .map(([r, cant]) => `${cant.toFixed(0)} ${RECURSO_NOMBRE[r] ?? r}`)
          .join(', ') || (c.estado === 'retornando' ? 'vacía' : '—');
      const estadoTxt = c.estado === 'retornando' ? 'Volviendo a origen' : c.estado === 'en_transito' ? 'En tránsito' : (c.estado ?? '—');
      const coordsTxt = `(${Math.round(c.posicionActual.x)}, ${Math.round(c.posicionActual.y)})`;
      return `<tr><td>${c.id}</td><td>${nombreAsentamiento(c.origenAsentamientoId!)}</td><td>${nombreAsentamiento(c.destinoAsentamientoId!)}</td><td>${estadoTxt}</td><td>${cargaTxt}</td><td>${coordsTxt}</td><td>${Math.round(c.progreso * 100)}%</td></tr>`;
    })
    .join('');
  return `<table class="mini-table">
      <thead><tr><th>Caravana</th><th>Origen</th><th>Destino</th><th>Estado</th><th>Carga</th><th>Coordenadas</th><th>Viaje completado</th></tr></thead>
      <tbody>${filas}</tbody>
    </table>
    <p class="legend-note">Escolta: no modelada todavía en Fase 0 (ver Doc 3.2/3.6 — el diseño objetivo la deja a elección del jugador).</p>`;
}

function renderPanelEconomia(state: GameState): void {
  const preciosHtml = CATALOGOS.recursosMercado.map((r) => `${r}: ${gameStore.precioReferencia(r).toFixed(2)}`).join(' · ');
  const nombreAsentamiento = (id: string) => {
    const asentamiento = state.asentamientos.find((a) => a.id === id);
    return asentamiento?.nombre ?? id;
  };
  const nombreFaccion = (asentamientoId: string) => {
    const asentamiento = state.asentamientos.find((a) => a.id === asentamientoId);
    return state.facciones.find((f) => f.id === asentamiento?.faccionId)?.nombre ?? asentamiento?.faccionId ?? '—';
  };
  const acuerdosActivos = state.acuerdos.filter((acuerdo) => acuerdo.estado === 'activo');
  const acuerdosHtml = acuerdosActivos.length
    ? `<div class="trade-active-list">
        ${acuerdosActivos
          .map((t) => {
            const progresoA = t.cantidadTotalA > 0 ? Math.min(1, t.cantidadEntregadaA / t.cantidadTotalA) : 1;
            const progresoB = t.cantidadTotalB > 0 ? Math.min(1, t.cantidadEntregadaB / t.cantidadTotalB) : 1;
            const porcentaje = Math.round(((progresoA + progresoB) / 2) * 100);
            const caravanas = state.caravanas.filter((c) => c.tipo === 'comercial' && c.origenAcuerdoId === t.id);
            const caravanasHtml = caravanas.length
              ? caravanas
                  .map((c) => {
                    const destino = c.destinoAsentamientoId ? nombreAsentamiento(c.destinoAsentamientoId) : 'sin destino';
                    const estado = c.estado === 'en_transito' ? 'en tránsito' : c.estado === 'retornando' ? 'retornando' : 'disponible';
                    return `<div class="trade-caravan-item">
                      <span class="trade-caravan-icon" aria-hidden="true">🚚</span>
                      <div class="trade-caravan-route"><strong>${c.id}</strong><small>${nombreAsentamiento(c.origenAsentamientoId)} → ${destino}</small></div>
                      <span class="trade-caravan-status">${estado}</span>
                    </div>`;
                  })
                  .join('')
              : '<div class="trade-caravan-empty">Ninguna asignada</div>';
            return `<div class="trade-active-card">
              <div class="trade-active-card-title"><div><small>ACUERDO DE TRUEQUE</small><strong>${t.id}</strong></div><span>${porcentaje}% completado</span></div>
              <div class="trade-route-banner">
                <div class="trade-route-place"><small>ORIGEN A</small><strong>${nombreAsentamiento(t.asentamientoAId)}</strong><span>${nombreFaccion(t.asentamientoAId)}</span></div>
                <span class="trade-route-arrow" aria-hidden="true">⇄</span>
                <div class="trade-route-place trade-route-place-right"><small>ORIGEN B</small><strong>${nombreAsentamiento(t.asentamientoBId)}</strong><span>${nombreFaccion(t.asentamientoBId)}</span></div>
              </div>
              <div class="trade-active-metrics">
                <div class="trade-delivery-card"><span>📦 Entrega de A</span><strong>${t.cantidadEntregadaA.toFixed(0)} <small>/ ${t.cantidadTotalA} ${RECURSO_NOMBRE[t.recursoA] ?? t.recursoA}</small></strong></div>
                <div class="trade-delivery-card"><span>📦 Entrega de B</span><strong>${t.cantidadEntregadaB.toFixed(0)} <small>/ ${t.cantidadTotalB} ${RECURSO_NOMBRE[t.recursoB] ?? t.recursoB}</small></strong></div>
                <div class="trade-lifetime-card"><span>⏳ Vigencia</span><strong>${fmtTiempoMundo(t.creadoEn)} → ${fmtTiempoMundo(t.expiraEn)}</strong><small>${Math.max(0, Math.round((t.expiraEn - state.instante) / 60000))} min restantes</small></div>
              </div>
              <div class="trade-caravans-block"><div class="trade-subsection-title">🚚 Caravanas asignadas <span>${caravanas.length}</span></div>${caravanasHtml}</div>
              <div class="mantenimiento-bar"><div class="mantenimiento-fill" style="width:${porcentaje}%"></div></div>
            </div>`;
          })
          .join('')}
      </div>`
    : '<p class="legend-note">No hay trueques activos.</p>';
  const ordenesHtml = state.ordenes
    .map(
      (o) =>
        `<div class="trade-order-card"><span class="trade-order-icon">${o.tipo === 'venta' ? '↗' : '↙'}</span><div><strong>${o.tipo === 'venta' ? 'Venta' : 'Compra'} · ${o.recurso}</strong><small>Orden ${o.id} · ${o.estado}</small></div><span class="trade-order-amount">${o.cantidadCumplida.toFixed(0)}/${o.cantidad}<small> @ ${o.precioUnitario.toFixed(2)} oro</small></span></div>`
    )
    .join('');
  economiaPanelEl.innerHTML = `<div class="trade-reference-strip"><span class="trade-reference-icon">🪙</span><div><strong>Precios de referencia</strong><small>Valor orientativo por unidad</small></div><span>${preciosHtml}</span></div>
    <section class="trade-overview-block"><h3>🔄 Trueques activos</h3>${acuerdosHtml}</section>
    <section class="trade-overview-block"><h3>🚚 Caravanas en ruta</h3>${caravanasEnRutaHtml(state)}</section>
    ${ordenesHtml ? `<section class="trade-overview-block"><h3>📋 Órdenes del mercado</h3><div class="trade-orders-list">${ordenesHtml}</div></section>` : ''}`;
}

function renderLeyenda(state: GameState): void {
  // Solo los recursos que realmente se dibujan como punto en el mapa (madera/trigo no tienen nodo propio, Doc 1.4).
  const recursosHtml = RECURSOS_EN_MAPA.map(
    (tipo) => `<div class="legend-row"><span class="swatch" style="background:${RECURSO_COLOR[tipo]}"></span>${RECURSO_NOMBRE[tipo] ?? tipo}</div>`
  ).join('');

  // Terreno: los biomas son la capa de FONDO del mapa (ver `drawTerreno`), así que van primero en la
  // leyenda. Sigue al toggle "Detalle de biomas": misma paleta/lista que se está pintando en el canvas.
  const paletaBiomas = mostrarDetalleBiomas ? BIOMA_COLOR : BIOMA_COLOR_SIMPLE;
  const nombresBiomas = mostrarDetalleBiomas ? BIOMA_NOMBRE : BIOMA_NOMBRE_SIMPLE;
  const biomasHtml = nombresBiomas
    .map(([bioma, nombre]) => `<div class="legend-row"><span class="swatch-poly" style="background:${paletaBiomas[bioma]}"></span>${nombre}</div>`)
    .join('');

  const edificiosHtml = Object.entries(EDIFICIO_COLOR)
    .map(([tipo, color]) => `<div class="legend-row"><span class="swatch-square" style="background:${color}"></span>${EDIFICIO_NOMBRE[tipo] ?? tipo}</div>`)
    .join('');

  const faccionesHtml =
    state.facciones
      .map((f, i) => `<div class="legend-row"><span class="swatch" style="background:${FACCION_COLORES[i % FACCION_COLORES.length]}"></span>${f.nombre}</div>`)
      .join('') || '<div class="legend-note">Sin Facciones.</div>';

  const legendMundoHtml = `
    <div class="legend-group">
      <h3>Terreno (de menor a mayor altura)</h3>
      ${biomasHtml}
      <div class="legend-row"><span class="swatch-poly" style="background:rgba(38,90,145,0.9)"></span>Río (grueso = navegable, futuro comercio fluvial)</div>
      <div class="legend-note">El relieve se sombrea con luz desde el noroeste: la ladera clara mira a la luz, la oscura queda a la sombra. En Cima no se puede fundar ni extraer.</div>
    </div>
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
      <h3>Otros</h3>
      <div class="legend-row"><span class="swatch-triangle"></span>Caravana en tránsito (color = Facción de origen)</div>
      <div class="legend-row"><span class="swatch-poly" style="background:#8b1a1a"></span>Campamento de bandidos</div>
    </div>
  `;

  const legendFaccionesHtml = `
    <div class="legend-group">
      <h3>Facciones (asentamiento y zona de influencia)</h3>
      ${faccionesHtml}
    </div>
  `;

  legendBodyEl.innerHTML = legendTabActivo === 'mundo' ? legendMundoHtml : legendFaccionesHtml;
}

function renderRegistro(state: GameState): void {
  logEl.innerHTML = state.log.map((e) => `<div>[${fmtTiempoMundo(e.momento)}] ${e.mensaje}</div>`).join('');
}

function actualizarTabs(): void {
  document.querySelectorAll<HTMLButtonElement>('#main-tabs .tab-btn').forEach((btn) => {
    btn.classList.toggle('active', btn.dataset.tab === tabActivo);
  });
  document.getElementById('tab-guerra')!.hidden = tabActivo !== 'guerra';
  if (tabActivo === 'guerra') renderRosterTropas();
  document.getElementById('tab-comercio')!.hidden = tabActivo !== 'comercio';
  document.getElementById('tab-asentamientos')!.hidden = tabActivo !== 'asentamientos';
  document.getElementById('tab-facciones')!.hidden = tabActivo !== 'facciones';
  document.getElementById('tab-jugadores')!.hidden = tabActivo !== 'jugadores';
  document.getElementById('tab-politicas')!.hidden = tabActivo !== 'politicas';
  document.getElementById('tab-registros')!.hidden = tabActivo !== 'registros';
  document.getElementById('tab-generacionMundo')!.hidden = tabActivo !== 'generacionMundo';
  if (tabActivo === 'politicas') renderPoliticasTab();
}

document.getElementById('main-tabs')!.addEventListener('click', (ev) => {
  const btn = (ev.target as HTMLElement).closest('.tab-btn') as HTMLButtonElement | null;
  if (!btn) return;
  tabActivo = btn.dataset.tab as 'guerra' | 'comercio' | 'asentamientos' | 'facciones' | 'jugadores' | 'politicas' | 'registros' | 'generacionMundo';
  actualizarTabs();
});

document.getElementById('tab-comercio')!.addEventListener('click', (ev) => {
  const btn = (ev.target as HTMLElement).closest('[data-commerce-detail-tab]') as HTMLButtonElement | null;
  if (!btn) return;
  comercioDetalleTab = btn.dataset.commerceDetailTab as 'acciones' | 'info';
  document.querySelectorAll<HTMLButtonElement>('[data-commerce-detail-tab]').forEach((tab) => {
    const activo = tab.dataset.commerceDetailTab === comercioDetalleTab;
    tab.classList.toggle('active', activo);
    tab.setAttribute('aria-selected', String(activo));
  });
  document.querySelectorAll<HTMLElement>('[data-commerce-detail-panel]').forEach((panel) => {
    const activo = panel.dataset.commerceDetailPanel === comercioDetalleTab;
    panel.hidden = !activo;
    panel.classList.toggle('active', activo);
  });
});

actualizarTabs();

document.getElementById('legend-toggle')!.addEventListener('click', () => {
  legendEl.classList.toggle('collapsed');
  const toggle = document.getElementById('legend-toggle')!;
  toggle.textContent = legendEl.classList.contains('collapsed') ? 'Leyenda ▸' : 'Leyenda ▾';
});

document.getElementById('legend-tabs')!.addEventListener('click', (ev) => {
  const btn = (ev.target as HTMLElement).closest('.legend-tab-btn') as HTMLButtonElement | null;
  if (!btn) return;
  legendTabActivo = btn.dataset.legendTab as 'mundo' | 'facciones';
  document.querySelectorAll<HTMLButtonElement>('#legend-tabs .legend-tab-btn').forEach((b) => {
    b.classList.toggle('active', b.dataset.legendTab === legendTabActivo);
  });
  renderLeyenda(gameStore.getState());
});

document.getElementById('fertilidad-checkbox')!.addEventListener('change', (ev) => {
  mostrarFiltroFertilidad = (ev.target as HTMLInputElement).checked;
  render();
});

document.getElementById('biomas-checkbox')!.addEventListener('change', (ev) => {
  mostrarDetalleBiomas = (ev.target as HTMLInputElement).checked;
  terrenoCache = null; // fuerza a repintar la capa de terreno con la paleta nueva (ver `terrenoCacheParaFrame`).
  renderLeyenda(gameStore.getState());
  render();
});

// Toggle de vista Mundo/Asentamiento (a petición del usuario): cambia qué dibuja el canvas principal.
const vistaAsentamientoSelectEl = document.getElementById('vista-asentamiento-select') as HTMLSelectElement;
document.getElementById('vista-mapa-toggle')!.addEventListener('click', (ev) => {
  const btn = (ev.target as HTMLElement).closest('.vista-btn') as HTMLButtonElement | null;
  if (!btn) return;
  vistaMapa = btn.dataset.vista as 'mundo' | 'asentamiento';
  // Al entrar a la vista de asentamiento sin ninguno elegido, cae al primero disponible.
  if (vistaMapa === 'asentamiento' && !asentamientoSeleccionadoId) {
    asentamientoSeleccionadoId = gameStore.getState().asentamientos[0]?.id ?? null;
  }
  ocultarTooltipEdificioAsentamiento();
  render();
});
vistaAsentamientoSelectEl.addEventListener('change', () => {
  asentamientoSeleccionadoId = vistaAsentamientoSelectEl.value || null;
  ocultarTooltipEdificioAsentamiento();
  render();
});

/** Sincroniza los controles de la barra del mapa con la vista activa: botón resaltado, opciones del selector
 * de asentamiento y visibilidad de los toggles que solo tienen sentido en el mapa general. */
function actualizarControlesVista(state: GameState): void {
  document.querySelectorAll<HTMLButtonElement>('#vista-mapa-toggle .vista-btn').forEach((b) => {
    b.classList.toggle('active', b.dataset.vista === vistaMapa);
  });
  const enAsentamiento = vistaMapa === 'asentamiento';
  vistaAsentamientoSelectEl.hidden = !enAsentamiento;
  document.querySelectorAll<HTMLElement>('.vista-mundo-only').forEach((el) => {
    el.style.display = enAsentamiento ? 'none' : '';
  });
  if (enAsentamiento) {
    const opciones = state.asentamientos
      .map((a) => `<option value="${a.id}">${etiquetaAsentamiento(a, state.facciones)}</option>`)
      .join('');
    vistaAsentamientoSelectEl.innerHTML = opciones || '<option value="">— sin asentamientos —</option>';
    if (asentamientoSeleccionadoId && state.asentamientos.some((a) => a.id === asentamientoSeleccionadoId)) {
      vistaAsentamientoSelectEl.value = asentamientoSeleccionadoId;
    }
  }
}

/** Devuelve el canvas offscreen con la capa de terreno para este mundo/tamaño, regenerándolo solo si
 * cambió la seed, la región (Fase 0.2 — misma seed con región distinta es un mundo distinto, ver
 * `Mapa.region`), el tamaño del canvas visible o el toggle de detalle de biomas (ver `terrenoCache`). */
function terrenoCacheParaFrame(mapa: DrawState['mapa']): HTMLCanvasElement {
  const key = `${mapa.seed}-${mapa.region ?? 'libre'}-${canvas.width}x${canvas.height}-${mostrarDetalleBiomas}`;
  if (terrenoCache?.key !== key) {
    const off = document.createElement('canvas');
    off.width = canvas.width;
    off.height = canvas.height;
    drawTerreno(off.getContext('2d')!, off, mapa, mostrarDetalleBiomas);
    terrenoCache = { key, canvas: off };
  }
  return terrenoCache.canvas;
}

function render(): void {
  const state = gameStore.getState();

  relojMundoEl.textContent = `🕑 ${fmtTiempoMundo(state.instante)}`;
  actualizarControlesVista(state);
  // Si el asentamiento en vista ya no existe (p. ej. cayó en ruinas o se importó otra partida), se vuelve al mapa general.
  const asentamientoEnVista = state.asentamientos.find((a) => a.id === asentamientoSeleccionadoId);
  if (vistaMapa === 'asentamiento' && asentamientoEnVista) {
    const trazado = gameStore.getTrazadoAsentamiento(asentamientoEnVista);
    drawAsentamiento(ctx, canvas, {
      asentamiento: asentamientoEnVista,
      etiqueta: etiquetaAsentamiento(asentamientoEnVista, state.facciones),
      calles: trazado.calles,
      caminos: trazado.caminos,
      huellas: trazado.huellas,
      tamanoCelda: CATALOGOS.tamanoCeldaAsentamiento,
      radioMapa: CATALOGOS.radioMapaAsentamiento,
    });
  } else {
    const zonas = gameStore.getZonas(state.asentamientos);
    const drawState: DrawState = {
      mapa: gameStore.getMapa(state),
      asentamientos: state.asentamientos,
      zonasFusionadas: gameStore.getZonasFusionadas(zonas, state.asentamientos),
      facciones: state.facciones,
      caravanas: state.caravanas,
      caminos: state.caminos,
      campamentosBandidos: state.campamentosBandidos,
      ejercitos: state.ejercitos,
    };
    draw(ctx, canvas, drawState, terrenoCacheParaFrame(drawState.mapa));
    if (mostrarFiltroFertilidad) drawFiltroFertilidad(ctx, canvas, gameStore.getMapa(state));
  }
  actualizarSelects(state);
  actualizarInfoTruequeAsentamientos(state);
  actualizarInfoFlota(state);
  renderPanelAsentamientos(state);
  renderAsentamientosTab(state);
  renderFaccionesTab(state);
  renderJugadoresTab(state);
  renderPanelPolitica(state);
  renderPanelProgresion(state);
  renderPanelMilitar(state);
  renderPanelEconomia(state);
  renderLeyenda(state);
  renderRegistro(state);
  renderInfoPartida(state);
}

// Única suscripción: cualquier acción del store dispara un re-render. La interfaz nunca
// vuelve a llamar `render()` manualmente tras una acción — eso sería recrear el acoplamiento.
gameStore.subscribe(render);

function ocultarTooltipEdificioAsentamiento(): void {
  settlementBuildingTooltipEl.hidden = true;
}

function actualizarTooltipEdificioAsentamiento(ev: MouseEvent): void {
  if (vistaMapa !== 'asentamiento') {
    ocultarTooltipEdificioAsentamiento();
    return;
  }

  const state = gameStore.getState();
  const asentamiento = state.asentamientos.find((a) => a.id === asentamientoSeleccionadoId);
  if (!asentamiento) {
    ocultarTooltipEdificioAsentamiento();
    return;
  }

  const trazado = gameStore.getTrazadoAsentamiento(asentamiento);
  const rect = canvas.getBoundingClientRect();
  const pixelX = (ev.clientX - rect.left) * (canvas.width / rect.width);
  const pixelY = (ev.clientY - rect.top) * (canvas.height / rect.height);
  const escala = (canvas.width * 0.92) / (CATALOGOS.radioMapaAsentamiento * 2);
  const localX = (pixelX - canvas.width / 2) / escala;
  const localY = (pixelY - canvas.height / 2) / escala;
  const edificio = [...asentamiento.edificios]
    .reverse()
    .find((e) => {
      if ((e.ambito ?? 'asentamiento') === 'mapa') return false;
      const huella = trazado.huellas[e.id];
      return !!huella && localX >= huella.x && localX <= huella.x + huella.ancho && localY >= huella.y && localY <= huella.y + huella.alto;
    });

  if (!edificio) {
    ocultarTooltipEdificioAsentamiento();
    return;
  }

  const economia = gameStore.edificioEconomiaInfo(asentamiento, edificio);
  const estado = edificio.estado === 'activo' ? 'Activo' : edificio.estado === 'en_construccion' ? 'En construcción' : 'En cola';
  const estadoTxt = edificio.danado ? `${estado} · dañado (reconstrucción, Doc 5.12.9)` : estado;
  const filasEconomia = (items: { recurso: string; cantidadPorMinuto: number }[], vacio: string) =>
    items.length
      ? items.map((item) => `<div><span>${RECURSO_NOMBRE[item.recurso] ?? item.recurso}</span><strong>${item.cantidadPorMinuto.toFixed(1)}/min</strong></div>`).join('')
      : `<span class="settlement-tooltip-muted">${vacio}</span>`;

  settlementBuildingTooltipEl.innerHTML = `
    <div class="settlement-tooltip-title">${EDIFICIO_NOMBRE[edificio.tipo] ?? edificio.tipo}</div>
    <div class="settlement-tooltip-meta">Nivel ${edificio.nivelInterno ?? 1} · ${estadoTxt}</div>
    <div class="settlement-tooltip-group"><span>Producción</span>${filasEconomia(economia.produccion, 'Sin producción modelada')}</div>
    <div class="settlement-tooltip-group"><span>Consumo</span>${filasEconomia(economia.consumo, 'Sin consumo modelado')}</div>
  `;
  settlementBuildingTooltipEl.hidden = false;

  const panelRect = mapPanelEl.getBoundingClientRect();
  const maxLeft = Math.max(8, mapPanelEl.clientWidth - settlementBuildingTooltipEl.offsetWidth - 8);
  const maxTop = Math.max(8, mapPanelEl.clientHeight - settlementBuildingTooltipEl.offsetHeight - 8);
  settlementBuildingTooltipEl.style.left = `${Math.min(maxLeft, Math.max(8, ev.clientX - panelRect.left + 14))}px`;
  settlementBuildingTooltipEl.style.top = `${Math.min(maxTop, Math.max(8, ev.clientY - panelRect.top + 14))}px`;
}

canvas.addEventListener('mousemove', (ev) => {
  if (vistaMapa === 'asentamiento') actualizarTooltipEdificioAsentamiento(ev);
});

canvas.addEventListener('mouseleave', () => {
  if (vistaMapa === 'asentamiento') ocultarTooltipEdificioAsentamiento();
});

document.getElementById('refrescar-btn')!.addEventListener('click', () => {
  void gameStore.refrescar();
});

// El mundo avanza SOLO en el servidor (reloj de mundo, Fase D / D5). Esta interfaz no tiene sincronización
// en vivo (ni WebSocket ni deltas), así que re-lee el estado cada 5 s mientras la pestaña esté visible —
// suficiente para un panel de administración, sin martillear el servidor cuando nadie lo mira.
setInterval(() => {
  if (!document.hidden) void gameStore.refrescar();
}, 5000);

document.getElementById('regenerar-btn')!.addEventListener('click', async () => {
  const confirmado = window.confirm('Esto descarta la partida actual y crea una nueva. Se pierde todo el progreso. ¿Continuar?');
  if (!confirmado) return;
  const region = regionSelect.value as RegionId | '';
  await gameStore.regenerarMundo(Number(seedInput.value) || 0, region || undefined);
});

document.getElementById('exportar-btn')!.addEventListener('click', () => {
  const json = gameStore.exportarSimulacion();
  const blob = new Blob([json], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const enlace = document.createElement('a');
  enlace.href = url;
  enlace.download = `bronze-age-sim-${new Date(gameStore.getState().instante).toISOString().slice(0, 16).replace(/[:T]/g, '-')}.json`;
  enlace.click();
  URL.revokeObjectURL(url);
});

render();
