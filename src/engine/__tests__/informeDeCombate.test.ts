// El informe de un combate (para el briefing de un cliente): cada evento de resolución lleva, por bando, el poder con que entró, los
// héroes implicados y lo que le pasó a cada escuadra, sin que el cliente tenga que parsear el `mensaje`.
import { describe, expect, it } from 'vitest';
import type { CampamentoBandido } from '../../domain/types';
import type { EjercitoConTropa } from '../tropa';
import { atacarCampamentoConColumna, poderTotal, resolverCombate, type PayloadAtaqueCampamento, type PayloadCombateResuelto } from '../combate';
import { createRng } from '../../worldgen';
import { crearFacciones, escuadronDePrueba } from './fixtures';

const total = (lado: { bajas: { antes: number; despues: number }[] }, campo: 'antes' | 'despues') => lado.bajas.reduce((n, b) => n + b[campo], 0);

describe('resolverCombate: el evento lleva el informe de los dos bandos', () => {
  it('héroes, poder y bajas por escuadra de cada lado, coherentes con lo que quedó', () => {
    const atacantes = [escuadronDePrueba('a1', 'ana', 'milicia_lanceros', 20), escuadronDePrueba('a2', 'ana', 'lenadores', 30)];
    const defensores = [escuadronDePrueba('d1', 'dan', 'milicia_lanceros', 25)];
    const r = resolverCombate(atacantes, defensores, createRng(3));

    const payload = (r.eventos[0] as { payload: PayloadCombateResuelto }).payload;
    expect(payload.atacante.heroesIds).toEqual(['ana']);
    expect(payload.defensor.heroesIds).toEqual(['dan']);
    expect(payload.atacante.poder).toBe(payload.poderAtacante);
    expect(payload.defensor.poder).toBe(payload.poderDefensor);
    expect(payload.atacante.bajas.map((b) => [b.escuadronId, b.tropaId, b.antes])).toEqual([['a1', 'milicia_lanceros', 20], ['a2', 'lenadores', 30]]);
    expect(total(payload.atacante, 'despues')).toBe(r.atacantes.reduce((n, e) => n + e.cantidad, 0));
    expect(total(payload.defensor, 'despues')).toBe(r.defensores.reduce((n, e) => n + e.cantidad, 0));
    // Alguien perdió gente: el perdedor siempre pierde más.
    const perdedor = payload.ganador === 'atacante' ? payload.defensor : payload.atacante;
    expect(total(perdedor, 'despues')).toBeLessThan(total(perdedor, 'antes'));
  });
});

describe('atacarCampamentoConColumna: el evento lleva el nivel y el poder del campamento y el informe de la columna', () => {
  it('con las bajas de cada escuadra, gane o pierda', () => {
    const tropa = [escuadronDePrueba('e1', 'ana', 'milicia_lanceros', 15), escuadronDePrueba('e2', 'ana', 'lenadores', 15), escuadronDePrueba('e3', 'ana', 'granjeros', 15)];
    const columna = { id: 'col-1', faccionId: 'faccion-1', escuadrones: tropa, suministro: {} } as unknown as EjercitoConTropa;
    const campamento = { id: 'bandidos-1', posicion: { x: 0, y: 0 }, bosqueId: 'b', nivel: 2, poder: poderTotal(tropa, false) * 0.1 } as CampamentoBandido;

    const r = atacarCampamentoConColumna(columna, campamento, crearFacciones(), createRng(1));
    const evento = r.eventos[0] as { codigo: string; payload: PayloadAtaqueCampamento };
    expect(evento.codigo).toBe('combate.campamento_destruido');
    expect(evento.payload).toMatchObject({ campamentoId: 'bandidos-1', nivelCampamento: 2, poderCampamento: campamento.poder });
    expect(evento.payload.atacante.heroesIds).toEqual(['ana']);
    expect(evento.payload.atacante.bajas).toHaveLength(3);
    expect(total(evento.payload.atacante, 'despues')).toBe(r.ejercito.escuadrones.reduce((n, e) => n + e.cantidad, 0));
    expect(total(evento.payload.atacante, 'antes')).toBe(45);

    const perdida = atacarCampamentoConColumna(columna, { ...campamento, poder: 1e9 }, crearFacciones(), createRng(1));
    expect((perdida.eventos[0] as { codigo: string }).codigo).toBe('combate.ataque_campamento_fallido');
  });
});
