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
 */
export const BALANCE_VERSION = 5;

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
  artesanos: { tasaCrecimientoBase: 0.1 },
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
    // Fracción de pesants+artesanos (nobleza protegida) perdida por minuto mientras la nutrición sigue en el
    // umbral — mismo valor que la deserción de tropas sin moral, por coherencia entre ambos sistemas.
    fraccionMuertePorMinutoHambre: 0.05,
  },
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
}

/** Un nivel interno de un edificio de transformación con tiers (Fundición/Curtiduría/Armería/Carpintería/
 * Barracón/Galería de tiro, Doc 4.2.1). El nivel 1 no lleva `costoMejora`/gates (ya se pagaron al construir). */
interface NivelEdificioTransformacion {
  trabajadoresRequeridos: number;
  recetas: RecetaProduccion[];
  costoMejora?: Partial<Record<string, number>>;
  requisitoNivelAsentamiento?: number;
  requiereEdificio?: string;
  requiereEdificioNivel?: number;
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
  /** Solo Granja: su huella en la rejilla CRECE con el nivel interno (1x1 → 6x6), a diferencia del resto de
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

export const EDIFICIO_CATALOGO = {
  // Único edificio que NO pasa por la cola de construcción (ni automática ni manual, Doc 1.3): nace
  // ya activo al fundar. costo/tiempoConstruccionMinutos quedan en 0 solo por consistencia de forma con
  // el resto del catálogo — nunca se leen, porque construirlo por otra vía no es posible.
  centroUrbano: { costo: {}, tiempoConstruccionMinutos: 0 },
  // Cupos SEPARADOS por clase (a petición del usuario, ver Correcciones): antes un único pool compartido
  // entre Pesants y Artesanos hacía que Pesants (crece ~2.4x más rápido) acaparara todo el cupo y dejara a
  // Artesanos varado — cada Vivienda ahora aporta 15 espacios de Pesants Y, por separado, 5 de Artesanos.
  vivienda: { costo: { madera: 10 }, tiempoConstruccionMinutos: 4, capacidadPesants: 15, capacidadArtesanos: 5 },
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
    tiempoConstruccionMinutos: 6,
    produccionBaseTrigo: 60,
    trabajadoresRequeridos: 4,
    niveles: {
      1: { trabajadoresRequeridos: 4, recetas: [], produccionBaseTrigo: 60, tamano: { ancho: 4, alto: 4 } },
      // Piedra añadida a las mejoras (Doc Fase_0_6, a petición del usuario): antes 100% madera. Sin gate de
      // nivel de asentamiento — las 4 mejoras siguen alcanzables estando en nivel 1.
      2: { trabajadoresRequeridos: 4, recetas: [], produccionBaseTrigo: 90, tamano: { ancho: 4, alto: 6 }, costoMejora: { madera: 60, piedra: 20 } },
      3: { trabajadoresRequeridos: 4, recetas: [], produccionBaseTrigo: 120, tamano: { ancho: 8, alto: 6 }, costoMejora: { madera: 120, piedra: 40 } },
      4: { trabajadoresRequeridos: 4, recetas: [], produccionBaseTrigo: 180, tamano: { ancho: 12, alto: 12 }, costoMejora: { madera: 240, piedra: 80 } },
    } as Record<number, NivelEdificioTransformacion>,
  },
  cantera: { costo: { madera: 20 }, tiempoConstruccionMinutos: 5, produccionBasePiedra: 5, trabajadoresRequeridos: 4 },
  lenera: { costo: { madera: 10 }, tiempoConstruccionMinutos: 3, produccionBaseMadera: 5, trabajadoresRequeridos: 4 },
  // Sin piedra en la construcción BASE (Doc Fase_0_6, a petición del usuario): nivel 1 completo se paga solo
  // en madera — la piedra recién se introduce en nivel 2 (ver EDIFICIO_CATALOGO.fundicion/curtiduria/armeria).
  almacen: { costo: { madera: 50 }, tiempoConstruccionMinutos: 6, capacidadPorRecursoAdicional: 300 },
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
    tiempoConstruccionMinutos: 6,
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
  mina: { costo: { madera: 40 }, tiempoConstruccionMinutos: 6, produccionBaseOro: 4, trabajadoresRequeridos: 6 },
  // Cobre (Doc 1.1/5.7): "relativamente abundante" — igual patrón que cantera/mina pero sobre nodos de cobre.
  minaCobre: { costo: { madera: 30 }, tiempoConstruccionMinutos: 4, produccionBaseCobre: 5, trabajadoresRequeridos: 8 },
  // Estaño (Doc 1.1/5.7): raro y concentrado (menos nodos que cobre/oro, ver RECURSO_RAREZA.raro) — costo más
  // alto y producción base más baja que el resto de minas, coherente con ser el cuello de botella del bronce.
  // produccionBaseEstano subido de 1.5 a 3 (pruebas del usuario) — el estaño era el cuello de botella más
  // duro de la cadena de bronce, más de lo que el diseño original pretendía.
  minaEstano: { costo: { madera: 50 }, tiempoConstruccionMinutos: 7, produccionBaseEstano: 3, trabajadoresRequeridos: 8 },
  // Corral (Doc 4.2.1, rediseño de progreso Fase 0): extractor de livestock, mismo patrón que cantera/minas —
  // liga a un nodo finito de livestock (Doc 1.4), con reemplazo automático al agotarse (ver EXTRACCION_MAXIMOS).
  corral: { costo: { madera: 30 }, tiempoConstruccionMinutos: 6, produccionBaseLivestock: 3, trabajadoresRequeridos: 4 },
  // "Único edificio de tier élite, exclusivo de asentamientos/Facciones de mayor nivel" — gate por nivel de Facción.
  // Se mantiene sin cambios (Doc 4.2, rediseño de progreso): queda para iteraciones posteriores la integración
  // con la nueva Fundición.
  granFundicion: { costo: { madera: 150, piedra: 100, oro: 50 }, tiempoConstruccionMinutos: 20, nivelFaccionMinimo: 3 },

  // --- Edificios de transformación (Doc 4.2.1, rediseño de progreso Fase 0): auto-construcción (sin gate de
  // nivel para la construcción BASE — solo las mejoras de nivel interno lo exigen), disparan Artesanos (Doc
  // 4.1) y cuentan para los gates de nivel de asentamiento (ver NIVEL_ASENTAMIENTO). En Fase 0 no exigen
  // "Planos"/Aedas (Doc 6.5). Fundición reemplaza y amplía la Fundición manual anterior (antes sin producción). ---

  fundicion: {
    costo: { madera: 80, piedra: 40 },
    tiempoConstruccionMinutos: 6,
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
          { produce: 'lingoteEstano', produccionBase: 3, consumePorUnidad: { estano: 5 } },
          { produce: 'lingoteBronce', produccionBase: 1, consumePorUnidad: { lingoteCobre: 1.6, lingoteEstano: 0.4 } },
        ],
      },
    } as Record<number, NivelEdificioTransformacion>,
  },

  curtiduria: {
    costo: { madera: 80, piedra: 30 },
    tiempoConstruccionMinutos: 8,
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
    tiempoConstruccionMinutos: 6,
    // Doc Fase_0_6: construcción BASE gateada a nivel 2 (ver nota en `fundicion`).
    requisitoNivelAsentamientoConstruccion: 2,
    niveles: {
      1: {
        trabajadoresRequeridos: 4,
        recetas: [
          { produce: 'armaCobre', produccionBase: 3, consumePorUnidad: { lingoteCobre: 1, madera: 1 } },
          { produce: 'armaduraBasica', produccionBase: 3, consumePorUnidad: { cuero: 5 } },
          RECETA_ARMA_MADERA,
        ],
      },
      2: {
        requisitoNivelAsentamiento: 2,
        requiereEdificio: 'carpinteria',
        costoMejora: { madera: 150, piedra: 100 },
        trabajadoresRequeridos: 8,
        recetas: [
          { produce: 'armaCobre', produccionBase: 3, consumePorUnidad: { lingoteCobre: 1, madera: 1 } },
          { produce: 'armaduraBasica', produccionBase: 3, consumePorUnidad: { cuero: 5 } },
          { produce: 'armaBronce', produccionBase: 2, consumePorUnidad: { lingoteBronce: 1, madera: 2 } },
          { produce: 'armaduraIntermedia', produccionBase: 2, consumePorUnidad: { lingoteCobre: 1, cueroCurtido: 5 } },
          RECETA_ARMA_MADERA,
        ],
      },
      3: {
        requisitoNivelAsentamiento: 3,
        requiereEdificio: 'palacio',
        costoMejora: { madera: 450, piedra: 200 },
        trabajadoresRequeridos: 20,
        recetas: [
          { produce: 'armaCobre', produccionBase: 3, consumePorUnidad: { lingoteCobre: 1, madera: 1 } },
          { produce: 'armaduraBasica', produccionBase: 3, consumePorUnidad: { cuero: 5 } },
          { produce: 'armaBronce', produccionBase: 2, consumePorUnidad: { lingoteBronce: 1, madera: 2 } },
          { produce: 'armaduraIntermedia', produccionBase: 2, consumePorUnidad: { lingoteCobre: 1, cueroCurtido: 5 } },
          { produce: 'armaBronceCalidad', produccionBase: 1, consumePorUnidad: { lingoteBronce: 5, madera: 5 } },
          { produce: 'armaduraBronce', produccionBase: 1, consumePorUnidad: { lingoteBronce: 1, cueroCalidad: 5 } },
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
  carpinteria: {
    costo: { madera: 60, piedra: 20 },
    tiempoConstruccionMinutos: 6,
    requisitoNivelAsentamientoConstruccion: 3,
    niveles: {
      1: { trabajadoresRequeridos: 0, recetas: [] },
      2: { requisitoNivelAsentamiento: 3, costoMejora: { madera: 120, piedra: 60 }, trabajadoresRequeridos: 0, recetas: [] },
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
    tiempoConstruccionMinutos: 6,
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
      3: {
        requisitoNivelAsentamiento: 3,
        requiereEdificio: 'palacio',
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
    tiempoConstruccionMinutos: 6,
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

  // Único tier — desbloquea la aparición de Nobleza (además del mínimo de ciudadanos ya existente, ver
  // engine/population.ts). requisitoNivelAsentamientoConstruccion gatea la construcción BASE (no hay mejoras).
  // Gate subido de nivel 3 a nivel 4 (Doc Fase_0_6): construirlo pasa a ser requisito para subir a nivel 5.
  palacio: {
    costo: { madera: 1500, piedra: 1000 },
    tiempoConstruccionMinutos: 20,
    requisitoNivelAsentamientoConstruccion: 4,
    capacidadNobles: 200,
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
    tiempoConstruccionMinutos: 8,
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
  // Pieza satélite de la zona de Carpintería (§9) — mismo patrón que puestoMercado, ver `crearTalleresDeCarpinteria`.
  tallerCarpinteria: { costo: {}, tiempoConstruccionMinutos: 0 },
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
    tiempoConstruccionMinutos: 200,
    requisitoNivelAsentamientoConstruccion: 5,
  },
} as const;

export const ALMACEN = {
  capacidadInicialPorRecurso: 200,
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
  maximoEnConstruccionSimultanea: 2,
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
 * es el CENTRO de ese rectángulo (para un 1x1 coincide con el centro de su celda, ver `celdaAPunto` en
 * engine/trazado.ts). El Centro Urbano es la única excepción: su posición (0,0) es el VÉRTICE de su esquina
 * inferior izquierda, no su centro (a petición del usuario) — ver `celdasDeEdificio`, engine/trazado.ts.
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
   * Lado de una celda en unidades locales. **6 → 3 en el Paso 1 de la Etapa 6** (doc trazado §E6.11): la
   * rejilla se discretiza al DOBLE de resolución y todas las huellas de `EDIFICIO_TAMANO` se doblan a la vez,
   * de modo que el tamaño FÍSICO de cada edificio no cambia — una Vivienda sigue midiendo 6 unidades locales,
   * solo que ahora son 2x2 celdas en vez de 1x1.
   *
   * Para qué: con las calles sobre CELDAS (Etapa 6) una calle necesita ancho propio, y a la resolución
   * anterior el ancho mínimo posible era una Vivienda entera. Al doblar la resolución, `anchoCalle = 1` mide
   * media Vivienda, que es la proporción buscada.
   *
   * La identidad `punto = (col + ancho/2) · tamanoCelda` se conserva EXACTA al doblar ambos a la vez
   * (`(2·col + 2·ancho/2) · 3 = (col + ancho/2) · 6`), así que `Edificio.posicion` —que está persistido en
   * unidades locales— no se mueve y **no hace falta migrar ninguna partida**. Congelado en
   * `engine/__tests__/escalaRejilla.test.ts` con una tabla generada antes del cambio.
   */
  tamanoCelda: 3,
  radioMapa: 220,
};

/**
 * Huella de cada tipo de edificio en la rejilla local, en celdas (a petición del usuario). Un tipo ausente
 * mide `EDIFICIO_TAMANO_POR_DEFECTO` — el caso por defecto (Vivienda, Leñera, las minas). Granja NO está aquí:
 * es el único tipo cuya huella cambia con el nivel interno, y vive en
 * `EDIFICIO_CATALOGO.granja.niveles[n].tamano`.
 *
 * Se lee siempre a través de `tamanoEdificio` (engine/trazado.ts), nunca directo, para que el caso de Granja
 * quede resuelto en un solo sitio.
 *
 * **Todos los valores se doblaron en el Paso 1 de la Etapa 6** (doc trazado §E6.11), a la vez que
 * `REJILLA_ASENTAMIENTO.tamanoCelda` pasaba de 6 a 3: el tamaño FÍSICO no cambia, solo la resolución a la que
 * se discretiza. Los comentarios que mencionan medidas ("2x2", "3x2") siguen refiriéndose a la rejilla
 * ORIGINAL, que es la unidad en la que se acordaron con el usuario.
 */
export const EDIFICIO_TAMANO: Record<string, { ancho: number; alto: number }> = {
  centroUrbano: { ancho: 6, alto: 6 },
  // 5x4 → 4x2 (Etapa 3, §9): Carpintería pasa a ser la PIEZA PRINCIPAL de su propia zona de tres piezas — los
  // otros dos talleres (`tallerCarpinteria`) ocupan el resto de lo que antes era un bloque monolítico único.
  carpinteria: { ancho: 8, alto: 4 },
  fundicion: { ancho: 4, alto: 4 },
  curtiduria: { ancho: 4, alto: 4 },
  armeria: { ancho: 4, alto: 6 },
  barracon: { ancho: 4, alto: 4 },
  galeriaDeTiro: { ancho: 4, alto: 8 },
  mercado: { ancho: 6, alto: 4 },
  palacio: { ancho: 8, alto: 8 },
  corral: { ancho: 8, alto: 6 },
  almacen: { ancho: 4, alto: 2 },
  // Granero: el doble de largo que el Almacén (4x2 de la rejilla original) — guarda un solo recurso pero
  // mucha cantidad, y que se distinga a simple vista del Almacén importa en la Vista de Asentamiento.
  granero: { ancho: 8, alto: 4 },
  // Anclas y satélites, Etapa 3 (§5.1/§6): las tres anclas nuevas miden 2x2 de la rejilla original.
  plaza: { ancho: 4, alto: 4 },
  plazaDeArmas: { ancho: 4, alto: 4 },
  patioDeGremios: { ancho: 4, alto: 4 },
  // Taller de carpintería (§9): igual que las otras piezas satélite pequeñas.
  tallerCarpinteria: { ancho: 4, alto: 4 },
  // Variedad de anclas residenciales (Etapa 4, punto 4): pozo el marcador mínimo (1x1 original), parque el
  // único no cuadrado de los tres (3x2 original), que ejercita la orientación intercambiable también en anclas.
  pozo: { ancho: 2, alto: 2 },
  parque: { ancho: 6, alto: 4 },
};

/**
 * Huella de un tipo AUSENTE de `EDIFICIO_TAMANO` (Vivienda, Leñera, las tres minas, Gran Fundición, Maravilla,
 * Muralla). Es 1x1 de la rejilla ORIGINAL, o sea 2x2 tras el Paso 1 de la Etapa 6.
 *
 * Existe como constante con nombre y no como literal en `tamanoEdificio` porque el reescalado tenía que
 * alcanzarla igual que a la tabla: dejarla en `{1,1}` habría dejado a la Vivienda —el edificio más numeroso de
 * cualquier ciudad— a la MITAD de su tamaño físico, y el síntoma habría sido "las casas encogieron", no un
 * error de tipos. Un literal repetido dentro de una función es justo lo que un reescalado se salta.
 */
export const EDIFICIO_TAMANO_POR_DEFECTO = { ancho: 2, alto: 2 };

/**
 * Formas que puede tener un puesto de Mercado (a petición del usuario: la zona se compone de piezas de tamaños
 * distintos). El discriminador es `Edificio.nivelInterno`, que en un puesto NO es progresión: identifica qué
 * forma tiene. Se reutiliza así el mecanismo que ya existe para Granja (`tamanoEdificio(tipo, nivelInterno)`,
 * engine/trazado.ts) en vez de persistir el tamaño en el `Edificio` — el tamaño siempre se DERIVA del tipo.
 */
export const PUESTO_MERCADO_FORMA: Record<number, { ancho: number; alto: number }> = {
  // Formas fijadas tras el playtest del laboratorio (2026-08-31): tres piezas estrechas (2 celdas de ancho),
  // que apiladas contra el Mercado forman un mercadillo de puestos alargados en vez de bloques cuadrados.
  1: { ancho: 2, alto: 4 },
  2: { ancho: 2, alto: 6 },
  3: { ancho: 2, alto: 2 },
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

/**
 * Zona de la Carpinteria (§9 del doc de trazado urbano). A diferencia del Mercado, no escalona por nivel
 * interno: los `talleres` nacen todos de una vez al completarse la pieza principal. Objeto (no un literal
 * suelto) para que el laboratorio pueda ajustar `talleres` en caliente -- misma razon que
 * `MERCADO_PUESTOS_POR_NIVEL`.
 */
export const CARPINTERIA_ZONA = {
  talleres: 2,
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
  // Ancho de manzana en celdas — doblado en el Paso 1 de la Etapa 6 (§E6.11) junto con `tamanoCelda`: la
  // manzana mide lo mismo físicamente, se discretiza al doble de resolución.
  largoFilaMin: 8,
  largoFilaMax: 16,
  // `radioAfuerasMin` y `anchoBandaAfueras` están en UNIDADES LOCALES, no en celdas (ver `radioMaximoAfueras`,
  // engine/trazado.ts, que los compara contra `radioPotencial`): el reescalado de la Etapa 6 NO los toca.
  radioAfuerasMin: 60,
  anchoBandaAfueras: 36,
  // Anclas y satélites, Etapa 2 (Consideraciones/Vista_Asentamiento_Trazado_Urbano.md §5.3/5.7): separación
  // mínima en celdas entre centros de ancla, usada por la búsqueda de ranura del árbol (`radioInicialRanura` /
  // `radioMaximoRanura`, engine/trazado.ts, que se derivan de aquí y se reescalan solas).
  // Ya NO define el núcleo de un ancla: desde §E6.21 ese es la banda de una manzana (`FONDO_MANZANA` celdas
  // desde el anillo de calle, en `sitiosPorAtraccionDura`), no `separacionMinimaAnclas / 2`.
  // Doblada en el Paso 1 de la Etapa 6 (§E6.11) — 6 celdas de la rejilla original.
  separacionMinimaAnclas: 12,
  // Zona de seguridad entre anclas (a petición del usuario): un PISO DURO, no relajable — a diferencia de
  // `separacionMinimaAnclas`, que el doc describe como negociable, esta nunca cede. Ningún ancla real nueva
  // (Mercado, Carpintería — `ANCLAS_REALES`, engine/trazado.ts) puede colocarse a menos de esta distancia,
  // BORDE A BORDE (`gapCeldas` en `huecoEnDireccion`), de OTRA ancla ya construida. Si ningún hueco la
  // cumple, no hay sitio válido en ese tick — la colocación se salta o se reintenta, igual que cualquier otro
  // "no cabe" del trazado.
  // Fijada en 6 celdas tras el playtest del laboratorio (2026-08-31) — deja espacio para una calle y una
  // hilera de satélites entre dos anclas vecinas sin que se pisen los núcleos.
  separacionSeguridadAnclas: 6,
  /**
   * Ancho de una calle EN CELDAS (Etapa 6, decisión 1 del doc trazado §E6.3). Con la rejilla del Paso 1
   * (`tamanoCelda` 3), una celda es media Vivienda: callejón estrecho, muy de la Edad de Bronce.
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
  /** Ritmo de obra: celdas levantadas por minuto (= por tick) mientras haya materiales. Es lo que hace que el
   * anillo se vea cerrarse poco a poco en vez de aparecer de golpe. */
  celdasPorMinuto: 1,
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
export const CARAVANA_CATALOGO = {
  // costoConstruccion (nuevo, ampliación de comercio): solo 'comercial' es un activo persistente que el
  // jugador construye y conserva (flota propia, ver `construirCaravanaComercial`) — militar/construccion/
  // contrabando siguen siendo instanciadas por su propio mecanismo (reclutamiento militar sin implementar
  // todavía; Caravana de Fundación, Doc 1.8) y no tienen costo de flota propio.
  comercial: { capacidad: 60, velocidad: 16, costoConstruccion: { madera: 50 } },
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

export const TRUEQUE = {
  // "expirar un plazo" sin número fijado en el diseño (ver Preguntas_Abiertas) — placeholder.
  plazoMinutosPorDefecto: 200,
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
  xpParaNivel: [50, 90, 150, 240, 380, 600, 950, 1500, 2400],
  // Experiencia otorgada por evento (placeholder). `combate` cubre asedio/campo abierto/interceptar caravana/
  // atacar campamento de bandidos por igual (participación, no solo victoria) — ver `engine/combate.ts`. Los
  // valores de abajo son POR JUGADOR PARTICIPANTE (a petición del usuario, Doc Fase_0_5 §8): si 3 jugadores
  // atacan juntos un campamento, la Facción recibe 3×`combate`, no un monto plano — ver
  // `jugadoresParticipantes` en `engine/combate.ts`. Excepción: `defensaCaravana` no se multiplica todavía
  // (la escolta de una caravana no tiene escuadrones/jugadores reales, Doc 3.10), y `edificioCompletado`
  // tampoco (un edificio no tiene "jugadores que lo completaron" en el modelo actual).
  xp: {
    combate: 5,
    edificioCompletado: 1,
    conquista: 20,
    defensaCaravana: 3,
    ataqueCaravana: 3,
  },
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
  // Espacios de "casas" por asentamiento (Doc 2.5): el cupo base coincide con el máximo de jugadores que
  // pueden fundar juntos (FUNDACION.maxJugadoresFundacionGrupal) — así el asentamiento siempre nace con
  // sitio para todos sus fundadores, y el resto queda libre para compras posteriores.
  casasBasePorAsentamiento: FUNDACION.maxJugadoresFundacionGrupal,
  casasPorNivelAdicional: 2,
  // Resuelve la pregunta abierta que dejaba `crearFaccion.ts` (a petición del usuario, 2026-08-27): tras
  // abandonar una Facción (`dejarFaccion`), cuánto hay que esperar para poder crear otra — anti-abuso contra
  // "crear, abandonar, crear" en bucle. Solo aplica a CREAR: unirse a una Facción existente (`unirseAFaccion`)
  // no tiene cooldown, solo la regla de siempre (no estar ya en otra).
  cooldownCreacionFaccionDias: 7,
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
  // expire la anterior. Compiten en ese mismo slot con Vía Rápida y Líneas de Producción, que es la tensión
  // interesante: forma contra velocidad contra logística.
  //
  // Como una política dura `duracionMinutosPorDefecto` (150 ticks) y nada mueve lo ya construido, cada una
  // deja un ESTRATO en la ciudad en vez de reformarla entera — la ciudad acaba registrando su historia
  // política en su geometría.
  //
  // PENDIENTE (a propósito, no olvido): ninguna tiene todavía coste/beneficio mecánico propio, así que hoy
  // compiten en desventaja contra Vía Rápida (−25% de tiempo de obra). `barrios_gremiales` es la que más cerca
  // está de tener uno solo: agrupar industria acorta la distancia a los insumos, que `factorLineaProduccion`
  // (engine/construction.ts) ya mide y ya premia. Sin calibrar.
  { id: 'postura_defensiva', cargo: 'maestroObras', nombre: 'Postura Defensiva', perfilTrazado: 'compacta' },
  { id: 'arterias_comerciales', cargo: 'maestroObras', nombre: 'Arterias Comerciales', perfilTrazado: 'caminera' },
  { id: 'barrios_gremiales', cargo: 'maestroObras', nombre: 'Barrios Gremiales', perfilTrazado: 'gremial' },
  { id: 'plazas_mayores', cargo: 'maestroObras', nombre: 'Plazas Mayores', perfilTrazado: 'nucleos' },
  // A petición del usuario, líneas de producción (Doc 4.2.1): mientras esté activa, la auto-construcción sitúa
  // los edificios de transformación nuevos (Fundición/Curtiduría/Armería) en el hueco de su zona que minimiza
  // la penalización de distancia a la fuente de sus insumos (`sitioConcentricoLineaProduccion`,
  // engine/construction.ts) en vez del primer hueco libre del barrido de anillos de siempre. Compite por el
  // único slot de Maestro de Obras con Vía Rápida de Construcción — no se pueden tener las dos a la vez.
  { id: 'lineas_produccion', cargo: 'maestroObras', nombre: 'Líneas de Producción', lineasProduccionPriorizadas: true },
  { id: 'comercio_abierto', cargo: 'tesorero', nombre: 'Comercio Abierto', factorComisionExterna: 0.6 },
  { id: 'aranceles', cargo: 'tesorero', nombre: 'Aranceles Proteccionistas', factorComisionExterna: 1.5 },
  { id: 'leva_forzosa', cargo: 'general', nombre: 'Leva Forzosa', factorCostoReclutamiento: 0.7 },
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
 * `reclutarTropa`, engine/tropas.ts). Cifras PLACEHOLDER sin calibrar por simulación todavía.
 */
export const TROPAS_RECLUTABLES: {
  id: string;
  nombre: string;
  edificio: 'centroUrbano' | 'barracon' | 'galeriaDeTiro';
  nivelRequerido: number;
  costoEquipo: Partial<Record<string, number>>;
  poderBase: number;
  unidadesPorDefecto: number;
  /** Velocidad de marcha por el mapa general (Doc 5.12.5). Un ejército va al ritmo de su escuadrón MÁS
   * LENTO, así que meter un solo escuadrón pesado en una partida de incursión la frena. Las dos reglas que
   * fijan estos números: una caravana inicial (comercial, 16) no puede ser más rápida que un ejército, y un
   * jugador solo con infantería ligera tiene que poder alcanzarla. */
  velocidad: number;
}[] = [
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
  { id: 'milicia_lanceros', nombre: 'Milicia de lanceros', edificio: 'centroUrbano', nivelRequerido: 1, costoEquipo: { madera: 2 }, poderBase: 2, unidadesPorDefecto: 25, velocidad: 20 },
  // Recosteadas a `armaMadera` (ver RECETA_ARMA_MADERA): antes exigían la cadena del cobre/cuero entera, lo
  // que era además temáticamente incoherente — un escudo de MIMBRE pagado con un arma de cobre, y unos
  // Honderos (una honda y una piedra) pagados con armadura de cuero. El cobre pasa a ser la MEJORA
  // (`espadachines_cobre`, que sí lo conserva), no el ticket de entrada.
  { id: 'lanceros_mimbre', nombre: 'Lanceros con escudo de mimbre', edificio: 'barracon', nivelRequerido: 1, costoEquipo: { armaMadera: 1 }, poderBase: 3, unidadesPorDefecto: 20, velocidad: 20 },
  { id: 'espadachines_cobre', nombre: 'Espadachines de espada corta de cobre', edificio: 'barracon', nivelRequerido: 1, costoEquipo: { armaCobre: 1, armaduraBasica: 1 }, poderBase: 4, unidadesPorDefecto: 20, velocidad: 16 },
  { id: 'hacheros_ligeros', nombre: 'Hacheros ligeros', edificio: 'barracon', nivelRequerido: 2, costoEquipo: { armaBronce: 1, armaduraBasica: 1 }, poderBase: 7, unidadesPorDefecto: 18, velocidad: 16 },
  { id: 'espadachines_bronce', nombre: 'Espadachines con espadas y escudos de bronce', edificio: 'barracon', nivelRequerido: 2, costoEquipo: { armaBronce: 2, armaduraIntermedia: 1 }, poderBase: 9, unidadesPorDefecto: 18, velocidad: 16 },
  { id: 'lanceros_pesados', nombre: 'Lanceros pesados micénicos', edificio: 'barracon', nivelRequerido: 3, costoEquipo: { armaBronce: 2, armaduraIntermedia: 2 }, poderBase: 14, unidadesPorDefecto: 15, velocidad: 12 },
  { id: 'hacheros_armados', nombre: 'Hacheros armados', edificio: 'barracon', nivelRequerido: 3, costoEquipo: { armaBronce: 1, armaduraIntermedia: 1 }, poderBase: 12, unidadesPorDefecto: 15, velocidad: 12 },
  { id: 'honderos', nombre: 'Honderos', edificio: 'galeriaDeTiro', nivelRequerido: 1, costoEquipo: { armaMadera: 1 }, poderBase: 5, unidadesPorDefecto: 25, velocidad: 20 },
  { id: 'escaramuzadores_jabalina', nombre: 'Escaramuzadores con jabalina', edificio: 'galeriaDeTiro', nivelRequerido: 2, costoEquipo: { armaBronce: 1, armaduraBasica: 1 }, poderBase: 8, unidadesPorDefecto: 20, velocidad: 20 },
  { id: 'arqueros', nombre: 'Arqueros', edificio: 'galeriaDeTiro', nivelRequerido: 2, costoEquipo: { armaBronce: 1, armaduraIntermedia: 1 }, poderBase: 9, unidadesPorDefecto: 25, velocidad: 16 },
  { id: 'arqueros_compuesto', nombre: 'Arqueros con arco compuesto', edificio: 'galeriaDeTiro', nivelRequerido: 3, costoEquipo: { armaBronce: 3, armaduraIntermedia: 2 }, poderBase: 15, unidadesPorDefecto: 20, velocidad: 12 },
];

export const MILITAR = {
  racionPorSoldadoPorMinuto: 0.15,
  regeneracionMoralPorMinuto: 5,
  degradacionMoralSinRacion: 20,
  // Fracción de la cantidad del escuadrón que deserta por minuto mientras la moral está a 0 (Doc 5.4).
  desercionFraccionPorMinutoSinMoral: 0.05,
  bonusVeteraniaPorPunto: 0.05,
  veteraniaGanadaPorVictoria: 1,
  veteraniaGanadaPorDerrota: 0.5,
  // Cohesión entre escuadrones defendiendo juntos (Doc 5.3), abstraída como bonus de poder (sin formaciones renderizadas).
  bonusCohesionPorEscuadronExtra: 0.1,
  duracionHeridoMinutos: 30,
  penalizacionHerido: 0.5,
  varianzaCombate: 0.15,
  // Combate de caravanas (Doc 3.10): umbral de captura del 50% y defensa base de una escolta no modelada en detalle.
  umbralCapturaCaravana: 0.5,
  defensaBaseCaravana: 15,
};

/**
 * Liderazgo (Doc 5.11): cuánto puede sacar a campaña un jugador. Es límite de SALIDA, no de posesión — la
 * guarnición no tiene tope, y por eso "¿qué me llevo?" es la decisión central de cada campaña.
 *
 * `factorCoste` PLACEHOLDER, a calibrar. El coste NO se escribe a mano tropa por tropa: se DERIVA de
 * `poderBase × unidadesPorDefecto × factorCoste` (ver `costeLiderazgo`, engine/liderazgo.ts). La razón es
 * concreta: `poderBase` sigue siendo placeholder pendiente de calibración (Doc 5.8), y once números escritos
 * a mano se desincronizarían del poder en cuanto se calibre. Una fórmula no — se calibra un solo número.
 *
 * Con `base: 50` y `factorCoste: 0.2` la Milicia de lanceros cuesta 10 (ancla elegida por el usuario) y los
 * Arqueros con arco compuesto 60: la tropa de élite es INFIELABLE sin progresión de liderazgo, que es la
 * consecuencia buscada (Doc 5.11.1).
 */
export const LIDERAZGO = {
  base: 50,
  factorCoste: 0.2,
};

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
  factorConsumoEstacionado: 0.5,
  /**
   * Campo de visión de un ejército en marcha, en unidades de MAPA (Doc 5.12.7).
   *
   * 150 sobre un mapa de 2000 es ~2 radios de provincia (~76, ver `ESCALA` y Doc 1.0a): un ejército ve
   * VARIAS ciudades por delante si la geografía lo permite, que es lo que se pedía. Y queda holgadamente por
   * debajo del radio de cohesión de un reino (`MANTENIMIENTO.escalaDistancia` = 400, ~5 provincias), así que
   * ver no equivale a controlar.
   *
   * Se descartó 30 —el radio de una zona de influencia recién fundada— porque bajo la escala rota parecía
   * razonable y con la escala declarada no llega ni al borde de la propia provincia.
   */
  radioVisionEjercito: 150,
  radioReabastecimiento: 60,
  radioEncuentro: 60,
};

/**
 * Campamentos de bandidos (Doc 1.9, a petición del usuario — inspirado en análisis comparativo con Travian):
 * amenaza NPC en bosques no reclamados que ataca caravanas cercanas. Todas las cifras son PLACEHOLDER, sin
 * calibrar por simulación todavía (ver `Preguntas_Abiertas.md` #14c) — mismo criterio que el resto del proyecto.
 */
export const CAMPAMENTOS_BANDIDOS = {
  // Poder de combate fijo del campamento — referencia: una Milicia de lanceros recién reclutada (25 unidades,
  // poderBase 2) ronda 50 de poder sin veteranía, así que este valor la deja en desventaja pero no indefensa.
  poder: 30,
  // Radio (unidades del mapa) dentro del cual un campamento ataca a una caravana que pase cerca.
  radioAtaqueCaravana: 40,
  // Ticks tras destruirse un campamento antes de que pueda aparecer uno nuevo ("N días" del diseño, Doc 1.9,
  // expresado en ticks — Fase 0 no tiene mapeo tick-a-tiempo-real todavía). Bajado de 60 a 10 (pruebas del
  // usuario) — a 60 ticks el farming de XP de Facción vía campamentos era demasiado lento frente al resto
  // de fuentes de experiencia.
  respawnMinutos: 10,
  // Recompensa fija al destruirlo (botín).
  recompensa: { madera: 40, piedra: 20, oro: 15 } as Partial<Record<string, number>>,
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

/**
 * ⚠️ SOLO PARA SIMULACIÓN — NO ES PARTE DEL JUEGO REAL (Fase_0_5_Definicion_Especializacion_y_Cupos.md §"lo
 * primero que hay que hacer"). El juego real sigue siendo 100% manual: un Tesorero/Gobernador humano propone
 * el trueque desde la UI (Doc 3.2). Este interruptor activa `engine/simulacionAutoComercio.ts`, un NPC
 * virtual que hace ese mismo trabajo automáticamente dentro de una Facción — para poder correr batches
 * largos y medir si la dependencia entre niveles/asentamientos (excedente de un asentamiento cubriendo el
 * déficit de otro) funciona, sin depender de que haya un jugador humano interactuando en cada tick.
 * `activo: 0` (apagado) por defecto — el juego real nunca lo ve encendido a menos que se active a propósito
 * desde este panel de balance. Para ELIMINAR este mecanismo por completo: borra este bloque, borra
 * `engine/simulacionAutoComercio.ts`, y borra la llamada gateada en `gameStore.ts` (buscar
 * "SIMULACION_AUTO_COMERCIO").
 */
export const SIMULACION_AUTO_COMERCIO = {
  activo: 0,
  // Colchón mínimo de stock (fracción de la capacidad) que un asentamiento debe conservar de un recurso
  // antes de poder ofrecerlo como excedente a otro — evita que el NPC vacíe su propia reserva de seguridad.
  colchonExcedente: 0.3,
  // Cantidad pactada por lado en cada trueque automático propuesto (placeholder, sin calibrar).
  cantidadPorTrueque: 30,
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
    3: { pesants: 500, artesanos: 200, edificios: ['armeria', 'curtiduria', 'fundicion', 'barracon', 'galeriaDeTiro'] },
    // Sustituye al viejo `edificios: ['muralla']` (Paso 5, `Consideraciones/Murallas_Definicion.md` §13): el
    // recinto ya no es un `EdificioTipo`, así que el gate deja de poder contarlo como edificio y pasa a
    // `recintoCompletoNivelMinimo` — cualquier recinto TERMINADO (integridad 1) de nivel 1 en adelante basta,
    // la empalizada barata cuenta igual que la muralla de piedra. `edificios: []` es intencional, no un
    // descuido: sin ningún tipo en la lista, `cumpleEdificios` es trivialmente cierto y el gate real es el
    // del recinto.
    4: { pesants: 1000, artesanos: 400, edificios: [], recintoCompletoNivelMinimo: 1 },
    5: { pesants: 2000, artesanos: 800, edificios: ['palacio'] },
  } as Record<
    number,
    { pesants: number; artesanos: number; edificios: string[]; edificiosMinimo?: number; recintoCompletoNivelMinimo?: number }
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
 * Mantenimiento (Doc 4.5): coste periódico que escala por nivel (sumando materiales, no reemplazando) y por
 * distancia al centro de poder de la Facción (aquí: su asentamiento más antiguo vivo, como proxy de "capital").
 * Cantidades y velocidad de degradación son PLACEHOLDER (Preguntas_Abiertas no fija cifras exactas).
 */
export const MANTENIMIENTO = {
  medidorInicial: 100,
  // Trigo NO va aquí (fix: era una "mecánica repetida" — Mantenimiento cobraba este valor fijo ADEMÁS del
  // consumo real de comida que ya se descuenta en `avanzarNutricionPoblacion`/`avanzarMantenimientoTropas`, duplicando
  // el gasto). El "apartado de trigo" que se muestra en el panel de Mantenimiento ahora es la suma real de
  // consumo de población + tropas (ver `gameStore.mantenimientoInfo`), no un placeholder desconectado.
  costoBase: { madera: 3 },
  // Rediseño de progreso (Fase 0): con el tope de nivel bajando de 10 a 3 (ver NIVEL_ASENTAMIENTO), los
  // umbrales de piedra/oro (antes nivel 3 y nivel 8, pensados para un rango 1-10) se recalibran al rango 1-3
  // para que los 3 niveles tengan una escalada de coste real — cifra exacta PLACEHOLDER pendiente de
  // calibración por simulación (ver Preguntas_Abiertas.md).
  nivelParaPiedra: 2,
  piedraBase: 3,
  nivelParaOro: 3,
  oroBase: 2,
  // Escala por POBLACIÓN real, no por nivel (Doc Fase_0_5 §3.2, reemplaza `factorCrecimientoPorNivel`, a
  // petición del usuario: "más allá de al tamaño del asentamiento y a la cantidad de edificios" — un nivel 3
  // con 6000 habitantes paga mucho más que un nivel 1 con 250, con o sin ascender de nivel; el nivel en sí ya
  // NO multiplica el coste directamente, ver §6.2: `nivelActual` degradado no reduce el mantenimiento porque
  // la misma gente sigue comiendo/gastando lo mismo). `factorPoblacion = 1 + poblacionTotal/poblacionReferencia`.
  // Se retira la idea (descartada) de sumar coste por Nº DE EDIFICIOS: chocaba con el factor de distancia de
  // abajo, que ya es el mecanismo real de "imperio disperso cuesta más" — sumar ambos habría castigado DOBLE
  // al imperio distribuido que el diseño quiere fomentar (ver Fase_0_5_Definicion...md §5.1). Placeholder sin
  // calibrar: a poblacionReferencia=500, nivel 1 en su techo (300 hab) paga factor 1.6; nivel 3 en su techo
  // (6000 hab) paga factor 13 — frente a un techo de extracción propia de ~50 madera/minuto, el mantenimiento
  // solo ya se come casi toda la producción bruta de una ciudad llena, antes de sumar comida/ejército/mejoras.
  poblacionReferencia: 500,
  escalaDistancia: 400,
  factorDistanciaMax: 2,
  degradacionPorDeficitTotal: 10,
  regeneracionSiPagoCompleto: 5,
  // Protección temporal a asentamientos recién fundados (Doc 1.3, pendiente en el diseño): sin esto, todo
  // asentamiento nuevo entra en déficit desde el tick 1 (antes de que la Granja llegue a construirse) y cae
  // en ruinas pase lo que pase. La gracia cubre el tiempo típico de estabilizar la economía base.
  graciaMinutos: 60,
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
