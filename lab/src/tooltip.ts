// Tooltip del Laboratorio: al pasar el cursor sobre un edificio, su ficha con la verdad DEL MOTOR.
//
// No importa el tooltip de `cliente/` (está acoplado a su `gameStore`: producción/consumo derivados en el
// cliente). Este muestra lo que el laboratorio necesita para depurar trazado: huella real, celda que ocupa,
// código de árbol si es ancla, y el estado de la próxima mejora (`estadoMejoraEdificio`) — que es el mismo
// dato que gobierna el botón de mejora manual de la pestaña de construcción.
import type { Asentamiento, Edificio } from '../../src/domain/types';
import { estadoMejoraEdificio } from '../../src/engine/construction';
import { celdaMinimaDeEdificio, tamanoDeEdificio } from '../../src/engine/trazado';
import { EDIFICIO_ETIQUETA } from './render';
import type { EstadoDibujoLab } from './render';
import { edificioEnPantalla } from './render';

const el = document.createElement('div');
el.id = 'lab-tooltip';
el.style.cssText = [
  'position:fixed',
  'z-index:50',
  'pointer-events:none',
  'max-width:280px',
  'background:#1b1a17',
  'border:1px solid #5a5343',
  'border-radius:4px',
  'padding:8px 10px',
  'font:12px/1.4 system-ui,sans-serif',
  'color:#e8e2d4',
  'box-shadow:0 4px 16px rgba(0,0,0,0.5)',
  'display:none',
].join(';');
document.body.appendChild(el);

/** Filas de árbol que ya calcula `debugAnclas` — para poner el código de ancla en la ficha sin recalcular. */
export interface CodigoAncla {
  id: string;
  codigo: string;
}

function ficha(asentamiento: Asentamiento, edificio: Edificio, codigoAncla: string | null): string {
  const min = celdaMinimaDeEdificio(edificio);
  const tam = tamanoDeEdificio(edificio);
  const nombre = EDIFICIO_ETIQUETA[edificio.tipo] ?? edificio.tipo;
  const nivel = edificio.nivelInterno ?? 1;

  const lineas: string[] = [
    `<b>${nombre}</b>  <span style="color:#a89f88">${edificio.id.slice(-16)}</span>`,
    `nivel interno ${nivel} · <span style="color:#a89f88">${edificio.estado}</span>`,
    `huella ${tam.ancho}×${tam.alto} celdas${edificio.rotado ? ' (rotado)' : ''}`,
    `celda mínima (${min.col}, ${min.row}) · centro (${edificio.posicion.x}, ${edificio.posicion.y})`,
  ];

  if (codigoAncla) lineas.push(`<span style="color:#ffd23f">ancla · código ${codigoAncla}</span>`);
  if (edificio.semillaSaturada) lineas.push('<span style="color:#c98">semilla saturada</span>');
  if (edificio.anclaLlena) lineas.push('<span style="color:#c98">ancla llena</span>');

  const piezas = asentamiento.edificios.filter(
    (e) =>
      (edificio.tipo === 'mercado' && e.tipo === 'puestoMercado') ||
      (edificio.tipo === 'carpinteria' && e.tipo === 'tallerCarpinteria')
  );
  if (edificio.tipo === 'mercado' || edificio.tipo === 'carpinteria') {
    lineas.push(`<span style="color:#a89f88">piezas dependientes: ${piezas.length}</span>`);
  }

  if (edificio.estado === 'activo') {
    const mejora = estadoMejoraEdificio(asentamiento, edificio, undefined);
    if (mejora) {
      const costo = Object.entries(mejora.costo)
        .map(([r, c]) => `${c} ${r}`)
        .join(', ');
      lineas.push(
        `<hr style="border:0;border-top:1px solid #4a4436;margin:5px 0">` +
          `<span style="color:${mejora.elegible ? '#9c6' : '#c88'}">` +
          `mejora → nivel ${mejora.nivelSiguiente}${costo ? ` (${costo})` : ''}` +
          `${mejora.elegible ? '' : ` · bloqueada`}</span>`
      );
    } else {
      lineas.push(`<span style="color:#a89f88">sin mejora disponible</span>`);
    }
  }

  return lineas.join('<br>');
}

/**
 * Wiring del tooltip: llamar `actualizar(ev)` en cada `mousemove` del canvas, `ocultar()` en `mouseleave`.
 * `estado()` devuelve el `EstadoDibujoLab` vigente (lo tiene la caché de `main.ts`); `codigoDe(id)` mapea un
 * edificio a su código de árbol si es ancla, o `null`.
 */
export function crearTooltip(
  canvas: HTMLCanvasElement,
  estado: () => EstadoDibujoLab | null,
  codigoDe: (id: string) => string | null
): { actualizar(ev: MouseEvent): string | null; ocultar(): void } {
  const ocultar = (): void => {
    el.style.display = 'none';
  };

  const actualizar = (ev: MouseEvent): string | null => {
    const est = estado();
    if (!est) {
      ocultar();
      return null;
    }
    const rect = canvas.getBoundingClientRect();
    const x = (ev.clientX - rect.left) * (canvas.width / rect.width);
    const y = (ev.clientY - rect.top) * (canvas.height / rect.height);
    const hit = edificioEnPantalla(canvas, est, x, y);
    if (!hit) {
      ocultar();
      return null;
    }
    const edificio = est.asentamiento.edificios.find((e) => e.id === hit.id);
    if (!edificio) {
      ocultar();
      return null;
    }
    el.innerHTML = ficha(est.asentamiento, edificio, codigoDe(hit.id));
    el.style.display = 'block';
    // Reposicionar tras medir, para no salirse de la ventana.
    const w = el.offsetWidth;
    const h = el.offsetHeight;
    el.style.left = `${Math.min(ev.clientX + 14, window.innerWidth - w - 8)}px`;
    el.style.top = `${Math.min(ev.clientY + 14, window.innerHeight - h - 8)}px`;
    return hit.id;
  };

  return { actualizar, ocultar };
}
