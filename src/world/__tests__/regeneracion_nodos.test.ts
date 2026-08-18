// Regeneración de yacimientos agotados (a petición del usuario): un nodo que llega a stock 0 debe volver a
// aparecer con su `cantidadInicial` completa pasados N ticks de cooldown — no antes, no quedarse agotado
// para siempre — y livestock debe regenerar más rápido que un yacimiento mineral (`REGENERACION_NODOS`).
import { describe, expect, it } from 'vitest';
import type { NodoRecurso } from '../../domain/types';
import { REGENERACION_NODOS } from '../../constants';
import { generarMapa, MAPA_DEFAULT } from '../../worldgen';
import { crearEstadoMapa, crearMapa } from '../mapa';

const SEED = 42;

function conMundo() {
  const generado = generarMapa({ ancho: MAPA_DEFAULT.ancho, alto: MAPA_DEFAULT.alto, seed: SEED });
  const estado = crearEstadoMapa();
  const mapa = crearMapa(generado, estado);
  return { generado, estado, mapa };
}

function agotar(mapa: ReturnType<typeof crearMapa>, nodo: NodoRecurso): void {
  const extraido = mapa.extraer(nodo.id, nodo.cantidadInicial);
  expect(extraido).toBe(nodo.cantidadInicial);
  expect(mapa.stock(nodo.id)).toBe(0);
}

describe('Mapa.avanzarRegeneracion — yacimientos agotados', () => {
  it('livestock tiene menos cooldown que un yacimiento mineral (regenera más rápido)', () => {
    expect(REGENERACION_NODOS.livestock.ticksCooldown).toBeLessThan(REGENERACION_NODOS.metales.ticksCooldown);
  });

  it('no regenera antes del cooldown y regenera a cantidadInicial completa justo al cumplirse', () => {
    const { generado, mapa } = conMundo();
    const mineral = generado.nodos.find((n) => n.tipo !== 'livestock');
    if (!mineral) throw new Error('El mundo de test no generó ningún nodo mineral — revisa la seed.');

    agotar(mapa, mineral);

    // Tick en el que se agota de verdad (primera llamada tras la extracción): solo agenda, no regenera.
    const tickAgotamiento = 1;
    let eventos = mapa.avanzarRegeneracion(tickAgotamiento);
    expect(eventos).toHaveLength(0);
    expect(mapa.stock(mineral.id)).toBe(0);

    const tickRegen = tickAgotamiento + REGENERACION_NODOS.metales.ticksCooldown;
    for (let tick = tickAgotamiento + 1; tick < tickRegen; tick++) {
      eventos = mapa.avanzarRegeneracion(tick);
      expect(eventos).toHaveLength(0);
      expect(mapa.stock(mineral.id)).toBe(0);
    }

    eventos = mapa.avanzarRegeneracion(tickRegen);
    expect(eventos).toHaveLength(1);
    expect(mapa.stock(mineral.id)).toBe(mineral.cantidadInicial);
  });

  it('un nodo livestock agotado el mismo tick que uno mineral regenera antes (cooldown menor)', () => {
    const { generado, mapa } = conMundo();
    const livestock = generado.nodos.find((n) => n.tipo === 'livestock');
    const mineral = generado.nodos.find((n) => n.tipo !== 'livestock');
    if (!livestock || !mineral) throw new Error('El mundo de test no generó livestock y mineral a la vez — revisa la seed.');

    agotar(mapa, livestock);
    agotar(mapa, mineral);

    const tickAgotamiento = 5;
    mapa.avanzarRegeneracion(tickAgotamiento); // agenda ambos

    const tickRegenLivestock = tickAgotamiento + REGENERACION_NODOS.livestock.ticksCooldown;
    const eventosEnRegenLivestock = mapa.avanzarRegeneracion(tickRegenLivestock);

    expect(mapa.stock(livestock.id)).toBe(livestock.cantidadInicial);
    expect(eventosEnRegenLivestock.some((e) => e.includes(livestock.id))).toBe(true);
    // El mineral todavía no le toca (su cooldown es mayor).
    expect(mapa.stock(mineral.id)).toBe(0);
  });
});
