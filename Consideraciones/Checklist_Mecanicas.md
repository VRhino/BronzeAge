# Checklist de Mecánicas

Lista completa de mecánicas del juego, organizadas por sistema. **Cada entrada lleva dos estados
independientes**, porque durante mucho tiempo se confundieron y el documento acabó diciendo "implementado"
de cosas que ya no existen en el repo:

| Marca | Significado (DISEÑO) |
|---|---|
| ✅ | decidido y cerrado |
| 🔶 | en discusión, con hueco pendiente |
| ❌ | descartado explícitamente |

| Marca | Significado (CÓDIGO, verificado contra `src/` el 2026-09-06) |
|---|---|
| `código: ✔` | implementado y en uso |
| `código: ◐` | parcialmente implementado — se detalla qué falta |
| `código: ✘` | no hay nada en `src/` |
| `código: —` | no aplica (ambientación, roadmap, decisión de alcance) |

Ver `Docs/Game/0_Glosario_de_Entidades_Politicas.md` para las definiciones centrales.
Las mecánicas **pendientes** son la lista de `Docs/Mecanicas a desarrollar.md`; aquí solo aparecen el resumen
y el estado, con un puntero a su sección allí.

---

## Auditoría — lo que NO está en el código (revisado 2026-09-09)

Diseñado (a veces "cerrado") pero **sin una línea en `src/`**:

- **Tecnología**: no existe. Ni árbol, ni las 3 vías (comercio / desarrollo propio / Aedas), ni compra con
  oro. Lo único que hay con el nombre "Aedas" es la narración de cambios de título (`engine/titulos.ts`).
  (`Mecanicas a desarrollar.md` §20)
- **Gremios** (los 4 gremios escasos a nivel de servidor, con sus 4 requisitos y su tirada periódica):
  `patioDeGremios` en `constants.ts` es solo una parcela decorativa del trazado urbano, nada más.
  (`Mecanicas a desarrollar.md` §21)
- **Exilio** como política de soberanía (Doc 5.9). (`Mecanicas a desarrollar.md` §22)
- **Attack Timer** (Doc 5.6) — estaba marcado como pospuesto, sigue pospuesto.
- **Comercio marítimo / unidades navales** — fuera de alcance de Fase 0, coherente.
- **Ciclo de servidor de 12 meses + legado NPC de la Facción ganadora** (el edificio Maravilla sí está).
  (`Mecanicas a desarrollar.md` §24)
- **Trueque compuesto** de varios materiales — `AcuerdoTrueque` es 1 recurso ↔ 1 recurso.
  (`Mecanicas a desarrollar.md` §5)
- ~~**Revamp de caravanas**~~ — **NÚCLEO IMPLEMENTADO 2026-09-08** (ver Comercio y economía más abajo). Solo quedan los trozos diferidos (planificación horaria, cría, visibilidad por tamaño, camello/desierto, catálogo de carros, unificación con `Ejercito.suministro`) — `Mecanicas a desarrollar.md` §8.1.
- **Eventos de asentamiento** y **landmarks del mundo** (`Mecanicas a desarrollar.md` §9, §10); **la capital
  como decisión** (§13).
- **Progresión de Liderazgo del jugador**: `Jugador.liderazgoBase` existe y el propio código anota que el
  efectivo debería ser base + progresión, pero esa progresión no está ni diseñada ni implementada.
  (`Mecanicas a desarrollar.md` §11)
- ~~**Visión compartida por alianza**~~ — **HECHO 2026-09-09** (Paso 4, ampliado a vasallaje;
  `Niebla_De_Guerra_Definicion.md` §5.6).
- **Materiales exóticos** (los pide el diseño de la Maravilla — `Mecanicas a desarrollar.md` §23) y **NPCs
  hostiles** más allá de los bandidos (§9).

**Cerrado desde la auditoría anterior (2026-09-06 → 2026-09-08):** impuestos / el oro como presupuesto
(`Economia_Del_Oro_Definicion.md`, motor hecho, calibrando); el jugador como entidad situada + su movimiento
libre + el spawn aleatorio de onboarding (`Jugador_Situado_Definicion.md`, 2026-09-07); comerciar con una
plaza ajena desde su puerta (`Comercio_Fisico_Definicion.md`, 2026-09-07).

Y una corrección en sentido contrario: **los chokepoints ya no existen**. Estaban marcados aquí como
implementados; `WORLDGEN_VERSION` v15 (2026-08-26, decisión explícita del usuario) los retiró por completo —
geometría, control por zona de influencia, peaje en oro, tipo de dominio y renderizado.

---

## Concepto y ambientación
- ✅ Ambientación: Edad Oscura de la Edad de Bronce — `código: —`
- ✅ Mapa enorme y desconectado al inicio, crecimiento deliberadamente lento — `código: ✔` (`worldgen/`)

## Fundación y zonas de influencia
- ✅ Fundación libre, zona de influencia automática, reglas de construcción, fronteras con límite duro — `código: ✔` (`engine/zones.ts`, `engine/settlement.ts`)
- ✅ Fundación grupal (hasta 5 jugadores) — `código: ✔` (`engine/settlement.ts`)
- ✅ Onboarding: spawn aleatorio del jugador en el mundo abierto — `código: ✔` (2026-09-07, parte del jugador situado — `Jugador_Situado_Definicion.md` §5 paso 9)
- ❌ **Chokepoints estratégicos — DESCARTADOS (2026-08-26, decisión del usuario: "no me está dando nada en este momento")** — `código: ✘`. Retirados por completo en `WORLDGEN_VERSION` v15: `worldgen/chokepoints.ts`, `engine/chokepoints.ts`, `CHOKEPOINTS_PEAJE`, el tipo `Chokepoint` y su renderizado. El resto del mundo sale bit a bit idéntico para la misma seed.
- ✅ Cap de fundación por Facción, escala con el nivel de Facción (1→7) — `código: ✔` (`CAP_FUNDACION_POR_NIVEL`)
- ✅ Período de gracia sin cobro de Mantenimiento al fundar — `código: ✔` (`MANTENIMIENTO.graciaMinutos = 60`)
- ✅ Radio inicial de zona de influencia 30, techo por nivel de asentamiento — `código: ✔` (`ZONA_INFLUENCIA.radioMaximoPorNivel`, ampliado a 5 niveles: 60/90/120/150/180)
- ✅ **Nivel de Facción por EXPERIENCIA** — reemplaza la fórmula por población; sube por combate, construcción, conquista y defensa/ataque de caravana, y es monótono — `código: ✔` (`NIVEL_FACCION`, `engine/faccion.ts` `aplicarAjustesExperiencia`). *Cierra el 🔶 que este documento arrastraba sobre "qué hace subir el nivel de Facción".* La curva `xpParaNivel` sigue siendo placeholder.
- ✅ Caravana de Fundación: coste, gates de nivel 2 + poder pagar, manual, interceptable, desarmable con recuperación completa — `código: ✔` (`engine/expansion.ts`, comandos `lanzarCaravanaFundacion` / `desarmarCaravanaFundacion`)
- ✅ Cooldown compartido de creación de caravanas (Fundación y comercial) — `código: ✔` (`CARAVANA_COOLDOWN`)

## Tecnología
- ✅ Tres vías de acceso (comercio, desarrollo propio, Aedas); sistema de Aedas; tecnología se compra con oro — **`código: ✘` — no existe nada** (`Docs/Mecanicas a desarrollar.md` §20)
- ✅ Narración de Aedas/Poetas al cambiar de manos un título de servidor — `código: ✔` (`engine/titulos.ts`, lo único "Aeda" que hay)
- 🔶 Árbol tecnológico del desarrollo propio: estructurado o libre/emergente — `código: ✘`

## Comercio y economía
- ✅ Comercio siempre manual; caravanas terrestres — `código: ✔` (`engine/trade.ts`)
- ✅ Comercio marítimo — `código: —` (fuera de Fase 0, por diseño)
- ✅ Economía dual (oro + trueque); oro = metal precioso, origen en minas — `código: ✔`
- ✅ Acuerdos de trueque; órdenes de mercado; precios dinámicos; comisiones más bajas intra-Facción — `código: ✔` (`engine/trade.ts`, `engine/market.ts`, `PRECIO_BASE`/`PRECIO_REFERENCIA`, `COMISION`)
- ✅ Categorías de caravana; bonificación por distancia; dependencia logística — `código: ✔` (`CARAVANA_CATALOGO`)
- ✅ **Flota propia de caravanas comerciales**: activo persistente que el jugador construye y conserva, con costo — `código: ✔` (`construirCaravanaComercial`)
- ✅ **Asignación automática de caravanas a lados pendientes de trueque** por scoring (urgencia de expiración / volumen / cercanía) — `código: ✔` (`ASIGNACION_CARAVANA`). Es el sustituto de Fase 0 de la carga manual del diseño objetivo.
- ✅ Caminos comerciales automáticos: se generan al proponer trueque, con pathfinding que evita terreno costoso, y dan bonus de velocidad — `código: ✔` (`engine/caminos.ts`, `world/rutas.ts`, `COSTE_MOVIMIENTO.factorCamino`)
- 🔶 Qué pasa con el camino si se rompe la relación que lo originó — hoy queda como infraestructura permanente — `código: —` (ver `Preguntas_Abiertas.md`)
- 🔶 **Rutas de caravana avanzadas**: rodear bosques, no cruzar ríos, fusión de caminos próximos en caminos principales por puntos de uso, paso obligado y comisión al cruzar una ciudad intermedia, inmunidad al cruzar ciudad neutral o aliada — `código: ◐`. El pathfinder por coste de terreno y la impasabilidad del agua sí están; los puntos de uso, la fusión de caminos, la comisión de paso y la inmunidad, no. Ficha en `Docs/Mecanicas a desarrollar.md` §3.
- ✅ **Revamp de caravanas — NÚCLEO IMPLEMENTADO (2026-09-08, Pasos 1-5)** — `código: ✔`: la caravana `comercial` es un contenedor de carros (cada uno con un animal) y deriva capacidad/velocidad (`capacidadCaravana`/`velocidadCaravana`, `engine/caravanas.ts`; el viejo `CARAVANA_CATALOGO.comercial` fijo se borró). Casco vacío gratis (`crearCaravana`) + piezas (`agregarCarroCaravana` 20/40 madera, `comprarAnimalCaravana` buey 30 madera / caballo 60 oro / camello 40 oro, `moverCarroCaravana`); `reservadaManual` fuera del reparto automático; **lanzamiento manual** con estado `'preparando'` cancelable (`prepararCaravana`/`cancelarCaravana`, `CARAVANA_PREPARACION`); **escolta sin héroe** (`Caravana.escolta`, `CARAVANA_ESCOLTA`, `cupoEscolta`; escuadrones cedidos por viaje que salen de la guarnición, cuentan Liderazgo, resuelven combate por `poderTotal` y vuelven a casa derrotados); migración de snapshot v11→v12. Batch NPC **bit-idéntico** en los cinco commits. Descartado por medición: el bootstrap del Mercado. Reglas en `Docs/Game/3` §3.13; decisiones y plan en `Consideraciones/Revamp_Caravanas_Definicion.md`. Todas las cifras son placeholder (calibración continua). **Quedan solo los trozos diferidos**: planificación horaria, cría, visibilidad por tamaño, inmunidad del camello al desierto, catálogo ampliado de carros, unificación con `Ejercito.suministro` (ver `Docs/Mecanicas a desarrollar.md` §8.1).
- 🔶 **Trueque compuesto** de varios materiales por lado — **`código: ✘`**, `AcuerdoTrueque` es 1 recurso ↔ 1 recurso.
- ✅ **Comerciar con una plaza ajena desde su puerta** (mercader visitante): estando en la puerta de un asentamiento que no es tuyo, abrir su escaparate, ver sus órdenes activas y comprarle o venderle contra su carro — `código: ✔` (2026-09-07): `comerciarEnPlaza` (motor + comando), escaparate visible solo para quien está en la puerta, oro que pesa y ocupa carro, comisión externa conservada. Diseño en `Comercio_Fisico_Definicion.md` (§8 puntos abiertos: publicar órdenes propias, cerrar mercado sin cerrar puerta). Reglas Doc 3 §3.2/§3.3/§3.5/§3.7/§3.8. Migración v10→v11.
- ✅ **Impuestos**: generación de oro en el asentamiento según población y clase social — `código: ✔` (motor 2026-09-08, calibración Paso 6 en curso). `recaudacionOro` + política Presión Fiscal + reclutamiento/animales en oro. Diseño y calibración en `Economia_Del_Oro_Definicion.md`. Canon Doc 3 §3.1/§3.13.2, Doc 4 §4.1/§4.4/§4.5, Doc 5 §5.8.
- 🔶 Uso de riqueza por comisiones; fórmula exacta de comisión — `código: ◐` (la fórmula existe, con cifras placeholder)

## Recursos
- ✅ Recursos desiguales en el mapa; generación de mundo por seed (elevación suavizada, biomas, bosques, ríos, fertilidad, nodos, regiones) — `código: ✔` (`worldgen/`, `WORLDGEN_VERSION` 15)
- ✅ Fuentes diferenciadas (cultivos por fertilidad+granja, madera por bosques-zona, minerales dispersos) — `código: ✔`
- ✅ Comida básica vs commodities de nobleza; madera/piedra; livestock capturable — `código: ✔`
- ✅ Cadenas de producción invisibles; almacenamiento con límites — `código: ✔` (`LINEAS_PRODUCCION`, `ALMACEN`, tope de Almacenes por nivel)
- ✅ **Regeneración de nodos agotados** con cooldown por tipo (metales / livestock) — `código: ✔` (`REGENERACION_NODOS`)
- 🔶 Lista completa de cultivos/livestock; pasos de procesamiento adicionales — `código: ◐`
- 🔶 Materiales **exóticos** (los pide el diseño de la Maravilla) — `código: ✘` (`Docs/Mecanicas a desarrollar.md` §23)

## Población
- ✅ 3 clases (Pesants, Artesanos, Nobleza) con roles, condiciones de aparición y fórmulas propias — `código: ✔` (`engine/population.ts`)
- ✅ Jugadores como entidad separada; reclutamiento por pool específico — `código: ✔`
- ✅ Disparador de Artesanos = primer edificio de transformación construido; tope por trabajadores requeridos — `código: ✔`
- ✅ **Techo de población total por nivel de asentamiento** (300 / 1500 / 6000 / 12000 / 20000) — `código: ✔` (`NIVEL_ASENTAMIENTO.techoPoblacion`)
- ✅ Hambruna: medidor `nutricionPoblacion` 0-100, factor de crecimiento lineal, pérdida del 5%/tick de pesants+artesanos con nobleza protegida — `código: ✔` (`POBLACION.hambre`, `hambruna.test.ts`)
- 🔶 Calibración de las cifras de Hambruna y de la curva de Artesanos; criterio exacto de cola de prioridad de reclutamiento — `código: ◐` (placeholders)

## Construcción automática y trazado urbano
- ✅ El jugador no elige ubicación ni tipo (excepto fundación y adición manual a la cola) — `código: ✔`
- ✅ Algoritmo por reglas; crecimiento por necesidad con reevaluación continua; Maestro de Obras — `código: ✔` (`engine/construction.ts`)
- ✅ Reemplazo automático de extractores cuando su fuente se agota, con tope por tipo — `código: ✔` (`EXTRACCION_MAXIMOS`, `EXTRACTOR_DESEMPATE`)
- ✅ Catálogo ampliado (Corral, Armería, Curtiduría, Carpintería, Palacio, Barracón, Galería de tiro, Gran Fundición, Maravilla…) con recetas multi-nivel — `código: ✔` (`EDIFICIO_CATALOGO`)
- ✅ Cola de PRIORIDAD por categoría (supervivencia > extractores > general) al gastar recursos — `código: ✔`
- ✅ Disparador de Granja por déficit real, hasta 3 a la vez; capacidad de Leñeras por tamaño de bosque — `código: ✔` (`LENERA_POR_BOSQUE`)
- ✅ Reserva de construcción calibrable, y no buscar sitio para lo que no se puede pagar — `código: ✔` (`RESERVA_CONSTRUCCION`, `noBuscarSinMateriales.test.ts`)
- ✅ **Trazado urbano dinámico sobre rejilla** (Etapas 1-6 de `Vista_Asentamiento_Trazado_Urbano.md`): árbol único de anclas, calles como CELDAS y no aristas, conectividad como condición de validez, afinidad entre edificios del mismo tipo, afueras que crecen hacia afuera, parcelas decorativas (plaza, plaza de armas, patio de gremios, pozo, parque, puestos de mercado por nivel). Huellas a escala de calle desde BA-005 (2026-09-13, §E6.24): resto ÷2, Centro Urbano 4×4, partidas de otra geometría rechazadas por `LAYOUT_VERSION` — `código: ✔` (`engine/trazado.ts`, `TRAZADO`, `REJILLA_ASENTAMIENTO`, `EDIFICIO_TAMANO`)
- ✅ **Perfiles de trazado como ordenanza del Maestro de Obras** (`nucleos` / `caminera` / `compacta` / `gremial`) — `código: ✔` (`PERFILES_TRAZADO` y las políticas `postura_defensiva` / `arterias_comerciales` / `barrios_gremiales` / `plazas_mayores`). **Esto es lo que cubre la entrada "políticas de ubicación de construcción"** del índice de pendientes: el jugador orienta *cómo* se distribuye la ciudad; dónde va cada edificio concreto sigue siendo automático por invariante de diseño.
- ✅ Caché de búsqueda de colocación: la búsqueda deja de repetirse cada tick — `código: ✔` (`cacheTrazado.test.ts`)

## Murallas y recintos *(mecánica nueva, no estaba en este documento)*
- ✅ Diseño cerrado en `Consideraciones/Murallas_Definicion.md` (2026-08-31) — `código: ✔` (`engine/muralla.ts`, `MURALLA`, comandos `comprometerRecinto` / `abandonarRecinto` / `mejorarRecinto`)
- ✅ El recinto NO es un `EdificioTipo`: entidad propia con trazo de celdas de muro, puertas geométricas congeladas con el trazo, torres por paso y niveles 1-3 (empalizada → muralla de piedra) — `código: ✔`
- ✅ Obra progresiva con coste por celda y upkeep por celda; ampliación de recinto; arrabal / extramuros — `código: ✔` (`costoDeTrazo`, `avanzarObraDeRecintos`, `upkeepDeRecintos`, `edificiosExtramurosDe`)
- ✅ Multiplicador defensivo por recinto (1.3 / 1.8 / 2.5) aplicado en asedio — `código: ✔` (`multiplicadorDefensivoDeRecintos`)
- ✅ Sustituye al viejo gate por edificio: subir a nivel 4 exige un recinto TERMINADO de nivel ≥1 — `código: ✔`
- 🔶 Calibración de tarifas, upkeep y bonos defensivos — `código: ◐` (placeholders)

## Escala social y política
- ✅ Glosario central: Jugador → Facción → Asentamientos; Liga = red de Facciones — `código: ✔`
- ✅ Progresión: independiente → vasallaje/alianza → Liga → Gran Rey (prestigio) — `código: ✔` (`engine/liga.ts`, `engine/titulos.ts`)
- ✅ Reglas de vasallaje completas (tributo, defensa mutua, 4 vías de ruptura) — `código: ✔` (`engine/diplomacia.ts`, comando `rebelionVasallo`)
- ✅ Coste de gobernanza absorbido en Mantenimiento — `código: ✔`
- 🔶 Identidad visual (sigilo / estandarte) — `código: ✘` (`Docs/Mecanicas a desarrollar.md` §27)

## Ciudadanía
- ✅ Ligada a la Facción; obtención por fundación o compra de casa; cupo de casas por nivel; cooldown anti-abuso al crear Facción — `código: ✔` (`CIUDADANIA`, `engine/faccion.ts` `comprarCasa`)
- 🔶 Los 5 beneficios (residencia en cualquier asentamiento, protección militar, voz en política exterior, nivel intermedio de comisiones…) — `código: ◐`: solo el de comisiones y el de residencia/reclutamiento están en el motor (`Docs/Mecanicas a desarrollar.md` §26)

## Cargos
- ✅ Nivel Facción (Rey, Embajador) y nivel asentamiento (Gobernador, Tesorero, General, Maestro de Obras, Sacerdote) — `código: ✔` (`engine/cargos.ts`)
- ✅ Multi-cargo permitido; liberación tras inactividad — `código: ✔`

## Mantenimiento
- ✅ Medidor 0-100, coste periódico escalonado por nivel y por distancia al centro de poder — `código: ✔` (`engine/mantenimiento.ts`, `MANTENIMIENTO`)
- ✅ Nivel de asentamiento por GATES (población + edificios específicos) — `código: ✔`
- ✅ **Expansión de niveles 3 → 5 (Fase 0.6)**: nivel 2 = 200 pesants + 3 de 7 extractores; nivel 3 = 500 pesants + 200 artesanos + armería/curtiduría/fundición/barracón/galería; nivel 4 = 1000/400 + recinto terminado; nivel 5 = 2000/800 + palacio — `código: ✔` (`NIVEL_ASENTAMIENTO.requisitos`). *Este documento decía "tope de Fase 0 = nivel 3"; ya no es cierto.*
- ✅ Tope de Almacenes por nivel (`NECESIDADES.maximoAlmacenesPorNivel`, cuenta activos + en obra + en cola, y aplica igual a la adición manual) y tope de Viviendas derivado del gate del nivel siguiente y del techo de población, no de una constante propia (`alcanzoTopeDeViviendas`, `engine/construction.ts`) — `código: ✔`
- ✅ Período de gracia al fundar — `código: ✔`
- ✅ Degradación proporcional del medidor; a 0 el asentamiento colapsa, con razón auditable de qué recurso faltó y cuánto duró el déficit — `código: ✔`
- ✅ El trigo NO es coste fijo de Mantenimiento: se descuenta una sola vez como consumo real de población + raciones de tropa — `código: ✔`
- ✅ Mantenimiento base en madera reducido a la mitad (3 → 1.5) porque inflaba la reserva de construcción hasta bloquear todo gasto discrecional — `código: ✔`
- 🔶 Cantidades finales por nivel, en qué nivel entran piedra y oro, velocidad de degradación, tope de extractores por tipo — `código: ◐` (placeholders)
- 🔶 **La capital como decisión del jugador**: hoy `encontrarCapital` devuelve el asentamiento vivo más antiguo, marcado en el propio código como *placeholder = proxy de capital*. De ahí sale un factor de distancia real (×1 en la capital, hasta ×2 a distancia 400, topado a partir de ahí), así que no es cosmético: es el anti-snowball de cohesión. No se elige, no se traslada, el Palacio no pinta nada, y el tope a 400 desactiva el anti-snowball más allá de ~5 provincias — `código: ✘` como mecánica. Ficha en `Docs/Mecanicas a desarrollar.md` §13.

## Especialización y dependencia entre asentamientos (Fase 0.5) *(no estaba en este documento)*
- ✅ Cupo de asentamientos por nivel, ligado al nivel de Facción — `código: ✔` (`CUPO_NIVEL_ASENTAMIENTO`)
- ✅ El suministro va por trueque o por comercio activo con otra Facción; cortarlo CONGELA capacidad, no reduce mantenimiento ni purga población ya asentada — `código: ✔`
- ❌ Roles / vocación de asentamiento, costes en materiales procesados, transformación a escala, ejército como vector de demanda, huella urbana contra tierra cultivable — descartados en el propio documento de la fase — `código: —`
- 🔶 Calibración de los cupos y del techo de población — `código: ◐`

## Entrada tardía y mundo lleno
- ✅ Mapa difícil de saturar; nuevos servidores; el deterioro libera zonas — `código: ◐` (el colapso por mantenimiento libera zona; "nuevos servidores" es infraestructura, ver Roadmap)

## Guerra y combate
- ✅ Combate héroe+tropa; formaciones y cohesión; permadeath + squad persistente; doble carril de progresión — `código: ✔` (`engine/combate.ts`, `MILITAR`)
- ✅ Las 4 modalidades de batalla instanciadas: **asedio**, **encuentro en campo abierto**, **intercepción de caravana** y **ataque a campamento de bandidos** — `código: ✔` (`iniciarAsedio`, `asediarConEjercito`, `encuentroEntreEjercitos`, `interceptarCaravanaConEjercito`, `atacarCampamentoBandidos`)
- ✅ Roster de tropas por edificio y nivel interno, con costo en equipo fabricado en Armería; Nobleza vía Gran Fundición — `código: ✔` (`TROPAS_RECLUTABLES`, `engine/tropas.ts`)
- ✅ La veteranía sube el poder del MISMO escuadrón y nunca cambia su `tropaId`; el código muerto del sistema de tiers se retiró por completo — `código: ✔`
- ✅ Moral por raciones, con deserción permanente a moral 0 — `código: ✔`
- ✅ Conquista tras asedio + **ocupación post-conquista** (guarnición = ejército conquistador, saqueo determinista, ventana de ocupación: inmune a nuevo asedio, recaudación/crecimiento ×0.5, mantenimiento congelado) — `código: ✔` (`aplicarConquista`, `estaOcupado`, `OCUPACION`; Doc 5.12.9; cifras placeholder, calibración batch en curso)
- ✅ Reclutar/reponer/mover tropa fuera de la residencia (nuevo escuadrón = residencia; reponer/mover = cualquier plaza propia con permiso, estando presente) + comando `cambiarResidencia` — `código: ✔` (`puedeReclutarEn`, `cambiarResidencia`; Doc 2.5, 5.8)
- ✅ `guarnecer` (marchar un ejército a una plaza propia y volcar la tropa en su guarnición) — `código: ✔` (2026-09-09). Comando `guarnecer` (`session/comandos/presencia.ts`), motor `guarnecer` (`engine/ejercitos.ts`). Las caravanas adjuntas quedan en estado `'aparcada'` en la plaza anfitriona (intercambian con su almacén vía `moverCargaCaravanaAparcada`, salen enganchadas a un ejército o con `enviarCaravanaAlOrigen`). Sin migración. Diseño y plan en `Ocupacion_Post_Conquista_Definicion.md` §2.3/§2.3d/§11.
- ✅ Adaptación temática completa (cobre / estaño) — `código: ✔`
- ✅ **Attack Timer** — decidido y **pospuesto** a fase posterior a Fase 0 (Doc 5.6) — `código: ✘`
- ✅ **Exilio** como política de soberanía (Doc 5.9 / 2.8) — diseño cerrado, **`código: ✘`** (`Docs/Mecanicas a desarrollar.md` §22)
- 🔶 Declaración formal de guerra — `código: ✘` (`Docs/Mecanicas a desarrollar.md` §28); unidades navales — fuera de alcance de Fase 0
- 🔶 Establos / carros de guerra sin edificio de reclutamiento definido; `poderBase` de las tropas es placeholder — `código: ◐`
- 🔶 Varianza de combate (abierta en `Movimiento_Ejercitos_Definicion.md` §1.4) — `código: ◐`

## Ejércitos, movimiento y suministro en campaña *(mecánica nueva, no estaba en este documento)*
- ✅ Diseño cerrado y mecánica **cerrada el 2026-09-04** (`Consideraciones/Movimiento_Ejercitos_Definicion.md`, reglas en Doc 5.11-5.13) — `código: ✔`
- ✅ Un ejército es la única forma de salir del asentamiento; un solo destino, y los encuentros salen de la geometría — `código: ✔` (`engine/ejercitos.ts`, `engine/movimiento.ts`). ⚠️ **Las dos últimas mitades las revisa el diseño del jugador situado** (2026-09-06): "un solo destino" pasa a ser rectificable y "los encuentros salen de la geometría" pasa a "la geometría ofrece, el jugador declara" — ver `Consideraciones/Jugador_Situado_Definicion.md` §1.1c.
- ✅ **Liderazgo**: coste por escalón de tropa y tope de lo que un jugador puede llevar — `código: ✔` (`engine/liderazgo.ts`, `LIDERAZGO`)
- ✅ Comandos completos: movilizar, unirse, replegar, estacionar, adjuntar / soltar / cargar / entregar caravana, reabastecer aliados — `código: ✔`
- ✅ Suministro en campaña: radio operativo, caravanas adjuntas como tren de suministros, ejército sin nada se disuelve — `código: ✔` (`LOGISTICA`)
- ✅ **Escolta de caravanas**, que llevaba años marcada como "no modelada" en Doc 3.10 — `código: ✔`
- ✅ La guarnición es lo único que defiende un asentamiento — `código: ✔`
- 🔶 Calibración de velocidades, radios y costes de liderazgo — `código: ◐` (continua, por simulación)
- 🔶 **Progresión de Liderazgo del jugador**: `Jugador.liderazgoBase` está, pero el efectivo debería ser base + progresión y esa progresión no está ni diseñada ni implementada — `código: ✘`. Ficha en `Docs/Mecanicas a desarrollar.md` §11.

## Niebla de guerra y conocimiento del mundo *(mecánica nueva, no estaba en este documento)*
- ✅ Diseño cerrado 2026-09-04 (`Consideraciones/Niebla_De_Guerra_Definicion.md`, reglas en Doc 5.12.7 / 5.12.8). Tres estados: nunca visto (tapado, terreno incluido) / visto antes (última foto, filtro oscuro) / viéndolo ahora. Regla única: **ver es conocer**.
- ✅ Paso 1 — la VISTA: `ejercitosAvistados` y `asentamientosAvistados` en la proyección, siempre redactados; radios en `VISION` — `código: ✔`
- ✅ Paso 2 — la MEMORIA se graba, por FACCIÓN, sobre rejilla de celdas de 25, y solo crece; migración de snapshot v6 → v7 — `código: ✔` (`engine/exploracion.ts`, `engine/memoria.ts`, `EXPLORACION`)
- ✅ Paso 3 — la memoria se proyecta: `asentamientosConocidos` + máscara `exploracion`; lo visto en vivo gana a lo recordado — `código: ✔`
- ✅ Paso 5 — se pinta: el cliente de jugador tapa lo no explorado y oscurece lo recordado a partir de dos máscaras (`celdas` y `visibles`); el cliente de administración no aplica máscara — `código: ✔`
- ✅ Fronteras ajenas (2026-09-05) — `código: ✔`
- ✅ **Paso 4 — visión compartida por alianza y vasallaje**, en vivo y no "último conocido" — `código: ✔` (2026-09-09). `compartenVision` (`engine/pertenencia.ts`) + ojos aliados solo en la capa "en vivo" de `proyectarParaJugador`; nunca en memoria. Sin migración. `Niebla_De_Guerra_Definicion.md` §5.6.
- 🔶 Paso 6 — calibración del margen de asentamiento y del tamaño de celda — `código: ◐`

## Diplomacia
- ✅ Score de confiabilidad de Facción (-100 a +100), con efecto sobre alianzas y comisiones — `código: ✔` (`engine/reputacion.ts`, `REPUTACION`)
- ❌ Rumores / espionaje — descartado — `código: —`

## Generación del mundo
- ✅ Generación de Fase 0 resuelta y versionada (`WORLDGEN_VERSION` 15) — `código: ✔`
- 🔶 Fases avanzadas: mapa fijo vs procedural, puntos de interés, biomas adicionales — `código: ✘`
- 🔶 **Landmarks reconocibles** que orienten al jugador que explora ("por dónde voy", "qué está cerca de qué") — **`código: ✘`**. Ficha en `Docs/Mecanicas a desarrollar.md` §10.
- 🔶 NPCs hostiles más allá de los bandidos — `código: ✘`

## Progresión de imperio
- ✅ Sandbox sin condición de victoria; títulos dinámicos de prestigio a nivel de servidor, narrados por los Aedas — `código: ✔` (`engine/titulos.ts`)
- 🔶 Diferencia imperio vs alianza grande; fragmentación; catálogo completo de títulos — `código: ◐`
- 🔶 Identidad visual y de audio — `código: ✘` (`Docs/Mecanicas a desarrollar.md` §27)

## Políticas
- ✅ Duración, renovación, múltiples activas, no cancelables; conexión con auto-construcción — `código: ✔` (`engine/politicas.ts`, `POLITICAS`)
- ✅ Slots por cargo (Gobernador 2→5 con el nivel de Facción, Tesorero 2, resto 1) — `código: ✔`
- ✅ **Catálogo concreto**, 15 políticas: `racionamiento`, `culto_fertilidad`, `via_rapida`, `postura_defensiva`, `arterias_comerciales`, `barrios_gremiales`, `plazas_mayores`, `lineas_produccion`, `comercio_abierto`, `aranceles`, `leva_forzosa`, `cupo_caravana_extra`, `carga_ampliada`, `rutas_rapidas`, `edicto_cosecha` — `código: ✔` (`POLITICA_CATALOGO`). *Cierra en gran parte el 🔶 de "catálogo concreto de políticas".*
- ✅ Las 4 políticas "Construir Barracón / Galería / Palacio / Mercado" se retiraron: eso pasó al control manual de cola — `código: ✔`
- ✅ **Exclusión dentro de un slot, resuelta sin regla nueva**: las 4 ordenanzas de trazado + Vía Rápida + Líneas de Producción comparten el único slot de Maestro de Obras, así que ya son mutuamente excluyentes. Como una política dura 150 ticks y nada mueve lo ya construido, cada una deja un ESTRATO en la ciudad — `código: ✔`
- ❌ **"Protección de Riesgos" (Maestro de Obras) ya NO existe** — este documento la daba por viva ("ampliada a 2 Leñeras + 3 Granjas"); no está en `POLITICA_CATALOGO` ni en el cliente. Su función la cubren hoy el tope por tipo de extractor (`EXTRACCION_MAXIMOS.porTipo`) y el disparador de Granja por déficit real — `código: ✘`
- 🔶 Las ordenanzas de trazado no tienen todavía coste/beneficio mecánico propio, así que compiten en desventaja contra Vía Rápida (−25% de tiempo de obra) — `código: ◐` (anotado como pendiente en el propio `constants.ts`; `Docs/Mecanicas a desarrollar.md` §25)

## Fusión y crecimiento de Facciones
- ✅ Menú con 2 opciones (Anexión / Fusión), reglas de Rey y cargos resultantes — `código: ✔` (`engine/fusion.ts`, comandos `anexionar` / `fusionar`)
- 🔶 Aceptación mutua obligatoria vs. anexión forzable unilateralmente — `código: ◐`

## Gremios
- ✅ 4 gremios (Comerciantes, Artesanos, Constructores, Ladrones), escasos a nivel de servidor, con tirada periódica sujeta a 4 requisitos simultáneos (reputación >90, título de servidor específico, nivel de asentamiento en el máximo y mantenimiento >90%); mecanismo de pérdida resuelto — **`código: ✘`**. El `patioDeGremios` de `constants.ts` es una parcela decorativa del trazado urbano, sin relación con esta mecánica. (`Docs/Mecanicas a desarrollar.md` §21)
- 🔶 Beneficios concretos y números — `código: ✘`

## Bandidos y eventos del mundo
- ✅ Campamentos de bandidos en bosques no reclamados: spawn / respawn automático, ataque a caravanas cada tick, ataque manual con recompensa, uno por asentamiento, marcador en el mapa — `código: ✔` (`engine/bandidos.ts`, `CAMPAMENTOS_BANDIDOS`)
- 🔶 **Eventos de asentamiento**: asedio de bandidos con una ventana de horas para organizar y jugar la defensa, y otros por diseñar — **`código: ✘`**. Ficha en `Docs/Mecanicas a desarrollar.md` §9.

## El jugador como entidad en el mundo

> **IMPLEMENTADO (2026-09-07).** Las dos entradas de abajo se diseñaron juntas —la segunda es el verbo de la
> primera— en `Consideraciones/Jugador_Situado_Definicion.md`: treinta y seis decisiones del usuario en once
> rondas, representación en el motor, plan de 10 pasos (todos cerrados), invariantes y abiertos. El movimiento
> es clic a destino (miniatura estilo Total War), así que Fase 0 y Fase 1 comparten modelo y **no hace falta
> netcode**. Canon: Doc 0 (glosario), Doc 1 §1.3/§1.10, Doc 2 §2.5, Doc 5 §5.1/§5.12/§5.14. El comercio con
> plaza ajena salió a mecánica propia y también está hecho (`Comercio_Fisico_Definicion.md`). Queda solo
> calibración de `MOVIMIENTO.*` / `VISION.*`.

- ✅ El jugador tiene una ubicación física: nace en mundo abierto, funda y entra en el asentamiento, sale, entra en otros; ve el mapa de lo conocido desde cualquier parte pero **nunca** el interior de un asentamiento en el que no esté físicamente; el Gobernador puede prohibir la entrada a neutrales o enemigos — `código: ✔` (2026-09-07). `Jugador.ubicacion` (`asentamiento`|`columna`|`desconectado`), proyección recortada al interior de la plaza que se pisa, `plazasRecordadas`, `politicaDeAcceso` + `vetarJugador`. Migración v8→v9/v10. Diseño en `Consideraciones/Jugador_Situado_Definicion.md`; canon Doc 0/1/2/5.
- ✅ Movimiento libre por el mapa general: más rápido sin tropas (`MOVIMIENTO.velocidadJugador`), consumo de trigo casi nulo, por clic sobre el mapa (`salirAlMundo`/`marcharA`) — `código: ✔` (2026-09-07). La duda de "100% cliente" quedó **resuelta: no** (la posición es T3 por doc 9). Comandos de interacción por anillo: `inspeccionar`/`atacar`/`perseguir`.

## Onboarding y curva inicial
- 🔶 Curva de progresión inicial más gradual: sin decidir qué se desbloquea y cuándo; pospuesto hasta que exista una interfaz de jugador individual (la actual es de GM) — `código: ✘` (`Docs/Mecanicas a desarrollar.md` §29)

## Maravilla y ciclo de servidor
- ✅ **El edificio** está implementado: único, exige nivel de asentamiento máximo (5, antes 3), disponible vía control manual de cola, sin recetas ni niveles — es un trofeo — `código: ✔` (`EDIFICIO_CATALOGO.maravilla`). Coste placeholder con recursos existentes; los materiales **exóticos** del diseño no existen todavía.
- 🔶 **El ciclo**: 12 meses de servidor + cierre al completarla + la Facción ganadora persiste como legado NPC — diseño cerrado, **`código: ✘`** (requiere infraestructura de servidor / multi-instancia; ver `Roadmap_Escalado.md` Eje 4; `Docs/Mecanicas a desarrollar.md` §24)

## Interfaz (herramienta de depuración, no mecánica de juego)
- ✅ Control manual de la cola de construcción (añadir / quitar / reordenar), con ubicación siempre automática, panel y segmento "Info:" — `código: ✔` (motor `engine/construction.ts`; comandos `anadirEdificioManualmente` / `quitarDeCola` / `moverEnCola`)
- ✅ Filtro de Facción por combobox en Asentamientos / Jugadores — `código: ✔`
- ✅ Selector de escuadrones propios por chips en la pestaña Guerra (cascada Facción → Asentamiento → escuadrones) — `código: ✔`
- ✅ Mejora manual de edificio y calibración manual de reserva — `código: ✔` (`mejorarEdificioAhora`, `calibrarReservaManual`)
- ✅ Renombrar asentamiento — `código: ✔`

## Herramientas de laboratorio (no son juego)
- ✅ **NPC de Gobernanza**: Facciones que juegan solas para poder correr batches largos; el flag vive en la capa de aplicación y nunca en `Faccion`; juega después del motor y sus errores de dominio se ignoran — `código: ✔` (`session/npcGobernanza.ts`, comandos `alternarFaccionNpc` / `avanzarFaccionesNpc`; doc `NPC_Gobernanza_Facciones_Controladas.md`)
- ✅ **Simulación de auto-comercio**: NPC virtual que propone trueques dentro de una Facción, apagado por defecto (`activo: 0`), con instrucciones de borrado incluidas — `código: ✔` (`engine/simulacionAutoComercio.ts`)
- ✅ Panel de balance, diarios de simulación batch, métricas y perfiles de trazado — `código: ✔`

## Arquitectura de partida *(no es mecánica de juego, pero condiciona varias)*
- ✅ Backend con `GameSession`, registro de comandos por nombre, identidad, roles de partida y autorización por comando — `código: ✔` (`src/session/`, `src/acceso/`, `src/server/`)
- ✅ Tiempo real en el servidor; proyección por audiencia (`proyectarParaJugador`), que es la base de la niebla de guerra — `código: ✔`
- ✅ Persistencia de partida con snapshots versionados y migraciones; respaldos — `código: ✔`

## Roadmap / escalado
- ✅ Eje de fidelidad visual (Fase 0 → Fase 1 → Fase final); eje naval (terrestre → con mar) — `código: —`
- 🔶 Eje 4: ciclo de servidor de 12 meses + Maravilla + legado NPC — `código: ✘`
- 🔶 Otros ejes (número de jugadores, complejidad de combate) — `código: —`
