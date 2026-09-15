// Facciones NPC (`session/npcGobernanza.ts`): las crea el admin ya asentadas (`crearFaccionNpc`).
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
// Lo que se blinda: el NPC juega SOLO las Facciones NPC (`faccionesNpcIds`) y no toca las que juega un
// jugador — ni asignándoles cargos, ni reservando su almacén, ni reclutando con sus residentes, ni pactando
// trueques que comprometan sus recursos.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, Ejercito, RecursoTipo } from '../../domain/types';
import { createRng } from '../../worldgen';
import { avanzarSimulacion } from '../../engine/simulation';
import { comerciarEnPlaza } from '../../engine/market';
import { avanzarNpcGobernanza } from '../npcGobernanza';
import {
  contextoDeTest,
  crearEstadoDeTest,
  crearFacciones,
  crearMapaDeterminista,
  escuadronDePrueba,
  fundarAsentamientoDeTest,
  heroesCon,
  instanteDeTest,
} from '../../engine/__tests__/fixtures';
import { campamentoDe } from '../../engine/tropa';

const mapaDeterminista = crearMapaDeterminista(42);
import { evaluarViabilidadFundacion } from '../../engine/settlement';
import { GameSession } from '../gameSession';
import { crearFaccion } from '../comandos/crearFaccion';
import { crearFaccionNpc } from '../comandos/crearFaccionNpc';
import { fundarAsentamiento } from '../comandos/fundarAsentamiento';
import { conHeroe } from './fixtures';

// Misma seed que usaba `new GameStore()` (constructor local de antes de la migración, `GameSession.crear
// ('local', { seed: 1 })`): varios tests de abajo dependen de que este mundo concreto tenga minerales extra
// alcanzables cerca de sitios con madera+piedra.
const SEED = 1;

function partidaConDosFacciones(): { sesion: GameSession; faccionNpcId: string; faccionManualId: string } {
  // La manual la juega una persona: crea su Facción y funda donde apareció (Doc 1.3). La NPC la crea después el
  // admin, ya asentada, así que su búsqueda de sitio ve la plaza manual y no se le echa encima.
  const sesion = conHeroe(GameSession.crear('test-npc', { seed: SEED }), 'jugador-manual');
  const manual = sesion.ejecutar(crearFaccion, { nombre: 'Facción Manual' }, { actor: 'jugador-manual' });
  if (!manual.ok) throw new Error('setup del test: no se pudo crear la Facción manual');
  const faccionManualId = manual.datos!.faccionId;
  sesion.ejecutar(fundarAsentamiento, { faccionId: faccionManualId }, { actor: 'jugador-manual' });

  const npc = sesion.ejecutar(crearFaccionNpc, { nombre: 'Facción NPC' });
  if (!npc.ok) throw new Error('setup del test: no se pudo crear la Facción NPC');
  expect(sesion.getState().asentamientos).toHaveLength(2);

  return { sesion, faccionNpcId: npc.datos!.faccionId, faccionManualId };
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
    sesion.avanzarTick();
    sesion.avanzarFaccionesNpc();
  }
}

describe('Facción controlada por NPC', () => {
  it('gobierna la Facción NPC y no toca la que juega el jugador', () => {
    const { sesion, faccionNpcId, faccionManualId } = partidaConDosFacciones();
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
    expect(campamentoDe(manual, sesion.getState().heroes)).toHaveLength(0);
    expect(sesion.getState().caravanas.some((c) => c.origenAsentamientoId === manual.id)).toBe(false);

    // Ningún trueque del NPC puede comprometer recursos de la Facción del jugador (a petición del usuario:
    // el NPC solo pacta con otras Facciones NPC).
    const idsManual = sesion.getState().asentamientos.filter((a) => a.faccionId === faccionManualId).map((a) => a.id);
    for (const acuerdo of sesion.getState().acuerdos) {
      expect(idsManual).not.toContain(acuerdo.asentamientoAId);
      expect(idsManual).not.toContain(acuerdo.asentamientoBId);
    }
  });

  it('la marca de NPC sobrevive a exportar/importar la partida', () => {
    const { sesion, faccionNpcId } = partidaConDosFacciones();
    avanzar(sesion, 5);

    // `exportar()`/`GameSession.importar()`, no el formato de archivo de descarga del navegador — es la vía
    // de persistencia REAL desde Fase B3 (`server/persistenciaPartida.ts` la usa igual).
    const otra = GameSession.importar(sesion.exportar());
    expect(otra.getState().faccionesNpcIds).toEqual([faccionNpcId]);
  });

  it('si se queda sin asentamientos, vuelve a fundar con sus propios héroes bot, sin crear otros', () => {
    const creada = GameSession.crear('test-npc-refunda', { seed: SEED });
    const r = creada.ejecutar(crearFaccionNpc, { nombre: 'Facción NPC' });
    if (!r.ok) throw new Error('setup del test: no se pudo crear la Facción NPC');
    const faccionId = r.datos!.faccionId;
    const bots = creada.getState().asentamientos[0]!.heroesFundadoresIds;

    const payload = creada.exportar();
    const sesion = GameSession.importar({ ...payload, state: { ...payload.state, asentamientos: [] } });
    avanzar(sesion, 1);

    const propios = sesion.getState().asentamientos.filter((a) => a.faccionId === faccionId);
    expect(propios).toHaveLength(1);
    expect(propios[0]!.heroesFundadoresIds).toEqual(bots);
    expect(sesion.getState().heroes).toHaveLength(bots.length);
  });

  it('funda su asentamiento inicial en un sitio con madera Y piedra alcanzables (a petición del usuario)', () => {
    const sesion = GameSession.crear('test-npc-piedra', { seed: SEED });
    const creada = sesion.ejecutar(crearFaccionNpc, { nombre: 'Facción NPC Piedra' });
    if (!creada.ok) throw new Error('setup del test: no se pudo crear la Facción');
    const faccionId = creada.datos!.faccionId;

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

    const creada = sesion.ejecutar(crearFaccionNpc, { nombre: 'Facción NPC Rica' });
    if (!creada.ok) throw new Error('setup del test: no se pudo crear la Facción');
    const faccionId = creada.datos!.faccionId;

    const asentamiento = sesion.getState().asentamientos.find((a) => a.faccionId === faccionId)!;
    const bonusEncontrado = otrosMinerales.filter((tipo) =>
      mapa.nodosEnRadio(asentamiento.posicion, asentamiento.radioPotencial).some((n) => n.tipo === tipo)
    ).length;
    expect(bonusEncontrado).toBeGreaterThan(0);
  });
});

// La politica de persecucion del NPC (paso 8e, Doc 5.12.3).
//
// Existe para tapar el riesgo mas silencioso de toda la mecanica: desde que los encuentros dejaron de salir
// de la geometria, un combate solo ocurre si alguien lo pide — y en el laboratorio no hay nadie pidiendo. Sin
// esto, las constantes militares ya calibradas se seguirian midiendo sobre un mundo en paz SIN QUE NINGUNA
// PRUEBA FALLARA. Estos tests son esa prueba.
/** Dos columnas NPC enemigas a la vista una de otra, en campo abierto. */
function dosColumnasNpc() {
  const facciones = crearFacciones();
  const uno = fundarAsentamientoDeTest(mapaDeterminista, facciones, 'faccion-1', []);
  const dos = fundarAsentamientoDeTest(mapaDeterminista, uno.facciones, 'faccion-2', [uno.asentamiento]);
  const punto = { x: 1000, y: 1000 };
  const tropa = (id: string) =>
    escuadronDePrueba(`esc-${id}`, `j-${id}`, 'milicia_lanceros', 30, { contenedor: { tipo: 'ejercito', ejercitoId: id } });
  const columna = (id: string, faccionId: string, origenId: string, x: number): Ejercito => ({
    id,
    faccionId,
    origenAsentamientoId: origenId,
    participantes: [{ heroeId: `j-${id}`, unidoEn: instanteDeTest(0) }],
    tipo: 'ejercito',
    liderId: `j-${id}`,
    politicaDeUnion: 'rechazar',
    escuadronIds: [`esc-${id}`],
    suministro: { trigo: 500 },
    caravanasAdjuntasIds: [],
    objetivo: { tipo: 'punto', punto },
    ruta: [punto, { x: punto.x + 10, y: punto.y }],
    progreso: 0,
    posicionActual: { x, y: punto.y },
    estado: 'estacionado',
  });
  const estado = crearEstadoDeTest([uno.asentamiento, dos.asentamiento], dos.facciones, {
    // A 10 una de otra: dentro del radio de encuentro, asi que la persecucion se cierra en el mismo tick.
    ejercitos: [columna('col-a', 'faccion-1', uno.asentamiento.id, punto.x), columna('col-b', 'faccion-2', dos.asentamiento.id, punto.x + 10)],
    // Héroes bot: para el NPC perseguir ya es su orden de ataque (Doc 5.12.3).
    heroes: heroesCon([tropa('col-a'), tropa('col-b')]).map((h) => ({ ...h, controlador: 'bot' as const })),
  });
  return { estado, mapa: mapaDeterminista };
}

describe('el NPC persigue: sin esto el batch se queda sin combates y nadie se entera', () => {
  it('fija persecucion contra una columna enemiga que tiene a la vista', () => {
    const { estado, mapa } = dosColumnasNpc();

    const r = avanzarNpcGobernanza(estado, mapa, contextoDeTest(1, createRng(5)), {});

    const cazador = r.estado.ejercitos.find((e) => e.id === 'col-a')!;
    expect(cazador.persiguiendo, 'sin presa fijada no habria combate nunca').toEqual({ tipo: 'ejercito', id: 'col-b' });
  });

  it('y en el tick siguiente eso PRODUCE combate: el laboratorio no se queda en paz', () => {
    const { estado, mapa } = dosColumnasNpc();
    const conPresas = avanzarNpcGobernanza(estado, mapa, contextoDeTest(1, createRng(5)), {}).estado;

    const sim = avanzarSimulacion(conPresas, mapa, contextoDeTest(2, createRng(5)));

    expect(sim.eventosDominio.some((e) => e.codigo === 'combate.encuentro'), 'hubo combate').toBe(true);
  });

  it('no persigue a los suyos: solo a enemigos', () => {
    const { estado, mapa } = dosColumnasNpc();
    const mismaFaccion = {
      ...estado,
      ejercitos: estado.ejercitos.map((e) => ({ ...e, faccionId: 'faccion-1' })),
    };

    const r = avanzarNpcGobernanza(mismaFaccion, mapa, contextoDeTest(1, createRng(5)), {});

    expect(r.estado.ejercitos.every((e) => e.persiguiendo === undefined)).toBe(true);
  });

  it('ni persigue con todos sus héroes HERIDOS: la regla vale igual para el NPC (Doc 5.16.4)', () => {
    const { estado, mapa } = dosColumnasNpc();
    const heridos = {
      ...estado,
      heroes: estado.heroes.map((h) => ({ ...h, heridoHasta: instanteDeTest(9999) })),
    };

    const r = avanzarNpcGobernanza(heridos, mapa, contextoDeTest(1, createRng(5)), {});

    expect(r.estado.ejercitos.every((e) => e.persiguiendo === undefined)).toBe(true);
  });
});

// La POSTURA de una Faccion NPC (Consideraciones/Entrada_Al_Mundo_Definicion.md, decision 5).
//
// Las Facciones sembradas al arrancar el servidor tienen que ser vecinos, no depredadores: un recien llegado
// no es aliado de nadie, asi que con la postura agresiva seria presa a la vista de cualquier columna NPC
// antes de tener con que defenderse. Y el laboratorio necesita justo lo contrario, asi que es un ajuste y no
// un borrado.
describe('postura defensiva: un vecino, no un depredador', () => {
  it('una Faccion DEFENSIVA no da caza a nadie, aunque lo tenga a tiro', () => {
    const { estado, mapa } = dosColumnasNpc();

    const r = avanzarNpcGobernanza(estado, mapa, contextoDeTest(1, createRng(5)), { postura: 'defensiva' });

    expect(r.estado.ejercitos.every((e) => e.persiguiendo === undefined)).toBe(true);
  });

  it('y por defecto sigue siendo AGRESIVA: el batch no puede quedarse sin combates en silencio', () => {
    const { estado, mapa } = dosColumnasNpc();

    const r = avanzarNpcGobernanza(estado, mapa, contextoDeTest(1, createRng(5)), {});

    expect(r.estado.ejercitos.some((e) => e.persiguiendo !== undefined), 'el defecto no cambia').toBe(true);
  });
});

// El NPC como SOCIO DE COMERCIO (Consideraciones/Entrada_Al_Mundo_Definicion.md §3).
//
// La via son ordenes de mercado y no trueques por una razon concreta: `proponerTrueque` pacta sin pedir
// consentimiento al otro lado, asi que un NPC proponiendoselo a un jugador le comprometeria recursos sin
// preguntarle. Una orden publicada no compromete a nadie — se toma o no se toma.
describe('el NPC publica en el mercado: un vecino con quien comerciar', () => {
  /** Una plaza NPC con Mercado activo, el silo de piedra a rebosar y el de madera casi vacio. */
  function plazaConMercado() {
    const facciones = crearFacciones();
    const uno = fundarAsentamientoDeTest(mapaDeterminista, facciones, 'faccion-1', []);
    const conMercado: Asentamiento = {
      ...uno.asentamiento,
      edificios: [...uno.asentamiento.edificios, { id: 'mercado-1', tipo: 'mercado', posicion: { x: 0, y: 0 }, estado: 'activo' }],
      almacen: {
        ...uno.asentamiento.almacen,
        piedra: { capacidad: 1000, cantidad: 950 },
        madera: { capacidad: 1000, cantidad: 50 },
      },
    };
    return crearEstadoDeTest([conMercado], uno.facciones);
  }

  it('pone a la venta lo que le SOBRA', () => {
    const estado = plazaConMercado();

    const r = avanzarNpcGobernanza(estado, mapaDeterminista, contextoDeTest(1, createRng(5)), {});

    const venta = r.estado.ordenes.find((o) => o.tipo === 'venta' && o.recurso === 'piedra');
    expect(venta, 'el silo lleno se vende, que guardar de mas no sirve').toBeDefined();
    expect(venta!.cantidad).toBeGreaterThan(0);
  });

  it('y publica compra de lo que le FALTA', () => {
    const estado = plazaConMercado();

    const r = avanzarNpcGobernanza(estado, mapaDeterminista, contextoDeTest(1, createRng(5)), {});

    expect(r.estado.ordenes.find((o) => o.tipo === 'compra' && o.recurso === 'madera')).toBeDefined();
  });

  it('no duplica: si ya tiene una orden viva de ese recurso, no publica otra', () => {
    const estado = plazaConMercado();
    const uno = avanzarNpcGobernanza(estado, mapaDeterminista, contextoDeTest(1, createRng(5)), {});

    const dos = avanzarNpcGobernanza(uno.estado, mapaDeterminista, contextoDeTest(2, createRng(5)), {});

    const dePiedra = dos.estado.ordenes.filter((o) => o.recurso === 'piedra' && o.estado === 'activa');
    expect(dePiedra).toHaveLength(1);
  });

  it('sin Mercado activo no publica nada: la regla del motor vale igual para el NPC', () => {
    const facciones = crearFacciones();
    const uno = fundarAsentamientoDeTest(mapaDeterminista, facciones, 'faccion-1', []);
    const sinMercado = crearEstadoDeTest(
      [{ ...uno.asentamiento, almacen: { ...uno.asentamiento.almacen, piedra: { capacidad: 1000, cantidad: 950 } } }],
      uno.facciones
    );

    const r = avanzarNpcGobernanza(sinMercado, mapaDeterminista, contextoDeTest(1, createRng(5)), {});

    expect(r.estado.ordenes).toEqual([]);
  });

  it('y con la palanca cerrada se comporta como antes de existir esto', () => {
    const estado = plazaConMercado();

    const r = avanzarNpcGobernanza(estado, mapaDeterminista, contextoDeTest(1, createRng(5)), { colocarOrdenes: false });

    expect(r.estado.ordenes).toEqual([]);
  });
});

// Y lo que de verdad prueba el objetivo: que un JUGADOR pueda comprarle. Publicar ordenes no sirve de nada si
// nadie puede tomarlas — y desde 2026-09-07 tomarlas significa IR HASTA ALLI con el oro encima
// (`Consideraciones/Comercio_Fisico_Definicion.md`), no un emparejamiento automatico entre almacenes.
describe('un jugador puede comerciar con una plaza NPC', () => {
  it('el NPC pone piedra a la venta, el jugador se planta en su puerta con oro, y se la lleva EN EL CARRO', () => {
    const facciones = crearFacciones();
    const npc = fundarAsentamientoDeTest(mapaDeterminista, facciones, 'faccion-1', []);
    const humano = fundarAsentamientoDeTest(mapaDeterminista, npc.facciones, 'faccion-2', [npc.asentamiento]);
    const conMercado = (a: Asentamiento, piedra: number): Asentamiento => ({
      ...a,
      edificios: [...a.edificios, { id: `mercado-${a.id}`, tipo: 'mercado', posicion: { x: 0, y: 0 }, estado: 'activo' }],
      almacen: { ...a.almacen, piedra: { capacidad: 1000, cantidad: piedra }, oro: { capacidad: 1000, cantidad: 500 } },
    });
    const plazaNpc = conMercado(npc.asentamiento, 950);
    const plazaHumano = conMercado(humano.asentamiento, 0);
    const estado = crearEstadoDeTest([plazaNpc, plazaHumano], humano.facciones);

    // El NPC gobierna SOLO su Faccion: la del humano no la toca, como en una partida real.
    const conOrdenes = avanzarNpcGobernanza(estado, mapaDeterminista, contextoDeTest(1, createRng(5)), {
      faccionesIds: ['faccion-1'],
    }).estado;
    const venta = conOrdenes.ordenes.find((o) => o.tipo === 'venta' && o.recurso === 'piedra' && o.asentamientoId === plazaNpc.id);
    expect(venta, 'el NPC ha puesto piedra a la venta').toBeDefined();

    // El jugador ha viajado hasta la plaza del NPC con el carro cargado de oro. Esa caminata es la mecanica
    // entera: sin ella no hay trato.
    const plazaNpcAhora = conOrdenes.asentamientos.find((a) => a.id === plazaNpc.id)!;
    const columna = {
      id: 'columna-humano',
      faccionId: 'faccion-2',
      liderId: 'jugador-humano',
      tipo: 'personal',
      participantes: [{ heroeId: 'jugador-humano', unidoEn: instanteDeTest(0) }],
      escuadrones: [],
      suministro: { oro: 400 },
      caravanasAdjuntasIds: [],
      posicionActual: plazaNpcAhora.posicion,
      estado: 'estacionado',
    } as unknown as Ejercito;

    const trato = comerciarEnPlaza(columna, 'jugador-humano', plazaNpcAhora, venta!, venta!.cantidad, 5000, instanteDeTest(1));

    expect(trato.cantidad, 'algo cambia de manos').toBeGreaterThan(0);
    expect(trato.ejercito.suministro['piedra'], 'la piedra va en el carro, no en su ciudad').toBeGreaterThan(0);
    expect(trato.plaza.almacen['piedra']!.cantidad, 'y al NPC le queda menos').toBeLessThan(950);

    // Lo que este test existe para fijar: la ciudad del jugador NO ha recibido nada todavia. Falta el viaje de
    // vuelta y depositar (`absorberColumna`), que es justo lo que el emparejamiento automatico se saltaba.
    const suPlaza = conOrdenes.asentamientos.find((a) => a.id === plazaHumano.id)!;
    expect(suPlaza.almacen['piedra']!.cantidad).toBe(0);
  });
});
