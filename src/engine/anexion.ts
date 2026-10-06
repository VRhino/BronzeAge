// Anexión (Doc 2.6, opción 1; decidido el 2026-10-06): A absorbe a B SOLO si el Rey de B lo acepta. La propone el Rey o el Embajador de A
// (`PropuestaAnexion`, pendiente y con caducidad) y la contesta el Rey de B. Aquí viven las reglas: quién puede proponer a quién y qué pasa
// con todo lo que colgaba de B. Quién es quién lo comprueba la capa de comandos (`session/comandos/anexion.ts`).
import { ANEXION } from '../constants';
import { sumar, dias, type Instante } from '../domain/tiempo';
import type {
  Asentamiento,
  Caravana,
  Ejercito,
  EstadoAedasResidentes,
  EstadoTecnologia,
  Faccion,
  MiradaIntel,
  PropuestaAnexion,
  RedCaminos,
  RelacionPolitica,
} from '../domain/types';
import type { EventoCrudo } from '../domain/eventos';
import { ReglaInvalidaError } from './errores';
import { fundirExploraciones, SIN_EXPLORAR } from './exploracion';
import type { MemoriaFaccion } from './memoria';

export class AnexionInvalidaError extends ReglaInvalidaError {}

/** Payloads de este subsistema: la anexión hace DESAPARECER una Facción, así que un consumidor necesita los ids. */
export interface PayloadAnexionPropuesta {
  propuestaId: string;
  absorbenteId: string;
  absorbidaId: string;
}
export interface PayloadAnexionRespondida extends PayloadAnexionPropuesta {
  aceptada: boolean;
}
export interface PayloadAnexion {
  faccionAbsorbenteId: string;
  faccionAbsorbidaId: string;
}

/** Las propuestas que aún valen: las caducadas se quedan en la lista hasta que algo las barre, pero ya no cuentan. */
export function propuestasVigentes(propuestas: readonly PropuestaAnexion[] | undefined, ahora: Instante): PropuestaAnexion[] {
  return (propuestas ?? []).filter((p) => ahora < p.expiraEn);
}

/** Lo que impide que A absorba a B ahora mismo, o nada. Se mira al proponer y otra vez al aceptar: el mundo cambia mientras espera. */
export function exigirAnexionPosible(facciones: readonly Faccion[], relaciones: readonly RelacionPolitica[], absorbenteId: string, absorbidaId: string): void {
  if (absorbenteId === absorbidaId) throw new AnexionInvalidaError('Una Facción no puede anexionarse a sí misma.');
  const absorbida = facciones.find((f) => f.id === absorbidaId);
  if (!facciones.some((f) => f.id === absorbenteId) || !absorbida) throw new AnexionInvalidaError('Alguna de las Facciones no existe.');
  if (absorbida.reyId === null) throw new AnexionInvalidaError(`${absorbida.nombre} no tiene Rey que pueda aceptar.`);
  if (relaciones.some((r) => r.estado === 'activa' && r.tipo === 'vasallaje' && r.faccionBId === absorbidaId && r.faccionAId !== absorbenteId)) {
    throw new AnexionInvalidaError(`${absorbida.nombre} es vasalla de otra Facción: su señor tiene que liberarla antes.`);
  }
}

/** Una propuesta nueva de `absorbenteId` a `absorbidaId`, o el motivo por el que no cabe. Una sola entre cada par de Facciones, en cualquier sentido. */
export function proponerAnexion(
  facciones: readonly Faccion[],
  relaciones: readonly RelacionPolitica[],
  propuestas: readonly PropuestaAnexion[] | undefined,
  absorbenteId: string,
  absorbidaId: string,
  propuestaPor: string,
  ahora: Instante,
  id: string
): PropuestaAnexion {
  exigirAnexionPosible(facciones, relaciones, absorbenteId, absorbidaId);
  const yaPropuesta = propuestasVigentes(propuestas, ahora).some(
    (p) => (p.absorbenteId === absorbenteId && p.absorbidaId === absorbidaId) || (p.absorbenteId === absorbidaId && p.absorbidaId === absorbenteId)
  );
  if (yaPropuesta) throw new AnexionInvalidaError('Ya hay una propuesta de anexión pendiente entre estas Facciones.');
  return { id, absorbenteId, absorbidaId, propuestaPor, creadaEn: ahora, expiraEn: sumar(ahora, dias(ANEXION.caducidadDias)) };
}

/** Lo que una anexión toca: todo lo que lleva el id de una Facción. Subconjunto de `GameSessionState`, para volver a esparcirlo sobre él. */
export interface MundoAnexable {
  facciones: Faccion[];
  asentamientos: Asentamiento[];
  relaciones: RelacionPolitica[];
  ejercitos: Ejercito[];
  caravanas: Caravana[];
  red?: RedCaminos;
  aedasResidentes?: EstadoAedasResidentes;
  miradasIntel?: MiradaIntel[];
  memoriaPorFaccion: Record<string, MemoriaFaccion>;
  tecnologia: EstadoTecnologia;
  propuestasAnexion?: PropuestaAnexion[];
}

const haceDeA = (b: string, a: string) => (id: string) => (id === b ? a : id);

/**
 * Ejecuta la anexión ya aceptada. A conserva nombre, Rey, Embajador, sigilo, reputación, experiencia y tecnología; de B pasan a A los
 * asentamientos (con sus cargos locales), los ciudadanos, los ejércitos, caravanas, rutas, Aedas residentes, miradas y lo que B
 * recordaba del mundo. Los cargos de Facción de B, su sigilo, su reputación y su tecnología desaparecen con ella.
 *
 * Relaciones de B: las de A con B terminan; sus alianzas y guerras se cancelan sin penalización; sus vasallos pasan a ser de A con
 * el mismo tributo, salvo que A ya tenga relación activa con ese vasallo, en cuyo caso queda libre.
 */
export function anexionar(mundo: MundoAnexable, absorbenteId: string, absorbidaId: string): { mundo: MundoAnexable; eventos: EventoCrudo[] } {
  const a = mundo.facciones.find((f) => f.id === absorbenteId);
  const b = mundo.facciones.find((f) => f.id === absorbidaId);
  if (!a || !b || a === b) throw new AnexionInvalidaError('Alguna de las Facciones no existe.');
  const aA = haceDeA(b.id, a.id);

  const activas = mundo.relaciones.filter((r) => r.estado === 'activa');
  const conA = (id: string) => activas.some((r) => (r.faccionAId === a.id && r.faccionBId === id) || (r.faccionBId === a.id && r.faccionAId === id));
  const relaciones = mundo.relaciones.map((r): RelacionPolitica => {
    if (r.estado !== 'activa' || (r.faccionAId !== b.id && r.faccionBId !== b.id)) return r;
    const hereda = r.tipo === 'vasallaje' && r.faccionAId === b.id && r.faccionBId !== a.id && !conA(r.faccionBId);
    return hereda ? { ...r, faccionAId: a.id } : { ...r, estado: 'rota' };
  });

  const asentamientosDeA = new Set(mundo.asentamientos.filter((s) => s.faccionId === a.id || s.faccionId === b.id).map((s) => s.id));
  const memA = mundo.memoriaPorFaccion[a.id];
  const memB = mundo.memoriaPorFaccion[b.id];
  const sinPropias = <T>(fichas: Record<string, T> | undefined): Record<string, T> =>
    Object.fromEntries(Object.entries(fichas ?? {}).filter(([id]) => !asentamientosDeA.has(id)));
  const { [b.id]: _memoriaDeB, ...memoriaRestante } = mundo.memoriaPorFaccion;
  const memoriaPorFaccion =
    memA || memB
      ? {
          ...memoriaRestante,
          [a.id]: {
            exploracion: fundirExploraciones(memA?.exploracion ?? SIN_EXPLORAR, memB?.exploracion ?? SIN_EXPLORAR),
            asentamientos: sinPropias({ ...memB?.asentamientos, ...memA?.asentamientos }),
            ...(memA?.informes || memB?.informes ? { informes: sinPropias({ ...memB?.informes, ...memA?.informes }) } : {}),
          },
        }
      : memoriaRestante;

  const { [b.id]: _tecnologiaDeB, ...porFaccion } = mundo.tecnologia.porFaccion;
  const primeros = Object.fromEntries(
    Object.entries(mundo.tecnologia.primeros).map(([tec, p]) => [tec, p && { ...p, faccionId: aA(p.faccionId) }])
  ) as EstadoTecnologia['primeros'];

  const residentes = mundo.aedasResidentes;
  const { [b.id]: epicasDeB = 0, ...cumplidasRestantes } = residentes?.cumplidas ?? {};

  return {
    mundo: {
      ...mundo,
      facciones: mundo.facciones
        .filter((f) => f.id !== b.id)
        .map((f) => {
          const sinB = f.derrotadaPor === b.id ? { ...f, derrotadaPor: a.id } : f;
          return f.id === a.id ? { ...sinB, ciudadanosIds: [...new Set([...a.ciudadanosIds, ...b.ciudadanosIds])] } : sinB;
        }),
      asentamientos: mundo.asentamientos.map((s) => (s.faccionId === b.id ? { ...s, faccionId: a.id } : s)),
      relaciones,
      ejercitos: mundo.ejercitos.map((e) => (e.faccionId === b.id ? { ...e, faccionId: a.id } : e)),
      caravanas: mundo.caravanas.map((c) => (c.faccionId === b.id ? { ...c, faccionId: a.id } : c)),
      ...(mundo.red ? { red: { ...mundo.red, rutas: mundo.red.rutas.map((r) => ({ ...r, faccionId: aA(r.faccionId) })) } } : {}),
      ...(residentes
        ? {
            aedasResidentes: {
              ...residentes,
              aedas: residentes.aedas.map((e) => ({ ...e, faccionId: aA(e.faccionId) })),
              cumplidas: epicasDeB > 0 ? { ...cumplidasRestantes, [a.id]: (cumplidasRestantes[a.id] ?? 0) + epicasDeB } : cumplidasRestantes,
            },
          }
        : {}),
      ...(mundo.miradasIntel ? { miradasIntel: mundo.miradasIntel.map((m) => ({ ...m, faccionId: aA(m.faccionId) })) } : {}),
      memoriaPorFaccion,
      tecnologia: { ...mundo.tecnologia, porFaccion, primeros },
      // Lo que se ofreció a B o por B ya no tiene sentido; lo que otros ofrecen a A sigue en pie.
      propuestasAnexion: (mundo.propuestasAnexion ?? []).filter((p) => p.absorbenteId !== b.id && p.absorbidaId !== b.id),
    },
    eventos: [
      {
        codigo: 'diplomacia.anexion',
        mensaje: `${a.nombre} anexiona a ${b.nombre}.`,
        payload: { faccionAbsorbenteId: a.id, faccionAbsorbidaId: b.id } satisfies PayloadAnexion,
      },
    ],
  };
}
