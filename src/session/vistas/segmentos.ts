// Canales de vista (`Docs/Coordinacion/03_Tiempo_real_por_segmentos.md` §2): qué foto tiene cada canal, para quién, y cuándo comparten foto
// dos suscriptores. Es NEGOCIO (qué ve quién), como `session/canales.ts`; el transporte vive en `server/difusion/difusorDeVistas.ts`, que solo
// sabe pedir fotos y compararlas.
//
// Hoy: los canales de administración (fase 1), los de jugador `vista/mapa` y `vista/heroe` (fase 2) y `vista/asentamiento/<id>` (fase 3).
//
// **Los canales de jugador salen de UNA sola proyección por héroe** (`proyectarParaJugador`, memoizada por flush): el reparto en `vista/mapa` y
// `vista/heroe` es solo de qué claves viajan por cada uno, así que la regla de qué se ve sigue escrita una vez y no puede divergir de la que
// sirve el GET. Y la clave del grupo es el HÉROE también en `vista/mapa` (contrato 03 §6.2, caso en que «A degenera en B»): hoy la proyección del
// mapa mezcla entradas de Facción con otras de héroe (`exploracionPersonal`, `plazasRecordadas`, la plaza que se pisa se excluye de los avistados,
// el almacén de una plaza propia si su columna está en la puerta), y compartir foto por Facción filtraría lo de uno a sus compañeros. Para agrupar
// por Facción hay que sacar antes esas entradas del mapa; ver el informe de medición.
import type { Asentamiento, CampamentoMercenarios } from '../../domain/types';
import { MANTENIMIENTO } from '../../constants';
import { estaOcupado } from '../../engine/asentamientoQuery';
import { esCiudadano } from '../../engine/faccion';
import { esResidente } from '../../engine/pertenencia';
import { ubicacionDeducida } from '../../engine/ubicacion';
import { poderEscuadron } from '../../engine/tropa';
import { instanteDeTick, vistaAdminDeEstado, type EventoLogAdmin, type GameSessionState, type GeometriaAsentamientos } from '../estado';
import { perspectivaDeMapa, proyectarMapa, proyectarPersonal, type MapaProyectado, type PerspectivaDeMapa } from '../proyecciones/jugador';
import { clavePorId, type EsquemaDeVista, type Foto } from './diff';
import type { ColeccionesMoviles } from './movimiento';

export type TipoDeCanal = 'admin/partida' | 'admin/asentamiento' | 'admin/campamento' | 'admin/heroe' | 'vista/mapa' | 'vista/heroe' | 'vista/asentamiento';

export interface CanalDeVista {
  nombre: string;
  tipo: TipoDeCanal;
  /** El id de lo que se mira (plaza, campamento, héroe); ausente en los canales sin parámetro. */
  id?: string;
}

export interface DefinicionDeCanal {
  esquema: EsquemaDeVista;
  moviles?: ColeccionesMoviles;
}

/** Quién pide: el administrador de la partida, o un jugador (por su `jugadorId`: su héroe puede no existir todavía al conectar). */
export type Audiencia = { tipo: 'admin' } | { tipo: 'jugador'; jugadorId: string };

/** Lo que solo sabe calcular quien tiene la partida abierta (caches del `RunnerDePartida`), y que la proyección de jugador necesita. */
export interface ImpurosDePartida {
  geometriaAsentamientos(): GeometriaAsentamientos;
  preciosReferencia(): Record<string, number>;
  produccionDeAsentamiento(asentamientoId: string): unknown;
  ascensoDeAsentamiento(asentamientoId: string): unknown;
}

export interface ContextoDeProyeccion {
  impuros: ImpurosDePartida;
  /** Lo ya calculado EN ESTE FLUSH: la perspectiva de cada héroe, el mapa de cada perspectiva y lo personal de cada héroe. */
  memo: Map<string, unknown>;
}

const CANALES_CON_ID_ADMIN: readonly TipoDeCanal[] = ['admin/asentamiento', 'admin/campamento', 'admin/heroe'];
const CANALES_DE_JUGADOR: readonly TipoDeCanal[] = ['vista/mapa', 'vista/heroe'];

/** El canal que nombra el cliente, o `undefined` si el servidor no lo implementa o no existe para esa audiencia (`canal_desconocido`). Un
 * canal `admin/*` no existe para un jugador, ni uno `vista/*` para un administrador: no se revela ni su nombre (03 §7.1.6). */
export function analizarCanal(nombre: string, audiencia: Audiencia): CanalDeVista | undefined {
  if (audiencia.tipo === 'jugador') {
    if (CANALES_DE_JUGADOR.find((t) => t === nombre)) return { nombre, tipo: nombre as TipoDeCanal };
    const prefijo = 'vista/asentamiento/';
    return nombre.startsWith(prefijo) && nombre.length > prefijo.length ? { nombre, tipo: 'vista/asentamiento', id: nombre.slice(prefijo.length) } : undefined;
  }
  if (nombre === 'admin/partida') return { nombre, tipo: 'admin/partida' };
  for (const tipo of CANALES_CON_ID_ADMIN) {
    if (nombre.startsWith(`${tipo}/`) && nombre.length > tipo.length + 1) return { nombre, tipo, id: nombre.slice(tipo.length + 1) };
  }
  return undefined;
}

/** `instante` y `version` van en la cabecera de cada mensaje (03 §3.2), no en `datos` ni en `escalares`. */
const CABECERA = ['instante', 'version', 'tick'] as const;

const porClave = <K extends string>(campo: K) => ((e: Record<K, string>) => e[campo]) as never;

const ESQUEMAS: Record<TipoDeCanal, DefinicionDeCanal> = {
  'admin/partida': {
    esquema: {
      colecciones: {
        asentamientos: clavePorId,
        heroes: clavePorId,
        campamentosMercenarios: clavePorId,
        ejercitos: clavePorId,
        caravanas: clavePorId,
        facciones: clavePorId,
        relaciones: clavePorId,
        batallas: clavePorId,
        campamentosBandidos: clavePorId,
        aedas: clavePorId,
        titulos: porClave('tituloId'),
        acuerdos: clavePorId,
        ordenes: clavePorId,
      },
      escalares: ['mapaId', 'estadoMapa'],
      cabecera: CABECERA,
    },
    moviles: { propios: ['ejercitos', 'caravanas'], ajenos: [] },
  },
  'admin/asentamiento': { esquema: { colecciones: { edificios: clavePorId }, unicas: ['asentamiento'], escalares: [], cabecera: CABECERA } },
  'admin/campamento': { esquema: { colecciones: {}, unicas: ['campamento'], escalares: [], cabecera: CABECERA } },
  'admin/heroe': { esquema: { colecciones: { historial: clavePorId }, unicas: ['heroe'], escalares: [], cabecera: CABECERA } },
  'vista/mapa': {
    esquema: {
      colecciones: {
        ejercitosAvistados: clavePorId,
        caravanasAvistadas: clavePorId,
        asentamientosAvistados: clavePorId,
        asentamientosConocidos: porClave('asentamientoId'),
        aedasAvistados: clavePorId,
        batallas: porClave('battleId'),
        campamentosBandidos: clavePorId,
        campamentosMercenarios: clavePorId,
        heroesVisibles: porClave('heroeId'),
        facciones: clavePorId,
        zonas: porClave('asentamientoId'),
        zonasFusionadas: porClave('faccionId'),
        caminos: clavePorId,
        relaciones: clavePorId,
        titulos: porClave('tituloId'),
      },
      mapas: ['territorioPorEjercito', 'nombresDeCompaneros', 'nombresDeDirigentes'],
      escalares: ['mapaId', 'estadoMapa'],
      cabecera: CABECERA,
    },
    moviles: { propios: [], ajenos: ['ejercitosAvistados', 'caravanasAvistadas'] },
  },
  'vista/asentamiento': { esquema: { colecciones: { edificios: clavePorId }, unicas: ['asentamiento'], escalares: [], cabecera: CABECERA } },
  'vista/heroe': {
    esquema: {
      colecciones: {
        ejercitos: clavePorId,
        caravanas: clavePorId,
        convocatorias: clavePorId,
        informesDeInspeccion: clavePorId,
        miradasIntel: clavePorId,
        informesPlaza: porClave('asentamientoId'),
        propuestasAnexion: clavePorId,
        propuestasRelacion: clavePorId,
        propuestasFusion: clavePorId,
        acuerdos: clavePorId,
        ordenes: clavePorId,
        aedasResidentes: clavePorId,
        historial: clavePorId,
        alijos: clavePorId,
        campamentosPropios: clavePorId,
        heroesVisiblesExtra: porClave('heroeId'),
        batallasPropias: porClave('battleId'),
      },
      mapas: ['detalleDeAvistados', 'ladoPropioPorBatalla'],
      unicas: ['heroe'],
      escalares: ['faccionId', 'miConvocatoriaId', 'plazaPisadaId'],
      cabecera: CABECERA,
    },
    moviles: { propios: ['ejercitos', 'caravanas'], ajenos: [] },
  },
};

export function definicionDe(canal: CanalDeVista): DefinicionDeCanal {
  return ESQUEMAS[canal.tipo];
}

/** El héroe de un jugador en esta partida, si ya lo creó. */
export function heroeDeJugador(estado: GameSessionState, jugadorId: string) {
  return estado.heroes.find((h) => h.jugadorId === jugadorId);
}

/** ¿Puede esta audiencia seguir viendo este canal ahora? Se pregunta al suscribirse y se reevalúa en cada flush (03 §7.2). El permiso de
 * administración o la membresía de jugador los comprueba quien abrió el socket; aquí se comprueba lo que sale del estado. */
export function puedeVer(estado: GameSessionState, canal: CanalDeVista, audiencia: Audiencia): boolean {
  if (audiencia.tipo === 'admin') return true;
  const heroe = heroeDeJugador(estado, audiencia.jugadorId);
  if (!heroe) return false;
  if (canal.tipo !== 'vista/asentamiento') return CANALES_DE_JUGADOR.includes(canal.tipo);
  // El interior de una plaza lo ve quien es de su Facción Y la está pisando: la misma regla que `asentamientos[0]` de la proyección HTTP (la
  // ciudadanía habilita, la presencia ejerce; Doc 1.10.1, 2.5). Salir de la plaza o perder la ciudadanía lo revoca en el siguiente flush.
  const plaza = estado.asentamientos.find((a) => a.id === canal.id);
  const faccion = plaza && estado.facciones.find((f) => f.id === plaza.faccionId);
  if (!plaza || !faccion || !esCiudadano(faccion, heroe.id)) return false;
  const ubicacion = heroe.ubicacion ?? ubicacionDeducida(heroe.id, estado.asentamientos, estado.ejercitos);
  return ubicacion.tipo === 'asentamiento' && ubicacion.asentamientoId === plaza.id;
}

/** Dos suscriptores con la misma clave ven exactamente la misma foto y comparten diff y serialización. Los canales de administración ven
 * todos lo mismo: la clave es el propio canal; también el interior de una plaza, que no depende de quién lo mire (el derecho a verlo se
 * comprueba por suscriptor). Los demás de jugador, el héroe (ver la nota de cabecera). */
export function claveDeGrupo(canal: CanalDeVista, audiencia: Audiencia, estado: GameSessionState, ctx: ContextoDeProyeccion): string {
  if (audiencia.tipo === 'admin' || canal.tipo === 'vista/asentamiento') return canal.nombre;
  const heroe = heroeDeJugador(estado, audiencia.jugadorId);
  if (!heroe) return `${canal.nombre}|`;
  // El mapa lo comparten quienes ven lo mismo (misma Facción y aliados; `PerspectivaDeMapa.clave`); lo personal, nadie.
  return canal.tipo === 'vista/mapa' ? `${canal.nombre}|${perspectivaDe(estado, heroe.id, ctx).clave}` : `${canal.nombre}|${heroe.id}`;
}

// --- Administración ---

/** Lo que cuenta de una plaza en el listado, calculado aquí con las funciones del motor (03 §2.1): así un tick no parchea el almacén de
 * todas las plazas, solo `resumen` cuando cambia algo de esto. */
export interface ResumenDeAsentamiento {
  edificiosActivos: number;
  edificiosEnCurso: number;
  almacenClave: Record<'madera' | 'piedra' | 'trigo' | 'oro', { cantidad: number; capacidad: number }>;
  /** El medidor de Mantenimiento está por debajo de su valor inicial: hay un déficit en curso. */
  mantenimientoIncumplido: boolean;
  ocupada: boolean;
  /** Σ poder de las escuadras que los residentes tienen en el campamento de esta plaza (la tropa en casa, no la que anda en columnas). */
  poderMilitar: number;
  tieneFundicion: boolean;
  tieneGranFundicion: boolean;
}

const RECURSOS_CLAVE = ['madera', 'piedra', 'trigo', 'oro'] as const;

export function resumenDeAsentamiento(estado: GameSessionState, a: Asentamiento): ResumenDeAsentamiento {
  let activos = 0;
  let enCurso = 0;
  let fundicion = false;
  let granFundicion = false;
  for (const e of a.edificios) {
    if (e.estado === 'activo') {
      activos++;
      if (e.tipo === 'fundicion') fundicion = true;
      else if (e.tipo === 'granFundicion') granFundicion = true;
    } else if (e.estado === 'en_construccion' || e.estado === 'en_cola') enCurso++;
  }
  const almacenClave = Object.fromEntries(
    RECURSOS_CLAVE.map((r) => [r, { cantidad: a.almacen[r]?.cantidad ?? 0, capacidad: a.almacen[r]?.capacidad ?? 0 }])
  ) as ResumenDeAsentamiento['almacenClave'];
  let poder = 0;
  for (const h of estado.heroes) {
    if (!esResidente(a, h.id)) continue;
    for (const e of h.escuadrones) if (e.contenedor.tipo === 'campamento' && e.cantidad > 0) poder += poderEscuadron(e);
  }
  return {
    edificiosActivos: activos,
    edificiosEnCurso: enCurso,
    almacenClave,
    mantenimientoIncumplido: a.medidorMantenimiento < MANTENIMIENTO.medidorInicial,
    ocupada: estaOcupado(a, instanteDeTick(estado.tick)),
    poderMilitar: poder,
    tieneFundicion: fundicion,
    tieneGranFundicion: granFundicion,
  };
}

/** La versión ligera de la partida completa (03 §2.1): todo sin niebla, sin lo pesado de cada plaza, héroe y campamento. */
function fotoAdminPartida(estado: GameSessionState, ctx: ContextoDeProyeccion): Foto {
  const v = vistaAdminDeEstado(estado);
  return {
    instante: v.instante,
    version: estado.version,
    tick: estado.tick,
    mapaId: v.mapaId,
    estadoMapa: estado.estadoMapa,
    asentamientos: estado.asentamientos.map((a) => {
      const { edificios, almacen: _almacen, recintos: _recintos, ...resto } = a;
      return { ...resto, edificios: edificios.filter((e) => e.ambito === 'mapa'), resumen: resumenDeAsentamiento(estado, a) };
    }),
    heroes: estado.heroes,
    campamentosMercenarios: estado.campamentosMercenarios.map(({ mercado: _mercado, fondos: _fondos, ...resto }) => resto),
    ejercitos: estado.ejercitos,
    caravanas: estado.caravanas,
    facciones: estado.facciones,
    relaciones: estado.relaciones,
    batallas: v.batallas,
    campamentosBandidos: estado.campamentosBandidos,
    aedas: estado.aedas ?? [],
    titulos: estado.titulos,
    tecnologia: estado.tecnologia,
    red: estado.red ?? { aristas: [], rutas: [] },
    acuerdos: estado.acuerdos,
    ordenes: estado.ordenes,
    aedasResidentes: estado.aedasResidentes ?? { aedas: [] },
    preciosReferencia: ctx.impuros.preciosReferencia(),
  };
}

/** Un id estable para una línea del historial, que no trae uno: el momento, un resumen del texto y cuántas iguales hay antes. */
function conIds(lineas: readonly EventoLogAdmin[]): (EventoLogAdmin & { id: string })[] {
  const repetidas = new Map<string, number>();
  return lineas.map((l) => {
    let h = 5381;
    for (let i = 0; i < l.mensaje.length; i++) h = (h * 33) ^ l.mensaje.charCodeAt(i);
    const base = `${l.momento}#${(h >>> 0).toString(36)}`;
    const n = repetidas.get(base) ?? 0;
    repetidas.set(base, n + 1);
    return { id: `${base}.${n}`, ...l };
  });
}

// --- Jugador ---

const memoizar = <T>(ctx: ContextoDeProyeccion, clave: string, calcular: () => T): T => {
  if (!ctx.memo.has(clave)) ctx.memo.set(clave, calcular());
  return ctx.memo.get(clave) as T;
};

const perspectivaDe = (estado: GameSessionState, heroeId: string, ctx: ContextoDeProyeccion): PerspectivaDeMapa =>
  memoizar(ctx, `perspectiva|${heroeId}`, () => perspectivaDeMapa(estado, heroeId));

/** El mapa de una perspectiva, hecho una vez por flush y compartido por todos los héroes con la misma `clave`. */
const mapaDe = (estado: GameSessionState, perspectiva: PerspectivaDeMapa, ctx: ContextoDeProyeccion): MapaProyectado =>
  memoizar(ctx, `mapa|${perspectiva.clave}`, () => proyectarMapa(estado, perspectiva, ctx.impuros.geometriaAsentamientos()));

/** Lo de un héroe que no es del mapa, con sus ajustes sobre él (03 §13.1) y las piezas que solo conoce el runner (precios, producción, ascenso:
 * estas dos solo de la plaza que se pisa, como en el GET). */
const personalDe = (estado: GameSessionState, heroeId: string, ctx: ContextoDeProyeccion) =>
  memoizar(ctx, `personal|${heroeId}`, () => {
    const perspectiva = perspectivaDe(estado, heroeId, ctx);
    const r = proyectarPersonal(estado, heroeId, perspectiva, mapaDe(estado, perspectiva, ctx), ctx.impuros.geometriaAsentamientos());
    return {
      ...r.personal,
      ...r.ajustes,
      preciosReferencia: ctx.impuros.preciosReferencia(),
      ...(r.dentroDe
        ? { produccionDeAsentamiento: ctx.impuros.produccionDeAsentamiento(r.dentroDe.id), ascensoDeAsentamiento: ctx.impuros.ascensoDeAsentamiento(r.dentroDe.id) }
        : {}),
    } as Record<string, unknown>;
  });

/** La foto de un canal para el estado dado, o `undefined` si aquello que se mira ya no existe (el difusor lo trata como revocado). */
export function proyectarCanal(estado: GameSessionState, canal: CanalDeVista, audiencia: Audiencia, ctx: ContextoDeProyeccion): Foto | undefined {
  const instante = instanteDeTick(estado.tick);
  switch (canal.tipo) {
    case 'admin/partida':
      return fotoAdminPartida(estado, ctx);
    case 'admin/asentamiento': {
      const a = estado.asentamientos.find((x) => x.id === canal.id);
      if (!a) return undefined;
      const { edificios, ...resto } = a;
      return { instante, asentamiento: resto, edificios };
    }
    case 'admin/campamento': {
      const c: CampamentoMercenarios | undefined = estado.campamentosMercenarios.find((x) => x.id === canal.id);
      return c ? { instante, campamento: c } : undefined;
    }
    case 'admin/heroe': {
      const h = estado.heroes.find((x) => x.id === canal.id);
      if (!h) return undefined;
      return { instante, heroe: { id: h.id, almacenPersonal: h.almacenPersonal ?? {} }, historial: conIds(estado.historialHeroes[h.id] ?? []) };
    }
    case 'vista/asentamiento': {
      // Hero-independiente a propósito: el grupo lo comparten todos los que están dentro, y lo proyecta el primero que lo abrió.
      const a = estado.asentamientos.find((x) => x.id === canal.id);
      if (!a) return undefined;
      const { edificios, ...resto } = a;
      return { instante, asentamiento: resto, edificios, trazado: ctx.impuros.geometriaAsentamientos().trazadoPorAsentamiento[a.id] };
    }
    case 'vista/mapa':
    case 'vista/heroe': {
      if (audiencia.tipo !== 'jugador') return undefined;
      const heroe = heroeDeJugador(estado, audiencia.jugadorId);
      if (!heroe) return undefined;
      if (canal.tipo === 'vista/mapa') return { instante, ...mapaDe(estado, perspectivaDe(estado, heroe.id, ctx), ctx) };
      const p = personalDe(estado, heroe.id, ctx);
      const informes = (p['informesDeInspeccion'] as { objetivo: { tipo: string; id: string } }[]).map((i) => ({ id: `${i.objetivo.tipo}:${i.objetivo.id}`, ...i }));
      return { instante, ...p, informesDeInspeccion: informes, historial: conIds(p['historial'] as EventoLogAdmin[]) };
    }
  }
}
