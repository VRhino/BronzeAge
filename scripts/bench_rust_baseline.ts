// Línea base de rendimiento para decidir si alguna capa merece pasar a Rust. Ejecutar con:
// npx tsx scripts/bench_rust_baseline.ts
//
// POR QUÉ existe: mide por separado las dos cargas de CPU puras del servidor — worldgen y el tick del
// motor— para poder responder "¿qué capa ganaría con Rust?" con cifras y no con intuición. Sus números son
// los que sostienen el estudio del modelo temporal (`Docs/Arquitectura/10_Modelo_Temporal.md`) y la decisión
// de que el tick no se puede bajar hoy. Se queda en el repo, como sus hermanos `medicion-*.ts`: la pregunta
// vuelve cada vez que la escala cambia, y entonces hay que poder RE-medir, no recordar.
import { performance } from 'node:perf_hooks';
import { avanzarSimulacion } from '../src/engine/simulation';
import { createRng } from '../src/worldgen';
import { generarMapa } from '../src/worldgen';
import { crearMapa } from '../src/world/mapa';
import {
  contextoDeTest,
  crearEstadoDeTest,
  crearFacciones,
  crearMapaDeterminista,
  fundarAsentamientoDeTest,
  posicionRecomendable,
} from '../src/engine/__tests__/fixtures';
import type { Asentamiento, Faccion } from '../src/domain/types';
import type { Mapa } from '../src/world/mapa';

function medir(etiqueta: string, veces: number, fn: () => void): number {
  fn(); // calentamiento
  const t0 = performance.now();
  for (let i = 0; i < veces; i++) fn();
  const total = performance.now() - t0;
  const media = total / veces;
  console.log(`  ${etiqueta.padEnd(46)} ${media.toFixed(2).padStart(9)} ms/op   (${veces} ops, ${total.toFixed(0)} ms)`);
  return media;
}

console.log('\n=== WORLDGEN (funcion pura de la semilla) ===');
for (const lado of [500, 1000, 2000]) {
  medir(`generarMapa ${lado}x${lado}`, lado >= 2000 ? 5 : 10, () => {
    generarMapa({ seed: 7, ancho: lado, alto: lado });
  });
}

console.log('\n=== TICK DEL MOTOR (avanzarSimulacion) ===');
function prepararEscenario(numAsentamientos: number) {
  const mapa: Mapa = crearMapaDeterminista(7);
  const facciones: Faccion[] = crearFacciones();
  const asentamientos: Asentamiento[] = [];
  for (let i = 0; i < numAsentamientos; i++) {
    const faccion = facciones[i % facciones.length]!;
    const posicion = posicionRecomendable(mapa, asentamientos);
    const { asentamiento } = fundarAsentamientoDeTest(mapa, facciones, faccion.id, asentamientos, 0, posicion);
    asentamientos.push(asentamiento);
  }
  return { mapa, estado: crearEstadoDeTest(asentamientos, facciones) };
}

for (const n of [1, 3]) {
  const { mapa, estado: inicial } = prepararEscenario(n);
  const rng = createRng(7);
  let estado = inicial;
  let tick = 0;
  medir(`avanzarSimulacion con ${n} asentamiento(s)`, 300, () => {
    tick += 1;
    estado = avanzarSimulacion(estado, mapa, contextoDeTest(tick, rng));
  });
}

console.log('\n=== SERIALIZACION DEL SNAPSHOT (lo que persiste cada tick) ===');
{
  const { mapa, estado: inicial } = prepararEscenario(3);
  const rng = createRng(7);
  let estado = inicial;
  for (let t = 1; t <= 200; t++) estado = avanzarSimulacion(estado, mapa, contextoDeTest(t, rng));
  const json = JSON.stringify(estado);
  console.log(`  tamano del estado tras 200 ticks: ${(json.length / 1024).toFixed(1)} KB`);
  medir('JSON.stringify(estado)', 200, () => {
    JSON.stringify(estado);
  });
  medir('JSON.parse(estado)', 200, () => {
    JSON.parse(json);
  });
}
console.log();
