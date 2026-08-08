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
  // Capacidad de Leñeras por bosque según su tamaño (a petición del usuario): un bosque grande admite más de
  // una Leñera trabajando en él a la vez, mín 1 / máx 3 — ver `capacidadLenerasBosque` en engine/construction.ts.
  // Umbrales repartidos en tercios del rango radioMin-radioMax (30-80): <45 -> 1, 45-64 -> 2, >=65 -> 3.
  capacidadLenerasPorRadio: { umbral2: 45, umbral3: 65 },
};

// Fertilidad: ruido continuo por suma de funciones seno con distintas frecuencias (sin dependencias externas).
export const FERTILIDAD = {
  octavas: 3,
  escala: 0.006,
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
}

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
  // Oro: metal precioso en bruto, origen en minas igual que cualquier otro recurso (Doc 3.1).
  mina: { costo: { madera: 40, piedra: 10 }, tiempoConstruccionTicks: 6, produccionBaseOro: 2, trabajadoresRequeridos: 6 },
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
  maximoEnCola: 4, // tope de edificios en estado 'en_cola' simultáneos (auto-construcción y manual comparten el mismo cupo)
  // Rebalance: sin esto, Vivienda/Almacén/Taller podían copar los `maximoEnCola` slots con proyectos
  // atascados por falta de recursos y dejar a Granja/Leñera —los recursos "de supervivencia" de los que
  // depende TODO lo demás, incluido el Mantenimiento— sin hueco para encolarse nunca (interbloqueo real
  // detectado en juego: asentamientos cayendo en ruinas por falta de madera con la Leñera siempre última).
  slotsReservadosSupervivencia: 1,
  // Rediseño de progreso (Fase 0, bug detectado en simulación): con Curtiduría/Armería/Fundición compitiendo
  // por los mismos slots generales que Cantera/minas, ambas podían quedar atascadas esperando piedra
  // (Curtiduría/Armería cuestan piedra) sin que Cantera —su única fuente— consiguiera nunca un hueco para
  // encolarse, porque Curtiduría/Armería ya ocupaban los slots generales desde antes de que hubiera zona
  // suficiente para alcanzar un nodo de piedra. Mismo patrón que el interbloqueo de Granja/Leñera: 1 slot
  // reservado EXCLUSIVAMENTE para los extractores base (cantera/minaCobre/mina/minaEstano/corral) — maximoEnCola
  // sube de 3 a 4 para no reducir la concurrencia general disponible al resto de edificios.
  slotsReservadosExtractores: 1,
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
 */
export const TROPAS_RECLUTABLES: {
  id: string;
  nombre: string;
  edificio: 'barracon' | 'galeriaDeTiro';
  nivelRequerido: number;
  costoEquipo: Partial<Record<string, number>>;
  poderBase: number;
}[] = [
  { id: 'lanceros_mimbre', nombre: 'Lanceros con escudo de mimbre', edificio: 'barracon', nivelRequerido: 1, costoEquipo: { armaCobre: 1 }, poderBase: 3 },
  { id: 'espadachines_cobre', nombre: 'Espadachines de espada corta de cobre', edificio: 'barracon', nivelRequerido: 1, costoEquipo: { armaCobre: 1, armaduraBasica: 1 }, poderBase: 4 },
  { id: 'hacheros_ligeros', nombre: 'Hacheros ligeros', edificio: 'barracon', nivelRequerido: 2, costoEquipo: { armaBronce: 1, armaduraBasica: 1 }, poderBase: 7 },
  { id: 'espadachines_bronce', nombre: 'Espadachines con espadas y escudos de bronce', edificio: 'barracon', nivelRequerido: 2, costoEquipo: { armaBronce: 2, armaduraIntermedia: 1 }, poderBase: 9 },
  { id: 'lanceros_pesados', nombre: 'Lanceros pesados micénicos', edificio: 'barracon', nivelRequerido: 3, costoEquipo: { armaBronce: 2, armaduraIntermedia: 2 }, poderBase: 14 },
  { id: 'hacheros_armados', nombre: 'Hacheros armados', edificio: 'barracon', nivelRequerido: 3, costoEquipo: { armaBronce: 1, armaduraIntermedia: 1 }, poderBase: 12 },
  { id: 'honderos', nombre: 'Honderos', edificio: 'galeriaDeTiro', nivelRequerido: 1, costoEquipo: { armaduraBasica: 1 }, poderBase: 5 },
  { id: 'escaramuzadores_jabalina', nombre: 'Escaramuzadores con jabalina', edificio: 'galeriaDeTiro', nivelRequerido: 2, costoEquipo: { armaBronce: 1, armaduraBasica: 1 }, poderBase: 8 },
  { id: 'arqueros', nombre: 'Arqueros', edificio: 'galeriaDeTiro', nivelRequerido: 2, costoEquipo: { armaBronce: 1, armaduraIntermedia: 1 }, poderBase: 9 },
  { id: 'arqueros_compuesto', nombre: 'Arqueros con arco compuesto', edificio: 'galeriaDeTiro', nivelRequerido: 3, costoEquipo: { armaBronce: 3, armaduraIntermedia: 2 }, poderBase: 15 },
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
 * Reserva mínima que la auto-construcción (y la manual) NUNCA puede tocar en los recursos que en ese
 * momento cobra Mantenimiento (madera+trigo siempre, +piedra/oro según nivel — ver `MANTENIMIENTO` y
 * `recursosProtegidosPorMantenimiento` en engine/mantenimiento.ts). Un edificio en cola solo arranca si
 * `disponible - costo >= reserva` en cada recurso protegido de su costo; si no, se queda esperando en cola
 * (mismo comportamiento que ya existía cuando faltaban recursos del todo).
 *
 * Excepción deliberada: Granja no respeta la reserva de trigo, ni Leñera la de madera (ver
 * `engine/construction.ts`) — son las únicas vías reales de recuperar esos recursos, así que bloquearlas
 * por la misma escasez que deben resolver sería un huevo-y-la-gallina sin salida (rompería en seco la
 * política "Protección de Riesgos").
 */
export const RESERVA_CONSTRUCCION = {
  madera: 30,
  trigo: 20,
  piedra: 20,
  oro: 10,
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
