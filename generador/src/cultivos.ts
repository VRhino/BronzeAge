import type { TerrenoTipo } from './tipos';
import { CULTIVOS } from './config';
import type { MuestraRelieve } from './muestras';

const clamp = (x: number) => Math.max(0, Math.min(1, x));
const suave = (x: number) => { const t = clamp(x); return t * t * (3 - 2 * t); };
interface Rampa { desde: number; hasta: number }
/** 0 hasta `desde`, 1 desde `hasta`, suave entre medias. */
const sube = (x: number, r: Rampa) => suave((x - r.desde) / (r.hasta - r.desde));
const baja = (x: number, r: Rampa) => 1 - sube(x, r);

/**
 * Fertilidad de la vid (0..1): laderas soleadas y secas. Pendiente suave a media, humedad baja a media; 0 en el agua y en las cumbres.
 * Funciones puras de la muestra del relieve: no consumen azar.
 */
export function fertilidadVid(m: MuestraRelieve, terreno: TerrenoTipo): number {
  if (terreno === 'agua') return 0;
  const c = CULTIVOS.vid;
  return sube(m.pendiente, c.pendienteSube) * baja(m.pendiente, c.pendienteBaja) * baja(m.humedad, c.humedadBaja) * baja(m.altura, c.alturaBaja);
}

/** Fertilidad de la oliva (0..1): tierra baja y cerca del mar, con humedad baja a media. `distanciaCosta` (> 0 en tierra) en unidades del generador. */
export function fertilidadOliva(m: MuestraRelieve, terreno: TerrenoTipo, distanciaCosta: number): number {
  if (terreno === 'agua') return 0;
  const c = CULTIVOS.oliva;
  return baja(m.altura, c.alturaBaja) * baja(distanciaCosta, c.distanciaCosta) * baja(m.humedad, c.humedadBaja) * baja(m.pendiente, c.pendienteBaja);
}
