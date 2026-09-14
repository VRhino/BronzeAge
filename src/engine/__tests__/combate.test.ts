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

function escuadron(id: string, heroeId: string, cantidad: number): Escuadron {
  return { id, nombre: id, heroeId, origen: 'pesants', tropaId: 'milicia_lanceros', cantidad, veterania: 0, moral: 100 };
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

  it('conquistar: la guarnición pasa a ser la del CONQUISTADOR, sin herencia ni residencia del vencido (Doc 5.4)', () => {
    // Ocupacion §2.2: los escuadrones seleccionados del atacante MARCHAN a guarnecer la plaza tomada y salen
    // de la suya; los cascarones congelados del vencido NO se quedan (huérfanos). Antes la guarnición caía a
    // 0 y la plaza quedaba indefensa para siempre — el ping-pong de conquistas.
    const { atacante, defensor } = ciudades();
    expect(defensor.escuadrones.length, 'el defensor arranca con guarnición').toBeGreaterThan(0);

    const resultado = iniciarAsedio(atacante, defensor, ['e-atacante'], crearFacciones(), [], instanteDeTest(0), rngSinVarianza);

    expect(resultado.conquistado).toBe(true);
    expect(resultado.defensor.faccionId).toBe(atacante.faccionId);
    // La guarnición es AHORA el escuadrón del conquistador (con sus bajas de asedio), no un cascarón del vencido.
    expect(resultado.defensor.escuadrones.map((e) => e.id)).toEqual(['e-atacante']);
    expect(resultado.defensor.escuadrones[0]!.heroeId).toBe('jugador-atacante');
    expect(resultado.defensor.escuadrones[0]!.cantidad).toBeGreaterThan(0);
    // Y ha salido de la guarnición del atacante: marchó a la plaza tomada.
    expect(resultado.atacante.escuadrones).toEqual([]);
    // Residencia y cargos del vencido: vacíos.
    expect(resultado.defensor.heroesFundadoresIds).toEqual([]);
    expect(resultado.defensor.casasCompradas).toEqual([]);
    expect(Object.values(resultado.defensor.cargos).every((v) => v === null)).toBe(true);
    // Abre la ventana de ocupación.
    expect(resultado.defensor.ocupacionHasta).toBeDefined();
  });

  it('conquistar saquea: población baja, edificios dañados salvo Centro Urbano + 1 Granja/1 Leñera', () => {
    const { atacante, defensor } = ciudades();
    const pobAntes = { ...defensor.poblacion };
    const activosAntes = defensor.edificios.filter((e) => e.estado === 'activo');
    expect(activosAntes.length, 'la fixture trae edificios activos que saquear').toBeGreaterThan(2);

    const r = iniciarAsedio(atacante, defensor, ['e-atacante'], crearFacciones(), [], instanteDeTest(0), rngSinVarianza).defensor;

    expect(r.poblacion.pesants).toBeLessThan(pobAntes.pesants);
    expect(r.poblacion.nobleza, 'la nobleza no se saquea').toBe(pobAntes.nobleza);
    const centro = r.edificios.find((e) => e.tipo === 'centroUrbano')!;
    expect(centro.estado, 'el Centro Urbano nunca se daña').toBe('activo');
    expect(r.edificios.some((e) => e.danado), 'algún edificio queda dañado').toBe(true);
    for (const tipo of ['granja', 'lenera'] as const) {
      const activasDeTipo = activosAntes.filter((e) => e.tipo === tipo);
      if (activasDeTipo.length > 0) {
        expect(r.edificios.some((e) => e.tipo === tipo && e.estado === 'activo'), `queda al menos una ${tipo} activa`).toBe(true);
      }
    }
  });

  it('el saqueo es determinista: misma entrada, misma salida', () => {
    const { atacante, defensor } = ciudades();
    const a = iniciarAsedio(atacante, defensor, ['e-atacante'], crearFacciones(), [], instanteDeTest(0), rngSinVarianza).defensor;
    const b = iniciarAsedio(atacante, defensor, ['e-atacante'], crearFacciones(), [], instanteDeTest(0), rngSinVarianza).defensor;
    expect(JSON.stringify(a)).toBe(JSON.stringify(b));
  });

  it('una plaza bajo ocupación reciente es INMUNE: rebota sin combate ni conquista (Ocupacion §2.4)', () => {
    const { atacante, defensor } = ciudades();
    const ocupado: Asentamiento = { ...defensor, ocupacionHasta: instanteDeTest(100) };
    const r = iniciarAsedio(atacante, ocupado, ['e-atacante'], crearFacciones(), [], instanteDeTest(10), rngSinVarianza);
    expect(r.conquistado).toBe(false);
    expect(r.defensor.faccionId).toBe(defensor.faccionId);
    expect(r.eventos.map((e) => (typeof e !== 'string' ? e.codigo : e))).toEqual(['combate.asedio_resistido']);
  });

  it('un asedio RESISTIDO no toca ni residencia ni cargos: solo deja bajas', () => {
    const { atacante, defensor } = ciudades();
    const amurallado: Asentamiento = { ...defensor, recintos: [recintoCompleto(3, 1, 10)] };

    const resultado = iniciarAsedio(atacante, amurallado, ['e-atacante'], crearFacciones(), [], instanteDeTest(0), rngSinVarianza);

    expect(resultado.conquistado).toBe(false);
    expect(resultado.defensor.faccionId).toBe(defensor.faccionId);
    expect(resultado.defensor.heroesFundadoresIds).toEqual(defensor.heroesFundadoresIds);
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
