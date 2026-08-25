// Ciclo exportar -> importar de una partida.
//
// El archivo NO guarda el mapa entero: guarda la seed, los nodos y los bosques, y al importar se REGENERA
// el mundo desde la seed para recuperar lo único que no viaja en el archivo — el campo de fertilidad, que
// son parámetros de ruido, no una lista de valores. Toda la corrección de este mecanismo descansa en que
// `generarMapa` sea determinista por seed; por eso los parámetros de generación están congelados fuera del
// panel de balance (ver `src/worldgen/config.ts`) y por eso el archivo lleva `worldgenVersion`.
//
// Lo que se verifica aquí es justo esa cadena: que una partida importada es indistinguible de la original,
// incluida la fertilidad regenerada y el agotamiento de yacimientos ya ocurrido.

import { describe, expect, it } from 'vitest';
import { GameStore, type SimulacionExportada } from '../gameStore';
import { WORLDGEN_VERSION } from '../../worldgen';

/** Partida arrancada, con un asentamiento fundado en un sitio viable y unos cuantos ticks corridos. */
function partidaEnMarcha(ticks = 30): GameStore {
  const store = new GameStore();
  store.crearFaccion('Faccion Test');
  const faccionId = store.getState().facciones[0]!.id;

  // Se busca un emplazamiento recomendable barriendo una grilla, igual que las fixtures del motor: así el
  // test no depende de que la seed por defecto tenga un bosque cerca de un punto elegido a mano.
  const { ancho, alto } = store.getMapa().limites;
  let fundado = false;
  for (let x = 40; x < ancho && !fundado; x += 40) {
    for (let y = 40; y < alto && !fundado; y += 40) {
      if (store.viabilidadFundacion({ x, y }).recomendable) {
        store.fundarAsentamiento(faccionId, { x, y }, 1);
        fundado = true;
      }
    }
  }
  expect(fundado).toBe(true);

  for (let i = 0; i < ticks; i++) store.avanzarTick();
  return store;
}

function fertilidadMuestreada(store: GameStore): string[] {
  const mapa = store.getMapa();
  const { ancho, alto } = mapa.limites;
  const muestras: string[] = [];
  for (let i = 0; i < 50; i++) {
    muestras.push(mapa.fertilidadEn({ x: ((i * 97) % ancho) + 0.5, y: ((i * 61) % alto) + 0.5 }).toFixed(10));
  }
  return muestras;
}

describe('exportar / importar una simulación', () => {
  it('la partida importada es idéntica a la original, con la fertilidad regenerada desde la seed', () => {
    const original = partidaEnMarcha();
    const json = original.exportarSimulacion();

    const importada = new GameStore();
    importada.importarSimulacion(json);

    const antes = original.getState();
    const despues = importada.getState();

    expect(despues.tick).toBe(antes.tick);
    expect(despues.asentamientos).toEqual(antes.asentamientos);
    expect(despues.facciones).toEqual(antes.facciones);
    expect(despues.caravanas).toEqual(antes.caravanas);
    expect(despues.mapa.config).toEqual(antes.mapa.config);
    expect(despues.mapa.bosques).toEqual(antes.mapa.bosques);
    // El mundo se regenera desde la seed y sale idéntico...
    expect(despues.mapa.nodos).toEqual(antes.mapa.nodos);
    // ...y el agotamiento acumulado, que no se puede regenerar, viaja en el archivo.
    expect(despues.estadoMapa.extraido).toEqual(antes.estadoMapa.extraido);
    // La fertilidad SÍ se regenera. Es el punto frágil del formato y el motivo de `worldgenVersion`.
    expect(fertilidadMuestreada(importada)).toEqual(fertilidadMuestreada(original));

    expect(despues.log[0]?.mensaje).toContain('Simulación importada');
  });

  it('el agotamiento de yacimientos sobrevive al viaje', () => {
    // El agotamiento se fuerza EN EL ARCHIVO, no llamando a `mapa.extraer()` a mano: desde que `Mapa` copia
    // el estado que recibe en vez de aliasarlo (Fase B3, ver `world/mapa.ts`), la fachada que devuelve
    // `getMapa()` ya no es un asa de escritura sobre la partida — escribir en ella y esperar que la partida
    // lo recuerde sería probar justo lo que ese cambio elimina. El formato v2 representa el agotamiento como
    // `recursos[].cantidad`, así que se edita ahí, que es como realmente le llegaría a `importarSimulacion`
    // un archivo con un yacimiento ya vaciado.
    const original = partidaEnMarcha();
    const nodo = original.getState().mapa.nodos.find((n) => n.tipo === 'piedra')!;
    const payload = JSON.parse(original.exportarSimulacion()) as SimulacionExportada;
    const nodoExportado = payload.world.recursos.find((n) => n.id === nodo.id)!;
    nodoExportado.cantidad = 0;

    const importada = new GameStore();
    importada.importarSimulacion(JSON.stringify(payload));

    expect(importada.getMapa().nodoProductivo(nodo.id)).toBe(false);
    expect(importada.getMapa().stock(nodo.id)).toBe(0);
  });

  it('rechaza un archivo generado con otra versión del generador, en vez de cargar otro mundo en silencio', () => {
    const original = partidaEnMarcha(5);
    const payload = JSON.parse(original.exportarSimulacion()) as SimulacionExportada;
    expect(payload.worldgenVersion).toBe(WORLDGEN_VERSION);
    payload.worldgenVersion = WORLDGEN_VERSION + 1;

    const destino = new GameStore();
    const tickAntes = destino.getState().tick;
    destino.importarSimulacion(JSON.stringify(payload));

    expect(destino.getState().tick).toBe(tickAntes);
    expect(destino.getState().asentamientos).toHaveLength(0);
    expect(destino.getState().log[0]?.mensaje).toContain('Importación rechazada');
  });

  it('rechaza archivos antiguos sin `worldgenVersion` (se asumen de la versión 1, y la actual ya no lo es)', () => {
    // Antes de Fase 0.1 (WORLDGEN_VERSION 1) un archivo sin el campo se aceptaba porque "ausente = versión
    // 1" coincidía con la versión vigente. Ahora que WORLDGEN_VERSION es 2, ese mismo archivo sigue
    // asumiéndose versión 1 — pero ya no coincide, así que debe rechazarse como cualquier otra versión
    // distinta (mismo camino que el test de arriba), no cargarse en silencio con un mundo distinto.
    const original = partidaEnMarcha(5);
    const payload = JSON.parse(original.exportarSimulacion()) as Partial<SimulacionExportada>;
    delete payload.worldgenVersion;

    const destino = new GameStore();
    const tickAntes = destino.getState().tick;
    destino.importarSimulacion(JSON.stringify(payload));

    expect(destino.getState().tick).toBe(tickAntes);
    expect(destino.getState().asentamientos).toHaveLength(0);
    expect(destino.getState().log[0]?.mensaje).toContain('Importación rechazada');
  });

  it('un archivo corrupto se rechaza sin romper la partida en curso', () => {
    const store = partidaEnMarcha(5);
    const asentamientosAntes = store.getState().asentamientos;

    store.importarSimulacion('{"esto": "no es una simulación"}');

    expect(store.getState().asentamientos).toEqual(asentamientosAntes);
    expect(store.getState().log[0]?.mensaje).toContain('Importación rechazada');
  });
});
