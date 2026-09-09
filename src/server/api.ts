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
import { RegistroDeAuditoria } from './auditoria';
import { TareaDeMantenimiento, type ConfiguracionMantenimiento } from './mantenimiento';
import { registrarRutasDeSesion } from './rutas/sesiones';
import { registrarRutasDeAdmin } from './rutas/admin';
import { registrarRutasDeJugador } from './rutas/jugador';
import { registrarRutaDeTiempoReal } from './rutas/tiempoReal';
import { registrarRutaDeBalance } from './rutas/balance';
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
  /**
   * Trabajo pendiente que hay que DRENAR al cerrar la instancia, además de las partidas.
   *
   * Existe por el repositorio de identidad en disco, que encola sus escrituras en segundo plano: hasta ahora
   * había que acordarse de `esperarEscrituras()` A MANO antes de `app.close()` (así lo hace el handler de
   * SIGINT en `index.ts`), y quien no lo hiciera perdía la última escritura o —en Windows— chocaba el
   * `rename` en vuelo contra el `rmdir` del directorio temporal. Eso hacía intermitente un test de la API
   * con `ENOTEMPTY`, y era el mismo fallo que ya se corrigió una vez en `persistenciaIdentidad.test.ts`.
   *
   * Con el gancho aquí, **cerrar el servidor drena de verdad**: ni el proceso ni los tests tienen que
   * recordar el orden correcto.
   */
  alCerrar?: () => Promise<void>;
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
  /** Fuente de ticks (Fase C12) — ver el comentario de `RegistroDePartidas`. `undefined` por defecto: sin
   * configurarlo, ninguna partida avanza sola, ni siquiera las de los tests que crean un servidor con
   * `crearServidor({directorio})` sin este campo. */
  intervaloTickMs?: number;
  /** Respaldos y poda automáticos (Fase E2, `TareaDeMantenimiento`). `undefined` por defecto, mismo criterio
   * que `intervaloTickMs`: sin configurarlo no se escribe ni se borra nada por su cuenta — un default que
   * borra archivos es el que nadie nota hasta que ya borró algo que hacía falta. */
  mantenimiento?: ConfiguracionMantenimiento;
  /** Código de invitación exigido en `POST /v1/registro` (alta de cuenta local). `undefined` = registro
   * abierto. */
  codigoRegistro?: string;
}

export function crearServidor(opciones: OpcionesServidor): FastifyInstance {
  // `coerceTypes: false` (Fastify por defecto lo trae a `true`): un cuerpo JSON ya llega tipado por
  // `JSON.parse` — a diferencia de query/params de URL, que SIEMPRE son texto y necesitan coerción para tener
  // sentido. Coercionar aquí (`nombre: 123` -> `"123"`) es exactamente lo que el esquema por comando de la
  // Fase C9 viene a evitar: un cliente con un bug de tipos debe ver un 400, no que el servidor le adivine lo
  // que quiso decir.
  const app = Fastify({ logger: false, ajv: { customOptions: { coerceTypes: false } } });
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

  const ahora = opciones.ahora ?? (() => new Date().toISOString());
  const deps: DependenciasDeRutas = {
    identidad,
    administradores: crearDirectorioDeAdministradores(opciones.administradoresGlobales ?? [], identidad.repositorio),
    // Mismo reloj de pared que el resto del servidor: así el reloj de mundo de cada partida y su catch-up
    // (D5, `RunnerDePartida.iniciarRelojDeMundo`) son inyectables en tests, no solo el reloj del sistema.
    partidas: new RegistroDePartidas(opciones.directorio, opciones.intervaloTickMs, ahora),
    ahora,
    hub: opciones.hub ?? new HubDeDifusion(),
    // Fase E2. Mismo directorio que los snapshots —una partida y su auditoría se copian, archivan y borran
    // juntas— y el mismo reloj de pared inyectado que el resto del servidor, para que un test pueda fechar
    // sus líneas de forma determinista en vez de depender de la hora del sistema.
    auditoria: new RegistroDeAuditoria(opciones.directorio, ahora),
    codigoRegistro: opciones.codigoRegistro,
  };

  // Apagado limpio: al cerrar la instancia, parar el reloj de mundo de cada partida abierta y dejar drenar
  // su cola (un tick a medio persistir no se aborta). `app.close()` —lo llama el handler de SIGINT/SIGTERM
  // en `index.ts`, y todos los tests en su `afterEach`— dispara este hook.
  // Mantenimiento (Fase E2): respaldos y poda periódicos. Solo si se configura.
  const mantenimiento = opciones.mantenimiento ? new TareaDeMantenimiento(opciones.directorio, opciones.mantenimiento, ahora) : undefined;
  mantenimiento?.iniciar();

  app.addHook('onClose', async () => {
    mantenimiento?.detener();
    await deps.partidas.cerrar();
    // La auditoría se escribe sin esperar (`registrar` no devuelve promesa, ver `auditoria.ts`), así que sin
    // este drenaje las últimas líneas se perderían al apagar — justo las del incidente que hizo apagar.
    await deps.auditoria.drenar();
    await opciones.alCerrar?.();
  });

  // Todas las superficies bajo /v1 (Fase C6) — ver el comentario de cabecera.
  app.register(
    async (v1) => {
      registrarRutasDeSesion(v1, deps);
      registrarRutasDeAdmin(v1, deps);
      registrarRutasDeJugador(v1, deps);
      registrarRutaDeTiempoReal(v1, deps);
      // El balance (Fase C7) es regla pública, no estado de partida — sin autenticar, mismo criterio que
      // `/openapi.json` justo debajo.
      registrarRutaDeBalance(v1);
      // El contrato publicado (doc 4: "para que los repos de cliente generen su cliente tipado"). Sin
      // autenticar a propósito: es lo primero que un cliente nuevo necesita leer, antes incluso de poder
      // hacer login.
      v1.get('/openapi.json', async () => app.swagger());
    },
    { prefix: '/v1' }
  );

  return app;
}
