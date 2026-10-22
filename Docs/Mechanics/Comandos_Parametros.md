# Forma de los parámetros de cada comando

**Alcance y fuentes.** La forma de `params` de los 122 comandos **del motor TypeScript** (el núcleo Rust numera 125 comandos y el contrato 05 trae 145 órdenes: ver el último apartado), tal como la valida el servidor en el borde (`src/session/comandos/esquemas.ts`,
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
- **Órdenes de avanzadas del núcleo Rust (2026-10-22, fase 4F; sin mapear aún al contrato 05):** `ColocarAvanzada { asentamiento, heroe, posicion }` (106, `colocarAvanzada`), `EntregarMaterialesAAvanzada { ejercito, avanzada }` (107, la rama avanzada de `entregarEnObra`), `MejorarAvanzada { heroe, avanzada }` (108), `RepararAvanzada { heroe, avanzada }` (109), `DestruirAvanzada { heroe, avanzada }` (110), `ElegirTropaDeAvanzada { heroe, avanzada, tropa }` (111; el contrato solo trae `tropa_elegida_escalon2`, el núcleo guarda una por escalón 2, 3 y 4), `GuarnecerAvanzada { heroe, avanzada, escuadras }` (112), `RetirarDeAvanzada { heroe, avanzada, escuadras }` (113, no está en el contrato), `RepostarEnAvanzada { heroe, avanzada }` (114, la rama avanzada de `repostar`) y `DepositarEnAvanzada { ejercito, avanzada, recurso, cantidad }` (115, no está en el contrato) y `CancelarObraDeAvanzada { heroe, avanzada }` (116, 2026-10-22: la rama avanzada de `cancelarObra`, `OrdenCancelarObra { heroe_id, obra_id }`; cancela una mejora o una reparación, sin reembolso). `atacar` ya trae el objetivo `avanzada` (95). Todas llevan el candado de las batallas y nombran la avanzada (`Comando::implicados`). `UnirseABatalla` pide además estar dentro para defender (`Movimiento_Ejercitos.md` §11.1).
- **Órdenes de «entrar con todo» del núcleo Rust (2026-10-22; sin mapear aún al contrato 05):** `FijarAcceso { asentamiento, aliados, neutrales, enemigos }` (117, provisional de `fijarPuerta`: la puerta de la plaza por relación; lo fija quien manda en ella y los pueblos y las avanzadas lo heredan) y `VaciarCarro { heroe }` (118: el carro que el residente trajo a su residencia, al almacén personal hasta donde quepa). Rechazos nuevos: `carro.vaciar_solo_en_tu_residencia`, `almacen_personal.lleno`; desaparecen `columna.no_aparcada` y `caravana.ajena_al_lugar`. Ver [`Presencia_Y_Convocatoria.md`](Presencia_Y_Convocatoria.md). `EntrarEnAsentamiento`, `SalirDeAsentamiento`, `Guarnecer`, `EntrarEnCampamento` y `SalirDelCampamento` mantienen su código (72-75, 71) con la semántica nueva.
- **Órdenes de entrar en un pueblo o en una avanzada del núcleo Rust (2026-10-22, segunda ronda; sin mapear aún al contrato 05):** `EntrarEnPueblo { pueblo, heroe }` (119), `SalirDePueblo { pueblo, heroe }` (120), `EntrarEnAvanzada { avanzada, heroe }` (121) y `SalirDeAvanzada { avanzada, heroe }` (122), a continuación de la 118. Entran con todo, de visita, con el acceso de la plaza madre (`puerta.cerrada`); un ejército entra por su Líder. Rechazos nuevos: `heroe.no_esta_en_el_pueblo`, `heroe.no_esta_en_la_avanzada` (más los que ya existían: `pueblo.desconocido`, `pueblo.no_esta_activo`, `pueblo.fuera_de_la_puerta`, `avanzada.desconocida`, `avanzada.no_operativa`, `avanzada.fuera_de_la_puerta`, `puerta.cerrada`, `columna.en_formacion`, `columna.solo_el_lider`, `batalla.bloqueado`). `FijarAcceso` (117) ahora también saca al mapa a quien ya no puede entrar y se rechaza con `batalla.bloqueado` si un pueblo o una avanzada de la plaza está en una batalla. `UnirseABatalla` rechaza con `batalla.invalida` («para atacar hay que estar fuera…») a quien está dentro. Hechos nuevos: `jugador.entra_en_pueblo`, `ejercito.entra_en_pueblo`, `jugador.sale_de_pueblo`, `jugador.entra_en_avanzada`, `ejercito.entra_en_avanzada`, `jugador.sale_de_avanzada`, `heroe.expulsado`; `carroPerdido` en `asentamiento.conquistada`, `combate.pueblo_saqueado`/`pueblo_resiste` y los de avanzada. Foto `BAN18`.
- **`UnirseABatalla` en la tercera ronda de «entrar con todo» (2026-10-22):** ni comando nuevo ni rechazo nuevo ni cambio de foto (`BAN18`). Quien está **dentro** de la plaza asediada, del pueblo saqueado o de la avanzada atacada y es de la Facción dueña, aliado o vasallo se une a la defensa (sin columna ni distancia: es el único participante, con lo que lleva encima); desde **fuera**, a ≤ `radioEncuentro`, se une al ataque la Facción atacante y los neutrales y enemigos del dueño. Los rechazos siguen siendo `batalla.invalida`, con otros mensajes: «para defender hay que estar dentro…» (un amigo del dueño desde fuera) y «para atacar hay que estar fuera…» (un neutral o un enemigo dentro, que antes recibía «tu Facción no combate aquí»; este mensaje queda para las caravanas). No hay hecho de invitación: el cliente muestra la opción por elegibilidad, y el estado de asedio es consultable con `Nucleo::batalla_sobre` (su proyección al contrato es la 4H).

## Órdenes del contrato en 4H (T1, 2026-10-22)

Cambios **aditivos** a `orders.proto` (ningún número ni campo anterior cambia; el contrato pasa de 135 a 145 órdenes, `Orden.cuerpo` 145-154). Sirven al núcleo Rust, que ya tiene los comandos; su traducción en `sesiones::traducir` es la tanda T2.

| Orden (`Orden.cuerpo`) | Campos | Comando del núcleo |
|---|---|---|
| `cancelar_orden_mercado` (145) | `orden_id` | `CancelarOrdenMercado` (25) |
| `aparcar_caravana` (146) | `caravana_id`, `asentamiento_id` | `AparcarCaravana` (40) |
| `retirar_de_avanzada` (147) | `heroe_id`, `avanzada_id`, `escuadron_ids[]` | `RetirarDeAvanzada` (113) |
| `depositar_en_avanzada` (148) | `ejercito_id`, `avanzada_id`, `recurso`, `cantidad` (milésimas) | `DepositarEnAvanzada` (115) |
| `vaciar_carro` (149) | `heroe_id` | `VaciarCarro` (118) |
| `entrar_en_pueblo` (150), `salir_de_pueblo` (151) | `pueblo_id`, `heroe_id` | `EntrarEnPueblo` (119), `SalirDePueblo` (120) |
| `entrar_en_avanzada` (152), `salir_de_avanzada` (153) | `avanzada_id`, `heroe_id` | `EntrarEnAvanzada` (121), `SalirDeAvanzada` (122) |
| `administracion` (154) | `codigo`, `parametros` (`Struct`) | el andamiaje de administración (abajo) |

- `repostar` gana `lugar_id?` (plaza `as-N` o avanzada `av-N`): cubre las órdenes 83 (`Repostar`, plaza) y 114 (`RepostarEnAvanzada`); sin él, el servidor busca la plaza o avanzada operativa más cercana a `radioPuerta` de la columna y, sin ninguna, rechaza con `repostar.sin_lugar`.
- El código 103 (`RegistrarTokens`) no es una orden de jugador: es el mensaje `TokensBatalla` de `MensajeDeServidorDeBatalla.tokens` (sesión de servidor de batalla).
- **`OrdenDeAdministracion { codigo, parametros }`** (decisión del autor, 2026-10-22): una sola orden genérica, solo para sesiones `CLIENTE_ADMIN_WEB` y temporal (desaparece con los sistemas político y de tecnología). Cubre los 19 comandos de andamiaje del núcleo sin orden propia: `crear_ejercito` (1), `fijar_nivel_faccion` (5), `dar_caravana_fundacion` (6), `fijar_nivel_asentamiento` (8), `agrandar_zona` (9), `reclamar_parcela` (10), `soltar_parcela` (11), `dar_recursos` (37), `dar_suministro` (38), `fijar_residencia` (43), `dar_almacen_personal` (44), `fijar_tecnologias` (65), `fijar_faccion_de_heroe` (66), `colocar_heroe` (67), `herir_heroe` (68), `fijar_relacion` (94), `colocar_bandidos` (96), `dar_escuadra` (97) y `fijar_controlador` (98). Los `parametros` llevan los campos del comando con los ids del contrato (`f-N`, `as-N`, `heroe-N`...) y las cantidades en milésimas; la forma exacta la fija T2.
- **Numeración:** el núcleo Rust numera **125** comandos (los códigos de `Comando`, 1-125); el contrato, **145** órdenes (135 + 10). No son la misma lista: 46 y 107 comparten `entregar_en_obra`, 83 y 114 comparten `repostar`, y hay órdenes del contrato sin comando (política, tecnología, refundación...).
