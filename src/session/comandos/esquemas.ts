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
import { ATRIBUTOS_HEROE, CARGOS_TIPO, EDIFICIOS_TIPO, RECURSOS_TIPO } from '../../domain/types';
import { CARGOS_CONSTRUCTOR } from './construccion';
import { TECNOLOGIAS } from '../../constants';
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
/** Lo que ofrece un lado de un trueque: al menos una línea, cada una con su recurso y su cantidad. */
const LINEAS_TRUEQUE = {
  type: 'array',
  minItems: 1,
  items: { type: 'object', properties: { recurso: RECURSO, cantidad: NUMERO }, required: ['recurso', 'cantidad'], additionalProperties: false },
} as const;
// A dónde va una columna: un asentamiento por id, o un punto del mapa. Se valida con `oneOf` para que un
// cliente no pueda colar un punto sin coordenadas ni un destino sin id. Lo comparten `movilizarEjercito`
// (fijarlo al salir) y `marcharA` (rectificarlo cuantas veces quiera un viajero solo).
const OBJETIVO_DE_INTERACCION = {
  oneOf: [
    { type: 'object', properties: { tipo: { type: 'string', enum: ['ejercito'] }, id: IDENTIFICADOR }, required: ['tipo', 'id'], additionalProperties: false },
    { type: 'object', properties: { tipo: { type: 'string', enum: ['caravana'] }, id: IDENTIFICADOR }, required: ['tipo', 'id'], additionalProperties: false },
  ],
} as const;
// `atacar` apunta además a un campamento de bandidos (Doc 1.9): se ataca con la columna, no se mira ni se persigue.
const OBJETIVO_DE_ATAQUE = {
  oneOf: [
    ...OBJETIVO_DE_INTERACCION.oneOf,
    { type: 'object', properties: { tipo: { type: 'string', enum: ['campamento', 'asentamiento'] }, id: IDENTIFICADOR }, required: ['tipo', 'id'], additionalProperties: false },
  ],
} as const;
const OBJETIVO_EJERCITO = {
  oneOf: [
    { type: 'object', properties: { tipo: { type: 'string', enum: ['asentamiento'] }, id: IDENTIFICADOR }, required: ['tipo', 'id'], additionalProperties: false },
    { type: 'object', properties: { tipo: { type: 'string', enum: ['punto'] }, punto: PUNTO }, required: ['tipo', 'punto'], additionalProperties: false },
  ],
} as const;
// Dónde se compra intel (Doc 5.12.10): la taberna de una plaza o la de un campamento de mercenarios.
const ORIGEN_DE_INTEL = objeto({ tipo: { type: 'string', enum: ['asentamiento', 'campamento'] }, id: IDENTIFICADOR }, ['tipo', 'id']);
const CARGO_FACCION = { type: 'string', enum: CARGOS_TIPO } as const;
const TECNOLOGIA = { type: 'string', enum: Object.keys(TECNOLOGIAS) } as const;
const CARGO_CONSTRUCTOR = { type: 'string', enum: CARGOS_CONSTRUCTOR } as const;

/** Fábrica de `{type:'object', properties, required, additionalProperties:false}` — evita repetir las tres
 * líneas de cierre en cada una de las 30 entradas de abajo. */
function objeto(properties: Record<string, unknown>, required: readonly string[]): EsquemaJson {
  return { type: 'object', properties, required: [...required], additionalProperties: false };
}

export const ESQUEMAS_PARAMS: Record<TipoComando, EsquemaJson> = {
  // --- Fundación y expansión ---
  // Sin `posicion`: se funda donde se está (Doc 1.3), y ese punto sale de la columna del fundador, no de un
  // parámetro que el cliente pueda elegir.
  lanzarCaravanaFundacion: objeto({ origenAsentamientoId: IDENTIFICADOR }, ['origenAsentamientoId']),
  desarmarCaravanaFundacion: objeto({ caravanaId: IDENTIFICADOR }, ['caravanaId']),

  // Sin `minLength` en `nombre`: vacío/solo-espacios ya es un rechazo de dominio con su propio código
  // (`faccion.nombre_vacio`, `crearFaccion.ts`) — mismo motivo que `NUMERO` más arriba.
  // `sigilo` es opcional: sin él, la Facción recibe uno libre (los bots). Valida la forma; que los ids existan y que
  // no se repita lo decide el comando (`faccion.sigilo_invalido`/`faccion.sigilo_duplicado`).
  crearFaccion: objeto(
    {
      nombre: { type: 'string' },
      sigilo: objeto(
        {
          formaId: IDENTIFICADOR,
          campoId: IDENTIFICADOR,
          emblemaId: IDENTIFICADOR,
          colorPrimarioId: IDENTIFICADOR,
          colorSecundarioId: IDENTIFICADOR,
          colorEmblemaId: IDENTIFICADOR,
          orlaId: IDENTIFICADOR,
          colorOrlaId: IDENTIFICADOR,
        },
        ['formaId', 'campoId', 'emblemaId', 'colorPrimarioId', 'colorSecundarioId', 'colorEmblemaId', 'orlaId', 'colorOrlaId']
      ),
    },
    ['nombre']
  ),
  // Sin `minLength` en `displayName`, mismo motivo que `nombre` de arriba (`heroe.nombre_vacio`).
  crearHeroe: objeto(
    {
      displayName: { type: 'string' },
      campamentoId: IDENTIFICADOR,
      classDefinitionId: IDENTIFICADOR,
      genero: { type: 'string', enum: ['masculino', 'femenino'] },
      avatar: objeto(
        { cabezaId: { type: 'string' }, peloId: { type: 'string' }, barbaId: { type: 'string' }, cejasId: { type: 'string' } },
        ['cabezaId', 'peloId', 'barbaId', 'cejasId']
      ),
    },
    ['displayName', 'campamentoId', 'classDefinitionId', 'genero', 'avatar']
  ),
  // Solo atributos: los perks esperan al catálogo de Conquest (CQ-004). Sin `minimum`, mismo motivo que `NUMERO`.
  repartirPuntos: objeto({ atributos: objeto(Object.fromEntries(ATRIBUTOS_HEROE.map((a) => [a, { type: 'integer' }])), []) }, ['atributos']),
  // Sin `minLength` en `displayName`: nombre vacío es un rechazo de dominio (`heroe.invalido`).
  guardarLoadout: objeto(
    {
      loadoutId: IDENTIFICADOR,
      displayName: { type: 'string' },
      squadIds: LISTA_DE_IDENTIFICADORES,
      perksSeleccionados: { type: 'array', items: { type: 'integer' } },
      activo: { type: 'boolean' },
    },
    ['displayName', 'squadIds', 'perksSeleccionados']
  ),
  borrarLoadout: objeto({ loadoutId: IDENTIFICADOR }, ['loadoutId']),
  guardarEnAlmacenPersonal: objeto({ recurso: RECURSO, cantidad: NUMERO }, ['recurso', 'cantidad']),
  sacarDelAlmacenPersonal: objeto({ recurso: RECURSO, cantidad: NUMERO }, ['recurso', 'cantidad']),
  ordenarEscuadras: objeto({ escuadronIds: LISTA_DE_IDENTIFICADORES }, ['escuadronIds']),
  asignarGuarnicion: objeto({ squadId: IDENTIFICADOR }, ['squadId']),
  retirarGuarnicion: objeto({ squadId: IDENTIFICADOR }, ['squadId']),
  solicitarIngreso: objeto({ faccionId: IDENTIFICADOR }, ['faccionId']),
  responderSolicitud: objeto({ faccionId: IDENTIFICADOR, heroeId: IDENTIFICADOR, aceptar: { type: 'boolean' } }, ['faccionId', 'heroeId', 'aceptar']),
  // Sin parámetros: el actor solo puede dejar SU PROPIA Facción — `objeto({}, [])` solo admite `{}`.
  dejarFaccion: objeto({}, []),

  // --- Cargos ---
  asignarRey: objeto({ faccionId: IDENTIFICADOR, heroeId: IDENTIFICADOR }, ['faccionId', 'heroeId']),
  asignarEmbajador: objeto({ faccionId: IDENTIFICADOR, heroeId: IDENTIFICADOR }, ['faccionId', 'heroeId']),
  asignarCargoLocal: objeto(
    { asentamientoId: IDENTIFICADOR, cargo: CARGO_FACCION, heroeId: IDENTIFICADOR },
    ['asentamientoId', 'cargo', 'heroeId']
  ),
  aportarARefundacion: objeto({ recurso: RECURSO, cantidad: NUMERO, lado: { type: 'string', enum: ['almacen', 'carro'] } }, ['recurso', 'cantidad', 'lado']),
  retirarDeRefundacion: objeto({ recurso: RECURSO, cantidad: NUMERO, lado: { type: 'string', enum: ['almacen', 'carro'] } }, ['recurso', 'cantidad', 'lado']),
  comprarCaravanaDeRefundacion: objeto({}, []),
  fundar: objeto({}, []),
  comprarEnCampamento: objeto({ recurso: RECURSO, cantidad: NUMERO, campamentoId: IDENTIFICADOR }, ['recurso', 'cantidad']),
  pedirPrestamo: objeto({ tropaIds: LISTA_DE_IDENTIFICADORES }, ['tropaIds']),
  reponerPrestamo: objeto({}, []),
  abrirAlijo: objeto({ alijoId: IDENTIFICADOR }, ['alijoId']),
  reclutarEnCampamento: objeto({ tropaId: IDENTIFICADOR, pagarCon: { type: 'string', enum: ['almacenPersonal', 'carro'] } }, ['tropaId']),
  residirEnCampamento: objeto({ heroeId: IDENTIFICADOR, campamentoId: IDENTIFICADOR }, ['heroeId', 'campamentoId']),
  dejarResidencia: objeto({ heroeId: IDENTIFICADOR }, ['heroeId']),
  cambiarResidencia: objeto({ destinoId: IDENTIFICADOR, heroeId: IDENTIFICADOR }, ['destinoId', 'heroeId']),

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
  alternarReceta: objeto({ asentamientoId: IDENTIFICADOR, recurso: RECURSO, pausada: { type: 'boolean' } }, ['asentamientoId', 'recurso', 'pausada']),
  alternarReabastecerAliados: objeto({ asentamientoId: IDENTIFICADOR, permitido: { type: 'boolean' } }, ['asentamientoId', 'permitido']),
  adjuntarCaravana: objeto({ ejercitoId: IDENTIFICADOR, caravanaId: IDENTIFICADOR, heroeId: IDENTIFICADOR }, [
    'ejercitoId',
    'caravanaId',
    'heroeId',
  ]),
  soltarCaravana: objeto({ ejercitoId: IDENTIFICADOR, caravanaId: IDENTIFICADOR, heroeId: IDENTIFICADOR }, [
    'ejercitoId',
    'caravanaId',
    'heroeId',
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
  solicitarAscenso: objeto({ asentamientoId: IDENTIFICADOR }, ['asentamientoId']),
  adoptarTecnologia: objeto({ faccionId: IDENTIFICADOR, tecnologiaId: TECNOLOGIA }, ['faccionId', 'tecnologiaId']),
  empezarEpica: objeto({ asentamientoId: IDENTIFICADOR, aedaId: IDENTIFICADOR, tecnologiaId: TECNOLOGIA }, ['asentamientoId', 'aedaId', 'tecnologiaId']),
  abandonarEpica: objeto({ asentamientoId: IDENTIFICADOR, aedaId: IDENTIFICADOR }, ['asentamientoId', 'aedaId']),
  comprarMirada: objeto({ origen: ORIGEN_DE_INTEL, centro: PUNTO }, ['origen', 'centro']),
  comprarInformePlaza: objeto({ origen: ORIGEN_DE_INTEL, asentamientoId: IDENTIFICADOR }, ['origen', 'asentamientoId']),
  comprarTecnologiaAeda: objeto({ asentamientoId: IDENTIFICADOR, tecnologiaId: TECNOLOGIA }, ['asentamientoId', 'tecnologiaId']),
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
  admitirOtrasFacciones: objeto({ faccionId: IDENTIFICADOR, admitir: { type: 'boolean' } }, ['faccionId', 'admitir']),
  designarCapital: objeto({ faccionId: IDENTIFICADOR, asentamientoId: IDENTIFICADOR }, ['faccionId', 'asentamientoId']),
  rebelionVasallo: objeto({ relacionId: IDENTIFICADOR }, ['relacionId']),
  declararGuerra: objeto({ faccionAId: IDENTIFICADOR, faccionBId: IDENTIFICADOR }, ['faccionAId', 'faccionBId']),
  proponerPaz: objeto({ relacionId: IDENTIFICADOR, faccionId: IDENTIFICADOR }, ['relacionId', 'faccionId']),
  proponerAnexion: objeto({ faccionAId: IDENTIFICADOR, faccionBId: IDENTIFICADOR }, ['faccionAId', 'faccionBId']),
  responderAnexion: objeto({ propuestaId: IDENTIFICADOR, aceptar: { type: 'boolean' } }, ['propuestaId', 'aceptar']),
  retirarAnexion: objeto({ propuestaId: IDENTIFICADOR }, ['propuestaId']),
  // Sin `minLength` en `nuevoNombre`: un nombre vacío es un rechazo de dominio con su propio código (`fusion.invalida`).
  proponerFusion: objeto(
    { faccionAId: IDENTIFICADOR, faccionBId: IDENTIFICADOR, nuevoNombre: { type: 'string' }, nuevoReyId: IDENTIFICADOR },
    ['faccionAId', 'faccionBId', 'nuevoNombre', 'nuevoReyId']
  ),
  responderFusion: objeto({ propuestaId: IDENTIFICADOR, aceptar: { type: 'boolean' } }, ['propuestaId', 'aceptar']),
  retirarFusion: objeto({ propuestaId: IDENTIFICADOR }, ['propuestaId']),

  // --- Comercio ---
  // Cada lado ofrece una o varias líneas (trueque compuesto, Doc 3.2).
  proponerTrueque: objeto(
    {
      asentamientoAId: IDENTIFICADOR,
      lineasA: LINEAS_TRUEQUE,
      asentamientoBId: IDENTIFICADOR,
      lineasB: LINEAS_TRUEQUE,
    },
    ['asentamientoAId', 'lineasA', 'asentamientoBId', 'lineasB']
  ),
  aceptarTrueque: objeto({ acuerdoId: IDENTIFICADOR }, ['acuerdoId']),
  rechazarTrueque: objeto({ acuerdoId: IDENTIFICADOR }, ['acuerdoId']),
  comerciarEnPlaza: objeto(
    { heroeId: IDENTIFICADOR, asentamientoId: IDENTIFICADOR, ordenId: IDENTIFICADOR, cantidad: NUMERO },
    ['heroeId', 'asentamientoId', 'ordenId', 'cantidad']
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
  agregarCarroCaravana: objeto(
    { caravanaId: IDENTIFICADOR, tipoCarro: { type: 'string', enum: ['basico', 'reforzado'] } },
    ['caravanaId', 'tipoCarro']
  ),
  comprarAnimalCaravana: objeto(
    { caravanaId: IDENTIFICADOR, carroIndice: NUMERO, tipoAnimal: { type: 'string', enum: ['buey', 'caballo', 'camello'] } },
    ['caravanaId', 'carroIndice', 'tipoAnimal']
  ),
  reservarCaravana: objeto({ caravanaId: IDENTIFICADOR, reservada: { type: 'boolean' } }, ['caravanaId', 'reservada']),
  prepararCaravana: objeto(
    {
      caravanaId: IDENTIFICADOR,
      heroeId: IDENTIFICADOR,
      destinoAsentamientoId: IDENTIFICADOR,
      carga: { type: 'object', additionalProperties: NUMERO },
      escoltaEscuadronIds: { type: 'array', items: IDENTIFICADOR },
      salirEn: NUMERO,
    },
    ['caravanaId', 'heroeId', 'destinoAsentamientoId', 'carga']
  ),
  cancelarCaravana: objeto({ caravanaId: IDENTIFICADOR }, ['caravanaId']),
  moverCarroCaravana: objeto(
    { desdeCaravanaId: IDENTIFICADOR, haciaCaravanaId: IDENTIFICADOR, carroIndice: NUMERO },
    ['desdeCaravanaId', 'haciaCaravanaId', 'carroIndice']
  ),
  // Caravanas 'aparcadas' tras guarnecer (Ocupacion §2.3d).
  moverCargaCaravanaAparcada: objeto(
    {
      heroeId: IDENTIFICADOR,
      caravanaId: IDENTIFICADOR,
      asentamientoId: IDENTIFICADOR,
      recurso: RECURSO,
      cantidad: NUMERO,
      sentido: { type: 'string', enum: ['cargar', 'descargar'] },
    },
    ['heroeId', 'caravanaId', 'asentamientoId', 'recurso', 'cantidad', 'sentido']
  ),
  enviarCaravanaAlOrigen: objeto(
    { heroeId: IDENTIFICADOR, caravanaId: IDENTIFICADOR, asentamientoId: IDENTIFICADOR },
    ['heroeId', 'caravanaId', 'asentamientoId']
  ),

  // --- Militar ---
  reclutarTropa: objeto(
    {
      asentamientoId: IDENTIFICADOR,
      heroeId: IDENTIFICADOR,
      tropaId: IDENTIFICADOR,
      origen: { type: 'string', enum: ['pesants', 'artesanos'] },
    },
    ['asentamientoId', 'heroeId', 'tropaId']
  ),
  iniciarAsedio: objeto(
    { atacanteId: IDENTIFICADOR, defensorId: IDENTIFICADOR, escuadronIds: LISTA_DE_IDENTIFICADORES },
    ['atacanteId', 'defensorId', 'escuadronIds']
  ),
  // Presencia del jugador (Doc 1.10). `carga` es un mapa recurso -> cantidad: el jugador elige QUÉ se lleva,
  // no solo cuánto, así que no vale una lista de ids ni un número suelto. Las cantidades se validan en el
  // motor (capacidad del carro, almacén, reserva); aquí solo que sean números.
  conectarse: objeto({ heroeId: IDENTIFICADOR }, ['heroeId']),
  desconectarse: objeto({ heroeId: IDENTIFICADOR }, ['heroeId']),
  salirAlMundo: objeto(
    {
      asentamientoId: IDENTIFICADOR,
      heroeId: IDENTIFICADOR,
      escuadronIds: LISTA_DE_IDENTIFICADORES,
      carga: { type: 'object', additionalProperties: NUMERO },
    },
    ['asentamientoId', 'heroeId', 'escuadronIds', 'carga']
  ),
  marcharA: objeto({ heroeId: IDENTIFICADOR, objetivo: OBJETIVO_EJERCITO }, ['heroeId', 'objetivo']),
  // Composición de una columna compartida (Doc 5.14). Separarse no lleva `ejercitoId`: se sale de la columna
  // en la que vas, y solo puedes ir en una.
  unirseEnCampo: objeto({ ejercitoId: IDENTIFICADOR, heroeId: IDENTIFICADOR }, ['ejercitoId', 'heroeId']),
  responderPeticionDeUnion: objeto(
    { ejercitoId: IDENTIFICADOR, heroeId: IDENTIFICADOR, solicitanteId: IDENTIFICADOR, aceptar: { type: 'boolean' } },
    ['ejercitoId', 'heroeId', 'solicitanteId', 'aceptar']
  ),
  separarseDelEjercito: objeto({ heroeId: IDENTIFICADOR }, ['heroeId']),
  organizarEjercito: objeto({ heroeId: IDENTIFICADOR, politicaDeUnion: { type: 'string', enum: ['aceptar', 'preguntar'] } }, ['heroeId', 'politicaDeUnion']),
  cancelarFormacion: objeto({ heroeId: IDENTIFICADOR }, ['heroeId']),
  // El menú de interacción (Doc 5.12.3). `objetivo` distingue columna de caravana: son entidades distintas
  // con anillos y consecuencias distintas, y mezclarlas en un id suelto obligaría al motor a adivinar.
  inspeccionar: objeto(
    {
      heroeId: IDENTIFICADOR,
      objetivo: {
        oneOf: [
          ...OBJETIVO_DE_INTERACCION.oneOf,
          { type: 'object', properties: { tipo: { type: 'string', enum: ['asentamiento'] }, id: IDENTIFICADOR }, required: ['tipo', 'id'], additionalProperties: false },
        ],
      },
    },
    ['heroeId', 'objetivo']
  ),
  atacar: objeto({ heroeId: IDENTIFICADOR, objetivo: OBJETIVO_DE_ATAQUE }, ['heroeId', 'objetivo']),
  perseguir: objeto({ heroeId: IDENTIFICADOR, objetivo: OBJETIVO_DE_INTERACCION }, ['heroeId', 'objetivo']),
  dejarDePerseguir: objeto({ heroeId: IDENTIFICADOR }, ['heroeId']),
  // Batallas de Unity (doc 02 §3.1).
  unirseABatalla: objeto({ heroeId: IDENTIFICADOR, battleId: IDENTIFICADOR, lado: { type: 'string', enum: ['atacante', 'defensor'] } }, ['heroeId', 'battleId']),
  cancelarBatalla: objeto({ battleId: IDENTIFICADOR }, ['battleId']),
  cederLiderazgo: objeto({ ejercitoId: IDENTIFICADOR, heroeId: IDENTIFICADOR, sucesorId: IDENTIFICADOR }, ['ejercitoId', 'heroeId', 'sucesorId']),
  entrarEnAsentamiento: objeto({ asentamientoId: IDENTIFICADOR, heroeId: IDENTIFICADOR }, ['asentamientoId', 'heroeId']),
  // La puerta (Doc 1.10.5). No va en `politicasActivas` porque no expira: una puerta que se abre sola a las
  // dos horas y media no es una puerta.
  fijarPuerta: objeto(
    {
      asentamientoId: IDENTIFICADOR,
      heroeId: IDENTIFICADOR,
      cerradaA: { type: 'array', items: { type: 'string', enum: ['neutrales', 'aliados', 'enemigos', 'aedas'] } },
    },
    ['asentamientoId', 'heroeId', 'cerradaA']
  ),
  vetarJugador: objeto(
    { asentamientoId: IDENTIFICADOR, heroeId: IDENTIFICADOR, vetadoId: IDENTIFICADOR, vetar: { type: 'boolean' } },
    ['asentamientoId', 'heroeId', 'vetadoId', 'vetar']
  ),
  salirDeAsentamiento: objeto({ asentamientoId: IDENTIFICADOR, heroeId: IDENTIFICADOR }, ['asentamientoId', 'heroeId']),
  entrarEnCampamento: objeto({ campamentoId: IDENTIFICADOR, heroeId: IDENTIFICADOR }, ['campamentoId', 'heroeId']),
  salirDelCampamento: objeto(
    {
      campamentoId: IDENTIFICADOR,
      heroeId: IDENTIFICADOR,
      escuadronIds: LISTA_DE_IDENTIFICADORES,
      carga: { type: 'object', additionalProperties: NUMERO },
      politicaDeUnion: { type: 'string', enum: ['rechazar', 'aceptar', 'preguntar'] },
      objetivo: OBJETIVO_EJERCITO,
    },
    ['campamentoId', 'heroeId', 'escuadronIds', 'carga']
  ),
  // `guarnecer` (Ocupacion §2.3): marchar un ejército a una plaza propia y volcar la tropa en su guarnición.
  guarnecer: objeto({ asentamientoId: IDENTIFICADOR, heroeId: IDENTIFICADOR }, ['asentamientoId', 'heroeId']),
  // Ejércitos (Doc 5.12).
  movilizarEjercito: objeto(
    {
      asentamientoId: IDENTIFICADOR,
      heroeId: IDENTIFICADOR,
      escuadronIds: LISTA_DE_IDENTIFICADORES,
      objetivo: OBJETIVO_EJERCITO,
      politicaDeUnion: { type: 'string', enum: ['rechazar', 'aceptar', 'preguntar'] },
    },
    ['asentamientoId', 'heroeId', 'escuadronIds', 'objetivo']
  ),
  unirseAEjercito: objeto(
    { ejercitoId: IDENTIFICADOR, asentamientoId: IDENTIFICADOR, heroeId: IDENTIFICADOR, escuadronIds: LISTA_DE_IDENTIFICADORES },
    ['ejercitoId', 'asentamientoId', 'heroeId', 'escuadronIds']
  ),
  replegarEjercito: objeto({ ejercitoId: IDENTIFICADOR }, ['ejercitoId']),
  estacionarEjercito: objeto({ ejercitoId: IDENTIFICADOR }, ['ejercitoId']),
};
