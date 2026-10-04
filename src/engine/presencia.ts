// Presencia (Doc 1.10.6, D33/D40/D40b): al desconectarse, el héroe sale del mundo en el punto donde quedó, con su
// tropa y su carro, y reaparece ahí al volver. Funciones puras sobre el mundo; cuándo se aplican (2:30 después de
// pedirlo, nunca en mitad de una batalla) lo decide la partida (`session/comandos/avanzarTick.ts`).
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
 * Saca del mundo a un héroe (Doc 1.10.6):
 *
 * - **Dentro de una plaza**: sale él con las escuadras libres de su campamento; la guarnición se queda (D40b).
 * - **En su columna, solo**: la columna deja de existir y él se lleva escuadras y carro. Si llevaba caravanas adjuntas,
 *   vuelven solas a su origen haciendo el camino (D40); sin origen o sin camino por tierra, se pierden.
 * - **En un ejército con más gente**: se separa con lo suyo y, como mucho, un carro (Doc 5.14.2), y el ejército sigue
 *   sin él. Si era el Líder, el mando pasa al de más antigüedad (Doc 5.14.3).
 *
 * La escolta que tenía cedida a una caravana se queda con ella (D40b).
 */
export function salirDelMundo(mundo: MundoPresencia, heroeId: string, mapa: Mapa): ResultadoPresencia {
  const heroe = mundo.heroes.find((h) => h.id === heroeId);
  const sinCambios = { ejercitos: [...mundo.ejercitos], caravanas: [...mundo.caravanas], heroes: [...mundo.heroes], eventos: [] };
  if (!heroe || heroe.fuera) return sinCambios;
  const ubicacion = heroe.ubicacion;

  if (ubicacion.tipo !== 'columna') {
    const plaza = ubicacion.tipo === 'asentamiento' ? mundo.asentamientos.find((a) => a.id === ubicacion.asentamientoId) : undefined;
    const fuera: Heroe = {
      ...heroe,
      desconectaEn: undefined,
      fuera: { ...(plaza ? { asentamientoId: plaza.id } : {}), carro: {} },
      ubicacion: { tipo: 'desconectado', punto: plaza?.posicion ?? (ubicacion.tipo === 'desconectado' ? ubicacion.punto : { x: 0, y: 0 }) },
      escuadrones: heroe.escuadrones.map((e) => (e.contenedor.tipo === 'campamento' && !e.enGuarnicion ? aFuera(e) : e)),
    };
    return { ...sinCambios, heroes: mundo.heroes.map((h) => (h.id === heroeId ? fuera : h)), eventos: [salida(heroe, plaza?.id)] };
  }

  const original = mundo.ejercitos.find((e) => e.id === ubicacion.ejercitoId);
  if (!original) return sinCambios;
  const columna = conTropa(original, indiceTropa(mundo.heroes));
  let ejercitos = mundo.ejercitos.filter((e) => e.id !== original.id);
  let caravanas = [...mundo.caravanas];
  const eventos: EventoCrudo[] = [];
  let carro: Record<string, number>;

  if (columna.participantes.length > 1) {
    // El mando pasa al de más antigüedad antes de separarse: el Líder no puede irse dejando la columna sin cabeza.
    const sucesor = [...columna.participantes]
      .filter((p) => p.heroeId !== heroeId)
      .sort((a, b) => a.unidoEn - b.unidoEn || (a.heroeId < b.heroeId ? -1 : 1))[0]!;
    const conMando = columna.liderId === heroeId ? { ...columna, liderId: sucesor.heroeId } : columna;
    const partido = desgajar(conMando, heroeId, `${original.id}-fuera`);
    ejercitos = [...ejercitos, sinTropa(partido.ejercito).ejercito];
    carro = partido.columna.suministro;
  } else {
    carro = columna.suministro;
    for (const caravana of caravanas.filter((c) => columna.caravanasAdjuntasIds.includes(c.id))) {
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
 * Devuelve al mundo a un héroe que estaba fuera, donde quedó (Doc 1.10.6): dentro de la plaza de la que salió si sigue
 * residiendo en ella, y si no, en su punto con su columna personal, sus escuadras y su carro.
 */
export function volverAlMundo(mundo: MundoPresencia, heroeId: string, ejercitoId: string, instante: Instante): ResultadoPresencia {
  const heroe = mundo.heroes.find((h) => h.id === heroeId);
  const sinCambios = { ejercitos: [...mundo.ejercitos], caravanas: [...mundo.caravanas], heroes: [...mundo.heroes], eventos: [] };
  if (!heroe?.fuera || heroe.ubicacion.tipo !== 'desconectado') return sinCambios;
  const { fuera } = heroe;
  const punto = heroe.ubicacion.punto;
  const plaza = fuera.asentamientoId ? mundo.asentamientos.find((a) => a.id === fuera.asentamientoId) : undefined;
  const vuelta = (h: Heroe): Heroe => ({ ...h, fuera: undefined, desconectaEn: undefined });
  const evento: EventoCrudo = {
    codigo: 'jugador.vuelve_al_mundo',
    ...(plaza ? { asentamientoId: plaza.id } : {}),
    mensaje: `${heroe.displayName} vuelve al mundo.`,
    payload: { heroeId },
  };

  if (plaza && esResidente(plaza, heroeId)) {
    const dentro: Heroe = {
      ...vuelta(heroe),
      ubicacion: { tipo: 'asentamiento', asentamientoId: plaza.id },
      escuadrones: heroe.escuadrones.map((e) => (e.contenedor.tipo === 'fuera' ? { ...e, contenedor: { tipo: 'campamento' } } : e)),
    };
    return { ...sinCambios, heroes: mundo.heroes.map((h) => (h.id === heroeId ? dentro : h)), eventos: [evento] };
  }

  const traidas = heroe.escuadrones.filter((e) => e.contenedor.tipo === 'fuera');
  const residencia = mundo.asentamientos.find((a) => esResidente(a, heroeId));
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
    eventos: [{ ...evento, asentamientoId: residencia?.id }],
  };
}
