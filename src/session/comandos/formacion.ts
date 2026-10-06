// Formar un ejército en campo (Doc 5.14.4): organizarlo y cancelarlo. Unirse, separarse y fijar el destino son los
// comandos de siempre (`columna.ts`, `presencia.ts`), que ya saben de formaciones.
import { MovilizacionInvalidaError } from '../../engine/ejercitos';
import { deshacerFormaciones, organizarEjercito as organizarEngine, type PayloadFormacion } from '../../engine/formacion';
import { conHistorialDeJugador } from '../estado';
import { exito } from './tipos';
import { comando, exigirColumnaDe } from './ayudas';
import { evento } from './eventos';

export interface ParamsOrganizarEjercito {
  heroeId: string;
  /** Quién puede unirse en campo (Doc 5.14.1). `rechazar` no tiene sentido en una formación. */
  politicaDeUnion: 'aceptar' | 'preguntar';
}

export interface ParamsCancelarFormacion {
  heroeId: string;
}

/** Una Columna personal que va sola se queda quieta y abre una formación, de la que es Líder. */
export const organizarEjercito = comando<ParamsOrganizarEjercito, { ejercitoId: string }>((estado, _mapa, ctx, params) => {
  const columna = exigirColumnaDe(estado, params.heroeId);
  const formacion = organizarEngine(columna, params.politicaDeUnion, ctx.instante);
  return exito(
    conHistorialDeJugador({ ...estado, ejercitos: estado.ejercitos.map((e) => (e.id === formacion.id ? formacion : e)) }, params.heroeId, 'Organiza un ejército en campo.'),
    [
      evento(ctx, {
        codigo: 'columna.formacion_iniciada',
        mensaje: `Un héroe organiza un ejército en campo (${formacion.id}).`,
        payload: { ejercitoId: formacion.id, liderId: params.heroeId } satisfies PayloadFormacion,
        asentamientoId: formacion.origenAsentamientoId,
      }),
    ],
    { ejercitoId: formacion.id }
  );
});

/** El Líder deshace la formación: cada uno vuelve a su Columna personal con lo que aportó. */
export const cancelarFormacion = comando<ParamsCancelarFormacion, void>((estado, _mapa, ctx, params) => {
  const formacion = exigirColumnaDe(estado, params.heroeId);
  if (!formacion.formacion) throw new MovilizacionInvalidaError('No estás en una formación.');
  if (formacion.liderId !== params.heroeId) throw new MovilizacionInvalidaError('Solo el Líder cancela la formación: los demás se separan.');
  const deshecha = deshacerFormaciones(estado.ejercitos, estado.heroes, new Set([formacion.id]));
  return exito(
    conHistorialDeJugador({ ...estado, ...deshecha }, params.heroeId, 'Cancela la formación del ejército.'),
    [
      evento(ctx, {
        codigo: 'columna.formacion_disuelta',
        mensaje: `El Líder cancela la formación ${formacion.id}.`,
        payload: { ejercitoId: formacion.id, liderId: params.heroeId } satisfies PayloadFormacion,
        asentamientoId: formacion.origenAsentamientoId,
      }),
    ]
  );
});
