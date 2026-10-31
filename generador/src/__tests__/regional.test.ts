import { describe, expect, it } from 'vitest';
import { crearRelieveRegional, evaluarRelieveRegional, distanciaCostaRegional } from '../regional';
import { evaluarElevacion, evaluarTerreno, gradienteElevacion } from '../elevacion';
import { generarRelieve } from '../relieve';

describe('laboratorio regional-4: costa mediterránea', () => {
  it('estrecha la península, abre golfos y conserva detalle en la costa norte', () => {
    const campo = crearRelieveRegional(42, { ancho: 32000, alto: 24000 });
    const seco = (x: number, y: number) => distanciaCostaRegional(campo, { x: x * 32000, y: y * 24000 }) > 0;
    const anchura = (y: number) => Array.from({ length: 320 }, (_, i) => 0.05 + i * 0.001)
      .filter((x) => seco(x, y)).length * 32;
    expect(anchura(0.23)).toBeGreaterThan(anchura(0.4) * 2);
    expect(seco(0.21, 0.395)).toBe(true);
    expect(seco(0.245, 0.395)).toBe(false);
    expect(seco(0.39, 0.83)).toBe(false);
    const orillaNorte = Array.from({ length: 20 }, (_, i) => 0.02 + i * 0.005).map((x) => {
      let ultimo = 0;
      for (let y = 0.05; y < 0.26; y += 0.001) if (seco(x, y)) ultimo = y;
      return ultimo * 24000;
    });
    expect(Math.max(...orillaNorte) - Math.min(...orillaNorte)).toBeGreaterThan(400);
  });
  it('serializa y reproduce la seed', () => {
    const grande = crearRelieveRegional(42, { ancho: 32000, alto: 24000 });
    expect(grande.areas).toHaveLength(16);
    expect(crearRelieveRegional(42, grande.limites)).toEqual(grande);
    expect(crearRelieveRegional(7, grande.limites)).not.toEqual(grande);
    const copia = JSON.parse(JSON.stringify(grande)) as typeof grande;
    expect(evaluarRelieveRegional(copia, { x: 1300, y: 2400 }))
      .toBe(evaluarRelieveRegional(grande, { x: 1300, y: 2400 }));
  });

  it('distingue cumbres y estribaciones sin convertir la cordillera en un muro uniforme', () => {
    const campo = crearRelieveRegional(42, { ancho: 32000, alto: 24000 });
    const b = campo.barreras[1]!;
    const alturas: number[] = [];
    for (const tramo of b.tramos) {
      const distancia = tramo.inicio + tramo.longitud / 2;
      if (distancia < 1500 || distancia > b.longitud - 1500
        || b.pasos.some(p => Math.abs(p.distancia - distancia) < p.radio * 1.6)) continue;
      alturas.push(evaluarRelieveRegional(campo, {
        x: (tramo.a.x + tramo.b.x) / 2, y: (tramo.a.y + tramo.b.y) / 2,
      }) * 1000);
    }
    expect(alturas.length).toBeGreaterThan(5);
    expect(Math.max(...alturas) - Math.min(...alturas)).toBeGreaterThan(100);
    expect(b.ramales!.some(r => Math.hypot(r.puntos[2]!.x-r.puntos[0]!.x,
      r.puntos[2]!.y-r.puntos[0]!.y) > b.ancho * 2)).toBe(true);
  });

  it('conserva fondos suaves y no introduce saltos verticales en los hombros de los puertos', () => {
    const campo = crearRelieveRegional(42, { ancho: 32000, alto: 24000 });
    const pendiente = (p: { x: number; y: number }) => Math.hypot(
      evaluarRelieveRegional(campo, { x: p.x + 1, y: p.y }) - evaluarRelieveRegional(campo, { x: p.x - 1, y: p.y }),
      evaluarRelieveRegional(campo, { x: p.x, y: p.y + 1 }) - evaluarRelieveRegional(campo, { x: p.x, y: p.y - 1 }),
    ) * 500;
    expect(campo.areas.filter((a) => pendiente(a.centro) < 0.12).length).toBeGreaterThan(campo.areas.length * 0.7);
    for (const paso of campo.barreras.flatMap((b) => b.pasos)) {
      let previo: number | undefined;
      for (let t = -1000; t <= 1000; t += 5) {
        const e = evaluarRelieveRegional(campo, { x: paso.centro.x + paso.tangente.x * t, y: paso.centro.y + paso.tangente.y * t });
        if (previo !== undefined) expect(Math.abs(e - previo) * 1000).toBeLessThan(15);
        previo = e;
      }
    }
  });

  for (const seed of [1, 7, 42]) it(`seed ${seed}: puertos secos, conexión terrestre medida y drenaje descendente`, () => {
    const mapa = generarRelieve(seed, 32000, 24000);
    expect(mapa.config.ancho).toBe(32000);
    expect(mapa.config.alto).toBe(24000);
    expect(mapa.rios.length).toBeGreaterThan(0);
    expect(mapa.lagos.length).toBeGreaterThan(0);
    // Medir los corredores previstos: cada puerto debe ser seco y de pendiente transitable.
    const pasos = mapa.elevacion.regional.barreras.flatMap((b) => b.pasos.map((paso) => {
      let pendienteMaxima = 0, seco = true;
      for (let t = -b.ancho * 2.8; t <= b.ancho * 2.8; t += 20) {
        const p = { x: paso.centro.x - paso.tangente.y * t, y: paso.centro.y + paso.tangente.x * t };
        const g = gradienteElevacion(mapa.elevacion, p, 2);
        pendienteMaxima = Math.max(pendienteMaxima, Math.hypot(g.dx, g.dy) * 1000);
        seco &&= evaluarTerreno(mapa.elevacion, p) !== 'agua';
      }
      return { ...paso.centro, pendienteMaxima, transitable: seco && pendienteMaxima <= 0.7 };
    }));
    expect(pasos.every((p) => p.transitable)).toBe(true);
    expect(mapa.conexion.conectadas).toBe(mapa.elevacion.regional.areas.length);
    const sinLagos = { ...mapa.elevacion, lagos: [] };
    for (const rio of mapa.rios) for (let i = 1; i < rio.puntos.length; i++)
      expect(evaluarElevacion(sinLagos, rio.puntos[i]!)).toBeLessThan(evaluarElevacion(sinLagos, rio.puntos[i - 1]!));
    for (const lago of mapa.lagos) expect(evaluarTerreno(mapa.elevacion, lago.centro)).toBe('agua');
  }, 180000); // el relieve incluye la rejilla de celdas de 4 u y la conexión medida en ella
});
