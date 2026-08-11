// Capa de interfaz (DOM/canvas). Regla de frontera: este archivo — y cualquier otro bajo `ui/` —
// solo puede importar de `./app/gameStore` (acciones + estado + suscripción) y de `./ui/canvas`
// (dibujo/color, puramente presentacional). Nunca importa nada de `./engine/*` ni captura errores
// de dominio: eso es responsabilidad exclusiva de `gameStore`. Los tipos de `./domain/types` se
// importan solo como `type` para tipar lo que se lee — no acoplan a ninguna lógica.
import type { Asentamiento, BiomaTipo, CargoTipo, EdificioTipo, Faccion, RegionId } from './domain/types';
import { CATALOGOS, gameStore, type GameState, type CampoBalance } from './app/gameStore';
import { draw, drawFiltroFertilidad, drawPreviewFundacion, drawTerreno, faccionColor, BIOMA_COLOR, BIOMA_COLOR_SIMPLE, RECURSO_COLOR, RECURSOS_EN_MAPA, EDIFICIO_COLOR, FACCION_COLORES, type DrawState } from './ui/canvas';

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
  maravilla: 'Maravilla',
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
  maravilla: 'Edificio trofeo de coste extremo — requiere asentamiento en nivel máximo (3). Solo se añade a la cola manualmente (Gobernador/Maestro de Obras). El ciclo de servidor que se cerraría al completarla no está implementado todavía.',
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
 * o el efecto especial de campos no multiplicativos como `minimoLenerasPrioritario`). */
function efectoPolitica(politica: (typeof CATALOGOS.politicas)[number]): string {
  const registro = politica as unknown as Record<string, unknown>;
  const efectos: string[] = [];
  if (typeof registro.minimoLenerasPrioritario === 'number') {
    efectos.push(`Prioriza Leñeras: bloquea el resto de auto-construcción hasta tener ${registro.minimoLenerasPrioritario} (activas o en curso)`);
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
let tabActivo: 'acciones' | 'guerra' | 'comercio' | 'asentamientos' | 'jugadores' | 'politicas' | 'balance' = 'acciones';
let asentamientoSeleccionadoId: string | null = null;
let jugadorSeleccionadoId: string | null = null;
/** Filtro de Facción (a petición del usuario): con muchas Facciones, listar el grupo de cada una a la vez
 * dejaba de caber en pantalla — las pestañas Asentamientos/Jugadores filtran a una Facción por combobox. */
let asentamientosFaccionFiltroId: string | null = null;
let jugadoresFaccionFiltroId: string | null = null;
/** Selector de escuadrones propios en Combate (a petición del usuario): chips en vez de ids escritos a mano
 * — se limpia solo al cambiar de Facción/Asentamiento o si un escuadrón deja de existir (ver `actualizarCombateEscuadrones`). */
const combateEscuadronesSeleccionados = new Set<string>();
/** Tick que el slider de línea de tiempo está mostrando. Sigue al tick en vivo salvo que el usuario arrastre hacia atrás. */
let viewedTick = 0;
let ultimoTickEnVivo = 0;
let mostrarFiltroFertilidad = false;
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
/** Grupos de la pestaña "Valores de simulación" que el usuario dejó expandidos (persiste solo en esta vista). */
const balanceGruposAbiertos = new Set<string>();
let balanceFiltro = '';

const app = document.getElementById('app')!;
app.innerHTML = `
  <div class="controls-panel">
    <h1>Bronze Age Collapse — Fase 0</h1>
    <div class="tabs" id="main-tabs">
      <button class="tab-btn" data-tab="acciones">Acciones</button>
      <button class="tab-btn" data-tab="guerra">Guerra</button>
      <button class="tab-btn" data-tab="comercio">Comercio</button>
      <button class="tab-btn" data-tab="asentamientos">Asentamientos</button>
      <button class="tab-btn" data-tab="jugadores">Jugadores</button>
      <button class="tab-btn" data-tab="politicas">Políticas</button>
      <button class="tab-btn" data-tab="balance">Valores de simulación</button>
    </div>

    <div class="tab-panel" id="tab-acciones">
    <div class="controls-grid">
      <div class="controls">
        <h2>Facciones (Doc 0)</h2>
        <label>Nombre de la nueva Facción <input id="faccion-crear-nombre" type="text" placeholder="Nombre de la Facción" /></label>
        <button id="faccion-crear-btn">Fundar Facción</button>
      </div>

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
        <div id="fundacion-viabilidad" class="fundacion-viabilidad">Pasa el cursor por el mapa para evaluar un emplazamiento.</div>
        <label>
          Seed del mundo
          <input id="seed-input" type="number" value="1" />
        </label>
        <label>
          Región geográfica (Fase 0.2)
          <select id="region-select">
            <option value="">Libre (procedural, sin sesgo)</option>
            ${Object.entries(REGION_NOMBRE)
              .map(([id, nombre]) => `<option value="${id}">${nombre}</option>`)
              .join('')}
          </select>
        </label>
      </div>

      <div class="controls">
        <h2>Caravana de Fundación (Doc 1.8)</h2>
        <label>Asentamiento de origen (nivel ≥2) <select id="expansion-origen"></select></label>
        <label><input type="checkbox" id="expansion-modo-clic" /> Elegir destino con clic en el mapa</label>
        <label>Destino elegido <input id="expansion-destino" type="text" readonly placeholder="clic en el mapa…" /></label>
        <label>Ciudadanos que fundarán (1-5) <input id="expansion-jugadores" type="number" value="1" min="1" max="5" /></label>
        <button id="expansion-lanzar-btn">Lanzar Caravana de Fundación</button>
        <label>Caravana de Fundación en tránsito <select id="expansion-caravana"></select></label>
        <button id="expansion-desarmar-btn">Desarmar (reembolso íntegro)</button>
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

    </div>
    </div>

    <div class="tab-panel" id="tab-guerra" hidden>
    <div class="controls-grid">
      <div class="controls">
        <h2>Reclutamiento (Doc 5.7/5.8)</h2>
        <label>Asentamiento <select id="guerra-asentamiento"></select></label>
        <label>Reclutar tropa (Centro Urbano/Barracón/Galería de tiro) <select id="reclutar-tropa"></select></label>
        <div class="tropa-info" id="reclutar-tropa-info"></div>
        <label>Origen <select id="reclutar-tropa-origen"></select></label>
        <button id="reclutar-tropa-btn">Reclutar tropa</button>
        <p class="legend-note">Cada tropa se recluta en bloque, al tamaño de escuadrón fijo indicado en "Info" arriba — no se elige la cantidad.</p>
        <button id="gran-fundicion-btn">Construir Gran Fundición</button>
      </div>

      <div class="controls">
        <h2>Combate (Doc 5.2/5.10)</h2>
        <label>Facción propia <select id="combate-faccion-select"></select></label>
        <label>Asentamiento propio <select id="combate-asentamiento-select"></select></label>
        <label>Escuadrones propios (clic para seleccionar/deseleccionar)</label>
        <div class="chip-row" id="combate-escuadrones-chips"></div>
        <label>Asentamiento objetivo/rival <select id="guerra-objetivo"></select></label>
        <label>Escuadrones del objetivo (solo campo abierto) <input id="guerra-escuadrones-objetivo" type="text" /></label>
        <button id="asedio-btn">Iniciar asedio</button>
        <button id="campo-abierto-btn">Combate en campo abierto</button>
        <label>Caravana a interceptar <select id="guerra-caravana"></select></label>
        <button id="interceptar-btn">Interceptar caravana</button>
        <label>Campamento de bandidos a atacar (Doc 1.9) <select id="guerra-campamento"></select></label>
        <button id="atacar-campamento-btn">Atacar campamento</button>
      </div>
    </div>

    <div class="detail-section roster-section">
      <h3>Roster de tropas (Doc 5.8)</h3>
      <div id="roster-tropas" class="table-scroll"></div>
    </div>
    </div>

    <div class="tab-panel" id="tab-comercio" hidden>
    <div class="controls-grid">
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
        <h2>Flota de Caravanas (Doc 3.2, ampliación de comercio)</h2>
        <label>Asentamiento <select id="flota-asentamiento"></select></label>
        <div class="tropa-info" id="flota-info"></div>
        <button id="flota-construir-btn">Construir caravana (50 madera)</button>
        <p class="legend-note">Requiere Mercado activo y cupo libre. Las caravanas propias no se pueden desmantelar — solo se pierden si las capturan en combate.</p>
      </div>
    </div>

    <div class="detail-section">
      <h3>Info de comercio (Doc 3.2/3.3)</h3>
      <div id="economia-panel" class="log-panel"></div>
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
      <p class="legend-note">Placeholders de balance (Doc — ver Consideraciones/Preguntas_Abiertas.md). Se editan en caliente: afectan de inmediato a la próxima acción o tick. Los parámetros de generación del mundo (tamaño, nodos, bosques, fertilidad) no están aquí a propósito: son fijos por diseño para que una seed dé siempre el mismo mapa — ver <code>src/worldgen/config.ts</code>.</p>
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
      <div class="map-toggles">
        <label class="fertilidad-toggle"><input type="checkbox" id="fertilidad-checkbox" /> Filtro de fertilidad</label>
        <label class="fertilidad-toggle"><input type="checkbox" id="biomas-checkbox" /> Detalle de biomas</label>
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
const faccionCrearNombreInput = document.getElementById('faccion-crear-nombre') as HTMLInputElement;
const seedInput = document.getElementById('seed-input') as HTMLInputElement;
const regionSelect = document.getElementById('region-select') as HTMLSelectElement;
const jugadoresInput = document.getElementById('jugadores-input') as HTMLInputElement;

const fundacionViabilidadEl = document.getElementById('fundacion-viabilidad')!;
/** Posición del cursor sobre el mapa, para previsualizar el emplazamiento antes de fundar (ver `render`). */
let hoverFundacion: { x: number; y: number } | null = null;

const expansionOrigenSelect = document.getElementById('expansion-origen') as HTMLSelectElement;
const expansionModoClicCheckbox = document.getElementById('expansion-modo-clic') as HTMLInputElement;
const expansionDestinoInput = document.getElementById('expansion-destino') as HTMLInputElement;
const expansionJugadoresInput = document.getElementById('expansion-jugadores') as HTMLInputElement;
const expansionCaravanaSelect = document.getElementById('expansion-caravana') as HTMLSelectElement;
let expansionDestino: { x: number; y: number } | null = null;

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
const reclutarTropaSelect = document.getElementById('reclutar-tropa') as HTMLSelectElement;
const reclutarTropaOrigenSelect = document.getElementById('reclutar-tropa-origen') as HTMLSelectElement;
const reclutarTropaInfoEl = document.getElementById('reclutar-tropa-info')!;
const rosterTropasEl = document.getElementById('roster-tropas')!;
const combateFaccionSelect = document.getElementById('combate-faccion-select') as HTMLSelectElement;
const combateAsentamientoSelect = document.getElementById('combate-asentamiento-select') as HTMLSelectElement;
const combateEscuadronesChipsEl = document.getElementById('combate-escuadrones-chips')!;
const guerraObjetivoSelect = document.getElementById('guerra-objetivo') as HTMLSelectElement;
const guerraEscuadronesObjetivoInput = document.getElementById('guerra-escuadrones-objetivo') as HTMLInputElement;
const guerraCaravanaSelect = document.getElementById('guerra-caravana') as HTMLSelectElement;
const guerraCampamentoSelect = document.getElementById('guerra-campamento') as HTMLSelectElement;
const militarPanelEl = document.getElementById('militar-panel')!;
const flotaAsentamientoSelect = document.getElementById('flota-asentamiento') as HTMLSelectElement;
const flotaInfoEl = document.getElementById('flota-info')!;
const progresionPanelEl = document.getElementById('progresion-panel')!;

cargoTipoSelect.innerHTML = CATALOGOS.cargos.map((c) => `<option value="${c}">${c}</option>`).join('');
politicaCargoSelect.innerHTML = CATALOGOS.cargos.map((c) => `<option value="${c}">${c}</option>`).join('');
politicaIdSelect.innerHTML = CATALOGOS.politicas.map((p) => `<option value="${p.id}">${p.nombre} (${p.cargo})</option>`).join('');
diploTributoRecursoSelect.innerHTML = CATALOGOS.recursosTrueque.map((r) => `<option value="${r}">${r}</option>`).join('');
truequeRecursoASelect.innerHTML = CATALOGOS.recursosTrueque.map((r) => `<option value="${r}">${r}</option>`).join('');
truequeRecursoBSelect.innerHTML = CATALOGOS.recursosTrueque.map((r) => `<option value="${r}">${r}</option>`).join('');
mercadoRecursoSelect.innerHTML = CATALOGOS.recursosMercado.map((r) => `<option value="${r}">${r}</option>`).join('');
reclutarTropaOrigenSelect.innerHTML = CATALOGOS.origenesTropa.map((o) => `<option value="${o}">${o}</option>`).join('');
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

/** Desglose de costo de una tropa, por soldado y para el escuadrón completo (`unidadesPorDefecto`, tamaño fijo). */
function costoTropaTxt(tropa: (typeof CATALOGOS.tropasReclutables)[number], porSoldado: boolean): string {
  const entradas = Object.entries(tropa.costoEquipo);
  if (entradas.length === 0) return '—';
  return entradas
    .map(([r, c]) => `${(c ?? 0) * (porSoldado ? 1 : tropa.unidadesPorDefecto)} ${RECURSO_NOMBRE[r] ?? r}`)
    .join(' + ');
}

/** Segmento "Info:" bajo el combo de reclutamiento (a petición del usuario): toda la info de la tropa
 * seleccionada — edificio/nivel exigido, tamaño fijo del escuadrón, costo por soldado y total, poder base. */
function actualizarInfoTropa(): void {
  const tropa = CATALOGOS.tropasReclutables.find((t) => t.id === reclutarTropaSelect.value);
  if (!tropa) {
    reclutarTropaInfoEl.innerHTML = '';
    return;
  }
  reclutarTropaInfoEl.innerHTML = `
    <div class="kv-row"><span>Info:</span><span>${tropa.nombre}</span></div>
    <div class="kv-row"><span>Edificio requerido</span><span>${edificioRequeridoTxt(tropa)}</span></div>
    <div class="kv-row"><span>Unidades por escuadrón</span><span>${tropa.unidadesPorDefecto} (tamaño fijo, no elegible)</span></div>
    <div class="kv-row"><span>Costo por soldado</span><span>${costoTropaTxt(tropa, true)}</span></div>
    <div class="kv-row"><span>Costo total del escuadrón</span><span>${costoTropaTxt(tropa, false)}</span></div>
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
  `;
}
flotaAsentamientoSelect.addEventListener('change', () => actualizarInfoFlota(gameStore.getState()));

/** Desglose de costo de un edificio del catálogo (para el segmento "Info:" del selector de "añadir a la cola"). */
function costoEdificioTxt(entrada: (typeof CATALOGOS.catalogoEdificios)[number]): string {
  const entradas = Object.entries(entrada.costo);
  if (entradas.length === 0) return '—';
  return entradas.map(([r, c]) => `${c} ${RECURSO_NOMBRE[r] ?? r}`).join(' + ');
}

/** Segmento "Info:" bajo el selector de "añadir a la cola" (Doc 4.2, a petición del usuario) — mismo patrón
 * que `actualizarInfoTropa`: costo de construcción, tiempo y gates de nivel antes de confirmar. El selector
 * se regenera en cada `renderAsentamientosTab` (vive dentro del detalle del asentamiento), así que esta
 * función se llama/recablea ahí, a diferencia de `actualizarInfoTropa` (selector estático, cableado una vez). */
function actualizarInfoEdificioCola(): void {
  const select = document.getElementById('cola-tipo-select') as HTMLSelectElement | null;
  const info = document.getElementById('cola-tipo-info');
  if (!select || !info) return;
  const entrada = CATALOGOS.catalogoEdificios.find((e) => e.tipo === select.value);
  if (!entrada) {
    info.innerHTML = '';
    return;
  }
  const gatesTxt = [
    entrada.requisitoNivelAsentamiento > 0 ? `nivel de asentamiento ${entrada.requisitoNivelAsentamiento}` : null,
    entrada.requisitoNivelFaccion > 0 ? `nivel de Facción ${entrada.requisitoNivelFaccion}` : null,
  ]
    .filter((s): s is string => s !== null)
    .join(', ');
  info.innerHTML = `
    <div class="kv-row"><span>Info:</span><span>${EDIFICIO_NOMBRE[entrada.tipo] ?? entrada.tipo}</span></div>
    <div class="kv-row"><span>Costo de construcción</span><span>${costoEdificioTxt(entrada)}</span></div>
    <div class="kv-row"><span>Tiempo de construcción</span><span>${entrada.tiempoConstruccionTicks} ticks</span></div>
    ${gatesTxt ? `<div class="kv-row"><span>Requisitos</span><span>${gatesTxt}</span></div>` : ''}
  `;
}

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

/** Selector de escuadrones propios en Combate (a petición del usuario): cascada Facción → Asentamiento →
 * chips de escuadrones, en vez de escribir ids de escuadrón a mano. El asentamiento solo ofrece los de la
 * Facción elegida (mismo patrón que `actualizarCargoJugadorSelect`); los chips seleccionados llevan borde
 * verde, el resto borde blanco. La selección se limpia sola si el escuadrón deja de existir (aniquilado,
 * o cambia la Facción/Asentamiento elegido). */
function actualizarCombateEscuadrones(state: GameState): void {
  const asentamientosDeFaccion = state.asentamientos.filter((a) => a.faccionId === combateFaccionSelect.value);

  const seleccionAsentamientoPrevia = combateAsentamientoSelect.value;
  combateAsentamientoSelect.innerHTML = asentamientosDeFaccion.length
    ? asentamientosDeFaccion.map((a) => `<option value="${a.id}">${a.id}</option>`).join('')
    : '<option value="">Sin asentamientos en esta Facción</option>';
  if (asentamientosDeFaccion.some((a) => a.id === seleccionAsentamientoPrevia)) {
    combateAsentamientoSelect.value = seleccionAsentamientoPrevia;
  }

  const asentamiento = asentamientosDeFaccion.find((a) => a.id === combateAsentamientoSelect.value);
  const escuadrones = asentamiento?.escuadrones.filter((e) => e.cantidad > 0) ?? [];
  for (const id of combateEscuadronesSeleccionados) {
    if (!escuadrones.some((e) => e.id === id)) combateEscuadronesSeleccionados.delete(id);
  }

  combateEscuadronesChipsEl.innerHTML = escuadrones.length
    ? escuadrones
        .map((e) => {
          const nombreTropa = CATALOGOS.tropasReclutables.find((t) => t.id === e.tropaId)?.nombre ?? e.nombre;
          const seleccionado = combateEscuadronesSeleccionados.has(e.id) ? ' selected' : '';
          return `<button type="button" class="chip-escuadron${seleccionado}" data-escuadron="${e.id}">${nombreTropa} (${e.cantidad})</button>`;
        })
        .join('')
    : '<p class="legend-note">Sin escuadrones en este asentamiento.</p>';

  combateEscuadronesChipsEl.querySelectorAll('.chip-escuadron').forEach((chip) => {
    chip.addEventListener('click', () => {
      const id = (chip as HTMLElement).dataset.escuadron!;
      if (combateEscuadronesSeleccionados.has(id)) combateEscuadronesSeleccionados.delete(id);
      else combateEscuadronesSeleccionados.add(id);
      render();
    });
  });
}

combateFaccionSelect.addEventListener('change', () => {
  combateEscuadronesSeleccionados.clear();
  actualizarCombateEscuadrones(gameStore.getState());
});
combateAsentamientoSelect.addEventListener('change', () => {
  combateEscuadronesSeleccionados.clear();
  actualizarCombateEscuadrones(gameStore.getState());
});

/** CSV de los escuadrones marcados en los chips de Combate (Doc 5.2/5.10) — mismo formato que las acciones
 * `gameStore.*` ya esperaban del antiguo input de texto libre. */
function idsDeChipsCombate(): string {
  return [...combateEscuadronesSeleccionados].join(',');
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
    flotaAsentamientoSelect,
  ]) {
    const seleccionPrevia = select.value;
    select.innerHTML = opcionesAsentamientos;
    if (state.asentamientos.some((a) => a.id === seleccionPrevia)) select.value = seleccionPrevia;
  }

  const opcionesCaravanas = state.caravanas
    .map((c) => {
      const destinoTxt = c.destinoPosicion ? `(${Math.round(c.destinoPosicion.x)}, ${Math.round(c.destinoPosicion.y)})` : c.destinoAsentamientoId;
      return `<option value="${c.id}">${c.id} (${c.origenAsentamientoId} → ${destinoTxt})</option>`;
    })
    .join('');
  const caravanaPrevia = guerraCaravanaSelect.value;
  guerraCaravanaSelect.innerHTML = opcionesCaravanas;
  if (state.caravanas.some((c) => c.id === caravanaPrevia)) guerraCaravanaSelect.value = caravanaPrevia;

  const opcionesCampamentos = state.campamentosBandidos
    .map((c) => `<option value="${c.id}">${c.id} (${Math.round(c.posicion.x)}, ${Math.round(c.posicion.y)}) · poder ${c.poder}</option>`)
    .join('');
  const campamentoPrevio = guerraCampamentoSelect.value;
  guerraCampamentoSelect.innerHTML = opcionesCampamentos || '<option value="">Ningún campamento activo</option>';
  if (state.campamentosBandidos.some((c) => c.id === campamentoPrevio)) guerraCampamentoSelect.value = campamentoPrevio;

  const opcionesAsentamientosNivel2 = state.asentamientos
    .filter((a) => a.nivel >= 2)
    .map((a) => `<option value="${a.id}">${etiquetaAsentamiento(a, state.facciones)} (nivel ${a.nivel})</option>`)
    .join('');
  const origenPrevio = expansionOrigenSelect.value;
  expansionOrigenSelect.innerHTML = opcionesAsentamientosNivel2 || '<option value="">Ningún asentamiento en nivel ≥2</option>';
  if (state.asentamientos.some((a) => a.id === origenPrevio && a.nivel >= 2)) expansionOrigenSelect.value = origenPrevio;

  const caravanasFundacion = state.caravanas.filter((c) => c.tipo === 'construccion' && c.destinoPosicion);
  const opcionesCaravanasFundacion = caravanasFundacion
    .map((c) => `<option value="${c.id}">${c.id} (${c.origenAsentamientoId} → (${Math.round(c.destinoPosicion!.x)}, ${Math.round(c.destinoPosicion!.y)}))</option>`)
    .join('');
  const caravanaFundacionPrevia = expansionCaravanaSelect.value;
  expansionCaravanaSelect.innerHTML = opcionesCaravanasFundacion || '<option value="">Ninguna en tránsito</option>';
  if (caravanasFundacion.some((c) => c.id === caravanaFundacionPrevia)) expansionCaravanaSelect.value = caravanaFundacionPrevia;

  const opcionesFacciones = state.facciones.map((f) => `<option value="${f.id}">${f.nombre}</option>`).join('');
  for (const select of [cargoFaccionSelect, diploASelect, diploBSelect, fusionASelect, fusionBSelect, combateFaccionSelect]) {
    const seleccionPrevia = select.value;
    select.innerHTML = opcionesFacciones;
    if (state.facciones.some((f) => f.id === seleccionPrevia)) select.value = seleccionPrevia;
  }

  actualizarCargoJugadorSelect(state);
  actualizarCombateEscuadrones(state);

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

  // La cola de construcción es única y compartida por TODO el asentamiento (no una por tipo de edificio).
  // Overhaul de auto-construcción: los "en_cola" ya están PAGADOS (el pago ocurre al comprometerse, no al
  // arrancar obra) y el motor los devuelve ordenados por `prioridad` (score de necesidad) — el índice en el
  // array SÍ coincide con el orden real en que competirán por un hueco de obra (ver engine/construction.ts).
  const colaGlobal = a.edificios.filter((e) => e.estado === 'en_cola');
  const posicionEnCola = new Map(colaGlobal.map((e, i) => [e.id, i + 1]));
  const enConstruccionCount = a.edificios.filter((e) => e.estado === 'en_construccion').length;

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
  const poblacion = gameStore.poblacionInfo(a);
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
        <thead><tr><th>Escuadrón</th><th>Origen</th><th>Nivel</th><th>Cantidad</th><th>Veteranía</th><th>Moral</th></tr></thead>
        <tbody>
          ${a.escuadrones
            .map(
              (e) =>
                `<tr><td>${e.nombre}${e.heridoHastaTick ? ' (herido)' : ''}</td><td>${e.origen}</td><td>${nivelTropaTxt(e.tropaId)}</td><td>${e.cantidad}</td><td>${e.veterania.toFixed(1)}</td><td>${e.moral.toFixed(0)}</td></tr>`
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
        <h3>Almacén</h3>
        <div class="kv-grid">${almacenHtml}</div>
      </div>

      <div class="detail-section">
        <h3>Edificios</h3>
        <div class="kv-row">
          <span>Auto-construcción: ${a.autoConstruccionPausada ? 'pausada' : 'activa'}</span>
          <button type="button" class="auto-construccion-toggle-btn" data-settlement="${a.id}">
            ${a.autoConstruccionPausada ? 'Reanudar' : 'Pausar'}
          </button>
        </div>
        <p class="legend-note">
          ${a.autoConstruccionPausada
            ? 'No se detectan nuevas necesidades. Lo ya pagado (en cola/en construcción) sigue avanzando.'
            : `Cupo de obras activas simultáneas: ${enConstruccionCount}/${CATALOGOS.maximoEnConstruccionSimultanea}.`}
        </p>
        ${edificiosHtml}
      </div>

      <div class="detail-section">
        <h3>Cola de construcción — control manual (Gobernador / Maestro de Obras)</h3>
        <p class="legend-note">Reordenar, añadir o quitar nunca elige ubicación — eso lo sigue decidiendo el algoritmo de colocación. Quitar solo es posible antes de que arranque la obra, y devuelve el costo completo pagado.</p>
        ${
          colaGlobal.length
            ? `<table class="mini-table">
                <thead><tr><th>#</th><th>Edificio</th><th>Acciones</th></tr></thead>
                <tbody>
                  ${colaGlobal
                    .map(
                      (e, i) => `<tr>
                        <td>${i + 1}</td>
                        <td>${EDIFICIO_NOMBRE[e.tipo] ?? e.tipo}</td>
                        <td>
                          <button type="button" class="cola-mover-btn" data-settlement="${a.id}" data-edificio="${e.id}" data-direccion="arriba" ${i === 0 ? 'disabled' : ''}>&#9650;</button>
                          <button type="button" class="cola-mover-btn" data-settlement="${a.id}" data-edificio="${e.id}" data-direccion="abajo" ${i === colaGlobal.length - 1 ? 'disabled' : ''}>&#9660;</button>
                          <button type="button" class="cola-quitar-btn" data-settlement="${a.id}" data-edificio="${e.id}">Quitar</button>
                        </td>
                      </tr>`
                    )
                    .join('')}
                </tbody>
              </table>`
            : '<p class="legend-note">Cola vacía.</p>'
        }
        <div class="kv-row" style="margin-top:8px; gap:6px; align-items:center;">
          <select id="cola-cargo-select">
            <option value="gobernador">Gobernador</option>
            <option value="maestroObras">Maestro de Obras</option>
          </select>
          <select id="cola-tipo-select">
            ${CATALOGOS.catalogoEdificios.map((e) => `<option value="${e.tipo}">${EDIFICIO_NOMBRE[e.tipo] ?? e.tipo}</option>`).join('')}
          </select>
          <button type="button" id="cola-add-btn" data-settlement="${a.id}">Añadir a la cola</button>
        </div>
        <div class="tropa-info" id="cola-tipo-info"></div>
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
      (a) =>
        `<button type="button" class="settlement-tab-btn${a.id === asentamientoSeleccionadoId ? ' active' : ''}" data-settlement="${a.id}">${a.id}</button>`
    )
    .join('');
  const gruposHtml = `<div class="faccion-group">
    <div class="faccion-group-header"><span class="swatch" style="background:${faccionColor(faccionFiltroId, state.facciones)}"></span>${faccionSelectHtml}</div>
    <div class="settlement-tab-row">${botones}</div>
  </div>`;

  const seleccionado = state.asentamientos.find((a) => a.id === asentamientoSeleccionadoId)!;
  cont.innerHTML = `${gruposHtml}${renderDetalleAsentamiento(seleccionado, state)}`;

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

  cont.querySelectorAll('.auto-construccion-toggle-btn').forEach((btn) => {
    btn.addEventListener('click', () => {
      const id = (btn as HTMLElement).dataset.settlement!;
      const asentamiento = state.asentamientos.find((a) => a.id === id)!;
      if (asentamiento.autoConstruccionPausada) gameStore.reanudarAutoConstruccion(id);
      else gameStore.pausarAutoConstruccion(id);
    });
  });

  // Control manual de cola (Doc 4.2, a petición del usuario): el mismo selector de cargo gobierna quién
  // "actúa" para mover/quitar/añadir en este panel — Gobernador o Maestro de Obras, ver Doc 2.2/4.2.
  const colaCargoSelect = document.getElementById('cola-cargo-select') as HTMLSelectElement | null;
  const cargoSeleccionado = (): 'gobernador' | 'maestroObras' => (colaCargoSelect?.value as 'gobernador' | 'maestroObras') ?? 'gobernador';

  cont.querySelectorAll('.cola-mover-btn').forEach((btn) => {
    btn.addEventListener('click', () => {
      const el = btn as HTMLButtonElement;
      gameStore.moverEnCola(el.dataset.settlement!, cargoSeleccionado(), el.dataset.edificio!, el.dataset.direccion as 'arriba' | 'abajo');
    });
  });

  cont.querySelectorAll('.cola-quitar-btn').forEach((btn) => {
    btn.addEventListener('click', () => {
      const el = btn as HTMLButtonElement;
      gameStore.quitarDeCola(el.dataset.settlement!, cargoSeleccionado(), el.dataset.edificio!);
    });
  });

  const colaAddBtn = document.getElementById('cola-add-btn') as HTMLButtonElement | null;
  const colaTipoSelect = document.getElementById('cola-tipo-select') as HTMLSelectElement | null;
  colaAddBtn?.addEventListener('click', () => {
    gameStore.anadirEdificioManualmente(colaAddBtn.dataset.settlement!, cargoSeleccionado(), colaTipoSelect!.value as EdificioTipo);
  });
  colaTipoSelect?.addEventListener('change', actualizarInfoEdificioCola);
  actualizarInfoEdificioCola();
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
              `<div>${e.id} — ${e.nombre} (${nivelTropaTxt(e.tropaId)}, ${e.origen}) · cantidad ${e.cantidad} · veterania ${e.veterania.toFixed(1)} · moral ${e.moral.toFixed(0)}${e.heridoHastaTick ? ` · herido hasta t${e.heridoHastaTick}` : ''}</div>`
          )
          .join('') || '<div>Sin escuadrones.</div>';
      return `<div><strong>${etiquetaAsentamiento(a, state.facciones)}</strong> — Fundición: ${tieneFundicion ? 'sí' : 'no'} · Gran Fundición: ${tieneGranFundicion ? 'sí' : 'no'}${escuadronesHtml}</div>`;
    })
    .join('');

  // Campamentos de bandidos (Doc 1.9): amenaza global del mundo, no de un asentamiento — se muestra aparte.
  const campamentosHtml = state.campamentosBandidos
    .map((c) => `<div>${c.id} — (${Math.round(c.posicion.x)}, ${Math.round(c.posicion.y)}) · poder ${c.poder}</div>`)
    .join('');
  militarPanelEl.innerHTML += `<div class="detail-section"><h3>Campamentos de bandidos (Doc 1.9)</h3>${campamentosHtml || '<div>Ninguno activo.</div>'}</div>`;
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
      <div class="legend-row"><span class="swatch" style="background:#f1e6c8"></span>Caravana en tránsito</div>
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
  logEl.innerHTML = state.log.map((e) => `<div>[t${e.tick}] ${e.mensaje}</div>`).join('');
}

function actualizarTabs(): void {
  document.querySelectorAll<HTMLButtonElement>('#main-tabs .tab-btn').forEach((btn) => {
    btn.classList.toggle('active', btn.dataset.tab === tabActivo);
  });
  document.getElementById('tab-acciones')!.hidden = tabActivo !== 'acciones';
  document.getElementById('tab-guerra')!.hidden = tabActivo !== 'guerra';
  if (tabActivo === 'guerra') renderRosterTropas();
  document.getElementById('tab-comercio')!.hidden = tabActivo !== 'comercio';
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
  tabActivo = btn.dataset.tab as 'acciones' | 'guerra' | 'comercio' | 'asentamientos' | 'jugadores' | 'politicas' | 'balance';
  actualizarTabs();
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

  const zonas = gameStore.getZonas(state.asentamientos);
  const drawState: DrawState = {
    mapa: gameStore.getMapa(state),
    asentamientos: state.asentamientos,
    zonas,
    facciones: state.facciones,
    caravanas: state.caravanas,
    caminos: state.caminos,
    campamentosBandidos: state.campamentosBandidos,
    chokepointsControl: gameStore.chokepointsControl(zonas),
  };
  draw(ctx, canvas, drawState, terrenoCacheParaFrame(drawState.mapa));
  if (mostrarFiltroFertilidad) drawFiltroFertilidad(ctx, canvas, gameStore.getMapa(state));
  renderViabilidadFundacion(viendoPasado);
  actualizarSelects(liveState);
  actualizarInfoFlota(state);
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

const RECURSO_NOMBRE_CORTO: Record<string, string> = {
  piedra: 'piedra', cobre: 'cobre', estano: 'estaño', oro: 'oro', livestock: 'ganado',
};

/**
 * Aviso previo de emplazamiento (no bloquea nada, ver `evaluarViabilidadFundacion`): dibuja el radio inicial
 * bajo el cursor y explica en texto si el sitio es sostenible. El criterio duro es la madera — sin bosque al
 * alcance el asentamiento casi siempre acaba en ruinas, y es con diferencia el mejor predictor de si llegará
 * a construir Leñera/Barracón y, con ello, a tener tropa.
 */
function renderViabilidadFundacion(viendoPasado: boolean): void {
  if (!hoverFundacion || viendoPasado) {
    fundacionViabilidadEl.textContent = viendoPasado
      ? 'Vuelve al presente para evaluar emplazamientos.'
      : 'Pasa el cursor por el mapa para evaluar un emplazamiento.';
    fundacionViabilidadEl.className = 'fundacion-viabilidad';
    return;
  }

  const v = gameStore.viabilidadFundacion(hoverFundacion);
  drawPreviewFundacion(ctx, canvas, gameStore.getMapa(), {
    posicion: hoverFundacion,
    radioInicial: v.radioInicial,
    fundable: v.fundable,
    bosqueAlcanzable: v.bosqueAlcanzable,
  });

  const recursos = v.recursosEnRadio
    .map((r) => `${r.nodos}× ${RECURSO_NOMBRE_CORTO[r.tipo] ?? r.tipo}`)
    .join(', ');

  if (!v.fundable) {
    fundacionViabilidadEl.textContent = !v.dentroDelMapa
      ? '✖ Fuera de los límites del mapa.'
      : !v.terrenoValido
        ? '✖ Terreno de cima: inhabitable, no se puede fundar aquí.'
        : '✖ Dentro de una zona de influencia existente.';
    fundacionViabilidadEl.className = 'fundacion-viabilidad no-fundable';
  } else if (!v.bosqueAlcanzable) {
    fundacionViabilidadEl.textContent = `⚠ Sin bosque al alcance: no podrá construir Leñera, y la madera paga Mantenimiento desde el primer tick. Se puede fundar igual, pero es el emplazamiento con más riesgo de acabar en ruinas.${recursos ? ` En el radio: ${recursos}.` : ''}`;
    fundacionViabilidadEl.className = 'fundacion-viabilidad aviso';
  } else {
    fundacionViabilidadEl.textContent = `✔ Emplazamiento sostenible: bosque al alcance.${recursos ? ` Además en el radio: ${recursos}.` : ' Sin nodos minerales en el radio inicial (la zona crece al construir).'}`;
    fundacionViabilidadEl.className = 'fundacion-viabilidad ok';
  }
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

/** Coordenadas de mundo bajo el puntero, a partir de un evento de ratón sobre el canvas. */
function posicionMundoDesdeEvento(ev: MouseEvent): { x: number; y: number } {
  const rect = canvas.getBoundingClientRect();
  const scale = gameStore.getMapa().limites.ancho / canvas.width;
  return { x: (ev.clientX - rect.left) * scale, y: (ev.clientY - rect.top) * scale };
}

canvas.addEventListener('mousemove', (ev) => {
  hoverFundacion = posicionMundoDesdeEvento(ev);
  render();
});

canvas.addEventListener('mouseleave', () => {
  hoverFundacion = null;
  render();
});

canvas.addEventListener('click', (ev) => {
  const state = gameStore.getState();
  const rect = canvas.getBoundingClientRect();
  const scale = gameStore.getMapa(state).limites.ancho / canvas.width;
  const worldX = (ev.clientX - rect.left) * scale;
  const worldY = (ev.clientY - rect.top) * scale;
  if (expansionModoClicCheckbox.checked) {
    expansionDestino = { x: worldX, y: worldY };
    expansionDestinoInput.value = `(${Math.round(worldX)}, ${Math.round(worldY)})`;
    return;
  }
  gameStore.fundarAsentamiento(faccionSelect.value, { x: worldX, y: worldY }, Number(jugadoresInput.value) || 1);
});

document.getElementById('expansion-lanzar-btn')!.addEventListener('click', () => {
  if (!expansionDestino) return;
  gameStore.lanzarCaravanaFundacion(expansionOrigenSelect.value, expansionDestino, Number(expansionJugadoresInput.value) || 1);
  expansionDestino = null;
  expansionDestinoInput.value = '';
  expansionModoClicCheckbox.checked = false;
});

document.getElementById('expansion-desarmar-btn')!.addEventListener('click', () => {
  gameStore.desarmarCaravanaFundacion(expansionCaravanaSelect.value);
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

document.getElementById('faccion-crear-btn')!.addEventListener('click', () => {
  gameStore.crearFaccion(faccionCrearNombreInput.value.trim());
  faccionCrearNombreInput.value = '';
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

document.getElementById('flota-construir-btn')!.addEventListener('click', () => {
  gameStore.crearCaravana(flotaAsentamientoSelect.value);
});

document.getElementById('reclutar-tropa-btn')!.addEventListener('click', () => {
  gameStore.reclutarTropa(guerraAsentamientoSelect.value, reclutarTropaSelect.value, reclutarTropaOrigenSelect.value as 'pesants' | 'artesanos');
});

document.getElementById('gran-fundicion-btn')!.addEventListener('click', () => {
  // Consolidada en el control manual de cola (Doc 4.2, a petición del usuario) — Gran Fundición ya no tiene
  // su propio camino especial, pasa por el mismo `anadirEdificioManualmente` que cualquier otro edificio.
  gameStore.anadirEdificioManualmente(guerraAsentamientoSelect.value, 'gobernador', 'granFundicion');
});

document.getElementById('asedio-btn')!.addEventListener('click', () => {
  gameStore.iniciarAsedio(combateAsentamientoSelect.value, guerraObjetivoSelect.value, idsDeChipsCombate());
});

document.getElementById('campo-abierto-btn')!.addEventListener('click', () => {
  gameStore.combateCampoAbierto(
    combateAsentamientoSelect.value,
    idsDeChipsCombate(),
    guerraObjetivoSelect.value,
    idsDeInput(guerraEscuadronesObjetivoInput)
  );
});

document.getElementById('interceptar-btn')!.addEventListener('click', () => {
  gameStore.interceptarCaravana(combateAsentamientoSelect.value, idsDeChipsCombate(), guerraCaravanaSelect.value);
});

document.getElementById('atacar-campamento-btn')!.addEventListener('click', () => {
  gameStore.atacarCampamentoBandidos(combateAsentamientoSelect.value, idsDeChipsCombate(), guerraCampamentoSelect.value);
});

document.getElementById('tick-btn')!.addEventListener('click', () => {
  gameStore.avanzarTick();
});

document.getElementById('regenerar-btn')!.addEventListener('click', () => {
  const region = regionSelect.value as RegionId | '';
  gameStore.regenerarMundo(Number(seedInput.value) || 0, region || undefined);
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
