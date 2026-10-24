# Economía entera: diseño e implementación (C3b, 2026-10-24)

Decisión del autor (punto **C3b** de [`Consideraciones/Calibracion_Sistema4_Decisiones.md`](../../Consideraciones/Calibracion_Sistema4_Decisiones.md)): **todo lo almacenado va en enteros**. Ningún almacén guarda decimales; los flujos por minuto acumulan un **resto oculto** determinista, como la población entera, y entran o salen unidades enteras; lo discreto (comercio, entregas, comisiones, acuñación, botín, saqueo, reparto de carga entre carros, cualquier fracción) **se redondea hacia abajo**: nunca se crea de más.

Este documento es el diseño (qué cambia, dónde vive cada resto, cómo se redondea cada operación) y el estado de lo hecho. Las reglas de juego están en Mechanics: [`Economia_Construccion.md`](../Mechanics/Economia_Construccion.md) §4, [`Comercio_Caravanas.md`](../Mechanics/Comercio_Caravanas.md) §1, [`Combate.md`](../Mechanics/Combate.md) §1 y [`Heroe_Y_Tropa.md`](../Mechanics/Heroe_Y_Tropa.md) §1; en el canon, Doc 4.3 y 3.5.

## 1. Representación: milésimas múltiplo de 1.000

Las cantidades siguen siendo `base::Mil` (milésimas, `i64`) en todo el código, en la foto y en el contrato; lo nuevo es el **invariante**: toda cantidad **almacenada** es múltiplo de `base::UNIDAD` (1.000). No se cambia el tipo por tres razones: el contrato v1 ya viaja en milésimas y un cambio de unidad sería incompatible (el `buf breaking` debe quedar limpio), las tasas, capacidades y factores siguen necesitando fracciones (una capacidad de 750, una producción de 0,37 por minuto) y el mismo `Mil` sirve para lo exacto (el resto) y para lo visible (las unidades). Dos funciones en `base` lo hacen explícito:

| Función | Qué hace | Para qué |
|---|---|---|
| `base::entero(x)` | redondea hacia abajo a unidades (`floor`) | todo lo que entra, sale, se pacta, se lleva o se reparte |
| `base::entero_arriba(x)` | redondea hacia arriba a unidades (`ceil`) | lo que se **cobra**: un coste, un precio en oro |

## 2. Inventario: dónde se guardan cantidades

| Contenedor | Tipo | Flujo continuo | Resto oculto |
|---|---|---|---|
| Almacén de plaza | `asentamiento::Almacen` | producción (granjas, Leñeras, extractores), recetas (producto e insumos), comida de la población y de la guarnición, mantenimiento, recaudación | **sí**, uno por recurso, en el `Almacen` |
| Almacén local de un pueblo extractor | `Almacen` | extracción del nodo | **sí** |
| Almacén de una avanzada (y de una huérfana) | `Almacen` | ninguno (solo depósitos, retiradas, reabastecer y saqueo) | lo tiene (es el mismo tipo), siempre 0 |
| Carro de una columna (`Columna::carga`) y del héroe (`HeroeNucleo::carro`) | `Carga` (`BTreeMap<Recurso, Mil>`) | ninguno | no |
| Contenido de una caravana | `Carga` | ninguno (los peajes son discretos) | no |
| Almacén personal | `Carga` | ninguno | no |
| Existencias de un campamento de mercenarios | `StockCampamento` | ninguno (la reposición es discreta) | no |
| Líneas de un trueque (pactado y entregado), órdenes del mercado | `Mil` | ninguno | no |
| Lo que un héroe recuerda de una plaza | `Carga` (copia de unidades) | — | no |
| Stock de un nodo del mapa (yacimiento) | `StockNodo` | se gasta con la extracción | **no es un almacén**: sigue en milésimas; el resto lo lleva el almacén que recibe |
| Víveres de una columna o de un héroe | `Ritmo` | consumo continuo (`racion`), en marcha, acampada o apartada | **sí** (EE-3): el valor exacto del ritmo es unidades más resto |

**Víveres (EE-3, 2026-10-24).** Van en unidades enteras con resto oculto, sin paso de minuto: el ritmo guarda lo exacto y hace de resto. Con la regla de EE-2 (una unidad sale cuando se completa su consumo) lo que se ve es lo exacto hacia arriba y los víveres **se acaban cuando lo exacto llega a 0**, el mismo instante que antes: el evento «víveres a 0», el hambre y la deserción no cambian, ni ningún cálculo de duración. Lo que entra desde un almacén o un carro va en unidades enteras hacia abajo (`racion::llenar_en_unidades`: salir con trigo, repostar en una plaza o una avanzada, pasar del carro; faltando menos de una unidad ya están llenos); lo gratuito (ración del campamento, `repostarEnCampamento`) llena hasta el tope, que es entero (`racion::llenar`). La parte de cada héroe al separarse va hacia abajo (`Columna::desgajar`) y el resto se queda; al unirse se suman tal cual. La proyección de un héroe los da hacia arriba; la de una columna lleva el ritmo exacto y el cliente lo enseña hacia arriba (comentario en `entities.proto`).

## 3. El almacén: dos formas de mover

`asentamiento::Almacen` guarda `cantidad`, `capacidad` y **`resto`** por recurso.

- **Lo exacto** es lo guardado más el resto (`Almacen::disponible`). `resto` está en `(−1000, 1000)`: positivo, una unidad a medio entrar; negativo, una a medio consumir. Lo exacto nunca es negativo ni pasa de la capacidad.
- **Lo que se ve** (`Almacen::visible`) es lo guardado, con la unidad que se está consumiendo. **Lo que se mueve** (`Almacen::cantidad`: se comercia, se carga y se paga) es lo guardado sin esa unidad empezada; puede ser una menos.
- **Discreto** — `agregar` y `quitar` mueven `entero(n)` (lo que llega con fracción la pierde), `poner` fija `entero(n)`, `devolver` reembolsa hacia abajo, y `tiene`/`pagar` cobran cada línea de un coste con `entero_arriba(n)`: un coste fraccionario (la reparación de una avanzada al 50 %, un coste escalado) nunca sale más barato.
- **Flujo por minuto** — `producir(r, n)` suma `n` (hasta el `hueco = capacidad − exacto`) al resto y pasa a lo guardado las unidades que se completan; `consumir(r, n)` resta `n` (hasta lo exacto) del resto y saca de lo guardado las unidades **cuyo consumo se completa** (EE-2, como la población). Una diferencia deliberada con la población: el resto **no se descarta** al cambiar de signo (producción y consumo del mismo recurso se compensan en él; descartarlo crearía o destruiría fracciones).

En N minutos de un flujo de `p` milésimas por minuto entra exactamente `entero(N × p)` en `cantidad` (prueba `un_flujo_da_en_n_minutos_exactamente_la_suma_entera`).

## 4. Cada operación

| Operación | Dónde | Antes | Ahora |
|---|---|---|---|
| Granjas, Leñeras, extractores de la ciudad | `produccion::extraer_y_cultivar` | `agregar` en milésimas | `producir`; el extractor pide al nodo `min(deseado, hueco)` |
| Extracción de un pueblo | `pueblos::extraer_pueblos` | `agregar` | `producir`; «lleno» es `hueco = 0` |
| Recetas | `produccion::recetas` | insumos y producto en milésimas | insumos con `consumir` (por unidad × cantidad, **hacia arriba** a la milésima) y producto con `producir`; disponibilidad y techo con lo exacto (`disponible`, `hueco`) |
| Comida de la población | `Asentamiento::avanzar_minuto` | `quitar` | `nutrir` lee lo exacto y `consumir` |
| Ración de la guarnición | `Nucleo::alimentar_guarnicion` | `quitar(min(ración, cantidad))` | `consumir(ración)` |
| Recaudación | `avanzar_minuto` | `agregar` | `producir` |
| Mantenimiento | `mantenimiento::avanzar_minuto` | cobertura con `cantidad`, `quitar` | cobertura con lo exacto, `consumir` |
| Costes (obra, mejora, ascenso, recinto, carro, animal, reposición del acarreo, reclutar, intel) | `Almacen::pagar` / `tiene`; `Nucleo::cobrar` (taberna) | milésimas | por línea, **hacia arriba** a unidades |
| Reembolso de una obra quitada de la cola | `Almacen::devolver` | milésimas | hacia abajo |
| Coste de reparar una avanzada | `ReglasAvanzadas::costo_de_reparar` | `por(n, 0,5)` | `entero_arriba` (lo que pide la obra es entero) |
| Proponer un trueque | `trueque::validar` | milésimas | cada línea hacia abajo; menos de una unidad es `CantidadNoPositiva` |
| Colocar una orden de mercado | `OrdenMercado::colocar` | milésimas | hacia abajo; menos de una unidad, `CantidadNoPositiva` |
| Reparto automático | `reparto::asignar_origen` | `min(stock, faltante, libre)` | lo mismo, hacia abajo a unidades |
| Lanzar, cargar y descargar una caravana | `Caravana::{preparar, cargar_aparcada, descargar_aparcada}` | milésimas | cada línea hacia abajo |
| Peaje | `Caravana::cobrar_peajes` | `tasa × n` hacia abajo a la milésima | hacia abajo **a unidades**, y como mínimo una unidad por peaje a toda caravana con carga (EE-4), sin dejarla nunca en cero |
| Entrega, comisión y recargo | `entrega::entregar` | `agregar` en milésimas | `agregar` acuña unidades enteras hacia abajo; el recargo que sale del origen es el mismo entero |
| Mostrador | `mostrador::comerciar` | `q` en milésimas, oro `⌈q × precio⌉` a la milésima | `q` en unidades (baja de unidad en unidad hasta cumplir los topes), oro del lote **hacia arriba a unidades** |
| Reposición de un campamento | `MercadoMercenario::{registrar_comercio, reponer}` | `stock + contador × proporción` | por comercio, `proporción × lo comerciado` hacia abajo y como mínimo una (EE-5) |
| Compras en el campamento | `mercenarios::comprar_*` | ya eran enteras | sin cambios |
| Pérdida de carros, Aguante, carga por carro (Unity) | `Caravana::{aplicar_perdida_de_carros, perder_fraccion_de_carga, carga_por_carro}` | proporcional hacia abajo a la milésima | hacia abajo a unidades |
| Mitad del carro del derrotado | `combate::botin::tras_derrota` | milésimas | hacia abajo a unidades |
| Botín y su reparto por rondas | `botin::{cargar_botin, repartir_botin}` | milésimas; el resto de la división de milésima en milésima | unidades; las unidades que sobran de la división, de una en una a las primeras columnas; la fracción del botín (la chatarra se suma exacta) **cuenta como perdida**: lo cogido más lo perdido es el botín |
| Saqueo de un pueblo | `combate::pueblo::saqueable` | lo que pasa de la mitad de la capacidad | lo mismo, hacia abajo a unidades |
| Saqueo de una avanzada | `combate::avanzada::saquear` | — | ya era de unidades (lo que hay) |
| Chatarra al almacén de quien resiste | `agregar` | milésimas | hacia abajo |
| Separarse o desarmar una columna (parte del carro) | `Columna::desgajar` | a prorrata en milésimas, el resto de milésima en milésima | lo mismo **en unidades** |
| Salir de un lugar con carga | `Nucleo::validar_salida` | milésimas | cada línea hacia abajo; el trigo a los víveres, entero |
| Llenar víveres (repostar, salir, pasar del carro, reabastecer en avanzada) | `racion::llenar_en_unidades` (y `llenar` para lo gratuito) | `min(hueco, ofrecido)` | desde un almacén o un carro, hacia abajo a unidades; lo gratuito, hasta el tope entero |

La **tendencia** «+N/min» del interior de una plaza (4H T3c) es la diferencia de lo **exacto** antes y después del paso: con lo visible, un flujo de 0,37 por minuto se rotularía +0 dos minutos y +1 el tercero.

## 5. La foto y el contrato

- **Foto `BAN27`** (sobre la `BAN26` de la calibración del sistema 4; `BAN26` se rechaza: sus almacenes guardaban milésimas sueltas y no traen el resto). `codec_economia` escribe, por recurso de cada almacén (plaza, pueblo, avanzada, huérfana), `cantidad · capacidad · resto`. Nada más cambia de formato: carros, caravanas, almacén personal y existencias ya se guardaban en milésimas y ahora son múltiplos de 1.000. La huella de reglas no cambia (no cambia ninguna cifra).
- **Contrato**: ningún campo cambia de tipo ni de unidad. Se documenta en `common.proto` (`Existencia`, `Cantidades`) y en `orders.proto` que lo almacenado es múltiplo de 1.000, que el resto oculto no viaja, que la tendencia es exacta y que una cantidad pedida se redondea hacia abajo. Solo comentarios: `buf breaking` limpio.

## 6. Cómo se vigila

- `Nucleo::fracciones_almacenadas()` recorre almacenes (cantidad múltiplo de 1.000 y resto en `[0, 1000)`), carros de columna y de héroe, caravanas, almacenes personales, lo que un héroe recuerda de una plaza, existencias de los campamentos, lo pactado y entregado de los trueques y las órdenes del mercado. La auditoría de cambios (`BRONZEAGE_AUDITAR_CAMBIOS=1`) lo exige vacío **en cada versión** de toda la suite del núcleo, y el batch de comercio y el militar lo cuentan cada 6 h (`inv_fracciones`). El de población (que corre la economía sin núcleo) lo mira en su invariante de almacén.
- Pruebas puras en `asentamiento::almacen` (la suma entera de N minutos, compensación de producción y consumo, tope de capacidad, coste hacia arriba), en `comercio` (peaje, comisión, mostrador, trueque y órdenes) y en `combate` (botín, aguante, disputa); `nucleo` (foto ida y vuelta con resto, versión y hash con y sin sesiones).
- Los batches cuentan la conservación con lo **exacto** (`auditmil::recursos`, `simcom::en_el_mundo`): con lo visible, el resto oculto parecería recurso perdido.

## 7. Lo que no cambia

- El stock de los nodos del mapa sigue en milésimas (no es un almacén).
- La regla «un resto de menos de una unidad salda la línea» del trueque (Comercio §4.4, 2026-10-19) **se retira** (EE-6): una línea solo se salda entregada entera.

## 8. Dudas del autor (decididas el 2026-10-24)

- **EE-1** Lo que se cobra va hacia arriba; lo que se mueve, hacia abajo (regla).
- **EE-2** La unidad sale del almacén al completarse su consumo (§3).
- **EE-3** Los víveres, en unidades enteras con resto oculto (§2).
- **EE-4** El peaje cobra como mínimo una unidad a toda caravana con carga: si la tasa no llega a una unidad en ningún recurso, una del recurso que más lleva (a igualdad, el primero del catálogo). Nunca deja la carga en cero (decisión del autor del mismo día): una caravana de una sola unidad pasa sin pagar.
- **EE-5** La reposición de los campamentos es como mínimo una unidad por comercio: se anota al registrar cada comercio en `MercadoMercenario::repone` (nuevo en la foto `BAN27`), sin tocar los contadores de lo comerciado.
- **EE-6** Se retira la regla de la línea saldada por resto.
- **EE-7** El salto de la reputación mediana se investiga en la fase 7, con C1b (`Mecanicas a balancear.md` §48).
- Decisión técnica: la huella de las reglas no depende de los finales de línea (`texto_de_parametros_de` quita los `\r`).
