// Contrato compartido "id inexistente -> codigoError sin mutar el estado" (revisión de duplicación
// 2026-08-25). Antes eran ~25 tests casi idénticos repartidos en 6 archivos de comandos, cada uno con el
// mismo cuerpo cambiando solo el comando y el código esperado: en `GameStore` estos comandos hacían
// `.find(...)!`, así que un id que no existiera producía un `TypeError` en vez de un rechazo limpio (ver
// Docs/Arquitectura/4_Plan_Evolucion_Tareas.md, Fase B). Ya migrados, lo que queda por probar es SIEMPRE la
// misma forma — se unifica aquí en una sola tabla en vez de repetir el cuerpo 25 veces: un comando que rompa
// esta garantía falla con su propio nombre en el título del test, no anónimamente dentro de un `for`.
//
// `fundarAsentamiento` NO está aquí a propósito: su caso de "Facción inexistente" ya se prueba junto al resto
// de sus rechazos en `gameSession.test.ts`, como parte de la narrativa propia de ese comando — duplicarlo
// aquí no añadiría cobertura real.
import { describe, expect, it } from 'vitest';
import type { ResultadoComando } from '../comandos/tipos';
import { activarPolitica, asignarCargoLocal, asignarRey, comprarCasa } from '../comandos/cargos';
import { alternarFaccionNpc } from '../comandos/alternarFaccionNpc';
import {
  alternarAutoConstruccion,
  anadirEdificioManualmente,
  mejorarEdificioAhora,
  moverEnCola,
  quitarDeCola,
  renombrarAsentamiento,
} from '../comandos/construccion';
import { anexionar, fusionar, rebelionVasallo, romperRelacion } from '../comandos/diplomacia';
import { atacarCampamentoBandidos, combateCampoAbierto, interceptarCaravana, iniciarAsedio, reclutarTropa } from '../comandos/militar';
import { colocarOrdenMercado, crearCaravana, proponerTrueque } from '../comandos/comercio';
import { desarmarCaravanaFundacion, lanzarCaravanaFundacion } from '../comandos/expansion';
import { OPC, partidaConAsentamiento } from './fixtures';

interface CasoIdInexistente {
  etiqueta: string;
  codigoEsperado: string;
  ejecutar: (ids: ReturnType<typeof partidaConAsentamiento>) => ResultadoComando<unknown>;
}

const CASOS: CasoIdInexistente[] = [
  {
    etiqueta: 'asignarRey: faccionId',
    codigoEsperado: 'faccion.no_existe',
    ejecutar: ({ sesion, fundador }) => sesion.ejecutar(asignarRey, { faccionId: 'no-existe', jugadorId: fundador }, OPC),
  },
  {
    etiqueta: 'asignarCargoLocal: asentamientoId',
    codigoEsperado: 'asentamiento.no_existe',
    ejecutar: ({ sesion, fundador }) =>
      sesion.ejecutar(asignarCargoLocal, { asentamientoId: 'no-existe', cargo: 'gobernador', jugadorId: fundador }, OPC),
  },
  {
    // Caso real: comprarCasa resuelve la Facción a partir del asentamiento, así que un asentamiento
    // inexistente se traduce como Facción inválida, no como "asentamiento.no_existe" — no es un error de
    // la tabla, es el código que de verdad devuelve el comando.
    etiqueta: 'comprarCasa: asentamientoId',
    codigoEsperado: 'faccion.invalida',
    ejecutar: ({ sesion }) => sesion.ejecutar(comprarCasa, { asentamientoId: 'no-existe', jugadorId: 'nuevo' }, OPC),
  },
  {
    etiqueta: 'activarPolitica: asentamientoId',
    codigoEsperado: 'asentamiento.no_existe',
    ejecutar: ({ sesion }) =>
      sesion.ejecutar(activarPolitica, { asentamientoId: 'no-existe', cargo: 'gobernador', politicaId: 'lineas_produccion' }, OPC),
  },
  {
    etiqueta: 'alternarFaccionNpc: faccionId',
    codigoEsperado: 'faccion.no_existe',
    ejecutar: ({ sesion }) => sesion.ejecutar(alternarFaccionNpc, { faccionId: 'no-existe', activo: true }, OPC),
  },
  {
    etiqueta: 'anadirEdificioManualmente: asentamientoId',
    codigoEsperado: 'asentamiento.no_existe',
    ejecutar: ({ sesion }) =>
      sesion.ejecutar(anadirEdificioManualmente, { asentamientoId: 'no-existe', cargo: 'gobernador', tipo: 'vivienda' }, OPC),
  },
  {
    etiqueta: 'quitarDeCola: asentamientoId',
    codigoEsperado: 'asentamiento.no_existe',
    ejecutar: ({ sesion }) => sesion.ejecutar(quitarDeCola, { asentamientoId: 'no-existe', cargo: 'gobernador', edificioId: 'x' }, OPC),
  },
  {
    etiqueta: 'moverEnCola: asentamientoId',
    codigoEsperado: 'asentamiento.no_existe',
    ejecutar: ({ sesion }) =>
      sesion.ejecutar(moverEnCola, { asentamientoId: 'no-existe', cargo: 'gobernador', edificioId: 'x', direccion: 'arriba' }, OPC),
  },
  {
    etiqueta: 'mejorarEdificioAhora: asentamientoId',
    codigoEsperado: 'asentamiento.no_existe',
    ejecutar: ({ sesion }) =>
      sesion.ejecutar(mejorarEdificioAhora, { asentamientoId: 'no-existe', cargo: 'gobernador', edificioId: 'x' }, OPC),
  },
  {
    etiqueta: 'alternarAutoConstruccion: asentamientoId',
    codigoEsperado: 'asentamiento.no_existe',
    ejecutar: ({ sesion }) => sesion.ejecutar(alternarAutoConstruccion, { asentamientoId: 'no-existe', pausada: true }, OPC),
  },
  {
    etiqueta: 'renombrarAsentamiento: asentamientoId',
    codigoEsperado: 'asentamiento.no_existe',
    ejecutar: ({ sesion }) => sesion.ejecutar(renombrarAsentamiento, { asentamientoId: 'no-existe', nombre: 'X' }, OPC),
  },
  {
    etiqueta: 'romperRelacion: relacionId',
    codigoEsperado: 'diplomacia.invalida',
    ejecutar: ({ sesion, faccionId }) => sesion.ejecutar(romperRelacion, { relacionId: 'no-existe', iniciadorFaccionId: faccionId }, OPC),
  },
  {
    etiqueta: 'rebelionVasallo: relacionId',
    codigoEsperado: 'diplomacia.invalida',
    ejecutar: ({ sesion }) => sesion.ejecutar(rebelionVasallo, { relacionId: 'no-existe' }, OPC),
  },
  {
    etiqueta: 'anexionar: faccionBId',
    codigoEsperado: 'fusion.invalida',
    ejecutar: ({ sesion, faccionId }) => sesion.ejecutar(anexionar, { faccionAId: faccionId, faccionBId: 'no-existe' }, OPC),
  },
  {
    etiqueta: 'fusionar: faccionBId',
    codigoEsperado: 'fusion.invalida',
    ejecutar: ({ sesion, faccionId }) =>
      sesion.ejecutar(fusionar, { faccionAId: faccionId, faccionBId: 'no-existe', nuevoNombre: 'X', nuevoReyId: 'y' }, OPC),
  },
  {
    etiqueta: 'reclutarTropa: asentamientoId',
    codigoEsperado: 'asentamiento.no_existe',
    ejecutar: ({ sesion, fundador }) =>
      sesion.ejecutar(reclutarTropa, { asentamientoId: 'no-existe', jugadorId: fundador, tropaId: 'milicia_lanceros', origen: 'pesants' }, OPC),
  },
  {
    // No es un id de ENTIDAD de la partida sino de CATÁLOGO (la tropa no existe en `TROPAS`), pero la forma
    // es la misma: id que no resuelve a nada -> codigoError, sin excepción sin capturar.
    etiqueta: 'reclutarTropa: tropaId (catálogo)',
    codigoEsperado: 'tropas.reclutamiento_invalido',
    ejecutar: ({ sesion, asentamientoId, fundador }) =>
      sesion.ejecutar(reclutarTropa, { asentamientoId, jugadorId: fundador, tropaId: 'no-existe', origen: 'pesants' }, OPC),
  },
  {
    etiqueta: 'iniciarAsedio: defensorId',
    codigoEsperado: 'asentamiento.no_existe',
    ejecutar: ({ sesion, asentamientoId }) =>
      sesion.ejecutar(iniciarAsedio, { atacanteId: asentamientoId, defensorId: 'no-existe', escuadronIds: [] }, OPC),
  },
  {
    etiqueta: 'combateCampoAbierto: asentamientoBId',
    codigoEsperado: 'asentamiento.no_existe',
    ejecutar: ({ sesion, asentamientoId }) =>
      sesion.ejecutar(
        combateCampoAbierto,
        { asentamientoAId: asentamientoId, escuadronIdsA: [], asentamientoBId: 'no-existe', escuadronIdsB: [] },
        OPC
      ),
  },
  {
    etiqueta: 'interceptarCaravana: caravanaId',
    codigoEsperado: 'caravana.no_existe',
    ejecutar: ({ sesion, asentamientoId }) =>
      sesion.ejecutar(interceptarCaravana, { atacanteId: asentamientoId, escuadronIds: [], caravanaId: 'no-existe' }, OPC),
  },
  {
    etiqueta: 'atacarCampamentoBandidos: campamentoId',
    codigoEsperado: 'campamento.no_existe',
    ejecutar: ({ sesion, asentamientoId }) =>
      sesion.ejecutar(atacarCampamentoBandidos, { atacanteId: asentamientoId, escuadronIds: [], campamentoId: 'no-existe' }, OPC),
  },
  {
    etiqueta: 'crearCaravana: asentamientoId',
    codigoEsperado: 'asentamiento.no_existe',
    ejecutar: ({ sesion }) => sesion.ejecutar(crearCaravana, { asentamientoId: 'no-existe' }, OPC),
  },
  {
    etiqueta: 'proponerTrueque: asentamientoBId',
    codigoEsperado: 'comercio.trueque_invalido',
    ejecutar: ({ sesion, asentamientoId }) =>
      sesion.ejecutar(
        proponerTrueque,
        { asentamientoAId: asentamientoId, recursoA: 'madera', cantidadA: 5, asentamientoBId: 'no-existe', recursoB: 'piedra', cantidadB: 5 },
        OPC
      ),
  },
  {
    etiqueta: 'colocarOrdenMercado: asentamientoId',
    codigoEsperado: 'mercado.orden_invalida',
    ejecutar: ({ sesion }) =>
      sesion.ejecutar(colocarOrdenMercado, { asentamientoId: 'no-existe', tipo: 'venta', recurso: 'madera', cantidad: 10 }, OPC),
  },
  {
    etiqueta: 'lanzarCaravanaFundacion: origenAsentamientoId',
    codigoEsperado: 'asentamiento.no_existe',
    ejecutar: ({ sesion }) =>
      sesion.ejecutar(lanzarCaravanaFundacion, { origenAsentamientoId: 'no-existe', destino: { x: 700, y: 700 }, numJugadores: 1 }, OPC),
  },
  {
    etiqueta: 'desarmarCaravanaFundacion: caravanaId',
    codigoEsperado: 'caravana.no_existe',
    ejecutar: ({ sesion }) => sesion.ejecutar(desarmarCaravanaFundacion, { caravanaId: 'no-existe' }, OPC),
  },
];

describe.each(CASOS)('$etiqueta inexistente', ({ codigoEsperado, ejecutar }) => {
  it('devuelve codigoError sin mutar el estado', () => {
    const ids = partidaConAsentamiento();
    const antes = ids.sesion.getState();

    const resultado = ejecutar(ids);

    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe(codigoEsperado);
    expect(ids.sesion.getState()).toBe(antes);
  });
});
