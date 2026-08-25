// Etapa 5 (Consideraciones/Vista_Asentamiento_Trazado_Urbano.md): árbol único de anclas — reemplaza la
// mecánica de "semilla-edificio" de la Etapa 3 (un edificio caía primero por el reparto de barrio, y RECIÉN
// DESPUÉS se comprobaba si necesitaba un ancla que no existía). Ahora el ancla nace SIEMPRE primero: cualquier
// edificio de una categoría con ancla exige que ya haya una alcanzable antes de calcular dónde se coloca
// (`asegurarAnclaPara`, construction.ts). El "árbol" en sí (`semillaActiva`/`crearAnclaNueva`, trazado.ts) es
// un único pool de TODAS las anclas del asentamiento, sin distinguir tipo ni categoría — una Plaza de Armas
// puede nacer como hija de un Pozo. Estos tests cubren ambas capas por separado: la Lógica 1 (árbol de
// anclas, este archivo) y la Lógica 2 (satélites de una ancla, `sitiosPorAtraccionDura` — sin cambios en esta
// etapa, solo se re-verifica que sigue funcionando con el nuevo mecanismo de creación).
import { describe, expect, it } from 'vitest';
import type { Asentamiento, Edificio } from '../../domain/types';
import { REJILLA_ASENTAMIENTO, ZONA_INFLUENCIA } from '../../constants';
import { anadirEdificioManualmente } from '../construction';
import {
  celdaMinimaDeEdificio,
  celdasDeEdificio,
  crearAnclaNueva,
  direccionesRotadas,
  sitioParaTipo as sitioTrazado,
  sitiosParaTipo as sitiosTrazado,
  tamanoDeEdificio,
  tipoAnclaParaCategoria,
} from '../trazado';
import { avanzarSimulacion } from '../simulation';
import { createRng } from '../../worldgen';
import { contextoDeTest, crearEstadoDeTest, crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest } from './fixtures';

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

/** Un edificio 1x1 sin uso mecánico, solo para ocupar celdas y forzar saturación/estrechez en los tests —
 * mismo patrón que un edificio "de verdad", pero su tipo (`vivienda`) es irrelevante aquí. */
function ocupante(col: number, row: number, id: string): Edificio {
  const T = REJILLA_ASENTAMIENTO.tamanoCelda;
  return {
    id,
    tipo: 'vivienda',
    posicion: { x: (col + 0.5) * T, y: (row + 0.5) * T },
    estado: 'activo',
    ticksRestantes: 0,
    ambito: 'asentamiento',
  };
}

describe('Etapa 5 — construir un edificio de una categoría con ancla garantiza el ancla primero', () => {
  it('el primer Barracón hace que exista una Plaza de Armas alcanzable', () => {
    const { asentamiento, faccion, mapa } = base(2);
    const conBarracon = anadirEdificioManualmente(asentamiento, faccion, 'gobernador', 'barracon', [], mapa, undefined, RECLAMOS_VACIOS);
    expect(porTipo(conBarracon, 'plazaDeArmas')).toHaveLength(1);
    expect(porTipo(conBarracon, 'barracon')).toHaveLength(1);
  });

  it('el primer edificio de industria (Fundición) hace que exista un Patio de Gremios', () => {
    const { asentamiento, faccion, mapa } = base(2);
    const conFundicion = anadirEdificioManualmente(asentamiento, faccion, 'gobernador', 'fundicion', [], mapa, undefined, RECLAMOS_VACIOS);
    expect(porTipo(conFundicion, 'patioDeGremios')).toHaveLength(1);
    expect(porTipo(conFundicion, 'fundicion')).toHaveLength(1);
  });

  it('Mercado nace como su propia ancla; nunca genera plaza/plazaDeArmas/patioDeGremios', () => {
    const { asentamiento, faccion, mapa } = base(3);
    const conMercado = anadirEdificioManualmente(asentamiento, faccion, 'gobernador', 'mercado', [], mapa, undefined, RECLAMOS_VACIOS);
    expect(porTipo(conMercado, 'mercado')).toHaveLength(1);
    expect(porTipo(conMercado, 'plaza')).toHaveLength(0);
    expect(porTipo(conMercado, 'plazaDeArmas')).toHaveLength(0);
    expect(porTipo(conMercado, 'patioDeGremios')).toHaveLength(0);
  });

  it('Carpintería (categoría militar) también hace que exista una Plaza de Armas si no había ninguna', () => {
    const { asentamiento, faccion, mapa } = base(3);
    const conCarpinteria = anadirEdificioManualmente(asentamiento, faccion, 'gobernador', 'carpinteria', [], mapa, undefined, RECLAMOS_VACIOS);
    const carpinteria = porTipo(conCarpinteria, 'carpinteria')[0]!;
    expect(carpinteria.estado).toBe('en_cola');
    expect(porTipo(conCarpinteria, 'plazaDeArmas')).toHaveLength(1);
  });
});

describe('Etapa 5 — atracción a un ancla ya existente (Lógica 2, no crea una segunda)', () => {
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

describe('Etapa 5 — Carpintería: zona de tres piezas (§9, Lógica 2, sin cambios)', () => {
  it('al completarse la construcción de Carpintería aparecen sus 2 talleres ya activos', () => {
    const rng = createRng(SEED);
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
    let estado = crearEstadoDeTest([{ ...asentamiento, edificios: [...asentamiento.edificios, enObra] }], facs);
    estado = avanzarSimulacion(estado, mapa, contextoDeTest(1, rng));

    const a = estado.asentamientos[0]!;
    expect(a.edificios.find((e) => e.id === enObra.id)!.estado).toBe('activo');
    expect(porTipo(a, 'tallerCarpinteria')).toHaveLength(2);
  });
});

describe('Etapa 5 — árbol único de anclas, a nivel de motor (trazado.ts)', () => {
  it('bootstrap: la primera ancla nace en una de las 8 direcciones desde Centro Urbano, sin colisionar', () => {
    const { asentamiento } = base(2);
    const resultado = crearAnclaNueva(asentamiento.id, asentamiento.edificios, 'plazaDeArmas', 'test-plaza-armas');
    expect(resultado).not.toBeNull();
    expect(resultado!.nuevaAncla.tipo).toBe('plazaDeArmas');
    expect(resultado!.anclasRecienSaturadas).toHaveLength(0);

    const ocupadas = new Set(asentamiento.edificios.flatMap((e) => celdasDeEdificio(e).map((c) => `${c.col},${c.row}`)));
    for (const c of celdasDeEdificio(resultado!.nuevaAncla)) {
      expect(ocupadas.has(`${c.col},${c.row}`)).toBe(false);
    }
  });

  it('árbol mixto: una ancla nueva puede nacer de una semilla de OTRO tipo, sin filtrar por categoría', () => {
    const { asentamiento } = base(2);
    const centroUrbano = porTipo(asentamiento, 'centroUrbano')[0]!;
    const T = REJILLA_ASENTAMIENTO.tamanoCelda;
    // Centro Urbano queda descartado a mano; un Pozo (categoría residencial, tipo distinto de lo que se va a
    // crear) es la ÚNICA semilla disponible — si el árbol filtrara por tipo/categoría, no habría ninguna
    // semilla "militar" válida para hacer nacer una Plaza de Armas aquí.
    const pozo: Edificio = {
      id: 'pozo-semilla-mixta',
      tipo: 'pozo',
      posicion: { x: 60 * T, y: 0 },
      estado: 'activo',
      ticksRestantes: 0,
      ambito: 'asentamiento',
    };
    const edificios = [
      ...asentamiento.edificios.map((e) => (e.id === centroUrbano.id ? { ...e, semillaSaturada: true } : e)),
      pozo,
    ];

    const resultado = crearAnclaNueva(asentamiento.id, edificios, 'plazaDeArmas', 'test-arbol-mixto');
    expect(resultado).not.toBeNull();
    expect(resultado!.nuevaAncla.tipo).toBe('plazaDeArmas');
    const distAlPozo = Math.hypot(resultado!.nuevaAncla.posicion.x - pozo.posicion.x, resultado!.nuevaAncla.posicion.y - pozo.posicion.y);
    expect(distAlPozo).toBeLessThan(30 * T);
  });

  it('saturación y descarte: si la semilla más cercana no tiene ninguna ranura libre, se descarta y se prueba la siguiente', () => {
    const { asentamiento } = base(2);
    const centroUrbano = porTipo(asentamiento, 'centroUrbano')[0]!;
    const cuMin = celdaMinimaDeEdificio(centroUrbano);
    const cuTamano = tamanoDeEdificio(centroUrbano);
    const T = REJILLA_ASENTAMIENTO.tamanoCelda;

    // Rodea Centro Urbano por completo (radio generoso: cubre cualquier rotación posible de las 8 direcciones)
    // para que NINGUNA de sus ranuras tenga hueco real.
    const relleno: Edificio[] = [];
    let contador = 0;
    const margen = 20;
    for (let col = cuMin.col - margen; col <= cuMin.col + cuTamano.ancho + margen; col++) {
      for (let row = cuMin.row - margen; row <= cuMin.row + cuTamano.alto + margen; row++) {
        const enCU = col >= cuMin.col && col < cuMin.col + cuTamano.ancho && row >= cuMin.row && row < cuMin.row + cuTamano.alto;
        if (enCU) continue;
        relleno.push(ocupante(col, row, `relleno-${contador++}`));
      }
    }
    // Una segunda ancla, bien lejos del relleno, con sitio de sobra alrededor — debe ser la que reciba la
    // ancla nueva una vez Centro Urbano quede descartado.
    const mercadoLejano: Edificio = {
      id: 'mercado-lejano',
      tipo: 'mercado',
      posicion: { x: 200 * T, y: 200 * T },
      estado: 'activo',
      ticksRestantes: 0,
      ambito: 'asentamiento',
    };
    const edificios = [...asentamiento.edificios, ...relleno, mercadoLejano];

    const resultado = crearAnclaNueva(asentamiento.id, edificios, 'plazaDeArmas', 'test-descarte');
    expect(resultado).not.toBeNull();
    expect(resultado!.anclasRecienSaturadas).toContain(centroUrbano.id);
    const distAlMercado = Math.hypot(
      resultado!.nuevaAncla.posicion.x - mercadoLejano.posicion.x,
      resultado!.nuevaAncla.posicion.y - mercadoLejano.posicion.y
    );
    expect(distAlMercado).toBeLessThan(50 * T);
  });
});

describe('Etapa 4 (sin cambios) — variedad de anclas residenciales, ahora aislada del árbol', () => {
  it('el sorteo determinista produce los 3 tipos a lo largo de suficientes intentos', () => {
    const tiposVistos = new Set<string>();
    for (let i = 0; i < 60; i++) {
      const tipo = tipoAnclaParaCategoria('residencial', `semilla-variedad-${i}`);
      expect(tipo).not.toBeNull();
      tiposVistos.add(tipo!);
    }
    expect(tiposVistos).toEqual(new Set(['plaza', 'pozo', 'parque']));
  });

  it('militar e industria siempre sortean su único tipo (no-op)', () => {
    expect(tipoAnclaParaCategoria('militar', 'cualquier-semilla')).toBe('plazaDeArmas');
    expect(tipoAnclaParaCategoria('industria', 'otra-semilla')).toBe('patioDeGremios');
  });

  it('mercado y carpinteria (sin ancla de saturación) devuelven null', () => {
    expect(tipoAnclaParaCategoria('mercado', 'x')).toBeNull();
    expect(tipoAnclaParaCategoria('carpinteria', 'y')).toBeNull();
  });
});

describe('Etapa 4 (sin cambios) — desempate por máximo borde compartido', () => {
  it('la atracción dura prefiere compartir todo un lado del ancla, no solo una esquina', () => {
    const { asentamiento, faccion, mapa } = base(2);
    const conMercado = anadirEdificioManualmente(asentamiento, faccion, 'gobernador', 'mercado', [], mapa, undefined, RECLAMOS_VACIOS);
    const mercado = porTipo(conMercado, 'mercado')[0]!;
    const mercadoMin = celdaMinimaDeEdificio(mercado);
    const mercadoTamano = tamanoDeEdificio(mercado);

    // Puesto de Mercado forma 2 (3x2, ver PUESTO_MERCADO_FORMA) atraído al Mercado recién construido.
    const sitio = sitioTrazado(conMercado, conMercado.edificios, 'puestoMercado', 2);
    expect(sitio).not.toBeNull();
    const puestoMin = celdaMinimaDeEdificio({ tipo: 'puestoMercado', nivelInterno: 2, posicion: sitio!.punto, rotado: sitio!.rotado });
    const puestoTamano = tamanoDeEdificio({ tipo: 'puestoMercado', nivelInterno: 2, rotado: sitio!.rotado });

    const tocaArriba = puestoMin.row + puestoTamano.alto === mercadoMin.row;
    const tocaAbajo = mercadoMin.row + mercadoTamano.alto === puestoMin.row;
    const tocaIzq = puestoMin.col + puestoTamano.ancho === mercadoMin.col;
    const tocaDer = mercadoMin.col + mercadoTamano.ancho === puestoMin.col;
    expect(tocaArriba || tocaAbajo || tocaIzq || tocaDer).toBe(true);

    // En el eje de contacto, el candidato elegido cubre TODO el lado corto compartido — la prueba directa de
    // que el desempate por borde ganó al primer hueco que tocara `hueco === 0` sin más criterio.
    if (tocaArriba || tocaAbajo) {
      const solape = Math.min(puestoMin.col + puestoTamano.ancho, mercadoMin.col + mercadoTamano.ancho) - Math.max(puestoMin.col, mercadoMin.col);
      expect(solape).toBe(Math.min(puestoTamano.ancho, mercadoTamano.ancho));
    } else {
      const solape = Math.min(puestoMin.row + puestoTamano.alto, mercadoMin.row + mercadoTamano.alto) - Math.max(puestoMin.row, mercadoMin.row);
      expect(solape).toBe(Math.min(puestoTamano.alto, mercadoTamano.alto));
    }
  });
});

describe('Etapa 4 (sin cambios) — orientación intercambiable (ancho↔alto)', () => {
  it('ofrece la huella girada cuando es la única que cabe, y coloca sin invadir nada', () => {
    const { asentamiento, faccion, mapa } = base(2);
    const conBarracon = anadirEdificioManualmente(asentamiento, faccion, 'gobernador', 'barracon', [], mapa, undefined, RECLAMOS_VACIOS);
    const plazaDeArmas = porTipo(conBarracon, 'plazaDeArmas')[0]!;
    const plazaMin = celdaMinimaDeEdificio(plazaDeArmas);
    const plazaTamano = tamanoDeEdificio(plazaDeArmas); // 2x2

    // Hueco libre pegado a la derecha de la Plaza de Armas: 2 columnas x 4 filas. Carpintería (4x2) no cabe
    // tal cual (4 de ancho no entra en 2 columnas), solo girada (2x4).
    const huecoMinCol = plazaMin.col + plazaTamano.ancho;
    const huecoMinRow = plazaMin.row - 1;
    const margen = 9;
    const relleno: Edificio[] = [];
    let contador = 0;
    for (let col = plazaMin.col - margen; col <= plazaMin.col + plazaTamano.ancho + margen; col++) {
      for (let row = plazaMin.row - margen; row <= plazaMin.row + plazaTamano.alto + margen; row++) {
        const enPlaza = col >= plazaMin.col && col < plazaMin.col + plazaTamano.ancho && row >= plazaMin.row && row < plazaMin.row + plazaTamano.alto;
        const enHueco = col >= huecoMinCol && col < huecoMinCol + 2 && row >= huecoMinRow && row < huecoMinRow + 4;
        if (enPlaza || enHueco) continue;
        relleno.push(ocupante(col, row, `relleno-${contador++}`));
      }
    }
    const edificios = [...conBarracon.edificios, ...relleno];

    const candidatos = sitiosTrazado(conBarracon, edificios, 'carpinteria');
    expect(candidatos.length).toBeGreaterThan(0);
    const elegido = candidatos[0]!;
    expect(elegido.rotado).toBe(true);

    const carpinteriaGirada: Edificio = {
      id: 'carpinteria-girada-test',
      tipo: 'carpinteria',
      posicion: elegido.punto,
      estado: 'activo',
      ticksRestantes: 0,
      ambito: 'asentamiento',
      rotado: true,
    };
    const ocupadas = new Set(edificios.flatMap((e) => celdasDeEdificio(e).map((c) => `${c.col},${c.row}`)));
    for (const c of celdasDeEdificio(carpinteriaGirada)) {
      expect(ocupadas.has(`${c.col},${c.row}`)).toBe(false);
    }
  });
});

describe('Etapa 5 — compás + eje rotado por asentamiento (mismo mecanismo, aplicado a las 8 ranuras)', () => {
  it('las 8 direcciones se desvían de los ángulos exactos, y varían entre asentamientos', () => {
    const direccionesA = direccionesRotadas('asentamiento-prueba-eje-a');
    const direccionesB = direccionesRotadas('asentamiento-prueba-eje-b');
    const paso = Math.PI / 4;

    for (const { angulo } of direccionesA) {
      const resto = ((angulo % paso) + paso) % paso;
      expect(resto).toBeGreaterThan(0);
    }
    expect(direccionesA.map((d) => d.angulo)).not.toEqual(direccionesB.map((d) => d.angulo));
  });
});
