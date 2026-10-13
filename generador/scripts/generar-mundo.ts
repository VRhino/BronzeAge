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

const r = medir('relieve', () => generarRegionalVisor(seed, ancho, alto));
const e = medir('campamentos, metales y parcelas', () => generarMundoEstrategico(r));
const recursos = medir('recursos', () => [...metalesARecursos(e.nodos), ...generarRecursosNoMetalicos(r, e.campamentos)]);
const apto = (p: Point) => evaluarTerreno(r.elevacion, p) !== 'agua' && esTransitable(r.elevacion, p) && r.conexion.tieneAcceso(p);
const { bandidos, alijos } = medir('bandidos y alijos', () => colocarBandidosYAlijos(seed, { ancho, alto }, e.campamentos, e.vetas, apto));

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
const grafo = medir('grafo de navegación', () => construirGrafo({ ancho, alto }, p => esTransitable(r.elevacion, p), p => costeEnPunto(r.elevacion, p), rios.flatMap(x => x.vados)));

const mundo: MundoGenerado = {
  seed, ancho, alto, generador: 'regional-17-costas-1+estrategico-1',
  campamentos: e.campamentos, recursos, vetas: e.vetas, masas: e.masas, parcelas: e.parcelas, bandidos, alijos, rios, grafo,
  muestras: {
    altura: p => evaluarElevacion(r.elevacion, p),
    terreno: p => evaluarTerreno(r.elevacion, p),
    fertilidad: p => evaluarSuelo(r.suelos, r.elevacion, p, evaluarForestal(r.forestacion, r.elevacion, p).densidad).fertilidad,
    humedad: p => humedadRegionalEn(r.elevacion.regional!, p),
    densidadBosque: p => evaluarForestal(r.forestacion, r.elevacion, p).densidad,
    transitable: p => esTransitable(r.elevacion, p),
  },
};
const salida = medir('artefacto', () => escribirArtefacto(mundo, carpeta));
console.log(`escrito ${salida.ruta}\nhash ${salida.hash}`);
