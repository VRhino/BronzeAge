// Formar un ejército en campo (Doc 5.14.4), de principio a fin con comandos: organizar, unirse, llegar a tres, fijar el
// destino una sola vez, separarse y deshacerse.
import { describe, expect, it } from 'vitest';
import { FORMACION_EJERCITO } from '../../constants';
import { REGISTRO_COMANDOS } from '../comandos/registro';
import { instanteDeTick } from '../estado';
import { abastecer, conHeroe, entraDeVecino, partidaConAsentamiento } from './fixtures';

/** Tres héroes de la misma Facción, cada uno en su columna personal junto a la puerta de la plaza. */
function tresSolitarios() {
  const base = partidaConAsentamiento();
  const { faccionId, asentamientoId, fundador, vecino } = base;
  entraDeVecino(base.sesion, faccionId, asentamientoId, fundador, 'tercero');
  let sesion = conHeroe(base.sesion, 'tercero');
  const heroes = [fundador, vecino, 'tercero'];
  for (const heroeId of heroes) {
    sesion = abastecer(sesion);
    const salida = sesion.ejecutar(REGISTRO_COMANDOS.salirAlMundo, { asentamientoId, heroeId, escuadronIds: [], carga: { trigo: 20 } }, { actor: heroeId });
    if (!salida.ok) throw new Error(`setup: ${heroeId} no sale (${salida.codigoError})`);
  }
  const columnaDe = (heroeId: string) => sesion.getState().ejercitos.find((e) => e.participantes.some((p) => p.heroeId === heroeId))!;
  const ejecutar = (tipo: keyof typeof REGISTRO_COMANDOS, heroeId: string, params: Record<string, unknown> = {}) =>
    sesion.ejecutar(REGISTRO_COMANDOS[tipo] as never, { heroeId, ...params } as never, { actor: heroeId });
  return { sesion, fundador, vecino, tercero: 'tercero', columnaDe, ejecutar };
}

describe('organizar un ejército en campo', () => {
  it('la columna se queda quieta como formación: sigue siendo personal, con su Líder y su plazo', () => {
    const { sesion, fundador, columnaDe, ejecutar } = tresSolitarios();

    const r = ejecutar('organizarEjercito', fundador, { politicaDeUnion: 'aceptar' });

    expect(r.ok).toBe(true);
    const formacion = columnaDe(fundador);
    expect(formacion.tipo, 'para el combate sigue siendo una columna personal').toBe('personal');
    expect(formacion.estado).toBe('estacionado');
    expect(formacion.liderId).toBe(fundador);
    expect(formacion.politicaDeUnion).toBe('aceptar');
    expect(formacion.formacion!.expiraEn).toBeGreaterThan(instanteDeTick(sesion.getState().tick));
  });

  it('no se organiza dos veces, y quien está en una formación ya no organiza otra', () => {
    const { fundador, vecino, columnaDe, ejecutar } = tresSolitarios();
    ejecutar('organizarEjercito', fundador, { politicaDeUnion: 'aceptar' });

    expect(ejecutar('organizarEjercito', fundador, { politicaDeUnion: 'aceptar' }).ok).toBe(false);
    ejecutar('unirseEnCampo', vecino, { ejercitoId: columnaDe(fundador).id });
    expect(ejecutar('organizarEjercito', vecino, { politicaDeUnion: 'aceptar' }).ok, 'ya va en la formación').toBe(false);
  });
});

describe('unirse y llegar a tres', () => {
  it('con dos sigue siendo una formación; con tres pasa a ser un ejército que espera su destino', () => {
    const { fundador, vecino, tercero, columnaDe, ejecutar } = tresSolitarios();
    ejecutar('organizarEjercito', fundador, { politicaDeUnion: 'aceptar' });
    const formacionId = columnaDe(fundador).id;

    expect(ejecutar('unirseEnCampo', vecino, { ejercitoId: formacionId }).ok).toBe(true);
    expect(columnaDe(fundador).tipo).toBe('personal');
    expect(columnaDe(fundador).formacion).toBeDefined();
    expect(columnaDe(fundador).participantes).toHaveLength(2);

    expect(ejecutar('unirseEnCampo', tercero, { ejercitoId: formacionId }).ok).toBe(true);
    const ejercito = columnaDe(fundador);
    expect(ejercito.tipo).toBe('ejercito');
    expect(ejercito.formacion).toBeUndefined();
    expect(ejercito.destinoPendiente).toBe(true);
    expect(ejercito.participantes).toHaveLength(FORMACION_EJERCITO.minimo);
    expect(ejercito.origenAsentamientoId, 'el origen es la residencia del Líder').toBeDefined();
  });

  it('el Líder fija el destino una sola vez; otro integrante no, y después ya no se toca', () => {
    const { fundador, vecino, tercero, columnaDe, ejecutar } = tresSolitarios();
    ejecutar('organizarEjercito', fundador, { politicaDeUnion: 'aceptar' });
    const formacionId = columnaDe(fundador).id;
    ejecutar('unirseEnCampo', vecino, { ejercitoId: formacionId });
    ejecutar('unirseEnCampo', tercero, { ejercitoId: formacionId });
    const destino = { tipo: 'punto', punto: { x: columnaDe(fundador).posicionActual.x + 100, y: columnaDe(fundador).posicionActual.y } };

    expect(ejecutar('marcharA', vecino, { objetivo: destino }).ok, 'solo el Líder').toBe(false);
    expect(ejecutar('marcharA', fundador, { objetivo: destino }).ok).toBe(true);
    expect(columnaDe(fundador).destinoPendiente).toBeUndefined();
    expect(columnaDe(fundador).estado).toBe('marchando');
    expect(ejecutar('marcharA', fundador, { objetivo: { tipo: 'punto', punto: { x: 5, y: 5 } } }).ok, 'ya es un ejército: no se rectifica').toBe(false);
  });

  it('una formación no se mueve', () => {
    const { fundador, ejecutar } = tresSolitarios();
    ejecutar('organizarEjercito', fundador, { politicaDeUnion: 'aceptar' });

    expect(ejecutar('marcharA', fundador, { objetivo: { tipo: 'punto', punto: { x: 5, y: 5 } } }).ok).toBe(false);
  });
});

describe('separarse y deshacerse', () => {
  it('un integrante se separa de la formación; si se va el Líder, el mando pasa al que lleva más tiempo', () => {
    const { fundador, vecino, tercero, columnaDe, ejecutar } = tresSolitarios();
    ejecutar('organizarEjercito', fundador, { politicaDeUnion: 'aceptar' });
    const formacionId = columnaDe(fundador).id;
    ejecutar('unirseEnCampo', vecino, { ejercitoId: formacionId });

    expect(ejecutar('separarseDelEjercito', fundador).ok, 'el Líder se va de una formación').toBe(true);

    expect(columnaDe(fundador).id).not.toBe(formacionId);
    expect(columnaDe(fundador).tipo).toBe('personal');
    expect(columnaDe(vecino).id).toBe(formacionId);
    expect(columnaDe(vecino).liderId, 'el mando pasa a quien queda').toBe(vecino);
    expect(columnaDe(tercero).formacion, 'el tercero ni se entera').toBeUndefined();
  });

  it('el Líder cancela la formación y cada uno vuelve a su columna personal; los demás no pueden cancelarla', () => {
    const { fundador, vecino, columnaDe, ejecutar } = tresSolitarios();
    ejecutar('organizarEjercito', fundador, { politicaDeUnion: 'aceptar' });
    const formacionId = columnaDe(fundador).id;
    ejecutar('unirseEnCampo', vecino, { ejercitoId: formacionId });

    expect(ejecutar('cancelarFormacion', vecino).ok).toBe(false);
    expect(ejecutar('cancelarFormacion', fundador).ok).toBe(true);

    expect(columnaDe(fundador).formacion).toBeUndefined();
    expect(columnaDe(fundador).participantes).toHaveLength(1);
    expect(columnaDe(vecino).id).not.toBe(columnaDe(fundador).id);
    expect(columnaDe(vecino).tipo).toBe('personal');
  });

  it('si no llega a tres en el plazo, se deshace sola en el tick y se avisa', () => {
    const { sesion, fundador, vecino, columnaDe, ejecutar } = tresSolitarios();
    ejecutar('organizarEjercito', fundador, { politicaDeUnion: 'aceptar' });
    ejecutar('unirseEnCampo', vecino, { ejercitoId: columnaDe(fundador).id });

    for (let i = 0; i < FORMACION_EJERCITO.plazoMinutos; i++) sesion.avanzarTick();

    expect(columnaDe(fundador).formacion).toBeUndefined();
    expect(columnaDe(vecino).id).not.toBe(columnaDe(fundador).id);
    expect(columnaDe(vecino).liderId).toBe(vecino);
    const heroeVecino = sesion.getState().heroes.find((h) => h.id === vecino)!;
    expect(heroeVecino.ubicacion).toEqual({ tipo: 'columna', ejercitoId: columnaDe(vecino).id });
  });
});

describe('quién se une', () => {
  it('solo columnas personales de la misma Facción: un ejército que ya existe no se funde con una formación', () => {
    const { fundador, vecino, tercero, columnaDe, ejecutar } = tresSolitarios();
    ejecutar('organizarEjercito', fundador, { politicaDeUnion: 'aceptar' });
    ejecutar('organizarEjercito', vecino, { politicaDeUnion: 'aceptar' });

    expect(ejecutar('unirseEnCampo', vecino, { ejercitoId: columnaDe(fundador).id }).ok, 'una formación no se une a otra').toBe(false);
    expect(ejecutar('unirseEnCampo', tercero, { ejercitoId: columnaDe(fundador).id }).ok).toBe(true);
  });

  it('una columna personal sin formación no admite a nadie', () => {
    const { fundador, vecino, columnaDe, ejecutar } = tresSolitarios();

    expect(ejecutar('unirseEnCampo', vecino, { ejercitoId: columnaDe(fundador).id }).ok).toBe(false);
  });
});
