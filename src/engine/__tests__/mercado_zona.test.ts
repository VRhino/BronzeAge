// El Mercado no es un edificio suelto sino una ZONA (a petición del usuario, ver
// Consideraciones/Vista_Asentamiento_Trazado_Urbano.md): una pieza principal —el `mercado` de siempre, que
// conserva la unicidad y el cupo de flota— más piezas satélite de tipo `puestoMercado` que aparecen al
// alcanzar cada nivel interno. 3 piezas en nivel 1, 10 en nivel 2, 12 en nivel 3.
//
// Lo que se protege aquí es que el COMERCIO no se entere del cambio: `cupoCaravanas` y `tieneMercadoActivo`
// siguen viendo exactamente una instancia de Mercado por muchos puestos que haya alrededor.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, Edificio } from '../../domain/types';
import { MERCADO_PUESTOS_POR_NIVEL, PUESTO_MERCADO_FORMA, ZONA_INFLUENCIA } from '../../constants';
import { cupoCaravanas, tieneMercadoActivo } from '../asentamientoQuery';
import { anadirEdificioManualmente, ConstruccionManualInvalidaError, migrarEdificiosAEspacioLocal } from '../construction';
import { avanzarSimulacion, type EstadoSimulacion } from '../simulation';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, mockMathRandomDeterminista } from './fixtures';

const SEED = 42;

/** Piezas totales de la zona en un nivel dado: la principal más todos los puestos acumulados hasta ahí. */
function piezasEsperadas(nivel: number): number {
  let puestos = 0;
  for (let n = 1; n <= nivel; n++) puestos += (MERCADO_PUESTOS_POR_NIVEL[n] ?? []).length;
  return puestos + 1;
}

/**
 * Asentamiento con un Mercado de nivel interno `nivelInterno` y SIN puestos — la forma en que se ven los saves
 * anteriores a que el Mercado fuera una zona.
 *
 * El asentamiento se pone en el nivel que ese Mercado exigiría (los gates de mejora del catálogo piden nivel 2
 * y 3), con el radio de influencia que le corresponde. No es decoración del test: la zona de nivel 3 son 36
 * celdas de puestos y no caben en la cuña del barrio con el radio inicial de 30 — se colocarían solo algunos y
 * el resto se saltaría, que es el comportamiento correcto pero no lo que este test quiere medir.
 */
function conMercado(base: Asentamiento, nivelInterno: number): Asentamiento {
  const mercado: Edificio = {
    id: `mercado-${base.id}`,
    tipo: 'mercado',
    posicion: { x: 30, y: 0 },
    estado: 'activo',
    ticksRestantes: 0,
    ambito: 'asentamiento',
    nivelInterno,
  };
  return {
    ...base,
    nivel: nivelInterno,
    radioPotencial: ZONA_INFLUENCIA.radioMaximoPorNivel[nivelInterno] ?? base.radioPotencial,
    edificios: [...base.edificios, mercado],
  };
}

function puestos(asentamiento: Asentamiento): Edificio[] {
  return asentamiento.edificios.filter((e) => e.tipo === 'puestoMercado');
}

describe('Mercado como zona de varias piezas', () => {
  it('la composición por nivel es 3 / 10 / 12 piezas', () => {
    expect(piezasEsperadas(1)).toBe(3);
    expect(piezasEsperadas(2)).toBe(10);
    expect(piezasEsperadas(3)).toBe(12);
  });

  it('todas las formas de puesto declaradas existen en la tabla de formas', () => {
    for (const formas of Object.values(MERCADO_PUESTOS_POR_NIVEL)) {
      for (const forma of formas) expect(PUESTO_MERCADO_FORMA[forma]).toBeDefined();
    }
  });

  it('la migración completa los puestos que le faltan a un Mercado de save viejo, y es idempotente', () => {
    const mapa = crearMapaDeterminista(SEED);
    const { asentamiento } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);

    for (const nivel of [1, 2, 3]) {
      const conZonaIncompleta = conMercado(asentamiento, nivel);
      expect(puestos(conZonaIncompleta)).toHaveLength(0);

      const [migrado] = migrarEdificiosAEspacioLocal([conZonaIncompleta]);
      const piezas = puestos(migrado!).length + 1;
      expect(piezas, `nivel ${nivel}`).toBe(piezasEsperadas(nivel));

      // Idempotente: volver a migrar no añade ni una pieza más.
      const [reMigrado] = migrarEdificiosAEspacioLocal([migrado!]);
      expect(puestos(reMigrado!)).toHaveLength(puestos(migrado!).length);
    }
  });

  it('los puestos no alteran el comercio: sigue habiendo UN Mercado y el cupo de flota es el de su nivel', () => {
    const mapa = crearMapaDeterminista(SEED);
    const { asentamiento } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
    const cupoPorNivel: Record<number, number> = { 1: 2, 2: 4, 3: 6 };

    for (const nivel of [1, 2, 3]) {
      const [conZona] = migrarEdificiosAEspacioLocal([conMercado(asentamiento, nivel)]);
      expect(conZona!.edificios.filter((e) => e.tipo === 'mercado')).toHaveLength(1);
      expect(tieneMercadoActivo(conZona!)).toBe(true);
      expect(cupoCaravanas(conZona!), `cupo en nivel ${nivel}`).toBe(cupoPorNivel[nivel]);
    }
  });

  it('ningún puesto se cuela como Mercado ni progresa de nivel por su cuenta', () => {
    const mapa = crearMapaDeterminista(SEED);
    const { asentamiento } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
    const [conZona] = migrarEdificiosAEspacioLocal([conMercado(asentamiento, 3)]);

    // `nivelInterno` en un puesto identifica su FORMA, no una progresión: nunca puede salirse de la tabla.
    for (const puesto of puestos(conZona!)) {
      expect(PUESTO_MERCADO_FORMA[puesto.nivelInterno ?? 1]).toBeDefined();
    }
    expect(conZona!.edificios.filter((e) => e.tipo === 'mercado')).toHaveLength(1);
  });

  it('al completarse la construcción del Mercado aparecen los puestos de su nivel 1', () => {
    const restaurar = mockMathRandomDeterminista(SEED);
    try {
      const mapa = crearMapaDeterminista(SEED);
      const facciones = crearFacciones();
      const { asentamiento, facciones: facs } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);
      const enObra: Edificio = {
        id: `mercado-obra-${asentamiento.id}`,
        tipo: 'mercado',
        posicion: { x: 30, y: 0 },
        estado: 'en_construccion',
        ticksRestantes: 1,
        ambito: 'asentamiento',
        nivelInterno: 1,
      };
      let estado: EstadoSimulacion = {
        asentamientos: [{ ...asentamiento, edificios: [...asentamiento.edificios, enObra] }],
        facciones: facs,
        caravanas: [],
        acuerdos: [],
        ordenes: [],
        relaciones: [],
        titulos: [],
        caminos: [],
        campamentosBandidos: [],
        bandidosProximoSpawnTick: 0,
      };
      estado = avanzarSimulacion(estado, mapa, 1);

      const a = estado.asentamientos[0]!;
      expect(a.edificios.find((e) => e.id === enObra.id)!.estado).toBe('activo');
      expect(puestos(a).length + 1).toBe(piezasEsperadas(1));
    } finally {
      restaurar();
    }
  });

  it('un puesto no se puede añadir a la cola a mano', () => {
    const mapa = crearMapaDeterminista(SEED);
    const facciones = crearFacciones();
    const { asentamiento, facciones: facs } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);
    const conGobernador: Asentamiento = {
      ...asentamiento,
      cargos: { ...asentamiento.cargos, gobernadorId: 'jugador-faccion-1-1' },
    };
    expect(() =>
      anadirEdificioManualmente(conGobernador, facs[0]!, 'gobernador', 'puestoMercado', [], mapa, undefined, {
        nodos: new Set(),
        lenerasPorBosque: new Map(),
      })
    ).toThrow(ConstruccionManualInvalidaError);
  });
});
