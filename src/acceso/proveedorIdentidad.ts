// Puerto de autenticación (patrón Strategy / Ports & Adapters). `servicioAutenticacion.ts` no sabe NADA de
// cómo se verifica una credencial concreta — cada proveedor (desarrollo, OAuth, JWT de un IdP externo...)
// implementa este único contrato y se indexa por su `esquema`. Añadir o retirar un proveedor es escribir o
// borrar un adaptador y darlo de alta; nada de lo que consume este puerto cambia.
//
// El puerto y el registro son NEGOCIO (qué forma tiene un proveedor, cómo se elige el que atiende una
// credencial); los adaptadores concretos y la lista de cuáles están activos son infraestructura y viven en
// `server/identidad/` — ver `proveedoresActivos.ts`.
export interface IdentidadExterna {
  proveedor: string;
  /** Id estable dentro de ESE proveedor (el `sub` de OIDC, el uid de un IdP, etc.). Junto a `proveedor` es la
   * clave que `servicioAutenticacion.ts` usa para el find-or-create de `Usuario`. */
  sujetoId: string;
  email?: string;
}

export class CredencialInvalidaError extends Error {
  constructor(mensaje = 'credencial invalida o expirada') {
    super(mensaje);
    this.name = 'CredencialInvalidaError';
  }
}

export interface ProveedorIdentidad {
  /** Esquema de credencial que este proveedor atiende (ej. `'dev'`). Es la clave del registro: una petición
   * llega al proveedor correcto sin que nada central conozca la lista completa de esquemas soportados. */
  readonly esquema: string;
  /** Verifica `valor` y devuelve la identidad externa que representa. Lanza `CredencialInvalidaError` si no
   * es válida — nunca devuelve un resultado "vacío" que el llamador tenga que interpretar. */
  autenticar(valor: string): Promise<IdentidadExterna>;
}

/** Indexa proveedores por esquema. Rechazar duplicados es deliberado: dos adaptadores compitiendo por el
 * mismo esquema es un error de configuración que debe romper al arrancar, no resolverse en silencio por
 * orden de registro. */
export function crearRegistroProveedores(proveedores: ProveedorIdentidad[]): Map<string, ProveedorIdentidad> {
  const registro = new Map<string, ProveedorIdentidad>();
  for (const proveedor of proveedores) {
    if (registro.has(proveedor.esquema)) {
      throw new Error(`esquema de proveedor de identidad duplicado: '${proveedor.esquema}'`);
    }
    registro.set(proveedor.esquema, proveedor);
  }
  return registro;
}
