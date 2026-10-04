// Presencia (Doc 1.10.6, D64-D71): desconectarse quita el control, no el sitio. Dentro de una plaza o de un campamento no se
// mueve nada; con la columna en el mapa, el héroe sale del mundo con ella y reaparece ahí al volver —a un jugador que no está
// no se le puede cazar—. Funciones puras sobre el mundo; cuándo se aplican (2:30 después de pedirlo, aplazado si le
// persiguen, nunca en mitad de una batalla) lo decide la partida (`session/comandos/avanzarTick.ts`).
import type { Asentamiento, Caravana, Ejercito, Escuadron, Faccion, Heroe } from '../domain/types';
import type { Instante } from '../domain/tiempo';
import type { EventoCrudo } from '../domain/eventos';
import { calcularRuta } from '../world/rutas';
import type { Mapa } from '../world/mapa';
import { desgajar } from './ejercitos';
import { conTropa, indiceTropa, sinTropa } from './tropa';
import { esResidente } from './pertenencia';
import { esCiudadano } from './faccion';

export interface MundoPresencia {
  asentamientos: readonly Asentamiento[];
  facciones: readonly Faccion[];
  ejercitos: readonly Ejercito[];
  caravanas: readonly Caravana[];
  heroes: readonly Heroe[];
}

export interface ResultadoPresencia {
  ejercitos: Ejercito[];
  caravanas: Caravana[];
  heroes: Heroe[];
  eventos: EventoCrudo[];
}

const aFuera = (e: Escuadron): Escuadron => ({ ...e, contenedor: { tipo: 'fuera' } });

/**
 * Desconecta a un héroe (Doc 1.10.6):
 *
 * - **Dentro de una plaza o de un campamento** (D64): no se mueve nada. Queda marcado (`fuera`) y no defiende en persona; la
 *   guarnición defiende como siempre.
 * - **En su columna, solo**: la columna deja de existir y él se lleva escuadras y carro. Si llevaba caravanas adjuntas,
 *   vuelven solas a su origen haciendo el camino (D40); sin origen o sin camino por tierra, se pierden.
 * - **En un ejército con más gente**: se separa con lo suyo y, como mucho, un carro (Doc 5.14.2), y el ejército sigue
 *   sin él. Si era el Líder, el mando pasa al CONECTADO de más antigüedad (D67, Doc 5.14.3).
 *
 * La escolta que tenía cedida a una caravana se queda con ella (D40b).
 */
export function salirDelMundo(mundo: MundoPresencia, heroeId: string, mapa: Mapa): ResultadoPresencia {
  const heroe = mundo.heroes.find((h) => h.id === heroeId);
  const sinCambios = { ejercitos: [...mundo.ejercitos], caravanas: [...mundo.caravanas], heroes: [...mundo.heroes], eventos: [] };
  if (!heroe || heroe.fuera) return sinCambios;
  const ubicacion = heroe.ubicacion;

  if (ubicacion.tipo !== 'columna') {
    const marcado: Heroe = { ...heroe, desconectaEn: undefined, fuera: { carro: {} } };
    const plazaId = ubicacion.tipo === 'asentamiento' ? ubicacion.asentamientoId : undefined;
    return { ...sinCambios, heroes: mundo.heroes.map((h) => (h.id === heroeId ? marcado : h)), eventos: [salida(heroe, plazaId)] };
  }

  const original = mundo.ejercitos.find((e) => e.id === ubicacion.ejercitoId);
  if (!original) return sinCambios;
  const columna = conTropa(original, indiceTropa(mundo.heroes));
  let ejercitos = mundo.ejercitos.filter((e) => e.id !== original.id);
  let caravanas = [...mundo.caravanas];
  const eventos: EventoCrudo[] = [];
  let carro: Record<string, number>;

  if (columna.participantes.length > 1) {
    // El mando pasa al conectado de más antigüedad antes de separarse (D67): el Líder no puede irse dejando la columna sin
    // cabeza, ni dársela a otro que tampoco está. Si no queda ninguno conectado, al de más antigüedad.
    const conectado = (id: string) => {
      const h = mundo.heroes.find((x) => x.id === id);
      return !!h && !h.fuera && h.desconectaEn === undefined;
    };
    const porAntiguedad = [...columna.participantes]
      .filter((p) => p.heroeId !== heroeId)
      .sort((a, b) => a.unidoEn - b.unidoEn || (a.heroeId < b.heroeId ? -1 : 1));
    const sucesor = porAntiguedad.find((p) => conectado(p.heroeId)) ?? porAntiguedad[0]!;
    const conMando = columna.liderId === heroeId ? { ...columna, liderId: sucesor.heroeId } : columna;
    const partido = desgajar(conMando, heroeId, `${original.id}-fuera`);
    ejercitos = [...ejercitos, sinTropa(partido.ejercito).ejercito];
    carro = partido.columna.suministro;
  } else {
    carro = columna.suministro;
    // La Caravana de Fundación de un campamento se queda: el tick la ve sin columna y la vuelve a SU campamento (D40, D68).
    for (const caravana of caravanas.filter((c) => columna.caravanasAdjuntasIds.includes(c.id) && !c.titularId)) {
      caravanas = caravanas.filter((c) => c.id !== caravana.id);
      const origen = mundo.asentamientos.find((a) => a.id === caravana.origenAsentamientoId);
      const ruta = origen && calcularRuta(mapa, columna.posicionActual, origen.posicion);
      if (!origen || !ruta) {
        eventos.push({ codigo: 'comercio.caravana_perdida', asentamientoId: caravana.origenAsentamientoId, mensaje: `La caravana ${caravana.id} se queda sin nadie que la lleve y sin camino a casa: se pierde.`, payload: { caravanaId: caravana.id } });
        continue;
      }
      caravanas.push({ ...caravana, estado: 'retornando', destinoAsentamientoId: undefined, ruta, progreso: 0, posicionActual: columna.posicionActual, peajes: undefined });
      eventos.push({ codigo: 'comercio.caravana_vuelve', asentamientoId: origen.id, mensaje: `La caravana ${caravana.id} se queda sin nadie que la lleve y vuelve sola a ${origen.id}.`, payload: { caravanaId: caravana.id } });
    }
  }

  const suyas = new Set(columna.escuadrones.filter((e) => e.heroeId === heroeId).map((e) => e.id));
  const fuera: Heroe = {
    ...heroe,
    desconectaEn: undefined,
    fuera: { carro },
    ubicacion: { tipo: 'desconectado', punto: columna.posicionActual },
    escuadrones: heroe.escuadrones.map((e) => (suyas.has(e.id) ? aFuera(e) : e)),
  };
  eventos.push(salida(heroe, columna.origenAsentamientoId || undefined));
  return { ejercitos, caravanas, heroes: mundo.heroes.map((h) => (h.id === heroeId ? fuera : h)), eventos };
}

function salida(heroe: Heroe, asentamientoId: string | undefined): EventoCrudo {
  return {
    codigo: 'jugador.sale_del_mundo',
    ...(asentamientoId ? { asentamientoId } : {}),
    mensaje: `${heroe.displayName} sale del mundo.`,
    payload: { heroeId: heroe.id },
  };
}

/**
 * Reconecta a un héroe (Doc 1.10.6). Si estaba dentro de una plaza o de un campamento, sigue ahí: solo vuelve a tener el control
 * (D64). Si salió del mundo con su columna, reaparece en su punto con su columna personal, sus escuadras y su carro.
 */
export function volverAlMundo(mundo: MundoPresencia, heroeId: string, ejercitoId: string, instante: Instante): ResultadoPresencia {
  const heroe = mundo.heroes.find((h) => h.id === heroeId);
  const sinCambios = { ejercitos: [...mundo.ejercitos], caravanas: [...mundo.caravanas], heroes: [...mundo.heroes], eventos: [] };
  if (!heroe?.fuera) return sinCambios;
  const vuelta = (h: Heroe): Heroe => ({ ...h, fuera: undefined, desconectaEn: undefined });
  const residencia = mundo.asentamientos.find((a) => esResidente(a, heroeId));
  const evento: EventoCrudo = {
    codigo: 'jugador.vuelve_al_mundo',
    ...(residencia ? { asentamientoId: residencia.id } : {}),
    mensaje: `${heroe.displayName} vuelve al mundo.`,
    payload: { heroeId },
  };
  if (heroe.ubicacion.tipo !== 'desconectado') {
    return { ...sinCambios, heroes: mundo.heroes.map((h) => (h.id === heroeId ? vuelta(h) : h)), eventos: [evento] };
  }
  const { fuera } = heroe;
  const punto = heroe.ubicacion.punto;

  const traidas = heroe.escuadrones.filter((e) => e.contenedor.tipo === 'fuera');
  const columna: Ejercito = {
    id: ejercitoId,
    faccionId: mundo.facciones.find((f) => esCiudadano(f, heroeId))?.id ?? '',
    origenAsentamientoId: residencia?.id ?? '',
    participantes: [{ heroeId, unidoEn: instante }],
    tipo: 'personal',
    liderId: heroeId,
    politicaDeUnion: 'rechazar',
    escuadronIds: traidas.map((e) => e.id),
    suministro: fuera.carro,
    caravanasAdjuntasIds: [],
    objetivo: { tipo: 'punto', punto },
    ruta: [],
    progreso: 0,
    posicionActual: punto,
    estado: 'estacionado',
  };
  const enColumna: Heroe = {
    ...vuelta(heroe),
    ubicacion: { tipo: 'columna', ejercitoId },
    escuadrones: heroe.escuadrones.map((e) => (e.contenedor.tipo === 'fuera' ? { ...e, contenedor: { tipo: 'ejercito', ejercitoId } } : e)),
  };
  return {
    ...sinCambios,
    ejercitos: [...mundo.ejercitos, columna],
    heroes: mundo.heroes.map((h) => (h.id === heroeId ? enColumna : h)),
    eventos: [evento],
  };
}
