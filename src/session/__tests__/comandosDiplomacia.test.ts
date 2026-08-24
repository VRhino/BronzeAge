// Grupo de comandos de diplomacia (`session/comandos/diplomacia.ts`). Verifica el contrato de la capa de
// partida, no las reglas del motor (requisitos de nivel, reputación, etc., ya cubiertos en `engine/`).
//
// El caso de `romperRelacion` con una relación inexistente es el que importa especialmente: en `GameStore`
// ese comando NO tenía try/catch, así que el `DiplomaciaInvalidaError` del motor llegaba crudo a la interfaz.
import { describe, expect, it } from 'vitest';
import { GameSession } from '../gameSession';
import { crearFaccion } from '../comandos/crearFaccion';
import { anexionar, fusionar, proponerRelacion, rebelionVasallo, romperRelacion } from '../comandos/diplomacia';

const MOMENTO = '2026-01-01T00:00:00.000Z';
const OPC = { momento: MOMENTO, actor: 'jugador-test' };

function partidaConDosFacciones() {
  const sesion = GameSession.crear('diplo-test', { seed: 42 });
  const a = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC).datos!.faccionId;
  const b = sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, OPC).datos!.faccionId;
  return { sesion, a, b };
}

describe('romperRelacion', () => {
  it('rechazo: relación inexistente devuelve codigoError (en GameStore lanzaba excepción sin capturar)', () => {
    const { sesion, a } = partidaConDosFacciones();
    const resultado = sesion.ejecutar(romperRelacion, { relacionId: 'no-existe', iniciadorFaccionId: a }, OPC);

    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe('diplomacia.invalida');
  });

  it('rechazo: sin relacionId no hace nada y no versiona', () => {
    const { sesion, a } = partidaConDosFacciones();
    const antes = sesion.getState();
    const resultado = sesion.ejecutar(romperRelacion, { relacionId: '', iniciadorFaccionId: a }, OPC);

    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe('diplomacia.relacion_no_indicada');
    expect(sesion.getState()).toBe(antes);
  });
});

describe('rebelionVasallo', () => {
  it('rechazo: relación inexistente se traduce a codigoError', () => {
    const { sesion } = partidaConDosFacciones();
    const resultado = sesion.ejecutar(rebelionVasallo, { relacionId: 'no-existe' }, OPC);

    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe('diplomacia.invalida');
  });

  it('rechazo: sin relacionId no muta el estado', () => {
    const { sesion } = partidaConDosFacciones();
    const antes = sesion.getState();
    sesion.ejecutar(rebelionVasallo, { relacionId: '' }, OPC);
    expect(sesion.getState()).toBe(antes);
  });
});

describe('proponerRelacion', () => {
  it('éxito: crea la alianza, la añade al estado y devuelve su id', () => {
    const { sesion, a, b } = partidaConDosFacciones();
    const resultado = sesion.ejecutar(proponerRelacion, { tipo: 'alianza', faccionAId: a, faccionBId: b }, OPC);

    expect(resultado.ok).toBe(true);
    expect(resultado.datos?.relacionId).toBeTruthy();
    expect(sesion.getState().relaciones).toHaveLength(1);
    expect(sesion.getState().relaciones[0]!.id).toBe(resultado.datos!.relacionId);
    expect(sesion.getState().relaciones[0]!.tipo).toBe('alianza');
  });

  it('éxito: un vasallaje guarda el tributo indicado', () => {
    const { sesion, a, b } = partidaConDosFacciones();
    const resultado = sesion.ejecutar(
      proponerRelacion,
      { tipo: 'vasallaje', faccionAId: a, faccionBId: b, tributoRecurso: 'madera', tributoCantidad: 5 },
      OPC
    );

    expect(resultado.ok).toBe(true);
    const relacion = sesion.getState().relaciones[0]!;
    expect(relacion.tipo).toBe('vasallaje');
    expect(relacion.tributo).toEqual({ recurso: 'madera', cantidadPorTick: 5 });
  });

  it('una relación creada se puede romper después, y el comando la marca como rota', () => {
    const { sesion, a, b } = partidaConDosFacciones();
    const creada = sesion.ejecutar(proponerRelacion, { tipo: 'alianza', faccionAId: a, faccionBId: b }, OPC);

    const rota = sesion.ejecutar(romperRelacion, { relacionId: creada.datos!.relacionId, iniciadorFaccionId: a }, OPC);

    expect(rota.ok).toBe(true);
    expect(sesion.getState().relaciones[0]!.estado).toBe('rota');
  });
});

describe('anexionar / fusionar', () => {
  it('anexionar rechaza una Facción consigo misma', () => {
    const { sesion, a } = partidaConDosFacciones();
    const resultado = sesion.ejecutar(anexionar, { faccionAId: a, faccionBId: a }, OPC);

    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe('fusion.invalida');
  });

  it('anexionar rechaza si alguna Facción no existe', () => {
    const { sesion, a } = partidaConDosFacciones();
    const resultado = sesion.ejecutar(anexionar, { faccionAId: a, faccionBId: 'no-existe' }, OPC);

    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe('fusion.invalida');
  });

  it('fusionar rechaza si alguna Facción no existe, sin mutar el estado', () => {
    const { sesion, a } = partidaConDosFacciones();
    const antes = sesion.getState();
    const resultado = sesion.ejecutar(fusionar, { faccionAId: a, faccionBId: 'no-existe', nuevoNombre: 'X', nuevoReyId: 'y' }, OPC);

    expect(resultado.ok).toBe(false);
    expect(sesion.getState()).toBe(antes);
  });

  it('una anexión con éxito limpia de faccionesNpcIds la Facción que desaparece', () => {
    // Se construye el estado con ambas Facciones cedidas al NPC y una relación de vasallaje ya activa, que es
    // lo que `anexionar` exige. Vía `importar()` porque no hay comando para fabricar la relación directamente.
    const { sesion, a, b } = partidaConDosFacciones();
    const payload = sesion.exportar();
    const conVasallaje = GameSession.importar({
      ...payload,
      state: {
        ...payload.state,
        faccionesNpcIds: [a, b],
        relaciones: [
          {
            id: 'rel-1',
            tipo: 'vasallaje',
            faccionAId: a,
            faccionBId: b,
            estado: 'activa',
            creadoEnTick: 0,
            tributo: { recurso: 'trigo', cantidadPorTick: 1 },
          },
        ],
      },
    });

    const resultado = conVasallaje.ejecutar(anexionar, { faccionAId: a, faccionBId: b }, OPC);

    expect(resultado.ok).toBe(true);
    const idsVivos = conVasallaje.getState().facciones.map((f) => f.id);
    // Ninguna id de faccionesNpcIds puede referirse a una Facción que ya no existe.
    for (const id of conVasallaje.getState().faccionesNpcIds) expect(idsVivos).toContain(id);
  });
});
