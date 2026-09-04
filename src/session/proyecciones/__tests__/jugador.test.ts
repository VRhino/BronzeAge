// `proyectarParaJugador` sobre estado de partida GENUINO (misma fixture que `comandos/autorizacion.ts`), no
// mocks — así estas pruebas fallan de verdad si cambia la forma de `GameSessionState` o el significado de
// ciudadanía, no solo si cambia la proyección.
import { describe, expect, it } from 'vitest';
import { instanteDeTest } from '../../../engine/__tests__/fixtures';
import { partidaConAsentamiento, MOMENTO, OPC } from '../../__tests__/fixtures';
import { crearFaccion } from '../../comandos/crearFaccion';
import { fundarAsentamiento } from '../../comandos/fundarAsentamiento';
import { idDeMapa, type GameSessionState, type GeometriaAsentamientos } from '../../estado';
import { estaExplorado, marcarVisto, rejillaDe } from '../../../engine/exploracion';
import { MEMORIA_VACIA, type FichaConocida } from '../../../engine/memoria';
import { proyectarParaJugador } from '../jugador';
import type { Ejercito, Escuadron, Point } from '../../../domain/types';
import { EXPLORACION, VISION } from '../../../constants';

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

  it('el asentamiento de una Facción rival nunca entra en `asentamientos`, se vea o no', () => {
    const base = partidaConAsentamiento();
    const opcRival = { ...OPC, actor: 'rival' };
    const rf = base.sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, opcRival);
    base.sesion.ejecutar(fundarAsentamiento, { faccionId: rf.datos!.faccionId, posicion: { x: 900, y: 900 } }, opcRival);

    const proyeccion = proyectarParaJugador(base.sesion.getState(), base.fundador, SIN_GEOMETRIA);
    expect(proyeccion.asentamientos).toHaveLength(1); // solo el propio, no los 2 que existen en la partida
    expect(proyeccion.asentamientos[0]!.faccionId).toBe(base.faccionId);
  });
});

// Niebla de guerra, Paso 1 (Consideraciones/Niebla_De_Guerra_Definicion.md §2.1-2.2). Se funda una plaza
// rival de verdad en vez de inyectarla en el estado: asi las pruebas van contra `radioPotencial` real y
// contra el `nivel` real, no contra un objeto a mano que podria no parecerse a un asentamiento.
describe('asentamientosAvistados: la FICHA de lo ajeno, solo si se ve', () => {
  function conPlazaRivalEn(posicion: Point) {
    const base = partidaConAsentamiento();
    const opcRival = { ...OPC, actor: 'rival' };
    const rf = base.sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, opcRival);
    const ra = base.sesion.ejecutar(fundarAsentamiento, { faccionId: rf.datos!.faccionId, posicion }, opcRival);
    return { ...base, faccionRivalId: rf.datos!.faccionId, asentamientoRivalId: ra.datos!.asentamientoId };
  }

  it('una plaza rival LEJOS de todo lo propio no aparece ni redactada', () => {
    const { sesion, fundador } = conPlazaRivalEn({ x: 900, y: 900 });
    const proyeccion = proyectarParaJugador(sesion.getState(), fundador, SIN_GEOMETRIA);
    expect(proyeccion.asentamientosAvistados).toEqual([]);
  });

  it('una plaza rival dentro de lo que vigila la propia se avista, con su ficha y nada mas', () => {
    // El fixture funda en (400,400) con radio inicial 30: (400,470) cae dentro de 30+60=90.
    const { sesion, fundador, faccionRivalId, asentamientoRivalId } = conPlazaRivalEn({ x: 400, y: 470 });
    const rival = sesion.getState().asentamientos.find((a) => a.id === asentamientoRivalId)!;

    const proyeccion = proyectarParaJugador(sesion.getState(), fundador, SIN_GEOMETRIA);
    expect(proyeccion.asentamientosAvistados).toEqual([
      { id: asentamientoRivalId, nombre: rival.nombre, faccionId: faccionRivalId, posicion: { x: 400, y: 470 }, nivel: rival.nivel },
    ]);
  });

  it('lo avistado NO lleva almacen, escuadrones, edificios, colas ni cargos: es telemetria de rival', () => {
    const { sesion, fundador } = conPlazaRivalEn({ x: 400, y: 470 });
    const avistado = proyectarParaJugador(sesion.getState(), fundador, SIN_GEOMETRIA).asentamientosAvistados[0]!;

    expect(Object.keys(avistado).sort()).toEqual(['faccionId', 'id', 'nivel', 'nombre', 'posicion']);
  });

  it('un ejercito propio en marcha tambien avista plazas rivales, a su propio radio', () => {
    const { sesion, faccionId, fundador, asentamientoRivalId } = conPlazaRivalEn({ x: 900, y: 900 });
    // A 100 de la plaza rival, muy lejos de la propia: la unica vision posible es la de la columna.
    const explorador = ejercito('e-explorador', faccionId, { x: 1000, y: 900 }, [escuadron('s1', fundador)]);
    const estado = { ...sesion.getState(), ejercitos: [explorador] };

    const proyeccion = proyectarParaJugador(estado, fundador, SIN_GEOMETRIA);
    expect(proyeccion.asentamientosAvistados.map((a) => a.id)).toEqual([asentamientoRivalId]);
  });

  it('la plaza PROPIA no se cuela en lo avistado: ya viaja entera en `asentamientos`', () => {
    const { sesion, fundador, asentamientoId } = conPlazaRivalEn({ x: 400, y: 470 });
    const proyeccion = proyectarParaJugador(sesion.getState(), fundador, SIN_GEOMETRIA);
    expect(proyeccion.asentamientosAvistados.map((a) => a.id)).not.toContain(asentamientoId);
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

  it('un ejercito rival dentro de lo que vigila una plaza propia se avista', () => {
    const { sesion, fundador } = partidaConAsentamiento();
    const rival = ejercito('e-rival', 'faccion-rival', { x: 410, y: 410 }, [escuadron('s1', 'otro')]);
    const estado = { ...sesion.getState(), ejercitos: [rival] };

    const proyeccion = proyectarParaJugador(estado, fundador, SIN_GEOMETRIA);
    expect(proyeccion.ejercitosAvistados.map((e) => e.id)).toEqual(['e-rival']);
  });

  it('una plaza vigila su radio MAS el margen, y ni una unidad mas', () => {
    const { sesion, fundador } = partidaConAsentamiento();
    // El fixture funda en (400,400); el radio es el que tenga la plaza en ese momento, no un numero a mano.
    const propio = sesion.getState().asentamientos[0]!;
    const alcance = propio.radioPotencial + VISION.margenAsentamiento;
    const dentro = ejercito('e-dentro', 'faccion-rival', { x: 400 + alcance - 1, y: 400 }, [escuadron('s1', 'otro')]);
    const fuera = ejercito('e-fuera', 'faccion-rival', { x: 400 + alcance + 1, y: 400 }, [escuadron('s2', 'otro')]);
    const estado = { ...sesion.getState(), ejercitos: [dentro, fuera] };

    const proyeccion = proyectarParaJugador(estado, fundador, SIN_GEOMETRIA);
    expect(proyeccion.ejercitosAvistados.map((e) => e.id)).toEqual(['e-dentro']);
  });

  it('la GEOMETRIA no da vision: quien ve es la plaza, no el poligono que llegue por parametro', () => {
    // Regresion de la niebla Paso 1: la vision se media contra el poligono de zona, que viene recortado por
    // las fronteras rivales (`computeZonaInfluencia`). Ese recorte es politico, no optico. Ahora se mide
    // contra el disco de la propia plaza, asi que un poligono inyectado —propio o ajeno— no cambia nada.
    const { sesion, asentamientoId, fundador } = partidaConAsentamiento();
    const lejos = ejercito('e-lejos', 'faccion-rival', { x: 910, y: 910 }, [escuadron('s1', 'otro')]);
    const estado = { ...sesion.getState(), ejercitos: [lejos] };

    // Un cuadrado enorme que lo cubre, atribuido a la plaza PROPIA: sigue sin verse.
    const conZonaPropia = proyectarParaJugador(estado, fundador, zonaCuadrada(asentamientoId, { x: 900, y: 900 }, 30));
    expect(conZonaPropia.ejercitosAvistados).toEqual([]);
    // Y la zona de un asentamiento AJENO tampoco presta vision, obviamente.
    const conZonaAjena = proyectarParaJugador(estado, fundador, zonaCuadrada('asentamiento-ajeno', { x: 900, y: 900 }, 30));
    expect(conZonaAjena.ejercitosAvistados).toEqual([]);
  });

  it('un ejercito propio avista lo que caiga en su radio de vision, y solo eso', () => {
    const { sesion, faccionId, fundador } = partidaConAsentamiento();
    const propio = ejercito('e-propio', faccionId, { x: 1000, y: 1000 }, [escuadron('s1', fundador)]);
    const dentro = ejercito('e-dentro', 'faccion-rival', { x: 1000 + VISION.ejercito - 1, y: 1000 }, [escuadron('s2', 'otro')]);
    const fuera = ejercito('e-fuera', 'faccion-rival', { x: 1000 + VISION.ejercito + 1, y: 1000 }, [escuadron('s3', 'otro')]);
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

// Niebla de guerra, Paso 3: la MEMORIA proyectada. Los tres estados que ve el jugador, y sobre todo el
// transito entre ellos — "al dejar de verlo, cae en la categoria anterior", que era el punto que la primera
// version del diseño se dejaba fuera.
describe('la memoria proyectada: lo que se vio y ya no se ve', () => {
  /** La ficha que una Faccion recuerda de una plaza, inyectada en el estado como la habria dejado el tick. */
  function recordando(estado: GameSessionState, faccionId: string, ficha: FichaConocida): GameSessionState {
    const previa = estado.memoriaPorFaccion[faccionId] ?? MEMORIA_VACIA;
    return {
      ...estado,
      memoriaPorFaccion: {
        ...estado.memoriaPorFaccion,
        [faccionId]: { ...previa, asentamientos: { ...previa.asentamientos, [ficha.asentamientoId]: ficha } },
      },
    };
  }

  function fichaDe(asentamientoId: string, posicion: Point): FichaConocida {
    return { asentamientoId, nombre: 'Troya', faccionId: 'faccion-rival', posicion, nivel: 2, conocidoEn: instanteDeTest(3) };
  }

  it('sin Faccion no hay memoria: listas vacias, nunca undefined', () => {
    const { sesion } = partidaConAsentamiento();
    const proyeccion = proyectarParaJugador(sesion.getState(), 'forastero', SIN_GEOMETRIA);

    expect(proyeccion.asentamientosConocidos).toEqual([]);
    expect(proyeccion.exploracion.celdas).toBe('');
    expect(proyeccion.exploracion.tamanoCelda).toBeGreaterThan(0);
  });

  it('lo que se vio y ya no se ve viaja como RECUERDO, con su instante', () => {
    const { sesion, faccionId, fundador } = partidaConAsentamiento();
    const ficha = fichaDe('asentamiento-lejano', { x: 1700, y: 1700 });
    const estado = recordando(sesion.getState(), faccionId, ficha);

    const proyeccion = proyectarParaJugador(estado, fundador, SIN_GEOMETRIA);
    expect(proyeccion.asentamientosConocidos).toEqual([ficha]);
    expect(proyeccion.asentamientosAvistados).toEqual([]);
  });

  it('lo que se ve Y ademas se recuerda aparece UNA sola vez, y como visto', () => {
    // La plaza rival esta a 70 de la propia, dentro de lo que vigila: se ve en vivo. Y ademas se recuerda,
    // con una foto vieja que dice nivel 2. Debe ganar la de en vivo.
    const base = partidaConAsentamiento();
    const opcRival = { ...OPC, actor: 'rival' };
    const rf = base.sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, opcRival);
    const ra = base.sesion.ejecutar(fundarAsentamiento, { faccionId: rf.datos!.faccionId, posicion: { x: 400, y: 470 } }, opcRival);
    const rivalId = ra.datos!.asentamientoId;
    const estado = recordando(base.sesion.getState(), base.faccionId, fichaDe(rivalId, { x: 400, y: 470 }));

    const proyeccion = proyectarParaJugador(estado, base.fundador, SIN_GEOMETRIA);
    expect(proyeccion.asentamientosAvistados.map((a) => a.id)).toEqual([rivalId]);
    expect(proyeccion.asentamientosConocidos).toEqual([]);
    // Y lo que viaja es el nivel REAL, no el 2 de la foto vieja.
    expect(proyeccion.asentamientosAvistados[0]!.nivel).toBe(1);
  });

  it('una plaza que se recordaba y que ahora es PROPIA no se proyecta como recuerdo: ya viaja entera', () => {
    const { sesion, faccionId, fundador, asentamientoId } = partidaConAsentamiento();
    const estado = recordando(sesion.getState(), faccionId, fichaDe(asentamientoId, { x: 400, y: 400 }));

    const proyeccion = proyectarParaJugador(estado, fundador, SIN_GEOMETRIA);
    expect(proyeccion.asentamientosConocidos).toEqual([]);
    expect(proyeccion.asentamientos.map((a) => a.id)).toEqual([asentamientoId]);
  });

  it('lo recordado NO se refresca solo: la foto es de cuando se tomo, aunque la plaza real haya cambiado', () => {
    const base = partidaConAsentamiento();
    const opcRival = { ...OPC, actor: 'rival' };
    const rf = base.sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, opcRival);
    const ra = base.sesion.ejecutar(fundarAsentamiento, { faccionId: rf.datos!.faccionId, posicion: { x: 1500, y: 1500 } }, opcRival);
    const rivalId = ra.datos!.asentamientoId;
    // Se recuerda en una posicion y un nivel que YA no son los reales.
    const estado = recordando(base.sesion.getState(), base.faccionId, fichaDe(rivalId, { x: 900, y: 900 }));

    const conocida = proyectarParaJugador(estado, base.fundador, SIN_GEOMETRIA).asentamientosConocidos[0]!;
    expect(conocida.posicion).toEqual({ x: 900, y: 900 });
    expect(conocida.nivel).toBe(2);
    expect(conocida.conocidoEn).toBe(instanteDeTest(3));
  });
});

describe('la exploracion proyectada: la mascara que tapa el terreno', () => {
  it('incluye lo que se ve AHORA aunque el tick no lo haya grabado todavia', () => {
    // Recien fundada y sin un solo tick corrido: `memoriaPorFaccion` esta vacia. Aun asi el jugador no puede
    // ver niebla encima de su propia plaza.
    const { sesion, fundador } = partidaConAsentamiento();
    const estado = sesion.getState();
    expect(estado.memoriaPorFaccion).toEqual({});

    const { exploracion } = proyectarParaJugador(estado, fundador, SIN_GEOMETRIA);
    const rejilla = { columnas: exploracion.columnas, filas: exploracion.filas, tamanoCelda: exploracion.tamanoCelda };
    expect(estaExplorado(exploracion.celdas, rejilla, { x: 400, y: 400 })).toBe(true);
    expect(estaExplorado(exploracion.celdas, rejilla, { x: 1800, y: 1800 })).toBe(false);
  });

  it('viaja con la geometria que hace falta para descifrarla', () => {
    const { sesion, fundador } = partidaConAsentamiento();
    const { exploracion } = proyectarParaJugador(sesion.getState(), fundador, SIN_GEOMETRIA);

    expect(exploracion.tamanoCelda).toBe(EXPLORACION.tamanoCelda);
    expect(exploracion.columnas * exploracion.tamanoCelda).toBeGreaterThanOrEqual(2000);
    expect(exploracion.filas * exploracion.tamanoCelda).toBeGreaterThanOrEqual(2000);
  });

  it('lo VISIBLE es un subconjunto de lo explorado: no se puede ver lo que no se ha explorado', () => {
    // Es la relacion de la que cuelgan los tres estados. Si se rompiera, habria celdas "visibles pero nunca
    // vistas" y el cliente tendria que decidir cual de las dos mascaras miente.
    const { sesion, faccionId, fundador } = partidaConAsentamiento();
    const estado = sesion.getState();
    const rejilla = rejillaDe(estado.mapa.config);
    const conMemoria: GameSessionState = {
      ...estado,
      memoriaPorFaccion: { [faccionId]: { exploracion: marcarVisto('', rejilla, { x: 1700, y: 1700 }, 100), asentamientos: {} } },
    };

    const { exploracion } = proyectarParaJugador(conMemoria, fundador, SIN_GEOMETRIA);
    for (let fila = 0; fila < exploracion.filas; fila++) {
      for (let columna = 0; columna < exploracion.columnas; columna++) {
        const punto = { x: columna * exploracion.tamanoCelda + 1, y: fila * exploracion.tamanoCelda + 1 };
        if (estaExplorado(exploracion.visibles, rejilla, punto)) {
          expect(estaExplorado(exploracion.celdas, rejilla, punto)).toBe(true);
        }
      }
    }
  });

  it('lo RECORDADO no esta en la mascara de visible: es justo lo que el cliente pinta oscuro', () => {
    const { sesion, faccionId, fundador } = partidaConAsentamiento();
    const estado = sesion.getState();
    const rejilla = rejillaDe(estado.mapa.config);
    const lejos = { x: 1700, y: 1700 };
    const conMemoria: GameSessionState = {
      ...estado,
      memoriaPorFaccion: { [faccionId]: { exploracion: marcarVisto('', rejilla, lejos, 100), asentamientos: {} } },
    };

    const { exploracion } = proyectarParaJugador(conMemoria, fundador, SIN_GEOMETRIA);
    // El rincon recordado: explorado SI, visible NO -> estado 2.
    expect(estaExplorado(exploracion.celdas, rejilla, lejos)).toBe(true);
    expect(estaExplorado(exploracion.visibles, rejilla, lejos)).toBe(false);
    // La propia plaza: las dos cosas -> estado 3.
    expect(estaExplorado(exploracion.celdas, rejilla, { x: 400, y: 400 })).toBe(true);
    expect(estaExplorado(exploracion.visibles, rejilla, { x: 400, y: 400 })).toBe(true);
  });

  it('sin Faccion las dos mascaras estan vacias', () => {
    const { sesion } = partidaConAsentamiento();
    const { exploracion } = proyectarParaJugador(sesion.getState(), 'forastero', SIN_GEOMETRIA);
    expect(exploracion.celdas).toBe('');
    expect(exploracion.visibles).toBe('');
  });

  it('lo GRABADO no se pierde al proyectar: la mascara es memoria mas vista, no solo vista', () => {
    const { sesion, faccionId, fundador } = partidaConAsentamiento();
    const estado = sesion.getState();
    const rejilla = rejillaDe(estado.mapa.config);
    // Una Faccion que en su dia exploro el otro extremo del mundo, donde hoy no tiene nada.
    const conMemoria: GameSessionState = {
      ...estado,
      memoriaPorFaccion: { [faccionId]: { exploracion: marcarVisto('', rejilla, { x: 1700, y: 1700 }, 100), asentamientos: {} } },
    };

    const { exploracion } = proyectarParaJugador(conMemoria, fundador, SIN_GEOMETRIA);
    expect(estaExplorado(exploracion.celdas, rejilla, { x: 1700, y: 1700 })).toBe(true);
    expect(estaExplorado(exploracion.celdas, rejilla, { x: 400, y: 400 })).toBe(true);
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
