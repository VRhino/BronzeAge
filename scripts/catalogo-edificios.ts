/**
 * Catálogo de edificios — informe consolidado (a petición del usuario).
 *
 * Reúne en un solo sitio, para CADA tipo de edificio, todo lo que hoy vive repartido entre
 * `src/constants.ts` (costo, tiempo, recetas, niveles internos, capacidades) y `src/engine/trazado.ts` /
 * `EDIFICIO_TAMANO` (huella en la rejilla local). Todo se DERIVA de esos módulos: este script no copia
 * ninguna cifra, así que no puede quedar desincronizado del balance real.
 *
 * Uso:
 *   npx tsx scripts/catalogo-edificios.ts            # informe legible por pantalla
 *   npx tsx scripts/catalogo-edificios.ts --json     # el mismo contenido como JSON
 *   npx tsx scripts/catalogo-edificios.ts --md       # el mismo contenido como tabla Markdown
 *
 * La huella se reporta en dos unidades:
 *   - celdas: lo que se guarda en `EDIFICIO_TAMANO` (rejilla doblada, Etapa 6 §E6.11).
 *   - unidades locales: celdas × `REJILLA_ASENTAMIENTO.tamanoCelda` — el tamaño FÍSICO en la Vista de
 *     Asentamiento, que NO cambió al doblar la resolución.
 */
import {
  EDIFICIO_CATALOGO,
  EDIFICIO_TAMANO,
  EDIFICIO_TAMANO_POR_DEFECTO,
  MERCADO_PUESTOS_POR_NIVEL,
  PUESTO_MERCADO_FORMA,
  REJILLA_ASENTAMIENTO,
  produccionTrigoDeGranja,
} from '../src/constants';
import { EDIFICIOS_TIPO, type EdificioTipo } from '../src/domain/types';
import { permiteRotacion, tamanoEdificio } from '../src/engine/trazado';

const T = REJILLA_ASENTAMIENTO.tamanoCelda;

/** Nombre legible — espejo de `cliente/src/ui/canvas.ts:EDIFICIO_ETIQUETA` (presentación pura, se mantiene a mano). */
const NOMBRE: Record<EdificioTipo, string> = {
  centroUrbano: 'Centro Urbano',
  vivienda: 'Vivienda',
  granja: 'Granja',
  cantera: 'Cantera',
  lenera: 'Leñera',
  almacen: 'Almacén',
  mina: 'Mina (oro)',
  minaCobre: 'Mina (cobre)',
  minaEstano: 'Mina (estaño)',
  fundicion: 'Fundición',
  granFundicion: 'Gran Fundición',
  corral: 'Corral',
  armeria: 'Armería',
  curtiduria: 'Curtiduría',
  carpinteria: 'Carpintería',
  palacio: 'Palacio',
  barracon: 'Barracón',
  galeriaDeTiro: 'Galería de tiro',
  mercado: 'Mercado',
  puestoMercado: 'Puesto de mercado',
  maravilla: 'Maravilla',
  // 'muralla' se retiró de `EdificioTipo` en el Paso 5 de la mecánica de murallas: una muralla ya no es un
  // edificio, es la entidad `Recinto` (perímetro de celdas con puertas, torres y niveles propios). Este
  // catálogo cubre EDIFICIOS, así que no le corresponde una fila — los recintos tendrían que salir de
  // `RECINTO_*` en constants.ts, y de momento nadie ha pedido ese informe.
  plaza: 'Plaza',
  plazaDeArmas: 'Plaza de Armas',
  patioDeGremios: 'Patio de Gremios',
  tallerCarpinteria: 'Taller de carpintería',
  pozo: 'Pozo',
  parque: 'Parque',
};

/**
 * Los siguientes tres conjuntos son privados de `src/engine/construction.ts`; se replican aquí SOLO para el
 * informe. Si cambian allá, actualizar aquí (o exportarlos y borrar esta copia).
 */
const UNICOS_POR_ASENTAMIENTO = new Set<EdificioTipo>([
  'barracon',
  'galeriaDeTiro',
  'palacio',
  'mercado',
  'granFundicion',
  'maravilla',
]);
/** Nacen ya activos por regla de fundación/semilla/satélite — nunca pasan por cola ni se añaden a mano. */
const GRATIS_NO_CONSTRUIBLES = new Set<EdificioTipo>([
  'centroUrbano',
  'puestoMercado',
  'plaza',
  'plazaDeArmas',
  'patioDeGremios',
  'tallerCarpinteria',
  'pozo',
  'parque',
]);
const MAXIMO_TRANSFORMACION_POR_NIVEL: Record<number, number> = { 2: 3, 3: 5, 4: 5, 5: 5 };
const TIPOS_TRANSFORMACION = new Set<EdificioTipo>(['fundicion', 'curtiduria', 'armeria', 'carpinteria']);

type Dim = { ancho: number; alto: number };

function huellaFisica(d: Dim) {
  return { celdas: `${d.ancho}×${d.alto}`, local: `${d.ancho * T}×${d.alto * T}`, celdasArea: d.ancho * d.alto };
}

function fmtRecursos(r: Partial<Record<string, number>> | undefined): string {
  const e = Object.entries(r ?? {});
  return e.length ? e.map(([k, v]) => `${v} ${k}`).join(', ') : '—';
}

interface NivelInfo {
  nivel: number;
  trabajadores: number;
  costoMejora: string;
  gateNivelAsentamiento?: number;
  requiereEdificio?: string;
  requiereEdificioNivel?: number;
  tamano?: ReturnType<typeof huellaFisica>;
  produccionTrigo?: number;
  cupoCaravanas?: number;
  recetas: string[];
}

interface EdificioInfo {
  tipo: EdificioTipo;
  nombre: string;
  costoBase: string;
  tiempoConstruccionMin: number;
  construccion: 'auto+manual' | 'solo manual' | 'gratis (no construible)';
  unicoPorAsentamiento: boolean;
  gateNivelAsentamiento: number;
  gateNivelFaccion?: number;
  rotable: boolean;
  huella: { fija?: ReturnType<typeof huellaFisica>; porNivel?: Record<number, ReturnType<typeof huellaFisica>>; formas?: Record<number, ReturnType<typeof huellaFisica>>; nota?: string };
  capacidades: Record<string, number>;
  topeTransformacionPorNivel?: Record<number, number>;
  niveles?: NivelInfo[];
  notasMercado?: Record<number, number[]>;
}

function recolectar(tipo: EdificioTipo): EdificioInfo {
  const cat = EDIFICIO_CATALOGO[tipo] as Record<string, unknown>;
  const nivelesRaw = cat.niveles as Record<number, Record<string, unknown>> | undefined;

  // Huella
  const huella: EdificioInfo['huella'] = {};
  if (tipo === 'granja' && nivelesRaw) {
    huella.porNivel = {};
    for (const n of Object.keys(nivelesRaw).map(Number)) huella.porNivel[n] = huellaFisica(tamanoEdificio('granja', n));
    huella.nota = 'crece con el nivel interno';
  } else if (tipo === 'puestoMercado') {
    huella.formas = {};
    // `f` sale de las propias claves del objeto, así que la entrada existe siempre.
    for (const f of Object.keys(PUESTO_MERCADO_FORMA).map(Number)) huella.formas[f] = huellaFisica(PUESTO_MERCADO_FORMA[f]!);
    huella.nota = 'forma según nivelInterno (no es progresión)';
  } else {
    const enTabla = EDIFICIO_TAMANO[tipo] as Dim | undefined;
    huella.fija = huellaFisica(enTabla ?? EDIFICIO_TAMANO_POR_DEFECTO);
    if (!enTabla) huella.nota = 'tamaño por defecto (ausente de EDIFICIO_TAMANO)';
  }

  // Capacidades especiales
  const capacidades: Record<string, number> = {};
  for (const k of ['capacidadPesants', 'capacidadArtesanos', 'capacidadPorRecursoAdicional', 'capacidadNobles', 'produccionBaseTrigo', 'produccionBasePiedra', 'produccionBaseMadera', 'produccionBaseOro', 'produccionBaseCobre', 'produccionBaseEstano', 'produccionBaseLivestock', 'trabajadoresRequeridos']) {
    if (typeof cat[k] === 'number') capacidades[k] = cat[k] as number;
  }

  // Niveles internos
  let niveles: NivelInfo[] | undefined;
  if (nivelesRaw) {
    niveles = Object.keys(nivelesRaw)
      .map(Number)
      .sort((a, b) => a - b)
      .map((n) => {
        const lv = nivelesRaw[n]!; // `n` sale de `Object.keys(nivelesRaw)`: la entrada existe.
        const recetas = ((lv.recetas as Array<Record<string, unknown>>) ?? []).map(
          (r) => `${r.produce} ×${r.produccionBase}/min ⟵ ${fmtRecursos(r.consumePorUnidad as Partial<Record<string, number>>)} (por unidad)`
        );
        const info: NivelInfo = {
          nivel: n,
          trabajadores: (lv.trabajadoresRequeridos as number) ?? 0,
          costoMejora: n === 1 ? '— (incluido en costo base)' : fmtRecursos(lv.costoMejora as Partial<Record<string, number>>),
          gateNivelAsentamiento: lv.requisitoNivelAsentamiento as number | undefined,
          requiereEdificio: lv.requiereEdificio as string | undefined,
          requiereEdificioNivel: lv.requiereEdificioNivel as number | undefined,
          produccionTrigo: tipo === 'granja' ? produccionTrigoDeGranja(n) : undefined,
          cupoCaravanas: lv.cupoCaravanas as number | undefined,
          recetas,
        };
        if (tipo === 'granja') info.tamano = huellaFisica(tamanoEdificio('granja', n));
        return info;
      });
  }

  const esGratis = GRATIS_NO_CONSTRUIBLES.has(tipo);
  return {
    tipo,
    nombre: NOMBRE[tipo],
    costoBase: fmtRecursos(cat.costo as Partial<Record<string, number>>),
    tiempoConstruccionMin: (cat.tiempoConstruccionMinutos as number) ?? 0,
    construccion: esGratis ? 'gratis (no construible)' : 'auto+manual',
    unicoPorAsentamiento: UNICOS_POR_ASENTAMIENTO.has(tipo),
    gateNivelAsentamiento: (cat.requisitoNivelAsentamientoConstruccion as number) ?? 0,
    gateNivelFaccion: cat.nivelFaccionMinimo as number | undefined,
    rotable: permiteRotacion(tipo, tamanoEdificio(tipo)),
    huella,
    capacidades,
    topeTransformacionPorNivel: TIPOS_TRANSFORMACION.has(tipo) ? MAXIMO_TRANSFORMACION_POR_NIVEL : undefined,
    niveles,
    notasMercado: tipo === 'mercado' ? MERCADO_PUESTOS_POR_NIVEL : undefined,
  };
}

const CATALOGO: EdificioInfo[] = EDIFICIOS_TIPO.map(recolectar);

// --- Salidas -------------------------------------------------------------------

function imprimirTexto(): void {
  console.log(`\nCATÁLOGO DE EDIFICIOS  (${CATALOGO.length} tipos)`);
  console.log(`rejilla local: 1 celda = ${T} unidades locales  ·  huella = celdas × ${T}\n`);
  for (const e of CATALOGO) {
    console.log('─'.repeat(90));
    console.log(`${e.nombre}  [${e.tipo}]`);
    console.log(`  construcción     : ${e.construccion}${e.unicoPorAsentamiento ? '  ·  único por asentamiento' : ''}`);
    if (e.construccion !== 'gratis (no construible)') {
      console.log(`  costo base       : ${e.costoBase}`);
      console.log(`  tiempo obra      : ${e.tiempoConstruccionMin} min`);
    }
    const gates: string[] = [];
    if (e.gateNivelAsentamiento) gates.push(`nivel asentamiento ≥ ${e.gateNivelAsentamiento}`);
    if (e.gateNivelFaccion) gates.push(`nivel facción ≥ ${e.gateNivelFaccion}`);
    if (gates.length) console.log(`  gate construcción: ${gates.join(', ')}`);
    if (e.huella.fija) console.log(`  huella           : ${e.huella.fija.celdas} celdas  (${e.huella.fija.local} locales)${e.huella.nota ? `  — ${e.huella.nota}` : ''}`);
    if (e.huella.porNivel) {
      console.log(`  huella por nivel : ${e.huella.nota}`);
      for (const [n, h] of Object.entries(e.huella.porNivel)) console.log(`      nivel ${n}: ${h.celdas} celdas  (${h.local} locales)`);
    }
    if (e.huella.formas) {
      console.log(`  huella por forma : ${e.huella.nota}`);
      for (const [f, h] of Object.entries(e.huella.formas)) console.log(`      forma ${f}: ${h.celdas} celdas  (${h.local} locales)`);
    }
    console.log(`  rotable          : ${e.rotable ? 'sí (ancho↔alto)' : 'no'}`);
    if (Object.keys(e.capacidades).length) {
      console.log(`  parámetros       : ${Object.entries(e.capacidades).map(([k, v]) => `${k}=${v}`).join(', ')}`);
    }
    if (e.topeTransformacionPorNivel) {
      console.log(`  tope por nivel   : ${Object.entries(e.topeTransformacionPorNivel).map(([n, v]) => `n${n}:${v}`).join(', ')} (edificios de este tipo por asentamiento)`);
    }
    if (e.notasMercado) {
      console.log(`  puestos zona     : ${Object.entries(e.notasMercado).map(([n, formas]) => `al nivel ${n} +[${formas.join(',')}]`).join('  ·  ')}`);
    }
    if (e.niveles) {
      console.log(`  niveles internos :`);
      for (const lv of e.niveles) {
        const g: string[] = [];
        if (lv.gateNivelAsentamiento) g.push(`asent≥${lv.gateNivelAsentamiento}`);
        if (lv.requiereEdificio) g.push(`requiere ${lv.requiereEdificio}${lv.requiereEdificioNivel ? ` n${lv.requiereEdificioNivel}` : ''}`);
        console.log(`      ─ nivel ${lv.nivel}${g.length ? `  (${g.join(', ')})` : ''}`);
        console.log(`        mejora     : ${lv.costoMejora}`);
        console.log(`        trabajadores: ${lv.trabajadores}`);
        if (lv.tamano) console.log(`        huella     : ${lv.tamano.celdas} celdas  (${lv.tamano.local} locales)`);
        if (lv.produccionTrigo !== undefined) console.log(`        trigo/min  : ${lv.produccionTrigo}`);
        if (lv.cupoCaravanas !== undefined) console.log(`        cupoCaravanas: ${lv.cupoCaravanas}`);
        if (lv.recetas.length) for (const r of lv.recetas) console.log(`        receta     : ${r}`);
      }
    }
    console.log('');
  }
}

function imprimirMarkdown(): void {
  console.log('| Edificio | tipo | Costo base | Obra (min) | Huella celdas | Huella local | Rotable | Único | Gate nivel | Niveles internos |');
  console.log('|---|---|---|---|---|---|---|---|---|---|');
  for (const e of CATALOGO) {
    const h = e.huella.fija
      ? [e.huella.fija.celdas, e.huella.fija.local]
      : e.huella.porNivel
        ? [Object.values(e.huella.porNivel).map((x) => x.celdas).join(' / '), Object.values(e.huella.porNivel).map((x) => x.local).join(' / ')]
        : e.huella.formas
          ? [Object.values(e.huella.formas).map((x) => x.celdas).join(' / '), Object.values(e.huella.formas).map((x) => x.local).join(' / ')]
          : ['—', '—'];
    console.log(
      `| ${e.nombre} | \`${e.tipo}\` | ${e.costoBase} | ${e.tiempoConstruccionMin} | ${h[0]} | ${h[1]} | ${e.rotable ? 'sí' : 'no'} | ${e.unicoPorAsentamiento ? 'sí' : 'no'} | ${e.gateNivelAsentamiento || '—'} | ${e.niveles ? e.niveles.length : '—'} |`
    );
  }
}

/** JSON mínimo (a petición del usuario): solo `{ nombre, ancho, alto }` por edificio, nada más. `ancho`/`alto`
 * van en celdas de la rejilla local (`EDIFICIO_TAMANO`). Los tipos cuya huella varía (Granja por nivel,
 * Puesto de mercado por forma) emiten una fila por variante, con la variante indicada en el nombre. */
/** "4×6" -> [4, 6]. Devuelve una tupla de números de verdad: destructurar `split().map(Number)` daba
 * `number | undefined` en cada posición, y esas dimensiones acaban en el JSON como números. */
function dimensionesDe(celdas: string): [number, number] {
  const [ancho, alto] = celdas.split('×').map(Number);
  return [ancho ?? 0, alto ?? 0];
}

function jsonMinimo(): Array<{ nombre: string; ancho: number; alto: number }> {
  const filas: Array<{ nombre: string; ancho: number; alto: number }> = [];
  for (const e of CATALOGO) {
    if (e.huella.fija) {
      const [ancho, alto] = dimensionesDe(e.huella.fija.celdas);
      filas.push({ nombre: e.nombre, ancho, alto });
    } else if (e.huella.porNivel) {
      for (const [n, h] of Object.entries(e.huella.porNivel)) {
        const [ancho, alto] = dimensionesDe(h.celdas);
        filas.push({ nombre: `${e.nombre} (nivel ${n})`, ancho, alto });
      }
    } else if (e.huella.formas) {
      for (const [f, h] of Object.entries(e.huella.formas)) {
        const [ancho, alto] = dimensionesDe(h.celdas);
        filas.push({ nombre: `${e.nombre} (forma ${f})`, ancho, alto });
      }
    }
  }
  return filas;
}

const modo = process.argv.slice(2);
if (modo.includes('--json')) console.log(JSON.stringify(jsonMinimo(), null, 2));
else if (modo.includes('--md')) imprimirMarkdown();
else imprimirTexto();
