// Copia local de los tipos de dominio devueltos por la proyección JSON.
// Mantiene a `cliente-jugador` desacoplado del motor de backend.

export interface Point {
  x: number;
  y: number;
}

export interface Faccion {
  id: string;
  nombre: string;
  reyId?: string | null;
  embajadorId?: string | null;
  nivel: number;
  experiencia?: number;
  ciudadanosIds?: string[];
  reputacion?: number;
  // ... ignoramos otros campos que no impactan el render ...
}

export interface Edificio {
  id: string;
  tipo: string; // EdificioTipo string
  estado: 'en_cola' | 'en_construccion' | 'activo';
  ambito?: 'asentamiento' | 'mapa';
  posicion: Point;
}

export interface Asentamiento {
  id: string;
  nombre?: string;
  faccionId: string;
  posicion: Point;
  nivel: number;
  nivelActual?: number;
  mantenimiento?: number;
  poblacion?: { pesants: number; artesanos: number; nobleza: number };
  almacen?: Record<string, { cantidad: number; capacidad: number }>;
  edificios: Edificio[];
}

export interface Caravana {
  id: string;
  origenAsentamientoId: string;
  destinoAsentamientoId?: string;
  posicionActual: Point;
  ruta?: Point[];
}

export interface ZonaFaccion {
  faccionId: string;
  contornos: Point[][];
}

export interface CampamentoBandido {
  id: string;
  posicion: Point;
}

export interface CaminoComercial {
  id: string;
  puntos: Point[];
}

export interface SegmentoTrazado {
  desde: Point;
  hasta: Point;
}

export interface TrazadoAsentamiento {
  calles: SegmentoTrazado[];
  caminos: SegmentoTrazado[];
  huellas: Record<string, { x: number; y: number; ancho: number; alto: number }>;
}
