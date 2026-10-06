// Sigilo de Facción (backend Doc 2.8.1): el dato son ids del catálogo y el dibujo es de este cliente.
// `catalogoSigilos.json` es copia de `src/contratos/v1/catalogoSigilos.json` del backend; los ids solo se añaden allí.
import catalogo from './catalogoSigilos.json';
import type { Sigilo } from '@motor/contratos/v1/dto';
import { svgEmblema } from './emblemas';
import insignias from './insignias.json';

export const CATALOGO_SIGILO = catalogo;

/** El sigilo de quien no tiene Facción, y el que se dibuja si una Facción viene sin él. */
export const SIGILO_NEUTRO: Sigilo = catalogo.reservados.neutro;

export function colorHex(colorId: string): string {
  return catalogo.colores.find((c) => c.id === colorId)?.hex ?? '#888888';
}

/** Siluetas del escudo en un lienzo de 100×120, con el centro y la escala del emblema que cabe dentro. */
const FORMAS: Record<string, { d: string; cx: number; cy: number; s: number }> = {
  clasico: { d: 'M4 4H96V70Q96 105 50 116Q4 105 4 70Z', cx: 50, cy: 52, s: 0.62 },
  aspis: { d: 'M2 60a48 48 0 1 0 96 0a48 48 0 1 0 -96 0Z', cx: 50, cy: 60, s: 0.62 },
  ocho: { d: 'M50 4C28 4 18 18 22 34C25 45 32 50 32 60C32 70 25 75 22 86C18 102 28 116 50 116C72 116 82 102 78 86C75 75 68 70 68 60C68 50 75 45 78 34C82 18 72 4 50 4Z', cx: 50, cy: 60, s: 0.38 },
  rombo: { d: 'M50 2L96 60L50 118L4 60Z', cx: 50, cy: 60, s: 0.42 },
  ovalo: { d: 'M50 2C82 2 96 34 96 62C96 92 76 118 50 118C24 118 4 92 4 62C4 34 18 2 50 2Z', cx: 50, cy: 60, s: 0.6 },
  torre: { d: 'M14 4H86L96 18V112H4V18Z', cx: 50, cy: 62, s: 0.62 },
};

/** Cuñas de un abanico que sale de abajo en el centro: el campo `radiante`. */
function radiante(s: string): string {
  const cunas: string[] = [];
  const punto = (i: number) => {
    const ang = Math.PI + (i * Math.PI) / 8;
    return `${(50 + 160 * Math.cos(ang)).toFixed(1)} ${(120 + 160 * Math.sin(ang)).toFixed(1)}`;
  };
  for (let k = 1; k < 8; k += 2) cunas.push(`<polygon points="50 120 ${punto(k)} ${punto(k + 1)}" fill="${s}"/>`);
  return cunas.join('');
}

/** Figuras del campo en un lienzo de 100×120, con `p` el color primario y `s` el secundario. */
function campo(campoId: string, p: string, s: string): string {
  const fondo = `<rect width="100" height="120" fill="${p}"/>`;
  const trazo = (d: string, w: number) => `<path d="${d}" fill="none" stroke="${s}" stroke-width="${w}" stroke-linejoin="round"/>`;
  switch (campoId) {
    case 'partido': return `${fondo}<rect x="50" width="50" height="120" fill="${s}"/>`;
    case 'cortado': return `${fondo}<rect y="60" width="100" height="60" fill="${s}"/>`;
    case 'cuartelado': return `${fondo}<rect x="50" width="50" height="60" fill="${s}"/><rect y="60" width="50" height="60" fill="${s}"/>`;
    case 'bandado': return `${fondo}<polygon points="0,20 0,50 100,100 100,70" fill="${s}"/>`;
    case 'palado': return `${fondo}<rect x="33" width="34" height="120" fill="${s}"/>`;
    case 'jefe': return `${fondo}<rect width="100" height="40" fill="${s}"/>`;
    case 'cruz': return `${fondo}<rect x="40" width="20" height="120" fill="${s}"/><rect y="45" width="100" height="20" fill="${s}"/>`;
    case 'aspa': return `${fondo}<polygon points="0,0 20,0 100,100 100,120 80,120 0,20" fill="${s}"/><polygon points="80,0 100,0 100,20 20,120 0,120 0,100" fill="${s}"/>`;
    case 'chevron': return `${fondo}<polygon points="0,70 50,30 100,70 100,95 50,55 0,95" fill="${s}"/>`;
    case 'bordura': return `${fondo}<rect x="9" y="9" width="82" height="102" fill="none" stroke="${s}" stroke-width="12"/>`;
    case 'losanjado': return `${fondo}<polygon points="50,5 95,60 50,115 5,60" fill="${s}"/>`;
    case 'bandas': return `${fondo}<rect y="24" width="100" height="24" fill="${s}"/><rect y="72" width="100" height="24" fill="${s}"/>`;
    case 'zigzag': return fondo + trazo('M0 40L12.5 28L25 40L37.5 28L50 40L62.5 28L75 40L87.5 28L100 40', 9) + trazo('M0 86L12.5 74L25 86L37.5 74L50 86L62.5 74L75 86L87.5 74L100 86', 9);
    case 'ondas': return fondo + trazo('M0 40Q12.5 24 25 40T50 40T75 40T100 40', 9) + trazo('M0 80Q12.5 64 25 80T50 80T75 80T100 80', 9);
    case 'greca': {
      const unidad = 'M0 24V0H18V20H6V8H12';
      return fondo + [0, 1, 2, 3, 4].map((i) => [30, 76].map((y) => `<g transform="translate(${i * 20} ${y})">${trazo(unidad, 4)}</g>`).join('')).join('');
    }
    case 'rombos': {
      const rombo = (cx: number, cy: number) => `<polygon points="${cx},${cy - 11} ${cx + 11},${cy} ${cx},${cy + 11} ${cx - 11},${cy}" fill="${s}"/>`;
      const filas: string[] = [];
      for (let fila = 0; fila < 5; fila++) {
        for (let col = 0; col < 5; col++) filas.push(rombo(col * 25 + (fila % 2 ? 0 : 12.5), fila * 26 + 6));
      }
      return fondo + filas.join('');
    }
    case 'radiante': return fondo + radiante(s);
    default: return fondo; // liso
  }
}

/** Marco opcional: se traza sobre el borde de la silueta (la mitad queda fuera del recorte). */
function orla(orlaId: string, forma: string, color: string): string {
  const base = `d="${forma}" fill="none" stroke="${color}"`;
  switch (orlaId) {
    case 'lisa': return `<path ${base} stroke-width="14"/>`;
    case 'greca': return `<path ${base} stroke-width="14" stroke-dasharray="11 5"/>`;
    case 'cuerda': return `<path ${base} stroke-width="14" stroke-dasharray="5 3" stroke-linecap="round"/>`;
    case 'puntos': return `<path ${base} stroke-width="12" stroke-dasharray="0 9" stroke-linecap="round"/>`;
    case 'dentada': return `<path ${base} stroke-width="16" stroke-dasharray="7 7"/>`;
    default: return '';
  }
}

/** Claro u oscuro según el color: el halo del emblema siempre contrasta con él. */
function haloPara(hex: string): string {
  const [r, g, b] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16));
  return 0.299 * r! + 0.587 * g! + 0.114 * b! > 140 ? '#1c1b1a' : '#f4efe4';
}

let contador = 0;

/** Corona dorada que el Gran Rey lleva sobre su estandarte (marco derivado, no se guarda). */
function corona(): string {
  const d = insignias.granRey.d;
  return `<g transform="translate(21 -30) scale(.1172)">`
    + `<path d="${d}" fill="#1c1b1a" stroke="#1c1b1a" stroke-width="30" stroke-linejoin="round"/>`
    + `<path d="${d}" fill="#e0b83a"/></g>`;
}

export interface OpcionesSigilo {
  /** Añade la corona del Gran Rey (Doc 2.8.1). */
  granRey?: boolean;
}

/** El sigilo como SVG en línea, de `ancho` píxeles de ancho. Sin sigilo, el neutro. */
export function svgSigilo(sigilo: Sigilo | undefined, ancho = 48, opciones: OpcionesSigilo = {}): string {
  const s = { ...SIGILO_NEUTRO, ...sigilo };
  const forma = FORMAS[s.formaId] ?? FORMAS.clasico!;
  const id = `sg${contador++}`;
  const relleno = colorHex(s.colorEmblemaId);
  const emblema = svgEmblema(s.emblemaId, relleno, haloPara(relleno));
  const escala = forma.s;
  const alto = opciones.granRey ? 150 : 120;
  return `<svg class="sigilo" width="${ancho}" height="${Math.round((ancho * alto) / 100)}" viewBox="0 ${opciones.granRey ? -30 : 0} 100 ${alto}" role="img" aria-label="${s.formaId} ${s.campoId} ${s.emblemaId}${opciones.granRey ? ' (Gran Rey)' : ''}">`
    + `<clipPath id="${id}"><path d="${forma.d}"/></clipPath>`
    + `<g clip-path="url(#${id})">${campo(s.campoId, colorHex(s.colorPrimarioId), colorHex(s.colorSecundarioId))}${orla(s.orlaId, forma.d, colorHex(s.colorOrlaId))}</g>`
    + `<path d="${forma.d}" fill="none" stroke="${opciones.granRey ? '#e0b83a' : '#0008'}" stroke-width="${opciones.granRey ? 5 : 3}"/>`
    + (emblema ? `<g transform="translate(${forma.cx - 50 * escala} ${forma.cy - 50 * escala}) scale(${escala})">${emblema}</g>` : '')
    + (opciones.granRey ? corona() : '')
    + '</svg>';
}

/** `doble_hacha` → `doble hacha`. */
export function nombreDeId(id: string): string {
  return id.replace(/_/g, ' ');
}

/** Los `<option>` de un selector: ids con su nombre. */
export function opciones(ids: readonly string[], elegido: string, nombre: (id: string) => string = nombreDeId): string {
  return ids.map((id) => `<option value="${id}"${id === elegido ? ' selected' : ''}>${nombre(id)}</option>`).join('');
}

/** Un sigilo al azar del catálogo (para ofrecer uno de partida al crear la Facción). */
export function sigiloAleatorio(): Sigilo {
  const al = <T>(lista: readonly T[]): T => lista[Math.floor(Math.random() * lista.length)]!;
  const primario = al(catalogo.colores).id;
  return {
    formaId: al(catalogo.formas),
    campoId: al(catalogo.campos),
    emblemaId: al(catalogo.emblemas),
    colorPrimarioId: primario,
    colorSecundarioId: al(catalogo.colores.filter((c) => c.id !== primario)).id,
    colorEmblemaId: al(catalogo.colores).id,
    orlaId: al(catalogo.orlas),
    colorOrlaId: al(catalogo.colores).id,
  };
}

/** El sigilo junto al nombre de su dueño (chips, cabeceras, listas). El texto va sin escapar: es el nombre del dominio. */
export function htmlNombreConSigilo(dueno: { nombre: string; sigilo?: Sigilo }, ancho = 20, opciones: OpcionesSigilo = {}): string {
  return `<span class="con-sigilo">${svgSigilo(dueno.sigilo, ancho, opciones)}<span>${dueno.nombre}</span></span>`;
}
