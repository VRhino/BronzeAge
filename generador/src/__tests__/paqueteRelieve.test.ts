import { createHash } from 'node:crypto';
import { fromBinary, toBinary } from '@bufbuild/protobuf';
import { describe, expect, it } from 'vitest';
import { FACTOR } from '../campamentos';
import { evaluarElevacion, type CampoElevacion } from '../elevacion';
import { PaqueteRelieveSchema, type TeselaRelieve } from '../gen/bronzeage/v1/world_pb';
import { codificarTesela, construirPaqueteRelieve, decodificarTesela, dimensionesRelieve, muestrearTesela } from '../paqueteRelieve';
import { crearRelieveRegional } from '../regional';

/** Un relieve regional sin hidrografía ni detalle, de 40 × 30 estratégicas: basta para muestrear teselas deprisa. */
const campo: CampoElevacion = { regional: crearRelieveRegional(42, { ancho: 40 * FACTOR, alto: 30 * FACTOR }) };

describe('paquete de relieve', () => {
  it('el mapa real: 5.335 × 4.001 muestras cada 0,5 u en 42 × 32 teselas de 64 u', () => {
    expect(dimensionesRelieve(2667, 2000)).toEqual({ ancho: 2667, alto: 2000, paso: 0.5, lado: 128, columnas: 5335, filas: 4001, teselasX: 42, teselasY: 32 });
  });

  it('una tesela codificada vuelve tal cual, con su agua; sin agua no guarda nada', () => {
    const lado = 16, n = (lado + 1) ** 2, altura = new Uint16Array(n), agua = new Uint8Array(n);
    // cimas y simas que cruzan el 0 y el 65535 en la predicción
    for (let k = 0; k < n; k++) { altura[k] = (k * 7919 + (k % 5 === 0 ? 65000 : 0)) & 0xffff; agua[k] = k % 11 === 0 ? 1 : 0; }
    const t = codificarTesela(altura, agua, lado);
    expect(decodificarTesela(t, lado)).toEqual({ altura, agua });
    const seca = codificarTesela(altura, new Uint8Array(n), lado);
    expect(seca.agua.length).toBe(0);
    expect(decodificarTesela(seca, lado).agua).toEqual(new Uint8Array(n));
  });

  it('las muestras son las del generador en los vértices, con los bordes compartidos y el del mapa repetido', () => {
    const d = dimensionesRelieve(40, 30, 0.5, 32); // 81 × 61 muestras, 3 × 2 teselas
    expect([d.columnas, d.filas, d.teselasX, d.teselasY]).toEqual([81, 61, 3, 2]);
    const a = muestrearTesela(campo, d, 0, 0), b = muestrearTesela(campo, d, 1, 0), n = d.lado + 1;
    const u16 = (x: number, y: number) => Math.round(Math.min(1, Math.max(0, evaluarElevacion(campo, { x: x * FACTOR, y: y * FACTOR }))) * 65535);
    expect(a.altura[3 * n + 5]).toBe(u16(2.5, 1.5));
    for (let j = 0; j < n; j++) expect(b.altura[j * n]).toBe(a.altura[j * n + d.lado]);
    // la tercera tesela pasa del borde este (x = 40): lo de fuera repite la columna x = 40
    const c = muestrearTesela(campo, d, 2, 0), borde = 80 - 2 * 32;
    for (let i = borde; i < n; i++) expect(c.altura[7 * n + i]).toBe(u16(40, 3.5));
  });

  it('el paquete lleva su hash y el del artefacto, y exige todas las teselas', () => {
    const d = dimensionesRelieve(40, 30, 0.5, 32), teselas: TeselaRelieve[] = [];
    for (let ty = 0; ty < d.teselasY; ty++) for (let tx = 0; tx < d.teselasX; tx++) {
      const m = muestrearTesela(campo, d, tx, ty);
      teselas.push(codificarTesela(m.altura, m.agua, d.lado));
    }
    const { bytes, hash } = construirPaqueteRelieve(d, teselas, 'abc');
    const p = fromBinary(PaqueteRelieveSchema, bytes);
    expect([p.hash, p.hashArtefacto, p.paso, p.ladoTesela, p.columnas, p.filas, p.teselasX, p.teselasY]).toEqual([hash, 'abc', 500, 32, 81, 61, 3, 2]);
    p.hash = '';
    expect(createHash('sha256').update(toBinary(PaqueteRelieveSchema, p)).digest('hex')).toBe(hash);
    expect(() => construirPaqueteRelieve(d, teselas.slice(1), 'abc')).toThrow();
  });
});
