import type { Point, RegionId } from './tipos';
import { distanciaASegmento, type Limites } from './colocacion';
import type { RandomFn } from './rng';

export interface PasoMontana {
  centro: Point;
  radio: number;
  elevacion: number;
}

export interface Cordillera {
  puntos: Point[];
  ancho: number;
  elevacion: number;
  pasos: PasoMontana[];
}

export interface ConectorGeografico { puntos: Point[]; radio: number; elevacion: number }

export interface EstructuraGeografica {
  perfil: 'continental' | 'archipielago' | 'fluvial';
  cordilleras: Cordillera[];
  conectores?: ConectorGeografico[];
  dimensiones: Limites;
}

const S = (v: number): number => {
  const t = Math.min(1, Math.max(0, v));
  return t * t * (3 - 2 * t);
};

/** Las posiciones de las crestas son la topología prevista. La seed solo desplaza su forma interior. */
export function generarEstructura(rng: RandomFn, limites: Limites, region?: RegionId): EstructuraGeografica {
  const { ancho, alto } = limites;
  const escala = Math.min(ancho, alto);
  const punto = (x: number, y: number): Point => ({ x: x * ancho, y: y * alto });
  const desvio = () => (rng() - 0.5) * 0.07;
  const cordilleraVertical = (x: number, fin: number, pasoY: number): Cordillera => ({
    puntos: [punto(x, 0), punto(x + desvio(), 0.22), punto(x + desvio(), pasoY), punto(x + desvio(), fin)],
    ancho: escala * 0.21,
    elevacion: 0.88,
    pasos: [{ centro: punto(x, pasoY), radio: escala * 0.085, elevacion: 0.51 }],
  });
  const cordilleraHorizontal = (y: number, pasoX: number): Cordillera => ({
    puntos: [punto(0, y), punto(0.25, y + desvio()), punto(pasoX, y + desvio()), punto(1, y)],
    ancho: escala * 0.2,
    elevacion: 0.84,
    pasos: [{ centro: punto(pasoX, y), radio: escala * 0.09, elevacion: 0.52 }],
  });

  switch (region) {
    case 'egeo': return { perfil: 'archipielago', cordilleras: [], dimensiones: limites };
    case 'nilo': return { perfil: 'fluvial', cordilleras: [cordilleraVertical(0.93, 1, 0.46)], dimensiones: limites };
    case 'mesopotamia': return { perfil: 'fluvial', cordilleras: [cordilleraVertical(0.92, 1, 0.57)], dimensiones: limites };
    case 'anatolia': return {
      perfil: 'continental',
      cordilleras: [cordilleraHorizontal(0.16, 0.44), cordilleraHorizontal(0.86, 0.58)], dimensiones: limites,
    };
    case 'greciaContinental': return {
      perfil: 'continental', cordilleras: [cordilleraVertical(0.42, 0.88, 0.49), cordilleraVertical(0.75, 0.52, 0.31)], dimensiones: limites,
    };
    default: return { perfil: 'continental', cordilleras: [cordilleraVertical(0.5, 0.58, 0.48)], dimensiones: limites };
  }
}

/** Aplica una cresta continua. La bajada del paso actúa sobre la altura objetivo, no sobre el ruido fino. */
export function aplicarEstructura(estructura: EstructuraGeografica, p: Point, base: number): number {
  let altura = base;
  for (const cordillera of estructura.cordilleras) {
    const inicio = cordillera.puntos[0]!;
    const fin = cordillera.puntos[cordillera.puntos.length - 1]!;
    const longitud = Math.hypot(fin.x - inicio.x, fin.y - inicio.y) || 1;
    const ejeX = (fin.x - inicio.x) / longitud;
    const ejeY = (fin.y - inicio.y) / longitud;
    let distancia = Infinity;
    for (let i = 1; i < cordillera.puntos.length; i++) {
      distancia = Math.min(distancia, distanciaASegmento(p, cordillera.puntos[i - 1]!, cordillera.puntos[i]!));
    }
    if (distancia >= cordillera.ancho * 1.5) continue;
    let factorPaso = 0;
    for (const paso of cordillera.pasos) {
      // El paso es una franja transversal que cruza toda la cordillera.
      const d = Math.abs((p.x - paso.centro.x) * ejeX + (p.y - paso.centro.y) * ejeY);
      const nucleo = paso.radio * 0.5;
      factorPaso = Math.max(factorPaso, d <= nucleo ? 1 : S(1 - (d - nucleo) / (paso.radio - nucleo)));
    }
    // Estribación amplia y caminable hasta la banda de montaña, seguida de
    // una pared interior. Esta separación deja minerales en la ladera accesible.
    const nucleo = cordillera.ancho * 0.38;
    const t = Math.max(0, Math.min(1, (cordillera.ancho - distancia) / (cordillera.ancho - nucleo)));
    const pesoEstribacion = 0.7 * t + 0.3 * S(t);
    const pesoPared = S(1 - distancia / nucleo);
    const objetivoEstribacion = 0.63 + (0.505 - 0.63) * factorPaso;
    const objetivoCresta = cordillera.elevacion + (0.51 - cordillera.elevacion) * factorPaso;
    const candidato = altura + (objetivoEstribacion - altura) * pesoEstribacion
      + (objetivoCresta - objetivoEstribacion) * pesoPared;
    altura = Math.max(altura, candidato);
    // El corredor atraviesa también las aproximaciones: de otro modo una bahía o
    // una cresta de la guía original puede dejar el puerto aislado tras la pared.
    if (factorPaso > 0) {
      const interior = S((cordillera.ancho * 1.5 - distancia) / (cordillera.ancho * 0.5));
      altura += (0.51 - altura) * factorPaso * interior;
    }
  }
  // Las reparaciones forman valles estrechos, solo sobre tierra. El núcleo
  // practicable mide al menos 150 m; las rampas de 75 m quedan a sus lados.
  for (const conector of estructura.conectores ?? []) {
    if (altura < 0.36) continue;
    let distancia = Infinity;
    for (let i = 1; i < conector.puntos.length; i++) {
      distancia = Math.min(distancia, distanciaASegmento(p, conector.puntos[i - 1]!, conector.puntos[i]!));
    }
    const nucleo = conector.radio * 0.5;
    const peso = distancia <= nucleo ? 1 : S((conector.radio - distancia) / (conector.radio - nucleo));
    altura += (conector.elevacion - altura) * peso;
  }
  return altura;
}
