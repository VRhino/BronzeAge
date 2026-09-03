// `proyectarParaJugador` sobre estado de partida GENUINO (misma fixture que `comandos/autorizacion.ts`), no
// mocks — así estas pruebas fallan de verdad si cambia la forma de `GameSessionState` o el significado de
// ciudadanía, no solo si cambia la proyección.
import { describe, expect, it } from 'vitest';
import { instanteDeTest } from '../../../engine/__tests__/fixtures';
import { partidaConAsentamiento, MOMENTO, OPC } from '../../__tests__/fixtures';
import { crearFaccion } from '../../comandos/crearFaccion';
import { fundarAsentamiento } from '../../comandos/fundarAsentamiento';
import { idDeMapa, type GeometriaAsentamientos } from '../../estado';
import { proyectarParaJugador } from '../jugador';
import type { Ejercito, Escuadron, Point } from '../../../domain/types';
import { LOGISTICA } from '../../../constants';

// Estas pruebas verifican filtrado por Facción/ciudadanía, no la geometría por frame (Fase C10, cubierta en
// su propia sección más abajo) — una entrada vacía basta y no obliga a construir asentamientos reales solo
// para pasarlos por `computeTodasLasZonas`.
const SIN_GEOMETRIA: GeometriaAsentamientos = { zonas: [], zonasFusionadas: [], trazadoPorAsentamiento: {} };

describe('faccionId se deriva de la ciudadanía, no de un campo guardado', () => {
  it('el fundador ve su propia Facción', () => {
    const { sesion, faccionId, fundador } = partidaConAsentamiento();
    const proyeccion = proyectarParaJugador(sesion.getState(), fundador, SIN_GEOMETRIA);
    expect(proyeccion.faccionId).toBe(faccionId);
  });

  it('un jugador sin ciudadanía en ninguna Facción tiene faccionId null', () => {
    const { sesion } = partidaConAsentamiento();
    const proyeccion = proyectarParaJugador(sesion.getState(), 'forastero', SIN_GEOMETRIA);
    expect(proyeccion.faccionId).toBeNull();
  });
});

describe('asentamientos: solo los de la Facción propia', () => {
  it('el fundador ve su asentamiento', () => {
    const { sesion, asentamientoId, fundador } = partidaConAsentamiento();
    const proyeccion = proyectarParaJugador(sesion.getState(), fundador, SIN_GEOMETRIA);
    expect(proyeccion.asentamientos.map((a) => a.id)).toEqual([asentamientoId]);
  });

  it('un forastero sin Facción no ve ningún asentamiento, aunque exista', () => {
    const { sesion } = partidaConAsentamiento();
    const proyeccion = proyectarParaJugador(sesion.getState(), 'forastero', SIN_GEOMETRIA);
    expect(proyeccion.asentamientos).toEqual([]);
  });

  it('el asentamiento de una Facción rival NO aparece — es justo lo que Slice 1 no resuelve todavía', () => {
    const base = partidaConAsentamiento();
    const opcRival = { ...OPC, actor: 'rival' };
    const rf = base.sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, opcRival);
    base.sesion.ejecutar(fundarAsentamiento, { faccionId: rf.datos!.faccionId, posicion: { x: 900, y: 900 } }, opcRival);

    const proyeccion = proyectarParaJugador(base.sesion.getState(), base.fundador, SIN_GEOMETRIA);
    expect(proyeccion.asentamientos).toHaveLength(1); // solo el propio, no los 2 que existen en la partida
    expect(proyeccion.asentamientos[0]!.faccionId).toBe(base.faccionId);
  });
});

describe('facciones: metadatos públicos de TODAS, sin filtrar', () => {
  it('incluye la Facción rival aunque sus asentamientos no aparezcan', () => {
    const base = partidaConAsentamiento();
    const opcRival = { ...OPC, actor: 'rival' };
    const rf = base.sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, opcRival);

    const proyeccion = proyectarParaJugador(base.sesion.getState(), base.fundador, SIN_GEOMETRIA);
    expect(proyeccion.facciones.map((f) => f.id).sort()).toEqual([base.faccionId, rf.datos!.faccionId].sort());
  });
});

describe('caravanas, acuerdos y ordenes: solo los que tocan un asentamiento propio', () => {
  // Se fabrica la orden directamente en el estado (en vez de vía comando) porque `colocarOrdenMercado`
  // exige un Mercado construido, irrelevante para lo que aquí se prueba: el filtro por `asentamientoId`.
  it('una orden del asentamiento propio aparece; una de un asentamiento ajeno no', () => {
    const { sesion, asentamientoId, fundador } = partidaConAsentamiento();
    const estadoConOrdenes = {
      ...sesion.getState(),
      ordenes: [
        { id: 'o1', asentamientoId, tipo: 'venta' as const, recurso: 'trigo', cantidad: 10, cantidadCumplida: 0, precioUnitario: 1, creadoEn: instanteDeTest(0), estado: 'activa' as const },
        { id: 'o2', asentamientoId: 'asentamiento-ajeno', tipo: 'venta' as const, recurso: 'trigo', cantidad: 10, cantidadCumplida: 0, precioUnitario: 1, creadoEn: instanteDeTest(0), estado: 'activa' as const },
      ],
    };

    const proyeccion = proyectarParaJugador(estadoConOrdenes, fundador, SIN_GEOMETRIA);
    expect(proyeccion.ordenes.map((o) => o.id)).toEqual(['o1']);
  });

  it('una caravana con destino (no origen) en un asentamiento propio también cuenta', () => {
    const { sesion, asentamientoId, fundador } = partidaConAsentamiento();
    const estadoConCaravana = {
      ...sesion.getState(),
      caravanas: [
        {
          id: 'c1',
          tipo: 'comercial' as const,
          origenAsentamientoId: 'ajeno',
          destinoAsentamientoId: asentamientoId,
          contenido: {},
          posicionActual: { x: 0, y: 0 },
          progreso: 0,
        },
      ],
    };

    const proyeccion = proyectarParaJugador(estadoConCaravana, fundador, SIN_GEOMETRIA);
    expect(proyeccion.caravanas.map((c) => c.id)).toEqual(['c1']);
  });
});

describe('eventosDominio: sin asentamientoId (globales) o con uno propio', () => {
  it('el evento de fundación (con asentamientoId propio) pasa el filtro', () => {
    const { sesion, asentamientoId, fundador } = partidaConAsentamiento();
    const proyeccion = proyectarParaJugador(sesion.getState(), fundador, SIN_GEOMETRIA);
    expect(proyeccion.eventosDominio.some((e) => e.asentamientoId === asentamientoId)).toBe(true);
  });

  it('un evento de asentamiento AJENO no aparece en la proyección de un jugador sin ese asentamiento', () => {
    const base = partidaConAsentamiento();
    const opcRival = { ...OPC, actor: 'rival' };
    const rf = base.sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, opcRival);
    const ra = base.sesion.ejecutar(fundarAsentamiento, { faccionId: rf.datos!.faccionId, posicion: { x: 900, y: 900 } }, opcRival);

    const proyeccion = proyectarParaJugador(base.sesion.getState(), base.fundador, SIN_GEOMETRIA);
    expect(proyeccion.eventosDominio.some((e) => e.asentamientoId === ra.datos!.asentamientoId)).toBe(false);
  });
});

describe('historial: el propio, nunca el de otro jugador', () => {
  it('devuelve el historial de ESE jugador', () => {
    const { sesion, fundador } = partidaConAsentamiento();
    const proyeccion = proyectarParaJugador(sesion.getState(), fundador, SIN_GEOMETRIA);
    expect(proyeccion.historial.length).toBeGreaterThan(0);
    expect(proyeccion.historial).toEqual(sesion.getState().historialJugadores[fundador]);
  });

  it('un jugador sin historial recibe un array vacío, no undefined', () => {
    const { sesion } = partidaConAsentamiento();
    const proyeccion = proyectarParaJugador(sesion.getState(), 'nadie-hizo-nada', SIN_GEOMETRIA);
    expect(proyeccion.historial).toEqual([]);
  });
});

describe('mapaId, relaciones, titulos, caminos y campamentosBandidos: públicos, sin filtrar', () => {
  it('relaciones, titulos, caminos y campamentosBandidos viajan tal cual desde el estado', () => {
    const { sesion, fundador } = partidaConAsentamiento();
    const estado = sesion.getState();
    const proyeccion = proyectarParaJugador(estado, fundador, SIN_GEOMETRIA);

    expect(proyeccion.relaciones).toBe(estado.relaciones);
    expect(proyeccion.titulos).toBe(estado.titulos);
    expect(proyeccion.caminos).toBe(estado.caminos);
    expect(proyeccion.campamentosBandidos).toBe(estado.campamentosBandidos);
  });

  it('mapaId identifica el mapa del estado (Fase C11) sin mandarlo entero', () => {
    const { sesion, fundador } = partidaConAsentamiento();
    const estado = sesion.getState();
    const proyeccion = proyectarParaJugador(estado, fundador, SIN_GEOMETRIA);

    expect(proyeccion).not.toHaveProperty('mapa');
    expect(proyeccion.mapaId).toBe(idDeMapa(estado.mapa));
  });
});

// ---------------------------------------------------------------------------------------------------------
// Ejercitos (Doc 5.12.7): la UNICA cosa de una Faccion rival que sale de esta proyeccion, y sale redactada.
// ---------------------------------------------------------------------------------------------------------

const escuadron = (id: string, jugadorId: string): Escuadron => ({
  id,
  nombre: 'milicia',
  jugadorId,
  origen: 'pesants',
  cantidad: 10,
  veterania: 0,
  moral: 100,
  tropaId: 'milicia_lanceros',
});

function ejercito(id: string, faccionId: string, posicion: Point, escuadrones: Escuadron[]): Ejercito {
  return {
    id,
    faccionId,
    origenAsentamientoId: `origen-de-${id}`,
    escuadrones,
    suministro: { trigo: 500 },
    caravanasAdjuntasIds: [],
    objetivo: { tipo: 'punto', punto: posicion },
    ruta: [{ x: 0, y: 0 }, posicion],
    progreso: 0.5,
    posicionActual: posicion,
    estado: 'marchando',
  };
}

/** Cuadrado de zona de influencia alrededor de un punto — basta para `pointInPolygon`, y evita construir
 * asentamientos reales solo para que `computeTodasLasZonas` los recorte. */
function zonaCuadrada(asentamientoId: string, centro: Point, radio: number): GeometriaAsentamientos {
  return {
    zonas: [
      {
        asentamientoId,
        poligono: [
          { x: centro.x - radio, y: centro.y - radio },
          { x: centro.x + radio, y: centro.y - radio },
          { x: centro.x + radio, y: centro.y + radio },
          { x: centro.x - radio, y: centro.y + radio },
        ],
      },
    ],
    zonasFusionadas: [],
    trazadoPorAsentamiento: {},
  };
}

describe('ejercitos: los propios, completos', () => {
  it('el ejercito de la Faccion propia viaja entero, con sus escuadrones', () => {
    const { sesion, faccionId, fundador } = partidaConAsentamiento();
    const propio = ejercito('e-propio', faccionId, { x: 1500, y: 1500 }, [escuadron('s1', fundador)]);
    const estado = { ...sesion.getState(), ejercitos: [propio] };

    const proyeccion = proyectarParaJugador(estado, fundador, SIN_GEOMETRIA);
    expect(proyeccion.ejercitos).toEqual([propio]);
    expect(proyeccion.ejercitosAvistados).toEqual([]);
  });

  it('se ve este donde este: lo propio no depende de tener vision sobre ello', () => {
    const { sesion, faccionId, fundador } = partidaConAsentamiento();
    // Al otro extremo del mundo, sin zona de influencia ni ningun otro ejercito cerca.
    const propio = ejercito('e-propio', faccionId, { x: 1990, y: 1990 }, [escuadron('s1', fundador)]);
    const estado = { ...sesion.getState(), ejercitos: [propio] };

    expect(proyectarParaJugador(estado, fundador, SIN_GEOMETRIA).ejercitos).toHaveLength(1);
  });

  it('un jugador HUERFANO (sin Faccion) sigue viendo la columna en la que va su propia tropa (Doc 5.4)', () => {
    const { sesion } = partidaConAsentamiento();
    const suyo = ejercito('e-huerfano', 'faccion-que-ya-no-es-suya', { x: 1000, y: 1000 }, [escuadron('s1', 'forastero')]);
    const estado = { ...sesion.getState(), ejercitos: [suyo] };

    const proyeccion = proyectarParaJugador(estado, 'forastero', SIN_GEOMETRIA);
    expect(proyeccion.faccionId).toBeNull();
    expect(proyeccion.ejercitos.map((e) => e.id)).toEqual(['e-huerfano']);
  });
});

describe('ejercitosAvistados: lo ajeno, solo si se ve y siempre redactado', () => {
  it('un ejercito rival LEJOS de todo lo propio no aparece por ningun lado', () => {
    const { sesion, fundador } = partidaConAsentamiento();
    const rival = ejercito('e-rival', 'faccion-rival', { x: 1900, y: 1900 }, [escuadron('s1', 'otro')]);
    const estado = { ...sesion.getState(), ejercitos: [rival] };

    const proyeccion = proyectarParaJugador(estado, fundador, SIN_GEOMETRIA);
    expect(proyeccion.ejercitos).toEqual([]);
    expect(proyeccion.ejercitosAvistados).toEqual([]);
  });

  it('un ejercito rival DENTRO de la zona de influencia propia se avista', () => {
    const { sesion, asentamientoId, fundador } = partidaConAsentamiento();
    const rival = ejercito('e-rival', 'faccion-rival', { x: 410, y: 410 }, [escuadron('s1', 'otro')]);
    const estado = { ...sesion.getState(), ejercitos: [rival] };

    const proyeccion = proyectarParaJugador(estado, fundador, zonaCuadrada(asentamientoId, { x: 400, y: 400 }, 30));
    expect(proyeccion.ejercitosAvistados.map((e) => e.id)).toEqual(['e-rival']);
  });

  it('la zona de influencia de un asentamiento AJENO no da vision, aunque llegue en la geometria', () => {
    const { sesion, fundador } = partidaConAsentamiento();
    const rival = ejercito('e-rival', 'faccion-rival', { x: 910, y: 910 }, [escuadron('s1', 'otro')]);
    const estado = { ...sesion.getState(), ejercitos: [rival] };

    const proyeccion = proyectarParaJugador(estado, fundador, zonaCuadrada('asentamiento-ajeno', { x: 900, y: 900 }, 30));
    expect(proyeccion.ejercitosAvistados).toEqual([]);
  });

  it('un ejercito propio avista lo que caiga en su radio de vision, y solo eso', () => {
    const { sesion, faccionId, fundador } = partidaConAsentamiento();
    const propio = ejercito('e-propio', faccionId, { x: 1000, y: 1000 }, [escuadron('s1', fundador)]);
    const dentro = ejercito('e-dentro', 'faccion-rival', { x: 1000 + LOGISTICA.radioVisionEjercito - 1, y: 1000 }, [escuadron('s2', 'otro')]);
    const fuera = ejercito('e-fuera', 'faccion-rival', { x: 1000 + LOGISTICA.radioVisionEjercito + 1, y: 1000 }, [escuadron('s3', 'otro')]);
    const estado = { ...sesion.getState(), ejercitos: [propio, dentro, fuera] };

    const proyeccion = proyectarParaJugador(estado, fundador, SIN_GEOMETRIA);
    expect(proyeccion.ejercitosAvistados.map((e) => e.id)).toEqual(['e-dentro']);
  });

  it('lo avistado NO lleva escuadrones, ruta, objetivo, estado, suministro ni origen: solo donde, de quien y cuantos', () => {
    const { sesion, faccionId, fundador } = partidaConAsentamiento();
    const propio = ejercito('e-propio', faccionId, { x: 1000, y: 1000 }, [escuadron('s1', fundador)]);
    // Tres escuadrones pero DOS jugadores: los rombos cuentan jugadores, no escuadrones (Doc 5.12.2).
    const rival = ejercito('e-rival', 'faccion-rival', { x: 1050, y: 1000 }, [
      escuadron('s2', 'rival-a'),
      escuadron('s3', 'rival-a'),
      escuadron('s4', 'rival-b'),
    ]);
    const estado = { ...sesion.getState(), ejercitos: [propio, rival] };

    const avistado = proyectarParaJugador(estado, fundador, SIN_GEOMETRIA).ejercitosAvistados[0]!;
    expect(avistado).toEqual({
      id: 'e-rival',
      faccionId: 'faccion-rival',
      posicionActual: { x: 1050, y: 1000 },
      participantes: 2,
    });
  });
});

describe('eventosDominio de campana: atribuidos a su origen, no globales', () => {
  it('lo que le pasa al ejercito de un rival NO llega al log de un jugador ajeno a esa Faccion', () => {
    const { sesion, fundador } = partidaConAsentamiento();
    const estado = sesion.getState();
    const conEventos = {
      ...estado,
      eventosDominio: [
        ...estado.eventosDominio,
        { codigo: 'ejercito.llega', mensaje: 'El ejercito e-rival llega a su destino y acampa.', momento: MOMENTO, asentamientoId: 'origen-rival', version: 99 },
      ],
    };

    const proyeccion = proyectarParaJugador(conEventos, fundador, SIN_GEOMETRIA);
    expect(proyeccion.eventosDominio.some((e) => e.codigo === 'ejercito.llega')).toBe(false);
  });
});
