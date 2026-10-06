// Gran Fundición (Doc 4.2.1): refunde la chatarra de las batallas en lingote. No consume equipo de recluta —eso
// formaría un bucle con la Armería—; el metal vuelve solo de las bajas, y el vencedor se lo lleva.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, Edificio } from '../../domain/types';
import { instante } from '../../domain/tiempo';
import { chatarraDeBajas, chatarraDeMuertos } from '../chatarra';
import { anadirEdificioManualmente, avanzarConstruccion, ConstruccionManualInvalidaError } from '../construction';
import { TODAS_LAS_TECNOLOGIAS } from '../tecnologia';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest } from './fixtures';

const RECLAMOS_VACIOS = { nodos: new Set<string>(), lenerasPorBosque: new Map<string, number>() };

function capitalPreparada(nivelAsentamiento: number) {
  const mapa = crearMapaDeterminista(42);
  const { asentamiento, facciones } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
  const lleno = (recurso: string) => ({ cantidad: 1000, capacidad: 5000, ...(asentamiento.almacen[recurso] ? { capacidad: Math.max(5000, asentamiento.almacen[recurso]!.capacidad) } : {}) });
  const preparado: Asentamiento = {
    ...asentamiento,
    nivel: nivelAsentamiento,
    nivelActual: nivelAsentamiento,
    cargos: { ...asentamiento.cargos, gobernadorId: 'jugador-faccion-1-1' },
    almacen: { ...asentamiento.almacen, madera: lleno('madera'), piedra: lleno('piedra'), oro: lleno('oro') },
  };
  return { asentamiento: preparado, faccion: { ...facciones[0]!, nivel: 6 }, mapa };
}

const construir = (p: ReturnType<typeof capitalPreparada>, opciones: { capital?: Asentamiento | undefined; yaTiene?: boolean } = {}) =>
  anadirEdificioManualmente(
    p.asentamiento,
    p.faccion,
    'gobernador',
    'granFundicion',
    [],
    p.mapa,
    'capital' in opciones ? opciones.capital : p.asentamiento,
    RECLAMOS_VACIOS,
    TODAS_LAS_TECNOLOGIAS,
    0,
    0,
    opciones.yaTiene ?? false
  );

describe('Gran Fundición — dónde se puede construir', () => {
  it('en la capital, con Facción de nivel 6 y asentamiento de nivel 4, entra en la cola', () => {
    const resultado = construir(capitalPreparada(4));
    expect(resultado.edificios.some((e) => e.tipo === 'granFundicion' && e.estado === 'en_cola')).toBe(true);
  });

  it('no con la Facción por debajo de nivel 6', () => {
    const p = capitalPreparada(4);
    expect(() => construir({ ...p, faccion: { ...p.faccion, nivel: 5 } })).toThrow(ConstruccionManualInvalidaError);
  });

  it('no en un asentamiento de nivel 3', () => {
    expect(() => construir(capitalPreparada(3))).toThrow(ConstruccionManualInvalidaError);
  });

  it('no fuera de la capital', () => {
    expect(() => construir(capitalPreparada(4), { capital: undefined })).toThrow(/capital/);
  });

  it('no si la Facción ya tiene una', () => {
    expect(() => construir(capitalPreparada(4), { yaTiene: true })).toThrow(/ya tiene/);
  });
});

describe('Gran Fundición — la chatarra', () => {
  it('cada baja deja la mitad del lingote que costó su equipo, por metal', () => {
    // Espadachines de bronce: 2 armas de bronce + 1 armadura intermedia por soldado = 2 lingotes de bronce y 1 de cobre.
    const chatarra = chatarraDeMuertos('espadachines_bronce', 4);
    expect(chatarra.chatarraBronce).toBeCloseTo(4);
    expect(chatarra.chatarraCobre).toBeCloseTo(2);
  });

  it('el equipo de madera y cuero no deja chatarra', () => {
    expect(chatarraDeMuertos('lanceros_mimbre', 10)).toEqual({});
    expect(chatarraDeMuertos('milicia_lanceros', 10)).toEqual({});
  });

  it('se cuenta por la diferencia de efectivos antes y después', () => {
    const e = { id: 'e1', tropaId: 'espadachines_bronce', cantidad: 18 } as never;
    const tras = { ...(e as object), cantidad: 14 } as never;
    expect(chatarraDeBajas([e], [tras]).chatarraBronce).toBeCloseTo(4);
  });

  it('la Gran Fundición la funde en lingote, 1 a 1, y deja intacto el equipo del almacén', () => {
    const { asentamiento, mapa } = capitalPreparada(4);
    const edificio: Edificio = { id: 'gf-1', tipo: 'granFundicion', posicion: { x: 20, y: 0 }, estado: 'activo', ambito: 'asentamiento' };
    const conChatarra: Asentamiento = {
      ...asentamiento,
      edificios: [...asentamiento.edificios, edificio],
      poblacion: { ...asentamiento.poblacion, artesanos: 500 },
      almacen: {
        ...asentamiento.almacen,
        chatarraBronce: { cantidad: 3, capacidad: 100 },
        lingoteBronce: { cantidad: 0, capacidad: 100 },
        armaBronce: { cantidad: 7, capacidad: 100 },
      },
    };
    const r = avanzarConstruccion(conChatarra, [], mapa, undefined, RECLAMOS_VACIOS, instante(1), 0, TODAS_LAS_TECNOLOGIAS);
    const almacen = r.asentamiento.almacen;
    expect(almacen.lingoteBronce!.cantidad).toBeGreaterThan(0);
    expect(almacen.lingoteBronce!.cantidad + almacen.chatarraBronce!.cantidad).toBeCloseTo(3);
    expect(almacen.armaBronce!.cantidad).toBe(7);
  });
});
