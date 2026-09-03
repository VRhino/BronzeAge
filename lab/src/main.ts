// Laboratorio visual de crecimiento de asentamientos — ámbito aparte del juego (`laboratorio.html`), fuera
// de `main.ts`/`app/gameStore.ts`. Un solo asentamiento, motor real (`avanzarSimulacion`), sin partida ni
// facciones de más alrededor: existe para ver en vivo cómo se aplica el árbol único de anclas (Etapa 5,
// `engine/trazado.ts`) sin tener que perseguirlo en una partida completa.
import type { Asentamiento, EdificioTipo, Faccion, Point, RecursoTipo } from '../../src/domain/types';
import { MAPA_DEFAULT, createRng, generarMapa, type RandomFn } from '../../src/worldgen';
import { crearMapa, type Mapa } from '../../src/world/mapa';
import { crearFaccion } from '../../src/engine/faccion';
import { asignarCargoLocal } from '../../src/engine/cargos';
import { evaluarViabilidadFundacion, fundarAsentamiento as fundarAsentamientoEngine } from '../../src/engine/settlement';
import { avanzarSimulacion, type EstadoSimulacion } from '../../src/engine/simulation';
import { progresoNivelAsentamiento } from '../../src/engine/asentamientoQuery';
import { celdasDeEdificio, redDeCalles, resolverPerfil, trazadoParaAsentamiento, type TrazadoAsentamiento } from '../../src/engine/trazado';
import { abandonarRecinto, comprometerRecinto, RecintoInvalidoError, trazadoDeRecinto, trazarRecinto, type TrazoRecinto } from '../../src/engine/muralla';
import type { TrazadoMuralla } from '../../src/engine/trazado';
import { anadirEdificioManualmente, mejorarEdificioManualmente, reclamosDeFuentes, ConstruccionManualInvalidaError } from '../../src/engine/construction';
import {
  REJILLA_ASENTAMIENTO,
  TRAZADO,
  EDIFICIO_TAMANO,
  NIVEL_FACCION,
  PERFILES_TRAZADO,
  type PerfilTrazado,
} from '../../src/constants';
import { instanteDeTick, isoDeInstante } from '../../src/session/estado';
import { dibujarAsentamientoLab, EDIFICIO_ETIQUETA, proyeccion, type EstadoDibujoLab } from './render';
import { crearTooltip } from './tooltip';
import { montarPanelParametros, renderPanelEdificios, renderPanelMejoras } from './panels';
import { anclaEnPosicion, dibujarOverlayAnclas, inspeccionarAnclas, type FilaAncla } from './debugAnclas';
import { calcularLayoutArbol, dibujarArbol, nodoEnPosicion, type LayoutArbol } from './vistaArbol';

/** Todos los tipos de recurso del juego (mismo listado que `almacenInicial` en `engine/settlement.ts`) —
 * el laboratorio le da 9999 de cada uno a su único asentamiento apenas se funda, para poder probar
 * crecimiento sin que la escasez de materiales sea una variable más a controlar. */
const RECURSOS_LAB: RecursoTipo[] = [
  'madera', 'piedra', 'trigo', 'cobre', 'estano', 'oro', 'livestock',
  'lingoteCobre', 'lingoteEstano', 'lingoteBronce',
  'cuero', 'cueroCurtido', 'cueroCalidad',
  'armaMadera', 'armaCobre', 'armaBronce', 'armaBronceCalidad',
  'armaduraBasica', 'armaduraIntermedia', 'armaduraBronce',
];

/** Tipos construibles manualmente en la pestaña "Construcción manual" — mismo criterio que
 * `anadirEdificioManualmente` (engine/construction.ts): excluye Centro Urbano y Puesto de Mercado (ambos
 * rechazados ahí explícitamente) y los "marcadores gratis" que solo nacen por la regla de semilla de grupo
 * (plaza/plazaDeArmas/patioDeGremios/tallerCarpinteria/pozo/parque, Etapa 5) — esos no tienen sitio propio
 * fuera del árbol de anclas y `sitioParaTipo` no sabe colocarlos sueltos. */
const TIPOS_CONSTRUIBLES_MANUAL: EdificioTipo[] = [
  'vivienda', 'granja', 'cantera', 'lenera', 'almacen', 'mina', 'minaCobre', 'minaEstano',
  'fundicion', 'granFundicion', 'corral', 'armeria', 'curtiduria', 'carpinteria', 'palacio',
  'barracon', 'galeriaDeTiro', 'mercado', 'maravilla',
];

/** Minutos de mundo que le quedan a una obra. Desde la Fase D3 el plazo es una FECHA absoluta
 * (`Edificio.completaEn`), no un contador que decrementa: se resta contra el instante del tick actual. */
function minutosRestantes(completaEn: number, tickActual: number): string {
  const restante = (completaEn - instanteDeTick(tickActual)) / 60_000;
  return restante <= 0 ? '0' : restante.toFixed(0);
}

/** Qué le falta al asentamiento para subir de nivel, para la línea de estado — mismos gates que
 * `calcularNivelAsentamiento` (Doc 4.5): población + "N de M" edificios activos. El cupo de nivel de la
 * partida real no aplica aquí (el laboratorio tiene un solo asentamiento). */
function faltaParaNivel(asentamiento: Asentamiento): string {
  const progreso = progresoNivelAsentamiento(asentamiento);
  if (progreso.esMaximo || !progreso.siguiente) return 'nivel máximo';
  const s = progreso.siguiente;
  const gate = (etiqueta: string, actual: number, requerido: number): string =>
    `${etiqueta} ${actual}/${requerido}${actual >= requerido ? ' ✓' : ''}`;

  const partes: string[] = [gate('pesants', s.pesants.actual, s.pesants.requerido)];
  if (s.artesanos.requerido > 0) partes.push(gate('artesanos', s.artesanos.actual, s.artesanos.requerido));

  const edificiosOk = s.edificiosConstruidos >= s.edificiosRequeridos;
  let edificios = `edificios ${s.edificiosConstruidos}/${s.edificiosRequeridos}${edificiosOk ? ' ✓' : ''}`;
  if (!edificiosOk && s.edificiosFaltantes.length > 0) {
    const faltan = s.edificiosRequeridos - s.edificiosConstruidos;
    const lista = s.edificiosFaltantes.map((tipo) => EDIFICIO_ETIQUETA[tipo as EdificioTipo] ?? tipo).join(', ');
    // Gate "N de M" (nivel 2): la lista son CANDIDATOS, basta construir `faltan` de ellos.
    edificios += s.edificiosFaltantes.length > faltan ? ` (${faltan} más de: ${lista})` : ` (faltan: ${lista})`;
  }
  partes.push(edificios);
  return `para nivel ${s.nivelObjetivo} — ${partes.join(' · ')}`;
}

function darMaterialesInfinitos(asentamiento: Asentamiento): Asentamiento {
  const almacen = { ...asentamiento.almacen };
  for (const tipo of RECURSOS_LAB) {
    almacen[tipo] = { cantidad: 9999, capacidad: 9999 };
  }
  return { ...asentamiento, almacen };
}

const canvas = document.getElementById('lab-canvas') as HTMLCanvasElement;
const ctx = canvas.getContext('2d')!;
const treeCanvas = document.getElementById('lab-tree-canvas') as HTMLCanvasElement;
const treeCtx = treeCanvas.getContext('2d')!;
const seedInput = document.getElementById('lab-seed') as HTMLInputElement;
const fundarBtn = document.getElementById('lab-fundar') as HTMLButtonElement;
const tick1Btn = document.getElementById('lab-tick-1') as HTMLButtonElement;
const tick10Btn = document.getElementById('lab-tick-10') as HTMLButtonElement;
const tick50Btn = document.getElementById('lab-tick-50') as HTMLButtonElement;
const autoBtn = document.getElementById('lab-auto') as HTMLButtonElement;
const rejillaChk = document.getElementById('lab-rejilla') as HTMLInputElement | null;
const murallaTrazarBtn = document.getElementById('lab-muralla-trazar') as HTMLButtonElement;
const murallaNivelSel = document.getElementById('lab-muralla-nivel') as HTMLSelectElement;
const murallaStatusEl = document.getElementById('lab-muralla-status')!;
const murallaComprometerBtn = document.getElementById('lab-muralla-comprometer') as HTMLButtonElement;
const murallaQuitarBtn = document.getElementById('lab-muralla-quitar') as HTMLButtonElement;
const murallaAbandonarBtn = document.getElementById('lab-muralla-abandonar') as HTMLButtonElement;
const perfilOpcionesEl = document.getElementById('lab-perfil-opciones')!;
const statusEl = document.getElementById('lab-status')!;
const anclasBodyEl = document.getElementById('lab-anclas-body')!;
const tabAnclasBtn = document.getElementById('lab-tab-anclas') as HTMLButtonElement;
const tabManualBtn = document.getElementById('lab-tab-manual') as HTMLButtonElement;
const panelAnclasEl = document.getElementById('lab-panel-anclas')!;
const panelManualEl = document.getElementById('lab-panel-manual')!;
const manualTipoSelect = document.getElementById('lab-manual-tipo') as HTMLSelectElement;
const manualEncolarBtn = document.getElementById('lab-manual-encolar') as HTMLButtonElement;
const manualStatusEl = document.getElementById('lab-manual-status')!;
const colaBodyEl = document.getElementById('lab-cola-body')!;
const mejorasBodyEl = document.getElementById('lab-mejoras-body')!;
const tabEdificiosBtn = document.getElementById('lab-tab-edificios') as HTMLButtonElement;
const tabParametrosBtn = document.getElementById('lab-tab-parametros') as HTMLButtonElement;
const panelEdificiosEl = document.getElementById('lab-panel-edificios')!;
const panelParametrosEl = document.getElementById('lab-panel-parametros')!;
const edificiosResumenEl = document.getElementById('lab-edificios-resumen')!;
const edificiosBodyEl = document.getElementById('lab-edificios-body')!;

let mapa: Mapa;
let estado: EstadoSimulacion;
let rng: RandomFn;
let faccionLab: Faccion;
let contadorManual = 0;
let tick = 0;
let nacimientos = new Map<string, number>();
let autoTimer: number | undefined;
// Presupuesto de muralla: el anillo PROPUESTO. No es estado del motor y no se persiste — se recalcula a mano
// con el botón, nunca por tick, porque el trazo es una consulta cara y el jugador tampoco lo vería cambiar.
let murallaPropuesta: TrazadoMuralla | undefined;
let murallaTrazo: TrazoRecinto | undefined;

for (const tipo of TIPOS_CONSTRUIBLES_MANUAL) {
  const opcion = document.createElement('option');
  opcion.value = tipo;
  opcion.textContent = EDIFICIO_ETIQUETA[tipo] ?? tipo;
  manualTipoSelect.appendChild(opcion);
}

// --- Estado cacheado del último `computar()` (motor), reutilizado por `pintar()` (solo hover, sin volver a
// tocar el motor ni recalcular trazado/árbol en cada movimiento del mouse). ---
let cacheAsentamiento: Asentamiento | null = null;
let cacheFilas: FilaAncla[] = [];
let cacheTrazado: TrazadoAsentamiento = { calles: [], caminos: [], huellas: {}, murallas: [] };
let cacheAPantalla: (p: Point) => Point = (p) => p;
let cacheEscala = 1;
let cacheLayoutArbol: LayoutArbol = { nodos: new Map(), ancho: 0, alto: 0 };
let hoveredId: string | null = null;
let cacheDibujo: EstadoDibujoLab | null = null;

/** Barre una grilla regular buscando una posición fundable con bosque alcanzable (mismo criterio que
 * `engine/__tests__/fixtures.ts::posicionRecomendable`, reimplementado aquí en vez de importar desde
 * `__tests__` — el laboratorio es una herramienta de desarrollo, no un test). */
function posicionRecomendable(mapaBase: Mapa, existentes: Asentamiento[]): Point {
  const paso = 40;
  for (let x = paso; x < mapaBase.limites.ancho; x += paso) {
    for (let y = paso; y < mapaBase.limites.alto; y += paso) {
      const posicion = { x, y };
      if (evaluarViabilidadFundacion(mapaBase, posicion, existentes).recomendable) return posicion;
    }
  }
  return { x: mapaBase.limites.ancho / 2, y: mapaBase.limites.alto / 2 };
}

function fundar(seed: number): void {
  if (autoTimer !== undefined) detenerAuto();
  mapa = crearMapa(generarMapa({ ...MAPA_DEFAULT, seed }));
  rng = createRng(seed);
  // Facción a nivel máximo. Sin esto, `CUPO_NIVEL_ASENTAMIENTO.maxNivel3` vale 0 para una Facción de nivel 1
  // y `avanzarNivelAsentamiento` (engine/mantenimiento.ts) nunca promueve el único asentamiento del lab de
  // nivel 2 a 3 aunque cumpla todos los gates — se queda "elegible, esperando cupo" para siempre. Ese cupo es
  // una restricción de la PARTIDA (cuántas ciudades grandes puede sostener una Facción), no del crecimiento
  // urbano que el lab estudia. `experiencia` alta y no solo `nivel`: `calcularNivelFaccion` lo recalcula
  // desde la experiencia en cada `avanzarSimulacion`, así que un `nivel` suelto volvería a 1 al primer tick.
  const faccionBase: Faccion = {
    ...crearFaccion('faccion-lab', 'Laboratorio'),
    nivel: NIVEL_FACCION.nivelMaximo,
    experiencia: (NIVEL_FACCION.xpParaNivel[NIVEL_FACCION.xpParaNivel.length - 1] ?? 0) + 1,
  };
  const posicion = posicionRecomendable(mapa, []);
  const { asentamiento, facciones } = fundarAsentamientoEngine(mapa, [faccionBase], faccionBase.id, posicion, ['jugador-lab'], [], instanteDeTick(0));
  faccionLab = facciones[0]!;
  // Gobernador propio (requisito de `anadirEdificioManualmente`) + 9999 de cada material, para que la
  // pestaña "Construcción manual" pueda encolar cualquier cosa sin que fondos o cargos sean la traba.
  const asentamientoConGobernador = asignarCargoLocal(darMaterialesInfinitos(asentamiento), faccionLab, 'gobernador', 'jugador-lab');
  estado = {
    asentamientos: [asentamientoConGobernador],
    facciones: [faccionLab],
    caravanas: [],
    ejercitos: [],
    acuerdos: [],
    ordenes: [],
    relaciones: [],
    titulos: [],
    caminos: [],
    campamentosBandidos: [],
    bandidosProximoSpawnEn: instanteDeTick(0),
  };
  tick = 0;
  contadorManual = 0;
  murallaPropuesta = undefined;
  murallaTrazo = undefined;
  murallaStatusEl.textContent = '—';
  nacimientos = new Map(asentamientoConGobernador.edificios.map((e) => [e.id, 0]));
  hoveredId = null;
  manualStatusEl.textContent = '—';
  computar();
}

function paso(): void {
  if (estado.asentamientos.length === 0) return;
  tick += 1;
  const instante = instanteDeTick(tick);
  estado = avanzarSimulacion(estado, mapa, { instante, momento: isoDeInstante(instante), rng });
  const asentamiento = estado.asentamientos[0];
  if (!asentamiento) return;
  for (const edificio of asentamiento.edificios) {
    if (!nacimientos.has(edificio.id)) nacimientos.set(edificio.id, tick);
  }
}

function avanzarNTicks(n: number): void {
  for (let i = 0; i < n; i++) {
    if (estado.asentamientos.length === 0) break;
    paso();
  }
  computar();
}

function toggleAuto(): void {
  if (autoTimer !== undefined) {
    detenerAuto();
    return;
  }
  autoBtn.textContent = '⏸ Auto';
  autoBtn.classList.add('activo');
  autoTimer = window.setInterval(() => {
    if (estado.asentamientos.length === 0) {
      detenerAuto();
      return;
    }
    paso();
    computar();
  }, 150);
}

function detenerAuto(): void {
  if (autoTimer !== undefined) window.clearInterval(autoTimer);
  autoTimer = undefined;
  autoBtn.textContent = '▶ Auto';
  autoBtn.classList.remove('activo');
}

function filaClase(fila: FilaAncla): string {
  if (fila.esSemillaActiva) return 'activa';
  if (fila.huerfana) return 'huerfana';
  return '';
}

function marca(valor: boolean): string {
  return valor ? '✓' : '—';
}

/** Publica el estado REAL del motor en `window.__lab` (ver README del laboratorio): sirve para medir
 * invariantes sobre la ciudad que estés mirando sin instrumentar nada. Se llama desde `computar()` y también
 * al trazar una muralla — si no, `murallaTrazo` se quedaría con el valor del último `computar()` y la consola
 * mentiría, que es peor que no exponer nada. */
function publicarEnConsola(): void {
  const asentamiento = cacheAsentamiento;
  if (!asentamiento) return;
  (window as unknown as { __lab: unknown }).__lab = {
    tick,
    estado,
    asentamiento,
    trazado: cacheTrazado,
    mapa,
    red: redDeCalles(asentamiento.id, asentamiento.edificios),
    celdasDeEdificio,
    TRAZADO,
    EDIFICIO_TAMANO,
    murallaTrazo,
    trazarRecinto,
  };
}

/** Recalcula todo lo que depende del motor (trazado, árbol, filas) tras fundar o avanzar ticks, guarda el
 * resultado en la caché y pinta. `pintar()` (hover) NUNCA llama a esto — solo redibuja con lo ya cacheado. */
function computar(): void {
  const asentamiento = estado.asentamientos[0] ?? null;
  cacheAsentamiento = asentamiento;
  if (!asentamiento) {
    cacheFilas = [];
    pintar();
    statusEl.textContent = `Tick ${tick} — el asentamiento colapsó. Fundá de nuevo.`;
    anclasBodyEl.innerHTML = '';
    return;
  }

  // El trazado llega YA RESUELTO del motor — el mismo `trazadoParaAsentamiento` que sirve el backend por HTTP.
  // Antes el laboratorio lo recalculaba a mano (redDeCalles + segmentosDeRed + huellas), lo que significaba
  // que podia divergir de lo que ve un cliente real. Ahora dibujar el laboratorio EJERCITA ese contrato.
  cacheTrazado = trazadoParaAsentamiento(asentamiento);
  refrescarEstadoDeMuralla(asentamiento);

  // Gancho de depuración: el estado REAL del motor accesible desde la consola del navegador
  // (`__lab.asentamiento`, `__lab.trazado`, `__lab.estado`). El laboratorio existe para mirar; poder además
  // consultar y medir sin instrumentar nada es la mitad de su valor.
  publicarEnConsola();

  const proy = proyeccion(canvas, REJILLA_ASENTAMIENTO.radioMapa);
  cacheEscala = proy.escala;
  cacheAPantalla = proy.aPantalla;

  cacheFilas = inspeccionarAnclas(asentamiento.edificios, asentamiento.id, tick, nacimientos);
  cacheLayoutArbol = calcularLayoutArbol(cacheFilas);

  const perfilActivo = resolverPerfil(asentamiento.id);
  statusEl.textContent =
    `Tick ${tick} · nivel ${asentamiento.nivel} · ${asentamiento.edificios.length} edificios · ` +
    `${cacheFilas.length} anclas de árbol · perfil ${perfilActivo}${TRAZADO.perfilForzado ? '' : ' (tradición)'} · ` +
    faltaParaNivel(asentamiento);
  anclasBodyEl.innerHTML = cacheFilas
    .map(
      (fila) =>
        `<tr class="${filaClase(fila)}" data-id="${fila.id}">` +
        `<td>${EDIFICIO_ETIQUETA[fila.tipo] ?? fila.tipo}</td>` +
        `<td>${fila.codigo}</td>` +
        `<td>${fila.nivel}</td>` +
        `<td>${fila.distanciaPadreCeldas === null ? '—' : fila.distanciaPadreCeldas.toFixed(1)}</td>` +
        `<td>${marca(fila.esSemillaActiva)}</td>` +
        `<td>${marca(fila.semillaSaturada)}</td>` +
        `<td>${marca(fila.anclaLlena)}</td>` +
        `<td>${fila.huerfana ? '⚠' : '—'}</td>` +
        `<td>${fila.edad}</td>` +
        `</tr>`
    )
    .join('');

  renderPanelEdificios(edificiosBodyEl, edificiosResumenEl, asentamiento);
  renderPanelMejoras(mejorasBodyEl, asentamiento, forzarMejora);

  const enColaOConstruccion = asentamiento.edificios
    .filter((e) => e.estado === 'en_cola' || e.estado === 'en_construccion')
    .sort((a, b) => (b.prioridad ?? 0) - (a.prioridad ?? 0));
  colaBodyEl.innerHTML = enColaOConstruccion
    .map(
      (e) =>
        `<tr><td>${EDIFICIO_ETIQUETA[e.tipo] ?? e.tipo}</td><td>${e.estado}</td><td>${e.completaEn === undefined ? '—' : minutosRestantes(e.completaEn, tick)}</td></tr>`
    )
    .join('');

  pintar();
}

/** Solo redibuja con lo ya cacheado por `computar()`, aplicando el resaltado de `hoveredId` — es lo único que
 * corre en cada movimiento del mouse, para que el resaltado cruzado mapa↔árbol no tenga que volver a tocar el
 * motor ni recalcular el trazado en cada frame. */
function pintar(): void {
  if (!cacheAsentamiento) {
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    treeCtx.clearRect(0, 0, treeCanvas.width, treeCanvas.height);
    return;
  }
  cacheDibujo = {
    asentamiento: cacheAsentamiento,
    trazado: cacheTrazado,
    tamanoCelda: REJILLA_ASENTAMIENTO.tamanoCelda,
    radioMapa: REJILLA_ASENTAMIENTO.radioMapa,
    mostrarRejilla: rejillaChk?.checked ?? true,
    murallaPropuesta,
    murallaTrazo,
  };
  dibujarAsentamientoLab(ctx, canvas, cacheDibujo);
  dibujarOverlayAnclas(ctx, cacheFilas, cacheAsentamiento.id, cacheAPantalla, cacheEscala, hoveredId);

  treeCanvas.width = Math.max(treeCanvas.parentElement!.clientWidth, cacheLayoutArbol.ancho);
  treeCanvas.height = Math.max(300, cacheLayoutArbol.alto);
  dibujarArbol(treeCtx, treeCanvas, cacheFilas, cacheLayoutArbol, hoveredId);

  for (const fila of Array.from(anclasBodyEl.children)) {
    fila.classList.toggle('hover', (fila as HTMLElement).dataset.id === hoveredId);
  }
}

function forzarMejora(edificioId: string): void {
  const asentamiento = estado.asentamientos[0];
  if (!asentamiento) return;
  try {
    const actualizado = mejorarEdificioManualmente(asentamiento, 'gobernador', edificioId, undefined);
    estado = { ...estado, asentamientos: [actualizado] };
    for (const e of actualizado.edificios) if (!nacimientos.has(e.id)) nacimientos.set(e.id, tick);
    manualStatusEl.textContent = 'Mejora forzada.';
    computar();
  } catch (err) {
    manualStatusEl.textContent = err instanceof ConstruccionManualInvalidaError ? err.message : String(err);
  }
}

function fijarHover(id: string | null): void {
  if (id === hoveredId) return;
  hoveredId = id;
  pintar();
}

canvas.addEventListener('mousemove', (ev) => {
  const rect = canvas.getBoundingClientRect();
  const x = (ev.clientX - rect.left) * (canvas.width / rect.width);
  const y = (ev.clientY - rect.top) * (canvas.height / rect.height);
  const sobreEdificio = tooltip.actualizar(ev);
  fijarHover(sobreEdificio ?? anclaEnPosicion(cacheFilas, cacheAPantalla, x, y));
});
canvas.addEventListener('mouseleave', () => {
  tooltip.ocultar();
  fijarHover(null);
});

treeCanvas.addEventListener('mousemove', (ev) => {
  const rect = treeCanvas.getBoundingClientRect();
  const x = (ev.clientX - rect.left) * (treeCanvas.width / rect.width);
  const y = (ev.clientY - rect.top) * (treeCanvas.height / rect.height);
  fijarHover(nodoEnPosicion(cacheLayoutArbol, x, y));
});
treeCanvas.addEventListener('mouseleave', () => fijarHover(null));

anclasBodyEl.addEventListener('mouseover', (ev) => {
  const fila = (ev.target as HTMLElement).closest('tr[data-id]') as HTMLElement | null;
  fijarHover(fila?.dataset.id ?? null);
});
anclasBodyEl.addEventListener('mouseleave', () => fijarHover(null));

type Tab = 'anclas' | 'edificios' | 'manual' | 'parametros';
function elegirTab(tab: Tab): void {
  const mapa: Record<Tab, [HTMLButtonElement, HTMLElement]> = {
    anclas: [tabAnclasBtn, panelAnclasEl],
    edificios: [tabEdificiosBtn, panelEdificiosEl],
    manual: [tabManualBtn, panelManualEl],
    parametros: [tabParametrosBtn, panelParametrosEl],
  };
  for (const [nombre, [btn, panel]] of Object.entries(mapa) as [Tab, [HTMLButtonElement, HTMLElement]][]) {
    btn.classList.toggle('activo', nombre === tab);
    panel.style.display = nombre === tab ? '' : 'none';
  }
}
tabAnclasBtn.addEventListener('click', () => elegirTab('anclas'));
tabEdificiosBtn.addEventListener('click', () => elegirTab('edificios'));
tabManualBtn.addEventListener('click', () => elegirTab('manual'));
tabParametrosBtn.addEventListener('click', () => elegirTab('parametros'));

const tooltip = crearTooltip(
  canvas,
  () => cacheDibujo,
  (id) => cacheFilas.find((f) => f.id === id)?.codigo ?? null
);

/**
 * Selector EXCLUYENTE de perfil de trazado (doc trazado §E6.23). Escribe `TRAZADO.perfilForzado`, que es lo
 * que lee `resolverPerfil` en el motor — el mismo camino que usará una política real.
 *
 * Se aplica EN VIVO a propósito, sin refundar: nada mueve lo ya construido, así que cambiar de perfil a mitad
 * de partida deja un estrato visible. Es exactamente lo que va a pasar cuando una política de 150 ticks
 * expire, y conviene poder verlo.
 */
const ETIQUETA_PERFIL: Record<PerfilTrazado, string> = {
  nucleos: 'Núcleos',
  caminera: 'Caminera',
  compacta: 'Compacta',
  gremial: 'Gremial',
};
const AYUDA_PERFIL: Record<PerfilTrazado, string> = {
  nucleos: 'Manda el hueco pegado al ancla. Racimos densos concéntricos — el comportamiento histórico.',
  caminera: 'Manda el frente de calle: se prefiere continuar una hilera existente antes que pegarse al ancla.',
  compacta: 'Manda la cercanía al centro de la ciudad. Cada barrio llena primero su cara interior; ciudad más apretada.',
  gremial: 'Manda el lado compartido con los AFINES. Barrios monocromos, oficios segregados.',
};

function montarSelectorPerfil(): void {
  const opciones: { valor: PerfilTrazado | null; texto: string; ayuda: string }[] = [
    { valor: null, texto: 'Tradición', ayuda: 'Sin forzar: cada asentamiento usa el perfil que le toca por su id (`perfilPorTradicion`).' },
    ...PERFILES_TRAZADO.map((p) => ({ valor: p, texto: ETIQUETA_PERFIL[p], ayuda: AYUDA_PERFIL[p] })),
  ];

  for (const { valor, texto, ayuda } of opciones) {
    const label = document.createElement('label');
    label.title = ayuda;
    const radio = document.createElement('input');
    radio.type = 'radio';
    radio.name = 'lab-perfil';
    radio.checked = TRAZADO.perfilForzado === valor;
    label.classList.toggle('activo', radio.checked);
    radio.addEventListener('change', () => {
      if (!radio.checked) return;
      TRAZADO.perfilForzado = valor;
      perfilOpcionesEl.querySelectorAll('label').forEach((otra) => otra.classList.remove('activo'));
      label.classList.add('activo');
      computar(); // solo refresca la línea de estado: el perfil muerde en la SIGUIENTE colocación.
    });
    label.append(radio, document.createTextNode(` ${texto}`));
    perfilOpcionesEl.appendChild(label);
  }
}
montarSelectorPerfil();

montarPanelParametros(panelParametrosEl, () => {
  fundar(Number(seedInput.value) || 1);
  elegirTab('parametros');
});

manualEncolarBtn.addEventListener('click', () => {
  const asentamiento = estado.asentamientos[0];
  if (!asentamiento) return;
  const tipo = manualTipoSelect.value as EdificioTipo;
  const reclamos = reclamosDeFuentes(estado.asentamientos);
  try {
    const actualizado = anadirEdificioManualmente(
      asentamiento,
      faccionLab,
      'gobernador',
      tipo,
      [],
      mapa,
      undefined,
      reclamos,
      contadorManual++
    );
    estado = { ...estado, asentamientos: [actualizado] };
    for (const edificio of actualizado.edificios) {
      if (!nacimientos.has(edificio.id)) nacimientos.set(edificio.id, tick);
    }
    manualStatusEl.textContent = `${EDIFICIO_ETIQUETA[tipo] ?? tipo} encolado.`;
    computar();
  } catch (err) {
    manualStatusEl.textContent = err instanceof ConstruccionManualInvalidaError ? err.message : String(err);
  }
});

/**
 * Presupuesto de muralla (§14 del doc): enseña dónde caería el anillo HOY y qué costaría, sin comprometer
 * nada. Es la mitad barata de la mecánica y la que responde su pregunta más peligrosa — si el trazo se
 * rechaza en ciudades de forma rara, se descubre aquí sin haber tocado ni una invariante persistida.
 */
function trazarMuralla(): void {
  if (!cacheAsentamiento) return;
  const nivel = Number(murallaNivelSel.value) || 1;
  const trazo = trazarRecinto(cacheAsentamiento, { nivel });
  if (!trazo) {
    murallaPropuesta = undefined;
    murallaTrazo = undefined;
    murallaStatusEl.textContent = 'Sin trazo válido: el anillo no se puede cerrar en esta ciudad.';
    publicarEnConsola();
    pintar();
    return;
  }
  murallaPropuesta = trazadoDeRecinto(trazo, nivel);
  murallaTrazo = trazo;
  murallaStatusEl.textContent = resumenDeTrazo(trazo, nivel);
  publicarEnConsola();
  pintar();
}

function resumenDeTrazo(trazo: TrazoRecinto, nivel: number): string {
  const coste = Object.entries(trazo.costo)
    .map(([recurso, cantidad]) => `${cantidad} ${recurso}`)
    .join(' + ');
  const partes = [
    `nivel ${nivel} · ${trazo.celdas.length} celdas`,
    `${trazo.puertas} puerta${trazo.puertas === 1 ? '' : 's'}`,
    `${trazo.torres} torres`,
    `${trazo.areaEncerrada} celdas encerradas`,
    `dentro ${trazo.dentro.length} / arrabal ${trazo.fuera.length}`,
    coste || 'gratis',
  ];
  if (trazo.afuerasDentro.length > 0) partes.push(`⚠ ${trazo.afuerasDentro.length} de afueras ENCERRADAS`);
  return partes.join(' · ');
}

/**
 * Comprometer: el trazo deja de ser una consulta y pasa a ser estado del asentamiento. A partir de aquí sus
 * celdas OCUPAN SUELO —`sueloOcupado` las cuenta— así que ningún edificio posterior puede plantarse encima,
 * que es justo el fallo que se veía antes de que existiera la entidad.
 */
function comprometerMuralla(): void {
  const actual = estado.asentamientos[0];
  if (!actual) return;
  try {
    const nivel = Number(murallaNivelSel.value) || 1;
    estado = { ...estado, asentamientos: [comprometerRecinto(actual, nivel, instanteDeTick(tick))] };
    murallaPropuesta = undefined;
    murallaTrazo = undefined;
    computar();
    const recinto = estado.asentamientos[0]!.recintos!.at(-1)!;
    const puertas = recinto.celdas.filter((c) => c.clase === 'puerta').length;
    murallaStatusEl.textContent =
      `Recinto comprometido (GRATIS): ${recinto.celdas.length} celdas · ${puertas} puertas · nivel ${recinto.nivel}. ` +
      `Su suelo ya está ocupado; la obra empieza a levantarlo y a cobrarlo con los ticks.`;
  } catch (err) {
    murallaStatusEl.textContent = err instanceof RecintoInvalidoError ? err.message : String(err);
  }
}

/** Atajo del LABORATORIO, no una mecánica del juego: borra los recintos para poder volver a probar sobre la
 * misma ciudad sin refundar. La mecánica real de abandono (con su coste hundido) es del Paso 2b. */
function quitarMurallas(): void {
  const actual = estado.asentamientos[0];
  if (!actual) return;
  estado = { ...estado, asentamientos: [{ ...actual, recintos: [] }] };
  murallaPropuesta = undefined;
  murallaTrazo = undefined;
  computar();
  murallaStatusEl.textContent = 'Murallas retiradas (atajo del laboratorio).';
}

/** Estado de la obra, refrescado en cada tick: sin esto la única forma de saber si el muro avanza sería
 * mirar el dibujo celda a celda. No pisa el presupuesto — mientras haya una propuesta en pantalla, manda ella. */
function refrescarEstadoDeMuralla(asentamiento: Asentamiento): void {
  if (murallaPropuesta) return;
  const recintos = asentamiento.recintos ?? [];
  murallaStatusEl.textContent =
    recintos.length === 0
      ? '—'
      : recintos
          .map((r) => {
            const puertas = r.celdas.filter((c) => c.clase === 'puerta').length;
            const pct = Math.round(((r.avance + 1) / r.celdas.length) * 100);
            const obra = pct === 100 ? 'cerrado' : `en obra ${r.avance + 1}/${r.celdas.length}`;
            return `nivel ${r.nivel} · ${pct}% (${obra}) · ${puertas} puertas`;
          })
          .join('  |  ');
}

murallaTrazarBtn.addEventListener('click', trazarMuralla);
murallaComprometerBtn.addEventListener('click', comprometerMuralla);
/** La mecánica real de abandono: solo mientras el anillo esté incompleto, y sin devolver nada. */
function abandonarMuralla(): void {
  const actual = estado.asentamientos[0];
  const recinto = actual?.recintos?.at(-1);
  if (!actual || !recinto) return;
  try {
    estado = { ...estado, asentamientos: [abandonarRecinto(actual, recinto.id)] };
    computar();
    murallaStatusEl.textContent = 'Recinto abandonado. El suelo queda libre; los materiales gastados no vuelven.';
  } catch (err) {
    murallaStatusEl.textContent = err instanceof RecintoInvalidoError ? err.message : String(err);
  }
}

murallaQuitarBtn.addEventListener('click', quitarMurallas);
murallaAbandonarBtn.addEventListener('click', abandonarMuralla);
// El nivel solo cambia tarifas y torres, no el trazo: si ya hay presupuesto, se recalcula al vuelo.
murallaNivelSel.addEventListener('change', () => {
  if (murallaPropuesta) trazarMuralla();
});

fundarBtn.addEventListener('click', () => fundar(Number(seedInput.value) || 1));
tick1Btn.addEventListener('click', () => avanzarNTicks(1));
tick10Btn.addEventListener('click', () => avanzarNTicks(10));
tick50Btn.addEventListener('click', () => avanzarNTicks(50));
autoBtn.addEventListener('click', toggleAuto);

fundar(Number(seedInput.value) || 1);
