//! Lo que se hace con una tecnología aparecida: adoptarla (TA §4), comprarla a un Aeda (TA §6.4), su bonus de producción (TA §4.1) y las que tiene un
//! campamento de mercenarios (TA §5). Quién da la orden (el Rey; el Rey o el Gobernador presentes) lo autoriza el núcleo; aquí van las condiciones
//! del dominio en el orden del TS, con rechazos finos (decisión técnica 9).

use crate::datos::Reglas;
use crate::estado::EstadoTecnologia;
use crate::{Conjunto, FaccionId, TecId};
use asentamiento::datos::UNO;
use asentamiento::{Almacen, Recurso};
use base::{Instante, Mil};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorAdopcion {
    /// `tecnologia.sin_capital`: la Facción no tiene capital efectiva.
    SinCapital,
    /// `tecnologia.rey_fuera_de_la_capital`.
    ReyFueraDeLaCapital,
    /// `tecnologia.ya_adoptada` (también las de arranque).
    YaAdoptada,
    /// `tecnologia.no_aparecida`.
    NoAparecida,
    /// `tecnologia.sin_tarifa`: el almacén de la capital no tiene la tarifa de la Era.
    SinTarifa,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorVenta {
    /// `aedas.era_no_vendible`: `orden > AEDAS.venta.ordenEraMaximo`.
    EraNoVendible,
    /// `aedas.no_la_conoce` (incluye «sin logro cumplido»: un Aeda solo conoce lo que tiene logro).
    NoLaConoce,
    /// `aedas.ya_aparecida`.
    YaAparecida,
    /// `aedas.sin_oro`: el almacén de la plaza no tiene el precio.
    SinOro,
}

/// `precioDeVenta`: el oro de la tarifa de adopción de su Era por `AEDAS.venta.factorOro`, en milésimas.
pub fn precio_de_venta(r: &Reglas, t: TecId) -> Mil {
    r.tarifa(t).iter().filter(|&&(rec, _)| rec == Recurso::Oro).map(|&(_, n)| n).sum::<Mil>() * r.aedas.factor_oro_venta
}

impl EstadoTecnologia {
    /// `adoptarTecnologia`: el Rey en la capital la adopta y la paga el almacén de la capital (`capital`; `None` si la Facción no tiene). Instantánea y
    /// sin descuento por la vía (`tecnologia.adoptada { faccion, tecnologia, capital }`, privado, atribuido a la capital).
    pub fn adoptar(&mut self, r: &Reglas, f: FaccionId, t: TecId, capital: Option<&mut Almacen>, rey_en_capital: bool) -> Result<(), ErrorAdopcion> {
        let almacen = capital.ok_or(ErrorAdopcion::SinCapital)?;
        if !rey_en_capital {
            return Err(ErrorAdopcion::ReyFueraDeLaCapital);
        }
        let propias = self.de(r, f);
        if propias.adoptadas.contiene(t) {
            return Err(ErrorAdopcion::YaAdoptada);
        }
        if !propias.aparecidas.contiene(t) {
            return Err(ErrorAdopcion::NoAparecida);
        }
        if !almacen.pagar(r.tarifa(t)) {
            return Err(ErrorAdopcion::SinTarifa);
        }
        self.de_mut(r, f).adoptadas.poner(t);
        Ok(())
    }

    /// `venderTecnologia` (con un Aeda parado en la plaza, que comprueba el núcleo): se salta el hito, **nunca el logro**; paga el almacén de la plaza y
    /// la tecnología pasa a aparecida (no a adoptada). Se vende a la Facción dueña de la plaza (decisión 16). Devuelve el precio cobrado (milésimas) para
    /// `aedas.venta { faccion, tecnologia, aeda, precio }` (privado de la plaza). Nunca es «la primera»: un Aeda solo vende lo que ya tiene descubridora.
    pub fn vender(&mut self, r: &Reglas, f: FaccionId, t: TecId, plaza: &mut Almacen, ahora: Instante) -> Result<Mil, ErrorVenta> {
        if r.orden_era(t) > r.aedas.orden_era_maximo_venta {
            return Err(ErrorVenta::EraNoVendible);
        }
        if !self.conocida_por_aedas(r, t, ahora) {
            return Err(ErrorVenta::NoLaConoce);
        }
        if self.de(r, f).aparecidas.contiene(t) {
            return Err(ErrorVenta::YaAparecida);
        }
        let precio = precio_de_venta(r, t);
        if !plaza.pagar(&[(Recurso::Oro, precio)]) {
            return Err(ErrorVenta::SinOro);
        }
        self.hacer_aparecer(r, f, t, ahora).expect("no aparecida");
        Ok(precio)
    }

    /// TA §5: ¿desbloquea el campamento la tecnología? Las de arranque, siempre; las demás, si alguien la desbloqueó (`primeros`) y ha pasado
    /// `horasTrasLaPrimera` o la tienen **adoptada** al menos `porcentajeFacciones` de las Facciones vivas (`vivas`, las que conservan algún asentamiento;
    /// 0 vivas → no). C1: adoptada, no «la tiene».
    pub fn del_campamento(&self, r: &Reglas, t: TecId, vivas: &[FaccionId], ahora: Instante) -> bool {
        if r.def(t).de_arranque {
            return true;
        }
        let Some(desde) = self.campamento_libre_en(r, t) else { return false };
        if ahora >= desde {
            return true;
        }
        let adoptan = vivas.iter().filter(|&&f| self.tiene(r, f, t)).count() as i64;
        !vivas.is_empty() && adoptan * UNO >= r.campamento.porcentaje_facciones * vivas.len() as i64
    }

    /// Cuándo la desbloquea el campamento por plazo (`primeros.en + horasTrasLaPrimera`): la cita de 6C. `None` si nadie la ha desbloqueado.
    pub fn campamento_libre_en(&self, r: &Reglas, t: TecId) -> Option<Instante> {
        self.primeros[t as usize].map(|p| p.en + r.campamento.tras_la_primera)
    }

    /// `tecnologiasDelCampamento`: todas las que desbloquea ahora.
    pub fn tecnologias_del_campamento(&self, r: &Reglas, vivas: &[FaccionId], ahora: Instante) -> Conjunto {
        (0..r.catalogo.len() as TecId).filter(|&t| self.del_campamento(r, t, vivas, ahora)).collect()
    }
}

/// `factorProduccionTecnologica` (ppm): el producto de los `bonusProduccion` de las adoptadas sobre ese recurso (hoy solo `canteria`, ×1,5 la piedra).
/// Es el `factor_tecnologia` del `Exterior` de la economía (6C), que se aplica al ritmo antes del resto oculto.
pub fn factor_produccion(r: &Reglas, adoptadas: Conjunto, recurso: Recurso) -> i64 {
    Conjunto(adoptadas.0 & r.con_bonus.0).iter().filter_map(|t| r.def(t).bonus).filter(|b| b.recurso == recurso).fold(UNO, |f, b| f * b.factor / UNO)
}

#[cfg(test)]
mod pruebas {
    //! Casos de `typescript-final:src/session/__tests__/comandosTecnologia.test.ts` (adopción y venta, la parte de reglas), `aedas.test.ts` («vender»),
    //! `canteria.test.ts` (el factor) y la parte de `tecnologiasDelCampamento` de `reclutamientoMercenario.test.ts`.
    use super::*;
    use crate::datos::HORA;
    use crate::estado::pruebas::{r, t};
    use crate::hito::pruebas::{con_cobre_desbloqueado, Vista};
    use crate::estado::Primero;

    fn almacen(oro: Mil, madera: Mil) -> Almacen {
        let mut a = Almacen::nuevo(10_000_000);
        a.poner(Recurso::Oro, oro);
        a.poner(Recurso::Madera, madera);
        a
    }

    /// La Facción 1 con `metalurgia_cobre` aparecida (`conCobreAparecido`).
    fn con_cobre_aparecido() -> EstadoTecnologia {
        let r = r();
        let mut e = EstadoTecnologia::nuevo(&r, 0);
        e.hacer_aparecer(&r, 1, t(&r, "metalurgia_cobre"), 0);
        e
    }

    #[test]
    fn el_rey_en_la_capital_la_adopta_y_la_paga_el_almacen_de_la_capital() {
        let r = r();
        let cobre = t(&r, "metalurgia_cobre");
        let mut e = con_cobre_aparecido();
        let mut capital = almacen(500_000, 500_000);
        assert_eq!(e.adoptar(&r, 1, cobre, Some(&mut capital), true), Ok(()));
        assert!(e.tiene(&r, 1, cobre));
        assert_eq!((capital.cantidad(Recurso::Oro), capital.cantidad(Recurso::Madera)), (400_000, 300_000), "tarifa de la Era I: 100 de oro y 200 de madera");
    }

    #[test]
    fn rechazos_de_la_adopcion_sin_mutar() {
        let r = r();
        let (cobre, bronce, leva) = (t(&r, "metalurgia_cobre"), t(&r, "aleacion_bronce"), t(&r, "leva_comunal"));
        let mut e = con_cobre_aparecido();
        let antes = e.clone();
        let mut capital = almacen(500_000, 500_000);
        assert_eq!(e.adoptar(&r, 1, bronce, Some(&mut capital), true), Err(ErrorAdopcion::NoAparecida));
        assert_eq!(e.adoptar(&r, 1, leva, Some(&mut capital), true), Err(ErrorAdopcion::YaAdoptada));
        assert_eq!(e.adoptar(&r, 1, cobre, Some(&mut capital), false), Err(ErrorAdopcion::ReyFueraDeLaCapital));
        assert_eq!(e.adoptar(&r, 1, cobre, None, true), Err(ErrorAdopcion::SinCapital));
        let mut pobre = almacen(99_000, 500_000);
        assert_eq!(e.adoptar(&r, 1, cobre, Some(&mut pobre), true), Err(ErrorAdopcion::SinTarifa));
        assert_eq!(pobre.cantidad(Recurso::Madera), 500_000, "sin tarifa no se cobra nada");
        assert_eq!((e, capital.cantidad(Recurso::Oro)), (antes, 500_000));
    }

    #[test]
    fn cuesta_el_doble_del_oro_de_la_tarifa_sale_de_la_plaza_y_aparece_sin_hito_no_adoptada() {
        let r = r();
        let cobre = t(&r, "metalurgia_cobre");
        assert_eq!(precio_de_venta(&r, cobre), 200_000);
        let mut e = con_cobre_desbloqueado();
        let mut plaza = almacen(5_000_000, 0);
        assert_eq!(e.vender(&r, 2, cobre, &mut plaza, r.aedas.retraso_conocimiento), Ok(200_000));
        assert_eq!(plaza.cantidad(Recurso::Oro), 4_800_000);
        let f2 = e.de(&r, 2);
        assert!(f2.aparecidas.contiene(cobre) && !f2.adoptadas.contiene(cobre));
        // Y desde ahí se adopta como cualquier aparecida, pagando la tarifa.
        let mut capital = almacen(500_000, 500_000);
        assert_eq!(e.adoptar(&r, 2, cobre, Some(&mut capital), true), Ok(()));
    }

    #[test]
    fn comprar_una_revelada_la_quita_de_las_reveladas() {
        let r = r();
        let cobre = t(&r, "metalurgia_cobre");
        let ret = r.aedas.retraso_conocimiento;
        let mut e = con_cobre_desbloqueado();
        e.revelar(&r, 2, &Vista::default(), ret);
        e.vender(&r, 2, cobre, &mut almacen(5_000_000, 0), ret).unwrap();
        assert!(e.de(&r, 2).reveladas.is_empty());
    }

    #[test]
    fn rechazos_de_la_venta_antes_de_que_la_conozca_sin_logro_ya_aparecida_o_sin_oro() {
        let r = r();
        let cobre = t(&r, "metalurgia_cobre");
        let ret = r.aedas.retraso_conocimiento;
        let mut rico = almacen(5_000_000, 0);
        assert_eq!(con_cobre_desbloqueado().vender(&r, 2, cobre, &mut rico, ret - 1), Err(ErrorVenta::NoLaConoce));
        assert_eq!(EstadoTecnologia::nuevo(&r, 0).vender(&r, 2, cobre, &mut rico, ret), Err(ErrorVenta::NoLaConoce));
        assert_eq!(con_cobre_desbloqueado().vender(&r, 2, cobre, &mut almacen(0, 0), ret), Err(ErrorVenta::SinOro));
        let mut una = con_cobre_desbloqueado();
        una.vender(&r, 2, cobre, &mut rico, ret).unwrap();
        assert_eq!(una.vender(&r, 2, cobre, &mut rico, ret), Err(ErrorVenta::YaAparecida));
        assert_eq!(rico.cantidad(Recurso::Oro), 4_800_000, "solo cobró la primera");
    }

    #[test]
    fn canteria_es_de_la_era_i_y_multiplica_solo_la_piedra() {
        let r = r();
        let canteria = t(&r, "canteria");
        assert_eq!(r.def(canteria).era, 0);
        let con: Conjunto = [canteria].into_iter().collect();
        assert_eq!(factor_produccion(&r, con, Recurso::Piedra), 1_500_000);
        assert_eq!(factor_produccion(&r, con, Recurso::Oro), UNO);
        assert_eq!(factor_produccion(&r, Conjunto::VACIO, Recurso::Piedra), UNO);
        assert_eq!(factor_produccion(&r, r.de_arranque, Recurso::Piedra), UNO, "las de arranque no dan bonus");
        assert_eq!(r.con_bonus, con, "hoy solo la cantería da bonus");
    }

    fn campamento(primera: bool, adoptan: &[FaccionId]) -> EstadoTecnologia {
        let r = r();
        let id = t(&r, "escudos_ligeros");
        let mut e = EstadoTecnologia::nuevo(&r, 0);
        if primera {
            e.primeros[id as usize] = Some(Primero { faccion: 1, en: 0 });
        }
        for &f in adoptan {
            let x = e.de_mut(&r, f);
            x.aparecidas.poner(id);
            x.adoptadas.poner(id);
        }
        e
    }

    #[test]
    fn el_campamento_la_desbloquea_cuando_la_tiene_adoptada_la_mitad_de_las_vivas() {
        let r = r();
        let id = t(&r, "escudos_ligeros");
        let e = campamento(true, &[1, 2]);
        assert!(e.del_campamento(&r, id, &[1, 2, 3, 4], 0));
        assert!(!e.del_campamento(&r, id, &[1, 2, 3, 4, 5], 0), "2 de 5 no llega a la mitad");
    }

    #[test]
    fn o_a_las_72_horas_de_que_la_primera_la_desbloqueo_lo_que_llegue_antes() {
        let r = r();
        let id = t(&r, "escudos_ligeros");
        let e = campamento(true, &[1]);
        assert!(!e.del_campamento(&r, id, &[1, 2, 3], 72 * HORA - 1));
        assert!(e.del_campamento(&r, id, &[1, 2, 3], 72 * HORA));
        assert_eq!(e.campamento_libre_en(&r, id), Some(72 * HORA));
    }

    #[test]
    fn una_que_nadie_ha_desbloqueado_no_esta_y_sin_facciones_vivas_manda_solo_el_plazo() {
        let r = r();
        let id = t(&r, "escudos_ligeros");
        assert!(!campamento(false, &[]).del_campamento(&r, id, &[1], 0));
        let e = campamento(true, &[]);
        assert!(!e.del_campamento(&r, id, &[], 0));
        assert!(e.del_campamento(&r, id, &[], 100 * HORA));
        assert!(e.tecnologias_del_campamento(&r, &[], 0).es_subconjunto_de(r.de_arranque) && r.de_arranque.es_subconjunto_de(e.tecnologias_del_campamento(&r, &[], 0)), "las de arranque, siempre");
    }
}
