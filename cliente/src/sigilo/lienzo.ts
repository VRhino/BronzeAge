// El sigilo como imagen para pintar en un canvas (`drawImage`): el SVG de `sigilo.ts` pasa por un `Image` con data URL. La carga es
// asíncrona, así que cada sigilo se carga una vez, se guarda, y `alCargar` avisa para repintar el mapa; mientras tanto no hay imagen
// y quien pinta usa su marcador de siempre.
import type { Sigilo } from '@motor/contratos/v1/dto';
import { svgSigilo } from './sigilo';

const imagenes = new Map<string, HTMLImageElement>();
const cargando = new Set<string>();
let alCargar: () => void = () => {};

/** Quién repinta cuando llega una imagen (la consola pone aquí su `render`). */
export function alCargarSigilos(callback: () => void): void {
  alCargar = callback;
}

/** La imagen del sigilo, o `undefined` si todavía se está cargando (en cuyo caso se repinta sola al llegar). `ancho`: píxeles de lienzo. */
export function imagenDeSigilo(sigilo: Sigilo, ancho = 44): HTMLImageElement | undefined {
  const clave = `${ancho}:${JSON.stringify(sigilo)}`;
  const lista = imagenes.get(clave);
  if (lista) return lista;
  if (cargando.has(clave)) return undefined;
  cargando.add(clave);
  // Un SVG suelto exige su espacio de nombres; en línea en el HTML no hace falta y `svgSigilo` no lo pone.
  const svg = svgSigilo(sigilo, ancho).replace('<svg ', '<svg xmlns="http://www.w3.org/2000/svg" ');
  const img = new Image();
  img.onload = () => {
    imagenes.set(clave, img);
    cargando.delete(clave);
    alCargar();
  };
  img.onerror = () => cargando.delete(clave);
  img.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
  return undefined;
}
