// ANDAMIO hasta el paso 4 de Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md (§9): crea una Facción de bots ya asentada
// para que el batch tenga a quién mover mientras los bots-héroe no lleguen por los campamentos. Se borra con D53/D58,
// junto con el héroe bot que crea (crear héroes es un poder que ningún jugador tiene, §7).
import type { Heroe, Point, RecursoTipo, UbicacionHeroe } from '../../domain/types';
import { LIDERAZGO } from '../../constants';
import { crearFaccion as crearFaccionEngine } from '../../engine/faccion';
import { asignarRey } from '../../engine/cargos';
import { progresionInicial } from '../../engine/heroe';
import { evaluarViabilidadFundacion, fundarAsentamiento } from '../../engine/settlement';
import { situarHeroes } from '../../engine/ubicacion';
import type { Mapa } from '../../world/mapa';
import type { Asentamiento } from '../../domain/types';
import { exito } from './tipos';
import { comando, rechazar } from './ayudas';
import { CODIGOS_ERROR } from './codigosDeError';
import { evento } from './eventos';

export interface ParamsCrearFaccionNpc {
  nombre: string;
  /** Dónde se funda su primer asentamiento. Sin ella, el mejor sitio del mapa (`mejorSitioInicial`). */
  posicion?: Point;
}

/** Héroes bot con los que se funda. */
const HEROES_POR_FUNDACION = 5;
/** Minerales que desempatan el sitio inicial: cada uno presente en el radio suma un punto. */
const MINERALES_BONUS: RecursoTipo[] = ['cobre', 'estano', 'oro', 'livestock'];
/** Paso del barrido: por debajo del radio inicial (30), para no saltarse grupos de recursos entre dos puntos. */
const PASO_BUSQUEDA = 25;

/** Un héroe bot (Doc 5.15.6): lo maneja el runner de bots, sin jugador detrás. */
export function heroeBot(id: string, displayName: string, ubicacion: UbicacionHeroe): Heroe {
  return {
    id,
    jugadorId: null,
    controlador: 'bot',
    displayName,
    classDefinitionId: 'Spear',
    genero: 'masculino',
    avatar: { cabezaId: '', peloId: '', barbaId: '', cejasId: '' },
    liderazgoBase: LIDERAZGO.base,
    ubicacion,
    escuadrones: [],
    ...progresionInicial(id),
  };
}

/** El punto del mapa con madera y piedra (`recomendable`) y más minerales de bonus en el radio. */
function mejorSitioInicial(mapa: Mapa, asentamientos: Asentamiento[]): Point | undefined {
  let mejor: { posicion: Point; bonus: number } | undefined;
  for (let x = PASO_BUSQUEDA; x < mapa.limites.ancho; x += PASO_BUSQUEDA) {
    for (let y = PASO_BUSQUEDA; y < mapa.limites.alto; y += PASO_BUSQUEDA) {
      const viabilidad = evaluarViabilidadFundacion(mapa, { x, y }, asentamientos);
      if (!viabilidad.recomendable) continue;
      const bonus = MINERALES_BONUS.filter((tipo) => viabilidad.recursosEnRadio.some((r) => r.tipo === tipo && r.nodos > 0)).length;
      if (mejor && bonus <= mejor.bonus) continue;
      mejor = { posicion: { x, y }, bonus };
      if (bonus === MINERALES_BONUS.length) return mejor.posicion;
    }
  }
  return mejor?.posicion;
}

/**
 * Crea una Facción de bots y funda en el acto su primer asentamiento con cinco héroes bot; el primero queda como Rey.
 * Comando de admin.
 */
export const crearFaccionNpc = comando<ParamsCrearFaccionNpc, { faccionId: string; asentamientoId: string }>((estado, mapa, ctx, params) => {
  const nombre = params.nombre.trim();
  if (!nombre) rechazar(CODIGOS_ERROR.faccionNombreVacio);
  if (estado.facciones.some((f) => f.nombre.toLowerCase() === nombre.toLowerCase())) rechazar(CODIGOS_ERROR.faccionNombreDuplicado);

  const faccion = crearFaccionEngine(`faccion-npc-${ctx.ids.siguiente()}`, nombre);
  const posicion = params.posicion ?? mejorSitioInicial(mapa, estado.asentamientos);
  if (!posicion) rechazar(CODIGOS_ERROR.fundacionInvalida);
  const heroesIds = Array.from({ length: HEROES_POR_FUNDACION }, (_, i) => `heroe-${faccion.id}-${i + 1}`);
  const fundada = fundarAsentamiento(mapa, [...estado.facciones, faccion], faccion.id, posicion, heroesIds, estado.asentamientos, ctx.instante);
  const asentamiento = fundada.asentamiento;
  const ubicacion = { tipo: 'asentamiento', asentamientoId: asentamiento.id } as const;
  const heroes = situarHeroes([...estado.heroes, ...heroesIds.map((id, i) => heroeBot(id, `${nombre} ${i + 1}`, ubicacion))], heroesIds, ubicacion);

  return exito(
    {
      ...estado,
      asentamientos: [...estado.asentamientos, asentamiento],
      facciones: fundada.facciones.map((f) => (f.id === faccion.id ? asignarRey(f, heroesIds[0]!) : f)),
      heroes,
      faccionesNpcIds: [...estado.faccionesNpcIds, faccion.id],
    },
    [
      evento(ctx, {
        codigo: 'faccion.npc_creada',
        mensaje: `Se crea la Facción NPC "${nombre}", asentada en ${asentamiento.id}.`,
        payload: { faccionId: faccion.id, asentamientoId: asentamiento.id },
        asentamientoId: asentamiento.id,
      }),
    ],
    { faccionId: faccion.id, asentamientoId: asentamiento.id }
  );
});
