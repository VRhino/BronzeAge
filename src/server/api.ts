// Raíz de composición del servidor HTTP: construye las dependencias (identidad, administradores, registro de
// partidas) y registra las tres superficies. Ninguna decisión de negocio vive aquí — las rutas están en
// `rutas/`, y lo que cada rol puede hacer, en `acceso/rolesDePartida.ts`.
//
// Superficies (Fase C3, doc 5) + tiempo real (Fase C5, doc 6 §2), todas bajo `/v1` (Fase C6 — ver más abajo):
//   - `/v1/sesiones`            login y whoami; la puerta a las otras dos
//   - `/v1/admin/*`             gobierno de la partida: crear, tick, estado completo. Rol de administración
//   - `/v1/jugador/*`           unirse, ejecutar comandos, y `/tiempo-real` (WebSocket con canales suscribibles)
//
// Antes de C3 había una sola superficie sin prefijo, con crear/tick/estado SIN autenticar. Ya no existe: no
// se dejaron alias de compatibilidad a propósito, porque un alias abierto habría mantenido el agujero
// abierto — que era justo lo que C3 venía a cerrar. Mismo criterio ahora con `/v1`: sin alias sin versión.
//
// Versionado por PREFIJO DE RUTA, no por cabecera (Fase C6, doc 4): cualquier cliente HTTP lo soporta sin
// configuración especial (`curl`, un proxy, un navegador pegando la URL), se ve en cualquier log de acceso
// sin inspeccionar cabeceras, y es lo que ya asume el propio `openapi.json` publicado más abajo (`servers`).
// El único coste es que una v2 futura convive con `/v1` en la misma URL en vez de negociarse — aceptable:
// con un solo consumidor externo hoy, no hay negociación de versión que hacer todavía.
import Fastify, { type FastifyInstance } from 'fastify';
import fastifyWebsocket from '@fastify/websocket';
import fastifyCors from '@fastify/cors';
import fastifySwagger from '@fastify/swagger';
import { crearRegistroProveedores } from '../acceso/proveedorIdentidad';
import type { ContextoAutenticacion } from '../acceso/servicioAutenticacion';
import { proveedoresPorDefecto } from './identidad/proveedoresActivos';
import { crearRepositorioIdentidadEnMemoria } from './identidad/repositorioEnMemoria';
import { crearDirectorioDeAdministradores, type AdministradorConfigurado } from './identidad/administradoresGlobales';
import { RegistroDePartidas } from './registroDePartidas';
import { HubDeDifusion } from './difusion/hub';
import { registrarRutasDeSesion } from './rutas/sesiones';
import { registrarRutasDeAdmin } from './rutas/admin';
import { registrarRutasDeJugador } from './rutas/jugador';
import { registrarRutaDeTiempoReal } from './rutas/tiempoReal';
import { opcionesOpenApi } from './openapi';
import type { DependenciasDeRutas } from './rutas/contexto';

export interface OpcionesServidor {
  /** Directorio donde `persistenciaPartida.ts` guarda los snapshots. */
  directorio: string;
  /** Contexto de autenticación (proveedores + repositorio). Por defecto, solo el proveedor de desarrollo
   * sobre un repositorio en memoria — inyectable para tests o para un proceso con otros proveedores. */
  identidad?: ContextoAutenticacion;
  /**
   * Identidades externas (`proveedor:sujetoId`) con rol `administrador_global`. **Vacío por defecto**: sin
   * configurarlo, nadie puede crear partidas. Es deliberado — conceder administración por omisión es la
   * clase de default que sobrevive hasta producción sin que nadie lo note.
   */
  administradoresGlobales?: readonly AdministradorConfigurado[];
  /** Reloj inyectado, como en el resto del proyecto: decide la vigencia de sesiones y membresías. */
  ahora?: () => string;
  /** Hub de difusión WebSocket (Fase C5). Inyectable por el mismo motivo que `identidad`: los tests
   * necesitan poder inspeccionarlo (`conexionesAbiertas`) sin exponer un decorador público en la instancia
   * de Fastify. Por defecto, uno nuevo y vacío. */
  hub?: HubDeDifusion;
  /**
   * Orígenes permitidos para CORS (Fase C6). **Vacío por defecto**: sin configurarlo, ningún origen cruzado
   * puede llamar a esta API — mismo criterio que `administradoresGlobales`, un default permisivo es el que
   * sobrevive hasta producción sin que nadie lo note. En desarrollo no hace falta: el proxy de Vite de
   * `cliente/` hace que las peticiones sean same-origin.
   */
  origenesPermitidos?: readonly string[];
}

export function crearServidor(opciones: OpcionesServidor): FastifyInstance {
  const app = Fastify({ logger: false });
  app.register(fastifyWebsocket);
  app.register(fastifyCors, {
    // `[]`/ausente -> `false` (CORS desactivado del todo, ningún origen cruzado pasa) en vez de un array
    // vacío: es la forma inequívoca de decir "nada permitido" en `@fastify/cors`, sin depender de cómo
    // interprete un array vacío.
    origin: opciones.origenesPermitidos && opciones.origenesPermitidos.length > 0 ? [...opciones.origenesPermitidos] : false,
  });
  app.register(fastifySwagger, opcionesOpenApi);

  const identidad: ContextoAutenticacion = opciones.identidad ?? {
    proveedores: crearRegistroProveedores(proveedoresPorDefecto()),
    repositorio: crearRepositorioIdentidadEnMemoria(),
  };

  const deps: DependenciasDeRutas = {
    identidad,
    administradores: crearDirectorioDeAdministradores(opciones.administradoresGlobales ?? [], identidad.repositorio),
    partidas: new RegistroDePartidas(opciones.directorio),
    ahora: opciones.ahora ?? (() => new Date().toISOString()),
    hub: opciones.hub ?? new HubDeDifusion(),
  };

  // Todas las superficies bajo /v1 (Fase C6) — ver el comentario de cabecera.
  app.register(
    async (v1) => {
      registrarRutasDeSesion(v1, deps);
      registrarRutasDeAdmin(v1, deps);
      registrarRutasDeJugador(v1, deps);
      registrarRutaDeTiempoReal(v1, deps);
      // El contrato publicado (doc 4: "para que los repos de cliente generen su cliente tipado"). Sin
      // autenticar a propósito: es lo primero que un cliente nuevo necesita leer, antes incluso de poder
      // hacer login.
      v1.get('/openapi.json', async () => app.swagger());
    },
    { prefix: '/v1' }
  );

  return app;
}
