//! La foto de los mercenarios por contrato (BAN54, entrada 48): las ofertas, los contratos, las fichas, las esperas, lo que se puede valorar y las marcas de ciudadanía dan la vuelta.

use mercenarios::{Ambito, Contrato, Ficha, Medalla, Objetivo, Oferta, Orden, Puesto, Terminado, TipoOrden, Trabajo};
use nucleo::{Estado, Reglas};
use politica::Marca;

#[test]
fn los_contratos_y_las_marcas_dan_la_vuelta_por_la_foto() {
    let reglas = Reglas::default();
    let mut s = Estado::nuevo(reglas.clone(), "h".into(), 1, 0);
    let rp = s.reglas.economia.politica.clone();
    s.politica.facciones.crear(&rp, 1, 10, "Micenas", None, 0).unwrap();
    s.politica.facciones.otorgar_marcado(1, 20, Marca::Permanente).unwrap();
    s.politica.facciones.otorgar_marcado(1, 21, Marca::Temporal).unwrap();
    s.contratos.ofertas.insert(30, Oferta { campamento: 2, desde: 5 });
    let orden = Orden { tipo: TipoOrden::Guardar, objetivo: Objetivo::Plaza(4), dada_por: 10, puesto: Some(Puesto::Rey), dada_en: 100, vence: None, contra: None, unido: false };
    s.contratos.contratos.insert(
        20,
        Contrato {
            mercenario: 20,
            faccion: 1,
            ambito: Ambito::Faccion,
            trabajo: Trabajo::Permanente,
            objetivo: Objetivo::Faccion(1),
            plazo: None,
            firmado_por: 10,
            puesto_firma: Puesto::Rey,
            firmado_en: 100,
            vence: None,
            campamento_base: 2,
            orden: Some(orden),
            fin_aplazado: None,
            renombre: 0,
        },
    );
    let mut ficha = Ficha::nueva();
    ficha.contadores.insert(Medalla::Leal, 7);
    s.contratos.fichas.insert(20, ficha);
    s.contratos.esperas.insert((22, 1), 9_000);
    s.contratos.terminados.insert((22, 1), Terminado { firmado_por: 10, renombre: 25, palabra: true, valorable_hasta: 9_000, valorado: false });

    let bytes = s.serializar();
    assert_eq!(&bytes[..5], b"BAN57");
    let vuelta = Estado::deserializar(&bytes, &reglas, "h").unwrap();
    assert_eq!(vuelta.contratos, s.contratos);
    assert_eq!(vuelta.politica, s.politica);
    assert_eq!(vuelta.politica.facciones.marca_de(20), Some(Marca::Permanente));
    assert_eq!(vuelta.serializar(), bytes);
}
