// Fixtures compartidas por los tests de `engine/trade.ts` que NO montan una simulación completa: usan
// objetos SINTÉTICOS mínimos (solo los campos que la ruta de código bajo prueba realmente toca), a
// diferencia de `fixtures.ts` (que construye todo vía el motor real). Antes estaban clonadas casi byte a
// byte en varios archivos (revisión de duplicación 2026-08-25) — unificadas aquí.
import type { Asentamiento, Caravana, Chokepoint, Point, RecursoAlmacenado } from '../../domain/types';
import type { Mapa } from '../../world/mapa';

/** `Record<string, RecursoAlmacenado>` a partir de un mapa plano `{recurso: cantidad}`, con capacidad de
 * sobra (100000) para que ningún test tenga que preocuparse por el tope de almacenaje. */
export function almacenSintetico(recursos: Record<string, number>): Record<string, RecursoAlmacenado> {
  const out: Record<string, RecursoAlmacenado> = {};
  for (const [r, cantidad] of Object.entries(recursos)) out[r] = { cantidad, capacidad: 100000 };
  return out;
}

/**
 * `Mapa` sintético: solo implementa `costeEnPunto`/`listarChokepoints`/`limites`, los únicos métodos que
 * `avanzarComercio` toca en estos tests — no hace falta un mundo generado real (mismo patrón que
 * `world/__tests__/rutas.test.ts`). Terreno siempre llano (coste 1) y sin chokepoints salvo que se indiquen.
 */
export function mapaSintetico(opciones: { chokepoints?: Chokepoint[]; limites?: { ancho: number; alto: number } } = {}): Mapa {
  return {
    costeEnPunto: () => 1,
    listarChokepoints: () => opciones.chokepoints ?? [],
    limites: opciones.limites ?? { ancho: 2000, alto: 2000 },
  } as unknown as Mapa;
}

/**
 * Caravana comercial a un paso de llegar a `destino` — progreso 0.9999999: cualquier velocidad positiva
 * basta para completar el tramo que falta y disparar la llegada en el MISMO tick (ver
 * `avanzarPosicionEnRuta`, `engine/movimiento.ts`), sin depender del valor exacto de
 * `CARAVANA_CATALOGO.comercial.velocidad`. `posicionActual` se deja fija a `{x:999,y:0}` porque todos los
 * usos actuales colocan `destino` en `{x:1000,y:0}` — si algún test necesitara otra geometría, se pasa por
 * `overrides`.
 */
export function caravanaComercialCasiLlegando(origen: Asentamiento, destino: Asentamiento, overrides: Partial<Caravana> = {}): Caravana {
  return {
    id: 'caravana-1',
    tipo: 'comercial',
    origenAsentamientoId: origen.id,
    destinoAsentamientoId: destino.id,
    contenido: {},
    posicionActual: { x: 999, y: 0 } as Point,
    progreso: 0.9999999,
    estado: 'en_transito',
    ruta: [origen.posicion, destino.posicion],
    ...overrides,
  };
}
