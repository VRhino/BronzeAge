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

export const EDIFICIO_ETIQUETA: Record<EdificioTipo, string> = {
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
  curtiduria: 'Curtiduría',
  armeria: 'Armería',
  carpinteria: 'Carpintería',
  palacio: 'Palacio',
  barracon: 'Barracón',
  galeriaDeTiro: 'Galería de tiro',
  mercado: 'Mercado',
  puestoMercado: 'Puesto de mercado',
  maravilla: 'Maravilla',
  muralla: 'Muralla',
  plaza: 'Plaza',
  plazaDeArmas: 'Plaza de Armas',
  patioDeGremios: 'Patio de Gremios',
  tallerCarpinteria: 'Taller de carpintería',
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
  // Muralla (Doc Fase_0_6): gris piedra oscuro, distinto del gris de Cantera para no confundirlos.
  muralla: '#5a5a5a',
  // Anclas y satélites, Etapa 3: tonos más claros de sus propias categorías (mismo criterio que puestoMercado
  // frente a mercado) — marcan visualmente que son piezas de zona, no edificios independientes.
  plaza: '#f0e8c8',
  plazaDeArmas: '#c97a7a',
  patioDeGremios: '#c99a6b',
  tallerCarpinteria: '#a87850',
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
  const { asentamiento, trazado, radioMapa, tamanoCelda, mostrarRejilla } = estado;
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
