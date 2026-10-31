# Rutas de caravanas automáticas

> **Diseño acordado el 2026-10-31 con el autor. Sin implementar** (vive en `Docs/Disenos_Por_Implementar/` hasta que se implemente; ver su README). Los tiempos de carga y descarga son placeholders por calibrar; no hay bloque `parametros` hasta implementarlo. Entrada 64 de `Docs/Mecanicas a desarrollar.md`.

## 1. Qué es

Una caravana comercial de la flota recibe un **encargo** de transporte con dos paradas, origen y destino, y en cada una sabe qué deja y qué recoge. Hay dos modos:

- **Ruta permanente:** va y viene entre dos plazas de mi Facción sin parar, hasta que se cancela o la destruyen.
- **Envío programado:** sale el día y la hora elegidos, hace una sola ida y vuelta y queda libre. Puede ir a una plaza de otra Facción, donde comercia con las órdenes de su mercado (§6).

Casi todo existe ya y se aprovecha:

| Pieza | Dónde | Qué aporta | Qué le falta |
|---|---|---|---|
| Lanzar a mano con `salirEn` | `Comercio_Caravanas.md` §5.6 | Salida programada hasta 3 días vista (`maxProgramacionDias`), carga reservada al preparar, ruta por tierra comprobada | Vuelve siempre vacía; no recoge en el destino |
| Viaje y entrega | §6 y §4.4 | Ida por plan, entrega topada por el almacén (lo que no cabe se pierde), vuelta desandando la ruta | El sobrante siempre se pierde; la vuelta no lleva carga |
| Reparto automático del trueque | §4.3 | Una caravana disponible sale sola si hay algo pendiente; `reservadaManual` la saca del reparto | Solo sirve a un trueque |
| Carro de acarreo | §12 | Ruta fija ida y vuelta sin fin; sale solo con el carro lleno («esperar existencias») | Es de un pueblo, no de la flota, y vuelve vacío |
| Escolta cedida | §5.5 | Escuadras de residentes del origen, cupo de Liderazgo del Mercado, sin ración | Vuelve al campamento al acabar cada viaje |
| Mostrador | §8.3 | Cuentas de compra y venta contra las órdenes de una plaza | Solo lo usa un héroe con su columna |

## 2. El encargo

La caravana gana un campo `encargo`; mientras lo tiene, el reparto del trueque no la usa (como con `reservadaManual`). Los carros, el animal de cada uno y la escolta son los que ya tiene cuando se le da el encargo: se montan antes, con las órdenes de siempre.

| Campo | Ruta permanente | Envío programado |
|---|---|---|
| `modo` | `ruta` | `envio` |
| `destino` | Una plaza de mi Facción, distinta del origen | Una plaza cualquiera, distinta del origen |
| `salir_en` | No; sale en cuanto se le da | Día y hora de mundo, hasta 3 días vista |
| `parada_origen` | Qué recoge y qué deja en el origen | Igual |
| `parada_destino` | Qué recoge y qué deja en el destino | Igual; en una plaza ajena, recoger es comprar a sus órdenes de venta y dejar es venderle a sus órdenes de compra |
| `vueltas` | Contador de idas y vueltas hechas | 0 o 1 |

Cada parada lleva dos listas, cada una con su interruptor:

- **Recoger:** recurso y cantidad. Con `esperar_existencias` en sí, la caravana no sale hasta que el almacén tenga todo lo pedido (sin plazo: se ve qué falta y se cancela a mano); en no, sale con lo que haya.
- **Dejar:** recurso y, si se quiere, un tope; sin tope deja todo lo que lleve de ese recurso. Con `desechar_sobrante` en sí, lo que no cabe en el almacén se tira; en no, se queda en el carro.

En una plaza ajena cada línea de recoger lleva además `precio_max` y cada línea de dejar `precio_min` (§6). En las plazas propias no llevan precio.

La suma de lo que se recoge en una parada no puede pasar de la capacidad de la caravana (carros con animal × capacidad, con «Carga Ampliada» si la tiene). Los estados son los de hoy (`disponible`, `preparando`, `en_transito`, `retornando`) y uno nuevo, **`esperando`**: dentro de una plaza, propia o ajena, cargada o vacía, a la espera de existencias o de su hora, o mientras descarga y carga (en una ajena, mientras vende y compra).

## 3. El ciclo

La caravana empieza recogiendo en el origen, y en cada parada primero deja y luego recoge; cada paso tarda su tiempo por carro (§5).

```
origen: recoge ──ida──▶ destino: deja → recoge ──vuelta, cargada──▶ origen: deja ─┬─ ruta: vuelve a recoger y repite
                                                                                  └─ envío: queda disponible
```

## 4. Reglas

La caravana con encargo es una caravana comercial más: se mueve, se ataca y se pierde igual; solo cambia lo que hace en cada punta.

| Tema | Regla |
|---|---|
| Quién da el encargo | El Maestro de Caravanas, el Tesorero o el Gobernador del origen, residente y dentro. También lo cambian y lo cancelan |
| Qué caravana | Una comercial del origen, `disponible` y con al menos un carro con animal. No gasta cupo nuevo: ya cuenta en la flota. No hay tope de rutas aparte del cupo de caravanas |
| Destino de una ruta | Plaza de mi Facción con ruta por tierra; si deja de serlo (conquista, ruina), la ruta termina. Nunca una plaza ajena ni aliada |
| Destino de un envío | Cualquier plaza con ruta por tierra. En una ajena con Mercado activo, vende y compra contra sus órdenes (§6) |
| Escolta | La cedida antes del encargo viaja en todas las vueltas y vuelve al campamento al terminar el encargo, no en cada vuelta. Mientras dura se puede añadir o quitar con la caravana `esperando` en el origen |
| Carga y descarga | Cada parada tarda `tiempoDescarga` por carro que deja algo y `tiempoCarga` por carro que recoge algo. Sustituye a la preparación de hoy (`kPorCarro`). Quien lleva la caravana a mano con su columna no lo paga |
| Carga | Se toma del almacén al empezar la carga: lo pedido cuando lo hay, o lo que haya si no espera. Sale al acabar el tiempo de carga |
| Entrega | Primero deja, luego recoge. Lo que deja entra topado por el almacén; el sobrante se tira o se queda a bordo según `desechar_sobrante` |
| Esperando | Dentro de la plaza, propia o ajena: no se ve en el mapa ni se ataca, porque la protege la plaza. La caravana solo es vulnerable en ruta. Si la plaza cae con ella dentro, corre la suerte de una caravana aparcada (se destruye) |
| Comisión | Ninguna en una ruta (si no, ir y venir entre dos plazas propias acuñaría oro sin fin). El envío cobra la de hoy por lo que entrega en otra plaza propia. En una ajena no hay entrega sino comercio: paga la tasa del mercado (§8.3) y no se acuña nada |
| Peajes | En cada tramo con carga que cruce una zona ajena, no solo a la ida |
| Ataques | Como hoy: en tránsito se intercepta; si pierde carros sigue con los que le queden; sin carros la caravana desaparece y el encargo con ella |
| Cancelar | Si está `esperando` en el origen, queda `disponible` allí; si no, acaba el tramo en curso, deja lo que lleve al llegar al origen y queda `disponible` |
| Fin del envío | Al volver al origen y dejar, queda `disponible` sin encargo |

## 5. El Maestro de Caravanas y los tiempos

**Maestro de Caravanas.** Un cargo local nuevo en cada plaza, por debajo del Tesorero. Solo hace una cosa: da, cambia y cancela los encargos de las caravanas de su plaza. El Tesorero y el Gobernador pueden hacer lo mismo siempre, haya o no Maestro de Caravanas; todo lo demás del comercio (trueques, órdenes del mercado, reservas, abrir el almacén a los aliados) sigue siendo del Tesorero.

- Lo nombra el Gobernador vigente, residente y dentro, como a los otros cuatro cargos locales. El designado es ciudadano de la Facción y reside en la plaza.
- Un mismo héroe puede ser Tesorero y Maestro de Caravanas a la vez.
- No tiene huecos de políticas.
- En el código: `Cargo` gana `MaestroCaravanas` (clave `maestroCaravanas`) y entra en `CARGOS_DE_PLAZA`, sin huecos en `slotsPorCargo`; `Cargos` del contrato gana `maestro_caravanas_id`.

**Tiempos de carga y descarga.** Al llegar, la caravana descarga: `tiempoDescarga` por cada carro que deja algo. Después carga: `tiempoCarga` por cada carro que recoge algo. Una parada en la que no deja ni recoge nada no tarda. Con «esperar existencias», el tiempo de carga empieza cuando el almacén ya tiene lo pedido. Los dos valores irán en un bloque `ENCARGO`, en minutos por carro, por calibrar.

| | Encargo automático | A mano, con una columna |
|---|---|---|
| Quién va | Nadie: la caravana sola con su escolta | Un héroe con su columna |
| Cargar y dejar | Paga `tiempoCarga` y `tiempoDescarga` en cada parada | Al momento, con `cargar_de_plaza` y `dejar_en_plaza` (o el mostrador en una plaza ajena) |
| Protección | La escolta cedida | La columna entera |
| Atención del jugador | Ninguna: va y viene sola | La lleva en cada viaje |

## 6. Comercio con órdenes en una plaza ajena

En una plaza de otra Facción la caravana no toca el almacén. Comercia con el mercado de la plaza contra las órdenes activas que esa plaza ha puesto, con las cuentas del mostrador (`Comercio_Caravanas.md` §8.3). Lo que puede recoger allí es lo que la plaza ofrece en sus órdenes de venta, no lo que tiene en el almacén.

- **Dejar es vender.** Cada línea lleva recurso, tope opcional y `precio_min`. Vende a las órdenes de compra activas de ese recurso con precio ≥ `precio_min`, de la que mejor paga a la que peor. Cobra precio × (1 − tasa) en oro.
- **Recoger es comprar.** Cada línea lleva recurso, cantidad y `precio_max`. Compra a las órdenes de venta activas de ese recurso con precio ≤ `precio_max`, de la más barata a la más cara, hasta la cantidad pedida. Paga precio × (1 + tasa) con el oro que lleva a bordo.
- **Primero vende, luego compra**, así lo cobrado sirve para comprar en el mismo viaje.
- **Los topes del mostrador:** lo pendiente de cada orden, el almacén de la plaza que vende, el hueco y el oro de la plaza que compra, y el oro y el espacio de la caravana. La tasa es la externa con el factor de las políticas de la plaza. La comisión se queda en la plaza, no se acuña nada y el oro se conserva.
- **El oro de la compra** se recoge en el origen como un recurso más (una línea de recoger «oro»). Al volver, el oro cobrado se deja en el origen como cualquier otra carga.
- **Lo que no se vende** vuelve a bordo. En una plaza ajena no se tira ni se regala (`desechar_sobrante` no cuenta allí). Si un aliado quiere recibir, pone una orden de compra.
- **No espera órdenes.** Entra en la plaza, vende y compra lo que haya con sus tiempos de descarga y carga, y vuelve. Todo el comercio pasa dentro, donde la protege la plaza; solo es vulnerable en ruta.
- **Sin Mercado activo** en la plaza al llegar, no comercia y vuelve con lo que lleva.
- **Qué se ve.** Mientras la caravana está dentro, su Facción ve las órdenes activas de esa plaza, como quien está dentro de una plaza ajena. Las órdenes caducan a los 200 min y no reservan nada, así que el resultado es lo que haya al llegar.
- **Hechos.** Cada compra o venta cumple la orden como el mostrador y emite `mercado.comercio_en_plaza` con `entreFacciones` y la caravana.

## 7. Contrato

Tres órdenes nuevas, un cargo nuevo, campos nuevos en `Caravana` y un estado. Los números de orden y de comando se ponen al implementarlo.

| Pieza | Forma |
|---|---|
| `encargar_caravana` | `caravana_id`, `destino_id`, `modo` (`ruta` o `envio`), `salir_en_ms?`, `parada_origen`, `parada_destino`. Cada parada: `recoger[{recurso, cantidad, precio_max?}]`, `esperar_existencias`, `dejar[{recurso, tope?, precio_min?}]`, `desechar_sobrante`. Los precios solo cuentan en una plaza ajena |
| `cambiar_encargo` | Los mismos campos menos `caravana_id` y `modo`; vale desde la próxima parada |
| `cancelar_encargo` | `caravana_id` |
| `Caravana.encargo` | El encargo tal cual más `vueltas`, `parada_actual` (origen o destino) y `fin_parada_ms` (cuándo acaba la descarga o la carga en curso); solo en la proyección propia |
| `EstadoCaravana.ESPERANDO` | Dentro de una plaza con encargo, sin salir aún |
| `Cargo.MAESTRO_CARAVANAS` | Cargo local nuevo: `Cargos.maestro_caravanas_id`, y `asignar_cargo_local` lo acepta |
| Órdenes a la vista | Las órdenes activas de una plaza ajena llegan a la Facción mientras una caravana suya está dentro de ella |
| Hechos | `caravana.encargo_dado`, `caravana.espera` (qué falta), `caravana.carga` y `caravana.descarga` (con lo tirado y lo que vuelve a bordo), `caravana.encargo_terminado` (motivo: cumplido, cancelado, destino perdido, caravana destruida). En una plaza ajena, cada compra o venta emite `mercado.comercio_en_plaza` con la caravana |
| Rechazos | `encargo.destino_ajeno` (ruta a una plaza de otra Facción), `encargo.falta_precio` (línea sin precio hacia una plaza ajena), `encargo.supera_capacidad`, `encargo.mismo_origen`, `encargo.ya_tiene`, `encargo.no_tiene`; se reutilizan `caravana.no_disponible`, `caravana.sin_ruta`, `caravana.salida_invalida`, `no_autorizado` y los `cargo.*` al designar |

La foto sube de formato (el encargo se guarda en la caravana y la plaza gana un cargo) y hace falta partida nueva, porque cambian las reglas del viaje: comisión, peajes y tiempos de parada.

## 8. Decisiones del autor (2026-10-31)

- La ruta va solo entre plazas propias; el envío puede ir a una ajena, y allí comercia con órdenes.
- Comisión: la ruta no cobra; el envío cobra como el lanzamiento a mano; el comercio con órdenes paga la tasa del mercado.
- Esperar existencias sin plazo.
- Quién da el encargo: el Maestro de Caravanas (cargo nuevo, sin huecos de políticas), el Tesorero o el Gobernador, siempre.
- Tiempos de carga y descarga por carro en cada parada, en lugar de la preparación; quien lo hace a mano con su columna se los salta.
- Peajes en cada tramo con carga. Sin tope de rutas aparte del cupo de caravanas.
- Sin regalo en una plaza ajena; sin esperar órdenes. Todo el comercio pasa dentro de la plaza: la caravana solo es vulnerable en ruta.

Diseño de trabajo: Claude Doc «Rutas de caravanas automáticas: diseño» (https://claude.ai/artifact/U7xW6aEzahV1MMk6EazaAi).
