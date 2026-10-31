import type { Point } from './tipos';
import { evaluarElevacion, evaluarTerreno, gradienteElevacion, type CampoElevacion } from './elevacion';
import { humedadRegionalEn } from './regional';
import type { RioRegional } from './hidrografiaRegional';
import { generarCampoRuido, evaluarRuido, type CampoRuido } from './ruido';
import { createRng, derivarSeed } from './rng';
import type { MuestraRelieve } from './muestras';

export interface CampoSuelos {
  ruido: CampoRuido; geologia: CampoRuido;
  agua: { a: Point; b: Point; alcance: number }[]; indice: Record<string, number[]>;
}
const clamp = (x: number) => Math.max(0, Math.min(1, x));
const suave = (x: number) => { const t = clamp(x); return t * t * (3 - 2 * t); };

export function crearSuelosRegional(seed: number, rios: RioRegional[]): CampoSuelos {
  const rng = createRng(derivarSeed(seed, 'suelos-regional-1'));
  const ruido = (f: number) => generarCampoRuido(rng, { octavas: 3, frecuenciaBase: f, lacunaridad: 2.1, persistencia: 0.5 });
  const campo: CampoSuelos = { ruido: ruido(1 / 2800), geologia: ruido(1 / 6500), agua: [], indice: {} };
  for (const rio of rios) for (let i = 1; i < rio.puntos.length; i++) {
    const a = rio.puntos[i - 1]!, b = rio.puntos[i]!, alcance = 450 + rio.orden * 220, id = campo.agua.length;
    campo.agua.push({ a, b, alcance });
    for (let y = Math.floor((Math.min(a.y, b.y) - alcance) / 1000); y <= Math.floor((Math.max(a.y, b.y) + alcance) / 1000); y++)
      for (let x = Math.floor((Math.min(a.x, b.x) - alcance) / 1000); x <= Math.floor((Math.max(a.x, b.x) + alcance) / 1000); x++)
        (campo.indice[`${x},${y}`] ??= []).push(id);
  }
  return campo;
}

/** `muestra` (altura, pendiente y humedad de `p`, de `muestrearRelieve`) evita recalcularlas. */
export function fertilidadSuelo(campo: CampoSuelos, elevacion: CampoElevacion, p: Point, muestra?: MuestraRelieve): number {
  if (evaluarTerreno(elevacion, p, muestra?.altura) === 'agua') return 0;
  const g = muestra ? undefined : gradienteElevacion(elevacion, p, 2);
  const pendiente = muestra?.pendiente ?? Math.hypot(g!.dx, g!.dy) * 1000;
  const humedad = muestra?.humedad ?? humedadRegionalEn(elevacion.regional, p), h = muestra?.altura ?? evaluarElevacion(elevacion, p);
  let agua = 0;
  for (const id of campo.indice[`${Math.floor(p.x / 1000)},${Math.floor(p.y / 1000)}`] ?? []) {
    const { a, b, alcance } = campo.agua[id]!, dx = b.x - a.x, dy = b.y - a.y;
    const t = clamp(((p.x - a.x) * dx + (p.y - a.y) * dy) / (dx * dx + dy * dy || 1));
    agua = Math.max(agua, 1 - suave(Math.hypot(p.x - a.x - dx * t, p.y - a.y - dy * t) / alcance));
  }
  for (const lago of elevacion.lagos ?? []) {
    let distancia = Infinity;
    for (const q of lago.contorno ?? [lago.centro]) distancia = Math.min(distancia, Math.hypot(q.x - p.x, q.y - p.y));
    agua = Math.max(agua, (1 - suave(distancia / 450)) * 0.65);
  }
  const llano = 1 - suave((pendiente - 0.025) / 0.24), aluvial = agua * llano;
  const suelo = evaluarRuido(campo.ruido, p);
  return clamp((0.12 + humedad * 0.55 + (suelo - 0.5) * 0.4 + aluvial * 0.35) * llano * (1 - suave((h - 0.6) / 0.2)));
}
