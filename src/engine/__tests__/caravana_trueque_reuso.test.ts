// Integración: una caravana comercial que hace VARIOS envíos del mismo trueque (a petición del usuario) —
// verifica que `cantidadEntregadaA` se actualiza correctamente en cada entrega real (Correcciones, punto
// "revisar si... se actualiza correctamente en el trueque activo") y que la MISMA caravana (mismo `id`) hace
// el viaje de vuelta y se reutiliza para el segundo envío en vez de destruirse o multiplicarse (Correcciones,
// punto "las caravanas no se destruyen al llegar a su destino, son reutilizables").

import { describe, expect, it } from 'vitest';
import type { Asentamiento, Caravana, Faccion, Point, RecursoAlmacenado } from '../../domain/types';
import type { Mapa } from '../../world/mapa';
import { avanzarComercio, construirCaravanaComercial, proponerTrueque } from '../trade';

function almacen(recursos: Record<string, number>): Record<string, RecursoAlmacenado> {
  const out: Record<string, RecursoAlmacenado> = {};
  for (const [r, cantidad] of Object.entries(recursos)) out[r] = { cantidad, capacidad: 100000 };
  return out;
}

function asentamiento(id: string, posicion: Point, recursos: Record<string, number>, conMercado: boolean): Asentamiento {
  return {
    id,
    faccionId: `faccion-${id}`,
    posicion,
    almacen: almacen(recursos),
    politicasActivas: [],
    edificios: conMercado
      ? [{ id: `mercado-${id}`, tipo: 'mercado', posicion, estado: 'activo', ticksRestantes: 0, nivelInterno: 1 }]
      : [],
  } as unknown as Asentamiento;
}

// Distancia < 45 (ESPACIADO_MALLA, world/rutas.ts): `calcularRuta` devuelve [origen, destino] directo, sin
// necesitar una malla de pathfinding real — mismo truco que mantiene el fixture mínimo.
const origenPos: Point = { x: 0, y: 0 };
const destinoPos: Point = { x: 30, y: 0 };
const mapaLlano: Mapa = { costeEnPunto: () => 1, listarChokepoints: () => [], limites: { ancho: 1000, alto: 1000 } } as unknown as Mapa;

describe('reuso de caravana propia a través de varios envíos del mismo trueque', () => {
  it('la MISMA caravana entrega en dos viajes, actualiza el trueque cada vez, y vuelve de verdad entre medias', () => {
    const origen0 = asentamiento('origen', origenPos, { madera: 50, piedra: 200 }, true);
    const destino0 = asentamiento('destino', destinoPos, { oro: 1000 }, false);

    const { asentamiento: origenTrasConstruir, caravana } = construirCaravanaComercial(origen0, [], 0, 0);
    expect(caravana.estado).toBe('disponible');

    // 100 piedra pactadas, capacidad de una caravana comercial = 60 (CARAVANA_CATALOGO.comercial.capacidad):
    // fuerza DOS envíos con la misma caravana en vez de uno.
    const acuerdo = proponerTrueque([origenTrasConstruir, destino0], 'origen', 'destino', 'piedra', 'oro', 100, 1, 0, 0);

    let asentamientos = [origenTrasConstruir, destino0];
    let caravanas: Caravana[] = [caravana];
    let acuerdos = [acuerdo];
    const facciones: Faccion[] = [];

    function tick(n: number) {
      const resultado = avanzarComercio(asentamientos, facciones, caravanas, acuerdos, mapaLlano, [], [], n);
      asentamientos = resultado.asentamientos;
      caravanas = resultado.caravanas;
      acuerdos = resultado.acuerdos;
      return resultado;
    }

    // Tick 1: se asigna la caravana disponible al primer envío (sale con 60 piedra, tope de capacidad).
    tick(1);
    expect(caravanas).toHaveLength(1);
    expect(caravanas[0]!.estado).toBe('en_transito');
    expect(caravanas[0]!.contenido['piedra']).toBe(60);

    // Avanza hasta que entregue el primer envío (distancia 30, velocidad 16/tick -> llega en tick 2).
    let entregoUna = false;
    for (let t = 2; t < 20 && !entregoUna; t++) {
      tick(t);
      if (acuerdos[0]!.cantidadEntregadaA > 0) entregoUna = true;
    }
    expect(entregoUna).toBe(true);
    expect(acuerdos[0]!.cantidadEntregadaA).toBe(60);
    expect(acuerdos[0]!.estado).toBe('activo'); // aún falta el segundo envío (40 piedra) — no "cumplido" todavía.

    // Justo tras entregar: la MISMA caravana (mismo id, no una nueva) está "retornando", no "disponible" — no
    // puede recibir un segundo envío hasta volver de verdad a origen.
    expect(caravanas).toHaveLength(1);
    expect(caravanas[0]!.id).toBe(caravana.id);
    expect(caravanas[0]!.estado).toBe('retornando');

    // Mientras retorna, nada la reasigna al resto pendiente del trueque (40 piedra) aunque origen tenga stock:
    // sigue "retornando", a medio camino (ni 0 ni 1) — prueba de que de verdad viaja, no se teletransporta.
    tick(6);
    expect(caravanas[0]!.estado).toBe('retornando');
    expect(caravanas[0]!.progreso).toBeGreaterThan(0);
    expect(caravanas[0]!.progreso).toBeLessThan(1);

    // Avanza hasta que vuelva a origen — el mismo tick en que llega, `asignarCaravanasATrueque` la reasigna
    // de inmediato al resto pendiente (40 piedra) y sale por segunda vez: sigue siendo la MISMA caravana.
    let salioSegundaVez = false;
    for (let t = 7; t < 20 && !salioSegundaVez; t++) {
      tick(t);
      if (caravanas[0]!.estado === 'en_transito') salioSegundaVez = true;
    }
    expect(salioSegundaVez).toBe(true);
    expect(caravanas).toHaveLength(1); // ninguna caravana nueva se creó ni la original desapareció.
    expect(caravanas[0]!.id).toBe(caravana.id);
    expect(caravanas[0]!.contenido['piedra']).toBe(40);

    // Entrega el segundo envío: el trueque queda completo en 100/100 con la MISMA caravana, en dos viajes reales.
    let entregoDos = false;
    for (let t = 21; t < 30 && !entregoDos; t++) {
      tick(t);
      if (acuerdos[0]!.cantidadEntregadaA >= 100) entregoDos = true;
    }
    expect(entregoDos).toBe(true);
    expect(acuerdos[0]!.cantidadEntregadaA).toBe(100);
    expect(caravanas).toHaveLength(1);
    expect(caravanas[0]!.id).toBe(caravana.id);
    expect(caravanas[0]!.estado).toBe('retornando'); // vuelve a casa una última vez, ya sin más pendiente.

    // Termina el último retorno de verdad en origen — nada queda pendiente, así que se queda "disponible".
    let terminoEnCasa = false;
    for (let t = 31; t < 40 && !terminoEnCasa; t++) {
      tick(t);
      if (caravanas[0]!.estado === 'disponible') terminoEnCasa = true;
    }
    expect(terminoEnCasa).toBe(true);
    expect(caravanas).toHaveLength(1);
    expect(caravanas[0]!.id).toBe(caravana.id);
    expect(caravanas[0]!.posicionActual).toEqual(origenPos);
  });
});
