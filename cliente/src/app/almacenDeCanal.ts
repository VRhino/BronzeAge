// El estado de UN canal de tiempo real (Docs/Coordinacion/03_Tiempo_real_por_segmentos.md): una foto que lo sustituye entero y deltas
// encadenados (`desde` → `hasta`) que lo mantienen al día. PURO: no sabe de sockets ni de la interfaz; `conexionAdmin.ts` le pasa los
// mensajes y `estadoEnVivo.ts` lee de aquí.
//
// Las colecciones (arrays con clave) se guardan como `Map` para que un `upsert`/`patch`/`remove` cueste O(1); la lista que ve quien lee se
// reconstruye solo si esa colección cambió, así que una colección sin cambios entrega siempre el MISMO array (identidad estable).

export type CambioDeCanal =
  | { t: string; id: string; op: 'upsert'; v: unknown }
  | { t: string; id: string; op: 'patch'; v: Record<string, unknown> }
  | { t: string; id: string; op: 'remove' }
  | { t: string; op: 'set'; v: unknown };

export interface DeltaDeCanal {
  desde: number;
  hasta: number;
  instante: number;
  cambios: CambioDeCanal[];
  escalares?: Record<string, unknown>;
}

export type ResultadoDeDelta = 'aplicado' | 'duplicado' | 'hueco';

type Entidad = Record<string, unknown>;

/** Cómo se saca el id de cada entidad de una colección (por defecto `id`). */
export type ClavesDeColeccion = Readonly<Record<string, string>>;

export class AlmacenDeCanal {
  /** Versión del último mensaje aplicado; -1 mientras no haya llegado ninguna foto. */
  version = -1;
  instante = 0;

  private readonly colecciones = new Map<string, Map<string, Entidad>>();
  private readonly listas = new Map<string, Entidad[]>();
  private readonly unicas = new Map<string, Entidad>();
  private readonly bloques = new Map<string, unknown>();

  /** `unicas`: claves de primer nivel que son UNA entidad con `id` (no un array): `asentamiento`, `campamento`, `heroe`. */
  constructor(
    private readonly unicasDelCanal: readonly string[] = [],
    private readonly claves: ClavesDeColeccion = {}
  ) {}

  get conFoto(): boolean {
    return this.version >= 0;
  }

  /** La foto sustituye todo lo que hubiera. */
  cargarFoto(version: number, instante: number, datos: Record<string, unknown>): void {
    this.colecciones.clear();
    this.listas.clear();
    this.unicas.clear();
    this.bloques.clear();
    for (const [k, v] of Object.entries(datos)) {
      if (Array.isArray(v)) {
        const mapa = new Map<string, Entidad>();
        for (const e of v as Entidad[]) mapa.set(this.idDe(k, e), e);
        this.colecciones.set(k, mapa);
      } else if (this.unicasDelCanal.includes(k) && v && typeof v === 'object') this.unicas.set(k, v as Entidad);
      else this.bloques.set(k, v);
    }
    this.version = version;
    this.instante = instante;
  }

  /** `hueco`: no encadena (o un `patch` sobre algo que no está): quien llama debe pedir una foto nueva y no mirar más deltas hasta recibirla. */
  aplicarDelta(delta: DeltaDeCanal): ResultadoDeDelta {
    if (this.version < 0) return 'hueco';
    if (delta.hasta <= this.version) return 'duplicado';
    if (delta.desde !== this.version) return 'hueco';
    for (const c of delta.cambios) if (!this.aplicarCambio(c)) return 'hueco';
    for (const [k, v] of Object.entries(delta.escalares ?? {})) this.bloques.set(k, v);
    this.version = delta.hasta;
    this.instante = delta.instante;
    return 'aplicado';
  }

  private aplicarCambio(c: CambioDeCanal): boolean {
    if (c.op === 'set') {
      this.bloques.set(c.t, c.v);
      return true;
    }
    if (this.unicasDelCanal.includes(c.t)) return this.aplicarEnUnica(c);
    let coleccion = this.colecciones.get(c.t);
    if (!coleccion) {
      coleccion = new Map();
      this.colecciones.set(c.t, coleccion);
    }
    if (c.op === 'remove') coleccion.delete(c.id);
    else if (c.op === 'upsert') coleccion.set(c.id, c.v as Entidad);
    else {
      const actual = coleccion.get(c.id);
      if (!actual) return false;
      coleccion.set(c.id, fusionar(actual, c.v));
    }
    this.listas.delete(c.t);
    return true;
  }

  private aplicarEnUnica(c: Exclude<CambioDeCanal, { op: 'set' }>): boolean {
    if (c.op === 'remove') this.unicas.delete(c.t);
    else if (c.op === 'upsert') this.unicas.set(c.t, c.v as Entidad);
    else {
      const actual = this.unicas.get(c.t);
      if (!actual) return false;
      this.unicas.set(c.t, fusionar(actual, c.v));
    }
    return true;
  }

  private idDe(coleccion: string, e: Entidad): string {
    return String(e[this.claves[coleccion] ?? 'id']);
  }

  /** La colección como array (el mismo mientras no cambie); vacío si el canal no la lleva. */
  lista<T>(nombre: string): T[] {
    let l = this.listas.get(nombre);
    if (!l) {
      l = [...(this.colecciones.get(nombre)?.values() ?? [])];
      this.listas.set(nombre, l);
    }
    return l as unknown as T[];
  }

  tiene(nombre: string): boolean {
    return this.colecciones.has(nombre) || this.unicas.has(nombre) || this.bloques.has(nombre);
  }

  unica<T>(nombre: string): T | undefined {
    return this.unicas.get(nombre) as T | undefined;
  }

  bloque<T>(nombre: string): T | undefined {
    return this.bloques.get(nombre) as T | undefined;
  }
}

/** `patch`: fusión superficial; un `null` explícito borra la clave (03 §4). */
function fusionar(actual: Entidad, cambios: Record<string, unknown>): Entidad {
  const nueva = { ...actual };
  for (const [k, v] of Object.entries(cambios)) {
    if (v === null) delete nueva[k];
    else nueva[k] = v;
  }
  return nueva;
}
