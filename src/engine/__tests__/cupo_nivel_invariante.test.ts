// Invariante de CUPO_NIVEL_ASENTAMIENTO (Doc Fase_0_5 §5): la suma de cupo de nivel 2 + nivel 3 en cada
// nivel de Facción debe cuadrar EXACTO con el presupuesto disponible, para garantizar que siempre queda al
// menos un asentamiento obligatoriamente en nivel 1 — EXCEPTO en Facción nivel 1, donde
// `CAP_FUNDACION_POR_NIVEL` ya es 1 (no hay un segundo asentamiento al que reservarle el puesto de granero,
// ver el comentario de `CUPO_NIVEL_ASENTAMIENTO` en constants.ts). Añadido tras un consejo LLM que detectó
// que la tabla vigente violaba el invariante en Facción nivel 3 y 6 sin que ningún test lo cubriera.
import { describe, expect, it } from 'vitest';
import { CAP_FUNDACION_POR_NIVEL, CUPO_NIVEL_ASENTAMIENTO } from '../../constants';
import { calcularCupoNivel } from '../faccion';

describe('CUPO_NIVEL_ASENTAMIENTO — invariante de cupo total', () => {
  it('la suma cuadra con el presupuesto disponible en todos los niveles de Facción', () => {
    for (let nivelFaccion = 1; nivelFaccion <= CAP_FUNDACION_POR_NIVEL.length; nivelFaccion++) {
      const suma = calcularCupoNivel(nivelFaccion, 2) + calcularCupoNivel(nivelFaccion, 3);
      const cap = CAP_FUNDACION_POR_NIVEL[nivelFaccion - 1]!;
      const esperado = cap > 1 ? cap - 1 : cap;
      expect(suma, `nivel de Facción ${nivelFaccion}: suma=${suma}, esperado=${esperado}`).toBe(esperado);
    }
  });

  it('las curvas son monótonas no decrecientes (más nivel de Facción nunca da menos cupo)', () => {
    for (let i = 1; i < CUPO_NIVEL_ASENTAMIENTO.maxNivel2.length; i++) {
      expect(CUPO_NIVEL_ASENTAMIENTO.maxNivel2[i]).toBeGreaterThanOrEqual(CUPO_NIVEL_ASENTAMIENTO.maxNivel2[i - 1]!);
      expect(CUPO_NIVEL_ASENTAMIENTO.maxNivel3[i]).toBeGreaterThanOrEqual(CUPO_NIVEL_ASENTAMIENTO.maxNivel3[i - 1]!);
    }
  });
});
