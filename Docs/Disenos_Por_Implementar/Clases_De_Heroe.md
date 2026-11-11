# Clases del héroe: el arma define la clase

Diseño cerrado con el autor el 2026-11-11 (hilo «Diseñar clases de héroe»), sin implementar. Entrada 71 de `Docs/Mecanicas a desarrollar.md`. Va con [`Objetos_Del_Heroe.md`](Objetos_Del_Heroe.md): sustituye su punto 2 de §3.1 («guardar la clase») y contesta su duda 6 de §4 («si la clase limita el arma»). Las armas y armaduras concretas están en el diseño del catálogo de objetos (`Catalogo_De_Objetos.md`, en su propio hilo).

Los atributos base, las habilidades y los perks de cada clase se cerraron **una a una** con el autor el mismo día (§3.1).

## 1. De dónde se parte

### 1.1 Conquest

Leído el 2026-11-11 de `VRhino/conquest_prototype` (commit `562ac34c`).

- **Cuatro clases** (`HeroClass`): `SwordAndShield`, `TwoHandedSword`, `Spear` y `Bow`. Cada una es un `HeroClassDefinition` en `Assets/Resources/Data/HeroClasses/`, que carga `HeroClassManager` por el `classId` del héroe. Hay un `Spear` más viejo en `Assets/ScriptableObjects/Hero/Class/`.
- **Qué lleva cada una:** atributos base (fuerza, destreza, armadura y vitalidad), topes por atributo (todos sin límite en los assets), los coeficientes que pasan atributos a vida, aguante, daño, defensa y penetración (`HeroStatFormula`), resiliencia, cómo golpea (`hitTraits`), cuatro habilidades (Q, E, R y la definitiva) y dos perks.
- **Atributos base de los assets** (F/D/A/V): espada y escudo 3/1/2/4, mandoble 3/4/2/3, lanza 0/0/0/0 (la vieja, 2/4/2/2) y arco 1/5/0/2. El GDD trae dos tablas más: la de §5.2 (4/2/4/3, 5/3/2/3, 3/4/2/3 y 2/5/1/2) y la de §5.4 (igual que los assets salvo el mandoble, 4/2/1/2).
- **Equipo inicial** (`HeroClassSelector.FillBaseItemIdsForClass`): el arma básica de la clase y torso, guantes, pantalones y botas: ligera el arco, media la lanza y el mandoble, pesada espada y escudo. Sin casco.
- **La clase se elige al crear el héroe y se guarda** (`HeroData.classId`), aunque el GDD dice que la define el arma equipada.
- **Una sola arma por héroe**, sin cambiarla en batalla. El escudo va con la espada en la misma pieza (`WeaS&SBasic`).
- **Compatibilidad** (`EquipmentManagerService`): arco con ligera; lanza y mandoble con ligera o media; espada y escudo con cualquiera.

### 1.2 Motor

- `crear_heroe` ignora `class_definition_id`, `genero` y `avatar` (Comandos_Parametros). El héroe se proyecta con la clase `default`.
- El héroe nace con los cuatro atributos a 0. El tope es uno para todos, `HEROE.topeAtributo` (100). `repartir_puntos` suma, y nada da puntos todavía (Heroe_Y_Tropa §4).
- El contrato ya trae `class_definition_id` en `OrdenCrearHeroe`, en `Heroe`, en el héroe público y en `HeroSnapshot`.

## 2. Decisiones (autor, 2026-11-11)

1. **Cinco clases**, cada una con un héroe como inspiración para tomar las decisiones (§3). La inspiración no es el nombre: **la clase se llama por el arma que lleva**.
2. **El arma define la clase**, como en Conquest. La clase no limita el arma: el héroe es de la clase del arma que lleva puesta.
3. **Las armas tienen tiers** (cobre, bronce, hierro temprano, hierro) que se abren con la tecnología. Una espada y escudo de cobre y una de hierro son la misma clase.
4. **Cada clase admite unos pesos de armadura**, y se empieza con el más ligero de ellos.
5. **El escudo va con el arma**, en la misma pieza y el mismo hueco, como en Conquest.
6. **El arco y lanza cambia de arma en batalla**, y eso lo resuelve Conquest.
7. **El hueco del arma nunca queda vacío**: el arma solo se cambia por otra.
8. **Atributos = base de la clase del arma + puntos repartidos.** Todas las clases suman 10 de base (§3.1).
9. **Sin topes por clase** por ahora: sigue el tope global.
10. **Equipo inicial según la clase**: el arma de cobre y la armadura más ligera que admite.
11. **El Liderazgo es el mismo para todas las clases.**
12. **Reparto:** el motor tiene la ficha de cada clase; Conquest, lo que hace en combate.
13. **Ids del motor**; Conquest se adapta a ellos.
14. **`crear_heroe` rechaza** una clase vacía o desconocida.
15. **La clase no cuenta en las batallas con números**, como el equipo.
16. **Habilidades y perks**, cuatro habilidades y dos perks por clase (§3.1).
17. **La tecnología del tier solo limita comprar y forjar**: un arma que ya se tiene se puede llevar siempre.
18. **Las clases tienen su propio bloque y su propia versión**, aparte del catálogo de objetos, que cambia mucho más a menudo.
19. **Las clases no dan nada en el mapa del mundo**: solo cuentan dentro de las batallas de Unity.

## 3. Las cinco clases

| id | Nombre (el arma) | Inspiración | Papel | Armaduras | Empieza con | Plantilla en Conquest |
| --- | --- | --- | --- | --- | --- | --- |
| `lanza_escudo_redondo` | Lanza y escudo grande redondo | Leónidas | aguantar la línea, anticarga | pesada | pesada | `Spear` más el bloqueo de `SwordAndShield` (no hay lanza con escudo) |
| `espada_escudo_mediano` | Espada corta y escudo mediano | Aquiles | duelista de primera línea | pesada, media | media | `SwordAndShield`, casi directa |
| `arco_lanza` | Arco compuesto y lanza | Ulises | distancia, con lanza para el cuerpo a cuerpo | ligera, media | ligera | `Bow`, más el cambio a la lanza (nuevo) |
| `hacha_dos_manos` | Hacha a dos manos | Ayax | fuerza bruta, rompe líneas | media | media | `TwoHandedSword` (el mandoble pasa a hacha) |
| `dagas` | Dagas | el espía y asesino | rápido, golpes por la espalda | ligera | ligera | ninguna: todo nuevo |

Los ids son los que usa el catálogo de objetos. Los nombres que ve el jugador son los del arma; la inspiración solo sirve para diseñar y, si acaso, para la descripción.

### 3.1 Clase a clase

Se cierran una a una con el autor. **Todas las clases suman 10 puntos de atributos base** (en Conquest suman entre 8 y 12), para que ninguna empiece más fuerte que otra. Los nombres de las habilidades y los perks son los del juego; entre paréntesis, la plantilla de Conquest de la que salen. Lo que hacen en combate (cifras, alcances, tiempos) lo fija Conquest.

#### Lanza y escudo grande redondo (decidido el 2026-11-11)

- **Atributos** (fuerza/destreza/armadura/vitalidad): 3/1/3/3.
- **Cómo pelea:** lento, con un bloqueo frontal grande por el escudo, alcance de lanza, daño perforante y mucha resiliencia.
- **Q, Golpe de escudo:** empuja al de delante y le rompe la formación (Shield Bash).
- **E, Estocada sobre el escudo:** golpe perforante largo que atraviesa escudos (Precise Thrust).
- **R, Escudos trabados:** más defensa para él y su escuadra mientras no se mueven (Reinforced Defense).
- **Definitiva, Nadie pasa:** él y su escuadra no retroceden, ganan defensa, y quien les carga se frena y recibe daño (Unbreakable Wall y Spike Wall).
- **Perk pasivo, Línea inquebrantable:** quieto 2 segundos, su escuadra recibe menos daño (Unbreakable Line).
- **Perk activo, Orden de aguantar:** los aliados cercanos mitigan más daño durante 6 segundos (Endurance Order).

#### Espada corta y escudo mediano (decidido el 2026-11-11)

- **Atributos:** 3/3/2/2.
- **Cómo pelea:** duelista que va a por el enemigo; más rápido que la lanza, golpes cortantes y estocadas, bloqueo y resiliencia medios.
- **Q, Acometida:** salto corto hacia un enemigo y golpe de escudo que lo desequilibra (Interception).
- **E, Tres tajos:** combo de tres golpes con daño creciente (Steel Rain).
- **R, Parada y respuesta:** alza el escudo un instante y, si para un golpe, contraataca con daño extra (nueva).
- **Definitiva, Furia del campeón:** unos segundos pega más y más rápido, y su escuadra gana ataque (Judgment of Steel).
- **Perk pasivo, Primero en la brecha:** el primer golpe tras moverse hace más daño y frena al enemigo (Destabilizing Thrust).
- **Perk activo, Carga conjunta:** si carga con su escuadra, todos hacen más daño (Coordinated Charge).

#### Arco compuesto y lanza (decidido el 2026-11-11)

- **Atributos:** 2/5/1/2.
- **Cómo pelea:** rápido, daño perforante a distancia que baja con los metros; con la lanza se defiende, pero no es de primera línea. El cambio entre arco y lanza lo resuelve Conquest.
- **Q, Disparo certero:** flecha que atraviesa armadura y hace mucho daño a los pesados (Focused Shot).
- **E, Lluvia de flechas:** andanada sobre una zona que desordena las formaciones (Arrow Rain).
- **R, Lanzada de retirada:** con la lanza, golpe que empuja al enemigo y le deja saltar atrás para volver al arco (nueva).
- **Definitiva, Flechas de fuego:** una zona ardiendo que hace daño con el tiempo (Flaming Arrow).
- **Perk pasivo, Tirador apostado:** quieto 3 segundos, sus flechas frenan al enemigo (Harassment Cover).
- **Perk activo, Ardid:** una flecha que impide al objetivo usar habilidades un rato (Silencing Arrow, que en Conquest es habilidad).

#### Hacha a dos manos (decidido el 2026-11-11)

- **Atributos:** 5/1/1/3.
- **Cómo pelea:** golpes lentos y muy duros, cortantes y contundentes; parte del daño pasa el bloqueo de los escudos y aturde bastante; sin escudo, resiliencia media.
- **Q, Molinete:** golpe en círculo que abre hueco alrededor (Whirlwind Slash).
- **E, Embestida brutal:** atraviesa enemigos y les desordena la línea (Brutal Charge).
- **R, Parte escudos:** hachazo de arriba abajo que rompe el bloqueo y aturde (nueva).
- **Definitiva, Rompelíneas:** carga en línea recta derribando a quien encuentra, y su escuadra le sigue más rápido (nueva).
- **Perk pasivo, Mole:** mientras ataca no se le puede derribar ni hacer retroceder (nuevo).
- **Perk activo, Zona prohibida:** marca el suelo; los enemigos que lo cruzan van más lentos y defienden peor (Exclusion Zone).

#### Dagas (decidido el 2026-11-11)

- **Atributos:** 3/5/0/2.
- **Cómo pelea:** la más rápida; golpes cortos y seguidos, cortantes y perforantes, con mucho más daño por la espalda o el flanco; no bloquea, esquiva; resiliencia baja.
- **Q, Golpe por la espalda:** mucho daño si está detrás del objetivo, y lo deja sangrando (nueva).
- **E, Finta:** salto corto en cualquier dirección y un instante en el que cuesta acertarle (nueva).
- **R, Sigilo:** se vuelve invisible durante un tiempo (nueva; Conquest necesita poder ocultar a un héroe).
- **Definitiva, Danza de dagas:** una ráfaga de golpes muy rápidos sobre un objetivo (nueva).
- **Perk pasivo, Hoja sucia:** sus golpes hacen sangrar, y el sangrado se acumula (nuevo).
- **Perk activo, Marca del cazador:** marca a un enemigo, y él y su escuadra le hacen más daño (la idea de Marking Arrow).

### 3.2 Resumen

| Clase | F | D | A | V | Q | E | R | Definitiva | Perk pasivo | Perk activo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Lanza y escudo grande redondo | 3 | 1 | 3 | 3 | Golpe de escudo | Estocada sobre el escudo | Escudos trabados | Nadie pasa | Línea inquebrantable | Orden de aguantar |
| Espada corta y escudo mediano | 3 | 3 | 2 | 2 | Acometida | Tres tajos | Parada y respuesta | Furia del campeón | Primero en la brecha | Carga conjunta |
| Arco compuesto y lanza | 2 | 5 | 1 | 2 | Disparo certero | Lluvia de flechas | Lanzada de retirada | Flechas de fuego | Tirador apostado | Ardid |
| Hacha a dos manos | 5 | 1 | 1 | 3 | Molinete | Embestida brutal | Parte escudos | Rompelíneas | Mole | Zona prohibida |
| Dagas | 3 | 5 | 0 | 2 | Golpe por la espalda | Finta | Sigilo | Danza de dagas | Hoja sucia | Marca del cazador |

## 4. El arma define la clase

- **El héroe no guarda la clase.** Se saca del arma que lleva puesta: cada arma del catálogo `OBJETOS` lleva su `clase` y su `tier`. `claseDe(héroe) = OBJETOS[héroe.equipamiento.arma].clase`.
- **El hueco del arma nunca queda vacío.** `desequipar` el arma se rechaza; `equipar` otra arma la cambia y la vieja va a la mochila (si no cabe, se rechaza: mochila llena).
- **Cambiar de arma es cambiar de clase**: de atributos base (§5), de armaduras que admite, de habilidades y de perks. Se puede donde se pueda equipar (duda 5 de `Objetos_Del_Heroe.md` §4, todavía abierta).
- **Armadura incompatible.** Al equipar un arma cuya clase no admite alguna pieza puesta, esa pieza pasa a la mochila; si no cabe, la orden se rechaza. Al equipar una armadura, se rechaza si la clase del arma no admite su peso. Es la regla de `Objetos_Del_Heroe.md` §3.1, punto 5.
- **Tiers.** Cada tier lo abre una tecnología de la Facción: cobre `metalurgia_cobre`, bronce `aleacion_bronce`, hierro temprano `forja_hierro_temprana` y hierro `forja_hierro_estandarizada`. La tecnología solo decide qué se puede **comprar o forjar** (tienda y forja); un arma que ya se tiene se puede llevar siempre.
- **El ticket** manda en `class_definition_id` la clase del arma. Conquest rechaza un ticket con una clase que no conozca, como ya hace con `tropaId`.
- **Proyección.** `Heroe.class_definition_id` y el del héroe público salen del arma. No hace falta campo nuevo.

## 5. Atributos

```
atributos(héroe) = CLASES_HEROE[claseDe(héroe)].atributosBase + héroe.puntosRepartidos
```

- El motor guarda solo los **puntos repartidos**. La base la pone la clase del arma de ese momento, así que cambiar de arma cambia los atributos sin perder los puntos.
- `repartir_puntos` comprueba el tope global con la suma (base + puntos).
- En el contrato, `atributos_base` del héroe y del ticket sigue llevando la suma, y Conquest no cambia. Si hace falta, se añade aparte lo repartido.
- **Valores por clase:** los de §3.2, todos con 10 puntos. Conquest tenía (F/D/A/V) espada y escudo 3/1/2/4, mandoble 3/4/2/3, lanza 2/4/2/2 (la vieja; la de `Resources` está a 0) y arco 1/5/0/2.

## 6. Crear el héroe

- `class_definition_id` de `crear_heroe` es el id de una clase (§3). Vacío o desconocido → `heroe.clase_desconocida` (nuevo en el catálogo 07).
- **Equipo inicial**: el arma de tier cobre de esa clase y la armadura del peso con el que empieza (§3). Las piezas concretas (si lleva casco o no) las fija el catálogo de objetos. Van puestas, no en la mochila.
- Los bots y el AdminClient mandan la clase como un jugador más; BotsBrain la reparte entre sus héroes.
- El cliente jugador pone un selector de las cinco clases al crear el héroe, con el nombre del arma.

## 7. El bloque `CLASES_HEROE` del motor

Forma propuesta, para Mechanics cuando se implemente. Va en `GET /v1/datos` junto a `OBJETOS`, con **su propia versión**: las clases cambian poco una vez definidas y el catálogo de objetos cambia a menudo, así que un precio nuevo no toca las clases ni al revés. Las armas de `OBJETOS` nombran su clase por id, y el motor comprueba al cargar que todas existen en `CLASES_HEROE`.

```json
{
  "version": "bronzeage-clases-1",
  "clases": [
    { "id": "lanza_escudo_redondo",  "nombre": "Lanza y escudo grande redondo", "armaduras": ["pesada"],          "armaduraInicial": "pesada", "atributosBase": { "fuerza": 3, "destreza": 1, "armadura": 3, "vitalidad": 3 } },
    { "id": "espada_escudo_mediano", "nombre": "Espada corta y escudo mediano", "armaduras": ["pesada", "media"], "armaduraInicial": "media",  "atributosBase": { "fuerza": 3, "destreza": 3, "armadura": 2, "vitalidad": 2 } },
    { "id": "arco_lanza",            "nombre": "Arco compuesto y lanza",        "armaduras": ["ligera", "media"], "armaduraInicial": "ligera", "atributosBase": { "fuerza": 2, "destreza": 5, "armadura": 1, "vitalidad": 2 } },
    { "id": "hacha_dos_manos",       "nombre": "Hacha a dos manos",             "armaduras": ["media"],           "armaduraInicial": "media",  "atributosBase": { "fuerza": 5, "destreza": 1, "armadura": 1, "vitalidad": 3 } },
    { "id": "dagas",                 "nombre": "Dagas",                         "armaduras": ["ligera"],          "armaduraInicial": "ligera", "atributosBase": { "fuerza": 3, "destreza": 5, "armadura": 0, "vitalidad": 2 } }
  ]
}
```

El arma inicial no se escribe aquí: es la de tier `cobre` de la clase en `OBJETOS`. La compatibilidad con la armadura (`armaduras`) vive aquí y no en `OBJETOS`.

## 8. Lo que se queda en Conquest

Todo lo que pasa dentro de la batalla, por clase: los coeficientes de `HeroStatFormula`, la resiliencia, los `hitTraits`, la velocidad, las cuatro habilidades, los perks y sus efectos, las animaciones y los modelos. Conquest carga la clase por el id del motor (renombra sus `HeroClassDefinition` o les pone el id) y pasa de cuatro clases a cinco:

| Clase | Trabajo en Conquest |
| --- | --- |
| `lanza_escudo_redondo` | nueva a partir de `Spear`, con escudo y bloqueo |
| `espada_escudo_mediano` | `SwordAndShield` renombrada |
| `arco_lanza` | `Bow` más el cambio a la lanza en batalla (Conquest no cambia de arma hoy) |
| `hacha_dos_manos` | `TwoHandedSword` con hacha en lugar de mandoble |
| `dagas` | nueva del todo: sin plantilla, habilidades ni animaciones |

**Habilidades y perks:** los de §3.1. Nuevas en Conquest: Parada y respuesta, Lanzada de retirada, Parte escudos, Rompelíneas, Mole y todas las de las dagas (Sigilo pide poder ocultar a un héroe). Las demás salen de las suyas con otro nombre. Los perks desbloqueados se guardan en el héroe (`perksDesbloqueados`); al cambiar de arma se conservan y solo se usan los de la clase del arma puesta.

### 8.1 Lo que necesita Conquest (revisado en su código el 2026-11-11)

Leído en `VRhino/conquest_prototype` (`Docs/Mechanics/SkillsAndPerks.md`, `DamageMechanics.md`, `Docs/Por_Abordar/Habilidades_Perks.md` y el código de `Hero/`, `Skills/`, `Combat/` y `Visual/`).

**Común a las cinco**

1. **De cuatro clases a cinco con los ids del motor.** El enum `HeroClass` y los `HeroClassDefinition` de `Resources/Data/HeroClasses`, y los sitios que lo miran a mano: `HeroClassSelector` (equipo inicial por clase), `HeroHorseVisual` y `HeroMountPresenter` (jinete por clase), `SkillIconCatalog` y el panel de perks. Rechazar un ticket con una clase que no conozca.
2. **La clase sale del arma del ticket**, no de `HeroData.classId`, y el héroe se monta desde el `HeroSnapshot` (hoy solo lee id, bando y si es bot; depende del paso a red).
3. **Los atributos base vienen del motor** ya sumados. El `HeroClassDefinition` se queda con los coeficientes, la resiliencia, los `hitTraits` y la velocidad.
4. **Arte:** las cinco armas en cuatro tiers y los dos escudos (redondo grande y mediano), y las armaduras del catálogo. Hoy todas las habilidades usan la misma animación de ataque provisional, así que cada clase necesita las suyas.
5. **IA de héroes** (`HeroAISkillSystem`): que sepa usar las habilidades nuevas, cuándo cambiar del arco a la lanza y cómo ir por la espalda con las dagas.
6. **El fichero de stats y descripciones** para los tooltips del cliente jugador (`Objetos_Del_Heroe.md` §2, punto 9).

**Mecánicas que no existen** (en su lista de pendientes como H4 las tres primeras)

| Mecánica | La piden |
| --- | --- |
| Silencio: impedir habilidades un tiempo (hoy la Flecha Silenciadora solo ralentiza) | Ardid |
| Zonas que duran en el suelo | Flechas de fuego, Zona prohibida, Nadie pasa |
| Daño por contacto | Nadie pasa (quien carga recibe daño) |
| Cambio de arma en batalla | Arco compuesto y lanza, Lanzada de retirada |
| Invisibilidad: que la IA no lo apunte y que el servidor no lo enseñe a los rivales | Sigilo |
| Sangrado al golpear (el daño en el tiempo solo existe en habilidades de área) | Hoja sucia, Golpe por la espalda |
| Contraataque al bloquear | Parada y respuesta |
| Un instante en el que no se le puede acertar | Finta |
| Inmunidad temporal a retroceso y derribo (hoy solo hay un permiso fijo por clase) | Mole, Nadie pasa |

**Lo que ya tiene y basta con datos:** embestida (`Dash`), golpes en área y en cono, modificadores temporales (defensa, daño, velocidad de ataque y de movimiento), condición «quieto N segundos», bonos por flanco y espalda (`ApplyConditional` con direcciones), derribo como reacción de una habilidad, golpes que pasan el escudo (`blockDamageRatio`, `ignoresShield`), el bloqueo del héroe con escudo y los perks Línea inquebrantable, Orden de resistencia, Carga coordinada, Estocada desestabilizadora, Cobertura de hostigamiento y Flecha marcadora.

**Por clase**

| Clase | Ya existe | Nuevo |
| --- | --- | --- |
| Lanza y escudo grande redondo | Golpe de escudo, Estocada sobre el escudo, Escudos trabados, los dos perks | escudo en una clase de lanza; Nadie pasa (zona, contacto, inmunidad) |
| Espada corta y escudo mediano | Acometida, Tres tajos, Furia del campeón, los dos perks | Parada y respuesta |
| Arco compuesto y lanza | Disparo certero, Lluvia de flechas, Tirador apostado | cambio de arma, Lanzada de retirada, zona de Flechas de fuego, silencio de Ardid |
| Hacha a dos manos | Molinete, Embestida brutal, Parte escudos (derribo y rasgos del arma), Rompelíneas (embestida, derribo y velocidad a la escuadra) | Mole (inmunidad), zona de Zona prohibida |
| Dagas | Golpe por la espalda (bono de espalda), Danza de dagas (golpes en cono), Marca del cazador (como Flecha marcadora, a un solo objetivo) | toda la clase y sus animaciones; Sigilo, Finta, el sangrado de Hoja sucia |

## 9. Lo que falta

### 9.1 Motor

1. Bloque `CLASES_HEROE` en Mechanics y en `GET /v1/datos` (§7).
2. `clase` y `tier` en cada arma de `OBJETOS`, y la tecnología de cada tier (catálogo de objetos).
3. El héroe guarda los puntos repartidos en lugar de los atributos (foto nueva, partida nueva).
4. `crear_heroe` con clase y equipo inicial; rechazo `heroe.clase_desconocida`.
5. Equipar y desequipar con las reglas de §4: el arma no se quita, la armadura incompatible va a la mochila.
6. Proyección y ticket con la clase del arma, los atributos sumados y la versión de `CLASES_HEROE` junto a la del catálogo de objetos.
7. `repartir_puntos` con el tope sobre la suma.
8. Docs: Heroe_Y_Tropa §2.1, §3 y §4, Comandos_Parametros, Proyecciones, Batalla_Unity_Ciclo, 07, canon 5.16 y `Pendientes_Cliente_Jugador.md`. Versiones de `militar`, `nucleo` y `contrato`.

### 9.2 Cliente jugador (BronzeAgeClient)

- Selector de clase al crear el héroe, con el nombre del arma y su descripción.
- La clase en la ficha del héroe y en la de los demás, sacada del arma.
- Aviso al cambiar de arma: cambia la clase y se quita la armadura que no admite.

### 9.3 Conquest

- Las cinco clases con los ids del motor (§8), y rechazar un ticket con una clase desconocida.
- Montar el héroe con la clase y los atributos del ticket (hoy solo lee id, bando y si es bot).
- El cambio de arma del arco y lanza.
- La clase de las dagas, desde cero, con la invisibilidad del Sigilo.
- Las habilidades y los perks de §3.1, con sus nombres.

### 9.4 Bots y administración

- **BotsBrain:** elegir la clase al crear cada héroe.
- **AdminClient:** crear héroes con clase y ver la clase en su ficha.

## 10. Por decidir

1. Lo que da cada arma en combate por tier (catálogo de objetos y Conquest).
2. Si el equipo inicial lleva casco (catálogo de objetos).
3. Dónde se puede cambiar de arma, que ahora es cambiar de clase (`Objetos_Del_Heroe.md` §4, duda 5).
