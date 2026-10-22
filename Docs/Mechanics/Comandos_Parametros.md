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
- **Órdenes de avanzadas del núcleo Rust (2026-10-22, fase 4F; mapeado al contrato 05 en 4H T2, ver el último apartado):** `ColocarAvanzada { asentamiento, heroe, posicion }` (106, `colocarAvanzada`), `EntregarMaterialesAAvanzada { ejercito, avanzada }` (107, la rama avanzada de `entregarEnObra`), `MejorarAvanzada { heroe, avanzada }` (108), `RepararAvanzada { heroe, avanzada }` (109), `DestruirAvanzada { heroe, avanzada }` (110), `ElegirTropaDeAvanzada { heroe, avanzada, tropa }` (111; el contrato solo trae `tropa_elegida_escalon2`, el núcleo guarda una por escalón 2, 3 y 4), `GuarnecerAvanzada { heroe, avanzada, escuadras }` (112), `RetirarDeAvanzada { heroe, avanzada, escuadras }` (113, no está en el contrato), `RepostarEnAvanzada { heroe, avanzada }` (114, la rama avanzada de `repostar`) y `DepositarEnAvanzada { ejercito, avanzada, recurso, cantidad }` (115, no está en el contrato) y `CancelarObraDeAvanzada { heroe, avanzada }` (116, 2026-10-22: la rama avanzada de `cancelarObra`, `OrdenCancelarObra { heroe_id, obra_id }`; cancela una mejora o una reparación, sin reembolso). `atacar` ya trae el objetivo `avanzada` (95). Todas llevan el candado de las batallas y nombran la avanzada (`Comando::implicados`). `UnirseABatalla` pide además estar dentro para defender (`Movimiento_Ejercitos.md` §11.1).
- **Órdenes de «entrar con todo» del núcleo Rust (2026-10-22; mapeado al contrato 05 en 4H T2, ver el último apartado):** `FijarAcceso { asentamiento, aliados, neutrales, enemigos }` (117, provisional de `fijarPuerta`: la puerta de la plaza por relación; lo fija quien manda en ella y los pueblos y las avanzadas lo heredan) y `VaciarCarro { heroe }` (118: el carro que el residente trajo a su residencia, al almacén personal hasta donde quepa). Rechazos nuevos: `carro.vaciar_solo_en_tu_residencia`, `almacen_personal.lleno`; desaparecen `columna.no_aparcada` y `caravana.ajena_al_lugar`. Ver [`Presencia_Y_Convocatoria.md`](Presencia_Y_Convocatoria.md). `EntrarEnAsentamiento`, `SalirDeAsentamiento`, `Guarnecer`, `EntrarEnCampamento` y `SalirDelCampamento` mantienen su código (72-75, 71) con la semántica nueva.
- **Órdenes de entrar en un pueblo o en una avanzada del núcleo Rust (2026-10-22, segunda ronda; mapeado al contrato 05 en 4H T2, ver el último apartado):** `EntrarEnPueblo { pueblo, heroe }` (119), `SalirDePueblo { pueblo, heroe }` (120), `EntrarEnAvanzada { avanzada, heroe }` (121) y `SalirDeAvanzada { avanzada, heroe }` (122), a continuación de la 118. Entran con todo, de visita, con el acceso de la plaza madre (`puerta.cerrada`); un ejército entra por su Líder. Rechazos nuevos: `heroe.no_esta_en_el_pueblo`, `heroe.no_esta_en_la_avanzada` (más los que ya existían: `pueblo.desconocido`, `pueblo.no_esta_activo`, `pueblo.fuera_de_la_puerta`, `avanzada.desconocida`, `avanzada.no_operativa`, `avanzada.fuera_de_la_puerta`, `puerta.cerrada`, `columna.en_formacion`, `columna.solo_el_lider`, `batalla.bloqueado`). `FijarAcceso` (117) ahora también saca al mapa a quien ya no puede entrar y se rechaza con `batalla.bloqueado` si un pueblo o una avanzada de la plaza está en una batalla. `UnirseABatalla` rechaza con `batalla.invalida` («para atacar hay que estar fuera…») a quien está dentro. Hechos nuevos: `jugador.entra_en_pueblo`, `ejercito.entra_en_pueblo`, `jugador.sale_de_pueblo`, `jugador.entra_en_avanzada`, `ejercito.entra_en_avanzada`, `jugador.sale_de_avanzada`, `heroe.expulsado`; `carroPerdido` en `asentamiento.conquistada`, `combate.pueblo_saqueado`/`pueblo_resiste` y los de avanzada. Foto `BAN18`.
- **`UnirseABatalla` en la tercera ronda de «entrar con todo» (2026-10-22):** ni comando nuevo ni rechazo nuevo ni cambio de foto (`BAN18`). Quien está **dentro** de la plaza asediada, del pueblo saqueado o de la avanzada atacada y es de la Facción dueña, aliado o vasallo se une a la defensa (sin columna ni distancia: es el único participante, con lo que lleva encima); desde **fuera**, a ≤ `radioEncuentro`, se une al ataque la Facción atacante y los neutrales y enemigos del dueño. Los rechazos siguen siendo `batalla.invalida`, con otros mensajes: «para defender hay que estar dentro…» (un amigo del dueño desde fuera) y «para atacar hay que estar fuera…» (un neutral o un enemigo dentro, que antes recibía «tu Facción no combate aquí»; este mensaje queda para las caravanas). No hay hecho de invitación: el cliente muestra la opción por elegibilidad, y el estado de asedio es consultable con `Nucleo::batalla_sobre` (su proyección al contrato es la 4H).

## Órdenes del contrato en 4H (T1, 2026-10-22)

Cambios **aditivos** a `orders.proto` (ningún número ni campo anterior cambia; el contrato pasa de 135 a 145 órdenes, `Orden.cuerpo` 145-154). Sirven al núcleo Rust, que ya tiene los comandos; su traducción, en `sesiones::ordenes`, es la tanda T2 (hecha: ver el último apartado).

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
- **`OrdenDeAdministracion { codigo, parametros }`** (decisión del autor, 2026-10-22): una sola orden genérica, solo para sesiones `CLIENTE_ADMIN_WEB` y temporal (desaparece con los sistemas político y de tecnología). Cubre los 19 comandos de andamiaje del núcleo sin orden propia: `crear_ejercito` (1), `fijar_nivel_faccion` (5), `dar_caravana_fundacion` (6), `fijar_nivel_asentamiento` (8), `agrandar_zona` (9), `reclamar_parcela` (10), `soltar_parcela` (11), `dar_recursos` (37), `dar_suministro` (38), `fijar_residencia` (43), `dar_almacen_personal` (44), `fijar_tecnologias` (65), `fijar_faccion_de_heroe` (66), `colocar_heroe` (67), `herir_heroe` (68), `fijar_relacion` (94), `colocar_bandidos` (96), `dar_escuadra` (97) y `fijar_controlador` (98). Los `parametros` llevan los campos del comando con los ids del contrato (`f-N`, `as-N`, `heroe-N`...) y las cantidades en milésimas; la forma exacta es la del último apartado.
- **Numeración:** el núcleo Rust numera **125** comandos (los códigos de `Comando`, 1-125); el contrato, **145** órdenes (135 + 10). No son la misma lista: 46 y 107 comparten `entregar_en_obra`, 83 y 114 comparten `repostar`, y hay órdenes del contrato sin comando (política, tecnología, refundación...).

## Traducción de las órdenes del contrato (4H, T2, 2026-10-22)

`sesiones::ordenes::traducir` (`crates/nucleo/src/sesiones/ordenes.rs`) lleva cada `Orden` del contrato 05 a una `Peticion` del núcleo. Las **145 órdenes** se reparten así: **99 órdenes del jugador** (que dan **101 comandos**: `entregar_en_obra` y `repostar` dan dos según la obra o el lugar), **`administracion`** (19 comandos de andamiaje, solo en una sesión de administración) y **45 sin comando en el núcleo** (`orden.no_disponible`). De los 125 comandos del núcleo, 101 vienen de una orden del jugador, 19 de la orden de administración y 5 (101-105) del canal del servidor de batalla (`MensajeDeServidorDeBatalla`). La prueba `sesiones::ordenes::pruebas::todo_comando_esta_clasificado_y_toda_orden_del_contrato_esta_mapeada_o_es_no_disponible` lo exige: lee las variantes de `Comando` de su definición, cuenta las órdenes de `orders.proto`, y el `match` sobre `Orden.cuerpo` no tiene comodín, así que un comando o una orden nuevos no compilan ni pasan hasta clasificarse.

**Reglas de la traducción**

- **Autorización**: la del núcleo (`Peticion.actor` = el cliente de la sesión, `Peticion.admin` = sesión `CLIENTE_ADMIN_WEB`). La traducción solo decide que la orden de administración no vale fuera de una sesión de administración (`no_autorizado`).
- **Lo implícito** se resuelve preguntando al mundo (`Consulta`): *el héroe de la cuenta* cuando la orden no nombra héroe (`heroe_de_cuenta`: el que no es de campo de la administración); *la columna del héroe* (`marchar_a`, `fundar`, `comerciar_en_plaza`, `entregar_en_obra`); *la plaza a la que se marcha* (`marchar_a` a un asentamiento termina en su posición, dentro de su `radioPuerta`); *el lugar donde repostar* (`repostar` sin `lugar_id`: la plaza o avanzada operativa más cercana a `radioPuerta` de la columna, las plazas antes que las avanzadas; sin ninguna, `repostar.sin_lugar`); *el campamento* (`comprar_en_campamento` y `repostar_en_campamento` sin `campamento_id`: aquel en el que está el héroe, dentro o a su puerta; si no, `heroe.no_esta_en_el_campamento`); *la obra* (`obra-N` es el pueblo o la avanzada `N`; los ids de pueblo y avanzada salen del mismo contador, así que no se confunden; si no es ninguno, `obra_de_mapa.desconocida`); y *la batalla* (su UUID a su id; `batalla.no_existe`).
- **Rechazos de forma**: un id que no tiene la forma de su entidad se rechaza con el código de «desconocida» de esa entidad (`heroe.desconocido`, `ejercito.desconocido`, `asentamiento.desconocido`, `caravana.desconocida`, `trueque.desconocido`, `orden.desconocida`, `pueblo.desconocido`, `avanzada.desconocida`, `convocatoria.desconocida`, `loadout.desconocido`, `campamento.desconocido`, `bandidos.desconocido`, `pueblo.nodo_desconocido`, `edificio.desconocido`, `murallas.recinto_desconocido`). Un valor fuera de un catálogo cerrado (recurso, tropa, tipo de edificio, cargo, política de unión, tipo de carro, animal, sentido, dirección, lado, grupo de la puerta, nivel de recinto), un id de escuadrón mal formado, un índice o perk negativo y todo campo obligatorio ausente, con `orden.forma_invalida`. Una orden vacía, con `orden.no_soportada`. Ninguno deja rastro en el núcleo.
- **Nada se descarta**: las 45 órdenes sin comando se rechazan con `orden.no_disponible` (el mensaje nombra el sistema que las traerá).

**Órdenes del jugador (99)**: orden → comando (código del núcleo); entre paréntesis, lo que la traducción ignora o decide.

| Familia | Orden → comando |
|---|---|
| Presencia y héroe | `crear_heroe` → `CrearHeroe` (49; la cuenta de la sesión, controlador `Humano`; ignora `display_name`, `class_definition_id`, `genero` y `avatar`), `reclutar_tropa` → `ReclutarTropa` (50; ignora `origen`), `reclutar_en_campamento` → `ReclutarEnCampamento` (51; `pagar_con` por defecto `almacenPersonal`), `pedir_prestamo` (52), `reponer_prestamo` (53), `guardar_loadout` (54), `borrar_loadout` (55), `repartir_puntos` (56; un atributo ausente es 0), `ordenar_escuadras` (57), `asignar_guarnicion` (58), `retirar_guarnicion` (59) y `pasar_a_viveres` (60), todas con el héroe de la cuenta; `asignar_escolta` (61), `quitar_escolta` (62; sin escuadrones retira todas las cedidas), `desconectarse` (63), `conectarse` (64), `comprar_en_campamento` → `ComprarEnCampamento` (41; héroe de la cuenta), `repostar_en_campamento` → `RepostarEnCampamento` (42) |
| Columnas y convocatorias | `marchar_a` → `MarcharA` (2), `estacionar_ejercito` → `Estacionar` (3), `controlar_reloj` → `Reloj` (4; `pausar`, `reanudar`, `fijar_factor`, `avanzar_hasta`), `fundar` → `Fundar` (7; la columna del héroe de la cuenta), `salir_al_mundo` (69), `movilizar_ejercito` (70; `politica_de_union` por defecto `rechazar`; con `objetivo` se rechaza con `objetivo.no_soportado`: sale sin destino y después `marchar_a`), `salir_del_campamento` (71; igual), `entrar_en_asentamiento` (72), `salir_de_asentamiento` (73), `guarnecer` (74), `entrar_en_campamento` (75), `unirse_a_ejercito` (76; sin carga: el contrato no la trae), `unirse_en_campo` (77), `responder_peticion_de_union` (78), `separarse_del_ejercito` (79), `ceder_liderazgo` (80; ignora `ejercito_id`), `organizar_ejercito` (81), `cancelar_formacion` (82), `repostar` → `Repostar` (83) o `RepostarEnAvanzada` (114) según `lugar_id` (`as-N` o `av-N`) o el lugar más cercano, `convocar_ejercito` (84), `unirse_a_convocatoria` (85), `responder_peticion_de_convocatoria` (86), `cambiar_seleccion_de_convocatoria` (87), `separarse_de_convocatoria` (88), `cancelar_convocatoria` (89), `partir_convocatoria` (90) |
| Combate y batalla | `perseguir` (91), `dejar_de_perseguir` (92), `inspeccionar` (93), `atacar` (95; los seis objetivos, `campamento_id` es un campamento de bandidos `band-N`; `declara_guerra` ausente = `false`), `unirse_a_batalla` (99; `battle_id` es el UUID), `cancelar_batalla` (100; el héroe de la cuenta, que ha de ser quien la inició) |
| Avanzadas | `colocar_avanzada` (106), `entregar_en_obra` → `EntregarMateriales` (46, pueblo) o `EntregarMaterialesAAvanzada` (107, avanzada; ignora `materiales`), `cancelar_obra` → `CancelarObraDeAvanzada` (116; la obra de un pueblo se rechaza con `objetivo.no_soportado`: es `destruir_pueblo`), `mejorar_avanzada` (108), `reparar_avanzada` (109), `destruir_avanzada` (110), `elegir_tropa_de_avanzada` (111), `guarnecer_avanzada` (112), `retirar_de_avanzada` (113), `depositar_en_avanzada` (115), `entrar_en_avanzada` (121), `salir_de_avanzada` (122) |
| Acceso y murallas | `fijar_puerta` → `FijarAcceso` (117; los grupos de `cerrada_a` quedan a `false` y el resto a `true`; `aedas` se acepta y no cierra nada), `vaciar_carro` (118), `comprometer_recinto` (123; `nivel` `"1"`, `"2"` o `"3"`), `mejorar_recinto` (124), `abandonar_recinto` (125); `recinto-<plaza>-<n>` ha de ser de la plaza de la orden |
| Economía y construcción | `anadir_edificio_manualmente` → `AnadirEdificio` (12; ignora `cargo`), `quitar_de_cola` (13), `mover_en_cola` (14; `arriba` o `abajo`), `mejorar_edificio_ahora` → `MejorarEdificio` (15), `alternar_auto_construccion` (16), `alternar_receta` (17), `calibrar_reserva_manual` → `CalibrarReserva` (18), `solicitar_ascenso` (19), `activar_politica` (20; el `cargo` sí se pasa) |
| Comercio | `proponer_trueque` (21), `aceptar_trueque` (22), `rechazar_trueque` (23), `colocar_orden_mercado` (24; `compra` o `venta`; `precio` ausente = el de referencia), `cancelar_orden_mercado` (25), `comerciar_en_plaza` (26; la columna del héroe), `crear_caravana` (27), `agregar_carro_caravana` → `AnadirCarro` (28; `basico` o `reforzado`), `comprar_animal_caravana` → `AnadirAnimal` (29; `buey`, `caballo` o `camello`), `mover_carro_caravana` → `MoverCarro` (30), `reservar_caravana` (31), `preparar_caravana` (32; con `escolta_escuadron_ids` se rechaza con `caravana.escolta_invalida`: la escolta se cede antes con `asignar_escolta`), `cancelar_caravana` (33), `mover_carga_caravana_aparcada` → `MoverCargaAparcada` (34; `cargar` o `descargar`), `enviar_caravana_al_origen` (35), `entregar_de_caravana` (36), `adjuntar_caravana` (39), `aparcar_caravana` (40) |
| Pueblos extractores | `marcar_pueblo` (45; `nodo-<n>` es el índice del nodo del artefacto de mundo), `destruir_pueblo` (47), `reponer_carro_de_acarreo` → `ReponerCarro` (48), `entrar_en_pueblo` (119), `salir_de_pueblo` (120) |

En las órdenes de obras y de murallas el `cargo` (Gobernador o Maestro de Obras) solo se pasa en `comprometer_recinto`, `mejorar_recinto` y `activar_politica`: los cargos son del sistema 5 y el núcleo autoriza con `manda_en`. El `heroe_id` de `adjuntar_caravana`, `mover_carga_caravana_aparcada`, `enviar_caravana_al_origen`, `preparar_caravana`, `marcar_pueblo`, `destruir_pueblo`, `reponer_carro_de_acarreo` y `fijar_puerta` tampoco cambia el comando: la autorización es por la plaza o por la columna.

**Órdenes sin comando en el núcleo (45, `orden.no_disponible`)**

| Grupo | Órdenes |
|---|---|
| Política y Facciones (sistema 5; 28) | `crear_faccion`, `solicitar_ingreso`, `dejar_faccion`, `responder_solicitud`, `asignar_rey`, `asignar_embajador`, `asignar_cargo_local`, `designar_capital`, `admitir_otras_facciones`, `proponer_relacion`, `responder_relacion`, `retirar_relacion`, `proponer_anexion`, `responder_anexion`, `retirar_anexion`, `proponer_fusion`, `responder_fusion`, `retirar_fusion`, `proponer_paz`, `romper_relacion`, `rebelion_vasallo`, `declarar_guerra`, `vetar_jugador`, `renombrar_asentamiento`, `alternar_reabastecer_aliados`, `cambiar_residencia`, `residir_en_campamento`, `dejar_residencia` |
| Tecnología, Aedas e intel (sistema 6 y Taberna; 7) | `adoptar_tecnologia`, `empezar_epica`, `abandonar_epica`, `comprar_tecnologia_aeda`, `comprar_mirada`, `comprar_informe_plaza`, `abrir_alijo` |
| Refundación (5) | `aportar_a_refundacion`, `retirar_de_refundacion`, `comprar_caravana_de_refundacion`, `lanzar_caravana_fundacion`, `desarmar_caravana_fundacion` |
| Caravanas a mano y almacén personal (deuda de columnas; 5) | `cargar_caravana`, `reclamar_caravana`, `soltar_caravana`, `guardar_en_almacen_personal`, `sacar_del_almacen_personal` |

**`administracion { codigo, parametros }` (solo sesión de administración, 19 códigos)**. `parametros` es un `Struct`: los ids como en el resto del contrato (`f-N`, `as-N`, `ej-N`, `heroe-N`, `camp-N`), las cantidades y las coordenadas en milésimas (números enteros: un `Struct` solo lleva `double`, exacto hasta 2⁵³), `posicion` como `{x, y}`. Lo opcional ausente o nulo es `None`. Un código desconocido, un parámetro que falta o de otro tipo, se rechaza con `orden.forma_invalida`.

| `codigo` | Parámetros | Comando (código) |
|---|---|---|
| `crear_ejercito` | `cuenta`, `faccion_id`, `posicion`, `velocidad`, `viveres` | `CrearEjercito` (1) |
| `fijar_nivel_faccion` | `faccion_id`, `nivel` | `FijarNivelFaccion` (5) |
| `dar_caravana_fundacion` | `ejercito_id` | `DarCaravanaFundacion` (6) |
| `fijar_nivel_asentamiento` | `asentamiento_id`, `nivel` | `FijarNivelAsentamiento` (8) |
| `agrandar_zona` | `asentamiento_id` | `AgrandarZona` (9) |
| `reclamar_parcela` | `asentamiento_id` | `ReclamarParcela` (10) |
| `soltar_parcela` | `asentamiento_id`, `parcela` | `SoltarParcela` (11) |
| `dar_recursos` | `asentamiento_id`, `recurso`, `cantidad` | `DarRecursos` (37) |
| `dar_suministro` | `ejercito_id`, `recurso`, `cantidad` | `DarSuministro` (38) |
| `fijar_residencia` | `heroe_id`, `residencia?` (`as-N` o `camp-N`) | `FijarResidencia` (43) |
| `dar_almacen_personal` | `heroe_id`, `recurso`, `cantidad` | `DarAlmacenPersonal` (44) |
| `fijar_tecnologias` | `faccion_id`, `tecnologias` (lista de ids de catálogo) | `FijarTecnologias` (65) |
| `fijar_faccion_de_heroe` | `heroe_id`, `faccion_id?` | `FijarFaccionDeHeroe` (66) |
| `colocar_heroe` | `heroe_id`, `ubicacion` (`as-N`, `camp-N` o `ej-N`) | `ColocarHeroe` (67) |
| `herir_heroe` | `heroe_id` | `HerirHeroe` (68) |
| `fijar_relacion` | `faccion_a_id`, `faccion_b_id`, `relacion?` (`alianza`, `vasallaje` o `guerra`) | `FijarRelacion` (94) |
| `colocar_bandidos` | `posicion`, `nivel` | `ColocarBandidos` (96) |
| `dar_escuadra` | `heroe_id`, `tropa_id`, `cantidad` | `DarEscuadra` (97) |
| `fijar_controlador` | `heroe_id`, `controlador` (`humano` o `bot`) | `FijarControlador` (98) |

**Códigos nuevos de rechazo**: `orden.no_disponible`, `orden.forma_invalida`, `obra_de_mapa.desconocida` y `repostar.sin_lugar`. Los ids nuevos del contrato: `nodo-<n>` (el nodo del artefacto de mundo).
