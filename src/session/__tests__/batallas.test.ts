// Batallas de Unity del lado de BronzeAge, fase 1 (doc 01 §15, Doc 5.15.1): abrirlas con el ticket del contrato,
// bloquear lo que interviene, unirse, cancelar y vencer. Con el flag apagado todo sigue con números, y eso lo cubren
// los tests de cada combate; aquí basta con uno que lo confirme.
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv from 'ajv';
import { describe, expect, it } from 'vitest';
import { GameSession } from '../gameSession';
import { REGISTRO_COMANDOS } from '../comandos/registro';
import { verificarAutorizacion } from '../comandos/autorizacion';
import { CODIGOS_ERROR } from '../comandos/codigosDeError';
import { proyectarParaJugador } from '../proyecciones/jugador';
import { enConvocatoria, escuadrasDe, faccionesEnBatalla, participacionesDe, type Batalla } from '../batallas';
import { aplicarResultado, confirmarInicio, registrarAsignacion, registrarSalida } from '../comandos/batalla';
import { instanteDeTick, type GeometriaAsentamientos } from '../estado';
import { conHeroe, enPie, frenteACampamento } from './fixtures';
import { heridosEn } from '../../engine/heroe';
import { SCHEMA_VERSION, type BattleResult } from '../../contratos/v1/dto';
import { BATALLA, CAMPAMENTOS_BANDIDOS, MOVIMIENTO } from '../../constants';

const SCHEMA = JSON.parse(readFileSync(join(dirname(fileURLToPath(import.meta.url)), '../../contratos/v1/contratos.schema.json'), 'utf8'));
const ajv = new Ajv({ allErrors: true, strict: true, strictRequired: false, allowUnionTypes: true });
ajv.addSchema(SCHEMA, 'v1');
function erroresDe(definicion: string, dato: unknown) {
  const validar = ajv.getSchema(`v1#/definitions/${definicion}`)!;
  return validar(dato) ? [] : validar.errors;
}

const SIN_GEOMETRIA: GeometriaAsentamientos = { zonas: [], zonasFusionadas: [], trazadoPorAsentamiento: {} };

const atacarCampamento = (sesion: GameSession, heroeId: string) =>
  sesion.ejecutar(REGISTRO_COMANDOS.atacar, { heroeId, objetivo: { tipo: 'campamento', id: 'camp-1' } }, { actor: heroeId });

const asediar = (sesion: GameSession, heroeId: string, plazaId: string) =>
  sesion.ejecutar(REGISTRO_COMANDOS.atacar, { heroeId, objetivo: { tipo: 'asentamiento', id: plazaId } }, { actor: heroeId });

const dejarDePerseguir = (sesion: GameSession, heroeId: string) =>
  sesion.ejecutar(REGISTRO_COMANDOS.dejarDePerseguir, { heroeId }, { actor: heroeId });

const reservadas = (sesion: GameSession, heroeId: string) =>
  sesion.getState().heroes.find((h) => h.id === heroeId)!.escuadrones.filter((e) => e.reservaBatalla !== undefined);

describe('abrir una batalla de Unity (doc 01 §15)', () => {
  it('sin servidores de batalla, atacar se sigue resolviendo con números', () => {
    const { sesion, fundador } = frenteACampamento(false);

    const r = atacarCampamento(sesion, fundador);

    expect(r.ok).toBe(true);
    expect(r.datos).toBeUndefined();
    expect(sesion.getState().batallas).toEqual([]);
  });

  it('con servidores, atacar un campamento abre una batalla con el ticket del contrato y reserva las escuadras', () => {
    const { sesion, fundador } = frenteACampamento();

    const r = atacarCampamento(sesion, fundador);

    expect(r.ok).toBe(true);
    const [batalla] = sesion.getState().batallas;
    expect(batalla!.id).toBe(r.datos!.battleId);
    expect(batalla!.estado).toBe('convocando');
    expect(erroresDe('BattleTicket', batalla!.ticket)).toEqual([]);
    expect(batalla!.ticket.bandos.atacante.participantes.map((p) => p.heroeId)).toEqual([fundador]);
    expect(batalla!.ticket.bandos.defensor.escuadrasSinHeroe).toMatchObject([{ heroeId: null, tropaId: 'milicia_lanceros', efectivosAutorizados: CAMPAMENTOS_BANDIDOS.niveles[1].unidades }]);
    expect(reservadas(sesion, fundador).map((e) => e.reservaBatalla!.battleId)).toEqual([batalla!.id]);
    expect(sesion.getState().campamentosBandidos, 'no se resuelve aquí: el campamento sigue en pie').toHaveLength(1);
  });
});

describe('asediar es una orden, no una llegada (Doc 5.12.4)', () => {
  it('sin servidores de batalla, atacar una plaza enemiga a 15 la asedia con números', () => {
    const { sesion, fundador, plazaId, faccionPropia } = frenteAPlaza(false);

    const r = asediar(sesion, fundador, plazaId);

    expect(r.ok).toBe(true);
    expect(sesion.getState().asentamientos.find((a) => a.id === plazaId)!.faccionId, 'sin defensores cae').toBe(faccionPropia);
  });

  it('con servidores, abre la batalla de asedio a nombre de quien la ordena, y un segundo asedio a la misma plaza espera', () => {
    const { sesion, fundador, vecino, plazaId } = frenteAPlaza();

    const r = asediar(sesion, fundador, plazaId);

    const batalla = sesion.getState().batallas[0]!;
    expect(batalla.id).toBe(r.datos!.battleId);
    expect(batalla.iniciadaPor).toBe(fundador);
    expect(batalla.ticket.contextoEstrategico).toEqual({ tipo: 'asedio', asentamientoId: plazaId });
    expect(batalla.bloqueo.asentamientoId).toBe(plazaId);
    expect(asediar(sesion, vecino, plazaId).ok, 'la plaza ya está en una batalla').toBe(false);
  });

  it('una columna personal no abre un asedio (Doc 5.15.1b)', () => {
    const { sesion, fundador, plazaId } = frenteAPlaza(false);
    const payload = sesion.exportar();
    const solitaria = GameSession.importar({
      ...payload,
      state: { ...payload.state, ejercitos: payload.state.ejercitos.map((e) => ({ ...e, tipo: 'personal' as const })) },
    });

    expect(asediar(solitaria, fundador, plazaId).ok).toBe(false);
    expect(solitaria.getState().asentamientos.find((a) => a.id === plazaId)!.faccionId, 'la plaza sigue siendo de quien era').not.toBe(
      payload.state.asentamientos[0]!.faccionId
    );
  });

  it('una plaza propia no se asedia', () => {
    const { sesion, fundador } = frenteAPlaza(false);

    expect(asediar(sesion, fundador, sesion.getState().asentamientos[0]!.id).ok).toBe(false);
  });
});

describe('la convocatoria de un asedio (Doc 5.15.1b)', () => {
  const asignar = (sesion: GameSession, battleId: string) =>
    sesion.ejecutar(
      registrarAsignacion,
      {
        servidorId: 's1',
        mensaje: {
          schemaVersion: SCHEMA_VERSION,
          battleId,
          ticketRevision: 0,
          intentoAsignacionId: 'intento-1',
          instancia: { host: 'batalla-1.example', puerto: 7777, protocolo: 'udp' },
          tokensParticipante: [],
        },
      },
      { actor: 'batalla-servidor:s1' }
    );
  const unirse = (sesion: GameSession, heroeId: string, battleId: string) => sesion.ejecutar(REGISTRO_COMANDOS.unirseABatalla, { heroeId, battleId }, { actor: heroeId });

  it('quien se une durante ella entra en el ticket, y la batalla no se asigna hasta que acaba', () => {
    const { sesion, fundador, vecino, plazaId } = frenteAPlaza();
    const battleId = asediar(sesion, fundador, plazaId).datos!.battleId;

    expect(enConvocatoria(sesion.getState().batallas[0]!, instanteDeTick(sesion.getState().tick))).toBe(true);
    expect(asignar(sesion, battleId).ok, 'Conquest todavía no puede asignarla').toBe(false);
    expect(unirse(sesion, vecino, battleId).ok).toBe(true);

    const durante = sesion.getState().batallas[0]!;
    expect(durante.ticket.bandos.atacante.participantes.map((p) => p.heroeId).sort()).toEqual([fundador, vecino].sort());
    expect(durante.incorporaciones, 'no es una incorporación').toEqual([]);
    expect(durante.ticket.ticketRevision).toBe(0);

    sesion.avanzarTick();
    expect(enConvocatoria(sesion.getState().batallas[0]!, instanteDeTick(sesion.getState().tick))).toBe(false);
    expect(asignar(sesion, battleId).ok).toBe(true);
  });

  it('quien llega después, ya con el ticket cerrado, entra como incorporación', () => {
    const { sesion, fundador, vecino, plazaId } = frenteAPlaza();
    const battleId = asediar(sesion, fundador, plazaId).datos!.battleId;
    sesion.avanzarTick();

    expect(unirse(sesion, vecino, battleId).ok).toBe(true);

    const b = sesion.getState().batallas[0]!;
    expect(b.ticket.bandos.atacante.participantes.map((p) => p.heroeId)).toEqual([fundador]);
    expect(b.incorporaciones.map((i) => i.participante.heroeId)).toEqual([vecino]);
  });

  it('las demás batallas no tienen convocatoria: salen hacia Unity al abrirse', () => {
    const { sesion, fundador } = frenteACampamento();
    atacarCampamento(sesion, fundador);

    expect(sesion.getState().batallas[0]!.convocatoriaHasta).toBeUndefined();
  });
});

describe('mientras se juega (Doc 5.15.1)', () => {
  it('lo que combate no recibe órdenes ni se mueve, y nadie más ataca el campamento', () => {
    const { sesion, fundador, vecino, columna } = frenteACampamento();
    atacarCampamento(sesion, fundador);
    const antes = sesion.getState().ejercitos.find((e) => e.id === columna);

    expect(dejarDePerseguir(sesion, fundador).codigoError).toBe(CODIGOS_ERROR.batallaBloqueo);
    expect(atacarCampamento(sesion, vecino).codigoError).toBe(CODIGOS_ERROR.batallaBloqueo);
    sesion.avanzarTick();
    expect(sesion.getState().ejercitos.find((e) => e.id === columna), 'fuera del tick: ni se mueve ni come').toEqual(antes);
  });

  it('si nadie la asigna a tiempo, falla sin castigo y suelta los candados', () => {
    const { sesion, fundador } = frenteACampamento();
    atacarCampamento(sesion, fundador);

    for (let i = 0; i < BATALLA.plazoAsignacionMinutos; i++) sesion.avanzarTick();

    expect(sesion.getState().batallas[0]!.estado).toBe('fallida');
    expect(reservadas(sesion, fundador)).toEqual([]);
    expect(sesion.getState().campamentosBandidos.some((c) => c.id === 'camp-1')).toBe(true);
    expect(dejarDePerseguir(sesion, fundador).ok).toBe(true);
  });

  it('quien la inició la cancela antes de empezar; otro jugador no puede', () => {
    const { sesion, fundador, vecino } = frenteACampamento();
    const battleId = atacarCampamento(sesion, fundador).datos!.battleId;

    expect(verificarAutorizacion('cancelarBatalla', { battleId }, sesion.getState(), { rol: 'jugador', heroeId: vecino }).autorizado).toBe(false);
    expect(sesion.ejecutar(REGISTRO_COMANDOS.cancelarBatalla, { battleId }, { actor: fundador }).ok).toBe(true);
    expect(sesion.getState().batallas[0]!.estado).toBe('cancelada');
    expect(reservadas(sesion, fundador)).toEqual([]);
  });

  it('la IA de las Facciones que combaten no gobierna mientras dura', () => {
    const { sesion, fundador } = frenteACampamento();
    atacarCampamento(sesion, fundador);
    const estado = sesion.getState();
    const suya = estado.facciones.find((f) => f.ciudadanosIds.includes(fundador))!.id;

    expect([...faccionesEnBatalla(estado, instanteDeTick(estado.tick))]).toEqual([suya]);
  });
});

describe('unirse a una batalla (Doc 5.15.1)', () => {
  it('un compañero de Facción se une con su columna mientras queda sitio', () => {
    const { sesion, fundador, vecino, columnaVecino } = frenteACampamento();
    const battleId = atacarCampamento(sesion, fundador).datos!.battleId;

    const r = sesion.ejecutar(REGISTRO_COMANDOS.unirseABatalla, { heroeId: vecino, battleId }, { actor: vecino });

    expect(r.ok).toBe(true);
    const batalla = sesion.getState().batallas[0]!;
    expect(batalla.incorporaciones).toMatchObject([{ secuencia: 1, lado: 'atacante', participante: { heroeId: vecino } }]);
    expect(erroresDe('IncorporacionBatalla', batalla.incorporaciones[0])).toEqual([]);
    expect(batalla.bloqueo.ejercitoIds).toContain(columnaVecino);
    expect(reservadas(sesion, vecino)).toHaveLength(1);
    expect(dejarDePerseguir(sesion, vecino).codigoError).toBe(CODIGOS_ERROR.batallaBloqueo);
  });

  it('no entra en un bando lleno; a un campamento de bandidos, que es un evento PvE, se une cualquiera, también sin Facción', () => {
    const { sesion, fundador, vecino } = frenteACampamento();
    const battleId = atacarCampamento(sesion, fundador).datos!.battleId;
    const unirse = (s: GameSession, heroeId: string) => s.ejecutar(REGISTRO_COMANDOS.unirseABatalla, { heroeId, battleId }, { actor: heroeId });

    expect(unirse(conCapacidad(sesion, 1), vecino).codigoError, 'el atacante ya está y el bando es de 1').toBe(CODIGOS_ERROR.batallaInvalida);
    const forastero = enPie(conHeroe(sesion, 'forastero'), 'forastero', sesion.getState().batallas[0]!.punto);
    expect(unirse(forastero, 'forastero').ok, 'contra los bandidos van todos juntos').toBe(true);
    expect(forastero.getState().batallas[0]!.incorporaciones.map((i) => i.lado)).toEqual(['atacante']);
  });
});

describe('la batalla en el mapa (doc 02 §4.1)', () => {
  it('quien combate la ve con su bando, y su compañero la ve desde la plaza', () => {
    const { sesion, fundador, vecino } = frenteACampamento();
    const battleId = atacarCampamento(sesion, fundador).datos!.battleId;

    const propia = proyectarParaJugador(sesion.getState(), fundador, SIN_GEOMETRIA).batallas;
    const ajena = proyectarParaJugador(sesion.getState(), vecino, SIN_GEOMETRIA).batallas;

    expect(propia).toMatchObject([{ battleId, ladoPropio: 'atacante', bandos: { atacante: { heroes: 1, capacidadMaxima: 5 } } }]);
    expect(ajena).toMatchObject([{ battleId }]);
    expect(ajena[0]!.ladoPropio).toBeUndefined();
  });
});

describe('aplicar el resultado (doc 02 §3.3, doc 01 §15-§16)', () => {
  it('gana el atacante: el campamento cae, cada escuadra queda con lo que le dejó Unity, y el héroe suma su XP y su botín', () => {
    const { sesion, fundador } = frenteACampamento();
    const batalla = empezar(sesion, atacarCampamento(sesion, fundador).datos!.battleId);
    const antes = sesion.getState().heroes.find((h) => h.id === fundador)!;
    const escuadra = batalla().ticket.bandos.atacante.participantes[0]!.escuadras[0]!;
    const botin = { objetos: [{ itemDefinitionId: 'botas_cuero', cantidad: 1, itemInstanceId: 'item-1' }], monedas: { bronce: 85, plata: 3, oro: 0 } };
    const r = resultado(batalla(), { porHeroe: [{ heroeId: fundador, participo: true, sobrevivioAlCierre: true, xpGanada: 50, botin }] });

    expect(aplicar(sesion, r).ok).toBe(true);

    const estado = sesion.getState();
    const heroe = estado.heroes.find((h) => h.id === fundador)!;
    const suya = heroe.escuadrones.find((e) => e.id === escuadra.squadId)!;
    expect(batalla().estado).toBe('aplicada');
    expect(estado.campamentosBandidos).toEqual([]);
    expect(suya).toMatchObject({ cantidad: escuadra.efectivosAutorizados - 1, experiencia: escuadra.experiencia + 10 });
    expect(suya.reservaBatalla, 'sin candado').toBeUndefined();
    expect(heroe.experienciaHaciaSiguienteNivel).toBe(antes.experienciaHaciaSiguienteNivel + 50);
    expect(heroe.nivel, 'sin la curva de Conquest el nivel no cambia').toBe(antes.nivel);
    expect(heroe.monedasHeroe).toEqual({ bronce: antes.monedasHeroe.bronce + 85, plata: antes.monedasHeroe.plata + 3, oro: antes.monedasHeroe.oro });
    expect(heroe.inventario).toMatchObject([{ itemDefinitionId: 'botas_cuero', itemInstanceId: 'item-1', casillaInventario: 0 }]);
    expect(heridosEn(estado.heroes, instanteDeTick(estado.tick)).has(fundador)).toBe(false);
  });

  it('el vencedor recoge la chatarra de los caídos en su carro (Doc 4.2.1)', () => {
    const frente = frenteACampamento();
    // Las escuadras de los héroes pasan a ser de espadachines de bronce, con equipo de metal, antes de abrir el ticket.
    const payload = frente.sesion.exportar();
    const heroes = payload.state.heroes.map((h) => ({ ...h, escuadrones: h.escuadrones.map((e) => ({ ...e, tropaId: 'espadachines_bronce' })) }));
    const sesion = GameSession.importar({ ...payload, state: { ...payload.state, heroes } }, { batallasEnUnity: true });
    const batalla = empezar(sesion, atacarCampamento(sesion, frente.fundador).datos!.battleId);
    const escuadra = batalla().ticket.bandos.atacante.participantes[0]!.escuadras[0]!;
    const r = resultado(batalla());
    const enElCarro = () => sesion.getState().ejercitos.find((e) => e.id === frente.columna)!.suministro['chatarraBronce'] ?? 0;

    expect(aplicar(sesion, { ...r, porEscuadra: r.porEscuadra.map((e) => (e.squadId === escuadra.squadId ? { ...e, muertos: 4, supervivientesAlCierre: e.desplegados - 4 } : e)) }).ok).toBe(true);

    expect(enElCarro()).toBeGreaterThan(0);
  });

  it('la chatarra se reparte a partes iguales entre las columnas del bando que gana, también la que se unió (Doc 5.12.3)', () => {
    const frente = frenteACampamento();
    const payload = frente.sesion.exportar();
    const heroes = payload.state.heroes.map((h) => ({ ...h, escuadrones: h.escuadrones.map((e) => ({ ...e, tropaId: 'espadachines_bronce' })) }));
    const sesion = GameSession.importar({ ...payload, state: { ...payload.state, heroes } }, { batallasEnUnity: true });
    const battleId = atacarCampamento(sesion, frente.fundador).datos!.battleId;
    expect(sesion.ejecutar(REGISTRO_COMANDOS.unirseABatalla, { heroeId: frente.vecino, battleId }, { actor: frente.vecino }).ok).toBe(true);
    const batalla = empezar(sesion, battleId);
    const r = resultado(batalla());
    const muertos = Object.fromEntries(batalla().ticket.bandos.atacante.participantes.map((p) => [p.escuadras[0]!.squadId, 4]));
    const conBajas = {
      ...r,
      porEscuadra: r.porEscuadra.map((e) => (e.squadId in muertos ? { ...e, muertos: 4, supervivientesAlCierre: e.desplegados - 4 } : e)),
    };
    const chatarraEn = (id: string) => sesion.getState().ejercitos.find((e) => e.id === id)!.suministro['chatarraBronce'] ?? 0;

    expect(aplicar(sesion, conBajas).ok).toBe(true);

    expect(chatarraEn(frente.columna)).toBeGreaterThan(0);
    expect(chatarraEn(frente.columnaVecino)).toBeCloseTo(chatarraEn(frente.columna));
  });

  it('ganan los bandidos: quien atacó queda herido y pierde la mitad del carro', () => {
    const { sesion, fundador, columna } = frenteACampamento();
    const batalla = empezar(sesion, atacarCampamento(sesion, fundador).datos!.battleId);
    const carro = sesion.getState().ejercitos.find((e) => e.id === columna)!.suministro['trigo']!;

    expect(aplicar(sesion, resultado(batalla(), { ganador: 'defensor', razon: 'tiempo_agotado' })).ok).toBe(true);

    const estado = sesion.getState();
    expect(heridosEn(estado.heroes, instanteDeTick(estado.tick)).has(fundador)).toBe(true);
    expect(estado.ejercitos.find((e) => e.id === columna)!.suministro['trigo']).toBeCloseTo(carro * (1 - MOVIMIENTO.fraccionRobada));
    expect(estado.campamentosBandidos).toHaveLength(1);
  });

  it('un asedio que gana el atacante conquista la plaza, con el saqueo de siempre', () => {
    const { sesion, fundador, plazaId, faccionPropia } = frenteAPlaza();
    const batalla = empezar(sesion, asediar(sesion, fundador, plazaId).datos!.battleId);

    expect(aplicar(sesion, resultado(batalla())).ok).toBe(true);

    const plaza = sesion.getState().asentamientos.find((a) => a.id === plazaId)!;
    expect(plaza.faccionId).toBe(faccionPropia);
    expect(plaza.ocupacionHasta, 'abre la ventana de ocupación').toBeDefined();
  });

  it('repetir el mismo resultado no cambia nada; otro distinto sobre la batalla ya aplicada se rechaza', () => {
    const { sesion, fundador } = frenteACampamento();
    const batalla = empezar(sesion, atacarCampamento(sesion, fundador).datos!.battleId);
    const r = resultado(batalla());
    aplicar(sesion, r);
    const tras = sesion.getState();

    expect(aplicar(sesion, r).ok).toBe(true);
    expect(sesion.getState()).toEqual(tras);
    expect(aplicar(sesion, { ...r, ganador: 'defensor' }).codigoError).toBe(CODIGOS_ERROR.batallaInvalida);
  });

  it('el checklist rechaza el resultado entero: la batalla sigue en curso y las escuadras reservadas', () => {
    const { sesion, fundador } = frenteACampamento();
    const batalla = empezar(sesion, atacarCampamento(sesion, fundador).datos!.battleId);
    const r = resultado(batalla());
    const casos: [string, BattleResult][] = [
      ['falta una escuadra', { ...r, porEscuadra: r.porEscuadra.slice(1) }],
      ['no cierra sobre lo autorizado', { ...r, porEscuadra: r.porEscuadra.map((e) => ({ ...e, muertos: e.muertos + 1 })) }],
      ['gana el atacante por tiempo', { ...r, razon: 'tiempo_agotado' }],
      ['dura más que las reglas', { ...r, fin: '2026-09-15T11:00:00Z' }],
      ['botín a quien no participó', { ...r, porHeroe: r.porHeroe.map((h) => ({ ...h, participo: false, botin: { objetos: [], monedas: { bronce: 1, plata: 0, oro: 0 } } })) }],
      ['otra revisión del ticket', { ...r, ticketRevision: 1 }],
      ['otro intento de asignación', { ...r, intentoAsignacionId: 'intento-2' }],
    ];

    for (const [caso, malo] of casos) expect(aplicar(sesion, malo).codigoError, caso).toBe(CODIGOS_ERROR.batallaInvalida);
    expect(aplicar(sesion, r, 's2').codigoError, 'otro servidor').toBe(CODIGOS_ERROR.batallaInvalida);
    expect(batalla().estado).toBe('en_curso');
    expect(reservadas(sesion, fundador)).not.toEqual([]);
  });
});

/** Conquest la asigna y la empieza (doc 02 §3.3). Devuelve cómo leerla después. */
function empezar(sesion: GameSession, battleId: string): () => Batalla {
  const comoS1 = { actor: 'batalla-servidor:s1' };
  // Un asedio no se asigna hasta que acaba su convocatoria (Doc 5.15.1b): un tick basta.
  if (enConvocatoria(sesion.getState().batallas.find((b) => b.id === battleId)!, instanteDeTick(sesion.getState().tick))) sesion.avanzarTick();
  const base = { schemaVersion: SCHEMA_VERSION, battleId, ticketRevision: 0, intentoAsignacionId: 'intento-1' } as const;
  const instancia = { host: 'batalla-1.example', puerto: 7777, protocolo: 'udp' };
  sesion.ejecutar(registrarAsignacion, { servidorId: 's1', mensaje: { ...base, instancia, tokensParticipante: [] } }, comoS1);
  sesion.ejecutar(confirmarInicio, { servidorId: 's1', mensaje: base }, comoS1);
  const batalla = () => sesion.getState().batallas.find((b) => b.id === battleId)!;
  if (batalla().estado !== 'en_curso') throw new Error(`setup: la batalla no empezó (${batalla().estado})`);
  return batalla;
}

/** Un resultado completo y válido para la batalla: gana el atacante, cada escuadra pierde un efectivo. */
function resultado(b: Batalla, cambios: Partial<BattleResult> = {}): BattleResult {
  return {
    schemaVersion: SCHEMA_VERSION,
    battleId: b.id,
    resultId: 'resultado-1',
    ticketRevision: 0,
    intentoAsignacionId: 'intento-1',
    inicio: '2026-09-15T10:00:00Z',
    fin: '2026-09-15T10:05:00Z',
    ganador: 'atacante',
    razon: 'aniquilacion',
    objetivos: [],
    porEscuadra: escuadrasDe(b).map((s) => ({ squadId: s.squadId, desplegados: s.efectivosAutorizados, supervivientesAlCierre: s.efectivosAutorizados - 1, muertos: 1, xpGanada: 10 })),
    porHeroe: participacionesDe(b).map((p) => ({ heroeId: p.participante.heroeId, participo: true, sobrevivioAlCierre: true, xpGanada: 50 })),
    versionServidor: 'conquest-test',
    ...cambios,
  };
}

const aplicar = (sesion: GameSession, r: BattleResult, servidorId = 's1') =>
  sesion.ejecutar(aplicarResultado, { servidorId, mensaje: r }, { actor: `batalla-servidor:${servidorId}` });

/** `frenteACampamento` con una plaza de otra Facción, sin nadie dentro, justo donde están las dos columnas. */
function frenteAPlaza(unity = true) {
  const frente = frenteACampamento(unity);
  const payload = frente.sesion.exportar();
  const { state } = payload;
  const propia = state.asentamientos[0]!;
  const rival = { ...state.facciones[0]!, id: 'faccion-rival', nombre: 'Troya', ciudadanosIds: [] };
  const plaza = {
    ...propia,
    id: 'plaza-rival',
    faccionId: rival.id,
    heroesFundadoresIds: [],
    casasCompradas: [],
    posicion: state.ejercitos.find((e) => e.id === frente.columna)!.posicionActual,
    // Copia de una plaza recién fundada: sin la protección con la que nació (Doc 5.12.9).
    protegidaHasta: undefined,
  };
  const sesion = GameSession.importar(
    {
      ...payload,
      // Solo un ejército abre un asedio (Doc 5.15.1b).
      state: {
        ...state,
        ejercitos: state.ejercitos.map((e) => ({ ...e, tipo: 'ejercito' as const })),
        facciones: [...state.facciones, rival],
        asentamientos: [...state.asentamientos, plaza],
      },
    },
    { batallasEnUnity: unity }
  );
  return { ...frente, sesion, plazaId: plaza.id, faccionPropia: propia.faccionId };
}

/** La misma partida con el bando atacante de su batalla limitado a `capacidadMaxima` héroes. */
function conCapacidad(sesion: GameSession, capacidadMaxima: number): GameSession {
  const payload = sesion.exportar();
  const batallas = payload.state.batallas.map((b) => ({
    ...b,
    ticket: { ...b.ticket, bandos: { ...b.ticket.bandos, atacante: { ...b.ticket.bandos.atacante, capacidadMaxima } } },
  }));
  return GameSession.importar({ ...payload, state: { ...payload.state, batallas } });
}

describe('un héroe sale de la batalla (doc 02 §3.3, CQ-011, Doc 5.15.1)', () => {
  const comoS1 = { actor: 'batalla-servidor:s1' };
  const salida = (b: Batalla, heroeId: string, cambios: Record<string, unknown> = {}) => ({
    schemaVersion: SCHEMA_VERSION,
    battleId: b.id,
    ticketRevision: b.ticket.ticketRevision,
    intentoAsignacionId: 'intento-1',
    heroeId,
    motivo: 'abandono',
    derrotado: true,
    escuadras: [],
    ...cambios,
  });
  const sale = (sesion: GameSession, mensaje: unknown) => sesion.ejecutar(registrarSalida, { servidorId: 's1', mensaje: mensaje as never }, comoS1);
  const asignada = (sesion: GameSession, battleId: string) => {
    const base = { schemaVersion: SCHEMA_VERSION, battleId, ticketRevision: 0, intentoAsignacionId: 'intento-1' } as const;
    const instancia = { host: 'batalla-1.example', puerto: 7777, protocolo: 'udp' };
    sesion.ejecutar(registrarAsignacion, { servidorId: 's1', mensaje: { ...base, instancia, tokensParticipante: [] } }, comoS1);
    return () => sesion.getState().batallas.find((b) => b.id === battleId)!;
  };
  /** Dos héroes de la misma Facción contra el campamento: el fundador la abre y su vecino se une. */
  function dosContraElCampamento() {
    const frente = frenteACampamento();
    const battleId = atacarCampamento(frente.sesion, frente.fundador).datos!.battleId;
    expect(frente.sesion.ejecutar(REGISTRO_COMANDOS.unirseABatalla, { heroeId: frente.vecino, battleId }, { actor: frente.vecino }).ok).toBe(true);
    return { ...frente, battleId };
  }

  it('antes de empezar, quien no se conecta sale sin castigo: sube la revisión, la asignación vale de nuevo y su columna queda libre', () => {
    const { sesion, fundador, vecino, battleId, columnaVecino } = dosContraElCampamento();
    const batalla = asignada(sesion, battleId);
    expect(batalla().estado).toBe('asignada');

    expect(sale(sesion, salida(batalla(), vecino, { motivo: 'no_conectado', derrotado: false })).ok).toBe(true);

    const b = batalla();
    expect(b.estado).toBe('convocando');
    expect(b.ticket.ticketRevision).toBe(1);
    expect(b.asignacion, 'la asignación anterior ya no vale').toBeUndefined();
    expect(participacionesDe(b).map((p) => p.participante.heroeId)).toEqual([fundador]);
    expect(reservadas(sesion, vecino), 'sus escuadras vuelven a ser suyas').toEqual([]);
    expect(reservadas(sesion, fundador).length).toBeGreaterThan(0);
    const estado = sesion.getState();
    expect(heridosEn(estado.heroes, instanteDeTick(estado.tick)).has(vecino), 'sin castigo').toBe(false);
    expect(b.bloqueo.ejercitoIds).not.toContain(columnaVecino);
  });

  it('con la revisión nueva se reasigna, y una asignación de la revisión vieja se rechaza', () => {
    const { sesion, vecino, battleId } = dosContraElCampamento();
    const batalla = asignada(sesion, battleId);
    sale(sesion, salida(batalla(), vecino, { motivo: 'no_conectado', derrotado: false }));
    const asignar = (ticketRevision: number, intentoAsignacionId: string) =>
      sesion.ejecutar(
        registrarAsignacion,
        {
          servidorId: 's1',
          mensaje: {
            schemaVersion: SCHEMA_VERSION,
            battleId,
            ticketRevision,
            intentoAsignacionId,
            instancia: { host: 'batalla-2.example', puerto: 7777, protocolo: 'udp' },
            tokensParticipante: [],
          },
        },
        comoS1
      );

    expect(asignar(0, 'intento-2').ok, 'la revisión vieja ya no vale').toBe(false);
    expect(asignar(1, 'intento-2').ok).toBe(true);
    expect(batalla().estado).toBe('asignada');
  });

  it('repetir la salida no cambia nada, y la de quien no combata en ella se rechaza', () => {
    const { sesion, vecino, battleId } = dosContraElCampamento();
    const batalla = asignada(sesion, battleId);
    const mensaje = salida(batalla(), vecino, { motivo: 'no_conectado', derrotado: false });
    sale(sesion, mensaje);
    const antes = sesion.getState();

    expect(sale(sesion, { ...mensaje, ticketRevision: 1 }).ok, 'ya está registrada').toBe(true);
    expect(sesion.getState().batallas[0]).toEqual(antes.batallas[0]);
    expect(sale(sesion, salida(batalla(), 'forastero', { motivo: 'no_conectado', derrotado: false })).ok).toBe(false);
  });

  it('no_conectado solo vale antes de empezar, y el resto de motivos solo con la partida en curso', () => {
    const { sesion, vecino, battleId } = dosContraElCampamento();
    const batalla = asignada(sesion, battleId);

    expect(sale(sesion, salida(batalla(), vecino, { motivo: 'abandono' })).ok, 'asignada todavía no es en curso').toBe(false);

    sesion.ejecutar(confirmarInicio, { servidorId: 's1', mensaje: { schemaVersion: SCHEMA_VERSION, battleId, ticketRevision: 0, intentoAsignacionId: 'intento-1' } }, comoS1);
    expect(sale(sesion, salida(batalla(), vecino, { motivo: 'no_conectado', derrotado: false })).ok, 'ya empezó').toBe(false);
  });

  it('si se queda sin héroes humanos, la batalla se cancela sin castigo', () => {
    const { sesion, fundador, vecino } = frenteACampamento();
    const battleId = atacarCampamento(sesion, fundador).datos!.battleId;
    const batalla = asignada(sesion, battleId);
    expect(vecino).toBeDefined();

    expect(sale(sesion, salida(batalla(), fundador, { motivo: 'no_conectado', derrotado: false })).ok).toBe(true);

    expect(batalla().estado).toBe('cancelada');
    expect(reservadas(sesion, fundador)).toEqual([]);
  });

  it('en curso, quien sale derrotado queda herido con lo que le quedó, su columna se libera y el resultado ya no lo repite', () => {
    const { sesion, fundador, vecino, battleId, columnaVecino } = dosContraElCampamento();
    const batalla = empezar(sesion, battleId);
    const escuadra = batalla().incorporaciones[0]!.participante.escuadras[0]!;
    const quedan = Math.floor(escuadra.efectivosAutorizados / 2);

    const r = sale(sesion, salida(batalla(), vecino, { escuadras: [{ squadId: escuadra.squadId, supervivientes: quedan, muertos: escuadra.efectivosAutorizados - quedan }] }));

    expect(r.ok).toBe(true);
    const estado = sesion.getState();
    expect(heridosEn(estado.heroes, instanteDeTick(estado.tick)).has(vecino)).toBe(true);
    expect(estado.heroes.find((h) => h.id === vecino)!.escuadrones.find((e) => e.id === escuadra.squadId)!.cantidad).toBe(quedan);
    expect(batalla().ticket.ticketRevision, 'el ticket no cambia en curso').toBe(0);
    expect(batalla().bloqueo.ejercitoIds).not.toContain(columnaVecino);
    expect(reservadas(sesion, vecino)).toEqual([]);

    const final = resultado(batalla());
    expect(final.porHeroe.map((h) => h.heroeId)).toEqual([fundador]);
    expect(aplicar(sesion, final).ok).toBe(true);
    expect(heridosEn(sesion.getState().heroes, instanteDeTick(sesion.getState().tick)).has(fundador), 'el ganador no queda herido').toBe(false);
  });

  it('en curso, una escuadra que no cierra sobre sus efectivos o que no es suya rechaza la salida entera', () => {
    const { sesion, fundador, vecino, battleId } = dosContraElCampamento();
    const batalla = empezar(sesion, battleId);
    const suya = batalla().incorporaciones[0]!.participante.escuadras[0]!;
    const ajena = batalla().ticket.bandos.atacante.participantes[0]!.escuadras[0]!;

    expect(sale(sesion, salida(batalla(), vecino, { escuadras: [{ squadId: suya.squadId, supervivientes: 1, muertos: 1 }] })).ok).toBe(false);
    expect(sale(sesion, salida(batalla(), vecino, { escuadras: [{ squadId: ajena.squadId, supervivientes: ajena.efectivosAutorizados, muertos: 0 }] })).ok).toBe(false);
    expect(participacionesDe(batalla()).map((p) => p.participante.heroeId).sort(), 'nadie salió').toEqual([fundador, vecino].sort());
  });

  it('la herida sale de `derrotado`: en el resultado, el ganador puede quedar herido y el perdedor no', () => {
    const { sesion, fundador, vecino, battleId } = dosContraElCampamento();
    const batalla = empezar(sesion, battleId);
    const r = resultado(batalla(), { ganador: 'defensor', razon: 'tiempo_agotado' });
    const conHerida = { ...r, porHeroe: r.porHeroe.map((h) => ({ ...h, derrotado: h.heroeId === vecino })) };

    expect(aplicar(sesion, conHerida).ok).toBe(true);

    const estado = sesion.getState();
    const heridos = heridosEn(estado.heroes, instanteDeTick(estado.tick));
    expect(heridos.has(vecino)).toBe(true);
    expect(heridos.has(fundador), 'perdió pero Unity no lo da por derrotado').toBe(false);
  });
});
