# Mecánicas por balancear

La **lista única de lo que ya está construido (o decidido) y solo necesita calibración con números**: batch,
medición o cifras que dependen de un tercero. Lo que aún necesita diseño o código vive en
`Docs/Mecanicas a desarrollar.md`.

Cuando una entrada se calibra, se **borra entera** de este archivo; lo aprendido pasa al canon (`Docs/Game/`) o a
la ficha de `Consideraciones/`. Los números de entrada se conservan de `Mecanicas a desarrollar.md` para que las
referencias antiguas (§34, §39…) sigan valiendo.

## Índice

| # | Área | Qué se balancea | Cómo se mide |
|---|---|---|---|
| 20 | TECNOLOGÍA | X de cada logro de las Eras I-III | fase 7, con bots reales (Q3); `batch tecnologia` deja el día de cada logro |
| 30 | MILITAR | El batch no ejercita la guerra: cero campañas con 30 bots | batch con checkpoint de Era II+ o bots más belicosos |
| 32 | POLÍTICA | Reputación: valor de cada evento, decaimiento y umbrales (`REPUTACION`) | batch / playtest |
| 32b | POLÍTICA | Caducidad de las propuestas de anexión y de fusión (`ANEXION.caducidadDias`, `FUSION.caducidadDias`) | playtest |
| 33 | COMERCIO | Tasa de comisión intermedia entre Facciones aliadas o vasallas de la misma Liga | batch |
| 34 | SUMINISTRO | La economía no llena los víveres de un ejército; los 350 por héroe | batch, tras el doble de la Granja y los víveres |
| 36 | HÉROE | Ritmo, prudencias y margen de los bots; cerebro «sin plaza» | batch; antes, ración en minutos en el motor |
| 39 | MILITAR | Escala de XP de escuadra: Unity vs. números | espera a CQ-001 (Conquest) |
| 35 | MUNDO | Poder de los campamentos de bandidos: si escala con la región o con la cercanía de Facciones fuertes | batch |
| 35b | MILITAR | Gran Fundición: `CHATARRA.fraccion`, producción, coste y trabajadores | batch |
| 40 | MUNDO | Cifras de `MERCENARIOS` (placeholder) | batch |
| 42 | TECNOLOGÍA | Cifras de `AEDAS` (retraso, itinerantes, estancia, precio de venta, residentes, épicas) | batch / playtest |
| 18 | INTEL | Cifras de `INTEL` (Mirada, Informe, cupos, coste de la Taberna) | batch / playtest |
| 8 | CARAVANAS | Visibilidad por tamaño: umbral y radio (`VISION.caravanaGrande`) | batch / playtest |
| 43 | COMERCIO | Peaje y trueque: las repescas que deja el 2 % — implementado 2026-10-19 (C3), re-medido 2026-10-24: 2,48 salidas por trueque, 0 % de repescas de menos de una unidad | decisión C3b (`comercio.json`) |
| 44 | COMERCIO | Ventana del acarreo en la solvencia del ascenso — implementado 2026-10-19 (C2), re-medido 2026-10-24: cuenta el 2,2 % del nominal; sin ascenso al nivel 3 que lo note | decisión C2b (`comercio.json`) |
| 45 | COMERCIO | Comisión acuñada contra el oro lleno — C4 implementado 2026-10-19, re-medido 2026-10-24: se acuña el 0,44 % del valor (tasas 3-12 %) | decisión C7 (`comercio.json`); bots con sumideros de oro (fase 7) |
| 46 | COMERCIO | Sumideros de bienes escasos: el comercio se apaga a los 13 días; cupos de la flota (M) | `batch comercio` con bots de la fase 7 (decisión C5) |
| 47 | MILITAR | Lo que la calibración del sistema 4 deja para los bots de la fase 7: saqueo de pueblos (M1), equipo de Barracón y Galería (M7), nivel 3 (C1), reputación concentrada (C1b), cifras de la empalizada (Murallas M2) | `batch militar`, `batch poblacion` y `batch comercio` con los bots de la fase 7 |
| 48 | COMERCIO | Economía entera: la reputación mediana final salta de 5,45 a 34,1 (EE-7) | fase 7, con C1b (`batch comercio` con bots) |

## 20. Tecnología por Eras: calibrar los logros

**Calibrar la X de cada logro** con el batch (`scripts/batch/medidorTecnologia.ts`): cada umbral, en lo que marque su
contador en su semana objetivo (Doc 6.3). Plan y bitácora: `typescript-final:Consideraciones/Tecnologia_Eras_I-III_Definicion.md`.

**Decisión Q3 del autor (2026-10-24):** las X se portaron tal cual del TypeScript y **no se vuelven a medir hasta la fase 7** (bots reales). El batch tecnológico de Rust (`batch tecnologia`, 6H, 2026-10-26) dejó el día de cada logro y de cada Era con esas X (la Era II llega por plazo el día 35; tabla en `Docs/Arquitectura/Sistema6_Contraste_y_Plan.md`, «Avance de 6H», y en «Para la fase 7» de `Docs/Mecanicas a desarrollar.md`). Lo mismo para la velocidad y la estancia de los Aedas (§42).

## 34. La economía no llena los víveres de un ejército

**2026-10-08:** la columna ya no come del carro sino de los **víveres** (350 por héroe, a un cuarto de ración en marcha y a una décima de eso acampada;
Doc 5.13). Lo que hay que volver a medir es si las plazas llenan esos 350 sin bajar de su reserva, y si 350 sostiene el
radio operativo con tropa propia (calculado con la prestada completa, 85 soldados).

**Estado: medido el 2026-09-04, sin decidir.** La capacidad del carro (Doc 5.13.1) se derivó del radio
operativo sin comprobar que hubiera trigo con el que llenarlo. En batch, ningún asentamiento llegaba a llenar un
carro y 26 de 28 no podían aportar ni un grano sin bajar de su reserva de comida. Después se dobló la
producción de la Granja (Doc 4.2.1); falta volver a medir. Cifras, causa y las cuatro palancas posibles en
`typescript-final:Consideraciones/Movimiento_Ejercitos_Definicion.md` §10.

## 36. Héroes bot: calibración

- **Cerebro «sin plaza»** (`src/bots/cerebro/sinPlaza.ts`): medirlo con batch cuando la ración en minutos esté en el
  motor (decisión del usuario 2026-10-04: con 60 de trigo fijos nadie llega a los bandidos del anillo).
- **Calibrar con batch** el ritmo (cada 5 ticks), las prudencias heredadas de la gobernanza y el margen sobre la
  defensa inspeccionada antes de una campaña.

Plan: `Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md`.

## 39. Escala de la experiencia de escuadra de Unity

La curva de nivel de escuadra (Doc 5.16.3, `MILITAR.experienciaParaSubirEscuadra`) está calibrada para el combate con
números, que da 1 de experiencia por victoria. La que trae una batalla de Unity (`xpGanada` por escuadra) entra en la
misma curva: cuando Conquest publique su escala (CQ-001), comprobar que encaja o convertirla al entrar.

## 40. Campamentos de mercenarios: cifras

**Re-medido el 2026-10-24:** 18,1 % de compras rechazadas por existencias (23,9 % con la semilla 7) con 219 obras; 1 viaje por obra; sin bloqueo. **Medido (`batch comercio`, 2026-10-19):** con 218 obras de pueblo (90 de madera cada una) sobre 9 campamentos, el 21,7 % de las compras se rechaza por falta de existencias (pila de 300, reposición cada 3 h) y todas las obras acaban entre los días 5 y 6 con 1 viaje de compra en la mediana; el cupo diario de residentes (60 de madera, 10 de piedra) se respeta y la reposición se disparó 50 veces. **Decidido por el autor el 2026-10-19 (C6):** mantener las cifras (pila de 300 de madera, reposición cada 3 h, cupo 60/10) hasta medir con bots.

Las cifras de `MERCENARIOS` (`constants.ts`) son placeholder, sin calibrar con batch. Desde el 2026-10-08 la tropa
prestada sale **completa** (25-30 por escuadra, antes 15): el anillo de bandidos se calculó contra 15 lanceros
(`typescript-final:Consideraciones/Campamentos_Entrada_Fundacion_Definicion.md` §423) y el cerebro «sin plaza» ya cuenta 50 de poder
(`PODER_PRESTADO`). Medir si los niveles de bandido siguen siendo un reto.

## 43. Peaje y trueque: repescas

El peaje (2 % de lo que lleva la caravana a cada ciudad ajena que cruza) deja la entrega por debajo de lo pactado y el trueque solo abona lo que llega: en `batch comercio` (2026-10-19) había 3,35 salidas de caravana por trueque cumplido (esperado 2) y el 24,3 % de los viajes de ida llevaban menos de una unidad (repescas de 0,6 u y 0,012 u). **Decidido por el autor el 2026-10-19 (C3) e implementado ese día:** una línea con algo entregado y menos de una unidad por entregar se da por saldada (el peaje del 2 % se conserva; una entrega grande sigue necesitando repesca). Regla en `Docs/Mechanics/Comercio_Caravanas.md` §4.4. Las otras opciones (peaje del almacén de origen al salir, abonar lo que sale) quedan descartadas. **Re-medido el 2026-10-24** (`batch comercio`, 30 días, semilla 42 dos veces con la misma huella y semilla 7; informe en `batch/salida/comercio/`, decisiones en `crates/batch/decisiones/comercio.json`; las cifras del 2026-10-19 de este apartado quedan **sustituidas**): salidas por trueque de 3,35 a **2,48** (2,45 con la semilla 7), viajes de ida de menos de una unidad del 24,3 % al **0 %**, trueque en el percentil 90 de 17,5 a **8,9 min**. Lo que queda por encima de 2 son repescas de más de una unidad (el 2 % de entregas de hasta 80 u). **C3b decidida por el autor el 2026-10-24:** regla nueva de fondo, **todo lo almacenado va en enteros** (resto oculto en los flujos por minuto, redondeo hacia abajo en todo lo demás); es una tanda propia («economía entera», fila en `Mecanicas a desarrollar.md`) y, con ella, las repescas de menos de una unidad desaparecen por construcción. Volver a medir tras esa tanda.

## 48. Economía entera: reputación mediana y repesca por peaje (EE-7)

Con la economía entera (C3b, 2026-10-24) `batch comercio` (160 plazas, 30 días, semilla 42) pasa de 139 a 136 trueques cumplidos y la **reputación mediana final de las Facciones salta de 5,45 a 34,1** (la máxima, 99,7 → 98,6; con reputación positiva, 17,5 % → 15 %), con otro reparto de los peajes (cobros 187 → 81 antes del mínimo de EE-4). **Decidido por el autor el 2026-10-24 (EE-7):** se investiga en la fase 7, junto a C1b (reputación concentrada), con bots reales. La repesca de la última unidad por una ruta con peaje, que con el peaje mínimo (EE-4) y sin la línea saldada por resto (EE-6) no llegaba nunca (10.421 caravanas y un 70 % de cumplimiento en el batch), quedó resuelta el mismo día: el peaje nunca deja la carga en cero.

## 44. Ventana del acarreo en la solvencia del ascenso

Medido (`batch comercio`, 2026-10-19): con la piedra de las madres al 96 % el acarreo perdía el 97,6 % de lo que transporta (ingresaban 1,64 de 67,1 millones de u) y aun así contaba entero para la solvencia como producción nominal (lo que sale es el 89,8 % del nominal). **Decidido por el autor el 2026-10-19 (C2) e implementado ese día:** cuenta la media de lo que ingresa en el almacén de la madre en las últimas 24 h (cubos horarios guardados en la plaza, formato de foto `BAN8`); con el almacén lleno un pueblo no hace solvente a nadie. Regla en `Docs/Mechanics/Economia_Construccion.md` §11.2 y §14. Las otras opciones (producción topada al hueco del almacén, dejarlo) quedan descartadas. **Re-medido el 2026-10-24** (`batch comercio`, 30 días, semilla 42 dos veces con la misma huella y semilla 7; informe en `batch/salida/comercio/`, decisiones en `crates/batch/decisiones/comercio.json`; las cifras del 2026-10-19 de este apartado quedan **sustituidas**): el acarreo sigue perdiendo el **97,5 %** (piedra de las madres al 96 %) y en la solvencia cuenta ahora el **2,2 %** de la producción nominal de los pueblos activos (2,1 % con la semilla 7); 0 ascensos rechazados por insolvencia, pero ninguna plaza se acerca al nivel 3 (114 de nivel 2, 0 de nivel 3), así que «¿cambia el ascenso a nivel 3?» sigue sin respuesta (`batch poblacion` tampoco llega al 3). Falta medirlo con almacenes que se vacían (fase 7). **C2b cerrada por el autor el 2026-10-24:** se da por implementada; se medirá cuando alguna plaza ascienda al nivel 3 (con los bots de la fase 7).

## 45. Precio de los bienes manufacturados y comisión acuñada

`PRECIO_BASE` solo tiene madera, piedra, trigo, cobre, estaño, hierro y ganado: las armas de madera, el cuero y los lingotes valían 0 al calcular la comisión, y con el oro de las plazas al 100 % desde el día 3 la comisión acuñada se topa por su capacidad: 519 entregas acuñaron 77,8 de oro (`batch comercio`, 2026-10-19). **Decidido por el autor el 2026-10-19 (C4) e implementado ese día:** cada bien con receta tiene un precio base derivado (lo que consume su receta a precio base; la receta más barata si hay varias), sin tocar `PRECIO_BASE` ni lo que venden los mercenarios. Regla en `Docs/Mechanics/Comercio_Caravanas.md` §3.1. Si los precios derivados resultan altos para las armas de bronce (21,6 el lingote) se recalibra con bots que fabriquen y consuman. **Re-medido el 2026-10-24** (`batch comercio`, 30 días, semilla 42 dos veces con la misma huella y semilla 7; informe en `batch/salida/comercio/`, decisiones en `crates/batch/decisiones/comercio.json`; las cifras del 2026-10-19 de este apartado quedan **sustituidas**): la comisión acuñada baja de 77,8 a **69,2** de oro (81,2 con la semilla 7) sobre **15.924** de oro de valor entregado (el `valor` va ahora en `comercio.caravana_llega`): un **0,44 %**, cuando las tasas darían entre el 3 % y el 12 %. El oro de las plazas está al 100 % desde el día 3 y la comisión que no cabe se pierde; mientras no haya sumideros de oro, el precio de los manufacturados no cambia nada medible. **C7 cerrada por el autor el 2026-10-24: se deja; la comisión que no cabe en el almacén se pierde** (regla, `Comercio_Caravanas.md` §3.3). Volver a mirarlo con bots que fabriquen y gasten oro (fase 7).

## 46. Sumideros de bienes escasos y cupos de la flota

En `batch comercio` los almacenes de madera, piedra, trigo y oro están al 96-100 % desde el día 3; solo comercian el ganado, las armas de madera y los minerales, y el comercio se apaga el día 13 (575 aristas, rutas caducadas a los 3 días, 0 trueques; **2026-10-24:** se apaga el día 9, con 390 aristas y 140 trueques propuestos en vez de 160, porque el jugador valora los manufacturados a su precio derivado y salen menos parejas). Las cifras de intensidad (0,035 trueques por plaza y día) no valen para calibrar hasta que haya demanda real (reclutamiento, equipo, obras: fase 7). El cupo del Mercado de nivel 1 (2 caravanas, 3 carros) sobra: una caravana de 2 carros sirve todo el comercio del batch (**decidido por el autor el 2026-10-19, C5:** dejar 2 caravanas y 3 carros). Informe: `batch/salida/comercio/informe.html` (`batch comercio --decisiones crates/batch/decisiones/comercio.json`).

## 8. Caravanas: visibilidad por tamaño

Una comercial con **3 o más carros con animal** se ve a **300** de cualquier plaza o columna ajena, también en `preparando`
(`VISION.caravanaGrande` en `src/constants.ts`, Doc 5.12.7, D88): placeholder. Medir si el umbral y el radio generan
intercepciones sin que todas las caravanas sean visibles. Hecho el 2026-10-05.

## 42. Aedas: cifras

Las cifras de `AEDAS` (`constants.ts`) son placeholder, sin calibrar: retraso de conocimiento (24 h), itinerantes por Facción
(1 por cada 3, mínimo 3), estancia (6 h), velocidad (20), precio de venta (2 × el oro de la tarifa), llegada de residentes (24 h), bonos de felicidad (5 %) y de nobleza (25 %) por residente, y las épicas: cantidades de cada capítulo (`EPICAS`), enfriamiento (6 h) y carga mínima de caravana. Medir con batch cuánto
adelantan las adopciones (Era I: nivel 2 a 5,8 días) y si la venta hace irrelevante el hito, cuánto tardan las épicas en cumplirse con un solo residente (objetivo: más que el hito, menos que esperar a la Era siguiente) y si algún hecho se farmea. Plan: `Docs/Mechanics/Tecnologia_Aedas.md`.

## 18. Taberna e intel: cifras

Las cifras de `INTEL` y de `EDIFICIO_CATALOGO.taberna` (`constants.ts`) son placeholder, sin calibrar: Mirada (radio 150, 2 h, base 40 de oro + 0,1 por unidad de distancia, cooldown de zona 2 h), Informe (60 de oro por nivel de la plaza mirada, cooldown 6 h por plaza), cupo de Miradas (1 / 2 / 3 por nivel de taberna, 1 en campamento) y el coste del edificio (150 madera + 60 piedra, mejoras 250+120 y 500+250). Es el **sink recurrente de oro** que la economía del oro necesita (`typescript-final:Consideraciones/Economia_Del_Oro_Definicion.md` §4.5): medir con batch (los bots ya construyen la taberna y compran informes) que `oroMedio` deje de subir sin techo y que el gasto en intel quede entre un tercio y la mitad del ingreso, sin impedir ninguna de las otras ambiciones; y si el precio por distancia basta para que mirar lejos sea una decisión. Plan: `typescript-final:Consideraciones/Taberna_Intel_Definicion.md`.

## 32. Reputación: cifras

**Medido (`batch comercio`, 2026-10-19, 30 días, 146 plazas, 155 trueques cumplidos):** el bonus de +5 dura 1,74 días con el decaimiento de 0,002/min; la máxima alcanzada es de 78 puntos (no satura), la mediana final entre las Facciones con dato, 15,8, solo el 17,5 % queda en positivo al final y nadie baja de 0 porque ningún trueque se incumple, así que el umbral de −40 no se ha ejercitado. Falta un escenario con incumplimientos (bots de la fase 7). **Re-medido el 2026-10-24:** con el comercio concentrado en los días 7-9, una Facción llega a **99,7** puntos (roza el tope), la mediana final baja a 5,5 y nadie baja de 0 (decisión C1b de `comercio.json`). **Decidido por el autor el 2026-10-19 (C1):** dejar las cifras (+5 por trueque, decaimiento 0,002/min, umbral −40) y recalibrar en la fase 7 con bots que incumplan; las otras opciones (bajar el decaimiento a 0,0005 o subir el umbral a −20) quedan descartadas por ahora.

`REPUTACION` (`constants.ts`) es placeholder: bonus por trueque cumplido (+5) y por liberar a un vasallo (+6), penalizaciones por trueque incumplido (−8), rebelión (−10), romper alianza (−12) y atacar a un aliado (−25), bonus por minuto de alianza (0,05), decaimiento (0,002 por minuto desde el 2026-10-18; era 0,2) y bonus por minuto de alianza (0,0005; era 0,05) y umbrales (−40 para el Embajador y para la comisión, ×1,5). Medir con batch que el score no se sature en ±100 ni quede pegado a 0, y que los umbrales muerdan sin bloquear la diplomacia. Canon: Doc 2.7.

## 33. Comisión intermedia entre aliadas y vasallas

Hoy `COMISION` tiene dos tasas: 3 % dentro de la Facción y 8 % fuera (Doc 3.5). Decidir con números si las Facciones aliadas o vasallas de la misma Liga pagan una tasa intermedia, y cuál; ¿alcanza la diferencia para que merezca la pena comerciar dentro de la Liga? Es la tasa a medir, no una mecánica nueva.

## 35. Campamentos de bandidos: escala del poder

Cada campamento sale con uno de tres niveles al azar, con su poder (`CAMPAMENTOS_BANDIDOS.niveles`, D21/D37). Medir con batch si el poder debe escalar además con la región o con la cercanía de Facciones fuertes, o si el azar de niveles basta.

## 35b. Gran Fundición: cifras

Implementada el 2026-10-06 (Doc 4.2.1). Son placeholder: `CHATARRA.fraccion` (0,5 del lingote del equipo por baja), las tres recetas de `EDIFICIO_CATALOGO.granFundicion` (producción base 4, 12 artesanos), el coste (150 madera + 100 piedra + 50 oro, 2 días) y los requisitos (Facción nivel 6, capital de asentamiento 4). Medir con batch cuánta chatarra deja una batalla típica, si el estaño que devuelve cambia de verdad el techo de las tropas de bronce, y si los 12 artesanos diluyen demasiado el ratio de mano de obra de la capital. Los bots no la construyen (`bots/cerebro/gobierno.ts`); decidir si deben, antes de medir.

## 32b. Anexión y fusión: caducidad de la propuesta

Implementada el 2026-10-06 (Doc 2.6). `ANEXION.caducidadDias` = 3 días de mundo es un placeholder: lo que tarda el Rey de la absorbida en conectarse y
contestar. Con 5 jugadores en el playtest, mirar cuántas propuestas caducan sin respuesta; si son muchas, subir. Los bots rechazan al instante, así que
no entran en la medida. La fusión (2026-10-06) tiene su propia constante, `FUSION.caducidadDias` = 3, con el mismo criterio y la misma medida; si salen
distintas, que sean dos cifras.

## 36. Batallas: convocatoria, persecución y formación en campo

Implementadas el 2026-10-06; las tres cifras son placeholder (`constants.ts`):

- `BATALLA.convocatoriaSegundos` = 30. El tiempo de mundo avanza por ticks de un minuto, así que la convocatoria de un asedio dura en la
  práctica hasta el siguiente tick. Mirar con jugadores si da tiempo a que lleguen los que estaban cerca; si no, alargarla (y entonces
  sí cuenta como segundos de verdad).
- `PERSECUCION.umbralRecalculo` = 30 (unidades de mapa). Con el radio de encuentro en 15, una presa que se mueve menos que esto no obliga a
  recalcular; contra una que va a 20 por tick se recalcula casi cada tick. Medir cuántos A* por tick cuesta con muchos perseguidores, y subirla
  si pesa.
- `FORMACION_EJERCITO.plazoMinutos` = 10 y `minimo` = 3. Con 5 jugadores en el playtest, ¿alcanzan a juntarse tres en diez minutos? Si no,
  subir el plazo antes que bajar el mínimo (el mínimo es decisión de diseño).

## 47. Lo que espera a los bots de la fase 7 (calibración del sistema 4, 2026-10-24)

Decisiones del autor sobre `crates/batch/decisiones/{militar,poblacion,comercio}.json` que **no cambian nada ahora** y se vuelven a medir con bots de verdad (`Consideraciones/Calibracion_Sistema4_Decisiones.md`):

- **Saqueo de los pueblos extractores (M1): se deja.** 2.144 saqueos en 30 días, el atacante gana siempre (poder 4,75 veces el de los 30 mineros en la mediana). Defender los pueblos (entrar en ellos) es parte del juego; cambiará cuando haya bots que lo hagan.
- **Equipo de Barracón y Galería de tiro (M7): fase 7.** 55 plazas con los dos edificios y ni una escuadra de esas tropas: piden armas y armaduras que el bot de hoy no encarga a la Armería. Medir con un bot que las encargue antes de tocar la cadena de producción.
- **Nivel 3 en el batch (C1): fase 7.** Ninguna plaza llega al nivel 3 (cupo de Facción 0, Barracón y Galería solo a mano, Curtiduría y Fundición sin materia prima).
- **Reputación concentrada (C1b): fase 7.**
- **Cifras de la empalizada (Murallas M2): se mantienen** (20 de madera y 2 de piedra por celda, puertas ×4, 15 min por celda, upkeep 0,02 de madera por celda y minuto) hasta medir con bots que compitan por la madera y con asedios.

Cerradas como regla y sin medida pendiente: el choque en campo abierto conserva la cohesión y el ×1,05 del defensor (M3, `Combate.md` §14), reclutar gasta la población que pide el ascenso (M9, `Heroe_Y_Tropa.md` §14) y el ganado se deja como está (P5).

## 48. Distancia a la capital: la escala del mantenimiento (5C-2, 2026-10-24)

Desde 5C el mantenimiento de una plaza sube con su distancia a la capital de su Facción: `× (1 + distancia / escalaDistancia)`, con `escalaDistancia` = 80 u (`Economia_Construccion.md` §10.1). A 80 u paga el doble; a 300 u, ×4,75; a 1.000 u, ×13,5 y cae en ruinas en un día. **Decisión del autor del 2026-10-24 (5C-2): sin cambio ahora; se calibra en 5I, con el batch político.** Lo que dio `batch comercio` (160 plazas pedidas, 146 colocadas, 30 días, semilla 42; Facciones de seis plazas a 30-70 u unas de otras, nivel 8, cap 6): plazas vivas al final 146 → 140, comprobaciones de ascenso insolventes 0 → 3.319, plazas en nivel 2 al final 114 → 108, trueques cumplidos 136 → 125, caravanas 390 → 338 (8 perdidas por plazas arruinadas), el acarreo pasa del 2,0 % al 9,8 % de la solvencia. Desde las correcciones de 5C (5C-1), conquistar una plaza no cambia la capital del conquistador: la conquistada, si queda lejos de ella, paga la distancia (en la prueba de la flota, a 1.500 u, cae en horas). Medir en 5I, con Facciones de bots que funden y conquisten de verdad, cuánto se puede alejar una plaza de su capital antes de no poder pagarse.

**Medido en 5I (2026-10-25, `batch politica` y `batch comercio`, 30 días, semilla 42; pregunta P1 de `crates/batch/decisiones/politica.json`).** Con 80 u lineal, en el batch político (Facciones que se fusionan y se anexionan) caen en ruinas 21 de 60 plazas, todas lejos de su capital (distancia mediana 1.048 u, ×14), con el multiplicador mediano al final en ×8,3 (p90 ×17,1); en el de comercio (plazas a 30-70 u) el multiplicador mediano es ×1,78 (p90 ×2,30), con 3.319 comprobaciones de ascenso insolventes y 141 plazas vivas de 146. Tres curvas probadas **como experimento local** (el bloque no cambia; el binario del experimento llevaba `asentamiento::mantenimiento::coste` parcheado y se revirtió):

| Curva | Político: vivas / ruinas | Político: multiplicador mediano (p90) | Político: nivel 2 | Comercio: vivas / insolventes | Comercio: multiplicador mediano (p90) |
|---|---|---|---|---|---|
| Lineal 80 u (hoy) | 40 / 21 | ×8,3 (×17,1) | 20 | 141 / 3.319 | ×1,78 (×2,30) |
| Lineal con tope ×3 | 55 / 6 | ×3,0 (×3,0) | 24 | 141 / 3.319 | ×1,78 (×2,30) |
| **Raíz `1 + √(d/80)`** (recomendada) | 57 / 4 | ×1,95 (×4,5) | 24 | 141 / 664 | ×1,88 (×2,15) |
| Lineal 200 u | 57 / 3 | ×1,34 (×5,7) | 26 | 145 / 0 | ×1,31 (×1,56) |

La raíz deja igual la expansión cercana (×2 a 80 u) y quita la ruina automática de las uniones y conquistas lejanas (×4,5 a 1.000 u).

**Decidido por el autor el 2026-10-25 (P1) e implementado ese día:** la fórmula lineal se queda y la escala pasa de 80 u a **300 u** (`1 + d/300`: ×2 a 300 u, ×3 a 600 u, ×4,3 a 1.000 u): un reino debe poder sostener **media región** alrededor de su capital y seguir limitado más allá. Regla en `Economia_Construccion.md` §10.1 y §15, Doc 1.0a y 4.5. **Re-medido el 2026-10-25** (semilla 42, 30 días, con P2, P4 y P7 aplicadas a la vez; las cifras de arriba quedan **sustituidas**):

| Medida (semilla 42, 30 días) | Antes (80 u, sin margen, cupo viejo) | Después |
|---|---|---|
| Político: plazas vivas / ruinas (lejos de su capital) | 40 / 21 (21) | **54 / 6 (6)**, distancia mediana de las lejanas 1.048 → 1.653 u |
| Político: multiplicador por distancia mediano (p90, máximo) | ×8,3 (×17,1, ×24,2) | **×1,47** (×7,1, ×7,9) |
| Político: ascensos insolventes / sin cupo de Facción | 3 / 5.134 | 1 / **1.631** |
| Político: plazas de nivel 2 | 20 | **48** |
| Político: títulos que cambian de manos (Ejército / Facción más grande / Poder económico / Gran Rey) | 215 (131 / 48 / 22 / 14) | **42** (11 / 24 / 6 / 1) |
| Político: caravanas perdidas | 119 | 92 |
| Comercio: plazas vivas / ascensos insolventes | 141 / 3.319 | **146 / 0** |
| Comercio: multiplicador mediano (p90) | ×1,78 (×2,30) | ×1,21 (×1,37) |
| Comercio: plazas de nivel 2 al final | 109 | 142 |
| Comercio: caravanas perdidas | 5 | 0 |
| Población | — | sin cambios (mismas métricas; solo el tiempo) |
| Violaciones e `inv_rechazos` (los tres batches) | 0 | 0 |

El cupo nuevo (P4) también abre plazas de nivel 2 en `batch comercio` (Facción de nivel 8: 3 → 4); con zonas más grandes los peajes suben del 0,7 % al 22,1 % de la carga que sale y las salidas por trueque de 2,70 a 3,23 (los dos en rojo): ver la duda de la calibración en `Docs/Arquitectura/Sistema5_Contraste_y_Plan.md`.

## 49. Política: lo que mide el batch político (5I, 2026-10-25)

`batch politica` (60 plazas, 30 días, semilla 42 dos veces con la misma huella y semilla 7; informe en `batch/salida/politica/`, preguntas en `crates/batch/decisiones/politica.json`). Para el autor, además de la distancia (§48, P1): **P2** los títulos cambian de manos 215 veces en 30 días (Ejército más grande 131): ¿un margen para cambiar de manos?; **P3** la Gran Fundición no se alcanza en 30 días (capital de nivel 4); **P4** el cupo de plazas de nivel 2 de una Facción de nivel 3 rechaza 5.134 ascensos; **P5** la nobleza (riesgo R1) aparece en las 18 plazas con Palacio (1.422 nobles, +6,6 % de población, ningún cambio en ruinas ni niveles frente al experimento sin nobleza); **P6** la reputación no sube nunca (mediana 0, mínima −46,5; 38 propuestas de alianza rechazadas por reputación baja: la EE-7 vista desde la política); **P7** la caravana adjunta de una plaza arruinada queda enganchada para siempre; **P8** refundar solo cuesta más de un día (almacén personal lleno, cupo diario de madera del campamento); **P9** si `batch militar` y `batch comercio` pasan a las órdenes reales de ciudadanía y residencia.

**Decidido por el autor el 2026-10-25** (cerradas en `politica.json`): **P1** escala de 300 u (§48); **P2** margen del 5 % para quitar un título (`TITULOS.margenAspirante`, Diplomacia §7): los cambios de manos bajan de 215 a 42; **P3** la Gran Fundición se deja (se medirá con batches de más semanas, fase 7); **P4** `maxNivel2` = [1,1,2,2,3,3,3,4,4,5] (dos tercios de las plazas; Doc 2.2.1): las plazas de nivel 2 del batch político pasan de 20 a 48 y los rechazos por cupo de 5.134 a 1.631; **P5** la nobleza con residentes reales se acepta; **P6** la reputación que no sube va a la fase 7 con EE-7 y C1b; **P7** la caravana adjunta cuya plaza de origen cae en ruinas se pierde con ella y con su carga, y la carga de la columna que iba en sus carros sigue a los carros (M10; Comercio §10); **P8** la refundación en solitario es cosa de los bots de la fase 7; **P9** los batches militar y de comercio siguen con el andamiaje; **P10** el redondeo de una millonésima de la reputación al reanclar se deja. Las de fase 7 tienen su fila en `Mecanicas a desarrollar.md` («Batch (5I, calibración)»).

## 30. El batch no ejercita la guerra

Con 30 bots, semilla 7, 120 960 ticks (y 60 480 con trigo ×3) el batch sale idéntico antes y después del trabajo de batallas del 2026-10-06, con cero campañas: los bots no hacen la guerra con este equilibrio, así que persecución, botín repartido, salidas y formación de ejércitos solo los cubren los tests. Para medirlos haría falta un checkpoint de Era II en adelante (los guardados hoy son del formato 1) o bots más belicosos. Ficha: `typescript-final:Consideraciones/Batalla_Este_Lado_Definicion.md`.

## Murallas: gate por nivel de asentamiento (2026-10-09)

La empalizada se abre en el nivel 2, el muro de piedra en el 3 y el adarve en el 4 (antes, todo desde el 3). Los bots comprometen el recinto de nivel 1 en cuanto pueden, así que ahora lo harán antes. Medir con batch cuánto retrasa el
crecimiento de una ciudad de nivel 2 pagar la empalizada (≈2.100 de madera, upkeep en madera) y si llega a tiempo de subirla a piedra antes del nivel 4. Ver `Docs/Mechanics/Murallas.md`.

**Tarifas acumuladas (2026-10-22, Q4).** Las tarifas por celda llana son 22 (empalizada), 52 (piedra) y 82 (adarve): construir directamente un nivel cuesta lo mismo que subir uno a uno (antes, 22 / 30 / 30, importadas del TypeScript, con el adarve más barato que la piedra). La empalizada de una ciudad de 60-90 celdas cuesta unas 1.200-2.000 de madera (20 por celda, con las puertas a ×4) (la cifra de ≈2.100 era del diseño antiguo). Siguen siendo placeholder: medir tiempo, coste, upkeep frente al mantenimiento y reparación tras una conquista cuando el autor pida el batch.

**Medido (`batch poblacion`, 2026-10-24, paso 7 del plan; informe en `batch/salida/poblacion/`, decisiones M1 y M2 en `crates/batch/decisiones/poblacion.json`).** El jugador compromete la empalizada en cuanto la plaza tiene el nivel operativo 2: el 100 % de las plazas de nivel 2 la compromete (día 5,25) y la cierra en **0,55 días** (54 celdas y 1 puerta de mediana, 15 min por celda, sin esperar nunca materiales: almacenes llenos). Cuesta **1.160 de madera y 116 de piedra**; su upkeep es el **10,8 %** del mantenimiento de la plaza. La mediana de las plazas no nota nada (73 edificios, 629 pesants). Pero en el **7,1 %** de las plazas amuralladas (5,7 % con la semilla 7) la ciudad deja de crecer (21 Viviendas y 314 pesants frente a 42 y 629: no caben Viviendas ni talleres) y **39 piezas** (Granjas sobre todo) se quedan sin salida a la red: el anillo se ciñe a la ciudad del momento y, al recalcular la red con el muro, el camino de las afueras ya no pasa por la única puerta (fila en `Mecanicas a desarrollar.md`, decisión M1). Con `franjaDeRonda` 2 (experimento, sin cambiar el bloque): 3,8 % bloqueadas y 14 piezas sin salida. Sin medir: la mejora a piedra (nivel 3) y la reparación tras una conquista.

**Re-medido el 2026-10-24 tras la decisión M1 (`LAYOUT_VERSION` 5: una puerta por cruce no parte la red y cada recinto conserva la puerta y su tramo de calle o camino a cada lado, `Murallas.md` §3):** plazas amuralladas que dejan de crecer, del 7,1 % al **0 %**; piezas sin salida, de 39 a **0**; la mediana no cambia (42 Viviendas, 629 pesants, 54 celdas) y la empalizada cuesta 1.180 de madera (antes 1.160, alguna puerta más). Las cifras de la empalizada se mantienen (M2) hasta medir con los bots de la fase 7 (§47).

## Economía: piezas gratuitas, almacén lleno y sin penalización por distancia (2026-10-11)

Los puestos del Mercado y las anclas pagan mantenimiento a la mitad (antes no contaban nada especial: peso 1); un almacén lleno pausa extractores y talleres en vez de desperdiciar; la Gran Fundición cuenta como mano de obra de artesanos; y desaparece la penalización por distancia de las recetas (que daba ×0,688 a toda la cadena de metal). Todo empuja a más producción y menos coste: medir con batch si la economía se acelera de más. Ver `Docs/Mechanics/Economia_Construccion.md`.

## Territorio: bosque, parcelas y metales aislados (2026-10-18)

Sistema 1 de la fase 6 (mundo y territorio) implementado; el informe de batch está en `batch/salida/territorio/` (se regenera con `batch territorio --decisiones crates/batch/decisiones/territorio.json`). Tras rescalar el mundo (piedra ×10 = 5.601 nodos, parcelas con `rp` = 4 = 23.877, metales 64 de estaño y 24 de oro) queda sin cerrar:

- **Bosque (decisión D2 abierta, dejado así por el autor):** el 21 % de las plazas no alcanza ninguna parcela con su zona máxima (objetivo ≤ 10 %) y solo se concede el 44 % de las Leñeras (objetivo ≥ 90 %; el 56 % se niega por «sin parcelas libres en la zona»). No falta bosque en total (3,8 % de las parcelas ocupadas): las parcelas se agrupan en masas y las zonas de 6-30 u caen a menudo fuera. Antes de rescalar otra vez, medir cuántas parcelas hay a ≤ 30 u de cada plaza y si las plazas sin parcela están lejos de toda masa; si hace falta, probar `rp` = 3 (otros ~12 min de generación) o dejar el 44 %.
- **Separación de metales aislados (`PARAMETROS_METALES.espacioAislado`):** bajada de 120 a 100 u porque con 64 de estaño (16 en frontera) la semilla 42 no admitía 16 nodos a 110. No está en el canon; si se cambia la semilla o el reparto por anillos, revisarla.
- **Cobertura de metales por la población sintética:** estaño 67 / 38 / 0 % (foco / frontera / lejana) y oro 100 / 33 / 0 %: el batch nunca se aleja más de 120 u del campamento; el gradiente 5 / 25 / 70 se mide de verdad con los bots de la fase 7.
- **Facciones sin plaza (26 %):** 400 Facciones sobre 9 campamentos saturan el foco (105 %); un jugador real buscaría más lejos. Vigilar con los bots.
