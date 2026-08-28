import { describe, expect, it } from 'vitest';
import { CredencialInvalidaError } from '../../../acceso/proveedorIdentidad';
import { ESQUEMA_DESARROLLO, proveedorDesarrollo } from '../proveedorDesarrollo';

describe('proveedorDesarrollo', () => {
  it('expone el esquema dev', () => {
    expect(proveedorDesarrollo.esquema).toBe(ESQUEMA_DESARROLLO);
  });

  it('resuelve sujetoId sin email', async () => {
    const identidad = await proveedorDesarrollo.autenticar('ana');
    expect(identidad).toEqual({ proveedor: 'dev', sujetoId: 'ana', email: undefined });
  });

  it('resuelve sujetoId con email', async () => {
    const identidad = await proveedorDesarrollo.autenticar('ana:ana@example.com');
    expect(identidad).toEqual({ proveedor: 'dev', sujetoId: 'ana', email: 'ana@example.com' });
  });

  it('rechaza valor vacio', async () => {
    await expect(proveedorDesarrollo.autenticar('   ')).rejects.toThrow(CredencialInvalidaError);
  });
});
