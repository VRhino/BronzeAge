import type { LagoZona, Point, RioZona } from './tipos';
import { evaluarElevacion, type CampoElevacion, type CauceTroncal } from './elevacion';
import { generarRiosPorDrenaje } from './drenaje';
import { createRng, derivarSeed } from './rng';
import { puntoEnLago } from './aguaRegional';

export interface RioRegional extends RioZona { orden: number }
interface Vertice { p: Point; siguiente?: string; entradas: string[]; orden: number; fin: RioZona['fin'] }
const clave = (p: Point) => `${p.x},${p.y}`;

/** Suavizado local con retroceso: una esquina inválida no cancela el resto.
 * Conserva extremos/confluencias y comprueba cada segmento modificado cada 25 m. */
export function suavizarCauceRegional(puntos: Point[], altura: (p: Point) => number): Point[] {
  const salida = puntos.map(p => ({ ...p }));
  const baja = (a: Point, b: Point) => {
    const pasos = Math.max(4, Math.ceil(Math.hypot(b.x - a.x, b.y - a.y) / 25));
    let anterior = altura(a);
    if (altura(b) >= anterior) return false;
    for (let j = 1; j <= pasos; j++) {
      const h = altura({ x: a.x + (b.x - a.x) * j / pasos, y: a.y + (b.y - a.y) * j / pasos });
      if (h > anterior + 1e-7) return false;
      anterior = h;
    }
    return true;
  };
  for (let pasada = 0; pasada < 3; pasada++) for (let k = 1; k < salida.length - 1; k++) {
    const i = pasada % 2 ? salida.length - 1 - k : k;
    const a = salida[i - 1]!, p = salida[i]!, b = salida[i + 1]!;
    for (const peso of [0.5, 0.25, 0.125]) {
      const q = { x: p.x + ((a.x + b.x) / 2 - p.x) * peso, y: p.y + ((a.y + b.y) / 2 - p.y) * peso };
      if (baja(a, q) && baja(q, b)) { salida[i] = q; break; }
    }
  }
  return salida;
}

function vallesPrincipales(campo: CampoElevacion): CauceTroncal[] {
  const { ancho, alto } = campo.regional.limites;
  const fase = campo.regional.fase;
  const rutas = [
    [[0.54, 0.995], [0.535, 0.94], [0.56, 0.88], [0.54, 0.82], [0.55, 0.76], [0.53, 0.68]],
    [[0.955, 0.36], [0.90, 0.46], [0.93, 0.55], [0.91, 0.67], [0.94, 0.78], [0.90, 0.87], [0.92, 1]],
  ];
  return rutas.map((ruta, id) => {
    const puntos: Point[] = [];
    for (let i = 0; i < ruta.length - 1; i++) {
      const a = ruta[Math.max(0, i - 1)]!, b = ruta[i]!, c = ruta[i + 1]!, d = ruta[Math.min(ruta.length - 1, i + 2)]!;
      for (let j = 0; j < 8; j++) {
        const t = j / 8, eje = (k: number) => 0.5 * (2 * b[k]! + (-a[k]! + c[k]!) * t
          + (2 * a[k]! - 5 * b[k]! + 4 * c[k]! - d[k]!) * t * t + (-a[k]! + 3 * b[k]! - 3 * c[k]! + d[k]!) * t * t * t);
        puntos.push({ x: (eje(0) + Math.sin((i + t) * 2 + fase) * 0.003 * Math.sin(Math.PI * (i + t) / (ruta.length - 1))) * ancho, y: eje(1) * alto });
      }
    }
    const ultimo = ruta[ruta.length - 1]!; puntos.push({ x: ultimo[0]! * ancho, y: ultimo[1]! * alto });
    return { puntos, cotas: puntos.map((_, i) => (id === 0 ? 0.447 : 0.493) - i / (puntos.length - 1) * (id === 0 ? 0.08 : 0.066)),
      radio: (id === 0 ? 750 : 900) * Math.min(ancho, alto) / 24000,
      caja: { minX: Math.min(...puntos.map(p => p.x)), maxX: Math.max(...puntos.map(p => p.x)), minY: Math.min(...puntos.map(p => p.y)), maxY: Math.max(...puntos.map(p => p.y)) } };
  });
}

/** Orilla por intersección de la cota de agua con el terreno. Se busca una cuenca
 * cerrada radialmente; si no cabe o amenaza un puerto, se reduce la lámina.
 * El modelo no simula todavía desbordamientos ni lagos con emisario. */
function orilla(lago: LagoZona, campo: CampoElevacion, escala: number, aceptar: (p: Point) => boolean, factorLagos: number): LagoZona | undefined {
  const altura = evaluarElevacion(campo, lago.centro);
  for (const subida of [0.006, 0.003, 0.0015, 0.0006]) {
    const nivel = altura + subida * factorLagos, puntos: Point[] = [];
    let valido = true, radio = 0;
    for (let k = 0; k < 64; k++) {
      const angulo = k * Math.PI * 2 / 64;
      const p = (r: number) => ({ x: lago.centro.x + Math.cos(angulo) * r, y: lago.centro.y + Math.sin(angulo) * r });
      let r = 25 * escala;
      while (r < 1400 * escala && evaluarElevacion(campo, p(r)) < nivel) r += 25 * escala;
      if (r >= 1400 * escala) { valido = false; break; }
      let a = Math.max(0, r - 25 * escala), b = r;
      for (let j = 0; j < 10; j++) { const m = (a + b) / 2; if (evaluarElevacion(campo, p(m)) < nivel) a = m; else b = m; }
      const borde = p((a + b) / 2);
      const { ancho, alto } = campo.regional.limites;
      if (!aceptar(borde) || borde.x <= 0 || borde.y <= 0 || borde.x >= ancho || borde.y >= alto) { valido = false; break; }
      puntos.push(borde); radio = Math.max(radio, b);
    }
    if (valido && radio > 30 * escala) return { ...lago, radio: radio + 1, contorno: puntos, elevacionCentro: nivel };
  }
  return undefined;
}

/** Red dirigida: cada tramo compartido se publica una sola vez. Las confluencias
 * son extremos comunes, con referencias explícitas aguas abajo y orden Strahler. */
export function generarHidrografiaRegional(seed: number, campo: CampoElevacion, factorLagos = 1) {
  const limites = campo.regional.limites, escala = Math.min(limites.ancho, limites.alto) / 24000;
  campo.caucesTroncales = vallesPrincipales(campo);
  const aceptar = (p: Point) => campo.regional.barreras.every(b => b.pasos.every(paso => {
    const dx = p.x - paso.centro.x, dy = p.y - paso.centro.y;
    return Math.abs(dx * paso.tangente.x + dy * paso.tangente.y) > 600 * escala
      || Math.abs(-dx * paso.tangente.y + dy * paso.tangente.x) > b.ancho * 2.8 + 300 * escala;
  }));
  const base = generarRiosPorDrenaje(createRng(derivarSeed(seed, 'regional-drenaje')), limites, campo, {
    lado: Math.max(80, Math.min(256, Math.ceil(limites.ancho / 100))), cantidad: 65,
    largoMinimo: 1200 * escala, radioLago: 220 * escala, aceptarLago: aceptar, permitirAfluentes: true, espacioNacimientos: 1100 * escala,
  });
  const principales: RioZona[] = campo.caucesTroncales.map((c, i) => {
    let puntos = c.puntos;
    const llegada = puntos.findIndex(p => evaluarElevacion(campo, p) < 0.375);
    if (llegada > 0) {
      const a = puntos[llegada - 1]!, b = puntos[llegada]!;
      let lo = 0, hi = 1;
      for (let k = 0; k < 16; k++) {
        const t = (lo + hi) / 2, p = { x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t };
        if (evaluarElevacion(campo, p) < 0.375) hi = t; else lo = t;
      }
      puntos = [...puntos.slice(0, llegada), { x: a.x + (b.x - a.x) * hi, y: a.y + (b.y - a.y) * hi }];
    }
    return { id: `principal-${i}`, puntos, fin: llegada >= 0 ? 'agua' : 'borde', terminaEnLago: false, navegable: false };
  });
  for (const rio of base.rios) {
    let unido = false;
    for (let k = 0; k < rio.puntos.length && !unido; k++) for (let i = 0; i < principales.length && !unido; i++) {
      const principal = principales[i]!, radio = campo.caucesTroncales[i]!.radio * 0.4;
      for (let j = 1; j < principal.puntos.length; j++) {
        const p = rio.puntos[k]!, q = principal.puntos[j]!;
        if (Math.hypot(p.x - q.x, p.y - q.y) < radio && evaluarElevacion(campo, q) < evaluarElevacion(campo, p)) {
          rio.puntos = [...rio.puntos.slice(0, k + 1), ...principal.puntos.slice(j)]; rio.fin = principal.fin; rio.terminaEnLago = false; unido = true; break;
        }
      }
    }
  }
  base.rios.push(...principales);
  const lagos: LagoZona[] = [];
  for (const candidato of base.lagos) {
    const lago = orilla(candidato, campo, escala, aceptar, factorLagos);
    if (lago && !lagos.some(l => Math.hypot(l.centro.x - lago.centro.x, l.centro.y - lago.centro.y) < l.radio + lago.radio)) lagos.push(lago);
  }
  const vertices = new Map<string, Vertice>();
  for (const rio of base.rios) {
    // No publicar un cauce que acaba en un mínimo sin lámina aceptada.
    if (rio.fin === 'lago' && !lagos.some(l => puntoEnLago(l, rio.puntos[rio.puntos.length - 1]!))) continue;
    for (let i = 0; i < rio.puntos.length; i++) {
      const p = rio.puntos[i]!, id = clave(p);
      if (!vertices.has(id)) vertices.set(id, { p, entradas: [], orden: 1, fin: rio.fin });
      const v = vertices.get(id)!;
      if (i < rio.puntos.length - 1) v.siguiente = clave(rio.puntos[i + 1]!);
    }
  }
  for (const [id, v] of vertices) if (v.siguiente) vertices.get(v.siguiente)!.entradas.push(id);
  const ordenados = [...vertices.values()].sort((a, b) => evaluarElevacion(campo, b.p) - evaluarElevacion(campo, a.p));
  for (const v of ordenados) {
    const ordenes = v.entradas.map(id => vertices.get(id)!.orden).sort((a, b) => b - a);
    const troncal = principales.some(r => r.puntos[0] === v.p);
    v.orden = Math.max(troncal ? 2 : 1, !ordenes.length ? 1 : ordenes[0]! + Number(ordenes.length > 1 && ordenes[0] === ordenes[1]));
  }
  const rios: RioRegional[] = [];
  for (const v of vertices.values()) {
    if (!v.siguiente || v.entradas.length === 1) continue;
    let actual = v;
    const puntos = [v.p];
    while (actual.siguiente) {
      actual = vertices.get(actual.siguiente)!; puntos.push(actual.p);
      if (actual.entradas.length !== 1) break;
    }
    const ps = suavizarCauceRegional(puntos, p => evaluarElevacion(campo, p));
    rios.push({ id: `tramo-${rios.length}`, puntos: ps, fin: actual.siguiente ? 'limite' : actual.fin,
      terminaEnLago: !actual.siguiente && actual.fin === 'lago', navegable: false, orden: v.orden });
  }
  return { rios, lagos };
}
