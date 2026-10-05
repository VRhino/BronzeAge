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
| 34 | SUMINISTRO | La economía no llena el carro de un ejército | batch, tras el doble de la Granja |
| 36 | HÉROE | Ritmo, prudencias y margen de los bots; cerebro «sin plaza» | batch; antes, ración en minutos en el motor |
| 39 | MILITAR | Escala de XP de escuadra: Unity vs. números | espera a CQ-001 (Conquest) |
| 40 | MUNDO | Cifras de `MERCENARIOS` (placeholder) | batch |
| 42 | TECNOLOGÍA | Cifras de `AEDAS` (retraso, nº de itinerantes, estancia, velocidad, precio de venta) | batch / playtest |
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
(1 por cada 3, mínimo 3), estancia (6 h), velocidad (20) y precio de venta (2 × el oro de la tarifa). Medir con batch cuánto
adelantan las adopciones (Era I: nivel 2 a 5,8 días) y si la venta hace irrelevante el hito. Plan: `Consideraciones/Aedas_Definicion.md`.
