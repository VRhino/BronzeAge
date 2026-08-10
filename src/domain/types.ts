// Entidades núcleo — Sprint 1 (mundo/fundación) + Sprint 2 (población/construcción/almacén)
// + Sprint 3 (economía) + Sprint 4 (estructura política) + Sprint 5 (guerra) + Sprint 6 (cierre).
// Ver Consideraciones/Plan_Implementacion_Tecnica.md para el modelo de datos completo.
// Murallas/torres/puerto (edificios estratégicos de colocación manual, Doc 4.2) quedan fuera de Fase 0.

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

export type Rareza = 'comun' | 'intermedio' | 'raro';

/** Cargos de nivel Facción (Doc 2.2): Rey (vasallaje, políticas superiores) y Embajador (designado por el Rey). */
export interface Faccion {
  id: string;
  nombre: string;
  reyId: string | null;
  embajadorId: string | null;
  /** Sube según actividad de la Facción (Doc 1.7) — ver NIVEL_FACCION en constants.ts para el criterio placeholder. */
  nivel: number;
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
  // de la flota de caravanas propias (ver CARAVANA_CATALOGO.comercial, engine/trade.ts). Vía política del
  // Tesorero, mismo patrón que Barracón/Galería de tiro/Palacio — no auto-construcción.
  | 'mercado';

export type EstadoEdificio = 'en_cola' | 'en_construccion' | 'activo';

export interface Edificio {
  id: string;
  tipo: EdificioTipo;
  posicion: Point;
  estado: EstadoEdificio;
  ticksRestantes: number;
  /** Nodo de recurso o zona de bosque que explota (cantera/lenera/corral), si aplica. */
  fuenteId?: string;
  /** Nivel interno de mejora (Doc 4.2.1): solo edificios de transformación con tiers (Fundición, Curtiduría,
   * Armería, Carpintería, Barracón, Galería de tiro). Ausente/1 para el resto. */
  nivelInterno?: number;
  /** Overhaul de auto-construcción: score de necesidad (ver `SCORE_BANDAS`, constants.ts) capturado en el
   * momento en que el proyecto se comprometió (pagó) y entró `en_cola` — determina el orden real en que
   * arranca la construcción cuando compite por un hueco de `maximoEnConstruccionSimultanea`. Ausente para
   * edificios que nunca pasan por la cola (ej. `centroUrbano`). */
  prioridad?: number;
}

/** Cargos de nivel asentamiento (Doc 2.2), uno de cada, designados por el Gobernador salvo él mismo. */
export interface CargosAsentamiento {
  gobernadorId: string | null;
  tesoreroId: string | null;
  generalId: string | null;
  maestroObrasId: string | null;
  sacerdoteId: string | null;
}

export type TropaTier = 1 | 2 | 3 | 4;
export type OrigenTropa = 'pesants' | 'artesanos' | 'nobleza';

/**
 * Escuadrón (Doc 5.1/5.4): el jugador lidera una tropa de unidades NPC, nunca combate individualmente.
 * El SQUAD (nombre, veteranía) persiste aunque `cantidad` llegue a 0 (aniquilado) — se puede rellenar reclutando
 * más del mismo origen en el asentamiento. PERMADEATH: las bajas reducen `cantidad` de forma permanente.
 */
export interface Escuadron {
  id: string;
  nombre: string;
  origen: OrigenTropa;
  tier: TropaTier;
  cantidad: number;
  /** Sube combatiendo (carril combate real, Doc 4.1/5.5); Nobleza no la usa (progresión plana). */
  veterania: number;
  /** Moral 0-100 por suministro de raciones (Doc 5.4); a 0 hay deserción permanente continua. */
  moral: number;
  /** Debuff temporal tras perder en mundo abierto (Doc 5.2.2), penaliza poder de combate mientras dura. */
  heridoHastaTick?: number;
  /** Tropa reclutada vía Centro Urbano/Barracón/Galería de tiro (Doc 5.7/5.8, ver TROPAS_RECLUTABLES en constants.ts) — solo
   * presente para escuadrones de Pesants reclutados por equipo. Determina el poderBase (ver `poderEscuadron`,
   * engine/combate.ts) en vez de TROPA_CATALOGO[tier], y desactiva el ascenso automático de tier por veteranía
   * (ver `ascenderTierSiCorresponde`, engine/tropas.ts) — "mejorar" pasa a ser reclutar una tropa mejor cuando
   * el edificio suba de nivel interno, no ascender el mismo escuadrón. Ausente para Artesanos/Nobleza. */
  tropaId?: string;
}

export interface Asentamiento {
  id: string;
  faccionId: string;
  jugadoresFundadoresIds: string[];
  posicion: Point;
  nivel: number;
  fundadoEnTick: number;
  /** Radio "potencial" de la zona de influencia si no hubiera fronteras vecinas; crece con el tiempo/nivel. */
  radioPotencial: number;
  poblacion: Poblacion;
  /** Recurso -> cantidad almacenada y capacidad actual (Doc 4.3). */
  almacen: Record<string, RecursoAlmacenado>;
  edificios: Edificio[];
  cargos: CargosAsentamiento;
  /** Jugadores que compraron casa aquí (Doc 2.5), vía de ciudadanía distinta de fundar. */
  casasCompradas: string[];
  politicasActivas: PoliticaActiva[];
  escuadrones: Escuadron[];
  /** Mantenimiento (Doc 4.5): medidor 0-100, empieza en 100; a 0 el asentamiento cae en ruinas (se elimina). */
  medidorMantenimiento: number;
  /** Overhaul de auto-construcción: mientras esté en `true`, el motor deja de detectar/comprometer NUEVAS
   * necesidades (`evaluarNecesidades`/`evaluarEdificiosEspeciales`) — lo ya pagado (`en_cola`/`en_construccion`)
   * sigue avanzando normal. La construcción manual (Gran Fundición) no se ve afectada. Ausente/`false` = activa. */
  autoConstruccionPausada?: boolean;
}

export interface ZonaInfluencia {
  asentamientoId: string;
  /** Polígono resultante de recortar el círculo potencial contra las fronteras con asentamientos rivales. */
  poligono: Point[];
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
 * Chokepoint estratégico (Fase 0.3, Doc 1.5): puerto de montaña detectado como punto de silla del campo de
 * elevación (ver `worldgen/chokepoints.ts`) — geometría, no arista de un grafo (ver `Fase_0_1_Definicion.md`).
 * `radio` es la zona de influencia del propio chokepoint: qué zona de asentamiento lo controla
 * (`engine/chokepoints.ts`) y a qué distancia de una ruta cuenta como "la ruta pasa por aquí" para el peaje.
 */
export interface Chokepoint {
  id: string;
  posicion: Point;
  radio: number;
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
  /** Polilínea calculada al lanzar la caravana (Fase 0.3, ver `world/rutas.ts` `calcularRuta` y
   * `engine/movimiento.ts`) — rodea terreno costoso en vez de ir en línea recta, y determina sobre qué
   * longitud real se mide `progreso`. Ausente en caravanas de partidas guardadas antes de Fase 0.3: esas
   * siguen moviéndose en línea recta sin coste de terreno, comportamiento sin cambios. */
  ruta?: Point[];
  /** Acuerdo de trueque que generó esta caravana (Doc 3.2) — indica a qué lado del acuerdo pertenece. */
  origenAcuerdoId?: string;
  ladoAcuerdo?: 'A' | 'B';
  /** Caravana de Fundación (Doc 1.8, tipo 'construccion'): punto del mapa donde fundará al llegar, en vez
   * de un asentamiento ya existente — ver `engine/expansion.ts`. */
  destinoPosicion?: Point;
  /** Caravana de Fundación: ciudadanos ya existentes de la Facción que fundarán el nuevo asentamiento al llegar. */
  jugadoresFundadoresIds?: string[];
  /** Flota de caravanas propias (ampliación de comercio, a petición del usuario): solo para `tipo: 'comercial'`
   * construidas vía Mercado (ver `construirCaravanaComercial`, engine/trade.ts) — un activo persistente y con
   * costo, no un objeto efímero. 'disponible' = construida, parada en `origenAsentamientoId`, sin asignar.
   * 'en_transito' = cargada y en ruta hacia `destinoAsentamientoId`. Al entregar, vuelve a 'disponible' en vez
   * de desaparecer (a diferencia del resto de tipos de caravana, que siguen siendo efímeros). Ausente para
   * caravanas de Fundación y para los tipos de caravana todavía sin uso real (militar/contrabando, Doc 3.6). */
  estado?: 'disponible' | 'en_transito';
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
  creadoEnTick: number;
  expiraEnTick: number;
  estado: 'activo' | 'cumplido' | 'expirado';
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
  creadoEnTick: number;
  estado: 'activa' | 'cumplida';
}

// --- Sprint 4: Estructura política (Doc 2) ---

export type CargoTipo = 'gobernador' | 'tesorero' | 'general' | 'maestroObras' | 'sacerdote';

/** Política activa en un asentamiento (Doc 4.4): slots/pools por cargo, duración fija, no cancelable antes de tiempo. */
export interface PoliticaActiva {
  id: string;
  politicaId: string;
  cargo: CargoTipo;
  activadaEnTick: number;
  expiraEnTick: number;
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
  /** Tributo periódico del vasallo al señor (Doc 2.4), solo aplica a vasallaje. */
  tributo?: { recurso: string; cantidadPorTick: number };
  creadoEnTick: number;
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
