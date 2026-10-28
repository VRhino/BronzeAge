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
//! Del sistema 5 (5E y 5H) viajan también los cargos, los fundadores, la puerta con sus vetos, si abre su almacén a los aliados y cuándo creó su última caravana.
//!
//! DUDA: el trazado de una pieza va en milésimas de unidad local como la del ticket (`Edificio.posicion` = centro de
//! su huella); la prioridad de un edificio en cola es su puntuación, recortada a 32 bits; la fuente de una Leñera se nombra `parcela-<n>`; de los edificios fijos de un campamento (taberna, mercado...) el motor solo conoce sus viviendas y su edificio militar; las celdas de cada recinto van enteras con su
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

/// El grupo de la puerta en el contrato.
fn grupo_de_puerta(g: politica::Grupo) -> pb::GrupoPuerta {
    match g {
        politica::Grupo::Neutrales => pb::GrupoPuerta::GrupoNeutrales,
        politica::Grupo::Aliados => pb::GrupoPuerta::GrupoAliados,
        politica::Grupo::Enemigos => pb::GrupoPuerta::GrupoEnemigos,
        politica::Grupo::Aedas => pb::GrupoPuerta::GrupoAedas,
    }
}

/// Los titulares de los cargos locales de una plaza (5E): viajan en su interior y, desde fuera, en la ficha de las plazas propias (`FACCION`).
pub(crate) fn cargos(c: &politica::Cargos) -> pb::Cargos {
    use asentamiento::Cargo::*;
    let t = |x| c.titular(x).map(ids::heroe);
    pb::Cargos { gobernador_id: t(Gobernador), tesorero_id: t(Tesorero), general_id: t(General), maestro_obras_id: t(MaestroObras), sacerdote_id: t(Sacerdote) }
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
    p.almacen = TODOS.iter().map(|&r| (r.nombre().to_owned(), existencia(&eco.almacen, r, Some(t)))).collect();
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
                semilla_saturada: pieza.is_some_and(|x| x.semilla_saturada),
                ancla_llena: pieza.is_some_and(|x| x.ancla_llena),
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
            mejoradas: r.mejora.map(|m| (m.avance + 1).max(0) as u32),
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
    p.puerta_cerrada_a = n.puerta_cerrada_a(id).unwrap_or_default().iter().map(|g| grupo_de_puerta(g) as i32).collect();
    p.vetados_ids = a.puerta.vetados.iter().map(|&h| ids::heroe(h)).collect();
    p.cargos = Some(cargos(&a.cargos));
    p.fundadores_ids = a.fundadores.iter().map(|&h| ids::heroe(h)).collect();
    // Si abre su almacén a los aliados (5D) y cuándo creó su última caravana (el cooldown de crearlas, compartido con la Caravana de Fundación), desde 5H.
    p.permite_reabastecer_aliados = a.permite_reabastecer_aliados;
    p.ultima_caravana_creada_en_ms = e.comercio.ultima_caravana.get(&id).copied();
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
    p.bandidos_reaparecen_en_ms = e.combate.reaparecen.get(&id).copied();
    p.auto_construccion_pausada = eco.auto_pausada;
    p.recetas_pausadas = eco.recetas_pausadas.iter().map(|r| r.nombre().to_owned()).collect();
    p.reserva_manual = Some(cantidades(TODOS.iter().copied().zip(eco.reserva_manual).filter(|&(_, v)| v != 0)));
    p.misiones = super::propios::misiones(n, crate::misiones::Dueno::Asentamiento(id));
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
    let planta = n.planta_de_campamento(c)?;
    let pieza = |x: &crate::urbanismo::PiezaVista| pb::Edificio {
        id: format!("{}-{}", ids::campamento(c), x.id),
        tipo: x.tipo.nombre().to_owned(),
        posicion: Some(punto(x.centro)),
        estado: pb::EstadoEdificio::EdificioActivo as i32,
        ambito: pb::AmbitoEdificio::Asentamiento as i32,
        rotado: x.rotado,
        ancho: x.rect.ancho.max(0) as u32,
        alto: x.rect.alto.max(0) as u32,
        ..Default::default()
    };
    Some(pb::CampamentoMercenarios {
        planta: planta.vista.piezas.iter().map(pieza).collect(),
        trazado: Some(pb::TrazadoUrbano {
            layout_version: trazado::LAYOUT_VERSION,
            unidades_por_celda: trazado::CELDA_UL as u32,
            radio_ul: planta.vista.radio_ul,
            calles: planta.vista.calles.iter().map(rect).collect(),
            caminos: planta.vista.caminos.iter().map(rect).collect(),
        }),
        empalizada: planta.muro.iter().map(rect).collect(),
        empalizada_puerta: planta.puerta.iter().map(rect).collect(),
        id: ids::campamento(c),
        version: n.version_de(Clase::Campamento, u64::from(c)),
        posicion: Some(punto(puerta)),
        origen,
        // Los fijos y el militar, en el orden del TypeScript (`[...edificiosFijos, militar]`: taberna, las viviendas y el mercado; paridad P7).
        edificios: std::iter::once("taberna".to_owned())
            .chain(std::iter::repeat_n("vivienda".to_owned(), mr.mercenarios.viviendas))
            .chain(["mercado".to_owned(), militar])
            .collect(),
        residentes_ids: n.heroes().filter(|h| residente(h)).map(|h| ids::heroe(h.heroe.id)).collect(),
        eligieron_como_inicial: n.eligieron_como_inicial(c),
        bandidos_en_ms: e.combate.anillos.get(&c).copied(),
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

/// El campamento como lo ve quien está dentro (`CAMPAMENTO`) y la administración: el de [`campamento`] con el fondo de refundación por Facción (lo de sus ciudadanos, §9.1; el contrato lo lleva
/// por Facción, `f-N`). Cada sesión ve **solo el de su propia Facción**; la administración, el de todas (decisión del autor 5G-b-7). La `VISTA` no lo lleva: es lo que hay dentro.
pub fn campamento_por_dentro(n: &Nucleo, c: u32, quien: super::propios::Espectador) -> Option<pb::CampamentoMercenarios> {
    let mut p = campamento(n, c)?;
    let suya = match quien {
        super::propios::Espectador::Admin => None,
        super::propios::Espectador::Heroe(h) => Some(h.and_then(|h| n.heroe(h)).and_then(|h| h.faccion)),
    };
    let facciones: std::collections::BTreeSet<u32> =
        n.estado().fundacion.fondos.get(&c).into_iter().flatten().filter_map(|(h, _)| n.heroe(*h).and_then(|h| h.faccion)).filter(|f| suya.is_none_or(|s| s == Some(*f))).collect();
    p.fondos = facciones.into_iter().map(|f| (ids::faccion(f), n.fondo_de_faccion(c, f))).filter(|(_, m)| !m.is_empty()).map(|(f, m)| (f, cantidades(m))).collect();
    if let super::propios::Espectador::Heroe(h) = quien {
        (p.precios_mercado, p.reclutables) = lo_que_cobra(n, c, h);
    }
    Some(p)
}

/// Lo que el campamento `c` cobra a quien está dentro (2026-10-27): el precio de cada bien de su mercado (el de `comprar_en_campamento`, igual para todos) y, si `heroe` reside en él, cada
/// tropa de su edificio militar con su precio para él y si está desbloqueada. Cambia sin que cambie la versión (el stock de las plazas, la reputación): la sesión lo compara en cada reparto.
pub fn lo_que_cobra(n: &Nucleo, c: u32, heroe: Option<u64>) -> (std::collections::BTreeMap<String, i64>, Vec<pb::TropaDeCampamento>) {
    let e = n.estado();
    let (cr, mr) = (&e.reglas.economia.comercio, &e.reglas.economia.militar);
    let Some(stock) = n.stock_de_campamento(c) else { return Default::default() };
    let tabla = n.tabla_de_precios();
    let precios = stock.stock.keys().filter_map(|&r| Some((r.nombre().to_owned(), comercio::mercenarios::precio_unitario(&cr.mercenarios, tabla.precio(r))?))).collect();
    let residente = heroe.filter(|&h| n.heroe(h).is_some_and(|x| x.heroe.residencia == Some(militar::heroe::Residencia::Campamento(c))));
    let reclutables = residente
        .and_then(|h| {
            let (factor, sin_plazas) = n.condiciones_de_recluta(h, n.ahora());
            let tropas = n.tropas_del_edificio_de_campamento(c)?;
            Some(
                tropas
                    .into_iter()
                    .map(|(t, desbloqueada)| {
                        let d = mr.tropas.tropa(t);
                        pb::TropaDeCampamento {
                            tropa_id: t.nombre().to_owned(),
                            desbloqueada,
                            precio_por_soldado: militar::mercenarios::precio_por_soldado(mr, d, factor, sin_plazas),
                            precio_escuadra: militar::mercenarios::precio(mr, d, d.unidades, factor, sin_plazas),
                            unidades: u32::try_from(d.unidades.max(0)).unwrap_or(u32::MAX),
                        }
                    })
                    .collect(),
            )
        })
        .unwrap_or_default();
    (precios, reclutables)
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
