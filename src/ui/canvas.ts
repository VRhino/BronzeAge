import type { Asentamiento, Caravana, EdificioTipo, Faccion, RecursoTipo, World, ZonaInfluencia } from '../domain/types';

export const FACCION_COLORES = ['#c0392b', '#2980b9', '#27ae60', '#8e44ad', '#d35400', '#16a085'];

const RECURSO_COLOR: Record<RecursoTipo, string> = {
  madera: '#3f7d3a',
  piedra: '#8d8d8d',
  trigo: '#d4b106',
  cobre: '#c0703c',
  estano: '#b8b8d0',
  oro: '#f1c40f',
  livestock: '#e0a458',
};

const EDIFICIO_COLOR: Record<EdificioTipo, string> = {
  vivienda: '#e8e2d0',
  granja: '#d4b106',
  cantera: '#8d8d8d',
  lenera: '#3f7d3a',
  almacen: '#7a5c3a',
  taller: '#c0703c',
  mina: '#f1c40f',
  minaCobre: '#c0703c',
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
