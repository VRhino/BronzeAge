# Progresión de las tropas y del héroe

Diseño del autor del 2026-11-11, sin implementar. Lo sumó a la fase de diseño de los objetos a las 22:32 y decidió las curvas entre las 22:47 y las 22:59. Lo cerró a las 22:59, salvo de dónde salen las medallas (§5).

Va con [`Cifras_De_Combate.md`](Cifras_De_Combate.md) (lo que vale cada tropa), [`Catalogo_De_Objetos.md`](Catalogo_De_Objetos.md) §3.3 (las medallas de experiencia) y [`Clases_De_Heroe.md`](Clases_De_Heroe.md) (las habilidades y los perks que se abren con el nivel).

## 1. Lo que hay hoy

- **Escuadra.** Niveles del 1 al 10 (`MILITAR.nivelMaximoEscuadra`). Pide 5, 10, 20, 40 y 40 hasta el 10, 275 en total (`experienciaParaSubirEscuadra`). Con números gana 1 por victoria y 0,5 por derrota, y cada nivel da un 3 % de poder (`bonusPoderPorNivelEscuadra`: ×1,03 en el 1 y ×1,30 en el 10). En Unity el nivel lo pone el motor, pero lo que da dentro de la batalla es de Unity (el 10 dobla vida, daño y defensa), y Conquest da 1 de experiencia al ganador, de relleno ([`Batalla_Unity_Ciclo.md`](../Mechanics/Batalla_Unity_Ciclo.md)).
- **Héroe.** No sube de nivel: espera la curva de Conquest (CQ-001). Su experiencia solo llega de Unity y se suma a `experienciaHaciaSiguienteNivel`; los combates con números no le dan nada. Conquest tiene su propia curva, 100 × 1,2^(n−1) hasta el 50 (3,8 millones), con un punto de perk por nivel.
- **Liderazgo.** `LIDERAZGO.base` = 100, fijo e igual para todos los héroes. Cada escuadra cuesta según su escalón: 7, 14, 22, 32 y 45.

## 2. La experiencia que se gana

**La unidad es una victoria con números: 1 de experiencia** a cada escuadra que combate (0,5 si pierde). Así sigue.

**En Unity, 4 por victoria y 2 por derrota** a cada escuadra que entra en la batalla (autor, 22:47). Una batalla de Unity dura más y pide al jugador más que un combate con números.

**El héroe gana lo mismo que sus escuadras** en cada combate en el que está: 1 o 0,5 con números, 4 o 2 en Unity. Lo gana una vez por combate, no una vez por escuadra. Con números también gana, cosa que hoy no pasa.

**Actividad al día.** Estimación corregida al alza porque el autor dijo que la anterior se quedaba corta, y dada por buena por él el 2026-11-11 a las 22:56.

| Actividad | Experiencia | Casual | Activo | Muy activo |
| --- | --- | ---: | ---: | ---: |
| Campamentos de bandidos | victoria con números (1) | 3 | 8 | 12 |
| Caravanas, encuentros, saqueos y avanzadas | con números, 80 % ganadas (0,9) | 2 | 8 | 20 |
| Asedios y defensas de plaza | con números, la mitad ganadas (0,75) | 0 | 2 | 4 |
| Batallas de Unity | 4 o 2, 80 % ganadas (3,6) | 1 | 3 | 6 |
| **Experiencia al día** | | **8,4** | **27,5** | **54,6** |

## 3. Las tropas: del nivel 1 al 10

**El nivel pesa lo mismo en Unity que con números:** un 3 % más de valor por nivel, ×1,03 en el 1 y ×1,30 en el 10 (autor, 22:47, la opción A). En Unity deja de doblar en el 10. Valor es aguante × daño, como en `Cifras_De_Combate.md` §2. Propuesta para Conquest: repartirlo a partes iguales, +1,5 % de aguante y +1,5 % de daño por nivel.

**La curva** es la de 1.375 de experiencia, 2,5 veces la de 550 que el autor eligió a las 22:47. Aquella se pensó para unos 12 de experiencia al día; con la actividad de §2, un jugador activo habría subido una escuadra al 10 en tres semanas. Con esta tarda unas 7 semanas, más o menos una Era (autor, 22:59).

| Nivel | Para subir a él | Acumulado | Casual | Activo | Muy activo | Valor |
| ---: | ---: | ---: | --- | --- | --- | ---: |
| 1 | — | 0 | — | — | — | ×1,03 |
| 2 | 10 | 10 | 1 día | el primer día | el primer día | ×1,06 |
| 3 | 25 | 35 | 4 días | 1 día | el primer día | ×1,09 |
| 4 | 50 | 85 | 10 días | 3 días | 2 días | ×1,12 |
| 5 | 100 | 185 | 3 semanas | 7 días | 3 días | ×1,15 |
| 6 | 150 | 335 | 6 semanas | 12 días | 6 días | ×1,18 |
| 7 | 200 | 535 | 2,1 meses | 3 semanas | 10 días | ×1,21 |
| 8 | 250 | 785 | 3,1 meses | 4 semanas | 2 semanas | ×1,24 |
| 9 | 275 | 1.060 | 4,2 meses | 6 semanas | 3 semanas | ×1,27 |
| 10 | 315 | 1.375 | 5,5 meses | 7 semanas | 4 semanas | ×1,30 |

**Las medallas de tropa suben en la misma proporción:** 5, 12, 25 y 50 de experiencia, en lugar de 2, 5, 10 y 20 (`Catalogo_De_Objetos.md` §3.3).

La tropa prestada por un campamento de mercenarios sigue sin ganar experiencia. Reponer soldados no toca el nivel.

## 4. El héroe: del nivel 1 al 30, y después sin fin

**La curva** es la del autor (22:47, aclarada a las 22:56): del 1 al 2 pide 100, del 2 al 3 120, del 3 al 4 140, y así 20 más cada nivel hasta el paso del 29 al 30, que pide 660: **11.020 en total**. Desde el 30 cada nivel pide 680, lo mismo siempre, y **el nivel sigue subiendo sin abrir nada**.

```
pide(n) = 100 + 20 · (min(n, 30) − 1)     lo que cuesta pasar del nivel n al n+1
```

**Cada nivel hasta el 30 da:**
- **un punto de atributo**, que el jugador reparte (fuerza, destreza, armadura o vitalidad). En el 30 lleva 29, y con ellos el héroe vale entre un 13 % y un 41 % más, según la clase y dónde los ponga;
- **3 de Liderazgo:** 14 en el nivel 1 y 100 en el 30 (autor, 22:47: «que empiece en 14 hasta llegar a 100»). Igual para todos los héroes del mismo nivel, sean de un jugador, de un bot o mercenarios. Sustituye al `LIDERAZGO.base` fijo de 100;
- en algunos niveles, una **habilidad** o un **perk** de su clase: habilidades en el 1, el 3, el 5 y el 10 (la definitiva), y perks en el 7 (el pasivo) y el 12 (el activo). Se abren solos, sin puntos. Lo que hacen lo fija Conquest.

Desde el 31 no da nada: ni puntos, ni Liderazgo, ni habilidades.

| Nivel | Para subir a él | Acumulado | Puntos | Liderazgo | Se abre | Activo llega en |
| ---: | ---: | ---: | ---: | ---: | --- | --- |
| 1 | — | 0 | 0 | 14 | habilidad 1 (Q) | — |
| 2 | 100 | 100 | 1 | 17 | | 4 días |
| 3 | 120 | 220 | 2 | 20 | habilidad 2 (E) | 8 días |
| 4 | 140 | 360 | 3 | 23 | | 13 días |
| 5 | 160 | 520 | 4 | 26 | habilidad 3 (R) | 3 semanas |
| 7 | 200 | 900 | 6 | 32 | perk pasivo | 5 semanas |
| 10 | 260 | 1.620 | 9 | 41 | definitiva | 8 semanas |
| 12 | 300 | 2.200 | 11 | 47 | perk activo | 2,7 meses |
| 15 | 360 | 3.220 | 14 | 56 | | 3,9 meses |
| 20 | 460 | 5.320 | 19 | 71 | | 6,4 meses |
| 25 | 560 | 7.920 | 24 | 86 | | 9,6 meses |
| 30 | 660 | 11.020 | 29 | 100 | | 13 meses |
| 31 en adelante | 680 | +680 cada uno | 29 | 100 | nada | |

**El ritmo** con la actividad de §2. En una temporada de 12 meses, un jugador casual llega hacia el nivel 14, uno activo hacia el 28 y uno muy activo hacia el 42. El activo llega al 30 en unos 13 meses y el muy activo en unos 7.

**Lo que puede llevar con su Liderazgo.** En el nivel 1, con 14, dos escuadras de escalón 1 o una de escalón 2. La primera de escalón 3 (22) cabe en el nivel 4, la de escalón 4 (32) en el 7 y la de escalón 5 (45) en el 12.

## 5. De dónde salen las medallas y el cofre de monturas

**Propuesta, pendiente de que el autor la vea.** Las medallas de tropa ayudan sobre todo a una escuadra nueva a alcanzar a las demás; las del héroe valen lo mismo a cualquier nivel (dan una parte de lo que pide el suyo; desde el 30, de 680).

| Fuente | Medallas de tropa | Medallas del héroe | Cofre de monturas |
| --- | --- | --- | --- |
| Campamento de bandidos | muy pequeña, 10 % | muy pequeña, 5 % | — |
| Misión diaria cumplida | una pequeña | una muy pequeña | — |
| Batalla de Unity ganada (botín de cada héroe) | mediana, 10 % | mediana, 5 % | 1 % |
| Alijo abierto (una vez) | una grande | — | — |
| Premio de rango 4 y 5 de un mercenario | — | una grande | uno |

## 6. Lo que toca en el motor

- **`MILITAR`:** la curva nueva en `experienciaParaSubirEscuadra` (§3) y dos cifras nuevas para Unity, `experienciaUnityPorVictoria` (4) y `experienciaUnityPorDerrota` (2), publicadas en `GET /v1/datos`.
- **El héroe sube de nivel** (cierra CQ-001 por el lado del motor). Bloque nuevo con la curva (`experienciaBase` 100, `experienciaPorNivel` 20, `nivelDeTope` 30), que también va en `GET /v1/datos`. La experiencia del héroe se guarda en milésimas, como la de la escuadra, porque con números puede ganar 0,5. Al cruzar un nivel: el punto de atributo (`puntosDeAtributoSinGastar`, hasta el 30) y el evento `heroe.nivel_subio`.
- **El héroe gana experiencia con números,** lo mismo que sus escuadras, una vez por combate.
- **`LIDERAZGO`:** `base` 14, `porNivel` 3 y `tope` 100, en lugar de la base fija de 100. `liderazgoDe(héroe)` sale del nivel. Los bots leen el Liderazgo de su observación, así que les llega solo.
- **Lo que se abre por nivel** (habilidades en 1, 3, 5 y 10, perks en 7 y 12), como dato del motor para el cliente y para el ticket.
- **La orden `usar_objeto`** para las medallas y el cofre de monturas (ya en `Catalogo_De_Objetos.md` §3.3), y las fuentes de §5 en el botín de cada sitio.
- **Partida nueva** al pasar: los héroes que hay nacieron con 100 de Liderazgo.

## 7. Lo que toca en Conquest

- No calcula curvas: usa el nivel de la escuadra y el del héroe que trae el ticket, y deja de usar `HeroExperienceCalculator` (100 × 1,2^(n−1)) en modo conectado.
- `xpGanada`: 4 a cada escuadra que entró si su bando gana y 2 si pierde, y lo mismo al héroe, con las cifras de `GET /v1/datos`.
- El nivel de la escuadra da +3 % de valor por nivel (§3), en lugar de doblar en el 10.
- Las habilidades y los perks del héroe se abren con su nivel (§4).

## 8. Pendiente

- **Del autor:** de dónde salen las medallas y el cofre de monturas (§5).
- **Al implementar:** si el motor recorta un `xpGanada` que traiga Unity por encima de 4, o si lo pone él sabiendo quién ganó.
