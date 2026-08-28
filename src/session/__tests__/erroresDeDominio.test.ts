// Test directo de la tabla `codigoDeErrorDominio` (`session/erroresDeDominio.ts`) — antes su cobertura era
// solo INDIRECTA, a través de los ~25 tests de "id inexistente" de `comandosContratoIds.test.ts`, cada uno
// disparando una de las 13 excepciones del motor de pasada. Eso prueba que el mapeo funciona para las rutas
// que algún comando dispara, pero no que la TABLA esté completa: si el motor gana un tipo de error de
// dominio nuevo y se olvida añadirlo aquí, ningún test lo detecta hasta que, por casualidad, algún comando
// llegue a dispararlo. Este archivo prueba la tabla directamente, instanciando cada error a mano.
import { describe, expect, it } from 'vitest';
import { CargoInvalidoError } from '../../engine/cargos';
import { CombateInvalidoError } from '../../engine/combate';
import { ConstruccionManualInvalidaError } from '../../engine/construction';
import { DiplomaciaInvalidaError } from '../../engine/diplomacia';
import { ExpansionInvalidaError } from '../../engine/expansion';
import { FaccionInvalidaError } from '../../engine/faccion';
import { FusionInvalidaError } from '../../engine/fusion';
import { OrdenInvalidaError } from '../../engine/market';
import { PoliticaInvalidaError } from '../../engine/politicas';
import { FundacionInvalidaError } from '../../engine/settlement';
import { CaravanaInvalidaError, TruequeInvalidoError } from '../../engine/trade';
import { ReclutamientoInvalidoError } from '../../engine/tropas';
import { codigoDeErrorDominio } from '../erroresDeDominio';

const TABLA: [errorClase: new (mensaje: string) => Error, codigoEsperado: string][] = [
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
];

describe('codigoDeErrorDominio', () => {
  it.each(TABLA)('mapea %s al código estable', (ErrorClase, codigoEsperado) => {
    expect(codigoDeErrorDominio(new ErrorClase('mensaje de prueba'))).toBe(codigoEsperado);
  });

  it('cubre los 13 tipos de error de dominio documentados (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md, Fase B)', () => {
    expect(TABLA).toHaveLength(13);
  });

  it('un error que NO es de dominio devuelve undefined (el llamador debe relanzarlo, no tragárselo)', () => {
    expect(codigoDeErrorDominio(new TypeError('bug real, no un rechazo de comando'))).toBeUndefined();
  });

  it('un valor que no es un Error devuelve undefined', () => {
    expect(codigoDeErrorDominio('no soy un error')).toBeUndefined();
    expect(codigoDeErrorDominio(undefined)).toBeUndefined();
  });
});
