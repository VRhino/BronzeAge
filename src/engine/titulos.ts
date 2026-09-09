import type { Asentamiento, Ejercito, Faccion, RelacionPolitica, Titulo } from '../domain/types';
import type { EventoCrudo } from '../domain/eventos';

/** Fase A5 — payloads de los eventos de este subsistema (ver `narrarCambiosDeTitulo`). */
export interface PayloadTituloCambiaManos {
  tituloNombre: string;
  previoFaccionId: string;
  actualFaccionId: string;
}
export interface PayloadTituloNace {
  tituloNombre: string;
  faccionId: string;
  valorMetrica: number;
}
import { cantidadDisponible } from './almacen';
import { poblacionTotal } from './asentamientoQuery';
import { computeLigas } from './liga';

/**
 * Títulos dinámicos de PRESTIGIO (Doc 2.9): sin beneficio mecánico, recalculados periódicamente según poder
 * relativo — no en tiempo real (aquí: se recalculan a demanda cada tick, no en cada frame visual). La lista de
 * ejemplos del diseño es EXPLÍCITAMENTE ABIERTA; Fase 0 cubre los que se derivan limpio del estado ya modelado.
 * "General con más victorias" queda fuera: exigiría rastrear estadísticas por jugador individual, un nivel de
 * detalle que el modelo actual (centrado en Facción/Asentamiento) no lleva y que la lista abierta no obliga a cubrir.
 */
export function calcularTitulos(
  facciones: Faccion[],
  asentamientos: Asentamiento[],
  relaciones: RelacionPolitica[],
  ejercitos: readonly Ejercito[] = []
): Titulo[] {
  if (facciones.length === 0) return [];
  const titulos: Titulo[] = [];

  const porFaccion = (f: Faccion) => asentamientos.filter((a) => a.faccionId === f.id);

  const masGrande = [...facciones].sort((a, b) => {
    const diff = porFaccion(b).length - porFaccion(a).length;
    if (diff !== 0) return diff;
    return porFaccion(b).reduce((acc, x) => acc + poblacionTotal(x), 0) - porFaccion(a).reduce((acc, x) => acc + poblacionTotal(x), 0);
  })[0]!;
  titulos.push({ nombre: 'Facción más grande', poseedorId: masGrande.id, valorMetrica: porFaccion(masGrande).length });

  const oroPorFaccion = (f: Faccion) => porFaccion(f).reduce((acc, a) => acc + cantidadDisponible(a.almacen, 'oro'), 0);
  const masRica = [...facciones].sort((a, b) => oroPorFaccion(b) - oroPorFaccion(a))[0]!;
  titulos.push({ nombre: 'Mayor poder económico', poseedorId: masRica.id, valorMetrica: oroPorFaccion(masRica) });

  // Guarnición + campo (Doc 5.12): desde que los escuadrones se van DE VERDAD del asentamiento al salir de
  // campaña, contar solo `asentamiento.escuadrones` haría que el título cambiara de manos cada vez que
  // alguien marcha —y que los Aedas narraran un `titulo.cambia_manos` que no ha ocurrido—. El ejército no
  // desaparece por estar fuera de casa.
  const soldados = (escuadrones: readonly { cantidad: number }[]) => escuadrones.reduce((acc, e) => acc + e.cantidad, 0);
  const tropasPorFaccion = (f: Faccion) =>
    porFaccion(f).reduce((acc, a) => acc + soldados(a.escuadrones), 0) +
    ejercitos.filter((e) => e.faccionId === f.id).reduce((acc, e) => acc + soldados(e.escuadrones), 0);
  const mayorEjercito = [...facciones].sort((a, b) => tropasPorFaccion(b) - tropasPorFaccion(a))[0]!;
  titulos.push({ nombre: 'Ejército más grande', poseedorId: mayorEjercito.id, valorMetrica: tropasPorFaccion(mayorEjercito) });

  const granRey = computeLigas(relaciones, facciones).find((l) => l.granReyFaccionId)?.granReyFaccionId;
  if (granRey) titulos.push({ nombre: 'Gran Rey', poseedorId: granRey, valorMetrica: 1 });

  return titulos;
}

/** Narración de Aedas/Poetas (Doc 6.3): log de texto cuando un título cambia de manos. */
export function narrarCambiosDeTitulo(anteriores: Titulo[], actuales: Titulo[], facciones: Faccion[]): EventoCrudo[] {
  const nombreFaccion = (id: string) => facciones.find((f) => f.id === id)?.nombre ?? id;
  const eventos: EventoCrudo[] = [];
  for (const actual of actuales) {
    const previo = anteriores.find((t) => t.nombre === actual.nombre);
    if (previo && previo.poseedorId !== actual.poseedorId) {
      eventos.push({
        codigo: 'titulo.cambia_manos',
        mensaje: `Los Aedas cantan: el título "${actual.nombre}" pasa de ${nombreFaccion(previo.poseedorId)} a ${nombreFaccion(actual.poseedorId)}.`,
        payload: {
          tituloNombre: actual.nombre,
          previoFaccionId: previo.poseedorId,
          actualFaccionId: actual.poseedorId,
        } satisfies PayloadTituloCambiaManos,
      });
    } else if (!previo) {
      eventos.push({
        codigo: 'titulo.nace',
        mensaje: `Los Aedas cantan: nace el título "${actual.nombre}", ostentado por ${nombreFaccion(actual.poseedorId)}.`,
        payload: {
          tituloNombre: actual.nombre,
          faccionId: actual.poseedorId,
          valorMetrica: actual.valorMetrica,
        } satisfies PayloadTituloNace,
      });
    }
  }
  return eventos;
}
