// Grupo de comandos de diplomacia (`session/comandos/diplomacia.ts`). Verifica el contrato de la capa de
// partida, no las reglas del motor (requisitos de nivel, reputación, etc., ya cubiertos en `engine/`).
//
// Los rechazos por RELACIÓN/FACCIÓN INEXISTENTE (en `GameStore`, `romperRelacion` no tenía try/catch y el
// `DiplomaciaInvalidaError` del motor llegaba crudo a la interfaz) están unificados en
// `comandosContratoIds.test.ts`, no repetidos aquí.
import { describe, expect, it } from 'vitest';
import { GameSession } from '../gameSession';
import { crearFaccion } from '../comandos/crearFaccion';
import { anexionar, declararGuerra, proponerPaz, proponerRelacion, rebelionVasallo, romperRelacion } from '../comandos/diplomacia';

const OPC = { actor: 'jugador-test' };

function partidaConDosFacciones() {
  const sesion = GameSession.crear('diplo-test', { seed: 42 });
  // Dos actores distintos: un jugador solo puede crear una Facción (Doc 2 "Entidades") — el mismo actor para
  // las dos habría rechazado la segunda con `faccion.ya_pertenece`.
  const a = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, { ...OPC, actor: 'jugador-a' }).datos!.faccionId;
  const b = sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, { ...OPC, actor: 'jugador-b' }).datos!.faccionId;
  return { sesion, a, b };
}

describe('romperRelacion', () => {
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
    expect(relacion.tributo).toEqual({ recurso: 'madera', cantidadPorMinuto: 5 });
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

});

describe('declararGuerra y proponerPaz (Doc 2.4.1)', () => {
  it('declarar crea la guerra y arrastra al señor del objetivo y a sus vasallos', () => {
    const sesion = GameSession.crear('guerra-test', { seed: 42 });
    const [a, b, c, d] = ['a', 'b', 'c', 'd'].map(
      (n) => sesion.ejecutar(crearFaccion, { nombre: n }, { actor: `j-${n}` }).datos!.faccionId
    );
    // b es vasallo de c; c tiene otro vasallo, d.
    sesion.ejecutar(proponerRelacion, { tipo: 'vasallaje', faccionAId: c!, faccionBId: b! }, OPC);
    sesion.ejecutar(proponerRelacion, { tipo: 'vasallaje', faccionAId: c!, faccionBId: d! }, OPC);

    const r = sesion.ejecutar(declararGuerra, { faccionAId: a!, faccionBId: b! }, OPC);

    expect(r.ok).toBe(true);
    const guerras = sesion.getState().relaciones.filter((x) => x.tipo === 'guerra');
    expect(guerras.map((g) => g.faccionBId).sort()).toEqual([b, c, d].sort());
  });

  it('rechazo: no se declara guerra a un aliado', () => {
    const { sesion, a, b } = partidaConDosFacciones();
    sesion.ejecutar(proponerRelacion, { tipo: 'alianza', faccionAId: a, faccionBId: b }, OPC);
    const antes = sesion.getState();
    expect(sesion.ejecutar(declararGuerra, { faccionAId: a, faccionBId: b }, OPC).ok).toBe(false);
    expect(sesion.getState()).toBe(antes);
  });

  it('la paz es mutua: la primera oferta no la acaba, la de la otra Facción sí', () => {
    const { sesion, a, b } = partidaConDosFacciones();
    const [guerraId] = sesion.ejecutar(declararGuerra, { faccionAId: a, faccionBId: b }, OPC).datos!.relacionIds;

    expect(sesion.ejecutar(proponerPaz, { relacionId: guerraId!, faccionId: a }, OPC).datos?.firmada).toBe(false);
    expect(sesion.ejecutar(proponerPaz, { relacionId: guerraId!, faccionId: a }, OPC).ok).toBe(false);
    expect(sesion.getState().relaciones[0]!.estado).toBe('activa');

    expect(sesion.ejecutar(proponerPaz, { relacionId: guerraId!, faccionId: b }, OPC).datos?.firmada).toBe(true);
    expect(sesion.getState().relaciones[0]!.estado).toBe('rota');
  });

  it('una guerra no se rompe con romperRelacion', () => {
    const { sesion, a, b } = partidaConDosFacciones();
    const [guerraId] = sesion.ejecutar(declararGuerra, { faccionAId: a, faccionBId: b }, OPC).datos!.relacionIds;
    expect(sesion.ejecutar(romperRelacion, { relacionId: guerraId!, iniciadorFaccionId: a }, OPC).ok).toBe(false);
  });

  it('la rebelión del vasallo deja a ambos en guerra', () => {
    const { sesion, a, b } = partidaConDosFacciones();
    const vasallajeId = sesion.ejecutar(proponerRelacion, { tipo: 'vasallaje', faccionAId: a, faccionBId: b }, OPC).datos!.relacionId;
    sesion.ejecutar(rebelionVasallo, { relacionId: vasallajeId }, OPC);
    const guerra = sesion.getState().relaciones.find((x) => x.tipo === 'guerra');
    expect(guerra).toMatchObject({ faccionAId: b, faccionBId: a, estado: 'activa' });
  });
});
