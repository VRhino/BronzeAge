// Bloque GUERRA del batch (petición del usuario, 2026-09-27): qué pasa en la guerra y por qué las Facciones suben tan
// deprisa. Solo mide: lee los eventos de dominio del motor y compara el estado antes y después de cada tick.
//
// Limitaciones conocidas, a propósito:
//  - Las bajas son la caída de soldados durante el paso del MOTOR (combates, deserción por moral); las de un ataque
//    del NPC a un campamento de bandidos caen en el paso del NPC y no se cuentan aquí.
//  - La experiencia de construcción, conquista y crecer en paz (ascenso, fundación, trueque cumplido) se atribuye
//    exacta (eventos × tarifa); el resto del delta de experiencia de la Facción es combate y bandidos.
import type { EventoDominio } from '../../src/domain/eventos';
import type { Heroe } from '../../src/domain/types';
import type { EstadoSimulacion } from '../../src/engine/simulation';
import type { PayloadAsedio, PayloadCombateResuelto } from '../../src/engine/combate';
import type { PayloadAsentamientoFundado } from '../../src/engine/expansion';
import type { PayloadNivelSubio } from '../../src/engine/mantenimiento';
import type { PayloadTruequeCumplido } from '../../src/engine/trade';
import type { StatsNpcGobernanza } from '../../src/session/npcGobernanza';
import { NIVEL_FACCION } from '../../src/constants';

const TICKS_POR_SEMANA = 10_080;

type TipoAsedio = 'conquista en combate' | 'conquista sin defensores' | 'resistido en combate' | 'rebote por protección' | 'sin atacantes';

interface Semana {
  campanas: number;
  repliegues: number;
  reclutamientos: number;
  asedios: Record<TipoAsedio, number>;
  bajas: number;
  bajasEnTicksDeCombate: number;
  xpConstruccion: number;
  xpCrecer: number;
  xpConquista: number;
  xpCombate: number;
}

const semanaVacia = (): Semana => ({
  campanas: 0,
  repliegues: 0,
  reclutamientos: 0,
  asedios: { 'conquista en combate': 0, 'conquista sin defensores': 0, 'resistido en combate': 0, 'rebote por protección': 0, 'sin atacantes': 0 },
  bajas: 0,
  bajasEnTicksDeCombate: 0,
  xpConstruccion: 0,
  xpCrecer: 0,
  xpConquista: 0,
  xpCombate: 0,
});

function soldadosPorTropa(heroes: readonly Heroe[]): Map<string, number> {
  const porTropa = new Map<string, number>();
  for (const h of heroes) for (const e of h.escuadrones) porTropa.set(e.tropaId, (porTropa.get(e.tropaId) ?? 0) + e.cantidad);
  return porTropa;
}

function clasificar(eventos: readonly EventoDominio[], i: number): TipoAsedio {
  const ev = eventos[i]!;
  const trasCombate = eventos[i - 1]?.codigo === 'combate.resuelto';
  if (ev.codigo === 'combate.asedio_conquista') return trasCombate ? 'conquista en combate' : 'conquista sin defensores';
  if (trasCombate) return 'resistido en combate';
  return ev.mensaje.includes('protegida') ? 'rebote por protección' : 'sin atacantes';
}

const mediana = (xs: number[]) => (xs.length === 0 ? 0 : [...xs].sort((a, b) => a - b)[Math.floor(xs.length / 2)]!);
const pct = (n: number, total: number) => (total === 0 ? '—' : `${Math.round((100 * n) / total)} %`);
const dias = (ticks: number) => (ticks / 1440).toFixed(1);

export class MedidorGuerra {
  private readonly semanas = new Map<number, Semana>();
  private readonly bajasPorTropa = new Map<string, number>();
  private readonly ratiosDePoder: number[] = [];
  private readonly conquistasPorPar = new Map<string, number>();
  private readonly hechasPorFaccion = new Map<string, number>();
  private readonly sufridasPorFaccion = new Map<string, number>();
  private readonly caidasPorPlaza = new Map<string, number>();
  /** Dueño actual de cada plaza, desde qué tick, y todos los que ha tenido. */
  private readonly dueno = new Map<string, { faccionId: string; desde: number; historial: Set<string> }>();
  private readonly tenencias: number[] = [];
  private recuperadas = 0;
  private readonly distanciasAlObjetivo: number[] = [];
  private readonly nivelesAlCaer = new Map<number, number>();
  private ascensosPerdidos = 0;
  private readonly conquistadaEn = new Map<string, number>();
  private colapsosTrasConquista = 0;
  private readonly tickDeNivel = new Map<string, Map<number, number>>();

  constructor(inicial: Pick<EstadoSimulacion, 'asentamientos' | 'facciones'>, tickInicial: number) {
    // Los niveles que ya traía al reanudar desde un checkpoint no se alcanzaron en esta corrida: salen como "antes".
    for (const f of inicial.facciones) {
      this.tickDeNivel.set(f.id, new Map(Array.from({ length: Math.max(0, f.nivel - 1) }, (_, i) => [i + 2, -1] as [number, number])));
    }
    for (const a of inicial.asentamientos) this.dueno.set(a.id, { faccionId: a.faccionId, desde: tickInicial, historial: new Set([a.faccionId]) });
  }

  private semana(tick: number): Semana {
    const n = Math.floor((tick - 1) / TICKS_POR_SEMANA) + 1;
    let s = this.semanas.get(n);
    if (!s) this.semanas.set(n, (s = semanaVacia()));
    return s;
  }

  registrarTick(
    tick: number,
    antes: EstadoSimulacion,
    trasMotor: EstadoSimulacion & { eventosDominio: EventoDominio[] },
    trasNpc: EstadoSimulacion,
    stats: StatsNpcGobernanza
  ): void {
    const s = this.semana(tick);
    s.campanas += stats.campanasLanzadas;
    s.repliegues += stats.repliegues;
    s.reclutamientos += stats.reclutamientosExitosos;

    const plazaAntes = new Map(antes.asentamientos.map((a) => [a.id, a]));
    const faccionDePlaza = new Map(trasMotor.asentamientos.map((a) => [a.id, a.faccionId]));
    const eventos = trasMotor.eventosDominio;
    const construccionPorFaccion = new Map<string, number>();
    const crecerPorFaccion = new Map<string, number>();
    const sumarCrecer = (asentamientoId: string | undefined, xp: number) => {
      const f = asentamientoId && faccionDePlaza.get(asentamientoId);
      if (f) crecerPorFaccion.set(f, (crecerPorFaccion.get(f) ?? 0) + xp);
    };
    const conquistasPorFaccion = new Map<string, number>();
    let huboCombate = false;

    for (let i = 0; i < eventos.length; i++) {
      const ev = eventos[i]!;
      if (ev.codigo === 'combate.resuelto') {
        huboCombate = true;
        const p = ev.payload as PayloadCombateResuelto;
        this.ratiosDePoder.push(p.poderAtacante / Math.max(p.poderDefensor, 1));
        continue;
      }
      if (ev.codigo === 'construccion.edificio_completado' && ev.asentamientoId) {
        const f = faccionDePlaza.get(ev.asentamientoId);
        if (f) construccionPorFaccion.set(f, (construccionPorFaccion.get(f) ?? 0) + 1);
        continue;
      }
      if (ev.codigo === 'asentamiento.nivel_subio') {
        const p = ev.payload as PayloadNivelSubio;
        sumarCrecer(p.asentamientoId, NIVEL_FACCION.xp.ascensoPorNivel * p.nivelNuevo);
        continue;
      }
      if (ev.codigo === 'expansion.asentamiento_fundado') {
        sumarCrecer((ev.payload as PayloadAsentamientoFundado).asentamientoId, NIVEL_FACCION.xp.fundacion);
        continue;
      }
      if (ev.codigo === 'comercio.trueque_cumplido') {
        const p = ev.payload as PayloadTruequeCumplido;
        sumarCrecer(p.asentamientoAId, NIVEL_FACCION.xp.truequeCumplido);
        sumarCrecer(p.asentamientoBId, NIVEL_FACCION.xp.truequeCumplido);
        continue;
      }
      if (ev.codigo !== 'combate.asedio_conquista' && ev.codigo !== 'combate.asedio_resistido') continue;

      s.asedios[clasificar(eventos, i)]++;
      if (ev.codigo !== 'combate.asedio_conquista') continue;
      const p = ev.payload as PayloadAsedio;
      conquistasPorFaccion.set(p.faccionAtacanteId, (conquistasPorFaccion.get(p.faccionAtacanteId) ?? 0) + 1);
      const par = `${p.faccionAtacanteId} → ${p.faccionDefensoraId}`;
      this.conquistasPorPar.set(par, (this.conquistasPorPar.get(par) ?? 0) + 1);
      this.hechasPorFaccion.set(p.faccionAtacanteId, (this.hechasPorFaccion.get(p.faccionAtacanteId) ?? 0) + 1);
      this.sufridasPorFaccion.set(p.faccionDefensoraId, (this.sufridasPorFaccion.get(p.faccionDefensoraId) ?? 0) + 1);
      this.caidasPorPlaza.set(p.defensorId, (this.caidasPorPlaza.get(p.defensorId) ?? 0) + 1);
      this.conquistadaEn.set(p.defensorId, tick);

      const plaza = plazaAntes.get(p.defensorId);
      if (plaza) {
        this.nivelesAlCaer.set(plaza.nivel, (this.nivelesAlCaer.get(plaza.nivel) ?? 0) + 1);
        if (plaza.ascenso) this.ascensosPerdidos++;
        const ejercito = antes.ejercitos.find((e) => e.id === p.atacanteId);
        const origen = ejercito && plazaAntes.get(ejercito.origenAsentamientoId);
        if (origen) this.distanciasAlObjetivo.push(Math.hypot(origen.posicion.x - plaza.posicion.x, origen.posicion.y - plaza.posicion.y));
      }
      const d = this.dueno.get(p.defensorId);
      if (d) {
        this.tenencias.push(tick - d.desde);
        if (d.historial.has(p.faccionAtacanteId)) this.recuperadas++;
        d.historial.add(p.faccionAtacanteId);
        d.faccionId = p.faccionAtacanteId;
        d.desde = tick;
      }
    }

    // Plazas nuevas (fundaciones) entran al registro de dueños; las que desaparecen, si cayeron hace menos de una
    // semana, cuentan como colapso tras conquista.
    const vivas = new Set(trasNpc.asentamientos.map((a) => a.id));
    for (const a of trasNpc.asentamientos) {
      if (!this.dueno.has(a.id)) this.dueno.set(a.id, { faccionId: a.faccionId, desde: tick, historial: new Set([a.faccionId]) });
    }
    for (const a of antes.asentamientos) {
      if (vivas.has(a.id)) continue;
      const cayo = this.conquistadaEn.get(a.id);
      if (cayo !== undefined && tick - cayo <= TICKS_POR_SEMANA) this.colapsosTrasConquista++;
    }

    // Bajas en el paso del motor.
    const soldadosAntes = soldadosPorTropa(antes.heroes);
    const soldadosDespues = soldadosPorTropa(trasMotor.heroes);
    for (const [tropa, n] of soldadosAntes) {
      const perdidos = n - (soldadosDespues.get(tropa) ?? 0);
      if (perdidos <= 0) continue;
      this.bajasPorTropa.set(tropa, (this.bajasPorTropa.get(tropa) ?? 0) + perdidos);
      s.bajas += perdidos;
      if (huboCombate) s.bajasEnTicksDeCombate += perdidos;
    }

    // Experiencia de Facción por origen, y el tick en que cada una alcanza cada nivel.
    const faccionesAntes = new Map(antes.facciones.map((f) => [f.id, f]));
    for (const f of trasNpc.facciones) {
      const delta = f.experiencia - (faccionesAntes.get(f.id)?.experiencia ?? f.experiencia);
      if (delta > 0) {
        const xpConstruccion = (construccionPorFaccion.get(f.id) ?? 0) * NIVEL_FACCION.xp.edificioCompletado;
        const xpConquista = (conquistasPorFaccion.get(f.id) ?? 0) * NIVEL_FACCION.xp.conquista;
        const xpCrecer = crecerPorFaccion.get(f.id) ?? 0;
        s.xpConstruccion += xpConstruccion;
        s.xpCrecer += xpCrecer;
        s.xpConquista += xpConquista;
        s.xpCombate += Math.max(0, delta - xpConstruccion - xpCrecer - xpConquista);
      }
      let niveles = this.tickDeNivel.get(f.id);
      if (!niveles) this.tickDeNivel.set(f.id, (niveles = new Map()));
      for (let n = 2; n <= f.nivel; n++) if (!niveles.has(n)) niveles.set(n, tick);
    }
  }

  informe(): string[] {
    const l: string[] = [];
    l.push('', '=== GUERRA ===');
    l.push('semana  campañas  repliegues  asedios  conq.combate  conq.sin-def  resist.combate  rebote-prot  reclutam.   bajas (en ticks de combate)   XP constr/crecer/conq/combate+bandidos');
    for (const [n, s] of [...this.semanas.entries()].sort((a, b) => a[0] - b[0])) {
      const asedios = Object.values(s.asedios).reduce((a, b) => a + b, 0);
      l.push(
        `  ${String(n).padStart(2)}    ${String(s.campanas).padStart(6)}  ${String(s.repliegues).padStart(10)}  ${String(asedios).padStart(7)}  ` +
          `${String(s.asedios['conquista en combate']).padStart(12)}  ${String(s.asedios['conquista sin defensores']).padStart(12)}  ` +
          `${String(s.asedios['resistido en combate']).padStart(14)}  ${String(s.asedios['rebote por protección']).padStart(11)}  ` +
          `${String(s.reclutamientos).padStart(9)}  ${String(Math.round(s.bajas)).padStart(8)} (${pct(s.bajasEnTicksDeCombate, s.bajas)})` +
          `   ${Math.round(s.xpConstruccion)}/${Math.round(s.xpCrecer)}/${Math.round(s.xpConquista)}/${Math.round(s.xpCombate)}`
      );
    }

    const totales = semanaVacia();
    for (const s of this.semanas.values()) for (const k of Object.keys(s.asedios) as TipoAsedio[]) totales.asedios[k] += s.asedios[k];
    const conquistas = totales.asedios['conquista en combate'] + totales.asedios['conquista sin defensores'];
    const combates = totales.asedios['conquista en combate'] + totales.asedios['resistido en combate'];
    l.push('');
    l.push(`Conquistas: ${conquistas} — sin un defensor en la plaza: ${pct(totales.asedios['conquista sin defensores'], conquistas)}`);
    l.push(`Asedios con combate: ${combates} — gana el atacante el ${pct(totales.asedios['conquista en combate'], combates)}`);
    const ratios = this.ratiosDePoder;
    l.push(
      `Poder atacante / defensor en combate (todos los combates, ${ratios.length}): mediana ${mediana(ratios).toFixed(2)}, ` +
        `atacante ≥ 2× en el ${pct(ratios.filter((r) => r >= 2).length, ratios.length)}, ≤ 0,5× en el ${pct(ratios.filter((r) => r <= 0.5).length, ratios.length)}`
    );

    const caidas = [...this.caidasPorPlaza.values()];
    const tramos: [string, (n: number) => boolean][] = [['1', (n) => n === 1], ['2-5', (n) => n >= 2 && n <= 5], ['6-20', (n) => n >= 6 && n <= 20], ['>20', (n) => n > 20]];
    l.push(
      `Plazas que cambiaron de dueño: ${caidas.length} de ${this.dueno.size} — veces que cayó cada una: ` +
        tramos.map(([nombre, f]) => `${nombre}: ${caidas.filter(f).length}`).join(', ') +
        ` (máximo ${Math.max(0, ...caidas)})`
    );
    l.push(
      `Cuánto aguanta un dueño antes de perderla: mediana ${dias(mediana(this.tenencias))} días; ` +
        `menos de 1 día el ${pct(this.tenencias.filter((t) => t < 1440).length, this.tenencias.length)}. ` +
        `Recuperada por alguien que ya la tuvo: ${pct(this.recuperadas, conquistas)}`
    );
    l.push(
      `Nivel de la plaza al caer: ${[...this.nivelesAlCaer.entries()].sort().map(([n, c]) => `nivel ${n}: ${c}`).join(', ')} · ` +
        `ascensos perdidos: ${this.ascensosPerdidos} · colapsadas en la semana siguiente a caer: ${this.colapsosTrasConquista}`
    );
    l.push(`Distancia del origen de la columna a la plaza conquistada: mediana ${mediana(this.distanciasAlObjetivo).toFixed(0)}, p90 ${[...this.distanciasAlObjetivo].sort((a, b) => a - b)[Math.floor(this.distanciasAlObjetivo.length * 0.9)]?.toFixed(0) ?? '—'}`);

    const pares = [...this.conquistasPorPar.entries()].sort((a, b) => b[1] - a[1]);
    l.push(`Pares atacante → defendida con más conquistas: ${pares.slice(0, 8).map(([p, n]) => `${p} (${n})`).join(', ')}`);
    const facciones = [...new Set([...this.hechasPorFaccion.keys(), ...this.sufridasPorFaccion.keys()])].sort();
    l.push(`Por Facción, conquistas hechas / sufridas: ${facciones.map((f) => `${f} ${this.hechasPorFaccion.get(f) ?? 0}/${this.sufridasPorFaccion.get(f) ?? 0}`).join(', ')}`);

    const bajas = [...this.bajasPorTropa.entries()].sort((a, b) => b[1] - a[1]);
    l.push(`Bajas por tropa: ${bajas.map(([t, n]) => `${t} ${Math.round(n)}`).join(', ')}`);

    l.push('Día en que cada Facción alcanza su nivel (2, 4, 6, 8, 10):');
    for (const [f, niveles] of [...this.tickDeNivel.entries()].sort()) {
      l.push(`  ${f.padEnd(11)} ${[2, 4, 6, 8, 10].map((n) => (!niveles.has(n) ? '    —' : niveles.get(n)! < 0 ? 'antes' : dias(niveles.get(n)!).padStart(5))).join(' ')}`);
    }
    return l;
  }
}
