// Cesión de una Facción al NPC de gobernanza (`session/npcGobernanza.ts`) desde la pestaña Facción.
//
// Portado desde `app/__tests__/faccionNpc.test.ts` (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md, Fase B3 —
// migración de `main.ts`): ese archivo probaba esto contra el `GameStore` local y SÍNCRONO de antes de la
// migración. Con `GameStore` convertido en adaptador de red (sin estado propio, todo async), ya no hay forma
// de ejercitar esto sin levantar un servidor real — pero el comportamiento que blinda (el NPC juega SOLO las
// Facciones cedidas, se funda a sí mismo, prefiere sitios con mineral extra...) es de `GameSession`/
// `avanzarNpcGobernanza`, no de la capa de red. Se prueba aquí, contra `GameSession` directo, igual que el
// resto de `session/__tests__/` — mismo lugar donde ya vivía la cobertura básica de `avanzarFaccionesNpc`
// (`gameSession.test.ts`).
//
// Lo que se blinda: el NPC juega SOLO las Facciones que se le han cedido (`faccionesNpcIds`) y no toca las que
// el jugador sigue jugando a mano — ni asignándoles cargos, ni reservando su almacén, ni reclutando con sus
// residentes, ni pactando trueques que comprometan sus recursos.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, RecursoTipo } from '../../domain/types';
import { evaluarViabilidadFundacion } from '../../engine/settlement';
import { GameSession } from '../gameSession';
import { alternarFaccionNpc } from '../comandos/alternarFaccionNpc';
import { crearFaccion } from '../comandos/crearFaccion';
import { fundarAsentamiento } from '../comandos/fundarAsentamiento';

// Misma seed que usaba `new GameStore()` (constructor local de antes de la migración, `GameSession.crear
// ('local', { seed: 1 })`): varios tests de abajo dependen de que este mundo concreto tenga minerales extra
// alcanzables cerca de sitios con madera+piedra.
const SEED = 1;
const MOMENTO = '2026-01-01T00:00:00.000Z';
const ACTOR = 'jugador-test';

function partidaConDosFacciones(): { sesion: GameSession; faccionNpcId: string; faccionManualId: string } {
  const sesion = GameSession.crear('test-npc', { seed: SEED });
  const r1 = sesion.ejecutar(crearFaccion, { nombre: 'Facción NPC' }, { momento: MOMENTO, actor: ACTOR });
  const r2 = sesion.ejecutar(crearFaccion, { nombre: 'Facción Manual' }, { momento: MOMENTO, actor: ACTOR });
  if (!r1.ok || !r2.ok) throw new Error('setup del test: no se pudieron crear las Facciones');
  const faccionNpcId = r1.datos!.faccionId;
  const faccionManualId = r2.datos!.faccionId;

  // Mismo barrido de grilla que el archivo original: el test no depende de que un punto elegido a mano sea
  // viable con esta seed.
  const { ancho, alto } = sesion.getMapa().limites;
  const posiciones: { x: number; y: number }[] = [];
  for (let x = 40; x < ancho && posiciones.length < 2; x += 40) {
    for (let y = 40; y < alto && posiciones.length < 2; y += 40) {
      const candidata = { x, y };
      if (!evaluarViabilidadFundacion(sesion.getMapa(), candidata, sesion.getState().asentamientos).recomendable) continue;
      if (posiciones.some((p) => Math.hypot(p.x - x, p.y - y) < 200)) continue;
      posiciones.push(candidata);
    }
  }
  expect(posiciones).toHaveLength(2);

  sesion.ejecutar(fundarAsentamiento, { faccionId: faccionNpcId, posicion: posiciones[0]!, numJugadores: 3 }, { momento: MOMENTO, actor: ACTOR });
  sesion.ejecutar(fundarAsentamiento, { faccionId: faccionManualId, posicion: posiciones[1]!, numJugadores: 3 }, { momento: MOMENTO, actor: ACTOR });
  expect(sesion.getState().asentamientos).toHaveLength(2);

  return { sesion, faccionNpcId, faccionManualId };
}

function asentamientoDe(sesion: GameSession, faccionId: string): Asentamiento {
  const asentamiento = sesion.getState().asentamientos.find((a) => a.faccionId === faccionId);
  expect(asentamiento).toBeDefined();
  return asentamiento!;
}

/** `GameSession.avanzarTick` es el tick PURO del motor — el turno del NPC es responsabilidad de quien la
 * llama (antes `GameStore.avanzarTick`, ahora `RunnerDePartida.avanzarTick`, ver `server/runnerDePartida.ts`).
 * Este helper reproduce ese mismo bundling para el test. */
function avanzar(sesion: GameSession, n: number): void {
  for (let i = 0; i < n; i++) {
    sesion.avanzarTick(MOMENTO);
    sesion.avanzarFaccionesNpc(MOMENTO);
  }
}

describe('Facción controlada por NPC', () => {
  it('gobierna la Facción cedida y no toca la que juega el jugador', () => {
    const { sesion, faccionNpcId, faccionManualId } = partidaConDosFacciones();
    sesion.ejecutar(alternarFaccionNpc, { faccionId: faccionNpcId, activo: true }, { momento: MOMENTO, actor: ACTOR });
    avanzar(sesion, 40);

    const npc = asentamientoDe(sesion, faccionNpcId);
    expect(npc.cargos.gobernadorId).toBeTruthy();
    expect(npc.cargos.tesoreroId).toBeTruthy();
    expect(npc.reservaManual?.madera ?? 0).toBeGreaterThan(0);

    // La Facción del jugador sigue exactamente como la dejó: sin cargos que él no haya nombrado, sin reservas
    // que él no haya puesto y sin escuadrones que él no haya reclutado. Su auto-construcción sí avanza — eso
    // es el motor (`avanzarConstruccion`), no el NPC.
    const manual = asentamientoDe(sesion, faccionManualId);
    expect(manual.cargos.gobernadorId).toBeNull();
    expect(manual.cargos.tesoreroId).toBeNull();
    expect(manual.reservaManual?.madera ?? 0).toBe(0);
    expect(manual.escuadrones).toHaveLength(0);
    expect(sesion.getState().caravanas.some((c) => c.origenAsentamientoId === manual.id)).toBe(false);

    // Ningún trueque del NPC puede comprometer recursos de la Facción del jugador (a petición del usuario:
    // el NPC solo pacta con otras Facciones NPC).
    const idsManual = sesion.getState().asentamientos.filter((a) => a.faccionId === faccionManualId).map((a) => a.id);
    for (const acuerdo of sesion.getState().acuerdos) {
      expect(idsManual).not.toContain(acuerdo.asentamientoAId);
      expect(idsManual).not.toContain(acuerdo.asentamientoBId);
    }
  });

  it('se cede y se retoma en caliente a mitad de partida', () => {
    const { sesion, faccionNpcId } = partidaConDosFacciones();

    avanzar(sesion, 20);
    expect(asentamientoDe(sesion, faccionNpcId).cargos.gobernadorId).toBeNull();

    sesion.ejecutar(alternarFaccionNpc, { faccionId: faccionNpcId, activo: true }, { momento: MOMENTO, actor: ACTOR });
    expect(sesion.getState().faccionesNpcIds).toContain(faccionNpcId);
    avanzar(sesion, 5);
    expect(asentamientoDe(sesion, faccionNpcId).cargos.gobernadorId).toBeTruthy();

    // Retomar el control no deshace lo que el NPC ya hizo (es estado normal del juego): solo deja de decidir.
    sesion.ejecutar(alternarFaccionNpc, { faccionId: faccionNpcId, activo: false }, { momento: MOMENTO, actor: ACTOR });
    expect(sesion.getState().faccionesNpcIds).not.toContain(faccionNpcId);
    const antes = asentamientoDe(sesion, faccionNpcId);
    avanzar(sesion, 5);
    const despues = asentamientoDe(sesion, faccionNpcId);
    expect(despues.cargos.gobernadorId).toBe(antes.cargos.gobernadorId);
    expect(despues.escuadrones.length).toBe(antes.escuadrones.length);
  });

  it('la marca de NPC sobrevive a exportar/importar la partida', () => {
    const { sesion, faccionNpcId } = partidaConDosFacciones();
    sesion.ejecutar(alternarFaccionNpc, { faccionId: faccionNpcId, activo: true }, { momento: MOMENTO, actor: ACTOR });
    avanzar(sesion, 5);

    // `exportar()`/`GameSession.importar()`, no el formato de archivo de descarga del navegador — es la vía
    // de persistencia REAL desde Fase B3 (`server/persistenciaPartida.ts` la usa igual).
    const otra = GameSession.importar(sesion.exportar());
    expect(otra.getState().faccionesNpcIds).toEqual([faccionNpcId]);
  });

  it('se funda a sí misma si se cede sin ningún asentamiento (reportado por el usuario: quedaba inerte)', () => {
    const sesion = GameSession.crear('test-npc-inerte', { seed: SEED });
    const creada = sesion.ejecutar(crearFaccion, { nombre: 'Facción NPC' }, { momento: MOMENTO, actor: ACTOR });
    if (!creada.ok) throw new Error('setup del test: no se pudo crear la Facción');
    const faccionId = creada.datos!.faccionId;

    // A diferencia de `partidaConDosFacciones`, aquí NO se funda nada a mano: el jugador crea la Facción, la
    // marca NPC y avanza tick — el punto de partida real que reportó el fallo.
    sesion.ejecutar(alternarFaccionNpc, { faccionId, activo: true }, { momento: MOMENTO, actor: ACTOR });
    expect(sesion.getState().asentamientos).toHaveLength(0);

    avanzar(sesion, 10);

    const propios = sesion.getState().asentamientos.filter((a) => a.faccionId === faccionId);
    expect(propios).toHaveLength(1);
    expect(propios[0]!.cargos.gobernadorId).toBeTruthy();
  });

  it('funda su asentamiento inicial en un sitio con madera Y piedra alcanzables (a petición del usuario)', () => {
    const sesion = GameSession.crear('test-npc-piedra', { seed: SEED });
    const creada = sesion.ejecutar(crearFaccion, { nombre: 'Facción NPC Piedra' }, { momento: MOMENTO, actor: ACTOR });
    if (!creada.ok) throw new Error('setup del test: no se pudo crear la Facción');
    const faccionId = creada.datos!.faccionId;

    sesion.ejecutar(alternarFaccionNpc, { faccionId, activo: true }, { momento: MOMENTO, actor: ACTOR });
    for (let i = 0; i < 5 && sesion.getState().asentamientos.length === 0; i++) avanzar(sesion, 1);

    const asentamiento = sesion.getState().asentamientos.find((a) => a.faccionId === faccionId);
    expect(asentamiento).toBeDefined();

    // Mismo radio que usa `evaluarViabilidadFundacion` para decidir viabilidad — se comprueba con la API
    // pública del mapa, no con el resultado de esa función, para no repetir la lógica que se está probando.
    const mapa = sesion.getMapa();
    const radio = asentamiento!.radioPotencial;
    expect(mapa.hayBosqueEnRadio(asentamiento!.posicion, radio)).toBe(true);
    const piedraEnRadio = mapa.nodosEnRadio(asentamiento!.posicion, radio).filter((n) => n.tipo === 'piedra');
    expect(piedraEnRadio.length).toBeGreaterThan(0);
  });

  it('prefiere un sitio con minerales extra a uno con solo madera y piedra, cuando el mapa ofrece ambos', () => {
    // Regresión directa del reporte del usuario: la primera versión se conformaba con el primer candidato
    // viable de un barrido grueso y podía devolver un punto con solo madera+piedra aunque el mapa tuviera de
    // sobra sitios con algún mineral extra cerca. Se prueba contra el mundo real por defecto (misma seed que
    // usaba `new GameStore()`): si el barrido denso encuentra AL MENOS un candidato con mineral extra en todo
    // el mapa (lo cual, para esta seed, es así — hay muchos), el elegido tiene que ser uno de ellos.
    const sesion = GameSession.crear('test-npc-rica', { seed: SEED });
    const mapa = sesion.getMapa();

    // Referencia independiente: mismo criterio que la heurística bajo prueba, pero recorrido aparte (no se
    // llama a la función que se está verificando) — barre el mapa entero al paso más fino y cuenta cuántos
    // candidatos válidos (madera+piedra) tienen 1+ mineral extra.
    let candidatosConBonus = 0;
    const otrosMinerales: RecursoTipo[] = ['cobre', 'estano', 'oro', 'livestock'];
    for (let x = 25; x < mapa.limites.ancho; x += 25) {
      for (let y = 25; y < mapa.limites.alto; y += 25) {
        const posicion = { x, y };
        const viabilidad = evaluarViabilidadFundacion(mapa, posicion, sesion.getState().asentamientos);
        if (!viabilidad.recomendable) continue;
        const tienePiedra = viabilidad.recursosEnRadio.some((r) => r.tipo === 'piedra' && r.nodos > 0);
        if (!tienePiedra) continue;
        if (otrosMinerales.some((tipo) => viabilidad.recursosEnRadio.some((r) => r.tipo === tipo && r.nodos > 0))) {
          candidatosConBonus++;
        }
      }
    }
    expect(candidatosConBonus).toBeGreaterThan(0);

    const creada = sesion.ejecutar(crearFaccion, { nombre: 'Facción NPC Rica' }, { momento: MOMENTO, actor: ACTOR });
    if (!creada.ok) throw new Error('setup del test: no se pudo crear la Facción');
    const faccionId = creada.datos!.faccionId;
    sesion.ejecutar(alternarFaccionNpc, { faccionId, activo: true }, { momento: MOMENTO, actor: ACTOR });
    for (let i = 0; i < 5 && sesion.getState().asentamientos.length === 0; i++) avanzar(sesion, 1);

    const asentamiento = sesion.getState().asentamientos.find((a) => a.faccionId === faccionId)!;
    const bonusEncontrado = otrosMinerales.filter((tipo) =>
      mapa.nodosEnRadio(asentamiento.posicion, asentamiento.radioPotencial).some((n) => n.tipo === tipo)
    ).length;
    expect(bonusEncontrado).toBeGreaterThan(0);
  });
});
