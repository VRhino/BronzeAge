// Vista de Asentamiento (a petición del usuario): dos espacios lógicos separados. Casi todo edificio se
// construye DENTRO del espacio plano del asentamiento (coords locales, origen en el Centro Urbano); solo los
// extractores minerales (mina/minaCobre/minaEstano/cantera) viven en el MAPA GENERAL, sobre su nodo. Granja/
// Leñera/Corral son internos aunque su producción dependa de rasgos de la zona en el mapa general.
import { describe, expect, it } from 'vitest';
import type { Asentamiento } from '../../domain/types';
import { REJILLA_ASENTAMIENTO, TRAZADO, ZONA_INFLUENCIA } from '../../constants';
import { avanzarSimulacion, type EstadoSimulacion } from '../simulation';
import { createRng } from '../../worldgen';
import { celdaMinimaDeEdificio, esDeAfueras, tamanoDeEdificio } from '../trazado';
import { computeTodasLasZonas, mejorFertilidadEnZona } from '../zones';
import { contextoDeTest, crearEstadoDeTest, crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest } from './fixtures';

const SEED = 42;
const EXTERNOS = new Set(['mina', 'minaCobre', 'minaEstano', 'cantera']);
const modulo = (p: { x: number; y: number }) => Math.hypot(p.x, p.y);

/**
 * Distancia del origen al punto MÁS CERCANO de la huella de un edificio (0 si el origen cae dentro).
 *
 * Es la métrica con la que se mide el radio vedado de las afueras desde 2026-08-31 (a petición del usuario:
 * "las granjas deben crecer hacia afuera de la ciudad, no hacia adentro"). Antes se medía el CENTRO, y una
 * Granja de nivel 4 —36 unidades de lado— cuyo centro cumpliera el radio metía medio edificio dentro del casco
 * urbano: medido, su borde interior llegaba a 39 con el veto en 60.
 */
function esquinaMasLejana(e: Parameters<typeof celdaMinimaDeEdificio>[0]): number {
  const T = REJILLA_ASENTAMIENTO.tamanoCelda;
  const min = celdaMinimaDeEdificio(e);
  const tam = tamanoDeEdificio(e);
  const dx = Math.max(Math.abs(min.col * T), Math.abs((min.col + tam.ancho) * T));
  const dy = Math.max(Math.abs(min.row * T), Math.abs((min.row + tam.alto) * T));
  return Math.hypot(dx, dy);
}

function bordeInterior(e: Parameters<typeof celdaMinimaDeEdificio>[0]): number {
  const T = REJILLA_ASENTAMIENTO.tamanoCelda;
  const min = celdaMinimaDeEdificio(e);
  const tam = tamanoDeEdificio(e);
  const dx = Math.max(min.col * T, 0, -((min.col + tam.ancho) * T));
  const dy = Math.max(min.row * T, 0, -((min.row + tam.alto) * T));
  return Math.hypot(dx, dy);
}

function estadoInicial(asentamiento: Asentamiento, facciones: ReturnType<typeof crearFacciones>): EstadoSimulacion {
  return crearEstadoDeTest([asentamiento], facciones);
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
      if (esDeAfueras(e.tipo)) {
        // La banda de afueras se mide sobre la HUELLA, no sobre el centro: lo que no puede entrar en la ciudad
        // es el edificio entero. Su centro sí puede quedar más allá de la banda — el edificio se extiende hacia
        // afuera desde su borde interior, que es lo que la banda acota.
        // El veto es un piso DURO: ninguna celda del edificio entra de aquí para dentro.
        expect(bordeInterior(e), `${e.tipo} se mete en el radio vedado`).toBeGreaterThanOrEqual(TRAZADO.radioAfuerasMin - 1e-6);
        // Por arriba NO hay regla dura: §11 dice que las afueras llegan "AL MENOS" hasta
        // `radioAfuerasMin + anchoBandaAfueras`, y el techo real sube con `radioPotencial` y con el tamaño del
        // propio edificio (`radioMaximoAfueras`). Lo que sí tiene que cumplirse es que quepa entero en el
        // espacio local que se dibuja — si no, quedaría fuera del lienzo de la Vista de Asentamiento.
        expect(esquinaMasLejana(e), `${e.tipo} se sale del espacio local dibujable`).toBeLessThanOrEqual(
          REJILLA_ASENTAMIENTO.radioMapa
        );
      } else {
        expect(modulo(e.posicion)).toBeLessThanOrEqual(ZONA_INFLUENCIA.radioInicial + 1e-6);
      }
    }
  });
});

describe('Vista de Asentamiento — colocación tras simulación', () => {
  it('los edificios internos quedan dentro del disco local; las minas/cantera van al mapa general sobre su nodo', () => {
    const mapa = crearMapaDeterminista(SEED);
    const facciones = crearFacciones();
    const rng = createRng(SEED);
    const { asentamiento } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);
    let estado = estadoInicial(asentamiento, facciones);
    for (let tick = 1; tick <= 90; tick++) estado = avanzarSimulacion(estado, mapa, contextoDeTest(tick, rng));

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
