// Batallas de Unity del lado de BronzeAge, fase 1 (doc 01 §15, Doc 5.15.1): abrirlas con el ticket del contrato,
// bloquear lo que interviene, unirse, cancelar y vencer. Con el flag apagado todo sigue con números, y eso lo cubren
// los tests de cada combate; aquí basta con uno que lo confirme.
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv from 'ajv';
import { describe, expect, it } from 'vitest';
import { GameSession } from '../gameSession';
import { REGISTRO_COMANDOS } from '../comandos/registro';
import { verificarAutorizacion } from '../comandos/autorizacion';
import { CODIGOS_ERROR } from '../comandos/codigosDeError';
import { proyectarParaJugador } from '../proyecciones/jugador';
import { faccionesEnBatalla } from '../batallas';
import { instanteDeTick, type GeometriaAsentamientos } from '../estado';
import { abastecer, conHeroe, enPie, partidaConAsentamiento } from './fixtures';
import { campamentoDe } from '../../engine/tropa';
import { BATALLA } from '../../constants';

const SCHEMA = JSON.parse(readFileSync(join(dirname(fileURLToPath(import.meta.url)), '../../contratos/v1/contratos.schema.json'), 'utf8'));
const ajv = new Ajv({ allErrors: true, strict: true, strictRequired: false, allowUnionTypes: true });
ajv.addSchema(SCHEMA, 'v1');
function erroresDe(definicion: string, dato: unknown) {
  const validar = ajv.getSchema(`v1#/definitions/${definicion}`)!;
  return validar(dato) ? [] : validar.errors;
}

const SIN_GEOMETRIA: GeometriaAsentamientos = { zonas: [], zonasFusionadas: [], trazadoPorAsentamiento: {} };

/**
 * Fundador y vecino, compañeros de Facción, cada uno en su columna con su milicia, junto a un campamento de bandidos.
 * Con `unity`, la partida corre como en un servidor con servidores de batalla declarados.
 */
function frenteACampamento(unity = true) {
  const base = partidaConAsentamiento();
  const { asentamientoId, fundador, vecino } = base;
  let sesion = base.sesion;
  for (const heroeId of [fundador, vecino]) {
    sesion = abastecer(sesion);
    const reclutado = sesion.ejecutar(REGISTRO_COMANDOS.reclutarTropa, { asentamientoId, heroeId, tropaId: 'milicia_lanceros', origen: 'pesants' }, { actor: heroeId });
    if (!reclutado.ok) throw new Error(`setup: ${heroeId} no recluta (${reclutado.codigoError})`);
    sesion = abastecer(sesion);
    const suyas = campamentoDe(sesion.getState().asentamientos[0]!, sesion.getState().heroes).filter((e) => e.heroeId === heroeId);
    const salida = sesion.ejecutar(
      REGISTRO_COMANDOS.salirAlMundo,
      { asentamientoId, heroeId, escuadronIds: suyas.map((e) => e.id), carga: { trigo: 60 } },
      { actor: heroeId }
    );
    if (!salida.ok) throw new Error(`setup: ${heroeId} no sale (${salida.codigoError})`);
  }

  const payload = sesion.exportar();
  const columnaDe = (heroeId: string) => payload.state.ejercitos.find((e) => e.participantes.some((p) => p.heroeId === heroeId))!;
  const posicion = columnaDe(fundador).posicionActual;
  const conCampamento = GameSession.importar(
    { ...payload, state: { ...payload.state, campamentosBandidos: [{ id: 'camp-1', posicion, bosqueId: 'b1', asentamientoId, poder: 30 }] } },
    { batallasEnUnity: unity }
  );
  return { sesion: conCampamento, fundador, vecino, columna: columnaDe(fundador).id, columnaVecino: columnaDe(vecino).id };
}

const atacarCampamento = (sesion: GameSession, heroeId: string) =>
  sesion.ejecutar(REGISTRO_COMANDOS.atacar, { heroeId, objetivo: { tipo: 'campamento', id: 'camp-1' } }, { actor: heroeId });

const dejarDePerseguir = (sesion: GameSession, heroeId: string) =>
  sesion.ejecutar(REGISTRO_COMANDOS.dejarDePerseguir, { heroeId }, { actor: heroeId });

const reservadas = (sesion: GameSession, heroeId: string) =>
  sesion.getState().heroes.find((h) => h.id === heroeId)!.escuadrones.filter((e) => e.reservaBatalla !== undefined);

describe('abrir una batalla de Unity (doc 01 §15)', () => {
  it('sin servidores de batalla, atacar se sigue resolviendo con números', () => {
    const { sesion, fundador } = frenteACampamento(false);

    const r = atacarCampamento(sesion, fundador);

    expect(r.ok).toBe(true);
    expect(r.datos).toBeUndefined();
    expect(sesion.getState().batallas).toEqual([]);
  });

  it('con servidores, atacar un campamento abre una batalla con el ticket del contrato y reserva las escuadras', () => {
    const { sesion, fundador } = frenteACampamento();

    const r = atacarCampamento(sesion, fundador);

    expect(r.ok).toBe(true);
    const [batalla] = sesion.getState().batallas;
    expect(batalla!.id).toBe(r.datos!.battleId);
    expect(batalla!.estado).toBe('convocando');
    expect(erroresDe('BattleTicket', batalla!.ticket)).toEqual([]);
    expect(batalla!.ticket.bandos.atacante.participantes.map((p) => p.heroeId)).toEqual([fundador]);
    expect(batalla!.ticket.bandos.defensor.escuadrasSinHeroe).toMatchObject([{ heroeId: null, tropaId: 'milicia_lanceros', efectivosAutorizados: 15 }]);
    expect(reservadas(sesion, fundador).map((e) => e.reservaBatalla!.battleId)).toEqual([batalla!.id]);
    expect(sesion.getState().campamentosBandidos, 'no se resuelve aquí: el campamento sigue en pie').toHaveLength(1);
  });
});

describe('mientras se juega (Doc 5.15.1)', () => {
  it('lo que combate no recibe órdenes ni se mueve, y nadie más ataca el campamento', () => {
    const { sesion, fundador, vecino, columna } = frenteACampamento();
    atacarCampamento(sesion, fundador);
    const antes = sesion.getState().ejercitos.find((e) => e.id === columna);

    expect(dejarDePerseguir(sesion, fundador).codigoError).toBe(CODIGOS_ERROR.batallaBloqueo);
    expect(atacarCampamento(sesion, vecino).codigoError).toBe(CODIGOS_ERROR.batallaBloqueo);
    sesion.avanzarTick();
    expect(sesion.getState().ejercitos.find((e) => e.id === columna), 'fuera del tick: ni se mueve ni come').toEqual(antes);
  });

  it('si nadie la asigna a tiempo, falla sin castigo y suelta los candados', () => {
    const { sesion, fundador } = frenteACampamento();
    atacarCampamento(sesion, fundador);

    for (let i = 0; i < BATALLA.plazoAsignacionMinutos; i++) sesion.avanzarTick();

    expect(sesion.getState().batallas[0]!.estado).toBe('fallida');
    expect(reservadas(sesion, fundador)).toEqual([]);
    expect(sesion.getState().campamentosBandidos.some((c) => c.id === 'camp-1')).toBe(true);
    expect(dejarDePerseguir(sesion, fundador).ok).toBe(true);
  });

  it('quien la inició la cancela antes de empezar; otro jugador no puede', () => {
    const { sesion, fundador, vecino } = frenteACampamento();
    const battleId = atacarCampamento(sesion, fundador).datos!.battleId;

    expect(verificarAutorizacion('cancelarBatalla', { battleId }, sesion.getState(), { rol: 'jugador', heroeId: vecino }).autorizado).toBe(false);
    expect(sesion.ejecutar(REGISTRO_COMANDOS.cancelarBatalla, { battleId }, { actor: fundador }).ok).toBe(true);
    expect(sesion.getState().batallas[0]!.estado).toBe('cancelada');
    expect(reservadas(sesion, fundador)).toEqual([]);
  });

  it('la IA de las Facciones que combaten no gobierna mientras dura', () => {
    const { sesion, fundador } = frenteACampamento();
    atacarCampamento(sesion, fundador);
    const estado = sesion.getState();
    const suya = estado.facciones.find((f) => f.ciudadanosIds.includes(fundador))!.id;

    expect([...faccionesEnBatalla(estado, instanteDeTick(estado.tick))]).toEqual([suya]);
  });
});

describe('unirse a una batalla (Doc 5.15.1)', () => {
  it('un compañero de Facción se une con su columna mientras queda sitio', () => {
    const { sesion, fundador, vecino, columnaVecino } = frenteACampamento();
    const battleId = atacarCampamento(sesion, fundador).datos!.battleId;

    const r = sesion.ejecutar(REGISTRO_COMANDOS.unirseABatalla, { heroeId: vecino, battleId }, { actor: vecino });

    expect(r.ok).toBe(true);
    const batalla = sesion.getState().batallas[0]!;
    expect(batalla.incorporaciones).toMatchObject([{ secuencia: 1, lado: 'atacante', participante: { heroeId: vecino } }]);
    expect(erroresDe('IncorporacionBatalla', batalla.incorporaciones[0])).toEqual([]);
    expect(batalla.bloqueo.ejercitoIds).toContain(columnaVecino);
    expect(reservadas(sesion, vecino)).toHaveLength(1);
    expect(dejarDePerseguir(sesion, vecino).codigoError).toBe(CODIGOS_ERROR.batallaBloqueo);
  });

  it('no entra en un bando lleno, ni quien no es de ninguno', () => {
    const { sesion, fundador, vecino } = frenteACampamento();
    const battleId = atacarCampamento(sesion, fundador).datos!.battleId;
    const unirse = (s: GameSession, heroeId: string) => s.ejecutar(REGISTRO_COMANDOS.unirseABatalla, { heroeId, battleId }, { actor: heroeId });

    expect(unirse(conCapacidad(sesion, 1), vecino).codigoError, 'el atacante ya está y el bando es de 1').toBe(CODIGOS_ERROR.batallaInvalida);
    const forastero = enPie(conHeroe(sesion, 'forastero'), 'forastero', sesion.getState().batallas[0]!.punto);
    expect(unirse(forastero, 'forastero').codigoError, 'sin Facción no es de ningún bando').toBe(CODIGOS_ERROR.batallaInvalida);
  });
});

describe('la batalla en el mapa (doc 02 §4.1)', () => {
  it('quien combate la ve con su bando, y su compañero la ve desde la plaza', () => {
    const { sesion, fundador, vecino } = frenteACampamento();
    const battleId = atacarCampamento(sesion, fundador).datos!.battleId;

    const propia = proyectarParaJugador(sesion.getState(), fundador, SIN_GEOMETRIA).batallas;
    const ajena = proyectarParaJugador(sesion.getState(), vecino, SIN_GEOMETRIA).batallas;

    expect(propia).toMatchObject([{ battleId, ladoPropio: 'atacante', bandos: { atacante: { heroes: 1, capacidadMaxima: 5 } } }]);
    expect(ajena).toMatchObject([{ battleId }]);
    expect(ajena[0]!.ladoPropio).toBeUndefined();
  });
});

/** La misma partida con el bando atacante de su batalla limitado a `capacidadMaxima` héroes. */
function conCapacidad(sesion: GameSession, capacidadMaxima: number): GameSession {
  const payload = sesion.exportar();
  const batallas = payload.state.batallas.map((b) => ({
    ...b,
    ticket: { ...b.ticket, bandos: { ...b.ticket.bandos, atacante: { ...b.ticket.bandos.atacante, capacidadMaxima } } },
  }));
  return GameSession.importar({ ...payload, state: { ...payload.state, batallas } });
}
