// El NPC que hace la guerra de verdad (`session/npcGobernanza.ts`, decisión del usuario 2026-09-26). Medido antes en
// el batch: 2 172 conquistas en la Era I, todas de columnas de milicia entrando en plazas vacías — cero guarnición,
// 29 de 36 plazas sin un héroe dentro y los 60 escuadrones de milicia. Cada caso fija una de las tres causas.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, Edificio, EdificioTipo, Heroe } from '../../domain/types';
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
} from '../../engine/__tests__/fixtures';

function activo(id: string, tipo: EdificioTipo): Edificio {
  return { id, tipo, posicion: { x: 0, y: 0 }, estado: 'activo', ambito: 'asentamiento', nivelInterno: 1 };
}

function conAlmacen(a: Asentamiento, cantidades: Record<string, number>): Asentamiento {
  const almacen = { ...a.almacen };
  for (const [recurso, cantidad] of Object.entries(cantidades)) almacen[recurso] = { cantidad, capacidad: 100_000 };
  return { ...a, almacen };
}

describe('el NPC defiende sus plazas', () => {
  it('una plaza con fundadores sin héroe los ve nacer dentro, y reclutan la mejor tropa que pueden (no milicia)', () => {
    const mapa = crearMapaDeterminista(7);
    const { asentamiento, facciones } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
    const fundador = asentamiento.heroesFundadoresIds[0]!;
    // Galería de tiro y armas de madera: puede hacer honderos (escalón 2), que el NPC prefiere a la milicia (1).
    const plaza = conAlmacen(
      { ...asentamiento, poblacion: { pesants: 400, artesanos: 0, nobleza: 0 }, edificios: [...asentamiento.edificios, activo('g', 'galeriaDeTiro')] },
      { madera: 2000, trigo: 5000, armaMadera: 200 }
    );

    const r = avanzarNpcGobernanza(crearEstadoDeTest([plaza], facciones), mapa, contextoDeTest(1, createRng(5)), {});

    const heroe = r.estado.heroes.find((h) => h.id === fundador);
    expect(heroe, 'el fundador tiene que existir como héroe').toBeDefined();
    expect(heroe!.controlador).toBe('bot');
    expect(heroe!.ubicacion).toEqual({ tipo: 'asentamiento', asentamientoId: plaza.id });
    expect(heroe!.escuadrones.map((e) => e.tropaId)).toContain('honderos');
  });

  it('a un jugador real NO le nacen héroes del aire: solo a las Facciones que gobierna el NPC', () => {
    const mapa = crearMapaDeterminista(7);
    const { asentamiento, facciones } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
    const fundador = asentamiento.heroesFundadoresIds[0]!;

    // En la partida real el NPC solo gobierna las Facciones de `faccionesIds`; la del jugador no está.
    const r = avanzarNpcGobernanza(crearEstadoDeTest([asentamiento], facciones), mapa, contextoDeTest(1, createRng(5)), {
      faccionesIds: ['faccion-2'],
    });

    expect(r.estado.heroes.find((h) => h.id === fundador)).toBeUndefined();
  });

  it('mete su escuadra más fuerte en la guarnición y deja fuera la última', () => {
    const mapa = crearMapaDeterminista(7);
    const { asentamiento, facciones } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
    const id = asentamiento.heroesFundadoresIds[0]!;
    // Barracón + Galería de nivel 1: cupo 7 + 7 = 14, lo justo para los honderos (escalón 2).
    const plaza = { ...asentamiento, edificios: [...asentamiento.edificios, activo('b', 'barracon'), activo('g', 'galeriaDeTiro')] };
    const bot: Heroe = heroeDePrueba(id, { tipo: 'asentamiento', asentamientoId: plaza.id }, {
      controlador: 'bot',
      escuadrones: [escuadronDePrueba('mil', id, 'milicia_lanceros', 25), escuadronDePrueba('hon', id, 'honderos', 20)],
    });

    const r = avanzarNpcGobernanza(crearEstadoDeTest([plaza], facciones, { heroes: [bot] }), mapa, contextoDeTest(1, createRng(5)), {});

    const escuadras = r.estado.heroes.find((h) => h.id === id)!.escuadrones;
    expect(escuadras.find((e) => e.id === 'hon')!.enGuarnicion).toBe(true);
    expect(escuadras.find((e) => e.id === 'mil')!.enGuarnicion).toBe(false);
  });

  it('sin Barracón ni Galería no hay cupo, y no guarnece nada', () => {
    const mapa = crearMapaDeterminista(7);
    const { asentamiento, facciones } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
    const id = asentamiento.heroesFundadoresIds[0]!;
    const bot: Heroe = heroeDePrueba(id, { tipo: 'asentamiento', asentamientoId: asentamiento.id }, {
      controlador: 'bot',
      escuadrones: [escuadronDePrueba('mil', id, 'milicia_lanceros', 25), escuadronDePrueba('hon', id, 'honderos', 20)],
    });

    const r = avanzarNpcGobernanza(crearEstadoDeTest([asentamiento], facciones, { heroes: [bot] }), mapa, contextoDeTest(1, createRng(5)), {});

    expect(r.estado.heroes.find((h) => h.id === id)!.escuadrones.some((e) => e.enGuarnicion)).toBe(false);
  });
});
