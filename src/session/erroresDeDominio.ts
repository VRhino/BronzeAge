// Mapea los tipos de error del motor a códigos ESTABLES de dominio (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md,
// Fase B: "Mapear los 13 tipos de error de dominio del motor a códigos estables"). Es superficie pública de la
// API — un cliente decide qué hacer según el código, nunca parseando el `.message` (que es texto en
// castellano, libre de cambiar). Se hace una vez y se documenta aquí; añadir un error nuevo del motor sin
// añadirlo a esta tabla hace que `codigoDeErrorDominio` devuelva 'desconocido' en vez de fallar en silencio.
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

/** Los 13 tipos de error de dominio que el motor puede lanzar, y su código estable. Orden alfabético por
 * código, no por módulo — así un código nuevo se ubica por lo que significa, no por dónde vive en `engine/`. */
const CODIGOS_DE_ERROR = new Map<Function, string>([
  [CargoInvalidoError, 'cargo.invalido'],
  [CombateInvalidoError, 'combate.invalido'],
  [ConstruccionManualInvalidaError, 'construccion.invalida'],
  [DiplomaciaInvalidaError, 'diplomacia.invalida'],
  [ExpansionInvalidaError, 'expansion.invalida'],
  [FaccionInvalidaError, 'faccion.invalida'],
  [FundacionInvalidaError, 'fundacion.invalida'],
  [FusionInvalidaError, 'fusion.invalida'],
  [OrdenInvalidaError, 'mercado.orden_invalida'],
  [PoliticaInvalidaError, 'politica.invalida'],
  [ReclutamientoInvalidoError, 'tropas.reclutamiento_invalido'],
  [CaravanaInvalidaError, 'comercio.caravana_invalida'],
  [TruequeInvalidoError, 'comercio.trueque_invalido'],
]);

/**
 * Código estable para un error capturado en un comando de `GameSession`, o `undefined` si no es un error de
 * dominio reconocido (en cuyo caso el llamador debe relanzarlo — un error inesperado es un bug, no un
 * rechazo de comando, y no debe convertirse en un `ResultadoComando` silencioso).
 */
export function codigoDeErrorDominio(err: unknown): string | undefined {
  if (!(err instanceof Error)) return undefined;
  return CODIGOS_DE_ERROR.get(err.constructor);
}
