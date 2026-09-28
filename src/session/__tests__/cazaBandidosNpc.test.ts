// El NPC caza los bandidos con una columna, como un jugador (Doc 1.9, decisión del usuario 2026-09-28). Antes la
// plaza los atacaba sin moverse y a cualquier distancia: en la Era I medida, la Facción 1 despachaba uno cada 10
// minutos y se quedaba con la experiencia de todos los bandidos del mapa.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, CampamentoBandido, Ejercito } from '../../domain/types';
import { createRng } from '../../worldgen';
import { avanzarNpcGobernanza } from '../npcGobernanza';
import {
  contextoDeTest,
  crearEstadoDeTest,
  crearFacciones,
  crearMapaDeterminista,
  escuadronDePrueba,
  fundarAsentamientoDeTest,
  heroeDePrueba,
  instanteDeTest,
} from '../../engine/__tests__/fixtures';

function escenario(columnaEnElCampamento: boolean) {
  const mapa = crearMapaDeterminista(7);
  const uno = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
  const almacen = { ...uno.asentamiento.almacen, trigo: { cantidad: 5000, capacidad: 100_000 } };
  const plaza: Asentamiento = { ...uno.asentamiento, casasCompradas: ['cazador'], almacen };
  const campamento: CampamentoBandido = {
    id: 'campamento-1',
    posicion: { x: plaza.posicion.x + 60, y: plaza.posicion.y },
    bosqueId: 'bosque-1',
    asentamientoId: plaza.id,
    poder: 30,
  };
  const escuadras = [escuadronDePrueba('c1', 'cazador', 'milicia_lanceros', 25)];
  const columna: Ejercito = {
    id: 'caza-1',
    faccionId: 'faccion-1',
    origenAsentamientoId: plaza.id,
    participantes: [{ heroeId: 'cazador', unidoEn: instanteDeTest(0) }],
    tipo: 'ejercito',
    politicaDeUnion: 'rechazar',
    liderId: 'cazador',
    escuadronIds: ['c1'],
    suministro: { trigo: 100 },
    caravanasAdjuntasIds: [],
    objetivo: { tipo: 'punto', punto: campamento.posicion },
    ruta: [],
    progreso: 1,
    posicionActual: campamento.posicion,
    estado: 'estacionado',
  };
  const cazador = heroeDePrueba(
    'cazador',
    columnaEnElCampamento ? { tipo: 'columna', ejercitoId: columna.id } : { tipo: 'asentamiento', asentamientoId: plaza.id },
    {
      controlador: 'bot',
      escuadrones: columnaEnElCampamento ? escuadras.map((e) => ({ ...e, contenedor: { tipo: 'ejercito' as const, ejercitoId: columna.id } })) : escuadras,
    }
  );
  const estado = crearEstadoDeTest([plaza], uno.facciones, {
    heroes: [cazador],
    campamentosBandidos: [campamento],
    ...(columnaEnElCampamento ? { ejercitos: [columna] } : {}),
  });
  return { r: avanzarNpcGobernanza(estado, mapa, contextoDeTest(1, createRng(5)), { lanzarCampanas: false }), plaza, campamento };
}

describe('el NPC caza sus bandidos con una columna', () => {
  it('manda a un héroe a por el campamento de su plaza', () => {
    const { r, campamento } = escenario(false);
    const caza = r.estado.ejercitos.find((e) => e.liderId === 'cazador')!;
    expect(caza.objetivo).toEqual({ tipo: 'punto', punto: campamento.posicion });
    expect(r.estado.campamentosBandidos, 'todavía no ha llegado').toHaveLength(1);
  });

  it('al llegar lo ataca, y al caer su plaza agenda la reaparición', () => {
    const { r, plaza } = escenario(true);
    expect(r.estado.campamentosBandidos).toEqual([]);
    expect(r.estado.asentamientos.find((a) => a.id === plaza.id)!.bandidosReaparecenEn).toBeDefined();
  });
});
