// Fusión (Doc 2.6, opción 2; decidida el 2026-10-06): A y B se disuelven y nace C, SOLO si el Rey de B lo acepta. La propone el Rey de A
// (`PropuestaFusion`, pendiente y con caducidad) fijando ya el nombre y el Rey de C, y la contesta el Rey de B: su `sí` es su voto. Aquí
// viven las reglas; el traslado de lo que colgaba de las dos es el de la anexión (`trasladoDeFaccion.ts`). Quién es quién lo comprueba la
// capa de comandos (`session/comandos/fusion.ts`).
import { FUSION } from '../constants';
import { sumar, dias, type Instante } from '../domain/tiempo';
import type { EventoCrudo } from '../domain/eventos';
import type { Faccion, PropuestaFusion, RelacionPolitica, TecnologiasFaccion } from '../domain/types';
import { ReglaInvalidaError } from './errores';
import { calcularNivelFaccion } from './faccion';
import { tecnologiasDe } from './tecnologia';
import { propuestasVigentes, senorAjeno, trasladarFaccion, type MundoDeFacciones } from './trasladoDeFaccion';

export class FusionInvalidaError extends ReglaInvalidaError {}

/** Payloads de este subsistema: la fusión hace DESAPARECER dos Facciones, así que un consumidor necesita los ids. */
export interface PayloadFusionPropuesta {
  propuestaId: string;
  faccionAId: string;
  faccionBId: string;
}
export interface PayloadFusionRespondida extends PayloadFusionPropuesta {
  aceptada: boolean;
}
export interface PayloadFusion {
  faccionAId: string;
  faccionBId: string;
  faccionNuevaId: string;
  nuevoReyId: string;
}

/**
 * Lo que impide fusionar A y B con esos datos ahora mismo, o nada. Se mira al proponer y otra vez al aceptar: el mundo cambia mientras espera.
 * Devuelve las dos Facciones y el nombre ya limpio.
 */
export function exigirFusionPosible(
  facciones: readonly Faccion[],
  relaciones: readonly RelacionPolitica[],
  faccionAId: string,
  faccionBId: string,
  nuevoNombre: string,
  nuevoReyId: string
): { a: Faccion; b: Faccion; nombre: string } {
  if (faccionAId === faccionBId) throw new FusionInvalidaError('Una Facción no puede fusionarse consigo misma.');
  const a = facciones.find((f) => f.id === faccionAId);
  const b = facciones.find((f) => f.id === faccionBId);
  if (!a || !b) throw new FusionInvalidaError('Alguna de las Facciones no existe.');
  for (const f of [a, b]) {
    if (f.reyId === null) throw new FusionInvalidaError(`${f.nombre} no tiene Rey que pueda consentir.`);
  }
  if (nuevoReyId !== a.reyId && nuevoReyId !== b.reyId) throw new FusionInvalidaError('El Rey de la Facción nueva ha de ser el Rey de una de las dos.');
  const nombre = nuevoNombre.trim();
  if (!nombre) throw new FusionInvalidaError('La Facción nueva necesita un nombre.');
  for (const [f, otra] of [[a, b], [b, a]] as const) {
    if (senorAjeno(relaciones, f.id, otra.id)) throw new FusionInvalidaError(`${f.nombre} es vasalla de otra Facción: su señor tiene que liberarla antes.`);
  }
  return { a, b, nombre };
}

/** Una propuesta nueva de `faccionAId` a `faccionBId`, o el motivo por el que no cabe. Una sola entre cada par de Facciones, en cualquier sentido. */
export function proponerFusion(
  facciones: readonly Faccion[],
  relaciones: readonly RelacionPolitica[],
  propuestas: readonly PropuestaFusion[] | undefined,
  faccionAId: string,
  faccionBId: string,
  nuevoNombre: string,
  nuevoReyId: string,
  ahora: Instante,
  id: string
): PropuestaFusion {
  // Que quien llama sea el Rey de A lo exige la autorización (`autorizacion.ts`); aquí se anota el Rey cuyo consentimiento recoge.
  const { a, nombre } = exigirFusionPosible(facciones, relaciones, faccionAId, faccionBId, nuevoNombre, nuevoReyId);
  const yaPropuesta = propuestasVigentes(propuestas, ahora).some(
    (p) => (p.faccionAId === faccionAId && p.faccionBId === faccionBId) || (p.faccionAId === faccionBId && p.faccionBId === faccionAId)
  );
  if (yaPropuesta) throw new FusionInvalidaError('Ya hay una propuesta de fusión pendiente entre estas Facciones.');
  return { id, faccionAId, faccionBId, nuevoNombre: nombre, nuevoReyId, propuestaPor: a.reyId!, creadaEn: ahora, expiraEn: sumar(ahora, dias(FUSION.caducidadDias)) };
}

const unir = <T>(x: readonly T[] | undefined, y: readonly T[] | undefined): T[] => [...new Set([...(x ?? []), ...(y ?? [])])];

function unirTecnologias(a: TecnologiasFaccion, b: TecnologiasFaccion): TecnologiasFaccion {
  const aparecidas = unir(a.aparecidas, b.aparecidas);
  const reveladas = unir(a.reveladas, b.reveladas).filter((t) => !aparecidas.includes(t));
  return { aparecidas, adoptadas: unir(a.adoptadas, b.adoptadas), ...(reveladas.length ? { reveladas } : {}) };
}

/**
 * Ejecuta la fusión ya aceptada: nace C, con el nombre y el Rey de la propuesta; el otro Rey pasa a ser un ciudadano más. C hereda el
 * sigilo de A (Doc 2.8.1: el de B se pierde con B), la experiencia y la reputación MAYORES de las dos (el nivel se recalcula), la unión de
 * sus tecnologías y la capital del Rey electo si la había designado. Embajador vacante hasta que el nuevo Rey designe. De las dos pasa a C
 * todo lo demás (`trasladarFaccion`, dos veces: primero A y luego B), así que las relaciones de ambas se cancelan salvo sus vasallos.
 */
export function fusionar(
  mundo: MundoDeFacciones,
  faccionAId: string,
  faccionBId: string,
  nuevoNombre: string,
  nuevoReyId: string,
  instante: Instante
): { mundo: MundoDeFacciones; eventos: EventoCrudo[] } {
  const { a, b, nombre } = exigirFusionPosible(mundo.facciones, mundo.relaciones, faccionAId, faccionBId, nuevoNombre, nuevoReyId);
  const deReyElecto = nuevoReyId === a.reyId ? a : b;
  const nueva: Faccion = {
    id: `faccion-fusion-${instante}-${a.id}-${b.id}`,
    nombre,
    sigilo: a.sigilo,
    reyId: nuevoReyId,
    embajadorId: null,
    nivel: 1,
    experiencia: Math.max(a.experiencia, b.experiencia),
    ciudadanosIds: [],
    reputacion: Math.max(a.reputacion, b.reputacion),
  };
  nueva.nivel = calcularNivelFaccion(nueva);

  const conC: MundoDeFacciones = {
    ...mundo,
    facciones: [...mundo.facciones, nueva],
    // La marca de capital de cada una sale de la plaza; solo la del Rey electo se queda, ya como de C.
    asentamientos: mundo.asentamientos.map((s) =>
      s.capitalDeFaccionId === a.id || s.capitalDeFaccionId === b.id
        ? { ...s, capitalDeFaccionId: s.capitalDeFaccionId === deReyElecto.id ? nueva.id : undefined }
        : s
    ),
    tecnologia: {
      ...mundo.tecnologia,
      porFaccion: { ...mundo.tecnologia.porFaccion, [nueva.id]: unirTecnologias(tecnologiasDe(mundo.tecnologia, a.id), tecnologiasDe(mundo.tecnologia, b.id)) },
    },
  };

  return {
    mundo: trasladarFaccion(trasladarFaccion(conC, a.id, nueva.id), b.id, nueva.id),
    eventos: [
      {
        codigo: 'diplomacia.fusion',
        mensaje: `${a.nombre} y ${b.nombre} se fusionan en ${nombre}.`,
        payload: { faccionAId: a.id, faccionBId: b.id, faccionNuevaId: nueva.id, nuevoReyId } satisfies PayloadFusion,
      },
    ],
  };
}
