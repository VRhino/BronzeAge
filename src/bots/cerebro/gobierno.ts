// El bot con cargo (Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md §10, filas 1-10 y 19-21): lo que hacía la gobernanza
// NPC, ahora hecho por el héroe que tiene el cargo, con sus comandos y con lo que ve. El Rey reparte gobiernos y adopta
// tecnología; el Gobernador construye, amuralla, pide subir de nivel y expande; el Tesorero reserva, comercia y contesta
// trueques. Las cifras son las que la gobernanza tenía calibradas (placeholders hasta medir con batch, §10.1).
import type { Asentamiento, EdificioTipo, RecursoTipo } from '../../domain/types';
import { RECURSOS_TIPO } from '../../domain/types';
import { ANIMAL_CATALOGO, CARRO_CATALOGO, EDIFICIO_CATALOGO, NIVEL_ASENTAMIENTO, TARIFA_ADOPCION, TECNOLOGIAS } from '../../constants';
import { cupoCaravanas, edificiosPorTipoYEstado, hayProyectoPendiente, nivelActualDe, tieneMercadoActivo } from '../../engine/asentamientoQuery';
import { cantidadDisponible, tieneRecursos } from '../../engine/almacen';
import { calcularCostoMantenimiento, calcularNivelAsentamiento } from '../../engine/mantenimiento';
import { costoCaravanaFundacion } from '../../engine/expansion';
import { tarifaDeAscenso } from '../../engine/ascenso';
import { tieneInsumoDeArranque } from '../../engine/construction';
import { esRecomendableParaFundar, fuentesOcupadas } from '../../engine/settlement';
import type { ContextoBot } from '../runner';
import { fraccionDe, plazaDentro, plazasPropias, residentesDe } from './comun';

/** La madera que el Tesorero aparta antes de dejar reclutar o construir: lo que Mantenimiento cobra a nivel 1. */
export const RESERVA_MADERA = 150;
const GRANJAS_MINIMAS = 2;
/** Por debajo de esta fracción de su capacidad, a un recurso no se le considera de sobra (ni para pagar ni para vender). */
const COLCHON = 0.3;
const UMBRAL_EXCEDENTE = 0.7;
const UMBRAL_ESCASEZ = 0.2;
const FRACCION_EXCEDENTE_A_VENDER = 0.25;
/** Cuántos ticks de Mantenimiento por delante hay que tener cubiertos para no pedir ayuda. */
const TICKS_ANTICIPACION = 240;
const RECURSOS_MANTENIMIENTO: RecursoTipo[] = ['madera', 'piedra', 'oro'];
const CARGA_CARAVANA = CARRO_CATALOGO.basico.capacidadBase * ANIMAL_CATALOGO.buey.factorCarga;
/** Héroes que viajan en una caravana de fundación: los que ya existen (§10.1), dejando al menos uno en casa. */
const HEROES_POR_FUNDACION = 2;

export function gobernar(ctx: ContextoBot): void {
  rey(ctx);
  const plaza = plazaDentro(ctx.vista);
  if (!plaza) return;
  if (plaza.cargos.gobernadorId === ctx.yo) gobernador(ctx, plaza);
  if (plaza.cargos.tesoreroId === ctx.yo) tesorero(ctx, plaza);
}

// --- Rey ---

function rey(ctx: ContextoBot): void {
  const { vista, yo, pizarra } = ctx;
  const faccion = vista.facciones.find((f) => f.id === vista.faccionId);
  if (!faccion || faccion.reyId !== yo) return;

  // Un Gobernador para cada plaza que no lo tenga: el residente de id más bajo que la pizarra conozca.
  for (const plaza of plazasPropias(vista)) {
    if (!plaza.cargos || plaza.cargos.gobernadorId) continue;
    const candidato = [...pizarra.residencias].filter(([, p]) => p === plaza.id).map(([h]) => h).sort()[0];
    if (candidato) ctx.intentar(`gobernador:${plaza.id}`, 'asignarCargoLocal', { asentamientoId: plaza.id, cargo: 'gobernador', heroeId: candidato });
  }

  // Adopta lo que le aparece, en orden, mientras la plaza que pisa guarde el doble de la tarifa (la otra mitad es margen
  // para el mantenimiento). ponytail: mira el almacén de la plaza donde está, que es la capital mientras el Rey no salga
  // de ella; si la capital fuera otra, pagaría ella sin que el Rey vea su margen.
  const dentro = plazaDentro(vista);
  const propias = vista.tecnologia.propias;
  if (!dentro || !propias) return;
  for (const id of propias.aparecidas.filter((t) => !propias.adoptadas.includes(t))) {
    const tarifa = TARIFA_ADOPCION[TECNOLOGIAS[id].era];
    const doble = Object.fromEntries(Object.entries(tarifa).map(([r, n]) => [r, 2 * (n ?? 0)]));
    if (!tieneRecursos(dentro.almacen, doble)) return;
    if (!ctx.intentar(`tecnologia:${id}`, 'adoptarTecnologia', { faccionId: faccion.id, tecnologiaId: id })?.ok) return;
  }
}

// --- Gobernador ---

function tieneOEnCurso(plaza: Asentamiento, tipo: EdificioTipo): boolean {
  return edificiosPorTipoYEstado(plaza, tipo).length > 0 || hayProyectoPendiente(plaza, tipo);
}

function gobernador(ctx: ContextoBot, plaza: Asentamiento): void {
  const { vista, yo } = ctx;
  const construir = (tipo: EdificioTipo) =>
    ctx.intentar(`edificio:${plaza.id}:${tipo}`, 'anadirEdificioManualmente', { asentamientoId: plaza.id, cargo: 'gobernador', tipo }, 10 * 60_000);

  if (!plaza.cargos.tesoreroId) ctx.intentar(`tesorero:${plaza.id}`, 'asignarCargoLocal', { asentamientoId: plaza.id, cargo: 'tesorero', heroeId: yo });

  // Comida antes que comercio o guarnición: sin las granjas mínimas no se construye nada más este turno.
  if (plaza.edificios.filter((e) => e.tipo === 'granja').length < GRANJAS_MINIMAS) {
    construir('granja');
    return;
  }
  if (!tieneMercadoActivo(plaza) && !tieneOEnCurso(plaza, 'mercado')) construir('mercado');

  const adoptadas = vista.tecnologia.propias?.adoptadas ?? [];
  const nivel = nivelActualDe(plaza);
  const militar: EdificioTipo | undefined = !tieneOEnCurso(plaza, 'barracon')
    ? 'barracon'
    : !tieneOEnCurso(plaza, 'galeriaDeTiro')
      ? 'galeriaDeTiro'
      : !tieneOEnCurso(plaza, 'caballerizas') && adoptadas.includes('cria_caballar')
        ? 'caballerizas'
        : !tieneOEnCurso(plaza, 'palacio') && nivel >= 2
          ? 'palacio'
          : !tieneOEnCurso(plaza, 'salaConsejo') && nivel >= 3 && adoptadas.includes('instituciones_civicas')
            ? 'salaConsejo'
            : undefined;
  if (militar) construir(militar);

  // El primer recinto, y la mejora a piedra cuando la subida a nivel 4 ya está al alcance (Doc 4.5).
  const recinto = (plaza.recintos ?? [])[0];
  if (!recinto) ctx.intentar(`recinto:${plaza.id}`, 'comprometerRecinto', { asentamientoId: plaza.id, cargo: 'gobernador', nivel: 1 });
  else if (recinto.nivel === 1 && recinto.mejorandoA === undefined && recinto.avance >= recinto.celdas.length - 1 && nivel >= 3 && adoptadas.includes('instituciones_civicas')) {
    ctx.intentar(`recinto-mejora:${plaza.id}`, 'mejorarRecinto', { asentamientoId: plaza.id, cargo: 'gobernador', recintoId: recinto.id });
  }

  // Subir de nivel en cuanto cumple los gates: el motor decide cupo, coste y solvencia.
  if (!plaza.ascenso && calcularNivelAsentamiento(plaza) > plaza.nivel) ctx.intentar(`ascenso:${plaza.id}`, 'solicitarAscenso', { asentamientoId: plaza.id });

  expandir(ctx, plaza);
}

/**
 * Caravana de fundación desde una plaza de nivel 2 con lo que cuesta, hacia el primer punto que le parezca viable con
 * lo que sabe del mundo (la geografía y las plazas que ve). Si el motor no lo acepta, espera y prueba otro.
 */
function expandir(ctx: ContextoBot, plaza: Asentamiento): void {
  const { vista } = ctx;
  if (nivelActualDe(plaza) < 2 || !tieneRecursos(plaza.almacen, costoCaravanaFundacion())) return;
  if (residentesDe(plaza).length <= HEROES_POR_FUNDACION) return;
  // Lo que sabe de los vecinos: las plazas que ve, con su silueta y sus edificios en pie. ponytail: fichas en lugar de
  // plazas completas; si el motor discrepa, el rechazo lo dice y se prueba otro punto.
  const conocidas = [plaza, ...vista.asentamientosAvistados.filter((a) => a.id !== plaza.id)] as unknown as Asentamiento[];
  const ocupadas = fuentesOcupadas(conocidas);
  for (let radio = 150; radio <= 600; radio += 150) {
    for (let angulo = 0; angulo < 360; angulo += 20) {
      const rad = (angulo * Math.PI) / 180;
      const destino = { x: plaza.posicion.x + Math.cos(rad) * radio, y: plaza.posicion.y + Math.sin(rad) * radio };
      const clave = `fundar:${Math.round(destino.x)}:${Math.round(destino.y)}`;
      if ((ctx.memoria.esperas.get(clave) ?? -Infinity) > vista.instante) continue;
      if (!esRecomendableParaFundar(ctx.mapa, destino, conocidas, plaza.faccionId, ocupadas)) continue;
      ctx.intentar(clave, 'lanzarCaravanaFundacion', { origenAsentamientoId: plaza.id, destino, numJugadores: HEROES_POR_FUNDACION }, 24 * 60 * 60_000);
      return;
    }
  }
}

// --- Tesorero ---

function tesorero(ctx: ContextoBot, plaza: Asentamiento): void {
  reservar(ctx, plaza);
  caravanasComerciales(ctx, plaza);
  contestarTrueques(ctx, plaza);
  pedirLoQueFalta(ctx, plaza);
  publicarOrdenes(ctx, plaza);
}

/** La madera de Mantenimiento y, desde nivel 2, lo que cuesta la caravana de fundación. */
function reservar(ctx: ContextoBot, plaza: Asentamiento): void {
  const objetivo: Partial<Record<string, number>> = { madera: RESERVA_MADERA };
  if (nivelActualDe(plaza) >= 2) for (const [r, n] of Object.entries(costoCaravanaFundacion())) objetivo[r] = Math.max(objetivo[r] ?? 0, n ?? 0);
  for (const [recurso, valor] of Object.entries(objetivo)) {
    if ((plaza.reservaManual?.[recurso as RecursoTipo] ?? 0) >= (valor ?? 0)) continue;
    ctx.intentar(`reserva:${plaza.id}:${recurso}`, 'calibrarReservaManual', { asentamientoId: plaza.id, recurso: recurso as RecursoTipo, valor: valor ?? 0 });
  }
}

/** Todas las caravanas comerciales que le quepan (carro básico con buey), completando antes las que quedaron a medias. */
function caravanasComerciales(ctx: ContextoBot, plaza: Asentamiento): void {
  if (!tieneMercadoActivo(plaza)) return;
  const propias = ctx.vista.caravanas.filter((c) => c.tipo === 'comercial' && c.origenAsentamientoId === plaza.id);
  for (const c of propias) {
    if ((c.carros ?? []).length === 0) ctx.intentar(`carro:${c.id}`, 'agregarCarroCaravana', { caravanaId: c.id, tipoCarro: 'basico' });
    else if (!c.carros![0]!.animal) ctx.intentar(`animal:${c.id}`, 'comprarAnimalCaravana', { caravanaId: c.id, carroIndice: 0, tipoAnimal: 'buey' });
  }
  if (propias.length >= cupoCaravanas(plaza) || propias.some((c) => (c.carros ?? []).length === 0 || !c.carros![0]!.animal)) return;
  const nueva = ctx.intentar(`caravana:${plaza.id}`, 'crearCaravana', { asentamientoId: plaza.id });
  if (!nueva?.ok || !nueva.datos) return;
  if (ctx.actuar('agregarCarroCaravana', { caravanaId: nueva.datos.caravanaId, tipoCarro: 'basico' }).ok) {
    ctx.actuar('comprarAnimalCaravana', { caravanaId: nueva.datos.caravanaId, carroIndice: 0, tipoAnimal: 'buey' });
  }
}

/** Acepta los trueques que le proponen si le sobra lo que tendría que entregar; si no, los rechaza. */
function contestarTrueques(ctx: ContextoBot, plaza: Asentamiento): void {
  for (const acuerdo of ctx.vista.acuerdos) {
    if (acuerdo.estado !== 'propuesto' || acuerdo.asentamientoBId !== plaza.id) continue;
    const leSobra = acuerdo.lineasB.every((l) => fraccionDe(plaza, l.recurso as RecursoTipo) > COLCHON && cantidadDisponible(plaza.almacen, l.recurso) > 0);
    ctx.actuar(leSobra ? 'aceptarTrueque' : 'rechazarTrueque', { acuerdoId: acuerdo.id });
  }
}

const ordenActiva = (ctx: ContextoBot, plaza: Asentamiento, recurso: string) =>
  ctx.vista.ordenes.some((o) => o.estado === 'activa' && o.asentamientoId === plaza.id && o.recurso === recurso);

/**
 * Lo que le falta para sobrevivir (el Mantenimiento de las próximas horas) y para crecer (lo que la subida pide y la plaza
 * no produce): primero por trueque con una plaza que su Facción sabe que lo vende (la pizarra, por el explorador), y si
 * no, con una orden de compra en su propio mercado (§10.1). Nada de leer almacenes ajenos.
 */
function pedirLoQueFalta(ctx: ContextoBot, plaza: Asentamiento): void {
  if (!tieneMercadoActivo(plaza)) return;
  const costo = calcularCostoMantenimiento(plaza, undefined);
  const falta = new Map<RecursoTipo, number>();
  for (const recurso of RECURSOS_MANTENIMIENTO) {
    const necesario = (costo[recurso] ?? 0) * TICKS_ANTICIPACION;
    if (necesario > 0 && (plaza.almacen[recurso]?.cantidad ?? 0) < necesario) falta.set(recurso, Math.min(CARGA_CARAVANA, Math.ceil(necesario)));
  }
  for (const n of necesidadesParaCrecer(plaza)) if (!falta.has(n.recurso)) falta.set(n.recurso, n.cantidad);

  for (const [recurso, cantidad] of falta) {
    if (ordenActiva(ctx, plaza, recurso) || yaPedido(ctx, plaza, recurso)) continue;
    if (truequeConQuienVende(ctx, plaza, recurso, cantidad, [...falta.keys()])) continue;
    ctx.intentar(`compra:${plaza.id}:${recurso}`, 'colocarOrdenMercado', { asentamientoId: plaza.id, tipo: 'compra', recurso, cantidad });
  }
}

const yaPedido = (ctx: ContextoBot, plaza: Asentamiento, recurso: string) =>
  ctx.vista.acuerdos.some(
    (a) => (a.estado === 'propuesto' || a.estado === 'activo') && a.asentamientoAId === plaza.id && a.lineasB.some((l) => l.recurso === recurso)
  );

/** Lo que la siguiente subida pide y no es de Mantenimiento (el bronce), y el insumo de arranque de sus edificios. */
function necesidadesParaCrecer(plaza: Asentamiento): { recurso: RecursoTipo; cantidad: number }[] {
  if (plaza.ascenso) return [];
  const objetivo = plaza.nivel + 1;
  const tarifa = tarifaDeAscenso(objetivo);
  const requisito = NIVEL_ASENTAMIENTO.requisitos[objetivo];
  if (!tarifa || !requisito) return [];
  const necesidades: { recurso: RecursoTipo; cantidad: number }[] = [];
  for (const [recurso, cantidad] of Object.entries(tarifa.costo)) {
    if (RECURSOS_MANTENIMIENTO.includes(recurso as RecursoTipo)) continue;
    const falta = (cantidad ?? 0) - cantidadDisponible(plaza.almacen, recurso);
    if (falta > 0) necesidades.push({ recurso: recurso as RecursoTipo, cantidad: Math.ceil(falta) });
  }
  for (const tipo of requisito.edificios as EdificioTipo[]) {
    if (plaza.edificios.some((e) => e.tipo === tipo) || tieneInsumoDeArranque(plaza, tipo)) continue;
    const niveles = (EDIFICIO_CATALOGO[tipo] as { niveles?: Record<number, { recetas: { consumePorUnidad: Partial<Record<string, number>> }[] }> }).niveles;
    const insumo = Object.keys(niveles?.[1]?.recetas[0]?.consumePorUnidad ?? {})[0];
    if (insumo) necesidades.push({ recurso: insumo as RecursoTipo, cantidad: 30 });
  }
  return necesidades;
}

/** Propone un trueque a una plaza que la pizarra sabe que vende `recurso`, pagando con lo que más le sobra. */
function truequeConQuienVende(ctx: ContextoBot, plaza: Asentamiento, recurso: RecursoTipo, cantidad: number, noPagarCon: RecursoTipo[]): boolean {
  const vendedora = [...ctx.pizarra.mostradores].find(
    ([id, m]) => id !== plaza.id && m.ordenes.some((o) => o.tipo === 'venta' && o.recurso === recurso && o.estado === 'activa')
  )?.[0];
  if (!vendedora) return false;
  const pago = RECURSOS_TIPO.filter((r) => r !== recurso && !noPagarCon.includes(r as RecursoTipo) && fraccionDe(plaza, r as RecursoTipo) > COLCHON).sort(
    (a, b) => fraccionDe(plaza, b as RecursoTipo) - fraccionDe(plaza, a as RecursoTipo)
  )[0];
  if (!pago) return false;
  const sobra = Math.floor((plaza.almacen[pago]?.cantidad ?? 0) - COLCHON * (plaza.almacen[pago]?.capacidad ?? 0));
  const pactada = Math.min(cantidad, sobra);
  if (pactada <= 0) return false;
  return (
    ctx.intentar(`trueque:${plaza.id}:${vendedora}:${recurso}`, 'proponerTrueque', {
      asentamientoAId: plaza.id,
      lineasA: [{ recurso: pago, cantidad: pactada }],
      asentamientoBId: vendedora,
      lineasB: [{ recurso, cantidad: pactada }],
    })?.ok === true
  );
}

/** Vende lo que le sobra y compra lo que le escasea, sin duplicar órdenes. */
function publicarOrdenes(ctx: ContextoBot, plaza: Asentamiento): void {
  if (!tieneMercadoActivo(plaza)) return;
  for (const recurso of RECURSOS_TIPO) {
    const item = plaza.almacen[recurso];
    if (!item || item.capacidad <= 0 || ordenActiva(ctx, plaza, recurso)) continue;
    const fraccion = fraccionDe(plaza, recurso as RecursoTipo);
    if (fraccion >= UMBRAL_EXCEDENTE) {
      const cantidad = Math.floor((item.cantidad - item.capacidad * UMBRAL_EXCEDENTE) * FRACCION_EXCEDENTE_A_VENDER);
      if (cantidad > 0) ctx.intentar(`venta:${plaza.id}:${recurso}`, 'colocarOrdenMercado', { asentamientoId: plaza.id, tipo: 'venta', recurso: recurso as RecursoTipo, cantidad });
    } else if (fraccion <= UMBRAL_ESCASEZ) {
      const hueco = Math.floor(item.capacidad * UMBRAL_ESCASEZ - item.cantidad);
      if (hueco > 0) ctx.intentar(`compra:${plaza.id}:${recurso}`, 'colocarOrdenMercado', { asentamientoId: plaza.id, tipo: 'compra', recurso: recurso as RecursoTipo, cantidad: hueco });
    }
  }
}
