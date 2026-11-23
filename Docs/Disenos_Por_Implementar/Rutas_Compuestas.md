# Rutas compuestas

> **Diseño acordado el 2026-11-23 con el autor. Sin implementar** (vive en `Docs/Disenos_Por_Implementar/` hasta que se implemente; ver su README). Entrada 80 de `Docs/Mecanicas a desarrollar.md`. Parte de lo que hoy es el encargo de transporte (`Docs/Mechanics/Comercio_Caravanas.md` §5.8). No hay bloque `parametros` hasta implementarlo.

## 1. Qué es

Hoy un encargo tiene **dos paradas**: el origen y un destino. La caravana va y viene entre ellas, y en cada una primero deja y luego recoge (§5.8.2).

Una **ruta compuesta** es un **circuito** de varias paradas en orden: A → B → C → A → B → … Al llegar a la última, la caravana **cierra el círculo** y vuelve a la primera. No desanda el camino. Así puede recoger algo en C y dejarlo en A. En cada parada hace lo de siempre: **primero deja y luego recoge**.

Ejemplo del autor:

| Parada | Deja | Recoge |
|---|---|---|
| A (el origen) | queso 20, carne 20 | madera 100 |
| B | madera 50 | piedra 10, queso 20 |
| C | madera 50, piedra 10 | carne 5 |

- **Primera vuelta:** en A no lleva nada, así que solo recoge. Desde la segunda vuelta ya deja.
- **Lo que deja no pasa de lo que lleva**, como hoy (`tope`): A pide dejar 20 de carne, pero C solo recoge 5, así que A deja 5 por vuelta. No es un error.

Las plazas que el camino atraviesa entre dos paradas sin ser paradas suyas son **escalas** (bloque E de `Tamanos_Y_Distancias_Del_Mapa.md`). La caravana entra en ellas y no comercia. En las ajenas paga el peaje; en las propias no paga nada.

## 2. Las paradas

- **El encargo** lleva una lista ordenada de paradas en lugar de `en_origen` y `en_destino`. La primera es siempre el origen de la caravana. Hay **un máximo de 5 paradas** por circuito, origen incluido, y como mínimo dos, que es el encargo de hoy.
- **Una misma plaza** no puede repetirse seguida. Puede aparecer dos veces si no son seguidas (A → B → A → C).
- **Cada parada** es la `Parada` de hoy (§5.8.2), con su plaza y sus cuatro campos:
  - `recoger`: líneas `{recurso, cantidad, precio_max?}`.
  - `esperar_existencias`: no carga hasta que el almacén tiene todo lo pedido, sin plazo.
  - `dejar`: líneas `{recurso, tope?, precio_min?}`. Sin `tope`, deja todo lo que lleve de ese recurso.
  - `desechar_sobrante`: lo que no cabe en el almacén se tira. Si no, se queda a bordo.
- **Una parada en una plaza ajena** compra y vende contra las órdenes de su mercado, con precio, como hoy un destino ajeno (§5.8.8). Allí la tasa del mostrador se queda en la plaza y no se acuña nada. La regla de hoy de que el oro no se compra ni se vende en una plaza ajena sigue igual: se recoge en una parada propia.

## 3. Qué se valida al crearla

Valen los rechazos de hoy (`encargo.linea_invalida`, `encargo.falta_precio`, rutas por tierra entre cada par de paradas seguidas, contando el cierre). Se añaden estos:

- **`encargo.deja_sin_recoger`:** una parada deja un recurso que **ninguna otra parada** del circuito recoge. Solo se puede dejar lo que se recogió en otra parada.
- **`encargo.supera_capacidad`**, ampliado: se simula una vuelta con todo lo pedido y se mira lo que la caravana lleva a bordo **en el peor tramo**. Las líneas recogen su cantidad entera y dejan su `tope`, o todo lo que lleve si no tiene tope. Si en algún tramo pasa de la capacidad efectiva, se rechaza. En el ejemplo, el peor tramo es de B a C: 50 de madera, 10 de piedra y 20 de queso. Hoy solo se mira lo que se recoge en cada parada.
- **`encargo.demasiadas_paradas`:** más de 5.
- **`encargo.parada_repetida`:** la misma plaza en dos paradas seguidas, contando el cierre (la última con la primera).

## 4. Cómo corre

- **Lo que sobra sigue a bordo.** Si se recoge más de lo que se deja, el resto se queda en la caravana de vuelta en vuelta. Al recoger, solo toma lo que cabe en el hueco, como hoy. Así la ruta nunca se atasca por ir llena: recoge menos.
- **`esperar_existencias` y `desechar_sobrante`** funcionan como hoy en cada parada. Con `esperar_existencias`, la espera solo mira el almacén de esa parada.
- **Tiempo de cada parada:** el de hoy, que es la descarga más la carga por carro del paso (§5.8.5), con el **mínimo de 2 min** del bloque E.
- **`vueltas`** sube en 1 cada vez que la caravana llega al origen.
- **Peajes:** cada tramo paga en las plazas ajenas que atraviesa (bloque E), con lo que lleve en ese momento. Una parada en una plaza ajena paga el peaje al entrar, como cualquier escala.
- **Comisión:** la de hoy.
  - En una **ruta**, todas las paradas propias reciben sin comisión. Ir y venir entre plazas propias no acuña oro.
  - En un **envío** de varias paradas, cada parada propia distinta del origen cobra la comisión de una entrega por lo que entra.
- **Escolta:** igual que hoy. Va en todas las vueltas y vuelve al terminar el encargo.

## 5. Los dos modos

- **Ruta:** da vueltas al circuito hasta que se cancela o se detiene. Hoy solo admite plazas de la Facción del origen. La ruta compuesta admite paradas ajenas, con precio en su mercado, tanto en ruta como en envío. El autor, el 2026-11-23, dijo que lo de hoy era la versión incompleta.
- **Envío:** una sola vuelta al circuito, A → B → C → A, y termina `cumplido` al dejar en el origen.

## 6. Cancelar, detener, cambiar y caídas

- **Cancelar:** igual que hoy en el destino. Si está en una parada que no es el origen, sale ya hacia el origen sin comerciar más, por el camino más corto y sin pasar por las paradas que faltan. Al llegar, termina.
- **Detener:** acaba la vuelta en curso comerciando en las paradas que le faltan, deja en el origen y termina `detenido`.
- **Cambiar:** las paradas solo cambian con la caravana `esperando` en su origen, como hoy el destino. Las líneas de una parada que aún no ha empezado pueden cambiar en cualquier momento.
- **Una parada que deja de valer** (no existe, o en una ruta deja de ser de la Facción y no era ajena al crearla) se **salta** y emite un hecho. Si ya no queda ninguna parada válida aparte del origen, el encargo termina `destino_perdido`.

## 7. Lo que toca

- **`crates/comercio`:** `Encargo` pasa de dos paradas a una lista, con la punta como índice. Se añaden la validación del peor tramo y los rechazos nuevos.
- **`crates/nucleo`:** el ciclo de `encargos.rs`, la proyección, la foto (sube su número) y la vista del encargo.
- **El contrato (`proto/`):** `OrdenEncargarCaravana` y `OrdenCambiarEncargo` llevan la lista de paradas. Sube la versión de `contrato`.
- **Docs:** `Comercio_Caravanas.md` §5.8, el canon (`Docs/Game/3_Sistema_Economico_y_Comercio.md` §3.13.9) y el catálogo de rechazos.
- **Clientes:** la ventana del encargo pasa de dos paradas a una lista que se ordena. BronzeAgeClient, Conquest y BotsBrain.

## 8. Por concretar al implementar

- Si se puede pasar de un encargo de dos paradas a uno de varias con `CambiarEncargo` o hay que darlo de nuevo.
- Cómo se ve en el mapa el circuito entero, y no solo el tramo en curso.
