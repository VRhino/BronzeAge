# Catálogo inicial de objetos del héroe

Diseño **aprobado por el autor el 2026-11-11** («si me cuadra todo esto», 18:12), sin implementar. Lo cambió el mismo día en el hilo:
- el tier de un objeto es su rareza en Conquest, sin tier 0 (19:59 y 20:40, §2);
- variación con sentido dentro del tier II (20:46 y 20:48, §4);
- las cifras de combate, ajustadas a sus referencias para el héroe. El autor dio el diseño por cerrado a las 21:14; las reglas y la escala están en [`Cifras_De_Combate.md`](Cifras_De_Combate.md).

Quedan abiertos los rangos de la forja (hay una propuesta en §6b), los objetos varios (trofeos, cofres, aspecto) y la tecnología `acero_templado`, que espera a las Eras IV y V.

Las cinco clases del autor (`Clases_De_Heroe.md`, cerrado) son la base de §3.2, §4 y §5. Este catálogo cierra el punto 1 (y en parte el 2, el 8 y el 10) de §4 de [`Objetos_Del_Heroe.md`](Objetos_Del_Heroe.md). Parte de las plantillas de Conquest (`Assets/Resources/Items`, `HeroClassSelector.FillBaseItemIdsForClass`, `BasicArmorStatGenerator`, `BasicWeaponStatGenerator`, `ItemPricingService`) y las lleva a la Edad del Bronce con los materiales y tecnologías que el motor ya usa en la Armería (cuero, cobre, bronce y hierro; [`Economia_Construccion.md`](../Mechanics/Economia_Construccion.md)).

**Reparto: datos en el motor, reglas en Conquest** (autor, 2026-11-11 19:30; cambia el de `Objetos_Del_Heroe.md` §2).
- La ficha de cada objeto (§3 y §6) y sus cifras de combate (§4) son del motor y van en `OBJETOS` de `GET /v1/datos`. Las cifras de combate de las tropas también pasan al motor.
- Conquest se queda con cómo se aplican: las fases del daño, lo que vale cada punto de atributo, los rasgos del golpe, las animaciones y el aspecto. Lee las cifras por versión al arrancar el servidor de batalla.
- Así el cliente jugador saca los tooltips del motor, la forja tira sobre cifras que el motor conoce y el héroe se equilibra contra las tropas con los dos lados en el mismo sitio.

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

## 2. Tiers: las cinco rarezas de Conquest

**El tier de un objeto es su rareza en Conquest** (`ItemRarity`; autor, 2026-11-11 19:59 y 20:40). No hay tier 0 ni rareza aparte: un objeto de tier III es raro, y uno raro es de tier III.

| Tier | Rareza | Color en Conquest | Armas | Armaduras |
| --- | --- | --- | --- | --- |
| I | común (`Common`) | `#8c8c8c` | de inicio | de inicio |
| II | poco común (`Uncommon`) | `#48B47A` | cobre o hierro temprano | cobre o hierro temprano |
| III | rara (`Rare`) | `#5983CB` | bronce | hierro |
| IV | épica (`Epic`) | `#A76CCB` | hierro | bronce |
| V | legendaria (`Legendary`) | `#D99F57` | acero | bronce laminado |

- **Tier I, el equipo de inicio:** materiales de pacotilla y sin tecnología. No se forja: lo vende la tienda. No lleva casco.
- **Tier II tiene dos metales,** cobre y hierro temprano, con las mismas cifras totales repartidas de otra forma (§4).
- **Las armas y las armaduras no ordenan igual los metales.** El hierro temprano protegía peor que el bronce (autor, 20:40), y en la Antigüedad la mejor armadura siguió siendo de bronce (la coraza del hoplita) mientras el hierro iba a las armas.
- **Bronce laminado** es «el bronce bueno» de las armaduras: la tecnología que ya abre la `armaduraBronceCalidad` de las tropas.
- **Acero** encaja al final de la Era V: el temple del hierro se conoce en Grecia desde Homero, y el acero nórico y el indio (wootz) aparecen hacia el 300 a. C. Depende de que existan las Eras IV y V, hoy sin implementar.

La tecnología de cada metal **solo limita comprar y forjar** (autor, 2026-11-11): un arma o una armadura que ya se tiene se puede llevar siempre.

| Metal | Tecnología | Era |
| --- | --- | --- |
| de inicio | ninguna | desde el principio |
| cobre | `metalurgia_cobre` | I, Reinos palaciales |
| hierro temprano | `forja_hierro_temprana` | II, Crisis y adaptación |
| bronce | `aleacion_bronce` | I |
| hierro | `forja_hierro_estandarizada` | III, Polis e imperios |
| bronce laminado | `bronce_laminado` | III |
| acero | `acero_templado`, tecnología nueva que aún no existe en el motor | V, Hegemonía macedónica |

En las armaduras, la tecnología no sigue al tier: la de bronce, de tier IV, se abre con la aleación del bronce de la Era I. Lo que la hace escasa es su plano (§6b).

Cada peso de armadura tiene su hechura, y el tier pone el metal:

| Peso | Hechura | Nombre, por ejemplo |
| --- | --- | --- |
| ligera | cuero con tachones del metal del tier | «Jubón de cuero con tachones de bronce» |
| media | cuero curtido con escamas del metal | «Coraza de escamas de hierro temprano» |
| pesada | placas del metal | «Coraza de bronce», «Grebas de hierro» |

Piezas de cada peso:
- **ligera:** gorro, jubón, brazales, faldellín y sandalias;
- **media:** casco con carrilleras, coraza de escamas, brazales tachonados, faldellín de tiras y botas;
- **pesada:** casco, coraza, brazales, faldar y grebas.

Las de inicio se llaman, de momento, por su hueco y su peso: «Torso de inicio (ligera)».

## 3. Las piezas (ficha del motor)

Ids en `snake_case`, como las tropas. El último trozo del id es el metal, porque el tier II tiene dos. `hueco` es el de `Equipamiento`; ninguna pieza se apila.

### 3.1 Armaduras: tres pesos por seis metales (87)

Ids: `{hueco}_{peso}_{metal}`, por ejemplo `torso_ligera_inicio` (tier I), `botas_ligera_hierro_temprano` (tier II), `torso_pesada_bronce` (tier IV) o `casco_pesada_bronce_laminado` (tier V).

Cada peso tiene 4 piezas de inicio (sin casco) y 5 de cada uno de los otros cinco metales: 29 por peso, **87 piezas**. Las doce de inicio sustituyen a las doce de Conquest (`TorLADef` → `torso_ligera_inicio`, `BootHADef` → `botas_pesada_inicio`, etc.).

### 3.2 Armas: cinco clases por seis metales (30)

Cambio del autor (2026-11-11, hilo de clases de héroe): hay **cinco clases** y **el arma define la clase**, como en Conquest. La clase no limita el arma: el héroe es de la clase del arma que lleva puesta.

La clase se llama como el arma que lleva (los héroes de la Antigüedad solo sirvieron de inspiración al autor):

| Clase = arma (un solo objeto, en el hueco `arma`) | id de la clase | Armaduras |
| --- | --- | --- |
| lanza y escudo grande redondo | `lanza_escudo_redondo` | pesada |
| espada corta y escudo mediano | `espada_escudo_mediano` | pesada, media |
| arco compuesto y lanza | `arco_lanza` | ligera, media |
| hacha a dos manos | `hacha_dos_manos` | media |
| dagas (espía, asesino) | `dagas` | ligera |

Las clases, sus armaduras, sus atributos base y sus habilidades están cerradas en `Clases_De_Heroe.md`; este catálogo pone las armas.

Ids: `{clase}_{metal}`. Nombre: el de la clase con el material. Cinco clases por seis metales, **30 armas**:

| Clase | I inicio | II cobre | II hierro temprano | III bronce | IV hierro | V acero |
| --- | --- | --- | --- | --- | --- | --- |
| lanza y escudo grande redondo | `lanza_escudo_redondo_inicio` | `…_cobre` | `…_hierro_temprano` | `…_bronce` | `…_hierro` | `…_acero` |
| espada corta y escudo mediano | `espada_escudo_mediano_inicio` | `…_cobre` | `…_hierro_temprano` | `…_bronce` | `…_hierro` | `…_acero` |
| arco compuesto y lanza | `arco_lanza_inicio` | `…_cobre` | `…_hierro_temprano` | `…_bronce` | `…_hierro` | `…_acero` |
| hacha a dos manos | `hacha_dos_manos_inicio` | `…_cobre` | `…_hierro_temprano` | `…_bronce` | `…_hierro` | `…_acero` |
| dagas | `dagas_inicio` | `…_cobre` | `…_hierro_temprano` | `…_bronce` | `…_hierro` | `…_acero` |

Por ejemplo, «Lanza de pacotilla y escudo grande redondo», «Hacha de hierro temprano a dos manos», «Dagas de acero». En el arco y lanza, el metal es el de la punta de las flechas y de la lanza.

### 3.3 Consumibles

Fuera del catálogo inicial (`Objetos_Del_Heroe.md` §4.10). La bolsa de bronce no hace falta: el botín ya suma monedas al héroe. El cofre de arma vuelve, si vuelve, con el botín.

## 4. Lo que da cada pieza en combate

Cifras fijas y enteras (`Estadistica.valor` es entero), sin generador. Hay dos clases de cifras:

- **Atributos** (fuerza, destreza, armadura, vitalidad), que se suman a los de la clase (10 puntos de base entre los cuatro). El equipo da **mucho**, y por eso cada clase solo admite unos pesos (§4.1).
- **Defensas, daño, penetración y salud**, que llegan al combate como modificadores de la pila de Conquest (hoy solo pasan los atributos; ver §8).

**Están ajustadas a las referencias del autor** con las reglas que se proponen a Conquest en `Cifras_De_Combate.md` §6. Con el equipo de inicio, el héroe aguanta como una unidad de tier V y daña como una de tier III; con el legendario, aguanta como tres de tier V y daña como una de tier V.
- El héroe con armadura **media** da justo esas referencias.
- La **pesada** aguanta algo más y pega algo menos.
- La **ligera**, al revés.
- La tabla de cada clase, tier a tier, está en ese documento.

Cómo sube cada cifra con el tier:

| Tier | I | II | III | IV | V |
| --- | --- | --- | --- | --- | --- |
| Atributos de la armadura | ×1 | ×1,25 | ×1,5 | ×1,75 | ×2 |
| Defensas y salud de la armadura | ×1 | ×2 | ×3 | ×4 | ×5 |
| Daño del arma (suma de los tres tipos) | 91 | 108 | 124 | 140 | 155 |
| Penetración del arma, en cada tipo que hace | 2 | 5 | 10 | 20 | 30 |

Los atributos suben poco porque cuentan en todas las cuentas de Conquest. Las defensas y la salud suben como la escala de las tropas, para que el héroe siga a la altura de las tropas de su tier.

### 4.1 Armaduras

Cada peso da lo suyo, y por eso cada clase solo admite unos pesos (autor, 2026-11-11):

- **ligera:** fuerza y destreza; de defensas, más contra el golpe contundente que contra el filo y la punta.
- **media:** equilibrada en todo.
- **pesada:** armadura y vitalidad; más defensa contra perforante y cortante, menos contra contundente.

Totales de un juego entero en el tier I (×1):

| Juego (tier I) | fuerza | destreza | armadura | vitalidad | def. perf. | def. cort. | def. cont. | salud |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ligera | 10 | 15 | 3 | 2 | 8 | 8 | 15 | 790 |
| media | 6 | 6 | 9 | 9 | 10 | 10 | 10 | 790 |
| pesada | 2 | 0 | 16 | 12 | 12 | 12 | 7 | 790 |

- Cada juego da 30 puntos de atributos, tres veces la base de una clase (10), así que el equipo cuenta tanto como la clase o más. Lo que vale cada punto en la batalla es una regla de Conquest.
- La salud es la misma en los tres pesos. Lo que cambia entre ellos es el alivio, que sale de la defensa y del atributo armadura.
- **Reparto por pieza:** torso 30 %, casco y pantalones 20 %, guantes y botas 15 % (redondeado). El juego de inicio no tiene casco y reparte el 100 % entre sus cuatro piezas, así que da lo mismo que un juego entero.

**Tier II: cada metal, pensado para unas clases** (autor, 2026-11-11 20:48). Varias clases llevan el mismo peso con necesidades distintas, así que el cobre y el hierro temprano reparten los mismos totales de otra forma:

| Peso | Metal | Pensado para | fue/des/arm/vit | def. perf/cort/cont | Por qué |
| --- | --- | --- | --- | --- | --- |
| ligera | cobre | arco y lanza | 7/18/3/2 | 12/6/13 | más destreza y más defensa contra flechas y jabalinas |
| ligera | hierro temprano | dagas | 13/12/3/2 | 6/12/13 | más fuerza y más defensa contra el filo, para el cuerpo a cuerpo |
| media | cobre | hacha | 9/3/8/10 | 8/11/11 | más fuerza y vitalidad, y más defensa contra el filo y el golpe |
| media | hierro temprano | espada, y arco y lanza | 4/9/10/7 | 12/9/9 | más destreza y armadura, y más defensa contra la punta |
| pesada | cobre | lanza | 1/0/18/11 | 14/11/6 | más armadura y más defensa contra la punta, para aguantar cargas |
| pesada | hierro temprano | espada | 4/2/14/10 | 11/13/7 | algo de fuerza y destreza, y más defensa contra el filo, para el duelo |

Son totales ×1 de un juego entero, como los de arriba; el tier II los multiplica (×1,25 los atributos y ×2 las defensas y la salud). Cualquier clase que admita el peso puede llevar los dos metales.

Por pieza:

**ligera** (fue/des/arm/vit · def. perf/cort/cont · salud)

| Pieza | I inicio | II cobre | II hierro temprano | III hierro | IV bronce | V bronce laminado |
| --- | --- | --- | --- | --- | --- | --- |
| torso | 4/6/1/1 · 3/3/6 · 296 | 3/7/1/1 · 7/4/8 · 474 | 5/5/1/1 · 4/7/8 · 474 | 5/7/1/1 · 7/7/14 · 711 | 5/8/2/1 · 10/10/18 · 948 | 6/9/2/1 · 12/12/23 · 1185 |
| casco | — | 2/5/1/1 · 5/2/5 · 316 | 3/3/1/1 · 2/5/5 · 316 | 3/5/1/1 · 5/5/9 · 474 | 4/5/1/1 · 6/6/12 · 632 | 4/6/1/1 · 8/8/15 · 790 |
| pantalones | 3/4/1/1 · 2/2/4 · 198 | 2/5/1/1 · 5/2/5 · 316 | 3/3/1/1 · 2/5/5 · 316 | 3/5/1/1 · 5/5/9 · 474 | 4/5/1/1 · 6/6/12 · 632 | 4/6/1/1 · 8/8/15 · 790 |
| guantes | 2/3/1/0 · 1/1/3 · 148 | 1/3/1/0 · 4/2/4 · 237 | 2/2/1/0 · 2/4/4 · 237 | 2/3/1/0 · 4/4/7 · 356 | 3/4/1/1 · 5/5/9 · 474 | 3/5/1/1 · 6/6/11 · 593 |
| botas | 2/3/1/0 · 1/1/3 · 148 | 1/3/1/0 · 4/2/4 · 237 | 2/2/1/0 · 2/4/4 · 237 | 2/3/1/0 · 4/4/7 · 356 | 3/4/1/1 · 5/5/9 · 474 | 3/5/1/1 · 6/6/11 · 593 |

**media** (fue/des/arm/vit · def. perf/cort/cont · salud)

| Pieza | I inicio | II cobre | II hierro temprano | III hierro | IV bronce | V bronce laminado |
| --- | --- | --- | --- | --- | --- | --- |
| torso | 2/2/3/3 · 4/4/4 · 296 | 3/1/3/4 · 5/7/7 · 474 | 2/3/4/3 · 7/5/5 · 474 | 3/3/4/4 · 9/9/9 · 711 | 3/3/5/5 · 12/12/12 · 948 | 4/4/5/5 · 15/15/15 · 1185 |
| casco | — | 2/1/2/3 · 3/4/4 · 316 | 1/2/3/2 · 5/4/4 · 316 | 2/2/3/3 · 6/6/6 · 474 | 2/2/3/3 · 8/8/8 · 632 | 2/2/4/4 · 10/10/10 · 790 |
| pantalones | 2/2/2/2 · 3/3/3 · 198 | 2/1/2/3 · 3/4/4 · 316 | 1/2/3/2 · 5/4/4 · 316 | 2/2/3/3 · 6/6/6 · 474 | 2/2/3/3 · 8/8/8 · 632 | 2/2/4/4 · 10/10/10 · 790 |
| guantes | 1/1/2/2 · 2/2/2 · 148 | 2/1/2/2 · 2/3/3 · 237 | 1/2/2/1 · 4/3/3 · 237 | 1/1/2/2 · 5/5/5 · 356 | 2/2/2/2 · 6/6/6 · 474 | 2/2/3/3 · 8/8/8 · 593 |
| botas | 1/1/2/2 · 2/2/2 · 148 | 2/1/2/2 · 2/3/3 · 237 | 1/2/2/1 · 4/3/3 · 237 | 1/1/2/2 · 5/5/5 · 356 | 2/2/2/2 · 6/6/6 · 474 | 2/2/3/3 · 8/8/8 · 593 |

**pesada** (fue/des/arm/vit · def. perf/cort/cont · salud)

| Pieza | I inicio | II cobre | II hierro temprano | III hierro | IV bronce | V bronce laminado |
| --- | --- | --- | --- | --- | --- | --- |
| torso | 1/0/6/4 · 4/4/3 · 296 | 0/0/7/4 · 8/7/4 · 474 | 2/1/5/4 · 7/8/4 · 474 | 1/0/7/5 · 11/11/6 · 711 | 1/0/8/6 · 14/14/8 · 948 | 1/0/10/7 · 18/18/11 · 1185 |
| casco | — | 0/0/5/3 · 6/4/2 · 316 | 1/1/4/3 · 4/5/3 · 316 | 1/0/5/4 · 7/7/4 · 474 | 1/0/6/4 · 10/10/6 · 632 | 1/0/6/5 · 12/12/7 · 790 |
| pantalones | 1/0/4/3 · 3/3/2 · 198 | 0/0/5/3 · 6/4/2 · 316 | 1/1/4/3 · 4/5/3 · 316 | 1/0/5/4 · 7/7/4 · 474 | 1/0/6/4 · 10/10/6 · 632 | 1/0/6/5 · 12/12/7 · 790 |
| guantes | 0/0/3/2 · 2/2/1 · 148 | 0/0/3/2 · 4/3/2 · 237 | 1/0/3/2 · 3/4/2 · 237 | 0/0/4/3 · 5/5/3 · 356 | 1/0/4/3 · 7/7/4 · 474 | 1/0/5/4 · 9/9/5 · 593 |
| botas | 0/0/3/2 · 2/2/1 · 148 | 0/0/3/2 · 4/3/2 · 237 | 1/0/3/2 · 3/4/2 · 237 | 0/0/4/3 · 5/5/3 · 356 | 1/0/4/3 · 7/7/4 · 474 | 1/0/5/4 · 9/9/5 · 593 |

### 4.2 Armas

Cada clase tiene un perfil, sacado de cómo pelea en `Clases_De_Heroe.md` §3.1, y el tier pone cuánto (tabla de §4):

| Clase | Reparto del daño (perf/cort/cont) | Atributo que da el arma |
| --- | --- | --- |
| lanza y escudo grande redondo | 78/0/22 % | armadura +2 (el escudo grande) |
| espada corta y escudo mediano | 33/67/0 % | armadura +1 (el escudo mediano) |
| arco compuesto y lanza | 100/0/0 % | destreza +1 |
| hacha a dos manos | 0/67/33 % | fuerza +1 |
| dagas | 50/50/0 % | destreza +1 |

- La penetración del tier va en cada tipo de daño que hace el arma.
- El atributo que da el arma es el mismo en todos los tiers: el tier mejora el filo, no al héroe.
- **Tier II** (autor, 2026-11-11 20:46): el cobre, metal blando que se dobla antes que romperse, pega ×1,1 y penetra ×0,8; el hierro temprano, de filo más duro pero quebradizo, pega ×0,9 y penetra ×1,3.

| Clase | I inicio | II cobre | II hierro temprano | III bronce | IV hierro | V acero |
| --- | --- | --- | --- | --- | --- | --- |
| lanza y escudo grande redondo | 71/0/20 · 2/0/2 | 92/0/26 · 4/0/4 | 76/0/22 · 7/0/7 | 96/0/28 · 10/0/10 | 109/0/31 · 20/0/20 | 121/0/34 · 30/0/30 |
| espada corta y escudo mediano | 30/61/0 · 2/2/0 | 40/79/0 · 4/4/0 | 32/65/0 · 7/7/0 | 41/83/0 · 10/10/0 | 47/93/0 · 20/20/0 | 52/103/0 · 30/30/0 |
| arco compuesto y lanza | 91/0/0 · 2/0/0 | 119/0/0 · 4/0/0 | 97/0/0 · 7/0/0 | 124/0/0 · 10/0/0 | 140/0/0 · 20/0/0 | 155/0/0 · 30/0/0 |
| hacha a dos manos | 0/61/30 · 0/2/2 | 0/79/40 · 0/4/4 | 0/65/32 · 0/7/7 | 0/83/41 · 0/10/10 | 0/93/47 · 0/20/20 | 0/103/52 · 0/30/30 |
| dagas | 46/46/0 · 2/2/0 | 59/59/0 · 4/4/0 | 49/49/0 · 7/7/0 | 62/62/0 · 10/10/0 | 70/70/0 · 20/20/0 | 78/78/0 · 30/30/0 |

(daño perforante/cortante/contundente · penetración perforante/cortante/contundente). La lanza del arco y lanza la pone Conquest con el cambio de arma; aquí van las cifras del arco.

## 5. Compatibilidad y equipo inicial

La compatibilidad es la columna «Armaduras» de §3.2, pieza a pieza (vale también para el casco). Como el arma define la clase, equipar un arma cambia la clase del héroe, y el motor no tiene que guardar la clase aparte: la saca del arma.

**Equipo inicial:** el arma de inicio (tier I) de la clase que elija al crear el héroe y el juego de inicio, **sin casco** (autor, 2026-11-11; el casco llega con el tier II), de su armadura inicial:
- pesada para la lanza y escudo redondo;
- media para la espada y escudo mediano y para el hacha;
- ligera para el arco y lanza y para las dagas.

La tienda vende también las piezas de inicio, para quien cambie de clase.

El hueco del arma nunca queda vacío: `desequipar` el arma se rechaza, solo se cambia por otra. Al cambiar a un arma cuya clase no admite la armadura puesta, esa armadura pasa a la mochila, y si no cabe se rechaza el cambio.

La compatibilidad y la armadura inicial de cada clase viven en el bloque `CLASES_HEROE` del diseño de clases (`Clases_De_Heroe.md`). Se sirve junto a `OBJETOS` pero con **su propia versión** (`bronzeage-clases-1`), porque las clases cambian poco y el catálogo mucho (autor, 2026-11-11). En `OBJETOS` cada arma solo dice su clase, por id, y su tier; el motor comprueba al cargar que esa clase existe.

## 6. Precios y tienda

El héroe nace con 1.500 monedas, gana 500 por una batalla ganada contra otra Facción y 50-100 por un campamento de bandidos. El precio de Conquest (24 una armadura, 30 un arma) no pesa nada en esa escala, así que se sube y se separa por peso. El precio sube con el tier: ×1, ×2, ×3,5, ×5 y ×7 del I al V. Los dos metales del tier II cuestan lo mismo.

**La tienda solo vende los tiers I y II** (autor, 2026-11-11 21:31). Los tiers III, IV y V no se compran: salen de la forja y del botín. Todo se puede vender a la tienda al 50 %, así que en los tiers III-V el precio solo sirve para saber por cuánto se vende.

| Pieza (tier I) | Compra | Venta (50 %) |
| --- | --- | --- |
| armadura ligera (torso 150) | 100 | 50 |
| armadura media (torso 300) | 200 | 100 |
| armadura pesada (torso 450) | 300 | 150 |
| arma | 200 | 100 |

Un juego, con casco desde el tier II (del III al V, solo para vender, a la mitad):

| Juego | I | II | III | IV | V |
| --- | --- | --- | --- | --- | --- |
| ligera | 450 | 1.100 | 1.925 | 2.750 | 3.850 |
| media | 900 | 2.200 | 3.850 | 5.500 | 7.700 |
| pesada | 1.350 | 3.300 | 5.775 | 8.250 | 11.550 |
| arma | 200 | 400 | 700 | 1.000 | 1.400 |

No hay un multiplicador de rareza aparte, porque la rareza es el tier.

## 6b. Coste de fabricación (forja)

Lo que cuesta **forjar** una pieza. La forja es la del diseño de objetos: el motor tira los valores dentro de los rangos de la receta (propuesta abajo, en «La calidad»). Lo paga el almacén personal del héroe, como decidió el autor. Los materiales son los recursos que ya fabrican la Fundición y la Curtiduría.

**Se forjan todos los tiers del II al V** (autor, 2026-11-11 21:31: «forjar se pueden todas»). Para los tiers III, IV y V, la forja y el botín son la única fuente. El tier I, el equipo de inicio, no se forja: lo vende la tienda (autor, 19:59).

### La calidad (propuesta, sin decidir)

- Cada pieza forjada sale con una **calidad del 90 al 110 %**, que multiplica todas sus cifras (atributos, defensas, salud, daño y penetración, redondeadas). Antes de forjar, el jugador ve el rango.
- La tienda y el botín dan la pieza al 100 %, con las cifras de §4.
- Con el héroe de `Cifras_De_Combate.md` §6, una pieza forjada al 110 % nunca aguanta como una normal del tier siguiente. En golpe, solo el arma de cobre del tier II llega a pegar como la de bronce, aunque penetra menos.
- La pega: un legendario forjado al 110 % pasa en un 10 % el tope de las referencias del autor (unos 7.500 de vida efectiva y 195 de golpe con armadura media).
- Alternativas: que cada cifra tire por su lado, o que la forja no tenga azar.

### El plano y el crisol

Además de los materiales, cada forja pide **un plano de su tier y crisoles** (autor, 2026-11-11).

- **Planos por tier** (`plano_2` … `plano_5`, «Plano de tier II», etc.): uno sirve para cualquier pieza o arma de ese tier, de cualquiera de sus metales. Para forjar una pieza de tier II hacen falta un plano de tier II, sus crisoles y sus materiales.
- **`crisol`**, «Crisol»: una forja gasta **tantos crisoles como el número de su tier**: 2 en el II, 3 en el III, 4 en el IV y 5 en el V.
- Los dos son objetos de la mochila, **apilables**, sin hueco, de un tipo nuevo `material` (junto a arma, armadura, consumible y visual). No se usan en batalla.
- **Son escasos a propósito** (autor, 2026-11-11 17:52): son el único freno a que un jugador forje cientos de piezas al día. Por eso ni se fabrican en la plaza ni se compran.
- **El plano se gasta** en cada forja, uno por pieza; si se quedara, solo repetiría el freno de la tecnología.
- **Comercio** (autor, 2026-11-11): de inicio solo se pueden **vender a la tienda**, por monedas; ni se compran en ella ni se cambian entre jugadores. Venta: crisol 25 monedas; plano 50 × el número de su tier.

#### De dónde salen

Objetivo: un jugador activo forja **una o dos piezas de tier II al día**, una de III cada una o dos semanas, alguna de IV cada pocos meses y una o dos de V en la season. **Cuanto más alto el tier, más escaso el plano** (autor, 2026-11-11 18:01), en cada fuente y en el total. Todas las fuentes van atadas a algo que ya tiene freno en el motor: la ventana de 24 h de los bandidos, las misiones diarias, los alijos que se abren una vez y los rangos de los mercenarios.

| Fuente | Crisoles | Planos |
| --- | --- | --- |
| Campamento de bandidos destruido (por héroe, solo las 8 completas de su ventana de 24 h) | 1 crisol con un 25 % en nivel 1, 35 % en nivel 2 y 50 % en nivel 3 | nivel 1: 12 % de plano II; nivel 2: 15 % de II; nivel 3: 6 % de II y 2 % de III |
| Misión diaria cumplida | 1 | — |
| Batalla de Unity (botín de cada héroe) | 1, y otro más si gana | 8 % de plano II, 4 % de III, 1 % de IV y 0,2 % de V |
| Alijo abierto (una vez) | 3 | uno: tier III (70 %) o IV (30 %) |
| Premio de rango de un mercenario | — | rango 4: plano IV; rango 5: plano V |

Los planos de bandidos que antes eran de tier I pasan al II, porque el tier I ya no se forja.

Las misiones del tablón no dan ni planos ni crisoles: las paga quien las publica, y eso sería cambiarlos entre jugadores.

Cuenta de un jugador activo, con 8 campamentos al día (3 de nivel 1, 3 de nivel 2 y 2 de nivel 3) y una batalla de Unity:
- unos 5 crisoles al día;
- un plano de tier II al día, más o menos;
- uno de tier III cada dos semanas, más los alijos;
- uno de tier IV cada tres o cuatro meses, más los alijos y el rango 4;
- casi ninguno de tier V fuera del rango 5 de un mercenario.

Las tablas del botín de Unity las tiene el motor y no Conquest, porque la escasez es de economía (duda 7 de `Objetos_Del_Heroe.md`).

### Materiales

Armaduras, por pieza (casco, pantalones, guantes y botas; **el torso, el doble**):

| Peso | cuero | lingote del metal |
| --- | --- | --- |
| ligera | 4 | 1 |
| media | 3 | 2 |
| pesada | 2 | 3 |

Armas, por arma:

| Clase | lingote del metal | madera | cuero |
| --- | --- | --- | --- |
| lanza y escudo grande redondo | 2 | 3 | 3 |
| espada corta y escudo mediano | 3 | 2 | 2 |
| arco compuesto y lanza | 1 | 4 | 1 |
| hacha a dos manos | 4 | 2 | 0 |
| dagas | 2 | 0 | 1 |

Qué lingote y qué cuero pide cada tier:

| Tier | Metal | Lingote | Cuero |
| --- | --- | --- | --- |
| II | cobre | `lingoteCobre` | `cuero` (las armaduras media y pesada, `cueroCurtido`) |
| II | hierro temprano | `lingoteHierro` | igual que el cobre |
| III | bronce (armas) | `lingoteBronce` | `cueroCurtido` |
| III | hierro (armaduras) | `lingoteHierro` | `cueroCurtido` |
| IV | hierro (armas) | `lingoteHierro` ×1,5 (redondeado arriba) | `cueroCurtido` |
| IV | bronce (armaduras) | `lingoteBronce` ×1,5 (redondeado arriba) | `cueroCurtido` |
| V | acero (armas) | `lingoteHierro` ×2 | `cueroCalidad` |
| V | bronce laminado (armaduras) | `lingoteBronce` ×2 | `cueroCalidad` |

Ejemplos:
- La coraza de bronce (torso pesada bronce, tier IV) pide un plano de tier IV, 9 `lingoteBronce`, 4 `cueroCurtido` y 4 crisoles.
- Las dagas de acero (tier V), un plano de tier V, 4 `lingoteHierro`, 1 `cueroCalidad` y 5 crisoles.
- Para comparar, la `armaduraBronceCalidad` de un soldado pide 2 `lingoteBronce` y 5 `cueroCalidad`.

## 7. El bloque `OBJETOS` del motor

Forma propuesta, para Mechanics cuando se implemente:

```json
{
  "version": "bronzeage-objetos-1",
  "ventaPct": 50,
  "tiendaHastaTier": 2,
  "rarezas": { "1": "comun", "2": "pocoComun", "3": "rara", "4": "epica", "5": "legendaria" },
  "metales": { "inicio": null, "cobre": "metalurgia_cobre", "hierro_temprano": "forja_hierro_temprana", "bronce": "aleacion_bronce", "hierro": "forja_hierro_estandarizada", "bronce_laminado": "bronce_laminado", "acero": "acero_templado" },
  "objetos": [
    { "id": "torso_pesada_bronce", "nombre": "Coraza de bronce", "hueco": "torso", "tipoArmadura": "pesada", "metal": "bronce", "tier": 4, "apilable": false, "precio": 2250,
      "combate": { "atributos": { "fuerza": 1, "destreza": 0, "armadura": 8, "vitalidad": 6 }, "defensa": { "perforante": 14, "cortante": 14, "contundente": 8 }, "salud": 948 },
      "forja": { "plano": 4, "crisoles": 4, "materiales": { "lingoteBronce": 9, "cueroCurtido": 4 } } },
    { "id": "lanza_escudo_redondo_inicio", "nombre": "Lanza de pacotilla y escudo grande redondo", "hueco": "arma", "clase": "lanza_escudo_redondo", "metal": "inicio", "tier": 1, "apilable": false, "precio": 200,
      "combate": { "atributos": { "armadura": 2 }, "dano": { "perforante": 71, "cortante": 0, "contundente": 20 }, "penetracion": { "perforante": 2, "cortante": 0, "contundente": 2 } },
      "forja": null }
  ]
}
```

- `tier` va del 1 al 5, y `rarezas` dice qué rareza de Conquest es cada uno. La tienda vende hasta el tier de `tiendaHastaTier`.
- `metales` dice qué tecnología abre cada metal; el de inicio no pide ninguna.
- `combate` lleva las cifras de §4, enteras como `Estadistica.valor`.
- `forja` lleva la receta de §6b; los rangos de la forja, cuando se decidan, irán ahí. El tier I no tiene receta.
- `version` sustituye a `conquest-items-1` en el ticket. El ticket lleva además la versión de `CLASES_HEROE` y la del catálogo de tropas (`Cifras_De_Combate.md` §8), y Conquest rechaza un ticket con una versión que no tenga.
- La clase del ticket sale del arma puesta; la compatibilidad y el equipo inicial, de `CLASES_HEROE`.

## 8. Lo que toca en Conquest

- Renombrar los `_id` de las 12 armaduras a las de inicio de §3.1 y crear las 75 de los demás metales, cascos incluidos (con su `visualPartId`).
- Pasar de cuatro clases a cinco y crear sus 30 armas (§3.2); la clase sale del arma.
- Sacar la rareza de cada objeto de su tier.
- Leer las cifras de §4 del bloque `OBJETOS` del motor, por versión, en lugar del generador.
- Aplicar todas las cifras del objeto (atributos, daño, penetración, defensa y salud) como modificadores de la pila por `StatId` (su pendiente P1); hoy solo pasan los cuatro atributos.
- Aplicar las reglas del héroe de `Cifras_De_Combate.md` §6: la vida, el alivio y el golpe.
- Leer del motor las cifras de combate de las tropas en lugar de las de sus `SquadData` (`Cifras_De_Combate.md`).
- `HeroClassSelector.FillBaseItemIdsForClass` con los ids de inicio, sin casco.
- Quitar del modo conectado la bolsa de bronce y el cofre.
