// Fundar con la Caravana de Fundación (Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md §3, Doc 1.8): la misma para el bot sin plaza que la compra
// en un campamento (`sinPlaza`) y para el que la lanza desde su plaza (`fundarDesdeCasa`). El titular la lleva enganchada a su columna hasta un
// sitio que le parece viable y funda con `fundar`; el motor decide si lo es, y si no, prueba otro.
import type { Asentamiento, Ejercito, Point } from '../../domain/types';
import { MERCENARIOS, MOVIMIENTO } from '../../constants';
import { tieneRecursos } from '../../engine/almacen';
import { nivelActualDe } from '../../engine/asentamientoQuery';
import { costoCaravanaFundacion } from '../../engine/expansion';
import { esRecomendableParaFundar, fuentesOcupadas } from '../../engine/settlement';
import { distancia } from '../../world/geometria';
import type { ContextoBot, Plan } from '../runner';
import { escuadrasLibres, loQueLeCabe, plazaDentro, plazasConocidas, residentesDe, type HeroeVisto } from './comun';

/** Dónde busca sitio para fundar: anillos alrededor de su origen. */
const SITIO = { radioMin: 130, radioMax: 450, paso: 40, angulos: 12, lejosDePlazas: 150, lejosDeBandidos: 80 };
/** Residentes que tiene que tener una plaza para que uno salga a fundar: se queda al menos uno en casa (§10.1). */
const RESIDENTES_PARA_EXPANDIR = 3;
/** El trigo del carro con el que sale a fundar sin tropa (con tropa, el ejército lleva el suyo). */
const TRIGO_DE_VIAJE = 100;

/**
 * Desde su plaza de nivel 2 con lo que cuesta, el residente sin cargo lanza la Caravana de Fundación, sale con su tropa y la engancha: la
 * lleva a un sitio que ve viable y funda. Devuelve si salió.
 */
export async function fundarDesdeCasa(ctx: ContextoBot, plaza: Asentamiento, heroe: HeroeVisto): Promise<boolean> {
  const { vista, yo } = ctx;
  if (nivelActualDe(plaza) < 2 || !tieneRecursos(plaza.almacen, costoCaravanaFundacion())) return false;
  if (residentesDe(plaza).length < RESIDENTES_PARA_EXPANDIR) return false;
  if (vista.caravanas.some((c) => c.tipo === 'construccion' && c.faccionId === vista.faccionId)) return false;
  const sitio = sitioParaFundar(ctx, plaza.posicion, []);
  if (!sitio) return false;

  const lanzada = await ctx.intentar(`fundar:${plaza.id}`, 'lanzarCaravanaFundacion', { origenAsentamientoId: plaza.id }, 24 * 60 * 60_000);
  if (!lanzada?.ok || !lanzada.datos) return false;
  const caravanaId = lanzada.datos.caravanaId;

  const escuadras = loQueLeCabe(heroe, escuadrasLibres(heroe)).map((e) => e.id);
  const salida =
    escuadras.length > 0
      ? await ctx.actuar('movilizarEjercito', { asentamientoId: plaza.id, heroeId: yo, escuadronIds: escuadras, objetivo: { tipo: 'punto', punto: sitio } })
      : await ctx.actuar('salirAlMundo', { asentamientoId: plaza.id, heroeId: yo, escuadronIds: [], carga: { trigo: Math.min(TRIGO_DE_VIAJE, plaza.almacen['trigo']?.cantidad ?? 0) } });
  if (!salida.ok || !salida.datos) {
    // Sin columna con la que llevarla, la deshace en la puerta y recupera lo que costó.
    await ctx.actuar('desarmarCaravanaFundacion', { caravanaId });
    return false;
  }
  await ctx.actuar('adjuntarCaravana', { ejercitoId: salida.datos.ejercitoId, caravanaId, heroeId: yo });
  ctx.memoria.plan = { tipo: 'fundar', caravanaId, sitio, descartados: [] };
  return true;
}

/**
 * El titular lleva la caravana al sitio y funda. Solo, rectifica si `fundar` dice que no; en ejército el rumbo es fijo, y si el sitio no
 * vale, se repliega a su origen a intentarlo de nuevo. Sin sitio que probar, suelta el plan: la caravana vuelve sola a su origen.
 */
export async function conducirCaravana(ctx: ContextoBot, centro: Point, columna: Ejercito, plan: Extract<Plan, { tipo: 'fundar' }>): Promise<void> {
  const { vista, yo } = ctx;
  if (!columna.caravanasAdjuntasIds.includes(plan.caravanaId)) {
    if (!vista.caravanas.some((c) => c.id === plan.caravanaId)) {
      delete ctx.memoria.plan;
      return;
    }
    await ctx.intentar(`adjuntar:${plan.caravanaId}`, 'adjuntarCaravana', { ejercitoId: columna.id, caravanaId: plan.caravanaId, heroeId: yo }, 5 * 60_000);
    return;
  }
  plan.sitio ??= sitioParaFundar(ctx, centro, plan.descartados);
  if (!plan.sitio || columna.estado !== 'estacionado') return;
  if (distancia(columna.posicionActual, plan.sitio) > MOVIMIENTO.radioPuerta) {
    if (columna.tipo === 'personal') await ctx.actuar('marcharA', { heroeId: yo, objetivo: { tipo: 'punto', punto: plan.sitio } });
    else await ctx.intentar(`replegar:${columna.id}`, 'replegarEjercito', { ejercitoId: columna.id }, 10 * 60_000);
    return;
  }
  if ((await ctx.actuar('fundar', {})).ok) {
    delete ctx.memoria.plan;
    return;
  }
  plan.descartados.push(plan.sitio);
  delete plan.sitio;
  if (columna.tipo === 'ejercito') await ctx.intentar(`replegar:${columna.id}`, 'replegarEjercito', { ejercitoId: columna.id }, 10 * 60_000);
}

/** Distancia de un punto al segmento a-b. */
function distanciaASegmento(p: Point, a: Point, b: Point): number {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const largo = dx * dx + dy * dy;
  const t = largo === 0 ? 0 : Math.max(0, Math.min(1, ((p.x - a.x) * dx + (p.y - a.y) * dy) / largo));
  return distancia(p, { x: a.x + t * dx, y: a.y + t * dy });
}

/**
 * El sitio más cercano a `centro` que el mapa público dice que sirve: no agua, recomendable (madera y piedra al alcance) con las plazas que
 * conoce, a la distancia de exclusión de cualquier campamento que conozca (D16) y lejos de esas plazas. Lo que no ve lo descubre `fundar`
 * al rechazarlo. ponytail: fichas de plazas en lugar de plazas completas.
 */
export function sitioParaFundar(ctx: ContextoBot, centro: Point, descartados: readonly Point[]): Point | undefined {
  const { vista, mapa } = ctx;
  const campamentos = vista.campamentosMercenarios.map((c) => c.posicion);
  const plazas = plazasConocidas(vista).map((a) => a.posicion);
  const dentro = plazaDentro(vista);
  const conocidas = [...(dentro ? [dentro] : []), ...vista.asentamientosAvistados.filter((a) => a.id !== dentro?.id)] as unknown as Asentamiento[];
  const ocupadas = fuentesOcupadas(conocidas);
  for (let radio = SITIO.radioMin; radio <= SITIO.radioMax; radio += SITIO.paso) {
    const giro = ctx.rng() * 2 * Math.PI;
    for (let i = 0; i < SITIO.angulos; i++) {
      const a = giro + (i * 2 * Math.PI) / SITIO.angulos;
      const p = { x: Math.round(centro.x + radio * Math.cos(a)), y: Math.round(centro.y + radio * Math.sin(a)) };
      if (!mapa.dentroDelMapa(p) || mapa.terrenoEn(p) === 'agua') continue;
      if (campamentos.some((c) => distancia(c, p) < MERCENARIOS.radioExclusionFundar + 10)) continue;
      if (plazas.some((q) => distancia(q, p) < SITIO.lejosDePlazas)) continue;
      if (descartados.some((d) => distancia(d, p) < SITIO.paso)) continue;
      // Ni el sitio ni el camino hasta él cerca de un bandido conocido: la caravana es lo que atacan.
      if ([...ctx.pizarra.bandidos.values()].some((b) => distanciaASegmento(b.posicion, centro, p) < SITIO.lejosDeBandidos)) continue;
      if (esRecomendableParaFundar(mapa, p, conocidas, vista.faccionId ?? undefined, ocupadas)) return p;
    }
  }
  return undefined;
}
