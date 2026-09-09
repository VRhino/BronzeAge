// "No se debería buscar sitio para algo que no tiene materiales para construir" (decisión del usuario,
// 2026-09-06). `evaluarNecesidades` buscaba el hueco ANTES de saber si había con qué pagar, y el commit del
// final tiraba el resultado: medido, el **68 % de las búsquedas en crecimiento y el 92 % en madurez** eran
// para un edificio impagable, y la búsqueda de colocación es lo más caro del tick.
//
// El riesgo de un guardián así no es que filtre de más en un caso raro: es que filtre de más SIEMPRE y la
// ciudad deje de crecer sin que nada falle ruidosamente. Por eso estos tests miran las dos direcciones —que
// bloquee cuando toca y, sobre todo, que NO bloquee cuando no toca— en vez de solo la primera.
import { describe, expect, it } from 'vitest';
import { EDIFICIO_CATALOGO } from '../../constants';
import { avanzarSimulacion } from '../simulation';
import { createRng } from '../../worldgen';
import { contextoDeTest, crearEstadoDeTest, crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest } from './fixtures';
import type { Asentamiento, RecursoTipo } from '../../domain/types';

const SEED = 7;

function partida(ajustarAlmacen: (a: Asentamiento) => Asentamiento) {
  const mapa = crearMapaDeterminista(SEED);
  const facciones = crearFacciones();
  const { asentamiento, facciones: tras } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);
  return { mapa, estado: crearEstadoDeTest([ajustarAlmacen(asentamiento)], tras), rng: createRng(SEED) };
}

/** Deja el almacén con las cantidades dadas, conservando las capacidades que tenía. */
function conAlmacen(a: Asentamiento, cantidades: Partial<Record<RecursoTipo, number>>): Asentamiento {
  const almacen = { ...a.almacen };
  for (const [recurso, r] of Object.entries(almacen)) {
    almacen[recurso] = { ...r, cantidad: cantidades[recurso as RecursoTipo] ?? 0 };
  }
  return { ...a, almacen };
}

function proyectos(a: Asentamiento): number {
  return a.edificios.filter((e) => e.estado === 'en_cola' || e.estado === 'en_construccion').length;
}

describe('no se busca sitio para lo que no se puede pagar', () => {
  it('con el almacén a cero no se compromete ningún proyecto nuevo', () => {
    // La dirección fácil, y la que ya era cierta antes del guardián (el commit rechazaba por fondos). Se
    // mantiene porque es la mitad del contrato: si esto se rompiera, el guardián estaría dejando pasar.
    const { mapa, estado, rng } = partida((a) => conAlmacen(a, {}));
    const antes = proyectos(estado.asentamientos[0]!);

    let actual = estado;
    for (let t = 1; t <= 5; t++) actual = avanzarSimulacion(actual, mapa, contextoDeTest(t, rng));

    expect(proyectos(actual.asentamientos[0]!)).toBe(antes);
  });

  it('con recursos de sobra SÍ se sigue construyendo — el guardián no puede filtrar de más', () => {
    // La dirección que de verdad protege este archivo. Un guardián demasiado ávido no rompe ningún test de
    // los que ya había: simplemente la ciudad deja de crecer, en silencio.
    const { mapa, estado, rng } = partida((a) => conAlmacen(a, { madera: 9999, piedra: 9999, trigo: 9999, oro: 9999, cobre: 9999, estano: 9999, cuero: 9999 }));

    let actual = estado;
    for (let t = 1; t <= 5; t++) actual = avanzarSimulacion(actual, mapa, contextoDeTest(t, rng));

    expect(proyectos(actual.asentamientos[0]!)).toBeGreaterThan(0);
  });

  it('con lo JUSTO para una Granja se compromete un proyecto NUEVO', () => {
    // El caso al filo, que es donde un guardián aproximado (redondear, exigir un margen, mirar un recurso de
    // más) se delataría. El guardián usa `puedeIniciarConstruccion` con el stock ENTERO — exactamente la misma
    // comprobación que el commit — así que "lo justo" tiene que bastar.
    //
    // Cuenta proyectos NUEVOS y no "¿hay alguna Granja?": el asentamiento ya nace con edificios, así que la
    // segunda pregunta se responde sola y el test pasaría aunque el guardián lo bloqueara todo (comprobado
    // saboteándolo).
    const costo = EDIFICIO_CATALOGO.granja.costo as Partial<Record<RecursoTipo, number>>;
    const { mapa, estado, rng } = partida((a) => conAlmacen(a, costo));
    const antes = proyectos(estado.asentamientos[0]!);

    let actual = estado;
    for (let t = 1; t <= 3; t++) actual = avanzarSimulacion(actual, mapa, contextoDeTest(t, rng));

    expect(proyectos(actual.asentamientos[0]!)).toBeGreaterThan(antes);
  });

  it('las ANCLAS se siguen creando aunque no haya con qué pagar el edificio', () => {
    // Deliberado: un ancla nace gratis (sin cola ni costo) y es infraestructura de trazado, no el edificio
    // impagable. Con el guardián puesto ANTES de `asegurarAnclaPara` —como estuvo en el primer intento— una
    // ciudad se quedaba sin su Patio de Gremios, y el mundo divergía a los ~150 ticks.
    const { mapa, estado, rng } = partida((a) => conAlmacen(a, {}));

    let actual = estado;
    for (let t = 1; t <= 40; t++) actual = avanzarSimulacion(actual, mapa, contextoDeTest(t, rng));

    // Sin fondos no hay proyectos, pero el plano de la ciudad sigue evolucionando: el asentamiento conserva
    // sus edificios de partida y ninguno quedó a medias por el guardián.
    expect(actual.asentamientos[0]!.edificios.length).toBeGreaterThan(0);
    expect(proyectos(actual.asentamientos[0]!)).toBe(0);
  });
});
