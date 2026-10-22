//! Dónde está un héroe y si está conectado (Presencia_Y_Convocatoria §2 y §7, Reloj_Y_Eventos §7). Aquí solo la máquina de estados de la
//! conexión; qué ocurre con la columna al salir del mundo o volver a él es del núcleo.

use crate::datos::Reglas;
use crate::tipos::{AsentamientoId, AvanzadaId, CampamentoId, ColumnaId};
use base::{Instante, Punto};

/// Los sitios posibles de un héroe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ubicacion {
    /// Dentro de una plaza.
    Asentamiento(AsentamientoId),
    /// En el mapa, en su columna (personal o ejército).
    Columna(ColumnaId),
    /// Dentro de un campamento de mercenarios.
    Mercenarios(CampamentoId),
    /// Dentro de un pueblo extractor (Entrar con todo, 2026-10-22): con su columna entera, fuera del mapa, como una visita a una plaza.
    Pueblo(u64),
    /// Dentro de una avanzada (Entrar con todo, 2026-10-22): igual que en un pueblo.
    Avanzada(AvanzadaId),
    /// Fuera del mundo, solo si salió con su columna: dónde reaparecerá.
    Desconectado(Punto),
}

/// Qué ocurre cuando el héroe pide conectarse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Conexion {
    /// Estaba esperando para salir y se cancela la salida (sin eventos).
    CancelaLaSalida,
    /// Estaba fuera del mundo y vuelve.
    Vuelve,
    /// Ya estaba conectado y sin salida pendiente.
    SinCambios,
}

/// Qué hace el evento de salida al vencer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Salida {
    /// Sale del mundo ya.
    Sale,
    /// Combate: no sale; el primer paso tras acabar la batalla vuelve a probar (la desconexión pedida sigue anotada).
    EsperaAlFinDeLaBatalla,
    /// Le persiguen: quien huye desconectándose no gana nada, así que no sale hasta ese instante (`pedido + topeAplazamiento`).
    AplazadaHasta(Instante),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Presencia {
    pub ubicacion: Ubicacion,
    /// Cuándo saldrá del mundo, si ha pedido desconectarse y aún está dentro.
    pub desconecta_en: Option<Instante>,
    /// Fuera del mundo: solo puede conectarse.
    pub fuera: bool,
}

impl Presencia {
    pub fn nueva(ubicacion: Ubicacion) -> Self {
        Self { ubicacion, desconecta_en: None, fuera: false }
    }

    /// `desconectarse`: anota la salida a `ahora + retardoDesconexion` y devuelve ese instante (el núcleo programa el evento). Si ya está
    /// fuera o ya la había pedido, no hace nada (`None`).
    pub fn desconectarse(&mut self, r: &Reglas, ahora: Instante) -> Option<Instante> {
        if self.fuera || self.desconecta_en.is_some() {
            return None;
        }
        let sale_en = ahora + r.presencia.retardo_desconexion;
        self.desconecta_en = Some(sale_en);
        Some(sale_en)
    }

    pub fn conectarse(&mut self) -> Conexion {
        if self.fuera {
            self.fuera = false;
            Conexion::Vuelve
        } else if self.desconecta_en.take().is_some() {
            Conexion::CancelaLaSalida
        } else {
            Conexion::SinCambios
        }
    }

    /// Qué hace el evento de salida cuando vence, o `None` si ya no hay nada que hacer (se conectó antes, o ya está fuera).
    pub fn al_vencer_la_salida(&self, r: &Reglas, ahora: Instante, combate: bool, perseguido: bool) -> Option<Salida> {
        let sale_en = self.desconecta_en.filter(|_| !self.fuera)?;
        if combate {
            return Some(Salida::EsperaAlFinDeLaBatalla);
        }
        let pedido = sale_en - r.presencia.retardo_desconexion;
        let tope = pedido + r.presencia.tope_aplazamiento;
        Some(if perseguido && ahora < tope { Salida::AplazadaHasta(tope) } else { Salida::Sale })
    }

    /// Aplica la salida: sale del mundo y, si iba en su columna, queda anotado dónde reaparecerá. En una plaza o un campamento solo pierde el
    /// control.
    pub fn salir_del_mundo(&mut self, punto: Punto) {
        self.fuera = true;
        self.desconecta_en = None;
        if matches!(self.ubicacion, Ubicacion::Columna(_)) {
            self.ubicacion = Ubicacion::Desconectado(punto);
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn en_la_columna() -> Presencia {
        Presencia::nueva(Ubicacion::Columna(9))
    }

    #[test]
    fn desconectarse_programa_la_salida_a_dos_minutos_y_medio() {
        let r = Reglas::cargar();
        let mut p = en_la_columna();
        assert_eq!(p.desconectarse(&r, 1_000), Some(151_000));
        assert_eq!(p.desconecta_en, Some(151_000));
        assert_eq!(p.desconectarse(&r, 2_000), None, "pedirlo dos veces no cambia nada");
        assert_eq!(p.desconecta_en, Some(151_000));
    }

    #[test]
    fn conectarse_antes_de_la_salida_la_cancela_y_conectado_no_hace_nada() {
        let r = Reglas::cargar();
        let mut p = en_la_columna();
        assert_eq!(p.conectarse(), Conexion::SinCambios);
        p.desconectarse(&r, 0);
        assert_eq!(p.conectarse(), Conexion::CancelaLaSalida);
        assert_eq!(p.desconecta_en, None);
        assert_eq!(p.al_vencer_la_salida(&r, 150_000, false, false), None, "cancelada: el evento ya no hace nada");
    }

    #[test]
    fn al_vencer_sale_si_no_combate_y_no_le_persiguen() {
        let r = Reglas::cargar();
        let mut p = en_la_columna();
        p.desconectarse(&r, 0);
        assert_eq!(p.al_vencer_la_salida(&r, 150_000, false, false), Some(Salida::Sale));
        assert_eq!(p.al_vencer_la_salida(&r, 150_000, true, false), Some(Salida::EsperaAlFinDeLaBatalla), "quien combate no sale hasta que acaba");
        assert_eq!(p.desconecta_en, Some(150_000), "la petición sigue anotada");
    }

    #[test]
    fn quien_huye_desconectandose_no_gana_nada_aplazamiento_de_cinco_minutos() {
        let r = Reglas::cargar();
        let mut p = en_la_columna();
        p.desconectarse(&r, 10_000);
        // Pedida a 10 s: salida a 160 s; le persiguen, así que no sale hasta 10 s + 5 min.
        assert_eq!(p.al_vencer_la_salida(&r, 160_000, false, true), Some(Salida::AplazadaHasta(310_000)));
        // El evento se reprograma a ese instante y, al vencer, ya sale aunque le persigan (la petición original sigue anotada).
        assert_eq!(p.desconecta_en, Some(160_000));
        assert_eq!(p.al_vencer_la_salida(&r, 310_000, false, true), Some(Salida::Sale));
    }

    #[test]
    fn salir_con_la_columna_anota_donde_reaparece_y_en_una_plaza_solo_se_pierde_el_control() {
        let r = Reglas::cargar();
        let mut c = en_la_columna();
        c.desconectarse(&r, 0);
        c.salir_del_mundo(Punto::new(5_000, 7_000));
        assert_eq!((c.fuera, c.desconecta_en, c.ubicacion), (true, None, Ubicacion::Desconectado(Punto::new(5_000, 7_000))));
        assert_eq!(c.desconectarse(&r, 1), None, "fuera no se desconecta otra vez");
        assert_eq!(c.conectarse(), Conexion::Vuelve);
        assert!(!c.fuera);

        let mut d = Presencia::nueva(Ubicacion::Asentamiento(4));
        d.desconectarse(&r, 0);
        d.salir_del_mundo(Punto::default());
        assert_eq!((d.fuera, d.ubicacion), (true, Ubicacion::Asentamiento(4)), "dentro de una plaza no se mueve nada");
        assert_eq!(d.al_vencer_la_salida(&r, 150_000, false, false), None, "ya está fuera");
    }
}
