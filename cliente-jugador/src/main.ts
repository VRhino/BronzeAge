// Boilerplate del cliente de jugador — ver README.md. Arranque mínimo: login, unirse a la partida, pintar el
// terreno con `terreno/` (cero motor) y mostrar la proyección propia. Punto de partida para construir encima,
// no un cliente completo.
import { ApiError, consultarProyeccion, obtenerMapa, unirseAPartida, type ProyeccionJugador } from './apiCliente';
import { pintarTerreno } from './render';
import type { MapaGenerado } from './terreno';

const GAME_ID = import.meta.env.VITE_GAME_ID ?? 'local';

const app = document.querySelector<HTMLDivElement>('#app')!;
app.innerHTML = `
  <h1>Bronze Age Collapse — cliente de jugador (boilerplate)</h1>
  <p id="estado">Conectando…</p>
  <canvas id="mapa" width="500" height="500"></canvas>
  <pre id="proyeccion"></pre>
`;

const elEstado = document.querySelector<HTMLParagraphElement>('#estado')!;
const elProyeccion = document.querySelector<HTMLPreElement>('#proyeccion')!;
const canvas = document.querySelector<HTMLCanvasElement>('#mapa')!;
const ctx = canvas.getContext('2d')!;

let mapaCache: { id: string; mapa: MapaGenerado } | null = null;

async function sincronizarMapa(mapaId: string, gameId: string): Promise<MapaGenerado> {
  if (mapaCache?.id === mapaId) return mapaCache.mapa;
  const mapa = await obtenerMapa(gameId, mapaId);
  mapaCache = { id: mapaId, mapa };
  return mapa;
}

async function refrescar(): Promise<void> {
  const proyeccion: ProyeccionJugador = await consultarProyeccion(GAME_ID);
  const mapa = await sincronizarMapa(proyeccion.mapaId, GAME_ID);

  const escala = canvas.width / mapa.config.ancho;
  pintarTerreno(ctx, mapa, escala);

  elEstado.textContent = `Conectado a '${GAME_ID}' — tick ${proyeccion.tick}, faccionId: ${proyeccion.faccionId ?? '(ninguna)'}`;
  elProyeccion.textContent = JSON.stringify(proyeccion, null, 2);
}

async function arrancar(): Promise<void> {
  try {
    await unirseAPartida(GAME_ID);
  } catch (err) {
    // 409: ya eras miembro (recarga de página) — no es un fallo, seguir directo a leer el estado.
    if (!(err instanceof ApiError) || err.status !== 409) throw err;
  }
  await refrescar();
}

arrancar().catch((err: unknown) => {
  elEstado.textContent = err instanceof ApiError ? `Error: ${err.message}` : 'Error desconocido al conectar.';
  console.error(err);
});

// Para ejecutar un comando: `ejecutarComando(GAME_ID, 'crearFaccion', { nombre: 'Micenas' }).then(refrescar)`
// — importar `ejecutarComando` de `./apiCliente`. La forma de `params` por comando está en
// `GET /v1/openapi.json` (Fase C9), no en este archivo.
