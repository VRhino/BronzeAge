// Chatarra de batalla (Doc 4.2.1, Gran Fundición): el metal del equipo de los soldados caídos. La recoge el vencedor y
// la Gran Fundición la funde en lingote. Es un recurso aparte, no el equipo de recluta, para que Armería y Gran
// Fundición no formen un bucle que quema madera y cuero a cambio de la mitad del metal.
import { CHATARRA, EDIFICIO_CATALOGO, TROPAS_RECLUTABLES, type NivelEdificioTransformacion } from '../constants';
import type { Escuadron } from '../domain/types';

const CHATARRA_DE_LINGOTE: Record<string, string> = {
  lingoteCobre: 'chatarraCobre',
  lingoteBronce: 'chatarraBronce',
  lingoteHierro: 'chatarraHierro',
};

/** Los lingotes que gasta fabricar cada pieza de equipo: lo que piden sus recetas en Armería y Carpintería. */
const LINGOTES_POR_EQUIPO: ReadonlyMap<string, Partial<Record<string, number>>> = new Map(
  [EDIFICIO_CATALOGO.armeria, EDIFICIO_CATALOGO.carpinteria].flatMap((edificio) =>
    Object.values(edificio.niveles as Record<number, NivelEdificioTransformacion>).flatMap((nivel) =>
      nivel.recetas.map((r): [string, Partial<Record<string, number>>] => [r.produce, r.consumePorUnidad])
    )
  )
);

/** Chatarra que deja un soldado de esta tropa al caer. Sin equipo de metal (madera, cuero) no deja nada. */
function chatarraDeSoldado(tropaId: string): Record<string, number> {
  const chatarra: Record<string, number> = {};
  const costoEquipo = TROPAS_RECLUTABLES.find((t) => t.id === tropaId)?.costoEquipo ?? {};
  for (const [equipo, piezas] of Object.entries(costoEquipo)) {
    for (const [lingote, porPieza] of Object.entries(LINGOTES_POR_EQUIPO.get(equipo) ?? {})) {
      const recurso = CHATARRA_DE_LINGOTE[lingote];
      if (recurso && piezas && porPieza) chatarra[recurso] = (chatarra[recurso] ?? 0) + piezas * porPieza * CHATARRA.fraccion;
    }
  }
  return chatarra;
}

/** La chatarra de las bajas de una batalla: cada escuadrón, antes y después. Los escuadrones se cruzan por `id`. */
export function chatarraDeBajas(antes: readonly Escuadron[], despues: readonly Escuadron[]): Record<string, number> {
  const quedan = new Map(despues.map((e) => [e.id, e.cantidad]));
  return sumarChatarra(...antes.map((e) => chatarraDeMuertos(e.tropaId, e.cantidad - (quedan.get(e.id) ?? e.cantidad))));
}

/** La chatarra de `muertos` soldados de una tropa (para resultados que ya traen el número de muertos, como Unity). */
export function chatarraDeMuertos(tropaId: string, muertos: number): Record<string, number> {
  if (muertos <= 0) return {};
  return Object.fromEntries(Object.entries(chatarraDeSoldado(tropaId)).map(([recurso, porSoldado]) => [recurso, porSoldado * muertos]));
}

export function sumarChatarra(...partes: readonly Record<string, number>[]): Record<string, number> {
  const total: Record<string, number> = {};
  for (const parte of partes) for (const [recurso, cantidad] of Object.entries(parte)) total[recurso] = (total[recurso] ?? 0) + cantidad;
  return total;
}
