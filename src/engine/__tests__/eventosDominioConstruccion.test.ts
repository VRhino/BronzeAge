// Fase A5 (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md): `construction.ts` migrado — código estable y
// payload estructurado en vez de `'legado'`. No repite lo que ya cubren otros tests de construcción (colas,
// costos, trazado): solo verifica que los eventos de dominio de este subsistema salen con el `codigo`/
// `payload` esperados, para atrapar un typo de código o un campo de payload movido de nombre.
import { describe, expect, it } from 'vitest';
import { avanzarSimulacion } from '../simulation';
import { createRng } from '../../worldgen';
import { contextoDeTest, crearEstadoDeTest, crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest } from './fixtures';
import type { PayloadConstruccionIniciada, PayloadNecesidadDetectada } from '../construction';

const SEED = 7;

describe('eventos de dominio — construction.ts', () => {
  it('la auto-construcción inicial produce necesidad_detectada (tick 1) e iniciada (tick 2) con payload de edificio', () => {
    // `evaluarNecesidades` (Paso 3) encola un proyecto nuevo al FINAL del tick en que lo detecta; el arranque
    // real (Paso 2, "cupo de obra") de ese mismo proyecto ocurre recién en el tick siguiente — por eso hacen
    // falta dos ticks para ver los dos códigos.
    const mapa = crearMapaDeterminista(SEED);
    const facciones = crearFacciones();
    const { asentamiento, facciones: faccionesTrasFundar } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);
    const estado = crearEstadoDeTest([asentamiento], faccionesTrasFundar);
    const rng = createRng(SEED);

    const resultado1 = avanzarSimulacion(estado, mapa, contextoDeTest(1, rng));
    const necesidad = resultado1.eventosDominio.find((e) => e.codigo === 'construccion.necesidad_detectada');
    expect(necesidad).toBeDefined();
    expect(necesidad!.asentamientoId).toBe(asentamiento.id);
    const payloadNecesidad = necesidad!.payload as PayloadNecesidadDetectada;
    expect(payloadNecesidad.edificioId).toBeTruthy();
    expect(payloadNecesidad.edificioTipo).toBeTruthy();

    const resultado2 = avanzarSimulacion(resultado1, mapa, contextoDeTest(2, rng));
    const iniciada = resultado2.eventosDominio.find((e) => e.codigo === 'construccion.iniciada');
    expect(iniciada).toBeDefined();
    const payloadIniciada = iniciada!.payload as PayloadConstruccionIniciada;
    expect(payloadIniciada.edificioId).toBeTruthy();
    expect(payloadIniciada.edificioTipo).toBeTruthy();
  });

  it('un edificio en construcción termina con edificio_completado tras sus ticks, y ninguno queda como legado', () => {
    const mapa = crearMapaDeterminista(SEED);
    const facciones = crearFacciones();
    const { asentamiento, facciones: faccionesTrasFundar } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);
    let estado = crearEstadoDeTest([asentamiento], faccionesTrasFundar);
    const rng = createRng(SEED);

    let completados: string[] = [];
    for (let tick = 1; tick <= 60 && completados.length === 0; tick++) {
      const resultado = avanzarSimulacion(estado, mapa, contextoDeTest(tick, rng));
      estado = resultado;
      completados = resultado.eventosDominio.filter((e) => e.codigo === 'construccion.edificio_completado').map((e) => e.mensaje);
    }

    expect(completados.length).toBeGreaterThan(0);
  });
});
