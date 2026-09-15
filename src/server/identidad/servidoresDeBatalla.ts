// Qué servidores de batalla de Conquest pueden hablar con esta instancia (doc 02 §3.3). Es configuración de despliegue,
// como los administradores: `SERVIDORES_BATALLA=servidorId:token,...`. Sin declararla no hay ninguno, y entonces
// tampoco se abre ninguna batalla de Unity: todo se resuelve con números (decisión del usuario, 2026-09-15).
//
// Es una credencial de SERVIDOR, distinta de cualquier sesión de jugador: con ella se asigna una batalla y se reporta
// su resultado, y nunca se reenvía a un cliente (doc 01 §15).
import { timingSafeEqual } from 'node:crypto';
import { credencialOpcionalDesdeCabecera } from './cabeceraAutorizacion';

export interface ServidorDeBatalla {
  id: string;
  token: string;
}

/** El esquema de `Authorization` con que se presenta un servidor de batalla. */
export const ESQUEMA_SERVIDOR_BATALLA = 'batalla-servidor';

/** Una entrada mal escrita es un fallo de arranque, y el mensaje no repite el valor: lleva secretos. */
export function parsearServidoresDeBatalla(valor: string | undefined): ServidorDeBatalla[] {
  if (!valor || valor.trim() === '') return [];
  const entradas = valor
    .split(',')
    .map((e) => e.trim())
    .filter((e) => e !== '');
  return entradas.map((entrada) => {
    const separador = entrada.indexOf(':');
    if (separador <= 0 || separador === entrada.length - 1) {
      throw new Error("SERVIDORES_BATALLA: cada entrada tiene que tener la forma 'servidorId:token'.");
    }
    return { id: entrada.slice(0, separador), token: entrada.slice(separador + 1) };
  });
}

/** El servidor que presenta esta cabecera (`Authorization: batalla-servidor <token>`), o `undefined`. Compara en tiempo
 * constante: un token que se adivina por lo que tarda la respuesta no es un secreto. */
export function servidorDeCabecera(cabecera: string | undefined, servidores: readonly ServidorDeBatalla[]): string | undefined {
  const credencial = credencialOpcionalDesdeCabecera(cabecera);
  if (!credencial || credencial.esquema !== ESQUEMA_SERVIDOR_BATALLA) return undefined;
  const presentado = Buffer.from(credencial.valor);
  return servidores.find((s) => {
    const esperado = Buffer.from(s.token);
    return esperado.length === presentado.length && timingSafeEqual(esperado, presentado);
  })?.id;
}
