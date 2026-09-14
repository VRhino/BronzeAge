import { LIDERAZGO, TROPAS_RECLUTABLES } from '../../constants';
import { SCHEMA_VERSION, type CatalogoTropas } from './dto';

/**
 * Versión del catálogo que citan los tickets (`BattleRules.versionCatalogoTropas`). Súbela al cambiar cualquier
 * dato de una tropa: el test no deja regenerar `catalogoTropas.json` sin subirla, porque es como sabe Conquest
 * que cambió.
 */
export const VERSION_CATALOGO_TROPAS = 1;

/** El catálogo de tropas que publica BronzeAge para Conquest (doc 01 §13, CQ-003): unidades y Liderazgo son suyos. */
export function catalogoTropas(): CatalogoTropas {
  return {
    schemaVersion: SCHEMA_VERSION,
    version: VERSION_CATALOGO_TROPAS,
    tropas: TROPAS_RECLUTABLES.map((t) => ({
      tropaId: t.id,
      nombre: t.nombre,
      escalon: t.escalon,
      unidades: t.unidadesPorDefecto,
      costeLiderazgo: LIDERAZGO.costePorEscalon[t.escalon]!,
      tipo: t.edificio === 'galeriaDeTiro' ? 'a_distancia' : 'cuerpo_a_cuerpo',
    })),
  };
}
