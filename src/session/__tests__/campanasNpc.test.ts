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

/** Una plaza NPC de nivel 2 lista para salir de campaña, y una plaza rival cerca con `guarnicion` soldados de guardia. */
function campanaContra(guarnicion: number) {
  const mapa = crearMapaDeterminista(7);
  const uno = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
  const dos = fundarAsentamientoDeTest(mapa, uno.facciones, 'faccion-2', [uno.asentamiento]);
  const origen = conAlmacen({ ...uno.asentamiento, nivel: 2, nivelActual: 2 }, { madera: 2000, trigo: 5000 });
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
  const r = avanzarNpcGobernanza(crearEstadoDeTest([origen, rival], dos.facciones, { heroes }), mapa, contextoDeTest(1, createRng(5)), {
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
