// El NPC adopta la tecnología que le aparece como un jugador: con su Rey en la capital, pagando la capital (Doc 6.5).
import { describe, expect, it } from 'vitest';
import { createRng } from '../../worldgen';
import { avanzarNpcGobernanza } from '../npcGobernanza';
import { estadoTecnologiaInicial, tecnologiasDe } from '../../engine/tecnologia';
import { instante } from '../../domain/tiempo';
import { contextoDeTest, crearEstadoDeTest, crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, heroeDePrueba } from '../../engine/__tests__/fixtures';

function escenario(reyEnCasa: boolean) {
  const mapa = crearMapaDeterminista(7);
  const uno = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
  const rey = uno.asentamiento.heroesFundadoresIds[0]!;
  const plaza = {
    ...uno.asentamiento,
    almacen: { ...uno.asentamiento.almacen, oro: { cantidad: 1000, capacidad: 2000 }, madera: { cantidad: 1000, capacidad: 2000 } },
  };
  const facciones = uno.facciones.map((f) => (f.id === 'faccion-1' ? { ...f, reyId: rey } : f));
  const ubicacion = reyEnCasa ? { tipo: 'asentamiento' as const, asentamientoId: plaza.id } : { tipo: 'desconectado' as const, punto: { x: 5, y: 5 } };
  const heroes = [heroeDePrueba(rey, ubicacion, { controlador: 'bot' })];
  const base = estadoTecnologiaInicial(instante(0));
  const tecnologia = {
    ...base,
    porFaccion: { 'faccion-1': { aparecidas: ['leva_comunal', 'hostigamiento_tribal', 'metalurgia_cobre'], adoptadas: ['leva_comunal', 'hostigamiento_tribal'] } },
  } as typeof base;
  const r = avanzarNpcGobernanza(crearEstadoDeTest([plaza], facciones, { heroes, tecnologia }), mapa, contextoDeTest(1, createRng(5)), {
    lanzarCampanas: false,
  });
  return { adoptadas: tecnologiasDe(r.estado.tecnologia, 'faccion-1').adoptadas, oro: r.estado.asentamientos[0]!.almacen['oro']!.cantidad };
}

describe('tecnología del NPC', () => {
  it('con el Rey en la capital adopta lo que le apareció y lo paga la capital', () => {
    const r = escenario(true);
    expect(r.adoptadas).toContain('metalurgia_cobre');
    expect(r.oro).toBeLessThan(1000);
  });

  it('con el Rey fuera de la capital no adopta', () => {
    expect(escenario(false).adoptadas).not.toContain('metalurgia_cobre');
  });
});
