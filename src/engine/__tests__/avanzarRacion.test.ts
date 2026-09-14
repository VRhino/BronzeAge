// La regla del hambre extraída (Doc 5.4 / 5.13, Paso 1 del movimiento de ejércitos): `avanzarRacion` no sabe
// de asentamientos ni de ejércitos, solo recibe escuadrones y un montón de trigo. Es lo que permite que la
// guarnición y un ejército en campaña compartan curva, constantes y evento sin duplicar nada.
//
// Lo que congela este archivo es el CONTRATO de la función extraída — sobre todo las dos cosas que el
// envoltorio de guarnición no ejercita y de las que el carro de suministros va a depender: `trigoConsumido`
// como valor de retorno (el llamador decide de dónde descontarlo) y `factorConsumo` (ejército estacionado,
// Doc 5.12.3). El resto del comportamiento ya lo cubren los tests de mantenimiento que no se tocaron.
import { describe, expect, it } from 'vitest';
import type { Escuadron } from '../../domain/types';
import { MILITAR } from '../../constants';
import { avanzarRacion, consumoRacionDeEscuadrones } from '../tropas';
import { escuadronDePrueba } from './fixtures';

const escuadron = (cantidad: number, moral: number): Escuadron => escuadronDePrueba('e1', 'jugador-1', 'milicia_lanceros', cantidad, { moral, nombre: 'Milicia de prueba' });

describe('avanzarRacion — el contrato de la despensa', () => {
  it('con trigo de sobra sube la moral y no consume más de lo que necesita', () => {
    const necesita = consumoRacionDeEscuadrones([escuadron(20, 50)]);
    const r = avanzarRacion([escuadron(20, 50)], necesita * 10);

    expect(r.trigoConsumido).toBeCloseTo(necesita);
    expect(r.escuadrones[0]!.moral).toBe(50 + MILITAR.regeneracionMoralPorMinuto);
    expect(r.escuadrones[0]!.cantidad).toBe(20);
    expect(r.eventos).toHaveLength(0);
  });

  it('sin nada de trigo la moral cae el máximo y no se consume nada', () => {
    const r = avanzarRacion([escuadron(20, 50)], 0);

    expect(r.trigoConsumido).toBe(0);
    expect(r.escuadrones[0]!.moral).toBe(50 - MILITAR.degradacionMoralSinRacion);
  });

  it('nunca consume más trigo del disponible, aunque la ración sea mayor', () => {
    const necesita = consumoRacionDeEscuadrones([escuadron(20, 100)]);
    const r = avanzarRacion([escuadron(20, 100)], necesita / 4);

    expect(r.trigoConsumido).toBeCloseTo(necesita / 4);
    expect(r.trigoConsumido).toBeLessThan(necesita);
  });

  it('a moral 0 hay deserción permanente y la narra', () => {
    const r = avanzarRacion([escuadron(20, 0)], 0);

    expect(r.escuadrones[0]!.cantidad).toBeLessThan(20);
    expect(r.eventos).toHaveLength(1);
    expect(r.eventos[0]).toMatchObject({ codigo: 'tropas.desercion' });
  });

  it('un escuadrón aniquilado conserva su identidad y no genera evento (Doc 5.4)', () => {
    const r = avanzarRacion([escuadron(0, 0)], 0);

    expect(r.escuadrones[0]!.cantidad).toBe(0);
    expect(r.escuadrones[0]!.nombre).toBe('Milicia de prueba');
    expect(r.eventos).toHaveLength(0);
  });

  it('factorConsumo reduce la ración proporcionalmente — ejército estacionado (Doc 5.12.3)', () => {
    const tropas = [escuadron(20, 100)];
    const completa = consumoRacionDeEscuadrones(tropas);
    const reducida = consumoRacionDeEscuadrones(tropas, 0.5);

    expect(reducida).toBeCloseTo(completa / 2);

    // Con la mitad de la ración justa, estacionado se alimenta entero y marchando pasa hambre: es la misma
    // despensa dando distinto resultado según el factor, que es justo para lo que existe el parámetro.
    const estacionado = avanzarRacion(tropas, completa / 2, 0.5);
    const marchando = avanzarRacion(tropas, completa / 2, 1);

    expect(estacionado.escuadrones[0]!.moral).toBe(100);
    expect(marchando.escuadrones[0]!.moral).toBeLessThan(100);
  });

  it('sin escuadrones no consume nada ni revienta', () => {
    const r = avanzarRacion([], 500);

    expect(r.trigoConsumido).toBe(0);
    expect(r.escuadrones).toEqual([]);
    expect(r.eventos).toHaveLength(0);
  });
});
