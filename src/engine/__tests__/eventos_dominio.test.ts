// Fase A5 (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md): `avanzarSimulacion` devuelve el log del tick en
// forma estructurada (`eventosDominio`) — único contrato de salida desde que se retiró el array de texto
// plano paralelo (`ResultadoTick.eventos`, redundante con `eventosDominio[].mensaje`). La migración
// subsistema-por-subsistema (marcador 13/13) reemplazó `codigo: 'legado'` por códigos estables en los 13
// subsistemas — este test protege el CONTRATO que no cambia con esa migración: `tick`/`momento` correctos,
// `asentamientoId` presente solo cuando el evento viene del bucle por asentamiento, y todo `codigo` no vacío
// (legado o migrado). No afirma qué `codigo` concreto lleva cada evento — eso lo prueba el test del
// subsistema que lo emite (ver p. ej. `eventosDominioConstruccion.test.ts`).
import { describe, expect, it } from 'vitest';
import { avanzarSimulacion } from '../simulation';
import { createRng } from '../../worldgen';
import { contextoDeTest, crearEstadoDeTest, crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest } from './fixtures';

describe('eventos de dominio (Fase A5)', () => {
  it('eventosDominio trae tick/momento correctos y asentamientoId solo donde aplica', () => {
    const mapa = crearMapaDeterminista(1);
    const facciones = crearFacciones();
    const { asentamiento, facciones: faccionesTrasFundar } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);

    const estado = crearEstadoDeTest([asentamiento], faccionesTrasFundar);

    const tick = 1;
    const contexto = contextoDeTest(tick, createRng(1));
    const resultado = avanzarSimulacion(estado, mapa, contexto);

    // Confirma que hubo dinámica real: si no hay eventos, las aserciones de abajo pasarían vacuamente.
    expect(resultado.eventosDominio.length).toBeGreaterThan(0);

    // `momento` es el campo temporal que viaja a los clientes (ver doc 6 §4, regla (b)); se comprueba que sale
    // del contexto INYECTADO y no de un reloj leído dentro del motor — eso último rompería el determinismo
    // (ver `determinismo.test.ts`). `codigo` ya no es siempre 'legado' (subsistemas migrados llevan el suyo
    // propio), pero SIEMPRE tiene que venir informado — un `codigo` vacío rompería el filtrado de Fase C/E que
    // esta migración existe para habilitar.
    for (const evento of resultado.eventosDominio) {
      expect(evento.codigo).not.toBe('');
      expect(evento.tick).toBe(tick);
      expect(evento.momento).toBe(contexto.momento);
    }

    // Los eventos del bucle por asentamiento llevan `asentamientoId`; los globales (mercado, comercio,
    // títulos...) no.
    const eventosDelAsentamiento = resultado.eventosDominio.filter((e) => e.asentamientoId === asentamiento.id);
    const eventosGlobales = resultado.eventosDominio.filter((e) => e.asentamientoId === undefined);
    expect(eventosDelAsentamiento.length + eventosGlobales.length).toBe(resultado.eventosDominio.length);
    expect(eventosDelAsentamiento.length).toBeGreaterThan(0);
  });
});
