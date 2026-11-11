# Lo que falta en Conquest

Todo lo que Conquest (`VRhino/conquest_prototype`) tiene que hacer para jugar las batallas del motor, sacado de los diseños del 2026-11-11 y de lo que ya está en `main`. Lo pidió el autor el 2026-11-11 a las 23:40.

Conquest tiene dos papeles:

- **Servidor de batallas.** Es un proceso por batalla, compilado como servidor dedicado. Lo arranca el lanzador del motor (`crates/lanzador`). Es autoritativo dentro de la batalla y su ejército sale del ticket.
- **Cliente del jugador en la batalla.** El jugador se conecta a ese proceso con la instancia y el token que le da el motor.

**El reparto** (autor, 2026-11-11): las reglas del mundo, el estado que dura y los datos de cada pieza (cifras de tropas, objetos, curvas) son del motor y viajan en `GET /v1/datos` y en el ticket. Todo lo táctico es de Conquest: el pipeline de daño, la resiliencia, los rasgos del golpe, las habilidades, la IA, los modelos y las animaciones. Cada número tiene un solo dueño.

**Cómo leer Conquest:** por su código (`Assets/Scripts/` y los `.asset`), no por su GDD ni su TDD, que están muy desactualizados.

**Qué indica cada estado:**
- **Listo en el motor:** ya está en `main`, así que Conquest puede empezar.
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
8. **Salidas:** `SalidaBatalla`, con el motivo, si quedó derrotado y los supervivientes y muertos de cada escuadra.
9. **El resultado:** ganador, razón y objetivos.
   - Por escuadra: desplegados, supervivientes al cierre y muertos.
   - Por héroe: si participó, si sobrevivió y si quedó derrotado.
   - La experiencia y el botín dependen de la decisión 7 (§7).
10. **Rechazar el ticket** que traiga una tropa, una clase, un objeto o una versión de catálogo que no conozca.
11. **IA:** maneja a los héroes con `controlador: bot` (también en bot contra bot) y a las escuadras sin héroe.

## 2. Las tropas

**Estado:**
- Las formaciones y el nivel ya están **listos en el motor**.
- El tier, las cifras de combate y los soldados **esperan al motor** (entrada 72, [`Cifras_De_Combate.md`](../Disenos_Por_Implementar/Cifras_De_Combate.md)).
- Las 10 tropas de las Eras IV y V esperan a la entrada 74 ([`Eras_IV_Y_V.md`](../Disenos_Por_Implementar/Eras_IV_Y_V.md)).

1. **Las 27 tropas de hoy como `SquadData` con el `tropaId` del motor** (CQ-003 y CQ-006, aceptadas y sin hacer). Hoy Conquest tiene 7 escuadras y ninguna con `tropaId`. Las levas sin formación (`lenadores` y `granjeros`), los `mineros` y los carros (`carros_guerra`) piden trabajo propio. Después vienen las 10 de las Eras IV y V.
2. **Los soldados de cada escuadra** son los del motor (Cifras §4: falange 32, distancia 20, caballerías 16…). La fuerza reservada de cada una es `efectivosAutorizados`.
3. **Las cifras de combate** (vida, daño, defensa, penetración, tiro…) se leen del bloque `combate` de cada tropa del motor, por versión, en lugar de las de sus `SquadData`.
4. **La rareza de escuadra sale del tier:** I leva (`levy_tier`), II milicia (`trained_tier`), III línea (`veteran_tier`), IV profesional (`elite_tier`) y V élite (`legendary_tier`).
5. **El nivel es el del ticket, del 1 al 10.** Da +3 % de valor por nivel, en lugar del ×2 en el 30 de `UnitStatScalingSystem`. En modo conectado dejan de usarse `SquadProgressionCurves` y la experiencia restante.
6. **Las habilidades de escuadra** se abren por nivel. `abilityUnlockLevels` pasa de 10/20/30 a la tabla de 1 a 10 (propuesto: 4, 7 y 10; decisión 8, §7).
7. **Las formaciones** ([`Heroe_Y_Tropa.md`](../Mechanics/Heroe_Y_Tropa.md) §10.4):
   - Cada tropa tiene su formación de reposo, la de seguir al héroe o pararse, con forma propia: unos granjeros van dispersos y unos hoplitas, juntos.
   - Conquest decide qué familias tiene cada tropa. Las familias son `reposo`, `linea`, `columna`, `dispersa`, `muro_escudos`, `testudo`, `cuadro` y `cuna`.
   - Solo se usan las de `formacionesPermitidas` del ticket. Se empieza en la de `formacion` si la tropa la tiene y, si no, en la de reposo.
8. **La experiencia de escuadra** es de 4 por victoria y 2 por derrota a cada escuadra que entró, con las cifras de `GET /v1/datos`. Puede pasar a ponerla el motor (decisión 7).
9. **El estandarte y el aspecto de cada escuadra**, del `SquadSnapshot`, para cuando haya arte.

## 3. El héroe: clases, atributos y nivel

**Estado: espera al motor** (entradas 71 y 73, [`Clases_De_Heroe.md`](../Disenos_Por_Implementar/Clases_De_Heroe.md) §8 y [`Progresion_Tropas_Y_Heroe.md`](../Disenos_Por_Implementar/Progresion_Tropas_Y_Heroe.md) §7).

1. **De cuatro clases a cinco, con los ids del motor:**

   | Clase | Trabajo en Conquest |
   | --- | --- |
   | `lanza_escudo_redondo` | nueva a partir de `Spear`, con escudo y bloqueo |
   | `espada_escudo_mediano` | `SwordAndShield` renombrada |
   | `arco_lanza` | `Bow` más el cambio a la lanza en batalla |
   | `hacha_dos_manos` | `TwoHandedSword` con hacha en lugar de mandoble |
   | `dagas` | nueva del todo |

   Se toca:
   - el enum `HeroClass`;
   - los `HeroClassDefinition` de `Resources/Data/HeroClasses`;
   - los sitios que miran la clase a mano: `HeroClassSelector`, `HeroHorseVisual`, `HeroMountPresenter`, `SkillIconCatalog` y el panel de perks.
2. **La clase sale del arma del ticket**, no de `HeroData.classId`.
3. **El héroe se monta desde el `HeroSnapshot`.** Hoy solo lee el id, el bando y si es bot.
4. **Los atributos base vienen del motor ya sumados.** El `HeroClassDefinition` se queda con los coeficientes, la resiliencia, los `hitTraits` y la velocidad.
5. **Las reglas del héroe** (Cifras §6):
   - **Vida** = 1000 + 30 por punto de vitalidad + la salud de la armadura.
   - **Alivio:** cada punto de defensa de un tipo quita un 0,25 % del daño de ese tipo, y cada punto del atributo armadura otro 0,25 % de todos, con tope del 75 %.
   - **Golpe** = daño del arma × (1 + 1 % por punto del atributo de daño de la clase).
6. **El nivel del héroe es el del ticket.** En modo conectado deja de usarse `HeroExperienceCalculator`.
   - Las habilidades se abren en los niveles 1, 3, 5 y 10, y los perks en el 7 y el 12.
   - Los perks desbloqueados llegan en `perksDesbloqueados`.
   - La experiencia es de 4 o 2, como la de la escuadra (decisión 7).
7. **Las habilidades y los perks de cada clase**, con sus nombres (Clases §3.1). Las que ya existen solo piden datos. Las mecánicas que no existen son nueve:

   | Mecánica | La piden |
   | --- | --- |
   | Silencio (impedir habilidades un tiempo) | Ardid |
   | Zonas que duran en el suelo | Flechas de fuego, Zona prohibida, Nadie pasa |
   | Daño por contacto | Nadie pasa |
   | Cambio de arma en batalla | Arco compuesto y lanza, Lanzada de retirada |
   | Invisibilidad (ni la IA ni los rivales lo ven) | Sigilo |
   | Sangrado al golpear | Hoja sucia, Golpe por la espalda |
   | Contraataque al bloquear | Parada y respuesta |
   | Un instante sin poder ser acertado | Finta |
   | Inmunidad temporal a retroceso y derribo | Mole, Nadie pasa |

8. **La IA de héroes** (`HeroAISkillSystem`): usar las habilidades nuevas, saber cuándo pasar del arco a la lanza e ir por la espalda con las dagas.

## 4. Los objetos

**Estado: espera al motor** (entrada 70, [`Objetos_Del_Heroe.md`](../Disenos_Por_Implementar/Objetos_Del_Heroe.md) §3.4 y [`Catalogo_De_Objetos.md`](../Disenos_Por_Implementar/Catalogo_De_Objetos.md) §8).

1. **Los ids de `ItemDataSO` son los del catálogo del motor.**
   - Las 12 armaduras de hoy se renombran a las de inicio.
   - Se crean las 75 de los demás metales, cascos incluidos, con su `visualPartId`.
   - Se crean las 30 armas: cinco clases por seis metales.
2. **La rareza de cada objeto sale de su tier.**
3. **Las cifras de cada pieza** (§4 del catálogo) se leen del bloque `OBJETOS` del motor, por versión, en lugar del generador al azar.
4. **Todas las cifras del objeto entran al combate** como modificadores de la pila por `StatId` (su pendiente P1): atributos, daño, penetración, defensas y salud. Hoy solo pasan los cuatro atributos.
5. **Las piezas forjadas traen sus valores en el ticket**, con una calidad del 90 al 110 %.
6. **El equipo inicial:** `HeroClassSelector.FillBaseItemIdsForClass` usa los ids de inicio, sin casco.
7. **La montura** se lee del ticket y usa sus cifras, en lugar de las fijas de `MountTuning`.
8. **En modo conectado se quitan** la bolsa de bronce, el cofre y los consumibles.
9. **El fichero de stats y descripciones** de cada pieza, para los tooltips del cliente jugador (Objetos §2, punto 9).
10. **El botín:** si el motor lo pone con sus tablas (decisión 7), Conquest no lo genera.

## 5. Conquest como cliente del jugador

**Estado: espera al motor** (entrada 70).

1. **La ventana de inventario** manda órdenes al motor, en lugar de usar los servicios locales `EquipmentManagerService`, `StoreTransactionService` y `SaveSystem`.
2. **El equipo de los demás héroes** se pinta desde el ticket.
3. **La conexión a la batalla** usa la instancia y el token de `Batalla.asignacion_propia` (Batalla_Unity_Ciclo §8.7).

## 6. Arte

- **Armas:** las cinco en sus seis metales (inicio, cobre, hierro temprano, bronce, hierro y acero) y los dos escudos (redondo grande y mediano).
- **Armaduras:** las 87 del catálogo, con los cascos.
- **Animaciones:** las de cada clase. Hoy todas las habilidades usan la misma animación provisional, y la clase dagas no tiene ninguna.
- **Tropas:** los modelos de las 27 de hoy y de las 10 de las Eras IV y V.
- **Edificios:** los modelos de los edificios nuevos (CQ-006).
- **Estandartes:** cada uno con su arte.

## 7. Lo que espera una decisión del autor

De las ocho que quedan para el motor (2026-11-11 23:38), tocan a Conquest tres:

- **6.** Si una batalla suelta armas y armaduras hechas. Propuesto: no. El botín trae solo crisoles, planos, medallas y el cofre de monturas.
- **7.** Qué manda Unity en el resultado. Propuesto: quién ganó, quién participó y quién sobrevivió. La experiencia (4 o 2) y el botín los pone el motor.
- **8.** En qué niveles abre una escuadra sus habilidades. Propuesto: el 4, el 7 y el 10.

Las propuestas CQ-001 (curvas) y CQ-004 (objetos) de Conquest se actualizan con este reparto.

## 8. Orden propuesto

1. **La red y el protocolo con el lanzador**, con las tropas de hoy (§1). Se prueba contra el motor real con el servidor de batalla de mentira como modelo.
2. **Las 27 tropas** con su `tropaId`, sus formaciones y el nivel 1-10 (§2.1 y §2.5-§2.8).
3. **Las clases, los atributos y el nivel del héroe** (§3), cuando el motor tenga las entradas 71 y 73.
4. **Los objetos y las cifras de combate** (§2.2-§2.4, §4 y §5), con las entradas 70 y 72.
5. **Las tropas de las Eras IV y V**, con la entrada 74.

El arte (§6) va en paralelo.
