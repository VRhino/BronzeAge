// Vista árbol del Laboratorio (fuera del ámbito del juego general): diagrama abstracto del árbol único de
// anclas (Etapa 5, Lógica 1) — posiciones de layout, NO coordenadas del asentamiento, para que la estructura
// se lea sin que la geometría real la aplaste. Consume las mismas `FilaAncla` que ya arma `debugAnclas.ts`
// (`padreId`/`codigo`/`nivel` incluidos), sin volver a tocar el motor.
import type { FilaAncla } from './debugAnclas';

export interface NodoLayout {
  id: string;
  x: number;
  y: number;
}

export interface LayoutArbol {
  nodos: Map<string, NodoLayout>;
  ancho: number;
  alto: number;
}

const ESPACIO_X = 26;
const ESPACIO_Y = 46;
const MARGEN = 20;

/**
 * Layout jerárquico simple: cada nivel es una fila (y = nivel × `ESPACIO_Y`); un nodo hoja ocupa la próxima
 * columna libre, un nodo con hijos se centra sobre el promedio de sus hijos (recorrido post-orden) — mismo
 * criterio que un árbol de directorios dibujado, así dos hermanos de la misma semilla (el caso que motivó
 * esta vista) quedan visualmente uno al lado del otro colgando del mismo padre.
 */
export function calcularLayoutArbol(filas: FilaAncla[]): LayoutArbol {
  const porId = new Map(filas.map((f) => [f.id, f]));
  const hijosDe = new Map<string, string[]>();
  const raices: string[] = [];
  for (const f of filas) {
    if (f.padreId && porId.has(f.padreId)) {
      const lista = hijosDe.get(f.padreId);
      if (lista) lista.push(f.id);
      else hijosDe.set(f.padreId, [f.id]);
    } else {
      raices.push(f.id);
    }
  }
  const porCodigo = (a: string, b: string): number => {
    const ca = porId.get(a)?.codigo ?? '';
    const cb = porId.get(b)?.codigo ?? '';
    return ca < cb ? -1 : ca > cb ? 1 : 0;
  };
  for (const lista of hijosDe.values()) lista.sort(porCodigo);
  raices.sort(porCodigo);

  const nodos = new Map<string, NodoLayout>();
  let siguienteColumna = 0;

  function ubicar(id: string): number {
    const hijos = hijosDe.get(id) ?? [];
    const nivel = porId.get(id)?.nivel ?? 0;
    const x = hijos.length === 0 ? siguienteColumna++ : hijos.map(ubicar).reduce((a, b) => a + b, 0) / hijos.length;
    nodos.set(id, { id, x: MARGEN + x * ESPACIO_X, y: MARGEN + nivel * ESPACIO_Y });
    return x;
  }
  for (const raiz of raices) ubicar(raiz);

  let maxX = 0;
  let maxY = 0;
  for (const nodo of nodos.values()) {
    if (nodo.x > maxX) maxX = nodo.x;
    if (nodo.y > maxY) maxY = nodo.y;
  }
  return { nodos, ancho: maxX + MARGEN * 2, alto: maxY + MARGEN * 2 };
}

const COLOR_HUERFANA = '#ff4d4d';
const COLOR_SEMILLA_ACTIVA = '#ffd23f';
const COLOR_SEMILLA_SATURADA = '#8a6a4a';
const COLOR_NORMAL = '#7fae63';

/** Dibuja el diagrama completo — nodos coloreados por el mismo estado que ya muestra la tabla/overlay del
 * mapa (huérfana > semilla activa > semilla saturada > normal), aristas padre-hijo, y un resaltado blanco
 * sobre `hoveredId` (resaltado cruzado con el mapa, ver `main.ts`). */
export function dibujarArbol(ctx: CanvasRenderingContext2D, canvas: HTMLCanvasElement, filas: FilaAncla[], layout: LayoutArbol, hoveredId: string | null): void {
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  ctx.fillStyle = '#1b1a17';
  ctx.fillRect(0, 0, canvas.width, canvas.height);

  const porId = new Map(filas.map((f) => [f.id, f]));

  ctx.strokeStyle = 'rgba(168, 159, 136, 0.35)';
  ctx.lineWidth = 1;
  ctx.beginPath();
  for (const fila of filas) {
    if (!fila.padreId) continue;
    const nodo = layout.nodos.get(fila.id);
    const padre = layout.nodos.get(fila.padreId);
    if (!nodo || !padre) continue;
    ctx.moveTo(padre.x, padre.y);
    ctx.lineTo(nodo.x, nodo.y);
  }
  ctx.stroke();

  for (const fila of filas) {
    const nodo = layout.nodos.get(fila.id);
    if (!nodo) continue;
    const esHover = fila.id === hoveredId;
    const color = fila.huerfana ? COLOR_HUERFANA : fila.esSemillaActiva ? COLOR_SEMILLA_ACTIVA : fila.semillaSaturada ? COLOR_SEMILLA_SATURADA : COLOR_NORMAL;
    ctx.beginPath();
    ctx.arc(nodo.x, nodo.y, esHover ? 6.5 : 4, 0, Math.PI * 2);
    ctx.fillStyle = color;
    ctx.fill();
    if (esHover) {
      ctx.strokeStyle = '#ffffff';
      ctx.lineWidth = 2;
      ctx.stroke();
    }
  }

  if (hoveredId) {
    const nodo = layout.nodos.get(hoveredId);
    const fila = porId.get(hoveredId);
    if (nodo && fila) {
      ctx.fillStyle = '#e8e2d0';
      ctx.font = 'bold 11px system-ui, sans-serif';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'top';
      ctx.fillText(`${fila.codigo} · ${fila.tipo}`, Math.min(Math.max(nodo.x, 60), canvas.width - 60), nodo.y + 10);
      ctx.textAlign = 'left';
      ctx.textBaseline = 'alphabetic';
    }
  }
}

/** Nodo bajo el cursor en el canvas de la vista árbol (coords ya en el sistema de `layout`, sin transformar —
 * el canvas de árbol se dibuja 1:1, sin escala como el del mapa). */
export function nodoEnPosicion(layout: LayoutArbol, x: number, y: number, radioPx = 8): string | null {
  let mejor: { id: string; distancia: number } | null = null;
  for (const nodo of layout.nodos.values()) {
    const distancia = Math.hypot(nodo.x - x, nodo.y - y);
    if (distancia <= radioPx && (!mejor || distancia < mejor.distancia)) mejor = { id: nodo.id, distancia };
  }
  return mejor?.id ?? null;
}
