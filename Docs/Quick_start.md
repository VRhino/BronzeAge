# QUICK START

Esta es la guia para arrancar este proyecto en local

Ya hay un servidor tuyo escuchando en **:3000** (mi arranque de prueba murió con **EADDRINUSE**, y **GET /v1/balance** respondió 200 desde el que ya tenías). Así que no toqué nada.

Cómo se levanta

Son dos procesos separados: backend Fastify en **:3000** y cliente Vite aparte.

1. Backend (desde la raíz del repo). En PowerShell la sintaxis **VAR='x' npm run ...** de los README no funciona — hay que exportar antes:(**:3000**)
```powershell
$env:ADMINISTRADORES='dev:jefa'; npm run server
```
2. Admin Panel — el de administración/depuración:(**:5173**)
```shell
cd cliente; npm install; npm run dev
```
3. Jugador:(**:5174**)
```shell
cd cliente-jugador; npm install; npm run dev
```
Ambos vite.config.ts proxyan /v1 → http://localhost:3000, así que en desarrollo no necesitas CORS.

Las variables que importan

Todas se leen en src/server/index.ts:20 y ninguna tiene default permisivo — eso es deliberado:

|Variable | default | Si no la pones|
| --- | --- | --- |
| ```ADMINISTRADORES``` | VACIO | nadie puede crear partida (403) formato ```proveedor:sujetoId```, coma-separado: ```dev:jefa,dev:ana``` |
|```PUERTO``` | 3000 | --- |
|```DIRECTORIO_PARTIDAS```| ```./partidas``` | --- |
|```ORIGENES_PERMITIDOS``` | VACIO | CORS APAGADO, DA IGUAL SI USAS EL PROXY DE VITE |
| ```INTERVALO_TICKS_MS``` | ```undefined``` | el mundo no vanasa solo, solo con ```POST .../ticks``` |

El sujeto con el que se identifica el cliente se pone con VITE_USUARIO (por defecto jefa en cliente/, ana en cliente-jugador/) y tiene que figurar en ADMINISTRADORES o el backend devuelve 403 al crear partida. cliente-jugador además usa VITE_GAME_ID (default local) y asume que la partida ya existe — no la crea.

Un arranque completo típico, con ticks automáticos:

```powershell
$env:ADMINISTRADORES='dev:jefa'; $env:INTERVALO_TICK_MS='5000'; npm run server
```