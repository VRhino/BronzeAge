// MURALLAS — invariantes del trazo (`Consideraciones/Murallas_Definicion.md` §15, Paso 1).
//
// Lo que congela este archivo es la propiedad sin la cual la mecánica no significa nada: **el anillo AÍSLA**.
// Un muro que se puede rodear no es un muro, y el fallo sería visual y tardío (una ciudad que parece amurallada
// y no lo está), nunca un error de tipos — exactamente el patrón que costó caro dos veces en el trazado.
//
// Dos ciudades de fixture, no una: la suite ya demostró (§E6.16) que un solo escenario no ve los bugs de esta
// capa — la seed 99 de `trazado.test.ts` no reproducía el edificio construido encima de una calle que el
// laboratorio encontró a la primera con otra ciudad.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, CeldaMuro, Recinto, RecursoAlmacenado } from '../../domain/types';
import { MURALLA, REJILLA_ASENTAMIENTO, ZONA_INFLUENCIA } from '../../constants';
import { avanzarSimulacion } from '../simulation';
import { createRng } from '../../worldgen';
import {
  celdaMinimaDeEdificio,
  celdasBloqueadasDeRecintos,
  celdasDeEdificio,
  edificiosInternos,
  esDeAfueras,
  redDeCalles,
  tamanoDeEdificio,
  trazadoParaAsentamiento,
} from '../trazado';
import {
  abandonarRecinto,
  abandonarRecintoManualmente,
  areaEncerradaDeRecinto,
  avanzarObraDeRecintos,
  comprometerRecinto,
  comprometerRecintoManualmente,
  costoDeTrazo,
  iniciarMejoraDeRecinto,
  iniciarMejoraDeRecintoManualmente,
  multiplicadorDefensivoDeRecintos,
  RecintoInvalidoError,
  trazadoDeRecinto,
  trazarRecinto,
  upkeepDeRecintos,
} from '../muralla';
import { calcularCostoMantenimiento } from '../mantenimiento';
import {
  contextoDeTest,
  crearEstadoDeTest,
  crearFacciones,
  crearMapaDeterminista,
  fundarAsentamientoDeTest,
  instanteDeTest,
} from './fixtures';

const RECURSOS_HARNESS = ['madera', 'piedra', 'trigo', 'cobre', 'estano', 'oro', 'livestock'] as const;

/** Materiales de sobra, igual que hace el laboratorio al fundar (`lab/src/main.ts`). Aquí se rellenan CADA
 * tick por dos razones: sin ellos la seed 99 colapsa por mantenimiento en el tick 105 —el "colapso de la
 * transición a nivel 2" ya documentado, que no tiene nada que ver con murallas— y porque una geometría no
 * debería romperse el día que alguien recalibre el coste de una Vivienda. Lo que este test congela es la FORMA
 * del recinto, no la economía que lo rodea. */
function conMateriales(a: Asentamiento): Asentamiento {
  const almacen = { ...a.almacen };
  for (const tipo of RECURSOS_HARNESS) almacen[tipo] = { cantidad: 9999, capacidad: 9999 };
  return { ...a, almacen };
}

/** Una ciudad real crecida por el motor. Nivel 2 y Gobernador para que llegue a tener anclas y distritos: un
 * asentamiento recién fundado no tiene tejido que amurallar y no probaría nada. */
function ciudad(seed: number, ticks: number): Asentamiento {
  const mapa = crearMapaDeterminista(seed);
  const { asentamiento, facciones } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
  const preparado: Asentamiento = {
    ...asentamiento,
    nivel: 2,
    nivelActual: 2,
    radioPotencial: ZONA_INFLUENCIA.radioMaximoPorNivel[2] ?? asentamiento.radioPotencial,
    cargos: { ...asentamiento.cargos, gobernadorId: 'jugador-faccion-1-1' },
  };
  let estado = crearEstadoDeTest([conMateriales(preparado)], facciones);
  const rng = createRng(seed);
  for (let tick = 1; tick <= ticks; tick++) {
    estado = avanzarSimulacion(estado, mapa, contextoDeTest(tick, rng));
    const a = estado.asentamientos[0];
    if (!a) throw new Error(`seed ${seed}: el asentamiento colapsó en el tick ${tick}`);
    estado = { ...estado, asentamientos: [conMateriales(a)] };
  }
  return estado.asentamientos[0]!;
}

const CIUDADES: [nombre: string, seed: number, ticks: number][] = [
  ['seed 99 (la de trazado.test.ts)', 99, 200],
  ['seed 60 (la del laboratorio)', 60, 200],
];

const clave = (col: number, row: number) => `${col},${row}`;

/** ¿El anillo separa el Centro Urbano del exterior? Flood fill desde el borde de la caja sin cruzar NINGUNA
 * celda del anillo. Las puertas cuentan aquí como parte del muro: lo que se prueba es que el anillo esté
 * CERRADO, no que sea impermeable a quien pasa por la puerta. */
function anilloCierra(celdas: CeldaMuro[], celdasCentro: string[]): boolean {
  const muro = new Set(celdas.map((c) => clave(c.col, c.row)));
  const cols = celdas.map((c) => c.col);
  const rows = celdas.map((c) => c.row);
  const minCol = Math.min(...cols) - 2;
  const maxCol = Math.max(...cols) + 2;
  const minRow = Math.min(...rows) - 2;
  const maxRow = Math.max(...rows) + 2;
  const objetivo = new Set(celdasCentro);
  const vistas = new Set<string>();
  const pila: [number, number][] = [];
  const empujar = (col: number, row: number) => {
    if (col < minCol || col > maxCol || row < minRow || row > maxRow) return;
    const k = clave(col, row);
    if (muro.has(k) || vistas.has(k)) return;
    vistas.add(k);
    pila.push([col, row]);
  };
  for (let col = minCol; col <= maxCol; col++) {
    empujar(col, minRow);
    empujar(col, maxRow);
  }
  while (pila.length > 0) {
    const [col, row] = pila.pop()!;
    if (objetivo.has(clave(col, row))) return false;
    empujar(col, row - 1);
    empujar(col + 1, row);
    empujar(col, row + 1);
    empujar(col - 1, row);
  }
  return true;
}

describe('murallas — el trazo del recinto', () => {
  for (const [nombre, seed, ticks] of CIUDADES) {
    describe(nombre, () => {
      const asentamiento = ciudad(seed, ticks);
      const trazo = trazarRecinto(asentamiento, { nivel: 2 });

      it('produce un trazo válido', () => {
        expect(trazo).not.toBeNull();
        expect(trazo!.celdas.length).toBeGreaterThan(0);
      });

      // §15 nº 1 — el invariante que da sentido a toda la mecánica.
      it('el anillo CIERRA: no se puede llegar al Centro Urbano desde fuera sin cruzarlo', () => {
        const centro = edificiosInternos(asentamiento.edificios).find((e) => e.tipo === 'centroUrbano')!;
        const celdasCentro = celdasDeEdificio(centro).map((c) => clave(c.col, c.row));
        expect(anilloCierra(trazo!.celdas, celdasCentro)).toBe(true);
      });

      // §15 nº 2 — una celda de muro sobre un edificio sería el bug de §E6.16 trasladado a esta capa.
      it('ninguna celda del anillo pisa un edificio', () => {
        const ocupadas = new Set<string>();
        for (const e of edificiosInternos(asentamiento.edificios)) {
          for (const c of celdasDeEdificio(e)) ocupadas.add(clave(c.col, c.row));
        }
        const pisadas = trazo!.celdas.filter((c) => ocupadas.has(clave(c.col, c.row)));
        expect(pisadas).toEqual([]);
      });

      // §15 nº 3 — la garantía del enunciado ("deja granjas y corrales fuera").
      it('ninguna Granja ni Corral queda dentro del recinto', () => {
        expect(trazo!.afuerasDentro).toEqual([]);
      });

      it('siempre hay al menos una puerta: sin ella la red quedaría partida (§5)', () => {
        expect(trazo!.puertas).toBeGreaterThan(0);
      });

      it('encierra al Centro Urbano y a la mayor parte de la ciudad', () => {
        const urbanos = edificiosInternos(asentamiento.edificios).filter((e) => !esDeAfueras(e.tipo));
        expect(trazo!.dentro).toContain(urbanos.find((e) => e.tipo === 'centroUrbano')!.id);
        expect(trazo!.dentro.length).toBeGreaterThan(trazo!.fuera.length);
      });

      // §15 nº 5 — sin esto no hay laboratorio ni batch reproducibles.
      it('es determinista: la misma ciudad da el mismo trazo', () => {
        expect(trazarRecinto(asentamiento, { nivel: 2 })).toEqual(trazo);
      });

      // Regresión del playtest (2026-08-31): un camino que corría PEGADO al muro antes de salir pintaba 4 y 5
      // celdas seguidas como si fueran un portón enorme — y peor, inflaba el nº de puertas, que es el divisor
      // del bono defensivo. Una puerta es UNA celda por cruce.
      it('una puerta es UNA celda por cruce: no hay dos puertas seguidas en el recorrido', () => {
        const n = trazo!.celdas.length;
        const seguidas: string[] = [];
        for (let i = 0; i < n; i++) {
          const a = trazo!.celdas[i]!;
          const b = trazo!.celdas[(i + 1) % n]!;
          if (a.clase === 'puerta' && b.clase === 'puerta') seguidas.push(`(${a.col},${a.row})-(${b.col},${b.row})`);
        }
        expect(seguidas).toEqual([]);
      });

      it('el recorrido no repite celdas y las cubre todas', () => {
        const claves = trazo!.celdas.map((c) => clave(c.col, c.row));
        expect(new Set(claves).size).toBe(claves.length);
      });
    });
  }

  // §15 nº 8 — el gate duro de §E6.5 aplicado a esta capa: fallar aquí es barato, fallar después de cobrar no.
  it('un asentamiento sin Centro Urbano no tiene trazo, y no revienta', () => {
    const asentamiento = ciudad(99, 200);
    const sinCentro: Asentamiento = {
      ...asentamiento,
      edificios: asentamiento.edificios.filter((e) => e.tipo !== 'centroUrbano'),
    };
    expect(trazarRecinto(sinCentro)).toBeNull();
  });

  it('el nivel 1 (empalizada) no tiene torres; el nivel 2 sí (§7)', () => {
    const asentamiento = ciudad(99, 200);
    expect(trazarRecinto(asentamiento, { nivel: 1 })!.torres).toBe(0);
    expect(trazarRecinto(asentamiento, { nivel: 2 })!.torres).toBeGreaterThan(0);
  });

  it('las torres respetan su separación mínima a lo largo del anillo', () => {
    const trazo = trazarRecinto(ciudad(99, 200), { nivel: 2 })!;
    const paso = MURALLA.pasoTorres[2]!;
    let ultima = -Infinity;
    for (let i = 0; i < trazo.celdas.length; i++) {
      if (trazo.celdas[i]!.clase !== 'torre') continue;
      if (ultima > -Infinity) expect(i - ultima).toBeGreaterThanOrEqual(paso);
      ultima = i;
    }
  });

  // §15 nº 6 — ni una celda gratis, ni una cobrada dos veces.
  it('el coste es exactamente la suma de las tarifas por clase de celda', () => {
    const celdas: CeldaMuro[] = [
      { col: 0, row: 0, clase: 'muro' },
      { col: 1, row: 0, clase: 'puerta' },
      { col: 2, row: 0, clase: 'torre' },
    ];
    const tarifa = MURALLA.tarifaPorCelda[1]!;
    const esperado = 1 + MURALLA.factorPuerta + MURALLA.factorTorre;
    expect(costoDeTrazo(celdas, 1)['madera']).toBe((tarifa['madera'] ?? 0) * esperado);
  });

  it('el contrato de dibujo fusiona las celdas en tiradas y separa las tres clases', () => {
    const trazo = trazarRecinto(ciudad(99, 200), { nivel: 2 })!;
    const dibujo = trazadoDeRecinto(trazo, 2);
    expect(dibujo.integridad).toBe(0);
    // `<=` y no `===`: dos puertas de cruces distintos pueden caer en celdas contiguas de la misma fila y
    // fusionarse en un rectángulo. No pasa en estas dos ciudades, pero afirmarlo sería afirmar algo que la
    // geometría no garantiza.
    expect(dibujo.puertas.length).toBeLessThanOrEqual(trazo.puertas);
    const celdasDibujadas =
      dibujo.muro.length + dibujo.puertas.length + dibujo.torres.length;
    expect(celdasDibujadas).toBeGreaterThan(0);
    expect(celdasDibujadas).toBeLessThanOrEqual(trazo.celdas.length);
  });
});

// --- Paso 2a: el recinto COMPROMETIDO, y lo que el motor tiene que respetar a partir de ahí ---
//
// Esto es lo que el Paso 1 no hacía y por eso el usuario veía edificios encima del anillo: mientras el trazo
// era solo una consulta, `sueloOcupado` no sabía nada de él. Estos tests congelan las dos mitades del cambio:
// el muro OCUPA (nadie construye encima) y las puertas SON RED (la ciudad no se parte en dos).
describe('murallas — el recinto comprometido', () => {
  /** Ciudad con muralla y 150 ticks MÁS de construcción encima. Los ticks posteriores son el punto: lo que
   * falla no es comprometer, es todo lo que la ciudad construye después. */
  function ciudadAmurallada(seed: number): { antes: Asentamiento; despues: Asentamiento } {
    const mapa = crearMapaDeterminista(seed);
    const { asentamiento, facciones } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
    const preparado: Asentamiento = {
      ...asentamiento,
      nivel: 2,
      nivelActual: 2,
      radioPotencial: ZONA_INFLUENCIA.radioMaximoPorNivel[2] ?? asentamiento.radioPotencial,
      cargos: { ...asentamiento.cargos, gobernadorId: 'jugador-faccion-1-1' },
    };
    let estado = crearEstadoDeTest([conMateriales(preparado)], facciones);
    const rng = createRng(seed);
    const paso = (tick: number) => {
      estado = avanzarSimulacion(estado, mapa, contextoDeTest(tick, rng));
      const a = estado.asentamientos[0];
      if (!a) throw new Error(`seed ${seed}: colapso en el tick ${tick}`);
      estado = { ...estado, asentamientos: [conMateriales(a)] };
    };
    for (let tick = 1; tick <= 150; tick++) paso(tick);
    const antes = comprometerRecinto(estado.asentamientos[0]!, 2, instanteDeTest(150));
    estado = { ...estado, asentamientos: [antes] };
    for (let tick = 151; tick <= 300; tick++) paso(tick);
    return { antes, despues: estado.asentamientos[0]! };
  }

  for (const seed of [99, 60]) {
    describe(`seed ${seed}`, () => {
      const { antes, despues } = ciudadAmurallada(seed);
      const recinto = despues.recintos![0]!;
      const bloqueadas = celdasBloqueadasDeRecintos(despues.recintos!);

      it('el recinto sobrevive intacto a 150 ticks de construcción', () => {
        expect(despues.recintos).toHaveLength(1);
        expect(recinto.celdas).toEqual(antes.recintos![0]!.celdas);
      });

      // NO se exige que crezca: una ciudad puede llenarse de verdad dentro de su recinto, y entonces lo
      // correcto es que deje de construir hasta ampliar o hasta que el arrabal funcione (Paso 4). Lo que sí se
      // exige es que el motor SEPA que está llena. La diferencia importa: mientras `anclaActivaParaCategoria`
      // no veía las celdas de muro, daba el Centro Urbano por usable con huecos que en realidad eran el propio
      // anillo, nunca lo marcaba lleno, jamás nacía una Plaza de relevo y la ciudad se quedaba en un deadlock
      // silencioso — 68 edificios sin muralla contra 47 con ella, y la población congelada a la mitad.
      it('no se queda en deadlock: o sigue creciendo, o reconoce que su núcleo está lleno', () => {
        const crecio = despues.edificios.length > antes.edificios.length;
        const reconoceLleno = despues.edificios.some((e) => e.anclaLlena);
        expect(crecio || reconoceLleno, 'ni creció ni marcó ningún ancla como llena').toBe(true);
      });

      // EL bug que reportó el usuario: "los edificios se montan sobre las celdas de la muralla".
      it('ningún edificio construido DESPUÉS pisa una celda del muro', () => {
        const pisadas: string[] = [];
        for (const e of edificiosInternos(despues.edificios)) {
          for (const c of celdasDeEdificio(e)) {
            if (bloqueadas.has(clave(c.col, c.row))) pisadas.push(`${e.tipo} en (${c.col},${c.row})`);
          }
        }
        expect(pisadas).toEqual([]);
      });

      it('ninguna calle ni camino nace encima del muro', () => {
        const red = redDeCalles(despues.id, despues.edificios, despues.recintos);
        const encima = [...red.calles, ...red.caminos].filter((k) => bloqueadas.has(k));
        expect(encima).toEqual([]);
      });

      // §15 nº 4 — el invariante de la Etapa 6 que esta mecánica es capaz de romper.
      it('la red sigue siendo UN único componente conexo', () => {
        const red = redDeCalles(despues.id, despues.edificios, despues.recintos);
        const todas = new Set([...red.calles, ...red.caminos]);
        const arranque = todas.values().next().value as string;
        const vistas = new Set([arranque]);
        const pila = [arranque];
        while (pila.length > 0) {
          const [col, row] = pila.pop()!.split(',').map(Number) as [number, number];
          for (const [dc, dr] of [[0, -1], [1, 0], [0, 1], [-1, 0]] as [number, number][]) {
            const k = clave(col + dc, row + dr);
            if (todas.has(k) && !vistas.has(k)) {
              vistas.add(k);
              pila.push(k);
            }
          }
        }
        expect(vistas.size).toBe(todas.size);
      });

      it('todo edificio conserva una celda de calle o camino adyacente', () => {
        const red = redDeCalles(despues.id, despues.edificios, despues.recintos);
        const sinSalida: string[] = [];
        for (const e of edificiosInternos(despues.edificios)) {
          const min = celdaMinimaDeEdificio(e);
          const tam = tamanoDeEdificio(e);
          let conFrente = false;
          for (let dc = 0; dc < tam.ancho && !conFrente; dc++) {
            for (const row of [min.row - 1, min.row + tam.alto]) {
              const k = clave(min.col + dc, row);
              if (red.calles.has(k) || red.caminos.has(k)) conFrente = true;
            }
          }
          for (let dr = 0; dr < tam.alto && !conFrente; dr++) {
            for (const col of [min.col - 1, min.col + tam.ancho]) {
              const k = clave(col, min.row + dr);
              if (red.calles.has(k) || red.caminos.has(k)) conFrente = true;
            }
          }
          if (!conFrente) sinSalida.push(`${e.tipo} (${e.id})`);
        }
        expect(sinSalida).toEqual([]);
      });

      // Una puerta es un HUECO en el anillo, no una calle prefabricada: no bloquea, y el replay la usa cuando
      // necesita cruzar. Sembrarlas como calle dejaba islas sueltas y partía la red (ver `redDeCalles`).
      it('ninguna puerta bloquea: son la única forma de cruzar el anillo', () => {
        for (const celda of recinto.celdas) {
          if (celda.clase !== 'puerta') continue;
          expect(bloqueadas.has(clave(celda.col, celda.row))).toBe(false);
        }
      });

      it('el trazado que se sirve para dibujar incluye el recinto', () => {
        const dibujo = trazadoParaAsentamiento(despues);
        expect(dibujo.murallas).toHaveLength(1);
        expect(dibujo.murallas[0]!.integridad).toBe(1);
        expect(dibujo.murallas[0]!.nivel).toBe(2);
      });
    });
  }

  it('no se puede comprometer un segundo recinto que no contenga al primero', () => {
    const a = ciudad(99, 200);
    const conMuro = comprometerRecinto(a, 1, instanteDeTest(200));
    // El mismo trazo sobre la misma ciudad no CONTIENE al anterior: es idéntico, así que se rechaza.
    expect(() => comprometerRecinto(conMuro, 1, instanteDeTest(201))).toThrow(RecintoInvalidoError);
  });
});

// --- Paso 2b: la obra se levanta celda a celda y se paga sobre la marcha ---
describe('murallas — la obra', () => {
  const almacenCon = (madera: number, piedra: number): Record<string, RecursoAlmacenado> => ({
    madera: { cantidad: madera, capacidad: 99999 },
    piedra: { cantidad: piedra, capacidad: 99999 },
  });

  const recintoDe = (celdas: CeldaMuro[], nivel = 1): Recinto => ({
    id: 'r1',
    nivel,
    celdas,
    avance: -1,
    comprometidoEn: instanteDeTest(0),
  });

  const anillo = (n: number): CeldaMuro[] =>
    Array.from({ length: n }, (_, i) => ({ col: i, row: 0, clase: i === 0 ? ('puerta' as const) : ('muro' as const) }));

  it('comprometer es GRATIS y deja el recinto sin una sola celda en pie', () => {
    const a = ciudad(99, 200);
    const con = comprometerRecinto(a, 1, instanteDeTest(200));
    expect(con.recintos![0]!.avance).toBe(-1);
    expect(con.recintos![0]!.completadoEn).toBeUndefined();
    expect(con.almacen).toEqual(a.almacen);
  });

  it('levanta `celdasPorMinuto` celdas por tick y cobra cada una', () => {
    const tarifa = MURALLA.tarifaPorCelda[1]!;
    const almacen = almacenCon(9999, 9999);
    const paso = avanzarObraDeRecintos([recintoDe(anillo(10))], almacen, {}, instanteDeTest(1));
    expect(paso.recintos[0]!.avance).toBe(MURALLA.celdasPorMinuto - 1);
    // La primera celda del recorrido es la puerta: cuesta su tarifa × factorPuerta.
    const gastado = 9999 - (paso.almacen['madera']?.cantidad ?? 0);
    expect(gastado).toBe((tarifa['madera'] ?? 0) * MURALLA.factorPuerta * MURALLA.celdasPorMinuto);
  });

  it('sin materiales la obra NO avanza, pero tampoco se cancela ni acumula deuda', () => {
    const recinto = recintoDe(anillo(10));
    const paso = avanzarObraDeRecintos([recinto], almacenCon(0, 0), {}, instanteDeTest(1));
    expect(paso.recintos[0]!.avance).toBe(-1);
    expect(paso.recintos[0]).toBe(recinto); // ni siquiera se clona: el tick no pasó nada
    expect(paso.eventos).toEqual([]);
  });

  it('la reserva de mantenimiento frena la obra antes que el almacén vacío', () => {
    const almacen = almacenCon(100, 100);
    const sinReserva = avanzarObraDeRecintos([recintoDe(anillo(10))], almacen, {}, instanteDeTest(1));
    const conReserva = avanzarObraDeRecintos([recintoDe(anillo(10))], almacen, { madera: 100 }, instanteDeTest(1));
    expect(sinReserva.recintos[0]!.avance).toBeGreaterThan(-1);
    expect(conReserva.recintos[0]!.avance).toBe(-1);
  });

  it('al cerrarse el anillo marca `completadoEn` y emite el evento de dominio', () => {
    let recintos = [recintoDe(anillo(3))];
    let almacen = almacenCon(9999, 9999);
    let eventos: unknown[] = [];
    for (let tick = 1; tick <= 10; tick++) {
      const paso = avanzarObraDeRecintos(recintos, almacen, {}, instanteDeTest(tick));
      recintos = paso.recintos;
      almacen = paso.almacen;
      eventos = [...eventos, ...paso.eventos];
    }
    expect(recintos[0]!.avance).toBe(2);
    expect(recintos[0]!.completadoEn).toBeDefined();
    expect(eventos).toHaveLength(1);
    expect((eventos[0] as { codigo: string }).codigo).toBe('construccion.recinto_completado');
  });

  it('una obra terminada no vuelve a cobrar nada', () => {
    const completo: Recinto = { ...recintoDe(anillo(3)), avance: 2 };
    const almacen = almacenCon(500, 500);
    const paso = avanzarObraDeRecintos([completo], almacen, {}, instanteDeTest(1));
    expect(paso.almacen).toBe(almacen);
    expect(paso.recintos[0]).toBe(completo);
  });

  // La escotilla de seguridad: sin ella, una obra atascada bloquearía su franja de suelo para siempre.
  it('se puede abandonar un recinto incompleto, y su suelo queda libre', () => {
    const a = comprometerRecinto(ciudad(99, 200), 1, instanteDeTest(200));
    const abandonado = abandonarRecinto(a, a.recintos![0]!.id);
    expect(abandonado.recintos).toEqual([]);
    expect(celdasBloqueadasDeRecintos(abandonado.recintos ?? []).size).toBe(0);
  });

  it('un recinto TERMINADO no se puede abandonar: en Fase 0 no se destruye nada', () => {
    const a = comprometerRecinto(ciudad(99, 200), 1, instanteDeTest(200));
    const terminado: Asentamiento = {
      ...a,
      recintos: [{ ...a.recintos![0]!, avance: a.recintos![0]!.celdas.length - 1 }],
    };
    expect(() => abandonarRecinto(terminado, terminado.recintos![0]!.id)).toThrow(RecintoInvalidoError);
  });

  // El punto de toda la obra progresiva: el anillo se ve cerrarse, no aparece de golpe.
  it('el dibujo solo trae las celdas YA levantadas', () => {
    const a = comprometerRecinto(ciudad(99, 200), 1, instanteDeTest(200));
    expect(trazadoParaAsentamiento(a).murallas[0]!.integridad).toBe(0);
    const aMedias: Asentamiento = { ...a, recintos: [{ ...a.recintos![0]!, avance: 9 }] };
    const dibujo = trazadoParaAsentamiento(aMedias).murallas[0]!;
    const celdasDibujadas = dibujo.muro.concat(dibujo.puertas, dibujo.torres).length;
    expect(celdasDibujadas).toBeGreaterThan(0);
    expect(dibujo.integridad).toBeCloseTo(10 / a.recintos![0]!.celdas.length, 5);
  });
});

// --- El anillo pendiente se DIBUJA: ocupa suelo, así que no puede ser invisible ---
describe('murallas — la obra pendiente es visible', () => {
  it('las celdas todavía no levantadas salen en `planificado`, no desaparecen', () => {
    const a = comprometerRecinto(ciudad(99, 200), 1, instanteDeTest(200));
    const recinto = a.recintos![0]!;

    const reciénComprometido = trazadoParaAsentamiento(a).murallas[0]!;
    const levantadas = reciénComprometido.muro.concat(reciénComprometido.puertas, reciénComprometido.torres);
    expect(levantadas).toEqual([]); // nada en pie todavía
    expect(reciénComprometido.planificado.length).toBeGreaterThan(0); // pero TODO se ve como obra

    const aMedias: Asentamiento = { ...a, recintos: [{ ...recinto, avance: 9 }] };
    const dibujo = trazadoParaAsentamiento(aMedias).murallas[0]!;
    expect(dibujo.muro.concat(dibujo.puertas, dibujo.torres).length).toBeGreaterThan(0);
    expect(dibujo.planificado.length).toBeGreaterThan(0);
  });

  // El invariante que evita el peor malentendido posible: suelo bloqueado que se ve vacío.
  it('toda celda que ocupa suelo se dibuja: levantada o pendiente, ninguna es invisible', () => {
    const a = comprometerRecinto(ciudad(99, 200), 1, instanteDeTest(200));
    for (const avance of [-1, 0, 20, a.recintos![0]!.celdas.length - 1]) {
      const estado: Asentamiento = { ...a, recintos: [{ ...a.recintos![0]!, avance }] };
      const dibujo = trazadoParaAsentamiento(estado).murallas[0]!;
      const dibujadas = [...dibujo.muro, ...dibujo.puertas, ...dibujo.torres, ...dibujo.planificado].reduce(
        (celdas, r) => celdas + (r.ancho / REJILLA_ASENTAMIENTO.tamanoCelda) * (r.alto / REJILLA_ASENTAMIENTO.tamanoCelda),
        0
      );
      expect(dibujadas, `avance ${avance}`).toBe(a.recintos![0]!.celdas.length);
    }
  });
});

// --- Paso 2c: el mando manual (Gobernador/Maestro de Obras), §8 del doc ---
describe('murallas — el mando manual (Paso 2c)', () => {
  it('comprometerRecintoManualmente exige el cargo indicado asignado', () => {
    const base = ciudad(99, 200);
    const nivel3SinCargos: Asentamiento = {
      ...base,
      nivel: 3,
      nivelActual: 3,
      cargos: { ...base.cargos, gobernadorId: null, maestroObrasId: null },
    };
    expect(() => comprometerRecintoManualmente(nivel3SinCargos, 'gobernador', 1, instanteDeTest(200))).toThrow(RecintoInvalidoError);
  });

  it('comprometerRecintoManualmente exige MURALLA.nivelMinimoConstruccion (el fixture está en nivel 2)', () => {
    const base = ciudad(99, 200);
    expect(() => comprometerRecintoManualmente(base, 'gobernador', 1, instanteDeTest(200))).toThrow(RecintoInvalidoError);
  });

  it('con cargo y nivel suficientes, delega en comprometerRecinto', () => {
    const base = ciudad(99, 200);
    const nivel3: Asentamiento = { ...base, nivel: 3, nivelActual: 3 };
    const conMuro = comprometerRecintoManualmente(nivel3, 'gobernador', 1, instanteDeTest(200));
    expect(conMuro.recintos).toHaveLength(1);
  });

  it('el Maestro de Obras también puede comprometer, sin Gobernador asignado', () => {
    const base = ciudad(99, 200);
    const soloMaestro: Asentamiento = {
      ...base,
      nivel: 3,
      nivelActual: 3,
      cargos: { ...base.cargos, gobernadorId: null, maestroObrasId: 'jugador-maestro' },
    };
    const conMuro = comprometerRecintoManualmente(soloMaestro, 'maestroObras', 1, instanteDeTest(200));
    expect(conMuro.recintos).toHaveLength(1);
  });

  it('abandonarRecintoManualmente exige Gobernador — el Maestro de Obras NO basta (§8: es decisión de gobierno)', () => {
    const base = ciudad(99, 200);
    const nivel3: Asentamiento = { ...base, nivel: 3, nivelActual: 3 };
    const conMuro = comprometerRecintoManualmente(nivel3, 'gobernador', 1, instanteDeTest(200));
    const soloMaestro: Asentamiento = { ...conMuro, cargos: { ...conMuro.cargos, gobernadorId: null, maestroObrasId: 'jugador-maestro' } };
    expect(() => abandonarRecintoManualmente(soloMaestro, conMuro.recintos![0]!.id)).toThrow(RecintoInvalidoError);
  });

  it('con Gobernador asignado, abandonarRecintoManualmente delega en abandonarRecinto', () => {
    const base = ciudad(99, 200);
    const nivel3: Asentamiento = { ...base, nivel: 3, nivelActual: 3 };
    const conMuro = comprometerRecintoManualmente(nivel3, 'gobernador', 1, instanteDeTest(200));
    const abandonado = abandonarRecintoManualmente(conMuro, conMuro.recintos![0]!.id);
    expect(abandonado.recintos).toEqual([]);
  });
});

// --- Área encerrada de un recinto ya comprometido: medición en batch del eje fortaleza↔metrópoli (Paso 2c) ---
describe('murallas — área encerrada de un recinto congelado', () => {
  it('coincide con TrazoRecinto.areaEncerrada del trazo que se comprometió', () => {
    const a = ciudad(99, 200);
    const trazo = trazarRecinto(a, { nivel: 1 })!;
    const conMuro = comprometerRecinto(a, 1, instanteDeTest(200));
    expect(areaEncerradaDeRecinto(conMuro, conMuro.recintos![0]!)).toBe(trazo.areaEncerrada);
  });

  it('no cambia con la obra a medias: el anillo ya está congelado, solo falta levantarlo', () => {
    const a = ciudad(99, 200);
    const conMuro = comprometerRecinto(a, 1, instanteDeTest(200));
    const recinto = conMuro.recintos![0]!;
    const completa = areaEncerradaDeRecinto(conMuro, recinto);
    const aMedias: Asentamiento = { ...conMuro, recintos: [{ ...recinto, avance: 5 }] };
    expect(areaEncerradaDeRecinto(aMedias, recinto)).toBe(completa);
  });
});

/** Ciudad con un recinto ya CERRADO (forzado a `avance` máximo, sin gastar 200 ticks de obra en levantarlo:
 * lo que estas pruebas ejercitan es la mejora/ampliación, no la obra progresiva, que ya tiene su propia
 * cobertura en `murallas — la obra`). */
function conRecintoCompleto(seed: number, ticks: number, nivel = 1): Asentamiento {
  const a = comprometerRecinto(ciudad(seed, ticks), nivel, instanteDeTest(ticks));
  const recinto = a.recintos![0]!;
  return { ...a, recintos: [{ ...recinto, avance: recinto.celdas.length - 1, completadoEn: instanteDeTest(ticks) }] };
}

// --- Paso 3: ampliación (§10) ---
describe('murallas — ampliación (Paso 3, §10)', () => {
  // La otra mitad del gate de §10 ("recinto exterior completo") no necesita chequeo propio: el guardia
  // genérico de arriba ("ya hay un recinto en obra") ya cubre CUALQUIER recinto incompleto, éste incluido.
  it('rechaza ampliar si el recinto exterior actual no está completo (vía el guardia genérico de obra)', () => {
    const a = comprometerRecinto(ciudad(99, 200), 1, instanteDeTest(200)); // avance -1, recién comprometido
    expect(() => comprometerRecinto(a, 1, instanteDeTest(201))).toThrow(RecintoInvalidoError);
  });

  it('rechaza ampliar sin al menos MURALLA.arrabalMinimo edificios extramuros', () => {
    const a = conRecintoCompleto(99, 200);
    // Recién amurallada: no tiene de sobra edificios urbanos fuera del anillo por defecto.
    expect(() => comprometerRecinto(a, 1, instanteDeTest(201))).toThrow(/edificios extramuros/);
  });

  it('con MURALLA.arrabalMinimo edificios extramuros, pasa el gate de arrabal (se rechaza luego por otra razón)', () => {
    const a = conRecintoCompleto(99, 200);
    const recinto = a.recintos![0]!;
    // Sintetiza viviendas bien fuera de la caja del anillo — basta con que no caigan dentro de su interior
    // para CONTAR como extramuros (`edificiosExtramurosDe` no exige que estén conectadas a la red).
    const colBase = Math.min(...recinto.celdas.map((c) => c.col));
    const rowExterior = Math.max(...recinto.celdas.map((c) => c.row)) + 20;
    const plantilla = a.edificios.find((e) => e.tipo === 'vivienda');
    expect(plantilla, 'la ciudad de fixture debería tener al menos una vivienda a tick 200').toBeTruthy();
    const extra = Array.from({ length: MURALLA.arrabalMinimo }, (_, i) => ({
      ...plantilla!,
      id: `extramuros-${i}`,
      posicion: {
        x: (colBase + i * 3) * REJILLA_ASENTAMIENTO.tamanoCelda,
        y: rowExterior * REJILLA_ASENTAMIENTO.tamanoCelda,
      },
    }));
    const conArrabal: Asentamiento = { ...a, edificios: [...a.edificios, ...extra] };
    // El gate de arrabal ya no dispara — lo que rechaza ahora es la CONTENCIÓN, porque el trazo nuevo no ve
    // esas viviendas sintéticas como conectadas a la red y no crece para encerrarlas (harina de otro costal:
    // eso es geometría de `trazarRecinto`, no el gate de §10).
    expect(() => comprometerRecinto(conArrabal, 1, instanteDeTest(201))).not.toThrow(/edificios extramuros/);
  });
});

// --- Paso 3: mejora de nivel (§7) ---
describe('murallas — la mejora de nivel (Paso 3, §7)', () => {
  it('no se puede mejorar un recinto incompleto: hay que cerrar el anillo primero', () => {
    const a = comprometerRecinto(ciudad(99, 200), 1, instanteDeTest(200));
    expect(() => iniciarMejoraDeRecinto(a, a.recintos![0]!.id)).toThrow(RecintoInvalidoError);
  });

  it('no se puede mejorar un recinto ya en su nivel máximo', () => {
    const a = conRecintoCompleto(99, 200, MURALLA.nivelMaximo);
    expect(() => iniciarMejoraDeRecinto(a, a.recintos![0]!.id)).toThrow(RecintoInvalidoError);
  });

  it('no se puede mejorar dos veces a la vez', () => {
    const a = conRecintoCompleto(99, 200, 1);
    const iniciada = iniciarMejoraDeRecinto(a, a.recintos![0]!.id);
    expect(() => iniciarMejoraDeRecinto(iniciada, iniciada.recintos![0]!.id)).toThrow(RecintoInvalidoError);
  });

  it('con un recinto completo, reinicia el avance y marca `mejorandoA` sin subir el nivel todavía', () => {
    const a = conRecintoCompleto(99, 200, 1);
    const iniciada = iniciarMejoraDeRecinto(a, a.recintos![0]!.id);
    expect(iniciada.recintos![0]!.avance).toBe(-1);
    expect(iniciada.recintos![0]!.mejorandoA).toBe(2);
    expect(iniciada.recintos![0]!.nivel).toBe(1); // sube al TERMINAR la obra, no al empezarla
  });

  it('iniciarMejoraDeRecintoManualmente exige el cargo indicado asignado', () => {
    const base = conRecintoCompleto(99, 200, 1);
    const sinCargos: Asentamiento = { ...base, cargos: { ...base.cargos, gobernadorId: null, maestroObrasId: null } };
    expect(() => iniciarMejoraDeRecintoManualmente(sinCargos, 'gobernador', base.recintos![0]!.id)).toThrow(RecintoInvalidoError);
  });

  it('al completar la obra de mejora: sube de nivel, reclasifica torres, no toca puertas ni el largo del anillo', () => {
    const a = conRecintoCompleto(99, 200, 1);
    const original = a.recintos![0]!;
    const puertasAntes = original.celdas.filter((c) => c.clase === 'puerta').map((c) => `${c.col},${c.row}`).sort();
    expect(original.celdas.filter((c) => c.clase === 'torre')).toHaveLength(0); // nivel 1 no tiene torres (§6)

    let asentamiento = iniciarMejoraDeRecinto(a, original.id);
    const almacen: Record<string, RecursoAlmacenado> = {
      madera: { cantidad: 999999, capacidad: 999999 },
      piedra: { cantidad: 999999, capacidad: 999999 },
    };
    let eventos: { codigo: string }[] = [];
    for (let tick = 0; tick < original.celdas.length + 5; tick++) {
      const obra = avanzarObraDeRecintos(asentamiento.recintos!, almacen, {}, instanteDeTest(tick), asentamiento.edificios);
      asentamiento = { ...asentamiento, recintos: obra.recintos };
      eventos = [...eventos, ...(obra.eventos as { codigo: string }[])];
    }
    const final = asentamiento.recintos![0]!;
    expect(final.nivel).toBe(2);
    expect(final.mejorandoA).toBeUndefined();
    expect(final.avance).toBe(final.celdas.length - 1);
    expect(final.celdas.length).toBe(original.celdas.length); // §6: "no se retraza nada"

    const puertasDespues = final.celdas.filter((c) => c.clase === 'puerta').map((c) => `${c.col},${c.row}`).sort();
    expect(puertasDespues).toEqual(puertasAntes); // §5.1: las puertas no cambian nunca, ni al mejorar

    expect(final.celdas.filter((c) => c.clase === 'torre').length).toBeGreaterThan(0); // nivel 2 SÍ tiene torres
    expect(eventos.some((e) => e.codigo === 'construccion.recinto_mejorado')).toBe(true);
    expect(eventos.filter((e) => e.codigo === 'construccion.recinto_completado')).toHaveLength(0);
  });

  it('sin `edificios` (llamador que no los necesita) sigue subiendo de nivel, pero sin reclasificar torres', () => {
    const a = conRecintoCompleto(99, 200, 1);
    const iniciada = iniciarMejoraDeRecinto(a, a.recintos![0]!.id);
    const almacen: Record<string, RecursoAlmacenado> = {
      madera: { cantidad: 999999, capacidad: 999999 },
      piedra: { cantidad: 999999, capacidad: 999999 },
    };
    let recintos = iniciada.recintos!;
    for (let tick = 0; tick < recintos[0]!.celdas.length + 5; tick++) {
      recintos = avanzarObraDeRecintos(recintos, almacen, {}, instanteDeTest(tick)).recintos; // sin `edificios`
    }
    expect(recintos[0]!.nivel).toBe(2);
    expect(recintos[0]!.celdas.filter((c) => c.clase === 'torre')).toHaveLength(0);
  });
});

// --- Paso 3b: defensa y upkeep — la razón de ser de toda la mecánica (§0, §16) ---
describe('murallas — defensa y upkeep (Paso 3b, §16)', () => {
  function recintoSintetico(nivel: number, puertas: number, totalCeldas: number, avance: number, mejorandoA?: number): Recinto {
    const celdas: CeldaMuro[] = Array.from({ length: totalCeldas }, (_, i) => ({
      col: i,
      row: 0,
      clase: i < puertas ? ('puerta' as const) : ('muro' as const),
    }));
    return { id: 'r', nivel, celdas, avance, comprometidoEn: instanteDeTest(0), ...(mejorandoA !== undefined ? { mejorandoA } : {}) };
  }

  describe('multiplicadorDefensivoDeRecintos', () => {
    it('sin recintos, no hay bono: 1 (neutro) — combate exactamente igual que sin muro', () => {
      expect(multiplicadorDefensivoDeRecintos([])).toBe(1);
    });

    it('recinto completo con 1 puerta: el multiplicador ES el bono base del nivel', () => {
      const r = recintoSintetico(3, 1, 10, 9); // avance 9 de 10 celdas => integridad 1
      expect(multiplicadorDefensivoDeRecintos([r])).toBeCloseTo(MURALLA.bonoDefensaPorNivel[3]!, 10);
    });

    it('un anillo a medio cerrar defiende proporcionalmente menos: se entra por el hueco de obra', () => {
      const r = recintoSintetico(3, 1, 10, 4); // avance 4 de 10 => integridad 0.5
      const esperado = 1 + (MURALLA.bonoDefensaPorNivel[3]! - 1) * 0.5;
      expect(multiplicadorDefensivoDeRecintos([r])).toBeCloseTo(esperado, 10);
    });

    it('más puertas reparten la ventaja entre más frentes: el multiplicador baja', () => {
      const unaPuerta = recintoSintetico(3, 1, 10, 9);
      const cuatroPuertas = recintoSintetico(3, 4, 10, 9);
      expect(multiplicadorDefensivoDeRecintos([cuatroPuertas])).toBeLessThan(multiplicadorDefensivoDeRecintos([unaPuerta]));
    });

    it('nunca baja de 1: un muro jamás perjudica al defensor, por muchas puertas que tenga', () => {
      const r = recintoSintetico(3, 8, 10, 9);
      expect(multiplicadorDefensivoDeRecintos([r])).toBeGreaterThanOrEqual(1);
    });

    it('tras una ampliación, usa el recinto EXTERIOR (el último) — el viejo ya no defiende por sí solo (§10)', () => {
      const viejo = recintoSintetico(1, 1, 10, 9);
      const nuevo = recintoSintetico(3, 1, 20, 19);
      expect(multiplicadorDefensivoDeRecintos([viejo, nuevo])).toBeCloseTo(MURALLA.bonoDefensaPorNivel[3]!, 10);
    });
  });

  describe('upkeepDeRecintos', () => {
    it('sin recintos, no cobra nada', () => {
      expect(upkeepDeRecintos([])).toEqual({});
    });

    it('un recinto sin nada levantado (avance -1) no cobra nada: no hay nada que mantener todavía', () => {
      expect(upkeepDeRecintos([recintoSintetico(1, 1, 10, -1)])).toEqual({});
    });

    it('cobra solo por las celdas YA levantadas, a la tarifa de su nivel', () => {
      const r = recintoSintetico(2, 1, 10, 4); // 5 celdas levantadas, nivel 2
      const tarifa = MURALLA.upkeepPorCelda[2]!;
      const esperado = Object.fromEntries(Object.entries(tarifa).map(([rec, cant]) => [rec, (cant ?? 0) * 5]));
      expect(upkeepDeRecintos([r])).toEqual(esperado);
    });

    it('suma el upkeep de varios recintos (una ampliación paga por los dos)', () => {
      const a = recintoSintetico(1, 1, 5, 4); // 5 celdas levantadas, nivel 1
      const b = recintoSintetico(1, 1, 3, 2); // 3 celdas levantadas, nivel 1
      const tarifa = MURALLA.upkeepPorCelda[1]!;
      const esperado = Object.fromEntries(Object.entries(tarifa).map(([rec, cant]) => [rec, (cant ?? 0) * 8]));
      expect(upkeepDeRecintos([a, b])).toEqual(esperado);
    });

    it('mientras se mejora, cobra la tarifa del nivel VIEJO: lo que está en pie, no lo que llegará a ser', () => {
      const r = recintoSintetico(1, 1, 10, 9, 2); // nivel 1, mejorandoA 2, completo
      const tarifa = MURALLA.upkeepPorCelda[1]!;
      const esperado = Object.fromEntries(Object.entries(tarifa).map(([rec, cant]) => [rec, (cant ?? 0) * 10]));
      expect(upkeepDeRecintos([r])).toEqual(esperado);
    });
  });

  // La pieza que cierra el círculo (§0): el upkeep NO es un pote aparte que solo degrada el muro por su
  // cuenta — se suma al mismo coste que ya puede hacer bajar `nivelActual` (`avanzarMantenimiento`), así que
  // un asentamiento pobre de verdad arriesga SU PROPIO mantenimiento por sostener una muralla cara, no solo
  // el muro.
  it('integración: calcularCostoMantenimiento suma el upkeep de la muralla al coste base, recurso por recurso', () => {
    const conMuro = conRecintoCompleto(99, 200, 3);
    const sinMuro: Asentamiento = { ...conMuro, recintos: [] };
    const costoBase = calcularCostoMantenimiento(sinMuro, undefined);
    const costoConMuro = calcularCostoMantenimiento(conMuro, undefined);
    const upkeep = upkeepDeRecintos(conMuro.recintos!);
    expect(Object.keys(upkeep).length).toBeGreaterThan(0); // el recinto nivel 3 sí cobra algo (piedra)
    for (const [recurso, cantidad] of Object.entries(upkeep)) {
      expect(costoConMuro[recurso]).toBeCloseTo((costoBase[recurso] ?? 0) + (cantidad ?? 0), 6);
    }
  });
});
