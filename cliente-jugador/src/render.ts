import { contornosBosques, evaluarBioma, evaluarElevacion, type MapaGenerado } from './terreno';
import type { ProyeccionJugador } from './apiCliente';
import type { Asentamiento, Edificio, Point, SegmentoTrazado, TrazadoAsentamiento } from './tiposDominio';
import {
  BIOMA_COLOR_SIMPLE,
  EDIFICIO_COLOR,
  RECURSO_COLOR,
  faccionColor,
} from './paletas';

// --- GENERADOR DETERMINISTA (Para los árboles) ---
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

function dibujarGlifoArbol(ctx: CanvasRenderingContext2D, x: number, y: number, tamano: number): void {
  ctx.beginPath();
  ctx.moveTo(x, y - tamano);
  ctx.lineTo(x + tamano * 0.65, y + tamano * 0.55);
  ctx.lineTo(x - tamano * 0.65, y + tamano * 0.55);
  ctx.closePath();
  ctx.fill();
}

// --- RENDER BASE DEL TERRENO (Cacheable) ---
const RES_BIOMA = 128;
const ESCALA_RELIEVE = 350;

function hexToRgb(hex: string): readonly [number, number, number] {
  return [
    parseInt(hex.slice(1, 3), 16),
    parseInt(hex.slice(3, 5), 16),
    parseInt(hex.slice(5, 7), 16),
  ];
}

const BIOMA_RGB_SIMPLE: Record<string, readonly [number, number, number]> = Object.fromEntries(
  Object.entries(BIOMA_COLOR_SIMPLE).map(([k, v]) => [k, hexToRgb(v)])
);

function pintarBiomas(mapa: MapaGenerado): HTMLCanvasElement {
  const capa = document.createElement('canvas');
  capa.width = RES_BIOMA;
  capa.height = RES_BIOMA;
  const cctx = capa.getContext('2d')!;
  const img = cctx.createImageData(RES_BIOMA, RES_BIOMA);
  const paso = mapa.config.ancho / RES_BIOMA;

  for (let fila = 0; fila < RES_BIOMA; fila++) {
    for (let col = 0; col < RES_BIOMA; col++) {
      const punto = { x: (col + 0.5) * paso, y: (fila + 0.5) * paso };
      const bioma = evaluarBioma(mapa.elevacion, mapa.fertilidad, mapa.rios, punto);
      const [r, g, b] = BIOMA_RGB_SIMPLE[bioma] || [0, 0, 0];
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

function aplicarSombreado(ctx: CanvasRenderingContext2D, canvas: HTMLCanvasElement, mapa: MapaGenerado): void {
  const ancho = canvas.width;
  const alto = canvas.height;
  const paso = mapa.config.ancho / ancho;

  const alturas = new Float64Array(ancho * alto);
  for (let fila = 0; fila < alto; fila++) {
    const y = (fila + 0.5) * paso;
    for (let col = 0; col < ancho; col++) {
      alturas[fila * ancho + col] = evaluarElevacion(mapa.elevacion, { x: (col + 0.5) * paso, y }) * ESCALA_RELIEVE;
    }
  }

  const luzX = -0.5;
  const luzY = -0.5;
  const luzZ = Math.SQRT1_2;
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
      const factor = Math.max(0.35, lambert / lambertPlano);

      const i = (actual + col) * 4;
      datos[i] = Math.min(255, datos[i]! * factor);
      datos[i + 1] = Math.min(255, datos[i + 1]! * factor);
      datos[i + 2] = Math.min(255, datos[i + 2]! * factor);
    }
  }
  ctx.putImageData(img, 0, 0);
}

// --- RENDER PRINCIPAL DEL MAPA DEL MUNDO ---

export function pintarTerreno(
  ctx: CanvasRenderingContext2D, 
  mapa: MapaGenerado, 
  proyeccion: ProyeccionJugador,
  escalaCanvas: number
): void {
  const width = mapa.config.ancho * escalaCanvas;
  const height = mapa.config.alto * escalaCanvas;

  // 1. DIBUJAR TERRENO BASE
  ctx.clearRect(0, 0, width, height);
  ctx.imageSmoothingEnabled = true;
  ctx.imageSmoothingQuality = 'high';
  ctx.drawImage(pintarBiomas(mapa), 0, 0, width, height);
  aplicarSombreado(ctx, ctx.canvas, mapa);

  // 2. DIBUJAR BOSQUES
  // Bosques como silueta fusionada (no círculos sueltos), debajo de los ríos — mismo criterio de capas que
  // el cliente de administración. `nonzero` (el relleno por defecto de canvas) recorta solos los claros que
  // queden encerrados por una corona de bosques, porque `contornosBosques` los devuelve como lazo opuesto.
  ctx.fillStyle = 'rgba(27, 94, 32, 0.55)';
  const lazosBosque = contornosBosques(mapa.bosques, mapa.config.ancho);
  if (lazosBosque.length > 0) {
    ctx.beginPath();
    for (const lazo of lazosBosque) {
      if (lazo.length < 3) continue;
      ctx.moveTo(lazo[0]!.x * escalaCanvas, lazo[0]!.y * escalaCanvas);
      for (const p of lazo.slice(1)) ctx.lineTo(p.x * escalaCanvas, p.y * escalaCanvas);
      ctx.closePath();
    }
    ctx.fill('nonzero');
  }

  // Árboles con semilla determinista para conservar el detalle visual y la densidad de cada bosque.
  ctx.fillStyle = 'rgba(22, 62, 24, 0.9)';
  for (const bosque of mapa.bosques) {
    const cx = bosque.centro.x * escalaCanvas;
    const cy = bosque.centro.y * escalaCanvas;
    const radioPx = bosque.radio * escalaCanvas;
    const rand = mulberry32(hashSemilla(bosque.id));
    const numArboles = Math.round(Math.min(60, Math.max(4, (radioPx * radioPx * bosque.densidad) / 22)));
    for (let i = 0; i < numArboles; i++) {
      const angulo = rand() * Math.PI * 2;
      const r = radioPx * Math.sqrt(rand());
      dibujarGlifoArbol(ctx, cx + Math.cos(angulo) * r, cy + Math.sin(angulo) * r, 2.2);
    }
  }

  // 3. DIBUJAR RÍOS
  ctx.strokeStyle = 'rgba(38, 90, 145, 0.9)';
  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';
  for (const rio of mapa.rios) {
    if (rio.puntos.length < 2) continue;
    ctx.lineWidth = rio.navegable ? 5 : 2.5;
    ctx.beginPath();
    rio.puntos.forEach((p, i) => {
      const x = p.x * escalaCanvas;
      const y = p.y * escalaCanvas;
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    });
    ctx.stroke();
  }

  // 4. DIBUJAR NODOS DE RECURSOS
  for (const nodo of mapa.nodos) {
    ctx.beginPath();
    ctx.arc(nodo.posicion.x * escalaCanvas, nodo.posicion.y * escalaCanvas, 3, 0, Math.PI * 2);
    ctx.fillStyle = RECURSO_COLOR[nodo.tipo] || '#fff';
    ctx.fill();
  }

  // 5. ZONAS DE INFLUENCIA FUSIONADAS
  for (const zona of proyeccion.zonasFusionadas || []) {
    if (zona.contornos.length === 0) continue;
    const color = faccionColor(zona.faccionId, proyeccion.facciones);
    ctx.beginPath();
    for (const contorno of zona.contornos) {
      contorno.forEach((p, i) => {
        const x = p.x * escalaCanvas;
        const y = p.y * escalaCanvas;
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      });
      ctx.closePath();
    }
    ctx.fillStyle = color + '33'; // 20% alpha hex
    ctx.fill();
    ctx.strokeStyle = color;
    ctx.lineWidth = 2;
    ctx.stroke();
  }

  // 6. CAMINOS COMERCIALES
  ctx.strokeStyle = 'rgba(139, 90, 43, 0.9)';
  ctx.lineWidth = 2.5;
  ctx.setLineDash([6, 4]);
  for (const camino of proyeccion.caminos || []) {
    if (camino.puntos.length < 2) continue;
    ctx.beginPath();
    camino.puntos.forEach((p, i) => {
      const x = p.x * escalaCanvas;
      const y = p.y * escalaCanvas;
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    });
    ctx.stroke();
  }
  ctx.setLineDash([]);

  // 7. EDIFICIOS DEL MAPA (Minas, Canteras...)
  const tamanoEdificio = 6;
  for (const asentamiento of proyeccion.asentamientos || []) {
    for (const edificio of asentamiento.edificios || []) {
      if ((edificio.ambito ?? 'asentamiento') !== 'mapa') continue;
      const x = edificio.posicion.x * escalaCanvas;
      const y = edificio.posicion.y * escalaCanvas;
      const color = EDIFICIO_COLOR[edificio.tipo] || '#fff';
      
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

  // 8. ASENTAMIENTOS (Centros)
  for (const asentamiento of proyeccion.asentamientos || []) {
    const color = faccionColor(asentamiento.faccionId, proyeccion.facciones);
    ctx.beginPath();
    ctx.arc(asentamiento.posicion.x * escalaCanvas, asentamiento.posicion.y * escalaCanvas, 6, 0, Math.PI * 2);
    ctx.fillStyle = color;
    ctx.fill();
    ctx.strokeStyle = '#1b1a17';
    ctx.lineWidth = 1.5;
    ctx.stroke();
  }

  // 9. RUTAS EN TRÁNSITO (Rastros tenues)
  ctx.strokeStyle = 'rgba(241, 230, 200, 0.35)';
  ctx.lineWidth = 1.5;
  for (const caravana of proyeccion.caravanas || []) {
    if (!caravana.ruta || caravana.ruta.length < 2) continue;
    ctx.beginPath();
    caravana.ruta.forEach((p, i) => {
      const x = p.x * escalaCanvas;
      const y = p.y * escalaCanvas;
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    });
    ctx.stroke();
  }

  // 10. CARAVANAS (Triángulos identificables por color de origen)
  for (const caravana of proyeccion.caravanas || []) {
    const x = caravana.posicionActual.x * escalaCanvas;
    const y = caravana.posicionActual.y * escalaCanvas;
    
    const origenCaravana = proyeccion.asentamientos.find((a) => a.id === caravana.origenAsentamientoId);
    const color = origenCaravana ? faccionColor(origenCaravana.faccionId, proyeccion.facciones) : '#f1e6c8';
    
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

  // 11. CAMPAMENTOS DE BANDIDOS (Diamantes rojos)
  for (const campamento of proyeccion.campamentosBandidos || []) {
    const x = campamento.posicion.x * escalaCanvas;
    const y = campamento.posicion.y * escalaCanvas;
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

export function pintarPrevisualizacionFundacion(
  ctx: CanvasRenderingContext2D,
  mapa: MapaGenerado,
  posicion: Point,
  escalaCanvas: number
): void {
  const x = posicion.x * escalaCanvas;
  const y = posicion.y * escalaCanvas;
  const radio = 30 * escalaCanvas;

  ctx.save();
  ctx.beginPath();
  ctx.arc(x, y, radio, 0, Math.PI * 2);
  ctx.fillStyle = 'rgba(200, 170, 110, 0.18)';
  ctx.fill();
  ctx.setLineDash([8, 5]);
  ctx.strokeStyle = '#f1d38b';
  ctx.lineWidth = 2;
  ctx.stroke();
  ctx.setLineDash([]);
  ctx.beginPath();
  ctx.arc(x, y, 5, 0, Math.PI * 2);
  ctx.fillStyle = '#f1d38b';
  ctx.fill();
  ctx.strokeStyle = '#1b1a17';
  ctx.lineWidth = 1.5;
  ctx.stroke();

  const radioCuadrado = 30 * 30;
  for (const nodo of mapa.nodos) {
    const dx = nodo.posicion.x - posicion.x;
    const dy = nodo.posicion.y - posicion.y;
    if (dx * dx + dy * dy > radioCuadrado) continue;

    const nodoX = nodo.posicion.x * escalaCanvas;
    const nodoY = nodo.posicion.y * escalaCanvas;
    ctx.beginPath();
    ctx.arc(nodoX, nodoY, 8, 0, Math.PI * 2);
    ctx.fillStyle = 'rgba(255, 235, 150, 0.9)';
    ctx.fill();
    ctx.strokeStyle = '#5a3d16';
    ctx.lineWidth = 1.5;
    ctx.stroke();
    ctx.fillStyle = '#1b1a17';
    ctx.font = 'bold 11px system-ui, sans-serif';
    ctx.textAlign = 'left';
    ctx.textBaseline = 'middle';
    ctx.fillText(nodo.tipo, nodoX + 11, nodoY);
  }
  ctx.restore();
}

// --- RENDER DE LA VISTA DE ASENTAMIENTO (CIUDAD) ---

const BIOMA_TIERRA_PLANA = '#93c26b';

function dibujarTramos(
  ctx: CanvasRenderingContext2D,
  tramos: SegmentoTrazado[],
  aPantalla: (p: Point) => Point,
  color: string,
  grosor: number
): void {
  if (!tramos || tramos.length === 0) return;
  ctx.strokeStyle = color;
  ctx.lineWidth = grosor;
  ctx.lineCap = 'round';
  ctx.beginPath();
  for (const tramo of tramos) {
    const a = aPantalla(tramo.desde);
    const b = aPantalla(tramo.hasta);
    ctx.moveTo(a.x, a.y);
    ctx.lineTo(b.x, b.y);
  }
  ctx.stroke();
}

function dibujarEdificioLocal(
  ctx: CanvasRenderingContext2D,
  edificio: Edificio,
  huella: { x: number; y: number; ancho: number; alto: number }
): void {
  const color = EDIFICIO_COLOR[edificio.tipo] || '#999';
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

export function pintarAsentamiento(
  ctx: CanvasRenderingContext2D,
  asentamiento: Asentamiento,
  trazado: TrazadoAsentamiento | undefined
): void {
  const width = ctx.canvas.width;
  const height = ctx.canvas.height;

  ctx.clearRect(0, 0, width, height);

  // Fondo verde llano
  ctx.fillStyle = BIOMA_TIERRA_PLANA;
  ctx.fillRect(0, 0, width, height);

  const radioMapa = 60; // Constante estática
  const tamanoCelda = 5;
  const usable = width * 0.92;
  const escala = usable / (radioMapa * 2);
  const cx = width / 2;
  const cy = height / 2;
  const aPantalla = (p: Point): Point => ({ x: cx + p.x * escala, y: cy + p.y * escala });

  // Cuadrícula de fondo
  ctx.strokeStyle = 'rgba(27, 26, 23, 0.08)';
  ctx.lineWidth = 1;
  const pasoPantalla = tamanoCelda * escala;
  const celdasX = Math.ceil(cx / pasoPantalla) + 1;
  const celdasY = Math.ceil(cy / pasoPantalla) + 1;
  ctx.beginPath();
  for (let i = -celdasX; i <= celdasX; i++) {
    const x = cx + i * pasoPantalla;
    ctx.moveTo(x, 0);
    ctx.lineTo(x, height);
  }
  for (let j = -celdasY; j <= celdasY; j++) {
    const y = cy + j * pasoPantalla;
    ctx.moveTo(0, y);
    ctx.lineTo(width, y);
  }
  ctx.stroke();

  if (!trazado) {
    ctx.fillStyle = '#1b1a17';
    ctx.font = 'bold 15px system-ui, sans-serif';
    ctx.fillText(`Asentamiento (${asentamiento.id}) · Criterio T2a`, 12, height - 24);
    return;
  }

  // Trazados
  dibujarTramos(ctx, trazado.caminos, aPantalla, 'rgba(140, 118, 88, 0.55)', 1.5);
  dibujarTramos(ctx, trazado.calles, aPantalla, 'rgba(120, 92, 58, 0.75)', 3);

  // Edificios internos
  const internos = (asentamiento.edificios || []).filter((e) => (e.ambito ?? 'asentamiento') !== 'mapa');
  const enPantalla = (id: string) => {
    const huella = trazado.huellas[id];
    if (!huella) return null;
    const origen = aPantalla({ x: huella.x, y: huella.y });
    return { x: origen.x, y: origen.y, ancho: huella.ancho * escala, alto: huella.alto * escala };
  };

  for (const edificio of [...internos.filter((e) => e.tipo === 'vivienda'), ...internos.filter((e) => e.tipo !== 'vivienda')]) {
    const huella = enPantalla(edificio.id);
    if (huella) dibujarEdificioLocal(ctx, edificio, huella);
  }

  // Texto
  ctx.fillStyle = '#1b1a17';
  ctx.font = 'bold 15px system-ui, sans-serif';
  ctx.textAlign = 'left';
  ctx.textBaseline = 'bottom';
  ctx.fillText(`Asentamiento · Nivel ${asentamiento.nivel}`, 12, height - 24);
  ctx.font = '11px system-ui, sans-serif';
  ctx.fillStyle = '#4a4436';
  ctx.fillText('Vista de asentamiento (ciudad) · minas y cantera pertenecen a la región (mapa general)', 12, height - 8);
}
