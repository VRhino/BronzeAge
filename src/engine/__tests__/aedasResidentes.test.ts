// Aedas residentes (Doc 6.7): cupo, llegada, marcha, reputación, hechos de las épicas y sus límites contra el farmeo.
import { describe, expect, it } from 'vitest';
import { AEDAS, EDIFICIO_CATALOGO, EPICAS, TECNOLOGIAS } from '../../constants';
import type { EventoDominio } from '../../domain/eventos';
import type { Asentamiento, EstadoAedasResidentes, EstadoTecnologia, Faccion, TecnologiaId } from '../../domain/types';
import { createRng } from '../../worldgen';
import { abandonarEpica, avanzarEpicas, avanzarResidentes, cupoDeResidentes, empezarEpica, EpicaInvalidaError, hechosDeEventos, RESIDENTES_VACIOS } from '../aedasResidentes';
import { crecerPoblacion } from '../population';
import { estadoTecnologiaInicial, tecnologiasDe } from '../tecnologia';
import { calcularTitulos } from '../titulos';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, instanteDeTest } from './fixtures';

const mapa = crearMapaDeterminista(7);
const f1 = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
const f2 = fundarAsentamientoDeTest(mapa, f1.facciones, 'faccion-2', [f1.asentamiento]);
const facciones: Faccion[] = f2.facciones;

/** La plaza con Palacio activo, nobleza y el nivel dado. */
function conPalacio(a: Asentamiento, nivel = 2, nobleza = 5): Asentamiento {
  return {
    ...a,
    nivel,
    nivelActual: nivel,
    poblacion: { ...a.poblacion, nobleza },
    edificios: [...a.edificios, { ...a.edificios[0]!, id: `${a.id}-palacio`, tipo: 'palacio', estado: 'activo', nivelInterno: 1 }],
  };
}
const plaza = conPalacio(f1.asentamiento);
const tecnologia: EstadoTecnologia = { ...estadoTecnologiaInicial(instanteDeTest(0)), logros: { metalurgia_cobre: instanteDeTest(0) } };
const ctx = (tick: number, asentamientos: readonly Asentamiento[] = [plaza], t: EstadoTecnologia = tecnologia, fs: readonly Faccion[] = facciones) => ({
  asentamientos,
  facciones: fs,
  tecnologia: t,
  instante: instanteDeTest(tick),
});
const LLEGADA = AEDAS.residentes.llegadaMinutos;
const residente = (extra = {}) => ({ id: 'aeda-r1', nombre: 'Femio', asentamientoId: plaza.id, faccionId: plaza.faccionId, llegadaEn: instanteDeTest(0), ...extra });
const conResidente = (extra = {}): EstadoAedasResidentes => ({ ...RESIDENTES_VACIOS, aedas: [residente(extra)], siguiente: 2 });
const epicaCobre = { tecnologiaId: 'metalurgia_cobre' as const, capitulo: 0, hechos: 0, claves: [] as string[] };

describe('catálogo de épicas', () => {
  it('toda tecnología con logro tiene épica, de 3, 4 o 5 capítulos según su Era, con edificios que existen', () => {
    const porEra = { reinos_palaciales: 3, crisis_adaptacion: 4, polis_imperios: 5 } as const;
    for (const id of Object.keys(TECNOLOGIAS) as TecnologiaId[]) {
      const t = TECNOLOGIAS[id];
      if (t.deArranque) {
        expect(EPICAS[id], id).toBeUndefined();
        continue;
      }
      const e = EPICAS[id]!;
      expect(e.capitulos.length, id).toBe(porEra[t.era]);
      for (const c of e.capitulos) {
        expect(c.cantidad).toBeGreaterThan(0);
        expect(c.hecho === 'obra', `${id}: solo las obras llevan edificio`).toBe(c.edificio !== undefined);
        if (c.edificio) expect(EDIFICIO_CATALOGO[c.edificio], `${id}: ${c.edificio}`).toBeDefined();
      }
    }
  });
});

describe('ciclo de vida', () => {
  it('cupo por nivel; sin Palacio o sin nobleza, ninguno', () => {
    expect([2, 3, 4, 5].map((n) => cupoDeResidentes(conPalacio(f1.asentamiento, n)))).toEqual([1, 2, 3, 4]);
    expect(cupoDeResidentes(conPalacio(f1.asentamiento, 3, 0))).toBe(0);
    expect(cupoDeResidentes({ ...f1.asentamiento, nivel: 3, nivelActual: 3, poblacion: { ...f1.asentamiento.poblacion, nobleza: 5 } })).toBe(0);
    expect(cupoDeResidentes(conPalacio(f1.asentamiento, 1))).toBe(0);
  });

  it('llega uno cuando se cumple el plazo, y no más que el cupo', () => {
    const espera = avanzarResidentes(RESIDENTES_VACIOS, ctx(0));
    expect(espera.estado.aedas).toEqual([]);
    expect(avanzarResidentes(espera.estado, ctx(LLEGADA - 1)).estado.aedas).toEqual([]);
    const llega = avanzarResidentes(espera.estado, ctx(LLEGADA));
    expect(llega.estado.aedas).toHaveLength(1);
    expect(llega.eventos[0]).toMatchObject({ codigo: 'aedas.residente_llega', asentamientoId: plaza.id });
    // Con el cupo lleno no llega otro ni queda plazo pendiente.
    const lleno = avanzarResidentes(llega.estado, ctx(LLEGADA * 3));
    expect(lleno.estado.aedas).toHaveLength(1);
    expect(lleno.estado.llegadaEn).toEqual({});
  });

  it('con la reputación por los suelos tardan tres veces más', () => {
    const baja = facciones.map((f) => (f.id === plaza.faccionId ? { ...f, reputacion: AEDAS.residentes.umbralReputacionBaja } : f));
    const espera = avanzarResidentes(RESIDENTES_VACIOS, ctx(0, [plaza], tecnologia, baja)).estado;
    expect(avanzarResidentes(espera, ctx(LLEGADA * 3 - 1, [plaza], tecnologia, baja)).estado.aedas).toHaveLength(0);
    expect(avanzarResidentes(espera, ctx(LLEGADA * 3, [plaza], tecnologia, baja)).estado.aedas).toHaveLength(1);
  });

  it('si la plaza pierde la nobleza se van, y si el cupo baja se va el último en llegar', () => {
    const dos = { ...RESIDENTES_VACIOS, siguiente: 3, aedas: [residente(), residente({ id: 'aeda-r2', nombre: 'Demódoco', llegadaEn: instanteDeTest(10) })] };
    const sinNobleza = avanzarResidentes(dos, ctx(20, [conPalacio(f1.asentamiento, 3, 0)]));
    expect(sinNobleza.estado.aedas).toEqual([]);
    expect(sinNobleza.eventos.every((e) => typeof e !== 'string' && e.codigo === 'aedas.residente_se_va')).toBe(true);
    const bajaNivel = avanzarResidentes(dos, ctx(20, [conPalacio(f1.asentamiento, 2)]));
    expect(bajaNivel.estado.aedas.map((a) => a.id)).toEqual(['aeda-r1']);
  });

  it('una plaza conquistada cambia de dueño: la épica en curso se pierde, el Aeda se queda', () => {
    const conquistada = { ...plaza, faccionId: 'faccion-2' };
    const r = avanzarResidentes(conResidente({ epica: { ...epicaCobre, capitulo: 1, hechos: 1, claves: ['x'] } }), ctx(5, [conquistada]));
    expect(r.estado.aedas[0]).toMatchObject({ faccionId: 'faccion-2', asentamientoId: plaza.id });
    expect(r.estado.aedas[0]!.epica).toBeUndefined();
  });

  it('una plaza arrasada se lleva a sus Aedas', () => {
    expect(avanzarResidentes(conResidente(), ctx(5, [])).estado.aedas).toEqual([]);
  });

  it('una épica cuya tecnología ya ha aparecido por otra vía se cierra', () => {
    const yaAparecida: EstadoTecnologia = {
      ...tecnologia,
      porFaccion: { [plaza.faccionId]: { aparecidas: ['leva_comunal', 'hostigamiento_tribal', 'metalurgia_cobre'], adoptadas: ['leva_comunal', 'hostigamiento_tribal'] } },
    };
    expect(avanzarResidentes(conResidente({ epica: epicaCobre }), ctx(5, [plaza], yaAparecida)).estado.aedas[0]!.epica).toBeUndefined();
  });
});

describe('efectos', () => {
  it('cada residente suma felicidad: la población no crece menos', () => {
    const base = { ...plaza, poblacion: { ...plaza.poblacion, pesants: 40 } };
    const crece = (n: number) => crecerPoblacion(base, createRng(3), undefined, n).poblacion.pesants;
    expect(crece(4)).toBeGreaterThanOrEqual(crece(0));
  });

  it('Mecenas de los Aedas: más épicas cumplidas, y a igualdad más residentes; sin ninguna, nadie', () => {
    const titulo = (aedas: EstadoAedasResidentes) => calcularTitulos(facciones, [plaza, f2.asentamiento], [], [], aedas).find((t) => t.nombre === 'Mecenas de los Aedas');
    expect(titulo(RESIDENTES_VACIOS)).toBeUndefined();
    expect(titulo({ ...RESIDENTES_VACIOS, aedas: [residente()] })?.poseedorId).toBe(plaza.faccionId);
    expect(titulo({ ...RESIDENTES_VACIOS, aedas: [residente()], cumplidas: { 'faccion-2': 1 } })?.poseedorId).toBe('faccion-2');
  });
});

describe('empezar y abandonar una épica', () => {
  const empezar = (id: TecnologiaId = 'metalurgia_cobre', estado = conResidente(), t = tecnologia) => empezarEpica(estado, t, plaza, 'aeda-r1', id);
  const conAparecida = (): EstadoTecnologia => ({
    ...tecnologia,
    porFaccion: { [plaza.faccionId]: { aparecidas: ['leva_comunal', 'hostigamiento_tribal', 'metalurgia_cobre'], adoptadas: [] } },
  });

  it('empieza con el logro cumplido y la tecnología sin aparecer, y se puede abandonar perdiendo lo avanzado', () => {
    const r = empezar();
    expect(r.estado.aedas[0]!.epica).toEqual(epicaCobre);
    expect(r.eventos[0]).toMatchObject({ codigo: 'aedas.epica_empieza', asentamientoId: plaza.id });
    const fuera = abandonarEpica(r.estado, plaza, 'aeda-r1');
    expect(fuera.estado.aedas[0]!.epica).toBeUndefined();
    expect(() => abandonarEpica(fuera.estado, plaza, 'aeda-r1')).toThrow(EpicaInvalidaError);
  });

  it('rechazo: sin logro, Era cerrada, ya aparecida, sin épica, Aeda ajeno, ya con épica o repetida por otro Aeda', () => {
    expect(() => empezar('aleacion_bronce')).toThrow(EpicaInvalidaError);
    expect(() => empezar('instituciones_civicas', conResidente(), { ...tecnologia, logros: { instituciones_civicas: instanteDeTest(0) } })).toThrow(EpicaInvalidaError);
    expect(() => empezar('metalurgia_cobre', conResidente(), conAparecida())).toThrow(EpicaInvalidaError);
    expect(() => empezar('leva_comunal')).toThrow(EpicaInvalidaError);
    expect(() => empezarEpica(conResidente(), tecnologia, plaza, 'otro', 'metalurgia_cobre')).toThrow(EpicaInvalidaError);
    expect(() => empezar('metalurgia_cobre', empezar().estado)).toThrow(EpicaInvalidaError);
    const dos: EstadoAedasResidentes = { ...RESIDENTES_VACIOS, aedas: [residente({ epica: epicaCobre }), residente({ id: 'aeda-r2' })], siguiente: 3 };
    expect(() => empezarEpica(dos, tecnologia, plaza, 'aeda-r2', 'metalurgia_cobre')).toThrow(EpicaInvalidaError);
  });
});

describe('hechos', () => {
  const ev = (codigo: string, payload: unknown, asentamientoId?: string, momento = 't0'): EventoDominio => ({ codigo, mensaje: '', momento, payload, asentamientoId });
  const asedio = (enCombate: boolean) => ({ atacanteId: 'a', defensorId: plaza.id, faccionAtacanteId: 'faccion-2', faccionDefensoraId: plaza.faccionId, enCombate, murallaCompleta: false, conResidentes: false });
  const hechos = (e: EventoDominio[]) => hechosDeEventos(e, [plaza, f2.asentamiento]);
  const llegaCaravana = (carga: number) => ev('comercio.caravana_llega', { caravanaId: 'c1', tipo: 'comercial', origenId: plaza.id, destinoId: 'x', contenido: { madera: carga }, comision: 0 });

  it('lo que cuenta: defensa y conquista en combate, batalla entre Facciones distintas, obra, ascenso, adopción y caravana con carga', () => {
    expect(hechos([ev('combate.asedio_resistido', asedio(true))]).map((h) => h.tipo)).toEqual(['defensa']);
    expect(hechos([ev('combate.asedio_conquista', asedio(true))]).map((h) => h.tipo)).toEqual(['conquista']);
    expect(hechos([ev('combate.encuentro', { ejercitoAId: 'a', ejercitoBId: 'b', faccionAId: 'faccion-1', faccionBId: 'faccion-2', ganadorId: 'b' })])[0]).toMatchObject({ tipo: 'batalla', faccionId: 'faccion-2' });
    expect(hechos([ev('construccion.edificio_completado', { edificioId: 'e1', edificioTipo: 'fundicion' }, plaza.id)])[0]).toMatchObject({ tipo: 'obra', edificio: 'fundicion', plazaId: plaza.id });
    expect(hechos([ev('asentamiento.nivel_subio', { asentamientoId: plaza.id, nivelNuevo: 3 })])[0]).toMatchObject({ tipo: 'ascenso' });
    expect(hechos([ev('tecnologia.adoptada', { faccionId: 'faccion-1', tecnologiaId: 'metalurgia_cobre' })])[0]).toMatchObject({ tipo: 'adopcion' });
    expect(hechos([llegaCaravana(AEDAS.epica.cargaMinimaCaravana)]).map((h) => h.tipo)).toEqual(['caravana']);
  });

  it('lo que no: asedio sin combate, batalla contra la propia Facción, caravana ligera', () => {
    expect(hechos([ev('combate.asedio_resistido', asedio(false)), ev('combate.asedio_conquista', asedio(false))])).toEqual([]);
    expect(hechos([ev('combate.encuentro', { ejercitoAId: 'a', ejercitoBId: 'b', faccionAId: 'faccion-1', faccionBId: 'faccion-1', ganadorId: 'a' })])).toEqual([]);
    expect(hechos([llegaCaravana(1)])).toEqual([]);
  });
});

describe('avance de una épica', () => {
  const obra = (id: string, tipo = 'fundicion', asentamientoId: string = plaza.id): EventoDominio => ({
    codigo: 'construccion.edificio_completado',
    mensaje: '',
    momento: id,
    payload: { edificioId: id, edificioTipo: tipo },
    asentamientoId,
  });
  const caravana = (id: string): EventoDominio => ({
    codigo: 'comercio.caravana_llega',
    mensaje: '',
    momento: id,
    payload: { caravanaId: id, tipo: 'comercial', origenId: plaza.id, destinoId: 'x', contenido: { madera: 500 }, comision: 0 },
  });
  const avanzar = (estado: EstadoAedasResidentes, eventos: EventoDominio[], tick: number) => avanzarEpicas(estado, eventos, ctx(tick));
  const ENFR = AEDAS.epica.enfriamientoMinutos;

  it('un hecho del tipo del capítulo lo cierra y pasa al siguiente; uno de otro tipo o de otra plaza no cuenta', () => {
    const e0 = conResidente({ epica: epicaCobre });
    expect(avanzar(e0, [obra('x', 'armeria')], 10).estado).toBe(e0);
    expect(avanzar(e0, [obra('x', 'fundicion', f2.asentamiento.id)], 10).estado).toBe(e0);
    const r = avanzar(e0, [obra('x')], 10);
    expect(r.estado.aedas[0]!.epica).toMatchObject({ capitulo: 1, hechos: 0, ultimoHechoEn: instanteDeTest(10) });
    expect(r.eventos[0]).toMatchObject({ codigo: 'aedas.epica_capitulo', asentamientoId: plaza.id });
  });

  it('el mismo hecho no cuenta dos veces y entre hechos hay que esperar el enfriamiento', () => {
    let estado = avanzar(conResidente({ epica: { ...epicaCobre, capitulo: 1 } }), [caravana('c1')], 10).estado;
    expect(estado.aedas[0]!.epica).toMatchObject({ capitulo: 1, hechos: 1 });
    expect(avanzar(estado, [caravana('c2')], 10 + ENFR - 1).estado).toBe(estado); // aún en enfriamiento
    expect(avanzar(estado, [caravana('c1')], 10 + ENFR).estado).toBe(estado); // misma clave
    estado = avanzar(estado, [caravana('c2')], 10 + ENFR).estado;
    expect(estado.aedas[0]!.epica).toMatchObject({ capitulo: 2, hechos: 0 });
  });

  it('al cerrar el último capítulo la tecnología aparece sin hito, la Facción suma una épica y la crónica canta sin nombrarla', () => {
    const ultimo = { ...epicaCobre, capitulo: EPICAS.metalurgia_cobre!.capitulos.length - 1 };
    const ascenso: EventoDominio = { codigo: 'asentamiento.nivel_subio', mensaje: '', momento: 'n', payload: { asentamientoId: plaza.id, nivelNuevo: 3 } };
    const r = avanzar(conResidente({ epica: ultimo }), [ascenso], 10);
    expect(r.estado.aedas[0]!.epica).toBeUndefined();
    expect(r.estado.cumplidas[plaza.faccionId]).toBe(1);
    expect(tecnologiasDe(r.tecnologia, plaza.faccionId).aparecidas).toContain('metalurgia_cobre');
    expect(tecnologiasDe(r.tecnologia, plaza.faccionId).adoptadas).not.toContain('metalurgia_cobre');
    const publico = r.eventos.find((e) => typeof e !== 'string' && e.codigo === 'aedas.epica_cumplida');
    expect(publico).toBeDefined();
    expect(typeof publico !== 'string' && publico!.mensaje).not.toMatch(/cobre/i);
    expect(typeof publico !== 'string' && publico!.asentamientoId).toBeUndefined();
  });
});
