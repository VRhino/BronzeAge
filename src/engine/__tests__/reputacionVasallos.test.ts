// Decisiones del usuario sobre reputación (2026-10-12, `Docs/Mechanics/Diplomacia_Facciones.md`): defender o no al vasallo, y guerra tras romper una alianza.
import { describe, expect, it } from 'vitest';
import type { BattleResult } from '../../contratos/v1/dto';
import { REPUTACION } from '../../constants';
import { dias, sumar } from '../../domain/tiempo';
import type { Faccion, RelacionPolitica } from '../../domain/types';
import type { GameSessionState } from '../../session/estado';
import type { Batalla } from '../../session/batallas';
import { conReputacionDeVasallos } from '../../session/resultadoBatalla';
import { ajustesPorGuerraTrasRomper, romperRelacion } from '../diplomacia';
import { instanteDeTest } from './fixtures';

const faccion = (id: string): Faccion => ({ id, nombre: id, reyId: `rey-${id}`, embajadorId: null, ciudadanosIds: [], reputacion: 0, experiencia: 0, nivel: 1 }) as unknown as Faccion;
const vasallaje: RelacionPolitica = { id: 'v', tipo: 'vasallaje', faccionAId: 'senora', faccionBId: 'vasalla', creadoEn: instanteDeTest(0), estado: 'activa' };

describe('defender al vasallo (batalla)', () => {
  const batalla = (heroesDefensores: Array<{ heroeId: string; faccionId: string }>): Batalla =>
    ({
      ticket: { bandos: { atacante: { faccionId: 'enemiga', participantes: [] }, defensor: { faccionId: 'vasalla', participantes: heroesDefensores.map((h) => ({ ...h, escuadras: [] })) } } },
      incorporaciones: [],
    }) as unknown as Batalla;
  const resultado = (ganador: 'atacante' | 'defensor', ids: string[]) => ({ ganador, porHeroe: ids.map((heroeId) => ({ heroeId, participo: true })) }) as unknown as BattleResult;
  const estado = () => ({ facciones: [faccion('senora'), faccion('vasalla')], relaciones: [vasallaje] }) as unknown as GameSessionState;
  const rep = (e: GameSessionState) => e.facciones.find((f) => f.id === 'senora')!.reputacion;

  it('un héroe de la señora en una defensa que aguanta le da reputación', () => {
    const b = batalla([{ heroeId: 'h1', faccionId: 'senora' }]);
    expect(rep(conReputacionDeVasallos(estado(), b, resultado('defensor', ['h1'])))).toBe(REPUTACION.bonusDefenderVasallo);
  });

  it('si el defensor cae sin ningún héroe de la señora, ella pierde reputación; si había uno, no', () => {
    expect(rep(conReputacionDeVasallos(estado(), batalla([]), resultado('atacante', [])))).toBe(REPUTACION.penalizacionNoDefenderVasallo);
    const b = batalla([{ heroeId: 'h1', faccionId: 'senora' }]);
    expect(rep(conReputacionDeVasallos(estado(), b, resultado('atacante', ['h1'])))).toBe(0);
  });

  it('si la defensa aguanta sin la señora, o el defensor no es vasallo, nada cambia', () => {
    expect(rep(conReputacionDeVasallos(estado(), batalla([]), resultado('defensor', [])))).toBe(0);
    const libre = { ...estado(), relaciones: [] } as unknown as GameSessionState;
    expect(rep(conReputacionDeVasallos(libre, batalla([]), resultado('atacante', [])))).toBe(0);
  });
});

describe('declarar la guerra a quien era aliada y se rompió hace poco', () => {
  const alianza: RelacionPolitica = { id: 'a', tipo: 'alianza', faccionAId: 'x', faccionBId: 'y', creadoEn: instanteDeTest(0), estado: 'activa' };

  it('romper una alianza anota quién y cuándo; la guerra dentro de la ventana penaliza a quien rompió', () => {
    const t0 = instanteDeTest(10);
    const { relaciones } = romperRelacion([faccion('x'), faccion('y')], [alianza], 'a', 'x', t0);
    expect(relaciones[0]).toMatchObject({ estado: 'rota', rotaPor: 'x', rotaEn: t0 });
    const dentro = sumar(t0, dias(REPUTACION.ventanaGuerraTrasRomperAlianzaDias - 1));
    expect(ajustesPorGuerraTrasRomper(relaciones, 'x', 'y', dentro)).toEqual([expect.objectContaining({ faccionId: 'x', delta: REPUTACION.penalizacionGuerraTrasRomperAlianza })]);
    expect(ajustesPorGuerraTrasRomper(relaciones, 'y', 'x', dentro), 'quien no rompió no paga').toEqual([]);
    expect(ajustesPorGuerraTrasRomper(relaciones, 'x', 'y', sumar(t0, dias(REPUTACION.ventanaGuerraTrasRomperAlianzaDias))), 'pasada la ventana').toEqual([]);
  });
});
