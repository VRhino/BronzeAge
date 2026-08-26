// Copia de las funciones de EVALUACIÓN de `src/worldgen/elevacion.ts` del backend.
//
// LIMITACIÓN DOCUMENTADA A PROPÓSITO: sin soporte de `region` (`RegionGeografica`, `worldgen/regiones.ts` del
// servidor, ~300 líneas de guías geográficas autoradas). Una partida creada con `region` en `POST
// /admin/partidas` trae `MapaGenerado.elevacion.region` definido, y este cliente lo IGNORA — el terreno que
// pinta diverge del real allí donde la guía de región pesaría (`avisarSiRegionNoSoportada` abajo). El caso
// SIN región ("mundo libre", el default de la mayoría de partidas) se reproduce completo, borde incluido.
// Ver el README de esta carpeta.
import { ELEVACION, ELEVACION_SUAVIZADO } from './config';
import { evaluarRuido, evaluarRuidoParcial } from './ruido';
import type { CampoElevacion, Point, TerrenoTipo } from './tipos';

function smoothstep(edge0: number, edge1: number, x: number): number {
  const t = Math.min(1, Math.max(0, (x - edge0) / (edge1 - edge0)));
  return t * t * (3 - 2 * t);
}

function factorBorde(limites: { ancho: number; alto: number }, p: Point, anchoFraccion: number): number {
  const distBorde = Math.min(p.x, limites.ancho - p.x, p.y, limites.alto - p.y);
  const anchoBorde = (anchoFraccion * (limites.ancho + limites.alto)) / 2;
  return 1 - smoothstep(0, anchoBorde, distBorde);
}

function bandaJugablePeso(e: number): number {
  const pesoBajo = smoothstep(ELEVACION.umbralAgua, ELEVACION.umbralCosta, e);
  const pesoAlto = 1 - smoothstep(ELEVACION.umbralMontana, ELEVACION.umbralCima, e);
  return pesoBajo * pesoAlto;
}

function elevacionSuavizada(campo: CampoElevacion, p: Point): number {
  // Sin blend de guía de región (limitación de arriba): con región, el servidor mezclaría la guía aquí.
  return evaluarRuidoParcial(campo.ruido, p, ELEVACION_SUAVIZADO.octavasSuaves);
}

let avisoRegionEmitido = false;
function avisarSiRegionNoSoportada(campo: CampoElevacion): void {
  if (campo.region && !avisoRegionEmitido) {
    avisoRegionEmitido = true;
    console.warn(
      '[terreno] Esta partida usa una región geográfica (`MapaGenerado.elevacion.region`) que este boilerplate ' +
        'no interpreta — el terreno pintado diverge del real donde pesa la guía de región. Ver la cabecera de elevacion.ts.'
    );
  }
}

/** Mismos parámetros que `ELEVACION_BORDE` del servidor (`worldgen/config.ts`) — no se movieron a `config.ts`
 * de este cliente porque solo los usa esta función, a diferencia de `ELEVACION`/`BIOMA`. */
const BORDE = { anchoFraccion: 0.09, objetivoCosta: 0.12, objetivoMontana: 0.97 } as const;

/** Debe coincidir bit a bit con `evaluarElevacion` del servidor para mundos SIN región. */
export function evaluarElevacion(campo: CampoElevacion, p: Point): number {
  avisarSiRegionNoSoportada(campo);
  let e = evaluarRuido(campo.ruido, p);

  let fBorde = 0;
  if (campo.borde) {
    fBorde = factorBorde(campo.borde.limites, p, BORDE.anchoFraccion);
    if (fBorde > 0) {
      const tipo = evaluarRuido(campo.borde.ruido, p);
      const objetivo = BORDE.objetivoCosta + tipo * (BORDE.objetivoMontana - BORDE.objetivoCosta);
      e = e + (objetivo - e) * fBorde;
    }
  }

  const pesoSuave = bandaJugablePeso(e) * (1 - fBorde) * ELEVACION_SUAVIZADO.pesoMaximo;
  if (pesoSuave > 0) {
    const suave = elevacionSuavizada(campo, p);
    e = e + (suave - e) * pesoSuave;
  }

  return Math.min(1, Math.max(0, e));
}

export function evaluarTerreno(campo: CampoElevacion, p: Point): TerrenoTipo {
  const e = evaluarElevacion(campo, p);
  if (e < ELEVACION.umbralAgua) return 'agua';
  if (e < ELEVACION.umbralCosta) return 'costa';
  if (e < ELEVACION.umbralColina) return 'llano';
  if (e < ELEVACION.umbralMontana) return 'colina';
  if (e < ELEVACION.umbralCima) return 'montana';
  return 'cima';
}
