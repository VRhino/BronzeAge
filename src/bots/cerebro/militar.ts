// El bot como soldado (Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md §10, filas 11-20): cada héroe recluta, guarnece y
// prepara su defensa para sí; el que no tiene cargo sale de casa —a explorar para su Facción, a cazar el campamento de
// bandidos de su plaza, de campaña contra una plaza inspeccionada o a mudarse donde hace falta— y, ya fuera, ataca
// cuando se le ofrece, se queda en lo que conquista y vuelve a casa al terminar.
//
// Las prudencias son las que la gobernanza tenía medidas en batch (placeholders hasta medir el bloque entero, §10.1).
import type { Asentamiento, Ejercito, Escuadron, Point } from '../../domain/types';
import { LOGISTICA, MILITAR, MOVIMIENTO, TROPAS_RECLUTABLES, VISION } from '../../constants';
import { edificiosPorTipoYEstado, nivelActualDe, nutricionPoblacionDe } from '../../engine/asentamientoQuery';
import { consumoRacionDeEscuadrones, reservaDeTrigo } from '../../engine/tropas';
import { poderTotal } from '../../engine/combate';
import { poderEscuadron } from '../../engine/tropa';
import { enLaPuertaDe } from '../../engine/ejercitos';
import { distancia } from '../../world/geometria';
import { estanAliadas } from '../../engine/pertenencia';
import type { ContextoBot } from '../runner';
import { soltarEncargos } from '../pizarra';
import { RESERVA_MADERA } from './gobierno';
import { columnaPropia, escuadrasLibres, plazasConocidas, estaHerido, loQueLeCabe, plazaDentro, plazasPropias, residentesDe, tieneCargo, type HeroeVisto } from './comun';

/** La mejor tropa primero; sin la leva de escalón 1, que ocuparía a 30 pesants por escuadra para casi nada. */
const TROPAS_POR_PREFERENCIA = TROPAS_RECLUTABLES.filter((t) => t.escalon > 1).sort((a, b) => b.escalon - a.escalon || b.poderBase - a.poderBase);
const NUTRICION_PARA_SALIR = 50;
const SALUD_PARA_SALIR = 0.6;
const NIVEL_PARA_CAMPANA = 2;
const ESCUADRAS_PARA_CAMPANA = 2;
const AUTONOMIA_MINIMA_TICKS = 20;
/** Lo que vale lo inspeccionado: más vieja, la defensa de una plaza ya no dice nada. */
const VIGENCIA_INSPECCION_MS = 2 * 60 * 60_000;
/** Margen sobre la defensa vista: los héroes que había dentro defienden con lo suyo, que no se ve. ponytail: a calibrar. */
const MARGEN_SOBRE_DEFENSA = 1.5;

export function residir(ctx: ContextoBot): void {
  const plaza = plazaDentro(ctx.vista);
  const heroe = ctx.vista.heroe;
  if (!plaza || !heroe || heroe.residenciaId !== plaza.id) return;
  reclutar(ctx, plaza, heroe);
  guarnecer(ctx, heroe);
  prepararDefensa(ctx);
}

/** Una escuadra por turno: la mejor que la plaza sepa hacer, abriendo una nueva cuando la mejor está al tope. */
function reclutar(ctx: ContextoBot, plaza: Asentamiento, heroe: HeroeVisto): void {
  if ((plaza.almacen['madera']?.cantidad ?? 0) < RESERVA_MADERA) return;
  const adoptadas = ctx.vista.tecnologia.propias?.adoptadas ?? [];
  for (const tropa of TROPAS_POR_PREFERENCIA) {
    if (!adoptadas.includes(tropa.tecnologia)) continue;
    const edificio = edificiosPorTipoYEstado(plaza, tropa.edificio)[0];
    if (!edificio || (edificio.nivelInterno ?? 1) < tropa.nivelRequerido) continue;
    if ((heroe.escuadrones.find((e) => e.tropaId === tropa.id)?.cantidad ?? 0) >= tropa.unidadesPorDefecto) continue;
    if (ctx.intentar(`reclutar:${tropa.id}`, 'reclutarTropa', { asentamientoId: plaza.id, heroeId: ctx.yo, tropaId: tropa.id }, 10 * 60_000)?.ok) return;
  }
}

/** Todo a la guarnición menos la última escuadra libre, con la que sale o defiende en persona (Doc 5.15.3). */
function guarnecer(ctx: ContextoBot, heroe: HeroeVisto): void {
  const libres = escuadrasLibres(heroe).sort((a, b) => poderEscuadron(b) - poderEscuadron(a) || (a.id < b.id ? -1 : 1));
  for (const escuadra of libres.slice(0, -1)) ctx.intentar(`guarnicion:${escuadra.id}`, 'asignarGuarnicion', { squadId: escuadra.id });
}

/** El loadout activo: lo que le cabe de lo que no está en la guarnición (Doc 5.12.4). Solo si cambia. */
function prepararDefensa(ctx: ContextoBot): void {
  const heroe = ctx.vista.heroe!;
  const elegidas = loQueLeCabe(heroe, escuadrasLibres(heroe)).map((e) => e.id);
  const activo = heroe.loadouts.find((l) => l.activo);
  if (activo && activo.perksSeleccionados.length === 0 && activo.squadIds.length === elegidas.length && elegidas.every((id, i) => activo.squadIds[i] === id)) return;
  ctx.actuar('guardarLoadout', { loadoutId: activo?.id, displayName: activo?.displayName ?? 'Default', squadIds: elegidas, perksSeleccionados: [], activo: true });
}

// --- Salir de casa ---

/** ¿Hasta dónde llega y vuelve con este trigo? (Doc 5.13.1). */
function alcanceDeIdaYVuelta(escuadras: readonly Escuadron[], trigo: number): number {
  const soldados = escuadras.reduce((n, e) => n + e.cantidad, 0);
  if (soldados <= 0) return 0;
  const ticks = trigo / (soldados * MILITAR.racionPorSoldadoPorMinuto);
  if (ticks < AUTONOMIA_MINIMA_TICKS) return 0;
  const velocidades = escuadras.map((e) => TROPAS_RECLUTABLES.find((t) => t.id === e.tropaId)?.velocidad).filter((v): v is number => v !== undefined);
  return velocidades.length === 0 ? 0 : (ticks * Math.min(...velocidades)) / 2;
}

function salud(escuadras: readonly Escuadron[]): number {
  let cantidad = 0;
  let nominal = 0;
  for (const e of escuadras) {
    const t = TROPAS_RECLUTABLES.find((x) => x.id === e.tropaId);
    if (!t) continue;
    cantidad += e.cantidad;
    nominal += t.unidadesPorDefecto;
  }
  return nominal <= 0 ? 1 : cantidad / nominal;
}

/** El que está en casa, sano y sin cargo, decide si sale y a qué. */
export function salir(ctx: ContextoBot): void {
  const { vista, yo, pizarra } = ctx;
  const plaza = plazaDentro(vista);
  const heroe = vista.heroe;
  if (!plaza || !heroe || heroe.residenciaId !== plaza.id || estaHerido(vista) || tieneCargo(vista, yo, plaza)) return;
  delete ctx.memoria.plan;
  soltarEncargos(pizarra, yo);

  if (unirseALaCampanaDeCasa(ctx, plaza, heroe)) return;
  if (explorar(ctx, plaza)) return;
  if (cazar(ctx, plaza, heroe)) return;
  if (lanzarCampana(ctx, plaza, heroe)) return;
  mudarse(ctx, plaza);
}

/** Un explorador por Facción (§10.1): el de id más bajo sin cargo, si la Facción tiene al menos tres héroes. */
function explorar(ctx: ContextoBot, plaza: Asentamiento): boolean {
  const { yo, pizarra } = ctx;
  if (pizarra.residencias.size < 3) return false;
  const explorador = pizarra.encargos.get('explorador');
  if (explorador && explorador !== yo) return false;
  const objetivo = siguienteAExplorar(ctx, plaza.posicion);
  if (!objetivo) return false;
  const salida = ctx.intentar('salir:explorar', 'salirAlMundo', { asentamientoId: plaza.id, heroeId: yo, escuadronIds: [], carga: { trigo: Math.min(100, plaza.almacen['trigo']?.cantidad ?? 0) } });
  if (!salida?.ok) return false;
  pizarra.encargos.set('explorador', yo);
  ctx.memoria.plan = { tipo: 'explorar', plazaId: objetivo.id };
  ctx.actuar('marcharA', { heroeId: yo, objetivo: { tipo: 'punto', punto: objetivo.posicion } });
  return true;
}

/** La plaza ajena conocida más cercana cuya defensa no está en la pizarra o está vieja. */
function siguienteAExplorar(ctx: ContextoBot, desde: Point): { id: string; posicion: Point } | undefined {
  const { vista, pizarra } = ctx;
  const ajenas = plazasConocidas(vista).filter((a) => a.faccionId !== vista.faccionId);
  return ajenas
    .filter((a) => {
      const vista_ = pizarra.defensas.get(a.id);
      return !vista_ || vista.instante - vista_.vistoEn > VIGENCIA_INSPECCION_MS;
    })
    .sort((a, b) => distancia(a.posicion, desde) - distancia(b.posicion, desde) || (a.id < b.id ? -1 : 1))[0];
}

/** El campamento de bandidos que atiende a su plaza (Doc 1.9), si lo ve, nadie va ya y puede con él. */
function cazar(ctx: ContextoBot, plaza: Asentamiento, heroe: HeroeVisto): boolean {
  const { vista, yo, pizarra } = ctx;
  const campamento = vista.campamentosBandidos.find((c) => c.asentamientoId === plaza.id);
  if (!campamento) return false;
  const encargado = pizarra.encargos.get(`bandido:${campamento.id}`);
  if (encargado && encargado !== yo) return false;
  if (nutricionPoblacionDe(plaza) < NUTRICION_PARA_SALIR) return false;
  const escuadras = loQueLeCabe(heroe, escuadrasLibres(heroe));
  if (escuadras.length === 0 || salud(escuadras) < SALUD_PARA_SALIR || poderTotal(escuadras, false) <= campamento.poder) return false;
  const trigo = Math.max(0, (plaza.almacen['trigo']?.cantidad ?? 0) - reservaDeTrigo(plaza, consumoRacionDeEscuadrones(escuadras)));
  if (distancia(plaza.posicion, campamento.posicion) > alcanceDeIdaYVuelta(escuadras, Math.min(trigo, LOGISTICA.capacidadCarroPorJugador))) return false;
  const r = ctx.intentar(`cazar:${campamento.id}`, 'movilizarEjercito', {
    asentamientoId: plaza.id,
    heroeId: yo,
    escuadronIds: escuadras.map((e) => e.id),
    objetivo: { tipo: 'punto', punto: campamento.posicion },
  });
  if (!r?.ok) return false;
  pizarra.encargos.set(`bandido:${campamento.id}`, yo);
  ctx.memoria.plan = { tipo: 'cazar', campamentoId: campamento.id };
  return true;
}

/**
 * Campaña contra una plaza rival que la pizarra tiene inspeccionada hace poco y que puede ganar (§10.1: solo contra lo que
 * se ha mirado), al alcance de la comida, que no sea la última de su Facción. Sale como líder; los demás que estén libres
 * en casa se le unen en la puerta (`unirseALaCampanaDeCasa`). Una sola campaña por plaza, y nunca se vacía la casa.
 */
function lanzarCampana(ctx: ContextoBot, plaza: Asentamiento, heroe: HeroeVisto): boolean {
  const { vista, yo, pizarra } = ctx;
  if (nivelActualDe(plaza) < NIVEL_PARA_CAMPANA || residentesDe(plaza).length < 2) return false;
  if ([...pizarra.encargos.keys()].some((k) => k.startsWith(`campana-desde:${plaza.id}:`))) return false;
  const escuadras = loQueLeCabe(heroe, escuadrasLibres(heroe));
  if (escuadras.length < Math.min(ESCUADRAS_PARA_CAMPANA, escuadrasLibres(heroe).length) || escuadras.length === 0) return false;
  const trigo = Math.max(0, (plaza.almacen['trigo']?.cantidad ?? 0) - reservaDeTrigo(plaza, consumoRacionDeEscuadrones(escuadras)));
  const alcance = alcanceDeIdaYVuelta(escuadras, Math.min(trigo, LOGISTICA.capacidadCarroPorJugador));
  const poder = poderTotal(escuadras, false);
  const conocidas = plazasConocidas(vista);
  const plazasDe = (faccionId: string) => new Set(conocidas.filter((a) => a.faccionId === faccionId).map((a) => a.id)).size;

  const objetivo = conocidas
    .filter((a) => a.faccionId !== vista.faccionId && !estanAliadas(vista.relaciones, vista.faccionId ?? '', a.faccionId))
    .filter((a) => plazasDe(a.faccionId) > 1 && distancia(a.posicion, plaza.posicion) <= alcance)
    .filter((a) => {
      const vista_ = pizarra.defensas.get(a.id);
      if (!vista_ || vista.instante - vista_.vistoEn > VIGENCIA_INSPECCION_MS) return false;
      const guarnicion = vista_.defensa.guarnicion.map((g) => ({ tropaId: g.tropaId, cantidad: g.cantidad, nivel: 0 }) as Escuadron);
      return poder > poderTotal(guarnicion, true) * MARGEN_SOBRE_DEFENSA * (1 + vista_.defensa.heroesIds.length);
    })
    .sort((a, b) => distancia(a.posicion, plaza.posicion) - distancia(b.posicion, plaza.posicion) || (a.id < b.id ? -1 : 1))[0];
  if (!objetivo) return false;

  const r = ctx.intentar(`campana:${objetivo.id}`, 'movilizarEjercito', {
    asentamientoId: plaza.id,
    heroeId: yo,
    escuadronIds: escuadras.map((e) => e.id),
    objetivo: { tipo: 'asentamiento', id: objetivo.id },
    politicaDeUnion: 'aceptar',
  });
  if (!r?.ok || !r.datos) return false;
  pizarra.encargos.set(`campana-desde:${plaza.id}:${r.datos.ejercitoId}`, yo);
  ctx.memoria.plan = { tipo: 'campana', plazaId: objetivo.id };
  return true;
}

/** Si un compañero acaba de salir de campaña desde casa y la columna sigue en la puerta, se le une con lo que le cabe. */
function unirseALaCampanaDeCasa(ctx: ContextoBot, plaza: Asentamiento, heroe: HeroeVisto): boolean {
  const { vista, yo, pizarra } = ctx;
  // Nunca se vacía la casa: se queda al menos el que tiene cargo, y como mucho salen todos menos uno.
  const fuera = vista.ejercitos.filter((e) => e.origenAsentamientoId === plaza.id).flatMap((e) => e.participantes).length;
  if (fuera >= residentesDe(plaza).length - 1) return false;
  for (const clave of pizarra.encargos.keys()) {
    if (!clave.startsWith(`campana-desde:${plaza.id}:`)) continue;
    const ejercitoId = clave.slice(`campana-desde:${plaza.id}:`.length);
    const columna = vista.ejercitos.find((e) => e.id === ejercitoId);
    if (!columna || !enLaPuertaDe(columna, plaza) || columna.objetivo.tipo !== 'asentamiento') continue;
    const escuadras = loQueLeCabe(heroe, escuadrasLibres(heroe));
    if (escuadras.length === 0) continue;
    if (ctx.intentar(`unirse:${ejercitoId}`, 'unirseAEjercito', { ejercitoId, asentamientoId: plaza.id, heroeId: yo, escuadronIds: escuadras.map((e) => e.id) })?.ok) {
      ctx.memoria.plan = { tipo: 'campana', plazaId: columna.objetivo.id };
      return true;
    }
  }
  return false;
}

/** Reparto de la defensa (2026-09-27): si su plaza tiene 2 residentes más que la que menos tiene, se muda a ella. */
function mudarse(ctx: ContextoBot, plaza: Asentamiento): void {
  const { pizarra, yo } = ctx;
  const cuenta = new Map<string, number>();
  for (const p of plazasPropias(ctx.vista)) cuenta.set(p.id, 0);
  for (const [, p] of pizarra.residencias) if (cuenta.has(p)) cuenta.set(p, cuenta.get(p)! + 1);
  const menos = [...cuenta].sort((a, b) => a[1] - b[1] || (a[0] < b[0] ? -1 : 1))[0];
  if (!menos || menos[0] === plaza.id || (cuenta.get(plaza.id) ?? 0) - menos[1] < 2) return;
  const destino = plazasPropias(ctx.vista).find((p) => p.id === menos[0])!;
  const salida = ctx.intentar(`mudarse:${destino.id}`, 'salirAlMundo', { asentamientoId: plaza.id, heroeId: yo, escuadronIds: [], carga: {} });
  if (!salida?.ok) return;
  if (!ctx.actuar('cambiarResidencia', { destinoId: destino.id, heroeId: yo }).ok) return;
  pizarra.residencias.set(yo, destino.id);
  ctx.memoria.plan = { tipo: 'mudarse', plazaId: destino.id };
  ctx.actuar('marcharA', { heroeId: yo, objetivo: { tipo: 'asentamiento', id: destino.id } });
}

// --- Fuera, en su columna ---

export function enColumna(ctx: ContextoBot): void {
  const { vista, yo, memoria } = ctx;
  const columna = columnaPropia(vista, yo);
  if (!columna || columna.liderId !== yo) return; // quien no lidera va donde va su líder
  const plan = memoria.plan;

  if (plan?.tipo === 'explorar') return enExploracion(ctx, columna, plan.plazaId);
  if (columna.estado !== 'estacionado') return perseguirLoQueVe(ctx, columna);

  if (plan?.tipo === 'cazar') {
    const campamento = vista.campamentosBandidos.find((c) => c.id === plan.campamentoId);
    if (campamento && distancia(columna.posicionActual, campamento.posicion) <= LOGISTICA.radioEncuentro) {
      ctx.actuar('atacar', { heroeId: yo, objetivo: { tipo: 'campamento', id: campamento.id } });
    }
    return volverACasa(ctx, columna);
  }

  if (plan?.tipo === 'campana') {
    const plaza = [...vista.asentamientosAvistados].find((a) => a.id === plan.plazaId);
    if (plaza && plaza.faccionId !== vista.faccionId && distancia(columna.posicionActual, plaza.posicion) <= LOGISTICA.radioEncuentro) {
      const r = ctx.actuar('atacar', { heroeId: yo, objetivo: { tipo: 'asentamiento', id: plaza.id } });
      if (r.ok) return; // el resultado se ve en el turno siguiente
    }
    if (plaza && plaza.faccionId === vista.faccionId && quedarseEnLoConquistado(ctx, plaza.id)) return;
    return volverACasa(ctx, columna);
  }

  if (plan?.tipo === 'mudarse') {
    const r = ctx.actuar('entrarEnAsentamiento', { asentamientoId: plan.plazaId, heroeId: yo });
    if (!r.ok) ctx.actuar('marcharA', { heroeId: yo, objetivo: { tipo: 'asentamiento', id: plan.plazaId } });
    return;
  }

  volverACasa(ctx, columna);
}

/**
 * Quien conquista se queda a defender (§4.9, 2026-09-27): la plaza recién ganada está sin residentes; el líder se muda a
 * ella y entra con su columna, si con eso no deja su casa vacía.
 */
function quedarseEnLoConquistado(ctx: ContextoBot, plazaId: string): boolean {
  const { yo, pizarra } = ctx;
  const casa = ctx.memoria.residenciaId;
  const quedanEnCasa = [...pizarra.residencias].filter(([h, p]) => p === casa && h !== yo).length;
  if (!casa || quedanEnCasa < 1) return false;
  if (!ctx.actuar('cambiarResidencia', { destinoId: plazaId, heroeId: yo }).ok) return false;
  pizarra.residencias.set(yo, plazaId);
  soltarEncargos(pizarra, yo);
  ctx.actuar('guarnecer', { asentamientoId: plazaId, heroeId: yo });
  delete ctx.memoria.plan;
  return true;
}

/** A casa: si está en su puerta, entra; si no, se repliega (ejército) o marcha (columna personal). */
function volverACasa(ctx: ContextoBot, columna: Ejercito): void {
  const { yo, memoria } = ctx;
  soltarEncargos(ctx.pizarra, yo);
  delete memoria.plan;
  const casa = memoria.residenciaId;
  if (!casa) return;
  if (ctx.actuar('entrarEnAsentamiento', { asentamientoId: casa, heroeId: yo }).ok) return;
  if (columna.tipo === 'ejercito') ctx.intentar(`replegar:${columna.id}`, 'replegarEjercito', { ejercitoId: columna.id }, 10 * 60_000);
  else ctx.intentar(`volver:${columna.id}`, 'marcharA', { heroeId: yo, objetivo: { tipo: 'asentamiento', id: casa } }, 10 * 60_000);
}

/**
 * Va a por lo que tiene a la vista (Doc 5.12.3): la columna enemiga más cercana con soldados o, si no, una caravana
 * enemiga sin escolta. Al alcanzarla, el motor se lo ofrece y la ataca. No persigue a los suyos ni a un aliado.
 */
function perseguirLoQueVe(ctx: ContextoBot, columna: Ejercito): void {
  const { vista, yo } = ctx;
  if (estaHerido(vista)) return;
  const enemiga = (faccionId: string | undefined) =>
    faccionId !== undefined && faccionId !== vista.faccionId && !estanAliadas(vista.relaciones, vista.faccionId ?? '', faccionId);
  const presa = columna.persiguiendo;
  if (presa) {
    const donde =
      presa.tipo === 'ejercito' ? vista.ejercitosAvistados.find((e) => e.id === presa.id)?.posicionActual : vista.caravanasAvistadas.find((c) => c.id === presa.id)?.posicionActual;
    if (donde && distancia(donde, columna.posicionActual) <= LOGISTICA.radioEncuentro) ctx.actuar('atacar', { heroeId: yo, objetivo: presa });
    return;
  }
  if (columna.escuadronIds.length === 0) return;
  const cerca = (p: Point) => distancia(p, columna.posicionActual) <= VISION.ejercito;
  const rival = vista.ejercitosAvistados.filter((e) => enemiga(e.faccionId) && cerca(e.posicionActual)).sort((a, b) => (a.id < b.id ? -1 : 1))[0];
  if (rival) {
    ctx.intentar(`perseguir:${rival.id}`, 'perseguir', { heroeId: yo, objetivo: { tipo: 'ejercito', id: rival.id } });
    return;
  }
  const caravana = vista.caravanasAvistadas.filter((c) => !c.escoltada && enemiga(c.faccionId) && cerca(c.posicionActual)).sort((a, b) => (a.id < b.id ? -1 : 1))[0];
  if (caravana) ctx.intentar(`perseguir:${caravana.id}`, 'perseguir', { heroeId: yo, objetivo: { tipo: 'caravana', id: caravana.id } });
}

/**
 * El explorador (§10.1): marcha a la plaza ajena, la inspecciona desde el anillo de 40 y, en su puerta, apunta lo que
 * vende y compra en su mostrador. Luego la siguiente; sin más que mirar, vuelve a casa.
 */
function enExploracion(ctx: ContextoBot, columna: Ejercito, plazaId: string): void {
  const { vista, yo, pizarra } = ctx;
  const ficha = plazasConocidas(vista).find((a) => a.id === plazaId);
  if (!ficha) return volverACasa(ctx, columna);
  const d = distancia(columna.posicionActual, ficha.posicion);
  if (d <= MOVIMIENTO.radioInspeccion && !inspeccionadaHace(ctx, plazaId)) {
    const r = ctx.actuar('inspeccionar', { heroeId: yo, objetivo: { tipo: 'asentamiento', id: plazaId } });
    if (r.ok && r.datos && 'guarnicion' in r.datos) pizarra.defensas.set(plazaId, { defensa: r.datos, vistoEn: vista.instante });
  }
  const mostrador = vista.ordenes.filter((o) => o.asentamientoId === plazaId && o.estado === 'activa');
  if (mostrador.length > 0 || (columna.estado === 'estacionado' && d <= MOVIMIENTO.radioInspeccion)) {
    pizarra.mostradores.set(plazaId, { ordenes: mostrador, vistoEn: vista.instante });
  }
  if (!inspeccionadaHace(ctx, plazaId) || !pizarra.mostradores.has(plazaId)) {
    if (columna.estado === 'estacionado') ctx.intentar(`explorar:${plazaId}`, 'marcharA', { heroeId: yo, objetivo: { tipo: 'asentamiento', id: plazaId } }, 10 * 60_000);
    return;
  }
  const siguiente = siguienteAExplorar(ctx, columna.posicionActual);
  if (!siguiente) return volverACasa(ctx, columna);
  ctx.memoria.plan = { tipo: 'explorar', plazaId: siguiente.id };
  ctx.actuar('marcharA', { heroeId: yo, objetivo: { tipo: 'punto', punto: siguiente.posicion } });
}

function inspeccionadaHace(ctx: ContextoBot, plazaId: string): boolean {
  const vista_ = ctx.pizarra.defensas.get(plazaId);
  return vista_ !== undefined && ctx.vista.instante - vista_.vistoEn <= VIGENCIA_INSPECCION_MS;
}
