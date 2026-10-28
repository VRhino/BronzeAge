# Mecánicas por balancear

La **lista única de lo que ya está construido (o decidido) y solo necesita calibración con números**: batch,
medición o cifras que dependen de un tercero. Lo que aún necesita diseño o código vive en
`Docs/Mecanicas a desarrollar.md`.

Cuando una entrada se calibra, se **borra entera** de este archivo; lo aprendido pasa al canon (`Docs/Game/`) o a
la ficha de `Consideraciones/`. Los números de entrada se conservan de `Mecanicas a desarrollar.md` para que las
referencias antiguas (§34, §39…) sigan valiendo.

> **Revisado al cerrar la fase 6 (2026-10-26).** Se borraron las entradas calibradas o decididas: §30 (el batch no
> ejercitaba la guerra: lo sustituye `batch militar`, fila «Batch militar (4G)» de Paridad), §43-§47 (la calibración del
> sistema 4 y de `batch comercio`: `Consideraciones/Calibracion_Sistema4_Decisiones.md`), §48 (la escala de la distancia a
> la capital, decidida en P1: `Consideraciones/Politica_Tecnologia_Decisiones.md`, `crates/batch/decisiones/politica.json`
> y `Docs/Arquitectura/Sistema5_Contraste_y_Plan.md`, «Calibración del batch político») y §49 (el batch político, P1-P10,
> en los mismos sitios). La EE-7, que también se numeraba §48, pasó a §32 (reputación). Las que esperan a los bots llevan
> la línea **Fase 7** y su fila en «Para la fase 7» de `Docs/Mecanicas a desarrollar.md`.

## Índice

| # | Área | Qué se balancea | Cómo se mide |
|---|---|---|---|
| 20 | TECNOLOGÍA | X de cada logro de las Eras I-III | **Fase 7** (Q3); `batch tecnologia` deja el día de cada logro |
| 32 | POLÍTICA | Reputación: valor de cada evento, decaimiento y umbrales (`REPUTACION`), con la concentrada (C1b), el salto con la economía entera (EE-7) y la que no sube en el batch político (P6) | **Fase 7**, con bots que incumplan trueques |
| 32b | POLÍTICA | Caducidad de las propuestas de anexión y de fusión (`ANEXION.caducidadDias`, `FUSION.caducidadDias`) | playtest |
| 33 | COMERCIO | Tasa de comisión intermedia entre Facciones aliadas o vasallas de la misma Liga | batch |
| 34 | SUMINISTRO | La economía no llena los víveres de un ejército; los 350 por héroe | batch, tras el doble de la Granja y los víveres |
| 35 | MUNDO | Poder de los campamentos de bandidos: si escala con la región o con la cercanía de Facciones fuertes | batch |
| 35b | MILITAR | Gran Fundición: `CHATARRA.fraccion`, producción, coste y trabajadores | batch; alcanzarla pide batches de más semanas (P3, **Fase 7**) |
| 36 | HÉROE | Ritmo, prudencias y margen de los bots; cerebro «sin plaza» | **Fase 7** (bots en Rust) |
| 36b | MILITAR | Convocatoria del asedio y formación de ejércitos en campo (`BATALLA.convocatoriaSegundos`, `FORMACION_EJERCITO`) | playtest |
| 39 | MILITAR | Escala de XP de escuadra: Unity vs. números | espera a CQ-001 (Conquest) |
| 40 | MUNDO | Cifras de `MERCENARIOS` (placeholder) | **Fase 7** (C6) |
| 42 | TECNOLOGÍA | Cifras de `AEDAS` (retraso, itinerantes, estancia, precio de venta, residentes, épicas) | **Fase 7** (Q3, T1) |
| 18 | INTEL | Cifras de `INTEL` (Mirada, Informe, cupos, coste de la Taberna) | batch / playtest |
| 8 | CARAVANAS | Visibilidad por tamaño: umbral y radio (`VISION.caravanaGrande`) | batch / playtest |
| — | MURALLAS | Cifras de la empalizada (M2) | **Fase 7** |
| — | ECONOMÍA | Piezas gratuitas, almacén lleno y recetas sin penalización por distancia (2026-10-11) | batch |
| — | TERRITORIO | Bosque, metales aislados, cobertura de metales y Facciones sin plaza | batch; la cobertura y las Facciones sin plaza, **Fase 7** |
| 56 | ECONOMÍA | Arranque rápido de la plaza (Vivienda 10, Granja 5 y Leñera 2 minutos) y gate del nivel 2 con Granja a nivel 2 (2026-10-28): el nivel 2 llega a los 3,6 días de mediana en vez de 5,2 y los 200 pesants a los 0,36 días; decidir si es el ritmo buscado (`Economia_Construccion.md` §11.1) | `batch poblacion` |

## 20. Tecnología por Eras: calibrar los logros

**Fase 7** (decisión Q3 del autor, 2026-10-24; fila «Tecnología, Q3» de «Para la fase 7»). **Calibrar la X de cada
logro**: cada umbral, en lo que marque su contador en su semana objetivo (Doc 6.3). Las X se portaron tal cual del
TypeScript (las de las Eras II y III eran placeholder) y no se vuelven a medir hasta tener bots reales. Punto de partida:
`batch tecnologia` (6H, 2026-10-26) deja el día de cada logro y de cada Era con esas X (la Era II llega por plazo el
día 35; tabla en `Docs/Arquitectura/Sistema6_Contraste_y_Plan.md`, «Avance de 6H»). Lo mismo para la velocidad y la
estancia de los Aedas (§42). Bitácora del TypeScript: `typescript-final:Consideraciones/Tecnologia_Eras_I-III_Definicion.md`.

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

**Fase 7**: los bots se rehacen en Rust (`Consideraciones/Reconstruccion_Definicion.md` §4). Lo que había que calibrar
en los del TypeScript, como lista para los nuevos:

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

**Fase 7** (C6, 2026-10-19: mantener las cifras hasta medir con bots; fila de mercenarios de «Para la fase 7»). Las
cifras de `MERCENARIOS` (`Campamentos_Mercenarios.md`) son placeholder. Medido con `batch comercio` (2026-10-24): el
18,1 % de las compras se rechaza por falta de existencias (23,9 % con la semilla 7) con 219 obras de pueblo, con 1 viaje
de compra por obra y sin bloqueo (pila de 300 de madera, reposición cada 3 h, cupo diario de residentes de 60 de madera
y 10 de piedra). Desde el 2026-10-08 la tropa prestada sale **completa** (25-30 por escuadra, antes 15): el anillo de
bandidos se calculó contra 15 lanceros (`typescript-final:Consideraciones/Campamentos_Entrada_Fundacion_Definicion.md`
§423). Medir si los niveles de bandido siguen siendo un reto.

## 8. Caravanas: visibilidad por tamaño

Una comercial con **3 o más carros con animal** se ve a **300** de cualquier plaza o columna ajena, también en `preparando`
(`VISION.caravanaGrande` en `src/constants.ts`, Doc 5.12.7, D88): placeholder. Medir si el umbral y el radio generan
intercepciones sin que todas las caravanas sean visibles. Hecho el 2026-10-05.

## 42. Aedas: cifras

**Fase 7** (Q3 para la velocidad y la estancia de los itinerantes; T1 para las épicas, que en `batch tecnologia` no se
cumplen nunca porque el hito llega antes; filas de «Para la fase 7»). Las cifras de `AEDAS` (`Tecnologia_Aedas.md`) son
placeholder, sin calibrar: retraso de conocimiento (24 h), itinerantes por Facción (1 por cada 3, mínimo 3), estancia
(6 h), velocidad (20), precio de venta (2 × el oro de la tarifa), llegada de residentes (24 h), bonos de felicidad (5 %)
y de nobleza (25 %) por residente, y las épicas: cantidades de cada capítulo (`EPICAS`), enfriamiento (6 h) y carga
mínima de caravana. Medir cuánto adelantan las adopciones (Era I: nivel 2 a 5,8 días) y si la venta hace irrelevante el
hito, cuánto tardan las épicas en cumplirse con un solo residente (objetivo: más que el hito, menos que esperar a la Era
siguiente) y si algún hecho se farmea.

## 18. Taberna e intel: cifras

Las cifras de `INTEL` y de `EDIFICIO_CATALOGO.taberna` (`constants.ts`) son placeholder, sin calibrar: Mirada (radio 150, 2 h, base 40 de oro + 0,1 por unidad de distancia, cooldown de zona 2 h), Informe (60 de oro por nivel de la plaza mirada, cooldown 6 h por plaza), cupo de Miradas (1 / 2 / 3 por nivel de taberna, 1 en campamento) y el coste del edificio (150 madera + 60 piedra, mejoras 250+120 y 500+250). Es el **sink recurrente de oro** que la economía del oro necesita (`typescript-final:Consideraciones/Economia_Del_Oro_Definicion.md` §4.5): medir con batch (los bots ya construyen la taberna y compran informes) que `oroMedio` deje de subir sin techo y que el gasto en intel quede entre un tercio y la mitad del ingreso, sin impedir ninguna de las otras ambiciones; y si el precio por distancia basta para que mirar lejos sea una decisión. Plan: `typescript-final:Consideraciones/Taberna_Intel_Definicion.md`.

## 32. Reputación: cifras

**Fase 7** (C1 del 2026-10-19, C1b y EE-7 del 2026-10-24, P6 del 2026-10-25; filas de reputación de «Para la fase 7»).
`REPUTACION` (`Comercio_Caravanas.md`, canon 2.7) es placeholder: el valor de cada suceso (+5 por trueque cumplido, −8 por
incumplido, −10 por rebelión, −12 por romper una alianza, −25 por atacar a un aliado…), el decaimiento (0,002 por minuto
desde el 2026-10-18; era 0,2), el bonus por minuto de alianza (0,0005; era 0,05) y los umbrales (−40 para el Embajador y
para la comisión, ×1,5). Medir con bots que incumplan trueques que el score no se sature en ±100 ni quede pegado a 0, y
que los umbrales muerdan sin bloquear la diplomacia. Lo medido hasta ahora (las decisiones, en
`Consideraciones/Calibracion_Sistema4_Decisiones.md` y `Consideraciones/Politica_Tecnologia_Decisiones.md`):

- **`batch comercio`, 2026-10-19** (30 días, 146 plazas, 155 trueques cumplidos): el +5 dura 1,74 días; la máxima es
  78 (no satura), la mediana final 15,8 y solo el 17,5 % queda en positivo; nadie baja de 0 porque ningún trueque se
  incumple, así que el umbral de −40 no se ha ejercitado. **C1:** se dejan las cifras (descartadas por ahora bajar el
  decaimiento a 0,0005 o subir el umbral a −20).
- **Re-medido el 2026-10-24:** con el comercio concentrado en los días 7-9 una Facción llega a **99,7** (roza el tope) y
  la mediana final baja a 5,5 (**C1b**: esperar a la fase 7).
- **Con la economía entera (C3b, 2026-10-24;** 160 plazas, semilla 42): la mediana final **salta de 5,45 a 34,1** (máxima
  99,7 → 98,6; en positivo, 17,5 % → 15 %), con 139 → 136 trueques cumplidos y otro reparto de los peajes (**EE-7**: se
  investiga con C1b).
- **`batch politica`, 2026-10-25:** mediana final 0, mínima −46,5 (−45 con la semilla 7): las rupturas, las rebeliones y
  la traición la bajan y el bono por alianza activa no supera el decaimiento; 38 propuestas de alianza rechazadas por
  reputación baja (73 con la semilla 7) (**P6**).

## 33. Comisión intermedia entre aliadas y vasallas

Hoy `COMISION` tiene dos tasas: 3 % dentro de la Facción y 8 % fuera (Doc 3.5). Decidir con números si las Facciones aliadas o vasallas de la misma Liga pagan una tasa intermedia, y cuál; ¿alcanza la diferencia para que merezca la pena comerciar dentro de la Liga? Es la tasa a medir, no una mecánica nueva.

## 35. Campamentos de bandidos: escala del poder

Cada campamento sale con uno de tres niveles al azar, con su poder (`CAMPAMENTOS_BANDIDOS.niveles`, D21/D37). Medir con batch si el poder debe escalar además con la región o con la cercanía de Facciones fuertes, o si el azar de niveles basta.

## 35b. Gran Fundición: cifras

Implementada el 2026-10-06 (Doc 4.2.1). Son placeholder: `CHATARRA.fraccion` (0,5 del lingote del equipo por baja), las tres recetas de `EDIFICIO_CATALOGO.granFundicion` (producción base 4, 12 artesanos), el coste (150 madera + 100 piedra + 50 oro, 2 días) y los requisitos (Facción nivel 6, capital de asentamiento 4). Medir con batch cuánta chatarra deja una batalla típica, si el estaño que devuelve cambia de verdad el techo de las tropas de bronce, y si los 12 artesanos diluyen demasiado el ratio de mano de obra de la capital. Los bots no la construyen (`bots/cerebro/gobierno.ts`); decidir si deben, antes de medir. En 30 días de `batch politica` no se alcanza (capital de nivel 4): **P3** (2026-10-25), se medirá con batches de más semanas (**Fase 7**).

## 32b. Anexión y fusión: caducidad de la propuesta

Implementada el 2026-10-06 (Doc 2.6). `ANEXION.caducidadDias` = 3 días de mundo es un placeholder: lo que tarda el Rey de la absorbida en conectarse y
contestar. Con 5 jugadores en el playtest, mirar cuántas propuestas caducan sin respuesta; si son muchas, subir. Los bots rechazan al instante, así que
no entran en la medida. La fusión (2026-10-06) tiene su propia constante, `FUSION.caducidadDias` = 3, con el mismo criterio y la misma medida; si salen
distintas, que sean dos cifras.

## 36b. Batallas: convocatoria y formación en campo

Implementadas el 2026-10-06; las cifras son placeholder (`Batalla_Unity_Ciclo.md`, `Movimiento_Ejercitos.md`). El
`PERSECUCION.umbralRecalculo` del TypeScript se retiró (el recálculo es un evento, `Movimiento_Ejercitos.md` §7).

- `BATALLA.convocatoriaSegundos` = 30, también la del asedio con números desde M8 (2026-10-24). Mirar con jugadores si
  da tiempo a que lleguen los que estaban cerca; si no, alargarla.
- `FORMACION_EJERCITO.plazoMinutos` = 10 y `minimo` = 3. Con 5 jugadores en el playtest, ¿alcanzan a juntarse tres en diez minutos? Si no,
  subir el plazo antes que bajar el mínimo (el mínimo es decisión de diseño).

## Murallas: cifras de la empalizada

**Fase 7** (Murallas M2, 2026-10-24: se mantienen hasta medir con bots que compitan por la madera y con asedios; fila
«Cifras de la empalizada» de «Para la fase 7»). La empalizada se abre en el nivel 2, el muro de piedra en el 3 y el
adarve en el 4 (2026-10-09). Las tarifas por celda llana son acumuladas (Q4, 2026-10-22): 22 (empalizada), 52 (piedra) y
82 (adarve); antes 22 / 30 / 30, del TypeScript. Las del bloque `MURALLA` (`Murallas.md`): 20 de madera y 2 de piedra por
celda, puertas ×4, 15 min por celda, upkeep de 0,02 de madera por celda y minuto.

**Medido (`batch poblacion`, 2026-10-24;** informe en `batch/salida/poblacion/`, decisiones M1 y M2 en
`crates/batch/decisiones/poblacion.json`). El jugador compromete la empalizada en cuanto la plaza tiene el nivel operativo
2: el 100 % de las plazas de nivel 2 la compromete (día 5,25) y la cierra en **0,55 días** (54 celdas y 1 puerta de
mediana, sin esperar nunca materiales: almacenes llenos). Cuesta **1.180 de madera** (1.160 antes de M1) y unos 116 de
piedra; su upkeep es el **10,8 %** del mantenimiento de la plaza. Tras M1 (`LAYOUT_VERSION` 5, `Murallas.md` §3) ninguna
plaza amurallada deja de crecer ni queda pieza sin salida (antes, el 7,1 % y 39 piezas). Sin medir: la mejora a piedra
(nivel 3, que ningún batch alcanza), la reparación tras una conquista y el efecto en los asedios.

## Economía: piezas gratuitas, almacén lleno y sin penalización por distancia (2026-10-11)

Los puestos del Mercado y las anclas pagan mantenimiento a la mitad (antes no contaban nada especial: peso 1); un almacén lleno pausa extractores y talleres en vez de desperdiciar; la Gran Fundición cuenta como mano de obra de artesanos; y desaparece la penalización por distancia de las recetas (que daba ×0,688 a toda la cadena de metal). Todo empuja a más producción y menos coste: medir con batch si la economía se acelera de más. Ver `Docs/Mechanics/Economia_Construccion.md`.

## Territorio: bosque, parcelas y metales aislados (2026-10-18)

Sistema 1 de la fase 6 (mundo y territorio) implementado; el informe de batch está en `batch/salida/territorio/` (se regenera con `batch territorio --decisiones crates/batch/decisiones/territorio.json`). Tras rescalar el mundo (piedra ×10 = 5.601 nodos, parcelas con `rp` = 4 = 23.877, metales 64 de estaño y 24 de oro) queda sin cerrar:

- **Bosque (dejado así por el autor):** el 21 % de las plazas no alcanza ninguna parcela con su zona máxima (objetivo ≤ 10 %) y solo se concede el 44 % de las Leñeras (objetivo ≥ 90 %; el 56 % se niega por «sin parcelas libres en la zona»). No falta bosque en total (3,8 % de las parcelas ocupadas): las parcelas se agrupan en masas y las zonas de 6-30 u caen a menudo fuera. Antes de rescalar otra vez, medir cuántas parcelas hay a ≤ 30 u de cada plaza y si las plazas sin parcela están lejos de toda masa; si hace falta, probar `rp` = 3 (otros ~12 min de generación) o dejar el 44 %.
- **Separación de metales aislados (`PARAMETROS_METALES.espacioAislado`):** bajada de 120 a 100 u porque con 64 de estaño (16 en frontera) la semilla 42 no admitía 16 nodos a 110. No está en el canon; si se cambia la semilla o el reparto por anillos, revisarla.
- **Cobertura de metales por la población sintética (Fase 7):** estaño 67 / 38 / 0 % (foco / frontera / lejana) y oro 100 / 33 / 0 %: el batch nunca se aleja más de 120 u del campamento; el gradiente 5 / 25 / 70 se mide de verdad con los bots de la fase 7.
- **Facciones sin plaza (26 %, Fase 7):** 400 Facciones sobre 9 campamentos saturan el foco (105 %); un jugador real buscaría más lejos. Vigilar con los bots.
