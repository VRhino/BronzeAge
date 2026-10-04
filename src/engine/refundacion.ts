// Refundar desde un campamento de mercenarios (Doc 1.9b, `Docs/Mecanicas a desarrollar.md` §40, paso 5): una Facción que se quedó sin
// asentamientos compra en el campamento una CARAVANA DE FUNDACIÓN al 75 % de lo que cuesta una normal, y la paga entre sus héroes: cada uno
// aporta lo que quiere desde su almacén personal a un fondo del campamento. La caravana sale del campamento y viaja y funda como cualquier otra
// (Doc 1.8). Fundar a pie en campo abierto no cambia: esto es la forma de recomponerse sin una plaza de la que partir.
import type { Asentamiento, CampamentoMercenarios, Caravana, Faccion, Heroe, Point } from '../domain/types';
import { FUNDACION, MERCENARIOS } from '../constants';
import type { Mapa } from '../world/mapa';
import { calcularRuta } from '../world/rutas';
import { totalAlmacenPersonal } from './almacenPersonal';
import { costoCaravanaFundacion } from './expansion';
import { calcularCapFundacion, esCiudadano } from './faccion';
import { MercenariosInvalidoError, campamentoDeResidente } from './mercenarios';
import { posicionLibreParaFundar } from './zones';

/** Lo que cuesta la caravana en el campamento: el coste normal de una Caravana de Fundación por `MERCENARIOS.refundacion.porcentajeCoste`. */
export function costoRefundacion(): Record<string, number> {
  return Object.fromEntries(
    Object.entries(costoCaravanaFundacion())
      .map(([recurso, cantidad]) => [recurso, Math.ceil((cantidad ?? 0) * MERCENARIOS.refundacion.porcentajeCoste)] as const)
      .filter(([, cantidad]) => cantidad > 0)
  );
}

const sinCeros = (r: Record<string, number>): Record<string, number> => Object.fromEntries(Object.entries(r).filter(([, n]) => n > 0));

/** El fondo de la Facción en un campamento: lo aportado por sus héroes (ciudadanos), por recurso. */
export function fondoDeFaccion(campamento: CampamentoMercenarios, faccion: Faccion): Record<string, number> {
  const total: Record<string, number> = {};
  for (const [heroeId, aporte] of Object.entries(campamento.fondos)) {
    if (!esCiudadano(faccion, heroeId)) continue;
    for (const [recurso, n] of Object.entries(aporte)) total[recurso] = (total[recurso] ?? 0) + n;
  }
  return total;
}

/** Solo una Facción sin ningún asentamiento refunda así: quien conserva una plaza expande con la Caravana de Fundación de siempre. */
function exigirFaccionSinAsentamientos(faccion: Faccion, asentamientos: readonly Asentamiento[]): void {
  if (asentamientos.some((a) => a.faccionId === faccion.id)) {
    throw new MercenariosInvalidoError('Solo una Facción sin asentamientos refunda desde un campamento.');
  }
}

/**
 * Un héroe aporta recursos de su almacén personal al fondo de refundación del campamento donde reside (Doc 1.9b). Cada héroe decide si
 * aporta y cuánto; lo suyo se puede retirar (`retirarDeRefundacion`) mientras no se haya gastado.
 */
export function aportarARefundacion(
  campamentos: readonly CampamentoMercenarios[],
  heroes: readonly Heroe[],
  faccion: Faccion,
  asentamientos: readonly Asentamiento[],
  heroeId: string,
  recurso: string,
  cantidad: number
): { campamentos: CampamentoMercenarios[]; heroes: Heroe[]; movido: number } {
  const campamento = campamentoDeResidente(campamentos, heroeId);
  if (!campamento) throw new MercenariosInvalidoError('Solo se aporta en el campamento donde se reside.');
  const heroe = heroes.find((h) => h.id === heroeId);
  if (!heroe) throw new MercenariosInvalidoError('Ese héroe no existe.');
  if (!esCiudadano(faccion, heroeId)) throw new MercenariosInvalidoError('Solo aportan los ciudadanos de la Facción que refunda.');
  exigirFaccionSinAsentamientos(faccion, asentamientos);
  if (!(cantidad > 0)) throw new MercenariosInvalidoError('La cantidad tiene que ser positiva.');
  const guardado = heroe.almacenPersonal?.[recurso] ?? 0;
  const movido = Math.min(cantidad, guardado);
  if (movido <= 0) throw new MercenariosInvalidoError(`No hay ${recurso} en el almacén personal.`);

  return {
    campamentos: campamentos.map((c) =>
      c.id === campamento.id ? { ...c, fondos: { ...c.fondos, [heroeId]: { ...c.fondos[heroeId], [recurso]: (c.fondos[heroeId]?.[recurso] ?? 0) + movido } } } : c
    ),
    heroes: heroes.map((h) => (h.id === heroeId ? { ...h, almacenPersonal: sinCeros({ ...h.almacenPersonal, [recurso]: guardado - movido }) } : h)),
    movido,
  };
}

/** Un héroe retira lo suyo del fondo, de vuelta a su almacén personal, hasta donde le quepa. */
export function retirarDeRefundacion(
  campamentos: readonly CampamentoMercenarios[],
  heroes: readonly Heroe[],
  heroeId: string,
  recurso: string,
  cantidad: number,
  capacidadAlmacen: number
): { campamentos: CampamentoMercenarios[]; heroes: Heroe[]; movido: number } {
  const campamento = campamentoDeResidente(campamentos, heroeId);
  if (!campamento) throw new MercenariosInvalidoError('Solo se retira en el campamento donde se reside.');
  const heroe = heroes.find((h) => h.id === heroeId);
  if (!heroe) throw new MercenariosInvalidoError('Ese héroe no existe.');
  if (!(cantidad > 0)) throw new MercenariosInvalidoError('La cantidad tiene que ser positiva.');
  const aportado = campamento.fondos[heroeId]?.[recurso] ?? 0;
  const movido = Math.min(cantidad, aportado, Math.max(0, capacidadAlmacen - totalAlmacenPersonal(heroe)));
  if (movido <= 0) throw new MercenariosInvalidoError(aportado <= 0 ? `No has aportado ${recurso}.` : 'El almacén personal está lleno.');

  return {
    campamentos: campamentos.map((c) =>
      c.id === campamento.id ? { ...c, fondos: { ...c.fondos, [heroeId]: sinCeros({ ...c.fondos[heroeId], [recurso]: aportado - movido }) } } : c
    ),
    heroes: heroes.map((h) => (h.id === heroeId ? { ...h, almacenPersonal: { ...h.almacenPersonal, [recurso]: (h.almacenPersonal?.[recurso] ?? 0) + movido } } : h)),
    movido,
  };
}

/**
 * Comprar la Caravana de Fundación (Doc 1.9b): el fondo de la Facción en el campamento tiene que cubrir `costoRefundacion`. Se gasta de los
 * aportes de sus ciudadanos por orden de ciudadanía hasta cubrirlo; lo que sobre de cada uno se queda en el fondo. La caravana sale del campamento hacia
 * `destino` con el que compra como fundador, y viaja y funda como cualquier otra (Doc 1.8): no deja el cupo hasta llegar, y se pierde si no
 * puede fundar al llegar. Lleva la Facción en sí misma porque no tiene asentamiento de origen del que sacarla.
 */
export function comprarCaravanaDeRefundacion(
  campamentos: readonly CampamentoMercenarios[],
  faccion: Faccion,
  asentamientos: readonly Asentamiento[],
  caravanasExistentes: readonly Caravana[],
  mapa: Mapa,
  heroeId: string,
  destino: Point,
  contador = 0
): { campamentos: CampamentoMercenarios[]; caravana: Caravana } {
  const campamento = campamentoDeResidente(campamentos, heroeId);
  if (!campamento) throw new MercenariosInvalidoError('Solo se compra en el campamento donde se reside.');
  if (!esCiudadano(faccion, heroeId)) throw new MercenariosInvalidoError('Solo compra un ciudadano de la Facción que refunda.');
  exigirFaccionSinAsentamientos(faccion, asentamientos);
  if (caravanasExistentes.some((c) => c.tipo === 'construccion' && c.faccionId === faccion.id)) {
    throw new MercenariosInvalidoError('La Facción ya tiene una Caravana de Fundación en camino.');
  }
  const cap = calcularCapFundacion(faccion.nivel);
  if (cap < 1) throw new MercenariosInvalidoError(`Cap de fundación alcanzado (0/${cap} en nivel ${faccion.nivel}).`);
  if (!posicionLibreParaFundar(destino, [...asentamientos])) throw new MercenariosInvalidoError('El destino cae dentro de una zona de influencia existente.');
  const ruta = calcularRuta(mapa, campamento.posicion, destino, { pasosRio: asentamientos.map((a) => a.posicion) });
  if (!ruta) throw new MercenariosInvalidoError('No hay ruta por tierra hasta ese punto de fundación: el agua no se cruza.');

  const costo = costoRefundacion();
  const fondo = fondoDeFaccion(campamento, faccion);
  const falta = Object.entries(costo).filter(([recurso, n]) => (fondo[recurso] ?? 0) < n);
  if (falta.length > 0) {
    throw new MercenariosInvalidoError(`El fondo no cubre el coste: faltan ${falta.map(([r, n]) => `${Math.ceil(n - (fondo[r] ?? 0))} ${r}`).join(', ')}.`);
  }

  // Se gasta sobre el MISMO conjunto que cuenta `fondoDeFaccion` —los aportes de sus ciudadanos, residan o no—, por orden de
  // ciudadanía. Gastar por residentes dejaba sin descontar a quien aportó y se mudó, y tocaba aportes de otras Facciones.
  const fondos: Record<string, Record<string, number>> = {};
  for (const [id, aporte] of Object.entries(campamento.fondos)) fondos[id] = { ...aporte };
  for (const [recurso, necesario] of Object.entries(costo)) {
    let pendiente = necesario;
    for (const id of faccion.ciudadanosIds) {
      const tiene = fondos[id]?.[recurso] ?? 0;
      if (tiene <= 0 || pendiente <= 0) continue;
      const toma = Math.min(tiene, pendiente);
      fondos[id]![recurso] = tiene - toma;
      pendiente -= toma;
    }
  }
  for (const id of Object.keys(fondos)) fondos[id] = sinCeros(fondos[id]!);

  const caravana: Caravana = {
    id: `caravana-fundacion-${campamento.id}-${contador}`,
    tipo: 'construccion',
    origenAsentamientoId: campamento.id,
    origenCampamentoId: campamento.id,
    faccionId: faccion.id,
    contenido: costo,
    posicionActual: campamento.posicion,
    progreso: 0,
    destinoPosicion: destino,
    heroesFundadoresIds: [heroeId].slice(0, FUNDACION.maxJugadoresFundacionGrupal),
    ruta,
  };
  return { campamentos: campamentos.map((c) => (c.id === campamento.id ? { ...c, fondos } : c)), caravana };
}
