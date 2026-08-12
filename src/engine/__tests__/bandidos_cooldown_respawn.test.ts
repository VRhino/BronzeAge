// Revisión pedida por el usuario: cada cuánto reaparecen los campamentos de bandidos tras ser destruidos.
// El diseño (Doc 1.9: "REAPARICIÓN: tras destruirse, aparece un campamento nuevo pasados N ticks") ya vive
// en `CAMPAMENTOS_BANDIDOS.ticksRespawn` + `bandidosProximoSpawnTick` (agendado por
// `GameStore.atacarCampamentoBandidos`, ver `app/gameStore.ts`) — este test fija que el gate por tick
// realmente bloquea/permite el respawn en el momento correcto, y que la cifra es la de `CAMPAMENTOS_BANDIDOS`
// (parametrizable en caliente desde el panel de balance, `app/balanceConfig.ts`).
import { describe, expect, it } from 'vitest';
import { computeTodasLasZonas } from '../zones';
import { avanzarSpawnBandidos } from '../bandidos';
import { CAMPAMENTOS_BANDIDOS } from '../../constants';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest } from './fixtures';

const SEED = 42;

describe('cooldown de reaparición de campamentos de bandidos', () => {
  it('no repone un campamento destruido antes de ticksRespawn, y lo repone justo al cumplirse', () => {
    const mapa = crearMapaDeterminista(SEED);
    const facciones = crearFacciones();
    const { asentamiento } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', [], 0);
    const asentamientos = [asentamiento];
    const zonas = computeTodasLasZonas(asentamientos);

    // Primer spawn: cubre al único asentamiento.
    let resultado = avanzarSpawnBandidos([], 0, zonas, asentamientos, mapa, 1, 1);
    expect(resultado.campamentos).toHaveLength(1);

    // Se "destruye" (mismo efecto que `GameStore.atacarCampamentoBandidos`): se quita de la lista y se
    // agenda el próximo tick de spawn permitido.
    const tickDestruccion = 10;
    const proximoSpawnEnTick = tickDestruccion + CAMPAMENTOS_BANDIDOS.ticksRespawn;

    for (let tick = tickDestruccion + 1; tick < proximoSpawnEnTick; tick++) {
      resultado = avanzarSpawnBandidos([], proximoSpawnEnTick, zonas, asentamientos, mapa, tick, tick);
      expect(resultado.campamentos).toHaveLength(0);
    }

    resultado = avanzarSpawnBandidos([], proximoSpawnEnTick, zonas, asentamientos, mapa, proximoSpawnEnTick, proximoSpawnEnTick);
    expect(resultado.campamentos).toHaveLength(1);
    expect(resultado.campamentos[0]?.asentamientoId).toBe(asentamiento.id);
  });
});
