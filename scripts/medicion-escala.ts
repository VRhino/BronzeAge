// Re-medición de escala del motor (Fase E3). Ejecutar con: npx tsx scripts/medicion-escala.ts
//
// POR QUÉ existe: la tabla de escala de `Docs/Arquitectura/6_Sincronizacion_Visibilidad_y_Escala.md` §1 se
// midió el 2026-08-24, y el propio documento avisa de que hay que rehacerla al cerrar la Fase D ("el modelo
// temporal cambia el coste por tick"). La Fase D se cerró el 2026-08-30 y la medición nunca se rehizo, así
// que las cifras que sostienen la decisión MÁS importante que queda abierta —qué hacer con el bloqueo de la
// cola serial por ticks largos, que es lo que condiciona el objetivo de 500 jugadores— son de un balance y un
// modelo temporal que ya no son los vigentes.
//
// METODOLOGÍA, calcada de la de agosto para que los números sean COMPARABLES y no solo nuevos: N
// asentamientos, **una Facción por asentamiento** (el cap de fundación de nivel 1 es 1 por Facción, así que
// es la única forma de llegar a N sin falsear el reparto), 50 ticks de calentamiento y promedio sobre 30
// ticks. Se repiten los cuatro tamaños de la tabla original (10/16/33/52) y se añaden los dos que de verdad
// importan: **70 y 100 asentamientos son el objetivo de 500 jugadores**, no los 500 asentamientos que la
// tabla extrapolaba antes de la corrección del mismo doc (un asentamiento aloja 5 residentes + 2 por nivel).
//
// Lo que NO mide, a propósito: nada de red ni de persistencia. La conclusión de agosto —el cuello de botella
// es la CPU del tick, persistir cuesta <0.1% de un tick— se comprueba aquí de nuevo con la serialización al
// final, pero el objeto de la medición es el tick.
import { performance } from 'node:perf_hooks';
import type { Asentamiento, Faccion } from '../src/domain/types';
import type { Mapa } from '../src/world/mapa';
import { avanzarSimulacion } from '../src/engine/simulation';
import { crearFaccion } from '../src/engine/faccion';
import { createRng } from '../src/worldgen';
import {
  contextoDeTest,
  crearEstadoDeTest,
  crearMapaDeterminista,
  fundarAsentamientoDeTest,
  posicionRecomendable,
} from '../src/engine/__tests__/fixtures';

const SEED = 7;
const TICKS_CALENTAMIENTO = 50;
const TICKS_MEDIDOS = 30;
/** Los cuatro de la tabla de agosto (comparabilidad) + los dos del objetivo real de 500 jugadores. */
const TAMANOS = [10, 16, 33, 52, 70, 100];

/** Una Facción por asentamiento, como en la medición original. `crearFacciones()` de las fixtures solo da 3. */
function facciones(n: number): Faccion[] {
  return Array.from({ length: n }, (_, i) => crearFaccion(`faccion-${i}`, `Faccion ${i}`));
}

function escenario(n: number): { mapa: Mapa; estado: ReturnType<typeof crearEstadoDeTest> } {
  const mapa = crearMapaDeterminista(SEED);
  const fs = facciones(n);
  const asentamientos: Asentamiento[] = [];
  for (let i = 0; i < n; i++) {
    // `posicionRecomendable` barre la grilla evitando los ya fundados: con 2000x2000 y paso 40 hay ~2500
    // candidatas, de sobra para 100.
    const posicion = posicionRecomendable(mapa, asentamientos);
    const { asentamiento } = fundarAsentamientoDeTest(mapa, fs, fs[i]!.id, asentamientos, 0, posicion);
    asentamientos.push(asentamiento);
  }
  return { mapa, estado: crearEstadoDeTest(asentamientos, fs) };
}

interface Medida {
  n: number;
  msPorTick: number;
  kbEstado: number;
  msSerializar: number;
  asentamientosVivos: number;
}

function medir(n: number): Medida {
  const { mapa, estado: inicial } = escenario(n);
  const rng = createRng(SEED);
  let estado = inicial;
  let tick = 0;

  for (let i = 0; i < TICKS_CALENTAMIENTO; i++) {
    tick += 1;
    estado = avanzarSimulacion(estado, mapa, contextoDeTest(tick, rng));
  }

  const t0 = performance.now();
  for (let i = 0; i < TICKS_MEDIDOS; i++) {
    tick += 1;
    estado = avanzarSimulacion(estado, mapa, contextoDeTest(tick, rng));
  }
  const msPorTick = (performance.now() - t0) / TICKS_MEDIDOS;

  const json = JSON.stringify(estado);
  const s0 = performance.now();
  for (let i = 0; i < 50; i++) JSON.stringify(estado);
  const msSerializar = (performance.now() - s0) / 50;

  return {
    n,
    msPorTick,
    kbEstado: json.length / 1024,
    msSerializar,
    // Un asentamiento puede COLAPSAR durante el calentamiento (mantenimiento en déficit): si los vivos no son
    // los fundados, la fila no mide lo que su encabezado dice y hay que saberlo.
    asentamientosVivos: estado.asentamientos.length,
  };
}

console.log(`\nRe-medicion de escala — ${TICKS_CALENTAMIENTO} ticks de calentamiento, promedio de ${TICKS_MEDIDOS}.`);
console.log('Metodologia identica a la tabla de agosto del doc 6 §1, para poder comparar.\n');
console.log('  Asent.  vivos    ms/tick   estado (KB)   serializar   % del tick');
console.log('  ' + '-'.repeat(66));

const medidas: Medida[] = [];
for (const n of TAMANOS) {
  const m = medir(n);
  medidas.push(m);
  const pct = (m.msSerializar / m.msPorTick) * 100;
  console.log(
    `  ${String(m.n).padStart(5)}  ${String(m.asentamientosVivos).padStart(5)}   ` +
      `${m.msPorTick.toFixed(1).padStart(8)}   ${m.kbEstado.toFixed(0).padStart(10)}   ` +
      `${m.msSerializar.toFixed(2).padStart(9)}   ${pct.toFixed(2).padStart(9)}%`
  );
}

// Forma de la curva: se ajusta ms/tick = k·n^e por minimos cuadrados sobre los logaritmos. El EXPONENTE es lo
// que decide si el problema es tratable (lineal-ish) o no (cuadratico) — en agosto salio n^1.5.
const puntos = medidas.filter((m) => m.asentamientosVivos > 1);
const lx = puntos.map((m) => Math.log(m.asentamientosVivos));
const ly = puntos.map((m) => Math.log(m.msPorTick));
const mx = lx.reduce((a, b) => a + b, 0) / lx.length;
const my = ly.reduce((a, b) => a + b, 0) / ly.length;
const exponente = lx.reduce((s, x, i) => s + (x - mx) * (ly[i]! - my), 0) / lx.reduce((s, x) => s + (x - mx) ** 2, 0);

console.log(`\n  Escalado ajustado: O(n^${exponente.toFixed(2)})   (en agosto: O(n^1.5))`);

const cien = medidas.find((m) => m.n === 100);
if (cien) {
  console.log(`\n  A 100 asentamientos (~500-700 jugadores): ${cien.msPorTick.toFixed(0)} ms/tick.`);
  console.log(`  El tick dura 60 000 ms, asi que el motor ocupa el ${((cien.msPorTick / 60_000) * 100).toFixed(2)}% del intervalo.`);
  console.log('  Lo que importa para la cola serial NO es ese porcentaje sino la LATENCIA que un tick');
  console.log('  inyecta en los comandos que esperan detras de el.');
}
console.log();
