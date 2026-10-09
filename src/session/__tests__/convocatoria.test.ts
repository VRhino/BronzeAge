// Ejército en preparación dentro de una plaza o de un campamento (Doc 5.14.5), de principio a fin con comandos: convocar, unirse,
// separarse, cancelar y partir, con la salida de cada integrante por el camino de siempre.
import { describe, expect, it } from 'vitest';
import { heroesNombrados } from '../../domain/eventos';
import { GameSession } from '../gameSession';
import { crearHeroe } from '../comandos/crearHeroe';
import { crearFaccion } from '../comandos/crearFaccion';
import { responderSolicitud, solicitarIngreso } from '../comandos/ingresoEnFaccion';
import { REGISTRO_COMANDOS } from '../comandos/registro';
import { proyectarParaJugador } from '../proyecciones/jugador';
import { conEscuadrones } from '../../engine/tropa';
import { instante } from '../../domain/tiempo';
import { escuadronDePrueba } from '../../engine/__tests__/fixtures';
import { abastecer, partidaConAsentamiento } from './fixtures';

const SIN_GEOMETRIA = { zonas: [], zonasFusionadas: [], trazadoPorAsentamiento: {} };

type Tipo = keyof typeof REGISTRO_COMANDOS;
const ejecutarEn = (sesion: GameSession) => (tipo: Tipo, heroeId: string, params: Record<string, unknown> = {}) =>
  sesion.ejecutar(REGISTRO_COMANDOS[tipo] as never, { heroeId, ...params } as never, { actor: heroeId });
const LISTO = { escuadronIds: [] as string[], carga: { trigo: 5 } };

function enLaPlaza() {
  const base = partidaConAsentamiento();
  // Un almacén holgado: los víveres de cada integrante salen del trigo que sobra de la reserva, y el de un solo héroe ya se lleva casi todo el inicial.
  const abastecida = abastecer(base.sesion).exportar();
  const a = abastecida.state.asentamientos[0]!;
  const sesion = GameSession.importar({
    ...abastecida,
    state: { ...abastecida.state, asentamientos: [{ ...a, almacen: { ...a.almacen, trigo: { ...a.almacen['trigo']!, cantidad: 5000, capacidad: 5000 } } }, ...abastecida.state.asentamientos.slice(1)] },
  });
  return { ...base, sesion, ejecutar: ejecutarEn(sesion), estado: () => sesion.getState() };
}

describe('en una plaza', () => {
  it('convocar deja el ejército en preparación sin sacar a nadie', () => {
    const { fundador, ejecutar, estado } = enLaPlaza();
    const antes = estado().asentamientos[0]!.almacen['trigo']!.cantidad;

    const r = ejecutar('convocarEjercito', fundador, { politicaDeUnion: 'aceptar', ...LISTO });

    expect(r.ok).toBe(true);
    expect(estado().convocatorias).toHaveLength(1);
    expect(estado().convocatorias![0]).toMatchObject({ liderId: fundador, politicaDeUnion: 'aceptar', lugar: { tipo: 'asentamiento' } });
    expect(estado().ejercitos, 'nadie ha salido').toHaveLength(0);
    expect(estado().heroes.find((h) => h.id === fundador)!.ubicacion.tipo).toBe('asentamiento');
    expect(estado().asentamientos[0]!.almacen['trigo']!.cantidad, 'el almacén sigue intacto').toBe(antes);
  });

  it('se une quien está dentro con la misma Facción, y parten juntos como UN ejército quieto en la puerta', () => {
    const { fundador, vecino, asentamientoId, ejecutar, estado } = enLaPlaza();
    ejecutar('convocarEjercito', fundador, { politicaDeUnion: 'aceptar', ...LISTO });
    const id = estado().convocatorias![0]!.id;

    expect(ejecutar('unirseAConvocatoria', vecino, { convocatoriaId: id, ...LISTO }).ok).toBe(true);
    const r = ejecutar('partirConvocatoria', fundador);

    expect(r.ok).toBe(true);
    const e = estado();
    expect(e.convocatorias).toHaveLength(0);
    expect(e.ejercitos).toHaveLength(1);
    expect(e.ejercitos[0]).toMatchObject({ tipo: 'ejercito', liderId: fundador, estado: 'estacionado', politicaDeUnion: 'aceptar', ruta: [] });
    expect(e.ejercitos[0]!.participantes.map((p) => p.heroeId).sort()).toEqual([fundador, vecino].sort());
    expect(e.ejercitos[0]!.suministro['trigo'], 'el carro suma lo de los dos').toBe(10);
    for (const h of [fundador, vecino]) expect(e.heroes.find((x) => x.id === h)!.ubicacion).toEqual({ tipo: 'columna', ejercitoId: e.ejercitos[0]!.id });
    expect(e.asentamientos.find((a) => a.id === asentamientoId)).toBeDefined();
  });

  it('el Líder dirige el ejército que parte con sus clics; el otro integrante no', () => {
    const { fundador, vecino, ejecutar, estado } = enLaPlaza();
    ejecutar('convocarEjercito', fundador, { politicaDeUnion: 'aceptar', ...LISTO });
    ejecutar('unirseAConvocatoria', vecino, { convocatoriaId: estado().convocatorias![0]!.id, ...LISTO });
    ejecutar('partirConvocatoria', fundador);
    const p = estado().ejercitos[0]!.posicionActual;
    const objetivo = { tipo: 'punto', punto: { x: p.x + 60, y: p.y } };

    expect(ejecutar('marcharA', vecino, { objetivo }).ok).toBe(false);
    expect(ejecutar('marcharA', fundador, { objetivo }).ok).toBe(true);
    expect(estado().ejercitos[0]!.estado).toBe('marchando');
  });

  it('cancelar no mueve nada: todos siguen dentro', () => {
    const { fundador, vecino, ejecutar, estado } = enLaPlaza();
    ejecutar('convocarEjercito', fundador, { politicaDeUnion: 'aceptar', ...LISTO });
    ejecutar('unirseAConvocatoria', vecino, { convocatoriaId: estado().convocatorias![0]!.id, ...LISTO });

    expect(ejecutar('cancelarConvocatoria', vecino).ok, 'solo el Líder cancela').toBe(false);
    expect(ejecutar('cancelarConvocatoria', fundador).ok).toBe(true);

    expect(estado().convocatorias).toHaveLength(0);
    expect(estado().ejercitos).toHaveLength(0);
    for (const h of [fundador, vecino]) expect(estado().heroes.find((x) => x.id === h)!.ubicacion.tipo).toBe('asentamiento');
  });

  it('un integrante se separa mientras se prepara; el Líder no, cancela', () => {
    const { fundador, vecino, ejecutar, estado } = enLaPlaza();
    ejecutar('convocarEjercito', fundador, { politicaDeUnion: 'aceptar', ...LISTO });
    ejecutar('unirseAConvocatoria', vecino, { convocatoriaId: estado().convocatorias![0]!.id, ...LISTO });

    expect(ejecutar('separarseDeConvocatoria', fundador).ok).toBe(false);
    expect(ejecutar('separarseDeConvocatoria', vecino).ok).toBe(true);
    expect(estado().convocatorias![0]!.integrantes.map((i) => i.heroeId)).toEqual([fundador]);

    ejecutar('partirConvocatoria', fundador);
    expect(estado().ejercitos[0]!.participantes).toHaveLength(1);
  });

  it('con política preguntar queda una petición que el Líder acepta o rechaza', () => {
    const { fundador, vecino, ejecutar, estado } = enLaPlaza();
    ejecutar('convocarEjercito', fundador, { politicaDeUnion: 'preguntar', ...LISTO });
    const id = estado().convocatorias![0]!.id;

    const pide = ejecutar('unirseAConvocatoria', vecino, { convocatoriaId: id, ...LISTO });
    expect(pide.ok && pide.datos).toEqual({ unido: false });
    expect(estado().convocatorias![0]!.peticiones).toHaveLength(1);
    expect(estado().convocatorias![0]!.integrantes).toHaveLength(1);

    expect(ejecutar('responderPeticionDeConvocatoria', vecino, { convocatoriaId: id, solicitanteId: vecino, aceptar: true }).ok, 'solo el Líder contesta').toBe(false);
    expect(ejecutar('responderPeticionDeConvocatoria', fundador, { convocatoriaId: id, solicitanteId: vecino, aceptar: true }).ok).toBe(true);
    expect(estado().convocatorias![0]!.integrantes.map((i) => i.heroeId)).toEqual([fundador, vecino]);
  });

  it('un integrante no sale por su cuenta, y no se une a dos ejércitos', () => {
    const { fundador, vecino, asentamientoId, ejecutar, estado } = enLaPlaza();
    ejecutar('convocarEjercito', fundador, { politicaDeUnion: 'aceptar', ...LISTO });
    const id = estado().convocatorias![0]!.id;

    const sale = ejecutar('salirAlMundo', fundador, { asentamientoId, ...LISTO });
    expect(sale.ok === false && sale.detalleError).toMatch(/ejército en preparación/);
    expect(ejecutar('convocarEjercito', fundador, { politicaDeUnion: 'aceptar', ...LISTO }).ok).toBe(false);

    ejecutar('unirseAConvocatoria', vecino, { convocatoriaId: id, ...LISTO });
    expect(ejecutar('unirseAConvocatoria', vecino, { convocatoriaId: id, ...LISTO }).ok).toBe(false);
  });

  it('no se convoca ni se une con una carga que luego no podría salir', () => {
    const { fundador, vecino, ejecutar, estado } = enLaPlaza();
    expect(ejecutar('convocarEjercito', fundador, { politicaDeUnion: 'aceptar', escuadronIds: [], carga: { trigo: 10_000_000 } }).ok).toBe(false);
    ejecutar('convocarEjercito', fundador, { politicaDeUnion: 'aceptar', ...LISTO });

    expect(ejecutar('unirseAConvocatoria', vecino, { convocatoriaId: estado().convocatorias![0]!.id, escuadronIds: [], carga: { trigo: 10_000_000 } }).ok).toBe(false);
    expect(estado().convocatorias![0]!.integrantes).toHaveLength(1);
  });

  it('quien salió por su cuenta del lugar queda fuera al partir', () => {
    const { fundador, vecino, asentamientoId, sesion, ejecutar, estado } = enLaPlaza();
    ejecutar('convocarEjercito', fundador, { politicaDeUnion: 'aceptar', ...LISTO });
    ejecutar('unirseAConvocatoria', vecino, { convocatoriaId: estado().convocatorias![0]!.id, ...LISTO });
    // El vecino sale por la vía de siempre (un administrador, un tick de presencia…): ya no está dentro.
    const p = sesion.exportar();
    const fuera = GameSession.importar({ ...p, state: { ...p.state, heroes: p.state.heroes.map((h) => (h.id === vecino ? { ...h, ubicacion: { tipo: 'desconectado' as const, punto: { x: 0, y: 0 } } } : h)) } });

    const r = ejecutarEn(fuera)('partirConvocatoria', fundador);
    expect(r.ok).toBe(true);
    expect(fuera.getState().ejercitos[0]!.participantes.map((x) => x.heroeId)).toEqual([fundador]);
    expect(asentamientoId).toBeDefined();
  });

  it('la proyección lista las convocatorias del lugar a los de la Facción, con la petición solo para su Líder y su dueño', () => {
    const { fundador, vecino, ejecutar, estado } = enLaPlaza();
    ejecutar('convocarEjercito', fundador, { politicaDeUnion: 'preguntar', ...LISTO });
    const id = estado().convocatorias![0]!.id;
    ejecutar('unirseAConvocatoria', vecino, { convocatoriaId: id, ...LISTO });

    const delVecino = proyectarParaJugador(estado(), vecino, SIN_GEOMETRIA);
    expect(delVecino.convocatorias).toHaveLength(1);
    expect(delVecino.convocatorias[0]).toMatchObject({ id, liderId: fundador, soyLider: false, soyIntegrante: false });
    expect(delVecino.convocatorias[0]!.peticiones.map((p) => p.heroeId)).toEqual([vecino]);
    expect(delVecino.miConvocatoriaId).toBeUndefined();

    const delLider = proyectarParaJugador(estado(), fundador, SIN_GEOMETRIA);
    expect(delLider.miConvocatoriaId).toBe(id);
    expect(delLider.convocatorias[0]).toMatchObject({ soyLider: true, soyIntegrante: true });
    expect(delLider.convocatorias[0]!.integrantes.map((i) => i.heroeId)).toEqual([fundador]);
    expect(proyectarParaJugador(estado(), 'forastero', SIN_GEOMETRIA).convocatorias).toEqual([]);
  });

  it('los avisos nombran al Líder y a cada integrante (canal heroe/<id>)', () => {
    const { fundador, vecino, ejecutar, estado } = enLaPlaza();
    ejecutar('convocarEjercito', fundador, { politicaDeUnion: 'aceptar', ...LISTO });
    const r = ejecutar('unirseAConvocatoria', vecino, { convocatoriaId: estado().convocatorias![0]!.id, ...LISTO });

    const unido = r.eventos.find((e) => e.codigo === 'convocatoria.unido')!;
    expect(heroesNombrados(unido).sort()).toEqual([fundador, vecino].sort());
    const parte = ejecutar('partirConvocatoria', fundador).eventos.find((e) => e.codigo === 'convocatoria.partio')!;
    expect(heroesNombrados(parte).sort()).toEqual([fundador, vecino].sort());
  });
});

describe('en un campamento de mercenarios', () => {
  const AVATAR = { cabezaId: '', peloId: '', barbaId: '', cejasId: '' };

  it('dos ciudadanos de la misma Facción dentro del campamento parten como un ejército en su puerta', () => {
    const sesion = GameSession.crear('campamento', { seed: 42 });
    const crear = (actor: string, nombre: string) =>
      sesion.ejecutar(crearHeroe, { displayName: nombre, campamentoId: 'mercenarios-0', classDefinitionId: 'Spear', genero: 'femenino', avatar: AVATAR }, { actor }).datos!.heroeId;
    const ana = crear('j1', 'Ana');
    const bea = crear('j2', 'Bea');
    const faccionId = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, { actor: ana }).datos!.faccionId;
    sesion.ejecutar(solicitarIngreso, { faccionId }, { actor: bea });
    sesion.ejecutar(responderSolicitud, { faccionId, heroeId: bea, aceptar: true }, { actor: ana });
    const ejecutar = ejecutarEn(sesion);

    expect(ejecutar('convocarEjercito', ana, { politicaDeUnion: 'aceptar', escuadronIds: [], carga: {} }).ok).toBe(true);
    const c = sesion.getState().convocatorias![0]!;
    expect(c.lugar).toEqual({ tipo: 'campamento', id: 'mercenarios-0' });
    expect(ejecutar('unirseAConvocatoria', bea, { convocatoriaId: c.id, escuadronIds: [], carga: {} }).ok).toBe(true);

    const r = ejecutar('partirConvocatoria', ana);
    expect(r.ok).toBe(true);
    const e = sesion.getState().ejercitos;
    expect(e).toHaveLength(1);
    expect(e[0]).toMatchObject({ tipo: 'ejercito', liderId: ana, estado: 'estacionado', origenCampamentoId: 'mercenarios-0' });
    expect(e[0]!.participantes).toHaveLength(2);
  });
});

describe('cambiar la selección en preparación', () => {
  /** La plaza con dos escuadras en el campamento del fundador y del vecino. */
  function conTropas() {
    const { sesion, ...resto } = enLaPlaza();
    const p = sesion.exportar();
    const heroes = conEscuadrones(p.state.heroes, [
      escuadronDePrueba('esc-a', resto.fundador, 'lenadores'),
      escuadronDePrueba('esc-b', resto.fundador, 'granjeros'),
      escuadronDePrueba('esc-v', resto.vecino, 'milicia_lanceros'),
    ]);
    const nueva = GameSession.importar({ ...p, state: { ...p.state, heroes } });
    return { ...resto, sesion: nueva, ejecutar: ejecutarEn(nueva), estado: () => nueva.getState() };
  }

  it('cada uno cambia SU selección, y el orden de escuadronIds es el orden con el que sale su columna', () => {
    const { fundador, vecino, ejecutar, estado } = conTropas();
    ejecutar('convocarEjercito', fundador, { politicaDeUnion: 'aceptar', escuadronIds: ['esc-a'], carga: {} });
    const id = estado().convocatorias![0]!.id;
    ejecutar('unirseAConvocatoria', vecino, { convocatoriaId: id, escuadronIds: [], carga: {} });

    expect(ejecutar('cambiarSeleccionDeConvocatoria', fundador, { escuadronIds: ['esc-b', 'esc-a'], carga: { trigo: 5 } }).ok).toBe(true);
    expect(ejecutar('cambiarSeleccionDeConvocatoria', vecino, { escuadronIds: ['esc-v'], carga: {} }).ok).toBe(true);
    expect(estado().convocatorias![0]!.integrantes.map((i) => [i.heroeId, i.escuadronIds])).toEqual([
      [fundador, ['esc-b', 'esc-a']],
      [vecino, ['esc-v']],
    ]);

    ejecutar('partirConvocatoria', fundador);
    expect(estado().ejercitos[0]!.escuadronIds).toEqual(['esc-b', 'esc-a', 'esc-v']);
    expect(estado().ejercitos[0]!.suministro['trigo']).toBe(5);
  });

  it('no se cambia la de otro, ni con algo que no podría sacar; el aviso llega a todos', () => {
    const { fundador, vecino, ejecutar, estado } = conTropas();
    ejecutar('convocarEjercito', fundador, { politicaDeUnion: 'aceptar', escuadronIds: [], carga: {} });
    ejecutar('unirseAConvocatoria', vecino, { convocatoriaId: estado().convocatorias![0]!.id, escuadronIds: [], carga: {} });

    expect(ejecutar('cambiarSeleccionDeConvocatoria', vecino, { escuadronIds: ['esc-a'], carga: {} }).ok, 'esc-a es del fundador').toBe(false);
    expect(ejecutar('cambiarSeleccionDeConvocatoria', fundador, { escuadronIds: [], carga: { trigo: 10_000_000 } }).ok).toBe(false);
    expect(ejecutar('cambiarSeleccionDeConvocatoria', 'forastero', { escuadronIds: [], carga: {} }).ok).toBe(false);

    const r = ejecutar('cambiarSeleccionDeConvocatoria', fundador, { escuadronIds: ['esc-a'], carga: {} });
    expect(heroesNombrados(r.eventos.find((e) => e.codigo === 'convocatoria.seleccion_cambiada')!).sort()).toEqual([fundador, vecino].sort());
  });

  it('la proyección enseña la tropa de cada integrante con nombre, nivel y moral, en el orden elegido', () => {
    const { fundador, ejecutar, estado } = conTropas();
    ejecutar('convocarEjercito', fundador, { politicaDeUnion: 'aceptar', escuadronIds: ['esc-b', 'esc-a'], carga: {} });

    const tropas = proyectarParaJugador(estado(), fundador, SIN_GEOMETRIA).convocatorias[0]!.integrantes[0]!.tropas;
    expect(tropas.map((t) => t.escuadronId)).toEqual(['esc-b', 'esc-a']);
    expect(tropas[0]).toMatchObject({ nombre: expect.any(String), nivel: expect.any(Number), moral: expect.any(Number), cantidad: expect.any(Number) });
  });
});

describe('reclutar en el campamento donde se reside (proyección)', () => {
  it('trae las tropas que permiten sus edificios con su precio por soldado, si están desbloqueadas, y los reclutas', () => {
    const sesion = GameSession.crear('reclutar', { seed: 42 });
    const ana = sesion.ejecutar(crearHeroe, { displayName: 'Ana', campamentoId: 'mercenarios-0', classDefinitionId: 'Spear', genero: 'femenino', avatar: { cabezaId: '', peloId: '', barbaId: '', cejasId: '' } }, { actor: 'j1' }).datos!.heroeId;

    const m = proyectarParaJugador(sesion.getState(), ana, SIN_GEOMETRIA).mercadoCampamento!;

    expect(m.reclutamiento).toBeDefined();
    expect(m.reclutamiento!.tropas.length).toBeGreaterThan(0);
    expect(m.reclutamiento!.tropas[0]).toMatchObject({ tropaId: expect.any(String), precioPorSoldado: expect.any(Number), desbloqueada: expect.any(Boolean) });
    expect(m.reclutamiento!.poblacion).toBeGreaterThanOrEqual(0);
    expect(m.reclutamiento!.topePoblacion).toBeGreaterThan(0);
  });
});

describe('la tecnología propia en la proyección (2026-10-09)', () => {
  it('trae la capital de la Facción, la Era de cada tecnología suya y las épicas posibles', () => {
    const { sesion, fundador, asentamientoId } = partidaConAsentamiento();

    const t = proyectarParaJugador(sesion.getState(), fundador, SIN_GEOMETRIA).tecnologia;

    expect(t.capitalId).toBe(asentamientoId);
    expect(t.eraDe['leva_comunal']).toBe('reinos_palaciales');
    expect(t.propias!.adoptadas.every((id) => t.eraDe[id] !== undefined)).toBe(true);
    expect(t.epicasPosibles, 'sin logros cumplidos no hay épica posible').toEqual([]);
    expect(proyectarParaJugador(sesion.getState(), 'forastero', SIN_GEOMETRIA).tecnologia.capitalId).toBeNull();
  });

  it('una épica es posible cuando su logro del servidor está cumplido y a la Facción aún no le ha aparecido', () => {
    const { sesion, fundador } = partidaConAsentamiento();
    const p = sesion.exportar();
    const con = GameSession.importar({ ...p, state: { ...p.state, tecnologia: { ...p.state.tecnologia, logros: { ...p.state.tecnologia.logros, metalurgia_cobre: instante(0) } } } });

    expect(proyectarParaJugador(con.getState(), fundador, SIN_GEOMETRIA).tecnologia.epicasPosibles).toEqual(['metalurgia_cobre']);
  });
});

describe('avistamientos de la proyección (2026-10-09)', () => {
  const geo = SIN_GEOMETRIA;

  it('una caravana con escolta cedida sin héroe llega como escoltada', () => {
    const { sesion, fundador } = partidaConAsentamiento();
    const p = sesion.exportar();
    const aqui = p.state.asentamientos[0]!.posicion;
    const caravana = (id: string, extra: object) => ({ id, tipo: 'comercial' as const, origenAsentamientoId: 'ajena', contenido: { trigo: 5 }, posicionActual: aqui, progreso: 0, estado: 'en_transito' as const, carros: [], ...extra });
    const con = GameSession.importar({ ...p, state: { ...p.state, caravanas: [caravana('con', { escoltaIds: ['esc-x'] }), caravana('sin', {})] } });

    const vistas = proyectarParaJugador(con.getState(), fundador, geo).caravanasAvistadas;
    expect(vistas.find((c) => c.id === 'con')!.escoltada).toBe(true);
    expect(vistas.find((c) => c.id === 'sin')!.escoltada).toBe(false);
  });

  it('el almacén de tu plaza llega cuando tu columna está a su puerta, y solo entonces', () => {
    const { sesion, fundador, asentamientoId } = enLaPlaza();
    sesion.ejecutar(REGISTRO_COMANDOS.salirAlMundo as never, { asentamientoId, heroeId: fundador, escuadronIds: [], carga: {} } as never, { actor: fundador });

    const aLaPuerta = proyectarParaJugador(sesion.getState(), fundador, geo).asentamientosAvistados.find((a) => a.id === asentamientoId)!;
    expect(aLaPuerta.almacen?.['trigo']?.cantidad).toBeGreaterThan(0);

    const p = sesion.exportar();
    const lejos = GameSession.importar({
      ...p,
      state: { ...p.state, ejercitos: p.state.ejercitos.map((e) => ({ ...e, posicionActual: { x: e.posicionActual.x + 30, y: e.posicionActual.y } })) },
    });
    const desdeLejos = proyectarParaJugador(lejos.getState(), fundador, geo).asentamientosAvistados.find((a) => a.id === asentamientoId);
    expect(desdeLejos?.almacen, 'a 30 de la puerta no se ve el almacén').toBeUndefined();
  });

  it('la partida lleva el nombre que le puso el administrador', () => {
    expect(GameSession.crear('p1', { seed: 42, nombre: '  Micenas  ' }).getState().nombre).toBe('Micenas');
    expect(GameSession.crear('p2', { seed: 42 }).getState().nombre).toBeUndefined();
  });
});
