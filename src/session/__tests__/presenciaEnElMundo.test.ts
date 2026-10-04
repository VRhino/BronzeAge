// Desconectarse y volver (Doc 1.10.6, D33/D40/D40b): el héroe sale del mundo 2:30 después de pedirlo, con su tropa y
// su carro, y reaparece donde quedó. La guarnición y la escolta se quedan; un ejército sigue sin él; las caravanas que
// iban con el último vuelven solas a su origen.
import { describe, expect, it } from 'vitest';
import { GameSession } from '../gameSession';
import { conectarse, desconectarse, salirAlMundo } from '../comandos/presencia';
import { movilizarEjercito, unirseAEjercito } from '../comandos/ejercitos';
import { verificarAutorizacion } from '../comandos/autorizacion';
import { OPC, partidaConAsentamiento } from './fixtures';
import { conEscuadrones } from '../../engine/tropa';
import { escuadronDePrueba } from '../../engine/__tests__/fixtures';
import type { Caravana } from '../../domain/types';

const opcDe = (heroeId: string) => ({ ...OPC, actor: heroeId });
const PUNTO_LEJOS = { tipo: 'punto', punto: { x: 900, y: 900 } } as const;

/** La plaza del fixture con trigo de sobra, una escuadra libre y otra en la guarnición del fundador, y una del vecino. */
function partida() {
  const base = partidaConAsentamiento();
  const payload = base.sesion.exportar();
  const a = payload.state.asentamientos[0]!;
  const sesion = GameSession.importar({
    ...payload,
    state: {
      ...payload.state,
      asentamientos: [{ ...a, almacen: { ...a.almacen, trigo: { capacidad: 100000, cantidad: 5000 } } }],
      heroes: conEscuadrones(payload.state.heroes, [
        escuadronDePrueba('esc-libre', base.fundador),
        escuadronDePrueba('esc-guardia', base.fundador, 'milicia_lanceros', 10, { enGuarnicion: true }),
        escuadronDePrueba('esc-vecino', base.vecino),
      ]),
    },
  });
  return { ...base, sesion };
}

const heroe = (s: GameSession, id: string) => s.getState().heroes.find((h) => h.id === id)!;
const escuadra = (s: GameSession, id: string) => s.getState().heroes.flatMap((h) => h.escuadrones).find((e) => e.id === id)!;
const ticks = (s: GameSession, n: number) => {
  for (let i = 0; i < n; i++) s.avanzarTick();
};

describe('desconectarse desde dentro de la plaza', () => {
  it('sigue dentro 2:30 y luego sale con sus escuadras libres; la guarnición se queda (D40b)', () => {
    const { sesion, fundador } = partida();
    expect(sesion.ejecutar(desconectarse, { heroeId: fundador }, opcDe(fundador)).ok).toBe(true);

    ticks(sesion, 2);
    expect(heroe(sesion, fundador).fuera, 'a los 2 minutos todavía está').toBeUndefined();
    ticks(sesion, 1);

    expect(heroe(sesion, fundador).ubicacion.tipo).toBe('desconectado');
    expect(heroe(sesion, fundador).fuera?.asentamientoId).toBe(sesion.getState().asentamientos[0]!.id);
    expect(escuadra(sesion, 'esc-libre').contenedor).toEqual({ tipo: 'fuera' });
    expect(escuadra(sesion, 'esc-guardia').contenedor, 'la guarnición sigue en la plaza').toEqual({ tipo: 'campamento' });
  });

  it('fuera no puede dar órdenes; al volver aparece dentro con sus escuadras en el campamento', () => {
    const { sesion, fundador, asentamientoId } = partida();
    sesion.ejecutar(desconectarse, { heroeId: fundador }, opcDe(fundador));
    ticks(sesion, 3);

    const orden = verificarAutorizacion('salirAlMundo', { asentamientoId, heroeId: fundador, escuadronIds: [], carga: {} }, sesion.getState(), { rol: 'jugador', heroeId: fundador });
    expect(orden).toEqual({ autorizado: false, motivo: 'fuera_del_mundo' });

    expect(sesion.ejecutar(conectarse, { heroeId: fundador }, opcDe(fundador)).ok).toBe(true);
    expect(heroe(sesion, fundador).ubicacion).toEqual({ tipo: 'asentamiento', asentamientoId });
    expect(heroe(sesion, fundador).fuera).toBeUndefined();
    expect(escuadra(sesion, 'esc-libre').contenedor).toEqual({ tipo: 'campamento' });
  });

  it('volver antes de que pasen 2:30 cancela la salida', () => {
    const { sesion, fundador } = partida();
    sesion.ejecutar(desconectarse, { heroeId: fundador }, opcDe(fundador));
    ticks(sesion, 1);
    sesion.ejecutar(conectarse, { heroeId: fundador }, opcDe(fundador));
    ticks(sesion, 5);

    expect(heroe(sesion, fundador).fuera).toBeUndefined();
    expect(heroe(sesion, fundador).desconectaEn).toBeUndefined();
  });
});

describe('desconectarse en el campo', () => {
  it('su columna deja de existir y se lleva tropa y carro; al volver reaparece en su punto con ellos', () => {
    const { sesion, fundador, asentamientoId } = partida();
    sesion.ejecutar(salirAlMundo, { asentamientoId, heroeId: fundador, escuadronIds: ['esc-libre'], carga: { trigo: 60 } }, opcDe(fundador));
    const columna = sesion.getState().ejercitos.find((e) => e.liderId === fundador)!;
    sesion.ejecutar(desconectarse, { heroeId: fundador }, opcDe(fundador));
    ticks(sesion, 3);

    expect(sesion.getState().ejercitos.some((e) => e.id === columna.id)).toBe(false);
    const fuera = heroe(sesion, fundador);
    expect(fuera.fuera!.carro['trigo']).toBeGreaterThan(0);
    expect(escuadra(sesion, 'esc-libre').contenedor).toEqual({ tipo: 'fuera' });

    sesion.ejecutar(conectarse, { heroeId: fundador }, opcDe(fundador));
    const nueva = sesion.getState().ejercitos.find((e) => e.liderId === fundador)!;
    expect(nueva.posicionActual).toEqual(fuera.ubicacion.tipo === 'desconectado' ? fuera.ubicacion.punto : undefined);
    expect(nueva.escuadronIds).toEqual(['esc-libre']);
    expect(nueva.suministro['trigo']).toBe(fuera.fuera!.carro['trigo']);
    expect(nueva.origenAsentamientoId, 'vuelve a tener casa a la que replegarse').toBe(asentamientoId);
  });

  it('si el Líder de un ejército se va, el mando pasa al de más antigüedad y el ejército sigue sin él', () => {
    const { sesion, fundador, vecino, asentamientoId } = partida();
    sesion.ejecutar(movilizarEjercito, { asentamientoId, heroeId: fundador, escuadronIds: ['esc-libre'], objetivo: PUNTO_LEJOS, politicaDeUnion: 'aceptar' }, opcDe(fundador));
    const ejercitoId = sesion.getState().ejercitos[0]!.id;
    expect(sesion.ejecutar(unirseAEjercito, { ejercitoId, asentamientoId, heroeId: vecino, escuadronIds: ['esc-vecino'] }, opcDe(vecino)).ok).toBe(true);

    sesion.ejecutar(desconectarse, { heroeId: fundador }, opcDe(fundador));
    ticks(sesion, 3);

    const ejercito = sesion.getState().ejercitos.find((e) => e.id === ejercitoId)!;
    expect(ejercito.liderId).toBe(vecino);
    expect(ejercito.participantes.map((p) => p.heroeId)).toEqual([vecino]);
    expect(ejercito.escuadronIds).toEqual(['esc-vecino']);
    expect(escuadra(sesion, 'esc-libre').contenedor).toEqual({ tipo: 'fuera' });
  });

  it('si se va el último, las caravanas adjuntas vuelven solas a su origen (D40)', () => {
    const { sesion: base, fundador, asentamientoId } = partida();
    base.ejecutar(movilizarEjercito, { asentamientoId, heroeId: fundador, escuadronIds: ['esc-libre'], objetivo: PUNTO_LEJOS }, opcDe(fundador));
    const payload = base.exportar();
    const ejercito = payload.state.ejercitos[0]!;
    const caravana: Caravana = {
      id: 'caravana-adjunta',
      tipo: 'comercial',
      origenAsentamientoId: asentamientoId,
      contenido: { madera: 30 },
      estado: 'adjunta',
      progreso: 0,
      posicionActual: ejercito.posicionActual,
    } as Caravana;
    const sesion = GameSession.importar({
      ...payload,
      state: { ...payload.state, caravanas: [caravana], ejercitos: [{ ...ejercito, caravanasAdjuntasIds: [caravana.id] }] },
    });

    sesion.ejecutar(desconectarse, { heroeId: fundador }, opcDe(fundador));
    ticks(sesion, 3);

    expect(sesion.getState().ejercitos).toHaveLength(0);
    expect(sesion.getState().caravanas.find((c) => c.id === caravana.id)?.estado).toBe('retornando');
  });
});
