import type { Asentamiento, EstadoAedasResidentes, Faccion, Heroe, RelacionPolitica, Titulo } from '../domain/types';
import type { EventoCrudo } from '../domain/eventos';

/** Fase A5 — payloads de los eventos de este subsistema (ver `narrarCambiosDeTitulo`). */
export interface PayloadTituloCambiaManos {
  tituloId: string;
  tituloNombre: string;
  previoFaccionId: string;
  actualFaccionId: string;
}
export interface PayloadTituloNace {
  tituloId: string;
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
  /** Dueños de las escuadras: la tropa de una Facción es la de sus ciudadanos, esté donde esté. */
  heroes: readonly Heroe[] = [],
  /** Los Aedas residentes: de ellos sale el título «Mecenas de los Aedas». */
  aedas?: EstadoAedasResidentes
): Titulo[] {
  if (facciones.length === 0) return [];
  const titulos: Titulo[] = [];

  const porFaccion = (f: Faccion) => asentamientos.filter((a) => a.faccionId === f.id);

  const masGrande = [...facciones].sort((a, b) => {
    const diff = porFaccion(b).length - porFaccion(a).length;
    if (diff !== 0) return diff;
    return porFaccion(b).reduce((acc, x) => acc + poblacionTotal(x), 0) - porFaccion(a).reduce((acc, x) => acc + poblacionTotal(x), 0);
  })[0]!;
  titulos.push({ tituloId: 'faccionMasGrande', nombre: 'Facción más grande', poseedorId: masGrande.id, valorMetrica: porFaccion(masGrande).length });

  const oroPorFaccion = (f: Faccion) => porFaccion(f).reduce((acc, a) => acc + cantidadDisponible(a.almacen, 'oro'), 0);
  const masRica = [...facciones].sort((a, b) => oroPorFaccion(b) - oroPorFaccion(a))[0]!;
  titulos.push({ tituloId: 'mayorPoderEconomico', nombre: 'Mayor poder económico', poseedorId: masRica.id, valorMetrica: oroPorFaccion(masRica) });

  // Toda la tropa de sus ciudadanos, esté donde esté (Doc 5.16.2): en el campamento, en campaña o de escolta.
  // Contar solo lo de casa haría que el título cambiara de manos cada vez que alguien marcha —y que los Aedas
  // narraran un `titulo.cambia_manos` que no ha ocurrido—.
  const tropasPorFaccion = (f: Faccion) =>
    heroes
      .filter((h) => f.ciudadanosIds.includes(h.id))
      .reduce((acc, h) => acc + h.escuadrones.reduce((suma, e) => suma + e.cantidad, 0), 0);
  const mayorEjercito = [...facciones].sort((a, b) => tropasPorFaccion(b) - tropasPorFaccion(a))[0]!;
  titulos.push({ tituloId: 'mayorEjercito', nombre: 'Ejército más grande', poseedorId: mayorEjercito.id, valorMetrica: tropasPorFaccion(mayorEjercito) });

  const granRey = computeLigas(relaciones, facciones).find((l) => l.granReyFaccionId)?.granReyFaccionId;
  if (granRey) titulos.push({ tituloId: 'granRey', nombre: 'Gran Rey', poseedorId: granRey, valorMetrica: 1 });

  // Mecenas de los Aedas: más épicas cumplidas, y a igualdad más Aedas residentes. Sin ninguna, nadie lo tiene.
  if (aedas) {
    const valor = (f: Faccion) => (aedas.cumplidas[f.id] ?? 0) * 10 + aedas.aedas.filter((a) => a.faccionId === f.id).length;
    const mecenas = facciones.reduce((mejor, f) => (valor(f) > valor(mejor) ? f : mejor));
    if (valor(mecenas) > 0) titulos.push({ tituloId: 'mecenasAedas', nombre: 'Mecenas de los Aedas', poseedorId: mecenas.id, valorMetrica: valor(mecenas) });
  }

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
          tituloId: actual.tituloId,
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
          tituloId: actual.tituloId,
          tituloNombre: actual.nombre,
          faccionId: actual.poseedorId,
          valorMetrica: actual.valorMetrica,
        } satisfies PayloadTituloNace,
      });
    }
  }
  return eventos;
}
