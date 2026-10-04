import type { ContadorLogro, EdificioCampamentoTipo, EdificioTipo, EraId, GrupoPuerta, RecursoTipo, TecnologiaId } from './domain/types';

// Valores numéricos PLACEHOLDER — ver Consideraciones/Preguntas_Abiertas.md.
// Centralizados aquí para poder re-balancear sin tocar la lógica del motor.
//
// NO están aquí los parámetros de GENERACIÓN de mundo (tamaño del mapa, rareza/cantidad de nodos, bosques,
// fertilidad): viven congelados en `src/worldgen/config.ts`, y la generación no es editable en caliente — la
// partida guardada solo almacena la seed y regenera el mapa al cargar. Ver el encabezado de `worldgen/config.ts`.
//
// Servido por HTTP sin autenticar en `GET /v1/balance` (Fase C7, `server/rutas/balance.ts`) — es regla
// pública (T1 en doc 9), no estado de partida: publicar las 40 tablas completas es más simple que mantener
// una lista de exclusión, y ninguna es una fuga de estado de un rival. El panel de administración que las
// editaba en caliente (`app/balanceConfig.ts`) se retiró al extraer el cliente (Fase C0) y no tiene dueño
// todavía — hoy este módulo es de solo lectura en tiempo de ejecución, mutable únicamente editando el código.

/**
 * Versión del balance. Sube cuando cambia el CONTENIDO de una tabla de este archivo (no cuando se retira
 * una tabla entera por eliminar una mecánica — eso ya lo delata el propio JSON servido). Se estampa en
 * `PartidaExportada.balanceVersion` al crear una partida (`GameSession.exportar`), como registro de qué
 * versión estaba viva en ese momento — a diferencia de `WORLDGEN_VERSION`, un desajuste NO se rechaza al
 * cargar el snapshot: el balance no es necesario para reconstruir el estado ya guardado (a diferencia de la
 * seed, que sin la versión exacta del algoritmo no reproduce el mismo mapa), solo cambia qué reglas rigen
 * los próximos comandos y ticks. Sin overrides por partida todavía: es un único valor de proceso, no hay
 * mecanismo para que dos partidas abiertas a la vez corran versiones distintas.
 *
 * v1 (2026-08-26): primera versión con número explícito — el balance existía desde antes, pero sin
 * identificador publicable.
 * v2 (2026-08-29): añadida la tabla `SIMULACION` (modelo temporal, Fase D — ver doc 10).
 * v3 (2026-08-29): D6 — plazos renombrados de ticks a minutos (`tiempoConstruccionMinutos`,
 *   `plazoMinutosPorDefecto`, `graciaMinutos`, `cooldownMinutos`, `respawnMinutos`, `duracionHeridoMinutos`,
 *   `duracionMinutosPorDefecto`) y tasas `*PorTick` → `*PorMinuto`. Mismos VALORES (1 tick = 1 min), otras
 *   claves en el JSON servido.
 * v4-v7: bumps sin entrada aquí (el registro se dejó de mantener entre 2026-08-29 y 2026-09-04).
 * v8 (2026-09-04): niebla de guerra, Paso 1 — nueva tabla `VISION`, con `radioVisionEjercito` mudado desde
 *   `LOGISTICA` (mismo valor, 150) y el nuevo `margenAsentamiento`. Ninguna de las dos se sirve todavía por
 *   `GET /v1/balance`, que arrastra nueve tablas sin dar de alta.
 * v9 (2026-09-13): BA-005 — huellas a la mitad (`EDIFICIO_TAMANO`, `PUESTO_MERCADO_FORMA`, `granja.niveles`),
 *   `TRAZADO` de escala de edificio a la mitad y nueva `LAYOUT_VERSION` en `geometriaUrbana`. Esta sí
 *   invalida partidas guardadas, pero por `LAYOUT_VERSION`, no por este número.
 * v10 (2026-09-14): modelo de Héroe, fase 2 — `MILITAR.bonusVeteraniaPorPunto`/`veteraniaGanada*` pasan a
 *   `bonusExperienciaPorPunto`/`experienciaGanada*` (mismos valores), y salen `duracionHeridoMinutos` y
 *   `penalizacionHerido`: los escuadrones ya no quedan heridos.
 * v11: bump sin entrada aquí.
 * v12 (2026-09-27): ritmo de crecimiento y guerra de la Era I — nivel de escuadra (`MILITAR.bonusPoderPorNivelEscuadra`,
 *   `nivelMaximoEscuadra`, `experienciaParaSubirEscuadra` en lugar de `bonusExperienciaPorPunto`), `ventajaDefensor`,
 *   `OCUPACION.proteccionMinutos`, `BATALLA.capacidad.asedio` a 5, fuera `CIUDADANIA.casas*`,
 *   `MANTENIMIENTO.edificiosReferencia` en lugar de `poblacionReferencia`, y los cambios de
 *   población, experiencia de Facción y ascenso de la rama `ritmo-crecimiento`.
 */
export const BALANCE_VERSION = 12;

/**
 * Modelo temporal (Fase D, Docs/Arquitectura/10_Modelo_Temporal.md). **Decisión del usuario (2026-08-29):
 * mundo = tiempo real, 1 tick = 1 minuto real, sin aceleración** (modelo OGame). No es un placeholder — es el
 * modelo. Lo que SÍ queda pendiente es el rebalanceo de los VALORES de las demás tablas para ese ritmo (una
 * pasada dedicada sobre el laboratorio batch, doc 10 §8), no el modelo en sí.
 *
 * El `instante` de mundo es `epocaInicial + tick × duracionTickMs`, DERIVADO del tick — nunca se lee el reloj
 * de pared ni se guarda en el estado (ver `instanteDeTick`, `session/estado.ts`). Un snapshot antiguo se
 * reconstruye del `tick` sin datos nuevos.
 *
 * `duracionTickMs` y `INTERVALO_TICK_MS` (el intervalo del reloj de mundo, `server/index.ts`) son 60 000 por
 * defecto —el mismo número, porque no hay aceleración—, pero se mantienen separados: uno calcula el `instante`,
 * el otro alimenta el `setInterval` de `RunnerDePartida.iniciarRelojDeMundo`, y así un "servidor rápido" no
 * exige rediseño.
 */
export const SIMULACION = {
  /** Fecha de mundo de la que arranca toda partida (ISO 8601). Neutral a propósito; la interfaz muestra
   * "día N desde la fundación" restando esto. */
  epocaInicial: '2026-01-01T00:00:00.000Z',
  /** Tiempo de mundo que representa un tick, en ms. 60 000 = 1 minuto (= 1:1 con el tiempo real). Es el
   * único sitio del repo que "sabe" cuánto dura un tick: el resto de plazos se declaran ya en minutos
   * (`*Minutos`) y el motor los convierte con `minutos()` de `domain/tiempo.ts` (D6, doc 10 §6). */
  duracionTickMs: 60_000,
} as const;

/**
 * Capacidad de Leñeras por bosque según su tamaño (a petición del usuario): un bosque grande admite más de
 * una Leñera trabajando en él a la vez, mín 1 / máx 3 — ver `capacidadLenerasBosque` en engine/construction.ts.
 * Umbrales repartidos en tercios del rango de radio con que se generan los bosques (30-80, ver
 * `BOSQUE` en worldgen/config.ts): <45 -> 1, 45-64 -> 2, >=65 -> 3.
 * Es balance de EXPLOTACIÓN, no de generación: cambiarlo no altera el mapa, solo cuántas Leñeras caben.
 */
export const LENERA_POR_BOSQUE = {
  umbral2: 45,
  umbral3: 65,
};

// Zona de influencia: radio inicial al fundar, crecimiento por edificio completado y tope máximo (escalará con nivel).
export const ZONA_INFLUENCIA = {
  // Rediseño de progreso (Fase 0): radio inicial sube de 15 a 30, y el techo de crecimiento deja de ser un
  // único radioMaximo fijo — ahora escala con el nivel del asentamiento (ver NIVEL_ASENTAMIENTO, Doc 1.2/4.5).
  radioInicial: 30,
  // Rediseño a petición del usuario (Doc 1.2): el crecimiento deja de ser puramente temporal (antes: fijo por
  // tick, sin relación con nada más) y pasa a estar ligado a CONSTRUCCIÓN ACTIVA — cada edificio nuevo
  // completado (auto-construcción o manual) empuja el radio hacia su techo. Ver `avanzarConstruccion`
  // (engine/construction.ts), donde se aplica al completarse cada edificio. PLACEHOLDER sin calibrar todavía.
  crecimientoPorEdificioCompletado: 5,
  // Niveles 4/5 (Doc Fase_0_6): placeholder continuando el incremento fijo de +30 que ya usaban los 3
  // niveles anteriores — pendiente de calibrar por simulación, igual que el resto de cifras de esta fase.
  radioMaximoPorNivel: { 1: 60, 2: 90, 3: 120, 4: 150, 5: 180 } as Record<number, number>,
  segmentosPoligono: 48, // resolución del círculo aproximado como polígono
  // Resolución (unidades de mapa) con la que se fusionan las zonas de una misma facción en una sola silueta
  // para dibujarlas — ver `computeZonasFusionadasPorFaccion` y `unirFormas`. Solo afecta al DIBUJO, ningún
  // cálculo de juego lo lee. 5 sobre un mapa de 2000 deja el error del contorno por debajo del grosor del
  // trazo con el que se pinta la frontera.
  //
  // El coste va con 1/paso², así que es la palanca si alguna vez molesta. MEDIDO en navegador, por recálculo
  // (que solo ocurre cuando cambia la posición, el radio o la facción de algún asentamiento — el resultado
  // se cachea, ver `GameStore.getZonasFusionadas`): 3 asentamientos a radio 60 ≈ 2 ms, 6 a radio 90 ≈ 8 ms,
  // 8 a radio 120 ≈ 17 ms, y el tope teórico de 12 asentamientos todos al radio máximo de nivel 5 ≈ 50 ms.
  pasoFusionContorno: 5,
};

export const FUNDACION = {
  maxJugadoresFundacionGrupal: 5,
  /**
   * Cuantos ciudadanos hacen falta para fundar una Faccion NUEVA. **La palanca contra la ola de
   * fundaciones** (`Consideraciones/Entrada_Al_Mundo_Definicion.md`): sin ella, 800 jugadores que entran son
   * 800 Facciones y 800 asentamientos en el primer minuto, porque `crearFaccion` no pide nada.
   *
   * La fundacion grupal existia desde el principio pero PERMITIA compartir sin obligar a nada; esto es lo
   * que la convierte en el freno que pretendia ser.
   *
   * **1 durante las primeras pruebas** (decision del usuario, 2026-09-07): con cinco testers el freno
   * estorba. Se sube cuando la poblacion lo pida — es una constante justamente para que eso no sea un cambio
   * de codigo.
   */
  minFundadoresParaFaccionNueva: 1,
  /**
   * Si para fundar hay que haber sido ciudadano de alguna Faccion antes. Convierte fundar en un **cisma**
   * —gente que ya vivia en algun sitio y se marcha— en vez de en el primer acto del juego, y fuerza a pasar
   * por la fase de huesped.
   *
   * **`false` durante las primeras pruebas** (decision del usuario, 2026-09-07), por lo mismo que la de
   * arriba.
   */
  exigeCiudadaniaPrevia: false,
  /**
   * A que distancia MINIMA de una plaza existente aparece un jugador nuevo (Doc 1.3). **200**, poco mas de un
   * radio de provincia: apareces en campo abierto, no dentro de la muralla de un desconocido, pero tampoco en
   * la otra punta del mundo — con la vista de un hombre solo (80) tienes que andar un rato para encontrar a
   * alguien, que es exactamente lo que hace que explorar valga la pena.
   */
  distanciaMinimaAparicion: 200,
  /**
   * Cuantos puntos se prueban antes de rendirse al buscar sitio donde aparecer. Con un mundo lleno puede no
   * haber ninguno que cumpla la distancia minima; entonces se afloja esa exigencia antes que dejar a un
   * jugador sin poder entrar.
   */
  intentosDeAparicion: 200,
  // La caravana de fundación (Doc 1.3) trae una reserva inicial generosa: además de materiales básicos,
  // trigo suficiente para no entrar en déficit de comida desde el primer tick y oro para las primeras
  // operaciones de mercado/trueque.
  materialesIniciales: { madera: 50, piedra: 20, trigo: 100, oro: 100 } as Record<string, number>,
  // Edificios que nacen ya activos con el asentamiento (Doc 1.3): un Centro Urbano (marcador único, no
  // se puede construir por ningún otro medio) + una Granja + 3 Viviendas, para no depender del todo de
  // la auto-construcción en los primeros ticks.
  viviendasIniciales: 3,
  // Caravana de Fundación (Doc 1.8): coste extra sobre materialesIniciales + costo de los edificios de
  // arranque, representando fabricar la caravana en sí — placeholder sin calibrar por simulación todavía.
  costoMaderaExtraCaravana: 50,
};

// --- Sprint 2: Población, construcción automática y almacenamiento (Doc 4) ---

export const POBLACION = {
  // Tasas subidas (rebalance post-Fase 0): con 0.05/0.03 la población tardaba muchísimos más ticks en
  // duplicarse que un asentamiento en subir de nivel o que una caravana en cruzar el mapa — la población
  // nunca alcanzaba a "sostener" el nivel del asentamiento.
  pesants: { inicial: 20, tasaCrecimientoBase: 0.12 },
  // Rediseño a petición del usuario: Artesanos crece con la MISMA fórmula proporcional que Pesants (ver
  // `crecerPoblacion` en engine/population.ts), solo que a esta tasa más lenta — ya no hay un tope numérico
  // ligado a `trabajadoresRequeridos` de los edificios de transformación (antes `capacidadArtesanos`,
  // retirada); esos edificios ahora solo GATILLAN la primera aparición, no limitan cuánto puede crecer después.
  // Subida de 0.05 a 0.1 (pruebas del usuario) — con 0.05 los Artesanos tardaban demasiado en poblar la
  // capacidad de Vivienda que ya tenían disponible frente al resto de ritmos del juego.
  // ×1,5 el 2026-09-27 (decisión del usuario): la tropa de línea (escalón 3) se recluta con artesanos (BA-006), así
  // que tienen que aparecer más deprisa.
  artesanos: { tasaCrecimientoBase: 0.15 },
  // "Cantidad mínima de ciudadanos" sin número fijado en el diseño (ver Preguntas_Abiertas) — placeholder.
  // Rediseño de progreso (Fase 0): además de este mínimo, ahora también requiere Palacio construido
  // (Doc 4.2.1 — "desbloquea la aparición de la población noble"), ver engine/population.ts.
  nobleza: { minCiudadanos: 3, tasaCrecimientoBase: 0.01 },
  consumoComidaPorHabitante: 0.1, // trigo/minuto por habitante (pesants+artesanos+nobleza)
  /**
   * Hambruna (a petición del usuario): efecto negativo de no poder mantener a la población con trigo — espejo
   * deliberado de `MILITAR.regeneracionMoralPorMinuto`/`degradacionMoralSinRacion`/`desercionFraccionPorMinutoSinMoral`
   * (mismo diseño ya validado para tropas, ver engine/tropas.ts). El medidor de nutrición sube/baja con la
   * fracción de consumo cubierta cada minuto (`avanzarNutricionPoblacion`, engine/population.ts); con estos
   * valores, trigo en 0 sostenido colapsa el medidor en 100/20 = 5 ticks, igual que la moral de tropas.
   * PLACEHOLDER sin calibrar por simulación todavía, igual que el resto de esta fase.
   */
  hambre: {
    nutricionInicial: 100,
    regeneracionPorMinuto: 5,
    degradacionSinComida: 20,
    // Suelo del factor de crecimiento cuando la nutrición está en 0 (Doc 4.1: antes era un booleano
    // trigo>0?1:0.2 — ahora escala linealmente entre este suelo y 1 según `nutricionPoblacion`/100).
    factorCrecimientoMinimo: 0.2,
    // Nutrición <= este umbral empieza a costar población real, no solo crecimiento.
    umbralMuertePorHambre: 0,
    // Fracción de pesants+artesanos perdida por minuto mientras la nutrición sigue en el umbral — mismo valor que
    // la deserción de tropas sin moral, por coherencia entre ambos sistemas.
    fraccionMuertePorMinutoHambre: 0.05,
    // La nobleza ya no está protegida (2026-09-27, decisión del usuario), pero es la más resistente: 1 % por minuto.
    fraccionMuertePorMinutoHambreNobleza: 0.01,
  },
};

/**
 * Recaudación de oro por población (Doc 4.1, bloque "economía del oro" —
 * `Consideraciones/Economia_Del_Oro_Definicion.md`): cada asentamiento genera oro cada minuto según cuánta
 * población tiene y de qué clase. `recaudacionOro` (engine/population.ts) es el espejo exacto de
 * `consumoComidaPoblacion`, signo opuesto: `Σ(habitantes_clase × tasa_clase)`, sumado al almacén.
 *
 * Nobleza > Artesanos > Pesants por cabeza (base imponible por riqueza). NO escala por distancia a la capital
 * (ese eje ya es el "impuesto de cohesión" del lado del coste, `MANTENIMIENTO.escalaDistancia`) ni por
 * `nivelActual`. Modulable por la política "Presión Fiscal" del Tesorero (`factorRecaudacion`).
 *
 * Todo PLACEHOLDER, a calibrar en la campaña conjunta del bloque (junto con el oro de reclutamiento, el buey y
 * `MANTENIMIENTO.nivelParaOro`). Las tasas son oro/minuto por habitante de cada clase.
 */
export const IMPUESTOS = {
  // A la mitad de la primera tentativa (0.008/0.03/0.12) — la medición de Pasos 1-5 dejó `oroMedio` en ×4 la
  // línea base con expansión desbocada. Calibración en curso (Paso 6). PLACEHOLDER.
  tasaPesants: 0.004,
  tasaArtesanos: 0.015,
  tasaNobleza: 0.06,
};

/**
 * Receta de crafting de un edificio de transformación (Doc 4.2.1, rediseño de progreso Fase 0): `produccionBase`
 * es la tasa objetivo por minuto (mismo criterio que produccionBaseTrigo/produccionBasePiedra etc.);
 * `consumePorUnidad` es cuánto de cada insumo hace falta por cada unidad de output, derivado de la proporción
 * de la receta original documentada (ej. "8 Lingote de Cobre + 2 Lingote de Estaño -> 5 Lingote de Bronce" con
 * produccionBase 1 LB/minuto => consumePorUnidad { lingoteCobre: 1.6, lingoteEstano: 0.4 }). La producción real de
 * cada tick se limita por `min(produccionBase * ratioManoObraArtesanos, insumo_disponible / consumePorUnidad)`,
 * mismo criterio que ya usan los extractores minerales contra `nodo.cantidad` (ver engine/construction.ts).
 */
export interface RecetaProduccion {
  produce: string;
  produccionBase: number;
  consumePorUnidad: Partial<Record<string, number>>;
  /** La tecnología que la Facción tiene que haber adoptado para producirla (Doc 4.2.1, Doc 6). */
  requiereTecnologia?: TecnologiaId;
  /** Un edificio que tiene que haber en la plaza, con este nivel interno mínimo (el Arma de Hierro de Calidad pide la
   * Fundición 3, Doc 4.2.1). */
  requiereEdificio?: { tipo: EdificioTipo; nivel: number };
}

/** Un nivel interno de un edificio de transformación con tiers (Fundición/Curtiduría/Armería/Carpintería/
 * Barracón/Galería de tiro, Doc 4.2.1). El nivel 1 no lleva `costoMejora`/gates (ya se pagaron al construir). */
export interface NivelEdificioTransformacion {
  trabajadoresRequeridos: number;
  recetas: RecetaProduccion[];
  costoMejora?: Partial<Record<string, number>>;
  requisitoNivelAsentamiento?: number;
  requiereEdificio?: string;
  requiereEdificioNivel?: number;
  /** La tecnología que la Facción tiene que haber adoptado para mejorar a este nivel (Doc 6). */
  requiereTecnologia?: TecnologiaId;
  /** Duración de la mejora a este nivel, si no sigue la regla general (`MEJORA_EDIFICIO`). */
  obraMinutos?: number;
  /** Solo Palacio: cupo de nobles de este nivel (Doc 4.2.1). */
  capacidadNobles?: number;
  /** Solo Mercado (ampliación de comercio, a petición del usuario): cupo de caravanas propias que administra
   * este nivel — ver `cupoCaravanas`, engine/asentamientoQuery.ts. Mercado no fabrica nada (recetas: []),
   * así que este campo reemplaza al de producción como "qué desbloquea" cada nivel para ese edificio. */
  cupoCaravanas?: number;
  /** Solo Granja (trazado urbano dinámico, a petición del usuario): rinde trigo en vez de ejecutar recetas
   * (recetas: []), así que su producción escala por nivel aquí — ver `produccionTrigoDeGranja`. */
  produccionBaseTrigo?: number;
  /** Solo Granero (a petición del usuario, 2026-09-04): capacidad de TRIGO que aporta este nivel — total, no
   * incremental. Mismo patrón que `cupoCaravanas` en Mercado: un edificio sin recetas cuyo nivel interno no
   * cambia lo que produce sino lo que habilita. Se aplica como DELTA contra el nivel anterior al mejorar,
   * ver `avanzarMejoras` (engine/construction.ts). */
  capacidadTrigo?: number;
  /** Solo Granja: su huella en la rejilla CRECE con el nivel interno (2x2 → 6x6), a diferencia del resto de
   * tipos, cuyo tamaño es fijo (`EDIFICIO_TAMANO`). Ver `tamanoEdificio`, engine/trazado.ts. */
  tamano?: { ancho: number; alto: number };
}

/**
 * Arma simple de madera endurecida — escalón de entrada de la Armería (verificado por simulación, ver
 * `Diario_Simulaciones_Batch_500_Transformacion.md`): antes, TODAS las tropas de nivel 1 exigían la cadena
 * metalúrgica o la del cuero entera (Mina de Cobre → Fundición → Armería, o Corral → Curtiduría → Armería),
 * y como solo ~6% de los asentamientos nace con un nodo de cobre/livestock dentro de su zona, la primera
 * tropa llegaba en el tick ~196 y solo al 5.7% de los asentamientos. Esta receta rompe esa dependencia:
 * madera es el único recurso al que casi todo asentamiento viable tiene acceso.
 *
 * Va la ÚLTIMA en las tres listas de recetas de Armería a propósito: las recetas se ejecutan en orden sobre
 * el mismo almacén, así que armaCobre/armaBronce (que también consumen madera) se sirven primero y esta
 * absorbe el sobrante — el arma de madera es el fallback, no la que compite por la materia prima buena.
 * Se repite en los 3 niveles porque las recetas se REEMPLAZAN al mejorar, no se acumulan: sin esto, mejorar
 * la Armería quitaría la capacidad de armar milicia.
 *
 * Cifras deliberadamente modestas (2/minuto a cambio de 4 madera/minuto): `avanzarRecetas` NO respeta la reserva
 * dinámica de Mantenimiento (a diferencia de la construcción, ver `puedeIniciarConstruccion`) — vacía el
 * stock hasta donde llegue. Una tasa más alta convertiría la Armería en una vía de colapso por falta de
 * madera para Mantenimiento.
 */
const RECETA_ARMA_MADERA = { produce: 'armaMadera', produccionBase: 2, consumePorUnidad: { madera: 2 } };

/**
 * Tiempos de obra (2026-09-26, decisión del usuario — `Consideraciones/Ritmo_Crecimiento_Asentamientos.md` §11):
 * pasaron de minutos a horas para encajar con las Eras (D44) y con qué nivel le toca a cada Era (D49). Por grupos:
 * supervivencia 1-2 h (Leñera, Granja), base de nivel 1 4-8 h, transformación y militar 12 h, Gran Fundición 2
 * días, Palacio 5 días, Maravilla 4 semanas. Las MEJORAS de nivel interno tardan la obra base multiplicada por
 * `MEJORA_EDIFICIO.multiplicadorPorNivel` elevado a (nivel − 1): el doble para el 2, cuatro veces para el 3, ocho
 * para el 4. PLACEHOLDER, a calibrar con el batch.
 */
export const EDIFICIO_CATALOGO = {
  // Único edificio que NO pasa por la cola de construcción (ni automática ni manual, Doc 1.3): nace
  // ya activo al fundar. costo/tiempoConstruccionMinutos quedan en 0 solo por consistencia de forma con
  // el resto del catálogo — nunca se leen, porque construirlo por otra vía no es posible.
  centroUrbano: { costo: {}, tiempoConstruccionMinutos: 0 },
  // Cupos SEPARADOS por clase (a petición del usuario, ver Correcciones): antes un único pool compartido
  // entre Pesants y Artesanos hacía que Pesants (crece ~2.4x más rápido) acaparara todo el cupo y dejara a
  // Artesanos varado — cada Vivienda ahora aporta 15 espacios de Pesants Y, por separado, 5 de Artesanos.
  // `capacidadArtesanos` ×2 el 2026-09-27 (decisión del usuario), con el mismo motivo que su tasa de crecimiento.
  vivienda: { costo: { madera: 10 }, tiempoConstruccionMinutos: 240, capacidadPesants: 15, capacidadArtesanos: 10 },
  /**
   * Granja: 4 niveles internos (a petición del usuario, trazado urbano dinámico). El costo en materiales
   * DUPLICA en cada salto, tomando como base su `costo` de construcción (madera 30 → 60, 120, 240).
   *
   * El rinde de trigo sube MUCHO más despacio que el costo: los multiplicadores son sobre el nivel 1, no
   * acumulativos — ×1 / ×1.5 / ×2 / ×3. Una granja de nivel 4 cuesta 8 veces la de nivel 1 y rinde 3.
   *
   * **La base se ha DOBLADO dos veces** (15 → 30 el 2026-09-02, 30 → 60 el 2026-09-04), las dos a petición del
   * usuario y las dos por la misma razón: el trigo era el cuello de botella de todo lo demás. La primera vez
   * fue por el nivel 3 (`Mecanicas` §9.4: un asentamiento nivel 1 a tope come 30/minuto y una Granja rendía
   * 15, o sea que nacía en déficit estructural). La segunda, porque medir el Paso 6 del movimiento de
   * ejércitos demostró que 26 de 28 ciudades no podían meter NI UN GRANO en el carro de un ejército sin
   * bajar de su reserva de comida (`Consideraciones/Movimiento_Ejercitos_Definicion.md` §10).
   *
   * El tamaño por nivel vive aquí mismo (`tamano`) y no en `EDIFICIO_TAMANO`, porque es el único tipo cuya
   * huella cambia con el nivel. `trabajadoresRequeridos` se repite igual en los 4 (el valor plano que Granja
   * ya tenía): sube el rinde por granja, no la mano de obra que exige.
   */
  granja: {
    costo: { madera: 30 },
    tiempoConstruccionMinutos: 120,
    produccionBaseTrigo: 60,
    trabajadoresRequeridos: 4,
    niveles: {
      1: { trabajadoresRequeridos: 4, recetas: [], produccionBaseTrigo: 60, tamano: { ancho: 2, alto: 2 } },
      // Piedra añadida a las mejoras (Doc Fase_0_6, a petición del usuario): antes 100% madera. Sin gate de
      // nivel de asentamiento — las 4 mejoras siguen alcanzables estando en nivel 1.
      2: { trabajadoresRequeridos: 4, recetas: [], produccionBaseTrigo: 90, tamano: { ancho: 2, alto: 3 }, costoMejora: { madera: 60, piedra: 20 } },
      3: { trabajadoresRequeridos: 4, recetas: [], produccionBaseTrigo: 120, tamano: { ancho: 4, alto: 3 }, costoMejora: { madera: 120, piedra: 40 } },
      4: { trabajadoresRequeridos: 4, recetas: [], produccionBaseTrigo: 180, tamano: { ancho: 6, alto: 6 }, costoMejora: { madera: 240, piedra: 80 } },
    } as Record<number, NivelEdificioTransformacion>,
  },
  // Piedra ×2 (5 → 10) el 2026-09-28, decisión del usuario: con el mantenimiento por edificios, una sola Cantera no
  // sostenía el nivel 2 y el 95 % de las plazas no pasaba la prueba de solvencia para subir.
  cantera: { costo: { madera: 20 }, tiempoConstruccionMinutos: 240, produccionBasePiedra: 10, trabajadoresRequeridos: 4 },
  lenera: { costo: { madera: 10 }, tiempoConstruccionMinutos: 60, produccionBaseMadera: 5, trabajadoresRequeridos: 4 },
  // Sin piedra en la construcción BASE (Doc Fase_0_6, a petición del usuario): nivel 1 completo se paga solo
  // en madera — la piedra recién se introduce en nivel 2 (ver EDIFICIO_CATALOGO.fundicion/curtiduria/armeria).
  almacen: { costo: { madera: 50 }, tiempoConstruccionMinutos: 360, capacidadPorRecursoAdicional: 300 },
  /**
   * Granero (a petición del usuario, 2026-09-04): almacén ESPECIALIZADO en grano. A diferencia del Almacén
   * —que amplía la capacidad de todos los recursos por igual, 300 cada uno— este solo guarda trigo, y a
   * cambio guarda mucho más.
   *
   * **Uno por asentamiento** (`EDIFICIOS_UNICOS`): crece por NIVEL INTERNO, no por número. Los cuatro niveles
   * escalan la capacidad con la misma forma que el rinde de la Granja —×1 / ×1.5 / ×2 / ×3 sobre el nivel 1,
   * no acumulativa— así que el techo va de 2.000 a 6.000. Para comparar: la reserva de comida de una ciudad
   * nivel 1 a tope ronda los 330 y el carro de un ejército son 500, o sea que un Granero de nivel 4 permite
   * acumular una docena de campañas.
   *
   * Los gates de mejora los fijó el usuario: subir al nivel 2 exige asentamiento nivel 2, y llegar al 4 exige
   * nivel 3. El gate del 2 al 3 se declara explícitamente aunque parezca redundante (no se puede tener un
   * Granero 2 sin haber sido nivel 2): un asentamiento DEGRADADO a nivel 1 sí existiría en ese estado, y sin
   * el gate podría seguir ampliando granero mientras se cae a pedazos.
   *
   * Costos siguiendo el estándar del resto del catálogo: base solo en madera (Doc Fase_0_6 — la piedra se
   * introduce a partir del nivel 2) y mejoras que DUPLICAN sobre la base, igual que Granja.
   */
  granero: {
    costo: { madera: 50 },
    tiempoConstruccionMinutos: 360,
    niveles: {
      1: { trabajadoresRequeridos: 0, recetas: [], capacidadTrigo: 2000 },
      2: { trabajadoresRequeridos: 0, recetas: [], capacidadTrigo: 3000, requisitoNivelAsentamiento: 2, costoMejora: { madera: 100, piedra: 30 } },
      3: { trabajadoresRequeridos: 0, recetas: [], capacidadTrigo: 4000, requisitoNivelAsentamiento: 2, costoMejora: { madera: 200, piedra: 60 } },
      4: { trabajadoresRequeridos: 0, recetas: [], capacidadTrigo: 6000, requisitoNivelAsentamiento: 3, costoMejora: { madera: 400, piedra: 120 } },
    } as Record<number, NivelEdificioTransformacion>,
  },
  // Oro: metal precioso en bruto, origen en minas igual que cualquier otro recurso (Doc 3.1). produccionBaseOro
  // recalibrado (verificación batch del overhaul de auto-construcción): a 2/minuto, una sola Mina (2) ya no
  // alcanzaba a cubrir el costo de Mantenimiento de oro a nivel 3 (`MANTENIMIENTO.oroBase` × escala ≈
  // 2.6-5.2/minuto) ni siquiera con el nodo recién descubierto — a diferencia de piedra/Cantera, este era un
  // problema real de TASA, no solo de tamaño de nodo. Sube a 4 para dar el mismo margen que Cantera tiene
  // sobre su propio costo de Mantenimiento (~1.4-1.5×) en la distancia base.
  // Sin piedra en la construcción BASE (Doc Fase_0_6): las 3 minas son extractores de nivel 1, tienen que ser
  // alcanzables sin piedra — es justo lo que hace falta construir para cumplir el gate de subir a nivel 2
  // (NIVEL_ASENTAMIENTO.requisitos[2], ≥3 edificios de extracción).
  mina: { costo: { madera: 40 }, tiempoConstruccionMinutos: 480, produccionBaseOro: 4, trabajadoresRequeridos: 6 },
  // Cobre (Doc 1.1/5.7): "relativamente abundante" — igual patrón que cantera/mina pero sobre nodos de cobre.
  minaCobre: { costo: { madera: 30 }, tiempoConstruccionMinutos: 360, produccionBaseCobre: 5, trabajadoresRequeridos: 8 },
  // Estaño (Doc 1.1/5.7): raro y concentrado (menos nodos que cobre/oro, ver RECURSO_RAREZA.raro) — costo más
  // alto y producción base más baja que el resto de minas, coherente con ser el cuello de botella del bronce.
  // produccionBaseEstano subido de 1.5 a 3 (pruebas del usuario) — el estaño era el cuello de botella más
  // duro de la cadena de bronce, más de lo que el diseño original pretendía.
  minaEstano: { costo: { madera: 50 }, tiempoConstruccionMinutos: 480, produccionBaseEstano: 3, trabajadoresRequeridos: 8 },
  // Hierro (Doc 1.4/4.2.1, pide `forja_hierro_temprana`): mineral abundante, mismo patrón y cifras que la de cobre.
  minaHierro: { costo: { madera: 30 }, tiempoConstruccionMinutos: 360, produccionBaseHierro: 5, trabajadoresRequeridos: 8, requiereTecnologia: 'forja_hierro_temprana' },
  // Corral (Doc 4.2.1, rediseño de progreso Fase 0): extractor de livestock, mismo patrón que cantera/minas —
  // liga a un nodo finito de livestock (Doc 1.4), con reemplazo automático al agotarse (ver EXTRACCION_MAXIMOS).
  corral: { costo: { madera: 30 }, tiempoConstruccionMinutos: 240, produccionBaseLivestock: 3, trabajadoresRequeridos: 4 },
  // "Único edificio de tier élite, exclusivo de asentamientos/Facciones de mayor nivel" — gate por nivel de Facción.
  // Se mantiene sin cambios (Doc 4.2, rediseño de progreso): queda para iteraciones posteriores la integración
  // con la nueva Fundición.
  granFundicion: { costo: { madera: 150, piedra: 100, oro: 50 }, tiempoConstruccionMinutos: 2880, nivelFaccionMinimo: 3 },

  // --- Edificios de transformación (Doc 4.2.1, rediseño de progreso Fase 0): auto-construcción (sin gate de
  // nivel para la construcción BASE — solo las mejoras de nivel interno lo exigen), disparan Artesanos (Doc
  // 4.1) y cuentan para los gates de nivel de asentamiento (ver NIVEL_ASENTAMIENTO). En Fase 0 no exigen
  // "Planos"/Aedas (Doc 6.5). Fundición reemplaza y amplía la Fundición manual anterior (antes sin producción). ---

  fundicion: {
    costo: { madera: 80, piedra: 40 },
    tiempoConstruccionMinutos: 720,
    // Doc Fase_0_6 (a petición del usuario): construcción BASE gateada a nivel 2 — antes era construible
    // desde nivel 1. Con esto la responsabilidad de "producir transformación" queda exclusivamente en manos
    // de los edificios de nivel 2.
    requisitoNivelAsentamientoConstruccion: 2,
    niveles: {
      1: {
        trabajadoresRequeridos: 4,
        recetas: [{ produce: 'lingoteCobre', produccionBase: 5, consumePorUnidad: { cobre: 2 } }],
      },
      2: {
        requisitoNivelAsentamiento: 2,
        costoMejora: { madera: 150, piedra: 100 },
        trabajadoresRequeridos: 8,
        recetas: [
          { produce: 'lingoteCobre', produccionBase: 5, consumePorUnidad: { cobre: 2 } },
          { produce: 'lingoteEstano', produccionBase: 3, consumePorUnidad: { estano: 5 }, requiereTecnologia: 'aleacion_bronce' },
          { produce: 'lingoteBronce', produccionBase: 1, consumePorUnidad: { lingoteCobre: 1.6, lingoteEstano: 0.4 }, requiereTecnologia: 'aleacion_bronce' },
          { produce: 'lingoteHierro', produccionBase: 5, consumePorUnidad: { hierro: 2, madera: 1 }, requiereTecnologia: 'forja_hierro_temprana' },
        ],
      },
      // Doc 4.2.1 (D11, P2): nivel 4 y `forja_hierro_estandarizada`; las mismas recetas con +50 % de lingotes. Es lo
      // que pide el Arma de Hierro de Calidad, así que la élite de hierro llega en el nivel 4.
      3: {
        requisitoNivelAsentamiento: 4,
        requiereTecnologia: 'forja_hierro_estandarizada',
        costoMejora: { madera: 300, piedra: 200 },
        trabajadoresRequeridos: 12,
        recetas: [
          { produce: 'lingoteCobre', produccionBase: 7.5, consumePorUnidad: { cobre: 2 } },
          { produce: 'lingoteEstano', produccionBase: 4.5, consumePorUnidad: { estano: 5 }, requiereTecnologia: 'aleacion_bronce' },
          { produce: 'lingoteBronce', produccionBase: 1.5, consumePorUnidad: { lingoteCobre: 1.6, lingoteEstano: 0.4 }, requiereTecnologia: 'aleacion_bronce' },
          { produce: 'lingoteHierro', produccionBase: 7.5, consumePorUnidad: { hierro: 2, madera: 1 }, requiereTecnologia: 'forja_hierro_temprana' },
        ],
      },
    } as Record<number, NivelEdificioTransformacion>,
  },

  curtiduria: {
    costo: { madera: 80, piedra: 30 },
    tiempoConstruccionMinutos: 720,
    // Doc Fase_0_6: construcción BASE gateada a nivel 2 (ver nota en `fundicion`).
    requisitoNivelAsentamientoConstruccion: 2,
    niveles: {
      1: {
        trabajadoresRequeridos: 4,
        recetas: [{ produce: 'cuero', produccionBase: 8, consumePorUnidad: { livestock: 0.5 } }],
      },
      2: {
        requisitoNivelAsentamiento: 2,
        costoMejora: { madera: 150, piedra: 100 },
        trabajadoresRequeridos: 6,
        recetas: [
          { produce: 'cuero', produccionBase: 12, consumePorUnidad: { livestock: 1 / 3 } },
          { produce: 'cueroCurtido', produccionBase: 2, consumePorUnidad: { cuero: 3 } },
        ],
      },
      3: {
        requisitoNivelAsentamiento: 3,
        costoMejora: { madera: 450, piedra: 200 },
        trabajadoresRequeridos: 8,
        recetas: [
          { produce: 'cuero', produccionBase: 6, consumePorUnidad: { livestock: 0.25 } },
          { produce: 'cueroCurtido', produccionBase: 3, consumePorUnidad: { cuero: 3 } },
          { produce: 'cueroCalidad', produccionBase: 1, consumePorUnidad: { cueroCurtido: 2, cuero: 1 } },
        ],
      },
    } as Record<number, NivelEdificioTransformacion>,
  },

  // Nivel 1 "3 AC" (corrección aplicada durante implementación): el diseño original decía "3 LC" en la
  // producción base de nivel 1, pero Lingote de Cobre es un INSUMO de Armería (lo produce Fundición), no algo
  // que fabrique — confirmado con el usuario que era un error de tipeo por "Arma de Cobre" (AC).
  armeria: {
    costo: { madera: 80, piedra: 30 },
    tiempoConstruccionMinutos: 720,
    // Doc Fase_0_6: construcción BASE gateada a nivel 2 (ver nota en `fundicion`).
    requisitoNivelAsentamientoConstruccion: 2,
    niveles: {
      1: {
        trabajadoresRequeridos: 4,
        recetas: [
          { produce: 'armaCobre', produccionBase: 3, consumePorUnidad: { lingoteCobre: 1, madera: 1 }, requiereTecnologia: 'metalurgia_cobre' },
          { produce: 'armaduraBasica', produccionBase: 3, consumePorUnidad: { cuero: 5 } },
          RECETA_ARMA_MADERA,
        ],
      },
      // P1 (Doc 4.2.1): el arma sale de la fundición, así que la Armería 2 y la 3 piden Fundición 2.
      2: {
        requisitoNivelAsentamiento: 2,
        requiereEdificio: 'fundicion',
        requiereEdificioNivel: 2,
        costoMejora: { madera: 150, piedra: 100 },
        trabajadoresRequeridos: 8,
        recetas: [
          { produce: 'armaCobre', produccionBase: 3, consumePorUnidad: { lingoteCobre: 1, madera: 1 }, requiereTecnologia: 'metalurgia_cobre' },
          { produce: 'armaduraBasica', produccionBase: 3, consumePorUnidad: { cuero: 5 } },
          { produce: 'armaBronce', produccionBase: 2, consumePorUnidad: { lingoteBronce: 1, madera: 2 }, requiereTecnologia: 'aleacion_bronce' },
          { produce: 'armaHierro', produccionBase: 2, consumePorUnidad: { lingoteHierro: 1, madera: 2 }, requiereTecnologia: 'forja_hierro_temprana' },
          { produce: 'armaduraIntermedia', produccionBase: 2, consumePorUnidad: { lingoteCobre: 1, cueroCurtido: 5 } },
          RECETA_ARMA_MADERA,
        ],
      },
      3: {
        requisitoNivelAsentamiento: 3,
        requiereEdificio: 'fundicion',
        requiereEdificioNivel: 2,
        costoMejora: { madera: 450, piedra: 200 },
        trabajadoresRequeridos: 20,
        recetas: [
          { produce: 'armaCobre', produccionBase: 3, consumePorUnidad: { lingoteCobre: 1, madera: 1 }, requiereTecnologia: 'metalurgia_cobre' },
          { produce: 'armaduraBasica', produccionBase: 3, consumePorUnidad: { cuero: 5 } },
          { produce: 'armaBronce', produccionBase: 2, consumePorUnidad: { lingoteBronce: 1, madera: 2 }, requiereTecnologia: 'aleacion_bronce' },
          { produce: 'armaHierro', produccionBase: 2, consumePorUnidad: { lingoteHierro: 1, madera: 2 }, requiereTecnologia: 'forja_hierro_temprana' },
          { produce: 'armaduraIntermedia', produccionBase: 2, consumePorUnidad: { lingoteCobre: 1, cueroCurtido: 5 } },
          { produce: 'armaBronceCalidad', produccionBase: 1, consumePorUnidad: { lingoteBronce: 5, madera: 5 }, requiereTecnologia: 'bronce_calidad_militar' },
          { produce: 'armaduraBronce', produccionBase: 1, consumePorUnidad: { lingoteBronce: 1, cueroCalidad: 5 }, requiereTecnologia: 'bronce_calidad_militar' },
          {
            produce: 'armaHierroCalidad',
            produccionBase: 1,
            consumePorUnidad: { lingoteHierro: 2, madera: 3 },
            requiereTecnologia: 'forja_hierro_estandarizada',
            requiereEdificio: { tipo: 'fundicion', nivel: 3 },
          },
          { produce: 'armaduraBronceCalidad', produccionBase: 1, consumePorUnidad: { lingoteBronce: 2, cueroCalidad: 5 }, requiereTecnologia: 'bronce_laminado' },
          RECETA_ARMA_MADERA,
        ],
      },
    } as Record<number, NivelEdificioTransformacion>,
  },

  // Sin cifras en el diseño original más allá del gate de nivel — costo/tiempo/mejora son PLACEHOLDER (ver
  // Preguntas_Abiertas.md). A diferencia del resto de edificios de transformación, la construcción BASE sí
  // exige nivel de asentamiento (requisitoNivelAsentamientoConstruccion). Sin recetas: arietes/torres de
  // asedio no se modelan en Fase 0 (combate resuelto como cálculo/log, Doc 5.10). Gate subido de nivel 2 a
  // nivel 3 (Doc Fase_0_6, a petición del usuario): habilita Armería/Barracón/Galería de tiro nivel 2 igual
  // que antes, pero ahora llega un escalón más tarde — se desbloquea junto con Murallas en nivel 3.
  // P2 (Era I, Doc 4.2.1): se construye desde el nivel 2 — abre Barracón 2 y Galería 2 —, y su nivel 2 (asentamiento
  // nivel 3) abre el nivel 3 de Barracón, Galería de tiro y Caballerizas y fabrica la pieza del carro de guerra.
  carpinteria: {
    costo: { madera: 60, piedra: 20 },
    tiempoConstruccionMinutos: 720,
    requisitoNivelAsentamientoConstruccion: 2,
    niveles: {
      1: { trabajadoresRequeridos: 0, recetas: [] },
      2: {
        requisitoNivelAsentamiento: 3,
        costoMejora: { madera: 120, piedra: 60 },
        trabajadoresRequeridos: 4,
        recetas: [
          { produce: 'carroGuerra', produccionBase: 1, consumePorUnidad: { madera: 20, cueroCurtido: 2, lingoteBronce: 1 }, requiereTecnologia: 'carros_guerra' },
        ],
      },
    } as Record<number, NivelEdificioTransformacion>,
  },

  // --- Edificios especiales (Doc 4.4, rediseño de progreso Fase 0 + cambio de base del control de cola, a
  // petición del usuario): NO son auto-construcción por necesidad — solo se añaden a la cola por decisión
  // MANUAL de Gobernador/Maestro de Obras (`anadirEdificioManualmente`, engine/construction.ts), igual que
  // cualquier otro edificio del catálogo (el mecanismo de política dedicada que existía antes se retiró por
  // completo). Sin recetas: el reclutamiento por equipo de Barracón/Galería de tiro queda fuera de alcance de
  // este plan (ver Doc 5.7/5.8 PENDIENTE), Palacio solo desbloquea Nobleza (engine/population.ts). ---

  barracon: {
    costo: { madera: 30 },
    tiempoConstruccionMinutos: 720,
    // Construcción BASE gateada a nivel 2 (trazado de anclas, ver Vista_Asentamiento_Trazado_Urbano.md §5.7.1):
    // el primer edificio militar arrastra tras de sí la Plaza de Armas, y al fundar (disco urbano de 5 celdas)
    // no existe ningún hueco que respete la separación mínima entre anclas — el núcleo militar nacía pegado al
    // Centro Urbano y, como ningún ancla se muda nunca, se quedaba ahí el resto de la partida. Sin gate esto
    // era alcanzable en el tick 1: 30 de madera contra los 50 que entrega la caravana de fundación.
    // No le quita nada al jugador: nivel 2 ya era el suelo REAL de facto, porque las tres tropas de
    // `nivelRequerido: 1` piden armaMadera/armaCobre/armaduraBasica y las tres las fabrica solo la Armería,
    // que ya exigía nivel 2 (ver `reclutarTropa`, engine/tropas.ts — no comprueba nivel de asentamiento).
    requisitoNivelAsentamientoConstruccion: 2,
    niveles: {
      1: { trabajadoresRequeridos: 0, recetas: [] },
      2: {
        requisitoNivelAsentamiento: 2,
        requiereEdificio: 'carpinteria',
        costoMejora: { madera: 100, piedra: 60 },
        trabajadoresRequeridos: 0,
        recetas: [],
      },
      // P2 (Doc 4.2.1): la infantería pesada va en la meseta de nivel 3 de la Era II, no tras el Palacio.
      3: {
        requisitoNivelAsentamiento: 3,
        requiereEdificio: 'carpinteria',
        requiereEdificioNivel: 2,
        costoMejora: { madera: 300, piedra: 200 },
        trabajadoresRequeridos: 0,
        recetas: [],
      },
    } as Record<number, NivelEdificioTransformacion>,
  },

  // Gate de nivel 3 INTENCIONALMENTE distinto a Armería/Barracón (pide Carpintería nivel 2, no Palacio) —
  // Galería de tiro sigue su propio camino de progresión, confirmado con el usuario que no se uniforma.
  galeriaDeTiro: {
    costo: { madera: 50 },
    tiempoConstruccionMinutos: 720,
    // Mismo gate y mismo motivo que Barracón (ver arriba): es el otro tipo capaz de abrir el grupo militar y
    // arrastrar la Plaza de Armas consigo.
    requisitoNivelAsentamientoConstruccion: 2,
    niveles: {
      1: { trabajadoresRequeridos: 0, recetas: [] },
      2: {
        requisitoNivelAsentamiento: 2,
        requiereEdificio: 'carpinteria',
        costoMejora: { madera: 140, piedra: 20 },
        trabajadoresRequeridos: 0,
        recetas: [],
      },
      3: {
        requisitoNivelAsentamiento: 3,
        requiereEdificio: 'carpinteria',
        requiereEdificioNivel: 2,
        costoMejora: { madera: 400, piedra: 100 },
        trabajadoresRequeridos: 0,
        recetas: [],
      },
    } as Record<number, NivelEdificioTransformacion>,
  },

  // Caballería y carros (Doc 4.2.1, pide `cria_caballar`): mismos requisitos y coste que el Barracón.
  caballerizas: {
    costo: { madera: 30 },
    tiempoConstruccionMinutos: 720,
    requiereTecnologia: 'cria_caballar',
    requisitoNivelAsentamientoConstruccion: 2,
    niveles: {
      1: { trabajadoresRequeridos: 0, recetas: [] },
      2: {
        requisitoNivelAsentamiento: 2,
        requiereEdificio: 'carpinteria',
        costoMejora: { madera: 100, piedra: 60 },
        trabajadoresRequeridos: 0,
        recetas: [],
      },
      3: {
        requisitoNivelAsentamiento: 3,
        requiereEdificio: 'carpinteria',
        requiereEdificioNivel: 2,
        costoMejora: { madera: 300, piedra: 200 },
        trabajadoresRequeridos: 0,
        recetas: [],
      },
    } as Record<number, NivelEdificioTransformacion>,
  },

  // Sala del Consejo (Doc 4.2.1, pide `instituciones_civicas`): requisito del nivel 4 y +1 ranura del Gobernador.
  salaConsejo: {
    costo: { madera: 800, piedra: 1200, oro: 300 },
    requiereTecnologia: 'instituciones_civicas',
    tiempoConstruccionMinutos: 2_880,
    requisitoNivelAsentamientoConstruccion: 3,
  },

  // Único tier — desbloquea la aparición de Nobleza (además del mínimo de ciudadanos ya existente, ver
  // engine/population.ts). requisitoNivelAsentamientoConstruccion gatea la construcción BASE (no hay mejoras).
  // Gate subido de nivel 3 a nivel 4 (Doc Fase_0_6): construirlo pasa a ser requisito para subir a nivel 5.
  palacio: {
    // Doc 4.2.1 (2026-09-29): se construye desde el nivel 2 y se mejora hasta el 3, que es el requisito del nivel 5. Cada
    // nivel cuesta lo que la subida al nivel de asentamiento en que se construye, con oro: es el recurso de la nobleza.
    costo: { madera: 600, piedra: 400, oro: 100 },
    tiempoConstruccionMinutos: 4_320, // 3 días
    requisitoNivelAsentamientoConstruccion: 2,
    niveles: {
      1: { trabajadoresRequeridos: 0, recetas: [], capacidadNobles: 80 },
      2: {
        requisitoNivelAsentamiento: 3,
        costoMejora: { madera: 1200, piedra: 1000, oro: 300 },
        obraMinutos: 7_200, // 5 días
        trabajadoresRequeridos: 0,
        recetas: [],
        capacidadNobles: 240,
      },
      3: {
        requisitoNivelAsentamiento: 4,
        costoMejora: { madera: 2500, piedra: 2500, oro: 800 },
        obraMinutos: 10_080, // 1 semana
        trabajadoresRequeridos: 0,
        recetas: [],
        capacidadNobles: 400,
      },
    } as Record<number, NivelEdificioTransformacion>,
  },

  // Ampliación de comercio (a petición del usuario, Doc 3.3): adición MANUAL de Gobernador/Maestro de Obras,
  // mismo patrón que Barracón/Galería de tiro — no auto-construcción por necesidad. Sin recetas: no fabrica
  // nada, sus niveles administran `cupoCaravanas` (ver `cupoCaravanas`, engine/asentamientoQuery.ts).
  // Gatea, además de la flota, las órdenes de Mercado (`colocarOrdenMercado`, engine/market.ts) — sin Mercado
  // activo no se puede ni construir una caravana ni colocar una orden.
  mercado: {
    // Sin piedra en la construcción BASE (a petición del usuario, corrige un deadlock real detectado
    // instrumentando `engine/simulacionAutoComercio.ts`): un asentamiento sin mineral alcanzable en su zona
    // (la MAYORÍA en nivel 1, ver diagnóstico de alcance de minerales) nunca junta más de los 20 piedra de
    // reserva inicial (Doc 1.3) — con el Mercado pidiendo 40, no podía construirlo NUNCA, y sin Mercado no
    // puede tener caravana propia, y sin caravana propia no puede entregar su lado de NINGÚN trueque (cada
    // lado envía desde su propio origen, ver `asignarCaravanasATrueque`, engine/trade.ts) — ni para recibir
    // ni para dar. El comercio es precisamente el mecanismo que debería resolver la falta de piedra, así que
    // no puede depender de tenerla ya. Las MEJORAS de Mercado (nivel 2/3, más abajo) sí siguen pidiendo
    // piedra sin cambios — para entonces el asentamiento ya tuvo tiempo de conseguirla, por extracción propia
    // o por el propio comercio que el Mercado nivel 1 acaba de destrabar.
    costo: { madera: 100 },
    tiempoConstruccionMinutos: 480,
    niveles: {
      1: { trabajadoresRequeridos: 0, recetas: [], cupoCaravanas: 2 },
      2: {
        requisitoNivelAsentamiento: 2,
        costoMejora: { madera: 150, piedra: 100 },
        trabajadoresRequeridos: 0,
        recetas: [],
        cupoCaravanas: 4,
      },
      3: {
        requisitoNivelAsentamiento: 3,
        costoMejora: { madera: 450, piedra: 200 },
        trabajadoresRequeridos: 0,
        recetas: [],
        cupoCaravanas: 6,
      },
    } as Record<number, NivelEdificioTransformacion>,
  },

  // Pieza satélite de la zona de Mercado (a petición del usuario). NO se construye: la crea el motor sola,
  // gratis y ya activa, cuando el Mercado alcanza cada nivel interno (ver `crearPuestosDeMercado`,
  // engine/construction.ts). El costo y el tiempo van a cero por la misma razón que en centroUrbano — la
  // entrada existe solo porque `EDIFICIO_CATALOGO[tipo]` se indexa con `EdificioTipo` en varios sitios.
  puestoMercado: { costo: {}, tiempoConstruccionMinutos: 0 },

  // Anclas y satélites, Etapa 3 (Consideraciones/Vista_Asentamiento_Trazado_Urbano.md §5): "marcadores
  // gratis", mismo patrón que puestoMercado — nunca pasan por cola ni se añaden a mano, nacen ya activos por
  // la regla de semilla de grupo (engine/trazado.ts).
  plaza: { costo: {}, tiempoConstruccionMinutos: 0 },
  plazaDeArmas: { costo: {}, tiempoConstruccionMinutos: 0 },
  patioDeGremios: { costo: {}, tiempoConstruccionMinutos: 0 },
  // Variedad de anclas residenciales (Etapa 4, punto 4) — mismo patrón "marcador gratis" que plaza.
  pozo: { costo: {}, tiempoConstruccionMinutos: 0 },
  parque: { costo: {}, tiempoConstruccionMinutos: 0 },

  // Maravilla (Roadmap_Escalado.md Eje 4, a petición del usuario) — SOLO el edificio en esta pasada: el ciclo
  // de servidor de 12 meses que se cierra al completarla (reset + Facción ganadora persistiendo como legado
  // NPC) queda fuera de alcance, requiere infraestructura de servidor/multi-instancia que Fase 0 no tiene (ver
  // Roadmap_Escalado.md). Único edificio: nivel de asentamiento MÁXIMO (5, Doc Fase_0_6 — antes 3) requerido
  // para construirla, coste PLACEHOLDER deliberadamente extremo (varias veces el de Palacio, el más caro hasta
  // ahora) usando TODOS los recursos en bruto del catálogo — Doc dice "todos los materiales conocidos más
  // algunos exóticos"; los materiales EXÓTICOS quedan PENDIENTES (no existe todavía ningún recurso/extractor
  // exótico en el juego, añadirlos es trabajo aparte) — ver `Preguntas_Abiertas.md`. Sin recetas ni niveles:
  // es un trofeo, no un edificio productivo.
  maravilla: {
    costo: { madera: 5000, piedra: 5000, oro: 500, cobre: 300, estano: 200, livestock: 200 },
    tiempoConstruccionMinutos: 40320,
    requisitoNivelAsentamientoConstruccion: 5,
  },
} as const;

export const ALMACEN = {
  /**
   * Capacidad de almacén con la que NACE un asentamiento, por recurso.
   *
   * **200 → 400 (2026-09-04, a petición del usuario tras jugar varias partidas).** No es un ajuste de holgura:
   * 200 producía un **interbloqueo**. La reserva de construcción impide gastar por debajo de
   * `mantenimiento × RESERVA_CONSTRUCCION.horizonteMinutosMantenimiento`, que en una ciudad madura del batch
   * son ~167 de madera; el Almacén cuesta 50; y como la capacidad de madera SOLO crece construyendo Almacenes,
   * el techo de 200 dejaba `200 − 50 = 150 < 167`. O sea: para subir el techo había que construir un Almacén,
   * y para construirlo hacía falta más margen del que el techo permitía guardar. El asentamiento quedaba
   * encerrado, y no al madurar sino desde el principio — mientras es pequeño la madera se va en Granja, Leñera
   * y extractores, que van en banda de score superior y cobran primero.
   *
   * Medido en batch (300 ticks, 30 Facciones), 200 → 400: **Almacenes 0 → 114, Graneros 0 → 28** (los
   * construyen TODOS), tropas vivas +44%, y el excedente de trigo disponible para el carro de un ejército pasa
   * de **10 a 4.792** — de un 2% de un carro a nueve carros llenos. Los asentamientos que no podían aportar ni
   * un grano pasan de 28 de 30 a **ninguno**. Colapsos y niveles alcanzados no se mueven: esto no regala
   * progreso, desatasca el que ya había.
   *
   * Se midió también 500 y **no aporta nada**: exactamente los +100 de capacidad extra en el excedente y un
   * Almacén menos. La diferencia estructural está entre 200 y 400, no más arriba.
   */
  capacidadInicialPorRecurso: 400,
};

// Umbrales que disparan auto-construcción por necesidad (Doc 4.2). Placeholders razonables.
export const NECESIDADES = {
  umbralViviendaOcupada: 0.85,
  // Máximo de Granjas simultáneas `en_cola`/`en_construccion` mientras el asentamiento esté en déficit de
  // trigo (producción actual < consumo actual, ver `evaluarNecesidades`) — fuera de déficit, solo 1 a la vez
  // (mismo criterio que el resto de edificios de supervivencia).
  maximoGranjasPendientesEnDeficit: 3,
  umbralAlmacenAmpliacion: 0.9,
  /**
   * Tope de Almacenes por nivel de asentamiento (a petición del usuario): sin él, la ampliación de capacidad
   * no tenía techo — cada vez que el recurso más lleno pasaba el umbral se encolaba otro Almacén, y la
   * capacidad podía crecer sin límite mientras hubiera madera y piedra.
   *
   * Cuenta los Almacenes en CUALQUIER estado (activos, en obra y en cola) para que el tope no se pueda saltar
   * encolando varios de golpe. Aplica tanto a la auto-construcción como a la adición manual: no es un
   * heurístico que proteja a la IA de sí misma, es una regla del juego.
   */
  // Niveles 4/5 (Doc Fase_0_6): placeholder desacelerando la duplicación anterior (4→8→16) a +50%/+33% —
  // pendiente de calibrar por simulación.
  maximoAlmacenesPorNivel: { 1: 4, 2: 8, 3: 16, 4: 24, 5: 32 } as Record<number, number>,
  // Overhaul de auto-construcción (modelo "pago al encolar", ver engine/construction.ts): dos cupos con
  // significado físico separado, en vez del único `maximoEnCola` + slots reservados por categoría de antes.
  // `maximoEnCola`: cuántos proyectos pueden estar PAGADOS Y A LA ESPERA de un hueco de obra (`en_cola`).
  // `maximoEnConstruccionSimultanea`: cuántos proyectos pueden estar construyéndose activamente a la vez
  // (`en_construccion`, contando ticks) — representa cuadrillas de obra limitadas: evita que un tick con el
  // almacén lleno dispare media docena de construcciones en paralelo y vacíe todos los recursos protegidos
  // a la vez. Con el pago ya comprometido al encolar (ver `puedeIniciarConstruccion`), ya no hace falta
  // reservar slots por categoría (`slotsReservadosSupervivencia`/`slotsReservadosExtractores`, retirados):
  // el orden por score (ver `SCORE_BANDAS`) ya garantiza que supervivencia gana el reparto cuando escasea.
  maximoEnCola: 4,
  // Desde el 2026-09-26 las MEJORAS de nivel interno también ocupan una de estas cuadrillas mientras duran
  // (decisión del usuario): construir y mejorar compiten por las mismas manos.
  maximoEnConstruccionSimultanea: 2,
};

/** Duración de una mejora de nivel interno: la obra base del edificio × `multiplicadorPorNivel`^(nivel − 1). Una
 * sola regla: las mejoras tardan más que la obra, y más cuanto más alto el nivel (decisión del usuario 2026-09-26). */
export const MEJORA_EDIFICIO = {
  multiplicadorPorNivel: 2,
};

/**
 * Bandas de score para la auto-construcción (overhaul, reemplaza los buckets `categoriaPrioridad` +
 * slots reservados de antes): cada candidato elegible recibe `base` de su banda + una `urgencia` 0-100 que
 * refleja qué tan crítica es la necesidad en este momento (ver `evaluarNecesidades`, engine/construction.ts).
 * Las bandas NO se solapan (diferencia de 500+ entre la urgencia máxima de una banda y la base de la
 * siguiente) para preservar la garantía ya probada en juego: supervivencia SIEMPRE gana el reparto de
 * recursos frente a crecimiento/lujo, sin importar cuán urgente esté este último.
 */
export const SCORE_BANDAS = {
  supervivencia: 10000, // granja, lenera
  extractorBase: 5000, // cantera, minaCobre, mina, minaEstano, corral
  crecimiento: 1000, // vivienda, almacen
  // Control manual de cola (Doc 4.2, a petición del usuario): banda para edificios añadidos manualmente por
  // Gobernador/Maestro de Obras (`anadirEdificioManualmente`, engine/construction.ts) — siempre por debajo de
  // extractorBase, así una necesidad de supervivencia/extracción detectada automáticamente el próximo tick
  // nunca queda por detrás de una decisión manual (mismo invariante que ya protegía crecimiento/transformación).
  manual: 900,
  transformacion: 500, // curtiduria, armeria, fundicion, carpinteria
};

/**
 * Desempate anti-inanición para los extractores base (cantera/corral/minaCobre/mina/minaEstano): los 5
 * comparten banda de score y, apenas fundado el asentamiento, también urgencia (100, ninguno tiene fuente
 * propia todavía) — sin esto, `Array.prototype.sort` (estable) hace ganar SIEMPRE al primero del array fijo
 * `extractores` (`cantera`, ver `evaluarNecesidades`, engine/construction.ts) cuando `NECESIDADES.maximoEnCola`
 * no da cupo para todos el mismo tick, y los perdedores nunca lo compensan porque su urgencia tampoco baja de
 * 100 mientras sigan sin fuente propia (issue: `issues/extractores_minerales_nunca_se_construyen.md`).
 *
 * Cada tick que un extractor se propone como candidato válido (sitio disponible) pero NO se llega a
 * comprometer (perdió el desempate o no había fondos) suma un tick a su contador
 * `Asentamiento.extractoresTicksSinCupo`; ese contador se traduce en un bonus de score que rompe el empate a
 * su favor cada vez más fuerte, y se resetea a 0 en cuanto el tipo consigue cupo. `bonusMaximo` deja un
 * margen amplio por debajo de la siguiente banda hacia arriba (`SCORE_BANDAS.supervivencia`, 10000) para que
 * el bonus nunca pueda hacer que un extractor le gane el turno a Granja/Leñera.
 */
export const EXTRACTOR_DESEMPATE = {
  bonusPorMinutoStarved: 1,
  bonusMaximo: 400,
};

/**
 * Líneas de producción (Doc 4.2.1, a petición del usuario): la distancia dentro del asentamiento entre un
 * edificio de transformación y la fuente más cercana de cada insumo de su receta penaliza CUÁNTO produce ese
 * tick — nunca lo que consume por unidad (`consumePorUnidad` no cambia) — simulando que el insumo tarda más
 * en llegar cuanto más lejos está su origen. Ver `factorPorDistancia`/`factorLineaProduccion`,
 * engine/construction.ts. PLACEHOLDER sin cifras de diseño previas (ver Preguntas_Abiertas.md), calibrado a
 * ojo contra `ZONA_INFLUENCIA.radioMaximoPorNivel` (hasta 120).
 */
export const LINEAS_PRODUCCION = {
  // Por debajo de esta distancia, sin penalización (factor 1) — cubre colocaciones ya cercanas por azar.
  distanciaSinPenalizacion: 25,
  // A partir de esta distancia, el factor no baja más (suelo en `factorMinimo`) — más o menos el diámetro de
  // una zona de nivel 3.
  distanciaMaxima: 150,
  // Producción nunca cae por debajo de este % solo por distancia, aunque la fuente esté en el otro extremo
  // del asentamiento — evita que la penalización por sí sola pueda parar una línea de producción del todo.
  factorMinimo: 0.4,
  // Distancia asumida cuando el insumo no tiene NINGUNA fuente propia en el asentamiento (llega solo por
  // trueque/caravana) — ni cerca ni lejos, un valor medio-alto a falta de una posición real que medir.
  distanciaEstandarSinFuente: 90,
};

/**
 * Tope de extractores por tipo (cantera/mina/minaCobre/minaEstano/lenera/Corral, Doc 4.2, rediseño de
 * progreso Fase 0): antes escalaba 1:1 con el nivel del asentamiento (hasta 10, el nivelMaximo anterior); con
 * el tope de nivel bajando a 3 (ver NIVEL_ASENTAMIENTO) un máximo ligado al nivel se quedaría corto, así que
 * se desacopla a un número fijo. Calibrado por simulación (150-600 ticks): con porTipo=5 y la tasa de
 * crecimiento de Pesants ya existente (12%/minuto, sin tope salvo Vivienda), todo asentamiento colapsaba por
 * déficit de Mantenimiento hacia el tick 200-700 — el tope de extracción se quedaba corto frente a una
 * población sin límite real, algo que el sistema anterior evitaba dejando llegar hasta 10 extractores por
 * tipo. Sube a 10 (mismo techo que el nivelMaximo anterior) para no perder ese margen. Sigue siendo
 * PLACEHOLDER pendiente de más calibración (ver Preguntas_Abiertas.md).
 */
export const EXTRACCION_MAXIMOS = {
  porTipo: 10,
};

// Colocación de edificios: crecimiento concéntrico desde el centro (Doc 4.2).
/** Muestreo angular para estimar la mejor fertilidad dentro de una zona (ver `engine/zones.ts`) — sin
 * relación con la colocación de edificios (eso vive en `REJILLA_ASENTAMIENTO`). */
export const SITIO = {
  muestrasFertilidad: 40,
};

/**
 * Vista de Asentamiento (a petición del usuario): el espacio plano local es una REJILLA de celdas cuadradas,
 * no coordenadas continuas — cada edificio ocupa un RECTÁNGULO de celdas (`EDIFICIO_TAMANO`, abajo), así que
 * "nunca uno dentro de otro" se comprueba por celdas ocupadas, sin geometría de colisión. `Edificio.posicion`
 * es el CENTRO de ese rectángulo (ver `puntoDeRectangulo`, engine/trazado.ts), también en el Centro Urbano:
 * su `(0,0)` es su centro real (doc trazado §E6.20).
 *
 * Ya no hay `margenCalle`: las calles corren sobre las ARISTAS de la rejilla (no ocupan celdas) y la red nace
 * con el perímetro del Centro Urbano, que hace de calle alrededor del centro por construcción. Ver
 * `Consideraciones/Vista_Asentamiento_Trazado_Urbano.md`.
 *
 * `radioMapa` (a petición del usuario): el espacio de la Vista de Asentamiento es ESTÁTICO — no crece con la
 * zona de influencia (`asentamiento.radioPotencial`, que sí cambia con nivel/construcción, ver
 * `ZONA_INFLUENCIA`). `ui/canvas.ts` usa este radio fijo (nunca `radioPotencial`) para calcular la escala del
 * lienzo, así que el zoom no varía a medida que el asentamiento crece — solo se va llenando. Debe cubrir con
 * margen el mayor `radioPotencial` alcanzable (tope 120, nivel 3 — ver `ZONA_INFLUENCIA.radioMaximoPorNivel`),
 * para que ningún edificio colocado por `sitioEnBarrio` quede jamás fuera del área dibujada.
 *
 * **150 -> 220 (2026-09-02): se había quedado corto.** Ese "tope 120, nivel 3" dejó de ser cierto cuando
 * Fase 0.6 subió el tope de asentamiento a nivel 5, y con él `radioMaximoPorNivel` hasta 180. La ciudad real
 * de un nivel 5 llega a **205** unidades locales (`radioMaximoAfueras`: 180 + la media diagonal de una Granja
 * nivel 4), así que sus afueras habrían caído FUERA del lienzo. Latente y sin observar todavía porque hoy
 * ningún asentamiento pasa de nivel 2 en batch, pero el bug estaba puesto. 220 cubre 205 con margen.
 *
 * Es solo PRESENTACIÓN: `radioMapa` decide el zoom del lienzo de la Vista de Asentamiento, nunca dónde se
 * coloca un edificio. Cambiarlo no mueve la simulación.
 */
/**
 * ESCALA DEL MUNDO (a petición del usuario, 2026-09-02) — la equivalencia que faltaba declarar.
 *
 * El motor maneja DOS espacios y hasta ahora nadie había dicho cómo se relacionan, así que el código los
 * trataba como iguales:
 *
 *  - **Mapa general** (`Asentamiento.posicion`, `radioPotencial`, rutas, ejércitos): 2000×2000 unidades.
 *  - **Vista de Asentamiento** (`Edificio.posicion`, `REJILLA_ASENTAMIENTO`, todo `engine/trazado.ts`):
 *    espacio plano y separado, centrado en el Centro Urbano.
 *
 * Sin esta declaración la ficción no cerraba: una zona de influencia debe ser una PROVINCIA y la ciudad un
 * punto dentro de ella, pero al compartir unidades la ciudad medía ~121 y su provincia 30-180 — la urbe era
 * más grande que el territorio que controlaba, y el propio `radioMaximoAfueras` lo daba por hecho ("el campo
 * de una ciudad está FUERA de su zona de influencia").
 *
 * **Por qué 40 y no 10.** El primer intento fue 10, y no bastaba: la ciudad tiene un TAMAÑO MÍNIMO de ~121
 * unidades locales que no encoge —el suelo `max(radioUrbano, radioAfuerasMin + anchoBandaAfueras)` de
 * `radioMaximoAfueras`, que existe porque al fundar la Granja inicial no cabe más cerca— mientras que la
 * provincia SÍ arranca pequeña (`radioInicial` 30). Con 10, una aldea recién fundada ocupaba el 40% de su
 * provincia y solo llegaba a la décima en los niveles altos.
 *
 * 40 es el factor que cubre el PEOR caso: al fundar, esa ciudad mínima mide 3,0 unidades de mapa dentro de
 * una provincia de 30 — el 10,1%. De ahí para arriba el ratio solo baja (5% en nivel 1, 3% en nivel 5),
 * porque la provincia crece y el suelo de la ciudad no. Es decir: **la ciudad nunca pasa de una décima de su
 * provincia**, que era la proporción buscada.
 *
 * Se llegó aquí sin tocar el trazado urbano ni `radioInicial`, que es lo que el usuario pidió preservar: la
 * escala local es la palanca, y basta con hacerla más pequeña frente a la mundial.
 *
 * **No cambia ningún número de la simulación**: el trazado urbano sigue midiendo lo mismo en sus unidades y
 * la zona de influencia sigue midiendo lo mismo en las suyas. Lo que cambia es que ahora está DICHO, y que
 * `radioUrbanoDe` (engine/asentamientoQuery.ts) es el único punto donde los dos espacios se tocan.
 */
export const ESCALA = {
  unidadesLocalesPorUnidadMapa: 40,
};

export const REJILLA_ASENTAMIENTO = {
  /**
   * Lado de una celda en unidades locales. Es la escala de la CALLE: una calle (`TRAZADO.anchoCalle` = 1) y una
   * celda de muralla miden exactamente una celda, y ese ancho está validado para combate en Unity (BA-005). Un
   * cambio de huellas NO la toca.
   *
   * Historial: 6 → 3 en el Paso 1 de la Etapa 6 (doc trazado §E6.11), doblando a la vez todas las huellas para
   * conservar su tamaño físico. BA-005 (2026-09-13) partió las huellas por dos SIN tocar la celda: los edificios
   * miden la mitad de lado y las calles lo mismo que antes.
   */
  tamanoCelda: 3,
  radioMapa: 220,
};

/**
 * Revisión de la GEOMETRÍA del asentamiento (BA-005): huellas, `tamanoCelda` y constantes de trazado que deciden
 * dónde cae cada edificio y por dónde pasan las calles. `Edificio.posicion` se persiste, pero su huella y la red
 * se DERIVAN de estas tablas: un snapshot de otra revisión se reinterpretaría en silencio (casas desplazadas
 * media celda, calles transversales cruzando edificios). Por eso `cargarPartida` (server/persistenciaPartida.ts)
 * lo rechaza si no coincide, igual que `worldgenVersion` — sin migración.
 *
 * Distinto de `BALANCE_VERSION` (solo registro, no rechaza) y del `version` de la proyección (contador de
 * cambios de la partida). Se sube al cambiar cualquier huella, `tamanoCelda`, `FONDO_MANZANA` o una constante
 * de `TRAZADO` que mueva lo ya colocado.
 *   1 — Etapa 6: celda 3, huellas dobladas (implícita: los snapshots de entonces no la guardan).
 *   2 — BA-005: huellas a la mitad, Centro Urbano 4×4, constantes de escala de edificio a la mitad.
 */
export const LAYOUT_VERSION = 2;

/**
 * Huella de cada tipo de edificio en la rejilla local, en celdas (a petición del usuario). Un tipo ausente
 * mide `EDIFICIO_TAMANO_POR_DEFECTO` — el caso por defecto (Vivienda, Leñera, las minas). Granja NO está aquí:
 * es el único tipo cuya huella cambia con el nivel interno, y vive en
 * `EDIFICIO_CATALOGO.granja.niveles[n].tamano`.
 *
 * Se lee siempre a través de `tamanoEdificio` (engine/trazado.ts), nunca directo, para que el caso de Granja
 * quede resuelto en un solo sitio.
 *
 * **Escala (BA-005, 2026-09-13):** una celda es el ancho de una calle. La Etapa 6 (doc trazado §E6.11) dobló
 * todas estas cifras al partir la celda de 6 a 3; BA-005 las volvió a partir por dos sin tocar la celda, así que
 * coinciden otra vez con las acordadas con el usuario en la rejilla original (doc trazado §6). Única excepción
 * al "÷2": el Centro Urbano, 6×6 → **4×4** y no 3×3 (decisión del usuario): con lados impares su centro caería
 * en mitad de una celda y no podría ser el `(0,0)` del asentamiento — `celdaMinimaDeEdificio` lo desplazaría
 * media celda. Cambiar cualquier cifra de esta tabla exige subir `LAYOUT_VERSION`.
 */
export const EDIFICIO_TAMANO: Record<string, { ancho: number; alto: number }> = {
  centroUrbano: { ancho: 4, alto: 4 },
  carpinteria: { ancho: 4, alto: 2 },
  fundicion: { ancho: 2, alto: 2 },
  curtiduria: { ancho: 2, alto: 2 },
  armeria: { ancho: 2, alto: 3 },
  barracon: { ancho: 2, alto: 2 },
  galeriaDeTiro: { ancho: 2, alto: 4 },
  // Eras I-III (Doc 4.2.1): tipos nuevos, no cambian ninguna huella existente.
  caballerizas: { ancho: 3, alto: 2 },
  salaConsejo: { ancho: 3, alto: 3 },
  mercado: { ancho: 3, alto: 2 },
  palacio: { ancho: 4, alto: 4 },
  corral: { ancho: 4, alto: 3 },
  almacen: { ancho: 2, alto: 1 },
  // Granero: el doble de largo que el Almacén — guarda un solo recurso pero mucha cantidad, y que se distinga
  // a simple vista del Almacén importa en la Vista de Asentamiento.
  granero: { ancho: 4, alto: 2 },
  // Anclas y satélites, Etapa 3 (§5.1/§6).
  plaza: { ancho: 2, alto: 2 },
  plazaDeArmas: { ancho: 2, alto: 2 },
  patioDeGremios: { ancho: 2, alto: 2 },
  // Variedad de anclas residenciales (Etapa 4, punto 4): pozo el marcador mínimo, parque el único no cuadrado
  // de los tres, que ejercita la orientación intercambiable también en anclas.
  pozo: { ancho: 1, alto: 1 },
  parque: { ancho: 3, alto: 2 },
};

/**
 * Huella de un tipo AUSENTE de `EDIFICIO_TAMANO` (Vivienda, Leñera, las tres minas, Gran Fundición, Maravilla):
 * una celda.
 *
 * Constante con nombre y no literal en `tamanoEdificio` para que un cambio de escala la alcance igual que a la
 * tabla: la Vivienda es el edificio más numeroso de cualquier ciudad, y un literal repetido dentro de una
 * función es justo lo que un reescalado se salta.
 */
export const EDIFICIO_TAMANO_POR_DEFECTO = { ancho: 1, alto: 1 };

/**
 * Formas que puede tener un puesto de Mercado (a petición del usuario: la zona se compone de piezas de tamaños
 * distintos). El discriminador es `Edificio.nivelInterno`, que en un puesto NO es progresión: identifica qué
 * forma tiene. Se reutiliza así el mecanismo que ya existe para Granja (`tamanoEdificio(tipo, nivelInterno)`,
 * engine/trazado.ts) en vez de persistir el tamaño en el `Edificio` — el tamaño siempre se DERIVA del tipo.
 */
export const PUESTO_MERCADO_FORMA: Record<number, { ancho: number; alto: number }> = {
  // Formas fijadas tras el playtest del laboratorio (2026-08-31): tres piezas estrechas (1 celda de ancho),
  // que apiladas contra el Mercado forman un mercadillo de puestos alargados en vez de bloques cuadrados.
  1: { ancho: 1, alto: 2 },
  2: { ancho: 1, alto: 3 },
  3: { ancho: 1, alto: 1 },
};

/**
 * Puestos que se AÑADEN al alcanzar cada nivel interno de Mercado, como lista de formas
 * (`PUESTO_MERCADO_FORMA`). Es ACUMULATIVO: cada nivel suma los suyos a los que ya había.
 *
 * Composición fijada en el playtest del laboratorio (2026-08-31). Con la pieza principal (el propio Mercado),
 * la zona queda en 6 piezas en nivel 1, 13 en nivel 2 y 17 en nivel 3.
 *   nivel 1 — forma 1 ×2, forma 2 ×2, forma 3 ×1
 *   nivel 2 — forma 1 ×2, forma 2 ×2, forma 3 ×3
 *   nivel 3 — forma 1 ×1, forma 2 ×2, forma 3 ×1
 */
export const MERCADO_PUESTOS_POR_NIVEL: Record<number, number[]> = {
  1: [1, 1, 2, 2, 3],
  2: [1, 1, 2, 2, 3, 3, 3],
  3: [1, 2, 2, 3],
};

/** Rinde de trigo de UNA Granja según su nivel interno — la mejora duplica producción y costo a la vez (ver
 * `EDIFICIO_CATALOGO.granja.niveles`). Punto único de lectura: la producción de trigo se calcula en tres
 * sitios distintos (engine/construction.ts y engine/asentamientoQuery.ts) y no pueden divergir. */
export function produccionTrigoDeGranja(nivelInterno: number | undefined): number {
  const niveles = EDIFICIO_CATALOGO.granja.niveles as Record<number, NivelEdificioTransformacion>;
  return niveles[nivelInterno ?? 1]?.produccionBaseTrigo ?? EDIFICIO_CATALOGO.granja.produccionBaseTrigo;
}

/**
 * Trazado urbano dinámico (a petición del usuario) — ver `Consideraciones/Vista_Asentamiento_Trazado_Urbano.md`.
 * Nada de esto pre-genera un plano: son los límites que hacen que las manzanas EMERJAN mientras la ciudad se
 * construye edificio a edificio.
 *
 * - `largoFilaMin`/`largoFilaMax`: ANCHO de manzana en celdas — cada cuántas columnas cae una calle
 *   transversal. Se sortea por asentamiento dentro de este rango (determinista por su id, ver `largoMaxFila`
 *   en engine/trazado.ts), nunca `Math.random()`. El fondo de la manzana no se configura: son siempre dos
 *   hileras, una a cada calle, porque más atrás ya no se puede construir sin traer otra calle
 *   (`FONDO_MANZANA`, engine/trazado.ts).
 * - `radioAfuerasMin`: radio VEDADO alrededor del Centro Urbano — dentro de él no puede aparecer ninguna
 *   Granja ni ningún Corral (§8 del doc). Son **10 celdas** a petición del usuario, que seguía viendo granjas
 *   demasiado cerca del centro con el suelo anterior de 4: no basta con que prefieran el hueco más lejano, hay
 *   que prohibir el cercano.
 * - `anchoBandaAfueras`: las afueras llegan al menos hasta `radioAfuerasMin + anchoBandaAfueras`, aunque la
 *   zona de influencia todavía sea más chica. Sin esto, al fundar (`ZONA_INFLUENCIA.radioInicial` = 30) no
 *   habría NINGÚN hueco válido para la Granja inicial y caería al fallback del origen, encima del Centro
 *   Urbano. Y tiene sentido de fondo: el campo de una ciudad está fuera de su zona de influencia, no dentro.
 */
/**
 * PERFIL DE TRAZADO (doc trazado §E6.23) — qué prefiere un edificio cuando elige entre huecos igual de
 * válidos dentro del núcleo de su ancla. Es una PERMUTACIÓN del desempate de `sitiosPorAtraccionDura`
 * (engine/trazado.ts), no una plantilla: las reglas siguen siendo locales y la forma sigue emergiendo, pero
 * la silueta agregada cambia.
 *
 * - `nucleos`   — pegado al ancla manda. Racimos densos concéntricos. Es el comportamiento histórico.
 * - `caminera`  — el frente de calle manda. La ciudad se encadena a las calles que ya existen.
 * - `compacta`  — la cercanía al CENTRO de la ciudad manda. Cada barrio llena primero su cara interior.
 * - `gremial`   — el lado compartido con los AFINES manda. Barrios monocromos, oficios segregados.
 *
 * Un quinto candidato, "palatina" (dominaba `bordeCompartido` con el ancla), se PROBÓ Y SE DESCARTÓ: ese
 * término solo tiene señal cuando el candidato toca el anillo del ancla, así que como criterio dominante vale
 * 0 en casi toda la banda y el resultado salía idéntico a `nucleos` (medido, doc trazado §E6.23). Sigue en el
 * desempate de todos los perfiles, donde sí sirve — pero no puede encabezar ninguno.
 */
export type PerfilTrazado = 'nucleos' | 'caminera' | 'compacta' | 'gremial';

export const PERFILES_TRAZADO: readonly PerfilTrazado[] = ['nucleos', 'caminera', 'compacta', 'gremial'];

export const TRAZADO = {
  /**
   * Override de perfil para el LABORATORIO: `null` = cada asentamiento usa el suyo (política > tradición,
   * ver `resolverPerfil` en engine/trazado.ts). Con un valor, TODOS los asentamientos usan ese perfil.
   *
   * Existe para poder comparar perfiles en el laboratorio y en el batch (`BATCH_PERFIL`) sin montar cargos ni
   * políticas. Nunca debería tener valor en una partida real — es una palanca de desarrollo, igual que el
   * resto de campos que el laboratorio muta en caliente.
   */
  perfilForzado: null as PerfilTrazado | null,
  // Ancho de manzana en celdas. Escala de EDIFICIO, no de calle: se dobló con las huellas en la Etapa 6
  // (§E6.11) y BA-005 lo partió por dos con ellas — la manzana sigue alojando las mismas casas por fila.
  largoFilaMin: 4,
  largoFilaMax: 8,
  // `radioAfuerasMin` y `anchoBandaAfueras` están en UNIDADES LOCALES, no en celdas (ver `radioMaximoAfueras`,
  // engine/trazado.ts, que los compara contra `radioPotencial`): el reescalado de la Etapa 6 NO los toca.
  radioAfuerasMin: 60,
  anchoBandaAfueras: 36,
  // Anclas y satélites, Etapa 2 (Consideraciones/Vista_Asentamiento_Trazado_Urbano.md §5.3/5.7): separación
  // mínima en celdas entre centros de ancla, usada por la búsqueda de ranura del árbol (`radioInicialRanura` /
  // `radioMaximoRanura`, engine/trazado.ts, que se derivan de aquí y se reescalan solas).
  // Ya NO define el núcleo de un ancla: desde §E6.21 ese es la banda de una manzana (`FONDO_MANZANA` celdas
  // desde el anillo de calle, en `sitiosPorAtraccionDura`), no `separacionMinimaAnclas / 2`.
  // Escala de edificio: doblada con las huellas en la Etapa 6 (§E6.11) y partida por dos con ellas en BA-005.
  separacionMinimaAnclas: 6,
  // Zona de seguridad entre anclas (a petición del usuario): un PISO DURO, no relajable — a diferencia de
  // `separacionMinimaAnclas`, que el doc describe como negociable, esta nunca cede. Ningún ancla real nueva
  // (Mercado — `ANCLAS_REALES`, engine/trazado.ts) puede colocarse a menos de esta distancia,
  // BORDE A BORDE (`gapCeldas` en `huecoEnDireccion`), de OTRA ancla ya construida. Si ningún hueco la
  // cumple, no hay sitio válido en ese tick — la colocación se salta o se reintenta, igual que cualquier otro
  // "no cabe" del trazado.
  // Fijada en 6 celdas tras el playtest del laboratorio (2026-08-31) — deja espacio para una calle y una
  // hilera de satélites entre dos anclas vecinas sin que se pisen los núcleos. 3 desde BA-005: los satélites
  // miden la mitad y la proporción se conserva.
  separacionSeguridadAnclas: 3,
  /**
   * Ancho de una calle EN CELDAS (Etapa 6, decisión 1 del doc trazado §E6.3). Con `tamanoCelda` 3 mide lo
   * mismo que el lado de una Vivienda (desde BA-005; en la Etapa 6 era media): ancho validado para combate.
   *
   * Es la constante que hace que la calle CUESTE SUELO, que es el fondo del rediseño: con las calles sobre
   * aristas eran gratis, y por eso el 51% de la red medida no existía físicamente (§E6.1/§E6.2).
   *
   * Uniforme a propósito: la jerarquía callejón/avenida queda aplazada, no descartada (ver "Abierto").
   */
  anchoCalle: 1,
  /**
   * Largo máximo, en celdas, del corredor que un edificio puede reclamar para alcanzar la red (§E6.10).
   * Es lo que impide enterrarse dentro de un coágulo: cuanto más corto, más se pega la ciudad a las calles
   * que ya existen. Sin calibrar todavía — es uno de los dos números del Paso 5.
   *
   * Granja y Corral NO lo usan: viven a `radioAfuerasMin` por diseño y su camino es largo a propósito, así que
   * tienen su propio tope (`capCorredorAfueras`). Un cap único los rechazaría a todos.
   *
   * Escala de CALLE (un corredor son celdas de calle): BA-005 no lo tocó. Si la ciudad se ve demasiado
   * esponjosa con las huellas nuevas, esta es la palanca a recalibrar en el laboratorio.
   */
  capCorredorUrbano: 12,
  capCorredorAfueras: 200,
};

/**
 * Murallas (`Consideraciones/Murallas_Definicion.md`). Un recinto es un ANILLO CERRADO DE CELDAS alrededor del
 * casco urbano, no un edificio: se paga por celda, se levanta celda a celda y sus puertas se congelan al
 * trazarlo.
 *
 * La razón de ser (§0 del doc) es lo que fija estas cifras: la muralla es una ventaja defensiva abrumadora
 * —**menos puertas benefician al defensor**, que solo tiene que defender un embudo— y por eso tiene que ser
 * cara de obtener Y de mantener. De ahí que el coste escale con el perímetro y que exista upkeep.
 *
 * TODO PLACEHOLDER, y las tarifas están MAL A PROPÓSITO hasta el Paso 1: salían de suponer un núcleo urbano
 * de 17x14 celdas, pero la medición del Paso 0 (§11.1) encontró tejidos de 22-29 celdas de radio en nivel 2 y
 * 33-44 en nivel 3 — el perímetro real es 2-3 veces mayor. Se fijan cuando `trazarRecinto` dé el perímetro
 * del trazo de verdad, no antes.
 */
export const MURALLA = {
  /** Gate de construcción (§8 del doc): el mismo nivel de asentamiento que exigía el viejo edificio `muralla`
   * que este recinto sustituye (`EDIFICIO_CATALOGO.muralla.requisitoNivelAsentamientoConstruccion`). */
  nivelMinimoConstruccion: 3,
  /** Nivel más alto de recinto (§7: 1 empalizada · 2 muro de piedra · 3 muralla con adarve). Tope de
   * `iniciarMejoraDeRecinto` — no hay nivel 4 de muralla. */
  nivelMaximo: 3,
  /** Celdas libres entre el último edificio y el muro (el *pomerium*): la banda por la que se circula y se
   * defiende. Es también la palanca contra el riesgo de que un muro interior asfixie el casco antiguo tras
   * una ampliación (§10 del doc). */
  franjaDeRonda: 1,
  /** Cada cuántas celdas de tramo recto aparece una torre, por nivel de recinto. Las esquinas convexas llevan
   * torre siempre. El nivel 1 (empalizada) no tiene torres, por eso no está en la tabla. */
  pasoTorres: { 2: 8, 3: 5 } as Record<number, number>,
  /** Edificios extramuros necesarios para poder AMPLIAR el recinto (§10). Sin un mínimo, ampliar sería spam. */
  arrabalMinimo: 6,
  /** Ritmo de obra: minutos que tarda cada celda, por el nivel que se está pagando (1 empalizada, 2 muro de piedra,
   * 3 adarve) — antes era una celda por minuto (2026-09-26, `Ritmo_Crecimiento_Asentamientos.md` §11). Es lo que hace
   * que el anillo se vea cerrarse poco a poco en vez de aparecer de golpe. */
  minutosPorCelda: { 1: 15, 2: 30, 3: 60 } as Record<number, number>,
  /** Multiplicadores de tarifa sobre la celda de muro llana. Una puerta es una casa-puerta, no un hueco. */
  factorPuerta: 4,
  factorTorre: 3,
  /** Coste de UNA celda de muro llana. Nivel 1 = empalizada (madera domina); 2 y 3 son el coste de MEJORAR
   * cada celda al nivel siguiente, no el coste total acumulado. */
  tarifaPorCelda: {
    1: { madera: 20, piedra: 2 },
    2: { madera: 5, piedra: 25 },
    3: { madera: 10, piedra: 20 },
  } as Record<number, Partial<Record<string, number>>>,
  /** Upkeep por celda y por tick. La mitad de "difícil de obtener Y DE MANTENER": sin esto, una ventaja
   * abrumadora se pagaría una sola vez y duraría para siempre. */
  upkeepPorCelda: {
    1: { madera: 0.02 },
    2: { piedra: 0.02 },
    3: { piedra: 0.04 },
  } as Record<number, Partial<Record<string, number>>>,
  /**
   * Bono defensivo base del recinto. El multiplicador REAL que se aplica a los defensores es
   *
   *     1 + (bonoDefensaPorNivel[nivel] − 1) × integridad / nºPuertas
   *
   * El divisor por puertas es lo que hace real el eje fortaleza↔metrópoli (§0): amurallar pronto da un
   * embudo barato, amurallar tarde protege más ciudad con más frente que cubrir. El "1 +" garantiza que un
   * muro NUNCA perjudique al defensor por muchas puertas que tenga.
   */
  bonoDefensaPorNivel: { 1: 1.3, 2: 1.8, 3: 2.5 } as Record<number, number>,
};

// --- Sprint 3: Economía (Doc 3) ---

// 4 categorías de caravana (Doc 3.6). Militar/construcción se activan en sprints posteriores
// (logística de guerra y fundación/ascenso respectivamente); Sprint 3 solo despacha 'comercial'.
// Velocidad ×2 en las 4 categorías (a petición del usuario, ampliación de comercio): el batch de diagnóstico
// mostró que a la velocidad original un trueque de tamaño moderado a distancia media podía necesitar más
// ticks de viaje (varios envíos en serie, uno por vez por cada lado del acuerdo, ver `asignarCaravanasATrueque`
// en engine/trade.ts) que `TRUEQUE.plazoMinutosPorDefecto` — el acuerdo expiraba antes de poder completarse
// pase lo que pase. Doblar la velocidad de las 4 a la vez mantiene el catálogo consistente entre sí.
// Categorías de caravana sin flota propia (Doc 3.6): la de Fundación (`construccion`, Doc 1.8) es la única
// con uso real; `militar`/`contrabando` siguen siendo solo datos, a la espera de sus disparadores. La
// caravana `comercial` YA NO está aquí — desde el revamp (Doc 3.13) deriva capacidad y velocidad de sus
// carros y animales (`capacidadCaravana`/`velocidadCaravana`, engine/caravanas.ts).
export const CARAVANA_CATALOGO = {
  militar: { capacidad: 40, velocidad: 12 },
  construccion: { capacidad: 150, velocidad: 10 },
  contrabando: { capacidad: 20, velocidad: 24 },
} as const;

/**
 * Cooldown de creación de caravanas (a petición del usuario): tras crear una caravana desde un asentamiento
 * —Fundación (`lanzarCaravanaFundacion`, engine/expansion.ts) o comercial (`construirCaravanaComercial`,
 * engine/trade.ts), COMPARTIDO entre las dos— hay que esperar `cooldownMinutos` ticks antes de poder crear otra
 * desde el mismo asentamiento. Evita que se spamee la creación cuando una caravana recién salida es destruida
 * (bandidos, `engine/bandidos.ts`; intercepción de otra Facción, `engine/combate.ts`) y el cupo/recursos
 * vuelven a estar disponibles de inmediato. Mismo patrón que `CAMPAMENTOS_BANDIDOS.respawnMinutos` /
 * `REGENERACION_NODOS.*.cooldownMinutos` — un solo número parametrizable, sin calibrar por simulación todavía.
 */
export const CARAVANA_COOLDOWN = {
  cooldownMinutos: 10,
};

/**
 * Revamp de caravanas (Doc 3.13, `Consideraciones/Revamp_Caravanas_Definicion.md`). Una caravana `comercial`
 * es una lista de carros, cada uno con su animal, y DERIVA de ahí su capacidad y velocidad
 * (`capacidadCaravana`/`velocidadCaravana`, `engine/caravanas.ts`).
 *
 * ANCLA DE CALIBRACIÓN (decisión del usuario, Ronda 2): `1 carro básico + 1 buey` da 500/16, y su coste
 * (20 madera del carro + 30 madera del buey = 50 madera) es el mismo que costaba antes crear una caravana, así
 * que el batch NPC —que nunca compone nada más, `construirCaravanaComercial`— no se mueve. Todas las demás
 * cifras son PLACEHOLDER sin calibrar por simulación, como el resto de Fase 0.
 */
export const CARRO_CATALOGO = {
  // capacidadBase se multiplica por el `factorCarga` del animal para dar la capacidad real del carro.
  basico: { capacidadBase: 500, costo: { madera: 20 }, fabrica: 'mercado' },
  reforzado: { capacidadBase: 800, costo: { madera: 40 }, fabrica: 'carpinteria' },
} as const;

export const ANIMAL_CATALOGO = {
  // factorCarga multiplica la `capacidadBase` del carro; velocidad entra en el `min` de la caravana; costo es
  // lo que cuesta comprarlo (la cría está diferida, Doc 3.13.7).
  //
  // El BUEY se paga en ORO (~12), no en madera (bloque "economía del oro", Doc 3.13.2 —
  // `Consideraciones/Economia_Del_Oro_Definicion.md` Paso 5). El deadlock que antes justificaba la madera
  // ("sin caravana no hay comercio, sin comercio no hay oro, sin oro no hay caravana") se corta porque la
  // fundación ya entrega 100 oro (`FUNDACION.materialesIniciales`) y la recaudación de oro por población
  // (Doc 4.1) lo repone aunque no haya mina — un asentamiento nuevo se paga su primera caravana (20 madera +
  // 12 oro) con lo que trae de fundar. Buey barato para que sea una decisión de cuántas caravanas montar, no
  // un muro. PLACEHOLDER — la caravana #0 gratis se probó y se quitó en calibración.
  buey: { factorCarga: 1.0, velocidad: 16, costo: { oro: 12 } },
  caballo: { factorCarga: 0.5, velocidad: 24, costo: { oro: 60 } },
  camello: { factorCarga: 0.75, velocidad: 19, costo: { oro: 40 } },
} as const;

/**
 * Preparación de una caravana lanzada a mano (Doc 3.13.3): antes de salir pasa por el estado `'preparando'`
 * en el origen durante `kPorCarro × max(0, nº carros − 1)` ticks — una caravana de 1 carro sale al instante,
 * las grandes tardan. Placeholder sin calibrar.
 */
export const CARAVANA_PREPARACION = {
  kPorCarro: 2,
  /** Cuánto en el futuro se puede programar la salida de una caravana (Doc 3.13.3): la carga queda reservada hasta entonces. PLACEHOLDER. */
  maxProgramacionDias: 3,
};

/**
 * Escolta sin héroe (Doc 3.13.4): un residente del origen cede escuadrones a una caravana como escolta
 * permanente por viaje. `cupoPorNivelMercado[n-1]` es cuántos escuadrones admite una caravana según el nivel
 * interno del Mercado del origen (1 → 1, 2 → 2, 3 → 3). Placeholder sin calibrar.
 */
export const CARAVANA_ESCOLTA = { cupoPorNivelMercado: [1, 2, 3] as const };

/**
 * Scoring de asignación de caravanas disponibles a lados pendientes de trueque (ampliación de comercio, a
 * petición del usuario — "solo simulación": en el diseño objetivo el jugador elige la caravana y la carga a
 * mano, Doc 3.2; esto es el sustituto automático de Fase 0, ver `asignarCaravanasATrueque` en engine/trade.ts).
 * Cada peso pondera un factor normalizado a 0-100; el score final ordena qué lado pendiente se sirve primero
 * cuando hay menos caravanas disponibles que envíos por hacer. Pesos y `distanciaReferencia` PLACEHOLDER,
 * confirmados con el usuario, sin calibrar por simulación todavía.
 */
export const ASIGNACION_CARAVANA = {
  pesoUrgenciaExpiracion: 0.5,
  pesoUrgenciaVolumen: 0.3,
  pesoCercania: 0.2,
  // Distancia a partir de la cual el bonus de cercanía se satura en 0 — mismo orden de magnitud que
  // COMISION.distanciaParaBonusMax (600), pero constante propia porque mide algo distinto (eficiencia de
  // asignación, no bonificación de comisión).
  distanciaReferencia: 600,
};

/**
 * Cuanto vive una orden de mercado sin que nadie la tome (Doc 3.3).
 *
 * **200 minutos, el mismo plazo que un trueque** (`TRUEQUE.plazoMinutosPorDefecto`), y a proposito: son la
 * misma clase de compromiso —una oferta en pie— y darles vidas distintas seria una diferencia que habria que
 * justificar y no hay con que.
 *
 * PLACEHOLDER a calibrar: es el numero que decide cada cuanto una plaza NPC revisa sus precios, porque solo
 * republica cuando la anterior ha caducado.
 */
export const MERCADO = {
  /**
   * Cuántas órdenes cerradas (cumplidas o expiradas) recuerda cada plaza en `historialOrdenes`: las últimas. Es el
   * "historial de tu mercado" que ve su dueño; sin tope crecía sin techo (219 362 órdenes y 65 MB a las 5 semanas de
   * batch, copiadas enteras en cada tick con órdenes cerradas).
   */
  historialPorPlaza: 200,
  plazoOrdenMinutos: 200,
};

export const TRUEQUE = {
  // Un día de mundo (2026-09-28, decisión del usuario): con 200 min, en la Era I medida, 2.633 de 2.720 trueques
  // caducaban sin que ninguna caravana llegara a salir.
  plazoMinutosPorDefecto: 1440,
  /**
   * Cuánto se conserva un trueque ya terminado (cumplido, expirado o rechazado) en `acuerdos`, contado desde su `expiraEn`: una
   * semana de mundo, el historial que ve su dueño. Pasado eso sale del estado, salvo que una caravana aún lo cite. Sin esto
   * crecía sin techo (1 050 a las 5 semanas de batch).
   */
  retencionTerminadosMinutos: 10_080,
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

// Red de caminos (Doc 1.6, `Consideraciones/Rutas_Caravana_Avanzadas_Definicion.md`). Cifras PLACEHOLDER.
export const RED_CAMINOS = {
  /** Una ruta deja de contar (peso) si su par lleva este tiempo sin lanzar una caravana. */
  caducidadMinutos: 3 * 24 * 60,
  /** Escalones visuales por peso: sendero por debajo de `camino`, calzada desde `calzada`. */
  escalonCamino: 2,
  escalonCalzada: 6,
  /** Logro `logistica_campana` (BA-006 D30): una arista fuera de toda zona con estas rutas de estas Facciones. */
  logroRutas: 10,
  logroFacciones: 3,
};

/** Paso forzado por una ciudad ajena (decisión 6): parte de cada recurso que lleva la caravana, en especie. */
export const PEAJE_PASO = {
  tasa: 0.02,
};

// --- Sprint 4: Estructura política (Doc 2) ---

/**
 * Nivel de Facción por EXPERIENCIA (Doc 1.7, rediseño Fase 0.5, a petición del usuario — reemplaza la fórmula
 * anterior de población total, que dejaba que fundar asentamientos de nivel 1 regalara cupos de nivel alto,
 * ver `Fase_0_5_Definicion_Especializacion_y_Cupos.md` §8). Sube por actividad real: combate, construcción,
 * conquista, defensa/ataque de caravana (`engine/faccion.ts` `aplicarAjustesExperiencia`) — MONÓTONO, nunca
 * baja. `xpParaNivel[i]` es el umbral ACUMULADO de experiencia para alcanzar el nivel `i+2` (índice 0 = nivel
 * 2, ..., índice 8 = nivel 10) — curva geométrica ×~1.6 por nivel ("cada nivel cuesta mucho más que el
 * anterior", a petición del usuario), placeholder sin calibrar por simulación.
 */
export const NIVEL_FACCION = {
  nivelMaximo: 10,
  // ×2 el 2026-09-27 (decisión del usuario): en la Era I medida, 6 de 12 Facciones llegaban a nivel 10 y una en 1,2
  // días (`Consideraciones/Ritmo_Crecimiento_Asentamientos.md` §11.4).
  xpParaNivel: [100, 180, 300, 480, 760, 1200, 1900, 3000, 4800],
  // Experiencia otorgada por evento (placeholder). `combate` cubre asedio/campo abierto/atacar campamento de
  // bandidos por igual (participación, no solo victoria) — ver `engine/combate.ts`. Es POR JUGADOR PARTICIPANTE (a
  // petición del usuario, Doc Fase_0_5 §8): si 3 jugadores atacan juntos un campamento, la Facción recibe
  // 3×`combate`, no un monto plano — ver `jugadoresParticipantes` en `engine/combate.ts`. `edificioCompletado` no se
  // multiplica (un edificio no tiene "jugadores que lo completaron" en el modelo actual).
  //
  // La guerra da la mitad desde el 2026-09-27 (combate 5 → 2,5, conquista 20 → 10, decisión del usuario), y el
  // combate solo da experiencia si es DIGNO (`ratioCombateDigno`). `ataqueCaravana` y `defensaCaravana` se quitaron:
  // nada los usaba (interceptar una caravana no da experiencia).
  //
  // Crecer en paz da experiencia desde el 2026-09-27 (decisión del usuario): con la guerra frenada, la experiencia
  // salía casi solo de construir y 9 de 12 Facciones acababan la Era I en nivel 1, sin poder fundar un segundo
  // asentamiento (ficha de ritmo §11.5). `ascensoPorNivel` es por nivel alcanzado (subir a 3 da 3×), `fundacion`
  // no cuenta el primer asentamiento de la Facción, y `truequeCumplido` va a cada uno de los dos lados.
  //
  // `bandidos` es por campamento destruido, NO por jugador participante, y se divide por el nivel de la Facción,
  // hasta 0 desde `nivelSinXpBandidos`: da experiencia aunque no sea un combate digno, pero cada vez menos.
  xp: {
    combate: 2.5,
    edificioCompletado: 1,
    conquista: 10,
    ascensoPorNivel: 30,
    fundacion: 20,
    truequeCumplido: 2,
    bandidos: 1,
  },
  nivelSinXpBandidos: 5,
  /**
   * Un combate es DIGNO si el bando más débil tiene al menos esta fracción del poder del más fuerte (con el jitter y la
   * muralla del propio combate). Solo el combate digno da experiencia de Facción: aplastar a quien no puede
   * defenderse, o estrellarse contra quien no se puede vencer, no enseña nada (decisión del usuario, 2026-09-27; en
   * la Era I medida, el atacante tenía de mediana 0,2 veces el poder del defensor y los dos bandos cobraban igual).
   * Cifra placeholder.
   */
  ratioCombateDigno: 0.5,
};

// Cap de fundación (Doc 1.7): "progresión fácil de 1 a 3, luego se complica hasta un máximo de 7" — curva placeholder.
export const CAP_FUNDACION_POR_NIVEL = [1, 2, 3, 3, 4, 5, 5, 6, 6, 7] as const;

/**
 * Cupo de asentamientos por NIVEL, derivado del nivel de Facción (Doc Fase_0_5 §5, a petición del usuario) —
 * no todos los asentamientos pueden ser de nivel alto: cada uno ocupa ÚNICAMENTE el cupo de SU
 * `nivelActual` operativo (subir de 2 a 3 libera el cupo de 2 que ocupaba; DEGRADAR también lo libera, ver
 * `engine/simulation.ts` — el cupo se cuenta contra `nivelActual`, no contra `nivel`/nivelAlcanzado, para que
 * una ciudad hundida por mal mantenimiento no bloquee un cupo para siempre). Nivel 1 no tiene cupo — el
 * invariante `maxNivel2[i] + maxNivel3[i] = CAP_FUNDACION_POR_NIVEL[i] - 1` garantiza que siempre queda al
 * menos un asentamiento obligatoriamente en nivel 1 (el granero/aserradero que nunca se puede convertir en
 * Ciudad) — cubierto por un test dedicado que falla si se rompe (ver `__tests__/cupo_nivel_invariante.test.ts`).
 * EXCEPCIÓN a ese invariante en Facción nivel 1 (a petición del usuario, corrige un deadlock real): ahí
 * `CAP_FUNDACION_POR_NIVEL` ya es 1 — solo se puede tener UN asentamiento fundado, así que no hay un segundo
 * al que reservarle el puesto de granero. Reservar el "-1" de todos modos dejaba `maxNivel2[0] = 0`, y como
 * lanzar una Caravana de Fundación exige nivelActual 2, una Facción nueva quedaba encerrada CON UN SOLO
 * asentamiento para siempre — la única salida era ganar XP de Facción sin depender de expandirse, algo lento
 * y solo viable jugando de forma activa (combate), no para una Facción pacífica. En Facción nivel 1 el
 * presupuesto de cupo es `CAP_FUNDACION_POR_NIVEL[0]` completo (1), no `CAP_FUNDACION_POR_NIVEL[0] - 1` (0).
 * Enforcement de la PROMOCIÓN (subir `nivel`/nivelAlcanzado): `avanzarNivelAsentamiento`
 * (engine/mantenimiento.ts) solo promueve si hay cupo libre en el nivel objetivo — un asentamiento que
 * cumple los gates pero no tiene cupo se queda "elegible" (nunca se bloquea ni se le baja nada) y reintenta
 * cada tick hasta que se libere uno. Conquista NO se re-evalúa contra este cupo — CONFIRMADO por el usuario
 * como intencional (Fase_0_5_Definicion...md §7 punto #2): incentiva la guerra como vía para saltarse el
 * cupo, igual criterio que `CAP_FUNDACION_POR_NIVEL` (Doc 1.7) ya aplicaba a la fundación. Placeholder sin
 * calibrar por simulación, igual que el resto de curvas del proyecto.
 */
export const CUPO_NIVEL_ASENTAMIENTO = {
  // Facción nivel 1 arranca en 1 (no 0, ver la excepción documentada arriba) — resuelve el deadlock de
  // expansión: la única Facción de una Facción nueva puede llegar a nivel 2 y lanzar su primera Caravana de
  // Fundación sin depender de ganar XP primero.
  maxNivel2: [1, 1, 1, 1, 2, 2, 2, 3, 3, 3] as const,
  // Corrección (consejo LLM detectó que la suma no cuadraba con CAP_FUNDACION_POR_NIVEL-1 en Facción nivel 3
  // y 6 — sin test que lo cubriera): antes [0,0,0,1,1,1,2,2,2,3], ahora +1 en nivel 3 y +1 en nivel 6.
  maxNivel3: [0, 0, 1, 1, 1, 2, 2, 2, 2, 3] as const,
};

export const CIUDADANIA = {
  // Resuelve la pregunta abierta que dejaba `crearFaccion.ts` (a petición del usuario, 2026-08-27): tras
  // abandonar una Facción (`dejarFaccion`), cuánto hay que esperar para poder crear otra — anti-abuso contra
  // "crear, abandonar, crear" en bucle. Solo aplica a CREAR: entrar en una Facción existente (`solicitarIngreso`)
  // no tiene cooldown, solo la regla de siempre (no estar ya en otra).
  cooldownCreacionFaccionDias: 7,
  /** Días de mundo entre un cambio de residencia (`cambiarResidencia`, `dejarResidencia`) y mudarse otra vez
   * (Doc 2.5, decidido el 2026-10-02). Frena mudarse en cada conquista para exprimir la recaudación. PLACEHOLDER. */
  cooldownCambioResidenciaDias: 3,
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
  /** Ranura extra del Gobernador con la Sala del Consejo activa (Doc 4.4), por encima del máximo. */
  slotSalaConsejo: 1,
  duracionMinutosPorDefecto: 150,
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
  // --- Ordenanzas de TRAZADO (doc trazado §E6.23) ---
  //
  // Las cuatro fijan el `perfilTrazado` del asentamiento mientras están activas: cambian QUÉ PREFIERE un
  // edificio al elegir entre huecos igual de válidos, no imponen ninguna plantilla — la forma sigue emergiendo
  // (ver `ORDEN_POR_PERFIL`, engine/trazado.ts). Son EXCLUYENTES ENTRE SÍ sin necesidad de ninguna regla nueva:
  // `maestroObras` tiene un único slot (`POLITICAS.slotsPorCargo`), así que activar una obliga a esperar a que
  // expire la anterior. Compiten en ese mismo slot con Vía Rápida, que es la tensión interesante: forma contra
  // velocidad.
  //
  // Como una política dura `duracionMinutosPorDefecto` (150 ticks) y nada mueve lo ya construido, cada una
  // deja un ESTRATO en la ciudad en vez de reformarla entera — la ciudad acaba registrando su historia
  // política en su geometría.
  //
  // Cada una lleva además un beneficio propio (decidido el 2026-10-02, Doc 4.4), para que elegir forma compita de
  // verdad con Vía Rápida (−25% de tiempo de obra). Cifras PLACEHOLDER, sin calibrar.
  { id: 'postura_defensiva', cargo: 'maestroObras', nombre: 'Postura Defensiva', perfilTrazado: 'compacta', factorTiempoMuralla: 0.75 },
  { id: 'arterias_comerciales', cargo: 'maestroObras', nombre: 'Arterias Comerciales', perfilTrazado: 'caminera', factorComisionExterna: 0.8 },
  { id: 'barrios_gremiales', cargo: 'maestroObras', nombre: 'Barrios Gremiales', perfilTrazado: 'gremial', factorProduccionTalleres: 1.1 },
  { id: 'plazas_mayores', cargo: 'maestroObras', nombre: 'Plazas Mayores', perfilTrazado: 'nucleos', factorCrecimientoPoblacion: 1.25 },
  { id: 'comercio_abierto', cargo: 'tesorero', nombre: 'Comercio Abierto', factorComisionExterna: 0.6 },
  { id: 'aranceles', cargo: 'tesorero', nombre: 'Aranceles Proteccionistas', factorComisionExterna: 1.5 },
  { id: 'leva_forzosa', cargo: 'general', nombre: 'Leva Forzosa', factorCostoReclutamiento: 0.7 },
  // Doc 4.4 / 5.15.3: suma al cupo de guarnición de CADA héroe residente. Aditivo, como `cupoCaravanaExtra`.
  { id: 'levas_guarnicion', cargo: 'general', nombre: 'Levas de guarnición', cupoGuarnicionExtra: 14 },
  // Retirado (a petición del usuario, cambio de base del control de cola): las 4 políticas "Construir
  // Barracón/Galería de Tiro/Palacio/Mercado" y el mecanismo `politicaActivaDesbloqueaEdificio` que las leía
  // desaparecen por completo. Barracón/Galería de tiro/Palacio/Mercado dejan de depender de una política
  // activa — pasan al mismo carril de adición MANUAL que cualquier otro edificio del catálogo (ver
  // `anadirEdificioManualmente`, engine/construction.ts), disponible para Gobernador y Maestro de Obras.
  // Ampliación de comercio (a petición del usuario, Doc 3.3): flota de caravanas propias, ver
  // `cupoCaravanas`/`factorCapacidadCaravana`/`factorVelocidadCaravana` en engine/asentamientoQuery.ts y
  // engine/politicas.ts. `cupoCaravanaExtra` es ADITIVO (no multiplicativo, ver `sumaFactorPolitica`), a
  // diferencia del resto de campos `factor*` de este catálogo.
  { id: 'cupo_caravana_extra', cargo: 'tesorero', nombre: 'Ampliación de Flota', cupoCaravanaExtra: 1 },
  { id: 'carga_ampliada', cargo: 'tesorero', nombre: 'Carga Ampliada', factorCapacidadCaravana: 1.5 },
  { id: 'rutas_rapidas', cargo: 'tesorero', nombre: 'Rutas Rápidas', factorVelocidadCaravana: 1.5 },
  // A petición del usuario: sube la producción de trigo de TODAS las Granjas activas ×1.5 (madera/piedra sin
  // cambios) — ver `factorProduccionTrigo` en engine/politicas.ts, aplicado en `avanzarConstruccion`.
  { id: 'edicto_cosecha', cargo: 'gobernador', nombre: 'Edicto de Cosecha', factorProduccionTrigo: 1.5 },
  // Presión Fiscal (Tesorero, bloque "economía del oro", Doc 4.1/4.4): sube la recaudación de oro por población
  // a cambio de frenar el crecimiento de las 3 clases. Sin sistema de felicidad todavía — el downside es
  // directo sobre el crecimiento (`factorCrecimientoPoblacion` en `crecerPoblacion`). Números PLACEHOLDER, a
  // calibrar en la campaña conjunta del bloque. "Alivio Fiscal" (ir por debajo del baseline) no entra en el
  // primer pase.
  { id: 'presion_fiscal', cargo: 'tesorero', nombre: 'Presión Fiscal', factorRecaudacion: 1.6, factorCrecimientoPoblacion: 0.8 },
] as const;

// --- Sprint 5: Guerra simplificada (Doc 5) ---

/**
 * Catálogo de TROPAS reclutables por equipo (Doc 5.7/5.8, rediseño de reclutamiento): cada tropa se recluta
 * de una vez vía Barracón (cuerpo a cuerpo) o Galería de tiro (a distancia), según el nivel interno del
 * edificio, pagando el equipo fabricado en Armería (ver engine/tropas.ts `reclutarTropa`). Tanto Pesants como
 * Artesanos reclutan por este carril (a petición del usuario — reemplaza también el antiguo reclutamiento
 * directo de Artesanos con cobre a secas, `RECLUTAMIENTO.artesanos`, ya retirado). El reclutamiento de Nobleza
 * (vía Gran Fundición) también se retiró — Nobleza como clase de población sigue existiendo sin cambios (Doc
 * 4.1), solo se quitó la posibilidad de convertirla en tropa. `poderBase` es PLACEHOLDER: no estaba en el
 * diseño original (solo equipo/nivel), interpolado a partir de la progresión del roster de 4 tiers genérico
 * anterior al rediseño (3 → 6 → 12 → 25, ya retirado del código — ver Doc 5.8) repartida en estas 10 tropas a
 * lo largo de 3 niveles.
 *
 * `unidadesPorDefecto` (a petición del usuario — antes el jugador elegía libremente `cantidad` al reclutar,
 * contradiciendo la propia terminología del diseño: "una tropa es el tipo de escuadrón que se recluta DE UNA
 * VEZ", Doc 0/Glosario y Doc 5.8): cada tropa forma un escuadrón de un tamaño fijo al reclutarse — `costoEquipo`
 * sigue siendo POR SOLDADO, así que el coste total de reclutar se multiplica por este número (ver
 * `reclutarTropa`, engine/tropas.ts).
 *
 * **Es un valor propio de cada tropa** (decisión del usuario, 2026-09-27; revierte la del 2026-09-04, que lo
 * derivaba del escalón). Los valores de partida son los que daba el escalón (25 / 20 / 18 / 15 / 12 del 1 al 5):
 * cuanto más de élite, menos cuerpos, que es lo que ordena el poder por punto de Liderazgo de mayor (leva) a menor
 * (élite). Al fijarlo a mano hay que vigilar ese orden: una tropa de pocos cuerpos con poder de leva, o al revés,
 * rompe la curva (así salieron los Arqueros dominantes de 2026-09-04).
 */
export const TROPAS_RECLUTABLES: {
  id: string;
  /** La tecnología que la Facción tiene que haber adoptado para reclutarla (las cuatro puertas, Doc 6.1). */
  tecnologia: TecnologiaId;
  nombre: string;
  edificio: 'centroUrbano' | 'barracon' | 'galeriaDeTiro' | 'caballerizas';
  nivelRequerido: number;
  costoEquipo: Partial<Record<string, number>>;
  poderBase: number;
  /** Soldados del escuadrón: propio de cada tropa (ver arriba). */
  unidadesPorDefecto: number;
  /** Velocidad de marcha por el mapa general (Doc 5.12.5). Un ejército va al ritmo de su escuadrón MÁS
   * LENTO, así que meter un solo escuadrón pesado en una partida de incursión la frena. Las dos reglas que
   * fijan estos números: una caravana inicial (comercial, 16) no puede ser más rápida que un ejército, y un
   * jugador solo con infantería ligera tiene que poder alcanzarla. */
  velocidad: number;
  /** Escalón de élite, 1 (leva) a 5 (élite). Decide su coste de Liderazgo — ver `LIDERAZGO.costePorEscalon`. */
  escalon: 1 | 2 | 3 | 4 | 5;
  /** Caballos por soldado (caballería 1, carros 2): se pagan en oro al reclutar y al reponer (`ORO_POR_CABALLO`, Doc 5.8). */
  caballos?: number;
}[] = (
  [
  // Escalón de entrada (a petición del usuario: la defensa mínima no debe depender de Barracón — que exige
  // añadirlo MANUALMENTE a la cola vía Gobernador/Maestro de Obras antes de siquiera empezar a construirse,
  // ver `anadirEdificioManualmente` en engine/construction.ts — sino de Centro Urbano, el único edificio que
  // nace `activo` con el asentamiento desde el tick de fundación, sin cola de construcción ni gate alguno, ver
  // `settlement.ts` `edificiosIniciales`). Así CUALQUIER asentamiento fundado puede defenderse desde el minuto
  // uno, así sea con la tropa más débil del roster — antes dependía indirectamente de tener madera de sobra
  // para pagar el Barracón (30 madera) y de que alguien lo añadiera a la cola primero. Se paga con madera EN BRUTO, sin
  // pasar por Armería. Débil a propósito (poderBase 2, por debajo de todo lo demás): existe para que el bucle
  // de juego arranque y las primeras escaramuzas ocurran pronto, no para ganar batallas. Sigue exigiendo un
  // General asignado (`reclutarTropa` en engine/tropas.ts) — eso no cambia, solo el edificio.
  { id: 'milicia_lanceros', tecnologia: 'leva_comunal', nombre: 'Milicia de lanceros', edificio: 'centroUrbano', nivelRequerido: 1, costoEquipo: { madera: 2 }, poderBase: 2, velocidad: 20, escalon: 2, unidadesPorDefecto: 25 },
  // Leva desorganizada (Doc 5.8): muy mala en combate y sin formación en la batalla. Del Centro Urbano, sin oro.
  { id: 'lenadores', tecnologia: 'leva_comunal', nombre: 'Leñadores', edificio: 'centroUrbano', nivelRequerido: 1, costoEquipo: { madera: 1 }, poderBase: 1, velocidad: 20, escalon: 1, unidadesPorDefecto: 30 },
  { id: 'granjeros', tecnologia: 'leva_comunal', nombre: 'Granjeros', edificio: 'centroUrbano', nivelRequerido: 1, costoEquipo: {}, poderBase: 1, velocidad: 20, escalon: 1, unidadesPorDefecto: 30 },
  // Recosteadas a `armaMadera` (ver RECETA_ARMA_MADERA): antes exigían la cadena del cobre/cuero entera, lo
  // que era además temáticamente incoherente — un escudo de MIMBRE pagado con un arma de cobre, y unos
  // Honderos (una honda y una piedra) pagados con armadura de cuero. El cobre pasa a ser la MEJORA
  // (`espadachines_cobre`, que sí lo conserva), no el ticket de entrada.
  { id: 'lanceros_mimbre', tecnologia: 'escudos_ligeros', nombre: 'Lanceros con escudo de mimbre', edificio: 'barracon', nivelRequerido: 1, costoEquipo: { armaMadera: 1 }, poderBase: 3, velocidad: 20, escalon: 2, unidadesPorDefecto: 25 },
  { id: 'espadachines_cobre', tecnologia: 'armamento_palacial', nombre: 'Espadachines de espada corta de cobre', edificio: 'barracon', nivelRequerido: 1, costoEquipo: { armaCobre: 1, armaduraBasica: 1 }, poderBase: 4, velocidad: 16, escalon: 2, unidadesPorDefecto: 20 },
  { id: 'hacheros_ligeros', tecnologia: 'armamento_palacial', nombre: 'Hacheros ligeros', edificio: 'barracon', nivelRequerido: 2, costoEquipo: { armaBronce: 1, armaduraBasica: 1 }, poderBase: 7, velocidad: 16, escalon: 3, unidadesPorDefecto: 18 },
  { id: 'espadachines_bronce', tecnologia: 'aleacion_bronce', nombre: 'Espadachines con espadas y escudos de bronce', edificio: 'barracon', nivelRequerido: 2, costoEquipo: { armaBronce: 2, armaduraIntermedia: 1 }, poderBase: 9, velocidad: 16, escalon: 3, unidadesPorDefecto: 18 },
  { id: 'lanceros_pesados', tecnologia: 'panoplia_bronce', nombre: 'Lanceros pesados micénicos', edificio: 'barracon', nivelRequerido: 3, costoEquipo: { armaBronce: 2, armaduraBronce: 1 }, poderBase: 14, velocidad: 12, escalon: 4, unidadesPorDefecto: 15 },
  { id: 'hacheros_armados', tecnologia: 'aleacion_bronce', nombre: 'Hacheros armados', edificio: 'barracon', nivelRequerido: 2, costoEquipo: { armaBronce: 1, armaduraIntermedia: 1 }, poderBase: 12, velocidad: 12, escalon: 3, unidadesPorDefecto: 15 },
  { id: 'honderos', tecnologia: 'hostigamiento_tribal', nombre: 'Honderos', edificio: 'galeriaDeTiro', nivelRequerido: 1, costoEquipo: { armaMadera: 1 }, poderBase: 5, velocidad: 20, escalon: 2, unidadesPorDefecto: 20 },
  { id: 'escaramuzadores_jabalina', tecnologia: 'hostigamiento_tribal', nombre: 'Escaramuzadores con jabalina', edificio: 'galeriaDeTiro', nivelRequerido: 2, costoEquipo: { armaCobre: 1, armaduraBasica: 1 }, poderBase: 8, velocidad: 20, escalon: 3, unidadesPorDefecto: 18 },
  { id: 'arqueros', tecnologia: 'arqueria_palacial', nombre: 'Arqueros', edificio: 'galeriaDeTiro', nivelRequerido: 2, costoEquipo: { armaBronce: 1, armaduraIntermedia: 1 }, poderBase: 9, velocidad: 16, escalon: 3, unidadesPorDefecto: 18 },
  { id: 'arqueros_compuesto', tecnologia: 'arco_compuesto', nombre: 'Arqueros con arco compuesto', edificio: 'galeriaDeTiro', nivelRequerido: 3, costoEquipo: { armaBronce: 3, armaduraIntermedia: 2 }, poderBase: 15, velocidad: 12, escalon: 5, unidadesPorDefecto: 12 },
  // Caballerizas (Doc 5.8): la caballería va a 28, la única clase que alcanza a las caravanas rápidas (24, D8); los
  // carros, a 20. Cada caballo se paga en oro (D5).
  { id: 'exploradores_caballo', tecnologia: 'cria_caballar', nombre: 'Exploradores a caballo', edificio: 'caballerizas', nivelRequerido: 1, costoEquipo: { armaCobre: 1 }, caballos: 1, poderBase: 3, velocidad: 28, escalon: 2, unidadesPorDefecto: 20 },
  { id: 'carros_guerra', tecnologia: 'carros_guerra', nombre: 'Carros de guerra', edificio: 'caballerizas', nivelRequerido: 2, costoEquipo: { carroGuerra: 1, armaBronce: 1 }, caballos: 2, poderBase: 13, velocidad: 20, escalon: 4, unidadesPorDefecto: 15 },
  // Era II — Crisis y adaptación (Doc 5.8).
  { id: 'jinetes_asirios', tecnologia: 'equitacion_militar', nombre: 'Jinetes asirios', edificio: 'caballerizas', nivelRequerido: 1, costoEquipo: { armaHierro: 1 }, caballos: 1, poderBase: 5, velocidad: 28, escalon: 2, unidadesPorDefecto: 20 },
  { id: 'guerreros_filisteos', tecnologia: 'forja_hierro_temprana', nombre: 'Guerreros filisteos (Peleset)', edificio: 'barracon', nivelRequerido: 2, costoEquipo: { armaHierro: 1, armaduraIntermedia: 1 }, poderBase: 9, velocidad: 16, escalon: 3, unidadesPorDefecto: 18 },
  { id: 'shardana', tecnologia: 'bronce_calidad_militar', nombre: 'Shardana', edificio: 'barracon', nivelRequerido: 2, costoEquipo: { armaBronceCalidad: 1, armaduraBasica: 1 }, poderBase: 15, velocidad: 16, escalon: 4, unidadesPorDefecto: 15 },
  { id: 'hequetai', tecnologia: 'panoplia_bronce', nombre: 'Hequetai', edificio: 'barracon', nivelRequerido: 3, costoEquipo: { armaBronceCalidad: 1, armaduraBronce: 1 }, poderBase: 17, velocidad: 12, escalon: 5, unidadesPorDefecto: 12 },
  // Era III — Polis e imperios (Doc 5.8).
  { id: 'honderos_rodios', tecnologia: 'pantalla_escaramuzadores', nombre: 'Honderos rodios', edificio: 'galeriaDeTiro', nivelRequerido: 2, costoEquipo: { armaHierro: 1, armaduraBasica: 1 }, poderBase: 9, velocidad: 20, escalon: 3, unidadesPorDefecto: 18 },
  { id: 'peltastas', tecnologia: 'pantalla_escaramuzadores', nombre: 'Peltastas', edificio: 'galeriaDeTiro', nivelRequerido: 2, costoEquipo: { armaHierro: 2 }, poderBase: 9, velocidad: 20, escalon: 3, unidadesPorDefecto: 18 },
  { id: 'jinetes_escitas', tecnologia: 'caballeria_organizada', nombre: 'Jinetes escitas', edificio: 'caballerizas', nivelRequerido: 2, costoEquipo: { armaBronce: 1, armaduraBasica: 1 }, caballos: 1, poderBase: 8, velocidad: 28, escalon: 3, unidadesPorDefecto: 18 },
  { id: 'hoplitas_ciudadanos', tecnologia: 'ciudadania_militar', nombre: 'Hoplitas ciudadanos', edificio: 'barracon', nivelRequerido: 2, costoEquipo: { armaHierro: 1, armaduraBronce: 1 }, poderBase: 10, velocidad: 16, escalon: 3, unidadesPorDefecto: 18 },
  { id: 'caballeria_asiria', tecnologia: 'caballeria_organizada', nombre: 'Caballería asiria', edificio: 'caballerizas', nivelRequerido: 2, costoEquipo: { armaHierro: 1, armaduraBronce: 1 }, caballos: 1, poderBase: 10, velocidad: 28, escalon: 3, unidadesPorDefecto: 18 },
  { id: 'arqueros_escitas', tecnologia: 'arqueria_especializada', nombre: 'Arqueros escitas', edificio: 'galeriaDeTiro', nivelRequerido: 3, costoEquipo: { armaHierro: 1, armaduraBasica: 1 }, poderBase: 17, velocidad: 16, escalon: 5, unidadesPorDefecto: 12 },
  { id: 'espartiatas', tecnologia: 'falange_hoplita', nombre: 'Espartiatas', edificio: 'barracon', nivelRequerido: 3, costoEquipo: { armaHierroCalidad: 1, armaduraBronceCalidad: 1 }, poderBase: 17, velocidad: 12, escalon: 4, unidadesPorDefecto: 15 },
  ] as const
);

export const MILITAR = {
  racionPorSoldadoPorMinuto: 0.15,
  regeneracionMoralPorMinuto: 5,
  degradacionMoralSinRacion: 20,
  // Fracción de la cantidad del escuadrón que deserta por minuto mientras la moral está a 0 (Doc 5.4).
  desercionFraccionPorMinutoSinMoral: 0.05,
  // Nivel de escuadra (Doc 5.16.3, decisión del usuario 2026-09-27): +1 % de poder por nivel, hasta el 10. Cada nivel
  // pide su experiencia: poca al principio, el doble cada vez hasta el nivel 5, y desde ahí todos lo mismo. Índice i =
  // la que pide pasar del nivel i+1 al i+2.
  bonusPoderPorNivelEscuadra: 0.01,
  nivelMaximoEscuadra: 10,
  experienciaParaSubirEscuadra: [5, 10, 20, 40, 40, 40, 40, 40, 40],
  // Experiencia de escuadra en el combate numérico (decisión del usuario 2026-09-14).
  experienciaGanadaPorVictoria: 1,
  experienciaGanadaPorDerrota: 0.5,
  // Ventaja del defensor en el combate numérico (decisión del usuario 2026-09-27): su poder se multiplica por esto.
  ventajaDefensor: 1.05,
  // Cohesión entre escuadrones defendiendo juntos (Doc 5.3), abstraída como bonus de poder (sin formaciones renderizadas).
  bonusCohesionPorEscuadronExtra: 0.1,
  varianzaCombate: 0.15,
  // Combate de caravanas (Doc 3.10): umbral de captura del 50% y defensa base de una escolta no modelada en detalle.
  umbralCapturaCaravana: 0.5,
  defensaBaseCaravana: 15,
};

/**
 * Tras la conquista (Doc 5.12.9, `Consideraciones/Ocupacion_Post_Conquista_Definicion.md`) el asentamiento se
 * saquea y arrancan dos relojes: la protección (inmune a un nuevo asedio) y la ventana de ocupación (recaudación y
 * crecimiento reducidos, mantenimiento congelado).
 */
export const OCUPACION = {
  /** Protección del nuevo dueño (2026-09-27, decisión del usuario): un día de mundo, lo que tarda la plaza en
   * recuperarse y reclutar con qué defenderse. Cuando la daba la ventana de ocupación, de 90 min, en la semana 1
   * medida un dueño conservaba la plaza 0,1 días de mediana. */
  proteccionMinutos: 1440,
  /** Ventana de ocupación: el coste de haberla tomado. */
  duracionMinutos: 90,
  /** Fracción de pesants, artesanos y nobleza que se pierde en el saqueo. */
  fraccionSaqueoPoblacion: 0.25,
  /** Fracción de los edificios `activo` que el saqueo baja a `en_cola` marcados `danado` — excluidos Centro
   * Urbano y al menos una Granja y una Leñera activas. */
  fraccionEdificiosDanados: 0.25,
  /** Un edificio `danado` se reconstruye pagando esta fracción del costo de catálogo y tardando esa fracción
   * de tiempo — se repara, no se levanta de cero. */
  fraccionCosteReconstruccion: 0.5,
  /** Reducción del `avance` de cada recinto completo (sobre `celdas.length`): la muralla se daña, no cae. */
  fraccionDanoMuralla: 0.3,
  /** Recaudación de oro del asentamiento durante la ventana (`recaudacionOro`). */
  factorRecaudacion: 0.5,
  /** Crecimiento de población durante la ventana (`crecerPoblacion`, factor `felicidad`). */
  factorCrecimiento: 0.5,
};

/**
 * Liderazgo (Doc 5.11): cuánta tropa puede sacar a campaña un Jugador de una vez. Límite de SALIDA, no de
 * posesión — lo que se queda es la guarnición, y es lo único que defiende (Doc 5.12.4).
 *
 * **El coste va por ESCALÓN, no derivado del poder (rediseño 2026-09-04, decisión del usuario).** Antes era
 * `poderBase × unidades × factor`, y esa fórmula tenía un defecto de fondo que el consejo ya había señalado:
 * al ser el coste exactamente proporcional al poder nominal, **el poder por punto de Liderazgo salía idéntico
 * para todas las tropas**. La élite no era mejor por punto, solo venía en envase más pequeño — así que elegir
 * no era una decisión, era aritmética.
 *
 * Con coste por escalón el coste crece MÁS DEPRISA que el poder, y eso es lo que se busca: la élite es
 * deliberadamente ineficiente por punto. Se la lleva uno porque veinte cuerpos de élite aguantan un paso que
 * cien de leva no, no porque rindan más por punto gastado.
 *
 * Los cinco escalones y su presupuesto están elegidos para que las composiciones interesantes queden JUSTO en
 * el techo, que es lo que hace que la decisión duela:
 *
 * | Escalón | Coste | Caben con 100 |
 * |---|---|---|
 * | 1 — leva | 7 | 14 |
 * | 2 — tropa de línea | 14 | 7 |
 * | 3 — veterana | 22 | 4 |
 * | 4 — pesada | 32 | 3 |
 * | 5 — élite | 45 | 2 |
 *
 * Y las mezclas máximas salen redondas: **1 élite + 1 pesada + 1 veterana = 99**, **2 pesadas + 1 veterana +
 * 1 de línea = 100**. Ninguna sobra ni falta por poco.
 *
 * `base` 100 (decisión del usuario). El techo con equipo queda para cuando exista la artesanía de armaduras
 * (`Docs/Mecanicas a desarrollar.md` §11, progresión de jugador): la mecánica ya lo admite sin tocar nada —
 * `Jugador.liderazgoBase` es por jugador y quien no lo tenga usa este valor.
 */
export const LIDERAZGO = {
  base: 100,
  costePorEscalon: { 1: 7, 2: 14, 3: 22, 4: 32, 5: 45 } as Record<number, number>,
};

/** El Héroe como personaje (Doc 5.16), con los valores de Conquest (`HeroDataService`, `HeroAttributeValidator`;
 * decisión del usuario 2026-09-14). */
export const HEROE = {
  bronceInicial: 500,
  topeAtributo: 100,
  /** Cuánto dura el estado Herido (Doc 5.16.4), en minutos de mundo. Lo sufren todos los héroes del bando que pierde
   * una batalla, y mientras dura no persiguen, no se les persigue ni entran en batallas. Sustituye a la Tregua. */
  heridoMinutos: 2,
};

/** Una batalla jugada en Unity (Doc 5.15.1; doc 01 §15). */
export const BATALLA = {
  /** Héroes por bando. Las escuadras sin héroe (guarnición, escolta, bandidos) no ocupan plaza. El asedio va a 5
   * mientras se prueba con NPC (decisión del usuario 2026-09-27); vuelve a 15 cuando entren jugadores
   * (`Docs/Mecanicas a desarrollar.md`). */
  capacidad: { asedio: 5, resto: 5 },
  /** Lo que dura como mucho la partida, en minutos: si se agota, gana el defensor. */
  duracionMinutos: { asedio: 30, resto: 15 },
  /** Plazos de infraestructura, en minutos de mundo: si nadie la asigna o la empieza a tiempo, `fallida` sin
   * castigo. El margen se suma a la duración para no dar por perdido un resultado que llega tras una caída corta. */
  plazoAsignacionMinutos: 5,
  plazoInicioMinutos: 5,
  margenMinutos: 5,
  /** Tropa sin dueño: la de un campamento (su poder 30 de hoy, Doc 1.9) y los carreteros de una caravana sin escolta
   * (Doc 3.10). No persisten entre batallas. */
  tropaCarreteros: { tropaId: 'milicia_lanceros', unidades: 13 },
  /** Casillas del inventario de Conquest (`InventoryStorageService.InventoryLimit`): lo que cabe de botín. */
  casillasInventario: 72,
  /** `ponytail:` versiones de los catálogos de Conquest, fijas hasta que los publique (CQ-004). */
  versionCatalogoHeroe: 'conquest-heroes-1',
  versionCatalogoObjetos: 'conquest-items-1',
};

/** Cupo de guarnición de cada héroe residente (Doc 5.15.3), en la escala del coste de Liderazgo (5.11.1). La
 * política "Levas de guarnición" suma lo suyo desde `POLITICA_CATALOGO` (`cupoGuarnicionExtra`). */
export const GUARNICION = {
  /** Barracón y Galería de tiro, cada uno por su nivel interno 1/2/3. */
  cupoPorNivelEdificio: [7, 14, 22],
  /** Caballerizas por nivel interno 1/2/3 (D37): nada, la caballería más barata (14) y la más cara del roster (32). */
  cupoPorNivelCaballerizas: [0, 14, 32],
  recintoCompleto: 14,
};

/**
 * Oro por soldado al reclutar (Doc 5.8, bloque "economía del oro" — `Consideraciones/Economia_Del_Oro_Definicion.md`
 * Paso 4): además del equipo (`costoEquipo`), reclutar cuesta oro según el escalón de la tropa. Curva que sube
 * más deprisa que el poder, igual criterio que `LIDERAZGO.costePorEscalon`. El coste total de oro es este
 * valor × nº de soldados reclutados/repuestos.
 *
 * **Única excepción: las tropas del Centro Urbano** (`tropa.edificio === 'centroUrbano'`: milicia, leñadores y
 * granjeros), que no pagan oro — la defensa mínima no depende del tesoro. Todo lo demás cuesta oro, más
 * `ORO_POR_CABALLO` por cada caballo. `factorCostoReclutamiento` ("Leva Forzosa") NO toca esta línea, solo el equipo.
 * Regla de motor uniforme (NPC + jugador). Todo PLACEHOLDER, a calibrar en la campaña conjunta del bloque.
 */
export const RECLUTAMIENTO_ORO_POR_ESCALON: Record<number, number> = { 1: 1, 2: 2, 3: 4, 4: 7, 5: 11 };

/** Oro por caballo al reclutar o reponer caballería y carros (D5, Doc 5.8): los caballos se compran. Placeholder. */
export const ORO_POR_CABALLO = 5;

/**
 * Logística de campaña (Doc 5.13). Todo PLACEHOLDER a calibrar.
 *
 * `capacidadCarroPorJugador` NO es un número elegido: sale del RADIO OPERATIVO objetivo que fijó el usuario
 * —"un jugador solo tiene que poder recorrer al menos un cuarto del mapa ida y vuelta"— sobre el mapa de
 * 2000×2000. Son 1.000 unidades de recorrido; una carga máxima de liderazgo (~70 soldados) a velocidad
 * ligera (20) tarda 50 ticks y come `70 × 0.15 × 50 = 525`. De ahí el 500 redondeado.
 *
 * IMPORTANTE al rebalancear: si cambia la ración o la producción de trigo, RECALCULAR desde el radio en vez
 * de ajustar este número a ojo — si no, el radio operativo se rompe en silencio. `autonomiaTicksObjetivo` es
 * el invariante de diseño del que cuelga todo lo demás.
 */
export const LOGISTICA = {
  capacidadCarroPorJugador: 500,
  autonomiaTicksObjetivo: 50,
  /**
   * Cuánto come un ejército ACAMPADO respecto a uno en marcha (Doc 5.12.3). **Una décima parte** (decisión
   * del usuario, 2026-09-04): con 0.5 estacionar apenas compraba tiempo —un carro lleno aguantaba el doble en
   * vez de diez veces más— y "plantarse en un sitio" no llegaba a ser una jugada. A 0.1 sí lo es: sostener un
   * paso de montaña deja de ser una carrera contra el hambre.
   *
   * Nunca 0, que es la otra mitad de la regla: acampar cuesta comida, solo que poca.
   */
  factorConsumoEstacionado: 0.1,
  radioReabastecimiento: 60,
  /**
   * Fracción de la capacidad del carro bajo la cual repostar es noticia (`ejercito.reabastecido`). Por encima es la ración
   * de cada minuto de un ejército acampado junto a su plaza y no se narra (era el 39 % del log a los 3 días).
   */
  umbralNarrarReposte: 0.5,
  /**
   * A qué distancia dos cosas que se mueven se TROPIEZAN (Paso 10). **15** (decisión del usuario,
   * 2026-09-04), frente a los 150 de visión: ver y chocar son cosas distintas y por eso los números no se
   * parecen. Con 15 un ejército divisa a otro con muchísima antelación y puede evitarlo, interceptarlo o
   * prepararse — el encuentro es una DECISIÓN, no un accidente por pasar cerca.
   *
   * El conflicto que esto cierra: con 60 (el valor de reabastecimiento, heredado sin decidir) el margen entre
   * ver y chocar era de solo 2.5×, y cualquier cruce de rutas acababa en combate quisiera o no.
   */
  radioEncuentro: 15,
};

/**
 * El jugador moviéndose por el mundo (Doc 1.10 y 5.12): lo que cuesta y lo que rinde ir por ahí, con tropas
 * o sin ellas.
 *
 * PLACEHOLDER a calibrar por simulación, como el resto de constantes militares.
 */
/**
 * Campamentos de mercenarios (Doc 1.9b, `Docs/Mecanicas a desarrollar.md` §40). Cifras decididas el 2026-10-02 pero PLACEHOLDER:
 * sin calibrar por simulación.
 */
export const MERCENARIOS = {
  /** Distancia mínima entre campamentos (D1, §8.2 de Campamentos_Entrada_Fundacion_Definicion): 900 da unos 4 en un
   * mapa de 2000 —el playtest—; 600, de 6 a 9 para más gente. PLACEHOLDER. */
  separacion: 900,
  /** Ningún campamento a menos de esto del borde del mapa: deja sitio a su anillo de bandidos. */
  margenBorde: 150,
  /** Puntos al azar que se prueban al colocarlos; caben los que caben. */
  intentosColocacion: 2000,
  /** Se mezcla con la seed del mapa: semilla derivada (D35), no consume el RNG de la partida. */
  salSemilla: 0x6d657263,
  /** Ración gratis del residente (D24, §8.1): este trigo al salir de su campamento, una vez cada tanto; no se acumula. PLACEHOLDER. */
  racion: { trigo: 60, cadaMinutos: 30 },
  /** Tropa prestada al residente (D25, D45, D80): escuadras de leva comunal de estas unidades, gratis al pedirlas y al reponerlas. PLACEHOLDER. */
  prestamo: { unidades: 15 },
  /** A menos de esto de un campamento nadie inicia un combate, ni jugadores ni bandidos (M4/D78, §8.2). PLACEHOLDER. */
  radioProteccion: 60,
  /** Variantes de aspecto, elegidas al nacer: no cambian nada de lo que hace. */
  origenes: 3,
  /** Siempre los tiene; el militar se elige al azar entre `edificiosMilitares`. */
  edificiosFijos: ['taberna', 'vivienda', 'vivienda', 'mercado'] as readonly EdificioCampamentoTipo[],
  edificiosMilitares: ['barracon', 'galeriaDeTiro', 'caballerizas'] as readonly EdificioCampamentoTipo[],
  /** Preferencia por un bosque: si hay uno a menos de esto del punto, el campamento se pega a su borde. */
  margenBosque: 100,
  pegadoAlBorde: 10,
  // --- Reclutamiento (paso 3) ---
  /** Reclutas que da cada vivienda: el tope es viviendas × esto, así que añadir viviendas al layout lo sube solo. */
  poblacionPorVivienda: 50,
  /** Reclutas que recupera por hora de mundo, hasta el tope. */
  poblacionPorHora: 10,
  /** Reclutar aquí cuesta esto veces el precio base (más caro que en casa: no sustituye a las plazas). */
  recargo: 1.5,
  /** Sobre el precio ya recargado, para la Facción que no tiene ningún asentamiento: puede recomponerse. */
  descuentoSinAsentamientos: 0.6,
  /** Nivel interno de sus edificios militares: todas las tropas de ese edificio, limitadas por la tecnología del campamento. */
  nivelEdificios: 3,
  /** El campamento cobra todo en oro: lo que vale una unidad de equipo (no hay precio de mercado de las armas fabricadas). */
  valorEquipoEnOro: 8,
  /** Desbloquea cada tecnología el último: cuando la tiene este porcentaje de las Facciones humanas vivas o pasan estas horas desde la primera. */
  tecnologia: { porcentajeFacciones: 0.5, horasTrasLaPrimera: 72 },
  // --- Mercado (paso 4) ---
  mercado: {
    /** Cada cuántas horas de mundo repone, y qué fracción de lo comerciado en ese tiempo recibe cada campamento. */
    cadaHoras: 3,
    proporcionRepone: 0.03,
    /** Stock máximo por bien en un campamento, y con el que nace (los bienes con precio de referencia). */
    topePorBien: 300,
    stockInicial: 100,
    /** Pila propia (D41, §8.1): estos bienes no dependen del comercio del mundo —en un mundo recién creado no lo hay—; nacen con
     * ella y vuelven a ella en cada reposición. Compiten por ella todos los que compran. PLACEHOLDER (el trigo no está calibrado). */
    pilas: { madera: 300, piedra: 60, trigo: 300 } as Readonly<Record<string, number>>,
    /** Cupo por héroe y día de mundo (D41): corta el acaparamiento. El trigo no tiene: es para repostar (D44). */
    cupoDiario: { madera: 60, piedra: 10 } as Readonly<Record<string, number>>,
    /** Se vende a este múltiplo del precio de referencia (+30 %): una válvula, no una competencia. El oro cobrado se destruye. */
    margen: 1.3,
  },
  // --- Refundar (paso 5) ---
  refundacion: {
    /** La Caravana de Fundación comprada en un campamento cuesta esta fracción de una normal (`costoCaravanaFundacion`). */
    porcentajeCoste: 0.75,
  },
} as const;

/** El almacén personal de un héroe (Doc 2.5, decidido el 2026-10-02): unidades en total, de cualquier recurso. PLACEHOLDER. */
export const ALMACEN_PERSONAL = { capacidad: 1000 } as const;

/** La puerta de los asentamientos (Doc 1.10.5). */
export const PUERTA = {
  /** Lo que cierra una plaza que nadie ha tocado: deja entrar a los suyos y a los amigos, y a nadie más. */
  cerradaAPorDefecto: ['neutrales', 'enemigos'] as readonly GrupoPuerta[],
} as const;

/** La capital de una Facción (Doc 2.2, decidido el 2026-10-02). */
export const CAPITAL = {
  /** Días de mundo entre una designación y la siguiente. PLACEHOLDER. */
  cooldownDias: 14,
} as const;

/** Presencia (Doc 1.10.6). */
export const PRESENCIA = {
  /** Lo que tarda en salir del mundo quien se desconecta: alcanza a quien ya tenía a tiro, no a quien iba lejos. */
  retardoDesconexionMs: 150_000,
} as const;

export const MOVIMIENTO = {
  /**
   * Velocidad de una columna SIN escuadrones — un jugador viajando solo. Por encima de la tropa ligera (20)
   * porque un hombre solo no arrastra impedimenta, y por debajo de la caravana de contrabando (24), que por
   * diseño escapa de todo.
   */
  velocidadJugador: 22,
  /**
   * Trigo por participante y minuto, con tropas o sin ellas. Existe para que una columna sin soldados no
   * consuma CERO: viajar tiene que costar algo, o el viajero solitario sería gratis e infinito. Poco en
   * absoluto —un carro lleno (500) da para más de 16 horas de viaje— pero nunca nada.
   */
  consumoPorParticipante: 0.5,
  /**
   * A qué distancia de una plaza se puede cruzar su puerta (Doc 1.10.3). **10**, la más corta de las
   * distancias de interacción — por debajo del choque (15) y muy por debajo de ver (150): entrar exige
   * estar literalmente en la puerta, no en las afueras. Es lo que hace de "entrar" un acto y no un roce.
   */
  radioPuerta: 10,
  /**
   * Cuánto vive una petición de unión sin contestar, antes de darse por RECHAZADA (Doc 5.14.1). **10**, y
   * corto a propósito: pocos para que el que pide no se quede plantado en mitad del mapa, bastantes para que
   * un grupo que está hablando se organice. La consecuencia se asume: *preguntar* solo funciona con el Líder
   * al teclado, así que una columna que marcha en serio elegirá casi siempre *aceptar* o *rechazar*.
   *
   * En segundos y no en minutos porque en minutos sería 1/6. Es el único plazo del juego por debajo del tick.
   */
  vidaPeticionUnionSegundos: 10,
  /**
   * A que distancia se puede INSPECCIONAR una columna o una caravana ajena, y a la que el observado se entera
   * (Doc 5.12.3). **40**: a media distancia entre ver (150) y chocar (15), que es lo que lo hace un juego de
   * dos. Bastante lejos como para que un explorador se acerque y se vaya antes de que una columna lo alcance
   * —es mas rapido—, bastante cerca como para que mirar cueste ser visto mirando.
   */
  radioInspeccion: 40,
  /**
   * Que fraccion del carro se lleva quien derrota a una columna en campo abierto (Doc 5.12.3). **La mitad**:
   * dejarle algo es lo que hace que valga la pena seguir el viaje en vez de reiniciarlo, y lo que distingue un
   * robo de una ruina. Con el carro vacio no hay botin; solo quedan los heridos (`HEROE.heridoMinutos`).
   *
   * Vale igual para una columna personal que para un Ejercito, cuyo carro es el de todos sus miembros. Las
   * caravanas adjuntas no entran: esas se pierden aparte, si el ejercito se deshace (Doc 5.13.2).
   */
  fraccionRobada: 0.5,
};

/**
 * Niebla de guerra (Doc 5.12.7 y Doc 6 §12): hasta dónde alcanza la vista de cada cosa, en unidades de MAPA.
 *
 * Los dos radios viven juntos porque **solo se entienden comparados**: la relación entre ellos es la regla de
 * juego, no cada cifra por su lado.
 */
export const VISION = {
  /**
   * Campo de visión de un ejército en marcha.
   *
   * 150 sobre un mapa de 2000 es ~2 radios de provincia (~76, ver `ESCALA` y Doc 1.0a): un ejército ve
   * VARIAS ciudades por delante si la geografía lo permite, que es lo que se pedía. Y queda holgadamente por
   * debajo del radio de cohesión de un reino (`MANTENIMIENTO.escalaDistancia` = 400, ~5 provincias), así que
   * ver no equivale a controlar.
   *
   * Se descartó 30 —el radio de una zona de influencia recién fundada— porque bajo la escala rota parecía
   * razonable y con la escala declarada no llega ni al borde de la propia provincia.
   */
  ejercito: 150,
  /**
   * Lo que ve un jugador SOLO, sin tropa (Doc 1.10). **80**, por debajo de los 150 de una columna: un hombre
   * solo no despliega batidores.
   *
   * La tension que fija el numero: bastante para viajar sin caer en emboscadas a ciegas —que es lo que hace
   * jugable el primer minuto de partida—, poco para que el explorador solitario sea la mejor unidad de
   * informacion del juego. Sigue por encima del anillo de inspeccion (40), asi que ver y mirar de cerca
   * siguen siendo cosas distintas tambien para el.
   */
  jugadorSolo: 80,
  /**
   * Cuánto ve un asentamiento MÁS ALLÁ de su radio de influencia (decisión del usuario, 2026-09-04): la plaza
   * vigila algo más allá de su frontera, como una atalaya. **60**, por tres razones:
   *
   * - **Menos que la vista de un ejército** (150), que es lo que mantiene el valor de explorar: una columna en
   *   marcha divisa una plaza mucho antes de que la plaza la divise a ella, y conserva la iniciativa.
   * - **Duplica el área vigilada de una plaza recién fundada**, cuya zona ronda 30-60. El margen se nota desde
   *   el primer minuto en vez de ser un detalle que solo importa tarde.
   * - Coincide con `LOGISTICA.radioReabastecimiento`, y esa coincidencia se lee bien: **si una columna está lo
   *   bastante cerca como para repostar en tu ciudad, tu ciudad la ve.**
   *
   * Consecuencia anotada para calibración (§2.1 del doc de niebla): como el radio crece con el nivel
   * (60 -> 180, `ZONA_INFLUENCIA.radioMaximoPorNivel`), una plaza de nivel 5 vigila 240 y por tanto ve más
   * lejos que un ejército. Es coherente con la ficción, pero diluye la ventaja de explorar cerca de las
   * grandes ciudades.
   */
  margenAsentamiento: 60,
};

/**
 * Discretización de lo EXPLORADO (niebla de guerra, Paso 2 — ver `engine/exploracion.ts`). Lo que una Facción
 * ha llegado a ver alguna vez es un área, y un área hay que trocearla para poder guardarla.
 *
 * **25** unidades de mapa por celda, que es un compromiso entre dos cosas medibles:
 *
 * - **Cómo se lee la frontera de la niebla.** Lo que vigila una plaza recién fundada (30 de radio + 60 de
 *   margen = 90) son ~3,6 celdas de radio, y la vista de un ejército (150), 6. Con celdas más gruesas la
 *   frontera se leería como un cuadrado en vez de como una forma.
 * - **Lo que ocupa en el snapshot.** Sobre el mundo de 2000 salen 80x80 = 6.400 celdas, o sea 800 bytes por
 *   Facción — 24 KB con las 30 del laboratorio, frente a los 125 KB que ya ocupa el mapa. Doblar la
 *   resolución multiplicaría eso por cuatro.
 */
export const EXPLORACION = {
  tamanoCelda: 25,
};

/**
 * Campamentos de bandidos (Doc 1.9, a petición del usuario — inspirado en análisis comparativo con Travian):
 * amenaza NPC en bosques no reclamados que ataca caravanas cercanas. Todas las cifras son PLACEHOLDER, sin
 * calibrar por simulación todavía (ver `Preguntas_Abiertas.md` #14c) — mismo criterio que el resto del proyecto.
 */
/**
 * Alijos de exploración (D29, D60-D63): por cada campamento de mercenarios, `cuantos` alijos de cada banda de distancia al campamento,
 * con su oro (≈30 por héroe en total, D62). La banda lejana llega hasta la mitad de la separación entre campamentos: cae en los
 * huecos. PLACEHOLDER.
 */
export const ALIJOS = {
  bandas: [
    { desde: 100, hasta: 200, cuantos: 3, oro: 3 },
    { desde: 200, hasta: 300, cuantos: 2, oro: 5 },
    { desde: 300, hasta: 450, cuantos: 1, oro: 11 },
  ],
  /** Se mezcla con la seed del mapa: semilla derivada (D35). */
  salSemilla: 0x616c696a,
};

export const CAMPAMENTOS_BANDIDOS = {
  /** Niveles (D21, D37, §8.1): salen al azar con su `peso`. `poder` es contra lo que se tira con números; `unidades`, la milicia
   * de lanceros que pone en una batalla de Unity (poderBase 2: la mitad del poder); `oroPorHeroe`, el botín de cada héroe de la
   * columna que lo destruye (D22: solo oro). PLACEHOLDER. */
  niveles: {
    1: { poder: 20, unidades: 10, oroPorHeroe: 9, peso: 0.5 },
    2: { poder: 60, unidades: 30, oroPorHeroe: 10, peso: 0.3 },
    3: { poder: 120, unidades: 60, oroPorHeroe: 11, peso: 0.2 },
  } as const,
  /** Rendimientos decrecientes por héroe (D26, §8.1): en una ventana de 24 h, botín completo en las primeras `completas`
   * destrucciones, luego `caidaPorCada` menos por cada una, y desde la `soloExperienciaDesde`ª nada de oro. */
  rendimientos: { ventanaHoras: 24, completas: 8, caidaPorCada: 0.15, soloExperienciaDesde: 14 },
  /** Los del anillo de un campamento de mercenarios (D42, D28, §8.2): a esta distancia, uno por cada `residentesPorBandido`
   * residentes de Facciones sin asentamiento, entre `minimo` y `maximo`, y aparece uno cada `reaparicionMinutos` mientras falten. */
  anillo: { radioMin: 150, radioMax: 250, residentesPorBandido: 2, minimo: 1, maximo: 6, reaparicionMinutos: 10 },
  // Radio (unidades del mapa) dentro del cual un campamento ataca a una caravana que pase cerca.
  radioAtaqueCaravana: 40,
  // Minutos de mundo tras destruirse un campamento hasta que reaparece el de ese asentamiento (Doc 1.9). Vuelve a 60
  // (2026-09-28, decisión del usuario): se bajó a 10 cuando los tiempos del servidor eran más cortos.
  respawnMinutos: 60,
};

/**
 * Regeneración de yacimientos agotados (a petición del usuario): un `NodoRecurso` que llega a stock 0
 * (`Mapa.stock`, ver `world/mapa.ts`) vuelve a aparecer con su `cantidadInicial` completa pasados N ticks —
 * mismo patrón que la reaparición de campamentos de bandidos (`CAMPAMENTOS_BANDIDOS.respawnMinutos`). Dos
 * cadencias: `livestock` (fauna, se recupera por reproducción/migración) más rápido que `metales` (todo el
 * resto de nodos: piedra/cobre/estaño/oro — yacimientos minerales, se repone más despacio). Bajadas de
 * 200/100 a 10/3 (pruebas del usuario) — a las cifras viejas, agotar un nodo dejaba al extractor parado
 * demasiado tiempo frente al resto del ritmo del juego ya recalibrado esta sesión.
 * Cifras PLACEHOLDER, sin calibrar por simulación todavía, mismo criterio que el resto del proyecto.
 */
export const REGENERACION_NODOS = {
  metales: { cooldownMinutos: 10 },
  livestock: { cooldownMinutos: 3 },
};

// --- Sprint 6: Cierre (Doc 4.5 mantenimiento, Doc 2.7 reputación, Doc 2.9 progresión) ---

/**
 * Nivel de asentamiento — rediseño de progreso (Fase 0): reemplaza por completo la fórmula de puntos anterior
 * (población/edificios activos). Ahora es un modelo de GATES: para subir de nivel hace falta cumplir a la vez
 * un mínimo de población (pesants + artesanos) Y tener construidos (activos) al menos `edificiosMinimo`
 * tipos DISTINTOS del conjunto `edificios` (si se omite, por defecto son TODOS — mismo comportamiento que
 * antes de Doc Fase_0_6). El nivel sube de forma MONÓTONA (nunca baja si la población cae después).
 *
 * Tope subido de nivel 3 a nivel 5 (Doc Fase_0_6, a petición del usuario) — reestructura completa de qué pide
 * cada escalón:
 * - Nivel 2: ≥3 de los 6 tipos de EXTRACCIÓN (antes pedía los 3 de transformación) — mismo criterio de
 *   "variedad, no instancias repetidas" que ya usaba el cupo de asentamientos (Fase_0_5).
 * - Nivel 3: los 3 de TRANSFORMACIÓN + los 2 MILITARES a la vez (antes solo pedía carpintería+barracón+
 *   galería). Como ambos subconjuntos exigen "todos los suyos", `edificiosMinimo` se omite (por defecto pide
 *   los 5 completos) — ver engine/mantenimiento.ts `calcularNivelAsentamiento`.
 * - Nivel 4 (nuevo): Muralla construida.
 * - Nivel 5 (nuevo, tope): Palacio construido.
 * Población de niveles 4/5 PLACEHOLDER, continuando la progresión de 2/3 (pesants ×2, artesanos ×2) —
 * pendiente de calibrar por simulación, igual que el resto de cifras de esta fase.
 */
export const NIVEL_ASENTAMIENTO = {
  nivelMaximo: 5,
  requisitos: {
    // Sin artesanos (Doc Fase_0_6, deadlock real detectado por el usuario): Artesanos no aparece hasta el
    // primer edificio de transformación activo (engine/population.ts) — pero Fundición/Curtidería/Armería
    // ahora exigen nivel 2 para construirse (ver §2 de Fase_0_6). Pedir artesanos aquí también habría hecho
    // el gate imposible de cumplir: sin nivel 2 no hay transformación, sin transformación no hay artesanos,
    // sin artesanos no hay nivel 2. Pesants se mantiene — no depende de ningún edificio de nivel 2.
    // `granja` se sumó a la lista (3 de 7, no 3 de 6): un asentamiento con cantera+lenera+granja debía poder
    // subir de nivel con solo esos tres (caso real detectado por el usuario) sin depender de tener un
    // recurso raro (cobre/estaño/oro/livestock) alcanzable, que muchos asentamientos nunca tienen cerca.
    2: {
      pesants: 200,
      artesanos: 0,
      edificios: ['cantera', 'lenera', 'granja', 'mina', 'minaCobre', 'minaEstano', 'corral'],
      edificiosMinimo: 3,
    },
    // Artesanos ×2 el 2026-09-27 (decisión del usuario), al doblar su sitio en las Viviendas.
    3: { pesants: 500, artesanos: 400, edificios: ['armeria', 'curtiduria', 'fundicion', 'barracon', 'galeriaDeTiro'] },
    // Sustituye al viejo `edificios: ['muralla']` (Paso 5, `Consideraciones/Murallas_Definicion.md` §13): el
    // recinto ya no es un `EdificioTipo`, así que el gate deja de poder contarlo como edificio y pasa a
    // `recintoCompletoNivelMinimo` — cualquier recinto TERMINADO (integridad 1) de nivel 1 en adelante basta,
    // la empalizada barata cuenta igual que la muralla de piedra. `edificios: []` es intencional, no un
    // descuido: sin ningún tipo en la lista, `cumpleEdificios` es trivialmente cierto y el gate real es el
    // del recinto.
    // Doc 4.5: recinto completo DE PIEDRA (nivel ≥ 2: muro de piedra o muralla con adarve) y la Sala del Consejo.
    4: { pesants: 1000, artesanos: 800, edificios: ['salaConsejo'], recintoCompletoNivelMinimo: 2 },
    5: { pesants: 2000, artesanos: 1600, edificios: ['palacio'], nivelInternoMinimo: { palacio: 3 } },
  } as Record<
    number,
    {
      pesants: number;
      artesanos: number;
      edificios: string[];
      edificiosMinimo?: number;
      recintoCompletoNivelMinimo?: number;
      /** Nivel interno mínimo que tiene que tener un edificio de la lista (el Palacio 3 del nivel 5, Doc 4.5). */
      nivelInternoMinimo?: Partial<Record<string, number>>;
    }
  >,
  /**
   * Techo de POBLACIÓN TOTAL (pesants+artesanos+nobleza) por nivel (Doc Fase_0_5 §3.1, a petición del
   * usuario) — por encima de este número, la Vivienda/Palacio dejan de dar cupo efectivo aunque tengan
   * capacidad física de sobra: el nivel pasa a ser lo que abre el techo de habitantes, no solo una llave de
   * edificios. Pieza central del rediseño de dependencia: un nivel 3 en su techo (6000 hab) consume 600
   * trigo/minuto contra un techo agrícola real de 150-450 (ver diagnóstico en Fase_0_5_Definicion...md §1) —
   * no puede sostenerse sin importar. Ligado a `asentamiento.nivel` (nivelAlcanzado, nunca baja — Doc §6.2:
   * un fallo de suministro NUNCA purga población ya asentada, solo congela capacidad de construir/mejorar).
   * Niveles 4/5 (Doc Fase_0_6): desacelera el múltiplo anterior (×5, ×4) a ×2/×1.7, asumiendo que son
   * niveles de "cierre de partida" más que de crecimiento bruto. Cifras PLACEHOLDER sin calibrar por
   * simulación.
   */
  techoPoblacion: { 1: 300, 2: 1500, 3: 6000, 4: 12000, 5: 20000 } as Record<number, number>,
};

/**
 * Subida de nivel de asentamiento MANUAL y con coste (2026-09-26, decisión del usuario —
 * `Consideraciones/Ritmo_Crecimiento_Asentamientos.md` §10, engine/ascenso.ts). Los gates de
 * `NIVEL_ASENTAMIENTO.requisitos` dejan de subir el nivel solos: son el requisito para que el Gobernador pueda
 * PEDIR la subida, que además exige pagar este coste, esperar esta obra y ser solvente en el nivel objetivo.
 *
 * Criterio de las cifras: el coste pide lo que el nivel siguiente va a cobrar (piedra y oro, que el
 * mantenimiento empieza a cobrar en el nivel 2) y, desde el nivel 3, un bien elaborado de la cadena del nivel
 * que se deja; cada coste cabe en el almacén máximo del nivel de partida (400 + 300 × `maximoAlmacenesPorNivel`:
 * 1600 / 2800 / 5200 / 7600 por recurso). La obra es la palanca de ritmo que no depende de la economía: pone
 * más o menos la mitad del tiempo objetivo de cada nivel de D49 (el resto lo tiene que poner la calibración).
 *
 * El 4 → 5 pedirá además bienes de lujo si se aprueban los talleres de lujo (pendiente, revisión de BA-006).
 * PLACEHOLDER: punto de partida para calibrar con el batch.
 */
export const ASCENSO_ASENTAMIENTO = {
  porNivelObjetivo: {
    2: { costo: { madera: 600, piedra: 400, oro: 150 }, obraMinutos: 4_320 }, // 3 días
    // Lingotes de bronce (D54, techo por Era derivado): la aleación pide estaño, que casi siempre llega por comercio,
    // así que el nivel 3 es raro en la Era I y la meseta llega en la Era II.
    3: { costo: { madera: 1200, piedra: 1000, oro: 400, lingoteBronce: 100 }, obraMinutos: 10_080 }, // 1 semana
    // Además pide la Sala del Consejo (`NIVEL_ASENTAMIENTO`), que pide `instituciones_civicas` (Era III, D54).
    4: { costo: { madera: 2500, piedra: 2500, oro: 1000, lingoteBronce: 150 }, obraMinutos: 20_160 }, // 2 semanas
    5: { costo: { madera: 5000, piedra: 5000, oro: 2500 }, obraMinutos: 30_240 }, // 3 semanas
  } as Record<number, { costo: Partial<Record<string, number>>; obraMinutos: number }>,
  /**
   * Techo PROVISIONAL en el nivel 4: el 4 → 5 lo tiene que desbloquear una tecnología de la Era V (D54), que aún no
   * existe. El 3 → 4 ya lo derivan la Sala del Consejo y `instituciones_civicas`. Se quita al implementar la Era V.
   */
  nivelTechoProvisional: 4,
};

/**
 * Mantenimiento (Doc 4.5): coste periódico que escala por nivel (sumando materiales, no reemplazando) y por
 * distancia al centro de poder de la Facción (su capital, Doc 2.2).
 * Cantidades y velocidad de degradación son PLACEHOLDER (Preguntas_Abiertas no fija cifras exactas).
 */
export const MANTENIMIENTO = {
  medidorInicial: 100,
  // Trigo NO va aquí (fix: era una "mecánica repetida" — Mantenimiento cobraba este valor fijo ADEMÁS del
  // consumo real de comida que ya se descuenta en `avanzarNutricionPoblacion`/`avanzarMantenimientoTropas`, duplicando
  // el gasto). El "apartado de trigo" que se muestra en el panel de Mantenimiento ahora es la suma real de
  // consumo de población + tropas (ver `gameStore.mantenimientoInfo`), no un placeholder desconectado.
  /**
   * Mantenimiento base en madera por minuto, antes de escalar por población y distancia a la capital.
   *
   * **A LA MITAD desde el 2026-09-04 (3 → 1.5, decisión del usuario).** El motivo, medido: era lo que
   * inflaba la reserva de construcción hasta hacerla infranqueable. La reserva de un recurso es su
   * mantenimiento × `RESERVA_CONSTRUCCION.horizonteMinutosMantenimiento`, así que una ciudad madura del batch
   * llegaba a exigirse **194 de madera guardada** mientras acumulaba **142** — y con eso
   * `puedeIniciarConstruccion` le vetaba CUALQUIER gasto discrecional para siempre. Resultado: cero Almacenes
   * y cero Graneros en 600 ticks, aunque hubiera madera entrando (ver
   * `Consideraciones/Movimiento_Ejercitos_Definicion.md` §11.1).
   *
   * No era un problema de producción sino del techo que el propio mantenimiento se imponía: la reserva escala
   * con el mantenimiento, que escala con lo construido, así que cuanto más crecía la ciudad menos podía
   * construir. Se eligió esta palanca sobre las otras tres (bajar el horizonte de reserva, subir el rinde de
   * la Leñera, eximir al almacenaje de la reserva) por ser la que ataca la causa y no el síntoma.
   */
  costoBase: { madera: 1.5 },
  // Rediseño de progreso (Fase 0): con el tope de nivel bajando de 10 a 3 (ver NIVEL_ASENTAMIENTO), los
  // umbrales de piedra/oro (antes nivel 3 y nivel 8, pensados para un rango 1-10) se recalibran al rango 1-3
  // para que los 3 niveles tengan una escalada de coste real — cifra exacta PLACEHOLDER pendiente de
  // calibración por simulación (ver Preguntas_Abiertas.md).
  nivelParaPiedra: 2,
  piedraBase: 3,
  // Vuelve a 3 (2026-09-26, decisión del usuario — `Consideraciones/Ritmo_Crecimiento_Asentamientos.md` §10.2).
  // Se había bajado a 2 porque el NPC casi nunca llegaba a nivel 3 y el oro era letra muerta; eso eran bugs ya
  // arreglados. Medido con la subida manual: en nivel 2 el oro es estructuralmente negativo — 200 pesants
  // recaudan 0,84/min contra 2,84/min de mantenimiento — y era la causa del 97 % de los colapsos (antes) y de que
  // nadie pudiera pedir la subida (después, por la prueba de solvencia). En el gate del nivel 3 ya hay 200
  // artesanos: 500 × 0,004 + 200 × 0,015 = 5/min de recaudación contra ~4,8/min junto a la capital; lejos, el
  // factor de distancia exige mina o comercio. PLACEHOLDER.
  nivelParaOro: 3,
  oroBase: 2,
  // El mantenimiento es de los EDIFICIOS (2026-09-27, decisión del usuario): escala con los activos, no con la
  // población, que ya paga lo suyo comiendo. `factorEdificios = 1 + edificiosActivos / edificiosReferencia`.
  // PLACEHOLDER: a 50, una plaza de nivel 2 con ~90 edificios paga casi lo mismo que pagaba por población (factor
  // 2,8 frente a 3 con 1.000 habitantes) y una de nivel 1 con ~36, algo más (1,7 frente a 1,4).
  edificiosReferencia: 50,
  /** Una Vivienda cuenta la mitad que otro edificio (2026-09-28, decisión del usuario): son la mayoría de los de una
   * plaza, unos 40 de los ~90 de un nivel 2. */
  pesoVivienda: 0.5,
  /** El coste crece 1 por cada `escalaDistancia` de distancia a la capital, sin tope (decidido el 2026-10-02): ×2 a 400, ×3 a 800... */
  escalaDistancia: 400,
  degradacionPorDeficitTotal: 10,
  regeneracionSiPagoCompleto: 5,
  // Protección temporal a asentamientos recién fundados (Doc 1.3, pendiente en el diseño): sin esto, todo
  // asentamiento nuevo entra en déficit desde el tick 1 (antes de que la Granja llegue a construirse) y cae
  // en ruinas pase lo que pase. La gracia cubre el tiempo típico de estabilizar la economía base.
  // 1 día desde el 2026-09-26 (decisión del usuario): con los tiempos de obra pasando de minutos a horas
  // (`Ritmo_Crecimiento_Asentamientos.md` §11), la Leñera ya no está en pie en la primera hora.
  graciaMinutos: 1440,
  // nivelActual (Doc Fase_0_5 §6.2, rediseño a petición del usuario): al tocar 0 el medidor, `nivelActual`
  // baja un escalón (nivel 3→2→1→ruinas, solo cae en ruinas ya en nivelActual 1) EN VEZ de destruirse
  // directamente, y el medidor se reinicia a `medidorInicial` — el asentamiento sigue vivo y produciendo,
  // solo pierde capacidad de construir/mejorar/reclutar de nivel alto hasta recuperarse. Solo vuelve a subir
  // tras `minutosSanosParaRecuperarNivel` ticks SEGUIDOS con mantenimiento pagado en full (no cada tick que
  // esté sano) — evita el yo-yo de subir/bajar por un solo bache. Placeholder sin calibrar por simulación.
  minutosSanosParaRecuperarNivel: 30,
};

/**
 * Reserva mínima que la auto-construcción (y la manual) NUNCA puede tocar al COMPROMETER (pagar) un proyecto
 * nuevo (ver `puedeIniciarConstruccion`/`evaluarNecesidades` en engine/construction.ts). Overhaul: reemplaza
 * los umbrales fijos anteriores (madera 30, trigo 20, piedra 20, oro 10 — pensados para el asentamiento
 * inicial, sin escalar nunca) por una reserva PROYECTADA: cuánto va a cobrar Mantenimiento/comida en los
 * próximos N ticks (ver `reservaDinamicaConstruccion`, engine/mantenimiento.ts, que reutiliza
 * `calcularCostoMantenimiento`). Corrige un colapso real documentado: tras subir de nivel (piedra/oro
 * entran a cobrarse) la reserva fija se quedaba corta frente al mantenimiento real ya escalado. Cifras de
 * horizonte PLACEHOLDER (ver Preguntas_Abiertas.md), calibradas contra `FUNDACION.materialesIniciales`: con
 * el stock inicial de madera (50) y el costo base de Mantenimiento a nivel 1 (3/minuto), un horizonte de 15
 * ticks reservaría 45 — casi todo el stock inicial, congelando cualquier construcción no exenta (Vivienda,
 * extractores...) durante los primeros ticks incluso en un asentamiento sano con acceso a bosque. 8 ticks dan
 * un margen real (protege contra el patrón de colapso post-ascenso de nivel ya documentado) sin dejar al
 * asentamiento recién fundado sin margen de maniobra.
 *
 * Excepción deliberada (sin cambios): Granja no respeta la reserva de trigo, ni Leñera la de madera (ver
 * `engine/construction.ts`) — son las únicas vías reales de recuperar esos recursos, así que bloquearlas
 * por la misma escasez que deben resolver sería un huevo-y-la-gallina sin salida.
 */
export const RESERVA_CONSTRUCCION = {
  horizonteMinutosMantenimiento: 8,
  horizonteMinutosComida: 8,
};

/**
 * Reputación de Facción (Doc 2.7): score público -100..+100, decae hacia 0 sin eventos nuevos.
 * Cifras exactas de cada evento sin cerrar en el diseño (Preguntas_Abiertas) — valores placeholder razonables.
 */
export const REPUTACION = {
  decaimientoPorMinuto: 0.2,
  bonusTruequeCumplido: 5,
  penalizacionTruequeIncumplido: -8,
  bonusLiberarVasalloVoluntario: 6,
  penalizacionRebelionParaSenora: -10,
  penalizacionRomperAlianza: -12,
  penalizacionAtacarAliado: -25,
  bonusPorMinutoAlianzaActiva: 0.05,
  // Restricción del Embajador (Doc 2.7, uso 3): por debajo de este umbral no puede proponer alianzas.
  umbralBajoParaEmbajador: -40,
  // Términos de comercio asimétricos (Doc 2.7, uso 1): score bajo encarece la comisión que paga esa Facción.
  umbralBajoParaComision: -40,
  factorComisionPorReputacionBaja: 1.5,
};

// --- Tecnología por Eras (Doc 6) ---

/** Eras con contenido (Doc 6.2). El plazo es un TECHO: la Era siguiente llega antes si se cumplen todos sus logros. */
export const ERAS: Record<EraId, { orden: number; nombre: string; plazoSemanas: number }> = {
  reinos_palaciales: { orden: 1, nombre: 'Reinos palaciales', plazoSemanas: 5 },
  crisis_adaptacion: { orden: 2, nombre: 'Crisis y adaptación', plazoSemanas: 6 },
  polis_imperios: { orden: 3, nombre: 'Polis e imperios', plazoSemanas: 7 },
};

/** Lo que cuesta adoptar una tecnología según su Era (Doc 6.5); lo paga el almacén de la capital. Placeholder. */
export const TARIFA_ADOPCION: Record<EraId, Partial<Record<RecursoTipo, number>>> = {
  reinos_palaciales: { oro: 100, madera: 200 },
  crisis_adaptacion: { oro: 300, lingoteBronce: 30 },
  polis_imperios: { oro: 600, lingoteHierro: 30 },
};

/** Una condición del hito de la Facción (Doc 6.3). "Edificio" = activo en cualquier asentamiento de la Facción;
 * `nivelInterno` es un mínimo. */
export type CondicionHito =
  | { tipo: 'edificio'; edificio: EdificioTipo; nivelInterno?: number }
  | { tipo: 'tecnologia'; id: TecnologiaId }
  | { tipo: 'recursoEnCapital'; recurso: RecursoTipo }
  | { tipo: 'capitalEnNivel'; nivel: number; conEdificio: EdificioTipo }
  | { tipo: 'yacimientoEnTerritorio'; recurso: RecursoTipo };

export interface DefinicionTecnologia {
  nombre: string;
  era: EraId;
  /** Las de arranque (Doc 6.2) las tiene adoptadas toda Facción desde que nace: sin logro ni hito. */
  deArranque?: true;
  /** `umbral` es la X del logro: PLACEHOLDER hasta calibrarlo con batch en su semana objetivo (Doc 6.3). */
  logro?: { contador: ContadorLogro; umbral: number };
  hito: CondicionHito[];
  /** Multiplica lo que sacan los extractores de la Facción de ese recurso (Doc 6.6, Cantería). */
  bonusProduccion?: { recurso: RecursoTipo; factor: number };
}

const hitoEdificio = (e: EdificioTipo, nivelInterno?: number): CondicionHito => ({ tipo: 'edificio', edificio: e, nivelInterno });
const hitoTecnologia = (id: TecnologiaId): CondicionHito => ({ tipo: 'tecnologia', id });

/** Catálogo de las Eras I-III (Doc 6.6). */
export const TECNOLOGIAS: Record<TecnologiaId, DefinicionTecnologia> = {
  // Era I — Reinos palaciales
  leva_comunal: { nombre: 'Leva comunal', era: 'reinos_palaciales', deArranque: true, hito: [] },
  hostigamiento_tribal: { nombre: 'Hostigamiento tribal', era: 'reinos_palaciales', deArranque: true, hito: [] },
  metalurgia_cobre: {
    nombre: 'Metalurgia del cobre',
    era: 'reinos_palaciales',
    logro: { contador: 'extraido.cobre', umbral: 41_000 },
    hito: [hitoEdificio('fundicion')],
  },
  aleacion_bronce: {
    nombre: 'Aleación del bronce',
    era: 'reinos_palaciales',
    // Estaño extraído en el mundo y no caravanas con estaño (2026-10-01, decisión del usuario): ese contador solo decía que
    // se comerciaba, y con estaño en casa casi nadie lo compra.
    logro: { contador: 'extraido.estano', umbral: 245_000 },
    hito: [hitoEdificio('fundicion', 2), { tipo: 'recursoEnCapital', recurso: 'estano' }],
  },
  escudos_ligeros: {
    nombre: 'Escudos ligeros',
    era: 'reinos_palaciales',
    logro: { contador: 'bandidos.campamentosDestruidos', umbral: 2_250 },
    hito: [hitoEdificio('barracon')],
  },
  armamento_palacial: {
    nombre: 'Armamento palacial',
    era: 'reinos_palaciales',
    logro: { contador: 'plazasEnNivel.2', umbral: 12 },
    hito: [hitoEdificio('armeria'), hitoTecnologia('metalurgia_cobre')],
  },
  arqueria_palacial: {
    nombre: 'Arquería palacial',
    era: 'reinos_palaciales',
    logro: { contador: 'batallas.libradas', umbral: 100 },
    hito: [hitoEdificio('galeriaDeTiro')],
  },
  cria_caballar: {
    nombre: 'Cría caballar',
    era: 'reinos_palaciales',
    logro: { contador: 'animales.comprados', umbral: 700 },
    hito: [hitoEdificio('corral')],
  },
  carros_guerra: {
    nombre: 'Carros de guerra',
    era: 'reinos_palaciales',
    logro: { contador: 'batallas.campoAbierto', umbral: 69 },
    hito: [hitoTecnologia('cria_caballar'), hitoEdificio('caballerizas', 2), hitoEdificio('carpinteria', 2)],
  },
  // Sin tropa ni receta detrás: la piedra es lo que frena el nivel 3 en la Era I medida (2026-09-30, decisión del
  // usuario). Sin semana en la tabla de BA-006: se puso en la semana 2, antes de que la piedra apriete.
  canteria: {
    nombre: 'Cantería',
    era: 'reinos_palaciales',
    logro: { contador: 'extraido.piedra', umbral: 11_000_000 },
    hito: [hitoEdificio('cantera')],
    bonusProduccion: { recurso: 'piedra', factor: 1.5 },
  },
  // Era II — Crisis y adaptación
  bronce_calidad_militar: {
    nombre: 'Bronce de calidad militar',
    era: 'crisis_adaptacion',
    logro: { contador: 'fabricado.equipoBronce', umbral: 5_900 },
    hito: [hitoEdificio('armeria', 3)],
  },
  forja_hierro_temprana: {
    nombre: 'Forja del hierro temprana',
    era: 'crisis_adaptacion',
    logro: { contador: 'caravanas.destruidasOCapturadas', umbral: 11_000 },
    hito: [hitoEdificio('fundicion', 2), { tipo: 'yacimientoEnTerritorio', recurso: 'hierro' }],
  },
  panoplia_bronce: {
    nombre: 'Panoplia de bronce',
    era: 'crisis_adaptacion',
    logro: { contador: 'reclutados.escuadrones', umbral: 1_200 },
    hito: [hitoTecnologia('bronce_calidad_militar'), hitoEdificio('barracon', 3)],
  },
  disciplina_formacion: {
    nombre: 'Disciplina de formación',
    era: 'crisis_adaptacion',
    logro: { contador: 'asedios.resistidosEnCombate', umbral: 140 },
    hito: [hitoEdificio('barracon', 2)],
  },
  arco_compuesto: {
    nombre: 'Arco compuesto',
    era: 'crisis_adaptacion',
    logro: { contador: 'reclutados.arqueros', umbral: 350 },
    hito: [hitoEdificio('carpinteria', 2), hitoEdificio('galeriaDeTiro', 3)],
  },
  equitacion_militar: {
    nombre: 'Equitación militar',
    era: 'crisis_adaptacion',
    // Soldados de caballería reclutados, el de un carro de guerra por cinco de jinete (2026-10-01, decisión del usuario): el
    // logro por carros solos casi no se movía, porque son de escalón 4 (Nobleza) y piden Carpintería 2.
    logro: { contador: 'reclutados.caballeria', umbral: 1_800 },
    hito: [hitoEdificio('caballerizas')],
  },
  carpinteria_militar: {
    nombre: 'Carpintería militar',
    era: 'crisis_adaptacion',
    logro: { contador: 'conquistas.conMurallaCompleta', umbral: 1 },
    hito: [hitoEdificio('carpinteria', 2)],
  },
  // Era III — Polis e imperios
  instituciones_civicas: {
    nombre: 'Instituciones cívicas',
    era: 'polis_imperios',
    logro: { contador: 'plazasEnNivel.3', umbral: 12 },
    hito: [{ tipo: 'capitalEnNivel', nivel: 3, conEdificio: 'mercado' }],
  },
  ciudadania_militar: {
    nombre: 'Ciudadanía militar',
    era: 'polis_imperios',
    logro: { contador: 'asedios.resistidosConResidentes', umbral: 158 },
    hito: [hitoTecnologia('instituciones_civicas'), hitoEdificio('barracon', 2)],
  },
  falange_hoplita: {
    nombre: 'Falange hoplita',
    era: 'polis_imperios',
    logro: { contador: 'batallas.conHoplitas', umbral: 10 },
    hito: [hitoTecnologia('ciudadania_militar'), hitoEdificio('barracon', 3)],
  },
  pantalla_escaramuzadores: {
    nombre: 'Pantalla de escaramuzadores',
    era: 'polis_imperios',
    logro: { contador: 'reclutados.escaramuzadores_jabalina', umbral: 1_240 },
    hito: [hitoEdificio('galeriaDeTiro', 2), hitoEdificio('armeria', 2)],
  },
  arqueria_especializada: {
    nombre: 'Arquería especializada',
    era: 'polis_imperios',
    logro: { contador: 'reclutados.arqueros_compuesto', umbral: 120 },
    hito: [hitoEdificio('galeriaDeTiro', 3)],
  },
  forja_hierro_estandarizada: {
    nombre: 'Forja del hierro estandarizada',
    era: 'polis_imperios',
    logro: { contador: 'extraido.hierro', umbral: 1_380_000 },
    hito: [hitoEdificio('fundicion', 2), hitoEdificio('minaHierro')],
  },
  bronce_laminado: {
    nombre: 'Bronce laminado',
    era: 'polis_imperios',
    logro: { contador: 'fabricado.armaduraBronce', umbral: 200 },
    hito: [hitoEdificio('armeria', 3)],
  },
  caballeria_organizada: {
    nombre: 'Caballería organizada',
    era: 'polis_imperios',
    logro: { contador: 'reclutados.jinetes_asirios', umbral: 1_700 },
    hito: [hitoEdificio('caballerizas', 2)],
  },
  trabajos_asedio: {
    nombre: 'Trabajos de asedio',
    era: 'polis_imperios',
    logro: { contador: 'asedios.contraMurallaCompleta', umbral: 10 },
    hito: [hitoEdificio('carpinteria', 2)],
  },
};
