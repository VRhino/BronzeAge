import type { Asentamiento, Caravana, EdificioTipo, Faccion, RecursoTipo, World, ZonaInfluencia } from '../domain/types';

export const FACCION_COLORES = ['#c0392b', '#2980b9', '#27ae60', '#8e44ad', '#d35400', '#16a085'];

// Paleta elegida para que cada recurso sea distinguible a simple vista (evitar tonos casi iguales,
// ej. trigo/oro ambos amarillentos) — madera no se dibuja como punto en el mapa (ver Doc 1.4: viene de
// bosques, no de nodos), pero conserva un color propio para paneles de almacén/mercado.
export const RECURSO_COLOR: Record<RecursoTipo, string> = {
  madera: '#6b4226',
  piedra: '#8d8d8d',
  trigo: '#c9a227',
  cobre: '#b5651d',
  estano: '#2f6fd1',
  oro: '#ffd700',
  livestock: '#b5658a',
};

/** Tipos de recurso que SÍ se dibujan como punto en el mapa (para la leyenda) — trigo y madera no lo son. */
export const RECURSOS_EN_MAPA: RecursoTipo[] = ['piedra', 'cobre', 'estano', 'oro', 'livestock'];

export const EDIFICIO_COLOR: Record<EdificioTipo, string> = {
  centroUrbano: '#9b59b6',
  vivienda: '#e8e2d0',
  granja: '#d4b106',
  cantera: '#8d8d8d',
  lenera: '#3f7d3a',
  almacen: '#7a5c3a',
  taller: '#c0703c',
  mina: '#f1c40f',
  minaCobre: '#c0703c',
  minaEstano: '#2f6fd1',
  fundicion: '#b33a3a',
  granFundicion: '#7a1f1f',
};

export function faccionColor(faccionId: string, facciones: Faccion[]): string {
  const idx = facciones.findIndex((f) => f.id === faccionId);
  return FACCION_COLORES[Math.max(0, idx) % FACCION_COLORES.length]!;
}

export interface DrawState {
  world: World;
  asentamientos: Asentamiento[];
  zonas: ZonaInfluencia[];
  facciones: Faccion[];
  caravanas: Caravana[];
}

/**
 * Filtro visual de fertilidad (Doc 1.4/4.2): rejilla semitransparente sobre el mapa, verde más intenso =
 * suelo más fértil. Se dibuja ENCIMA de todo lo demás a propósito (es un "filtro" que se puede apagar),
 * no una capa base — por eso vive aparte de `draw()` y el caller decide si llamarlo.
 */
export function drawFiltroFertilidad(ctx: CanvasRenderingContext2D, canvas: HTMLCanvasElement, world: World): void {
  const scale = canvas.width / world.config.ancho;
  const celdas = 40;
  const tamanoMundo = world.config.ancho;
  const tamanoCelda = tamanoMundo / celdas;

  for (let fila = 0; fila < celdas; fila++) {
    for (let col = 0; col < celdas; col++) {
      const centro = { x: (col + 0.5) * tamanoCelda, y: (fila + 0.5) * tamanoCelda };
      const fertilidad = world.fertilidadEn(centro);
      ctx.fillStyle = `rgba(46, 204, 64, ${fertilidad * 0.45})`;
      ctx.fillRect(col * tamanoCelda * scale, fila * tamanoCelda * scale, tamanoCelda * scale, tamanoCelda * scale);
    }
  }
}

export function draw(ctx: CanvasRenderingContext2D, canvas: HTMLCanvasElement, state: DrawState): void {
  const scale = canvas.width / state.world.config.ancho;
  ctx.clearRect(0, 0, canvas.width, canvas.height);

  // Límites del mapa
  ctx.strokeStyle = '#4a4436';
  ctx.strokeRect(0, 0, canvas.width, canvas.height);

  // Bosques (zonas)
  for (const bosque of state.world.bosques) {
    ctx.beginPath();
    ctx.arc(bosque.centro.x * scale, bosque.centro.y * scale, bosque.radio * scale, 0, Math.PI * 2);
    ctx.fillStyle = `rgba(63, 125, 58, ${0.12 + bosque.densidad * 0.18})`;
    ctx.fill();
  }

  // Nodos de recurso
  for (const nodo of state.world.recursos) {
    ctx.beginPath();
    ctx.arc(nodo.posicion.x * scale, nodo.posicion.y * scale, 3, 0, Math.PI * 2);
    ctx.fillStyle = RECURSO_COLOR[nodo.tipo];
    ctx.fill();
  }

  // Zonas de influencia (polígono recortado por fronteras)
  for (const zona of state.zonas) {
    const asentamiento = state.asentamientos.find((a) => a.id === zona.asentamientoId);
    if (!asentamiento || zona.poligono.length === 0) continue;
    const color = faccionColor(asentamiento.faccionId, state.facciones);
    ctx.beginPath();
    zona.poligono.forEach((p, i) => {
      const x = p.x * scale;
      const y = p.y * scale;
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    });
    ctx.closePath();
    ctx.fillStyle = color + '33';
    ctx.fill();
    ctx.strokeStyle = color;
    ctx.lineWidth = 2;
    ctx.stroke();
  }

  // Edificios (Doc 4.2): cuadrado relleno = activo, semitransparente = en construcción, solo contorno = en cola.
  const tamanoEdificio = 6;
  for (const asentamiento of state.asentamientos) {
    for (const edificio of asentamiento.edificios) {
      const x = edificio.posicion.x * scale;
      const y = edificio.posicion.y * scale;
      const color = EDIFICIO_COLOR[edificio.tipo];
      ctx.lineWidth = 1;
      ctx.strokeStyle = color;
      if (edificio.estado === 'activo') {
        ctx.fillStyle = color;
        ctx.fillRect(x - tamanoEdificio / 2, y - tamanoEdificio / 2, tamanoEdificio, tamanoEdificio);
      } else if (edificio.estado === 'en_construccion') {
        ctx.fillStyle = color + '88';
        ctx.fillRect(x - tamanoEdificio / 2, y - tamanoEdificio / 2, tamanoEdificio, tamanoEdificio);
        ctx.strokeRect(x - tamanoEdificio / 2, y - tamanoEdificio / 2, tamanoEdificio, tamanoEdificio);
      } else {
        ctx.strokeRect(x - tamanoEdificio / 2, y - tamanoEdificio / 2, tamanoEdificio, tamanoEdificio);
      }
    }
  }

  // Asentamientos (punto de fundación)
  for (const asentamiento of state.asentamientos) {
    const color = faccionColor(asentamiento.faccionId, state.facciones);
    ctx.beginPath();
    ctx.arc(asentamiento.posicion.x * scale, asentamiento.posicion.y * scale, 6, 0, Math.PI * 2);
    ctx.fillStyle = color;
    ctx.fill();
    ctx.strokeStyle = '#1b1a17';
    ctx.lineWidth = 1.5;
    ctx.stroke();
  }

  // Caravanas en tránsito (Doc 3.2/3.6): punto moviéndose entre origen y destino.
  for (const caravana of state.caravanas) {
    const x = caravana.posicionActual.x * scale;
    const y = caravana.posicionActual.y * scale;
    ctx.beginPath();
    ctx.arc(x, y, 3.5, 0, Math.PI * 2);
    ctx.fillStyle = '#f1e6c8';
    ctx.fill();
    ctx.strokeStyle = '#1b1a17';
    ctx.lineWidth = 1;
    ctx.stroke();
  }
}
