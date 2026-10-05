import type {
  Asentamiento,
  ContadorLogro,
  EdificioTipo,
  Ejercito,
  EraId,
  EstadoTecnologia,
  Faccion,
  Heroe,
  RecursoTipo,
  TecnologiaId,
  TecnologiasFaccion,
  ZonaInfluencia,
} from '../domain/types';
import type { EventoCrudo } from '../domain/eventos';
import { minutos, sumar as despues, type Instante } from '../domain/tiempo';
import type { Mapa } from '../world/mapa';
import { AEDAS, ERAS, TARIFA_ADOPCION, TECNOLOGIAS, TROPAS_RECLUTABLES, type CondicionHito } from '../constants';
import { descontarRecursos, tieneRecursos } from './almacen';
import { estaEnAsentamiento } from './ubicacion';
import { nivelActualDe } from './asentamientoQuery';
import { encontrarCapital } from './mantenimiento';
import type { PayloadAsedio, PayloadCombateResuelto, PayloadInterceptacionEjercito } from './combate';
import { ReglaInvalidaError } from './errores';

/** Las tecnologías con las que nace toda Facción (Doc 6.2). */
export const TECNOLOGIAS_DE_ARRANQUE: readonly TecnologiaId[] = (Object.keys(TECNOLOGIAS) as TecnologiaId[]).filter(
  (id) => TECNOLOGIAS[id].deArranque
);

const IDS_TECNOLOGIA = Object.keys(TECNOLOGIAS) as TecnologiaId[];

/** Todo el catálogo: para herramientas que no juegan por Eras (laboratorio de trazado, tests que no prueban tecnología). */
export const TODAS_LAS_TECNOLOGIAS: readonly TecnologiaId[] = IDS_TECNOLOGIA;
const ERAS_EN_ORDEN = (Object.keys(ERAS) as EraId[]).sort((a, b) => ERAS[a].orden - ERAS[b].orden);
const MINUTOS_POR_SEMANA = 7 * 24 * 60;

/** Lo que dice la crónica de cada logro (Doc 6.3): qué ha pasado, nunca qué tecnología abre. */
const LOGRO_CANTADO: Record<ContadorLogro, string> = {
  'extraido.cobre': 'el cobre extraído en el mundo',
  'extraido.hierro': 'el hierro extraído en el mundo',
  'extraido.piedra': 'la piedra extraída en el mundo',
  'extraido.estano': 'el estaño extraído en el mundo',
  'fabricado.equipoBronce': 'las piezas de equipo de bronce forjadas',
  'fabricado.armaduraBronce': 'las armaduras de bronce forjadas',
  'caravanas.destruidasOCapturadas': 'las caravanas destruidas o capturadas',
  'bandidos.campamentosDestruidos': 'los campamentos de bandidos arrasados',
  'animales.comprados': 'los animales comprados',
  'batallas.libradas': 'las batallas libradas',
  'batallas.campoAbierto': 'las batallas a campo abierto',
  'batallas.conHoplitas': 'las batallas con hoplitas',
  'asedios.resistidosEnCombate': 'los asedios resistidos',
  'asedios.resistidosConResidentes': 'los asedios resistidos con sus señores dentro',
  'asedios.contraMurallaCompleta': 'los asedios contra ciudades amuralladas',
  'conquistas.conMurallaCompleta': 'la conquista de una ciudad amurallada',
  'reclutados.escuadrones': 'los escuadrones reclutados',
  'reclutados.arqueros': 'los arqueros reclutados',
  'reclutados.caballeria': 'la caballería y los carros reclutados',
  'reclutados.escaramuzadores_jabalina': 'los escaramuzadores reclutados',
  'reclutados.arqueros_compuesto': 'los arqueros de arco compuesto reclutados',
  'reclutados.jinetes_asirios': 'los jinetes reclutados',
  'plazasEnNivel.2': 'las ciudades que han crecido',
  'plazasEnNivel.3': 'las grandes ciudades',
};

export type DeltaContadores = Partial<Record<ContadorLogro, number>>;

/** Payload de `tecnologia.logro` (público: dice qué pasó, no qué tecnología abre). */
export interface PayloadLogroServidor {
  contador: ContadorLogro;
  umbral: number;
}
/** Payload de `tecnologia.aparece` (privado de la Facción). */
export interface PayloadTecnologiaAparece {
  faccionId: string;
  tecnologiaId: TecnologiaId;
  primera: boolean;
}
/** Payload de `aedas.canta_descubrimiento` (público: la crónica nombra a la descubridora, Doc 6.7). */
export interface PayloadAedasDescubrimiento {
  tecnologiaId: TecnologiaId;
  faccionId: string;
}
/** Payload de `era.comienza` (público). */
export interface PayloadEraComienza {
  era: EraId;
  porPlazo: boolean;
}

/** El servidor arranca en la Era I, sin logros (Doc 6.2). */
export function estadoTecnologiaInicial(desde: Instante): EstadoTecnologia {
  return { era: 'reinos_palaciales', eraDesde: desde, contadores: {}, logros: {}, primeros: {}, porFaccion: {} };
}

/** Tecnologías de una Facción. Una Facción que aún no tiene entrada solo tiene las de arranque. */
export function tecnologiasDe(estado: EstadoTecnologia, faccionId: string): TecnologiasFaccion {
  return estado.porFaccion[faccionId] ?? { aparecidas: [...TECNOLOGIAS_DE_ARRANQUE], adoptadas: [...TECNOLOGIAS_DE_ARRANQUE] };
}

export function tieneTecnologia(estado: EstadoTecnologia, faccionId: string, id: TecnologiaId): boolean {
  return tecnologiasDe(estado, faccionId).adoptadas.includes(id);
}

export function sumarContadores(estado: EstadoTecnologia, delta: DeltaContadores): EstadoTecnologia {
  const entradas = Object.entries(delta).filter(([, n]) => n);
  if (entradas.length === 0) return estado;
  const contadores = { ...estado.contadores };
  for (const [clave, n] of entradas) contadores[clave as ContadorLogro] = (contadores[clave as ContadorLogro] ?? 0) + n!;
  return { ...estado, contadores };
}

export function sumarDeltas(a: DeltaContadores, b: DeltaContadores): DeltaContadores {
  const r = { ...a };
  for (const [clave, n] of Object.entries(b)) if (n) sumar(r, clave as ContadorLogro, n);
  return r;
}

function sumar(delta: DeltaContadores, clave: ContadorLogro, n = 1): void {
  delta[clave] = (delta[clave] ?? 0) + n;
}

/** Contadores que salen de los eventos del motor (combates, asedios, caravanas, bandidos). */
export function contadoresDeEventos(eventos: readonly EventoCrudo[]): DeltaContadores {
  const delta: DeltaContadores = {};
  for (const e of eventos) {
    if (typeof e === 'string') continue;
    switch (e.codigo) {
      case 'combate.resuelto': {
        sumar(delta, 'batallas.libradas');
        if ((e.payload as PayloadCombateResuelto).tropaIds?.includes('hoplitas_ciudadanos')) sumar(delta, 'batallas.conHoplitas');
        break;
      }
      case 'combate.encuentro':
        sumar(delta, 'batallas.campoAbierto');
        break;
      case 'combate.asedio_resistido':
      case 'combate.asedio_conquista': {
        const p = e.payload as PayloadAsedio;
        const conquista = e.codigo === 'combate.asedio_conquista';
        if (p.enCombate && !conquista) sumar(delta, 'asedios.resistidosEnCombate');
        if (p.enCombate && !conquista && p.conResidentes) sumar(delta, 'asedios.resistidosConResidentes');
        if (p.enCombate && p.murallaCompleta) sumar(delta, 'asedios.contraMurallaCompleta');
        if (conquista && p.murallaCompleta) sumar(delta, 'conquistas.conMurallaCompleta');
        break;
      }
      case 'comercio.caravana_animal_comprado':
        sumar(delta, 'animales.comprados');
        break;
      case 'combate.campamento_destruido':
        sumar(delta, 'bandidos.campamentosDestruidos');
        break;
      case 'combate.caravana_interceptada_por_ejercito':
        if ((e.payload as PayloadInterceptacionEjercito).capturada) sumar(delta, 'caravanas.destruidasOCapturadas');
        break;
      case 'bandidos.caravana_interceptada':
        sumar(delta, 'caravanas.destruidasOCapturadas');
        break;
    }
  }
  return delta;
}

/** Contadores de lo extraído y fabricado en un tick (sin evento por unidad: serían miles). */
export function contadoresDeProduccion(
  extraido: Partial<Record<RecursoTipo, number>>,
  fabricado: Partial<Record<RecursoTipo, number>>
): DeltaContadores {
  const delta: DeltaContadores = {};
  if (extraido.cobre) sumar(delta, 'extraido.cobre', extraido.cobre);
  if (extraido.hierro) sumar(delta, 'extraido.hierro', extraido.hierro);
  if (extraido.piedra) sumar(delta, 'extraido.piedra', extraido.piedra);
  if (extraido.estano) sumar(delta, 'extraido.estano', extraido.estano);
  const bronce = (fabricado.armaBronce ?? 0) + (fabricado.armaBronceCalidad ?? 0) + (fabricado.armaduraBronce ?? 0) + (fabricado.armaduraBronceCalidad ?? 0);
  if (bronce) sumar(delta, 'fabricado.equipoBronce', bronce);
  if (fabricado.armaduraBronce) sumar(delta, 'fabricado.armaduraBronce', fabricado.armaduraBronce);
  return delta;
}

/** Contadores de un reclutamiento: soldados de la tropa (si tiene logro propio) y escuadrón nuevo. */
/** Lo que cuenta cada soldado de Caballeriza en `reclutados.caballeria` (2026-10-01, decisión del usuario): uno de carro
 * de guerra vale por 5 de jinete; el resto de la caballería, uno. */
const PESO_EN_CABALLERIA: Record<string, number> = { carros_guerra: 5 };

export function contadoresDeReclutamiento(tropaId: string, soldados: number, escuadronNuevo: boolean): DeltaContadores {
  const delta: DeltaContadores = {};
  if (escuadronNuevo) sumar(delta, 'reclutados.escuadrones');
  if (soldados > 0 && TROPAS_RECLUTABLES.find((t) => t.id === tropaId)?.edificio === 'caballerizas') {
    sumar(delta, 'reclutados.caballeria', soldados * (PESO_EN_CABALLERIA[tropaId] ?? 1));
  }
  const clave = `reclutados.${tropaId}` as ContadorLogro;
  if (clave in LOGRO_CANTADO && soldados > 0) sumar(delta, clave, soldados);
  return delta;
}

export interface ContextoTecnologia {
  asentamientos: readonly Asentamiento[];
  facciones: readonly Faccion[];
  zonas: readonly ZonaInfluencia[];
  mapa: Mapa;
  instante: Instante;
}

function edificioActivo(asentamientos: readonly Asentamiento[], tipo: EdificioTipo, nivelInterno = 1): boolean {
  return asentamientos.some((a) => a.edificios.some((e) => e.tipo === tipo && e.estado === 'activo' && (e.nivelInterno ?? 1) >= nivelInterno));
}

function cumpleCondicion(
  c: CondicionHito,
  faccionId: string,
  tecnologias: TecnologiasFaccion,
  propios: readonly Asentamiento[],
  ctx: ContextoTecnologia
): boolean {
  switch (c.tipo) {
    case 'edificio':
      return edificioActivo(propios, c.edificio, c.nivelInterno);
    case 'tecnologia':
      return tecnologias.adoptadas.includes(c.id);
    case 'recursoEnCapital': {
      const capital = encontrarCapital(faccionId, [...propios]);
      return (capital?.almacen[c.recurso]?.cantidad ?? 0) > 0;
    }
    case 'capitalEnNivel': {
      const capital = encontrarCapital(faccionId, [...propios]);
      return !!capital && nivelActualDe(capital) >= c.nivel && edificioActivo([capital], c.conEdificio);
    }
    case 'yacimientoEnTerritorio': {
      const ids = new Set(propios.map((a) => a.id));
      return ctx.zonas.some((z) => ids.has(z.asentamientoId) && ctx.mapa.nodosEnPoligono(z.poligono, { tipo: c.recurso }).length > 0);
    }
  }
}

/**
 * Un tick de tecnología (Doc 6.2-6.4), al final de la simulación: récords de plazas por nivel, logros que cruzan su
 * umbral, avance de Era y aparición de tecnologías a las Facciones que cumplen su hito. Determinista, sin RNG.
 */
export function avanzarTecnologia(estado: EstadoTecnologia, ctx: ContextoTecnologia): { tecnologia: EstadoTecnologia; eventos: EventoCrudo[] } {
  const eventos: EventoCrudo[] = [];
  const contadores = { ...estado.contadores };
  for (const nivel of [2, 3] as const) {
    const clave = `plazasEnNivel.${nivel}` as const;
    const ahora = ctx.asentamientos.filter((a) => nivelActualDe(a) >= nivel).length;
    if (ahora > (contadores[clave] ?? 0)) contadores[clave] = ahora;
  }

  // 1. Logros del servidor: se fijan para siempre (Doc 6.3). Públicos, sin nombrar la tecnología.
  const logros = { ...estado.logros };
  for (const id of IDS_TECNOLOGIA) {
    const logro = TECNOLOGIAS[id].logro;
    if (!logro || logros[id] !== undefined || (contadores[logro.contador] ?? 0) < logro.umbral) continue;
    logros[id] = ctx.instante;
    eventos.push({
      codigo: 'tecnologia.logro',
      mensaje: `Los Aedas cantan un logro del mundo: ${LOGRO_CANTADO[logro.contador]} llegan a ${logro.umbral}.`,
      payload: { contador: logro.contador, umbral: logro.umbral } satisfies PayloadLogroServidor,
    });
  }

  // 2. Era (Doc 6.2): la siguiente llega con TODOS los logros de la vigente o al agotarse su plazo.
  let era = estado.era;
  let eraDesde = estado.eraDesde;
  const siguiente = ERAS_EN_ORDEN[ERAS_EN_ORDEN.indexOf(era) + 1];
  if (siguiente) {
    const todos = IDS_TECNOLOGIA.filter((id) => TECNOLOGIAS[id].era === era && TECNOLOGIAS[id].logro).every((id) => logros[id] !== undefined);
    const porPlazo = ctx.instante - eraDesde >= minutos(ERAS[era].plazoSemanas * MINUTOS_POR_SEMANA);
    if (todos || porPlazo) {
      era = siguiente;
      eraDesde = ctx.instante;
      eventos.push({
        codigo: 'era.comienza',
        mensaje: `Empieza una nueva Era: ${ERAS[era].nombre}.`,
        payload: { era, porPlazo: !todos } satisfies PayloadEraComienza,
      });
    }
  }

  // 3. Aparición (Doc 6.3-6.4): a cada Facción, sola, cuando cumple el hito de una tecnología con logro y Era abiertos.
  const ordenEra = ERAS[era].orden;
  const porFaccion = { ...estado.porFaccion };
  const primeros = { ...estado.primeros };
  for (const faccion of ctx.facciones) {
    const propios = ctx.asentamientos.filter((a) => a.faccionId === faccion.id);
    if (propios.length === 0) continue;
    let tecnologias = tecnologiasDe(estado, faccion.id);
    let cambio = false;
    for (const id of IDS_TECNOLOGIA) {
      const t = TECNOLOGIAS[id];
      if (t.deArranque || tecnologias.aparecidas.includes(id) || logros[id] === undefined || ERAS[t.era].orden > ordenEra) continue;
      if (!t.hito.every((c) => cumpleCondicion(c, faccion.id, tecnologias, propios, ctx))) continue;
      tecnologias = conAparecida(tecnologias, id);
      cambio = true;
      const primera = primeros[id] === undefined;
      if (primera) primeros[id] = { faccionId: faccion.id, en: ctx.instante };
      eventos.push({
        codigo: 'tecnologia.aparece',
        mensaje: `${faccion.nombre} descubre ${t.nombre}${primera ? ', la primera del mundo' : ''}.`,
        payload: { faccionId: faccion.id, tecnologiaId: id, primera } satisfies PayloadTecnologiaAparece,
        asentamientoId: encontrarCapital(faccion.id, propios)?.id,
      });
    }
    if (cambio) porFaccion[faccion.id] = tecnologias;
  }

  // 4. Crónica (Doc 6.7): cuando el retraso se cumple, los Aedas cantan quién desbloqueó cada tecnología.
  const cantadas = [...(estado.cantadas ?? [])];
  const conocimiento = { ...estado, era, logros, primeros };
  for (const id of IDS_TECNOLOGIA) {
    if (cantadas.includes(id) || !conocidaPorAedas(conocimiento, id, ctx.instante)) continue;
    cantadas.push(id);
    const descubridor = primeros[id]!.faccionId;
    eventos.push({
      codigo: 'aedas.canta_descubrimiento',
      mensaje: `Los Aedas cantan que ${ctx.facciones.find((f) => f.id === descubridor)?.nombre ?? descubridor} desbloqueó ${TECNOLOGIAS[id].nombre}.`,
      payload: { tecnologiaId: id, faccionId: descubridor } satisfies PayloadAedasDescubrimiento,
    });
  }

  return { tecnologia: { era, eraDesde, contadores, logros, primeros, ...(cantadas.length > 0 ? { cantadas } : {}), porFaccion }, eventos };
}

/** La tecnología pasa a aparecida (Doc 6.4): deja de estar solo revelada. */
function conAparecida(tecnologias: TecnologiasFaccion, id: TecnologiaId): TecnologiasFaccion {
  return {
    ...tecnologias,
    aparecidas: [...tecnologias.aparecidas, id],
    ...(tecnologias.reveladas ? { reveladas: tecnologias.reveladas.filter((r) => r !== id) } : {}),
  };
}

export class AdopcionInvalidaError extends ReglaInvalidaError {}

/** Payload de `tecnologia.adoptada` (privado de la Facción). */
export interface PayloadTecnologiaAdoptada {
  faccionId: string;
  tecnologiaId: TecnologiaId;
  capitalId: string;
}

/**
 * El Rey adopta una tecnología estando en la capital, y la paga el almacén de la capital (Doc 6.5). Tiene que
 * haberle aparecido a su Facción y no estar ya adoptada. La adopción es instantánea.
 */
export function adoptarTecnologia(
  estado: EstadoTecnologia,
  faccion: Faccion,
  id: TecnologiaId,
  mundo: { asentamientos: readonly Asentamiento[]; heroes: readonly Heroe[]; ejercitos: readonly Ejercito[] }
): { tecnologia: EstadoTecnologia; capital: Asentamiento; eventos: EventoCrudo[] } {
  const definicion = TECNOLOGIAS[id];
  if (!definicion) throw new AdopcionInvalidaError('Esa tecnología no existe.');
  if (!faccion.reyId) throw new AdopcionInvalidaError('La Facción no tiene Rey: solo él adopta tecnología.');
  const capital = encontrarCapital(faccion.id, [...mundo.asentamientos]);
  if (!capital) throw new AdopcionInvalidaError('La Facción no tiene capital.');
  if (!estaEnAsentamiento(mundo.heroes, faccion.reyId, capital.id, mundo.asentamientos, mundo.ejercitos)) {
    throw new AdopcionInvalidaError(`El Rey tiene que estar en la capital (${capital.nombre}) para adoptar una tecnología.`);
  }
  const tecnologias = tecnologiasDe(estado, faccion.id);
  if (tecnologias.adoptadas.includes(id)) throw new AdopcionInvalidaError(`${definicion.nombre} ya está adoptada.`);
  if (!tecnologias.aparecidas.includes(id)) throw new AdopcionInvalidaError(`${definicion.nombre} no le ha aparecido a la Facción.`);
  const tarifa = TARIFA_ADOPCION[definicion.era];
  if (!tieneRecursos(capital.almacen, tarifa)) {
    const pide = Object.entries(tarifa).map(([r, n]) => `${n} ${r}`).join(', ');
    throw new AdopcionInvalidaError(`La capital no tiene con qué pagar ${definicion.nombre}: pide ${pide}.`);
  }
  return {
    tecnologia: { ...estado, porFaccion: { ...estado.porFaccion, [faccion.id]: { ...tecnologias, adoptadas: [...tecnologias.adoptadas, id] } } },
    capital: { ...capital, almacen: descontarRecursos(capital.almacen, tarifa) },
    eventos: [
      {
        codigo: 'tecnologia.adoptada',
        mensaje: `${faccion.nombre} adopta ${definicion.nombre}.`,
        payload: { faccionId: faccion.id, tecnologiaId: id, capitalId: capital.id } satisfies PayloadTecnologiaAdoptada,
        asentamientoId: capital.id,
      },
    ],
  };
}

// --- Aedas (Doc 6.4, 6.7): lo que conocen, lo que revelan y lo que venden ---

/**
 * ¿La conocen los Aedas ya? Con el logro cumplido, la Era abierta y alguien que la desbloqueó hace más que el retraso
 * (la ventaja del primero, Doc 6.4).
 */
function conocidaPorAedas(estado: EstadoTecnologia, id: TecnologiaId, instante: Instante): boolean {
  const primero = estado.primeros[id];
  return (
    estado.logros[id] !== undefined &&
    primero !== undefined &&
    despues(primero.en, minutos(AEDAS.retrasoConocimientoMinutos)) <= instante &&
    ERAS[TECNOLOGIAS[id].era].orden <= ERAS[estado.era].orden
  );
}

function describirCondicion(c: CondicionHito): string {
  switch (c.tipo) {
    case 'edificio':
      return `${c.edificio}${c.nivelInterno ? ` de nivel ${c.nivelInterno}` : ''} activo`;
    case 'tecnologia':
      return `${TECNOLOGIAS[c.id].nombre} adoptada`;
    case 'recursoEnCapital':
      return `${c.recurso} en el almacén de la capital`;
    case 'capitalEnNivel':
      return `capital en nivel ${c.nivel} con ${c.conEdificio} activo`;
    case 'yacimientoEnTerritorio':
      return `un yacimiento de ${c.recurso} en su territorio`;
  }
}

/** Payload de `aedas.revela` (privado de la Facción). */
export interface PayloadAedasRevela {
  faccionId: string;
  tecnologiaId: TecnologiaId;
  descubridorFaccionId: string;
  /** Lo que le falta del hito, en texto. */
  faltan: string[];
}

/**
 * Un Aeda detenido en una plaza revela a su Facción lo que conoce y ella no tiene aparecido ni revelado: qué es, quién
 * la desbloqueó y qué le falta del hito (Doc 6.4). Se llama cada tick por cada Aeda parado, así que lo que pasa a
 * conocerse durante su estancia se revela cuando el retraso se cumple.
 */
export function revelarTecnologias(
  estado: EstadoTecnologia,
  faccion: Faccion,
  plazaId: string,
  ctx: ContextoTecnologia
): { tecnologia: EstadoTecnologia; eventos: EventoCrudo[] } {
  const propios = ctx.asentamientos.filter((a) => a.faccionId === faccion.id);
  let tecnologias = tecnologiasDe(estado, faccion.id);
  const eventos: EventoCrudo[] = [];
  const reveladas = [...(tecnologias.reveladas ?? [])];
  for (const id of IDS_TECNOLOGIA) {
    if (tecnologias.aparecidas.includes(id) || reveladas.includes(id) || !conocidaPorAedas(estado, id, ctx.instante)) continue;
    const descubridor = estado.primeros[id]!.faccionId;
    const faltan = TECNOLOGIAS[id].hito.filter((c) => !cumpleCondicion(c, faccion.id, tecnologias, propios, ctx)).map(describirCondicion);
    reveladas.push(id);
    eventos.push({
      codigo: 'aedas.revela',
      mensaje: `Un Aeda revela a ${faccion.nombre} ${TECNOLOGIAS[id].nombre}, que desbloqueó ${ctx.facciones.find((f) => f.id === descubridor)?.nombre ?? descubridor}${faltan.length > 0 ? `; le falta: ${faltan.join(', ')}` : ''}.`,
      payload: { faccionId: faccion.id, tecnologiaId: id, descubridorFaccionId: descubridor, faltan } satisfies PayloadAedasRevela,
      asentamientoId: plazaId,
    });
  }
  if (eventos.length === 0) return { tecnologia: estado, eventos };
  tecnologias = { ...tecnologias, reveladas };
  return { tecnologia: { ...estado, porFaccion: { ...estado.porFaccion, [faccion.id]: tecnologias } }, eventos };
}

export class VentaInvalidaError extends ReglaInvalidaError {}

/** Lo que cobra un Aeda por una tecnología: el oro de la tarifa de adopción de su Era, multiplicado (Doc 6.7). */
export function precioDeVenta(id: TecnologiaId): number {
  return (TARIFA_ADOPCION[TECNOLOGIAS[id].era].oro ?? 0) * AEDAS.venta.factorOro;
}

/** Payload de `aedas.venta` (privado de la Facción). */
export interface PayloadAedasVenta {
  faccionId: string;
  tecnologiaId: TecnologiaId;
  aedaId: string;
  precio: number;
}

/**
 * Compra de una tecnología a un Aeda detenido en `plaza` (Doc 6.7): se salta el hito de la Facción, nunca el logro del
 * servidor. Tiene que conocerla el Aeda, ser de una Era que vendan y no estar ya aparecida; paga el almacén de la plaza.
 * La tecnología pasa a aparecida —no a adoptada: la adopción se paga igual (Doc 6.5)—.
 */
export function venderTecnologia(
  estado: EstadoTecnologia,
  faccion: Faccion,
  plaza: Asentamiento,
  aedaId: string,
  id: TecnologiaId,
  instante: Instante
): { tecnologia: EstadoTecnologia; plaza: Asentamiento; eventos: EventoCrudo[] } {
  const definicion = TECNOLOGIAS[id];
  if (!definicion) throw new VentaInvalidaError('Esa tecnología no existe.');
  if (ERAS[definicion.era].orden > AEDAS.venta.ordenEraMaximo) throw new VentaInvalidaError(`Los Aedas no venden ${definicion.nombre}: es de una Era que solo llega por hito, comercio, conquista o épica.`);
  if (!conocidaPorAedas(estado, id, instante)) throw new VentaInvalidaError(`El Aeda aún no conoce ${definicion.nombre}.`);
  const tecnologias = tecnologiasDe(estado, faccion.id);
  if (tecnologias.aparecidas.includes(id)) throw new VentaInvalidaError(`${definicion.nombre} ya le ha aparecido a la Facción.`);
  const precio = precioDeVenta(id);
  if (!tieneRecursos(plaza.almacen, { oro: precio })) throw new VentaInvalidaError(`El almacén de ${plaza.nombre ?? plaza.id} no tiene los ${precio} de oro que pide el Aeda.`);
  return {
    tecnologia: { ...estado, porFaccion: { ...estado.porFaccion, [faccion.id]: conAparecida(tecnologias, id) } },
    plaza: { ...plaza, almacen: descontarRecursos(plaza.almacen, { oro: precio }) },
    eventos: [
      {
        codigo: 'aedas.venta',
        mensaje: `${faccion.nombre} compra ${definicion.nombre} a un Aeda por ${precio} de oro.`,
        payload: { faccionId: faccion.id, tecnologiaId: id, aedaId, precio } satisfies PayloadAedasVenta,
        asentamientoId: plaza.id,
      },
    ],
  };
}
