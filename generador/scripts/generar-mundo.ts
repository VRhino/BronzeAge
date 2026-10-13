// Genera el mundo completo y escribe el artefacto (`mundo-<hash>.bin`) en la carpeta dada.
// Uso: npx tsx scripts/generar-mundo.ts [seed] [carpeta]    (tarda varios minutos: relieve ~2 min y grafo ~5 min)
import { generarRegionalVisor } from './visor-worldgen.regional';
import { generarMundoEstrategico } from '../src/mundoEstrategico';
import { metalesARecursos, generarRecursosNoMetalicos } from '../src/recursos';
import { colocarBandidosYAlijos } from '../src/contenidoVivo';
import { construirGrafo } from '../src/navegacion';
import { escribirArtefacto } from '../src/artefacto';
import { costeEnPunto, esTransitable } from '../src/costeMovimiento';
import { evaluarElevacion, evaluarTerreno } from '../src/elevacion';
import { evaluarForestal } from '../src/forestacionRegional';
import { evaluarSuelo } from '../src/suelosRegional';
import { humedadRegionalEn } from '../src/regional';
import { FACTOR } from '../src/campamentos';
import type { MundoGenerado, RioMundo } from '../src/mundoTipos';
import type { Point } from '../src/tipos';

const seed = Number(process.argv[2] ?? 42), carpeta = process.argv[3] ?? 'salida';
const ancho = 2667 * FACTOR, alto = 2000 * FACTOR;
const medir = <T>(nombre: string, f: () => T): T => { const t = Date.now(); const r = f(); console.log(`${nombre}: ${((Date.now() - t) / 1000).toFixed(1)} s`); return r; };

// La transitabilidad del artefacto es el raster de celdas de 4 unidades estratégicas (se muestrea el centro). Todo lo demás
// —grafo, recursos, campamentos— se comprueba contra esa misma rejilla para que el servidor lo valide sin discrepancias.
const lado = 4 * FACTOR, nx = Math.ceil(ancho / lado), ny = Math.ceil(alto / lado);
const celdaT = new Uint8Array(nx * ny).fill(2);
const centroCelda = (i: number): Point => ({ x: ((i % nx) + 0.5) * lado, y: (Math.floor(i / nx) + 0.5) * lado });
const r = medir('relieve', () => generarRegionalVisor(seed, ancho, alto));
const transitableCelda = (p: Point): boolean => {
  const cx = Math.floor(p.x / lado), cy = Math.floor(p.y / lado);
  if (cx < 0 || cy < 0 || cx >= nx || cy >= ny) return false;
  const i = cy * nx + cx;
  if (celdaT[i] === 2) celdaT[i] = esTransitable(r.elevacion, centroCelda(i)) ? 1 : 0;
  return celdaT[i] === 1;
};
/** Si la celda de `p` no es transitable, lo lleva al centro de la celda transitable más cercana (hasta 6 celdas). */
const aCeldaTransitable = (p: Point): Point => {
  if (transitableCelda(p)) return p;
  const cx = Math.floor(p.x / lado), cy = Math.floor(p.y / lado);
  let mejor: Point | undefined, d = Infinity;
  for (let dy = -6; dy <= 6; dy++) for (let dx = -6; dx <= 6; dx++) {
    const q = { x: (cx + dx + 0.5) * lado, y: (cy + dy + 0.5) * lado };
    const dd = Math.hypot(q.x - p.x, q.y - p.y);
    if (dd < d && transitableCelda(q)) { d = dd; mejor = q; }
  }
  if (!mejor) throw new Error(`Sin celda transitable cerca de (${p.x}, ${p.y})`);
  return mejor;
};
const e = medir('campamentos, metales y parcelas', () => generarMundoEstrategico(r));
const recursos = medir('recursos', () => [...metalesARecursos(e.nodos), ...generarRecursosNoMetalicos(r, e.campamentos, e.nodos.map(n => n.posicion))]
  .map(x => ({ ...x, posicion: aCeldaTransitable(x.posicion) })));
for (const c of e.campamentos) c.posicion = aCeldaTransitable(c.posicion);
const apto = (p: Point) => evaluarTerreno(r.elevacion, p) !== 'agua' && esTransitable(r.elevacion, p) && r.conexion.tieneAcceso(p);
const colocados = medir('bandidos y alijos', () => colocarBandidosYAlijos(seed, { ancho, alto }, e.campamentos, e.vetas, apto));
const bandidos = colocados.bandidos.map(b => ({ ...b, posicion: aCeldaTransitable(b.posicion) }));
const alijos = colocados.alijos.map(a => ({ ...a, posicion: aCeldaTransitable(a.posicion) }));

// Vados provisionales: un punto del cauce cada ~100 estratégicas de río (los ríos no bloquean `esTransitable` en este generador).
const rios: RioMundo[] = r.rios.map(rio => {
  const vados: Point[] = [];
  let acumulado = 0;
  for (let i = 1; i < rio.puntos.length; i++) {
    acumulado += Math.hypot(rio.puntos[i]!.x - rio.puntos[i - 1]!.x, rio.puntos[i]!.y - rio.puntos[i - 1]!.y);
    if (acumulado >= 100 * FACTOR) { vados.push(rio.puntos[i]!); acumulado = 0; }
  }
  return { id: rio.id, puntos: rio.puntos, terminaEnLago: rio.terminaEnLago, navegable: rio.navegable, vados };
});
const grafo = medir('grafo de navegación', () => construirGrafo({ ancho, alto }, transitableCelda, p => costeEnPunto(r.elevacion, p), rios.flatMap(x => x.vados)));

const mundo: MundoGenerado = {
  seed, ancho, alto, generador: 'regional-17-costas-1+estrategico-1',
  campamentos: e.campamentos, recursos, vetas: e.vetas, masas: e.masas, parcelas: e.parcelas, bandidos, alijos, rios, grafo,
  muestras: {
    altura: p => evaluarElevacion(r.elevacion, p),
    terreno: p => evaluarTerreno(r.elevacion, p),
    fertilidad: p => evaluarSuelo(r.suelos, r.elevacion, p, evaluarForestal(r.forestacion, r.elevacion, p).densidad).fertilidad,
    humedad: p => humedadRegionalEn(r.elevacion.regional!, p),
    densidadBosque: p => evaluarForestal(r.forestacion, r.elevacion, p).densidad,
    transitable: transitableCelda,
  },
};
const salida = medir('artefacto', () => escribirArtefacto(mundo, carpeta));
console.log(`escrito ${salida.ruta}\nhash ${salida.hash}`);
