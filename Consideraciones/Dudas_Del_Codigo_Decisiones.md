# Decisiones del autor sobre las DUDA del código (2026-10-23)

Numeración del repaso (bloque A). **Aplicadas el 2026-10-23** (sin commit), salvo el 7 (Granja inicial que no cabe), pendiente de confirmar. Junto a cada punto, dónde quedó escrito y qué prueba lo cubre. Foto `BAN24`, `LAYOUT_VERSION` 4.

## Economía, construcción y trazado
- 1 radioCiudad: solo cuentan los edificios de la cola (como hoy) y las cifras 30/5/60-180 pasan a un bloque `<!-- parametros -->` en Trazado_Urbano.
  → **Aplicado.** Bloque `RADIO_CIUDAD` en `Trazado_Urbano.md` §12 (entra en la huella de las reglas), que lee `nucleo::urbanismo::Urbanismo::parametros`; la regla de qué edificios cuentan, en §2.1. Sin `DUDA` en `asentamiento.rs` ni `urbanismo.rs`.
- 2 Mejora automática: comprueba el hueco al arrancar, como la manual (nunca retiene cuadrilla). CAMBIO.
  → **Aplicado.** `Asentamiento::iniciar_mejoras_automaticas`; `Economia_Construccion.md` §7. Prueba `la_mejora_automatica_no_arranca_sin_hueco_para_la_huella_nueva` (`asentamiento/tests/ordenes.rs`).
- 3 Política: caduca después del paso de minuto (como hoy).
  → **Aplicado.** `Economia_Construccion.md` §1 (tabla); comentario de `PRIO_CADUCA_POLITICA`; fila de paridad cerrada.
- 4 Obra de ascenso a un nivel ya concedido por el atajo: se descarta (como hoy).
  → **Aplicado.** `Economia_Construccion.md` §11.2; comentario en `nucleo/zonas.rs`.
- 5 Pueblo sin ruta por tierra a su madre: no se puede fundar. CAMBIO (rechazo nuevo).
  → **Aplicado.** `MarcarPueblo` rechaza con `pueblo.sin_ruta` (`rechazos::PUEBLO_SIN_RUTA`); `Economia_Construccion.md` §14 y canon Doc 4.2.2 («Requisitos»). Prueba `un_nodo_sin_ruta_por_tierra_hasta_la_madre_no_admite_pueblo` (`nucleo/tests/pueblos.rs`).
- 6 (ver 1).
- 7 Granja inicial que no cabe: el autor recuerda que el TS la reubicaba al mejorar; el análisis confirma que el Rust ya reubica en cada cambio de huella igual que el TS y que la fundación sin sitio es inalcanzable. Pendiente de confirmar con el autor.
  → **Sin aplicar** (pendiente): las `// DUDA:` de `nucleo/urbanismo.rs` (`Urbe::fundar`) y `trazado/src/fundacion.rs` se quedan.
- 8 Pueblo: 6 casas. La planta interna del pueblo toma el radio que necesite para sus piezas (sin tope de 8 u, que nadie decidió: lo derivó la fase 3C del «0,2 de mapa»). En el mapa del mundo sigue ocupando 0,2. CAMBIO.
  → **Aplicado.** `PUEBLOS_EXTRACTORES.casasDeLaPlanta` = 6 (`Economia_Construccion.md` §14); `urbanismo::planta_de_pueblo` traza desde 8 u, crece si algo no cabe y la vista lleva el disco que contiene las piezas; `Trazado_Urbano.md` (nota de los pueblos) y canon Doc 4.2.2 («Trazado»). Pruebas `la_planta_de_un_pueblo_toma_el_radio_que_necesitan_sus_piezas` y `la_planta_de_un_pueblo_sale_de_su_id...` (`nucleo/src/urbanismo.rs`).
- 9-11 Trazado (ancla en disco, ancla llena, orden de muralla): el autor quiere revisarlas en detalle frente al TS (el balance del TS importa). Análisis hecho; pendiente de decidir.
  → Decididas en «Elección del laboratorio» (abajo).
- 12 Reparación de la muralla tras la conquista: empieza al instante (como hoy).
  → **Aplicado.** `Murallas.md` §9.6; comentario en `nucleo/conquistas.rs`; ítem cerrado en la fila de paridad de la tercera tanda de murallas.

## Combate y bandidos
- 13/16 Bandidos: sorteo como el TypeScript (ángulo y radio, con enteros y tabla de senos); bosque a ≥10 del borde de la zona. CAMBIO.
  → **Aplicado.** `combate::bandidos::punto_del_anillo` (ángulo en grados enteros con `trazado::cos_sen_1000`, después el radio; dos tiradas por intento); el margen de 10 u es la regla (`MARGEN_DE_ZONA`). `Generacion_Del_Mundo.md` §13 (notas de 4A-2 y 4D), `Combate.md` (nota de 4A-2), canon Doc 1.9. Prueba `el_punto_del_anillo_es_uniforme_en_el_radio_como_el_typescript` (`combate/src/bandidos.rs`).
- 14 Un héroe solo puede atacar un campamento, pero debe perder siempre. Ya se cumple: 15 × 1,15 = 17,25 < 20 (nivel 1). Escribirlo como regla en Mechanics («un héroe solo no destruye ningún campamento»).
  → **Aplicado.** `Combate.md` §7.2; prueba `un_heroe_solo_no_destruye_ni_el_campamento_mas_flojo` (`combate/src/bandidos.rs`).
- 15 Reaparición de bandidos: se reprograma al cambiar la demanda (llega o se va un residente del campamento de mercenarios; un bosque válido que queda libre cerca de una plaza). CAMBIO.
  → **Aplicado.** `Nucleo::reprogramar_anillo` (al crear un héroe, al mudarse de residencia y cuando una Facción se queda sin plazas) y `plazas_sin_bosque_buscan` (al irse un campamento con bosque y al caer una plaza en ruinas; la primera que espera, por id, y en cadena). `Generacion_Del_Mundo.md` §13 (nota de 4D), canon Doc 1.9. Pruebas `la_cita_del_anillo_se_rehace_cuando_llega_o_se_va_un_residente` y `una_plaza_sin_bosque_libre_recibe_su_campamento_en_cuanto_se_libera_uno` (`nucleo/tests/combate.rs`).
- 17 Con números se pierden los últimos carros añadidos (como hoy).
  → **Aplicado.** `Combate.md` §7.1; comentario de `aplicar_al_dueno`.
- 18 Aguante: nunca se pierde el último carro; el 30 % sigue en él. Se corrige el canon (Doc 3.10: desaparece «con los carreteros»). CAMBIO. Con eso desaparece el caso del 30 % sin carros (combates.rs:494).
  → **Aplicado.** `combate::caravana::aplicar_al_dueno` (si se perderían todos, se queda el primero; también con Unity); `Perdida.carga_salvada` y la entrega al almacén de origen retiradas de `nucleo/combates.rs`. `Combate.md` §7.1, `Comercio_Caravanas.md` (nota de 4E), canon Doc 3.10 y 3.13.6. Prueba `en_un_aguante_nunca_se_pierde_el_ultimo_carro_y_el_30_por_ciento_sigue_en_el` (`combate/src/caravana.rs`).
- 19 Resultado de Unity: debe nombrar todas las escuadras autorizadas; si omite alguna, se rechaza. CAMBIO.
  → **Aplicado** a la **salida** de un héroe (`registrar_salida`, la `DUDA` que lo pedía; el resultado final ya lo exigía con `EscuadrasIncompletas`): `ErrorBatalla::SalidaIncompleta` → `batalla.salida_incompleta` (`rechazos::BATALLA_SALIDA_INCOMPLETA`). `Batalla_Unity_Ciclo.md` §8.4 y lista de rechazos. Pruebas `una_salida_en_curso_que_omite_una_escuadra_autorizada_se_rechaza_entera` (`combate/src/ciclo/pruebas.rs`) y el caso `s3b` de `con_la_partida_en_curso_la_salida_aplica_las_bajas...` (`nucleo/tests/batallas.rs`).
- 20 Pueblo: ventajas multiplicadas ×1,3125 (como hoy).
  → **Aplicado.** `Combate.md` §7.3; comentario de `combate::pueblo::resolver_saqueo`.
- 21 Cohesión: los 15 del héroe se suman antes y el héroe no cuenta para la cohesión (como hoy).
  → **Aplicado.** `Combate.md` §3.3; comentario de `resolver_combate`.
- 22 Avanzada: el castigo al defensor solo si gana el atacante (como hoy).
  → **Aplicado.** `Combate.md` (nota de la tercera ronda, «Todo defensor que pierde»); comentario en `nucleo/avanzadas.rs`.
- 23-24 Guarnición del pueblo: reposición gradual, 1 minero cada 12 min (30 en 6 h), empezando con la primera baja; una cita por minero. CAMBIO. Chatarra si resiste: al almacén de la madre (como hoy).
  → **Aplicado.** `PUEBLOS_EXTRACTORES.mineros.minutosPorMinero` = 12 (sustituye a `reposicionMinutos`); `combate::pueblo::{reposicion_en, reponer}` y `Nucleo::{guardar_guarnicion, programar_minero, repone_guarnicion}`. `Combate.md` §7.3 y nota de 4D, `Economia_Construccion.md` §14, `Reloj_Y_Eventos.md`, canon Doc 4.2.2. Pruebas `la_reposicion_gratuita_es_un_minero_cada_12_minutos_y_30_en_seis_horas` (`combate/src/pueblo.rs`), `saquear_un_pueblo_..._se_repone_minero_a_minero` (`nucleo/tests/combate.rs`) y la de Unity en `nucleo/tests/batallas.rs`.
- 25 lado_para_unirse no mira si el bando está lleno: se deja (lo rechaza la orden).
  → **Aplicado.** `Batalla_Unity_Ciclo.md` §7.1; comentario de `Nucleo::lado_para_unirse`; ítem cerrado en la fila de T4.

## Comercio y caravanas
- 26 Carga perdida proporcional a la capacidad de los carros (como hoy).
- 27 Cotización: receta más barata (como hoy).
- 28-29 Oro topado por lo que cabe; el que paga solo paga lo que entró (como hoy).
- 30-31 Orden sobre el oro con precio > 0 se rechaza; cupo del campamento por recurso (como hoy).
- 32 Trueque: vencida no se acepta; línea a línea; lo cumplido no reanuncia (como hoy).
  → **26-32 aplicados.** `Comercio_Caravanas.md` §15 («Reglas del Rust decididas por el autor el 2026-10-23»); comentarios en `comercio/{caravana,datos,entrega,mostrador,mercenarios,trueque}.rs`; fila de paridad del reparto de carga cerrada.
- 33 Radio del mercado 2 y se corrige el bloque del documento (10 → 2). CAMBIO DE DOC.
  → **Aplicado.** El bloque `MOVIMIENTO.radioPuerta` ya valía 2 (corregido en la 4A-1); la constante `RADIO_PUERTA` del núcleo se retira y todo lee el bloque (`Nucleo::radio_puerta`).
- 34-35 Reputación por Facción; sin umbral alto (como hoy).
- 36-37 Retirar orden antes de plazo y aviso «llegó a casa»: a Mechanics (como hoy).
  → **34-37 aplicados.** `Comercio_Caravanas.md` §15; comentarios en `nucleo/mercado.rs` y `nucleo/viaje.rs`; ítem de `CancelarOrdenMercado` cerrado en su fila de paridad.
- 38 Al perder el animal más lento, el plan en pausa se reajusta a la velocidad nueva. CAMBIO.
  → **Aplicado.** `Nucleo::reajustar_plan_en_pausa` (desde `aplicar_perdida_de_carros` y `caer_caravana`); `Comercio_Caravanas.md` (nota de 3B-2, «Batalla»). Prueba `si_en_la_batalla_se_pierde_el_animal_mas_lento_el_plan_en_pausa_sigue_a_la_velocidad_nueva` (`nucleo/tests/caravanas.rs`).

## Trazado (análisis frente al TS hecho)
- 7 Granja al mejorar: exacta como el TS (la búsqueda quita la Granja del suelo ocupado pero conserva su camino como frente). CAMBIO.
- 9 Disco urbano: fuera para anclas y satélites, como el TS (quitar el invariante del test y la nota de LAYOUT_VERSION 3). CAMBIO.
- 10 / a Ancla llena: se marca llena en cuanto no cabe la huella pedida, como el TS. CAMBIO.
- b (círculo del núcleo), c (semilla excluida de la separación), C (ancla al proponer) y D (continuaFila con calles): LABORATORIO con ciudades dibujadas para que el autor elija.
- 11 Orden de la muralla: medir puertas y torres con los dos órdenes en 20 ciudades y mantener el Rust.
- Todos los cambios de trazado se aplican juntos después del laboratorio.
  → **Aplicado** (ver abajo).

## Niebla, vista y órdenes
- 39 Grabar lo explorado al cambiar el mundo pasa a ser la regla: corregir Niebla §4.1. Alcance del vigía, como hoy.
  → **Aplicado.** `Niebla_De_Guerra.md` §4.1 reescrito como regla (con el alcance del vigía); comentario en `nucleo/exploracion.rs`; ítems cerrados en la fila de T3d.
- 40 Tramo ajeno recortado al alcance de los ojos, no al rectángulo. CAMBIO.
  → **Aplicado.** `vision::tramo_visible` recorta a la unión de los discos de los ojos (exacto en enteros; una caravana grande que solo llama la atención, a sus 60 u); `proyeccion/vista.rs`. `Niebla_De_Guerra.md` §5.4 y nota de T3d. Pruebas `tramo_visible_se_recorta_al_alcance_de_los_ojos...` y `tramo_visible_vacio_fuera_del_alcance...` (`vision/src/tests.rs`) y `el_tramo_ajeno_se_recorta_al_alcance_de_los_ojos_y_no_al_rectangulo_de_la_vista` (`nucleo/tests/niebla.rs`).
- 41-42 lado_para_unirse estático; convocatoria a toda la Facción (como hoy).
  → **Aplicado.** `Batalla_Unity_Ciclo.md` §7.1 y `Presencia_Y_Convocatoria.md` §9.6; comentarios en `proyeccion/{propios,audiencia}.rs`; ítems cerrados en las filas de T3b, T3d y T4.
- 43-45 crear_ejercito con objetivo se rechaza; unirse sin carga; obra de pueblo rechazada por destruir_pueblo (como hoy).
  → **Aplicado.** `Presencia_Y_Convocatoria.md` §13; comentarios en `sesiones/ordenes.rs`; ítems cerrados en la fila de T2.

## Cuándo
- Los CAMBIO de reglas (no de trazado) se aplican con un subagente cuando termine T6 (T6 toca sesiones y proyección; el 40 toca vista.rs).
- Toda decisión «como hoy» se escribe como regla en Mechanics (quitar la DUDA del código y de la tabla de paridad) y en el canon si lo dice.
  → Hecho el 2026-10-23. Además se corrigieron dos comentarios desfasados: la `VISTA` en `sesiones.rs` (lo que sale del rectángulo o de la vista sale con `QUITA` en el muestreo siguiente) y `te_sigue` en `proyeccion/propios.rs` (lo pone la vista del perseguido).

## Elección del laboratorio (2026-10-23)
- Se aplican la base (sin disco para anclas y satélites, ancla llena por huella, Granja exacta), **b** (círculo del núcleo del ancla, como el TS) y **C** (el ancla nace al proponer y se queda aunque la obra no se comprometa, **dándola de alta en la economía al proponer**, como edificio gratuito). **c y D no.** Muralla con el orden del Rust.
- Código de referencia: worktree `C:\Users\VRINO\Desarrollo\BronzeAgeFase0\.claude\worktrees\agent-aa3ae1ce3d3e53d72`, rama `worktree-agent-aa3ae1ce3d3e53d72` (comentarios `LAB`); hay que aplicarlo sin el `struct Lab`, subir `LAYOUT_VERSION` a 4 y actualizar Trazado_Urbano.md; el contador `ViolacionesCiudad.disco` del batch se limita a los genéricos.
  → **Aplicado**, sin `struct Lab` ni interruptores, `LAYOUT_VERSION` = 4 (foto `BAN24`):
  - Base: `trazado/src/anclas.rs` (`hueco_en_direccion` sin disco; `ancla_activa` marca llena en cuanto no cabe la huella pedida), `colocacion.rs` (`atraccion_dura` sin disco), `ciudad.rs` (`sin_pieza` libera solo el suelo y conserva la red). b: círculo de barrido en `atraccion_dura`. C: `Urbe::proponer` (sustituye a `cabe`), `Sitios::proponer` devuelve las anclas que crea y `Asentamiento::proponer` las da de alta; la orden manual comprueba los recursos antes de preguntar por sitio, para que un rechazo no deje ancla. Muralla: el orden del Rust es la regla (`Murallas.md` §3; en las 20 ciudades de la prueba, con la geometría 4, salen de 0 a 6 saltos por anillo).
  - `Trazado_Urbano.md`: nota de implementación (versión 4, «Cuándo se coloca»), §2.1, §6.5, §6.7, §7.2, §12 (`LAYOUT_VERSION` 4) y §14. El invariante «dentro del disco» queda solo para los genéricos (pruebas de `trazado` y `ViolacionesCiudad.disco` del batch).
  - Pruebas: `las_anclas_y_sus_satelites_no_se_limitan_al_disco_urbano`, `el_nucleo_de_un_ancla_se_barre_con_su_circulo_y_no_con_el_cuadrado`, `un_ancla_queda_llena_en_cuanto_no_cabe_la_huella_que_se_pide` (`trazado/tests/trazado.rs`), `la_granja_que_crece_busca_sitio_sin_su_suelo_pero_con_su_camino` (`trazado/src/colocacion.rs`), `el_gate_de_anclas_..._nace_al_proponer`, `una_obra_de_industria_en_nivel_2_crea_su_ancla_al_proponer...` y `con_las_anclas_naciendo_al_proponer_cada_pieza_tiene_su_edificio_y_cada_edificio_su_pieza` (`nucleo/tests/urbanismo.rs`).

## Dudas de T6 (decididas por el autor el 2026-10-24; prioridad: el rendimiento del servidor, sin clientes todavía pero dejándolo todo preparado)
- T6-1 Vista: tras una orden se reconcilia al momento solo la `VISTA` de quien la dio; las demás, en el siguiente muestreo de 250 ms (con el reloj parado, todas al momento). Lo que se envía sigue siendo foto al conectar y después solo cambios. CAMBIO.
  → **Aplicado.** `sesiones.rs` (`distribuir(causante)`: con el reloj en marcha, tras una orden o un mensaje de batalla solo la vista de esa sesión; en el muestreo, tras un cambio que no viene de una orden o con el reloj parado, todas) y `sesiones/ambitos.rs` (`reconciliar_vistas(.., solo, ..)`, que conserva la niebla de las perspectivas que no mira). `05_Contrato_de_Sesion.md` §3 y `Reloj_Y_Eventos.md` §4. Prueba `tras_una_orden_solo_la_vista_de_quien_la_da_se_pone_al_dia_y_las_demas_en_el_muestreo` (`nucleo/tests/vista.rs`); `tests/carga.rs` mide ahora con el reloj parado y en marcha (×1).
- T6-2 Idempotencia: las últimas **64 órdenes por cuenta** (no 500 globales). Va en la foto, en la misma subida. CAMBIO.
  → **Aplicado.** `estado.rs` (`vistos` por actor, `limite_idempotencia` = 64, serializado por actor en `BAN24`); bloque `INFRA` de `Reloj_Y_Eventos.md` (`limiteIdempotenciaPorCuenta: 64`) y `Proyecciones_Y_Autorizacion.md`. Prueba `la_memoria_de_idempotencia_es_de_cada_cuenta` (`nucleo/tests/nucleo.rs`).
- T6-10 Reloj: pararlo es solo para mantenimiento o último recurso de la administración (y en pruebas); avisar a todas las sesiones al pararlo se queda. Escribirlo como regla en Reloj_Y_Eventos.md.
  → **Aplicado.** `Reloj_Y_Eventos.md` §1 («Parar el reloj»); ítem cerrado en la fila de T6.
- T6-3 a T6-9: se quedan como están, anotadas (sin cliente todavía).

## Dudas de T5, T3d y T3e (decididas por el autor el 2026-10-24)
- T5-1 Un hecho sin plaza que nombra a alguien llega solo a los nombrados y sus Facciones; a todos, solo los públicos (crónica, mercenarios repuestos). Como hoy: escribirlo como regla en 05 §8 y Proyecciones_Y_Autorizacion (corrige «sin plaza, a todos»).
  → **Aplicado.** `05_Contrato_de_Sesion.md` §8 y `Proyecciones_Y_Autorizacion.md` (audiencia de los hechos); comentario en `proyeccion/eventos.rs`; ítem cerrado en la fila de T5.
- T5-2 Los hechos de una plaza los oye su Facción **y quien esté dentro**, sea ciudadano o visitante (aliado o neutral). CAMBIO.
  → **Aplicado.** `sesiones/ambitos.rs` (`Mesa::oyentes`: también la sesión cuyo héroe está dentro de una plaza del hecho, por su `HEROE`); `05` §8 y `Proyecciones_Y_Autorizacion.md`. Pruebas `el_visitante_que_esta_dentro_de_una_plaza_oye_sus_hechos` (`nucleo/tests/eventos.rs`) y la de `propios.rs` ajustada (el visitante oye el hecho de la plaza).
- T5-6 Los ids del payload genérico de los eventos se traducen siempre al formato del contrato (`heroeId: "heroe-3"`), como en los tipados y en las tablas de Mechanics. CAMBIO.
  → **Aplicado.** `proyeccion/eventos.rs` (`id_del_contrato`: cada clave que nombra una entidad viaja como `<clave>Id` con el id del contrato; la batalla como `battleId`); `persecuciones.rs` nombra la presa como `presa` (columna) o `caravana` (caravana), sin la marca aparte; `05` §8 y `Movimiento_Ejercitos.md`. Prueba `el_payload_generico_lleva_los_ids_del_contrato` (`nucleo/tests/eventos.rs`).
- T5-7 Con números, una columna derrotada que lleva caravanas adjuntas: cada adjunta pierde la mitad de sus carros con su carga, como con Unity; se emite `ejercito.caravanas_perdidas`. CAMBIO.
  → **Aplicado.** `nucleo/combates.rs` (`adjuntas_de_la_derrotada`: una disputa por adjunta, la que se queda sin carros desaparece; la carga perdida entra en el botín del ganador; `ejercito.caravanas_perdidas { caravanas, carros, carga }`). `Combate.md` §5, `Comercio_Caravanas.md`, `Movimiento_Ejercitos.md` §9 y el canon (Docs/Game 0, 1, 3 y 5). Prueba `con_numeros_la_columna_derrotada_pierde_la_mitad_de_los_carros_de_cada_caravana_adjunta` (`nucleo/tests/combate.rs`).
- T3e-3 Administración: un solo rol hasta el sistema 5 (como hoy).
  → **Aplicado.** `Proyecciones_Y_Autorizacion.md` §2.2 y `05` (La administración); ítem cerrado en la fila de T3e.
- T3d-4 Memoria de la niebla: se recuerda todo lo que se vio, ruinas sin dueño incluidas, y pueblos y avanzadas con su nivel o etapa y su estado (activo, en obra, en ruinas) tal como estaban. CAMBIO (memoria en la foto: misma subida).
  → **Aplicado.** `memoria.rs` (`EstadoSitio` en `DatosSitio`, en la foto `BAN24`), `nucleo/exploracion.rs` (las ruinas sin dueño son sitios de la Facción 0; el estado entra en lo que «cambia» de un sitio), `proyeccion/vista.rs` (`FichaConocida.estado`, `faccion_id` vacío de nadie; la ruina sin dueño que se ve ya no sale además como ficha). Esquema aditivo: `EstadoSitio` y `FichaConocida.estado = 9`. `Niebla_De_Guerra.md` §4.1, `05` (tabla de `VISTA`) y canon Docs/Game 5. Pruebas `el_pueblo_ajeno_que_se_vio_en_obra_se_recuerda_en_obra_aunque_despues_se_termine` (`nucleo/tests/pueblos.rs`) y la de las ruinas de ciudadela de `nucleo/tests/avanzadas.rs` (la ficha de nadie, etapa 3, en ruinas).
