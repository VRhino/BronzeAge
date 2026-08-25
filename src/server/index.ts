// Punto de arranque del proceso backend (Fastify + RunnerDePartida). Ejecutable con `npm run server`.
//
// NO lo usa `main.ts`: ese sigue siendo el cliente de navegador local sobre `GameStore`/`GameSession` en
// memoria. Migrarlo para hablar con esta API es la tarea siguiente del doc 4, todavía sin abordar — hasta
// entonces este es un proceso Node completamente aparte, sin ningún consumidor real todavía.
import { crearServidor } from './api';

const PUERTO = Number(process.env.PUERTO ?? 3000);
const DIRECTORIO_PARTIDAS = process.env.DIRECTORIO_PARTIDAS ?? './partidas';

const app = crearServidor({ directorio: DIRECTORIO_PARTIDAS });

app
  .listen({ port: PUERTO, host: '0.0.0.0' })
  .then(() => {
    console.log(`servidor escuchando en :${PUERTO} — partidas en '${DIRECTORIO_PARTIDAS}'`);
  })
  .catch((err: unknown) => {
    console.error(err);
    process.exitCode = 1;
  });
