# Lo que falta en Conquest

Todo lo que Conquest (`VRhino/conquest_prototype`) tiene que hacer para jugar las batallas del motor, sacado de los diseños del 2026-11-11 y de lo que ya está en el motor. Lo pidió el autor el 2026-11-11 a las 23:40. Desde el 2026-11-12 este documento guarda además la parte de Conquest de esos diseños (habilidades, perks, las reglas del héroe en la batalla), que ya no están en `Docs/Disenos_Por_Implementar/`: lo del motor está en `Docs/Mechanics/`.

Conquest tiene dos papeles:

- **Servidor de batallas.** Es un proceso por batalla, compilado como servidor dedicado. Lo arranca el lanzador del motor (`crates/lanzador`). Es autoritativo dentro de la batalla y su ejército sale del ticket.
- **Cliente del jugador en la batalla.** El jugador se conecta a ese proceso con la instancia y el token que le da el motor.

**El reparto** (autor, 2026-11-11): las reglas del mundo, el estado que dura y los datos de cada pieza (cifras de tropas, objetos, curvas) son del motor y viajan en `GET /v1/datos` y en el ticket. Todo lo táctico es de Conquest: el pipeline de daño, la resiliencia, los rasgos del golpe, las habilidades, la IA, los modelos y las animaciones. Cada número tiene un solo dueño.

**Cómo leer Conquest:** por su código (`Assets/Scripts/` y los `.asset`), no por su GDD ni su TDD, que están muy desactualizados.

**Qué indica cada estado:**
- **Listo en el motor:** el motor ya lo publica, así que Conquest puede empezar. Lleva el commit: los de la tanda de objetos, clases, cifras, progresión y Eras IV-V (2026-11-12) están en `main` desde los PR #22 y #23 (merge `c82d6ed`).
- **Espera al motor:** está diseñado, pero el motor aún no lo publica. Lleva el número de su entrada en `Docs/Mecanicas a desarrollar.md`.
- **Espera una decisión:** falta que el autor lo cierre.

## 1. El servidor de batallas en red

**Estado: listo en el motor** (PR #18, `main` e89f8d3): el lanzador, las cuentas de servidor de batalla, el interruptor `BRONZEAGE_BATALLAS_EN_UNITY` y el servidor de batalla de mentira. Referencia: [`Batalla_Unity_Ciclo.md`](../Mechanics/Batalla_Unity_Ciclo.md) §3.2 y §8.

1. **Pasar a Netcode for Entities como servidor dedicado.** Hoy Conquest tiene Netcode for GameObjects 2.13.1 y ningún contrato de red. Es el punto 1 de «Lo que falta» de su `22_Servidor_De_Batalla_Red`, y todo lo demás depende de él.
2. **El movimiento del héroe a ECS**, en el servidor y en el cliente. Hoy lo hace `LocalHeroCharacterMotor`, que es un MonoBehaviour.
3. **El arranque.** El lanzador llama al proceso con `--batalla <uuid> --lanzador 127.0.0.1:<p> --puerto <n>`. El proceso tiene 120 s para conectarse a ese socket TCP local; si no, el lanzador lo mata.
4. **El protocolo con el lanzador.** Son mensajes Protobuf delimitados, con la longitud en varint delante (`WriteDelimitedTo` y `ParseDelimitedFrom`), de hasta 16 MB.
   - Le llega `MensajeParaProcesoDeBatalla` (`session.proto`). Primero vienen el ticket y la asignación con los tokens de los humanos. Después llegan cada incorporación, los tokens de quien se une más tarde y el acuse del motor a cada mensaje suyo.
   - Manda `MensajeDeServidorDeBatalla`: `inicio`, `salida` y `resultado`.
   - Tras el acuse de su resultado sale; si no sale, el lanzador lo mata a los 10 s.
   - El ejemplo que funciona es el servidor de batalla de mentira (`lanzador::mentira`, ejecutable `bronzeage-batalla-de-mentira`).
5. **El token del jugador.** El proceso compara el token cuando un cliente se conecta. El motor nunca lo verifica.
6. **Montar la batalla desde el ticket:**
   - **Contexto:** asedio con el mapa de la plaza (`SettlementBattleSnapshot`), campo abierto, caravana, campamento de bandidos, saqueo de pueblo o avanzada. La caravana puede traer varias y nombra cada carro perdido como `"<caravana>:<posición>"`.
   - **Bandos:** participantes humanos y bots, y escuadras sin héroe (la guarnición, los bandidos y los carreteros).
   - **Terreno:** sale del artefacto del mundo, por su hash y el centro, con la altura del paquete de relieve ([`06_Contrato_de_Mundo.md`](06_Contrato_de_Mundo.md) §4 y §8).
   - **Reglas:** la duración máxima y las versiones de los catálogos.
7. **Incorporaciones:** quien se une con la batalla ya empezada.
8. **Salidas:** `SalidaBatalla`, con el motivo, si quedó derrotado y los supervivientes y muertos de cada escuadra. Cuándo saca Conquest a un héroe (decisiones del autor del 2026-11-12, sobre el Doc 5.15.1):
   - **Se desconecta a media batalla:** la IA lleva a su héroe 60 s para que pueda volver; si no vuelve, sale (`desconexion`) con sus bajas y herido.
   - **No llega a conectarse antes de empezar:** sale sin castigo (`no_conectado`) y el motor reabre la convocatoria sin él. No lo juega la IA.
   - **Abandona:** una orden del jugador; sale (`abandono`) con sus bajas y herido.
   - **Muere donde no se reaparece:** que el héroe reaparezca lo decide el tipo de batalla. **En campo abierto no reaparece:** al morir sale (`eliminado`), herido y con sus bajas. En asedio, caravana, campamento de bandidos, saqueo de pueblo y avanzada reaparece, igual los héroes de los jugadores que los de la IA.
9. **El resultado:** ganador, razón y objetivos.
   - Por escuadra: desplegados, supervivientes al cierre y muertos.
   - Por héroe: si participó y si sobrevivió. **`derrotado` no se manda** (decisión del autor, 2026-11-12): el héroe gana o pierde con su bando, caiga o no en la batalla, y sin ese dato el motor hiere a los del bando perdedor ([`Batalla_Unity_Ciclo.md`](../Mechanics/Batalla_Unity_Ciclo.md) §9.1).
   - **Nada más** (D7, 2026-11-12): la experiencia y el botín los pone el motor (§7). `ResultadoEscuadra.xp_ganada` (5), `ResultadoHeroe.xp_ganada` (4) y `ResultadoHeroe.botin` (6) son `reserved`, y los mensajes `Botin` y `ObjetoBotin` ya no existen.
10. **Rechazar el ticket** que traiga una tropa, una clase, un objeto o una versión de catálogo que no conozca. Las versiones van en `BattleRules`: `version_catalogo_tropas` (3, hoy 2: `VERSION_CATALOGO_TROPAS`), `version_catalogo_heroe` (4, la `version` de `CLASES_HEROE`, hoy `bronzeage-clases-1`) y `version_catalogo_objetos` (5, la de `OBJETOS`, hoy `bronzeage-objetos-1`, que sustituye a la fija `conquest-items-1`); los tres bloques se leen de `GET /v1/datos`.
11. **IA:** maneja a los héroes con `controlador: bot` (también en bot contra bot) y a las escuadras sin héroe. Cada escuadra sin héroe sale con un capitán de la IA que no figura en el resultado (decisión del autor, 2026-11-12).

## 2. Las tropas

**Estado:**
- Las formaciones y el nivel ya están **listos en el motor**.
- El tier, las cifras de combate y los soldados están **listos en el motor** (commit `1d87eb1`; [`Heroe_Y_Tropa.md`](../Mechanics/Heroe_Y_Tropa.md) §12).
- Las 10 tropas de las Eras IV y V están **listas en el motor** (commits `b39fdc1` y `ab793fc`; 37 tropas en `TROPAS_RECLUTABLES`).

1. **Las 37 tropas como `SquadData` con el `tropaId` del motor** (CQ-003 y CQ-006, aceptadas y sin hacer). Hoy Conquest tiene 7 escuadras y ninguna con `tropaId`. Las levas sin formación (`lenadores` y `granjeros`), los `mineros` y los carros (`carros_guerra`) piden trabajo propio. El Batallón Sagrado no existe (lo quitó el autor el 2026-11-11).
2. **Los soldados de cada escuadra** son los del motor (`unidadesPorDefecto`: falangitas 32, las de distancia 20, hipaspistas 20, hequetai 18, espartiatas y las caballerías de tier V 16…). La fuerza reservada de cada una es `efectivosAutorizados`.
3. **Las cifras de combate** se leen del bloque `combate` de cada tropa en `TROPAS_RECLUTABLES`, por `version_catalogo_tropas`, en lugar de las de sus `SquadData`: `vida` por soldado; `dano`, `penetracion`, `defensa` (plana) y `alivio` (en %), cada una con su terna perforante/cortante/contundente; `bloqueo` del escudo; `cadencia` (segundos entre golpes); `alcance` (metros) y, en las de distancia, `precision` (1-100) y `municion` (para `SquadRangedData`). Son un borrador que el autor revisará tropa a tropa al probarlas en batalla.
4. **La rareza de escuadra sale del `tier`:** I leva (`levy_tier`), II milicia (`trained_tier`), III línea (`veteran_tier`), IV profesional (`elite_tier`) y V élite (`legendary_tier`). El tier solo pone las cifras de combate; el `escalon` sigue poniendo el Liderazgo, el oro y la clase de población.
5. **Cómo se hicieron las cifras**, para cuando Conquest las aplique y el autor las revise:
   - El tier III vale 1: 1000 de vida y 100 por golpe, un golpe cada 1,5 s. Cada tier vale el doble que el anterior por soldado (aguanta ×1,5 y daña ×1,35): vida 450, 670, 1000, 1500 y 2250 y golpe 55, 75, 100, 135 y 180 del I al V. «Aguante» es la vida efectiva frente al mismo golpe (vida, defensa y alivio juntos); «daño», lo que quita por segundo.
   - Dentro del tier, el **papel** multiplica el aguante y el daño y pone la cadencia (leva ×1/×1 cada 1,6 s; lanceros ×1,2/×0,85, 1,5 s; espadachines ×1/×1, 1,4 s; hacheros ×0,85/×1,2, 1,8 s; honderos ×0,7/×1,4, 1,8 s; jabalinas ×0,8/×1,25, 2,5 s; arqueros ×0,7/×1,45, 2 s; exploradores ×0,8/×0,85, 1,6 s; jinetes ×0,9/×1,1, 1,6 s; caballería ×1/×1, 1,8 s; carros ×1,1/×0,95, 2,2 s; guardia de élite ×1,1/×0,95, 1,5 s; lanza y arco ×0,95/×1,1, 1,6 s). Vida = 1000 × aguante del tier × aguante del papel; golpe = 100 × daño del tier × daño del papel × cadencia / 1,5, repartido entre los tipos según el arma.
   - **La armadura va encima de la vida** (mejor equipo, mejor tropa): ninguna 0 de defensa y 0/0/0 de alivio; básica 5 y 5/5/10; intermedia 10 y 10/10/10; de bronce 15 y 20/20/10; de bronce de calidad 20 y 25/25/12; el carro 10 y 15/15/5. **El escudo** da bloqueo y alivio contra la punta: mimbre 100 y +5, pelta 80 y +5, mediano 250 y +5, grande 400 y +10. **El metal del arma** pone la penetración en cada tipo que hace: madera 0, piedra 2, cobre 5, bronce 10, bronce de calidad 15, hierro 20 y hierro de calidad 25.
   - **El tiro:** precisión de base por tier (II 35, III 50, IV 60, V 70; la jabalina +10, la honda y la lanza y arco −5); munición: honda 30, jabalina 4, arco 24, lanza y arco 16.
   - **Los rasgos del golpe** (cuánto suma la carga, cuánto pierde el tiro con la distancia, cuánto gasta el escudo) son reglas de cada arma en Conquest: el motor solo guarda los números (autor, 2026-11-11 21:14).
6. **El nivel es el del ticket, del 1 al 10.** Da +3 % de valor por nivel (propuesta: +1,5 % de aguante y +1,5 % de daño), como con números, en lugar del ×2 en el 30 de `UnitStatScalingSystem`. En modo conectado dejan de usarse `SquadProgressionCurves` y la experiencia restante. **Listo en el motor.**
7. **Las habilidades de escuadra** se abren en los niveles 4, 7 y 10 (D8): `abilityUnlockLevels` pasa de 10/20/30 a los de `MILITAR.nivelesHabilidadEscuadra`, que el motor publica en `GET /v1/datos`. Qué habilidad abre cada uno es de Conquest. **Listo en el motor** (commit `c38ec28`).
8. **Las formaciones** ([`Heroe_Y_Tropa.md`](../Mechanics/Heroe_Y_Tropa.md) §10.4):
   - Cada tropa tiene su formación de reposo, la de seguir al héroe o pararse, con forma propia: unos granjeros van dispersos y unos hoplitas, juntos.
   - Conquest decide qué familias tiene cada tropa. Las familias son `reposo`, `linea`, `columna`, `dispersa`, `muro_escudos`, `testudo`, `cuadro` y `cuna`.
   - Solo se usan las de `formacionesPermitidas` del ticket. Se empieza en la de `formacion` si la tropa la tiene y, si no, en la de reposo.
9. **La experiencia de escuadra la pone el motor** (D7): 4 por victoria y 2 por derrota a cada escuadra que entró (`MILITAR.experienciaUnityPorVictoria` y `experienciaUnityPorDerrota`). Conquest no la calcula ni la manda (`xp_ganada` es `reserved`). **Listo en el motor** (commit `c38ec28`).
10. **El estandarte y el aspecto de cada escuadra** (`SquadSnapshot.estandarte`, 13, y `aspecto`, 14): viajan vacíos (D20). El motor no los guarda ni tiene orden para elegirlos hasta que haya un estandarte en el catálogo y arte (pendiente en `Docs/Mecanicas a desarrollar.md`). Conquest solo tiene que leerlos cuando lleguen.

## 3. El héroe: clases, atributos y nivel

**Estado: listo en el motor** (commits `1ebd64f` y `c38ec28`; [`Heroe_Y_Tropa.md`](../Mechanics/Heroe_Y_Tropa.md) §3 y §4 y [`Objetos_Del_Heroe.md`](../Mechanics/Objetos_Del_Heroe.md) §3.2). Las habilidades, los perks y las reglas del héroe en la batalla son de Conquest y están aquí.

1. **De cuatro clases a cinco, con los ids del motor** (bloque `CLASES_HEROE`; `crear_heroe` rechaza una clase que no está en él, D21):

   | Clase | Inspiración | Papel | Trabajo en Conquest |
   | --- | --- | --- | --- |
   | `lanza_escudo_redondo` | Leónidas | aguantar la línea, anticarga | nueva a partir de `Spear`, con escudo y el bloqueo de `SwordAndShield` |
   | `espada_escudo_mediano` | Aquiles | duelista de primera línea | `SwordAndShield` renombrada |
   | `arco_lanza` | Ulises | distancia, con lanza para el cuerpo a cuerpo | `Bow` más el cambio a la lanza en batalla |
   | `hacha_dos_manos` | Ayax | fuerza bruta, rompe líneas | `TwoHandedSword` con hacha en lugar de mandoble |
   | `dagas` | el espía y asesino | rápido, golpes por la espalda | nueva del todo |

   Se toca:
   - el enum `HeroClass`;
   - los `HeroClassDefinition` de `Resources/Data/HeroClasses`;
   - los sitios que miran la clase a mano: `HeroClassSelector`, `HeroHorseVisual`, `HeroMountPresenter`, `SkillIconCatalog` y el panel de perks.
2. **La clase sale del arma del ticket** (`HeroSnapshot.class_definition_id` es la `clase` del arma puesta), no de `HeroData.classId`. El arma nunca falta.
3. **El héroe se monta desde el `HeroSnapshot`.** Hoy solo lee el id, el bando y si es bot.
4. **Los atributos base vienen del motor ya sumados** (D16): `atributos_efectivos` (8) lleva la base de la clase más los puntos repartidos; **el equipo lo suma Conquest**. El `HeroClassDefinition` se queda con los coeficientes, la resiliencia, los `hitTraits` y la velocidad.
5. **Las reglas del héroe** (propuestas por el autor el 2026-11-11 para Conquest; con ellas están ajustadas las cifras de `OBJETOS`):
   - **Vida** = 1000 + 30 por punto de vitalidad + la salud de la armadura. Es igual para todas las clases.
   - **Alivio:** cada punto de defensa de un tipo quita un 0,25 % del daño de ese tipo, y cada punto del atributo armadura otro 0,25 % de todos, con tope del 75 % (el `maxRelief` de hoy).
   - **Golpe** = daño del arma × (1 + 1 % por punto del atributo de daño de la clase): la fuerza en la lanza y el hacha, la destreza en el arco y las dagas, la media de las dos en la espada.
   - **Referencias del autor:** con el equipo de tier I el héroe aguanta como una unidad de tier V y daña como una de tier III (2250 de vida efectiva y 100 de golpe); con el de tier V, como tres de tier V y una de tier V (6750 y 180); en línea recta entre medias. Con armadura media da justo esas cifras; la pesada aguanta algo más y pega algo menos, y la ligera al revés. Cada clase con su armadura inicial y sin puntos (vida efectiva / golpe, tiers I a V): lanza 2381/96, 3495/122, 4715/131, 5921/151, 7393/167; espada 2243/99, 3207/133, 4364/139, 5446/160, 6806/178; arco 1984/110, 2928/152, 3891/159, 4971/183, 6048/211; hacha 2269/101, 3350/138, 4385/141, 5465/162, 6822/181; dagas 1979/111, 2920/142, 3880/159, 4957/183, 6030/212. Una pieza forjada al 110 % nunca aguanta como una normal del tier siguiente.
6. **El nivel del héroe es el del ticket** (`HeroSnapshot.nivel`). En modo conectado deja de usarse `HeroExperienceCalculator` (100 × 1,2^(n−1)).
   - Las habilidades se abren en los niveles 1, 4, 8 y 16 (la definitiva) y los perks en el 11 (el pasivo) y el 20 (el activo), solos, sin puntos: `PROGRESION_HEROE.habilidadesEnNivel` y `perksEnNivel` (desde el 2026-11-13, con el tope del héroe en el 50; antes 1, 3, 5 y 10, y 7 y 12). Conquest debe leerlos de `GET /v1/datos`, no tenerlos fijos.
   - **Aspecto del héroe (2026-11-13):** el motor guarda el género y las piezas del avatar de `crearHeroe` y los manda en `HeroSnapshot.femenino` y `HeroSnapshot.avatar` (y en `Heroe` propio y público). Sin catálogo de piezas, valida solo la forma del id (hasta 32 letras ASCII, cifras, `_` o `-`; vacío = pieza por defecto). Conquest tiene que mandar el aspecto del creador de avatar en `OrdenCrearHeroe.genero`/`avatar` y vestir al héroe con lo del ticket; cuando publique su catálogo de piezas, el motor puede validarlas contra él.
   - Los perks los abre el nivel (D15): ni el motor ni el ticket los guardan. `HeroSnapshot.perks_desbloqueados` (6), `Heroe.puntos_de_perk_sin_gastar` (12) y `perks_desbloqueados` (14), y `perks_seleccionados` de `Loadout` (4) y de `OrdenGuardarLoadout` (4) son `reserved`. Al cambiar de arma solo cuentan los de la clase del arma puesta.
   - La experiencia la pone el motor (D7): 4 si gana su bando y 2 si pierde, a cada héroe que participó.
7. **Las habilidades y los perks de cada clase** (cerrados con el autor clase a clase el 2026-11-11; entre paréntesis, la plantilla de Conquest de la que salen). Lo que hacen en combate (cifras, alcances, tiempos) lo fija Conquest. Los atributos de cada clase están en `CLASES_HEROE`.
   - **Lanza y escudo grande redondo.** Lento, con un bloqueo frontal grande, alcance de lanza, daño perforante y mucha resiliencia. Q, Golpe de escudo: empuja al de delante y le rompe la formación (Shield Bash). E, Estocada sobre el escudo: golpe perforante largo que atraviesa escudos (Precise Thrust). R, Escudos trabados: más defensa para él y su escuadra mientras no se mueven (Reinforced Defense). Definitiva, Nadie pasa: él y su escuadra no retroceden, ganan defensa, y quien les carga se frena y recibe daño (Unbreakable Wall y Spike Wall). Perk pasivo, Línea inquebrantable: quieto 2 s, su escuadra recibe menos daño (Unbreakable Line). Perk activo, Orden de aguantar: los aliados cercanos mitigan más daño durante 6 s (Endurance Order).
   - **Espada corta y escudo mediano.** Duelista que va a por el enemigo; más rápido que la lanza, golpes cortantes y estocadas, bloqueo y resiliencia medios. Q, Acometida: salto corto hacia un enemigo y golpe de escudo que lo desequilibra (Interception). E, Tres tajos: combo de tres golpes con daño creciente (Steel Rain). R, Parada y respuesta: alza el escudo un instante y, si para un golpe, contraataca con daño extra (nueva). Definitiva, Furia del campeón: unos segundos pega más y más rápido, y su escuadra gana ataque (Judgment of Steel). Perk pasivo, Primero en la brecha: el primer golpe tras moverse hace más daño y frena al enemigo (Destabilizing Thrust). Perk activo, Carga conjunta: si carga con su escuadra, todos hacen más daño (Coordinated Charge).
   - **Arco compuesto y lanza.** Rápido, daño perforante a distancia que baja con los metros; con la lanza se defiende, pero no es de primera línea. Q, Disparo certero: flecha que atraviesa armadura y hace mucho daño a los pesados (Focused Shot). E, Lluvia de flechas: andanada sobre una zona que desordena las formaciones (Arrow Rain). R, Lanzada de retirada: con la lanza, golpe que empuja al enemigo y le deja saltar atrás para volver al arco (nueva). Definitiva, Flechas de fuego: una zona ardiendo que hace daño con el tiempo (Flaming Arrow). Perk pasivo, Tirador apostado: quieto 3 s, sus flechas frenan al enemigo (Harassment Cover). Perk activo, Ardid: una flecha que impide al objetivo usar habilidades un rato (Silencing Arrow, que en Conquest es habilidad). Las cifras del arma de `OBJETOS` son las del arco; la lanza la pone Conquest con el cambio de arma.
   - **Hacha a dos manos.** Golpes lentos y muy duros, cortantes y contundentes; parte del daño pasa el bloqueo de los escudos y aturde bastante; sin escudo, resiliencia media. Q, Molinete: golpe en círculo que abre hueco alrededor (Whirlwind Slash). E, Embestida brutal: atraviesa enemigos y les desordena la línea (Brutal Charge). R, Parte escudos: hachazo de arriba abajo que rompe el bloqueo y aturde (nueva). Definitiva, Rompelíneas: carga en línea recta derribando a quien encuentra, y su escuadra le sigue más rápido (nueva). Perk pasivo, Mole: mientras ataca no se le puede derribar ni hacer retroceder (nuevo). Perk activo, Zona prohibida: marca el suelo; los enemigos que lo cruzan van más lentos y defienden peor (Exclusion Zone).
   - **Dagas.** La más rápida; golpes cortos y seguidos, cortantes y perforantes, con mucho más daño por la espalda o el flanco; no bloquea, esquiva; resiliencia baja. Q, Golpe por la espalda: mucho daño si está detrás del objetivo, y lo deja sangrando (nueva). E, Finta: salto corto en cualquier dirección y un instante en el que cuesta acertarle (nueva). R, Sigilo: se vuelve invisible durante un tiempo (nueva). Definitiva, Danza de dagas: una ráfaga de golpes muy rápidos sobre un objetivo (nueva). Perk pasivo, Hoja sucia: sus golpes hacen sangrar, y el sangrado se acumula (nuevo). Perk activo, Marca del cazador: marca a un enemigo, y él y su escuadra le hacen más daño (la idea de Marking Arrow).

   **Lo que ya tiene Conquest y basta con datos:** embestida (`Dash`), golpes en área y en cono, modificadores temporales (defensa, daño, velocidad de ataque y de movimiento), la condición «quieto N segundos», bonos por flanco y espalda (`ApplyConditional`), derribo como reacción de una habilidad, golpes que pasan el escudo (`blockDamageRatio`, `ignoresShield`), el bloqueo del héroe con escudo y los perks Línea inquebrantable, Orden de resistencia, Carga coordinada, Estocada desestabilizadora, Cobertura de hostigamiento y Flecha marcadora. **Las mecánicas que no existen** son nueve (las tres primeras, en su pendiente H4):

   | Mecánica | La piden |
   | --- | --- |
   | Silencio (impedir habilidades un tiempo; hoy la Flecha Silenciadora solo ralentiza) | Ardid |
   | Zonas que duran en el suelo | Flechas de fuego, Zona prohibida, Nadie pasa |
   | Daño por contacto | Nadie pasa |
   | Cambio de arma en batalla | Arco compuesto y lanza, Lanzada de retirada |
   | Invisibilidad (ni la IA ni los rivales lo ven; el servidor no lo enseña) | Sigilo |
   | Sangrado al golpear (el daño en el tiempo solo existe en habilidades de área) | Hoja sucia, Golpe por la espalda |
   | Contraataque al bloquear | Parada y respuesta |
   | Un instante sin poder ser acertado | Finta |
   | Inmunidad temporal a retroceso y derribo (hoy solo hay un permiso fijo por clase) | Mole, Nadie pasa |

8. **La IA de héroes** (`HeroAISkillSystem`): usar las habilidades nuevas, saber cuándo pasar del arco a la lanza e ir por la espalda con las dagas.

## 4. Los objetos

**Estado: listo en el motor** (commits `fa08c46`, `1ebd64f`, `d42ac78` y `0c105c6`; [`Objetos_Del_Heroe.md`](../Mechanics/Objetos_Del_Heroe.md)).

1. **Los ids de `ItemDataSO` son los del catálogo `OBJETOS` del motor.**
   - Las 12 armaduras de hoy se renombran a las de inicio (`TorLADef` → `torso_ligera_inicio`, `BootHADef` → `botas_pesada_inicio`, etc.).
   - Se crean las 75 de los demás metales, cascos incluidos, con su `visualPartId`.
   - Se crean las 30 armas: cinco clases por seis metales (inicio, cobre, hierro temprano, bronce, hierro y acero).
   - Se crean las cinco monturas (`montura_tiro` … `montura_nisea`).
2. **La rareza de cada objeto sale de su tier** (`OBJETOS.rarezas`).
3. **Las cifras de cada pieza** (`combate`) se leen del bloque `OBJETOS` del motor, por `version_catalogo_objetos`, en lugar del generador al azar. Ligera, media y pesada dan cosas distintas ([`Objetos_Del_Heroe.md`](../Mechanics/Objetos_Del_Heroe.md) §2.1).
4. **Todas las cifras del objeto entran al combate** como modificadores de la pila por `StatId` (su pendiente P1): atributos, daño, penetración, defensas y salud. Hoy solo pasan los cuatro atributos.
5. **Las piezas forjadas traen sus valores en el ticket** (D13): `item_instance_id` (`obj-N`) y `estadisticas`, ya multiplicadas por la calidad (del 90 al 110 %), con los nombres en camelCase: `fuerza`, `destreza`, `armadura`, `vitalidad`, `danoPerforante`, `danoCortante`, `danoContundente`, `penetracionPerforante`, `penetracionCortante`, `penetracionContundente`, `defensaPerforante`, `defensaCortante`, `defensaContundente` y `salud`. Las de tienda y botín van sin ellos: son la ficha al 100 %. `ItemInstancia.tipo` es el `tipo` de la ficha (`arma`, `armadura`, `montura`, `material` o `consumible`).
6. **El equipo inicial:** `HeroClassSelector.FillBaseItemIdsForClass` usa los ids de inicio, sin casco, y la montura de tier I. En modo conectado el equipo llega en el ticket.
7. **La montura** (`Equipamiento.montura`, 7) se lee del ticket y usa sus cifras (`vida`, `trote`, `galope`, `aguante`), en lugar de las fijas de `MountTuning` (vida 1200, trote 6, galope 11 y aguante 100, que son las de `montura_tiro`).
8. **En modo conectado se quitan** la bolsa de bronce, el cofre de arma y los consumibles de batalla: no hay consumibles en la batalla.
9. **Los tooltips del cliente jugador** salen de `OBJETOS` del motor: Conquest ya no tiene que exportar un fichero de stats.
10. **El botín lo pone el motor** (D6 y D7): cada héroe que participó tira la tabla `unity` de `BOTIN_OBJETOS` (crisoles, planos, medallas y el cofre, nunca piezas hechas; contra bandidos, la tabla de bandidos de su nivel en su lugar) y las monedas salen de `MONEDAS` como con números. Conquest no genera botín ni lo manda (`ResultadoHeroe.botin`, 6, `reserved`), ni necesita saber las casillas libres de la mochila (`HeroSnapshot.casillas_inventario_libres`, 9, `reserved`; `BATALLA.casillasInventario` ya no existe). Lo que no cabe en la mochila va al almacén de objetos o se pierde, en el motor.
11. **El equipo no se pierde al perder una batalla** (D4): no hay durabilidad.

## 5. Conquest como cliente del jugador

**Estado: listo en el motor** (commits `1ebd64f` y `d42ac78`).

1. **La ventana de inventario** manda órdenes al motor, en lugar de usar los servicios locales `EquipmentManagerService`, `StoreTransactionService` y `SaveSystem`: `equipar_objeto` (192), `desequipar_objeto` (193), `guardar_objeto` (194), `sacar_objeto` (195), `comprar_objeto` (196), `vender_objeto` (197), `forjar_objeto` (198) y `usar_objeto` (199) ([`Heroe_Y_Tropa.md`](../Mechanics/Heroe_Y_Tropa.md) §4.5 y §4.6). La mochila es `Heroe.inventario` (72 casillas) y el almacén de objetos, `Heroe.almacen_objetos` (44; 144 casillas). Se equipa dentro de una plaza, un pueblo o un campamento y nunca con escuadras reservadas para una batalla (D2).
2. **El equipo de los demás héroes** se pinta desde el ticket.
3. **La conexión a la batalla** usa la instancia y el token de `Batalla.asignacion_propia` (Batalla_Unity_Ciclo §8.7).

## 6. Arte

- **Armas:** las cinco en sus seis metales (inicio, cobre, hierro temprano, bronce, hierro y acero) y los dos escudos (redondo grande y mediano).
- **Armaduras:** las 87 del catálogo, con los cascos.
- **Monturas:** las cinco.
- **Animaciones:** las de cada clase. Hoy todas las habilidades usan la misma animación provisional, y la clase dagas no tiene ninguna.
- **Tropas:** los modelos de las 37 de `TROPAS_RECLUTABLES`.
- **Edificios:** los modelos de los edificios nuevos (CQ-006).
- **Estandartes:** cada uno con su arte, cuando el autor los diseñe (D20).

## 7. Las decisiones del autor que tocan a Conquest

Cerradas el 2026-11-12, con las demás del plan de la tanda:

- **D6.** Una batalla de Unity no suelta armas ni armaduras hechas: el botín trae crisoles, planos, medallas y el cofre de monturas, con las tablas del motor.
- **D7.** De Unity el motor lee quién ganó, quién participó, quién sobrevivió y los soldados que quedan. La experiencia (4 o 2) y el botín los pone el motor, y las monedas salen de `MONEDAS`, como con números.
- **D8.** Las escuadras abren sus habilidades en los niveles 4, 7 y 10, publicados en `MILITAR`.
- **D15.** Los perks se abren solos con el nivel y los define Conquest; el motor solo publica los niveles (7 y 12) y manda el nivel en el ticket.
- **D16.** `atributos_efectivos` lleva la base de la clase más los puntos repartidos; el equipo lo suma Conquest.
- **D20.** El estandarte y el aspecto de la escuadra: solo los dos campos del `SquadSnapshot`, vacíos.

Las propuestas CQ-001 (curvas) y CQ-004 (objetos) de Conquest se actualizan con este reparto.

## 8. Orden propuesto

1. **La red y el protocolo con el lanzador** (§1). Se prueba contra el motor real con el servidor de batalla de mentira como modelo.
2. **Las 37 tropas** con su `tropaId`, sus formaciones y el nivel 1-10 (§2.1 y §2.6-§2.9).
3. **Las clases, los atributos y el nivel del héroe** (§3).
4. **Los objetos y las cifras de combate** (§2.2-§2.5, §4 y §5).

El arte (§6) va en paralelo.
