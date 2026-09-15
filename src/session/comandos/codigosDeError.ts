// Catálogo de códigos de error de dominio ESTABLES (Docs/Arquitectura/2_Estudio_Evolucion_Backend_Multifrontend.md,
// punto 6: "un cliente decide qué hacer según el código, nunca parseando `.message`"). Antes vivían en dos
// sitios sin conexión entre sí: los 13 del motor como literales dentro de `erroresDeDominio.ts`, y los que
// añade la capa de partida como literales sueltos (y hasta triplicados — `ASENTAMIENTO_NO_EXISTE` declarado
// por separado en `cargos.ts`/`construccion.ts`/`militar.ts`) repartidos por `session/comandos/*.ts`. Un
// cliente que quisiera traducir códigos a mensajes no tenía ninguna lista completa de la que partir. Un único
// catálogo también permite que `ResultadoComando.codigoError` tenga un tipo cerrado (`CodigoError`) en vez de
// `string` suelto.
export const CODIGOS_ERROR = {
  // Los 14 que el MOTOR (`engine/*.ts`) lanza como excepción — mapeados desde la clase de error concreta en
  // `erroresDeDominio.ts`, que importa este catálogo en vez de escribir los literales por su cuenta.
  cargoInvalido: 'cargo.invalido',
  combateInvalido: 'combate.invalido',
  construccionInvalida: 'construccion.invalida',
  diplomaciaInvalida: 'diplomacia.invalida',
  expansionInvalida: 'expansion.invalida',
  faccionInvalida: 'faccion.invalida',
  fundacionInvalida: 'fundacion.invalida',
  fusionInvalida: 'fusion.invalida',
  mercadoOrdenInvalida: 'mercado.orden_invalida',
  politicaInvalida: 'politica.invalida',
  tropasReclutamientoInvalido: 'tropas.reclutamiento_invalido',
  comercioCaravanaInvalida: 'comercio.caravana_invalida',
  comercioTruequeInvalido: 'comercio.trueque_invalido',
  recintoInvalido: 'recinto.invalido',
  movilizacionInvalida: 'movilizacion.invalida',
  heroeInvalido: 'heroe.invalido',

  // Los que rechaza la CAPA DE PARTIDA (`session/comandos/*.ts`) sin que el motor llegue a verlos: casi todos
  // "la entidad referenciada por id no existe" (el motor recibe la entidad ya resuelta, nunca un id suelto),
  // más un puñado de validaciones que son reglas de ESTA partida y no del modelo de juego (ver
  // `crearFaccion.ts`: nombre vacío/duplicado no los valida el motor).
  asentamientoNoExiste: 'asentamiento.no_existe',
  faccionNoExiste: 'faccion.no_existe',
  caravanaNoExiste: 'caravana.no_existe',
  caravanaNoAparcadaAqui: 'caravana.no_aparcada_aqui',
  campamentoNoExiste: 'campamento.no_existe',
  ejercitoNoExiste: 'ejercito.no_existe',
  acuerdoNoExiste: 'acuerdo.no_existe',
  ordenNoExiste: 'orden.no_existe',
  sinColumna: 'jugador.sin_columna',
  jugadorNoExiste: 'jugador.no_existe',
  puertaInvalida: 'puerta.invalida',
  reservaSinTesorero: 'reserva.sin_tesorero',
  diplomaciaRelacionNoIndicada: 'diplomacia.relacion_no_indicada',
  faccionNombreVacio: 'faccion.nombre_vacio',
  faccionNombreDuplicado: 'faccion.nombre_duplicado',
  // Membresía de Facción (a petición del usuario, 2026-08-27): `crearFaccion` y `unirseAFaccion` comparten el
  // primero (1 jugador, 1 Facción, Doc 2 "Entidades"); el segundo solo lo dispara `crearFaccion`, ver
  // `CIUDADANIA.cooldownCreacionFaccionDias`; el tercero lo dispara `dejarFaccion` cuando el actor no es
  // ciudadano de ninguna.
  faccionYaPerteneces: 'faccion.ya_pertenece',
  faccionCooldownCreacion: 'faccion.cooldown_creacion',
  faccionNoPerteneces: 'faccion.no_pertenece',
  // `crearHeroe` (doc 02 §4.2): un héroe por jugador y partida, con nombre.
  heroeYaExiste: 'heroe.ya_existe',
  heroeNombreVacio: 'heroe.nombre_vacio',
  // Batallas de Unity (doc 02 §3.1): lo que está en una batalla activa no se toca (Doc 5.15.1), y unirse o cancelar
  // tienen sus propias reglas.
  batallaNoExiste: 'batalla.no_existe',
  batallaBloqueo: 'batalla.bloqueado',
  batallaInvalida: 'batalla.invalida',
  batallaYaAsignada: 'batalla.ya_asignada',
} as const;

export type CodigoError = (typeof CODIGOS_ERROR)[keyof typeof CODIGOS_ERROR];
