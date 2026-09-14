// Fixtures dorados del contrato v1. Son la fuente de `fixtures/*.json` (los que lee Conquest): el test los valida
// contra el schema y comprueba que el JSON publicado es idéntico. Para regenerarlos tras un cambio:
// `ACTUALIZAR_CONTRATOS=1 npx vitest run src/contratos`.
import {
  SCHEMA_VERSION,
  type Atributos,
  type BattleParticipantSnapshot,
  type BattleResult,
  type BattleRules,
  type BattleServerAssignment,
  type BattleTicket,
  type Equipamiento,
  type EscuadronDto,
  type HeroeDto,
  type HeroePublicoDto,
  type InicioBatalla,
  type ProgresionEscuadra,
  type SquadSnapshot,
} from './dto';

const GAME_ID = 'partida-demo';
const BATALLA_ASEDIO = '3f2a9c1e-7b4d-4e8a-9f6c-2d1b0a5e8c73';
const BATALLA_BANDIDOS = '9d4e6b2a-1c3f-4a5b-8e7d-6f0a2b4c8d1e';
const INTENTO = 'intento-5b1e';
/** Instante de mundo hasta el que quedan heridos los que perdieron el asedio (Doc 5.16.4). */
const HERIDO_HASTA = 1_789_410_720_000;

const VERSIONES = {
  versionBalance: 9,
  versionCatalogoTropas: 1,
  versionCatalogoHeroe: 'conquest-heroes-1',
  versionCatalogoObjetos: 'conquest-items-1',
} satisfies Partial<BattleRules>;

function snapshotDe(e: EscuadronDto): SquadSnapshot {
  return {
    squadId: e.id,
    heroeId: e.heroeId,
    tropaId: e.tropaId,
    efectivosAutorizados: e.cantidad,
    nivel: e.nivel,
    experiencia: e.experiencia,
    moral: e.moral,
    habilidadesDesbloqueadas: e.habilidadesDesbloqueadas,
    formacionesDesbloqueadas: e.formacionesDesbloqueadas,
    formacionSeleccionada: e.formacionSeleccionada,
  };
}

/** Escuadra del ticket escrita a mano. Sin `progresion`, recién reclutada: así van bandidos y carreteros. */
function escuadra(
  squadId: string,
  heroeId: string | null,
  tropaId: string,
  efectivos: number,
  progresion: Partial<ProgresionEscuadra> = {},
): SquadSnapshot {
  return {
    squadId,
    heroeId,
    tropaId,
    efectivosAutorizados: efectivos,
    nivel: 1,
    experiencia: 0,
    moral: 100,
    habilidadesDesbloqueadas: [],
    formacionesDesbloqueadas: [0],
    formacionSeleccionada: 0,
    ...progresion,
  };
}

function enBatalla(
  h: HeroeDto,
  casillasInventarioLibres: number,
  escuadras: SquadSnapshot[],
  atributosEfectivos: Atributos = h.atributosBase,
): BattleParticipantSnapshot {
  const { displayName, classDefinitionId, nivel, genero, avatar, perksDesbloqueados, equipamiento } = h;
  return {
    heroeId: h.id,
    controlador: h.controlador,
    heroe: {
      displayName,
      classDefinitionId,
      nivel,
      genero,
      avatar,
      atributosEfectivos,
      perksDesbloqueados,
      equipamiento,
      casillasInventarioLibres,
    },
    escuadras,
  };
}

/** Héroe humano en una columna: dos escuadras reservadas para el asedio y una en la guarnición de su residencia. */
export const HEROE: HeroeDto = {
  schemaVersion: SCHEMA_VERSION,
  id: 'heroe-ana',
  jugadorId: 'jugador-1',
  controlador: 'humano',
  displayName: 'Ana de Micenas',
  classDefinitionId: 'guerrero',
  genero: 'femenino',
  avatar: { cabezaId: 'cabeza_03', peloId: 'pelo_07', barbaId: '', cejasId: 'cejas_02' },
  nivel: 4,
  experienciaHaciaSiguienteNivel: 120,
  puntosDeAtributoSinGastar: 2,
  puntosDePerkSinGastar: 1,
  atributosBase: { fuerza: 14, destreza: 11, armadura: 9, vitalidad: 12 },
  perksDesbloqueados: [101, 104],
  liderazgoBase: 100,
  ubicacion: { tipo: 'columna', ejercitoId: 'ejercito-7' },
  escuadrones: [
    {
      id: 'escuadron-11',
      heroeId: 'heroe-ana',
      tropaId: 'espadachines_cobre',
      nombre: 'Espadas de Ana',
      cantidad: 20,
      nivel: 3,
      experiencia: 640,
      moral: 92,
      habilidadesDesbloqueadas: ['carga'],
      formacionesDesbloqueadas: [0, 1],
      formacionSeleccionada: 1,
      contenedor: { tipo: 'ejercito', ejercitoId: 'ejercito-7' },
      enGuarnicion: false,
      reservaBatalla: { battleId: BATALLA_ASEDIO },
    },
    {
      id: 'escuadron-12',
      heroeId: 'heroe-ana',
      tropaId: 'honderos',
      nombre: 'Honderos',
      cantidad: 20,
      nivel: 2,
      experiencia: 210,
      moral: 100,
      habilidadesDesbloqueadas: [],
      formacionesDesbloqueadas: [0],
      formacionSeleccionada: 0,
      contenedor: { tipo: 'ejercito', ejercitoId: 'ejercito-7' },
      enGuarnicion: false,
      reservaBatalla: { battleId: BATALLA_ASEDIO },
    },
    {
      id: 'escuadron-13',
      heroeId: 'heroe-ana',
      tropaId: 'milicia_lanceros',
      nombre: 'Milicia',
      cantidad: 25,
      nivel: 1,
      experiencia: 0,
      moral: 100,
      habilidadesDesbloqueadas: [],
      formacionesDesbloqueadas: [0],
      formacionSeleccionada: 0,
      contenedor: { tipo: 'campamento' },
      enGuarnicion: true,
    },
  ],
  loadouts: [
    {
      id: 'loadout-1',
      heroeId: 'heroe-ana',
      displayName: 'Asalto',
      squadIds: ['escuadron-11', 'escuadron-12'],
      perksSeleccionados: [101],
      activo: true,
      liderazgoTotal: 28,
    },
  ],
  inventario: [
    { itemDefinitionId: 'pocion_curacion', tipo: 'consumible', cantidad: 3, precio: 5, casillaInventario: 0 },
    {
      itemDefinitionId: 'casco_cuero',
      tipo: 'armadura',
      cantidad: 1,
      itemInstanceId: 'item-0003',
      estadisticas: [{ nombre: 'armadura', valor: 3 }],
      precio: 12,
      casillaInventario: 1,
    },
  ],
  equipamiento: {
    arma: {
      itemDefinitionId: 'espada_bronce',
      tipo: 'arma',
      cantidad: 1,
      itemInstanceId: 'item-0001',
      estadisticas: [{ nombre: 'dano', valor: 12.5 }],
      precio: 40,
      casillaInventario: -1,
    },
    casco: null,
    torso: {
      itemDefinitionId: 'coraza_bronce',
      tipo: 'armadura',
      cantidad: 1,
      itemInstanceId: 'item-0002',
      estadisticas: [{ nombre: 'armadura', valor: 8 }],
      precio: 55,
      casillaInventario: -1,
    },
    guantes: null,
    pantalones: null,
    botas: null,
  },
  monedasHeroe: { bronce: 340, plata: 12, oro: 1 },
  cupoGuarnicion: 36,
};

const EQUIPO_BOT: Equipamiento = {
  arma: {
    itemDefinitionId: 'lanza_bronce',
    tipo: 'arma',
    cantidad: 1,
    itemInstanceId: 'item-0200',
    estadisticas: [{ nombre: 'dano', valor: 9 }],
    precio: 30,
    casillaInventario: -1,
  },
  casco: null,
  torso: null,
  guantes: null,
  pantalones: null,
  botas: null,
};

/**
 * Héroe bot DESPUÉS de `RESULTADO_ASEDIO`: sin jugador, herido y fuera, en columna, con lo que le quedó de la
 * escuadra que usó (Doc 5.15.5): 6 supervivientes y 150 + 60 de experiencia.
 */
export const HEROE_BOT: HeroeDto = {
  schemaVersion: SCHEMA_VERSION,
  id: 'heroe-bot-1',
  jugadorId: null,
  controlador: 'bot',
  displayName: 'Capitán de Tirinto',
  classDefinitionId: 'guerrero',
  genero: 'masculino',
  avatar: { cabezaId: 'cabeza_01', peloId: 'pelo_02', barbaId: 'barba_04', cejasId: 'cejas_01' },
  nivel: 3,
  experienciaHaciaSiguienteNivel: 40,
  puntosDeAtributoSinGastar: 0,
  puntosDePerkSinGastar: 0,
  atributosBase: { fuerza: 13, destreza: 10, armadura: 12, vitalidad: 11 },
  perksDesbloqueados: [],
  liderazgoBase: 100,
  ubicacion: { tipo: 'columna', ejercitoId: 'ejercito-15' },
  heridoHasta: HERIDO_HASTA,
  escuadrones: [
    {
      id: 'escuadron-40',
      heroeId: 'heroe-bot-1',
      tropaId: 'lanceros_mimbre',
      nombre: 'Lanceros de Tirinto',
      cantidad: 6,
      nivel: 2,
      experiencia: 210,
      moral: 100,
      habilidadesDesbloqueadas: [],
      formacionesDesbloqueadas: [0],
      formacionSeleccionada: 0,
      contenedor: { tipo: 'ejercito', ejercitoId: 'ejercito-15' },
      enGuarnicion: false,
    },
  ],
  loadouts: [],
  inventario: [],
  equipamiento: EQUIPO_BOT,
  monedasHeroe: { bronce: 0, plata: 0, oro: 0 },
  cupoGuarnicion: 0,
};

/** El mismo héroe bot visto por otro jugador que se cruza con su columna (doc 02 §4.1). */
export const HEROE_PUBLICO: HeroePublicoDto = {
  heroeId: HEROE_BOT.id,
  displayName: HEROE_BOT.displayName,
  classDefinitionId: HEROE_BOT.classDefinitionId,
  nivel: HEROE_BOT.nivel,
  heridoHasta: HERIDO_HASTA,
  escuadrasQueLleva: [{ tropaId: 'lanceros_mimbre', cantidad: 6, nivel: 2 }],
  equipamiento: { arma: 'lanza_bronce', casco: null, torso: null, guantes: null, pantalones: null, botas: null },
};

/** Asedio: héroe humano contra un héroe bot y la guarnición de un asentamiento NPC (CQ-002). */
export const TICKET_ASEDIO: BattleTicket = {
  schemaVersion: SCHEMA_VERSION,
  battleId: BATALLA_ASEDIO,
  gameId: GAME_ID,
  ticketRevision: 0,
  contextoEstrategico: { tipo: 'asedio', asentamientoId: 'asentamiento-9' },
  bandos: {
    atacante: {
      faccionId: 'faccion-1',
      capacidadMaxima: 15,
      participantes: [
        enBatalla(
          HEROE,
          22,
          HEROE.escuadrones.filter((e) => e.reservaBatalla?.battleId === BATALLA_ASEDIO).map(snapshotDe),
          { fuerza: 16, destreza: 11, armadura: 17, vitalidad: 12 },
        ),
      ],
      escuadrasSinHeroe: [],
    },
    defensor: {
      faccionId: 'faccion-npc-2',
      capacidadMaxima: 15,
      participantes: [
        enBatalla(HEROE_BOT, 24, [escuadra('escuadron-40', HEROE_BOT.id, 'lanceros_mimbre', 25, { nivel: 2, experiencia: 150 })]),
      ],
      escuadrasSinHeroe: [escuadra('escuadron-41', 'heroe-bot-2', 'milicia_lanceros', 25)],
    },
  },
  mapa: {
    tipo: 'asentamiento',
    settlementId: 'asentamiento-9',
    layoutVersion: 2,
    unidadesPorCelda: 3,
    edificios: [
      { edificioId: 'edificio-1', tipo: 'centroUrbano', posicion: { x: 0, y: 0 }, ancho: 4, alto: 4, estado: 'activo' },
      { edificioId: 'edificio-2', tipo: 'vivienda', posicion: { x: 10.5, y: -1.5 }, ancho: 1, alto: 1, estado: 'activo' },
      {
        edificioId: 'edificio-3',
        tipo: 'vivienda',
        posicion: { x: 10.5, y: 1.5 },
        ancho: 1,
        alto: 1,
        estado: 'en_cola',
        danado: true,
      },
      {
        edificioId: 'edificio-4',
        tipo: 'barracon',
        posicion: { x: -12, y: 0 },
        ancho: 2,
        alto: 2,
        nivelInterno: 2,
        estado: 'activo',
      },
    ],
    // Empalizada a medio levantar: solo viajan las cinco primeras celdas del anillo, las que ya están en pie.
    recintos: [
      {
        recintoId: 'recinto-1',
        nivel: 1,
        celdas: [
          { col: -1, row: -5, clase: 'puerta' },
          { col: 0, row: -5, clase: 'muro' },
          { col: 1, row: -5, clase: 'muro' },
          { col: 2, row: -5, clase: 'torre' },
          { col: 3, row: -5, clase: 'muro' },
        ],
      },
    ],
  },
  reglas: { duracionMaximaSegundos: 1800, ganadorPorTiempo: 'defensor', ...VERSIONES },
};

/** Héroe humano contra un campamento de bandidos: el bando defensor no es de nadie y lo maneja la IA (Doc 1.9). */
export const TICKET_BANDIDOS: BattleTicket = {
  schemaVersion: SCHEMA_VERSION,
  battleId: BATALLA_BANDIDOS,
  gameId: GAME_ID,
  ticketRevision: 0,
  contextoEstrategico: { tipo: 'campamento_bandidos', campamentoId: 'bandidos-3', punto: { x: 812, y: 1340 } },
  bandos: {
    atacante: {
      faccionId: 'faccion-1',
      capacidadMaxima: 5,
      participantes: [
        {
          heroeId: 'heroe-bruno',
          controlador: 'humano',
          heroe: {
            displayName: 'Bruno',
            classDefinitionId: 'arquero',
            nivel: 2,
            genero: 'masculino',
            avatar: { cabezaId: 'cabeza_05', peloId: 'pelo_01', barbaId: 'barba_02', cejasId: 'cejas_03' },
            atributosEfectivos: { fuerza: 9, destreza: 14, armadura: 6, vitalidad: 10 },
            perksDesbloqueados: [],
            equipamiento: { arma: null, casco: null, torso: null, guantes: null, pantalones: null, botas: null },
            casillasInventarioLibres: 30,
          },
          escuadras: [escuadra('escuadron-20', 'heroe-bruno', 'lanceros_mimbre', 25, { experiencia: 40 })],
        },
      ],
      escuadrasSinHeroe: [],
    },
    defensor: {
      faccionId: null,
      capacidadMaxima: 5,
      participantes: [],
      escuadrasSinHeroe: [escuadra('bandidos-3-tropa', null, 'milicia_lanceros', 15)],
    },
  },
  mapa: { tipo: 'mapa', mapaId: 'mapa-principal', centro: { x: 812, y: 1340 } },
  // `ganadorPorTiempo` fuera del asedio está pendiente de decidir (Mecánicas a desarrollar): valor de ejemplo.
  reglas: { duracionMaximaSegundos: 900, ganadorPorTiempo: 'defensor', ...VERSIONES },
};

export const ASIGNACION: BattleServerAssignment = {
  schemaVersion: SCHEMA_VERSION,
  battleId: BATALLA_ASEDIO,
  ticketRevision: 0,
  intentoAsignacionId: INTENTO,
  instancia: { host: 'batalla-1.conquest.example', puerto: 7777, protocolo: 'udp' },
  tokensParticipante: [{ heroeId: 'heroe-ana', token: 'token-de-ejemplo', expiraEn: '2026-09-14T18:30:00Z' }],
};

export const INICIO: InicioBatalla = {
  schemaVersion: SCHEMA_VERSION,
  battleId: BATALLA_ASEDIO,
  ticketRevision: 0,
  intentoAsignacionId: INTENTO,
};

/** Resultado de `TICKET_ASEDIO`: cierra la conservación de cada escuadra y trae botín para quien participó. */
export const RESULTADO_ASEDIO: BattleResult = {
  schemaVersion: SCHEMA_VERSION,
  battleId: BATALLA_ASEDIO,
  resultId: 'resultado-8c2f',
  ticketRevision: 0,
  intentoAsignacionId: INTENTO,
  inicio: '2026-09-14T18:05:00Z',
  fin: '2026-09-14T18:26:40Z',
  ganador: 'atacante',
  razon: 'objetivos_capturados',
  objetivos: [
    { objetivoId: 'bandera-puerta', capturadoPor: 'atacante' },
    { objetivoId: 'bandera-plaza', capturadoPor: 'atacante' },
  ],
  porEscuadra: [
    { squadId: 'escuadron-11', desplegados: 20, supervivientesAlCierre: 13, muertos: 7, xpGanada: 180 },
    { squadId: 'escuadron-12', desplegados: 20, supervivientesAlCierre: 18, muertos: 2, xpGanada: 95 },
    { squadId: 'escuadron-40', desplegados: 25, supervivientesAlCierre: 6, muertos: 19, xpGanada: 60 },
    { squadId: 'escuadron-41', desplegados: 25, supervivientesAlCierre: 0, muertos: 25, xpGanada: 20 },
  ],
  porHeroe: [
    {
      heroeId: 'heroe-ana',
      participo: true,
      sobrevivioAlCierre: true,
      xpGanada: 340,
      botin: {
        objetos: [
          { itemDefinitionId: 'botas_cuero', cantidad: 1, itemInstanceId: 'item-0107', estadisticas: [{ nombre: 'armadura', valor: 2 }] },
          { itemDefinitionId: 'pocion_curacion', cantidad: 2 },
        ],
        monedas: { bronce: 85, plata: 3, oro: 0 },
      },
    },
    { heroeId: 'heroe-bot-1', participo: true, sobrevivioAlCierre: false, xpGanada: 40 },
  ],
  versionServidor: 'conquest-server-0.9.3',
};
