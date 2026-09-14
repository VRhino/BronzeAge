import { crearFaccion as crearFaccionEngine, esCiudadano, otorgarCiudadania } from '../../engine/faccion';
import { asignarRey } from '../../engine/cargos';
import { CIUDADANIA } from '../../constants';
import { dias, transcurrido } from '../../domain/tiempo';
import { exito } from './tipos';
import { comando, conExploracionFundida, rechazar } from './ayudas';
import { CODIGOS_ERROR } from './codigosDeError';
import { evento } from './eventos';

export interface PayloadFaccionCreada {
  faccionId: string;
  nombre: string;
  fundadorId: string;
}

export interface ParamsCrearFaccion {
  nombre: string;
}

/**
 * Crea una Facción nueva y otorga ciudadanía inmediata a quien la crea (a petición del usuario, 2026-08-27:
 * antes nacía sin ciudadanos, y solo se convertía en la Facción de su fundador cuando este fundaba un
 * asentamiento o compraba una casa — un hueco entre "crear" y "pertenecer" que no tenía por qué existir).
 *
 * Quien la crea queda además como su **primer Rey** (a petición del usuario, 2026-09-10): una Facción SIEMPRE
 * tiene Rey. Es el caso base de "Rey automático si la Liga se formó por vasallaje" (Doc 2.2) aplicado a una
 * Facción de un solo miembro — sin votación. La sucesión al abandonar el trono la cubre `quitarCiudadania`
 * (`engine/faccion.ts`): pasa al siguiente ciudadano mientras quede alguno.
 *
 * Tres validaciones viven AQUÍ y no en el motor — son reglas de ESTA partida, no del modelo de juego:
 *  - nombre vacío / duplicado (ya existían)
 *  - el actor no puede ser ya ciudadano de OTRA Facción (Doc 2 "Entidades": 1 jugador, 1 Facción — resuelve la
 *    pregunta que este mismo archivo dejaba abierta hasta ahora)
 *  - si abandonó una Facción hace menos de `CIUDADANIA.cooldownCreacionFaccionDias`, no puede crear otra
 *    todavía (anti-abuso "crear, abandonar, crear"; ver `dejarFaccion.ts`, que es quien estampa
 *    `salidasFaccionPorHeroe`)
 */
export const crearFaccion = comando<ParamsCrearFaccion, { faccionId: string }>((estado, _mapa, ctx, params) => {
  const nombre = params.nombre.trim();
  if (!nombre) rechazar(CODIGOS_ERROR.faccionNombreVacio);
  if (estado.facciones.some((f) => f.nombre.toLowerCase() === nombre.toLowerCase())) {
    rechazar(CODIGOS_ERROR.faccionNombreDuplicado);
  }
  if (estado.facciones.some((f) => esCiudadano(f, ctx.actor))) {
    rechazar(CODIGOS_ERROR.faccionYaPerteneces);
  }
  const salida = estado.salidasFaccionPorHeroe[ctx.actor];
  if (salida !== undefined && transcurrido(salida, ctx.instante) < dias(CIUDADANIA.cooldownCreacionFaccionDias)) {
    rechazar(CODIGOS_ERROR.faccionCooldownCreacion);
  }

  const nueva = asignarRey(
    otorgarCiudadania(crearFaccionEngine(`faccion-custom-${ctx.ids.siguiente()}`, nombre), ctx.actor),
    ctx.actor
  );
  // Lo que anduvo sin bandera pasa a ser conocimiento de la Facción recién creada (Doc 1.3).
  const siguiente = conExploracionFundida({ ...estado, facciones: [...estado.facciones, nueva] }, ctx.actor, nueva.id);
  return exito(
    siguiente,
    [
      evento(ctx, {
        codigo: 'faccion.creada',
        mensaje: `Se crea la Facción "${nueva.nombre}", fundada por ${ctx.actor}, que queda como su Rey.`,
        payload: { faccionId: nueva.id, nombre: nueva.nombre, fundadorId: ctx.actor } satisfies PayloadFaccionCreada,
      }),
    ],
    { faccionId: nueva.id }
  );
});
