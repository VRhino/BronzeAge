# Catálogo inicial de objetos del héroe

Propuesta del 2026-11-11, **pendiente del sí del autor** y sin implementar. Cambiada el mismo día con las cinco clases del autor, cuyo diseño (`Clases_De_Heroe.md`, cerrado) es la base de §3.2, §4 y §5. Cierra el punto 1 (y en parte el 2, el 8 y el 10) de §4 de [`Objetos_Del_Heroe.md`](Objetos_Del_Heroe.md). Parte de las plantillas de Conquest (`Assets/Resources/Items`, `HeroClassSelector.FillBaseItemIdsForClass`, `BasicArmorStatGenerator`, `BasicWeaponStatGenerator`, `ItemPricingService`) y las lleva a la Edad del Bronce con los materiales y tecnologías que el motor ya usa en la Armería (cuero, cobre, bronce y hierro; [`Economia_Construccion.md`](../Mechanics/Economia_Construccion.md)).

Reparto (de `Objetos_Del_Heroe.md` §2): la **ficha** de cada objeto (§3 y §6) es del motor y va en `OBJETOS` de `GET /v1/datos`; lo que hace **en combate** (§4) es de Conquest, que lo carga con el mismo id. Las cifras de §4 son una propuesta para Conquest, no un bloque del motor.

## 1. De dónde se parte (Conquest)

| Conquest | Qué es |
| --- | --- |
| `TorLADef`, `GloLADef`, `PanLADef`, `BootLADef` | armadura ligera: torso, guantes, pantalones, botas |
| `TorMADef` … `BootMADef` | armadura media |
| `TorHADef` … `BootHADef` | armadura pesada |
| — | **ningún casco** (el hueco `Helmet` existe, sin piezas) |
| `WeaBBasic`, `WeaSBasic`, `WeaS2HBasic`, `WeaS&SBasic` | arco, lanza, mandoble, espada y escudo |
| `BasicWeaponChest`, `SmallBronceBag` | consumibles: cofre de arma y bolsa de bronce |

Todo es Común. Un solo generador al azar para las armaduras (defensa perforante 8-15, cortante 10-18, contundente 12-20, salud 50-100, armadura 30-60, vitalidad 30-60, según los valores por defecto del código) y otro para las armas, así que ligera, media y pesada dan lo mismo. Equipo inicial por clase: arco con ligera, lanza y mandoble con media, espada y escudo con pesada, y sin casco. Precio: base por rareza (20, 50, 100, 250, 500) × tipo (arma 1,5, armadura 1,2, consumible 0,8); venta al 50 %.

## 2. Tema: tres pesos, cuatro tiers

Armaduras y armas van por **tiers** que abre la tecnología (autor, 2026-11-11): cobre (`metalurgia_cobre`), bronce (`aleacion_bronce`), hierro temprano (`forja_hierro_temprana`) y hierro (`forja_hierro_estandarizada`). La tecnología solo limita comprar y forjar.

Cada peso de armadura tiene su hechura, y el tier pone el metal:

| Peso | Hechura | Nombre, por ejemplo |
| --- | --- | --- |
| ligera | cuero con tachones del metal del tier | «Jubón de cuero con tachones de bronce» |
| media | cuero curtido con escamas del metal | «Coraza de escamas de hierro temprano» |
| pesada | placas del metal | «Coraza de bronce», «Grebas de hierro» |

Piezas de cada peso: ligera gorro, jubón, brazales, faldellín y sandalias; media casco con carrilleras, coraza de escamas, brazales tachonados, faldellín de tiras y botas; pesada casco, coraza, brazales, faldar y grebas.

**El tier cobre no tiene casco** (autor, 2026-11-11): es el equipo inicial, de cuatro piezas como en Conquest. Desde el bronce los juegos traen las cinco.

Las rarezas se quedan las cinco de Conquest (`comun`, `pocoComun`, `rara`, `epica`, `legendaria`). El catálogo inicial es todo `comun`; las demás quedan para la forja y el botín.

## 3. Las piezas (ficha del motor)

Ids en `snake_case`, como las tropas. `hueco` es el de `Equipamiento`; ninguna pieza se apila.

### 3.1 Armaduras: tres pesos por cuatro tiers (57)

Ids: `{hueco}_{peso}_{tier}`, por ejemplo `torso_pesada_bronce` o `botas_ligera_hierro_temprano`. Tres pesos por cuatro tiers por cinco huecos son 60, menos los tres cascos de cobre: **57 piezas**. Las doce de cobre sustituyen a las doce de Conquest (`TorLADef` → `torso_ligera_cobre`, `BootHADef` → `botas_pesada_cobre`, etc.).

### 3.2 Armas: cinco clases por cuatro tiers

Cambio del autor (2026-11-11, hilo de clases de héroe): hay **cinco clases** y **el arma define la clase**, como en Conquest. La clase no limita el arma: el héroe es de la clase del arma que lleva puesta. Cada arma tiene **tiers** que se abren con la tecnología:

| Tier | Tecnología que lo abre |
| --- | --- |
| cobre | `metalurgia_cobre` |
| bronce | `aleacion_bronce` |
| hierro temprano | `forja_hierro_temprana` |
| hierro | `forja_hierro_estandarizada` |

La clase se llama como el arma que lleva (los héroes de la Antigüedad solo sirvieron de inspiración al autor):

| Clase = arma (un solo objeto, en el hueco `arma`) | id de la clase | Armaduras |
| --- | --- | --- |
| lanza y escudo grande redondo | `lanza_escudo_redondo` | pesada |
| espada corta y escudo mediano | `espada_escudo_mediano` | pesada, media |
| arco compuesto y lanza | `arco_lanza` | ligera, media |
| hacha a dos manos | `hacha_dos_manos` | media |
| dagas (espía, asesino) | `dagas` | ligera |

Las clases, sus armaduras, sus atributos base y sus habilidades están cerradas en `Clases_De_Heroe.md` (rama `claude/project-thread-492isp`); este catálogo pone las armas.

Ids: `{clase}_{tier}`. Nombre: el de la clase con el material del tier. Cinco clases por cuatro tiers, **veinte armas**:

| Clase | cobre | bronce | hierro temprano | hierro |
| --- | --- | --- | --- | --- |
| lanza y escudo grande redondo | `lanza_escudo_redondo_cobre` | `…_bronce` | `…_hierro_temprano` | `…_hierro` |
| espada corta y escudo mediano | `espada_escudo_mediano_cobre` | `…_bronce` | `…_hierro_temprano` | `…_hierro` |
| arco compuesto y lanza | `arco_lanza_cobre` | `…_bronce` | `…_hierro_temprano` | `…_hierro` |
| hacha a dos manos | `hacha_dos_manos_cobre` | `…_bronce` | `…_hierro_temprano` | `…_hierro` |
| dagas | `dagas_cobre` | `…_bronce` | `…_hierro_temprano` | `…_hierro` |

Por ejemplo, «Lanza de bronce y escudo grande redondo», «Hacha de hierro temprano a dos manos», «Dagas de cobre». En el arco y lanza el tier es el de la punta de las flechas y de la lanza.

### 3.3 Consumibles

Fuera del catálogo inicial (`Objetos_Del_Heroe.md` §4.10). La bolsa de bronce no hace falta: el botín ya suma monedas al héroe. El cofre de arma vuelve, si vuelve, con el botín.

## 4. Lo que da cada pieza en combate (propuesta para Conquest)

Cifras fijas, enteras (`Estadistica.valor` es entero), sin generador. Hay dos clases de cifras:

- **Atributos** (fuerza, destreza, armadura, vitalidad), que se suman a los de la clase (10 puntos de base entre los cuatro). El equipo da **mucho**, y por eso cada clase solo admite unos pesos (§4.1).
- **Defensas, daño, penetración y salud**, que Conquest decide si llegan al combate o solo salen en el tooltip (`Objetos_Del_Heroe.md` §3.4).

### 4.1 Armaduras

Cada peso da lo suyo, y por eso cada clase solo admite unos pesos (autor, 2026-11-11):

- **ligera:** fuerza y destreza; de defensas, más contra el golpe contundente que contra el filo y la punta.
- **media:** equilibrada en todo.
- **pesada:** armadura y vitalidad; mucha defensa contra perforante y cortante, menos contra contundente.

Totales de un juego entero de cinco piezas en el tier de referencia; cada tier los multiplica como a las armas: ×1 cobre, ×1,25 bronce, ×1,5 hierro temprano, ×1,75 hierro. Se reparten torso 30 %, casco y pantalones 20 %, guantes y botas 15 % (redondeado). Un juego de cobre, sin casco, da el 80 %.

| Juego entero (×1) | fuerza | destreza | armadura | vitalidad | def. perf. | def. cort. | def. cont. | salud |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ligera | 10 | 15 | 3 | 2 | 15 | 15 | 25 | 50 |
| media | 6 | 6 | 9 | 9 | 35 | 35 | 35 | 100 |
| pesada | 2 | 0 | 16 | 12 | 50 | 50 | 30 | 150 |

Cada juego da 30 puntos de atributos, tres veces la base de una clase (10), así que el equipo cuenta tanto como la clase o más. Las defensas y la salud, Conquest decide si llegan al combate o solo salen en el tooltip.

Por pieza:

**ligera** (fue/des/arm/vit · def. perf/cort/cont · salud)

| Pieza | cobre | bronce | hierro temprano | hierro |
| --- | --- | --- | --- | --- |
| torso | 3/5/1/1 · 5/5/8 · 15 | 4/6/1/1 · 6/6/9 · 19 | 5/7/1/1 · 7/7/11 · 23 | 5/8/2/1 · 8/8/13 · 26 |
| casco | — | 3/4/1/1 · 4/4/6 · 13 | 3/5/1/1 · 5/5/8 · 15 | 4/5/1/1 · 5/5/9 · 18 |
| pantalones | 2/3/1/0 · 3/3/5 · 10 | 3/4/1/1 · 4/4/6 · 13 | 3/5/1/1 · 5/5/8 · 15 | 4/5/1/1 · 5/5/9 · 18 |
| guantes | 2/2/0/0 · 2/2/4 · 8 | 2/3/1/0 · 3/3/5 · 9 | 2/3/1/0 · 3/3/6 · 11 | 3/4/1/1 · 4/4/7 · 13 |
| botas | 2/2/0/0 · 2/2/4 · 8 | 2/3/1/0 · 3/3/5 · 9 | 2/3/1/0 · 3/3/6 · 11 | 3/4/1/1 · 4/4/7 · 13 |

**media** (fue/des/arm/vit · def. perf/cort/cont · salud)

| Pieza | cobre | bronce | hierro temprano | hierro |
| --- | --- | --- | --- | --- |
| torso | 2/2/3/3 · 11/11/11 · 30 | 2/2/3/3 · 13/13/13 · 38 | 3/3/4/4 · 16/16/16 · 45 | 3/3/5/5 · 18/18/18 · 53 |
| casco | — | 2/2/2/2 · 9/9/9 · 25 | 2/2/3/3 · 11/11/11 · 30 | 2/2/3/3 · 12/12/12 · 35 |
| pantalones | 1/1/2/2 · 7/7/7 · 20 | 2/2/2/2 · 9/9/9 · 25 | 2/2/3/3 · 11/11/11 · 30 | 2/2/3/3 · 12/12/12 · 35 |
| guantes | 1/1/1/1 · 5/5/5 · 15 | 1/1/2/2 · 7/7/7 · 19 | 1/1/2/2 · 8/8/8 · 23 | 2/2/2/2 · 9/9/9 · 26 |
| botas | 1/1/1/1 · 5/5/5 · 15 | 1/1/2/2 · 7/7/7 · 19 | 1/1/2/2 · 8/8/8 · 23 | 2/2/2/2 · 9/9/9 · 26 |

**pesada** (fue/des/arm/vit · def. perf/cort/cont · salud)

| Pieza | cobre | bronce | hierro temprano | hierro |
| --- | --- | --- | --- | --- |
| torso | 1/0/5/4 · 15/15/9 · 45 | 1/0/6/5 · 19/19/11 · 56 | 1/0/7/5 · 23/23/14 · 68 | 1/0/8/6 · 26/26/16 · 79 |
| casco | — | 1/0/4/3 · 13/13/8 · 38 | 1/0/5/4 · 15/15/9 · 45 | 1/0/6/4 · 18/18/11 · 53 |
| pantalones | 0/0/3/2 · 10/10/6 · 30 | 1/0/4/3 · 13/13/8 · 38 | 1/0/5/4 · 15/15/9 · 45 | 1/0/6/4 · 18/18/11 · 53 |
| guantes | 0/0/2/2 · 8/8/5 · 23 | 0/0/3/2 · 9/9/6 · 28 | 0/0/4/3 · 11/11/7 · 34 | 1/0/4/3 · 13/13/8 · 39 |
| botas | 0/0/2/2 · 8/8/5 · 23 | 0/0/3/2 · 9/9/6 · 28 | 0/0/4/3 · 11/11/7 · 34 | 1/0/4/3 · 13/13/8 · 39 |


### 4.2 Armas (provisional: el autor quiere discutirlo)

Cada clase tiene un perfil, sacado de cómo pelea en `Clases_De_Heroe.md` §3.1, y el tier lo multiplica: daño y penetración ×1 en cobre, ×1,25 en bronce, ×1,5 en hierro temprano y ×1,75 en hierro (redondeado). El atributo que da el arma es el mismo en todos los tiers: el tier mejora el filo, no al héroe.

| Clase (tier cobre) | daño perf. | daño cort. | daño cont. | pen. perf. | pen. cort. | pen. cont. | atributo |
| --- | --- | --- | --- | --- | --- | --- | --- |
| lanza y escudo grande redondo | 14 | 0 | 4 | 4 | 0 | 0 | armadura +2 (el escudo grande) |
| espada corta y escudo mediano | 6 | 12 | 0 | 2 | 3 | 0 | armadura +1 (el escudo mediano) |
| arco compuesto y lanza | 16 | 0 | 0 | 5 | 0 | 0 | destreza +1 |
| hacha a dos manos | 0 | 16 | 8 | 0 | 4 | 4 | fuerza +1 |
| dagas | 8 | 8 | 0 | 3 | 3 | 0 | destreza +1 |

| Clase | bronce | hierro temprano | hierro |
| --- | --- | --- | --- |
| lanza y escudo redondo | 18/0/5 · 5/0/0 | 21/0/6 · 6/0/0 | 24/0/7 · 7/0/0 |
| espada y escudo mediano | 8/15/0 · 2/4/0 | 9/18/0 · 3/4/0 | 10/21/0 · 4/5/0 |
| arco y lanza | 20/0/0 · 6/0/0 | 24/0/0 · 8/0/0 | 28/0/0 · 9/0/0 |
| hacha a dos manos | 0/20/10 · 0/5/5 | 0/24/12 · 0/6/6 | 0/28/14 · 0/7/7 |
| dagas | 10/10/0 · 4/4/0 | 12/12/0 · 4/4/0 | 14/14/0 · 5/5/0 |

(daño perforante/cortante/contundente · penetración perforante/cortante/contundente). La lanza del arco y lanza la pone Conquest con el cambio de arma; aquí van las cifras del arco.

## 5. Compatibilidad y equipo inicial

La compatibilidad es la columna «Armaduras» de §3.2, pieza a pieza (vale también para el casco). Como el arma define la clase, equipar un arma cambia la clase del héroe, y el motor no tiene que guardar la clase aparte: la saca del arma.

Equipo inicial: el arma de tier cobre de la clase que elija al crear el héroe y el juego de cobre, **sin casco** (autor, 2026-11-11; los cascos llegan con los juegos de más tier), de la armadura más ligera que admita esa clase: pesada para la lanza y escudo redondo, media para la espada y escudo mediano y para el hacha, ligera para el arco y lanza y para las dagas.

El hueco del arma nunca queda vacío: `desequipar` el arma se rechaza, solo se cambia por otra. Al cambiar a un arma cuya clase no admite la armadura puesta, esa armadura pasa a la mochila, y si no cabe se rechaza el cambio.

La compatibilidad y la armadura inicial de cada clase viven en el bloque `CLASES_HEROE` del diseño de clases (`Clases_De_Heroe.md`, ya en main), servido junto a `OBJETOS` pero con **su propia versión** (`bronzeage-clases-1`), porque las clases cambian poco y el catálogo mucho (autor, 2026-11-11). En `OBJETOS` cada arma solo dice su clase, por id, y su tier; el motor comprueba al cargar que esa clase existe.

La tecnología del tier **solo limita comprar y forjar** (autor, 2026-11-11): un arma o una armadura que ya se tiene se puede llevar siempre.

## 6. Precios

El héroe nace con 1.500 monedas, gana 500 por una batalla ganada contra otra Facción y 50-100 por un campamento de bandidos. El precio de Conquest (24 una armadura, 30 un arma) no pesa nada en esa escala, así que se sube y se separa por material:

El precio sube con el tier: ×1 cobre, ×2 bronce, ×3,5 hierro temprano y ×5 hierro.

| Pieza (tier cobre) | Compra | Venta (50 %) |
| --- | --- | --- |
| armadura ligera (torso 150) | 100 | 50 |
| armadura media (torso 300) | 200 | 100 |
| armadura pesada (torso 450) | 300 | 150 |
| arma | 200 | 100 |

Un juego ligero de cobre cuesta 450 y uno pesado de bronce, con casco, 3.600. Un arma: 200, 400, 700 y 1.000 según el tier. Las rarezas, cuando lleguen, multiplican como en Conquest: ×1, ×2,5, ×5, ×12,5 y ×25.

## 7. El bloque `OBJETOS` del motor

Forma propuesta, para Mechanics cuando se implemente:

```json
{
  "version": "bronzeage-objetos-1",
  "ventaPct": 50,
  "rarezas": ["comun", "pocoComun", "rara", "epica", "legendaria"],
  "tiers": { "cobre": "metalurgia_cobre", "bronce": "aleacion_bronce", "hierro_temprano": "forja_hierro_temprana", "hierro": "forja_hierro_estandarizada" },
  "objetos": [
    { "id": "torso_pesada_bronce", "nombre": "Coraza de bronce", "hueco": "torso", "tipoArmadura": "pesada", "tier": "bronce", "rareza": "comun", "apilable": false, "precio": 900 },
    { "id": "lanza_escudo_redondo_cobre", "nombre": "Lanza de cobre y escudo redondo", "hueco": "arma", "clase": "lanza_escudo_redondo", "tier": "cobre", "rareza": "comun", "apilable": false, "precio": 200 }
  ]
}
```

`version` sustituye a `conquest-items-1` en el ticket, que lleva además la versión de `CLASES_HEROE`. La clase del ticket sale del arma puesta; la compatibilidad y el equipo inicial, de `CLASES_HEROE`.

## 8. Lo que toca en Conquest

- Renombrar los `_id` de las 12 armaduras a las de tier cobre de §3.1 y crear las 45 de los demás tiers, cascos incluidos (con su `visualPartId`).
- Pasar de cuatro clases a cinco y crear sus armas por tier (§3.2); la clase sale del arma.
- Cambiar el generador por las cifras fijas de §4.
- `HeroClassSelector.FillBaseItemIdsForClass` con los ids nuevos y el casco.
- Quitar del modo conectado la bolsa de bronce y el cofre.
