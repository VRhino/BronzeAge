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
    assert!(n >= 65, "faltan ficheros dorados: {n}");
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
    assert_eq!((a.edificios[0].prioridad, a.edificios[1].mejora.as_ref().unwrap().nivel_objetivo), (Some(120), 3));
    assert_eq!((a.guarnicion_ids.len(), a.cupo_guarnicion, a.presentes_ids.len(), a.caravanas_aparcadas_ids.len(), a.flota_ids.len()), (2, 24, 2, 1, 2));
    assert_eq!((a.politicas_activas[0].cargo.as_str(), a.ascenso.unwrap().nivel_objetivo), ("sacerdote", 3));
    // El paso de minuto manda solo lo que cambia, con su máscara.
    let ms::Cuerpo::Delta(d) = de_servidor("delta_paso_de_minuto") else { panic!() };
    assert!(!d.cambios[0].reemplaza);
    assert_eq!(d.cambios[0].campos, ["version", "poblacion", "almacen", "nutricion"]);
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
