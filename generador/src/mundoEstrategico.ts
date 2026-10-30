import type { Point } from './tipos';
import { evaluarElevacion, evaluarTerreno, gradienteElevacion, type CampoElevacion } from './elevacion';
import { esTransitable } from './costeMovimiento';
import { densidadForestal, type CampoForestal } from './forestacionRegional';
import { evaluarSuelo, type CampoSuelos } from './suelosRegional';
import { evaluarRuido } from './ruido';
import { createRng, derivarSeed } from './rng';
import { colocarCampamentos, type Campamento } from './campamentos';
import { repartirMetales, type Sitio } from './metales';
import { generarParcelas } from './parcelas';
import { muestrearRelieve } from './muestras';

/** Lo que necesita del relieve regional ya generado (`generarRelieve`). */
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
  const altura = evaluarElevacion(r.elevacion, p);
  if (evaluarTerreno(r.elevacion, p, altura) !== 'llano' || !esTransitable(r.elevacion, p, altura) || !r.conexion.tieneAcceso(p)) return 0;
  // El sondeo en `p` y el suelo comparten la muestra del relieve; los otros cuatro sondeos son de otros puntos.
  const muestra = muestrearRelieve(r.elevacion, p);
  const bosque = Math.max(...[[0, 0], [240, 0], [-240, 0], [0, 240], [0, -240]]
    .map(([dx, dy]) => densidadForestal(r.forestacion, r.elevacion, { x: p.x + dx!, y: p.y + dy! }, dx === 0 && dy === 0 ? muestra : undefined)));
  const fertilidad = evaluarSuelo(r.suelos, r.elevacion, p, bosque, muestra).fertilidad;
  return fertilidad >= 0.35 && bosque >= 0.15 ? fertilidad + 0.5 * bosque : 0;
}

/** Sitios aptos para metal: colina o montaña, transitables y con acceso. La geología solo pesa en la preferencia. */
export function sitiosDeMetal(r: Relieve, paso = 120): Sitio[] {
  const rng = createRng(derivarSeed(r.config.seed, 'sitios-metal-1'));
  const sitios: Sitio[] = [];
  for (let y = paso / 2; y < r.config.alto; y += paso) for (let x = paso / 2; x < r.config.ancho; x += paso) {
    const p = { x: x + (rng() - 0.5) * paso * 0.7, y: y + (rng() - 0.5) * paso * 0.7 };
    const altura = evaluarElevacion(r.elevacion, p);
    const terreno = evaluarTerreno(r.elevacion, p, altura);
    if (terreno !== 'colina' && terreno !== 'montana') continue;
    const g = gradienteElevacion(r.elevacion, p, 2);
    if (Math.hypot(g.dx, g.dy) * 1000 > 0.35 || !esTransitable(r.elevacion, p, altura) || !r.conexion.tieneAcceso(p)) continue;
    const geologia = evaluarRuido(r.suelos.geologia, p);
    sitios.push({ posicion: p, calidad: {
      estano: (1 - suave((geologia - 0.37) / 0.12)) * suave((altura - 0.48) / 0.08),
      oro: suave((geologia - 0.54) / 0.14) * suave((altura - 0.50) / 0.08),
    } });
  }
  return sitios;
}

/** `densidadEn` da la densidad forestal de las celdas de parcelas; quien ya la tenga calculada para esos centros la sustituye. */
export function generarMundoEstrategico(r: Relieve, densidadEn: (p: Point) => number = p => densidadForestal(r.forestacion, r.elevacion, p)) {
  const campamentos: Campamento[] = colocarCampamentos(r.config.seed, r.config, p => puntuarCampamento(r, p));
  const sitios = sitiosDeMetal(r);
  const { nodos, vetas } = repartirMetales(r.config.seed, campamentos, sitios);
  const { masas, parcelas } = generarParcelas(r.config, densidadEn, campamentos);
  return { campamentos, nodos, vetas, masas, parcelas, sitios: sitios.length };
}
