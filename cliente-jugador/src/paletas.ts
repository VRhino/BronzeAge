export const FACCION_COLORES = ['#c0392b', '#2980b9', '#27ae60', '#8e44ad', '#d35400', '#16a085'];

export const RECURSO_COLOR: Record<string, string> = {
  madera: '#6b4226',
  piedra: '#8d8d8d',
  trigo: '#c9a227',
  cobre: '#b5651d',
  estano: '#2f6fd1',
  oro: '#ffd700',
  livestock: '#b5658a',
  lingoteCobre: '#d98a4a',
  lingoteEstano: '#5a8fd9',
  lingoteBronce: '#a97142',
  cuero: '#8a5a3c',
  cueroCurtido: '#6b4226',
  cueroCalidad: '#4a2e18',
  armaMadera: '#7a5c3a',
  armaCobre: '#c98a4a',
  armaBronce: '#a97142',
  armaBronceCalidad: '#8a5a2e',
  armaduraBasica: '#a9a9a9',
  armaduraIntermedia: '#8d8d8d',
  armaduraBronce: '#6b6b6b',
};

export const BIOMA_COLOR: Record<string, string> = {
  agua: '#3a6ea5',
  costa: '#c9b98a',
  estepa: '#b0a15a',
  llanuraFertil: '#5a8f4a',
  colina: '#7a6b4a',
  montana: '#6b6b6b',
  cima: '#e9edf0',
};

const BIOMA_TIERRA_PLANA = '#93c26b';
export const BIOMA_COLOR_SIMPLE: Record<string, string> = {
  agua: BIOMA_COLOR.agua!,
  costa: BIOMA_TIERRA_PLANA,
  estepa: BIOMA_TIERRA_PLANA,
  llanuraFertil: BIOMA_TIERRA_PLANA,
  colina: BIOMA_TIERRA_PLANA,
  montana: BIOMA_COLOR.montana!,
  cima: BIOMA_COLOR.cima!,
};

export const EDIFICIO_COLOR: Record<string, string> = {
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
  corral: '#b5658a',
  curtiduria: '#8a5a3c',
  armeria: '#a83232',
  carpinteria: '#6b4226',
  barracon: '#8b3a3a',
  galeriaDeTiro: '#4a7a4a',
  palacio: '#c9a227',
  mercado: '#2d9c8f',
  puestoMercado: '#7fc9bf',
  maravilla: '#ffd700',
  muralla: '#5a5a5a',
  plaza: '#f0e8c8',
  plazaDeArmas: '#c97a7a',
  patioDeGremios: '#c99a6b',
  tallerCarpinteria: '#a87850',
  pozo: '#d8e4e8',
  parque: '#c8e0b8',
};

export function faccionColor(faccionId: string, facciones: { id: string }[]): string {
  const idx = facciones.findIndex((f) => f.id === faccionId);
  return FACCION_COLORES[Math.max(0, idx) % FACCION_COLORES.length]!;
}
