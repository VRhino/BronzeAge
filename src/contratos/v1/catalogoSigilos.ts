import { CATALOGO_SIGILO } from '../../constants';
import { SCHEMA_VERSION, type CatalogoSigilos } from './dto';

/**
 * El catálogo del sigilo de Facción que publica BronzeAge para sus clientes (Doc 2.8.1). Los ids solo se añaden, nunca
 * se retiran, así que subir `CATALOGO_SIGILO.version` al ampliarlo es informativo; el test no deja regenerar
 * `catalogoSigilos.json` sin subirla.
 */
export function catalogoSigilos(): CatalogoSigilos {
  return {
    schemaVersion: SCHEMA_VERSION,
    version: CATALOGO_SIGILO.version,
    formas: [...CATALOGO_SIGILO.formas],
    campos: [...CATALOGO_SIGILO.campos],
    orlas: [...CATALOGO_SIGILO.orlas],
    emblemas: [...CATALOGO_SIGILO.emblemas],
    colores: CATALOGO_SIGILO.colores.map((c) => ({ ...c })),
    reservados: {
      neutro: { ...CATALOGO_SIGILO.reservados.neutro },
      bandidos: { ...CATALOGO_SIGILO.reservados.bandidos },
      mercenarios: { ...CATALOGO_SIGILO.reservados.mercenarios },
    },
  };
}
