import type { NodoRecurso, Point, Rareza, ZonaBosque } from './tipos';
import { evaluarBioma } from './biomas';
import { LIVESTOCK, RECURSO_BIOMA_PERMITIDO, RECURSO_CANTIDAD_NODO, RECURSO_RAREZA, RECURSO_TIPOS_POR_RAREZA } from './config';
import { colocarValidos, type Limites } from './colocacion';
import { evaluarTerreno } from './elevacion';
import { esTransitable } from './costeMovimiento';
import { randInt, randRange, type RandomFn } from './rng';
import type { CampoElevacion, CampoFertilidad } from './types';
import type { RioZona } from './tipos';

/** Solo acepta puntos transitables cuyo bioma admite el recurso. La cantidad final
 * depende del hábitat disponible; no se fuerzan nodos en terreno inadecuado. */
function aceptaTerrenoPara(
  tipo: string,
  elevacion: CampoElevacion,
  fertilidad: CampoFertilidad,
  rios: readonly RioZona[]
): (p: Point) => boolean {
  const permitido = RECURSO_BIOMA_PERMITIDO[tipo];
  if (!permitido) return () => true;
  return (p) => esTransitable(elevacion, p) && permitido.includes(evaluarBioma(elevacion, fertilidad, rios, p));
}

/** Escala la cuota a la superficie donde realmente puede aparecer cada tipo. */
function cuotaPorHabitat(tipo: string, base: number, limites: Limites, elevacion: CampoElevacion): number {
  const referencia = tipo === 'piedra' ? 0.75 : tipo === 'livestock' ? 0.5 : tipo === 'oro' ? 0.08 : 0.22;
  let aptas = 0;
  const celdas = 40;
  for (let y = 0; y < celdas; y++) for (let x = 0; x < celdas; x++) {
    const terreno = evaluarTerreno(elevacion, {
      x: ((x + 0.5) * limites.ancho) / celdas,
      y: ((y + 0.5) * limites.alto) / celdas,
    });
    if (tipo === 'livestock' ? terreno === 'llano'
      : tipo === 'oro' ? terreno === 'montana'
      : tipo === 'cobre' || tipo === 'estano' ? terreno === 'montana' || terreno === 'colina'
      : terreno === 'llano' || terreno === 'colina' || terreno === 'montana') aptas++;
  }
  return Math.min(base, Math.ceil(base * (aptas / (celdas * celdas)) / referencia));
}

/**
 * Nodos minerales de una rareza (Doc 1.1). `colocadosGlobal` se comparte entre llamadas y se MUTA: cada
 * rareza respeta el espaciado frente a lo ya colocado por las anteriores, aplicando su propio
 * `espacioMinimo` (los raros se separan mucho más que los comunes).
 *
 * Las posiciones de todos los tipos se sortean antes que las cantidades, no intercaladas — el orden de
 * consumo del PRNG es parte del contrato: alterarlo cambia todos los mundos ya guardados para una seed
 * dada (ver `WORLDGEN_VERSION`).
 */
export function generarNodosDeRareza(
  rng: RandomFn,
  limites: Limites,
  rareza: Rareza,
  colocadosGlobal: Point[],
  bosques: ZonaBosque[],
  elevacion: CampoElevacion,
  fertilidad: CampoFertilidad,
  rios: readonly RioZona[]
): NodoRecurso[] {
  const { cantidadBase, espacioMinimo } = RECURSO_RAREZA[rareza];
  const tipos = RECURSO_TIPOS_POR_RAREZA[rareza];
  const nodos: NodoRecurso[] = [];
  for (const tipo of tipos) {
    const posiciones = colocarValidos(
      rng,
      limites,
      cuotaPorHabitat(tipo, cantidadBase, limites, elevacion),
      espacioMinimo,
      colocadosGlobal,
      bosques,
      aceptaTerrenoPara(tipo, elevacion, fertilidad, rios)
    );
    const rango = RECURSO_CANTIDAD_NODO[tipo as keyof typeof RECURSO_CANTIDAD_NODO];
    for (const posicion of posiciones) {
      colocadosGlobal.push(posicion);
      nodos.push({
        id: `${tipo}-${nodos.length}-${Math.round(posicion.x)}-${Math.round(posicion.y)}`,
        tipo: tipo as NodoRecurso['tipo'],
        rareza,
        posicion,
        cantidadInicial: Math.round(randRange(rng, rango.min, rango.max)),
      });
    }
  }
  return nodos;
}

/** Livestock (Doc 1.4): fauna libre, fuera del esquema de rareza de los minerales. */
export function generarLivestock(
  rng: RandomFn,
  limites: Limites,
  colocadosGlobal: Point[],
  bosques: ZonaBosque[],
  elevacion: CampoElevacion,
  fertilidad: CampoFertilidad,
  rios: readonly RioZona[]
): NodoRecurso[] {
  const posiciones = colocarValidos(
    rng,
    limites,
    cuotaPorHabitat('livestock', LIVESTOCK.cantidadBase, limites, elevacion),
    LIVESTOCK.espacioMinimo,
    colocadosGlobal,
    bosques,
    aceptaTerrenoPara('livestock', elevacion, fertilidad, rios)
  );
  return posiciones.map((posicion, i) => {
    colocadosGlobal.push(posicion);
    return {
      id: `livestock-${i}-${Math.round(posicion.x)}-${Math.round(posicion.y)}`,
      tipo: 'livestock' as const,
      rareza: 'comun' as const,
      posicion,
      cantidadInicial: randInt(rng, LIVESTOCK.cantidadPorManada.min, LIVESTOCK.cantidadPorManada.max),
    };
  });
}
