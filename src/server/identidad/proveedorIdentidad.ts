// Puerto de autenticación (patrón Strategy / Ports & Adapters). `servicioAutenticacion.ts` no sabe NADA de
// cómo se verifica una credencial concreta — cada proveedor (desarrollo, OAuth, JWT de un IdP externo...)
// implementa este único contrato y se da de alta en `registroProveedores.ts`. Añadir o retirar un proveedor
// es escribir/borrar un adaptador + una línea de registro; nada de lo que consume este puerto cambia.
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
  /** Esquema que este proveedor atiende en la cabecera `Authorization: <esquema> <valor>` (ej. `'dev'`). Es
   * la clave de `registroProveedores.ts` — así una request se enruta al proveedor correcto sin que nada
   * central conozca la lista completa de esquemas soportados. */
  readonly esquema: string;
  /** Verifica `valor` y devuelve la identidad externa que representa. Lanza `CredencialInvalidaError` si no
   * es válida — nunca devuelve un resultado "vacío" que el llamador tenga que interpretar. */
  autenticar(valor: string): Promise<IdentidadExterna>;
}
