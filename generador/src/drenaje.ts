import type { LagoZona, Point, RioZona } from './tipos';
import { distancia, distanciaASegmento, type Limites } from './geometria';
import { evaluarElevacion, evaluarTerreno, type CampoElevacion } from './elevacion';
import type { RandomFn } from './rng';

/** La malla solo decide drenaje; la superficie pública continúa siendo continua. */
const VECINOS: readonly [number, number][] = [
  [-1, -1], [0, -1], [1, -1], [-1, 0], [1, 0], [-1, 1], [0, 1], [1, 1],
];
interface Celda { punto: Point; altura: number; agua: boolean; montana: boolean; siguiente: number; acumulacion: number }

function construirMalla(campo: CampoElevacion, limites: Limites, LADO: number): Celda[] {
  const celdas: Celda[] = [];
  for (let y = 0; y < LADO; y++) for (let x = 0; x < LADO; x++) {
    const punto = { x: (x + 0.5) * limites.ancho / LADO, y: (y + 0.5) * limites.alto / LADO };
    const altura = evaluarElevacion(campo, punto);
    const terreno = evaluarTerreno(campo, punto, altura);
    celdas.push({ punto, altura, agua: terreno === 'agua', montana: terreno === 'montana' || terreno === 'cima',
      siguiente: -1, acumulacion: 1 });
  }
  // D8: cada celda descarga en el vecino estrictamente más bajo. Un mínimo
  // verdadero queda identificado como cuenca cerrada, sin inventar una salida.
  for (let i = 0; i < celdas.length; i++) {
    const celda = celdas[i]!;
    if (celda.agua) continue;
    const x = i % LADO, y = Math.floor(i / LADO);
    let mejor = -1, descenso = 1e-8;
    for (const [dx, dy] of VECINOS) {
      const nx = x + dx, ny = y + dy;
      if (nx < 0 || nx >= LADO || ny < 0 || ny >= LADO) continue;
      const j = ny * LADO + nx, otra = celdas[j]!;
      const pendiente = (celda.altura - otra.altura) / distancia(celda.punto, otra.punto);
      if (pendiente > descenso) { descenso = pendiente; mejor = j; }
    }
    celda.siguiente = mejor;
  }
  // Acumulación real de área aguas arriba: las celdas más altas entregan su
  // aportación a las inferiores, con desempate estable por índice.
  const orden = celdas.map((_, i) => i).sort((a, b) => celdas[b]!.altura - celdas[a]!.altura || a - b);
  for (const i of orden) {
    const j = celdas[i]!.siguiente;
    if (j >= 0) celdas[j]!.acumulacion += celdas[i]!.acumulacion;
  }
  return celdas;
}

function trazar(celdas: Celda[], origen: number, LADO: number): { indices: number[]; fin: RioZona['fin'] } {
  const indices = [origen];
  let actual = origen;
  for (let paso = 0; paso < LADO * LADO; paso++) {
    const celda = celdas[actual]!;
    if (celda.agua) return { indices, fin: 'agua' };
    const x = actual % LADO, y = Math.floor(actual / LADO);
    if (celda.siguiente < 0) return { indices, fin: x === 0 || y === 0 || x === LADO - 1 || y === LADO - 1 ? 'borde' : 'lago' };
    actual = celda.siguiente;
    indices.push(actual);
  }
  throw new Error('Drenaje D8 excede el número de celdas: posible ciclo');
}

/** Cauces por drenaje D8 y acumulación. Solo se publican cuencas con salida
 * física o lagos con área tributaria suficiente; un mínimo microscópico no es lago. */
export function generarRiosPorDrenaje(rng: RandomFn, limites: Limites, campo: CampoElevacion,
  opciones: { lado: number; cantidad: number; largoMinimo: number; radioLago: number; aceptarLago: (p: Point) => boolean; permitirAfluentes: boolean; espacioNacimientos: number }): { rios: RioZona[]; lagos: LagoZona[] } {
  const LADO = opciones.lado;
  const celdas = construirMalla(campo, limites, LADO);
  const candidatos = celdas.map((celda, i) => ({ i, celda, desempate: rng() }))
    .filter(({ celda }) => celda.montana && !celda.agua)
    .sort((a, b) => b.celda.altura - a.celda.altura || a.desempate - b.desempate);
  const rios: RioZona[] = [];
  const lagos: LagoZona[] = [];
  const nacimientos: Point[] = [];
  const sumideros = new Set<number>();
  const minimoLargo = opciones.largoMinimo;
  for (const { i, celda } of candidatos) {
    if (rios.length >= opciones.cantidad) break;
    if (nacimientos.some((p) => distancia(p, celda.punto) < opciones.espacioNacimientos)) continue;
    const traza = trazar(celdas, i, LADO);
    const final = traza.indices[traza.indices.length - 1]!;
    const terminal = celdas[final]!;
    let fin = traza.fin;
    const puntos = traza.indices.map((j) => celdas[j]!.punto);
    if (fin === 'borde') {
      const x = final % LADO, y = Math.floor(final / LADO);
      const salidas: Point[] = [];
      if (x === 0) salidas.push({ x: 0, y: terminal.punto.y });
      if (x === LADO - 1) salidas.push({ x: limites.ancho, y: terminal.punto.y });
      if (y === 0) salidas.push({ x: terminal.punto.x, y: 0 });
      if (y === LADO - 1) salidas.push({ x: terminal.punto.x, y: limites.alto });
      const salida = salidas.map((p) => ({ p, altura: evaluarElevacion(campo, p) }))
        .filter((v) => v.altura < terminal.altura - 1e-8).sort((a, b) => a.altura - b.altura)[0];
      if (salida) puntos.push(salida.p);
      else fin = 'lago';
    }
    if (fin === 'lago' && (terminal.acumulacion < 12 || (!opciones.permitirAfluentes && sumideros.has(final)))) continue;
    if (fin === 'lago' && !opciones.aceptarLago(terminal.punto)) continue;
    // Una depresión pegada al cauce autorado es tributaria de ese valle, no un
    // lago independiente que deba cortar la lámina del troncal.
    if (fin === 'lago' && campo.caucesTroncales?.some((cauce) =>
      cauce.puntos.slice(1).some((p, k) => distanciaASegmento(terminal.punto, cauce.puntos[k]!, p)
        < cauce.radio + Math.min(limites.ancho, limites.alto) * 0.014))) continue;
    const largo = puntos.reduce((s, p, k) => k ? s + distancia(puntos[k - 1]!, p) : s, 0);
    if (puntos.length < 2 || largo < minimoLargo) continue;
    nacimientos.push(celda.punto);
    if (fin === 'lago' && !sumideros.has(final)) {
      sumideros.add(final);
      lagos.push({ id: `lago-${rios.length}`, centro: terminal.punto,
        radio: opciones.radioLago,
        elevacionCentro: terminal.altura - 0.005 });
    }
    rios.push({ id: `rio-${rios.length}`, puntos, fin,
      terminaEnLago: fin === 'lago', navegable: false });
  }
  return { rios, lagos };
}
