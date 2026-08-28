// El servicio de autenticación se prueba con `Credencial` ya parseada, sin cabeceras HTTP: desde la revisión
// de separación negocio/infraestructura (2026-08-25) el parseo de `Authorization` vive en
// `server/identidad/cabeceraAutorizacion.ts` y tiene sus propias pruebas. Lo que se comprueba aquí es la
// POLÍTICA: find-or-create de `Usuario`, emisión y caducidad de `Sesion`, cuentas deshabilitadas.
//
// Sin importar NADA de `server/`: los dos puertos se satisfacen con dobles locales. Si algún día este
// archivo necesitara un adaptador real para pasar, sería señal de que la separación se rompió.
import { describe, expect, it } from 'vitest';
import { ProveedorDesconocidoError, autenticar, resolverSesion } from '../servicioAutenticacion';
import { CredencialInvalidaError, crearRegistroProveedores, type ProveedorIdentidad } from '../proveedorIdentidad';
import { repositorioDePrueba } from './repositorioDePrueba';

/** Doble del proveedor `dev`: acepta `<sujetoId>` y rechaza el vacío. Deliberadamente NO es el adaptador
 * real de `server/identidad/` — este servicio no debe necesitar infraestructura para probarse. */
const proveedorFalso: ProveedorIdentidad = {
  esquema: 'dev',
  async autenticar(valor: string) {
    const [sujetoId] = valor.split(':');
    if (!sujetoId) throw new CredencialInvalidaError();
    return { proveedor: 'dev', sujetoId };
  },
};

function crearContexto() {
  return {
    proveedores: crearRegistroProveedores([proveedorFalso]),
    repositorio: repositorioDePrueba(),
    ahora: () => '2026-01-01T00:00:00.000Z',
    generarSesionId: (() => {
      let n = 0;
      return () => `sesion-${++n}`;
    })(),
  };
}

/** Credencial del esquema `dev`, ya parseada. */
function dev(valor: string) {
  return { esquema: 'dev', valor };
}

describe('autenticar', () => {
  it('crea un Usuario nuevo la primera vez y emite una Sesion', async () => {
    const ctx = crearContexto();
    const { usuario, sesion } = await autenticar(dev('ana'), ctx);

    expect(usuario.deshabilitado).toBe(false);
    expect(sesion.usuarioId).toBe(usuario.id);
    expect(sesion.emitidaEn).toBe('2026-01-01T00:00:00.000Z');
    expect(sesion.expiraEn > sesion.emitidaEn).toBe(true);
  });

  it('reutiliza el mismo Usuario para la misma identidad externa, pero emite Sesion nueva cada vez', async () => {
    const ctx = crearContexto();
    const primera = await autenticar(dev('ana'), ctx);
    const segunda = await autenticar(dev('ana'), ctx);

    expect(segunda.usuario.id).toBe(primera.usuario.id);
    expect(segunda.sesion.id).not.toBe(primera.sesion.id);
  });

  it('usuarios distintos (sujetoId distinto) obtienen Usuario distinto', async () => {
    const ctx = crearContexto();
    const ana = await autenticar(dev('ana'), ctx);
    const luis = await autenticar(dev('luis'), ctx);
    expect(ana.usuario.id).not.toBe(luis.usuario.id);
  });

  it('rechaza un esquema sin proveedor registrado', async () => {
    const ctx = crearContexto();
    await expect(autenticar({ esquema: 'oauth', valor: 'token-x' }, ctx)).rejects.toThrow(ProveedorDesconocidoError);
  });

  it('propaga el rechazo del proveedor (credencial invalida)', async () => {
    const ctx = crearContexto();
    // Valor no vacío pero con sujetoId vacío: lo rechaza el proveedor, no el parseo.
    await expect(autenticar(dev(':correo@example.com'), ctx)).rejects.toThrow(CredencialInvalidaError);
  });

  it('rechaza si el Usuario ya vinculado esta deshabilitado', async () => {
    const ctx = crearContexto();
    const { usuario } = await autenticar(dev('ana'), ctx);
    // No hay setter en el puerto (intencional): se muta el objeto del repositorio en memoria, simulando un
    // baneo aplicado por fuera de este flujo.
    usuario.deshabilitado = true;

    await expect(autenticar(dev('ana'), ctx)).rejects.toThrow(CredencialInvalidaError);
  });
});

describe('resolverSesion', () => {
  it('resuelve una sesion vigente emitida por autenticar', async () => {
    const ctx = crearContexto();
    const { usuario, sesion } = await autenticar(dev('ana'), ctx);

    const resuelto = resolverSesion({ esquema: 'sesion', valor: sesion.id }, ctx);
    expect(resuelto?.usuario.id).toBe(usuario.id);
    expect(resuelto?.sesion.id).toBe(sesion.id);
  });

  it('devuelve undefined sin credencial, con otro esquema, o con un id inexistente', async () => {
    const ctx = crearContexto();
    const { sesion } = await autenticar(dev('ana'), ctx);

    expect(resolverSesion(undefined, ctx)).toBeUndefined();
    expect(resolverSesion(dev(sesion.id), ctx)).toBeUndefined(); // esquema equivocado
    expect(resolverSesion({ esquema: 'sesion', valor: 'inexistente' }, ctx)).toBeUndefined();
  });

  it('devuelve undefined si la sesion ya expiro', async () => {
    const ctx = crearContexto();
    const { sesion } = await autenticar(dev('ana'), ctx);

    const resuelto = resolverSesion({ esquema: 'sesion', valor: sesion.id }, { ...ctx, ahora: () => '2027-01-01T00:00:00.000Z' });
    expect(resuelto).toBeUndefined();
  });

  it('devuelve undefined si el Usuario de la sesion fue deshabilitado despues de emitirla', async () => {
    const ctx = crearContexto();
    const { usuario, sesion } = await autenticar(dev('ana'), ctx);
    usuario.deshabilitado = true;

    expect(resolverSesion({ esquema: 'sesion', valor: sesion.id }, ctx)).toBeUndefined();
  });
});
