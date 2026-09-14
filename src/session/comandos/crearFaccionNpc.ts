import type { Point } from '../../domain/types';
import { crearFaccion as crearFaccionEngine } from '../../engine/faccion';
import { asignarRey } from '../../engine/cargos';
import { buscarPosicionFundacionInicialPorDefecto, fundarAsentamientosIniciales, HEROES_POR_FUNDACION_NPC } from '../npcGobernanza';
import { exito } from './tipos';
import { comando, rechazar } from './ayudas';
import { CODIGOS_ERROR } from './codigosDeError';
import { evento } from './eventos';

export interface ParamsCrearFaccionNpc {
  nombre: string;
  /** Dónde se funda su primer asentamiento. Sin ella, donde la gobernanza NPC ve el mejor sitio. */
  posicion?: Point;
}

/**
 * Crea una Facción NPC YA ASENTADA (comando de admin; Doc 1.3: "el servidor arranca con Facciones NPC ya
 * asentadas"). Nace gobernada por el NPC y así sigue hasta que se destruya: ninguna Facción de jugador pasa a
 * la IA. Funda en el acto su primer asentamiento con sus héroes bot, y el primero queda como Rey: toda Facción
 * tiene Rey.
 */
export const crearFaccionNpc = comando<ParamsCrearFaccionNpc, { faccionId: string; asentamientoId: string }>((estado, mapa, ctx, params) => {
  const nombre = params.nombre.trim();
  if (!nombre) rechazar(CODIGOS_ERROR.faccionNombreVacio);
  if (estado.facciones.some((f) => f.nombre.toLowerCase() === nombre.toLowerCase())) rechazar(CODIGOS_ERROR.faccionNombreDuplicado);

  const faccion = crearFaccionEngine(`faccion-npc-${ctx.ids.siguiente()}`, nombre);
  const { posicion } = params;
  const fundada = fundarAsentamientosIniciales(
    estado.asentamientos,
    [...estado.facciones, faccion],
    estado.heroes,
    [faccion.id],
    mapa,
    ctx.instante,
    HEROES_POR_FUNDACION_NPC,
    posicion ? () => posicion : buscarPosicionFundacionInicialPorDefecto
  );
  const asentamiento = fundada.asentamientos.find((a) => a.faccionId === faccion.id);
  if (!asentamiento) rechazar(CODIGOS_ERROR.fundacionInvalida);
  const rey = asentamiento.heroesFundadoresIds[0]!;

  return exito(
    {
      ...estado,
      asentamientos: fundada.asentamientos,
      facciones: fundada.facciones.map((f) => (f.id === faccion.id ? asignarRey(f, rey) : f)),
      heroes: fundada.heroes,
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
