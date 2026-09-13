// Invariantes de la rejilla del asentamiento: huellas en celdas (BA-005) y la inversa posición ↔ celda.
//
// Qué congela:
//  - La tabla de huellas de `Docs/Coordinacion/propuestas/BA-005_huellas_y_escala_asentamiento.md`: los 27 tipos
//    y sus 32 combinaciones de tipo/nivel/forma, en orientación base. Si un tipo cambia de tamaño, esta tabla
//    cambia a la vez que `LAYOUT_VERSION` (constants.ts): un tamaño nuevo reinterpreta las partidas guardadas.
//  - El Centro Urbano, que nace en `(0,0)`, queda CENTRADO en el origen (por eso mide 4×4 y no 3×3).
//  - `posicion` y celda mínima son inversas exactas para todo tipo, rotación y celda, también negativas.
//
// Historia: hasta BA-005 este archivo congelaba la identidad de la Etapa 6 (celda 6 → 3 con huellas ×2, cero
// migración). BA-005 rompe esa identidad a propósito —los edificios miden la mitad— y las partidas viejas se
// rechazan por `LAYOUT_VERSION` en vez de migrarse, así que aquella tabla cruzada entre versiones ya no aplica.
import { describe, expect, it } from 'vitest';
import { EDIFICIOS_TIPO } from '../../domain/types';
import { celdaMinimaDeEdificio, permiteRotacion, puntoDeRectangulo, tamanoDeEdificio, tamanoEdificio } from '../trazado';

/** Ancho × alto en celdas, orientación base. Clave `tipo` o `tipo:nivelInterno` para Granja y puesto de mercado. */
const HUELLAS_BA005: Record<string, [number, number]> = {
  centroUrbano: [4, 4],
  vivienda: [1, 1],
  cantera: [1, 1],
  lenera: [1, 1],
  almacen: [2, 1],
  granero: [4, 2],
  mina: [1, 1],
  minaCobre: [1, 1],
  minaEstano: [1, 1],
  fundicion: [2, 2],
  granFundicion: [1, 1],
  corral: [4, 3],
  armeria: [2, 3],
  curtiduria: [2, 2],
  carpinteria: [4, 2],
  palacio: [4, 4],
  barracon: [2, 2],
  galeriaDeTiro: [2, 4],
  mercado: [3, 2],
  maravilla: [1, 1],
  plaza: [2, 2],
  plazaDeArmas: [2, 2],
  patioDeGremios: [2, 2],
  pozo: [1, 1],
  parque: [3, 2],
  'granja:1': [2, 2],
  'granja:2': [2, 3],
  'granja:3': [4, 3],
  'granja:4': [6, 6],
  'puestoMercado:1': [1, 2],
  'puestoMercado:2': [1, 3],
  'puestoMercado:3': [1, 1],
};

function nivelesDe(tipo: string): (number | undefined)[] {
  return tipo === 'granja' ? [1, 2, 3, 4] : tipo === 'puestoMercado' ? [1, 2, 3] : [undefined];
}

describe('rejilla del asentamiento (BA-005)', () => {
  it('las huellas de los 27 tipos (32 combinaciones) son las de la tabla BA-005', () => {
    const obtenidas: Record<string, [number, number]> = {};
    for (const tipo of EDIFICIOS_TIPO) {
      for (const nivel of nivelesDe(tipo)) {
        const t = tamanoEdificio(tipo, nivel);
        obtenidas[nivel === undefined ? tipo : `${tipo}:${nivel}`] = [t.ancho, t.alto];
      }
    }
    expect(obtenidas).toEqual(HUELLAS_BA005);
  });

  it('el Centro Urbano nacido en (0,0) queda centrado en el origen', () => {
    const cu = { tipo: 'centroUrbano' as const, posicion: { x: 0, y: 0 } };
    const tamano = tamanoDeEdificio(cu);
    const min = celdaMinimaDeEdificio(cu);
    expect(min).toEqual({ col: -tamano.ancho / 2, row: -tamano.alto / 2 });
    expect(puntoDeRectangulo(min, tamano)).toEqual({ x: 0, y: 0 });
  });

  it('`posicion` y celda son inversas exactas para todo tipo, rotación y celda (incluidas negativas)', () => {
    const desajustes: string[] = [];
    for (const tipo of EDIFICIOS_TIPO) {
      for (const nivelInterno of nivelesDe(tipo)) {
        const base = tamanoEdificio(tipo, nivelInterno);
        for (const rotado of permiteRotacion(tipo, base) ? [false, true] : [false]) {
          const tamano = rotado ? { ancho: base.alto, alto: base.ancho } : base;
          for (const col of [-8, -3, 0, 1, 7]) {
            for (const row of [-5, -2, 0, 4, 9]) {
              const posicion = puntoDeRectangulo({ col, row }, tamano);
              const min = celdaMinimaDeEdificio({ tipo, nivelInterno, posicion, rotado });
              if (min.col !== col || min.row !== row) {
                desajustes.push(`${tipo}${nivelInterno ?? ''}${rotado ? '(rotado)' : ''} (${col},${row}) -> (${min.col},${min.row})`);
              }
            }
          }
        }
      }
    }
    expect(desajustes, `\n${desajustes.join('\n')}\n`).toEqual([]);
  });
});
