// Formar un ejército en campo (Doc 5.14.4): una Columna personal «organiza» y otras de su Facción se le unen hasta ser tres
// o más. Mientras no llegan, la formación está quieta y para el combate sigue siendo una Columna personal.
import type { Ejercito, Heroe } from '../domain/types';
import type { EventoCrudo } from '../domain/eventos';
import { FORMACION_EJERCITO } from '../constants';
import { minutos, sumar, type Instante } from '../domain/tiempo';
import { desgajar, MovilizacionInvalidaError } from './ejercitos';
import { conEscuadrones, conTropa, indiceTropa, sinTropa, type EjercitoConTropa } from './tropa';

/** Payload de los eventos `columna.formacion_*`. */
export interface PayloadFormacion {
  ejercitoId: string;
  liderId: string;
}

/**
 * Empieza una formación: la columna se queda quieta donde está, y su héroe es el Líder y fija quién puede unirse. Solo
 * una Columna personal que va sola.
 */
export function organizarEjercito(columna: Ejercito, politicaDeUnion: 'aceptar' | 'preguntar', ahora: Instante): Ejercito {
  if (columna.tipo !== 'personal') throw new MovilizacionInvalidaError('Un ejército no organiza otro.');
  if (columna.formacion) throw new MovilizacionInvalidaError('Ya estás organizando un ejército.');
  if (columna.participantes.length !== 1) throw new MovilizacionInvalidaError('Solo se organiza un ejército yendo solo.');
  return {
    ...columna,
    politicaDeUnion,
    estado: 'estacionado',
    objetivo: { tipo: 'punto', punto: columna.posicionActual },
    persiguiendo: undefined,
    formacion: { expiraEn: sumar(ahora, minutos(FORMACION_EJERCITO.plazoMinutos)) },
  };
}

/** Con tres o más, la formación pasa a ser un ejército que espera su destino (Doc 5.14.4). Sin ellos, sigue igual. */
export function completarFormacion<E extends Ejercito>(ejercito: E): E {
  if (!ejercito.formacion || ejercito.participantes.length < FORMACION_EJERCITO.minimo) return ejercito;
  return { ...ejercito, formacion: undefined, tipo: 'ejercito', destinoPendiente: true };
}

/** Un ejército recién formado, sin destino todavía: lo fija su Líder una sola vez (`marcharA`). */
export function esperaDestino(ejercito: Pick<Ejercito, 'destinoPendiente'>): boolean {
  return ejercito.destinoPendiente === true;
}

/** Quién manda tras irse el Líder de una formación: el que lleva más tiempo dentro. */
export function sucesorEnFormacion(ejercito: Ejercito, saleId: string): string {
  const quedan = ejercito.participantes.filter((p) => p.heroeId !== saleId);
  return [...quedan].sort((a, b) => a.unidoEn - b.unidoEn || (a.heroeId < b.heroeId ? -1 : 1))[0]!.heroeId;
}

/**
 * Deshace una formación: el Líder se queda con la columna y cada uno de los demás nace como Columna personal donde
 * estaba, con lo que aportó (`desgajar`). Devuelve las columnas, con su tropa puesta, y el id de quien vuelve a cada una.
 */
export function disolverFormacion(formacion: EjercitoConTropa): EjercitoConTropa[] {
  const lider = formacion.liderId;
  let resto: EjercitoConTropa = formacion;
  const sueltos: EjercitoConTropa[] = [];
  for (const p of formacion.participantes.filter((q) => q.heroeId !== lider)) {
    const separado = desgajar(resto, p.heroeId, `${formacion.id}-${p.heroeId}`);
    resto = separado.ejercito;
    sueltos.push(separado.columna);
  }
  const { formacion: _, ...sinFormacion } = resto;
  return [{ ...sinFormacion, politicaDeUnion: 'rechazar', peticionesDeUnion: undefined }, ...sueltos];
}

/** Deshace las formaciones con esos ids: sus columnas vuelven a ser personales y los héroes se sitúan en la suya. */
export function deshacerFormaciones(
  ejercitos: readonly Ejercito[],
  heroes: readonly Heroe[],
  ids: ReadonlySet<string>
): { ejercitos: Ejercito[]; heroes: Heroe[] } {
  const indice = indiceTropa(heroes);
  const nuevas = ejercitos.filter((e) => ids.has(e.id)).flatMap((f) => disolverFormacion(conTropa(f, indice)).map(sinTropa));
  const ubicados = new Map(nuevas.flatMap(({ ejercito }) => ejercito.participantes.map((p) => [p.heroeId, ejercito.id] as const)));
  return {
    ejercitos: [...ejercitos.filter((e) => !ids.has(e.id)), ...nuevas.map((n) => n.ejercito)],
    heroes: conEscuadrones(heroes, nuevas.flatMap((n) => n.tropa)).map((h) =>
      ubicados.has(h.id) ? { ...h, ubicacion: { tipo: 'columna' as const, ejercitoId: ubicados.get(h.id)! } } : h
    ),
  };
}

/** Las formaciones cuyo plazo venció sin llegar a tres se deshacen en el tick (Doc 5.14.4). */
export function disolverFormacionesVencidas(
  ejercitos: readonly Ejercito[],
  heroes: readonly Heroe[],
  ahora: Instante
): { ejercitos: Ejercito[]; heroes: Heroe[]; eventos: EventoCrudo[] } {
  const vencidas = ejercitos.filter((e) => e.formacion && ahora >= e.formacion.expiraEn);
  if (vencidas.length === 0) return { ejercitos: [...ejercitos], heroes: [...heroes], eventos: [] };
  const eventos = vencidas.map(
    (f): EventoCrudo => ({
      codigo: 'columna.formacion_disuelta',
      mensaje: `La formación ${f.id} no llega a ${FORMACION_EJERCITO.minimo} héroes a tiempo y se deshace.`,
      payload: { ejercitoId: f.id, liderId: f.liderId } satisfies PayloadFormacion,
      asentamientoId: f.origenAsentamientoId,
    })
  );
  return { ...deshacerFormaciones(ejercitos, heroes, new Set(vencidas.map((f) => f.id))), eventos };
}
