// Punto de arranque del proceso backend (Fastify + RunnerDePartida). Ejecutable con `npm run server`.
//
// Aquí, y solo aquí, se decide la configuración del despliegue: puerto, dónde viven las partidas y QUIÉN
// administra la instancia. `crearServidor()` no trae ningún administrador por defecto — si esta variable no
// se declara, nadie puede crear partidas. Es a propósito: un default que concede administración es de los
// que sobreviven hasta producción sin que nadie los vea.
//
// `ADMINISTRADORES` es una lista `proveedor:sujetoId` separada por comas, ej. `dev:jefa,oauth:1234`.
import { crearServidor } from './api';
import { parsearAdministradores } from './identidad/administradoresGlobales';

const PUERTO = Number(process.env.PUERTO ?? 3000);
const DIRECTORIO_PARTIDAS = process.env.DIRECTORIO_PARTIDAS ?? './partidas';
const ADMINISTRADORES = parsearAdministradores(process.env.ADMINISTRADORES);

const app = crearServidor({ directorio: DIRECTORIO_PARTIDAS, administradoresGlobales: ADMINISTRADORES });

app
  .listen({ port: PUERTO, host: '0.0.0.0' })
  .then(() => {
    console.log(`servidor escuchando en :${PUERTO} — partidas en '${DIRECTORIO_PARTIDAS}'`);
    if (ADMINISTRADORES.length === 0) {
      console.warn('AVISO: sin ADMINISTRADORES configurados — nadie puede crear partidas.');
      console.warn("       ej: ADMINISTRADORES='dev:jefa' npm run server");
    } else {
      console.log(`administradores: ${ADMINISTRADORES.map((a) => `${a.proveedor}:${a.sujetoId}`).join(', ')}`);
    }
  })
  .catch((err: unknown) => {
    console.error(err);
    process.exitCode = 1;
  });
