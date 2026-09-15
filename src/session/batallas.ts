// Las batallas que se juegan en Unity (doc 01 §15, Doc 5.15.1), del lado de BronzeAge: abrirlas con su ticket
// congelado, bloquear lo que interviene, dejar que se una quien llega y cerrarlas sin castigo si vence un plazo o se
// cancelan. Aplicar el resultado es la fase 2.
//
// No es estado del motor: el motor solo devuelve los combates que no resuelve (`CombatePorAbrir`), y lo que está en
// una batalla activa ni siquiera entra en el tick. Todo lo que viaja a Conquest tiene la forma del contrato
// (`contratos/v1/dto.ts`).
import type { Asentamiento, CampamentoBandido, Caravana, Ejercito, Escuadron, Heroe, Point } from '../domain/types';
import {
  SCHEMA_VERSION,
  type BattleParticipantSnapshot,
  type BattleServerAssignment,
  type BattleSide,
  type BattleTicket,
  type ContextoEstrategico,
  type IncorporacionBatalla,
  type InicioBatalla,
  type LadoId,
  type SettlementBattleSnapshot,
  type SquadSnapshot,
  type TokenParticipante,
  type TokensBatalla,
} from '../contratos/v1/dto';
import { VERSION_CATALOGO_TROPAS } from '../contratos/v1/catalogoTropas';
import { BALANCE_VERSION, BATALLA, LAYOUT_VERSION, LOGISTICA, REJILLA_ASENTAMIENTO } from '../constants';
import { minutos, sumar, type Instante } from '../domain/tiempo';
import { distancia } from '../world/geometria';
import { columnaDe, type CombatePorAbrir } from '../engine/ejercitos';
import { esCiudadano } from '../engine/faccion';
import { heridosEn } from '../engine/heroe';
import { edificiosInternos, tamanoDeEdificio } from '../engine/trazado';
import { guarnicionDe, heroesQueDefienden, indiceTropa } from '../engine/tropa';
import { idDeMapa, type GameSessionState } from './estado';

export type EstadoBatalla = 'convocando' | 'asignada' | 'en_curso' | 'aplicada' | 'cancelada' | 'fallida';

/** Una batalla de Unity (doc 01 §15). */
export interface Batalla {
  /** UUID: las rutas de Conquest la nombran sin `gameId`, así que tiene que ser única entre partidas. */
  id: string;
  estado: EstadoBatalla;
  /** Quien comprometió el combate: puede cancelarla antes de que empiece la partida. */
  iniciadaPor: string;
  /** Dónde se ve en el mapa (Doc 5.15.1); en un asedio, la plaza. */
  punto: Point;
  /** Congelado al abrir. Quien se une después va en `incorporaciones`, que solo crece. */
  ticket: BattleTicket;
  incorporaciones: IncorporacionBatalla[];
  /** Lo que se queda quieto mientras dura (Doc 5.15.1), además de sus héroes. */
  bloqueo: { ejercitoIds: string[]; caravanaIds: string[]; asentamientoId?: string; campamentoId?: string };
  /** Plazo del estado actual, en tiempo de mundo: que la asignen, que empiece o, en curso, la duración más un margen.
   * Si vence, `fallida` sin castigo (`vencerBatallas`). */
  expiraEn: Instante;
  iniciadaEn?: Instante;
  /** La asignación aceptada, con quién la registró y los tokens de los que se unen después. */
  asignacion?: BattleServerAssignment & { servidorId: string };
}

/** Unirse, cancelar o un mensaje de Conquest que no vale (Doc 5.15.1, doc 02 §3.3). Lo traduce `erroresDeDominio.ts` a
 * `batalla.invalida`. */
export class BatallaInvalidaError extends Error {}

/** Otro servidor de batalla ya tiene esta batalla (doc 02 §3.3). Lo traduce `erroresDeDominio.ts` a `batalla.ya_asignada`. */
export class BatallaYaAsignadaError extends Error {}

const ACTIVAS: ReadonlySet<EstadoBatalla> = new Set(['convocando', 'asignada', 'en_curso']);

/** Abiertas y dentro de plazo. Una vencida deja de bloquear ya, aunque la cierre el siguiente tick (doc 01 §15). */
export function batallasActivas(estado: Pick<GameSessionState, 'batallas'>, ahora: Instante): Batalla[] {
  return estado.batallas.filter((b) => ACTIVAS.has(b.estado) && ahora < b.expiraEn);
}

export interface Participacion {
  lado: LadoId;
  participante: BattleParticipantSnapshot;
}

/** Los héroes que combaten: los del ticket y los que se unieron después. */
export function participacionesDe(b: Batalla): Participacion[] {
  const del = (lado: LadoId) => b.ticket.bandos[lado].participantes.map((participante) => ({ lado, participante }));
  return [...del('atacante'), ...del('defensor'), ...b.incorporaciones.map(({ lado, participante }) => ({ lado, participante }))];
}

/** Las escuadras reservadas que tienen dueño. La tropa sin dueño (bandidos, carreteros) solo existe en el ticket. */
function escuadrasConDueno(b: Batalla): string[] {
  const { atacante, defensor } = b.ticket.bandos;
  return [...participacionesDe(b).flatMap((p) => p.participante.escuadras), ...atacante.escuadrasSinHeroe, ...defensor.escuadrasSinHeroe]
    .filter((e) => e.heroeId !== null)
    .map((e) => e.squadId);
}

export interface Bloqueos {
  heroes: Set<string>;
  ejercitos: Set<string>;
  caravanas: Set<string>;
  asentamientos: Set<string>;
  campamentos: Set<string>;
}

/** Todo lo que está quieto por las batallas activas (Doc 5.15.1). */
export function bloqueosDe(estado: Pick<GameSessionState, 'batallas'>, ahora: Instante): Bloqueos {
  const b: Bloqueos = { heroes: new Set(), ejercitos: new Set(), caravanas: new Set(), asentamientos: new Set(), campamentos: new Set() };
  for (const batalla of batallasActivas(estado, ahora)) {
    for (const p of participacionesDe(batalla)) b.heroes.add(p.participante.heroeId);
    for (const id of batalla.bloqueo.ejercitoIds) b.ejercitos.add(id);
    for (const id of batalla.bloqueo.caravanaIds) b.caravanas.add(id);
    if (batalla.bloqueo.asentamientoId) b.asentamientos.add(batalla.bloqueo.asentamientoId);
    if (batalla.bloqueo.campamentoId) b.campamentos.add(batalla.bloqueo.campamentoId);
  }
  return b;
}

const CLAVES_DE_ASENTAMIENTO = ['asentamientoId', 'atacanteId', 'defensorId', 'destinoId', 'origenAsentamientoId', 'asentamientoAId', 'asentamientoBId'];
const CLAVES_DE_CARAVANA = ['caravanaId', 'desdeCaravanaId', 'haciaCaravanaId'];

/**
 * ¿Toca este comando algo que está en una batalla (Doc 5.15.1)? Un héroe que combate no hace nada más, ni quien está
 * dentro de una plaza asediada, y nadie apunta a una columna, caravana, plaza o campamento bloqueados. Marchar HACIA
 * una plaza asediada sí se puede: se llega y se espera a la puerta, o se une uno a la batalla.
 *
 * Mira los parámetros por su NOMBRE, que es la convención de todo el registro (`esquemas.ts`), así que cubre los
 * comandos que vengan sin tocarlos uno a uno.
 */
export function tocaLoBloqueado(estado: GameSessionState, ahora: Instante, actor: string, params: unknown): boolean {
  if (!estado.batallas.some((b) => ACTIVAS.has(b.estado))) return false;
  const b = bloqueosDe(estado, ahora);
  const p = (params ?? {}) as Record<string, unknown>;
  const textos = (claves: readonly string[]) => claves.map((k) => p[k]).filter((v): v is string => typeof v === 'string');
  const heroeBloqueado = (id: string) => {
    if (b.heroes.has(id)) return true;
    const u = estado.heroes.find((h) => h.id === id)?.ubicacion;
    return u?.tipo === 'asentamiento' && b.asentamientos.has(u.asentamientoId);
  };
  const objetivo = p['objetivo'] as { tipo?: string; id?: string } | undefined;
  const apuntables: Record<string, Set<string>> = { ejercito: b.ejercitos, caravana: b.caravanas, campamento: b.campamentos };
  return (
    [actor, ...textos(['heroeId'])].some(heroeBloqueado) ||
    textos(CLAVES_DE_ASENTAMIENTO).some((id) => b.asentamientos.has(id)) ||
    textos(['ejercitoId']).some((id) => b.ejercitos.has(id)) ||
    textos(CLAVES_DE_CARAVANA).some((id) => b.caravanas.has(id)) ||
    (objetivo?.tipo !== undefined && objetivo.id !== undefined && (apuntables[objetivo.tipo]?.has(objetivo.id) ?? false))
  );
}

/** Las Facciones con algo en una batalla activa: su IA no gobierna mientras dure (`avanzarFaccionesNpc`). */
export function faccionesEnBatalla(estado: Pick<GameSessionState, 'batallas'>, ahora: Instante): Set<string> {
  return new Set(
    batallasActivas(estado, ahora)
      .flatMap((b) => [b.ticket.bandos.atacante.faccionId, b.ticket.bandos.defensor.faccionId])
      .filter((f): f is string => f !== null)
  );
}

// --- El ticket: la foto que viaja a Conquest ---

function escuadraEnTicket(e: Escuadron): SquadSnapshot {
  return {
    squadId: e.id,
    heroeId: e.heroeId,
    tropaId: e.tropaId,
    efectivosAutorizados: e.cantidad,
    nivel: e.nivel,
    experiencia: e.experiencia,
    moral: e.moral,
    habilidadesDesbloqueadas: e.habilidadesDesbloqueadas,
    formacionesDesbloqueadas: e.formacionesDesbloqueadas,
    formacionSeleccionada: e.formacionSeleccionada,
  };
}

/** Tropa sin dueño y recién reclutada: la de un campamento de bandidos o los carreteros de una caravana. */
function tropaSinDueno(squadId: string, tropa: { tropaId: string; unidades: number }): SquadSnapshot {
  return {
    squadId,
    heroeId: null,
    tropaId: tropa.tropaId,
    efectivosAutorizados: tropa.unidades,
    nivel: 1,
    experiencia: 0,
    moral: 100,
    habilidadesDesbloqueadas: [],
    formacionesDesbloqueadas: [0],
    formacionSeleccionada: 0,
  };
}

function enBatalla(h: Heroe, escuadras: readonly Escuadron[]): BattleParticipantSnapshot {
  const ocupadas = h.inventario.filter((i) => i.casillaInventario >= 0).length;
  return {
    heroeId: h.id,
    controlador: h.controlador,
    heroe: {
      displayName: h.displayName,
      classDefinitionId: h.classDefinitionId,
      nivel: h.nivel,
      genero: h.genero,
      avatar: h.avatar,
      // `ponytail:` sin el catálogo de objetos de Conquest (CQ-004) el equipo no suma: van los atributos base.
      atributosEfectivos: h.atributosBase,
      perksDesbloqueados: h.perksDesbloqueados,
      equipamiento: h.equipamiento,
      casillasInventarioLibres: Math.max(0, BATALLA.casillasInventario - ocupadas),
    },
    escuadras: escuadras.filter((e) => e.cantidad > 0).map(escuadraEnTicket),
  };
}

/** Los héroes sanos de una columna, con lo que llevan en ella (Doc 5.15.1, 5.16.4). */
function heroesDeColumna(estado: GameSessionState, columna: Ejercito, heridos: ReadonlySet<string>): BattleParticipantSnapshot[] {
  return columna.participantes
    .filter((p) => !heridos.has(p.heroeId))
    .flatMap((p) => {
      const h = estado.heroes.find((x) => x.id === p.heroeId);
      return h ? [enBatalla(h, h.escuadrones.filter((e) => e.contenedor.tipo === 'ejercito' && e.contenedor.ejercitoId === columna.id))] : [];
    });
}

/** La plaza congelada al abrir (doc 01 §17): lo que está en pie ahora, en coordenadas locales. */
function mapaDeAsedio(plaza: Asentamiento): SettlementBattleSnapshot {
  return {
    tipo: 'asentamiento',
    settlementId: plaza.id,
    layoutVersion: LAYOUT_VERSION,
    unidadesPorCelda: REJILLA_ASENTAMIENTO.tamanoCelda,
    edificios: edificiosInternos(plaza.edificios).map((e) => ({
      edificioId: e.id,
      tipo: e.tipo,
      posicion: e.posicion,
      ...tamanoDeEdificio(e),
      ...(e.nivelInterno !== undefined ? { nivelInterno: e.nivelInterno } : {}),
      estado: e.estado,
      ...(e.danado ? { danado: true } : {}),
    })),
    // Con una mejora en curso sigue en pie el anillo entero al nivel de antes; si no, lo levantado hasta `avance`.
    recintos: (plaza.recintos ?? [])
      .map((r) => ({ recintoId: r.id, nivel: r.nivel, celdas: r.mejorandoA !== undefined ? r.celdas : r.celdas.slice(0, r.avance + 1) }))
      .filter((r) => r.celdas.length > 0),
  };
}

interface Bando {
  faccionId: string | null;
  participantes: BattleParticipantSnapshot[];
  escuadrasSinHeroe: SquadSnapshot[];
}

/** Lo que hace falta para abrir una batalla: qué se disputa, los dos bandos y lo que se queda quieto. */
export interface Apertura {
  contexto: ContextoEstrategico;
  punto: Point;
  iniciadaPor: string;
  atacante: Bando;
  defensor: Bando;
  bloqueo: Batalla['bloqueo'];
  mapa: BattleTicket['mapa'];
}

function enElMapa(estado: GameSessionState, centro: Point): BattleTicket['mapa'] {
  return { tipo: 'mapa', mapaId: idDeMapa(estado.mapa), centro };
}

function bandoDeColumna(estado: GameSessionState, columna: Ejercito, heridos: ReadonlySet<string>): Bando {
  return { faccionId: columna.faccionId, participantes: heroesDeColumna(estado, columna, heridos), escuadrasSinHeroe: [] };
}

export function aperturaContraColumna(estado: GameSessionState, atacante: Ejercito, defensor: Ejercito, iniciadaPor: string, heridos: ReadonlySet<string>): Apertura {
  const punto = defensor.posicionActual;
  return {
    contexto: { tipo: 'campo_abierto', punto },
    punto,
    iniciadaPor,
    atacante: bandoDeColumna(estado, atacante, heridos),
    defensor: bandoDeColumna(estado, defensor, heridos),
    bloqueo: { ejercitoIds: [atacante.id, defensor.id], caravanaIds: [] },
    mapa: enElMapa(estado, punto),
  };
}

/** Contra una caravana defiende su escolta y, sin escolta, sus carreteros, que no son de nadie (Doc 3.10, 5.15.4). */
export function aperturaContraCaravana(estado: GameSessionState, atacante: Ejercito, caravana: Caravana, iniciadaPor: string, heridos: ReadonlySet<string>): Apertura {
  const punto = caravana.posicionActual;
  const tropa = indiceTropa(estado.heroes);
  const escolta = (caravana.escoltaIds ?? []).flatMap((id) => tropa.get(id) ?? []).filter((e) => e.cantidad > 0);
  return {
    contexto: { tipo: 'caravana', caravanaId: caravana.id, punto },
    punto,
    iniciadaPor,
    atacante: bandoDeColumna(estado, atacante, heridos),
    defensor: {
      faccionId: estado.asentamientos.find((a) => a.id === caravana.origenAsentamientoId)?.faccionId ?? null,
      participantes: [],
      escuadrasSinHeroe: escolta.length > 0 ? escolta.map(escuadraEnTicket) : [tropaSinDueno(`${caravana.id}-carreteros`, BATALLA.tropaCarreteros)],
    },
    bloqueo: { ejercitoIds: [atacante.id], caravanaIds: [caravana.id] },
    mapa: enElMapa(estado, punto),
  };
}

export function aperturaContraCampamento(estado: GameSessionState, atacante: Ejercito, campamento: CampamentoBandido, iniciadaPor: string, heridos: ReadonlySet<string>): Apertura {
  const punto = campamento.posicion;
  return {
    contexto: { tipo: 'campamento_bandidos', campamentoId: campamento.id, punto },
    punto,
    iniciadaPor,
    atacante: bandoDeColumna(estado, atacante, heridos),
    defensor: { faccionId: null, participantes: [], escuadrasSinHeroe: [tropaSinDueno(`${campamento.id}-tropa`, BATALLA.tropaBandidos)] },
    bloqueo: { ejercitoIds: [atacante.id], caravanaIds: [], campamentoId: campamento.id },
    mapa: enElMapa(estado, punto),
  };
}

/** En un asedio defienden los residentes sanos que están dentro, con su loadout activo, y la guarnición sin héroe
 * (Doc 5.15.1, 5.15.3). Sin nadie, se juega igual, sin defensores. */
export function aperturaDeAsedio(estado: GameSessionState, ejercito: Ejercito, plaza: Asentamiento, heridos: ReadonlySet<string>): Apertura {
  const defensores = heroesQueDefienden(plaza, estado.heroes, heridos).map((h) => {
    const loadout = new Set(h.loadouts.find((l) => l.activo)?.squadIds ?? []);
    return enBatalla(h, h.escuadrones.filter((e) => e.contenedor.tipo === 'campamento' && !e.enGuarnicion && loadout.has(e.id)));
  });
  return {
    contexto: { tipo: 'asedio', asentamientoId: plaza.id },
    punto: plaza.posicion,
    iniciadaPor: ejercito.liderId,
    atacante: bandoDeColumna(estado, ejercito, heridos),
    defensor: {
      faccionId: plaza.faccionId,
      participantes: defensores,
      escuadrasSinHeroe: guarnicionDe(plaza, estado.heroes).filter((e) => e.cantidad > 0).map(escuadraEnTicket),
    },
    bloqueo: { ejercitoIds: [ejercito.id], caravanaIds: [], asentamientoId: plaza.id },
    mapa: mapaDeAsedio(plaza),
  };
}

/** Llega a Unity si en toda la batalla hay algún héroe humano (Doc 5.15.6): NPC contra NPC sigue con números. */
export function hayHumano(apertura: Apertura): boolean {
  return [...apertura.atacante.participantes, ...apertura.defensor.participantes].some((p) => p.controlador === 'humano');
}

// --- El ciclo ---

/** Pone o quita el candado `reservaBatalla` a las escuadras indicadas. */
function conReserva(heroes: readonly Heroe[], ids: readonly string[], battleId: string | undefined): Heroe[] {
  const marcar = new Set(ids);
  const conCandado = ({ reservaBatalla: _, ...libre }: Escuadron): Escuadron => (battleId ? { ...libre, reservaBatalla: { battleId } } : libre);
  return heroes.map((h) =>
    h.escuadrones.some((e) => marcar.has(e.id)) ? { ...h, escuadrones: h.escuadrones.map((e) => (marcar.has(e.id) ? conCandado(e) : e)) } : h
  );
}

/**
 * UUID de una batalla (doc 01 §15). Las rutas de Conquest la nombran sin `gameId`, así que tiene que ser única entre
 * partidas: sale de la partida (su `gameId`, resumido) y de su contador de ids, sin aleatoriedad global (doc 10).
 */
export function idDeBatalla(gameId: string, n: number): string {
  const resumen = (semilla: number) => {
    let h = semilla;
    for (let i = 0; i < gameId.length; i++) h = Math.imul(h ^ gameId.charCodeAt(i), 0x01000193) >>> 0;
    return h.toString(16).padStart(8, '0');
  };
  const a = resumen(0x811c9dc5);
  const b = resumen(0x01000193);
  const c = n.toString(16).padStart(16, '0');
  return `${a}-${b.slice(0, 4)}-${b.slice(4)}-${c.slice(0, 4)}-${c.slice(4)}`;
}

/** Abre la batalla en `convocando`, con el ticket congelado y las escuadras reservadas, a la espera de que Conquest la
 * asigne (doc 01 §15). */
export function abrirBatalla(estado: GameSessionState, apertura: Apertura, ahora: Instante, id: string): { estado: GameSessionState; batalla: Batalla } {
  const asedio = apertura.contexto.tipo === 'asedio';
  const lado = (b: Bando): BattleSide => ({ ...b, capacidadMaxima: asedio ? BATALLA.capacidad.asedio : BATALLA.capacidad.resto });
  const batalla: Batalla = {
    id,
    estado: 'convocando',
    iniciadaPor: apertura.iniciadaPor,
    punto: apertura.punto,
    ticket: {
      schemaVersion: SCHEMA_VERSION,
      battleId: id,
      gameId: estado.gameId,
      ticketRevision: 0,
      contextoEstrategico: apertura.contexto,
      bandos: { atacante: lado(apertura.atacante), defensor: lado(apertura.defensor) },
      mapa: apertura.mapa,
      reglas: {
        duracionMaximaSegundos: (asedio ? BATALLA.duracionMinutos.asedio : BATALLA.duracionMinutos.resto) * 60,
        versionBalance: BALANCE_VERSION,
        versionCatalogoTropas: VERSION_CATALOGO_TROPAS,
        versionCatalogoHeroe: BATALLA.versionCatalogoHeroe,
        versionCatalogoObjetos: BATALLA.versionCatalogoObjetos,
      },
    },
    incorporaciones: [],
    bloqueo: apertura.bloqueo,
    expiraEn: sumar(ahora, minutos(BATALLA.plazoAsignacionMinutos)),
  };
  return {
    estado: { ...estado, batallas: [...estado.batallas, batalla], heroes: conReserva(estado.heroes, escuadrasConDueno(batalla), id) },
    batalla,
  };
}

/** Abre las batallas de los combates que el tick no resolvió, con el estado ya avanzado. */
export function abrirCombatesDelTick(
  estado: GameSessionState,
  combates: readonly CombatePorAbrir[],
  ahora: Instante,
  nuevoId: () => number
): { estado: GameSessionState; abiertas: Batalla[] } {
  const heridos = heridosEn(estado.heroes, ahora);
  let actual = estado;
  const abiertas: Batalla[] = [];
  for (const c of combates) {
    const apertura = aperturaDelTick(actual, c, heridos);
    if (!apertura) continue;
    const r = abrirBatalla(actual, apertura, ahora, idDeBatalla(estado.gameId, nuevoId()));
    actual = r.estado;
    abiertas.push(r.batalla);
  }
  return { estado: actual, abiertas };
}

function aperturaDelTick(estado: GameSessionState, c: CombatePorAbrir, heridos: ReadonlySet<string>): Apertura | undefined {
  const ejercito = estado.ejercitos.find((e) => e.id === c.ejercitoId);
  if (!ejercito) return undefined;
  if (c.tipo === 'asedio') {
    const plaza = estado.asentamientos.find((a) => a.id === c.asentamientoId);
    return plaza && aperturaDeAsedio(estado, ejercito, plaza, heridos);
  }
  if (c.tipo === 'columna') {
    const rival = estado.ejercitos.find((e) => e.id === c.rivalId);
    return rival && aperturaContraColumna(estado, ejercito, rival, ejercito.liderId, heridos);
  }
  const caravana = estado.caravanas.find((x) => x.id === c.caravanaId);
  return caravana && aperturaContraCaravana(estado, ejercito, caravana, ejercito.liderId, heridos);
}

/**
 * Unirse a una batalla que no ha terminado (Doc 5.15.1): la columna del héroe entra en el bando de su Facción con sus
 * héroes sanos y lo que llevan, si está a distancia de ataque del punto y el bando no se pasa de su capacidad. Cada
 * héroe es una incorporación; el ticket no cambia.
 */
export function unirseABatalla(estado: GameSessionState, batalla: Batalla, heroeId: string, ahora: Instante): { estado: GameSessionState; nuevas: IncorporacionBatalla[] } {
  const heridos = heridosEn(estado.heroes, ahora);
  if (heridos.has(heroeId)) throw new BatallaInvalidaError('Estás herido: no puedes entrar en batalla.');
  const bloqueos = bloqueosDe(estado, ahora);
  if (bloqueos.heroes.has(heroeId)) throw new BatallaInvalidaError('Ya estás en una batalla.');
  const columna = columnaDe(estado.ejercitos, heroeId);
  if (!columna) throw new BatallaInvalidaError('Hay que llegar con tu columna.');
  if (bloqueos.ejercitos.has(columna.id)) throw new BatallaInvalidaError('Tu columna ya está en una batalla.');
  if (distancia(columna.posicionActual, batalla.punto) > LOGISTICA.radioEncuentro) {
    throw new BatallaInvalidaError(`Hay que estar a menos de ${LOGISTICA.radioEncuentro} de la batalla.`);
  }
  const faccionId = estado.facciones.find((f) => esCiudadano(f, heroeId))?.id;
  const lado = (['atacante', 'defensor'] as const).find((l) => faccionId !== undefined && batalla.ticket.bandos[l].faccionId === faccionId);
  if (!lado) throw new BatallaInvalidaError('Tu Facción no combate en esta batalla.');

  const entran = heroesDeColumna(estado, columna, heridos);
  const yaDentro = participacionesDe(batalla).filter((p) => p.lado === lado).length;
  if (yaDentro + entran.length > batalla.ticket.bandos[lado].capacidadMaxima) throw new BatallaInvalidaError('Ese bando está lleno.');

  const nuevas: IncorporacionBatalla[] = entran.map((participante, i) => ({
    schemaVersion: SCHEMA_VERSION,
    battleId: batalla.id,
    secuencia: batalla.incorporaciones.length + i + 1,
    lado,
    participante,
  }));
  const actualizada: Batalla = {
    ...batalla,
    incorporaciones: [...batalla.incorporaciones, ...nuevas],
    bloqueo: { ...batalla.bloqueo, ejercitoIds: [...batalla.bloqueo.ejercitoIds, columna.id] },
  };
  const reservadas = nuevas.flatMap((n) => n.participante.escuadras.map((e) => e.squadId));
  return {
    estado: {
      ...estado,
      batallas: estado.batallas.map((b) => (b.id === batalla.id ? actualizada : b)),
      heroes: conReserva(estado.heroes, reservadas, batalla.id),
    },
    nuevas,
  };
}

/** Cierra sin aplicar nada: suelta los candados y nadie gana ni pierde (doc 01 §15). */
function cerrarSinResultado(estado: GameSessionState, batalla: Batalla, final: 'cancelada' | 'fallida'): GameSessionState {
  return {
    ...estado,
    heroes: conReserva(estado.heroes, escuadrasConDueno(batalla), undefined),
    batallas: estado.batallas.map((b) => (b.id === batalla.id ? { ...b, estado: final } : b)),
  };
}

/** Cancelar antes de que empiece la partida (doc 02 §3.1). Quién puede lo decide la matriz de autorización. */
export function cancelarBatalla(estado: GameSessionState, batalla: Batalla): GameSessionState {
  if (batalla.estado === 'en_curso') throw new BatallaInvalidaError('La partida ya empezó: no se puede cancelar.');
  return cerrarSinResultado(estado, batalla, 'cancelada');
}

/** Pasa a `fallida` las que vencieron su plazo: nadie la asignó, nadie la empezó o el resultado no llegó a tiempo. */
export function vencerBatallas(estado: GameSessionState, ahora: Instante): { estado: GameSessionState; vencidas: Batalla[] } {
  const vencidas = estado.batallas.filter((b) => ACTIVAS.has(b.estado) && ahora >= b.expiraEn);
  return { estado: vencidas.reduce((e, b) => cerrarSinResultado(e, b, 'fallida'), estado), vencidas };
}

// --- Lo que manda Conquest (doc 02 §3.3) ---

function conBatalla(estado: GameSessionState, batalla: Batalla): GameSessionState {
  return { ...estado, batallas: estado.batallas.map((b) => (b.id === batalla.id ? batalla : b)) };
}

/** Los tokens son solo para los héroes humanos de la batalla: los bot no se conectan (doc 01 §15). */
function exigirTokensDeHumanos(batalla: Batalla, tokens: readonly TokenParticipante[]): void {
  const humanos = new Set(participacionesDe(batalla).filter((p) => p.participante.controlador === 'humano').map((p) => p.participante.heroeId));
  if (tokens.some((t) => !humanos.has(t.heroeId))) throw new BatallaInvalidaError('Hay un token para quien no es un héroe humano de esta batalla.');
}

function exigirRevisionVigente(batalla: Batalla, ticketRevision: number): void {
  if (ticketRevision !== batalla.ticket.ticketRevision) throw new BatallaInvalidaError('Esa revisión del ticket ya no vale.');
}

/** Solo el servidor que registró la asignación, con ese mismo intento, sigue hablando por ella (doc 01 §15, R03). */
function exigirAsignacionActiva(batalla: Batalla, intentoAsignacionId: string, servidorId: string): NonNullable<Batalla['asignacion']> {
  const asignacion = batalla.asignacion;
  if (!asignacion || asignacion.intentoAsignacionId !== intentoAsignacionId || asignacion.servidorId !== servidorId) {
    throw new BatallaInvalidaError('Esa no es la asignación activa de esta batalla.');
  }
  return asignacion;
}

/**
 * Conquest dice dónde se juega (doc 02 §3.3). Gana la primera asignación válida: repetirla con el mismo intento no
 * cambia nada, y otra distinta se rechaza. Desde aquí corre el plazo para que empiece.
 */
export function registrarAsignacion(
  estado: GameSessionState,
  batalla: Batalla,
  asignacion: BattleServerAssignment,
  servidorId: string,
  ahora: Instante
): GameSessionState {
  if (batalla.asignacion?.intentoAsignacionId === asignacion.intentoAsignacionId && batalla.asignacion.servidorId === servidorId) return estado;
  if (batalla.estado !== 'convocando') throw new BatallaYaAsignadaError('La batalla ya tiene servidor.');
  exigirRevisionVigente(batalla, asignacion.ticketRevision);
  exigirTokensDeHumanos(batalla, asignacion.tokensParticipante);
  return conBatalla(estado, {
    ...batalla,
    estado: 'asignada',
    asignacion: { ...asignacion, servidorId },
    expiraEn: sumar(ahora, minutos(BATALLA.plazoInicioMinutos)),
  });
}

/** La partida empezó de verdad (doc 02 §3.3). Desde aquí el plazo es su duración más el margen. */
export function confirmarInicio(estado: GameSessionState, batalla: Batalla, inicio: InicioBatalla, servidorId: string, ahora: Instante): GameSessionState {
  exigirAsignacionActiva(batalla, inicio.intentoAsignacionId, servidorId);
  exigirRevisionVigente(batalla, inicio.ticketRevision);
  if (batalla.estado === 'en_curso') return estado;
  const plazo = batalla.ticket.reglas.duracionMaximaSegundos / 60 + BATALLA.margenMinutos;
  return conBatalla(estado, { ...batalla, estado: 'en_curso', iniciadaEn: ahora, expiraEn: sumar(ahora, minutos(plazo)) });
}

/** Los tokens de quienes se unieron después de la asignación (doc 02 §3.3). Uno nuevo para el mismo héroe sustituye al
 * anterior. */
export function registrarTokens(estado: GameSessionState, batalla: Batalla, tokens: TokensBatalla, servidorId: string): GameSessionState {
  const asignacion = exigirAsignacionActiva(batalla, tokens.intentoAsignacionId, servidorId);
  exigirTokensDeHumanos(batalla, tokens.tokensParticipante);
  const porHeroe = new Map([...asignacion.tokensParticipante, ...tokens.tokensParticipante].map((t) => [t.heroeId, t]));
  return conBatalla(estado, { ...batalla, asignacion: { ...asignacion, tokensParticipante: [...porHeroe.values()] } });
}

// --- Lo que se cuenta ---

export interface PayloadBatalla {
  battleId: string;
  contexto: ContextoEstrategico;
}

/** Un evento por cada plaza implicada (la de cada columna y caravana, y la asediada): se entera cada hogar, no el mundo. */
export function eventosDeBatalla(
  estado: GameSessionState,
  batalla: Batalla,
  codigo: string,
  mensaje: string
): { codigo: string; mensaje: string; payload: PayloadBatalla; asentamientoId: string }[] {
  const plazas = new Set([
    ...estado.ejercitos.filter((e) => batalla.bloqueo.ejercitoIds.includes(e.id)).map((e) => e.origenAsentamientoId),
    ...estado.caravanas.filter((c) => batalla.bloqueo.caravanaIds.includes(c.id)).map((c) => c.origenAsentamientoId),
    ...(batalla.bloqueo.asentamientoId ? [batalla.bloqueo.asentamientoId] : []),
  ]);
  const payload: PayloadBatalla = { battleId: batalla.id, contexto: batalla.ticket.contextoEstrategico };
  return [...plazas].map((asentamientoId) => ({ codigo, mensaje, payload, asentamientoId }));
}

/** Lo que un jugador ve de una batalla en el mapa (doc 02 §4.1): estado público, sin tokens ni escuadras. */
export interface BatallaVisible {
  battleId: string;
  estado: EstadoBatalla;
  contexto: ContextoEstrategico;
  punto: Point;
  bandos: Record<LadoId, { faccionId: string | null; heroes: number; capacidadMaxima: number }>;
  /** El bando del jugador, si combate en ella. */
  ladoPropio?: LadoId;
}

export function batallaVisible(b: Batalla, heroeId: string): BatallaVisible {
  const participaciones = participacionesDe(b);
  const bando = (lado: LadoId) => ({
    faccionId: b.ticket.bandos[lado].faccionId,
    heroes: participaciones.filter((p) => p.lado === lado).length,
    capacidadMaxima: b.ticket.bandos[lado].capacidadMaxima,
  });
  const propio = participaciones.find((p) => p.participante.heroeId === heroeId)?.lado;
  return {
    battleId: b.id,
    estado: b.estado,
    contexto: b.ticket.contextoEstrategico,
    punto: b.punto,
    bandos: { atacante: bando('atacante'), defensor: bando('defensor') },
    ...(propio ? { ladoPropio: propio } : {}),
  };
}
