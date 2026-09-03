// Proyección de jugador (Fase C4, Slice 1 — Docs/Arquitectura/6_Sincronizacion_Visibilidad_y_Escala.md §3).
// Filtra `GameSessionState` a lo que un jugador concreto puede ver.
//
// Regla de base, deliberadamente conservadora: la Facción propia se ve COMPLETA (asentamientos, escuadrones,
// colas, almacén); las demás Facciones no aportan ni un asentamiento, ni siquiera resumido. Mejor "no ves
// nada del rival" que exponer un nivel de detalle que nadie ha decidido que sea seguro. Lo que SÍ viaja de
// todas las Facciones son los metadatos ya públicos en la ficción del juego (nombre, nivel, reputación,
// Rey/Embajador) — sin ellos la pantalla de diplomacia no tendría con qué pintarse.
//
// **La única excepción son los EJÉRCITOS ajenos** (`ejercitosAvistados`), y solo desde que Doc 5.12.7 fijó
// el número que faltaba: un ejército se ve si entra en tu territorio o si cae dentro del radio de visión de
// uno de los tuyos. Viajan REDACTADOS —posición, Facción y nº de estandartes, nada más— porque una columna
// en campaña es, por la ficción, visible: cruza campo abierto a la vista de quien vigile ese campo. Un
// asentamiento rival, en cambio, sigue sin proyectarse en absoluto.
//
// Lo que sigue faltando de la niebla de guerra (§12) es la MEMORIA: aquí se responde "¿se ve AHORA?", no
// "¿qué recuerdo de la última vez que lo vi?". Eso es `ConocimientoJugador` y sigue diferido, junto con las
// otras dos fuentes de visibilidad que diseña el doc 6 (contacto y alianza).
import type {
  AcuerdoTrueque,
  Asentamiento,
  CaminoComercial,
  CampamentoBandido,
  Caravana,
  Ejercito,
  Faccion,
  OrdenMercado,
  Point,
  RelacionPolitica,
  Titulo,
  ZonaFaccion,
  ZonaInfluencia,
} from '../../domain/types';
import { LOGISTICA } from '../../constants';
import { distancia, pointInPolygon } from '../../world/geometria';
import type { EstadoMapa } from '../../world/mapa';
import type { TrazadoAsentamiento } from '../../engine/trazado';
import type { Instante } from '../../domain/tiempo';
import { esCiudadano } from '../../engine/faccion';
// El mismo recuento que usa el motor para los carros (Doc 5.13): un participante es un carro Y un rombo.
import { participantesDe } from '../../engine/ejercitos';
import {
  eventosDesde,
  idDeMapa,
  instanteDeTick,
  type EventoDominioConVersion,
  type EventoLogAdmin,
  type GameSessionState,
  type GeometriaAsentamientos,
} from '../estado';

/**
 * Un ejército AJENO tal como se ve desde fuera (Doc 5.12.7): dónde está, de qué Facción es y cuántos
 * estandartes se le cuentan —el número de rombos del mapa, Doc 5.12.2—. Nada más.
 *
 * Lo que deliberadamente NO lleva, y por qué:
 * - `escuadrones`: es la composición y, con ella, el poder exacto de la columna. Es la telemetría de rival
 *   que el doc prohíbe explícitamente; saber que vienen 200 hombres no es lo mismo que saber que son 200
 *   arqueros con la moral por los suelos.
 * - `ruta` y `objetivo`: son su INTENCIÓN. Ver pasar un ejército no es leerle el plan de campaña.
 * - `estado`: que vuelva a casa, acampe o siga avanzando es una decisión de su jugador, no algo que se
 *   distinga a la vista de una columna en movimiento.
 * - `origenAsentamientoId` y `suministro`: de dónde salió y cuánto aguanta. Lo segundo es directamente la
 *   respuesta a "¿me basta con esperar a que se le acabe el trigo?".
 *
 * Se redacta AQUÍ y no en el cliente: lo que no sale del servidor no se puede mirar en un DevTools.
 */
export interface EjercitoAvistado {
  id: string;
  faccionId: string;
  posicionActual: Point;
  /** Jugadores distintos que marchan en él. Es el único dato de "tamaño" que viaja. */
  participantes: number;
}

export interface ProyeccionJugador {
  gameId: string;
  /** Instante de MUNDO "ahora" de la partida (doc 10) — `instanteDeTick(estado.tick)`, derivado, no
   * almacenado. La ÚNICA referencia temporal del contrato (Fase D cerrada): con esto el cliente pinta cuentas
   * atrás localmente (`completaEn - instante`, `expiraEn - instante`). El `tick` interno del motor no viaja. */
  instante: Instante;
  version: number;
  jugadorId: string;
  /** Derivado de `Faccion.ciudadanosIds` en el momento de proyectar — nunca almacenado (ver `Membresia` en
   * `acceso/tipos.ts`: es la misma razón por la que se retiró de ahí). `null` antes de unirse a una. */
  faccionId: string | null;
  /**
   * Identidad del mapa (Fase C11, doc 9), NO el mapa en sí. Geografía del mundo: no es secreta (todos ven el
   * mismo terreno) — pero tampoco cambia NUNCA durante la partida (125 KB medidos, idénticos byte a byte en
   * todo el tick 0-200, doc 6 §6.4), así que mandarla completa en cada comando/proyección era pura repetición.
   * El cliente lo pide una vez, por `GET .../mapa/:mapaId`, cacheable para siempre porque el id ya captura su
   * identidad — y compara este campo en cada proyección para saber si ese mapa sigue siendo el vigente.
   */
  mapaId: string;
  estadoMapa: EstadoMapa;
  /** Todas las Facciones, sin redactar: nada en `Faccion` (nombre, nivel, reputación, Rey/Embajador,
   * ciudadanía) es información táctica — es el mismo tipo de dato público que "quién gobierna Troya" en la
   * ficción del juego. Lo táctico/económico vive en `Asentamiento`, que sí se filtra. */
  facciones: Faccion[];
  /** SOLO los de la Facción propia (Slice 1). El Slice 2 añade aquí lo visible por espacio/contacto/alianza. */
  asentamientos: Asentamiento[];
  caravanas: Caravana[];
  /** Los de la Facción propia, COMPLETOS — mismo criterio que `asentamientos`: de lo tuyo se ve todo. */
  ejercitos: Ejercito[];
  /** Los de CUALQUIER otra Facción que se estén viendo ahora mismo, redactados (ver `EjercitoAvistado`).
   * Van en un array aparte y no mezclados con `ejercitos` a propósito: la diferencia entre "lo veo entero"
   * y "solo lo avisto" es de tipo, no de un campo opcional que el cliente pueda olvidarse de mirar. */
  ejercitosAvistados: EjercitoAvistado[];
  acuerdos: AcuerdoTrueque[];
  ordenes: OrdenMercado[];
  /** Las relaciones diplomáticas son públicas por naturaleza — quién está aliado o es vasallo de quién no es
   * un secreto en ningún juego de estrategia. */
  relaciones: RelacionPolitica[];
  /** Títulos/ranking (Doc 2.6-ish): son competitivos por diseño — un ranking que no se puede ver no sirve
   * como ranking. */
  titulos: Titulo[];
  /** Infraestructura del mundo (rutas comerciales trazadas), no actividad económica en curso: se trata como
   * el mapa, no como los `acuerdos`/`ordenes` que sí filtran. */
  caminos: CaminoComercial[];
  /** Entidades del MUNDO (bandidos), no de ninguna Facción — visibles para cualquiera que pueda atacarlas. */
  campamentosBandidos: CampamentoBandido[];
  /** Sin `asentamientoId` (eventos globales/de Facción) o con uno propio. Es el mismo criterio que evita la
   * fuga que el doc 7 §7.1 señalaba en el log administrativo: el log global narra TODO el mundo. */
  eventosDominio: EventoDominioConVersion[];
  historial: EventoLogAdmin[];
  /** Geometría por frame (Fase C10, doc 9) — SOLO de los asentamientos propios, mismo criterio de "mejor no
   * ver nada del rival" que el resto de esta proyección: `computeZonaInfluencia` necesita la posición de
   * TODOS los asentamientos del mundo para recortar contra rivales (entrada privilegiada), así que ni el
   * cálculo ni la zona resultante de un rival viajan aquí — a diferencia de `facciones`/`relaciones`, que sí
   * son públicas por diseño. Antes lo recalculaba `render()` en cada `mousemove` del cliente; ahora llega ya
   * resuelto, filtrado por `proyectarParaJugador`. */
  zonas: ZonaInfluencia[];
  /** La silueta fusionada de la Facción propia (como mucho una entrada) — nunca la de un rival. */
  zonasFusionadas: ZonaFaccion[];
  /** Trazado urbano de cada asentamiento propio, por id — de los rivales no hay ni metadatos (Slice 1: no se
   * ven en absoluto), así que tampoco hay trazado que filtrar para ellos. */
  trazadoPorAsentamiento: Record<string, TrazadoAsentamiento>;
  /** Precio de referencia por recurso — auditoría de doc 9 (2026-08-26): `calcularPrecioReferencia` necesita
   * el almacén de TODOS los asentamientos del mundo (entrada privilegiada), así que el jugador nunca podría
   * calcularlo aunque quisiera. Lo calcula `RunnerDePartida.preciosReferencia()` (caché de un minuto real,
   * impuro) y esta función NO lo rellena — igual que `mapaId`, es parte del contrato de wire pero lo añade
   * el llamador HTTP, ver `server/rutas/jugador.ts`. */
  preciosReferencia: Record<string, number>;
}

function faccionDe(estado: GameSessionState, jugadorId: string): string | null {
  return estado.facciones.find((f) => esCiudadano(f, jugadorId))?.id ?? null;
}

/** Lo que "propio" significa para un jugador (Slice 1: su Facción, nada de rivales) — factorizado para que
 * `proyectarParaJugador` y `eventosDominioParaJugador` (cursor, Fase C13) usen exactamente el mismo criterio,
 * en vez de que cada uno recalcule su propia versión y puedan divergir. */
function propioDeJugador(estado: GameSessionState, jugadorId: string) {
  const faccionId = faccionDe(estado, jugadorId);
  const asentamientosPropios = estado.asentamientos.filter((a) => a.faccionId === faccionId);
  const idsPropios = new Set(asentamientosPropios.map((a) => a.id));
  return { faccionId, asentamientosPropios, esPropio: (asentamientoId: string) => idsPropios.has(asentamientoId) };
}

/**
 * Fuente ESPACIAL de la niebla de guerra (Doc 5.12.7): un jugador ve de lo ajeno lo que entra en su
 * territorio —que vigila por definición— y lo que sus propios ejércitos alcanzan a ver mientras marchan,
 * `LOGISTICA.radioVisionEjercito` a la redonda.
 *
 * Sin memoria, a propósito: esto responde "¿se ve AHORA?" y nada más. El "último conocido" —recordar lo que
 * viste cuando dejas de verlo— es la otra mitad de §12 y necesita una entidad (`ConocimientoJugador`) que no
 * existe todavía. Mientras tanto un ejército rival aparece y desaparece del mapa, que es conservador en la
 * dirección correcta: se filtra de menos, nunca de más.
 */
function seVeAhora(punto: Point, zonasPropias: readonly ZonaInfluencia[], ejercitosPropios: readonly Ejercito[]): boolean {
  return (
    zonasPropias.some((z) => pointInPolygon(punto, z.poligono)) ||
    ejercitosPropios.some((e) => distancia(punto, e.posicionActual) <= LOGISTICA.radioVisionEjercito)
  );
}

export function proyectarParaJugador(
  estado: GameSessionState,
  jugadorId: string,
  geometria: GeometriaAsentamientos
): Omit<ProyeccionJugador, 'preciosReferencia'> {
  const { faccionId, asentamientosPropios, esPropio } = propioDeJugador(estado, jugadorId);

  // Un ejército es "propio" si es de tu Facción o si llevas tropa TUYA dentro. Lo segundo no es redundante:
  // un jugador huérfano (Doc 5.4) se queda sin Facción pero no sin los escuadrones que iban con él, y no
  // tendría sentido que dejara de ver la columna en la que va montado.
  const ejercitosPropios = estado.ejercitos.filter(
    (e) => (faccionId !== null && e.faccionId === faccionId) || e.escuadrones.some((esc) => esc.jugadorId === jugadorId)
  );
  const zonasPropias = geometria.zonas.filter((z) => esPropio(z.asentamientoId));
  const propios = new Set(ejercitosPropios.map((e) => e.id));

  return {
    gameId: estado.gameId,
    instante: instanteDeTick(estado.tick),
    version: estado.version,
    jugadorId,
    faccionId,
    mapaId: idDeMapa(estado.mapa),
    estadoMapa: estado.estadoMapa,
    facciones: estado.facciones,
    asentamientos: asentamientosPropios,
    caravanas: estado.caravanas.filter((c) => esPropio(c.origenAsentamientoId) || (c.destinoAsentamientoId !== undefined && esPropio(c.destinoAsentamientoId))),
    ejercitos: ejercitosPropios,
    ejercitosAvistados: estado.ejercitos
      .filter((e) => !propios.has(e.id) && seVeAhora(e.posicionActual, zonasPropias, ejercitosPropios))
      .map((e) => ({ id: e.id, faccionId: e.faccionId, posicionActual: e.posicionActual, participantes: participantesDe(e.escuadrones) })),
    acuerdos: estado.acuerdos.filter((a) => esPropio(a.asentamientoAId) || esPropio(a.asentamientoBId)),
    ordenes: estado.ordenes.filter((o) => esPropio(o.asentamientoId)),
    relaciones: estado.relaciones,
    titulos: estado.titulos,
    caminos: estado.caminos,
    campamentosBandidos: estado.campamentosBandidos,
    eventosDominio: estado.eventosDominio.filter((e) => e.asentamientoId === undefined || esPropio(e.asentamientoId)),
    historial: estado.historialJugadores[jugadorId] ?? [],
    zonas: zonasPropias,
    zonasFusionadas: geometria.zonasFusionadas.filter((zf) => zf.faccionId === faccionId),
    trazadoPorAsentamiento: Object.fromEntries(Object.entries(geometria.trazadoPorAsentamiento).filter(([id]) => esPropio(id))),
  };
}

/**
 * Eventos con `version` > `desde`, filtrados a lo que ESE jugador puede ver (Fase C13) — mismo criterio que
 * `proyectarParaJugador.eventosDominio`, factorizado con `propioDeJugador` para no divergir. Es lo que
 * permite a un cliente que ya tiene la proyección inicial ponerse al día tras un aviso por WebSocket sin
 * volver a pedir la proyección entera — solo los eventos nuevos.
 */
export function eventosDominioParaJugador(estado: GameSessionState, jugadorId: string, desde: number): EventoDominioConVersion[] {
  const { esPropio } = propioDeJugador(estado, jugadorId);
  return eventosDesde(estado, desde).filter((e) => e.asentamientoId === undefined || esPropio(e.asentamientoId));
}
