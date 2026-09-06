// Esquema de `params` por comando (Fase C9, doc 4: "Hoy `ESQUEMA_EJECUTAR_COMANDO` declara `params: {}` — el
// contrato publicado tiene el agujero justo donde un cliente externo lo necesita, y un `params` malformado
// revienta dentro del manejador y sale como 409 en vez de 400").
//
// Exhaustividad garantizada en COMPILACIÓN por `Record<TipoComando, EsquemaJson>`, mismo mecanismo que
// `MATRIZ_AUTORIZACION` en `autorizacion.ts`: falta un comando o sobra uno con el nombre mal escrito y esto no
// compila.
//
// Vive en `session/` y no en `server/`, mismo criterio que `autorizacion.ts`: la FORMA de `params` de cada
// comando es parte de su contrato de negocio (cambia cuando cambia el comando, no cuando cambia Fastify), no
// un detalle de la capa HTTP.
//
// Solo valida FORMA (campos presentes, tipo JS correcto, enteros/cadenas dentro de un catálogo cerrado
// conocido en compilación) — nunca EXISTENCIA de una entidad (`faccionId` real, `asentamientoId` real): eso
// sigue siendo del comando (`*.no_existe`, `ayudas.ts`), que ya lo hace bien y no debe duplicarse aquí. La
// frontera es la misma que ya traza el doc 2: forma en el borde, reglas de dominio en el motor.
import { CARGOS_TIPO, EDIFICIOS_TIPO, RECURSOS_TIPO } from '../../domain/types';
import { CARGOS_CONSTRUCTOR } from './construccion';
import type { TipoComando } from './registro';

/** JSON Schema de un `params`, tal y como lo consume Fastify/ajv. Sin tipo más estricto que `object` a
 * propósito: es el mismo nivel de precisión con el que ya se tipan el resto de esquemas de este proyecto
 * (`server/rutas/esquemas.ts`) — un tipo de JSON Schema completo es una dependencia que no aporta nada aquí. */
type EsquemaJson = Record<string, unknown>;

const IDENTIFICADOR = { type: 'string', minLength: 1 } as const;
const LISTA_DE_IDENTIFICADORES = { type: 'array', items: IDENTIFICADOR } as const;
// Sin `minimum` a propósito: `cantidad <= 0` ya es un rechazo de dominio con su propio código
// (`TruequeInvalidoError`/`OrdenInvalidaError` en `engine/trade.ts`/`engine/market.ts`) — poner un límite aquí
// lo convertiría en 400 para negativos y en 200/ok:false para cero, dos formas de la MISMA regla. Lección de
// `nombre` más abajo: cuando el dominio ya tiene un código para un valor "vacío"/"fuera de rango", este
// archivo no debe adelantarse a decidirlo.
const NUMERO = { type: 'number' } as const;
const PUNTO = {
  type: 'object',
  properties: { x: { type: 'number' }, y: { type: 'number' } },
  required: ['x', 'y'],
  additionalProperties: false,
} as const;
const RECURSO = { type: 'string', enum: RECURSOS_TIPO } as const;
// A dónde va una columna: un asentamiento por id, o un punto del mapa. Se valida con `oneOf` para que un
// cliente no pueda colar un punto sin coordenadas ni un destino sin id. Lo comparten `movilizarEjercito`
// (fijarlo al salir) y `marcharA` (rectificarlo cuantas veces quiera un viajero solo).
const OBJETIVO_DE_INTERACCION = {
  oneOf: [
    { type: 'object', properties: { tipo: { type: 'string', enum: ['ejercito'] }, id: IDENTIFICADOR }, required: ['tipo', 'id'], additionalProperties: false },
    { type: 'object', properties: { tipo: { type: 'string', enum: ['caravana'] }, id: IDENTIFICADOR }, required: ['tipo', 'id'], additionalProperties: false },
  ],
} as const;
const OBJETIVO_EJERCITO = {
  oneOf: [
    { type: 'object', properties: { tipo: { type: 'string', enum: ['asentamiento'] }, id: IDENTIFICADOR }, required: ['tipo', 'id'], additionalProperties: false },
    { type: 'object', properties: { tipo: { type: 'string', enum: ['punto'] }, punto: PUNTO }, required: ['tipo', 'punto'], additionalProperties: false },
  ],
} as const;
const CARGO_FACCION = { type: 'string', enum: CARGOS_TIPO } as const;
const CARGO_CONSTRUCTOR = { type: 'string', enum: CARGOS_CONSTRUCTOR } as const;

/** Fábrica de `{type:'object', properties, required, additionalProperties:false}` — evita repetir las tres
 * líneas de cierre en cada una de las 30 entradas de abajo. */
function objeto(properties: Record<string, unknown>, required: readonly string[]): EsquemaJson {
  return { type: 'object', properties, required: [...required], additionalProperties: false };
}

export const ESQUEMAS_PARAMS: Record<TipoComando, EsquemaJson> = {
  // --- Fundación y expansión ---
  fundarAsentamiento: objeto({ faccionId: IDENTIFICADOR, posicion: PUNTO }, ['faccionId', 'posicion']),
  lanzarCaravanaFundacion: objeto(
    { origenAsentamientoId: IDENTIFICADOR, destino: PUNTO, numJugadores: NUMERO },
    ['origenAsentamientoId', 'destino', 'numJugadores']
  ),
  desarmarCaravanaFundacion: objeto({ caravanaId: IDENTIFICADOR }, ['caravanaId']),

  // Sin `minLength` en `nombre`: vacío/solo-espacios ya es un rechazo de dominio con su propio código
  // (`faccion.nombre_vacio`, `crearFaccion.ts`) — mismo motivo que `NUMERO` más arriba.
  crearFaccion: objeto({ nombre: { type: 'string' } }, ['nombre']),
  unirseAFaccion: objeto({ faccionId: IDENTIFICADOR }, ['faccionId']),
  // Sin parámetros: el actor solo puede dejar SU PROPIA Facción — `objeto({}, [])` solo admite `{}`.
  dejarFaccion: objeto({}, []),

  // --- Cargos ---
  alternarFaccionNpc: objeto({ faccionId: IDENTIFICADOR, activo: { type: 'boolean' } }, ['faccionId', 'activo']),
  asignarRey: objeto({ faccionId: IDENTIFICADOR, jugadorId: IDENTIFICADOR }, ['faccionId', 'jugadorId']),
  asignarEmbajador: objeto({ faccionId: IDENTIFICADOR, jugadorId: IDENTIFICADOR }, ['faccionId', 'jugadorId']),
  asignarCargoLocal: objeto(
    { asentamientoId: IDENTIFICADOR, cargo: CARGO_FACCION, jugadorId: IDENTIFICADOR },
    ['asentamientoId', 'cargo', 'jugadorId']
  ),
  comprarCasa: objeto({ asentamientoId: IDENTIFICADOR, jugadorId: IDENTIFICADOR }, ['asentamientoId', 'jugadorId']),

  // --- Construcción y gestión local ---
  activarPolitica: objeto(
    { asentamientoId: IDENTIFICADOR, cargo: CARGO_FACCION, politicaId: IDENTIFICADOR },
    ['asentamientoId', 'cargo', 'politicaId']
  ),
  anadirEdificioManualmente: objeto(
    { asentamientoId: IDENTIFICADOR, cargo: CARGO_CONSTRUCTOR, tipo: { type: 'string', enum: EDIFICIOS_TIPO } },
    ['asentamientoId', 'cargo', 'tipo']
  ),
  quitarDeCola: objeto(
    { asentamientoId: IDENTIFICADOR, cargo: CARGO_CONSTRUCTOR, edificioId: IDENTIFICADOR },
    ['asentamientoId', 'cargo', 'edificioId']
  ),
  moverEnCola: objeto(
    {
      asentamientoId: IDENTIFICADOR,
      cargo: CARGO_CONSTRUCTOR,
      edificioId: IDENTIFICADOR,
      direccion: { type: 'string', enum: ['arriba', 'abajo'] },
    },
    ['asentamientoId', 'cargo', 'edificioId', 'direccion']
  ),
  mejorarEdificioAhora: objeto(
    { asentamientoId: IDENTIFICADOR, cargo: CARGO_CONSTRUCTOR, edificioId: IDENTIFICADOR },
    ['asentamientoId', 'cargo', 'edificioId']
  ),
  alternarAutoConstruccion: objeto({ asentamientoId: IDENTIFICADOR, pausada: { type: 'boolean' } }, ['asentamientoId', 'pausada']),
  alternarReabastecerAliados: objeto({ asentamientoId: IDENTIFICADOR, permitido: { type: 'boolean' } }, ['asentamientoId', 'permitido']),
  adjuntarCaravana: objeto({ ejercitoId: IDENTIFICADOR, caravanaId: IDENTIFICADOR, jugadorId: IDENTIFICADOR }, [
    'ejercitoId',
    'caravanaId',
    'jugadorId',
  ]),
  soltarCaravana: objeto({ ejercitoId: IDENTIFICADOR, caravanaId: IDENTIFICADOR, jugadorId: IDENTIFICADOR }, [
    'ejercitoId',
    'caravanaId',
    'jugadorId',
  ]),
  cargarCaravana: objeto(
    { ejercitoId: IDENTIFICADOR, caravanaId: IDENTIFICADOR, asentamientoId: IDENTIFICADOR, recurso: RECURSO, cantidad: NUMERO },
    ['ejercitoId', 'caravanaId', 'asentamientoId', 'recurso', 'cantidad']
  ),
  entregarDeCaravana: objeto({ ejercitoId: IDENTIFICADOR, caravanaId: IDENTIFICADOR, acuerdoId: IDENTIFICADOR }, [
    'ejercitoId',
    'caravanaId',
    'acuerdoId',
  ]),
  calibrarReservaManual: objeto({ asentamientoId: IDENTIFICADOR, recurso: RECURSO, valor: NUMERO }, [
    'asentamientoId',
    'recurso',
    'valor',
  ]),
  // Sin `minLength` en `nombre`: vacío es un valor legítimo aquí ("volver a mostrar el id",
  // `renombrarAsentamiento`, `construccion.ts`) — no un error de forma.
  renombrarAsentamiento: objeto({ asentamientoId: IDENTIFICADOR, nombre: { type: 'string' } }, ['asentamientoId', 'nombre']),

  // --- Murallas (Consideraciones/Murallas_Definicion.md, Paso 2c) ---
  comprometerRecinto: objeto(
    { asentamientoId: IDENTIFICADOR, cargo: CARGO_CONSTRUCTOR, nivel: { type: 'integer', enum: [1, 2, 3] } },
    ['asentamientoId', 'cargo', 'nivel']
  ),
  // Sin `cargo`: abandonar es solo del Gobernador (§8 del doc), no un `CargoConstructor` cualquiera.
  abandonarRecinto: objeto({ asentamientoId: IDENTIFICADOR, recintoId: IDENTIFICADOR }, ['asentamientoId', 'recintoId']),
  mejorarRecinto: objeto(
    { asentamientoId: IDENTIFICADOR, cargo: CARGO_CONSTRUCTOR, recintoId: IDENTIFICADOR },
    ['asentamientoId', 'cargo', 'recintoId']
  ),

  // --- Diplomacia ---
  proponerRelacion: {
    type: 'object',
    properties: {
      tipo: { type: 'string', enum: ['vasallaje', 'alianza'] },
      faccionAId: IDENTIFICADOR,
      faccionBId: IDENTIFICADOR,
      // Solo tienen sentido en vasallaje; el comando los ignora en alianzas (ver `diplomacia.ts`), así que
      // aquí siguen siendo opcionales en vez de condicionar el esquema al valor de `tipo` — un `if/then` de
      // JSON Schema por este único caso no paga su complejidad.
      tributoRecurso: RECURSO,
      tributoCantidad: NUMERO,
    },
    required: ['tipo', 'faccionAId', 'faccionBId'],
    additionalProperties: false,
  },
  romperRelacion: objeto({ relacionId: IDENTIFICADOR, iniciadorFaccionId: IDENTIFICADOR }, ['relacionId', 'iniciadorFaccionId']),
  rebelionVasallo: objeto({ relacionId: IDENTIFICADOR }, ['relacionId']),
  anexionar: objeto({ faccionAId: IDENTIFICADOR, faccionBId: IDENTIFICADOR }, ['faccionAId', 'faccionBId']),
  // Sin `minLength` en `nuevoNombre`: vacío cae a un nombre por defecto (`params.nuevoNombre || 'Facción
  // Fusionada'`, `diplomacia.ts`) — uso soportado, no un error de forma.
  fusionar: objeto(
    { faccionAId: IDENTIFICADOR, faccionBId: IDENTIFICADOR, nuevoNombre: { type: 'string' }, nuevoReyId: IDENTIFICADOR },
    ['faccionAId', 'faccionBId', 'nuevoNombre', 'nuevoReyId']
  ),

  // --- Comercio ---
  proponerTrueque: objeto(
    {
      asentamientoAId: IDENTIFICADOR,
      recursoA: RECURSO,
      cantidadA: NUMERO,
      asentamientoBId: IDENTIFICADOR,
      recursoB: RECURSO,
      cantidadB: NUMERO,
    },
    ['asentamientoAId', 'recursoA', 'cantidadA', 'asentamientoBId', 'recursoB', 'cantidadB']
  ),
  colocarOrdenMercado: {
    type: 'object',
    properties: {
      asentamientoId: IDENTIFICADOR,
      tipo: { type: 'string', enum: ['compra', 'venta'] },
      recurso: RECURSO,
      cantidad: NUMERO,
      precio: NUMERO,
    },
    required: ['asentamientoId', 'tipo', 'recurso', 'cantidad'],
    additionalProperties: false,
  },
  crearCaravana: objeto({ asentamientoId: IDENTIFICADOR }, ['asentamientoId']),

  // --- Militar ---
  reclutarTropa: objeto(
    {
      asentamientoId: IDENTIFICADOR,
      jugadorId: IDENTIFICADOR,
      tropaId: IDENTIFICADOR,
      origen: { type: 'string', enum: ['pesants', 'artesanos'] },
    },
    ['asentamientoId', 'jugadorId', 'tropaId', 'origen']
  ),
  iniciarAsedio: objeto(
    { atacanteId: IDENTIFICADOR, defensorId: IDENTIFICADOR, escuadronIds: LISTA_DE_IDENTIFICADORES },
    ['atacanteId', 'defensorId', 'escuadronIds']
  ),
  // Presencia del jugador (Doc 1.10). `carga` es un mapa recurso -> cantidad: el jugador elige QUÉ se lleva,
  // no solo cuánto, así que no vale una lista de ids ni un número suelto. Las cantidades se validan en el
  // motor (capacidad del carro, almacén, reserva); aquí solo que sean números.
  salirAlMundo: objeto(
    {
      asentamientoId: IDENTIFICADOR,
      jugadorId: IDENTIFICADOR,
      escuadronIds: LISTA_DE_IDENTIFICADORES,
      carga: { type: 'object', additionalProperties: NUMERO },
    },
    ['asentamientoId', 'jugadorId', 'escuadronIds', 'carga']
  ),
  marcharA: objeto({ jugadorId: IDENTIFICADOR, objetivo: OBJETIVO_EJERCITO }, ['jugadorId', 'objetivo']),
  // Composición de una columna compartida (Doc 5.14). Separarse no lleva `ejercitoId`: se sale de la columna
  // en la que vas, y solo puedes ir en una.
  unirseEnCampo: objeto({ ejercitoId: IDENTIFICADOR, jugadorId: IDENTIFICADOR }, ['ejercitoId', 'jugadorId']),
  responderPeticionDeUnion: objeto(
    { ejercitoId: IDENTIFICADOR, jugadorId: IDENTIFICADOR, solicitanteId: IDENTIFICADOR, aceptar: { type: 'boolean' } },
    ['ejercitoId', 'jugadorId', 'solicitanteId', 'aceptar']
  ),
  separarseDelEjercito: objeto({ jugadorId: IDENTIFICADOR }, ['jugadorId']),
  // El menú de interacción (Doc 5.12.3). `objetivo` distingue columna de caravana: son entidades distintas
  // con anillos y consecuencias distintas, y mezclarlas en un id suelto obligaría al motor a adivinar.
  inspeccionar: objeto({ jugadorId: IDENTIFICADOR, objetivo: OBJETIVO_DE_INTERACCION }, ['jugadorId', 'objetivo']),
  cederLiderazgo: objeto({ ejercitoId: IDENTIFICADOR, jugadorId: IDENTIFICADOR, sucesorId: IDENTIFICADOR }, ['ejercitoId', 'jugadorId', 'sucesorId']),
  entrarEnAsentamiento: objeto({ asentamientoId: IDENTIFICADOR, jugadorId: IDENTIFICADOR }, ['asentamientoId', 'jugadorId']),
  // La puerta (Doc 1.10.5). No va en `politicasActivas` porque no expira: una puerta que se abre sola a las
  // dos horas y media no es una puerta.
  fijarPoliticaDeAcceso: objeto(
    {
      asentamientoId: IDENTIFICADOR,
      jugadorId: IDENTIFICADOR,
      politica: { type: 'string', enum: ['abierto', 'faccion_y_aliados', 'solo_faccion', 'cerrado'] },
    },
    ['asentamientoId', 'jugadorId', 'politica']
  ),
  vetarJugador: objeto(
    { asentamientoId: IDENTIFICADOR, jugadorId: IDENTIFICADOR, vetadoId: IDENTIFICADOR, vetar: { type: 'boolean' } },
    ['asentamientoId', 'jugadorId', 'vetadoId', 'vetar']
  ),
  salirDeAsentamiento: objeto({ asentamientoId: IDENTIFICADOR, jugadorId: IDENTIFICADOR }, ['asentamientoId', 'jugadorId']),
  // Ejércitos (Doc 5.12).
  movilizarEjercito: objeto(
    {
      asentamientoId: IDENTIFICADOR,
      jugadorId: IDENTIFICADOR,
      escuadronIds: LISTA_DE_IDENTIFICADORES,
      objetivo: OBJETIVO_EJERCITO,
      politicaDeUnion: { type: 'string', enum: ['rechazar', 'aceptar', 'preguntar'] },
    },
    ['asentamientoId', 'jugadorId', 'escuadronIds', 'objetivo']
  ),
  unirseAEjercito: objeto(
    { ejercitoId: IDENTIFICADOR, asentamientoId: IDENTIFICADOR, jugadorId: IDENTIFICADOR, escuadronIds: LISTA_DE_IDENTIFICADORES },
    ['ejercitoId', 'asentamientoId', 'jugadorId', 'escuadronIds']
  ),
  replegarEjercito: objeto({ ejercitoId: IDENTIFICADOR }, ['ejercitoId']),
  estacionarEjercito: objeto({ ejercitoId: IDENTIFICADOR }, ['ejercitoId']),
  atacarCampamentoBandidos: objeto(
    { atacanteId: IDENTIFICADOR, escuadronIds: LISTA_DE_IDENTIFICADORES, campamentoId: IDENTIFICADOR },
    ['atacanteId', 'escuadronIds', 'campamentoId']
  ),
};
