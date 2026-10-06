// Lo que pasa cuando una Facción deja de existir y otra recoge lo suyo: la ruta ÚNICA de reasignación que comparten la anexión
// (`anexion.ts`, B pasa a A) y la fusión (`fusion.ts`, A y B pasan a C). Quién puede hacerlo y con qué consentimiento lo deciden ellas.
import type { Instante } from '../domain/tiempo';
import type {
  Asentamiento,
  Caravana,
  Ejercito,
  EstadoAedasResidentes,
  EstadoTecnologia,
  Faccion,
  MiradaIntel,
  PropuestaAnexion,
  PropuestaFusion,
  RedCaminos,
  RelacionPolitica,
} from '../domain/types';
import { fundirExploraciones, SIN_EXPLORAR } from './exploracion';
import type { MemoriaFaccion } from './memoria';

/** Todo lo que lleva el id de una Facción. Subconjunto de `GameSessionState`, para volver a esparcirlo sobre él. */
export interface MundoDeFacciones {
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
  propuestasFusion?: PropuestaFusion[];
}

/** Las propuestas (de anexión o de fusión) que aún valen: las caducadas se quedan en la lista hasta que algo las barre, pero ya no cuentan. */
export function propuestasVigentes<T extends { expiraEn: Instante }>(propuestas: readonly T[] | undefined, ahora: Instante): T[] {
  return (propuestas ?? []).filter((p) => ahora < p.expiraEn);
}

/** El señor de `faccionId` si es vasalla de alguien que no es `exceptoId` (su señor propio puede absorberla o fusionarse con ella: el vasallaje termina). */
export function senorAjeno(relaciones: readonly RelacionPolitica[], faccionId: string, exceptoId: string): string | undefined {
  return relaciones.find((r) => r.estado === 'activa' && r.tipo === 'vasallaje' && r.faccionBId === faccionId && r.faccionAId !== exceptoId)?.faccionAId;
}

/**
 * `origen` desaparece y `destino` recoge lo suyo: asentamientos (con sus cargos locales), ciudadanos, ejércitos, caravanas, rutas, Aedas
 * residentes y épicas cumplidas, miradas y lo que recordaba del mundo (la niebla se funde; en un choque de fichas gana `destino`).
 * Se pierden sus cargos de Facción, su sigilo, su reputación, su experiencia y su tecnología: si el destino ha de heredarlos, se los
 * pone el llamador antes (la fusión une la tecnología).
 *
 * Relaciones del origen: sus alianzas y guerras se cancelan sin penalización; sus vasallos pasan a `destino` con el mismo tributo, salvo
 * que `destino` ya tenga relación activa con ese vasallo, en cuyo caso queda libre. Se descartan las propuestas de anexión y de fusión
 * que lo nombraban.
 */
export function trasladarFaccion(mundo: MundoDeFacciones, origenId: string, destinoId: string): MundoDeFacciones {
  const sustituye = (id: string) => (id === origenId ? destinoId : id);

  const activas = mundo.relaciones.filter((r) => r.estado === 'activa');
  const conDestino = (id: string) => activas.some((r) => (r.faccionAId === destinoId && r.faccionBId === id) || (r.faccionBId === destinoId && r.faccionAId === id));
  const relaciones = mundo.relaciones.map((r): RelacionPolitica => {
    if (r.estado !== 'activa' || (r.faccionAId !== origenId && r.faccionBId !== origenId)) return r;
    const hereda = r.tipo === 'vasallaje' && r.faccionAId === origenId && r.faccionBId !== destinoId && !conDestino(r.faccionBId);
    return hereda ? { ...r, faccionAId: destinoId } : { ...r, estado: 'rota' };
  });

  const plazasDelDestino = new Set(mundo.asentamientos.filter((s) => s.faccionId === destinoId || s.faccionId === origenId).map((s) => s.id));
  const memDestino = mundo.memoriaPorFaccion[destinoId];
  const memOrigen = mundo.memoriaPorFaccion[origenId];
  const sinPropias = <T>(fichas: Record<string, T> | undefined): Record<string, T> =>
    Object.fromEntries(Object.entries(fichas ?? {}).filter(([id]) => !plazasDelDestino.has(id)));
  const { [origenId]: _memoriaDelOrigen, ...memoriaRestante } = mundo.memoriaPorFaccion;
  const memoriaPorFaccion =
    memDestino || memOrigen
      ? {
          ...memoriaRestante,
          [destinoId]: {
            exploracion: fundirExploraciones(memDestino?.exploracion ?? SIN_EXPLORAR, memOrigen?.exploracion ?? SIN_EXPLORAR),
            asentamientos: sinPropias({ ...memOrigen?.asentamientos, ...memDestino?.asentamientos }),
            ...(memDestino?.informes || memOrigen?.informes ? { informes: sinPropias({ ...memOrigen?.informes, ...memDestino?.informes }) } : {}),
          },
        }
      : memoriaRestante;

  const { [origenId]: _tecnologiaDelOrigen, ...porFaccion } = mundo.tecnologia.porFaccion;
  const primeros = Object.fromEntries(
    Object.entries(mundo.tecnologia.primeros).map(([tec, p]) => [tec, p && { ...p, faccionId: sustituye(p.faccionId) }])
  ) as EstadoTecnologia['primeros'];

  const residentes = mundo.aedasResidentes;
  const { [origenId]: epicasDelOrigen = 0, ...cumplidasRestantes } = residentes?.cumplidas ?? {};
  const nombraOrigen = (p: { absorbenteId: string; absorbidaId: string } | { faccionAId: string; faccionBId: string }) =>
    ('absorbenteId' in p ? [p.absorbenteId, p.absorbidaId] : [p.faccionAId, p.faccionBId]).includes(origenId);
  const origen = mundo.facciones.find((f) => f.id === origenId);

  return {
    ...mundo,
    facciones: mundo.facciones
      .filter((f) => f.id !== origenId)
      .map((f) => {
        const sinOrigen = f.derrotadaPor === origenId ? { ...f, derrotadaPor: destinoId } : f;
        return f.id === destinoId ? { ...sinOrigen, ciudadanosIds: [...new Set([...f.ciudadanosIds, ...(origen?.ciudadanosIds ?? [])])] } : sinOrigen;
      }),
    asentamientos: mundo.asentamientos.map((s) => (s.faccionId === origenId ? { ...s, faccionId: destinoId } : s)),
    relaciones,
    ejercitos: mundo.ejercitos.map((e) => (e.faccionId === origenId ? { ...e, faccionId: destinoId } : e)),
    caravanas: mundo.caravanas.map((c) => (c.faccionId === origenId ? { ...c, faccionId: destinoId } : c)),
    ...(mundo.red ? { red: { ...mundo.red, rutas: mundo.red.rutas.map((r) => ({ ...r, faccionId: sustituye(r.faccionId) })) } } : {}),
    ...(residentes
      ? {
          aedasResidentes: {
            ...residentes,
            aedas: residentes.aedas.map((e) => ({ ...e, faccionId: sustituye(e.faccionId) })),
            cumplidas: epicasDelOrigen > 0 ? { ...cumplidasRestantes, [destinoId]: (cumplidasRestantes[destinoId] ?? 0) + epicasDelOrigen } : cumplidasRestantes,
          },
        }
      : {}),
    ...(mundo.miradasIntel ? { miradasIntel: mundo.miradasIntel.map((m) => ({ ...m, faccionId: sustituye(m.faccionId) })) } : {}),
    memoriaPorFaccion,
    tecnologia: { ...mundo.tecnologia, porFaccion, primeros },
    // Lo que se ofreció al origen o por él ya no tiene sentido; lo que otros ofrecen al destino sigue en pie.
    propuestasAnexion: (mundo.propuestasAnexion ?? []).filter((p) => !nombraOrigen(p)),
    propuestasFusion: (mundo.propuestasFusion ?? []).filter((p) => !nombraOrigen(p)),
  };
}
