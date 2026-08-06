// Valores numéricos PLACEHOLDER — ver Consideraciones/Preguntas_Abiertas.md.
// Centralizados aquí para poder re-balancear sin tocar la lógica del motor.

export const WORLD_DEFAULT: { ancho: number; alto: number } = {
  ancho: 1000,
  alto: 1000,
};

// Trigo NO genera nodo: depende del campo de fertilidad (ver FERTILIDAD) + Granja (Sprint 2).
// Madera TAMPOCO genera nodo propio (Doc 1.4: "proviene de BOSQUES, representados como ZONAS, no puntos") —
// solo se generaban aquí por error de implementación (Sprint 1): un nodo "madera" sin ningún uso en el motor
// (la lenera siempre lee de world.bosques, nunca de world.recursos), y visualmente confundible con los bosques.
// Cantidad de nodos por 1000x1000 y espaciado mínimo entre nodos de la misma rareza (unidades de mapa).
export const RECURSO_RAREZA = {
  comun: { cantidadBase: 60, espacioMinimo: 20 },
  intermedio: { cantidadBase: 20, espacioMinimo: 40 },
  raro: { cantidadBase: 6, espacioMinimo: 120 },
} as const;

export const RECURSO_TIPOS_POR_RAREZA: Record<keyof typeof RECURSO_RAREZA, string[]> = {
  comun: ['piedra'],
  intermedio: ['cobre'],
  raro: ['estano', 'oro'],
};

export const RECURSO_CANTIDAD_NODO = {
  piedra: { min: 200, max: 500 },
  cobre: { min: 100, max: 300 },
  estano: { min: 50, max: 150 },
  oro: { min: 30, max: 100 },
} as const;

// Livestock: fauna libre, no sigue las mismas reglas de rareza (no ligada a minerales).
export const LIVESTOCK = {
  cantidadBase: 25,
  espacioMinimo: 30,
  cantidadPorManada: { min: 10, max: 40 },
};

export const BOSQUE = {
  cantidad: 25,
  radioMin: 30,
  radioMax: 80,
  densidadMin: 0.4,
  densidadMax: 1.0,
};

// Fertilidad: ruido continuo por suma de funciones seno con distintas frecuencias (sin dependencias externas).
export const FERTILIDAD = {
  octavas: 3,
  escala: 0.006,
};

// Zona de influencia: radio inicial al fundar, crecimiento por tick y tope máximo (escalará con nivel en sprints futuros).
export const ZONA_INFLUENCIA = {
  radioInicial: 15,
  crecimientoPorTick: 1.5,
  radioMaximo: 120,
  segmentosPoligono: 48, // resolución del círculo aproximado como polígono
};

export const FUNDACION = {
  maxJugadoresFundacionGrupal: 5,
  // La caravana de fundación (Doc 1.3) trae una reserva básica para arrancar la primera construcción
  // (sin esto, ningún edificio podría levantarse: la lenera que produce madera también cuesta madera).
  materialesIniciales: { madera: 60, piedra: 20 } as Record<string, number>,
};

// --- Sprint 2: Población, construcción automática y almacenamiento (Doc 4) ---

export const POBLACION = {
  pesants: { inicial: 20, tasaCrecimientoBase: 0.05 },
  artesanos: { tasaCrecimientoBase: 0.03, capacidadPorTaller: 15 },
  // "Cantidad mínima de ciudadanos" sin número fijado en el diseño (ver Preguntas_Abiertas) — placeholder.
  nobleza: { minCiudadanos: 3, tasaCrecimientoBase: 0.01 },
  consumoComidaPorHabitante: 0.1, // trigo/tick por habitante (pesants+artesanos+nobleza)
};

export const EDIFICIO_CATALOGO = {
  vivienda: { costo: { madera: 40, piedra: 20 }, tiempoConstruccionTicks: 4, capacidadHabitantes: 15 },
  granja: { costo: { madera: 30 }, tiempoConstruccionTicks: 5, produccionBaseTrigo: 6, trabajadoresRequeridos: 4 },
  cantera: { costo: { madera: 20 }, tiempoConstruccionTicks: 5, produccionBasePiedra: 5, trabajadoresRequeridos: 4 },
  lenera: { costo: { madera: 10 }, tiempoConstruccionTicks: 4, produccionBaseMadera: 5, trabajadoresRequeridos: 4 },
  almacen: { costo: { madera: 50, piedra: 30 }, tiempoConstruccionTicks: 6, capacidadPorRecursoAdicional: 300 },
  // Genérico: representa cualquier edificio de producción especializada (Fundición/Curtidor/Carpintería, ver Doc 5.7)
  // que dispara la aparición de Artesanos; se especializa en Sprint 5 (militar).
  taller: { costo: { madera: 60, piedra: 20 }, tiempoConstruccionTicks: 8 },
  // Oro: metal precioso en bruto, origen en minas igual que cualquier otro recurso (Doc 3.1).
  mina: { costo: { madera: 40 }, tiempoConstruccionTicks: 6, produccionBaseOro: 2, trabajadoresRequeridos: 4 },
  // Cobre (Doc 1.1/5.7): "relativamente abundante" — igual patrón que cantera/mina pero sobre nodos de cobre.
  minaCobre: { costo: { madera: 30 }, tiempoConstruccionTicks: 5, produccionBaseCobre: 4, trabajadoresRequeridos: 4 },
  // Fundición/Gran Fundición (Doc 5.7): edificios de colocación MANUAL (decisión militar deliberada, no
  // auto-construcción por necesidad). Curtidor-Armero/Carpintería se abstraen dentro de estas dos (cadenas
  // de producción invisibles, Doc 4.2) — Fase 0 no necesita rastrear cada oficio como edificio separado.
  fundicion: { costo: { madera: 80, piedra: 40 }, tiempoConstruccionTicks: 10 },
  // "Único edificio de tier élite, exclusivo de asentamientos/Facciones de mayor nivel" — gate por nivel de Facción.
  granFundicion: { costo: { madera: 150, piedra: 100, oro: 50 }, tiempoConstruccionTicks: 20, nivelFaccionMinimo: 3 },
} as const;

export const ALMACEN = {
  capacidadInicialPorRecurso: 200,
};

// Umbrales que disparan auto-construcción por necesidad (Doc 4.2). Placeholders razonables.
export const NECESIDADES = {
  umbralViviendaOcupada: 0.85,
  umbralComidaTicksReserva: 5,
  umbralAlmacenAmpliacion: 0.9,
  pesantsParaHabilitarTaller: 30, // placeholder: sustituye a un disparador de excedente de cobre aún no modelado
};

// Colocación de edificios: crecimiento concéntrico desde el centro (Doc 4.2).
export const SITIO = {
  anillos: 6,
  muestrasPorAnillo: 16,
  muestrasFertilidad: 40,
  espacioMinimoEntreEdificios: 8,
};

// --- Sprint 3: Economía (Doc 3) ---

// 4 categorías de caravana (Doc 3.6). Militar/construcción se activan en sprints posteriores
// (logística de guerra y fundación/ascenso respectivamente); Sprint 3 solo despacha 'comercial'.
export const CARAVANA_CATALOGO = {
  comercial: { capacidad: 60, velocidad: 8 },
  militar: { capacidad: 40, velocidad: 6 },
  construccion: { capacidad: 150, velocidad: 5 },
  contrabando: { capacidad: 20, velocidad: 12 },
} as const;

export const TRUEQUE = {
  // "expirar un plazo" sin número fijado en el diseño (ver Preguntas_Abiertas) — placeholder.
  plazoTicksPorDefecto: 200,
};

// Precio de referencia por defecto, según escasez/abundancia GLOBAL (Doc 3.4, sin componente de distancia).
export const PRECIO_BASE: Record<string, number> = {
  madera: 1,
  piedra: 1.2,
  trigo: 1.5,
  cobre: 3,
  estano: 6,
  livestock: 2,
};

export const PRECIO_REFERENCIA = {
  // Nivel de stock GLOBAL (sumado entre todos los asentamientos) considerado "saludable" por recurso — placeholder.
  stockObjetivoGlobal: 500,
  factorMin: 0.4,
  factorMax: 3,
};

// Comisión de comercio (Doc 3.5): más baja intra-Facción. Bonificación por distancia (Doc 3.8) sobre trueques.
export const COMISION = {
  tasaMismaFaccion: 0.03,
  tasaExterna: 0.08,
  bonusPorDistanciaMax: 1.5,
  distanciaParaBonusMax: 600,
};

// --- Sprint 4: Estructura política (Doc 2) ---

// "Qué hace subir el nivel de Facción" no está cerrado en el diseño (Doc 1.7/Preguntas_Abiertas) — placeholder:
// combina nº de asentamientos propios y población NPC total de la Facción.
export const NIVEL_FACCION = {
  puntosPorAsentamiento: 1,
  poblacionPorPunto: 60,
  puntosPorNivel: 2,
  nivelMaximo: 10,
};

// Cap de fundación (Doc 1.7): "progresión fácil de 1 a 3, luego se complica hasta un máximo de 7" — curva placeholder.
export const CAP_FUNDACION_POR_NIVEL = [1, 2, 3, 3, 4, 5, 5, 6, 6, 7] as const;

export const CIUDADANIA = {
  // Espacios de "casas" por asentamiento (Doc 2.5): el cupo base coincide con el máximo de jugadores que
  // pueden fundar juntos (FUNDACION.maxJugadoresFundacionGrupal) — así el asentamiento siempre nace con
  // sitio para todos sus fundadores, y el resto queda libre para compras posteriores.
  casasBasePorAsentamiento: FUNDACION.maxJugadoresFundacionGrupal,
  casasPorNivelAdicional: 2,
};

export const POLITICAS = {
  // Slots por cargo (Doc 4.4): Gobernador escala con el nivel de Facción hasta un máximo de 5.
  slotsPorCargo: {
    gobernador: { base: 2, maximo: 5 },
    tesorero: { base: 2, maximo: 2 },
    general: { base: 1, maximo: 1 },
    maestroObras: { base: 1, maximo: 1 },
    sacerdote: { base: 1, maximo: 1 },
  } as const,
  nivelFaccionPorSlotExtraGobernador: 3,
  duracionTicksPorDefecto: 150,
};

/**
 * Catálogo de políticas — Doc 4.4 deja el catálogo concreto PENDIENTE (ver Preguntas_Abiertas); se define aquí
 * un puñado ilustrativo con efecto mecánico real en sistemas ya existentes, no solo flags inertes.
 * El Gobernador puede activar cualquiera (pool completa); el resto de cargos solo las de su propio pool.
 */
export const POLITICA_CATALOGO = [
  { id: 'racionamiento', cargo: 'sacerdote', nombre: 'Racionamiento', factorConsumoComida: 0.8 },
  { id: 'culto_fertilidad', cargo: 'sacerdote', nombre: 'Culto a la Fertilidad', factorCrecimientoNobleza: 1.5 },
  { id: 'via_rapida', cargo: 'maestroObras', nombre: 'Vía Rápida de Construcción', factorTiempoConstruccion: 0.75 },
  { id: 'postura_defensiva', cargo: 'maestroObras', nombre: 'Postura Defensiva' }, // flag de layout, Doc 4.2 — sin efecto visual en Fase 0
  { id: 'comercio_abierto', cargo: 'tesorero', nombre: 'Comercio Abierto', factorComisionExterna: 0.6 },
  { id: 'aranceles', cargo: 'tesorero', nombre: 'Aranceles Proteccionistas', factorComisionExterna: 1.5 },
  { id: 'leva_forzosa', cargo: 'general', nombre: 'Leva Forzosa', factorCostoReclutamiento: 0.7 },
] as const;

// --- Sprint 5: Guerra simplificada (Doc 5) ---

export const TROPA_CATALOGO: Record<number, { nombre: string; poderBase: number }> = {
  1: { nombre: 'Lanceros', poderBase: 3 },
  2: { nombre: 'Arqueros', poderBase: 6 },
  3: { nombre: 'Lanceros Pesados', poderBase: 12 },
  4: { nombre: 'Guerreros de Élite', poderBase: 25 },
};

/**
 * Reclutamiento (Doc 4.1/5.7): Pesants nacen Tier 1, Artesanos nacen Tier 2 (no necesitan veteranizar),
 * Nobleza nace Tier 4 directo (progresión plana, requiere Gran Fundición). El ascenso 1→2→3 por veterania
 * vía combate real se define en ASCENSO_TROPA. Costos PLACEHOLDER (sin cifras cerradas en el diseño).
 */
export const RECLUTAMIENTO = {
  pesants: { tierInicial: 1 as const, costo: { cobre: 1 } },
  artesanos: { tierInicial: 2 as const, costo: { cobre: 2 } },
  nobleza: { tierInicial: 4 as const, costo: { cobre: 4, estano: 2, oro: 3 }, requiereEdificio: 'granFundicion' as const },
};

export const ASCENSO_TROPA = {
  veteraniaParaTier2: 3,
  // Tier 2 -> 3 requiere ADEMÁS Fundición activa en el asentamiento (Doc 5.8: "Requiere Fundición + veteranía").
  veteraniaParaTier3: 8,
};

export const MILITAR = {
  racionPorSoldadoPorTick: 0.15,
  regeneracionMoralPorTick: 5,
  degradacionMoralSinRacion: 20,
  // Fracción de la cantidad del escuadrón que deserta por tick mientras la moral está a 0 (Doc 5.4).
  desercionFraccionPorTickSinMoral: 0.05,
  bonusVeteraniaPorPunto: 0.05,
  veteraniaGanadaPorVictoria: 1,
  veteraniaGanadaPorDerrota: 0.5,
  // Cohesión entre escuadrones defendiendo juntos (Doc 5.3), abstraída como bonus de poder (sin formaciones renderizadas).
  bonusCohesionPorEscuadronExtra: 0.1,
  duracionHeridoTicks: 30,
  penalizacionHerido: 0.5,
  varianzaCombate: 0.15,
  // Combate de caravanas (Doc 3.10): umbral de captura del 50% y defensa base de una escolta no modelada en detalle.
  umbralCapturaCaravana: 0.5,
  defensaBaseCaravana: 15,
};

// --- Sprint 6: Cierre (Doc 4.5 mantenimiento, Doc 2.7 reputación, Doc 2.9 progresión) ---

// Nivel de asentamiento: "qué lo hace subir" no está cerrado en el diseño — mismo criterio placeholder que
// el nivel de Facción (NIVEL_FACCION), aplicado aquí a escala de un solo asentamiento.
export const NIVEL_ASENTAMIENTO = {
  poblacionPorPunto: 40,
  puntosPorEdificioActivo: 1,
  puntosPorNivel: 3,
  nivelMaximo: 10,
};

/**
 * Mantenimiento (Doc 4.5): coste periódico que escala por nivel (sumando materiales, no reemplazando) y por
 * distancia al centro de poder de la Facción (aquí: su asentamiento más antiguo vivo, como proxy de "capital").
 * Cantidades y velocidad de degradación son PLACEHOLDER (Preguntas_Abiertas no fija cifras exactas).
 */
export const MANTENIMIENTO = {
  medidorInicial: 100,
  // Trigo deliberadamente bajo: compite por la misma Granja que ya alimenta a la población (Doc 4.1) y con
  // varios edificios de extracción repartiéndose el mismo pool de Pesants (Doc 4.2) — un coste alto aquí
  // hacía que CUALQUIER asentamiento entrase en espiral de déficit sin importar la gestión, no solo el abandono.
  costoBase: { madera: 3, trigo: 1 },
  nivelParaPiedra: 3,
  piedraBase: 3,
  // Nivel 8 (no 6): el oro es un recurso RARO por diseño (Doc 1.1) — exigirlo demasiado pronto condena a
  // cualquier asentamiento sin mina de oro local a la ruina salvo que ya tenga una red de trueque activa
  // (Doc 3.9, dependencia logística real). Se deja como concern de nivel tardío, no del arranque en solitario.
  nivelParaOro: 8,
  oroBase: 2,
  factorCrecimientoPorNivel: 0.15,
  escalaDistancia: 400,
  factorDistanciaMax: 2,
  degradacionPorDeficitTotal: 10,
  regeneracionSiPagoCompleto: 5,
  // Protección temporal a asentamientos recién fundados (Doc 1.3, pendiente en el diseño): sin esto, todo
  // asentamiento nuevo entra en déficit desde el tick 1 (antes de que la Granja llegue a construirse) y cae
  // en ruinas pase lo que pase. La gracia cubre el tiempo típico de estabilizar la economía base.
  graciaTicks: 60,
};

/**
 * Reputación de Facción (Doc 2.7): score público -100..+100, decae hacia 0 sin eventos nuevos.
 * Cifras exactas de cada evento sin cerrar en el diseño (Preguntas_Abiertas) — valores placeholder razonables.
 */
export const REPUTACION = {
  decaimientoPorTick: 0.2,
  bonusTruequeCumplido: 5,
  penalizacionTruequeIncumplido: -8,
  bonusLiberarVasalloVoluntario: 6,
  penalizacionRebelionParaSenora: -10,
  penalizacionRomperAlianza: -12,
  penalizacionAtacarAliado: -25,
  bonusPorTickAlianzaActiva: 0.05,
  // Restricción del Embajador (Doc 2.7, uso 3): por debajo de este umbral no puede proponer alianzas.
  umbralBajoParaEmbajador: -40,
  // Términos de comercio asimétricos (Doc 2.7, uso 1): score bajo encarece la comisión que paga esa Facción.
  umbralBajoParaComision: -40,
  factorComisionPorReputacionBaja: 1.5,
};
