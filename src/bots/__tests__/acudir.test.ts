// Un bot parado y sin plan acude a donde los suyos combaten o se juntan (Doc 5.14.4, 5.15.1b): batalla de su Facción o formación de un compañero.
import { describe, expect, it } from 'vitest';
import { LOGISTICA } from '../../constants';
import type { ContextoBot } from '../runner';
import type { Vista } from '../puerto';
import { acudirALosSuyos } from '../cerebro/acudir';

const YO = 'heroe-bot';
const AQUI = { x: 500, y: 500 };

interface Llamada {
  tipo: string;
  params: Record<string, unknown>;
}

/** Un contexto con solo lo que mira el cerebro: la vista y quien registra lo que pide. */
function contexto(vista: Partial<Vista> & Record<string, unknown>, aceptar = true) {
  const llamadas: Llamada[] = [];
  const registrar = async (tipo: string, params: unknown) => {
    llamadas.push({ tipo, params: params as Record<string, unknown> });
    return { ok: aceptar, eventos: [], version: 1 };
  };
  const ctx = {
    yo: YO,
    vista: { faccionId: 'f-1', instante: 1000, heroe: { heridoHasta: undefined }, batallas: [], ejercitos: [], ...vista },
    actuar: registrar,
    intentar: (_clave: string, tipo: string, params: unknown) => registrar(tipo, params),
  } as unknown as ContextoBot;
  return { ctx, llamadas };
}

const columna = (extra: Record<string, unknown> = {}) => ({
  id: 'col-yo',
  tipo: 'personal',
  estado: 'estacionado',
  participantes: [{ heroeId: YO, unidoEn: 0 }],
  posicionActual: AQUI,
  ...extra,
});

const batalla = (extra: Record<string, unknown> = {}) => ({
  battleId: 'b-1',
  estado: 'en_curso',
  contexto: { tipo: 'asedio', asentamientoId: 'plaza-1' },
  punto: { x: AQUI.x + 5, y: AQUI.y },
  bandos: { atacante: { faccionId: 'f-2', heroes: 1, capacidadMaxima: 5 }, defensor: { faccionId: 'f-1', heroes: 1, capacidadMaxima: 5 } },
  ...extra,
});

const formacion = (extra: Record<string, unknown> = {}) => ({
  id: 'col-amiga',
  tipo: 'personal',
  estado: 'estacionado',
  participantes: [{ heroeId: 'amiga', unidoEn: 0 }],
  posicionActual: { x: AQUI.x + 4, y: AQUI.y },
  formacion: { expiraEn: 99999 },
  ...extra,
});

describe('acudir a los suyos', () => {
  it('a la batalla de su Facción, si está a 15: pide unirse', async () => {
    const { ctx, llamadas } = contexto({ ejercitos: [columna()] as never, batallas: [batalla()] as never });

    expect(await acudirALosSuyos(ctx)).toBe(true);
    expect(llamadas).toEqual([{ tipo: 'unirseABatalla', params: { heroeId: YO, battleId: 'b-1' } }]);
  });

  it('si queda lejos, se pone en marcha hacia ella', async () => {
    const lejos = batalla({ punto: { x: AQUI.x + LOGISTICA.radioEncuentro + 50, y: AQUI.y } });
    const { ctx, llamadas } = contexto({ ejercitos: [columna()] as never, batallas: [lejos] as never });

    expect(await acudirALosSuyos(ctx)).toBe(true);
    expect(llamadas).toEqual([{ tipo: 'marcharA', params: { heroeId: YO, objetivo: { tipo: 'punto', punto: lejos.punto } } }]);
  });

  it('no toma partido en lo que no es de los suyos: otra Facción, una persecución o una batalla campal', async () => {
    const ajena = batalla({ bandos: { atacante: { faccionId: 'f-2', heroes: 1, capacidadMaxima: 5 }, defensor: { faccionId: 'f-3', heroes: 1, capacidadMaxima: 5 } } });
    const persecucion = batalla({ contexto: { tipo: 'campo_abierto', columnas: 'solitarios' } });
    const campal = batalla({ contexto: { tipo: 'campo_abierto', columnas: 'ejercitos' } });
    for (const b of [ajena, persecucion, campal]) {
      const { ctx, llamadas } = contexto({ ejercitos: [columna()] as never, batallas: [b] as never });
      expect(await acudirALosSuyos(ctx)).toBe(false);
      expect(llamadas).toEqual([]);
    }
  });

  it('no se une dos veces a la misma batalla, ni a una terminada', async () => {
    for (const b of [batalla({ ladoPropio: 'defensor' }), batalla({ estado: 'aplicada' }), batalla({ estado: 'cancelada' })]) {
      const { ctx, llamadas } = contexto({ ejercitos: [columna()] as never, batallas: [b] as never });
      expect(await acudirALosSuyos(ctx)).toBe(false);
      expect(llamadas).toEqual([]);
    }
  });

  it('a la formación de un compañero, junto a ella: pide unirse', async () => {
    const { ctx, llamadas } = contexto({ ejercitos: [columna(), formacion()] as never });

    expect(await acudirALosSuyos(ctx)).toBe(true);
    expect(llamadas).toEqual([{ tipo: 'unirseEnCampo', params: { ejercitoId: 'col-amiga', heroeId: YO } }]);
  });

  it('una formación fuera de su vista no la llama', async () => {
    const lejos = formacion({ posicionActual: { x: AQUI.x + 5000, y: AQUI.y } });
    const { ctx, llamadas } = contexto({ ejercitos: [columna(), lejos] as never });

    expect(await acudirALosSuyos(ctx)).toBe(false);
    expect(llamadas).toEqual([]);
  });

  it('solo una columna personal que va sola, parada y sana', async () => {
    const batallas = [batalla()] as never;
    const casos = [
      contexto({ ejercitos: [columna({ estado: 'marchando' })] as never, batallas }),
      contexto({ ejercitos: [columna({ tipo: 'ejercito' })] as never, batallas }),
      contexto({ ejercitos: [columna({ participantes: [{ heroeId: YO, unidoEn: 0 }, { heroeId: 'x', unidoEn: 0 }] })] as never, batallas }),
      contexto({ ejercitos: [columna({ formacion: { expiraEn: 1 } })] as never, batallas }),
      contexto({ ejercitos: [columna()] as never, batallas, heroe: { heridoHasta: 5000 } as never }),
      contexto({ ejercitos: [] as never, batallas }),
    ];
    for (const { ctx, llamadas } of casos) {
      expect(await acudirALosSuyos(ctx)).toBe(false);
      expect(llamadas).toEqual([]);
    }
  });

  it('sin batallas ni formaciones no hace nada: los bots de siempre juegan igual', async () => {
    const { ctx, llamadas } = contexto({ ejercitos: [columna()] as never });

    expect(await acudirALosSuyos(ctx)).toBe(false);
    expect(llamadas).toEqual([]);
  });

  it('si el motor rechaza la petición, no cuenta como hecho', async () => {
    const { ctx } = contexto({ ejercitos: [columna()] as never, batallas: [batalla()] as never }, false);

    expect(await acudirALosSuyos(ctx)).toBe(false);
  });
});
