//! El equipo del héroe: siete huecos, con el arma siempre puesta (Clases_De_Heroe §4). El arma define la clase, y la clase, los atributos base y los
//! pesos de armadura que admite. Equipar y quitar trabajan sobre la mochila y no cambian nada si se rechazan (D17: con la mochila llena, antes de
//! mover nada). Dónde se puede equipar (D2) lo comprueba quien llama.

use crate::datos::{Atributos, Catalogo, Clase, Hueco, Tipo};
use crate::inventario::{ErrorInventario, Inventario, Pieza};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Equipo {
    pub arma: Pieza,
    pub casco: Option<Pieza>,
    pub torso: Option<Pieza>,
    pub guantes: Option<Pieza>,
    pub pantalones: Option<Pieza>,
    pub botas: Option<Pieza>,
    pub montura: Option<Pieza>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorEquipo {
    /// Al sacar de la mochila o al meter en ella lo que se quita (`NoCabe`: mochila llena).
    Inventario(ErrorInventario),
    /// La clase no existe (`heroe.clase_desconocida`, Clases_De_Heroe §6).
    ClaseDesconocida,
    /// La pieza (la de la casilla o una puesta) no está en el catálogo.
    Desconocido,
    /// Un material o un consumible: no va en ningún hueco.
    NoEquipable,
    /// La clase del arma puesta no admite el peso de esa armadura.
    Incompatible,
    /// El arma no se quita: solo se cambia por otra.
    ArmaFija,
    /// No hay nada puesto en ese hueco.
    HuecoVacio,
}

impl From<ErrorInventario> for ErrorEquipo {
    fn from(e: ErrorInventario) -> Self {
        ErrorEquipo::Inventario(e)
    }
}

impl Equipo {
    /// El equipo con el que nace un héroe de esa clase (Clases_De_Heroe §6): el arma de inicio, el juego de inicio de su peso sin casco y la montura de tier I.
    pub fn inicial(cat: &Catalogo, clase: &str) -> Result<Equipo, ErrorEquipo> {
        cat.clases.get(clase).map(|c| c.inicial.clone()).ok_or(ErrorEquipo::ClaseDesconocida)
    }

    /// La clase del héroe: la del arma puesta (`claseDe`, Clases_De_Heroe §4).
    pub fn clase<'a>(&self, cat: &'a Catalogo) -> Result<&'a str, ErrorEquipo> {
        match cat.objetos.get(&self.arma.objeto).map(|f| &f.tipo) {
            Some(Tipo::Arma { clase, .. }) => Ok(clase),
            _ => Err(ErrorEquipo::Desconocido),
        }
    }

    fn ficha_de_clase<'a>(&self, cat: &'a Catalogo) -> Result<&'a Clase, ErrorEquipo> {
        let id = self.clase(cat)?;
        Ok(cat.clases.get(id).expect("toda arma nombra una clase que existe (comprobado al cargar)"))
    }

    /// Los atributos del héroe (Clases_De_Heroe §5): la base de la clase del arma más los puntos repartidos.
    pub fn atributos(&self, cat: &Catalogo, puntos: Atributos) -> Result<Atributos, ErrorEquipo> {
        Ok(self.ficha_de_clase(cat)?.atributos_base.mas(puntos))
    }

    /// El hueco de una armadura o de la montura; el arma no está aquí, porque nunca queda vacía.
    fn ranura(&mut self, h: Hueco) -> Option<&mut Option<Pieza>> {
        match h {
            Hueco::Arma => None,
            Hueco::Casco => Some(&mut self.casco),
            Hueco::Torso => Some(&mut self.torso),
            Hueco::Guantes => Some(&mut self.guantes),
            Hueco::Pantalones => Some(&mut self.pantalones),
            Hueco::Botas => Some(&mut self.botas),
            Hueco::Montura => Some(&mut self.montura),
        }
    }

    /// `equipar` desde la casilla `casilla` de la mochila (Clases_De_Heroe §4). Lo que estaba en el hueco va a la mochila. Un arma cambia la clase: las
    /// armaduras puestas cuyo peso no admite la nueva van a la mochila. Una armadura se rechaza si la clase del arma puesta no admite su peso.
    pub fn equipar(&mut self, mochila: &mut Inventario, cat: &Catalogo, casilla: u32) -> Result<(), ErrorEquipo> {
        let (mut e, mut m) = (self.clone(), mochila.clone());
        let pieza = m.sacar(casilla, 1)?;
        let ficha = cat.objetos.get(&pieza.objeto).ok_or(ErrorEquipo::Desconocido)?;
        match &ficha.tipo {
            Tipo::Arma { clase, .. } => {
                let admite = &cat.clases.get(clase).expect("toda arma nombra una clase que existe (comprobado al cargar)").armaduras;
                m.meter(cat, std::mem::replace(&mut e.arma, pieza), 1)?;
                for h in Hueco::ARMADURA {
                    let r = e.ranura(h).expect("hueco de armadura");
                    if let Some(p) = r.take() {
                        let peso = match cat.objetos.get(&p.objeto).map(|f| &f.tipo) {
                            Some(Tipo::Armadura { peso, .. }) => *peso,
                            _ => return Err(ErrorEquipo::Desconocido),
                        };
                        if admite.contains(&peso) {
                            *r = Some(p);
                        } else {
                            m.meter(cat, p, 1)?;
                        }
                    }
                }
            }
            Tipo::Armadura { hueco, peso, .. } => {
                if !e.ficha_de_clase(cat)?.armaduras.contains(peso) {
                    return Err(ErrorEquipo::Incompatible);
                }
                if let Some(vieja) = e.ranura(*hueco).expect("hueco de armadura").replace(pieza) {
                    m.meter(cat, vieja, 1)?;
                }
            }
            Tipo::Montura => {
                if let Some(vieja) = e.montura.replace(pieza) {
                    m.meter(cat, vieja, 1)?;
                }
            }
            Tipo::Material | Tipo::Consumible(_) => return Err(ErrorEquipo::NoEquipable),
        }
        (*self, *mochila) = (e, m);
        Ok(())
    }

    /// `desequipar`: lo puesto en ese hueco va a la mochila. El arma no se quita (Clases_De_Heroe §4).
    pub fn quitar(&mut self, mochila: &mut Inventario, cat: &Catalogo, hueco: Hueco) -> Result<(), ErrorEquipo> {
        let r = self.ranura(hueco).ok_or(ErrorEquipo::ArmaFija)?;
        let p = r.as_ref().ok_or(ErrorEquipo::HuecoVacio)?;
        mochila.meter(cat, p.clone(), 1)?;
        *r = None;
        Ok(())
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn con(cat: &Catalogo, objetos: &[&str]) -> Inventario {
        let mut m = Inventario::vacio(cat.casillas_mochila);
        for o in objetos {
            m.meter(cat, Pieza::del_catalogo(o), 1).unwrap();
        }
        m
    }

    fn objetos(m: &Inventario) -> Vec<&str> {
        m.casillas.values().map(|c| c.pieza.objeto.as_str()).collect()
    }

    #[test]
    fn el_equipo_inicial_es_el_arma_y_el_juego_de_inicio_sin_casco_y_la_montura_de_tiro() {
        let cat = Catalogo::default();
        let e = Equipo::inicial(&cat, "espada_escudo_mediano").unwrap();
        let p = |o: &str| Some(Pieza::del_catalogo(o));
        assert_eq!(
            e,
            Equipo { arma: Pieza::del_catalogo("espada_escudo_mediano_inicio"), casco: None, torso: p("torso_media_inicio"), guantes: p("guantes_media_inicio"), pantalones: p("pantalones_media_inicio"), botas: p("botas_media_inicio"), montura: p("montura_tiro") }
        );
        assert_eq!(Equipo::inicial(&cat, "dagas").unwrap().torso, p("torso_ligera_inicio"));
        assert_eq!(Equipo::inicial(&cat, "lanza_escudo_redondo").unwrap().botas, p("botas_pesada_inicio"));
        assert_eq!(Equipo::inicial(&cat, ""), Err(ErrorEquipo::ClaseDesconocida));
        assert_eq!(Equipo::inicial(&cat, "mandoble"), Err(ErrorEquipo::ClaseDesconocida));
    }

    #[test]
    fn la_clase_es_la_del_arma_y_los_atributos_su_base_mas_los_puntos() {
        let cat = Catalogo::default();
        let mut e = Equipo::inicial(&cat, "hacha_dos_manos").unwrap();
        assert_eq!(e.clase(&cat), Ok("hacha_dos_manos"));
        let puntos = Atributos { fuerza: 1, destreza: 2, armadura: 3, vitalidad: 4 };
        assert_eq!(e.atributos(&cat, puntos), Ok(Atributos { fuerza: 6, destreza: 3, armadura: 4, vitalidad: 7 }));
        e.arma = Pieza::del_catalogo("arco_lanza_bronce");
        assert_eq!(e.atributos(&cat, puntos), Ok(Atributos { fuerza: 3, destreza: 7, armadura: 4, vitalidad: 6 }), "cambiar de arma cambia la base, no los puntos");
        e.arma = Pieza::del_catalogo("torso_media_inicio");
        assert_eq!(e.clase(&cat), Err(ErrorEquipo::Desconocido));
        assert_eq!(e.atributos(&cat, puntos), Err(ErrorEquipo::Desconocido));
    }

    #[test]
    fn equipar_un_arma_cambia_la_clase_y_manda_a_la_mochila_la_vieja_y_la_armadura_que_no_admite() {
        let cat = Catalogo::default();
        let mut e = Equipo::inicial(&cat, "espada_escudo_mediano").unwrap();
        e.casco = Some(Pieza::del_catalogo("casco_pesada_cobre"));
        let mut m = con(&cat, &["crisol", "dagas_cobre"]);
        e.equipar(&mut m, &cat, 1).unwrap();
        assert_eq!(e.clase(&cat), Ok("dagas"));
        assert_eq!((e.casco.clone(), e.torso.clone(), e.montura.clone()), (None, None, Some(Pieza::del_catalogo("montura_tiro"))));
        assert_eq!(objetos(&m), vec!["crisol", "espada_escudo_mediano_inicio", "casco_pesada_cobre", "torso_media_inicio", "guantes_media_inicio", "pantalones_media_inicio", "botas_media_inicio"]);
        // De la espada (pesada y media) a la lanza (pesada): el casco pesado se queda.
        let mut e = Equipo::inicial(&cat, "espada_escudo_mediano").unwrap();
        e.casco = Some(Pieza::del_catalogo("casco_pesada_cobre"));
        let mut m = con(&cat, &["lanza_escudo_redondo_cobre"]);
        e.equipar(&mut m, &cat, 0).unwrap();
        assert_eq!((e.casco, e.torso), (Some(Pieza::del_catalogo("casco_pesada_cobre")), None));
    }

    #[test]
    fn equipar_una_armadura_o_una_montura_cambia_la_puesta_y_la_incompatible_se_rechaza() {
        let cat = Catalogo::default();
        let mut e = Equipo::inicial(&cat, "arco_lanza").unwrap();
        let mut m = con(&cat, &["torso_media_cobre", "casco_ligera_cobre", "torso_pesada_cobre", "montura_estepa"]);
        e.equipar(&mut m, &cat, 0).unwrap();
        assert_eq!(e.torso, Some(Pieza::del_catalogo("torso_media_cobre")), "el arco admite media");
        e.equipar(&mut m, &cat, 1).unwrap();
        assert_eq!(e.casco, Some(Pieza::del_catalogo("casco_ligera_cobre")), "en un hueco vacío");
        let antes = (e.clone(), m.clone());
        assert_eq!(e.equipar(&mut m, &cat, 2), Err(ErrorEquipo::Incompatible));
        assert_eq!((e.clone(), m.clone()), antes, "no cambia nada");
        e.equipar(&mut m, &cat, 3).unwrap();
        assert_eq!(e.montura, Some(Pieza::del_catalogo("montura_estepa")));
        assert_eq!(m.casillas.iter().map(|(n, c)| (*n, c.pieza.objeto.as_str())).collect::<Vec<_>>(), vec![(0, "torso_ligera_inicio"), (1, "montura_tiro"), (2, "torso_pesada_cobre")], "cada una a la casilla libre más baja");
    }

    #[test]
    fn equipar_rechaza_lo_que_no_se_equipa_la_casilla_que_no_hay_y_la_mochila_llena() {
        let cat = Catalogo::default();
        let mut e = Equipo::inicial(&cat, "espada_escudo_mediano").unwrap();
        let mut m = con(&cat, &["crisol", "medalla_heroe_grande"]);
        assert_eq!(e.equipar(&mut m, &cat, 0), Err(ErrorEquipo::NoEquipable));
        assert_eq!(e.equipar(&mut m, &cat, 1), Err(ErrorEquipo::NoEquipable));
        assert_eq!(e.equipar(&mut m, &cat, 2), Err(ErrorEquipo::Inventario(ErrorInventario::SinCasilla)));
        // Con la mochila llena el arma vieja cabe en la casilla que deja la nueva, pero las cuatro armaduras medias no.
        let mut m = Inventario::vacio(2);
        m.meter(&cat, Pieza::del_catalogo("crisol"), 1).unwrap();
        m.meter(&cat, Pieza::del_catalogo("dagas_cobre"), 1).unwrap();
        let antes = (e.clone(), m.clone());
        assert_eq!(e.equipar(&mut m, &cat, 1), Err(ErrorEquipo::Inventario(ErrorInventario::NoCabe)));
        assert_eq!((e.clone(), m.clone()), antes, "no cambia nada");
        let mut puesta = e.clone();
        puesta.torso = Some(Pieza::del_catalogo("torso_de_madera"));
        assert_eq!(puesta.equipar(&mut con(&cat, &["dagas_cobre"]), &cat, 0), Err(ErrorEquipo::Desconocido), "una pieza puesta que no está en el catálogo");
        let mut rara = Inventario::vacio(1);
        rara.casillas.insert(0, crate::inventario::Casilla { pieza: Pieza::del_catalogo("torso_de_madera"), cantidad: 1 });
        assert_eq!(e.equipar(&mut rara, &cat, 0), Err(ErrorEquipo::Desconocido), "una pieza de la mochila que no está en el catálogo");
    }

    #[test]
    fn quitar_manda_la_pieza_a_la_mochila_pero_el_arma_no_se_quita() {
        let cat = Catalogo::default();
        let mut e = Equipo::inicial(&cat, "dagas").unwrap();
        let mut m = Inventario::vacio(1);
        e.quitar(&mut m, &cat, Hueco::Montura).unwrap();
        assert_eq!((e.montura.clone(), objetos(&m)), (None, vec!["montura_tiro"]));
        assert_eq!(e.quitar(&mut m, &cat, Hueco::Torso), Err(ErrorEquipo::Inventario(ErrorInventario::NoCabe)));
        assert_eq!(e.torso, Some(Pieza::del_catalogo("torso_ligera_inicio")), "con la mochila llena se queda puesta");
        assert_eq!(e.quitar(&mut m, &cat, Hueco::Casco), Err(ErrorEquipo::HuecoVacio));
        assert_eq!(e.quitar(&mut m, &cat, Hueco::Arma), Err(ErrorEquipo::ArmaFija));
    }
}
