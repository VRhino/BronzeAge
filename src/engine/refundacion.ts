// Refundar desde un campamento de mercenarios (Doc 1.9b, `Docs/Mecanicas a desarrollar.md` §40, paso 5): una Facción que se quedó sin
// asentamientos compra en el campamento una CARAVANA DE FUNDACIÓN al 75 % de lo que cuesta una normal, y la paga entre sus héroes: cada uno
// aporta lo que quiere a un fondo del campamento. La caravana nace sin destino; su titular la lleva enganchada y funda con `fundar` (D10).
import type { Asentamiento, CampamentoMercenarios, Caravana, Ejercito, Faccion, Heroe } from '../domain/types';
import { capacidadCargaDe, racionQueQueda } from './ejercitos';
import { minutos, sumar, type Instante } from '../domain/tiempo';
import { ALMACEN_PERSONAL, FUNDACION, MERCENARIOS } from '../constants';
import { totalAlmacenPersonal } from './almacenPersonal';
import { costoCaravanaFundacion } from './expansion';
import { calcularCapFundacion, esCiudadano } from './faccion';
import { MercenariosInvalidoError } from './mercenarios';

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

/** De dónde sale o adónde va lo del fondo (D39): el almacén personal —que está en el campamento donde se reside— o el carro de la columna
 * que está en la puerta del campamento. Los materiales no viajan solos: para aportar en otro campamento hay que llevarlos en el carro. */
export type LadoDelFondo = 'almacen' | 'carro';

function exigirLado(campamento: CampamentoMercenarios, heroe: Heroe, columna: Ejercito | undefined, lado: LadoDelFondo): void {
  if (lado === 'almacen' && !campamento.residentesIds.includes(heroe.id)) {
    throw new MercenariosInvalidoError('El almacén personal está en el campamento donde se reside: aquí se aporta desde el carro.');
  }
  if (lado === 'carro' && (!columna || columna.liderId !== heroe.id)) throw new MercenariosInvalidoError('Hace falta tu columna en la puerta, y ser su Líder.');
}

/**
 * Un ciudadano aporta al fondo de refundación de su Facción en el campamento donde está (Doc 1.9b, D39): desde su almacén personal si
 * reside allí, o desde el carro de su columna en la puerta. El oro sale primero del oro de botín (D27: el fondo es uno de sus dos
 * destinos). La ración gratis del carro no se aporta (D50). Lo suyo se puede retirar (`retirarDeRefundacion`) mientras no se gaste.
 */
export function aportarARefundacion(
  campamento: CampamentoMercenarios,
  heroe: Heroe,
  columna: Ejercito | undefined,
  faccion: Faccion,
  asentamientos: readonly Asentamiento[],
  recurso: string,
  cantidad: number,
  desde: LadoDelFondo
): { campamento: CampamentoMercenarios; heroe: Heroe; columna: Ejercito | undefined; movido: number } {
  if (!esCiudadano(faccion, heroe.id)) throw new MercenariosInvalidoError('Solo aportan los ciudadanos de la Facción que refunda.');
  exigirFaccionSinAsentamientos(faccion, asentamientos);
  if (!(cantidad > 0)) throw new MercenariosInvalidoError('La cantidad tiene que ser positiva.');
  exigirLado(campamento, heroe, columna, desde);
  const botin = recurso === 'oro' && desde === 'almacen' ? (heroe.oroDeBotin ?? 0) : 0;
  const enCarro = columna ? (columna.suministro[recurso] ?? 0) - (recurso === 'trigo' ? racionQueQueda(columna) : 0) : 0;
  const disponible = desde === 'almacen' ? botin + (heroe.almacenPersonal?.[recurso] ?? 0) : enCarro;
  const movido = Math.min(cantidad, disponible);
  if (movido <= 0) throw new MercenariosInvalidoError(desde === 'almacen' ? `No hay ${recurso} en el almacén personal.` : `El carro no lleva ${recurso} que aportar.`);
  const deBotin = Math.min(botin, movido);
  const fondos = { ...campamento.fondos, [heroe.id]: { ...campamento.fondos[heroe.id], [recurso]: (campamento.fondos[heroe.id]?.[recurso] ?? 0) + movido } };

  if (desde === 'carro') {
    return { campamento: { ...campamento, fondos }, heroe, columna: { ...columna!, suministro: sinCeros({ ...columna!.suministro, [recurso]: columna!.suministro[recurso]! - movido }) }, movido };
  }
  const guardado = heroe.almacenPersonal?.[recurso] ?? 0;
  return {
    campamento: { ...campamento, fondos },
    heroe: { ...heroe, ...(deBotin > 0 ? { oroDeBotin: botin - deBotin } : {}), almacenPersonal: sinCeros({ ...heroe.almacenPersonal, [recurso]: guardado - (movido - deBotin) }) },
    columna,
    movido,
  };
}

/**
 * Un héroe retira lo suyo del fondo del campamento donde está, al almacén personal (si reside allí) o al carro de su columna, hasta donde
 * quepa. El oro vuelve siempre como oro de botín (D27): si no, aportar y retirar lavaría el botín hacia la economía de una plaza.
 */
export function retirarDeRefundacion(
  campamento: CampamentoMercenarios,
  heroe: Heroe,
  columna: Ejercito | undefined,
  recurso: string,
  cantidad: number,
  hacia: LadoDelFondo
): { campamento: CampamentoMercenarios; heroe: Heroe; columna: Ejercito | undefined; movido: number } {
  if (!(cantidad > 0)) throw new MercenariosInvalidoError('La cantidad tiene que ser positiva.');
  const esOro = recurso === 'oro';
  if (!esOro) exigirLado(campamento, heroe, columna, hacia);
  const aportado = campamento.fondos[heroe.id]?.[recurso] ?? 0;
  const sitio = esOro
    ? Infinity
    : hacia === 'almacen'
      ? ALMACEN_PERSONAL.capacidad - totalAlmacenPersonal(heroe)
      : capacidadCargaDe(columna!) - Object.values(columna!.suministro).reduce((a, b) => a + b, 0);
  const movido = Math.min(cantidad, aportado, Math.max(0, sitio));
  if (movido <= 0) throw new MercenariosInvalidoError(aportado <= 0 ? `No has aportado ${recurso}.` : 'No queda sitio donde guardarlo.');
  const fondos = { ...campamento.fondos, [heroe.id]: sinCeros({ ...campamento.fondos[heroe.id], [recurso]: aportado - movido }) };

  if (esOro) return { campamento: { ...campamento, fondos }, heroe: { ...heroe, oroDeBotin: (heroe.oroDeBotin ?? 0) + movido }, columna, movido };
  if (hacia === 'carro') {
    return { campamento: { ...campamento, fondos }, heroe, columna: { ...columna!, suministro: { ...columna!.suministro, [recurso]: (columna!.suministro[recurso] ?? 0) + movido } }, movido };
  }
  return { campamento: { ...campamento, fondos }, heroe: { ...heroe, almacenPersonal: { ...heroe.almacenPersonal, [recurso]: (heroe.almacenPersonal?.[recurso] ?? 0) + movido } }, columna, movido };
}

/**
 * Comprar la Caravana de Fundación (Doc 1.9b, D9-D12, D34): el fondo de la Facción en el campamento tiene que cubrir `costoRefundacion`. Se
 * gasta de los aportes de sus ciudadanos por orden de ciudadanía hasta cubrirlo; lo que sobre de cada uno se queda en el fondo, y lo gastado
 * a cada uno queda apuntado en la caravana (`aportes`) para devolvérselo si caduca. Nace **sin destino**, parada en el campamento: su
 * titular (el que compra) la engancha a su columna y funda con `fundar` donde esté (D10, D11). Si nadie la lleva, caduca en
 * `FUNDACION.caducidadCaravanaHoras` (D14). Lleva la Facción en sí misma (D48): su origen es el campamento (D36).
 */
export function comprarCaravanaDeRefundacion(
  campamento: CampamentoMercenarios,
  faccion: Faccion,
  asentamientos: readonly Asentamiento[],
  caravanasExistentes: readonly Caravana[],
  heroeId: string,
  instante: Instante,
  contador = 0
): { campamento: CampamentoMercenarios; caravana: Caravana } {
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
    caducaEn: sumar(instante, minutos(60 * FUNDACION.caducidadCaravanaHoras)),
  };
  return { campamento: { ...campamento, fondos }, caravana };
}
