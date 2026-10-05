// La estructura interna de un campamento de mercenarios (Doc 1.9b): el grid de sus edificios y su empalizada, en celdas locales
// (origen en el centro, `y` hacia abajo). Es función pura de `origen` y `edificios`: no se guarda nada y la misma entidad siempre da lo mismo.
import type { CampamentoMercenarios, EdificioCampamentoTipo } from '../domain/types';
import { MERCENARIOS, REJILLA_ASENTAMIENTO } from '../constants';

export interface HuellaCampamento {
  tipo: EdificioCampamentoTipo;
  /** Celda de la esquina superior izquierda. */
  col: number;
  row: number;
  ancho: number;
  alto: number;
}

export interface LayoutCampamento {
  unidadesPorCelda: number;
  edificios: HuellaCampamento[];
  /** Las celdas de la empalizada; `puerta` es por donde se entra y se sale. */
  empalizada: { col: number; row: number; clase: 'muro' | 'puerta' }[];
}

const MILITARES = new Set<EdificioCampamentoTipo>(MERCENARIOS.edificiosMilitares);

export function layoutCampamento(campamento: Pick<CampamentoMercenarios, 'origen' | 'edificios'>): LayoutCampamento {
  const { huellas, disposiciones, interior } = MERCENARIOS.layout;
  const disposicion = disposiciones[campamento.origen % disposiciones.length]!;
  let viviendas = 0;
  const edificios = campamento.edificios.map((tipo): HuellaCampamento => {
    const hueco = tipo === 'vivienda' ? (viviendas++ % 2 === 0 ? 'vivienda0' : 'vivienda1') : MILITARES.has(tipo) ? 'militar' : (tipo as 'taberna' | 'mercado');
    const [col, row] = disposicion[hueco];
    return { tipo, col, row, ...huellas[tipo] };
  });

  const { colMin, colMax, rowMin, rowMax } = interior;
  const empalizada: LayoutCampamento['empalizada'] = [];
  for (let col = colMin - 1; col <= colMax + 1; col++) {
    for (let row = rowMin - 1; row <= rowMax + 1; row++) {
      if (col !== colMin - 1 && col !== colMax + 1 && row !== rowMin - 1 && row !== rowMax + 1) continue;
      const esPuerta = row === rowMax + 1 && (col === -1 || col === 0);
      empalizada.push({ col, row, clase: esPuerta ? 'puerta' : 'muro' });
    }
  }
  return { unidadesPorCelda: REJILLA_ASENTAMIENTO.tamanoCelda, edificios, empalizada };
}
