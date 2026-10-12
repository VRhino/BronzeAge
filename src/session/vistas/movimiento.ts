// Lo móvil en las fotos de un canal (03 §4.1): la `ruta` viaja una vez y luego solo `progreso` + `avancePorTick` (propios, que traen su
// ruta) o `posicionActual` + `desplazamientoPorTick` (ajenos, que no la traen). El avance de un tick no sale de ningún cálculo nuevo: es la
// diferencia con la foto anterior del mismo canal, que es lo que el cliente necesita para seguir moviéndose hasta el próximo delta.
import { SIMULACION } from '../../constants';
import { iguales, type Foto } from './diff';

export interface ColeccionesMoviles {
  /** Con `ruta` y `progreso` (0‥1 sobre la longitud de la polilínea): llevan `avancePorTick`. */
  propios: readonly string[];
  /** Solo con `posicionActual`: llevan `desplazamientoPorTick`. */
  ajenos: readonly string[];
}

interface Punto {
  x: number;
  y: number;
}
type Entidad = Record<string, unknown> & { id?: string };

const ES_NUMERO = (v: unknown): v is number => typeof v === 'number' && Number.isFinite(v);
const ES_PUNTO = (v: unknown): v is Punto => typeof v === 'object' && v !== null && ES_NUMERO((v as Punto).x) && ES_NUMERO((v as Punto).y);

/** Ticks de mundo entre dos instantes de foto (≥ 1 si el reloj avanzó). 0 si es el mismo instante: un comando entre dos ticks no mueve nada. */
export function ticksEntre(antes: unknown, despues: unknown): number {
  if (!ES_NUMERO(antes) || !ES_NUMERO(despues) || despues <= antes) return 0;
  return Math.max(1, Math.round((despues - antes) / SIMULACION.duracionTickMs));
}

function indexarPorId(coleccion: unknown): Map<string, Entidad> {
  const indice = new Map<string, Entidad>();
  if (Array.isArray(coleccion)) for (const e of coleccion as Entidad[]) if (typeof e.id === 'string') indice.set(e.id, e);
  return indice;
}

/**
 * La foto nueva con el movimiento anotado. No muta la foto de entrada (sus entidades pueden ser las del estado): las que llevan
 * anotación son copias. `anterior` es la foto del flush anterior de ESTE canal (con sus anotaciones); sin ella no hay de qué deducir
 * el avance y las claves quedan ausentes (el cliente no extrapola hasta el siguiente delta).
 */
export function anotarMovimiento(nueva: Foto, anterior: Foto | undefined, moviles: ColeccionesMoviles): Foto {
  if (!anterior) return nueva;
  const ticks = ticksEntre(anterior.instante, nueva.instante);
  const resultado: Foto = { ...nueva };

  for (const nombre of moviles.propios) {
    const previas = indexarPorId(anterior[nombre]);
    const coleccion = nueva[nombre];
    if (!Array.isArray(coleccion)) continue;
    resultado[nombre] = (coleccion as Entidad[]).map((e) => {
      const previa = typeof e.id === 'string' ? previas.get(e.id) : undefined;
      if (e.estado === 'estacionado') return { ...e, avancePorTick: 0 };
      if (!previa) return e;
      // El mismo instante no mueve nada: se hereda lo anotado. Con otra ruta el progreso no es comparable, y se espera al siguiente tick.
      if (ticks === 0) return previa.avancePorTick === undefined ? e : { ...e, avancePorTick: previa.avancePorTick };
      if (!ES_NUMERO(e.progreso) || !ES_NUMERO(previa.progreso) || !iguales(e.ruta, previa.ruta)) return e;
      return { ...e, avancePorTick: (e.progreso - previa.progreso) / ticks };
    });
  }

  for (const nombre of moviles.ajenos) {
    const previas = indexarPorId(anterior[nombre]);
    const coleccion = nueva[nombre];
    if (!Array.isArray(coleccion)) continue;
    resultado[nombre] = (coleccion as Entidad[]).map((e) => {
      const previa = typeof e.id === 'string' ? previas.get(e.id) : undefined;
      if (!previa) return e;
      if (ticks === 0) return previa.desplazamientoPorTick === undefined ? e : { ...e, desplazamientoPorTick: previa.desplazamientoPorTick };
      if (!ES_PUNTO(e.posicionActual) || !ES_PUNTO(previa.posicionActual)) return e;
      return { ...e, desplazamientoPorTick: { x: (e.posicionActual.x - previa.posicionActual.x) / ticks, y: (e.posicionActual.y - previa.posicionActual.y) / ticks } };
    });
  }
  return resultado;
}
