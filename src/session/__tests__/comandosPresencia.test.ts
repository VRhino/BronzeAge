// Presencia del jugador (`session/comandos/presencia.ts`, paso 3 del jugador situado, Doc 1.10).
//
// Es el primer punto en que la ubicación se MUEVE: hasta aquí el jugador tenía un campo que nadie escribía
// salvo fundar. Lo que este archivo congela es la asimetría del canon —de tu residencia se sale eligiendo,
// de una plaza ajena se sale con lo que llevabas— y las dos consecuencias que Doc 1.10.3 da por gratis: la
// columna aparcada sobrevive a lo que le pase a la plaza, y volver a casa la deshace.
import { describe, expect, it } from 'vitest';
import { GameSession } from '../gameSession';
import { entrarEnAsentamiento, fijarPoliticaDeAcceso, guarnecer, marcharA, salirAlMundo, salirDeAsentamiento, vetarJugador } from '../comandos/presencia';
import { movilizarEjercito } from '../comandos/ejercitos';
import { OPC, partidaConAsentamiento } from './fixtures';
import { FUNDACION, LOGISTICA, MOVIMIENTO, VISION } from '../../constants';
import { exigirPuertaDeFundacion } from '../../engine/settlement';
import { proyectarParaJugador } from '../proyecciones/jugador';
import type { GeometriaAsentamientos } from '../estado';
import { campamentoDe, conEscuadrones } from '../../engine/tropa';
import { escuadronDePrueba } from '../../engine/__tests__/fixtures';

/** Sin geometria calculada: la proyeccion no la necesita para lo que se prueba aqui. */
const SIN_GEOMETRIA: GeometriaAsentamientos = { zonas: [], zonasFusionadas: [], trazadoPorAsentamiento: {} };

/** Partida con un escuadrón del fundador ya puesto, y trigo de sobra para poder cargar el carro. */
function partidaLista() {
  const base = partidaConAsentamiento();
  const payload = base.sesion.exportar();
  const a = payload.state.asentamientos[0]!;
  const sesion = GameSession.importar({
    ...payload,
    state: {
      ...payload.state,
      asentamientos: [
        {
          ...a,
          almacen: { ...a.almacen, trigo: { capacidad: 100000, cantidad: 5000 }, madera: { capacidad: 100000, cantidad: 5000 } },
        },
        ...payload.state.asentamientos.slice(1),
      ],
      heroes: conEscuadrones(payload.state.heroes, [escuadronDePrueba('esc-1', base.fundador)]),
    },
  });
  return { ...base, sesion };
}

/** El campamento de la plaza de la fixture: lo que la defiende (Doc 5.15.2). */
const campamento = (sesion: GameSession) => campamentoDe(sesion.getState().asentamientos[0]!, sesion.getState().heroes);

const opcDe = (heroeId: string) => ({ ...OPC, actor: heroeId });

// La plaza del fixture está en (400,400), en llano. (500,500) es AGUA en la seed 42 — el mismo dato que
// obliga a `partidaConAsentamiento` a fundar en (400,400) y no ahí.
const TIERRA_FIRME = { x: 430, y: 430 };
const AGUA = { x: 500, y: 500 };

const ubicacionDe = (sesion: GameSession, heroeId: string) => sesion.getState().heroes.find((j) => j.id === heroeId)!.ubicacion;

describe('salirAlMundo — desde la residencia, eligiendo (Doc 1.10.2)', () => {
  it('saca al jugador con las tropas y la carga que elige, y lo sitúa en su columna', () => {
    const { sesion, asentamientoId, fundador } = partidaLista();

    const r = sesion.ejecutar(
      salirAlMundo,
      { asentamientoId, heroeId: fundador, escuadronIds: ['esc-1'], carga: { trigo: 100, madera: 50 } },
      opcDe(fundador)
    );

    expect(r.ok).toBe(true);
    const columna = sesion.getState().ejercitos[0]!;
    expect(columna.tipo, 'salir a lo tuyo hace una columna personal, no un ejército').toBe('personal');
    expect(columna.estado, 'nace parada junto a la plaza: el destino es un marcharA posterior').toBe('estacionado');
    expect(columna.suministro).toEqual({ trigo: 100, madera: 50 });
    expect(ubicacionDe(sesion, fundador)).toEqual({ tipo: 'columna', ejercitoId: columna.id });
    // Los escuadrones se van DE VERDAD: la guarnición es lo único que defiende (Doc 5.12.4).
    expect(campamento(sesion)).toHaveLength(0);
  });

  it('se puede salir SIN tropas: el viajero solo es una forma de jugar, no un error', () => {
    const { sesion, asentamientoId, fundador } = partidaLista();

    const r = sesion.ejecutar(salirAlMundo, { asentamientoId, heroeId: fundador, escuadronIds: [], carga: {} }, opcDe(fundador));

    expect(r.ok).toBe(true);
    const columna = sesion.getState().ejercitos[0]!;
    expect(columna.escuadronIds).toEqual([]);
    expect(columna.participantes.map((p) => p.heroeId), 'sin tropas, pero va alguien dentro').toEqual([fundador]);
  });

  it('rechaza una carga que no cabe en el carro', () => {
    const { sesion, asentamientoId, fundador } = partidaLista();

    const r = sesion.ejecutar(
      salirAlMundo,
      { asentamientoId, heroeId: fundador, escuadronIds: [], carga: { trigo: LOGISTICA.capacidadCarroPorJugador + 1 } },
      opcDe(fundador)
    );

    expect(r.ok, 'el carro tiene un tope y pedir de más es un error, no un recorte silencioso').toBe(false);
    expect(sesion.getState().ejercitos).toHaveLength(0);
  });

  it('el tope del carro se mide sobre el TOTAL, no por recurso', () => {
    const { sesion, asentamientoId, fundador } = partidaLista();
    const mitad = LOGISTICA.capacidadCarroPorJugador / 2;

    const r = sesion.ejecutar(
      salirAlMundo,
      { asentamientoId, heroeId: fundador, escuadronIds: [], carga: { trigo: mitad + 1, madera: mitad + 1 } },
      opcDe(fundador)
    );

    expect(r.ok, 'es un carro, no una estantería con un cajón por material').toBe(false);
  });

  it('no deja a la plaza sin la comida de su guarnición', () => {
    const { sesion, asentamientoId, fundador } = partidaLista();
    const disponible = sesion.getState().asentamientos[0]!.almacen['trigo']!.cantidad;

    const r = sesion.ejecutar(
      salirAlMundo,
      { asentamientoId, heroeId: fundador, escuadronIds: [], carga: { trigo: disponible } },
      opcDe(fundador)
    );

    expect(r.ok, 'sacar el carro no puede vaciar la despensa por debajo de la reserva').toBe(false);
  });

  it('no se sale dos veces', () => {
    const { sesion, asentamientoId, fundador } = partidaLista();
    sesion.ejecutar(salirAlMundo, { asentamientoId, heroeId: fundador, escuadronIds: [], carga: {} }, opcDe(fundador));

    const r = sesion.ejecutar(salirAlMundo, { asentamientoId, heroeId: fundador, escuadronIds: [], carga: {} }, opcDe(fundador));

    expect(r.ok).toBe(false);
    expect(sesion.getState().ejercitos).toHaveLength(1);
  });
});

describe('entrarEnAsentamiento — la puerta (Doc 1.10.3)', () => {
  it('volver a la propia residencia DESHACE la columna: tropas a la guarnición y carro al almacén', () => {
    const { sesion, asentamientoId, fundador } = partidaLista();
    const trigoAntes = sesion.getState().asentamientos[0]!.almacen['trigo']!.cantidad;
    sesion.ejecutar(
      salirAlMundo,
      { asentamientoId, heroeId: fundador, escuadronIds: ['esc-1'], carga: { trigo: 100 } },
      opcDe(fundador)
    );

    const r = sesion.ejecutar(entrarEnAsentamiento, { asentamientoId, heroeId: fundador }, opcDe(fundador));

    expect(r.ok).toBe(true);
    expect(sesion.getState().ejercitos, 'la columna deja de existir').toHaveLength(0);
    expect(campamento(sesion).map((e) => e.id)).toEqual(['esc-1']);
    expect(sesion.getState().asentamientos[0]!.almacen['trigo']!.cantidad).toBe(trigoAntes);
    expect(ubicacionDe(sesion, fundador)).toEqual({ tipo: 'asentamiento', asentamientoId });
  });

  it('entrar en una plaza que NO es tu residencia deja la columna aparcada, intacta', () => {
    // El vecino compró casa en la plaza del fundador, así que reside ahí; para él esa plaza SÍ es su
    // residencia. Se usa al fundador entrando en una plaza de su Facción de la que no es residente: se
    // fuerza quitándolo de los fundadores, que es la vía de residencia que tenía.
    const { sesion, asentamientoId, fundador } = partidaLista();
    sesion.ejecutar(
      salirAlMundo,
      { asentamientoId, heroeId: fundador, escuadronIds: ['esc-1'], carga: { trigo: 100 } },
      opcDe(fundador)
    );
    const payload = sesion.exportar();
    const a = payload.state.asentamientos[0]!;
    const ajena = GameSession.importar({
      ...payload,
      state: {
        ...payload.state,
        asentamientos: [
          { ...a, heroesFundadoresIds: a.heroesFundadoresIds.filter((id) => id !== fundador), casasCompradas: a.casasCompradas.filter((id) => id !== fundador) },
          ...payload.state.asentamientos.slice(1),
        ],
      },
    });

    const r = ajena.ejecutar(entrarEnAsentamiento, { asentamientoId, heroeId: fundador }, opcDe(fundador));

    expect(r.ok).toBe(true);
    const columna = ajena.getState().ejercitos[0];
    expect(columna, 'la columna sigue ahí fuera, esperando').toBeDefined();
    expect(columna!.escuadronIds).toEqual(['esc-1']);
    expect(columna!.suministro['trigo']).toBe(100);
    expect(ubicacionDe(ajena, fundador)).toEqual({ tipo: 'asentamiento', asentamientoId });
    // Y sigue contándolo como participante: `participantes` dice a quién PERTENECE la columna, no dónde está
    // su cuerpo. Sin esto la columna se quedaría sin nadie dentro y se disolvería sola (Doc 5.13.4).
    expect(columna!.participantes.map((p) => p.heroeId)).toEqual([fundador]);
  });

  it('no se entra desde lejos: hay que estar en la puerta', () => {
    const { sesion, asentamientoId, fundador } = partidaLista();
    sesion.ejecutar(
      movilizarEjercito,
      { asentamientoId, heroeId: fundador, escuadronIds: ['esc-1'], objetivo: { tipo: 'punto', punto: { x: 900, y: 900 } } },
      opcDe(fundador)
    );
    const payload = sesion.exportar();
    const lejos = GameSession.importar({
      ...payload,
      state: {
        ...payload.state,
        ejercitos: [{ ...payload.state.ejercitos[0]!, posicionActual: { x: 400 + MOVIMIENTO.radioPuerta + 1, y: 400 } }],
      },
    });

    const r = lejos.ejecutar(entrarEnAsentamiento, { asentamientoId, heroeId: fundador }, opcDe(fundador));

    expect(r.ok).toBe(false);
  });

  it('no se entra yendo en un EJÉRCITO: hay que separarse antes', () => {
    // Un ejército lleva a varios. Si se pudiera entrar desde dentro, volver a casa disolvería la columna con
    // la gente de los demás dentro — y sería una salida encubierta que se salta al Líder (Doc 5.14.2).
    const { sesion, asentamientoId, fundador } = partidaLista();
    sesion.ejecutar(
      movilizarEjercito,
      { asentamientoId, heroeId: fundador, escuadronIds: ['esc-1'], objetivo: { tipo: 'punto', punto: { x: 900, y: 900 } } },
      opcDe(fundador)
    );

    const r = sesion.ejecutar(entrarEnAsentamiento, { asentamientoId, heroeId: fundador }, opcDe(fundador));

    expect(r.ok).toBe(false);
    expect(sesion.getState().ejercitos, 'el ejército sigue entero').toHaveLength(1);
  });

  it('quien no está en el mundo no tiene puerta que cruzar', () => {
    const { sesion, asentamientoId, fundador } = partidaLista();

    const r = sesion.ejecutar(entrarEnAsentamiento, { asentamientoId, heroeId: fundador }, opcDe(fundador));

    expect(r.ok).toBe(false);
  });
});

describe('salirDeAsentamiento — retomar lo aparcado (Doc 1.10.3)', () => {
  it('desde una plaza ajena devuelve al jugador a su columna, con lo que llevaba', () => {
    const { sesion, asentamientoId, fundador } = partidaLista();
    sesion.ejecutar(
      salirAlMundo,
      { asentamientoId, heroeId: fundador, escuadronIds: ['esc-1'], carga: { trigo: 100 } },
      opcDe(fundador)
    );
    const payload = sesion.exportar();
    const a = payload.state.asentamientos[0]!;
    const ajena = GameSession.importar({
      ...payload,
      state: {
        ...payload.state,
        asentamientos: [
          { ...a, heroesFundadoresIds: a.heroesFundadoresIds.filter((id) => id !== fundador), casasCompradas: a.casasCompradas.filter((id) => id !== fundador) },
          ...payload.state.asentamientos.slice(1),
        ],
      },
    });
    ajena.ejecutar(entrarEnAsentamiento, { asentamientoId, heroeId: fundador }, opcDe(fundador));

    const r = ajena.ejecutar(salirDeAsentamiento, { asentamientoId, heroeId: fundador }, opcDe(fundador));

    expect(r.ok).toBe(true);
    const columna = ajena.getState().ejercitos[0]!;
    expect(ubicacionDe(ajena, fundador)).toEqual({ tipo: 'columna', ejercitoId: columna.id });
    expect(columna.suministro['trigo'], 'sin pantalla de equipamiento: sales con lo que tenías').toBe(100);
  });

  it('desde tu propia residencia se rechaza, y manda a `salirAlMundo`', () => {
    const { sesion, asentamientoId, fundador } = partidaLista();

    const r = sesion.ejecutar(salirDeAsentamiento, { asentamientoId, heroeId: fundador }, opcDe(fundador));

    expect(r.ok, 'en tu casa hay un roster y un almacén que elegir; eso es otra operación').toBe(false);
  });
});

describe('guarnecer — un ejército marcha a una plaza propia y vuelca la tropa (Ocupacion §2.3)', () => {
  /** Fundador con un EJÉRCITO (movilizado, no columna personal) parado en la puerta de su propia plaza. */
  function ejercitoEnLaPuerta() {
    const { sesion, asentamientoId, fundador } = partidaLista();
    sesion.ejecutar(
      movilizarEjercito,
      { asentamientoId, heroeId: fundador, escuadronIds: ['esc-1'], objetivo: { tipo: 'punto', punto: { x: 900, y: 900 } } },
      opcDe(fundador)
    );
    const payload = sesion.exportar();
    const plaza = payload.state.asentamientos[0]!;
    return {
      asentamientoId,
      fundador,
      sesion: GameSession.importar({
        ...payload,
        state: {
          ...payload.state,
          ejercitos: [{ ...payload.state.ejercitos[0]!, posicionActual: plaza.posicion, estado: 'estacionado' as const }],
        },
      }),
    };
  }

  it('vuelca los escuadrones en la guarnición, consume el ejército y deja al jugador dentro', () => {
    const { sesion, asentamientoId, fundador } = ejercitoEnLaPuerta();

    const r = sesion.ejecutar(guarnecer, { asentamientoId, heroeId: fundador }, opcDe(fundador));

    expect(r.ok).toBe(true);
    expect(sesion.getState().ejercitos, 'el ejército se consume').toHaveLength(0);
    expect(campamento(sesion).map((e) => e.id)).toEqual(['esc-1']);
    expect(ubicacionDe(sesion, fundador)).toEqual({ tipo: 'asentamiento', asentamientoId });
  });

  it('una columna PERSONAL no guarnece: hay que separarse antes', () => {
    const { sesion, asentamientoId, fundador } = partidaLista();
    sesion.ejecutar(salirAlMundo, { asentamientoId, heroeId: fundador, escuadronIds: [], carga: {} }, opcDe(fundador));

    const r = sesion.ejecutar(guarnecer, { asentamientoId, heroeId: fundador }, opcDe(fundador));

    expect(r.ok).toBe(false);
  });
});

describe('marcharA — el destino de un viajero se rectifica (Doc 5.12.1)', () => {
  /** Fundador ya fuera, parado junto a su plaza: el estado en que lo deja `salirAlMundo`. */
  function fuera() {
    const base = partidaLista();
    base.sesion.ejecutar(
      salirAlMundo,
      { asentamientoId: base.asentamientoId, heroeId: base.fundador, escuadronIds: ['esc-1'], carga: { trigo: 100 } },
      opcDe(base.fundador)
    );
    return base;
  }

  it('pone en marcha a la columna que salió parada', () => {
    const { sesion, fundador } = fuera();
    expect(sesion.getState().ejercitos[0]!.estado).toBe('estacionado');

    const r = sesion.ejecutar(marcharA, { heroeId: fundador, objetivo: { tipo: 'punto', punto: TIERRA_FIRME } }, opcDe(fundador));

    expect(r.ok).toBe(true);
    const columna = sesion.getState().ejercitos[0]!;
    expect(columna.estado).toBe('marchando');
    expect(columna.ruta.length).toBeGreaterThan(1);
    expect(columna.progreso).toBe(0);
  });

  it('se rectifica en marcha, desde donde esté y cuantas veces quiera', () => {
    const { sesion, fundador } = fuera();
    sesion.ejecutar(marcharA, { heroeId: fundador, objetivo: { tipo: 'punto', punto: TIERRA_FIRME } }, opcDe(fundador));
    // Se le mueve a mitad de camino para comprobar que la ruta nueva sale de AHÍ y no del origen.
    const payload = sesion.exportar();
    const aMitad = GameSession.importar({
      ...payload,
      state: { ...payload.state, ejercitos: [{ ...payload.state.ejercitos[0]!, posicionActual: { x: 450, y: 450 }, progreso: 0.5 }] },
    });

    const r = aMitad.ejecutar(marcharA, { heroeId: fundador, objetivo: { tipo: 'punto', punto: { x: 380, y: 460 } } }, opcDe(fundador));

    expect(r.ok).toBe(true);
    const columna = aMitad.getState().ejercitos[0]!;
    expect(columna.ruta[0], 'la ruta nueva empieza donde estaba, no en su plaza').toEqual({ x: 450, y: 450 });
    expect(columna.progreso, 'y el progreso se reinicia sobre la ruta nueva').toBe(0);
  });

  it('un EJÉRCITO no cambia de rumbo, aunque vaya uno solo dentro', () => {
    // Es la regla que §1.1f corrigió: la línea no es cuánta gente va dentro, es cómo salió la columna.
    const { sesion, asentamientoId, fundador } = partidaLista();
    sesion.ejecutar(
      movilizarEjercito,
      { asentamientoId, heroeId: fundador, escuadronIds: ['esc-1'], objetivo: { tipo: 'punto', punto: { x: 900, y: 900 } } },
      opcDe(fundador)
    );
    expect(sesion.getState().ejercitos[0]!.participantes).toHaveLength(1);

    const r = sesion.ejecutar(marcharA, { heroeId: fundador, objetivo: { tipo: 'punto', punto: TIERRA_FIRME } }, opcDe(fundador));

    expect(r.ok, 'el rumbo se acordó al salir: su salida es cancelar y volver').toBe(false);
    expect(sesion.getState().ejercitos[0]!.objetivo).toEqual({ tipo: 'punto', punto: { x: 900, y: 900 } });
  });

  it('no se marcha desde dentro de una plaza: hay que salir antes', () => {
    const { sesion, asentamientoId, fundador } = fuera();
    sesion.ejecutar(entrarEnAsentamiento, { asentamientoId, heroeId: fundador }, opcDe(fundador));

    const r = sesion.ejecutar(marcharA, { heroeId: fundador, objetivo: { tipo: 'punto', punto: TIERRA_FIRME } }, opcDe(fundador));

    expect(r.ok).toBe(false);
  });

  it('no hay marcha por mar', () => {
    const { sesion, fundador } = fuera();

    const r = sesion.ejecutar(marcharA, { heroeId: fundador, objetivo: { tipo: 'punto', punto: AGUA } }, opcDe(fundador));

    expect(r.ok).toBe(false);
  });
});

describe('la puerta la controla el Gobernador (Doc 1.10.5)', () => {
  /** El fundador fuera con su columna, y esa misma plaza dejando de ser su residencia: el forastero. */
  function forasteroEnLaPuerta() {
    const base = partidaLista();
    base.sesion.ejecutar(
      salirAlMundo,
      { asentamientoId: base.asentamientoId, heroeId: base.fundador, escuadronIds: [], carga: {} },
      opcDe(base.fundador)
    );
    const payload = base.sesion.exportar();
    const a = payload.state.asentamientos[0]!;
    const sesion = GameSession.importar({
      ...payload,
      state: {
        ...payload.state,
        asentamientos: [
          {
            ...a,
            heroesFundadoresIds: a.heroesFundadoresIds.filter((id) => id !== base.fundador),
            casasCompradas: a.casasCompradas.filter((id) => id !== base.fundador),
            // El vecino se queda de Gobernador: alguien tiene que poder tocar la puerta.
            cargos: { ...a.cargos, gobernadorId: base.vecino },
          },
          ...payload.state.asentamientos.slice(1),
        ],
      },
    });
    return { ...base, sesion };
  }

  it('por defecto una plaza deja entrar a los suyos', () => {
    const { sesion, asentamientoId, fundador } = forasteroEnLaPuerta();
    expect(sesion.getState().asentamientos[0]!.politicaDeAcceso, 'sin decidir nada').toBeUndefined();

    expect(sesion.ejecutar(entrarEnAsentamiento, { asentamientoId, heroeId: fundador }, opcDe(fundador)).ok).toBe(true);
  });

  it('cerrada, no entra ni uno de la propia Facción', () => {
    const { sesion, asentamientoId, fundador, vecino } = forasteroEnLaPuerta();
    sesion.ejecutar(fijarPoliticaDeAcceso, { asentamientoId, heroeId: vecino, politica: 'cerrado' }, opcDe(vecino));

    expect(sesion.ejecutar(entrarEnAsentamiento, { asentamientoId, heroeId: fundador }, opcDe(fundador)).ok).toBe(false);
  });

  it('el veto pesa MÁS que la política: abierta a todos menos a ti', () => {
    const { sesion, asentamientoId, fundador, vecino } = forasteroEnLaPuerta();
    sesion.ejecutar(fijarPoliticaDeAcceso, { asentamientoId, heroeId: vecino, politica: 'abierto' }, opcDe(vecino));
    sesion.ejecutar(vetarJugador, { asentamientoId, heroeId: vecino, vetadoId: fundador, vetar: true }, opcDe(vecino));

    expect(sesion.ejecutar(entrarEnAsentamiento, { asentamientoId, heroeId: fundador }, opcDe(fundador)).ok).toBe(false);
  });

  it('y se levanta con el mismo comando', () => {
    const { sesion, asentamientoId, fundador, vecino } = forasteroEnLaPuerta();
    sesion.ejecutar(fijarPoliticaDeAcceso, { asentamientoId, heroeId: vecino, politica: 'abierto' }, opcDe(vecino));
    sesion.ejecutar(vetarJugador, { asentamientoId, heroeId: vecino, vetadoId: fundador, vetar: true }, opcDe(vecino));

    sesion.ejecutar(vetarJugador, { asentamientoId, heroeId: vecino, vetadoId: fundador, vetar: false }, opcDe(vecino));

    expect(sesion.ejecutar(entrarEnAsentamiento, { asentamientoId, heroeId: fundador }, opcDe(fundador)).ok).toBe(true);
  });

  it('un RESIDENTE entra aunque la plaza esté cerrada: nadie se queda fuera de su casa', () => {
    const { sesion, asentamientoId, fundador, vecino } = partidaLista();
    sesion.ejecutar(salirAlMundo, { asentamientoId, heroeId: fundador, escuadronIds: [], carga: {} }, opcDe(fundador));
    const payload = sesion.exportar();
    const a = payload.state.asentamientos[0]!;
    const cerrada = GameSession.importar({
      ...payload,
      state: {
        ...payload.state,
        asentamientos: [{ ...a, politicaDeAcceso: 'cerrado' as const, cargos: { ...a.cargos, gobernadorId: vecino } }, ...payload.state.asentamientos.slice(1)],
      },
    });

    expect(cerrada.ejecutar(entrarEnAsentamiento, { asentamientoId, heroeId: fundador }, opcDe(fundador)).ok).toBe(true);
  });

  it('y a un residente no se le veta: eso sería expulsarlo sin pasar por el exilio', () => {
    const { sesion, asentamientoId, fundador, vecino } = partidaLista();
    const payload = sesion.exportar();
    const a = payload.state.asentamientos[0]!;
    const conGobernador = GameSession.importar({
      ...payload,
      state: { ...payload.state, asentamientos: [{ ...a, cargos: { ...a.cargos, gobernadorId: vecino } }, ...payload.state.asentamientos.slice(1)] },
    });

    const r = conGobernador.ejecutar(vetarJugador, { asentamientoId, heroeId: vecino, vetadoId: fundador, vetar: true }, opcDe(vecino));

    expect(r.ok).toBe(false);
  });
});

// La puerta de fundacion (Consideraciones/Entrada_Al_Mundo_Definicion.md, decisiones 2-3).
describe('la puerta de fundacion: el freno a la ola', () => {
  it('con las palancas abiertas se funda como siempre — es la configuracion de las primeras pruebas', () => {
    expect(FUNDACION.minFundadoresParaFaccionNueva, 'grupo de 1 mientras haya cinco testers').toBe(1);
    expect(FUNDACION.exigeCiudadaniaPrevia, 'ciudadania opcional de momento').toBe(false);

    const { sesion, asentamientoId } = partidaLista();

    expect(sesion.getState().asentamientos.map((a) => a.id)).toContain(asentamientoId);
  });

  it('exigirPuertaDeFundacion rechaza un grupo corto', () => {
    // Se prueba la puerta directamente y no via constante: lo que hay que congelar es que la regla EXISTE y
    // muerde, no el valor de hoy, que esta puesto para que no muerda.
    expect(() => exigirPuertaDeFundacion([], true)).toThrow();
  });

  it('y rechaza a quien nunca fue ciudadano, cuando se exige', () => {
    // Mismo motivo: cuando la palanca se cierre, esto es lo que tiene que pasar.
    const original = FUNDACION.exigeCiudadaniaPrevia;
    try {
      (FUNDACION as { exigeCiudadaniaPrevia: boolean }).exigeCiudadaniaPrevia = true;
      expect(() => exigirPuertaDeFundacion(['alguien'], false)).toThrow();
      expect(() => exigirPuertaDeFundacion(['alguien'], true), 'un ex-ciudadano si').not.toThrow();
    } finally {
      (FUNDACION as { exigeCiudadaniaPrevia: boolean }).exigeCiudadaniaPrevia = original;
    }
  });
});

// Existir sin bandera (Consideraciones/Entrada_Al_Mundo_Definicion.md §0): sin esto no hay vestibulo posible.
describe('un jugador sin Faccion ve el mundo desde su columna', () => {
  it('ve su PROPIA columna, que es lo minimo para poder jugar', () => {
    const { sesion, asentamientoId, fundador } = partidaLista();
    sesion.ejecutar(salirAlMundo, { asentamientoId, heroeId: fundador, escuadronIds: [], carga: {} }, opcDe(fundador));
    // Se le quita la Faccion: es el estado de un recien llegado, que no tiene ninguna.
    const payload = sesion.exportar();
    const sinBandera = GameSession.importar({
      ...payload,
      state: { ...payload.state, facciones: payload.state.facciones.map((f) => ({ ...f, ciudadanosIds: [] })) },
    });

    const proyeccion = proyectarParaJugador(sinBandera.getState(), fundador, SIN_GEOMETRIA);

    expect(proyeccion.faccionId, 'no tiene bandera').toBeNull();
    expect(proyeccion.ejercitos.map((e) => e.id), 'pero se ve a si mismo').toHaveLength(1);
  });

  it('y ve MENOS que una columna con tropa: un hombre solo no despliega batidores', () => {
    expect(VISION.jugadorSolo).toBeLessThan(VISION.ejercito);
    expect(VISION.jugadorSolo, 'pero mas que el anillo de inspeccion: ver y mirar de cerca siguen siendo distintos').toBeGreaterThan(
      MOVIMIENTO.radioInspeccion
    );
  });
});
