/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** Sujeto con el que `app/apiCliente.ts` se identifica ante el backend (proveedor de desarrollo). Debe
   * figurar en la variable `ADMINISTRADORES` del servidor, o este cliente recibirá 403 al crear la partida.
   * Ausente = `'jefa'`, que es el sujeto que usa la configuración de arranque por defecto. */
  readonly VITE_USUARIO?: string;
  /** Inyectadas por `vite.config.ts` (`define`) desde el entorno del servidor de dev — la pestaña "Mundo" las
   * muestra. `VITE_BACKEND_URL` = a qué backend proxya (para decidir Local / En la nube); `VITE_CODIGO_INVITACION`
   * = el `CODIGO_REGISTRO` del backend, si lo hay; `VITE_PROVEEDOR_AUTH` = proveedor de identidad en uso. */
  readonly VITE_BACKEND_URL?: string;
  readonly VITE_CODIGO_INVITACION?: string;
  readonly VITE_PROVEEDOR_AUTH?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
