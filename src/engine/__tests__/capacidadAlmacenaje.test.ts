// La capacidad del almacén sigue a los Almacenes que funcionan (engine/almacen.ts, `aplicarCapacidadDeEdificio`).
// Salió en el batch de la Era I (2026-09-26): la capacidad solo se sumaba al terminar una obra, y como una conquista
// daña Almacenes que luego se reconstruyen, cada conquista la inflaba para siempre (302 800 por recurso con 8
// Almacenes, en vez de 2 800).
import { describe, expect, it } from 'vitest';
import type { Asentamiento } from '../../domain/types';
import { EDIFICIO_CATALOGO } from '../../constants';
import { aplicarConquista } from '../combate';
import { avanzarConstruccion, reclamosDeFuentes } from '../construction';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, instanteDeTest } from './fixtures';
import { TODAS_LAS_TECNOLOGIAS } from '../tecnologia';

const BASE = 10_000;
const BONUS = EDIFICIO_CATALOGO.almacen.capacidadPorRecursoAdicional;

describe('capacidad de almacenaje', () => {
  it('un Almacén dañado en una conquista deja de sumar, y al reconstruirse vuelve a sumar una sola vez', () => {
    const mapa = crearMapaDeterminista(7);
    const { asentamiento } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
    const almacen = Object.fromEntries(Object.entries(asentamiento.almacen).map(([r, v]) => [r, { ...v, cantidad: BASE, capacidad: BASE }]));
    // El primero por id, para que la conquista lo dañe (daña por orden de id); sin auto-construcción que meta ruido.
    let plaza: Asentamiento = {
      ...asentamiento,
      almacen,
      autoConstruccionPausada: true,
      edificios: [
        { id: '0-almacen', tipo: 'almacen', posicion: { x: 0, y: 0 }, estado: 'en_construccion', ambito: 'asentamiento', completaEn: instanteDeTest(1) },
        ...asentamiento.edificios,
      ],
    };
    const tick = (a: Asentamiento, t: number) => avanzarConstruccion(a, [], mapa, undefined, reclamosDeFuentes([a]), instanteDeTest(t), 0, TODAS_LAS_TECNOLOGIAS).asentamiento;
    const capacidadMadera = (a: Asentamiento) => a.almacen['madera']!.capacidad;

    plaza = tick(plaza, 1);
    expect(capacidadMadera(plaza)).toBe(BASE + BONUS);

    plaza = { ...plaza, almacen: { ...plaza.almacen, madera: { cantidad: BASE + BONUS, capacidad: BASE + BONUS } } }; // lleno
    plaza = aplicarConquista(plaza, 'faccion-2', instanteDeTest(2));
    expect(plaza.edificios.find((e) => e.id === '0-almacen')!.danado).toBe(true);
    expect(capacidadMadera(plaza)).toBe(BASE);
    // Lo que ya no cabe se pierde con el Almacén.
    expect(plaza.almacen['madera']!.cantidad).toBe(BASE);

    plaza = tick(plaza, 3); // arranca la reconstrucción
    plaza = tick(plaza, 100_000); // y la termina
    expect(plaza.edificios.find((e) => e.id === '0-almacen')!.estado).toBe('activo');
    expect(capacidadMadera(plaza)).toBe(BASE + BONUS);
  });
});
