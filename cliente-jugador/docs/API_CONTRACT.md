# Contrato API del cliente jugador

Documento de referencia para `cliente-jugador`. El backend escucha bajo el prefijo `/v1`.

## Estado de implementación

### Llamadas utilizadas actualmente por la UI

- [x] `POST /v1/sesiones`
- [x] `POST /v1/jugador/partidas/{gameId}/membresia`
- [x] `GET /v1/jugador/partidas/{gameId}`
- [x] `GET /v1/jugador/partidas/{gameId}/mapa/{mapaId}`
- [x] Reintento automático tras una respuesta `401`, creando una sesión nueva con `POST /v1/sesiones`

### Funciones disponibles en `apiCliente.ts`, pero sin llamada desde la UI actual

- [ ] `GET /v1/sesiones/actual`
- [ ] `POST /v1/jugador/partidas/{gameId}/comandos`

### Superficie de backend todavía no consumida por este cliente

- [ ] `GET /v1/jugador/partidas/{gameId}/eventos`
- [ ] WebSocket de tiempo real de la partida

## Autenticación

Todas las rutas, excepto el login, requieren la cabecera:

```http
Authorization: sesion <sesionId>
```

El login de desarrollo utiliza:

```http
Authorization: dev <usuario>
```

La implementación actual usa el nombre de usuario introducido en el formulario como credencial `dev`.
Este mecanismo corresponde al proveedor de desarrollo del backend y no debe considerarse autenticación de
producción.

## Endpoints utilizados

### Crear sesión

```http
POST /v1/sesiones
Authorization: dev <usuario>
```

No requiere body. Devuelve `201 Created`:

```json
{
  "usuarioId": "ana",
  "sesionId": "sesion-...",
  "expiraEn": "2026-08-27T12:00:00.000Z"
}
```

Errores relevantes:

- `401`: credencial, proveedor o cabecera inválida.
- `0` en el cliente: no se pudo contactar con el backend.

### Unirse a una partida

```http
POST /v1/jugador/partidas/{gameId}/membresia
Authorization: sesion <sesionId>
```

No requiere body. Devuelve `201 Created`:

```json
{
  "jugadorId": "ana"
}
```

El cliente trata `409 Conflict` como membresía ya existente y continúa con el login.
Otros errores no se ignoran:

- `401`: sesión inválida o ausente.
- `404`: partida no abierta o inexistente.
- `409`: ya existe una membresía para ese usuario y partida.

### Consultar proyección del jugador

```http
GET /v1/jugador/partidas/{gameId}
Authorization: sesion <sesionId>
```

La respuesta es la proyección filtrada para el jugador, no el estado completo de la partida. El cliente
consume estos campos:

```json
{
  "gameId": "local",
  "tick": 42,
  "version": 7,
  "jugadorId": "ana",
  "faccionId": "faccion-1",
  "mapaId": "mapa-...",
  "facciones": [],
  "asentamientos": [],
  "caravanas": [],
  "caminos": [],
  "campamentosBandidos": [],
  "zonasFusionadas": [],
  "trazadoPorAsentamiento": {},
  "preciosReferencia": {}
}
```

La proyección puede incluir campos adicionales. El tipo local permite esos campos mediante un índice abierto.
El campo `mapa` no viaja en esta respuesta: solo se entrega `mapaId` para localizar el asset del mapa.

Errores relevantes:

- `401`: sesión inválida o ausente.
- `403`: la sesión no tiene membresía de jugador en la partida.
- `404`: partida no abierta o inexistente.

### Obtener el mapa como asset

```http
GET /v1/jugador/partidas/{gameId}/mapa/{mapaId}
Authorization: sesion <sesionId>
```

Devuelve el `MapaGenerado` actual. El backend establece:

```http
Cache-Control: private, max-age=31536000, immutable
```

`mapaId` funciona como identificador de versión/cache-buster. El servidor valida la sesión y la partida, pero
el handler no interpreta el valor del `mapaId`; devuelve el mapa vigente del runner.

El cliente cachea el resultado en memoria y actualmente lo identifica solo por `mapaId`.

Forma principal de la respuesta:

```json
{
  "version": 7,
  "config": {
    "ancho": 2000,
    "alto": 2000,
    "seed": 123
  },
  "bosques": [],
  "nodos": [],
  "fertilidad": {},
  "elevacion": {},
  "rios": []
}
```

El campo `elevacion.region` puede estar presente. La copia del evaluador en este cliente no interpreta
regiones geográficas y el terreno renderizado puede divergir del backend en ese caso.

Errores relevantes:

- `401`: sesión inválida o ausente.
- `403`: la sesión no tiene membresía de jugador.
- `404`: partida no abierta o inexistente.

## Endpoints preparados pero no usados

### Consultar identidad y rol

```http
GET /v1/sesiones/actual?gameId={gameId}
Authorization: sesion <sesionId>
```

Respuesta esperada:

```json
{
  "usuarioId": "ana",
  "esAdministradorGlobal": false,
  "gameId": "local",
  "rol": "jugador",
  "jugadorId": "ana"
}
```

`gameId` es opcional. Sin él, la respuesta solo garantiza `usuarioId` y `esAdministradorGlobal`.

### Ejecutar comando

```http
POST /v1/jugador/partidas/{gameId}/comandos
Authorization: sesion <sesionId>
Content-Type: application/json
```

Body mínimo:

```json
{
  "tipo": "<tipo-de-comando>",
  "params": {}
}
```

También admite opcionalmente `idempotencyKey`. El backend valida `params` según el `tipo` de comando.
El catálogo actual incluye 32 comandos, entre ellos `unirseAFaccion` (con `faccionId`) y `dejarFaccion`
(con `params: {}`).
En caso de éxito, la respuesta incluye `resultado`, resumen de partida y una `proyeccion` actualizada.

Errores relevantes: `400` petición o comando inválido, `401` sesión inválida, `403` no autorizado y `409`
rechazo del comando o fallo de persistencia.

## Errores comunes del wrapper

`apiCliente.ts` convierte todas las respuestas HTTP no exitosas en `ApiError`, conservando `status` y `message`.
El mensaje se obtiene de `{ "error": "..." }` cuando el backend lo devuelve. Los fallos de red usan estado
`0` y un mensaje local indicando que el backend no está disponible.

Ante un `401` de una petición autenticada, el wrapper intenta volver a autenticarse automáticamente usando el
último usuario conocido. Si la reautenticación funciona, repite la petición original una vez.

## Fuentes del contrato

- Cliente: `cliente-jugador/src/apiCliente.ts`
- Pantalla y secuencia de llamadas: `cliente-jugador/src/main.ts`
- Rutas de jugador: `src/server/rutas/jugador.ts`
- Rutas de sesión: `src/server/rutas/sesiones.ts`
- Asset de mapa: `src/server/rutas/mapa.ts`
- Comandos HTTP: `src/server/rutas/comandos.ts`
