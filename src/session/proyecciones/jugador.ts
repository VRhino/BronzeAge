// Proyección de jugador (Fase C4, Slice 1 — Docs/Arquitectura/6_Sincronizacion_Visibilidad_y_Escala.md §3).
// Filtra `GameSessionState` a lo que un jugador concreto puede ver.
//
// Regla de base, deliberadamente conservadora: la Facción propia se ve COMPLETA (asentamientos, escuadrones,
// colas, almacén); las demás Facciones no aportan ni un asentamiento, ni siquiera resumido. Mejor "no ves
// nada del rival" que exponer un nivel de detalle que nadie ha decidido que sea seguro. Lo que SÍ viaja de
// todas las Facciones son los metadatos ya públicos en la ficción del juego (nombre, nivel, reputación,
// Rey/Embajador) — sin ellos la pantalla de diplomacia no tendría con qué pintarse.
//
// **La excepción es lo que se VE**, y viaja siempre REDACTADO: los ejércitos ajenos (`ejercitosAvistados`,
// Doc 5.12.7) y los asentamientos ajenos (`asentamientosAvistados`, niebla de guerra Paso 1). De ambos se
// proyecta lo que se distingue desde fuera —quién es y dónde está— y nada de su interior. Es la misma
// ficción en los dos casos: una columna cruza campo abierto a la vista de quien vigile ese campo, y una
// ciudad no se puede esconder.
//
// A eso se le suma la MEMORIA (`memoriaPorFaccion`, `engine/memoria.ts`), que es lo que produce los tres
// estados que el jugador ve en pantalla:
//
//   1. Nunca visto -> la celda no está en `exploracion`. El cliente de jugador lo tapa.
//   2. Visto antes -> `asentamientosConocidos`, la última foto con su `conocidoEn`.
//   3. Viéndolo    -> `asentamientosAvistados` / `ejercitosAvistados`, en vivo.
//
// Y el tránsito sale solo: al dejar de ver algo, deja de estar en la lista de en vivo y se queda la última
// foto. Cuando las dos coinciden gana la de en vivo — estar mirándolo es mejor información que recordarlo.
//
// Lo que sigue faltando es la visión compartida por ALIANZA (Paso 4).
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
import { VISION } from '../../constants';
import { distancia } from '../../world/geometria';
import type { EstadoMapa } from '../../world/mapa';
import type { TrazadoAsentamiento } from '../../engine/trazado';
import type { Instante } from '../../domain/tiempo';
import { esCiudadano } from '../../engine/faccion';
// El mismo recuento que usa el motor para los carros (Doc 5.13): un participante es un carro Y un rombo.
import { participantesDe } from '../../engine/ejercitos';
import { marcarVisto, proyectarNiebla, rejillaDe, SIN_EXPLORAR, type NieblaProyectada } from '../../engine/exploracion';
import { MEMORIA_VACIA, type FichaConocida } from '../../engine/memoria';
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

/**
 * Un asentamiento AJENO tal como se ve desde fuera: su FICHA (decisión del usuario, 2026-09-04). Quién es,
 * de quién es, dónde está y cómo de grande — que es exactamente lo que se distingue mirando una ciudad.
 *
 * El `nivel` SÍ entra: una ciudad grande se ve grande, y no dice cuánta tropa tiene dentro, que es lo que
 * decidiría un ataque. Queda fuera a propósito todo lo demás —almacén, escuadrones, edificios, colas,
 * cargos, trazado urbano—: eso es telemetría de un rival, y es justo lo que esta proyección existe para
 * impedir.
 */
export interface AsentamientoAvistado {
  id: string;
  /** Opcional por el mismo motivo que en `Asentamiento`: ausente = el cliente muestra el `id`. */
  nombre?: string;
  faccionId: string;
  posicion: Point;
  nivel: number;
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
  /** SOLO los de la Facción propia, COMPLETOS. Lo ajeno que se vea va aparte, en `asentamientosAvistados`. */
  asentamientos: Asentamiento[];
  /** Los de CUALQUIER otra Facción que se estén viendo AHORA, redactados a su ficha (ver
   * `AsentamientoAvistado`). Array aparte y no mezclado con `asentamientos`, por el mismo motivo que
   * `ejercitosAvistados`: la diferencia entre "lo veo entero" y "solo lo avisto" es de TIPO, no un campo
   * opcional que el cliente pueda olvidarse de mirar. */
  asentamientosAvistados: AsentamientoAvistado[];
  /** Las que se vieron ALGUNA VEZ y ahora no se ven: la última foto, con el instante en que se tomó (ver
   * `FichaConocida`). Nunca repite lo que ya está en `asentamientosAvistados` — cuando algo se ve y además se
   * recuerda, gana lo que se ve. Vacío para un jugador sin Facción: la memoria es de la Facción. */
  asentamientosConocidos: FichaConocida[];
  /** Las DOS máscaras de celdas —lo explorado alguna vez y lo visible ahora— con las que el cliente
   * distingue los tres estados (ver `NieblaProyectada`). **Quien las aplica es el CLIENTE DE JUGADOR**
   * (decisión del usuario, 2026-09-04), no el servidor. La geografía no es información táctica —es la misma para todos
   * y el cliente la cachea para siempre por su `mapaId`—, así que ocultarla aquí rompería esa caché a cambio
   * de nada: lo que no puede salir del servidor son las ENTIDADES, y eso ya se filtra arriba. El cliente de
   * ADMINISTRACIÓN, que es herramienta de operación y no un jugador, lo ve todo sin máscara.
   *
   * Incluye lo que se está viendo AHORA aunque el tick todavía no lo haya grabado, para que la máscara nunca
   * deje un agujero justo donde el jugador está mirando. */
  exploracion: NieblaProyectada;
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
 * Fuente ESPACIAL de la niebla de guerra: un jugador ve de lo ajeno lo que sus plazas vigilan —su radio de
 * influencia MÁS `VISION.margenAsentamiento`, como una atalaya que mira algo más allá de la frontera— y lo
 * que sus ejércitos alcanzan a ver mientras marchan, `VISION.ejercito` a la redonda.
 *
 * Un detalle que conviene tener presente: se mide contra `radioPotencial`, el DISCO, no contra el polígono de
 * la zona. La zona está recortada por las fronteras con Facciones rivales (`computeZonaInfluencia`) y ese
 * recorte es político, no óptico: que un rival tenga su frontera pegada a tu ciudad no ciega a tus vigías —
 * si acaso lo contrario. Como el polígono siempre está contenido en el disco, esto solo ensancha la vista,
 * nunca la recorta.
 *
 * Sin memoria, a propósito: esto responde "¿se ve AHORA?" y nada más. El "último conocido" —recordar lo que
 * viste cuando dejas de verlo— es la otra mitad de la mecánica (Pasos 2-3). Mientras tanto lo ajeno aparece y
 * desaparece del mapa, que es conservador en la dirección correcta: se filtra de menos, nunca de más.
 */
function seVeAhora(
  punto: Point,
  asentamientosPropios: readonly Asentamiento[],
  ejercitosPropios: readonly Ejercito[]
): boolean {
  return (
    asentamientosPropios.some((a) => distancia(punto, a.posicion) <= a.radioPotencial + VISION.margenAsentamiento) ||
    ejercitosPropios.some((e) => distancia(punto, e.posicionActual) <= VISION.ejercito)
  );
}

/**
 * Las dos máscaras de niebla que viajan al cliente (ver `NieblaProyectada`): lo VISIBLE ahora mismo, y lo
 * EXPLORADO alguna vez —que es lo que la Facción tiene grabado MÁS lo visible—.
 *
 * Las dos se calculan con los mismos ojos y el mismo `marcarVisto` que usa el motor al grabar, en vez de con
 * una versión propia: si el criterio de "hasta dónde ve una plaza" cambiara, cambiaría en los dos sitios a la
 * vez porque es el mismo código.
 *
 * Y unir lo grabado con lo visible no es redundante. El tick es quien graba, así que entre un comando y el
 * siguiente tick hay una ventana en la que lo recién visto —una plaza recién fundada, por ejemplo— todavía no
 * está en la memoria. Sin esta unión el cliente pintaría niebla justo encima de lo que el jugador acaba de
 * hacer, que es la clase de agujero que nadie relaciona con un desfase de un tick.
 */
function nieblaDe(
  grabada: string,
  estado: GameSessionState,
  asentamientosPropios: readonly Asentamiento[],
  ejercitosPropios: readonly Ejercito[]
): NieblaProyectada {
  const rejilla = rejillaDe(estado.mapa.config);
  let visibles = SIN_EXPLORAR;
  for (const a of asentamientosPropios) visibles = marcarVisto(visibles, rejilla, a.posicion, a.radioPotencial + VISION.margenAsentamiento);
  for (const e of ejercitosPropios) visibles = marcarVisto(visibles, rejilla, e.posicionActual, VISION.ejercito);

  let celdas = grabada;
  for (const a of asentamientosPropios) celdas = marcarVisto(celdas, rejilla, a.posicion, a.radioPotencial + VISION.margenAsentamiento);
  for (const e of ejercitosPropios) celdas = marcarVisto(celdas, rejilla, e.posicionActual, VISION.ejercito);

  return proyectarNiebla(celdas, visibles, rejilla);
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

  const avistados = estado.asentamientos
    .filter((a) => !esPropio(a.id) && seVeAhora(a.posicion, asentamientosPropios, ejercitosPropios))
    .map((a) => ({ id: a.id, nombre: a.nombre, faccionId: a.faccionId, posicion: a.posicion, nivel: a.nivel }));
  const seVe = new Set(avistados.map((a) => a.id));

  const memoria = (faccionId !== null ? estado.memoriaPorFaccion[faccionId] : undefined) ?? MEMORIA_VACIA;

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
    asentamientosAvistados: avistados,
    // Lo recordado MENOS lo que se ve ahora, y menos lo que entretanto pasó a ser propio (eso viaja completo
    // en `asentamientos`). Cada plaza aparece en una lista o en la otra, nunca en las dos.
    asentamientosConocidos: Object.values(memoria.asentamientos).filter((f) => !seVe.has(f.asentamientoId) && !esPropio(f.asentamientoId)),
    exploracion: nieblaDe(memoria.exploracion, estado, asentamientosPropios, ejercitosPropios),
    caravanas: estado.caravanas.filter((c) => esPropio(c.origenAsentamientoId) || (c.destinoAsentamientoId !== undefined && esPropio(c.destinoAsentamientoId))),
    ejercitos: ejercitosPropios,
    ejercitosAvistados: estado.ejercitos
      .filter((e) => !propios.has(e.id) && seVeAhora(e.posicionActual, asentamientosPropios, ejercitosPropios))
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
