// Vista de Asentamiento (a petición del usuario): dos espacios lógicos separados. Casi todo edificio se
// construye DENTRO del espacio plano del asentamiento (coords locales, origen en el Centro Urbano); solo los
// extractores minerales (mina/minaCobre/minaEstano/cantera) viven en el MAPA GENERAL, sobre su nodo. Granja/
// Leñera/Corral son internos aunque su producción dependa de rasgos de la zona en el mapa general.
import { describe, expect, it } from 'vitest';
import type { Asentamiento } from '../../domain/types';
import { REJILLA_ASENTAMIENTO, TRAZADO, ZONA_INFLUENCIA } from '../../constants';
import { avanzarSimulacion, type EstadoSimulacion } from '../simulation';
import { esDeAfueras } from '../trazado';
import { computeTodasLasZonas, mejorFertilidadEnZona } from '../zones';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, mockMathRandomDeterminista } from './fixtures';

const SEED = 42;
const EXTERNOS = new Set(['mina', 'minaCobre', 'minaEstano', 'cantera']);
const modulo = (p: { x: number; y: number }) => Math.hypot(p.x, p.y);

function estadoInicial(asentamiento: Asentamiento, facciones: ReturnType<typeof crearFacciones>): EstadoSimulacion {
  return {
    asentamientos: [asentamiento],
    facciones,
    caravanas: [],
    acuerdos: [],
    ordenes: [],
    relaciones: [],
    titulos: [],
    caminos: [],
    campamentosBandidos: [],
    bandidosProximoSpawnTick: 0,
  };
}

describe('Vista de Asentamiento — fundación en espacio local', () => {
  it('todo edificio inicial es interno, con el Centro Urbano en el origen (0,0) y dentro del radio inicial', () => {
    const { asentamiento } = fundarAsentamientoDeTest(crearMapaDeterminista(SEED), crearFacciones(), 'faccion-1', []);
    const centro = asentamiento.edificios.find((e) => e.tipo === 'centroUrbano')!;

    expect(centro.ambito).toBe('asentamiento');
    expect(centro.posicion).toEqual({ x: 0, y: 0 });

    for (const e of asentamiento.edificios) {
      expect(e.ambito).toBe('asentamiento');
      // Coords LOCALES: nada que ver con la posición del asentamiento en el mapa general (~500,500).
      //
      // Granja y Corral se miden contra otro techo: viven A LAS AFUERAS, fuera del radio vedado de
      // `TRAZADO.radioAfuerasMin`, que es mayor que el radio inicial de la zona de influencia. El campo de una
      // ciudad está fuera de su zona de influencia, no dentro (ver `radioMaximoAfueras`, engine/trazado.ts).
      const techo = esDeAfueras(e.tipo) ? TRAZADO.radioAfuerasMin + TRAZADO.anchoBandaAfueras : ZONA_INFLUENCIA.radioInicial;
      expect(modulo(e.posicion)).toBeLessThanOrEqual(techo + 1e-6);
      if (esDeAfueras(e.tipo)) expect(modulo(e.posicion)).toBeGreaterThanOrEqual(TRAZADO.radioAfuerasMin);
    }
  });
});

describe('Vista de Asentamiento — colocación tras simulación', () => {
  it('los edificios internos quedan dentro del disco local; las minas/cantera van al mapa general sobre su nodo', () => {
    const mapa = crearMapaDeterminista(SEED);
    const facciones = crearFacciones();
    const restaurar = mockMathRandomDeterminista(SEED);
    try {
      const { asentamiento } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);
      let estado = estadoInicial(asentamiento, facciones);
      for (let tick = 1; tick <= 90; tick++) estado = avanzarSimulacion(estado, mapa, tick);

      const a = estado.asentamientos[0]!;
      // Debe haber crecido más allá de los edificios iniciales (prueba que la auto-construcción coloca en local).
      expect(a.edificios.length).toBeGreaterThan(5);

      for (const e of a.edificios) {
        if (EXTERNOS.has(e.tipo)) {
          // Externo: en el mapa general, plantado exactamente sobre su nodo de recurso.
          expect(e.ambito).toBe('mapa');
          const nodo = mapa.nodo(e.fuenteId);
          expect(nodo).toBeDefined();
          expect(e.posicion).toEqual(nodo!.posicion);
        } else {
          // Interno: coords locales dentro del disco local. Granja y Corral se salen a propósito del
          // `radioPotencial` —están a las afueras, fuera del radio vedado— así que su techo es el del lienzo.
          expect(e.ambito ?? 'asentamiento').toBe('asentamiento');
          const techo = esDeAfueras(e.tipo) ? REJILLA_ASENTAMIENTO.radioMapa : a.radioPotencial;
          expect(modulo(e.posicion)).toBeLessThanOrEqual(techo + 1e-6);
          if (esDeAfueras(e.tipo)) expect(modulo(e.posicion)).toBeGreaterThanOrEqual(TRAZADO.radioAfuerasMin);
        }
      }
    } finally {
      restaurar();
    }
  });
});

describe('Vista de Asentamiento — fertilidad de zona para las Granjas', () => {
  it('mejorFertilidadEnZona devuelve la MEJOR fertilidad de la zona (única, compartida por todas las Granjas)', () => {
    const mapa = crearMapaDeterminista(SEED);
    const { asentamiento } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
    const zona = computeTodasLasZonas([asentamiento]).find((z) => z.asentamientoId === asentamiento.id)!.poligono;

    const fertilidad = mejorFertilidadEnZona(asentamiento, zona, mapa);
    expect(fertilidad).toBeGreaterThan(0);
    // No debe ser menor que la fertilidad del propio centro (que siempre es candidato).
    expect(fertilidad).toBeGreaterThanOrEqual(mapa.fertilidadEn(asentamiento.posicion) - 1e-9);
  });
});
