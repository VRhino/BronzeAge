//! Ficheros dorados (`contratos/golden`): el .bin lo genera `contratos/ts`; aquí se decodifica y se vuelve a codificar igual.
use contrato::v1::*;
use prost::Message;

fn ida_y_vuelta<M: Message + Default>(bin: &[u8]) {
    let m = M::decode(bin).unwrap();
    assert_eq!(m.encode_to_vec(), bin);
}

fn dorado(nombre: &str) -> Vec<u8> {
    std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contratos/golden").join(format!("{nombre}.bin"))).unwrap_or_else(|e| panic!("{nombre}: {e}"))
}

fn de_servidor(nombre: &str) -> mensaje_de_servidor::Cuerpo {
    MensajeDeServidor::decode(dorado(&format!("MensajeDeServidor__{nombre}")).as_slice()).unwrap().cuerpo.expect("sin cuerpo")
}

fn de_cliente(nombre: &str) -> mensaje_de_cliente::Cuerpo {
    MensajeDeCliente::decode(dorado(&format!("MensajeDeCliente__{nombre}")).as_slice()).unwrap().cuerpo.expect("sin cuerpo")
}

#[test]
fn dorados() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contratos/golden");
    let mut n = 0;
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.extension().is_none_or(|x| x != "bin") {
            continue;
        }
        let bin = std::fs::read(&p).unwrap();
        let nombre = p.file_stem().unwrap().to_str().unwrap().to_string();
        match nombre.split("__").next().unwrap() {
            "MensajeDeCliente" => ida_y_vuelta::<MensajeDeCliente>(&bin),
            "MensajeDeServidor" => ida_y_vuelta::<MensajeDeServidor>(&bin),
            "ArtefactoDeMundo" => ida_y_vuelta::<ArtefactoDeMundo>(&bin),
            t => panic!("tipo dorado desconocido: {t}"),
        }
        n += 1;
    }
    assert!(n >= 70, "faltan ficheros dorados: {n}");
}

/// Lo que los dorados de 4H dicen, leído por Rust: cada campo nuevo está donde el esquema lo pone (el JSON es la fuente; aquí se comprueba el significado, no solo los bytes).
#[test]
fn los_dorados_de_4h_dicen_lo_que_deben() {
    use mensaje_de_cliente as mc;
    use mensaje_de_servidor as ms;
    use orden::Cuerpo as O;

    // La economía en escalones, la población entera, la ciudad y el asedio visible.
    let ms::Cuerpo::Foto(f) = de_servidor("foto_asentamiento") else { panic!("no es una foto") };
    let Some(entidad::Cuerpo::Asentamiento(a)) = f.entidades[0].cuerpo.clone() else { panic!("no es un asentamiento") };
    assert!(a.almacen["madera"].cantidad.as_ref().unwrap().en_escalones && a.poblacion.as_ref().unwrap().pesants.as_ref().unwrap().en_escalones);
    assert_eq!(a.poblacion.unwrap().pesants.unwrap().valor_en_t0 % 1_000, 0, "la población es entera");
    assert_eq!((a.edificios[1].ancho, a.edificios[1].alto, a.edificios[1].rotado), (6, 12, true));
    let t = a.trazado.unwrap();
    assert_eq!((t.layout_version, t.unidades_por_celda, t.radio_ul, t.calles.len(), t.caminos.len()), (2, 3, 30, 1, 1));
    assert_eq!(a.recintos[0].id, "recinto-3-1");
    assert!(a.batalla_id.is_some());

    // El interior de la plaza (T3c): la tendencia en el contador, la recuperación del hambre, la cola con su prioridad, la guarnición y quién está dentro.
    let ms::Cuerpo::Foto(f) = de_servidor("foto_asentamiento_interior") else { panic!() };
    let Some(entidad::Cuerpo::Asentamiento(a)) = f.entidades[0].cuerpo.clone() else { panic!() };
    let trigo = a.almacen["trigo"].cantidad.as_ref().unwrap();
    assert_eq!((trigo.valor_en_t0, trigo.por_minuto, trigo.en_escalones), (88_000, -2_500, true), "valor del servidor y tendencia «-N/min»");
    assert!(a.poblacion.as_ref().unwrap().en_recuperacion);
    assert_eq!(a.version_interior, 361);
    assert_eq!((a.edificios[0].prioridad, a.edificios[1].mejora.as_ref().unwrap().nivel_objetivo), (Some(120), 3));
    assert_eq!((a.guarnicion_ids.len(), a.cupo_guarnicion, a.presentes_ids.len(), a.caravanas_aparcadas_ids.len(), a.flota_ids.len()), (2, 24, 2, 1, 2));
    assert_eq!((a.politicas_activas[0].cargo.as_str(), a.ascenso.unwrap().nivel_objetivo), ("sacerdote", 3));
    // El paso de minuto manda solo lo que cambia, con su máscara.
    let ms::Cuerpo::Delta(d) = de_servidor("delta_paso_de_minuto") else { panic!() };
    assert!(!d.cambios[0].reemplaza);
    assert_eq!(d.cambios[0].campos, ["version", "version_interior", "poblacion", "almacen", "nutricion"]);
    // La secuencia propia del interior: el paso de minuto no sube la versión del mundo (`desde == hasta`) y la plaza y su pueblo dicen de qué minuto son.
    assert_eq!((d.desde, d.hasta), (90, 90));
    let Some(entidad::Cuerpo::Asentamiento(a)) = d.cambios[0].datos.clone().and_then(|e| e.cuerpo) else { panic!() };
    let Some(entidad::Cuerpo::Pueblo(p)) = d.cambios[1].datos.clone().and_then(|e| e.cuerpo) else { panic!() };
    assert_eq!((a.version, a.version_interior, p.version_interior), (90, 362, 362));
    // Y la reanudación la lleva en los dos sentidos: lo que el cliente conserva y lo que el servidor tiene.
    let mc::Cuerpo::Hola(h) = de_cliente("hola_reanudando_la_plaza") else { panic!() };
    assert_eq!((h.versiones_conocidas["ASENTAMIENTO:as-3"], h.versiones_interiores_conocidas["ASENTAMIENTO:as-3"]), (90, 361));
    let ms::Cuerpo::Bienvenida(b) = de_servidor("bienvenida_reanudando_la_plaza") else { panic!() };
    assert_eq!(b.ambitos.iter().map(|x| (x.decision, x.version_interior_actual)).collect::<Vec<_>>(), [(DecisionReanudacion::ReanudaSinNovedad as i32, 0), (DecisionReanudacion::ReanudaConDeltas as i32, 362)]);
    // El campamento visto desde dentro.
    let ms::Cuerpo::Foto(f) = de_servidor("foto_campamento_mercenarios") else { panic!() };
    let Some(entidad::Cuerpo::CampamentoMercenarios(c)) = f.entidades[0].cuerpo.clone() else { panic!() };
    assert_eq!((c.presentes_ids.len(), c.prestamos_ids.as_slice(), c.tropas_ofrecidas.len()), (2, &["esc-31".to_owned()][..], 2));
    assert_eq!(c.cupo_diario.unwrap().por_id["madera"], 60_000);

    // La ubicación dentro de un pueblo, la columna dentro de un lugar y la avanzada con sus elecciones por escalón.
    let ms::Cuerpo::Foto(f) = de_servidor("foto_heroe") else { panic!() };
    let Some(entidad::Cuerpo::Heroe(h)) = f.entidades[0].cuerpo.clone() else { panic!() };
    assert_eq!(h.ubicacion.unwrap().donde, Some(ubicacion_heroe::Donde::PuebloId("pueblo-5".into())));
    let ms::Cuerpo::Foto(f) = de_servidor("foto_heroe_con_residencia") else { panic!() };
    let Some(entidad::Cuerpo::Heroe(h)) = f.entidades[0].cuerpo.clone() else { panic!() };
    assert_eq!((h.residencia_id.as_deref(), h.ubicacion.unwrap().donde), (Some("as-7"), Some(ubicacion_heroe::Donde::AsentamientoId("as-7".into()))), "la residencia es aparte de dónde está");
    let ms::Cuerpo::Foto(f) = de_servidor("foto_ejercito_dentro") else { panic!() };
    let Some(entidad::Cuerpo::Ejercito(e)) = f.entidades[0].cuerpo.clone() else { panic!() };
    assert_eq!((e.dentro_de.as_deref(), e.en_batalla, e.batalla_id.is_some()), (Some("pueblo-5"), true, true));
    let ms::Cuerpo::Foto(f) = de_servidor("foto_avanzada") else { panic!() };
    let Some(entidad::Cuerpo::Avanzada(v)) = f.entidades[0].cuerpo.clone() else { panic!() };
    assert_eq!((v.tropa_elegida_por_escalon[&2].as_str(), v.guarnicion_jugadores_ids.len()), ("lanceros_mimbre", 1));

    // La batalla: lado para unirse, token propio y la lista de caravanas del contexto.
    let ms::Cuerpo::Foto(f) = de_servidor("foto_batalla") else { panic!() };
    let Some(entidad::Cuerpo::Batalla(b)) = f.entidades[0].cuerpo.clone() else { panic!() };
    assert_eq!((b.lado_para_unirse, b.asignacion_propia.unwrap().token.as_str()), (Some(LadoId::Atacante as i32), "tok-heroe-3"));
    let Some(contexto_estrategico::Tipo::Caravana(c)) = b.ticket.unwrap().contexto_estrategico.unwrap().tipo else { panic!() };
    assert_eq!((c.caravanas.len(), c.caravanas[1].carros.len(), c.caravanas[1].carros[1].carro_id.as_str()), (2, 2, "car-6:1"));

    // La forma híbrida de un cambio.
    let ms::Cuerpo::Delta(d) = de_servidor("delta_cambio_completo") else { panic!() };
    assert!(d.cambios[0].reemplaza && d.cambios[0].campos.is_empty());
    let ms::Cuerpo::Delta(d) = de_servidor("delta_cambio_parcial_de_asentamiento") else { panic!() };
    assert!(!d.cambios[0].reemplaza);
    assert_eq!(d.cambios[0].campos, ["almacen", "trazado", "batalla_id"]);

    // Los quince eventos tipados, cada uno con su mensaje, más el payload genérico.
    let ms::Cuerpo::Delta(d) = de_servidor("delta_eventos_tipados") else { panic!() };
    assert_eq!(d.eventos.len(), 15);
    assert!(d.eventos.iter().all(|e| e.tipado.is_some() && e.payload.is_some()));
    assert!(matches!(&d.eventos[2].tipado, Some(evento_dominio::Tipado::BatallaCerrada(c)) if c.estado == EstadoBatalla::BatallaAplicada as i32 && c.ganador == Some(LadoId::Atacante as i32)));
    // T5: las Facciones que oyen un hecho y un payload con una lista de textos (la entrada pública de la crónica, sin plaza ni nombrados).
    let ms::Cuerpo::Delta(d) = de_servidor("delta_eventos_de_facciones_y_publico") else { panic!() };
    assert_eq!(d.eventos[0].facciones_ids, ["f-1", "f-2"]);
    let cronica = &d.eventos[1];
    assert!(cronica.asentamiento_id.is_none() && cronica.heroes_ids.is_empty() && cronica.facciones_ids.is_empty() && cronica.tipado.is_none());
    let Some(prost_types::value::Kind::ListValue(l)) = &cronica.payload.as_ref().unwrap().fields["faccionIds"].kind else { panic!() };
    assert_eq!(l.values.len(), 2);

    // El informe de inspección con recintos, lo creado con su clase, el detalle del rechazo de guerra y el campamento para nacer.
    let ms::Cuerpo::Foto(f) = de_servidor("foto_informe_inspeccion_plaza") else { panic!() };
    let Some(entidad::Cuerpo::InformeInspeccion(i)) = f.entidades[0].cuerpo.clone() else { panic!() };
    assert!(matches!(i.contenido, Some(informe_inspeccion::Contenido::Plaza(p)) if p.recintos.len() == 1 && p.guarnicion.len() == 1));
    let ms::Cuerpo::Resultado(r) = de_servidor("resultado_creadas") else { panic!() };
    assert_eq!((r.creadas[0].entidad, r.creadas[0].id.as_str()), (TipoEntidad::Edificio as i32, "ed-3-12"));
    let ms::Cuerpo::Resultado(r) = de_servidor("resultado_guerra_confirmar") else { panic!() };
    assert_eq!(r.rechazo.unwrap().detalle.unwrap().fields["objetivo"].kind, Some(prost_types::value::Kind::StringValue("f-2".into())));
    let ms::Cuerpo::Bienvenida(b) = de_servidor("bienvenida_sin_heroe") else { panic!() };
    assert!(b.heroe_id.is_none() && b.campamentos_para_elegir[0].residentes == 5);

    // El canal de batalla: tokens y acuse hacia el servidor de batalla, tokens y carros perdidos desde él.
    let ms::Cuerpo::Batalla(m) = de_servidor("batalla_acuse_rechazado") else { panic!() };
    let Some(mensaje_para_servidor_de_batalla::Cuerpo::Acuse(a)) = m.cuerpo else { panic!() };
    assert_eq!((a.mensaje.as_str(), a.resultado, a.rechazo.unwrap().codigo.as_str()), ("asignacion", Resultado::OrdenRechazada as i32, "batalla.ya_asignada"));
    let mc::Cuerpo::Batalla(m) = de_cliente("batalla_tokens") else { panic!() };
    assert!(matches!(m.cuerpo, Some(mensaje_de_servidor_de_batalla::Cuerpo::Tokens(t)) if t.tokens_participante[0].heroe_id == "heroe-4"));
    let mc::Cuerpo::Batalla(m) = de_cliente("batalla_resultado_con_carros_perdidos") else { panic!() };
    assert!(matches!(m.cuerpo, Some(mensaje_de_servidor_de_batalla::Cuerpo::Resultado(r)) if r.carros_perdidos == ["car-7:0", "car-7:1"]));
    let mc::Cuerpo::Batalla(m) = de_cliente("batalla_asignacion_sin_humanos") else { panic!() };
    assert!(matches!(m.cuerpo, Some(mensaje_de_servidor_de_batalla::Cuerpo::Asignacion(a)) if a.tokens_participante.is_empty()), "una batalla sin humanos se asigna sin tokens");

    // La vista con niebla (T3d): lo ajeno redactado (la columna con su tramo y `te_sigue`, la caravana con sus recursos sin cantidades, el héroe con lo público), lo recordado como ficha
    // (una plaza con su radio y un pueblo) y la vista de un héroe sin Facción, con el `QUITA` de lo que deja de verse y la ficha de una avanzada en una delta que no sube la versión.
    let ms::Cuerpo::Foto(f) = de_servidor("foto_vista_con_lo_ajeno") else { panic!() };
    assert_eq!(f.niebla.as_ref().map(|n| (n.celdas.as_slice(), n.visibles.as_slice())), Some((&[0xff, 0x0f][..], &[0x06, 0x00][..])));
    let cuerpos: Vec<entidad::Cuerpo> = f.entidades.into_iter().filter_map(|e| e.cuerpo).collect();
    let [entidad::Cuerpo::Ejercito(x), entidad::Cuerpo::Caravana(c), entidad::Cuerpo::Heroe(h), entidad::Cuerpo::FichaConocida(plaza), entidad::Cuerpo::FichaConocida(pueblo)] = cuerpos.as_slice() else { panic!("{cuerpos:?}") };
    assert!(x.te_sigue && x.escuadron_ids.is_empty() && x.plan.as_ref().unwrap().vertices.len() == 2);
    assert_eq!((c.recursos_avistados.as_slice(), c.contenido.is_none()), (&["madera".to_owned(), "trigo".to_owned()][..], true));
    assert_eq!((h.escuadras_que_lleva[0].tropa_id.as_str(), h.escuadras_que_lleva[0].cantidad, h.jugador_id.is_none()), ("miliciaLanceros", 40, true));
    assert_eq!((plaza.tipo, plaza.radio_potencial, pueblo.tipo, pueblo.radio_potencial), (TipoEntidad::Asentamiento as i32, Some(15_000), TipoEntidad::Pueblo as i32, None));
    let ms::Cuerpo::Delta(d) = de_servidor("delta_vista_sin_faccion") else { panic!() };
    assert_eq!((d.ambito.as_ref().unwrap().id.as_str(), d.desde, d.hasta, d.niebla.is_some()), ("heroe-3", 81, 81, true));
    assert_eq!((d.cambios[0].tipo, d.cambios[1].entidad), (TipoCambio::CambioQuita as i32, TipoEntidad::FichaConocida as i32));
    assert!(matches!(d.cambios[1].datos.as_ref().and_then(|e| e.cuerpo.as_ref()), Some(entidad::Cuerpo::FichaConocida(a)) if a.tipo == TipoEntidad::Avanzada as i32 && a.nivel == 2));
    // La niebla parcial (T6): sin las máscaras, con los tramos de bytes que cambiaron de cada una.
    let ms::Cuerpo::Delta(d) = de_servidor("delta_vista_niebla_parcial") else { panic!() };
    let niebla = d.niebla.expect("la niebla");
    assert!(niebla.parcial && niebla.celdas.is_empty() && niebla.visibles.is_empty() && d.cambios.is_empty());
    assert_eq!((niebla.tramos_celdas[0].desde, niebla.tramos_celdas[0].datos.as_slice(), niebla.tramos_visibles[0].desde, niebla.tramos_visibles[0].datos.as_slice()), (1, &[0x0F][..], 0, &[0x02][..]));

    // `ADMIN` (T3e): la batalla entera con la asignación que ganó, sin tokens ni el token propio de nadie.
    let ms::Cuerpo::Foto(f) = de_servidor("foto_admin_batalla") else { panic!() };
    assert_eq!(f.ambito.as_ref().map(|a| (a.tipo, a.id.as_str())), Some((AmbitoTipo::AmbitoAdmin as i32, "")));
    let Some(entidad::Cuerpo::Batalla(b)) = f.entidades[0].cuerpo.clone() else { panic!() };
    let a = b.asignacion.expect("la asignación");
    assert_eq!((a.intento_asignacion_id.as_str(), a.instancia.unwrap().host.as_str(), a.tokens_participante.len(), a.servidor_id.as_str()), ("intento-a", "unity-1.example", 0, "unity-1"));
    assert!(b.ticket.is_some() && b.asignacion_propia.is_none() && b.lado_propio.is_none());

    // T7: lo que el núcleo ya sabía y el contrato no llevaba. En `VISTA`, de quién es el suelo bajo una columna propia, el almacén de la plaza propia a cuya puerta está, la zona de la
    // Facción fundida (el exterior antihorario y el hueco horario) y la red con el escalón de cada arista y las rutas propias; en `HEROE`, el interior que recuerda de su plaza.
    let ms::Cuerpo::Foto(f) = de_servidor("foto_vista_suelo_caminos_y_zona_fundida") else { panic!() };
    assert_eq!(f.ambito.as_ref().map(|a| a.id.as_str()), Some("f-1"), "la `VISTA` de una Facción es `VISTA:f-N`");
    let cuerpos: Vec<entidad::Cuerpo> = f.entidades.into_iter().filter_map(|e| e.cuerpo).collect();
    let [entidad::Cuerpo::Ejercito(e), entidad::Cuerpo::Asentamiento(plaza), entidad::Cuerpo::ZonaFaccion(z), entidad::Cuerpo::RedCaminos(red)] = cuerpos.as_slice() else { panic!("{cuerpos:?}") };
    assert_eq!(e.territorio_faccion_id.as_deref(), Some("f-2"));
    assert_eq!(plaza.almacen["madera"].cantidad.as_ref().unwrap().valor_en_t0, 5_000);
    let area = |c: &ContornoZona| (0..c.puntos.len()).map(|i| { let (p, q) = (&c.puntos[i], &c.puntos[(i + 1) % c.puntos.len()]); p.x * q.y - p.y * q.x }).sum::<i64>();
    assert!(area(&z.contornos[0]) > 0 && area(&z.contornos[1]) < 0, "el exterior y el hueco van en sentidos opuestos");
    assert_eq!((red.aristas.len(), red.escalones.as_slice(), red.rutas[0].id.as_str()), (1, &[1][..], "ruta-3-4"));
    let ms::Cuerpo::Foto(f) = de_servidor("foto_heroe_con_plazas_recordadas") else { panic!() };
    let Some(entidad::Cuerpo::Heroe(h)) = f.entidades[0].cuerpo.clone() else { panic!() };
    let r = &h.plazas_recordadas["as-3"];
    assert_eq!((r.visto_en_ms, r.cola[0].id.as_str(), r.cola[0].completa_en_ms, r.guarnicion_escuadrones_ids.as_slice()), (60_000, "ed-3-7", Some(90_000), &["esc-5".to_owned()][..]));

    // Los ids de los dorados son los reales: los ámbitos `TIPO:id` de `proyeccion::ambito` (la `VISTA` de una Facción, `f-N`), las batallas como UUID y la partida como `<mundo>:<semilla>`.
    for nombre in ["foto_ejercito", "foto_ejercito_dentro", "foto_vista_con_niebla", "delta_parcial", "delta_cambio_completo"] {
        let ambito = match de_servidor(nombre) {
            ms::Cuerpo::Foto(f) => f.ambito,
            ms::Cuerpo::Delta(d) => d.ambito,
            otro => panic!("{nombre}: {otro:?}"),
        };
        assert_eq!(ambito.map(|a| (a.tipo, a.id)), Some((AmbitoTipo::AmbitoVista as i32, "f-1".to_owned())), "{nombre}");
    }
    let ms::Cuerpo::Batalla(m) = de_servidor("batalla_ticket_asedio") else { panic!() };
    let Some(mensaje_para_servidor_de_batalla::Cuerpo::Ticket(t)) = m.cuerpo else { panic!() };
    assert!(t.battle_id.len() == 36 && t.battle_id.matches('-').count() == 4 && t.game_id.contains(':'), "{} {}", t.battle_id, t.game_id);

    // Las órdenes nuevas.
    let orden_de = |nombre: &str| match de_cliente(nombre) {
        mc::Cuerpo::Orden(o) => o.cuerpo.unwrap(),
        otro => panic!("{nombre}: {otro:?}"),
    };
    assert!(matches!(orden_de("orden_cancelar_orden_mercado"), O::CancelarOrdenMercado(o) if o.orden_id == "ord-9"));
    assert!(matches!(orden_de("orden_aparcar_caravana"), O::AparcarCaravana(o) if (o.caravana_id.as_str(), o.asentamiento_id.as_str()) == ("car-4", "as-3")));
    assert!(matches!(orden_de("orden_retirar_de_avanzada"), O::RetirarDeAvanzada(o) if o.escuadron_ids == ["esc-9"]));
    assert!(matches!(orden_de("orden_depositar_en_avanzada"), O::DepositarEnAvanzada(o) if o.cantidad == 5_000));
    assert!(matches!(orden_de("orden_vaciar_carro"), O::VaciarCarro(o) if o.heroe_id == "heroe-3"));
    assert!(matches!(orden_de("orden_entrar_en_pueblo"), O::EntrarEnPueblo(o) if o.pueblo_id == "pueblo-5"));
    assert!(matches!(orden_de("orden_salir_de_pueblo"), O::SalirDePueblo(o) if o.heroe_id == "heroe-3"));
    assert!(matches!(orden_de("orden_entrar_en_avanzada"), O::EntrarEnAvanzada(o) if o.avanzada_id == "av-6"));
    assert!(matches!(orden_de("orden_salir_de_avanzada"), O::SalirDeAvanzada(o) if o.avanzada_id == "av-6"));
    assert!(matches!(orden_de("orden_repostar_con_lugar"), O::Repostar(o) if o.lugar_id.as_deref() == Some("av-6")));
    assert!(matches!(orden_de("orden_administracion"), O::Administracion(o) if o.codigo == "dar_recursos" && o.parametros.as_ref().unwrap().fields.len() == 1));
}

/// Lo que los dorados de 5H (la política en el contrato) dicen, leído por Rust.
#[test]
fn los_dorados_de_5h_dicen_lo_que_deben() {
    use evento_dominio::Tipado as T;
    use mensaje_de_servidor as ms;

    // `FACCION`: la Facción con su Liga y las tres clases de propuesta, cada una con el héroe que la hizo (5A-7).
    let ms::Cuerpo::Foto(f) = de_servidor("foto_faccion_con_propuestas_y_liga") else { panic!() };
    let cuerpos: Vec<entidad::Cuerpo> = f.entidades.into_iter().filter_map(|e| e.cuerpo).collect();
    let [entidad::Cuerpo::Faccion(fa), entidad::Cuerpo::Propuesta(rel), entidad::Cuerpo::Propuesta(anx), entidad::Cuerpo::Propuesta(fus)] = cuerpos.as_slice() else { panic!("{cuerpos:?}") };
    let liga = fa.liga.as_ref().expect("la Liga");
    assert_eq!((liga.miembros_ids.len(), liga.tiene_vasallaje, liga.gran_rey_id.as_deref()), (3, true, Some("f-1")));
    assert!(matches!(&rel.clase, Some(propuesta::Clase::Relacion(r)) if r.tipo == TipoRelacion::RelacionVasallaje as i32 && r.tributo.as_ref().unwrap().cantidad_por_minuto == 1_500));
    assert!(matches!(&anx.clase, Some(propuesta::Clase::Anexion(a)) if a.absorbida_id == "f-5"));
    assert!(matches!(&fus.clase, Some(propuesta::Clase::Fusion(x)) if x.nuevo_nombre == "Hélade" && x.nuevo_rey_id == "heroe-9"));
    assert_eq!((rel.propuesta_por.as_str(), anx.propuesta_por.as_str()), ("heroe-3", "heroe-5"));
    // Contestarla la quita con `QUITA` y trae la relación, con su hecho tipado.
    let ms::Cuerpo::Delta(d) = de_servidor("delta_propuesta_contestada") else { panic!() };
    assert_eq!((d.cambios[0].tipo, d.cambios[0].entidad, d.cambios[0].id.as_str()), (TipoCambio::CambioQuita as i32, TipoEntidad::Propuesta as i32, "prop-40"));
    assert!(matches!(&d.eventos[0].tipado, Some(T::PropuestaResuelta(r)) if r.resultado == "aceptada" && r.relacion_id.as_deref() == Some("rel-43")));
    // `VISTA`: la forma pública de otra Facción, sin experiencia ni capital, y el Gran Rey.
    let ms::Cuerpo::Foto(f) = de_servidor("foto_vista_faccion_publica") else { panic!() };
    let Some(entidad::Cuerpo::Faccion(p)) = f.entidades[0].cuerpo.clone() else { panic!() };
    assert!(p.experiencia == 0 && p.capital_id.is_none() && p.capital_designada_en_ms.is_none() && p.liga.is_some() && p.reputacion.unwrap().valor_en_t0 == -4_000);
    assert!(matches!(&f.entidades[1].cuerpo, Some(entidad::Cuerpo::Titulo(t)) if t.titulo_id == "granRey"));
    // `HEROE`: su Facción y sus solicitudes (5B-11); `ASENTAMIENTO`: sus fundadores (5B-9) y su Gobernador de oficio (5E-3).
    let ms::Cuerpo::Foto(f) = de_servidor("foto_heroe_con_faccion_y_solicitudes") else { panic!() };
    let Some(entidad::Cuerpo::Heroe(h)) = f.entidades[0].cuerpo.clone() else { panic!() };
    assert_eq!((h.faccion_id.as_deref(), h.solicitudes_faccion_ids.as_slice()), (Some("f-3"), &["f-1".to_owned(), "f-2".to_owned()][..]));
    let ms::Cuerpo::Foto(f) = de_servidor("foto_asentamiento_con_fundadores") else { panic!() };
    let Some(entidad::Cuerpo::Asentamiento(a)) = f.entidades[0].cuerpo.clone() else { panic!() };
    assert_eq!((a.fundadores_ids.as_slice(), a.cargos.unwrap().gobernador_id.as_deref()), (&["heroe-3".to_owned(), "heroe-5".to_owned()][..], Some("heroe-3")));
    // Los ocho hechos políticos tipados, con su payload.
    let ms::Cuerpo::Delta(d) = de_servidor("delta_eventos_politicos") else { panic!() };
    assert_eq!(d.eventos.len(), 8);
    assert!(d.eventos.iter().all(|e| e.tipado.is_some() && e.payload.is_some()));
    let t: Vec<&T> = d.eventos.iter().filter_map(|e| e.tipado.as_ref()).collect();
    assert!(matches!(t[0], T::PropuestaDiplomatica(p) if p.clase == "anexion" && p.tipo.is_none()));
    assert!(matches!(t[1], T::PropuestaDiplomatica(p) if p.tipo == Some(TipoRelacion::RelacionAlianza as i32)));
    assert!(matches!(t[2], T::RelacionTerminada(r) if r.como == "rota" && r.por_faccion_id.as_deref() == Some("f-2")));
    assert!(matches!(t[3], T::FaccionesUnidas(u) if u.resultante_id == "f-9" && u.nombre.as_deref() == Some("Hélade")));
    assert!(matches!(t[4], T::CargoAsignado(c) if c.cargo == "tesorero" && c.asentamiento_id.as_deref() == Some("as-7")));
    assert!(matches!(t[5], T::TituloCambiado(x) if x.faccion_anterior_id.as_deref() == Some("f-2")));
    assert!(matches!(t[6], T::SolicitudDeIngreso(s) if s.heroe_id == "heroe-11"));
    assert!(matches!(t[7], T::CiudadaniaCambiada(c) if c.como == "ingreso"));
}

/// La crónica por páginas (sistema 6, tanda 6C): la petición hacia atrás desde una versión y la página con sus entradas `cronica.entrada`, la versión del mundo y si quedan más.
#[test]
fn la_cronica_por_paginas() {
    let mensaje_de_cliente::Cuerpo::PedirCronica(p) = de_cliente("pedir_cronica") else { panic!() };
    assert_eq!((p.peticion_id.as_str(), p.desde_version, p.hasta_version, p.limite), ("cronica-2", None, Some(639), 200));
    let mensaje_de_servidor::Cuerpo::Cronica(c) = de_servidor("cronica") else { panic!() };
    assert_eq!((c.peticion_id.as_str(), c.version, c.mas_antiguas, c.mas_recientes), ("cronica-2", 812, true, false));
    assert!(c.entradas.iter().all(|e| e.codigo == "cronica.entrada" && e.asentamiento_id.is_none() && e.tipado.is_none()));
    assert!(c.entradas.windows(2).all(|w| w[0].version < w[1].version) && c.entradas.last().unwrap().version <= p.hasta_version.unwrap());
}

/// Lo que los dorados de 6F (los alijos) dicen, leído por Rust: la orden con el id del artefacto y el héroe con los que abrió.
#[test]
fn los_dorados_de_6f_dicen_lo_que_deben() {
    use mensaje_de_servidor as ms;
    use orden::Cuerpo as O;
    let mensaje_de_cliente::Cuerpo::Orden(o) = de_cliente("orden_abrir_alijo") else { panic!() };
    assert!(matches!(o.cuerpo, Some(O::AbrirAlijo(a)) if a.alijo_id == "alijo-veta-3"));
    let ms::Cuerpo::Foto(f) = de_servidor("foto_heroe_con_alijos_abiertos") else { panic!() };
    let Some(entidad::Cuerpo::Heroe(h)) = f.entidades[0].cuerpo.clone() else { panic!() };
    assert_eq!(h.alijos_abiertos, ["alijo-mercenarios-0-1", "alijo-veta-3"]);
}

/// Lo que los dorados de 6G (la tecnología y los Aedas en el contrato) dicen, leído por Rust: cada campo nuevo donde el esquema lo pone y los ids reales.
#[test]
fn los_dorados_de_6g_dicen_lo_que_deben() {
    use entidad::Cuerpo as C;
    use evento_dominio::Tipado as T;
    use mensaje_de_servidor as ms;
    let cuerpos = |f: &Foto| f.entidades.iter().filter_map(|e| e.cuerpo.clone()).collect::<Vec<_>>();
    // FACCION: las tres listas, el detalle de la revelada, las épicas posibles y el residente con su épica (sin claves).
    let ms::Cuerpo::Foto(f) = de_servidor("foto_faccion_con_tecnologias_y_residente") else { panic!() };
    let [C::Tecnologias(t), C::AedaResidente(r)] = &cuerpos(&f)[..] else { panic!("{f:?}") };
    assert_eq!((t.reveladas.clone(), t.detalle_reveladas[0].descubridora_faccion_id.as_str(), t.detalle_reveladas[0].cumplidas.clone(), t.epicas_posibles.clone()), (vec!["metalurgia_cobre".to_owned()], "f-1", vec![true, false], vec!["metalurgia_cobre".to_owned()]));
    let e = r.epica.as_ref().unwrap();
    assert_eq!((r.id.as_str(), e.capitulos, e.hechos_necesarios, e.claves.len()), ("aeda-r1", 2, 3, 0));
    // HEROE: la forma pública (sin contadores ni logros por tecnología) y un alijo a la vista.
    let ms::Cuerpo::Foto(f) = de_servidor("foto_heroe_con_tecnologia_publica_y_alijo") else { panic!() };
    let [C::EstadoTecnologia(t), C::Alijo(a)] = &cuerpos(&f)[..] else { panic!("{f:?}") };
    assert!(t.contadores.is_empty() && t.logros.is_empty());
    assert_eq!((t.logros_cumplidos[0].contador.as_str(), t.logros_cumplidos[0].umbral, t.cantadas.clone(), a.id.as_str(), a.oro), ("plazasEnNivel.2", 1, vec!["metalurgia_cobre".to_owned()], "alijo-veta-3", 5_500));
    // VISTA: el Aeda parado con su plaza y el que anda con su tramo; ni estancia, ni destino, ni dónde estuvo.
    let ms::Cuerpo::Foto(f) = de_servidor("foto_vista_con_aedas") else { panic!() };
    let [C::AedaItinerante(x), C::AedaItinerante(y)] = &cuerpos(&f)[..] else { panic!("{f:?}") };
    assert_eq!((x.en_asentamiento_id.as_deref(), y.plan.as_ref().map(|p| p.vertices.len())), (Some("as-6"), Some(2)));
    assert!([x, y].iter().all(|a| a.hasta_ms.is_none() && a.destino_id.is_none() && a.recientes.is_empty()));
    // ADMIN: el estado entero con su contador en curso y su clase propia `ESTADO_TECNOLOGIA`, el Aeda entero y el residente con sus claves.
    let ms::Cuerpo::Delta(d) = de_servidor("delta_admin_tecnologia_y_aedas") else { panic!() };
    assert_eq!(d.cambios.iter().map(|c| (c.entidad, c.id.as_str())).collect::<Vec<_>>(), [(TipoEntidad::EstadoTecnologia as i32, "tecnologia"), (TipoEntidad::Aeda as i32, "aeda-1"), (TipoEntidad::Aeda as i32, "aeda-r1")]);
    let Some(C::EstadoTecnologia(t)) = d.cambios[0].datos.as_ref().and_then(|x| x.cuerpo.clone()) else { panic!() };
    assert_eq!((t.contadores.get("extraido.cobre"), t.logros.get("metalurgia_cobre")), (Some(&412), Some(&3_600_000)));
    // Los diez mensajes tipados de la tecnología y los Aedas.
    let ms::Cuerpo::Delta(d) = de_servidor("delta_eventos_tecnologia") else { panic!() };
    let tipados: Vec<T> = d.eventos.iter().filter_map(|e| e.tipado.clone()).collect();
    assert_eq!(tipados.len(), 10);
    assert!(matches!(&tipados[1], T::EraComenzada(x) if x.era_id == "edad_bronce" && x.por_plazo));
    assert!(matches!(&tipados[6], T::CompraAAeda(x) if x.aeda_id == "aeda-1" && x.precio == 200_000));
    assert!(matches!(&tipados[8], T::EpicaCambia(x) if x.motivo.as_deref() == Some("unificada")));
    assert!(matches!(&tipados[9], T::AlijoAbierto(x) if x.cargado == 3_000));
}
