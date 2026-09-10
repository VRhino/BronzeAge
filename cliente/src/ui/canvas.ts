import type { Asentamiento, BiomaTipo, CaminoComercial, CampamentoBandido, Caravana, Edificio, EdificioTipo, Ejercito, Faccion, Point, RecursoTipo, ZonaFaccion } from '@motor/domain/types';
import type { Mapa } from '@motor/world/mapa';

export const FACCION_COLORES = ['#c0392b', '#2980b9', '#27ae60', '#8e44ad', '#d35400', '#16a085'];

// Paleta elegida para que cada recurso sea distinguible a simple vista (evitar tonos casi iguales,
// ej. trigo/oro ambos amarillentos) — madera no se dibuja como punto en el mapa (ver Doc 1.4: viene de
// bosques, no de nodos), pero conserva un color propio para paneles de almacén/mercado.
export const RECURSO_COLOR: Record<RecursoTipo, string> = {
  madera: '#6b4226',
  piedra: '#8d8d8d',
  trigo: '#c9a227',
  cobre: '#b5651d',
  estano: '#2f6fd1',
  oro: '#ffd700',
  livestock: '#b5658a',
  // Rediseño de progreso (Fase 0, Doc 4.2.1): recursos intermedios de las cadenas de crafting — no se dibujan
  // como nodo en el mapa (RECURSOS_EN_MAPA), pero conservan color propio para paneles de almacén/producción.
  lingoteCobre: '#d98a4a',
  lingoteEstano: '#5a8fd9',
  lingoteBronce: '#a97142',
  cuero: '#8a5a3c',
  cueroCurtido: '#6b4226',
  cueroCalidad: '#4a2e18',
  armaMadera: '#7a5c3a',
  armaCobre: '#c98a4a',
  armaBronce: '#a97142',
  armaBronceCalidad: '#8a5a2e',
  armaduraBasica: '#a9a9a9',
  armaduraIntermedia: '#8d8d8d',
  armaduraBronce: '#6b6b6b',
};

/** Tipos de recurso que SÍ se dibujan como punto en el mapa (para la leyenda) — trigo y madera no lo son. */
export const RECURSOS_EN_MAPA: RecursoTipo[] = ['piedra', 'cobre', 'estano', 'oro', 'livestock'];

// Paleta de biomas (Fase 0.1) — tonos de terreno, deliberadamente más apagados que RECURSO_COLOR para que
// nodos/bosques/edificios sigan destacando encima. 'cima' pálida (aspecto de nieve): señal visual de "aquí
// no se puede fundar ni extraer" (ver `evaluarViabilidadFundacion`).
export const BIOMA_COLOR: Record<BiomaTipo, string> = {
  agua: '#3a6ea5',
  costa: '#c9b98a',
  estepa: '#b0a15a',
  llanuraFertil: '#5a8f4a',
  colina: '#7a6b4a',
  montana: '#6b6b6b',
  cima: '#e9edf0',
};

/**
 * Paleta SIMPLIFICADA (vista por defecto): agua/montaña/cima son las tres bandas que de verdad cambian
 * qué se puede hacer en el terreno (fundar, extraer, atravesar a pie), así que conservan su color propio;
 * costa/estepa/llanuraFertil/colina son variaciones de "tierra habitable normal" y comparten un único tono
 * neutro para que el mapa se lea de un vistazo. El detalle completo (`BIOMA_COLOR`) queda detrás del
 * toggle "Detalle de biomas" para cuando sí importa distinguirlas (ver `mostrarDetalleBiomas` en `main.ts`).
 */
const BIOMA_TIERRA_PLANA = '#93c26b';
export const BIOMA_COLOR_SIMPLE: Record<BiomaTipo, string> = {
  agua: BIOMA_COLOR.agua,
  costa: BIOMA_TIERRA_PLANA,
  estepa: BIOMA_TIERRA_PLANA,
  llanuraFertil: BIOMA_TIERRA_PLANA,
  colina: BIOMA_TIERRA_PLANA,
  montana: BIOMA_COLOR.montana,
  cima: BIOMA_COLOR.cima,
};

/** Precalcula una paleta de biomas a componentes RGB: el pintado del terreno escribe en un `ImageData`
 * píxel a píxel (mucho más rápido que un `fillRect` por celda) y ahí hacen falta los canales sueltos, no
 * el hex. */
function paletaARgb(paleta: Record<BiomaTipo, string>): Record<BiomaTipo, readonly [number, number, number]> {
  return Object.fromEntries(
    Object.entries(paleta).map(([bioma, hex]) => [
      bioma,
      [parseInt(hex.slice(1, 3), 16), parseInt(hex.slice(3, 5), 16), parseInt(hex.slice(5, 7), 16)] as const,
    ])
  ) as Record<BiomaTipo, readonly [number, number, number]>;
}

const BIOMA_RGB = paletaARgb(BIOMA_COLOR);
const BIOMA_RGB_SIMPLE = paletaARgb(BIOMA_COLOR_SIMPLE);

// --- Capa de terreno: color de bioma + sombreado de relieve (hillshade) ---
//
// El COLOR y el RELIEVE se muestrean a resoluciones distintas a propósito, porque cuestan cosas muy
// diferentes: `biomaEn` es cara (fertilidad + distancia a cada segmento de cada río) y no necesita detalle
// fino (son manchas grandes), mientras que `elevacionEn` es barata (4 octavas de seno) y sí lo necesita —
// el relieve es lo que aporta el detalle, y a baja resolución se ve escalonado. El color se pinta pequeño
// y se escala con el suavizado del navegador (así no quedan bordes de bloque); el sombreado se calcula a
// resolución del canvas y se multiplica sobre el resultado.

/** Resolución de muestreo del COLOR de bioma. */
const RES_BIOMA = 128;

/**
 * Convierte la elevación normalizada 0-1 en una ALTURA en unidades de mapa, para que `dz/dx` sea una
 * pendiente geométrica real con la que el modelo Lambert tenga sentido.
 *
 * MEDIDO, no estimado: el gradiente del campo (`|∇elevación|` por unidad de mapa, seed 1) es p50 ≈ 1.1e-3,
 * p90 ≈ 2.1e-3, máx ≈ 3.2e-3. Con 350 la pendiente típica queda en ~0.39 (≈21°) y la máxima en ~1.1 (≈48°):
 * rango donde el sombreado modela volumen de verdad sin que toda ladera sature. Los primeros intentos de
 * esta capa fallaron justo aquí — con una escala ~6x mayor cualquier ladera daba 60-80° y el resultado eran
 * bandas duras en vez de relieve.
 */
const ESCALA_RELIEVE = 350;

/** Color de bioma en una rejilla `RES_BIOMA`², sin sombrear. `detalle` elige la paleta (ver `BIOMA_COLOR`
 * vs `BIOMA_COLOR_SIMPLE`) — el resto del pintado (resolución, sombreado) no cambia entre una y otra. */
function pintarBiomas(mapa: Mapa, detalle: boolean): HTMLCanvasElement {
  const tabla = detalle ? BIOMA_RGB : BIOMA_RGB_SIMPLE;
  const capa = document.createElement('canvas');
  capa.width = RES_BIOMA;
  capa.height = RES_BIOMA;
  const cctx = capa.getContext('2d')!;
  const img = cctx.createImageData(RES_BIOMA, RES_BIOMA);
  const paso = mapa.limites.ancho / RES_BIOMA;

  for (let fila = 0; fila < RES_BIOMA; fila++) {
    for (let col = 0; col < RES_BIOMA; col++) {
      const [r, g, b] = tabla[mapa.biomaEn({ x: (col + 0.5) * paso, y: (fila + 0.5) * paso })];
      const i = (fila * RES_BIOMA + col) * 4;
      img.data[i] = r;
      img.data[i + 1] = g;
      img.data[i + 2] = b;
      img.data[i + 3] = 255;
    }
  }
  cctx.putImageData(img, 0, 0);
  return capa;
}

/**
 * Multiplica el sombreado de relieve sobre los píxeles YA pintados del canvas (el color de bioma).
 *
 * Se hace leyendo y reescribiendo el `ImageData` en vez de componer dos capas con
 * `globalCompositeOperation`: 'multiply' oscurecería el mapa entero (nada puede aclarar) y 'overlay' —que sí
 * aclara y oscurece— arrastra los tonos hacia el gris medio, dejando el mapa lavado. Multiplicando a mano se
 * controla el factor exacto: <1 oscurece la ladera en sombra, >1 aclara la iluminada, y el color de bioma
 * conserva su saturación.
 *
 * Modelo Lambert estándar de cartografía: normal de la superficie `(-dz/dx, -dz/dy, 1)` contra una luz fija
 * en azimut 315° (noroeste) y 45° sobre el horizonte. El noroeste es la convención de los mapas de relieve
 * porque el cerebro interpreta "luz desde arriba-izquierda" como volumen saliente; con la luz desde el sur
 * el relieve se invierte visualmente (las montañas se leen como cráteres).
 */
function aplicarSombreado(ctx: CanvasRenderingContext2D, canvas: HTMLCanvasElement, mapa: Mapa): void {
  const ancho = canvas.width;
  const alto = canvas.height;
  const paso = mapa.limites.ancho / ancho;

  // Alturas precalculadas: cada píxel consulta a sus 4 vecinos para el gradiente, así que muestrear al vuelo
  // repetiría cada punto ~5 veces.
  const alturas = new Float64Array(ancho * alto);
  for (let fila = 0; fila < alto; fila++) {
    const y = (fila + 0.5) * paso;
    for (let col = 0; col < ancho; col++) {
      alturas[fila * ancho + col] = mapa.elevacionEn({ x: (col + 0.5) * paso, y }) * ESCALA_RELIEVE;
    }
  }

  // Vector unitario hacia la luz (azimut 315°, altitud 45°) en coordenadas de pantalla (+x este, +y sur).
  const luzX = -0.5;
  const luzY = -0.5;
  const luzZ = Math.SQRT1_2;
  /** Iluminación de una superficie horizontal: el factor de una zona llana debe ser 1 (color intacto). */
  const lambertPlano = luzZ;

  const img = ctx.getImageData(0, 0, ancho, alto);
  const datos = img.data;

  for (let fila = 0; fila < alto; fila++) {
    const arriba = Math.max(0, fila - 1) * ancho;
    const abajo = Math.min(alto - 1, fila + 1) * ancho;
    const actual = fila * ancho;
    for (let col = 0; col < ancho; col++) {
      const dzdx = (alturas[actual + Math.min(ancho - 1, col + 1)]! - alturas[actual + Math.max(0, col - 1)]!) / (2 * paso);
      const dzdy = (alturas[abajo + col]! - alturas[arriba + col]!) / (2 * paso);
      const longitud = Math.hypot(dzdx, dzdy, 1);
      const lambert = Math.max(0, (-dzdx * luzX - dzdy * luzY + luzZ) / longitud);
      // Suelo en 0.35 para que la cara en sombra conserve algo de color en vez de irse a negro — el
      // equivalente barato de una luz ambiente.
      const factor = Math.max(0.35, lambert / lambertPlano);

      const i = (actual + col) * 4;
      datos[i] = Math.min(255, datos[i]! * factor);
      datos[i + 1] = Math.min(255, datos[i + 1]! * factor);
      datos[i + 2] = Math.min(255, datos[i + 2]! * factor);
    }
  }
  ctx.putImageData(img, 0, 0);
}

/**
 * Capa de FONDO de terreno: color de bioma + sombreado de relieve + ríos. A diferencia de
 * `drawFiltroFertilidad` (overlay opcional, dibujado ENCIMA de todo) esta es la base: el caller la pinta
 * ANTES que bosques/nodos/zonas/edificios. Coste alto (`RES_BIOMA`² consultas de bioma + una pasada de
 * elevación por píxel) — el caller debe cachearla en un canvas offscreen en vez de llamarla en cada
 * `render()` (ver `main.ts`), y esa cache debe incluir `detalleBiomas` en su clave (cambia el resultado).
 *
 * `detalleBiomas`: `false` (por defecto en `main.ts`) pinta la paleta simplificada — agua/montaña/cima
 * distintas, el resto de tierra habitable en un único tono (`BIOMA_COLOR_SIMPLE`) — `true` pinta la
 * paleta completa (`BIOMA_COLOR`), siete tonos distintos.
 */
export function drawTerreno(ctx: CanvasRenderingContext2D, canvas: HTMLCanvasElement, mapa: Mapa, detalleBiomas: boolean): void {
  const scale = canvas.width / mapa.limites.ancho;

  ctx.imageSmoothingEnabled = true;
  ctx.imageSmoothingQuality = 'high';
  ctx.drawImage(pintarBiomas(mapa, detalleBiomas), 0, 0, canvas.width, canvas.height);
  aplicarSombreado(ctx, canvas, mapa);

  // Ríos encima del relieve ya compuesto — no deben teñirse por el sombreado. Los navegables (Doc comercio
  // fluvial, fases futuras — ver `RioZona.navegable`) se dibujan más gruesos: es la única señal hoy de que
  // ya existe esa distinción bajo el capó, antes de que haya barcos que la usen de verdad.
  ctx.strokeStyle = 'rgba(38, 90, 145, 0.9)';
  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';
  for (const rio of mapa.listarRios()) {
    if (rio.puntos.length < 2) continue;
    ctx.lineWidth = rio.navegable ? 5 : 2.5;
    ctx.beginPath();
    rio.puntos.forEach((p, i) => {
      const x = p.x * scale;
      const y = p.y * scale;
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    });
    ctx.stroke();
  }

}

/** Nombre legible de cada tipo de edificio para las etiquetas de la Vista de Asentamiento. */
export const EDIFICIO_ETIQUETA: Record<EdificioTipo, string> = {
  centroUrbano: 'Centro Urbano',
  vivienda: 'Vivienda',
  granja: 'Granja',
  cantera: 'Cantera',
  lenera: 'Leñera',
  almacen: 'Almacén',
  granero: 'Granero',
  mina: 'Mina (oro)',
  minaCobre: 'Mina (cobre)',
  minaEstano: 'Mina (estaño)',
  fundicion: 'Fundición',
  granFundicion: 'Gran Fundición',
  corral: 'Corral',
  curtiduria: 'Curtiduría',
  armeria: 'Armería',
  carpinteria: 'Carpintería',
  palacio: 'Palacio',
  barracon: 'Barracón',
  galeriaDeTiro: 'Galería de tiro',
  mercado: 'Mercado',
  puestoMercado: 'Puesto de mercado',
  maravilla: 'Maravilla',
  plaza: 'Plaza',
  plazaDeArmas: 'Plaza de Armas',
  patioDeGremios: 'Patio de Gremios',
  tallerCarpinteria: 'Taller de carpintería',
  pozo: 'Pozo',
  parque: 'Parque',
};

export const EDIFICIO_COLOR: Record<EdificioTipo, string> = {
  centroUrbano: '#9b59b6',
  vivienda: '#e8e2d0',
  granja: '#d4b106',
  cantera: '#8d8d8d',
  lenera: '#3f7d3a',
  almacen: '#7a5c3a',
  // Grano: dorado apagado, emparentado con la Granja (#d4b106) pero más terroso — se lee como "aquí va el
  // trigo" sin confundirse con el campo que lo produce.
  granero: '#b8933f',
  mina: '#f1c40f',
  minaCobre: '#c0703c',
  minaEstano: '#2f6fd1',
  fundicion: '#b33a3a',
  granFundicion: '#7a1f1f',
  // Rediseño de progreso (Fase 0, Doc 4.2.1): edificios nuevos.
  corral: '#b5658a',
  curtiduria: '#8a5a3c',
  armeria: '#a83232',
  carpinteria: '#6b4226',
  barracon: '#8b3a3a',
  galeriaDeTiro: '#4a7a4a',
  palacio: '#c9a227',
  mercado: '#2d9c8f',
  // Tono más claro del Mercado a propósito: los puestos son piezas de SU zona, y con la etiqueta de texto
  // retirada del lienzo el color es lo único que agrupa el conjunto a la vista.
  puestoMercado: '#7fc9bf',
  maravilla: '#ffd700',
  // Anclas y satélites, Etapa 3: tonos más claros de sus propias categorías (mismo criterio que puestoMercado
  // frente a mercado) — marcan visualmente que son piezas de zona, no edificios independientes.
  plaza: '#f0e8c8',
  plazaDeArmas: '#c97a7a',
  patioDeGremios: '#c99a6b',
  tallerCarpinteria: '#a87850',
  pozo: '#d8e4e8',
  parque: '#c8e0b8',
};

export function faccionColor(faccionId: string, facciones: Faccion[]): string {
  const idx = facciones.findIndex((f) => f.id === faccionId);
  return FACCION_COLORES[Math.max(0, idx) % FACCION_COLORES.length]!;
}

export interface DrawState {
  mapa: Mapa;
  asentamientos: Asentamiento[];
  /**
   * Territorio POR FACCIÓN, ya fusionado en una sola silueta por el motor
   * (`GameStore.getZonasFusionadas` -> `computeZonasFusionadasPorFaccion`). Antes llegaba la lista de zonas
   * por asentamiento y se pintaba un disco por cada uno: como los asentamientos de una misma facción se
   * solapan por diseño, se veían fronteras internas inexistentes y el relleno translúcido se acumulaba en los
   * solapes. Esta capa no fusiona nada por su cuenta — recibe los contornos resueltos.
   */
  zonasFusionadas: ZonaFaccion[];
  facciones: Faccion[];
  caravanas: Caravana[];
  /** Caminos comerciales (Fase 0.3, Doc 1.6) — estado de PARTIDA, a diferencia de los ríos (mundo generado):
   * se dibujan en `draw()` en vivo, nunca en la capa cacheada `drawTerreno`. */
  caminos: CaminoComercial[];
  /** Campamentos de bandidos (Doc 1.9) — estado de partida, se dibujan en vivo igual que las caravanas. */
  campamentosBandidos: CampamentoBandido[];
  /** Ejércitos en campaña (Doc 5.12) — estado de partida, en vivo como las caravanas. */
  ejercitos: Ejercito[];
}

/**
 * Filtro visual de fertilidad (Doc 1.4/4.2): rejilla semitransparente sobre el mapa, verde más intenso =
 * suelo más fértil. Se dibuja ENCIMA de todo lo demás a propósito (es un "filtro" que se puede apagar),
 * no una capa base — por eso vive aparte de `draw()` y el caller decide si llamarlo.
 */
export function drawFiltroFertilidad(ctx: CanvasRenderingContext2D, canvas: HTMLCanvasElement, mapa: Mapa): void {
  const scale = canvas.width / mapa.limites.ancho;
  const celdas = 40;
  const tamanoMundo = mapa.limites.ancho;
  const tamanoCelda = tamanoMundo / celdas;

  for (let fila = 0; fila < celdas; fila++) {
    for (let col = 0; col < celdas; col++) {
      const centro = { x: (col + 0.5) * tamanoCelda, y: (fila + 0.5) * tamanoCelda };
      const fertilidad = mapa.fertilidadEn(centro);
      ctx.fillStyle = `rgba(46, 204, 64, ${fertilidad * 0.45})`;
      ctx.fillRect(col * tamanoCelda * scale, fila * tamanoCelda * scale, tamanoCelda * scale, tamanoCelda * scale);
    }
  }
}

// --- Glifos de árbol para la capa de bosques ---
//
// `hashSemilla`/`mulberry32`: PRNG determinista y barato para dispersar los árboles dentro de cada bosque.
// Se deriva del `id` del bosque (no del RNG del juego, que `draw()` no debe consumir) para que las
// posiciones sean estables entre frames y al hacer pan/zoom, en vez de recalcularse al azar cada vez.
function hashSemilla(texto: string): number {
  let h = 2166136261;
  for (let i = 0; i < texto.length; i++) {
    h ^= texto.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return h >>> 0;
}

function mulberry32(semilla: number): () => number {
  let a = semilla;
  return () => {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** Silueta diminuta de árbol (copa triangular + tronco) centrada en (x, y), del tamaño de canvas dado. */
function dibujarGlifoArbol(ctx: CanvasRenderingContext2D, x: number, y: number, tamano: number): void {
  ctx.beginPath();
  ctx.moveTo(x, y - tamano);
  ctx.lineTo(x + tamano * 0.65, y + tamano * 0.55);
  ctx.lineTo(x - tamano * 0.65, y + tamano * 0.55);
  ctx.closePath();
  ctx.fill();
}

/**
 * `terrenoCache`: canvas offscreen ya pintado por `drawTerreno` (ver `main.ts`) — parámetro aparte de
 * `DrawState` a propósito, porque es un artefacto de RENDER (cacheado por seed+tamaño), no dato de juego.
 * `undefined` = no pintar capa de terreno (p. ej. mientras se genera el primer frame).
 */
export function draw(ctx: CanvasRenderingContext2D, canvas: HTMLCanvasElement, state: DrawState, terrenoCache?: HTMLCanvasElement): void {
  const scale = canvas.width / state.mapa.limites.ancho;
  ctx.clearRect(0, 0, canvas.width, canvas.height);

  if (terrenoCache) ctx.drawImage(terrenoCache, 0, 0);

  // Límites del mapa
  ctx.strokeStyle = '#4a4436';
  ctx.strokeRect(0, 0, canvas.width, canvas.height);

  // Bosques: se pinta la MANCHA fusionada (contorno de la unión de todos los discos, ya resuelto por
  // `Mapa.contornosBosques`), no un círculo por bosque. Los 170 bosques del mapa se solapan muchísimo por
  // diseño, así que dibujarlos uno a uno daba un amasijo de círculos con un anillo más oscuro en cada
  // intersección; con una sola pasada de relleno la mancha queda con opacidad uniforme y silueta propia,
  // distinguible de las otras capas translúcidas (fertilidad, zonas de influencia) que comparten el mismo
  // vocabulario de "polígono semitransparente". Todos los lazos van en el mismo path: los claros encerrados
  // por una corona de bosque se recortan solos con la regla `nonzero`.
  //
  // El borde SÍ se traza ahora (antes no había ninguno, porque el borde lo insinuaban los propios árboles):
  // es lo que hace que la mancha se lea como una masa forestal con forma y no como una nube difusa.
  ctx.beginPath();
  for (const contorno of state.mapa.contornosBosques()) {
    contorno.forEach((p, i) => {
      const x = p.x * scale;
      const y = p.y * scale;
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    });
    ctx.closePath();
  }
  ctx.fillStyle = 'rgba(38, 92, 40, 0.32)';
  ctx.fill();
  ctx.strokeStyle = 'rgba(24, 64, 26, 0.45)';
  ctx.lineWidth = 1;
  ctx.stroke();

  // Árboles: siguen sembrándose bosque a bosque, y son ahora la ÚNICA señal de densidad de madera (el relleno
  // pasó a ser uniforme al fusionarse — un solo tono no puede representar 170 densidades distintas, y donde
  // dos bosques se solapan tampoco habría un valor "correcto" que mostrar). Cantidad ∝ área*densidad;
  // posiciones deterministas por id de bosque (ver `mulberry32` arriba) así no bailan entre frames. Muestreo
  // uniforme en disco: r = radio·√rand, no r = radio·rand (que amontonaría los puntos en el centro).
  ctx.fillStyle = 'rgba(22, 62, 24, 0.9)';
  for (const bosque of state.mapa.listarBosques()) {
    const cx = bosque.centro.x * scale;
    const cy = bosque.centro.y * scale;
    const radioPx = bosque.radio * scale;
    const rand = mulberry32(hashSemilla(bosque.id));
    const numArboles = Math.round(Math.min(60, Math.max(4, (radioPx * radioPx * bosque.densidad) / 22)));
    for (let i = 0; i < numArboles; i++) {
      const angulo = rand() * Math.PI * 2;
      const r = radioPx * Math.sqrt(rand());
      dibujarGlifoArbol(ctx, cx + Math.cos(angulo) * r, cy + Math.sin(angulo) * r, 2.2);
    }
  }

  // Nodos de recurso
  for (const nodo of state.mapa.listarNodos()) {
    ctx.beginPath();
    ctx.arc(nodo.posicion.x * scale, nodo.posicion.y * scale, 3, 0, Math.PI * 2);
    ctx.fillStyle = RECURSO_COLOR[nodo.tipo];
    ctx.fill();
  }

  // Zonas de influencia: UNA silueta por facción, no un disco por asentamiento (ver `zonasFusionadas`).
  // Todos los lazos de una facción van en el MISMO path y se rellenan de una sola pasada: así el interior
  // queda con opacidad uniforme (no se acumula donde dos asentamientos hermanos se solapan) y los huecos que
  // la facción rodea sin reclamar se recortan solos por la regla de relleno `nonzero`, que es la de por
  // defecto y la que los contornos ya vienen preparados para aprovechar (orientación opuesta).
  for (const zonaFaccion of state.zonasFusionadas) {
    if (zonaFaccion.contornos.length === 0) continue;
    const color = faccionColor(zonaFaccion.faccionId, state.facciones);
    ctx.beginPath();
    for (const contorno of zonaFaccion.contornos) {
      contorno.forEach((p, i) => {
        const x = p.x * scale;
        const y = p.y * scale;
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      });
      ctx.closePath();
    }
    ctx.fillStyle = color + '33';
    ctx.fill();
    ctx.strokeStyle = color;
    ctx.lineWidth = 2;
    ctx.stroke();
  }

  // Caminos comerciales (Fase 0.3, Doc 1.6): estado de partida, no del mundo generado — a diferencia de los
  // ríos, se dibujan aquí en vivo. Trazo discontinuo para distinguirlos de ríos (sólido, azul) y fronteras
  // de zona (sólido, color de Facción).
  ctx.strokeStyle = 'rgba(139, 90, 43, 0.9)';
  ctx.lineWidth = 2.5;
  ctx.setLineDash([6, 4]);
  for (const camino of state.caminos) {
    if (camino.puntos.length < 2) continue;
    ctx.beginPath();
    camino.puntos.forEach((p, i) => {
      const x = p.x * scale;
      const y = p.y * scale;
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    });
    ctx.stroke();
  }
  ctx.setLineDash([]);

  // Edificios (Doc 4.2): cuadrado relleno = activo, semitransparente = en construcción, solo contorno = en cola.
  // Vista de Asentamiento (a petición del usuario): en el mapa general SOLO se dibujan los edificios de
  // `ambito: 'mapa'` (extractores minerales, plantados sobre su nodo). Todo lo demás vive en el espacio plano
  // del asentamiento (coords locales) y se dibuja en `drawAsentamiento`, nunca aquí.
  const tamanoEdificio = 6;
  for (const asentamiento of state.asentamientos) {
    for (const edificio of asentamiento.edificios) {
      if ((edificio.ambito ?? 'asentamiento') !== 'mapa') continue;
      const x = edificio.posicion.x * scale;
      const y = edificio.posicion.y * scale;
      const color = EDIFICIO_COLOR[edificio.tipo];
      ctx.lineWidth = 1;
      ctx.strokeStyle = color;
      if (edificio.estado === 'activo') {
        ctx.fillStyle = color;
        ctx.fillRect(x - tamanoEdificio / 2, y - tamanoEdificio / 2, tamanoEdificio, tamanoEdificio);
      } else if (edificio.estado === 'en_construccion') {
        ctx.fillStyle = color + '88';
        ctx.fillRect(x - tamanoEdificio / 2, y - tamanoEdificio / 2, tamanoEdificio, tamanoEdificio);
        ctx.strokeRect(x - tamanoEdificio / 2, y - tamanoEdificio / 2, tamanoEdificio, tamanoEdificio);
      } else {
        ctx.strokeRect(x - tamanoEdificio / 2, y - tamanoEdificio / 2, tamanoEdificio, tamanoEdificio);
      }
    }
  }

  // Asentamientos (punto de fundación)
  for (const asentamiento of state.asentamientos) {
    const color = faccionColor(asentamiento.faccionId, state.facciones);
    ctx.beginPath();
    ctx.arc(asentamiento.posicion.x * scale, asentamiento.posicion.y * scale, 6, 0, Math.PI * 2);
    ctx.fillStyle = color;
    ctx.fill();
    ctx.strokeStyle = '#1b1a17';
    ctx.lineWidth = 1.5;
    ctx.stroke();
  }

  // Ruta de caravana en tránsito (Fase 0.3): traza tenue detrás del punto — ayuda a depurar visualmente que
  // el pathfinding rodea montaña/agua en vez de ir en línea recta (ver `world/rutas.ts`).
  ctx.strokeStyle = 'rgba(241, 230, 200, 0.35)';
  ctx.lineWidth = 1.5;
  for (const caravana of state.caravanas) {
    if (!caravana.ruta || caravana.ruta.length < 2) continue;
    ctx.beginPath();
    caravana.ruta.forEach((p, i) => {
      const x = p.x * scale;
      const y = p.y * scale;
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    });
    ctx.stroke();
  }

  // Caravanas en tránsito (Doc 3.2/3.6, a petición del usuario): antes un punto crema fijo, indistinguible
  // entre facciones — ahora un triángulo (distinto de los círculos de asentamiento y del diamante de
  // campamentos de bandidos, ver abajo) coloreado según la Facción del asentamiento de ORIGEN (dueño real de
  // la flota, `origenAsentamientoId` nunca cambia — ver `estado` en domain/types.ts), para poder identificar
  // de un vistazo a quién pertenece cada una.
  for (const caravana of state.caravanas) {
    const x = caravana.posicionActual.x * scale;
    const y = caravana.posicionActual.y * scale;
    const origenCaravana = state.asentamientos.find((a) => a.id === caravana.origenAsentamientoId);
    const color = origenCaravana ? faccionColor(origenCaravana.faccionId, state.facciones) : '#f1e6c8';
    const r = 4.5;
    ctx.beginPath();
    ctx.moveTo(x, y - r);
    ctx.lineTo(x + r, y + r);
    ctx.lineTo(x - r, y + r);
    ctx.closePath();
    ctx.fillStyle = color;
    ctx.fill();
    ctx.strokeStyle = '#1b1a17';
    ctx.lineWidth = 1;
    ctx.stroke();
  }

  // Ejércitos en campaña (Doc 5.12.2): traza de ruta + ROMBOS, uno por cada jugador que va dentro, apilados
  // medio superpuestos y del color de la Facción.
  //
  // El número de rombos es `ejercito.participantes.length` (Doc 5.12.1): quién va DENTRO, aporte tropas o no
  // — un ciudadano que sale solo a explorar es un ejército de un participante y SIN escuadrones. Derivarlo de
  // los `jugadorId` de los escuadrones (como se hacía antes) lo dejaba invisible en el mapa. Así, de un
  // vistazo, el tamaño del racimo dice cuánta gente va en esa columna.
  //
  // Rombo y no triángulo (caravana) ni círculo (asentamiento) ni diamante rojo (campamento bandido): las
  // cuatro cosas que se mueven o amenazan en este mapa tienen forma propia, para no depender del color.
  ctx.strokeStyle = 'rgba(241, 230, 200, 0.35)';
  ctx.lineWidth = 1.5;
  for (const ejercito of state.ejercitos) {
    // Estacionado no tiene trayecto pendiente que enseñar (acampó); marchando y regresando sí.
    if (ejercito.estado === 'estacionado' || ejercito.ruta.length < 2) continue;
    ctx.beginPath();
    ejercito.ruta.forEach((p, i) => {
      const x = p.x * scale;
      const y = p.y * scale;
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    });
    ctx.stroke();
  }

  for (const ejercito of state.ejercitos) {
    const origen = state.asentamientos.find((a) => a.id === ejercito.origenAsentamientoId);
    // Columna de un ciudadano sin Facción todavía (Doc 5.12.1): dorado neutro, no el color de una Facción,
    // para no confundirla con un ejército.
    const color = ejercito.faccionId
      ? origen
        ? faccionColor(origen.faccionId, state.facciones)
        : faccionColor(ejercito.faccionId, state.facciones)
      : '#f1d38b';
    const participantes = Math.max(1, ejercito.participantes.length);
    const r = 5;
    // Cada rombo se desplaza medio ancho respecto al anterior (solape del 50%), y el racimo entero se
    // recentra para que la POSICIÓN del ejército siga cayendo en el medio y no en el primer rombo.
    const inicio = ((participantes - 1) * r) / 2;
    const cx = ejercito.posicionActual.x * scale;
    const cy = ejercito.posicionActual.y * scale;

    // De atrás hacia delante, para que el primero quede ENCIMA y el racimo se lea como una columna.
    for (let i = participantes - 1; i >= 0; i--) {
      const x = cx - inicio + i * r;
      ctx.beginPath();
      ctx.moveTo(x, cy - r);
      ctx.lineTo(x + r, cy);
      ctx.lineTo(x, cy + r);
      ctx.lineTo(x - r, cy);
      ctx.closePath();
      ctx.fillStyle = color;
      ctx.fill();
      ctx.strokeStyle = '#1b1a17';
      ctx.lineWidth = 1;
      ctx.stroke();
    }
  }

  // Campamentos de bandidos (Doc 1.9): marcador en forma de diamante, distinto de asentamientos (círculos) y
  // caravanas (puntos claros) para que se reconozca de un vistazo como amenaza, no como activo propio.
  for (const campamento of state.campamentosBandidos) {
    const x = campamento.posicion.x * scale;
    const y = campamento.posicion.y * scale;
    const r = 6;
    ctx.beginPath();
    ctx.moveTo(x, y - r);
    ctx.lineTo(x + r, y);
    ctx.lineTo(x, y + r);
    ctx.lineTo(x - r, y);
    ctx.closePath();
    ctx.fillStyle = '#8b1a1a';
    ctx.fill();
    ctx.strokeStyle = '#1b1a17';
    ctx.lineWidth = 1.5;
    ctx.stroke();
  }
}

// --- Vista de Asentamiento (espacio plano local, a petición del usuario) ---
//
// Un espacio lógico SEPARADO del mapa general: plano (sin relieve/biomas/ríos), centrado en el Centro Urbano.
// Aquí se dibujan, con detalle y etiquetas, TODOS los edificios internos (`ambito !== 'mapa'`), a sus
// coordenadas LOCALES (origen `(0,0)` en el centro). Los extractores minerales (`ambito: 'mapa'`) NO viven
// aquí — se ven en el mapa general.
//
// ACOPLAMIENTO 0, y en esta vista importa especialmente: este archivo NO sabe qué es una celda ocupada, una
// arista, un barrio ni una manzana. Todo el trazado urbano (qué rectángulo ocupa cada edificio, por dónde
// pasan las calles y los caminos) lo calcula el motor en `engine/trazado.ts` y llega ya resuelto a
// coordenadas locales vía `gameStore.getTrazadoAsentamiento`. Aquí solo se pinta. Si algo del dibujo parece
// mal colocado, el arreglo va en el motor, nunca aquí.

export interface DrawAsentamientoState {
  asentamiento: Asentamiento;
  /** Nombre a mostrar (el `nombre` del asentamiento o su `id`) — lo resuelve el caller. */
  etiqueta: string;
  /** Tiradas de calle urbana, en coordenadas locales — `gameStore.getTrazadoAsentamiento`.
   * Etapa 6: son ÁREAS (la calle ocupa suelo), no líneas sin grosor. */
  calles: { x: number; y: number; ancho: number; alto: number }[];
  /** Tiradas de camino rural (a Granja/Corral, en las afueras): clase aparte de la calle, se pinta más apagada. */
  caminos: { x: number; y: number; ancho: number; alto: number }[];
  /** Rectángulo que ocupa cada edificio interno, por id, en unidades locales (esquina superior izquierda +
   * ancho/alto). Los tamaños varían por tipo y, en Granja, por nivel interno. */
  huellas: Record<string, { x: number; y: number; ancho: number; alto: number }>;
  /** Tamaño de celda de la rejilla local (unidades locales) — `CATALOGOS.tamanoCeldaAsentamiento`. */
  tamanoCelda: number;
  /** Radio ESTÁTICO del espacio de la Vista de Asentamiento (unidades locales) — `CATALOGOS.radioMapaAsentamiento`
   * / `REJILLA_ASENTAMIENTO.radioMapa`. A petición del usuario: define la escala del lienzo en vez de
   * `asentamiento.radioPotencial` (que crece con nivel/construcción) para que el mapa NO haga zoom a medida que
   * el asentamiento crece — el espacio se ve fijo desde el principio, solo se va llenando. */
  radioMapa: number;
}

/**
 * Marcador de un edificio interno en la Vista de Asentamiento. Relleno = activo, semitransparente = en
 * construcción, solo contorno = en cola (mismo vocabulario que el mapa general, pero más grande).
 *
 * SIN etiqueta de texto (a petición del usuario): el tipo se identifica por color, y el rótulo bajo cada
 * edificio ensuciaba la vista ahora que la ciudad es densa y las manzanas se leen como bloques.
 *
 * `huella` es el rectángulo que ocupa el edificio, en píxeles de pantalla, ya calculado por el motor: cada
 * tipo tiene su tamaño y Granja además cambia con el nivel. Se dibuja con un margen HACIA ADENTRO porque los
 * edificios van pared con pared: sin ese margen, la calle —que corre justo sobre el borde compartido— quedaría
 * tapada y el dibujo se leería como una masa continua en vez de una manzana.
 */
function dibujarEdificioLocal(
  ctx: CanvasRenderingContext2D,
  edificio: Edificio,
  huella: { x: number; y: number; ancho: number; alto: number }
): void {
  const color = EDIFICIO_COLOR[edificio.tipo];
  const esCentro = edificio.tipo === 'centroUrbano';
  const margen = Math.min(1.5, huella.ancho * 0.12, huella.alto * 0.12);
  const x = huella.x + margen;
  const y = huella.y + margen;
  const ancho = Math.max(2, huella.ancho - margen * 2);
  const alto = Math.max(2, huella.alto - margen * 2);

  ctx.lineWidth = esCentro ? 2 : 1.4;
  ctx.strokeStyle = esCentro ? '#1b1a17' : color;
  if (edificio.estado === 'activo') {
    ctx.fillStyle = color;
    ctx.fillRect(x, y, ancho, alto);
    if (esCentro) ctx.strokeRect(x, y, ancho, alto);
  } else if (edificio.estado === 'en_construccion') {
    ctx.fillStyle = color + '88';
    ctx.fillRect(x, y, ancho, alto);
    ctx.strokeRect(x, y, ancho, alto);
  } else {
    ctx.setLineDash([3, 2]);
    ctx.strokeRect(x, y, ancho, alto);
    ctx.setLineDash([]);
  }
}

/** Pinta una tanda de tiradas de calle ya resueltas por el motor. No decide ningún trazado: solo rellena los
 * rectángulos que le llegan. Etapa 6: la calle es SUPERFICIE, así que se rellena en vez de trazarse — que es
 * justamente lo que el modelo de aristas no podía representar (una línea no tiene ancho). */
function dibujarTiradas(
  ctx: CanvasRenderingContext2D,
  tiradas: { x: number; y: number; ancho: number; alto: number }[],
  aPantalla: (p: Point) => Point,
  escala: number,
  color: string
): void {
  if (tiradas.length === 0) return;
  ctx.fillStyle = color;
  for (const t of tiradas) {
    const esquina = aPantalla({ x: t.x, y: t.y });
    ctx.fillRect(esquina.x, esquina.y, t.ancho * escala, t.alto * escala);
  }
}

/**
 * Pinta el espacio plano de UN asentamiento a pantalla completa del canvas. `terrenoCache` no aplica aquí:
 * este espacio no tiene terreno que cachear (es plano por definición).
 */
export function drawAsentamiento(ctx: CanvasRenderingContext2D, canvas: HTMLCanvasElement, state: DrawAsentamientoState): void {
  const { asentamiento, etiqueta, calles, caminos, huellas, tamanoCelda, radioMapa } = state;
  ctx.clearRect(0, 0, canvas.width, canvas.height);

  // Fondo verde llano: mismo tono que la tierra habitable del mapa general en su paleta simplificada — la
  // ciudad es "el mismo mundo", no un espacio de otro color que rompa la continuidad visual.
  ctx.fillStyle = BIOMA_TIERRA_PLANA;
  ctx.fillRect(0, 0, canvas.width, canvas.height);

  // Escala FIJA sobre `radioMapa` (a petición del usuario), nunca sobre `asentamiento.radioPotencial`: ese
  // crece con nivel/construcción y haría que el lienzo hiciera zoom con el tiempo — el espacio de la Vista de
  // Asentamiento es estático, solo se va llenando de edificios.
  const usable = canvas.width * 0.92;
  const escala = usable / (radioMapa * 2);
  const cx = canvas.width / 2;
  const cy = canvas.height / 2;
  const aPantalla = (p: Point): Point => ({ x: cx + p.x * escala, y: cy + p.y * escala });

  // Cuadrícula de fondo (a petición del usuario): mismas celdas que usa la colocación (`tamanoCelda`) — da
  // referencia visual de escala real y de dónde puede caer el próximo edificio. Cubre el lienzo entero (no solo
  // `radioMapa`), para no dejar un borde sin marcar.
  ctx.strokeStyle = 'rgba(27, 26, 23, 0.08)';
  ctx.lineWidth = 1;
  const pasoPantalla = tamanoCelda * escala;
  const celdasX = Math.ceil(cx / pasoPantalla) + 1;
  const celdasY = Math.ceil(cy / pasoPantalla) + 1;
  ctx.beginPath();
  for (let i = -celdasX; i <= celdasX; i++) {
    const x = cx + i * pasoPantalla;
    ctx.moveTo(x, 0);
    ctx.lineTo(x, canvas.height);
  }
  for (let j = -celdasY; j <= celdasY; j++) {
    const y = cy + j * pasoPantalla;
    ctx.moveTo(0, y);
    ctx.lineTo(canvas.width, y);
  }
  ctx.stroke();

  // Edificios internos (ambito !== 'mapa'), a coords locales.
  const internos = asentamiento.edificios.filter((e) => (e.ambito ?? 'asentamiento') !== 'mapa');

  // Trazado: caminos primero (más finos, van por debajo) y calles encima. Los tramos vienen ya resueltos del
  // motor; aquí no se decide por dónde pasa ninguno.
  dibujarTiradas(ctx, caminos, aPantalla, escala, 'rgba(140, 118, 88, 0.45)');
  dibujarTiradas(ctx, calles, aPantalla, escala, 'rgba(120, 92, 58, 0.60)');

  // Edificios: cada uno con su huella real (varía por tipo, y por nivel interno en Granja). Se dibujan las
  // Viviendas primero para que las etiquetas de los edificios singulares queden por encima.
  const enPantalla = (id: string): { x: number; y: number; ancho: number; alto: number } | null => {
    const huella = huellas[id];
    if (!huella) return null;
    const origen = aPantalla({ x: huella.x, y: huella.y });
    return { x: origen.x, y: origen.y, ancho: huella.ancho * escala, alto: huella.alto * escala };
  };
  for (const edificio of [...internos.filter((e) => e.tipo === 'vivienda'), ...internos.filter((e) => e.tipo !== 'vivienda')]) {
    const huella = enPantalla(edificio.id);
    if (huella) dibujarEdificioLocal(ctx, edificio, huella);
  }

  // Título y nota, abajo (para no solaparse con el toggle Mundo/Asentamiento que flota sobre la esquina
  // superior izquierda del lienzo, ver `.map-toggles` en style.css).
  ctx.fillStyle = '#1b1a17';
  ctx.font = 'bold 15px system-ui, sans-serif';
  ctx.textAlign = 'left';
  ctx.textBaseline = 'bottom';
  ctx.fillText(`${etiqueta} · nivel ${asentamiento.nivel}`, 12, canvas.height - 24);
  ctx.font = '11px system-ui, sans-serif';
  ctx.fillStyle = '#4a4436';
  ctx.fillText('Vista de asentamiento (ciudad) · minas y cantera pertenecen a la región (mapa general)', 12, canvas.height - 8);
  ctx.textAlign = 'left';
  ctx.textBaseline = 'alphabetic';
}
