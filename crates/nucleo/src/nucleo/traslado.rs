//! La anexión, la fusión y el traslado de una Facción que desaparece (sistema 5, tanda 5F; Diplomacia_Facciones §6, Doc 2.6). Las reglas puras son de `politica` (`propuestas`: proponer,
//! responder y retirar con su revalidación; `traslado`: el plan de lo político, con `planificar` y `aplicar`); aquí, las órdenes y **todo lo que no es político** del traslado, en la misma
//! transición en que se acepta.
//!
//! # El traslado (`trasladarFaccion(origen → destino)`, §6.3)
//! `politica` ya pasó las relaciones (los vasallajes en que el origen es la señora pasan al destino, todo lo demás se rompe sin reputación), descartó las propuestas que nombraban al
//! origen, sumó sus ciudadanos al final de los del destino (en su orden: la sucesión del trono sigue ese orden), redirigió los `derrotadaPor` y borró el origen. El núcleo, después:
//! - **la ciudadanía**: la copia de cada ciudadano (`HeroeNucleo::faccion`), la columna de quien va solo y la demanda de bandidos del campamento donde reside (`tras_cambiar_de_faccion`);
//! - **las plazas**: cambian de Facción con sus cargos locales, su guarnición, sus residentes, su flota, sus pueblos y sus avanzadas (todo cuelga de la plaza); pierden la marca de capital
//!   del origen y el resto oculto de su tributo; en la anexión son del destino **desde ahora** (`de_la_faccion_desde`: la capital del destino no cambia), en la fusión conservan su antigüedad;
//! - **las columnas, las convocatorias, las Caravanas de Fundación y las rutas de la red** del origen pasan al destino; las caravanas comerciales y los trueques van con su plaza (Q5: los trueques
//!   siguen; los que quedan dentro de la misma Facción ya no mueven la reputación al cumplirse o vencer, porque `comercio` la mira con la Facción de cada plaza en ese instante);
//! - **la memoria de la niebla** se funde al cerrar la versión, después de grabar lo que vieron los ojos del origen (`fundir_memorias`): lo explorado por unión, las fichas y los Informes con
//!   los del destino delante, sin los de sitios ya propios, y las Miradas pasan al destino;
//! - **la reputación y las tecnologías del andamiaje** del origen se borran (en la fusión, la Facción nueva nace con la mayor reputación y la unión de las tecnologías);
//! - **la puerta**: quien está dentro de un lugar que ya no le admite (un aliado del origen dentro de una plaza suya, que ya no es aliado del dueño) sale al mapa con lo suyo, con el motivo
//!   `cambia_de_dueno` de la conquista (5F-9).
//!
//! Lo que nunca se traslada es una batalla: no se acepta con una batalla abierta de alguna de las dos ni con un héroe suyo combatiendo en una, como bando o incorporado (§6.1, 5F-3). La experiencia y el Rey los decide `politica` (la fusión: la mayor y el electo).
//! Nada de esto deja citas en la cola.

use super::*;
use crate::estado::Dato;
use crate::memoria::Sitio;
use militar::presencia::Ubicacion;
use politica::propuestas::{Clase as ClaseDePropuesta, ErrorPropuesta, Union};
use politica::traslado::{Cambio, Plan};
use std::collections::BTreeSet;

fn de_union(e: ErrorPropuesta) -> (&'static str, &'static str) {
    use rechazos::*;
    match e {
        ErrorPropuesta::Diplomacia(d) => super::diplomacia::de_diplomacia(d),
        ErrorPropuesta::SinRey(_) => (DIPLOMACIA_SIN_REY, "una de las dos Facciones no tiene Rey que acepte o consienta"),
        ErrorPropuesta::VasallaDeTercera(_) => (DIPLOMACIA_VASALLA_DE_TERCERA, "una de las dos es vasalla de una tercera: su señora tiene que liberarla antes"),
        ErrorPropuesta::ReputacionBaja => (DIPLOMACIA_REPUTACION_BAJA, "con esa reputación nadie pacta"),
        ErrorPropuesta::YaPendiente => (DIPLOMACIA_YA_PENDIENTE, "ya hay una propuesta de esa clase pendiente entre las dos"),
        ErrorPropuesta::NoExiste => (DIPLOMACIA_PROPUESTA_DESCONOCIDA, "esa propuesta no existe"),
        ErrorPropuesta::Caducada => (DIPLOMACIA_PROPUESTA_CADUCADA, "la propuesta caducó"),
        ErrorPropuesta::BatallaAbierta => (DIPLOMACIA_BATALLA_ABIERTA, "alguna de las dos combate en una batalla abierta: se contesta cuando termine"),
        ErrorPropuesta::ReyNoValido => (DIPLOMACIA_REY_NO_VALIDO, "el Rey de la Facción nueva ha de ser el Rey de una de las dos"),
        ErrorPropuesta::NombreVacio => (FACCION_NOMBRE_VACIO, "la Facción nueva necesita un nombre"),
        ErrorPropuesta::NombreDuplicado => (FACCION_NOMBRE_DUPLICADO, "otra Facción ya se llama así"),
        ErrorPropuesta::ProponenteYaNoEsRey => (DIPLOMACIA_PROPONENTE_YA_NO_ES_REY, "la Facción que la propuso ya no tiene el Rey que la propuso"),
    }
}

/// Las dos clases de propuesta de esta tanda.
#[derive(Clone, Copy, PartialEq, Eq)]
enum DeUnion {
    Anexion,
    Fusion,
}

impl Nucleo {
    // ───────────────────────────── Órdenes ─────────────────────────────

    /// Las órdenes de la anexión y la fusión (168-173). Los rechazos y «sin cambios» salen como `Err` sin haber tocado el estado.
    pub(super) fn ejecutar_anexion_y_fusion(&mut self, t: Instante, p: &Peticion) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |(c, m): (&'static str, &'static str)| Respuesta::rechazo(c, m, v);
        let r = self.estado.reglas.economia.politica.clone();
        match &p.comando {
            Comando::ProponerAnexion { heroe, absorbente, absorbida } => {
                self.exigir_autoridad(p, *heroe, *absorbente)?;
                let id = self.estado.siguiente_id;
                self.estado.politica.proponer_anexion(&r, id, *absorbente, *absorbida, *heroe, t).map_err(|e| rechazo(de_union(e)))?;
                self.gastar_ids_hasta(Some(id));
                self.tr.tocar_todos(Clase::Faccion, [u64::from(*absorbente), u64::from(*absorbida)]);
                let datos = vec![("propuesta", id as i64), ("faccionA", i64::from(*absorbente)), ("faccionB", i64::from(*absorbida)), ("heroe", *heroe as i64)];
                self.hecho_de_relacion(t, "diplomacia.anexion_propuesta", vec![*absorbente, *absorbida], datos, vec![]);
                Ok(vec![id])
            }
            Comando::ResponderAnexion { heroe, propuesta, aceptar } => {
                let (a, b) = self.propuesta_de_union(*propuesta, t, DeUnion::Anexion)?;
                self.exigir_rey(p, *heroe, b)?;
                let batalla = self.union_en_batalla(a, b);
                let union = self.estado.politica.responder_anexion(*propuesta, *aceptar, batalla, t).map_err(|e| rechazo(de_union(e)))?;
                self.tr.tocar_todos(Clase::Faccion, [u64::from(a), u64::from(b)]);
                let datos = vec![("propuesta", *propuesta as i64), ("faccionA", i64::from(a)), ("faccionB", i64::from(b)), ("heroe", *heroe as i64)];
                let Some(Union::Anexion { absorbente, absorbida }) = union else {
                    self.hecho_de_relacion(t, "diplomacia.anexion_rechazada", vec![a, b], datos, vec![]);
                    return Ok(vec![]);
                };
                let plan = politica::traslado::anexionar(&mut self.estado.politica, absorbente, absorbida);
                self.trasladar(t, &plan, true);
                self.hecho_de_relacion(t, "diplomacia.anexion", vec![absorbente, absorbida], datos, vec![]);
                Ok(vec![])
            }
            Comando::RetirarAnexion { heroe, propuesta } => {
                let (a, b) = self.propuesta_de_union(*propuesta, t, DeUnion::Anexion)?;
                self.exigir_autoridad(p, *heroe, a)?;
                self.estado.politica.retirar_propuesta(*propuesta, t).map_err(|e| rechazo(de_union(e)))?;
                self.tr.tocar_todos(Clase::Faccion, [u64::from(a), u64::from(b)]);
                let datos = vec![("propuesta", *propuesta as i64), ("faccionA", i64::from(a)), ("faccionB", i64::from(b)), ("heroe", *heroe as i64)];
                self.hecho_de_relacion(t, "diplomacia.anexion_retirada", vec![a, b], datos, vec![]);
                Ok(vec![])
            }
            Comando::ProponerFusion { heroe, a, b, nombre, rey } => {
                self.exigir_rey(p, *heroe, *a)?;
                let id = self.estado.siguiente_id;
                let nombre = self.estado.politica.proponer_fusion(&r, id, *a, *b, nombre, *rey, t).map_err(|e| rechazo(de_union(e)))?.clase.clone();
                let ClaseDePropuesta::Fusion { nombre, .. } = nombre else { unreachable!("es de fusión") };
                self.gastar_ids_hasta(Some(id));
                self.tr.tocar_todos(Clase::Faccion, [u64::from(*a), u64::from(*b)]);
                let datos = vec![("propuesta", id as i64), ("faccionA", i64::from(*a)), ("faccionB", i64::from(*b)), ("heroe", *heroe as i64), ("rey", *rey as i64)];
                self.hecho_de_relacion(t, "diplomacia.fusion_propuesta", vec![*a, *b], datos, vec![("nombre", Dato::Texto(nombre))]);
                Ok(vec![id])
            }
            Comando::ResponderFusion { heroe, propuesta, aceptar } => {
                let (a, b) = self.propuesta_de_union(*propuesta, t, DeUnion::Fusion)?;
                self.exigir_rey(p, *heroe, b)?;
                let batalla = self.union_en_batalla(a, b);
                let union = self.estado.politica.responder_fusion(*propuesta, *aceptar, batalla, t).map_err(|e| rechazo(de_union(e)))?;
                self.tr.tocar_todos(Clase::Faccion, [u64::from(a), u64::from(b)]);
                let mut datos = vec![("propuesta", *propuesta as i64), ("faccionA", i64::from(a)), ("faccionB", i64::from(b)), ("heroe", *heroe as i64)];
                let Some(union) = union else {
                    self.hecho_de_relacion(t, "diplomacia.fusion_rechazada", vec![a, b], datos, vec![]);
                    return Ok(vec![]);
                };
                let c = self.fusionar(t, &union);
                let Union::Fusion { nombre, rey, .. } = union else { unreachable!("es de fusión") };
                datos.extend([("faccion", i64::from(c)), ("rey", rey as i64)]);
                self.hecho_de_relacion(t, "diplomacia.fusion", vec![a, b, c], datos, vec![("nombre", Dato::Texto(nombre))]);
                Ok(vec![u64::from(c)])
            }
            Comando::RetirarFusion { heroe, propuesta } => {
                let (a, b) = self.propuesta_de_union(*propuesta, t, DeUnion::Fusion)?;
                self.exigir_rey(p, *heroe, a)?;
                self.estado.politica.retirar_propuesta(*propuesta, t).map_err(|e| rechazo(de_union(e)))?;
                self.tr.tocar_todos(Clase::Faccion, [u64::from(a), u64::from(b)]);
                let datos = vec![("propuesta", *propuesta as i64), ("faccionA", i64::from(a)), ("faccionB", i64::from(b)), ("heroe", *heroe as i64)];
                self.hecho_de_relacion(t, "diplomacia.fusion_retirada", vec![a, b], datos, vec![]);
                Ok(vec![])
            }
            _ => unreachable!("solo se llama con órdenes de anexión y fusión"),
        }
    }

    /// ¿Impide una batalla abierta la anexión o la fusión de `a` y `b`? Sí si alguna de las dos ataca o defiende una batalla activa (la de las relaciones, 5D-7) o si **algún héroe de las dos**
    /// combate en una, como bando o incorporado (5F-3, decisión del autor del 2026-10-25): cambiaría de Facción a mitad de la batalla.
    fn union_en_batalla(&self, a: u32, b: u32) -> bool {
        let t = self.estado.ahora;
        let de_las_dos = |h: u64| self.estado.militar.heroes.get(&h).and_then(|x| x.faccion).is_some_and(|f| f == a || f == b);
        self.faccion_en_batalla(a) || self.faccion_en_batalla(b) || self.batallas().any(|x| x.activa(t) && x.heroes_que_combaten().any(de_las_dos))
    }

    /// La propuesta `id`, vigente y de la clase de la orden: `(quien propone, quien contesta)`.
    fn propuesta_de_union(&self, id: u64, t: Instante, clase: DeUnion) -> Result<(u32, u32), Respuesta> {
        let v = self.estado.version;
        let p = self.estado.politica.propuestas.vigente(id, t).map_err(|e| {
            let (c, m) = de_union(e);
            Respuesta::rechazo(c, m, v)
        })?;
        match (&p.clase, clase) {
            (ClaseDePropuesta::Anexion { absorbente, absorbida }, DeUnion::Anexion) => Ok((*absorbente, *absorbida)),
            (ClaseDePropuesta::Fusion { a, b, .. }, DeUnion::Fusion) => Ok((*a, *b)),
            _ => Err(Respuesta::rechazo(rechazos::DIPLOMACIA_PROPUESTA_DESCONOCIDA, if clase == DeUnion::Anexion { "esa propuesta no es de anexión" } else { "esa propuesta no es de fusión" }, v)),
        }
    }

    // ───────────────────────────── La fusión ─────────────────────────────

    /// Nace C con el id siguiente a la mayor Facción del mundo (§6.2): lo que es solo de la fusión (la reputación mayor de las dos, la unión de sus tecnologías del andamiaje y la marca de
    /// capital que había designado el Rey electo, que pasa a C; la de la otra se borra) se pone **antes** de los dos traslados, que borran lo del origen. Devuelve el id de C.
    fn fusionar(&mut self, t: Instante, union: &Union) -> u32 {
        let Union::Fusion { a, b, rey, .. } = *union else { unreachable!("es de fusión") };
        let c = self.siguiente_faccion();
        let de_rey_electo = if self.estado.politica.facciones.get(a).and_then(|f| f.rey) == Some(rey) { a } else { b };
        // La reputación mayor, en el instante de la fusión (con sus alianzas en 0: C nace sin relaciones salvo las vasallas, que no cuentan para el ritmo).
        let rr = self.estado.reglas.economia.comercio.reputacion.clone();
        let rep = self.reputacion(a).max(self.reputacion(b));
        if rep != 0 || self.estado.comercio.reputacion.contains_key(&a) || self.estado.comercio.reputacion.contains_key(&b) {
            self.estado.comercio.reputacion.insert(c, comercio::reputacion::Reputacion::desde(&rr, rep, t, 0));
            self.programar_reputacion(c, t);
        }
        // Las tecnologías: la unión de los tres conjuntos (decisión técnica 13 del plan del sistema 6; TS `fusion.ts`). Lo revelado que ya apareció deja de estar revelado.
        let tec = &mut self.estado.tecnologia;
        if tec.por_faccion.contains_key(&a) || tec.por_faccion.contains_key(&b) {
            let rt = &self.estado.reglas.economia.tecnologia;
            let (x, y) = (tec.de(rt, a), tec.de(rt, b));
            let aparecidas = x.aparecidas.union(y.aparecidas);
            let reveladas = tecnologia::Conjunto(x.reveladas.union(y.reveladas).0 & !aparecidas.0);
            tec.por_faccion.insert(c, tecnologia::TecnologiasFaccion { aparecidas, adoptadas: x.adoptadas.union(y.adoptadas), reveladas });
        }
        // La marca de capital: solo la del Rey electo pasa a C, tal cual (5A-5).
        for x in self.estado.asentamientos.values_mut() {
            let marca = politica::traslado::marca_tras_fusion(x.capital_de, a, b, de_rey_electo, c);
            if marca != x.capital_de {
                x.capital_de = marca;
                self.tr.tocar(Clase::Asentamiento, x.id);
            }
        }
        let r = self.estado.reglas.economia.politica.clone();
        let planes = politica::traslado::fusionar(&mut self.estado.politica, &r, c, union).expect("la respuesta ya revalidó la fusión");
        self.tr.tocar(Clase::Faccion, u64::from(c));
        for plan in &planes {
            self.trasladar(t, plan, false);
        }
        c
    }

    // ───────────────────────────── El traslado ─────────────────────────────

    /// Lo que no es político del traslado `plan.origen → plan.destino` (§6.3), con lo político ya aplicado por `politica::traslado`. `anexion` dice si las plazas del origen pasan a ser del
    /// destino desde ahora (la anexión) o conservan su antigüedad (la fusión: la Facción nueva no tiene plazas más antiguas que las suyas).
    fn trasladar(&mut self, t: Instante, plan: &Plan, anexion: bool) {
        let (origen, destino) = (plan.origen, plan.destino);
        let sin_plazas_antes = self.plazas_de(origen).next().is_none() || self.plazas_de(destino).next().is_none();
        self.tr.tocar_todos(Clase::Faccion, [u64::from(origen), u64::from(destino)]);

        // 1. Las relaciones heredadas o rotas: su clase en el registro y el ritmo de las alianzas de sus dos Facciones.
        for c in &plan.cambios {
            if let Cambio::Heredado { relacion, .. } | Cambio::Rota { relacion } = c {
                self.tocar_relacion(t, *relacion);
            }
        }
        // Las Facciones que derrotó el origen recuerdan ahora al destino.
        for c in &plan.cambios {
            if let Cambio::Derrota { faccion } = c {
                self.tr.tocar(Clase::Faccion, u64::from(*faccion));
            }
        }

        // 2. Las plazas: cambian de Facción con todo lo que cuelga de ellas, sus cargos locales incluidos. Van antes que la ciudadanía (5E): un residente que cambia de
        //    Facción y ya no es de la dueña de su plaza deja sus cargos en ella, y aquí la plaza se va con él.
        // DUDA: si las dos tenían Gran Fundición, el destino se queda con dos (la regla «una por Facción» es de construir, Doc 4.2.1 y 5C-5; ninguna dice que se pierda una).
        let plazas: Vec<u64> = self.plazas_de(origen).map(|a| a.id).collect();
        for &id in &plazas {
            let a = self.estado.asentamientos.get_mut(&id).expect("existe");
            a.faccion = destino;
            a.resto_tributo = 0;
            if anexion {
                // En la anexión la plaza es del destino desde ahora (5C-1: la capital del destino no cambia, como en una conquista); en la fusión conserva su antigüedad y la capital
                // efectiva de C sin designar es la plaza más antigua de las dos (Doc 2.6).
                a.de_la_faccion_desde = t;
            }
            self.tr.tocar(Clase::Asentamiento, id);
            self.cambiar_plaza_de_faccion(id, origen, Some(destino));
        }
        // La marca de capital del origen se borra de las plazas que la llevan (estén donde estén: sobrevive a la conquista).
        for x in self.estado.asentamientos.values_mut().filter(|x| x.capital_de == Some(origen)) {
            x.capital_de = None;
            self.tr.tocar(Clase::Asentamiento, x.id);
        }
        // Sus Aedas residentes pasan con ellas, con su épica, y sus épicas cumplidas se suman (6E, decisión técnica 13 del plan del sistema 6).
        self.trasladar_residentes(t, origen, destino, &plazas);
        // Una Facción derrotada que recibe plazas deja de estarlo (§2.1: `derrotadaPor` solo mientras no tiene ningún asentamiento).
        if !plazas.is_empty() && self.estado.politica.facciones.get(destino).is_some_and(|f| f.derrota.is_some()) {
            self.estado.politica.facciones.anular_derrota(destino);
        }

        // 3. La ciudadanía: la copia de cada ciudadano del origen, la columna de quien va solo y la demanda de bandidos del campamento donde reside.
        let nuevos: Vec<u64> = plan.cambios.iter().find_map(|c| if let Cambio::Ciudadanos { nuevos } = c { Some(nuevos.clone()) } else { None }).unwrap_or_default();
        for &h in &nuevos {
            self.tras_cambiar_de_faccion(t, h, Some(origen));
        }

        // 4. Las columnas, las convocatorias, las Caravanas de Fundación y las rutas de la red.
        let columnas: Vec<u64> = self.estado.ejercitos.values().filter(|e| e.columna.faccion == Some(origen)).map(|e| e.id).collect();
        for id in columnas {
            self.estado.ejercitos.get_mut(&id).expect("existe").columna.faccion = Some(destino);
            self.tr.tocar(Clase::Ejercito, id);
        }
        let convocatorias: Vec<u64> = self.estado.militar.convocatorias.values().filter(|c| c.faccion == origen).map(|c| c.id).collect();
        for id in convocatorias {
            self.estado.militar.convocatorias.get_mut(&id).expect("existe").faccion = destino;
            self.tr.tocar(Clase::Convocatoria, id);
        }
        let fundaciones: Vec<u64> = self.estado.fundacion.caravanas.values().filter(|c| c.faccion == Some(origen)).map(|c| c.id).collect();
        for id in fundaciones {
            self.estado.fundacion.caravanas.get_mut(&id).expect("existe").faccion = Some(destino);
            self.tr.tocar(Clase::Caravana, id);
        }
        self.estado.comercio.red.trasladar_faccion(origen, destino);

        // 5. La reputación y las tecnologías del andamiaje del origen se pierden (§6.1; en la fusión, C ya tiene las suyas). Su cita de reputación muere con su generación.
        if self.estado.comercio.reputacion.remove(&origen).is_some() {
            self.estado.generaciones.subir(crate::estado_comercio::entidad_reputacion(origen));
        }
        // Las tecnologías del origen se pierden (en la fusión, C ya tiene la unión); las que descubrió pasan a la que lo recoge (decisión técnica 13 del plan del sistema 6).
        self.estado.tecnologia.por_faccion.remove(&origen);
        for p in self.estado.tecnologia.primeros.iter_mut().flatten().filter(|p| p.faccion == origen) {
            p.faccion = destino;
        }

        // 6. La memoria de la niebla se funde al cerrar la versión, cuando ya se ha grabado lo que vieron los ojos del origen.
        self.uniones_de_memoria.push((origen, destino));

        // 7. Los bandidos de los anillos: si una de las dos no tenía plazas, cambian los residentes «sin plaza» de los campamentos.
        if sin_plazas_antes {
            self.reprogramar_anillos(t);
        }

        // 8. La puerta: quien está dentro de una plaza del origen, o es ciudadano del origen y está dentro de un lugar ajeno, sale al mapa si el lugar ya no le admite, con el motivo de la
        //    conquista (`cambia_de_dueno`; 5F-9, decisión del autor del 2026-10-25).
        let movidas: BTreeSet<u64> = plazas.iter().copied().collect();
        let ciudadanos: BTreeSet<u64> = nuevos.iter().copied().collect();
        let dentro: Vec<u64> = self
            .heroes()
            .filter(|h| {
                let plaza = match h.heroe.presencia.ubicacion {
                    Ubicacion::Asentamiento(a) => Some(u64::from(a)),
                    Ubicacion::Pueblo(x) => self.pueblo(x).map(|(m, _)| m),
                    Ubicacion::Avanzada(x) => self.avanzada(x).and_then(|(m, _)| m),
                    Ubicacion::Columna(_) | Ubicacion::Mercenarios(_) | Ubicacion::Desconectado(_) => None,
                };
                plaza.is_some_and(|p| movidas.contains(&p) || ciudadanos.contains(&h.heroe.id))
            })
            .map(|h| h.heroe.id)
            .collect();
        for h in dentro {
            self.echar_si_ya_no_entra(t, h, super::dentro::MOTIVO_CAMBIA_DE_DUENO);
        }
    }

    /// Las memorias de la niebla de los traslados de esta transición (§6.3), después de `explorar`: lo explorado por el origen se suma al destino; sus fichas y sus Informes, a los del destino
    /// (gana el del destino si los dos recuerdan el mismo sitio) sin los de sitios que ya son propios; sus Miradas pasan al destino. La memoria del origen desaparece.
    pub(super) fn fundir_memorias(&mut self) {
        for (origen, destino) in std::mem::take(&mut self.uniones_de_memoria) {
            let propio = |n: &Nucleo, s: Sitio| match s {
                Sitio::Plaza(id) => n.estado.asentamientos.get(&id).is_some_and(|a| a.faccion == destino),
                Sitio::Pueblo(id) => n.pueblo(id).is_some_and(|(m, _)| n.estado.asentamientos.get(&m).is_some_and(|a| a.faccion == destino)),
                Sitio::Avanzada(id) => n.avanzada(id).and_then(|(m, _)| m).is_some_and(|m| n.estado.asentamientos.get(&m).is_some_and(|a| a.faccion == destino)),
            };
            let mut mem = std::mem::take(&mut self.estado.memoria);
            if let Some(m) = mem.exploracion.remove(&origen) {
                match mem.exploracion.get_mut(&destino) {
                    Some(d) => d.fundir(&m),
                    None => {
                        mem.exploracion.insert(destino, m);
                    }
                }
            }
            if let Some(fichas) = mem.fichas.remove(&origen) {
                let de = mem.fichas.entry(destino).or_default();
                for (s, f) in fichas.into_iter().filter(|&(s, _)| !propio(self, s)) {
                    de.entry(s).or_insert(f);
                }
                if de.is_empty() {
                    mem.fichas.remove(&destino);
                }
            }
            if let Some(informes) = mem.informes.remove(&origen) {
                let de = mem.informes.entry(destino).or_default();
                for (plaza, i) in informes.into_iter().filter(|&(p, _)| !propio(self, Sitio::Plaza(p))) {
                    de.entry(plaza).or_insert(i);
                }
                if de.is_empty() {
                    mem.informes.remove(&destino);
                }
            }
            mem.miradas.iter_mut().filter(|m| m.faccion == origen).for_each(|m| m.faccion = destino);
            self.estado.memoria = mem;
        }
    }
}
