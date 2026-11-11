# Clases del héroe: el arma define la clase

Diseño cerrado con el autor el 2026-11-11 (hilo «Diseñar clases de héroe»), sin implementar. Entrada 71 de `Docs/Mecanicas a desarrollar.md`. Va con [`Objetos_Del_Heroe.md`](Objetos_Del_Heroe.md): sustituye su punto 2 de §3.1 («guardar la clase») y contesta su duda 6 de §4 («si la clase limita el arma»). Las armas y armaduras concretas están en el diseño del catálogo de objetos (`Catalogo_De_Objetos.md`, en su propio hilo).

Quedan **por cerrar héroe a héroe**, como pidió el autor: los atributos base de cada clase (§5) y sus habilidades y perks (§8).

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
8. **Atributos = base de la clase del arma + puntos repartidos.** Los valores, héroe a héroe.
9. **Sin topes por clase** por ahora: sigue el tope global.
10. **Equipo inicial según la clase**: el arma de cobre y la armadura más ligera que admite.
11. **El Liderazgo es el mismo para todas las clases.**
12. **Reparto:** el motor tiene la ficha de cada clase; Conquest, lo que hace en combate.
13. **Ids del motor**; Conquest se adapta a ellos.
14. **`crear_heroe` rechaza** una clase vacía o desconocida.
15. **La clase no cuenta en las batallas con números**, como el equipo.
16. **Habilidades y perks**, héroe a héroe.

## 3. Las cinco clases

| id | Nombre (el arma) | Inspiración | Papel | Armaduras | Empieza con | Plantilla en Conquest |
| --- | --- | --- | --- | --- | --- | --- |
| `lanza_escudo_redondo` | Lanza y escudo grande redondo | Leónidas | aguantar la línea, anticarga | pesada | pesada | `Spear` más el bloqueo de `SwordAndShield` (no hay lanza con escudo) |
| `espada_escudo_mediano` | Espada corta y escudo mediano | Aquiles | duelista de primera línea | pesada, media | media | `SwordAndShield`, casi directa |
| `arco_lanza` | Arco compuesto y lanza | Ulises | distancia, con lanza para el cuerpo a cuerpo | ligera, media | ligera | `Bow`, más el cambio a la lanza (nuevo) |
| `hacha_dos_manos` | Hacha a dos manos | Ayax | fuerza bruta, rompe líneas | media | media | `TwoHandedSword` (el mandoble pasa a hacha) |
| `dagas` | Dagas | el espía y asesino | rápido, golpes por la espalda | ligera | ligera | ninguna: todo nuevo |

Los ids son los que usa el catálogo de objetos. Los nombres que ve el jugador son los del arma; la inspiración solo sirve para diseñar y, si acaso, para la descripción.

## 4. El arma define la clase

- **El héroe no guarda la clase.** Se saca del arma que lleva puesta: cada arma del catálogo `OBJETOS` lleva su `clase` y su `tier`. `claseDe(héroe) = OBJETOS[héroe.equipamiento.arma].clase`.
- **El hueco del arma nunca queda vacío.** `desequipar` el arma se rechaza; `equipar` otra arma la cambia y la vieja va a la mochila (si no cabe, se rechaza: mochila llena).
- **Cambiar de arma es cambiar de clase**: de atributos base (§5), de armaduras que admite, de habilidades y de perks. Se puede donde se pueda equipar (duda 5 de `Objetos_Del_Heroe.md` §4, todavía abierta).
- **Armadura incompatible.** Al equipar un arma cuya clase no admite alguna pieza puesta, esa pieza pasa a la mochila; si no cabe, la orden se rechaza. Al equipar una armadura, se rechaza si la clase del arma no admite su peso. Es la regla de `Objetos_Del_Heroe.md` §3.1, punto 5.
- **Tiers.** Cada tier lo abre una tecnología de la Facción: cobre `metalurgia_cobre`, bronce `aleacion_bronce`, hierro temprano `forja_hierro_temprana` y hierro `forja_hierro_estandarizada`. La tecnología decide qué se puede **comprar o forjar** (tienda y forja); un arma que ya se tiene se puede llevar siempre.
- **El ticket** manda en `class_definition_id` la clase del arma. Conquest rechaza un ticket con una clase que no conozca, como ya hace con `tropaId`.
- **Proyección.** `Heroe.class_definition_id` y el del héroe público salen del arma. No hace falta campo nuevo.

## 5. Atributos

```
atributos(héroe) = CLASES_HEROE[claseDe(héroe)].atributosBase + héroe.puntosRepartidos
```

- El motor guarda solo los **puntos repartidos**. La base la pone la clase del arma de ese momento, así que cambiar de arma cambia los atributos sin perder los puntos.
- `repartir_puntos` comprueba el tope global con la suma (base + puntos).
- En el contrato, `atributos_base` del héroe y del ticket sigue llevando la suma, y Conquest no cambia. Si hace falta, se añade aparte lo repartido.
- **Valores por clase: por decidir héroe a héroe.** Referencia de Conquest (assets de hoy, F/D/A/V): espada y escudo 3/1/2/4, mandoble 3/4/2/3, lanza 2/4/2/2 (la vieja; la de `Resources` está a 0) y arco 1/5/0/2. Las dagas no tienen referencia.

## 6. Crear el héroe

- `class_definition_id` de `crear_heroe` es el id de una clase (§3). Vacío o desconocido → `heroe.clase_desconocida` (nuevo en el catálogo 07).
- **Equipo inicial**: el arma de tier cobre de esa clase y la armadura del peso con el que empieza (§3). Las piezas concretas (si lleva casco o no) las fija el catálogo de objetos. Van puestas, no en la mochila.
- Los bots y el AdminClient mandan la clase como un jugador más; BotsBrain la reparte entre sus héroes.
- El cliente jugador pone un selector de las cinco clases al crear el héroe, con el nombre del arma.

## 7. El bloque `CLASES_HEROE` del motor

Forma propuesta, para Mechanics cuando se implemente (los atributos, a 0 hasta cerrarlos héroe a héroe). Va en `GET /v1/datos` junto a `OBJETOS` y con la misma versión, porque la clase sale del arma: un cambio de clases es un cambio del catálogo de objetos.

```json
{
  "clases": [
    { "id": "lanza_escudo_redondo",  "nombre": "Lanza y escudo grande redondo", "armaduras": ["pesada"],          "armaduraInicial": "pesada", "atributosBase": { "fuerza": 0, "destreza": 0, "armadura": 0, "vitalidad": 0 } },
    { "id": "espada_escudo_mediano", "nombre": "Espada corta y escudo mediano", "armaduras": ["pesada", "media"], "armaduraInicial": "media",  "atributosBase": { "fuerza": 0, "destreza": 0, "armadura": 0, "vitalidad": 0 } },
    { "id": "arco_lanza",            "nombre": "Arco compuesto y lanza",        "armaduras": ["ligera", "media"], "armaduraInicial": "ligera", "atributosBase": { "fuerza": 0, "destreza": 0, "armadura": 0, "vitalidad": 0 } },
    { "id": "hacha_dos_manos",       "nombre": "Hacha a dos manos",             "armaduras": ["media"],           "armaduraInicial": "media",  "atributosBase": { "fuerza": 0, "destreza": 0, "armadura": 0, "vitalidad": 0 } },
    { "id": "dagas",                 "nombre": "Dagas",                         "armaduras": ["ligera"],          "armaduraInicial": "ligera", "atributosBase": { "fuerza": 0, "destreza": 0, "armadura": 0, "vitalidad": 0 } }
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

**Habilidades y perks: por decidir héroe a héroe**, renombrados al tema. Los perks desbloqueados se guardan en el héroe (`perksDesbloqueados`); al cambiar de arma se conservan y solo se usan los de la clase del arma puesta.

## 9. Lo que falta

### 9.1 Motor

1. Bloque `CLASES_HEROE` en Mechanics y en `GET /v1/datos` (§7).
2. `clase` y `tier` en cada arma de `OBJETOS`, y la tecnología de cada tier (catálogo de objetos).
3. El héroe guarda los puntos repartidos en lugar de los atributos (foto nueva, partida nueva).
4. `crear_heroe` con clase y equipo inicial; rechazo `heroe.clase_desconocida`.
5. Equipar y desequipar con las reglas de §4: el arma no se quita, la armadura incompatible va a la mochila.
6. Proyección y ticket con la clase del arma y los atributos sumados.
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
- La clase de las dagas, desde cero.

### 9.4 Bots y administración

- **BotsBrain:** elegir la clase al crear cada héroe.
- **AdminClient:** crear héroes con clase y ver la clase en su ficha.

## 10. Por decidir

1. Los atributos base de cada clase (héroe a héroe).
2. Las habilidades y los perks de cada clase (héroe a héroe).
3. Lo que da cada arma en combate por tier (catálogo de objetos y Conquest).
4. Si el equipo inicial lleva casco (catálogo de objetos).
5. Dónde se puede cambiar de arma, que ahora es cambiar de clase (`Objetos_Del_Heroe.md` §4, duda 5).
