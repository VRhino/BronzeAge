// El hub difunde solo donde hay quien escuche: un canal sin suscriptores no cuesta ni un JSON.stringify, y los que escuchan reciben el mismo string.
import { afterEach, describe, expect, it, vi } from 'vitest';
import type WebSocket from 'ws';
import type { EventoDominio } from '../../../domain/eventos';
import { HubDeDifusion, SIN_JUGADOR } from '../hub';

class SocketFalso {
  readonly OPEN = 1;
  readyState = 1;
  enviados: string[] = [];
  send(m: string): void {
    this.enviados.push(m);
  }
}
const sock = () => new SocketFalso() as unknown as WebSocket & SocketFalso;
const evento = (asentamientoId?: string): EventoDominio => ({ codigo: 'x.y', mensaje: 'algo', momento: '2026-01-01T00:00:00.000Z', ...(asentamientoId ? { asentamientoId } : {}) });

afterEach(() => vi.restoreAllMocks());

describe('HubDeDifusion.difundir', () => {
  it('sin suscriptores en ningún canal no serializa nada', () => {
    const hub = new HubDeDifusion();
    hub.conectar('g1', 'j1', sock()); // conectado pero sin suscribirse
    const espia = vi.spyOn(JSON, 'stringify');
    hub.difundir('g1', [evento('a1'), evento(), evento('a2')]);
    expect(espia).not.toHaveBeenCalled();
  });

  it('serializa una vez por evento y canal con oyentes, y todos reciben el mismo string', () => {
    const hub = new HubDeDifusion();
    const [a, b, otro] = [sock(), sock(), sock()];
    for (const [s, id] of [[a, 'j1'], [b, 'j2'], [otro, 'j3']] as const) hub.conectar('g1', id, s);
    hub.suscribir('g1', a, 'asentamiento/a1');
    hub.suscribir('g1', b, 'asentamiento/a1');
    hub.suscribir('g1', otro, 'asentamiento/a2');
    const espia = vi.spyOn(JSON, 'stringify');
    hub.difundir('g1', [evento('a1')]);
    expect(espia).toHaveBeenCalledTimes(1);
    expect(a.enviados).toHaveLength(1);
    expect(a.enviados[0]).toBe(b.enviados[0]);
    expect(otro.enviados).toHaveLength(0);
  });

  it('admin/eventos recibe todo, atribuido o no, y desuscribir o desconectar lo saca del índice', () => {
    const hub = new HubDeDifusion();
    const admin = sock();
    hub.conectar('g1', SIN_JUGADOR, admin);
    hub.suscribir('g1', admin, 'admin/eventos');
    hub.difundir('g1', [evento('a1'), evento(), evento('a2')]);
    expect(admin.enviados.map((m) => JSON.parse(m).canal)).toEqual(['admin/eventos', 'admin/eventos', 'admin/eventos']);

    hub.desuscribir('g1', admin, 'admin/eventos');
    hub.difundir('g1', [evento('a1')]);
    expect(admin.enviados).toHaveLength(3);

    hub.suscribir('g1', admin, 'admin/eventos');
    hub.desconectar('g1', admin);
    hub.difundir('g1', [evento()]);
    expect(admin.enviados).toHaveLength(3);
  });

  it('una conexión de administración cuyo derecho ya no vale recibe revocado, una sola vez, y no más eventos', () => {
    const hub = new HubDeDifusion();
    const admin = sock();
    let vigente = true;
    hub.conectar('g1', SIN_JUGADOR, admin, () => vigente);
    hub.suscribir('g1', admin, 'admin/eventos');
    hub.difundir('g1', [evento()]);
    vigente = false;
    hub.difundir('g1', [evento(), evento()]);
    hub.difundir('g1', [evento()]);
    expect(admin.enviados.map((m) => JSON.parse(m).tipo)).toEqual(['evento', 'revocado']);
  });
});
