// Punto de arranque del proceso backend (Fastify + RunnerDePartida). Ejecutable con `npm run server`.
//
// Aquí, y solo aquí, se decide la configuración del despliegue: puerto, dónde viven las partidas y QUIÉN
// administra la instancia. `crearServidor()` no trae ningún administrador por defecto — si esta variable no
// se declara, nadie puede crear partidas. Es a propósito: un default que concede administración es de los
// que sobreviven hasta producción sin que nadie los vea.
//
// `ADMINISTRADORES` es una lista `proveedor:sujetoId` separada por comas, ej. `dev:jefa,oauth:1234`.
//
// `ORIGENES_PERMITIDOS` (Fase C6, CORS) es una lista de orígenes separada por comas, ej.
// `https://jugador.ejemplo.com,https://admin.ejemplo.com`. Vacía por defecto: sin ella, ningún origen
// cruzado puede llamar a esta API — mismo criterio que `ADMINISTRADORES`.
//
// `INTERVALO_TICK_MS` (Fase C12) arranca el scheduler de ticks automáticos para cada partida que se abra.
// Sin declarar, `undefined` — ninguna partida avanza sola (ver el comentario de `RegistroDePartidas`). El
// valor, si se declara, es un PLACEHOLDER: el ritmo de juego real no está decidido en ningún doc de este
// repo, esto solo existe para que el mundo no quede congelado en un despliegue real.
import { crearServidor } from './api';
import { parsearAdministradores } from './identidad/administradoresGlobales';

const PUERTO = Number(process.env.PUERTO ?? 3000);
const DIRECTORIO_PARTIDAS = process.env.DIRECTORIO_PARTIDAS ?? './partidas';
const ADMINISTRADORES = parsearAdministradores(process.env.ADMINISTRADORES);
const ORIGENES_PERMITIDOS = (process.env.ORIGENES_PERMITIDOS ?? '')
  .split(',')
  .map((o) => o.trim())
  .filter((o) => o !== '');
const INTERVALO_TICK_MS = process.env.INTERVALO_TICK_MS ? Number(process.env.INTERVALO_TICK_MS) : undefined;

const app = crearServidor({
  directorio: DIRECTORIO_PARTIDAS,
  administradoresGlobales: ADMINISTRADORES,
  origenesPermitidos: ORIGENES_PERMITIDOS,
  intervaloTickMs: INTERVALO_TICK_MS,
});

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
    if (ORIGENES_PERMITIDOS.length === 0) {
      console.warn('AVISO: sin ORIGENES_PERMITIDOS configurados — CORS desactivado, ningún origen cruzado puede llamar a esta API.');
    } else {
      console.log(`origenes CORS permitidos: ${ORIGENES_PERMITIDOS.join(', ')}`);
    }
    if (INTERVALO_TICK_MS === undefined) {
      console.warn('AVISO: sin INTERVALO_TICK_MS configurado — ninguna partida avanza sola, solo con POST .../tick a mano.');
    } else {
      console.log(`ticks automáticos cada ${INTERVALO_TICK_MS} ms (placeholder, sin decisión de ritmo de juego todavía).`);
    }
  })
  .catch((err: unknown) => {
    console.error(err);
    process.exitCode = 1;
  });
