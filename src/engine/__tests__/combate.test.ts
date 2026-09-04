// COMBATE — cobertura mínima que faltaba (no había ningún test directo de `resolverCombate`/`iniciarAsedio`
// antes de esto) más el multiplicador defensivo de murallas (`Consideraciones/Murallas_Definicion.md`, Paso
// 3b, §16): la RAZÓN DE SER de toda la mecánica, así que el punto de esta suite es demostrar que de verdad
// cambia quién gana un asedio, no solo que el número sale bien.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, CeldaMuro, Escuadron, Recinto } from '../../domain/types';
import { MURALLA } from '../../constants';
import type { RandomFn } from '../../worldgen';
import { iniciarAsedio, resolverCombate } from '../combate';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, instanteDeTest } from './fixtures';

/** RNG SIN varianza: `rng() = 0.5` deja `jitter = 1 + (0.5×2−1)×varianza = 1` exacto en los dos bandos, así
 * que el resultado depende SOLO del poder calculado, no de una tirada — necesario para que estos tests sean
 * deterministas sobre la comparación que quieren probar (el efecto del muro), no sobre el azar del combate. */
const rngSinVarianza: RandomFn = Object.assign(() => 0.5, { estado: () => 0 });

function escuadron(id: string, jugadorId: string, cantidad: number): Escuadron {
  return { id, nombre: id, jugadorId, origen: 'pesants', tropaId: 'milicia_lanceros', cantidad, veterania: 0, moral: 100 };
}

// `milicia_lanceros`: poderBase 2. Con un solo escuadrón por bando no entra la cohesión (exige length > 1),
// así que el poder es sencillamente `2 × cantidad`.
const ATACANTE = [escuadron('e-atacante', 'jugador-atacante', 100)]; // poder 200
const DEFENSOR = [escuadron('e-defensor', 'jugador-defensor', 90)]; // poder 180 — MENOS que el atacante

describe('resolverCombate — el multiplicador defensivo (Paso 3b)', () => {
  it('sin multiplicador (o en 1), gana quien tiene más poder — aquí, el atacante', () => {
    const resultado = resolverCombate(ATACANTE, DEFENSOR, instanteDeTest(0), rngSinVarianza);
    expect(resultado.ganador).toBe('atacante');
  });

  it('con el multiplicador de una muralla de nivel 3 y una puerta (×2.5), el mismo defensor RESISTE', () => {
    const resultado = resolverCombate(ATACANTE, DEFENSOR, instanteDeTest(0), rngSinVarianza, MURALLA.bonoDefensaPorNivel[3]);
    expect(resultado.ganador).toBe('defensor');
  });

  it('un multiplicador de 1 (sin muro) es indistinguible de no pasar el parámetro', () => {
    const conUno = resolverCombate(ATACANTE, DEFENSOR, instanteDeTest(0), rngSinVarianza, 1);
    const sinParametro = resolverCombate(ATACANTE, DEFENSOR, instanteDeTest(0), rngSinVarianza);
    expect(conUno.ganador).toBe(sinParametro.ganador);
  });
});

describe('iniciarAsedio — la muralla del DEFENSOR decide, no la del atacante', () => {
  function ciudades(): { atacante: Asentamiento; defensor: Asentamiento } {
    const mapa = crearMapaDeterminista(1);
    const facciones = crearFacciones();
    const { asentamiento: base1 } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', [], 0, { x: 0, y: 0 });
    const { asentamiento: base2 } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-2', [base1], 0, { x: 2000, y: 2000 });
    const atacante: Asentamiento = { ...base2, cargos: { ...base2.cargos, generalId: 'jugador-atacante' }, escuadrones: ATACANTE };
    const defensor: Asentamiento = { ...base1, escuadrones: DEFENSOR };
    return { atacante, defensor };
  }

  function recintoCompleto(nivel: number, puertas: number, totalCeldas: number): Recinto {
    const celdas: CeldaMuro[] = Array.from({ length: totalCeldas }, (_, i) => ({
      col: i,
      row: 0,
      clase: i < puertas ? ('puerta' as const) : ('muro' as const),
    }));
    return { id: 'r1', nivel, celdas, avance: totalCeldas - 1, comprometidoEn: instanteDeTest(0), completadoEn: instanteDeTest(0) };
  }

  it('sin recinto, el defensor cae exactamente como hoy (sin cambio de comportamiento)', () => {
    const { atacante, defensor } = ciudades();
    const resultado = iniciarAsedio(atacante, defensor, ['e-atacante'], crearFacciones(), [], instanteDeTest(0), rngSinVarianza);
    expect(resultado.conquistado).toBe(true);
  });

  it('conquistar NO hereda la guarnición ni la residencia del vencido (Doc 5.4)', () => {
    // Regresión: hasta 2026-09-04 el conquistador se quedaba con los escuadrones del vencido —los mismos que
    // el doc llama "personales de otro jugador, no botín transferible"— y sus antiguos residentes seguían
    // figurando como tales en una ciudad ahora enemiga. No lo cubría ningún test.
    const { atacante, defensor } = ciudades();
    expect(defensor.escuadrones.length, 'el defensor arranca con guarnición').toBeGreaterThan(0);

    const resultado = iniciarAsedio(atacante, defensor, ['e-atacante'], crearFacciones(), [], instanteDeTest(0), rngSinVarianza);

    expect(resultado.conquistado).toBe(true);
    expect(resultado.defensor.faccionId).toBe(atacante.faccionId);
    // Cascarones a cero: sin hombres, pero el escuadrón y su progreso siguen siendo de su dueño.
    expect(resultado.defensor.escuadrones.map((e) => e.cantidad)).toEqual([0]);
    expect(resultado.defensor.escuadrones[0]!.jugadorId).toBe(defensor.escuadrones[0]!.jugadorId);
    expect(resultado.defensor.escuadrones[0]!.veterania).toBe(defensor.escuadrones[0]!.veterania);
    expect(resultado.defensor.jugadoresFundadoresIds).toEqual([]);
    expect(resultado.defensor.casasCompradas).toEqual([]);
    expect(Object.values(resultado.defensor.cargos).every((v) => v === null)).toBe(true);
  });

  it('un asedio RESISTIDO no toca ni residencia ni cargos: solo deja bajas', () => {
    const { atacante, defensor } = ciudades();
    const amurallado: Asentamiento = { ...defensor, recintos: [recintoCompleto(3, 1, 10)] };

    const resultado = iniciarAsedio(atacante, amurallado, ['e-atacante'], crearFacciones(), [], instanteDeTest(0), rngSinVarianza);

    expect(resultado.conquistado).toBe(false);
    expect(resultado.defensor.faccionId).toBe(defensor.faccionId);
    expect(resultado.defensor.jugadoresFundadoresIds).toEqual(defensor.jugadoresFundadoresIds);
    expect(resultado.defensor.escuadrones.length).toBe(defensor.escuadrones.length);
  });

  it('con un recinto nivel 3 de una puerta, completo, el defensor resiste el MISMO ataque', () => {
    const { atacante, defensor } = ciudades();
    const amurallado: Asentamiento = { ...defensor, recintos: [recintoCompleto(3, 1, 10)] };
    const resultado = iniciarAsedio(atacante, amurallado, ['e-atacante'], crearFacciones(), [], instanteDeTest(0), rngSinVarianza);
    expect(resultado.conquistado).toBe(false);
  });

  it('la muralla del ATACANTE no cuenta para nada: solo importa la del defensor', () => {
    const { atacante, defensor } = ciudades();
    const atacanteAmurallado: Asentamiento = { ...atacante, recintos: [recintoCompleto(3, 1, 10)] };
    const resultado = iniciarAsedio(atacanteAmurallado, defensor, ['e-atacante'], crearFacciones(), [], instanteDeTest(0), rngSinVarianza);
    expect(resultado.conquistado).toBe(true); // igual que sin ningún recinto de por medio
  });

  it('un recinto recién comprometido (integridad 0, nada levantado) no defiende todavía', () => {
    const { atacante, defensor } = ciudades();
    const sinLevantar: Recinto = { ...recintoCompleto(3, 1, 10), avance: -1 }; // integridad 0
    const amurallado: Asentamiento = { ...defensor, recintos: [sinLevantar] };
    const resultado = iniciarAsedio(atacante, amurallado, ['e-atacante'], crearFacciones(), [], instanteDeTest(0), rngSinVarianza);
    expect(resultado.conquistado).toBe(true); // igual que sin ningún recinto: no hay nada en pie que atravesar
  });
});
