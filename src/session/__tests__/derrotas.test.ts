// Una Facción que pierde su último asentamiento, sea de humanos o de bots (D59, doc 12 §10.1): queda marcada como
// derrotada y no se anexiona ni se disuelve. Quien se queda sin casa pasa a residir en un campamento (9f9a098).
import { describe, expect, it } from 'vitest';
import { registrarDerrota } from '../../engine/faccion';
import { esResidente } from '../../engine/pertenencia';
import { crearFacciones, escuadronDePrueba } from '../../engine/__tests__/fixtures';
import { GameSession } from '../gameSession';
import { crearFaccionNpc } from '../comandos/crearFaccionNpc';
import { iniciarAsedio } from '../comandos/militar';
import type { GameSessionState } from '../estado';

const residenciasDe = (e: { asentamientos: GameSessionState['asentamientos']; campamentosMercenarios: GameSessionState['campamentosMercenarios'] }, id: string) => [
  ...e.asentamientos.filter((a) => esResidente(a, id)).map((a) => a.id),
  ...e.campamentosMercenarios.filter((c) => c.residentesIds.includes(id)).map((c) => c.id),
];

describe('registrarDerrota', () => {
  it('marca quién la derrotó solo si se queda sin plazas', () => {
    expect(registrarDerrota(crearFacciones(), [], 'faccion-2', 'faccion-1').find((f) => f.id === 'faccion-2')!.derrotadaPor).toBe('faccion-1');
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

describe('derrotas en la partida real (D59)', () => {
  it('la ruina de la última plaza de una Facción de bots no la disuelve: sus héroes siguen y residen en un campamento', () => {
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
    const r = s.avanzarTick();
    expect(r.ok && r.eventos.some((e) => e.codigo === 'asentamiento.ruinas')).toBe(true);
    const e = s.getState();
    expect(e.facciones.some((f) => f.id === alfa.id)).toBe(true);
    for (const id of plaza.heroesFundadoresIds) expect(e.heroes.some((h) => h.id === id), id).toBe(true);
    expectResidenciaCoherente(e);
  });

  it('iniciarAsedio registra la derrota (B4) y la Facción vencida no se anexiona: sus bots siguen siendo suyos', () => {
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
    expect(e.facciones.find((f) => f.id === beta.id)!.derrotadaPor).toBe(alfa.id);
    const alfaFinal = e.facciones.find((f) => f.id === alfa.id)!;
    for (const id of botsBeta) expect(alfaFinal.ciudadanosIds).not.toContain(id);
    expectResidenciaCoherente(e);
  });
});
