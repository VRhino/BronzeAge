// Relieve de producción: relieve regional, hidrografía, costas, bosque y suelos de una seed, con la conexión terrestre comprobada.
import { crearCostasRegional } from './costasRegional';
import { evaluarElevacion, type CampoElevacion } from './elevacion';
import { crearCampoForestal } from './forestacionRegional';
import { crearRelieveRegional } from './regional';
import { anchoDeCaudal, generarHidrografiaRegional, type RioRegional } from './hidrografiaRegional';
import type { Point } from './tipos';
import { ELEVACION } from './config';
import { crearErosion, EROSION } from './erosion';
import { puntoEnLago } from './aguaRegional';
import { enPaso } from './relieveDetalle';
import { crearSuelosRegional } from './suelosRegional';
import { comprobarConexion, conexionPorCeldas } from './conexion';
import { muestrearCeldas } from './celdas';
import { FACTOR } from './campamentos';
import { crearDetalleRelieve } from './relieveDetalle';
import { crearRegionAlpina } from './regionAlpina';
import { potencia } from './potencia';

export function generarRelieve(seed: number, ancho: number, alto: number) {
  const config = { ancho, alto, seed };
  const regional = crearRelieveRegional(seed, config);
  const elevacion: CampoElevacion = { regional };
  let hidro = generarHidrografiaRegional(seed, elevacion);
  elevacion.lagos = hidro.lagos;
  let conexion = comprobarConexion(elevacion), factorLagos = 1;
  for (const factor of [0.5, 0.25, 0.1]) {
    if (conexion.conectadas === conexion.zonas) break;
    elevacion.lagos = [];
    hidro = generarHidrografiaRegional(seed, elevacion, factor);
    elevacion.lagos = hidro.lagos; factorLagos = factor;
    conexion = comprobarConexion(elevacion);
  }
  const { lagos } = hidro;
  let rios = hidro.rios;
  elevacion.costas = crearCostasRegional(seed, regional, p => evaluarElevacion(elevacion, p), rios);
  // El detalle va el último: ríos, lagos y costas se trazan sobre el relieve regional y el detalle respeta sus valles.
  const forestacion = crearCampoForestal(seed, rios);
  // Con el detalle, la conexión se mide en las celdas de 4 u del artefacto (la malla gruesa ya no ve los valles). Si
  // alguna región queda aislada, se repite con menos detalle, como con los lagos. La rejilla se devuelve para que nadie
  // vuelva a muestrear los mismos centros.
  const alpina = crearRegionAlpina(seed, regional, rios, lagos);
  let fuerzaDetalle = 1, celdas, final;
  for (const fuerza of [1, 0.8, 0.6, 0.4, 0]) {
    fuerzaDetalle = fuerza;
    elevacion.detalle = crearDetalleRelieve(seed, rios, regional.barreras, fuerza, alpina);
    celdas = muestrearCeldas({ elevacion, forestacion }, { ancho, alto }, 4 * FACTOR);
    final = conexionPorCeldas(regional, celdas);
    if (final.conectadas === final.zonas) break;
  }
  if (final!.conectadas !== final!.zonas) throw new Error(`Seed ${seed}: solo ${final!.conectadas}/${final!.zonas} zonas tienen conexión terrestre`);
  // La erosión por gotas, sobre todo lo anterior; lejos de los ríos, el agua y la costa. Si corta el paso por tierra, no se queda.
  elevacion.erosion = crearErosion(seed, celdas!, pesoDeErosion(elevacion, rios));
  const conErosion = muestrearCeldas({ elevacion, forestacion }, { ancho, alto }, 4 * FACTOR), conexionErosion = conexionPorCeldas(regional, conErosion);
  if (conexionErosion.conectadas === conexionErosion.zonas) { celdas = conErosion; final = conexionErosion; } else elevacion.erosion = undefined;
  // Las bocas, sobre el relieve final; los suelos (que miran la ribera) ya con los ríos definitivos.
  rios = rios.flatMap(r => (r.fin === 'agua' ? desembocadura(r, elevacion) : [r])).map(r => soloBajando(r, elevacion))
    .filter(r => r.puntos.length > 2 || evaluarElevacion({ ...elevacion, lagos: [] }, r.puntos[1]!) < evaluarElevacion({ ...elevacion, lagos: [] }, r.puntos[0]!));
  // una trenza cuya vuelta quedó fuera del río (su boca, recortada en la costa) ya no se apoya en él: fuera
  const clave = (p: Point) => `${p.x},${p.y}`, puntosDe = new Map(rios.map(r => [r.id, new Set(r.puntos.map(clave))] as const));
  rios = rios.filter(r => { const m = r.id.match(/^(.*)-trenza-\d+$/); if (!m) return true; const madre = puntosDe.get(m[1]!); return !!madre && madre.has(clave(r.puntos[0]!)) && madre.has(clave(r.puntos.at(-1)!)); });
  const suelos = crearSuelosRegional(seed, rios);
  return { config, elevacion, rios, lagos, suelos, forestacion,
    conexion: final!, factorLagos, fuerzaDetalle, celdas: celdas! };
}

/** La boca de los ríos que llegan al mar (experimento/rios, fase 4), en metros. El estuario ensancha el cauce en sus últimos `estuario`
 * m, hasta `ensanche` veces en costa tendida y `ensancheEmpinado` en costa empinada. Si la boca es tendida (pendiente final bajo
 * `pendienteDelta`) y el río ancho (`anchoDelta` m), se abre en delta: `brazos` brazos más, nacidos a `desde` m de la boca con
 * `angulo` grados a cada lado, que bajan al mar en línea recta a pasos de `paso` m; cada uno lleva `parte` del caudal. */
export const BOCA = { estuario: 300, ensanche: 3, ensancheEmpinado: 1.5, tramoPendiente: 600, pendienteDelta: 0.02, anchoDelta: 30,
  brazos: 2, desde: 600, angulo: 28, paso: 40, largoMaximo: 2500, parte: 0.35 };

/** Cuánto puede cambiar la erosión en cada punto (0-1): nada en los cauces (hasta `margenRio` m de su orilla), los lagos, el agua, la
 * tierra baja de la costa y los puertos de montaña (sus corredores deben seguir transitables), y entero a `transicion` m de ahí. */
function pesoDeErosion(elevacion: CampoElevacion, rios: readonly RioRegional[]): (p: Point, e: number) => number {
  const celda = 400, cubos = new Map<string, number[]>(), segs: number[] = [];
  const alcance = (Math.max(...rios.flatMap(r => r.anchos)) / 2) + EROSION.margenRio + EROSION.transicion;
  for (const r of rios) for (let i = 0; i + 1 < r.puntos.length; i++) {
    const a = r.puntos[i]!, b = r.puntos[i + 1]!, k = segs.length / 5;
    segs.push(a.x, a.y, b.x, b.y, Math.max(r.anchos[i]!, r.anchos[i + 1]!) / 2);
    for (let y = Math.floor((Math.min(a.y, b.y) - alcance) / celda); y <= Math.floor((Math.max(a.y, b.y) + alcance) / celda); y++)
      for (let x = Math.floor((Math.min(a.x, b.x) - alcance) / celda); x <= Math.floor((Math.max(a.x, b.x) + alcance) / celda); x++) {
        const c = `${x},${y}`; (cubos.get(c) ?? cubos.set(c, []).get(c)!).push(k);
      }
  }
  const suave = (t: number) => { const x = Math.min(1, Math.max(0, t)); return x * x * (3 - 2 * x); };
  const pasos = elevacion.regional.barreras.flatMap(b => b.pasos.flatMap(q => [q.centro.x, q.centro.y, q.tangente.x, q.tangente.y, q.radio, b.ancho]));
  return (p, e) => {
    if ((elevacion.lagos ?? []).some(l => puntoEnLago(l, p))) return 0;
    let fuera = Infinity;
    for (const k of cubos.get(`${Math.floor(p.x / celda)},${Math.floor(p.y / celda)}`) ?? []) {
      const ax = segs[5 * k]!, ay = segs[5 * k + 1]!, dx = segs[5 * k + 2]! - ax, dy = segs[5 * k + 3]! - ay;
      const t = Math.max(0, Math.min(1, ((p.x - ax) * dx + (p.y - ay) * dy) / (dx * dx + dy * dy || 1)));
      fuera = Math.min(fuera, Math.hypot(p.x - ax - dx * t, p.y - ay - dy * t) - segs[5 * k + 4]!);
    }
    return suave((fuera - EROSION.margenRio) / EROSION.transicion) * suave((e - ELEVACION.umbralAgua - 0.015) / 0.015) * (1 - enPaso(elevacion.regional.barreras.length ? pasos : [], p));
  };
}

/** El río, sin los puntos de dentro que no quedan por debajo del anterior en el relieve final (subidas de milímetros donde se cruzan los
 * valles de varios ríos): así baja siempre, punto a punto. Los nacimientos y las confluencias se quedan; un tramo de solo dos puntos
 * entre confluencias que no baja (unas decenas de metros) se quita entero. */
function soloBajando(rio: RioRegional, elevacion: CampoElevacion): RioRegional {
  const campo = { ...elevacion, lagos: [] }, n = rio.puntos.length, quedan = [0];
  let antes = evaluarElevacion(campo, rio.puntos[0]!);
  for (let i = 1; i < n - 1; i++) {
    const v = evaluarElevacion(campo, rio.puntos[i]!);
    if (v < antes) { quedan.push(i); antes = v; }
  }
  // el último punto se queda si es una confluencia (lo comparte otro tramo); si el río acaba en un lago, el mar o el borde, solo si baja
  const ultima = evaluarElevacion(campo, rio.puntos[n - 1]!);
  // antes de una confluencia, fuera los puntos que ya no quedan por encima de ella (los que caen en el cauce del río que lo recibe)
  if (rio.fin === 'limite') while (quedan.length > 1 && !(evaluarElevacion(campo, rio.puntos[quedan[quedan.length - 1]!]!) > ultima)) quedan.pop();
  if (rio.fin === 'limite' || ultima < antes || quedan.length < 2) quedan.push(n - 1);
  if (quedan.length === n) return rio;
  const de = <T,>(xs: T[]) => quedan.map(i => xs[i]!);
  return { ...rio, puntos: de(rio.puntos), caudal: de(rio.caudal), anchos: de(rio.anchos) };
}

/** La boca de un río que llega al mar, sobre el relieve final (con la costa): acaba en su primer punto de agua (lo de después es
 * estuario o mar) y, en la franja de la costa, pierde los puntos que la playa deja a la altura del anterior o por encima. Luego, el
 * estuario y, si toca, los brazos del delta. */
function desembocadura(rio: RioRegional, elevacion: CampoElevacion): RioRegional[] {
  const campo = { ...elevacion, lagos: [] }, e = (p: Point) => evaluarElevacion(campo, p), quedan = [0];
  let antes = e(rio.puntos[0]!);
  for (let i = 1; i < rio.puntos.length; i++) {
    const v = e(rio.puntos[i]!);
    if (v >= antes && v < ELEVACION.umbralCosta) continue;
    quedan.push(i); antes = v;
    if (v < ELEVACION.umbralAgua) break;
  }
  const de = <T,>(xs: T[]) => quedan.map(i => xs[i]!);
  const puntos = de(rio.puntos), caudal = de(rio.caudal), anchos = de(rio.anchos), n = puntos.length;
  if (n < 2) return [{ ...rio, puntos, caudal, anchos }];
  // lo que queda hasta la boca, desde cada punto
  const hasta = new Array<number>(n).fill(0);
  for (let i = n - 2; i >= 0; i--) hasta[i] = hasta[i + 1]! + Math.hypot(puntos[i + 1]!.x - puntos[i]!.x, puntos[i + 1]!.y - puntos[i]!.y);
  const k0 = Math.max(0, hasta.findIndex(d => d <= BOCA.tramoPendiente) - 1);
  const pendiente = Math.max(0, (e(puntos[k0]!) - e(puntos[n - 1]!)) * 1000 / (hasta[k0]! || 1));
  const tendida = pendiente < BOCA.pendienteDelta;
  const ensanche = tendida ? BOCA.ensanche : BOCA.ensancheEmpinado;
  for (let i = 0; i < n; i++) if (hasta[i]! < BOCA.estuario) anchos[i] = anchos[i]! * (1 + (ensanche - 1) * potencia(1 - hasta[i]! / BOCA.estuario, 2));
  const salida: RioRegional[] = [{ ...rio, puntos, caudal, anchos }];
  if (!tendida || anchoDeCaudal(caudal[n - 1]!) < BOCA.anchoDelta) return salida;
  // el delta: cada brazo nace en el punto a `desde` m de la boca y baja recto al mar; se queda si llega bajando siempre
  const k = Math.max(0, hasta.findIndex(d => d <= BOCA.desde) - 1), origen = puntos[k]!;
  const dx = puntos[n - 1]!.x - origen.x, dy = puntos[n - 1]!.y - origen.y, l = Math.hypot(dx, dy) || 1;
  for (let b = 0; b < BOCA.brazos; b++) {
    const giro = (b % 2 ? -1 : 1) * BOCA.angulo * (1 + Math.floor(b / 2) * 0.6) * Math.PI / 180;
    const ux = (dx * Math.cos(giro) - dy * Math.sin(giro)) / l, uy = (dx * Math.sin(giro) + dy * Math.cos(giro)) / l;
    const brazo: Point[] = [origen];
    let previo = e(origen), llega = false;
    for (let d = BOCA.paso; d <= BOCA.largoMaximo; d += BOCA.paso) {
      const p = { x: origen.x + ux * d, y: origen.y + uy * d }, v = e(p);
      if (v >= previo) { if (v < ELEVACION.umbralCosta) continue; break; }
      brazo.push(p); previo = v;
      if (v < ELEVACION.umbralAgua) { llega = true; break; }
    }
    if (!llega || brazo.length < 3) continue;
    const q = caudal[k]! * BOCA.parte;
    salida.push({ ...rio, id: `${rio.id}-brazo-${b}`, puntos: brazo, caudal: brazo.map(() => q), anchos: brazo.map(() => anchoDeCaudal(q)) });
  }
  return salida;
}
