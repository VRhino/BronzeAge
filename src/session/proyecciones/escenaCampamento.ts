import { LAYOUT_VERSION, REJILLA_ASENTAMIENTO } from '../../constants';
import type { CampamentoMercenarios } from '../../domain/types';
import { layoutCampamento } from '../../engine/layoutCampamento';
import type { RectanguloLocal } from '../../engine/trazado';
import type { EscenaCampamento } from '../../contratos/v1/dto';

const T = REJILLA_ASENTAMIENTO.tamanoCelda;
const enCeldas = (r: RectanguloLocal) => ({ col: r.x / T, row: r.y / T, ancho: r.ancho / T, alto: r.alto / T });

/** La planta de un campamento para Conquest (doc 01 §10): el trazado del motor, en celdas como `SettlementBattleSnapshot`. */
export function escenaDeCampamento(campamento: Pick<CampamentoMercenarios, 'id' | 'origen' | 'edificios'>): EscenaCampamento {
  const { elementos, calles, empalizada } = layoutCampamento(campamento);
  const celdas = (rects: RectanguloLocal[], clase: 'muro' | 'puerta') =>
    rects.flatMap((r) => {
      const { col, row, ancho, alto } = enCeldas(r);
      return Array.from({ length: ancho * alto }, (_, i) => ({ col: col + (i % ancho), row: row + Math.floor(i / ancho), clase }));
    });
  return {
    campamentoId: campamento.id,
    origen: campamento.origen,
    layoutVersion: LAYOUT_VERSION,
    unidadesPorCelda: T,
    edificios: elementos.map((e) => ({
      edificioId: e.id,
      tipo: e.tipo,
      posicion: { x: e.huella.x + e.huella.ancho / 2, y: e.huella.y + e.huella.alto / 2 },
      ancho: e.huella.ancho / T,
      alto: e.huella.alto / T,
    })),
    calles: calles.map(enCeldas),
    empalizada: [...celdas(empalizada.muro, 'muro'), ...celdas(empalizada.puerta, 'puerta')],
  };
}
