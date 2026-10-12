// Ruptura 3 del vasallaje (2026-10-12, `Docs/Mechanics/Diplomacia_Facciones.md` §5.5): un tercero que conquista todas las plazas de la vasalla se queda con ella.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, Faccion, RelacionPolitica } from '../../domain/types';
import { pasarVasallasConquistadas } from '../diplomacia';
import { instanteDeTest } from './fixtures';

const faccion = (id: string, derrotadaPor?: string | null): Faccion => ({ id, nombre: id, reyId: `rey-${id}`, embajadorId: null, ciudadanosIds: [], reputacion: 0, experiencia: 0, nivel: 1, ...(derrotadaPor !== undefined ? { derrotadaPor } : {}) }) as unknown as Faccion;
const plaza = (faccionId: string) => ({ id: `p-${faccionId}`, faccionId }) as unknown as Asentamiento;
const vasallaje: RelacionPolitica = { id: 'v', tipo: 'vasallaje', faccionAId: 'senora', faccionBId: 'vasalla', tributo: { recurso: 'madera', cantidadPorMinuto: 4 }, creadoEn: instanteDeTest(0), estado: 'activa' };
const guerra: RelacionPolitica = { id: 'g', tipo: 'guerra', faccionAId: 'tercero', faccionBId: 'vasalla', creadoEn: instanteDeTest(0), estado: 'activa' };
const ahora = instanteDeTest(50);

describe('vasalla conquistada por un tercero', () => {
  const facciones = (d: string | null) => [faccion('senora'), faccion('vasalla', d), faccion('tercero')];

  it('sin plazas y derrotada por un tercero, pasa a su vasalla con el mismo tributo y se acaba la guerra entre ellos', () => {
    const r = pasarVasallasConquistadas([vasallaje, guerra], facciones('tercero'), [plaza('senora'), plaza('tercero')], ahora);
    expect(r.relaciones.find((x) => x.id === 'v')!.estado).toBe('rota');
    expect(r.relaciones.find((x) => x.id === 'g')!.estado).toBe('rota');
    expect(r.relaciones.find((x) => x.tipo === 'vasallaje' && x.estado === 'activa')).toMatchObject({ faccionAId: 'tercero', faccionBId: 'vasalla', tributo: vasallaje.tributo });
    expect(r.eventos.map((e) => (typeof e === 'string' ? e : e.codigo))).toEqual(['diplomacia.vasallo_conquistado']);
  });

  it('no cambia nada si conserva una plaza, si cayó por ruina, si la conquistó su propia señora, o si ya es vasalla del conquistador', () => {
    const base = [plaza('senora'), plaza('tercero')];
    expect(pasarVasallasConquistadas([vasallaje], facciones('tercero'), [...base, plaza('vasalla')], ahora).eventos).toEqual([]);
    expect(pasarVasallasConquistadas([vasallaje], facciones(null), base, ahora).eventos).toEqual([]);
    expect(pasarVasallasConquistadas([vasallaje], facciones('senora'), base, ahora).eventos).toEqual([]);
    const yaSuya: RelacionPolitica = { ...vasallaje, faccionAId: 'tercero' };
    const r = pasarVasallasConquistadas([yaSuya], facciones('tercero'), base, ahora);
    expect(r.eventos).toEqual([]);
    expect(r.relaciones).toEqual([yaSuya]);
  });
});
