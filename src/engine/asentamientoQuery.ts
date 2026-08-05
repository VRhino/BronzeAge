import type { Asentamiento, EdificioTipo } from '../domain/types';
import { EDIFICIO_CATALOGO } from '../constants';

export function edificiosPorTipoYEstado(
  asentamiento: Asentamiento,
  tipo: EdificioTipo,
  estado: 'activo' | 'en_construccion' | 'en_cola' = 'activo'
) {
  return asentamiento.edificios.filter((e) => e.tipo === tipo && e.estado === estado);
}

export function hayProyectoPendiente(asentamiento: Asentamiento, tipo: EdificioTipo): boolean {
  return asentamiento.edificios.some((e) => e.tipo === tipo && e.estado !== 'activo');
}

export function capacidadHabitacional(asentamiento: Asentamiento): number {
  return edificiosPorTipoYEstado(asentamiento, 'vivienda').length * EDIFICIO_CATALOGO.vivienda.capacidadHabitantes;
}

export function poblacionTotal(asentamiento: Asentamiento): number {
  const { pesants, artesanos, nobleza } = asentamiento.poblacion;
  return pesants + artesanos + nobleza;
}
