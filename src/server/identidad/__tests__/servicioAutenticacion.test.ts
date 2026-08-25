import { describe, expect, it } from 'vitest';
import { crearRepositorioIdentidadEnMemoria } from '../repositorio';
import { crearRegistroProveedores } from '../registroProveedores';
import { proveedorDesarrollo } from '../proveedorDesarrollo';
import {
  CabeceraAutorizacionInvalidaError,
  ProveedorDesconocidoError,
  autenticar,
  resolverSesion,
} from '../servicioAutenticacion';
import { CredencialInvalidaError } from '../proveedorIdentidad';

function crearContexto() {
  return {
    proveedores: crearRegistroProveedores([proveedorDesarrollo]),
    repositorio: crearRepositorioIdentidadEnMemoria(),
    ahora: () => '2026-01-01T00:00:00.000Z',
    generarSesionId: (() => {
      let n = 0;
      return () => `sesion-${++n}`;
    })(),
  };
}

describe('autenticar', () => {
  it('crea un Usuario nuevo la primera vez y emite una Sesion', async () => {
    const ctx = crearContexto();
    const { usuario, sesion } = await autenticar('dev ana', ctx);

    expect(usuario.deshabilitado).toBe(false);
    expect(sesion.usuarioId).toBe(usuario.id);
    expect(sesion.emitidaEn).toBe('2026-01-01T00:00:00.000Z');
    expect(sesion.expiraEn > sesion.emitidaEn).toBe(true);
  });

  it('reutiliza el mismo Usuario para la misma identidad externa, pero emite Sesion nueva cada vez', async () => {
    const ctx = crearContexto();
    const primera = await autenticar('dev ana', ctx);
    const segunda = await autenticar('dev ana', ctx);

    expect(segunda.usuario.id).toBe(primera.usuario.id);
    expect(segunda.sesion.id).not.toBe(primera.sesion.id);
  });

  it('usuarios distintos (sujetoId distinto) obtienen Usuario distinto', async () => {
    const ctx = crearContexto();
    const ana = await autenticar('dev ana', ctx);
    const luis = await autenticar('dev luis', ctx);
    expect(ana.usuario.id).not.toBe(luis.usuario.id);
  });

  it('rechaza cabecera ausente o mal formada', async () => {
    const ctx = crearContexto();
    await expect(autenticar(undefined, ctx)).rejects.toThrow(CabeceraAutorizacionInvalidaError);
    await expect(autenticar('sin-espacio', ctx)).rejects.toThrow(CabeceraAutorizacionInvalidaError);
    await expect(autenticar('dev', ctx)).rejects.toThrow(CabeceraAutorizacionInvalidaError);
  });

  it('rechaza esquema sin proveedor registrado', async () => {
    const ctx = crearContexto();
    await expect(autenticar('oauth token-x', ctx)).rejects.toThrow(ProveedorDesconocidoError);
  });

  it('propaga el rechazo del proveedor (credencial invalida)', async () => {
    const ctx = crearContexto();
    // Cabecera bien formada (esquema + valor no vacío) pero con sujetoId vacío tras el ':' — el proveedor de
    // desarrollo es quien rechaza esto, no el parseo de la cabecera en sí.
    await expect(autenticar('dev :correo@example.com', ctx)).rejects.toThrow(CredencialInvalidaError);
  });

  it('rechaza si el Usuario ya vinculado esta deshabilitado', async () => {
    const ctx = crearContexto();
    const { usuario } = await autenticar('dev ana', ctx);
    // No hay setter en el puerto (intencional, ver repositorio.ts): se muta directamente el objeto que
    // devuelve el repositorio en memoria, simulando un baneo aplicado por fuera de este flujo.
    usuario.deshabilitado = true;

    await expect(autenticar('dev ana', ctx)).rejects.toThrow(CredencialInvalidaError);
  });
});

describe('resolverSesion', () => {
  it('resuelve una sesion vigente emitida por autenticar', async () => {
    const ctx = crearContexto();
    const { usuario, sesion } = await autenticar('dev ana', ctx);

    const resuelto = resolverSesion(`sesion ${sesion.id}`, ctx);
    expect(resuelto?.usuario.id).toBe(usuario.id);
    expect(resuelto?.sesion.id).toBe(sesion.id);
  });

  it('devuelve undefined si la cabecera falta, no es del esquema sesion, o el id no existe', async () => {
    const ctx = crearContexto();
    const { sesion } = await autenticar('dev ana', ctx);

    expect(resolverSesion(undefined, ctx)).toBeUndefined();
    expect(resolverSesion(`dev ${sesion.id}`, ctx)).toBeUndefined(); // esquema equivocado
    expect(resolverSesion('sesion inexistente', ctx)).toBeUndefined();
  });

  it('devuelve undefined si la sesion ya expiro', async () => {
    const ctx = crearContexto();
    const { sesion } = await autenticar('dev ana', ctx);

    const resuelto = resolverSesion(`sesion ${sesion.id}`, { ...ctx, ahora: () => '2027-01-01T00:00:00.000Z' });
    expect(resuelto).toBeUndefined();
  });
});
