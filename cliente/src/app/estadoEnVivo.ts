// Compone el `EstadoAdmin` que consume la interfaz a partir de lo que llega por tiempo real: la versión LIGERA de la partida (`admin/partida`)
// más el detalle de lo que se está mirando (`admin/asentamiento/<id>`, `admin/campamento/<id>`, `admin/heroe/<id>`), Docs/Coordinacion/03 §2.1.
//
// Lo que el canal no lleva (el interior de las plazas que no se miran, el mercado de los campamentos, y mientras no viajen por WS los
// acuerdos, las órdenes, los Aedas residentes y los precios) se toma de la última lectura HTTP completa (`base`): así la interfaz nunca
// ve un hueco ni un cero falso, y como mucho ve un dato de hace unos segundos en algo que no está a la vista. Las listas globales no
// dependen de ese dato porque leen `resumen`, que sí viaja fresco.
import type { EstadoAdmin } from '@motor/session/estado';
import type { ResumenDeAsentamiento } from '@motor/session/vistas/segmentos';
import type { AlmacenDeCanal } from './almacenDeCanal';
import { canalAsentamiento, canalCampamento, canalHeroe } from './conexionAdmin';

type Asentamiento = EstadoAdmin['asentamientos'][number];
type Campamento = EstadoAdmin['campamentosMercenarios'][number];

/** Un asentamiento con el `resumen` que calcula el servidor; sin él (lectura HTTP completa) se deriva del asentamiento. */
export type AsentamientoConResumen = Asentamiento & { resumen?: ResumenDeAsentamiento };

/** Colecciones y bloques de `admin/partida` que se copian tal cual al estado cuando el canal los lleva. */
const COLECCIONES = ['heroes', 'ejercitos', 'caravanas', 'facciones', 'relaciones', 'batallas', 'campamentosBandidos', 'aedas', 'titulos', 'acuerdos', 'ordenes'] as const;
const BLOQUES = ['tecnologia', 'red', 'estadoMapa', 'mapaId', 'aedasResidentes', 'preciosReferencia'] as const;

export function fusionarEstado(
  base: EstadoAdmin,
  partida: AlmacenDeCanal,
  detalle: (canal: string) => AlmacenDeCanal | undefined,
  intervaloTickMs: number | null | undefined
): EstadoAdmin {
  const nuevo: Record<string, unknown> = { ...base };
  for (const k of COLECCIONES) if (partida.tiene(k)) nuevo[k] = partida.lista(k);
  for (const k of BLOQUES) if (partida.tiene(k)) nuevo[k] = partida.bloque(k);

  const previosA = new Map(base.asentamientos.map((a) => [a.id, a]));
  nuevo['asentamientos'] = partida.lista<AsentamientoConResumen>('asentamientos').map((ligero) => {
    const d = detalle(canalAsentamiento(ligero.id));
    const completo = d?.unica<Asentamiento>('asentamiento');
    if (d && completo) return { ...ligero, ...completo, edificios: d.lista('edificios'), resumen: ligero.resumen };
    const previo = previosA.get(ligero.id);
    if (!previo) return { ...ligero, almacen: {}, recintos: [] };
    // Los edificios de ámbito `mapa` llegan frescos en la versión ligera; el interior se queda como estaba hasta que se abra el detalle.
    const interior = previo.edificios.filter((e) => (e.ambito ?? 'asentamiento') !== 'mapa');
    return { ...previo, ...ligero, edificios: [...interior, ...ligero.edificios] };
  });

  const previosC = new Map(base.campamentosMercenarios.map((c) => [c.id, c]));
  nuevo['campamentosMercenarios'] = partida.lista<Campamento>('campamentosMercenarios').map((ligero) => {
    const completo = detalle(canalCampamento(ligero.id))?.unica<Campamento>('campamento');
    if (completo) return { ...ligero, ...completo };
    const previo = previosC.get(ligero.id);
    return { ...previo, ...ligero, mercado: previo?.mercado ?? {}, fondos: previo?.fondos ?? {} } as Campamento;
  });

  // El historial de un héroe solo viaja en su canal de detalle.
  const historial = { ...base.historialHeroes };
  for (const h of partida.lista<{ id: string }>('heroes')) {
    const d = detalle(canalHeroe(h.id));
    if (d?.tiene('historial')) historial[h.id] = d.lista('historial');
  }
  nuevo['historialHeroes'] = historial;

  nuevo['instante'] = partida.instante;
  nuevo['version'] = partida.version;
  if (intervaloTickMs !== undefined) nuevo['relojDeMundoIntervaloMs'] = intervaloTickMs;
  return nuevo as unknown as EstadoAdmin;
}
