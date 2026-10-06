// El hub entrega un evento de batalla por los dos canales que le tocan (doc 02 §3.5): a quien sigue la batalla y a los
// hogares implicados, cada uno por el suyo.
import { describe, expect, it } from 'vitest';
import type WebSocket from 'ws';
import type { EventoDominio } from '../../domain/eventos';
import { HubDeDifusion } from '../difusion/hub';

function socketFalso() {
  const recibidos: { canal: string; evento: EventoDominio }[] = [];
  const socket = { OPEN: 1, readyState: 1, send: (m: string) => recibidos.push(JSON.parse(m)) } as unknown as WebSocket;
  return { socket, recibidos };
}

const evento = (codigo: string, payload: unknown, asentamientoId?: string): EventoDominio => ({
  codigo,
  mensaje: '',
  momento: '2026-01-01T00:00:00.000Z',
  payload,
  ...(asentamientoId ? { asentamientoId } : {}),
});

describe('HubDeDifusion con eventos de batalla', () => {
  it('cada suscriptor recibe el evento una vez, por el canal al que está suscrito', () => {
    const hub = new HubDeDifusion();
    const hogar = socketFalso();
    const siguiendo = socketFalso();
    const ambos = socketFalso();
    const ajeno = socketFalso();
    for (const [i, s] of [hogar, siguiendo, ambos, ajeno].entries()) hub.conectar('p1', `j${i}`, s.socket);
    hub.suscribir('p1', hogar.socket, 'asentamiento/a1');
    hub.suscribir('p1', siguiendo.socket, 'batalla/b-1');
    hub.suscribir('p1', ambos.socket, 'asentamiento/a1');
    hub.suscribir('p1', ambos.socket, 'batalla/b-1');
    hub.suscribir('p1', ajeno.socket, 'batalla/otra');

    hub.difundir('p1', [evento('batalla.abierta', { battleId: 'b-1' }, 'a1')]);

    expect(hogar.recibidos.map((m) => m.canal)).toEqual(['asentamiento/a1']);
    expect(siguiendo.recibidos.map((m) => m.canal)).toEqual(['batalla/b-1']);
    expect(ambos.recibidos.map((m) => m.canal).sort(), 'por cada canal suscrito, con su nombre').toEqual(['asentamiento/a1', 'batalla/b-1']);
    expect(ajeno.recibidos).toEqual([]);
  });

  it('un evento que no es de batalla sigue yendo solo por su canal', () => {
    const hub = new HubDeDifusion();
    const s = socketFalso();
    hub.conectar('p1', 'j', s.socket);
    hub.suscribir('p1', s.socket, 'batalla/b-1');

    hub.difundir('p1', [evento('construccion.completada', { battleId: 'b-1' }, 'a1')]);

    expect(s.recibidos).toEqual([]);
  });
});
