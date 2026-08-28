/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** Sujeto con el que `app/apiCliente.ts` se identifica ante el backend (proveedor de desarrollo). Debe
   * figurar en la variable `ADMINISTRADORES` del servidor, o este cliente recibirá 403 al crear la partida.
   * Ausente = `'jefa'`, que es el sujeto que usa la configuración de arranque por defecto. */
  readonly VITE_USUARIO?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
