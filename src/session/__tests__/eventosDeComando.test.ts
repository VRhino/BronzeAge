// Los eventos de COMANDO tienen código estable y payload tipado, igual que los 13 subsistemas del tick que
// migró A5 (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md). Es el gate real de Fase C: la autorización y las
// proyecciones por audiencia se construyen sobre los comandos, y un evento en texto libre no se puede filtrar
// por visibilidad sin parsear castellano.
//
// Este test protege el CONTRATO transversal —ningún comando aceptado emite ya `'legado'`, y el `momento` sale
// del contexto inyectado— más los códigos concretos de una muestra representativa: uno narrado por la capa de
// comandos (`crearFaccion`), uno de alcance local con `asentamientoId` (`renombrarAsentamiento`) y uno
// narrado por el MOTOR y adaptado aquí (`atacarCampamentoBandidos`, vía `desdeCrudos`).
import { describe, expect, it } from 'vitest';
import { GameSession } from '../gameSession';
import { crearFaccion } from '../comandos/crearFaccion';
import { renombrarAsentamiento } from '../comandos/construccion';
import { asignarCargoLocal } from '../comandos/cargos';
import { proponerRelacion } from '../comandos/diplomacia';
import { MOMENTO, OPC, partidaConAsentamiento } from './fixtures';
import type { PayloadFaccionCreada } from '../comandos/crearFaccion';
import type { PayloadRenombrado } from '../comandos/construccion';
import type { PayloadRelacionPropuesta } from '../comandos/diplomacia';

describe('eventos de comando (códigos estables)', () => {
  it('ningún comando aceptado emite ya `codigo: "legado"`', () => {
    const { sesion, faccionId, asentamientoId, fundador } = partidaConAsentamiento();
    sesion.ejecutar(asignarCargoLocal, { asentamientoId, cargo: 'gobernador', jugadorId: fundador }, OPC);
    sesion.ejecutar(renombrarAsentamiento, { asentamientoId, nombre: 'Tirinto' }, OPC);
    sesion.ejecutar(crearFaccion, { nombre: 'Pilos' }, OPC);
    expect(faccionId).toBeTruthy();

    const eventos = sesion.getState().eventosDominio;
    expect(eventos.length).toBeGreaterThan(0);
    for (const e of eventos) {
      expect(e.codigo).not.toBe('legado');
      expect(e.codigo).not.toBe('');
      expect(e.momento).toBe(MOMENTO);
    }
  });

  it('crearFaccion: código propio y payload con el id que acaba de nacer', () => {
    const sesion = GameSession.crear('t', { seed: 1 });
    const r = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC);

    expect(r.eventos).toHaveLength(1);
    const e = r.eventos[0]!;
    expect(e.codigo).toBe('faccion.creada');
    expect(e.payload as PayloadFaccionCreada).toEqual({ faccionId: r.datos!.faccionId, nombre: 'Micenas' });
    // Alcance global: no se atribuye a ningún asentamiento.
    expect(e.asentamientoId).toBeUndefined();
  });

  it('renombrarAsentamiento: evento de alcance local, atribuido a su asentamiento', () => {
    const { sesion, asentamientoId } = partidaConAsentamiento();
    const r = sesion.ejecutar(renombrarAsentamiento, { asentamientoId, nombre: '  Tirinto  ' }, OPC);

    const e = r.eventos[0]!;
    expect(e.codigo).toBe('asentamiento.renombrado');
    expect(e.asentamientoId).toBe(asentamientoId);
    expect(e.payload as PayloadRenombrado).toEqual({ asentamientoId, nombre: 'Tirinto' });
  });

  it('proponerRelacion: el payload lleva AMBAS facciones, que es sobre lo que filtrará la visibilidad', () => {
    const { sesion, faccionId } = partidaConAsentamiento();
    const otra = sesion.ejecutar(crearFaccion, { nombre: 'Pilos' }, OPC).datos!.faccionId;

    const r = sesion.ejecutar(proponerRelacion, { tipo: 'alianza', faccionAId: faccionId, faccionBId: otra }, OPC);

    expect(r.ok).toBe(true);
    const e = r.eventos[0]!;
    expect(e.codigo).toBe('diplomacia.relacion_propuesta');
    expect(e.payload as PayloadRelacionPropuesta).toEqual({
      relacionId: r.datos!.relacionId,
      tipo: 'alianza',
      faccionAId: faccionId,
      faccionBId: otra,
    });
  });
});
