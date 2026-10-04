// Estar dentro de un campamento de mercenarios (D74-D77): nacer dentro, salir eligiendo tropa y carga, volver y entrar en otro.
import { describe, expect, it } from 'vitest';
import type { Heroe } from '../../domain/types';
import { ALMACEN_PERSONAL, MERCENARIOS } from '../../constants';
import { guardarEnAlmacenPersonal } from '../comandos/heroe';
import { GameSession } from '../gameSession';
import { crearHeroe } from '../comandos/crearHeroe';
import { entrarEnCampamento, salirDelCampamento } from '../comandos/presencia';
import { atacar } from '../comandos/interaccion';
import { escuadronDePrueba } from '../../engine/__tests__/fixtures';

const PARAMS = { displayName: 'Ana', classDefinitionId: 'Spear', genero: 'femenino' as const, avatar: { cabezaId: '', peloId: '', barbaId: '', cejasId: '' } };

/** Héroe nacido en `mercenarios-0`, retocado (almacén, tropa) vía `importar`. */
function nacido(cambios: (h: Heroe) => Partial<Heroe> = () => ({})) {
  const creada = GameSession.crear('campamento', { seed: 42 });
  const heroeId = creada.ejecutar(crearHeroe, { ...PARAMS, campamentoId: 'mercenarios-0' }, { actor: 'jugador-1' }).datos!.heroeId;
  const p = creada.exportar();
  const sesion = GameSession.importar({ ...p, state: { ...p.state, heroes: p.state.heroes.map((h) => (h.id === heroeId ? { ...h, ...cambios(h) } : h)) } });
  const heroe = () => sesion.getState().heroes.find((h) => h.id === heroeId)!;
  const columna = () => sesion.getState().ejercitos.find((e) => e.liderId === heroeId);
  return { sesion, heroeId, heroe, columna, opc: { actor: heroeId } };
}

const campamento = (sesion: GameSession, id: string) => sesion.getState().campamentosMercenarios.find((c) => c.id === id)!;

describe('su campamento de residencia', () => {
  it('sale con la tropa y la carga que elige, y al volver la columna se deshace: tropa al campamento, carro al almacén', () => {
    const { sesion, heroeId, heroe, columna, opc } = nacido((h) => ({
      almacenPersonal: { trigo: 100, oro: 20 },
      escuadrones: [{ ...escuadronDePrueba('esc-1', h.id, 'lenadores'), contenedor: { tipo: 'campamento' } }],
    }));

    const salida = sesion.ejecutar(salirDelCampamento, { campamentoId: 'mercenarios-0', heroeId, escuadronIds: ['esc-1'], carga: { trigo: 60 } }, opc);
    expect(salida.ok).toBe(true);
    // Lo cargado (60) más la ración gratis del residente (D24).
    expect(columna()).toMatchObject({ suministro: { trigo: 60 + MERCENARIOS.racion.trigo }, racion: MERCENARIOS.racion.trigo, escuadronIds: ['esc-1'], posicionActual: campamento(sesion, 'mercenarios-0').posicion });
    expect(heroe().ubicacion).toEqual({ tipo: 'columna', ejercitoId: columna()!.id });
    expect(heroe().almacenPersonal).toEqual({ trigo: 40, oro: 20 });

    expect(sesion.ejecutar(entrarEnCampamento, { campamentoId: 'mercenarios-0', heroeId }, opc).ok).toBe(true);
    expect(columna()).toBeUndefined();
    expect(heroe().ubicacion).toEqual({ tipo: 'mercenarios', campamentoId: 'mercenarios-0' });
    // Lo suyo vuelve al almacén; la ración, al campamento (D50).
    expect(heroe().almacenPersonal).toEqual({ trigo: 100, oro: 20 });
    expect(campamento(sesion, 'mercenarios-0').mercado['trigo']).toBe(MERCENARIOS.mercado.pilas['trigo']! + MERCENARIOS.racion.trigo);
    expect(heroe().escuadrones[0]!.contenedor).toEqual({ tipo: 'campamento' });
  });

  it('lo que no cabe en el almacén personal se queda en el carro, aparcado en la puerta, y sale con él la próxima vez', () => {
    const { sesion, heroeId, opc } = nacido(() => ({ almacenPersonal: { piedra: ALMACEN_PERSONAL.capacidad - 50, trigo: 100 } }));
    sesion.ejecutar(salirDelCampamento, { campamentoId: 'mercenarios-0', heroeId, escuadronIds: [], carga: { trigo: 100 } }, opc);
    // Se llena el almacén mientras está fuera: al volver solo caben 50 del carro.
    const p = sesion.exportar();
    const lleno = GameSession.importar({
      ...p,
      state: { ...p.state, heroes: p.state.heroes.map((h) => (h.id === heroeId ? { ...h, almacenPersonal: { piedra: ALMACEN_PERSONAL.capacidad - 50 } } : h)) },
    });
    expect(lleno.ejecutar(entrarEnCampamento, { campamentoId: 'mercenarios-0', heroeId }, opc).ok).toBe(true);
    const aparcada = lleno.getState().ejercitos.find((e) => e.liderId === heroeId)!;
    expect(aparcada.suministro).toEqual({ trigo: 50 });
    expect(lleno.getState().heroes.find((h) => h.id === heroeId)!.ubicacion.tipo).toBe('mercenarios');

    const r = lleno.ejecutar(salirDelCampamento, { campamentoId: 'mercenarios-0', heroeId, escuadronIds: [], carga: {} }, opc);
    expect(r.datos!.ejercitoId).toBe(aparcada.id);
  });

  it('rechaza cargar lo que no hay en el almacén, y salir sin estar dentro', () => {
    const { sesion, heroeId, opc } = nacido(() => ({ almacenPersonal: { trigo: 10 } }));
    expect(sesion.ejecutar(salirDelCampamento, { campamentoId: 'mercenarios-0', heroeId, escuadronIds: [], carga: { trigo: 11 } }, opc).ok).toBe(false);
    expect(sesion.ejecutar(salirDelCampamento, { campamentoId: 'mercenarios-1', heroeId, escuadronIds: [], carga: {} }, opc).ok).toBe(false);
  });
});

describe('otro campamento (enclave neutral, D77)', () => {
  it('entra con su columna, que queda en la puerta intacta, y al salir la retoma tal cual', () => {
    const { sesion, heroeId, opc } = nacido(() => ({ almacenPersonal: { trigo: 30 } }));
    sesion.ejecutar(salirDelCampamento, { campamentoId: 'mercenarios-0', heroeId, escuadronIds: [], carga: { trigo: 30 } }, opc);
    // Lleva la columna a la puerta de otro campamento (caminar costaría ticks que este test no mide).
    const p = sesion.exportar();
    const otro = p.state.campamentosMercenarios[1]!;
    const alli = GameSession.importar({
      ...p,
      state: { ...p.state, ejercitos: p.state.ejercitos.map((e) => (e.liderId === heroeId ? { ...e, posicionActual: otro.posicion } : e)) },
    });

    expect(alli.ejecutar(entrarEnCampamento, { campamentoId: otro.id, heroeId }, opc).ok).toBe(true);
    const columna = alli.getState().ejercitos.find((e) => e.liderId === heroeId)!;
    expect(columna.suministro).toEqual({ trigo: 30 + MERCENARIOS.racion.trigo });
    expect(alli.getState().heroes.find((h) => h.id === heroeId)!.ubicacion).toEqual({ tipo: 'mercenarios', campamentoId: otro.id });

    expect(alli.ejecutar(salirDelCampamento, { campamentoId: otro.id, heroeId, escuadronIds: [], carga: {} }, opc).datos!.ejercitoId).toBe(columna.id);
  });

  it('lejos de la puerta no se entra', () => {
    const { sesion, heroeId, opc } = nacido();
    sesion.ejecutar(salirDelCampamento, { campamentoId: 'mercenarios-0', heroeId, escuadronIds: [], carga: {} }, opc);
    expect(sesion.ejecutar(entrarEnCampamento, { campamentoId: 'mercenarios-1', heroeId }, opc).ok).toBe(false);
  });
});

describe('protección del campamento (M4/D78)', () => {
  it('nadie inicia un combate junto a él', () => {
    const { sesion, heroeId, columna, opc } = nacido();
    sesion.ejecutar(crearHeroe, { ...PARAMS, displayName: 'Bea', campamentoId: 'mercenarios-0' }, { actor: 'jugador-2' });
    const otro = sesion.getState().heroes.find((h) => h.jugadorId === 'jugador-2')!.id;
    sesion.ejecutar(salirDelCampamento, { campamentoId: 'mercenarios-0', heroeId, escuadronIds: [], carga: {} }, opc);
    sesion.ejecutar(salirDelCampamento, { campamentoId: 'mercenarios-0', heroeId: otro, escuadronIds: [], carga: {} }, { actor: otro });
    const suya = sesion.getState().ejercitos.find((e) => e.liderId === otro)!;

    const r = sesion.ejecutar(atacar, { heroeId, objetivo: { tipo: 'ejercito', id: suya.id } }, opc);
    expect(r.codigoError).toBe('campamento.proteccion');
    expect(columna()).toBeDefined();
  });
});

describe('la ración gratis del residente (D24, D50)', () => {
  it('se da al salir una vez por plazo, se come la primera y no se puede guardar', () => {
    const { sesion, heroeId, columna, opc } = nacido();
    sesion.ejecutar(salirDelCampamento, { campamentoId: 'mercenarios-0', heroeId, escuadronIds: [], carga: {} }, opc);
    expect(columna()!.suministro).toEqual({ trigo: MERCENARIOS.racion.trigo });
    expect(sesion.ejecutar(guardarEnAlmacenPersonal, { recurso: 'trigo', cantidad: 10 }, opc).ok, 'la ración no se guarda').toBe(false);

    // Vuelve y sale enseguida: dentro del plazo no hay otra.
    sesion.ejecutar(entrarEnCampamento, { campamentoId: 'mercenarios-0', heroeId }, opc);
    sesion.ejecutar(salirDelCampamento, { campamentoId: 'mercenarios-0', heroeId, escuadronIds: [], carga: {} }, opc);
    expect(columna()!.suministro['trigo'] ?? 0).toBe(0);
  });
});
