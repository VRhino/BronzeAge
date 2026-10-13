// Mide el grafo de navegación sobre el relieve regional real: nodos, aristas, tiempo, fracción en la componente mayor y
// cuántos campamentos, nodos de metal y parcelas de bosque quedan fuera de ella (o sin nodo a un espaciado).
// Uso: npx tsx scripts/medir-navegacion.ts [seed]   (genera el relieve: ~2 min)
import { generarRegionalVisor } from './visor-worldgen.regional';
import { generarMundoEstrategico } from '../src/mundoEstrategico';
import { costeEnPunto, esTransitable } from '../src/costeMovimiento';
import { FACTOR } from '../src/campamentos';
import { componentes, construirGrafo, estadisticasGrafo, nodoMasCercano } from '../src/navegacion';
import type { Point } from '../src/tipos';

const seed = Number(process.argv[2] ?? 42), ANCHO = 2667 * FACTOR, ALTO = 2000 * FACTOR;
let t = Date.now();
const regional = generarRegionalVisor(seed, ANCHO, ALTO);
console.log(`relieve ${(Date.now() - t) / 1000} s`);

// Vados provisionales: un punto del cauce cada ~100 estratégicas de río (el generador definitivo los fijará de verdad).
const vados: Point[] = [];
for (const rio of regional.rios) {
  let acumulado = 0;
  for (let i = 1; i < rio.puntos.length; i++) {
    const a = rio.puntos[i - 1]!, b = rio.puntos[i]!;
    acumulado += Math.hypot(b.x - a.x, b.y - a.y);
    if (acumulado >= 100 * FACTOR) { vados.push(b); acumulado = 0; }
  }
}

let llamadasT = 0, llamadasC = 0, msT = 0, msC = 0;
const transitable = (p: Point) => { const t0 = performance.now(); const r = esTransitable(regional.elevacion, p); msT += performance.now() - t0; llamadasT++; return r; };
const coste = (p: Point) => { const t0 = performance.now(); const r = costeEnPunto(regional.elevacion, p); msC += performance.now() - t0; llamadasC++; return r; };
t = Date.now();
const grafo = construirGrafo({ ancho: ANCHO, alto: ALTO }, transitable, coste, vados);
const msGrafo = Date.now() - t;
console.log(`esTransitable: ${llamadasT} llamadas, ${(msT / 1000).toFixed(1)} s; costeEnPunto: ${llamadasC} llamadas, ${(msC / 1000).toFixed(1)} s; resto del algoritmo ${((msGrafo - msT - msC) / 1000).toFixed(2)} s`);
t = Date.now();
const comp = componentes(grafo), est = estadisticasGrafo(grafo, comp);
console.log(`grafo ${msGrafo} ms; componentes ${Date.now() - t} ms`);
console.log(`nodos ${est.nodos}, aristas ${est.aristas} (vado ${est.aristasVado}; vados declarados ${vados.length}), componentes ${est.componentes}`);
console.log(`componente mayor ${est.nodosMayor} nodos = ${(100 * est.fraccionMayor).toFixed(2)} %`);
console.log(`siguientes componentes: ${[...comp.tamanos].sort((a, b) => b - a).slice(1, 8).join(', ')}`);

const mundo = generarMundoEstrategico({ ...regional, conexion: regional.conexion });
function fuera(nombre: string, puntos: Point[]) {
  let sinNodo = 0, otraComp = 0;
  for (const p of puntos) {
    const n = nodoMasCercano(grafo, p);
    if (!n) sinNodo++; else if (comp.componenteDe[n.id] !== est.mayor) otraComp++;
  }
  console.log(`${nombre}: ${puntos.length} — sin nodo a un espaciado ${sinNodo}, en otra componente ${otraComp}`);
}
fuera('campamentos', mundo.campamentos.map(c => c.posicion));
fuera('nodos de recurso', mundo.nodos.map(n => n.posicion));
fuera('parcelas de bosque', mundo.parcelas.map(q => q.centro));
