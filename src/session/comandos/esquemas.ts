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
  calibrarReservaManual: objeto({ asentamientoId: IDENTIFICADOR, recurso: RECURSO, valor: NUMERO }, [
    'asentamientoId',
    'recurso',
    'valor',
  ]),
  // Sin `minLength` en `nombre`: vacío es un valor legítimo aquí ("volver a mostrar el id",
  // `renombrarAsentamiento`, `construccion.ts`) — no un error de forma.
  renombrarAsentamiento: objeto({ asentamientoId: IDENTIFICADOR, nombre: { type: 'string' } }, ['asentamientoId', 'nombre']),

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
  combateCampoAbierto: objeto(
    {
      asentamientoAId: IDENTIFICADOR,
      escuadronIdsA: LISTA_DE_IDENTIFICADORES,
      asentamientoBId: IDENTIFICADOR,
      escuadronIdsB: LISTA_DE_IDENTIFICADORES,
    },
    ['asentamientoAId', 'escuadronIdsA', 'asentamientoBId', 'escuadronIdsB']
  ),
  interceptarCaravana: objeto(
    { atacanteId: IDENTIFICADOR, escuadronIds: LISTA_DE_IDENTIFICADORES, caravanaId: IDENTIFICADOR },
    ['atacanteId', 'escuadronIds', 'caravanaId']
  ),
  atacarCampamentoBandidos: objeto(
    { atacanteId: IDENTIFICADOR, escuadronIds: LISTA_DE_IDENTIFICADORES, campamentoId: IDENTIFICADOR },
    ['atacanteId', 'escuadronIds', 'campamentoId']
  ),
};
