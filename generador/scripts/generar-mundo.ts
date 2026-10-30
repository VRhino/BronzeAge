// Genera el mundo completo y escribe el artefacto (`mundo-<hash>.bin`) en la carpeta dada.
// Uso: npx tsx scripts/generar-mundo.ts [seed] [carpeta]    (algo más de un minuto)
import { generarRelieve } from '../src/relieve';
import { generarMundoEstrategico } from '../src/mundoEstrategico';
import { metalesARecursos, generarRecursosNoMetalicos } from '../src/recursos';
import { colocarBandidosYAlijos } from '../src/contenidoVivo';
import { componentes, construirGrafo, estadisticasGrafo, nodoMasCercano } from '../src/navegacion';
import { bloquearRios, colocarVados, PARAMETROS_VADOS } from '../src/vados';
import { escribirArtefacto } from '../src/artefacto';
import { costeEnPunto, esTransitable } from '../src/costeMovimiento';
import { evaluarElevacion, evaluarTerreno } from '../src/elevacion';
import { densidadForestal } from '../src/forestacionRegional';
import { fertilidadSuelo } from '../src/suelosRegional';
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
const r = medir('relieve', () => generarRelieve(seed, ancho, alto));
const transitableCelda = (p: Point): boolean => {
  const cx = Math.floor(p.x / lado), cy = Math.floor(p.y / lado);
  if (cx < 0 || cy < 0 || cx >= nx || cy >= ny) return false;
  const i = cy * nx + cx;
  return bloqueoRios[i] === 0 && celdas.transitable[i] === 1;
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
// Los ríos cortan el paso salvo en los vados: se trazan como barrera en la misma rejilla.
const vadosPorRio = colocarVados(r.rios);
const bloqueoRios = bloquearRios({ ancho, alto }, r.rios, [...vadosPorRio.values()].flat(), lado);
// Parcelas, transitabilidad y artefacto muestrean el relieve en los mismos centros de celda de 48 unidades: los evaluó el
// relieve una sola vez (también mide con ellos la conexión).
const celdas = r.celdas;
if (celdas.lado !== lado || celdas.columnas !== nx || celdas.filas !== ny) throw new Error('la rejilla de celdas del relieve no es la del artefacto');
const densidadEn = (p: Point): number => { const i = celdas.indice(p); return i >= 0 ? celdas.densidad[i]! : densidadForestal(r.forestacion, r.elevacion, p); };
const e = medir('campamentos, metales y parcelas', () => generarMundoEstrategico(r, densidadEn));
// Desde el 2026-10-27 el mundo no tiene nodos de ganado: el ganado sale de los rebaños de las aldeas agrícolas (mecánicas 50 y 51).
const recursos = medir('recursos', () => [...metalesARecursos(e.nodos), ...generarRecursosNoMetalicos(r, e.campamentos, e.nodos.map(n => n.posicion))]
  .map(x => ({ ...x, posicion: aCeldaTransitable(x.posicion) })));
for (const c of e.campamentos) c.posicion = aCeldaTransitable(c.posicion);
const apto = (p: Point) => evaluarTerreno(r.elevacion, p) !== 'agua' && esTransitable(r.elevacion, p) && r.conexion.tieneAcceso(p);
const colocados = medir('bandidos y alijos', () => colocarBandidosYAlijos(seed, { ancho, alto }, e.campamentos, e.vetas, apto));
const bandidos = colocados.bandidos.map(b => ({ ...b, posicion: aCeldaTransitable(b.posicion) }));
const alijos = colocados.alijos.map(a => ({ ...a, posicion: aCeldaTransitable(a.posicion) }));

const rios: RioMundo[] = r.rios.map(rio => ({ id: rio.id, puntos: rio.puntos, terminaEnLago: rio.terminaEnLago, navegable: rio.navegable, vados: vadosPorRio.get(rio.id) ?? [] }));
// Coste de movimiento por celda, una sola vez: lo usan el grafo (sus nodos son los centros de las celdas) y la capa `coste`.
const costeCelda = medir('coste por celda', () => {
  const c = new Float64Array(nx * ny);
  for (let i = 0; i < c.length; i++) {
    const p = { x: ((i % nx) + 0.5) * lado, y: (Math.floor(i / nx) + 0.5) * lado };
    if (transitableCelda(p)) c[i] = costeEnPunto(r.elevacion, p, celdas.altura[i]);
  }
  return c;
});
const costeEn = (p: Point): number => { const i = celdas.indice(p); return i >= 0 ? costeCelda[i]! : transitableCelda(p) ? costeEnPunto(r.elevacion, p) : 0; };
const grafo = medir('grafo de navegación', () => construirGrafo({ ancho, alto }, transitableCelda, costeEn, rios.flatMap(x => x.vados),
  { radioVado: PARAMETROS_VADOS.radioHueco, espaciado: lado, costePorExtremos: true }));

const comp = componentes(grafo), est = estadisticasGrafo(grafo, comp);
const fuera = (puntos: Point[]) => puntos.filter(p => { const n = nodoMasCercano(grafo, p); return !n || comp.componenteDe[n.id] !== est.mayor; }).length;
console.log(`grafo: ${est.nodos} nodos, ${est.aristas} aristas (${est.aristasVado} de vado), ${est.componentes} componentes, mayor ${(100 * est.fraccionMayor).toFixed(2)} %; ` +
  `fuera de la mayor: campamentos ${fuera(e.campamentos.map(c => c.posicion))}/${e.campamentos.length}, recursos ${fuera(recursos.map(x => x.posicion))}/${recursos.length}, vados ${[...vadosPorRio.values()].flat().length}`);

const mundo: MundoGenerado = {
  seed, ancho, alto, generador: 'regional-17-costas-1+estrategico-2',
  campamentos: e.campamentos, recursos, vetas: e.vetas, masas: e.masas, parcelas: e.parcelas, bandidos, alijos, rios, grafo,
  muestras: {
    altura: p => { const i = celdas.indice(p); return i >= 0 ? celdas.altura[i]! : evaluarElevacion(r.elevacion, p); },
    terreno: p => { const i = celdas.indice(p); return evaluarTerreno(r.elevacion, p, i >= 0 ? celdas.altura[i]! : undefined); },
    fertilidad: p => {
      const i = celdas.indice(p);
      if (i < 0) return fertilidadSuelo(r.suelos, r.elevacion, p);
      return fertilidadSuelo(r.suelos, r.elevacion, p,
        { altura: celdas.altura[i]!, pendiente: celdas.pendiente[i]!, humedad: celdas.humedad[i]! });
    },
    humedad: p => { const i = celdas.indice(p); return i >= 0 ? celdas.humedad[i]! : humedadRegionalEn(r.elevacion.regional, p); },
    densidadBosque: densidadEn,
    transitable: transitableCelda,
    coste: costeEn,
  },
};
const salida = medir('artefacto', () => escribirArtefacto(mundo, carpeta));
console.log(`escrito ${salida.ruta}\nhash ${salida.hash}`);
