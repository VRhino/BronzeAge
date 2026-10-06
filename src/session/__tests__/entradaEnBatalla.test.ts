// Quién puede unirse a una batalla y a qué bando (Doc 5.15.1b): la regla pura, con facciones y relaciones sintéticas.
import { describe, expect, it } from 'vitest';
import type { Faccion, RelacionPolitica } from '../../domain/types';
import { instante } from '../../domain/tiempo';
import type { ContextoEstrategico, LadoId } from '../../contratos/v1/dto';
import { BatallaInvalidaError, type Batalla } from '../batallas';
import { faccionDe, ladoParaUnirse, sonAliadasEnBatalla } from '../entradaEnBatalla';

const faccion = (id: string, ciudadanosIds: string[], extra: Partial<Faccion> = {}): Faccion =>
  ({ id, nombre: id, ciudadanosIds, reyId: ciudadanosIds[0] ?? null, ...extra }) as Faccion;
const relacion = (tipo: RelacionPolitica['tipo'], a: string, b: string, estado: RelacionPolitica['estado'] = 'activa'): RelacionPolitica =>
  ({ id: `${tipo}-${a}-${b}`, tipo, faccionAId: a, faccionBId: b, creadoEn: instante(0), estado }) as RelacionPolitica;

/** Una batalla mínima: lo único que mira la regla es el contexto y las Facciones titulares. */
const batalla = (contextoEstrategico: ContextoEstrategico, atacante: string | null, defensor: string | null): Batalla =>
  ({ ticket: { contextoEstrategico, bandos: { atacante: { faccionId: atacante }, defensor: { faccionId: defensor } } } }) as unknown as Batalla;

const ASEDIO: ContextoEstrategico = { tipo: 'asedio', asentamientoId: 'plaza' };
const CAMPAL: ContextoEstrategico = { tipo: 'campo_abierto', punto: { x: 0, y: 0 }, columnas: 'ejercitos' };
const PERSECUCION: ContextoEstrategico = { tipo: 'campo_abierto', punto: { x: 0, y: 0 }, columnas: 'solitarios' };
const BANDIDOS: ContextoEstrategico = { tipo: 'campamento_bandidos', campamentoId: 'c', punto: { x: 0, y: 0 } };

function mundo(opciones: { admite?: boolean; relaciones?: RelacionPolitica[] } = {}) {
  return {
    facciones: [
      faccion('ataca', ['a1', 'a2'], opciones.admite ? { admiteOtrasEnAtaques: true } : {}),
      faccion('defiende', ['d1']),
      faccion('aliada', ['l1']),
      faccion('vasalla', ['v1']),
      faccion('neutral', ['n1']),
    ],
    relaciones: opciones.relaciones ?? [relacion('alianza', 'defiende', 'aliada'), relacion('vasallaje', 'defiende', 'vasalla')],
  };
}
const lado = (m: ReturnType<typeof mundo>, b: Batalla, heroeId: string, tipo: 'personal' | 'ejercito' = 'ejercito', pedido?: LadoId) =>
  ladoParaUnirse(m, b, { id: heroeId }, { tipo }, pedido);
const rechaza = (f: () => unknown) => expect(f).toThrow(BatallaInvalidaError);

describe('sonAliadasEnBatalla', () => {
  it('una alianza o un vasallaje activos, y nada más', () => {
    const m = mundo({ relaciones: [relacion('alianza', 'a', 'b'), relacion('vasallaje', 'c', 'd'), relacion('guerra', 'e', 'f'), relacion('alianza', 'g', 'h', 'rota')] });

    expect(sonAliadasEnBatalla(m.relaciones, 'b', 'a')).toBe(true);
    expect(sonAliadasEnBatalla(m.relaciones, 'c', 'd'), 'señor y vasallo cuentan como aliados').toBe(true);
    expect(sonAliadasEnBatalla(m.relaciones, 'e', 'f')).toBe(false);
    expect(sonAliadasEnBatalla(m.relaciones, 'g', 'h')).toBe(false);
  });
});

describe('asedio y asalto de caravana', () => {
  const b = batalla(ASEDIO, 'ataca', 'defiende');

  it('la Facción de cada bando entra en el suyo', () => {
    expect(lado(mundo(), b, 'a2')).toBe('atacante');
    expect(lado(mundo(), b, 'd1')).toBe('defensor');
  });

  it('los aliados y los vasallos del defensor lo defienden', () => {
    expect(lado(mundo(), b, 'l1')).toBe('defensor');
    expect(lado(mundo(), b, 'v1')).toBe('defensor');
  });

  it('un neutral solo entra al ataque si el Rey de la Facción atacante lo admite', () => {
    rechaza(() => lado(mundo(), b, 'n1'));
    expect(lado(mundo({ admite: true }), b, 'n1')).toBe('atacante');
  });

  it('aunque se admita a otros, nunca un aliado del defensor se une al ataque', () => {
    expect(lado(mundo({ admite: true }), b, 'l1')).toBe('defensor');
    rechaza(() => lado(mundo({ admite: true }), b, 'l1', 'ejercito', 'atacante'));
  });

  it('sin Facción no se entra en un asedio', () => {
    rechaza(() => lado(mundo({ admite: true }), b, 'sinfaccion'));
  });

  it('también una Columna personal se une a un asedio abierto', () => {
    expect(lado(mundo(), b, 'a2', 'personal')).toBe('atacante');
  });

  it('el asalto a una caravana sigue las mismas reglas', () => {
    const caravana = batalla({ tipo: 'caravana', caravanaId: 'c', punto: { x: 0, y: 0 } }, 'ataca', 'defiende');

    expect(lado(mundo(), caravana, 'l1')).toBe('defensor');
    rechaza(() => lado(mundo(), caravana, 'n1'));
    expect(lado(mundo({ admite: true }), caravana, 'n1')).toBe('atacante');
  });
});

describe('batalla campal y persecución', () => {
  it('una batalla campal no admite a nadie de fuera', () => {
    rechaza(() => lado(mundo(), batalla(CAMPAL, 'ataca', 'defiende'), 'a2'));
  });

  it('una persecución es libre: cualquiera, al bando que elija, aunque sea contra los suyos; pero sin elegir no entra', () => {
    const b = batalla(PERSECUCION, 'ataca', 'defiende');

    expect(lado(mundo(), b, 'a2', 'personal', 'defensor')).toBe('defensor');
    expect(lado(mundo(), b, 'sinfaccion', 'personal', 'atacante')).toBe('atacante');
    rechaza(() => lado(mundo(), b, 'a2', 'personal'));
  });

  it('un ejército no se une a una persecución', () => {
    rechaza(() => lado(mundo(), batalla(PERSECUCION, 'ataca', 'defiende'), 'a2', 'ejercito', 'atacante'));
  });
});

describe('campamento de bandidos (evento PvE)', () => {
  const b = batalla(BANDIDOS, 'ataca', null);

  it('cualquiera se une, siempre contra los bandidos, tenga o no Facción', () => {
    expect(lado(mundo(), b, 'n1')).toBe('atacante');
    expect(lado(mundo(), b, 'sinfaccion', 'personal')).toBe('atacante');
  });

  it('nadie puede ayudar a los bandidos', () => {
    rechaza(() => lado(mundo(), b, 'n1', 'ejercito', 'defensor'));
  });
});

describe('faccionDe', () => {
  it('la Facción de la que se es ciudadano, o ninguna', () => {
    expect(faccionDe(mundo(), 'a1')).toBe('ataca');
    expect(faccionDe(mundo(), 'nadie')).toBeNull();
  });
});
