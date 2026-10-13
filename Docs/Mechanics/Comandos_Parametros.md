# Forma de los parámetros de cada comando

**Alcance y fuentes.** La forma de `params` de los 122 comandos, tal como la valida el servidor en el borde (`src/session/comandos/esquemas.ts`,
`ESQUEMAS_PARAMS`, verificado contra `b8f7a3d`). Es la base de los mensajes de orden del contrato 05 (Protobuf). Quién puede ejecutar cada comando y qué
rechaza el motor están en [`Proyecciones_Y_Autorizacion.md`](Proyecciones_Y_Autorizacion.md) y en el documento de cada sistema.

**Regla de frontera.** Estos esquemas validan solo la **forma** (campos presentes, tipo, enteros y cadenas dentro de catálogos cerrados). Nunca validan que
una entidad exista (`faccionId`, `asentamientoId`…) ni rangos de dominio: `cantidad <= 0`, nombres vacíos y similares los rechaza el motor con su propio
código de error, para no tener dos formas de la misma regla (400 en un caso, `ok: false` en otro). El contrato 05 conserva esta frontera.

**Generación.** La tabla se volcó una vez desde `ESQUEMAS_PARAMS`; las cadenas de identificador son `string` no vacío. Al reescribir el contrato, esta tabla
es la lista de mensajes de orden que hay que cubrir.

| Comando | Parámetros (`?` = opcional) |
|---|---|
| `lanzarCaravanaFundacion` | `origenAsentamientoId`: string |
| `desarmarCaravanaFundacion` | `caravanaId`: string |
| `crearFaccion` | `nombre`: string<br>`sigilo?`: {formaId: string, campoId: string, emblemaId: string, colorPrimarioId: string, colorSecundarioId: string, colorEmblemaId: string, orlaId: string, colorOrlaId: string} |
| `crearHeroe` | `displayName`: string<br>`campamentoId`: string<br>`classDefinitionId`: string<br>`genero`: "masculino"|"femenino"<br>`avatar`: {cabezaId: string, peloId: string, barbaId: string, cejasId: string} |
| `repartirPuntos` | `atributos`: {fuerza?: integer, destreza?: integer, armadura?: integer, vitalidad?: integer} |
| `guardarLoadout` | `loadoutId?`: string<br>`displayName`: string<br>`squadIds`: string[]<br>`perksSeleccionados`: integer[]<br>`activo?`: boolean |
| `borrarLoadout` | `loadoutId`: string |
| `guardarEnAlmacenPersonal` | `recurso`: "madera"|"piedra"|"trigo"|"cobre"|"estano"|"oro"|"livestock"|"lingoteCobre"|"lingoteEstano"|"lingoteBronce"|"cuero"|"cueroCurtido"|"cueroCalidad"|"armaMadera"|"armaCobre"|"armaBronce"|"armaBronceCalidad"|"armaduraBasica"|"armaduraIntermedia"|"armaduraBronce"|"hierro"|"lingoteHierro"|"armaHierro"|"armaHierroCalidad"|"armaduraBronceCalidad"|"carroGuerra"|"chatarraCobre"|"chatarraBronce"|"chatarraHierro"<br>`cantidad`: number |
| `sacarDelAlmacenPersonal` | `recurso`: "madera"|"piedra"|"trigo"|"cobre"|"estano"|"oro"|"livestock"|"lingoteCobre"|"lingoteEstano"|"lingoteBronce"|"cuero"|"cueroCurtido"|"cueroCalidad"|"armaMadera"|"armaCobre"|"armaBronce"|"armaBronceCalidad"|"armaduraBasica"|"armaduraIntermedia"|"armaduraBronce"|"hierro"|"lingoteHierro"|"armaHierro"|"armaHierroCalidad"|"armaduraBronceCalidad"|"carroGuerra"|"chatarraCobre"|"chatarraBronce"|"chatarraHierro"<br>`cantidad`: number |
| `pasarAViveres` | `cantidad`: number |
| `ordenarEscuadras` | `escuadronIds`: string[] |
| `asignarGuarnicion` | `squadId`: string |
| `retirarGuarnicion` | `squadId`: string |
| `solicitarIngreso` | `faccionId`: string |
| `responderSolicitud` | `faccionId`: string<br>`heroeId`: string<br>`aceptar`: boolean |
| `dejarFaccion` | — |
| `asignarRey` | `faccionId`: string<br>`heroeId`: string |
| `asignarEmbajador` | `faccionId`: string<br>`heroeId`: string |
| `asignarCargoLocal` | `asentamientoId`: string<br>`cargo`: "gobernador"|"tesorero"|"general"|"maestroObras"|"sacerdote"<br>`heroeId`: string |
| `aportarARefundacion` | `recurso`: "madera"|"piedra"|"trigo"|"cobre"|"estano"|"oro"|"livestock"|"lingoteCobre"|"lingoteEstano"|"lingoteBronce"|"cuero"|"cueroCurtido"|"cueroCalidad"|"armaMadera"|"armaCobre"|"armaBronce"|"armaBronceCalidad"|"armaduraBasica"|"armaduraIntermedia"|"armaduraBronce"|"hierro"|"lingoteHierro"|"armaHierro"|"armaHierroCalidad"|"armaduraBronceCalidad"|"carroGuerra"|"chatarraCobre"|"chatarraBronce"|"chatarraHierro"<br>`cantidad`: number<br>`lado`: "almacen"|"carro" |
| `retirarDeRefundacion` | `recurso`: "madera"|"piedra"|"trigo"|"cobre"|"estano"|"oro"|"livestock"|"lingoteCobre"|"lingoteEstano"|"lingoteBronce"|"cuero"|"cueroCurtido"|"cueroCalidad"|"armaMadera"|"armaCobre"|"armaBronce"|"armaBronceCalidad"|"armaduraBasica"|"armaduraIntermedia"|"armaduraBronce"|"hierro"|"lingoteHierro"|"armaHierro"|"armaHierroCalidad"|"armaduraBronceCalidad"|"carroGuerra"|"chatarraCobre"|"chatarraBronce"|"chatarraHierro"<br>`cantidad`: number<br>`lado`: "almacen"|"carro" |
| `comprarCaravanaDeRefundacion` | — |
| `fundar` | — |
| `repostarEnCampamento` | `campamentoId?`: string |
| `comprarEnCampamento` | `recurso`: "madera"|"piedra"|"trigo"|"cobre"|"estano"|"oro"|"livestock"|"lingoteCobre"|"lingoteEstano"|"lingoteBronce"|"cuero"|"cueroCurtido"|"cueroCalidad"|"armaMadera"|"armaCobre"|"armaBronce"|"armaBronceCalidad"|"armaduraBasica"|"armaduraIntermedia"|"armaduraBronce"|"hierro"|"lingoteHierro"|"armaHierro"|"armaHierroCalidad"|"armaduraBronceCalidad"|"carroGuerra"|"chatarraCobre"|"chatarraBronce"|"chatarraHierro"<br>`cantidad`: number<br>`campamentoId?`: string |
| `pedirPrestamo` | `tropaIds`: string[] |
| `reponerPrestamo` | — |
| `abrirAlijo` | `alijoId`: string |
| `reclutarEnCampamento` | `tropaId`: string<br>`pagarCon?`: "almacenPersonal"|"carro" |
| `residirEnCampamento` | `heroeId`: string<br>`campamentoId`: string |
| `dejarResidencia` | `heroeId`: string |
| `cambiarResidencia` | `destinoId`: string<br>`heroeId`: string |
| `activarPolitica` | `asentamientoId`: string<br>`cargo`: "gobernador"|"tesorero"|"general"|"maestroObras"|"sacerdote"<br>`politicaId`: string |
| `anadirEdificioManualmente` | `asentamientoId`: string<br>`cargo`: "gobernador"|"maestroObras"<br>`tipo`: "centroUrbano"|"vivienda"|"granja"|"cantera"|"lenera"|"almacen"|"granero"|"mina"|"minaCobre"|"minaEstano"|"fundicion"|"granFundicion"|"corral"|"armeria"|"curtiduria"|"carpinteria"|"palacio"|"barracon"|"galeriaDeTiro"|"caballerizas"|"minaHierro"|"salaConsejo"|"taberna"|"mercado"|"puestoMercado"|"maravilla"|"plaza"|"plazaDeArmas"|"patioDeGremios"|"pozo"|"parque" |
| `quitarDeCola` | `asentamientoId`: string<br>`cargo`: "gobernador"|"maestroObras"<br>`edificioId`: string |
| `moverEnCola` | `asentamientoId`: string<br>`cargo`: "gobernador"|"maestroObras"<br>`edificioId`: string<br>`direccion`: "arriba"|"abajo" |
| `mejorarEdificioAhora` | `asentamientoId`: string<br>`cargo`: "gobernador"|"maestroObras"<br>`edificioId`: string |
| `alternarAutoConstruccion` | `asentamientoId`: string<br>`pausada`: boolean |
| `alternarReceta` | `asentamientoId`: string<br>`recurso`: "madera"|"piedra"|"trigo"|"cobre"|"estano"|"oro"|"livestock"|"lingoteCobre"|"lingoteEstano"|"lingoteBronce"|"cuero"|"cueroCurtido"|"cueroCalidad"|"armaMadera"|"armaCobre"|"armaBronce"|"armaBronceCalidad"|"armaduraBasica"|"armaduraIntermedia"|"armaduraBronce"|"hierro"|"lingoteHierro"|"armaHierro"|"armaHierroCalidad"|"armaduraBronceCalidad"|"carroGuerra"|"chatarraCobre"|"chatarraBronce"|"chatarraHierro"<br>`pausada`: boolean |
| `alternarReabastecerAliados` | `asentamientoId`: string<br>`permitido`: boolean |
| `adjuntarCaravana` | `ejercitoId`: string<br>`caravanaId`: string<br>`heroeId`: string |
| `soltarCaravana` | `ejercitoId`: string<br>`caravanaId`: string<br>`heroeId`: string |
| `cargarCaravana` | `ejercitoId`: string<br>`caravanaId`: string<br>`asentamientoId`: string<br>`recurso`: "madera"|"piedra"|"trigo"|"cobre"|"estano"|"oro"|"livestock"|"lingoteCobre"|"lingoteEstano"|"lingoteBronce"|"cuero"|"cueroCurtido"|"cueroCalidad"|"armaMadera"|"armaCobre"|"armaBronce"|"armaBronceCalidad"|"armaduraBasica"|"armaduraIntermedia"|"armaduraBronce"|"hierro"|"lingoteHierro"|"armaHierro"|"armaHierroCalidad"|"armaduraBronceCalidad"|"carroGuerra"|"chatarraCobre"|"chatarraBronce"|"chatarraHierro"<br>`cantidad`: number |
| `entregarDeCaravana` | `ejercitoId`: string<br>`caravanaId`: string<br>`acuerdoId`: string |
| `calibrarReservaManual` | `asentamientoId`: string<br>`recurso`: "madera"|"piedra"|"trigo"|"cobre"|"estano"|"oro"|"livestock"|"lingoteCobre"|"lingoteEstano"|"lingoteBronce"|"cuero"|"cueroCurtido"|"cueroCalidad"|"armaMadera"|"armaCobre"|"armaBronce"|"armaBronceCalidad"|"armaduraBasica"|"armaduraIntermedia"|"armaduraBronce"|"hierro"|"lingoteHierro"|"armaHierro"|"armaHierroCalidad"|"armaduraBronceCalidad"|"carroGuerra"|"chatarraCobre"|"chatarraBronce"|"chatarraHierro"<br>`valor`: number |
| `renombrarAsentamiento` | `asentamientoId`: string<br>`nombre`: string |
| `comprometerRecinto` | `asentamientoId`: string<br>`cargo`: "gobernador"|"maestroObras"<br>`nivel`: 1|2|3 |
| `abandonarRecinto` | `asentamientoId`: string<br>`recintoId`: string |
| `solicitarAscenso` | `asentamientoId`: string |
| `adoptarTecnologia` | `faccionId`: string<br>`tecnologiaId`: "leva_comunal"|"hostigamiento_tribal"|"metalurgia_cobre"|"aleacion_bronce"|"escudos_ligeros"|"armamento_palacial"|"arqueria_palacial"|"cria_caballar"|"carros_guerra"|"canteria"|"bronce_calidad_militar"|"forja_hierro_temprana"|"panoplia_bronce"|"disciplina_formacion"|"arco_compuesto"|"equitacion_militar"|"carpinteria_militar"|"instituciones_civicas"|"ciudadania_militar"|"falange_hoplita"|"pantalla_escaramuzadores"|"arqueria_especializada"|"forja_hierro_estandarizada"|"bronce_laminado"|"caballeria_organizada"|"trabajos_asedio" |
| `empezarEpica` | `asentamientoId`: string<br>`aedaId`: string<br>`tecnologiaId`: "leva_comunal"|"hostigamiento_tribal"|"metalurgia_cobre"|"aleacion_bronce"|"escudos_ligeros"|"armamento_palacial"|"arqueria_palacial"|"cria_caballar"|"carros_guerra"|"canteria"|"bronce_calidad_militar"|"forja_hierro_temprana"|"panoplia_bronce"|"disciplina_formacion"|"arco_compuesto"|"equitacion_militar"|"carpinteria_militar"|"instituciones_civicas"|"ciudadania_militar"|"falange_hoplita"|"pantalla_escaramuzadores"|"arqueria_especializada"|"forja_hierro_estandarizada"|"bronce_laminado"|"caballeria_organizada"|"trabajos_asedio" |
| `abandonarEpica` | `asentamientoId`: string<br>`aedaId`: string |
| `comprarMirada` | `origen`: {tipo: "asentamiento"|"campamento", id: string}<br>`centro`: {x: number, y: number} |
| `comprarInformePlaza` | `origen`: {tipo: "asentamiento"|"campamento", id: string}<br>`asentamientoId`: string |
| `comprarTecnologiaAeda` | `asentamientoId`: string<br>`tecnologiaId`: "leva_comunal"|"hostigamiento_tribal"|"metalurgia_cobre"|"aleacion_bronce"|"escudos_ligeros"|"armamento_palacial"|"arqueria_palacial"|"cria_caballar"|"carros_guerra"|"canteria"|"bronce_calidad_militar"|"forja_hierro_temprana"|"panoplia_bronce"|"disciplina_formacion"|"arco_compuesto"|"equitacion_militar"|"carpinteria_militar"|"instituciones_civicas"|"ciudadania_militar"|"falange_hoplita"|"pantalla_escaramuzadores"|"arqueria_especializada"|"forja_hierro_estandarizada"|"bronce_laminado"|"caballeria_organizada"|"trabajos_asedio" |
| `mejorarRecinto` | `asentamientoId`: string<br>`cargo`: "gobernador"|"maestroObras"<br>`recintoId`: string |
| `proponerRelacion` | `tipo`: "vasallaje"|"alianza"<br>`faccionAId`: string<br>`faccionBId`: string<br>`tributoRecurso?`: "madera"|"piedra"|"trigo"|"cobre"|"estano"|"oro"|"livestock"|"lingoteCobre"|"lingoteEstano"|"lingoteBronce"|"cuero"|"cueroCurtido"|"cueroCalidad"|"armaMadera"|"armaCobre"|"armaBronce"|"armaBronceCalidad"|"armaduraBasica"|"armaduraIntermedia"|"armaduraBronce"|"hierro"|"lingoteHierro"|"armaHierro"|"armaHierroCalidad"|"armaduraBronceCalidad"|"carroGuerra"|"chatarraCobre"|"chatarraBronce"|"chatarraHierro"<br>`tributoCantidad?`: number |
| `romperRelacion` | `relacionId`: string<br>`iniciadorFaccionId`: string |
| `admitirOtrasFacciones` | `faccionId`: string<br>`admitir`: boolean |
| `designarCapital` | `faccionId`: string<br>`asentamientoId`: string |
| `rebelionVasallo` | `relacionId`: string |
| `declararGuerra` | `faccionAId`: string<br>`faccionBId`: string |
| `proponerPaz` | `relacionId`: string<br>`faccionId`: string |
| `responderRelacion` | `propuestaId`: string<br>`aceptar`: boolean |
| `retirarRelacion` | `propuestaId`: string |
| `proponerAnexion` | `faccionAId`: string<br>`faccionBId`: string |
| `responderAnexion` | `propuestaId`: string<br>`aceptar`: boolean |
| `retirarAnexion` | `propuestaId`: string |
| `proponerFusion` | `faccionAId`: string<br>`faccionBId`: string<br>`nuevoNombre`: string<br>`nuevoReyId`: string |
| `responderFusion` | `propuestaId`: string<br>`aceptar`: boolean |
| `retirarFusion` | `propuestaId`: string |
| `proponerTrueque` | `asentamientoAId`: string<br>`lineasA`: {recurso: "madera"|"piedra"|"trigo"|"cobre"|"estano"|"oro"|"livestock"|"lingoteCobre"|"lingoteEstano"|"lingoteBronce"|"cuero"|"cueroCurtido"|"cueroCalidad"|"armaMadera"|"armaCobre"|"armaBronce"|"armaBronceCalidad"|"armaduraBasica"|"armaduraIntermedia"|"armaduraBronce"|"hierro"|"lingoteHierro"|"armaHierro"|"armaHierroCalidad"|"armaduraBronceCalidad"|"carroGuerra"|"chatarraCobre"|"chatarraBronce"|"chatarraHierro", cantidad: number}[]<br>`asentamientoBId`: string<br>`lineasB`: {recurso: "madera"|"piedra"|"trigo"|"cobre"|"estano"|"oro"|"livestock"|"lingoteCobre"|"lingoteEstano"|"lingoteBronce"|"cuero"|"cueroCurtido"|"cueroCalidad"|"armaMadera"|"armaCobre"|"armaBronce"|"armaBronceCalidad"|"armaduraBasica"|"armaduraIntermedia"|"armaduraBronce"|"hierro"|"lingoteHierro"|"armaHierro"|"armaHierroCalidad"|"armaduraBronceCalidad"|"carroGuerra"|"chatarraCobre"|"chatarraBronce"|"chatarraHierro", cantidad: number}[] |
| `aceptarTrueque` | `acuerdoId`: string |
| `rechazarTrueque` | `acuerdoId`: string |
| `comerciarEnPlaza` | `heroeId`: string<br>`asentamientoId`: string<br>`ordenId`: string<br>`cantidad`: number |
| `colocarOrdenMercado` | `asentamientoId`: string<br>`tipo`: "compra"|"venta"<br>`recurso`: "madera"|"piedra"|"trigo"|"cobre"|"estano"|"oro"|"livestock"|"lingoteCobre"|"lingoteEstano"|"lingoteBronce"|"cuero"|"cueroCurtido"|"cueroCalidad"|"armaMadera"|"armaCobre"|"armaBronce"|"armaBronceCalidad"|"armaduraBasica"|"armaduraIntermedia"|"armaduraBronce"|"hierro"|"lingoteHierro"|"armaHierro"|"armaHierroCalidad"|"armaduraBronceCalidad"|"carroGuerra"|"chatarraCobre"|"chatarraBronce"|"chatarraHierro"<br>`cantidad`: number<br>`precio?`: number |
| `crearCaravana` | `asentamientoId`: string |
| `agregarCarroCaravana` | `caravanaId`: string<br>`tipoCarro`: "basico"|"reforzado" |
| `comprarAnimalCaravana` | `caravanaId`: string<br>`carroIndice`: number<br>`tipoAnimal`: "buey"|"caballo"|"camello" |
| `reservarCaravana` | `caravanaId`: string<br>`reservada`: boolean |
| `prepararCaravana` | `caravanaId`: string<br>`heroeId`: string<br>`destinoAsentamientoId`: string<br>`carga`: object<br>`escoltaEscuadronIds?`: string[]<br>`salirEn?`: number |
| `asignarEscolta` | `caravanaId`: string<br>`heroeId`: string<br>`escuadronIds`: string[] |
| `quitarEscolta` | `caravanaId`: string<br>`heroeId`: string<br>`escuadronIds?`: string[] |
| `cancelarCaravana` | `caravanaId`: string |
| `moverCarroCaravana` | `desdeCaravanaId`: string<br>`haciaCaravanaId`: string<br>`carroIndice`: number |
| `moverCargaCaravanaAparcada` | `heroeId`: string<br>`caravanaId`: string<br>`asentamientoId`: string<br>`recurso`: "madera"|"piedra"|"trigo"|"cobre"|"estano"|"oro"|"livestock"|"lingoteCobre"|"lingoteEstano"|"lingoteBronce"|"cuero"|"cueroCurtido"|"cueroCalidad"|"armaMadera"|"armaCobre"|"armaBronce"|"armaBronceCalidad"|"armaduraBasica"|"armaduraIntermedia"|"armaduraBronce"|"hierro"|"lingoteHierro"|"armaHierro"|"armaHierroCalidad"|"armaduraBronceCalidad"|"carroGuerra"|"chatarraCobre"|"chatarraBronce"|"chatarraHierro"<br>`cantidad`: number<br>`sentido`: "cargar"|"descargar" |
| `enviarCaravanaAlOrigen` | `heroeId`: string<br>`caravanaId`: string<br>`asentamientoId`: string |
| `reclutarTropa` | `asentamientoId`: string<br>`heroeId`: string<br>`tropaId`: string<br>`origen?`: "pesants"|"artesanos" |
| `conectarse` | `heroeId`: string |
| `desconectarse` | `heroeId`: string |
| `salirAlMundo` | `asentamientoId`: string<br>`heroeId`: string<br>`escuadronIds`: string[]<br>`carga`: object |
| `marcharA` | `heroeId`: string<br>`objetivo`: {tipo: "asentamiento", id: string}|{tipo: "punto", punto: {x: number, y: number}} |
| `unirseEnCampo` | `ejercitoId`: string<br>`heroeId`: string |
| `responderPeticionDeUnion` | `ejercitoId`: string<br>`heroeId`: string<br>`solicitanteId`: string<br>`aceptar`: boolean |
| `separarseDelEjercito` | `heroeId`: string |
| `organizarEjercito` | `heroeId`: string<br>`politicaDeUnion`: "aceptar"|"preguntar" |
| `cancelarFormacion` | `heroeId`: string |
| `convocarEjercito` | `heroeId`: string<br>`politicaDeUnion`: "aceptar"|"preguntar"<br>`escuadronIds`: string[]<br>`carga`: object |
| `unirseAConvocatoria` | `heroeId`: string<br>`convocatoriaId`: string<br>`escuadronIds`: string[]<br>`carga`: object |
| `responderPeticionDeConvocatoria` | `heroeId`: string<br>`convocatoriaId`: string<br>`solicitanteId`: string<br>`aceptar`: boolean |
| `cambiarSeleccionDeConvocatoria` | `heroeId`: string<br>`escuadronIds`: string[]<br>`carga`: object |
| `separarseDeConvocatoria` | `heroeId`: string |
| `cancelarConvocatoria` | `heroeId`: string |
| `partirConvocatoria` | `heroeId`: string |
| `inspeccionar` | `heroeId`: string<br>`objetivo`: {tipo: "ejercito", id: string}|{tipo: "caravana", id: string}|{tipo: "asentamiento", id: string} |
| `atacar` | `heroeId`: string<br>`objetivo`: {tipo: "ejercito", id: string}|{tipo: "caravana", id: string}|{tipo: "campamento"|"asentamiento", id: string} |
| `perseguir` | `heroeId`: string<br>`objetivo`: {tipo: "ejercito", id: string}|{tipo: "caravana", id: string} |
| `dejarDePerseguir` | `heroeId`: string |
| `unirseABatalla` | `heroeId`: string<br>`battleId`: string<br>`lado?`: "atacante"|"defensor" |
| `cancelarBatalla` | `battleId`: string |
| `cederLiderazgo` | `ejercitoId`: string<br>`heroeId`: string<br>`sucesorId`: string |
| `entrarEnAsentamiento` | `asentamientoId`: string<br>`heroeId`: string |
| `fijarPuerta` | `asentamientoId`: string<br>`heroeId`: string<br>`cerradaA`: "neutrales"|"aliados"|"enemigos"|"aedas"[] |
| `vetarJugador` | `asentamientoId`: string<br>`heroeId`: string<br>`vetadoId`: string<br>`vetar`: boolean |
| `salirDeAsentamiento` | `asentamientoId`: string<br>`heroeId`: string |
| `entrarEnCampamento` | `campamentoId`: string<br>`heroeId`: string |
| `salirDelCampamento` | `campamentoId`: string<br>`heroeId`: string<br>`escuadronIds`: string[]<br>`carga`: object<br>`politicaDeUnion?`: "rechazar"|"aceptar"|"preguntar"<br>`objetivo?`: {tipo: "asentamiento", id: string}|{tipo: "punto", punto: {x: number, y: number}} |
| `guarnecer` | `asentamientoId`: string<br>`heroeId`: string |
| `movilizarEjercito` | `asentamientoId`: string<br>`heroeId`: string<br>`escuadronIds`: string[]<br>`objetivo?`: {tipo: "asentamiento", id: string}|{tipo: "punto", punto: {x: number, y: number}}<br>`politicaDeUnion?`: "rechazar"|"aceptar"|"preguntar"<br>`carga?`: object |
| `unirseAEjercito` | `ejercitoId`: string<br>`asentamientoId`: string<br>`heroeId`: string<br>`escuadronIds`: string[] |
| `replegarEjercito` | `ejercitoId`: string |
| `estacionarEjercito` | `ejercitoId`: string |

## Cambios de la reconstrucción (2026-10-13)

Esta tabla es la del motor TypeScript. En el contrato 05 (Protobuf):

- **Se retira** `replegarEjercito`.
- **Se amplía** `atacar`: `declaraGuerra?: boolean` y los objetivos `pueblo` y `avanzada`.
- **Nuevos**: `repostar { heroeId }`, `reclamarCaravana { heroeId, caravanaId, politicaDeUnion }`, `marcarPueblo`, `colocarAvanzada`, `entregarEnObra`, `cancelarObra`, `destruirPueblo`, `destruirAvanzada`, `mejorarAvanzada`, `repararAvanzada`, `elegirTropaDeAvanzada`, `guarnecerAvanzada`, `reponerCarroDeAcarreo`.
- La frontera «forma en el borde, reglas en el motor» se conserva.

