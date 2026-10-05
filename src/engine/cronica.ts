// Crónica de los Aedas (Doc 6.7): lo que los Aedas cantan de los hechos públicos del servidor. No es estado: es una vista
// sobre los eventos. Unos ya son públicos (logros, Eras, títulos, descubrimientos: `CODIGOS_CRONICA_PUBLICOS`); de los
// demás (caídas, guerras, fundaciones…) se deriva un evento público `cronica.entrada` que dice lo que cualquiera puede
// saber, sin posiciones ni detalles. Puro y determinista: la variante de cada frase sale de un hash del propio evento.
import type { Asentamiento, Faccion } from '../domain/types';
import type { EventoDominio } from '../domain/eventos';
import type { PayloadAsedio } from './combate';
import type { PayloadAnexion, PayloadFusion } from './fusion';
import type { PayloadRebelionVasallo } from './diplomacia';
import type { PayloadAsentamientoRuinas, PayloadNivelSubio } from './mantenimiento';

// Lo que la crónica lee de dos payloads que viven en la capa de sesión (`PayloadFundado`, `PayloadGuerraDeclarada`): el motor
// no puede importarlos, así que declara solo los campos que usa.
interface PayloadFundado {
  asentamientoId: string;
  faccionId: string;
}
interface PayloadGuerraDeclarada {
  faccionAId: string;
  faccionesEnemigasIds: string[];
}

/** Código del evento derivado. */
export const CODIGO_CRONICA = 'cronica.entrada';

/** Eventos que ya son públicos (sin `asentamientoId`) y que cuentan en la crónica tal cual. */
export const CODIGOS_CRONICA_PUBLICOS: readonly string[] = ['tecnologia.logro', 'era.comienza', 'titulo.nace', 'titulo.cambia_manos', 'aedas.canta_descubrimiento'];

/** Payload de `cronica.entrada`. */
export interface PayloadCronica {
  /** Código del evento del que sale. */
  tipo: string;
  faccionIds: string[];
}

/** ¿Este evento va en la crónica? Los derivados y los públicos de la lista. */
export function esDeCronica(evento: Pick<EventoDominio, 'codigo' | 'asentamientoId'>): boolean {
  return evento.codigo === CODIGO_CRONICA || (CODIGOS_CRONICA_PUBLICOS.includes(evento.codigo) && evento.asentamientoId === undefined);
}

/** Primera plaza del mundo en cada nivel a partir del que se canta (Doc 6.7). */
const NIVEL_MINIMO_CANTADO = 3;

/** Una de las `opciones`, siempre la misma para la misma `clave`. */
function variante(clave: string, opciones: readonly string[]): string {
  let h = 0;
  for (const c of clave) h = (h * 31 + c.charCodeAt(0)) >>> 0;
  return opciones[h % opciones.length]!;
}

export interface MundoParaCronica {
  facciones: readonly Faccion[];
  asentamientos: readonly Asentamiento[];
}

function entrada(origen: EventoDominio, tipo: string, texto: string, faccionIds: string[]): EventoDominio {
  return { codigo: CODIGO_CRONICA, mensaje: texto, momento: origen.momento, payload: { tipo, faccionIds } satisfies PayloadCronica };
}

/** Los eventos públicos de crónica que se derivan de unos eventos recién ocurridos. */
export function entradasDeCronica(eventos: readonly EventoDominio[], mundo: MundoParaCronica): EventoDominio[] {
  const faccion = (id: string) => mundo.facciones.find((f) => f.id === id)?.nombre ?? id;
  const plaza = (id: string) => mundo.asentamientos.find((a) => a.id === id)?.nombre ?? id;
  const salida: EventoDominio[] = [];
  for (const e of eventos) {
    const clave = `${e.codigo}${e.momento}${e.mensaje}`;
    switch (e.codigo) {
      case 'combate.asedio_conquista': {
        const p = e.payload as PayloadAsedio;
        salida.push(entrada(e, e.codigo, variante(clave, [
          `Los Aedas cantan la caída de ${plaza(p.defensorId)}: ${faccion(p.faccionAtacanteId)} la arranca de las manos de ${faccion(p.faccionDefensoraId)}.`,
          `Los Aedas cantan que ${plaza(p.defensorId)} ha caído, y que ${faccion(p.faccionAtacanteId)} manda donde mandaba ${faccion(p.faccionDefensoraId)}.`,
        ]), [p.faccionAtacanteId, p.faccionDefensoraId]));
        break;
      }
      case 'asentamiento.ruinas': {
        const p = e.payload as PayloadAsentamientoRuinas;
        salida.push(entrada(e, e.codigo, `Los Aedas lloran las ruinas de ${p.nombre}, que fue de ${faccion(p.faccionId)}.`, [p.faccionId]));
        break;
      }
      case 'fundacion.asentamiento_fundado': {
        const p = e.payload as PayloadFundado;
        salida.push(entrada(e, e.codigo, `Los Aedas cantan la fundación de ${plaza(p.asentamientoId)} por ${faccion(p.faccionId)}.`, [p.faccionId]));
        break;
      }
      case 'diplomacia.guerra_declarada': {
        const p = e.payload as PayloadGuerraDeclarada;
        salida.push(entrada(e, e.codigo, `Los Aedas cantan que ${faccion(p.faccionAId)} declara la guerra a ${p.faccionesEnemigasIds.map(faccion).join(', ')}.`, [p.faccionAId, ...p.faccionesEnemigasIds]));
        break;
      }
      case 'diplomacia.rebelion_vasallo': {
        const p = e.payload as PayloadRebelionVasallo;
        salida.push(entrada(e, e.codigo, `Los Aedas cantan la rebelión de ${faccion(p.faccionVasallaId)} contra su señor ${faccion(p.faccionSenoraId)}.`, [p.faccionSenoraId, p.faccionVasallaId]));
        break;
      }
      case 'diplomacia.anexion': {
        const p = e.payload as PayloadAnexion;
        salida.push(entrada(e, e.codigo, `Los Aedas cantan: ${e.mensaje}`, [p.faccionAbsorbenteId, p.faccionAbsorbidaId]));
        break;
      }
      case 'diplomacia.fusion': {
        const p = e.payload as PayloadFusion;
        salida.push(entrada(e, e.codigo, `Los Aedas cantan: ${e.mensaje}`, [p.faccionAId, p.faccionBId, p.faccionNuevaId]));
        break;
      }
      case 'asentamiento.nivel_subio': {
        const p = e.payload as PayloadNivelSubio;
        const plazaNueva = mundo.asentamientos.find((a) => a.id === p.asentamientoId);
        const primera = plazaNueva && p.nivelNuevo >= NIVEL_MINIMO_CANTADO && !mundo.asentamientos.some((a) => a.id !== plazaNueva.id && a.nivel >= p.nivelNuevo);
        if (primera) salida.push(entrada(e, e.codigo, `Los Aedas cantan: ${plaza(plazaNueva.id)}, de ${faccion(plazaNueva.faccionId)}, es la primera ciudad del mundo en alcanzar el nivel ${p.nivelNuevo}.`, [plazaNueva.faccionId]));
        break;
      }
    }
  }
  return salida;
}
