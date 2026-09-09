// Contrato que TODO adaptador de `AlmacenDeObjetos` debe cumplir. Cada adaptador (`enDisco`, `enLibsql`, y
// los que vengan) llama a `pruebasDeContrato` con una fábrica que devuelve un almacén vacío y aislado. Si el
// comportamiento observable es el mismo, `persistenciaPartida`/`eventosDePartida`/etc. funcionan igual sobre
// cualquiera de ellos — que es justo lo que el puerto promete.
import { describe, expect, it } from 'vitest';
import type { AlmacenDeObjetos } from '../almacenDeObjetos';

export function pruebasDeContrato(nombre: string, crear: () => Promise<AlmacenDeObjetos>): void {
  describe(`AlmacenDeObjetos — contrato (${nombre})`, () => {
    it('leer de una clave inexistente devuelve null, no lanza', async () => {
      const a = await crear();
      expect(await a.leer('no-existe.json')).toBeNull();
    });

    it('escribir y volver a leer devuelve el mismo contenido', async () => {
      const a = await crear();
      await a.escribir('g1.json', '{"v":1}');
      expect(await a.leer('g1.json')).toBe('{"v":1}');
    });

    it('escribir reemplaza el contenido entero', async () => {
      const a = await crear();
      await a.escribir('g1.json', 'viejo-y-largo');
      await a.escribir('g1.json', 'nuevo');
      expect(await a.leer('g1.json')).toBe('nuevo');
    });

    it('anexar acumula al final y crea la clave si no existe', async () => {
      const a = await crear();
      await a.anexar('g1.jsonl', 'a\n');
      await a.anexar('g1.jsonl', 'b\n');
      expect(await a.leer('g1.jsonl')).toBe('a\nb\n');
    });

    it('anexar tras escribir concatena, no reemplaza', async () => {
      const a = await crear();
      await a.escribir('g1.jsonl', 'base\n');
      await a.anexar('g1.jsonl', 'mas\n');
      expect(await a.leer('g1.jsonl')).toBe('base\nmas\n');
    });

    it('listar filtra por prefijo; "" devuelve todas las claves', async () => {
      const a = await crear();
      await a.escribir('g1.json', '{}');
      await a.escribir('g1.eventos.jsonl', '');
      await a.escribir('identidad.json', '{}');
      expect((await a.listar('')).sort()).toEqual(['g1.eventos.jsonl', 'g1.json', 'identidad.json']);
      expect((await a.listar('g1.')).sort()).toEqual(['g1.eventos.jsonl', 'g1.json']);
    });

    it('listar de un almacén vacío devuelve [], no lanza', async () => {
      const a = await crear();
      expect(await a.listar('')).toEqual([]);
    });

    it('el contenido con caracteres especiales de SQL/comodín sobrevive un round-trip', async () => {
      const a = await crear();
      const raro = `{"s":"100% de a_b\\nc'd"}`;
      await a.escribir("g_%.json", raro);
      expect(await a.leer("g_%.json")).toBe(raro);
      // Y el `_`/`%` de la clave no ensancha el filtro de `listar`.
      await a.escribir('gX.json', '{}');
      expect(await a.listar('g_%')).toEqual(['g_%.json']);
    });
  });
}
