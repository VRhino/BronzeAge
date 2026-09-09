// Tooltip del Laboratorio: al pasar el cursor sobre un edificio, su ficha con la verdad DEL MOTOR.
//
// No importa el tooltip de `cliente/` (está acoplado a su `gameStore`: producción/consumo derivados en el
// cliente). Este muestra lo que el laboratorio necesita para depurar trazado: huella real, celda que ocupa,
// código de árbol si es ancla, y el estado de la próxima mejora (`estadoMejoraEdificio`) — que es el mismo
// dato que gobierna el botón de mejora manual de la pestaña de construcción.
import type { Asentamiento, Edificio } from '../../src/domain/types';
import { estadoMejoraEdificio } from '../../src/engine/construction';
import { celdaMinimaDeEdificio, tamanoDeEdificio } from '../../src/engine/trazado';
import { costoDeTrazo, type CeldaMuro } from '../../src/engine/muralla';
import { EDIFICIO_ETIQUETA } from './render';
import type { EstadoDibujoLab } from './render';
import { celdaMurallaEnPantalla, edificioEnPantalla } from './render';
import type { ContextoMuralla } from './render';

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

const ETIQUETA_MURO: Record<CeldaMuro['clase'], string> = {
  muro: 'Muro',
  puerta: 'Puerta',
  torre: 'Torre',
};

/**
 * Ficha de una celda del anillo. Existe porque el anillo era lo único dibujado que no se podía interrogar con
 * el ratón, y el laboratorio está para inspeccionar: sin esto no hay forma de comprobar a mano por qué una
 * celda concreta salió puerta y no muro, que es exactamente el bug de las puertas de 4-5 celdas.
 */
function fichaMuralla(celda: CeldaMuro, indice: number, estado: EstadoDibujoLab, contexto: ContextoMuralla): string {
  const { total, puertas, torres, nivel, levantada, comprometido, areaEncerrada, dentro, fuera } = contexto;
  const costo = Object.entries(costoDeTrazo([celda], nivel))
    .map(([recurso, cantidad]) => `${cantidad} ${recurso}`)
    .join(', ');
  const color = celda.clase === 'puerta' ? '#d69e2e' : celda.clase === 'torre' ? '#c9c9c9' : '#a89f88';

  const lineas: string[] = [
    `<b style="color:${color}">${ETIQUETA_MURO[celda.clase]}</b>  <span style="color:#a89f88">muralla nivel ${nivel}</span>`,
    `celda (${celda.col}, ${celda.row})`,
  ];

  if (!comprometido) {
    lineas.push(`obra: celda ${indice + 1} de ${total} del recorrido`);
  } else if (levantada) {
    lineas.push(`<span style="color:#9c6">levantada</span> · celda ${indice + 1} de ${total} del recorrido`);
  } else {
    // Lo que evita la pregunta "¿por qué no puedo construir aquí, si está vacío?": esta celda YA ocupa suelo
    // desde que se comprometió el recinto, aunque todavía no esté en pie.
    lineas.push(
      `<span style="color:#e06">por construir</span> · celda ${indice + 1} de ${total} del recorrido`,
      `<span style="color:#a89f88">su suelo ya está ocupado: nadie puede edificar aquí</span>`
    );
  }
  lineas.push(`<span style="color:#a89f88">coste de esta celda: ${costo || 'gratis'}</span>`);

  if (celda.clase === 'puerta') {
    lineas.push(
      `<span style="color:#d69e2e">una puerta por cruce · ${puertas} en todo el recinto</span>`,
      `<span style="color:#a89f88">menos puertas = más defensa (embudo), pero los caminos rodean</span>`
    );
  }
  if (celda.clase === 'torre') lineas.push(`<span style="color:#a89f88">${torres} torres en el recinto</span>`);

  const pct = Math.round((estado.trazado.murallas[0]?.integridad ?? 0) * 100);
  const resumen = comprometido
    ? `recinto COMPROMETIDO · ${total} celdas · ${pct}% levantado`
    : `recinto PROPUESTO, sin comprometer · ${total} celdas · ${areaEncerrada} encerradas · dentro ${dentro} / arrabal ${fuera}`;
  lineas.push(`<hr style="border:0;border-top:1px solid #4a4436;margin:5px 0"><span style="color:#a89f88">${resumen}</span>`);
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
    const edificio = hit ? est.asentamiento.edificios.find((e) => e.id === hit.id) : undefined;
    if (edificio) {
      el.innerHTML = ficha(est.asentamiento, edificio, codigoDe(edificio.id));
    } else {
      // La muralla se consulta DESPUÉS del edificio: se dibuja encima, pero nunca se solapa con uno (el trazo
      // lo garantiza), así que el orden solo decide qué gana si el ratón cae en el borde de un píxel.
      const muro = celdaMurallaEnPantalla(canvas, est, x, y);
      if (!muro) {
        ocultar();
        return null;
      }
      el.innerHTML = fichaMuralla(muro.celda, muro.indice, est, muro.contexto);
    }
    el.style.display = 'block';
    // Reposicionar tras medir, para no salirse de la ventana.
    const w = el.offsetWidth;
    const h = el.offsetHeight;
    el.style.left = `${Math.min(ev.clientX + 14, window.innerWidth - w - 8)}px`;
    el.style.top = `${Math.min(ev.clientY + 14, window.innerHeight - h - 8)}px`;
    return edificio?.id ?? null;
  };

  return { actualizar, ocultar };
}
