// Ejército en preparación dentro de una plaza o un campamento (Doc 5.14.5): reglas puras de quién convoca, quién se une y quién
// se va. Partir (sacar a todos al mundo) lo hace la capa de sesión, que es quien sabe de estados de partida y de salir.
import type { Convocatoria, Heroe, LugarConvocatoria, SeleccionDeSalida } from '../domain/types';
import type { Instante } from '../domain/tiempo';
import { MovilizacionInvalidaError } from './ejercitos';

/** Dónde está dentro este héroe, si está dentro de algo. Una columna en el mapa no es un lugar. */
export function lugarDe(heroe: Heroe): LugarConvocatoria | undefined {
  const u = heroe.ubicacion;
  if (u.tipo === 'asentamiento') return { tipo: 'asentamiento', id: u.asentamientoId };
  if (u.tipo === 'mercenarios') return { tipo: 'campamento', id: u.campamentoId };
  return undefined;
}

export const mismoLugar = (a: LugarConvocatoria | undefined, b: LugarConvocatoria | undefined): boolean => a !== undefined && b !== undefined && a.tipo === b.tipo && a.id === b.id;

/** La convocatoria en la que ya va este héroe, si va en alguna. */
export const convocatoriaDe = (convocatorias: readonly Convocatoria[], heroeId: string): Convocatoria | undefined =>
  convocatorias.find((c) => c.integrantes.some((i) => i.heroeId === heroeId));

/** Los que siguen dentro del lugar: quien se fue por su cuenta (o se desconectó) no sale con los demás. */
export const integrantesDentro = (convocatoria: Convocatoria, heroes: readonly Heroe[]): Convocatoria['integrantes'] =>
  convocatoria.integrantes.filter((i) => {
    const heroe = heroes.find((h) => h.id === i.heroeId);
    return heroe !== undefined && mismoLugar(lugarDe(heroe), convocatoria.lugar);
  });

function exigirDentroYDeLaFaccion(convocatoria: Pick<Convocatoria, 'lugar' | 'faccionId'>, heroe: Heroe, faccionId: string): void {
  if (!mismoLugar(lugarDe(heroe), convocatoria.lugar)) throw new MovilizacionInvalidaError('Solo se une quien está dentro del mismo lugar.');
  if (!faccionId || faccionId !== convocatoria.faccionId) throw new MovilizacionInvalidaError('Un ejército lo componen ciudadanos de una sola Facción.');
}

export function nuevaConvocatoria(
  convocatorias: readonly Convocatoria[],
  heroe: Heroe,
  faccionId: string,
  politicaDeUnion: Convocatoria['politicaDeUnion'],
  seleccion: SeleccionDeSalida,
  id: string,
  ahora: Instante
): Convocatoria {
  const lugar = lugarDe(heroe);
  if (!lugar) throw new MovilizacionInvalidaError('Un ejército se convoca desde dentro de una plaza o de un campamento.');
  if (!faccionId) throw new MovilizacionInvalidaError('Sin Facción no hay compañeros con quienes formar un ejército.');
  if (convocatoriaDe(convocatorias, heroe.id)) throw new MovilizacionInvalidaError('Ya estás en un ejército en preparación.');
  return { id, faccionId, lugar, liderId: heroe.id, politicaDeUnion, integrantes: [{ heroeId: heroe.id, unidoEn: ahora, ...seleccion }], creadaEn: ahora };
}

function exigirPuedeUnirse(convocatorias: readonly Convocatoria[], convocatoria: Convocatoria, heroe: Heroe, faccionId: string): void {
  exigirDentroYDeLaFaccion(convocatoria, heroe, faccionId);
  if (convocatoriaDe(convocatorias, heroe.id)) throw new MovilizacionInvalidaError('Ya estás en un ejército en preparación.');
}

/** Se une ya (política `aceptar`, o el Líder aceptó la petición). */
export function unirseAConvocatoria(convocatorias: readonly Convocatoria[], convocatoria: Convocatoria, heroe: Heroe, faccionId: string, seleccion: SeleccionDeSalida, ahora: Instante): Convocatoria {
  exigirPuedeUnirse(convocatorias, convocatoria, heroe, faccionId);
  return {
    ...convocatoria,
    integrantes: [...convocatoria.integrantes, { heroeId: heroe.id, unidoEn: ahora, ...seleccion }],
    peticiones: convocatoria.peticiones?.filter((p) => p.heroeId !== heroe.id),
  };
}

/** Política `preguntar`: deja la petición al Líder, con lo que sacaría. Vive hasta `expiraEn`; la caducada se barre al anotar otra. */
export function pedirUnion(convocatorias: readonly Convocatoria[], convocatoria: Convocatoria, heroe: Heroe, faccionId: string, seleccion: SeleccionDeSalida, ahora: Instante, expiraEn: Instante): Convocatoria {
  exigirPuedeUnirse(convocatorias, convocatoria, heroe, faccionId);
  const vivas = (convocatoria.peticiones ?? []).filter((p) => ahora < p.expiraEn && p.heroeId !== heroe.id);
  return { ...convocatoria, peticiones: [...vivas, { heroeId: heroe.id, pedidoEn: ahora, expiraEn, ...seleccion }] };
}

/** Saca de la lista la petición que el Líder contesta y la devuelve; falla si no es el Líder, no existe o caducó. */
export function retirarPeticion(convocatoria: Convocatoria, liderId: string, solicitanteId: string, ahora: Instante): { convocatoria: Convocatoria; peticion: NonNullable<Convocatoria['peticiones']>[number] } {
  if (convocatoria.liderId !== liderId) throw new MovilizacionInvalidaError('Solo el Líder contesta las peticiones de unión.');
  const peticion = convocatoria.peticiones?.find((p) => p.heroeId === solicitanteId);
  if (!peticion) throw new MovilizacionInvalidaError('No hay ninguna petición de ese jugador.');
  if (ahora >= peticion.expiraEn) throw new MovilizacionInvalidaError('Esa petición ya caducó: el silencio cuenta como un no.');
  return { convocatoria: { ...convocatoria, peticiones: convocatoria.peticiones!.filter((p) => p !== peticion) }, peticion };
}

/** Cada integrante cambia SOLO su propia selección (el Líder también): el orden de `escuadronIds` es el orden con el que su columna sale y combate. */
export function cambiarSeleccion(convocatoria: Convocatoria, heroeId: string, seleccion: SeleccionDeSalida): Convocatoria {
  if (!convocatoria.integrantes.some((i) => i.heroeId === heroeId)) throw new MovilizacionInvalidaError('No estás en ese ejército.');
  return { ...convocatoria, integrantes: convocatoria.integrantes.map((i) => (i.heroeId === heroeId ? { ...i, ...seleccion } : i)) };
}

/** Un integrante se va mientras el ejército se prepara: sigue dentro del lugar, no hay nada que devolver. El Líder cancela. */
export function separarseDeConvocatoria(convocatoria: Convocatoria, heroeId: string): Convocatoria {
  if (!convocatoria.integrantes.some((i) => i.heroeId === heroeId)) throw new MovilizacionInvalidaError('No estás en ese ejército.');
  if (convocatoria.liderId === heroeId) throw new MovilizacionInvalidaError('El Líder no se separa de su ejército: lo cancela.');
  return { ...convocatoria, integrantes: convocatoria.integrantes.filter((i) => i.heroeId !== heroeId) };
}
