# Tanda de economía, héroe y cultivos (2026-11-08)

> **Diseño en curso con el autor, sin implementar** (vive en `Docs/Disenos_Por_Implementar/` hasta que se implemente; ver su README). Entrada 69 de `Docs/Mecanicas a desarrollar.md`. Contrastado con el motor en `main` ef5db11. Las cifras marcadas «placeholder» son del estudio y las cambia el autor. Lo marcado **Abierto** espera respuesta. Revisado entero el 2026-11-08 por la tarde con las respuestas del autor. **No se implementa nada hasta que esté todo cerrado** (autor, 2026-11-08).

## 0. Orden de implementación

| Tanda | Contenido | Toca | Partida nueva | Depende de |
|---|---|---|---|---|
| 1 | Bandidos que dan recursos (§1), refundación sin oro (§2), requisitos de población y techos al doble (§3) | motor | sí (reglas) | — |
| 2 | Moneda única del héroe: fuentes y usos (§4), mercenarios por contrato a 1/4 del oro o en monedas (§5) | motor, contrato, Unity, clientes | sí | — |
| 3 | Oro de peajes, comisiones y compras desde la capital (§6), información regional de la taberna y toma de órdenes a distancia (§7), mercenarios de la región en la taberna (§8) | motor, contrato, cliente | no | 2; entrada 65 (informes por tipo) |
| 4 | Prestigio (§13) | motor, contrato, cliente | no | — |
| 5 | Recursos nuevos en el contrato (los nueve de §9-§12), bienes por clase (§9) y ciclo del pan (§10) | motor, contrato, clientes | sí | — |
| 6 | Carne, leche y queso (§11) | motor | sí | 5 |
| 7 | Vid y oliva: fertilidad, aldeas de un cultivo, vino y aceite (§12) | generador, motor, cliente | sí (mundo nuevo) | 5 |
| aparte | Puertas de la muralla en línea recta (§14) | `trazado` | sí (`LAYOUT_VERSION`) | — |

- El prestigio sube del puesto 7 al 4: no depende de la comida (combate, conquista, comercio, fundación y nivel ya existen) y la cadena de alimentos es la parte más grande.
- Los nueve recursos nuevos (harina, pan, carne, leche, queso, uva, aceituna, vino, aceite) entran **todos a la vez** en el enum `Recurso`, el `.proto`, los dorados, los precios de referencia y la capacidad del almacén al empezar la tanda 5, para que clientes, bots y admin se adapten una sola vez. Se comercian como el resto (órdenes, trueques, caravanas).
- Las tandas 1, 2 y 5 a 7 piden partida nueva: conviene agruparlas para no reiniciar varias veces.

## 1. Bandidos que dan recursos

Hoy: `CAMPAMENTOS_BANDIDOS.niveles.N.oroPorHeroe` (30 / 33 / 36) de oro-recurso al carro, con rendimientos decrecientes en 24 h; el anillo de un campamento de mercenarios solo paga a los héroes sin plaza (Combate §7.2).

Cambio (autor, 2026-11-08):
- El campamento caído **deja de dar oro**. Cada héroe que combatió recibe **madera, piedra y trigo**, de los tres: **5** en el nivel 1, **7** en el 2 y **10** en el 3, al carro, lo que quepa.
- Siguen los rendimientos decrecientes de 24 h por héroe y la regla del anillo (solo cobran los héroes sin plaza; los demás, solo experiencia).
- Vale igual en todos los caminos que hoy pagan el oro: ataque con números, Unity, emboscada a una columna con caravanas y caravana suelta con escolta cedida (el botín viaja en la caravana).
- Además, **monedas del héroe** en poca cantidad (§4.2), desde la tanda 2.
- El hueco «oro del anillo» de la entrada 68 pasa a ser «recursos del anillo».
- **Consecuencia a vigilar:** sin el oro de los bandidos, el héroe sin plaza solo gana oro con los alijos y las misiones de héroe, y en el campamento lo sigue gastando (reclutar tropa, mercado). No se cambia nada; se mide con el batch al implementar.

## 2. Refundación sin oro

La caravana que compra en el campamento una Facción sin plazas (`ComprarCaravanaDeRefundacion`) deja de pedir oro: queda en **64 de madera, 8 de piedra y 40 de trigo** (hoy, además, 40 de oro). La Caravana de Fundación que sale de una plaza **no cambia**. Con §1, lo que dan los bandidos va directo al fondo de refundación (unos 7 a 13 campamentos para la madera).

## 3. Requisitos de población y techos al doble

En lugar de bajar el crecimiento (descartado por el autor), se dobla **solo la población** de los requisitos de `NIVEL_ASENTAMIENTO` (edificios, Granja a nivel 2, pueblo activo, coste y obra del ascenso no cambian), y se doblan **todos los techos** de población:

| Nivel | Pesants | Artesanos | Techo |
|---|---|---|---|
| 1 | — | — | 300 → 600 |
| 2 | 200 → 400 | 0 | 1.500 → 3.000 |
| 3 | 500 → 1.000 | 400 → 800 | 6.000 → 12.000 |
| 4 | 1.000 → 2.000 | 800 → 1.600 | 12.000 → 24.000 |
| 5 | 2.000 → 4.000 | 1.600 → 3.200 | 20.000 → 40.000 |

Las **viviendas por nivel** se doblan solas: su tope ya sale de los requisitos del nivel siguiente (y del techo en el nivel máximo), Economía §6.5:

| Nivel | Viviendas hoy | Con el cambio |
|---|---|---|
| 1 | 16 | 29 |
| 2 | 42 | 82 |
| 3 | 82 | 162 |
| 4 | 162 | 322 |
| 5 | 1.334 | 2.667 |

**A medir al implementar:** el doble de viviendas ocupa el doble de ciudad (cabida del trazado en la zona y separación de 20 u entre plazas) y sube el mantenimiento por edificio.

## 4. Moneda única del héroe

### 4.1 Una sola moneda
`Monedas { bronce, plata, oro }` pasa a un solo saldo, **monedas**: monedas sueltas, distintas del oro-recurso de las plazas (metal en bruto). **Equivalencia: 100 monedas = 1 de oro** (autor), la que usan los precios que admiten las dos. Partida nueva sin convertir saldos. El botín de Unity da un solo número (cambia el contrato: proto, `codec_batallas`, Unity).

### 4.2 De dónde salen (cifras placeholder, a la escala de 100 = 1 de oro)
- **Saldo inicial:** 1.500 (lo que cuesta un contrato de un día de un bot de nivel 1, §5).
- **Héroe contra héroe:** todo combate con héroes de **dos Facciones distintas** (columnas, asedio, pueblos, avanzadas, caravanas con escolta y Unity). Cobran los dos bandos: **500** por héroe ganador y **150** por perdedor. Rendimientos decrecientes por **par de Facciones** en una ventana de 24 h, para que no se pueda farmear entre amigos.
- **Bandidos:** **50 / 75 / 100** por héroe según el nivel, con su ventana.

### 4.3 En qué se gastan
Solo en cosas del héroe: objetos (cuando exista el catálogo, CQ-004) y el pago de **mercenarios por contrato** (§5), que admite oro o monedas.

## 5. Mercenarios por contrato: 1/4 del oro o monedas

Solo los **bots por contrato** (Mercenarios_Por_Contrato §5.1); la tropa que se recluta en el campamento no cambia.
- La tarifa en oro pasa a **1/4** de la de hoy.
- Se añade la **tarifa en monedas** = tarifa en oro (ya reducida) × 100. Quien contrata paga **una de las dos**, a su elección (`pagar_con` gana la opción monedas, de las del héroe que firma).
- Las monedas pagadas van a las **monedas del mercenario**, como el oro va a su almacén personal.
- Orden de magnitud: un bot de nivel 1 con 30 lanceros vale unos 400 de oro; su contrato de un día cuesta hoy unos 60 de oro y pasaría a unos 15 de oro o 1.500 monedas.

## 6. El oro del comercio sale de la capital y entra en la capital

Hoy el peaje es el 2 % de la carga **en especie** y va a la ciudad cruzada; la comisión se acuña en el almacén del destino; lo que compra una caravana en un mercado ajeno lo paga con el oro que lleva.

Cambio (autor, 2026-11-08): **la caravana no lleva oro para nada de esto.**
- **Peaje:** el 2 % del **valor** de la carga (precio de referencia), en **oro**, que sale del almacén de la **capital** de la Facción dueña de la caravana y entra en la **capital** de la Facción de la ciudad cruzada.
- **Comisión:** se acuña en la **capital** de la Facción del destino.
- **Compras y ventas en un mercado ajeno** (Comercio §5.8.8 y §7 de aquí): el oro de una compra sale de la capital del comprador y el de una venta entra en ella.
- Si la capital no tiene oro bastante para un peaje, paga lo que pueda y el resto **en especie de la carga**, como ahora. Una compra sin oro en la capital se sirve hasta donde llegue.
- Lo que no cabe en el almacén de la capital que cobra se pierde, como hoy.
- Una Facción sin capital (o un héroe sin Facción) paga y cobra en la plaza de origen y en la cruzada, como ahora.

## 7. Información regional de la taberna y toma de órdenes a distancia

**Región = región del relieve del generador** (`regionEn`: Altiplano interior, Estepa abierta…).

La taberna de la plaza **vende información**, como la Mirada y los informes (encaja como dos tipos nuevos del catálogo de informes por tipo de la entrada 65):
1. **Las plazas neutrales de la región.**
2. **Las órdenes de compra y venta** de esas plazas.

Con esa información comprada, **desde el Mercado** de la plaza se puede tomar a distancia cualquiera de esas órdenes, **de compra y de venta**: la toma lanza una caravana libre de la plaza con un encargo a esa plaza (Comercio §5.8.8), y **la mercancía viaja**; el oro, no (§6). La orden no se reserva; se liquida al llegar con lo que quede.

- **Qué plazas entran** (autor, 2026-11-08): **todas menos las de Facciones enemigas** (en guerra), también las aliadas y vasallas. Las plazas aliadas y vasallas ya se ven por la visión compartida (Niebla §5), pero sus órdenes no: hoy las órdenes de una plaza ajena solo llegan con el héroe dentro o con una caravana propia esperando en ella (`proyeccion/vista.rs`, `ordenes_activas`).
- **Precio, enfriamiento y caducidad** (autor, 2026-11-08): como el Informe de plaza; la información caduca a los `plazoOrdenMinutos` (200 min). La compran el Rey, el Embajador o el Gobernador.

## 8. Mercenarios de la región en la taberna

La taberna de la plaza enseña el **tablón de mercenarios** de los campamentos de su región (los bots ofrecidos, con ficha, tropa y tarifas) y permite **contratarlos** desde allí, sin ir en persona: paga el almacén de la plaza (oro) o el héroe que firma (monedas, §5). El mercenario contratado se pone en camino solo.

- **Ver el tablón es gratis** (autor, 2026-11-08), como en el campamento.

## 9. Bienes por clase de población

Los bienes de comida nuevos **alimentan** y además dan un **modificador de crecimiento** propio de cada clase (autor, 2026-11-08). Los **víveres de movimiento** (columnas, ejércitos, héroes) **solo** usan trigo.

| Bien | Quién lo consume | Modificador (ración entera) | Ración por cabeza y minuto (placeholder) |
|---|---|---|---|
| Trigo | todos, también los víveres | — (comida base) | lo que falte hasta la necesidad |
| Carne | todas las clases | ×1,2 a todas | 0,02 |
| Pan | pesants | ×2 | 0,04 |
| Queso | artesanos | ×1,5 (placeholder) | 0,02 |
| Vino | nobleza | ×1,5 (placeholder) | 0,01 |
| Aceite | nobleza | ×1,5 (placeholder) | 0,01 |
| Leche | nadie: solo es materia prima de la quesera (autor, 2026-11-08) | — | — |

**El consumo mezclado, cada minuto y en cada plaza** (propuesta del estudio sobre la decisión del autor):
1. Cada clase tiene su **necesidad de comida**, la de hoy (`consumoComidaPorHabitante`, 0,1 por habitante y minuto, por las políticas).
2. Primero consume sus **bienes propios**, cada uno hasta su ración por cabeza.
3. Cada bien consumido **alimenta 1 a 1**, como el trigo; lo que falte hasta la necesidad lo pone el **trigo**. Las raciones de una clase suman menos que su necesidad: el trigo siempre hace falta.
4. **Cobertura proporcional (autor, 2026-11-08):** cada bien da su modificador en proporción a lo consumido de su ración en ese minuto: `1 + (M − 1) × consumido / ración`. El pan con media ración da ×1,5. Los modificadores se **multiplican** entre sí y con los factores de crecimiento de hoy.
5. Un bien que no llega para todos se reparte **en proporción** entre las clases que lo consumen.
6. La **nutrición y el hambre** se miden como hoy, con la necesidad cubierta por todo lo consumido (bienes y trigo).

Ejemplo: 400 pesants necesitan 40 por minuto y quieren 16 de pan y 8 de carne. Con solo 8 de pan en el almacén: pan ×1,5, carne ×1,2 y 24 de trigo; crecen ×1,8.

- **La leche** no se consume: solo es materia prima de la quesera (autor, 2026-11-08).
- **La guarnición** come solo **trigo y carne** (autor, 2026-11-08): no crece, así que los bienes no le dan nada, y así no le quita a la población el pan, el queso, el vino ni el aceite.

## 10. Ciclo del pan

- **Molino:** trigo → **harina**. Admite **un buey** asignado, que se compra como el de una caravana (12 de oro; autor, 2026-11-08) y se queda allí; dobla su producción.
- **Horno:** harina → **pan**.
- Los dos desde el nivel 1 de la plaza; harina y pan se comercian como el resto. El pan lo consumen los pesants (§9).
- **Sin tecnología** (autor, 2026-11-08): los edificios nuevos (molino, horno, quesera, prensa y bodega) piden **solo el nivel de la plaza**; el árbol de tecnologías no cambia.

## 11. Carne, leche y queso

Retoma la idea 53 de `Ideas a diseñar.md`.
- De los **corrales** de la aldea: la **leche** sale del rebaño sin sacrificar; la **carne**, al sacrificar, junto al ganado de hoy (que sigue haciendo falta para el cuero y la Maravilla).
- La **carne** alimenta como el trigo (1 a 1) y da ×1,2 al crecimiento de todas las clases (§9); no vale para los víveres de movimiento.
- **Quesera** (nivel 2 de la plaza): leche → **queso**, que consumen los artesanos (§9).
- El carro de la aldea lleva ya dos recursos (trigo y ganado); con carne y leche, cuatro.
- El consumo de la carne, la leche y el queso es el de §9.

## 12. Vid y oliva

- **Fertilidad de la vid y de la oliva:** dos capas nuevas del generador, junto a la de hoy, y pintadas en el cliente. Mundo nuevo.
- **Aldeas de un solo cultivo:** al marcar la aldea se elige su cultivo (trigo, vid u oliva) y no cambia; todas sus granjas son de ese cultivo y su sitio mira la fertilidad de ese cultivo. Vid desde el nivel 3 de la plaza madre; oliva desde el 4. Los corrales, solo en las aldeas de trigo. La Granja de la ciudad sigue siendo de trigo.
- Las granjas de vid dan **uva**, y las de oliva, **aceituna**.
- **Prensa** (autor): aceituna → **aceite**.
- **Bodega** (autor, 2026-11-08): uva → **vino**.
- Vino y aceite los consume la nobleza (§9). Bodega y prensa, desde el nivel de su cultivo (3 y 4).

## 13. Prestigio

- **De Facción, acumulado; no se gasta ni se pierde.**
- **Se gana** (placeholders): victoria contra héroes de otra Facción 5 por batalla (no bandidos), conquista de una plaza 50, fundación 20, subida de nivel de una plaza 10 × nivel nuevo, y comercio entre Facciones distintas (trueque cumplido u orden tomada) 2.
- **Se usa por umbrales** (placeholders):
  - **Alianzas abiertas** a la vez: 1, y una más cada 100 de prestigio (hoy no hay tope).
  - **Estatuas** en la capital: **decorativas**, desbloqueadas con prestigio (200, 500 y 1.000).
  - **Maravilla** en la plaza de nivel 5: ya pide el nivel 5 (`requisitoNivelAsentamientoConstruccion`); su `ordenEraMinimo` 5 (la Era V, que no existe) se sustituye por un umbral de prestigio (2.000).
- **Cifras aprobadas por el autor (2026-11-08)** como punto de partida; se calibran con el batch.

## 14. Puertas de la muralla en línea recta

Hoy las puertas salen donde una calle o un camino cruza el anillo (máx. 4, 2 celdas de ancho), pero el muro a su lado puede quebrarse justo en la puerta.

Regla nueva (autor, 2026-11-08): cada puerta tiene **al menos 2 celdas de muro en línea recta a cada lado**, en la misma fila (o columna) que la puerta; una puerta de 2 celdas queda en un tramo recto de 6:

```
x-------
-xxpxx--
-------x
```

Toca `crates/trazado` (el trazo del anillo, junto a `estrecharPuertas`/`limitarPuertas`), sube `LAYOUT_VERSION` y pide partida nueva. Las murallas ya trazadas no cambian (las puertas están congeladas).

- **Cuando el anillo no tiene el tramo recto** donde va la puerta, **se endereza el muro** a su alrededor, para que la puerta quede donde llega el camino (autor, 2026-11-08: «es primordial»). Puede encerrar o dejar fuera alguna celda; la puerta no se mueve.
