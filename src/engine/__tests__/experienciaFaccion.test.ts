// De dónde sale la experiencia de Facción (decisiones del usuario, 2026-09-27). Medido en la Era I: con la guerra
// dando experiencia por cualquier combate, 6 de 12 Facciones llegaban a nivel 10; al frenarla, 9 de 12 se quedaban en
// nivel 1 sin poder fundar. Ahora el combate solo cuenta si es DIGNO, crecer en paz también da, y los bandidos dan
// cada vez menos.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, CampamentoBandido, Faccion } from '../../domain/types';
import { NIVEL_FACCION } from '../../constants';
import { atacarCampamentoConColumna, esCombateDigno, poderTotal, resolverCombate } from '../combate';
import type { EjercitoConTropa } from '../tropa';
import { avanzarSimulacion } from '../simulation';
import { aplicarEntregaATrueque, proponerTrueque } from '../trade';
import { createRng } from '../../worldgen';
import {
  contextoDeTest,
  crearEstadoDeTest,
  crearFacciones,
  crearMapaDeterminista,
  escuadronDePrueba,
  fundarAsentamientoDeTest,
  instanteDeTest,
  prepararParaSubirANivel2,
} from './fixtures';

const xpDe = (facciones: readonly Faccion[], id = 'faccion-1') => facciones.find((f) => f.id === id)!.experiencia;

describe('combate digno', () => {
  it('lo es si el bando débil tiene al menos `ratioCombateDigno` del poder del fuerte', () => {
    expect(esCombateDigno(100, 100 * NIVEL_FACCION.ratioCombateDigno)).toBe(true);
    expect(esCombateDigno(100 * NIVEL_FACCION.ratioCombateDigno - 1, 100)).toBe(false);
  });

  it('el combate lo marca: parejo sí, aplastante no', () => {
    const parejo = resolverCombate([escuadronDePrueba('a', 'h1')], [escuadronDePrueba('d', 'h2')], createRng(1));
    const aplastante = resolverCombate([escuadronDePrueba('a', 'h1', 'milicia_lanceros', 1000)], [escuadronDePrueba('d', 'h2')], createRng(1));
    expect(parejo.digno).toBe(true);
    expect(aplastante.digno).toBe(false);
  });
});

describe('campamentos de bandidos', () => {
  function atacar(nivelFaccion: number, poderRelativoDelCampamento: number): number {
    const mapa = crearMapaDeterminista(7);
    const creado = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
    const facciones = creado.facciones.map((f) => (f.id === 'faccion-1' ? { ...f, nivel: nivelFaccion } : f));
    const tropa = [escuadronDePrueba('e1', 'heroe-1')];
    const campamento = {
      id: 'bandidos-1',
      posicion: creado.asentamiento.posicion,
      bosqueId: 'bosque-1',
      asentamientoId: creado.asentamiento.id,
      poder: poderTotal(tropa, false) * poderRelativoDelCampamento,
    } as CampamentoBandido;
    const columna = { id: 'col-1', faccionId: 'faccion-1', escuadrones: tropa, suministro: {} } as unknown as EjercitoConTropa;
    const r = atacarCampamentoConColumna(columna, campamento, facciones, createRng(1));
    return xpDe(r.facciones) - xpDe(facciones);
  }

  it('destruir uno da experiencia aunque no sea un combate digno, dividida por el nivel de la Facción', () => {
    expect(atacar(1, 0.1)).toBe(NIVEL_FACCION.xp.bandidos);
    expect(atacar(2, 0.1)).toBe(NIVEL_FACCION.xp.bandidos / 2);
  });

  it('desde `nivelSinXpBandidos` ya no da nada, y un ataque fallido tampoco', () => {
    expect(atacar(NIVEL_FACCION.nivelSinXpBandidos, 0.1)).toBe(0);
    expect(atacar(1, 10)).toBe(0);
  });
});

describe('crecer en paz', () => {
  it('fundar un asentamiento nuevo da experiencia; el primero de la Facción, no', () => {
    const mapa = crearMapaDeterminista(7);
    const facciones = crearFacciones().map((f) => (f.id === 'faccion-1' ? { ...f, nivel: 2 } : f));
    const primero = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);
    expect(xpDe(primero.facciones)).toBe(xpDe(facciones));
    const segundo = fundarAsentamientoDeTest(mapa, primero.facciones, 'faccion-1', [primero.asentamiento]);
    expect(xpDe(segundo.facciones) - xpDe(primero.facciones)).toBe(NIVEL_FACCION.xp.fundacion);
  });

  it('terminar la subida de un asentamiento a nivel N da `ascensoPorNivel` × N', () => {
    const mapa = crearMapaDeterminista(7);
    const creado = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
    const enObra: Asentamiento = {
      ...prepararParaSubirANivel2(creado.asentamiento, mapa),
      ascenso: { nivelObjetivo: 2, iniciadoEn: instanteDeTest(0), completaEn: instanteDeTest(1) },
    };
    const tras = avanzarSimulacion(crearEstadoDeTest([enObra], creado.facciones), mapa, contextoDeTest(1, createRng(1)));
    expect(tras.asentamientos[0]!.nivel).toBe(2);
    // Lo que se termine de construir en el mismo tick suma aparte, a 1 por edificio.
    const ganada = xpDe(tras.facciones) - xpDe(creado.facciones);
    expect(ganada).toBeGreaterThanOrEqual(NIVEL_FACCION.xp.ascensoPorNivel * 2);
    expect(ganada).toBeLessThan(NIVEL_FACCION.xp.ascensoPorNivel * 2 + 10);
  });

  it('cerrar un trueque da experiencia a los dos lados', () => {
    const mapa = crearMapaDeterminista(7);
    const uno = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
    const dos = fundarAsentamientoDeTest(mapa, uno.facciones, 'faccion-2', [uno.asentamiento]);
    const plazas = [uno.asentamiento, dos.asentamiento];
    const porId = new Map(plazas.map((a) => [a.id, a]));
    const acuerdo = proponerTrueque(plazas, uno.asentamiento.id, dos.asentamiento.id, [{ recurso: 'madera', cantidad: 10 }], [{ recurso: 'piedra', cantidad: 10 }], instanteDeTest(0), 0);

    const mitad = aplicarEntregaATrueque(acuerdo, 'A', { madera: 10 }, porId);
    expect(mitad.ajustesExperiencia).toEqual([]);
    const cerrado = aplicarEntregaATrueque(mitad.acuerdo, 'B', { piedra: 10 }, porId);
    expect(cerrado.ajustesExperiencia.map((a) => [a.faccionId, a.delta])).toEqual([
      ['faccion-1', NIVEL_FACCION.xp.truequeCumplido],
      ['faccion-2', NIVEL_FACCION.xp.truequeCumplido],
    ]);
  });

  it('un trueque entre plazas de la misma Facción no da experiencia', () => {
    const mapa = crearMapaDeterminista(7);
    const facciones = crearFacciones().map((f) => (f.id === 'faccion-1' ? { ...f, nivel: 2 } : f));
    const uno = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);
    const dos = fundarAsentamientoDeTest(mapa, uno.facciones, 'faccion-1', [uno.asentamiento]);
    const plazas = [uno.asentamiento, dos.asentamiento];
    const porId = new Map(plazas.map((a) => [a.id, a]));
    const acuerdo = proponerTrueque(plazas, uno.asentamiento.id, dos.asentamiento.id, [{ recurso: 'madera', cantidad: 10 }], [{ recurso: 'piedra', cantidad: 10 }], instanteDeTest(0), 0);
    const mitad = aplicarEntregaATrueque(acuerdo, 'A', { madera: 10 }, porId);
    expect(aplicarEntregaATrueque(mitad.acuerdo, 'B', { piedra: 10 }, porId).ajustesExperiencia).toEqual([]);
  });
});
