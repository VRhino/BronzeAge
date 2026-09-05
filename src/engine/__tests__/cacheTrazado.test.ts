// Memoización de la red de calles (Fase E3). `calcularRedDeCalles` era el 47 % del tick y el 81 % de sus
// llamadas recalculaban con entradas idénticas — ver el bloque de comentarios de `redDeCalles` y el doc 6 §1.
//
// Lo ÚNICO que este archivo tiene que demostrar es que cachear **no cambia ni una celda**. Una optimización de
// rendimiento que altera el mundo no es una optimización, es un bug con buena excusa; y como el trazado
// determina dónde cabe cada edificio, un desvío de una sola celda diverge la partida entera a los pocos ticks.
import { describe, expect, it, beforeEach } from 'vitest';
import type { Asentamiento, Edificio, Faccion } from '../../domain/types';
import { avanzarSimulacion } from '../simulation';
import { crearFaccion } from '../faccion';
import { createRng } from '../../worldgen';
import { limpiarCacheTrazado, redDeCalles, tamanoCacheTrazado } from '../trazado';
import {
  contextoDeTest,
  crearEstadoDeTest,
  crearMapaDeterminista,
  fundarAsentamientoDeTest,
  posicionRecomendable,
} from './fixtures';

const SEED = 7;

beforeEach(() => {
  limpiarCacheTrazado();
});

/** Una partida pequeña ya avanzada: asentamientos con historial de edificios de verdad, que es lo que el
 * replay recorre. Sin eso, la red sería el anillo del Centro Urbano y no probaría nada. */
function partidaAvanzada(nAsentamientos: number, ticks: number) {
  const mapa = crearMapaDeterminista(SEED);
  const facciones: Faccion[] = Array.from({ length: nAsentamientos }, (_, i) => crearFaccion(`faccion-${i}`, `Faccion ${i}`));
  const asentamientos: Asentamiento[] = [];
  for (let i = 0; i < nAsentamientos; i++) {
    const posicion = posicionRecomendable(mapa, asentamientos);
    asentamientos.push(fundarAsentamientoDeTest(mapa, facciones, facciones[i]!.id, asentamientos, 0, posicion).asentamiento);
  }
  let estado = crearEstadoDeTest(asentamientos, facciones);
  const rng = createRng(SEED);
  for (let t = 1; t <= ticks; t++) estado = avanzarSimulacion(estado, mapa, contextoDeTest(t, rng));
  return { mapa, estado };
}

const ordenadas = (s: ReadonlySet<string>): string[] => [...s].sort();

describe('memoización de redDeCalles', () => {
  it('un acierto devuelve exactamente lo mismo que el cálculo, celda a celda', () => {
    const { estado } = partidaAvanzada(6, 60);

    for (const a of estado.asentamientos) {
      limpiarCacheTrazado();
      const calculada = redDeCalles(a.id, a.edificios, a.recintos ?? []);
      // Segunda llamada: sale de la caché.
      const cacheada = redDeCalles(a.id, a.edificios, a.recintos ?? []);

      expect(ordenadas(cacheada.calles)).toEqual(ordenadas(calculada.calles));
      expect(ordenadas(cacheada.caminos)).toEqual(ordenadas(calculada.caminos));
      expect(calculada.calles.size).toBeGreaterThan(0); // si no, el test no probaría nada
    }
  });

  it('un array DISTINTO con el mismo contenido acierta — la referencia no se conserva nunca', () => {
    // Es la razón de que la clave sea el contenido y no la identidad: medido, la referencia de `edificios` se
    // repite el 0 % de las veces porque el array se reconstruye en cada paso.
    const { estado } = partidaAvanzada(3, 40);
    const a = estado.asentamientos[0]!;
    // La partida de arriba ya ha usado la caché al avanzar sus ticks: se vacía aquí para que el
    // recuento de entradas cuente SOLO las llamadas de este test.
    limpiarCacheTrazado();

    const primera = redDeCalles(a.id, a.edificios, a.recintos ?? []);
    const copia: Edificio[] = a.edificios.map((e) => ({ ...e }));
    const segunda = redDeCalles(a.id, copia, a.recintos ?? []);

    expect(segunda).toBe(primera); // mismo objeto: acertó pese a ser otro array
    expect(tamanoCacheTrazado()).toBe(1);
  });

  it('un cambio en la geometría de UN edificio da una entrada nueva, no un acierto', () => {
    const { estado } = partidaAvanzada(3, 40);
    const a = estado.asentamientos[0]!;
    // La partida de arriba ya ha usado la caché al avanzar sus ticks: se vacía aquí para que el
    // recuento de entradas cuente SOLO las llamadas de este test.
    limpiarCacheTrazado();
    redDeCalles(a.id, a.edificios, a.recintos ?? []);

    const movido = a.edificios.map((e, i) => (i === a.edificios.length - 1 ? { ...e, posicion: { x: e.posicion.x + 99, y: e.posicion.y + 99 } } : e));
    redDeCalles(a.id, movido, a.recintos ?? []);

    expect(tamanoCacheTrazado()).toBe(2);
  });

  it('el ORDEN de los edificios forma parte de la clave — el replay depende de él', () => {
    // `calcularRedDeCalles` es un replay dependiente del orden, y ese orden es load-bearing para el balance.
    // Dos arrays con los mismos edificios en distinto orden son entradas DISTINTAS, no la misma.
    const { estado } = partidaAvanzada(3, 40);
    const a = estado.asentamientos[0]!;
    expect(a.edificios.length).toBeGreaterThan(3);
    // La partida de arriba ya ha usado la caché al avanzar sus ticks: se vacía aquí para que el
    // recuento de entradas cuente SOLO las llamadas de este test.
    limpiarCacheTrazado();

    redDeCalles(a.id, a.edificios, a.recintos ?? []);
    const reordenados = [a.edificios[0]!, ...a.edificios.slice(1).reverse()];
    redDeCalles(a.id, reordenados, a.recintos ?? []);

    expect(tamanoCacheTrazado()).toBe(2);
  });

  it('dos asentamientos con el mismo trazado NO comparten entrada: el id entra en la clave', () => {
    // El id siembra el desfase pseudoaleatorio del retículo (`pseudoAleatorio(hashTexto(...))`), así que dos
    // ciudades idénticas tienen calles distintas. Compartir entrada aquí daría el trazado del vecino.
    const { estado } = partidaAvanzada(3, 40);
    const a = estado.asentamientos[0]!;
    // La partida de arriba ya ha usado la caché al avanzar sus ticks: se vacía aquí para que el
    // recuento de entradas cuente SOLO las llamadas de este test.
    limpiarCacheTrazado();

    const propia = redDeCalles(a.id, a.edificios, a.recintos ?? []);
    const ajena = redDeCalles('otro-asentamiento', a.edificios, a.recintos ?? []);

    expect(ajena).not.toBe(propia);
    expect(tamanoCacheTrazado()).toBe(2);
  });

  it('la caché no crece sin techo', () => {
    // Cada edificio nuevo genera una clave nueva; sin desalojo esto sería una fuga.
    const { estado } = partidaAvanzada(2, 30);
    const a = estado.asentamientos[0]!;
    // La partida de arriba ya ha usado la caché al avanzar sus ticks: se vacía aquí para que el
    // recuento de entradas cuente SOLO las llamadas de este test.
    limpiarCacheTrazado();
    for (let i = 0; i < 700; i++) {
      redDeCalles(`asentamiento-sintetico-${i}`, a.edificios, []);
    }
    expect(tamanoCacheTrazado()).toBeLessThanOrEqual(512);
  });

  it('una partida completa avanza IGUAL con la caché fría que caliente', () => {
    // La prueba de fondo: no compara redes sueltas sino el mundo entero tras 60 ticks. Si la memoización
    // desviara una sola celda, el trazado cambiaría dónde cabe el siguiente edificio y los dos estados
    // divergirían visiblemente.
    limpiarCacheTrazado();
    const frio = partidaAvanzada(5, 60);
    // Segunda pasada con la caché ya poblada por la primera: mismos ids de asentamiento, mismas claves.
    const caliente = partidaAvanzada(5, 60);

    const resumen = (e: typeof frio.estado) =>
      e.asentamientos
        .map((a) => `${a.id}|${a.edificios.map((b) => `${b.tipo}@${b.posicion.x},${b.posicion.y}:${b.estado}`).join(';')}`)
        .sort();

    expect(resumen(caliente.estado)).toEqual(resumen(frio.estado));
    expect(caliente.estado.asentamientos.reduce((s, a) => s + a.edificios.length, 0)).toBeGreaterThan(20);
  });
});
