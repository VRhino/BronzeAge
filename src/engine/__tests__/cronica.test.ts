// Crónica de los Aedas (Doc 6.7): qué hechos se cantan, con qué datos públicos, y que el descubrimiento espere al retraso.
import { describe, expect, it } from 'vitest';
import { AEDAS } from '../../constants';
import type { EventoDominio } from '../../domain/eventos';
import type { Asentamiento, EstadoTecnologia } from '../../domain/types';
import { CODIGO_CRONICA, entradasDeCronica, esDeCronica } from '../cronica';
import { avanzarTecnologia, estadoTecnologiaInicial } from '../tecnologia';
import { computeTodasLasZonas } from '../zones';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, instanteDeTest } from './fixtures';

const mapa = crearMapaDeterminista(7);
const f1 = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
const f2 = fundarAsentamientoDeTest(mapa, f1.facciones, 'faccion-2', [f1.asentamiento]);
const facciones = f2.facciones;
const plazas: Asentamiento[] = [{ ...f1.asentamiento, nombre: 'Pilos' }, { ...f2.asentamiento, nombre: 'Tebas' }];
const mundo = { facciones, asentamientos: plazas };
const evento = (codigo: string, payload: unknown, mensaje = 'x', asentamientoId?: string): EventoDominio => ({ codigo, mensaje, momento: '2026-01-01T00:00:00.000Z', payload, asentamientoId });
const nombre = (id: string) => facciones.find((f) => f.id === id)!.nombre;

describe('qué va en la crónica', () => {
  it('los públicos de la lista sí; con asentamiento, o fuera de la lista, no', () => {
    expect(esDeCronica({ codigo: 'tecnologia.logro' })).toBe(true);
    expect(esDeCronica({ codigo: 'titulo.cambia_manos' })).toBe(true);
    expect(esDeCronica({ codigo: CODIGO_CRONICA })).toBe(true);
    expect(esDeCronica({ codigo: 'tecnologia.logro', asentamientoId: 'p' })).toBe(false);
    expect(esDeCronica({ codigo: 'tecnologia.aparece' })).toBe(false);
  });
});

describe('entradas derivadas', () => {
  it('una conquista canta la plaza y las dos Facciones, sin más datos que esos', () => {
    const e = evento('combate.asedio_conquista', { atacanteId: 'a', defensorId: plazas[1]!.id, faccionAtacanteId: 'faccion-1', faccionDefensoraId: 'faccion-2', enCombate: true, murallaCompleta: false, conResidentes: false }, 'privado', plazas[1]!.id);
    const [c, ...resto] = entradasDeCronica([e], mundo);
    expect(resto).toEqual([]);
    expect(c).toMatchObject({ codigo: CODIGO_CRONICA, momento: e.momento, payload: { tipo: 'combate.asedio_conquista', faccionIds: ['faccion-1', 'faccion-2'] } });
    expect(c!.asentamientoId).toBeUndefined();
    expect(c!.mensaje).toContain('Tebas');
    expect(c!.mensaje).toContain(nombre('faccion-1'));
    expect(c!.mensaje).not.toContain('privado');
  });

  it('las ruinas se cantan con el nombre y el dueño del payload, aunque la plaza ya no esté en el estado', () => {
    const [c] = entradasDeCronica([evento('asentamiento.ruinas', { faccionId: 'faccion-2', nombre: 'Hattusa' }, 'p', 'gone')], mundo);
    expect(c!.mensaje).toContain('Hattusa');
    expect(c!.mensaje).toContain(nombre('faccion-2'));
  });

  it('fundación, guerra, rebelión, anexión y fusión', () => {
    const e = [
      evento('fundacion.asentamiento_fundado', { asentamientoId: plazas[0]!.id, faccionId: 'faccion-1' }),
      evento('diplomacia.guerra_declarada', { faccionAId: 'faccion-1', faccionesEnemigasIds: ['faccion-2'] }),
      evento('diplomacia.rebelion_vasallo', { faccionSenoraId: 'faccion-1', faccionVasallaId: 'faccion-2' }),
      evento('diplomacia.anexion', { faccionAbsorbenteId: 'faccion-1', faccionAbsorbidaId: 'faccion-2' }, 'A anexiona a B.'),
      evento('diplomacia.fusion', { faccionAId: 'faccion-1', faccionBId: 'faccion-2', faccionNuevaId: 'n' }, 'A y B se fusionan.'),
    ];
    const textos = entradasDeCronica(e, mundo).map((c) => c.mensaje);
    expect(textos).toHaveLength(5);
    expect(textos[0]).toContain('Pilos');
    expect(textos[1]).toContain('declara la guerra');
    expect(textos[3]).toContain('A anexiona a B.');
  });

  it('solo se canta la primera plaza del mundo en llegar a un nivel de ciudad grande', () => {
    const sube = (nivel: number, otras: number) => entradasDeCronica([evento('asentamiento.nivel_subio', { asentamientoId: plazas[0]!.id, nivelNuevo: nivel })], {
      facciones,
      asentamientos: [{ ...plazas[0]!, nivel }, { ...plazas[1]!, nivel: otras }],
    });
    expect(sube(3, 2)).toHaveLength(1);
    expect(sube(3, 3)).toHaveLength(0);
    expect(sube(2, 1)).toHaveLength(0);
    // Dos que suben a la vez: la primera se canta una sola vez.
    const a = evento('asentamiento.nivel_subio', { asentamientoId: plazas[0]!.id, nivelNuevo: 3 });
    const b = evento('asentamiento.nivel_subio', { asentamientoId: plazas[1]!.id, nivelNuevo: 3 });
    expect(entradasDeCronica([a, b], { facciones, asentamientos: plazas.map((p) => ({ ...p, nivel: 3 })) })).toHaveLength(1);
  });

  it('es determinista: el mismo evento produce siempre la misma frase', () => {
    const e = evento('combate.asedio_conquista', { defensorId: plazas[1]!.id, faccionAtacanteId: 'faccion-1', faccionDefensoraId: 'faccion-2' });
    expect(entradasDeCronica([e], mundo)).toEqual(entradasDeCronica([e], mundo));
  });
});

describe('descubrimientos cantados', () => {
  const asentamientos = plazas;
  const ctx = (tick: number) => ({ asentamientos, facciones, zonas: computeTodasLasZonas(asentamientos), mapa, instante: instanteDeTest(tick) });
  const desbloqueada = (): EstadoTecnologia => ({
    ...estadoTecnologiaInicial(instanteDeTest(0)),
    logros: { metalurgia_cobre: instanteDeTest(0) },
    primeros: { metalurgia_cobre: { faccionId: 'faccion-1', en: instanteDeTest(0) } },
  });
  const cantos = (r: ReturnType<typeof avanzarTecnologia>) => r.eventos.filter((e) => typeof e !== 'string' && e.codigo === 'aedas.canta_descubrimiento');

  it('se canta una vez, cuando pasa el retraso, nombrando a la descubridora', () => {
    expect(cantos(avanzarTecnologia(desbloqueada(), ctx(AEDAS.retrasoConocimientoMinutos - 1)))).toHaveLength(0);
    const r = avanzarTecnologia(desbloqueada(), ctx(AEDAS.retrasoConocimientoMinutos));
    expect(cantos(r)).toHaveLength(1);
    expect(cantos(r)[0]).toMatchObject({ payload: { tecnologiaId: 'metalurgia_cobre', faccionId: 'faccion-1' } });
    expect(r.tecnologia.cantadas).toEqual(['metalurgia_cobre']);
    expect(cantos(avanzarTecnologia(r.tecnologia, ctx(AEDAS.retrasoConocimientoMinutos + 1)))).toHaveLength(0);
  });
});
