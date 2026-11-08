//! Los mercenarios por contrato en el contrato 05 (Mercenarios_Por_Contrato §10): de los tipos del crate `mercenarios` y del estado de los contratos a los mensajes del proto. Qué ámbito recibe cada
//! cosa lo decide [`super::audiencia`] (y `sesiones/ambitos.rs` para los derechos); aquí solo se dice **qué es**.
//!
//! | Mensaje | Dónde |
//! |---|---|
//! | `Heroe.contrato` | solo en el `HEROE` del propio mercenario (y en `ADMIN`) |
//! | `Heroe.ficha_mercenario` | entera en su `HEROE`; reducida (rango, renombre y grado de cada medalla) en el héroe público de la `VISTA` |
//! | `Heroe.contratado` | pública: la marca, si tiene contrato |
//! | `Faccion.contratos` | en la `FACCION` de la Facción (que un marcado no recibe) y en `ADMIN` |
//! | `CampamentoMercenarios.tablon` | en el `CAMPAMENTO` (y en `ADMIN`), nunca en la `VISTA` |

use super::ids;
use crate::estado::Estado;
use crate::nucleo::Nucleo;
use ::mercenarios::contrato::tarifa_en_monedas;
use ::mercenarios::{Ambito, Datos, Ficha, Fin, Medalla, Objetivo, Orden, Puesto, TipoOrden};
use asentamiento::Cargo;
use contrato::v1 as pb;
use politica::Marca;

/// La marca de ciudadanía como la lleva el contrato (`MiembroFaccion.marca`, `Heroe.contratado`).
pub(super) fn marca(m: Marca) -> &'static str {
    match m {
        Marca::Temporal => "temporal",
        Marca::Permanente => "permanente",
    }
}

/// A qué apunta un contrato o una orden, con los ids del contrato de cada entidad.
fn objetivo(o: &Objetivo) -> pb::ObjetivoMercenario {
    use pb::objetivo_mercenario::Tipo;
    let tipo = match *o {
        Objetivo::Plaza(id) => Tipo::AsentamientoId(ids::asentamiento(id)),
        Objetivo::Pueblo(id) => Tipo::PuebloId(ids::pueblo(id)),
        Objetivo::Avanzada(id) => Tipo::AvanzadaId(ids::avanzada(id)),
        Objetivo::Caravana(id) => Tipo::CaravanaId(ids::caravana(id)),
        Objetivo::Bandidos(id) => Tipo::CampamentoId(ids::bandidos(id)),
        Objetivo::Ejercito(id) => Tipo::EjercitoId(ids::ejercito(id)),
        Objetivo::Convocatoria(id) => Tipo::ConvocatoriaId(ids::convocatoria(id)),
        Objetivo::Faccion(id) => Tipo::FaccionId(ids::faccion(id)),
        Objetivo::Punto(p) => Tipo::Punto(super::punto(p)),
    };
    pb::ObjetivoMercenario { tipo: Some(tipo) }
}

/// El cargo con el que se firmó o se ordenó: `rey`, `embajador` o `<cargo>@as-N` (la misma clave que en `MiembroFaccion.cargos`).
fn puesto(p: Puesto) -> String {
    let local = |c: Cargo, plaza: u64| format!("{}@{}", c.clave(), ids::asentamiento(plaza));
    match p {
        Puesto::Rey => "rey".to_owned(),
        Puesto::Embajador => "embajador".to_owned(),
        Puesto::General(plaza) => local(Cargo::General, plaza),
        Puesto::Gobernador(plaza) => local(Cargo::Gobernador, plaza),
        Puesto::MaestroCaravanas(plaza) => local(Cargo::MaestroCaravanas, plaza),
    }
}

/// La orden vigente de un mercenario (§6.3).
fn orden(o: &Orden) -> pb::OrdenMercenario {
    let tipo = match o.tipo {
        TipoOrden::Guardar => "guardar",
        TipoOrden::IrA => "ir_a",
        TipoOrden::Atacar => "atacar",
        TipoOrden::Unirse => "unirse",
    };
    pb::OrdenMercenario {
        tipo: tipo.to_owned(),
        objetivo: Some(objetivo(&o.objetivo)),
        dada_por_id: ids::heroe(o.dada_por),
        puesto: o.puesto.map(puesto),
        dada_en_ms: o.dada_en,
        vence_en_ms: o.vence,
        contra_faccion_id: o.contra.map(ids::faccion),
        unido: o.unido,
    }
}

/// Por qué acabará un contrato cuando acabe la batalla del mercenario (§7).
fn fin(f: Fin) -> &'static str {
    match f {
        Fin::Plazo => "plazo",
        Fin::Despido => "despido",
        Fin::PlazaPerdida => "plaza_perdida",
        Fin::SinCiudadanos => "sin_ciudadanos",
        Fin::FaccionPerdida => "faccion_perdida",
    }
}

/// Un contrato vigente (§5): lo que lleva `Heroe.contrato` y `Faccion.contratos`.
fn contrato(c: &::mercenarios::Contrato) -> pb::Contrato {
    pb::Contrato {
        mercenario_id: ids::heroe(c.mercenario),
        faccion_id: ids::faccion(c.faccion),
        asentamiento_id: match c.ambito {
            Ambito::Plaza(plaza) => Some(ids::asentamiento(plaza)),
            Ambito::Faccion => None,
        },
        trabajo: c.trabajo.id().to_owned(),
        objetivo: Some(objetivo(&c.objetivo)),
        plazo_dias: c.plazo.map(u32::from),
        firmado_por_id: ids::heroe(c.firmado_por),
        puesto_firma: puesto(c.puesto_firma),
        firmado_en_ms: c.firmado_en,
        vence_en_ms: c.vence,
        campamento_base_id: ids::campamento(c.campamento_base),
        orden: c.orden.as_ref().map(orden),
        fin_aplazado: c.fin_aplazado.map(|f| fin(f).to_owned()),
        renombre: c.renombre,
    }
}

/// La ficha de progresión (§8 y §9). `completa` es la de su `HEROE` y la del tablón: con el renombre del siguiente rango, las ocho medallas con su cuenta y el umbral del grado siguiente, el historial
/// (cifras, nunca nombres) y los premios sin reclamar. Sin ella (`VISTA`), solo el rango, el renombre y el grado de cada medalla.
fn ficha(d: &Datos, f: &Ficha, completa: bool) -> pb::FichaMercenario {
    let medallas = Medalla::TODAS.map(|m| {
        let grado = f.grados.get(&m).copied().unwrap_or(0);
        pb::MedallaMercenario {
            id: m.id().to_owned(),
            grado: u32::from(grado),
            cuenta: if completa { f.contadores.get(&m).copied().unwrap_or(0) } else { 0 },
            // El umbral del grado siguiente al ganado; ninguno en el oro.
            siguiente_umbral: d.umbrales(m).get(usize::from(grado)).copied().filter(|_| completa),
        }
    });
    pb::FichaMercenario {
        rango: u32::from(f.rango),
        rango_id: d.rangos[usize::from(f.rango) - 1].id.clone(),
        renombre: f.renombre,
        renombre_siguiente_rango: f.falta_para_subir(d).map(|falta| f.renombre + falta).filter(|_| completa),
        medallas: medallas.into(),
        historial: completa.then_some(pb::HistorialMercenario {
            firmados: f.historial.firmados,
            terminados: f.historial.terminados,
            despidos: f.historial.despidos,
            buenas: f.historial.buenas,
            malas: f.historial.malas,
        }),
        premios: if completa { u32::from(f.premios) } else { 0 },
    }
}

/// Lo que un héroe lleva de mercenario, en un `Heroe` con solo esos campos para completarlo con `..`: la marca (`contratado`) y la ficha, si la tiene (nace al primer ofrecimiento); y, si es
/// el héroe `propio` (`HEROE` y `ADMIN`) y no el público de la `VISTA`, su contrato y la ficha entera.
pub(super) fn del_heroe(n: &Nucleo, id: u64, propio: bool) -> pb::Heroe {
    let e = n.estado();
    pb::Heroe {
        contrato: e.contratos.contratos.get(&id).filter(|_| propio).map(contrato),
        ficha_mercenario: e.contratos.fichas.get(&id).map(|f| ficha(&e.reglas.economia.contratos, f, propio)),
        contratado: e.politica.facciones.marca_de(id).map(|m| marca(m).to_owned()),
        ..Default::default()
    }
}

/// Los contratos vigentes de la Facción `f`, por héroe: el de cada uno de sus marcados.
pub(super) fn contratos_de(e: &Estado, f: u32) -> Vec<pb::Contrato> {
    let marcados = e.politica.facciones.get(f).into_iter().flat_map(|x| x.marcas.keys());
    marcados.filter_map(|m| e.contratos.contratos.get(m)).map(contrato).collect()
}

/// El tablón del campamento `c` (§3 y §10): una oferta por cada bot ofrecido en él, con el campamento, su ficha entera, su tropa (la suya, no la prestada), si está herido y lo que cobra, en oro y en monedas, por cada plazo y por el
/// permanente, que es lo que cobraría la firma de hoy ([`Nucleo::tarifa_de`]). Por orden de héroe.
pub(super) fn tablon(n: &Nucleo, c: u32) -> Vec<pb::OfertaMercenario> {
    tablon_de(n, &[c])
}

/// El tablón de los campamentos `cs` juntos, por orden de héroe: el de uno solo, o el de la región que enseña la taberna de una plaza (tanda 69 §8).
pub(super) fn tablon_de(n: &Nucleo, cs: &[u32]) -> Vec<pb::OfertaMercenario> {
    let e = n.estado();
    let plazos = e.reglas.economia.contratos.plazos.iter().map(|p| Some(p.dias)).chain([None]);
    let ofrecidos = e.contratos.ofertas.iter().filter(|(_, o)| cs.contains(&o.campamento));
    ofrecidos
        .map(|(&heroe, oferta)| {
            let h = n.heroe(heroe).expect("un ofrecido es un héroe que existe");
            let f = e.contratos.fichas.get(&heroe).expect("la ficha nace con la oferta");
            pb::OfertaMercenario {
                campamento_id: ids::campamento(oferta.campamento),
                heroe_id: ids::heroe(heroe),
                desde_ms: oferta.desde,
                ficha: Some(ficha(&e.reglas.economia.contratos, f, true)),
                tropa: h.heroe.escuadras.iter().filter(|s| s.prestada.is_none() && s.con_soldados()).map(|s| super::vista::escuadra_que_lleva(n, s)).collect(),
                herido_hasta_ms: h.heroe.herido_hasta,
                tarifas: plazos
                    .clone()
                    .map(|plazo| {
                        let oro = n.tarifa_de(heroe, plazo);
                        pb::TarifaMercenario { plazo_dias: plazo.map(u32::from), oro, monedas: tarifa_en_monedas(&e.reglas.economia.contratos, oro) }
                    })
                    .collect(),
            }
        })
        .collect()
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn la_ficha_publica_lleva_el_rango_el_renombre_y_el_grado_de_cada_medalla_y_nada_mas() {
        let d = Datos::cargar();
        let mut f = Ficha::nueva();
        f.sumar_renombre(&d, d.rangos[1].renombre + 5);
        f.contar(&d, Medalla::Muralla, d.umbrales(Medalla::Muralla)[0] + 1);
        f.historial.firmados = 4;
        f.premios = 1;

        let publica = ficha(&d, &f, false);
        assert_eq!((publica.rango, publica.rango_id.as_str(), publica.renombre), (2, "lanza", d.rangos[1].renombre + 5));
        assert_eq!((publica.renombre_siguiente_rango, publica.historial, publica.premios), (None, None, 0));
        assert_eq!(publica.medallas.len(), Medalla::TODAS.len());
        let muralla = publica.medallas.iter().find(|m| m.id == "muralla").expect("la medalla");
        assert_eq!((muralla.grado, muralla.cuenta, muralla.siguiente_umbral), (1, 0, None));

        let entera = ficha(&d, &f, true);
        assert_eq!(entera.renombre_siguiente_rango, Some(d.rangos[2].renombre));
        assert_eq!((entera.historial.map(|h| h.firmados), entera.premios), (Some(4), 1));
        let muralla = entera.medallas.iter().find(|m| m.id == "muralla").expect("la medalla");
        assert_eq!((muralla.grado, muralla.cuenta, muralla.siguiente_umbral), (1, d.umbrales(Medalla::Muralla)[0] + 1, Some(d.umbrales(Medalla::Muralla)[1])));
    }

    #[test]
    fn el_ultimo_rango_y_el_oro_de_una_medalla_no_tienen_siguiente() {
        let d = Datos::cargar();
        let mut f = Ficha::nueva();
        f.sumar_renombre(&d, d.rangos.last().expect("hay rangos").renombre);
        f.contar(&d, Medalla::Leal, d.umbrales(Medalla::Leal)[2]);
        let entera = ficha(&d, &f, true);
        let leal = entera.medallas.iter().find(|m| m.id == "leal").expect("la medalla");
        assert_eq!((entera.rango_id.as_str(), entera.renombre_siguiente_rango, leal.grado, leal.siguiente_umbral), ("senor_de_la_guerra", None, 3, None));
    }

    #[test]
    fn el_objetivo_y_el_puesto_llevan_los_ids_del_contrato() {
        use pb::objetivo_mercenario::Tipo;
        assert_eq!(objetivo(&Objetivo::Caravana(7)).tipo, Some(Tipo::CaravanaId("car-7".into())));
        assert_eq!(objetivo(&Objetivo::Bandidos(2)).tipo, Some(Tipo::CampamentoId("band-2".into())));
        assert_eq!(objetivo(&Objetivo::Faccion(3)).tipo, Some(Tipo::FaccionId("f-3".into())));
        assert_eq!((puesto(Puesto::Rey), puesto(Puesto::Gobernador(4)), puesto(Puesto::MaestroCaravanas(4))), ("rey".into(), "gobernador@as-4".into(), "maestroCaravanas@as-4".into()));
    }
}
