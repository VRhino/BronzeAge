// La Caravana de Fundación (Doc 1.8): la única forma de fundar. La de un campamento de mercenarios y la de una plaza son la misma
// entidad: nace SIN destino, parada en su origen, con un titular que la engancha a su columna y funda con `fundar` donde esté.
// Sin titular que la lleve vuelve sola a su origen, y suelta caduca; al desarmarse o caducar devuelve lo que costó.
import type { Asentamiento, CampamentoMercenarios, Caravana, Ejercito, Faccion, Heroe, Point } from '../domain/types';
import type { EventoCrudo } from '../domain/eventos';
import { minutos, sumar, type Instante } from '../domain/tiempo';
import type { Mapa } from '../world/mapa';
import { distancia } from '../world/geometria';
import { calcularRuta } from '../world/rutas';
import { ALMACEN_PERSONAL, CARAVANA_CATALOGO, EDIFICIO_CATALOGO, FUNDACION, MERCENARIOS, MOVIMIENTO } from '../constants';
import { agregarRecurso, descontarRecursos, tieneRecursos } from './almacen';
import { totalAlmacenPersonal } from './almacenPersonal';
import { avanzarPosicionEnRuta } from './movimiento';
import { calcularCapFundacion, esCiudadano } from './faccion';
import { fundarAsentamiento, FundacionInvalidaError } from './settlement';
import { nivelActualDe, puedeCrearCaravana, cooldownCaravanaRestante } from './asentamientoQuery';
import { ReglaInvalidaError } from './errores';

export class ExpansionInvalidaError extends ReglaInvalidaError {}

/** Coste total de una Caravana de Fundación (Doc 1.8): materiales iniciales + edificios de arranque
 * (Centro Urbano no cuesta nada) + madera extra por fabricar la caravana en sí. */
export function costoCaravanaFundacion(): Partial<Record<string, number>> {
  const costo: Partial<Record<string, number>> = { ...FUNDACION.materialesIniciales };
  for (const [recurso, cantidad] of Object.entries(EDIFICIO_CATALOGO.granja.costo)) {
    costo[recurso] = (costo[recurso] ?? 0) + cantidad;
  }
  for (const [recurso, cantidad] of Object.entries(EDIFICIO_CATALOGO.vivienda.costo)) {
    costo[recurso] = (costo[recurso] ?? 0) + cantidad * FUNDACION.viviendasIniciales;
  }
  costo.madera = (costo.madera ?? 0) + FUNDACION.costoMaderaExtraCaravana;
  return costo;
}

/** Facción dueña de la caravana (D48): la que lleva puesta o, si no, la de su asentamiento de origen. Undefined si no lleva y el
 * origen ya no existe (asentamiento colapsado en tránsito). */
export function faccionDeCaravana(caravana: Caravana, asentamientos: readonly Asentamiento[]): string | undefined {
  // La comprada en un campamento de mercenarios (Doc 1.9b) no tiene asentamiento de origen: lleva su Facción.
  return caravana.faccionId ?? asentamientos.find((a) => a.id === caravana.origenAsentamientoId)?.faccionId;
}

/** Nº de "asentamientos efectivos" de una Facción a efectos del Cap de Fundación (Doc 1.7/1.8): los ya fundados MÁS sus Caravanas de
 * Fundación vivas (el cupo se reserva al lanzar, no al fundar). */
function asentamientosEfectivos(faccionId: string, asentamientos: readonly Asentamiento[], caravanas: readonly Caravana[]): number {
  const propios = asentamientos.filter((a) => a.faccionId === faccionId).length;
  const vivas = caravanas.filter((c) => c.tipo === 'construccion' && faccionDeCaravana(c, asentamientos) === faccionId).length;
  return propios + vivas;
}

/**
 * Lanza una Caravana de Fundación desde una plaza (Doc 1.8): nivel ≥ 2, cooldown de caravanas, cupo del Cap de Fundación de la Facción
 * (reservado desde ya) y el coste completo del almacén. Nace **sin destino**, parada en la plaza, con `titularId` como titular: él la
 * engancha a su columna y funda donde esté (`fundar`). Si nadie la lleva, caduca y devuelve el coste a la plaza.
 */
export function lanzarCaravanaFundacion(
  origen: Asentamiento,
  faccion: Faccion,
  titularId: string,
  asentamientosExistentes: readonly Asentamiento[],
  caravanasExistentes: readonly Caravana[],
  instante: Instante,
  contador = 0
): { origenActualizado: Asentamiento; caravana: Caravana } {
  if (origen.faccionId !== faccion.id) {
    throw new ExpansionInvalidaError('El asentamiento de origen no pertenece a esta Facción.');
  }
  if (!esCiudadano(faccion, titularId)) {
    throw new ExpansionInvalidaError('Solo un ciudadano de la Facción lleva su Caravana de Fundación.');
  }
  // nivelActual (Doc Fase_0_5 §6.2), no nivelAlcanzado: un origen degradado por debajo de nivel 2 no puede
  // lanzar una Caravana de Fundación hasta recuperarse, aunque haya llegado a nivel 2 alguna vez.
  if (nivelActualDe(origen) < 2) {
    throw new ExpansionInvalidaError('El asentamiento de origen debe estar en nivel 2 como mínimo para lanzar una Caravana de Fundación.');
  }
  if (!puedeCrearCaravana(origen, instante)) {
    throw new ExpansionInvalidaError(
      `Cooldown de creación de caravanas: faltan ~${Math.round(cooldownCaravanaRestante(origen, instante) / 60_000)} min para poder crear otra desde este asentamiento.`
    );
  }
  const cap = calcularCapFundacion(faccion.nivel);
  const efectivos = asentamientosEfectivos(faccion.id, asentamientosExistentes, caravanasExistentes);
  if (efectivos >= cap) {
    throw new ExpansionInvalidaError(`Cap de fundación alcanzado (${efectivos}/${cap} en nivel ${faccion.nivel}, incluyendo caravanas ya lanzadas).`);
  }
  const costo = costoCaravanaFundacion();
  if (!tieneRecursos(origen.almacen, costo)) {
    throw new ExpansionInvalidaError('El asentamiento de origen no tiene recursos suficientes para la Caravana de Fundación.');
  }

  const caravana: Caravana = {
    id: `caravana-fundacion-${origen.id}-${contador}`,
    tipo: 'construccion',
    origenAsentamientoId: origen.id,
    faccionId: faccion.id,
    contenido: costo as Record<string, number>,
    posicionActual: origen.posicion,
    progreso: 0,
    estado: 'disponible',
    titularId,
    caducaEn: sumar(instante, minutos(60 * FUNDACION.caducidadCaravanaHoras)),
  };
  return {
    origenActualizado: { ...origen, almacen: descontarRecursos(origen.almacen, costo), ultimaCaravanaCreadaEn: instante },
    caravana,
  };
}

/** Lo que cambia al devolver una Caravana de Fundación a quien la pagó. */
export interface MundoDeDevolucion {
  asentamientos: Asentamiento[];
  heroes: Heroe[];
}

/**
 * Devuelve lo que costó la caravana (D14, D34, D43, D68): la de un campamento, a cada aportante según su registro (`devolverAportes`); la
 * de una plaza, íntegra al almacén de su origen, hasta donde quepa. Si el origen ya no existe, se pierde.
 */
export function devolverCaravana(caravana: Caravana, mundo: MundoDeDevolucion, facciones: readonly Faccion[]): MundoDeDevolucion {
  if (caravana.origenCampamentoId) return { ...mundo, heroes: devolverAportes(caravana, mundo.heroes, facciones) };
  return {
    ...mundo,
    asentamientos: mundo.asentamientos.map((a) => {
      if (a.id !== caravana.origenAsentamientoId) return a;
      let almacen = a.almacen;
      for (const [recurso, cantidad] of Object.entries(caravana.contenido)) if (cantidad > 0) almacen = agregarRecurso(almacen, recurso, cantidad);
      return { ...a, almacen };
    }),
  };
}

/**
 * Desarma a mano una Caravana de Fundación (la de un campamento o la de una plaza) y devuelve lo que costó. Solo parada en su origen, a
 * su puerta: si no, desarmarla sería llevarse el coste de un sitio a otro sin recorrer el camino (la que anda sola o caduca ya lo
 * devuelve donde le toca).
 */
export function desarmarCaravanaFundacion(caravana: Caravana, posicionOrigen: Point | undefined, mundo: MundoDeDevolucion, facciones: readonly Faccion[]): MundoDeDevolucion {
  if (caravana.tipo !== 'construccion') throw new ExpansionInvalidaError('Esa caravana no es una Caravana de Fundación.');
  if (caravana.estado !== 'disponible') throw new ExpansionInvalidaError('Solo se desarma una Caravana de Fundación suelta: desengánchala primero.');
  if (!posicionOrigen || distancia(caravana.posicionActual, posicionOrigen) > MOVIMIENTO.radioPuerta) {
    throw new ExpansionInvalidaError('Solo se desarma en la puerta de su origen: de lo contrario, déjala caducar o que vuelva sola.');
  }
  return devolverCaravana(caravana, mundo, facciones);
}

/**
 * El único mecanismo de fundación (D30): fundar con una caravana en un punto. Lo llama el comando `fundar` del titular. Además de lo que
 * exige `fundarAsentamiento` (zonas, cima, cupo), el punto no puede ser agua ni caer a menos de `MERCENARIOS.radioExclusionFundar` de un
 * campamento de mercenarios (D16).
 */
export function fundarConCaravana(
  mapa: Mapa,
  facciones: Faccion[],
  faccionId: string,
  posicion: Point,
  fundadores: string[],
  asentamientos: Asentamiento[],
  campamentos: readonly CampamentoMercenarios[],
  instante: Instante
): ReturnType<typeof fundarAsentamiento> {
  if (mapa.terrenoEn(posicion) === 'agua') throw new FundacionInvalidaError('No se funda sobre el agua.');
  if (campamentos.some((c) => distancia(c.posicion, posicion) < MERCENARIOS.radioExclusionFundar)) {
    throw new FundacionInvalidaError('Demasiado cerca de un campamento de mercenarios.');
  }
  return fundarAsentamiento(mapa, facciones, faccionId, posicion, fundadores, asentamientos, instante);
}

/**
 * Devuelve lo que costó la caravana a cada aportante, según su registro (D14, D34, D43, D68): a su almacén personal hasta donde quepa, el
 * oro como oro de botín. Lo que no se pueda recibir —no cabe, o el aportante ya no es ciudadano de esa Facción— se pierde.
 */
export function devolverAportes(caravana: Caravana, heroes: readonly Heroe[], facciones: readonly Faccion[]): Heroe[] {
  const faccion = facciones.find((f) => f.id === caravana.faccionId);
  const aportes = caravana.aportes ?? {};
  return heroes.map((h) => {
    const suyo = aportes[h.id];
    if (!suyo || !faccion || !esCiudadano(faccion, h.id)) return h;
    let libre = ALMACEN_PERSONAL.capacidad - totalAlmacenPersonal(h);
    const almacenPersonal = { ...h.almacenPersonal };
    let oroDeBotin = h.oroDeBotin ?? 0;
    for (const [recurso, n] of Object.entries(suyo)) {
      if (recurso === 'oro') {
        oroDeBotin += n;
        continue;
      }
      const cabe = Math.max(0, Math.min(n, libre));
      if (cabe > 0) almacenPersonal[recurso] = (almacenPersonal[recurso] ?? 0) + cabe;
      libre -= cabe;
    }
    return { ...h, almacenPersonal, oroDeBotin };
  });
}

/**
 * Lo que le pasa con el tiempo a la Caravana de Fundación (D13, D14, D40, D68):
 *
 * - **Enganchada sin su titular**: si quien la lleva ya no es su titular, o el titular dejó de ser ciudadano, se suelta y **vuelve sola
 *   a su origen** (otro ciudadano puede reclamarla por el camino, `adjuntarCaravana`). Si la columna ya no existe (su titular salió
 *   del mundo, por ejemplo), lo mismo.
 * - **Volviendo**: avanza hacia su origen; al llegar **se desarma y devuelve** lo que costó (`devolverCaravana`).
 * - **Suelta, sin nadie**: al cumplirse `caducaEn`, **caduca y devuelve**. La que una columna dejó aparcada en una plaza al guarnecer
 *   cuenta como suelta, y empieza a caducar desde entonces.
 *
 * Sin camino de vuelta por tierra, o sin origen, se pierde.
 */
export function avanzarCaravanasDeFundacion<E extends Ejercito>(
  caravanas: readonly Caravana[],
  ejercitos: readonly E[],
  mundo: MundoDeDevolucion,
  facciones: readonly Faccion[],
  campamentos: readonly CampamentoMercenarios[],
  mapa: Mapa,
  instante: Instante
): { caravanas: Caravana[]; ejercitos: E[]; asentamientos: Asentamiento[]; heroes: Heroe[]; eventos: EventoCrudo[] } {
  if (!caravanas.some((c) => c.titularId)) return { caravanas: [...caravanas], ejercitos: [...ejercitos], asentamientos: mundo.asentamientos, heroes: mundo.heroes, eventos: [] };
  const eventos: EventoCrudo[] = [];
  let ejercitosTras = [...ejercitos];
  let tras: MundoDeDevolucion = mundo;
  const restantes: Caravana[] = [];
  const devolver = (c: Caravana, porque: string) => {
    if (!c.origenCampamentoId && !tras.asentamientos.some((a) => a.id === c.origenAsentamientoId)) {
      eventos.push({ codigo: 'fundacion.caravana_perdida', mensaje: `La Caravana de Fundación ${c.id} ${porque}, pero su origen ya no existe: se pierde.`, payload: { caravanaId: c.id } });
      return;
    }
    tras = devolverCaravana(c, tras, facciones);
    eventos.push({ codigo: 'fundacion.caravana_devuelta', mensaje: `La Caravana de Fundación ${c.id} ${porque} y devuelve lo que costó.`, payload: { caravanaId: c.id, faccionId: c.faccionId } });
  };
  const caducaDesdeAhora = sumar(instante, minutos(60 * FUNDACION.caducidadCaravanaHoras));

  for (const c of caravanas) {
    if (!c.titularId) {
      restantes.push(c);
      continue;
    }
    const origen = c.origenCampamentoId ? campamentos.find((m) => m.id === c.origenCampamentoId) : tras.asentamientos.find((a) => a.id === c.origenAsentamientoId);
    if (c.estado === 'adjunta') {
      const columna = ejercitosTras.find((e) => e.caravanasAdjuntasIds.includes(c.id));
      const faccion = facciones.find((f) => f.id === c.faccionId);
      const titularLaLleva = !!columna && columna.participantes.some((p) => p.heroeId === c.titularId) && !!faccion && esCiudadano(faccion, c.titularId);
      if (titularLaLleva) {
        restantes.push(c);
        continue;
      }
      if (columna) ejercitosTras = ejercitosTras.map((e) => (e.id === columna.id ? { ...e, caravanasAdjuntasIds: e.caravanasAdjuntasIds.filter((id) => id !== c.id) } : e));
      const ruta = origen && calcularRuta(mapa, c.posicionActual, origen.posicion);
      if (!ruta) {
        eventos.push({ codigo: 'fundacion.caravana_perdida', mensaje: `La Caravana de Fundación ${c.id} se queda sin nadie que la lleve y sin camino a casa: se pierde.`, payload: { caravanaId: c.id } });
        continue;
      }
      restantes.push({ ...c, estado: 'retornando', ruta, progreso: 0 });
      eventos.push({ codigo: 'fundacion.caravana_vuelve', mensaje: `La Caravana de Fundación ${c.id} se queda sin su titular y vuelve sola a ${origen!.id}.`, payload: { caravanaId: c.id, faccionId: c.faccionId } });
      continue;
    }
    if (c.estado === 'retornando' && c.ruta) {
      const avance = avanzarPosicionEnRuta(mapa, c.ruta, c.progreso, CARAVANA_CATALOGO.construccion.velocidad);
      if (avance.progreso < 1) restantes.push({ ...c, progreso: avance.progreso, posicionActual: avance.posicion });
      else devolver(c, `llega sin nadie a ${origen?.id ?? 'su origen'}, se desarma`);
      continue;
    }
    if (c.estado === 'aparcada' && c.caducaEn === undefined) {
      restantes.push({ ...c, estado: 'disponible', caducaEn: caducaDesdeAhora });
      continue;
    }
    if (c.estado === 'disponible' && c.caducaEn !== undefined && instante >= c.caducaEn) {
      devolver(c, 'caduca sin que nadie la lleve');
      continue;
    }
    restantes.push(c);
  }
  return { caravanas: restantes, ejercitos: ejercitosTras, asentamientos: tras.asentamientos, heroes: tras.heroes, eventos };
}
