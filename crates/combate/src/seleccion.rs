//! Quién entra en una batalla con números (Combate §3.1): hay un tope de **héroes** por bando (`BATALLA.capacidad`: 15 en un asedio y en una
//! avanzada, 5 en el resto). Entran los de más nivel y, a igual nivel, los de escuadras más fuertes, y a igual poder el de id menor. El resto
//! se **aparta**: no combate ni sufre bajas, y sus escuadras vuelven intactas a su columna. Los heridos no son candidatos (Doc 5.16.4).

use crate::datos::Reglas;
use base::Mil;
use militar::escuadra::Escuadra;
use militar::heroe::Heroe;
use militar::HeroeId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Candidato {
    pub heroe: HeroeId,
    pub nivel: u32,
    /// Poder de las escuadras suyas que llevaría a la batalla.
    pub poder: Mil,
}

impl Candidato {
    pub fn de(r: &Reglas, heroe: &Heroe, escuadras: &[&Escuadra]) -> Self {
        Self { heroe: heroe.id, nivel: heroe.nivel, poder: escuadras.iter().map(|e| e.poder(&r.militar)).sum() }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Seleccion {
    pub entran: Vec<HeroeId>,
    pub apartados: Vec<HeroeId>,
}

/// El tope de héroes por bando: 15 en un asedio (y en una avanzada, que se juega como asedio), 5 en el resto.
pub fn tope(r: &Reglas, asedio: bool) -> usize {
    if asedio {
        r.batalla.capacidad_asedio
    } else {
        r.batalla.capacidad_resto
    }
}

/// `alTopeDeBatalla`: los `tope` mejores, por nivel descendente, luego poder descendente, luego id ascendente.
pub fn seleccionar(candidatos: &[Candidato], tope: usize) -> Seleccion {
    let mut v = candidatos.to_vec();
    v.sort_by(|a, b| b.nivel.cmp(&a.nivel).then(b.poder.cmp(&a.poder)).then(a.heroe.cmp(&b.heroe)));
    let apartados = v.split_off(tope.min(v.len()));
    Seleccion { entran: v.iter().map(|c| c.heroe).collect(), apartados: apartados.iter().map(|c| c.heroe).collect() }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::resolucion::apoyo::*;
    use militar::Tropa;

    fn c(heroe: HeroeId, nivel: u32, poder: Mil) -> Candidato {
        Candidato { heroe, nivel, poder }
    }

    #[test]
    fn el_tope_es_15_en_un_asedio_y_5_en_el_resto() {
        let r = r();
        assert_eq!((tope(r, true), tope(r, false)), (15, 5));
    }

    #[test]
    fn entran_los_de_mas_nivel_luego_los_mas_fuertes_luego_el_id_menor() {
        let cs = [c(7, 1, 900), c(3, 5, 10), c(9, 5, 50), c(1, 1, 900), c(4, 1, 10)];
        let s = seleccionar(&cs, 3);
        assert_eq!(s.entran, vec![9, 3, 1], "nivel 5 (más poder primero), y entre los de nivel 1 con igual poder, el id menor");
        assert_eq!(s.apartados, vec![7, 4]);
    }

    #[test]
    fn un_tope_mayor_que_los_candidatos_no_aparta_a_nadie_y_el_cero_aparta_a_todos() {
        let cs = [c(1, 1, 1), c(2, 1, 1)];
        assert_eq!(seleccionar(&cs, 15).apartados, Vec::<HeroeId>::new());
        assert_eq!(seleccionar(&cs, 0).entran, Vec::<HeroeId>::new());
        assert_eq!(seleccionar(&[], 5), Seleccion { entran: vec![], apartados: vec![] });
    }

    #[test]
    fn es_independiente_del_orden_de_entrada() {
        let cs = [c(5, 2, 40), c(6, 2, 40), c(2, 3, 0), c(8, 1, 99), c(1, 2, 41)];
        let mut al_reves = cs;
        al_reves.reverse();
        assert_eq!(seleccionar(&cs, 3), seleccionar(&al_reves, 3));
        let s = seleccionar(&cs, 4);
        assert_eq!(s.entran.len() + s.apartados.len(), cs.len(), "ni se pierde ni se duplica nadie");
    }

    #[test]
    fn el_candidato_suma_el_poder_de_sus_escuadras() {
        let r = r();
        let mut h = Heroe::nuevo(&r.militar, 4, militar::heroe::Controlador::Bot, 1, 1);
        h.nivel = 3;
        let (a, b) = (escuadra(1, 4, Tropa::MiliciaLanceros), escuadra(2, 4, Tropa::Arqueros));
        assert_eq!(Candidato::de(r, &h, &[&a, &b]), c(4, 3, a.poder(&r.militar) + b.poder(&r.militar)));
        assert_eq!(Candidato::de(r, &h, &[]).poder, 0);
    }
}
