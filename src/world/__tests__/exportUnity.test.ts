// Export a Unity Terrain: el heightmap debe salir del tamaño/formato que Unity exige, el tallado de ríos
// debe rebajar el relieve cerca del cauce respecto al campo de elevación sin tallar (si no, el río queda
// "flotando" sobre terreno que no lo refleja — ver cabecera de `exportUnity.ts`), y el splatmap de biomas
// debe coincidir con `evaluarBioma` — `generarPesosSplatmap` duplica su lógica por rendimiento (ver
// comentario en `exportUnity.ts`), así que este archivo es lo único que detecta si ambas divergen.
import type { BiomaTipo } from '../../domain/types';
import { describe, expect, it } from 'vitest';
import { evaluarBioma, evaluarElevacion, generarMapa } from '../../worldgen';
import { BIOMA_ORDEN, exportarParaUnityTerrain, UNITY_EXPORT_DEFAULT } from '../exportUnity';

// Mundo pequeño + resolución baja (pero válida: 2^5+1) para que el test corra rápido sin perder cobertura
// del algoritmo — el tamaño real de export (4097, mundo 2000x2000) se prueba aparte, solo en tamaño de salida.
const RESOLUCION_TEST = 33;

function generarMundoConRios(seed: number) {
  let generado = generarMapa({ ancho: 800, alto: 800, seed });
  // Unas pocas seeds no producen ríos que lleguen a buen puerto en un mundo tan pequeño; se prueba con varias
  // hasta encontrar una con al menos un río de 2+ puntos, para poder verificar el tallado.
  let intento = seed;
  while (!generado.rios.some((r) => r.puntos.length >= 2) && intento < seed + 20) {
    intento++;
    generado = generarMapa({ ancho: 800, alto: 800, seed: intento });
  }
  return generado;
}

describe('exportarParaUnityTerrain', () => {
  it('rechaza una resolución que no sea 2^n + 1', () => {
    const generado = generarMapa({ ancho: 800, alto: 800, seed: 1 });
    expect(() => exportarParaUnityTerrain(generado, [], { resolucion: 1000 })).toThrow();
  });

  it('acepta las resoluciones válidas por defecto', () => {
    expect(UNITY_EXPORT_DEFAULT.resolucion).toBe(4097);
    expect(Number.isInteger(Math.log2(UNITY_EXPORT_DEFAULT.resolucion - 1))).toBe(true);
  });

  it('el heightmap RAW mide resolución² × 2 bytes (16-bit) y cada muestra cae en [0, 65535]', () => {
    const generado = generarMapa({ ancho: 800, alto: 800, seed: 1 });
    const { heightmapRaw } = exportarParaUnityTerrain(generado, [], { resolucion: RESOLUCION_TEST });

    expect(heightmapRaw.length).toBe(RESOLUCION_TEST * RESOLUCION_TEST * 2);

    const vista = new DataView(heightmapRaw.buffer);
    for (let i = 0; i < RESOLUCION_TEST * RESOLUCION_TEST; i++) {
      const valor = vista.getUint16(i * 2, true);
      expect(valor).toBeGreaterThanOrEqual(0);
      expect(valor).toBeLessThanOrEqual(65535);
    }
  });

  it('el tallado de ríos rebaja la altura exportada por debajo del campo de elevación sin tallar', () => {
    const generado = generarMundoConRios(1);
    const rio = generado.rios.find((r) => r.puntos.length >= 2);
    expect(rio).toBeDefined();

    const { heightmapRaw, metadata } = exportarParaUnityTerrain(generado, [], { resolucion: RESOLUCION_TEST });
    const vista = new DataView(heightmapRaw.buffer);
    const { ancho, alto } = generado.config;

    // Punto intermedio del río: se busca la celda de la rejilla más cercana y se compara la altura exportada
    // (tallada) contra `evaluarElevacion` en ese mismo punto (sin tallar) — debe haber bajado.
    const puntoRio = rio!.puntos[Math.floor(rio!.puntos.length / 2)]!;
    const col = Math.round((puntoRio.x / ancho) * (RESOLUCION_TEST - 1));
    const fila = Math.round((puntoRio.y / alto) * (RESOLUCION_TEST - 1));
    const idx = fila * RESOLUCION_TEST + col;

    const alturaExportada = vista.getUint16(idx * 2, true) / 65535;
    const alturaSinTallar = evaluarElevacion(generado.elevacion, puntoRio);

    expect(alturaExportada).toBeLessThan(alturaSinTallar);

    // La metadata reporta el mismo río con sus puntos en metros (x=mapX, z=mapY).
    const rioMeta = metadata.rios.find((r) => r.id === rio!.id);
    expect(rioMeta?.puntos.length).toBe(rio!.puntos.length);
  });

  it('la metadata lista tantos nodos/bosques/chokepoints como el mundo generado, y respeta ejes x/z', () => {
    const generado = generarMapa({ ancho: 800, alto: 800, seed: 5 });
    const { metadata } = exportarParaUnityTerrain(generado, [], { resolucion: RESOLUCION_TEST });

    expect(metadata.nodos.length).toBe(generado.nodos.length);
    expect(metadata.bosques.length).toBe(generado.bosques.length);
    expect(metadata.chokepoints.length).toBe(generado.chokepoints.length);
    expect(metadata.mundoMetros).toEqual({ ancho: generado.config.ancho, alto: generado.config.alto });

    if (generado.nodos.length > 0) {
      const nodo = generado.nodos[0]!;
      const nodoMeta = metadata.nodos.find((n) => n.id === nodo.id)!;
      expect(nodoMeta.x).toBe(nodo.posicion.x);
      expect(nodoMeta.z).toBe(nodo.posicion.y);
    }
  });

  describe('splatmap de biomas', () => {
    const RESOLUCION_SPLATMAP_TEST = 40;

    it('trae una capa por bioma de BIOMA_ORDEN, cada una de resolución² bytes', () => {
      const generado = generarMapa({ ancho: 800, alto: 800, seed: 3 });
      const { splatmap } = exportarParaUnityTerrain(generado, [], { resolucionSplatmap: RESOLUCION_SPLATMAP_TEST });

      expect(splatmap.resolucion).toBe(RESOLUCION_SPLATMAP_TEST);
      expect(Object.keys(splatmap.capas).sort()).toEqual([...BIOMA_ORDEN].sort());
      for (const bioma of BIOMA_ORDEN) {
        expect(splatmap.capas[bioma].length).toBe(RESOLUCION_SPLATMAP_TEST * RESOLUCION_SPLATMAP_TEST);
      }
    });

    it('es clasificación dura: cada celda tiene peso 255 en exactamente una capa y 0 en el resto', () => {
      const generado = generarMapa({ ancho: 800, alto: 800, seed: 3 });
      const { splatmap } = exportarParaUnityTerrain(generado, [], { resolucionSplatmap: RESOLUCION_SPLATMAP_TEST });

      const total = RESOLUCION_SPLATMAP_TEST * RESOLUCION_SPLATMAP_TEST;
      for (let idx = 0; idx < total; idx++) {
        const pesos = BIOMA_ORDEN.map((bioma) => splatmap.capas[bioma]![idx]!);
        expect(pesos.filter((p) => p === 255).length).toBe(1);
        expect(pesos.every((p) => p === 0 || p === 255)).toBe(true);
      }
    });

    it('coincide con evaluarBioma en cada celda muestreada (misma silla que el motor real)', () => {
      const generado = generarMapa({ ancho: 800, alto: 800, seed: 7 });
      const { splatmap } = exportarParaUnityTerrain(generado, [], { resolucionSplatmap: RESOLUCION_SPLATMAP_TEST });
      const { ancho, alto } = generado.config;

      for (let fila = 0; fila < RESOLUCION_SPLATMAP_TEST; fila += 3) {
        for (let col = 0; col < RESOLUCION_SPLATMAP_TEST; col += 3) {
          const mapX = (col / (RESOLUCION_SPLATMAP_TEST - 1)) * ancho;
          const mapY = (fila / (RESOLUCION_SPLATMAP_TEST - 1)) * alto;
          const esperado = evaluarBioma(generado.elevacion, generado.fertilidad, generado.rios, { x: mapX, y: mapY });
          const idx = fila * RESOLUCION_SPLATMAP_TEST + col;
          const biomaExportado = BIOMA_ORDEN.find((b) => splatmap.capas[b]![idx] === 255) as BiomaTipo;
          expect(biomaExportado).toBe(esperado);
        }
      }
    });

    it('la resolución de splatmap por defecto es independiente de la del heightmap', () => {
      expect(UNITY_EXPORT_DEFAULT.resolucionSplatmap).toBe(2048);
    });
  });
});
