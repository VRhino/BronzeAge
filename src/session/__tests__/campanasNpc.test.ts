// Las campañas del NPC (`session/npcGobernanza.ts`, decisiones del usuario 2026-09-27), medidas en la Era I: se
// estrellaba una y otra vez contra guarniciones cinco veces más fuertes (2 256 asedios contra plazas defendidas, el
// atacante ganó el 0,4 %), y lo que conquistaba lo dejaba vacío (una plaza cambió de manos 204 veces).
import { describe, expect, it } from 'vitest';
import type { Asentamiento, Ejercito, Heroe } from '../../domain/types';
import { createRng } from '../../worldgen';
import { avanzarNpcGobernanza } from '../npcGobernanza';
import {
  contextoDeTest,
  crearEstadoDeTest,
  crearFacciones,
  crearMapaDeterminista,
  escuadronDePrueba,
  fundarAsentamientoDeTest,
  heroeDePrueba,
  instanteDeTest,
} from '../../engine/__tests__/fixtures';

function conAlmacen(a: Asentamiento, cantidades: Record<string, number>): Asentamiento {
  const almacen = { ...a.almacen };
  for (const [recurso, cantidad] of Object.entries(cantidades)) almacen[recurso] = { cantidad, capacidad: 100_000 };
  return { ...a, almacen };
}

/** Una plaza NPC de nivel 2 lista para salir de campaña —con un segundo residente, para que quien sale pueda
 * quedarse en lo que conquiste—, y una plaza rival cerca con `guarnicion` soldados de guardia. La Facción rival
 * tiene otra plaza lejos, salvo con `rivalConUnaSolaPlaza`. */
function campanaContra(guarnicion: number, otrosResidentes: string[] = ['vecino'], rivalConUnaSolaPlaza = false) {
  const mapa = crearMapaDeterminista(7);
  const uno = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
  const dos = fundarAsentamientoDeTest(mapa, uno.facciones, 'faccion-2', [uno.asentamiento]);
  const origen = conAlmacen({ ...uno.asentamiento, nivel: 2, nivelActual: 2, casasCompradas: otrosResidentes }, { madera: 2000, trigo: 5000 });
  // El fixture la funda lejos (a 960); se acerca a una jornada corta para que quede al alcance del carro.
  const rival = { ...dos.asentamiento, posicion: { x: origen.posicion.x + 120, y: origen.posicion.y } };
  const atacanteId = origen.heroesFundadoresIds[0]!;
  const defensorId = dos.asentamiento.heroesFundadoresIds[0]!;
  const heroes: Heroe[] = [
    heroeDePrueba(atacanteId, { tipo: 'asentamiento', asentamientoId: origen.id }, {
      controlador: 'bot',
      escuadrones: [1, 2, 3, 4].map((n) => escuadronDePrueba(`a${n}`, atacanteId, 'milicia_lanceros', 25)),
    }),
    heroeDePrueba(defensorId, { tipo: 'asentamiento', asentamientoId: rival.id }, {
      controlador: 'bot',
      escuadrones: guarnicion > 0 ? [escuadronDePrueba('d1', defensorId, 'milicia_lanceros', guarnicion, { enGuarnicion: true })] : [],
    }),
  ];
  // Solo la Facción atacante es NPC: la defensora no toca su guarnición.
  const otraDelRival = { ...dos.asentamiento, id: `${dos.asentamiento.id}-b`, heroesFundadoresIds: [] };
  const plazas = rivalConUnaSolaPlaza ? [origen, rival] : [origen, rival, otraDelRival];
  const r = avanzarNpcGobernanza(crearEstadoDeTest(plazas, dos.facciones, { heroes }), mapa, contextoDeTest(1, createRng(5)), {
    faccionesIds: ['faccion-1'],
  });
  return r.stats.campanasLanzadas;
}

describe('campañas del NPC', () => {
  it('sale contra una plaza rival que puede ganar', () => {
    expect(campanaContra(0)).toBe(1);
  });

  it('no sale contra una guarnición que no puede vencer', () => {
    expect(campanaContra(1000)).toBe(0);
  });

  it('no sale contra la última plaza de una Facción: la haría desaparecer', () => {
    expect(campanaContra(0, ['vecino'], true)).toBe(0);
  });

  it('no sale de una casa con un solo residente: no podría quedarse en lo que conquistara', () => {
    expect(campanaContra(0, [])).toBe(0);
  });
});

describe('quien conquista se queda', () => {
  it('la columna acampada a la puerta de la plaza que acaba de conquistar se muda a ella y entra', () => {
    const mapa = crearMapaDeterminista(7);
    const facciones = crearFacciones().map((f) => (f.id === 'faccion-1' ? { ...f, nivel: 2 } : f));
    const uno = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);
    const dos = fundarAsentamientoDeTest(mapa, uno.facciones, 'faccion-1', [uno.asentamiento]);
    const lider = uno.asentamiento.heroesFundadoresIds[0]!;
    // Su casa tiene otro residente, así que mudarse no la vacía; la conquistada ya no tiene a nadie.
    const casa: Asentamiento = { ...uno.asentamiento, casasCompradas: ['vecino'] };
    const conquistada: Asentamiento = { ...dos.asentamiento, heroesFundadoresIds: [], casasCompradas: [] };
    const columna: Ejercito = {
      id: 'col-1',
      faccionId: 'faccion-1',
      origenAsentamientoId: casa.id,
      participantes: [{ heroeId: lider, unidoEn: instanteDeTest(0) }],
      tipo: 'ejercito',
      politicaDeUnion: 'rechazar',
      liderId: lider,
      escuadronIds: ['a1'],
      suministro: {},
      caravanasAdjuntasIds: [],
      objetivo: { tipo: 'asentamiento', id: conquistada.id },
      ruta: [],
      progreso: 1,
      posicionActual: conquistada.posicion,
      estado: 'estacionado',
    };
    const heroe = heroeDePrueba(lider, { tipo: 'columna', ejercitoId: 'col-1' }, {
      controlador: 'bot',
      escuadrones: [escuadronDePrueba('a1', lider, 'milicia_lanceros', 25, { contenedor: { tipo: 'ejercito', ejercitoId: 'col-1' } })],
    });

    const r = avanzarNpcGobernanza(
      crearEstadoDeTest([casa, conquistada], dos.facciones, { heroes: [heroe], ejercitos: [columna] }),
      mapa,
      contextoDeTest(1, createRng(5)),
      { lanzarCampanas: false }
    );

    const plaza = r.estado.asentamientos.find((a) => a.id === conquistada.id)!;
    expect(plaza.casasCompradas).toContain(lider);
    expect(r.estado.asentamientos.find((a) => a.id === casa.id)!.heroesFundadoresIds).not.toContain(lider);
    expect(r.estado.ejercitos.some((e) => e.id === 'col-1'), 'la columna se deshace dentro').toBe(false);
    const despues = r.estado.heroes.find((h) => h.id === lider)!;
    expect(despues.ubicacion).toEqual({ tipo: 'asentamiento', asentamientoId: conquistada.id });
    expect(despues.escuadrones.find((e) => e.id === 'a1')!.contenedor).toEqual({ tipo: 'campamento' });
  });
});

describe('las columnas personales de los bots vuelven a casa', () => {
  function columnaDe(heroeId: string, casa: Asentamiento, posicion: { x: number; y: number }): Ejercito {
    return {
      id: 'salida-1',
      faccionId: casa.faccionId,
      origenAsentamientoId: casa.id,
      participantes: [{ heroeId, unidoEn: instanteDeTest(0) }],
      tipo: 'personal',
      politicaDeUnion: 'rechazar',
      liderId: heroeId,
      escuadronIds: [],
      suministro: {},
      caravanasAdjuntasIds: [],
      objetivo: { tipo: 'punto', punto: posicion },
      ruta: [],
      progreso: 0,
      posicionActual: posicion,
      estado: 'estacionado',
    };
  }
  function tras(posicion: (casa: Asentamiento) => { x: number; y: number }) {
    const mapa = crearMapaDeterminista(7);
    const { asentamiento, facciones } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
    const id = asentamiento.heroesFundadoresIds[0]!;
    const heroe = heroeDePrueba(id, { tipo: 'columna', ejercitoId: 'salida-1' }, { controlador: 'bot' });
    return avanzarNpcGobernanza(
      crearEstadoDeTest([asentamiento], facciones, { heroes: [heroe], ejercitos: [columnaDe(id, asentamiento, posicion(asentamiento))] }),
      mapa,
      contextoDeTest(1, createRng(5)),
      { lanzarCampanas: false }
    ).estado;
  }

  it('a la puerta de su residencia, entra y la columna se deshace', () => {
    const e = tras((casa) => casa.posicion);
    expect(e.ejercitos).toEqual([]);
    expect(e.heroes[0]!.ubicacion).toEqual({ tipo: 'asentamiento', asentamientoId: e.asentamientos[0]!.id });
  });

  it('lejos de ella, se pone en marcha hacia casa', () => {
    const e = tras((casa) => ({ x: casa.posicion.x + 150, y: casa.posicion.y }));
    expect(e.ejercitos[0]!.estado).toBe('regresando');
  });
});
