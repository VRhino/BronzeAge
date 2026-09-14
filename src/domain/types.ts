// Entidades núcleo — Sprint 1 (mundo/fundación) + Sprint 2 (población/construcción/almacén)
// + Sprint 3 (economía) + Sprint 4 (estructura política) + Sprint 5 (guerra) + Sprint 6 (cierre).
// Ver Consideraciones/Plan_Implementacion_Tecnica.md para el modelo de datos completo.
// Murallas/torres/puerto (edificios estratégicos de colocación manual, Doc 4.2) quedan fuera de Fase 0.

import type { Instante } from './tiempo';

export interface Point {
  x: number;
  y: number;
}

export type RecursoTipo =
  | 'madera'
  | 'piedra'
  | 'trigo'
  | 'cobre'
  | 'estano'
  | 'oro'
  | 'livestock'
  // --- Rediseño de progreso (Fase 0): cadenas de crafting de los edificios de transformación (Doc 4.2.1) ---
  | 'lingoteCobre'
  | 'lingoteEstano'
  | 'lingoteBronce'
  | 'cuero'
  | 'cueroCurtido'
  | 'cueroCalidad'
  // Escalón de entrada militar (ver TROPAS_RECLUTABLES en constants.ts): arma simple de madera endurecida,
  // fabricada en Armería nivel 1 sin depender de la cadena metalúrgica — es lo que permite que un
  // asentamiento recién fundado pueda armar tropa antes de encontrar cobre.
  | 'armaMadera'
  | 'armaCobre'
  | 'armaBronce'
  | 'armaBronceCalidad'
  | 'armaduraBasica'
  | 'armaduraIntermedia'
  | 'armaduraBronce';

/**
 * Los valores de `RecursoTipo`, en runtime — para validar por HTTP que un `recurso` recibido de un cliente es
 * uno de verdad, sin duplicar la lista a mano (Fase C9, doc 4: "params malformado revienta dentro del
 * manejador y sale como 409 en vez de 400"). Ninguna tabla de `constants.ts` cubre los 20 (`PRECIO_BASE`, por
 * ejemplo, solo tiene los 6 comprables — los fabricados no tienen precio base), así que esto NO se deriva de
 * ninguna — se deriva del propio TIPO: `Record<RecursoTipo, true>` obliga en COMPILACIÓN a listar cada
 * miembro exactamente una vez; olvidar uno o escribir uno que no existe es un error de `tsc`, no un desajuste
 * silencioso que alguien tenga que notar a mano.
 */
const TODOS_LOS_RECURSOS: Record<RecursoTipo, true> = {
  madera: true,
  piedra: true,
  trigo: true,
  cobre: true,
  estano: true,
  oro: true,
  livestock: true,
  lingoteCobre: true,
  lingoteEstano: true,
  lingoteBronce: true,
  cuero: true,
  cueroCurtido: true,
  cueroCalidad: true,
  armaMadera: true,
  armaCobre: true,
  armaBronce: true,
  armaBronceCalidad: true,
  armaduraBasica: true,
  armaduraIntermedia: true,
  armaduraBronce: true,
};
export const RECURSOS_TIPO = Object.keys(TODOS_LOS_RECURSOS) as RecursoTipo[];

export type Rareza = 'comun' | 'intermedio' | 'raro';

/** Cargos de nivel Facción (Doc 2.2): Rey (vasallaje, políticas superiores) y Embajador (designado por el Rey). */
export interface Faccion {
  id: string;
  nombre: string;
  reyId: string | null;
  embajadorId: string | null;
  /** Derivado de `experiencia` vía `calcularNivelFaccion` (Doc 1.7, rediseño Fase 0.5) — nunca se guarda
   * "suelto", se recalcula cada tick a partir de la XP. Sube el cupo de asentamientos por nivel (Doc
   * Fase_0_5_Definicion_Especializacion_y_Cupos.md §5) y el cap de fundación (`CAP_FUNDACION_POR_NIVEL`). */
  nivel: number;
  /** Experiencia acumulada MONÓTONA (nunca baja, Doc Fase_0_5 §8): combate, construcción, conquista, defensa/
   * ataque de caravana (`engine/faccion.ts` `aplicarAjustesExperiencia`, `NIVEL_FACCION.xp`). Reemplaza la
   * fórmula anterior basada en población total — esa dejaba que fundar asentamientos de nivel 1 regalara
   * cupos de nivel alto, en vez de exigir actividad real de la Facción. */
  experiencia: number;
  /** Ciudadanía (Doc 2.5): jugadores con ciudadanía en ESTA Facción (no se extiende a toda la Liga). */
  ciudadanosIds: string[];
  /** Score de confiabilidad PÚBLICO -100..+100 (Doc 2.7), decae hacia 0 sin eventos nuevos. */
  reputacion: number;
}

/** 3 clases de población NPC — Doc 4.1. Los Jugadores son una categoría separada. */
export interface Poblacion {
  pesants: number;
  artesanos: number;
  nobleza: number;
}

export interface RecursoAlmacenado {
  cantidad: number;
  capacidad: number;
}

export type EdificioTipo =
  | 'centroUrbano'
  | 'vivienda'
  | 'granja'
  | 'cantera'
  | 'lenera'
  | 'almacen'
  // Almacén especializado en grano (a petición del usuario, 2026-09-04): solo trigo, mucha más capacidad,
  // uno por asentamiento y con 4 niveles internos. Ver `EDIFICIO_CATALOGO.granero`.
  | 'granero'
  | 'mina'
  | 'minaCobre'
  | 'minaEstano'
  | 'fundicion'
  | 'granFundicion'
  // --- Rediseño de progreso (Fase 0, Doc 4.2.1): reemplaza a 'taller' ---
  | 'corral'
  | 'armeria'
  | 'curtiduria'
  | 'carpinteria'
  | 'palacio'
  | 'barracon'
  | 'galeriaDeTiro'
  // Ampliación de comercio (a petición del usuario): gatea las órdenes de Mercado (Doc 3.3) y aloja el cupo
  // de la flota de caravanas propias (`cupoCaravanas`, engine/asentamientoQuery.ts). Vía política del
  // Tesorero, mismo patrón que Barracón/Galería de tiro/Palacio — no auto-construcción.
  | 'mercado'
  // Pieza satélite de la ZONA de Mercado (a petición del usuario, ver
  // Consideraciones/Vista_Asentamiento_Trazado_Urbano.md): el Mercado no es un edificio suelto sino una zona
  // que se puebla de puestos al subir de nivel interno. Deliberadamente un tipo APARTE de 'mercado': así el
  // comercio (cupoCaravanas, tieneMercadoActivo, EDIFICIOS_UNICOS) sigue viendo exactamente UNA instancia de
  // Mercado y no hay que tocarlo. Un puesto no se construye, no progresa y no gatea nada — solo ocupa suelo.
  | 'puestoMercado'
  // Maravilla (Roadmap_Escalado.md Eje 4, a petición del usuario): edificio único de coste extremo, solo el
  // EDIFICIO en sí — el ciclo de servidor de 12 meses que cierra al completarla queda fuera de esta pasada
  // (requiere infraestructura de servidor/multi-instancia que Fase 0 no tiene, ver Roadmap_Escalado.md).
  | 'maravilla'
  // Anclas y satélites, Etapa 3 (Consideraciones/Vista_Asentamiento_Trazado_Urbano.md §5): "marcadores
  // gratis" — mismo patrón que 'puestoMercado' (costo {}, nacen ya activos, nunca pasan por cola, no se
  // pueden añadir a mano). 'plaza' es el ancla de saturación del núcleo residencial (nunca ancla primaria:
  // Centro Urbano ya cumple ese papel desde la fundación); 'plazaDeArmas'/'patioDeGremios' son ancla
  // primaria Y de saturación de sus categorías (militar/industria no tienen ancla de fundación). Nacen por
  // la regla de semilla de grupo (§5.4/5.5), no por construcción normal.
  | 'plaza'
  | 'plazaDeArmas'
  | 'patioDeGremios'
  // Variedad de anclas residenciales (Etapa 4, punto 4, a petición del usuario): mismo patrón "marcador
  // gratis" que 'plaza' — cuando el núcleo residencial satura, se sortea (determinista) entre las tres.
  | 'pozo'
  | 'parque';

/** Los valores de `EdificioTipo` en runtime, mismo motivo y mismo mecanismo de exhaustividad que
 * `RECURSOS_TIPO` — ver su comentario. Un `tipo` inválido llegado por HTTP hoy revienta más adentro
 * (`EDIFICIO_CATALOGO[tipo].costo` en `engine/construction.ts` no comprueba existencia): esto lo convierte en
 * un 400 en el borde, antes de tocar el motor. */
const TODOS_LOS_EDIFICIOS: Record<EdificioTipo, true> = {
  centroUrbano: true,
  vivienda: true,
  granja: true,
  cantera: true,
  lenera: true,
  almacen: true,
  granero: true,
  mina: true,
  minaCobre: true,
  minaEstano: true,
  fundicion: true,
  granFundicion: true,
  corral: true,
  armeria: true,
  curtiduria: true,
  carpinteria: true,
  palacio: true,
  barracon: true,
  galeriaDeTiro: true,
  mercado: true,
  puestoMercado: true,
  maravilla: true,
  plaza: true,
  plazaDeArmas: true,
  patioDeGremios: true,
  pozo: true,
  parque: true,
};
export const EDIFICIOS_TIPO = Object.keys(TODOS_LOS_EDIFICIOS) as EdificioTipo[];

export type EstadoEdificio = 'en_cola' | 'en_construccion' | 'activo';

/**
 * Una celda del anillo de un recinto amurallado (`Consideraciones/Murallas_Definicion.md`).
 *
 * `clase` se decide AL TRAZAR y no cambia nunca: una celda que nació puerta muere puerta (§5.1 — las puertas
 * se congelan con el trazo, así que un recinto ya levantado no gana puertas jamás y ningún camino posterior lo
 * atraviesa). Lo que el nivel del recinto cambia es lo que esa puerta VALE, no que exista.
 */
export interface CeldaMuro {
  col: number;
  row: number;
  clase: 'muro' | 'puerta' | 'torre';
}

/**
 * Un recinto amurallado: un ANILLO CERRADO DE CELDAS alrededor del casco urbano. No es un edificio —no tiene
 * huella rectangular, se paga por celda y se levanta celda a celda—, por eso es una entidad propia y no un
 * `EdificioTipo` (el `muralla` de 1 celda que existía antes desaparece, §13 del doc).
 *
 * Es la primera FRONTERA persistente del motor: divide el asentamiento en intramuros y arrabal.
 */
export interface Recinto {
  id: string;
  /** 1 = empalizada · 2 = muro de piedra · 3 = muralla con adarve (§7 del doc). */
  nivel: number;
  /** El anillo completo, EN ORDEN DE RECORRIDO desde la puerta principal — que es el orden en que se levanta.
   * Congelado al comprometerse: ni una mejora de nivel ni el crecimiento de la ciudad lo modifican. */
  celdas: CeldaMuro[];
  /** Índice de la última celda YA LEVANTADA del recorrido. -1 = trazo comprometido pero sin una sola celda en
   * pie. Las celdas todavía no levantadas **ocupan suelo igual** desde el commit: si no, un edificio se
   * plantaría encima del trazo a medio hacer y el anillo dejaría de poder cerrarse. */
  avance: number;
  /** Nivel al que se está mejorando ahora mismo, si hay una mejora en curso (§7 del doc). Mientras esté
   * presente, `avance` mide el progreso de la MEJORA (reiniciado a -1 al empezarla), no el de la construcción
   * original — solo se puede mejorar un recinto ya completo. Se borra al terminar, cuando `nivel` sube. */
  mejorandoA?: number;
  comprometidoEn: Instante;
  completadoEn?: Instante;
}

export interface Edificio {
  id: string;
  tipo: EdificioTipo;
  /**
   * Posición del edificio EN SU PROPIO ESPACIO (ver `ambito`):
   * - `ambito: 'asentamiento'` (por defecto): coordenada LOCAL del espacio plano del asentamiento (Vista de
   *   Asentamiento, a petición del usuario) — origen `(0,0)` en el Centro Urbano, radio del disco = el
   *   `radioPotencial` del asentamiento. NO son coordenadas del mapa general y NO se dibujan en él.
   * - `ambito: 'mapa'`: coordenada del MAPA GENERAL, como siempre — solo los extractores minerales
   *   (mina/minaCobre/minaEstano/cantera), que se plantan sobre su nodo del mapa (`ambitoDe`, engine/construction.ts).
   */
  posicion: Point;
  estado: EstadoEdificio;
  /**
   * Fase D / doc 10 — instante de MUNDO en el que la obra pasa a `activo` (FECHA ABSOLUTA, no un contador
   * descendente: doc 6 §4, regla (a) "fechas, nunca contadores"). Presente SOLO mientras
   * `estado === 'en_construccion'`; ausente en `en_cola` (la obra aún no arrancó y su duración se fija al
   * empezar, ver Vía Rápida en `avanzarConstruccion`) y en `activo` (ya terminó).
   */
  completaEn?: Instante;
  /**
   * Espacio lógico en el que vive el edificio (Vista de Asentamiento). Ausente = `'asentamiento'` (el caso
   * común: casi todo edificio se construye DENTRO del espacio plano del asentamiento). Solo los extractores
   * minerales llevan `'mapa'`, porque se construyen sobre su yacimiento en el mapa general. Granja, Leñera y
   * Corral son `'asentamiento'` aunque su producción/elegibilidad dependa de rasgos del mapa dentro de la
   * zona de influencia (bosque/fertilidad/livestock) — ver `ambitoDe` y `evaluarNecesidades`, engine/construction.ts.
   */
  ambito?: 'asentamiento' | 'mapa';
  /** Nodo de recurso o zona de bosque que explota (cantera/lenera/corral), si aplica. */
  fuenteId?: string;
  /** Nivel interno de mejora (Doc 4.2.1): solo edificios de transformación con tiers (Fundición, Curtiduría,
   * Armería, Carpintería, Barracón, Galería de tiro). Ausente/1 para el resto. */
  nivelInterno?: number;
  /** Orientación intercambiable ancho↔alto (Etapa 4, a petición del usuario: variedad de silueta entre
   * ciudades, ej. un Mercado 3x2 puede nacer como 3x2 o 2x3). Decidida una vez al colocarse
   * (`sitiosParaTipo`/`crearAnclaNueva`, engine/trazado.ts) y fija después — solo tiene efecto si el tipo
   * tiene footprint no cuadrado; `granja` (progresión real de tamaño) y `puestoMercado` (`nivelInterno` ya
   * codifica una forma concreta de zona) nunca lo usan. Ausente/false = orientación normal. */
  rotado?: boolean;
  /** Etapa 5, exclusivo de anclas reales (`ANCLAS_REALES`, engine/trazado.ts): esta instancia ya agotó sus 8
   * direcciones de crecimiento (sin hueco real en ninguna) y quedó descartada PARA SIEMPRE como semilla de
   * nuevas anclas (`semillaActiva`/`crearAnclaNueva`) — se marca una sola vez y nunca se revisa. No tiene
   * relación con la saturación del núcleo de satélites de un ancla (`anclaLlena`, Lógica 2, sin cambios): un
   * ancla puede estar `semillaSaturada` y seguir teniendo hueco de sobra para sus propios satélites, o estar
   * `anclaLlena` y seguir siendo la semilla activa del árbol mientras sus 5 ranuras tengan sitio — CRITERIOS
   * INDEPENDIENTES, ninguno dispara al otro (confirmado con el usuario tras una primera corrección que sí los
   * mezclaba). */
  semillaSaturada?: boolean;
  /** Lógica 2 (satélites, `sitiosPorAtraccionDura`/`anclaActivaParaCategoria`, engine/trazado.ts): esta
   * instancia ya no tiene hueco para el próximo edificio dependiente de su categoría — se marca una sola vez y
   * nunca se revisa (nada libera celdas). Es un concepto DISTINTO de `semillaSaturada` (Lógica 1: agotamiento
   * de las 5 ranuras de crecimiento del árbol, radio mucho mayor) y no la afecta — `anclaLlena` solo cambia a
   * qué instancia se atrae el PRÓXIMO satélite de esa categoría (`anclaActivaParaCategoria`), nunca decide
   * dónde nace la siguiente ancla. Antes de esto, la búsqueda de a qué instancia atraerse siempre volvía a la
   * más cercana al origen sin memoria de si tenía hueco — con Centro Urbano (`posicion` fija en el origen)
   * eso significaba que, una vez lleno, ninguna otra instancia de su categoría se volvía a usar jamás y el
   * asentamiento fabricaba anclas nuevas sin parar en su lugar (bug detectado con el laboratorio visual). */
  anclaLlena?: boolean;
  /** Overhaul de auto-construcción: score de necesidad (ver `SCORE_BANDAS`, constants.ts) capturado en el
   * momento en que el proyecto se comprometió (pagó) y entró `en_cola` — determina el orden real en que
   * arranca la construcción cuando compite por un hueco de `maximoEnConstruccionSimultanea`. Ausente para
   * edificios que nunca pasan por la cola (ej. `centroUrbano`). */
  prioridad?: number;
  /** Recalculado cada tick en `avanzarConstruccion` (Doc 4.2, instrumentación de excedente): true si este
   * edificio produjo algo este tick que no cupo en el almacén (capacidad llena) — la producción sobrante se
   * pierde en vez de acumularse (ver `agregarRecursoConSobrante`, engine/almacen.ts). Solo informativo/UI,
   * no bloquea nada por sí mismo. Ausente/false si no aplica o produjo sin tope. */
  pausadoPorAlmacenLleno?: boolean;
  /** Ocupación post-conquista (Doc 5.4, `Consideraciones/Ocupacion_Post_Conquista_Definicion.md` §2.2): el
   * saqueo baja un edificio `activo` a `estado: 'en_cola'` marcándolo `danado`. Al comprometer la obra,
   * `avanzarConstruccion` cobra solo `OCUPACION.fraccionCosteReconstruccion` del costo y tarda esa fracción
   * de tiempo — un dañado NO se reconstruye desde cero, se repara. Se borra al volver a `activo`. Ausente =
   * edificio sano (caso normal). */
  danado?: boolean;
}

/** Cargos de nivel asentamiento (Doc 2.2), uno de cada, designados por el Gobernador salvo él mismo. */
export interface CargosAsentamiento {
  gobernadorId: string | null;
  tesoreroId: string | null;
  generalId: string | null;
  maestroObrasId: string | null;
  sacerdoteId: string | null;
}

/**
 * Dónde está un Héroe (Doc 1.10). Son los TRES únicos sitios donde puede estar, y la unión cerrada es lo
 * que impide el cuarto estado que el motor tenía de facto: en ninguna parte, viendo el mundo entero.
 *
 * `desconectado` guarda un punto y no una columna a propósito: al salir del mundo la columna de un viajero
 * solo deja de existir como entidad —no consume, no ve, no la ven— y al volver se reconstruye ahí mismo.
 */
export type UbicacionHeroe =
  | { tipo: 'asentamiento'; asentamientoId: string }
  | { tipo: 'columna'; ejercitoId: string }
  | { tipo: 'desconectado'; punto: Point };

/**
 * El Héroe (Doc 5.16, `Docs/Coordinacion/01_Modelo_de_datos_compartido.md` §12): el personaje con el que se
 * juega, uno por jugador y partida. Es el dueño de todo lo que antes se atribuía al jugador (escuadrones,
 * cargos, residencia, ciudadanía). La identidad (usuario, sesión, membresía) vive en `acceso/`: aquí solo el
 * vínculo `jugadorId`.
 */
export interface Heroe {
  id: string;
  /** `Membresia.jugadorId` del jugador que lo maneja, único por partida. `null` en los héroes bot. */
  jugadorId: string | null;
  controlador: 'humano' | 'bot';
  displayName: string;
  /** Catálogo de clases de Conquest. BronzeAge lo guarda y lo sirve, no lo interpreta. */
  classDefinitionId: string;
  genero: 'masculino' | 'femenino';
  /** Piezas del catálogo visual de Conquest. Cosmético. */
  avatar: { cabezaId: string; peloId: string; barbaId: string; cejasId: string };
  /** Liderazgo BASE (Doc 5.11). El efectivo es base + progresión, pero la progresión todavía no está
   * diseñada (`Docs/Mecanicas a desarrollar.md` §11), así que hoy coinciden. Un id sin registro en
   * `GameSessionState.heroes` (los fundadores de los escenarios de batch) usa `LIDERAZGO.base`. */
  liderazgoBase: number;
  /**
   * Dónde está (Doc 1.10). Es lo que convierte al jugador en una entidad SITUADA: solo ve el interior del
   * asentamiento en el que está, y solo puede dar órdenes ahí.
   *
   * Es el primer campo de este registro sin valor por defecto posible — "ausente = usa la base" funcionaba
   * para el Liderazgo, pero "ausente = está en ninguna parte" no significa nada. Por eso el registro deja de
   * ser opcional y las partidas guardadas sí necesitan migración.
   */
  ubicacion: UbicacionHeroe;
  /**
   * Lo ultimo que vio del interior de cada plaza que ha pisado (Doc 1.10.1), por `asentamientoId`.
   *
   * **Es lo unico de todo el modelo que NO se puede derivar.** La proyeccion es una vista: sabe filtrar el
   * presente, no recordar el pasado. "Trigo 4.200 hace doce minutos" no esta en ningun sitio del estado vivo
   * —ahi pone 3.100— asi que o se anota al salir, o no existe.
   *
   * **Del JUGADOR y no de la Faccion**, que es la diferencia con `memoriaPorFaccion`. Si fuera de la Faccion,
   * a un grupo le bastaria dejar a uno sentado en casa refrescandola para que todos vieran el almacen en vivo
   * desde cualquier parte del mapa, y la mecanica entera se cae: la ciudadania habilita, la PRESENCIA ejerce
   * (Doc 2.5).
   *
   * Se escribe al SALIR y no cada tick: mientras estas dentro ves lo vivo, asi que refrescar la foto no
   * cambiaria nada de lo que ves y costaria trabajo en cada latido.
   */
  plazasRecordadas?: Record<string, InteriorRecordado>;
  /**
   * Lo que ha explorado ANTES de tener bandera (Doc 1.3).
   *
   * La memoria del mundo es de la Faccion (`memoriaPorFaccion`), asi que sin esto el primer tramo de partida
   * —el que va desde que apareces hasta que fundas o te unes— seria un paseo a ciegas SIN REGISTRO: cada vez
   * que miraras el mapa estaria igual de negro que al empezar.
   *
   * Se FUNDE con la de la Faccion al fundar o al entrar en una, y desaparece: a partir de ahi manda la de la
   * Faccion. Lo que anduviste solo pasa a ser conocimiento de los tuyos, que es lo que un recien llegado
   * aporta de verdad.
   */
  exploracionPersonal?: Exploracion;
}

/**
 * La foto MINIMA del interior de una plaza (decision del usuario, 2026-09-06): las tres cifras que se miran
 * al volver a casa, y nada mas.
 *
 * Se descarto guardar el `Asentamiento` entero: seria fiel a "lo ultimo que viste" pero duplicaria en el
 * snapshot lo que ya esta vivo, y engordaria justo la mecanica que nació para adelgazar la proyeccion.
 */
export interface InteriorRecordado {
  /** Cuando se tomo. Es lo que convierte el dato en "hace doce minutos" en vez de en una mentira. */
  vistoEn: Instante;
  almacen: Record<string, RecursoAlmacenado>;
  /** Lo que estaba levantandose o esperando turno: `en_cola` y `en_construccion` (lo `activo` es publico y
   * viaja vivo en la ficha, no hace falta recordarlo). */
  cola: Edificio[];
  /** Quien defendia la plaza (Doc 5.12.4). El dato mas tactico del juego, y por eso solo se recuerda de
   * donde has estado. */
  guarnicion: Escuadron[];
}

/**
 * Celdas exploradas, un bit por celda, en hexadecimal (niebla de guerra). Cadena vacia = nada explorado.
 *
 * Vive aqui y no en `engine/exploracion.ts` —que es donde esta toda su aritmetica— porque es un VALOR DEL
 * ESTADO: lo guardan `MemoriaFaccion` y `Jugador.exploracionPersonal`, y `domain` no puede mirar hacia
 * `engine`. El tipo es de quien lo almacena; las funciones que lo manipulan, del motor.
 */
export type Exploracion = string;

export type OrigenTropa = 'pesants' | 'artesanos' | 'nobleza';

/**
 * Escuadrón (Doc 5.1/5.4): el jugador lidera una tropa de unidades NPC, nunca combate individualmente.
 * Escuadrón de UN jugador (Doc 2.5, a petición del usuario — corrige el bug donde dos jugadores reclutando la
 * misma tropa en el mismo asentamiento se fundían en un solo escuadrón): `heroeId` + `tropaId` identifican de
 * forma única al escuadrón dentro de `Asentamiento.escuadrones` — un jugador solo puede tener UNO por tropa,
 * porque solo pertenece a un asentamiento (Doc 2.1) y ahí solo puede tener sus propias tropas.
 * El SQUAD (nombre, veteranía) persiste aunque `cantidad` llegue a 0 (aniquilado) — se puede rellenar reclutando
 * más del mismo origen en el asentamiento. PERMADEATH: las bajas reducen `cantidad` de forma permanente.
 */
export interface Escuadron {
  id: string;
  nombre: string;
  /** Dueño del escuadrón (Doc 2.5) — reclutar ya no depende del cargo de General, cualquier jugador residente
   * del asentamiento (fundador o con casa comprada) recluta y amplía SU PROPIO escuadrón. */
  heroeId: string;
  origen: OrigenTropa;
  cantidad: number;
  /** Sube combatiendo (carril combate real, Doc 4.1/5.5): da un bonus de poder continuo al MISMO escuadrón
   * (`poderEscuadron`, engine/combate.ts) — NUNCA cambia `tropaId` (Doc 5.8, a petición del usuario: una tropa
   * jamás cambia de identidad al ganar veteranía). Nobleza no la usa (progresión plana). */
  veterania: number;
  /** Moral 0-100 por suministro de raciones (Doc 5.4); a 0 hay deserción permanente continua. */
  moral: number;
  /** Debuff temporal tras perder en mundo abierto (Doc 5.2.2): penaliza el poder de combate hasta este
   * instante de mundo (Fase D). Ausente = sano. */
  heridoHasta?: Instante;
  /** Tropa reclutada vía Centro Urbano/Barracón/Galería de tiro (Doc 5.7/5.8, ver TROPAS_RECLUTABLES en
   * constants.ts) — determina el poderBase (`poderEscuadron`, engine/combate.ts). Único origen de escuadrones
   * en el motor (`reclutarTropa`, engine/tropas.ts), por eso es obligatorio: "mejorar" una tropa siempre es
   * reclutar una tropa DISTINTA y mejor cuando el edificio suba de nivel interno, nunca transformar el
   * escuadrón existente. */
  tropaId: string;
}

export interface Asentamiento {
  id: string;
  /** Nombre editable por el jugador (a petición del usuario) — puramente de presentación, igual que
   * `Faccion.nombre`. `id` sigue siendo la llave interna estable (lookups, `origenAsentamientoId` /
   * `destinoAsentamientoId` de `Caravana`, `asentamientoId` de `AcuerdoTrueque`/`OrdenMercado`/
   * `CaminoComercial`/`ZonaInfluencia`, etc.) y NUNCA cambia al renombrar. Ausente = usar `id` como display
   * (asentamientos ya existentes de partidas guardadas antes de esta función). */
  nombre?: string;
  faccionId: string;
  heroesFundadoresIds: string[];
  posicion: Point;
  /** NIVEL ALCANZADO (Doc Fase_0_5 §6.2): histórico, MONÓTONO, nunca baja — sube por gates de
   * población+edificios (`calcularNivelAsentamiento`/`avanzarNivelAsentamiento`, engine/mantenimiento.ts).
   * De aquí salen el techo de POBLACIÓN (`NIVEL_ASENTAMIENTO.techoPoblacion`) y el techo de RADIO de zona de
   * influencia (`ZONA_INFLUENCIA.radioMaximoPorNivel`) — ninguno de los dos se purga por una crisis de
   * mantenimiento temporal. Distinto de `nivelActual` (abajo), que sí puede bajar. */
  nivel: number;
  /** NIVEL ACTUAL / operativo (Doc Fase_0_5 §6.2, nuevo): puede subir y bajar según la salud sostenida del
   * mantenimiento — nunca por encima de `nivel` (nivelAlcanzado). De aquí sale qué se puede
   * CONSTRUIR/MEJORAR/RECLUTAR ahora mismo (gates de nivel en `engine/construction.ts`/`engine/expansion.ts`).
   * Baja un escalón cuando el medidor de Mantenimiento toca 0 (en vez de caer en ruinas directamente, salvo
   * ya en nivel 1); solo sube tras mantenimiento sano varios ticks seguidos (`rachaMantenimientoSano`), no
   * cada tick — evita el yo-yo de subir/bajar por un solo bache. Los edificios YA construidos de un nivel
   * superior a `nivelActual` SIGUEN PRODUCIENDO con normalidad — esto solo congela construcción/mejora nueva,
   * nunca apaga nada ni purga población. */
  nivelActual: number;
  /** Racha de ticks CONSECUTIVOS con Mantenimiento pagado en full (Doc Fase_0_5 §6.2) — al llegar a
   * `MANTENIMIENTO.minutosSanosParaRecuperarNivel` sube `nivelActual` un escalón (tope `nivel`) y se reinicia a
   * 0; cualquier tick en déficit también la reinicia a 0. Ausente = 0. */
  rachaMantenimientoSano?: number;
  /** Instante de mundo en que se fundó el asentamiento (Fase D). */
  fundadoEn: Instante;
  /** Radio "potencial" de la zona de influencia si no hubiera fronteras vecinas; crece con el tiempo/nivel. */
  radioPotencial: number;
  poblacion: Poblacion;
  /** Recurso -> cantidad almacenada y capacidad actual (Doc 4.3). */
  almacen: Record<string, RecursoAlmacenado>;
  edificios: Edificio[];
  /** Recintos amurallados, del más interior al más exterior (orden de construcción). Al ampliar, el recinto
   * viejo SE QUEDA con sus puertas abiertas —estratos, no reformas— así que esto es una lista, no un campo
   * único. Ausente = asentamiento sin murallas, que es el caso normal. */
  recintos?: Recinto[];
  cargos: CargosAsentamiento;
  /** Jugadores que compraron casa aquí (Doc 2.5), vía de ciudadanía distinta de fundar. */
  casasCompradas: string[];
  /**
   * Quién puede cruzar la puerta (Doc 1.10.5). La fija el Gobernador y **no expira**: no es una política de
   * las de Doc 4.4 — una puerta que se abre sola a las dos horas y media no es una puerta.
   *
   * Ausente = `faccion_y_aliados`, que es lo que una ciudad hace por defecto: los suyos y los amigos entran,
   * el resto no. Un residente entra SIEMPRE, mire lo que mire esto: nadie se queda fuera de su propia casa.
   */
  politicaDeAcceso?: 'abierto' | 'faccion_y_aliados' | 'solo_faccion' | 'cerrado';
  /** Vetados por el Gobernador, por encima de la política (Doc 1.10.5): un veto cierra la puerta a alguien
   * concreto aunque la plaza esté abierta de par en par. Ausente = nadie. */
  vetadosIds?: string[];
  politicasActivas: PoliticaActiva[];
  escuadrones: Escuadron[];
  /**
   * Ocupación militar tras una conquista (Doc 5.4, `Consideraciones/Ocupacion_Post_Conquista_Definicion.md`):
   * instante de mundo en que TERMINA. Mientras `instante < ocupacionHasta` el asentamiento es INMUNE a un
   * nuevo asedio, recauda oro reducido (`OCUPACION.factorRecaudacion`), crece más lento
   * (`OCUPACION.factorCrecimiento`) y su medidor de mantenimiento no degrada. La ventana es puro tiempo —
   * no se acorta ni se cancela. Se comprueba AL LEER (`estaOcupado`, engine/asentamientoQuery.ts) salvo su
   * expiración, que `avanzarSimulacion` limpia. Ausente = no ocupado (caso normal).
   */
  ocupacionHasta?: Instante;
  /** Mantenimiento (Doc 4.5): medidor 0-100, empieza en 100; a 0 el asentamiento cae en ruinas (se elimina). */
  medidorMantenimiento: number;
  /** Nutrición de la población (Doc 4.1, hambruna — a petición del usuario, espejo de la moral de tropas por
   * ración, ver `Escuadron.moral`): medidor 0-100 que sube mientras el trigo alcanza para el consumo del
   * tick y baja cuando no alcanza (`avanzarNutricionPoblacion`, engine/population.ts). Por encima de 0 solo
   * frena el crecimiento; al tocar 0 y mantenerse ahí empieza a costar población real (pesants+artesanos,
   * nobleza protegida — "los nobles comen primero"). Distinto de la `felicidad` de política/Sacerdote
   * (placeholder aparte en `crecerPoblacion`, aún sin implementar) — esto es solo comida, no bienestar
   * general. Ausente (partidas guardadas antes de este campo) = tratar como 100 (neutro, sin hambre previa
   * que reconstruir), ver `nutricionPoblacionDe` en engine/asentamientoQuery.ts. */
  nutricionPoblacion?: number;
  /** Overhaul de auto-construcción: mientras esté en `true`, el motor deja de detectar/comprometer NUEVAS
   * necesidades (`evaluarNecesidades`) — lo ya pagado (`en_cola`/`en_construccion`) sigue avanzando normal.
   * La adición MANUAL de edificios (`anadirEdificioManualmente`, Gobernador/Maestro de Obras) no se ve
   * afectada. Ausente/`false` = activa. */
  autoConstruccionPausada?: boolean;
  /**
   * ¿Este asentamiento deja repostar a los ejércitos de sus ALIADOS? (Doc 5.13, Paso 8). Ausente = `false`:
   * abrir tu almacén a la columna de otro es una decisión explícita, no el estado por defecto — te cuesta
   * stock real y la reserva de comida protege a tu propia gente, no a la suya.
   *
   * No aplica a los ejércitos propios, que reponen SIEMPRE en cualquier plaza de su Facción sin permiso que
   * valga, ni a los neutrales u hostiles, que no reponen nunca.
   */
  permiteReabastecerAliados?: boolean;
  /** Reserva manual por recurso (0-999, a petición del usuario), calibrada por el Tesorero: se SUMA a
   * `reservaDinamicaConstruccion` (engine/mantenimiento.ts) y solo la respeta el camino AUTOMÁTICO de
   * construcción (`avanzarConstruccion`/`avanzarMejoras`, engine/construction.ts) — `anadirEdificioManualmente`
   * queda exenta a propósito, ya que el jugador la autoriza explícitamente al usarla. Ausente = sin reserva
   * manual extra (comportamiento sin cambios). */
  reservaManual?: Partial<Record<RecursoTipo, number>>;
  /** Desempate anti-inanición de extractores base (ver `EXTRACTOR_DESEMPATE`, constants.ts): ticks
   * CONSECUTIVOS que cada tipo (cantera/corral/minaCobre/mina/minaEstano) lleva proponiéndose como candidato
   * válido en `evaluarNecesidades` sin conseguir cupo — se resetea a 0 en cuanto el tipo consigue cupo o deja
   * de ser candidato. Ausente/tipo ausente = 0 (comportamiento sin cambios: sin historial de inanición). */
  extractoresTicksSinCupo?: Partial<Record<EdificioTipo, number>>;
  /** Instante de mundo en que este asentamiento creó su última caravana (Fundación o comercial) — cooldown
   * compartido entre los dos mecanismos (`CARAVANA_COOLDOWN.cooldownMinutos`, constants.ts): evita que se
   * spamee la creación cuando una caravana recién salida es destruida y el cupo/recursos vuelven a estar
   * disponibles. Ausente = nunca creó ninguna, así que el cooldown no aplica. Ver `puedeCrearCaravana`,
   * engine/asentamientoQuery.ts. */
  ultimaCaravanaCreadaEn?: Instante;
}

export interface ZonaInfluencia {
  asentamientoId: string;
  /** Polígono resultante de recortar el círculo potencial contra las fronteras con asentamientos rivales. */
  poligono: Point[];
}

/**
 * El territorio de UNA facción como una sola silueta: la unión de las `ZonaInfluencia` de todos sus
 * asentamientos (que se solapan por diseño — mismo bando no compite, Doc 1.2). Derivado puro y SOLO PARA
 * DIBUJAR, nunca persistido ni consultado por ninguna regla: la pertenencia territorial se sigue resolviendo
 * asentamiento a asentamiento contra `ZonaInfluencia.poligono`.
 * Lo calcula `computeZonasFusionadasPorFaccion` (engine/zones.ts).
 */
export interface ZonaFaccion {
  faccionId: string;
  /**
   * Lazos CERRADOS del contorno (el último punto enlaza con el primero, no se repite): uno por grupo de
   * asentamientos conectados, más uno por cada hueco interior que la facción rodee sin reclamar. Los huecos
   * salen con orientación opuesta a los contornos exteriores, así que pintarlos todos en un mismo path y
   * rellenar con la regla `nonzero` (la de por defecto en canvas) los recorta solo — ver `unirFormas`.
   */
  contornos: Point[][];
}

export interface NodoRecurso {
  id: string;
  tipo: RecursoTipo;
  rareza: Rareza;
  posicion: Point;
  /**
   * Unidades con las que el yacimiento NACE al generarse el mundo. No es lo que le queda: el agotamiento
   * es estado de partida y vive en `EstadoMapa.extraido` (ver `src/world/mapa.ts`) — para lo que queda de
   * verdad, `mapa.stock(id)`. Separados a propósito: mientras fueron el mismo campo, guardar una foto del
   * historial obligaba a clonar el mapa entero cada tick.
   */
  cantidadInicial: number;
}

/** Bosques: representados como zonas (polígono/círculo), no puntos — fuente de madera. */
export interface ZonaBosque {
  id: string;
  centro: Point;
  radio: number;
  densidad: number; // 0-1, afecta rendimiento de madera
}

/**
 * Relieve (Fase 0.1, Doc `Fase_0_1_Definicion.md`). Deriva de `CampoElevacion` por umbral — nunca se guarda
 * por punto, se consulta con `Mapa.terrenoEn`/`evaluarTerreno`. 'agua' aquí es "elevación baja" (lago/cauce),
 * no mar — comercio marítimo sigue fuera de alcance. 'cima' es la franja de elevación MÁS alta, por encima de
 * 'montana': terreno inhabitable — no se puede fundar ahí (`evaluarViabilidadFundacion`/`fundarAsentamiento`)
 * ni se generan recursos ahí (`RECURSO_BIOMA_PERMITIDO`/`BOSQUE_TERRENO_PERMITIDO` nunca la listan).
 */
export type TerrenoTipo = 'agua' | 'costa' | 'llano' | 'colina' | 'montana' | 'cima';

/**
 * Bioma (Fase 0.1): terreno + fertilidad + humedad (proxy = cercanía a río). Nunca se guarda por punto —
 * ver `evaluarBioma`. Mismo nivel de granularidad que `Rareza`.
 */
export type BiomaTipo = 'agua' | 'costa' | 'estepa' | 'llanuraFertil' | 'colina' | 'montana' | 'cima';

/**
 * Río (Fase 0.1): polilínea de puntos, no celdas — nace en un punto alto y desciende por gradiente de
 * máxima pendiente del campo de elevación hasta agua/borde del mapa, o queda atrapado en un mínimo local
 * (lago) — ver `generarRios`.
 */
export interface RioZona {
  id: string;
  puntos: Point[];
  /** true si el descenso terminó en un mínimo local (gradiente ~0) en vez de llegar a 'agua' o al borde
   * del mapa — informativo, no cambia el trazo. */
  terminaEnLago: boolean;
  /** true si el río es lo bastante largo y de verdad desemboca (no `terminaEnLago`) como para ser
   * navegable — pensado para el comercio fluvial con barcos de fases futuras (ver `RIOS.proporcionNavegable`
   * en `worldgen/config.ts` y `generarRios`). No afecta la geometría del trazo, solo cómo se interpreta. */
  navegable: boolean;
}

/**
 * Región geográfica opcional (Fase 0.2, ver `worldgen/regiones.ts`): sesga la generación de elevación para
 * que se parezca al carácter conocido de una zona real del Egeo/Levante de la Edad de Bronce, en vez del
 * mundo libre de siempre. Vive aquí (no en `worldgen/`) porque `WorldConfig` es una entidad de dominio y
 * `worldgen/` depende de `domain/types`, nunca al revés.
 */
export type RegionId = 'greciaContinental' | 'anatolia' | 'egeo' | 'nilo' | 'mesopotamia';

export interface WorldConfig {
  ancho: number;
  alto: number;
  seed: number;
  /** `undefined` = generación libre de siempre (comportamiento sin cambios). */
  region?: RegionId;
}

// El antiguo `World` (config + recursos + bosques + `fertilidadEn`) ya no existe: la generación devuelve
// `MapaGenerado` (datos puros, ver `src/worldgen/`) y el motor consulta el mapa a través de la fachada
// `Mapa` (`src/world/mapa.ts`), nunca de los arrays crudos.

// --- Sprint 3: Economía (Doc 3) ---

export type CaravanaTipo = 'comercial' | 'militar' | 'construccion' | 'contrabando';

/** Revamp de caravanas (Doc 3.13). Un carro se fabrica en el Mercado ('basico', 20 madera) o en la
 * Carpintería ('reforzado', más capacidad). El catálogo se ampliará más adelante. Ver `CARRO_CATALOGO`. */
export type CarroTipo = 'basico' | 'reforzado';

/** Revamp de caravanas (Doc 3.13). Un carro lleva como mucho UN animal, obligatorio para que se mueva.
 * Buey: lento, más carga, barato. Caballo: rápido, menos carga, caro. Camello: intermedio (la inmunidad al
 * desierto está diferida — no hay bioma árido). Ver `ANIMAL_CATALOGO`. */
export type AnimalTipo = 'buey' | 'caballo' | 'camello';

/** Un carro de una caravana compuesta (Doc 3.13.1). Solo los carros CON animal viajan y cuentan capacidad;
 * un carro sin animal se queda en el origen. */
export interface CarroCaravana {
  tipoCarro: CarroTipo;
  animal?: AnimalTipo;
}

export interface Caravana {
  id: string;
  tipo: CaravanaTipo;
  origenAsentamientoId: string;
  /** Ausente en caravanas de fundación (Doc 1.8): el destino todavía no es un asentamiento, ver `destinoPosicion`. */
  destinoAsentamientoId?: string;
  contenido: Record<string, number>;
  posicionActual: Point;
  /** 0-1, avance a lo largo de la ruta origen->destino. */
  progreso: number;
  /** Polilínea calculada al lanzar la caravana (ver `world/rutas.ts` `calcularRuta` y `engine/movimiento.ts`)
   * — rodea terreno costoso en vez de ir en línea recta, y determina sobre qué longitud real se mide
   * `progreso`. Toda caravana con `destinoAsentamientoId`/`destinoPosicion` (en movimiento) la trae puesta al
   * despacharse. Ausente solo en estado `'disponible'` (flota propia sin asignar todavía, Doc 3.2): ahí no
   * hay trayecto que recorrer hasta la siguiente asignación. */
  ruta?: Point[];
  /** Acuerdo de trueque que generó esta caravana (Doc 3.2) — indica a qué lado del acuerdo pertenece. */
  origenAcuerdoId?: string;
  ladoAcuerdo?: 'A' | 'B';
  /** Caravana de Fundación (Doc 1.8, tipo 'construccion'): punto del mapa donde fundará al llegar, en vez
   * de un asentamiento ya existente — ver `engine/expansion.ts`. */
  destinoPosicion?: Point;
  /** Caravana de Fundación: ciudadanos ya existentes de la Facción que fundarán el nuevo asentamiento al llegar. */
  heroesFundadoresIds?: string[];
  /** Flota de caravanas propias (ampliación de comercio, a petición del usuario): solo para `tipo: 'comercial'`
   * construidas vía Mercado (ver `construirCaravanaComercial`, engine/trade.ts) — un activo persistente y con
   * costo, no un objeto efímero. 'disponible' = construida, parada en `origenAsentamientoId`, sin asignar.
   * 'en_transito' = cargada y en ruta hacia `destinoAsentamientoId`. Al entregar, en vez de desaparecer O de
   * reaparecer instantáneamente en `origenAsentamientoId` (bug corregido a petición del usuario: una caravana
   * NUNCA se teletransporta), pasa a 'retornando' — recorre la MISMA `ruta` en sentido inverso (ver
   * `avanzarCaravanas`, engine/trade.ts) de vuelta a `origenAsentamientoId`, vacía, y solo entonces vuelve a
   * 'disponible'. Ausente para caravanas de Fundación y para los tipos de caravana todavía sin uso real
   * (militar/contrabando, Doc 3.6).
   *
   * 'adjunta' = enganchada a un ejército como tren de suministros o escolta (Doc 5.13.2). Es un ESTADO propio
   * y no un 'disponible' con una bandera aparte por una razón concreta: `asignarCaravanasATrueque` reparte
   * las 'disponible', así que dejarla ahí permitía que el comercio automático la despachara por debajo del
   * ejército que la lleva. Mientras esté 'adjunta' se mueve con la columna, y se carga y entrega A MANO
   * (Doc 5.13.3) — el reparto automático no la ve. Soltarla la devuelve a 'disponible' donde esté.
   *
   * 'preparando' = revamp (Doc 3.13.3): lanzada a mano pero todavía en el origen mientras corre el tiempo de
   * preparación (`preparaHasta`). La carga ya está reservada del almacén y las piezas/escolta bloqueadas;
   * cancelar antes de salir lo devuelve todo. Sin cablear todavía (Paso 3 del plan).
   *
   * 'aparcada' = una caravana adjunta que su ejército dejó en una plaza de la Facción al `guarnecer`
   * (`Ocupacion_Post_Conquista_Definicion.md` §2.3d). Sigue siendo de su `origenAsentamientoId`, hospedada en
   * otra plaza (su `posicionActual`): NO la usa la plaza anfitriona ni el reparto automático, intercambia con
   * el almacén de esa plaza (`moverCargaCarroAparcada`), y solo sale enganchada a un ejército
   * (`adjuntarCaravana`) o enviada a su origen (`enviarCaravanaAlOrigen`). */
  estado?: 'disponible' | 'preparando' | 'adjunta' | 'aparcada' | 'en_transito' | 'retornando';
  /** Revamp de caravanas (Doc 3.13). Solo `tipo: 'comercial'`, y ahí SIEMPRE presente desde el snapshot v12
   * (la migración le puso 1 carro básico + 1 buey a las que venían del modelo viejo). La caravana deriva su
   * capacidad y velocidad de esta lista (`capacidadCaravana`/`velocidadCaravana`, engine/caravanas.ts); sin
   * carros con animal, ambas son 0. Opcional en el tipo solo porque las categorías militar/construccion/
   * contrabando no lo llevan. */
  carros?: CarroCaravana[];
  /** Revamp (Doc 3.13.4). Escuadrones que un residente del origen cede como escolta sin héroe, POR VIAJE. Son
   * los escuadrones EN SÍ (no ids): salen de `Asentamiento.escuadrones` del origen al preparar la caravana y
   * vuelven a la guarnición cuando la caravana regresa (`avanzarCaravanas`). Mientras viajan, la caravana se
   * defiende con su poder (`poderTotal`) en vez de con la defensa base fija (Doc 3.10). Presente solo en viaje. */
  escolta?: Escuadron[];
  /** Revamp (Doc 3.13.5). `true` = fuera del reparto automático (`asignarCaravanasATrueque`), decisión
   * explícita del jugador, sea cual sea el tamaño de la caravana. */
  reservadaManual?: boolean;
  /** Revamp (Doc 3.13.3). Instante de mundo en que termina la preparación; presente solo en estado
   * 'preparando'. Sin cablear todavía (Paso 3). */
  preparaHasta?: Instante;
}

/**
 * Ejército (Doc 5.12): uno o más escuadrones que salieron del asentamiento y se mueven por el mapa como UNA
 * sola entidad, con la misma maquinaria de rutas que las caravanas (`calcularRuta` + `avanzarPosicionEnRuta`).
 *
 * Salir SOLO y salir en ejército no son dos casos: salir solo es un ejército de un participante. Por eso no
 * hay dos tipos ni una lista de participantes guardada — los participantes se DERIVAN de los `heroeId`
 * distintos de sus escuadrones, y ese mismo número es el de rombos a dibujar en el mapa (Doc 5.12.2).
 *
 * Tampoco se guardan: la Facción y el color (salen de `origenAsentamientoId`), el poder (`poderTotal`), la
 * capacidad del carro (nº de participantes × `LOGISTICA.capacidadCarroPorJugador`) ni la velocidad (el `min`
 * sobre las velocidades de sus escuadrones y caravanas adjuntas).
 */
export interface Ejercito {
  id: string;
  faccionId: string;
  /** De dónde salió y a dónde vuelve. Se reasigna al asentamiento propio más cercano si este cae; si la
   * Facción no conserva ninguno, el ejército queda sin hogar y sus jugadores huérfanos (Doc 5.4). */
  origenAsentamientoId: string;
  /**
   * Quién va DENTRO, con independencia de si aporta escuadrones (Doc 5.12.1). Antes se derivaba de los
   * escuadrones, y por eso un jugador sin tropas no existía como participante y una columna cuyos soldados
   * caían todos se volvía un ejército fantasma — con gente dentro y sin forma de decirlo.
   *
   * No es una lista de ids sino de entradas: la sucesión del líder va por ANTIGÜEDAD, y eso no se lee de un
   * array de strings sin depender del orden de inserción, que separarse y volver a unirse reordena.
   */
  participantes: { heroeId: string; unidoEn: Instante }[];
  /**
   * Qué NACIÓ esta columna, fijado al crearla y jamás modificado (Doc 5.12.1). Lo decide EL COMANDO que la
   * pare: `salirAlMundo` —sin destino— hace una columna `personal`; `movilizarEjercito` —contra un
   * destino— hace un `ejercito`, aunque salga uno solo.
   *
   * **No se deriva de `participantes.length`:** un ejército al que se le separan miembros hasta quedar en uno
   * sigue siendo un ejército, con su ruta fija y sus caravanas.
   */
  tipo: 'personal' | 'ejercito';
  /** Quién la formó (Doc 5.14.3). No puede separarse —para irse cede el liderazgo— y es el único que
   * cancela la marcha. Si se desconecta pasa al participante más antiguo. En una columna `personal` es su
   * único participante y no significa nada. */
  liderId: string;
  /**
   * Qué se hace con quien pide unirse en campo (Doc 5.14.1). La fija el Líder al parir la columna y no
   * cambia: quien se une comparte tu carro, tu destino y tus encuentros, así que poder negarse no es un
   * lujo.
   */
  politicaDeUnion: 'rechazar' | 'aceptar' | 'preguntar';
  /**
   * Peticiones vivas cuando la política es `preguntar` (Doc 5.14.1). **Caducan sin temporizador**: nada se
   * dispara a los 10 s, lo comprueban contra el instante actual los dos únicos sitios que las miran —el
   * comando con el que el Líder responde y la proyección del que pidió—. Ausente = ninguna viva.
   */
  peticionesDeUnion?: { heroeId: string; pedidoEn: Instante; expiraEn: Instante }[];
  /** Escuadrones MOVIDOS aquí desde `Asentamiento.escuadrones` — se van de verdad, por eso la guarnición es
   * lo único que defiende (Doc 5.12.4) y por eso `consumoRacionTropas` ya cuenta solo lo que quedó en casa. */
  escuadrones: Escuadron[];
  /** El carro: los de todos sus jugadores, ya sumados. Solo trigo en Fase 0. En marcha se come de AQUÍ, no
   * del almacén (Doc 5.13) — misma regla del hambre vía `avanzarRacion`, distinta despensa. */
  suministro: Record<string, number>;
  /**
   * A quien persigue, si persigue a alguien (Doc 5.12.3). Un objetivo MOVIL en vez de un punto: la ruta se
   * recalcula cada tick hacia donde este. Ausente = marcha normal contra `objetivo`.
   *
   * La persecucion es lo que sustituye al choque automatico: acercarse ya no basta para pelear, hay que
   * haber decidido ir a por alguien. Termina al alcanzarlo —y entonces si hay combate, porque ya lo elegiste—
   * o al rectificar el rumbo.
   */
  persiguiendo?: { tipo: 'ejercito' | 'caravana'; id: string };
  /**
   * Derrotado hace poco (Doc 5.12.3). Corta por los DOS lados: nadie puede perseguirle ni atacarle, y el
   * tampoco puede perseguir ni atacar.
   *
   * El nombre importa. Se llamo `noPerseguibleHasta` mientras se penso como una proteccion, y describia solo
   * la mitad: asi implementada, la inmunidad seria un escudo para depredar sin riesgo.
   */
  enTreguaHasta?: Instante;
  /** Caravanas que marchan con el ejército (Doc 5.13.2): amplían la carga, entran en el `min` de velocidad,
   * pueden ir cargadas de mercancía (escolta, Doc 5.13.3) y se pierden si el ejército es derrotado. */
  caravanasAdjuntasIds: string[];
  objetivo: { tipo: 'asentamiento'; id: string } | { tipo: 'punto'; punto: Point };
  /** Polilínea calculada al movilizar, igual que en `Caravana.ruta`. */
  ruta: Point[];
  /** 0-1 a lo largo de `ruta`. Al cancelar o al terminar, el regreso desanda la MISMA ruta invertida. */
  progreso: number;
  posicionActual: Point;
  /** `estacionado` consume reducido pero nunca 0 (Doc 5.12.3). Cancelar una marcha pasa a `regresando`
   * (Doc 5.12.6). */
  estado: 'marchando' | 'estacionado' | 'regresando';
}

/**
 * Campamento de bandidos (Doc 1.9, a petición del usuario — inspirado en análisis comparativo con Travian):
 * aparece en un bosque sin ninguna zona de influencia encima (territorio no reclamado por ninguna Facción).
 * Ataca caravanas que pasen cerca mientras sigue en pie (`engine/bandidos.ts`); un jugador puede destruirlo
 * con sus escuadrones para obtener recompensa (`atacarCampamentoBandidos`, engine/combate.ts).
 */
export interface CampamentoBandido {
  id: string;
  posicion: Point;
  /** Bosque que ocupa (Doc 1.4/1.9) — determina dónde puede aparecer, no se agota por esto. */
  bosqueId: string;
  /** Asentamiento al que este campamento "atiende" (Doc 1.9) — el spawn asigna como mucho un campamento
   * por asentamiento SIEMPRE, con independencia de si otro asentamiento cercano ya tiene el suyo (ver
   * `engine/bandidos.ts`). Reemplaza a un criterio anterior por distancia que dejaba asentamientos vecinos
   * sin campamento propio para siempre si compartían radio de cobertura con otro. */
  asentamientoId: string;
  /** Poder de combate fijo (placeholder, ver `CAMPAMENTOS_BANDIDOS` en constants.ts) — mismo tipo de
   * resolución que el resto del combate (Doc 5.2/5.10), sin escuadrones propios que sufran bajas graduales. */
  poder: number;
}

/**
 * Contrato marco abierto en el tiempo entre dos asentamientos (Doc 3.2). "Funciona en ambas direcciones":
 * A se compromete a entregar cantidadTotalA de recursoA, B se compromete a entregar cantidadTotalB de recursoB;
 * cada lado despacha sus propias caravanas de forma independiente hasta cumplir su cupo o expirar el plazo.
 */
export interface AcuerdoTrueque {
  id: string;
  asentamientoAId: string;
  asentamientoBId: string;
  recursoA: string;
  recursoB: string;
  cantidadTotalA: number;
  cantidadTotalB: number;
  cantidadEntregadaA: number;
  cantidadEntregadaB: number;
  /** Instantes de mundo de creación y vencimiento del acuerdo (Fase D). */
  creadoEn: Instante;
  /**
   * Cuando deja de valer. Mientras esta `'propuesto'` es el plazo para CONTESTAR; al aceptar se recalcula
   * desde el instante de la aceptacion (`aceptarTrueque`, engine/trade.ts), para que una propuesta contestada
   * tarde no nazca ya sin tiempo de cumplirse.
   */
  expiraEn: Instante;
  /**
   * `'propuesto'` = ofrecido y sin contestar; no obliga a nadie y ninguna caravana lo mira todavia
   * (`Consideraciones/Comercio_Fisico_Definicion.md`, decision 5). Nace asi SIEMPRE: hasta 2026-09-07 nacia
   * `'activo'` porque no habia jugador interactivo al que preguntarle, y eso pactaba en nombre del otro.
   *
   * `'rechazado'` es distinto de `'expirado'` a proposito: uno es una respuesta y el otro un silencio.
   */
  estado: 'propuesto' | 'activo' | 'rechazado' | 'cumplido' | 'expirado';
}

/**
 * Camino comercial (Fase 0.3, Doc 1.6): se genera automáticamente al establecer la primera relación
 * comercial entre dos asentamientos (ver `engine/caminos.ts`, disparado desde `GameStore.proponerTrueque`).
 * Estado de PARTIDA, no de mundo generado — depende de qué relaciones existen, no de la seed. Persiste
 * aunque el `AcuerdoTrueque` que lo originó expire o se cumpla (PENDIENTE en `Preguntas_Abiertas.md`: qué
 * pasa si se rompe la relación — de momento el camino queda como infraestructura física permanente).
 */
export interface CaminoComercial {
  id: string;
  asentamientoAId: string;
  asentamientoBId: string;
  puntos: Point[];
}

/** Orden de compra/venta en el Mercado de un asentamiento, pagada en oro (Doc 3.3/3.4). */
export interface OrdenMercado {
  id: string;
  asentamientoId: string;
  tipo: 'compra' | 'venta';
  recurso: string;
  cantidad: number;
  cantidadCumplida: number;
  precioUnitario: number;
  /** Instante de mundo en que se colocó la orden (Fase D). */
  creadoEn: Instante;
  /**
   * Cuando la oferta se retira sola (`MERCADO.plazoOrdenMinutos`).
   *
   * No es un adorno: desde que las ordenes se cumplen EN EL MOSTRADOR y no por emparejamiento automatico
   * (`Consideraciones/Comercio_Fisico_Definicion.md` §3), una orden que nadie toma no se cierra nunca. Sin
   * caducidad, una plaza acumularia ofertas eternas a precios viejos y no volveria a ajustarlos jamas.
   *
   * Ausente en ordenes de partidas guardadas antes de 2026-09-07; se rellena al migrar.
   */
  expiraEn: Instante;
  estado: 'activa' | 'cumplida' | 'expirada';
}

// --- Sprint 4: Estructura política (Doc 2) ---

export type CargoTipo = 'gobernador' | 'tesorero' | 'general' | 'maestroObras' | 'sacerdote';

/** Mismo mecanismo de exhaustividad que `RECURSOS_TIPO`/`EDIFICIOS_TIPO`. */
const TODOS_LOS_CARGOS: Record<CargoTipo, true> = {
  gobernador: true,
  tesorero: true,
  general: true,
  maestroObras: true,
  sacerdote: true,
};
export const CARGOS_TIPO = Object.keys(TODOS_LOS_CARGOS) as CargoTipo[];

/** Política activa en un asentamiento (Doc 4.4): slots/pools por cargo, duración fija, no cancelable antes de tiempo. */
export interface PoliticaActiva {
  id: string;
  politicaId: string;
  cargo: CargoTipo;
  /** Instantes de mundo de activación y vencimiento (Fase D). Duración fija, no cancelable antes de tiempo. */
  activadaEn: Instante;
  expiraEn: Instante;
}

/**
 * Vasallaje o Alianza ENTRE FACCIONES (nunca entre asentamientos sueltos, Doc 0/2.4). En vasallaje,
 * faccionAId es la Facción señora y faccionBId la vasalla; en alianza la relación es simétrica.
 * La Liga (Doc 0) no se guarda como entidad propia: se DERIVA de la red de relaciones activas (ver engine/liga.ts).
 */
export interface RelacionPolitica {
  id: string;
  tipo: 'vasallaje' | 'alianza';
  faccionAId: string;
  faccionBId: string;
  /** Tributo periódico del vasallo al señor (Doc 2.4), solo aplica a vasallaje — `cantidadPorMinuto` unidades
   * de mundo por minuto (D6; 1 tick = 1 minuto, ver `SIMULACION`). */
  tributo?: { recurso: string; cantidadPorMinuto: number };
  /** Instante de mundo en que se estableció la relación (Fase D). */
  creadoEn: Instante;
  estado: 'activa' | 'rota';
}

// --- Sprint 6: Cierre (Doc 2.7/2.9, mantenimiento Doc 4.5) ---

/**
 * Título dinámico de PRESTIGIO (Doc 2.9): sin beneficio mecánico, recalculado periódicamente según poder
 * relativo. Se deriva bajo demanda (no se persiste como estado propio) — ver engine/titulos.ts.
 */
export interface Titulo {
  nombre: string;
  /** Facción (o, según el título, jugador) que lo ostenta actualmente. */
  poseedorId: string;
  valorMetrica: number;
}
