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
import { celdasDeEdificio, redDeCalles, trazadoParaAsentamiento, type TrazadoAsentamiento } from '../../src/engine/trazado';
import { anadirEdificioManualmente, mejorarEdificioManualmente, reclamosDeFuentes, ConstruccionManualInvalidaError } from '../../src/engine/construction';
import { REJILLA_ASENTAMIENTO, TRAZADO, EDIFICIO_TAMANO, NIVEL_FACCION } from '../../src/constants';
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
  'barracon', 'galeriaDeTiro', 'mercado', 'maravilla', 'muralla',
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
let cacheTrazado: TrazadoAsentamiento = { calles: [], caminos: [], huellas: {} };
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

  // Gancho de depuración: el estado REAL del motor accesible desde la consola del navegador
  // (`__lab.asentamiento`, `__lab.trazado`, `__lab.estado`). El laboratorio existe para mirar; poder además
  // consultar y medir sin instrumentar nada es la mitad de su valor.
  (window as unknown as { __lab: unknown }).__lab = { tick, estado, asentamiento, trazado: cacheTrazado, mapa, red: redDeCalles(asentamiento.id, asentamiento.edificios), celdasDeEdificio, TRAZADO, EDIFICIO_TAMANO };

  const proy = proyeccion(canvas, REJILLA_ASENTAMIENTO.radioMapa);
  cacheEscala = proy.escala;
  cacheAPantalla = proy.aPantalla;

  cacheFilas = inspeccionarAnclas(asentamiento.edificios, asentamiento.id, tick, nacimientos);
  cacheLayoutArbol = calcularLayoutArbol(cacheFilas);

  statusEl.textContent =
    `Tick ${tick} · nivel ${asentamiento.nivel} · ${asentamiento.edificios.length} edificios · ` +
    `${cacheFilas.length} anclas de árbol · ${faltaParaNivel(asentamiento)}`;
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

fundarBtn.addEventListener('click', () => fundar(Number(seedInput.value) || 1));
tick1Btn.addEventListener('click', () => avanzarNTicks(1));
tick10Btn.addEventListener('click', () => avanzarNTicks(10));
tick50Btn.addEventListener('click', () => avanzarNTicks(50));
autoBtn.addEventListener('click', toggleAuto);

fundar(Number(seedInput.value) || 1);
