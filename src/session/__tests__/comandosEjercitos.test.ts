// Comandos de ejércitos (`session/comandos/ejercitos.ts`, Paso 3 del movimiento de ejércitos).
//
// Este es el primer punto de la mecánica que se puede VER funcionar: hasta aquí solo había tipos, constantes
// y un módulo puro. Lo que congela este archivo es lo que el consejo pidió comprobar en vivo antes de dar los
// pasos anteriores por cerrados — que el tope de Liderazgo rechaza una movilización de verdad — más los tres
// invariantes de composición: los escuadrones se van DE VERDAD del asentamiento, replegar no teletransporta,
// y unirse exige proximidad.
import { describe, expect, it } from 'vitest';
import { GameSession } from '../gameSession';
import { alternarReabastecerAliados, estacionarEjercito, movilizarEjercito, replegarEjercito, unirseAEjercito } from '../comandos/ejercitos';
import { OPC, partidaConAsentamiento } from './fixtures';
import { CODIGOS_ERROR } from '../comandos/codigosDeError';
import { LOGISTICA } from '../../constants';
import { reservaDeTrigo } from '../../engine/tropas';

/**
 * Partida con escuadrones YA puestos en el asentamiento, inyectados vía `importar` en vez de reclutados.
 *
 * Reclutar de verdad exigiría Barracón y Galería de tiro construidos y con nivel interno, más el equipo en
 * almacén — obstáculos reales del motor (`reclutarTropa`) pero ajenos a lo que se prueba aquí, que es la
 * COMPOSICIÓN de un ejército. Mismo atajo que usa `autorizacionComandos.test.ts` por la misma razón.
 *
 * Las tropas elegidas cubren dos escalones de coste de Liderazgo: milicia y lanceros de mimbre son leva
 * (escalón 1), honderos es tropa de línea (escalón 2).
 */
function partidaConTropas(liderazgoBase?: number) {
  const base = partidaConAsentamiento();
  const payload = base.sesion.exportar();
  const asentamiento = payload.state.asentamientos[0]!;
  const escuadron = (id: string, tropaId: string, jugadorId: string) => ({
    id,
    nombre: tropaId,
    jugadorId,
    origen: 'pesants' as const,
    cantidad: 10,
    veterania: 0,
    moral: 100,
    tropaId,
  });
  const sesion = GameSession.importar({
    ...payload,
    state: {
      ...payload.state,
      asentamientos: [
        {
          ...asentamiento,
          escuadrones: [
            escuadron('esc-milicia', 'milicia_lanceros', base.fundador),
            escuadron('esc-mimbre', 'lanceros_mimbre', base.fundador),
            escuadron('esc-honderos', 'honderos', base.fundador),
            escuadron('esc-vecino', 'milicia_lanceros', base.vecino),
          ],
        },
        ...payload.state.asentamientos.slice(1),
      ],
      jugadores:
        liderazgoBase === undefined
          ? []
          : [{ id: base.fundador, liderazgoBase, ubicacion: { tipo: 'asentamiento' as const, asentamientoId: payload.state.asentamientos[0]!.id } }],
    },
  });
  return { ...base, sesion };
}

const PUNTO_LEJOS = { tipo: 'punto', punto: { x: 900, y: 900 } } as const;

const trigoDe = (a: { almacen: Record<string, { cantidad: number }> }) => a.almacen['trigo']?.cantidad ?? 0;

/** Deja el almacén de trigo en `cantidad`, para colocar al asentamiento a un lado u otro de su reserva. */
function conTrigo(base: ReturnType<typeof partidaConTropas>, cantidad: number) {
  const payload = base.sesion.exportar();
  const a = payload.state.asentamientos[0]!;
  const sesion = GameSession.importar({
    ...payload,
    state: {
      ...payload.state,
      asentamientos: [
        { ...a, almacen: { ...a.almacen, trigo: { ...(a.almacen['trigo'] ?? { capacidad: 100000 }), cantidad } } },
        ...payload.state.asentamientos.slice(1),
      ],
    },
  });
  return { ...base, sesion };
}

describe('movilizarEjercito', () => {
  it('saca los escuadrones DE VERDAD del asentamiento y crea el ejército', () => {
    const { sesion, asentamientoId, fundador } = partidaConTropas();
    const escuadronId = 'esc-milicia';

    const r = sesion.ejecutar(
      movilizarEjercito,
      { asentamientoId, jugadorId: fundador, escuadronIds: [escuadronId], objetivo: PUNTO_LEJOS },
      OPC
    );

    expect(r.ok).toBe(true);
    const estado = sesion.getState();
    // La guarnición se quedó sin ese escuadrón: es lo que hace que "tu ciudad queda desnuda" se cumpla por
    // construcción, sin ningún predicado extra (Doc 5.12.4).
    expect(estado.asentamientos[0]!.escuadrones.map((e) => e.id)).not.toContain(escuadronId);
    expect(estado.ejercitos).toHaveLength(1);
    const ejercito = estado.ejercitos[0]!;
    expect(ejercito.escuadrones.map((e) => e.id)).toEqual([escuadronId]);
    expect(ejercito.estado).toBe('marchando');
    expect(ejercito.origenAsentamientoId).toBe(asentamientoId);
    // La ruta se calcula al salir, como una caravana al despacharse.
    expect(ejercito.ruta.length).toBeGreaterThanOrEqual(2);
    expect(ejercito.progreso).toBe(0);
    // Y sale con el carro cargado del almacén (Paso 6): algo lleva, y salió de la despensa de la ciudad.
    expect(ejercito.suministro['trigo']).toBeGreaterThan(0);
    const trigoAntes = trigoDe(partidaConTropas().sesion.getState().asentamientos[0]!);
    expect(trigoDe(estado.asentamientos[0]!)).toBeCloseTo(trigoAntes - ejercito.suministro['trigo']!);
  });

  it('RECHAZA si los escuadrones exceden el Liderazgo del jugador', () => {
    // Liderazgo 7: justo una tropa de leva (escalón 1) y nada más — los honderos son de línea (14) y se pasan.
    const { sesion, asentamientoId, fundador } = partidaConTropas(7);
    const milicia = 'esc-milicia';
    const honderos = 'esc-honderos';

    const cabe = sesion.ejecutar(
      movilizarEjercito,
      { asentamientoId, jugadorId: fundador, escuadronIds: [milicia], objetivo: PUNTO_LEJOS },
      OPC
    );
    expect(cabe.ok).toBe(true);

    const noCabe = sesion.ejecutar(
      movilizarEjercito,
      { asentamientoId, jugadorId: fundador, escuadronIds: [honderos], objetivo: PUNTO_LEJOS },
      OPC
    );
    expect(noCabe.ok).toBe(false);
    expect(noCabe.ok === false && noCabe.codigoError).toBe(CODIGOS_ERROR.movilizacionInvalida);
    // Y el rechazo no deja rastro: el escuadrón sigue en casa y no hay un segundo ejército.
    expect(sesion.getState().asentamientos[0]!.escuadrones.map((e) => e.id)).toContain(honderos);
    expect(sesion.getState().ejercitos).toHaveLength(1);
  });

  it('rechaza llevarse el escuadrón de otro jugador', () => {
    const { sesion, asentamientoId } = partidaConTropas();
    const escuadronId = 'esc-milicia';

    const r = sesion.ejecutar(
      movilizarEjercito,
      { asentamientoId, jugadorId: 'jugador-intruso', escuadronIds: [escuadronId], objetivo: PUNTO_LEJOS },
      OPC
    );
    expect(r.ok).toBe(false);
  });

  it('rechaza salir hacia el propio asentamiento de origen', () => {
    const { sesion, asentamientoId, fundador } = partidaConTropas();
    const escuadronId = 'esc-milicia';

    const r = sesion.ejecutar(
      movilizarEjercito,
      { asentamientoId, jugadorId: fundador, escuadronIds: [escuadronId], objetivo: { tipo: 'asentamiento', id: asentamientoId } },
      OPC
    );
    expect(r.ok).toBe(false);
  });

  it('rechaza salir sin escuadrones', () => {
    const { sesion, asentamientoId, fundador } = partidaConTropas();
    const r = sesion.ejecutar(
      movilizarEjercito,
      { asentamientoId, jugadorId: fundador, escuadronIds: [], objetivo: PUNTO_LEJOS },
      OPC
    );
    expect(r.ok).toBe(false);
  });
});

describe('replegarEjercito — cancelar la marcha (Doc 5.12.6)', () => {
  it('marchando: da media vuelta SIN moverse del sitio', () => {
    const { sesion, asentamientoId, fundador } = partidaConTropas();
    const escuadronId = 'esc-milicia';
    sesion.ejecutar(
      movilizarEjercito,
      { asentamientoId, jugadorId: fundador, escuadronIds: [escuadronId], objetivo: PUNTO_LEJOS },
      OPC
    );

    const antes = sesion.getState().ejercitos[0]!;
    const r = sesion.ejecutar(replegarEjercito, { ejercitoId: antes.id }, OPC);
    expect(r.ok).toBe(true);

    const despues = sesion.getState().ejercitos[0]!;
    expect(despues.estado).toBe('regresando');
    // La posición no se mueve ni un punto: volver cuesta el mismo camino que costó ir.
    expect(despues.posicionActual).toEqual(antes.posicionActual);
    // Ruta invertida y progreso espejado, que es lo que preserva la posición sobre la polilínea.
    expect(despues.ruta[0]).toEqual(antes.ruta[antes.ruta.length - 1]);
    expect(despues.progreso).toBeCloseTo(1 - antes.progreso);
  });

  it('estacionado: calcula una ruta nueva a casa en vez de desandar', () => {
    const { sesion, asentamientoId, fundador } = partidaConTropas();
    const escuadronId = 'esc-milicia';
    sesion.ejecutar(
      movilizarEjercito,
      { asentamientoId, jugadorId: fundador, escuadronIds: [escuadronId], objetivo: PUNTO_LEJOS },
      OPC
    );
    const ejercitoId = sesion.getState().ejercitos[0]!.id;
    expect(sesion.ejecutar(estacionarEjercito, { ejercitoId }, OPC).ok).toBe(true);
    expect(sesion.getState().ejercitos[0]!.estado).toBe('estacionado');

    const r = sesion.ejecutar(replegarEjercito, { ejercitoId }, OPC);
    expect(r.ok).toBe(true);
    const despues = sesion.getState().ejercitos[0]!;
    expect(despues.estado).toBe('regresando');
    expect(despues.progreso).toBe(0);
  });

  it('no se puede replegar dos veces', () => {
    const { sesion, asentamientoId, fundador } = partidaConTropas();
    const escuadronId = 'esc-milicia';
    sesion.ejecutar(
      movilizarEjercito,
      { asentamientoId, jugadorId: fundador, escuadronIds: [escuadronId], objetivo: PUNTO_LEJOS },
      OPC
    );
    const ejercitoId = sesion.getState().ejercitos[0]!.id;

    expect(sesion.ejecutar(replegarEjercito, { ejercitoId }, OPC).ok).toBe(true);
    expect(sesion.ejecutar(replegarEjercito, { ejercitoId }, OPC).ok).toBe(false);
  });
});

describe('unirseAEjercito', () => {
  it('recoge tropa del asentamiento por el que pasa', () => {
    const { sesion, asentamientoId, fundador } = partidaConTropas();
    const primero = 'esc-milicia';
    const segundo = 'esc-mimbre';

    sesion.ejecutar(
      movilizarEjercito,
      { asentamientoId, jugadorId: fundador, escuadronIds: [primero], objetivo: PUNTO_LEJOS },
      OPC
    );
    const ejercitoId = sesion.getState().ejercitos[0]!.id;

    // Todavía no se ha movido: sigue en la posición del asentamiento, así que está dentro del radio.
    const r = sesion.ejecutar(unirseAEjercito, { ejercitoId, asentamientoId, jugadorId: fundador, escuadronIds: [segundo] }, OPC);
    expect(r.ok).toBe(true);

    const estado = sesion.getState();
    expect(estado.ejercitos[0]!.escuadrones.map((e) => e.id).sort()).toEqual([primero, segundo].sort());
    expect(estado.asentamientos[0]!.escuadrones.map((e) => e.id)).not.toContain(segundo);
  });

  it('el refuerzo también cuenta contra el Liderazgo, sumando lo que ese jugador YA lleva dentro', () => {
    // Liderazgo 10: cabe una tropa de leva sola (7), pero no dos (14) — que es lo que este test comprueba:
    // el refuerzo suma contra lo que ese jugador YA lleva dentro, no se valida en el vacío.
    const { sesion, asentamientoId, fundador } = partidaConTropas(10);
    const primero = 'esc-milicia';
    const segundo = 'esc-mimbre';

    sesion.ejecutar(
      movilizarEjercito,
      { asentamientoId, jugadorId: fundador, escuadronIds: [primero], objetivo: PUNTO_LEJOS },
      OPC
    );
    const ejercitoId = sesion.getState().ejercitos[0]!.id;

    const r = sesion.ejecutar(unirseAEjercito, { ejercitoId, asentamientoId, jugadorId: fundador, escuadronIds: [segundo] }, OPC);
    expect(r.ok).toBe(false);
    expect(r.ok === false && r.codigoError).toBe(CODIGOS_ERROR.movilizacionInvalida);
  });

  it('rechaza unirse a un ejército que no existe', () => {
    const { sesion, asentamientoId, fundador } = partidaConTropas();
    const escuadronId = 'esc-milicia';
    const r = sesion.ejecutar(
      unirseAEjercito,
      { ejercitoId: 'ejercito-fantasma', asentamientoId, jugadorId: fundador, escuadronIds: [escuadronId] },
      OPC
    );
    expect(r.ok).toBe(false);
    expect(r.ok === false && r.codigoError).toBe(CODIGOS_ERROR.ejercitoNoExiste);
  });
});

// ---------------------------------------------------------------------------------------------------------
// Paso 6 — la carga del carro (Doc 5.13). Lo que se congela aquí: que sacar un ejército CUESTA stock real,
// que ese coste tiene un suelo por debajo del cual no baja, y que ese suelo no bloquea la salida.
// ---------------------------------------------------------------------------------------------------------

describe('carga del carro desde el almacén', () => {
  it('se lleva un carro entero si el almacén va sobrado', () => {
    const { sesion, asentamientoId, fundador } = conTrigo(partidaConTropas(), 100000);

    sesion.ejecutar(movilizarEjercito, { asentamientoId, jugadorId: fundador, escuadronIds: ['esc-milicia'], objetivo: PUNTO_LEJOS }, OPC);

    const estado = sesion.getState();
    expect(estado.ejercitos[0]!.suministro['trigo']).toBe(LOGISTICA.capacidadCarroPorJugador);
    expect(trigoDe(estado.asentamientos[0]!)).toBeCloseTo(100000 - LOGISTICA.capacidadCarroPorJugador);
  });

  it('NUNCA baja de la reserva: con la despensa justa, se lleva solo el sobrante', () => {
    const base = partidaConTropas();
    // Reserva + 30: hay margen, pero muchísimo menos que un carro entero.
    const reserva = reservaDeTrigo(base.sesion.getState().asentamientos[0]!);
    const { sesion, asentamientoId, fundador } = conTrigo(base, reserva + 30);

    sesion.ejecutar(movilizarEjercito, { asentamientoId, jugadorId: fundador, escuadronIds: ['esc-milicia'], objetivo: PUNTO_LEJOS }, OPC);

    const estado = sesion.getState();
    const cargado = estado.ejercitos[0]!.suministro['trigo']!;
    expect(cargado).toBeGreaterThan(0);
    expect(cargado).toBeLessThan(LOGISTICA.capacidadCarroPorJugador);
    // Lo que queda no baja de la reserva del asentamiento YA SIN esos escuadrones (que dejaron de comer aquí).
    expect(trigoDe(estado.asentamientos[0]!)).toBeGreaterThanOrEqual(reservaDeTrigo(estado.asentamientos[0]!) - 1e-9);
  });

  it('con el almacén por debajo de la reserva sale IGUAL, con el carro vacío — no se bloquea la salida', () => {
    const base = partidaConTropas();
    const { sesion, asentamientoId, fundador } = conTrigo(base, 1);

    const r = sesion.ejecutar(movilizarEjercito, { asentamientoId, jugadorId: fundador, escuadronIds: ['esc-milicia'], objetivo: PUNTO_LEJOS }, OPC);

    expect(r.ok).toBe(true);
    const estado = sesion.getState();
    expect(estado.ejercitos[0]!.suministro['trigo']).toBe(0);
    expect(trigoDe(estado.asentamientos[0]!)).toBe(1); // ni un grano: no se toca lo que está bajo reserva
  });

  it('el que se une trae SU carro y lo carga de SU asentamiento', () => {
    const { sesion, asentamientoId, fundador, vecino } = conTrigo(partidaConTropas(), 100000);
    sesion.ejecutar(movilizarEjercito, { asentamientoId, jugadorId: fundador, escuadronIds: ['esc-milicia'], objetivo: PUNTO_LEJOS }, OPC);
    const ejercitoId = sesion.getState().ejercitos[0]!.id;

    const r = sesion.ejecutar(unirseAEjercito, { ejercitoId, asentamientoId, jugadorId: vecino, escuadronIds: ['esc-vecino'] }, { actor: vecino });

    expect(r.ok).toBe(true);
    // Dos participantes, dos carros: el suministro dobla.
    expect(sesion.getState().ejercitos[0]!.suministro['trigo']).toBe(2 * LOGISTICA.capacidadCarroPorJugador);
  });

  it('unirse DOS veces no duplica el carro: el tope va contra los participantes, no contra las veces', () => {
    const { sesion, asentamientoId, fundador } = conTrigo(partidaConTropas(), 100000);
    sesion.ejecutar(movilizarEjercito, { asentamientoId, jugadorId: fundador, escuadronIds: ['esc-milicia'], objetivo: PUNTO_LEJOS }, OPC);
    const ejercitoId = sesion.getState().ejercitos[0]!.id;

    // El MISMO jugador suma más escuadrones suyos: sigue siendo un participante, así que sigue siendo un carro.
    sesion.ejecutar(unirseAEjercito, { ejercitoId, asentamientoId, jugadorId: fundador, escuadronIds: ['esc-mimbre'] }, OPC);
    sesion.ejecutar(unirseAEjercito, { ejercitoId, asentamientoId, jugadorId: fundador, escuadronIds: ['esc-honderos'] }, OPC);

    const ejercito = sesion.getState().ejercitos[0]!;
    expect(new Set(ejercito.escuadrones.map((e) => e.jugadorId)).size).toBe(1);
    expect(ejercito.suministro['trigo']).toBe(LOGISTICA.capacidadCarroPorJugador);
  });

  it('el trigo se conserva: lo que sale del almacén es exactamente lo que entra en el carro', () => {
    const { sesion, asentamientoId, fundador } = conTrigo(partidaConTropas(), 700);
    const antes = trigoDe(sesion.getState().asentamientos[0]!);

    sesion.ejecutar(movilizarEjercito, { asentamientoId, jugadorId: fundador, escuadronIds: ['esc-milicia'], objetivo: PUNTO_LEJOS }, OPC);

    const estado = sesion.getState();
    expect(trigoDe(estado.asentamientos[0]!) + estado.ejercitos[0]!.suministro['trigo']!).toBeCloseTo(antes);
  });
});

describe('alternarReabastecerAliados (Paso 8)', () => {
  it('abre y cierra el almacén a los aliados, y no repite evento si no cambia nada', () => {
    const { sesion, asentamientoId } = partidaConTropas();
    expect(sesion.getState().asentamientos[0]!.permiteReabastecerAliados ?? false, 'cerrado por defecto').toBe(false);

    const abrir = sesion.ejecutar(alternarReabastecerAliados, { asentamientoId, permitido: true }, OPC);
    expect(abrir.ok).toBe(true);
    expect(sesion.getState().asentamientos[0]!.permiteReabastecerAliados).toBe(true);

    // Volver a abrir lo ya abierto no es un error, pero tampoco un hecho que narrar.
    const versionAntes = sesion.getState().version;
    const repetir = sesion.ejecutar(alternarReabastecerAliados, { asentamientoId, permitido: true }, OPC);
    expect(repetir.ok).toBe(true);
    expect(sesion.getState().version, 'un no-cambio no sube la versión de la partida').toBe(versionAntes);

    sesion.ejecutar(alternarReabastecerAliados, { asentamientoId, permitido: false }, OPC);
    expect(sesion.getState().asentamientos[0]!.permiteReabastecerAliados).toBe(false);
  });
});
