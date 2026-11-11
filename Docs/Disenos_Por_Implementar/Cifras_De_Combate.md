# Cifras de combate de las tropas y del héroe

Diseño cerrado por el autor el 2026-11-11 a las 21:14 (hilo del catálogo de objetos), sin implementar. Las cifras de cada tropa son un borrador que se revisará al probarlas en batalla (§9). Sigue a [`Catalogo_De_Objetos.md`](Catalogo_De_Objetos.md), que pone las cifras de cada arma y armadura.

**Reparto.** El autor lo decidió el 2026-11-11 a las 19:30: los datos van en el motor y las reglas en Conquest.
- Las cifras de combate de cada tropa y de cada objeto son datos del motor y se sirven en `GET /v1/datos`.
- Conquest lee esas cifras por versión al arrancar el servidor de batalla, y aplica sus reglas: las fases del daño, lo que vale cada punto de atributo, los rasgos del golpe y las formaciones.

**Lo que no sirve de escala.** Las cifras de los `SquadData` de Conquest son de relleno, y las batallas con números del motor eran un sustituto provisional. Ni unas ni otras sirven de escala (autor, 2026-11-11 19:37). La escala nueva sale de las referencias del autor (§6) y de los tiers (§1).

## 1. Tiers de tropa (I-V)

Las tropas van del tier I al V, con los colores de rareza de las escuadras de Conquest (autor, 2026-11-11 20:34). No hay tier 0.

| Tier | Nombre | Rareza de escuadra en Conquest | Color |
| --- | --- | --- | --- |
| I | leva | `levy_tier` | gris |
| II | milicia | `trained_tier` | verde |
| III | línea | `veteran_tier` | azul |
| IV | profesional | `elite_tier` | morado |
| V | élite | `legendary_tier` | dorado |

**El tier y el escalón son dos números distintos** (autor, 2026-11-11 20:05).
- El **escalón** de `TROPAS_RECLUTABLES` sigue decidiendo lo que ya decidía: el Liderazgo (7/14/22/32/45), el oro por soldado y la clase de población.
- El **tier** solo decide las cifras de combate.
- Que una tropa rinda más o menos por punto de Liderazgo no es un problema. También cuentan su utilidad, sus formaciones y otros detalles (autor, 20:15).

Reparto del autor (20:34):
- **I:** granjeros, leñadores, mineros.
- **II:** milicia de lanceros, honderos, exploradores a caballo.
- **III:** lanceros con escudo de mimbre, espadachines de cobre, jinetes asirios, escaramuzadores con jabalina, hacheros ligeros, espadachines de bronce, hacheros armados, guerreros filisteos, arqueros, honderos rodios, jinetes escitas, pródromos.
- **IV:** carros de guerra, lanceros pesados micénicos, shardana, hoplitas ciudadanos, peltastas, arqueros con arco compuesto, caballería asiria, epílektoi, Inmortales, falangitas, peltastas de Ifícrates, caballería tesalia.
- **V:** espartiatas, arqueros escitas, hequetai, hipaspistas, Compañeros, agrianos, caballería noble persa.
- **El Batallón Sagrado se quita del todo.**

## 2. La escala por soldado

El tier III vale 1: 1000 de vida y 100 por golpe, con un golpe cada 1,5 s. Cada tier vale el doble que el anterior por soldado: aguanta ×1,5 y daña ×1,35. El autor pidió una escala más marcada a las 20:15.

| Tier | Aguante | Daño | Valor (aguante × daño) | Vida | Golpe |
| --- | --- | --- | --- | --- | --- |
| I | 0,45 | 0,55 | 0,25 | 450 | 55 |
| II | 0,67 | 0,75 | 0,5 | 670 | 75 |
| III | 1 | 1 | 1 | 1000 | 100 |
| IV | 1,5 | 1,35 | 2 | 1500 | 135 |
| V | 2,25 | 1,8 | 4 | 2250 | 180 |

- **Aguante** es la vida efectiva frente al mismo golpe: vida, defensa y alivio juntos.
- **Daño** es lo que quita por segundo.

## 3. Dentro del tier

Dentro de un tier hay tropas mejores que otras y con funciones distintas (autor, 2026-11-11 20:34). La escala de §2 es la base del tier, y sobre ella:

- **La armadura va encima de la vida.** La vida de la tropa es la del tier por su papel, y la armadura no la compensa. Así, con mejor equipo sale mejor tropa.
- **El metal del arma pone la penetración,** en cada tipo de daño que hace el arma.
- **El papel multiplica el aguante y el daño,** y marca su función.

Penetración por metal del arma:

| Metal | madera | piedra | cobre | bronce | bronce de calidad | hierro | hierro de calidad |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Penetración | 0 | 2 | 5 | 10 | 15 | 20 | 25 |

Defensa y alivio por armadura del soldado. La defensa es plana; el alivio va en %, por tipo perforante/cortante/contundente.

| Armadura | Defensa | Alivio |
| --- | --- | --- |
| ninguna | 0 | 0/0/0 |
| básica | 5 | 5/5/10 |
| intermedia | 10 | 10/10/10 |
| de bronce | 15 | 20/20/10 |
| de bronce de calidad | 20 | 25/25/12 |
| carro | 10 | 15/15/5 |

Cada escudo da un **bloqueo** y suma alivio contra la punta:

| Escudo | Bloqueo | Alivio perforante extra |
| --- | --- | --- |
| mimbre | 100 | +5 |
| pelta | 80 | +5 |
| mediano | 250 | +5 |
| grande | 400 | +10 |

Papeles, con su aguante y su daño frente a la base del tier, y su cadencia:

| Papel | Aguante | Daño | Golpe cada | Qué es |
| --- | --- | --- | --- | --- |
| leva | ×1 | ×1 | 1,6 s | muy mala y no forma; es el tier I entero |
| lanceros | ×1,2 | ×0,85 | 1,5 s | aguantan la línea |
| espadachines | ×1 | ×1 | 1,4 s | equilibrados |
| hacheros | ×0,85 | ×1,2 | 1,8 s | pegan más; en Conquest gastan más escudo |
| honderos | ×0,7 | ×1,4 | 1,8 s | daño contundente a distancia |
| jabalinas | ×0,8 | ×1,25 | 2,5 s | golpe fuerte a media distancia |
| arqueros | ×0,7 | ×1,45 | 2 s | el mejor daño a distancia |
| exploradores | ×0,8 | ×0,85 | 1,6 s | caballería de exploración, floja |
| jinetes | ×0,9 | ×1,1 | 1,6 s | rápidos, con carga |
| caballería | ×1 | ×1 | 1,8 s | equilibrada, con carga |
| carros | ×1,1 | ×0,95 | 2,2 s | aguantan y cargan |
| guardia de élite | ×1,1 | ×0,95 | 1,5 s | aguanta más de lo que pega |
| lanza y arco | ×0,95 | ×1,1 | 1,6 s | tira antes de cerrar (Inmortales) |

Cuentas:
- vida = 1000 × aguante del tier × aguante del papel;
- golpe = 100 × daño del tier × daño del papel × cadencia / 1,5, repartido entre los tipos según el arma.

## 4. Soldados por escuadra

Cada tropa tiene su propio número de soldados (autor, 2026-11-11 20:15).

**Lo que hay que cambiar en el motor.** El campo `unidadesPorDefecto` ya es de cada tropa, pero sus valores se copiaron del escalón (30, 25-20, 18-15, 15, 12). Además, la prueba `el_tamano_de_la_escuadra_baja_con_el_escalon` de `crates/militar/src/catalogo.rs` obliga a que bajen con él. Esa prueba se quita.

Reglas del autor (20:34):
- **La falange macedónica tiene 32 soldados:** cuatro filas de ocho.
- **Las tropas de distancia tienen todas más o menos los mismos soldados;** lo que cambia es su precisión, su alcance y su daño.
- **Las de distancia, 20 todas** (el autor lo dio por bueno a las 21:14).
- **Tier V** (autor, 21:14: 12 es muy poco para una tropa legendaria):
  - hipaspistas, 20;
  - hequetai, 18;
  - espartiatas, 16;
  - las caballerías (Compañeros y caballería noble persa), 16;
  - arqueros escitas y agrianos, 20, como las demás de distancia.
- **Inmortales,** que bajan al tier IV: 15, como la infantería de ese tier.
- El resto se queda como está.

## 5. Tiro

Conquest ya tiene en `SquadRangedData` los campos de tiro: `accuracy` de 1 a 100, `range`, `fireRate`, `reloadSpeed` y `ammo`. El motor guarda tres por tropa:

- **Precisión (1-100).** Base por tier: II 35, III 50, IV 60, V 70. La jabalina suma 10; la honda y la lanza y arco restan 5.
- **Alcance**, en metros:
  - honderos 30, honderos rodios 45;
  - escaramuzadores y peltastas 20, peltastas de Ifícrates y agrianos 25;
  - arqueros 40, jinetes escitas 35, Inmortales 40;
  - arqueros con arco compuesto y arqueros escitas 55.
- **Munición:** honda 30, jabalina 4, arco 24 y lanza y arco 16.

## 6. El héroe

### Referencias del autor (2026-11-11 19:37)

El tier del equipo del héroe es su rareza en Conquest (§2 del catálogo). El tier I de la referencia es el equipo de inicio: el autor dio el diseño por cerrado a las 21:14 con esa recomendación.
- **Tier I** (común, el equipo de inicio): el héroe aguanta como una unidad de tier V y daña como una de tier III.
- **Tier V** (legendario): aguanta como tres de tier V y daña como una de tier V.
- Entre medias, en línea recta.

| Equipo | Vida efectiva | Golpe |
| --- | --- | --- |
| I común | 2250 | 100 |
| II poco común | 3375 | 120 |
| III raro | 4500 | 140 |
| IV épico | 5625 | 160 |
| V legendario | 6750 | 180 |

### Reglas propuestas para Conquest

1. **Vida** = 1000 + 30 por punto de vitalidad + la salud de la armadura. Es igual para todas las clases.
2. **Alivio:** cada punto de defensa de un tipo quita un 0,25 % del daño de ese tipo, y cada punto del atributo armadura otro 0,25 % de todos. El tope es el 75 %, como el `maxRelief` de hoy.
3. **Golpe** = daño del arma × (1 + 1 % por punto del atributo de daño de la clase). Ese atributo es:
   - la fuerza, en la lanza y en el hacha;
   - la destreza, en el arco y en las dagas;
   - la media de las dos, en la espada.

Con estas reglas, las cifras de los objetos del catálogo (§4) están ajustadas a las referencias:
- El héroe con armadura **media** da justo las referencias.
- La **pesada** aguanta algo más y pega algo menos.
- La **ligera**, al revés.

Cada clase con su armadura inicial y sin puntos repartidos (vida efectiva / golpe; en el tier II, el metal pensado para la clase):

| Clase | Armadura | I | II | III | IV | V |
| --- | --- | --- | --- | --- | --- | --- |
| lanza y escudo grande redondo | pesada | 2381 / 96 | 3495 / 122 | 4715 / 131 | 5921 / 151 | 7393 / 167 |
| espada corta y escudo mediano | media | 2243 / 99 | 3207 / 133 | 4364 / 139 | 5446 / 160 | 6806 / 178 |
| arco compuesto y lanza | ligera | 1984 / 110 | 2928 / 152 | 3891 / 159 | 4971 / 183 | 6048 / 211 |
| hacha a dos manos | media | 2269 / 101 | 3350 / 138 | 4385 / 141 | 5465 / 162 | 6822 / 181 |
| dagas | ligera | 1979 / 111 | 2920 / 142 | 3880 / 159 | 4957 / 183 | 6030 / 212 |

## 7. Las tropas

Borrador. Las cifras salen de §2-§5 y se ajustarán cuando el autor revise tropa a tropa. Las marcadas con * son de las Eras IV y V: están diseñadas en `Consideraciones/BA-006_Revision_Tecnologia_Eras.md`, pero no están en `TROPAS_RECLUTABLES`, que solo tiene las 27 de las Eras I-III.

Columnas (cada terna va en el orden perforante/cortante/contundente):
- **Esc.:** escalón;
- **Sold.:** soldados por escuadra;
- **Vida:** vida por soldado;
- **Daño:** daño por golpe;
- **Pen.:** penetración;
- **Def.:** defensa plana;
- **Alivio:** en %;
- **Bloq.:** bloqueo del escudo;
- **Cad.:** segundos entre golpes;
- **Alc.:** alcance en metros;
- **Tiro:** precisión · munición.

| Tier | Tropa | id | Edificio | Esc. | Sold. | Vida | Daño | Pen. | Def. | Alivio | Bloq. | Cad. | Alc. | Tiro |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| I | Granjeros | `granjeros` | Centro Urbano | 1 | 30 | 450 | 29/0/29 | 0/0/0 | 0/0/0 | 0/0/0 | 0 | 1,6 | 2 | — |
| I | Leñadores | `lenadores` | Centro Urbano | 1 | 30 | 450 | 0/41/18 | 0/0/0 | 0/0/0 | 0/0/0 | 0 | 1,6 | 2 | — |
| I | Mineros | `mineros` | Centro Urbano | 1 | 30 | 450 | 35/0/23 | 2/0/2 | 0/0/0 | 0/0/0 | 0 | 1,6 | 2 | — |
| II | Milicia de lanceros | `milicia_lanceros` | Centro Urbano | 2 | 25 | 800 | 64/0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0 | 1,5 | 2 | — |
| II | Honderos | `honderos` | Galería de tiro | 2 | 20 | 470 | 0/0/126 | 0/0/2 | 0/0/0 | 0/0/0 | 0 | 1,8 | 30 | 30 · 30 |
| II | Exploradores a caballo | `exploradores_caballo` | Caballerizas | 2 | 20 | 540 | 48/20/0 | 5/5/0 | 0/0/0 | 0/0/0 | 0 | 1,6 | 2 | — |
| III | Lanceros con escudo de mimbre | `lanceros_mimbre` | Barracón | 2 | 25 | 1200 | 85/0/0 | 0/0/0 | 0/0/0 | 5/0/0 | 100 | 1,5 | 2 | — |
| III | Espadachines de espada corta de cobre | `espadachines_cobre` | Barracón | 2 | 20 | 1000 | 28/65/0 | 5/5/0 | 5/5/5 | 5/5/10 | 0 | 1,4 | 2 | — |
| III | Hacheros ligeros | `hacheros_ligeros` | Barracón | 3 | 18 | 850 | 0/101/43 | 0/10/10 | 5/5/5 | 5/5/10 | 0 | 1,8 | 2 | — |
| III | Espadachines con espadas y escudos de bronce | `espadachines_bronce` | Barracón | 3 | 18 | 1000 | 28/65/0 | 10/10/0 | 10/10/10 | 15/10/10 | 250 | 1,4 | 2 | — |
| III | Hacheros armados | `hacheros_armados` | Barracón | 3 | 15 | 850 | 0/94/50 | 0/10/10 | 10/10/10 | 10/10/10 | 0 | 1,8 | 2 | — |
| III | Guerreros filisteos (Peleset) | `guerreros_filisteos` | Barracón | 3 | 18 | 1000 | 28/65/0 | 20/20/0 | 10/10/10 | 15/10/10 | 250 | 1,4 | 2 | — |
| III | Escaramuzadores con jabalina | `escaramuzadores_jabalina` | Galería de tiro | 3 | 20 | 800 | 208/0/0 | 5/0/0 | 5/5/5 | 5/5/10 | 0 | 2,5 | 20 | 60 · 4 |
| III | Arqueros | `arqueros` | Galería de tiro | 3 | 20 | 700 | 193/0/0 | 10/0/0 | 10/10/10 | 10/10/10 | 0 | 2 | 40 | 50 · 24 |
| III | Honderos rodios | `honderos_rodios` | Galería de tiro | 3 | 20 | 700 | 0/0/168 | 0/0/20 | 5/5/5 | 5/5/10 | 0 | 1,8 | 45 | 45 · 30 |
| III | Jinetes asirios | `jinetes_asirios` | Caballerizas | 2 | 20 | 900 | 94/23/0 | 20/20/0 | 0/0/0 | 0/0/0 | 0 | 1,6 | 2 | — |
| III | Jinetes escitas | `jinetes_escitas` | Caballerizas | 3 | 20 | 700 | 193/0/0 | 10/0/0 | 5/5/5 | 5/5/10 | 0 | 2 | 35 | 50 · 24 |
| III | Pródromos * | `prodromos` | Caballerizas | 3 | 18 | 900 | 106/12/0 | 20/20/0 | 0/0/0 | 0/0/0 | 0 | 1,6 | 2 | — |
| IV | Lanceros pesados micénicos | `lanceros_pesados` | Barracón | 4 | 15 | 1800 | 98/17/0 | 10/10/0 | 15/15/15 | 30/20/10 | 400 | 1,5 | 2 | — |
| IV | Shardana | `shardana` | Barracón | 4 | 15 | 1500 | 32/94/0 | 15/15/0 | 5/5/5 | 10/5/10 | 250 | 1,4 | 2 | — |
| IV | Hoplitas ciudadanos | `hoplitas_ciudadanos` | Barracón | 3 | 18 | 1800 | 103/0/11 | 20/0/20 | 15/15/15 | 30/20/10 | 400 | 1,5 | 2 | — |
| IV | Epílektoi («los escogidos») * | `epilektoi` | Barracón | 3 | 18 | 1800 | 98/17/0 | 25/25/0 | 15/15/15 | 30/20/10 | 400 | 1,5 | 2 | — |
| IV | Inmortales * | `inmortales` | Barracón | 5 | 15 | 1420 | 158/0/0 | 25/0/0 | 10/10/10 | 15/10/10 | 100 | 1,6 | 40 | 55 · 16 |
| IV | Falangitas * | `falangitas` | Barracón | 4 | 32 | 1800 | 115/0/0 | 25/0/0 | 10/10/10 | 15/10/10 | 80 | 1,5 | 2 | — |
| IV | Peltastas | `peltastas` | Galería de tiro | 3 | 20 | 1200 | 281/0/0 | 20/0/0 | 0/0/0 | 5/0/0 | 80 | 2,5 | 20 | 70 · 4 |
| IV | Arqueros con arco compuesto | `arqueros_compuesto` | Galería de tiro | 5 | 20 | 1050 | 261/0/0 | 10/0/0 | 10/10/10 | 10/10/10 | 0 | 2 | 55 | 60 · 24 |
| IV | Peltastas de Ifícrates * | `peltastas_ificrates` | Galería de tiro | 4 | 20 | 1200 | 281/0/0 | 25/0/0 | 5/5/5 | 10/5/10 | 80 | 2,5 | 25 | 70 · 4 |
| IV | Carros de guerra | `carros_guerra` | Caballerizas | 4 | 15 | 1650 | 132/0/56 | 10/0/10 | 10/10/10 | 15/15/5 | 0 | 2,2 | 2 | — |
| IV | Caballería asiria | `caballeria_asiria` | Caballerizas | 3 | 18 | 1500 | 113/49/0 | 20/20/0 | 15/15/15 | 20/20/10 | 0 | 1,8 | 2 | — |
| IV | Caballería tesalia * | `caballeria_tesalia` | Caballerizas | 4 | 15 | 1500 | 138/24/0 | 25/25/0 | 10/10/10 | 10/10/10 | 0 | 1,8 | 2 | — |
| V | Espartiatas | `espartiatas` | Barracón | 4 | 16 | 2700 | 130/23/0 | 25/25/0 | 20/20/20 | 35/25/12 | 400 | 1,5 | 2 | — |
| V | Hequetai | `hequetai` | Barracón | 5 | 18 | 2480 | 86/86/0 | 15/15/0 | 15/15/15 | 25/20/10 | 250 | 1,5 | 2 | — |
| V | Hipaspistas * | `hipaspistas` | Barracón | 5 | 20 | 2480 | 103/68/0 | 25/25/0 | 20/20/20 | 35/25/12 | 400 | 1,5 | 2 | — |
| V | Arqueros escitas | `arqueros_escitas` | Galería de tiro | 5 | 20 | 1580 | 348/0/0 | 20/0/0 | 5/5/5 | 5/5/10 | 0 | 2 | 55 | 70 · 24 |
| V | Agrianos * | `agrianos` | Galería de tiro | 5 | 20 | 1800 | 375/0/0 | 25/0/0 | 5/5/5 | 5/5/10 | 0 | 2,5 | 25 | 80 · 4 |
| V | Caballería noble persa * | `caballeria_noble_persa` | Caballerizas | 4 | 16 | 2250 | 216/0/0 | 20/0/0 | 15/15/15 | 20/20/10 | 0 | 1,8 | 2 | — |
| V | Compañeros * | `companeros` | Caballerizas | 5 | 16 | 2250 | 173/43/0 | 25/25/0 | 20/20/20 | 25/25/12 | 0 | 1,8 | 2 | — |

## 8. El bloque del motor

Cada tropa de `TROPAS_RECLUTABLES` gana `tier` y un bloque `combate`, por ejemplo:

```json
{
  "id": "hoplitas_ciudadanos",
  "escalon": 3,
  "tier": 4,
  "unidadesPorDefecto": 18,
  "combate": {
    "vida": 1800,
    "dano": { "perforante": 103, "cortante": 0, "contundente": 11 },
    "penetracion": { "perforante": 20, "cortante": 0, "contundente": 20 },
    "defensa": { "perforante": 15, "cortante": 15, "contundente": 15 },
    "alivio": { "perforante": 30, "cortante": 20, "contundente": 10 },
    "bloqueo": 400,
    "cadencia": 1.5,
    "alcance": 2
  }
}
```

Las de distancia llevan además `precision` y `municion`. El ticket lleva la versión del catálogo de tropas, como la de `OBJETOS` y la de `CLASES_HEROE`.

## 9. Pendiente

- **Cerrado por el autor a las 21:14:**
  - el tier I de su referencia es el equipo de inicio;
  - los rasgos del golpe (cuánto suma la carga, cuánto pierde el tiro con la distancia, cuánto gasta el escudo) se quedan en Conquest como reglas de cada arma, y el motor solo guarda los números;
  - los soldados del tier V (§4).
- **Revisar las cifras tropa a tropa** sobre el borrador de §7, cuando se prueben en batalla.
- **En el motor:**
  - el `tier` y el bloque `combate` de cada tropa (§8);
  - los soldados de §4, y quitar la prueba `el_tamano_de_la_escuadra_baja_con_el_escalon`;
  - las 11 tropas de las Eras IV y V. Necesitan las Eras IV y V y sus tecnologías, que el motor no tiene. Es un trabajo aparte.
- **En Conquest:**
  - leer estas cifras del motor en lugar de las de sus `SquadData`;
  - la rareza de escuadra sale del tier (§1);
  - aplicar las reglas de §6 y los rasgos del golpe.
