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
import { REJILLA_ASENTAMIENTO, TRAZADO, ZONA_INFLUENCIA } from '../../constants';
import { anadirEdificioManualmente } from '../construction';
import {
  celdaMinimaDeEdificio,
  celdasDeEdificio,
  crearAnclaNueva,
  direccionesRotadas,
  puntoDeRectangulo,
  radioMaximoRanura,
  sitioParaTipo as sitioTrazado,
  sitiosPorAtraccionDura,
  type RedDeCalles,
  tamanoDeEdificio,
  tamanoEdificio,
  tiposAfines,
  tipoAnclaParaCategoria,
} from '../trazado';
import { avanzarSimulacion } from '../simulation';
import { createRng } from '../../worldgen';
import { contextoDeTest, crearEstadoDeTest, crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, instanteDeTest } from './fixtures';

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

/** Huella del edificio de relleno. `ocupante` planta una Vivienda, que es el tipo más pequeño del catálogo —
 * pero desde el Paso 1 de la Etapa 6 (§E6.11) YA NO mide una celda, sino `TAMANO_OCUPANTE`.
 *
 * Es justo lo que rompió estos tests al reescalar: `ocupante(col, row)` prometía "ocupa ESTA celda", los
 * bucles de relleno iteraban celda a celda contando con eso, y al doblarse la huella cada Vivienda se
 * desbordaba sobre sus vecinas — incluido el hueco que el test dejaba libre a propósito, que quedaba tapado.
 * El síntoma era "0 candidatos", que no dice nada de la causa. Los rellenos recorren ahora la rejilla A PASOS
 * de esta huella, y `ocupante` recibe la celda MÍNIMA del rectángulo, no "su" celda. */
const TAMANO_OCUPANTE = tamanoEdificio('vivienda');

/** Un edificio de relleno sin uso mecánico, anclado por su celda MÍNIMA — solo para ocupar suelo y forzar
 * saturación/estrechez en los tests. Su tipo (`vivienda`) es irrelevante; lo que importa es su huella. */
function ocupante(col: number, row: number, id: string): Edificio {
  return {
    id,
    tipo: 'vivienda',
    posicion: puntoDeRectangulo({ col, row }, TAMANO_OCUPANTE),
    estado: 'activo',
    ambito: 'asentamiento',
  };
}

/** Rellena de `ocupante`s el rectángulo `[minCol, maxCol] x [minRow, maxRow]`, saltando cualquier posición
 * cuya HUELLA COMPLETA solape alguno de los rectángulos `libres`. Recorre a pasos del tamaño del ocupante,
 * no celda a celda: con huellas mayores que una celda, iterar por celda produce solapes y desbordes. */
function rellenar(
  minCol: number,
  maxCol: number,
  minRow: number,
  maxRow: number,
  libres: { minCol: number; minRow: number; ancho: number; alto: number }[]
): Edificio[] {
  const relleno: Edificio[] = [];
  let contador = 0;
  for (let col = minCol; col <= maxCol; col += TAMANO_OCUPANTE.ancho) {
    for (let row = minRow; row <= maxRow; row += TAMANO_OCUPANTE.alto) {
      const solapa = libres.some(
        (l) =>
          col < l.minCol + l.ancho &&
          col + TAMANO_OCUPANTE.ancho > l.minCol &&
          row < l.minRow + l.alto &&
          row + TAMANO_OCUPANTE.alto > l.minRow
      );
      if (solapa) continue;
      relleno.push(ocupante(col, row, `relleno-${contador++}`));
    }
  }
  return relleno;
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
      completaEn: instanteDeTest(1),
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
  it('bootstrap: la primera ancla nace en una de las 5 ranuras desde Centro Urbano, sin colisionar', () => {
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

    // Rodea Centro Urbano por completo (radio generoso: cubre cualquier rotación posible de las 5 ranuras)
    // para que NINGUNA de sus ranuras tenga hueco real. El margen se DERIVA del radio máximo que llega a
    // explorar una ranura (`RADIO_MAXIMO_RANURA`, engine/trazado.ts): con un literal, el Paso 1 de la Etapa 6
    // (§E6.11) lo dejó corto —el radio de búsqueda se dobló y el relleno no— y Centro Urbano seguía
    // encontrando hueco justo por fuera, así que nunca se marcaba como saturado.
    const margen = radioMaximoRanura() + cuTamano.ancho;
    const relleno = rellenar(
      cuMin.col - margen,
      cuMin.col + cuTamano.ancho + margen,
      cuMin.row - margen,
      cuMin.row + cuTamano.alto + margen,
      [{ minCol: cuMin.col, minRow: cuMin.row, ancho: cuTamano.ancho, alto: cuTamano.alto }]
    );
    // Una segunda ancla, bien lejos del relleno, con sitio de sobra alrededor — debe ser la que reciba la
    // ancla nueva una vez Centro Urbano quede descartado.
    const mercadoLejano: Edificio = {
      id: 'mercado-lejano',
      tipo: 'mercado',
      posicion: { x: 200 * T, y: 200 * T },
      estado: 'activo',
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

    // Puesto de Mercado forma 2 (2x6, ver PUESTO_MERCADO_FORMA) atraído al Mercado recién construido. Desde
    // 2026-08-31 el puesto puede salir girado si su lado largo pega mejor al Mercado — el test usa `sitio.rotado`.
    const sitio = sitioTrazado(conMercado, conMercado.edificios, 'puestoMercado', 2);
    expect(sitio).not.toBeNull();
    const puestoMin = celdaMinimaDeEdificio({ tipo: 'puestoMercado', nivelInterno: 2, posicion: sitio!.punto, rotado: sitio!.rotado });
    const puestoTamano = tamanoDeEdificio({ tipo: 'puestoMercado', nivelInterno: 2, rotado: sitio!.rotado });

    // Etapa 6 (§E6.7): el Mercado siembra su anillo de calle, así que "pegado al ancla" YA NO es tocarla —
    // es mirarla desde el otro lado de su calle. La referencia del desempate pasa a ser el ancla EXPANDIDA
    // por su anillo, que es contra lo que mide `sitiosPorAtraccionDura`.
    const anillo = TRAZADO.anchoCalle;
    const anclaMin = { col: mercadoMin.col - anillo, row: mercadoMin.row - anillo };
    const anclaTamano = { ancho: mercadoTamano.ancho + anillo * 2, alto: mercadoTamano.alto + anillo * 2 };

    const tocaArriba = puestoMin.row + puestoTamano.alto === anclaMin.row;
    const tocaAbajo = anclaMin.row + anclaTamano.alto === puestoMin.row;
    const tocaIzq = puestoMin.col + puestoTamano.ancho === anclaMin.col;
    const tocaDer = anclaMin.col + anclaTamano.ancho === puestoMin.col;
    expect(
      tocaArriba || tocaAbajo || tocaIzq || tocaDer,
      'el puesto debería quedar al otro lado del anillo de calle del Mercado'
    ).toBe(true);

    // En el eje de contacto, el candidato elegido cubre TODO el lado corto compartido — la prueba directa de
    // que el desempate por borde ganó al primer hueco que empatara en distancia sin más criterio.
    if (tocaArriba || tocaAbajo) {
      const solape = Math.min(puestoMin.col + puestoTamano.ancho, anclaMin.col + anclaTamano.ancho) - Math.max(puestoMin.col, anclaMin.col);
      expect(solape).toBe(Math.min(puestoTamano.ancho, anclaTamano.ancho));
    } else {
      const solape = Math.min(puestoMin.row + puestoTamano.alto, anclaMin.row + anclaTamano.alto) - Math.max(puestoMin.row, anclaMin.row);
      expect(solape).toBe(Math.min(puestoTamano.alto, anclaTamano.alto));
    }
  });
});

describe('Etapa 4 (sin cambios) — orientación intercambiable (ancho↔alto)', () => {
  it('ofrece la huella girada cuando es la única que cabe, y coloca sin invadir nada', () => {
    // Test UNITARIO sobre `sitiosPorAtraccionDura`, con `ocupadas` y `red` construidas a mano.
    //
    // Antes montaba el escenario con una simulación real y un relleno de edificios. Eso dejó de funcionar en la
    // Etapa 6 y por una razón que vale la pena dejar escrita: con las calles sobre CELDAS, `redDeCalles` planta
    // calles DENTRO del espacio libre (corredores de los propios edificios de relleno), así que un "hueco de
    // exactamente la huella girada" ya no se puede construir desde fuera — medido, 21 de sus 32 celdas
    // aparecían ocupadas por calle. No es un fallo del motor: es la calle costando suelo, que es justo el
    // objetivo del rediseño. Lo que dejó de ser posible es fabricar ese escenario indirectamente.
    const ancla: Edificio = {
      id: 'ancla-rotacion',
      tipo: 'plazaDeArmas',
      posicion: puntoDeRectangulo({ col: 0, row: 0 }, tamanoEdificio('plazaDeArmas')),
      estado: 'activo',
      ambito: 'asentamiento',
    };
    const anclaMin = celdaMinimaDeEdificio(ancla);
    const anclaTamano = tamanoDeEdificio(ancla);
    const anillo = TRAZADO.anchoCalle;

    const carpinteria = tamanoEdificio('carpinteria');
    // El hueco tiene EXACTAMENTE la huella girada: la orientación normal no entra (su ancho no cabe en estas
    // columnas), solo la girada. Va al otro lado del anillo de calle del ancla (§E6.7).
    const huecoAncho = carpinteria.alto;
    const huecoAlto = carpinteria.ancho;
    const huecoMinCol = anclaMin.col + anclaTamano.ancho + anillo;
    const huecoMinRow = anclaMin.row - Math.floor((huecoAlto - anclaTamano.alto) / 2);

    const dentro = (col: number, row: number, minCol: number, minRow: number, ancho: number, alto: number) =>
      col >= minCol && col < minCol + ancho && row >= minRow && row < minRow + alto;

    // Red: SOLO el anillo del ancla. Al construirla a mano, nada puede aparecer donde no se quiere.
    const red: RedDeCalles = { calles: new Set(), caminos: new Set() };
    for (let col = anclaMin.col - anillo; col < anclaMin.col + anclaTamano.ancho + anillo; col++) {
      for (let row = anclaMin.row - anillo; row < anclaMin.row + anclaTamano.alto + anillo; row++) {
        if (dentro(col, row, anclaMin.col, anclaMin.row, anclaTamano.ancho, anclaTamano.alto)) continue;
        red.calles.add(`${col},${row}`);
      }
    }

    // Ocupado: todo el entorno menos el ancla, su anillo y el hueco.
    const ocupadas = new Set<string>();
    const margen = huecoAlto + anclaTamano.alto + 8;
    for (let col = anclaMin.col - margen; col <= anclaMin.col + anclaTamano.ancho + margen; col++) {
      for (let row = anclaMin.row - margen; row <= anclaMin.row + anclaTamano.alto + margen; row++) {
        if (dentro(col, row, anclaMin.col - anillo, anclaMin.row - anillo, anclaTamano.ancho + anillo * 2, anclaTamano.alto + anillo * 2)) continue;
        if (dentro(col, row, huecoMinCol, huecoMinRow, huecoAncho, huecoAlto)) continue;
        ocupadas.add(`${col},${row}`);
      }
    }

    const candidatos = sitiosPorAtraccionDura(ancla, carpinteria, ocupadas, red, true);
    expect(candidatos.length, 'el hueco girado debería ofrecer al menos un candidato').toBeGreaterThan(0);
    const elegido = candidatos[0]!;
    expect(elegido.rotado, 'solo la huella GIRADA cabe en un hueco de esas columnas').toBe(true);

    // Y no invade nada: sus celdas caen todas dentro del hueco que se dejó libre.
    const colocada: Edificio = { ...ancla, id: 'carpinteria-girada', tipo: 'carpinteria', posicion: elegido.punto, rotado: true };
    for (const c of celdasDeEdificio(colocada)) {
      expect(ocupadas.has(`${c.col},${c.row}`), `pisa la celda ocupada ${c.col},${c.row}`).toBe(false);
      expect(red.calles.has(`${c.col},${c.row}`), `pisa la celda de calle ${c.col},${c.row}`).toBe(false);
    }
  });
});
describe('Etapa 5 — compás + eje rotado por asentamiento (mismo mecanismo, aplicado a las ranuras)', () => {
  it('son 5 ranuras equidistantes (72°), con el eje rotado por asentamiento y variando entre asentamientos', () => {
    const direccionesA = direccionesRotadas('asentamiento-prueba-eje-a');
    const direccionesB = direccionesRotadas('asentamiento-prueba-eje-b');
    const paso = (Math.PI * 2) / 5; // 72°

    expect(direccionesA).toHaveLength(5);

    // Equidistantes: el hueco entre ranuras consecutivas es siempre `paso`.
    for (let i = 1; i < direccionesA.length; i++) {
      expect(direccionesA[i]!.angulo - direccionesA[i - 1]!.angulo).toBeCloseTo(paso);
    }

    // El eje está rotado: ninguna ranura cae en su ángulo base sin rotar (-90° + k·72°).
    const base = Array.from({ length: 5 }, (_, k) => -Math.PI / 2 + k * paso);
    for (let k = 0; k < 5; k++) {
      expect(Math.abs(direccionesA[k]!.angulo - base[k]!)).toBeGreaterThan(1e-6);
    }

    // Y la rotación varía entre asentamientos.
    expect(direccionesA.map((d) => d.angulo)).not.toEqual(direccionesB.map((d) => d.angulo));
  });
});

describe('Regla de afinidad (2026-08-31) — desempate por vecindad con edificios del mismo tipo/categoría', () => {
  it('tiposAfines: mismo tipo o misma categoría funcional; un ancla nunca sale afín a su satélite', () => {
    expect(tiposAfines('vivienda', 'vivienda')).toBe(true);
    expect(tiposAfines('puestoMercado', 'puestoMercado')).toBe(true);
    // Industria: fundición/curtiduría/armería son afines entre sí aunque sean tipos distintos.
    expect(tiposAfines('fundicion', 'curtiduria')).toBe(true);
    expect(tiposAfines('fundicion', 'armeria')).toBe(true);
    expect(tiposAfines('armeria', 'curtiduria')).toBe(true);
    // Categorías distintas: no afines.
    expect(tiposAfines('vivienda', 'fundicion')).toBe(false);
    expect(tiposAfines('barracon', 'mercado')).toBe(false);
    // Los tipos de ancla puros (Centro Urbano, Plaza de Armas…) no están en CATEGORIA_POR_TIPO, así que un
    // satélite nunca sale afín a su ancla por esta vía — la adyacencia al ancla ya es el criterio primario.
    expect(tiposAfines('vivienda', 'centroUrbano')).toBe(false);
    expect(tiposAfines('barracon', 'plazaDeArmas')).toBe(false);
  });

  it('entre dos huecos igual de pegados al ancla, elige el que toca a los suyos', () => {
    // Mercado 6x4 en el origen, con su anillo de calle sembrado a mano (nada aparece donde no se quiere).
    const mercado: Edificio = {
      id: 'mercado-afinidad',
      tipo: 'mercado',
      posicion: puntoDeRectangulo({ col: 0, row: 0 }, tamanoEdificio('mercado')),
      estado: 'activo',
      ambito: 'asentamiento',
    };
    const anclaMin = celdaMinimaDeEdificio(mercado);
    const anclaTam = tamanoDeEdificio(mercado);

    const red: RedDeCalles = { calles: new Set(), caminos: new Set() };
    for (let dc = -1; dc <= anclaTam.ancho; dc++) {
      red.calles.add(`${anclaMin.col + dc},${anclaMin.row - 1}`);
      red.calles.add(`${anclaMin.col + dc},${anclaMin.row + anclaTam.alto}`);
    }
    for (let dr = 0; dr < anclaTam.alto; dr++) {
      red.calles.add(`${anclaMin.col - 1},${anclaMin.row + dr}`);
      red.calles.add(`${anclaMin.col + anclaTam.ancho},${anclaMin.row + dr}`);
    }

    // `ocupadas` = lo que ve toda colocación real: huella del ancla + celdas de la red (en el motor las une
    // `sueloOcupado`). Sin las celdas de red, `candidatosLibres` propondría huecos ENCIMA de la calle.
    const ocupadas = new Set<string>();
    for (let dc = 0; dc < anclaTam.ancho; dc++) {
      for (let dr = 0; dr < anclaTam.alto; dr++) ocupadas.add(`${anclaMin.col + dc},${anclaMin.row + dr}`);
    }
    for (const clave of red.calles) ocupadas.add(clave);

    // Un puesto (2x2) YA construido pegado a la cara sur, al otro lado del anillo de calle.
    const puestoTam = tamanoEdificio('puestoMercado', 3); // forma 2x2
    const existenteMin = { col: anclaMin.col, row: anclaMin.row + anclaTam.alto + 1 };
    const celdasAfines = new Set<string>();
    for (let dc = 0; dc < puestoTam.ancho; dc++) {
      for (let dr = 0; dr < puestoTam.alto; dr++) {
        const clave = `${existenteMin.col + dc},${existenteMin.row + dr}`;
        ocupadas.add(clave);
        celdasAfines.add(clave);
      }
    }

    const tocaAfin = (punto: { x: number; y: number }): boolean => {
      const min = celdaMinimaDeEdificio({ tipo: 'puestoMercado', nivelInterno: 3, posicion: punto, rotado: false });
      for (let dc = 0; dc < puestoTam.ancho; dc++) {
        for (let dr = 0; dr < puestoTam.alto; dr++) {
          for (const [ec, er] of [[0, -1], [1, 0], [0, 1], [-1, 0]] as const) {
            if (celdasAfines.has(`${min.col + dc + ec},${min.row + dr + er}`)) return true;
          }
        }
      }
      return false;
    };

    const sinAfinidad = sitiosPorAtraccionDura(mercado, puestoTam, ocupadas, red, false, false);
    const conAfinidad = sitiosPorAtraccionDura(mercado, puestoTam, ocupadas, red, false, false, celdasAfines);

    // Hay elección real: muchos huecos igual de válidos alrededor del Mercado.
    expect(sinAfinidad.length).toBeGreaterThan(6);
    // Con la regla de afinidad, el hueco elegido comparte lado con el puesto ya construido...
    expect(tocaAfin(conAfinidad[0]!.punto)).toBe(true);
    // ...y sin la regla NO lo hacía (el desempate lo decidía solo la semilla determinista).
    expect(tocaAfin(sinAfinidad[0]!.punto)).toBe(false);
  });
});
