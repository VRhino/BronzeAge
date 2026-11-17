//! La puerta de una plaza (Diplomacia_Facciones §4.3, Presencia §8, Doc 1.10.5), con la decisión **Q1** del autor (2026-10-24): se cierra **por grupos**
//! (neutrales, aliados —también vasallos y señores—, enemigos en guerra y los Aedas), el Gobernador veta a jugadores concretos, una plaza que nadie ha tocado
//! está **cerrada a neutrales y enemigos** (`PUERTA.cerradaAPorDefecto`) y **cerrar o vetar expulsa** a quien ya no puede entrar. Las capas, en este orden:
//! un residente entra siempre; un vetado no entra; si no, entra si su grupo no está cerrado; la propia Facción nunca se bloquea.
//! Sustituye al acceso provisional por relación (`FijarAcceso`, 2026-10-22).

use crate::datos::Reglas;
use crate::relaciones::Relaciones;
use crate::tipos::{FaccionId, Grupo, Grupos, HeroeId};
use std::collections::BTreeSet;

/// A qué grupo pertenece quien llega respecto a la Facción de la plaza.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrupoDePuerta {
    Propia,
    De(Grupo),
}

/// `grupoDePuerta`: la misma Facción → propia; alianza o vasallaje activos en cualquier sentido → aliados; guerra activa → enemigos; el resto (y quien no
/// tiene Facción) → neutrales.
pub fn grupo_de_puerta(relaciones: &Relaciones, del_heroe: Option<FaccionId>, de_la_plaza: FaccionId) -> GrupoDePuerta {
    match del_heroe {
        Some(f) if f == de_la_plaza => GrupoDePuerta::Propia,
        Some(f) if relaciones.comparten_vision(f, de_la_plaza) => GrupoDePuerta::De(Grupo::Aliados),
        Some(f) if relaciones.en_guerra(f, de_la_plaza) => GrupoDePuerta::De(Grupo::Enemigos),
        _ => GrupoDePuerta::De(Grupo::Neutrales),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorPuerta {
    /// A un residente no se le veta: eso sería el exilio.
    Residente,
}

/// La puerta de una plaza: los grupos cerrados (`None` = los de por defecto, también tras una conquista) y los vetados, ordenados.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Puerta {
    pub cerrada_a: Option<Grupos>,
    pub vetados: BTreeSet<HeroeId>,
}

/// Quien llega a la puerta.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Visitante {
    pub heroe: HeroeId,
    pub faccion: Option<FaccionId>,
    pub residente: bool,
}

impl Puerta {
    /// Los grupos que cierra ahora.
    pub fn cerrada(&self, r: &Reglas) -> Grupos {
        self.cerrada_a.unwrap_or(r.puerta_por_defecto)
    }

    /// `puedeEntrarEn`, por capas.
    pub fn puede_entrar(&self, r: &Reglas, relaciones: &Relaciones, de_la_plaza: FaccionId, v: Visitante) -> bool {
        if v.residente {
            return true;
        }
        if self.vetados.contains(&v.heroe) {
            return false;
        }
        match grupo_de_puerta(relaciones, v.faccion, de_la_plaza) {
            GrupoDePuerta::Propia => true,
            GrupoDePuerta::De(g) => !self.cerrada(r).contiene(g),
        }
    }

    /// ¿Entran los Aedas (Tecnologia_Aedas)? Solo los mira el grupo `aedas`.
    pub fn admite_aedas(&self, r: &Reglas) -> bool {
        !self.cerrada(r).contiene(Grupo::Aedas)
    }

    /// `fijarPuerta`: sustituye el conjunto de grupos cerrados; no caduca y manda el último.
    pub fn fijar(&mut self, cerrada_a: Grupos) {
        self.cerrada_a = Some(cerrada_a);
    }

    /// `vetarJugador`: añade (o quita, con `vetar = false`) a `heroe`. A un residente no se le veta. Devuelve si cambió.
    pub fn vetar(&mut self, heroe: HeroeId, vetar: bool, es_residente: bool) -> Result<bool, ErrorPuerta> {
        match vetar {
            true if es_residente => Err(ErrorPuerta::Residente),
            true => Ok(self.vetados.insert(heroe)),
            false => Ok(self.vetados.remove(&heroe)),
        }
    }

    /// Tras cerrar o vetar: de quienes están dentro (de la plaza, de sus pueblos o de sus avanzadas, que heredan su puerta), los que ya no pueden entrar
    /// salen al mapa con todo lo suyo (lo hace el núcleo). La misma Facción y los residentes no salen nunca.
    pub fn expulsados(&self, r: &Reglas, relaciones: &Relaciones, de_la_plaza: FaccionId, dentro: impl IntoIterator<Item = Visitante>) -> Vec<HeroeId> {
        dentro.into_iter().filter(|&v| !self.puede_entrar(r, relaciones, de_la_plaza, v)).map(|v| v.heroe).collect()
    }
}

#[cfg(test)]
mod pruebas {
    //! Casos de `typescript-final:src/engine/__tests__/puerta.test.ts` y de la parte de puerta de `comandosPresencia.test.ts`.
    use super::*;
    use crate::relaciones::pruebas::facciones;
    use crate::relaciones::TipoRelacion;

    const PROPIA: FaccionId = 1;
    const OTRA: FaccionId = 2;

    fn con(tipo: Option<TipoRelacion>) -> Relaciones {
        let mut rs = Relaciones::default();
        if let Some(t) = tipo {
            rs.crear(&facciones(2), 1, t, PROPIA, OTRA, None, 0).unwrap();
        }
        rs
    }

    fn forastero(faccion: FaccionId) -> Visitante {
        Visitante { heroe: 7, faccion: Some(faccion), residente: false }
    }

    #[test]
    fn el_grupo_sale_de_la_relacion_activa() {
        assert_eq!(grupo_de_puerta(&con(None), Some(PROPIA), PROPIA), GrupoDePuerta::Propia);
        assert_eq!(grupo_de_puerta(&con(None), Some(OTRA), PROPIA), GrupoDePuerta::De(Grupo::Neutrales));
        assert_eq!(grupo_de_puerta(&con(None), None, PROPIA), GrupoDePuerta::De(Grupo::Neutrales), "sin Facción, neutral");
        assert_eq!(grupo_de_puerta(&con(Some(TipoRelacion::Alianza)), Some(OTRA), PROPIA), GrupoDePuerta::De(Grupo::Aliados));
        assert_eq!(grupo_de_puerta(&con(Some(TipoRelacion::Vasallaje)), Some(OTRA), PROPIA), GrupoDePuerta::De(Grupo::Aliados));
        assert_eq!(grupo_de_puerta(&con(Some(TipoRelacion::Guerra)), Some(OTRA), PROPIA), GrupoDePuerta::De(Grupo::Enemigos));
        let mut rota = con(Some(TipoRelacion::Guerra));
        rota.terminar(1);
        assert_eq!(grupo_de_puerta(&rota, Some(OTRA), PROPIA), GrupoDePuerta::De(Grupo::Neutrales), "una relación rota ya no cuenta");
    }

    #[test]
    fn q1_por_defecto_entran_los_aliados_y_se_cierra_a_neutrales_y_enemigos() {
        let (r, p) = (Reglas::default(), Puerta::default());
        assert!(p.puede_entrar(&r, &con(Some(TipoRelacion::Alianza)), PROPIA, forastero(OTRA)));
        assert!(!p.puede_entrar(&r, &con(None), PROPIA, forastero(OTRA)));
        assert!(!p.puede_entrar(&r, &con(Some(TipoRelacion::Guerra)), PROPIA, forastero(OTRA)));
        assert!(p.admite_aedas(&r), "los Aedas entran por defecto");
    }

    #[test]
    fn cada_grupo_se_cierra_y_se_abre_por_separado() {
        let r = Reglas::default();
        let mut p = Puerta::default();
        p.fijar(Grupos::de([Grupo::Aliados, Grupo::Aedas]));
        assert!(!p.puede_entrar(&r, &con(Some(TipoRelacion::Alianza)), PROPIA, forastero(OTRA)));
        assert!(p.puede_entrar(&r, &con(None), PROPIA, forastero(OTRA)));
        assert!(p.puede_entrar(&r, &con(Some(TipoRelacion::Guerra)), PROPIA, forastero(OTRA)));
        assert!(!p.admite_aedas(&r));
    }

    #[test]
    fn la_propia_faccion_nunca_se_bloquea() {
        let r = Reglas::default();
        let mut p = Puerta::default();
        p.fijar(Grupos::de(Grupo::TODOS));
        assert!(p.puede_entrar(&r, &con(None), PROPIA, forastero(PROPIA)));
    }

    #[test]
    fn un_veto_pesa_mas_que_el_grupo_y_un_residente_entra_siempre() {
        let r = Reglas::default();
        let mut p = Puerta::default();
        p.fijar(Grupos::default());
        assert_eq!(p.vetar(7, true, false), Ok(true));
        assert_eq!(p.vetar(7, true, false), Ok(false));
        assert!(!p.puede_entrar(&r, &con(None), PROPIA, forastero(OTRA)));
        assert!(!p.puede_entrar(&r, &con(None), PROPIA, forastero(PROPIA)), "el veto alcanza también a la propia Facción");
        assert_eq!(p.vetar(8, true, true), Err(ErrorPuerta::Residente));
        // Un vetado que acaba residiendo (reubicado desde una ruina) entra siempre.
        p.fijar(Grupos::de(Grupo::TODOS));
        assert!(p.puede_entrar(&r, &con(None), PROPIA, Visitante { heroe: 7, faccion: Some(OTRA), residente: true }));
        assert_eq!(p.vetar(7, false, false), Ok(true));
    }

    #[test]
    fn cerrar_o_vetar_expulsa_a_quien_ya_no_puede_entrar() {
        let r = Reglas::default();
        let rs = con(Some(TipoRelacion::Alianza));
        let mut p = Puerta::default();
        let dentro = [forastero(OTRA), Visitante { heroe: 8, faccion: Some(PROPIA), residente: false }, Visitante { heroe: 9, faccion: Some(OTRA), residente: true }];
        assert!(p.expulsados(&r, &rs, PROPIA, dentro).is_empty());
        p.fijar(Grupos::de([Grupo::Aliados]));
        assert_eq!(p.expulsados(&r, &rs, PROPIA, dentro), vec![7], "sale la aliada; la propia y el residente se quedan");
        p.fijar(Grupos::default());
        p.vetar(8, true, false).unwrap();
        assert_eq!(p.expulsados(&r, &rs, PROPIA, dentro), vec![8]);
    }
}
