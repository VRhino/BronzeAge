// Refundar desde un campamento de mercenarios (Doc 1.9b, `Docs/Mecanicas a desarrollar.md` §40, paso 5): una Facción que se quedó sin
// asentamientos compra en el campamento una CARAVANA DE FUNDACIÓN al 75 % de lo que cuesta una normal, y la paga entre sus héroes: cada uno
// aporta lo que quiere a un fondo del campamento. La caravana nace sin destino; su titular la lleva enganchada y funda con `fundar` (D10).
import type { Asentamiento, CampamentoMercenarios, Caravana, Faccion, Heroe } from '../domain/types';
import { minutos, sumar, type Instante } from '../domain/tiempo';
import { MERCENARIOS } from '../constants';
import { totalAlmacenPersonal } from './almacenPersonal';
import { costoCaravanaFundacion } from './expansion';
import { calcularCapFundacion, esCiudadano } from './faccion';
import { MercenariosInvalidoError, campamentoDeResidente } from './mercenarios';

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
  // El oro sale primero del oro de botín (D27: el fondo es uno de sus dos destinos) y luego del almacén personal.
  const botin = recurso === 'oro' ? (heroe.oroDeBotin ?? 0) : 0;
  const guardado = heroe.almacenPersonal?.[recurso] ?? 0;
  const movido = Math.min(cantidad, botin + guardado);
  if (movido <= 0) throw new MercenariosInvalidoError(`No hay ${recurso} en el almacén personal.`);
  const deBotin = Math.min(botin, movido);

  return {
    campamentos: campamentos.map((c) =>
      c.id === campamento.id ? { ...c, fondos: { ...c.fondos, [heroeId]: { ...c.fondos[heroeId], [recurso]: (c.fondos[heroeId]?.[recurso] ?? 0) + movido } } } : c
    ),
    heroes: heroes.map((h) =>
      h.id === heroeId
        ? { ...h, ...(deBotin > 0 ? { oroDeBotin: botin - deBotin } : {}), almacenPersonal: sinCeros({ ...h.almacenPersonal, [recurso]: guardado - (movido - deBotin) }) }
        : h
    ),
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
  // El oro vuelve como oro de botín (D27): si no, aportar y retirar lavaría el botín hacia la economía de una plaza.
  const esOro = recurso === 'oro';
  const movido = Math.min(cantidad, aportado, esOro ? Infinity : Math.max(0, capacidadAlmacen - totalAlmacenPersonal(heroe)));
  if (movido <= 0) throw new MercenariosInvalidoError(aportado <= 0 ? `No has aportado ${recurso}.` : 'El almacén personal está lleno.');

  return {
    campamentos: campamentos.map((c) =>
      c.id === campamento.id ? { ...c, fondos: { ...c.fondos, [heroeId]: sinCeros({ ...c.fondos[heroeId], [recurso]: aportado - movido }) } } : c
    ),
    heroes: heroes.map((h) =>
      h.id !== heroeId
        ? h
        : esOro
          ? { ...h, oroDeBotin: (h.oroDeBotin ?? 0) + movido }
          : { ...h, almacenPersonal: { ...h.almacenPersonal, [recurso]: (h.almacenPersonal?.[recurso] ?? 0) + movido } }
    ),
    movido,
  };
}

/**
 * Comprar la Caravana de Fundación (Doc 1.9b, D9-D12, D34): el fondo de la Facción en el campamento tiene que cubrir `costoRefundacion`. Se
 * gasta de los aportes de sus ciudadanos por orden de ciudadanía hasta cubrirlo; lo que sobre de cada uno se queda en el fondo, y lo gastado
 * a cada uno queda apuntado en la caravana (`aportes`) para devolvérselo si caduca. Nace **sin destino**, parada en el campamento: su
 * titular (el que compra) la engancha a su columna y funda con `fundar` donde esté (D10, D11). Si nadie la lleva, caduca en
 * `MERCENARIOS.caducidadCaravanaHoras` (D14). Lleva la Facción en sí misma (D48): su origen es el campamento (D36).
 */
export function comprarCaravanaDeRefundacion(
  campamentos: readonly CampamentoMercenarios[],
  faccion: Faccion,
  asentamientos: readonly Asentamiento[],
  caravanasExistentes: readonly Caravana[],
  heroeId: string,
  instante: Instante,
  contador = 0
): { campamentos: CampamentoMercenarios[]; caravana: Caravana } {
  const campamento = campamentoDeResidente(campamentos, heroeId);
  if (!campamento) throw new MercenariosInvalidoError('Solo se compra en el campamento donde se reside.');
  if (!esCiudadano(faccion, heroeId)) throw new MercenariosInvalidoError('Solo compra un ciudadano de la Facción que refunda.');
  exigirFaccionSinAsentamientos(faccion, asentamientos);
  if (caravanasExistentes.some((c) => c.tipo === 'construccion' && c.faccionId === faccion.id)) {
    throw new MercenariosInvalidoError('La Facción ya tiene una Caravana de Fundación.');
  }
  const cap = calcularCapFundacion(faccion.nivel);
  if (cap < 1) throw new MercenariosInvalidoError(`Cap de fundación alcanzado (0/${cap} en nivel ${faccion.nivel}).`);

  const costo = costoRefundacion();
  const fondo = fondoDeFaccion(campamento, faccion);
  const falta = Object.entries(costo).filter(([recurso, n]) => (fondo[recurso] ?? 0) < n);
  if (falta.length > 0) {
    throw new MercenariosInvalidoError(`El fondo no cubre el coste: faltan ${falta.map(([r, n]) => `${Math.ceil(n - (fondo[r] ?? 0))} ${r}`).join(', ')}.`);
  }

  // Se gasta sobre el MISMO conjunto que cuenta `fondoDeFaccion` —los aportes de sus ciudadanos, residan o no—, por orden de
  // ciudadanía. Gastar por residentes dejaba sin descontar a quien aportó y se mudó, y tocaba aportes de otras Facciones.
  const fondos: Record<string, Record<string, number>> = {};
  const aportes: Record<string, Record<string, number>> = {};
  for (const [id, aporte] of Object.entries(campamento.fondos)) fondos[id] = { ...aporte };
  for (const [recurso, necesario] of Object.entries(costo)) {
    let pendiente = necesario;
    for (const id of faccion.ciudadanosIds) {
      const tiene = fondos[id]?.[recurso] ?? 0;
      if (tiene <= 0 || pendiente <= 0) continue;
      const toma = Math.min(tiene, pendiente);
      fondos[id]![recurso] = tiene - toma;
      aportes[id] = { ...aportes[id], [recurso]: (aportes[id]?.[recurso] ?? 0) + toma };
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
    estado: 'disponible',
    titularId: heroeId,
    aportes,
    caducaEn: sumar(instante, minutos(60 * MERCENARIOS.caducidadCaravanaHoras)),
  };
  return { campamentos: campamentos.map((c) => (c.id === campamento.id ? { ...c, fondos } : c)), caravana };
}
