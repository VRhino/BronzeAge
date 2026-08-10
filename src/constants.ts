// Valores numéricos PLACEHOLDER — ver Consideraciones/Preguntas_Abiertas.md.
// Centralizados aquí para poder re-balancear sin tocar la lógica del motor.
//
// NO están aquí los parámetros de GENERACIÓN de mundo (tamaño del mapa, rareza/cantidad de nodos, bosques,
// fertilidad): viven congelados en `src/worldgen/config.ts`. Todo lo de este archivo es editable en caliente
// desde el panel de balance (ver `app/balanceConfig.ts`), y la generación no puede serlo — la partida
// guardada solo almacena la seed y regenera el mapa al cargar. Ver el encabezado de `worldgen/config.ts`.

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
  radioMaximoPorNivel: { 1: 60, 2: 90, 3: 120 } as Record<number, number>,
  segmentosPoligono: 48, // resolución del círculo aproximado como polígono
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
  artesanos: { tasaCrecimientoBase: 0.05 },
  // "Cantidad mínima de ciudadanos" sin número fijado en el diseño (ver Preguntas_Abiertas) — placeholder.
  // Rediseño de progreso (Fase 0): además de este mínimo, ahora también requiere Palacio construido
  // (Doc 4.2.1 — "desbloquea la aparición de la población noble"), ver engine/population.ts.
  nobleza: { minCiudadanos: 3, tasaCrecimientoBase: 0.01 },
  consumoComidaPorHabitante: 0.1, // trigo/tick por habitante (pesants+artesanos+nobleza)
};

/**
 * Receta de crafting de un edificio de transformación (Doc 4.2.1, rediseño de progreso Fase 0): `produccionBase`
 * es la tasa objetivo por tick (mismo criterio que produccionBaseTrigo/produccionBasePiedra etc.);
 * `consumePorUnidad` es cuánto de cada insumo hace falta por cada unidad de output, derivado de la proporción
 * de la receta original documentada (ej. "8 Lingote de Cobre + 2 Lingote de Estaño -> 5 Lingote de Bronce" con
 * produccionBase 1 LB/tick => consumePorUnidad { lingoteCobre: 1.6, lingoteEstano: 0.4 }). La producción real de
 * cada tick se limita por `min(produccionBase * ratioManoObraArtesanos, insumo_disponible / consumePorUnidad)`,
 * mismo criterio que ya usan los extractores minerales contra `nodo.cantidad` (ver engine/construction.ts).
 */
interface RecetaProduccion {
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
 * Cifras deliberadamente modestas (2/tick a cambio de 4 madera/tick): `avanzarRecetas` NO respeta la reserva
 * dinámica de Mantenimiento (a diferencia de la construcción, ver `puedeIniciarConstruccion`) — vacía el
 * stock hasta donde llegue. Una tasa más alta convertiría la Armería en una vía de colapso por falta de
 * madera para Mantenimiento.
 */
const RECETA_ARMA_MADERA = { produce: 'armaMadera', produccionBase: 2, consumePorUnidad: { madera: 2 } };

export const EDIFICIO_CATALOGO = {
  // Único edificio que NO pasa por la cola de construcción (ni automática ni manual, Doc 1.3): nace
  // ya activo al fundar. costo/tiempoConstruccionTicks quedan en 0 solo por consistencia de forma con
  // el resto del catálogo — nunca se leen, porque construirlo por otra vía no es posible.
  centroUrbano: { costo: {}, tiempoConstruccionTicks: 0 },
  // Cupos SEPARADOS por clase (a petición del usuario, ver Correcciones): antes un único pool compartido
  // entre Pesants y Artesanos hacía que Pesants (crece ~2.4x más rápido) acaparara todo el cupo y dejara a
  // Artesanos varado — cada Vivienda ahora aporta 15 espacios de Pesants Y, por separado, 5 de Artesanos.
  vivienda: { costo: { madera: 10 }, tiempoConstruccionTicks: 4, capacidadPesants: 15, capacidadArtesanos: 5 },
  granja: { costo: { madera: 30 }, tiempoConstruccionTicks: 6, produccionBaseTrigo: 15, trabajadoresRequeridos: 4 },
  cantera: { costo: { madera: 20 }, tiempoConstruccionTicks: 5, produccionBasePiedra: 5, trabajadoresRequeridos: 4 },
  lenera: { costo: { madera: 10 }, tiempoConstruccionTicks: 3, produccionBaseMadera: 5, trabajadoresRequeridos: 4 },
  almacen: { costo: { madera: 50, piedra: 30 }, tiempoConstruccionTicks: 6, capacidadPorRecursoAdicional: 300 },
  // Oro: metal precioso en bruto, origen en minas igual que cualquier otro recurso (Doc 3.1). produccionBaseOro
  // recalibrado (verificación batch del overhaul de auto-construcción): a 2/tick, una sola Mina (2) ya no
  // alcanzaba a cubrir el costo de Mantenimiento de oro a nivel 3 (`MANTENIMIENTO.oroBase` × escala ≈
  // 2.6-5.2/tick) ni siquiera con el nodo recién descubierto — a diferencia de piedra/Cantera, este era un
  // problema real de TASA, no solo de tamaño de nodo. Sube a 4 para dar el mismo margen que Cantera tiene
  // sobre su propio costo de Mantenimiento (~1.4-1.5×) en la distancia base.
  mina: { costo: { madera: 40, piedra: 10 }, tiempoConstruccionTicks: 6, produccionBaseOro: 4, trabajadoresRequeridos: 6 },
  // Cobre (Doc 1.1/5.7): "relativamente abundante" — igual patrón que cantera/mina pero sobre nodos de cobre.
  minaCobre: { costo: { madera: 30, piedra: 5 }, tiempoConstruccionTicks: 4, produccionBaseCobre: 5, trabajadoresRequeridos: 8 },
  // Estaño (Doc 1.1/5.7): raro y concentrado (menos nodos que cobre/oro, ver RECURSO_RAREZA.raro) — costo más
  // alto y producción base más baja que el resto de minas, coherente con ser el cuello de botella del bronce.
  minaEstano: { costo: { madera: 50, piedra: 20 }, tiempoConstruccionTicks: 7, produccionBaseEstano: 1.5, trabajadoresRequeridos: 8 },
  // Corral (Doc 4.2.1, rediseño de progreso Fase 0): extractor de livestock, mismo patrón que cantera/minas —
  // liga a un nodo finito de livestock (Doc 1.4), con reemplazo automático al agotarse (ver EXTRACCION_MAXIMOS).
  corral: { costo: { madera: 30 }, tiempoConstruccionTicks: 6, produccionBaseLivestock: 3, trabajadoresRequeridos: 4 },
  // "Único edificio de tier élite, exclusivo de asentamientos/Facciones de mayor nivel" — gate por nivel de Facción.
  // Se mantiene sin cambios (Doc 4.2, rediseño de progreso): queda para iteraciones posteriores la integración
  // con la nueva Fundición.
  granFundicion: { costo: { madera: 150, piedra: 100, oro: 50 }, tiempoConstruccionTicks: 20, nivelFaccionMinimo: 3 },

  // --- Edificios de transformación (Doc 4.2.1, rediseño de progreso Fase 0): auto-construcción (sin gate de
  // nivel para la construcción BASE — solo las mejoras de nivel interno lo exigen), disparan Artesanos (Doc
  // 4.1) y cuentan para los gates de nivel de asentamiento (ver NIVEL_ASENTAMIENTO). En Fase 0 no exigen
  // "Planos"/Aedas (Doc 6.5). Fundición reemplaza y amplía la Fundición manual anterior (antes sin producción). ---

  fundicion: {
    costo: { madera: 80, piedra: 40 },
    tiempoConstruccionTicks: 6,
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
    tiempoConstruccionTicks: 8,
    niveles: {
      1: {
        trabajadoresRequeridos: 4,
        recetas: [{ produce: 'cuero', produccionBase: 4, consumePorUnidad: { livestock: 0.5 } }],
      },
      2: {
        requisitoNivelAsentamiento: 2,
        costoMejora: { madera: 150, piedra: 100 },
        trabajadoresRequeridos: 6,
        recetas: [
          { produce: 'cuero', produccionBase: 4, consumePorUnidad: { livestock: 1 / 3 } },
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
    tiempoConstruccionTicks: 6,
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
  // exige nivel de asentamiento (requisitoNivelAsentamientoConstruccion): habilita Armería/Barracón/Galería de
  // tiro nivel 2, así que tiene sentido que llegue un poco después que ellos. Sin recetas: arietes/torres de
  // asedio no se modelan en Fase 0 (combate resuelto como cálculo/log, Doc 5.10).
  carpinteria: {
    costo: { madera: 60, piedra: 20 },
    tiempoConstruccionTicks: 6,
    requisitoNivelAsentamientoConstruccion: 2,
    niveles: {
      1: { trabajadoresRequeridos: 0, recetas: [] },
      2: { requisitoNivelAsentamiento: 3, costoMejora: { madera: 120, piedra: 60 }, trabajadoresRequeridos: 0, recetas: [] },
    } as Record<number, NivelEdificioTransformacion>,
  },

  // --- Edificios especiales vía política (Doc 4.4, rediseño de progreso Fase 0): NO son auto-construcción —
  // solo se encolan mientras la política de desbloqueo correspondiente esté activa (ver engine/politicas.ts
  // `politicaActivaDesbloqueaEdificio`), en un cluster de cola aparte que no cuenta contra
  // NECESIDADES.maximoEnCola. Sin recetas: el reclutamiento por equipo de Barracón/Galería de tiro queda fuera
  // de alcance de este plan (ver Doc 5.7/5.8 PENDIENTE), Palacio solo desbloquea Nobleza (engine/population.ts). ---

  barracon: {
    costo: { madera: 30 },
    tiempoConstruccionTicks: 6,
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
    tiempoConstruccionTicks: 6,
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
  palacio: {
    costo: { madera: 1500, piedra: 1000 },
    tiempoConstruccionTicks: 20,
    requisitoNivelAsentamientoConstruccion: 3,
    capacidadNobles: 200,
  },

  // Ampliación de comercio (a petición del usuario, Doc 3.3): vía política del Tesorero ("Construir Mercado"),
  // mismo patrón que Barracón/Galería de tiro — cluster de cola aparte, no auto-construcción. Sin recetas: no
  // fabrica nada, sus niveles administran `cupoCaravanas` (ver `cupoCaravanas`, engine/asentamientoQuery.ts).
  // Gatea, además de la flota, las órdenes de Mercado (`colocarOrdenMercado`, engine/market.ts) — sin Mercado
  // activo no se puede ni construir una caravana ni colocar una orden.
  mercado: {
    costo: { madera: 100, piedra: 40 },
    tiempoConstruccionTicks: 8,
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
  transformacion: 500, // curtiduria, armeria, fundicion, carpinteria
};

/**
 * Tope de extractores por tipo (cantera/mina/minaCobre/minaEstano/lenera/Corral, Doc 4.2, rediseño de
 * progreso Fase 0): antes escalaba 1:1 con el nivel del asentamiento (hasta 10, el nivelMaximo anterior); con
 * el tope de nivel bajando a 3 (ver NIVEL_ASENTAMIENTO) un máximo ligado al nivel se quedaría corto, así que
 * se desacopla a un número fijo. Calibrado por simulación (150-600 ticks): con porTipo=5 y la tasa de
 * crecimiento de Pesants ya existente (12%/tick, sin tope salvo Vivienda), todo asentamiento colapsaba por
 * déficit de Mantenimiento hacia el tick 200-700 — el tope de extracción se quedaba corto frente a una
 * población sin límite real, algo que el sistema anterior evitaba dejando llegar hasta 10 extractores por
 * tipo. Sube a 10 (mismo techo que el nivelMaximo anterior) para no perder ese margen. Sigue siendo
 * PLACEHOLDER pendiente de más calibración (ver Preguntas_Abiertas.md).
 */
export const EXTRACCION_MAXIMOS = {
  porTipo: 10,
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
// Velocidad ×2 en las 4 categorías (a petición del usuario, ampliación de comercio): el batch de diagnóstico
// mostró que a la velocidad original un trueque de tamaño moderado a distancia media podía necesitar más
// ticks de viaje (varios envíos en serie, uno por vez por cada lado del acuerdo, ver `asignarCaravanasATrueque`
// en engine/trade.ts) que `TRUEQUE.plazoTicksPorDefecto` — el acuerdo expiraba antes de poder completarse
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

// Peaje de chokepoints (Fase 0.3, Doc 1.5): se cobra al asentamiento DESTINO por cada chokepoint controlado
// por una Facción rival que la ruta de la caravana atraviesa, y se abona al asentamiento controlador — ver
// `engine/chokepoints.ts`. Placeholder sin calibrar por simulación, mismo criterio que el resto de cifras
// nuevas del proyecto. Bloqueo/escolta militar quedan PENDIENTE (ver `Preguntas_Abiertas.md`).
export const CHOKEPOINTS_PEAJE = {
  oro: 15,
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
  // Modo de emergencia ante escasez de madera/comida (a petición del usuario): mientras esté activa, la
  // auto-construcción SOLO evalúa Leñera/Granja hasta llegar a estos mínimos (activas + en curso/cola por
  // tipo), ignorando cualquier otra necesidad detectada ese tick — ver `evaluarNecesidades` en construction.ts.
  { id: 'proteccion_riesgos', cargo: 'maestroObras', nombre: 'Protección de Riesgos', minimoLenerasPrioritario: 2, minimoGranjasPrioritario: 3 },
  { id: 'comercio_abierto', cargo: 'tesorero', nombre: 'Comercio Abierto', factorComisionExterna: 0.6 },
  { id: 'aranceles', cargo: 'tesorero', nombre: 'Aranceles Proteccionistas', factorComisionExterna: 1.5 },
  { id: 'leva_forzosa', cargo: 'general', nombre: 'Leva Forzosa', factorCostoReclutamiento: 0.7 },
  // Desbloqueo de edificios especiales (Doc 4.4, rediseño de progreso Fase 0): mientras esté activa, el
  // edificio correspondiente puede encolarse en un cluster de cola aparte (no cuenta contra
  // NECESIDADES.maximoEnCola) — ver `politicaActivaDesbloqueaEdificio` en engine/politicas.ts.
  { id: 'construir_barracon', cargo: 'general', nombre: 'Construir Barracón', desbloqueaEdificio: 'barracon' },
  { id: 'construir_galeria_tiro', cargo: 'general', nombre: 'Construir Galería de Tiro', desbloqueaEdificio: 'galeriaDeTiro' },
  { id: 'construir_palacio', cargo: 'gobernador', nombre: 'Construir Palacio', desbloqueaEdificio: 'palacio' },
  { id: 'construir_mercado', cargo: 'tesorero', nombre: 'Construir Mercado', desbloqueaEdificio: 'mercado' },
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

export const TROPA_CATALOGO: Record<number, { nombre: string; poderBase: number }> = {
  1: { nombre: 'Lanceros', poderBase: 3 },
  2: { nombre: 'Arqueros', poderBase: 6 },
  3: { nombre: 'Lanceros Pesados', poderBase: 12 },
  4: { nombre: 'Guerreros de Élite', poderBase: 25 },
};

/**
 * Catálogo de TROPAS reclutables por equipo (Doc 5.7/5.8, rediseño de reclutamiento): cada tropa se recluta
 * de una vez vía Barracón (cuerpo a cuerpo) o Galería de tiro (a distancia), según el nivel interno del
 * edificio, pagando el equipo fabricado en Armería (ver engine/tropas.ts `reclutarTropa`). Tanto Pesants como
 * Artesanos reclutan por este carril (a petición del usuario — reemplaza también el antiguo reclutamiento
 * directo de Artesanos con cobre a secas, `RECLUTAMIENTO.artesanos`, ya retirado). El reclutamiento de Nobleza
 * (vía Gran Fundición) también se retiró — Nobleza como clase de población sigue existiendo sin cambios (Doc
 * 4.1), solo se quitó la posibilidad de convertirla en tropa. `poderBase` es PLACEHOLDER: no estaba en el
 * diseño original (solo equipo/nivel), interpolado a partir de la progresión ya existente en TROPA_CATALOGO
 * (3 → 6 → 12 → 25 en 4 tiers) repartida en estas 10 tropas a lo largo de 3 niveles.
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
}[] = [
  // Escalón de entrada (a petición del usuario: la defensa mínima no debe depender de Barracón — que exige la
  // política "Construir Barracón" del General ANTES de siquiera empezar a construirse, ver `politicas.ts`
  // `politicaActivaDesbloqueaEdificio` — sino de Centro Urbano, el único edificio que nace `activo` con el
  // asentamiento desde el tick de fundación, sin cola de construcción ni gate alguno, ver `settlement.ts`
  // `edificiosIniciales`). Así CUALQUIER asentamiento fundado puede defenderse desde el minuto uno, así sea
  // con la tropa más débil del roster — antes dependía indirectamente de tener madera de sobra para pagar el
  // Barracón (30 madera) y de que el General activara esa política primero. Se paga con madera EN BRUTO, sin
  // pasar por Armería. Débil a propósito (poderBase 2, por debajo de todo lo demás): existe para que el bucle
  // de juego arranque y las primeras escaramuzas ocurran pronto, no para ganar batallas. Sigue exigiendo un
  // General asignado (`reclutarTropa` en engine/tropas.ts) — eso no cambia, solo el edificio.
  { id: 'milicia_lanceros', nombre: 'Milicia de lanceros', edificio: 'centroUrbano', nivelRequerido: 1, costoEquipo: { madera: 2 }, poderBase: 2, unidadesPorDefecto: 25 },
  // Recosteadas a `armaMadera` (ver RECETA_ARMA_MADERA): antes exigían la cadena del cobre/cuero entera, lo
  // que era además temáticamente incoherente — un escudo de MIMBRE pagado con un arma de cobre, y unos
  // Honderos (una honda y una piedra) pagados con armadura de cuero. El cobre pasa a ser la MEJORA
  // (`espadachines_cobre`, que sí lo conserva), no el ticket de entrada.
  { id: 'lanceros_mimbre', nombre: 'Lanceros con escudo de mimbre', edificio: 'barracon', nivelRequerido: 1, costoEquipo: { armaMadera: 1 }, poderBase: 3, unidadesPorDefecto: 20 },
  { id: 'espadachines_cobre', nombre: 'Espadachines de espada corta de cobre', edificio: 'barracon', nivelRequerido: 1, costoEquipo: { armaCobre: 1, armaduraBasica: 1 }, poderBase: 4, unidadesPorDefecto: 20 },
  { id: 'hacheros_ligeros', nombre: 'Hacheros ligeros', edificio: 'barracon', nivelRequerido: 2, costoEquipo: { armaBronce: 1, armaduraBasica: 1 }, poderBase: 7, unidadesPorDefecto: 18 },
  { id: 'espadachines_bronce', nombre: 'Espadachines con espadas y escudos de bronce', edificio: 'barracon', nivelRequerido: 2, costoEquipo: { armaBronce: 2, armaduraIntermedia: 1 }, poderBase: 9, unidadesPorDefecto: 18 },
  { id: 'lanceros_pesados', nombre: 'Lanceros pesados micénicos', edificio: 'barracon', nivelRequerido: 3, costoEquipo: { armaBronce: 2, armaduraIntermedia: 2 }, poderBase: 14, unidadesPorDefecto: 15 },
  { id: 'hacheros_armados', nombre: 'Hacheros armados', edificio: 'barracon', nivelRequerido: 3, costoEquipo: { armaBronce: 1, armaduraIntermedia: 1 }, poderBase: 12, unidadesPorDefecto: 15 },
  { id: 'honderos', nombre: 'Honderos', edificio: 'galeriaDeTiro', nivelRequerido: 1, costoEquipo: { armaMadera: 1 }, poderBase: 5, unidadesPorDefecto: 25 },
  { id: 'escaramuzadores_jabalina', nombre: 'Escaramuzadores con jabalina', edificio: 'galeriaDeTiro', nivelRequerido: 2, costoEquipo: { armaBronce: 1, armaduraBasica: 1 }, poderBase: 8, unidadesPorDefecto: 20 },
  { id: 'arqueros', nombre: 'Arqueros', edificio: 'galeriaDeTiro', nivelRequerido: 2, costoEquipo: { armaBronce: 1, armaduraIntermedia: 1 }, poderBase: 9, unidadesPorDefecto: 25 },
  { id: 'arqueros_compuesto', nombre: 'Arqueros con arco compuesto', edificio: 'galeriaDeTiro', nivelRequerido: 3, costoEquipo: { armaBronce: 3, armaduraIntermedia: 2 }, poderBase: 15, unidadesPorDefecto: 20 },
];

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

/**
 * Nivel de asentamiento — rediseño de progreso (Fase 0): reemplaza por completo la fórmula de puntos anterior
 * (población/edificios activos). Ahora es un modelo de GATES: para subir de nivel hace falta cumplir a la vez
 * un mínimo de población (pesants + artesanos) Y tener construidos (activos) los edificios listados. Tope de
 * Fase 0 = nivel 3 (Doc 4.5). El nivel sube de forma MONÓTONA (nunca baja si la población cae después).
 */
export const NIVEL_ASENTAMIENTO = {
  nivelMaximo: 3,
  requisitos: {
    2: { pesants: 200, artesanos: 50, edificios: ['armeria', 'curtiduria', 'fundicion'] },
    3: { pesants: 500, artesanos: 200, edificios: ['carpinteria', 'barracon', 'galeriaDeTiro'] },
  } as Record<number, { pesants: number; artesanos: number; edificios: string[] }>,
};

/**
 * Mantenimiento (Doc 4.5): coste periódico que escala por nivel (sumando materiales, no reemplazando) y por
 * distancia al centro de poder de la Facción (aquí: su asentamiento más antiguo vivo, como proxy de "capital").
 * Cantidades y velocidad de degradación son PLACEHOLDER (Preguntas_Abiertas no fija cifras exactas).
 */
export const MANTENIMIENTO = {
  medidorInicial: 100,
  // Trigo NO va aquí (fix: era una "mecánica repetida" — Mantenimiento cobraba este valor fijo ADEMÁS del
  // consumo real de comida que ya se descuenta en `consumirComida`/`avanzarMantenimientoTropas`, duplicando
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
 * Reserva mínima que la auto-construcción (y la manual) NUNCA puede tocar al COMPROMETER (pagar) un proyecto
 * nuevo (ver `puedeIniciarConstruccion`/`evaluarNecesidades` en engine/construction.ts). Overhaul: reemplaza
 * los umbrales fijos anteriores (madera 30, trigo 20, piedra 20, oro 10 — pensados para el asentamiento
 * inicial, sin escalar nunca) por una reserva PROYECTADA: cuánto va a cobrar Mantenimiento/comida en los
 * próximos N ticks (ver `reservaDinamicaConstruccion`, engine/mantenimiento.ts, que reutiliza
 * `calcularCostoMantenimiento`). Corrige un colapso real documentado: tras subir de nivel (piedra/oro
 * entran a cobrarse) la reserva fija se quedaba corta frente al mantenimiento real ya escalado. Cifras de
 * horizonte PLACEHOLDER (ver Preguntas_Abiertas.md), calibradas contra `FUNDACION.materialesIniciales`: con
 * el stock inicial de madera (50) y el costo base de Mantenimiento a nivel 1 (3/tick), un horizonte de 15
 * ticks reservaría 45 — casi todo el stock inicial, congelando cualquier construcción no exenta (Vivienda,
 * extractores...) durante los primeros ticks incluso en un asentamiento sano con acceso a bosque. 8 ticks dan
 * un margen real (protege contra el patrón de colapso post-ascenso de nivel ya documentado) sin dejar al
 * asentamiento recién fundado sin margen de maniobra.
 *
 * Excepción deliberada (sin cambios): Granja no respeta la reserva de trigo, ni Leñera la de madera (ver
 * `engine/construction.ts`) — son las únicas vías reales de recuperar esos recursos, así que bloquearlas
 * por la misma escasez que deben resolver sería un huevo-y-la-gallina sin salida (rompería en seco la
 * política "Protección de Riesgos").
 */
export const RESERVA_CONSTRUCCION = {
  horizonteTicksMantenimiento: 8,
  horizonteTicksComida: 8,
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
