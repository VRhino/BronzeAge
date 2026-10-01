import { describe, expect, it } from 'vitest';
import { ERAS, TARIFA_ADOPCION, TECNOLOGIAS } from '../../constants';
import type { Asentamiento, TecnologiaId } from '../../domain/types';
import { instante } from '../../domain/tiempo';
import {
  avanzarTecnologia,
  contadoresDeEventos,
  contadoresDeReclutamiento,
  estadoTecnologiaInicial,
  sumarContadores,
  tecnologiasDe,
  tieneTecnologia,
  TECNOLOGIAS_DE_ARRANQUE,
  type DeltaContadores,
} from '../tecnologia';
import { computeTodasLasZonas } from '../zones';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, instanteDeTest } from './fixtures';

const ids = Object.keys(TECNOLOGIAS) as TecnologiaId[];

describe('catálogo de tecnologías (Doc 6.6)', () => {
  it('solo las de arranque van sin logro, y todas las demás llevan logro con umbral positivo', () => {
    for (const id of ids) {
      const t = TECNOLOGIAS[id];
      if (t.deArranque) expect(t.logro, id).toBeUndefined();
      else expect(t.logro!.umbral, id).toBeGreaterThan(0);
    }
    expect([...TECNOLOGIAS_DE_ARRANQUE].sort()).toEqual(['hostigamiento_tribal', 'leva_comunal']);
  });

  it('un hito solo pide tecnologías de su Era o anteriores', () => {
    for (const id of ids) {
      for (const c of TECNOLOGIAS[id].hito) {
        if (c.tipo !== 'tecnologia') continue;
        expect(ERAS[TECNOLOGIAS[c.id].era].orden, `${id} pide ${c.id}`).toBeLessThanOrEqual(ERAS[TECNOLOGIAS[id].era].orden);
      }
    }
  });

  it('cada Era tiene tarifa de adopción', () => {
    for (const era of Object.keys(ERAS) as (keyof typeof ERAS)[]) expect(TARIFA_ADOPCION[era].oro).toBeGreaterThan(0);
  });
});

describe('estado de tecnología', () => {
  it('el servidor arranca en la Era I y una Facción sin entrada solo tiene las de arranque', () => {
    const estado = estadoTecnologiaInicial(instante(0));
    expect(estado.era).toBe('reinos_palaciales');
    expect(tecnologiasDe(estado, 'f1').adoptadas.sort()).toEqual(['hostigamiento_tribal', 'leva_comunal']);
    expect(tieneTecnologia(estado, 'f1', 'leva_comunal')).toBe(true);
    expect(tieneTecnologia(estado, 'f1', 'metalurgia_cobre')).toBe(false);
  });
});

describe('avance de tecnología (Doc 6.2-6.4)', () => {
  const mapa = crearMapaDeterminista(7);
  const facciones = crearFacciones();
  const f1 = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);
  const f2 = fundarAsentamientoDeTest(mapa, f1.facciones, 'faccion-2', [f1.asentamiento]);
  const conFundicion = (a: Asentamiento): Asentamiento => ({
    ...a,
    edificios: [...a.edificios, { ...a.edificios[0]!, id: `${a.id}-fundicion`, tipo: 'fundicion', estado: 'activo', nivelInterno: 1 }],
  });
  const asentamientos = [conFundicion(f1.asentamiento), f2.asentamiento];
  const ctx = (tick: number) => ({ asentamientos, facciones: f2.facciones, zonas: computeTodasLasZonas(asentamientos), mapa, instante: instanteDeTest(tick) });
  const umbralCobre = TECNOLOGIAS.metalurgia_cobre.logro!.umbral;

  it('sin logro no aparece nada; con logro, solo a quien cumple el hito, y el logro queda fijado', () => {
    const sinLogro = avanzarTecnologia(estadoTecnologiaInicial(instanteDeTest(0)), ctx(1));
    expect(sinLogro.tecnologia.logros.metalurgia_cobre).toBeUndefined();

    const conCobre = sumarContadores(estadoTecnologiaInicial(instanteDeTest(0)), { 'extraido.cobre': umbralCobre });
    const r = avanzarTecnologia(conCobre, ctx(2));
    expect(r.tecnologia.logros.metalurgia_cobre).toBe(instanteDeTest(2));
    expect(tecnologiasDe(r.tecnologia, 'faccion-1').aparecidas).toContain('metalurgia_cobre');
    expect(tecnologiasDe(r.tecnologia, 'faccion-1').adoptadas).not.toContain('metalurgia_cobre');
    expect(tecnologiasDe(r.tecnologia, 'faccion-2').aparecidas).not.toContain('metalurgia_cobre');
    expect(r.tecnologia.primeros.metalurgia_cobre).toEqual({ faccionId: 'faccion-1', en: instanteDeTest(2) });
    const logro = r.eventos.find((e) => typeof e !== 'string' && e.codigo === 'tecnologia.logro');
    expect(typeof logro !== 'string' && logro?.mensaje).not.toContain('Metalurgia');

    const otra = avanzarTecnologia(r.tecnologia, ctx(3));
    expect(otra.tecnologia.logros.metalurgia_cobre).toBe(instanteDeTest(2));
    expect(otra.eventos).toEqual([]);
  });

  it('una tecnología de una Era aún cerrada no aparece aunque su logro y su hito se cumplan', () => {
    const t = sumarContadores(estadoTecnologiaInicial(instanteDeTest(0)), { 'plazasEnNivel.3': 99 });
    const r = avanzarTecnologia({ ...t, logros: { instituciones_civicas: instanteDeTest(0) } }, ctx(1));
    expect(r.tecnologia.era).toBe('reinos_palaciales');
    expect(tecnologiasDe(r.tecnologia, 'faccion-1').aparecidas).not.toContain('instituciones_civicas');
  });

  it('la Era avanza al agotar el plazo o al cumplir todos sus logros, lo que llegue antes', () => {
    const inicial = estadoTecnologiaInicial(instanteDeTest(0));
    const semanas5 = ERAS.reinos_palaciales.plazoSemanas * 7 * 24 * 60;
    expect(avanzarTecnologia(inicial, ctx(semanas5 - 1)).tecnologia.era).toBe('reinos_palaciales');
    const porPlazo = avanzarTecnologia(inicial, ctx(semanas5));
    expect(porPlazo.tecnologia.era).toBe('crisis_adaptacion');
    expect(porPlazo.tecnologia.eraDesde).toBe(instanteDeTest(semanas5));

    const todos: DeltaContadores = {};
    for (const id of ids) {
      const t = TECNOLOGIAS[id];
      if (t.era === 'reinos_palaciales' && t.logro) todos[t.logro.contador] = t.logro.umbral;
    }
    const porLogros = avanzarTecnologia(sumarContadores(inicial, todos), ctx(10));
    expect(porLogros.tecnologia.era).toBe('crisis_adaptacion');
  });
});

describe('contadores por eventos (Doc 6.3)', () => {
  const asedio = (codigo: string, enCombate: boolean, murallaCompleta: boolean, conResidentes: boolean) => ({
    codigo,
    mensaje: '',
    payload: { atacanteId: 'a', defensorId: 'd', faccionAtacanteId: 'fa', faccionDefensoraId: 'fd', enCombate, murallaCompleta, conResidentes },
  });

  it('un rebote por protección no cuenta como asedio resistido; uno en combate sí, con muralla y residentes', () => {
    expect(contadoresDeEventos([asedio('combate.asedio_resistido', false, true, false)])).toEqual({});
    expect(contadoresDeEventos([asedio('combate.asedio_resistido', true, true, true)])).toEqual({
      'asedios.resistidosEnCombate': 1,
      'asedios.resistidosConResidentes': 1,
      'asedios.contraMurallaCompleta': 1,
    });
    expect(contadoresDeEventos([asedio('combate.asedio_conquista', false, true, false)])).toEqual({ 'conquistas.conMurallaCompleta': 1 });
  });

  it('batallas, hoplitas y animales', () => {
    const delta = contadoresDeEventos([
      { codigo: 'combate.resuelto', mensaje: '', payload: { ganador: 'atacante', poderAtacante: 1, poderDefensor: 1, tropaIds: ['hoplitas_ciudadanos'] } },
      { codigo: 'comercio.caravana_animal_comprado', mensaje: '' },
    ]);
    expect(delta).toEqual({ 'batallas.libradas': 1, 'batallas.conHoplitas': 1, 'animales.comprados': 1 });
  });

  it('reclutar cuenta soldados de las tropas con logro y escuadrones nuevos', () => {
    expect(contadoresDeReclutamiento('arqueros', 18, true)).toEqual({ 'reclutados.escuadrones': 1, 'reclutados.arqueros': 18 });
    expect(contadoresDeReclutamiento('milicia_lanceros', 5, false)).toEqual({});
  });
});
