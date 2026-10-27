# Mecánicas por desarrollar

La **lista única de lo pendiente**. El estado completo de todas las mecánicas del juego —hechas, descartadas y
estas— vive en `typescript-final:Consideraciones/Checklist_Mecanicas.md`, que para lo pendiente solo resume y apunta aquí.
Lo que solo necesita calibración con números (batch, mediciones) vive en `Docs/Mecanicas a balancear.md`. Las ideas
aparcadas sin plan van a `Docs/Ideas a diseñar.md`.

Cuando una entrada se cierra (diseño **y** implementación), se **borra entera** de este archivo: lo útil que
no esté ya en el canon (`Docs/Game/`) o en una ficha de `Consideraciones/` se mueve allí primero. Mientras una
mecánica se está diseñando, sus acuerdos provisionales pueden vivir aquí como notas.

> **Revisado al cerrar la fase 6 (2026-10-26).** Se borró lo cerrado por los sistemas 1-6 y por las decisiones del autor
> (`Consideraciones/Calibracion_Sistema4_Decisiones.md`, `Consideraciones/Dudas_Del_Codigo_Decisiones.md` y
> `Consideraciones/Politica_Tecnologia_Decisiones.md`); quedan los recortes de paridad abiertos, las dudas sin decidir (cerradas el 2026-10-27) y,
> reunido en un solo sitio, todo lo que se dejó **para la fase 7**. La prueba `crates/nucleo/tests/catalogo.rs` exige que
> los códigos de evento del catálogo que el núcleo aún no emite tengan aquí su fila (hoy, los dos de los yacimientos).

## Índice

| # | Área | Mecánica | Código hoy |
|---|---|---|---|
| 20 | TECNOLOGÍA | Eras IV y V (pospuestas con el motor nuevo) | Eras I-III en canon y en Rust |
| 39 | POLÍTICA | Elección del Rey de Facción por voto entre representantes (decidido el 2026-10-17: queda aquí, no implementada) | el Rey se asigna, sin votación |
| 48 | MILITAR / POLÍTICA | Mercenarios por contrato (diseño cerrado el 2026-10-26; `Docs/Disenos_Por_Implementar/Mercenarios_Por_Contrato.md`) | nada |
| 50 | ECONOMÍA | Aldea agrícola, corrales solo en ella y nuevo gate del nivel 2 (`Docs/Disenos_Por_Implementar/Aldea_Agricola_Y_Cria.md` §1) | Corral y Granja dentro de la ciudad |
| 51 | ECONOMÍA | Cría ampliada: sin nodos de livestock, rebaños en los corrales de las aldeas (`Aldea_Agricola_Y_Cria.md` §2; necesita 50) | livestock como nodos libres |
| — | PARIDAD | Recortes temporales del motor Rust frente al TypeScript (sección «Paridad con TypeScript») | el motor no está migrado hasta cerrarlos |
| — | DUDAS | Ninguna sin decisión desde el 2026-10-27 (sección «Dudas abiertas»: las `// DUDA:` que quedan en el código) | — |
| — | FASE 7 | Todo lo que se dejó para los bots reales y la fase 7 (sección «Para la fase 7») | — |

**Descartado** (decisión del usuario): los **landmarks** (antes §10, 2026-10-05), la **curva de progresión inicial gradual** (antes §29, 2026-10-06), el **bloqueo de la explotación del bosque por los campamentos de bandidos** y el **catálogo ampliado de políticas** (los dos, antes en los cabos sueltos §32/§35, 2026-10-06), y también, de los cabos sueltos §33/§35 (2026-10-06): la **riqueza acumulada** por comisiones, el **bonus por distancia en el mostrador**, **cortar rutas como guerra económica**, la **cola de prioridad de reclutamiento** y las **armas de asedio** como mecánica aparte (y, el 2026-10-19, todo efecto del equipo de asedio en el combate con números).

**Pospuesto explícitamente, fuera de esta lista:** las **Eras IV y V** de tecnología con todo lo que cuelga de ellas (decisión del
usuario, 2026-10-05; ver §20), el **ciclo de servidor de 12 meses + Maravilla + legado NPC** (antiguo §24, 2026-10-06; su texto está en `Docs/Ideas a diseñar.md`), el **Attack Timer** (Doc 5.6, decidido y aplazado a
fase posterior a Fase 0) y el **comercio marítimo / unidades navales** (fuera del alcance de Fase 0 por
diseño).

## 20. Tecnología por Eras: lo que falta tras las Eras I-III

**Estado: Eras I-III en el canon (Doc 6) y en el núcleo Rust (sistema 6, 2026-10-25/26).** Falta:

- **Eras IV y V — POSPUESTAS** (decisión del usuario, 2026-10-05): el alcance se queda en las Eras I-III. Catálogo, roster y
  logros siguen en la revisión (`Consideraciones/BA-006_Revision_Tecnologia_Eras.md`, EV-1 a EV-7). La subida 4 → 5 la tiene
  que desbloquear una tecnología de la Era V (D54): hasta entonces el código mantiene el techo provisional en el nivel 4
  (`ASCENSO_ASENTAMIENTO.nivelTechoProvisional`), que es lo esperado mientras la Era V esté fuera. El núcleo no conoce el número de Eras: la III es la última (sin cita de plazo) mientras los bloques de la IV y la V no existan.
- **Logro de `logistica_campana` (Era IV) — POSPUESTO** con la Era IV: la condición ya existe como consulta pura en el TypeScript,
  `caminoCompartidoAbierto` (`engine/redCaminos.ts`); se engancha al catálogo al escribir la Era IV.
- **Otras vías** (conquista y comercio, Doc 6.1): descartadas el 2026-10-11 (van a `Docs/Ideas a diseñar.md`). El equipo de asedio (`carpinteria_militar`, `trabajos_asedio`) se adopta pero no tiene efecto en el combate con números (decidido el 2026-10-19); en Unity, si llega, lo decide la partida.
- **Clientes**: en BronzeAgeClient, el panel de tecnología; en Conquest, las definiciones de escuadra de las 15 tropas nuevas y los modelos de los edificios nuevos (CQ-006).

## 39. Elección del Rey por voto

Decidido el 2026-10-17: la elección del Rey por voto entre representantes (antes ligada a la Liga por alianza) **no existe hoy** y queda como mecánica pendiente. Hoy el Rey de una Facción lo asigna el anterior (el primero es quien la crea; la sucesión sigue el orden de ingreso). La Liga no tiene Rey ni cargos (Doc 2.2, `Diplomacia_Facciones.md` §3.4 y §5.7).

## 48-51. Mercenarios, misiones, aldea agrícola y cría (diseño cerrado el 2026-10-26)

Cuatro ideas del autor, estudiadas y decididas en un hilo de solo diseño; sin código ni tandas todavía (las tandas se crean cuando el autor lo pida). Cada una tiene su documento de `Docs/Disenos_Por_Implementar/` con lo decidido, lo propuesto y lo que toca; este apunte es el pendiente:

- **48. Mercenarios por contrato**: `Docs/Disenos_Por_Implementar/Mercenarios_Por_Contrato.md`. Bots contratados como miembros de la Facción por Rey, Embajador o Gobernador (este con alcance de su plaza), temporales con trabajo de la tabla y permanentes como ciudadanos con marca. Pendiente: implementar tras los bots de la fase 7 (necesita el runner jugando por el contrato 05), calibrar tarifas, plazos y topes, y la pestaña de control en BronzeAgeClient.
- **50. Aldea agrícola**: `Docs/Disenos_Por_Implementar/Aldea_Agricola_Y_Cria.md` §1. Pendiente: pueblo sin nodo y por tipo, cupo 1/3/4/5/6, gate del nivel 2, Corral fuera de la ciudad y calibración del coste en `batch poblacion`.
- **51. Cría ampliada**: `Aldea_Agricola_Y_Cria.md` §2. Pendiente: rebaño, tabla de especies, mundo sin nodos de livestock; depende de 50 (las misiones, 49, ya están).
- **Fuera de alcance**: carne y leche (también del MVP) y las misiones entre humanos van a `Docs/Ideas a diseñar.md` (entradas 52 y 53 de ese archivo).

## Paridad con TypeScript: recortes temporales del motor Rust (2026-10-18)

**Regla del autor:** los recortes de funcionalidad al implementar un sistema en Rust son **temporales**. El motor **no se considera migrado** hasta que tenga todas las funcionalidades de la versión TypeScript (`typescript-final`, ver su `Consideraciones/Checklist_Mecanicas.md`), adaptadas a los ritmos, eventos y demás del mundo reactivo. Cada vez que se recorta algo se anota aquí; al volver, se borra la fila. **Antes de retirar el TypeScript (fase 9) esta tabla debe estar vacía.** Los seis sistemas del canon están implementados desde el 2026-10-26 (fase 6); lo que sigue es lo que les falta.

| Sistema | Recorte temporal hoy | Dónde vuelve |
|---|---|---|
| Territorio y fundación | `Fundar` no deshace la columna dentro de la plaza nueva (Fundacion §5.5-6; duda 9 de 5G-b, técnica: cambiaría cientos de pruebas que siguen usando la columna tras fundar). De la Caravana de Fundación queda que, adjunta a una columna con héroes y con servidores de batalla, entre en la batalla de acoso de Unity (hoy siempre se resuelve con números; P2 hizo la persecución, el acoso de bandidos y la velocidad de la columna) | Fundación (Doc 1.3, 1.8), Movimiento_Ejercitos §9, Combate §7.1 |
| Población y construcción | El batch de población usa la ciudad real pero no construye Mercados (no se auto-construyen), así que no mide los puestos | Bots (fase 7) |
| Población y construcción | Pueblos y obras de mapa: los materiales salen del carro común de la columna y de sus caravanas adjuntas (el héroe ciudadano que entrega da lo de toda la columna, no lo suyo: no hay carga por héroe); no hay panel (solo `candidatos_de_pueblo`); la obra pendiente no se puede atacar; un pueblo completado no agranda la zona de la madre | Héroes y columnas |
| Población y construcción | Avanzadas, obras de mapa: sin panel (solo las consultas `avanzada`, `avanzadas_de`, `posicion_de_avanzada`); no hay orden para sacar cosas del almacén (solo repostar y el saqueo); costes, duraciones y cupo de guarnición son los placeholders del canon; sin planta de avanzada (su diámetro de `TAMANOS_MAPA`, 0,2 por ahora, solo evita que se pisen elementos; la ciudad de la madre se toma como una décima del radio de su zona, Calibración §4.2, y no hay una cifra de canon para ella) | Calibración con bots |
| Población y construcción | Solvencia del ascenso con acarreo (C2): cuenta la media de lo ingresado en 24 h en cubos horarios (`BAN8`), por lo que un pueblo recién levantado no pesa del todo hasta tener un día de entregas y la ventana abarca 23-24 h; la división trunca y el divisor es siempre 1.440 min | Refinar con bots (fase 7) |
| Comercio | Manufacturados (C4): el precio base se deriva de la receta más barata del catálogo y no se venden a los mercenarios (`BIENES_EN_VENTA` solo con `PRECIO_BASE`) | Fase 7 y bots que fabriquen y consuman |
| Población y construcción | Murallas: falta medir la piedra (nivel 3) y la reparación tras una conquista (plan en `Docs/Arquitectura/Murallas_Contraste_y_Plan.md`); las cifras de la empalizada se mantienen (decisión M2) hasta medir con bots | Bots (fase 7) |
| Población y construcción | Murallas, tercera tanda: las pruebas de `tests/urbanismo.rs` siguen usando muros inventados para bloquear sitio a propósito (la colocación con recintos reales se prueba en `tests/murallas_asedio.rs`) | `Murallas.md` |
| Militar y combate (4A-1) | Héroe y escuadra, solo lo que las reglas interpretan: sin clase, género, avatar, inventario, equipamiento ni catálogo de perks (solo la lista de ids desbloqueados), sin habilidades ni formaciones de la escuadra, el nivel del héroe no sube (CQ-001) y el botín de Unity no tiene dónde caer (el resultado suma su experiencia y sus monedas; los objetos se descartan) | Conquest (CQ-001 y CQ-004); el contrato lo proyecta con valores por defecto |
| Militar y combate (4C) | Encuentros: no hay estado «a la espera de combate» ni se recuerda que un par estaba cerca. Un par que ya estaba junto cuando cambia una condición (un herido que se cura, una alianza que se rompe) no deja encuentro nuevo hasta que se alejen y vuelvan. Radio fijo de 3 (`radioEncuentro`). Los de mercenarios siguen fuera de la rejilla | Refinar con bots |
| Militar y combate (4B-1) | `crearHeroe` sin clase, género ni avatar (la proyección los lleva por defecto, decisión 4 del autor; el nombre sí se guarda desde el 2026-10-27); los hechos de héroe llevan el héroe en `datos` | Conquest (CQ-001) |
| Militar y combate (4B-1) | La reubicación por ruina y el desalojo por conquista solo alcanzan a residentes y presentes; los cargos que tenían en la plaza que dejan se vacían y nadie los reasigna solo; la residencia de héroes no tiene cupo de viviendas que el traslado pueda mirar | Política |
| Militar y combate (4E) | Unirse a una batalla: la persecución entre columnas personales no se prueba contra un mundo real (solo el `Contexto`) | Pruebas |
| Militar y combate (4E) | Candado de batalla: la administración no lo lleva (sus órdenes son andamiaje de escenarios); `Comando::implicados` mira los héroes, columnas, caravanas, plazas, pueblos y campamentos de bandidos que nombra cada orden, no el `actor` salvo por su héroe de cuenta (`heroe_de_cuenta`); no hay `sinCandadosVencidos` (el evento de prioridad 1 hace innecesario soltar a mano el candado de una vencida) | Revisar con la autorización por cargos (5E) |
| Militar y combate (4E) | Batalla en curso y el resto del mundo: la columna apartada que pasa hambre sigue pasándola; el campamento de bandidos atacado sigue acosando a las caravanas que pasan a ≤ 7 u. mientras dura su batalla; la columna retoma su plan pero **no** la persecución que llevaba; una plaza asediada que cae en ruinas durante la batalla no la cancela (el resultado ya no conquista nada) | Refinar con bots |
| Población y construcción | Obra de pueblo con precisión de un minuto (sin evento propio: la termina el paso de minuto de su plaza; los pueblos siguen por minuto por decisión del 2026-10-22); cobertura parcial de hambre y reemplazo de fuentes con regeneración resueltos por simplificación | Refinar con bots (fase 7) |
| Población y construcción | Mejora que espera hueco (la Granja sin sitio): conserva su cuadrilla y se reintenta en cada paso de minuto, no en el instante en que se libera un hueco | Refinar si en las pruebas con bots pasa a menudo |
| Batch | `batch territorio` simula plazas con economía propia: sus cifras de Leñera y tiempo ya no valen; se recorta a las invariantes de territorio | Informe de cada sistema |
| Batch | `batch comercio` usa un «jugador» mínimo con andamiaje de administración (oro en el carro de las columnas, residencia y almacén personal del sondeo de cupo, Facciones de nivel 8): un héroe visitante sin tropa por plaza, sin guerra (ningún encuentro se resuelve), sin demanda de tropas ni obras que gasten los bienes escasos (el comercio se apaga a los 9 días), sin mostrador entre plazas más allá del sondeo de minerales, sin caravanas adjuntas ni aparcadas, solo pueblos de Cantera, y la conservación exacta solo cubre estaño y hierro. Las tecnologías llegan por la vía de juego desde 6H (el Rey pasa la corona al Gobernador de oficio de la capital) | Bots de la fase 7 |
| Economía y comercio (3B-2) | Andamiaje provisional que queda: `DarRecursos` y `DarSuministro` (administración: montan escenarios sin salir de un lugar). Engancharse, aparcar y soltar caravanas son órdenes del Líder de la columna (desde el 2026-10-27), con la puerta de 2 u y la Facción comprobadas (la administración salta la puerta) | Héroes y columnas; Movimiento_Ejercitos §9-§11 |
| Economía y comercio (3B-2) | Las órdenes de mercado de una plaza conquistada se quedan; el aviso de experiencia de Facción por trueque cumplido no lleva cifra (no está en ningún bloque); el umbral de reputación solo es el bajo (−40). (Los pasos de río se cerraron el 2026-10-27, D7: solo los vados del mapa, Comercio_Caravanas) | Facciones (Doc 2.2.1) |
| Economía y comercio (3A) | Escolta cedida: la escolta destruida de una caravana pierde sus soldados; una escolta sin soldados cuenta como sin escolta. Falta decidir la escolta de la flota traspasada por una conquista (hoy vuelve a sus héroes; `// DUDA:` en `comercio/ruina.rs`) | Militar y combate |
| Militar y combate (4B-2) | Un evento invalidado (`Desercion`, `Caduca*`, `Umbral`) permanece en la cola hasta que pasa su instante y entonces se descarta; no hay borrado activo | Medir con 500 bots (fase 7); si pesa, purga activa o cola con borrado |
| Militar y combate (Entrar con todo) | Con servidores de batalla **toda batalla va a Unity, también bot contra bot**; sin servidores, todo con números. Falta del lado de Unity que el servidor de batalla las tome y juegue, y que un bot cuya batalla no asigne nadie reintente (falla por plazo sin castigo) | Unity (CQ), bots (fase 7) |
| Militar y combate (Entrar con todo) | La batalla de caravana contra bandidos (con algún héroe sano y servidores de batalla): falta del lado de Unity la IA que maneja a los bandidos (el prefijo de sus hechos, `batalla.`, lo decidió el autor el 2026-10-27, D11) | Unity (CQ) |
| Batch militar (4G) | **Lo que `batch militar` no ejercita** (bots mínimos, `crates/batch/src/simmil.rs`): las batallas de Unity (corre sin servidores de batalla), la desconexión y la vuelta al mundo, el préstamo y el recluta de campamento, entrar en pueblos y avanzadas para defenderlos, la guarnición de jugadores de una avanzada, mejorar, reparar o cancelar avanzadas, mejorar recintos, el vasallaje, la intel de la taberna, `UnirseEnCampo` con `Preguntar`, `CederLiderazgo`, `SepararseDelEjercito` y las formaciones en campo. La ciudadanía, la residencia, las alianzas y el nivel de Facción son andamiaje de administración (P9) | Fase 7 (bots) |
| Batch político (5I) | **Lo que `batch politica` no ejercita**: la Gran Fundición (P3), el tributo que no cabe en la capital, las batallas de Unity, la elección de Rey por voto y los gremios (no existen), el veto a un residente, retirar propuestas (`RetirarRelacion`, `RetirarAnexion`, `RetirarFusion`), `RenombrarAsentamiento`, `ResidirEnCampamento`, `DejarResidencia`, `LanzarCaravanaFundacion` y `DesarmarCaravanaFundacion`. **Andamiaje que queda:** el nivel de partida de las Facciones (`FijarNivelFaccion` a 3), la colonia de cada plaza (`crear_ejercito` + `DarCaravanaFundacion`) y colocar a cada héroe dentro de su plaza al nacer (`ColocarHeroe`). **Pruebas del núcleo** con el andamiaje de 5E (`tests/comun::mandaba`, `como_dueno`; `BRONZEAGE_SIN_ANDAMIAJE=1` lo apaga y mide): 179 de 539 pruebas en 20 binarios; la puerta de antes abierta a los neutrales (`comun::acceso`) en 7 binarios (42 usos) | Fase 7 (bots) |
| Batch tecnológico (6H) | **Lo que `batch tecnologia` no ejercita**: las batallas de Unity, una épica cumplida (T1), el estaño y el bronce (T2), las Eras por todos sus logros y la III (42 días), la Gran Fundición y la Maravilla, comprar en una plaza ajena y el título «Mecenas de los Aedas» con épicas cumplidas. **Andamiaje que queda:** el del político, y el oro de los buscadores de alijos al nacer (`DarAlmacenPersonal`, declarado: el de los alijos no paga un recluta, T3) | Fase 7 (bots) |
| Contrato (4H) | Lo que depende de Conquest o de mecánicas que no existen: del héroe, clase, género, avatar, inventario y equipo (`crear_heroe` los ignora; el nombre sí se guarda desde el 2026-10-27; el `Heroe` y el `HeroSnapshot` del ticket van con valores por defecto, decisión 4; CQ-001, CQ-004); `nombresDeCompaneros`/`nombresDeDirigentes`, el historial del héroe y `preciosReferencia`. Las órdenes sin héroe usan el de la cuenta: un jugador con varios héroes no existe aún | Conquest (CQ-001, CQ-004), columnas |
| Contrato (4H) | **Recorte de la proyección que queda** (la paridad del 2026-10-27 cerró la escuadra en una avanzada, el almacén de la avanzada, los héroes de campo en el `HEROE` de su cuenta, la memoria de la niebla, las zonas y la red en `ADMIN` y la retirada del informe de inspección caducado): el almacén de un pueblo solo se refresca por minuto en `ASENTAMIENTO`; en `FACCION` llega con el último cambio discreto del pueblo (refrescarlo por minuto pediría recordar por sesión cada pueblo, como la plaza). Los pueblos van completos en cada delta del interior. No es un hueco frente al TypeScript, que no tenía almacén de pueblo | Proyecciones, cliente |
| Aedas itinerantes (6D) | (1) el plan del viaje se fija al salir, con la red de caminos de ese instante: un camino que se abre a mitad del viaje no lo acelera (el TS avanzaba cada tick con las aristas del momento); (2) sin `pasosRio` en las posiciones de las plazas: el A* es el de las caravanas, con los vados del grafo. Decisión del autor (2026-10-25, duda 4 de 6D): dejarlo hasta medir con bots | Fase 7 |

## Dudas abiertas

**Ninguna sin decisión** (2026-10-27): el autor cerró las de 4H, T7, 5H, 6A, la crónica y la Granja inicial. Dos cambiaron (`movilizarEjercito` guarda el interior recordado; el héroe público lleva `faccion_id`) y el resto quedó como está, escrito como regla en `Proyecciones_Y_Autorizacion.md` y `Tecnologia_Aedas.md` («Decisiones del autor del 2026-10-27») y anotado en `Consideraciones/Dudas_Del_Codigo_Decisiones.md`.

**`// DUDA:` que siguen en el código** (`git grep -n "DUDA" -- '*.rs'`, 23 marcas al 2026-10-27, tras cerrar las dudas abiertas y la paridad de la proyección). Ninguna espera decisión: marcan lo que depende de Conquest (valores por defecto del héroe, `proyeccion/batalla.rs`, `proyeccion/propios.rs`, `sesiones/ordenes.rs`), costes que se miden con bots (`nucleo/pertenencias.rs`, `distribuir` en `sesiones/ambitos.rs`) y detalles técnicos anotados: `combate/src/resolucion.rs` (el id 0 como héroe «sin dueño»), `comercio/src/red_caminos.rs` (dos puntos que redondean al mismo entero no son tramo), `comercio/src/ruina.rs` (escolta de la flota traspasada; caravana `EnBatalla`), `proyeccion.rs` (la obra de mapa antes de ser `Pueblo` o `Avanzada`), `proyeccion/ambito.rs` (id de la `VISTA`), `proyeccion/interior.rs` (el trazado en milésimas de unidad local), `proyeccion/propios.rs` (pueblos en `FACCION`; la obra de un pueblo en marcha; lo que `ADMIN` aún no lleva) y `sesiones/ambitos.rs` (lo que una entidad enseña de otra; el héroe de una cuenta con solo héroes de campo; los pueblos completos en cada delta).

## Para la fase 7

Todo lo que se ha ido dejando «para la fase 7» (bots en Rust y admin web, `Consideraciones/Reconstruccion_Definicion.md` §4) en las decisiones del autor y en la documentación, en un solo sitio. **La fase 7 no empieza sin una orden directa del autor.** Cada entrada lleva su origen.

**Calibración con bots reales** (las reglas están hechas; faltan cifras medidas con jugadores que juegan de verdad):

| Qué | Por qué espera a la fase 7 | Origen |
|---|---|---|
| Saqueo de pueblos extractores (el atacante gana siempre: 2.144 saqueos en 30 días) | Defenderlos es parte del juego; se vuelve a mirar con bots | M1, `Calibracion_Sistema4_Decisiones.md`; `Combate.md` §14 |
| Equipo de las tropas de Barracón y Galería de tiro (ninguna escuadra en 30 días) | El bot de hoy no encarga armas a la Armería | M7, `Calibracion_Sistema4_Decisiones.md`; `militar.json` |
| Reclutar gasta la población que pide el ascenso | Es la regla (decisión del jugador); los bots tienen que tenerlo en cuenta | M9; `Heroe_Y_Tropa.md` §14; `militar.json` |
| Nivel 3 en el batch de población (ninguna plaza llega) | Pide Mercados, Barracón y Galería a mano y cupo de Facción | C1, `Calibracion_Sistema4_Decisiones.md`; `poblacion.json` |
| Cifras de la empalizada (madera, piedra, puertas, minutos por celda, upkeep) | Se mantienen hasta medir con bots que compitan por la madera y con asedios; la empalizada temprana (M1) ya se arregló con `LAYOUT_VERSION` 5; falta medir la piedra (nivel 3) y la reparación tras una conquista | Murallas M2, `poblacion.json`; balancear «Murallas»; fila «Murallas» de Paridad |
| Ganado (Corral en el 12-13 % de las plazas) | Se deja y se mide con comercio | P5 del batch de población, `poblacion.json` |
| Reputación concentrada (una Facción a 99,7) | Esperar a bots que incumplan trueques | C1b, `comercio.json`; balancear §32 |
| Salto de la reputación mediana con la economía entera (5,45 → 34,1) | Se investiga con C1b | EE-7, `Calibracion_Sistema4_Decisiones.md`; balancear §32 |
| Reputación que no sube en el batch político (mediana 0) | Con EE-7 y C1b | P6, `politica.json`; `Politica_Tecnologia_Decisiones.md`; balancear §32 |
| Comisión acuñada contra el oro lleno (0,44 % del valor) | Se deja (se pierde); medir con bots con sumideros de oro (taberna, tropas, obras) | C7, `comercio.json`; `Calibracion_Sistema4_Decisiones.md` |
| Sumideros de bienes escasos (el comercio se apaga a los 9-13 días) | Bots con demanda de tropas y obras | C5 (2026-10-19), `Calibracion_Sistema4_Decisiones.md` |
| Ventana del acarreo en la solvencia del ascenso (cuenta el 2,2 % del nominal) | Se medirá con un ascenso al nivel 3 | C2b, `comercio.json`; `Calibracion_Sistema4_Decisiones.md`; fila «Solvencia» de Paridad |
| Peajes de `batch comercio` (22 % de la carga con el cupo nuevo de nivel 2; 28 % con la cantería desde 6H) | Se deja para la fase 7 | Calibración 5I (2026-10-25), `Politica_Tecnologia_Decisiones.md` |
| Gran Fundición inalcanzable en 30 días | Batches de más semanas | P3, `politica.json`; balancear §35b |
| Refundación en solitario lenta (más de un día para el fondo) | Es cosa de los bots | P8, `politica.json` |
| Qué recurso sale cuando la carga no cabe | Abierta para la fase 7 | Cal-8, `Calibracion_Sistema4_Decisiones.md` |
| Calibración de la reputación, cifras de mercenarios (`MERCENARIOS`) y caravanas del Mercado | Dejadas como están hasta medir con bots | `Docs/Arquitectura/Handoff_Fase6.md`; balancear §32, §40 |
| Cobertura de metales por zonas (gradiente 5 / 25 / 70 % de estaño y oro) y Facciones sin plaza (26 %) | El batch nunca se aleja más de 120 u del campamento ni busca sitio más lejos | `Mecanicas a balancear.md` (territorio) |
| **Tecnología, Q3:** las X de los logros (las del TS; las de las Eras II y III eran placeholder), la velocidad (20 u/min) y la estancia (360 min) de los Aedas itinerantes | Se portaron del TypeScript sin medir. El batch 6H dejó, con las X del TS (60 plazas, 42 días, semillas 42 / 7), la Era II **por plazo el día 35**; Era I: carros de guerra 3,1 / 4,7, arquería palacial 4,6 / 5,6, armamento palacial 5,5 / 5,5, metalurgia del cobre 15,6 / 22,2, cantería 18,7 / 20,2, cría caballar 24,6 / —, escudos ligeros 26,0 / —, aleación del bronce — (sin estaño); de las Eras II y III, casi ninguno | Q3, `Sistema6_Contraste_y_Plan.md` («Avance de 6H»); `tecnologia.json`; balancear §20 y §42 |
| **T1** Ninguna épica se cumple (el hito llega antes que la épica) | La épica es la vía de quien no puede cumplir el hito; lo medirán los bots | T1, `tecnologia.json` (decisión del 2026-10-26) |
| **T2** No se extrae estaño en este mundo con estos bots (sin aleación del bronce ni bronce) | Medir con bots que comercian el estaño o fundan cerca de él, con las X | T2, `tecnologia.json` (2026-10-26) |
| **T3** Los alijos dan ≈ 4 u de oro y un recluta de campamento cuesta 300 u | Medir con la refundación y los héroes sin plaza de verdad | T3, `tecnologia.json` (2026-10-26) |
| El viaje de los Aedas itinerantes con la red de caminos del momento y los pasos de río | Dejarlo hasta medir con bots | Duda 4 de 6D; fila «Aedas itinerantes (6D)» de Paridad |

**Rendimiento con 500 bots:** la cola con eventos invalidados que no se borran (fila de Paridad «4B-2»), la vista que se reconcilia en cada reparto (≈ 15 ms por orden con 450 vistas: un techo de ≈ 65 órdenes/s; T6 de 4H) y el coste por orden de la forma pública de las Facciones (5H, duda 3). Medidas de referencia en `Docs/Arquitectura/Handoff_Fase6.md`.

**Bots y lo que los batches no ejercitan:** las filas «Batch militar (4G)», «Batch político (5I)», «Batch tecnológico (6H)» y `batch comercio` de Paridad; los batches militar y de comercio siguen con el andamiaje de ciudadanía y residencia para seguir comparables (P9); el bot cuya batalla de Unity no asigna nadie tiene que reintentar (fila «Entrar con todo» de Paridad); los perfiles de bots que usan las órdenes del sistema 4 (`Docs/Arquitectura/Sistema4_Contraste_y_Plan.md`); comprometer el recinto como el TypeScript (`bots/cerebro/gobierno.ts`, `Murallas_Contraste_y_Plan.md`); la calibración de los cerebros de bot (ritmo, prudencias, margen y «sin plaza»; balancear §36); la lista de bots emergentes y sus reglas (`Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md` §15-§16, con la rotación de guarnición del NPC y la política «Presión Fiscal», `Consideraciones/Fase1_Verificacion_B.md` Y5); el batch de territorio (D2 y D3, sin decidir).

**Telemetría, auditoría e informe de carga del servidor (pendiente; anotado el 2026-10-26 por orden del autor, que pidió documentarlo y no acometerlo todavía).** El servicio Rust (`bronzeage-servidor`, `crates/servicio`) no expone nada de esto: solo `GET /salud`, y el núcleo cuenta en memoria `Metricas` (eventos, órdenes aceptadas, rechazadas, sin cambios y duplicadas, rutas calculadas) sin servirlas. El TypeScript (`typescript-final`) tenía tres piezas, que el admin web (7H) necesita para las pruebas de carga del panel:

| Pieza del TS | Qué hacía | Lo que haría falta en Rust |
|---|---|---|
| `src/server/telemetria.ts` | Muestreo cada 10 s (apagado por defecto; `TELEMETRIA_INTERVALO_MS` o `PUT /admin/partidas/:gameId/playtest`): retraso del bucle, memoria, coste del tick y por canal de difusión (bytes, fotos, sockets lentos) en `<partida>.carga.jsonl`, con p50/p95/máx por ventana; no anotaba la partida ociosa | Muestras del actor del núcleo, del reparto de `sesiones` y del proceso, en un jsonl de la carpeta de datos; servidas como `GET /v1/telemetria` (y encenderla por petición) |
| `src/server/auditoria.ts` | Quién pidió qué, desde qué sesión y con qué resultado, también los rechazos (la señal de moderación), en `<partida>.auditoria.jsonl` con poda por edad; `GET /admin/partidas/:gameId/auditoria` con filtros y el recuento por comando de `GET /admin/metricas` | Un registro por orden (cuenta, héroe, tipo de cliente, código de orden, resultado o código de rechazo, coste en µs) junto a la crónica; servido con filtros |
| `src/server/informeDeCarga.ts` (+ `scripts/informe-carga.ts`) | Informe puro de una prueba de carga: agrega muestras y auditoría, compara bytes/min por cliente con el sondeo y dice qué mejora de rendimiento pide cada umbral (`UMBRALES`) | `GET /v1/telemetria/informe` (o el cálculo en el admin web a partir de las muestras) |

Todo, solo para cuentas de administrador (`Authorization: sesion <token>` de un nick de `ADMINISTRADORES`), y fuera del camino caliente del núcleo (bots y cargas de 500 jugadores, ver «Rendimiento con 500 bots» arriba). Las rutas y formatos son una propuesta, sin decidir: se cierran con el frente admin web cuando el autor dé la orden.
