// Cesión de una Facción al NPC de gobernanza (`session/npcGobernanza.ts`) desde la pestaña Facción.
//
// Lo que se blinda aquí es la garantía que hace usable la mecánica: el NPC juega SOLO las Facciones que el
// jugador le ha cedido (`GameState.faccionesNpcIds`) y no toca las que este sigue jugando a mano — ni
// asignándoles cargos, ni reservando su almacén, ni reclutando con sus residentes, ni pactando trueques que
// comprometan sus recursos (`proponerTrueque` no pide consentimiento al otro lado, ver `truequeDeSupervivencia`).

import { describe, expect, it } from 'vitest';
import { GameStore } from '../gameStore';
import type { Asentamiento, RecursoTipo } from '../../domain/types';

/** Dos Facciones, cada una con un asentamiento fundado en un emplazamiento viable y bien separado. */
function partidaConDosFacciones(): { store: GameStore; faccionNpcId: string; faccionManualId: string } {
  const store = new GameStore();
  store.crearFaccion('Facción NPC');
  store.crearFaccion('Facción Manual');
  const [faccionNpc, faccionManual] = store.getState().facciones;

  // Mismo barrido de grilla que `exportarImportar.test.ts`: el test no depende de que un punto elegido a mano
  // sea viable con la seed por defecto.
  const { ancho, alto } = store.getMapa().limites;
  const posiciones: { x: number; y: number }[] = [];
  for (let x = 40; x < ancho && posiciones.length < 2; x += 40) {
    for (let y = 40; y < alto && posiciones.length < 2; y += 40) {
      const candidata = { x, y };
      if (!store.viabilidadFundacion(candidata).recomendable) continue;
      if (posiciones.some((p) => Math.hypot(p.x - x, p.y - y) < 200)) continue;
      posiciones.push(candidata);
    }
  }
  expect(posiciones).toHaveLength(2);

  store.fundarAsentamiento(faccionNpc!.id, posiciones[0]!, 3);
  store.fundarAsentamiento(faccionManual!.id, posiciones[1]!, 3);
  expect(store.getState().asentamientos).toHaveLength(2);

  return { store, faccionNpcId: faccionNpc!.id, faccionManualId: faccionManual!.id };
}

function asentamientoDe(store: GameStore, faccionId: string): Asentamiento {
  const asentamiento = store.getState().asentamientos.find((a) => a.faccionId === faccionId);
  expect(asentamiento).toBeDefined();
  return asentamiento!;
}

describe('Facción controlada por NPC', () => {
  it('gobierna la Facción cedida y no toca la que juega el jugador', () => {
    const { store, faccionNpcId, faccionManualId } = partidaConDosFacciones();
    store.alternarFaccionNpc(faccionNpcId, true);
    for (let i = 0; i < 40; i++) store.avanzarTick();

    const npc = asentamientoDe(store, faccionNpcId);
    expect(npc.cargos.gobernadorId).toBeTruthy();
    expect(npc.cargos.tesoreroId).toBeTruthy();
    expect(npc.reservaManual?.madera ?? 0).toBeGreaterThan(0);

    // La Facción del jugador sigue exactamente como la dejó: sin cargos que él no haya nombrado, sin reservas
    // que él no haya puesto y sin escuadrones que él no haya reclutado. Su auto-construcción sí avanza — eso
    // es el motor (`avanzarConstruccion`), no el NPC.
    const manual = asentamientoDe(store, faccionManualId);
    expect(manual.cargos.gobernadorId).toBeNull();
    expect(manual.cargos.tesoreroId).toBeNull();
    expect(manual.reservaManual?.madera ?? 0).toBe(0);
    expect(manual.escuadrones).toHaveLength(0);
    expect(store.getState().caravanas.some((c) => c.origenAsentamientoId === manual.id)).toBe(false);

    // Ningún trueque del NPC puede comprometer recursos de la Facción del jugador (a petición del usuario:
    // el NPC solo pacta con otras Facciones NPC).
    const idsManual = store.getState().asentamientos.filter((a) => a.faccionId === faccionManualId).map((a) => a.id);
    for (const acuerdo of store.getState().acuerdos) {
      expect(idsManual).not.toContain(acuerdo.asentamientoAId);
      expect(idsManual).not.toContain(acuerdo.asentamientoBId);
    }
  });

  it('se cede y se retoma en caliente a mitad de partida', () => {
    const { store, faccionNpcId } = partidaConDosFacciones();

    for (let i = 0; i < 20; i++) store.avanzarTick();
    expect(asentamientoDe(store, faccionNpcId).cargos.gobernadorId).toBeNull();

    store.alternarFaccionNpc(faccionNpcId, true);
    expect(store.esFaccionNpc(faccionNpcId)).toBe(true);
    for (let i = 0; i < 5; i++) store.avanzarTick();
    expect(asentamientoDe(store, faccionNpcId).cargos.gobernadorId).toBeTruthy();

    // Retomar el control no deshace lo que el NPC ya hizo (es estado normal del juego): solo deja de decidir.
    store.alternarFaccionNpc(faccionNpcId, false);
    expect(store.esFaccionNpc(faccionNpcId)).toBe(false);
    const antes = asentamientoDe(store, faccionNpcId);
    for (let i = 0; i < 5; i++) store.avanzarTick();
    const despues = asentamientoDe(store, faccionNpcId);
    expect(despues.cargos.gobernadorId).toBe(antes.cargos.gobernadorId);
    expect(despues.escuadrones.length).toBe(antes.escuadrones.length);
  });

  it('la marca de NPC sobrevive a exportar/importar', () => {
    const { store, faccionNpcId } = partidaConDosFacciones();
    store.alternarFaccionNpc(faccionNpcId, true);
    for (let i = 0; i < 5; i++) store.avanzarTick();

    const otro = new GameStore();
    otro.importarSimulacion(store.exportarSimulacion());
    expect(otro.getState().faccionesNpcIds).toEqual([faccionNpcId]);
  });

  it('se funda a sí misma si se cede sin ningún asentamiento (reportado por el usuario: quedaba inerte)', () => {
    const store = new GameStore();
    store.crearFaccion('Facción NPC');
    const faccionId = store.getState().facciones[0]!.id;

    // A diferencia de `partidaConDosFacciones`, aquí NO se funda nada a mano: el jugador crea la Facción, la
    // marca NPC y avanza tick — el punto de partida real que reportó el fallo.
    store.alternarFaccionNpc(faccionId, true);
    expect(store.getState().asentamientos).toHaveLength(0);

    for (let i = 0; i < 10; i++) store.avanzarTick();

    const propios = store.getState().asentamientos.filter((a) => a.faccionId === faccionId);
    expect(propios).toHaveLength(1);
    expect(propios[0]!.cargos.gobernadorId).toBeTruthy();
  });

  it('funda su asentamiento inicial en un sitio con madera Y piedra alcanzables (a petición del usuario)', () => {
    const store = new GameStore();
    store.crearFaccion('Facción NPC Piedra');
    const faccionId = store.getState().facciones[0]!.id;

    store.alternarFaccionNpc(faccionId, true);
    for (let i = 0; i < 5 && store.getState().asentamientos.length === 0; i++) store.avanzarTick();

    const asentamiento = store.getState().asentamientos.find((a) => a.faccionId === faccionId);
    expect(asentamiento).toBeDefined();

    // Mismo radio que usa `evaluarViabilidadFundacion` para decidir viabilidad — se comprueba con la API
    // pública del mapa, no con el resultado de esa función, para no repetir la lógica que se está probando.
    const mapa = store.getMapa();
    const radio = asentamiento!.radioPotencial;
    expect(mapa.hayBosqueEnRadio(asentamiento!.posicion, radio)).toBe(true);
    const piedraEnRadio = mapa.nodosEnRadio(asentamiento!.posicion, radio).filter((n) => n.tipo === 'piedra');
    expect(piedraEnRadio.length).toBeGreaterThan(0);
  });

  it('prefiere un sitio con minerales extra a uno con solo madera y piedra, cuando el mapa ofrece ambos', () => {
    // Regresión directa del reporte del usuario: la primera versión se conformaba con el primer candidato
    // viable de un barrido grueso (paso 300, muy por encima de `ZONA_INFLUENCIA.radioInicial` = 30) y podía
    // devolver un punto con solo madera+piedra aunque el mapa tuviera de sobra sitios con algún mineral extra
    // cerca. Se prueba contra el mundo real por defecto (seed 1, el mismo que usa `new GameStore()`): si el
    // barrido denso encuentra AL MENOS un candidato con mineral extra en todo el mapa (lo cual, para esta
    // seed, es así — hay muchos), el elegido tiene que ser uno de ellos, no cualquiera con bonus 0.
    const store = new GameStore();
    const mapa = store.getMapa();

    // Referencia independiente: mismo criterio que la heurística bajo prueba, pero recorrido aparte (no se
    // llama a la función que se está verificando) — barre el mapa entero al paso más fino y cuenta cuántos
    // candidatos válidos (madera+piedra) tienen 1+ mineral extra, para saber si el mundo por defecto ofrece
    // margen de mejora sobre "solo piedra".
    let candidatosConBonus = 0;
    const otrosMinerales: RecursoTipo[] = ['cobre', 'estano', 'oro', 'livestock'];
    for (let x = 25; x < mapa.limites.ancho; x += 25) {
      for (let y = 25; y < mapa.limites.alto; y += 25) {
        const posicion = { x, y };
        const viabilidad = store.viabilidadFundacion(posicion);
        if (!viabilidad.recomendable) continue;
        const tienePiedra = viabilidad.recursosEnRadio.some((r) => r.tipo === 'piedra' && r.nodos > 0);
        if (!tienePiedra) continue;
        if (otrosMinerales.some((tipo) => viabilidad.recursosEnRadio.some((r) => r.tipo === tipo && r.nodos > 0))) {
          candidatosConBonus++;
        }
      }
    }
    expect(candidatosConBonus).toBeGreaterThan(0);

    store.crearFaccion('Facción NPC Rica');
    const faccionId = store.getState().facciones[0]!.id;
    store.alternarFaccionNpc(faccionId, true);
    for (let i = 0; i < 5 && store.getState().asentamientos.length === 0; i++) store.avanzarTick();

    const asentamiento = store.getState().asentamientos.find((a) => a.faccionId === faccionId)!;
    const bonusEncontrado = otrosMinerales.filter((tipo) =>
      mapa.nodosEnRadio(asentamiento.posicion, asentamiento.radioPotencial).some((n) => n.tipo === tipo)
    ).length;
    expect(bonusEncontrado).toBeGreaterThan(0);
  });
});
