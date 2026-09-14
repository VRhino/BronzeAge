// Mapea los tipos de error del motor a códigos ESTABLES de dominio (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md,
// Fase B: "Mapear los 13 tipos de error de dominio del motor a códigos estables"). El catálogo de códigos en sí
// vive en `comandos/codigosDeError.ts` (compartido con los códigos que añade la capa de partida, ver ese
// archivo); este módulo solo resuelve QUÉ código le corresponde a cada CLASE de error del motor. Añadir un
// error nuevo del motor sin añadirlo a esta tabla hace que `codigoDeErrorDominio` devuelva `undefined` — el
// llamador lo relanza como bug en vez de fallar en silencio (ver `rechazoDesdeError`, `comandos/tipos.ts`).
import { CargoInvalidoError } from '../engine/cargos';
import { CombateInvalidoError } from '../engine/combate';
import { ConstruccionManualInvalidaError } from '../engine/construction';
import { DiplomaciaInvalidaError } from '../engine/diplomacia';
import { ExpansionInvalidaError } from '../engine/expansion';
import { FaccionInvalidaError } from '../engine/faccion';
import { FusionInvalidaError } from '../engine/fusion';
import { OrdenInvalidaError } from '../engine/market';
import { PoliticaInvalidaError } from '../engine/politicas';
import { FundacionInvalidaError } from '../engine/settlement';
import { CaravanaInvalidaError, TruequeInvalidoError } from '../engine/trade';
import { ReclutamientoInvalidoError } from '../engine/tropas';
import { RecintoInvalidoError } from '../engine/muralla';
import { MovilizacionInvalidaError } from '../engine/ejercitos';
import { PuertaInvalidaError } from '../engine/pertenencia';
import { HeroeInvalidoError } from '../engine/heroe';
import { CODIGOS_ERROR, type CodigoError } from './comandos/codigosDeError';

/** Los 17 tipos de error de dominio que el motor puede lanzar, y su código estable. Orden alfabético por
 * código, no por módulo — así un código nuevo se ubica por lo que significa, no por dónde vive en `engine/`. */
const CODIGOS_DE_ERROR = new Map<Function, CodigoError>([
  [CargoInvalidoError, CODIGOS_ERROR.cargoInvalido],
  [CombateInvalidoError, CODIGOS_ERROR.combateInvalido],
  [MovilizacionInvalidaError, CODIGOS_ERROR.movilizacionInvalida],
  [ConstruccionManualInvalidaError, CODIGOS_ERROR.construccionInvalida],
  [DiplomaciaInvalidaError, CODIGOS_ERROR.diplomaciaInvalida],
  [ExpansionInvalidaError, CODIGOS_ERROR.expansionInvalida],
  [FaccionInvalidaError, CODIGOS_ERROR.faccionInvalida],
  [FundacionInvalidaError, CODIGOS_ERROR.fundacionInvalida],
  [FusionInvalidaError, CODIGOS_ERROR.fusionInvalida],
  [HeroeInvalidoError, CODIGOS_ERROR.heroeInvalido],
  [OrdenInvalidaError, CODIGOS_ERROR.mercadoOrdenInvalida],
  [PoliticaInvalidaError, CODIGOS_ERROR.politicaInvalida],
  [PuertaInvalidaError, CODIGOS_ERROR.puertaInvalida],
  [ReclutamientoInvalidoError, CODIGOS_ERROR.tropasReclutamientoInvalido],
  [CaravanaInvalidaError, CODIGOS_ERROR.comercioCaravanaInvalida],
  [TruequeInvalidoError, CODIGOS_ERROR.comercioTruequeInvalido],
  [RecintoInvalidoError, CODIGOS_ERROR.recintoInvalido],
]);

/**
 * Código estable para un error capturado en un comando de `GameSession`, o `undefined` si no es un error de
 * dominio reconocido (en cuyo caso el llamador debe relanzarlo — un error inesperado es un bug, no un
 * rechazo de comando, y no debe convertirse en un `ResultadoComando` silencioso).
 */
export function codigoDeErrorDominio(err: unknown): CodigoError | undefined {
  if (!(err instanceof Error)) return undefined;
  return CODIGOS_DE_ERROR.get(err.constructor);
}
