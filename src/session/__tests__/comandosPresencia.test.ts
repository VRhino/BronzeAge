// Presencia del jugador (`session/comandos/presencia.ts`, paso 3 del jugador situado, Doc 1.10).
//
// Es el primer punto en que la ubicación se MUEVE: hasta aquí el jugador tenía un campo que nadie escribía
// salvo fundar. Lo que este archivo congela es la asimetría del canon —de tu residencia se sale eligiendo,
// de una plaza ajena se sale con lo que llevabas— y las dos consecuencias que Doc 1.10.3 da por gratis: la
// columna aparcada sobrevive a lo que le pase a la plaza, y volver a casa la deshace.
import { describe, expect, it } from 'vitest';
import { GameSession } from '../gameSession';
import { entrarEnAsentamiento, salirAlMundo, salirDeAsentamiento } from '../comandos/presencia';
import { movilizarEjercito } from '../comandos/ejercitos';
import { OPC, partidaConAsentamiento } from './fixtures';
import { LOGISTICA, MOVIMIENTO } from '../../constants';

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
          escuadrones: [
            {
              id: 'esc-1',
              nombre: 'milicia_lanceros',
              jugadorId: base.fundador,
              origen: 'pesants' as const,
              cantidad: 10,
              veterania: 0,
              moral: 100,
              tropaId: 'milicia_lanceros',
            },
          ],
        },
        ...payload.state.asentamientos.slice(1),
      ],
    },
  });
  return { ...base, sesion };
}

const opcDe = (jugadorId: string) => ({ ...OPC, actor: jugadorId });

const ubicacionDe = (sesion: GameSession, jugadorId: string) => sesion.getState().jugadores.find((j) => j.id === jugadorId)!.ubicacion;

describe('salirAlMundo — desde la residencia, eligiendo (Doc 1.10.2)', () => {
  it('saca al jugador con las tropas y la carga que elige, y lo sitúa en su columna', () => {
    const { sesion, asentamientoId, fundador } = partidaLista();

    const r = sesion.ejecutar(
      salirAlMundo,
      { asentamientoId, jugadorId: fundador, escuadronIds: ['esc-1'], carga: { trigo: 100, madera: 50 } },
      opcDe(fundador)
    );

    expect(r.ok).toBe(true);
    const columna = sesion.getState().ejercitos[0]!;
    expect(columna.tipo, 'salir a lo tuyo hace una columna personal, no un ejército').toBe('personal');
    expect(columna.estado, 'nace parada junto a la plaza: el destino es un marcharA posterior').toBe('estacionado');
    expect(columna.suministro).toEqual({ trigo: 100, madera: 50 });
    expect(ubicacionDe(sesion, fundador)).toEqual({ tipo: 'columna', ejercitoId: columna.id });
    // Los escuadrones se van DE VERDAD: la guarnición es lo único que defiende (Doc 5.12.4).
    expect(sesion.getState().asentamientos[0]!.escuadrones).toHaveLength(0);
  });

  it('se puede salir SIN tropas: el viajero solo es una forma de jugar, no un error', () => {
    const { sesion, asentamientoId, fundador } = partidaLista();

    const r = sesion.ejecutar(salirAlMundo, { asentamientoId, jugadorId: fundador, escuadronIds: [], carga: {} }, opcDe(fundador));

    expect(r.ok).toBe(true);
    const columna = sesion.getState().ejercitos[0]!;
    expect(columna.escuadrones).toEqual([]);
    expect(columna.participantes.map((p) => p.jugadorId), 'sin tropas, pero va alguien dentro').toEqual([fundador]);
  });

  it('rechaza una carga que no cabe en el carro', () => {
    const { sesion, asentamientoId, fundador } = partidaLista();

    const r = sesion.ejecutar(
      salirAlMundo,
      { asentamientoId, jugadorId: fundador, escuadronIds: [], carga: { trigo: LOGISTICA.capacidadCarroPorJugador + 1 } },
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
      { asentamientoId, jugadorId: fundador, escuadronIds: [], carga: { trigo: mitad + 1, madera: mitad + 1 } },
      opcDe(fundador)
    );

    expect(r.ok, 'es un carro, no una estantería con un cajón por material').toBe(false);
  });

  it('no deja a la plaza sin la comida de su guarnición', () => {
    const { sesion, asentamientoId, fundador } = partidaLista();
    const disponible = sesion.getState().asentamientos[0]!.almacen['trigo']!.cantidad;

    const r = sesion.ejecutar(
      salirAlMundo,
      { asentamientoId, jugadorId: fundador, escuadronIds: [], carga: { trigo: disponible } },
      opcDe(fundador)
    );

    expect(r.ok, 'sacar el carro no puede vaciar la despensa por debajo de la reserva').toBe(false);
  });

  it('no se sale dos veces', () => {
    const { sesion, asentamientoId, fundador } = partidaLista();
    sesion.ejecutar(salirAlMundo, { asentamientoId, jugadorId: fundador, escuadronIds: [], carga: {} }, opcDe(fundador));

    const r = sesion.ejecutar(salirAlMundo, { asentamientoId, jugadorId: fundador, escuadronIds: [], carga: {} }, opcDe(fundador));

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
      { asentamientoId, jugadorId: fundador, escuadronIds: ['esc-1'], carga: { trigo: 100 } },
      opcDe(fundador)
    );

    const r = sesion.ejecutar(entrarEnAsentamiento, { asentamientoId, jugadorId: fundador }, opcDe(fundador));

    expect(r.ok).toBe(true);
    expect(sesion.getState().ejercitos, 'la columna deja de existir').toHaveLength(0);
    expect(sesion.getState().asentamientos[0]!.escuadrones.map((e) => e.id)).toEqual(['esc-1']);
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
      { asentamientoId, jugadorId: fundador, escuadronIds: ['esc-1'], carga: { trigo: 100 } },
      opcDe(fundador)
    );
    const payload = sesion.exportar();
    const a = payload.state.asentamientos[0]!;
    const ajena = GameSession.importar({
      ...payload,
      state: {
        ...payload.state,
        asentamientos: [
          { ...a, jugadoresFundadoresIds: a.jugadoresFundadoresIds.filter((id) => id !== fundador), casasCompradas: a.casasCompradas.filter((id) => id !== fundador) },
          ...payload.state.asentamientos.slice(1),
        ],
      },
    });

    const r = ajena.ejecutar(entrarEnAsentamiento, { asentamientoId, jugadorId: fundador }, opcDe(fundador));

    expect(r.ok).toBe(true);
    const columna = ajena.getState().ejercitos[0];
    expect(columna, 'la columna sigue ahí fuera, esperando').toBeDefined();
    expect(columna!.escuadrones.map((e) => e.id)).toEqual(['esc-1']);
    expect(columna!.suministro['trigo']).toBe(100);
    expect(ubicacionDe(ajena, fundador)).toEqual({ tipo: 'asentamiento', asentamientoId });
    // Y sigue contándolo como participante: `participantes` dice a quién PERTENECE la columna, no dónde está
    // su cuerpo. Sin esto la columna se quedaría sin nadie dentro y se disolvería sola (Doc 5.13.4).
    expect(columna!.participantes.map((p) => p.jugadorId)).toEqual([fundador]);
  });

  it('no se entra desde lejos: hay que estar en la puerta', () => {
    const { sesion, asentamientoId, fundador } = partidaLista();
    sesion.ejecutar(
      movilizarEjercito,
      { asentamientoId, jugadorId: fundador, escuadronIds: ['esc-1'], objetivo: { tipo: 'punto', punto: { x: 900, y: 900 } } },
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

    const r = lejos.ejecutar(entrarEnAsentamiento, { asentamientoId, jugadorId: fundador }, opcDe(fundador));

    expect(r.ok).toBe(false);
  });

  it('no se entra yendo en un EJÉRCITO: hay que separarse antes', () => {
    // Un ejército lleva a varios. Si se pudiera entrar desde dentro, volver a casa disolvería la columna con
    // la gente de los demás dentro — y sería una salida encubierta que se salta al Líder (Doc 5.14.2).
    const { sesion, asentamientoId, fundador } = partidaLista();
    sesion.ejecutar(
      movilizarEjercito,
      { asentamientoId, jugadorId: fundador, escuadronIds: ['esc-1'], objetivo: { tipo: 'punto', punto: { x: 900, y: 900 } } },
      opcDe(fundador)
    );

    const r = sesion.ejecutar(entrarEnAsentamiento, { asentamientoId, jugadorId: fundador }, opcDe(fundador));

    expect(r.ok).toBe(false);
    expect(sesion.getState().ejercitos, 'el ejército sigue entero').toHaveLength(1);
  });

  it('quien no está en el mundo no tiene puerta que cruzar', () => {
    const { sesion, asentamientoId, fundador } = partidaLista();

    const r = sesion.ejecutar(entrarEnAsentamiento, { asentamientoId, jugadorId: fundador }, opcDe(fundador));

    expect(r.ok).toBe(false);
  });
});

describe('salirDeAsentamiento — retomar lo aparcado (Doc 1.10.3)', () => {
  it('desde una plaza ajena devuelve al jugador a su columna, con lo que llevaba', () => {
    const { sesion, asentamientoId, fundador } = partidaLista();
    sesion.ejecutar(
      salirAlMundo,
      { asentamientoId, jugadorId: fundador, escuadronIds: ['esc-1'], carga: { trigo: 100 } },
      opcDe(fundador)
    );
    const payload = sesion.exportar();
    const a = payload.state.asentamientos[0]!;
    const ajena = GameSession.importar({
      ...payload,
      state: {
        ...payload.state,
        asentamientos: [
          { ...a, jugadoresFundadoresIds: a.jugadoresFundadoresIds.filter((id) => id !== fundador), casasCompradas: a.casasCompradas.filter((id) => id !== fundador) },
          ...payload.state.asentamientos.slice(1),
        ],
      },
    });
    ajena.ejecutar(entrarEnAsentamiento, { asentamientoId, jugadorId: fundador }, opcDe(fundador));

    const r = ajena.ejecutar(salirDeAsentamiento, { asentamientoId, jugadorId: fundador }, opcDe(fundador));

    expect(r.ok).toBe(true);
    const columna = ajena.getState().ejercitos[0]!;
    expect(ubicacionDe(ajena, fundador)).toEqual({ tipo: 'columna', ejercitoId: columna.id });
    expect(columna.suministro['trigo'], 'sin pantalla de equipamiento: sales con lo que tenías').toBe(100);
  });

  it('desde tu propia residencia se rechaza, y manda a `salirAlMundo`', () => {
    const { sesion, asentamientoId, fundador } = partidaLista();

    const r = sesion.ejecutar(salirDeAsentamiento, { asentamientoId, jugadorId: fundador }, opcDe(fundador));

    expect(r.ok, 'en tu casa hay un roster y un almacén que elegir; eso es otra operación').toBe(false);
  });
});
