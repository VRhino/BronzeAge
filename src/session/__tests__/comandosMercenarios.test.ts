// Campamentos de mercenarios, paso 2 (Doc 2.5, 2026-10-02): residir en uno, no quedar huérfano, y el almacén personal.
import { describe, expect, it } from 'vitest';
import type { CampamentoMercenarios } from '../../domain/types';
import { CIUDADANIA, SIMULACION } from '../../constants';
import { GameSession } from '../gameSession';
import { comprarCasa, dejarResidencia, residirEnCampamento } from '../comandos/cargos';
import { guardarEnAlmacenPersonal, sacarDelAlmacenPersonal } from '../comandos/heroe';
import { comprarEnCampamento, reclutarEnCampamento } from '../comandos/mercenarios';
import { exito } from '../comandos/tipos';
import { aportarARefundacion, comprarCaravanaDeRefundacion } from '../comandos/mercenarios';
import { costoRefundacion } from '../../engine/refundacion';
import { salirAlMundo } from '../comandos/presencia';
import { partidaConAsentamiento } from './fixtures';

const TICKS_POR_DIA = (24 * 60 * 60 * 1000) / SIMULACION.duracionTickMs;

const campamentoEn = (id: string, x: number, y: number): CampamentoMercenarios =>
  ({
    id,
    posicion: { x, y },
    origen: 0,
    edificios: ['taberna', 'vivienda', 'vivienda', 'mercado', 'barracon'],
    residentesIds: [],
    poblacion: 100,
    poblacionEn: 0,
    mercado: { madera: 100 },
    fondos: {},
    creadoEn: 0,
  }) as unknown as CampamentoMercenarios;

/** La partida de la fixture con dos campamentos puestos a mano, y el fundador actuando con su propio id. */
function conCampamentos() {
  const base = partidaConAsentamiento();
  const payload = base.sesion.exportar();
  const sesion = GameSession.importar({
    ...payload,
    state: { ...payload.state, campamentosMercenarios: [campamentoEn('merc-1', 900, 900), campamentoEn('merc-2', 1800, 1800)] },
  });
  return { ...base, sesion, opc: { actor: base.fundador } };
}

const adelantarDias = (sesion: GameSession, dias: number): GameSession => {
  const p = sesion.exportar();
  return GameSession.importar({ ...p, state: { ...p.state, tick: p.state.tick + dias * TICKS_POR_DIA } });
};
const campamentoDe = (sesion: GameSession, id: string) => sesion.getState().campamentosMercenarios.find((c) => c.id === id)!;
const casaDe = (sesion: GameSession, asentamientoId: string) => sesion.getState().asentamientos.find((a) => a.id === asentamientoId)!;

describe('residirEnCampamento', () => {
  it('deja la casa y los cargos locales y pasa a residir en el campamento, siendo de una Facción que tiene plazas', () => {
    const { sesion, asentamientoId, fundador, opc } = conCampamentos();

    const r = sesion.ejecutar(residirEnCampamento, { heroeId: fundador, campamentoId: 'merc-1' }, opc);

    expect(r.ok).toBe(true);
    expect(campamentoDe(sesion, 'merc-1').residentesIds).toEqual([fundador]);
    expect(casaDe(sesion, asentamientoId).heroesFundadoresIds).not.toContain(fundador);
  });

  it('rechazo: un campamento que no existe, y no versiona', () => {
    const { sesion, fundador, opc } = conCampamentos();
    const antes = sesion.getState();

    const r = sesion.ejecutar(residirEnCampamento, { heroeId: fundador, campamentoId: 'nada' }, opc);

    expect(r.ok).toBe(false);
    expect(r.codigoError).toBe('mercenarios.invalido');
    expect(sesion.getState()).toBe(antes);
  });

  it('cuenta para el cooldown de residencia: no se compra casa enseguida, sí pasado el plazo, y al comprarla deja el campamento', () => {
    const { sesion, asentamientoId, fundador, opc } = conCampamentos();
    sesion.ejecutar(residirEnCampamento, { heroeId: fundador, campamentoId: 'merc-1' }, opc);

    expect(sesion.ejecutar(comprarCasa, { asentamientoId, heroeId: fundador }, opc).ok).toBe(false);

    const despues = adelantarDias(sesion, CIUDADANIA.cooldownCambioResidenciaDias);
    expect(despues.ejecutar(comprarCasa, { asentamientoId, heroeId: fundador }, opc).ok).toBe(true);
    expect(campamentoDe(despues, 'merc-1').residentesIds).toEqual([]);
  });
});

describe('se acaba el huérfano', () => {
  it('quien deja su casa pasa, en el acto, al campamento más cercano a donde está', () => {
    const { sesion, fundador, opc } = conCampamentos();
    sesion.ejecutar(dejarResidencia, { heroeId: fundador }, opc);

    const donde = sesion.getState().campamentosMercenarios.find((c) => c.residentesIds.includes(fundador));
    expect(donde, 'reside en algún campamento').toBeDefined();
    // La fixture lo deja en el centro de su plaza (400,400): el más cercano es el primero.
    expect(donde!.id).toBe('merc-1');
  });
});

describe('almacén personal', () => {
  /** El fundador sale al mundo con su columna y con algo en el carro. */
  function conCarro() {
    const base = conCampamentos();
    base.sesion.ejecutar(salirAlMundo, { asentamientoId: base.asentamientoId, heroeId: base.fundador, escuadronIds: [], carga: {} }, base.opc);
    const payload = base.sesion.exportar();
    const columna = payload.state.ejercitos.find((e) => e.participantes.some((p) => p.heroeId === base.fundador))!;
    const sesion = GameSession.importar({
      ...payload,
      state: {
        ...payload.state,
        ejercitos: payload.state.ejercitos.map((e) => (e.id === columna.id ? { ...e, suministro: { madera: 100 } } : e)),
      },
    });
    return { ...base, sesion, columnaId: columna.id };
  }
  const heroeDe = (sesion: GameSession, id: string) => sesion.getState().heroes.find((h) => h.id === id)!;

  it('guarda del carro y saca de vuelta', () => {
    const { sesion, fundador, columnaId, opc } = conCarro();

    const g = sesion.ejecutar(guardarEnAlmacenPersonal, { recurso: 'madera', cantidad: 60 }, opc);
    expect(g.datos?.movido).toBe(60);
    expect(heroeDe(sesion, fundador).almacenPersonal).toEqual({ madera: 60 });
    expect(sesion.getState().ejercitos.find((e) => e.id === columnaId)!.suministro['madera']).toBe(40);

    const s = sesion.ejecutar(sacarDelAlmacenPersonal, { recurso: 'madera', cantidad: 25 }, opc);
    expect(s.datos?.movido).toBe(25);
    expect(heroeDe(sesion, fundador).almacenPersonal).toEqual({ madera: 35 });
  });

  it('rechazo: nada que guardar, sin versionar', () => {
    const { sesion, opc } = conCarro();
    const antes = sesion.getState();

    const r = sesion.ejecutar(guardarEnAlmacenPersonal, { recurso: 'piedra', cantidad: 5 }, opc);

    expect(r.ok).toBe(false);
    expect(r.codigoError).toBe('heroe.invalido');
    expect(sesion.getState()).toBe(antes);
  });
});

describe('reclutarEnCampamento', () => {
  /** El fundador reside en el campamento, con oro, y su Facción es la única humana y ya tiene la tecnología de la tropa. */
  function residenteConOro(oro: number) {
    const base = conCampamentos();
    base.sesion.ejecutar(residirEnCampamento, { heroeId: base.fundador, campamentoId: 'merc-1' }, base.opc);
    const payload = base.sesion.exportar();
    const { state } = payload;
    const sesion = GameSession.importar({
      ...payload,
      state: {
        ...state,
        heroes: state.heroes.map((h) => (h.id === base.fundador ? { ...h, almacenPersonal: { oro }, ubicacion: { tipo: 'mercenarios' as const, campamentoId: 'merc-1' } } : h)),
        tecnologia: {
          ...state.tecnologia,
          primeros: { ...state.tecnologia.primeros, escudos_ligeros: { faccionId: base.faccionId, en: 0 as never } },
          porFaccion: { ...state.tecnologia.porFaccion, [base.faccionId]: { aparecidas: ['escudos_ligeros'], adoptadas: ['escudos_ligeros'] } },
        },
      },
    });
    return { ...base, sesion };
  }

  it('recluta con la tecnología del campamento y el oro del almacén personal, y deja al héroe con su escuadrón', () => {
    const { sesion, fundador, opc } = residenteConOro(5000);

    const r = sesion.ejecutar(reclutarEnCampamento, { tropaId: 'lanceros_mimbre' }, opc);

    expect(r.ok).toBe(true);
    expect(r.datos?.cantidad).toBe(25);
    const heroe = sesion.getState().heroes.find((h) => h.id === fundador)!;
    expect(heroe.escuadrones.some((e) => e.tropaId === 'lanceros_mimbre' && e.cantidad === 25)).toBe(true);
    expect(heroe.almacenPersonal?.['oro']).toBe(5000 - r.datos!.oro);
    expect(campamentoDe(sesion, 'merc-1').poblacion).toBe(75);
  });

  it('rechazo: sin oro, y no versiona', () => {
    const { sesion, opc } = residenteConOro(1);
    const antes = sesion.getState();

    const r = sesion.ejecutar(reclutarEnCampamento, { tropaId: 'lanceros_mimbre' }, opc);

    expect(r.ok).toBe(false);
    expect(r.codigoError).toBe('mercenarios.invalido');
    expect(sesion.getState()).toBe(antes);
  });
});

describe('comprarEnCampamento', () => {
  it('compra con el oro del almacén personal, que se destruye, y resta stock del campamento', () => {
    const base = conCampamentos();
    base.sesion.ejecutar(residirEnCampamento, { heroeId: base.fundador, campamentoId: 'merc-1' }, base.opc);
    const payload = base.sesion.exportar();
    const sesion = GameSession.importar({
      ...payload,
      state: { ...payload.state, heroes: payload.state.heroes.map((h) => (h.id === base.fundador ? { ...h, almacenPersonal: { oro: 500 }, ubicacion: { tipo: 'mercenarios' as const, campamentoId: 'merc-1' } } : h)) },
    });

    const r = sesion.ejecutar(comprarEnCampamento, { recurso: 'madera', cantidad: 10 }, base.opc);

    expect(r.ok).toBe(true);
    const heroe = sesion.getState().heroes.find((h) => h.id === base.fundador)!;
    expect(heroe.almacenPersonal?.['madera']).toBe(10);
    expect(heroe.almacenPersonal?.['oro']).toBe(500 - r.datos!.oro);
    expect(campamentoDe(sesion, 'merc-1').mercado['madera']).toBe(90);
  });

  it('rechazo: residiendo pero lejos del campamento (D75: dentro o con la columna en su puerta)', () => {
    const base = conCampamentos();
    base.sesion.ejecutar(residirEnCampamento, { heroeId: base.fundador, campamentoId: 'merc-1' }, base.opc);
    const r = base.sesion.ejecutar(comprarEnCampamento, { recurso: 'madera', cantidad: 1 }, base.opc);
    expect(r.codigoError).toBe('campamento.lejos');
  });

  it('rechazo: sin residir en un campamento, y no versiona', () => {
    const { sesion, opc } = conCampamentos();
    const antes = sesion.getState();
    const r = sesion.ejecutar(comprarEnCampamento, { recurso: 'madera', cantidad: 1 }, opc);
    expect(r.ok).toBe(false);
    expect(r.codigoError).toBe('mercenarios.invalido');
    expect(sesion.getState()).toBe(antes);
  });
});

describe('el comercio del mundo alimenta los mercados', () => {
  it('exito suma a los contadores el trueque cerrado entre Facciones y no el hecho consigo misma', () => {
    const { sesion } = conCampamentos();
    const e = (entreFacciones: boolean) => ({ codigo: 'comercio.trueque_cumplido', mensaje: '', payload: { entreFacciones, intercambiado: { madera: 40 } }, momento: '', version: 0 }) as never;

    const entre = exito(sesion.getState(), [e(true)]).estado;
    expect(entre.mercadoMercenario.contadores).toEqual({ madera: 40 });
    const consigo = exito(entre, [e(false)]).estado;
    expect(consigo.mercadoMercenario.contadores).toEqual({ madera: 40 });
  });
});

describe('refundar desde el campamento', () => {
  /** Una Facción sin asentamientos, con su héroe en el campamento y los recursos del coste en su almacén personal. */
  function sinPlazas() {
    const base = conCampamentos();
    const payload = base.sesion.exportar();
    const costo = costoRefundacion();
    const sesion = GameSession.importar({
      ...payload,
      state: {
        ...payload.state,
        asentamientos: [],
        campamentosMercenarios: payload.state.campamentosMercenarios.map((c) => (c.id === 'merc-1' ? { ...c, residentesIds: [base.fundador] } : c)),
        heroes: payload.state.heroes.map((h) => (h.id === base.fundador ? { ...h, almacenPersonal: costo, ubicacion: { tipo: 'mercenarios' as const, campamentoId: 'merc-1' } } : h)),
      },
    });
    return { ...base, sesion, costo };
  }

  it('aporta al fondo y compra la caravana, que sale del campamento con su Facción y es visible para ella', () => {
    const { sesion, faccionId, fundador, opc, costo } = sinPlazas();

    for (const [recurso, cantidad] of Object.entries(costo)) {
      expect(sesion.ejecutar(aportarARefundacion, { recurso: recurso as never, cantidad }, opc).ok).toBe(true);
    }
    expect(campamentoDe(sesion, 'merc-1').fondos[fundador]).toEqual(costo);

    const r = sesion.ejecutar(comprarCaravanaDeRefundacion, { destino: { x: 430, y: 430 } }, opc);

    expect(r.ok).toBe(true);
    const caravana = sesion.getState().caravanas.find((c) => c.id === r.datos!.caravanaId)!;
    expect(caravana).toMatchObject({ tipo: 'construccion', faccionId, origenCampamentoId: 'merc-1' });
    expect(campamentoDe(sesion, 'merc-1').fondos[fundador] ?? {}).toEqual({});
  });

  it('rechazo: sin fondo suficiente, y no versiona', () => {
    const { sesion, opc } = sinPlazas();
    const antes = sesion.getState();

    const r = sesion.ejecutar(comprarCaravanaDeRefundacion, { destino: { x: 430, y: 430 } }, opc);

    expect(r.ok).toBe(false);
    expect(r.codigoError).toBe('mercenarios.invalido');
    expect(sesion.getState()).toBe(antes);
  });
});
