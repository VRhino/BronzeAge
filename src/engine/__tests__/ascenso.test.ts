// Subida de nivel manual y con coste (engine/ascenso.ts, Doc 4.5). Cada caso fija una regla: los gates son
// requisito y no subida, el coste se cobra al empezar, la solvencia se mira recurso a recurso, el cupo se reserva
// al pedir, la obra termina sola en el tick y la conquista la borra.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, Edificio, EdificioTipo } from '../../domain/types';
import { ASCENSO_ASENTAMIENTO } from '../../constants';
import { minutos, sumar } from '../../domain/tiempo';
import type { Mapa } from '../../world/mapa';
import { avanzarAscenso, cupoLibreParaNivel, evaluarAscenso, iniciarAscenso, AscensoInvalidoError } from '../ascenso';
import { aplicarConquista } from '../combate';
import type { PayloadNivelSubio } from '../mantenimiento';
import { avanzarSimulacion } from '../simulation';
import { createRng } from '../../worldgen';
import {
  contextoDeTest,
  crearEstadoDeTest,
  crearFacciones,
  crearMapaDeterminista,
  fundarAsentamientoDeTest,
  instanteDeTest,
  prepararParaSubirANivel2,
} from './fixtures';

const AHORA = instanteDeTest(100);
const TARIFA_2 = ASCENSO_ASENTAMIENTO.porNivelObjetivo[2]!;

function activo(id: string, tipo: EdificioTipo): Edificio {
  return { id, tipo, posicion: { x: 0, y: 0 }, estado: 'activo', ambito: 'mapa' };
}

function listoParaSubir(mapa: Mapa): Asentamiento {
  return prepararParaSubirANivel2(fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []).asentamiento, mapa);
}

describe('evaluarAscenso — cuándo se puede pedir la subida', () => {
  it('un asentamiento que cumple gates, paga, es solvente y tiene cupo, puede', () => {
    const mapa = crearMapaDeterminista(7);
    const a = listoParaSubir(mapa);
    const e = evaluarAscenso(a, [a], crearFacciones(), mapa, AHORA);
    expect(e.bloqueos).toEqual([]);
    expect(e.puede).toBe(true);
    expect(e.nivelObjetivo).toBe(2);
    expect(e.costo).toEqual(TARIFA_2.costo);
  });

  it('cumplir los gates ya no basta: sin cantera es insolvente en piedra, que el nivel 2 cobra', () => {
    const mapa = crearMapaDeterminista(7);
    const a = listoParaSubir(mapa);
    const sinExtraccionDeCobro = { ...a, edificios: a.edificios.filter((e) => e.tipo !== 'cantera' && e.tipo !== 'mina') };
    // Mantiene 3 tipos para el gate (leñera, granja y corral de relleno) — lo único que falla es la solvencia.
    const conGate = { ...sinExtraccionDeCobro, edificios: [...sinExtraccionDeCobro.edificios, activo('g', 'granja'), activo('k', 'corral')] };
    const e = evaluarAscenso(conGate, [conGate], crearFacciones(), mapa, AHORA);
    expect(e.bloqueos).toEqual(['insolvente']);
    const piedra = e.solvencia.find((s) => s.recurso === 'piedra')!;
    expect(piedra.ingresoPorMinuto).toBe(0);
    expect(piedra.costoPorMinuto).toBeGreaterThan(0);
  });

  it('separa qué mitad del gate falta', () => {
    const mapa = crearMapaDeterminista(7);
    const a = listoParaSubir(mapa);
    const pocaGente = { ...a, poblacion: { pesants: 50, artesanos: 0, nobleza: 0 } };
    expect(evaluarAscenso(pocaGente, [pocaGente], crearFacciones(), mapa, AHORA).bloqueos).toContain('falta_poblacion');
    const sinEdificios = { ...a, edificios: a.edificios.filter((e) => e.tipo === 'centroUrbano') };
    expect(evaluarAscenso(sinEdificios, [sinEdificios], crearFacciones(), mapa, AHORA).bloqueos).toContain('faltan_edificios');
  });

  it('sin el coste en el almacén no se puede', () => {
    const mapa = crearMapaDeterminista(7);
    const a = listoParaSubir(mapa);
    const pobre = { ...a, almacen: { ...a.almacen, oro: { cantidad: 0, capacidad: 10000 } } };
    expect(evaluarAscenso(pobre, [pobre], crearFacciones(), mapa, AHORA).bloqueos).toContain('recursos_insuficientes');
  });

  it('en el nivel máximo no hay a dónde subir', () => {
    const mapa = crearMapaDeterminista(7);
    const a = { ...listoParaSubir(mapa), nivel: 5, nivelActual: 5 };
    const e = evaluarAscenso(a, [a], crearFacciones(), mapa, AHORA);
    expect(e.nivelObjetivo).toBeNull();
    expect(e.bloqueos).toEqual(['nivel_maximo']);
  });
});

describe('el tick ya no sube el nivel solo', () => {
  it('cumpliendo de sobra los gates y el coste, sin obra pedida sigue en nivel 1', () => {
    const mapa = crearMapaDeterminista(7);
    const a = listoParaSubir(mapa);
    const tras = avanzarSimulacion(crearEstadoDeTest([a], crearFacciones()), mapa, contextoDeTest(100, createRng(1)));
    expect(tras.asentamientos[0]!.nivel).toBe(1);
  });
});

describe('cupo de Facción — se reserva al pedir', () => {
  it('una obra en curso hacia el nivel 2 ocupa la única plaza de una Facción de nivel 1', () => {
    const mapa = crearMapaDeterminista(7);
    const facciones = crearFacciones();
    const a = listoParaSubir(mapa);
    const otro = { ...a, id: 'otro', ascenso: { nivelObjetivo: 2, iniciadoEn: AHORA, completaEn: AHORA } };
    expect(cupoLibreParaNivel(facciones[0], [a], 2)).toBe(true);
    expect(cupoLibreParaNivel(facciones[0], [a, otro], 2)).toBe(false);
    expect(evaluarAscenso(a, [a, otro], facciones, mapa, AHORA).bloqueos).toContain('sin_cupo_de_faccion');
  });

  it('por ahora nadie pasa del nivel 3: desde ahí no hay a dónde subir (techo provisional)', () => {
    const mapa = crearMapaDeterminista(7);
    const a = { ...listoParaSubir(mapa), nivel: 3, nivelActual: 3 };
    expect(evaluarAscenso(a, [a], crearFacciones(), mapa, AHORA).bloqueos).toEqual(['nivel_maximo']);
  });

  it('los niveles 4 y 5 no tienen cupo', () => {
    expect(cupoLibreParaNivel(crearFacciones()[0], [], 4)).toBe(true);
  });
});

describe('iniciarAscenso y avanzarAscenso — la obra', () => {
  it('cobra el coste entero al empezar y fija cuándo termina', () => {
    const mapa = crearMapaDeterminista(7);
    const a = listoParaSubir(mapa);
    const { asentamiento, eventos } = iniciarAscenso(a, [a], crearFacciones(), mapa, AHORA);
    expect(asentamiento.almacen['madera']!.cantidad).toBe(5000 - TARIFA_2.costo['madera']!);
    expect(asentamiento.almacen['oro']!.cantidad).toBe(5000 - TARIFA_2.costo['oro']!);
    expect(asentamiento.ascenso).toEqual({ nivelObjetivo: 2, iniciadoEn: AHORA, completaEn: sumar(AHORA, minutos(TARIFA_2.obraMinutos)) });
    expect(asentamiento.nivel).toBe(1);
    expect(eventos[0]).toMatchObject({ codigo: 'asentamiento.ascenso_iniciado' });
  });

  it('rechaza con todos los motivos si no se puede', () => {
    const mapa = crearMapaDeterminista(7);
    const a = { ...listoParaSubir(mapa), poblacion: { pesants: 0, artesanos: 0, nobleza: 0 } };
    expect(() => iniciarAscenso(a, [a], crearFacciones(), mapa, AHORA)).toThrow(AscensoInvalidoError);
  });

  it('no sube antes de tiempo; al terminar sube nivel y nivelActual y emite asentamiento.nivel_subio', () => {
    const mapa = crearMapaDeterminista(7);
    const a = listoParaSubir(mapa);
    const enObra = iniciarAscenso(a, [a], crearFacciones(), mapa, AHORA).asentamiento;
    const fin = enObra.ascenso!.completaEn;

    expect(avanzarAscenso(enObra, sumar(fin, minutos(-1))).asentamiento.nivel).toBe(1);

    const { asentamiento, eventos } = avanzarAscenso(enObra, fin);
    expect(asentamiento.nivel).toBe(2);
    expect(asentamiento.nivelActual).toBe(2);
    expect(asentamiento.ascenso).toBeUndefined();
    const p = (eventos[0] as { payload: PayloadNivelSubio }).payload;
    expect(p).toEqual({ asentamientoId: a.id, nivelNuevo: 2 });
  });

  it('una conquista a mitad de obra la borra', () => {
    const mapa = crearMapaDeterminista(7);
    const a = listoParaSubir(mapa);
    const enObra = iniciarAscenso(a, [a], crearFacciones(), mapa, AHORA).asentamiento;
    expect(aplicarConquista(enObra, 'faccion-2', AHORA).ascenso).toBeUndefined();
  });
});
