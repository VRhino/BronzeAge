// Caravanas comerciales y de Fundación (`engine/trade.ts`, `engine/expansion.ts`) — fusiona lo que antes
// eran 4 archivos separados (revisión de duplicación 2026-08-25: mismo subsistema, helpers casi idénticos
// entre ellos, ver `tradeFixtures.ts`), uno por cada bug/regresión real que motivó su prueba. Cada `describe`
// documenta su origen tal como lo hacía el archivo del que viene.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, Caravana, CaminoComercial, Faccion, Point } from '../../domain/types';
import { CARAVANA_COOLDOWN } from '../../constants';
import { avanzarComercio, construirCaravanaComercial, proponerTrueque, CaravanaInvalidaError } from '../trade';
import { lanzarCaravanaFundacion, ExpansionInvalidaError } from '../expansion';
import { almacenSintetico, caravanaComercialCasiLlegando, mapaSintetico } from './tradeFixtures';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, instanteDeTest, posicionRecomendable } from './fixtures';

// ---------------------------------------------------------------------------------------------------------
// Orientación de la ruta al reutilizar un Camino Comercial existente
// (antes `caravana_camino_direccion.test.ts`)
//
// Regresión (bug real detectado con un save del usuario, tick 69): un Camino Comercial ya construido entre
// dos asentamientos se reutiliza para AMBAS direcciones de trueque (Doc 1.6, `asignarCaravanasATrueque`,
// engine/trade.ts). `buscarCamino` empareja el par en cualquier orden pero devuelve `puntos` siempre en el
// orden en que se guardó (`asentamientoAId` -> `asentamientoBId`, ver engine/caminos.ts) — si no se reorienta,
// una caravana que viaja en sentido CONTRARIO recibe una polilínea invertida: `progreso: 0` cae en el punto
// del camino más cercano al DESTINO, no al propio origen, y la caravana "salta" hasta allí en su primer paso
// de movimiento (ver `avanzarPosicionEnRuta`, engine/movimiento.ts, que siempre mide desde `ruta[0]`).
// ---------------------------------------------------------------------------------------------------------
describe('orientación de la ruta al reutilizar un Camino Comercial existente', () => {
  function asentamientoConMercado(id: string, posicion: Point, recursos: Record<string, number>): Asentamiento {
    return {
      id,
      faccionId: `faccion-${id}`,
      posicion,
      almacen: almacenSintetico(recursos),
      politicasActivas: [],
      edificios: [{ id: `mercado-${id}`, tipo: 'mercado', posicion, estado: 'activo', nivelInterno: 1 }],
    } as unknown as Asentamiento;
  }

  it('una caravana que viaja B -> A recibe la polilínea invertida, no la original A -> B', () => {
    const posA: Point = { x: 0, y: 0 };
    const posB: Point = { x: 1000, y: 0 };
    const a = asentamientoConMercado('A', posA, { madera: 50, cobre: 200 });
    const b = asentamientoConMercado('B', posB, { madera: 50, oro: 1000 });

    // Camino ya construido A -> B (mismo patrón que un save real: la infraestructura persiste
    // independientemente de qué lado la use después) — sus `puntos` están ordenados desde `posA` hacia `posB`.
    const caminoAB: CaminoComercial = { id: 'camino-A-B', asentamientoAId: 'A', asentamientoBId: 'B', puntos: [posA, { x: 500, y: 0 }, posB] };

    // Cada asentamiento construye su propia caravana (mismo patrón que el save real: una por lado).
    const { asentamiento: aTrasConstruir, caravana: caravanaA } = construirCaravanaComercial(a, [], instanteDeTest(0), 0);
    const { asentamiento: bTrasConstruir, caravana: caravanaB } = construirCaravanaComercial(b, [caravanaA], instanteDeTest(0), 1);

    // Trueque A<->B: A entrega cobre a B (caravana de A viaja A->B, a favor del camino), B entrega oro a A
    // (caravana de B viaja B->A, EN CONTRA del orden guardado del camino) — el mismo patrón de dos trueques
    // opuestos por el mismo camino que expuso el bug en la partida real.
    const acuerdo = proponerTrueque([aTrasConstruir, bTrasConstruir], 'A', 'B', 'cobre', 'oro', 50, 20, instanteDeTest(0), 0);

    const resultado = avanzarComercio(
      [aTrasConstruir, bTrasConstruir],
      [] as Faccion[],
      [caravanaA, caravanaB],
      [acuerdo],
      mapaSintetico(),
      [caminoAB],
      instanteDeTest(1)
    );

    const cA = resultado.caravanas.find((c) => c.id === caravanaA.id)!;
    const cB = resultado.caravanas.find((c) => c.id === caravanaB.id)!;

    expect(cA.estado).toBe('en_transito');
    expect(cB.estado).toBe('en_transito');

    // Caravana de A (viaja A->B, a favor del camino): ruta empieza en su propio origen.
    expect(cA.ruta?.[0]).toEqual(posA);
    // Caravana de B (viaja B->A, en CONTRA del camino guardado): antes del fix, esto también daba `posA`
    // (el bug) — corregido, debe empezar en su propio origen, `posB`.
    expect(cB.ruta?.[0]).toEqual(posB);

    // Y por tanto, en el primer tick de movimiento, NINGUNA de las dos aparece ya en la posición de la otra.
    expect(cA.posicionActual).not.toEqual(cB.posicionActual);
    // La caravana de B se movió gradualmente desde su origen (B, x=1000), no saltó hacia x=0.
    expect(cB.posicionActual.x).toBeGreaterThan(900);
  });
});

// ---------------------------------------------------------------------------------------------------------
// Cooldown de creación de caravanas (antes `caravana_cooldown.test.ts`)
//
// A petición del usuario: tras crear una caravana (Fundación o comercial) desde un asentamiento, hay que
// esperar `CARAVANA_COOLDOWN.cooldownMinutos` ticks antes de poder crear otra desde el mismo asentamiento —
// evita spam de creación cuando una caravana recién salida es destruida (bandidos, intercepción) y el
// cupo/recursos vuelven a estar disponibles de inmediato. Ver `Docs/3_Sistema_Economico_y_Comercio.md` y
// `engine/asentamientoQuery.ts` (`puedeCrearCaravana`).
// ---------------------------------------------------------------------------------------------------------
describe('cooldown de creación de caravanas', () => {
  function asentamientoConMercado(): Asentamiento {
    const mapa = crearMapaDeterminista(1);
    const facciones = crearFacciones();
    const { asentamiento } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);
    return {
      ...asentamiento,
      almacen: { ...asentamiento.almacen, madera: { cantidad: 1000, capacidad: 2000 } },
      edificios: [
        ...asentamiento.edificios,
        { id: 'mercado-test', tipo: 'mercado', posicion: { x: 100, y: 100 }, estado: 'activo', ambito: 'asentamiento' },
      ],
    };
  }

  describe('caravana comercial (construirCaravanaComercial)', () => {
    it('rechaza crear una segunda caravana antes de que pase el cooldown', () => {
      const asentamiento = asentamientoConMercado();
      const r1 = construirCaravanaComercial(asentamiento, [], instanteDeTest(0), 0);
      expect(() => construirCaravanaComercial(r1.asentamiento, [r1.caravana], instanteDeTest(1), 1)).toThrow(CaravanaInvalidaError);
    });

    it('permite crear otra en cuanto pasa CARAVANA_COOLDOWN.cooldownMinutos ticks', () => {
      const asentamiento = asentamientoConMercado();
      const r1 = construirCaravanaComercial(asentamiento, [], instanteDeTest(0), 0);
      expect(() =>
        construirCaravanaComercial(r1.asentamiento, [r1.caravana], instanteDeTest(CARAVANA_COOLDOWN.cooldownMinutos), 1)
      ).not.toThrow();
    });

    it('registra el instante de creación en ultimaCaravanaCreadaEn', () => {
      const asentamiento = asentamientoConMercado();
      const r1 = construirCaravanaComercial(asentamiento, [], instanteDeTest(5), 0);
      expect(r1.asentamiento.ultimaCaravanaCreadaEn).toBe(instanteDeTest(5));
    });

    it('un asentamiento que nunca creó ninguna no está en cooldown', () => {
      const asentamiento = asentamientoConMercado();
      expect(() => construirCaravanaComercial(asentamiento, [], instanteDeTest(0), 0)).not.toThrow();
    });
  });

  describe('Caravana de Fundación (lanzarCaravanaFundacion)', () => {
    function contextoNivel2() {
      const mapa = crearMapaDeterminista(1);
      const facciones = crearFacciones().map((f) => (f.id === 'faccion-1' ? { ...f, nivel: 3 } : f));
      const { asentamiento, facciones: trasFundar } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);
      const abundante: Asentamiento = {
        ...asentamiento,
        nivel: 2,
        nivelActual: 2,
        almacen: Object.fromEntries(
          Object.entries(asentamiento.almacen).map(([recurso, item]) => [recurso, { ...item, cantidad: 5000, capacidad: 5000 }])
        ),
      };
      const faccion = trasFundar.find((f) => f.id === 'faccion-1')!;
      const destino = posicionRecomendable(mapa, [abundante]);
      return { mapa, asentamiento: abundante, faccion, destino };
    }

    it('rechaza lanzar una segunda Caravana de Fundación antes de que pase el cooldown', () => {
      const { mapa, asentamiento, faccion, destino } = contextoNivel2();
      const r1 = lanzarCaravanaFundacion(mapa, asentamiento, faccion, destino, [asentamiento], [], 1, instanteDeTest(0), 0);
      expect(() =>
        lanzarCaravanaFundacion(mapa, r1.origenActualizado, faccion, destino, [asentamiento], [r1.caravana], 1, instanteDeTest(0), 1)
      ).toThrow(ExpansionInvalidaError);
    });

    it('permite lanzar otra en cuanto pasa CARAVANA_COOLDOWN.cooldownMinutos ticks', () => {
      const { mapa, asentamiento, faccion, destino } = contextoNivel2();
      const r1 = lanzarCaravanaFundacion(mapa, asentamiento, faccion, destino, [asentamiento], [], 1, instanteDeTest(0), 0);
      expect(() =>
        lanzarCaravanaFundacion(
          mapa,
          r1.origenActualizado,
          faccion,
          destino,
          [asentamiento],
          [r1.caravana],
          1,
          instanteDeTest(CARAVANA_COOLDOWN.cooldownMinutos),
          1
        )
      ).not.toThrow();
    });
  });
});

// ---------------------------------------------------------------------------------------------------------
// Retorno real de una caravana comercial tras entregar (antes `caravana_retorno.test.ts`)
//
// Regresión (a petición del usuario): una caravana comercial que entrega en `destino` ya NO se teletransporta
// de vuelta a `origen` — debe recorrer la ruta de vuelta tick a tick, igual que fue. Antes, `avanzarCaravanas`
// (engine/trade.ts) la reseteaba a `estado: 'disponible'` con `posicionActual: origen.posicion` en el MISMO
// tick de la entrega.
// ---------------------------------------------------------------------------------------------------------
describe('retorno real de una caravana comercial tras entregar', () => {
  const origen = { id: 'origen', faccionId: 'faccion-1', posicion: { x: 0, y: 0 }, almacen: almacenSintetico({ oro: 0 }), politicasActivas: [] } as unknown as Asentamiento;
  const destino = { id: 'destino', faccionId: 'faccion-1', posicion: { x: 1000, y: 0 }, almacen: almacenSintetico({ oro: 0 }), politicasActivas: [] } as unknown as Asentamiento;

  function avanzar(caravanas: Caravana[]) {
    return avanzarComercio([origen, destino], [] as Faccion[], caravanas, [], mapaSintetico(), [], instanteDeTest(1));
  }

  it('al entregar pasa a "retornando" en destino, NO a "disponible" en origen', () => {
    const resultado = avanzar([caravanaComercialCasiLlegando(origen, destino)]);
    const caravana = resultado.caravanas.find((c) => c.id === 'caravana-1')!;

    expect(caravana.estado).toBe('retornando');
    expect(caravana.posicionActual).toEqual(destino.posicion);
    expect(caravana.progreso).toBe(0);
  });

  it('en el siguiente tick avanza gradualmente hacia origen, sin saltar directo a él', () => {
    const trasEntrega = avanzar([caravanaComercialCasiLlegando(origen, destino)]).caravanas;
    const trasUnTick = avanzar(trasEntrega).caravanas.find((c) => c.id === 'caravana-1')!;

    expect(trasUnTick.estado).toBe('retornando');
    expect(trasUnTick.posicionActual.x).toBeLessThan(destino.posicion.x);
    expect(trasUnTick.posicionActual.x).toBeGreaterThan(origen.posicion.x);
    expect(trasUnTick.progreso).toBeGreaterThan(0);
    expect(trasUnTick.progreso).toBeLessThan(1);
  });

  it('no puede reasignarse a un nuevo envío mientras está retornando (no cuenta como "disponible")', () => {
    const trasEntrega = avanzar([caravanaComercialCasiLlegando(origen, destino)]).caravanas;
    const caravana = trasEntrega.find((c) => c.id === 'caravana-1')!;
    expect(caravana.estado).not.toBe('disponible');
  });

  it('tras completar el viaje de vuelta, llega de verdad a origen y solo entonces vuelve a "disponible"', () => {
    let caravanas = avanzar([caravanaComercialCasiLlegando(origen, destino)]).caravanas;
    for (let i = 0; i < 200 && caravanas.find((c) => c.id === 'caravana-1')!.estado !== 'disponible'; i++) {
      caravanas = avanzar(caravanas).caravanas;
    }
    const caravanaFinal = caravanas.find((c) => c.id === 'caravana-1')!;

    expect(caravanaFinal.estado).toBe('disponible');
    expect(caravanaFinal.posicionActual).toEqual(origen.posicion);
  });
});

// ---------------------------------------------------------------------------------------------------------
// Reuso de caravana propia a través de varios envíos del mismo trueque (antes `caravana_trueque_reuso.test.ts`)
//
// Integración: una caravana comercial que hace VARIOS envíos del mismo trueque (a petición del usuario) —
// verifica que `cantidadEntregadaA` se actualiza correctamente en cada entrega real (Correcciones, punto
// "revisar si... se actualiza correctamente en el trueque activo") y que la MISMA caravana (mismo `id`) hace
// el viaje de vuelta y se reutiliza para el segundo envío en vez de destruirse o multiplicarse (Correcciones,
// punto "las caravanas no se destruyen al llegar a su destino, son reutilizables").
// ---------------------------------------------------------------------------------------------------------
describe('reuso de caravana propia a través de varios envíos del mismo trueque', () => {
  function asentamientoSintetico(id: string, posicion: Point, recursos: Record<string, number>, conMercado: boolean): Asentamiento {
    return {
      id,
      faccionId: `faccion-${id}`,
      posicion,
      almacen: almacenSintetico(recursos),
      politicasActivas: [],
      edificios: conMercado
        ? [{ id: `mercado-${id}`, tipo: 'mercado', posicion, estado: 'activo', nivelInterno: 1 }]
        : [],
    } as unknown as Asentamiento;
  }

  it('la MISMA caravana entrega en dos viajes, actualiza el trueque cada vez, y vuelve de verdad entre medias', () => {
    // Distancia < 45 (ESPACIADO_MALLA, world/rutas.ts): `calcularRuta` devuelve [origen, destino] directo, sin
    // necesitar una malla de pathfinding real — mismo truco que mantiene el fixture mínimo.
    const origenPos: Point = { x: 0, y: 0 };
    const destinoPos: Point = { x: 30, y: 0 };
    const mapa = mapaSintetico({ limites: { ancho: 1000, alto: 1000 } });

    const origen0 = asentamientoSintetico('origen', origenPos, { madera: 50, piedra: 200 }, true);
    const destino0 = asentamientoSintetico('destino', destinoPos, { oro: 1000 }, false);

    const { asentamiento: origenTrasConstruir, caravana } = construirCaravanaComercial(origen0, [], instanteDeTest(0), 0);
    expect(caravana.estado).toBe('disponible');

    // 100 piedra pactadas, capacidad de una caravana comercial = 60 (CARAVANA_CATALOGO.comercial.capacidad):
    // fuerza DOS envíos con la misma caravana en vez de uno.
    const acuerdo = proponerTrueque([origenTrasConstruir, destino0], 'origen', 'destino', 'piedra', 'oro', 100, 1, instanteDeTest(0), 0);

    let asentamientos = [origenTrasConstruir, destino0];
    let caravanas: Caravana[] = [caravana];
    let acuerdos = [acuerdo];
    const facciones: Faccion[] = [];

    function tick(n: number) {
      const resultado = avanzarComercio(asentamientos, facciones, caravanas, acuerdos, mapa, [], instanteDeTest(n));
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
