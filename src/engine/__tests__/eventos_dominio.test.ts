// Fase A5 (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md): `avanzarSimulacion` empieza a devolver, junto al
// log en texto de siempre (`eventos`), la misma información en forma estructurada (`eventosDominio`). Esta
// primera versión no migra ningún subsistema — todo evento sale con `codigo: 'legado'` — así que lo único que
// hay que proteger aquí es el contrato: mismo contenido que `eventos`, con `tick` correcto y `asentamientoId`
// presente solo cuando el evento viene del bucle por asentamiento.
import { describe, expect, it } from 'vitest';
import { avanzarSimulacion } from '../simulation';
import { createRng } from '../../worldgen';
import { contextoDeTest, crearEstadoDeTest, crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest } from './fixtures';

describe('eventos de dominio (Fase A5, base)', () => {
  it('eventosDominio tiene el mismo contenido que eventos, con tick y asentamientoId cuando aplica', () => {
    const mapa = crearMapaDeterminista(1);
    const facciones = crearFacciones();
    const { asentamiento, facciones: faccionesTrasFundar } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);

    const estado = crearEstadoDeTest([asentamiento], faccionesTrasFundar);

    const tick = 1;
    const contexto = contextoDeTest(tick, createRng(1));
    const resultado = avanzarSimulacion(estado, mapa, contexto);

    expect(resultado.eventosDominio).toHaveLength(resultado.eventos.length);
    // Todo evento estructurado de esta primera versión es 'legado' y lleva el tick y el momento del avance.
    // `momento` es el campo temporal que viajará a los clientes (ver doc 6 §4, regla (b)); se comprueba que
    // sale del contexto INYECTADO y no de un reloj leído dentro del motor — eso último rompería el
    // determinismo (ver `determinismo.test.ts`).
    for (const evento of resultado.eventosDominio) {
      expect(evento.codigo).toBe('legado');
      expect(evento.tick).toBe(tick);
      expect(evento.momento).toBe(contexto.momento);
    }

    // Los eventos por asentamiento (`eventos` los prefija con "<id>: ") deben coincidir 1:1, en el mismo
    // orden, con los eventos de dominio que llevan ese `asentamientoId` — sin duplicar ni perder ninguno.
    const prefijo = `${asentamiento.id}: `;
    const eventosDelAsentamientoComoTexto = resultado.eventos.filter((e) => e.startsWith(prefijo)).map((e) => e.slice(prefijo.length));
    const eventosDominioDelAsentamiento = resultado.eventosDominio.filter((e) => e.asentamientoId === asentamiento.id).map((e) => e.mensaje);
    expect(eventosDominioDelAsentamiento).toEqual(eventosDelAsentamientoComoTexto);

    // Los eventos globales (mercado, comercio, títulos...) no llevan asentamientoId.
    const eventosGlobales = resultado.eventosDominio.filter((e) => e.asentamientoId === undefined);
    expect(eventosGlobales.length).toBe(resultado.eventosDominio.length - eventosDominioDelAsentamiento.length);
  });
});
