// Fragmentos de JSON Schema compartidos entre rutas (Fase C6): evita declarar la misma forma de `{error}` o
// `ResumenPartida` en cada archivo, y es la base sobre la que `@fastify/swagger` genera el OpenAPI.
//
// DELIBERADAMENTE sin `schema.response` para los cuerpos GRANDES o de forma variable (estado completo de
// partida, proyección de jugador, `resultado.datos` que cambia según el comando): el `response` de Fastify
// no es solo documentación, es un FILTRO DE SERIALIZACIÓN — un campo real que no esté en el schema se
// DESCARTA de la respuesta en caliente. Declarar esos con un schema aproximado arriesgaba romper payloads de
// verdad en silencio; se prefiere que esas rutas queden sin cuerpo de respuesta documentado en el OpenAPI
// (siguen apareciendo con su método, parámetros y seguridad) antes que arriesgar eso.
export const ERROR_RESPUESTA = {
  type: 'object',
  properties: { error: { type: 'string' } },
  required: ['error'],
} as const;

export const RESUMEN_PARTIDA_RESPUESTA = {
  type: 'object',
  properties: {
    gameId: { type: 'string' },
    tick: { type: 'number' },
    version: { type: 'number' },
    mapaId: { type: 'string' },
  },
  // `mapaId` es de `resumenDe` (Fase C11) igual que los otros tres: si se le olvida a este `required` no pasa
  // nada en tiempo de ejecución (Fastify no exige `required` en la salida), pero se DESCARTARÍA del cuerpo en
  // caliente por no estar en `properties` si algún día faltara aquí — ver la cabecera de este archivo.
  required: ['gameId', 'tick', 'version', 'mapaId'],
} as const;

export const PARAMS_GAME_ID = {
  type: 'object',
  properties: { gameId: { type: 'string' } },
  required: ['gameId'],
} as const;

export const PARAMS_GAME_ID_MAPA = {
  type: 'object',
  properties: { gameId: { type: 'string' }, mapaId: { type: 'string' } },
  required: ['gameId', 'mapaId'],
} as const;

/** Cursor de eventos (Fase C13): `desde` llega SIEMPRE como texto por ser un query param de URL — a
 * diferencia de un `body` JSON, que ya trae tipos reales (ver `coerceTypes: false` en `api.ts`), aquí no hay
 * forma de que sea otra cosa. `pattern` valida forma (dígitos), no rango; la ruta hace `Number(...)` y
 * responde 400 si no da un entero no negativo — mismo criterio de "solo forma en el esquema" que C9. */
export const QUERY_DESDE = {
  type: 'object',
  properties: { desde: { type: 'string', pattern: '^[0-9]+$' } },
} as const;
