// Equivalencia de la fachada `Mapa` con el acceso crudo que hacía el motor.
//
// Cada test contrapone un método del mapa contra una REFERENCIA: la implementación vieja transcrita tal cual
// estaba en `construction.ts` / `settlement.ts` / `asentamientoQuery.ts`, recorriendo los arrays a pelo. Si
// las dos coinciden sobre cientos de consultas aleatorias y varias seeds, migrar los consumidores a la
// fachada (siguiente paso) no puede cambiar el comportamiento del motor.
//
// Importa especialmente el ORDEN de los resultados, no solo el conjunto: el motor se queda con `candidatos[0]`
// para decidir dónde planta una cantera o una leñera. Dos listas con los mismos elementos en distinto orden
// producirían partidas distintas, así que se comparan como secuencias.

import { describe, expect, it } from 'vitest';
import type { NodoRecurso, Point, ZonaBosque } from '../../domain/types';
import { LENERA_POR_BOSQUE } from '../../constants';
import {
  createRng,
  distanciaARioMasCercano,
  evaluarBioma,
  evaluarElevacion,
  evaluarTerreno,
  generarMapa,
  MAPA_DEFAULT,
  type MapaGenerado,
  type RandomFn,
} from '../../worldgen';
import { crearEstadoMapa, crearMapa, type EstadoMapa, type Mapa } from '../mapa';
import { pointInPolygon } from '../geometria';

const SEEDS = [1, 42, 7];
const CONSULTAS = 120;

function generar(seed: number): MapaGenerado {
  return generarMapa({ ancho: MAPA_DEFAULT.ancho, alto: MAPA_DEFAULT.alto, seed });
}

function conMapa(seed: number): { generado: MapaGenerado; estado: EstadoMapa; mapa: Mapa; rng: RandomFn } {
  const generado = generar(seed);
  const estado = crearEstadoMapa();
  // rng propio para las CONSULTAS (no para generar): hace los casos de prueba reproducibles sin tocar el mundo.
  return { generado, estado, mapa: crearMapa(generado, estado), rng: createRng(seed * 977 + 13) };
}

function puntoAleatorio(rng: RandomFn): Point {
  return { x: rng() * MAPA_DEFAULT.ancho, y: rng() * MAPA_DEFAULT.alto };
}

/** Polígono circular equivalente al que produce `computeZonaInfluencia` antes de recortarse (48 segmentos). */
function poligonoCircular(centro: Point, radio: number, segmentos = 48): Point[] {
  return Array.from({ length: segmentos }, (_, i) => {
    const angulo = (i / segmentos) * Math.PI * 2;
    return { x: centro.x + Math.cos(angulo) * radio, y: centro.y + Math.sin(angulo) * radio };
  });
}

function distancia(a: Point, b: Point): number {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

function ids(nodos: readonly NodoRecurso[]): string[] {
  return nodos.map((n) => n.id);
}

/** Deshace un agotamiento provocado por un test: el estado de partida es un simple registro de extraídos. */
function devolverStock(estado: EstadoMapa, nodoId: string): void {
  delete estado.extraido[nodoId];
}

describe('Mapa — equivalencia con el acceso crudo anterior', () => {
  it('nodo(id) devuelve lo mismo que buscar linealmente en el array', () => {
    for (const seed of SEEDS) {
      const { generado, mapa } = conMapa(seed);
      for (const nodo of generado.nodos) {
        expect(mapa.nodo(nodo.id)).toBe(generado.nodos.find((n) => n.id === nodo.id));
      }
      expect(mapa.nodo('no-existe')).toBeUndefined();
      // El motor pasa `edificio.fuenteId`, que es opcional: no debe explotar con undefined.
      expect(mapa.nodo(undefined)).toBeUndefined();
    }
  });

  it('bosque(id) devuelve lo mismo que buscar linealmente en el array', () => {
    for (const seed of SEEDS) {
      const { generado, mapa } = conMapa(seed);
      for (const bosque of generado.bosques) {
        expect(mapa.bosque(bosque.id)).toBe(generado.bosques.find((b) => b.id === bosque.id));
      }
      expect(mapa.bosque('no-existe')).toBeUndefined();
      expect(mapa.bosque(undefined)).toBeUndefined();
    }
  });

  it('nodosEnRadio coincide con el recorrido lineal de `evaluarViabilidadFundacion`', () => {
    // Referencia (settlement.ts): recorrer TODOS los nodos y quedarse con los que cumplen `<= radio`.
    for (const seed of SEEDS) {
      const { generado, mapa, rng } = conMapa(seed);
      for (let i = 0; i < CONSULTAS; i++) {
        const centro = puntoAleatorio(rng);
        const radio = 5 + rng() * 200;
        const referencia = generado.nodos.filter((n) => distancia(n.posicion, centro) <= radio);
        expect(ids(mapa.nodosEnRadio(centro, radio))).toEqual(ids(referencia));
      }
    }
  });

  it('nodosEnRadio no pierde nodos con radios que desbordan el mapa ni con radio 0', () => {
    // Casos límite del índice por celdas: cajas que caen fuera de la rejilla por arriba o por abajo.
    for (const seed of SEEDS) {
      const { generado, mapa } = conMapa(seed);
      const centro = { x: MAPA_DEFAULT.ancho / 2, y: MAPA_DEFAULT.alto / 2 };
      expect(ids(mapa.nodosEnRadio(centro, 5000))).toEqual(ids(generado.nodos));
      expect(mapa.nodosEnRadio(centro, 0)).toEqual([]);
      expect(mapa.nodosEnRadio({ x: -500, y: -500 }, 10)).toEqual([]);
      expect(mapa.nodosEnRadio({ x: MAPA_DEFAULT.ancho + 500, y: MAPA_DEFAULT.alto + 500 }, 10)).toEqual([]);
    }
  });

  it('nodosEnPoligono con filtros y orden coincide con `sitioCercaDeNodo`', () => {
    // Referencia (construction.ts): filter(tipo + cantidad>0 + !excluidas + pointInPolygon).sort(por distancia).
    for (const seed of SEEDS) {
      const { generado, estado, mapa, rng } = conMapa(seed);
      const tipos = ['piedra', 'cobre', 'estano', 'oro', 'livestock'];

      for (let i = 0; i < CONSULTAS; i++) {
        const centro = puntoAleatorio(rng);
        const poligono = poligonoCircular(centro, 30 + rng() * 90);
        const tipo = tipos[Math.floor(rng() * tipos.length)]!;

        // Se agota un nodo al azar y se excluye otro, para ejercitar los dos filtros a la vez.
        const victima = generado.nodos[Math.floor(rng() * generado.nodos.length)]!;
        const agotadoAntes = mapa.stock(victima.id);
        mapa.extraer(victima.id, agotadoAntes);
        const excluir = new Set<string>([generado.nodos[Math.floor(rng() * generado.nodos.length)]!.id]);

        const referencia = generado.nodos
          .filter((n) => n.tipo === tipo && mapa.stock(n.id) > 0 && !excluir.has(n.id) && pointInPolygon(n.posicion, poligono))
          .sort((a, b) => distancia(a.posicion, centro) - distancia(b.posicion, centro));

        const obtenido = mapa.nodosEnPoligono(poligono, { tipo, conStock: true, excluir, ordenarPorCercaniaA: centro });
        expect(ids(obtenido)).toEqual(ids(referencia));

        devolverStock(estado, victima.id);
      }
    }
  });

  it('nodosEnPoligono conserva el orden de generación cuando no se pide ordenar por cercanía', () => {
    // Sin este contrato, el índice por celdas devolvería los nodos agrupados por celda y los empates de
    // distancia se resolverían distinto que con el recorrido lineal — la simulación dejaría de ser reproducible.
    for (const seed of SEEDS) {
      const { generado, mapa, rng } = conMapa(seed);
      for (let i = 0; i < 40; i++) {
        const poligono = poligonoCircular(puntoAleatorio(rng), 60 + rng() * 200);
        const referencia = generado.nodos.filter((n) => pointInPolygon(n.posicion, poligono));
        expect(ids(mapa.nodosEnPoligono(poligono))).toEqual(ids(referencia));
      }
    }
  });

  it('hayBosqueEnRadio coincide con el criterio de `evaluarViabilidadFundacion`', () => {
    for (const seed of SEEDS) {
      const { generado, mapa, rng } = conMapa(seed);
      for (let i = 0; i < CONSULTAS; i++) {
        const centro = puntoAleatorio(rng);
        const radio = 5 + rng() * 120;
        const referencia = generado.bosques.some((b) => distancia(b.centro, centro) < radio + b.radio);
        expect(mapa.hayBosqueEnRadio(centro, radio)).toBe(referencia);
      }
    }
  });

  it('capacidadLeneras coincide con `capacidadLenerasBosque`', () => {
    const referencia = (bosque: ZonaBosque): number => {
      if (bosque.radio >= LENERA_POR_BOSQUE.umbral3) return 3;
      if (bosque.radio >= LENERA_POR_BOSQUE.umbral2) return 2;
      return 1;
    };
    for (const seed of SEEDS) {
      const { generado, mapa } = conMapa(seed);
      for (const bosque of generado.bosques) {
        expect(mapa.capacidadLeneras(bosque.id)).toBe(referencia(bosque));
      }
      expect(mapa.capacidadLeneras('no-existe')).toBe(0);
    }
  });

  it('puntoDeTrabajoEnBosque coincide con `puntoEnBosqueDentroDeZona`', () => {
    // Referencia (construction.ts), transcrita literal.
    const referencia = (bosque: ZonaBosque, poligono: Point[], indiceOcupacion: number): Point | null => {
      const preferido =
        indiceOcupacion === 0
          ? bosque.centro
          : {
              x: bosque.centro.x + Math.cos((indiceOcupacion / 3) * Math.PI * 2) * bosque.radio * 0.4,
              y: bosque.centro.y + Math.sin((indiceOcupacion / 3) * Math.PI * 2) * bosque.radio * 0.4,
            };
      if (pointInPolygon(preferido, poligono)) return preferido;
      const muestrasPorAnillo = 12;
      for (let anillo = 1; anillo <= 3; anillo++) {
        const radio = (bosque.radio * anillo) / 3;
        for (let i = 0; i < muestrasPorAnillo; i++) {
          const angulo = (i / muestrasPorAnillo) * Math.PI * 2;
          const candidato: Point = { x: bosque.centro.x + Math.cos(angulo) * radio, y: bosque.centro.y + Math.sin(angulo) * radio };
          if (pointInPolygon(candidato, poligono)) return candidato;
        }
      }
      return null;
    };

    for (const seed of SEEDS) {
      const { generado, mapa, rng } = conMapa(seed);
      let encontradosNoNulos = 0;
      for (let i = 0; i < CONSULTAS; i++) {
        // Se centra el polígono cerca de un bosque real para que el caso interesante (sí hay punto) ocurra.
        const bosque = generado.bosques[Math.floor(rng() * generado.bosques.length)]!;
        const centro = { x: bosque.centro.x + (rng() - 0.5) * 160, y: bosque.centro.y + (rng() - 0.5) * 160 };
        const poligono = poligonoCircular(centro, 30 + rng() * 90);
        const indice = Math.floor(rng() * 3);

        const obtenido = mapa.puntoDeTrabajoEnBosque(bosque.id, poligono, indice);
        expect(obtenido).toEqual(referencia(bosque, poligono, indice));
        if (obtenido) encontradosNoNulos++;
      }
      // Si todo saliera null el test estaría comparando dos "no hay nada" y no probaría nada.
      expect(encontradosNoNulos).toBeGreaterThan(10);
    }
  });

  it('bosqueParaLenera coincide con `sitioEnBosque`', () => {
    for (const seed of SEEDS) {
      const { generado, mapa, rng } = conMapa(seed);
      let elegidos = 0;

      for (let i = 0; i < CONSULTAS; i++) {
        const bosqueFoco = generado.bosques[Math.floor(rng() * generado.bosques.length)]!;
        const centro = { x: bosqueFoco.centro.x + (rng() - 0.5) * 200, y: bosqueFoco.centro.y + (rng() - 0.5) * 200 };
        const poligono = poligonoCircular(centro, 40 + rng() * 90);

        // Ocupación parcial al azar, para ejercitar el descarte por capacidad.
        const ocupacion = new Map<string, number>();
        for (const b of generado.bosques) {
          if (rng() < 0.3) ocupacion.set(b.id, Math.floor(rng() * 4));
        }

        const referencia = generado.bosques
          .map((bosque) => {
            const ocupadas = ocupacion.get(bosque.id) ?? 0;
            const capacidad = bosque.radio >= LENERA_POR_BOSQUE.umbral3 ? 3 : bosque.radio >= LENERA_POR_BOSQUE.umbral2 ? 2 : 1;
            if (ocupadas >= capacidad) return null;
            const punto = mapa.puntoDeTrabajoEnBosque(bosque.id, poligono, ocupadas);
            return punto ? { bosque, punto } : null;
          })
          .filter((c): c is { bosque: ZonaBosque; punto: Point } => c !== null)
          .sort((a, b) => distancia(a.bosque.centro, centro) - distancia(b.bosque.centro, centro))[0];

        const obtenido = mapa.bosqueParaLenera(poligono, ocupacion, centro);
        expect(obtenido).toEqual(referencia ? { posicion: referencia.punto, fuenteId: referencia.bosque.id } : null);
        if (obtenido) elegidos++;
      }

      expect(elegidos).toBeGreaterThan(10);
    }
  });

  it('fertilidadEn coincide con el campo del mapa generado', () => {
    for (const seed of SEEDS) {
      const { mapa, rng } = conMapa(seed);
      const world = crearMapa(generar(seed));
      for (let i = 0; i < CONSULTAS; i++) {
        const p = puntoAleatorio(rng);
        expect(mapa.fertilidadEn(p)).toBe(world.fertilidadEn(p));
      }
    }
  });

  it('mejorPorFertilidad elige el candidato más fértil, con el mismo desempate que el motor', () => {
    // El motor recorría los candidatos quedándose con el PRIMERO estrictamente mejor (`>`), así que ante
    // empate gana el de menor índice. Se conserva ese criterio.
    for (const seed of SEEDS) {
      const { mapa, rng } = conMapa(seed);
      for (let i = 0; i < 40; i++) {
        const candidatos = Array.from({ length: 12 }, () => puntoAleatorio(rng));
        let mejor: Point | null = null;
        let mejorFertilidad = -1;
        for (const c of candidatos) {
          const f = mapa.fertilidadEn(c);
          if (f > mejorFertilidad) {
            mejorFertilidad = f;
            mejor = c;
          }
        }
        expect(mapa.mejorPorFertilidad(candidatos)).toEqual({ punto: mejor, fertilidad: mejorFertilidad });
      }
      expect(mapa.mejorPorFertilidad([])).toBeNull();
    }
  });

  it('dentroDelMapa coincide con la comprobación de `settlement.ts`', () => {
    const { mapa } = conMapa(1);
    const { ancho, alto } = mapa.limites;
    expect(mapa.dentroDelMapa({ x: 0, y: 0 })).toBe(true);
    expect(mapa.dentroDelMapa({ x: ancho, y: alto })).toBe(true);
    expect(mapa.dentroDelMapa({ x: -0.1, y: 10 })).toBe(false);
    expect(mapa.dentroDelMapa({ x: 10, y: alto + 0.1 })).toBe(false);
  });
});

describe('Mapa — extracción de yacimientos', () => {
  it('extraer descuenta como lo hacía `avanzarConstruccion` y devuelve lo realmente extraído', () => {
    const { generado, mapa } = conMapa(1);
    const nodo = generado.nodos.find((n) => n.tipo === 'piedra')!;
    const inicial = nodo.cantidadInicial;

    expect(mapa.extraer(nodo.id, 10)).toBe(10);
    expect(mapa.stock(nodo.id)).toBe(inicial - 10);

    // Pedir más de lo que queda entrega solo el resto y deja el nodo agotado, nunca en negativo.
    expect(mapa.extraer(nodo.id, inicial * 2)).toBe(inicial - 10);
    expect(mapa.stock(nodo.id)).toBe(0);

    // Un nodo agotado ya no entrega nada.
    expect(mapa.extraer(nodo.id, 5)).toBe(0);
    expect(mapa.stock(nodo.id)).toBe(0);
  });

  it('extraer nunca toca el mundo generado, solo el estado de partida', () => {
    // Es la razón de ser de la separación: el `MapaGenerado` puede compartirse por referencia entre todas
    // las fotos del historial precisamente porque nadie lo escribe.
    const { generado, estado, mapa } = conMapa(1);
    const nodo = generado.nodos.find((n) => n.tipo === 'piedra')!;
    const inicial = nodo.cantidadInicial;

    mapa.extraer(nodo.id, 25);

    expect(nodo.cantidadInicial).toBe(inicial);
    expect(estado.extraido[nodo.id]).toBe(25);
    expect(mapa.stock(nodo.id)).toBe(inicial - 25);
  });

  it('dos mapas sobre el mismo mundo con estados distintos no se contaminan', () => {
    // Exactamente el caso del historial: una foto pasada y la partida en curso comparten `MapaGenerado`.
    const generado = generar(1);
    const partida = crearMapa(generado, crearEstadoMapa());
    const foto = crearMapa(generado, crearEstadoMapa());
    const nodo = generado.nodos[0]!;

    partida.extraer(nodo.id, 40);

    expect(partida.stock(nodo.id)).toBe(nodo.cantidadInicial - 40);
    expect(foto.stock(nodo.id)).toBe(nodo.cantidadInicial);
  });

  it('extraer es inocuo con fuentes inexistentes o cantidades no positivas', () => {
    const { generado, mapa } = conMapa(1);
    const nodo = generado.nodos[0]!;
    const inicial = nodo.cantidadInicial;

    expect(mapa.extraer(undefined, 10)).toBe(0);
    expect(mapa.extraer('no-existe', 10)).toBe(0);
    expect(mapa.extraer(nodo.id, 0)).toBe(0);
    expect(mapa.extraer(nodo.id, -5)).toBe(0);
    expect(mapa.stock(nodo.id)).toBe(inicial);
  });

  it('nodoProductivo refleja el estado de agotamiento', () => {
    const { generado, mapa } = conMapa(1);
    const nodo = generado.nodos[0]!;
    expect(mapa.nodoProductivo(nodo.id)).toBe(true);
    mapa.extraer(nodo.id, nodo.cantidadInicial);
    expect(mapa.nodoProductivo(nodo.id)).toBe(false);
    expect(mapa.nodoProductivo('no-existe')).toBe(false);
    expect(mapa.nodoProductivo(undefined)).toBe(false);
  });

  it('nodosConStock refleja lo que queda, para serializar la partida', () => {
    const { generado, mapa } = conMapa(1);
    const nodo = generado.nodos[0]!;
    mapa.extraer(nodo.id, 30);

    const serializado = mapa.nodosConStock();
    expect(serializado).toHaveLength(generado.nodos.length);
    expect(serializado.find((n) => n.id === nodo.id)?.cantidad).toBe(nodo.cantidadInicial - 30);
  });

  it('las consultas ven el agotamiento inmediatamente (índices y datos no se desincronizan)', () => {
    const { generado, mapa } = conMapa(1);
    const nodo = generado.nodos.find((n) => n.tipo === 'cobre')!;
    const poligono = poligonoCircular(nodo.posicion, 20);

    expect(ids(mapa.nodosEnPoligono(poligono, { tipo: 'cobre', conStock: true }))).toContain(nodo.id);
    mapa.extraer(nodo.id, nodo.cantidadInicial);
    expect(ids(mapa.nodosEnPoligono(poligono, { tipo: 'cobre', conStock: true }))).not.toContain(nodo.id);
    // Sin el filtro de stock sigue estando: el nodo no desaparece del mapa, solo deja de producir.
    expect(ids(mapa.nodosEnPoligono(poligono, { tipo: 'cobre' }))).toContain(nodo.id);
  });
});

describe('Mapa — elevación y terreno (Fase 0.1)', () => {
  it('elevacionEn coincide con evaluarElevacion sobre el campo generado', () => {
    for (const seed of SEEDS) {
      const { generado, mapa, rng } = conMapa(seed);
      for (let i = 0; i < CONSULTAS; i++) {
        const p = puntoAleatorio(rng);
        expect(mapa.elevacionEn(p)).toBe(evaluarElevacion(generado.elevacion, p));
      }
    }
  });

  it('terrenoEn coincide con evaluarTerreno sobre el campo generado', () => {
    for (const seed of SEEDS) {
      const { generado, mapa, rng } = conMapa(seed);
      for (let i = 0; i < CONSULTAS; i++) {
        const p = puntoAleatorio(rng);
        expect(mapa.terrenoEn(p)).toBe(evaluarTerreno(generado.elevacion, p));
      }
    }
  });

  it('biomaEn coincide con evaluarBioma(elevacion, fertilidad, rios, p)', () => {
    for (const seed of SEEDS) {
      const { generado, mapa, rng } = conMapa(seed);
      for (let i = 0; i < CONSULTAS; i++) {
        const p = puntoAleatorio(rng);
        expect(mapa.biomaEn(p)).toBe(evaluarBioma(generado.elevacion, generado.fertilidad, generado.rios, p));
      }
    }
  });
});

describe('Mapa — ríos (Fase 0.1)', () => {
  it('listarRios devuelve los ríos del mundo generado, en orden', () => {
    for (const seed of SEEDS) {
      const { generado, mapa } = conMapa(seed);
      expect(mapa.listarRios()).toEqual(generado.rios);
    }
  });

  it('rioMasCercano coincide con un recorrido lineal por distancia mínima punto-polilínea', () => {
    for (const seed of SEEDS) {
      const { generado, mapa, rng } = conMapa(seed);
      for (let i = 0; i < CONSULTAS; i++) {
        const p = puntoAleatorio(rng);
        let esperado: { rio: (typeof generado.rios)[number]; distancia: number } | null = null;
        for (const rio of generado.rios) {
          const d = distanciaARioMasCercano([rio], p);
          if (!esperado || d < esperado.distancia) esperado = { rio, distancia: d };
        }
        const resultado = mapa.rioMasCercano(p);
        expect(resultado?.rio.id).toBe(esperado?.rio.id);
        expect(resultado?.distancia).toBe(esperado?.distancia);
      }
    }
  });
});
