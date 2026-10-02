// La puerta por grupos (el exilio, Doc 1.10.5 y 2.8): a quién se le cierra una plaza según la relación entre
// su Facción y la del que llega. Solo regla pura; los comandos y la autorización van en `session/__tests__`.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, RelacionPolitica } from '../../domain/types';
import { grupoDePuerta, puedeEntrarEn, puedeReclutarEn } from '../pertenencia';

const relacion = (tipo: RelacionPolitica['tipo'], estado: RelacionPolitica['estado'] = 'activa'): RelacionPolitica => ({
  id: `${tipo}-1`,
  tipo,
  faccionAId: 'propia',
  faccionBId: 'otra',
  creadoEn: 0 as RelacionPolitica['creadoEn'],
  estado,
});

/** Plaza de la Facción `propia`; el que llega es un forastero de `otra`, sin casa aquí. */
const plaza = (extra: Partial<Asentamiento> = {}) =>
  ({ id: 'a', faccionId: 'propia', heroesFundadoresIds: ['residente'], casasCompradas: [], ...extra }) as unknown as Asentamiento;

describe('grupoDePuerta', () => {
  it('clasifica por la relación activa entre las Facciones', () => {
    expect(grupoDePuerta([], 'propia', 'propia')).toBe('propia');
    expect(grupoDePuerta([], 'otra', 'propia')).toBe('neutrales');
    expect(grupoDePuerta([relacion('alianza')], 'otra', 'propia')).toBe('aliados');
    expect(grupoDePuerta([relacion('vasallaje')], 'otra', 'propia')).toBe('aliados');
    expect(grupoDePuerta([relacion('guerra')], 'otra', 'propia')).toBe('enemigos');
  });

  it('una relación rota ya no cuenta', () => {
    expect(grupoDePuerta([relacion('guerra', 'rota')], 'otra', 'propia')).toBe('neutrales');
    expect(grupoDePuerta([relacion('alianza', 'rota')], 'otra', 'propia')).toBe('neutrales');
  });
});

describe('puedeEntrarEn', () => {
  it('por defecto deja entrar a los aliados y cierra a neutrales y enemigos', () => {
    expect(puedeEntrarEn(plaza(), 'h', 'otra', [relacion('alianza')])).toBe(true);
    expect(puedeEntrarEn(plaza(), 'h', 'otra', [])).toBe(false);
    expect(puedeEntrarEn(plaza(), 'h', 'otra', [relacion('guerra')])).toBe(false);
  });

  it('cada grupo se cierra y se abre por separado', () => {
    const soloAliados = plaza({ puertaCerradaA: ['aliados'] });
    expect(puedeEntrarEn(soloAliados, 'h', 'otra', [relacion('alianza')])).toBe(false);
    expect(puedeEntrarEn(soloAliados, 'h', 'otra', [])).toBe(true);
    expect(puedeEntrarEn(soloAliados, 'h', 'otra', [relacion('guerra')])).toBe(true);
  });

  it('la propia Facción nunca se bloquea, ni aunque se cierre a todos', () => {
    const todo = plaza({ puertaCerradaA: ['neutrales', 'aliados', 'enemigos'] });
    expect(puedeEntrarEn(todo, 'h', 'propia', [])).toBe(true);
  });

  it('un veto pesa más que el grupo, y un residente entra siempre', () => {
    expect(puedeEntrarEn(plaza({ puertaCerradaA: [], vetadosIds: ['h'] }), 'h', 'otra', [])).toBe(false);
    expect(puedeEntrarEn(plaza({ puertaCerradaA: ['neutrales', 'aliados', 'enemigos'] }), 'residente', 'otra', [])).toBe(true);
  });
});

describe('puedeReclutarEn', () => {
  it('un ciudadano de la Facción repone en una plaza que no es su casa aunque la puerta esté cerrada a todos', () => {
    const todo = plaza({ puertaCerradaA: ['neutrales', 'aliados', 'enemigos'] });
    expect(puedeReclutarEn(todo, 'h', 'propia')).toBe('solo_reponer');
    expect(puedeReclutarEn(plaza({ vetadosIds: ['h'] }), 'h', 'propia')).toBe('no');
  });
});
