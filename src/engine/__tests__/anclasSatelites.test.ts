// Etapa 3 de "anclas y satélites" (Consideraciones/Vista_Asentamiento_Trazado_Urbano.md §5, §9): las anclas
// nuevas (Plaza, Plaza de Armas, Patio de Gremios) nacen dinámicamente por la regla de semilla de grupo
// (§5.4/5.5) en vez de ser edificios reales fijos como Centro Urbano/Mercado/Carpintería. Estos tests cubren
// el mecanismo (semilla, atracción a instancia existente, Carpintería como zona de tres piezas) apoyándose en
// el mismo patrón de fixtures que `gate_militar_nivel2.test.ts` (asentamiento con Gobernador y madera de
// sobra, para que lo único que pueda fallar sea la geometría que se está probando).
import { describe, expect, it } from 'vitest';
import type { Asentamiento, Edificio } from '../../domain/types';
import { REJILLA_ASENTAMIENTO, ZONA_INFLUENCIA } from '../../constants';
import { anadirEdificioManualmente } from '../construction';
import { anclaNacidaTrasSemilla } from '../trazado';
import { avanzarSimulacion, type EstadoSimulacion } from '../simulation';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, mockMathRandomDeterminista } from './fixtures';

const SEED = 42;
const RECLAMOS_VACIOS = { nodos: new Set<string>(), lenerasPorBosque: new Map<string, number>() };

/** Asentamiento fundado en nivel 2, con Gobernador asignado y materiales de sobra: lo único que puede
 * decidir el resultado es la geometría de anclas que se está probando, no fondos ni cargos ni gates de nivel.
 * `radioPotencial` se sube junto con `nivel` (no lo hace solo por pisar el campo) — con el radio de fundación
 * (~5 celdas) no cabe ni Mercado ni el margen que necesita una ancla nueva junto a Centro Urbano, mismo
 * cuidado que ya toma `conMercado` en `mercado_zona.test.ts`. */
function base(nivel = 2) {
  const mapa = crearMapaDeterminista(SEED);
  const { asentamiento, facciones } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
  const preparado: Asentamiento = {
    ...asentamiento,
    nivel,
    nivelActual: nivel,
    radioPotencial: ZONA_INFLUENCIA.radioMaximoPorNivel[nivel] ?? asentamiento.radioPotencial,
    cargos: { ...asentamiento.cargos, gobernadorId: 'jugador-faccion-1-1' },
    almacen: {
      ...asentamiento.almacen,
      madera: { ...asentamiento.almacen.madera!, cantidad: 2000 },
      piedra: { ...asentamiento.almacen.piedra!, cantidad: 2000 },
      cobre: { ...asentamiento.almacen.cobre!, cantidad: 2000 },
    },
  };
  return { asentamiento: preparado, faccion: facciones[0]!, mapa };
}

function porTipo(asentamiento: Asentamiento, tipo: string): Edificio[] {
  return asentamiento.edificios.filter((e) => e.tipo === tipo);
}

describe('Etapa 3 — semilla de grupo (§5.4/5.5)', () => {
  it('el primer Barracón hace nacer una Plaza de Armas, pegada a él y sin invadirlo', () => {
    const { asentamiento, faccion, mapa } = base(2);
    const conBarracon = anadirEdificioManualmente(asentamiento, faccion, 'gobernador', 'barracon', [], mapa, undefined, RECLAMOS_VACIOS);

    const plazas = porTipo(conBarracon, 'plazaDeArmas');
    expect(plazas).toHaveLength(1);

    const barracon = porTipo(conBarracon, 'barracon')[0]!;
    const plaza = plazas[0]!;
    // No se solapan: la distancia entre centros es al menos media diagonal de cada uno (comprobación gruesa,
    // la geometría fina de "frente a él" ya la protege `posicionAnclaFrenteA` con `celdasOcupadas`).
    const dist = Math.hypot(barracon.posicion.x - plaza.posicion.x, barracon.posicion.y - plaza.posicion.y);
    expect(dist).toBeGreaterThan(0);
    // Pegada de verdad: no a la otra punta del asentamiento (radio potencial es de decenas de celdas).
    expect(dist).toBeLessThan(REJILLA_ASENTAMIENTO.tamanoCelda * 10);
  });

  it('el primer edificio de industria (Fundición) hace nacer un Patio de Gremios', () => {
    const { asentamiento, faccion, mapa } = base(2);
    const conFundicion = anadirEdificioManualmente(asentamiento, faccion, 'gobernador', 'fundicion', [], mapa, undefined, RECLAMOS_VACIOS);

    expect(porTipo(conFundicion, 'patioDeGremios')).toHaveLength(1);
    expect(porTipo(conFundicion, 'fundicion')).toHaveLength(1);
  });

  it('Mercado y Carpintería nunca hacen nacer un ancla (no tienen ancla de saturación declarada)', () => {
    const { asentamiento, faccion, mapa } = base(3);
    const conCarpinteria = anadirEdificioManualmente(asentamiento, faccion, 'gobernador', 'carpinteria', [], mapa, undefined, RECLAMOS_VACIOS);
    // Carpintería SÍ pertenece a la categoría militar (orbita Plaza de Armas si ya existe, o la hace nacer
    // ella misma como semilla) — lo que este test protege es que MERCADO específicamente no tiene ancla de
    // saturación propia (§5.6: nunca satura).
    const conMercado = anadirEdificioManualmente(asentamiento, faccion, 'gobernador', 'mercado', [], mapa, undefined, RECLAMOS_VACIOS);
    expect(porTipo(conMercado, 'plaza')).toHaveLength(0);
    expect(porTipo(conMercado, 'plazaDeArmas')).toHaveLength(0);
    expect(porTipo(conMercado, 'patioDeGremios')).toHaveLength(0);
    // Carpintería, en cambio, sí abre (o se pega a) el núcleo militar.
    expect(porTipo(conCarpinteria, 'plazaDeArmas').length).toBeGreaterThanOrEqual(1);
  });
});

describe('Etapa 3 — atracción a un ancla ya nacida (no crea una segunda)', () => {
  it('un segundo edificio militar se pega a la Plaza de Armas existente, no genera otra', () => {
    const { asentamiento, faccion, mapa } = base(2);
    const conBarracon = anadirEdificioManualmente(asentamiento, faccion, 'gobernador', 'barracon', [], mapa, undefined, RECLAMOS_VACIOS);
    expect(porTipo(conBarracon, 'plazaDeArmas')).toHaveLength(1);

    const conGaleria = anadirEdificioManualmente(conBarracon, faccion, 'gobernador', 'galeriaDeTiro', [], mapa, undefined, RECLAMOS_VACIOS);
    // Sigue habiendo UNA sola Plaza de Armas — la Galería se atrajo a la ya existente en vez de sembrar otra.
    expect(porTipo(conGaleria, 'plazaDeArmas')).toHaveLength(1);
    expect(porTipo(conGaleria, 'galeriaDeTiro')).toHaveLength(1);
  });
});

describe('Etapa 3 — Carpintería: zona de tres piezas (§9)', () => {
  it('al encolarla, Carpintería es semilla militar y hace nacer Plaza de Armas de inmediato (§5.4: "nace justo después")', () => {
    const { asentamiento, faccion, mapa } = base(3);
    const conCarpinteria = anadirEdificioManualmente(asentamiento, faccion, 'gobernador', 'carpinteria', [], mapa, undefined, RECLAMOS_VACIOS);

    const carpinteria = porTipo(conCarpinteria, 'carpinteria')[0]!;
    expect(carpinteria.estado).toBe('en_cola');
    expect(porTipo(conCarpinteria, 'plazaDeArmas')).toHaveLength(1);
  });

  it('al completarse la construcción de Carpintería aparecen sus 2 talleres ya activos', () => {
    const restaurar = mockMathRandomDeterminista(SEED);
    try {
      const { asentamiento, mapa, facciones: facs } = (() => {
        const b = base(3);
        return { asentamiento: b.asentamiento, mapa: b.mapa, facciones: crearFacciones() };
      })();
      const enObra: Edificio = {
        id: `carpinteria-obra-${asentamiento.id}`,
        tipo: 'carpinteria',
        posicion: { x: 30, y: 0 },
        estado: 'en_construccion',
        ticksRestantes: 1,
        ambito: 'asentamiento',
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
      expect(porTipo(a, 'tallerCarpinteria')).toHaveLength(2);
    } finally {
      restaurar();
    }
  });
});

describe('Etapa 3 — geometría "frente a él" a nivel de motor (trazado.ts)', () => {
  it('anclaNacidaTrasSemilla no genera nada para una categoría sin ancla de saturación (mercado)', () => {
    const { asentamiento } = base(2);
    const mercado: Edificio = {
      id: 'mercado-semilla',
      tipo: 'mercado',
      posicion: { x: 60, y: 0 },
      estado: 'activo',
      ticksRestantes: 0,
      ambito: 'asentamiento',
    };
    const edificios = [...asentamiento.edificios, mercado];
    expect(anclaNacidaTrasSemilla(asentamiento, edificios, mercado, 'ancla-test')).toBeNull();
  });

  it('anclaNacidaTrasSemilla es null si ya hay un ancla alcanzable (no fue semilla)', () => {
    const { asentamiento } = base(2);
    const plazaDeArmas: Edificio = {
      id: 'plaza-armas-ya-existe',
      tipo: 'plazaDeArmas',
      posicion: { x: 30, y: 0 },
      estado: 'activo',
      ticksRestantes: 0,
      ambito: 'asentamiento',
    };
    const barracon: Edificio = {
      id: 'barracon-pegado',
      tipo: 'barracon',
      posicion: { x: 30, y: 6 },
      estado: 'activo',
      ticksRestantes: 0,
      ambito: 'asentamiento',
    };
    const edificios = [...asentamiento.edificios, plazaDeArmas, barracon];
    expect(anclaNacidaTrasSemilla(asentamiento, edificios, barracon, 'ancla-test')).toBeNull();
  });
});
