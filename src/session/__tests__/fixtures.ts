// Fixtures compartidas por los tests de `session/comandos/*` (revisión de duplicación 2026-08-25):
// `partidaConAsentamiento` estaba clonada casi byte a byte en 4 archivos de test distintos (cargos,
// construcción, militar, expansión) — mismo seed, misma Facción, mismo punto de fundación, y solo cambiaba
// el `gameId` (cosmético, para distinguir sesiones en un log si hiciera falta depurar).
import { GameSession } from '../gameSession';
import { crearFaccion } from '../comandos/crearFaccion';
import { fundarAsentamiento } from '../comandos/fundarAsentamiento';
import { comprarCasa } from '../comandos/cargos';
import { REGISTRO_COMANDOS } from '../comandos/registro';
import { campamentoDe } from '../../engine/tropa';
import { instanteDeTick, isoDeInstante } from '../estado';
import { columnaDeAparicion, ubicacionDeducida } from '../../engine/ubicacion';
import { heroeDePrueba } from '../../engine/__tests__/fixtures';
import { crearMapa } from '../../world/mapa';
import { createRng, restaurarRng } from '../../worldgen';

/** `EventoDominio.momento` (ISO 8601) de un comando sobre una partida recién creada — instante del tick 0
 * (= `SIMULACION.epocaInicial`). Ya no se inyecta un `momento`, lo deriva `GameSession` del tick (Fase D /
 * doc 10). */
export const MOMENTO = isoDeInstante(instanteDeTick(0));
export const ACTOR = 'jugador-test';
export const OPC = { actor: ACTOR };

/** Segundo residente del asentamiento de la fixture, que entra comprando casa. */
export const VECINO = 'jugador-vecino';

/**
 * Da de alta al héroe `heroeId` con un id elegido por el test, para que el test siga actuando con
 * `{ actor: heroeId }`. Lo sitúa donde el mundo ya lo pone (su columna o su residencia, `ubicacionDeducida`)
 * y, si no está en ninguna parte, lo hace APARECER con su columna, como `crearHeroe` (Doc 1.3). Idempotente.
 *
 * Tira del RNG de la propia partida, y lo deja avanzado: así aparece donde aparecía con el alta perezosa que
 * sustituye, y los tests que dependen de dónde cae una columna no cambian de mundo.
 */
export function conHeroe(sesion: GameSession, heroeId: string): GameSession {
  const payload = sesion.exportar();
  const { state } = payload;
  if (state.heroes.some((h) => h.id === heroeId)) return sesion;
  const rng = payload.estadoRng !== undefined ? restaurarRng(payload.estadoRng) : createRng(state.mapa.config.seed);
  const deducida = ubicacionDeducida(heroeId, state.asentamientos, state.ejercitos);
  const columna =
    deducida.tipo === 'desconectado'
      ? columnaDeAparicion(`ejercito-${heroeId}`, heroeId, crearMapa(state.mapa, state.estadoMapa), state.asentamientos, rng, instanteDeTick(state.tick))
      : undefined;
  const heroe = heroeDePrueba(heroeId, columna ? { tipo: 'columna', ejercitoId: columna.id } : deducida);
  return GameSession.importar({
    ...payload,
    estadoRng: rng.estado(),
    state: { ...state, heroes: [...state.heroes, heroe], ejercitos: columna ? [...state.ejercitos, columna] : state.ejercitos },
  });
}

/**
 * Planta la columna de un jugador en un punto concreto, DEVOLVIENDO una sesión nueva (`GameSession` no
 * expone ninguna forma de mover una columna sin pasar por un comando de movimiento real).
 *
 * Existe porque se funda donde se está (Doc 1.3): un test que quiera fundar en un sitio elegido —o construir
 * geometría relativa a un punto fijo, como "una plaza rival a tal distancia de la propia"— tiene que LLEVAR
 * ahí a su fundador antes. Caminar de verdad costaría ticks que ningún test de estos mide.
 */
export function enPie(sesion: GameSession, heroeId: string, punto: { x: number; y: number }): GameSession {
  const payload = sesion.exportar();
  const columna = payload.state.ejercitos.find((e) => e.participantes.some((p) => p.heroeId === heroeId));
  if (!columna) throw new Error(`el fixture esperaba que ${heroeId} tuviera columna: ¿ha ejecutado algún comando?`);
  return GameSession.importar({
    ...payload,
    state: {
      ...payload.state,
      ejercitos: payload.state.ejercitos.map((e) => (e.id === columna.id ? { ...e, posicionActual: punto } : e)),
    },
  });
}

/**
 * Partida con una Facción y un asentamiento fundado (seed 42, en `{x:400,y:400}`).
 *
 * El fundador es el ACTOR de `OPC`: desde que `fundarAsentamiento` dejó de fabricar fundadores ficticios
 * (`jugador-<faccionId>-<n>`), funda quien ejecuta el comando, y con ello se gana casa y ciudadanía. Se
 * devuelve `fundador` igualmente para no obligar a cada test a resolverlo.
 *
 * `vecino` es un SEGUNDO residente que entra por la otra vía que admite el juego: comprar casa (Doc 2.5).
 * Antes salía de una fundación grupal con jugadores inventados; ahora se gana la residencia como lo haría una
 * persona. Lo usan los tests que necesitan distinguir "reside aquí" de "manda aquí".
 */
export function partidaConAsentamiento(gameId = 'test'): {
  sesion: GameSession;
  faccionId: string;
  asentamientoId: string;
  fundador: string;
  vecino: string;
} {
  let sesion = conHeroe(GameSession.crear(gameId, { seed: 42 }), ACTOR);
  const rf = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC);
  const faccionId = rf.datos!.faccionId;
  // Se funda DONDE SE ESTA (Doc 1.3): se lleva al fundador a un punto fijo antes de fundar, ya no es un
  // parámetro del comando. (400,400) y no (500,500): en la seed 42 ese punto es AGUA, y desde que el agua es
  // infranqueable (2026-09-02) fundar ahí ya no es legal — el fixture llevaba fundando en el mar sin que se
  // notara, porque nada dependía del terreno. Este está en llano y es `recomendable`.
  sesion = enPie(sesion, ACTOR, { x: 400, y: 400 });
  const ra = sesion.ejecutar(fundarAsentamiento, { faccionId }, OPC);
  const asentamientoId = ra.datos!.asentamientoId;
  const fundador = sesion.getState().asentamientos[0]!.heroesFundadoresIds[0]!;
  sesion.ejecutar(comprarCasa, { asentamientoId, heroeId: VECINO }, OPC);
  return { sesion: conHeroe(sesion, VECINO), faccionId, asentamientoId, fundador, vecino: VECINO };
}

/**
 * Almacén lleno y 200 pesants en el (único) asentamiento, para poder reclutar. Un recién fundado no puede, por dos
 * reglas reales del motor: la milicia recluta 25 de golpe y la plaza arranca con 20 pesants, y reclutar exige
 * reserva de trigo y equipo que el almacén inicial no cubre. No hay comandos para rellenar almacén ni población,
 * así que se construye el estado de una partida ya en marcha (vía `importar()`).
 */
export function abastecer(original: GameSession): GameSession {
  const payload = original.exportar();
  const asentamiento = payload.state.asentamientos[0]!;
  const almacen = Object.fromEntries(
    Object.entries(asentamiento.almacen).map(([recurso, item]) => [recurso, { ...item, cantidad: item.capacidad }])
  );
  return GameSession.importar({
    ...payload,
    state: {
      ...payload.state,
      asentamientos: [{ ...asentamiento, almacen, poblacion: { ...asentamiento.poblacion, pesants: 200 } }],
    },
  });
}

/**
 * Fundador y vecino, compañeros de Facción, cada uno en su columna con su milicia, junto a un campamento de bandidos
 * (`camp-1`). Con `unity`, la partida corre como en un servidor con servidores de batalla declarados (doc 01 §15).
 */
export function frenteACampamento(unity = true) {
  const base = partidaConAsentamiento();
  const { asentamientoId, fundador, vecino } = base;
  let sesion = base.sesion;
  for (const heroeId of [fundador, vecino]) {
    sesion = abastecer(sesion);
    const reclutado = sesion.ejecutar(REGISTRO_COMANDOS.reclutarTropa, { asentamientoId, heroeId, tropaId: 'milicia_lanceros', origen: 'pesants' }, { actor: heroeId });
    if (!reclutado.ok) throw new Error(`setup: ${heroeId} no recluta (${reclutado.codigoError})`);
    sesion = abastecer(sesion);
    const suyas = campamentoDe(sesion.getState().asentamientos[0]!, sesion.getState().heroes).filter((e) => e.heroeId === heroeId);
    const salida = sesion.ejecutar(
      REGISTRO_COMANDOS.salirAlMundo,
      { asentamientoId, heroeId, escuadronIds: suyas.map((e) => e.id), carga: { trigo: 60 } },
      { actor: heroeId }
    );
    if (!salida.ok) throw new Error(`setup: ${heroeId} no sale (${salida.codigoError})`);
  }

  const payload = sesion.exportar();
  const columnaDe = (heroeId: string) => payload.state.ejercitos.find((e) => e.participantes.some((p) => p.heroeId === heroeId))!;
  const posicion = columnaDe(fundador).posicionActual;
  const conCampamento = GameSession.importar(
    { ...payload, state: { ...payload.state, campamentosBandidos: [{ id: 'camp-1', posicion, bosqueId: 'b1', asentamientoId, poder: 30 }] } },
    { batallasEnUnity: unity }
  );
  return { sesion: conCampamento, fundador, vecino, columna: columnaDe(fundador).id, columnaVecino: columnaDe(vecino).id };
}
