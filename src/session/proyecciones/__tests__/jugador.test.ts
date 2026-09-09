// `proyectarParaJugador` sobre estado de partida GENUINO (misma fixture que `comandos/autorizacion.ts`), no
// mocks — así estas pruebas fallan de verdad si cambia la forma de `GameSessionState` o el significado de
// ciudadanía, no solo si cambia la proyección.
import { describe, expect, it } from 'vitest';
import { instante } from '../../../domain/tiempo';
import { GameSession } from '../../gameSession';
import { instanteDeTest } from '../../../engine/__tests__/fixtures';
import { enPie, partidaConAsentamiento, MOMENTO, OPC } from '../../__tests__/fixtures';
import { crearFaccion } from '../../comandos/crearFaccion';
import { fundarAsentamiento } from '../../comandos/fundarAsentamiento';
import { entrarEnAsentamiento, salirAlMundo } from '../../comandos/presencia';
import { idDeMapa, type GameSessionState, type GeometriaAsentamientos } from '../../estado';
import { computeTodasLasZonas } from '../../../engine/zones';
import { estaExplorado, marcarVisto, rejillaDe } from '../../../engine/exploracion';
import { MEMORIA_VACIA, type FichaConocida } from '../../../engine/memoria';
import { eventosDominioParaJugador, proyectarParaJugador } from '../jugador';
import type { Asentamiento, CaminoComercial, CampamentoBandido, Ejercito, Escuadron, Point } from '../../../domain/types';
import { EXPLORACION, VISION, ZONA_INFLUENCIA } from '../../../constants';

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

describe('asentamientos: SOLO el interior de la plaza que se pisa (Doc 1.10.1)', () => {
  it('el fundador ve su asentamiento porque está DENTRO de él', () => {
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
  it('al salir al mundo deja de ver el interior de su propia ciudad, y pasa a verla como ficha', () => {
    // Es el corazón de Doc 13b: la ciudadanía habilita, la presencia ejerce. Un Gobernador de campaña no ve
    // su almacén desde el camino.
    const { sesion, asentamientoId, fundador } = partidaConAsentamiento();
    sesion.ejecutar(salirAlMundo, { asentamientoId, jugadorId: fundador, escuadronIds: [], carga: {} }, { ...OPC, actor: fundador });

    const proyeccion = proyectarParaJugador(sesion.getState(), fundador, SIN_GEOMETRIA);

    expect(proyeccion.asentamientos, 'ya no pisa ninguna plaza').toEqual([]);
    expect(proyeccion.asentamientosAvistados.map((a) => a.id), 'pero la sigue viendo desde fuera').toEqual([asentamientoId]);
  });

  it('y al volver a entrar lo recupera', () => {
    const { sesion, asentamientoId, fundador } = partidaConAsentamiento();
    const opc = { ...OPC, actor: fundador };
    sesion.ejecutar(salirAlMundo, { asentamientoId, jugadorId: fundador, escuadronIds: [], carga: {} }, opc);
    sesion.ejecutar(entrarEnAsentamiento, { asentamientoId, jugadorId: fundador }, opc);

    const proyeccion = proyectarParaJugador(sesion.getState(), fundador, SIN_GEOMETRIA);

    expect(proyeccion.asentamientos.map((a) => a.id)).toEqual([asentamientoId]);
    expect(proyeccion.asentamientosAvistados).toEqual([]);
  });

  it('de OTRA plaza de su propia Facción solo ve la ficha, aunque sea suya', () => {
    // Lo que cambia con el jugador situado: antes viajaban COMPLETOS todos los asentamientos propios.
    //
    // La segunda plaza se inyecta en el estado en vez de fundarse: una Facción que ya tiene asentamiento se
    // expande con caravana de fundación, no con `fundarAsentamiento`, y eso es maquinaria ajena a lo que se
    // prueba aquí. Una plaza propia se ve a sí misma, así que entra en `avistados` esté donde esté.
    const base = partidaConAsentamiento();
    const payload = base.sesion.exportar();
    const primera = payload.state.asentamientos[0]!;
    const sesion = GameSession.importar({
      ...payload,
      state: {
        ...payload.state,
        asentamientos: [primera, { ...primera, id: 'segunda-plaza', posicion: { x: 900, y: 900 }, jugadoresFundadoresIds: ['colono'], casasCompradas: [] }],
      },
    });

    const proyeccion = proyectarParaJugador(sesion.getState(), base.fundador, SIN_GEOMETRIA);

    expect(proyeccion.asentamientos.map((a) => a.id), 'solo la que pisa').toEqual([base.asentamientoId]);
    expect(
      proyeccion.asentamientosAvistados.map((a) => a.id),
      'la otra viaja redactada, sin almacén ni cola ni guarnición'
    ).toEqual(['segunda-plaza']);
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
    // Se funda donde se está (Doc 1.3): se lleva al rival al punto elegido antes de fundar.
    base.sesion = enPie(base.sesion, 'rival', posicion);
    const ra = base.sesion.ejecutar(fundarAsentamiento, { faccionId: rf.datos!.faccionId }, opcRival);
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
      // `SIN_GEOMETRIA` no trae zonas, asi que el contorno viaja vacio en vez de romper la proyeccion — el
      // caso con geometria de verdad lo cubre el test de mas abajo.
      {
        id: asentamientoRivalId,
        nombre: rival.nombre,
        faccionId: faccionRivalId,
        posicion: { x: 400, y: 470 },
        nivel: rival.nivel,
        zona: [],
        // Lo que tiene EN PIE se ve desde fuera; ni cargos ni politicas, que son de puertas adentro.
        edificios: rival.edificios.filter((e) => e.estado === 'activo'),
      },
    ]);
  });

  it('la zona que viaja es la SILUETA REAL del motor, recortada contra los vecinos', () => {
    const base = conPlazaRivalEn({ x: 400, y: 470 });
    // Las dos plazas se crecen hasta que sus discos se pisan: al fundar no puede haber recorte (fundar
    // exige no solapar, y con radios iguales la frontera cae siempre fuera de los dos circulos), asi que
    // sin esto el poligono seria un circulo entero y el test no probaria nada.
    const estado = {
      ...base.sesion.getState(),
      asentamientos: base.sesion.getState().asentamientos.map((a) => ({ ...a, radioPotencial: 60 })),
    };
    const zonas = computeTodasLasZonas(estado.asentamientos);
    const geometria: GeometriaAsentamientos = { zonas, zonasFusionadas: [], trazadoPorAsentamiento: {} };

    const avistado = proyectarParaJugador(estado, base.fundador, geometria).asentamientosAvistados[0]!;
    expect(avistado.zona).toEqual(zonas.find((z) => z.asentamientoId === base.asentamientoRivalId)!.poligono);

    // Y esta recortada DE VERDAD, que es lo que separa "silueta real" de "circulo pintado". La señal es el
    // numero de vertices, no su distancia al centro: recortar un circulo le quita el arco que sobra y deja
    // una cuerda, pero los vertices que quedan —los de la circunferencia y los dos de corte— siguen todos
    // a la distancia del radio. Contar es lo unico que distingue las dos cosas.
    expect(avistado.zona.length).toBeLessThan(ZONA_INFLUENCIA.segmentosPoligono);
    expect(avistado.zona.length).toBeGreaterThan(2);
    // La cuerda: un lado mucho mas largo que el resto, que es justo la frontera con la plaza propia.
    const lados = avistado.zona.map((p, i) => {
      const q = avistado.zona[(i + 1) % avistado.zona.length]!;
      return Math.hypot(q.x - p.x, q.y - p.y);
    });
    expect(Math.max(...lados)).toBeGreaterThan(5 * (Math.min(...lados) || 1));
  });

  it('lo avistado de un RIVAL no lleva almacen, guarnicion, cola ni cargos', () => {
    const { sesion, fundador } = conPlazaRivalEn({ x: 400, y: 470 });
    const avistado = proyectarParaJugador(sesion.getState(), fundador, SIN_GEOMETRIA).asentamientosAvistados[0]!;

    // Dos cosas SI entran, y las dos por decision del usuario:
    //  - la ZONA (2026-09-05): una frontera esta marcada sobre el terreno y quien pasa por delante la ve;
    //  - los EDIFICIOS en pie (2026-09-06): se ve lo levantado, no lo planeado.
    // Lo que sigue fuera es lo que decide una guerra: que tiene guardado, con que se defiende, que esta
    // construyendo y quien manda.
    expect(Object.keys(avistado).sort()).toEqual(['edificios', 'faccionId', 'id', 'nivel', 'nombre', 'posicion', 'zona']);
    expect(avistado.edificios.every((e) => e.estado === 'activo'), 'la cola es privada').toBe(true);
  });

  it('de una plaza de TU Faccion se ve ademas quien manda y bajo que politicas', () => {
    // Dentro de casa los cargos son publicos; desde fuera no se sabe ni quien gobierna.
    const base = partidaConAsentamiento();
    const payload = base.sesion.exportar();
    const primera = payload.state.asentamientos[0]!;
    const sesion = GameSession.importar({
      ...payload,
      state: {
        ...payload.state,
        asentamientos: [primera, { ...primera, id: 'segunda-plaza', posicion: { x: 900, y: 900 }, jugadoresFundadoresIds: ['colono'], casasCompradas: [] }],
      },
    });

    const propia = proyectarParaJugador(sesion.getState(), base.fundador, SIN_GEOMETRIA).asentamientosAvistados.find((a) => a.id === 'segunda-plaza')!;

    expect(propia.cargos, 'de los tuyos sabes quien manda').toBeDefined();
    expect(propia.politicasActivas).toBeDefined();
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
        { id: 'o1', asentamientoId, tipo: 'venta' as const, recurso: 'trigo', cantidad: 10, cantidadCumplida: 0, precioUnitario: 1, creadoEn: instanteDeTest(0), expiraEn: instanteDeTest(200), estado: 'activa' as const },
        { id: 'o2', asentamientoId: 'asentamiento-ajeno', tipo: 'venta' as const, recurso: 'trigo', cantidad: 10, cantidadCumplida: 0, precioUnitario: 1, creadoEn: instanteDeTest(0), expiraEn: instanteDeTest(200), estado: 'activa' as const },
      ],
    };

    const proyeccion = proyectarParaJugador(estadoConOrdenes, fundador, SIN_GEOMETRIA);
    expect(proyeccion.ordenes.map((o) => o.id)).toEqual(['o1']);
  });

  // Desde que las ordenes se cumplen EN EL MOSTRADOR (`comerciarEnPlaza`), el filtro de arriba a secas dejaria
  // la mecanica inusable: habria que comprar a ciegas en una plaza ajena. La regla es la de un mercado de
  // verdad — ensena sus ofertas a quien esta dentro, y solo a ese.
  it('el escaparate de una plaza AJENA se ve al estar en su puerta, y solo lo que sigue en pie', () => {
    const { sesion, asentamientoId, fundador } = partidaConAsentamiento();
    const base = sesion.getState();
    const plazaAjena: Asentamiento = {
      ...base.asentamientos.find((a) => a.id === asentamientoId)!,
      id: 'plaza-ajena',
      faccionId: 'faccion-ajena',
      posicion: { x: 1200, y: 1200 },
      jugadoresFundadoresIds: [],
    };
    const orden = (id: string, aId: string, estado: 'activa' | 'cumplida') => ({
      id,
      asentamientoId: aId,
      tipo: 'venta' as const,
      recurso: 'trigo',
      cantidad: 10,
      cantidadCumplida: 0,
      precioUnitario: 1,
      creadoEn: instanteDeTest(0),
      expiraEn: instanteDeTest(200),
      estado,
    });
    const columna = {
      ...base.ejercitos[0],
      id: 'columna-visitante',
      faccionId: base.asentamientos.find((a) => a.id === asentamientoId)!.faccionId,
      liderId: fundador,
      tipo: 'personal',
      participantes: [{ jugadorId: fundador, unidoEn: instanteDeTest(0) }],
      escuadrones: [],
      suministro: {},
      caravanasAdjuntasIds: [],
      posicionActual: plazaAjena.posicion,
      estado: 'estacionado',
    } as unknown as Ejercito;

    const estado = {
      ...base,
      asentamientos: [...base.asentamientos, plazaAjena],
      ejercitos: [columna],
      ordenes: [orden('propia', asentamientoId, 'activa'), orden('ajena-viva', 'plaza-ajena', 'activa'), orden('ajena-cerrada', 'plaza-ajena', 'cumplida')],
    };

    const proyeccion = proyectarParaJugador(estado, fundador, SIN_GEOMETRIA);
    expect(proyeccion.ordenes.map((o) => o.id).sort()).toEqual(['ajena-viva', 'propia']);
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

// El filtro de propiedad de los eventos es la parte con valor de SEGURIDAD de todo esto, y sigue viva: desde
// el 2026-09-05 `eventosDominio` no viaja en la proyección (follow-up de C13) y estos tests apuntan a
// `eventosDominioParaJugador`, el cursor, que es donde el filtro vive ahora. Se mueven en vez de borrarse
// justamente porque lo que protegen —que un jugador no vea lo que le pasa a un rival— no ha cambiado.
describe('eventosDominioParaJugador: sin asentamientoId (globales) o con uno propio', () => {
  it('el evento de fundación (con asentamientoId propio) pasa el filtro', () => {
    const { sesion, asentamientoId, fundador } = partidaConAsentamiento();
    const eventos = eventosDominioParaJugador(sesion.getState(), fundador, 0);
    expect(eventos.some((e) => e.asentamientoId === asentamientoId)).toBe(true);
  });

  it('un evento de asentamiento AJENO no le llega a un jugador sin ese asentamiento', () => {
    const base = partidaConAsentamiento();
    const opcRival = { ...OPC, actor: 'rival' };
    const rf = base.sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, opcRival);
    const ra = base.sesion.ejecutar(fundarAsentamiento, { faccionId: rf.datos!.faccionId, posicion: { x: 900, y: 900 } }, opcRival);

    const eventos = eventosDominioParaJugador(base.sesion.getState(), base.fundador, 0);
    expect(eventos.some((e) => e.asentamientoId === ra.datos!.asentamientoId)).toBe(false);
  });

  it('la proyección ya NO los lleva: se piden por el cursor, no en cada lectura de estado', () => {
    // Lo que cierra el follow-up de C13. `eventosDominio` era el 88 % de una lectura de estado y crecía sin
    // techo; ahora se pide una vez y se extiende con `?desde=<version>`.
    const { sesion, fundador } = partidaConAsentamiento();
    expect(proyectarParaJugador(sesion.getState(), fundador, SIN_GEOMETRIA)).not.toHaveProperty('eventosDominio');
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

describe('mapaId, relaciones y titulos: públicos, sin filtrar', () => {
  it('relaciones y titulos viajan tal cual desde el estado', () => {
    const { sesion, fundador } = partidaConAsentamiento();
    const estado = sesion.getState();
    const proyeccion = proyectarParaJugador(estado, fundador, SIN_GEOMETRIA);

    expect(proyeccion.relaciones).toBe(estado.relaciones);
    expect(proyeccion.titulos).toBe(estado.titulos);
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
    participantes: [...new Set(escuadrones.map((e) => e.jugadorId))].map((jugadorId) => ({ jugadorId, unidoEn: instante(0) })),
    tipo: 'ejercito',
    politicaDeUnion: 'rechazar',
    liderId: escuadrones[0]?.jugadorId ?? 'j1',
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

// Niebla de guerra, Paso 4 (2026-09-09): un aliado —o un señor/vasallo— ve lo que ves tu, EN VIVO. Se suma a
// la capa "viendolo ahora" (avistados, mascara `visibles`), nunca a la memoria ni a lo explorado (`celdas`):
// al romperse la relacion desaparece en la proyeccion siguiente. Alianza Y vasallaje comparten vision.
describe('vision compartida por alianza y vasallaje (Paso 4)', () => {
  /**
   * Partida propia en (400,400). Una Faccion `esparta` con una columna lejos, en (2000,2000). Un ejercito de
   * `faccion-rival` a un paso de esa columna, fuera del alcance de todo lo propio. Devuelve un estado sin
   * relacion todavia — cada test pone la que quiere probar.
   */
  function conAliadoLejano() {
    const base = partidaConAsentamiento();
    const rf = base.sesion.ejecutar(crearFaccion, { nombre: 'Esparta' }, { ...OPC, actor: 'espartano' });
    const espartaId = rf.datos!.faccionId as string;
    // Lejos de la plaza propia (400,400) — su alcance es ~90 — y dentro del mapa de 2000x2000.
    const columnaAliada = ejercito('e-aliado', espartaId, { x: 1700, y: 1700 }, [escuadron('sa', 'espartano')]);
    const rivalCerca = ejercito('e-rival', 'faccion-rival', { x: 1700, y: 1700 + VISION.ejercito - 1 }, [escuadron('sr', 'otro')]);
    const rivalLejos = ejercito('e-lejos', 'faccion-rival', { x: 1700, y: 300 }, [escuadron('sl', 'otro')]);
    const estado: GameSessionState = {
      ...base.sesion.getState(),
      ejercitos: [columnaAliada, rivalCerca, rivalLejos],
    };
    return { estado, fundador: base.fundador, faccionId: base.faccionId, espartaId };
  }

  const relacion = (tipo: 'alianza' | 'vasallaje', a: string, b: string, estado: 'activa' | 'rota') => ({
    id: `rel-${tipo}`,
    tipo,
    faccionAId: a,
    faccionBId: b,
    creadoEn: instante(0),
    estado,
  });

  it('sin relacion, la columna del aliado NO presta vision', () => {
    const { estado, fundador } = conAliadoLejano();
    const proyeccion = proyectarParaJugador(estado, fundador, SIN_GEOMETRIA);
    expect(proyeccion.ejercitosAvistados.map((e) => e.id).sort()).toEqual([]);
  });

  it('con ALIANZA activa se ve lo que ve el aliado, pero NO entra en la memoria ni en lo explorado', () => {
    const { estado, fundador, faccionId, espartaId } = conAliadoLejano();
    const conAlianza: GameSessionState = { ...estado, relaciones: [relacion('alianza', faccionId, espartaId, 'activa')] };

    const proyeccion = proyectarParaJugador(conAlianza, fundador, SIN_GEOMETRIA);

    // El rival pegado a la columna aliada se ve; el que esta en la otra punta del mapa, no. La propia columna
    // del aliado tambien aparece como avistada (no es "mia", pero la veo).
    expect(proyeccion.ejercitosAvistados.map((e) => e.id).sort()).toEqual(['e-aliado', 'e-rival']);

    const rejilla = {
      columnas: proyeccion.exploracion.columnas,
      filas: proyeccion.exploracion.filas,
      tamanoCelda: proyeccion.exploracion.tamanoCelda,
    };
    // Lo que ve el aliado esta en `visibles` (en vivo) pero NO en `celdas` (explorado/memoria): la vision
    // compartida no se graba.
    expect(estaExplorado(proyeccion.exploracion.visibles, rejilla, { x: 1700, y: 1700 })).toBe(true);
    expect(estaExplorado(proyeccion.exploracion.celdas, rejilla, { x: 1700, y: 1700 })).toBe(false);
    expect(proyeccion.asentamientosConocidos).toEqual([]);
  });

  it('un VASALLAJE tambien comparte vision, en las dos direcciones', () => {
    const { estado, fundador, faccionId, espartaId } = conAliadoLejano();
    // Da igual quien es el señor y quien el vasallo: la relacion es simetrica para la vista.
    const comoSenor: GameSessionState = { ...estado, relaciones: [relacion('vasallaje', faccionId, espartaId, 'activa')] };
    const comoVasallo: GameSessionState = { ...estado, relaciones: [relacion('vasallaje', espartaId, faccionId, 'activa')] };

    expect(proyectarParaJugador(comoSenor, fundador, SIN_GEOMETRIA).ejercitosAvistados.map((e) => e.id)).toContain('e-rival');
    expect(proyectarParaJugador(comoVasallo, fundador, SIN_GEOMETRIA).ejercitosAvistados.map((e) => e.id)).toContain('e-rival');
  });

  it('al romperse la relacion, lo que solo se veia por ella DESAPARECE', () => {
    const { estado, fundador, faccionId, espartaId } = conAliadoLejano();
    const rota: GameSessionState = { ...estado, relaciones: [relacion('alianza', faccionId, espartaId, 'rota')] };

    const proyeccion = proyectarParaJugador(rota, fundador, SIN_GEOMETRIA);
    expect(proyeccion.ejercitosAvistados.map((e) => e.id)).not.toContain('e-rival');
    expect(proyeccion.ejercitosAvistados.map((e) => e.id)).not.toContain('e-aliado');
  });
});

// Niebla de guerra, Paso 3: la MEMORIA proyectada. Los tres estados que ve el jugador, y sobre todo el
// transito entre ellos — "al dejar de verlo, cae en la categoria anterior", que era el punto que la primera
// version del diseño se dejaba fuera.
// A peticion del usuario (2026-09-05): "deberia poder saber si estoy en el territorio de otro cuando voy
// caminando". Lo resuelve el servidor y no el cliente porque el cliente solo tiene las zonas de lo que ve.
describe('territorioPorEjercito: de quien es el suelo que pisas', () => {
  /** Partida con plaza propia en (400,400) y una rival en (400,470), las dos crecidas hasta radio 60. */
  function dosPlazasVecinas() {
    const base = partidaConAsentamiento();
    const opcRival = { ...OPC, actor: 'rival' };
    const rf = base.sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, opcRival);
    // Se funda donde se está (Doc 1.3): se lleva al rival al punto elegido antes de fundar.
    base.sesion = enPie(base.sesion, 'rival', { x: 400, y: 470 });
    base.sesion.ejecutar(fundarAsentamiento, { faccionId: rf.datos!.faccionId }, opcRival);
    const asentamientos = base.sesion.getState().asentamientos.map((a) => ({ ...a, radioPotencial: 60 }));
    const zonas = computeTodasLasZonas(asentamientos);
    return {
      ...base,
      faccionRivalId: rf.datos!.faccionId,
      asentamientos,
      geometria: { zonas, zonasFusionadas: [], trazadoPorAsentamiento: {} } as GeometriaAsentamientos,
    };
  }

  function conEjercitoEn(punto: Point) {
    const d = dosPlazasVecinas();
    const columna = ejercito('e-propio', d.faccionId, punto, [escuadron('s1', d.fundador)]);
    return { ...d, estado: { ...d.sesion.getState(), asentamientos: d.asentamientos, ejercitos: [columna] } };
  }

  it('marchando por tierra de nadie, no hay entrada: el silencio es "campo abierto"', () => {
    const { estado, fundador, geometria } = conEjercitoEn({ x: 1500, y: 1500 });
    expect(proyectarParaJugador(estado, fundador, geometria).territorioPorEjercito).toEqual({});
  });

  it('marchando por tierra RIVAL, dice de que Faccion es', () => {
    // (400,500) esta a 30 de la plaza rival y a 100 de la propia: cae de lleno en la zona de Troya.
    const { estado, fundador, geometria, faccionRivalId } = conEjercitoEn({ x: 400, y: 500 });
    expect(proyectarParaJugador(estado, fundador, geometria).territorioPorEjercito).toEqual({ 'e-propio': faccionRivalId });
  });

  it('marchando por tierra PROPIA tambien lo dice: sirve igual para "estas en casa"', () => {
    const { estado, fundador, geometria, faccionId } = conEjercitoEn({ x: 400, y: 380 });
    expect(proyectarParaJugador(estado, fundador, geometria).territorioPorEjercito).toEqual({ 'e-propio': faccionId });
  });

  it('funciona aunque la ciudad que manda en esa tierra NO se vea: es el caso que lo justifica', () => {
    // Una capital vigila hasta 240 y una columna ve 150, asi que se puede estar dentro de su territorio sin
    // haberla divisado. Aqui se fuerza ese hueco: la plaza rival con radio 300 y el ejercito a 200 de ella,
    // lejos de todo lo propio — dentro de su zona, pero fuera de lo que la columna alcanza a ver.
    const base = partidaConAsentamiento();
    const opcRival = { ...OPC, actor: 'rival' };
    const rf = base.sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, opcRival);
    // Se funda donde se está (Doc 1.3): se lleva al rival al punto elegido antes de fundar.
    base.sesion = enPie(base.sesion, 'rival', { x: 1500, y: 1500 });
    base.sesion.ejecutar(fundarAsentamiento, { faccionId: rf.datos!.faccionId }, opcRival);
    const asentamientos = base.sesion.getState().asentamientos.map((a) =>
      a.posicion.x === 1500 ? { ...a, radioPotencial: 300 } : a
    );
    const geometria: GeometriaAsentamientos = {
      zonas: computeTodasLasZonas(asentamientos),
      zonasFusionadas: [],
      trazadoPorAsentamiento: {},
    };
    const columna = ejercito('e-propio', base.faccionId, { x: 1700, y: 1500 }, [escuadron('s1', base.fundador)]);
    const estado = { ...base.sesion.getState(), asentamientos, ejercitos: [columna] };

    const proyeccion = proyectarParaJugador(estado, base.fundador, geometria);
    expect(proyeccion.asentamientosAvistados).toEqual([]); // no la ve...
    expect(proyeccion.territorioPorEjercito).toEqual({ 'e-propio': rf.datos!.faccionId }); // ...pero pisa su tierra
  });

  it('sin ejercitos, el mapa va vacio y no se recorre ni una zona', () => {
    const { sesion, fundador } = partidaConAsentamiento();
    expect(proyectarParaJugador(sesion.getState(), fundador, SIN_GEOMETRIA).territorioPorEjercito).toEqual({});
  });
});

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
    // Se funda donde se está (Doc 1.3): se lleva al rival al punto elegido antes de fundar.
    base.sesion = enPie(base.sesion, 'rival', { x: 400, y: 470 });
    const ra = base.sesion.ejecutar(fundarAsentamiento, { faccionId: rf.datos!.faccionId }, opcRival);
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

// Lo que hay en el MUNDO y no es de nadie tambien pasa por la niebla, cada cosa con SU regla: los caminos
// por lo EXPLORADO (son infraestructura estatica, como el terreno) y los campamentos de bandidos por lo que
// se ve AHORA (aparecen y desaparecen, asi que no tienen memoria). La pareja de tests del final congela
// justo esa asimetria: mismo rincon recordado, resultados opuestos.
const LEJOS: Point = { x: 1700, y: 1700 };

function caminoPor(...puntos: Point[]): CaminoComercial {
  return { id: 'cam-1', asentamientoAId: 'a-1', asentamientoBId: 'a-2', puntos };
}

function campamentoEn(posicion: Point): CampamentoBandido {
  return { id: 'camp-1', posicion, bosqueId: 'b-1', asentamientoId: 'a-1', poder: 50 };
}

/** Estado con memoria de haber explorado `LEJOS` en su dia, donde hoy la Faccion no tiene ni ojos ni nada. */
function conRinconRecordado(estado: GameSessionState, faccionId: string): GameSessionState {
  const rejilla = rejillaDe(estado.mapa.config);
  return {
    ...estado,
    memoriaPorFaccion: { [faccionId]: { exploracion: marcarVisto('', rejilla, LEJOS, 100), asentamientos: {} } },
  };
}

describe('caminos: solo los que la Faccion ha PISADO', () => {
  it('un camino entero en tierra que nadie ha explorado no viaja', () => {
    const { sesion, fundador } = partidaConAsentamiento();
    const estado = { ...sesion.getState(), caminos: [caminoPor(LEJOS, { x: 1800, y: 1800 })] };

    expect(proyectarParaJugador(estado, fundador, SIN_GEOMETRIA).caminos).toEqual([]);
  });

  it('un camino que pasa por lo que la propia plaza ve viaja, y entero', () => {
    // Entero a proposito: recortarlo a los tramos explorados daria una polilinea con agujeros que el cliente
    // uniria con rectas falsas. Lo que se acepta a cambio es que un tramo andado revele los dos extremos.
    const { sesion, fundador } = partidaConAsentamiento();
    const camino = caminoPor({ x: 400, y: 400 }, LEJOS);
    const estado = { ...sesion.getState(), caminos: [camino] };

    const proyeccion = proyectarParaJugador(estado, fundador, SIN_GEOMETRIA);
    expect(proyeccion.caminos).toHaveLength(1);
    expect(proyeccion.caminos[0]!.puntos).toEqual(camino.puntos);
  });

  it('vale lo EXPLORADO, no solo lo visible: un camino por un rincon que se vio hace rato sigue viajando', () => {
    const { sesion, faccionId, fundador } = partidaConAsentamiento();
    const estado = conRinconRecordado(sesion.getState(), faccionId);
    const conCamino = { ...estado, caminos: [caminoPor(LEJOS, { x: 1800, y: 1800 })] };

    expect(proyectarParaJugador(conCamino, fundador, SIN_GEOMETRIA).caminos).toHaveLength(1);
  });

  it('un ejercito propio en marcha destapa el camino que cruza', () => {
    const { sesion, faccionId, fundador } = partidaConAsentamiento();
    const estado = sesion.getState();
    const conEjercito: GameSessionState = {
      ...estado,
      ejercitos: [ejercito('e-propio', faccionId, { x: 1200, y: 1200 }, [escuadron('esc-1', 'jugador-test')])],
      caminos: [caminoPor({ x: 1200, y: 1250 }, { x: 1800, y: 1800 })],
    };

    expect(proyectarParaJugador(conEjercito, fundador, SIN_GEOMETRIA).caminos).toHaveLength(1);
  });

  it('sin Faccion no hay caminos: la mascara esta vacia y no se ha pisado nada', () => {
    const { sesion } = partidaConAsentamiento();
    const estado = { ...sesion.getState(), caminos: [caminoPor({ x: 400, y: 400 }, LEJOS)] };

    expect(proyectarParaJugador(estado, 'forastero', SIN_GEOMETRIA).caminos).toEqual([]);
  });
});

describe('campamentosBandidos: solo los que se ven AHORA', () => {
  it('un campamento lejos de todo lo propio no aparece por ningun lado', () => {
    const { sesion, fundador } = partidaConAsentamiento();
    const estado = { ...sesion.getState(), campamentosBandidos: [campamentoEn(LEJOS)] };

    expect(proyectarParaJugador(estado, fundador, SIN_GEOMETRIA).campamentosBandidos).toEqual([]);
  });

  it('uno dentro de lo que vigila la propia plaza viaja entero: del mundo no hay nada que redactar', () => {
    const { sesion, fundador } = partidaConAsentamiento();
    const campamento = campamentoEn({ x: 480, y: 400 }); // a 80 del centro: dentro de radio (60) + margen (60)
    const estado = { ...sesion.getState(), campamentosBandidos: [campamento] };

    expect(proyectarParaJugador(estado, fundador, SIN_GEOMETRIA).campamentosBandidos).toEqual([campamento]);
  });

  it('un ejercito propio en marcha tambien los avista, a su propio radio', () => {
    const { sesion, faccionId, fundador } = partidaConAsentamiento();
    const estado = sesion.getState();
    const conEjercito: GameSessionState = {
      ...estado,
      ejercitos: [ejercito('e-propio', faccionId, { x: 1200, y: 1200 }, [escuadron('esc-1', 'jugador-test')])],
      campamentosBandidos: [campamentoEn({ x: 1200 + VISION.ejercito - 1, y: 1200 }), campamentoEn(LEJOS)],
    };

    const proyeccion = proyectarParaJugador(conEjercito, fundador, SIN_GEOMETRIA);
    expect(proyeccion.campamentosBandidos).toHaveLength(1);
  });

  it('NO tienen memoria: uno en un rincon explorado hace rato no viaja, aunque el camino que pasa por ahi si', () => {
    // La asimetria, en un solo test. El campamento pudo nacer DESPUES de que la Faccion pasara por alli:
    // ensenarlo por "explorado" seria regalar informacion que nadie ha ido a buscar. Un camino no aparece de
    // la nada, asi que ahi "explorado" es exactamente la regla correcta.
    const { sesion, faccionId, fundador } = partidaConAsentamiento();
    const estado = conRinconRecordado(sesion.getState(), faccionId);
    const conAmbos: GameSessionState = {
      ...estado,
      caminos: [caminoPor(LEJOS, { x: 1800, y: 1800 })],
      campamentosBandidos: [campamentoEn(LEJOS)],
    };

    const proyeccion = proyectarParaJugador(conAmbos, fundador, SIN_GEOMETRIA);
    expect(proyeccion.caminos).toHaveLength(1);
    expect(proyeccion.campamentosBandidos).toEqual([]);
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

    const eventos = eventosDominioParaJugador(conEventos, fundador, 0);
    expect(eventos.some((e) => e.codigo === 'ejercito.llega')).toBe(false);
  });
});

// La foto del interior (paso 6b, Doc 1.10.1). Es lo unico del modelo que NO se deriva: una vista sabe
// filtrar el presente, no recordar el pasado.
describe('interiorRecordado: la foto minima de lo que dejaste atras', () => {
  it('al salir queda una foto FECHADA del almacen, la cola y la guarnicion', () => {
    const { sesion, asentamientoId, fundador } = partidaConAsentamiento();
    const trigoAlSalir = sesion.getState().asentamientos[0]!.almacen['trigo']?.cantidad ?? 0;

    sesion.ejecutar(salirAlMundo, { asentamientoId, jugadorId: fundador, escuadronIds: [], carga: {} }, { ...OPC, actor: fundador });

    const ficha = proyectarParaJugador(sesion.getState(), fundador, SIN_GEOMETRIA).asentamientosAvistados[0]!;
    expect(ficha.interiorRecordado, 'de donde has estado, recuerdas').toBeDefined();
    expect(ficha.interiorRecordado!.almacen['trigo']?.cantidad).toBe(trigoAlSalir);
    expect(ficha.interiorRecordado!.vistoEn, 'con fecha, o no seria una foto sino una mentira').toBeDefined();
  });

  it('y la foto NO se refresca sola: la ciudad sigue viviendo y el recuerdo se queda quieto', () => {
    const { sesion, asentamientoId, fundador } = partidaConAsentamiento();
    sesion.ejecutar(salirAlMundo, { asentamientoId, jugadorId: fundador, escuadronIds: [], carga: {} }, { ...OPC, actor: fundador });
    const recordadoAlSalir = proyectarParaJugador(sesion.getState(), fundador, SIN_GEOMETRIA).asentamientosAvistados[0]!.interiorRecordado!;

    for (let i = 0; i < 5; i++) sesion.avanzarTick();

    const ahora = proyectarParaJugador(sesion.getState(), fundador, SIN_GEOMETRIA).asentamientosAvistados[0]!.interiorRecordado!;
    expect(ahora.vistoEn, 'la foto es de cuando salio, no de ahora').toBe(recordadoAlSalir.vistoEn);
    const vivo = sesion.getState().asentamientos[0]!.almacen['trigo']?.cantidad ?? 0;
    expect(ahora.almacen['trigo']?.cantidad, 'y el almacen real se ha movido por debajo').not.toBe(vivo);
  });

  it('de una plaza que NUNCA has pisado no hay foto, aunque la veas', () => {
    // Es de su propia Facción y la ve, pero nunca ha entrado: ficha sí, recuerdo no.
    const base = partidaConAsentamiento();
    const payload = base.sesion.exportar();
    const primera = payload.state.asentamientos[0]!;
    const sesion = GameSession.importar({
      ...payload,
      state: {
        ...payload.state,
        asentamientos: [primera, { ...primera, id: 'nunca-pisada', posicion: { x: 900, y: 900 }, jugadoresFundadoresIds: ['colono'], casasCompradas: [] }],
      },
    });

    const avistada = proyectarParaJugador(sesion.getState(), base.fundador, SIN_GEOMETRIA).asentamientosAvistados.find(
      (a) => a.id === 'nunca-pisada'
    )!;

    expect(avistada.interiorRecordado).toBeUndefined();
  });

  it('la foto es del JUGADOR, no de la Faccion: que otro siga dentro no te la refresca', () => {
    // Es lo que sostiene la mecanica entera. Si fuera de la Faccion, bastaria dejar a uno sentado en casa
    // para que todos vieran el almacen en vivo desde cualquier parte del mapa.
    const { sesion, asentamientoId, fundador, vecino } = partidaConAsentamiento();
    sesion.ejecutar(salirAlMundo, { asentamientoId, jugadorId: fundador, escuadronIds: [], carga: {} }, { ...OPC, actor: fundador });

    // El vecino sigue dentro y ve el interior vivo; el fundador, fuera, solo su foto.
    const dentro = proyectarParaJugador(sesion.getState(), vecino, SIN_GEOMETRIA);
    const fuera = proyectarParaJugador(sesion.getState(), fundador, SIN_GEOMETRIA);

    expect(dentro.asentamientos.map((a) => a.id), 'el que se queda ve la ciudad entera').toEqual([asentamientoId]);
    expect(fuera.asentamientos, 'el que se fue, no').toEqual([]);
    expect(fuera.asentamientosAvistados[0]!.interiorRecordado, 'solo su propia foto').toBeDefined();
  });
});
