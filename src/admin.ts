// Panel de administración (`admin.html`) — mismo patrón que `lab/main.ts`: punto de entrada aparte de
// `main.ts`/`app/gameStore.ts` (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md, Fase B3). Habla directo con
// `apiCliente.ts`, sin pasar por `GameStore`: no necesita historial, `subscribe`/`notify` ni ninguna de las
// ~50 consultas de jugador, solo esta única operación.
//
// Alcance de ESTA pasada, acotado a propósito: solo "crear/regenerar mundo" — es la única pieza que BLOQUEA a
// `main.ts` (sin una partida creada, el jugador no tiene a qué conectarse). Balance, importar partida y el
// histórico de ticks son candidatos a sumarse aquí en una vuelta futura de este mismo panel; hasta entonces no
// tienen dueño.
import type { RegionId } from './domain/types';
import { ApiError, crearOResumirPartida } from './app/apiCliente';

const gameIdInput = document.getElementById('admin-gameid') as HTMLInputElement;
const seedInput = document.getElementById('admin-seed') as HTMLInputElement;
const regionSelect = document.getElementById('admin-region') as HTMLSelectElement;
const crearBtn = document.getElementById('admin-crear') as HTMLButtonElement;
const statusEl = document.getElementById('admin-status')!;

function region(): RegionId | undefined {
  return regionSelect.value === '' ? undefined : (regionSelect.value as RegionId);
}

crearBtn.addEventListener('click', () => {
  void crearORegenerar();
});

async function crearORegenerar(): Promise<void> {
  const gameId = gameIdInput.value.trim();
  const seed = Number(seedInput.value) || 1;
  if (!gameId) {
    statusEl.textContent = 'Falta el gameId.';
    return;
  }

  crearBtn.disabled = true;
  statusEl.textContent = 'Creando...';
  try {
    const resumen = await crearOResumirPartida(gameId, seed, region());
    statusEl.textContent = `Lista: gameId=${resumen.gameId}, tick=${resumen.tick}, version=${resumen.version}.`;
  } catch (err) {
    if (err instanceof ApiError && err.status === 409) {
      const confirmado = window.confirm(
        `Ya existe una partida abierta para '${gameId}'. ¿Descartarla y crear una nueva? Se pierde todo el progreso.`
      );
      if (!confirmado) {
        statusEl.textContent = 'Cancelado — la partida existente sigue intacta.';
        crearBtn.disabled = false;
        return;
      }
      try {
        statusEl.textContent = 'Reiniciando...';
        const resumen = await crearOResumirPartida(gameId, seed, region(), true);
        statusEl.textContent = `Reiniciada: gameId=${resumen.gameId}, tick=${resumen.tick}, version=${resumen.version}.`;
      } catch (err2) {
        statusEl.textContent = err2 instanceof Error ? `Error: ${err2.message}` : 'Error desconocido.';
      }
    } else {
      statusEl.textContent = err instanceof Error ? `Error: ${err.message}` : 'Error desconocido.';
    }
  } finally {
    crearBtn.disabled = false;
  }
}
