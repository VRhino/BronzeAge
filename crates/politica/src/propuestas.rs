//! Propuestas pendientes (Diplomacia_Facciones §2.4, §5.3, §6.1-§6.2; Doc 2.4 y 2.6): alianza o vasallaje, anexión y fusión, **en un solo almacén**.
//! Entre dos Facciones hay **una sola pendiente de cada clase, en cualquier sentido**. Una propuesta es **vigente** mientras `ahora < expira_en`; las caducadas
//! no hacen nada y se podan al añadir otra de su clase (sin eventos por espera, decisión 4). Al aceptar se revalida todo: el mundo cambió mientras esperaba.

use crate::faccion::Facciones;
use crate::relaciones::{ErrorDiplomacia, Relaciones, TipoRelacion, Tributo};
use crate::tipos::{FaccionId, HeroeId, PropuestaId, RelacionId};
use crate::{Politica, Reglas};
use base::Instante;
use comercio::datos::ReglasReputacion;
use std::collections::BTreeMap;

/// Lo que se puede pactar con una propuesta (la guerra no: se declara).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pacto {
    Alianza,
    Vasallaje,
}

impl Pacto {
    pub fn tipo(self) -> TipoRelacion {
        match self {
            Pacto::Alianza => TipoRelacion::Alianza,
            Pacto::Vasallaje => TipoRelacion::Vasallaje,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Clase {
    /// A ofrece y el Rey de B contesta; en un vasallaje A es la señora.
    Relacion { pacto: Pacto, a: FaccionId, b: FaccionId, tributo: Option<Tributo> },
    /// La absorbente propone.
    Anexion { absorbente: FaccionId, absorbida: FaccionId },
    /// Fija ya el nombre y el Rey de la Facción nueva.
    Fusion { a: FaccionId, b: FaccionId, nombre: String, rey: HeroeId },
}

impl Clase {
    /// El par (quien propone, quien contesta).
    pub fn par(&self) -> (FaccionId, FaccionId) {
        match *self {
            Clase::Relacion { a, b, .. } | Clase::Fusion { a, b, .. } => (a, b),
            Clase::Anexion { absorbente, absorbida } => (absorbente, absorbida),
        }
    }

    fn misma_clase(&self, o: &Clase) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(o)
    }

    pub fn nombra(&self, f: FaccionId) -> bool {
        let (a, b) = self.par();
        a == f || b == f
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Propuesta {
    pub id: PropuestaId,
    pub clase: Clase,
    /// El héroe que la hizo (en la fusión, el Rey de A).
    pub propuesta_por: HeroeId,
    pub creada_en: Instante,
    pub expira_en: Instante,
}

impl Propuesta {
    pub fn vigente(&self, ahora: Instante) -> bool {
        ahora < self.expira_en
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorPropuesta {
    Diplomacia(ErrorDiplomacia),
    /// Esa Facción no tiene Rey que pueda pactar, aceptar o consentir.
    SinRey(FaccionId),
    /// Esa Facción es vasalla de una tercera: su señora tiene que liberarla antes.
    VasallaDeTercera(FaccionId),
    /// La reputación de quien propone una alianza está por debajo de `umbralBajoParaEmbajador`.
    ReputacionBaja,
    /// Ya hay una pendiente de esa clase entre las dos.
    YaPendiente,
    NoExiste,
    Caducada,
    /// Alguna de las dos tiene una batalla abierta: no se acepta hasta que termine.
    BatallaAbierta,
    /// Fusión: el Rey nuevo ha de ser el Rey de una de las dos.
    ReyNoValido,
    NombreVacio,
    NombreDuplicado,
    /// Fusión: el Rey de A ya no es quien la propuso.
    ProponenteYaNoEsRey,
}

impl From<ErrorDiplomacia> for ErrorPropuesta {
    fn from(e: ErrorDiplomacia) -> Self {
        ErrorPropuesta::Diplomacia(e)
    }
}

/// Lo que deja aceptar una anexión o una fusión: el traslado lo hace `traslado`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Union {
    Anexion { absorbente: FaccionId, absorbida: FaccionId },
    Fusion { a: FaccionId, b: FaccionId, nombre: String, rey: HeroeId },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Propuestas {
    por_id: BTreeMap<PropuestaId, Propuesta>,
}

impl Propuestas {
    pub fn desde(p: impl IntoIterator<Item = Propuesta>) -> Self {
        Self { por_id: p.into_iter().map(|p| (p.id, p)).collect() }
    }

    pub fn get(&self, id: PropuestaId) -> Option<&Propuesta> {
        self.por_id.get(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Propuesta> {
        self.por_id.values()
    }

    /// La propuesta `id`, si sigue vigente.
    pub fn vigente(&self, id: PropuestaId, ahora: Instante) -> Result<&Propuesta, ErrorPropuesta> {
        let p = self.por_id.get(&id).ok_or(ErrorPropuesta::NoExiste)?;
        if p.vigente(ahora) {
            Ok(p)
        } else {
            Err(ErrorPropuesta::Caducada)
        }
    }

    fn pendiente_entre(&self, clase: &Clase, ahora: Instante) -> bool {
        let (a, b) = clase.par();
        self.por_id.values().any(|p| p.vigente(ahora) && p.clase.misma_clase(clase) && p.clase.nombra(a) && p.clase.nombra(b))
    }

    /// Añade una propuesta, podando antes las caducadas de su clase.
    fn anadir(&mut self, p: Propuesta, ahora: Instante) -> Result<&Propuesta, ErrorPropuesta> {
        if self.pendiente_entre(&p.clase, ahora) {
            return Err(ErrorPropuesta::YaPendiente);
        }
        self.por_id.retain(|_, q| q.vigente(ahora) || !q.clase.misma_clase(&p.clase));
        let id = p.id;
        self.por_id.insert(id, p);
        Ok(&self.por_id[&id])
    }

    pub(crate) fn quitar(&mut self, id: PropuestaId) -> Option<Propuesta> {
        self.por_id.remove(&id)
    }
}

fn con_rey(fs: &Facciones, f: FaccionId) -> Result<(), ErrorPropuesta> {
    match fs.get(f) {
        None => Err(ErrorDiplomacia::FaccionNoExiste.into()),
        Some(x) if x.rey.is_none() => Err(ErrorPropuesta::SinRey(f)),
        Some(_) => Ok(()),
    }
}

fn distintas_y_existentes(fs: &Facciones, a: FaccionId, b: FaccionId) -> Result<(), ErrorPropuesta> {
    if a == b {
        return Err(ErrorDiplomacia::MismaFaccion.into());
    }
    if !fs.existe(a) || !fs.existe(b) {
        return Err(ErrorDiplomacia::FaccionNoExiste.into());
    }
    Ok(())
}

/// `exigirRelacionPosible` (§5.3): par válido; las dos con Rey; en un vasallaje ninguna es ya vasalla de nadie; en una alianza, la reputación de A
/// (`reputacion_a`, ppm de punto) ≥ `umbralBajoParaEmbajador` (la de B no se mira).
pub fn exigir_relacion_posible(fs: &Facciones, rs: &Relaciones, rr: &ReglasReputacion, pacto: Pacto, a: FaccionId, b: FaccionId, reputacion_a: i64) -> Result<(), ErrorPropuesta> {
    rs.validar_par(fs, a, b)?;
    con_rey(fs, a)?;
    con_rey(fs, b)?;
    match pacto {
        Pacto::Vasallaje => match [a, b].into_iter().find(|&f| rs.senora_de(f).is_some()) {
            Some(f) => Err(ErrorPropuesta::VasallaDeTercera(f)),
            None => Ok(()),
        },
        Pacto::Alianza if reputacion_a < rr.umbral_bajo_para_embajador => Err(ErrorPropuesta::ReputacionBaja),
        Pacto::Alianza => Ok(()),
    }
}

/// `exigirAnexionPosible` (§6.1): distintas, existen, la absorbida tiene Rey y no es vasalla de una tercera (si su señora es la absorbente, vale).
pub fn exigir_anexion_posible(fs: &Facciones, rs: &Relaciones, absorbente: FaccionId, absorbida: FaccionId) -> Result<(), ErrorPropuesta> {
    distintas_y_existentes(fs, absorbente, absorbida)?;
    con_rey(fs, absorbida)?;
    match rs.senora_ajena(absorbida, absorbente) {
        Some(_) => Err(ErrorPropuesta::VasallaDeTercera(absorbida)),
        None => Ok(()),
    }
}

/// `exigirFusionPosible` (§6.2): distintas, existen, las dos con Rey, el Rey nuevo es el de una de las dos, nombre no vacío y libre (sin mirar mayúsculas; las
/// dos que se disuelven no cuentan) y ninguna vasalla de una tercera. Devuelve el nombre recortado.
pub fn exigir_fusion_posible(fs: &Facciones, rs: &Relaciones, a: FaccionId, b: FaccionId, nombre: &str, rey: HeroeId) -> Result<String, ErrorPropuesta> {
    distintas_y_existentes(fs, a, b)?;
    con_rey(fs, a)?;
    con_rey(fs, b)?;
    if fs.get(a).unwrap().rey != Some(rey) && fs.get(b).unwrap().rey != Some(rey) {
        return Err(ErrorPropuesta::ReyNoValido);
    }
    let nombre = nombre.trim();
    if nombre.is_empty() {
        return Err(ErrorPropuesta::NombreVacio);
    }
    if !fs.nombre_libre(nombre, &[a, b]) {
        return Err(ErrorPropuesta::NombreDuplicado);
    }
    for (f, otra) in [(a, b), (b, a)] {
        if rs.senora_ajena(f, otra).is_some() {
            return Err(ErrorPropuesta::VasallaDeTercera(f));
        }
    }
    Ok(nombre.to_string())
}

impl Politica {
    /// `proponerRelacion`: lo posible ahora (`exigir_relacion_posible`) y sin otra pendiente de relación entre las dos. Caduca a `RELACION_PROPUESTA.caducidadDias`;
    /// un vasallaje sin tributo lleva trigo a 0 (sin validar la cantidad).
    #[allow(clippy::too_many_arguments)]
    pub fn proponer_relacion(&mut self, r: &Reglas, rr: &ReglasReputacion, id: PropuestaId, pacto: Pacto, a: FaccionId, b: FaccionId, tributo: Option<Tributo>, por: HeroeId, reputacion_a: i64, ahora: Instante) -> Result<&Propuesta, ErrorPropuesta> {
        exigir_relacion_posible(&self.facciones, &self.relaciones, rr, pacto, a, b, reputacion_a)?;
        let tributo = (pacto == Pacto::Vasallaje).then(|| tributo.unwrap_or(Tributo::POR_DEFECTO));
        self.propuestas.anadir(Propuesta { id, clase: Clase::Relacion { pacto, a, b, tributo }, propuesta_por: por, creada_en: ahora, expira_en: ahora + r.caducidad_relacion }, ahora)
    }

    /// `responderRelacion`: vigente; rechazar la quita (`Ok(None)`); aceptar revalida, exige que ninguna de las dos tenga una batalla abierta, crea la relación
    /// activa con el id `relacion` y quita la propuesta.
    #[allow(clippy::too_many_arguments)]
    pub fn responder_relacion(&mut self, rr: &ReglasReputacion, id: PropuestaId, aceptar: bool, reputacion_a: i64, batalla_abierta: bool, relacion: RelacionId, ahora: Instante) -> Result<Option<RelacionId>, ErrorPropuesta> {
        let Clase::Relacion { pacto, a, b, tributo } = self.propuestas.vigente(id, ahora)?.clase.clone() else { return Err(ErrorPropuesta::NoExiste) };
        if aceptar {
            exigir_relacion_posible(&self.facciones, &self.relaciones, rr, pacto, a, b, reputacion_a)?;
            if batalla_abierta {
                return Err(ErrorPropuesta::BatallaAbierta);
            }
            self.relaciones.crear(&self.facciones, relacion, pacto.tipo(), a, b, tributo, ahora)?;
        }
        self.propuestas.quitar(id);
        Ok(aceptar.then_some(relacion))
    }

    /// `retirarRelacion`, `retirarAnexion`, `retirarFusion`: la quita mientras siga vigente (quién puede, la autorización).
    pub fn retirar_propuesta(&mut self, id: PropuestaId, ahora: Instante) -> Result<Propuesta, ErrorPropuesta> {
        self.propuestas.vigente(id, ahora)?;
        Ok(self.propuestas.quitar(id).expect("vigente"))
    }

    /// `proponerAnexion`: posible ahora y sin otra pendiente de anexión entre las dos. Caduca a `ANEXION.caducidadDias`.
    pub fn proponer_anexion(&mut self, r: &Reglas, id: PropuestaId, absorbente: FaccionId, absorbida: FaccionId, por: HeroeId, ahora: Instante) -> Result<&Propuesta, ErrorPropuesta> {
        exigir_anexion_posible(&self.facciones, &self.relaciones, absorbente, absorbida)?;
        self.propuestas.anadir(Propuesta { id, clase: Clase::Anexion { absorbente, absorbida }, propuesta_por: por, creada_en: ahora, expira_en: ahora + r.caducidad_anexion }, ahora)
    }

    /// `responderAnexion`: vigente; rechazar la quita; aceptar revalida, sin batallas abiertas, y la quita. No se vuelve a mirar que quien propuso conserve la
    /// autoridad. El traslado (`traslado::anexionar`) va después.
    pub fn responder_anexion(&mut self, id: PropuestaId, aceptar: bool, batalla_abierta: bool, ahora: Instante) -> Result<Option<Union>, ErrorPropuesta> {
        let Clase::Anexion { absorbente, absorbida } = self.propuestas.vigente(id, ahora)?.clase else { return Err(ErrorPropuesta::NoExiste) };
        if aceptar {
            exigir_anexion_posible(&self.facciones, &self.relaciones, absorbente, absorbida)?;
            if batalla_abierta {
                return Err(ErrorPropuesta::BatallaAbierta);
            }
        }
        self.propuestas.quitar(id);
        Ok(aceptar.then_some(Union::Anexion { absorbente, absorbida }))
    }

    /// `proponerFusion`: posible ahora y sin otra pendiente de fusión entre las dos; `propuesta_por` es el Rey de A. Caduca a `FUSION.caducidadDias`.
    #[allow(clippy::too_many_arguments)]
    pub fn proponer_fusion(&mut self, r: &Reglas, id: PropuestaId, a: FaccionId, b: FaccionId, nombre: &str, rey: HeroeId, ahora: Instante) -> Result<&Propuesta, ErrorPropuesta> {
        let nombre = exigir_fusion_posible(&self.facciones, &self.relaciones, a, b, nombre, rey)?;
        let por = self.facciones.get(a).and_then(|f| f.rey).expect("con Rey");
        self.propuestas.anadir(Propuesta { id, clase: Clase::Fusion { a, b, nombre, rey }, propuesta_por: por, creada_en: ahora, expira_en: ahora + r.caducidad_fusion }, ahora)
    }

    /// `responderFusion`: vigente; rechazar la quita; aceptar revalida todo, que el Rey de A siga siendo quien la propuso y que no haya batalla abierta, y la quita.
    /// El nacimiento de C y los traslados (`traslado::fusionar`) van después.
    pub fn responder_fusion(&mut self, id: PropuestaId, aceptar: bool, batalla_abierta: bool, ahora: Instante) -> Result<Option<Union>, ErrorPropuesta> {
        let p = self.propuestas.vigente(id, ahora)?.clone();
        let Clase::Fusion { a, b, nombre, rey } = p.clase else { return Err(ErrorPropuesta::NoExiste) };
        if aceptar {
            exigir_fusion_posible(&self.facciones, &self.relaciones, a, b, &nombre, rey)?;
            if self.facciones.get(a).and_then(|f| f.rey) != Some(p.propuesta_por) {
                return Err(ErrorPropuesta::ProponenteYaNoEsRey);
            }
            if batalla_abierta {
                return Err(ErrorPropuesta::BatallaAbierta);
            }
        }
        self.propuestas.quitar(id);
        Ok(aceptar.then_some(Union::Fusion { a, b, nombre, rey }))
    }
}

#[cfg(test)]
mod pruebas {
    //! Casos de las reglas de `engine/{diplomacia,anexion,fusion}.ts` que prueban `comandosDiplomacia.test.ts`, `comandosAnexion.test.ts`,
    //! `comandosFusion.test.ts` y `diplomaciaDecisiones.test.ts` (la fusión revisa el nombre como `crearFaccion`).
    use super::*;
    use crate::datos::DIA;
    use crate::relaciones::pruebas::facciones;
    use asentamiento::Recurso;
    use comercio::datos::{Reglas as ReglasComercio, PUNTO};

    fn mundo(n: FaccionId) -> Politica {
        Politica { facciones: facciones(n), ..Default::default() }
    }

    fn rr() -> ReglasReputacion {
        ReglasComercio::cargar().reputacion
    }

    #[test]
    fn proponer_y_aceptar_una_alianza() {
        let (r, rr) = (Reglas::default(), rr());
        let mut m = mundo(2);
        let p = m.proponer_relacion(&r, &rr, 1, Pacto::Alianza, 1, 2, None, 10, 0, 0).unwrap();
        assert_eq!((p.expira_en, p.propuesta_por), (3 * DIA, 10));
        assert_eq!(m.proponer_relacion(&r, &rr, 2, Pacto::Alianza, 2, 1, None, 20, 0, 0).unwrap_err(), ErrorPropuesta::YaPendiente, "una sola en cualquier sentido");
        assert_eq!(m.responder_relacion(&rr, 1, true, 0, false, 50, DIA), Ok(Some(50)));
        assert!(m.relaciones.estan_aliadas(1, 2));
        assert_eq!(m.propuestas.get(1), None);
    }

    #[test]
    fn rechazar_quita_y_la_caducada_no_se_contesta() {
        let (r, rr) = (Reglas::default(), rr());
        let mut m = mundo(2);
        m.proponer_relacion(&r, &rr, 1, Pacto::Alianza, 1, 2, None, 10, 0, 0).unwrap();
        assert_eq!(m.responder_relacion(&rr, 1, false, 0, false, 50, 0), Ok(None));
        assert!(m.relaciones.activas().next().is_none() && m.propuestas.get(1).is_none());
        m.proponer_relacion(&r, &rr, 2, Pacto::Alianza, 1, 2, None, 10, 0, 0).unwrap();
        assert_eq!(m.responder_relacion(&rr, 2, true, 0, false, 50, 3 * DIA), Err(ErrorPropuesta::Caducada));
        assert_eq!(m.responder_relacion(&rr, 9, true, 0, false, 50, 0), Err(ErrorPropuesta::NoExiste));
        // La caducada ya no cuenta como pendiente, y se poda al añadir otra.
        m.proponer_relacion(&r, &rr, 3, Pacto::Alianza, 2, 1, None, 20, 0, 3 * DIA).unwrap();
        assert_eq!(m.propuestas.iter().map(|p| p.id).collect::<Vec<_>>(), vec![3]);
    }

    #[test]
    fn la_alianza_pide_la_reputacion_de_quien_propone() {
        let (r, rr) = (Reglas::default(), rr());
        let mut m = mundo(2);
        assert_eq!(m.proponer_relacion(&r, &rr, 1, Pacto::Alianza, 1, 2, None, 10, -41 * PUNTO, 0).unwrap_err(), ErrorPropuesta::ReputacionBaja);
        assert!(m.proponer_relacion(&r, &rr, 1, Pacto::Alianza, 1, 2, None, 10, -40 * PUNTO, 0).is_ok(), "en el umbral, sí");
        // Al aceptar se vuelve a mirar.
        assert_eq!(m.responder_relacion(&rr, 1, true, -41 * PUNTO, false, 50, 0), Err(ErrorPropuesta::ReputacionBaja));
        assert!(m.propuestas.get(1).is_some(), "un rechazo de la regla no la consume");
    }

    #[test]
    fn el_vasallaje_lleva_tributo_y_no_lo_pacta_quien_ya_es_vasalla() {
        let (r, rr) = (Reglas::default(), rr());
        let mut m = mundo(3);
        m.proponer_relacion(&r, &rr, 1, Pacto::Vasallaje, 1, 2, None, 10, 0, 0).unwrap();
        assert_eq!(m.responder_relacion(&rr, 1, true, 0, false, 50, 0), Ok(Some(50)));
        assert_eq!(m.relaciones.get(50).unwrap().tributo, Some(Tributo::POR_DEFECTO));
        // 2 ya es vasalla de 1: ni de señora ni de vasalla con 3.
        assert_eq!(m.proponer_relacion(&r, &rr, 2, Pacto::Vasallaje, 2, 3, None, 20, 0, 0).unwrap_err(), ErrorPropuesta::VasallaDeTercera(2));
        assert_eq!(m.proponer_relacion(&r, &rr, 2, Pacto::Vasallaje, 3, 2, None, 30, 0, 0).unwrap_err(), ErrorPropuesta::VasallaDeTercera(2));
        let t = Tributo { recurso: Recurso::Madera, cantidad_por_minuto: 10_000 };
        let p = m.proponer_relacion(&r, &rr, 3, Pacto::Vasallaje, 1, 3, Some(t), 10, 0, 0).unwrap();
        assert_eq!(p.clase, Clase::Relacion { pacto: Pacto::Vasallaje, a: 1, b: 3, tributo: Some(t) });
    }

    #[test]
    fn aceptar_revalida_el_par_y_las_batallas() {
        let (r, rr) = (Reglas::default(), rr());
        let mut m = mundo(2);
        m.proponer_relacion(&r, &rr, 1, Pacto::Alianza, 1, 2, None, 10, 0, 0).unwrap();
        assert_eq!(m.responder_relacion(&rr, 1, true, 0, true, 50, 0), Err(ErrorPropuesta::BatallaAbierta));
        m.relaciones.declarar_guerra(&m.facciones, 1, 2, 0, &mut || 60).unwrap();
        assert_eq!(m.responder_relacion(&rr, 1, true, 0, false, 50, 0), Err(ErrorPropuesta::Diplomacia(ErrorDiplomacia::YaHayRelacion)));
        assert_eq!(m.retirar_propuesta(1, 0).unwrap().id, 1);
        assert_eq!(m.retirar_propuesta(1, 0), Err(ErrorPropuesta::NoExiste));
    }

    #[test]
    fn la_relacion_pide_rey_en_las_dos() {
        let (r, rr) = (Reglas::default(), rr());
        let mut m = mundo(2);
        m.facciones.dejar(20, 0, false).unwrap();
        assert_eq!(m.proponer_relacion(&r, &rr, 1, Pacto::Alianza, 1, 2, None, 10, 0, 0).unwrap_err(), ErrorPropuesta::SinRey(2));
    }

    #[test]
    fn anexion_propuesta_respuesta_y_vasalla_de_tercera() {
        let r = Reglas::default();
        let mut m = mundo(3);
        assert_eq!(m.proponer_anexion(&r, 1, 1, 1, 10, 0).unwrap_err(), ErrorPropuesta::Diplomacia(ErrorDiplomacia::MismaFaccion));
        m.relaciones.crear(&m.facciones, 70, TipoRelacion::Vasallaje, 3, 2, None, 0).unwrap();
        assert_eq!(m.proponer_anexion(&r, 1, 1, 2, 10, 0).unwrap_err(), ErrorPropuesta::VasallaDeTercera(2));
        assert!(m.proponer_anexion(&r, 1, 3, 2, 30, 0).is_ok(), "su propia señora sí puede absorberla");
        assert_eq!(m.proponer_anexion(&r, 2, 2, 3, 20, 0).unwrap_err(), ErrorPropuesta::YaPendiente);
        assert_eq!(m.responder_anexion(1, true, true, 0), Err(ErrorPropuesta::BatallaAbierta));
        assert_eq!(m.responder_anexion(1, true, false, DIA), Ok(Some(Union::Anexion { absorbente: 3, absorbida: 2 })));
        assert!(m.propuestas.get(1).is_none());
    }

    #[test]
    fn la_fusion_revisa_el_nombre_como_crear_faccion() {
        // `diplomaciaDecisiones.test.ts`: rechaza el nombre de una tercera (sin mirar mayúsculas) y uno vacío; acepta uno libre y el de una de las dos.
        let m = mundo(3);
        let (fs, rs) = (&m.facciones, &m.relaciones);
        assert_eq!(exigir_fusion_posible(fs, rs, 1, 2, " f3 ", 10), Err(ErrorPropuesta::NombreDuplicado));
        assert_eq!(exigir_fusion_posible(fs, rs, 1, 2, "   ", 10), Err(ErrorPropuesta::NombreVacio));
        assert_eq!(exigir_fusion_posible(fs, rs, 1, 2, "Delta", 10), Ok("Delta".into()));
        assert_eq!(exigir_fusion_posible(fs, rs, 1, 2, "f1", 10), Ok("f1".into()));
        assert_eq!(exigir_fusion_posible(fs, rs, 1, 2, "Delta", 30), Err(ErrorPropuesta::ReyNoValido));
    }

    #[test]
    fn aceptar_una_fusion_exige_que_el_rey_de_a_siga_siendo_quien_la_propuso() {
        let r = Reglas::default();
        let mut m = mundo(2);
        m.facciones.otorgar(1, 11).unwrap();
        let p = m.proponer_fusion(&r, 1, 1, 2, " Delta ", 20, 0).unwrap();
        assert_eq!((p.propuesta_por, p.clase.clone()), (10, Clase::Fusion { a: 1, b: 2, nombre: "Delta".into(), rey: 20 }));
        m.facciones.asignar_rey(1, 11).unwrap();
        assert_eq!(m.responder_fusion(1, true, false, 0), Err(ErrorPropuesta::ProponenteYaNoEsRey));
        m.facciones.asignar_rey(1, 10).unwrap();
        assert_eq!(m.responder_fusion(1, true, false, 0), Ok(Some(Union::Fusion { a: 1, b: 2, nombre: "Delta".into(), rey: 20 })));
    }

    #[test]
    fn cada_clase_tiene_su_propia_pendiente() {
        let (r, rr) = (Reglas::default(), rr());
        let mut m = mundo(2);
        m.proponer_relacion(&r, &rr, 1, Pacto::Alianza, 1, 2, None, 10, 0, 0).unwrap();
        assert!(m.proponer_anexion(&r, 2, 1, 2, 10, 0).is_ok());
        assert!(m.proponer_fusion(&r, 3, 2, 1, "Delta", 20, 0).is_ok());
        assert_eq!(m.responder_anexion(1, true, false, 0), Err(ErrorPropuesta::NoExiste), "la 1 es de relación");
    }
}
