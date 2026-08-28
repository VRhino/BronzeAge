// El registro de proveedores se prueba con dobles definidos aquí mismo: es la demostración de que el puerto
// no necesita ningún adaptador real para funcionar (y de que `acceso/` no depende de `server/`).
import { describe, expect, it } from 'vitest';
import { crearRegistroProveedores, type ProveedorIdentidad } from '../proveedorIdentidad';

function proveedorFalso(esquema: string): ProveedorIdentidad {
  return {
    esquema,
    async autenticar() {
      return { proveedor: esquema, sujetoId: 'x' };
    },
  };
}

describe('crearRegistroProveedores', () => {
  it('indexa por esquema', () => {
    const registro = crearRegistroProveedores([proveedorFalso('a'), proveedorFalso('b')]);
    expect(registro.get('a')?.esquema).toBe('a');
    expect(registro.get('b')?.esquema).toBe('b');
    expect(registro.get('c')).toBeUndefined();
  });

  it('rechaza esquemas duplicados en vez de quedarse con uno en silencio', () => {
    expect(() => crearRegistroProveedores([proveedorFalso('a'), proveedorFalso('a')])).toThrow(/duplicado/);
  });
});
