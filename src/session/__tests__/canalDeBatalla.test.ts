// El canal `batalla/<battleId>` (doc 02 §3.5): quién puede seguirlo, qué eventos llevan a él y, sobre todo, que lo que viaja
// es estado público y nunca un secreto.
import { describe, expect, it } from 'vitest';
import type { EventoDominio } from '../../domain/eventos';
import { SCHEMA_VERSION } from '../../contratos/v1/dto';
import { GameSession } from '../gameSession';
import { REGISTRO_COMANDOS } from '../comandos/registro';
import { confirmarInicio, registrarAsignacion, registrarSalida } from '../comandos/batalla';
import { CANAL_GENERAL, canalDeAsentamiento, canalDeBatalla, canalesDeEvento, puedeSuscribirseA } from '../canales';
import { conHeroe, enPie, frenteACampamento } from './fixtures';

const comoS1 = { actor: 'batalla-servidor:s1' };
const evento = (codigo: string, payload?: unknown, asentamientoId?: string): EventoDominio => ({
  codigo,
  mensaje: '',
  momento: '2026-01-01T00:00:00.000Z',
  ...(payload !== undefined ? { payload } : {}),
  ...(asentamientoId ? { asentamientoId } : {}),
});

describe('canalesDeEvento', () => {
  it('un evento de batalla va a su canal además del de siempre', () => {
    expect(canalesDeEvento(evento('batalla.abierta', { battleId: 'b-1' }, 'a1'))).toEqual([canalDeAsentamiento('a1'), 'batalla/b-1']);
    expect(canalesDeEvento(evento('evento_pve.abierta', { battleId: 'b-2' }, 'a1'))).toEqual([canalDeAsentamiento('a1'), canalDeBatalla('b-2')]);
  });

  it('lo demás va solo por el suyo, y un evento de batalla sin `battleId` tampoco inventa canal', () => {
    expect(canalesDeEvento(evento('faccion.creada'))).toEqual([CANAL_GENERAL]);
    expect(canalesDeEvento(evento('construccion.completada', { battleId: 'x' }, 'a1'))).toEqual([canalDeAsentamiento('a1')]);
    expect(canalesDeEvento(evento('batalla.abierta', {}, 'a1'))).toEqual([canalDeAsentamiento('a1')]);
  });
});

describe('quién puede seguir una batalla', () => {
  it('quien combata en ella, y quien la vea ahora bajo la niebla; no un forastero que no ve nada', () => {
    const { sesion, fundador, vecino } = frenteACampamento();
    const battleId = sesion.ejecutar(REGISTRO_COMANDOS.atacar, { heroeId: fundador, objetivo: { tipo: 'campamento', id: 'camp-1' } }, { actor: fundador }).datos!.battleId;
    const lejos = enPie(conHeroe(sesion, 'forastero'), 'forastero', { x: 1900, y: 1900 });
    const estado = sesion.getState();

    expect(puedeSuscribirseA(estado, fundador, canalDeBatalla(battleId)), 'combate en ella').toBe(true);
    expect(puedeSuscribirseA(estado, vecino, canalDeBatalla(battleId)), 'su compañero la ve desde la plaza').toBe(true);
    expect(puedeSuscribirseA(lejos.getState(), 'forastero', canalDeBatalla(battleId)), 'no la ve').toBe(false);
  });

  it('una batalla que no existe no autoriza', () => {
    const { sesion, fundador } = frenteACampamento();

    expect(puedeSuscribirseA(sesion.getState(), fundador, canalDeBatalla('no-existe'))).toBe(false);
  });

  it('quien salió de ella sigue pudiendo seguirla', () => {
    const { sesion, fundador, vecino } = frenteACampamento();
    const battleId = sesion.ejecutar(REGISTRO_COMANDOS.atacar, { heroeId: fundador, objetivo: { tipo: 'campamento', id: 'camp-1' } }, { actor: fundador }).datos!.battleId;
    sesion.ejecutar(REGISTRO_COMANDOS.unirseABatalla, { heroeId: vecino, battleId }, { actor: vecino });
    const base = { schemaVersion: SCHEMA_VERSION, battleId, ticketRevision: 0, intentoAsignacionId: 'intento-1' } as const;
    sesion.ejecutar(
      registrarAsignacion,
      { servidorId: 's1', mensaje: { ...base, instancia: { host: 'h', puerto: 7777, protocolo: 'udp' }, tokensParticipante: [] } },
      comoS1
    );
    sesion.ejecutar(registrarSalida, { servidorId: 's1', mensaje: { ...base, heroeId: vecino, motivo: 'no_conectado', derrotado: false, escuadras: [] } }, comoS1);

    const lejos = enPie(GameSession.importar(sesion.exportar()), vecino, { x: 1900, y: 1900 });
    expect(puedeSuscribirseA(lejos.getState(), vecino, canalDeBatalla(battleId))).toBe(true);
  });
});

describe('lo que viaja por el canal es público', () => {
  it('ningún evento del ciclo lleva tokens, escuadras ni listas de héroes, pero sí el recuento por bando', () => {
    const { sesion, fundador } = frenteACampamento();
    const eventos: EventoDominio[] = [];
    const recoger = (r: { eventos: readonly EventoDominio[] }) => eventos.push(...r.eventos);

    const abrir = sesion.ejecutar(REGISTRO_COMANDOS.atacar, { heroeId: fundador, objetivo: { tipo: 'campamento', id: 'camp-1' } }, { actor: fundador });
    recoger(abrir);
    const battleId = abrir.datos!.battleId;
    const base = { schemaVersion: SCHEMA_VERSION, battleId, ticketRevision: 0, intentoAsignacionId: 'intento-1' } as const;
    recoger(
      sesion.ejecutar(
        registrarAsignacion,
        {
          servidorId: 's1',
          mensaje: {
            ...base,
            instancia: { host: 'h', puerto: 7777, protocolo: 'udp' },
            tokensParticipante: [{ heroeId: fundador, token: 'SECRETO-DEL-JUGADOR', expiraEn: '2026-09-14T18:30:00Z' }],
          },
        },
        comoS1
      )
    );
    recoger(sesion.ejecutar(confirmarInicio, { servidorId: 's1', mensaje: base }, comoS1));

    const deBatalla = eventos.filter((e) => canalesDeEvento(e).includes(canalDeBatalla(battleId)));
    expect(deBatalla.map((e) => e.codigo)).toContain('evento_pve.abierta');
    expect(deBatalla.map((e) => e.codigo)).toContain('evento_pve.asignada');
    expect(deBatalla.map((e) => e.codigo)).toContain('evento_pve.en_curso');
    const cable = JSON.stringify(deBatalla);
    for (const secreto of ['SECRETO-DEL-JUGADOR', 'token', 'escuadras', 'participantes', 'squadId', 'heroeId']) {
      expect(cable, `no debe viajar «${secreto}»`).not.toContain(secreto);
    }
    expect(deBatalla[0]!.payload).toMatchObject({ battleId, bandos: { atacante: { heroes: 1, capacidadMaxima: 5 }, defensor: { heroes: 0 } } });
  });
});
