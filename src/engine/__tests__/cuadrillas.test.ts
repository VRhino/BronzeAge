// Reparto de las cuadrillas de obra (engine/construction.ts, decisiones del usuario del 2026-09-26): las mejoras
// ocupan cuadrilla, pero las automáticas dejan siempre una libre para construir; y una reconstrucción tras un saqueo no
// ocupa ninguna ni espera turno. Salió al medir: con obras de horas, una plaza saqueada no llegaba a reconstruirse y una
// cadena de mejoras de la Granja retenía una cuadrilla durante más de mil ticks.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, Edificio } from '../../domain/types';
import { EDIFICIO_CATALOGO, NECESIDADES } from '../../constants';
import { avanzarConstruccion, quitarDeCola, reclamosDeFuentes } from '../construction';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, instanteDeTest } from './fixtures';

const AHORA = instanteDeTest(10);
const LEJOS = instanteDeTest(100_000);

function plazaConFondos(): { mapa: ReturnType<typeof crearMapaDeterminista>; plaza: Asentamiento } {
  const mapa = crearMapaDeterminista(7);
  const { asentamiento } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
  const almacen = Object.fromEntries(Object.entries(asentamiento.almacen).map(([r, v]) => [r, { ...v, cantidad: 5000, capacidad: 99_999 }]));
  return { mapa, plaza: { ...asentamiento, almacen } };
}

function obra(id: string): Edificio {
  return { id, tipo: 'vivienda', posicion: { x: 0, y: 0 }, estado: 'en_construccion', ambito: 'asentamiento', completaEn: LEJOS };
}

const tick = (mapa: ReturnType<typeof crearMapaDeterminista>, a: Asentamiento) =>
  avanzarConstruccion(a, [], mapa, undefined, reclamosDeFuentes([a]), AHORA, 0).asentamiento;

describe('cuadrillas de obra', () => {
  it('una reconstrucción arranca aunque las cuadrillas estén todas ocupadas, y no ocupa ninguna', () => {
    const { mapa, plaza } = plazaConFondos();
    const danada = plaza.edificios.find((e) => e.tipo === 'vivienda')!;
    const ocupadas = Array.from({ length: NECESIDADES.maximoEnConstruccionSimultanea }, (_, i) => obra(`obra-${i}`));
    const a: Asentamiento = {
      ...plaza,
      edificios: [...plaza.edificios.map((e) => (e.id === danada.id ? { ...e, estado: 'en_cola' as const, danado: true } : e)), ...ocupadas],
    };

    const tras = tick(mapa, a);

    expect(tras.edificios.find((e) => e.id === danada.id)!.estado).toBe('en_construccion');
    // Las obras que ya iban siguen siendo las que ocupan las cuadrillas: la reconstrucción no ha desplazado a nadie.
    expect(tras.edificios.filter((e) => e.id.startsWith('obra-') && e.estado === 'en_construccion')).toHaveLength(ocupadas.length);
  });

  it('con todas las cuadrillas libres, una mejora automática arranca', () => {
    const { mapa, plaza } = plazaConFondos();
    const tras = tick(mapa, plaza);
    expect(tras.edificios.some((e) => e.mejora !== undefined)).toBe(true);
  });

  it('una mejora automática no ocupa la última cuadrilla libre', () => {
    const { mapa, plaza } = plazaConFondos();
    // Todas menos una ocupadas: la que queda es para construir.
    const ocupadas = Array.from({ length: NECESIDADES.maximoEnConstruccionSimultanea - 1 }, (_, i) => obra(`obra-${i}`));
    const tras = tick(mapa, { ...plaza, edificios: [...plaza.edificios, ...ocupadas] });
    expect(tras.edificios.some((e) => e.mejora !== undefined)).toBe(false);
  });

  it('quitar de la cola una reconstrucción devuelve lo que se pagó en su día: la obra y sus mejoras', () => {
    const { plaza } = plazaConFondos();
    const granja = plaza.edificios.find((e) => e.tipo === 'granja')!;
    const a: Asentamiento = {
      ...plaza,
      cargos: { ...plaza.cargos, gobernadorId: 'gobernador' },
      edificios: plaza.edificios.map((e) => (e.id === granja.id ? { ...e, estado: 'en_cola' as const, danado: true, nivelInterno: 3 } : e)),
    };
    const niveles = EDIFICIO_CATALOGO.granja.niveles as Record<number, { costoMejora?: Record<string, number> }>;
    const madera = (EDIFICIO_CATALOGO.granja.costo as Record<string, number>)['madera'] ?? 0;
    const esperada = madera + (niveles[2]!.costoMejora!['madera'] ?? 0) + (niveles[3]!.costoMejora!['madera'] ?? 0);

    const tras = quitarDeCola(a, 'gobernador', granja.id);

    expect(tras.almacen['madera']!.cantidad - a.almacen['madera']!.cantidad).toBe(esperada);
    expect(tras.edificios.some((e) => e.id === granja.id)).toBe(false);
  });
});
