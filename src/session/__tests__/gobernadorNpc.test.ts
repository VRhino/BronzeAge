// Toda plaza NPC tiene Gobernador (2026-09-28): el primer fundador o, si no queda ninguno, el primer residente. Una
// plaza conquistada o de la que se mudó su fundador se quedaba sin él para siempre, y sin Gobernador el NPC no
// construye caravanas ni edificios manuales ni pide subidas (en la Era I medida, 25 de 39 plazas).
import { describe, expect, it } from 'vitest';
import { createRng } from '../../worldgen';
import { avanzarNpcGobernanza } from '../npcGobernanza';
import { contextoDeTest, crearEstadoDeTest, crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, heroeDePrueba } from '../../engine/__tests__/fixtures';

describe('Gobernador de una plaza NPC', () => {
  it('sin fundador, lo es su primer residente', () => {
    const mapa = crearMapaDeterminista(7);
    const uno = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
    const plaza = { ...uno.asentamiento, heroesFundadoresIds: [], casasCompradas: ['conquistador'] };
    const facciones = uno.facciones.map((f) => (f.id === 'faccion-1' ? { ...f, ciudadanosIds: [...f.ciudadanosIds, 'conquistador'] } : f));
    const heroe = heroeDePrueba('conquistador', { tipo: 'asentamiento', asentamientoId: plaza.id }, { controlador: 'bot' });
    const r = avanzarNpcGobernanza(crearEstadoDeTest([plaza], facciones, { heroes: [heroe] }), mapa, contextoDeTest(1, createRng(5)), { lanzarCampanas: false });
    expect(r.estado.asentamientos[0]!.cargos.gobernadorId).toBe('conquistador');
  });
});
