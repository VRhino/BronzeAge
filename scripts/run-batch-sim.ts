import type { Asentamiento, Edificio, Faccion, Point } from '../src/domain/types';
import { createRng, generarMapa, MAPA_DEFAULT } from '../src/worldgen';
import { crearMapa, type Mapa } from '../src/world/mapa';
import { avanzarSimulacion, type EstadoSimulacion } from '../src/engine/simulation';
import { crearFaccion } from '../src/engine/faccion';
import { evaluarViabilidadFundacion, fundarAsentamiento } from '../src/engine/settlement';
import { nivelActualDe, tieneMercadoActivo, edificiosPorTipoYEstado, nutricionPoblacionDe } from '../src/engine/asentamientoQuery';
import { calcularNivelAsentamiento } from '../src/engine/mantenimiento';

import { reservaDeTrigo } from '../src/engine/tropas';
import { NECESIDADES } from '../src/constants';
const NECESIDADES_UMBRAL_AMPLIACION = NECESIDADES.umbralAlmacenAmpliacion;
import { avanzarNpcGobernanza, type ConfigNpcGobernanza, MINERALES_BONUS_FUNDACION } from '../src/session/npcGobernanza';
import {
  CATEGORIA_POR_TIPO,
  celdaMinimaDeEdificio,
  celdasDeEdificio,
  edificiosInternos,
  esDeAfueras,
  redDeCalles,
  tamanoDeEdificio,
  integridadDeRecinto,
} from '../src/engine/trazado';
import { costoDeTrazo, areaEncerradaDeRecinto, edificiosExtramurosDe } from '../src/engine/muralla';
import { EDIFICIO_CATALOGO, LOGISTICA, PERFILES_TRAZADO, SIMULACION, TRAZADO, ZONA_INFLUENCIA, type PerfilTrazado } from '../src/constants';
import { instanteDeTick, isoDeInstante } from '../src/session/estado';

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

/**
 * `BATCH_PERFIL=<nucleos|caminera|compacta|gremial>`: fuerza el perfil de trazado (doc trazado §E6.23) en
 * TODOS los asentamientos de la corrida, para poder comparar perfiles con las mismas semillas.
 *
 * Sin la variable no se toca nada y cada asentamiento usa el suyo (tradición local por id), que es lo que
 * corre en una partida real. Es la misma palanca que el selector del laboratorio — `TRAZADO.perfilForzado`.
 */
const PERFIL_FORZADO = process.env['BATCH_PERFIL'];
if (PERFIL_FORZADO !== undefined) {
  if (!(PERFILES_TRAZADO as readonly string[]).includes(PERFIL_FORZADO)) {
    throw new Error(`BATCH_PERFIL="${PERFIL_FORZADO}" no es un perfil válido (${PERFILES_TRAZADO.join(', ')}).`);
  }
  TRAZADO.perfilForzado = PERFIL_FORZADO as PerfilTrazado;
}

/**
 * `BATCH_TRIGO_X=<n>`: multiplica la producción base de trigo de TODOS los niveles de Granja.
 *
 * Existe para el experimento de producción de 2026-09-02 (ver `Consideraciones/Movimiento_Ejercitos_Definicion.md`
 * §9.4): la aritmética dice que un asentamiento nivel 1 a tope de población come 30 trigo/tick mientras una
 * Granja nivel 1 produce 15, así que el asentamiento nace en déficit estructural y eso es ANTERIOR a los
 * ejércitos. Esta palanca permite medir 1× / 2× / 3× con la misma seed sin tocar `constants.ts` entre
 * corridas — mismo criterio que `BATCH_SIN_RECLUTAMIENTO`/`BATCH_SIN_ATAQUES`.
 *
 * Muta el catálogo, que es el punto ÚNICO de lectura (`produccionTrigoDeGranja`, constants.ts). Sin la
 * variable no se toca nada y la corrida es idéntica a las de siempre.
 */
const TRIGO_X = Number(process.env['BATCH_TRIGO_X'] ?? 1);
if (Number.isFinite(TRIGO_X) && TRIGO_X > 0 && TRIGO_X !== 1) {
  const granja = EDIFICIO_CATALOGO.granja as { produccionBaseTrigo?: number; niveles?: Record<number, { produccionBaseTrigo?: number }> };
  if (granja.produccionBaseTrigo !== undefined) granja.produccionBaseTrigo *= TRIGO_X;
  for (const nivel of Object.values(granja.niveles ?? {})) {
    if (nivel.produccionBaseTrigo !== undefined) nivel.produccionBaseTrigo *= TRIGO_X;
  }
}

const TIPOS_EXTRACTOR = ['cantera', 'lenera', 'mina', 'minaCobre', 'minaEstano', 'corral'] as const;

function distancia(a: Point, b: Point): number {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

interface CandidatoFundacion {
  posicion: Point;
  tienePiedra: boolean;
  tieneOtroMineral: boolean;
  /** Leñeras que admite el bosque a su alcance — ver `MIN_CAPACIDAD_LENERAS`. */
  capacidadLeneras: number;
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
/**
 * Cuántas Leñeras da de sí el bosque que un asentamiento tendría a su alcance, sumando la capacidad de todos
 * los bosques cuyo borde entra en el radio indicado (`Mapa.capacidadLeneras`, 1-3 según el tamaño del disco).
 *
 * Existe porque `evaluarViabilidadFundacion` solo responde SÍ/NO (`bosqueAlcanzable`: hay al menos un bosque
 * tocando el radio inicial), y eso resultó ser una garantía mucho más débil de lo que parecía — ver
 * `MIN_CAPACIDAD_LENERAS`.
 */
function capacidadLenerasEnRadio(mapa: Mapa, centro: Point, radio: number): number {
  return mapa
    .listarBosques()
    .filter((b) => Math.hypot(b.centro.x - centro.x, b.centro.y - centro.y) < radio + b.radio)
    .reduce((suma: number, b) => suma + mapa.capacidadLeneras(b.id), 0);
}

/**
 * Capacidad mínima de Leñeras que se le exige a un emplazamiento del batch (a petición del usuario,
 * 2026-09-04): **un asentamiento del laboratorio tiene que poder farmear madera desde que se funda, y no
 * pararse por no tener de dónde sacarla.**
 *
 * Por qué hacía falta, medido: el filtro anterior era `viabilidad.recomendable`, que solo exige UN bosque
 * tocando el radio inicial. Con eso, los asentamientos del batch se quedaban en **5,5 Leñeras de media
 * contra un tope de 10** (`EXTRACCION_MAXIMOS.porTipo`) — o sea limitados por el bosque de su zona, no por
 * la regla. Y esa media era el cuello de botella real de todo lo demás: con la madera racionada, el
 * Almacén (50 de madera) nunca llegaba a pagarse —CERO almacenes en 600 ticks— y sin capacidad de
 * almacenaje el excedente de trigo se perdía contra el techo, que es lo que hacía imposible medir la
 * logística de campaña (ver `Consideraciones/Movimiento_Ejercitos_Definicion.md` §10.1).
 *
 * Se mide sobre el radio de nivel 2 y no sobre el inicial: la zona CRECE, y lo que interesa es si el sitio da
 * madera durante la vida del asentamiento, no solo el primer minuto.
 */
const MIN_CAPACIDAD_LENERAS = 8;

function recolectarCandidatos(mapa: Mapa, paso: number): CandidatoFundacion[] {
  const candidatos: CandidatoFundacion[] = [];
  const radioMaduro = ZONA_INFLUENCIA.radioMaximoPorNivel[2] ?? ZONA_INFLUENCIA.radioInicial;
  for (let x = paso; x < mapa.limites.ancho; x += paso) {
    for (let y = paso; y < mapa.limites.alto; y += paso) {
      const posicion = { x, y };
      const viabilidad = evaluarViabilidadFundacion(mapa, posicion, []);
      if (!viabilidad.recomendable) continue;
      const tienePiedra = viabilidad.recursosEnRadio.some((r) => r.tipo === 'piedra' && r.nodos > 0);
      if (!tienePiedra) continue;
      const capacidadLeneras = capacidadLenerasEnRadio(mapa, posicion, radioMaduro);
      if (capacidadLeneras < MIN_CAPACIDAD_LENERAS) continue;
      const bonusMinerales = MINERALES_BONUS_FUNDACION.filter((tipo) =>
        viabilidad.recursosEnRadio.some((r) => r.tipo === tipo && r.nodos > 0)
      ).length;
      candidatos.push({
        posicion,
        tienePiedra,
        tieneOtroMineral: bonusMinerales > 0,
        capacidadLeneras,
        // La madera manda en el desempate por encima de los minerales: sin ella no se construye NADA, y los
        // minerales solo deciden qué se puede construir después.
        score: capacidadLeneras * 10 + bonusMinerales,
      });
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

/**
 * Unidad FIJA en la que se expresan las métricas de DISTANCIA (`dispersion*`, `UMBRAL_COMPONENTE_CELDAS`).
 * Deliberadamente un literal y NO `REJILLA_ASENTAMIENTO.tamanoCelda`.
 *
 * Por qué: la Etapa 6 (§E6.11) parte la celda por la mitad (6 → 3) y dobla todas las huellas, de modo que la
 * geometría física no cambia. Si la dispersión se dividiera por la celda VIVA, el mismo edificio en el mismo
 * sitio pasaría de "3.44 celdas" a "6.88 celdas" — el doble, sin que nada haya empeorado, y la comparación
 * contra la línea base del Paso 0 (y contra los números históricos de las Etapas 1 y 2, medidos con celda 6)
 * quedaría corrupta en silencio.
 *
 * Congelándola en 6 la métrica pasa a ser una unidad FÍSICA estable, independiente de a qué resolución se
 * discretice la rejilla. Solo debe cambiar si cambia el tamaño físico de los edificios, no su representación.
 */
const CELDA_METRICA = 6;

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
  // `recintos`: mismo fix que `medirCalles` (Paso 2c) — sin esto, en cuanto la gobernanza NPC amuralla algo,
  // esta función seguiría contando calles sobre celdas que ahora son muro.
  const red = redDeCalles(asentamiento.id, asentamiento.edificios, asentamiento.recintos);
  const celdas = new Set([...red.calles, ...red.caminos]);
  if (celdas.size === 0) return 0;

  // Etapa 6, Paso 2: la red son CELDAS, no aristas. El grafo cuyos ciclos se cuentan pasa a ser el de
  // adyacencia ortogonal entre celdas de calle — misma fórmula de Euler, otro grafo. Cada par de celdas
  // vecinas aporta una arista; cada celda, un vértice.
  const uf = crearUnionFind();
  for (const clave of celdas) uf.agregar(clave);
  let aristas = 0;
  for (const clave of celdas) {
    const [col, row] = clave.split(',').map(Number) as [number, number];
    // Solo dos de las cuatro direcciones, para no contar cada arista dos veces.
    for (const [dc, dr] of PARES_ADELANTE) {
      const vecina = `${col + dc},${row + dr}`;
      if (!celdas.has(vecina)) continue;
      aristas++;
      uf.unir(clave, vecina);
    }
  }
  return aristas - celdas.size + uf.componentes();
}

/** Las dos direcciones que bastan para recorrer cada adyacencia ortogonal UNA vez. */
const PARES_ADELANTE: readonly [number, number][] = [
  [1, 0],
  [0, 1],
];

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
      if (distancia(edificios[i]!.posicion, edificios[j]!.posicion) / CELDA_METRICA <= umbralCeldas) {
        uf.unir(String(i), String(j));
      }
    }
  }
  return uf.componentes();
}

/** Umbral de enlace simple para `componentesDeCategoria`, en `CELDA_METRICA` (unidad física fija, no celdas de
 * la rejilla viva — ver `CELDA_METRICA`). Del orden del diámetro que tendría un núcleo con
 * `separacionMinimaAnclas` = 6 celdas de la rejilla ORIGINAL (§5.3: `radioMaximoNucleo = separacionMinimaAnclas / 2`).
 * No se toca al reescalar la rejilla en la Etapa 6: la distancia física que representa es la misma. */
const UMBRAL_COMPONENTE_CELDAS = 4;

// --- Métricas de la red de calles (Etapa 6, §E6.14 del doc de trazado) ---
//
// Se instrumentaron en el Paso 0, ANTES de tocar `engine/trazado.ts`, para tener línea base contra la que
// juzgar el rediseño — mismo orden que la Etapa 0 del rediseño de anclas.
//
// Deliberadamente NO dependían del formato de clave de arista (`H i,j`/`V i,j`), que era interno de
// `trazado.ts`: esa precaución es lo que permitió que sobrevivieran al Paso 2, donde ese formato desapareció.
// Lo que sí cambió de sentido con las celdas está explicado en `MedidasDeCalle`, más abajo.

const CATEGORIAS_MEDIDAS = ['residencial', 'industria', 'militar', 'mercado', 'almacenaje'] as const;

/** Distancia media, en celdas, de cada satélite a su pieza de referencia. `null` si no hay ningún par que
 * medir (el asentamiento no tiene todavía ese edificio). Mide la DERIVA: el defecto (b) que motiva el
 * rediseño es que la acreción mira al vecino más cercano de la categoría, no a la pieza que debería ordenar
 * el grupo, así que la cadena puede alejarse paso a paso. */
function dispersionMedia(satelites: Edificio[], referencia: Edificio | undefined): number | null {
  if (!referencia || satelites.length === 0) return null;
  const suma = satelites.reduce((acc, s) => acc + distancia(s.posicion, referencia.posicion) / CELDA_METRICA, 0);
  return suma / satelites.length;
}

/**
 * Medidas de la red de calles. Etapa 6, Paso 2: dos de las tres cambiaron de sentido al pasar de aristas a
 * celdas, y conviene dejar escrito por que.
 *
 * - `anchoCeroPct` **se retira**: medía tramos que separaban dos edificios distintos, o sea calles de ancho
 *   cero. Con las calles ocupando celdas eso es imposible por construccion, y lo congela un test permanente
 *   (`trazado.test.ts`, "ningun edificio pisa una celda de calle"). Una metrica que solo puede valer 0 no es
 *   una metrica, es un invariante — y como invariante vive mejor en la suite que en el batch.
 * - `conFrenteRealPct` se queda, por la razon inversa: sigue siendo el numero que dice si la ciudad tiene
 *   salida a la calle de verdad. Pasa de ~33% a deber ser 100.
 * - `componentesDeRed` es NUEVA y es la que 3D necesita: en cuantos trozos inconexos esta partida la red.
 *   Debe ser 1. No se podia ni formular con aristas.
 */
interface MedidasDeCalle {
  /** % de edificios internos con una CELDA de calle ortogonalmente adyacente — algo por lo que salir andando. */
  conFrenteRealPct: number | null;
  /** Trozos inconexos de la red (adyacencia ortogonal entre celdas de calle). Debe ser 1. */
  componentesDeRed: number | null;
  /** % de ocupacion del NUCLEO urbano (sin afueras) dentro de su propia caja. Mide el coagulo que produce la
   * atraccion dura y que la decision 4 de la Etapa 6 quiere aflojar — medido en 48-73% antes del rediseno. */
  ocupacionNucleoPct: number | null;
}

function medirCalles(asentamiento: Asentamiento): MedidasDeCalle {
  const internos = edificiosInternos(asentamiento.edificios);
  if (internos.length === 0) return { conFrenteRealPct: null, componentesDeRed: null, ocupacionNucleoPct: null };

  // `recintos`: desde que la gobernanza NPC compromete murallas (Paso 2c), medir sin esto vería una red que
  // ya no es la que el motor usa de verdad — el mismo bug que costó 68→47 edificios antes de centralizar
  // `ocupadasConRed` (ver Consideraciones/Murallas_Definicion.md, hallazgos del Paso 2a).
  const red = redDeCalles(asentamiento.id, asentamiento.edificios, asentamiento.recintos);
  const celdasRed = new Set([...red.calles, ...red.caminos]);

  let conFrenteReal = 0;
  for (const e of internos) {
    const min = celdaMinimaDeEdificio(e);
    const t = tamanoDeEdificio(e);
    let frente = false;
    for (let dc = 0; dc < t.ancho && !frente; dc++) {
      if (celdasRed.has(`${min.col + dc},${min.row - 1}`) || celdasRed.has(`${min.col + dc},${min.row + t.alto}`)) frente = true;
    }
    for (let dr = 0; dr < t.alto && !frente; dr++) {
      if (celdasRed.has(`${min.col - 1},${min.row + dr}`) || celdasRed.has(`${min.col + t.ancho},${min.row + dr}`)) frente = true;
    }
    if (frente) conFrenteReal++;
  }

  let componentesDeRed: number | null = null;
  if (celdasRed.size > 0) {
    const uf = crearUnionFind();
    for (const clave of celdasRed) uf.agregar(clave);
    for (const clave of celdasRed) {
      const [col, row] = clave.split(',').map(Number) as [number, number];
      for (const [dc, dr] of PARES_ADELANTE) {
        const vecina = `${col + dc},${row + dr}`;
        if (celdasRed.has(vecina)) uf.unir(clave, vecina);
      }
    }
    componentesDeRed = uf.componentes();
  }

  // Ocupacion del nucleo: Granja y Corral viven a `radioAfuerasMin` por diseno y estirarian la caja hasta
  // volver la metrica insignificante (medido: 7% con afueras dentro contra 69% sin ellas).
  const nucleo = internos.filter((e) => !esDeAfueras(e.tipo));
  let ocupacionNucleoPct: number | null = null;
  if (nucleo.length > 0) {
    let minCol = Infinity;
    let maxCol = -Infinity;
    let minRow = Infinity;
    let maxRow = -Infinity;
    let celdas = 0;
    for (const e of nucleo) {
      for (const c of celdasDeEdificio(e)) {
        celdas++;
        minCol = Math.min(minCol, c.col);
        maxCol = Math.max(maxCol, c.col);
        minRow = Math.min(minRow, c.row);
        maxRow = Math.max(maxRow, c.row);
      }
    }
    const caja = (maxCol - minCol + 1) * (maxRow - minRow + 1);
    ocupacionNucleoPct = caja === 0 ? null : (celdas / caja) * 100;
  }

  return { conFrenteRealPct: (conFrenteReal / internos.length) * 100, componentesDeRed, ocupacionNucleoPct };
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
  // --- Logística de campaña (Paso 6 del movimiento de ejércitos, Doc 5.13) ---
  /** Trigo que un asentamiento podría meter en un carro AHORA MISMO sin bajar de su reserva, promediado.
   * Es la autonomía que la economía puede PAGAR, frente a la capacidad teórica del carro.
   *
   * Es un SUELO, no la cifra exacta: se mide con la guarnición entera dentro, y sacar tropa reduce la propia
   * reserva (los que se van dejan de comer aquí), así que un jugador que se lleve 40 soldados libera
   * `40 × 0.15 × 8 = 48` de margen. A la escala del carro (500) esa corrección no cambia la conclusión, pero
   * conviene no leer este número como si fuera exacto. */
  sobranteParaCarroMedia: number;
  /** Asentamientos que ni siquiera llegan a llenar un carro entero. Si son casi todos, la constante de
   * capacidad está calibrada contra una economía que no existe (entrada para el Paso 13). */
  sinCarroCompleto: number;
  /** Asentamientos que no pueden aportar ni un grano: sacar un ejército de aquí es marchar en ayunas. */
  sinNadaQueCargar: number;
  /** Graneros activos y suma de sus niveles internos: si son 0, la capacidad de grano nueva no se está
   * usando y cualquier lectura sobre el excedente está midiendo otra cosa. */
  granerosActivos: number;
  /** Campañas del NPC (Paso 12): ejércitos vivos y columnas lanzadas en total. Si salen 0, la mecánica de
   * ejércitos no se está ejercitando y cualquier lectura del batch sobre ella no dice nada. */
  ejercitosVivos: number;
  campanasLanzadasAcumuladas: number;
  /** Repliegues: si son ~0 con muchas campañas, las columnas se están quedando fuera hasta morir de hambre. */
  replieguesAcumulados: number;
  conquistasAcumuladas: number;
  /** Asentamientos con el almacén de trigo por encima del umbral que dispara ampliar capacidad. */
  conTrigoDesbordado: number;
  /** Oro medio por asentamiento. Es el termómetro del comercio: la capacidad de una caravana decide cuánto
   * entrega por viaje, así que cualquier cambio ahí se lee aquí antes que en ningún otro sitio. */
  oroMedio: number;
  /** Almacenes activos: la otra mitad de la pregunta "¿por qué no crece la capacidad?" — si tampoco hay
   * Almacenes, el problema no es del Granero sino de que la ciudad no puede pagar almacenaje ninguno. */
  almacenesActivos: number;
  /** Leñeras activas por asentamiento. Contra `EXTRACCION_MAXIMOS.porTipo` dice si el límite es la regla o el
   * bosque disponible — la distinción que destapó el cuello de botella de la madera. */
  lenerasMedia: number;
  granerosNivelSuma: number;
  /** Ocupación media del almacén de trigo (0-1): es lo que dispara la construcción del Granero, así que si el
   * excedente no crece hay que saber si es porque no hay grano o porque el grano no llega a acumularse. */
  ocupacionTrigoMedia: number;
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
  // --- Línea base de la Etapa 6 (calles como celdas). Ver `medirCalles`. ---
  /** % de edificios con frente de calle REAL. Con celdas tiene que ser 100: es el invariante nuevo (§E6.12). */
  edificiosConFrenteRealPct: number | null;
  /** Trozos inconexos de la red por asentamiento. Debe ser 1 — lo que 3D necesita y las aristas no daban. */
  componentesDeRedMedia: number | null;
  /** % de ocupación del núcleo urbano dentro de su caja. Es el coágulo que la decisión 4 quiere aflojar. */
  ocupacionNucleoPct: number | null;
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
  // --- Murallas (Paso 2c): invariantes 4/6/9 en batch + guardián de que el gate de nivel 4 sea alcanzable ---
  /** Asentamientos con al menos un recinto (en obra o terminado). Si esto se queda en 0 con el batch corrido
   * lo bastante, `asegurarMuralla` no está disparando — mismo diagnóstico que
   * `issues/nivel_3_inalcanzable_sin_jugador_humano.md` tuvo con Barracón/Galería. */
  asentamientosConRecinto: number;
  /** De esos, cuántos tienen su recinto exterior COMPLETO (integridad 1). Es lo que de verdad habilita el
   * efecto defensivo (Paso 3b) y el gate de nivel 4 (Paso 5). */
  asentamientosConRecintoCompleto: number;
  /** Integridad media (0-1) de todos los recintos existentes, terminados o no — el pulso de "cuánta obra hay
   * en curso" del batch en un momento dado. */
  integridadRecintoMedia: number | null;
  /** Nº medio de celdas de los recintos COMPLETOS (perímetro real) — sustituye a `murallasActivas` (Paso 5,
   * §13: `muralla` ya no es un `EdificioTipo`, no hay nada que contar con `edificiosPorTipoYEstado`). */
  celdasMuroMedia: number | null;
  // --- Paso 4: arrabal y presión intramuros ---
  /** % de edificios urbanos (no Granja/Corral) que quedan FUERA del recinto exterior completo, medido solo
   * sobre asentamientos que tienen uno — un asentamiento sin recinto no aporta arrabal, contarlo como 0
   * mentiría hacia abajo. Es la contraparte de `ocupacionNucleoPct`/`manzanasCerradasMedia` de más abajo: si
   * la preferencia intramuros (§9) funciona, la presión que no cabe dentro debería aparecer aquí, no
   * acumularse como coágulo del núcleo. */
  arrabalPct: number | null;
}

function construirFotoResumen(
  estado: EstadoSimulacion,
  tick: number,
  colapsados: number,
  excepcionesAcumuladas: number,
  reclutamientosAcumulados: number,
  campamentosDestruidosAcumulados: number,
  truequesSupervivenciaAcumulados: number,
  caravanasFundacionLanzadasAcumuladas: number,
  campanasLanzadasAcumuladas: number,
  replieguesAcumulados: number,
  conquistasAcumuladas: number
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
  let palaciosActivos = 0;
  let conMercado = 0;
  let conGateNivel2Cumplido = 0;
  let tropasVivas = 0;
  let sobranteParaCarroSuma = 0;
  let sinCarroCompleto = 0;
  let sinNadaQueCargar = 0;
  let granerosActivos = 0;
  let conTrigoDesbordado = 0;
  let oroSuma = 0;
  let almacenesActivos = 0;
  let lenerasSuma = 0;
  let granerosNivelSuma = 0;
  let ocupacionTrigoSuma = 0;
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
  // Etapa 6: cada una lleva su propio contador porque son `null` de forma independiente (una ciudad puede
  // tener edificios pero ninguna calle todavía, o núcleo pero ninguna manzana).
  let componentesRedSuma = 0;
  let componentesRedN = 0;
  let frenteRealSuma = 0;
  let frenteRealN = 0;
  let ocupacionNucleoSuma = 0;
  let ocupacionNucleoN = 0;
  let viviendasSuma = 0;
  let pesantsSuma = 0;
  let pesantsMaximo = 0;
  let nutricionSuma = 0;
  let granjasActivasSuma = 0;
  let granjasPendientesSuma = 0;
  let granjasNivelSuma = 0;
  let granjasNivelN = 0;
  let asentamientosConRecinto = 0;
  let asentamientosConRecintoCompleto = 0;
  let integridadRecintoSuma = 0;
  let integridadRecintoN = 0;
  let celdasMuroSuma = 0;
  let celdasMuroN = 0;
  let arrabalSuma = 0;
  let arrabalN = 0;

  for (const a of estado.asentamientos) {
    const nivel = nivelActualDe(a);
    nivelesAsentamiento[String(nivel)] = (nivelesAsentamiento[String(nivel)] ?? 0) + 1;
    artesanosTotal += a.poblacion.artesanos;
    edificiosTransformacionActivosTotal +=
      edificiosPorTipoYEstado(a, 'curtiduria').length + edificiosPorTipoYEstado(a, 'armeria').length + edificiosPorTipoYEstado(a, 'fundicion').length;
    palaciosActivos += edificiosPorTipoYEstado(a, 'palacio').length;
    if (tieneMercadoActivo(a)) conMercado++;
    if (calcularNivelAsentamiento(a) >= 2) conGateNivel2Cumplido++;
    tropasVivas += a.escuadrones.reduce((acc, e) => acc + e.cantidad, 0);
    // Lo que este asentamiento podría cargar en un carro sin comprometer su despensa (Doc 5.13).
    const sobrante = Math.max(0, (a.almacen['trigo']?.cantidad ?? 0) - reservaDeTrigo(a));
    sobranteParaCarroSuma += sobrante;
    if (sobrante < LOGISTICA.capacidadCarroPorJugador) sinCarroCompleto++;
    if (sobrante <= 0) sinNadaQueCargar++;
    for (const g of edificiosPorTipoYEstado(a, 'granero')) {
      granerosActivos++;
      granerosNivelSuma += g.nivelInterno ?? 1;
    }
    const trigoAlm = a.almacen['trigo'];
    const ocupTrigo = trigoAlm && trigoAlm.capacidad > 0 ? trigoAlm.cantidad / trigoAlm.capacidad : 0;
    ocupacionTrigoSuma += ocupTrigo;
    if (ocupTrigo >= NECESIDADES_UMBRAL_AMPLIACION) conTrigoDesbordado++;
    oroSuma += a.almacen['oro']?.cantidad ?? 0;
    almacenesActivos += edificiosPorTipoYEstado(a, 'almacen').length;
    lenerasSuma += edificiosPorTipoYEstado(a, 'lenera').length;
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

    // --- Murallas (Paso 2c) ---
    const recintos = a.recintos ?? [];
    if (recintos.length > 0) {
      asentamientosConRecinto++;
      if (recintos.some((r) => integridadDeRecinto(r) >= 1)) asentamientosConRecintoCompleto++;
      for (const r of recintos) {
        integridadRecintoSuma += integridadDeRecinto(r);
        integridadRecintoN++;
        if (integridadDeRecinto(r) >= 1) {
          celdasMuroSuma += r.celdas.length;
          celdasMuroN++;
        }
      }

      // --- Arrabal (Paso 4) — solo tiene sentido con el recinto EXTERIOR completo: es lo que activa la
      // preferencia intramuros (`conPreferenciaIntramuros`, engine/trazado.ts) y por tanto lo único que
      // podría estar empujando edificios afuera en vez de apretarlos dentro.
      const exterior = recintos[recintos.length - 1]!;
      if (integridadDeRecinto(exterior) >= 1) {
        const urbanos = edificiosInternos(a.edificios).filter((e) => !esDeAfueras(e.tipo)).length;
        if (urbanos > 0) {
          arrabalSuma += (edificiosExtramurosDe(a, exterior) / urbanos) * 100;
          arrabalN++;
        }
      }
    }

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

    const calles = medirCalles(a);
    if (calles.componentesDeRed !== null) {
      componentesRedSuma += calles.componentesDeRed;
      componentesRedN++;
    }
    if (calles.conFrenteRealPct !== null) {
      frenteRealSuma += calles.conFrenteRealPct;
      frenteRealN++;
    }
    if (calles.ocupacionNucleoPct !== null) {
      ocupacionNucleoSuma += calles.ocupacionNucleoPct;
      ocupacionNucleoN++;
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
    palaciosActivos,
    conMercado,
    extraccionPorTipoActivosTotal,
    conGateNivel2Cumplido,
    tropasVivas,
    reclutamientosAcumulados,
    campamentosDestruidosAcumulados,
    truequesSupervivenciaAcumulados,
    sobranteParaCarroMedia:
      estado.asentamientos.length === 0 ? 0 : Math.round((sobranteParaCarroSuma / estado.asentamientos.length) * 100) / 100,
    sinCarroCompleto,
    sinNadaQueCargar,
    granerosActivos,
    ejercitosVivos: estado.ejercitos.length,
    campanasLanzadasAcumuladas,
    replieguesAcumulados,
    conquistasAcumuladas,
    conTrigoDesbordado,
    oroMedio: Math.round((oroSuma / Math.max(1, estado.asentamientos.length)) * 10) / 10,
    almacenesActivos,
    lenerasMedia: Math.round((lenerasSuma / Math.max(1, estado.asentamientos.length)) * 10) / 10,
    granerosNivelSuma,
    ocupacionTrigoMedia:
      estado.asentamientos.length === 0 ? 0 : Math.round((ocupacionTrigoSuma / estado.asentamientos.length) * 1000) / 1000,
    acuerdosActivos,
    acuerdosCumplidos,
    manzanasCerradasMedia: vivos === 0 ? 0 : redondear(manzanasSuma / vivos),
    conAlgunaManzanaCerrada,
    dispersionPuestoMercado: media(dispersionPuestoSuma, dispersionPuestoN),
    dispersionViviendaCentro: media(dispersionViviendaSuma, dispersionViviendaN),
    componentesPorCategoria: Object.fromEntries(
      CATEGORIAS_MEDIDAS.map((c) => [c, media(componentesSuma[c] ?? 0, componentesN[c] ?? 0)])
    ),
    edificiosConFrenteRealPct: media(frenteRealSuma, frenteRealN),
    componentesDeRedMedia: media(componentesRedSuma, componentesRedN),
    ocupacionNucleoPct: media(ocupacionNucleoSuma, ocupacionNucleoN),
    viviendasMedia: vivos === 0 ? 0 : redondear(viviendasSuma / vivos),
    granjasActivasMedia: vivos === 0 ? 0 : redondear(granjasActivasSuma / vivos),
    granjasPendientesMedia: vivos === 0 ? 0 : redondear(granjasPendientesSuma / vivos),
    granjasNivelInternoMedia: media(granjasNivelSuma, granjasNivelN),
    pesantsMedia: vivos === 0 ? 0 : redondear(pesantsSuma / vivos),
    pesantsMaximo,
    nutricionMedia: vivos === 0 ? 0 : redondear(nutricionSuma / vivos),
    asentamientosConRecinto,
    asentamientosConRecintoCompleto,
    integridadRecintoMedia: media(integridadRecintoSuma, integridadRecintoN),
    celdasMuroMedia: media(celdasMuroSuma, celdasMuroN),
    arrabalPct: media(arrabalSuma, arrabalN),
  };
}

async function main() {
  const rng = createRng(SEED);

  const mapaGenerado = generarMapa({ ancho: MAPA_DEFAULT.ancho, alto: MAPA_DEFAULT.alto, seed: SEED });
  // Una sola fachada para toda la corrida: es dueña de su propio estado de partida del mapa, así que los
  // yacimientos que se agotan y regeneran se acumulan tick a tick igual que en una partida real.
  const mapa = crearMapa(mapaGenerado);

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
    const resultado = fundarAsentamiento(mapa, facciones, faccion.id, posicion, jugadores, asentamientos, instanteDeTick(0));
    asentamientos.push(resultado.asentamiento);
    idsFundados.push(resultado.asentamiento.id);
    facciones = resultado.facciones;
  }

  let estado: EstadoSimulacion = {
    asentamientos,
    facciones,
    caravanas: [],
    ejercitos: [],
    memoriaPorFaccion: {},
    acuerdos: [],
    ordenes: [],
    relaciones: [],
    titulos: [],
    caminos: [],
    campamentosBandidos: [],
    bandidosProximoSpawnEn: instanteDeTick(0),
    jugadores: [],
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
  let campanasLanzadasAcumuladas = 0;
  let replieguesAcumulados = 0;
  // Las conquistas se cuentan por CAMBIO DE DUEÑO entre ticks: el evento vive en el log del motor y aquí solo
  // llega el estado, así que se deduce comparando a quién pertenecía cada plaza.
  let conquistasAcumuladas = 0;
  let duenoPorAsentamiento = new Map(estado.asentamientos.map((a) => [a.id, a.faccionId]));

  for (let tick = 1; tick <= TICKS; tick++) {
    try {
      // `instante`/`momento` derivados del tick con la misma fórmula que el backend (`instanteDeTick`,
      // Fase D / doc 10): una corrida de batch tiene que ser reproducible (mismo SEED -> mismo resultado),
      // así que nada del contexto puede depender del reloj de la máquina.
      const instante = instanteDeTick(tick);
      const contexto = { instante, momento: isoDeInstante(instante), rng };
      const trasMotor = avanzarSimulacion(estado, mapa, contexto);
      const trasNpc = avanzarNpcGobernanza(trasMotor, mapa, contexto, config);
      estado = trasNpc.estado;
      reclutamientosAcumulados += trasNpc.stats.reclutamientosExitosos;
      campamentosDestruidosAcumulados += trasNpc.stats.campamentosDestruidos;
      truequesSupervivenciaAcumulados += trasNpc.stats.truequesSupervivenciaPropuestos;
      caravanasFundacionLanzadasAcumuladas += trasNpc.stats.caravanasFundacionLanzadas;
      campanasLanzadasAcumuladas += trasNpc.stats.campanasLanzadas;
      replieguesAcumulados += trasNpc.stats.repliegues;
      for (const a of estado.asentamientos) {
        const antes = duenoPorAsentamiento.get(a.id);
        if (antes !== undefined && antes !== a.faccionId) conquistasAcumuladas++;
      }
      duenoPorAsentamiento = new Map(estado.asentamientos.map((a) => [a.id, a.faccionId]));
    } catch (err) {
      excepcionesAcumuladas++;
      // `BATCH_MOSTRAR_ERRORES=1` imprime las 3 primeras. Añadido tras perder un rato con un batch que daba
      // 600 excepciones silenciosas: el contador dice QUE algo falla, nunca QUÉ, y sin esto hay que
      // instrumentar a mano cada vez.
      if (process.env['BATCH_MOSTRAR_ERRORES'] === '1' && excepcionesAcumuladas <= 3) {
        console.error('EXCEPCION:', err instanceof Error ? err.message : String(err));
      }
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
          caravanasFundacionLanzadasAcumuladas,
          campanasLanzadasAcumuladas,
          replieguesAcumulados,
          conquistasAcumuladas
        )
      );
    }
  }

  console.log(`Excepciones totales: ${excepcionesAcumuladas}`);

  // --- El eje fortaleza↔metrópoli (§0/§18, Paso 2c) ---
  //
  // Una fila por recinto vivo al final de la corrida: el anillo está CONGELADO desde que se compromete (§5.1
  // — sus celdas, puertas y coste ya no cambian jamás), así que leerlo al final da el mismo número que leerlo
  // el día que se comprometió. Lo único que varía entre filas es `comprometidoEnTick`: es la variable
  // independiente de la tabla que dice si amurallar pronto (embudo barato, pocas puertas) y amurallar tarde
  // (más ciudad protegida, más puertas que cubrir) son los dos extremos jugables que el diseño quiere, o si
  // uno domina al otro — con ella se calibran `MURALLA.tarifaPorCelda`, `upkeepPorCelda` y el riesgo 11.
  interface FilaFortalezaMetropoli {
    asentamientoId: string;
    comprometidoEnTick: number;
    nivel: number;
    completo: boolean;
    celdas: number;
    puertas: number;
    torres: number;
    areaEncerrada: number | null;
    costoTotal: Partial<Record<string, number>>;
  }
  // Inverso de `instanteDeTick` (`session/estado.ts`, `EPOCA_MS` no exportada): `comprometidoEn` es un
  // `Instante` de mundo, y aquí solo hace falta en qué TICK de la corrida cayó.
  const epocaMs = new Date(SIMULACION.epocaInicial).getTime();
  const tickDe = (i: number) => Math.round((i - epocaMs) / SIMULACION.duracionTickMs);

  const filasMuralla: FilaFortalezaMetropoli[] = [];
  for (const a of estado.asentamientos) {
    for (const r of a.recintos ?? []) {
      filasMuralla.push({
        asentamientoId: a.id,
        comprometidoEnTick: tickDe(r.comprometidoEn),
        nivel: r.nivel,
        completo: integridadDeRecinto(r) >= 1,
        celdas: r.celdas.length,
        puertas: r.celdas.filter((c) => c.clase === 'puerta').length,
        torres: r.celdas.filter((c) => c.clase === 'torre').length,
        areaEncerrada: areaEncerradaDeRecinto(a, r),
        costoTotal: costoDeTrazo(r.celdas, r.nivel),
      });
    }
  }
  console.log(`\nRecintos vivos al final de la corrida: ${filasMuralla.length}`);
  console.log(JSON.stringify(filasMuralla, null, 2));

  console.log(JSON.stringify({ fotos, excepciones: excepcionesAcumuladas }, null, 2));
}

main().catch(console.error);
