//! La tienda de la Armería (D1): comprar y vender con las monedas del héroe (Catalogo_De_Objetos §6). Dónde se puede comprar y vender lo comprueba
//! quien llama; si una tecnología está adoptada por la Facción de la plaza, también (`adoptada`).

use crate::datos::{Catalogo, Tipo};
use crate::inventario::{ErrorInventario, Inventario, Pieza};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorTienda {
    /// Al meter en la mochila lo comprado (`NoCabe`: mochila llena) o al sacar de ella lo vendido.
    Inventario(ErrorInventario),
    /// El objeto no está en el catálogo.
    Desconocido,
    /// La tienda no lo vende: no es arma, armadura ni montura, o su tier pasa de `tiendaHastaTier`.
    NoSeVende,
    /// La Facción de la plaza no tiene adoptada la tecnología de su metal.
    MetalCerrado,
    /// El héroe no tiene las monedas del precio.
    SinMonedas,
}

impl From<ErrorInventario> for ErrorTienda {
    fn from(e: ErrorInventario) -> Self {
        ErrorTienda::Inventario(e)
    }
}

/// `comprar`: una pieza de la ficha al 100 %, a su precio. Solo armas, armaduras y monturas hasta `tiendaHastaTier`, con el metal abierto. Con la
/// mochila llena se rechaza sin cobrar (D17).
pub fn comprar(mochila: &mut Inventario, cat: &Catalogo, objeto: &str, monedas: &mut i64, adoptada: impl Fn(&str) -> bool) -> Result<(), ErrorTienda> {
    let ficha = cat.objetos.get(objeto).ok_or(ErrorTienda::Desconocido)?;
    let tecnologia = match &ficha.tipo {
        Tipo::Arma { combate, .. } | Tipo::Armadura { combate, .. } => combate.tecnologia.as_deref(),
        Tipo::Montura => None,
        Tipo::Material | Tipo::Consumible(_) => return Err(ErrorTienda::NoSeVende),
    };
    if ficha.tier.is_none_or(|t| t > cat.tienda_hasta_tier) {
        return Err(ErrorTienda::NoSeVende);
    }
    if tecnologia.is_some_and(|t| !adoptada(t)) {
        return Err(ErrorTienda::MetalCerrado);
    }
    if *monedas < ficha.precio {
        return Err(ErrorTienda::SinMonedas);
    }
    mochila.meter(cat, Pieza::del_catalogo(objeto), 1)?;
    *monedas -= ficha.precio;
    Ok(())
}

/// `vender`: `cantidad` de la casilla `casilla`, cada una a `ventaPct` del precio de su ficha, hacia abajo; también una pieza forjada, porque la
/// calidad no cambia el precio (D18). Devuelve lo que cobra el héroe, que ya está sumado a `monedas`.
pub fn vender(mochila: &mut Inventario, cat: &Catalogo, casilla: u32, cantidad: u32, monedas: &mut i64) -> Result<i64, ErrorTienda> {
    let objeto = &mochila.casillas.get(&casilla).ok_or(ErrorInventario::SinCasilla)?.pieza.objeto;
    let ficha = cat.objetos.get(objeto).ok_or(ErrorTienda::Desconocido)?;
    let cobro = ficha.precio * cat.venta_pct / 100 * i64::from(cantidad);
    mochila.sacar(casilla, cantidad)?;
    *monedas += cobro;
    Ok(cobro)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn todo_abierto(_: &str) -> bool {
        true
    }

    #[test]
    fn comprar_cobra_el_precio_y_mete_la_pieza_al_100() {
        let cat = Catalogo::default();
        let mut m = Inventario::vacio(cat.casillas_mochila);
        let mut monedas = 1_500;
        comprar(&mut m, &cat, "torso_pesada_inicio", &mut monedas, |_| false).unwrap();
        comprar(&mut m, &cat, "montura_estepa", &mut monedas, |_| false).unwrap();
        comprar(&mut m, &cat, "casco_media_cobre", &mut monedas, |t| t == "metalurgia_cobre").unwrap();
        assert_eq!(monedas, 1_500 - 450 - 600 - cat.objetos["casco_media_cobre"].precio);
        assert_eq!(m.casillas.values().map(|c| c.pieza.clone()).collect::<Vec<_>>(), ["torso_pesada_inicio", "montura_estepa", "casco_media_cobre"].map(Pieza::del_catalogo));
    }

    #[test]
    fn comprar_rechaza_lo_que_no_vende_el_metal_cerrado_y_sin_monedas() {
        let cat = Catalogo::default();
        let mut m = Inventario::vacio(cat.casillas_mochila);
        let mut monedas = 100_000;
        assert_eq!(comprar(&mut m, &cat, "espada_de_madera", &mut monedas, todo_abierto), Err(ErrorTienda::Desconocido));
        for o in ["torso_pesada_hierro", "lanza_escudo_redondo_bronce", "montura_escita", "crisol", "plano_2", "medalla_tropa_grande", "cofre_monturas"] {
            assert_eq!(comprar(&mut m, &cat, o, &mut monedas, todo_abierto), Err(ErrorTienda::NoSeVende), "{o}");
        }
        assert_eq!(comprar(&mut m, &cat, "dagas_hierro_temprano", &mut monedas, |t| t == "metalurgia_cobre"), Err(ErrorTienda::MetalCerrado));
        let mut pocas = 399;
        assert_eq!(comprar(&mut m, &cat, "dagas_cobre", &mut pocas, todo_abierto), Err(ErrorTienda::SinMonedas));
        assert_eq!((monedas, pocas, m.casillas.len()), (100_000, 399, 0));
    }

    #[test]
    fn comprar_con_la_mochila_llena_no_cobra() {
        let cat = Catalogo::default();
        let mut m = Inventario::vacio(1);
        m.meter(&cat, Pieza::del_catalogo("crisol"), 1).unwrap();
        let mut monedas = 1_000;
        assert_eq!(comprar(&mut m, &cat, "dagas_inicio", &mut monedas, todo_abierto), Err(ErrorTienda::Inventario(ErrorInventario::NoCabe)));
        assert_eq!(monedas, 1_000);
    }

    #[test]
    fn vender_paga_la_mitad_de_la_ficha_tambien_si_es_forjada() {
        let cat = Catalogo::default();
        let mut m = Inventario::vacio(cat.casillas_mochila);
        m.meter(&cat, Pieza::del_catalogo("medalla_tropa_muy_pequena"), 5).unwrap();
        m.meter(&cat, Pieza::del_catalogo("torso_ligera_hierro"), 1).unwrap();
        let cifras = match &cat.objetos["dagas_acero"].tipo {
            Tipo::Arma { combate, .. } => combate.cifras,
            _ => unreachable!(),
        };
        m.meter(&cat, Pieza { objeto: "dagas_acero".into(), forjada: Some(crate::inventario::Forjada { id: 7, cifras }) }, 1).unwrap();
        let mut monedas = 0;
        assert_eq!(vender(&mut m, &cat, 0, 3, &mut monedas), Ok(6), "2 cada medalla de tropa muy pequeña (D19)");
        assert_eq!(vender(&mut m, &cat, 1, 1, &mut monedas), Ok(262), "525 al 50 %, hacia abajo");
        assert_eq!(vender(&mut m, &cat, 2, 1, &mut monedas), Ok(700), "la forjada sigue en su casilla");
        assert_eq!((monedas, m.casillas.len(), m.casillas[&0].cantidad), (968, 1, 2));
    }

    #[test]
    fn vender_rechaza_la_casilla_que_no_hay_y_mas_de_lo_que_tiene() {
        let cat = Catalogo::default();
        let mut m = Inventario::vacio(cat.casillas_mochila);
        m.meter(&cat, Pieza::del_catalogo("crisol"), 2).unwrap();
        let mut monedas = 0;
        assert_eq!(vender(&mut m, &cat, 1, 1, &mut monedas), Err(ErrorTienda::Inventario(ErrorInventario::SinCasilla)));
        assert_eq!(vender(&mut m, &cat, 0, 3, &mut monedas), Err(ErrorTienda::Inventario(ErrorInventario::PocaCantidad)));
        m.casillas.insert(1, crate::inventario::Casilla { pieza: Pieza::del_catalogo("espada_de_madera"), cantidad: 1 });
        assert_eq!(vender(&mut m, &cat, 1, 1, &mut monedas), Err(ErrorTienda::Desconocido));
        assert_eq!((monedas, m.casillas[&0].cantidad), (0, 2));
    }
}
