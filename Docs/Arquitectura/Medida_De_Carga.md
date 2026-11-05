# Medida de carga del motor

Implementado el 2026-11-05 (pedido del autor: ver el rendimiento en los playtest sin afectar a producción). El código
está en `crates/nucleo/src/sesiones/carga.rs`; los contadores de red, en `servidor::MetricasRed`.

## Qué mide

El actor del núcleo (el único que escribe el mundo) mide su propio trabajo en **ventanas de 10 s de pared**. Al
cerrar cada ventana escribe una línea `carga:` en el registro del servidor, la guarda como la última (la que da
`GET /v1/admin/carga`) y vuelve a cero.

Tiene dos niveles:

- **Lo básico.** Está encendido salvo que el servidor arranque con `BRONZEAGE_CARGA=0`. Cuesta un par de lecturas
  del reloj por mensaje del actor y unos contadores atómicos por mensaje de red.
- **El detalle.** Está apagado hasta que la administración lo enciende en caliente. Añade el tiempo de cada fase del
  reparto, las órdenes por tipo, las sesiones con el canal a medio llenar y una línea JSON por ventana en
  `<partida>/carga.jsonl` (junto a la carpeta `mundo/` de la partida).

Encender o apagar cualquiera de los dos empieza una ventana nueva. Con lo básico apagado no se mide nada y el
detalle tampoco corre.

## La línea del registro

```
carga: actor ocupado 42.5 % (vistas 31.2 %); 180 órdenes en 10 s, p50 3.1 ms, p95 9.8 ms, p99 14.2 ms, la más lenta 22.4 ms; hasta 6 esperando en la fila; 120 sesiones, 118 con VISTA; red 85.3 KB/s
```

## Rutas de administración

Las dos piden un administrador, como el resto de `/v1/admin/`.

| Ruta | Cuerpo | Respuesta |
|---|---|---|
| `GET /v1/admin/carga` | ninguno | `{ activo, detalle, cada_s, ultima }` |
| `POST /v1/admin/carga` | `{ activo?, detalle? }`, booleanos | igual que el `GET`, ya con el cambio |

- `activo` es lo básico y `detalle` el detalle. `cada_s` es la duración de la ventana (10).
- `ultima` es la última ventana cerrada, o `null` si aún no se ha cerrado ninguna desde que arrancó o desde el último
  cambio.
- Un campo que no sea booleano da 400 `datos_invalidos`. Otro método da 405. Si el núcleo no contesta, 503.

## La ventana

```json
{
  "hasta_ms": 1762339200000,
  "segundos": 10.0,
  "actor_ocupado_pct": 42.5,
  "vistas_pct": 31.2,
  "ordenes": 180,
  "ordenes_por_s": 18.0,
  "orden_ms": { "media": 3.6, "p50": 3.1, "p95": 9.8, "p99": 14.2, "max": 22.4 },
  "fila_maxima": 6,
  "sesiones": 120,
  "vistas": 118,
  "descartadas_por_canal_lleno": 0,
  "red": {
    "mensajes_por_s": 512.0,
    "kb_por_s_crudos": 310.4,
    "kb_por_s_enviados": 85.3,
    "compresion": 0.275,
    "escrituras_lentas": 0,
    "tramas_recibidas_por_s": 20.1,
    "kb_por_s_recibidos": 1.2
  },
  "detalle": {
    "fases_ms": { "mundo": 120.0, "derechos y deltas": 40.0, "fuera del registro": 2.0, "vistas": 3120.0, "alijos": 80.0, "entregar": 15.0, "batallas": 0.5, "aplicar órdenes": 300.0 },
    "ordenes_por_tipo": [ { "tipo": "MarcharA", "n": 90, "media_ms": 4.1, "max_ms": 22.4, "total_ms": 369.0 } ],
    "canales_medio_llenos": 0
  }
}
```

| Campo | Qué es |
|---|---|
| `hasta_ms` | cuándo se cerró la ventana, en ms desde 1970 (pared) |
| `segundos` | lo que duró de verdad la ventana |
| `actor_ocupado_pct` | parte de la ventana en que el actor estuvo trabajando (órdenes, avanzar el mundo, repartir, peticiones). Cerca de 100 es que no da abasto |
| `vistas_pct` | la parte de la ventana que se fue en poner al día las `VISTA` y los alijos del `HEROE` (incluida en la anterior) |
| `ordenes`, `ordenes_por_s` | las órdenes de jugador atendidas |
| `orden_ms` | lo que tardó cada orden, desde que el actor la saca de la fila hasta que reparte su resultado |
| `fila_maxima` | cuántos mensajes llegó a haber esperando al actor a la vez |
| `sesiones`, `vistas` | sesiones abiertas y cuántas tienen una `VISTA` abierta |
| `descartadas_por_canal_lleno` | mensajes que no cupieron en el canal de una sesión (cliente que no lee) |
| `red.mensajes_por_s` | mensajes enviados a los clientes |
| `red.kb_por_s_crudos`, `red.kb_por_s_enviados`, `red.compresion` | bytes antes y después de comprimir, y su razón |
| `red.escrituras_lentas` | escrituras que no acabaron en `timeout_escritura`; la sesión se cierra por cliente lento |
| `red.tramas_recibidas_por_s`, `red.kb_por_s_recibidos` | lo que llega de los clientes |
| `detalle.fases_ms` | tiempo total de cada fase en la ventana: `mundo` (avanzar la simulación), las fases del reparto y `aplicar órdenes` |
| `detalle.ordenes_por_tipo` | las diez órdenes de más tiempo total, por el nombre de su cuerpo del contrato |
| `detalle.canales_medio_llenos` | sesiones con el canal de salida a medio llenar o más |

## Cómo leerla

- `actor_ocupado_pct` alto con `vistas_pct` casi igual: el coste es el reparto de las `VISTA` (lo medido en el
  análisis de escalado del 2026-11-05).
- `orden_ms.p95` alto con `fila_maxima` alta: las órdenes esperan detrás de otro trabajo del actor.
- `canales_medio_llenos` o `escrituras_lentas` por encima de cero: el cuello está en la red o en clientes lentos, no
  en el motor.
