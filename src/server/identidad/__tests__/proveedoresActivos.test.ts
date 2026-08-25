import { describe, expect, it } from 'vitest';
import { proveedoresPorDefecto } from '../proveedoresActivos';
import { proveedorDesarrollo } from '../proveedorDesarrollo';

describe('proveedoresPorDefecto', () => {
  it('incluye el proveedor de desarrollo (unico activo hoy)', () => {
    expect(proveedoresPorDefecto()).toContain(proveedorDesarrollo);
  });

  it('no declara dos adaptadores para el mismo esquema', () => {
    const esquemas = proveedoresPorDefecto().map((p) => p.esquema);
    expect(new Set(esquemas).size).toBe(esquemas.length);
  });
});
