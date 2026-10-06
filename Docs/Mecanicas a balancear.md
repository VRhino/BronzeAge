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
| 20 | TECNOLOGÍA | X de cada logro de las Eras I-III | batch (`scripts/batch/medidorTecnologia.ts`) |
| 32 | POLÍTICA | Reputación: valor de cada evento, decaimiento y umbrales (`REPUTACION`) | batch / playtest |
| 32b | POLÍTICA | Caducidad de la propuesta de anexión (`ANEXION.caducidadDias`) | playtest |
| 33 | COMERCIO | Tasa de comisión intermedia entre Facciones aliadas o vasallas de la misma Liga | batch |
| 34 | SUMINISTRO | La economía no llena el carro de un ejército | batch, tras el doble de la Granja |
| 36 | HÉROE | Ritmo, prudencias y margen de los bots; cerebro «sin plaza» | batch; antes, ración en minutos en el motor |
| 39 | MILITAR | Escala de XP de escuadra: Unity vs. números | espera a CQ-001 (Conquest) |
| 35 | MUNDO | Poder de los campamentos de bandidos: si escala con la región o con la cercanía de Facciones fuertes | batch |
| 35b | MILITAR | Gran Fundición: `CHATARRA.fraccion`, producción, coste y trabajadores | batch |
| 40 | MUNDO | Cifras de `MERCENARIOS` (placeholder) | batch |
| 42 | TECNOLOGÍA | Cifras de `AEDAS` (retraso, itinerantes, estancia, precio de venta, residentes, épicas) | batch / playtest |
| 18 | INTEL | Cifras de `INTEL` (Mirada, Informe, cupos, coste de la Taberna) | batch / playtest |
| 8 | CARAVANAS | Visibilidad por tamaño: umbral y radio (`VISION.caravanaGrande`) | batch / playtest |

## 20. Tecnología por Eras: calibrar los logros

**Calibrar la X de cada logro** con el batch (`scripts/batch/medidorTecnologia.ts`): cada umbral, en lo que marque su
contador en su semana objetivo (Doc 6.3). Plan y bitácora: `Consideraciones/Tecnologia_Eras_I-III_Definicion.md`.

## 34. La economía no llena el carro de un ejército

**Estado: medido el 2026-09-04, sin decidir.** La capacidad del carro (Doc 5.13.1) se derivó del radio
operativo sin comprobar que hubiera trigo con el que llenarlo. En batch, ningún asentamiento llegaba a llenar un
carro y 26 de 28 no podían aportar ni un grano sin bajar de su reserva de comida. Después se dobló la
producción de la Granja (Doc 4.2.1); falta volver a medir. Cifras, causa y las cuatro palancas posibles en
`Consideraciones/Movimiento_Ejercitos_Definicion.md` §10.

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

Las cifras de `MERCENARIOS` (`constants.ts`) son placeholder, sin calibrar con batch.

## 8. Caravanas: visibilidad por tamaño

Una comercial con **3 o más carros con animal** se ve a **300** de cualquier plaza o columna ajena, también en `preparando`
(`VISION.caravanaGrande` en `src/constants.ts`, Doc 5.12.7, D88): placeholder. Medir si el umbral y el radio generan
intercepciones sin que todas las caravanas sean visibles. Hecho el 2026-10-05.

## 42. Aedas: cifras

Las cifras de `AEDAS` (`constants.ts`) son placeholder, sin calibrar: retraso de conocimiento (24 h), itinerantes por Facción
(1 por cada 3, mínimo 3), estancia (6 h), velocidad (20), precio de venta (2 × el oro de la tarifa), llegada de residentes (24 h), bonos de felicidad (5 %) y de nobleza (25 %) por residente, y las épicas: cantidades de cada capítulo (`EPICAS`), enfriamiento (6 h) y carga mínima de caravana. Medir con batch cuánto
adelantan las adopciones (Era I: nivel 2 a 5,8 días) y si la venta hace irrelevante el hito, cuánto tardan las épicas en cumplirse con un solo residente (objetivo: más que el hito, menos que esperar a la Era siguiente) y si algún hecho se farmea. Plan: `Consideraciones/Aedas_Definicion.md`.

## 18. Taberna e intel: cifras

Las cifras de `INTEL` y de `EDIFICIO_CATALOGO.taberna` (`constants.ts`) son placeholder, sin calibrar: Mirada (radio 150, 2 h, base 40 de oro + 0,1 por unidad de distancia, cooldown de zona 2 h), Informe (60 de oro por nivel de la plaza mirada, cooldown 6 h por plaza), cupo de Miradas (1 / 2 / 3 por nivel de taberna, 1 en campamento) y el coste del edificio (150 madera + 60 piedra, mejoras 250+120 y 500+250). Es el **sink recurrente de oro** que la economía del oro necesita (`Economia_Del_Oro_Definicion.md` §4.5): medir con batch (los bots ya construyen la taberna y compran informes) que `oroMedio` deje de subir sin techo y que el gasto en intel quede entre un tercio y la mitad del ingreso, sin impedir ninguna de las otras ambiciones; y si el precio por distancia basta para que mirar lejos sea una decisión. Plan: `Consideraciones/Taberna_Intel_Definicion.md`.

## 32. Reputación: cifras

`REPUTACION` (`constants.ts`) es placeholder: bonus por trueque cumplido (+5) y por liberar a un vasallo (+6), penalizaciones por trueque incumplido (−8), rebelión (−10), romper alianza (−12) y atacar a un aliado (−25), bonus por minuto de alianza (0,05), decaimiento (0,2 por minuto) y umbrales (−40 para el Embajador y para la comisión, ×1,5). Medir con batch que el score no se sature en ±100 ni quede pegado a 0, y que los umbrales muerdan sin bloquear la diplomacia. Canon: Doc 2.7.

## 33. Comisión intermedia entre aliadas y vasallas

Hoy `COMISION` tiene dos tasas: 3 % dentro de la Facción y 8 % fuera (Doc 3.5). Decidir con números si las Facciones aliadas o vasallas de la misma Liga pagan una tasa intermedia, y cuál; ¿alcanza la diferencia para que merezca la pena comerciar dentro de la Liga? Es la tasa a medir, no una mecánica nueva.

## 35. Campamentos de bandidos: escala del poder

Cada campamento sale con uno de tres niveles al azar, con su poder (`CAMPAMENTOS_BANDIDOS.niveles`, D21/D37). Medir con batch si el poder debe escalar además con la región o con la cercanía de Facciones fuertes, o si el azar de niveles basta.

## 35b. Gran Fundición: cifras

Implementada el 2026-10-06 (Doc 4.2.1). Son placeholder: `CHATARRA.fraccion` (0,5 del lingote del equipo por baja), las tres recetas de `EDIFICIO_CATALOGO.granFundicion` (producción base 4, 12 artesanos), el coste (150 madera + 100 piedra + 50 oro, 2 días) y los requisitos (Facción nivel 6, capital de asentamiento 4). Medir con batch cuánta chatarra deja una batalla típica, si el estaño que devuelve cambia de verdad el techo de las tropas de bronce, y si los 12 artesanos diluyen demasiado el ratio de mano de obra de la capital. Los bots no la construyen (`bots/cerebro/gobierno.ts`); decidir si deben, antes de medir.

## 32b. Anexión: caducidad de la propuesta

Implementada el 2026-10-06 (Doc 2.6). `ANEXION.caducidadDias` = 3 días de mundo es un placeholder: lo que tarda el Rey de la absorbida en conectarse y
contestar. Con 5 jugadores en el playtest, mirar cuántas propuestas caducan sin respuesta; si son muchas, subir. Los bots rechazan al instante, así que
no entran en la medida.

