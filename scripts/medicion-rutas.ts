// ¿Cuánto cuesta recalcular una ruta? Ejecutar con: npx tsx scripts/medicion-rutas.ts
//
// POR QUÉ existe: el jugador situado introduce dos cosas que piden pathfinding con una frecuencia que el
// motor no había visto nunca. Rectificar destino (`marcharA`) corre tantas veces como el jugador haga clic,
// y la PERSECUCIÓN recalcularía la ruta hacia un objetivo que se mueve — potencialmente en cada tick y por
// cada perseguidor. Hasta ahora `calcularRuta` solo corría al movilizar, un puñado de veces por partida.
//
// El diseño (`Consideraciones/Jugador_Situado_Definicion.md` §9) dejó esto anotado como la única medición
// pendiente, y con un motivo concreto: no cambia SI se persigue, cambia CÓMO. Recalcular cada tick, cada k
// ticks, o solo cuando el objetivo se ha desviado lo bastante son tres implementaciones distintas, y elegir
// sin cifras es elegir a ciegas.
import { performance } from 'node:perf_hooks';
import { createRng, generarMapa, MAPA_DEFAULT } from '../src/worldgen';
import { crearMapa } from '../src/world/mapa';
import { calcularRuta } from '../src/world/rutas';
import { SIMULACION } from '../src/constants';

const SEED = 42;
const rng = createRng(SEED);
const mapa = crearMapa(generarMapa({ ...MAPA_DEFAULT, seed: SEED }, rng), undefined);

/** Puntos de tierra firme, que es donde se calculan rutas de verdad. */
function puntosEnTierra(cuantos: number): { x: number; y: number }[] {
  const puntos: { x: number; y: number }[] = [];
  let intentos = 0;
  while (puntos.length < cuantos && intentos < cuantos * 200) {
    intentos++;
    const x = rng() * MAPA_DEFAULT.ancho;
    const y = rng() * MAPA_DEFAULT.alto;
    // Una ruta de longitud cero contra sí mismo confirma que el punto es transitable.
    if (calcularRuta(mapa, { x, y }, { x, y })) puntos.push({ x, y });
  }
  return puntos;
}

function medir(nombre: string, pares: [{ x: number; y: number }, { x: number; y: number }][]): number {
  const inicio = performance.now();
  let conRuta = 0;
  for (const [a, b] of pares) if (calcularRuta(mapa, a, b)) conRuta++;
  const ms = (performance.now() - inicio) / pares.length;
  console.log(`  ${nombre.padEnd(34)} ${ms.toFixed(3).padStart(8)} ms/ruta   (${conRuta}/${pares.length} con camino)`);
  return ms;
}

const tierra = puntosEnTierra(120);
if (tierra.length < 40) throw new Error('no se han encontrado suficientes puntos en tierra para medir');

const cerca: [{ x: number; y: number }, { x: number; y: number }][] = [];
const lejos: [{ x: number; y: number }, { x: number; y: number }][] = [];
for (let i = 0; i + 1 < tierra.length; i += 2) {
  const a = tierra[i]!;
  const b = tierra[i + 1]!;
  lejos.push([a, b]);
  // Una persecución recalcula hacia un objetivo que está CERCA: el perseguidor ya lo tiene a la vista.
  cerca.push([a, { x: a.x + 60, y: a.y + 60 }]);
}

console.log('\ncalcularRuta — coste por llamada (A* sobre coste de terreno)\n');
const msCerca = medir('objetivo a la vista (~85)', cerca);
const msLejos = medir('punto a punto del mapa', lejos);

console.log('\n  Lo que decide el CÓMO de la persecución:\n');
for (const n of [1, 5, 10, 25, 50]) {
  const porTick = msCerca * n;
  const pct = (porTick / SIMULACION.duracionTickMs) * 100;
  console.log(
    `  ${String(n).padStart(3)} persecuciones activas -> ${porTick.toFixed(1).padStart(7)} ms/tick   (${pct.toFixed(3)}% del tick de 60 s)`
  );
}
console.log(`\n  Referencia: el tick entero cuesta ~34 ms a 100 asentamientos (scripts/medicion-escala.ts).`);
console.log(`  Una ruta larga cuesta ${(msLejos / msCerca).toFixed(1)}x una corta, y una persecución solo hace cortas.\n`);
