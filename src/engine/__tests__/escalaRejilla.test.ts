// Invariante de ESCALA de la rejilla del asentamiento (Etapa 6, `Consideraciones/
// Vista_Asentamiento_Trazado_Urbano.md` §E6.11, Paso 1 del plan §E6.15).
//
// Qué congela: **la huella de un edificio en UNIDADES LOCALES es función de sus datos persistidos, no de la
// resolución a la que se discretice la rejilla.**
//
// Por qué importa: la Etapa 6 parte la celda por la mitad (`tamanoCelda` 6 → 3) y dobla todas las huellas
// (`EDIFICIO_TAMANO` ×2), de modo que la geometría física no cambia. De esa identidad depende la afirmación
// más fuerte del rediseño — **cero migración de snapshot**: `Edificio.posicion` está persistido en unidades
// locales, así que si el mismo registro guardado se dibujara en otro sitio tras el reescalado, todas las
// partidas quedarían corruptas y habría que migrarlas.
//
// El álgebra dice que la identidad es exacta, sin error de redondeo. Con `col' = 2·col` y `ancho' = 2·ancho`:
//
//   celdaMin' = round(punto.x / 3 − ancho') = round(2·col + ancho − ancho) = 2·col
//   punto'    = (2·col + ancho'/2) · 3 = (2·col + ancho) · 3 = (col + ancho/2) · 6 = punto
//
// Pero un test que reimplementara esa fórmula solo se probaría a sí mismo. Por eso la tabla de abajo se
// GENERÓ con el código real ANTES del reescalado (`tamanoCelda = 6`) y se dejó fija: es una aserción
// CRUZADA ENTRE VERSIONES, y esa es toda su utilidad. Si alguien vuelve a tocar la escala, esto tiene que
// seguir en verde sin editar un solo número.
//
// Los 40 casos están anclados todos en la MISMA celda de la rejilla original, la (1, −2), para que la huella
// esperada sea siempre `x: 6, y: -12`: si un tipo se desplaza, salta a la vista cuál.
import { describe, expect, it } from 'vitest';
import type { Edificio, EdificioTipo } from '../../domain/types';
import { EDIFICIOS_TIPO } from '../../domain/types';
import { REJILLA_ASENTAMIENTO } from '../../constants';
import {
  celdaMinimaDeEdificio,
  permiteRotacion,
  puntoDeRectangulo,
  tamanoDeEdificio,
  tamanoEdificio,
} from '../trazado';

type CasoDorado = Pick<Edificio, 'tipo'> & {
  nivelInterno?: number;
  rotado?: boolean;
  posicion: { x: number; y: number };
  huella: { x: number; y: number; ancho: number; alto: number };
};

/** Generado con el código real a `tamanoCelda = 6`, celda de anclaje (1, −2). NO regenerar al cambiar la
 * ESCALA: el valor del test es justamente que estos números no se toquen por eso.
 *
 * SÍ se actualiza la fila de un tipo cuando cambia su TAMAÑO FÍSICO real (otra tabla de `constants.ts`, no la
 * escala) — igual que cualquier snapshot. Historial: `puestoMercado` formas 1 y 2 (playtest del lab,
 * 2026-08-31): forma 1 pasó de 2×2 a 1×2 celdas originales, forma 2 de 3×2 a 1×3, y ese mismo día
 * `puestoMercado` salió de `TIPOS_SIN_ROTACION` — de ahí sus filas `rotado: true`. */
const DORADOS: CasoDorado[] = [
  // Centro Urbano: `posicion` pasó de ser el VÉRTICE de una esquina a ser el CENTRO real (doc trazado §E6.20,
  // 2026-08-31). Ahora su huella cae en la misma celda de anclaje que todos los demás — `x: 6, y: -12`.
  { tipo: 'centroUrbano', posicion: { x: 15, y: -3 }, huella: { x: 6, y: -12, ancho: 18, alto: 18 } },
  { tipo: 'vivienda', posicion: { x: 9, y: -9 }, huella: { x: 6, y: -12, ancho: 6, alto: 6 } },
  { tipo: 'granja', nivelInterno: 1, posicion: { x: 12, y: -6 }, huella: { x: 6, y: -12, ancho: 12, alto: 12 } },
  { tipo: 'granja', nivelInterno: 2, posicion: { x: 12, y: -3 }, huella: { x: 6, y: -12, ancho: 12, alto: 18 } },
  { tipo: 'granja', nivelInterno: 3, posicion: { x: 18, y: -3 }, huella: { x: 6, y: -12, ancho: 24, alto: 18 } },
  { tipo: 'granja', nivelInterno: 4, posicion: { x: 24, y: 6 }, huella: { x: 6, y: -12, ancho: 36, alto: 36 } },
  { tipo: 'cantera', posicion: { x: 9, y: -9 }, huella: { x: 6, y: -12, ancho: 6, alto: 6 } },
  { tipo: 'lenera', posicion: { x: 9, y: -9 }, huella: { x: 6, y: -12, ancho: 6, alto: 6 } },
  { tipo: 'almacen', posicion: { x: 12, y: -9 }, huella: { x: 6, y: -12, ancho: 12, alto: 6 } },
  { tipo: 'almacen', rotado: true, posicion: { x: 9, y: -6 }, huella: { x: 6, y: -12, ancho: 6, alto: 12 } },
  // Granero (2026-09-04): NO es una aserción cruzada entre versiones como el resto de la tabla — este tipo no
  // existía antes del reescalado, así que sus números se generaron con el código de hoy anclándolo en la misma
  // celda que los demás. Sigue cumpliendo la otra mitad de su función: si alguien vuelve a tocar la escala,
  // esto tiene que quedarse igual.
  { tipo: 'granero', posicion: { x: 18, y: -6 }, huella: { x: 6, y: -12, ancho: 24, alto: 12 } },
  { tipo: 'granero', rotado: true, posicion: { x: 12, y: 0 }, huella: { x: 6, y: -12, ancho: 12, alto: 24 } },
  { tipo: 'mina', posicion: { x: 9, y: -9 }, huella: { x: 6, y: -12, ancho: 6, alto: 6 } },
  { tipo: 'minaCobre', posicion: { x: 9, y: -9 }, huella: { x: 6, y: -12, ancho: 6, alto: 6 } },
  { tipo: 'minaEstano', posicion: { x: 9, y: -9 }, huella: { x: 6, y: -12, ancho: 6, alto: 6 } },
  { tipo: 'fundicion', posicion: { x: 12, y: -6 }, huella: { x: 6, y: -12, ancho: 12, alto: 12 } },
  { tipo: 'granFundicion', posicion: { x: 9, y: -9 }, huella: { x: 6, y: -12, ancho: 6, alto: 6 } },
  { tipo: 'corral', posicion: { x: 18, y: -3 }, huella: { x: 6, y: -12, ancho: 24, alto: 18 } },
  { tipo: 'corral', rotado: true, posicion: { x: 15, y: 0 }, huella: { x: 6, y: -12, ancho: 18, alto: 24 } },
  { tipo: 'armeria', posicion: { x: 12, y: -3 }, huella: { x: 6, y: -12, ancho: 12, alto: 18 } },
  { tipo: 'armeria', rotado: true, posicion: { x: 15, y: -6 }, huella: { x: 6, y: -12, ancho: 18, alto: 12 } },
  { tipo: 'curtiduria', posicion: { x: 12, y: -6 }, huella: { x: 6, y: -12, ancho: 12, alto: 12 } },
  { tipo: 'carpinteria', posicion: { x: 18, y: -6 }, huella: { x: 6, y: -12, ancho: 24, alto: 12 } },
  { tipo: 'carpinteria', rotado: true, posicion: { x: 12, y: 0 }, huella: { x: 6, y: -12, ancho: 12, alto: 24 } },
  { tipo: 'palacio', posicion: { x: 18, y: 0 }, huella: { x: 6, y: -12, ancho: 24, alto: 24 } },
  { tipo: 'barracon', posicion: { x: 12, y: -6 }, huella: { x: 6, y: -12, ancho: 12, alto: 12 } },
  { tipo: 'galeriaDeTiro', posicion: { x: 12, y: 0 }, huella: { x: 6, y: -12, ancho: 12, alto: 24 } },
  { tipo: 'galeriaDeTiro', rotado: true, posicion: { x: 18, y: -6 }, huella: { x: 6, y: -12, ancho: 24, alto: 12 } },
  { tipo: 'mercado', posicion: { x: 15, y: -6 }, huella: { x: 6, y: -12, ancho: 18, alto: 12 } },
  { tipo: 'mercado', rotado: true, posicion: { x: 12, y: -3 }, huella: { x: 6, y: -12, ancho: 12, alto: 18 } },
  { tipo: 'puestoMercado', nivelInterno: 1, posicion: { x: 9, y: -6 }, huella: { x: 6, y: -12, ancho: 6, alto: 12 } },
  { tipo: 'puestoMercado', nivelInterno: 1, rotado: true, posicion: { x: 12, y: -9 }, huella: { x: 6, y: -12, ancho: 12, alto: 6 } },
  { tipo: 'puestoMercado', nivelInterno: 2, posicion: { x: 9, y: -3 }, huella: { x: 6, y: -12, ancho: 6, alto: 18 } },
  { tipo: 'puestoMercado', nivelInterno: 2, rotado: true, posicion: { x: 15, y: -9 }, huella: { x: 6, y: -12, ancho: 18, alto: 6 } },
  { tipo: 'puestoMercado', nivelInterno: 3, posicion: { x: 9, y: -9 }, huella: { x: 6, y: -12, ancho: 6, alto: 6 } },
  { tipo: 'maravilla', posicion: { x: 9, y: -9 }, huella: { x: 6, y: -12, ancho: 6, alto: 6 } },
  { tipo: 'plaza', posicion: { x: 12, y: -6 }, huella: { x: 6, y: -12, ancho: 12, alto: 12 } },
  { tipo: 'plazaDeArmas', posicion: { x: 12, y: -6 }, huella: { x: 6, y: -12, ancho: 12, alto: 12 } },
  { tipo: 'patioDeGremios', posicion: { x: 12, y: -6 }, huella: { x: 6, y: -12, ancho: 12, alto: 12 } },
  { tipo: 'tallerCarpinteria', posicion: { x: 12, y: -6 }, huella: { x: 6, y: -12, ancho: 12, alto: 12 } },
  { tipo: 'pozo', posicion: { x: 9, y: -9 }, huella: { x: 6, y: -12, ancho: 6, alto: 6 } },
  { tipo: 'parque', posicion: { x: 15, y: -6 }, huella: { x: 6, y: -12, ancho: 18, alto: 12 } },
  { tipo: 'parque', rotado: true, posicion: { x: 12, y: -3 }, huella: { x: 6, y: -12, ancho: 12, alto: 18 } },
];

/** Huella en unidades locales, tal y como la calcula `trazadoParaAsentamiento` para el cliente. */
function huellaLocal(caso: CasoDorado): { x: number; y: number; ancho: number; alto: number } {
  const T = REJILLA_ASENTAMIENTO.tamanoCelda;
  const e = { tipo: caso.tipo, nivelInterno: caso.nivelInterno, posicion: caso.posicion, rotado: caso.rotado };
  const min = celdaMinimaDeEdificio(e);
  const tamano = tamanoDeEdificio(e);
  return { x: min.col * T, y: min.row * T, ancho: tamano.ancho * T, alto: tamano.alto * T };
}

describe('escala de la rejilla (doc trazado §E6.11)', () => {
  it('la huella LOCAL de un edificio persistido no depende de la resolución de la rejilla', () => {
    // Se comparan las 40 de una vez y no una por `it`: si el reescalado desplaza algo, interesa ver TODOS los
    // tipos afectados en el mismo fallo, no el primero que salte.
    const obtenidas = DORADOS.map((c) => ({ tipo: c.tipo, nivelInterno: c.nivelInterno, rotado: c.rotado, huella: huellaLocal(c) }));
    const esperadas = DORADOS.map((c) => ({ tipo: c.tipo, nivelInterno: c.nivelInterno, rotado: c.rotado, huella: c.huella }));
    expect(obtenidas).toEqual(esperadas);
  });

  it('la tabla dorada cubre todos los tipos del catálogo (si no, un tipo nuevo se colaría sin cobertura)', () => {
    const cubiertos = new Set<EdificioTipo>(DORADOS.map((c) => c.tipo));
    expect([...EDIFICIOS_TIPO].filter((t) => !cubiertos.has(t))).toEqual([]);
  });

  it('`posicion` y celda son inversas exactas a CUALQUIER escala, para todo tipo y en un rango de celdas', () => {
    // Complementa a la tabla dorada: aquella es una aserción cruzada entre versiones (números fijos), esta es
    // la propiedad algebraica, que tiene que cumplirse sea cual sea `tamanoCelda` hoy.
    // Centro Urbano ya NO es excepción (doc trazado §E6.20): su `posicion` es su centro, como todos.
    const desajustes: string[] = [];
    for (const tipo of EDIFICIOS_TIPO) {
      const niveles = tipo === 'granja' ? [1, 2, 3, 4] : tipo === 'puestoMercado' ? [1, 2, 3] : [undefined];
      for (const nivelInterno of niveles) {
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
