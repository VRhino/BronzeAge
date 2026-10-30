import type { Point } from './tipos';
import type { Limites } from './geometria';
import { createRng, derivarSeed } from './rng';

export interface AreaRegional {
  id: number; centro: Point; radio: number; altura: number; humedad: number;
  ondulacion: number; nombre: string;
}
export interface PasoRegional { centro: Point; tangente: Point; distancia: number; radio: number }
interface Tramo { a: Point; b: Point; longitud: number; inicio: number }
export type MorfologiaRegional = 'macizos' | 'cordillera' | 'longitudinal' | 'escarpe' | 'meseta';
export interface RamalRegional { puntos: Point[]; ancho: number; altura: number; }
export interface BarreraRegional {
  cumbres?: { centro: Point; radio: number; altura: number; fase: number;
    eje?: Point; longitud?: number; cimaPlana?: number; sesgo?: number; diseccion?: number; }[];
  morfologia?: MorfologiaRegional; ramales?: RamalRegional[]; fase?: number;
  puntos: Point[]; tramos: Tramo[]; longitud: number; ancho: number; altura: number;
  pasos: PasoRegional[]; caja: { minX: number; maxX: number; minY: number; maxY: number };
}
export interface RelieveRegional {
  revision: 'regional-8'; limites: Limites; areas: AreaRegional[]; barreras: BarreraRegional[];
  fase: number; islas: { centro: Point; radio: number }[];
}
const suave = (x: number) => { const t = Math.max(0, Math.min(1, x)); return t * t * (3 - 2 * t); };

/** Positiva en tierra. La península occidental es una banda sinuosa que se estrecha
 * hacia el extremo, con dos bahías que muerden la costa; evita el óvalo de v3. */
export function distanciaCostaRegional(campo: RelieveRegional, p: Point): number {
  const { ancho, alto } = campo.limites, x = p.x / ancho, y = p.y / alto;
  const f = campo.fase;
  // Variación amplia de forma, ensenadas de 1–3 km y detalle menor de 400–1000 m.
  const norte = (0.17 + 0.012 * Math.sin(x * 24 + f) + 0.006 * Math.sin(x * 67 - f)
    + 0.0025 * Math.sin(x * 145 + f * 1.7) - y) * alto;
  const este = (x - (0.705 + 0.018 * Math.sin(y * 19 + f) + 0.009 * Math.sin(y * 41)
    + 0.004 * Math.sin(y * 103 - f))) * ancho;
  const sur = (y - (0.79 + 0.018 * Math.sin(x * 14 + f) + 0.005 * Math.sin(x * 45 - f))) * alto;
  const elipse = (cx: number, cy: number, rx: number, ry: number) =>
    (1 - Math.hypot((x - cx) / rx, (y - cy) / ry)) * Math.min(rx * ancho, ry * alto);
  const t = Math.max(0, Math.min(1, (y - 0.13) / 0.40));
  const eje = 0.215 + 0.020 * Math.sin(y * 16 + f) + 0.009 * Math.sin(y * 43 - f);
  const anchoPeninsula = 0.079 - 0.029 * t - 0.032 * suave((t - 0.76) / 0.24)
    + 0.006 * Math.sin(y * 72 + f);
  const peninsula = Math.min((anchoPeninsula - Math.abs(x - eje)) * ancho,
    (0.523 - y + 0.004 * Math.sin(x * 93 + f)) * alto);
  // El saliente septentrional conserva su interior, pero pierde el arco ovalado:
  // cabos desiguales, dos entrantes amplios y ensenadas menores. La deformación
  // se desvanece tierra adentro y no consume RNG de montañas, áreas ni islas.
  const campana = (centro: number, radio: number) => Math.exp(-(((x - centro) / radio) ** 2));
  const costaAnatolia = 0.031 * campana(0.405, 0.026) - 0.036 * campana(0.485, 0.035)
    + 0.029 * campana(0.565, 0.032) - 0.028 * campana(0.65, 0.024)
    + 0.009 * Math.sin(x * 107 + f) + 0.004 * Math.sin(x * 231 - f);
  const desplazamiento = costaAnatolia * suave((y - 0.16) / 0.10);
  const anatolia = (1 - Math.hypot((x - 0.56) / 0.27, (y - 0.22 - desplazamiento) / 0.15))
    * Math.min(0.27 * ancho, 0.15 * alto);
  const delta = elipse(0.54, 0.79, 0.095, 0.095);
  const tierra = Math.max(norte, este, sur, peninsula, anatolia, delta);
  // Dos golfos profundos en tierra conectada. Sus cabeceras redondeadas no
  // atraviesan la península ni el arco continental.
  const golfoPeninsula = elipse(0.34, 0.395, 0.115, 0.065);
  const golfoMeridional = elipse(0.39, 0.755, 0.072, 0.087);
  return Math.min(tierra, -Math.max(golfoPeninsula, golfoMeridional));
}

/** Las áreas son influencias solapadas y de tamaños distintos; sus límites no levantan montañas. */
function pesoArea(campo: RelieveRegional, area: AreaRegional, p: Point): number {
  const escala = Math.min(campo.limites.ancho, campo.limites.alto) / 24000;
  const x = p.x + 330 * escala * Math.sin(p.y / (1800 * escala) + campo.fase);
  const y = p.y + 290 * escala * Math.sin(p.x / (2200 * escala) - campo.fase);
  const d2 = ((x - area.centro.x) ** 2 + (y - area.centro.y) ** 2) / (area.radio ** 2);
  return 1 / (0.03 + d2 ** 3);
}

export function humedadRegionalEn(campo: RelieveRegional, p: Point): number {
  let suma = 0, pesos = 0;
  for (const area of campo.areas) {
    const peso = pesoArea(campo, area, p);
    suma += area.humedad * peso; pesos += peso;
  }
  return suma / pesos;
}

/** Catmull–Rom muestreada: crestas sinuosas con continuidad en los puntos de control. */
function curva(controles: Point[]): Point[] {
  const puntos: Point[] = [];
  for (let i = 0; i < controles.length - 1; i++) {
    const a = controles[Math.max(0, i - 1)]!, b = controles[i]!;
    const c = controles[i + 1]!, d = controles[Math.min(controles.length - 1, i + 2)]!;
    for (let j = 0; j < 10; j++) {
      const t = j / 10, t2 = t * t, t3 = t2 * t;
      const eje = (k: 'x' | 'y') => 0.5 * ((2 * b[k]) + (-a[k] + c[k]) * t
        + (2 * a[k] - 5 * b[k] + 4 * c[k] - d[k]) * t2
        + (-a[k] + 3 * b[k] - 3 * c[k] + d[k]) * t3);
      puntos.push({ x: eje('x'), y: eje('y') });
    }
  }
  puntos.push(controles[controles.length - 1]!);
  return puntos;
}

export function crearRelieveRegional(seed: number, limites: Limites): RelieveRegional {
  const rng = createRng(derivarSeed(seed, 'regional-4'));
  const escala = Math.min(limites.ancho, limites.alto) / 24000;
  const fase = rng() * Math.PI * 2;
  const punto = (x: number, y: number, desvio = 0.025): Point => ({
    x: (x + (rng() - 0.5) * desvio) * limites.ancho,
    y: (y + (rng() - 0.5) * desvio) * limites.alto,
  });
  // Disposición amplia mediterránea, sin reproducir ningún país. La seed cambia
  // centros, extensiones, elevaciones, crestas y puertos dentro de esta composición.
  const patrones: [number, number, number, number, number, number, string][] = [
    [0.12, 0.08, 2500, 0.46, 0.70, 0.013, 'Costa boscosa'],
    [0.55, 0.10, 3800, 0.55, 0.42, 0.023, 'Altiplano interior'],
    [0.84, 0.12, 4500, 0.58, 0.25, 0.035, 'Tierras altas secas'],
    [0.755, 0.51, 2300, 0.445, 0.77, 0.007, 'Corredor levantino'],
    [0.53, 0.27, 2900, 0.455, 0.80, 0.009, 'Cuenca fértil'],
    [0.89, 0.46, 4400, 0.465, 0.50, 0.006, 'Gran llanura fluvial'],
    [0.95, 0.28, 2100, 0.53, 0.23, 0.019, 'Piedemonte oriental'],
    [0.22, 0.25, 2100, 0.49, 0.85, 0.025, 'Colinas boscosas'],
    [0.25, 0.945, 3900, 0.44, 0.55, 0.009, 'Llanura costera meridional'],
    [0.54, 0.94, 3000, 0.435, 0.79, 0.004, 'Valle meridional'],
    [0.91, 0.84, 4200, 0.50, 0.15, 0.010, 'Estepa abierta'],
    [0.22, 0.425, 1500, 0.46, 0.62, 0.014, 'Península de los golfos'],
    [0.35, 0.105, 2100, 0.47, 0.68, 0.010, 'Tierras del estrecho'],
    [0.755, 0.78, 2500, 0.51, 0.18, 0.013, 'Meseta del istmo'],
    [0.54, 0.76, 1900, 0.43, 0.88, 0.003, 'Delta interior'],
    [0.88, 0.65, 2500, 0.445, 0.65, 0.006, 'Valles orientales'],
  ];
  const areas = patrones.map(([x, y, radio, altura, humedad, ondulacion, nombre], id) => ({
    id, centro: punto(x, y, 0.025), radio: radio * escala * (0.8 + rng() * 0.4),
    altura: altura + (rng() - 0.5) * 0.025, humedad, ondulacion, nombre,
  }));
  const trazados: [number, number][][] = [
    [[0.20, 0.04], [0.235, 0.14], [0.195, 0.25], [0.235, 0.34], [0.21, 0.44]],
    [[0.42, 0.19], [0.53, 0.20], [0.65, 0.20], [0.78, 0.22], [0.89, 0.31]],
    [[0.80, 0.39], [0.82, 0.48], [0.79, 0.58], [0.81, 0.68], [0.84, 0.76]],
    [[0.64, 0.935], [0.69, 0.95], [0.75, 0.935]],
    [[0.90, 0.05], [0.92, 0.13], [0.97, 0.19]],
  ];
  const crearBarrera = (trazado: [number, number][], id: number): BarreraRegional => {
    const puntos = curva(trazado.map(([x, y]) => punto(x, y, 0.025)));
    let longitud = 0;
    const tramos = puntos.slice(1).map((b, i) => {
      const a = puntos[i]!, largo = Math.hypot(b.x - a.x, b.y - a.y);
      const tramo = { a, b, longitud: largo, inicio: longitud }; longitud += largo; return tramo;
    });
    const fracciones = id < 3 ? [0.28 + rng() * 0.08, 0.65 + rng() * 0.08] : [0.48 + rng() * 0.04];
    const pasos = fracciones.map((f): PasoRegional => {
      const distancia = f * longitud;
      const tramo = tramos.find((t) => t.inicio + t.longitud >= distancia)!;
      const t = (distancia - tramo.inicio) / tramo.longitud;
      return { centro: { x: tramo.a.x + (tramo.b.x - tramo.a.x) * t, y: tramo.a.y + (tramo.b.y - tramo.a.y) * t },
        tangente: { x: (tramo.b.x - tramo.a.x) / tramo.longitud, y: (tramo.b.y - tramo.a.y) / tramo.longitud },
        distancia, radio: (id >= 5 ? 550 + rng() * 200 : 750 + rng() * 250) * escala };
    });
    return { puntos, tramos, longitud, pasos, ancho: (id < 3 ? 480 + rng() * 180 : id >= 5 ? 420 + rng() * 100 : 350 + rng() * 120) * escala,
      altura: id < 3 ? 0.32 + rng() * 0.09 : id >= 5 ? 0.27 + rng() * 0.04 : 0.20 + rng() * 0.06,
      caja: { minX: Math.min(...puntos.map((p) => p.x)), maxX: Math.max(...puntos.map((p) => p.x)),
        minY: Math.min(...puntos.map((p) => p.y)), maxY: Math.max(...puntos.map((p) => p.y)) } };
  };
  const barreras = trazados.map(crearBarrera);
  const campo: RelieveRegional = { revision: 'regional-8', limites, areas, barreras, fase, islas: [] };
  // Islas pequeñas y secundarias, en la franja marítima. No reciben áreas principales.
  for (const [x, y] of [[0.34, 0.44], [0.41, 0.40], [0.32, 0.59], [0.60, 0.55], [0.45, 0.63]]) {
    campo.islas.push({ centro: punto(x!, y!, 0.015), radio: (300 + rng() * 250) * escala });
  }
  // Sierras meridionales: dos flancos desiguales en la zona 9, borde occidental
  // del valle 10 y arco abierto en la estepa 11. No cierran recintos regulares.
  // Se generan después de las islas para conservar toda la estructura anterior.
  const sierrasSur: [number, number][][] = [
    [[0.075, 0.86], [0.145, 0.875], [0.215, 0.855], [0.29, 0.885]],
    [[0.10, 0.975], [0.18, 0.96], [0.26, 0.98], [0.34, 0.96]],
    [[0.43, 0.865], [0.445, 0.925], [0.48, 0.985]],
    [[0.87, 0.795], [0.90, 0.84], [0.935, 0.885], [0.975, 0.88]],
  ];
  barreras.push(...sierrasSur.map((trazado, i) => crearBarrera(trazado, trazados.length + i)));
  // Stream independiente: no desplaza costas, islas ni posiciones de los puertos.
  const ramas = createRng(derivarSeed(seed, 'ramales-regional-5'));
  for (const [id, b] of barreras.entries()) {
    b.morfologia = id === 0 || id === 5 || id === 6 ? 'macizos'
      : id === 4 ? 'meseta' : id === 2 || id === 8 ? 'longitudinal' : id === 3 || id === 7 ? 'escarpe' : 'cordillera';
    b.fase = ramas() * 1000; b.ramales = [];
    const cantidad = Math.max(3, Math.floor(b.longitud / (1050 * escala)));
    for (let i = 0; i < cantidad; i++) {
      const distancia = b.longitud * (0.07 + 0.86 * (i + ramas() * 0.6) / cantidad);
      if (b.pasos.some(p => Math.abs(p.distancia - distancia) < p.radio * 1.25)) continue;
      const tramo = b.tramos.find(t => t.inicio + t.longitud >= distancia)!;
      const t = (distancia - tramo.inicio) / tramo.longitud;
      const origen = { x: tramo.a.x + (tramo.b.x - tramo.a.x) * t, y: tramo.a.y + (tramo.b.y - tramo.a.y) * t };
      const dx = (tramo.b.x - tramo.a.x) / tramo.longitud, dy = (tramo.b.y - tramo.a.y) / tramo.longitud;
      const lado = i % 2 ? -1 : 1, largo = b.ancho * (1.8 + ramas() * 1.5);
      const oblicuo = (ramas() - 0.5) * 0.95;
      const extremo = { x: origen.x + (-dy * lado + dx * oblicuo) * largo,
        y: origen.y + (dx * lado + dy * oblicuo) * largo };
      b.ramales.push({ puntos: [origen, { x: (origen.x + extremo.x) / 2 + dx * largo * 0.16,
        y: (origen.y + extremo.y) / 2 + dy * largo * 0.16 }, extremo],
        ancho: b.ancho * (0.45 + ramas() * 0.27), altura: b.altura * (0.65 + ramas() * 0.30) });
    }
  }
  const macizos = createRng(derivarSeed(seed, 'coronas-regional-6'));
  const estilos = createRng(derivarSeed(seed, 'formas-regional-7'));
  for(const b of barreras){
    b.cumbres=[];
    if(b.morfologia==='escarpe')continue;
    const cantidad=Math.max(3,Math.floor(b.longitud/(850*escala)));
    for(let i=0;i<cantidad;i++){
      const posicion=b.longitud*(.08+.84*(i+macizos()*.7)/cantidad);
      if(b.pasos.some(p=>Math.abs(posicion-p.distancia)<p.radio*1.65))continue;
      const t=b.tramos.find(t=>t.inicio+t.longitud>=posicion)!;
      const u=(posicion-t.inicio)/t.longitud,lado=(macizos()-.5)*b.ancho*.55;
      b.cumbres.push({centro:{x:t.a.x+(t.b.x-t.a.x)*u-(t.b.y-t.a.y)/t.longitud*lado,
        y:t.a.y+(t.b.y-t.a.y)*u+(t.b.x-t.a.x)/t.longitud*lado},
        radio:b.ancho*(1.05+macizos()*.75),altura:b.altura*(.7+macizos()*.55),fase:macizos()*1000});
      const c=b.cumbres[b.cumbres.length-1]!;
      const giro=(estilos()-.5)*.45;
      const tx=(t.b.x-t.a.x)/t.longitud,ty=(t.b.y-t.a.y)/t.longitud;
      c.eje={x:tx*Math.cos(giro)-ty*Math.sin(giro),y:tx*Math.sin(giro)+ty*Math.cos(giro)};
      const elongacion=b.morfologia==='longitudinal'?1.9+estilos()*.4
        :b.morfologia==='meseta'?1.6+estilos()*.35
        :b.morfologia==='cordillera'?1.3+estilos()*.6:1.05+estilos()*.45;
      // End the elongated summit before the approach to the nearest pass.
      const distanciaPaso=Math.min(...b.pasos.map(p=>Math.abs(posicion-p.distancia)));
      c.longitud=Math.min(c.radio*elongacion,Math.max(c.radio*.7,distanciaPaso*.72));
      c.cimaPlana=b.morfologia==='meseta'?.34+estilos()*.16
        :b.morfologia==='macizos'?.04+estilos()*.12:.02;
      c.sesgo=(estilos()-.5)*.45;
      // First regional iteration: Anatolia only. No additional RNG consumption,
      // so every other range, coastline and pass retains its previous geometry.
      if(b===barreras[1]) c.diseccion=.28;
    }
  }
  return campo;
}

export function evaluarRelieveRegional(campo: RelieveRegional, p: Point): number {
  const escala = Math.min(campo.limites.ancho, campo.limites.alto) / 24000;
  const x = p.x / escala, y = p.y / escala;
  let suma = 0, pesos = 0;
  for (const area of campo.areas) {
    const peso = pesoArea(campo, area, p);
    const d2 = (p.x - area.centro.x) ** 2 + (p.y - area.centro.y) ** 2;
    const ondulacion = Math.sin(x / 1100 + campo.fase) * Math.cos(y / 1430 - campo.fase)
      + 0.3 * Math.sin((x + y) / 490 + campo.fase);
    suma += peso * (area.altura + area.ondulacion * ondulacion
      + Math.min(0.026, d2 / (area.radio * area.radio) * 0.018));
    pesos += peso;
  }
  let e = suma / pesos;
  let relieve = 0;
  for (const b of campo.barreras) {
    const margen = b.ancho * 4.5;
    if (p.x < b.caja.minX - margen || p.x > b.caja.maxX + margen
      || p.y < b.caja.minY - margen || p.y > b.caja.maxY + margen) continue;
    let sumaCresta = 0, pesoCresta = 0;
    for (const tramo of b.tramos) {
      const dx = tramo.b.x - tramo.a.x, dy = tramo.b.y - tramo.a.y;
      const t = Math.max(0, Math.min(1, ((p.x - tramo.a.x) * dx + (p.y - tramo.a.y) * dy) / (tramo.longitud ** 2)));
      const d = Math.hypot(p.x - tramo.a.x - t * dx, p.y - tramo.a.y - t * dy);
      const posicion = tramo.inicio + t * tramo.longitud;
      // Mezclar tramos evita saltos de altura en el interior de curvas cerradas.
      // Cresta estrecha, piedemonte amplio: un kernel estrecho a gran distancia
      // cambia demasiado deprisa de tramo en curvas y vuelve a cortar los accesos.
      const anchoMezcla = b.ancho * (0.18 + 0.42 * suave(d / (b.ancho * 1.3)));
      const peso = Math.exp(-(d * d) / (anchoMezcla ** 2)) * tramo.longitud;
      if (peso === 0) continue;
      let silla = 1;
      for (const paso of b.pasos) silla = Math.min(silla, suave((Math.abs(posicion - paso.distancia) - 120 * escala) / paso.radio));
      const terminacion = suave(Math.min(posicion, b.longitud - posicion) / (1000 * escala));
      const f = b.fase ?? campo.fase;
      const perfil = b.morfologia ?? 'cordillera';
      const lateral = ((p.x - tramo.a.x) * -dy + (p.y - tramo.a.y) * dx) / tramo.longitud;
      const variacion = ruidoCresta(posicion / (850 * escala), f);
      const asimetria = perfil === 'escarpe' ? 0.55 : perfil === 'longitudinal' ? 0.30 : 0.24;
      // Ribs and gullies bend the contour in world space, rather than applying
      // fine noise to an otherwise straight ramp. Fade them at the ridge axis.
      const costilla = ruidoCresta(posicion / (210 * escala), f + 29);
      const ancho = b.ancho * (0.65 + variacion * 0.65)
        * (1 + asimetria * Math.tanh(lateral / (b.ancho * 0.28)))
        * (1 + (costilla - 0.5) * 0.65 * suave(d / (b.ancho * 0.35)));
      const estribacion = 0.040 * (1 - suave(d / margen));
      const q = Math.min(1, d / ancho);
      let perfilCresta: number;
      if (perfil === 'escarpe' || perfil === 'meseta') {
        // Unequal, rounded ledges: a broad cap above two eroded cliff bands.
        perfilCresta = 0.56 * (1 - suave((q - 0.30) / 0.16))
          + 0.28 * (1 - suave((q - 0.60) / 0.14))
          + 0.16 * (1 - suave((q - 0.82) / 0.18));
      } else {
        perfilCresta = Math.pow(1 - q, 1.75) * (1 - suave((q - 0.80) / 0.20));
      }
      // Large crowns and smaller shoulders combine at different wavelengths.
      // Floors remain connected barriers; peaks no longer share one height.
      const grandes = ruidoCresta(posicion / (perfil === 'macizos' ? 950 : 1250) / escala, f + 71);
      const hombros = ruidoCresta(posicion / (310 * escala), f + 113);
      const cumbres = perfil === 'meseta' ? .90 + .10 * grandes : perfil === 'escarpe' ? 0.78 + 0.22 * grandes
        : 0.38 + 0.82 * Math.pow(grandes, 1.2) + 0.20 * hombros;
      const cresta = b.altura * perfilCresta * cumbres * (perfil === 'escarpe' ? 1 : perfil === 'meseta' ? .7 : 0.48);
      sumaCresta += peso * (estribacion + cresta) * (0.10 + 0.90 * silla) * terminacion;
      pesoCresta += peso;
    }
    let alturaBarrera = pesoCresta > 0 ? sumaCresta / pesoCresta : 0;
    for (const ramal of b.ramales ?? []) {
      let alturaMezclada = 0, pesoRamal = 0;
      for (let i = 1; i < ramal.puntos.length; i++) {
        const a = ramal.puntos[i - 1]!, c = ramal.puntos[i]!;
        const dx = c.x - a.x, dy = c.y - a.y, l2 = dx * dx + dy * dy;
        const t = Math.max(0, Math.min(1, ((p.x - a.x) * dx + (p.y - a.y) * dy) / l2));
        const d = Math.hypot(p.x - a.x - t * dx, p.y - a.y - t * dy);
        const avance = (i - 1 + t) / (ramal.puntos.length - 1);
        const peso = Math.exp(-d * d / (ramal.ancho * ramal.ancho * 0.36));
        const pie = Math.max(0, 1 - d / ramal.ancho);
        alturaMezclada += peso * ramal.altura * Math.pow(pie, 1.65)
          * (1 - suave(avance)) * (0.75 + 0.25 * ruidoCresta(avance * 4, b.fase ?? 0));
        pesoRamal += peso;
      }
      const alturaRamal = pesoRamal > 1e-20 ? alturaMezclada / pesoRamal : 0;
      // Máximo suave: las uniones no presentan aristas por selección de cresta.
      const mezcla = 0.012;
      const h = Math.max(0, 1 - Math.abs(alturaBarrera - alturaRamal) / mezcla);
      alturaBarrera = Math.max(alturaBarrera, alturaRamal) + mezcla * h * h / 4
        * Math.min(1, alturaRamal / mezcla, alturaBarrera / mezcla);
    }
    // Compound radial crowns give the ridge a body, instead of a continuous
    // extruded wall. Contour ribs fade smoothly into the shared foothills.
    for(const c of b.cumbres ?? []){
      const dx=p.x-c.centro.x,dy=p.y-c.centro.y;
      const eje=c.eje??{x:1,y:0};
      const axial=dx*eje.x+dy*eje.y,lateral=-dx*eje.y+dy*eje.x;
      const sesgo=1+(c.sesgo??0)*Math.tanh(lateral/(c.radio*.4));
      const angulo=Math.atan2(lateral,axial);
      const contorno=1+.14*Math.sin(angulo*3+c.fase)+.08*Math.sin(angulo*5-c.fase);
      const q=Math.hypot(axial/(c.longitud??c.radio),lateral/(c.radio*sesgo))/contorno;
      if(q>=1)continue;
      const tapa=c.cimaPlana??0;
      // Broad shelves in the dry highlands; asymmetric shoulders and an elongated
      // upper ridge elsewhere. The taper is zero with a smooth derivative at the foot.
      const u=Math.max(0,(q-tapa)/(1-tapa));
      const perfilCima=b.morfologia==='meseta'
        ? .68*(1-suave(u/.62))+.32*(1-suave((u-.45)/.55))
        : Math.pow(1-u,b.morfologia==='longitudinal'?1.05:1.35);
      const erosion=b.morfologia==='meseta'?1-.065*(.5+.5*Math.sin(angulo*7+c.fase))*suave(u/.5):1;
      let cuerpo=perfilCima;
      if(c.diseccion){
        // Curved radial gullies, with no angular singularity at the summit.
        // Their finite wavelength survives the 15.625 m heightfield sampling.
        const barranco=Math.pow(.5+.5*Math.sin(angulo*6+c.fase+q*2.8
          +.45*Math.sin(angulo*3-c.fase)),4);
        const flanco=suave((q-.10)/.25)*(1-suave((q-.72)/.22));
        cuerpo*=1-c.diseccion*barranco*flanco;
        // Unequal secondary shoulders fuse into the same crown, rather than
        // adding identical cones or raising the entire ridge to one height.
        for(const [a,l,h,r] of [[-.33,.18,.77,.42],[.27,-.23,.64,.34]]){
          const dist=Math.hypot((axial/c.longitud!-a!)/r!,
            (lateral/c.radio-l!)/(r!*.82));
          const hombro=h!*Math.pow(Math.max(0,1-dist),1.35);
          const fusion=.06,mezcla=Math.max(0,1-Math.abs(cuerpo-hombro)/fusion);
          cuerpo=Math.max(cuerpo,hombro)+fusion*mezcla*mezcla/4;
        }
      }
      const cima=c.altura*cuerpo*(1-suave((q-.78)/.22))*erosion;
      const mezcla=.025,h=Math.max(0,1-Math.abs(alturaBarrera-cima)/mezcla);
      alturaBarrera=Math.max(alturaBarrera,cima)+mezcla*h*h/4*Math.min(1,cima/mezcla,alturaBarrera/mezcla);
    }
    // Collado + valle de aproximación transversal: las ramas tampoco invaden el paso.
    // El corredor se ensancha al alejarse de la silla y vuelve gradualmente al piedemonte.
    for (const paso of b.pasos) {
      const dx = p.x - paso.centro.x, dy = p.y - paso.centro.y;
      const transversal = Math.abs(-dx * paso.tangente.y + dy * paso.tangente.x);
      const longitudinal = Math.abs(dx * paso.tangente.x + dy * paso.tangente.y);
      const valle = (1 - suave(longitudinal / (paso.radio * 0.42 + transversal * 0.24)))
        * (1 - suave(transversal / (b.ancho * 2.8)));
      alturaBarrera *= 1 - 0.82 * valle;
    }
    relieve = Math.max(relieve, alturaBarrera);
  }
  e += relieve;
  const nx = p.x / campo.limites.ancho, ny = p.y / campo.limites.alto;
  // Anchura de la transición suave entre tipos de orilla. La etiqueta visual
  // puede cambiar por zona; la geometría no debe crear una pared en ese límite.
  const roca = Math.max((1 - suave((nx - 0.35) / 0.06)) * (1 - suave((ny - 0.52) / 0.06)),
    suave((nx - 0.35) / 0.06) * (1 - suave((ny - 0.29) / 0.06)));
  const delta = suave((nx - 0.43) / 0.04) * (1 - suave((nx - 0.64) / 0.04)) * suave((ny - 0.675) / 0.07);
  const marisma = suave((ny - 0.705) / 0.07) * Math.max(1 - suave((nx - 0.32) / 0.05), suave((nx - 0.65) / 0.05));
  const anchoCosta = 1100 - 450 * roca + 600 * delta + 350 * marisma;
  e = 0.28 + (e - 0.28) * suave((distanciaCostaRegional(campo, p) + 400 * escala) / (anchoCosta * escala));
  for (const isla of campo.islas) {
    const d = Math.hypot(p.x - isla.centro.x, p.y - isla.centro.y);
    if (d < isla.radio) e = Math.max(e, 0.28 + 0.22 * (1 - suave(d / isla.radio)));
  }
  return Math.min(0.99, Math.max(0, e));
}

/** Ruido 1D continuo; las cimas no repiten una onda periódica. */
function ruidoCresta(x: number, fase: number, angular = false): number {
  const i = Math.floor(x), fraccion = x - i;
  const t = angular ? fraccion * 0.85 + suave(fraccion) * 0.15 : suave(fraccion);
  const hash = (v: number) => { const n = Math.sin(v * 127.1 + fase * 311.7) * 43758.5453; return n - Math.floor(n); };
  return hash(i) * (1 - t) + hash(i + 1) * t;
}
