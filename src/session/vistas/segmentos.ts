// Canales de vista (`Docs/Coordinacion/03_Tiempo_real_por_segmentos.md` §2): qué foto tiene cada canal, para quién, y cuándo siguen
// siendo del mismo grupo dos suscriptores. Es NEGOCIO (qué ve quién), como `session/canales.ts`; el transporte vive en
// `server/difusion/difusorDeVistas.ts`, que solo sabe pedir fotos y compararlas.
//
// Hoy: los canales de administración (Fase 1 del contrato). Los de jugador (`vista/mapa`, `vista/heroe`, `vista/asentamiento/<id>`) entran
// en las fases 2 y 3 por este mismo módulo.
import type { Asentamiento, CampamentoMercenarios } from '../../domain/types';
import { MANTENIMIENTO } from '../../constants';
import { estaOcupado } from '../../engine/asentamientoQuery';
import { esResidente } from '../../engine/pertenencia';
import { poderEscuadron } from '../../engine/tropa';
import { instanteDeTick, vistaAdminDeEstado, type EventoLogAdmin, type GameSessionState } from '../estado';
import { clavePorId, type EsquemaDeVista, type Foto } from './diff';
import type { ColeccionesMoviles } from './movimiento';

export type TipoDeCanal = 'admin/partida' | 'admin/asentamiento' | 'admin/campamento' | 'admin/heroe';

export interface CanalDeVista {
  nombre: string;
  tipo: TipoDeCanal;
  /** El id de lo que se mira (plaza, campamento, héroe); ausente en `admin/partida`. */
  id?: string;
}

export interface DefinicionDeCanal {
  esquema: EsquemaDeVista;
  moviles?: ColeccionesMoviles;
}

/** Quién pide: el administrador de la partida. (Los jugadores se añaden con las vistas de jugador.) */
export type Audiencia = { tipo: 'admin' };

const CANALES_CON_ID: readonly TipoDeCanal[] = ['admin/asentamiento', 'admin/campamento', 'admin/heroe'];

/** El canal que nombra el cliente, o `undefined` si el servidor no lo implementa o no existe para esa audiencia (`canal_desconocido`). Un
 * canal `admin/*` no existe para un jugador: no se revela ni su nombre (03 §7.1.6). */
export function analizarCanal(nombre: string, audiencia: Audiencia): CanalDeVista | undefined {
  if (audiencia.tipo !== 'admin') return undefined;
  if (nombre === 'admin/partida') return { nombre, tipo: 'admin/partida' };
  for (const tipo of CANALES_CON_ID) {
    if (nombre.startsWith(`${tipo}/`) && nombre.length > tipo.length + 1) return { nombre, tipo, id: nombre.slice(tipo.length + 1) };
  }
  return undefined;
}

/** `instante` y `version` van en la cabecera de cada mensaje (03 §3.2), no en `datos` ni en `escalares`. */
const CABECERA = ['instante', 'version', 'tick'] as const;

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
        titulos: ((t: { tituloId: string }) => t.tituloId) as never,
      },
      escalares: ['mapaId', 'estadoMapa'],
      cabecera: CABECERA,
    },
    moviles: { propios: ['ejercitos', 'caravanas'], ajenos: [] },
  },
  'admin/asentamiento': { esquema: { colecciones: { edificios: clavePorId }, unicas: ['asentamiento'], escalares: [], cabecera: CABECERA } },
  'admin/campamento': { esquema: { colecciones: {}, unicas: ['campamento'], escalares: [], cabecera: CABECERA } },
  'admin/heroe': { esquema: { colecciones: { historial: clavePorId }, unicas: ['heroe'], escalares: [], cabecera: CABECERA } },
};

export function definicionDe(canal: CanalDeVista): DefinicionDeCanal {
  return ESQUEMAS[canal.tipo];
}

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
function fotoAdminPartida(estado: GameSessionState): Foto {
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
  };
}

/** Un id estable para una línea del historial, que no trae uno: el momento, un resumen del texto y cuántas iguales hay antes. */
function idDeLinea(l: EventoLogAdmin, repetidas: Map<string, number>): string {
  let h = 5381;
  for (let i = 0; i < l.mensaje.length; i++) h = (h * 33) ^ l.mensaje.charCodeAt(i);
  const base = `${l.momento}#${(h >>> 0).toString(36)}`;
  const n = repetidas.get(base) ?? 0;
  repetidas.set(base, n + 1);
  return `${base}.${n}`;
}

/** La foto de un canal para el estado dado, o `undefined` si aquello que se mira ya no existe (el difusor lo trata como revocado). */
export function proyectarCanal(estado: GameSessionState, canal: CanalDeVista): Foto | undefined {
  switch (canal.tipo) {
    case 'admin/partida':
      return fotoAdminPartida(estado);
    case 'admin/asentamiento': {
      const a = estado.asentamientos.find((x) => x.id === canal.id);
      if (!a) return undefined;
      const { edificios, ...resto } = a;
      return { instante: instanteDeTick(estado.tick), asentamiento: resto, edificios };
    }
    case 'admin/campamento': {
      const c: CampamentoMercenarios | undefined = estado.campamentosMercenarios.find((x) => x.id === canal.id);
      return c ? { instante: instanteDeTick(estado.tick), campamento: c } : undefined;
    }
    case 'admin/heroe': {
      const h = estado.heroes.find((x) => x.id === canal.id);
      if (!h) return undefined;
      const repetidas = new Map<string, number>();
      const historial = (estado.historialHeroes[h.id] ?? []).map((l) => ({ id: idDeLinea(l, repetidas), ...l }));
      return { instante: instanteDeTick(estado.tick), heroe: { id: h.id, almacenPersonal: h.almacenPersonal ?? {} }, historial };
    }
  }
}

/** Dos suscriptores con la misma clave ven exactamente la misma foto y comparten diff y serialización. Los canales de administración ven
 * todos lo mismo: la clave es el propio canal. */
export function claveDeGrupo(canal: CanalDeVista, _audiencia: Audiencia): string {
  return canal.nombre;
}
