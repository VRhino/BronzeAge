import type { MapaGenerado } from '../terreno';
import type { ProyeccionJugador } from '../apiCliente';

export interface EstadoClienteJugador {
  usuarioActivo: string;
  gameIdActivo: string;
  mapaCache: { id: string; mapa: MapaGenerado } | null;
  modoVista: 'mundo' | 'asentamiento';
  proyeccionUltima: ProyeccionJugador | null;
  modoPanelFaccion: 'inicio' | 'crear' | 'unirse';
  pestanaInteraccion: 'faccion' | 'asentamientos' | 'informacion';
  modoFundacionActivo: boolean;
  posicionFundacion: { x: number; y: number } | null;
  puntoFundacionFijado: boolean;
  indiceTip: number;
  asentamientoDetalleTab: 'general' | 'edificios' | 'almacen' | 'produccion' | 'militar';
}

export const estadoCliente: EstadoClienteJugador = {
  usuarioActivo: '',
  gameIdActivo: 'local',
  mapaCache: null,
  modoVista: 'mundo',
  proyeccionUltima: null,
  modoPanelFaccion: 'inicio',
  pestanaInteraccion: 'faccion',
  modoFundacionActivo: false,
  posicionFundacion: null,
  puntoFundacionFijado: false,
  indiceTip: 0,
  asentamientoDetalleTab: 'general',
};

export const TIPS_FUNDACION = [
  'Usa el cursor para elegir el lugar de fundación y comprobar visualmente la zona de influencia.',
  'Es recomendable fundar donde tengas acceso a madera y piedra.',
  'Evita las zonas de influencia de otros asentamientos para encontrar una posición válida.',
] as const;

export const CAP_FUNDACION_POR_NIVEL = [1, 2, 3, 3, 4, 5, 5, 6, 6, 7] as const;
