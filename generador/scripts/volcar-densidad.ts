// Vuelca el raster de densidad de bosque (celda de 4 estratégicas) para experimentar con las masas sin regenerar el relieve.
import { writeFileSync } from 'node:fs';
import { generarRegionalVisor } from './visor-worldgen.regional';
import { evaluarForestal } from '../src/forestacionRegional';
import { FACTOR } from '../src/campamentos';

const seed = Number(process.argv[2] ?? 42), ANCHO = 2667 * FACTOR, ALTO = 2000 * FACTOR, lado = 4 * FACTOR;
const r = generarRegionalVisor(seed, ANCHO, ALTO);
const nx = Math.ceil(ANCHO / lado), ny = Math.ceil(ALTO / lado), d = new Float32Array(nx * ny);
for (let i = 0; i < d.length; i++) d[i] = evaluarForestal(r.forestacion, r.elevacion, { x: ((i % nx) + 0.5) * lado, y: (Math.floor(i / nx) + 0.5) * lado }).densidad;
writeFileSync(`salida/densidad-${seed}.bin`, Buffer.from(d.buffer));
console.log(`densidad ${nx}x${ny}`);
