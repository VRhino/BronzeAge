import { crearCostasRegional } from '../src/costasRegional';
import { evaluarElevacion } from '../src/elevacion';
import { analizarEstrategia } from './visor-worldgen.estrategia';
import { crearCampoForestal, resumirSectoresForestales } from '../src/forestacionRegional';
import { crearRelieveRegional } from '../src/regional';
import { generarCampoElevacion, evaluarTerreno, gradienteElevacion } from '../src/elevacion';
import { generarHidrografiaRegional } from '../src/hidrografiaRegional';
import { crearSuelosRegional, generarRecursosRegional } from '../src/suelosRegional';
import { createRng, derivarSeed } from '../src/rng';
import { comprobarConexionRegional } from './visor-worldgen.conexion';

/** Laboratorio geográfico regional-4. No es una partida v16 ni calibra su economía. */
export function generarRegionalVisor(seed: number, lado: number, alto = 24000, densidadRecursos = 4) {
  const config = { ancho: lado, alto, seed };
  const rngDe = (etapa: string) => createRng(derivarSeed(seed, etapa));
  const regional = crearRelieveRegional(seed, config);
  const elevacion = generarCampoElevacion(rngDe('regional-elevacion'), config);
  elevacion.regional = regional;
  let hidro = generarHidrografiaRegional(seed, elevacion);
  elevacion.lagos = hidro.lagos;
  let conexion = comprobarConexionRegional(elevacion), factorLagos = 1;
  for (const factor of [0.5, 0.25, 0.1]) {
    if (conexion.conectadas === conexion.zonas) break;
    elevacion.lagos = [];
    hidro = generarHidrografiaRegional(seed, elevacion, factor);
    elevacion.lagos = hidro.lagos; factorLagos = factor;
    conexion = comprobarConexionRegional(elevacion);
  }
  const { rios, lagos, cuencas } = hidro;
  elevacion.costas = crearCostasRegional(seed, regional, p => evaluarElevacion(elevacion, p), rios);
  conexion = comprobarConexionRegional(elevacion);
  const forestacion = crearCampoForestal(seed, rios);
  const suelos = crearSuelosRegional(seed, rios);
  const candidatosRecursos = generarRecursosRegional(seed, suelos, elevacion, forestacion, densidadRecursos, conexion.tieneAcceso);
  const sectoresForestales = resumirSectoresForestales(forestacion, elevacion, lado, alto);
  // Medir los corredores previstos. No declararlos conectados solo por dibujar un símbolo.
  const pasos = regional.barreras.flatMap((b) => b.pasos.map((paso) => {
    let pendienteMaxima = 0, seco = true;
    for (let t = -b.ancho * 2.8; t <= b.ancho * 2.8; t += 20) {
      const p = { x: paso.centro.x - paso.tangente.y * t, y: paso.centro.y + paso.tangente.x * t };
      const g = gradienteElevacion(elevacion, p, 2);
      pendienteMaxima = Math.max(pendienteMaxima, Math.hypot(g.dx, g.dy) * 1000);
      seco &&= evaluarTerreno(elevacion, p) !== 'agua';
    }
    return { ...paso.centro, pendienteMaxima, transitable: seco && pendienteMaxima <= 0.7 };
  }));
  if (conexion.conectadas !== conexion.zonas) throw new Error(`Seed ${seed}: solo ${conexion.conectadas}/${conexion.zonas} zonas tienen conexión terrestre`);
  const estrategia = analizarEstrategia(elevacion, forestacion, candidatosRecursos, suelos);
  const recursos = candidatosRecursos.filter(r => !estrategia.recursosAislados.includes(r.id));
  for (const area of regional.areas) {
    if (!recursos.some(r => r.tipo === 'estano' && r.area === area.id)) throw new Error(`Área ${area.id+1}: sin estaño conectado`);
    if (!recursos.some(r => r.tipo === 'cobre' && r.area === area.id)) throw new Error(`Área ${area.id+1}: sin cobre conectado`);
  }
  return { estrategia, factorLagos, densidadRecursos, version: 'regional-17-costas-1 (experimental)', config, elevacion, rios, lagos, cuencas, suelos, recursos, forestacion, sectoresForestales, pasos, areas: regional.areas, conexion };
}
