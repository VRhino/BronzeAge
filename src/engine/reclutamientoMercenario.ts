// Reclutar en un campamento de mercenarios (Doc 1.9b, `Docs/Mecanicas a desarrollar.md` §40, paso 3): solo las tropas que permiten
// sus edificios, con una tecnología PROPIA que desbloquea cada una el último, pagando oro y gastando la población del campamento.
// Es la puerta para recomponerse que una plaza no da: una Facción que no alcanza una tropa por edificio o por materiales la compra aquí.
import type { CampamentoMercenarios, EdificioCampamentoTipo, Ejercito, Escuadron, Faccion, Heroe, TecnologiaId, EstadoTecnologia } from '../domain/types';
import { MERCENARIOS, MOVIMIENTO, ORO_POR_CABALLO, RECLUTAMIENTO_ORO_POR_ESCALON, TECNOLOGIAS, TROPAS_RECLUTABLES } from '../constants';
import { minutos, transcurrido, type Instante } from '../domain/tiempo';
import { distancia } from '../world/geometria';
import { conEscuadrones } from './tropa';
import { puedeLlevar } from './liderazgo';
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
 * oro del carro; si no, el héroe con su almacén personal. El escuadrón nuevo nace en su campamento, **salvo que su Líder tenga la
 * columna a la puerta y le quede Liderazgo** (Doc 5.11, contando lo que ya lleva fuera): entonces se une a ella, para no obligarle
 * a entrar y volver a salir (`seUne`).
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
): { campamentos: CampamentoMercenarios[]; heroes: Heroe[]; ejercitos: Ejercito[]; cantidad: number; oro: number; seUne: boolean } {
  const campamento = campamentoDeResidente(campamentos, heroeId);
  if (!campamento) throw new MercenariosInvalidoError('Solo se recluta en el campamento donde se reside.');
  const heroe = heroes.find((h) => h.id === heroeId);
  if (!heroe) throw new MercenariosInvalidoError('Ese héroe no existe.');
  const tropa = tropasDelCampamento(campamento, adoptadas).find((t) => t.id === tropaId);
  if (!tropa) throw new MercenariosInvalidoError('Este campamento no ofrece esa tropa: le falta el edificio o la tecnología.');

  const existente = heroe.escuadrones.find((e) => e.tropaId === tropaId && !e.prestada);
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

  const nuevo: Escuadron | undefined = existente
    ? undefined
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
  // Se une a la columna que lidera a la puerta si cabe en su Liderazgo, junto a lo que ya tiene fuera del campamento.
  const aLaPuerta = ejercitosTras.find((e) => e.liderId === heroeId && distancia(e.posicionActual, campamento.posicion) <= MOVIMIENTO.radioPuerta);
  const yaFuera = heroe.escuadrones.filter((e) => e.contenedor.tipo !== 'campamento');
  const seUne = !!nuevo && !!aLaPuerta && puedeLlevar(heroe, [...yaFuera, nuevo]);
  if (seUne) ejercitosTras = ejercitosTras.map((e) => (e.id === aLaPuerta!.id ? { ...e, escuadronIds: [...e.escuadronIds, nuevo!.id] } : e));
  const escuadron: Escuadron = nuevo ? (seUne ? { ...nuevo, contenedor: { tipo: 'ejercito', ejercitoId: aLaPuerta!.id } } : nuevo) : { ...existente!, cantidad: existente!.cantidad + cantidad };

  return {
    campamentos: campamentos.map((c) => (c.id === campamento.id ? { ...c, poblacion: disponible - cantidad, poblacionEn: instante } : c)),
    heroes: conEscuadrones(heroes.map((h) => (h.id === heroeTras.id ? heroeTras : h)), [escuadron]),
    ejercitos: ejercitosTras,
    cantidad,
    oro,
    seUne,
  };
}

/** ¿Se repone aquí? En el campamento del héroe, o en su columna si está a la puerta del campamento. */
function reponibleAqui(e: Escuadron, campamento: CampamentoMercenarios, ejercitos: readonly Ejercito[]): boolean {
  const contenedor = e.contenedor;
  if (contenedor.tipo === 'campamento') return true;
  if (contenedor.tipo !== 'ejercito') return false;
  const columna = ejercitos.find((x) => x.id === contenedor.ejercitoId);
  return !!columna && distancia(columna.posicionActual, campamento.posicion) <= MOVIMIENTO.radioPuerta;
}

/**
 * Pedir tropa prestada al campamento donde reside (D25, D45, D80): una escuadra de 15 por cada tropa de leva comunal que elija —milicia de
 * lanceros, leñadores, granjeros; una, dos o las tres—, gratis, en su campamento. Sirve para aprender a usar tropa antes de tener la suya.
 * No es reclutar: la escuadra no es del héroe aunque la mande, y deja de estar disponible si deja de residir aquí.
 */
export function pedirPrestamo(campamentos: readonly CampamentoMercenarios[], heroe: Heroe, tropaIds: readonly string[]): Escuadron[] {
  const campamento = campamentoDeResidente(campamentos, heroe.id);
  if (!campamento) throw new MercenariosInvalidoError('Solo se presta tropa a quien reside en el campamento.');
  if (tropaIds.length === 0) throw new MercenariosInvalidoError('Elige al menos una tropa.');
  return [...new Set(tropaIds)].map((tropaId) => {
    const tropa = TROPAS_RECLUTABLES.find((t) => t.id === tropaId);
    if (!tropa || tropa.tecnologia !== 'leva_comunal') throw new MercenariosInvalidoError('Solo se presta leva comunal.');
    if (heroe.escuadrones.some((e) => e.prestada && e.tropaId === tropaId)) throw new MercenariosInvalidoError(`Ya tienes ${tropa.nombre} prestada.`);
    return {
      id: `prestada-${heroe.id}-${tropaId}`,
      nombre: `${tropa.nombre} (prestada por ${campamento.id})`,
      heroeId: heroe.id,
      origen: poblacionDeTropa(tropa),
      cantidad: MERCENARIOS.prestamo.unidades,
      ...PROGRESION_INICIAL,
      moral: 100,
      tropaId,
      contenedor: { tipo: 'campamento' },
      enGuarnicion: false,
      prestada: { campamentoId: campamento.id },
    };
  });
}

/** Reponer gratis la tropa prestada hasta su tamaño (D80), la que esté en el campamento o en la columna a su puerta. */
export function reponerPrestamo(campamentos: readonly CampamentoMercenarios[], heroe: Heroe, ejercitos: readonly Ejercito[]): { heroe: Heroe; repuestas: number } {
  const aqui = (e: Escuadron) => {
    const campamento = e.prestada && campamentos.find((c) => c.id === e.prestada!.campamentoId);
    return !!campamento && reponibleAqui(e, campamento, ejercitos) && e.cantidad < MERCENARIOS.prestamo.unidades;
  };
  const repuestas = heroe.escuadrones.filter(aqui).reduce((n, e) => n + MERCENARIOS.prestamo.unidades - e.cantidad, 0);
  if (repuestas === 0) throw new MercenariosInvalidoError('No hay tropa prestada que reponer aquí.');
  return { heroe: { ...heroe, escuadrones: heroe.escuadrones.map((e) => (aqui(e) ? { ...e, cantidad: MERCENARIOS.prestamo.unidades } : e)) }, repuestas };
}

/**
 * El campamento retira la tropa prestada a quien ya no reside en él (D45), esté donde esté: también de la columna o de la escolta donde
 * iba. Lo aplica el tick, que es por donde pasan todos los caminos de dejar de residir. Sin nada que retirar devuelve lo mismo.
 */
export function sinPrestamosAjenos<E extends { escuadronIds: string[] }, C extends { escoltaIds?: string[] }>(
  heroes: readonly Heroe[],
  ejercitos: readonly E[],
  caravanas: readonly C[],
  campamentos: readonly CampamentoMercenarios[]
): { heroes: Heroe[]; ejercitos: E[]; caravanas: C[] } {
  const caducada = (h: Heroe, e: Escuadron) => !!e.prestada && !campamentos.some((c) => c.id === e.prestada!.campamentoId && c.residentesIds.includes(h.id));
  const retiradas = new Set(heroes.flatMap((h) => h.escuadrones.filter((e) => caducada(h, e)).map((e) => e.id)));
  if (retiradas.size === 0) return { heroes: heroes as Heroe[], ejercitos: ejercitos as E[], caravanas: caravanas as C[] };
  return {
    heroes: heroes.map((h) => (h.escuadrones.some((e) => retiradas.has(e.id)) ? { ...h, escuadrones: h.escuadrones.filter((e) => !retiradas.has(e.id)) } : h)),
    ejercitos: ejercitos.map((e) => (e.escuadronIds.some((id) => retiradas.has(id)) ? { ...e, escuadronIds: e.escuadronIds.filter((id) => !retiradas.has(id)) } : e)),
    caravanas: caravanas.map((c) => (c.escoltaIds?.some((id) => retiradas.has(id)) ? { ...c, escoltaIds: c.escoltaIds.filter((id) => !retiradas.has(id)) } : c)),
  };
}
