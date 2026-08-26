// `proyectarParaJugador` sobre estado de partida GENUINO (misma fixture que `comandos/autorizacion.ts`), no
// mocks — así estas pruebas fallan de verdad si cambia la forma de `GameSessionState` o el significado de
// ciudadanía, no solo si cambia la proyección.
import { describe, expect, it } from 'vitest';
import { partidaConAsentamiento, OPC } from '../../__tests__/fixtures';
import { crearFaccion } from '../../comandos/crearFaccion';
import { fundarAsentamiento } from '../../comandos/fundarAsentamiento';
import { proyectarParaJugador } from '../jugador';

describe('faccionId se deriva de la ciudadanía, no de un campo guardado', () => {
  it('el fundador ve su propia Facción', () => {
    const { sesion, faccionId, fundador } = partidaConAsentamiento();
    const proyeccion = proyectarParaJugador(sesion.getState(), fundador);
    expect(proyeccion.faccionId).toBe(faccionId);
  });

  it('un jugador sin ciudadanía en ninguna Facción tiene faccionId null', () => {
    const { sesion } = partidaConAsentamiento();
    const proyeccion = proyectarParaJugador(sesion.getState(), 'forastero');
    expect(proyeccion.faccionId).toBeNull();
  });
});

describe('asentamientos: solo los de la Facción propia', () => {
  it('el fundador ve su asentamiento', () => {
    const { sesion, asentamientoId, fundador } = partidaConAsentamiento();
    const proyeccion = proyectarParaJugador(sesion.getState(), fundador);
    expect(proyeccion.asentamientos.map((a) => a.id)).toEqual([asentamientoId]);
  });

  it('un forastero sin Facción no ve ningún asentamiento, aunque exista', () => {
    const { sesion } = partidaConAsentamiento();
    const proyeccion = proyectarParaJugador(sesion.getState(), 'forastero');
    expect(proyeccion.asentamientos).toEqual([]);
  });

  it('el asentamiento de una Facción rival NO aparece — es justo lo que Slice 1 no resuelve todavía', () => {
    const base = partidaConAsentamiento();
    const opcRival = { ...OPC, actor: 'rival' };
    const rf = base.sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, opcRival);
    base.sesion.ejecutar(fundarAsentamiento, { faccionId: rf.datos!.faccionId, posicion: { x: 900, y: 900 } }, opcRival);

    const proyeccion = proyectarParaJugador(base.sesion.getState(), base.fundador);
    expect(proyeccion.asentamientos).toHaveLength(1); // solo el propio, no los 2 que existen en la partida
    expect(proyeccion.asentamientos[0]!.faccionId).toBe(base.faccionId);
  });
});

describe('facciones: metadatos públicos de TODAS, sin filtrar', () => {
  it('incluye la Facción rival aunque sus asentamientos no aparezcan', () => {
    const base = partidaConAsentamiento();
    const opcRival = { ...OPC, actor: 'rival' };
    const rf = base.sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, opcRival);

    const proyeccion = proyectarParaJugador(base.sesion.getState(), base.fundador);
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
        { id: 'o1', asentamientoId, tipo: 'venta' as const, recurso: 'trigo', cantidad: 10, cantidadCumplida: 0, precioUnitario: 1, creadoEnTick: 0, estado: 'activa' as const },
        { id: 'o2', asentamientoId: 'asentamiento-ajeno', tipo: 'venta' as const, recurso: 'trigo', cantidad: 10, cantidadCumplida: 0, precioUnitario: 1, creadoEnTick: 0, estado: 'activa' as const },
      ],
    };

    const proyeccion = proyectarParaJugador(estadoConOrdenes, fundador);
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

    const proyeccion = proyectarParaJugador(estadoConCaravana, fundador);
    expect(proyeccion.caravanas.map((c) => c.id)).toEqual(['c1']);
  });
});

describe('eventosDominio: sin asentamientoId (globales) o con uno propio', () => {
  it('el evento de fundación (con asentamientoId propio) pasa el filtro', () => {
    const { sesion, asentamientoId, fundador } = partidaConAsentamiento();
    const proyeccion = proyectarParaJugador(sesion.getState(), fundador);
    expect(proyeccion.eventosDominio.some((e) => e.asentamientoId === asentamientoId)).toBe(true);
  });

  it('un evento de asentamiento AJENO no aparece en la proyección de un jugador sin ese asentamiento', () => {
    const base = partidaConAsentamiento();
    const opcRival = { ...OPC, actor: 'rival' };
    const rf = base.sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, opcRival);
    const ra = base.sesion.ejecutar(fundarAsentamiento, { faccionId: rf.datos!.faccionId, posicion: { x: 900, y: 900 } }, opcRival);

    const proyeccion = proyectarParaJugador(base.sesion.getState(), base.fundador);
    expect(proyeccion.eventosDominio.some((e) => e.asentamientoId === ra.datos!.asentamientoId)).toBe(false);
  });
});

describe('historial: el propio, nunca el de otro jugador', () => {
  it('devuelve el historial de ESE jugador', () => {
    const { sesion, fundador } = partidaConAsentamiento();
    const proyeccion = proyectarParaJugador(sesion.getState(), fundador);
    expect(proyeccion.historial.length).toBeGreaterThan(0);
    expect(proyeccion.historial).toEqual(sesion.getState().historialJugadores[fundador]);
  });

  it('un jugador sin historial recibe un array vacío, no undefined', () => {
    const { sesion } = partidaConAsentamiento();
    const proyeccion = proyectarParaJugador(sesion.getState(), 'nadie-hizo-nada');
    expect(proyeccion.historial).toEqual([]);
  });
});

describe('mapa, relaciones, titulos, caminos y campamentosBandidos: públicos, sin filtrar', () => {
  it('viajan tal cual desde el estado', () => {
    const { sesion, fundador } = partidaConAsentamiento();
    const estado = sesion.getState();
    const proyeccion = proyectarParaJugador(estado, fundador);

    expect(proyeccion.mapa).toBe(estado.mapa);
    expect(proyeccion.relaciones).toBe(estado.relaciones);
    expect(proyeccion.titulos).toBe(estado.titulos);
    expect(proyeccion.caminos).toBe(estado.caminos);
    expect(proyeccion.campamentosBandidos).toBe(estado.campamentosBandidos);
  });
});
