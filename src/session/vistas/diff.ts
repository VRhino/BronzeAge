// Diff genérico de las fotos de un canal de vista (`Docs/Coordinacion/03_Tiempo_real_por_segmentos.md` §4). PURO: dos fotos
// ya filtradas para la misma audiencia entran, los cambios que llevan de una a otra salen. Nada de aquí sabe de niebla ni de
// quién ve qué; eso lo decide quien construye las fotos (`vistas/segmentos.ts`). Que el diff sea siempre entre fotos ya filtradas
// es lo que impide que un delta arrastre algo que el cliente no podía ver (03 §7.1).
//
// Una foto es un objeto de primer nivel. Cada clave es de uno de tres tipos, que fija el `EsquemaDeVista`:
//  - **colección con clave**: un array de entidades con `id` (o la clave que diga el esquema) o un mapa `id → valor`;
//    se difiere por entidad: `upsert` (nueva), `patch` (cambian algunas claves de primer nivel) o `remove`.
//  - **escalar**: viaja en `escalares`, solo si cambia.
//  - **bloque**: cualquier otra clave; se sustituye entera (`set`) si cambia.

export type Cambio =
  | { t: string; id: string; op: 'upsert'; v: unknown }
  | { t: string; id: string; op: 'patch'; v: Record<string, unknown> }
  | { t: string; id: string; op: 'remove' }
  | { t: string; op: 'set'; v: unknown };

export type Foto = Record<string, unknown>;

export interface EsquemaDeVista {
  /** Colecciones en forma de array: nombre → cómo se saca el id de cada entidad. */
  colecciones: Readonly<Record<string, (entidad: never) => string>>;
  /** Colecciones en forma de mapa `id → valor` (`nombresDe*`, `territorioPorEjercito`). */
  mapas?: readonly string[];
  /** Claves de primer nivel que viajan fusionadas en `escalares`. */
  escalares: readonly string[];
}

export interface Diferencia {
  cambios: Cambio[];
  escalares: Record<string, unknown>;
}

const porId = (entidad: { id: string }): string => entidad.id;
export const clavePorId: (entidad: never) => string = porId as (entidad: never) => string;

/** Igualdad estructural de JSON (objetos planos, arrays, primitivos). La identidad corta antes de recorrer: una ruta que no cambia es
 * el mismo array en las dos fotos cuando la foto nueva reutiliza la entidad del estado, y es el caso más caro. */
export function iguales(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (typeof a !== 'object' || typeof b !== 'object' || a === null || b === null) return false;
  if (Array.isArray(a)) {
    if (!Array.isArray(b) || a.length !== b.length) return false;
    for (let i = 0; i < a.length; i++) if (!iguales(a[i], b[i])) return false;
    return true;
  }
  if (Array.isArray(b)) return false;
  const ka = Object.keys(a as object);
  if (ka.length !== Object.keys(b as object).length) return false;
  for (const k of ka) {
    if (!(k in (b as object)) || !iguales((a as Record<string, unknown>)[k], (b as Record<string, unknown>)[k])) return false;
  }
  return true;
}

/** Una entidad es patcheable si no tiene ningún `null`: en un `patch` un `null` explícito BORRA la clave, así que una entidad cuyo
 * valor legítimo fuera `null` se mandaría entera (`upsert`) para no confundir un valor con un borrado. */
function tieneNulos(entidad: Record<string, unknown>): boolean {
  return Object.values(entidad).some((v) => v === null);
}

function diferirEntidad(t: string, id: string, antes: Record<string, unknown>, despues: Record<string, unknown>): Cambio | undefined {
  const parche: Record<string, unknown> = {};
  let cambiadas = 0;
  for (const k of Object.keys(despues)) {
    if (!(k in antes) || !iguales(antes[k], despues[k])) {
      parche[k] = despues[k];
      cambiadas++;
    }
  }
  for (const k of Object.keys(antes)) {
    if (!(k in despues)) {
      parche[k] = null;
      cambiadas++;
    }
  }
  if (cambiadas === 0) return undefined;
  return tieneNulos(despues)
    ? { t, id, op: 'upsert', v: despues }
    : { t, id, op: 'patch', v: parche };
}

function indexar(coleccion: unknown, clave: (e: never) => string): Map<string, Record<string, unknown>> {
  const indice = new Map<string, Record<string, unknown>>();
  if (!Array.isArray(coleccion)) return indice;
  for (const entidad of coleccion) indice.set(clave(entidad as never), entidad as Record<string, unknown>);
  return indice;
}

/** Los cambios que llevan de la foto `antes` a la foto `despues`. Primero todos los `remove`, luego `upsert`/`patch` (03 §4): así un id
 * reutilizado no se borra tras reinsertarse. Las colecciones salen en el orden del esquema y las entidades en el de la foto nueva. */
export function diferenciar(antes: Foto, despues: Foto, esquema: EsquemaDeVista): Diferencia {
  const quitar: Cambio[] = [];
  const poner: Cambio[] = [];
  const escalares: Record<string, unknown> = {};
  const mapas = new Set(esquema.mapas ?? []);
  const escalaresDe = new Set(esquema.escalares);

  const claves = new Set([...Object.keys(antes), ...Object.keys(despues)]);
  for (const k of [...Object.keys(esquema.colecciones), ...mapas, ...claves]) {
    if (!claves.delete(k)) continue; // ya tratada (o ausente en las dos)
    const a = antes[k];
    const d = despues[k];
    if (a === d) continue;

    if (escalaresDe.has(k)) {
      if (!iguales(a, d)) escalares[k] = d;
      continue;
    }

    const clave = esquema.colecciones[k];
    if (clave) {
      const previas = indexar(a, clave);
      const nuevas = indexar(d, clave);
      for (const id of previas.keys()) if (!nuevas.has(id)) quitar.push({ t: k, id, op: 'remove' });
      for (const [id, entidad] of nuevas) {
        const previa = previas.get(id);
        if (!previa) {
          poner.push({ t: k, id, op: 'upsert', v: entidad });
          continue;
        }
        if (previa === entidad) continue;
        const cambio = diferirEntidad(k, id, previa, entidad);
        if (cambio) poner.push(cambio);
      }
      continue;
    }

    if (mapas.has(k)) {
      const previo = (a ?? {}) as Record<string, unknown>;
      const nuevo = (d ?? {}) as Record<string, unknown>;
      for (const id of Object.keys(previo)) if (!(id in nuevo)) quitar.push({ t: k, id, op: 'remove' });
      for (const [id, valor] of Object.entries(nuevo)) if (!(id in previo) || !iguales(previo[id], valor)) poner.push({ t: k, id, op: 'upsert', v: valor });
      continue;
    }

    if (!iguales(a, d)) poner.push({ t: k, op: 'set', v: d });
  }

  return { cambios: [...quitar, ...poner], escalares };
}

/**
 * Aplica una diferencia a una foto, como lo haría el cliente (03 §3.3). Es la referencia contra la que se prueba el diff
 * (aplicar foto + N deltas ≡ la foto final) y lo que usa el difusor para no depender de que dos fotos consecutivas compartan estado.
 * Devuelve una foto nueva; no muta la de entrada. Un `patch` sobre una entidad que no existe es un hueco: lanza.
 */
export function aplicarDiferencia(foto: Foto, diferencia: Diferencia, esquema: EsquemaDeVista): Foto {
  const resultado: Foto = { ...foto, ...diferencia.escalares };
  const mapas = new Set(esquema.mapas ?? []);
  const copiadas = new Set<string>();
  const lista = (t: string): Record<string, unknown>[] => {
    if (!copiadas.has(t)) {
      resultado[t] = [...((resultado[t] as Record<string, unknown>[] | undefined) ?? [])];
      copiadas.add(t);
    }
    return resultado[t] as Record<string, unknown>[];
  };
  const mapa = (t: string): Record<string, unknown> => {
    if (!copiadas.has(t)) {
      resultado[t] = { ...((resultado[t] as Record<string, unknown> | undefined) ?? {}) };
      copiadas.add(t);
    }
    return resultado[t] as Record<string, unknown>;
  };

  for (const c of diferencia.cambios) {
    if (c.op === 'set') {
      resultado[c.t] = c.v;
      continue;
    }
    if (mapas.has(c.t)) {
      const m = mapa(c.t);
      if (c.op === 'remove') delete m[c.id];
      else m[c.id] = c.op === 'upsert' ? c.v : { ...(m[c.id] as object), ...c.v };
      continue;
    }
    const clave = esquema.colecciones[c.t];
    if (!clave) throw new Error(`colección desconocida: ${c.t}`);
    const entidades = lista(c.t);
    const i = entidades.findIndex((e) => clave(e as never) === c.id);
    if (c.op === 'remove') {
      if (i >= 0) entidades.splice(i, 1);
    } else if (c.op === 'upsert') {
      if (i >= 0) entidades[i] = c.v as Record<string, unknown>;
      else entidades.push(c.v as Record<string, unknown>);
    } else {
      if (i < 0) throw new Error(`patch de ${c.t}/${c.id}, que no existe: hay un hueco`);
      const fusionada: Record<string, unknown> = { ...entidades[i]! };
      for (const [k, v] of Object.entries(c.v)) {
        if (v === null) delete fusionada[k];
        else fusionada[k] = v;
      }
      entidades[i] = fusionada;
    }
  }
  return resultado;
}
