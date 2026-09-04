// PERFILES DE TRAZADO (doc trazado §E6.23): una política del Maestro de Obras cambia la SILUETA de la ciudad
// permutando el desempate de colocación, sin imponer ninguna plantilla — las reglas siguen siendo locales y la
// forma sigue emergiendo.
//
// Lo que congela este archivo es la propiedad que da sentido a toda la mecánica: **perfiles distintos producen
// ciudades distintas, y ninguno rompe los invariantes del trazado.** Sin lo primero la política es cosmética;
// sin lo segundo es un bug con nombre bonito.
import { describe, expect, it } from 'vitest';
import type { Asentamiento } from '../../domain/types';
import { PERFILES_TRAZADO, POLITICAS, TRAZADO, ZONA_INFLUENCIA, type PerfilTrazado } from '../../constants';
import { activarPolitica, perfilTrazadoDePolitica } from '../politicas';
import { avanzarSimulacion } from '../simulation';
import { createRng } from '../../worldgen';
import {
  celdaMinimaDeEdificio,
  celdasDeEdificio,
  edificiosInternos,
  perfilPorTradicion,
  resolverPerfil,
  sueloOcupado,
} from '../trazado';
import { contextoDeTest, crearEstadoDeTest, crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, instanteDeTest } from './fixtures';

const SEED = 42;

/** Corre una simulación real de `ticks` con `perfil` forzado y devuelve la ciudad resultante. Restaura
 * `TRAZADO.perfilForzado` pase lo que pase: es estado global mutable y dejarlo sucio contamina otros tests. */
function ciudadCon(perfil: PerfilTrazado, ticks: number): Asentamiento {
  const previo = TRAZADO.perfilForzado;
  TRAZADO.perfilForzado = perfil;
  try {
    const mapa = crearMapaDeterminista(SEED);
    const { asentamiento, facciones } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
    const preparado: Asentamiento = {
      ...asentamiento,
      nivel: 2,
      nivelActual: 2,
      radioPotencial: ZONA_INFLUENCIA.radioMaximoPorNivel[2] ?? asentamiento.radioPotencial,
      cargos: { ...asentamiento.cargos, gobernadorId: 'jugador-faccion-1-1' },
    };
    let estado = crearEstadoDeTest([preparado], facciones);
    const rng = createRng(SEED);
    for (let tick = 1; tick <= ticks; tick++) estado = avanzarSimulacion(estado, mapa, contextoDeTest(tick, rng));
    return estado.asentamientos[0]!;
  } finally {
    TRAZADO.perfilForzado = previo;
  }
}

/** Firma posicional de una ciudad: tipo + celda mínima de cada edificio interno, en orden de construcción. */
function firma(a: Asentamiento): string {
  return edificiosInternos(a.edificios)
    .map((e) => {
      const min = celdaMinimaDeEdificio(e);
      return `${e.tipo}@${min.col},${min.row}`;
    })
    .join('|');
}

describe('perfiles de trazado — la permutación del desempate cambia la ciudad', () => {
  it('cada perfil produce una ciudad DISTINTA sobre la misma semilla', () => {
    // Es la propiedad que justifica la mecánica entera. Si dos perfiles coinciden, ese perfil es decorativo:
    // pasó con "palatina" (dominaba `bordeCompartido`, que solo tiene señal a hueco 0) y por eso se descartó
    // antes de llegar al catálogo — este test es lo que impide que vuelva a colarse uno así.
    const firmas = new Map<string, PerfilTrazado[]>();
    for (const perfil of PERFILES_TRAZADO) {
      const f = firma(ciudadCon(perfil, 120));
      firmas.set(f, [...(firmas.get(f) ?? []), perfil]);
    }
    const duplicados = [...firmas.values()].filter((ps) => ps.length > 1);
    expect(duplicados, `perfiles con ciudad idéntica: ${duplicados.map((d) => d.join('=')).join(' · ')}`).toEqual([]);
    expect(firmas.size).toBe(PERFILES_TRAZADO.length);
  });

  it('ningún perfil rompe los invariantes del trazado (nada sobre calle, nada solapado)', () => {
    for (const perfil of PERFILES_TRAZADO) {
      const a = ciudadCon(perfil, 120);
      const { red } = sueloOcupado(a.id, a.edificios);
      const ocupadaPor = new Map<string, string>();

      for (const e of edificiosInternos(a.edificios)) {
        for (const c of celdasDeEdificio(e)) {
          const clave = `${c.col},${c.row}`;
          expect(red.calles.has(clave), `${perfil}: ${e.tipo} pisa la celda de calle ${clave}`).toBe(false);
          expect(ocupadaPor.has(clave), `${perfil}: ${e.tipo} se solapa con ${ocupadaPor.get(clave)} en ${clave}`).toBe(false);
          ocupadaPor.set(clave, e.tipo);
        }
      }
    }
    // Timeout explícito, mismo motivo que en `regresiones_historicas`: construye una ciudad de 120 ticks por
    // cada perfil del catálogo, y eso vive al filo del default de 5 s — medido en 5.0-5.3 s cuando la máquina
    // va cargada (y ~5.5 s desde que el rinde de trigo doblado hace ciudades más grandes), o sea que fallaba
    // por reloj y no por regresión. Es coste real del test, no lentitud a investigar.
  }, 30_000);

  it('el perfil no altera QUÉ se construye, solo DÓNDE', () => {
    // La forma no puede ser una ventaja económica encubierta: dos ciudades con el mismo material y los mismos
    // ticks deben levantar el mismo censo de edificios, esté el que esté al mando del desempate.
    const censo = (a: Asentamiento): string =>
      [...a.edificios.reduce((m, e) => m.set(e.tipo, (m.get(e.tipo) ?? 0) + 1), new Map<string, number>())]
        .sort(([x], [y]) => x.localeCompare(y))
        .map(([t, n]) => `${t}:${n}`)
        .join(',');

    const referencia = censo(ciudadCon('nucleos', 120));
    for (const perfil of PERFILES_TRAZADO) {
      expect(censo(ciudadCon(perfil, 120)), `censo de ${perfil}`).toBe(referencia);
    }
  });
});

describe('perfiles de trazado — de dónde sale el perfil de un asentamiento', () => {
  it('la tradición local es determinista y reparte los cuatro perfiles', () => {
    expect(perfilPorTradicion('asentamiento-x')).toBe(perfilPorTradicion('asentamiento-x'));
    const vistos = new Set<PerfilTrazado>();
    for (let i = 0; i < 200; i++) vistos.add(perfilPorTradicion(`asentamiento-${i}`));
    expect([...vistos].sort()).toEqual([...PERFILES_TRAZADO].sort());
  });

  it('precedencia: override del laboratorio > política > tradición', () => {
    const id = 'asentamiento-precedencia';
    const tradicion = perfilPorTradicion(id);
    const otro = PERFILES_TRAZADO.find((p) => p !== tradicion)!;
    const tercero = PERFILES_TRAZADO.find((p) => p !== tradicion && p !== otro)!;

    expect(resolverPerfil(id)).toBe(tradicion);
    expect(resolverPerfil(id, otro)).toBe(otro);

    const previo = TRAZADO.perfilForzado;
    TRAZADO.perfilForzado = tercero;
    try {
      expect(resolverPerfil(id), 'el override debe ganarle también a la tradición').toBe(tercero);
      expect(resolverPerfil(id, otro), 'el override debe ganarle también a la política').toBe(tercero);
    } finally {
      TRAZADO.perfilForzado = previo;
    }
  });

  it('una ordenanza activa del Maestro de Obras fija el perfil, y son excluyentes entre sí', () => {
    const mapa = crearMapaDeterminista(SEED);
    const { asentamiento, facciones } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
    const conCargo: Asentamiento = {
      ...asentamiento,
      cargos: { ...asentamiento.cargos, maestroObrasId: 'jugador-faccion-1-1' },
    };
    expect(perfilTrazadoDePolitica(conCargo)).toBeNull();

    const conOrdenanza = activarPolitica(conCargo, facciones[0]!, 'maestroObras', 'postura_defensiva', instanteDeTest(1));
    expect(perfilTrazadoDePolitica(conOrdenanza)).toBe('compacta');

    // Excluyentes SIN regla nueva: `maestroObras` tiene un único slot, así que la segunda ordenanza no entra
    // mientras la primera siga viva. Es lo que hace que "4 políticas que no pueden estar activas a la vez"
    // salga gratis del sistema de cargos que ya existía.
    expect(POLITICAS.slotsPorCargo.maestroObras.base).toBe(1);
    expect(() => activarPolitica(conOrdenanza, facciones[0]!, 'maestroObras', 'barrios_gremiales', instanteDeTest(2))).toThrow(
      /Sin slots libres/
    );
  });

  it('cada perfil tiene exactamente una ordenanza que lo activa', () => {
    const porPerfil = new Map<PerfilTrazado, string[]>();
    for (const perfil of PERFILES_TRAZADO) porPerfil.set(perfil, []);
    for (const def of [
      'postura_defensiva',
      'arterias_comerciales',
      'barrios_gremiales',
      'plazas_mayores',
    ] as const) {
      const falso = { politicasActivas: [{ id: 'x', politicaId: def, cargo: 'maestroObras' as const, activadaEn: instanteDeTest(0), expiraEn: instanteDeTest(9999) }] };
      const perfil = perfilTrazadoDePolitica(falso);
      expect(perfil, `la ordenanza ${def} debería fijar un perfil`).not.toBeNull();
      porPerfil.get(perfil!)!.push(def);
    }
    for (const [perfil, ordenanzas] of porPerfil) {
      expect(ordenanzas, `perfil ${perfil}`).toHaveLength(1);
    }
  });
});
