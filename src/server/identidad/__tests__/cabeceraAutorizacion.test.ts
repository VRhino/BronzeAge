import { describe, expect, it } from 'vitest';
import {
  CabeceraAutorizacionInvalidaError,
  credencialDesdeCabecera,
  credencialOpcionalDesdeCabecera,
} from '../cabeceraAutorizacion';

describe('credencialDesdeCabecera', () => {
  it('separa esquema y valor', () => {
    expect(credencialDesdeCabecera('dev ana')).toEqual({ esquema: 'dev', valor: 'ana' });
    expect(credencialDesdeCabecera('sesion sesion-1')).toEqual({ esquema: 'sesion', valor: 'sesion-1' });
  });

  it('normaliza el esquema a minusculas: es insensible a mayusculas por RFC 7235', () => {
    expect(credencialDesdeCabecera('DEV ana').esquema).toBe('dev');
  });

  it('conserva el valor tal cual, incluidos los separadores internos', () => {
    expect(credencialDesdeCabecera('dev ana:ana@example.com').valor).toBe('ana:ana@example.com');
  });

  it('rechaza cabecera ausente, sin espacio, o sin valor', () => {
    expect(() => credencialDesdeCabecera(undefined)).toThrow(CabeceraAutorizacionInvalidaError);
    expect(() => credencialDesdeCabecera('sin-espacio')).toThrow(CabeceraAutorizacionInvalidaError);
    expect(() => credencialDesdeCabecera('dev')).toThrow(CabeceraAutorizacionInvalidaError);
    expect(() => credencialDesdeCabecera('dev   ')).toThrow(CabeceraAutorizacionInvalidaError);
    expect(() => credencialDesdeCabecera(' ana')).toThrow(CabeceraAutorizacionInvalidaError); // esquema vacío
  });
});

describe('credencialOpcionalDesdeCabecera', () => {
  it('devuelve undefined en vez de lanzar', () => {
    expect(credencialOpcionalDesdeCabecera(undefined)).toBeUndefined();
    expect(credencialOpcionalDesdeCabecera('sin-espacio')).toBeUndefined();
    expect(credencialOpcionalDesdeCabecera('dev ana')).toEqual({ esquema: 'dev', valor: 'ana' });
  });
});
