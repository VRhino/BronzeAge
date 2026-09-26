// Compara la consulta de fundación con su cálculo directo sobre un checkpoint real.
// npx tsx scripts/bench-fundacion-zonas.ts <checkpoint.json> [barridos=100]
import { readFileSync } from 'node:fs';
import { performance } from 'node:perf_hooks';
import type { Asentamiento, Point } from '../src/domain/types';
import { computeTodasLasZonas, pointInPolygon, posicionLibreParaFundar } from '../src/engine/zones';

const ruta = process.argv[2];
const barridos = Number(process.argv[3] ?? 100);
if (!ruta || !Number.isSafeInteger(barridos) || barridos <= 0) throw new Error('Indica checkpoint.json y barridos > 0');
const cp = JSON.parse(readFileSync(ruta, 'utf8')) as { estado: { asentamientos: Asentamiento[] } };
const asentamientos = cp.estado.asentamientos;
const origen = asentamientos[0]?.posicion;
if (!origen) throw new Error('El checkpoint debe contener al menos un asentamiento');
const puntos: Point[] = [];
for (let radio = 150; radio <= 600; radio += 150) {
  for (let angulo = 0; angulo < 360; angulo += 20) {
    const rad = angulo * Math.PI / 180;
    puntos.push({ x: origen.x + Math.cos(rad) * radio, y: origen.y + Math.sin(rad) * radio });
  }
}
const directo = (p: Point, plazas: Asentamiento[]) => computeTodasLasZonas(plazas).every((z) => !pointInPolygon(p, z.poligono));
// Calentamiento y comparación punto a punto antes de medir.
let libresPorBarrido = 0;
for (const p of puntos) {
  const libre = directo(p, asentamientos);
  if (libre !== posicionLibreParaFundar(p, asentamientos)) throw new Error('Resultado distinto');
  if (libre) libresPorBarrido++;
}
function medir(consulta: typeof directo): number {
  let libres = 0;
  const inicio = performance.now();
  for (let n = 0; n < barridos; n++) {
    // El array y las entidades cambian de identidad entre ticks aunque la geometría siga igual.
    const plazas = asentamientos.map((a) => ({ ...a, posicion: { ...a.posicion } }));
    for (const p of puntos) if (consulta(p, plazas)) libres++;
  }
  const ms = performance.now() - inicio;
  if (libres !== libresPorBarrido * barridos) throw new Error('Resultado distinto durante la medición');
  return ms;
}
const antes: number[] = [];
const despues: number[] = [];
for (let i = 0; i < 3; i++) {
  // Alternar el orden reduce el sesgo de calentamiento y de carga de la máquina.
  if (i % 2 === 0) { antes.push(medir(directo)); despues.push(medir(posicionLibreParaFundar)); }
  else { despues.push(medir(posicionLibreParaFundar)); antes.push(medir(directo)); }
}
const mediana = (xs: number[]) => [...xs].sort((a, b) => a - b)[1]!;
console.log(JSON.stringify({
  asentamientos: asentamientos.length,
  consultasPorRepeticion: barridos * puntos.length,
  directoMs: antes,
  cacheMs: despues,
  aceleracionMediana: mediana(antes) / mediana(despues),
}, null, 2));
