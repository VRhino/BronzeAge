import { describe, expect, it } from 'vitest';
import type { ProveedorIdentidad } from '../proveedorIdentidad';
import { crearRegistroProveedores, proveedoresPorDefecto } from '../registroProveedores';
import { proveedorDesarrollo } from '../proveedorDesarrollo';

function proveedorFalso(esquema: string): ProveedorIdentidad {
  return { esquema, async autenticar() { return { proveedor: esquema, sujetoId: 'x' }; } };
}

describe('registroProveedores', () => {
  it('proveedoresPorDefecto incluye el de desarrollo', () => {
    expect(proveedoresPorDefecto()).toContain(proveedorDesarrollo);
  });

  it('indexa por esquema', () => {
    const registro = crearRegistroProveedores([proveedorFalso('a'), proveedorFalso('b')]);
    expect(registro.get('a')?.esquema).toBe('a');
    expect(registro.get('b')?.esquema).toBe('b');
    expect(registro.get('c')).toBeUndefined();
  });

  it('rechaza esquemas duplicados', () => {
    expect(() => crearRegistroProveedores([proveedorFalso('a'), proveedorFalso('a')])).toThrow(/duplicado/);
  });
});
