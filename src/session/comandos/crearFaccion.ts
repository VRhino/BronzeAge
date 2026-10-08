import { crearFaccion as crearFaccionEngine, esCiudadano, otorgarCiudadania, sinSolicitudesDe } from '../../engine/faccion';
import { motivoSigiloRechazado, sigiloLibre } from '../../engine/sigilo';
import type { Sigilo } from '../../domain/types';
import { asignarRey } from '../../engine/cargos';
import { CIUDADANIA } from '../../constants';
import { dias, transcurrido } from '../../domain/tiempo';
import { exito } from './tipos';
import { comando, conExploracionFundida, conFaccionEnSuColumna, rechazar } from './ayudas';
import { CODIGOS_ERROR } from './codigosDeError';
import { evento } from './eventos';

export interface PayloadFaccionCreada {
  faccionId: string;
  nombre: string;
  fundadorId: string;
  sigilo: Sigilo;
}

export interface ParamsCrearFaccion {
  nombre: string;
  /** Sigilo de la Facción (Doc 2.8.1): se elige aquí y ya no se cambia. Sin él, se asigna uno libre. */
  sigilo?: Sigilo;
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
 * Cuatro validaciones viven AQUÍ y no en el motor — son reglas de ESTA partida, no del modelo de juego:
 *  - nombre vacío / duplicado (ya existían)
 *  - el sigilo, si lo trae, es del catálogo y no lo lleva ya otra Facción (Doc 2.8.1)
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
  const ocupados = estado.facciones.map((f) => f.sigilo);
  if (params.sigilo) {
    const motivo = motivoSigiloRechazado(params.sigilo, ocupados);
    if (motivo === 'invalido') rechazar(CODIGOS_ERROR.faccionSigiloInvalido);
    if (motivo === 'duplicado') rechazar(CODIGOS_ERROR.faccionSigiloDuplicado);
  }
  if (estado.facciones.some((f) => esCiudadano(f, ctx.actor))) {
    rechazar(CODIGOS_ERROR.faccionYaPerteneces);
  }
  const salida = estado.salidasFaccionPorHeroe[ctx.actor];
  if (salida !== undefined && transcurrido(salida, ctx.instante) < dias(CIUDADANIA.cooldownCreacionFaccionDias)) {
    rechazar(CODIGOS_ERROR.faccionCooldownCreacion);
  }

  const faccionId = `faccion-custom-${ctx.ids.siguiente()}`;
  const nueva = asignarRey(
    otorgarCiudadania(crearFaccionEngine(faccionId, nombre, params.sigilo ?? sigiloLibre(faccionId, ocupados)), ctx.actor),
    ctx.actor
  );
  // Lo que anduvo sin bandera pasa a ser conocimiento de la Facción recién creada (Doc 1.3).
  const siguiente = conFaccionEnSuColumna(conExploracionFundida({ ...estado, facciones: [...sinSolicitudesDe(estado.facciones, [ctx.actor]), nueva] }, ctx.actor, nueva.id), ctx.actor);
  return exito(
    siguiente,
    [
      evento(ctx, {
        codigo: 'faccion.creada',
        mensaje: `Se crea la Facción "${nueva.nombre}", fundada por ${ctx.actor}, que queda como su Rey.`,
        payload: { faccionId: nueva.id, nombre: nueva.nombre, fundadorId: ctx.actor, sigilo: nueva.sigilo } satisfies PayloadFaccionCreada,
      }),
    ],
    { faccionId: nueva.id }
  );
});
