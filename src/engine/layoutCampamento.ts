// La estructura interna de un campamento de mercenarios (Doc 1.9b): sus edificios, calles y empalizada en coordenadas locales (origen en el
// centro, `y` hacia abajo). Reutiliza el trazado de asentamientos (`engine/trazado.ts`) tal cual, con la taberna como ancla principal (el
// papel del Centro Urbano), pero una sola vez: el campamento no crece, no se mantiene, no se degrada ni sube de nivel, así que no hay
// construcción, solo la colocación. Función pura del campamento: no se guarda nada y la misma entidad siempre da lo mismo.
import type { CampamentoMercenarios, Edificio, EdificioCampamentoTipo, EdificioTipo } from '../domain/types';
import { MERCADO_PUESTOS_POR_NIVEL, MERCENARIOS, REJILLA_ASENTAMIENTO } from '../constants';
import {
  anclaActivaParaCategoria,
  CATEGORIA_POR_TIPO,
  crearAnclaNueva,
  fusionarCeldas,
  resolverPerfil,
  redDeCalles,
  sitioParaTipo,
  tipoAnclaParaCategoria,
  trazadoParaAsentamiento,
  type Celda,
  type RectanguloLocal,
} from './trazado';

const T = REJILLA_ASENTAMIENTO.tamanoCelda;

/** Lo que se dibuja en el campamento: sus edificios y, de relleno del trazado, los puestos del mercado y las plazas que hacen de ancla. */
export type ElementoCampamentoTipo = EdificioCampamentoTipo | 'puestoMercado' | 'plazaDeArmas' | 'plaza' | 'pozo' | 'parque';

export interface ElementoCampamento {
  id: string;
  tipo: ElementoCampamentoTipo;
  huella: RectanguloLocal;
}

export interface LayoutCampamento {
  elementos: ElementoCampamento[];
  calles: RectanguloLocal[];
  /** La empalizada que lo rodea (decorativa, sin efecto de juego: dentro no hay combate) y su puerta, al sur. */
  empalizada: { muro: RectanguloLocal[]; puerta: RectanguloLocal[] };
}

/** La taberna hace de Centro Urbano: es lo que siembra la red de calles y a lo que se pegan las viviendas. */
const TIPO_DE_MOTOR = (tipo: EdificioCampamentoTipo): EdificioTipo => (tipo === 'taberna' ? 'centroUrbano' : tipo);
const TIPO_DE_CAMPAMENTO = (tipo: EdificioTipo): ElementoCampamentoTipo => (tipo === 'centroUrbano' ? 'taberna' : (tipo as ElementoCampamentoTipo));

/** Primero lo que ancla (taberna, mercado), luego el militar y por último las viviendas, que se pegan a la taberna. */
const ORDEN: readonly EdificioCampamentoTipo[] = ['taberna', 'mercado', 'barracon', 'galeriaDeTiro', 'caballerizas', 'vivienda'];

export function layoutCampamento(campamento: Pick<CampamentoMercenarios, 'id' | 'edificios'>): LayoutCampamento {
  const { id } = campamento;
  const perfil = resolverPerfil(id);
  const edificios: Edificio[] = [];
  const nuevo = (tipo: EdificioTipo, posicion: Edificio['posicion'], extra: Partial<Edificio> = {}): Edificio => ({
    id: `${id}-${edificios.length}`,
    tipo,
    posicion,
    estado: 'activo',
    ambito: 'asentamiento',
    ...extra,
  });

  // Mismo trabajo que `asegurarAnclaPara` de la construcción, sin el cupo de nivel: el campamento no tiene.
  const asegurarAncla = (tipo: EdificioTipo): void => {
    const categoria = CATEGORIA_POR_TIPO[tipo];
    if (!categoria) return;
    const { instancia, anclasRecienLlenas } = anclaActivaParaCategoria(categoria, tipo, undefined, edificios, redDeCalles(id, edificios), perfil);
    for (const [i, e] of edificios.entries()) if (anclasRecienLlenas.includes(e.id)) edificios[i] = { ...e, anclaLlena: true };
    if (instancia) return;
    const idAncla = `${id}-ancla-${edificios.length}`;
    const tipoAncla = tipoAnclaParaCategoria(categoria, idAncla);
    const resultado = tipoAncla && crearAnclaNueva(id, edificios, tipoAncla, idAncla);
    if (!resultado) return;
    for (const [i, e] of edificios.entries()) if (resultado.anclasRecienSaturadas.includes(e.id)) edificios[i] = { ...e, semillaSaturada: true };
    edificios.push(resultado.nuevaAncla);
  };

  const colocar = (tipo: EdificioTipo, nivelInterno?: number): void => {
    asegurarAncla(tipo);
    const sitio = sitioParaTipo({ id, radioPotencial: MERCENARIOS.layout.radioLocal }, edificios, tipo, nivelInterno, perfil);
    if (sitio) edificios.push(nuevo(tipo, sitio.punto, { ...(sitio.rotado ? { rotado: true } : {}), ...(nivelInterno ? { nivelInterno } : {}) }));
  };

  const taberna = campamento.edificios.includes('taberna');
  if (taberna) edificios.push(nuevo('centroUrbano', { x: 0, y: 0 }));
  for (const tipo of ORDEN.filter((t) => t !== 'taberna')) {
    for (const e of campamento.edificios) {
      if (e !== tipo) continue;
      colocar(TIPO_DE_MOTOR(e));
      // La zona del mercado: sus puestos nacen con él, como en un asentamiento.
      if (e === 'mercado') for (const forma of MERCADO_PUESTOS_POR_NIVEL[MERCENARIOS.layout.nivelMercado] ?? []) colocar('puestoMercado', forma);
    }
  }

  const trazado = trazadoParaAsentamiento({ id, edificios, recintos: [] });
  const elementos = edificios.map((e) => ({ id: e.id, tipo: TIPO_DE_CAMPAMENTO(e.tipo), huella: trazado.huellas[e.id]! }));
  return { elementos, calles: trazado.calles, empalizada: empalizadaAlrededor([...elementos.map((e) => e.huella), ...trazado.calles]) };
}

/** Un anillo de una celda a una celda de margen alrededor de las huellas, con la puerta de dos celdas en el centro del lado sur. */
function empalizadaAlrededor(huellas: readonly RectanguloLocal[]): LayoutCampamento['empalizada'] {
  if (huellas.length === 0) return { muro: [], puerta: [] };
  const minCol = Math.floor(Math.min(...huellas.map((h) => h.x)) / T) - 2;
  const maxCol = Math.ceil(Math.max(...huellas.map((h) => h.x + h.ancho)) / T) + 1;
  const minRow = Math.floor(Math.min(...huellas.map((h) => h.y)) / T) - 2;
  const maxRow = Math.ceil(Math.max(...huellas.map((h) => h.y + h.alto)) / T) + 1;
  const centro = Math.floor((minCol + maxCol) / 2);
  const muro: Celda[] = [];
  const puerta: Celda[] = [];
  for (let col = minCol; col <= maxCol; col++) {
    for (let row = minRow; row <= maxRow; row++) {
      if (col !== minCol && col !== maxCol && row !== minRow && row !== maxRow) continue;
      (row === maxRow && (col === centro || col === centro + 1) ? puerta : muro).push({ col, row });
    }
  }
  return { muro: fusionarCeldas(muro), puerta: fusionarCeldas(puerta) };
}
