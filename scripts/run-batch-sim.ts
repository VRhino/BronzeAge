import * as fs from 'fs';
import type { Asentamiento, Faccion, Point } from '../src/domain/types';
import { generarMapa, MAPA_DEFAULT } from '../src/worldgen';
import { crearMapa, crearEstadoMapa, type Mapa } from '../src/world/mapa';
import { avanzarSimulacion, type EstadoSimulacion } from '../src/engine/simulation';
import { crearFaccion } from '../src/engine/faccion';
import { evaluarViabilidadFundacion, fundarAsentamiento } from '../src/engine/settlement';
import { nivelActualDe, tieneMercadoActivo, edificiosPorTipoYEstado } from '../src/engine/asentamientoQuery';
import { calcularNivelAsentamiento } from '../src/engine/mantenimiento';
import { mockMathRandomDeterminista } from '../src/engine/__tests__/fixtures';
import { avanzarNpcGobernanza, type ConfigNpcGobernanza } from '../simulaciones-batch/npcGobernanza';

const SEED = 7;
const NUM_FACCIONES = 100;
const JUGADORES_POR_ASENTAMIENTO = 5;
const TICKS = 3000;
const FOTO_CADA = 100;
const MIN_SEPARACION = 100;

const OTROS_MINERALES = ['cobre', 'estano', 'livestock'];
const TIPOS_EXTRACTOR = ['cantera', 'lenera', 'mina', 'minaCobre', 'minaEstano', 'corral'] as const;

function distancia(a: Point, b: Point): number {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

interface CandidatoFundacion {
  posicion: Point;
  tienePiedra: boolean;
  tieneOtroMineral: boolean;
  score: number;
}

function recolectarCandidatos(mapa: Mapa, paso: number): CandidatoFundacion[] {
  const candidatos: CandidatoFundacion[] = [];
  for (let x = paso; x < mapa.limites.ancho; x += paso) {
    for (let y = paso; y < mapa.limites.alto; y += paso) {
      const posicion = { x, y };
      const viabilidad = evaluarViabilidadFundacion(mapa, posicion, []);
      if (!viabilidad.recomendable) continue;
      const tienePiedra = viabilidad.recursosEnRadio.some((r) => r.tipo === 'piedra' && r.nodos > 0);
      const tieneOtroMineral = viabilidad.recursosEnRadio.some((r) => OTROS_MINERALES.includes(r.tipo) && r.nodos > 0);
      candidatos.push({ posicion, tienePiedra, tieneOtroMineral, score: (tienePiedra ? 2 : 0) + (tieneOtroMineral ? 1 : 0) });
    }
  }
  return candidatos;
}

function elegirPosicionesFundacion(mapa: Mapa, cantidad: number): CandidatoFundacion[] {
  const elegidas: CandidatoFundacion[] = [];
  for (const paso of [100, 50, 25, 10]) {
    if (elegidas.length >= cantidad) break;
    const pool = recolectarCandidatos(mapa, paso).sort((a, b) => b.score - a.score);
    for (const candidato of pool) {
      if (elegidas.length >= cantidad) break;
      if (elegidas.every((p) => distancia(p.posicion, candidato.posicion) >= MIN_SEPARACION)) {
        elegidas.push(candidato);
      }
    }
  }
  return elegidas;
}

function buscarDestinoFundacion(origen: Asentamiento, mapa: Mapa, asentamientos: Asentamiento[]): Point | undefined {
  for (let radio = 150; radio <= 600; radio += 150) {
    for (let angulo = 0; angulo < 360; angulo += 20) {
      const rad = (angulo * Math.PI) / 180;
      const posicion = { x: origen.posicion.x + Math.cos(rad) * radio, y: origen.posicion.y + Math.sin(rad) * radio };
      if (posicion.x < 0 || posicion.y < 0 || posicion.x >= mapa.limites.ancho || posicion.y >= mapa.limites.alto) continue;
      if (evaluarViabilidadFundacion(mapa, posicion, asentamientos).recomendable) return posicion;
    }
  }
  return undefined;
}

interface Foto {
  tick: number;
  vivos: number;
  colapsados: number;
  excepcionesAcumuladas: number;
  nivelesFaccion: Record<string, number>;
  nivelFaccionMax: number;
  xpFaccionMin: number;
  xpFaccionMedia: number;
  xpFaccionMax: number;
  faccionesConMasDeUnAsentamiento: number;
  caravanasFundacionLanzadasAcumuladas: number;
  nivelesAsentamiento: Record<string, number>;
  artesanosTotal: number;
  edificiosTransformacionActivosTotal: number;
  murallasActivas: number;
  palaciosActivos: number;
  conMercado: number;
  extraccionPorTipoActivosTotal: Record<string, number>;
  conGateNivel2Cumplido: number;
  tropasVivas: number;
  reclutamientosAcumulados: number;
  campamentosDestruidosAcumulados: number;
  truequesSupervivenciaAcumulados: number;
  acuerdosActivos: number;
  acuerdosCumplidos: number;
}

function construirFotoResumen(
  estado: EstadoSimulacion,
  tick: number,
  colapsados: number,
  excepcionesAcumuladas: number,
  reclutamientosAcumulados: number,
  campamentosDestruidosAcumulados: number,
  truequesSupervivenciaAcumulados: number,
  caravanasFundacionLanzadasAcumuladas: number
): Foto {
  const vivos = estado.asentamientos.length;

  const nivelesFaccion: Record<string, number> = {};
  let nivelFaccionMax = 0;
  let xpMin = Infinity;
  let xpMax = -Infinity;
  let xpSuma = 0;
  for (const f of estado.facciones) {
    nivelesFaccion[String(f.nivel)] = (nivelesFaccion[String(f.nivel)] ?? 0) + 1;
    if (f.nivel > nivelFaccionMax) nivelFaccionMax = f.nivel;
    if (f.experiencia < xpMin) xpMin = f.experiencia;
    if (f.experiencia > xpMax) xpMax = f.experiencia;
    xpSuma += f.experiencia;
  }

  const porFaccion = new Map<string, number>();
  for (const a of estado.asentamientos) porFaccion.set(a.faccionId, (porFaccion.get(a.faccionId) ?? 0) + 1);
  const faccionesConMasDeUnAsentamiento = [...porFaccion.values()].filter((n) => n > 1).length;

  const nivelesAsentamiento: Record<string, number> = { '1': 0, '2': 0, '3': 0, '4': 0, '5': 0 };
  let artesanosTotal = 0;
  let edificiosTransformacionActivosTotal = 0;
  let murallasActivas = 0;
  let palaciosActivos = 0;
  let conMercado = 0;
  let conGateNivel2Cumplido = 0;
  let tropasVivas = 0;
  const extraccionPorTipoActivosTotal: Record<string, number> = Object.fromEntries(TIPOS_EXTRACTOR.map((t) => [t, 0]));

  for (const a of estado.asentamientos) {
    const nivel = nivelActualDe(a);
    nivelesAsentamiento[String(nivel)] = (nivelesAsentamiento[String(nivel)] ?? 0) + 1;
    artesanosTotal += a.poblacion.artesanos;
    edificiosTransformacionActivosTotal +=
      edificiosPorTipoYEstado(a, 'curtiduria').length + edificiosPorTipoYEstado(a, 'armeria').length + edificiosPorTipoYEstado(a, 'fundicion').length;
    murallasActivas += edificiosPorTipoYEstado(a, 'muralla').length;
    palaciosActivos += edificiosPorTipoYEstado(a, 'palacio').length;
    if (tieneMercadoActivo(a)) conMercado++;
    if (calcularNivelAsentamiento(a) >= 2) conGateNivel2Cumplido++;
    tropasVivas += a.escuadrones.reduce((acc, e) => acc + e.cantidad, 0);
    for (const tipo of TIPOS_EXTRACTOR) {
      extraccionPorTipoActivosTotal[tipo] = (extraccionPorTipoActivosTotal[tipo] ?? 0) + edificiosPorTipoYEstado(a, tipo).length;
    }
  }

  const acuerdosActivos = estado.acuerdos.filter((ac) => ac.estado === 'activo').length;
  const acuerdosCumplidos = estado.acuerdos.filter((ac) => ac.estado === 'cumplido').length;

  return {
    tick,
    vivos,
    colapsados,
    excepcionesAcumuladas,
    nivelesFaccion,
    nivelFaccionMax,
    xpFaccionMin: xpMin === Infinity ? 0 : xpMin,
    xpFaccionMedia: Math.round((xpSuma / estado.facciones.length) * 100) / 100,
    xpFaccionMax: xpMax === -Infinity ? 0 : xpMax,
    faccionesConMasDeUnAsentamiento,
    caravanasFundacionLanzadasAcumuladas,
    nivelesAsentamiento,
    artesanosTotal,
    edificiosTransformacionActivosTotal,
    murallasActivas,
    palaciosActivos,
    conMercado,
    extraccionPorTipoActivosTotal,
    conGateNivel2Cumplido,
    tropasVivas,
    reclutamientosAcumulados,
    campamentosDestruidosAcumulados,
    truequesSupervivenciaAcumulados,
    acuerdosActivos,
    acuerdosCumplidos,
  };
}

async function main() {
  const restaurarMathRandom = mockMathRandomDeterminista(SEED);

  const mapaGenerado = generarMapa({ ancho: MAPA_DEFAULT.ancho, alto: MAPA_DEFAULT.alto, seed: SEED });
  const estadoMapa = crearEstadoMapa();
  const mapa = crearMapa(mapaGenerado, estadoMapa);

  let facciones: Faccion[] = [];
  for (let i = 0; i < NUM_FACCIONES; i++) {
    facciones.push(crearFaccion(`faccion-${i + 1}`, `Faccion ${i + 1}`));
  }

  const candidatos = elegirPosicionesFundacion(mapa, NUM_FACCIONES);
  const conPiedra = candidatos.filter((c) => c.tienePiedra).length;
  const conOtroMineral = candidatos.filter((c) => c.tieneOtroMineral).length;
  console.log(`Posiciones con piedra alcanzable: ${conPiedra}/${candidatos.length}`);
  console.log(`Posiciones con algún otro mineral alcanzable: ${conOtroMineral}/${candidatos.length}`);

  let asentamientos: Asentamiento[] = [];
  const idsFundados: string[] = [];
  for (let i = 0; i < candidatos.length; i++) {
    const faccion = facciones[i]!;
    const posicion = candidatos[i]!.posicion;
    const jugadores = Array.from({ length: JUGADORES_POR_ASENTAMIENTO }, (_, j) => `jugador-${faccion.id}-${j + 1}`);
    const resultado = fundarAsentamiento(mapa, facciones, faccion.id, posicion, jugadores, asentamientos, 0);
    asentamientos.push(resultado.asentamiento);
    idsFundados.push(resultado.asentamiento.id);
    facciones = resultado.facciones;
  }

  let estado: EstadoSimulacion = {
    asentamientos,
    facciones,
    caravanas: [],
    acuerdos: [],
    ordenes: [],
    relaciones: [],
    titulos: [],
    caminos: [],
    campamentosBandidos: [],
    bandidosProximoSpawnTick: 0,
  };

  const config: ConfigNpcGobernanza = { buscarDestinoFundacion };

  const fotos: Foto[] = [];
  let excepcionesAcumuladas = 0;
  let idsVivosAntes = new Set(idsFundados);
  const idsColapsadosVistos = new Set<string>();
  let reclutamientosAcumulados = 0;
  let campamentosDestruidosAcumulados = 0;
  let truequesSupervivenciaAcumulados = 0;
  let caravanasFundacionLanzadasAcumuladas = 0;

  for (let tick = 1; tick <= TICKS; tick++) {
    try {
      const trasMotor = avanzarSimulacion(estado, mapa, tick);
      const trasNpc = avanzarNpcGobernanza(trasMotor, mapa, tick, config);
      estado = trasNpc.estado;
      reclutamientosAcumulados += trasNpc.stats.reclutamientosExitosos;
      campamentosDestruidosAcumulados += trasNpc.stats.campamentosDestruidos;
      truequesSupervivenciaAcumulados += trasNpc.stats.truequesSupervivenciaPropuestos;
      caravanasFundacionLanzadasAcumuladas += trasNpc.stats.caravanasFundacionLanzadas;
    } catch (err) {
      excepcionesAcumuladas++;
    }

    const idsVivosAhora = new Set(estado.asentamientos.map((a) => a.id));
    for (const id of idsVivosAntes) {
      if (!idsVivosAhora.has(id)) idsColapsadosVistos.add(id);
    }
    idsVivosAntes = idsVivosAhora;

    if (tick % FOTO_CADA === 0) {
      fotos.push(
        construirFotoResumen(
          estado,
          tick,
          idsColapsadosVistos.size,
          excepcionesAcumuladas,
          reclutamientosAcumulados,
          campamentosDestruidosAcumulados,
          truequesSupervivenciaAcumulados,
          caravanasFundacionLanzadasAcumuladas
        )
      );
    }
  }

  console.log(`Excepciones totales: ${excepcionesAcumuladas}`);
  console.log(JSON.stringify({ fotos, excepciones: excepcionesAcumuladas }, null, 2));

  restaurarMathRandom();
}

main().catch(console.error);
