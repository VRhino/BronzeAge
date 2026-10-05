// Lo que cuelga de un asentamiento que cae en ruinas se cierra EN EL MOMENTO DE LA RUINA (decisiones del usuario,
// 2026-10-02), no cuando el tick tropieza con ello más tarde (`Consideraciones/Auditoria_Tick_Eventos.md`, ficha B).
//
// Antes, cada dependiente se enteraba a su manera o no se enteraba: la caravana que iba hacia la plaza se perdía sin aviso
// al pasar por el comercio; su campamento de bandidos se quedaba huérfano para siempre (B1); un trueque activo con ella
// seguía vivo hasta vencer y entonces penalizaba al socio por no cumplir un pacto que ya no podía cumplir (B2).
//
// Ahora, al caer una plaza:
//  - **Su campamento de bandidos se dispersa.**
//  - **Sus trueques activos o propuestos se cancelan**, sin penalizar a nadie (quedan `expirado`).
//  - **Las caravanas que iban hacia ella vuelven a casa con su carga** (la ruta se recalcula desde donde están); las
//    que se preparaban para salir hacia ella devuelven la carga al almacén y quedan disponibles; las aparcadas en ella
//    vuelven a su origen. Sin camino por tierra para volver, se pierden, con evento.
//  - **Las caravanas que salían de ella se pierden**, con evento; su escolta vuelve al campamento de su héroe.
//
// Los residentes los reubica `reubicarResidentesDeRuina` (`engine/mercenarios.ts`) y la derrota de su Facción la marca
// `registrarDerrota`; las rutas de la red de caminos se podan solas (`podarRutas`). Las Caravanas de Fundación que
// salían de ella las cierra su propio subsistema (`avanzarCaravanasDeFundacion`: sin origen, se pierden con su evento) y las
// adjuntas a un ejército siguen con él.
import type { AcuerdoTrueque, Asentamiento, CampamentoBandido, Caravana } from '../domain/types';
import type { EventoCrudo } from '../domain/eventos';
import type { Mapa } from '../world/mapa';
import { calcularRuta } from '../world/rutas';
import { agregarRecurso } from './almacen';

export interface DependientesDeRuina {
  asentamientos: Asentamiento[];
  caravanas: Caravana[];
  acuerdos: AcuerdoTrueque[];
  campamentosBandidos: CampamentoBandido[];
}

/** Payloads de los eventos de este cierre. */
export interface PayloadTruequeCancelado {
  acuerdoId: string;
  asentamientoArruinadoId: string;
}
export interface PayloadCaravanaPorRuina {
  caravanaId: string;
  asentamientoArruinadoId: string;
}
export interface PayloadCampamentoDisperso {
  campamentoId: string;
  asentamientoArruinadoId: string;
}

const enPie = (a: AcuerdoTrueque): boolean => a.estado === 'activo' || a.estado === 'propuesto';
const mismaPosicion = (a: { x: number; y: number }, b: { x: number; y: number }): boolean => a.x === b.x && a.y === b.y;

/**
 * Cierra lo que cuelga de `ruina`, ya fuera de `restantes`. Devuelve el mundo actualizado, los eventos y los ids de las
 * escoltas que vuelven al campamento de su héroe (las de las caravanas que se pierden).
 */
export function cerrarDependientesDeRuina(
  ruina: Asentamiento,
  mundo: DependientesDeRuina,
  mapa: Mapa
): DependientesDeRuina & { eventos: EventoCrudo[]; escoltasLiberadas: string[] } {
  const eventos: EventoCrudo[] = [];
  const escoltasLiberadas: string[] = [];
  let asentamientos = mundo.asentamientos;
  const porId = (id: string) => asentamientos.find((a) => a.id === id);

  // Su campamento de bandidos: sin plaza a la que acosar, se dispersa.
  const campamentosBandidos = mundo.campamentosBandidos.filter((c) => {
    if (c.asentamientoId !== ruina.id) return true;
    eventos.push({
      codigo: 'bandidos.campamento_disperso',
      mensaje: `El campamento de bandidos ${c.id} se dispersa: ${ruina.id} ya no existe.`,
      payload: { campamentoId: c.id, asentamientoArruinadoId: ruina.id } satisfies PayloadCampamentoDisperso,
    });
    return false;
  });

  // Sus trueques en pie: se cancelan sin penalizar a nadie. El evento va a la plaza que sigue en pie.
  const acuerdos = mundo.acuerdos.map((ac) => {
    if (!enPie(ac) || (ac.asentamientoAId !== ruina.id && ac.asentamientoBId !== ruina.id)) return ac;
    const socio = ac.asentamientoAId === ruina.id ? ac.asentamientoBId : ac.asentamientoAId;
    eventos.push({
      codigo: 'comercio.trueque_cancelado',
      asentamientoId: socio,
      mensaje: `El trueque ${ac.id} se cancela: ${ruina.id} ha caído en ruinas. Nadie pierde reputación.`,
      payload: { acuerdoId: ac.id, asentamientoArruinadoId: ruina.id } satisfies PayloadTruequeCancelado,
    });
    return { ...ac, estado: 'expirado' as const };
  });

  const caravanas: Caravana[] = [];
  const perder = (c: Caravana, motivo: string): void => {
    escoltasLiberadas.push(...(c.escoltaIds ?? []));
    eventos.push({
      codigo: 'comercio.caravana_perdida',
      asentamientoId: porId(c.origenAsentamientoId) ? c.origenAsentamientoId : undefined,
      mensaje: `La caravana ${c.id} se pierde: ${motivo}.`,
      payload: { caravanaId: c.id, asentamientoArruinadoId: ruina.id } satisfies PayloadCaravanaPorRuina,
    });
  };

  for (const c of mundo.caravanas) {
    // Las de fundación las cierra su propio subsistema, y las adjuntas siguen con su ejército.
    if (c.tipo === 'construccion' || c.estado === 'adjunta') {
      caravanas.push(c);
      continue;
    }
    if (c.origenAsentamientoId === ruina.id) {
      perder(c, `${ruina.id}, su origen, ha caído en ruinas`);
      continue;
    }
    const origen = porId(c.origenAsentamientoId);
    const ibaHaciaLaRuina = c.destinoAsentamientoId === ruina.id && (c.estado === 'en_transito' || c.estado === 'preparando');
    const aparcadaEnLaRuina = c.estado === 'aparcada' && mismaPosicion(c.posicionActual, ruina.posicion);
    if (!ibaHaciaLaRuina && !aparcadaEnLaRuina) {
      caravanas.push(c);
      continue;
    }
    if (!origen) {
      perder(c, `ni su destino ni su origen siguen en pie`);
      continue;
    }

    // Aún en su plaza preparándose: la carga vuelve al almacén y queda disponible.
    if (c.estado === 'preparando') {
      let almacen = origen.almacen;
      for (const [recurso, cantidad] of Object.entries(c.contenido)) if (cantidad > 0) almacen = agregarRecurso(almacen, recurso, cantidad);
      asentamientos = asentamientos.map((a) => (a.id === origen.id ? { ...a, almacen } : a));
      caravanas.push({ ...c, estado: 'disponible', contenido: {}, destinoAsentamientoId: undefined, preparaHasta: undefined, ruta: undefined, progreso: 0, posicionActual: origen.posicion, origenAcuerdoId: undefined, ladoAcuerdo: undefined, peajes: undefined });
      eventos.push({
        codigo: 'comercio.caravana_vuelve',
        asentamientoId: origen.id,
        mensaje: `La caravana ${c.id} no llega a salir: ${ruina.id} ha caído en ruinas. Su carga vuelve al almacén.`,
        payload: { caravanaId: c.id, asentamientoArruinadoId: ruina.id } satisfies PayloadCaravanaPorRuina,
      });
      continue;
    }

    // En camino o aparcada allí: vuelve a casa con lo que lleve, desde donde está (nunca se teletransporta).
    const ruta = calcularRuta(mapa, c.posicionActual, origen.posicion);
    if (!ruta) {
      perder(c, `${ruina.id} ha caído en ruinas y no hay camino por tierra de vuelta a ${origen.id}`);
      continue;
    }
    caravanas.push({ ...c, estado: 'retornando', destinoAsentamientoId: ruina.id, ruta, progreso: 0, peajes: undefined });
    eventos.push({
      codigo: 'comercio.caravana_vuelve',
      asentamientoId: origen.id,
      mensaje: `La caravana ${c.id} da la vuelta: ${ruina.id} ha caído en ruinas. Vuelve a ${origen.id} con su carga.`,
      payload: { caravanaId: c.id, asentamientoArruinadoId: ruina.id } satisfies PayloadCaravanaPorRuina,
    });
  }

  return { asentamientos, caravanas, acuerdos, campamentosBandidos, eventos, escoltasLiberadas };
}
