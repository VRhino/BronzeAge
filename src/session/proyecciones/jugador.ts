// Proyección de jugador (Fase C4, Slice 1 — Docs/Arquitectura/6_Sincronizacion_Visibilidad_y_Escala.md §3).
// Filtra `GameSessionState` a lo que un jugador concreto puede ver.
//
// Regla de base, desde el jugador situado (Doc 1.10, 2026-09-06): **se ve el interior de UNA plaza, la que
// se está pisando**, y de todas las demás solo su ficha. Ni siquiera de las de la propia Facción: la
// ciudadanía habilita, la presencia ejerce (Doc 2.5), y un Gobernador de campaña no ve su almacén desde el
// camino.
//
// Es un cambio grande respecto a la versión anterior, que mandaba COMPLETOS todos los asentamientos de la
// Facción propia. Y hace que la proyección ENCOJA: donde antes viajaban N interiores, ahora viaja uno más
// N-1 fichas. Va en la misma dirección que el trabajo de escala, no en contra.
//
// De las demás Facciones no se aporta ni un asentamiento sin verlo. Lo que SÍ viaja de todas es lo ya
// público en la ficción del juego (nombre, nivel, reputación, Rey/Embajador) — sin ello la pantalla de
// diplomacia no tendría con qué pintarse.
//
// **La excepción es lo que se VE**, y viaja siempre REDACTADO: los ejércitos ajenos (`ejercitosAvistados`,
// Doc 5.12.7) y los asentamientos ajenos (`asentamientosAvistados`, niebla de guerra Paso 1). De ambos se
// proyecta lo que se distingue desde fuera —quién es y dónde está— y nada de su interior. Es la misma
// ficción en los dos casos: una columna cruza campo abierto a la vista de quien vigile ese campo, y una
// ciudad no se puede esconder.
//
// Con la ciudad viaja también SU FRONTERA (decisión del usuario, 2026-09-05), y la silueta REAL, no una
// estimada: una frontera está marcada sobre el terreno y quien pasa por delante la ve. Lo que la acota es
// la misma niebla que acota todo lo demás — el cliente la dibuja bajo la máscara, así que del contorno solo
// se llega a ver el tramo que cae en tierra explorada.
//
// Lo que hay en el MUNDO —que no es de nadie— se filtra por la misma niebla, cada cosa con la regla que le
// toca: los campamentos de bandidos por lo que se ve AHORA (`campamentosAvistados`) y los caminos por lo
// EXPLORADO (`caminosConocidos`). Que no pertenezcan a ninguna Facción no los hace públicos: un campamento
// en un bosque que nadie ha pisado, o una calzada que une dos ciudades al otro lado del mundo, son
// información que el jugador no ha ido a buscar.
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
  CargosAsentamiento,
  Edificio,
  Ejercito,
  Faccion,
  OrdenMercado,
  Point,
  InteriorRecordado,
  PoliticaActiva,
  RelacionPolitica,
  Titulo,
  ZonaFaccion,
  ZonaInfluencia,
} from '../../domain/types';
import { VISION } from '../../constants';
import { distancia, pointInPolygon } from '../../world/geometria';
import type { EstadoMapa } from '../../world/mapa';
import type { TrazadoAsentamiento } from '../../engine/trazado';
import type { Instante } from '../../domain/tiempo';
import { esCiudadano } from '../../engine/faccion';
import { compartenVision } from '../../engine/pertenencia';
import { ubicacionDeducida } from '../../engine/ubicacion';
// El mismo recuento que usa el motor para los carros (Doc 5.13): un participante es un carro Y un rombo.
import { alcanceDeVista, enLaPuertaDe, participantesDe } from '../../engine/ejercitos';
import {
  estaExplorado,
  fundirExploraciones,
  marcarVisto,
  proyectarNiebla,
  rejillaDe,
  SIN_EXPLORAR,
  type NieblaProyectada,
  type Rejilla,
} from '../../engine/exploracion';
import { MEMORIA_VACIA, type FichaConocida } from '../../engine/memoria';
import type { ProduccionItem } from '../../engine/asentamientoQuery';
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
/**
 * Lo que se sabe de una plaza en la que NO se esta (Doc 1.10.4). Un solo tipo con secciones opcionales, y no
 * un tipo por nivel de acceso (a peticion del usuario, 2026-09-06): el cliente pinta lo que llega y no
 * pregunta por que falta, asi que un nivel nuevo no le rompe ningun camino.
 *
 * Lo que decide que secciones vienen es el SERVIDOR, siempre. Mandarlo todo y esconderlo en el cliente no lo
 * esconde: el cliente corre en la maquina del jugador y basta leer la respuesta a pelo. La proyeccion es la
 * frontera de seguridad, no una capa de presentacion.
 *
 * Tres niveles, de fuera hacia dentro:
 *
 *  1. **Publico** — lo que se ve desde el camino: donde esta, de quien es, y **lo que tiene en pie**.
 *  2. **De la Faccion** — quien manda y bajo que politicas. Publico dentro de casa, opaco fuera.
 *  3. **De quien la pisa** — el interior, y eso no viaja aqui: viaja completo en `asentamientos`.
 */
export interface AsentamientoAvistado {
  id: string;
  /** Opcional por el mismo motivo que en `Asentamiento`: ausente = el cliente muestra el `id`. */
  nombre?: string;
  faccionId: string;
  posicion: Point;
  nivel: number;
  /**
   * Su zona de influencia, con la silueta REAL — la misma que calcula el motor, recortada contra sus
   * vecinos (`computeZonaInfluencia`).
   *
   * **Fuga conocida y aceptada** (decisión del usuario, 2026-09-05): esa silueta está recortada contra
   * TODOS los vecinos de otra Facción, incluidos los que este jugador no ha visto nunca, y el corte se
   * reparte según el `radioPotencial` de cada uno. O sea que un lado plano en la frontera de una ciudad
   * delata que hay un tercero en esa dirección, y con cuánta fuerza relativa.
   *
   * Se acepta porque solo viaja la zona de una plaza que YA se está viendo —de quien no ves, no ves nada— y
   * porque la alternativa (recortar solo contra lo que el jugador conoce) enseñaría una frontera que no es
   * la de verdad. Entre "exacta con una pista de más" y "limpia pero falsa" se eligió la primera.
   */
  zona: Point[];
  /**
   * Los edificios EN PIE, y solo esos (decisión del usuario, 2026-09-06). Se ve lo que está levantado, no lo
   * que está en el papel: un muro a medio construir se distingue desde fuera, el plan de construirlo no.
   *
   * La línea cae sobre un campo que ya existía —`Edificio.estado`— así que no hay dos listas que mantener:
   * lo público es `activo`, y `en_cola`/`en_construccion` son la cola, que es privada.
   */
  edificios: Edificio[];
  /** Quién manda aquí. **Solo para ciudadanos de su Facción** (decisión del usuario, 2026-09-06): dentro de
   * casa es público, desde fuera no se sabe ni quién gobierna. Ausente = no eres de los suyos. */
  cargos?: CargosAsentamiento;
  /** Bajo qué políticas vive. Mismo nivel de acceso que `cargos`, y por el mismo motivo. */
  politicasActivas?: PoliticaActiva[];
  /**
   * Lo último que ESTE jugador vio de su interior, con la fecha en que lo vio (Doc 1.10.1). Ausente si nunca
   * la ha pisado — de una plaza en la que no has estado no recuerdas nada, la veas o no.
   *
   * Es información VIEJA a propósito, y por eso lleva `vistoEn`: el cliente escribe "hace 12 min" y quien la
   * lee sabe que está decidiendo con una foto, no con la realidad. Es la tensión que busca la mecánica — un
   * Gobernador de campaña manda sobre lo que recuerda.
   */
  interiorRecordado?: InteriorRecordado;
}

/**
 * Una caravana AJENA que se esta viendo (decision del usuario, 2026-09-06). Existe porque las interacciones
 * dejaron de ser automaticas: si el jugador tiene que hacer clic para interceptar, primero tiene que verla.
 * Antes de esto ninguna caravana ajena viajaba en la proyeccion, asi que no habia nada sobre lo que pulsar.
 *
 * **Se ve QUE lleva, no CUANTO.** Los nombres de los recursos y si va escoltada; ni una cifra. Es lo que se
 * distingue de lejos: una fila de carros con sacos y unos hombres armados al lado. Y es justo lo que hace
 * falta para decidir si vale la pena — sin convertir mirar en una auditoria del comercio rival.
 */
export interface CaravanaAvistada {
  id: string;
  posicionActual: Point;
  /** De quien es, por su plaza de origen. Ausente si esa plaza ya no existe: entonces no es de nadie. */
  faccionId?: string;
  /** Si marcha con un ejercito (Doc 5.13.2). Cambia la decision entera: una caravana escoltada no es una
   * presa, es un combate. */
  escoltada: boolean;
  /** QUE lleva, sin cuanto. Ordenado para que la lista no baile entre ticks por el orden del objeto. */
  recursos: string[];
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
  /**
   * El interior de la plaza donde el jugador ESTÁ FÍSICAMENTE, y solo esa (Doc 1.10.1). Cero o un elemento:
   * cero si está en el mapa o desconectado, uno si está dentro de una plaza de su propia Facción.
   *
   * Sigue siendo un array y no un objeto opcional para no romper a un cliente que itera, y porque la
   * mecánica de invitados —estar dentro de una plaza AJENA— tiene su propia capa pública pendiente de
   * definir: cuando exista, entrará por aquí sin cambiar la forma.
   */
  asentamientos: Asentamiento[];
  /**
   * Todo lo que se ve AHORA y no se está pisando, redactado a su ficha (ver `AsentamientoAvistado`) —
   * **incluidas las plazas de la propia Facción**, que es lo que cambió con el jugador situado: desde fuera,
   * la ciudad de uno se ve igual que cualquier otra.
   *
   * Array aparte y no mezclado con `asentamientos`, por el mismo motivo que `ejercitosAvistados`: la
   * diferencia entre "estoy dentro" y "lo veo de lejos" es de TIPO, no un campo opcional que el cliente
   * pueda olvidarse de mirar.
   */
  asentamientosAvistados: AsentamientoAvistado[];
  /** Las que se vieron ALGUNA VEZ y ahora no se ven: la última foto, con el instante en que se tomó (ver
   * `FichaConocida`). Nunca repite lo que ya está en `asentamientosAvistados` — cuando algo se ve y además se
   * recuerda, gana lo que se ve. Vacío para un jugador sin Facción: la memoria es de la Facción. */
  asentamientosConocidos: FichaConocida[];
  /** En qué territorio pisa cada ejército PROPIO: `ejercitoId` -> `faccionId` de quien manda en esa tierra,
   * o ausente si marcha por tierra de nadie. La propia Facción también cuenta, así que sirve igual para
   * "estás en casa" que para "te has metido en tierra de Troya".
   *
   * Lo resuelve el servidor y no el cliente aunque el cliente tenga los polígonos, por dos motivos: es un
   * HECHO del juego (de quién es el suelo que pisas), no una preferencia de dibujo; y el cliente solo tiene
   * las zonas de lo que ve, así que se equivocaría justo en el caso que importa — una capital de nivel 5
   * vigila 240 y una columna ve 150, o sea que se puede entrar en su tierra sin llegar a ver la ciudad. */
  territorioPorEjercito: Record<string, string>;
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
  /** Las PROPIAS, completas: las que salen o llegan a una plaza tuya. */
  caravanas: Caravana[];
  /** Las ajenas que se ven AHORA, redactadas (ver `CaravanaAvistada`). Fuera del radio de vision no existen
   * para el jugador — no hay lista de "caravanas del mundo" que consultar. */
  caravanasAvistadas: CaravanaAvistada[];
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
  /** Infraestructura del mundo (rutas comerciales trazadas). Solo los que la Facción ha PISADO: se filtran
   * como el terreno, por lo explorado, no por Facción como los `acuerdos`/`ordenes` — ver `caminosConocidos`. */
  caminos: CaminoComercial[];
  /** Entidades del MUNDO (bandidos). Solo los que se están VIENDO ahora mismo, sin memoria — ver
   * `campamentosAvistados`. */
  campamentosBandidos: CampamentoBandido[];
  /** Sin `asentamientoId` (eventos globales/de Facción) o con uno propio. Es el mismo criterio que evita la
   * fuga que el doc 7 §7.1 señalaba en el log administrativo: el log global narra TODO el mundo. */
  /* `eventosDominio` NO viaja aquí (follow-up de C13, cerrado el 2026-09-05) — ver la nota de cabecera. */
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
  /** Producción por minuto de mundo de cada edificio productor/transformador de la plaza que el jugador PISA
   * (`asentamientos[0]`) — `undefined` si está en el mundo. `produccionPorMinuto` (`engine/asentamientoQuery.ts`)
   * necesita la fachada `Mapa` y el polígono de zona (entrada privilegiada: bosques, stock de yacimientos),
   * así que el cliente de jugador —sin motor— no puede calcularlo. Lo rellena `RunnerDePartida` y lo fusiona
   * la ruta HTTP, igual que `preciosReferencia`. */
  produccionDeAsentamiento?: ProduccionItem[];
}

function faccionDe(estado: GameSessionState, jugadorId: string): string | null {
  return estado.facciones.find((f) => esCiudadano(f, jugadorId))?.id ?? null;
}

/**
 * **Los eventos NO viajan en las lecturas de estado** (follow-up de C13, cerrado el 2026-09-05).
 *
 * C13 añadió el cursor incremental —`GET /admin|jugador/partidas/:gameId/eventos?desde=<version>`— pero dejó
 * `eventosDominio` también dentro de `EstadoAdmin` y `ProyeccionJugador`, porque quitarlo habría roto al
 * único cliente que existía sin que hubiera ninguno migrado al cursor. Ya no es el caso.
 *
 * Lo que costaba: `eventosDominio` **solo crece** —`exito()` antepone, nada poda— y era el **87-88 % de una
 * lectura de estado** (medido: 1 017 eventos y 264 KB de 303 KB en el tick 200; 309 KB de 351 KB en el
 * 1 200). Se pagaba entero en cada lectura Y en cada respuesta de comando, para reenviar un historial que el
 * cliente ya tenía.
 *
 * Cómo se obtienen ahora, y por qué basta:
 *  1. Al conectar, una vez: `?desde=0` — el mismo payload que antes, pero una vez en vez de siempre.
 *  2. Después, lo nuevo: `?desde=<la mayor version vista>`.
 *  3. Y de los comandos propios ni eso hace falta: `ResultadoComando.eventos` ya trae los del comando con su
 *     `version` (C13), así que la respuesta que el cliente ya está leyendo le sirve de incremento.
 *
 * El filtro de propiedad —la parte con valor de SEGURIDAD, que un jugador no vea lo que le pasa a un rival—
 * no se ha tocado: vive en `eventosDominioParaJugador`, aquí abajo, con los mismos tests que antes cubrían la
 * proyección.
 */
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
 * que sus columnas alcanzan a ver mientras marchan — `VISION.ejercito` con tropa, `VISION.jugadorSolo` sin
 * ella (`alcanceDeVista`).
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
    ejercitosPropios.some((e) => distancia(punto, e.posicionActual) <= alcanceDeVista(e))
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
  ejercitosPropios: readonly Ejercito[],
  /** Ojos de aliados/señor/vasallo (`compartenVision`, niebla Paso 4). Solo cuentan para lo VISIBLE ahora,
   * nunca para lo explorado (`celdas`): la visión compartida es en vivo y no se graba — al romperse la
   * relación desaparece en la proyección siguiente. */
  asentamientosAliados: readonly Asentamiento[] = [],
  ejercitosAliados: readonly Ejercito[] = []
): NieblaProyectada {
  const rejilla = rejillaDe(estado.mapa.config);
  let visibles = SIN_EXPLORAR;
  for (const a of [...asentamientosPropios, ...asentamientosAliados]) visibles = marcarVisto(visibles, rejilla, a.posicion, a.radioPotencial + VISION.margenAsentamiento);
  for (const e of [...ejercitosPropios, ...ejercitosAliados]) visibles = marcarVisto(visibles, rejilla, e.posicionActual, alcanceDeVista(e));

  let celdas = grabada;
  for (const a of asentamientosPropios) celdas = marcarVisto(celdas, rejilla, a.posicion, a.radioPotencial + VISION.margenAsentamiento);
  for (const e of ejercitosPropios) celdas = marcarVisto(celdas, rejilla, e.posicionActual, alcanceDeVista(e));

  return proyectarNiebla(celdas, visibles, rejilla);
}

/**
 * Los caminos que la Facción CONOCE: los que ha pisado. Un camino es infraestructura estática, así que le
 * toca la misma regla que al terreno —"explorado", no "visible ahora"—: la calzada que recorriste sigue
 * donde estaba aunque hoy no la mires, igual que la colina que hay al lado.
 *
 * Se mide contra la máscara que YA viaja (`NieblaProyectada.celdas`) y no contra `memoria.exploracion` a
 * secas, para que el camino y el suelo que pisa no puedan discrepar: si la máscara destapa terreno por lo
 * que se ve ahora mismo —la ventana de un tick que `nieblaDe` cubre a propósito—, el camino que pase por
 * ahí viaja con él.
 *
 * **Entero o nada**, y a propósito. Recortar el trazado a los tramos explorados no daría "medio camino":
 * daría una polilínea con agujeros que el cliente uniría con rectas falsas, o trozos sin identidad (el
 * `id` y los dos extremos son del camino, no de cada tramo). Lo que se acepta a cambio es que haber andado
 * un tramo revele qué dos plazas une — que es, en la ficción, justo lo que una calzada dice de sí misma.
 */
function caminosConocidos(caminos: readonly CaminoComercial[], niebla: NieblaProyectada): CaminoComercial[] {
  // La rejilla sale de la propia máscara y no de `rejillaDe`: son la misma geometría por construcción, y
  // leerla de aquí quita de raíz la posibilidad de descifrar la máscara con una rejilla distinta.
  const rejilla: Rejilla = { columnas: niebla.columnas, filas: niebla.filas, tamanoCelda: niebla.tamanoCelda };
  return caminos.filter((c) => c.puntos.some((p) => estaExplorado(niebla.celdas, rejilla, p)));
}

/**
 * Los campamentos de bandidos que se ven AHORA — sin memoria, como los ejércitos avistados y a diferencia de
 * los asentamientos.
 *
 * No es una asimetría gratuita con los caminos: un campamento APARECE y DESAPARECE (`engine/bandidos.ts` los
 * genera, el combate los borra), y sobre algo que va y viene "explorado" mentiría en las dos direcciones.
 * Enseñaría el campamento que nació la semana pasada en un bosque que visitaste una vez —información que
 * nadie ha ido a buscar, que es exactamente la fuga— y seguiría enseñando el que otra Facción ya arrasó.
 *
 * Podría tener memoria, como una plaza: no se mueve, y "aquí había bandidos, hace tres horas" sería una
 * ficha razonable. Se descarta porque exigiría grabarlos en `memoriaPorFaccion` con su propio `conocidoEn` y
 * su propio tránsito del estado 3 al 2, y de eso no hay una sola regla escrita en el canon. Si algún día la
 * hay, el sitio es `engine/memoria.ts` y esto pasa a ser un `campamentosConocidos`.
 *
 * Consecuencia en el CLIENTE: al no tener memoria, un campamento se pinta DESPUÉS de la máscara de niebla,
 * junto a lo demás que se está viendo — taparlo sería taparle al jugador su propia información.
 */
function campamentosAvistados(
  campamentos: readonly CampamentoBandido[],
  asentamientosPropios: readonly Asentamiento[],
  ejercitosPropios: readonly Ejercito[]
): CampamentoBandido[] {
  return campamentos.filter((c) => seVeAhora(c.posicion, asentamientosPropios, ejercitosPropios));
}

/**
 * De quién es el suelo que pisa cada ejército propio (a petición del usuario, 2026-09-05: "debería poder
 * saber si estoy en el territorio de otro cuando voy caminando").
 *
 * Se resuelve contra las zonas de TODAS las Facciones, que es entrada privilegiada y por eso vive aquí y no
 * en el cliente: la respuesta tiene que ser correcta incluso cuando el jugador no ve la ciudad que manda en
 * esa tierra. Y ocurre — una capital de nivel 5 vigila 240 mientras una columna ve 150, así que hay una
 * franja en la que estás dentro de su territorio sin haberla divisado.
 *
 * Lo que sale de aquí es solo un `faccionId`, nunca qué asentamiento concreto: pisar la tierra de alguien te
 * dice de quién es, no dónde tiene la capital.
 */
function territorioDeCadaEjercito(
  ejercitosPropios: readonly Ejercito[],
  zonas: readonly ZonaInfluencia[],
  asentamientos: readonly Asentamiento[]
): Record<string, string> {
  if (ejercitosPropios.length === 0) return {};
  const faccionDeZona = new Map(asentamientos.map((a) => [a.id, a.faccionId]));

  const salida: Record<string, string> = {};
  for (const ejercito of ejercitosPropios) {
    const dentro = zonas.find((z) => pointInPolygon(ejercito.posicionActual, z.poligono));
    const duena = dentro ? faccionDeZona.get(dentro.asentamientoId) : undefined;
    if (duena !== undefined) salida[ejercito.id] = duena;
  }
  return salida;
}

/**
 * Las caravanas ajenas que se ven ahora mismo, redactadas (Doc 5.12.7 aplicado al comercio).
 *
 * Se excluyen las PROPIAS —ya viajan completas— y las `disponible`, que son flota aparcada dentro de una
 * plaza y no algo que cruce el campo. Una caravana en estado `adjunta` SI se ve, y marcada como escoltada:
 * es informacion que cambia la decision de quien la mira.
 */
function caravanasAvistadas(
  estado: GameSessionState,
  esPropio: (asentamientoId: string) => boolean,
  asentamientosPropios: readonly Asentamiento[],
  ejercitosPropios: readonly Ejercito[]
): CaravanaAvistada[] {
  const faccionDePlaza = new Map(estado.asentamientos.map((a) => [a.id, a.faccionId]));
  const adjuntas = new Set(estado.ejercitos.flatMap((e) => e.caravanasAdjuntasIds));

  return estado.caravanas
    .filter((c) => !esPropio(c.origenAsentamientoId) && !(c.destinoAsentamientoId !== undefined && esPropio(c.destinoAsentamientoId)))
    .filter((c) => c.estado !== 'disponible')
    .filter((c) => seVeAhora(c.posicionActual, asentamientosPropios, ejercitosPropios))
    .map((c) => ({
      id: c.id,
      posicionActual: c.posicionActual,
      ...(faccionDePlaza.has(c.origenAsentamientoId) ? { faccionId: faccionDePlaza.get(c.origenAsentamientoId)! } : {}),
      escoltada: adjuntas.has(c.id),
      recursos: Object.keys(c.contenido)
        .filter((r) => (c.contenido[r] ?? 0) > 0)
        .sort(),
    }));
}

export function proyectarParaJugador(
  estado: GameSessionState,
  jugadorId: string,
  geometria: GeometriaAsentamientos
): Omit<ProyeccionJugador, 'preciosReferencia' | 'produccionDeAsentamiento'> {
  const { faccionId, asentamientosPropios, esPropio } = propioDeJugador(estado, jugadorId);

  // La plaza que el jugador PISA, que es la única cuyo interior viaja (Doc 1.10.1). Se exige además que sea
  // de su Facción: dentro de una plaza ajena solo se ve la capa pública (Doc 1.10.4), y qué lleva esa capa
  // es una decisión que el diseño todavía no ha tomado — hasta que la tome, no enseñamos de más.
  const jugador = estado.jugadores.find((j) => j.id === jugadorId);
  const recordadas = jugador?.plazasRecordadas ?? {};
  // Sin registro se DEDUCE, con la misma función que usa el alta: proyectar es LEER, y una lectura no puede
  // escribir en el estado para darse de alta a sí misma. Y pasa de verdad — un jugador que entra en la
  // partida y pide su pantalla antes de dar ninguna orden todavía no tiene registro; sin esto vería un mundo
  // vacío desde dentro de su propia ciudad.
  const ubicacion = jugador?.ubicacion ?? ubicacionDeducida(jugadorId, estado.asentamientos, estado.ejercitos);
  const dentroDe =
    ubicacion.tipo === 'asentamiento'
      ? asentamientosPropios.find((a) => a.id === ubicacion.asentamientoId)
      : undefined;

  // Un ejército es "propio" si es de tu Facción, si VAS DENTRO, o si llevas tropa tuya en él.
  //
  // La de en medio es la que hace posible existir sin bandera, y faltaba: un recién llegado no tiene Facción
  // NI escuadrones, así que con las otras dos no cumplía ninguna — `seVeAhora` le salía siempre falso y
  // caminaba por un mapa negro sin ver ni su propia columna, porque las propias son las que viajan en
  // `ejercitos`. Leerlo de `participantes` es lo correcto por lo mismo que en el motor: un viajero sin tropas
  // también va dentro de la suya (Doc 5.12.1).
  //
  // La tercera tampoco es redundante: un jugador huérfano (Doc 5.4) se queda sin Facción pero no sin los
  // escuadrones que iban con él.
  const ejercitosPropios = estado.ejercitos.filter(
    (e) =>
      (faccionId !== null && e.faccionId === faccionId) ||
      e.participantes.some((p) => p.jugadorId === jugadorId) ||
      e.escuadrones.some((esc) => esc.jugadorId === jugadorId)
  );
  // Las plazas en cuya PUERTA hay una columna de este jugador. Un mercado enseña sus ofertas a quien esta
  // dentro, y solo a ese: sin esto el mostrador (`comerciarEnPlaza`) seria inusable —habria que comprar a
  // ciegas—, y con una lista global cualquiera podria leer los precios del mundo entero sin moverse.
  const enElMostradorDe = new Set(
    estado.asentamientos.filter((a) => ejercitosPropios.some((e) => enLaPuertaDe(e, a))).map((a) => a.id)
  );
  const zonasPropias = geometria.zonas.filter((z) => esPropio(z.asentamientoId));
  const propios = new Set(ejercitosPropios.map((e) => e.id));

  // Niebla Paso 4: un aliado (o señor/vasallo) ve lo que ves tú, EN VIVO. Sus plazas y columnas se suman a la
  // capa "viéndolo ahora" —`seVeAhora`, avistados, la máscara `visibles`—, nunca a la memoria ni a lo
  // explorado: al romperse la relación, `compartenVision` deja de incluir esa Facción y en la proyección
  // siguiente lo que solo veías por ella desaparece (no queda "último conocido"). Ver `Niebla_De_Guerra_Definicion.md` §5.6.
  const faccionesQueComparten =
    faccionId !== null
      ? new Set(
          estado.facciones
            .filter((f) => f.id !== faccionId && compartenVision(estado.relaciones, faccionId, f.id))
            .map((f) => f.id)
        )
      : new Set<string>();
  const asentamientosAliados = estado.asentamientos.filter((a) => faccionesQueComparten.has(a.faccionId));
  const ejercitosAliados = estado.ejercitos.filter((e) => faccionesQueComparten.has(e.faccionId));
  // Los ojos que cuentan para "ver ahora" = propios + aliados. Se reutiliza el mismo array cuando no hay
  // aliados para no reasignar nada en el caso normal.
  const ojosAsent = asentamientosAliados.length > 0 ? [...asentamientosPropios, ...asentamientosAliados] : asentamientosPropios;
  const ojosEjercito = ejercitosAliados.length > 0 ? [...ejercitosPropios, ...ejercitosAliados] : ejercitosPropios;

  // La zona sale de `geometria`, que el runner ya calculó y cachea para TODOS los asentamientos: adjuntarla
  // aquí no cuesta un cálculo más. Una plaza sin zona en la geometría (no debería pasar) viaja con el
  // contorno vacío en vez de romper la proyección entera.
  const poligonoDe = new Map(geometria.zonas.map((z) => [z.asentamientoId, z.poligono]));
  // Lo que se ve pero no se pisa. Los propios entran aquí igual que los ajenos: se excluye SOLO la plaza en
  // la que el jugador está, que es la única que viaja entera.
  const avistados = estado.asentamientos
    .filter((a) => a.id !== dentroDe?.id && seVeAhora(a.posicion, ojosAsent, ojosEjercito))
    .map((a) => ({
      id: a.id,
      nombre: a.nombre,
      faccionId: a.faccionId,
      posicion: a.posicion,
      nivel: a.nivel,
      zona: poligonoDe.get(a.id) ?? [],
      edificios: a.edificios.filter((e) => e.estado === 'activo'),
      ...(esPropio(a.id) ? { cargos: a.cargos, politicasActivas: a.politicasActivas } : {}),
      ...(recordadas[a.id] ? { interiorRecordado: recordadas[a.id] } : {}),
    }));
  const seVe = new Set(avistados.map((a) => a.id));

  const memoria = (faccionId !== null ? estado.memoriaPorFaccion[faccionId] : undefined) ?? MEMORIA_VACIA;
  // El jugador ve SIEMPRE por donde ha andado él, tenga bandera o no. El tick graba ese rastro en
  // `Jugador.exploracionPersonal` mientras su columna sea huérfana (`grabarExploracionPersonal`), y hay que
  // fundirlo con la memoria de la Facción: un ciudadano cuya columna de aparición nunca se marcó con su
  // bandera —`unirseAFaccion` funde lo andado ANTES de unirse, pero lo de después queda solo aquí— caminaría
  // si no por un mapa que se cierra de nuevo tras cada paso (niebla de guerra, los tres niveles).
  const exploradoDelJugador = fundirExploraciones(memoria.exploracion, jugador?.exploracionPersonal ?? SIN_EXPLORAR);
  // La niebla se calcula ANTES del objeto porque además de viajar es el filtro de los caminos: la misma
  // máscara que tapa el terreno decide qué calzadas existen para este jugador.
  const exploracion = nieblaDe(exploradoDelJugador, estado, asentamientosPropios, ejercitosPropios, asentamientosAliados, ejercitosAliados);

  return {
    gameId: estado.gameId,
    instante: instanteDeTick(estado.tick),
    version: estado.version,
    jugadorId,
    faccionId,
    mapaId: idDeMapa(estado.mapa),
    estadoMapa: estado.estadoMapa,
    facciones: estado.facciones,
    asentamientos: dentroDe ? [dentroDe] : [],
    asentamientosAvistados: avistados,
    // Lo recordado MENOS lo que se ve ahora. Cada plaza aparece en una lista o en la otra, nunca en las dos.
    // Las propias ya no se descuentan aquí: desde que solo viaja entera la que se pisa, una plaza de tu
    // Facción que no estés viendo es exactamente igual de recordada que cualquier otra.
    asentamientosConocidos: Object.values(memoria.asentamientos).filter((f) => !seVe.has(f.asentamientoId) && f.asentamientoId !== dentroDe?.id),
    territorioPorEjercito: territorioDeCadaEjercito(ejercitosPropios, geometria.zonas, estado.asentamientos),
    exploracion,
    caravanas: estado.caravanas.filter((c) => esPropio(c.origenAsentamientoId) || (c.destinoAsentamientoId !== undefined && esPropio(c.destinoAsentamientoId))),
    caravanasAvistadas: caravanasAvistadas(estado, esPropio, ojosAsent, ojosEjercito),
    ejercitos: ejercitosPropios,
    ejercitosAvistados: estado.ejercitos
      .filter((e) => !propios.has(e.id) && seVeAhora(e.posicionActual, ojosAsent, ojosEjercito))
      .map((e) => ({ id: e.id, faccionId: e.faccionId, posicionActual: e.posicionActual, participantes: participantesDe(e) })),
    acuerdos: estado.acuerdos.filter((a) => esPropio(a.asentamientoAId) || esPropio(a.asentamientoBId)),
    // De las propias, todas —incluidas las cumplidas, que son el historial de tu mercado—. De una plaza ajena
    // en cuya puerta estas, solo las que siguen EN PIE: es el escaparate, no su contabilidad.
    ordenes: estado.ordenes.filter(
      (o) => esPropio(o.asentamientoId) || (o.estado === 'activa' && enElMostradorDe.has(o.asentamientoId))
    ),
    relaciones: estado.relaciones,
    titulos: estado.titulos,
    caminos: caminosConocidos(estado.caminos, exploracion),
    campamentosBandidos: campamentosAvistados(estado.campamentosBandidos, ojosAsent, ojosEjercito),
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
