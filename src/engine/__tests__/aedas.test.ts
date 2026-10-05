// Aedas itinerantes (Doc 6.7): cuántos hay, cómo recorren las plazas, qué revelan y qué venden.
import { describe, expect, it } from 'vitest';
import { AEDAS, TARIFA_ADOPCION } from '../../constants';
import type { AedaItinerante, Asentamiento, EstadoTecnologia } from '../../domain/types';
import { avanzarAedas, aedaEn, itinerantesObjetivo, type ContextoAedas } from '../aedas';
import { RED_VACIA } from '../redCaminos';
import { estadoTecnologiaInicial, precioDeVenta, revelarTecnologias, tecnologiasDe, venderTecnologia, VentaInvalidaError } from '../tecnologia';
import { agregarRecurso } from '../almacen';
import { computeTodasLasZonas } from '../zones';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, instanteDeTest } from './fixtures';

const mapa = crearMapaDeterminista(7);
const f1 = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
const f2 = fundarAsentamientoDeTest(mapa, f1.facciones, 'faccion-2', [f1.asentamiento]);
const facciones = f2.facciones;
const plazas = [f1.asentamiento, f2.asentamiento];
const contexto = (tick: number, asentamientos: readonly Asentamiento[] = plazas): ContextoAedas => ({
  asentamientos,
  facciones,
  zonas: computeTodasLasZonas([...asentamientos]),
  mapa,
  instante: instanteDeTest(tick),
  red: RED_VACIA,
});

/** Metalurgia del cobre desbloqueada por la Facción 1 en el tick 0 (su logro y su hito, a mano). */
const conCobreDesbloqueado = (): EstadoTecnologia => ({
  ...estadoTecnologiaInicial(instanteDeTest(0)),
  logros: { metalurgia_cobre: instanteDeTest(0) },
  primeros: { metalurgia_cobre: { faccionId: 'faccion-1', en: instanteDeTest(0) } },
});
const RETRASO = AEDAS.retrasoConocimientoMinutos;

describe('cuántos itinerantes hay', () => {
  it('ninguno sin plazas; con plazas, uno por cada tres Facciones y mínimo tres', () => {
    expect(itinerantesObjetivo([])).toBe(0);
    expect(itinerantesObjetivo(plazas)).toBe(AEDAS.itinerantes.minimo);
    const diez = Array.from({ length: 10 }, (_, i) => ({ ...f1.asentamiento, id: `p${i}`, faccionId: `f${i}` }));
    expect(itinerantesObjetivo(diez)).toBe(Math.ceil(10 / AEDAS.itinerantes.faccionesPorAeda));
  });
});

describe('recorrido', () => {
  it('nacen el mínimo en el primer tick, repartidos por las plazas, y no nacen más', () => {
    const r = avanzarAedas([], estadoTecnologiaInicial(instanteDeTest(0)), contexto(0));
    expect(r.aedas.map((a) => a.id)).toEqual(['aeda-1', 'aeda-2', 'aeda-3']);
    expect(new Set(r.aedas.map((a) => a.enAsentamientoId))).toEqual(new Set(plazas.map((p) => p.id)));
    expect(avanzarAedas(r.aedas, r.tecnologia, contexto(1)).aedas).toHaveLength(3);
  });

  it('salen de su plaza tras la estancia y llegan a la otra, dejando constancia de por dónde han pasado', () => {
    let aedas: AedaItinerante[] = [];
    let tecnologia = estadoTecnologiaInicial(instanteDeTest(0));
    const vistas = new Map<string, Set<string>>();
    let salio = false;
    for (let tick = 0; tick < 4000; tick++) {
      const r = avanzarAedas(aedas, tecnologia, contexto(tick));
      aedas = r.aedas;
      tecnologia = r.tecnologia;
      for (const a of aedas) {
        if (a.enAsentamientoId) vistas.set(a.id, (vistas.get(a.id) ?? new Set()).add(a.enAsentamientoId));
        if (a.ruta && a.progreso > 0) salio = true;
      }
    }
    expect(salio).toBe(true);
    for (const a of aedas) expect(vistas.get(a.id)!.size, a.id).toBe(2);
  });

  it('una plaza que les cerró la puerta a los Aedas no los recibe, pero el cierre por defecto (neutrales y enemigos) no cuenta', () => {
    const cerrada = { ...f2.asentamiento, puertaCerradaA: ['neutrales' as const, 'enemigos' as const, 'aedas' as const] };
    const r = avanzarAedas([], estadoTecnologiaInicial(instanteDeTest(0)), contexto(0, [f1.asentamiento, cerrada]));
    expect(r.aedas.every((a) => a.enAsentamientoId === f1.asentamiento.id)).toBe(true);
    expect(aedaEn(r.aedas, cerrada.id)).toBeUndefined();
    expect(f2.asentamiento.puertaCerradaA).toBeUndefined();
    const porDefecto = { ...f2.asentamiento, puertaCerradaA: ['neutrales' as const, 'enemigos' as const] };
    expect(avanzarAedas([], estadoTecnologiaInicial(instanteDeTest(0)), contexto(0, [f1.asentamiento, porDefecto])).aedas.some((a) => a.enAsentamientoId === porDefecto.id)).toBe(true);
  });
});

describe('revelar (Doc 6.4)', () => {
  const revelar = (tick: number, estado = conCobreDesbloqueado()) => revelarTecnologias(estado, facciones.find((f) => f.id === 'faccion-2')!, f2.asentamiento.id, contexto(tick));

  it('la ventaja del primero: hasta que pasa el retraso no la conocen, y entonces se la revelan a quien aún no la tiene', () => {
    expect(revelar(RETRASO - 1).eventos).toEqual([]);
    const r = revelar(RETRASO);
    expect(tecnologiasDe(r.tecnologia, 'faccion-2').reveladas).toEqual(['metalurgia_cobre']);
    expect(tecnologiasDe(r.tecnologia, 'faccion-2').aparecidas).not.toContain('metalurgia_cobre');
    expect(r.eventos).toHaveLength(1);
    expect(r.eventos[0]).toMatchObject({
      codigo: 'aedas.revela',
      asentamientoId: f2.asentamiento.id,
      payload: { faccionId: 'faccion-2', tecnologiaId: 'metalurgia_cobre', descubridorFaccionId: 'faccion-1', faltan: ['fundicion activo'] },
    });
  });

  it('se revela una sola vez y no a quien ya la tiene aparecida', () => {
    const una = revelar(RETRASO);
    expect(revelar(RETRASO + 10, una.tecnologia).eventos).toEqual([]);
    const yaTiene = { ...conCobreDesbloqueado(), porFaccion: { 'faccion-2': { aparecidas: ['leva_comunal', 'hostigamiento_tribal', 'metalurgia_cobre'], adoptadas: ['leva_comunal', 'hostigamiento_tribal'] } } } as EstadoTecnologia;
    expect(revelar(RETRASO, yaTiene).eventos).toEqual([]);
  });

  it('un Aeda detenido en la plaza revela durante su estancia en cuanto el retraso se cumple', () => {
    const estancia = { id: 'aeda-1', posicion: f2.asentamiento.posicion, enAsentamientoId: f2.asentamiento.id, hasta: instanteDeTest(RETRASO + 100), progreso: 0, recientes: [] };
    const r = avanzarAedas([estancia], conCobreDesbloqueado(), contexto(RETRASO));
    expect(r.eventos.some((e) => typeof e !== 'string' && e.codigo === 'aedas.revela')).toBe(true);
  });
});

describe('vender (Doc 6.7)', () => {
  const faccion2 = facciones.find((f) => f.id === 'faccion-2')!;
  const conOro = (oro: number): Asentamiento => ({ ...f2.asentamiento, almacen: agregarRecurso(f2.asentamiento.almacen, 'oro', oro) });
  const oroDe = (a: Asentamiento) => a.almacen['oro']?.cantidad ?? 0;
  const comprar = (tick: number, plaza = conOro(5000), estado = conCobreDesbloqueado()) =>
    venderTecnologia(estado, faccion2, plaza, 'aeda-1', 'metalurgia_cobre', instanteDeTest(tick));

  it('cuesta el doble del oro de la tarifa de adopción de su Era, sale del almacén de la plaza y la tecnología aparece sin hito (no adoptada)', () => {
    expect(precioDeVenta('metalurgia_cobre')).toBe(2 * TARIFA_ADOPCION.reinos_palaciales.oro!);
    const plaza = conOro(5000);
    const r = comprar(RETRASO, plaza);
    expect(oroDe(r.plaza)).toBe(oroDe(plaza) - precioDeVenta('metalurgia_cobre'));
    const t = tecnologiasDe(r.tecnologia, 'faccion-2');
    expect(t.aparecidas).toContain('metalurgia_cobre');
    expect(t.adoptadas).not.toContain('metalurgia_cobre');
    expect(r.eventos[0]).toMatchObject({ codigo: 'aedas.venta', asentamientoId: plaza.id, payload: { tecnologiaId: 'metalurgia_cobre', aedaId: 'aeda-1' } });
  });

  it('comprar una revelada la quita de las reveladas', () => {
    const revelada = revelarTecnologias(conCobreDesbloqueado(), faccion2, f2.asentamiento.id, contexto(RETRASO)).tecnologia;
    expect(tecnologiasDe(comprar(RETRASO, undefined, revelada).tecnologia, 'faccion-2').reveladas).toEqual([]);
  });

  it('rechazo: antes de que la conozca, sin logro cumplido, ya aparecida o sin oro', () => {
    expect(() => comprar(RETRASO - 1)).toThrow(VentaInvalidaError);
    expect(() => comprar(RETRASO, undefined, estadoTecnologiaInicial(instanteDeTest(0)))).toThrow(VentaInvalidaError);
    expect(() => comprar(RETRASO, conOro(0))).toThrow(VentaInvalidaError);
    const una = comprar(RETRASO).tecnologia;
    expect(() => comprar(RETRASO, undefined, una)).toThrow(VentaInvalidaError);
  });
});
