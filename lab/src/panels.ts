// Paneles del Laboratorio que no tocan el árbol de anclas: lista de edificios, mejora manual y parámetros
// editables. Todo lee/escribe el motor real — los parámetros se mutan EN SITIO sobre los objetos de
// `constants.ts` (son `const` de binding, no de contenido), así que al refundar el motor los ve cambiados.
import type { Asentamiento, Edificio, EdificioTipo } from '../../src/domain/types';
import { EDIFICIOS_TIPO } from '../../src/domain/types';
import {
  CARPINTERIA_ZONA,
  EDIFICIO_CATALOGO,
  EDIFICIO_TAMANO,
  EDIFICIO_TAMANO_POR_DEFECTO,
  MERCADO_PUESTOS_POR_NIVEL,
  PUESTO_MERCADO_FORMA,
  TRAZADO,
} from '../../src/constants';
import { estadoMejoraEdificio } from '../../src/engine/construction';
import {
  ANCLA_PRIMARIA_POR_CATEGORIA,
  ANCLA_SATURACION_POR_CATEGORIA,
  CATEGORIA_POR_TIPO,
  celdaMinimaDeEdificio,
  edificiosInternos,
  esDeAfueras,
  tamanoDeEdificio,
  type CategoriaAsentamiento,
} from '../../src/engine/trazado';
import { EDIFICIO_ETIQUETA } from './render';

const ENTIDADES: Record<string, string> = { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' };
const esc = (s: string): string => s.replace(/[&<>"]/g, (c) => ENTIDADES[c] ?? c);
const etiqueta = (tipo: string): string => EDIFICIO_ETIQUETA[tipo as EdificioTipo] ?? tipo;

// --- Feature 2: todos los edificios del asentamiento con su número ---

export function renderPanelEdificios(bodyEl: HTMLElement, resumenEl: HTMLElement, asentamiento: Asentamiento): void {
  const internos = edificiosInternos(asentamiento.edificios);
  const externos = asentamiento.edificios.filter((e) => (e.ambito ?? 'asentamiento') === 'mapa');

  const cuenta = new Map<string, number>();
  for (const e of asentamiento.edificios) cuenta.set(e.tipo, (cuenta.get(e.tipo) ?? 0) + 1);
  resumenEl.innerHTML = [...cuenta.entries()]
    .sort((a, b) => b[1] - a[1])
    .map(([tipo, n]) => `<span style="margin-right:10px">${esc(etiqueta(tipo))}: <b>${n}</b></span>`)
    .join('');

  const fila = (e: Edificio, i: number): string => {
    const min = celdaMinimaDeEdificio(e);
    const tam = tamanoDeEdificio(e);
    const marcaMapa = (e.ambito ?? 'asentamiento') === 'mapa' ? ' <span style="color:#a89f88">(mapa)</span>' : '';
    return (
      `<tr data-id="${e.id}">` +
      `<td>${i + 1}</td>` +
      `<td>${esc(etiqueta(e.tipo))}${marcaMapa}</td>` +
      `<td>${e.nivelInterno ?? 1}</td>` +
      `<td>${e.estado}</td>` +
      `<td>${tam.ancho}&times;${tam.alto}</td>` +
      `<td>(${min.col}, ${min.row})</td>` +
      `<td style="color:#a89f88">${e.id.slice(-12)}</td>` +
      `</tr>`
    );
  };

  bodyEl.innerHTML = [...internos, ...externos].map(fila).join('');
}

// --- Feature 3: mejora manual de los edificios que pueden subir de nivel ---

export function renderPanelMejoras(
  bodyEl: HTMLElement,
  asentamiento: Asentamiento,
  onMejorar: (edificioId: string) => void
): void {
  const mejorables = edificiosInternos(asentamiento.edificios)
    .filter((e) => e.estado === 'activo')
    .map((e) => ({ e, mejora: estadoMejoraEdificio(asentamiento, e, undefined) }))
    .filter((x): x is { e: Edificio; mejora: NonNullable<typeof x.mejora> } => x.mejora !== null);

  if (mejorables.length === 0) {
    bodyEl.innerHTML =
      '<tr><td colspan="5" style="color:#a89f88">Ningún edificio activo tiene una mejora disponible ahora mismo.</td></tr>';
    return;
  }

  bodyEl.innerHTML = mejorables
    .map(({ e, mejora }) => {
      const costo = Object.entries(mejora.costo).map(([r, c]) => `${c} ${r}`).join(', ') || '—';
      const accion = mejora.elegible
        ? `<button data-mejorar="${e.id}">Subir a ${mejora.nivelSiguiente}</button>`
        : `<span style="color:#c88" title="${esc(mejora.motivoBloqueo ?? '')}">bloqueada</span>`;
      return (
        '<tr>' +
        `<td>${esc(etiqueta(e.tipo))}</td>` +
        `<td>${mejora.nivelActual} &rarr; ${mejora.nivelSiguiente}</td>` +
        `<td>${esc(costo)}</td>` +
        `<td>${accion}</td>` +
        `<td style="color:#a89f88">${e.id.slice(-12)}</td>` +
        '</tr>'
      );
    })
    .join('');

  bodyEl.querySelectorAll<HTMLButtonElement>('button[data-mejorar]').forEach((b) => {
    b.addEventListener('click', () => onMejorar(b.dataset.mejorar!));
  });
}

// --- Feature 4: parámetros editables (mutan constants.ts en sitio) ---

interface Snapshot {
  tamano: Record<string, { ancho: number; alto: number }>;
  porDefecto: { ancho: number; alto: number };
  granja: Record<number, { ancho: number; alto: number }>;
  formaPuesto: Record<number, { ancho: number; alto: number }>;
  puestosPorNivel: Record<number, number[]>;
  talleres: number;
  trazado: Record<string, number>;
}

function granjaNiveles(): Record<number, { tamano?: { ancho: number; alto: number } }> {
  return EDIFICIO_CATALOGO.granja.niveles as Record<number, { tamano?: { ancho: number; alto: number } }>;
}

function tomarSnapshot(): Snapshot {
  const gn = granjaNiveles();
  return {
    tamano: JSON.parse(JSON.stringify(EDIFICIO_TAMANO)),
    porDefecto: { ...EDIFICIO_TAMANO_POR_DEFECTO },
    granja: { 1: { ...gn[1]!.tamano! }, 2: { ...gn[2]!.tamano! }, 3: { ...gn[3]!.tamano! }, 4: { ...gn[4]!.tamano! } },
    formaPuesto: JSON.parse(JSON.stringify(PUESTO_MERCADO_FORMA)),
    puestosPorNivel: JSON.parse(JSON.stringify(MERCADO_PUESTOS_POR_NIVEL)),
    talleres: CARPINTERIA_ZONA.talleres,
    trazado: JSON.parse(JSON.stringify(TRAZADO)),
  };
}

function restaurar(s: Snapshot): void {
  for (const k of Object.keys(EDIFICIO_TAMANO)) delete EDIFICIO_TAMANO[k];
  for (const [k, v] of Object.entries(s.tamano)) EDIFICIO_TAMANO[k] = { ...v };
  EDIFICIO_TAMANO_POR_DEFECTO.ancho = s.porDefecto.ancho;
  EDIFICIO_TAMANO_POR_DEFECTO.alto = s.porDefecto.alto;
  const gn = granjaNiveles();
  for (const n of [1, 2, 3, 4]) {
    gn[n]!.tamano!.ancho = s.granja[n]!.ancho;
    gn[n]!.tamano!.alto = s.granja[n]!.alto;
  }
  for (const [k, v] of Object.entries(s.formaPuesto)) {
    PUESTO_MERCADO_FORMA[Number(k)]!.ancho = v.ancho;
    PUESTO_MERCADO_FORMA[Number(k)]!.alto = v.alto;
  }
  for (const [k, v] of Object.entries(s.puestosPorNivel)) MERCADO_PUESTOS_POR_NIVEL[Number(k)] = [...v];
  CARPINTERIA_ZONA.talleres = s.talleres;
  for (const [k, v] of Object.entries(s.trazado)) (TRAZADO as unknown as Record<string, number>)[k] = v;
}

/** Buena práctica del laboratorio: un parámetro editado se marca en amarillo (clase `lab-cambiado`) mientras
 * su valor difiera del que tenía constants.ts al abrir la pestaña — así se localizan de un vistazo los que se
 * han tocado, aunque haya que refundar para verlos surtir efecto. */
function marcarCambio(inp: HTMLInputElement, cambiado: boolean): void {
  inp.classList.toggle('lab-cambiado', cambiado);
}

/** input numérico que muta `obj[clave]` al cambiar (entero >= `minimo`). `base` = valor original: si se pasa,
 * el input se pinta de amarillo cuando el valor actual difiere de él. */
function inputNum(obj: Record<string, number>, clave: string, ancho = 52, minimo = 1, base?: number): HTMLInputElement {
  const inp = document.createElement('input');
  inp.type = 'number';
  inp.value = String(obj[clave]);
  inp.style.width = `${ancho}px`;
  if (base !== undefined) marcarCambio(inp, obj[clave] !== base);
  inp.addEventListener('change', () => {
    const v = Number(inp.value);
    if (Number.isFinite(v) && v >= minimo) obj[clave] = Math.round(v);
    inp.value = String(obj[clave]);
    if (base !== undefined) marcarCambio(inp, obj[clave] !== base);
  });
  return inp;
}

function celdaAnchoAlto(obj: { ancho: number; alto: number }, base?: { ancho: number; alto: number }): HTMLTableCellElement {
  const td = document.createElement('td');
  td.append(
    inputNum(obj as unknown as Record<string, number>, 'ancho', 52, 1, base?.ancho),
    document.createTextNode(' × '),
    inputNum(obj as unknown as Record<string, number>, 'alto', 52, 1, base?.alto)
  );
  return td;
}

function wrap(t: HTMLTableElement): HTMLElement {
  const d = document.createElement('div');
  d.className = 'lab-anclas-wrap';
  d.appendChild(t);
  return d;
}

/** Envuelve `contenido` en un `<details>` colapsable con `titulo`. `abierta` fija el estado inicial. */
function seccion(titulo: string, abierta: boolean, ...contenido: Node[]): HTMLDetailsElement {
  const d = document.createElement('details');
  d.open = abierta;
  d.style.marginTop = '8px';
  const sum = document.createElement('summary');
  sum.textContent = titulo;
  sum.style.cssText = 'cursor:pointer;font-weight:600;font-size:14px;padding:2px 0';
  d.append(sum, ...contenido);
  return d;
}

/** fila «etiqueta : input» para un parámetro numérico suelto de un objeto. `base` = valor original (amarillo
 * si difiere). */
function filaParam(
  obj: Record<string, number>,
  clave: string,
  etiquetaTxt: string,
  ayuda: string,
  minimo = 1,
  base?: number
): HTMLElement {
  const tr = document.createElement('tr');
  const tdL = document.createElement('td');
  tdL.textContent = etiquetaTxt;
  tdL.title = ayuda;
  const tdI = document.createElement('td');
  tdI.appendChild(inputNum(obj, clave, 60, minimo, base));
  tr.append(tdL, tdI);
  return tr;
}

/**
 * Categoría de ANCLA con la que se agrupa un tipo en la tabla de dimensiones. No es exactamente
 * `CATEGORIA_POR_TIPO`: los tipos de ancla (Centro Urbano, Plaza, Plaza de Armas, Patio de Gremios...) no
 * figuran ahí, y `esDeAfueras` manda sobre la categoría para Granja/Corral. `'otros'` = sin ancla (Palacio,
 * extractores del mapa).
 */
type GrupoDim = CategoriaAsentamiento | 'afueras' | 'otros';

const CATEGORIA_DE_ANCLA: Partial<Record<string, CategoriaAsentamiento>> = {};
for (const [cat, tipo] of Object.entries(ANCLA_PRIMARIA_POR_CATEGORIA)) CATEGORIA_DE_ANCLA[tipo!] = cat as CategoriaAsentamiento;
for (const [cat, tipos] of Object.entries(ANCLA_SATURACION_POR_CATEGORIA)) {
  for (const tipo of tipos ?? []) CATEGORIA_DE_ANCLA[tipo] = cat as CategoriaAsentamiento;
}

function grupoDeDimension(tipo: string): GrupoDim {
  if (esDeAfueras(tipo as never)) return 'afueras';
  return CATEGORIA_DE_ANCLA[tipo] ?? CATEGORIA_POR_TIPO[tipo as never] ?? 'otros';
}

/** Rol del tipo dentro de su grupo: ancla primaria, ancla de saturación, o satélite normal. */
function rolAncla(tipo: string): '' | ' · ancla primaria' | ' · ancla' {
  if (Object.values(ANCLA_PRIMARIA_POR_CATEGORIA).includes(tipo as never)) return ' · ancla primaria';
  for (const tipos of Object.values(ANCLA_SATURACION_POR_CATEGORIA)) if ((tipos ?? []).includes(tipo as never)) return ' · ancla';
  return '';
}

const GRUPOS_DIM: { clave: GrupoDim; etiqueta: string }[] = [
  { clave: 'residencial', etiqueta: 'Residencial · ancla Centro Urbano / Plaza-Pozo-Parque' },
  { clave: 'mercado', etiqueta: 'Mercado · ancla Mercado' },
  { clave: 'militar', etiqueta: 'Militar · ancla Plaza de Armas' },
  { clave: 'carpinteria', etiqueta: 'Carpintería · ancla Carpintería (sub-zona militar)' },
  { clave: 'industria', etiqueta: 'Industria · ancla Patio de Gremios' },
  { clave: 'almacenaje', etiqueta: 'Almacenaje · sin ancla, regla genérica' },
  { clave: 'afueras', etiqueta: 'Afueras · sin ancla, van al borde de la ciudad' },
  { clave: 'otros', etiqueta: 'Sin ancla · Palacio y extractores del mapa' },
];

/**
 * Monta la pestaña de parámetros DENTRO de `contenedor`. Los inputs mutan los objetos de `constants.ts` en el
 * acto; nada surte efecto hasta refundar, así que "Aplicar y refundar" llama a `onRefundar`.
 */
export function montarPanelParametros(contenedor: HTMLElement, onRefundar: () => void): void {
  const snap = tomarSnapshot();
  contenedor.innerHTML = '';

  const hint = document.createElement('p');
  hint.className = 'hint';
  hint.textContent =
    'Editan las tablas reales de constants.ts en caliente. Nada cambia hasta refundar: el asentamiento actual se construyó con los valores viejos. Los campos que hayas tocado quedan marcados en amarillo.';
  contenedor.appendChild(hint);

  const barra = document.createElement('div');
  barra.className = 'lab-row';
  const btnAplicar = document.createElement('button');
  btnAplicar.textContent = 'Aplicar y refundar';
  btnAplicar.addEventListener('click', onRefundar);
  const btnRestaurar = document.createElement('button');
  btnRestaurar.textContent = 'Restaurar valores';
  btnRestaurar.addEventListener('click', () => {
    restaurar(snap);
    montarPanelParametros(contenedor, onRefundar);
    onRefundar();
  });
  barra.append(btnAplicar, btnRestaurar);
  contenedor.appendChild(barra);

  // --- Sección: dimensiones de edificios, agrupadas por categoría de ancla asociada ---
  {
    const t1 = document.createElement('table');
    t1.className = 'lab-anclas';
    t1.innerHTML = '<thead><tr><th>Edificio</th><th>Ancho &times; Alto</th></tr></thead>';
    const b1 = document.createElement('tbody');

    const subheader = (texto: string): void => {
      const tr = document.createElement('tr');
      const td = document.createElement('td');
      td.colSpan = 2;
      td.textContent = texto;
      td.style.cssText = 'background:#302c24;font-weight:600;color:#d8cfb4;padding-top:5px;padding-bottom:5px';
      tr.appendChild(td);
      b1.appendChild(tr);
    };
    const filaTam = (nombre: string, obj: { ancho: number; alto: number }, base?: { ancho: number; alto: number }): void => {
      const tr = document.createElement('tr');
      const td = document.createElement('td');
      td.textContent = nombre;
      tr.append(td, celdaAnchoAlto(obj, base));
      b1.appendChild(tr);
    };

    // Reparto de cada tipo con entrada propia en EDIFICIO_TAMANO a su grupo; anclas primero dentro del grupo.
    const porGrupo = new Map<GrupoDim, string[]>();
    for (const tipo of Object.keys(EDIFICIO_TAMANO)) {
      const g = grupoDeDimension(tipo);
      if (!porGrupo.has(g)) porGrupo.set(g, []);
      porGrupo.get(g)!.push(tipo);
    }
    const ordenar = (tipos: string[]): string[] =>
      [...tipos].sort((a, b) => (rolAncla(b) ? 1 : 0) - (rolAncla(a) ? 1 : 0) || etiqueta(a).localeCompare(etiqueta(b)));

    // Tipos que no tienen entrada propia (comparten EDIFICIO_TAMANO_POR_DEFECTO), por grupo — solo para
    // ANOTAR bajo el grupo; el input es uno solo, en "Otros".
    const sinEntrada = EDIFICIOS_TIPO.filter((t) => !(t in EDIFICIO_TAMANO) && t !== 'granja' && t !== 'puestoMercado');
    const porDefectoPorGrupo = new Map<GrupoDim, string[]>();
    for (const t of sinEntrada) {
      const g = grupoDeDimension(t);
      if (!porDefectoPorGrupo.has(g)) porDefectoPorGrupo.set(g, []);
      porDefectoPorGrupo.get(g)!.push(etiqueta(t));
    }

    for (const { clave, etiqueta: labelGrupo } of GRUPOS_DIM) {
      const tipos = ordenar(porGrupo.get(clave) ?? []);
      const compartenDefecto = porDefectoPorGrupo.get(clave) ?? [];
      const esAfueras = clave === 'afueras';
      if (tipos.length === 0 && compartenDefecto.length === 0 && !esAfueras) continue;

      subheader(labelGrupo);
      for (const tipo of tipos) filaTam(etiqueta(tipo) + rolAncla(tipo), EDIFICIO_TAMANO[tipo]!, snap.tamano[tipo]);

      // Granja: cuatro tamaños por nivel — va en Afueras.
      if (esAfueras) {
        const gn = granjaNiveles();
        for (const n of [1, 2, 3, 4]) filaTam(`Granja nivel ${n}`, gn[n]!.tamano!, snap.granja[n]);
      }

      // Los tipos sin entrada propia comparten un único input, en la sección final. Aquí solo se anotan.
      if (compartenDefecto.length > 0) {
        const tr = document.createElement('tr');
        const td = document.createElement('td');
        td.colSpan = 2;
        td.style.cssText = 'color:#a89f88;font-size:10px;padding-left:14px';
        td.textContent = `+ tamaño por defecto: ${compartenDefecto.join(', ')}`;
        tr.appendChild(td);
        b1.appendChild(tr);
      }
    }

    // Fila única del tamaño por defecto, al final.
    subheader('Tamaño por defecto (compartido)');
    {
      const tr = document.createElement('tr');
      const td = document.createElement('td');
      td.innerHTML = `<i>por defecto</i><br><span style="color:#a89f88;font-size:10px">Vivienda, ${sinEntrada
        .filter((t) => t !== 'vivienda')
        .map((t) => etiqueta(t))
        .join(', ')}</span>`;
      tr.append(td, celdaAnchoAlto(EDIFICIO_TAMANO_POR_DEFECTO, snap.porDefecto));
      b1.appendChild(tr);
    }

    t1.appendChild(b1);
    contenedor.appendChild(seccion('Dimensiones de edificios (celdas)', false, wrap(t1)));
  }

  // --- Sección: trazado y anclas ---
  {
    const tr = document.createElement('table');
    tr.className = 'lab-anclas';
    tr.innerHTML = '<thead><tr><th>Parámetro</th><th>Valor</th></tr></thead>';
    const body = document.createElement('tbody');
    const T = TRAZADO as unknown as Record<string, number>;
    body.append(
      filaParam(T, 'separacionMinimaAnclas', 'Separación mínima entre anclas', 'Celdas centro a centro. También es el radio inicial y (×3) el máximo de la búsqueda de ranura del árbol.', 1, snap.trazado['separacionMinimaAnclas']),
      filaParam(T, 'separacionSeguridadAnclas', 'Zona de seguridad entre anclas', 'Piso DURO no relajable: ninguna ancla real nueva puede quedar más cerca de otra que esto.', 1, snap.trazado['separacionSeguridadAnclas']),
      filaParam(T, 'anchoCalle', 'Ancho de calle', 'Celdas de grosor de una calle. 1 = media Vivienda. Es lo que hace que la calle cueste suelo.', 1, snap.trazado['anchoCalle']),
      filaParam(T, 'capCorredorUrbano', 'Cap de corredor (urbano)', 'Celdas máximas que un edificio urbano abre para llegar a la red. Corto = más pegado a las calles que ya existen.', 1, snap.trazado['capCorredorUrbano']),
      filaParam(T, 'capCorredorAfueras', 'Cap de corredor (afueras)', 'Igual pero para Granja/Corral, cuyo camino es largo a propósito.', 1, snap.trazado['capCorredorAfueras']),
      filaParam(T, 'radioAfuerasMin', 'Radio vedado de afueras', 'Unidades locales. Ninguna celda de Granja/Corral entra de aquí para dentro.', 1, snap.trazado['radioAfuerasMin']),
      filaParam(T, 'anchoBandaAfueras', 'Ancho de la banda de afueras', 'Unidades locales. La banda donde caben Granja y Corral empieza en el radio vedado y llega al menos hasta radio + esto.', 1, snap.trazado['anchoBandaAfueras']),
      filaParam(T, 'largoFilaMin', 'Largo de fila mínimo', 'Celdas. Cada asentamiento sortea su ancho de manzana entre este y el máximo.', 1, snap.trazado['largoFilaMin']),
      filaParam(T, 'largoFilaMax', 'Largo de fila máximo', 'Celdas. Ver arriba.', 1, snap.trazado['largoFilaMax'])
    );
    tr.appendChild(body);
    contenedor.appendChild(seccion('Trazado y anclas', true, wrap(tr)));
  }

  // --- Sección: piezas dependientes ---
  {
    const hMerc = document.createElement('p');
    hMerc.className = 'hint';
    hMerc.textContent =
      'Mercado: la pieza principal es un edificio real; los puestos nacen gratis. Cada puesto tiene una de las 3 formas de abajo. La lista por nivel es cuántos puestos de cada forma se añaden al alcanzar ese nivel interno.';

    const t2 = document.createElement('table');
    t2.className = 'lab-anclas';
    t2.innerHTML = '<thead><tr><th>Forma de puesto</th><th>Ancho &times; Alto</th></tr></thead>';
    const b2 = document.createElement('tbody');
    for (const f of [1, 2, 3]) {
      const row = document.createElement('tr');
      const td = document.createElement('td');
      td.textContent = `Forma ${f}`;
      row.append(td, celdaAnchoAlto(PUESTO_MERCADO_FORMA[f]!, snap.formaPuesto[f]));
      b2.appendChild(row);
    }
    t2.appendChild(b2);

    const t3 = document.createElement('table');
    t3.className = 'lab-anclas';
    t3.innerHTML = '<thead><tr><th>Nivel de Mercado</th><th>Puestos que se añaden (forma: cantidad)</th></tr></thead>';
    const b3 = document.createElement('tbody');
    for (const nivel of [1, 2, 3]) {
      const row = document.createElement('tr');
      const tdN = document.createElement('td');
      tdN.textContent = `Nivel ${nivel}`;
      const tdC = document.createElement('td');
      const cuentaFormas = (l: number[]): Record<'1' | '2' | '3', number> => ({
        '1': l.filter((x) => x === 1).length,
        '2': l.filter((x) => x === 2).length,
        '3': l.filter((x) => x === 3).length,
      });
      const contador = cuentaFormas(MERCADO_PUESTOS_POR_NIVEL[nivel] ?? []);
      const original = cuentaFormas(snap.puestosPorNivel[nivel] ?? []);
      const sync = (): void => {
        MERCADO_PUESTOS_POR_NIVEL[nivel] = [
          ...Array<number>(contador['1']).fill(1),
          ...Array<number>(contador['2']).fill(2),
          ...Array<number>(contador['3']).fill(3),
        ];
      };
      for (const f of ['1', '2', '3'] as const) {
        const inp = document.createElement('input');
        inp.type = 'number';
        inp.value = String(contador[f]);
        inp.style.width = '46px';
        inp.title = `puestos de forma ${f}`;
        marcarCambio(inp, contador[f] !== original[f]);
        inp.addEventListener('change', () => {
          const v = Number(inp.value);
          contador[f] = Number.isFinite(v) && v >= 0 ? Math.round(v) : 0;
          inp.value = String(contador[f]);
          marcarCambio(inp, contador[f] !== original[f]);
          sync();
        });
        tdC.append(document.createTextNode(` ${f}: `), inp);
      }
      row.append(tdN, tdC);
      b3.appendChild(row);
    }
    t3.appendChild(b3);

    const carp = document.createElement('div');
    carp.className = 'lab-row';
    carp.style.marginTop = '4px';
    carp.append(
      document.createTextNode('Carpintería · talleres dependientes: '),
      inputNum(CARPINTERIA_ZONA as unknown as Record<string, number>, 'talleres', 60, 0, snap.talleres)
    );

    contenedor.appendChild(seccion('Piezas dependientes', false, hMerc, wrap(t2), wrap(t3), carp));
  }
}
