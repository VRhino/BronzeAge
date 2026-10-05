import { CATALOGO_SIGILO } from '../constants';
import type { Sigilo } from '../domain/types';

export type MotivoSigiloRechazado = 'invalido' | 'duplicado';

export function mismoSigilo(a: Sigilo, b: Sigilo): boolean {
  const conOrla = a.orlaId !== 'ninguna';
  return (
    a.formaId === b.formaId &&
    a.campoId === b.campoId &&
    a.emblemaId === b.emblemaId &&
    a.colorPrimarioId === b.colorPrimarioId &&
    a.colorSecundarioId === b.colorSecundarioId &&
    a.colorEmblemaId === b.colorEmblemaId &&
    a.orlaId === b.orlaId &&
    // Sin marco, el color del marco no se ve: no distingue un sigilo de otro.
    (!conOrla || a.colorOrlaId === b.colorOrlaId)
  );
}

function esDelCatalogo(s: Sigilo): boolean {
  const colores = CATALOGO_SIGILO.colores.map((c) => c.id) as string[];
  return (
    (CATALOGO_SIGILO.formas as readonly string[]).includes(s.formaId) &&
    (CATALOGO_SIGILO.campos as readonly string[]).includes(s.campoId) &&
    (CATALOGO_SIGILO.emblemas as readonly string[]).includes(s.emblemaId) &&
    (CATALOGO_SIGILO.orlas as readonly string[]).includes(s.orlaId) &&
    colores.includes(s.colorPrimarioId) &&
    colores.includes(s.colorSecundarioId) &&
    colores.includes(s.colorEmblemaId) &&
    colores.includes(s.colorOrlaId) &&
    s.colorPrimarioId !== s.colorSecundarioId
  );
}

/**
 * Por qué no vale un sigilo, o `null` si vale: tiene que ser del catálogo (con los dos colores del fondo distintos) y
 * ninguna otra Facción puede llevar uno idéntico. Solo se rechaza el duplicado EXACTO; los parecidos se permiten
 * (decidido por el usuario, 2026-10-05). Y es para siempre: no hay cambio de sigilo, así que no hay nada que liberar.
 */
export function motivoSigiloRechazado(s: Sigilo, ocupados: readonly Sigilo[]): MotivoSigiloRechazado | null {
  if (!esDelCatalogo(s)) return 'invalido';
  return ocupados.some((o) => mismoSigilo(o, s)) ? 'duplicado' : null;
}

/** Hash FNV-1a de 32 bits: solo hace falta un reparto estable de ids, no criptografía. */
function hash32(texto: string): number {
  let h = 0x811c9dc5;
  for (let i = 0; i < texto.length; i++) {
    h ^= texto.charCodeAt(i);
    h = Math.imul(h, 0x01000193);
  }
  return h >>> 0;
}

/**
 * Un sigilo libre elegido a partir de `semilla` (el id de la Facción): el mismo id da el mismo sigilo mientras esté
 * libre, y si no, el siguiente. Es lo que reciben las Facciones que no eligen uno (los bots): al azar, sin criterio.
 * El catálogo da más de 600 millones de combinaciones, así que la búsqueda siempre termina.
 */
export function sigiloLibre(semilla: string, ocupados: readonly Sigilo[]): Sigilo {
  const { formas, campos, emblemas, colores, orlas } = CATALOGO_SIGILO;
  const k = colores.length;
  const combinaciones = formas.length * campos.length * emblemas.length * orlas.length * k * (k - 1) * k * k;
  for (let paso = 0; paso < combinaciones; paso++) {
    let n = (hash32(semilla) + paso) % combinaciones;
    const saca = (modulo: number): number => {
      const resto = n % modulo;
      n = Math.floor(n / modulo);
      return resto;
    };
    const formaId = formas[saca(formas.length)]!;
    const campoId = campos[saca(campos.length)]!;
    const emblemaId = emblemas[saca(emblemas.length)]!;
    const orlaId = orlas[saca(orlas.length)]!;
    const primario = saca(k);
    // El secundario es cualquiera de los otros: se salta el primario.
    const secundario = saca(k - 1);
    const candidato: Sigilo = {
      formaId,
      campoId,
      emblemaId,
      colorPrimarioId: colores[primario]!.id,
      colorSecundarioId: colores[secundario >= primario ? secundario + 1 : secundario]!.id,
      colorEmblemaId: colores[saca(k)]!.id,
      orlaId,
      colorOrlaId: colores[saca(k)]!.id,
    };
    if (!ocupados.some((o) => mismoSigilo(o, candidato))) return candidato;
  }
  throw new Error('No queda ningún sigilo libre en el catálogo.');
}
