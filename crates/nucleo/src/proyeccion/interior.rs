//! El interior de una plaza y de un campamento de mercenarios, vistos desde dentro (contrato 05 §3, 4H T3c): los ámbitos `ASENTAMIENTO` y `CAMPAMENTO`.
//!
//! - [`asentamiento`]: la plaza completa. El almacén y la población van **en escalones** (la economía va por minuto): cada contador lleva el valor del servidor y su tendencia «+N/min» (el neto real del
//!   último paso de minuto, guardado por plaza: `asentamiento::asentamiento::Tendencia`); la población es entera por clase. Con ellos, el medidor de mantenimiento y la nutrición (más la marca de recuperación), los edificios con
//!   su cola y sus fines exactos, las mejoras, los recintos de muralla y su obra, el trazado urbano, las políticas, el ascenso, el acceso por relación, la guarnición, quiénes están dentro, las
//!   caravanas aparcadas y la flota, y el asedio si lo hay. Es función del estado, no del instante en que se pide: dos proyecciones del mismo estado son iguales.
//! - [`mascara`] y [`aplicar`]: la plaza viaja con **cambios parciales con máscara** (decisión 2 del autor): del cambio solo van los campos que cambiaron, y la máscara dice cuáles, de modo que el paso
//!   de minuto manda el almacén, la población y poco más, no la ciudad entera. `aplicar` es lo que hace el cliente.
//! - [`campamento`]: el campamento de mercenarios completo (viaja entero, como todo lo demás): reclutas, stock en venta, contadores del mercado, residentes y presentes, préstamos y tropas que ofrece.
//!
//! DUDA: el cargo de cada puesto (Gobernador...) y el nombre son del sistema 5 y salen vacíos; el trazado de una pieza va en milésimas de unidad local como la del ticket (`Edificio.posicion` = centro de
//! su huella); la prioridad de un edificio en cola es su puntuación, recortada a 32 bits; la fuente de una Leñera se nombra `parcela-<n>`; `Cargos`, `permite_reabastecer_aliados` y `ultima_caravana_creada_en_ms`
//! salen a su valor por defecto; de los edificios fijos de un campamento (taberna, mercado...) el motor solo conoce sus viviendas y su edificio militar; las celdas de cada recinto van enteras con su
//! `avance` (el cliente deduce las que están en pie y las que faltan) y el progreso de una mejora de recinto no se dibuja.

use super::propios::{batalla_sobre, cantidades, celdas, constante, existencia};
use super::{ficha_de_asentamiento, ids, punto};
use crate::estado::Clase;
use crate::nucleo::Nucleo;
use asentamiento::edificio::{Estado as EstadoEdificio, Fuente};
use asentamiento::recurso::TODOS;
use contrato::v1 as pb;
use militar::presencia::Ubicacion;

fn ritmo_en_escalones(valor: base::Mil, tendencia: base::Mil, maximo: Option<base::Mil>) -> pb::Ritmo {
    pb::Ritmo { en_escalones: true, maximo, minimo: Some(0), por_minuto: tendencia, ..constante(valor) }
}

fn rect(r: &trazado::RectLocal) -> pb::RectLocal {
    pb::RectLocal { x: r.x, y: r.y, ancho: r.ancho.max(0) as u32, alto: r.alto.max(0) as u32 }
}


/// La plaza completa para quien está dentro, o `None` si ya no existe.
pub fn asentamiento(n: &Nucleo, id: u64) -> Option<pb::Asentamiento> {
    let e = n.estado();
    let a = e.asentamientos.get(&id)?;
    let (eco, datos) = (&a.economia, &e.reglas.economia.datos);
    let ciudad = n.vista_ciudad(id)?;
    let mut p = ficha_de_asentamiento(a, &n.poligono_zona(id), batalla_sobre(n, Ubicacion::Asentamiento(id as u32)));
    p.version = n.version_de(Clase::Asentamiento, id);
    p.version_interior = n.version_interior(id)?;

    p.racha_mantenimiento_sano = Some(eco.medidor.racha);
    let (t, entera) = (&eco.tendencia, |v: i64| v * 1_000);
    let (pesants, artesanos, nobleza) = eco.poblacion.enteros();
    p.poblacion = Some(pb::Poblacion {
        pesants: Some(ritmo_en_escalones(entera(pesants), entera(t.poblacion[0]), None)),
        artesanos: Some(ritmo_en_escalones(entera(artesanos), entera(t.poblacion[1]), None)),
        nobleza: Some(ritmo_en_escalones(entera(nobleza), entera(t.poblacion[2]), None)),
        en_recuperacion: eco.poblacion.recuperando,
    });
    p.almacen = TODOS.iter().map(|&r| (r.nombre().to_owned(), existencia(&eco.almacen, r, t.almacen[r.indice()]))).collect();
    p.medidor_mantenimiento = Some(ritmo_en_escalones(eco.medidor.valor, 0, Some(100_000)));
    p.nutricion = Some(ritmo_en_escalones(eco.poblacion.nutricion, 0, Some(100_000)));

    p.edificios = eco
        .edificios
        .iter()
        .map(|b| {
            let pieza = ciudad.piezas.iter().find(|x| x.id == b.id);
            pb::Edificio {
                id: ids::edificio(id, b.id),
                tipo: b.tipo.nombre().to_owned(),
                posicion: pieza.map(|x| punto(x.centro)),
                estado: match b.estado {
                    EstadoEdificio::EnCola => pb::EstadoEdificio::EdificioEnCola,
                    EstadoEdificio::EnConstruccion => pb::EstadoEdificio::EdificioEnConstruccion,
                    EstadoEdificio::Activo => pb::EstadoEdificio::EdificioActivo,
                } as i32,
                completa_en_ms: b.completa_en.filter(|_| b.estado == EstadoEdificio::EnConstruccion),
                ambito: pb::AmbitoEdificio::Asentamiento as i32,
                fuente_id: b.fuente.map(|f| match f {
                    Fuente::Nodo(x) => ids::nodo(x),
                    Fuente::Parcela(x) => ids::parcela(x),
                }),
                nivel_interno: Some(u32::from(b.nivel_interno)),
                mejora: b.mejora.map(|m| pb::MejoraEnCurso { nivel_objetivo: u32::from(m.nivel_objetivo), completa_en_ms: m.completa_en }),
                rotado: pieza.is_some_and(|x| x.rotado),
                prioridad: (b.estado == EstadoEdificio::EnCola).then(|| u32::try_from(b.prioridad.max(0)).unwrap_or(u32::MAX)),
                pausado_por_almacen_lleno: b.pausado_por_almacen,
                danado: b.danado,
                ancho: pieza.map_or(0, |x| x.rect.ancho.max(0) as u32),
                alto: pieza.map_or(0, |x| x.rect.alto.max(0) as u32),
                ..Default::default()
            }
        })
        .collect();
    p.recintos = eco
        .recintos
        .iter()
        .map(|r| pb::Recinto {
            id: ids::recinto(id, r.id),
            nivel: u32::from(r.nivel),
            celdas: celdas(&r.celdas),
            avance: r.levantadas() as u32,
            mejorando_a: r.mejora.map(|m| u32::from(m.nivel)),
            siguiente_celda_en_ms: r.siguiente_celda_en,
            comprometido_en_ms: r.comprometido_en,
            completado_en_ms: r.completado_en,
        })
        .collect();
    p.trazado = Some(pb::TrazadoUrbano {
        layout_version: trazado::LAYOUT_VERSION,
        unidades_por_celda: trazado::CELDA_UL as u32,
        radio_ul: ciudad.radio_ul,
        calles: ciudad.calles.iter().map(rect).collect(),
        caminos: ciudad.caminos.iter().map(rect).collect(),
    });

    p.casas_compradas = e.militar.residentes_de(id).map(|h| ids::heroe(h.heroe.id)).collect();
    p.presentes_ids = n.heroes().filter(|h| matches!(h.heroe.presencia.ubicacion, Ubicacion::Asentamiento(x) if u64::from(x) == id)).map(|h| ids::heroe(h.heroe.id)).collect();
    p.guarnicion_ids = n.guarnicion_de_plaza(id).into_iter().map(ids::escuadra).collect();
    p.cupo_guarnicion = u32::try_from(n.cupo_de_guarnicion(id).unwrap_or(0)).unwrap_or(0);
    p.puerta_cerrada_a = [(a.acceso.neutrales, pb::GrupoPuerta::GrupoNeutrales), (a.acceso.aliados, pb::GrupoPuerta::GrupoAliados), (a.acceso.enemigos, pb::GrupoPuerta::GrupoEnemigos)].into_iter().filter(|(abierta, _)| !abierta).map(|(_, g)| g as i32).collect();
    p.caravanas_aparcadas_ids = e.comercio.flota.iter().filter(|c| c.estado == comercio::caravana::EstadoCaravana::Aparcada && c.anfitriona.is_some_and(|x| u64::from(x) == id)).map(|c| ids::caravana(c.id)).collect();
    p.flota_ids = e.comercio.flota.iter().filter(|c| u64::from(c.origen) == id).map(|c| ids::caravana(c.id)).collect();

    let duracion = datos.politicas.duracion_minutos * base::MS_POR_MINUTO;
    p.politicas_activas = eco
        .politicas
        .activas()
        .iter()
        .map(|x| pb::PoliticaActiva { id: format!("pol-{id}-{}-{}", x.cargo.clave(), x.id), politica_id: x.id.clone(), cargo: x.cargo.clave().to_owned(), activada_en_ms: x.expira_en - duracion, expira_en_ms: x.expira_en })
        .collect();
    p.ascenso = eco.ascenso.map(|x| pb::AscensoEnCurso { nivel_objetivo: x.nivel_objetivo, iniciado_en_ms: x.iniciado_en, completa_en_ms: x.completa_en });
    p.ocupacion_hasta_ms = a.ocupada_hasta;
    p.protegida_hasta_ms = a.protegida_hasta;
    p.auto_construccion_pausada = eco.auto_pausada;
    p.recetas_pausadas = eco.recetas_pausadas.iter().map(|r| r.nombre().to_owned()).collect();
    p.reserva_manual = Some(cantidades(TODOS.iter().copied().zip(eco.reserva_manual).filter(|&(_, v)| v != 0)));
    Some(p)
}

pub use contrato::plaza::{aplicar, mascara, todos_los_campos};

/// El cambio de la plaza `id` para un cliente que tiene `antes` (la plaza que se le mandó por última vez), y la plaza que tendrá después. Con `antes`, un cambio **parcial con máscara** de lo que
/// cambió (ninguno si nada cambió); sin `antes` (el cliente reanuda y no se sabe qué tiene), la plaza entera, que sustituye a la suya. Una plaza que ya no existe se quita.
pub fn cambio_de_plaza(n: &Nucleo, id: u64, antes: Option<&pb::Asentamiento>) -> (Option<pb::Cambio>, Option<pb::Asentamiento>) {
    let cambio = |datos: pb::Asentamiento, reemplaza: bool, campos: Vec<String>| pb::Cambio {
        tipo: pb::TipoCambio::CambioActualiza as i32,
        entidad: pb::TipoEntidad::Asentamiento as i32,
        id: ids::asentamiento(id),
        datos: Some(pb::Entidad { cuerpo: Some(pb::entidad::Cuerpo::Asentamiento(datos)) }),
        reemplaza,
        campos,
        ..Default::default()
    };
    match (asentamiento(n, id), antes) {
        (None, _) => (Some(super::cambio_quita(pb::TipoEntidad::Asentamiento, ids::asentamiento(id))), None),
        (Some(ahora), None) => (Some(cambio(ahora.clone(), true, Vec::new())), Some(ahora)),
        (Some(ahora), Some(antes)) => (mascara(antes, &ahora).map(|(campos, datos)| cambio(datos, false, campos)), Some(ahora)),
    }
}

/// El campamento de mercenarios `c` del artefacto, completo, o `None` si no existe. Los reclutas son un ritmo (crecen solos hasta el tope de sus viviendas), el stock y los contadores del mercado
/// son lo que hay ahora; lo que cada héroe lleva comprado hoy (cupo) y su almacén personal van en su `Heroe` (`HEROE`).
pub fn campamento(n: &Nucleo, c: u32) -> Option<pb::CampamentoMercenarios> {
    let e = n.estado();
    let (mr, cm) = (&e.reglas.economia.militar, &e.reglas.economia.comercio);
    let (puerta, origen) = (n.puerta_de_campamento(c)?, n.origen_de_campamento(c)?);
    let reclutas = n.reclutas_de(c);
    let stock = n.stock_de_campamento(c)?;
    let mercado = &e.comercio.mercenarios.mercado;
    let militar = n.edificio_militar(c)?.nombre().to_owned();
    let residente = |h: &crate::estado_militar::HeroeNucleo| h.heroe.residencia == Some(militar::heroe::Residencia::Campamento(c));
    let prestamos = n.heroes().flat_map(|h| h.heroe.escuadras.iter()).filter(|s| s.prestada == Some(c)).map(|s| ids::escuadra(s.id));
    // Los que están dentro ahora: residentes y visitantes.
    let dentro = |h: &crate::estado_militar::HeroeNucleo| h.heroe.presencia.ubicacion == Ubicacion::Mercenarios(c);
    Some(pb::CampamentoMercenarios {
        id: ids::campamento(c),
        version: n.version_de(Clase::Campamento, u64::from(c)),
        posicion: Some(punto(puerta)),
        origen,
        edificios: std::iter::once(militar).chain(std::iter::repeat_n("vivienda".to_owned(), mr.mercenarios.viviendas)).collect(),
        residentes_ids: n.heroes().filter(|h| residente(h)).map(|h| ids::heroe(h.heroe.id)).collect(),
        poblacion: Some(pb::Ritmo {
            valor_en_t0: reclutas.poblacion,
            t0_ms: reclutas.en,
            por_minuto: mr.mercenarios.poblacion_por_hora * 1_000 / 60,
            maximo: Some(militar::mercenarios::tope(mr, mr.mercenarios.viviendas)),
            minimo: Some(0),
            ..Default::default()
        }),
        mercado: Some(cantidades(stock.stock.iter().map(|(&r, &v)| (r, v)))),
        contadores_mercado: mercado.contadores.iter().map(|(r, &v)| (r.nombre().to_owned(), u32::try_from(v).unwrap_or(u32::MAX))).collect(),
        repone_en_ms: mercado.repone_en,
        presentes_ids: n.heroes().filter(|h| dentro(h)).map(|h| ids::heroe(h.heroe.id)).collect(),
        prestamos_ids: prestamos.collect(),
        tropas_ofrecidas: n.tropas_del_campamento(c)?.into_iter().map(|t| t.nombre().to_owned()).collect(),
        cupo_diario: Some(cantidades(cm.mercenarios.cupo_diario.iter().map(|(&r, &v)| (r, v)))),
        ..Default::default()
    })
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn la_mascara_lleva_solo_lo_que_cambia_y_aplicarla_da_la_plaza_nueva() {
        let antes = pb::Asentamiento { id: "as-1".into(), version: 3, version_interior: 7, nivel: 2, edificios: vec![pb::Edificio { id: "ed-1-0".into(), ..Default::default() }], batalla_id: Some("b".into()), ..Default::default() };
        // Cambia un escalar, se borra un opcional y un campo vuelve a su valor por defecto.
        let ahora = pb::Asentamiento { id: "as-1".into(), version: 4, version_interior: 8, nivel: 3, edificios: vec![pb::Edificio { id: "ed-1-0".into(), ..Default::default() }], batalla_id: None, nivel_actual: 0, ..Default::default() };
        let (campos, datos) = mascara(&antes, &ahora).expect("hay cambios");
        assert_eq!(campos, ["version", "version_interior", "nivel", "batalla_id"]);
        assert!(datos.edificios.is_empty(), "lo que no cambia no viaja");
        let mut cliente = antes.clone();
        aplicar(&mut cliente, &datos, &campos);
        assert_eq!(cliente, ahora);
        assert_eq!((cliente.version, cliente.version_interior), (4, 8));
        // Sin cambios no hay máscara, aunque las versiones sean otras.
        assert_eq!(mascara(&ahora, &pb::Asentamiento { version: 9, version_interior: 12, ..ahora.clone() }), None);
        // La máscara de todo lo copia todo.
        let mut vacio = pb::Asentamiento::default();
        aplicar(&mut vacio, &ahora, &todos_los_campos());
        assert_eq!(vacio.nivel, 3);
        assert_eq!((vacio.version, vacio.batalla_id), (4, None));
    }
}
