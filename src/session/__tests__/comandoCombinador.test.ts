// `comando()` (session/comandos/ayudas.ts) es lo que hace que las dos clases de bug que el doc 4 documenta
// —el `.find(...)!` sistemático y el `try/catch` olvidado— dejen de depender de que quien escribe el manejador
// se acuerde. Este test comprueba las TRES salidas del envoltorio, que es su contrato entero:
//
//   - `RechazoDominio` (lo señala un resolutor de esta capa) -> rechazo con código estable.
//   - Error de dominio del MOTOR -> rechazo con su código mapeado (`erroresDeDominio.ts`).
//   - Cualquier otra excepción -> se RELANZA. Enterrarla en un `ResultadoComando` la haría indistinguible de
//     un rechazo legítimo, y un error inesperado es un bug, no una jugada rechazada.
import { describe, expect, it } from 'vitest';
import { GameSession } from '../gameSession';
import { comando, exigirAsentamiento } from '../comandos/ayudas';
import { asignarCargoLocal } from '../comandos/cargos';
import { renombrarAsentamiento } from '../comandos/construccion';
import { OPC, partidaConAsentamiento } from './fixtures';
import { CombateInvalidoError } from '../../engine/combate';

describe('comando(): traducción uniforme de fallos a rechazo', () => {
  it('un id inexistente se rechaza con código estable, NO revienta con un TypeError', () => {
    const { sesion } = partidaConAsentamiento();
    const r = sesion.ejecutar(renombrarAsentamiento, { asentamientoId: 'no-existe', nombre: 'X' }, OPC);

    expect(r.ok).toBe(false);
    expect(r.codigoError).toBe('asentamiento.no_existe');
    expect(r.eventos).toEqual([]);
  });

  it('un rechazo no incrementa la versión de la partida', () => {
    const { sesion } = partidaConAsentamiento();
    const versionAntes = sesion.getState().version;

    sesion.ejecutar(renombrarAsentamiento, { asentamientoId: 'no-existe', nombre: 'X' }, OPC);

    expect(sesion.getState().version).toBe(versionAntes);
  });

  it('un error de dominio del MOTOR se traduce a su código mapeado', () => {
    const { sesion, asentamientoId, fundador } = partidaConAsentamiento();
    // El motor exige Gobernador antes que cualquier otro cargo local (`engine/cargos.ts`): pedir Tesorero
    // primero lanza `CargoInvalidoError`, que `comando()` traduce vía `rechazoDesdeError`.
    const r = sesion.ejecutar(asignarCargoLocal, { asentamientoId, cargo: 'tesorero', heroeId: fundador }, OPC);

    expect(r.ok).toBe(false);
    expect(r.codigoError).toBe('cargo.invalido');
  });

  it('una excepción que NO es de dominio se relanza en vez de enterrarse como rechazo', () => {
    const explota = comando<void, void>(() => {
      throw new RangeError('bug de programación, no una jugada inválida');
    });
    const sesion = GameSession.crear('t', { seed: 1 });

    expect(() => sesion.ejecutar(explota, undefined, OPC)).toThrow(RangeError);
  });

  it('los resolutores devuelven la entidad ya estrechada cuando existe', () => {
    const { sesion, asentamientoId } = partidaConAsentamiento();
    const leer = comando<void, string>((estado) => {
      const a = exigirAsentamiento(estado, asentamientoId);
      return { estado, resultado: { ok: true, datos: a.id, eventos: [], version: estado.version } };
    });

    expect(sesion.ejecutar(leer, undefined, OPC).datos).toBe(asentamientoId);
  });
});

describe('detalleError: el motivo concreto viaja con el código (2026-10-07)', () => {
  it('un error del motor deja su mensaje en detalleError', () => {
    const lanza = comando<Record<string, never>, void>(() => { throw new CombateInvalidoError('La columna no lleva soldados con los que atacar.'); });
    const { sesion } = partidaConAsentamiento();
    const r = lanza(sesion.getState(), undefined as never, undefined as never, {});
    expect(r.resultado).toMatchObject({ ok: false, codigoError: 'combate.invalido', detalleError: 'La columna no lleva soldados con los que atacar.' });
  });

  it('un rechazo que solo tiene código no inventa detalle', () => {
    const lanza = comando<Record<string, never>, void>((estado) => exigirAsentamiento(estado, 'no-existe') as never);
    const { sesion } = partidaConAsentamiento();
    const r = lanza(sesion.getState(), undefined as never, undefined as never, {});
    expect(r.resultado.codigoError).toBe('asentamiento.no_existe');
    expect(r.resultado).not.toHaveProperty('detalleError');
  });
});
