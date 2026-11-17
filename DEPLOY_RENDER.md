# El servidor en Render (plan gratis) con la partida en Turso

Cómo se publica `bronzeage-servidor` sin pagar. Lo que hay en el repositorio: el [`Dockerfile`](Dockerfile) (genera el mundo y compila el servidor), el Blueprint
[`render.yaml`](render.yaml) y el espejo de la carpeta de datos (`crates/servicio/src/espejo.rs`).

## Por qué así

El plan gratis de Render tiene dos límites:

1. **El disco no dura**: se borra en cada despliegue y cada vez que el servicio arranca tras dormirse. El servidor guarda la partida en su carpeta de datos, así que con
   `ALMACEN_URL` copia esa carpeta (registro, fotos, crónica y cuentas) a una base **libSQL de Turso**: la baja al arrancar con la carpeta vacía y la sube cada 5 s y al parar.
   Una parada normal (despliegue, sueño) no pierde nada; si el proceso muere de golpe se pierde, como mucho, lo escrito desde la última copia.
2. **Se duerme a los 15 min sin tráfico**. No se pierde nada (el reloj de mundo se para con el servidor y sigue al despertar), pero el mundo no avanza mientras duerme y el primero
   que entra espera el arranque. Si se quiere despierto, un pinger externo a `/salud` cada 10 min (por ejemplo cron-job.org).

Memoria: el servidor con el mundo real ocupa unos 110 MB en reposo; el plan da 512 MB.

## Variables del servicio

| Variable | Qué |
|---|---|
| `ALMACEN_URL` | la URL de la base: `turso db show <base> --url` (`libsql://…`) |
| `ALMACEN_TOKEN` | `turso db tokens create <base>` |
| `CODIGO_REGISTRO` | el código que se pasa a los jugadores; sin él, cualquiera se registra |
| `CODIGO_REGISTRO_BOTS` | el de las cuentas de bot; sin él, no hay bots |
| `CODIGO_REGISTRO_SERVIDORES` | el de la cuenta del lanzador de batallas; solo con `BRONZEAGE_BATALLAS_EN_UNITY=1` (Render no da UDP: los servidores de batalla irían en otra máquina) |
| `BRONZEAGE_LLAVE_ADMIN` | una llave de 24 caracteres o más. Detrás del proxy de Render la IP que ve el servidor es la del proxy, así que la lista de IPs no vale: se administra con la cuenta admin **y** esta llave, en la cabecera `X-Llave-Admin` o en la consulta (`?llave=…`, para el WebSocket de un navegador) |
| `BRONZEAGE_ORIGENES` | nombres de máquina de las páginas web que pueden llamar al servidor desde otro dominio; una página abierta en `localhost` ya puede |
| `BRONZEAGE_SEMILLA` | la semilla de la partida (solo cuenta al crearla) |

`PORT` lo pone Render; el servidor escucha entonces en `0.0.0.0:PORT`. La semilla del **mundo** es el argumento `SEMILLA_MUNDO` del `Dockerfile` (42).

## La primera cuenta admin

Con la partida nueva no hay administradores: el servidor imprime en el registro de Render un **código de administrador de un solo uso**. Quien se registra con él es admin.

## Partida nueva

Cuando un cambio del motor pide partida nueva, hay que vaciar la copia antes de desplegar (si no, el servidor la baja y sigue la vieja):

```bash
turso db shell <base> "DELETE FROM espejo"
```

Si la carpeta de datos de una máquina y la copia tienen partidas distintas, el servidor no arranca: no pisa una con otra.

## Comprobar

`curl https://<servicio>.onrender.com/salud` contesta `ok`. En el registro del arranque: `copia remota: …` y `escuchando en 0.0.0.0:<puerto>`; al parar, `copia remota al día`.
