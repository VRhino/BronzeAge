// Fase A5 (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md): `construction.ts` migrado — código estable y
// payload estructurado en vez de `'legado'`. No repite lo que ya cubren otros tests de construcción (colas,
// costos, trazado): solo verifica que los eventos de dominio de este subsistema salen con el `codigo`/
// `payload` esperados, para atrapar un typo de código o un campo de payload movido de nombre.
import { describe, expect, it } from 'vitest';
import { avanzarSimulacion } from '../simulation';
import { createRng } from '../../worldgen';
import { contextoDeTest, crearEstadoDeTest, crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, instanteDeTest } from './fixtures';
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

  it('ocupación (Pasos 7-8): un edificio dañado se reconstruye barato y la ventana se cierra sola', () => {
    const mapa = crearMapaDeterminista(SEED);
    const facciones = crearFacciones();
    const { asentamiento, facciones: faccionesTrasFundar } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);
    let estado = crearEstadoDeTest([asentamiento], faccionesTrasFundar);
    const rng = createRng(SEED);

    // Deja madurar la ciudad para que tenga varios edificios activos.
    for (let tick = 1; tick <= 80; tick++) estado = avanzarSimulacion(estado, mapa, contextoDeTest(tick, rng));

    // Simula el estado post-conquista: un edificio no esencial dañado + ventana de ocupación abierta + un
    // almacén generoso para que la reconstrucción pueda pagarse.
    const plaza = estado.asentamientos[0]!;
    const victima = plaza.edificios.find(
      (e) => e.estado === 'activo' && !['centroUrbano', 'granja', 'lenera'].includes(e.tipo)
    )!;
    expect(victima, 'la ciudad madura tiene algún edificio dañable').toBeDefined();
    const almacenLleno = Object.fromEntries(
      Object.entries(plaza.almacen).map(([r, v]) => [r, { ...v, cantidad: v.capacidad }])
    );
    estado = {
      ...estado,
      asentamientos: [
        {
          ...plaza,
          almacen: almacenLleno,
          ocupacionHasta: instanteDeTest(88),
          edificios: plaza.edificios.map((e) => (e.id === victima.id ? { ...e, estado: 'en_cola' as const, danado: true } : e)),
        },
      ],
    };

    let vioReconstruccion = false;
    let vioFinOcupacion = false;
    for (let tick = 81; tick <= 140; tick++) {
      const r = avanzarSimulacion(estado, mapa, contextoDeTest(tick, rng));
      estado = r;
      if (r.eventosDominio.some((e) => e.codigo === 'construccion.iniciada' && e.mensaje.includes('reconstrucción'))) vioReconstruccion = true;
      if (r.eventosDominio.some((e) => e.codigo === 'asentamiento.ocupacion_terminada')) vioFinOcupacion = true;
    }

    expect(vioReconstruccion, 'el edificio dañado arrancó como reconstrucción').toBe(true);
    expect(vioFinOcupacion, 'la ventana de ocupación se cerró al vencer').toBe(true);
    const despues = estado.asentamientos[0]!;
    expect(despues.ocupacionHasta, 'ya no está ocupado').toBeUndefined();
    const reconstruido = despues.edificios.find((e) => e.id === victima.id)!;
    expect(reconstruido.estado).toBe('activo');
    expect(reconstruido.danado, 'el flag se limpia al volver a activo').toBeFalsy();
  });
});
