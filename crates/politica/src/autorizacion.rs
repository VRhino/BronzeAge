//! Los predicados de la matriz de autorización (Proyecciones_Y_Autorizacion §4.3 y §5.2): ¿tiene el héroe del actor, por su relación de juego con la entidad,
//! derecho a **intentarlo**? Lo demás (hay oro, está a la puerta…) lo rechaza la orden con su código. El núcleo da la [`Vista`] de las plazas (dueña, residencia,
//! presencia y cargos); la Facción y sus cargos salen de [`Politica`]. Sustituyen a `manda_en` y a «cualquier ciudadano presente» en 5E.
//!
//! **Fail-open** (§4.2): casi todos devuelven `true` si la plaza o la Facción **no existe**, para que la orden la rechace con su código (`asentamiento.no_existe`…),
//! que dice mucho más que un `no_autorizado`. Excepción: [`Actor::responde_solicitudes`] es fail-closed.
//! «La ciudadanía habilita, la presencia ejerce» (Doc 2.5): `cargo(A, c)` exige residir **y** estar dentro, y ser el titular (Q4: el titular reside).

use crate::tipos::{AsentamientoId, Cargo, Cargos, FaccionId, HeroeId};
use crate::Politica;

/// Lo que la autorización mira de las plazas; lo implementa el núcleo.
pub trait Vista {
    /// La Facción dueña de la plaza; `None` si la plaza no existe.
    fn duena(&self, a: AsentamientoId) -> Option<FaccionId>;
    /// `residente(A)`: `h` reside en la plaza (fundador o mudado).
    fn residente(&self, h: HeroeId, a: AsentamientoId) -> bool;
    /// `presente(A)`: `h` está **dentro** de esa plaza.
    fn presente(&self, h: HeroeId, a: AsentamientoId) -> bool;
    /// Los cargos locales de la plaza.
    fn cargos(&self, a: AsentamientoId) -> Cargos;
}

/// El héroe del actor (`yo`) frente al estado.
pub struct Actor<'a, V: Vista> {
    pub yo: HeroeId,
    pub politica: &'a Politica,
    pub vista: &'a V,
}

impl<V: Vista> Actor<'_, V> {
    /// `yo`: nadie actúa a nombre de otro.
    pub fn es_yo(&self, h: HeroeId) -> bool {
        h == self.yo
    }

    fn existe(&self, f: FaccionId) -> bool {
        self.politica.facciones.existe(f)
    }

    /// `ciudadano(F)`.
    pub fn ciudadano(&self, f: FaccionId) -> bool {
        self.politica.facciones.faccion_de(self.yo) == Some(f)
    }

    /// `rey(F)`.
    pub fn rey(&self, f: FaccionId) -> bool {
        self.politica.facciones.get(f).is_some_and(|x| x.rey == Some(self.yo))
    }

    /// `reyOEmbajador(F)`.
    pub fn rey_o_embajador(&self, f: FaccionId) -> bool {
        self.politica.facciones.get(f).is_some_and(|x| x.rey == Some(self.yo) || x.embajador == Some(self.yo))
    }

    pub fn residente(&self, a: AsentamientoId) -> bool {
        self.vista.residente(self.yo, a)
    }

    pub fn presente(&self, a: AsentamientoId) -> bool {
        self.vista.presente(self.yo, a)
    }

    /// `reside(A)` = residente ∧ presente; plaza inexistente: verdadero.
    pub fn reside(&self, a: AsentamientoId) -> bool {
        self.vista.duena(a).is_none() || (self.residente(a) && self.presente(a))
    }

    fn titular(&self, a: AsentamientoId, c: Cargo) -> bool {
        self.vista.cargos(a).es_titular(c, self.yo)
    }

    /// `cargo(A, c)` = reside ∧ titular de **ese** cargo (no si el puesto está cubierto); plaza inexistente: verdadero.
    pub fn cargo(&self, a: AsentamientoId, c: Cargo) -> bool {
        self.vista.duena(a).is_none() || (self.reside(a) && self.titular(a, c))
    }

    /// `cargo(A, c1) ∨ cargo(A, c2) ∨ …` (comerciar: Tesorero o Gobernador; recetas: Gobernador o Maestro de Obras…).
    pub fn algun_cargo(&self, a: AsentamientoId, cargos: &[Cargo]) -> bool {
        cargos.iter().any(|&c| self.cargo(a, c))
    }

    /// `autoridadDiplomatica(F)` = ciudadano ∧ (Rey ∨ Embajador); Facción inexistente: verdadero. `proponerRelacion`, `declararGuerra`, `proponerAnexion`,
    /// `proponerPaz`, `romperRelacion`, `rebelionVasallo` (de la vasalla), `retirarRelacion`, `retirarAnexion`.
    pub fn autoridad_diplomatica(&self, f: FaccionId) -> bool {
        !self.existe(f) || (self.ciudadano(f) && self.rey_o_embajador(f))
    }

    /// `reyDeFaccion(F)` = ciudadano ∧ Rey; Facción inexistente: verdadero. `designarCapital`, `admitirOtrasFacciones`, `asignarEmbajador`, `responderRelacion`,
    /// `proponerFusion`, `retirarFusion`, `responderFusion`, `adoptarTecnologia`.
    pub fn rey_de_faccion(&self, f: FaccionId) -> bool {
        !self.existe(f) || (self.ciudadano(f) && self.rey(f))
    }

    /// `responderAnexion`: `rey(absorbida)`; Facción inexistente: verdadero.
    pub fn rey_o_inexistente(&self, f: FaccionId) -> bool {
        !self.existe(f) || self.rey(f)
    }

    /// `reyDeLaPlaza(A)`: Rey de la Facción dueña, **sin** residir ni estar presente (designar Gobernador es un acto de nivel Facción); plaza inexistente: verdadero.
    pub fn rey_de_la_plaza(&self, a: AsentamientoId) -> bool {
        self.vista.duena(a).is_none_or(|f| self.rey_de_faccion(f))
    }

    /// `responderSolicitud`: **fail-closed**, solo el Rey de esa Facción (una Facción inexistente deniega).
    pub fn responde_solicitudes(&self, f: FaccionId) -> bool {
        self.rey(f)
    }

    /// `asignarRey`: ciudadano ∧ (sin Rey ∨ Rey): la primera asignación, cualquier ciudadano; con Rey, solo él traspasa. Facción inexistente: verdadero.
    pub fn asigna_rey(&self, f: FaccionId) -> bool {
        match self.politica.facciones.get(f) {
            None => true,
            Some(x) => self.ciudadano(f) && (x.rey.is_none() || x.rey == Some(self.yo)),
        }
    }

    /// `asignarCargoLocal`: el Gobernador lo designa el Rey de la plaza (sin presencia); los otros cuatro, el Gobernador residente y presente.
    pub fn asigna_cargo_local(&self, a: AsentamientoId, c: Cargo) -> bool {
        if c == Cargo::Gobernador {
            self.rey_de_la_plaza(a)
        } else {
            self.cargo(a, Cargo::Gobernador)
        }
    }

    /// `fijarPuerta`: el Gobernador residente y presente, o el Rey de la plaza (no tiene que estar presente).
    pub fn fija_puerta(&self, a: AsentamientoId) -> bool {
        self.cargo(a, Cargo::Gobernador) || self.rey_de_la_plaza(a)
    }

    /// `vetarJugador`: solo el Gobernador residente y presente; **el Rey no** (asimetría con `fijarPuerta`, Presencia §8).
    pub fn veta(&self, a: AsentamientoId) -> bool {
        self.cargo(a, Cargo::Gobernador)
    }

    /// `puedeMoverTropaDe(A)` = residente ∧ presente (`movilizarEjercito`); plaza inexistente: verdadero.
    pub fn mueve_tropa_de(&self, a: AsentamientoId) -> bool {
        self.reside(a)
    }

    /// `puedeOperarAparcada(caravana, A)`: residente del **origen** de la caravana y presente **donde está aparcada**.
    pub fn opera_aparcada(&self, origen: AsentamientoId, aparcada_en: AsentamientoId) -> bool {
        self.residente(origen) && self.presente(aparcada_en)
    }

    /// `dirigeAedas(A)` = presente ∧ (Rey de la dueña ∨ Gobernador ∨ Sacerdote titular); no exige residir. Plaza inexistente: verdadero.
    pub fn dirige_aedas(&self, a: AsentamientoId) -> bool {
        self.vista.duena(a).is_none_or(|f| self.presente(a) && (self.rey(f) || self.titular(a, Cargo::Gobernador) || self.titular(a, Cargo::Sacerdote)))
    }

    /// `compraIntel` en una plaza = presente ∧ (Rey o Embajador de la dueña ∨ Gobernador titular). En un campamento, verdadero (lo mira la orden).
    pub fn compra_intel_en(&self, a: AsentamientoId) -> bool {
        self.vista.duena(a).is_none_or(|f| self.presente(a) && (self.rey_o_embajador(f) || self.titular(a, Cargo::Gobernador)))
    }

    /// `comprarTecnologiaAeda`: plaza y Facción existen ⇒ presente ∧ (Rey ∨ Gobernador titular).
    pub fn compra_tecnologia_aeda(&self, a: AsentamientoId) -> bool {
        match self.vista.duena(a).filter(|&f| self.existe(f)) {
            None => true,
            Some(f) => self.presente(a) && (self.rey(f) || self.titular(a, Cargo::Gobernador)),
        }
    }

    /// `cambiarResidencia`: el destino no existe ∨ ciudadano de su dueña.
    pub fn se_muda_a(&self, destino: AsentamientoId) -> bool {
        self.vista.duena(destino).is_none_or(|f| self.ciudadano(f))
    }

    /// `desarmarCaravanaFundacion`: ciudadano de la Facción de la caravana (sin Facción derivable: pasa).
    pub fn ciudadano_o_sin_faccion(&self, f: Option<FaccionId>) -> bool {
        f.is_none_or(|f| self.ciudadano(f))
    }
}

#[cfg(test)]
mod pruebas {
    //! Casos de `typescript-final:src/session/__tests__/autorizacionComandos.test.ts` que miran estos predicados (la Facción del actor, `responderSolicitud`, la
    //! puerta, la capital, admitir a otras, `asignarCargoLocal`, el cargo por titular y la residencia en la plaza objetivo).
    use super::*;
    use crate::faccion::pruebas::con_faccion;
    use std::collections::{BTreeMap, BTreeSet};

    /// Plaza 1 de la Facción 1 (Rey 10, ciudadanos 11 y 12; 11 Gobernador residente; 12 residente sin cargo). Plaza 2 de la Facción 2 (Rey 20).
    struct Mundo {
        duenas: BTreeMap<AsentamientoId, FaccionId>,
        residentes: BTreeSet<(HeroeId, AsentamientoId)>,
        dentro: BTreeSet<(HeroeId, AsentamientoId)>,
        cargos: BTreeMap<AsentamientoId, Cargos>,
    }

    impl Vista for Mundo {
        fn duena(&self, a: AsentamientoId) -> Option<FaccionId> {
            self.duenas.get(&a).copied()
        }
        fn residente(&self, h: HeroeId, a: AsentamientoId) -> bool {
            self.residentes.contains(&(h, a))
        }
        fn presente(&self, h: HeroeId, a: AsentamientoId) -> bool {
            self.dentro.contains(&(h, a))
        }
        fn cargos(&self, a: AsentamientoId) -> Cargos {
            self.cargos.get(&a).copied().unwrap_or_default()
        }
    }

    fn escenario() -> (Politica, Mundo) {
        let mut p = Politica::default();
        con_faccion(&mut p.facciones, 1, "Micenas", 10, &[11, 12]);
        con_faccion(&mut p.facciones, 2, "Troya", 20, &[]);
        p.facciones.asignar_embajador(1, 12).unwrap();
        let mut c = Cargos::default();
        c.fijar(Cargo::Gobernador, Some(11));
        c.fijar(Cargo::Tesorero, Some(12));
        let m = Mundo {
            duenas: BTreeMap::from([(1, 1), (2, 2)]),
            residentes: BTreeSet::from([(11, 1), (12, 1), (20, 2)]),
            dentro: BTreeSet::from([(11, 1), (12, 1)]),
            cargos: BTreeMap::from([(1, c)]),
        };
        (p, m)
    }

    fn actor<'a>(yo: HeroeId, p: &'a Politica, m: &'a Mundo) -> Actor<'a, Mundo> {
        Actor { yo, politica: p, vista: m }
    }

    #[test]
    fn la_faccion_del_actor_es_su_ciudadania() {
        let (p, m) = escenario();
        assert!(actor(11, &p, &m).ciudadano(1));
        assert!(!actor(99, &p, &m).ciudadano(1), "un forastero");
        assert!(!actor(20, &p, &m).ciudadano(1), "un ciudadano de la rival");
    }

    #[test]
    fn responder_solicitudes_es_del_rey_y_fail_closed() {
        let (p, m) = escenario();
        assert!(actor(10, &p, &m).responde_solicitudes(1));
        assert!(!actor(11, &p, &m).responde_solicitudes(1));
        assert!(!actor(10, &p, &m).responde_solicitudes(9), "Facción inexistente: deniega");
        assert!(actor(10, &p, &m).rey_de_faccion(9), "las demás pasan");
    }

    #[test]
    fn la_puerta_la_fija_el_gobernador_o_el_rey_y_solo_el_gobernador_veta() {
        let (p, m) = escenario();
        assert!(actor(11, &p, &m).fija_puerta(1) && actor(11, &p, &m).veta(1));
        assert!(actor(10, &p, &m).fija_puerta(1), "el Rey sin estar presente");
        assert!(!actor(10, &p, &m).veta(1), "el Rey no veta");
        assert!(!actor(12, &p, &m).fija_puerta(1) && !actor(20, &p, &m).fija_puerta(1));
        assert!(actor(20, &p, &m).fija_puerta(77), "plaza inexistente: pasa");
    }

    #[test]
    fn capital_admision_y_embajador_solo_el_rey() {
        let (p, m) = escenario();
        assert!(actor(10, &p, &m).rey_de_faccion(1));
        assert!(!actor(11, &p, &m).rey_de_faccion(1) && !actor(20, &p, &m).rey_de_faccion(1));
        assert!(actor(12, &p, &m).autoridad_diplomatica(1), "el Embajador sí tiene autoridad diplomática");
        assert!(!actor(11, &p, &m).autoridad_diplomatica(1));
    }

    #[test]
    fn asignar_cargo_local() {
        let (p, m) = escenario();
        assert!(actor(10, &p, &m).asigna_cargo_local(1, Cargo::Gobernador), "el Rey designa Gobernador sin residir");
        assert!(!actor(11, &p, &m).asigna_cargo_local(1, Cargo::Gobernador));
        assert!(actor(11, &p, &m).asigna_cargo_local(1, Cargo::Tesorero), "el Gobernador vigente designa al resto");
        assert!(!actor(12, &p, &m).asigna_cargo_local(1, Cargo::Tesorero), "otro residente sin ese cargo, no");
        assert!(!actor(10, &p, &m).asigna_cargo_local(1, Cargo::Tesorero), "el Rey tampoco");
    }

    #[test]
    fn el_cargo_se_comprueba_sobre_el_titular_y_la_presencia_ejerce() {
        let (p, mut m) = escenario();
        assert!(actor(12, &p, &m).cargo(1, Cargo::Tesorero));
        assert!(!actor(11, &p, &m).cargo(1, Cargo::Tesorero), "el puesto está ocupado, pero no por él");
        assert!(actor(11, &p, &m).algun_cargo(1, &[Cargo::Tesorero, Cargo::Gobernador]));
        m.dentro.remove(&(12, 1));
        assert!(!actor(12, &p, &m).cargo(1, Cargo::Tesorero), "un Tesorero de campaña no comercia desde el camino");
        assert!(!actor(12, &p, &m).reside(1) && actor(12, &p, &m).residente(1));
    }

    #[test]
    fn asignar_rey_abierto_sin_rey_y_cerrado_con_el() {
        let (mut p, m) = escenario();
        assert!(actor(10, &p, &m).asigna_rey(1) && !actor(11, &p, &m).asigna_rey(1));
        p.facciones.get_mut(1).unwrap().rey = None;
        assert!(actor(11, &p, &m).asigna_rey(1), "sin Rey, cualquier ciudadano");
        assert!(!actor(20, &p, &m).asigna_rey(1));
    }

    #[test]
    fn intel_aedas_y_aparcadas() {
        let (p, mut m) = escenario();
        assert!(actor(11, &p, &m).compra_intel_en(1), "el Gobernador dentro");
        assert!(actor(12, &p, &m).compra_intel_en(1), "el Embajador dentro");
        assert!(!actor(10, &p, &m).compra_intel_en(1), "el Rey fuera, no");
        m.dentro.insert((10, 1));
        assert!(actor(10, &p, &m).compra_intel_en(1) && actor(10, &p, &m).dirige_aedas(1), "dentro, sin residir");
        assert!(!actor(12, &p, &m).dirige_aedas(1));
        assert!(actor(11, &p, &m).opera_aparcada(1, 1) && !actor(11, &p, &m).opera_aparcada(1, 2));
        assert!(actor(11, &p, &m).se_muda_a(1) && !actor(20, &p, &m).se_muda_a(1) && actor(20, &p, &m).se_muda_a(77));
        assert!(actor(11, &p, &m).ciudadano_o_sin_faccion(None) && !actor(20, &p, &m).ciudadano_o_sin_faccion(Some(1)));
    }
}
