// Puerto de ALMACENAMIENTO DE BYTES (Ports & Adapters). Todo lo que este backend persiste —snapshots de
// partida, historial de eventos, registro de auditoría, dominio de acceso— son objetos de texto
// identificados por una clave plana. Este puerto es la única frontera que sabe DÓNDE viven esos bytes; los
// módulos de persistencia (`persistenciaPartida`, `eventosDePartida`, `auditoria`, `persistenciaIdentidad`)
// razonan sobre la FORMA de los datos y nada más.
//
// Cambiar de proveedor (disco local -> object storage -> base de datos) es escribir un adaptador nuevo de
// este puerto y elegirlo en la raíz de composición (`server/index.ts`). Ningún módulo de persistencia, ni
// `RunnerDePartida`, ni las rutas, se enteran.
//
// El único adaptador hoy es `enDisco.ts` (un archivo por clave bajo un directorio). Un adaptador remoto
// típico:
//   - `escribir`: PUT de object storage (ya atómico), o UPSERT en SQL.
//   - `anexar`: INSERT en SQL; en object storage sin append, leer+concatenar+escribir (correcto con un solo
//     escritor — `RunnerDePartida` ya serializa por partida).
//   - `listar`: LIST por prefijo, o SELECT.
//
// Las CLAVES son planas y con la extensión como pista de tipo, igual que los nombres de archivo actuales:
// `"<gameId>.json"`, `"<gameId>.eventos.jsonl"`, `"<gameId>.auditoria.jsonl"`, `"identidad.json"`. Un
// adaptador que no use un sistema de archivos las trata como identificadores opacos.
export interface AlmacenDeObjetos {
  /** Contenido del objeto, o `null` si no existe. Nunca lanza por "no existe" — eso es un caso normal
   * ("partida nueva", "primer arranque"), no un error. */
  leer(clave: string): Promise<string | null>;

  /**
   * Reemplaza el objeto entero. Debe ser ATÓMICO: un fallo a mitad deja el contenido ANTERIOR intacto, nunca
   * uno truncado. En disco es `.tmp` + `rename`; en object storage un PUT ya lo es; en SQL un UPDATE.
   */
  escribir(clave: string, contenido: string): Promise<void>;

  /**
   * Añade `contenido` al final del objeto (lo crea si no existe). Para los `.jsonl` append-only. El llamador
   * ya incluye el `\n` final si lo necesita. Seguro con un solo escritor por clave (que es el caso: la cola
   * serial de `RunnerDePartida` y la de `auditoria.ts`).
   */
  anexar(clave: string, contenido: string): Promise<void>;

  /** Claves existentes que empiezan por `prefijo` (`""` = todas). Orden no garantizado. */
  listar(prefijo: string): Promise<string[]>;
}
