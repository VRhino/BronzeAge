// Reclutar en un campamento de mercenarios (Doc 1.9b, `Docs/Mecanicas a desarrollar.md` §40, paso 3): solo las tropas que permiten
// sus edificios, con una tecnología PROPIA que desbloquea cada una el último, pagando oro y gastando la población del campamento.
// Es la puerta para recomponerse que una plaza no da: una Facción que no alcanza una tropa por edificio o por materiales la compra aquí.
import type { CampamentoMercenarios, EdificioCampamentoTipo, Ejercito, Escuadron, Faccion, Heroe, TecnologiaId, EstadoTecnologia } from '../domain/types';
import { MERCENARIOS, MOVIMIENTO, ORO_POR_CABALLO, RECLUTAMIENTO_ORO_POR_ESCALON, TECNOLOGIAS, TROPAS_RECLUTABLES } from '../constants';
import { minutos, transcurrido, type Instante } from '../domain/tiempo';
import { distancia } from '../world/geometria';
import { conEscuadrones } from './tropa';
import { MercenariosInvalidoError, campamentoDeResidente } from './mercenarios';
import { factorComisionPorReputacion } from './reputacion';
import { poblacionDeTropa, PROGRESION_INICIAL } from './tropas';
import { tecnologiasDe } from './tecnologia';

/** Cuántos reclutas caben a la vez: lo que dan sus viviendas, así que añadir viviendas al layout sube el tope solo. */
export const topePoblacion = (campamento: CampamentoMercenarios): number =>
  campamento.edificios.filter((e) => e === 'vivienda').length * MERCENARIOS.poblacionPorVivienda;

/** Los reclutas disponibles ahora: lo guardado más lo recuperado desde entonces, hasta el tope. Se calcula al mirar, no cada tick. */
export function poblacionActual(campamento: CampamentoMercenarios, instante: Instante): number {
  const horas = Math.max(0, transcurrido(campamento.poblacionEn, instante)) / minutos(60);
  return Math.min(topePoblacion(campamento), campamento.poblacion + horas * MERCENARIOS.poblacionPorHora);
}

/**
 * Las tecnologías que desbloquea el campamento (Doc 1.9b): cada una el ÚLTIMO, al instante y gratis, cuando la tiene al menos
 * `MERCENARIOS.tecnologia.porcentajeFacciones` de las Facciones humanas vivas o han pasado `horasTrasLaPrimera` desde que la primera
 * la desbloqueó, lo que llegue antes. Las de arranque (que toda Facción tiene) ya están. Una tecnología que nadie ha desbloqueado,
 * no. `humanasVivas`: las Facciones que no gobierna la IA y que conservan algún asentamiento.
 */
export function tecnologiasDelCampamento(tecnologia: EstadoTecnologia, humanasVivas: readonly Faccion[], instante: Instante): TecnologiaId[] {
  const plazo = minutos(60 * MERCENARIOS.tecnologia.horasTrasLaPrimera);
  return (Object.keys(TECNOLOGIAS) as TecnologiaId[]).filter((id) => {
    if (TECNOLOGIAS[id].deArranque) return true;
    const primera = tecnologia.primeros[id];
    if (!primera) return false;
    if (transcurrido(primera.en, instante) >= plazo) return true;
    if (humanasVivas.length === 0) return false;
    const la_tienen = humanasVivas.filter((f) => tecnologiasDe(tecnologia, f.id).adoptadas.includes(id)).length;
    return la_tienen / humanasVivas.length >= MERCENARIOS.tecnologia.porcentajeFacciones;
  });
}

/** El edificio del campamento que da esa tropa; el Centro Urbano no existe aquí, así que la leva de arranque no se recluta. */
const edificioDelCampamento = (edificio: (typeof TROPAS_RECLUTABLES)[number]['edificio']): EdificioCampamentoTipo | undefined =>
  edificio === 'centroUrbano' ? undefined : edificio === 'galeriaDeTiro' ? 'galeriaDeTiro' : edificio;

/** Las tropas que se pueden reclutar en este campamento con esas tecnologías: lo que permiten sus edificios, y desbloqueado. */
export function tropasDelCampamento(campamento: CampamentoMercenarios, adoptadas: readonly TecnologiaId[]): typeof TROPAS_RECLUTABLES {
  return TROPAS_RECLUTABLES.filter((t) => {
    const edificio = edificioDelCampamento(t.edificio);
    return edificio !== undefined && campamento.edificios.includes(edificio) && t.nivelRequerido <= MERCENARIOS.nivelEdificios && adoptadas.includes(t.tecnologia);
  });
}

/**
 * Lo que cuesta un soldado: el oro de su escalón y de sus caballos, más su equipo a `MERCENARIOS.valorEquipoEnOro` la unidad (el
 * campamento no recibe equipo fabricado: cobra todo en oro) → recargo del campamento → recargo por reputación de la Facción
 * (Doc 2.7) → descuento si esa Facción no tiene asentamientos.
 */
export function precioPorSoldado(tropa: (typeof TROPAS_RECLUTABLES)[number], faccion: Faccion | undefined, faccionConAsentamientos: boolean): number {
  const equipo = Object.values(tropa.costoEquipo).reduce<number>((a, n) => a + (n ?? 0), 0);
  const base = (RECLUTAMIENTO_ORO_POR_ESCALON[tropa.escalon] ?? 0) + (tropa.caballos ?? 0) * ORO_POR_CABALLO + equipo * MERCENARIOS.valorEquipoEnOro;
  const conRecargo = base * MERCENARIOS.recargo * (faccion ? factorComisionPorReputacion(faccion) : 1);
  return faccion && !faccionConAsentamientos ? conRecargo * MERCENARIOS.descuentoSinAsentamientos : conRecargo;
}

/** De dónde sale el oro: de su almacén personal o, con su columna a la puerta del campamento, del carro. */
export type PagarCon = 'almacenPersonal' | 'carro';

/**
 * Reclutar o reponer una tropa en el campamento donde reside el héroe (Doc 1.9b). Un escuadrón por tropa en toda la partida (Doc
 * 5.8): si ya lo tiene, repone hasta el tope. Con `pagarCon: 'carro'` paga el Líder de la columna que está a la puerta, con el
 * oro del carro; si no, el héroe con su almacén personal. El escuadrón nuevo nace en su campamento.
 */
export function reclutarEnCampamento(
  campamentos: readonly CampamentoMercenarios[],
  heroes: readonly Heroe[],
  ejercitos: readonly Ejercito[],
  heroeId: string,
  tropaId: string,
  adoptadas: readonly TecnologiaId[],
  faccion: Faccion | undefined,
  faccionConAsentamientos: boolean,
  pagarCon: PagarCon,
  instante: Instante,
  contador = 0
): { campamentos: CampamentoMercenarios[]; heroes: Heroe[]; ejercitos: Ejercito[]; cantidad: number; oro: number } {
  const campamento = campamentoDeResidente(campamentos, heroeId);
  if (!campamento) throw new MercenariosInvalidoError('Solo se recluta en el campamento donde se reside.');
  const heroe = heroes.find((h) => h.id === heroeId);
  if (!heroe) throw new MercenariosInvalidoError('Ese héroe no existe.');
  const tropa = tropasDelCampamento(campamento, adoptadas).find((t) => t.id === tropaId);
  if (!tropa) throw new MercenariosInvalidoError('Este campamento no ofrece esa tropa: le falta el edificio o la tecnología.');

  const existente = heroe.escuadrones.find((e) => e.tropaId === tropaId);
  if (existente && !reponibleAqui(existente, campamento, ejercitos)) {
    throw new MercenariosInvalidoError(`Ya tienes una escuadra de ${tropa.nombre}: solo puedes reponerla donde está.`);
  }
  const cantidad = tropa.unidadesPorDefecto - (existente?.cantidad ?? 0);
  if (cantidad <= 0) throw new MercenariosInvalidoError('Este escuadrón ya está al tope de unidades.');
  const disponible = poblacionActual(campamento, instante);
  if (disponible < cantidad) {
    throw new MercenariosInvalidoError(`El campamento no tiene reclutas suficientes (hacen falta ${cantidad}, hay ${Math.floor(disponible)}).`);
  }

  const oro = Math.ceil(precioPorSoldado(tropa, faccion, faccionConAsentamientos) * cantidad);
  let heroeTras = heroe;
  let ejercitosTras = [...ejercitos];
  if (pagarCon === 'carro') {
    const columna = ejercitos.find((e) => e.liderId === heroeId);
    if (!columna || distancia(columna.posicionActual, campamento.posicion) > MOVIMIENTO.radioPuerta) {
      throw new MercenariosInvalidoError('Para pagar con el carro hace falta ser el Líder de una columna a la puerta del campamento.');
    }
    if ((columna.suministro['oro'] ?? 0) < oro) throw new MercenariosInvalidoError(`No hay oro suficiente en el carro (hacen falta ${oro}).`);
    const suministro: Record<string, number> = { ...columna.suministro, oro: (columna.suministro['oro'] ?? 0) - oro };
    if (suministro['oro'] === 0) delete suministro['oro'];
    ejercitosTras = ejercitos.map((e) => (e.id === columna.id ? { ...e, suministro } : e));
  } else {
    const guardado = heroe.almacenPersonal?.['oro'] ?? 0;
    if (guardado < oro) throw new MercenariosInvalidoError(`No hay oro suficiente en el almacén personal (hacen falta ${oro}).`);
    const almacenPersonal: Record<string, number> = { ...heroe.almacenPersonal, oro: guardado - oro };
    if (almacenPersonal['oro'] === 0) delete almacenPersonal['oro'];
    heroeTras = { ...heroe, almacenPersonal };
  }

  const escuadron: Escuadron = existente
    ? { ...existente, cantidad: existente.cantidad + cantidad }
    : {
        id: `escuadron-${campamento.id}-${contador}`,
        nombre: `${tropa.nombre} de ${heroeId}`,
        heroeId,
        origen: poblacionDeTropa(tropa),
        cantidad,
        ...PROGRESION_INICIAL,
        moral: 100,
        tropaId,
        contenedor: { tipo: 'campamento' },
        enGuarnicion: false,
      };

  return {
    campamentos: campamentos.map((c) => (c.id === campamento.id ? { ...c, poblacion: disponible - cantidad, poblacionEn: instante } : c)),
    heroes: conEscuadrones(heroes.map((h) => (h.id === heroeTras.id ? heroeTras : h)), [escuadron]),
    ejercitos: ejercitosTras,
    cantidad,
    oro,
  };
}

/** ¿Se repone aquí? En el campamento del héroe, o en su columna si está a la puerta del campamento. */
function reponibleAqui(e: Escuadron, campamento: CampamentoMercenarios, ejercitos: readonly Ejercito[]): boolean {
  const contenedor = e.contenedor;
  if (contenedor.tipo === 'campamento') return true;
  if (contenedor.tipo === 'escolta') return false;
  const columna = ejercitos.find((x) => x.id === contenedor.ejercitoId);
  return !!columna && distancia(columna.posicionActual, campamento.posicion) <= MOVIMIENTO.radioPuerta;
}
