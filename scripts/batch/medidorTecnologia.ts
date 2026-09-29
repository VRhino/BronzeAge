// Tecnología por Eras en el batch (Doc 6; `Consideraciones/Tecnologia_Eras_I-III_Definicion.md`, Paso 12): con esto se
// fija la X de cada logro en lo que marca su contador en su semana objetivo, y se ve a qué ritmo adopta cada Facción.
import type { ContadorLogro, EstadoTecnologia, TecnologiaId } from '../../src/domain/types';
import { ERAS, TECNOLOGIAS } from '../../src/constants';

const MINUTOS_SEMANA = 7 * 24 * 60;

export class MedidorTecnologia {
  /** Contadores al cerrar cada semana de mundo (desde el tick 0 de la partida, no de la corrida). */
  private readonly porSemana: { semana: number; contadores: EstadoTecnologia['contadores'] }[] = [];
  /** Primer tick en que cada Facción tiene cada tecnología adoptada. */
  private readonly adopciones = new Map<string, Map<TecnologiaId, number>>();
  private readonly eras: { era: string; tick: number }[] = [];

  constructor(inicial: EstadoTecnologia, tickInicial: number) {
    this.eras.push({ era: inicial.era, tick: tickInicial });
  }

  registrarTick(tick: number, t: EstadoTecnologia): void {
    if (tick % MINUTOS_SEMANA === 0) this.porSemana.push({ semana: tick / MINUTOS_SEMANA, contadores: { ...t.contadores } });
    if (this.eras[this.eras.length - 1]!.era !== t.era) this.eras.push({ era: t.era, tick });
    for (const [faccionId, { adoptadas }] of Object.entries(t.porFaccion)) {
      const suyas = this.adopciones.get(faccionId) ?? new Map<TecnologiaId, number>();
      for (const id of adoptadas) if (!suyas.has(id) && !TECNOLOGIAS[id].deArranque) suyas.set(id, tick);
      this.adopciones.set(faccionId, suyas);
    }
  }

  /** `tickDe`: el inverso de `instanteDeTick`, para fechar los logros. */
  informe(final: EstadoTecnologia, tickDe: (instante: number) => number): string[] {
    const dia = (tick: number) => (tick / (24 * 60)).toFixed(1);
    const lineas = ['', '[TECNOLOGÍA] Eras:'];
    for (const e of this.eras) lineas.push(`  ${ERAS[e.era as keyof typeof ERAS]?.nombre ?? e.era} desde el día ${dia(e.tick)}`);

    lineas.push('[TECNOLOGÍA] Logros (umbral actual → cuándo se cumplió):');
    for (const id of Object.keys(TECNOLOGIAS) as TecnologiaId[]) {
      const logro = TECNOLOGIAS[id].logro;
      if (!logro) continue;
      const en = final.logros[id];
      const valor = final.contadores[logro.contador] ?? 0;
      lineas.push(`  ${id.padEnd(28)} ${logro.contador.padEnd(36)} ${String(logro.umbral).padStart(7)} → ${en === undefined ? `sin cumplir (va por ${Math.round(valor)})` : `día ${dia(tickDe(en))}`}`);
    }

    lineas.push('[TECNOLOGÍA] Contadores al cerrar cada semana (para fijar la X de cada logro en su semana objetivo):');
    const claves = [...new Set(this.porSemana.flatMap((s) => Object.keys(s.contadores)))].sort() as ContadorLogro[];
    for (const clave of claves) {
      lineas.push(`  ${clave.padEnd(36)} ${this.porSemana.map((s) => `s${s.semana}:${Math.round(s.contadores[clave] ?? 0)}`).join('  ')}`);
    }

    lineas.push('[TECNOLOGÍA] Adopciones por Facción (día):');
    for (const [faccionId, suyas] of this.adopciones) {
      const lista = [...suyas.entries()].sort((a, b) => a[1] - b[1]).map(([id, tick]) => `${id}@${dia(tick)}`);
      lineas.push(`  ${faccionId}: ${lista.length === 0 ? '—' : lista.join(', ')}`);
    }
    return lineas;
  }
}
