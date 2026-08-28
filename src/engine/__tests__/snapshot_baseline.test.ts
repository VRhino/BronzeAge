// Snapshot de regresión general: un resumen determinista del estado del motor en varios ticks de una
// simulación de referencia, congelado como baseline versionado (`__snapshots__/`). Cualquier cambio futuro
// en el motor que altere el resultado de una partida "de referencia" — aunque no toque ninguna invariante
// ni reproduzca ninguno de los bugs históricos ya cubiertos — hará que este test falle y obligue a revisar
// el diff a propósito (`vitest run -u` para aceptarlo conscientemente) en vez de colarse en silencio.
import { beforeEach, describe, expect, it } from 'vitest';
import type { Asentamiento, Faccion } from '../../domain/types';
import { avanzarSimulacion } from '../simulation';
import { createRng, type RandomFn } from '../../worldgen';
import {
  contextoDeTest,
  crearEstadoDeTest,
  crearFacciones,
  crearMapaDeterminista,
  fundarAsentamientoDeTest,
  posicionRecomendable,
} from './fixtures';

const SEED = 99;
const TICKS_DE_CORTE = [1, 10, 25, 50, 100];

// Resumen curado en vez del estado crudo completo: más legible en el diff del snapshot cuando algo cambia
// de verdad, y no se rompe por ruido incidental (ids, orden de propiedades) que no aporta a la comparación.
function resumirAsentamiento(a: Asentamiento) {
  const porTipoEstado: Record<string, number> = {};
  for (const edificio of a.edificios) {
    const clave = `${edificio.tipo}:${edificio.estado}`;
    porTipoEstado[clave] = (porTipoEstado[clave] ?? 0) + 1;
  }
  const recursosClave = ['madera', 'piedra', 'trigo', 'cobre', 'estano', 'oro', 'livestock'].map((r) => ({
    recurso: r,
    cantidad: Math.round((a.almacen[r]?.cantidad ?? 0) * 100) / 100,
  }));
  return {
    nivel: a.nivel,
    poblacion: a.poblacion,
    medidorMantenimiento: Math.round(a.medidorMantenimiento * 100) / 100,
    escuadrones: a.escuadrones.length,
    edificios: porTipoEstado,
    recursosClave,
  };
}

function resumirFaccion(f: Faccion) {
  return { nivel: f.nivel, reputacion: Math.round(f.reputacion * 100) / 100 };
}

describe('snapshot de regresión general', () => {
  let rng: RandomFn;

  beforeEach(() => {
    rng = createRng(SEED);
  });

  it('resumen del estado en ticks de referencia coincide con el baseline versionado', () => {
    const mapa = crearMapaDeterminista(SEED);
    const facciones = crearFacciones();
    const posicion = posicionRecomendable(mapa);
    const { asentamiento, facciones: faccionesTrasFundar } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', [], 0, posicion);

    let estado = crearEstadoDeTest([asentamiento], faccionesTrasFundar);

    const cortes: Record<number, unknown> = {};
    for (let tick = 1; tick <= Math.max(...TICKS_DE_CORTE); tick++) {
      estado = avanzarSimulacion(estado, mapa, contextoDeTest(tick, rng));
      if (TICKS_DE_CORTE.includes(tick)) {
        cortes[tick] = {
          asentamientos: estado.asentamientos.map(resumirAsentamiento),
          facciones: estado.facciones.map(resumirFaccion),
        };
      }
    }

    expect(cortes).toMatchSnapshot();
  });
});
