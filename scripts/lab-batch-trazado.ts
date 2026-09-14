// Corre, sobre MUCHAS seeds, el mismo guion manual que el usuario documentó a mano en `lab/` (mercado en
// tick 1 → +200 ticks → comprometer muralla → Barracón → Galería de tiro → +100 ticks) para medir cuánto
// salen los tres fallos que vio a simple vista en el laboratorio: ancla huérfana, ancla sin padre reconocible
// en el árbol (`codigo === '?'`, la misma señal que usa `lab/src/debugAnclas.ts`) y edificio sin frente de
// calle real. Reutiliza el motor y `inspeccionarAnclas` del laboratorio tal cual — no reimplementa nada de
// eso, solo automatiza la secuencia de clics sobre muchas seeds seguidas.
//
// `LAB_SEED_DESDE`/`LAB_SEED_HASTA`/`LAB_MURALLA_NIVEL`/`LAB_TICKS_FASE1`/`LAB_TICKS_FASE2`: overrides de
// entorno, mismo criterio que `run-batch-sim.ts` — sin variables, corre seeds 1-40 con muralla nivel 2,
// 200 ticks antes de la muralla y 100 después.
import type { Asentamiento, Faccion, Point, RecursoTipo } from '../src/domain/types';
import { createRng, generarMapa, MAPA_DEFAULT, type RandomFn } from '../src/worldgen';
import { crearMapa, type Mapa } from '../src/world/mapa';
import { crearFaccion } from '../src/engine/faccion';
import { asignarCargoLocal } from '../src/engine/cargos';
import { evaluarViabilidadFundacion, fundarAsentamiento } from '../src/engine/settlement';
import { avanzarSimulacion, type EstadoSimulacion } from '../src/engine/simulation';
import { celdaMinimaDeEdificio, celdasDeEdificio, edificiosInternos, redDeCalles, tamanoDeEdificio } from '../src/engine/trazado';
import { comprometerRecinto, RecintoInvalidoError, trazarRecinto } from '../src/engine/muralla';
import { anadirEdificioManualmente, ConstruccionManualInvalidaError, reclamosDeFuentes } from '../src/engine/construction';
import { NIVEL_FACCION } from '../src/constants';
import { instanteDeTick, isoDeInstante } from '../src/session/estado';
import { inspeccionarAnclas, type FilaAncla } from '../lab/src/debugAnclas';

const num = (nombre: string, porDefecto: number): number => {
  const crudo = process.env[nombre];
  const valor = crudo === undefined ? NaN : Number(crudo);
  return Number.isFinite(valor) ? Math.floor(valor) : porDefecto;
};

const SEED_DESDE = num('LAB_SEED_DESDE', 1);
const SEED_HASTA = num('LAB_SEED_HASTA', 40);
const MURALLA_NIVEL = num('LAB_MURALLA_NIVEL', 2);
const TICKS_FASE1 = num('LAB_TICKS_FASE1', 200);
const TICKS_FASE2 = num('LAB_TICKS_FASE2', 100);
/** Por defecto NO rellena materiales en cada tick — el lab del navegador solo los da una vez, al fundar
 * (`lab/src/main.ts::fundar`), y reproducir exactamente esa condición es lo que hace falta para perseguir
 * bugs (como la ancla huérfana de la seed 12) que el usuario vio a mano bajo esa misma escasez. `LAB_REABASTECER=1`
 * la activa para corridas largas donde el colapso por hambre (ver `medicion-muralla.ts`) taparía otra cosa. */
const REABASTECER_CADA_TICK = num('LAB_REABASTECER', 0) !== 0;

/** Mismo listado que `lab/src/main.ts::RECURSOS_LAB` — el asentamiento del lab nace y se mantiene con 9999
 * de cada uno para que la escasez de materiales nunca sea la variable que explica lo que se está probando. */
const RECURSOS: RecursoTipo[] = [
  'madera', 'piedra', 'trigo', 'cobre', 'estano', 'oro', 'livestock',
  'lingoteCobre', 'lingoteEstano', 'lingoteBronce',
  'cuero', 'cueroCurtido', 'cueroCalidad',
  'armaMadera', 'armaCobre', 'armaBronce', 'armaBronceCalidad',
  'armaduraBasica', 'armaduraIntermedia', 'armaduraBronce',
];

function conMaterialesInfinitos(a: Asentamiento): Asentamiento {
  const almacen = { ...a.almacen };
  for (const tipo of RECURSOS) almacen[tipo] = { cantidad: 9999, capacidad: 9999 };
  return { ...a, almacen };
}

/** Idéntico a `lab/src/main.ts::posicionRecomendable` — barrido en rejilla de paso 40 devolviendo el primer
 * punto `recomendable` (fundable + bosque LIBRE alcanzable). Deliberadamente NO exige piedra — a diferencia de
 * `buscarPosicionFundacionInicialPorDefecto` (session/npcGobernanza.ts) y de `elegirPosicionesFundacion`
 * (run-batch-sim.ts), que sí la exigen. Esa es justo la divergencia que reporta la nota extra del usuario. */
function posicionRecomendable(mapa: Mapa): Point {
  const paso = 40;
  for (let x = paso; x < mapa.limites.ancho; x += paso) {
    for (let y = paso; y < mapa.limites.alto; y += paso) {
      const posicion = { x, y };
      if (evaluarViabilidadFundacion(mapa, posicion, []).recomendable) return posicion;
    }
  }
  return { x: mapa.limites.ancho / 2, y: mapa.limites.alto / 2 };
}

/** Union-find mínimo sobre claves de celda, copiado de `run-batch-sim.ts::crearUnionFind` (no exportada
 * desde allí) — mismo criterio que ese script usa para `componentesDeRed`. */
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

const PARES_ADELANTE: readonly [number, number][] = [
  [1, 0],
  [0, 1],
];

interface AnalisisCalles {
  /** Trozos inconexos de la red (adyacencia ortogonal entre celdas de calle/camino). Debe ser 1 (o null si el
   * asentamiento todavía no tiene ninguna celda de red). */
  componentes: number | null;
  /** Edificios internos SIN ninguna celda de calle ortogonalmente adyacente — sin salida a la calle. */
  sinFrente: string[];
  /** Edificios internos con alguna celda propia solapando una celda de la red — no debería pasar nunca
   * (invariante ya cubierto por un test permanente), se comprueba igual porque es barato. */
  pisandoCalle: string[];
}

function analizarCalles(a: Asentamiento): AnalisisCalles {
  const internos = edificiosInternos(a.edificios);
  const red = redDeCalles(a.id, a.edificios, a.recintos ?? []);
  const celdas = new Set([...red.calles, ...red.caminos]);

  let componentes: number | null = null;
  if (celdas.size > 0) {
    const uf = crearUnionFind();
    for (const c of celdas) uf.agregar(c);
    for (const c of celdas) {
      const [col, row] = c.split(',').map(Number) as [number, number];
      for (const [dc, dr] of PARES_ADELANTE) {
        const vecina = `${col + dc},${row + dr}`;
        if (celdas.has(vecina)) uf.unir(c, vecina);
      }
    }
    componentes = uf.componentes();
  }

  const sinFrente: string[] = [];
  const pisandoCalle: string[] = [];
  for (const e of internos) {
    const etiqueta = `${e.tipo}#${e.id.slice(-6)}`;
    if (celdasDeEdificio(e).some((c) => celdas.has(`${c.col},${c.row}`))) pisandoCalle.push(etiqueta);

    const min = celdaMinimaDeEdificio(e);
    const t = tamanoDeEdificio(e);
    let frente = false;
    for (let dc = 0; dc < t.ancho && !frente; dc++) {
      if (celdas.has(`${min.col + dc},${min.row - 1}`) || celdas.has(`${min.col + dc},${min.row + t.alto}`)) frente = true;
    }
    for (let dr = 0; dr < t.alto && !frente; dr++) {
      if (celdas.has(`${min.col - 1},${min.row + dr}`) || celdas.has(`${min.col + t.ancho},${min.row + dr}`)) frente = true;
    }
    if (!frente) sinFrente.push(etiqueta);
  }
  return { componentes, sinFrente, pisandoCalle };
}

interface Rechazo {
  aceptado: boolean;
  motivo?: string;
}

interface ResultadoSeed {
  seed: number;
  fundacion: { x: number; y: number; id: string; tienePiedra: boolean };
  colapsoEnTick: number | null;
  mercado: Rechazo;
  barracon: Rechazo;
  galeria: Rechazo;
  muralla: { trazado: boolean; comprometido: boolean; motivo?: string; puertas?: number; celdas?: number };
  anclas: FilaAncla[];
  calles: AnalisisCalles | null;
  nivelFinal: number | null;
  edificiosFinal: number | null;
}

function encolar(
  asentamiento: Asentamiento,
  faccion: Faccion,
  tipo: Parameters<typeof anadirEdificioManualmente>[3],
  todos: Asentamiento[],
  mapa: Mapa,
  contador: number
): { asentamiento: Asentamiento; resultado: Rechazo } {
  try {
    const actualizado = anadirEdificioManualmente(asentamiento, faccion, 'gobernador', tipo, [], mapa, undefined, reclamosDeFuentes(todos), contador);
    return { asentamiento: actualizado, resultado: { aceptado: true } };
  } catch (err) {
    const motivo = err instanceof ConstruccionManualInvalidaError ? err.message : String(err);
    return { asentamiento, resultado: { aceptado: false, motivo } };
  }
}

function correrSeed(seed: number): ResultadoSeed {
  const mapa = crearMapa(generarMapa({ ...MAPA_DEFAULT, seed }));
  const rng: RandomFn = createRng(seed);

  const faccionBase: Faccion = {
    ...crearFaccion(`faccion-lab-${seed}`, 'Laboratorio'),
    nivel: NIVEL_FACCION.nivelMaximo,
    experiencia: (NIVEL_FACCION.xpParaNivel[NIVEL_FACCION.xpParaNivel.length - 1] ?? 0) + 1,
  };
  const posicion = posicionRecomendable(mapa);
  const viabilidad = evaluarViabilidadFundacion(mapa, posicion, []);
  const tienePiedra = viabilidad.recursosEnRadio.some((r) => r.tipo === 'piedra' && r.nodos > 0);

  const { asentamiento, facciones } = fundarAsentamiento(mapa, [faccionBase], faccionBase.id, posicion, ['jugador-lab'], [], instanteDeTick(0));
  const faccionLab = facciones[0]!;
  const conGobernador = asignarCargoLocal(conMaterialesInfinitos(asentamiento), faccionLab, 'gobernador', 'jugador-lab');

  let estado: EstadoSimulacion = {
    asentamientos: [conGobernador],
    facciones: [faccionLab],
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
    heroes: [],
  };
  let tick = 0;
  let contador = 0;
  const nacimientos = new Map<string, number>(conGobernador.edificios.map((e) => [e.id, 0]));

  const resultado: ResultadoSeed = {
    seed,
    fundacion: { x: posicion.x, y: posicion.y, id: asentamiento.id, tienePiedra },
    colapsoEnTick: null,
    mercado: { aceptado: false },
    barracon: { aceptado: false },
    galeria: { aceptado: false },
    muralla: { trazado: false, comprometido: false },
    anclas: [],
    calles: null,
    nivelFinal: null,
    edificiosFinal: null,
  };

  function registrarNacimientos(a: Asentamiento): void {
    for (const e of a.edificios) if (!nacimientos.has(e.id)) nacimientos.set(e.id, tick);
  }

  /** Avanza `n` ticks. Con `REABASTECER_CADA_TICK` en false (por defecto) el almacén evoluciona solo — igual
   * que el lab del navegador, que solo da materiales al fundar — así que una corrida muy larga puede colapsar
   * de verdad por mantenimiento; eso queda registrado en `colapsoEnTick`, no oculto. Con la variable activa
   * se rellena cada tick, mismo ajuste que documenta `scripts/medicion-muralla.ts` para corridas largas.
   * Devuelve `false` en cuanto el asentamiento colapsa. */
  function avanzar(n: number): boolean {
    for (let i = 0; i < n; i++) {
      tick += 1;
      const instante = instanteDeTick(tick);
      estado = avanzarSimulacion(estado, mapa, { instante, momento: isoDeInstante(instante), rng });
      const a = estado.asentamientos[0];
      if (!a) {
        resultado.colapsoEnTick = tick;
        return false;
      }
      const aFinal = REABASTECER_CADA_TICK ? conMaterialesInfinitos(a) : a;
      estado = { ...estado, asentamientos: [aFinal] };
      registrarNacimientos(aFinal);
    }
    return true;
  }

  // --- Paso 1: mercado en tick 1 ---
  {
    const a = estado.asentamientos[0]!;
    const { asentamiento: tras, resultado: r } = encolar(a, faccionLab, 'mercado', estado.asentamientos, mapa, contador++);
    resultado.mercado = r;
    estado = { ...estado, asentamientos: [tras] };
    registrarNacimientos(tras);
  }

  // --- Paso 2: 4×+50 ticks (200 por defecto) ---
  if (!avanzar(TICKS_FASE1)) return finalizar(resultado, null, tick, nacimientos);

  // --- Paso 3: comprometer muralla ---
  {
    const a = estado.asentamientos[0]!;
    const trazo = trazarRecinto(a, { nivel: MURALLA_NIVEL });
    if (!trazo) {
      resultado.muralla = { trazado: false, comprometido: false, motivo: 'Sin trazo válido: el anillo no se puede cerrar en esta ciudad.' };
    } else {
      resultado.muralla = { trazado: true, comprometido: false, puertas: trazo.puertas, celdas: trazo.celdas.length };
      try {
        const conMuro = comprometerRecinto(a, MURALLA_NIVEL, instanteDeTick(tick));
        estado = { ...estado, asentamientos: [conMuro] };
        resultado.muralla.comprometido = true;
      } catch (err) {
        resultado.muralla.motivo = err instanceof RecintoInvalidoError ? err.message : String(err);
      }
    }
  }

  // --- Paso 4: barracón ---
  {
    const a = estado.asentamientos[0]!;
    const { asentamiento: tras, resultado: r } = encolar(a, faccionLab, 'barracon', estado.asentamientos, mapa, contador++);
    resultado.barracon = r;
    estado = { ...estado, asentamientos: [tras] };
    registrarNacimientos(tras);
  }

  // --- Paso 5: galería de tiro ---
  {
    const a = estado.asentamientos[0]!;
    const { asentamiento: tras, resultado: r } = encolar(a, faccionLab, 'galeriaDeTiro', estado.asentamientos, mapa, contador++);
    resultado.galeria = r;
    estado = { ...estado, asentamientos: [tras] };
    registrarNacimientos(tras);
  }

  // --- Paso 6: 2×+50 ticks (100 por defecto) más ---
  if (!avanzar(TICKS_FASE2)) return finalizar(resultado, null, tick, nacimientos);

  return finalizar(resultado, estado.asentamientos[0]!, tick, nacimientos);
}

function finalizar(
  resultado: ResultadoSeed,
  asentamientoFinal: Asentamiento | null,
  tickFinal: number,
  nacimientos: Map<string, number>
): ResultadoSeed {
  if (!asentamientoFinal) return resultado;
  resultado.nivelFinal = asentamientoFinal.nivel;
  resultado.edificiosFinal = asentamientoFinal.edificios.length;
  // `nacimientos`/`tickFinal` REALES (no un Map vacío ni un tick fijo): `inspeccionarAnclas` calcula la edad
  // de cada ancla contra ellos, y `huerfana` exige `edad >= GRACIA_TICKS_HUERFANA` (debugAnclas.ts) — con un
  // Map vacío la edad sale siempre 0 y ninguna ancla puede marcarse huérfana jamás, sin importar la corrida.
  resultado.anclas = inspeccionarAnclas(asentamientoFinal.edificios, asentamientoFinal.id, tickFinal, nacimientos, asentamientoFinal.recintos ?? []);
  resultado.calles = analizarCalles(asentamientoFinal);
  return resultado;
}

// --- Corrida ---

const resultados: ResultadoSeed[] = [];
for (let seed = SEED_DESDE; seed <= SEED_HASTA; seed++) {
  resultados.push(correrSeed(seed));
}

function rechazoTxt(r: Rechazo): string {
  return r.aceptado ? 'OK' : `NO (${r.motivo ?? '?'})`;
}

console.log(`Corrida lab batch: seeds ${SEED_DESDE}-${SEED_HASTA}, muralla nivel ${MURALLA_NIVEL}\n`);
console.log(
  'seed | fundación (piedra) | colapso | mercado | barracón | galería | muralla | huérfanas | sin-padre(?) | calles: comp/sinFrente/pisa'
);
for (const r of resultados) {
  const fundacionTxt = `(${r.fundacion.x},${r.fundacion.y})${r.fundacion.tienePiedra ? '' : ' SIN PIEDRA'}`;
  const colapsoTxt = r.colapsoEnTick === null ? '—' : `tick ${r.colapsoEnTick}`;
  const murallaTxt = !r.muralla.trazado
    ? `sin trazo (${r.muralla.motivo ?? '?'})`
    : r.muralla.comprometido
      ? `OK ${r.muralla.puertas}p/${r.muralla.celdas}c`
      : `rechazada (${r.muralla.motivo ?? '?'})`;
  const huerfanas = r.anclas.filter((f) => f.huerfana);
  const sinPadre = r.anclas.filter((f) => f.codigo === '?');
  const callesTxt = r.calles ? `${r.calles.componentes ?? '—'}/${r.calles.sinFrente.length}/${r.calles.pisandoCalle.length}` : '—';
  console.log(
    `${String(r.seed).padStart(4)} | ${fundacionTxt.padEnd(19)} | ${colapsoTxt.padEnd(8)} | ` +
      `${rechazoTxt(r.mercado).padEnd(7)} | ${rechazoTxt(r.barracon).padEnd(8)} | ${rechazoTxt(r.galeria).padEnd(7)} | ` +
      `${murallaTxt.padEnd(20)} | ${String(huerfanas.length).padStart(9)} | ${String(sinPadre.length).padStart(12)} | ${callesTxt}`
  );
  for (const f of huerfanas) console.log(`       ⚠ huérfana: ${f.tipo} ${f.codigo} (id ...${f.id.slice(-6)})`);
  for (const f of sinPadre) console.log(`       ? sin padre: ${f.tipo} (id ...${f.id.slice(-6)})`);
  if (r.calles && r.calles.sinFrente.length > 0) console.log(`       sin frente de calle: ${r.calles.sinFrente.slice(0, 5).join(', ')}${r.calles.sinFrente.length > 5 ? '…' : ''}`);
  if (r.calles && r.calles.pisandoCalle.length > 0) console.log(`       PISANDO CALLE: ${r.calles.pisandoCalle.join(', ')}`);
}

// --- Resumen ---
const total = resultados.length;
const colapsados = resultados.filter((r) => r.colapsoEnTick !== null).length;
const sinPiedra = resultados.filter((r) => !r.fundacion.tienePiedra).length;
const barraconRechazado = resultados.filter((r) => !r.barracon.aceptado).length;
const galeriaRechazada = resultados.filter((r) => !r.galeria.aceptado).length;
const murallaRechazada = resultados.filter((r) => r.muralla.trazado && !r.muralla.comprometido).length;
const murallaSinTrazo = resultados.filter((r) => !r.muralla.trazado).length;
const conHuerfanas = resultados.filter((r) => r.anclas.some((f) => f.huerfana)).length;
const conSinPadre = resultados.filter((r) => r.anclas.some((f) => f.codigo === '?')).length;
const conMultiplesComponentes = resultados.filter((r) => (r.calles?.componentes ?? 1) > 1).length;
const conSinFrente = resultados.filter((r) => (r.calles?.sinFrente.length ?? 0) > 0).length;
const conPisandoCalle = resultados.filter((r) => (r.calles?.pisandoCalle.length ?? 0) > 0).length;

const posicionesUnicas = new Map<string, number>();
for (const r of resultados) {
  const clave = `${r.fundacion.x},${r.fundacion.y}`;
  posicionesUnicas.set(clave, (posicionesUnicas.get(clave) ?? 0) + 1);
}

console.log('\n--- RESUMEN ---');
console.log(`${total} seeds corridas.`);
console.log(`Colapsaron antes de terminar el guion: ${colapsados}/${total}`);
console.log(`Fundaron sin piedra alcanzable (sin Cantera posible): ${sinPiedra}/${total}`);
console.log(`Posiciones de fundación DISTINTAS entre las ${total} seeds: ${posicionesUnicas.size} (si es mucho menor que ${total}, muchas seeds repiten exactamente el mismo punto de la rejilla de 40)`);
for (const [pos, n] of [...posicionesUnicas.entries()].sort((a, b) => b[1] - a[1]).slice(0, 5)) {
  if (n > 1) console.log(`  ${pos}: ${n} seeds`);
}
console.log(`Barracón rechazado: ${barraconRechazado}/${total}`);
console.log(`Galería de tiro rechazada: ${galeriaRechazada}/${total}`);
console.log(`Muralla sin trazo válido: ${murallaSinTrazo}/${total}`);
console.log(`Muralla con trazo pero rechazada al comprometer: ${murallaRechazada}/${total}`);
console.log(`Con al menos un ancla huérfana al final: ${conHuerfanas}/${total}`);
console.log(`Con al menos un ancla sin padre reconocible ("?"): ${conSinPadre}/${total}`);
console.log(`Con la red de calles partida en más de un componente: ${conMultiplesComponentes}/${total}`);
console.log(`Con algún edificio sin frente de calle: ${conSinFrente}/${total}`);
console.log(`Con algún edificio pisando una celda de calle (no debería pasar nunca): ${conPisandoCalle}/${total}`);
