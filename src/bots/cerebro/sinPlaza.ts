// El bot sin plaza (Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md §3, D53-D59): el arranque de un jugador en los campamentos.
// Se busca una Facción (la crea o pide entrar, según su perfil, D57), pide la tropa prestada, caza los bandidos del anillo de
// su campamento —solo o con los compañeros que residen allí— y abre los alijos que ve por el camino; con el oro de botín compra
// en el mercado y aporta al fondo, y cuando el fondo alcanza, uno compra la caravana, sale con los que quieran ir, la lleva a un
// sitio y funda. Si su Facción ya tiene una plaza, se muda a ella. Una Facción que pierde su última plaza vuelve a esto (D59).
//
// Las cifras son placeholders hasta medir con batch (§10.1).
import type { CampamentoMercenarios, Ejercito, Escuadron, Faccion, Point } from '../../domain/types';
import { CAMPAMENTOS_BANDIDOS, LOGISTICA, MERCENARIOS, MOVIMIENTO, TROPAS_RECLUTABLES, VISION } from '../../constants';
import { costoRefundacion, fondoDeFaccion } from '../../engine/refundacion';
import { poderTotal } from '../../engine/combate';
import { consumoRacionDeColumna } from '../../engine/tropas';
import { tamanoPrestada } from '../../engine/reclutamientoMercenario';
import { distancia } from '../../world/geometria';
import { instante, type Instante } from '../../domain/tiempo';
import type { ContextoBot } from '../runner';
import { columnaPropia, plazasConocidas } from './comun';
import { conducirCaravana, sitioParaFundar } from './fundar';

/** La tropa que pide prestada: la milicia de lanceros (poder 50 contra los 20 de un nivel 1). Con las tres comería el triple. */
const PRESTAMO = ['milicia_lanceros'];
/** Poder de la leva que pide prestada: 25 lanceros de poder 2, la escuadra completa (D80). Fija cuántos hacen falta para cada nivel de bandido. */
const PODER_PRESTADO = 50;
/** Lo que vale «estoy dentro y listo» en la pizarra: algo más que el ritmo con el que piensa un bot (cada 5 min). */
const ESPERA_LISTO_MS = 6 * 60_000;
/** Lo que el titular espera a sus compañeros antes de salir a fundar sin ellos. */
const ESPERA_COFUNDADORES_MS = 40 * 60_000;
/** Puntos de la vuelta al anillo del explorador, y lo que anda como mucho antes de volver. */
const VUELTA_ANILLO = 8;
const EXPLORACION_MAXIMA_MS = 3 * 60 * 60_000;
/** Lo que vale un bandido apuntado en la pizarra. */
const VIGENCIA_BANDIDO_MS = 3 * 60 * 60_000;
/** Lo que espera respuesta a una solicitud de ingreso antes de fundar la suya. */
const ESPERA_SOLICITUD_MS = 2 * 60 * 60_000;
/** Margen de trigo para volver a casa, sobre lo justo, y los minutos de más: piensa cada 5 y el camino no es recto. */
const MARGEN_TRIGO = 1.1;
const MINUTOS_DE_MAS = 3;
/** Por debajo de esta fracción de su tropa, vuelve a reponer. */
const SALUD_PARA_SEGUIR = 0.5;
const VELOCIDAD_A_PIE = 20;
const ORDEN_DEL_FONDO = ['madera', 'piedra', 'trigo', 'oro'];

export async function sinPlaza(ctx: ContextoBot): Promise<void> {
  const { vista, yo } = ctx;
  const heroe = vista.heroe!;
  const casa = vista.campamentosMercenarios.find((c) => c.residentesIds.includes(yo));
  const dentro = heroe.ubicacion.tipo === 'mercenarios';
  const faccion = vista.facciones.find((f) => f.id === vista.faccionId);
  if (!faccion) {
    if (dentro) await buscarFaccion(ctx);
    return;
  }
  if (await irAVivirConLosSuyos(ctx, faccion)) return;
  if (!casa) return;
  if (dentro) return await enElCampamento(ctx, casa, faccion);
  const columna = columnaPropia(vista, yo);
  if (columna) await enCampo(ctx, casa);
}

// --- La Facción ---

/** D57: el líder de los amigos y el solitario crean la suya; los demás piden entrar, y sin respuesta al rato, la crean. */
async function buscarFaccion(ctx: ContextoBot): Promise<void> {
  const { vista, yo, perfil, memoria } = ctx;
  const crear = async () => {
    const nombre = `Casa de ${vista.heroe!.displayName}`;
    const r = await ctx.intentar('crearFaccion', 'crearFaccion', { nombre });
    // Otro héroe del mundo ya se llama como este (restos de una corrida anterior en la misma partida): se distingue por el id.
    if (r && !r.ok && r.codigoError === 'faccion.nombre_duplicado') await ctx.actuar('crearFaccion', { nombre: `${nombre} (${yo})` });
  };
  if (memoria.solicitud) {
    if (vista.instante - memoria.solicitud.desde < ESPERA_SOLICITUD_MS) return;
    delete memoria.solicitud;
    await crear();
    return;
  }
  if (perfil.tipo === 'solitario' || (perfil.tipo === 'amigos' && perfil.lider === yo)) {
    await crear();
    return;
  }
  const destino =
    perfil.tipo === 'amigos'
      ? vista.facciones.find((f) => f.reyId === perfil.lider)
      : elegir(ctx, vista.facciones.filter((f) => f.reyId));
  if (!destino) return; // el líder aún no la ha creado
  if ((await ctx.actuar('solicitarIngreso', { faccionId: destino.id })).ok) memoria.solicitud = { faccionId: destino.id, desde: vista.instante };
}

function elegir<T>(ctx: ContextoBot, lista: readonly T[]): T | undefined {
  return lista.length === 0 ? undefined : lista[Math.floor(ctx.rng() * lista.length)];
}

/** Si su Facción tiene una plaza que conoce, se muda a ella (Doc 2: `cambiarResidencia`): sale, cambia de casa y va. */
async function irAVivirConLosSuyos(ctx: ContextoBot, faccion: Faccion): Promise<boolean> {
  const { vista, yo } = ctx;
  delete ctx.memoria.solicitud;
  const desde = columnaPropia(vista, yo)?.posicionActual ?? vista.campamentosMercenarios.find((c) => c.residentesIds.includes(yo))?.posicion;
  if (!desde) return false;
  const plaza = plazasConocidas(vista)
    .filter((a) => a.faccionId === faccion.id)
    .sort((a, b) => distancia(a.posicion, desde) - distancia(b.posicion, desde) || (a.id < b.id ? -1 : 1))[0];
  if (!plaza) return false;
  const heroe = vista.heroe!;
  if (heroe.ubicacion.tipo === 'mercenarios') {
    const propias = heroe.escuadrones.filter((e) => e.contenedor.tipo === 'campamento' && !e.prestada).map((e) => e.id);
    if (!(await ctx.intentar(`salir:${heroe.ubicacion.campamentoId}`, 'salirDelCampamento', { campamentoId: heroe.ubicacion.campamentoId, heroeId: yo, escuadronIds: propias, carga: {} }, 10 * 60_000))?.ok) return false;
  }
  if (!(await ctx.intentar(`mudarse:${plaza.id}`, 'cambiarResidencia', { destinoId: plaza.id, heroeId: yo }))?.ok) return false;
  ctx.memoria.plan = { tipo: 'mudarse', plazaId: plaza.id };
  await ctx.actuar('marcharA', { heroeId: yo, objetivo: { tipo: 'asentamiento', id: plaza.id } });
  return true;
}

// --- Dentro del campamento ---

async function enElCampamento(ctx: ContextoBot, casa: CampamentoMercenarios, faccion: Faccion): Promise<void> {
  const { vista, yo, pizarra } = ctx;
  const heroe = vista.heroe!;
  await pedirTropa(ctx);

  // Un compañero acaba de salir y le espera en la puerta: se le une, en este mismo tick (el runner le llama).
  const salida = pizarra.salidas.get(casa.id);
  if (salida && salida.liderId !== yo && salida.hasta > vista.instante) {
    // A cazar y a fundar, con su leva: a fundar va de escolta de la caravana (los bandidos atacan una caravana sin ella).
  const escuadras = tropaEnCampamento(heroe.escuadrones).map((e) => e.id);
    if (!(await ctx.intentar(`salir:${casa.id}`, 'salirDelCampamento', { campamentoId: casa.id, heroeId: yo, escuadronIds: escuadras, carga: {} }, 10 * 60_000))?.ok) return;
    pizarra.listos.delete(yo);
    ctx.memoria.plan = (await ctx.actuar('unirseEnCampo', { ejercitoId: salida.ejercitoId, heroeId: yo })).ok
      ? { tipo: 'unirse', ejercitoId: salida.ejercitoId, para: salida.para }
      : { tipo: 'anillo', campamentoId: casa.id };
    return;
  }

  const racionLista = heroe.racionEn === undefined || vista.instante - heroe.racionEn >= MERCENARIOS.racion.cadaMinutos * 60_000;
  const escuadras = tropaEnCampamento(heroe.escuadrones);
  const conRacion = racionLista && escuadras.length > 0 && salud(escuadras) >= SALUD_PARA_SEGUIR;
  pizarra.listos.set(yo, { campamentoId: casa.id, conRacion, hasta: instante(vista.instante + ESPERA_LISTO_MS) });
  const listos = [...pizarra.listos].filter(([, l]) => l.campamentoId === casa.id && l.hasta > vista.instante);

  // La caravana que compró: espera un rato a que vuelvan los compañeros que viven aquí y sale a fundar con los que estén
  // dentro (cofundadores, M2).
  const caravana = vista.caravanas.find((c) => c.tipo === 'construccion' && c.titularId === yo && c.faccionId === faccion.id);
  if (caravana) {
    // Antes de sacarla, que alguien haya mirado el anillo hace poco: por dónde andan los bandidos decide el sitio y el camino.
    const explorado = pizarra.explorados.get(casa.id);
    if (explorado === undefined || vista.instante - explorado > VIGENCIA_BANDIDO_MS) return await explorarElAnillo(ctx, casa, listos);
    const companeros = casa.residentesIds.filter((id) => id !== yo && faccion.ciudadanosIds.includes(id)).length;
    ctx.memoria.esperaFundar ??= vista.instante;
    if (listos.length - 1 < companeros && vista.instante - ctx.memoria.esperaFundar < ESPERA_COFUNDADORES_MS) return;
    delete ctx.memoria.esperaFundar;
    return await salirAFundar(ctx, casa, caravana.id, listos.length - 1);
  }
  // La de un compañero: se queda dentro, listo para salir con él.
  if (vista.caravanas.some((c) => c.tipo === 'construccion' && c.faccionId === faccion.id && distancia(c.posicionActual, casa.posicion) <= MOVIMIENTO.radioPuerta)) return;

  if (await aportarYComprar(ctx, casa, faccion)) return;

  // Sin un bandido conocido que puedan los que viven aquí, uno sale a buscar sin tropa: no come, y la ración (D24) solo da
  // para ir y volver del anillo, no para recorrerlo.
  const residentes = casa.residentesIds.filter((id) => faccion.ciudadanosIds.includes(id)).length;
  if (!presaConocida(ctx, casa.posicion, residentes)) return await explorarElAnillo(ctx, casa, listos);
  if (!conRacion) return;

  // A cazar: el bandido conocido más cercano que puedan los que están listos, derecho a él. El de menor id lleva al grupo.
  const enGrupo = listos.filter(([, l]) => l.conRacion).map(([id]) => id).sort();
  const presa = presaConocida(ctx, casa.posicion, enGrupo.length);
  if (!presa) return; // esperan a juntarse los que hacen falta
  const necesarios = heroesPara(presa.poder);
  if (necesarios > 1 && enGrupo[0] !== yo) return; // le llamará el que lleva al grupo
  if (necesarios === 1) {
    if (!await salirSolo(ctx, casa, escuadras.map((e) => e.id))) return;
    ctx.memoria.plan = { tipo: 'anillo', campamentoId: casa.id };
    await ctx.actuar('marcharA', { heroeId: yo, objetivo: { tipo: 'punto', punto: presa.posicion } });
    return;
  }
  const r = await ctx.intentar(`salir:${casa.id}`, 'salirDelCampamento', {
    campamentoId: casa.id,
    heroeId: yo,
    escuadronIds: escuadras.map((e) => e.id),
    carga: {},
    politicaDeUnion: 'aceptar',
    objetivo: { tipo: 'punto', punto: presa!.posicion },
  }, 10 * 60_000);
  if (!r?.ok || !r.datos) return;
  pizarra.listos.delete(yo);
  pizarra.salidas.set(casa.id, { ejercitoId: r.datos.ejercitoId, liderId: yo, hasta: instante(vista.instante + 1), para: 'cazar' });
  ctx.memoria.plan = { tipo: 'anillo', campamentoId: casa.id };
}

const tropaEnCampamento = (escuadrones: readonly Escuadron[]) => escuadrones.filter((e) => e.contenedor.tipo === 'campamento' && e.cantidad > 0);

/** La ración gratis con la que sale (D24, D90): los víveres llenos. */
const racionAlSalir = (): number => LOGISTICA.capacidadViveresPorHeroe;

/** Cuántos héroes con su leva prestada hacen falta para vencer ese poder. */
const heroesPara = (poder: number) => Math.floor(poder / PODER_PRESTADO) + 1;

/**
 * De los bandidos que la pizarra conoce, el más cercano que puedan `listos` héroes y al que les llegue la ración (D24) para ir y
 * volver: a uno más lejos se llega sin trigo y la tropa deserta.
 */
function presaConocida(ctx: ContextoBot, desde: Point, listos: number): { posicion: Point; poder: number } | undefined {
  const leva = ctx.vista.heroe!.escuadrones.filter((e) => e.prestada);
  return [...ctx.pizarra.bandidos.values()]
    .filter((b) => ctx.vista.instante - b.vistoEn <= VIGENCIA_BANDIDO_MS && heroesPara(b.poder) <= listos)
    .filter((b) => leva.length === 0 || 2 * trigoDelViaje(ctx, desde, b.posicion, leva) * MARGEN_TRIGO <= racionAlSalir())
    .sort((a, b) => distancia(a.posicion, desde) - distancia(b.posicion, desde))[0];
}

/** La leva comunal prestada (D80), y repuesta cuando ha perdido gente. */
async function pedirTropa(ctx: ContextoBot): Promise<void> {
  const heroe = ctx.vista.heroe!;
  const prestadas = heroe.escuadrones.filter((e) => e.prestada);
  const faltan = PRESTAMO.filter((t) => !prestadas.some((e) => e.tropaId === t));
  if (faltan.length > 0) await ctx.intentar('prestamo', 'pedirPrestamo', { tropaIds: faltan });
  else if (prestadas.some((e) => e.cantidad < tamanoPrestada(e))) await ctx.intentar('reponer', 'reponerPrestamo', {});
}

/** Sale sin tropa a recorrer el anillo, si no hay ya otro fuera haciéndolo. */
async function explorarElAnillo(ctx: ContextoBot, casa: CampamentoMercenarios, listos: readonly [string, unknown][]): Promise<void> {
  const { pizarra, yo, vista } = ctx;
  const explorador = pizarra.encargos.get(`explorar:${casa.id}`);
  if (explorador && explorador !== yo && listos.every(([id]) => id !== explorador)) return; // ya hay uno fuera
  if (!await salirSolo(ctx, casa, [])) return;
  pizarra.encargos.set(`explorar:${casa.id}`, yo);
  ctx.memoria.plan = { tipo: 'anillo', campamentoId: casa.id, explorar: { desde: vista.instante, paso: 0, giro: ctx.rng() * 2 * Math.PI } };
}

/** Sale solo, con su columna personal: rectifica el rumbo cuando quiere (Doc 5.12.1). */
async function salirSolo(ctx: ContextoBot, casa: CampamentoMercenarios, escuadronIds: string[]): Promise<boolean> {
  const r = await ctx.intentar(`salir:${casa.id}`, 'salirDelCampamento', { campamentoId: casa.id, heroeId: ctx.yo, escuadronIds, carga: {} }, 10 * 60_000);
  if (r?.ok) ctx.pizarra.listos.delete(ctx.yo);
  return r?.ok === true;
}

/**
 * El titular sale con la caravana (D10-D12) hacia un sitio elegido antes de salir: con compañeros dentro, sale como ejército con
 * ese rumbo y ellos se le unen en este mismo tick (cofundadores, M2); solo, con su columna personal. La engancha en la puerta,
 * antes de que la columna se mueva.
 */
async function salirAFundar(ctx: ContextoBot, casa: CampamentoMercenarios, caravanaId: string, companeros: number): Promise<void> {
  const { vista, yo, pizarra } = ctx;
  const sitio = sitioParaFundar(ctx, casa.posicion, []);
  if (!sitio) return;
  const r = await ctx.intentar(`salir:${casa.id}`, 'salirDelCampamento', {
    campamentoId: casa.id,
    heroeId: yo,
    escuadronIds: tropaEnCampamento(vista.heroe!.escuadrones).map((e) => e.id),
    carga: {},
    ...(companeros > 0 ? { politicaDeUnion: 'aceptar' as const, objetivo: { tipo: 'punto' as const, punto: sitio } } : {}),
  }, 10 * 60_000);
  if (!r?.ok || !r.datos) return;
  pizarra.listos.delete(yo);
  await ctx.actuar('adjuntarCaravana', { ejercitoId: r.datos.ejercitoId, caravanaId, heroeId: yo });
  if (companeros > 0) pizarra.salidas.set(casa.id, { ejercitoId: r.datos.ejercitoId, liderId: yo, hasta: instante(vista.instante + 1), para: 'fundar' });
  ctx.memoria.plan = { tipo: 'fundar', caravanaId, sitio, descartados: [] };
}

/**
 * El fondo de su Facción en este campamento (D39): si ya alcanza, compra la caravana; si no, pone lo que falta de lo que tiene,
 * comprando en el mercado con su oro de botín lo que no tenga, y el oro al final. Devuelve si compró la caravana.
 */
async function aportarYComprar(ctx: ContextoBot, casa: CampamentoMercenarios, faccion: Faccion): Promise<boolean> {
  const heroe = ctx.vista.heroe!;
  const coste = costoRefundacion();
  const fondo = fondoDeFaccion(casa, faccion);
  const falta = (r: string) => Math.max(0, (coste[r] ?? 0) - (fondo[r] ?? 0));
  if (ORDEN_DEL_FONDO.every((r) => falta(r) === 0)) {
    return (await ctx.intentar(`caravana:${casa.id}`, 'comprarCaravanaDeRefundacion', {}, 10 * 60_000))?.ok === true;
  }
  for (const recurso of ORDEN_DEL_FONDO) {
    const pide = falta(recurso);
    if (pide === 0) continue;
    // El oro de botín paga primero los materiales en el mercado; al fondo, el que sobre cuando ya estén.
    if (recurso === 'oro' && ORDEN_DEL_FONDO.some((r) => r !== 'oro' && falta(r) > 0)) break;
    let tiene = recurso === 'oro' ? (heroe.oroDeBotin ?? 0) : (heroe.almacenPersonal?.[recurso] ?? 0);
    if (recurso !== 'oro' && tiene < pide) {
      const compra = await ctx.intentar(`comprar:${recurso}`, 'comprarEnCampamento', { recurso, cantidad: pide - tiene }, 60 * 60_000);
      tiene += compra?.ok && compra.datos ? compra.datos.cantidad : 0;
    }
    if (tiene > 0) await ctx.intentar(`aportar:${recurso}`, 'aportarARefundacion', { recurso, cantidad: Math.min(pide, tiene), lado: 'almacen' }, 10 * 60_000);
  }
  return false;
}

// --- Fuera ---

async function enCampo(ctx: ContextoBot, casa: CampamentoMercenarios): Promise<void> {
  const { vista, yo, memoria } = ctx;
  const columna = columnaPropia(vista, yo)!;
  const enLaPuerta = distancia(columna.posicionActual, casa.posicion) <= MOVIMIENTO.radioPuerta;
  const plan = memoria.plan;
  if ((plan?.tipo === 'anillo' || plan?.tipo === 'unirse') && !enLaPuerta) plan.salio = true;
  apuntarBandidos(ctx, columna.posicionActual);
  await abrirAlijoCercano(ctx, columna.posicionActual);

  // Un compañero ha comprado la caravana: vuelve para ir a fundar con él (M2).
  const caravanaEnCasa = vista.caravanas.some(
    (c) => c.tipo === 'construccion' && c.faccionId === vista.faccionId && c.titularId !== yo && distancia(c.posicionActual, casa.posicion) <= MOVIMIENTO.radioPuerta
  );
  if (caravanaEnCasa && columna.tipo === 'personal' && plan?.tipo !== 'fundar') return await volverACasa(ctx, columna, casa, enLaPuerta);

  if (columna.liderId !== yo) {
    // Va donde va su líder, que es quien repliega el ejército. De vuelta en la puerta, se separa para entrar (se entra solo).
    // A fundar va hasta el final: es cofundador. Si la fundación no salió y el ejército volvió a la puerta, también se separa.
    if (plan?.tipo !== 'unirse' || columna.estado !== 'estacionado' || !plan.salio || !enLaPuerta) return;
    if (!(await ctx.actuar('separarseDelEjercito', { heroeId: yo })).ok) return;
    if (enLaPuerta) await ctx.actuar('entrarEnCampamento', { campamentoId: casa.id, heroeId: yo });
    memoria.plan = { tipo: 'anillo', campamentoId: casa.id, salio: true };
    return;
  }
  if (plan?.tipo === 'fundar') return await conducirCaravana(ctx, casa.posicion, columna, plan);

  const escuadras = escuadrasDeLaColumna(ctx, columna.escuadronIds, columna.participantes.map((p) => p.heroeId));
  const poder = poderTotal(escuadras, false);
  const presa = presaAlAlcance(ctx, columna.posicionActual, poder);
  if (presa && distancia(presa.posicion, columna.posicionActual) <= LOGISTICA.radioEncuentro) {
    await ctx.actuar('atacar', { heroeId: yo, objetivo: { tipo: 'campamento', id: presa.id } });
    return;
  }

  // Un ejército lleva el rumbo fijo (Doc 5.12.1): llega, ataca y, sin más que hacer allí, se repliega cuando se van los demás.
  if (plan?.tipo === 'anillo' && plan.explorar) return await explorando(ctx, columna, casa, enLaPuerta, plan.explorar);
  // El rumbo no se cambia: si ya atacó lo que había (o no puede) y sigue parado, o se queda sin tropa, se repliega entero; en la puerta,
  // los demás se separan solos y el último entra.
  if (columna.tipo === 'ejercito') {
    if (enLaPuerta) {
      if (columna.estado === 'estacionado' && plan?.tipo === 'anillo' && plan.salio && columna.participantes.length === 1) {
        await ctx.actuar('entrarEnCampamento', { campamentoId: casa.id, heroeId: yo });
      }
    } else if (columna.estado === 'estacionado' || (columna.estado === 'marchando' && sinFuerza(escuadras))) {
      await ctx.intentar(`replegar:${columna.id}`, 'replegarEjercito', { ejercitoId: columna.id }, 10 * 60_000);
    }
    return;
  }

  if (debeVolver(ctx, columna.posicionActual, vista.heroe?.viveres ?? 0, escuadras, casa)) {
    delete memoria.plan;
    return await volverACasa(ctx, columna, casa, enLaPuerta);
  }
  // Con tropa va derecho a lo que sabe; sin presa a la vista, vuelve (no le da la ración para recorrer el anillo).
  if (escuadras.length > 0 && !presa && columna.estado === 'estacionado') return await volverACasa(ctx, columna, casa, enLaPuerta);

  // Lo que ve —un bandido que puede vencer, un alijo— le desvía aunque vaya de camino; si no ve nada, recorre el anillo.
  const alijo = [...vista.alijos].sort((a, b) => distancia(a.posicion, columna.posicionActual) - distancia(b.posicion, columna.posicionActual))[0];
  const visto = presa?.posicion ?? alijo?.posicion;
  const yaVa = columna.objetivo.tipo === 'punto' && visto !== undefined && distancia(columna.objetivo.punto, visto) < 1;
  if (columna.estado !== 'estacionado' && (visto === undefined || yaVa)) return;
  memoria.plan = plan?.tipo === 'anillo' ? plan : { tipo: 'anillo', campamentoId: casa.id };
  await ctx.actuar('marcharA', { heroeId: yo, objetivo: { tipo: 'punto', punto: visto ?? puntoDelAnillo(ctx, casa.posicion) } });
}

/** A casa: en la puerta, entra; si no, marcha a ella (si no va ya). */
async function volverACasa(ctx: ContextoBot, columna: Ejercito, casa: CampamentoMercenarios, enLaPuerta: boolean): Promise<void> {
  if (enLaPuerta) {
    if (columna.estado === 'estacionado') await ctx.actuar('entrarEnCampamento', { campamentoId: casa.id, heroeId: ctx.yo });
    return;
  }
  if (columna.tipo === 'ejercito') {
    if (columna.estado !== 'regresando') await ctx.intentar(`replegar:${columna.id}`, 'replegarEjercito', { ejercitoId: columna.id }, 10 * 60_000);
    return;
  }
  if (columna.estado === 'estacionado' || columna.objetivo.tipo !== 'punto' || distancia(columna.objetivo.punto, casa.posicion) > MOVIMIENTO.radioPuerta) {
    await ctx.intentar(`volver:${casa.id}`, 'marcharA', { heroeId: ctx.yo, objetivo: { tipo: 'punto', punto: casa.posicion } }, 5 * 60_000);
  }
}

/** El bandido visible más cercano con menos poder que `poder`. */
function presaAlAlcance(ctx: ContextoBot, donde: Point, poder: number) {
  return ctx.vista.campamentosBandidos
    .filter((b) => b.poder < poder)
    .sort((a, b) => distancia(a.posicion, donde) - distancia(b.posicion, donde) || (a.id < b.id ? -1 : 1))[0];
}

/**
 * Lo que ve de los bandidos va a la pizarra (los campamentos de bandidos no tienen memoria): así quien está dentro sabe a qué
 * salir. Lo que estaba apuntado a la vista y ya no está, se borra.
 */
function apuntarBandidos(ctx: ContextoBot, donde: Point): void {
  const { vista, pizarra } = ctx;
  const vistos = new Set(vista.campamentosBandidos.map((b) => b.id));
  for (const [id, b] of pizarra.bandidos) {
    if ((!vistos.has(id) && distancia(b.posicion, donde) <= VISION.jugadorSolo) || vista.instante - b.vistoEn > VIGENCIA_BANDIDO_MS) pizarra.bandidos.delete(id);
  }
  for (const b of vista.campamentosBandidos) pizarra.bandidos.set(b.id, { posicion: b.posicion, poder: b.poder, vistoEn: vista.instante });
}

async function abrirAlijoCercano(ctx: ContextoBot, donde: Point): Promise<void> {
  for (const alijo of ctx.vista.alijos) {
    if (distancia(alijo.posicion, donde) <= MOVIMIENTO.radioPuerta) await ctx.intentar(`alijo:${alijo.id}`, 'abrirAlijo', { alijoId: alijo.id });
  }
}

/** Sus escuadras en la columna y, de los compañeros, las que se les ven llevar (Doc 5.16.7). */
function escuadrasDeLaColumna(ctx: ContextoBot, escuadronIds: readonly string[], participantes: readonly string[]): Escuadron[] {
  const propias = ctx.vista.heroe!.escuadrones.filter((e) => escuadronIds.includes(e.id));
  const ajenas = ctx.vista.heroesVisibles
    .filter((h) => h.heroeId !== ctx.yo && participantes.includes(h.heroeId))
    .flatMap((h) => h.escuadrasQueLleva.map((e) => ({ ...e }) as unknown as Escuadron));
  return [...propias, ...ajenas];
}

/** Sin un soldado, o con menos de la mitad de los que salieron: no hay nada que hacer fuera. */
function sinFuerza(escuadras: readonly Escuadron[]): boolean {
  const suyas = escuadras.filter((e) => e.id !== undefined);
  return escuadras.every((e) => e.cantidad === 0) || (suyas.length > 0 && salud(suyas) < SALUD_PARA_SEGUIR);
}

/** Vuelve si el trigo no le da para llegar a casa con margen, o si ha perdido demasiada gente. */
function debeVolver(ctx: ContextoBot, desde: Point, trigo: number, escuadras: readonly Escuadron[], casa: CampamentoMercenarios): boolean {
  // La salud, de las suyas: de las de los compañeros solo se ve cuántos llevan.
  const suyas = escuadras.filter((e) => e.id !== undefined);
  if (suyas.length > 0 && salud(suyas) < SALUD_PARA_SEGUIR) return true;
  if (escuadras.length === 0) return false;
  return trigo < trigoDelViaje(ctx, desde, casa.posicion, escuadras, MINUTOS_DE_MAS) * MARGEN_TRIGO;
}

/**
 * El trigo de un viaje en línea recta con esa tropa (Doc 5.13.1), con lo que cuesta pasar por el terreno que cruza: el mapa es
 * público y el coste de paso también (`costeEnPunto`). ponytail: línea recta muestreada; el camino real rodea el agua.
 */
function trigoDelViaje(ctx: ContextoBot, desde: Point, hasta: Point, escuadras: readonly Escuadron[], minutosDeMas = 0): number {
  const velocidad = Math.min(...escuadras.map((e) => TROPAS_RECLUTABLES.find((t) => t.id === e.tropaId)?.velocidad ?? VELOCIDAD_A_PIE));
  const d = distancia(desde, hasta);
  const muestras = Math.max(1, Math.ceil(d / 20));
  let coste = 0;
  for (let i = 0; i < muestras; i++) {
    const t = (i + 0.5) / muestras;
    coste += ctx.mapa.costeEnPunto({ x: desde.x + (hasta.x - desde.x) * t, y: desde.y + (hasta.y - desde.y) * t });
  }
  const minutos = (d * (coste / muestras)) / velocidad + minutosDeMas;
  return consumoRacionDeColumna(escuadras, 1, LOGISTICA.factorConsumoEnMarcha) * minutos;
}

function salud(escuadras: readonly Escuadron[]): number {
  let cantidad = 0;
  let nominal = 0;
  for (const e of escuadras) {
    cantidad += e.cantidad;
    nominal += tamanoPrestada(e);
  }
  return nominal <= 0 ? 1 : cantidad / nominal;
}

/**
 * El explorador da la vuelta al anillo (D42): `VUELTA_ANILLO` puntos a la mitad del anillo, que con su vista (80) lo cubren
 * entero. Se desvía a los alijos que ve. Al acabar la vuelta, el anillo queda explorado para la pizarra y vuelve; si antes
 * ve presa y no lleva caravana que guardar, vuelve a por la tropa.
 */
async function explorando(ctx: ContextoBot, columna: Ejercito, casa: CampamentoMercenarios, enLaPuerta: boolean, explorar: { desde: Instante; paso: number; giro: number }): Promise<void> {
  const { vista, yo, pizarra } = ctx;
  const residentes = casa.residentesIds.filter((id) => vista.facciones.find((f) => f.id === vista.faccionId)?.ciudadanosIds.includes(id)).length;
  const titular = vista.caravanas.some((c) => c.tipo === 'construccion' && c.titularId === yo);
  const acabada = explorar.paso >= VUELTA_ANILLO || vista.instante - explorar.desde > EXPLORACION_MAXIMA_MS;
  if (acabada || (!titular && presaConocida(ctx, casa.posicion, residentes))) {
    pizarra.encargos.delete(`explorar:${casa.id}`);
    if (acabada) pizarra.explorados.set(casa.id, vista.instante);
    return await volverACasa(ctx, columna, casa, enLaPuerta);
  }
  const alijo = [...vista.alijos].sort((a, b) => distancia(a.posicion, columna.posicionActual) - distancia(b.posicion, columna.posicionActual))[0];
  const yaVa = columna.objetivo.tipo === 'punto' && alijo !== undefined && distancia(columna.objetivo.punto, alijo.posicion) < 1;
  if (alijo && !yaVa) {
    await ctx.actuar('marcharA', { heroeId: yo, objetivo: { tipo: 'punto', punto: alijo.posicion } });
    return;
  }
  if (columna.estado !== 'estacionado') return;
  const radio = (CAMPAMENTOS_BANDIDOS.anillo.radioMin + CAMPAMENTOS_BANDIDOS.anillo.radioMax) / 2;
  while (explorar.paso < VUELTA_ANILLO) {
    const a = explorar.giro + (explorar.paso++ * 2 * Math.PI) / VUELTA_ANILLO;
    const p = { x: Math.round(casa.posicion.x + radio * Math.cos(a)), y: Math.round(casa.posicion.y + radio * Math.sin(a)) };
    if (!ctx.mapa.dentroDelMapa(p) || ctx.mapa.terrenoEn(p) === 'agua') continue;
    if ((await ctx.actuar('marcharA', { heroeId: yo, objetivo: { tipo: 'punto', punto: p } })).ok) return;
  }
}

/** Un punto al azar del anillo de bandidos de su campamento (D42), en tierra y dentro del mapa: a recorrer hasta ver uno. */
function puntoDelAnillo(ctx: ContextoBot, centro: Point): Point {
  const { radioMin, radioMax } = CAMPAMENTOS_BANDIDOS.anillo;
  for (let intento = 0; intento < 12; intento++) {
    const angulo = ctx.rng() * 2 * Math.PI;
    const radio = radioMin + ctx.rng() * (radioMax - radioMin);
    const p = { x: Math.round(centro.x + radio * Math.cos(angulo)), y: Math.round(centro.y + radio * Math.sin(angulo)) };
    if (ctx.mapa.dentroDelMapa(p) && ctx.mapa.terrenoEn(p) !== 'agua') return p;
  }
  return centro;
}
