// Registro de conexiones WebSocket activas y difusión de eventos (Fase C5). Es INFRAESTRUCTURA: cómo se
// abre/cierra un socket y se manda un mensaje. La decisión de QUIÉN puede suscribirse a QUÉ vive en
// `session/canales.ts` (negocio) — este hub confía en que ya se comprobó antes de llamar a `suscribir`.
import type WebSocket from 'ws';
import type { EventoDominio } from '../../domain/eventos';
import { canalDeEvento } from '../../session/canales';

interface Conexion {
  jugadorId: string;
  canales: Set<string>;
}

export class HubDeDifusion {
  private readonly partidas = new Map<string, Map<WebSocket, Conexion>>();

  conectar(gameId: string, jugadorId: string, socket: WebSocket): void {
    let conexiones = this.partidas.get(gameId);
    if (!conexiones) {
      conexiones = new Map();
      this.partidas.set(gameId, conexiones);
    }
    conexiones.set(socket, { jugadorId, canales: new Set() });
  }

  /** Limpieza al cerrarse el socket. Sin rastro de qué tenía suscrito: al reconectar se empieza de cero (doc
   * 6 §2) — no hay nada que "recordar" entre conexiones. */
  desconectar(gameId: string, socket: WebSocket): void {
    const conexiones = this.partidas.get(gameId);
    if (!conexiones) return;
    conexiones.delete(socket);
    if (conexiones.size === 0) this.partidas.delete(gameId);
  }

  suscribir(gameId: string, socket: WebSocket, canal: string): void {
    this.partidas.get(gameId)?.get(socket)?.canales.add(canal);
  }

  desuscribir(gameId: string, socket: WebSocket, canal: string): void {
    this.partidas.get(gameId)?.get(socket)?.canales.delete(canal);
  }

  /** Cuántas conexiones tiene abiertas esta partida ahora mismo — para tests y, más adelante, métricas
   * (doc 4, Fase E3: "clientes conectados"). */
  conexionesAbiertas(gameId: string): number {
    return this.partidas.get(gameId)?.size ?? 0;
  }

  /**
   * Manda cada evento a las conexiones de `gameId` suscritas a su canal (`canalDeEvento`). Serializa el
   * mensaje UNA vez por evento, no una vez por conexión — con muchos suscriptores al mismo canal (doc 6:
   * hasta 500 jugadores por partida) evita repetir `JSON.stringify` para cada uno.
   */
  difundir(gameId: string, eventos: readonly EventoDominio[]): void {
    const conexiones = this.partidas.get(gameId);
    if (!conexiones || conexiones.size === 0 || eventos.length === 0) return;

    for (const evento of eventos) {
      const canal = canalDeEvento(evento);
      const mensaje = JSON.stringify({ tipo: 'evento', canal, evento });
      for (const [socket, conexion] of conexiones) {
        if (conexion.canales.has(canal) && socket.readyState === socket.OPEN) socket.send(mensaje);
      }
    }
  }
}
