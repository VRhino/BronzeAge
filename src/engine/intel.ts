// La intel de las tabernas (Doc 5.12.10): información pagada sobre la niebla, con caducidad.
//
// Dos productos, los dos EXACTOS en el instante de la compra (no hay ruido ni datos falsos):
// - **Mirada**: un ojo prestado sobre un punto del mapa durante un rato. No se graba nada: mientras dura suma a «lo que se ve ahora»
//   de la Facción (y de sus aliados), igual que los ojos aliados, y al caducar desaparece sin dejar memoria.
// - **Informe de plaza**: la foto, con su fecha, del layout y la defensa de una plaza ajena. Se guarda en la memoria de la Facción y
//   envejece a la vista, como una ficha conocida.
//
// Aquí solo viven las reglas puras (precio, cupo, cooldown, qué entra en un informe). Quién puede comprar y de dónde sale el oro lo
// decide el comando (`session/comandos/intel.ts`): una taberna de plaza paga con el almacén, la de un campamento con el oro de botín.
import type { Asentamiento, Heroe, InformePlaza, MiradaIntel, Point } from '../domain/types';
import { minutos, sumar, type Instante } from '../domain/tiempo';
import { INTEL } from '../constants';
import { distancia } from '../world/geometria';
import { defensaDePlaza } from './ejercitos';
import { ReglaInvalidaError } from './errores';

export class IntelInvalidaError extends ReglaInvalidaError {}

/** ¿Sigue abierto este ojo? (`libreEn` es otra cosa: cuándo se puede volver a mirar la zona.) */
function miradaActiva(mirada: MiradaIntel, ahora: Instante): boolean {
  return mirada.expiraEn > ahora;
}

/** Las Miradas que ve una Facción ahora mismo. */
export function miradasActivasDe(miradas: readonly MiradaIntel[], faccionIds: ReadonlySet<string>, ahora: Instante): MiradaIntel[] {
  return miradas.filter((m) => faccionIds.has(m.faccionId) && miradaActiva(m, ahora));
}

/** El oro de una Mirada: una base más un tanto por la distancia a los ojos propios más cercanos —mirar lejos cuesta más (Economía §4.5)—. */
export function precioMirada(centro: Point, ojosPropios: readonly Point[]): number {
  const lejos = ojosPropios.length === 0 ? 0 : Math.min(...ojosPropios.map((o) => distancia(centro, o)));
  return Math.ceil(INTEL.mirada.oroBase + INTEL.mirada.oroPorUnidad * lejos);
}

/** El oro de un Informe: según el valor de lo que se mira, o sea el nivel de la plaza. */
export function precioInforme(objetivo: Pick<Asentamiento, 'nivel'>): number {
  return INTEL.informe.oroPorNivel * objetivo.nivel;
}

export interface PeticionMirada {
  id: string;
  faccionId: string;
  /** La taberna donde se compra (asentamiento o campamento): el cupo se cuenta por origen. */
  origenId: string;
  /** Cuántas Miradas admite esa taberna a la vez. */
  cupo: number;
  centro: Point;
  /** Plazas y columnas propias y la propia taberna: contra ellas se mide la distancia del precio. */
  ojosPropios: readonly Point[];
  limites: { ancho: number; alto: number };
  ahora: Instante;
}

/**
 * Compra una Mirada. Devuelve las Miradas del servidor sin las que ya han caducado Y quedado libres, más la nueva. El cobro lo hace
 * quien llama con el `precio` que sale de aquí.
 */
export function comprarMirada(miradas: readonly MiradaIntel[], p: PeticionMirada): { miradas: MiradaIntel[]; mirada: MiradaIntel; precio: number } {
  if (p.centro.x < 0 || p.centro.y < 0 || p.centro.x > p.limites.ancho || p.centro.y > p.limites.alto) {
    throw new IntelInvalidaError('Esa zona queda fuera del mapa.');
  }
  if (p.cupo <= 0) throw new IntelInvalidaError('Esta taberna no admite Miradas.');
  const vigentes = miradas.filter((m) => m.libreEn > p.ahora);
  const propias = vigentes.filter((m) => m.faccionId === p.faccionId);
  if (propias.filter((m) => m.origenId === p.origenId && miradaActiva(m, p.ahora)).length >= p.cupo) {
    throw new IntelInvalidaError('Esta taberna ya tiene abiertas todas las Miradas que admite.');
  }
  if (propias.some((m) => distancia(m.centro, p.centro) < INTEL.mirada.radio)) {
    throw new IntelInvalidaError('Esa zona se ha mirado hace poco: hay que esperar a que se enfríe.');
  }
  const expiraEn = sumar(p.ahora, minutos(INTEL.mirada.duracionMinutos));
  const mirada: MiradaIntel = {
    id: p.id,
    faccionId: p.faccionId,
    origenId: p.origenId,
    centro: p.centro,
    radio: INTEL.mirada.radio,
    compradaEn: p.ahora,
    expiraEn,
    libreEn: sumar(expiraEn, minutos(INTEL.mirada.cooldownMinutos)),
  };
  return { miradas: [...vigentes, mirada], mirada, precio: precioMirada(p.centro, p.ojosPropios) };
}

/**
 * Valida que un Informe se puede pedir: no de la propia Facción, de una plaza que la Facción conoce (si no, bastaría probar ids
 * para recorrer el mundo) y no repetido antes de su cooldown. `previo` es el último Informe que la Facción tiene de esa plaza.
 */
export function validarInforme(objetivo: Asentamiento, faccionId: string, conoce: boolean, previo: InformePlaza | undefined, ahora: Instante): number {
  if (objetivo.faccionId === faccionId) throw new IntelInvalidaError('Esa plaza es de tu Facción.');
  if (!conoce) throw new IntelInvalidaError('No conoces esa plaza: hay que haberla visto antes.');
  if (previo && sumar(previo.conocidoEn, minutos(INTEL.informe.cooldownMinutos)) > ahora) {
    throw new IntelInvalidaError('Ya tienes un informe reciente de esa plaza.');
  }
  return precioInforme(objetivo);
}

/** La foto de una plaza ajena: sus edificios y recintos, su guarnición y quién hay dentro. Nunca el almacén, las colas ni los cargos. */
export function levantarInforme(plaza: Asentamiento, heroes: readonly Heroe[], ahora: Instante): InformePlaza {
  const defensa = defensaDePlaza(plaza, heroes);
  return {
    asentamientoId: plaza.id,
    faccionId: plaza.faccionId,
    ...(plaza.nombre !== undefined ? { nombre: plaza.nombre } : {}),
    nivel: plaza.nivel,
    conocidoEn: ahora,
    edificios: plaza.edificios
      .filter((e) => e.estado !== 'en_cola')
      .map((e) => ({
        tipo: e.tipo,
        posicion: e.posicion,
        estado: e.estado,
        ...(e.nivelInterno !== undefined ? { nivelInterno: e.nivelInterno } : {}),
        ...(e.ambito !== undefined ? { ambito: e.ambito } : {}),
      })),
    recintos: (plaza.recintos ?? []).map((r) => ({ nivel: r.nivel, celdas: r.celdas, avance: r.avance })),
    guarnicion: defensa.guarnicion,
    heroesIds: defensa.heroesIds,
  };
}
