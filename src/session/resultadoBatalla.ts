// Aplicar el resultado de una batalla de Unity (doc 01 §15-§16, doc 02 §3.3): el checklist de lo que manda Conquest y
// sus consecuencias, en una sola mutación. Unity dice quién ganó, qué le quedó a cada escuadra, la XP y el botín;
// BronzeAge decide el resto a partir de esos hechos: quién queda herido, el carro, la conquista, la caravana o el
// campamento, y la XP de Facción.
import type { Ejercito, Heroe, ItemInstancia } from '../domain/types';
import type { BattleResult, Botin, LadoId } from '../contratos/v1/dto';
import { BATALLA, CAMPAMENTOS_BANDIDOS, MILITAR, NIVEL_FACCION, REPUTACION } from '../constants';
import { minutos, sumar, type Instante } from '../domain/tiempo';
import { aplicarConquista, desalojarResidentes } from '../engine/combate';
import { capacidadCargaDe, cargarBotin, trasDerrota } from '../engine/ejercitos';
import { aplicarAjustesExperiencia, type AjusteExperiencia } from '../engine/faccion';
import { herir } from '../engine/heroe';
import { estanAliadas } from '../engine/pertenencia';
import { aplicarAjustesReputacion } from '../engine/reputacion';
import { alCampamento, conEscuadrones, sinTropa } from '../engine/tropa';
import {
  BatallaInvalidaError,
  conBatalla,
  conReserva,
  escuadrasConDueno,
  escuadrasDe,
  exigirAsignacionActiva,
  exigirRevisionVigente,
  participacionesDe,
  type Batalla,
} from './batallas';
import type { GameSessionState } from './estado';

/**
 * `en_curso -> aplicada` (doc 01 §15). Repetir el mismo resultado no cambia nada, y otro distinto sobre una batalla ya
 * aplicada se rechaza (§16): es lo que deja a Conquest reintentar hasta tener respuesta.
 */
export function aplicarResultado(estado: GameSessionState, b: Batalla, r: BattleResult, servidorId: string, ahora: Instante): GameSessionState {
  if (b.estado === 'aplicada') {
    if (JSON.stringify(b.resultado) === JSON.stringify(r)) return estado;
    throw new BatallaInvalidaError('Esa batalla ya se aplicó con otro resultado.');
  }
  if (b.estado !== 'en_curso' || ahora >= b.expiraEn) throw new BatallaInvalidaError('La batalla no está en curso: no admite resultado.');
  validar(estado, b, r, servidorId);

  const perdedores = participacionesDe(b)
    .filter((p) => p.lado !== r.ganador)
    .map((p) => p.participante.heroeId);
  // Todos los héroes del bando que pierde quedan heridos (Doc 5.16.4).
  let siguiente: GameSessionState = { ...estado, heroes: herir(conLoQueTrajo(estado.heroes, b, r), perdedores, ahora) };
  siguiente = consecuencias(siguiente, b, r.ganador, ahora);
  siguiente = conXpDeFaccion(siguiente, b, r);
  return conBatalla(siguiente, { ...b, estado: 'aplicada', resultado: r, aplicadaEn: ahora });
}

/** El checklist de doc 02 §3.3, puntos 1-8 y 10-12 (el 9, la versión del contrato, lo mira la ruta). Cualquier fallo
 * rechaza el resultado entero: nunca se aplica a medias. */
function validar(estado: GameSessionState, b: Batalla, r: BattleResult, servidorId: string): void {
  const falla = (motivo: string): never => {
    throw new BatallaInvalidaError(`Resultado inválido: ${motivo}.`);
  };
  exigirAsignacionActiva(b, r.intentoAsignacionId, servidorId);
  exigirRevisionVigente(b, r.ticketRevision);

  const autorizados = new Map(escuadrasDe(b).map((s) => [s.squadId, s.efectivosAutorizados]));
  const participantes = new Map(participacionesDe(b).map((p) => [p.participante.heroeId, p.participante]));
  const sinRepetir = (ids: string[]) => new Set(ids).size === ids.length;
  if (!sinRepetir(r.porEscuadra.map((e) => e.squadId)) || !sinRepetir(r.porHeroe.map((h) => h.heroeId))) falla('hay ids repetidos');
  if (r.porEscuadra.length !== autorizados.size || r.porEscuadra.some((e) => !autorizados.has(e.squadId))) {
    falla('porEscuadra no cubre exactamente las escuadras de la batalla');
  }
  if (r.porHeroe.length !== participantes.size || r.porHeroe.some((h) => !participantes.has(h.heroeId))) {
    falla('porHeroe no cubre exactamente a los héroes de la batalla');
  }
  for (const e of r.porEscuadra) {
    const autorizadas = autorizados.get(e.squadId)!;
    if (e.supervivientesAlCierre + e.muertos !== autorizadas) falla(`${e.squadId} no cierra sobre sus ${autorizadas} efectivos`);
    if (e.desplegados > autorizadas) falla(`${e.squadId} despliega más de lo autorizado`);
  }
  if (r.razon === 'tiempo_agotado' && r.ganador !== 'defensor') falla('si se agota el tiempo gana el defensor');
  const duracion = Date.parse(r.fin) - Date.parse(r.inicio);
  if (!(duracion > 0) || duracion > b.ticket.reglas.duracionMaximaSegundos * 1000) falla('la duración no cabe en las reglas');

  // `ponytail:` sin el catálogo de objetos de Conquest (CQ-004) no se comprueba que cada `itemDefinitionId` exista
  // (decisión del usuario, 2026-09-15: aceptarlos).
  const yaExisten = new Set(
    estado.heroes.flatMap((h) => [...h.inventario, ...Object.values(h.equipamiento)]).flatMap((i) => (i?.itemInstanceId ? [i.itemInstanceId] : []))
  );
  const nuevos = r.porHeroe.flatMap((h) => h.botin?.objetos ?? []).flatMap((o) => (o.itemInstanceId ? [o.itemInstanceId] : []));
  if (!sinRepetir(nuevos) || nuevos.some((id) => yaExisten.has(id))) falla('algún itemInstanceId no es nuevo en la partida');
  for (const h of r.porHeroe) {
    if (!h.botin) continue;
    if (!h.participo) falla(`${h.heroeId} no participó y trae botín`);
    if (h.botin.objetos.length > participantes.get(h.heroeId)!.heroe.casillasInventarioLibres) falla(`el botín de ${h.heroeId} no cabe en su inventario`);
  }
}

/** Lo que Unity dice de cada uno: supervivientes y XP de cada escuadra, XP y botín de cada héroe, y las escuadras sin
 * candado. La XP se suma y el nivel no cambia hasta que Conquest publique su curva (CQ-001; decisión del usuario,
 * 2026-09-15). */
function conLoQueTrajo(heroes: readonly Heroe[], b: Batalla, r: BattleResult): Heroe[] {
  const porEscuadra = new Map(r.porEscuadra.map((e) => [e.squadId, e]));
  const porHeroe = new Map(r.porHeroe.map((h) => [h.heroeId, h]));
  const tras = heroes.map((h) => {
    const escuadrones = h.escuadrones.map((e) => {
      const suya = porEscuadra.get(e.id);
      return suya ? { ...e, cantidad: suya.supervivientesAlCierre, experiencia: e.experiencia + suya.xpGanada } : e;
    });
    const suyo = porHeroe.get(h.id);
    if (!suyo) return { ...h, escuadrones };
    return {
      ...h,
      escuadrones,
      experienciaHaciaSiguienteNivel: h.experienciaHaciaSiguienteNivel + suyo.xpGanada,
      ...(suyo.botin ? conBotin(h, suyo.botin) : {}),
    };
  });
  return conReserva(tras, escuadrasConDueno(b), undefined);
}

/** El botín se suma a lo que ya tiene (doc 01 §15), cada objeto en una casilla libre. `ponytail:` sin el catálogo de
 * objetos (CQ-004) no se sabe el tipo ni el precio: van `visual` y 0 hasta que Conquest lo publique. */
function conBotin(h: Heroe, botin: Botin): Pick<Heroe, 'inventario' | 'monedasHeroe'> {
  const ocupadas = new Set(h.inventario.map((i) => i.casillaInventario));
  const libres = Array.from({ length: BATALLA.casillasInventario }, (_, i) => i).filter((i) => !ocupadas.has(i));
  const nuevos: ItemInstancia[] = botin.objetos.map((o, i) => ({ ...o, tipo: 'visual', precio: 0, casillaInventario: libres[i]! }));
  const { bronce, plata, oro } = h.monedasHeroe;
  return {
    inventario: [...h.inventario, ...nuevos],
    monedasHeroe: { bronce: bronce + botin.monedas.bronce, plata: plata + botin.monedas.plata, oro: oro + botin.monedas.oro },
  };
}

/** Lo que decide BronzeAge según dónde se combatió (doc 01 §15, Doc 5.15-5.16). */
function consecuencias(estado: GameSessionState, b: Batalla, ganador: LadoId, ahora: Instante): GameSessionState {
  const contexto = b.ticket.contextoEstrategico;
  if (contexto.tipo === 'asedio') return ganador === 'atacante' ? conquistar(estado, b, ahora) : estado;
  const trasDerrotas = derrotaEnCampo(estado, b, ganador);
  if (ganador === 'defensor') return trasDerrotas;
  if (contexto.tipo === 'caravana') return capturarCaravana(trasDerrotas, b, contexto.caravanaId);
  if (contexto.tipo === 'campamento_bandidos') return destruirCampamento(trasDerrotas, b, contexto.campamentoId, ahora);
  return trasDerrotas;
}

/** Las columnas de un bando, en el orden del bloqueo: la primera es la que abrió o recibió el combate. */
function columnasDelBando(estado: GameSessionState, b: Batalla, lado: LadoId): Ejercito[] {
  const suyos = new Set(
    participacionesDe(b)
      .filter((p) => p.lado === lado)
      .map((p) => p.participante.heroeId)
  );
  return b.bloqueo.ejercitoIds.flatMap((id) => estado.ejercitos.find((e) => e.id === id && e.participantes.some((p) => suyos.has(p.heroeId))) ?? []);
}

function conEjercitos(estado: GameSessionState, actualizados: readonly Ejercito[]): GameSessionState {
  const porId = new Map(actualizados.map((e) => [e.id, e]));
  return { ...estado, ejercitos: estado.ejercitos.map((e) => porId.get(e.id) ?? e) };
}

/** La primera columna del bando, con el botín cargado hasta donde le quepa. Sin columna, el botín se pierde. */
function alCarroDe(estado: GameSessionState, b: Batalla, lado: LadoId, botin: Record<string, number>): GameSessionState {
  const columna = columnasDelBando(estado, b, lado)[0];
  return columna ? conEjercitos(estado, [cargarBotin(columna, botin, capacidadCargaDe(columna, estado.caravanas))]) : estado;
}

/**
 * Mundo abierto (Doc 5.12.3, 5.16.6): cada columna del bando que pierde entrega la mitad de su carro, y va a la primera
 * columna del que gana, lo que quepa. Contra una caravana o un campamento, esa mitad no se la lleva nadie.
 * `ponytail:` los que se unieron al bando ganador no cogen botín; repartirlo si se echa en falta.
 */
function derrotaEnCampo(estado: GameSessionState, b: Batalla, ganador: LadoId): GameSessionState {
  const tras = columnasDelBando(estado, b, ganador === 'atacante' ? 'defensor' : 'atacante').map((c) => trasDerrota(c));
  if (tras.length === 0) return estado;
  const botin: Record<string, number> = {};
  for (const t of tras) for (const [recurso, cantidad] of Object.entries(t.botin)) botin[recurso] = (botin[recurso] ?? 0) + cantidad;
  return alCarroDe(
    conEjercitos(
      estado,
      tras.map((t) => t.perdedor)
    ),
    b,
    ganador,
    botin
  );
}

/** La caravana cae (Doc 5.15.4): su escolta queda a 0 y vuelve al campamento de su héroe, y la mitad de su carga va al
 * carro de quien la atacó, lo que quepa (Doc 3.10). */
function capturarCaravana(estado: GameSessionState, b: Batalla, caravanaId: string): GameSessionState {
  const caravana = estado.caravanas.find((c) => c.id === caravanaId);
  if (!caravana) return estado;
  const escolta = new Set(caravana.escoltaIds ?? []);
  const perdida = estado.heroes.flatMap((h) => h.escuadrones.filter((e) => escolta.has(e.id))).map((e) => ({ ...e, cantidad: 0 }));
  const botin = Object.fromEntries(Object.entries(caravana.contenido).map(([recurso, cantidad]) => [recurso, cantidad * MILITAR.umbralCapturaCaravana]));
  return alCarroDe(
    { ...estado, heroes: conEscuadrones(estado.heroes, alCampamento(perdida)), caravanas: estado.caravanas.filter((c) => c.id !== caravanaId) },
    b,
    'atacante',
    botin
  );
}

/** El campamento cae (Doc 1.9): su recompensa va al carro de quien lo atacó, lo que quepa, y se agenda su reaparición. */
function destruirCampamento(estado: GameSessionState, b: Batalla, campamentoId: string, ahora: Instante): GameSessionState {
  const recompensa = Object.fromEntries(Object.entries(CAMPAMENTOS_BANDIDOS.recompensa).filter((e): e is [string, number] => (e[1] ?? 0) > 0));
  return alCarroDe(
    {
      ...estado,
      campamentosBandidos: estado.campamentosBandidos.filter((c) => c.id !== campamentoId),
      bandidosProximoSpawnEn: sumar(ahora, minutos(CAMPAMENTOS_BANDIDOS.respawnMinutos)),
    },
    b,
    'atacante',
    recompensa
  );
}

/** Asedio que gana el atacante (Doc 5.15.5): la plaza pasa a su Facción con el saqueo de siempre (Doc 5.12.9), y quien
 * estaba dentro sale; los defensores, con las escuadras que les quedaron. Unity no reporta daño: lo pone el saqueo. */
function conquistar(estado: GameSessionState, b: Batalla, ahora: Instante): GameSessionState {
  const plaza = estado.asentamientos.find((a) => a.id === b.bloqueo.asentamientoId);
  const faccionId = b.ticket.bandos.atacante.faccionId;
  if (!plaza || !faccionId) return estado;
  const lucharon = new Set(
    participacionesDe(b)
      .filter((p) => p.lado === 'defensor')
      .flatMap((p) => p.participante.escuadras.map((e) => e.squadId))
  );
  const conquistada = estado.asentamientos.map((a) => (a.id === plaza.id ? aplicarConquista(plaza, faccionId, ahora) : a));
  const desalojo = desalojarResidentes(plaza, conquistada, estado.heroes, estado.ejercitos, lucharon, ahora);
  const fuera = desalojo.columnas.map(sinTropa);
  return {
    ...estado,
    asentamientos: desalojo.asentamientos,
    ejercitos: [...estado.ejercitos, ...fuera.map((f) => f.ejercito)],
    heroes: conEscuadrones(desalojo.heroes, fuera.flatMap((f) => f.tropa)),
  };
}

/** XP de Facción por combatir, como en el combate con números (Doc Fase_0_5 §8): por cada héroe de su bando que
 * participó, más la de conquista a quien gana un asedio. Asediar a un Aliado cuesta reputación (Doc 2.7). */
function conXpDeFaccion(estado: GameSessionState, b: Batalla, r: BattleResult): GameSessionState {
  const participaron = new Set(r.porHeroe.filter((h) => h.participo).map((h) => h.heroeId));
  const { atacante, defensor } = b.ticket.bandos;
  const ajustes: AjusteExperiencia[] = (['atacante', 'defensor'] as const).flatMap((lado) => {
    const faccionId = b.ticket.bandos[lado].faccionId;
    const heroes = participacionesDe(b).filter((p) => p.lado === lado && participaron.has(p.participante.heroeId)).length;
    return faccionId && heroes > 0 ? [{ faccionId, delta: NIVEL_FACCION.xp.combate * heroes, razon: 'combate (batalla)' }] : [];
  });
  const asedio = b.ticket.contextoEstrategico.tipo === 'asedio';
  if (asedio && r.ganador === 'atacante' && atacante.faccionId) ajustes.push({ faccionId: atacante.faccionId, delta: NIVEL_FACCION.xp.conquista, razon: 'conquista' });
  const facciones = aplicarAjustesExperiencia(estado.facciones, ajustes);
  const contraAliado = asedio && atacante.faccionId && defensor.faccionId && estanAliadas(estado.relaciones, atacante.faccionId, defensor.faccionId);
  return {
    ...estado,
    facciones: contraAliado
      ? aplicarAjustesReputacion(facciones, [{ faccionId: atacante.faccionId!, delta: REPUTACION.penalizacionAtacarAliado, razon: 'atacar a un Aliado' }])
      : facciones,
  };
}
