import type { Asentamiento, BiomaTipo, Caravana, EdificioTipo, Faccion, RecursoTipo, ZonaInfluencia } from '../domain/types';
import type { Mapa } from '../world/mapa';

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

/** `BIOMA_COLOR` precalculado a componentes RGB: el pintado del terreno escribe en un `ImageData` píxel a
 * píxel (mucho más rápido que un `fillRect` por celda) y ahí hacen falta los canales sueltos, no el hex. */
const BIOMA_RGB = Object.fromEntries(
  Object.entries(BIOMA_COLOR).map(([bioma, hex]) => [
    bioma,
    [parseInt(hex.slice(1, 3), 16), parseInt(hex.slice(3, 5), 16), parseInt(hex.slice(5, 7), 16)] as const,
  ])
) as Record<BiomaTipo, readonly [number, number, number]>;

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

/** Color de bioma en una rejilla `RES_BIOMA`², sin sombrear. */
function pintarBiomas(mapa: Mapa): HTMLCanvasElement {
  const capa = document.createElement('canvas');
  capa.width = RES_BIOMA;
  capa.height = RES_BIOMA;
  const cctx = capa.getContext('2d')!;
  const img = cctx.createImageData(RES_BIOMA, RES_BIOMA);
  const paso = mapa.limites.ancho / RES_BIOMA;

  for (let fila = 0; fila < RES_BIOMA; fila++) {
    for (let col = 0; col < RES_BIOMA; col++) {
      const [r, g, b] = BIOMA_RGB[mapa.biomaEn({ x: (col + 0.5) * paso, y: (fila + 0.5) * paso })];
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
 * `render()` (ver `main.ts`).
 */
export function drawTerreno(ctx: CanvasRenderingContext2D, canvas: HTMLCanvasElement, mapa: Mapa): void {
  const scale = canvas.width / mapa.limites.ancho;

  ctx.imageSmoothingEnabled = true;
  ctx.imageSmoothingQuality = 'high';
  ctx.drawImage(pintarBiomas(mapa), 0, 0, canvas.width, canvas.height);
  aplicarSombreado(ctx, canvas, mapa);

  // Ríos encima del relieve ya compuesto — no deben teñirse por el sombreado.
  ctx.strokeStyle = 'rgba(38, 90, 145, 0.9)';
  ctx.lineWidth = 2.5;
  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';
  for (const rio of mapa.listarRios()) {
    if (rio.puntos.length < 2) continue;
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

export const EDIFICIO_COLOR: Record<EdificioTipo, string> = {
  centroUrbano: '#9b59b6',
  vivienda: '#e8e2d0',
  granja: '#d4b106',
  cantera: '#8d8d8d',
  lenera: '#3f7d3a',
  almacen: '#7a5c3a',
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
};

export function faccionColor(faccionId: string, facciones: Faccion[]): string {
  const idx = facciones.findIndex((f) => f.id === faccionId);
  return FACCION_COLORES[Math.max(0, idx) % FACCION_COLORES.length]!;
}

export interface DrawState {
  mapa: Mapa;
  asentamientos: Asentamiento[];
  zonas: ZonaInfluencia[];
  facciones: Faccion[];
  caravanas: Caravana[];
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

export interface PreviewFundacion {
  posicion: { x: number; y: number };
  radioInicial: number;
  fundable: boolean;
  bosqueAlcanzable: boolean;
}

/**
 * Previsualización del emplazamiento bajo el cursor, antes de fundar. Verde = fundable y con bosque al
 * alcance; ámbar = legal pero SIN madera alcanzable (se puede fundar, pero el asentamiento casi siempre
 * acaba en ruinas — ver `evaluarViabilidadFundacion`); rojo = no se puede fundar ahí.
 * Se dibuja encima de todo, igual que el filtro de fertilidad, y el caller decide cuándo llamarlo.
 */
export function drawPreviewFundacion(
  ctx: CanvasRenderingContext2D,
  canvas: HTMLCanvasElement,
  mapa: Mapa,
  preview: PreviewFundacion
): void {
  const scale = canvas.width / mapa.limites.ancho;
  const color = !preview.fundable ? '#c0392b' : preview.bosqueAlcanzable ? '#27ae60' : '#e0a020';

  ctx.beginPath();
  ctx.arc(preview.posicion.x * scale, preview.posicion.y * scale, preview.radioInicial * scale, 0, Math.PI * 2);
  ctx.fillStyle = color + '22';
  ctx.fill();
  ctx.strokeStyle = color;
  ctx.lineWidth = 2;
  ctx.setLineDash([5, 4]);
  ctx.stroke();
  ctx.setLineDash([]);

  ctx.beginPath();
  ctx.arc(preview.posicion.x * scale, preview.posicion.y * scale, 3, 0, Math.PI * 2);
  ctx.fillStyle = color;
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

  // Bosques (zonas). Se apoyan en el CONTORNO más que en el relleno: con 100 bosques sobre un relieve ya
  // detallado, un relleno opaco tapaba el terreno y el mapa se volvía una masa verde. El contorno propio es
  // lo que los hace legibles sin depender de contrastar con el bioma de debajo (varios son verdes/caqui
  // parecidos), y el relleno translúcido solo insinúa la densidad de madera.
  for (const bosque of state.mapa.listarBosques()) {
    ctx.beginPath();
    ctx.arc(bosque.centro.x * scale, bosque.centro.y * scale, bosque.radio * scale, 0, Math.PI * 2);
    ctx.fillStyle = `rgba(38, 92, 40, ${0.1 + bosque.densidad * 0.2})`;
    ctx.fill();
    ctx.strokeStyle = 'rgba(24, 66, 26, 0.75)';
    ctx.lineWidth = 1.2;
    ctx.stroke();
  }

  // Nodos de recurso
  for (const nodo of state.mapa.listarNodos()) {
    ctx.beginPath();
    ctx.arc(nodo.posicion.x * scale, nodo.posicion.y * scale, 3, 0, Math.PI * 2);
    ctx.fillStyle = RECURSO_COLOR[nodo.tipo];
    ctx.fill();
  }

  // Zonas de influencia (polígono recortado por fronteras)
  for (const zona of state.zonas) {
    const asentamiento = state.asentamientos.find((a) => a.id === zona.asentamientoId);
    if (!asentamiento || zona.poligono.length === 0) continue;
    const color = faccionColor(asentamiento.faccionId, state.facciones);
    ctx.beginPath();
    zona.poligono.forEach((p, i) => {
      const x = p.x * scale;
      const y = p.y * scale;
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    });
    ctx.closePath();
    ctx.fillStyle = color + '33';
    ctx.fill();
    ctx.strokeStyle = color;
    ctx.lineWidth = 2;
    ctx.stroke();
  }

  // Edificios (Doc 4.2): cuadrado relleno = activo, semitransparente = en construcción, solo contorno = en cola.
  const tamanoEdificio = 6;
  for (const asentamiento of state.asentamientos) {
    for (const edificio of asentamiento.edificios) {
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

  // Caravanas en tránsito (Doc 3.2/3.6): punto moviéndose entre origen y destino.
  for (const caravana of state.caravanas) {
    const x = caravana.posicionActual.x * scale;
    const y = caravana.posicionActual.y * scale;
    ctx.beginPath();
    ctx.arc(x, y, 3.5, 0, Math.PI * 2);
    ctx.fillStyle = '#f1e6c8';
    ctx.fill();
    ctx.strokeStyle = '#1b1a17';
    ctx.lineWidth = 1;
    ctx.stroke();
  }
}
