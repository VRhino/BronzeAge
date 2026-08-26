// Pinta un `MapaGenerado` (Fase C11a) a un canvas usando SOLO `terreno/` — cero motor. Es la prueba de que
// C11b no hacía falta: el terreno se recalcula aquí, con la misma semilla pública que ve cualquiera.
import { colorDeBioma, evaluarBioma, type MapaGenerado } from './terreno';

/** Tamaño de celda de muestreo, en unidades de mapa — no 1:1 (2000×2000 = 4M muestras, demasiado para pintar
 * en un frame). 8 unidades da 250×250 = 62 500 muestras, suficiente para un mapa de estrategia a esta escala
 * y rápido de recalcular si la partida regenera el mundo (`regenerarMundo`, cambia `mapaId`). */
const TAMANO_CELDA = 8;

export function pintarTerreno(ctx: CanvasRenderingContext2D, mapa: MapaGenerado, escalaCanvas: number): void {
  const { ancho, alto } = mapa.config;
  for (let y = 0; y < alto; y += TAMANO_CELDA) {
    for (let x = 0; x < ancho; x += TAMANO_CELDA) {
      const bioma = evaluarBioma(mapa.elevacion, mapa.fertilidad, mapa.rios, { x, y });
      ctx.fillStyle = colorDeBioma(bioma);
      ctx.fillRect(x * escalaCanvas, y * escalaCanvas, TAMANO_CELDA * escalaCanvas, TAMANO_CELDA * escalaCanvas);
    }
  }

  // Ríos encima del terreno, como polilíneas — mismos datos (`mapa.rios`) que ya usa `evaluarBioma` para la
  // humedad, así que pintarlos no cuesta ninguna consulta nueva.
  ctx.strokeStyle = '#1a4d7a';
  ctx.lineWidth = Math.max(1, escalaCanvas * 3);
  for (const rio of mapa.rios) {
    if (rio.puntos.length < 2) continue;
    ctx.beginPath();
    ctx.moveTo(rio.puntos[0]!.x * escalaCanvas, rio.puntos[0]!.y * escalaCanvas);
    for (const p of rio.puntos.slice(1)) ctx.lineTo(p.x * escalaCanvas, p.y * escalaCanvas);
    ctx.stroke();
  }
}
