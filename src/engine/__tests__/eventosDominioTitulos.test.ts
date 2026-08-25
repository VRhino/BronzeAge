// Fase A5 (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md): `titulos.ts` migrado.
import { describe, expect, it } from 'vitest';
import type { Faccion, Titulo } from '../../domain/types';
import { narrarCambiosDeTitulo } from '../titulos';
import type { PayloadTituloCambiaManos, PayloadTituloNace } from '../titulos';
import { crearFaccion } from '../faccion';

describe('eventos de dominio — titulos.ts', () => {
  const facciones: Faccion[] = [crearFaccion('f1', 'Micenas'), crearFaccion('f2', 'Troya')];

  it('un título nuevo (sin previo) produce titulo.nace', () => {
    const actuales: Titulo[] = [{ nombre: 'Facción más grande', poseedorId: 'f1', valorMetrica: 3 }];

    const eventos = narrarCambiosDeTitulo([], actuales, facciones);

    expect(eventos).toHaveLength(1);
    const evento = eventos[0]!;
    if (typeof evento === 'string') throw new Error('esperaba evento migrado');
    expect(evento.codigo).toBe('titulo.nace');
    const p = evento.payload as PayloadTituloNace;
    expect(p.tituloNombre).toBe('Facción más grande');
    expect(p.faccionId).toBe('f1');
    expect(p.valorMetrica).toBe(3);
  });

  it('un título que cambia de poseedor produce titulo.cambia_manos', () => {
    const previos: Titulo[] = [{ nombre: 'Facción más grande', poseedorId: 'f1', valorMetrica: 3 }];
    const actuales: Titulo[] = [{ nombre: 'Facción más grande', poseedorId: 'f2', valorMetrica: 5 }];

    const eventos = narrarCambiosDeTitulo(previos, actuales, facciones);

    expect(eventos).toHaveLength(1);
    const evento = eventos[0]!;
    if (typeof evento === 'string') throw new Error('esperaba evento migrado');
    expect(evento.codigo).toBe('titulo.cambia_manos');
    const p = evento.payload as PayloadTituloCambiaManos;
    expect(p.tituloNombre).toBe('Facción más grande');
    expect(p.previoFaccionId).toBe('f1');
    expect(p.actualFaccionId).toBe('f2');
  });

  it('el mismo poseedor no produce ningún evento', () => {
    const previos: Titulo[] = [{ nombre: 'Facción más grande', poseedorId: 'f1', valorMetrica: 3 }];
    const actuales: Titulo[] = [{ nombre: 'Facción más grande', poseedorId: 'f1', valorMetrica: 4 }];

    const eventos = narrarCambiosDeTitulo(previos, actuales, facciones);

    expect(eventos).toHaveLength(0);
  });
});
