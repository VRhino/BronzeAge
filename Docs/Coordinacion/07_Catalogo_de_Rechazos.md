# 07. Catálogo de rechazos

El catálogo **estable** de los códigos con que el núcleo rechaza una orden (`RechazoOrden.codigo` de `ResultadoOrden`, contrato 05 §6) o un mensaje del servidor de batalla (`AcuseBatalla.rechazo`). Decisión 7 de la fase 4H (`Docs/Arquitectura/4H_Contraste_y_Plan.md` §5): **un cliente decide por el código, nunca por el texto**, que es solo para personas y puede cambiar. Un código de esta lista no cambia de significado ni se reutiliza; uno que deje de usarse se marca aquí como retirado en lugar de borrarse.

- **De dónde sale.** El bloque de abajo se escribe a partir del módulo `rechazos` de `crates/nucleo/src/comando.rs` (una constante por código, agrupadas por los comentarios del módulo, con lo que dice cada una). La prueba `cada_rechazo_del_nucleo_esta_en_el_catalogo_de_rechazos` (`crates/nucleo/tests/catalogo.rs`) falla si un rechazo del núcleo no está aquí, si aquí sobra alguno o si cambió lo que dice; `BRONZEAGE_REGENERAR=1 cargo test -p nucleo --test catalogo` lo reescribe. Todo rechazo del núcleo es una de estas constantes: un literal `"familia.nombre"` suelto en el código lo detecta la prueba del catálogo de eventos de la misma fuente.
- **Detalle.** Algunos llevan además `RechazoOrden.detalle` (`google.protobuf.Struct`) con lo que el cliente necesita para actuar: hoy solo `guerra.confirmar` (`{ objetivo: "f-N", facciones: [...] }`, contrato 05 §6).
- **Qué no está aquí.** Los errores de la sesión, que no son rechazos de una orden sino del protocolo (`ErrorSesion`: `hola.repetido`, `partida_no_disponible`, `acceso.*`...), y los códigos de `Aviso` (`reloj.cambiado`, `vista.invalida`, `vista.sin_derecho`): están en el contrato 05 (§2, §4 y §11). Tampoco los hechos de dominio, que tienen su propio catálogo en cada documento de `Docs/Mechanics` (bloques `<!-- eventos -->`).
- **Duplicadas.** Una orden repetida con la misma `peticion_id` no es un rechazo: vuelve como `ORDEN_DUPLICADA` con la versión de entonces (contrato 05 §6).

<!-- rechazos -->
### Generales

| Código | Constante | Qué dice |
|---|---|---|
| `no_autorizado` | `NO_AUTORIZADO` | — |
| `version.obsoleta` | `VERSION_OBSOLETA` | — |
| `ejercito.desconocido` | `EJERCITO_DESCONOCIDO` | — |
| `heroe.desconocido` | `HEROE_DESCONOCIDO` | — |
| `ruta.imposible` | `RUTA_IMPOSIBLE` | — |
| `orden.no_soportada` | `ORDEN_NO_SOPORTADA` | Una orden vacía (sin cuerpo). |
| `orden.no_disponible` | `ORDEN_NO_DISPONIBLE` | Una orden del contrato que el núcleo aún no tiene (política, tecnología, Aedas y refundación): se rechaza con este código estable en vez de descartarse. |
| `orden.forma_invalida` | `ORDEN_FORMA_INVALIDA` | Un valor de la orden fuera de su catálogo cerrado, o un id que no tiene la forma de ninguna entidad (`orden.forma_invalida`). |
| `obra_de_mapa.desconocida` | `OBRA_DE_MAPA_DESCONOCIDA` | `obra-N` no es ningún pueblo ni ninguna avanzada. |
| `repostar.sin_lugar` | `REPOSTAR_SIN_LUGAR` | `repostar` sin `lugar_id` y sin ninguna plaza ni avanzada operativa a la puerta de la columna. |
| `objetivo.no_soportado` | `OBJETIVO_NO_SOPORTADO` | — |
| `reloj.invalido` | `RELOJ_INVALIDO` | — |
| `mundo.no_cargado` | `SIN_MUNDO` | — |
| `asentamiento.desconocido` | `ASENTAMIENTO_DESCONOCIDO` | — |
| `fundacion.sin_caravana` | `SIN_CARAVANA` | — |
| `fundacion.sin_faccion` | `FUNDACION_SIN_FACCION` | — |
| `fundacion.sin_fundadores` | `FUNDACION_SIN_FUNDADORES` | Ningún héroe de la columna es ciudadano de la Facción de la caravana: no hay fundadores (`minFundadoresParaFaccionNueva`, 1). |
| `fundacion.cap_alcanzado` | `CAP_ALCANZADO` | — |
| `nivel.invalido` | `NIVEL_INVALIDO` | — |
| `parcela.sin_libres` | `PARCELA_SIN_LIBRES` | — |
| `parcela.cupo_agotado` | `PARCELA_CUPO_AGOTADO` | — |
| `parcela.no_es_tuya` | `PARCELA_AJENA` | — |

### Obras (`ErrorObra` del crate `asentamiento`)

| Código | Constante | Qué dice |
|---|---|---|
| `obra.tecnologia_faltante` | `OBRA_TECNOLOGIA_FALTANTE` | — |
| `obra.era_insuficiente` | `OBRA_ERA_INSUFICIENTE` | — |
| `obra.tipo_no_manual` | `OBRA_TIPO_NO_MANUAL` | — |
| `obra.unico` | `OBRA_UNICO` | — |
| `obra.tope_almacenes` | `OBRA_TOPE_ALMACENES` | — |
| `obra.tope_viviendas` | `OBRA_TOPE_VIVIENDAS` | — |
| `obra.nivel_insuficiente` | `OBRA_NIVEL_INSUFICIENTE` | — |
| `obra.solo_por_pueblo` | `OBRA_SOLO_POR_PUEBLO` | — |
| `obra.tope_transformacion` | `OBRA_TOPE_TRANSFORMACION` | — |
| `obra.gran_fundicion` | `OBRA_GRAN_FUNDICION` | — |
| `obra.cola_llena` | `OBRA_COLA_LLENA` | — |
| `obra.sin_sitio` | `OBRA_SIN_SITIO` | — |
| `obra.recursos_insuficientes` | `OBRA_RECURSOS_INSUFICIENTES` | — |
| `edificio.desconocido` | `EDIFICIO_DESCONOCIDO` | — |
| `obra.no_esta_en_cola` | `OBRA_NO_ESTA_EN_COLA` | — |
| `obra.no_mejorable` | `OBRA_NO_MEJORABLE` | — |
| `obra.sin_cuadrilla` | `OBRA_SIN_CUADRILLA` | — |

### Ascenso (`Bloqueo`): el rechazo lleva el código del primero y todos en el mensaje

| Código | Constante | Qué dice |
|---|---|---|
| `ascenso.nivel_maximo` | `ASCENSO_NIVEL_MAXIMO` | — |
| `ascenso.en_curso` | `ASCENSO_EN_CURSO` | — |
| `ascenso.falta_poblacion` | `ASCENSO_FALTA_POBLACION` | — |
| `ascenso.faltan_edificios` | `ASCENSO_FALTAN_EDIFICIOS` | — |
| `ascenso.sin_cupo_de_faccion` | `ASCENSO_SIN_CUPO_DE_FACCION` | — |
| `ascenso.recursos_insuficientes` | `ASCENSO_RECURSOS_INSUFICIENTES` | — |
| `ascenso.insolvente` | `ASCENSO_INSOLVENTE` | — |

### Políticas (`ErrorPolitica`)

| Código | Constante | Qué dice |
|---|---|---|
| `politica.desconocida` | `POLITICA_DESCONOCIDA` | — |
| `politica.fuera_de_pool` | `POLITICA_FUERA_DE_POOL` | — |
| `politica.cargo_vacio` | `POLITICA_CARGO_VACIO` | — |
| `politica.ya_activa` | `POLITICA_YA_ACTIVA` | — |
| `politica.sin_slot` | `POLITICA_SIN_SLOT` | — |

### Trueque (`ErrorTrueque`)

| Código | Constante | Qué dice |
|---|---|---|
| `trueque.desconocido` | `TRUEQUE_DESCONOCIDO` | — |
| `trueque.mismo_asentamiento` | `TRUEQUE_MISMO_ASENTAMIENTO` | — |
| `trueque.lado_vacio` | `TRUEQUE_LADO_VACIO` | — |
| `trueque.cantidad_no_positiva` | `TRUEQUE_CANTIDAD_NO_POSITIVA` | — |
| `trueque.recurso_repetido` | `TRUEQUE_RECURSO_REPETIDO` | — |
| `trueque.no_esta_propuesto` | `TRUEQUE_NO_ESTA_PROPUESTO` | — |
| `trueque.propuesta_vencida` | `TRUEQUE_PROPUESTA_VENCIDA` | — |
| `trueque.no_activo` | `TRUEQUE_NO_ACTIVO` | — |

### Mercado de la plaza (`ErrorOrden`) y mostrador (`ErrorMostrador`)

| Código | Constante | Qué dice |
|---|---|---|
| `mercado.sin_mercado` | `MERCADO_SIN_MERCADO` | — |
| `orden.desconocida` | `ORDEN_DESCONOCIDA` | — |
| `orden.no_activa` | `ORDEN_NO_ACTIVA` | — |
| `orden.cantidad_no_positiva` | `ORDEN_CANTIDAD_NO_POSITIVA` | — |
| `orden.sin_stock` | `ORDEN_SIN_STOCK` | — |
| `mostrador.no_es_lider` | `MOSTRADOR_NO_ES_LIDER` | — |
| `mostrador.orden_ajena` | `MOSTRADOR_ORDEN_AJENA` | — |
| `mostrador.orden_caducada` | `MOSTRADOR_ORDEN_CADUCADA` | — |
| `mostrador.fuera_de_la_puerta` | `MOSTRADOR_FUERA_DE_LA_PUERTA` | — |
| `mostrador.cantidad_no_positiva` | `MOSTRADOR_CANTIDAD_NO_POSITIVA` | — |
| `mostrador.precio_no_positivo` | `MOSTRADOR_PRECIO_NO_POSITIVO` | — |
| `mostrador.recurso_oro` | `MOSTRADOR_RECURSO_ORO` | — |
| `mostrador.nada_que_servir` | `MOSTRADOR_NADA_QUE_SERVIR` | — |

### Caravanas (`ErrorCaravana`)

| Código | Constante | Qué dice |
|---|---|---|
| `caravana.desconocida` | `CARAVANA_DESCONOCIDA` | — |
| `caravana.no_es_comercial` | `CARAVANA_NO_ES_COMERCIAL` | — |
| `caravana.no_disponible` | `CARAVANA_NO_DISPONIBLE` | — |
| `caravana.no_preparando` | `CARAVANA_NO_PREPARANDO` | — |
| `caravana.no_aparcada` | `CARAVANA_NO_APARCADA` | — |
| `caravana.estado_invalido` | `CARAVANA_ESTADO_INVALIDO` | — |
| `caravana.sin_mercado` | `CARAVANA_SIN_MERCADO` | — |
| `caravana.en_cooldown` | `CARAVANA_EN_COOLDOWN` | — |
| `caravana.cupo_de_caravanas` | `CARAVANA_CUPO_DE_CARAVANAS` | — |
| `caravana.cupo_de_carros` | `CARAVANA_CUPO_DE_CARROS` | — |
| `caravana.sin_carpinteria` | `CARAVANA_SIN_CARPINTERIA` | — |
| `caravana.sin_recursos` | `CARAVANA_SIN_RECURSOS` | — |
| `caravana.carro_inexistente` | `CARAVANA_CARRO_INEXISTENTE` | — |
| `caravana.carro_ya_tiene_animal` | `CARAVANA_CARRO_YA_TIENE_ANIMAL` | — |
| `caravana.origen_distinto` | `CARAVANA_ORIGEN_DISTINTO` | — |
| `caravana.sin_capacidad` | `CARAVANA_SIN_CAPACIDAD` | — |
| `caravana.carga_invalida` | `CARAVANA_CARGA_INVALIDA` | — |
| `caravana.sin_stock` | `CARAVANA_SIN_STOCK` | — |
| `caravana.cupo_de_escolta` | `CARAVANA_CUPO_DE_ESCOLTA` | — |
| `caravana.escolta_invalida` | `CARAVANA_ESCOLTA_INVALIDA` | — |
| `caravana.sin_ruta` | `CARAVANA_SIN_RUTA` | — |
| `caravana.salida_invalida` | `CARAVANA_SALIDA_INVALIDA` | — |
| `caravana.no_ha_llegado` | `CARAVANA_NO_HA_LLEGADO` | — |
| `caravana.sin_plan` | `CARAVANA_SIN_PLAN` | — |
| `caravana.nada_que_mover` | `CARAVANA_NADA_QUE_MOVER` | — |
| `caravana.fuera_de_la_puerta` | `CARAVANA_FUERA_DE_LA_PUERTA` | Adjuntar o aparcar con la columna lejos de la puerta de la plaza donde está la caravana. |
| `caravana.faccion_ajena` | `CARAVANA_FACCION_AJENA` | Adjuntar a una columna de otra Facción. |
| `caravana.plaza_ajena` | `CARAVANA_PLAZA_AJENA` | — |
| `caravana.no_esta_adjunta` | `CARAVANA_NO_ESTA_ADJUNTA` | — |

### Entrega manual (§4.5)

| Código | Constante | Qué dice |
|---|---|---|
| `entrega.invalida` | `ENTREGA_INVALIDA` | — |
| `entrega.no_debe` | `ENTREGA_NO_DEBE` | — |
| `entrega.fuera_de_la_puerta` | `ENTREGA_FUERA_DE_LA_PUERTA` | — |
| `cantidad.no_positiva` | `CANTIDAD_NO_POSITIVA` | — |

### Pueblos extractores y obras de mapa (`ErrorPueblo` y lo que el núcleo comprueba sobre el mapa)

| Código | Constante | Qué dice |
|---|---|---|
| `pueblo.desconocido` | `PUEBLO_DESCONOCIDO` | — |
| `pueblo.nodo_desconocido` | `PUEBLO_NODO_DESCONOCIDO` | — |
| `pueblo.nodo_no_extraible` | `PUEBLO_NODO_NO_EXTRAIBLE` | El nodo es de ganado (el Corral es de la ciudad) o el tipo no va por pueblo. |
| `pueblo.nodo_ocupado` | `PUEBLO_NODO_OCUPADO` | — |
| `pueblo.fuera_de_la_zona` | `PUEBLO_FUERA_DE_LA_ZONA` | — |
| `pueblo.sin_ruta` | `PUEBLO_SIN_RUTA` | El nodo no tiene ruta por tierra hasta la plaza madre: su carro no podría acarrear. |
| `pueblo.tecnologia_faltante` | `PUEBLO_TECNOLOGIA_FALTANTE` | — |
| `pueblo.nivel_insuficiente` | `PUEBLO_NIVEL_INSUFICIENTE` | — |
| `pueblo.sin_cupo` | `PUEBLO_SIN_CUPO` | — |
| `pueblo.no_esta_pendiente` | `PUEBLO_NO_ESTA_PENDIENTE` | — |
| `pueblo.nada_que_entregar` | `PUEBLO_NADA_QUE_ENTREGAR` | — |
| `pueblo.faccion_ajena` | `PUEBLO_FACCION_AJENA` | — |
| `pueblo.fuera_de_la_puerta` | `PUEBLO_FUERA_DE_LA_PUERTA` | — |
| `pueblo.no_esta_activo` | `PUEBLO_NO_ESTA_ACTIVO` | — |
| `pueblo.ya_tiene_carro` | `PUEBLO_YA_TIENE_CARRO` | — |

### Avanzadas (`ErrorAvanzada` y lo que el núcleo comprueba sobre el mapa, la columna y la guarnición; fase 4F)

| Código | Constante | Qué dice |
|---|---|---|
| `avanzada.desconocida` | `AVANZADA_DESCONOCIDA` | — |
| `avanzada.nivel_insuficiente` | `AVANZADA_NIVEL_INSUFICIENTE` | — |
| `avanzada.sin_cupo` | `AVANZADA_SIN_CUPO` | — |
| `avanzada.fuera_de_la_zona` | `AVANZADA_FUERA_DE_LA_ZONA` | — |
| `avanzada.dentro_del_asentamiento` | `AVANZADA_DENTRO_DEL_ASENTAMIENTO` | El sitio cae sobre la ciudad de la plaza madre (su radio es una décima del de su zona). |
| `avanzada.pisa_otro_elemento` | `AVANZADA_PISA_OTRO_ELEMENTO` | El sitio pisa otra avanzada, un pueblo o un campamento (cada elemento del mapa mide 0,2). |
| `avanzada.nada_que_cancelar` | `AVANZADA_NADA_QUE_CANCELAR` | No hay una mejora ni una reparación que cancelar. |
| `avanzada.fuera_del_sitio` | `AVANZADA_FUERA_DEL_SITIO` | La columna del héroe no está en el sitio donde se pide colocarla. |
| `avanzada.faccion_ajena` | `AVANZADA_FACCION_AJENA` | — |
| `avanzada.fuera_de_la_puerta` | `AVANZADA_FUERA_DE_LA_PUERTA` | — |
| `avanzada.no_esta_pendiente` | `AVANZADA_NO_ESTA_PENDIENTE` | — |
| `avanzada.nada_que_entregar` | `AVANZADA_NADA_QUE_ENTREGAR` | — |
| `avanzada.no_mejorable` | `AVANZADA_NO_MEJORABLE` | — |
| `avanzada.no_reparable` | `AVANZADA_NO_REPARABLE` | — |
| `avanzada.ya_hay_obra` | `AVANZADA_YA_HAY_OBRA` | — |
| `avanzada.ciudadela_no_se_destruye` | `AVANZADA_CIUDADELA_NO_SE_DESTRUYE` | — |
| `avanzada.no_operativa` | `AVANZADA_NO_OPERATIVA` | Solo funciona una avanzada operativa (no una obra ni unas ruinas). |
| `avanzada.sin_almacen` | `AVANZADA_SIN_ALMACEN` | — |
| `avanzada.almacen_lleno` | `AVANZADA_ALMACEN_LLENO` | — |
| `avanzada.nada_que_depositar` | `AVANZADA_NADA_QUE_DEPOSITAR` | — |
| `avanzada.no_resides` | `AVANZADA_NO_RESIDES` | — |
| `avanzada.escuadra_fuera_de_tu_columna` | `AVANZADA_ESCUADRA_FUERA_DE_TU_COLUMNA` | — |
| `avanzada.escuadra_no_esta` | `AVANZADA_ESCUADRA_NO_ESTA` | — |
| `avanzada.tropa_no_elegible` | `AVANZADA_TROPA_NO_ELEGIBLE` | — |
| `avanzada.tropa_sin_tecnologia` | `AVANZADA_TROPA_SIN_TECNOLOGIA` | — |
| `avanzada.tropa_sin_edificio` | `AVANZADA_TROPA_SIN_EDIFICIO` | La plaza madre no tiene el edificio activo, al nivel que pide la tropa, para reclutarla. |

### Murallas (`ErrorRecinto` del crate `asentamiento` y `ErrorMuralla` del trazo), en el orden en que se comprueban

| Código | Constante | Qué dice |
|---|---|---|
| `murallas.sin_cargo` | `MURALLAS_SIN_CARGO` | El cargo que se invoca no es el Gobernador ni el Maestro de Obras. |
| `murallas.nivel_inexistente` | `MURALLAS_NIVEL_INEXISTENTE` | — |
| `murallas.nivel_de_plaza` | `MURALLAS_NIVEL_DE_PLAZA` | La plaza no tiene el nivel operativo que pide el nivel de recinto (empalizada 2, piedra 3, adarve 4). |
| `murallas.en_obra` | `MURALLAS_EN_OBRA` | Hay un recinto sin cerrar (en obra, o dañado por una conquista). |
| `murallas.arrabal` | `MURALLAS_ARRABAL` | Ampliar exige 6 edificios fuera del recinto exterior. |
| `murallas.sin_centro_urbano` | `MURALLAS_SIN_CENTRO_URBANO` | — |
| `murallas.no_se_puede_cerrar` | `MURALLAS_NO_SE_PUEDE_CERRAR` | — |
| `murallas.fuera_de_la_ventana` | `MURALLAS_FUERA_DE_LA_VENTANA` | — |
| `murallas.no_envuelve` | `MURALLAS_NO_ENVUELVE` | El anillo nuevo no envuelve estrictamente al anterior. |
| `murallas.recinto_desconocido` | `MURALLAS_RECINTO_DESCONOCIDO` | — |
| `murallas.incompleto` | `MURALLAS_INCOMPLETO` | Mejorar un recinto sin cerrar. |
| `murallas.ya_mejorando` | `MURALLAS_YA_MEJORANDO` | — |
| `murallas.en_el_maximo` | `MURALLAS_EN_EL_MAXIMO` | — |
| `murallas.ya_cerrado` | `MURALLAS_YA_CERRADO` | Abandonar un recinto que alguna vez estuvo completo. |

### Mercado de los campamentos de mercenarios (`ErrorMercenarios`)

| Código | Constante | Qué dice |
|---|---|---|
| `campamento.desconocido` | `CAMPAMENTO_DESCONOCIDO` | — |
| `campamento.fuera_de_la_puerta` | `CAMPAMENTO_FUERA_DE_LA_PUERTA` | — |
| `mercenarios.cantidad_invalida` | `MERCENARIOS_CANTIDAD_INVALIDA` | — |
| `mercenarios.sin_precio` | `MERCENARIOS_SIN_PRECIO` | — |
| `mercenarios.trigo_no_va_al_carro` | `MERCENARIOS_TRIGO_NO_VA_AL_CARRO` | — |
| `mercenarios.sin_stock` | `MERCENARIOS_SIN_STOCK` | — |
| `mercenarios.cupo_agotado` | `MERCENARIOS_CUPO_AGOTADO` | — |
| `mercenarios.sin_oro` | `MERCENARIOS_SIN_ORO` | — |
| `mercenarios.sin_sitio` | `MERCENARIOS_SIN_SITIO` | — |
| `mercenarios.viveres_llenos` | `MERCENARIOS_VIVERES_LLENOS` | — |

### Reclutar en un campamento (`ErrorMercenarios`) y la tropa prestada

| Código | Constante | Qué dice |
|---|---|---|
| `mercenarios.no_resides` | `MERCENARIOS_NO_RESIDES` | — |
| `mercenarios.tropa_no_disponible` | `MERCENARIOS_TROPA_NO_DISPONIBLE` | — |
| `mercenarios.ya_la_tienes_fuera` | `MERCENARIOS_YA_LA_TIENES_FUERA` | — |
| `mercenarios.al_tope` | `MERCENARIOS_AL_TOPE` | — |
| `mercenarios.sin_reclutas` | `MERCENARIOS_SIN_RECLUTAS` | — |
| `mercenarios.prestamo_sin_tropas` | `MERCENARIOS_PRESTAMO_SIN_TROPAS` | — |
| `mercenarios.no_se_presta` | `MERCENARIOS_NO_SE_PRESTA` | — |
| `mercenarios.prestamo_repetida` | `MERCENARIOS_PRESTAMO_REPETIDA` | — |
| `mercenarios.ya_prestada` | `MERCENARIOS_YA_PRESTADA` | — |
| `mercenarios.nada_que_reponer` | `MERCENARIOS_NADA_QUE_REPONER` | — |

### Reclutar en una plaza (`ErrorReclutamiento`), en el orden en que se comprueban

| Código | Constante | Qué dice |
|---|---|---|
| `reclutamiento.no_puedes` | `RECLUTAMIENTO_NO_PUEDES` | — |
| `reclutamiento.sin_tecnologia` | `RECLUTAMIENTO_SIN_TECNOLOGIA` | — |
| `reclutamiento.reservada` | `RECLUTAMIENTO_RESERVADA` | — |
| `reclutamiento.ya_la_tienes_fuera` | `RECLUTAMIENTO_YA_LA_TIENES_FUERA` | — |
| `reclutamiento.solo_reponer` | `RECLUTAMIENTO_SOLO_REPONER` | — |
| `reclutamiento.al_tope` | `RECLUTAMIENTO_AL_TOPE` | — |
| `reclutamiento.sin_poblacion` | `RECLUTAMIENTO_SIN_POBLACION` | — |
| `reclutamiento.sin_edificio` | `RECLUTAMIENTO_SIN_EDIFICIO` | — |
| `reclutamiento.sin_recursos` | `RECLUTAMIENTO_SIN_RECURSOS` | — |
| `reclutamiento.sin_reserva_de_trigo` | `RECLUTAMIENTO_SIN_RESERVA_DE_TRIGO` | — |

### El héroe, sus puntos, loadouts, escuadras y guarnición (`ErrorHeroe`)

| Código | Constante | Qué dice |
|---|---|---|
| `heroe.ya_existe` | `HEROE_YA_EXISTE` | — |
| `heroe.campamento_desconocido` | `HEROE_CAMPAMENTO_DESCONOCIDO` | — |
| `heroe.puntos_negativos` | `HEROE_PUNTOS_NEGATIVOS` | — |
| `heroe.ningun_punto` | `HEROE_NINGUN_PUNTO` | — |
| `heroe.puntos_insuficientes` | `HEROE_PUNTOS_INSUFICIENTES` | — |
| `heroe.tope_de_atributo` | `HEROE_TOPE_DE_ATRIBUTO` | — |
| `loadout.sin_nombre` | `LOADOUT_SIN_NOMBRE` | — |
| `loadout.desconocido` | `LOADOUT_DESCONOCIDO` | — |
| `loadout.perk_no_desbloqueado` | `LOADOUT_PERK_NO_DESBLOQUEADO` | — |
| `escuadra.repetida` | `ESCUADRA_REPETIDA` | — |
| `escuadra.ajena` | `ESCUADRA_AJENA` | — |
| `escuadra.en_batalla` | `ESCUADRA_EN_BATALLA` | — |
| `liderazgo.superado` | `LIDERAZGO_SUPERADO` | — |
| `guarnicion.sin_residencia` | `GUARNICION_SIN_RESIDENCIA` | — |
| `guarnicion.no_en_campamento` | `GUARNICION_NO_EN_CAMPAMENTO` | — |
| `guarnicion.ya_asignada` | `GUARNICION_YA_ASIGNADA` | — |
| `guarnicion.supera_el_cupo` | `GUARNICION_SUPERA_EL_CUPO` | — |
| `guarnicion.no_asignada` | `GUARNICION_NO_ASIGNADA` | — |

### Presencia: dónde está el héroe y si está en el mundo

| Código | Constante | Qué dice |
|---|---|---|
| `heroe.fuera_del_mundo` | `HEROE_FUERA_DEL_MUNDO` | — |
| `heroe.desconectandose` | `HEROE_DESCONECTANDOSE` | — |
| `heroe.sin_columna` | `HEROE_SIN_COLUMNA` | — |
| `heroe.en_columna` | `HEROE_EN_COLUMNA` | — |
| `heroe.no_esta_en_la_plaza` | `HEROE_NO_ESTA_EN_LA_PLAZA` | — |
| `heroe.sin_residencia` | `HEROE_SIN_RESIDENCIA` | — |
| `heroe.ubicacion_invalida` | `HEROE_UBICACION_INVALIDA` | — |

### Tecnologías (andamiaje de administración)

| Código | Constante | Qué dice |
|---|---|---|
| `tecnologia.desconocida` | `TECNOLOGIA_DESCONOCIDA` | — |

### Columnas (`ErrorColumna`) y convocatorias (`ErrorConvocatoria`)

| Código | Constante | Qué dice |
|---|---|---|
| `columna.ya_vas_en_ella` | `COLUMNA_YA_VAS_EN_ELLA` | — |
| `columna.no_admite` | `COLUMNA_NO_ADMITE` | — |
| `columna.no_es_ejercito` | `COLUMNA_NO_ES_EJERCITO` | — |
| `columna.solo_una_personal` | `COLUMNA_SOLO_UNA_PERSONAL` | — |
| `columna.otra_faccion` | `COLUMNA_OTRA_FACCION` | — |
| `columna.lejos` | `COLUMNA_LEJOS` | — |
| `columna.no_pregunta` | `COLUMNA_NO_PREGUNTA` | — |
| `columna.no_va_dentro` | `COLUMNA_NO_VA_DENTRO` | — |
| `columna.de_una_personal_no_te_separas` | `COLUMNA_DE_UNA_PERSONAL_NO_TE_SEPARAS` | — |
| `columna.el_lider_no_puede` | `COLUMNA_EL_LIDER_NO_PUEDE` | — |
| `columna.es_el_ultimo` | `COLUMNA_ES_EL_ULTIMO` | — |
| `columna.solo_el_lider` | `COLUMNA_SOLO_EL_LIDER` | — |
| `columna.ya_eres_el_lider` | `COLUMNA_YA_ERES_EL_LIDER` | — |
| `columna.sucesor_fuera` | `COLUMNA_SUCESOR_FUERA` | — |
| `columna.no_hay_peticion` | `COLUMNA_NO_HAY_PETICION` | — |
| `columna.peticion_caducada` | `COLUMNA_PETICION_CADUCADA` | — |
| `columna.ya_es_ejercito` | `COLUMNA_YA_ES_EJERCITO` | — |
| `columna.ya_organizando` | `COLUMNA_YA_ORGANIZANDO` | — |
| `columna.no_esta_solo` | `COLUMNA_NO_ESTA_SOLO` | — |
| `columna.politica_invalida` | `COLUMNA_POLITICA_INVALIDA` | — |
| `columna.no_es_una_formacion` | `COLUMNA_NO_ES_UNA_FORMACION` | — |
| `convocatoria.desconocida` | `CONVOCATORIA_DESCONOCIDA` | — |
| `convocatoria.fuera_de_un_lugar` | `CONVOCATORIA_FUERA_DE_UN_LUGAR` | — |
| `convocatoria.sin_faccion` | `CONVOCATORIA_SIN_FACCION` | — |
| `convocatoria.ya_en_ella` | `CONVOCATORIA_YA_EN_ELLA` | — |
| `convocatoria.politica_invalida` | `CONVOCATORIA_POLITICA_INVALIDA` | — |
| `convocatoria.otro_lugar` | `CONVOCATORIA_OTRO_LUGAR` | — |
| `convocatoria.otra_faccion` | `CONVOCATORIA_OTRA_FACCION` | — |
| `convocatoria.no_estas` | `CONVOCATORIA_NO_ESTAS` | — |
| `convocatoria.el_lider_cancela` | `CONVOCATORIA_EL_LIDER_CANCELA` | — |
| `convocatoria.solo_el_lider` | `CONVOCATORIA_SOLO_EL_LIDER` | — |
| `convocatoria.no_hay_peticion` | `CONVOCATORIA_NO_HAY_PETICION` | — |
| `convocatoria.peticion_caducada` | `CONVOCATORIA_PETICION_CADUCADA` | — |
| `convocatoria.lider_fuera` | `CONVOCATORIA_LIDER_FUERA` | — |
| `convocatoria.no_puede_salir` | `CONVOCATORIA_NO_PUEDE_SALIR` | — |

### Salir y entrar en un lugar, y repostar

| Código | Constante | Qué dice |
|---|---|---|
| `heroe.ya_en_columna` | `HEROE_YA_EN_COLUMNA` | — |
| `heroe.en_convocatoria` | `HEROE_EN_CONVOCATORIA` | — |
| `heroe.no_esta_en_el_campamento` | `HEROE_NO_ESTA_EN_EL_CAMPAMENTO` | — |
| `heroe.dentro_de_un_lugar` | `HEROE_DENTRO_DE_UN_LUGAR` | — |
| `salida.solo_de_tu_residencia` | `SALIDA_SOLO_DE_TU_RESIDENCIA` | — |
| `salida.desde_tu_residencia` | `SALIDA_DESDE_TU_RESIDENCIA` | — |
| `salida.sin_escuadras` | `SALIDA_SIN_ESCUADRAS` | — |
| `escuadra.aniquilada` | `ESCUADRA_ANIQUILADA` | — |
| `escuadra.fuera_del_campamento` | `ESCUADRA_FUERA_DEL_CAMPAMENTO` | — |
| `escuadra.en_la_guarnicion` | `ESCUADRA_EN_LA_GUARNICION` | — |
| `carga.negativa` | `CARGA_NEGATIVA` | — |
| `carro.no_cabe` | `CARRO_LLENO` | — |
| `almacen.sin_existencias` | `ALMACEN_SIN_EXISTENCIAS` | — |
| `columna.es_un_ejercito` | `COLUMNA_ES_UN_EJERCITO` | — |
| `columna.en_formacion` | `COLUMNA_EN_FORMACION` | — |
| `columna.personal_sin_caravanas` | `COLUMNA_PERSONAL_SIN_CARAVANAS` | — |
| `columna.viveres_llenos` | `COLUMNA_VIVERES_LLENOS` | — |
| `asentamiento.fuera_de_la_puerta` | `ASENTAMIENTO_FUERA_DE_LA_PUERTA` | — |
| `puerta.cerrada` | `PUERTA_CERRADA` | — |
| `heroe.no_esta_en_el_pueblo` | `HEROE_NO_ESTA_EN_EL_PUEBLO` | — |
| `heroe.no_esta_en_la_avanzada` | `HEROE_NO_ESTA_EN_LA_AVANZADA` | — |
| `carro.vaciar_solo_en_tu_residencia` | `VACIAR_SOLO_EN_TU_RESIDENCIA` | — |
| `almacen_personal.lleno` | `ALMACEN_PERSONAL_LLENO` | — |
| `almacen_personal.fuera_de_tu_residencia` | `ALMACEN_PERSONAL_FUERA_DE_TU_RESIDENCIA` | Guardar o sacar del almacén personal sin la columna a la puerta de la propia residencia, donde vive el almacén (5G-a). |
| `carro.no_lleva` | `CARRO_NO_LLEVA` | Guardar en el almacén personal un recurso que el carro de la columna no lleva (5G-a). |
| `caravana.no_reclamable` | `CARAVANA_NO_RECLAMABLE` | Reclamar una caravana que no es la Caravana de Fundación que vuelve sola a su origen (5G-a): solo esa se reclama en mundo abierto. |
| `caravana.soltar_solo_dentro` | `CARAVANA_SOLTAR_SOLO_DENTRO` | Soltar o aparcar una caravana con la columna fuera de una plaza (a la puerta o en mundo abierto): solo se deja dentro, donde la columna entró con todo (5G-a, decisión 5G-a-5). |
| `repostar.plaza_cerrada` | `REPOSTAR_PLAZA_CERRADA` | — |
| `repostar.sin_existencias` | `REPOSTAR_SIN_EXISTENCIAS` | — |

### Persecución (`ErrorPersecucion`), inspección (`ErrorInspeccion`) y batalla

| Código | Constante | Qué dice |
|---|---|---|
| `persecucion.sin_heroe_sano` | `PERSECUCION_SIN_HEROE_SANO` | — |
| `persecucion.presa_sin_heroe_sano` | `PERSECUCION_PRESA_SIN_HEROE_SANO` | — |
| `persecucion.clase_distinta` | `PERSECUCION_CLASE_DISTINTA` | — |
| `persecucion.propia_o_aliada` | `PERSECUCION_PROPIA_O_ALIADA` | — |
| `persecucion.caravana_adjunta` | `PERSECUCION_CARAVANA_ADJUNTA` | — |
| `persecucion.ejercito_sin_caravanas` | `PERSECUCION_EJERCITO_SIN_CARAVANAS` | — |
| `persecucion.a_si_misma` | `PERSECUCION_A_SI_MISMA` | — |
| `persecucion.a_cubierto` | `PERSECUCION_A_CUBIERTO` | — |
| `persecucion.fuera_de_vista` | `PERSECUCION_FUERA_DE_VISTA` | — |
| `inspeccion.la_tuya` | `INSPECCION_LA_TUYA` | — |
| `inspeccion.plaza_propia` | `INSPECCION_PLAZA_PROPIA` | — |
| `inspeccion.lejos` | `INSPECCION_LEJOS` | — |
| `inspeccion.a_cubierto` | `INSPECCION_A_CUBIERTO` | — |

### La intel de las tabernas (4H T3d; Niebla_De_Guerra §6)

| Código | Constante | Qué dice |
|---|---|---|
| `intel.sin_faccion` | `INTEL_SIN_FACCION` | Quien compra intel tiene que ser ciudadano de una Facción (es para ella). |
| `intel.plaza_ajena` | `INTEL_PLAZA_AJENA` | La taberna es de una plaza de otra Facción. |
| `intel.no_presente` | `INTEL_NO_PRESENTE` | Hay que estar dentro del lugar de la taberna. |
| `intel.sin_taberna` | `INTEL_SIN_TABERNA` | La plaza no tiene una Taberna levantada. |
| `intel.fuera_del_mapa` | `INTEL_FUERA_DEL_MAPA` | — |
| `intel.sin_cupo` | `INTEL_SIN_CUPO` | La taberna no admite Miradas (cupo 0). |
| `intel.cupo_lleno` | `INTEL_CUPO_LLENO` | La taberna ya tiene abiertas todas las Miradas que admite. |
| `intel.zona_reciente` | `INTEL_ZONA_RECIENTE` | La Facción miró esa zona hace poco: hay que esperar a que se enfríe. |
| `intel.plaza_propia` | `INTEL_PLAZA_PROPIA` | — |
| `intel.plaza_desconocida` | `INTEL_PLAZA_DESCONOCIDA` | La Facción no conoce esa plaza: ni tiene su ficha ni cae en una Mirada abierta. |
| `intel.informe_reciente` | `INTEL_INFORME_RECIENTE` | Ya tiene un Informe reciente de esa plaza. |
| `intel.sin_oro` | `INTEL_SIN_ORO` | No hay oro para pagarla (en el almacén de la plaza, en el personal o en el carro). |
| `columna.en_batalla` | `COLUMNA_EN_BATALLA` | La columna está apartada por una batalla: no se mueve ni se le dan órdenes de movimiento, de persecución ni de inspección. |
| `relacion.consigo_misma` | `RELACION_CONSIGO_MISMA` | — |

### Combate con números (fase 4D)

| Código | Constante | Qué dice |
|---|---|---|
| `heroe.herido` | `HEROE_HERIDO` | — |
| `combate.proteccion_de_campamento` | `COMBATE_PROTECCION_DE_CAMPAMENTO` | — |
| `combate.lejos` | `COMBATE_LEJOS` | — |
| `combate.propia` | `COMBATE_PROPIA` | — |
| `combate.aliada` | `COMBATE_ALIADA` | Retirado en 5D (decisión Q2 del autor, 2026-10-24): atacar a un aliado con `declara_guerra` es una traición que rompe la alianza y declara la guerra; sin ella, `guerra.confirmar`. |
| `combate.vasallaje` | `COMBATE_VASALLAJE` | Atacar a la propia señora o a la propia vasalla (5D): un vasallaje no se convierte en guerra; la vasalla se rebela (`rebelion_vasallo`) y la señora la libera (`romper_relacion`). |
| `guerra.confirmar` | `GUERRA_CONFIRMAR` | `atacar` contra una Facción con la que no se está en guerra y sin `declara_guerra`: el cliente tiene que confirmarla (`Detalle::Guerra` dice contra quién). |
| `combate.a_cubierto` | `COMBATE_A_CUBIERTO` | — |
| `combate.clase_distinta` | `COMBATE_CLASE_DISTINTA` | — |
| `combate.sin_heroe_sano` | `COMBATE_SIN_HEROE_SANO` | — |
| `escuadra.cantidad_invalida` | `ESCUADRA_CANTIDAD_INVALIDA` | — |
| `combate.sin_faccion` | `COMBATE_SIN_FACCION` | — |
| `combate.sin_atacantes` | `COMBATE_SIN_ATACANTES` | — |
| `combate.solo_un_ejercito` | `COMBATE_SOLO_UN_EJERCITO` | — |
| `combate.caravana_adjunta` | `COMBATE_CARAVANA_ADJUNTA` | — |
| `combate.plaza_protegida` | `COMBATE_PLAZA_PROTEGIDA` | — |
| `combate.plaza_propia` | `COMBATE_PLAZA_PROPIA` | — |
| `combate.pueblo_inactivo` | `COMBATE_PUEBLO_INACTIVO` | — |
| `combate.avanzada_no_existe` | `COMBATE_AVANZADA_NO_EXISTE` | — |
| `combate.avanzada_en_obra` | `COMBATE_AVANZADA_EN_OBRA` | La avanzada aún es una obra pendiente de materiales o en marcha: se ve en el mapa, pero no se puede atacar (Doc 4.2.2). |
| `combate.avanzada_invulnerable` | `COMBATE_AVANZADA_INVULNERABLE` | Un fuerte en ruinas es invulnerable un día (Doc 5.17). |
| `combate.avanzada_sin_dueno` | `COMBATE_AVANZADA_SIN_DUENO` | Unas ruinas de ciudadela sin dueño: no hay Facción con la que estar en guerra ni nada que defender. |
| `bandidos.desconocido` | `CAMPAMENTO_DE_BANDIDOS_DESCONOCIDO` | — |
| `bandidos.nivel_invalido` | `BANDIDOS_NIVEL_INVALIDO` | — |
| `bandidos.sobre_una_zona` | `BANDIDOS_SOBRE_UNA_ZONA` | — |

### Batallas de Unity (fase 4E; Batalla_Unity_Ciclo §5-§9). El contrato responde `404` a `batalla.no_existe` y `409` al resto

| Código | Constante | Qué dice |
|---|---|---|
| `batalla.bloqueado` | `BATALLA_BLOQUEADO` | Lo que toca la orden está en una batalla activa (§5): ni el héroe, ni su columna, ni la plaza asediada, ni el objetivo reciben órdenes. |
| `batalla.no_existe` | `BATALLA_NO_EXISTE` | No existe, está cerrada o su plazo venció (404). |
| `batalla.invalida` | `BATALLA_INVALIDA` | No puede unirse: está herido, ya combate, su columna ya combate, hay que estar a ≤ `radioEncuentro` del punto o el bando está lleno. |
| `batalla.sin_columna` | `BATALLA_SIN_COLUMNA` | — |
| `batalla.lejos` | `BATALLA_LEJOS` | — |
| `batalla.lado_lleno` | `BATALLA_LADO_LLENO` | — |
| `batalla.ya_combate` | `BATALLA_YA_COMBATE` | — |
| `batalla.ya_salio` | `BATALLA_YA_SALIO` | — |
| `batalla.nadie_de_fuera` | `BATALLA_NADIE_DE_FUERA` | — |
| `batalla.solo_personales` | `BATALLA_SOLO_PERSONALES` | — |
| `batalla.lado_obligatorio` | `BATALLA_LADO_OBLIGATORIO` | — |
| `batalla.nadie_ayuda_a_bandidos` | `BATALLA_NADIE_AYUDA_A_BANDIDOS` | — |
| `batalla.sin_faccion` | `BATALLA_SIN_FACCION` | — |
| `batalla.no_combate_aqui` | `BATALLA_NO_COMBATE_AQUI` | — |
| `batalla.lado_distinto` | `BATALLA_LADO_DISTINTO` | — |
| `batalla.no_puede_cancelar` | `BATALLA_NO_PUEDE_CANCELAR` | — |
| `batalla.ya_asignada` | `BATALLA_YA_ASIGNADA` | Otro servidor la tiene: gana la primera asignación. |
| `batalla.en_convocatoria` | `BATALLA_EN_CONVOCATORIA` | — |
| `batalla.revision_vieja` | `BATALLA_REVISION_VIEJA` | — |
| `batalla.estado_invalido` | `BATALLA_ESTADO_INVALIDO` | — |
| `batalla.servidor_ajeno` | `BATALLA_SERVIDOR_AJENO` | No es el servidor de la asignación activa. |
| `batalla.sin_credencial` | `BATALLA_SIN_CREDENCIAL` | El mensaje no viene con la credencial de un servidor de batalla (`batalla-servidor:<id>`). |
| `batalla.token_invalido` | `BATALLA_TOKEN_INVALIDO` | — |
| `batalla.no_combate` | `BATALLA_NO_COMBATE` | — |
| `batalla.salida_invalida` | `BATALLA_SALIDA_INVALIDA` | — |
| `batalla.salida_incompleta` | `BATALLA_SALIDA_INCOMPLETA` | La salida de una partida en curso omite alguna de las escuadras autorizadas del héroe (Batalla_Unity_Ciclo §8.4). |
| `batalla.resultado_invalido` | `BATALLA_RESULTADO_INVALIDO` | El checklist de §8.5 rechaza el resultado entero (el mensaje dice cuál de sus puntos). |
| `batalla.otro_resultado` | `BATALLA_OTRO_RESULTADO` | «Ya se aplicó con otro resultado» (409). |

### La Facción, la ciudadanía y la residencia (sistema 5, tanda 5B; `ErrorFaccion` y `ErrorResidencia` del crate `politica`)

| Código | Constante | Qué dice |
|---|---|---|
| `faccion.no_existe` | `FACCION_NO_EXISTE` | — |
| `faccion.nombre_vacio` | `FACCION_NOMBRE_VACIO` | El nombre queda vacío tras recortarlo. |
| `faccion.nombre_duplicado` | `FACCION_NOMBRE_DUPLICADO` | Otra Facción ya se llama así (sin mirar mayúsculas). |
| `faccion.sigilo_invalido` | `FACCION_SIGILO_INVALIDO` | Alguna pieza del sigilo no es del catálogo, o los dos colores del fondo son el mismo. |
| `faccion.sigilo_duplicado` | `FACCION_SIGILO_DUPLICADO` | Otra Facción lleva un sigilo idéntico. |
| `faccion.ya_pertenece` | `FACCION_YA_PERTENECE` | Ya es ciudadano de una Facción: 1 héroe, 1 Facción. |
| `faccion.cooldown_creacion` | `FACCION_COOLDOWN_CREACION` | Dejó una Facción hace menos de `CIUDADANIA.cooldownCreacionFaccionDias`. |
| `faccion.no_ha_pedido` | `FACCION_NO_HA_PEDIDO` | Responder a quien no ha pedido entrar. |
| `faccion.no_pertenece` | `FACCION_NO_PERTENECE` | No es ciudadano de ninguna Facción. |
| `faccion.en_ejercito` | `FACCION_EN_EJERCITO` | Dejar la Facción yendo en un ejército (aunque sea solo) o en uno en preparación (Q7): primero se separa. |
| `faccion.no_ciudadano` | `FACCION_NO_CIUDADANO` | El designado para Rey o Embajador no es ciudadano de la Facción. |
| `faccion.sin_rey` | `FACCION_SIN_REY` | Nombrar Embajador en una Facción sin Rey. |
| `residencia.cooldown` | `RESIDENCIA_COOLDOWN` | Cambió de residencia hace menos de `CIUDADANIA.cooldownCambioResidenciaDias`. |
| `residencia.plaza_ajena` | `RESIDENCIA_PLAZA_AJENA` | Solo se reside en una plaza de la propia Facción. |
| `residencia.ya_reside` | `RESIDENCIA_YA_RESIDE` | — |
| `residencia.vetado` | `RESIDENCIA_VETADO` | La plaza lo ha vetado (los vetos llegan en 5E). |
| `residencia.sin_plaza` | `RESIDENCIA_SIN_PLAZA` | Dejar la residencia sin residir en ninguna plaza (en un campamento de mercenarios se cambia de campamento o se muda a una plaza). |

### La capital de la Facción (sistema 5, tanda 5C; `ErrorCapital` del crate `politica`)

| Código | Constante | Qué dice |
|---|---|---|
| `capital.plaza_ajena` | `CAPITAL_PLAZA_AJENA` | Designar capital una plaza que no es de la Facción. |
| `capital.sin_palacio` | `CAPITAL_SIN_PALACIO` | La plaza no tiene un Palacio activo. |
| `capital.ya_es_la_capital` | `CAPITAL_YA_ES_LA_CAPITAL` | La plaza ya es la capital (efectiva) de la Facción. |
| `capital.cooldown` | `CAPITAL_COOLDOWN` | La capital designada sigue en pie y no han pasado `CAPITAL.cooldownDias` desde que se designó. |

### El nombre de la plaza (correcciones del autor a 5C, 5C-6)

| Código | Constante | Qué dice |
|---|---|---|
| `asentamiento.nombre_largo` | `ASENTAMIENTO_NOMBRE_LARGO` | El nombre recortado pasa de `NOMBRE_ASENTAMIENTO.largoMaximo` caracteres. |
| `asentamiento.nombre_duplicado` | `ASENTAMIENTO_NOMBRE_DUPLICADO` | Otra plaza viva ya se llama así (sin mirar tildes ni mayúsculas). |

### Las relaciones entre Facciones (sistema 5, tanda 5D; `ErrorDiplomacia` y `ErrorPropuesta` del crate `politica`)

| Código | Constante | Qué dice |
|---|---|---|
| `diplomacia.ya_hay_relacion` | `DIPLOMACIA_YA_HAY_RELACION` | Ya hay una relación activa entre las dos (de cualquier clase, la guerra incluida): para cambiarla hay que romperla antes (o firmar la paz). |
| `diplomacia.relacion_desconocida` | `DIPLOMACIA_RELACION_DESCONOCIDA` | `rel-N` no es ninguna relación. |
| `diplomacia.relacion_rota` | `DIPLOMACIA_RELACION_ROTA` | La relación ya está rota (o la paz ya se firmó). |
| `diplomacia.no_es_guerra` | `DIPLOMACIA_NO_ES_GUERRA` | La paz es de una guerra. |
| `diplomacia.no_es_vasallaje` | `DIPLOMACIA_NO_ES_VASALLAJE` | La rebelión es de un vasallaje. |
| `diplomacia.no_es_parte` | `DIPLOMACIA_NO_ES_PARTE` | Esa Facción no es parte de esa relación. |
| `diplomacia.ya_ofrecio_paz` | `DIPLOMACIA_YA_OFRECIO_PAZ` | Esa Facción ya ofreció la paz: falta la otra. |
| `diplomacia.guerra_no_se_rompe` | `DIPLOMACIA_GUERRA_NO_SE_ROMPE` | Una guerra no se rompe: se acaba con la paz. |
| `diplomacia.solo_la_senora` | `DIPLOMACIA_SOLO_LA_SENORA` | Un vasallaje solo lo libera la señora; la vasalla se rebela. |
| `diplomacia.sin_rey` | `DIPLOMACIA_SIN_REY` | Una de las dos Facciones no tiene Rey que pacte o conteste. |
| `diplomacia.vasalla_de_tercera` | `DIPLOMACIA_VASALLA_DE_TERCERA` | Una de las dos ya es vasalla de una tercera (una Facción tiene como mucho una señora). |
| `diplomacia.reputacion_baja` | `DIPLOMACIA_REPUTACION_BAJA` | Proponer una alianza con la reputación por debajo de `REPUTACION.umbralBajoParaEmbajador`. |
| `diplomacia.ya_pendiente` | `DIPLOMACIA_YA_PENDIENTE` | Ya hay una propuesta de relación vigente entre las dos, en cualquier sentido. |
| `diplomacia.propuesta_desconocida` | `DIPLOMACIA_PROPUESTA_DESCONOCIDA` | `prop-N` no es ninguna propuesta de relación pendiente (o ya se contestó o se retiró). |
| `diplomacia.propuesta_caducada` | `DIPLOMACIA_PROPUESTA_CADUCADA` | La propuesta caducó (`RELACION_PROPUESTA.caducidadDias`). |
| `diplomacia.batalla_abierta` | `DIPLOMACIA_BATALLA_ABIERTA` | Aceptar una relación con una batalla abierta en la que combate alguna de las dos: se contesta cuando termine. |
| `diplomacia.tributo_invalido` | `DIPLOMACIA_TRIBUTO_INVALIDO` | El tributo de un vasallaje es una cantidad por minuto de un recurso, no negativa. |

### Refundación y Caravana de Fundación (sistema 5, tanda 5G-b; Fundacion_Y_Refundacion §4, §6 y §9)

| Código | Constante | Qué dice |
|---|---|---|
| `fundacion.no_resides` | `FUNDACION_NO_RESIDES` | Lanzar una Caravana de Fundación sin residir en la plaza o sin estar dentro de ella (Proyecciones §5.2.4). |
| `fundacion.no_ciudadano` | `FUNDACION_NO_CIUDADANO` | El titular no es ciudadano de la Facción de la plaza de origen. |
| `fundacion.nivel_insuficiente` | `FUNDACION_NIVEL_INSUFICIENTE` | La plaza de origen no está a nivel 2 (el operativo, no el alcanzado). |
| `fundacion.no_titular` | `FUNDACION_NO_TITULAR` | Solo su titular desarma o engancha la Caravana de Fundación (y tiene que ir en la columna que la engancha). |
| `refundacion.con_plazas` | `REFUNDACION_CON_PLAZAS` | Refundar desde un campamento es de una Facción sin ninguna plaza. |
| `refundacion.ya_tiene_caravana` | `REFUNDACION_YA_TIENE_CARAVANA` | La Facción ya tiene una Caravana de Fundación. |
| `refundacion.fondo_corto` | `REFUNDACION_FONDO_CORTO` | El fondo de la Facción en el campamento no cubre el coste de la caravana (el mensaje dice cuánto falta). |
| `refundacion.no_resides` | `REFUNDACION_NO_RESIDES` | Aportar o retirar con el almacén personal sin residir en ese campamento: el almacén vive donde se reside (aquí, con el carro). |
| `refundacion.sin_aporte` | `REFUNDACION_SIN_APORTE` | Retirar un recurso que el héroe no aportó al fondo de ese campamento. |
| `caravana.lejos` | `CARAVANA_LEJOS` | Reclamar la Caravana de Fundación lejos de ella (a más de `radioEncuentro`). |

### Cargos locales y puerta (sistema 5, tanda 5E; Diplomacia_Facciones §3.4 y §4.3)

| Código | Constante | Qué dice |
|---|---|---|
| `cargo.no_ciudadano` | `CARGO_NO_CIUDADANO` | El designado para un cargo local no es ciudadano de la Facción dueña de la plaza. |
| `cargo.no_reside` | `CARGO_NO_RESIDE` | El designado para un cargo local no reside en la plaza (Q4: solo se designa a quien reside). |
| `cargo.sin_gobernador` | `CARGO_SIN_GOBERNADOR` | Tesorero, General, Maestro de Obras y Sacerdote se designan cuando la plaza ya tiene Gobernador. |
| `puerta.residente` | `PUERTA_RESIDENTE` | A un residente de la plaza no se le veta (sería el exilio). |
<!-- /rechazos -->
