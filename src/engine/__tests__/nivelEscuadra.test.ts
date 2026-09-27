// Nivel de escuadra (Doc 5.16.3, decisión del usuario 2026-09-27): +1 % de poder por nivel hasta el 10, con una curva
// que dobla lo que pide cada nivel hasta el 5 y luego se queda plana. Antes cada punto de experiencia daba +5 %, y una
// escuadra que cazaba bandidos cada tick llegaba a 46 veces su poder en una semana.
import { describe, expect, it } from 'vitest';
import { MILITAR } from '../../constants';
import { conExperiencia, nivelDeEscuadra, poderEscuadron } from '../tropa';
import { aplicarBajas } from '../combate';
import { escuadronDePrueba } from './fixtures';

const pide = MILITAR.experienciaParaSubirEscuadra;
const hasta = (nivel: number) => pide.slice(0, nivel - 1).reduce((a, b) => a + b, 0);

describe('nivel de escuadra', () => {
  it('sube al llenar lo que pide cada nivel', () => {
    expect(nivelDeEscuadra(0)).toBe(1);
    expect(nivelDeEscuadra(pide[0]! - 0.5)).toBe(1);
    expect(nivelDeEscuadra(pide[0]!)).toBe(2);
    expect(nivelDeEscuadra(hasta(5))).toBe(5);
    expect(nivelDeEscuadra(hasta(5) - 1)).toBe(4);
  });

  it('dobla lo que pide hasta el nivel 5 y luego se queda plano', () => {
    expect(pide.slice(0, 4)).toEqual([pide[0]!, pide[0]! * 2, pide[0]! * 4, pide[0]! * 8]);
    expect(new Set(pide.slice(3)).size).toBe(1);
  });

  it('no pasa del nivel 10: como mucho, un 10 % más de poder', () => {
    expect(nivelDeEscuadra(1_000_000)).toBe(MILITAR.nivelMaximoEscuadra);
    const novata = escuadronDePrueba('a', 'h', 'milicia_lanceros', 25);
    const veterana = conExperiencia(novata, 1_000_000);
    expect(poderEscuadron(veterana) / poderEscuadron(novata)).toBeCloseTo((1 + 10 * MILITAR.bonusPoderPorNivelEscuadra) / (1 + MILITAR.bonusPoderPorNivelEscuadra));
  });

  it('el combate con números da experiencia y deja el nivel al día', () => {
    const e = { ...escuadronDePrueba('a', 'h', 'milicia_lanceros', 25), experiencia: pide[0]! - MILITAR.experienciaGanadaPorVictoria };
    expect(aplicarBajas([e], 0, true)[0]!.nivel).toBe(2);
  });
});
