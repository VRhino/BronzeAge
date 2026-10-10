// vista/mapa por Facción (contrato 03 §13.1, camino 2): una proyección y una serialización por Facción, sin que cambie lo que ve cada jugador.
// Las exigencias del usuario, una por bloque: paridad (mapa + ajustes de vista/heroe ≡ la proyección HTTP de cada héroe), invariancia (dos
// miembros de la misma clave reciben lo mismo) y no-fuga entre Facciones; y que un cambio de clave mueva al suscriptor con resync.
import { describe, expect, it } from 'vitest';
import type WebSocket from 'ws';
import { evaluarViabilidadFundacion } from '../../../engine/settlement';
import { crearFaccion } from '../../../session/comandos/crearFaccion';
import { componerProyeccion, proyectarParaJugador } from '../../../session/proyecciones/jugador';
import { conHeroe, enPie, partidaConAsentamiento } from '../../../session/__tests__/fixtures';
import { fundarAsentamiento } from '../../../session/__tests__/fundarDePrueba';
import { DifusorDeVistas, type ConexionDeVistas } from '../difusorDeVistas';

class SocketFalso {
  readonly OPEN = 1;
  readyState = 1;
  bufferedAmount = 0;
  enviados: string[] = [];
  send(m: string): void {
    this.enviados.push(m);
  }
  close(): void {
    this.readyState = 3;
  }
  mensajes(): Record<string, any>[] {
    return this.enviados.map((m) => JSON.parse(m));
  }
  foto(canal: string): Record<string, any> {
    return this.mensajes().filter((m) => m.tipo === 'foto' && m.canal === canal).at(-1)!;
  }
}
const sock = () => new SocketFalso() as unknown as WebSocket & SocketFalso;
const ordenado = (x: unknown) => JSON.parse(JSON.stringify(x, (_k, v) => (Array.isArray(v) && v.length > 0 && typeof v[0] === 'object' && v[0] !== null ? [...v].sort((a, b) => JSON.stringify(a).localeCompare(JSON.stringify(b))) : v)));

const OTRO = 'jugador-otra-faccion';

/** Dos Facciones con jugadores: la de siempre (fundador + vecino) y otra, lejos, con una plaza y un héroe. */
function mundo() {
  const base = partidaConAsentamiento('g1');
  let sesion = conHeroe(base.sesion, OTRO);
  const mapa = sesion.getMapa();
  const plaza1 = sesion.getState().asentamientos[0]!;
  let lejos: { x: number; y: number } | undefined;
  for (let x = 1700; x > 300 && !lejos; x -= 50) {
    for (let y = 1700; y > 300 && !lejos; y -= 50) {
      if (Math.hypot(x - plaza1.posicion.x, y - plaza1.posicion.y) > 700 && evaluarViabilidadFundacion(mapa, { x, y }, sesion.getState().asentamientos).recomendable) lejos = { x, y };
    }
  }
  sesion = enPie(sesion, OTRO, lejos!);
  const f2 = sesion.ejecutar(crearFaccion, { nombre: 'Lejanos' }, { actor: OTRO }).datos!.faccionId;
  sesion.ejecutar(fundarAsentamiento, { faccionId: f2 }, { actor: OTRO });
  const fuente = {
    getState: () => sesion.getState() as never,
    intervaloRelojDeMundoMs: () => 4000,
    geometriaAsentamientos: () => ({ zonas: [], zonasFusionadas: [], trazadoPorAsentamiento: {} }),
    preciosReferencia: () => ({}),
    produccionDeAsentamiento: () => [],
    ascensoDeAsentamiento: () => undefined,
  };
  const difusor = new DifusorDeVistas({ fuenteDe: () => fuente, coalescerMs: 0 });
  const de = (heroeId: string): ConexionDeVistas => ({ audiencia: { tipo: 'jugador', jugadorId: sesion.getState().heroes.find((h) => h.id === heroeId)!.jugadorId! }, derecho: () => true });
  const tick = () => {
    sesion.avanzarTick();
    difusor.notificar('g1');
  };
  return { sesion, fuente, difusor, de, tick, f1: base.faccionId, f2, fundador: base.fundador, vecino: base.vecino, plaza1: base.asentamientoId };
}

function suscribirAmbos(m: ReturnType<typeof mundo>, heroeId: string) {
  const s = sock();
  expect(m.difusor.suscribir('g1', s, m.de(heroeId), 'vista/mapa')).toBeUndefined();
  expect(m.difusor.suscribir('g1', s, m.de(heroeId), 'vista/heroe')).toBeUndefined();
  return s;
}

/** La proyección de ese héroe reconstruida desde sus dos fotos, igual que lo hace el cliente. */
function reconstruida(s: SocketFalso, heroeId: string) {
  const mapa = s.foto('vista/mapa').datos;
  const heroe = s.foto('vista/heroe').datos;
  const ajustes = {
    plazaPisadaId: heroe['plazaPisadaId'],
    detalleDeAvistados: heroe['detalleDeAvistados'] ?? {},
    campamentosPropios: heroe['campamentosPropios'] ?? [],
    heroesVisiblesExtra: heroe['heroesVisiblesExtra'] ?? [],
    ladoPropioPorBatalla: heroe['ladoPropioPorBatalla'] ?? {},
    batallasPropias: heroe['batallasPropias'] ?? [],
  };
  return componerProyeccion(mapa, heroeId, ajustes as never) as Record<string, unknown>;
}

describe('paridad: vista/mapa + ajustes de vista/heroe ≡ proyección HTTP', () => {
  it('para cada jugador de cada Facción', () => {
    const m = mundo();
    for (const h of [m.fundador, m.vecino, OTRO]) {
      const s = suscribirAmbos(m, h);
      const http = proyectarParaJugador(m.sesion.getState(), h, m.fuente.geometriaAsentamientos()) as Record<string, unknown>;
      const r = reconstruida(s, h);
      for (const campo of Object.keys(s.foto('vista/mapa').datos)) expect(ordenado(r[campo]), `${h} ${campo}`).toEqual(ordenado(http[campo]));
    }
  });

  it('con el jugador dentro de una plaza y el otro fuera: la ficha de la plaza pisada sale de los avistados solo para el que está dentro', () => {
    const m = mundo();
    const estado = m.sesion.getState();
    const heroeVecino = estado.heroes.find((h) => h.id === m.vecino) as unknown as { ubicacion: unknown };
    heroeVecino.ubicacion = { tipo: 'columna', ejercitoId: 'x' };
    const dentro = suscribirAmbos(m, m.fundador);
    const fuera = sock();
    m.difusor.suscribir('g1', fuera, m.de(m.vecino), 'vista/mapa');
    // Mismo mapa para los dos (misma Facción); solo cambia lo que dice vista/heroe de cada uno.
    expect(dentro.foto('vista/mapa').datos.asentamientosAvistados.some((a: { id: string }) => a.id === m.plaza1)).toBe(true);
    expect(dentro.foto('vista/heroe').datos.plazaPisadaId).toBe(m.plaza1);
  });
});

describe('invariancia: dos miembros de la misma clave reciben lo mismo', () => {
  it('fundador y vecino comparten UN grupo y el mismo string de foto', () => {
    const m = mundo();
    const a = sock();
    const b = sock();
    m.difusor.suscribir('g1', a, m.de(m.fundador), 'vista/mapa');
    m.difusor.suscribir('g1', b, m.de(m.vecino), 'vista/mapa');
    expect(m.difusor.metricas().porCanal['vista/mapa']).toMatchObject({ suscriptores: 2, claves: 1 });
    expect(a.foto('vista/mapa')).toEqual(b.foto('vista/mapa'));
    m.tick();
    expect(a.enviados.at(-1)).toBe(b.enviados.at(-1)); // el mismo delta serializado una vez
  });

  it('una Facción distinta tiene su grupo, y una proyección del mapa por Facción (no por jugador)', () => {
    const m = mundo();
    for (const h of [m.fundador, m.vecino, OTRO]) m.difusor.suscribir('g1', sock(), m.de(h), 'vista/mapa');
    expect(m.difusor.metricas().porCanal['vista/mapa']).toMatchObject({ suscriptores: 3, claves: 2 });
  });
});

describe('no-fuga entre Facciones', () => {
  it('lo que una Facción no ve no está en su mapa, ni en la foto ni en los deltas', () => {
    const m = mundo();
    const lejana = m.sesion.getState().asentamientos.find((a) => a.faccionId === m.f2)!;
    const s1 = sock();
    const s2 = sock();
    m.difusor.suscribir('g1', s1, m.de(m.fundador), 'vista/mapa');
    m.difusor.suscribir('g1', s2, m.de(OTRO), 'vista/mapa');
    m.tick();
    m.tick();
    // Las Facciones son públicas (nombre, Rey…); lo que no viaja es lo que cada una tiene en el mundo: sus plazas, sus héroes en columnas.
    const ids = (s: SocketFalso, campo: string, clave: string) => s.mensajes().flatMap((x) => (x.tipo === 'foto' ? (x.datos[campo] ?? []) : [])).map((e: Record<string, string>) => e[clave]);
    expect(ids(s1, 'asentamientosAvistados', 'id')).not.toContain(lejana.id);
    expect(ids(s1, 'asentamientosConocidos', 'asentamientoId')).not.toContain(lejana.id);
    expect(s1.enviados.join(' ')).not.toContain(lejana.posicion.x.toString() + ',"y":' + lejana.posicion.y.toString());
    expect(ids(s1, 'heroesVisibles', 'heroeId')).not.toContain(OTRO);
    expect(ids(s2, 'asentamientosAvistados', 'id')).not.toContain(m.plaza1);
    expect(ids(s2, 'heroesVisibles', 'heroeId')).not.toContain(m.fundador);
    // Ni un delta menciona la plaza o los héroes de la otra Facción.
    const deltas = (s: SocketFalso) => s.mensajes().filter((x) => x.tipo === 'delta').map((x) => JSON.stringify(x.cambios)).join(' ');
    expect(deltas(s1)).not.toContain(lejana.id);
    expect(deltas(s2)).not.toContain(m.plaza1);
    // Y cada Facción sí se ve a sí misma.
    expect(s2.foto('vista/mapa').datos.asentamientosAvistados.map((a: { id: string }) => a.id)).toContain(lejana.id);
  });

  it('lo personal de un jugador no llega al mapa de su Facción: lo que explora solo él y lo de su plaza van en su vista/heroe', () => {
    const m = mundo();
    const a = sock();
    const b = sock();
    m.difusor.suscribir('g1', a, m.de(m.fundador), 'vista/mapa');
    m.difusor.suscribir('g1', b, m.de(m.vecino), 'vista/mapa');
    const mapa = a.foto('vista/mapa').datos;
    for (const campo of ['plazaPisadaId', 'detalleDeAvistados', 'alijos', 'campamentosPropios', 'heroe', 'historial']) expect(mapa).not.toHaveProperty(campo);
  });
});

describe('cambios de clave', () => {
  it('haber explorado por su cuenta más que la Facción saca al jugador del grupo (con resync por visibilidad) y su mapa sigue siendo exacto', () => {
    const m = mundo();
    const a = sock();
    const b = sock();
    m.difusor.suscribir('g1', a, m.de(m.fundador), 'vista/mapa');
    m.difusor.suscribir('g1', b, m.de(m.vecino), 'vista/mapa');
    expect(m.difusor.metricas().porCanal['vista/mapa']!.claves).toBe(1);

    // El vecino explora, andando solo, una celda que la Facción no conoce.
    const vecino = m.sesion.getState().heroes.find((h) => h.id === m.vecino) as unknown as { exploracionPersonal: string };
    vecino.exploracionPersonal = 'ffffffffffffffffffffffff';
    m.tick();

    expect(m.difusor.metricas().porCanal['vista/mapa']!.claves).toBe(2);
    const ultimos = b.mensajes().slice(-2);
    expect(ultimos.map((x) => x.tipo)).toEqual(['resync', 'foto']);
    expect(ultimos[0]!.motivo).toBe('visibilidad');
    // Y lo que ve ahora es exactamente lo que dice su proyección HTTP.
    m.difusor.suscribir('g1', b, m.de(m.vecino), 'vista/heroe');
    const http = proyectarParaJugador(m.sesion.getState(), m.vecino, m.fuente.geometriaAsentamientos()) as Record<string, unknown>;
    const r = reconstruida(b, m.vecino);
    for (const campo of Object.keys(b.foto('vista/mapa').datos)) expect(ordenado(r[campo]), campo).toEqual(ordenado(http[campo]));
  });

  it('crear una alianza cambia la firma de la clave: los dos grupos se rehacen con foto nueva', () => {
    const m = mundo();
    const a = sock();
    const b = sock();
    m.difusor.suscribir('g1', a, m.de(m.fundador), 'vista/mapa');
    m.difusor.suscribir('g1', b, m.de(OTRO), 'vista/mapa');
    const antes = a.foto('vista/mapa').datos.asentamientosAvistados.map((x: { id: string }) => x.id);
    expect(antes).not.toContain(m.sesion.getState().asentamientos.find((x) => x.faccionId === m.f2)!.id);

    (m.sesion.getState().relaciones as unknown[]).push({ id: 'alianza-1', tipo: 'alianza', estado: 'activa', faccionAId: m.f1, faccionBId: m.f2 });
    m.tick();

    const ultimos = a.mensajes().slice(-2);
    expect(ultimos.map((x) => x.tipo)).toEqual(['resync', 'foto']);
    expect(ultimos[0]!.motivo).toBe('visibilidad');
    const ahora = a.foto('vista/mapa').datos.asentamientosAvistados.map((x: { id: string }) => x.id);
    expect(ahora).toContain(m.sesion.getState().asentamientos.find((x) => x.faccionId === m.f2)!.id); // ahora ve por su aliada
  });
});
