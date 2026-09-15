// Configuración de `@fastify/swagger` (Fase C6, doc 4: "publicar el contrato como OpenAPI... para que los
// repos de cliente generen su cliente tipado sin acoplarse a este ni a TypeScript"). El documento se genera
// a partir de los `schema` de cada ruta — es un REFLEJO del código, no una segunda fuente de verdad que
// pueda desincronizarse: si una ruta cambia su esquema, el JSON publicado cambia con ella sin tocar esto.
//
// Dos esquemas de seguridad `apiKey` sobre la MISMA cabecera `Authorization`, porque el protocolo real usa
// un único header con dos significados según el prefijo (ver `acceso/servicioAutenticacion.ts`): OpenAPI no
// tiene forma de expresar "el esquema depende del prefijo del valor", así que esto es lo más preciso que se
// puede declarar sin inventar una convención que el servidor no sigue. La `description` de cada uno lleva el
// formato exacto para que no haga falta adivinarlo leyendo el código.
import type { FastifyDynamicSwaggerOptions } from '@fastify/swagger';

export const ESQUEMA_SESION_AUTH = 'sesionAuth';
export const ESQUEMA_CREDENCIAL_PROVEEDOR = 'credencialProveedor';
export const ESQUEMA_SERVIDOR_BATALLA_AUTH = 'servidorBatallaAuth';

export const opcionesOpenApi: FastifyDynamicSwaggerOptions = {
  openapi: {
    openapi: '3.0.0',
    info: {
      title: 'Bronze Age Collapse — API de partida',
      description:
        'Backend de partida multijugador (Fase C, solo servidor). Dos superficies: administración ' +
        '(gobierno de la partida) y jugador (unirse, ejecutar comandos, tiempo real). El WebSocket de ' +
        '`/jugador/partidas/{gameId}/tiempo-real` no aparece aquí: OpenAPI 3.0 no describe WebSocket.',
      version: '1.0.0',
    },
    servers: [{ url: '/v1', description: 'Prefijo de versión de este proceso' }],
    components: {
      securitySchemes: {
        [ESQUEMA_SESION_AUTH]: {
          type: 'apiKey',
          in: 'header',
          name: 'Authorization',
          description: "Sesión ya emitida por el login. Formato: 'sesion <sesionId>'.",
        },
        [ESQUEMA_CREDENCIAL_PROVEEDOR]: {
          type: 'apiKey',
          in: 'header',
          name: 'Authorization',
          description:
            "Credencial de un proveedor de identidad dado de alta (ver acceso/registroProveedores.ts). " +
            "Formato: '<esquema-de-proveedor> <credencial>' — en desarrollo, 'dev <sujetoId>'.",
        },
        [ESQUEMA_SERVIDOR_BATALLA_AUTH]: {
          type: 'apiKey',
          in: 'header',
          name: 'Authorization',
          description:
            "Credencial de un servidor de batalla de Conquest declarado en SERVIDORES_BATALLA (doc 02 §3.3), nunca la de " +
            "un jugador. Formato: 'batalla-servidor <token>'.",
        },
      },
    },
  },
};
