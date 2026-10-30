// Relieve de producción: relieve regional, hidrografía, costas, bosque y suelos de una seed, con la conexión terrestre comprobada.
import { crearCostasRegional } from './costasRegional';
import { evaluarElevacion, type CampoElevacion } from './elevacion';
import { crearCampoForestal } from './forestacionRegional';
import { crearRelieveRegional } from './regional';
import { generarHidrografiaRegional } from './hidrografiaRegional';
import { crearSuelosRegional } from './suelosRegional';
import { comprobarConexion, conexionPorCeldas } from './conexion';
import { muestrearCeldas } from './celdas';
import { FACTOR } from './campamentos';
import { crearDetalleRelieve } from './relieveDetalle';

export function generarRelieve(seed: number, ancho: number, alto: number) {
  const config = { ancho, alto, seed };
  const regional = crearRelieveRegional(seed, config);
  const elevacion: CampoElevacion = { regional };
  let hidro = generarHidrografiaRegional(seed, elevacion);
  elevacion.lagos = hidro.lagos;
  let conexion = comprobarConexion(elevacion), factorLagos = 1;
  for (const factor of [0.5, 0.25, 0.1]) {
    if (conexion.conectadas === conexion.zonas) break;
    elevacion.lagos = [];
    hidro = generarHidrografiaRegional(seed, elevacion, factor);
    elevacion.lagos = hidro.lagos; factorLagos = factor;
    conexion = comprobarConexion(elevacion);
  }
  const { rios, lagos } = hidro;
  elevacion.costas = crearCostasRegional(seed, regional, p => evaluarElevacion(elevacion, p), rios);
  // El detalle va el último: ríos, lagos y costas se trazan sobre el relieve regional y el detalle respeta sus valles.
  const forestacion = crearCampoForestal(seed, rios);
  const suelos = crearSuelosRegional(seed, rios);
  // Con el detalle, la conexión se mide en las celdas de 4 u del artefacto (la malla gruesa ya no ve los valles). Si
  // alguna región queda aislada, se repite con menos detalle, como con los lagos. La rejilla se devuelve para que nadie
  // vuelva a muestrear los mismos centros.
  let fuerzaDetalle = 1, celdas, final;
  for (const fuerza of [1, 0.8, 0.6, 0.4, 0]) {
    fuerzaDetalle = fuerza;
    elevacion.detalle = crearDetalleRelieve(seed, rios, regional.barreras, fuerza);
    celdas = muestrearCeldas({ elevacion, forestacion }, { ancho, alto }, 4 * FACTOR);
    final = conexionPorCeldas(regional, celdas);
    if (final.conectadas === final.zonas) break;
  }
  if (final!.conectadas !== final!.zonas) throw new Error(`Seed ${seed}: solo ${final!.conectadas}/${final!.zonas} zonas tienen conexión terrestre`);
  return { config, elevacion, rios, lagos, suelos, forestacion, conexion: final!, factorLagos, fuerzaDetalle, celdas: celdas! };
}
