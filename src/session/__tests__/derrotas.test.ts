// Qué pasa con una Facción NPC que pierde su último asentamiento, resuelto en el momento de la derrota
// (`session/derrotas.ts`, decisiones del usuario 2026-10-02). Sustituye a la acogida que hacía el turno NPC barriendo
// (`acogerHeroesNpc`), que al convivir con la residencia en campamentos de mercenarios dejaba héroes borrados como
// residentes y bots residiendo en dos sitios (`Consideraciones/Auditoria_Tick_Eventos.md`, B5).
import { describe, expect, it } from 'vitest';
import type { Faccion } from '../../domain/types';
import { registrarDerrota } from '../../engine/faccion';
import { esResidente } from '../../engine/pertenencia';
import { crearFacciones, crearMapaDeterminista, escuadronDePrueba, fundarAsentamientoDeTest, heroeDePrueba, crearEstadoDeTest } from '../../engine/__tests__/fixtures';
import { GameSession } from '../gameSession';
import { crearFaccionNpc } from '../comandos/crearFaccionNpc';
import { iniciarAsedio } from '../comandos/militar';
import { derrotasEntre, resolverDerrotasNpc } from '../derrotas';
import type { GameSessionState } from '../estado';

const BOT = 'bot-derrotado';
const todasNpc = () => true;

/** La Facción 1 conserva su plaza; la 2 acaba de perder la última y le queda un bot, residente en un campamento. */
function mundo(derrotadaPor: string | null) {
  const mapa = crearMapaDeterminista(7);
  const uno = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
  const facciones: Faccion[] = uno.facciones.map((f) => (f.id === 'faccion-2' ? { ...f, ciudadanosIds: [BOT], derrotadaPor } : f));
  const bot = heroeDePrueba(BOT, { tipo: 'desconectado', punto: uno.asentamiento.posicion }, { controlador: 'bot' });
  const campamento = { id: 'mercenarios-0', posicion: { x: 0, y: 0 }, origen: 0, edificios: [], residentesIds: [BOT], poblacion: 0, poblacionEn: 0, mercado: {} } as never;
  const estado = crearEstadoDeTest([uno.asentamiento], facciones, { heroes: [bot], campamentosMercenarios: [campamento] });
  return { estado, plaza: uno.asentamiento };
}

const residenciasDe = (e: { asentamientos: GameSessionState['asentamientos']; campamentosMercenarios: GameSessionState['campamentosMercenarios'] }, id: string) => [
  ...e.asentamientos.filter((a) => esResidente(a, id)).map((a) => a.id),
  ...e.campamentosMercenarios.filter((c) => c.residentesIds.includes(id)).map((c) => c.id),
];

describe('derrota de una Facción NPC, resuelta en el momento', () => {
  it('registrarDerrota marca quién la derrotó solo si se queda sin plazas', () => {
    expect(registrarDerrota(crearFacciones(), [], 'faccion-2', 'faccion-1').find((f) => f.id === 'faccion-2')!.derrotadaPor).toBe('faccion-1');
  });

  it('derrotasEntre ve solo las derrotas nuevas', () => {
    const { estado } = mundo('faccion-1');
    const antes = { ...estado, facciones: estado.facciones.map((f) => (f.id === 'faccion-2' ? { ...f, derrotadaPor: undefined } : f)) };
    expect(derrotasEntre(antes, estado)).toEqual([{ faccionId: 'faccion-2', ganadoraId: 'faccion-1', plazaId: undefined }]);
    expect(derrotasEntre(estado, estado)).toEqual([]);
  });

  it('derrotada por otra NPC: se anexiona, y su bot pasa a residir en una plaza de la ganadora y sale del campamento', () => {
    const { estado, plaza } = mundo('faccion-1');
    const r = resolverDerrotasNpc(estado, [{ faccionId: 'faccion-2', ganadoraId: 'faccion-1' }], todasNpc);
    expect(r.estado.facciones.some((f) => f.id === 'faccion-2')).toBe(false);
    expect(r.estado.facciones.find((f) => f.id === 'faccion-1')!.ciudadanosIds).toContain(BOT);
    expect(residenciasDe(r.estado, BOT)).toEqual([plaza.id]);
    expect(r.eventos.map((e) => (typeof e === 'string' ? e : e.codigo))).toEqual(['faccion.anexionada']);
  });

  it('caída por colapso: se disuelve, su bot desaparece y ningún campamento lo sigue contando', () => {
    const { estado } = mundo(null);
    const r = resolverDerrotasNpc(estado, [{ faccionId: 'faccion-2', ganadoraId: null }], todasNpc);
    expect(r.estado.facciones.some((f) => f.id === 'faccion-2')).toBe(false);
    expect(r.estado.heroes.some((h) => h.id === BOT)).toBe(false);
    expect(r.estado.campamentosMercenarios[0]!.residentesIds).not.toContain(BOT);
  });

  it('derrotada por un jugador: se disuelve igual que por colapso', () => {
    const { estado } = mundo('faccion-1');
    const soloLaDos = (id: string) => id === 'faccion-2';
    const r = resolverDerrotasNpc(estado, [{ faccionId: 'faccion-2', ganadoraId: 'faccion-1' }], soloLaDos);
    expect(r.estado.facciones.some((f) => f.id === 'faccion-2')).toBe(false);
    expect(r.estado.heroes.some((h) => h.id === BOT)).toBe(false);
    expect(r.estado.facciones.find((f) => f.id === 'faccion-1')!.ciudadanosIds).not.toContain(BOT);
  });

  it('una Facción que no es NPC no se toca', () => {
    const { estado } = mundo('faccion-1');
    const r = resolverDerrotasNpc(estado, [{ faccionId: 'faccion-2', ganadoraId: 'faccion-1' }], () => false);
    expect(r.estado).toBe(estado);
  });
});

/** Dos Facciones NPC con su plaza, un tick corrido para que exista el campamento de mercenarios del día 1. */
function partidaNpc() {
  const s = GameSession.crear('derrotas', { seed: 7 });
  s.ejecutar(crearFaccionNpc, { nombre: 'Alfa' });
  s.ejecutar(crearFaccionNpc, { nombre: 'Beta' });
  s.avanzarTick();
  const e = s.getState();
  const [alfa, beta] = ['Alfa', 'Beta'].map((n) => e.facciones.find((f) => f.nombre === n)!);
  return { s, alfa: alfa!, beta: beta! };
}

function conEstado(s: GameSession, f: (e: GameSessionState) => GameSessionState): GameSession {
  const x = s.exportar();
  return GameSession.importar({ ...x, state: f(x.state) });
}

/** Ningún residente fantasma y nadie residiendo en dos sitios. */
function expectResidenciaCoherente(e: GameSessionState) {
  const existentes = new Set(e.heroes.map((h) => h.id));
  for (const c of e.campamentosMercenarios) for (const id of c.residentesIds) expect(existentes.has(id), `${id} en ${c.id}`).toBe(true);
  for (const h of e.heroes) expect(residenciasDe(e, h.id).length, h.id).toBeLessThanOrEqual(1);
}

describe('derrotas en la partida real', () => {
  it('la ruina de la última plaza de una Facción NPC la disuelve en el mismo tick, sin dejar residentes fantasma', () => {
    const { s: s0, alfa } = partidaNpc();
    expect(s0.getState().campamentosMercenarios.length).toBeGreaterThan(0);
    const plaza = s0.getState().asentamientos.find((a) => a.faccionId === alfa.id)!;
    const s = conEstado(s0, (st) => ({
      ...st,
      asentamientos: st.asentamientos.map((a) =>
        a.id !== plaza.id
          ? a
          : {
              ...a,
              medidorMantenimiento: 0.0001,
              nivelActual: 1,
              fundadoEn: (a.fundadoEn - 1e9) as typeof a.fundadoEn,
              protegidaHasta: undefined,
              almacen: Object.fromEntries(Object.entries(a.almacen).map(([k, v]) => [k, { ...v, cantidad: 0 }])),
            }
      ),
    }));
    const r = s.avanzarTick(); // solo el tick: sin turno NPC
    expect(r.ok && r.eventos.some((e) => e.codigo === 'asentamiento.ruinas')).toBe(true);
    expect(r.ok && r.eventos.some((e) => e.codigo === 'faccion.disuelta')).toBe(true);
    expect(s.getState().facciones.some((f) => f.id === alfa.id)).toBe(false);
    expectResidenciaCoherente(s.getState());
  });

  it('iniciarAsedio registra la derrota (B4) y la Facción NPC vencida por otra NPC se anexiona en el acto', () => {
    const { s: s0, alfa, beta } = partidaNpc();
    const origen = s0.getState().asentamientos.find((a) => a.faccionId === alfa.id)!;
    const objetivo = s0.getState().asentamientos.find((a) => a.faccionId === beta.id)!;
    const general = origen.heroesFundadoresIds[0]!;
    const botsBeta = [...objetivo.heroesFundadoresIds];
    const s = conEstado(s0, (st) => ({
      ...st,
      asentamientos: st.asentamientos.map((a) =>
        a.id === origen.id ? { ...a, cargos: { ...a.cargos, generalId: general } } : a.id === objetivo.id ? { ...a, protegidaHasta: undefined, ocupacionHasta: undefined } : a
      ),
      // El comando no asedia una plaza sin ninguna escuadra que la defienda: se le deja una testimonial, en la guarnición.
      heroes: st.heroes.map((h) =>
        h.id === general
          ? { ...h, escuadrones: [escuadronDePrueba('esc-asalto', general, 'milicia_lanceros', 500)] }
          : h.id === botsBeta[0]
            ? { ...h, escuadrones: [escuadronDePrueba('esc-defensa', h.id, 'milicia_lanceros', 1, { enGuarnicion: true })] }
            : h
      ),
    }));
    const r = s.ejecutar(iniciarAsedio, { atacanteId: origen.id, defensorId: objetivo.id, escuadronIds: ['esc-asalto'] });
    expect(r.ok && r.datos?.conquistado).toBe(true);
    const e = s.getState();
    expect(e.facciones.some((f) => f.id === beta.id)).toBe(false);
    // Los bots de Beta son ahora de Alfa y residen en sus plazas, repartidos, contando la recién conquistada.
    const alfaFinal = e.facciones.find((f) => f.id === alfa.id)!;
    for (const id of botsBeta) {
      expect(alfaFinal.ciudadanosIds).toContain(id);
      expect(residenciasDe(e, id)).toHaveLength(1);
      expect(e.asentamientos.find((a) => a.id === residenciasDe(e, id)[0])!.faccionId).toBe(alfa.id);
    }
    expect(botsBeta.some((id) => esResidente(e.asentamientos.find((a) => a.id === objetivo.id)!, id))).toBe(true);
    expectResidenciaCoherente(e);
  });
});
