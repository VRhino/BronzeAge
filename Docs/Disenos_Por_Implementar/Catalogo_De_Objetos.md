# Catálogo inicial de objetos del héroe

Diseño **aprobado por el autor el 2026-11-11** («si me cuadra todo esto», 18:12), sin implementar. Quedan abiertos el daño de las armas y la escala del héroe frente a las tropas (§4.2, por discutir), los rangos de la forja y la tecnología `acero_templado`, que espera a las Eras IV y V. Cambiada el mismo día con las cinco clases del autor, cuyo diseño (`Clases_De_Heroe.md`, cerrado) es la base de §3.2, §4 y §5. Cierra el punto 1 (y en parte el 2, el 8 y el 10) de §4 de [`Objetos_Del_Heroe.md`](Objetos_Del_Heroe.md). Parte de las plantillas de Conquest (`Assets/Resources/Items`, `HeroClassSelector.FillBaseItemIdsForClass`, `BasicArmorStatGenerator`, `BasicWeaponStatGenerator`, `ItemPricingService`) y las lleva a la Edad del Bronce con los materiales y tecnologías que el motor ya usa en la Armería (cuero, cobre, bronce y hierro; [`Economia_Construccion.md`](../Mechanics/Economia_Construccion.md)).

Reparto (decidido por el autor el 2026-11-11, 19:30; cambia el de `Objetos_Del_Heroe.md` §2): **datos en el motor, reglas en Conquest.** La ficha de cada objeto (§3 y §6) y sus cifras de combate (§4) son del motor y van en `OBJETOS` de `GET /v1/datos`; las cifras de combate de las tropas también pasan al motor. Conquest se queda con cómo se aplican (las fases del daño, los coeficientes de cada clase, los rasgos del golpe, las animaciones y el aspecto) y lee las cifras por versión al arrancar el servidor de batalla. Así el cliente jugador saca los tooltips del motor, la forja tira sobre cifras que el motor conoce y el héroe se equilibra contra las tropas con los dos lados en el mismo sitio.

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

## 2. Tema: tres pesos, cinco tiers

Armaduras y armas van por **tiers** que abre la tecnología (autor, 2026-11-11). La tecnología solo limita comprar y forjar. Los cuatro primeros son comunes; el quinto es distinto para armas y armaduras (autor, 2026-11-11 17:43):

| Tier | Armas | Armaduras | Tecnología | Era |
| --- | --- | --- | --- | --- |
| I | cobre | cobre | `metalurgia_cobre` | I, Reinos palaciales |
| II | bronce | bronce | `aleacion_bronce` | I |
| III | hierro temprano | hierro temprano | `forja_hierro_temprana` | II, Crisis y adaptación |
| IV | hierro | hierro | `forja_hierro_estandarizada` | III, Polis e imperios |
| V | **acero** | **bronce laminado** | armas: tecnología nueva `acero_templado` de la Era V (Hegemonía macedónica), que aún no existe en el motor; armaduras: `bronce_laminado` | V / III |

- **Bronce laminado** es «el bronce bueno» de las armaduras: la tecnología que ya abre la `armaduraBronceCalidad` de las tropas. En la Antigüedad la mejor armadura siguió siendo de bronce (la coraza del hoplita) mientras el hierro iba a las armas, por eso es el tier más alto de armadura aunque su tecnología sea de la Era III, como el hierro.
- **Acero** encaja al final de la Era V: el temple del hierro se conoce en Grecia desde Homero, y el acero nórico y el indio (wootz) aparecen hacia el 300 a. C. Depende de que existan las Eras IV y V, hoy sin implementar.

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

### 3.1 Armaduras: tres pesos por cinco tiers (72)

Ids: `{hueco}_{peso}_{tier}`, por ejemplo `torso_pesada_bronce`, `botas_ligera_hierro_temprano` o `casco_pesada_bronce_laminado`. Tres pesos por cinco tiers por cinco huecos son 75, menos los tres cascos de cobre: **72 piezas**. Las doce de cobre sustituyen a las doce de Conquest (`TorLADef` → `torso_ligera_cobre`, `BootHADef` → `botas_pesada_cobre`, etc.).

### 3.2 Armas: cinco clases por cinco tiers (25)

Cambio del autor (2026-11-11, hilo de clases de héroe): hay **cinco clases** y **el arma define la clase**, como en Conquest. La clase no limita el arma: el héroe es de la clase del arma que lleva puesta. Cada arma tiene los cinco tiers de armas de §2 (cobre, bronce, hierro temprano, hierro y acero).

La clase se llama como el arma que lleva (los héroes de la Antigüedad solo sirvieron de inspiración al autor):

| Clase = arma (un solo objeto, en el hueco `arma`) | id de la clase | Armaduras |
| --- | --- | --- |
| lanza y escudo grande redondo | `lanza_escudo_redondo` | pesada |
| espada corta y escudo mediano | `espada_escudo_mediano` | pesada, media |
| arco compuesto y lanza | `arco_lanza` | ligera, media |
| hacha a dos manos | `hacha_dos_manos` | media |
| dagas (espía, asesino) | `dagas` | ligera |

Las clases, sus armaduras, sus atributos base y sus habilidades están cerradas en `Clases_De_Heroe.md`; este catálogo pone las armas.

Ids: `{clase}_{tier}`. Nombre: el de la clase con el material del tier. Cinco clases por cinco tiers, **25 armas**:

| Clase | cobre | bronce | hierro temprano | hierro | acero |
| --- | --- | --- | --- | --- | --- |
| lanza y escudo grande redondo | `lanza_escudo_redondo_cobre` | `…_bronce` | `…_hierro_temprano` | `…_hierro` | `…_acero` |
| espada corta y escudo mediano | `espada_escudo_mediano_cobre` | `…_bronce` | `…_hierro_temprano` | `…_hierro` | `…_acero` |
| arco compuesto y lanza | `arco_lanza_cobre` | `…_bronce` | `…_hierro_temprano` | `…_hierro` | `…_acero` |
| hacha a dos manos | `hacha_dos_manos_cobre` | `…_bronce` | `…_hierro_temprano` | `…_hierro` | `…_acero` |
| dagas | `dagas_cobre` | `…_bronce` | `…_hierro_temprano` | `…_hierro` | `…_acero` |

Por ejemplo, «Lanza de bronce y escudo grande redondo», «Hacha de hierro temprano a dos manos», «Dagas de cobre». En el arco y lanza el tier es el de la punta de las flechas y de la lanza.

### 3.3 Consumibles

Fuera del catálogo inicial (`Objetos_Del_Heroe.md` §4.10). La bolsa de bronce no hace falta: el botín ya suma monedas al héroe. El cofre de arma vuelve, si vuelve, con el botín.

## 4. Lo que da cada pieza en combate (propuesta para Conquest)

Cifras fijas, enteras (`Estadistica.valor` es entero), sin generador. Hay dos clases de cifras:

- **Atributos** (fuerza, destreza, armadura, vitalidad), que se suman a los de la clase (10 puntos de base entre los cuatro). El equipo da **mucho**, y por eso cada clase solo admite unos pesos (§4.1).
- **Defensas, daño, penetración y salud**, que llegan al combate como modificadores de la pila de Conquest (hoy solo pasan los atributos; ver §8).

### 4.1 Armaduras

Cada peso da lo suyo, y por eso cada clase solo admite unos pesos (autor, 2026-11-11):

- **ligera:** fuerza y destreza; de defensas, más contra el golpe contundente que contra el filo y la punta.
- **media:** equilibrada en todo.
- **pesada:** armadura y vitalidad; mucha defensa contra perforante y cortante, menos contra contundente.

Totales de un juego entero de cinco piezas en el tier de referencia; cada tier los multiplica como a las armas: ×1 cobre, ×1,25 bronce, ×1,5 hierro temprano, ×1,75 hierro y ×2 bronce laminado. Se reparten torso 30 %, casco y pantalones 20 %, guantes y botas 15 % (redondeado). Un juego de cobre, sin casco, da el 80 %.

| Juego entero (×1) | fuerza | destreza | armadura | vitalidad | def. perf. | def. cort. | def. cont. | salud |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ligera | 10 | 15 | 3 | 2 | 15 | 15 | 25 | 50 |
| media | 6 | 6 | 9 | 9 | 35 | 35 | 35 | 100 |
| pesada | 2 | 0 | 16 | 12 | 50 | 50 | 30 | 150 |

Cada juego da 30 puntos de atributos, tres veces la base de una clase (10), así que el equipo cuenta tanto como la clase o más. Lo que vale cada punto en la batalla es una regla de Conquest (§4.2).

Por pieza:

**ligera** (fue/des/arm/vit · def. perf/cort/cont · salud)

| Pieza | cobre | bronce | hierro temprano | hierro | bronce laminado |
| --- | --- | --- | --- | --- | --- |
| torso | 3/5/1/1 · 5/5/8 · 15 | 4/6/1/1 · 6/6/9 · 19 | 5/7/1/1 · 7/7/11 · 23 | 5/8/2/1 · 8/8/13 · 26 | 6/9/2/1 · 9/9/15 · 30 |
| casco | — | 3/4/1/1 · 4/4/6 · 13 | 3/5/1/1 · 5/5/8 · 15 | 4/5/1/1 · 5/5/9 · 18 | 4/6/1/1 · 6/6/10 · 20 |
| pantalones | 2/3/1/0 · 3/3/5 · 10 | 3/4/1/1 · 4/4/6 · 13 | 3/5/1/1 · 5/5/8 · 15 | 4/5/1/1 · 5/5/9 · 18 | 4/6/1/1 · 6/6/10 · 20 |
| guantes | 2/2/0/0 · 2/2/4 · 8 | 2/3/1/0 · 3/3/5 · 9 | 2/3/1/0 · 3/3/6 · 11 | 3/4/1/1 · 4/4/7 · 13 | 3/5/1/1 · 5/5/8 · 15 |
| botas | 2/2/0/0 · 2/2/4 · 8 | 2/3/1/0 · 3/3/5 · 9 | 2/3/1/0 · 3/3/6 · 11 | 3/4/1/1 · 4/4/7 · 13 | 3/5/1/1 · 5/5/8 · 15 |

**media** (fue/des/arm/vit · def. perf/cort/cont · salud)

| Pieza | cobre | bronce | hierro temprano | hierro | bronce laminado |
| --- | --- | --- | --- | --- | --- |
| torso | 2/2/3/3 · 11/11/11 · 30 | 2/2/3/3 · 13/13/13 · 38 | 3/3/4/4 · 16/16/16 · 45 | 3/3/5/5 · 18/18/18 · 53 | 4/4/5/5 · 21/21/21 · 60 |
| casco | — | 2/2/2/2 · 9/9/9 · 25 | 2/2/3/3 · 11/11/11 · 30 | 2/2/3/3 · 12/12/12 · 35 | 2/2/4/4 · 14/14/14 · 40 |
| pantalones | 1/1/2/2 · 7/7/7 · 20 | 2/2/2/2 · 9/9/9 · 25 | 2/2/3/3 · 11/11/11 · 30 | 2/2/3/3 · 12/12/12 · 35 | 2/2/4/4 · 14/14/14 · 40 |
| guantes | 1/1/1/1 · 5/5/5 · 15 | 1/1/2/2 · 7/7/7 · 19 | 1/1/2/2 · 8/8/8 · 23 | 2/2/2/2 · 9/9/9 · 26 | 2/2/3/3 · 11/11/11 · 30 |
| botas | 1/1/1/1 · 5/5/5 · 15 | 1/1/2/2 · 7/7/7 · 19 | 1/1/2/2 · 8/8/8 · 23 | 2/2/2/2 · 9/9/9 · 26 | 2/2/3/3 · 11/11/11 · 30 |

**pesada** (fue/des/arm/vit · def. perf/cort/cont · salud)

| Pieza | cobre | bronce | hierro temprano | hierro | bronce laminado |
| --- | --- | --- | --- | --- | --- |
| torso | 1/0/5/4 · 15/15/9 · 45 | 1/0/6/5 · 19/19/11 · 56 | 1/0/7/5 · 23/23/14 · 68 | 1/0/8/6 · 26/26/16 · 79 | 1/0/10/7 · 30/30/18 · 90 |
| casco | — | 1/0/4/3 · 13/13/8 · 38 | 1/0/5/4 · 15/15/9 · 45 | 1/0/6/4 · 18/18/11 · 53 | 1/0/6/5 · 20/20/12 · 60 |
| pantalones | 0/0/3/2 · 10/10/6 · 30 | 1/0/4/3 · 13/13/8 · 38 | 1/0/5/4 · 15/15/9 · 45 | 1/0/6/4 · 18/18/11 · 53 | 1/0/6/5 · 20/20/12 · 60 |
| guantes | 0/0/2/2 · 8/8/5 · 23 | 0/0/3/2 · 9/9/6 · 28 | 0/0/4/3 · 11/11/7 · 34 | 1/0/4/3 · 13/13/8 · 39 | 1/0/5/4 · 15/15/9 · 45 |
| botas | 0/0/2/2 · 8/8/5 · 23 | 0/0/3/2 · 9/9/6 · 28 | 0/0/4/3 · 11/11/7 · 34 | 1/0/4/3 · 13/13/8 · 39 | 1/0/5/4 · 15/15/9 · 45 |

### 4.2 Armas (provisional: el autor quiere discutirlo)

> **Regla del autor (2026-11-11, 18:38):** un héroe vale lo que **una unidad de una tropa de tier V**, no más; el arma y la armadura lo complementan, y como mucho aguanta algo más que esa unidad. Las cifras de esta sección, revisadas contra el código de daño de Conquest, lo dejaban muy por encima, así que cambian. En el hilo hay una propuesta sin aprobar: escala de las tropas por tier (vida y daño con la raíz del `poderBase`, el tier III con las cifras de hoy de Conquest), el héroe con equipo de cobre como una unidad de tier III y con el de tier 5 como una de tier V, el arma con unos 85 de daño base en cobre y un 7,5 % más por tier, y lo que vale cada punto en batalla (fuerza o destreza 0,75 de daño y 0,5 de penetración, vitalidad 30 de vida, defensa 0,25 % de alivio).

Cada clase tiene un perfil, sacado de cómo pelea en `Clases_De_Heroe.md` §3.1, y el tier lo multiplica: daño y penetración ×1 en cobre, ×1,25 en bronce, ×1,5 en hierro temprano, ×1,75 en hierro y ×2 en acero (redondeado). El atributo que da el arma es el mismo en todos los tiers: el tier mejora el filo, no al héroe.

| Clase (tier cobre) | daño perf. | daño cort. | daño cont. | pen. perf. | pen. cort. | pen. cont. | atributo |
| --- | --- | --- | --- | --- | --- | --- | --- |
| lanza y escudo grande redondo | 14 | 0 | 4 | 4 | 0 | 0 | armadura +2 (el escudo grande) |
| espada corta y escudo mediano | 6 | 12 | 0 | 2 | 3 | 0 | armadura +1 (el escudo mediano) |
| arco compuesto y lanza | 16 | 0 | 0 | 5 | 0 | 0 | destreza +1 |
| hacha a dos manos | 0 | 16 | 8 | 0 | 4 | 4 | fuerza +1 |
| dagas | 8 | 8 | 0 | 3 | 3 | 0 | destreza +1 |

| Clase | bronce | hierro temprano | hierro | acero |
| --- | --- | --- | --- | --- |
| lanza y escudo redondo | 18/0/5 · 5/0/0 | 21/0/6 · 6/0/0 | 25/0/7 · 7/0/0 | 28/0/8 · 8/0/0 |
| espada y escudo mediano | 8/15/0 · 3/4/0 | 9/18/0 · 3/5/0 | 11/21/0 · 4/5/0 | 12/24/0 · 4/6/0 |
| arco y lanza | 20/0/0 · 6/0/0 | 24/0/0 · 8/0/0 | 28/0/0 · 9/0/0 | 32/0/0 · 10/0/0 |
| hacha a dos manos | 0/20/10 · 0/5/5 | 0/24/12 · 0/6/6 | 0/28/14 · 0/7/7 | 0/32/16 · 0/8/8 |
| dagas | 10/10/0 · 4/4/0 | 12/12/0 · 5/5/0 | 14/14/0 · 5/5/0 | 16/16/0 · 6/6/0 |

(daño perforante/cortante/contundente · penetración perforante/cortante/contundente). La lanza del arco y lanza la pone Conquest con el cambio de arma; aquí van las cifras del arco.

## 5. Compatibilidad y equipo inicial

La compatibilidad es la columna «Armaduras» de §3.2, pieza a pieza (vale también para el casco). Como el arma define la clase, equipar un arma cambia la clase del héroe, y el motor no tiene que guardar la clase aparte: la saca del arma.

Equipo inicial: el arma de tier cobre de la clase que elija al crear el héroe y el juego de cobre, **sin casco** (autor, 2026-11-11; los cascos llegan con los juegos de más tier), de la armadura más ligera que admita esa clase: pesada para la lanza y escudo redondo, media para la espada y escudo mediano y para el hacha, ligera para el arco y lanza y para las dagas.

El hueco del arma nunca queda vacío: `desequipar` el arma se rechaza, solo se cambia por otra. Al cambiar a un arma cuya clase no admite la armadura puesta, esa armadura pasa a la mochila, y si no cabe se rechaza el cambio.

La compatibilidad y la armadura inicial de cada clase viven en el bloque `CLASES_HEROE` del diseño de clases (`Clases_De_Heroe.md`), servido junto a `OBJETOS` pero con **su propia versión** (`bronzeage-clases-1`), porque las clases cambian poco y el catálogo mucho (autor, 2026-11-11). En `OBJETOS` cada arma solo dice su clase, por id, y su tier; el motor comprueba al cargar que esa clase existe.

La tecnología del tier **solo limita comprar y forjar** (autor, 2026-11-11): un arma o una armadura que ya se tiene se puede llevar siempre.

## 6. Precios

El héroe nace con 1.500 monedas, gana 500 por una batalla ganada contra otra Facción y 50-100 por un campamento de bandidos. El precio de Conquest (24 una armadura, 30 un arma) no pesa nada en esa escala, así que se sube y se separa por material:

El precio sube con el tier: ×1 cobre, ×2 bronce, ×3,5 hierro temprano, ×5 hierro y ×7 el quinto (acero o bronce laminado).

| Pieza (tier cobre) | Compra | Venta (50 %) |
| --- | --- | --- |
| armadura ligera (torso 150) | 100 | 50 |
| armadura media (torso 300) | 200 | 100 |
| armadura pesada (torso 450) | 300 | 150 |
| arma | 200 | 100 |

Un juego ligero de cobre cuesta 450 y uno pesado de bronce, con casco, 3.600. Un arma: 200, 400, 700, 1.000 y 1.400 según el tier. Las rarezas, cuando lleguen, multiplican como en Conquest: ×1, ×2,5, ×5, ×12,5 y ×25.

## 6b. Coste de fabricación (forja)

Lo que cuesta **forjar** una pieza (la forja es la del diseño de objetos: el motor tira los valores dentro de los rangos de la receta; los rangos están por decidir). Lo paga el almacén personal del héroe, como decidió el autor. Los materiales son los recursos que ya fabrican la Fundición y la Curtiduría.

### El plano y el crisol

Además de los materiales, cada forja pide **un plano de su tier y crisoles** (autor, 2026-11-11).

- **Planos por tier** (`plano_1` … `plano_5`, «Plano de tier II», etc.): uno sirve para cualquier pieza o arma de ese tier. Para forjar una pieza de tier 2 hace falta un plano de tier 2, sus crisoles y sus materiales.
- `crisol`, «Crisol»: una forja gasta **tantos crisoles como el número de su tier**: 1 en cobre, 2 en bronce, 3 en hierro temprano, 4 en hierro y 5 en el quinto.
- Los dos son objetos de la mochila, **apilables**, sin hueco, de un tipo nuevo `material` (junto a arma, armadura, consumible y visual). No se usan en batalla.
- **Son escasos a propósito** (autor, 2026-11-11 17:52): son el único freno a que un jugador forje cientos de piezas al día. Por eso ni se fabrican en la plaza ni se compran.
- **El plano se gasta** en cada forja, uno por pieza; si se quedara, solo repetiría el freno de la tecnología.
- **Comercio** (autor, 2026-11-11): de inicio solo se pueden **vender a la tienda**, por monedas; ni se compran en ella ni se cambian entre jugadores. Venta: crisol 25 monedas; plano 50 × el número de su tier.

#### De dónde salen

Objetivo: un jugador activo forja **una o dos piezas de tier I o II al día**, una de III a la semana, alguna de IV cada pocos meses y una o dos de V en la season. **Cuanto más alto el tier, más escaso el plano** (autor, 2026-11-11 18:01), en cada fuente y en el total. Todas las fuentes van atadas a algo que ya tiene freno en el motor (la ventana de 24 h de los bandidos, las misiones diarias, los alijos que se abren una vez, los rangos de los mercenarios).

| Fuente | Crisoles | Planos |
| --- | --- | --- |
| Campamento de bandidos destruido (por héroe, solo las 8 completas de su ventana de 24 h) | 1 crisol con un 25 % en nivel 1, 35 % en nivel 2 y 50 % en nivel 3 | nivel 1: 12 % de plano I; nivel 2: 10 % de I y 5 % de II; nivel 3: 6 % de II y 2 % de III |
| Misión diaria cumplida | 1 | — |
| Batalla de Unity (botín de cada héroe) | 1, y otro más si gana | 8 % de plano II, 4 % de III, 1 % de IV y 0,2 % de V |
| Alijo abierto (una vez) | 3 | uno: tier III (70 %) o IV (30 %) |
| Premio de rango de un mercenario | — | rango 4: plano IV; rango 5: plano V |

Las misiones del tablón no dan ni planos ni crisoles: las paga quien las publica, y eso sería cambiarlos entre jugadores. Cuenta de un jugador activo (8 campamentos al día, 3 de nivel 1, 3 de nivel 2 y 2 de nivel 3, y una batalla de Unity): unos 3-4 crisoles al día y, de planos, unos 4-5 de tier I a la semana, 2-3 de tier II, uno de tier III (más los alijos), uno de tier IV cada tres o cuatro meses (más los alijos y el rango 4) y casi ninguno de tier V fuera del rango 5 de un mercenario. Las tablas del botín de Unity las tiene el motor y no Conquest, porque la escasez es de economía (duda 7 de `Objetos_Del_Heroe.md`).

### Materiales

Armaduras, por pieza (casco, pantalones, guantes y botas; **el torso, el doble**):

| Peso | cuero | lingote del tier |
| --- | --- | --- |
| ligera | 4 | 1 |
| media | 3 | 2 |
| pesada | 2 | 3 |

Armas, por arma:

| Clase | lingote del tier | madera | cuero |
| --- | --- | --- | --- |
| lanza y escudo grande redondo | 2 | 3 | 3 |
| espada corta y escudo mediano | 3 | 2 | 2 |
| arco compuesto y lanza | 1 | 4 | 1 |
| hacha a dos manos | 4 | 2 | 0 |
| dagas | 2 | 0 | 1 |

Qué lingote y qué cuero pide cada tier:

| Tier | Lingote | Cuero |
| --- | --- | --- |
| I cobre | `lingoteCobre` | `cuero` (la media y la pesada, `cueroCurtido`) |
| II bronce | `lingoteBronce` | igual que el cobre |
| III hierro temprano | `lingoteHierro` | `cueroCurtido` |
| IV hierro | `lingoteHierro` ×1,5 (redondeado arriba) | `cueroCurtido` |
| V bronce laminado | `lingoteBronce` ×2 | `cueroCalidad` |
| V acero | `lingoteHierro` ×2 | `cueroCalidad` |

Ejemplos: la coraza de bronce (torso pesada bronce) pide un plano de tier II, 6 `lingoteBronce`, 4 `cueroCurtido` y 2 crisoles; las dagas de acero, un plano de tier V, 4 `lingoteHierro`, 1 `cueroCalidad` y 5 crisoles. Para comparar, la `armaduraBronceCalidad` de un soldado pide 2 `lingoteBronce` y 5 `cueroCalidad`.

## 7. El bloque `OBJETOS` del motor

Forma propuesta, para Mechanics cuando se implemente:

```json
{
  "version": "bronzeage-objetos-1",
  "ventaPct": 50,
  "rarezas": ["comun", "pocoComun", "rara", "epica", "legendaria"],
  "tiers": { "cobre": "metalurgia_cobre", "bronce": "aleacion_bronce", "hierro_temprano": "forja_hierro_temprana", "hierro": "forja_hierro_estandarizada", "bronce_laminado": "bronce_laminado", "acero": "acero_templado" },
  "objetos": [
    { "id": "torso_pesada_bronce", "nombre": "Coraza de bronce", "hueco": "torso", "tipoArmadura": "pesada", "tier": "bronce", "rareza": "comun", "apilable": false, "precio": 900,
      "combate": { "atributos": { "fuerza": 1, "destreza": 0, "armadura": 6, "vitalidad": 5 }, "defensa": { "perforante": 19, "cortante": 19, "contundente": 11 }, "salud": 56 } },
    { "id": "lanza_escudo_redondo_cobre", "nombre": "Lanza de cobre y escudo redondo", "hueco": "arma", "clase": "lanza_escudo_redondo", "tier": "cobre", "rareza": "comun", "apilable": false, "precio": 200,
      "combate": { "atributos": { "armadura": 2 }, "dano": { "perforante": 14, "cortante": 0, "contundente": 4 }, "penetracion": { "perforante": 4, "cortante": 0, "contundente": 0 } } }
  ]
}
```

`combate` lleva las cifras de §4 (las del arma, provisionales hasta cerrar §4.2), enteras como `Estadistica.valor`. `version` sustituye a `conquest-items-1` en el ticket, que lleva además la versión de `CLASES_HEROE` y la del catálogo de tropas; Conquest rechaza un ticket con una versión que no tenga. La clase del ticket sale del arma puesta; la compatibilidad y el equipo inicial, de `CLASES_HEROE`.

## 8. Lo que toca en Conquest

- Renombrar los `_id` de las 12 armaduras a las de tier cobre de §3.1 y crear las 60 de los demás tiers, cascos incluidos (con su `visualPartId`).
- Pasar de cuatro clases a cinco y crear sus armas por tier (§3.2); la clase sale del arma.
- Leer las cifras de §4 del bloque `OBJETOS` del motor, por versión, en lugar del generador.
- Aplicar todas las cifras del objeto (atributos, daño, penetración, defensa o alivio, y salud) como modificadores de la pila por `StatId` (su pendiente P1); hoy solo pasan los cuatro atributos.
- Leer del motor las cifras de combate de las tropas en lugar de las de sus `SquadData`.
- Ajustar sus coeficientes de clase a lo que se cierre en §4.2: cuánto daño, penetración y vida vale cada punto de atributo.
- `HeroClassSelector.FillBaseItemIdsForClass` con los ids nuevos y el casco.
- Quitar del modo conectado la bolsa de bronce y el cofre.
