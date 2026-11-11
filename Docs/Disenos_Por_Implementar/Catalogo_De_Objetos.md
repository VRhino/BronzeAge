# Catálogo inicial de objetos del héroe

Propuesta del 2026-11-11, **pendiente del sí del autor** y sin implementar. Cierra el punto 1 (y en parte el 2, el 8 y el 10) de §4 de [`Objetos_Del_Heroe.md`](Objetos_Del_Heroe.md). Parte de las plantillas de Conquest (`Assets/Resources/Items`, `HeroClassSelector.FillBaseItemIdsForClass`, `BasicArmorStatGenerator`, `BasicWeaponStatGenerator`, `ItemPricingService`) y las lleva a la Edad del Bronce con los materiales que el motor ya usa en la Armería (cuero, cuero curtido, cobre y bronce; [`Economia_Construccion.md`](../Mechanics/Economia_Construccion.md)).

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

## 2. Tema: tres armaduras, tres materiales

| Tipo | Material | Por qué |
| --- | --- | --- |
| ligera | cuero | como la `armaduraBasica` de las tropas (cuero) |
| media | cuero curtido con placas de cobre | como la `armaduraIntermedia` (cobre y cuero curtido) |
| pesada | bronce | como la `armaduraBronce`; la panoplia de Dendra |

Las rarezas se quedan las cinco de Conquest (`comun`, `pocoComun`, `rara`, `epica`, `legendaria`). El catálogo inicial es todo `comun`; las demás quedan para la forja y el botín.

## 3. Las piezas (ficha del motor)

Ids en `snake_case`, como las tropas. `hueco` es el de `Equipamiento`; ninguna pieza se apila.

### 3.1 Armaduras (15)

| id | Nombre | hueco | tipo | Conquest de hoy |
| --- | --- | --- | --- | --- |
| `gorro_cuero` | Gorro de cuero | casco | ligera | — (nuevo) |
| `jubon_cuero` | Jubón de cuero | torso | ligera | `TorLADef` |
| `brazales_cuero` | Brazales de cuero | guantes | ligera | `GloLADef` |
| `faldellin_cuero` | Faldellín de cuero | pantalones | ligera | `PanLADef` |
| `sandalias_cuero` | Sandalias de cuero | botas | ligera | `BootLADef` |
| `casco_colmillos` | Casco de colmillos de jabalí | casco | media | — (nuevo) |
| `coraza_escamas_cobre` | Coraza de escamas de cobre | torso | media | `TorMADef` |
| `brazales_tachonados` | Brazales tachonados de cobre | guantes | media | `GloMADef` |
| `faldellin_tiras` | Faldellín de tiras de cuero curtido | pantalones | media | `PanMADef` |
| `botas_cuero_curtido` | Botas de cuero curtido | botas | media | `BootMADef` |
| `casco_bronce` | Casco de bronce | casco | pesada | — (nuevo) |
| `coraza_bronce` | Coraza de bronce | torso | pesada | `TorHADef` |
| `brazales_bronce` | Brazales de bronce | guantes | pesada | `GloHADef` |
| `faldar_bronce` | Faldar de placas de bronce | pantalones | pesada | `PanHADef` |
| `grebas_bronce` | Grebas de bronce | botas | pesada | `BootHADef` |

### 3.2 Armas (4)

Una por clase de Conquest. Los nombres dependen del diseño de las clases (hilo aparte): si una clase cambia de arma, cambia aquí su fila.

| id | Nombre | categoría | Conquest de hoy |
| --- | --- | --- | --- |
| `arco_simple` | Arco de madera | arco | `WeaBBasic` |
| `lanza_cobre` | Lanza con punta de cobre | lanza | `WeaSBasic` |
| `hacha_doble_bronce` | Hacha de dos manos de bronce | dos_manos | `WeaS2HBasic` |
| `espada_escudo_bronce` | Espada corta de bronce y escudo de cuero | espada_escudo | `WeaS&SBasic` |

El mandoble de Conquest se vuelve hacha de dos manos: las espadas largas de dos manos no son de la época. La categoría del motor deja de llamarse por el arma (`dos_manos` en vez de `TwoHandedSword`) para que un cambio de nombre no toque el contrato.

### 3.3 Consumibles

Fuera del catálogo inicial (`Objetos_Del_Heroe.md` §4.10). La bolsa de bronce no hace falta: el botín ya suma monedas al héroe. El cofre de arma vuelve, si vuelve, con el botín.

## 4. Lo que da cada pieza en combate (propuesta para Conquest)

Cifras fijas, sin generador. Cada tipo tiene un total por juego completo de cinco piezas, y cada hueco se lleva una parte: torso 30 %, casco 20 %, pantalones 20 %, guantes 15 % y botas 15 % (redondeado).

| Juego completo | armadura | vitalidad | destreza | fuerza | def. perforante | def. cortante | def. contundente | salud |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ligera (cuero) | 20 | 10 | 15 | 0 | 25 | 25 | 15 | 50 |
| media (cobre) | 35 | 20 | 5 | 5 | 40 | 40 | 30 | 100 |
| pesada (bronce) | 50 | 30 | 0 | 0 | 55 | 60 | 45 | 150 |

La ligera da destreza y poca defensa; la pesada, el doble y medio de armadura y nada de destreza; el metal frena mejor el corte que el golpe. Ninguna pieza resta: Conquest no tiene valores negativos.

Por pieza:

| Pieza | arm. | vit. | des. | fue. | perf. | cort. | cont. | salud |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| jubón de cuero | 6 | 3 | 4 | 0 | 8 | 8 | 4 | 15 |
| gorro, faldellín de cuero | 4 | 2 | 3 | 0 | 5 | 5 | 3 | 10 |
| brazales, sandalias de cuero | 3 | 2 | 2 | 0 | 4 | 4 | 2 | 8 |
| coraza de escamas de cobre | 10 | 6 | 2 | 2 | 12 | 12 | 9 | 30 |
| casco de colmillos, faldellín de tiras | 7 | 4 | 1 | 1 | 8 | 8 | 6 | 20 |
| brazales tachonados, botas de cuero curtido | 5 | 3 | 1 | 1 | 6 | 6 | 4 | 15 |
| coraza de bronce | 15 | 9 | 0 | 0 | 16 | 18 | 14 | 45 |
| casco de bronce, faldar de bronce | 10 | 6 | 0 | 0 | 11 | 12 | 9 | 30 |
| brazales de bronce, grebas de bronce | 8 | 4 | 0 | 0 | 8 | 9 | 7 | 22 |

Armas:

| Arma | daño perf. | daño cort. | daño cont. | pen. perf. | pen. cort. | pen. cont. | fuerza | destreza | armadura |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| arco de madera | 18 | 0 | 0 | 6 | 0 | 0 | 0 | 10 | 0 |
| lanza con punta de cobre | 16 | 6 | 0 | 8 | 2 | 0 | 5 | 5 | 0 |
| hacha de dos manos de bronce | 0 | 20 | 10 | 0 | 6 | 4 | 10 | 0 | 0 |
| espada corta y escudo | 6 | 14 | 0 | 2 | 4 | 0 | 5 | 0 | 10 |

El escudo da armadura, por eso la espada y escudo pega menos.

Escala: el héroe tiene sus cuatro atributos de 0 a `HEROE.topeAtributo` (100). Un juego pesado entero suma 50 de armadura y 30 de vitalidad; los 30-60 por pieza del generador de hoy darían hasta 240 y se comerían los atributos. Que las defensas, el daño y la penetración lleguen al combate o solo salgan en el tooltip sigue siendo de Conquest (`Objetos_Del_Heroe.md` §3.4).

## 5. Compatibilidad y equipo inicial

Compatibilidad arma/armadura, la de Conquest, pieza a pieza (vale también para el casco):

| Arma | Armaduras que admite |
| --- | --- |
| arco | ligera |
| lanza, dos manos | ligera, media |
| espada y escudo | ligera, media, pesada |

Equipo inicial, el de Conquest con casco:

| Clase (Conquest) | Arma | Armadura |
| --- | --- | --- |
| arco | `arco_simple` | las cinco de cuero |
| lanza | `lanza_cobre` | las cinco de cobre |
| dos manos | `hacha_doble_bronce` | las cinco de cobre |
| espada y escudo | `espada_escudo_bronce` | las cinco de bronce |

## 6. Precios

El héroe nace con 1.500 monedas, gana 500 por una batalla ganada contra otra Facción y 50-100 por un campamento de bandidos. El precio de Conquest (24 una armadura, 30 un arma) no pesa nada en esa escala, así que se sube y se separa por material:

| | Precio de compra | Venta (50 %) |
| --- | --- | --- |
| pieza de cuero (torso 150) | 100 | 50 |
| pieza de cobre (torso 300) | 200 | 100 |
| pieza de bronce (torso 450) | 300 | 150 |
| arco, lanza | 200 | 100 |
| hacha de dos manos, espada y escudo | 300 | 150 |

Un juego de cuero entero cuesta 600 y uno de bronce 1.800. Las rarezas, cuando lleguen, multiplican como en Conquest: ×1, ×2,5, ×5, ×12,5 y ×25.

## 7. El bloque `OBJETOS` del motor

Forma propuesta, para Mechanics cuando se implemente:

```json
{
  "version": "bronzeage-objetos-1",
  "ventaPct": 50,
  "rarezas": ["comun", "pocoComun", "rara", "epica", "legendaria"],
  "compatibilidad": {
    "arco": ["ligera"],
    "lanza": ["ligera", "media"],
    "dos_manos": ["ligera", "media"],
    "espada_escudo": ["ligera", "media", "pesada"]
  },
  "objetos": [
    { "id": "coraza_bronce", "nombre": "Coraza de bronce", "hueco": "torso", "tipoArmadura": "pesada", "rareza": "comun", "apilable": false, "precio": 450 },
    { "id": "lanza_cobre", "nombre": "Lanza con punta de cobre", "hueco": "arma", "categoria": "lanza", "rareza": "comun", "apilable": false, "precio": 200 }
  ],
  "equipoInicial": {
    "arco": ["arco_simple", "gorro_cuero", "jubon_cuero", "brazales_cuero", "faldellin_cuero", "sandalias_cuero"]
  }
}
```

`version` sustituye a `conquest-items-1` en el ticket. `equipoInicial` va por clase y espera al catálogo de clases.

## 8. Lo que toca en Conquest

- Renombrar los `_id` de los 16 `ItemDataSO` a los de §3 y crear los tres cascos (con su `visualPartId`).
- Cambiar el generador por las cifras fijas de §4.
- `HeroClassSelector.FillBaseItemIdsForClass` con los ids nuevos y el casco.
- Quitar del modo conectado la bolsa de bronce y el cofre.
