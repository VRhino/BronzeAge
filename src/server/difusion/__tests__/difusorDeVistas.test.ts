// El difusor de vistas (03 §6): foto al suscribirse, deltas encadenados, un diff y una serialización por grupo, backpressure,
// reevaluación del derecho y resincronización. Sin red: sockets de mentira y una `GameSession` real.
import { describe, expect, it } from 'vitest';
import type WebSocket from 'ws';
import { aplicarDiferencia } from '../../../session/vistas/diff';
import { analizarCanal, definicionDe, proyectarCanal } from '../../../session/vistas/segmentos';
import { partidaConAsentamiento } from '../../../session/__tests__/fixtures';
import { DifusorDeVistas, type ConexionDeVistas } from '../difusorDeVistas';

class SocketFalso {
  readonly OPEN = 1;
  readyState = 1;
  bufferedAmount = 0;
  enviados: string[] = [];
  cierre?: { codigo: number; motivo: string };
  send(m: string): void {
    this.enviados.push(m);
  }
  close(codigo: number, motivo: string): void {
    this.cierre = { codigo, motivo };
    this.readyState = 3;
  }
  mensajes(): Record<string, any>[] {
    return this.enviados.map((m) => JSON.parse(m));
  }
  ultimo(): Record<string, any> {
    return JSON.parse(this.enviados[this.enviados.length - 1]!);
  }
}
const sock = () => new SocketFalso() as unknown as WebSocket & SocketFalso;

function montar(opciones: Partial<ConstructorParameters<typeof DifusorDeVistas>[0]> = {}) {
  const { sesion, asentamientoId } = partidaConAsentamiento('g1');
  const fuente = { getState: () => sesion.getState() as never, intervaloRelojDeMundoMs: () => 4000 };
  const ms = { t: 0 };
  const difusor = new DifusorDeVistas({ fuenteDe: (id) => (id === 'g1' ? fuente : undefined), coalescerMs: 0, ahoraMs: () => ms.t, ...opciones });
  const admin: { vivo: boolean } = { vivo: true };
  const conexion: ConexionDeVistas = { audiencia: { tipo: 'admin' }, derecho: () => admin.vivo };
  return {
    sesion,
    asentamientoId,
    difusor,
    admin,
    conexion,
    ms,
    tick: () => {
      sesion.avanzarTick();
      difusor.notificar('g1');
    },
  };
}

describe('suscribir', () => {
  it('responde suscrito con el intervalo y manda la foto con la versión, sin instante ni version dentro de datos', () => {
    const { difusor, conexion, sesion } = montar();
    const s = sock();
    expect(difusor.suscribir('g1', s, conexion, 'admin/partida')).toBeUndefined();
    const [suscrito, foto] = s.mensajes();
    expect(suscrito).toEqual({ tipo: 'suscrito', canal: 'admin/partida', intervaloTickMs: 4000 });
    expect(foto).toMatchObject({ tipo: 'foto', gameId: 'g1', canal: 'admin/partida', version: sesion.getState().version });
    expect(foto!.datos).not.toHaveProperty('instante');
    expect(foto!.datos).not.toHaveProperty('version');
    expect(foto!.datos.asentamientos[0]).toHaveProperty('resumen');
    expect(foto!.datos.asentamientos[0]).not.toHaveProperty('almacen');
  });

  it('un canal que no existe o es de jugador es canal_desconocido; uno de una plaza que no existe, no_autorizado', () => {
    const { difusor, conexion } = montar();
    const s = sock();
    expect(difusor.suscribir('g1', s, conexion, 'vista/mapa')).toEqual({ error: 'canal desconocido', codigo: 'canal_desconocido' });
    expect(difusor.suscribir('g1', s, conexion, 'admin/loquesea')).toMatchObject({ codigo: 'canal_desconocido' });
    expect(difusor.suscribir('g1', s, conexion, 'admin/asentamiento/no-existe')).toMatchObject({ codigo: 'no_autorizado' });
    expect(difusor.suscribir('otra', s, conexion, 'admin/partida')).toMatchObject({ codigo: 'no_autorizado' });
  });

  it('sin derecho no hay foto', () => {
    const { difusor, conexion, admin } = montar();
    admin.vivo = false;
    const s = sock();
    expect(difusor.suscribir('g1', s, conexion, 'admin/partida')).toMatchObject({ codigo: 'no_autorizado' });
    expect(s.enviados).toEqual([]);
  });

  it('el tope de suscripciones por socket protege de quien suscribe a todo', () => {
    const { difusor, conexion, asentamientoId } = montar({ maxSuscripcionesPorSocket: 2 });
    const s = sock();
    expect(difusor.suscribir('g1', s, conexion, 'admin/partida')).toBeUndefined();
    expect(difusor.suscribir('g1', s, conexion, `admin/asentamiento/${asentamientoId}`)).toBeUndefined();
    expect(difusor.suscribir('g1', s, conexion, 'admin/partida')).toBeUndefined(); // repetir no cuenta
    expect(difusor.suscribir('g1', s, conexion, 'admin/heroe/x')).toMatchObject({ codigo: 'formato' });
  });
});

describe('deltas', () => {
  it('foto + deltas encadenados reproducen la foto del estado, tick tras tick (paridad)', () => {
    const { difusor, conexion, sesion, tick } = montar();
    const s = sock();
    difusor.suscribir('g1', s, conexion, 'admin/partida');
    const foto = s.mensajes()[1]!;
    let cliente = foto.datos as Record<string, unknown>;
    let version = foto.version as number;
    const esquema = definicionDe(analizarCanal('admin/partida', { tipo: 'admin' })!).esquema;

    for (let i = 0; i < 5; i++) {
      tick();
      const delta = s.ultimo();
      expect(delta).toMatchObject({ tipo: 'delta', canal: 'admin/partida', desde: version });
      expect(delta.hasta).toBe(sesion.getState().version);
      cliente = aplicarDiferencia(cliente, { cambios: delta.cambios, escalares: delta.escalares }, esquema);
      version = delta.hasta;
    }
    const { instante: _i, version: _v, tick: _t, ...esperado } = proyectarCanal(sesion.getState() as never, { nombre: 'admin/partida', tipo: 'admin/partida' })!;
    const ordena = (x: unknown) =>
      JSON.parse(JSON.stringify(x, (_k, v) => (Array.isArray(v) ? [...v].sort((a, b) => String(a?.id ?? a?.tituloId).localeCompare(String(b?.id ?? b?.tituloId))) : v)));
    expect(ordena(cliente)).toEqual(ordena(esperado));
  });

  it('cada tick manda al menos el latido (el instante va en la cabecera)', () => {
    const { difusor, conexion, tick } = montar();
    const s = sock();
    difusor.suscribir('g1', s, conexion, 'admin/partida');
    tick();
    expect(s.ultimo()).toMatchObject({ tipo: 'delta', desde: expect.any(Number), hasta: expect.any(Number), instante: expect.any(Number) });
  });

  it('dos suscriptores del mismo canal reciben el MISMO string, y se proyecta una sola vez', () => {
    const { difusor, conexion, tick } = montar();
    const a = sock();
    const b = sock();
    difusor.suscribir('g1', a, conexion, 'admin/partida');
    difusor.suscribir('g1', b, conexion, 'admin/partida');
    const antes = difusor.metricas().porCanal['admin/partida']!.flushes;
    tick();
    expect(a.enviados.at(-1)).toBe(b.enviados.at(-1));
    expect(difusor.metricas().porCanal['admin/partida']!.flushes - antes).toBe(1);
    expect(difusor.metricas().grupos).toBe(1);
  });

  it('una suscripción nueva en mitad de la cadena recibe la foto ya volcada y sigue sin hueco', () => {
    const { difusor, conexion, tick, sesion } = montar();
    const a = sock();
    difusor.suscribir('g1', a, conexion, 'admin/partida');
    tick();
    const b = sock();
    difusor.suscribir('g1', b, conexion, 'admin/partida');
    expect(b.mensajes()[1]!.version).toBe(sesion.getState().version);
    tick();
    expect(b.ultimo().desde).toBe(b.mensajes()[1]!.version);
  });

  it('el coalescer junta varias notificaciones en un solo flush', () => {
    const pendientes: (() => void)[] = [];
    const { difusor, conexion, sesion } = montar({ coalescerMs: 100, programar: (fn) => pendientes.push(fn) });
    const s = sock();
    difusor.suscribir('g1', s, conexion, 'admin/partida');
    const v0 = sesion.getState().version;
    for (let i = 0; i < 3; i++) {
      sesion.avanzarTick();
      difusor.notificar('g1');
    }
    expect(pendientes).toHaveLength(1);
    pendientes[0]!();
    const deltas = s.mensajes().filter((m) => m.tipo === 'delta');
    expect(deltas).toHaveLength(1);
    expect(deltas[0]).toMatchObject({ desde: v0, hasta: sesion.getState().version });
    expect(difusor.metricas().porCanal['admin/partida']!.versionesPorDelta.max).toBe(3);
  });

  it('sin suscriptores no se proyecta nada', () => {
    const { difusor, tick } = montar();
    tick();
    expect(difusor.metricas().porCanal).toEqual({});
  });
});

describe('derecho', () => {
  it('perderlo manda revocado en el mismo flush y deja de recibir deltas', () => {
    const { difusor, conexion, admin, tick } = montar();
    const s = sock();
    difusor.suscribir('g1', s, conexion, 'admin/partida');
    admin.vivo = false;
    tick();
    expect(s.ultimo()).toMatchObject({ tipo: 'revocado', canal: 'admin/partida' });
    const n = s.enviados.length;
    admin.vivo = true;
    tick();
    expect(s.enviados).toHaveLength(n);
    expect(difusor.metricas().grupos).toBe(0);
  });

  it('una plaza que desaparece revoca a quienes la miraban', () => {
    const { difusor, conexion, sesion, asentamientoId } = montar();
    const s = sock();
    difusor.suscribir('g1', s, conexion, `admin/asentamiento/${asentamientoId}`);
    const estado = sesion.getState() as unknown as { asentamientos: unknown[]; version: number };
    estado.asentamientos = [];
    estado.version++;
    difusor.notificar('g1');
    expect(s.ultimo()).toMatchObject({ tipo: 'revocado', motivo: 'ya_no_existe' });
  });
});

describe('backpressure', () => {
  it('un socket lento no acumula: se le saltan los deltas y al recuperarse recibe resync + foto', () => {
    const { difusor, conexion, tick, sesion } = montar();
    const lento = sock();
    const bueno = sock();
    difusor.suscribir('g1', lento, conexion, 'admin/partida');
    difusor.suscribir('g1', bueno, conexion, 'admin/partida');
    lento.bufferedAmount = 1_000_000;
    const n = lento.enviados.length;
    tick();
    tick();
    expect(lento.enviados).toHaveLength(n);
    expect(bueno.mensajes().filter((m) => m.tipo === 'delta')).toHaveLength(2);

    lento.bufferedAmount = 0;
    tick();
    const tipos = lento.mensajes().slice(n).map((m) => m.tipo);
    expect(tipos).toEqual(['resync', 'foto']);
    expect(lento.ultimo().version).toBe(sesion.getState().version);
    tick();
    expect(lento.ultimo().tipo).toBe('delta');
  });

  it('desactualizado durante demasiado tiempo se cierra con 4408', () => {
    const { difusor, conexion, tick, ms } = montar({ lentoMaxMs: 15_000 });
    const s = sock();
    difusor.suscribir('g1', s, conexion, 'admin/partida');
    s.bufferedAmount = 1_000_000;
    tick();
    ms.t = 16_000;
    tick();
    expect(s.cierre).toEqual({ codigo: 4408, motivo: 'cliente lento' });
    expect(difusor.metricas().porCanal['admin/partida']!.cierres4408).toBe(1);
  });
});

describe('resincronizar', () => {
  it('manda la foto de nuevo, con rate limit de una cada 2 s por canal', () => {
    const { difusor, conexion, ms } = montar();
    const s = sock();
    difusor.suscribir('g1', s, conexion, 'admin/partida');
    ms.t = 500;
    expect(difusor.resincronizar('g1', s, 'admin/partida')).toMatchObject({ codigo: 'demasiado_frecuente' });
    ms.t = 2500;
    const n = s.enviados.length;
    expect(difusor.resincronizar('g1', s, 'admin/partida')).toBeUndefined();
    expect(s.mensajes()[n]!.tipo).toBe('foto');
    expect(difusor.resincronizar('g1', s, 'admin/asentamiento/x')).toMatchObject({ codigo: 'no_autorizado' });
  });
});

describe('partida reemplazada', () => {
  it('si la versión baja, resync con partida_reemplazada y foto nueva', () => {
    const { difusor, conexion, sesion } = montar();
    const s = sock();
    difusor.suscribir('g1', s, conexion, 'admin/partida');
    const estado = sesion.getState() as unknown as { version: number };
    estado.version = 0;
    difusor.notificar('g1');
    const ultimos = s.mensajes().slice(-2);
    expect(ultimos.map((m) => m.tipo)).toEqual(['resync', 'foto']);
    expect(ultimos[0]!.motivo).toBe('partida_reemplazada');
  });
});

describe('canales de detalle', () => {
  it('admin/asentamiento lleva el Asentamiento completo y sus edificios como entidades', () => {
    const { difusor, conexion, asentamientoId } = montar();
    const s = sock();
    difusor.suscribir('g1', s, conexion, `admin/asentamiento/${asentamientoId}`);
    const foto = s.mensajes()[1]!;
    expect(foto.datos.asentamiento).toMatchObject({ id: asentamientoId });
    expect(foto.datos.asentamiento).toHaveProperty('almacen');
    expect(foto.datos.asentamiento).not.toHaveProperty('edificios');
    expect(foto.datos.edificios.length).toBeGreaterThan(0);
  });

  it('admin/heroe lleva el historial con ids sintéticos únicos', () => {
    const { difusor, conexion, sesion } = montar();
    const heroe = sesion.getState().heroes[0]!;
    const s = sock();
    difusor.suscribir('g1', s, conexion, `admin/heroe/${heroe.id}`);
    expect(s.mensajes()[1]!.datos.heroe).toMatchObject({ id: heroe.id });
    const ids = s.mensajes()[1]!.datos.historial.map((l: { id: string }) => l.id);
    expect(new Set(ids).size).toBe(ids.length);
  });
});
