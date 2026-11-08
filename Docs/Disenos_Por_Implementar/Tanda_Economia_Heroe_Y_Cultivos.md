# Tanda de economía, héroe y cultivos (2026-11-08)

> **Diseño en curso con el autor, sin implementar** (vive en `Docs/Disenos_Por_Implementar/` hasta que se implemente; ver su README). Entrada 69 de `Docs/Mecanicas a desarrollar.md`. Contrastado con el motor en `main` ef5db11. Las cifras marcadas «placeholder» son del estudio y las cambia el autor. Lo marcado **Abierto** espera respuesta.

## 0. Orden de implementación

| Tanda | Contenido | Toca | Partida nueva |
|---|---|---|---|
| 1 | Bandidos que dan recursos (§1), refundación sin oro (§2), requisitos de nivel al doble (§3) | motor | sí (reglas) |
| 2 | Moneda única del héroe, sus fuentes y sus usos (§4), mercenarios por contrato a 1/4 del oro o en monedas (§5) | motor, contrato, Unity, clientes | sí |
| 3 | Peajes y acuñado desde la capital (§6), taberna regional y compra a distancia (§7), mercenarios en la taberna (§8) | motor, contrato, cliente | no |
| 4 | Bienes por clase (§9) y ciclo del pan (§10) | motor, contrato, clientes | sí |
| 5 | Carne, leche y queso (§11) | motor | sí |
| 6 | Vid y oliva: fertilidad, aldeas de un cultivo, vino y aceite (§12) | generador, motor, cliente | sí (mundo nuevo) |
| 7 | Prestigio (§13) | motor, contrato, cliente | no |
| aparte | Puertas de la muralla en línea recta (§14) | `trazado` | sí (`LAYOUT_VERSION`) |

Los recursos nuevos de las tandas 4 a 6 (harina, pan, carne, leche, queso, uva, aceituna, vino, aceite) entran **todos a la vez** en el enum `Recurso`, el `.proto` y los dorados al empezar la tanda 4, para que clientes, bots y admin se adapten una sola vez. Cada uno lleva precio de referencia y se comercia como el resto (órdenes, trueques, caravanas).

## 1. Bandidos que dan recursos

Hoy: `CAMPAMENTOS_BANDIDOS.niveles.N.oroPorHeroe` (30 / 33 / 36) de oro-recurso al carro, con rendimientos decrecientes en 24 h; el anillo de un campamento de mercenarios solo paga a los héroes sin plaza (Combate §7.2).

Cambio (autor, 2026-11-08):
- El campamento caído **deja de dar oro**. Cada héroe que combatió recibe **madera, piedra y trigo**, de los tres: **5** en el nivel 1, **7** en el 2 y **10** en el 3, al carro, lo que quepa.
- Siguen los rendimientos decrecientes de 24 h por héroe y la regla del anillo (solo cobran los héroes sin plaza; los demás, solo experiencia).
- Vale igual para todos los caminos que hoy pagan el oro: ataque con números, Unity, emboscada a una columna con caravanas y caravana suelta con escolta cedida (el botín viaja en la caravana).
- Además, **monedas del héroe** en poca cantidad (§4.2), desde la tanda 2.
- Afecta al hueco «oro del anillo» de la entrada 68 (que pasa a ser «recursos del anillo»).

## 2. Refundación sin oro

La caravana que compra en el campamento una Facción sin plazas (`ComprarCaravanaDeRefundacion`) deja de pedir oro: queda en **64 de madera, 8 de piedra y 40 de trigo** (hoy, además, 40 de oro). La Caravana de Fundación que sale de una plaza **no cambia**. Con §1, lo que dan los bandidos va directo al fondo de refundación.

## 3. Requisitos de subida de nivel de plaza al doble

En lugar de bajar el crecimiento (descartado por el autor), se doblan los requisitos de población de `NIVEL_ASENTAMIENTO`:

| Nivel | Pesants | Artesanos |
|---|---|---|
| 2 | 200 → 400 | 0 |
| 3 | 500 → 1.000 | 400 → 800 |
| 4 | 1.000 → 2.000 | 800 → 1.600 |
| 5 | 2.000 → 4.000 | 1.600 → 3.200 |

Los edificios, la Granja a nivel 2 y el pueblo activo no cambian (no se pueden «doblar»).

**Abierto 3.1: el techo de población.** Con el techo de hoy (`techoPoblacion`: 300 en el nivel 1, 1.500 en el 2) **no se podría subir** al 2 (pide 400) ni al 3 (pide 1.800). Propuesta: subir el techo del nivel 1 a 600 y el del 2 a 3.000 (el doble también), dejando 3-5 como están, que ya bastan.

**Abierto 3.2:** ¿se doblan también otras cosas del ascenso (coste u obra del ascenso), o solo la población?

## 4. Moneda única del héroe

### 4.1 Una sola moneda
`Monedas { bronce, plata, oro }` pasa a un solo saldo, **monedas**, sin relación con el oro-recurso de las plazas. Partida nueva sin convertir saldos; el héroe nace con **50** (placeholder; hoy 500 de bronce). El botín de Unity da un solo número (cambia el contrato: proto, `codec_batallas`, Unity).

### 4.2 De dónde salen
- **Héroe contra héroe:** todo combate con héroes de **dos Facciones distintas** (columnas, asedio, pueblos, avanzadas, caravanas con escolta y Unity). Cobran los dos bandos y el ganador más: **10** por héroe ganador y **3** por perdedor (placeholders). Rendimientos decrecientes por **par de Facciones** en una ventana (como la de los bandidos) para que no se pueda farmear entre amigos.
- **Bandidos:** 1, 2 o 3 monedas por héroe según el nivel (placeholder), con su ventana.

### 4.3 En qué se gastan
Solo en cosas del héroe: objetos (cuando exista el catálogo, CQ-004) y el pago de **mercenarios por contrato** (§5), que admite oro o monedas.

## 5. Mercenarios por contrato: 1/4 del oro o monedas

Solo los **bots por contrato** (Mercenarios_Por_Contrato §5.1); la tropa que se recluta en el campamento no cambia.
- La tarifa en oro pasa a **1/4** de la de hoy.
- Se añade una **tarifa en monedas**. Quien contrata paga **una de las dos**, a su elección (`pagar_con` gana la opción monedas, de las del héroe que firma).
- **Abierto 5.1:** la equivalencia oro ↔ monedas (propuesta: 1 moneda = 2 de oro de la tarifa ya reducida, placeholder) y adónde van las monedas pagadas (propuesta: a las monedas del mercenario, como el oro va a su almacén personal).

## 6. Peajes y acuñado desde la capital

Hoy el peaje es el 2 % de la carga **en especie** y va a la ciudad cruzada; la comisión se acuña en el almacén del destino.

Cambio (autor, 2026-11-08):
- **Peaje:** el 2 % del **valor** de la carga (precio de referencia), en **oro**, que sale del almacén de la **capital** de la Facción dueña de la caravana y entra en la **capital** de la Facción de la ciudad cruzada. La caravana no lleva oro para pagarlo.
- Si la capital no tiene oro bastante, paga lo que pueda y el resto **en especie de la carga**, como ahora.
- **Comisión:** se acuña en la **capital** de la Facción del destino.
- También lo que compra una caravana en un mercado ajeno (§5.8.8): el oro sale de la capital, no del carro.
- **Abierto 6.1:** una Facción sin capital (o una plaza sin Facción) paga y cobra en la plaza de origen y en la cruzada, como ahora.

## 7. Taberna regional y compra a distancia

**Región = región del relieve del generador** (`regionEn`: Altiplano interior, Estepa abierta…).

La taberna de la plaza **vende información** (como la Mirada y el Informe):
1. **Las plazas neutrales de la región** (de Facciones sin alianza, vasallaje ni guerra con la del comprador).
2. **Las órdenes de compra y venta** de esas plazas.

Con esa información comprada, **desde el Mercado** de la plaza se puede tomar a distancia cualquiera de esas órdenes, **de compra y de venta**: la toma lanza una caravana libre de la plaza con un encargo a esa plaza (Comercio §5.8.8), y **la mercancía viaja**. La orden no se reserva; se liquida al llegar con lo que quede.

**Abierto 7.1:** precio, enfriamiento y caducidad de esa información (propuesta: como el Informe de plaza, y que caduque a los `plazoOrdenMinutos` de las órdenes, 200 min), y quién la compra (propuesta: Rey, Embajador o Gobernador, como hoy).

## 8. Mercenarios del campamento de la región en la taberna

La taberna de la plaza enseña el **tablón de mercenarios** de los campamentos de su región (los bots ofrecidos, con ficha, tropa y tarifas) y permite **contratarlos** desde allí, sin ir en persona: paga el almacén de la plaza (oro) o el héroe que firma (monedas, §5). El mercenario contratado se pone en camino solo.

## 9. Bienes por clase de población

Un marco general: un bien de consumo multiplica el crecimiento de una clase cuando esa clase lo consume (por habitante y por minuto, como el trigo).

| Bien | Clase | Efecto |
|---|---|---|
| Pan | pesants | ×2 |
| Queso | artesanos | placeholder ×1,5 |
| Vino | nobleza | placeholder ×1,5 |
| Aceite | nobleza | placeholder ×1,5 |

Los efectos se multiplican entre sí (aprobado).

**Abierto 9.1: cobertura parcial.** Si el almacén solo tiene pan para la mitad de los pesants ese minuto, ¿qué pasa? (a) **todo o nada**: o hay pan para todos y crecen ×2, o no hay bonus; (b) **proporcional**: con pan para la mitad crecen ×1,5, para un cuarto ×1,25. Recomendación: (b), porque con (a) una plaza grande pierde todo el bonus por un pan que falte.

## 10. Ciclo del pan

- **Molino:** trigo → **harina**. Admite un **buey** asignado (se queda allí) que dobla su producción.
- **Horno:** harina → **pan**.
- Los dos desde el nivel 1 de la plaza; harina y pan se comercian como el resto.
- El pan lo consumen los pesants (§9).

## 11. Carne, leche y queso

Retoma la idea 53 de `Ideas a diseñar.md`.
- De los **corrales** de la aldea: la **leche** sale del rebaño sin sacrificar; la **carne**, al sacrificar, junto al ganado de hoy.
- La **carne alimenta como el trigo y rinde 1,2×** (una unidad de carne cubre 1,2 de trigo en el consumo de la población, la guarnición y los víveres).
- **Quesera** (nivel 2 de la plaza): leche → **queso**, que consumen los artesanos (§9).

## 12. Vid y oliva

- **Fertilidad de la vid y de la oliva:** dos capas nuevas del generador, junto a la de hoy, y pintadas en el cliente. Mundo nuevo.
- **Aldeas de un solo cultivo:** al marcar la aldea se elige su cultivo (trigo, vid u oliva) y no cambia; todas sus granjas son de ese cultivo y su sitio mira la fertilidad de ese cultivo. Vid desde el nivel 3 de la plaza madre; oliva desde el 4. Los corrales solo en las aldeas de trigo. La Granja de la ciudad sigue siendo de trigo.
- **Uva y aceituna** son lo que dan esas granjas.
- **Lagar** (uva → **vino**) y **almazara** (aceituna → **aceite**): los edificios de la plaza para el vino y el aceite, que consume la nobleza (§9). Niveles: los de su cultivo (placeholder).

## 13. Prestigio

- **De Facción, acumulado; no se gasta ni se pierde.**
- **Se gana:** combatiendo (no contra bandidos), conquistando, comerciando, fundando y subiendo de nivel plazas. Cifras por calibrar.
- **Se usa por umbrales:**
  - Número de **alianzas abiertas** a la vez (hoy no hay tope).
  - **Estatuas** en la capital: **decorativas**, se desbloquean con prestigio.
  - **Maravilla** en la plaza de nivel 5: pide nivel 5 y un umbral de prestigio en lugar de la Era V, que no existe.
- **Abierto 13.1:** qué cuenta como «comerciando» (propuesta: trueque cumplido y orden tomada entre Facciones distintas) y las cifras de cada fuente y de cada umbral.

## 14. Puertas de la muralla en línea recta

Hoy las puertas salen donde una calle o un camino cruza el anillo (máx. 4, 2 celdas de ancho), pero el muro a su lado puede quebrarse en diagonal justo en la puerta.

Regla nueva (autor, 2026-11-08): cada puerta tiene **al menos 2 celdas de muro en línea recta a cada lado**, en la misma fila (o columna) que la puerta:

```
x-------
-xxpxx--
-------x
```

Toca `crates/trazado` (el trazo del anillo, `estrecharPuertas`/`limitarPuertas`), sube `LAYOUT_VERSION` y pide partida nueva. Las murallas ya trazadas no cambian (las puertas están congeladas).
