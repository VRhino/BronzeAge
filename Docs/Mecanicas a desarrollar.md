# Mecánicas por desarrollar

La **lista única de lo pendiente**. El estado completo de todas las mecánicas del juego —hechas, descartadas y
estas— vive en `typescript-final:Consideraciones/Checklist_Mecanicas.md`, que para lo pendiente solo resume y apunta aquí.
Re-contrastado contra `src/` el 2026-09-09. Lo que solo necesita calibración con números (batch, mediciones) vive en
`Docs/Mecanicas a balancear.md`. Las ideas aparcadas sin plan van a `Docs/Ideas a diseñar.md`.

Cuando una entrada se cierra (diseño **y** implementación), se **borra entera** de este archivo: lo útil que
no esté ya en el canon (`Docs/Game/`) o en una ficha de `Consideraciones/` se mueve allí primero. Mientras una
mecánica se está diseñando, sus acuerdos provisionales pueden vivir aquí como notas.

> **2026-10-13 (reconstrucción):** la guerra logística (antes §44) y el repostar por zona con víveres del ejército (antes §45) están decididos y
> escritos en el canon (`Docs/Game/` 2.4.1, 3.10, 3.13.8, 4.2.2, 5.13, 5.17); los trozos diferidos de caravanas (antes §8.1) pasaron a
> `Docs/Ideas a diseñar.md` §47; la lista de bots emergentes (antes §46) vive en `Docs/Arquitectura/12` §15. Las Eras IV y V quedan
> pospuestas para después, con el motor nuevo.

## Índice

| # | Área | Mecánica | Código hoy |
|---|---|---|---|
| 20 | TECNOLOGÍA | Eras IV y V (pospuestas con el motor nuevo) | Eras I-III en canon |
| 38 | MILITAR | Tope de héroes en asedio: 15 contra 15 (decidido el 2026-10-13) | constante del archivo común |
| 39 | POLÍTICA | Elección del Rey de Facción por voto entre representantes (decidido el 2026-10-17: queda aquí, no implementada) | el Rey se asigna, sin votación |
| — | MUNDO | Índice espacial de `viabilidad` (rendimiento del batch de territorio) | recorre todo el mundo en cada llamada |
| — | PARIDAD | Recortes temporales del motor Rust frente al TypeScript (sección «Paridad con TypeScript») | el motor no está migrado hasta cerrarlos |

**Descartado** (decisión del usuario): los **landmarks** (antes §10, 2026-10-05), la **curva de progresión inicial gradual** (antes §29, 2026-10-06), el **bloqueo de la explotación del bosque por los campamentos de bandidos** y el **catálogo ampliado de políticas** (los dos, antes en los cabos sueltos §32/§35, 2026-10-06), y también, de los cabos sueltos §33/§35 (2026-10-06): la **riqueza acumulada** por comisiones, el **bonus por distancia en el mostrador**, **cortar rutas como guerra económica**, la **cola de prioridad de reclutamiento** y las **armas de asedio** como mecánica aparte.

**Pospuesto explícitamente, fuera de esta lista:** las **Eras IV y V** de tecnología con todo lo que cuelga de ellas (decisión del
usuario, 2026-10-05; ver §20), el **ciclo de servidor de 12 meses + Maravilla + legado NPC** (antiguo §24, 2026-10-06; su texto está en `Docs/Ideas a diseñar.md`), el **Attack Timer** (Doc 5.6, decidido y aplazado a
fase posterior a Fase 0) y el **comercio marítimo / unidades navales** (fuera del alcance de Fase 0 por
diseño). Los ajustes de calibración de mecánicas ya construidas viven en cada ficha de `Consideraciones/` y en
`typescript-final:Consideraciones/Preguntas_Abiertas.md`, no aquí. Lo ya cerrado (diseño + implementación) se retira de este archivo por
completo; su estado queda en el checklist.

## 20. Tecnología por Eras: lo que falta tras las Eras I-III

**Estado: Eras I-III en el canon (Doc 6) y en el código (2026-09-29, vía del desarrollo), `código: ◐`.** Plan y
bitácora: `typescript-final:Consideraciones/Tecnologia_Eras_I-III_Definicion.md`. Falta:

- **Eras IV y V — POSPUESTAS** (decisión del usuario, 2026-10-05): el alcance se queda en las Eras I-III. Catálogo, roster y
  logros siguen en la revisión (`Consideraciones/BA-006_Revision_Tecnologia_Eras.md`, EV-1 a EV-7). La subida 4 → 5 la tiene
  que desbloquear una tecnología de la Era V (D54): hasta entonces el código mantiene el techo provisional en el nivel 4
  (`ASCENSO_ASENTAMIENTO.nivelTechoProvisional`), que es lo esperado mientras la Era V esté fuera.
- **Otras vías** (conquista y comercio, Doc 6.1) y **equipo de asedio**: las vías van a `Docs/Ideas a diseñar.md`; el equipo de asedio (`carpinteria_militar`, `trabajos_asedio`, hoy sin efecto en combate) entra con el sistema militar.
- **Logro de `logistica_campana` (Era IV) — POSPUESTO** con la Era IV: la condición ya existe como consulta pura,
  `caminoCompartidoAbierto` (`engine/redCaminos.ts`); se engancha al catálogo al escribir la Era IV.
- **Equipo de asedio**: `carpinteria_militar` y `trabajos_asedio` se adoptan, pero su equipo no tiene efecto en combate.
- **Clientes**: el panel de tecnología del cliente de administración está hecho (2026-10-02). Faltan, en BronzeAgeClient,
  el mismo panel, y en Conquest las definiciones de escuadra de las 15 tropas nuevas y los modelos de los edificios nuevos (CQ-006).

## 38. Tope de héroes en asedio

Decidido el 2026-10-13: **15 contra 15** en el canon (Doc 5.15.1) y como constante del archivo común (`BATALLA.capacidad.asedio`); la capacidad que abre la instancia en Unity (`BattleSide.capacidadMaxima`) sale de ahí.

## 39. Elección del Rey por voto

Decidido el 2026-10-17: la elección del Rey por voto entre representantes (antes ligada a la Liga por alianza) **no existe hoy** y queda como mecánica pendiente. Hoy el Rey de una Facción lo asigna el anterior (el primero es quien la crea; la sucesión sigue el orden de ingreso). La Liga no tiene Rey ni cargos (Doc 2.2, `Diplomacia_Facciones.md` §3.4 y §5.7).

## Índice espacial de `viabilidad` (2026-10-18)

La consulta `viabilidad` (y `validar_fundacion`) recorre todas las piedras y parcelas del mundo en cada llamada. Con el mundo rescalado (5.601 piedras, 23.877 parcelas) el batch de territorio tarda 287 s (objetivo 120 s): el 99 % son 1,12 millones de llamadas de ~250 µs. Hace falta una rejilla espacial en `crates/territorio` que no cambie los resultados (el informe antes y después debe salir idéntico). Dejado para más adelante por el autor; los bots de la fase 7 lo harían inviable.

## Paridad con TypeScript: recortes temporales del motor Rust (2026-10-18)

**Regla del autor:** los recortes de funcionalidad al implementar un sistema en Rust son **temporales**. El motor **no se considera migrado** hasta que tenga todas las funcionalidades de la versión TypeScript (`typescript-final`, ver su `Consideraciones/Checklist_Mecanicas.md`), adaptadas a los ritmos, eventos y demás del mundo reactivo. Cada vez que se recorta algo se anota aquí; al volver, se borra la fila. **Antes de retirar el TypeScript (fase 9) esta tabla debe estar vacía** y los sistemas aún sin empezar (economía y comercio, militar y combate, política, tecnología y Aedas) implementados.

| Sistema | Recorte temporal hoy | Dónde vuelve |
|---|---|---|
| Territorio | Fundar sin titular ni fundadores; sin protección de un día, nombre de ciudad, experiencia de Facción ni anulación de la derrota; la columna no se disuelve al fundar | Sistema de fundación (Doc 1.3, 1.8) y Facciones |
| Territorio | Caravana de Fundación solo por atajo de administración: faltan comprar en un campamento, lanzar, desarmar, caducar y refundar | Economía y comercio (Doc 3.13) |
| Territorio | Proyección de plazas a las sesiones (ámbito Facción y visión); índice espacial de `viabilidad` | Proyecciones y visión; sección «Índice espacial» |
| Población y construcción | Trazado urbano integrado (2026-10-18) salvo: el batch de población ya usa la ciudad real (2026-10-19) pero no construye Mercados (no se auto-construyen), así que no mide los puestos; la mejora automática de la Granja arranca sin comprobar el hueco (solo la manual; la que no cabe espera con su cuadrilla); el radio de la ciudad cuenta solo los edificios de la cola y sus cifras salen de los valores por defecto del crate | Mercados y puestos en el batch (necesita órdenes manuales o bots); calibrar con bots (fase 7) |
| Población y construcción | Pueblos extractores (fase 3C, 2026-10-19), lo que se recortó: la guarnición de serie de 30 mineros sin dueño (el dato está en `PUEBLOS_EXTRACTORES`; sin tropa ni IA no existe y no se repone en 6 h) y el saqueo de un pueblo (batalla propia, el pueblo protege el 50 %); la tropa Mineros del Centro Urbano y de la tropa prestada | Militar y combate (Doc 5.8, 5.15.1b) |
| Población y construcción | Pueblos y obras de mapa, provisionales: la orden la da la administración o el propietario de una columna de la Facción de la madre (sin Gobernador ni Maestro de Obras presentes) y quien entrega materiales es el propietario de una columna de esa Facción (sin ciudadanos como tales); los materiales salen del carro de la columna y de sus caravanas adjuntas (un solo carro por columna, sin participantes ni reparto por héroe); no hay panel (solo `candidatos_de_pueblo`) ni proyección al jugador, ni memoria de niebla del pueblo ni de su obra pendiente; la obra pendiente no se puede atacar (no hay ataque aún); un pueblo completado no agranda la zona de la madre; la tecnología sigue fija (el hierro no es candidato) | Sistema de héroes y columnas; proyecciones y niebla |
| Población y construcción | Avanzadas (Doc 5.17): atalaya, fuerte, ciudadela, su mejora y reparación como obras de mapa, la visión recortada de las plazas, la guarnición y el almacén | Guerra logística y movimiento |
| Población y construcción | Solvencia del ascenso con acarreo (2026-10-19, C2): cuenta la media de lo ingresado en 24 h en cubos horarios (`BAN8`), por lo que un pueblo recién levantado no pesa del todo hasta tener un día de entregas y la ventana abarca 23-24 h; la división trunca y el divisor es siempre 1.440 min | Refinar al calibrar con bots (fase 7) |
| Comercio | Manufacturados (C4, 2026-10-19): el precio base se deriva de la receta más barata del catálogo y no se venden a los mercenarios (`BIENES_EN_VENTA` solo con `PRECIO_BASE`); el «jugador» de `batch comercio` ya los valora al proponer trueques. Línea saldada por resto (C3): el umbral de una unidad es una constante del código (`trueque::UNIDAD`), no un parámetro | Fase 7 y bots que fabriquen y consuman |
| Población y construcción | Reconstrucción tras saqueo (`danado`) solo en el modelo, sin conquista que la dispare | Militar y combate |
| Población y construcción | Sin tecnologías: no hay requisitos de edificios ni de recetas ni bonus de producción (p. ej. `canteria`); la producción fabricada no alimenta logros | Tecnología y Aedas (Doc 6) |
| Población y construcción | Sin guarnición ni ración de tropas en el trigo, ni reserva de comida por reclutamiento; sin pool de reclutamiento real | Militar y combate |
| Población y construcción | Sin recintos de muralla: nivel de recinto = 0, sin upkeep ni obra de recintos | `Murallas.md` |
| Población y construcción | Cascada de ruinas: la plaza desaparece y cierra su comercio (trueques anulados, caravanas perdidas, devueltas o que dan la vuelta con su carga, órdenes borradas; fases 3B-1 y 3B-2), pero no pasan sus residentes, no se dispersa su campamento de bandidos ni se derrota a la Facción sin plazas | Política y militar |
| Población y construcción | Sin cargos ni residencia: las órdenes las autoriza la administración o el propietario de una columna; las políticas asumen el cargo ocupado; sin Sala del Consejo en los slots; sin `renombrarAsentamiento` | Política (Doc 2.2) |
| Población y construcción | Sin capital designada ni distancia a la capital (mantenimiento ×1 siempre); Gran Fundición sin tope por Facción | Política y Facciones |
| Población y construcción | Fertilidad de zona fija en 1,0 (falta el muestreo de 40 puntos del terreno); sin Aedas (felicidad y nobleza), sin ocupación tras conquista, ciudadanos fijos en 1 | Mundo, tecnología y militar |
| Población y construcción | Experiencia de Facción por edificio completado y por ascenso; cupo de Facción real por nivel de Facción (se asume cupo libre) | Facciones (Doc 2.2.1) |
| Población y construcción | Fines de obra, mejora, política, ascenso y obra de pueblo con precisión de un minuto (sin evento propio); cobertura parcial de hambre y reemplazo de fuentes con regeneración resueltos por simplificación | Refinar al calibrar con bots (fase 7) |
| Contrato | Las órdenes nuevas de economía (códigos 12-20 del núcleo) y de pueblos (45-48: `MarcarPueblo`, `EntregarMateriales`, `DestruirPueblo`, `ReponerCarro`) no están mapeadas a órdenes del contrato Protobuf 05, ni `Pueblo` y `ObraDeMapa` de `entities.proto` salen de la proyección | Contrato 05 y servidor de sesiones |
| Batch | `batch territorio` simula plazas con economía propia: sus cifras de Leñera y tiempo ya no valen; se recorta a las invariantes de territorio | Informe de cada sistema |
| Batch | **Re-medir con `batch comercio` tras C2-C4 (2026-10-19)**: comisión acuñada (precio de los manufacturados), repescas por trueque (peaje y línea saldada por resto) e ingreso real del acarreo en la solvencia del ascenso; y `batch poblacion` por si cambia el ascenso. No se corrió al implementarlos (el autor lo dejó para después) y las cifras citadas en `decisiones/comercio.json` y en `Mecanicas a balancear.md` §43-§45 son todavía las de la corrida previa a los cambios (519 salidas, 155 trueques, 77,8 de oro, acarreo ingresado 2,4 %) | Batch posterior |
| Batch | `batch comercio` (2026-10-19) usa un «jugador» mínimo con andamiaje de administración (oro en el carro de las columnas, residencia y almacén personal del sondeo de cupo, Facciones de nivel 8): sin héroes ni cargos, sin tecnologías (solo pueblos de Cantera), sin guerra (ningún encuentro se resuelve), sin demanda de tropas ni obras que gasten los bienes escasos (el comercio se apaga a los 13 días), sin mostrador entre plazas más allá del sondeo de minerales, sin caravanas adjuntas ni aparcadas, y la conservación exacta solo cubre estaño y hierro (el cobre se funde y las mejoras gastan los lingotes) | Bots de la fase 7; héroes y columnas; tecnología |
| Economía y comercio (3B-2) | Integrado en el núcleo (2026-10-19, estado `BAN6`; hoy `BAN8`) lo estático (3B-1) y el movimiento (3B-2): plan, salida, peajes, llegada, regreso, reparto automático con pulso, red de caminos con poda perezosa, mercado de mercenarios, caravana adjunta/aparcada, hook de batalla y rejilla de encuentros. **Falta**: la poda periódica de trueques (hoy perezosa: al tocarlos) y las órdenes del contrato 05 (el carro de acarreo y los pueblos extractores son de la fase 3C, 2026-10-19) | Contrato 05 |
| Economía y comercio (3B-2) | Andamiaje provisional que queda: `DarRecursos`, `DarSuministro`, `FijarResidencia` y `DarAlmacenPersonal` (administración). El carro de la columna es `Ejercito.suministro` (capacidad: un carro de 500 por columna más las caravanas adjuntas, con «Carga Ampliada»); engancharse y aparcar son órdenes del dueño de la columna con la puerta de 2 u y la Facción comprobadas (la administración salta la puerta). Falta del sistema de héroes y columnas: participantes (un carro por héroe, reparto al separarse), `soltarCaravana` y `reclamarCaravana` en mundo abierto, `cargarCaravana` a una adjunta, `caravanasAlEntrar` (todas del lugar), pérdida de las adjuntas al derrotar a la columna y vuelta a casa de las huérfanas, y que el Líder sea el cargo real | Sistema de héroes y columnas; Movimiento_Ejercitos §9-§11 |
| Economía y comercio (3B-1) | `CancelarOrdenMercado` existe porque lo pidió el autor al encargar la fase, pero `Comercio_Caravanas` §8.2 dice que «nada cierra una orden salvo el mostrador o el plazo»: falta decidir si queda (hoy retira la orden como una expirada, con `mercado.orden_cancelada`). Los hechos `comercio.preparacion_terminada`, `comercio.flota_traspasada`, `reputacion.cambio`, `reputacion.umbral_bajo` y `faccion.experiencia` no están en §12 | Decisión del autor; documentar en §12 |
| Economía y comercio (3B-2) | Pasos de río (`pasosRio`, `radioPasoCiudad`): el grafo del artefacto solo deja cruzar los ríos por sus vados (el generador deja transitable el tramo del vado), así que una ciudad ribereña no abre un paso; `NAVEGACION.radioPasoCiudadMin` dice 60 en el bloque y 10 en Movimiento §4.3, sin usarse. Las órdenes de mercado de una plaza conquistada se quedan; el aviso de experiencia de Facción por trueque cumplido no lleva cifra (no está en ningún bloque); el umbral de reputación solo es el bajo (−40) | Generador de mundo (vados y pasos por ciudad); Facciones (Doc 2.2.1) |
| Economía y comercio (3B-2) | Falta de `tramosDeRed` (pintar caminos con niebla) y de la proyección de caminos al jugador; las plazas aún no son observadoras de la visión (solo las columnas ven; una caravana grande llama la atención a 60 unidades, el valor vigente de `VISION.caravanaGrande.radio`, no las 300 del bloque); solo la que va en tránsito llama la atención (la que vuelve, no) | Proyecciones y niebla (Niebla_De_Guerra §5) |
| Economía y comercio (3A) | Escolta cedida solo como estructuras (`Escuadra`, `CandidataEscolta`, cupo de Liderazgo): sin héroes ni tropa reales, sin contenedor `escolta` ni vuelta al campamento; la escolta de la flota traspasada por una conquista (escuadras de héroes desalojados) queda sin decidir | Héroe y tropa; militar y combate |
| Economía y comercio (3A) | Combate de caravanas (intercepción, tipos A/B, defensa base, captura, bandidos, escolta por ejército): el núcleo ya deja el hecho `caravana.encuentro` cuando el plan de una caravana cruza el de otra caravana o una columna (rejilla de encuentros), y expone `caravana_entra_en_batalla`, `caravana_sale_de_batalla` y `aplicar_perdida_de_carros` (reparto de la carga por capacidad de cada carro: criterio del implementador, por confirmar; el plan en pausa no se reescala si cambia la velocidad al perder animales), pero la **resolución** del encuentro es del sistema militar | Militar y combate (Doc 3.10, `Combate.md` §7.1) |
| Economía y comercio (3A) | La experiencia de Facción por trueque cumplido devuelve las Facciones que la ganan, pero la cifra `NIVEL_FACCION.xp.truequeCumplido` no está en ningún bloque de Mechanics (el bloque `NIVEL_FACCION` de `Combate.md` está vacío) | Facciones (Doc 2.2.1): escribir el bloque |
| Economía y comercio (3A) | Reputación modelada como ritmo continuo: con alianzas cuyo bonus no supera al decaimiento queda en 0 (el TypeScript dejaba un resto ≈ alianzas × 0,0005); sin los sucesos de diplomacia conectados (alianzas activas, vasallos, guerra tras romper alianza) | Política y Facciones (Doc 2.7) |
| Economía y comercio (3B-1) | Autorización por cargo, residencia, presencia en la puerta y Líder de la columna: hoy manda la administración o el propietario de una columna de la Facción de la plaza (`manda_en`) y el Líder es el propietario de la columna; sin hecho `caravana` de las épicas de Aedas, sin contador de logros `animales.comprados`, sin proyección del jugador de acuerdos, órdenes y caminos (§8.4). El mercado de mercenarios usa la columna como héroe: residencia, almacén personal y cupo diario por columna provisionales (`FijarResidencia`, `DarAlmacenPersonal`), víveres hasta la capacidad de la columna y sin tropa prestada ni ración gratis del campamento | Política (Doc 2.2); tecnología y Aedas (Doc 6); héroes y residentes; proyecciones y visión |
| Economía y comercio (3C) | Carro de acarreo (2026-10-19): sin escolta de residentes de la madre (el cupo 100/200/300 existe en `comercio::flota` pero no hay residentes ni héroes), la reposición de un carro destruido es una orden (`ReponerCarro`) y no automática, la ruta pueblo ↔ madre se traza en cada viaje (A\* memorizado) en vez de guardarse «una sola vez», no registra su ruta en la red de caminos (la red es de rutas entre plazas), y un pueblo sin ruta por tierra a su madre no saca el carro; la resolución del combate del carro (casi siempre B) es del sistema militar | Militar y combate; Guerra logística y movimiento |
| Economía y comercio (3A) | Fusión o anexión de Facciones a mitad de trueque sin decidir | Política |
