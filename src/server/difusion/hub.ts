// Registro de conexiones WebSocket activas y difusión de eventos (Fase C5). Es INFRAESTRUCTURA: cómo se
// abre/cierra un socket y se manda un mensaje. La decisión de QUIÉN puede suscribirse a QUÉ vive en
// `session/canales.ts` (negocio) — este hub confía en que ya se comprobó antes de llamar a `suscribir`.
import type WebSocket from 'ws';
import type { EventoDominio } from '../../domain/eventos';
import { CANAL_ADMIN_EVENTOS, canalesDeEvento } from '../../session/canales';

interface Conexion {
  jugadorId: string;
  canales: Set<string>;
  /** Solo en las de administración: ¿sigue teniendo derecho a lo que recibe? Se mira al difundir; si no, se le revoca el canal (03 §7.2). */
  vigente?: () => boolean;
}

/** `jugadorId` de las conexiones de administración (`admin/eventos`): no son de ningún jugador, así que no cuentan para su presencia. */
export const SIN_JUGADOR = '';

export class HubDeDifusion {
  private readonly partidas = new Map<string, Map<WebSocket, Conexion>>();
  /** Quién escucha cada canal, por partida: difundir recorre solo a los suscritos (y se salta los canales que nadie escucha) en vez de a todos. */
  private readonly oyentes = new Map<string, Map<string, Set<WebSocket>>>();

  conectar(gameId: string, jugadorId: string, socket: WebSocket, vigente?: () => boolean): void {
    let conexiones = this.partidas.get(gameId);
    if (!conexiones) {
      conexiones = new Map();
      this.partidas.set(gameId, conexiones);
    }
    conexiones.set(socket, { jugadorId, canales: new Set(), ...(vigente ? { vigente } : {}) });
  }

  /** Limpieza al cerrarse el socket. Sin rastro de qué tenía suscrito: al reconectar se empieza de cero (doc
   * 6 §2) — no hay nada que "recordar" entre conexiones. */
  desconectar(gameId: string, socket: WebSocket): void {
    const conexiones = this.partidas.get(gameId);
    if (!conexiones) return;
    for (const canal of conexiones.get(socket)?.canales ?? []) this.quitarOyente(gameId, canal, socket);
    conexiones.delete(socket);
    if (conexiones.size === 0) this.partidas.delete(gameId);
  }

  /** Cierra todos los sockets de una partida (se ha borrado). El código 4404 dice al cliente por qué, y que no reconecte. */
  cerrarPartida(gameId: string): number {
    const conexiones = this.partidas.get(gameId);
    if (!conexiones) return 0;
    this.partidas.delete(gameId);
    this.oyentes.delete(gameId);
    for (const socket of conexiones.keys()) socket.close(4404, 'partida borrada');
    return conexiones.size;
  }

  suscribir(gameId: string, socket: WebSocket, canal: string): void {
    const conexion = this.partidas.get(gameId)?.get(socket);
    if (!conexion) return;
    conexion.canales.add(canal);
    let delJuego = this.oyentes.get(gameId);
    if (!delJuego) this.oyentes.set(gameId, (delJuego = new Map()));
    let sockets = delJuego.get(canal);
    if (!sockets) delJuego.set(canal, (sockets = new Set()));
    sockets.add(socket);
  }

  desuscribir(gameId: string, socket: WebSocket, canal: string): void {
    this.partidas.get(gameId)?.get(socket)?.canales.delete(canal);
    this.quitarOyente(gameId, canal, socket);
  }

  private quitarOyente(gameId: string, canal: string, socket: WebSocket): void {
    const sockets = this.oyentes.get(gameId)?.get(canal);
    if (!sockets) return;
    sockets.delete(socket);
    if (sockets.size === 0) this.oyentes.get(gameId)!.delete(canal);
  }

  /** ¿Le queda a este jugador algún socket abierto en la partida? Con varias pestañas, cerrar una no lo desconecta. */
  sigueConectado(gameId: string, jugadorId: string): boolean {
    return [...(this.partidas.get(gameId)?.values() ?? [])].some((c) => c.jugadorId === jugadorId);
  }

  /** Cuántas conexiones tiene abiertas esta partida ahora mismo — para tests y, más adelante, métricas
   * (doc 4, Fase E3: "clientes conectados"). */
  conexionesAbiertas(gameId: string): number {
    return this.partidas.get(gameId)?.size ?? 0;
  }

  /**
   * Manda cada evento a las conexiones de `gameId` suscritas a su canal (`canalDeEvento`) y a las que escuchan `admin/eventos` (todos, sin
   * filtro: es la vista de administración). Serializa el mensaje UNA vez por evento y canal, no una vez por conexión, y **solo si hay alguien que
   * lo escuche**: un canal sin suscriptores no cuesta ni un `JSON.stringify`. Con las de administración se comprueba antes que su derecho siga
   * vigente (03 §7.2); si no, `revocado` y fuera.
   */
  difundir(gameId: string, eventos: readonly EventoDominio[]): void {
    const delJuego = this.oyentes.get(gameId);
    if (!delJuego || delJuego.size === 0 || eventos.length === 0) return;

    const conexiones = this.partidas.get(gameId);
    const vigencia = new Map<WebSocket, boolean>();
    for (const evento of eventos) {
      for (const canal of [...canalesDeEvento(evento), CANAL_ADMIN_EVENTOS]) {
        const sockets = delJuego.get(canal);
        if (!sockets || sockets.size === 0) continue;
        let mensaje: string | undefined;
        for (const socket of [...sockets]) {
          if (socket.readyState !== socket.OPEN) continue;
          const vigente = conexiones?.get(socket)?.vigente;
          if (vigente) {
            if (!vigencia.has(socket)) vigencia.set(socket, vigente());
            if (!vigencia.get(socket)) {
              socket.send(JSON.stringify({ tipo: 'revocado', canal, motivo: 'sin_derecho' }));
              this.desuscribir(gameId, socket, canal);
              continue;
            }
          }
          mensaje ??= JSON.stringify({ tipo: 'evento', gameId, canal, evento });
          socket.send(mensaje);
        }
      }
    }
  }
}
