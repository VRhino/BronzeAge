// Relieve de producción: relieve regional, hidrografía, costas, bosque y suelos de una seed, con la conexión terrestre comprobada.
import { crearCostasRegional } from './costasRegional';
import { evaluarElevacion, type CampoElevacion } from './elevacion';
import { crearCampoForestal } from './forestacionRegional';
import { crearRelieveRegional } from './regional';
import { generarHidrografiaRegional } from './hidrografiaRegional';
import { crearSuelosRegional } from './suelosRegional';
import { comprobarConexion } from './conexion';

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
  conexion = comprobarConexion(elevacion);
  const forestacion = crearCampoForestal(seed, rios);
  const suelos = crearSuelosRegional(seed, rios);
  if (conexion.conectadas !== conexion.zonas) throw new Error(`Seed ${seed}: solo ${conexion.conectadas}/${conexion.zonas} zonas tienen conexión terrestre`);
  return { config, elevacion, rios, lagos, suelos, forestacion, conexion, factorLagos };
}
