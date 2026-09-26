// El NPC pide la subida de nivel (`session/npcGobernanza.ts`, Doc 4.5). Desde que el nivel ya no sube solo, un NPC
// que no la pidiera se quedaría en nivel 1 para siempre, y con él el batch: es el mismo agujero que tuvo el nivel 3
// con Barracón y Galería (`issues/nivel_3_inalcanzable_sin_jugador_humano.md`).
import { describe, expect, it } from 'vitest';
import { createRng } from '../../worldgen';
import { avanzarNpcGobernanza } from '../npcGobernanza';
import {
  contextoDeTest,
  crearEstadoDeTest,
  crearFacciones,
  crearMapaDeterminista,
  fundarAsentamientoDeTest,
  prepararParaSubirANivel2,
} from '../../engine/__tests__/fixtures';

describe('el NPC pide la subida de nivel', () => {
  it('en cuanto el motor dice que puede, arranca la obra', () => {
    const mapa = crearMapaDeterminista(7);
    const { asentamiento, facciones } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
    const listo = prepararParaSubirANivel2(asentamiento, mapa);

    const r = avanzarNpcGobernanza(crearEstadoDeTest([listo], facciones), mapa, contextoDeTest(1, createRng(5)), {});

    expect(r.estado.asentamientos[0]!.ascenso?.nivelObjetivo).toBe(2);
  });

  it('si no cumple los gates, no la pide', () => {
    const mapa = crearMapaDeterminista(7);
    const { asentamiento, facciones } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);

    const r = avanzarNpcGobernanza(crearEstadoDeTest([asentamiento], facciones), mapa, contextoDeTest(1, createRng(5)), {});

    expect(r.estado.asentamientos[0]!.ascenso).toBeUndefined();
  });

  it('dos plazas de la misma Facción listas a la vez no se quedan la misma única plaza de nivel 2', () => {
    const mapa = crearMapaDeterminista(7);
    // Facción de nivel 2: puede tener dos asentamientos (`CAP_FUNDACION_POR_NIVEL[1]`) y sigue teniendo UNA sola
    // plaza de nivel 2 (`CUPO_NIVEL_ASENTAMIENTO.maxNivel2[1]`).
    const facciones = crearFacciones().map((f) => (f.id === 'faccion-1' ? { ...f, nivel: 2 } : f));
    const uno = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);
    const dos = fundarAsentamientoDeTest(mapa, uno.facciones, 'faccion-1', [uno.asentamiento]);
    const listos = [prepararParaSubirANivel2(uno.asentamiento, mapa), prepararParaSubirANivel2(dos.asentamiento, mapa)];

    const r = avanzarNpcGobernanza(crearEstadoDeTest(listos, dos.facciones), mapa, contextoDeTest(1, createRng(5)), {});

    expect(r.estado.asentamientos.filter((a) => a.ascenso).length).toBe(1);
  });
});
