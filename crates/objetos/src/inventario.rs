//! La mochila del héroe y su almacén de objetos (D22): el mismo tipo, con `casillasMochila` (72) y `casillasAlmacen` (144) casillas de número fijo.
//! Lo apilable va junto en una sola casilla, sin tope; cada pieza que no se apila ocupa la suya (Objetos_Del_Heroe §3.1, punto 3). Meter y sacar no
//! cambian nada si fallan.

use crate::datos::{Catalogo, Cifras};
use std::collections::BTreeMap;

/// Una pieza: el id del catálogo y, solo si es forjada, su id propio y sus cifras (D13). La de tienda o botín es la ficha al 100 %.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pieza {
    pub objeto: String,
    pub forjada: Option<Forjada>,
}

impl Pieza {
    /// La pieza del catálogo, sin forjar.
    pub fn del_catalogo(objeto: &str) -> Pieza {
        Pieza { objeto: objeto.to_owned(), forjada: None }
    }
}

/// Lo propio de una pieza forjada: su número (`obj-{id}`, del contador del estado) y sus cifras ya multiplicadas por la calidad.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Forjada {
    pub id: u64,
    pub cifras: Cifras,
}

/// Una casilla ocupada: una pieza y cuántas (más de 1 solo si es apilable).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Casilla {
    pub pieza: Pieza,
    pub cantidad: u32,
}

/// La mochila o el almacén de objetos: las casillas ocupadas por su número fijo, de 0 a `capacidad − 1` (`ItemInstancia.casilla_inventario` del
/// contrato), y cuántas hay. Una orden que nombra una casilla actúa siempre sobre la misma pieza: sacar no mueve las demás.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inventario {
    pub capacidad: u32,
    pub casillas: BTreeMap<u32, Casilla>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorInventario {
    /// Una cantidad de 0, o más de una de una pieza forjada.
    Cantidad,
    /// El objeto no está en el catálogo.
    Desconocido,
    /// No quedan casillas libres para lo que llega (mochila o almacén llenos).
    NoCabe,
    /// La casilla está vacía o pasa de la capacidad.
    SinCasilla,
    /// La casilla tiene menos de lo que se pide.
    PocaCantidad,
}

impl Inventario {
    pub fn vacio(capacidad: u32) -> Inventario {
        Inventario { capacidad, casillas: BTreeMap::new() }
    }

    /// La casilla de la pila de ese objeto, si la hay.
    pub(crate) fn pila(&self, objeto: &str) -> Option<u32> {
        self.casillas.iter().find(|(_, c)| c.pieza.objeto == objeto).map(|(n, _)| *n)
    }

    /// Mete `cantidad` de la pieza: lo apilable se suma a su pila o, si no la hay, ocupa la casilla libre de número más bajo; lo que no se apila, una
    /// casilla libre por pieza, de la más baja hacia arriba. Si no cabe todo, no mete nada.
    pub fn meter(&mut self, cat: &Catalogo, pieza: Pieza, cantidad: u32) -> Result<(), ErrorInventario> {
        if cantidad == 0 || (pieza.forjada.is_some() && cantidad > 1) {
            return Err(ErrorInventario::Cantidad);
        }
        let ficha = cat.objetos.get(&pieza.objeto).ok_or(ErrorInventario::Desconocido)?;
        if ficha.apilable {
            if let Some(n) = self.pila(&pieza.objeto) {
                let c = self.casillas.get_mut(&n).expect("la pila está");
                c.cantidad = c.cantidad.checked_add(cantidad).ok_or(ErrorInventario::Cantidad)?;
                return Ok(());
            }
        }
        let (piezas, por_casilla) = if ficha.apilable { (1, cantidad) } else { (cantidad, 1) };
        let libres: Vec<u32> = (0..self.capacidad).filter(|n| !self.casillas.contains_key(n)).take(piezas as usize).collect();
        if libres.len() < piezas as usize {
            return Err(ErrorInventario::NoCabe);
        }
        for n in libres {
            self.casillas.insert(n, Casilla { pieza: pieza.clone(), cantidad: por_casilla });
        }
        Ok(())
    }

    /// Saca `cantidad` de la casilla `casilla` y devuelve la pieza. La casilla que se vacía queda libre; las demás no se mueven.
    pub fn sacar(&mut self, casilla: u32, cantidad: u32) -> Result<Pieza, ErrorInventario> {
        let c = self.casillas.get_mut(&casilla).ok_or(ErrorInventario::SinCasilla)?;
        if cantidad == 0 {
            return Err(ErrorInventario::Cantidad);
        }
        if cantidad > c.cantidad {
            return Err(ErrorInventario::PocaCantidad);
        }
        c.cantidad -= cantidad;
        let pieza = c.pieza.clone();
        if c.cantidad == 0 {
            self.casillas.remove(&casilla);
        }
        Ok(pieza)
    }

    /// Pasa `cantidad` de la casilla `casilla` a `destino`: de la mochila al almacén de objetos o al revés (D22). Sale como en [`Inventario::sacar`] y entra como
    /// en [`Inventario::meter`]; si no cabe (`NoCabe`: el destino está lleno), no mueve nada.
    pub fn pasar(&mut self, destino: &mut Inventario, cat: &Catalogo, casilla: u32, cantidad: u32) -> Result<(), ErrorInventario> {
        let mut origen = self.clone();
        let pieza = origen.sacar(casilla, cantidad)?;
        destino.meter(cat, pieza, cantidad)?;
        *self = origen;
        Ok(())
    }
}

/// Adónde fue a parar lo que llega sin pedirlo, con [`entregar`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Destino {
    Mochila,
    Almacen,
    Perdido,
}

/// Lo que llega sin pedirlo (el botín, una misión, la montura de un cofre; D17 y D22): a la mochila y, si no cabe, al almacén de objetos, aunque el
/// héroe esté fuera; si tampoco cabe allí, se pierde. Cualquier otro error de [`Inventario::meter`] (un objeto desconocido o una cantidad rara) no
/// mete nada y se devuelve.
pub fn entregar(mochila: &mut Inventario, almacen: &mut Inventario, cat: &Catalogo, pieza: Pieza, cantidad: u32) -> Result<Destino, ErrorInventario> {
    match mochila.meter(cat, pieza.clone(), cantidad) {
        Err(ErrorInventario::NoCabe) => {}
        r => return r.map(|()| Destino::Mochila),
    }
    match almacen.meter(cat, pieza, cantidad) {
        Err(ErrorInventario::NoCabe) => Ok(Destino::Perdido),
        r => r.map(|()| Destino::Almacen),
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::datos::{Atributos, PorDano};

    fn forjada(id: u64) -> Pieza {
        let cero = PorDano { perforante: 0, cortante: 0, contundente: 0 };
        Pieza { objeto: "dagas_cobre".into(), forjada: Some(Forjada { id, cifras: Cifras::Arma { atributos: Atributos::default(), dano: cero, penetracion: cero } }) }
    }

    #[test]
    fn lo_apilable_va_junto_y_cada_pieza_en_su_casilla() {
        let cat = Catalogo::default();
        let mut m = Inventario::vacio(cat.casillas_mochila);
        m.meter(&cat, Pieza::del_catalogo("crisol"), 3).unwrap();
        m.meter(&cat, Pieza::del_catalogo("crisol"), 2).unwrap();
        m.meter(&cat, Pieza::del_catalogo("dagas_inicio"), 2).unwrap();
        m.meter(&cat, forjada(1), 1).unwrap();
        assert_eq!(m.casillas.iter().map(|(n, c)| (*n, c.pieza.objeto.as_str(), c.cantidad)).collect::<Vec<_>>(), vec![(0, "crisol", 5), (1, "dagas_inicio", 1), (2, "dagas_inicio", 1), (3, "dagas_cobre", 1)]);
        m.sacar(1, 1).unwrap();
        m.meter(&cat, Pieza::del_catalogo("plano_2"), 1).unwrap();
        assert_eq!(m.casillas[&1].pieza.objeto, "plano_2", "ocupa la libre más baja");
        assert_eq!(Inventario::vacio(cat.casillas_almacen).capacidad, 144);
    }

    #[test]
    fn meter_rechaza_lo_que_no_cabe_sin_meter_nada() {
        let cat = Catalogo::default();
        let mut m = Inventario::vacio(2);
        m.meter(&cat, Pieza::del_catalogo("crisol"), 1).unwrap();
        assert_eq!(m.meter(&cat, Pieza::del_catalogo("dagas_inicio"), 2), Err(ErrorInventario::NoCabe));
        assert_eq!(m.casillas.len(), 1, "no mete la primera si no caben las dos");
        m.meter(&cat, Pieza::del_catalogo("plano_2"), 1).unwrap();
        assert_eq!(m.meter(&cat, Pieza::del_catalogo("plano_3"), 1), Err(ErrorInventario::NoCabe));
        assert_eq!(m.meter(&cat, Pieza::del_catalogo("crisol"), 9), Ok(()), "a una pila que ya está siempre cabe");
    }

    #[test]
    fn meter_rechaza_lo_desconocido_y_las_cantidades_raras() {
        let cat = Catalogo::default();
        let mut m = Inventario::vacio(10);
        assert_eq!(m.meter(&cat, Pieza::del_catalogo("espada_de_madera"), 1), Err(ErrorInventario::Desconocido));
        assert_eq!(m.meter(&cat, Pieza::del_catalogo("crisol"), 0), Err(ErrorInventario::Cantidad));
        assert_eq!(m.meter(&cat, forjada(1), 2), Err(ErrorInventario::Cantidad), "una pieza forjada es única");
        m.meter(&cat, Pieza::del_catalogo("crisol"), u32::MAX).unwrap();
        assert_eq!(m.meter(&cat, Pieza::del_catalogo("crisol"), 1), Err(ErrorInventario::Cantidad));
        assert_eq!(m.casillas[&0].cantidad, u32::MAX);
    }

    #[test]
    fn sacar_quita_de_la_casilla_y_la_vaciada_queda_libre_sin_mover_las_demas() {
        let cat = Catalogo::default();
        let mut m = Inventario::vacio(10);
        m.meter(&cat, Pieza::del_catalogo("crisol"), 3).unwrap();
        m.meter(&cat, Pieza::del_catalogo("dagas_inicio"), 1).unwrap();
        assert_eq!(m.sacar(0, 2), Ok(Pieza::del_catalogo("crisol")));
        assert_eq!(m.casillas[&0].cantidad, 1);
        assert_eq!(m.sacar(0, 1), Ok(Pieza::del_catalogo("crisol")));
        assert_eq!(m.casillas.keys().copied().collect::<Vec<_>>(), vec![1]);
        assert_eq!(m.casillas[&1].pieza.objeto, "dagas_inicio", "las demás no se mueven");
        assert_eq!(m.sacar(0, 1), Err(ErrorInventario::SinCasilla), "una casilla vacía");
        assert_eq!(m.sacar(10, 1), Err(ErrorInventario::SinCasilla), "fuera de la capacidad");
        assert_eq!(m.sacar(1, 2), Err(ErrorInventario::PocaCantidad));
        assert_eq!(m.sacar(1, 0), Err(ErrorInventario::Cantidad));
        assert_eq!(m.casillas.len(), 1);
    }

    #[test]
    fn pasar_saca_de_una_casilla_y_mete_en_el_otro_o_no_mueve_nada() {
        let cat = Catalogo::default();
        let (mut mochila, mut almacen) = (Inventario::vacio(3), Inventario::vacio(1));
        mochila.meter(&cat, Pieza::del_catalogo("crisol"), 5).unwrap();
        mochila.meter(&cat, Pieza::del_catalogo("dagas_inicio"), 1).unwrap();
        mochila.pasar(&mut almacen, &cat, 0, 2).unwrap();
        assert_eq!((mochila.casillas[&0].cantidad, almacen.casillas[&0].pieza.objeto.as_str(), almacen.casillas[&0].cantidad), (3, "crisol", 2));
        mochila.pasar(&mut almacen, &cat, 0, 3).unwrap();
        assert_eq!((mochila.casillas.keys().copied().collect::<Vec<_>>(), almacen.casillas[&0].cantidad), (vec![1], 5), "se apila con lo que ya había y la casilla vaciada queda libre");
        let antes = (mochila.clone(), almacen.clone());
        assert_eq!(mochila.pasar(&mut almacen, &cat, 1, 1), Err(ErrorInventario::NoCabe), "el almacén está lleno");
        assert_eq!(mochila.pasar(&mut almacen, &cat, 0, 1), Err(ErrorInventario::SinCasilla));
        assert_eq!(mochila.pasar(&mut almacen, &cat, 1, 2), Err(ErrorInventario::PocaCantidad));
        assert_eq!((mochila, almacen), antes, "un rechazo no mueve nada");
    }

    #[test]
    fn entregar_va_a_la_mochila_luego_al_almacen_y_si_no_se_pierde() {
        let cat = Catalogo::default();
        let (mut mochila, mut almacen) = (Inventario::vacio(1), Inventario::vacio(1));
        let mut dar = |objeto: &str, cantidad| entregar(&mut mochila, &mut almacen, &cat, Pieza::del_catalogo(objeto), cantidad);
        assert_eq!(dar("crisol", 2), Ok(Destino::Mochila));
        assert_eq!(dar("crisol", 1), Ok(Destino::Mochila), "a su pila");
        assert_eq!(dar("plano_2", 1), Ok(Destino::Almacen), "la mochila está llena");
        assert_eq!(dar("plano_3", 1), Ok(Destino::Perdido), "los dos están llenos");
        assert_eq!(dar("crisol", 0), Err(ErrorInventario::Cantidad));
        assert_eq!(dar("espada_de_madera", 1), Err(ErrorInventario::Desconocido));
        assert_eq!((mochila.casillas[&0].cantidad, almacen.casillas[&0].pieza.objeto.as_str()), (3, "plano_2"));
    }
}
