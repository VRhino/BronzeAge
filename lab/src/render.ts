// Renderer del Laboratorio. Copia deliberada, NO una importación de `cliente/`:
//
// - `cliente/` es un cliente de RED y está previsto que salga a su propio repositorio; el laboratorio
//   ejecuta el motor en el navegador y por eso vive aquí (fue justo la razón de que lo borraran en
//   `73a12dfb`: "no era un cliente y no podía aislarse por red"). Depender de él reintroduciría el problema.
// - Y no hace falta que sean iguales: este dibuja para DEPURAR (rejilla visible, celdas de calle marcadas),
//   no para jugar.
//
// El trazado NO se recalcula aquí: llega ya resuelto de `trazadoParaAsentamiento` (engine/trazado.ts), que
// es el mismo contrato que consume el servidor. Dibujar el laboratorio ejercita ese contrato de verdad.
import type { Asentamiento, EdificioTipo, Point } from '../../src/domain/types';
import type { RectanguloLocal, TrazadoAsentamiento } from '../../src/engine/trazado';
import { edificiosInternos } from '../../src/engine/trazado';
import type { CeldaMuro, TrazadoMuralla, TrazoRecinto } from '../../src/engine/muralla';

export const EDIFICIO_ETIQUETA: Record<EdificioTipo, string> = {
  centroUrbano: 'Centro Urbano',
  vivienda: 'Vivienda',
  granja: 'Granja',
  cantera: 'Cantera',
  lenera: 'Leñera',
  almacen: 'Almacén',
  granero: 'Granero',
  mina: 'Mina (oro)',
  minaCobre: 'Mina (cobre)',
  minaEstano: 'Mina (estaño)',
  fundicion: 'Fundición',
  granFundicion: 'Gran Fundición',
  corral: 'Corral',
  curtiduria: 'Curtiduría',
  armeria: 'Armería',
  carpinteria: 'Carpintería',
  palacio: 'Palacio',
  barracon: 'Barracón',
  galeriaDeTiro: 'Galería de tiro',
  mercado: 'Mercado',
  puestoMercado: 'Puesto de mercado',
  maravilla: 'Maravilla',
  plaza: 'Plaza',
  plazaDeArmas: 'Plaza de Armas',
  patioDeGremios: 'Patio de Gremios',
  pozo: 'Pozo',
  parque: 'Parque',
};

export const EDIFICIO_COLOR: Record<EdificioTipo, string> = {
  centroUrbano: '#9b59b6',
  vivienda: '#e8e2d0',
  granja: '#d4b106',
  cantera: '#8d8d8d',
  lenera: '#3f7d3a',
  almacen: '#7a5c3a',
  granero: '#b8933f',
  mina: '#f1c40f',
  minaCobre: '#c0703c',
  minaEstano: '#2f6fd1',
  fundicion: '#b33a3a',
  granFundicion: '#7a1f1f',
  // Rediseño de progreso (Fase 0, Doc 4.2.1): edificios nuevos.
  corral: '#b5658a',
  curtiduria: '#8a5a3c',
  armeria: '#a83232',
  carpinteria: '#6b4226',
  barracon: '#8b3a3a',
  galeriaDeTiro: '#4a7a4a',
  palacio: '#c9a227',
  mercado: '#2d9c8f',
  // Tono más claro del Mercado a propósito: los puestos son piezas de SU zona, y con la etiqueta de texto
  // retirada del lienzo el color es lo único que agrupa el conjunto a la vista.
  puestoMercado: '#7fc9bf',
  maravilla: '#ffd700',
  // Anclas y satélites, Etapa 3: tonos más claros de sus propias categorías (mismo criterio que puestoMercado
  // frente a mercado) — marcan visualmente que son piezas de zona, no edificios independientes.
  plaza: '#f0e8c8',
  plazaDeArmas: '#c97a7a',
  patioDeGremios: '#c99a6b',
  pozo: '#d8e4e8',
  parque: '#c8e0b8',
};

export interface EstadoDibujoLab {
  asentamiento: Asentamiento;
  trazado: TrazadoAsentamiento;
  /** Radio ESTÁTICO del espacio local — `REJILLA_ASENTAMIENTO.radioMapa`. Fija la escala del lienzo para que
   * el zoom no cambie a medida que la ciudad crece: el espacio se ve fijo y se va llenando. */
  radioMapa: number;
  tamanoCelda: number;
  /** Rejilla de fondo: en el laboratorio interesa VER las celdas, para juzgar huellas y anchos de calle. */
  mostrarRejilla: boolean;
  /** Presupuesto de muralla: el anillo PROPUESTO, todavía sin comprometer. Ausente = no se ha pedido ninguno. */
  murallaPropuesta?: TrazadoMuralla;
  /** El mismo trazo sin fusionar, celda a celda y EN ORDEN DE RECORRIDO — lo que necesita el tooltip para
   * poder decir de una celda concreta qué clase es, qué cuesta y en qué punto de la obra se levantará. */
  murallaTrazo?: TrazoRecinto;
}

/** Escala y traslación de coordenadas locales a píxeles del lienzo. */
export function proyeccion(canvas: HTMLCanvasElement, radioMapa: number): { escala: number; aPantalla: (p: Point) => Point } {
  const lado = Math.min(canvas.width, canvas.height);
  const escala = lado / (radioMapa * 2);
  const aPantalla = (p: Point): Point => ({
    x: canvas.width / 2 + p.x * escala,
    y: canvas.height / 2 + p.y * escala,
  });
  return { escala, aPantalla };
}

/** Pinta el espacio plano del asentamiento: rejilla, calles y caminos como ÁREAS, y cada edificio con su
 * huella real. Nada se decide aquí — todo llega resuelto del motor. */
export function dibujarAsentamientoLab(
  ctx: CanvasRenderingContext2D,
  canvas: HTMLCanvasElement,
  estado: EstadoDibujoLab
): void {
  const { asentamiento, trazado, radioMapa, tamanoCelda, mostrarRejilla, murallaPropuesta } = estado;
  const { escala, aPantalla } = proyeccion(canvas, radioMapa);

  ctx.clearRect(0, 0, canvas.width, canvas.height);
  // Fondo VERDE, el mismo contra el que se eligió la paleta de `EDIFICIO_COLOR`. Un primer intento usó un
  // crema (#efe7d8) y la Vivienda —#e8e2d0, casi el mismo color— desaparecía: parecía que el motor no las
  // construía. Se dibujaban perfectamente; no se veían. De ahí también el contorno de abajo.
  ctx.fillStyle = '#93c26b';
  ctx.fillRect(0, 0, canvas.width, canvas.height);

  if (mostrarRejilla) {
    const paso = tamanoCelda * escala;
    if (paso >= 3) {
      ctx.strokeStyle = 'rgba(0, 0, 0, 0.07)';
      ctx.lineWidth = 1;
      ctx.beginPath();
      const celdas = Math.ceil(radioMapa / tamanoCelda);
      for (let i = -celdas; i <= celdas; i++) {
        const a = aPantalla({ x: i * tamanoCelda, y: -radioMapa });
        const b = aPantalla({ x: i * tamanoCelda, y: radioMapa });
        ctx.moveTo(a.x, a.y);
        ctx.lineTo(b.x, b.y);
        const c = aPantalla({ x: -radioMapa, y: i * tamanoCelda });
        const d = aPantalla({ x: radioMapa, y: i * tamanoCelda });
        ctx.moveTo(c.x, c.y);
        ctx.lineTo(d.x, d.y);
      }
      ctx.stroke();
    }
  }

  // Calles y caminos: SUPERFICIE, no líneas. Desde la Etapa 6 una calle ocupa celdas, y verlas rellenas es
  // justo lo que permite juzgar si el ancho y el trazado tienen sentido.
  const tiradas = (rects: RectanguloLocal[], color: string): void => {
    ctx.fillStyle = color;
    for (const r of rects) {
      const esquina = aPantalla({ x: r.x, y: r.y });
      ctx.fillRect(esquina.x, esquina.y, r.ancho * escala, r.alto * escala);
    }
  };
  tiradas(trazado.caminos, 'rgba(150, 128, 96, 0.55)');
  tiradas(trazado.calles, 'rgba(120, 92, 58, 0.70)');

  // Edificios: Viviendas primero, para que las etiquetas de los singulares queden por encima.
  const internos = edificiosInternos(asentamiento.edificios);
  const orden = [...internos].sort((a, b) => (a.tipo === 'vivienda' ? -1 : 0) - (b.tipo === 'vivienda' ? -1 : 0));
  for (const edificio of orden) {
    const huella = trazado.huellas[edificio.id];
    if (!huella) continue;
    const esquina = aPantalla({ x: huella.x, y: huella.y });
    const ancho = huella.ancho * escala;
    const alto = huella.alto * escala;
    // Margen hacia adentro: los edificios van pared con pared, y sin él no se distingue dónde acaba uno.
    const margen = Math.min(1.5, ancho / 8, alto / 8);
    const x = esquina.x + margen;
    const y = esquina.y + margen;
    const w = ancho - margen * 2;
    const h = alto - margen * 2;
    ctx.fillStyle = EDIFICIO_COLOR[edificio.tipo] ?? '#999';
    ctx.fillRect(x, y, w, h);
    // Contorno SIEMPRE: ningún tipo puede volver a confundirse con el fondo ni con su vecino, que es
    // exactamente el fallo que hizo parecer que las Viviendas no se construían.
    ctx.strokeStyle = edificio.estado === 'activo' ? 'rgba(30, 26, 20, 0.55)' : 'rgba(200, 40, 40, 0.95)';
    ctx.lineWidth = edificio.estado === 'activo' ? 1 : 1.5;
    ctx.strokeRect(x, y, w, h);
  }

  // La muralla va ENCIMA de todo: es lo que hay que juzgar cuando se pide un presupuesto, y a esta escala una
  // celda son pocos píxeles. Se dibuja translúcida y con contorno discontinuo para que se lea como PROPUESTA
  // y no como algo ya construido — mientras no exista la entidad `Recinto`, nada de esto está comprometido.
  {
    const pintar = (rects: RectanguloLocal[], relleno: string, borde: string, guiones: number[]) => {
      ctx.fillStyle = relleno;
      ctx.strokeStyle = borde;
      ctx.lineWidth = 1;
      ctx.setLineDash(guiones);
      for (const r of rects) {
        const esquina = aPantalla({ x: r.x, y: r.y });
        const w = r.ancho * escala;
        const h = r.alto * escala;
        ctx.fillRect(esquina.x, esquina.y, w, h);
        ctx.strokeRect(esquina.x, esquina.y, w, h);
      }
      ctx.setLineDash([]);
    };
    // Recintos YA COMPROMETIDOS: sólidos y con contorno continuo. Su suelo está ocupado de verdad.
    for (const muralla of trazado.murallas) {
      // Obra pendiente: MISMO contorno rojo que un edificio `en_construccion`, porque es literalmente lo
      // mismo — suelo ya comprometido que todavía no está en pie. Si no se dibujara, en su sitio se vería
      // césped vacío que rechaza edificios sin explicación, que es la peor clase de bug: el que parece un
      // bug del motor y en realidad es una omisión del dibujo.
      pintar(muralla.planificado, 'rgba(90, 90, 90, 0.30)', 'rgba(200, 40, 40, 0.95)', []);
      pintar(muralla.muro, 'rgba(90, 90, 90, 0.95)', 'rgba(20, 20, 20, 1)', []);
      // Torre: gris casi negro y opaca — es la lectura visual inmediata del nivel del recinto (§6 del doc).
      pintar(muralla.torres, 'rgba(45, 45, 45, 1)', 'rgba(10, 10, 10, 1)', []);
      // Puerta: ocre, y a propósito el color más llamativo del conjunto. Es el dato irreversible del trazo —
      // cuántas hay decide si el recinto es una fortaleza o una metrópoli (§0).
      pintar(muralla.puertas, 'rgba(214, 158, 46, 1)', 'rgba(120, 84, 10, 1)', []);
    }
    // Presupuesto todavía SIN comprometer: translúcido y punteado, para que no se confunda con lo real.
    if (murallaPropuesta) {
      pintar(murallaPropuesta.muro, 'rgba(90, 90, 90, 0.55)', 'rgba(20, 20, 20, 0.9)', [3, 2]);
      pintar(murallaPropuesta.torres, 'rgba(45, 45, 45, 0.7)', 'rgba(10, 10, 10, 0.9)', [3, 2]);
      pintar(murallaPropuesta.puertas, 'rgba(214, 158, 46, 0.75)', 'rgba(120, 84, 10, 0.9)', [3, 2]);
    }
  }
}

/** El edificio cuya huella contiene un punto de PANTALLA, o `null`. Para inspeccionar con el ratón. */
export function edificioEnPantalla(
  canvas: HTMLCanvasElement,
  estado: EstadoDibujoLab,
  x: number,
  y: number
): { id: string; tipo: EdificioTipo } | null {
  const { escala, aPantalla } = proyeccion(canvas, estado.radioMapa);
  for (const edificio of edificiosInternos(estado.asentamiento.edificios)) {
    const huella = estado.trazado.huellas[edificio.id];
    if (!huella) continue;
    const esquina = aPantalla({ x: huella.x, y: huella.y });
    if (x >= esquina.x && x <= esquina.x + huella.ancho * escala && y >= esquina.y && y <= esquina.y + huella.alto * escala) {
      return { id: edificio.id, tipo: edificio.tipo };
    }
  }
  return null;
}

/** Lo que el tooltip necesita saber del recinto al que pertenece una celda, sin volver a calcular nada. */
export interface ContextoMuralla {
  total: number;
  puertas: number;
  torres: number;
  nivel: number;
  /** Solo tiene sentido en un recinto COMPROMETIDO: si esa celda ya está en pie o sigue siendo obra pendiente. */
  levantada: boolean;
  comprometido: boolean;
  areaEncerrada: number;
  dentro: number;
  fuera: number;
}

/**
 * La celda de muralla bajo un punto de PANTALLA, con su posición en el recorrido de la obra, o `null`.
 * Espejo de `edificioEnPantalla`: el laboratorio existe para inspeccionar, y el anillo era lo único dibujado
 * que no se podía interrogar con el ratón.
 *
 * Mira primero los recintos COMPROMETIDOS y después el presupuesto: si hay muro de verdad, es lo que interesa
 * saber —sobre todo si esa celda está levantada o es obra pendiente, que es la diferencia entre "aquí hay un
 * muro" y "aquí no puedes construir aunque parezca vacío"—.
 */
export function celdaMurallaEnPantalla(
  canvas: HTMLCanvasElement,
  estado: EstadoDibujoLab,
  x: number,
  y: number
): { celda: CeldaMuro; indice: number; contexto: ContextoMuralla } | null {
  const { escala } = proyeccion(canvas, estado.radioMapa);
  const col = Math.floor((x - canvas.width / 2) / escala / estado.tamanoCelda);
  const row = Math.floor((y - canvas.height / 2) / escala / estado.tamanoCelda);

  for (const recinto of estado.asentamiento.recintos ?? []) {
    const indice = recinto.celdas.findIndex((c) => c.col === col && c.row === row);
    if (indice === -1) continue;
    return {
      celda: recinto.celdas[indice]!,
      indice,
      contexto: {
        total: recinto.celdas.length,
        puertas: recinto.celdas.filter((c) => c.clase === 'puerta').length,
        torres: recinto.celdas.filter((c) => c.clase === 'torre').length,
        nivel: recinto.nivel,
        levantada: indice <= recinto.avance,
        comprometido: true,
        areaEncerrada: 0,
        dentro: 0,
        fuera: 0,
      },
    };
  }

  const trazo = estado.murallaTrazo;
  if (!trazo) return null;
  const indice = trazo.celdas.findIndex((c) => c.col === col && c.row === row);
  if (indice === -1) return null;
  return {
    celda: trazo.celdas[indice]!,
    indice,
    contexto: {
      total: trazo.celdas.length,
      puertas: trazo.puertas,
      torres: trazo.torres,
      nivel: estado.murallaPropuesta?.nivel ?? 1,
      levantada: false,
      comprometido: false,
      areaEncerrada: trazo.areaEncerrada,
      dentro: trazo.dentro.length,
      fuera: trazo.fuera.length,
    },
  };
}
