import type { Asentamiento, Edificio, Faccion, Point } from '../src/domain/types';
import { createRng, generarMapa, MAPA_DEFAULT } from '../src/worldgen';
import { crearMapa, crearEstadoMapa, type Mapa } from '../src/world/mapa';
import { avanzarSimulacion, type EstadoSimulacion } from '../src/engine/simulation';
import { crearFaccion } from '../src/engine/faccion';
import { evaluarViabilidadFundacion, fundarAsentamiento } from '../src/engine/settlement';
import { nivelActualDe, tieneMercadoActivo, edificiosPorTipoYEstado, nutricionPoblacionDe } from '../src/engine/asentamientoQuery';
import { calcularNivelAsentamiento } from '../src/engine/mantenimiento';
import { avanzarNpcGobernanza, type ConfigNpcGobernanza, MINERALES_BONUS_FUNDACION } from '../src/session/npcGobernanza';
import { CATEGORIA_POR_TIPO, edificiosInternos, redDeCalles, segmentosDeRed } from '../src/engine/trazado';
import { REJILLA_ASENTAMIENTO } from '../src/constants';

/** Overrides por entorno para poder hacer pasadas cortas de humo sin esperar la corrida completa
 * (`BATCH_TICKS=200 BATCH_FACCIONES=10 node ...`). Sin variables, los valores son los de siempre — ninguna
 * corrida existente cambia de resultado. */
const num = (nombre: string, porDefecto: number) => {
  const crudo = process.env[nombre];
  const valor = crudo === undefined ? NaN : Number(crudo);
  return Number.isFinite(valor) && valor > 0 ? Math.floor(valor) : porDefecto;
};

const SEED = num('BATCH_SEED', 7);
const NUM_FACCIONES = num('BATCH_FACCIONES', 100);
const JUGADORES_POR_ASENTAMIENTO = 5;
const TICKS = num('BATCH_TICKS', 3000);
const FOTO_CADA = num('BATCH_FOTO_CADA', 100);
const MIN_SEPARACION = 100;

const TIPOS_EXTRACTOR = ['cantera', 'lenera', 'mina', 'minaCobre', 'minaEstano', 'corral'] as const;

/** Fecha fija de arranque y duración por tick para el `momento` de `ContextoSimulacion` — arbitrarias y
 * deterministas a propósito: nada del motor las usa todavía (el tick no tiene duración real hasta la Fase D),
 * solo sirven para que el momento avance de forma monótona sin depender del reloj de la máquina. */
const INICIO_BATCH = Date.UTC(2026, 0, 1, 0, 0, 0);
const MS_POR_TICK = 60_000;

function distancia(a: Point, b: Point): number {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

interface CandidatoFundacion {
  posicion: Point;
  tienePiedra: boolean;
  tieneOtroMineral: boolean;
  score: number;
}

/**
 * Selección de posiciones para la fundación INICIAL del batch — alineada con
 * `buscarPosicionFundacionInicialPorDefecto` (`src/session/npcGobernanza.ts`), que ya resolvió las mismas dos
 * decisiones para la partida real. La divergencia previa entre ambas (piedra como puntuación blanda en vez de
 * requisito, y un barrido que arrancaba en paso 100) hacía que el 76% de los asentamientos del batch fundaran
 * sin piedra alcanzable — sin Cantera, el gate de nivel 2 (Doc Fase_0_6: 3 de 6 extractores) es inalcanzable
 * sin importar cuánto avance la simulación (ver `issues/granjas_no_escalan_con_poblacion.md`, que arrancó
 * investigando por qué el batch nunca alcanzaba nivel 2).
 *
 * - **Piedra es requisito, no puntuación**: un candidato sin ningún nodo de piedra en el radio inicial NO
 *   entra en el pool. Mismo criterio que el NPC, misma razón: sin Cantera, ese asentamiento ya nació sin
 *   poder cumplir el gate.
 * - **El barrido no pasa de paso 25** — más grueso que eso se salta clusters de recursos enteros entre dos
 *   puntos consecutivos, el mismo hallazgo que documenta `PASO_BUSQUEDA_FUNDACION_INICIAL` en el NPC. Un paso
 *   más fino (10) sigue disponible como refuerzo SOLO si 25 no basta para separar `cantidad` posiciones — el
 *   batch, a diferencia del NPC, tiene que colocar muchas a la vez y necesita más candidatos que el NPC
 *   (que solo busca una).
 * - **El desempate usa `MINERALES_BONUS_FUNDACION`** (importada de `npcGobernanza.ts`, no duplicada): cuenta
 *   de 0 a 4 según cuántos de esos minerales tiene alcanzables, igual que el NPC — antes el batch usaba su
 *   propia lista de 3 sin oro, que además es uno de los 6 tipos del gate de nivel 2 (`mina`).
 */
function recolectarCandidatos(mapa: Mapa, paso: number): CandidatoFundacion[] {
  const candidatos: CandidatoFundacion[] = [];
  for (let x = paso; x < mapa.limites.ancho; x += paso) {
    for (let y = paso; y < mapa.limites.alto; y += paso) {
      const posicion = { x, y };
      const viabilidad = evaluarViabilidadFundacion(mapa, posicion, []);
      if (!viabilidad.recomendable) continue;
      const tienePiedra = viabilidad.recursosEnRadio.some((r) => r.tipo === 'piedra' && r.nodos > 0);
      if (!tienePiedra) continue;
      const bonusMinerales = MINERALES_BONUS_FUNDACION.filter((tipo) =>
        viabilidad.recursosEnRadio.some((r) => r.tipo === tipo && r.nodos > 0)
      ).length;
      candidatos.push({ posicion, tienePiedra, tieneOtroMineral: bonusMinerales > 0, score: bonusMinerales });
    }
  }
  return candidatos;
}

function elegirPosicionesFundacion(mapa: Mapa, cantidad: number): CandidatoFundacion[] {
  const elegidas: CandidatoFundacion[] = [];
  for (const paso of [25, 10]) {
    if (elegidas.length >= cantidad) break;
    const pool = recolectarCandidatos(mapa, paso).sort((a, b) => b.score - a.score);
    for (const candidato of pool) {
      if (elegidas.length >= cantidad) break;
      if (elegidas.every((p) => distancia(p.posicion, candidato.posicion) >= MIN_SEPARACION)) {
        elegidas.push(candidato);
      }
    }
  }
  return elegidas;
}

// --- Métricas de TRAZADO URBANO (Etapa 0 del rediseño "anclas y satélites") ---
//
// Existen para tener LÍNEA BASE del sistema de barrios ANTES de tocar la colocación. Sin ellas no hay forma
// de decir si el rediseño mejora o empeora la ciudad: "se ve peor" no es medible, y el consejo que revisó la
// mecánica coincidió en que reescribir el núcleo de colocación sin una línea base es un acto de fe.
// Especificación de lo que se va a medir: `Consideraciones/Vista_Asentamiento_Trazado_Urbano.md`.
//
// Las tres se calculan SOLO sobre el trazado ya existente — no anticipan nada del rediseño, así que sirven
// igual para el "antes" (barrios) y para el "después" (anclas).

const CELDA = REJILLA_ASENTAMIENTO.tamanoCelda;

/** Union-find mínimo sobre claves de texto, para contar componentes conexas sin traer una dependencia. */
function crearUnionFind() {
  const padre = new Map<string, string>();
  const raiz = (x: string): string => {
    let actual = padre.get(x) ?? x;
    if (!padre.has(x)) padre.set(x, x);
    while (actual !== (padre.get(actual) ?? actual)) actual = padre.get(actual)!;
    padre.set(x, actual);
    return actual;
  };
  return {
    agregar: (x: string) => raiz(x),
    unir: (a: string, b: string) => {
      const ra = raiz(a);
      const rb = raiz(b);
      if (ra !== rb) padre.set(ra, rb);
    },
    componentes: () => new Set([...padre.keys()].map(raiz)).size,
  };
}

/**
 * Manzanas cerradas = ciclos independientes de la red de calles. Sale de la fórmula de Euler para grafos
 * planos (`ciclos = aristas − vértices + componentes`), así que no hay que buscar los ciclos: basta contarlos.
 *
 * Es el GUARDIÁN DE REGRESIÓN del rediseño. `candidatosLibres` (engine/trazado.ts) documenta una regresión ya
 * medida: sin preferir la continuación de fila, la ciudad crece como un borrón compacto y solo 3 de 50
 * edificios llegaban a cerrar manzana. La prioridad "pegado al ancla" (§5.3) vuelve a poner presión justo
 * ahí, y esta métrica es lo que avisará si la reintroduce.
 */
function manzanasCerradas(asentamiento: Asentamiento): number {
  const { calles } = segmentosDeRed(redDeCalles(asentamiento.id, asentamiento.edificios));
  if (calles.length === 0) return 0;

  const uf = crearUnionFind();
  const clave = (p: Point) => `${Math.round(p.x)},${Math.round(p.y)}`;
  const vertices = new Set<string>();
  for (const segmento of calles) {
    const a = clave(segmento.desde);
    const b = clave(segmento.hasta);
    vertices.add(a);
    vertices.add(b);
    uf.agregar(a);
    uf.agregar(b);
    uf.unir(a, b);
  }
  return calles.length - vertices.size + uf.componentes();
}

/**
 * Cuántos GRUPOS SEPARADOS forma una categoría dentro del asentamiento (enlace simple: dos edificios caen en
 * el mismo grupo si sus centros están a `umbralCeldas` o menos).
 *
 * Es la métrica del OBJETIVO del rediseño. Con barrios, una categoría es una sola mancha que crece
 * radialmente y este número debería quedarse pegado a 1; con núcleos debería subir a medida que la ciudad
 * abre anclas nuevas. El umbral es arbitrario pero constante entre el "antes" y el "después", que es lo único
 * que hace falta para que la comparación signifique algo.
 */
function componentesDeCategoria(edificios: Edificio[], umbralCeldas: number): number {
  if (edificios.length === 0) return 0;
  const uf = crearUnionFind();
  edificios.forEach((_, i) => uf.agregar(String(i)));
  for (let i = 0; i < edificios.length; i++) {
    for (let j = i + 1; j < edificios.length; j++) {
      if (distancia(edificios[i]!.posicion, edificios[j]!.posicion) / CELDA <= umbralCeldas) {
        uf.unir(String(i), String(j));
      }
    }
  }
  return uf.componentes();
}

/** Umbral de enlace simple para `componentesDeCategoria`, en celdas. Del orden del diámetro que tendría un
 * núcleo con `separacionMinimaAnclas = 6` (§5.3: `radioMaximoNucleo = separacionMinimaAnclas / 2`). */
const UMBRAL_COMPONENTE_CELDAS = 4;

const CATEGORIAS_MEDIDAS = ['residencial', 'industria', 'militar', 'mercado', 'almacenaje'] as const;

/** Distancia media, en celdas, de cada satélite a su pieza de referencia. `null` si no hay ningún par que
 * medir (el asentamiento no tiene todavía ese edificio). Mide la DERIVA: el defecto (b) que motiva el
 * rediseño es que la acreción mira al vecino más cercano de la categoría, no a la pieza que debería ordenar
 * el grupo, así que la cadena puede alejarse paso a paso. */
function dispersionMedia(satelites: Edificio[], referencia: Edificio | undefined): number | null {
  if (!referencia || satelites.length === 0) return null;
  const suma = satelites.reduce((acc, s) => acc + distancia(s.posicion, referencia.posicion) / CELDA, 0);
  return suma / satelites.length;
}

interface Foto {
  tick: number;
  vivos: number;
  colapsados: number;
  excepcionesAcumuladas: number;
  nivelesFaccion: Record<string, number>;
  nivelFaccionMax: number;
  xpFaccionMin: number;
  xpFaccionMedia: number;
  xpFaccionMax: number;
  faccionesConMasDeUnAsentamiento: number;
  caravanasFundacionLanzadasAcumuladas: number;
  nivelesAsentamiento: Record<string, number>;
  artesanosTotal: number;
  edificiosTransformacionActivosTotal: number;
  murallasActivas: number;
  palaciosActivos: number;
  conMercado: number;
  extraccionPorTipoActivosTotal: Record<string, number>;
  conGateNivel2Cumplido: number;
  tropasVivas: number;
  reclutamientosAcumulados: number;
  campamentosDestruidosAcumulados: number;
  truequesSupervivenciaAcumulados: number;
  acuerdosActivos: number;
  acuerdosCumplidos: number;
  // --- Trazado urbano (línea base para el rediseño "anclas y satélites") ---
  /** Ciclos de la red de calles por asentamiento, promediado. Guardián de regresión de la alineación. */
  manzanasCerradasMedia: number;
  /** Asentamientos con al menos una manzana cerrada — el promedio solo no distingue "pocas ciudades con
   * muchas manzanas" de "muchas ciudades con una". */
  conAlgunaManzanaCerrada: number;
  /** Distancia media puesto→Mercado en celdas, sobre los asentamientos que tienen ambos. Mide la deriva. */
  dispersionPuestoMercado: number | null;
  /** Distancia media Vivienda→Centro Urbano en celdas. La otra relación ancla/satélite que ya existe hoy. */
  dispersionViviendaCentro: number | null;
  /** Grupos separados que forma cada categoría, promediado sobre los asentamientos que tienen esa categoría.
   * Con barrios debería rondar 1 (una sola mancha); con núcleos debería crecer. */
  componentesPorCategoria: Record<string, number | null>;
  // --- Población contra el gate de nivel 2 (diagnóstico del estancamiento en nivel 1) ---
  /** Viviendas activas por asentamiento. El tope en nivel 1 es 14 (`maximoViviendasPorNivel`), que da 210 de
   * capacidad de pesants contra los 200 que pide el gate: si esto no llega a 14, el gate es inalcanzable por
   * falta de capacidad, no por falta de crecimiento. */
  viviendasMedia: number;
  /** Granjas por asentamiento: activas (produciendo), en obra/cola (pagadas, todavía no producen), y nivel
   * interno medio de las activas (1-4, ver §7 del trazado — el rinde sube ×1/×1.5/×2/×3 con el nivel sin
   * aumentar `trabajadoresRequeridos`). Faltaba en la instrumentación: hasta ahora el diagnóstico de hambre se
   * apoyaba solo en nutrición/pesants, nunca en contar Granjas de verdad — este campo lo cierra. */
  granjasActivasMedia: number;
  granjasPendientesMedia: number;
  granjasNivelInternoMedia: number | null;
  /** Pesants por asentamiento, promedio y máximo. El máximo es el que dice si ALGÚN asentamiento se acerca
   * siquiera a los 200 del gate — el promedio lo esconde. */
  pesantsMedia: number;
  pesantsMaximo: number;
  /** Nutrición media (0-100). Entra como factor multiplicativo directo del crecimiento
   * (`comidaFactor`, engine/population.ts), así que una nutrición baja frena el pool aunque sobre capacidad. */
  nutricionMedia: number;
}

function construirFotoResumen(
  estado: EstadoSimulacion,
  tick: number,
  colapsados: number,
  excepcionesAcumuladas: number,
  reclutamientosAcumulados: number,
  campamentosDestruidosAcumulados: number,
  truequesSupervivenciaAcumulados: number,
  caravanasFundacionLanzadasAcumuladas: number
): Foto {
  const vivos = estado.asentamientos.length;

  const nivelesFaccion: Record<string, number> = {};
  let nivelFaccionMax = 0;
  let xpMin = Infinity;
  let xpMax = -Infinity;
  let xpSuma = 0;
  for (const f of estado.facciones) {
    nivelesFaccion[String(f.nivel)] = (nivelesFaccion[String(f.nivel)] ?? 0) + 1;
    if (f.nivel > nivelFaccionMax) nivelFaccionMax = f.nivel;
    if (f.experiencia < xpMin) xpMin = f.experiencia;
    if (f.experiencia > xpMax) xpMax = f.experiencia;
    xpSuma += f.experiencia;
  }

  const porFaccion = new Map<string, number>();
  for (const a of estado.asentamientos) porFaccion.set(a.faccionId, (porFaccion.get(a.faccionId) ?? 0) + 1);
  const faccionesConMasDeUnAsentamiento = [...porFaccion.values()].filter((n) => n > 1).length;

  const nivelesAsentamiento: Record<string, number> = { '1': 0, '2': 0, '3': 0, '4': 0, '5': 0 };
  let artesanosTotal = 0;
  let edificiosTransformacionActivosTotal = 0;
  let murallasActivas = 0;
  let palaciosActivos = 0;
  let conMercado = 0;
  let conGateNivel2Cumplido = 0;
  let tropasVivas = 0;
  const extraccionPorTipoActivosTotal: Record<string, number> = Object.fromEntries(TIPOS_EXTRACTOR.map((t) => [t, 0]));

  // Trazado urbano: se acumulan suma y contador por separado porque cada métrica solo aplica a los
  // asentamientos que tienen las piezas que mide (un asentamiento sin Mercado no aporta dispersión de puestos,
  // y promediarlo como 0 mentiría hacia abajo).
  let manzanasSuma = 0;
  let conAlgunaManzanaCerrada = 0;
  let dispersionPuestoSuma = 0;
  let dispersionPuestoN = 0;
  let dispersionViviendaSuma = 0;
  let dispersionViviendaN = 0;
  const componentesSuma: Record<string, number> = Object.fromEntries(CATEGORIAS_MEDIDAS.map((c) => [c, 0]));
  const componentesN: Record<string, number> = Object.fromEntries(CATEGORIAS_MEDIDAS.map((c) => [c, 0]));
  let viviendasSuma = 0;
  let pesantsSuma = 0;
  let pesantsMaximo = 0;
  let nutricionSuma = 0;
  let granjasActivasSuma = 0;
  let granjasPendientesSuma = 0;
  let granjasNivelSuma = 0;
  let granjasNivelN = 0;

  for (const a of estado.asentamientos) {
    const nivel = nivelActualDe(a);
    nivelesAsentamiento[String(nivel)] = (nivelesAsentamiento[String(nivel)] ?? 0) + 1;
    artesanosTotal += a.poblacion.artesanos;
    edificiosTransformacionActivosTotal +=
      edificiosPorTipoYEstado(a, 'curtiduria').length + edificiosPorTipoYEstado(a, 'armeria').length + edificiosPorTipoYEstado(a, 'fundicion').length;
    murallasActivas += edificiosPorTipoYEstado(a, 'muralla').length;
    palaciosActivos += edificiosPorTipoYEstado(a, 'palacio').length;
    if (tieneMercadoActivo(a)) conMercado++;
    if (calcularNivelAsentamiento(a) >= 2) conGateNivel2Cumplido++;
    tropasVivas += a.escuadrones.reduce((acc, e) => acc + e.cantidad, 0);
    for (const tipo of TIPOS_EXTRACTOR) {
      extraccionPorTipoActivosTotal[tipo] = (extraccionPorTipoActivosTotal[tipo] ?? 0) + edificiosPorTipoYEstado(a, tipo).length;
    }

    viviendasSuma += edificiosPorTipoYEstado(a, 'vivienda').length;
    pesantsSuma += a.poblacion.pesants;
    if (a.poblacion.pesants > pesantsMaximo) pesantsMaximo = a.poblacion.pesants;

    const granjasActivas = edificiosPorTipoYEstado(a, 'granja');
    granjasActivasSuma += granjasActivas.length;
    granjasPendientesSuma += a.edificios.filter((e) => e.tipo === 'granja' && e.estado !== 'activo').length;
    for (const g of granjasActivas) {
      granjasNivelSuma += g.nivelInterno ?? 1;
      granjasNivelN++;
    }
    nutricionSuma += nutricionPoblacionDe(a);

    // --- Trazado urbano ---
    const manzanas = manzanasCerradas(a);
    manzanasSuma += manzanas;
    if (manzanas > 0) conAlgunaManzanaCerrada++;

    // `edificiosInternos` descarta lo que vive en el mapa general (minas, cantera): no ocupan la rejilla local
    // y contarlos falsearía tanto la dispersión como las componentes.
    const internos = edificiosInternos(a.edificios);

    const dispersionPuesto = dispersionMedia(
      internos.filter((e) => e.tipo === 'puestoMercado'),
      internos.find((e) => e.tipo === 'mercado')
    );
    if (dispersionPuesto !== null) {
      dispersionPuestoSuma += dispersionPuesto;
      dispersionPuestoN++;
    }

    const dispersionVivienda = dispersionMedia(
      internos.filter((e) => e.tipo === 'vivienda'),
      internos.find((e) => e.tipo === 'centroUrbano')
    );
    if (dispersionVivienda !== null) {
      dispersionViviendaSuma += dispersionVivienda;
      dispersionViviendaN++;
    }

    for (const categoria of CATEGORIAS_MEDIDAS) {
      const deLaCategoria = internos.filter((e) => CATEGORIA_POR_TIPO[e.tipo] === categoria);
      if (deLaCategoria.length === 0) continue;
      componentesSuma[categoria] = (componentesSuma[categoria] ?? 0) + componentesDeCategoria(deLaCategoria, UMBRAL_COMPONENTE_CELDAS);
      componentesN[categoria] = (componentesN[categoria] ?? 0) + 1;
    }
  }

  const redondear = (x: number) => Math.round(x * 100) / 100;
  const media = (suma: number, n: number): number | null => (n === 0 ? null : redondear(suma / n));

  const acuerdosActivos = estado.acuerdos.filter((ac) => ac.estado === 'activo').length;
  const acuerdosCumplidos = estado.acuerdos.filter((ac) => ac.estado === 'cumplido').length;

  return {
    tick,
    vivos,
    colapsados,
    excepcionesAcumuladas,
    nivelesFaccion,
    nivelFaccionMax,
    xpFaccionMin: xpMin === Infinity ? 0 : xpMin,
    xpFaccionMedia: Math.round((xpSuma / estado.facciones.length) * 100) / 100,
    xpFaccionMax: xpMax === -Infinity ? 0 : xpMax,
    faccionesConMasDeUnAsentamiento,
    caravanasFundacionLanzadasAcumuladas,
    nivelesAsentamiento,
    artesanosTotal,
    edificiosTransformacionActivosTotal,
    murallasActivas,
    palaciosActivos,
    conMercado,
    extraccionPorTipoActivosTotal,
    conGateNivel2Cumplido,
    tropasVivas,
    reclutamientosAcumulados,
    campamentosDestruidosAcumulados,
    truequesSupervivenciaAcumulados,
    acuerdosActivos,
    acuerdosCumplidos,
    manzanasCerradasMedia: vivos === 0 ? 0 : redondear(manzanasSuma / vivos),
    conAlgunaManzanaCerrada,
    dispersionPuestoMercado: media(dispersionPuestoSuma, dispersionPuestoN),
    dispersionViviendaCentro: media(dispersionViviendaSuma, dispersionViviendaN),
    componentesPorCategoria: Object.fromEntries(
      CATEGORIAS_MEDIDAS.map((c) => [c, media(componentesSuma[c] ?? 0, componentesN[c] ?? 0)])
    ),
    viviendasMedia: vivos === 0 ? 0 : redondear(viviendasSuma / vivos),
    granjasActivasMedia: vivos === 0 ? 0 : redondear(granjasActivasSuma / vivos),
    granjasPendientesMedia: vivos === 0 ? 0 : redondear(granjasPendientesSuma / vivos),
    granjasNivelInternoMedia: media(granjasNivelSuma, granjasNivelN),
    pesantsMedia: vivos === 0 ? 0 : redondear(pesantsSuma / vivos),
    pesantsMaximo,
    nutricionMedia: vivos === 0 ? 0 : redondear(nutricionSuma / vivos),
  };
}

async function main() {
  const rng = createRng(SEED);

  const mapaGenerado = generarMapa({ ancho: MAPA_DEFAULT.ancho, alto: MAPA_DEFAULT.alto, seed: SEED });
  const estadoMapa = crearEstadoMapa();
  const mapa = crearMapa(mapaGenerado, estadoMapa);

  let facciones: Faccion[] = [];
  for (let i = 0; i < NUM_FACCIONES; i++) {
    facciones.push(crearFaccion(`faccion-${i + 1}`, `Faccion ${i + 1}`));
  }

  const candidatos = elegirPosicionesFundacion(mapa, NUM_FACCIONES);
  const conPiedra = candidatos.filter((c) => c.tienePiedra).length;
  const conOtroMineral = candidatos.filter((c) => c.tieneOtroMineral).length;
  console.log(`Posiciones con piedra alcanzable: ${conPiedra}/${candidatos.length}`);
  console.log(`Posiciones con algún otro mineral alcanzable: ${conOtroMineral}/${candidatos.length}`);

  let asentamientos: Asentamiento[] = [];
  const idsFundados: string[] = [];
  for (let i = 0; i < candidatos.length; i++) {
    const faccion = facciones[i]!;
    const posicion = candidatos[i]!.posicion;
    const jugadores = Array.from({ length: JUGADORES_POR_ASENTAMIENTO }, (_, j) => `jugador-${faccion.id}-${j + 1}`);
    const resultado = fundarAsentamiento(mapa, facciones, faccion.id, posicion, jugadores, asentamientos, 0);
    asentamientos.push(resultado.asentamiento);
    idsFundados.push(resultado.asentamiento.id);
    facciones = resultado.facciones;
  }

  let estado: EstadoSimulacion = {
    asentamientos,
    facciones,
    caravanas: [],
    acuerdos: [],
    ordenes: [],
    relaciones: [],
    titulos: [],
    caminos: [],
    campamentosBandidos: [],
    bandidosProximoSpawnTick: 0,
  };

  // Palancas de EXPERIMENTO, ninguna cambia el comportamiento por defecto:
  // - `BATCH_SIN_RECLUTAMIENTO=1`: apunta `tropaId` a una tropa que no existe en `TROPAS_RECLUTABLES`, así
  //   que `reclutarTropa` lanza `ReclutamientoInvalidoError` y `reclutarParaTodos` lo traga en silencio —
  //   reclutamiento desactivado sin tocar una línea de la lógica del NPC.
  // - `BATCH_SIN_ATAQUES=1`: `atacarCampamentos: false` — el NPC deja de atacar campamentos de bandidos.
  // Sirven para aislar qué sostiene el reclutamiento continuo medido en el batch (`reclutamientosAcumulados`
  // sube sin que `tropasVivas` crezca): ¿deserción por hambre (moral colapsada, `avanzarMantenimientoTropas`)
  // o reposición de bajas de combate (`atacarCampamentosCercanos`, permadeath real)? Con las dos activas a la
  // vez se aísla cada mecanismo por separado — ver `issues/granjas_no_escalan_con_poblacion.md`.
  const config: ConfigNpcGobernanza = {
    ...(process.env['BATCH_SIN_RECLUTAMIENTO'] === '1' ? { tropaId: '__experimento_sin_reclutamiento__' } : {}),
    ...(process.env['BATCH_SIN_ATAQUES'] === '1' ? { atacarCampamentos: false } : {}),
  };

  const fotos: Foto[] = [];
  let excepcionesAcumuladas = 0;
  let idsVivosAntes = new Set(idsFundados);
  const idsColapsadosVistos = new Set<string>();
  let reclutamientosAcumulados = 0;
  let campamentosDestruidosAcumulados = 0;
  let truequesSupervivenciaAcumulados = 0;
  let caravanasFundacionLanzadasAcumuladas = 0;

  for (let tick = 1; tick <= TICKS; tick++) {
    try {
      // `momento` derivado del tick, no del reloj real: una corrida de batch tiene que ser reproducible
      // (mismo SEED -> mismo resultado), igual que los tests del motor.
      const contexto = { tick, momento: new Date(INICIO_BATCH + tick * MS_POR_TICK).toISOString(), rng };
      const trasMotor = avanzarSimulacion(estado, mapa, contexto);
      const trasNpc = avanzarNpcGobernanza(trasMotor, mapa, contexto, config);
      estado = trasNpc.estado;
      reclutamientosAcumulados += trasNpc.stats.reclutamientosExitosos;
      campamentosDestruidosAcumulados += trasNpc.stats.campamentosDestruidos;
      truequesSupervivenciaAcumulados += trasNpc.stats.truequesSupervivenciaPropuestos;
      caravanasFundacionLanzadasAcumuladas += trasNpc.stats.caravanasFundacionLanzadas;
    } catch (err) {
      excepcionesAcumuladas++;
    }

    const idsVivosAhora = new Set(estado.asentamientos.map((a) => a.id));
    for (const id of idsVivosAntes) {
      if (!idsVivosAhora.has(id)) idsColapsadosVistos.add(id);
    }
    idsVivosAntes = idsVivosAhora;

    if (tick % FOTO_CADA === 0) {
      fotos.push(
        construirFotoResumen(
          estado,
          tick,
          idsColapsadosVistos.size,
          excepcionesAcumuladas,
          reclutamientosAcumulados,
          campamentosDestruidosAcumulados,
          truequesSupervivenciaAcumulados,
          caravanasFundacionLanzadasAcumuladas
        )
      );
    }
  }

  console.log(`Excepciones totales: ${excepcionesAcumuladas}`);
  console.log(JSON.stringify({ fotos, excepciones: excepcionesAcumuladas }, null, 2));
}

main().catch(console.error);
