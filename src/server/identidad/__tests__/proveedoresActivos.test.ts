import { describe, expect, it } from 'vitest';
import { crearRepositorioIdentidadEnMemoria } from '../repositorioEnMemoria';
import { proveedoresDeProceso, proveedoresPorDefecto } from '../proveedoresActivos';
import { proveedorDesarrollo } from '../proveedorDesarrollo';

describe('proveedoresPorDefecto (default de crearServidor, solo tests)', () => {
  it('es solo el proveedor de desarrollo', () => {
    expect(proveedoresPorDefecto()).toEqual([proveedorDesarrollo]);
  });
});

describe('proveedoresDeProceso (lo que arranca el proceso real)', () => {
  const proveedores = proveedoresDeProceso(crearRepositorioIdentidadEnMemoria());

  it('trae cuentas locales (clave) y el proveedor dev para el cliente de admin', () => {
    expect(proveedores.map((p) => p.esquema).sort()).toEqual(['clave', 'dev']);
  });

  it('no declara dos adaptadores para el mismo esquema', () => {
    const esquemas = proveedores.map((p) => p.esquema);
    expect(new Set(esquemas).size).toBe(esquemas.length);
  });
});
