import type { Point } from './tipos';
import type { CampoElevacion } from './types';
import { evaluarElevacion, evaluarTerreno, gradienteElevacion } from './elevacion';
import { esTransitable } from './costeMovimiento';
import { evaluarForestal, type CampoForestal } from './forestacionRegional';
import { evaluarSuelo, type CampoSuelos } from './suelosRegional';
import { evaluarRuido } from './ruido';
import { createRng, derivarSeed } from './rng';
import { colocarCampamentos, type Campamento } from './campamentos';
import { repartirMetales, type Sitio } from './metales';
import { generarParcelas } from './parcelas';

/** Lo que necesita del relieve regional ya generado (`generarRegionalVisor`). */
export interface Relieve {
  config: { ancho: number; alto: number; seed: number };
  elevacion: CampoElevacion;
  forestacion: CampoForestal;
  suelos: CampoSuelos;
  conexion: { tieneAcceso: (p: Point) => boolean };
}

const clamp = (x: number) => Math.max(0, Math.min(1, x));
const suave = (x: number) => { const t = clamp(x); return t * t * (3 - 2 * t); };

/** Puntuación de un sitio para campamento: fértil, llano, accesible y con bosque a la vista (Doc 1.9b). 0 = no apto. */
export function puntuarCampamento(r: Relieve, p: Point): number {
  if (evaluarTerreno(r.elevacion, p) !== 'llano' || !esTransitable(r.elevacion, p) || !r.conexion.tieneAcceso(p)) return 0;
  const bosque = Math.max(...[[0, 0], [240, 0], [-240, 0], [0, 240], [0, -240]]
    .map(([dx, dy]) => evaluarForestal(r.forestacion, r.elevacion, { x: p.x + dx!, y: p.y + dy! }).densidad));
  const fertilidad = evaluarSuelo(r.suelos, r.elevacion, p, bosque).fertilidad;
  return fertilidad >= 0.35 && bosque >= 0.15 ? fertilidad + 0.5 * bosque : 0;
}

/** Sitios aptos para metal: colina o montaña, transitables y con acceso. La geología solo pesa en la preferencia. */
export function sitiosDeMetal(r: Relieve, paso = 120): Sitio[] {
  const rng = createRng(derivarSeed(r.config.seed, 'sitios-metal-1'));
  const sitios: Sitio[] = [];
  for (let y = paso / 2; y < r.config.alto; y += paso) for (let x = paso / 2; x < r.config.ancho; x += paso) {
    const p = { x: x + (rng() - 0.5) * paso * 0.7, y: y + (rng() - 0.5) * paso * 0.7 };
    const terreno = evaluarTerreno(r.elevacion, p);
    if (terreno !== 'colina' && terreno !== 'montana') continue;
    const g = gradienteElevacion(r.elevacion, p, 2);
    if (Math.hypot(g.dx, g.dy) * 1000 > 0.35 || !esTransitable(r.elevacion, p) || !r.conexion.tieneAcceso(p)) continue;
    const altura = evaluarElevacion(r.elevacion, p), geologia = evaluarRuido(r.suelos.geologia, p);
    sitios.push({ posicion: p, calidad: {
      estano: (1 - suave((geologia - 0.37) / 0.12)) * suave((altura - 0.48) / 0.08),
      oro: suave((geologia - 0.54) / 0.14) * suave((altura - 0.50) / 0.08),
    } });
  }
  return sitios;
}

export function generarMundoEstrategico(r: Relieve) {
  const campamentos: Campamento[] = colocarCampamentos(r.config.seed, r.config, p => puntuarCampamento(r, p));
  const sitios = sitiosDeMetal(r);
  const { nodos, vetas } = repartirMetales(r.config.seed, campamentos, sitios);
  const { masas, parcelas } = generarParcelas(r.config, p => evaluarForestal(r.forestacion, r.elevacion, p).densidad, campamentos);
  return { campamentos, nodos, vetas, masas, parcelas, sitios: sitios.length };
}
