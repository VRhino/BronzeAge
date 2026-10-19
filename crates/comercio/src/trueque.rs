//! El trueque (Comercio_Caravanas §4): el acuerdo y su máquina de estados
//! `propuesto → activo → cumplido | expirado`, o `propuesto → rechazado`. Reglas puras; la autorización por cargo (Tesorero o
//! Gobernador, residente y dentro) la decide quien llama y llega como un booleano.

use crate::datos::{Reglas, ReglasTrueque};
use crate::reputacion::Suceso;
use crate::tipos::{AcuerdoId, AsentamientoId, Carga, FaccionId, Lado};
use asentamiento::datos::{razon, UNO};
use asentamiento::Recurso;
use base::{Instante, Mil};

/// Una línea de un lado: un recurso con lo pactado y lo ya entregado.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Linea {
    pub recurso: Recurso,
    pub cantidad_total: Mil,
    pub cantidad_entregada: Mil,
}

impl Linea {
    pub fn nueva(recurso: Recurso, cantidad_total: Mil) -> Self {
        Self { recurso, cantidad_total, cantidad_entregada: 0 }
    }
    /// Lo que falta por entregar (nunca negativo).
    pub fn faltante(&self) -> Mil {
        (self.cantidad_total - self.cantidad_entregada).max(0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EstadoTrueque {
    Propuesto,
    Activo,
    Cumplido,
    Expirado,
    Rechazado,
}

impl EstadoTrueque {
    pub fn terminado(self) -> bool {
        matches!(self, EstadoTrueque::Cumplido | EstadoTrueque::Expirado | EstadoTrueque::Rechazado)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorTrueque {
    /// Los dos lados son el mismo asentamiento.
    MismoAsentamiento,
    /// Un lado sin ninguna línea.
    LadoVacio,
    /// Una cantidad ≤ 0.
    CantidadNoPositiva,
    /// Un lado repite recurso (se suman en una sola línea).
    RecursoRepetido,
    /// El actor no es Tesorero ni Gobernador del asentamiento que le toca (residente y dentro).
    NoAutorizado,
    /// Solo se contesta una propuesta, y una sola vez.
    NoEstaPropuesto,
    /// El plazo para contestar ya venció (la caducidad la cierra a continuación).
    PropuestaVencida,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcuerdoTrueque {
    pub id: AcuerdoId,
    pub a: AsentamientoId,
    pub b: AsentamientoId,
    pub lineas_a: Vec<Linea>,
    pub lineas_b: Vec<Linea>,
    pub creado_en: Instante,
    /// Instante de la aceptación; la urgencia del reparto se cuenta desde aquí (decidido el 2026-10-18).
    pub aceptado_en: Option<Instante>,
    /// Propuesto: plazo para contestar. Activo: plazo para cumplir, desde el sí.
    pub expira_en: Instante,
    pub estado: EstadoTrueque,
}

/// Qué pasó cuando un acuerdo llegó a su `expira_en`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Vencimiento {
    /// Una propuesta sin contestar caduca sin penalizar a nadie.
    PropuestaCaducada,
    /// Un acuerdo activo venció: lo ya entregado no se devuelve; la reputación (solo entre Facciones distintas) baja en proporción a
    /// lo no entregado de cada lado.
    Incumplido { reputacion: Vec<(FaccionId, Suceso)> },
}

/// Lo que ocurre al saldarse el último abono.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cierre {
    pub entre_facciones: bool,
    /// `Σ cantidadTotal` por recurso, de los dos lados (lo que lee el mercado de mercenarios).
    pub intercambiado: Carga,
    /// `bonusTruequeCumplido` a cada Facción (solo entre Facciones distintas).
    pub reputacion: Vec<(FaccionId, Suceso)>,
    /// Facciones que ganan `NIVEL_FACCION.xp.truequeCumplido` (solo entre Facciones distintas).
    pub experiencia: Vec<FaccionId>,
    /// El acuerdo ya estaba `expirado` y un abono tardío lo saldó (rareza (a) de §15: se deja así).
    pub tras_vencer: bool,
}

fn validar(lineas: &[(Recurso, Mil)]) -> Result<Vec<Linea>, ErrorTrueque> {
    if lineas.is_empty() {
        return Err(ErrorTrueque::LadoVacio);
    }
    let mut vistos = Vec::new();
    for &(r, n) in lineas {
        if n <= 0 {
            return Err(ErrorTrueque::CantidadNoPositiva);
        }
        if vistos.contains(&r) {
            return Err(ErrorTrueque::RecursoRepetido);
        }
        vistos.push(r);
    }
    Ok(lineas.iter().map(|&(r, n)| Linea::nueva(r, n)).collect())
}

impl AcuerdoTrueque {
    /// `proponerTrueque` (§4.1): A ≠ B y líneas válidas. No se mira Mercado, stock ni relación entre las Facciones. `puede_proponer` es
    /// la autorización del actor sobre A. Nace `propuesto` con `expira_en = ahora + plazo`.
    #[allow(clippy::too_many_arguments)]
    pub fn proponer(
        r: &ReglasTrueque,
        id: AcuerdoId,
        a: AsentamientoId,
        b: AsentamientoId,
        lineas_a: &[(Recurso, Mil)],
        lineas_b: &[(Recurso, Mil)],
        ahora: Instante,
        puede_proponer: bool,
    ) -> Result<Self, ErrorTrueque> {
        if a == b {
            return Err(ErrorTrueque::MismoAsentamiento);
        }
        let (la, lb) = (validar(lineas_a)?, validar(lineas_b)?);
        if !puede_proponer {
            return Err(ErrorTrueque::NoAutorizado);
        }
        Ok(Self { id, a, b, lineas_a: la, lineas_b: lb, creado_en: ahora, aceptado_en: None, expira_en: ahora + r.plazo, estado: EstadoTrueque::Propuesto })
    }

    /// `aceptarTrueque` (§4.2): `activo`, y el plazo se **cuenta desde el sí**. Devuelve el nuevo `expira_en` (para agendar el evento).
    pub fn aceptar(&mut self, r: &ReglasTrueque, ahora: Instante, puede_contestar: bool) -> Result<Instante, ErrorTrueque> {
        self.comprobar_contestable(ahora, puede_contestar)?;
        self.estado = EstadoTrueque::Activo;
        self.aceptado_en = Some(ahora);
        self.expira_en = ahora + r.plazo;
        Ok(self.expira_en)
    }

    /// `rechazarTrueque`: queda `rechazado` (no se borra).
    pub fn rechazar(&mut self, ahora: Instante, puede_contestar: bool) -> Result<(), ErrorTrueque> {
        self.comprobar_contestable(ahora, puede_contestar)?;
        self.estado = EstadoTrueque::Rechazado;
        Ok(())
    }

    fn comprobar_contestable(&self, ahora: Instante, puede_contestar: bool) -> Result<(), ErrorTrueque> {
        if self.estado != EstadoTrueque::Propuesto {
            return Err(ErrorTrueque::NoEstaPropuesto);
        }
        if !puede_contestar {
            return Err(ErrorTrueque::NoAutorizado);
        }
        // DUDA: el TypeScript solo cerraba la propuesta en el paso del reparto, así que entre `expira_en` y ese paso aún se podía aceptar. Aquí
        // «vence» es `ahora >= expira_en` (Reloj_Y_Eventos §1): no se acepta lo que ya venció.
        if ahora >= self.expira_en {
            return Err(ErrorTrueque::PropuestaVencida);
        }
        Ok(())
    }

    pub fn lineas(&self, lado: Lado) -> &[Linea] {
        match lado {
            Lado::A => &self.lineas_a,
            Lado::B => &self.lineas_b,
        }
    }

    fn lineas_mut(&mut self, lado: Lado) -> &mut Vec<Linea> {
        match lado {
            Lado::A => &mut self.lineas_a,
            Lado::B => &mut self.lineas_b,
        }
    }

    /// Asentamiento del que sale el cargamento de `lado` (A sale de A hacia B; B al revés).
    pub fn origen_de(&self, lado: Lado) -> AsentamientoId {
        match lado {
            Lado::A => self.a,
            Lado::B => self.b,
        }
    }

    pub fn destino_de(&self, lado: Lado) -> AsentamientoId {
        self.origen_de(lado.otro())
    }

    /// Las líneas de `lado` a las que aún les falta algo, en el orden del acuerdo, con su faltante.
    pub fn pendiente(&self, lado: Lado) -> Vec<(Recurso, Mil)> {
        self.lineas(lado).iter().filter(|l| l.faltante() > 0).map(|l| (l.recurso, l.faltante())).collect()
    }

    /// ¿Todas las líneas de los dos lados saldadas?
    pub fn saldado(&self) -> bool {
        self.lineas_a.iter().chain(&self.lineas_b).all(|l| l.cantidad_entregada >= l.cantidad_total)
    }

    /// Fracción (ppm) de lo pactado en `lado` que no se entregó: `1 − Σ min(entregado, total) / Σ total`, sumando cantidades de recursos
    /// distintos sin ponderar por precio.
    // DUDA: el documento escribe `min(Σ entregado por línea, cantidadTotal)`; se lee línea a línea (lo que sobra en una no tapa lo que
    // falta en otra), que es la lectura que más penaliza.
    pub fn fraccion_incumplida(&self, lado: Lado) -> i64 {
        let (total, cumplido) = self.lineas(lado).iter().fold((0, 0), |(t, c), l| (t + l.cantidad_total, c + l.cantidad_entregada.min(l.cantidad_total)));
        if total <= 0 {
            return 0;
        }
        UNO - razon(cumplido, total)
    }

    /// Lo que se puede entregar a mano con una caravana adjunta (§4.5): por línea `min(lo que lleva, lo que falta)`. Vacío = error de
    /// entrega para quien llama.
    pub fn entregable(&self, lado: Lado, lleva: &Carga) -> Carga {
        let mut c = Carga::new();
        for l in self.lineas(lado) {
            let n = lleva.get(&l.recurso).copied().unwrap_or(0).min(l.faltante());
            if n > 0 {
                c.insert(l.recurso, n);
            }
        }
        c
    }

    /// El primer lado (A antes que B) que debe la Facción del ejército con líneas pendientes (`ladoPendienteParaEjercito`); solo con el
    /// trueque `activo`.
    pub fn lado_que_debe(&self, faccion: FaccionId, faccion_a: FaccionId, faccion_b: FaccionId) -> Option<Lado> {
        if self.estado != EstadoTrueque::Activo {
            return None;
        }
        [(Lado::A, faccion_a), (Lado::B, faccion_b)].into_iter().find(|&(l, f)| f == faccion && !self.pendiente(l).is_empty()).map(|(l, _)| l)
    }

    /// El instante en que hay que mirar este acuerdo (propuesto o activo); los terminados no caducan.
    pub fn proxima_caducidad(&self) -> Option<Instante> {
        matches!(self.estado, EstadoTrueque::Propuesto | EstadoTrueque::Activo).then_some(self.expira_en)
    }

    /// Al llegar a `expira_en` (§4.3 pasos 1 y 3): la propuesta caduca sin penalizar; el acuerdo activo vence y cada lado con
    /// `fraccionIncumplida > 0` recibe `penalizacionTruequeIncumplido × fracción` **si las Facciones son distintas**. `None` si aún no vence.
    pub fn vencer(&mut self, ahora: Instante, facciones: (FaccionId, FaccionId)) -> Option<Vencimiento> {
        if ahora < self.expira_en {
            return None;
        }
        match self.estado {
            EstadoTrueque::Propuesto => {
                self.estado = EstadoTrueque::Expirado;
                Some(Vencimiento::PropuestaCaducada)
            }
            EstadoTrueque::Activo => {
                self.estado = EstadoTrueque::Expirado;
                let mut reputacion = Vec::new();
                if facciones.0 != facciones.1 {
                    for (lado, f) in [(Lado::A, facciones.0), (Lado::B, facciones.1)] {
                        let fraccion = self.fraccion_incumplida(lado);
                        if fraccion > 0 {
                            reputacion.push((f, Suceso::TruequeIncumplido { fraccion }));
                        }
                    }
                }
                Some(Vencimiento::Incumplido { reputacion })
            }
            _ => None,
        }
    }

    /// Cancela el acuerdo por la ruina de una de las dos plazas o por la conquista de una (§10): `expirado` **sin penalizar a nadie**.
    /// Devuelve `true` si estaba vivo (`propuesto` o `activo`).
    pub fn cancelar_por_plaza(&mut self) -> bool {
        let vivo = matches!(self.estado, EstadoTrueque::Propuesto | EstadoTrueque::Activo);
        if vivo {
            self.estado = EstadoTrueque::Expirado;
        }
        vivo
    }

    /// `aplicarEntregaATrueque` (§4.4): abona lo que la caravana **llevaba al llegar** (tras los peajes) a las líneas del `lado` de su
    /// recurso; lo que ninguna línea pide no suma. No mira el estado salvo para no reabrir lo cerrado: un `activo` o un `expirado` que se
    /// salda pasa a `cumplido` (rareza (a) de §15, decisión del 2026-10-11) y reparte la recompensa aunque ya se hubiera penalizado.
    pub fn abonar(&mut self, lado: Lado, carga: &Carga, facciones: (FaccionId, FaccionId)) -> Option<Cierre> {
        // DUDA: en TypeScript un abono sobre un acuerdo ya `cumplido` volvía a emitir el cierre; aquí no se reemite (lo cumplido no se reabre).
        if !matches!(self.estado, EstadoTrueque::Activo | EstadoTrueque::Expirado) {
            return None;
        }
        for l in self.lineas_mut(lado) {
            if let Some(&n) = carga.get(&l.recurso) {
                l.cantidad_entregada += n.max(0);
            }
        }
        if !self.saldado() {
            return None;
        }
        let tras_vencer = self.estado == EstadoTrueque::Expirado;
        self.estado = EstadoTrueque::Cumplido;
        let mut intercambiado = Carga::new();
        for l in self.lineas_a.iter().chain(&self.lineas_b) {
            *intercambiado.entry(l.recurso).or_insert(0) += l.cantidad_total;
        }
        let entre_facciones = facciones.0 != facciones.1;
        let (reputacion, experiencia) = if entre_facciones {
            (vec![(facciones.0, Suceso::TruequeCumplido), (facciones.1, Suceso::TruequeCumplido)], vec![facciones.0, facciones.1])
        } else {
            (vec![], vec![])
        };
        Some(Cierre { entre_facciones, intercambiado, reputacion, experiencia, tras_vencer })
    }
}

/// `podarAcuerdosTerminados` (§4.7): quita el `cumplido`, `expirado` o `rechazado` con `ahora > expira_en + retención` que ninguna
/// caravana cite. Devuelve cuántos quitó (0 = el vector queda intacto).
pub fn podar(acuerdos: &mut Vec<AcuerdoTrueque>, r: &Reglas, ahora: Instante, citado: impl Fn(AcuerdoId) -> bool) -> usize {
    let antes = acuerdos.len();
    acuerdos.retain(|a| !(a.estado.terminado() && ahora > a.expira_en + r.trueque.retencion_terminados && !citado(a.id)));
    antes - acuerdos.len()
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::datos::PUNTO;
    use Recurso::*;

    const MIN: i64 = 60_000;

    fn reglas() -> Reglas {
        Reglas::cargar()
    }

    fn acuerdo(r: &Reglas) -> AcuerdoTrueque {
        AcuerdoTrueque::proponer(&r.trueque, 1, 10, 20, &[(Madera, 100_000), (Piedra, 50_000)], &[(Cobre, 30_000)], 1_000, true).unwrap()
    }

    #[test]
    fn proponer_valida_lados_cantidades_y_autorizacion() {
        let r = reglas();
        let p = |a, b, la: &[(Recurso, Mil)], lb: &[(Recurso, Mil)], aut| AcuerdoTrueque::proponer(&r.trueque, 1, a, b, la, lb, 0, aut);
        assert_eq!(p(1, 1, &[(Madera, 1)], &[(Piedra, 1)], true).unwrap_err(), ErrorTrueque::MismoAsentamiento);
        assert_eq!(p(1, 2, &[], &[(Piedra, 1)], true).unwrap_err(), ErrorTrueque::LadoVacio);
        assert_eq!(p(1, 2, &[(Madera, 1)], &[], true).unwrap_err(), ErrorTrueque::LadoVacio);
        assert_eq!(p(1, 2, &[(Madera, 0)], &[(Piedra, 1)], true).unwrap_err(), ErrorTrueque::CantidadNoPositiva);
        assert_eq!(p(1, 2, &[(Madera, 5), (Madera, 5)], &[(Piedra, 1)], true).unwrap_err(), ErrorTrueque::RecursoRepetido);
        assert_eq!(p(1, 2, &[(Madera, 5)], &[(Piedra, 1)], false).unwrap_err(), ErrorTrueque::NoAutorizado);
        assert!(p(1, 2, &[(Madera, 5)], &[(Madera, 5)], true).is_ok(), "el mismo recurso en lados distintos vale; el oro también");
        assert!(p(1, 2, &[(Oro, 5)], &[(Madera, 5)], true).is_ok());
    }

    #[test]
    fn nace_propuesto_con_plazo_de_un_dia() {
        let r = reglas();
        let a = acuerdo(&r);
        assert_eq!((a.estado, a.creado_en, a.aceptado_en), (EstadoTrueque::Propuesto, 1_000, None));
        assert_eq!(a.expira_en, 1_000 + 1440 * MIN);
        assert_eq!(a.proxima_caducidad(), Some(a.expira_en));
    }

    #[test]
    fn aceptar_cuenta_el_plazo_desde_el_si_y_guarda_aceptado_en() {
        let r = reglas();
        let mut a = acuerdo(&r);
        assert_eq!(a.aceptar(&r.trueque, 5_000, false), Err(ErrorTrueque::NoAutorizado));
        let al_filo = a.expira_en - MIN;
        assert_eq!(a.aceptar(&r.trueque, al_filo, true), Ok(al_filo + 1440 * MIN), "una propuesta contestada al filo nace con un día entero");
        assert_eq!((a.estado, a.aceptado_en), (EstadoTrueque::Activo, Some(al_filo)));
        assert_eq!(a.aceptar(&r.trueque, al_filo, true), Err(ErrorTrueque::NoEstaPropuesto), "se contesta una sola vez");
        assert_eq!(a.rechazar(al_filo, true), Err(ErrorTrueque::NoEstaPropuesto), "un activo no se retira");
    }

    #[test]
    fn rechazar_deja_el_acuerdo_y_una_propuesta_vencida_no_se_acepta() {
        let r = reglas();
        let mut a = acuerdo(&r);
        a.rechazar(2_000, true).unwrap();
        assert_eq!(a.estado, EstadoTrueque::Rechazado);
        assert_eq!(a.proxima_caducidad(), None);
        let mut b = acuerdo(&r);
        assert_eq!(b.aceptar(&r.trueque, b.expira_en, true), Err(ErrorTrueque::PropuestaVencida));
    }

    #[test]
    fn una_propuesta_sin_contestar_caduca_sin_penalizar() {
        let r = reglas();
        let mut a = acuerdo(&r);
        assert_eq!(a.vencer(a.expira_en - 1, (1, 2)), None);
        assert_eq!(a.vencer(a.expira_en, (1, 2)), Some(Vencimiento::PropuestaCaducada));
        assert_eq!(a.estado, EstadoTrueque::Expirado);
        assert_eq!(a.vencer(a.expira_en + 1, (1, 2)), None, "ya no vence otra vez");
    }

    #[test]
    fn vencer_penaliza_en_proporcion_a_lo_no_entregado_solo_entre_facciones() {
        let r = reglas();
        let mut a = acuerdo(&r);
        a.aceptar(&r.trueque, 2_000, true).unwrap();
        // Lado A: 100 de madera + 50 de piedra; entrega 100 de madera y 0 de piedra → falta 50/150 = 1/3. Lado B: nada → 100 %.
        let mut c = Carga::new();
        c.insert(Madera, 100_000);
        assert!(a.abonar(Lado::A, &c, (1, 2)).is_none());
        assert_eq!(a.fraccion_incumplida(Lado::A), 333_333);
        assert_eq!(a.fraccion_incumplida(Lado::B), UNO);
        let mut entre = a.clone();
        match entre.vencer(entre.expira_en, (1, 2)).unwrap() {
            Vencimiento::Incumplido { reputacion } => {
                let rr = &r.reputacion;
                assert_eq!(reputacion.len(), 2);
                assert_eq!(reputacion[0], (1, Suceso::TruequeIncumplido { fraccion: 333_333 }));
                assert_eq!(reputacion[0].1.delta(rr), -2_666_664);
                assert_eq!(reputacion[1].1.delta(rr), -8 * PUNTO);
            }
            v => panic!("{v:?}"),
        }
        assert_eq!(entre.estado, EstadoTrueque::Expirado);
        // Misma Facción: vence igual pero sin tocar la reputación.
        assert_eq!(a.vencer(a.expira_en, (3, 3)), Some(Vencimiento::Incumplido { reputacion: vec![] }));
    }

    #[test]
    fn un_lado_cumplido_del_todo_no_se_penaliza() {
        let r = reglas();
        let mut a = acuerdo(&r);
        a.aceptar(&r.trueque, 0, true).unwrap();
        let mut c = Carga::new();
        c.insert(Cobre, 30_000);
        a.abonar(Lado::B, &c, (1, 2));
        match a.vencer(a.expira_en, (1, 2)).unwrap() {
            Vencimiento::Incumplido { reputacion } => assert_eq!(reputacion.len(), 1, "solo el lado A debe"),
            v => panic!("{v:?}"),
        }
    }

    #[test]
    fn abonos_y_cierre_con_recompensa_entre_facciones() {
        let r = reglas();
        let mut a = acuerdo(&r);
        a.aceptar(&r.trueque, 0, true).unwrap();
        let mut ca = Carga::new();
        ca.insert(Madera, 100_000);
        ca.insert(Piedra, 50_000);
        ca.insert(Hierro, 9_999); // ninguna línea lo pide: no suma
        assert!(a.abonar(Lado::A, &ca, (1, 2)).is_none());
        assert!(!a.saldado());
        assert_eq!(a.pendiente(Lado::A), vec![]);
        assert_eq!(a.pendiente(Lado::B), vec![(Cobre, 30_000)]);
        let mut cb = Carga::new();
        cb.insert(Cobre, 30_000);
        let cierre = a.abonar(Lado::B, &cb, (1, 2)).expect("se salda");
        assert_eq!(a.estado, EstadoTrueque::Cumplido);
        assert!(cierre.entre_facciones && !cierre.tras_vencer);
        assert_eq!(cierre.intercambiado[&Madera], 100_000);
        assert_eq!(cierre.intercambiado.len(), 3, "solo los recursos pactados");
        assert_eq!(cierre.reputacion, vec![(1, Suceso::TruequeCumplido), (2, Suceso::TruequeCumplido)]);
        assert_eq!(cierre.experiencia, vec![1, 2]);
    }

    #[test]
    fn entre_plazas_de_la_misma_faccion_cumplir_no_da_reputacion_ni_experiencia() {
        let r = reglas();
        let mut a = acuerdo(&r);
        a.aceptar(&r.trueque, 0, true).unwrap();
        let mut todo = Carga::new();
        todo.insert(Madera, 100_000);
        todo.insert(Piedra, 50_000);
        a.abonar(Lado::A, &todo, (7, 7));
        let mut cb = Carga::new();
        cb.insert(Cobre, 30_000);
        let c = a.abonar(Lado::B, &cb, (7, 7)).unwrap();
        assert!(!c.entre_facciones);
        assert!(c.reputacion.is_empty() && c.experiencia.is_empty());
        assert_eq!(c.intercambiado.len(), 3, "el evento sale igual (con entreFacciones falso)");
    }

    #[test]
    fn rareza_un_expirado_que_se_salda_pasa_a_cumplido() {
        let r = reglas();
        let mut a = acuerdo(&r);
        a.aceptar(&r.trueque, 0, true).unwrap();
        a.vencer(a.expira_en, (1, 2));
        assert_eq!(a.estado, EstadoTrueque::Expirado);
        let mut ca = Carga::new();
        ca.insert(Madera, 100_000);
        ca.insert(Piedra, 50_000);
        a.abonar(Lado::A, &ca, (1, 2));
        let mut cb = Carga::new();
        cb.insert(Cobre, 30_000);
        let c = a.abonar(Lado::B, &cb, (1, 2)).unwrap();
        assert_eq!(a.estado, EstadoTrueque::Cumplido);
        assert!(c.tras_vencer, "reparte la recompensa aunque ya se hubiera penalizado");
        assert_eq!(c.reputacion.len(), 2);
        // Y no se reabre: otro abono sobre lo cumplido no reemite el cierre.
        assert!(a.abonar(Lado::B, &cb, (1, 2)).is_none());
    }

    #[test]
    fn un_abono_sobre_un_rechazado_o_propuesto_no_hace_nada() {
        let r = reglas();
        let mut a = acuerdo(&r);
        let mut c = Carga::new();
        c.insert(Madera, 100_000);
        assert!(a.abonar(Lado::A, &c, (1, 2)).is_none());
        assert_eq!(a.lineas_a[0].cantidad_entregada, 0);
    }

    #[test]
    fn la_fraccion_incumplida_no_se_deja_tapar_por_un_exceso() {
        let r = reglas();
        let mut a = acuerdo(&r);
        a.aceptar(&r.trueque, 0, true).unwrap();
        let mut c = Carga::new();
        c.insert(Madera, 500_000); // sobra mucho en una línea…
        a.abonar(Lado::A, &c, (1, 2));
        assert_eq!(a.fraccion_incumplida(Lado::A), 333_333, "…pero la piedra sigue sin llegar");
    }

    #[test]
    fn entrega_manual_por_linea_min_de_lo_que_lleva_y_lo_que_falta() {
        let r = reglas();
        let mut a = acuerdo(&r);
        a.aceptar(&r.trueque, 0, true).unwrap();
        let mut lleva = Carga::new();
        lleva.insert(Madera, 300_000);
        lleva.insert(Piedra, 20_000);
        lleva.insert(Cobre, 99_000);
        let e = a.entregable(Lado::A, &lleva);
        assert_eq!((e[&Madera], e[&Piedra], e.len()), (100_000, 20_000, 2));
        assert!(a.entregable(Lado::A, &Carga::new()).is_empty());
        // Qué lado debe una Facción: el primero (A antes que B) con pendientes.
        assert_eq!(a.lado_que_debe(1, 1, 2), Some(Lado::A));
        assert_eq!(a.lado_que_debe(2, 1, 2), Some(Lado::B));
        assert_eq!(a.lado_que_debe(1, 1, 1), Some(Lado::A), "misma Facción en los dos lados: A primero");
        assert_eq!(a.lado_que_debe(9, 1, 2), None);
        a.estado = EstadoTrueque::Propuesto;
        assert_eq!(a.lado_que_debe(1, 1, 2), None, "solo con el trueque activo");
    }

    #[test]
    fn cancelar_por_plaza_no_penaliza_y_solo_afecta_a_lo_vivo() {
        let r = reglas();
        let mut a = acuerdo(&r);
        assert!(a.cancelar_por_plaza());
        assert_eq!(a.estado, EstadoTrueque::Expirado);
        assert!(!a.cancelar_por_plaza());
        let mut c = acuerdo(&r);
        c.estado = EstadoTrueque::Cumplido;
        assert!(!c.cancelar_por_plaza());
        assert_eq!(c.estado, EstadoTrueque::Cumplido);
    }

    #[test]
    fn poda_con_retencion_y_respeta_lo_citado() {
        let r = reglas();
        let mut v: Vec<AcuerdoTrueque> = (1..=4).map(|i| AcuerdoTrueque { id: i, ..acuerdo(&r) }).collect();
        v[0].estado = EstadoTrueque::Cumplido;
        v[1].estado = EstadoTrueque::Expirado;
        v[2].estado = EstadoTrueque::Rechazado;
        v[3].estado = EstadoTrueque::Activo;
        let ret = r.trueque.retencion_terminados;
        let exp = v[0].expira_en;
        assert_eq!(podar(&mut v, &r, exp + ret, |_| false), 0, "justo en el límite aún se conserva (es `>`)");
        assert_eq!(podar(&mut v, &r, exp + ret + 1, |id| id == 2), 2, "el citado se queda; el activo nunca se poda");
        assert_eq!(v.iter().map(|a| a.id).collect::<Vec<_>>(), vec![2, 4]);
    }
}
