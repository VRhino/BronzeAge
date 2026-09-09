// EFÍMERO — Paso 0 de Consideraciones/Murallas_Definicion.md §18.
//
// Responde UNA pregunta antes de escribir una línea de la mecánica de murallas (§11):
//   ¿Cabe de verdad el tejido urbano de una ciudad madura dentro del disco de `radioMaximoMuro` (19 celdas)?
//
// El blind spot: `radioMaximoRanura()` = separacionMinimaAnclas × 3 = 36 CELDAS desde la semilla, mientras que
// las afueras empiezan en `radioAfuerasMin` = 60 unidades = 20 celdas. O sea que el árbol de anclas puede,
// sobre el papel, colocar un ancla MÁS LEJOS de lo que empiezan las granjas — y entonces un distrito entero
// nacería extramuros, o peor, el muro no podría encerrarlo nunca.
//
// Se borra cuando sus números estén anotados en el documento.
import type { Asentamiento, Faccion, RecursoTipo } from '../src/domain/types';
import { NIVEL_FACCION, REJILLA_ASENTAMIENTO, TRAZADO, ZONA_INFLUENCIA, type PerfilTrazado } from '../src/constants';
import { avanzarSimulacion } from '../src/engine/simulation';
import { createRng } from '../src/worldgen';
import { ANCLAS_REALES, celdasDeEdificio, edificiosInternos, esDeAfueras, redDeCalles } from '../src/engine/trazado';
import { trazarRecinto } from '../src/engine/muralla';
import {
  contextoDeTest,
  crearEstadoDeTest,
  crearFacciones,
  crearMapaDeterminista,
  fundarAsentamientoDeTest,
} from '../src/engine/__tests__/fixtures';

const T = REJILLA_ASENTAMIENTO.tamanoCelda;
/** Las afueras empiezan aquí, EN CELDAS. Ninguna celda de Granja/Corral entra de aquí para dentro (§E6.17). */
const RADIO_AFUERAS_CELDAS = TRAZADO.radioAfuerasMin / T;
/** El tope propuesto para el muro: una celda por dentro del radio vedado (§11 del doc de murallas). */
const RADIO_MAX_MURO = RADIO_AFUERAS_CELDAS - 1;

const RECURSOS: RecursoTipo[] = [
  'madera', 'piedra', 'trigo', 'cobre', 'estano', 'oro', 'livestock',
  'lingoteCobre', 'lingoteEstano', 'lingoteBronce',
  'cuero', 'cueroCurtido', 'cueroCalidad',
];

const tiposFuera = new Map<string, number>();

const SEEDS = [7, 13, 42, 60, 99];
const TICKS = Number(process.env['TICKS'] ?? 400);
const PERFIL = process.env['PERFIL'] as PerfilTrazado | undefined;
const NIVEL = Number(process.env['NIVEL'] ?? 2);

/** Distancia del CENTRO de una celda al origen del asentamiento, en celdas. */
function radioDeCelda(col: number, row: number): number {
  return Math.hypot(col + 0.5, row + 0.5);
}

function percentil(valores: number[], p: number): number {
  if (valores.length === 0) return 0;
  const orden = [...valores].sort((a, b) => a - b);
  return orden[Math.min(orden.length - 1, Math.floor((orden.length - 1) * p))]!;
}

/** Mismo montaje que el LABORATORIO (`lab/src/main.ts`): Facción a nivel máximo (si no, el cupo de la partida
 * congela el asentamiento en nivel 2 y nunca vemos una ciudad madura) + Gobernador + materiales de sobra.
 *
 * Los materiales se rellenan CADA TICK, a diferencia del lab, que solo los da al fundar: a 400+ ticks el
 * stock inicial se agota y el asentamiento colapsa por mantenimiento (comprobado: la primera versión de este
 * script perdía el asentamiento antes del tick 400). Es además la medición correcta para la pregunta: una
 * ciudad SIN restricción de materiales es la más grande que el trazado puede producir, así que si esa cabe
 * en el disco, cabe cualquiera. Es una cota superior, no un caso medio. */
function ciudad(seed: number, ticks: number): Asentamiento {
  const mapa = crearMapaDeterminista(seed);
  const facciónMax: Faccion = {
    ...crearFacciones()[0]!,
    nivel: NIVEL_FACCION.nivelMaximo,
    experiencia: (NIVEL_FACCION.xpParaNivel[NIVEL_FACCION.xpParaNivel.length - 1] ?? 0) + 1,
  };
  const { asentamiento, facciones } = fundarAsentamientoDeTest(mapa, [facciónMax], facciónMax.id, []);
  const preparado: Asentamiento = {
    ...asentamiento,
    nivel: NIVEL,
    nivelActual: NIVEL,
    radioPotencial: ZONA_INFLUENCIA.radioMaximoPorNivel[NIVEL] ?? asentamiento.radioPotencial,
    cargos: { ...asentamiento.cargos, gobernadorId: `jugador-${facciónMax.id}-1` },
  };
  let estado = crearEstadoDeTest([conMateriales(preparado)], facciones);
  const rng = createRng(seed);
  for (let tick = 1; tick <= ticks; tick++) {
    estado = avanzarSimulacion(estado, mapa, contextoDeTest(tick, rng));
    const a = estado.asentamientos[0];
    if (!a) throw new Error(`seed ${seed}: el asentamiento colapsó en el tick ${tick}`);
    estado = { ...estado, asentamientos: [conMateriales(a)] };
  }
  return estado.asentamientos[0]!;
}

function conMateriales(a: Asentamiento): Asentamiento {
  const almacen = { ...a.almacen };
  for (const tipo of RECURSOS) almacen[tipo] = { cantidad: 9999, capacidad: 9999 };
  return { ...a, almacen };
}

interface Medida {
  seed: number;
  edificiosUrbanos: number;
  radioMaxCelda: number;
  radioP50: number;
  radioP90: number;
  radioP99: number;
  /** Edificios urbanos con TODAS sus celdas dentro del disco de `RADIO_MAX_MURO`. */
  dentro: number;
  /** Edificios urbanos con alguna celda dentro y alguna fuera — el muro tendría que rodearlos o excluirlos. */
  aCaballo: number;
  /** Edificios urbanos enteramente fuera: nacen extramuros pase lo que pase. */
  fuera: number;
  radioMaxAncla: number;
  anclas: number;
  /** Aproximación al nº de puertas: celdas de red cuyo centro cae en la corona [R, R+1) del anillo. */
  crucesDeRedEnElAnillo: number;
  radioAnilloAprox: number;
  radioMinAfueras: number;
  urbanasMasAlla: number;
  celdasUrbanas: number;
}

function medir(seed: number, ticks: number): Medida {
  const a = ciudad(seed, ticks);
  const internos = edificiosInternos(a.edificios);
  const urbanos = internos.filter((e) => !esDeAfueras(e.tipo));

  const radiosCelda: number[] = [];
  let dentro = 0;
  let aCaballo = 0;
  let fuera = 0;
  let radioMaxCelda = 0;
  for (const e of urbanos) {
    let algunaDentro = false;
    let algunaFuera = false;
    for (const c of celdasDeEdificio(e)) {
      const r = radioDeCelda(c.col, c.row);
      radiosCelda.push(r);
      if (r > radioMaxCelda) radioMaxCelda = r;
      if (r <= RADIO_MAX_MURO) algunaDentro = true;
      else algunaFuera = true;
    }
    if (algunaDentro && algunaFuera) aCaballo += 1;
    else if (algunaDentro) dentro += 1;
    else fuera += 1;
  }

  // Qué tipos se salen: no es lo mismo que sobre el muro queden Leñeras sueltas (regla genérica, van donde
  // caben) que un ANCLA entera con su distrito — esa segunda es industria o milicia extramuros para siempre.
  for (const e of urbanos) {
    const radios = celdasDeEdificio(e).map((c) => radioDeCelda(c.col, c.row));
    if (Math.max(...radios) > RADIO_MAX_MURO) {
      const clave = `${e.tipo}${ANCLAS_REALES.has(e.tipo) ? ' (ANCLA)' : ''}`;
      tiposFuera.set(clave, (tiposFuera.get(clave) ?? 0) + 1);
    }
  }

  // ¿Siguen las afueras estando AFUERA? Radio de la celda de afueras más cercana al centro, contra cuántas
  // celdas urbanas hay MÁS ALLÁ de ella. Si ese número no es cero, granjas y ciudad están entremezcladas y
  // "las afueras" dejó de significar lo que dice §11.
  let radioMinAfueras = Infinity;
  for (const e of internos.filter((x) => esDeAfueras(x.tipo))) {
    for (const c of celdasDeEdificio(e)) radioMinAfueras = Math.min(radioMinAfueras, radioDeCelda(c.col, c.row));
  }
  const urbanasMasAlla = radiosCelda.filter((r) => r > radioMinAfueras).length;

  const anclas = urbanos.filter((e) => ANCLAS_REALES.has(e.tipo));
  let radioMaxAncla = 0;
  for (const e of anclas) {
    for (const c of celdasDeEdificio(e)) {
      const r = radioDeCelda(c.col, c.row);
      if (r > radioMaxAncla) radioMaxAncla = r;
    }
  }

  // Aproximación de puertas: el anillo real se traza por dilatación (§4), pero una corona circular al borde
  // del tejido urbano da el orden de magnitud de cuántos caminos/calles lo cruzarían.
  const red = redDeCalles(a.id, a.edificios);
  const radioAnillo = Math.min(RADIO_MAX_MURO, Math.ceil(radioMaxCelda) + 1);
  let cruces = 0;
  for (const clave of [...red.calles, ...red.caminos]) {
    const coma = clave.indexOf(',');
    const col = Number(clave.slice(0, coma));
    const row = Number(clave.slice(coma + 1));
    const r = radioDeCelda(col, row);
    if (r >= radioAnillo && r < radioAnillo + 1) cruces += 1;
  }

  return {
    seed,
    edificiosUrbanos: urbanos.length,
    radioMaxCelda,
    radioP50: percentil(radiosCelda, 0.5),
    radioP90: percentil(radiosCelda, 0.9),
    radioP99: percentil(radiosCelda, 0.99),
    dentro,
    aCaballo,
    fuera,
    radioMaxAncla,
    anclas: anclas.length,
    crucesDeRedEnElAnillo: cruces,
    radioAnilloAprox: radioAnillo,
    radioMinAfueras,
    urbanasMasAlla,
    celdasUrbanas: radiosCelda.length,
  };
}

const previo = TRAZADO.perfilForzado;
if (PERFIL) TRAZADO.perfilForzado = PERFIL;
try {
  console.log(`Paso 0 — murallas. ticks=${TICKS} perfil=${PERFIL ?? 'tradición'}`);
  console.log(`radioAfuerasMin = ${TRAZADO.radioAfuerasMin} unidades = ${RADIO_AFUERAS_CELDAS} celdas`);
  console.log(`radioMaximoMuro propuesto = ${RADIO_MAX_MURO} celdas`);
  console.log(`radioMaximoRanura = ${TRAZADO.separacionMinimaAnclas * 3} celdas (desde la semilla)`);
  const rp = ZONA_INFLUENCIA.radioMaximoPorNivel[NIVEL]!;
  console.log(`nivel ${NIVEL} → radioPotencial = ${rp} unidades = ${rp / T} celdas (tope de candidatosLibres)\n`);

  const filas = SEEDS.map((s) => medir(s, TICKS));
  console.log('seed | urb | rMax  | p50  | p90  | p99  | dentro/caballo/fuera | anclas rMax | cruces@R');
  for (const f of filas) {
    console.log(
      `${String(f.seed).padStart(4)} | ${String(f.edificiosUrbanos).padStart(3)} | ` +
        `${f.radioMaxCelda.toFixed(1).padStart(5)} | ${f.radioP50.toFixed(1).padStart(4)} | ` +
        `${f.radioP90.toFixed(1).padStart(4)} | ${f.radioP99.toFixed(1).padStart(4)} | ` +
        `${String(f.dentro).padStart(3)}/${String(f.aCaballo).padStart(2)}/${String(f.fuera).padStart(2)}` +
        `${' '.repeat(12)}| ${String(f.anclas).padStart(2)} ${f.radioMaxAncla.toFixed(1).padStart(5)} | ` +
        `${String(f.crucesDeRedEnElAnillo).padStart(2)}@${f.radioAnilloAprox}`
    );
  }

  const totalUrb = filas.reduce((s, f) => s + f.edificiosUrbanos, 0);
  const totalFuera = filas.reduce((s, f) => s + f.fuera + f.aCaballo, 0);
  const peorRadio = Math.max(...filas.map((f) => f.radioMaxCelda));
  const peorAncla = Math.max(...filas.map((f) => f.radioMaxAncla));
  console.log(
    `\nRESUMEN: ${totalFuera}/${totalUrb} edificios urbanos (${((100 * totalFuera) / totalUrb).toFixed(1)}%) ` +
      `no caben enteros en el disco de ${RADIO_MAX_MURO} celdas.`
  );
  console.log(`Radio máximo de celda urbana: ${peorRadio.toFixed(1)} celdas. Ancla más lejana: ${peorAncla.toFixed(1)} celdas.`);
  console.log('');
  console.log('EL TRAZO REAL (trazarRecinto, nivel 2 — con torres):');
  console.log('seed | celdas | puerta | torre | area | dentro/fuera | coste');
  for (const seed of SEEDS) {
    const a = ciudad(seed, TICKS);
    const t = trazarRecinto(a, { nivel: 2 });
    if (!t) {
      console.log(`${String(seed).padStart(4)} | TRAZO RECHAZADO`);
      continue;
    }
    const coste = Object.entries(t.costo)
      .map(([r, n]) => `${r} ${n}`)
      .join(' + ');
    console.log(
      `${String(seed).padStart(4)} | ${String(t.celdas.length).padStart(6)} | ${String(t.puertas).padStart(6)} | ` +
        `${String(t.torres).padStart(5)} | ${String(t.areaEncerrada).padStart(4)} | ` +
        `${String(t.dentro.length).padStart(3)}/${String(t.fuera.length).padStart(3)}      | ${coste}`
    );
  }

  console.log('');
  console.log('¿Siguen las afueras estando AFUERA?  (celdas urbanas MÁS LEJOS que la granja más cercana)');
  for (const f of filas) {
    const pct = (100 * f.urbanasMasAlla) / f.celdasUrbanas;
    console.log(
      `  seed ${String(f.seed).padStart(3)}: granja más cercana a ${f.radioMinAfueras.toFixed(1)} celdas · ` +
        `${String(f.urbanasMasAlla).padStart(4)}/${f.celdasUrbanas} celdas urbanas (${pct.toFixed(1)}%) están MÁS LEJOS que ella`
    );
  }
  console.log('');
  console.log('Tipos que se salen del disco (suma de las 5 seeds):');
  for (const [tipo, n] of [...tiposFuera.entries()].sort((a, b) => b[1] - a[1])) console.log(`  ${String(n).padStart(3)}  ${tipo}`);
  console.log(
    peorRadio <= RADIO_MAX_MURO
      ? `VEREDICTO: el tejido urbano CABE. radioAfuerasMin (${TRAZADO.radioAfuerasMin}) no necesita moverse.`
      : `VEREDICTO: NO cabe — el tejido llega a ${peorRadio.toFixed(1)} celdas contra un tope de ${RADIO_MAX_MURO}. ` +
          `radioAfuerasMin tendría que subir a ~${Math.ceil((peorRadio + 1) * T)} unidades para encerrarlo entero.`
  );
} finally {
  TRAZADO.perfilForzado = previo;
}
