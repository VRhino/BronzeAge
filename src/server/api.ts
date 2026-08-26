// Raíz de composición del servidor HTTP: construye las dependencias (identidad, administradores, registro de
// partidas) y registra las tres superficies. Ninguna decisión de negocio vive aquí — las rutas están en
// `rutas/`, y lo que cada rol puede hacer, en `acceso/rolesDePartida.ts`.
//
// Superficies (Fase C3, doc 5) + tiempo real (Fase C5, doc 6 §2):
//   - `/sesiones`            login y whoami; la puerta a las otras dos
//   - `/admin/*`             gobierno de la partida: crear, tick, estado completo. Rol de administración
//   - `/jugador/*`           unirse, ejecutar comandos, y `/tiempo-real` (WebSocket con canales suscribibles)
//
// Antes de C3 había una sola superficie sin prefijo, con crear/tick/estado SIN autenticar. Ya no existe: no
// se dejaron alias de compatibilidad a propósito, porque un alias abierto habría mantenido el agujero
// abierto — que era justo lo que C3 venía a cerrar.
//
// `crearServidor()` construye la instancia SIN escuchar ningún puerto — eso lo decide el llamador
// (`index.ts` en producción, los tests vía `.inject()`). Es el patrón recomendado de Fastify para probar
// rutas sin abrir sockets de verdad.
import Fastify, { type FastifyInstance } from 'fastify';
import fastifyWebsocket from '@fastify/websocket';
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
}

export function crearServidor(opciones: OpcionesServidor): FastifyInstance {
  const app = Fastify({ logger: false });
  app.register(fastifyWebsocket);

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

  registrarRutasDeSesion(app, deps);
  registrarRutasDeAdmin(app, deps);
  registrarRutasDeJugador(app, deps);
  registrarRutaDeTiempoReal(app, deps);

  return app;
}
