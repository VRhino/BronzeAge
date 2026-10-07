// El plan de llegadas de los bots (D56, D57): grupos de cinco amigos, pocos solitarios y muchos tardíos, para que se junten en Facciones
// grandes y estables en vez de una nube de Facciones de uno.
import { describe, expect, it } from 'vitest';
import { AMIGOS_POR_GRUPO, planDeLlegadas } from '../llegadas';

const SEMILLAS = Array.from({ length: 200 }, (_, i) => i + 1);

describe('planDeLlegadas', () => {
  it('suma exactamente los bots pedidos, y los amigos llegan de cinco en cinco', () => {
    for (const semilla of SEMILLAS) {
      const plan = planDeLlegadas(semilla, 30, 3);
      expect(plan.reduce((n, l) => n + l.cuantos, 0)).toBeGreaterThanOrEqual(30);
      for (const l of plan) expect(l.cuantos).toBe(l.perfil === 'amigos' ? AMIGOS_POR_GRUPO : 1);
    }
    expect(AMIGOS_POR_GRUPO).toBe(5);
  });

  it('por bots: la mayoría amigos, muy pocos solitarios y bastantes tardíos', () => {
    const bots = { amigos: 0, solitario: 0, tardio: 0 };
    for (const semilla of SEMILLAS) for (const l of planDeLlegadas(semilla, 60, 3)) bots[l.perfil] += l.cuantos;
    const total = bots.amigos + bots.solitario + bots.tardio;
    expect(bots.amigos / total).toBeGreaterThan(0.45);
    expect(bots.solitario / total).toBeLessThan(0.12);
    expect(bots.tardio / total).toBeGreaterThan(0.3);
  });

  it('el primer grupo son siempre amigos (cuando caben), y los tardíos llegan en la segunda mitad', () => {
    const minutos = 3 * 24 * 60;
    for (const semilla of SEMILLAS) {
      const plan = planDeLlegadas(semilla, 30, 3);
      expect(plan.find((l) => l.grupo === 0)!.perfil).toBe('amigos');
      for (const l of plan.filter((x) => x.perfil === 'tardio')) expect(l.tick).toBeGreaterThan(minutos / 2);
    }
  });

  it('con menos bots de los que forman un grupo, el primero funda como solitario y nadie queda como «amigos» incompletos', () => {
    for (const semilla of SEMILLAS) {
      const plan = planDeLlegadas(semilla, 3, 1);
      expect(plan.every((l) => l.perfil !== 'amigos')).toBe(true);
      expect(plan.find((l) => l.grupo === 0)!.perfil).toBe('solitario');
    }
  });

  it('misma semilla, mismo plan', () => {
    expect(planDeLlegadas(7, 30, 3)).toEqual(planDeLlegadas(7, 30, 3));
  });
});
