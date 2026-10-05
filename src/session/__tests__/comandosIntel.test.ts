// Intel de las tabernas (Doc 5.12.10), por la ruta real: comandos `comprarMirada` y `comprarInformePlaza`, su autorización y lo que
// el jugador ve después en su proyección.
import { describe, expect, it } from 'vitest';
import { INTEL, MERCENARIOS } from '../../constants';
import type { CampamentoMercenarios, Edificio, RelacionPolitica } from '../../domain/types';
import { instante } from '../../domain/tiempo';
import { precioInforme, precioMirada } from '../../engine/intel';
import { verificarAutorizacion } from '../comandos/autorizacion';
import { comprarInformePlaza, comprarMirada } from '../comandos/intel';
import { proyectarParaJugador } from '../proyecciones/jugador';
import type { GeometriaAsentamientos } from '../estado';
import { GameSession } from '../gameSession';
import { abastecer, partidaConAsentamiento } from './fixtures';

const SIN_GEOMETRIA: GeometriaAsentamientos = { zonas: [], zonasFusionadas: [], trazadoPorAsentamiento: {} };
const RIVAL_EN = { x: 1400, y: 400 };

const tabernaDe = (nivelInterno = 1): Edificio => ({ id: 'taberna-1', tipo: 'taberna', posicion: { x: 6, y: 6 }, estado: 'activo', nivelInterno });

/** La partida del fixture, abastecida, con taberna en la plaza y una plaza rival lejos, de otra Facción (sin conocer). */
function conTaberna(opciones: { taberna?: boolean; nivelTaberna?: number } = {}) {
  const base = partidaConAsentamiento();
  const payload = abastecer(base.sesion).exportar();
  const [propia] = payload.state.asentamientos;
  const [faccionPropia] = payload.state.facciones;
  const rival = { ...propia!, id: 'plaza-rival', nombre: 'Troya', faccionId: 'f-rival', nivel: 2, posicion: RIVAL_EN };
  const sesion = GameSession.importar({
    ...payload,
    state: {
      ...payload.state,
      facciones: [...payload.state.facciones, { ...faccionPropia!, id: 'f-rival', nombre: 'Troya', reyId: null, ciudadanosIds: [] }],
      asentamientos: [
        opciones.taberna === false ? propia! : { ...propia!, edificios: [...propia!.edificios, tabernaDe(opciones.nivelTaberna)] },
        rival,
      ],
    },
  });
  return { ...base, sesion, opc: { actor: base.fundador }, origen: { tipo: 'asentamiento' as const, id: base.asentamientoId } };
}

const oroDe = (sesion: GameSession) => sesion.getState().asentamientos[0]!.almacen['oro']!.cantidad;
const vista = (sesion: GameSession, heroeId: string, ticks = 0) => proyectarParaJugador({ ...sesion.getState(), tick: sesion.getState().tick + ticks }, heroeId, SIN_GEOMETRIA);

describe('comprarMirada', () => {
  it('abre un ojo sobre una zona lejana: cobra el oro del almacén y la plaza rival, antes oculta, pasa a verse en vivo', () => {
    const { sesion, opc, origen, fundador } = conTaberna();
    expect(vista(sesion, fundador).asentamientosAvistados.map((a) => a.id)).not.toContain('plaza-rival');
    const oroAntes = oroDe(sesion);
    const ojos = sesion.getState().asentamientos.filter((a) => a.faccionId === sesion.getState().facciones[0]!.id).map((a) => a.posicion);

    const r = sesion.ejecutar(comprarMirada, { origen, centro: RIVAL_EN }, opc);

    expect(r.ok).toBe(true);
    expect(oroDe(sesion)).toBe(oroAntes - precioMirada(RIVAL_EN, ojos));
    expect(r.datos).toMatchObject({ origenId: origen.id, centro: RIVAL_EN, radio: INTEL.mirada.radio });
    const v = vista(sesion, fundador);
    expect(v.asentamientosAvistados.find((a) => a.id === 'plaza-rival')).toMatchObject({ nombre: 'Troya', nivel: 2, faccionId: 'f-rival' });
    expect(v.miradasIntel).toHaveLength(1);
    expect(v.tarifasIntel.mirada.radio).toBe(INTEL.mirada.radio);
  });

  it('no deja memoria: al caducar la plaza vuelve a la niebla, y la zona sigue en enfriamiento un rato', () => {
    const { sesion, opc, origen, fundador } = conTaberna();
    sesion.ejecutar(comprarMirada, { origen, centro: RIVAL_EN }, opc);

    const caducada = vista(sesion, fundador, INTEL.mirada.duracionMinutos + 1);
    expect(caducada.asentamientosAvistados.map((a) => a.id)).not.toContain('plaza-rival');
    expect(caducada.asentamientosConocidos.map((a) => a.asentamientoId)).not.toContain('plaza-rival');
    expect(caducada.miradasIntel).toHaveLength(1); // en enfriamiento
    expect(vista(sesion, fundador, INTEL.mirada.duracionMinutos + INTEL.mirada.cooldownMinutos + 1).miradasIntel).toHaveLength(0);
  });

  it('la ven en vivo los aliados de la Facción compradora, y solo mientras dura la alianza', () => {
    const { sesion, opc, origen, fundador, faccionId } = conTaberna();
    sesion.ejecutar(comprarMirada, { origen, centro: RIVAL_EN }, opc);
    const otraPlaza = { ...sesion.getState().asentamientos[0]!, id: 'plaza-aliada', faccionId: 'f-aliada', posicion: { x: 100, y: 1800 } };
    const payload = sesion.exportar();
    const alianza: RelacionPolitica = { id: 'r1', tipo: 'alianza', faccionAId: faccionId, faccionBId: 'f-aliada', creadoEn: instante(0), estado: 'activa' };
    const conAliada = GameSession.importar({
      ...payload,
      state: {
        ...payload.state,
        facciones: [...payload.state.facciones, { ...payload.state.facciones[0]!, id: 'f-aliada', reyId: 'rey-aliado', ciudadanosIds: ['rey-aliado'] }],
        asentamientos: [...payload.state.asentamientos, otraPlaza],
        heroes: [...payload.state.heroes, { ...payload.state.heroes.find((h) => h.id === fundador)!, id: 'rey-aliado' }],
        relaciones: [alianza],
      },
    });
    expect(vista(conAliada, 'rey-aliado').asentamientosAvistados.map((a) => a.id)).toContain('plaza-rival');
    expect(vista(conAliada, 'rey-aliado').miradasIntel).toEqual([]); // ve lo que mira, no la compra del aliado
    const rota = GameSession.importar({ ...conAliada.exportar(), state: { ...conAliada.getState(), relaciones: [{ ...alianza, estado: 'rota' as const }] } });
    expect(vista(rota, 'rey-aliado').asentamientosAvistados.map((a) => a.id)).not.toContain('plaza-rival');
  });

  it('rechazos sin tocar el estado: plaza sin taberna, sin oro, cupo lleno y zona fuera del mapa', () => {
    const sinTaberna = conTaberna({ taberna: false });
    const antes = sinTaberna.sesion.getState();
    expect(sinTaberna.sesion.ejecutar(comprarMirada, { origen: sinTaberna.origen, centro: RIVAL_EN }, sinTaberna.opc).codigoError).toBe('intel.invalida');
    expect(sinTaberna.sesion.getState()).toBe(antes);

    const { sesion, opc, origen } = conTaberna();
    const payload = sesion.exportar();
    const pobre = GameSession.importar({
      ...payload,
      state: { ...payload.state, asentamientos: payload.state.asentamientos.map((a, i) => (i === 0 ? { ...a, almacen: { ...a.almacen, oro: { ...a.almacen['oro']!, cantidad: 1 } } } : a)) },
    });
    expect(pobre.ejecutar(comprarMirada, { origen, centro: RIVAL_EN }, opc).codigoError).toBe('intel.invalida');

    expect(sesion.ejecutar(comprarMirada, { origen, centro: { x: -5, y: 100 } }, opc).codigoError).toBe('intel.invalida');
    expect(sesion.ejecutar(comprarMirada, { origen, centro: RIVAL_EN }, opc).ok).toBe(true);
    const lleno = sesion.ejecutar(comprarMirada, { origen, centro: { x: 200, y: 1700 } }, opc);
    expect(lleno.codigoError).toBe('intel.invalida');
  });

  it('el nivel de la taberna sube el cupo de Miradas abiertas a la vez', () => {
    const { sesion, opc, origen } = conTaberna({ nivelTaberna: 2 });
    expect(sesion.ejecutar(comprarMirada, { origen, centro: RIVAL_EN }, opc).ok).toBe(true);
    expect(sesion.ejecutar(comprarMirada, { origen, centro: { x: 200, y: 1700 } }, opc).ok).toBe(true);
    expect(sesion.ejecutar(comprarMirada, { origen, centro: { x: 1700, y: 1700 } }, opc).ok).toBe(false);
  });
});

describe('comprarInformePlaza', () => {
  it('exige conocer la plaza; con ella en una Mirada se compra: layout y defensa con fecha, sin almacén, y avisa a la víctima sin firma', () => {
    const { sesion, opc, origen, fundador } = conTaberna();
    expect(sesion.ejecutar(comprarInformePlaza, { origen, asentamientoId: 'plaza-rival' }, opc).codigoError).toBe('intel.invalida');
    sesion.ejecutar(comprarMirada, { origen, centro: RIVAL_EN }, opc);
    const oroAntes = oroDe(sesion);

    const r = sesion.ejecutar(comprarInformePlaza, { origen, asentamientoId: 'plaza-rival' }, opc);

    expect(r.ok).toBe(true);
    expect(oroDe(sesion)).toBe(oroAntes - precioInforme({ nivel: 2 }));
    expect(r.datos).toMatchObject({ asentamientoId: 'plaza-rival', faccionId: 'f-rival', nivel: 2 });
    expect(r.datos!.edificios.length).toBeGreaterThan(0);
    expect(JSON.stringify(r.datos)).not.toMatch(/almacen/);
    // El aviso es para la plaza espiada y no dice quién.
    expect(r.eventos).toHaveLength(1);
    expect(r.eventos[0]).toMatchObject({ codigo: 'asentamiento.informe_pedido', asentamientoId: 'plaza-rival' });
    expect(JSON.stringify(r.eventos[0])).not.toContain(fundador);
    // Y queda en la memoria de la Facción, también con la Mirada ya caducada.
    expect(vista(sesion, fundador, INTEL.mirada.duracionMinutos + 1).informesPlaza.map((i) => i.asentamientoId)).toEqual(['plaza-rival']);
  });

  it('el mismo informe no se repite antes de su cooldown, y de la propia plaza no se pide', () => {
    const { sesion, opc, origen, asentamientoId } = conTaberna();
    sesion.ejecutar(comprarMirada, { origen, centro: RIVAL_EN }, opc);
    expect(sesion.ejecutar(comprarInformePlaza, { origen, asentamientoId: 'plaza-rival' }, opc).ok).toBe(true);
    const antes = sesion.getState();
    expect(sesion.ejecutar(comprarInformePlaza, { origen, asentamientoId: 'plaza-rival' }, opc).codigoError).toBe('intel.invalida');
    expect(sesion.getState()).toBe(antes);
    expect(sesion.ejecutar(comprarInformePlaza, { origen, asentamientoId }, opc).codigoError).toBe('intel.invalida');
  });

  it('el informe sobrevive a un tick: la memoria de la Facción lo conserva al grabar lo visto', () => {
    const { sesion, opc, origen, fundador } = conTaberna();
    sesion.ejecutar(comprarMirada, { origen, centro: RIVAL_EN }, opc);
    sesion.ejecutar(comprarInformePlaza, { origen, asentamientoId: 'plaza-rival' }, opc);
    sesion.avanzarTick();
    expect(vista(sesion, fundador).informesPlaza).toHaveLength(1);
  });
});

describe('autorización', () => {
  it('en una taberna de plaza compran el Rey, el Embajador o el Gobernador presentes, y un vecino sin cargo no', () => {
    const { sesion, origen, fundador, vecino } = conTaberna();
    const quien = (heroeId: string) => verificarAutorizacion('comprarMirada', { origen, centro: RIVAL_EN }, sesion.getState(), { rol: 'jugador', heroeId });
    expect(quien(fundador)).toEqual({ autorizado: true });
    expect(quien(vecino)).toEqual({ autorizado: false, motivo: 'condicion_dominio' });
  });
});

describe('taberna de un campamento de mercenarios', () => {
  const campamento = { id: 'merc-1', posicion: { x: 900, y: 900 }, origen: 0, edificios: [...MERCENARIOS.edificiosFijos, 'barracon'], residentesIds: [], poblacion: 100, poblacionEn: 0, mercado: {}, fondos: {}, creadoEn: 0 } as unknown as CampamentoMercenarios;

  /** El fundador dentro del campamento, con oro de botín; sin taberna en la plaza: aquí compra cualquier héroe de una Facción. */
  function enElCampamento(oro: number) {
    const base = conTaberna({ taberna: false });
    const payload = base.sesion.exportar();
    const sesion = GameSession.importar({
      ...payload,
      state: {
        ...payload.state,
        campamentosMercenarios: [campamento],
        heroes: payload.state.heroes.map((h) => (h.id === base.fundador ? { ...h, oroDeBotin: oro, ubicacion: { tipo: 'mercenarios' as const, campamentoId: 'merc-1' } } : h)),
      },
    });
    return { ...base, sesion, origen: { tipo: 'campamento' as const, id: 'merc-1' } };
  }
  const botin = (sesion: GameSession, heroeId: string) => sesion.getState().heroes.find((h) => h.id === heroeId)!.oroDeBotin ?? 0;

  it('compra con el oro de botín del héroe y la Mirada es de su Facción, con un cupo fijo', () => {
    const { sesion, opc, origen, fundador, faccionId } = enElCampamento(500);

    const r = sesion.ejecutar(comprarMirada, { origen, centro: RIVAL_EN }, opc);

    expect(r.ok).toBe(true);
    expect(botin(sesion, fundador)).toBeLessThan(500);
    expect(sesion.getState().miradasIntel).toEqual([expect.objectContaining({ faccionId, origenId: 'merc-1' })]);
    expect(sesion.ejecutar(comprarMirada, { origen, centro: { x: 200, y: 1700 } }, opc).codigoError).toBe('intel.invalida');
  });

  it('rechazo: sin oro de botín, y fuera del campamento', () => {
    const pobre = enElCampamento(1);
    expect(pobre.sesion.ejecutar(comprarMirada, { origen: pobre.origen, centro: RIVAL_EN }, pobre.opc).codigoError).toBe('intel.invalida');

    const { sesion, opc, origen } = enElCampamento(500);
    const payload = sesion.exportar();
    const lejos = GameSession.importar({
      ...payload,
      state: { ...payload.state, heroes: payload.state.heroes.map((h) => ({ ...h, ubicacion: { tipo: 'desconectado' as const, punto: { x: 5, y: 5 } } })) },
    });
    expect(lejos.ejecutar(comprarMirada, { origen, centro: RIVAL_EN }, opc).codigoError).toBe('campamento.lejos');
  });
});
