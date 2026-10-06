// Cliente del canal de control del servicio de bots (`src/bots/control/contrato.ts`): una conexión WebSocket directa al servicio,
// sin pasar por el backend del juego. Solo red: del motor importa tipos del contrato, que no tiene imports.
//
// Se autentica con el token como primera orden, y si la conexión se cae sin que lo pidiera quien la usa, reintenta con espera
// creciente. Un token incorrecto (cierre 1008) no reintenta: sería insistir con lo mismo.
import type { AccionDeBot, ComandoDeControl, EstadoServicio, MensajeDeServicio } from '@motor/bots/control/contrato';

export type EstadoConexion = 'desconectado' | 'conectando' | 'conectado' | 'reintentando';

type Respuesta = Extract<MensajeDeServicio, { tipo: 'respuesta' }>;
type Registro = Extract<MensajeDeServicio, { tipo: 'registro' }>;
/** Una orden, sin el `id` que pone el cliente. */
type Orden = Exclude<ComandoDeControl, { accion: 'autenticar' }>;

export interface OyentesControl {
  conexion(estado: EstadoConexion, detalle?: string): void;
  estado(estado: EstadoServicio): void;
  accion(accion: AccionDeBot): void;
  registro(registro: Omit<Registro, 'tipo'>): void;
  historial(acciones: AccionDeBot[], registros: Omit<Registro, 'tipo'>[]): void;
}

const ESPERA_MIN_MS = 1000;
const ESPERA_MAX_MS = 15_000;
const RESPUESTA_MAX_MS = 20_000;

export class ClienteControl {
  private ws?: WebSocket;
  private url = '';
  private token = '';
  private quiereEstar = false;
  private espera = ESPERA_MIN_MS;
  private reintento?: ReturnType<typeof setTimeout>;
  private siguiente = 1;
  private readonly pendientes = new Map<number, { resolver: (r: Respuesta) => void; limite: ReturnType<typeof setTimeout> }>();
  estado: EstadoConexion = 'desconectado';

  constructor(private readonly oyentes: OyentesControl) {}

  conectar(url: string, token: string): void {
    this.cerrar();
    this.url = url;
    this.token = token;
    this.quiereEstar = true;
    this.espera = ESPERA_MIN_MS;
    this.abrir();
  }

  desconectar(): void {
    this.quiereEstar = false;
    this.cerrar();
    this.cambiar('desconectado');
  }

  /** Manda una orden y espera su respuesta. Rechaza con el motivo si el servicio la rechaza o no contesta. */
  pedir(orden: Orden): Promise<unknown> {
    const ws = this.ws;
    if (!ws || ws.readyState !== WebSocket.OPEN || this.estado !== 'conectado') return Promise.reject(new Error('sin conexión con el servicio de bots'));
    return new Promise((resolver, rechazar) => {
      const id = this.siguiente++;
      const limite = setTimeout(() => {
        this.pendientes.delete(id);
        rechazar(new Error('el servicio no contestó'));
      }, RESPUESTA_MAX_MS);
      this.pendientes.set(id, { limite, resolver: (r) => (r.ok ? resolver(r.datos) : rechazar(new Error(r.error ?? 'rechazado'))) });
      ws.send(JSON.stringify({ id, ...orden }));
    });
  }

  private abrir(): void {
    this.cambiar(this.espera > ESPERA_MIN_MS ? 'reintentando' : 'conectando');
    let ws: WebSocket;
    try {
      ws = new WebSocket(this.url);
    } catch (err) {
      this.cambiar('desconectado', err instanceof Error ? err.message : 'dirección no válida');
      this.quiereEstar = false;
      return;
    }
    this.ws = ws;
    ws.addEventListener('open', () => {
      // La primera orden es el token; hasta que no la acepte no hay "conectado".
      const id = this.siguiente++;
      this.pendientes.set(id, {
        limite: setTimeout(() => ws.close(), RESPUESTA_MAX_MS),
        resolver: (r) => {
          if (!r.ok) return;
          this.espera = ESPERA_MIN_MS;
          this.cambiar('conectado');
        },
      });
      ws.send(JSON.stringify({ id, accion: 'autenticar', token: this.token }));
    });
    ws.addEventListener('message', (ev) => this.recibir(JSON.parse(String(ev.data)) as MensajeDeServicio));
    ws.addEventListener('close', (ev) => {
      if (this.ws !== ws) return;
      this.ws = undefined;
      for (const [id, p] of this.pendientes) {
        clearTimeout(p.limite);
        p.resolver({ tipo: 'respuesta', id, ok: false, error: 'conexión cerrada' });
      }
      this.pendientes.clear();
      if (ev.code === 1008) {
        this.quiereEstar = false;
        this.cambiar('desconectado', 'token incorrecto');
      } else if (this.quiereEstar) {
        this.cambiar('reintentando');
        this.reintento = setTimeout(() => this.abrir(), this.espera);
        this.espera = Math.min(this.espera * 2, ESPERA_MAX_MS);
      } else {
        this.cambiar('desconectado');
      }
    });
  }

  private recibir(m: MensajeDeServicio): void {
    switch (m.tipo) {
      case 'respuesta': {
        const p = this.pendientes.get(m.id);
        if (!p) return;
        clearTimeout(p.limite);
        this.pendientes.delete(m.id);
        p.resolver(m);
        return;
      }
      case 'estado':
        return this.oyentes.estado(m.estado);
      case 'accion':
        return this.oyentes.accion(m.accion);
      case 'registro':
        return this.oyentes.registro(m);
      case 'historial':
        return this.oyentes.historial(m.acciones, m.registros);
    }
  }

  private cerrar(): void {
    if (this.reintento) clearTimeout(this.reintento);
    const ws = this.ws;
    this.ws = undefined;
    ws?.close();
  }

  private cambiar(estado: EstadoConexion, detalle?: string): void {
    this.estado = estado;
    this.oyentes.conexion(estado, detalle);
  }
}
