//! El hito propio de cada tecnología (TA §3.1) sobre una vista de la Facción que da el núcleo, la aparición por hito (TA §3 paso 3) y lo que revela un
//! Aeda parado en una plaza (TA §6.3).
//!
//! Disparadores de la aparición (plan §5.1, sin muestreo): al fijarse un logro o empezar una Era, todas las Facciones vivas; al completarse o mejorarse
//! un edificio o subir de nivel una plaza, su Facción; al adoptar, la Facción; al crecer la zona de una plaza, su Facción; al ganar una plaza o cambiar
//! de capital, la Facción; y en el paso de minuto de la capital solo si [`EstadoTecnologia::pide_recurso_en_capital`]. Perder algo no se mira: aparecer
//! es para siempre. «Viva» (con algún asentamiento) lo comprueba el núcleo antes de llamar.

use crate::datos::{Condicion, Reglas};
use crate::estado::{EstadoTecnologia, TecnologiasFaccion};
use crate::{FaccionId, TecId};
use asentamiento::{Recurso, Tipo};
use base::Instante;

/// Lo que el hito mira de una Facción: lo implementa el núcleo sobre sus plazas (≤ 7) y su capital efectiva. La condición `tecnologia` la mira este
/// crate con las adoptadas.
pub trait VistaFaccion {
    /// Algún asentamiento de la Facción con ese edificio **activo** y `nivelInterno ≥ nivel`.
    fn edificio_activo(&self, tipo: Tipo, nivel: u8) -> bool;
    /// El almacén de la capital tiene más de 0 de ese recurso.
    fn recurso_en_capital(&self, r: Recurso) -> bool;
    /// Hay capital, con nivel **actual** ≥ `nivel` y ese edificio activo en ella.
    fn capital_en_nivel(&self, nivel: u8, con: Tipo) -> bool;
    /// Alguna zona de influencia de una plaza de la Facción contiene un nodo de ese recurso (con o sin stock).
    fn yacimiento_en_territorio(&self, r: Recurso) -> bool;
}

/// `cumpleCondicion`.
pub fn cumple(c: &Condicion, propias: &TecnologiasFaccion, v: &impl VistaFaccion) -> bool {
    match *c {
        Condicion::Edificio { tipo, nivel } => v.edificio_activo(tipo, nivel),
        Condicion::Tecnologia(t) => propias.adoptadas.contiene(t),
        Condicion::RecursoEnCapital(r) => v.recurso_en_capital(r),
        Condicion::CapitalEnNivel { nivel, con } => v.capital_en_nivel(nivel, con),
        Condicion::YacimientoEnTerritorio(r) => v.yacimiento_en_territorio(r),
    }
}

/// Los índices de las condiciones del hito que la Facción **no** cumple (`faltan` de `aedas.revela`, decisión 12: índices, no texto).
pub fn faltan(r: &Reglas, t: TecId, propias: &TecnologiasFaccion, v: &impl VistaFaccion) -> Vec<u8> {
    r.def(t).hito.iter().enumerate().filter(|(_, c)| !cumple(c, propias, v)).map(|(i, _)| i as u8).collect()
}

/// Le aparece una tecnología a una Facción (`tecnologia.aparece { faccion, tecnologia, primera }`, privado, atribuido a la capital).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Aparicion {
    pub tec: TecId,
    pub primera: bool,
}

/// Un Aeda revela una tecnología a la Facción dueña de la plaza (`aedas.revela`, privado de la plaza).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Revelacion {
    pub tec: TecId,
    pub descubridora: FaccionId,
    pub faltan: Vec<u8>,
}

impl EstadoTecnologia {
    /// TA §3 paso 3, para una Facción viva: cada candidata (logro cumplido, Era abierta, no aparecida) cuyo hito se cumple entero aparece.
    pub fn aparecer_por_hito(&mut self, r: &Reglas, f: FaccionId, v: &impl VistaFaccion, ahora: Instante) -> Vec<Aparicion> {
        let propias = self.de(r, f);
        let listas: Vec<TecId> = self.candidatas(r, f).filter(|&t| r.def(t).hito.iter().all(|c| cumple(c, &propias, v))).collect();
        listas.into_iter().filter_map(|t| self.hacer_aparecer(r, f, t, ahora).map(|primera| Aparicion { tec: t, primera })).collect()
    }

    /// ¿Alguna candidata de la Facción pide `recursoEnCapital`? Solo entonces el paso de minuto de la capital evalúa la aparición (plan §5.1).
    pub fn pide_recurso_en_capital(&self, r: &Reglas, f: FaccionId) -> bool {
        self.candidatas(r, f).any(|t| r.def(t).hito.iter().any(|c| matches!(c, Condicion::RecursoEnCapital(_))))
    }

    /// `revelarTecnologias` (TA §6.3), con un Aeda parado en una plaza de la Facción `f`: cada tecnología que el Aeda conoce y la Facción no tiene
    /// aparecida ni revelada pasa a revelada. La revelación es a la dueña de la plaza, sea amiga o enemiga.
    pub fn revelar(&mut self, r: &Reglas, f: FaccionId, v: &impl VistaFaccion, ahora: Instante) -> Vec<Revelacion> {
        let propias = self.de(r, f);
        let nuevas: Vec<Revelacion> = (0..r.catalogo.len() as TecId)
            .filter(|&t| !propias.aparecidas.contiene(t) && !propias.reveladas.contiene(t) && self.conocida_por_aedas(r, t, ahora))
            .map(|t| Revelacion { tec: t, descubridora: self.primeros[t as usize].expect("conocida").faccion, faltan: faltan(r, t, &propias, v) })
            .collect();
        if !nuevas.is_empty() {
            let e = self.de_mut(r, f);
            nuevas.iter().for_each(|x| e.reveladas.poner(x.tec));
        }
        nuevas
    }
}

#[cfg(test)]
pub(crate) mod pruebas {
    //! Casos de `typescript-final:src/engine/__tests__/tecnologia.test.ts` («avance de tecnología») y `aedas.test.ts` («revelar»).
    use super::*;
    use crate::contador::Contador;
    use crate::estado::pruebas::{r, t};
    use crate::estado::Primero;

    /// Una vista de prueba: los edificios activos (tipo, nivel), lo que hay en la capital, el nivel de la capital con sus edificios y los yacimientos.
    #[derive(Default)]
    pub struct Vista {
        pub edificios: Vec<(Tipo, u8)>,
        pub capital: Vec<Recurso>,
        pub nivel_capital: u8,
        pub yacimientos: Vec<Recurso>,
    }

    impl VistaFaccion for Vista {
        fn edificio_activo(&self, tipo: Tipo, nivel: u8) -> bool {
            self.edificios.iter().any(|&(t, n)| t == tipo && n >= nivel)
        }
        fn recurso_en_capital(&self, r: Recurso) -> bool {
            self.capital.contains(&r)
        }
        fn capital_en_nivel(&self, nivel: u8, con: Tipo) -> bool {
            self.nivel_capital >= nivel && self.edificio_activo(con, 1)
        }
        fn yacimiento_en_territorio(&self, r: Recurso) -> bool {
            self.yacimientos.contains(&r)
        }
    }

    pub fn con_fundicion() -> Vista {
        Vista { edificios: vec![(Tipo::Fundicion, 1)], ..Vista::default() }
    }

    /// Metalurgia del cobre desbloqueada por la Facción 1 en el instante 0 (su logro y su hito, a mano), como `conCobreDesbloqueado` del TS.
    pub fn con_cobre_desbloqueado() -> EstadoTecnologia {
        let r = r();
        let mut e = EstadoTecnologia::nuevo(&r, 0);
        let cobre = t(&r, "metalurgia_cobre");
        e.logros[cobre as usize] = Some(0);
        e.primeros[cobre as usize] = Some(Primero { faccion: 1, en: 0 });
        e
    }

    #[test]
    fn sin_logro_no_aparece_nada_con_logro_solo_a_quien_cumple_el_hito_y_el_logro_queda_fijado() {
        let r = r();
        let cobre = t(&r, "metalurgia_cobre");
        let (f1, f2) = (con_fundicion(), Vista::default());
        let mut e = EstadoTecnologia::nuevo(&r, 0);
        assert!(e.aparecer_por_hito(&r, 1, &f1, 1).is_empty());
        assert!(e.logros[cobre as usize].is_none());

        e.sumar(&r, Contador::ExtraidoCobre, 41_000, 2);
        assert_eq!(e.aparecer_por_hito(&r, 1, &f1, 2), vec![Aparicion { tec: cobre, primera: true }]);
        assert!(e.aparecer_por_hito(&r, 2, &f2, 2).is_empty());
        assert_eq!(e.logros[cobre as usize], Some(2));
        assert!(e.de(&r, 1).aparecidas.contiene(cobre) && !e.de(&r, 1).adoptadas.contiene(cobre));
        assert!(!e.de(&r, 2).aparecidas.contiene(cobre));
        assert_eq!(e.primeros[cobre as usize], Some(Primero { faccion: 1, en: 2 }));
        assert!(e.aparecer_por_hito(&r, 1, &f1, 3).is_empty(), "otra evaluación no repite");
    }

    #[test]
    fn una_tecnologia_de_una_era_aun_cerrada_no_aparece_aunque_su_logro_y_su_hito_se_cumplan() {
        let r = r();
        let civicas = t(&r, "instituciones_civicas");
        let mut e = EstadoTecnologia::nuevo(&r, 0);
        e.subir_record(&r, Contador::PlazasEnNivel3, 99, 0);
        let v = Vista { edificios: vec![(Tipo::Mercado, 1)], nivel_capital: 3, ..Vista::default() };
        assert!(e.aparecer_por_hito(&r, 1, &v, 1).is_empty());
        assert_eq!(e.era, 0);
        e.era = 2;
        assert_eq!(e.aparecer_por_hito(&r, 1, &v, 2), vec![Aparicion { tec: civicas, primera: true }]);
    }

    #[test]
    fn una_condicion_tecnologia_mira_las_adoptadas() {
        let r = r();
        let (cobre, palacial) = (t(&r, "metalurgia_cobre"), t(&r, "armamento_palacial"));
        let mut e = EstadoTecnologia::nuevo(&r, 0);
        e.subir_record(&r, Contador::PlazasEnNivel2, 12, 0);
        e.hacer_aparecer(&r, 1, cobre, 0);
        let v = Vista { edificios: vec![(Tipo::Armeria, 1)], ..Vista::default() };
        assert!(e.aparecer_por_hito(&r, 1, &v, 1).is_empty(), "aparecida no basta");
        e.de_mut(&r, 1).adoptadas.poner(cobre);
        assert_eq!(e.aparecer_por_hito(&r, 1, &v, 2), vec![Aparicion { tec: palacial, primera: true }]);
    }

    #[test]
    fn el_recurso_en_capital_solo_se_mira_si_una_candidata_lo_pide() {
        let r = r();
        let mut e = EstadoTecnologia::nuevo(&r, 0);
        assert!(!e.pide_recurso_en_capital(&r, 1));
        e.sumar(&r, Contador::ExtraidoEstano, 245_000, 0);
        assert!(e.pide_recurso_en_capital(&r, 1));
        let v = Vista { edificios: vec![(Tipo::Fundicion, 2)], capital: vec![Recurso::Estano], ..Vista::default() };
        assert_eq!(e.aparecer_por_hito(&r, 1, &v, 1).len(), 1);
        assert!(!e.pide_recurso_en_capital(&r, 1));
    }

    #[test]
    fn la_ventaja_del_primero_hasta_que_pasa_el_retraso_no_la_conocen_y_entonces_se_la_revelan_a_quien_aun_no_la_tiene() {
        let r = r();
        let cobre = t(&r, "metalurgia_cobre");
        let retraso = r.aedas.retraso_conocimiento;
        let mut e = con_cobre_desbloqueado();
        assert!(e.revelar(&r, 2, &Vista::default(), retraso - 1).is_empty());
        assert_eq!(e.revelar(&r, 2, &Vista::default(), retraso), vec![Revelacion { tec: cobre, descubridora: 1, faltan: vec![0] }], "le falta la fundición");
        assert!(e.de(&r, 2).reveladas.contiene(cobre) && !e.de(&r, 2).aparecidas.contiene(cobre));
    }

    #[test]
    fn se_revela_una_sola_vez_y_no_a_quien_ya_la_tiene_aparecida() {
        let r = r();
        let retraso = r.aedas.retraso_conocimiento;
        let mut e = con_cobre_desbloqueado();
        assert_eq!(e.revelar(&r, 2, &Vista::default(), retraso).len(), 1);
        assert!(e.revelar(&r, 2, &Vista::default(), retraso + 10).is_empty());
        let mut ya = con_cobre_desbloqueado();
        ya.hacer_aparecer(&r, 2, t(&r, "metalurgia_cobre"), 0);
        assert!(ya.revelar(&r, 2, &Vista::default(), retraso).is_empty());
    }

    #[test]
    fn una_revelada_aparece_al_cumplir_el_hito_y_sale_de_las_reveladas() {
        let r = r();
        let cobre = t(&r, "metalurgia_cobre");
        let mut e = con_cobre_desbloqueado();
        e.revelar(&r, 2, &Vista::default(), r.aedas.retraso_conocimiento);
        assert_eq!(e.aparecer_por_hito(&r, 2, &con_fundicion(), r.aedas.retraso_conocimiento + 1), vec![Aparicion { tec: cobre, primera: false }]);
        assert!(!e.de(&r, 2).reveladas.contiene(cobre));
    }
}
