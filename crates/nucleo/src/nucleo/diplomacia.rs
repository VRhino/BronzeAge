//! Las relaciones entre Facciones en el núcleo (sistema 5, tanda 5D; Diplomacia_Facciones §5, Doc 2.3-2.4.1). Las reglas puras son de `politica` (`relaciones`, `propuestas`,
//! `tributo`); aquí, las órdenes, lo que arrastran y la reputación que mueven (los `Suceso`s que devuelve `politica`, aplicados en `comercio::reputacion`).
//!
//! # Las relaciones viven en `Estado::politica`
//! Con dirección (en un vasallaje `a` es la señora; en una guerra, quien declaró), estado (`activa`/`rota`, las rotas se conservan), fechas, tributo y quién ofreció la paz. Los
//! consumidores de siempre (encuentros, defensa, puerta, unirse a batallas, visión compartida, tipo A/B de caravana) miran la clase de la **activa** de cada par con
//! `Estado::relacion`, que va por el índice del almacén (O(log n)). Los ids de relaciones y propuestas salen del contador común de la partida.
//!
//! # Toda agresión exige guerra o la declara (§5.4)
//! `Atacar` con `declara_guerra` usa la misma declaración que la orden `DeclararGuerra`, con el **arrastre por vasallaje**; contra una aliada es la **traición** de Q2 (rompe la
//! alianza, −25, y declara la guerra); contra la señora o la vasalla propias se rechaza (`combate.vasallaje`). `DeclararGuerra` contra una aliada exige romper antes.
//!
//! # Lo que pasa solo, sin eventos en la cola
//! - El **tributo** se paga en el paso de minuto de cada plaza vasalla (Q3), en unidades enteras con su resto oculto (`Asentamiento::resto_tributo`); lo que no cabe en la capital de la
//!   señora se paga igual y se pierde (5A-2).
//! - Al caer la última plaza de una Facción: el **desarme del señor** (sus vasallajes se rompen) y la **vasalla conquistada** (pasa al conquistador con el mismo tributo), en ese orden.
//! - La **reputación**: romper una alianza, liberar a la vasalla, la rebelión (a la señora), la traición, la guerra tras romper y la defensa del vasallo, en el instante del hecho; las
//!   alianzas activas de cada Facción entran en el ritmo de su reputación al cambiar.

use super::*;
use crate::estado::Dato;
use crate::estado_militar::Relacion;
use comercio::reputacion::Suceso;
use politica::autorizacion::Actor;
use politica::propuestas::{Clase as ClaseDePropuesta, ErrorPropuesta, Pacto};
use politica::relaciones::{Declaracion, ErrorDiplomacia, Guerra, RelacionPolitica, TipoRelacion, Tributo};
use politica::Politica;
use std::collections::BTreeSet;

fn de_diplomacia(e: ErrorDiplomacia) -> (&'static str, &'static str) {
    use rechazos::*;
    use ErrorDiplomacia::*;
    match e {
        MismaFaccion => (RELACION_CONSIGO_MISMA, "una Facción no tiene relación consigo misma"),
        FaccionNoExiste => (FACCION_NO_EXISTE, "alguna de las dos Facciones no existe"),
        YaHayRelacion => (DIPLOMACIA_YA_HAY_RELACION, "ya hay una relación activa entre las dos: hay que romperla (o firmar la paz) antes"),
        RelacionNoExiste => (DIPLOMACIA_RELACION_DESCONOCIDA, "esa relación no existe"),
        NoActiva => (DIPLOMACIA_RELACION_ROTA, "esa relación ya está rota"),
        NoEsGuerra => (DIPLOMACIA_NO_ES_GUERRA, "la paz se ofrece en una guerra"),
        NoEsVasallaje => (DIPLOMACIA_NO_ES_VASALLAJE, "solo una vasalla se rebela"),
        NoEsParte => (DIPLOMACIA_NO_ES_PARTE, "esa Facción no es parte de esa relación"),
        YaOfrecioPaz => (DIPLOMACIA_YA_OFRECIO_PAZ, "esa Facción ya ofreció la paz: falta la otra"),
        GuerraNoSeRompe => (DIPLOMACIA_GUERRA_NO_SE_ROMPE, "una guerra no se rompe: se acaba con la paz"),
        SoloLaSenora => (DIPLOMACIA_SOLO_LA_SENORA, "un vasallaje solo lo libera la señora; la vasalla se rebela"),
        YaEsVasalla => (DIPLOMACIA_VASALLA_DE_TERCERA, "la vasalla ya lo es de otra señora"),
    }
}

fn de_propuesta(e: ErrorPropuesta) -> (&'static str, &'static str) {
    use rechazos::*;
    match e {
        ErrorPropuesta::Diplomacia(d) => de_diplomacia(d),
        ErrorPropuesta::SinRey(_) => (DIPLOMACIA_SIN_REY, "una de las dos Facciones no tiene Rey"),
        ErrorPropuesta::VasallaDeTercera(_) => (DIPLOMACIA_VASALLA_DE_TERCERA, "una de las dos ya es vasalla de otra: su señora tiene que liberarla antes"),
        ErrorPropuesta::ReputacionBaja => (DIPLOMACIA_REPUTACION_BAJA, "con esa reputación nadie se alía: está por debajo del umbral del Embajador"),
        ErrorPropuesta::YaPendiente => (DIPLOMACIA_YA_PENDIENTE, "ya hay una propuesta de relación pendiente entre las dos"),
        ErrorPropuesta::Caducada => (DIPLOMACIA_PROPUESTA_CADUCADA, "la propuesta caducó"),
        ErrorPropuesta::BatallaAbierta => (DIPLOMACIA_BATALLA_ABIERTA, "alguna de las dos combate en una batalla abierta: se contesta cuando termine"),
        // Las de la anexión y la fusión (5F) no salen de una propuesta de relación.
        ErrorPropuesta::NoExiste | ErrorPropuesta::ReyNoValido | ErrorPropuesta::NombreVacio | ErrorPropuesta::NombreDuplicado | ErrorPropuesta::ProponenteYaNoEsRey => {
            (DIPLOMACIA_PROPUESTA_DESCONOCIDA, "esa propuesta de relación no existe")
        }
    }
}

fn tipo_codigo(t: TipoRelacion) -> i64 {
    match t {
        TipoRelacion::Alianza => 0,
        TipoRelacion::Vasallaje => 1,
        TipoRelacion::Guerra => 2,
    }
}

impl Nucleo {
    // ───────────────────────────── Consultas ─────────────────────────────

    /// La clase de la relación activa entre dos Facciones, si la hay.
    pub fn relacion(&self, a: u32, b: u32) -> Option<Relacion> {
        self.estado.relacion(a, b)
    }

    /// La relación activa entre dos Facciones, entera (con su dirección).
    pub fn relacion_activa(&self, a: u32, b: u32) -> Option<&RelacionPolitica> {
        self.estado.politica.relaciones.activa_entre(a, b)
    }

    /// Una relación por su id, activa o rota.
    pub fn relacion_politica(&self, id: u64) -> Option<&RelacionPolitica> {
        self.estado.politica.relaciones.get(id)
    }

    /// Una propuesta por su id (las caducadas siguen hasta que otra de su clase las poda).
    pub fn propuesta(&self, id: u64) -> Option<&politica::Propuesta> {
        self.estado.politica.propuestas.get(id)
    }

    /// ¿Combate la Facción en alguna batalla abierta (la ataca o la defiende)? Una relación no se acepta mientras tanto (§5.3).
    fn faccion_en_batalla(&self, f: u32) -> bool {
        let t = self.estado.ahora;
        self.batallas().any(|b| b.activa(t) && (b.ticket.atacante.faccion == Some(f) || b.ticket.defensor.faccion == Some(f)))
    }

    /// ¿Abre la plaza su almacén a una columna de la Facción `faccion` (`puedeRepostarEn`, Movimiento_Ejercitos §8)? La propia, siempre; una aliada o vasalla (o la señora), si la plaza
    /// lo permite (`permiteReabastecerAliados`); nadie más.
    pub fn abre_su_almacen_a(&self, plaza: u64, faccion: Option<u32>) -> bool {
        let Some(a) = self.estado.asentamientos.get(&plaza) else { return false };
        faccion == Some(a.faccion) || (a.permite_reabastecer_aliados && faccion.is_some_and(|f| matches!(self.estado.relacion(f, a.faccion), Some(Relacion::Alianza | Relacion::Vasallaje))))
    }

    // ───────────────────────────── Ayudas ─────────────────────────────

    /// Las Facciones que una relación nombra existen como entidad: la administración y la guerra al atacar las crean vacías si hace falta (como `FijarNivelFaccion`, decisión 9).
    fn asegurar_facciones(&mut self, fs: &[u32]) {
        let r = self.estado.reglas.economia.politica.clone();
        for &f in fs {
            if !self.estado.politica.facciones.existe(f) {
                self.estado.politica.facciones.crear_vacia(&r, f);
                self.tr.tocar(Clase::Faccion, u64::from(f));
            }
        }
    }

    /// `autoridadDiplomatica(f)`: el héroe es de la cuenta (y en el mundo) y es Rey o Embajador de `f`. Una Facción que no existe pasa (la orden la rechaza con su código).
    fn exigir_autoridad(&self, p: &Peticion, heroe: u64, f: u32) -> Result<(), Respuesta> {
        self.autorizar_heroe(p, heroe, false)?;
        if p.admin || (Actor { yo: heroe, politica: &self.estado.politica, vista: self }).autoridad_diplomatica(f) {
            return Ok(());
        }
        Err(Respuesta::rechazo(rechazos::NO_AUTORIZADO, format!("solo el Rey o el Embajador de f-{f}"), self.estado.version))
    }

    fn exigir_rey(&self, p: &Peticion, heroe: u64, f: u32) -> Result<(), Respuesta> {
        self.autorizar_heroe(p, heroe, false)?;
        if p.admin || (Actor { yo: heroe, politica: &self.estado.politica, vista: self }).rey_de_faccion(f) {
            return Ok(());
        }
        Err(Respuesta::rechazo(rechazos::NO_AUTORIZADO, format!("solo el Rey de f-{f}"), self.estado.version))
    }

    /// Aplica los sucesos de reputación que devuelve `politica`, en su orden.
    pub(super) fn aplicar_sucesos(&mut self, t: Instante, sucesos: &[(u32, Suceso)]) {
        for &(f, s) in sucesos {
            self.aplicar_reputacion(t, f, s);
        }
    }

    /// Las alianzas activas de cada Facción entran en el ritmo de su reputación (`bonusPorMinutoAlianzaActiva`): se recuentan cuando cambian. Una Facción sin reputación todavía y sin
    /// alianzas no la necesita.
    fn recontar_alianzas(&mut self, t: Instante, fs: &[u32]) {
        let rr = self.estado.reglas.economia.comercio.reputacion.clone();
        for &f in fs {
            let n = self.estado.politica.relaciones.alianzas_de(f);
            let rep = &mut self.estado.comercio.reputacion;
            if n == 0 && !rep.contains_key(&f) || rep.get(&f).is_some_and(|r| r.alianzas() == n) {
                continue;
            }
            rep.entry(f).or_insert_with(|| comercio::reputacion::Reputacion::nueva(t)).con_alianzas(&rr, t, n);
            self.tr.tocar(Clase::Faccion, u64::from(f));
            self.programar_reputacion(f, t);
        }
    }

    /// Lo que cambia con una relación: ella, sus dos Facciones y, si es una alianza, el ritmo de su reputación.
    fn tocar_relacion(&mut self, t: Instante, id: u64) {
        let Some(r) = self.estado.politica.relaciones.get(id) else { return };
        let (a, b, alianza) = (r.a, r.b, r.tipo == TipoRelacion::Alianza);
        self.tr.tocar(Clase::Relacion, id);
        self.tr.tocar_todos(Clase::Faccion, [u64::from(a), u64::from(b)]);
        if alianza {
            self.recontar_alianzas(t, &[a, b]);
        }
    }

    fn hecho_de_relacion(&mut self, t: Instante, codigo: &'static str, facciones: Vec<u32>, datos: Vec<(&'static str, i64)>, valores: Vec<(&'static str, Dato)>) {
        let mut facciones = facciones;
        facciones.sort_unstable();
        facciones.dedup();
        self.tr.emitir(&self.estado, EventoNucleo { facciones, valores, ..EventoNucleo::nuevo(codigo, t, datos) });
    }

    /// Una guerra recién declarada: sus relaciones y Facciones, y el hecho `diplomacia.guerra_declarada` con las arrastradas (`GuerraDeclarada.arrastradas_ids`).
    fn anotar_guerra(&mut self, t: Instante, g: &Guerra, traicion: bool) {
        for &id in &g.relaciones {
            self.tocar_relacion(t, id);
        }
        let arrastradas = Dato::Lista(g.arrastradas.iter().map(|&f| Dato::Texto(ids::faccion(f))).collect());
        let todas: Vec<u32> = [g.atacante, g.objetivo].into_iter().chain(g.arrastradas.iter().copied()).collect();
        let datos = vec![("atacante", i64::from(g.atacante)), ("contra", i64::from(g.objetivo)), ("traicion", traicion as i64)];
        self.hecho_de_relacion(t, "diplomacia.guerra_declarada", todas, datos, vec![("arrastradas", arrastradas)]);
    }

    /// Una relación rota (alianza o vasallaje): el hecho `diplomacia.relacion_rota` con quién la rompió.
    fn anotar_ruptura(&mut self, t: Instante, id: u64, por: Option<u32>) {
        self.tocar_relacion(t, id);
        let r = self.estado.politica.relaciones.get(id).expect("existe").clone();
        let mut datos = vec![("relacion", id as i64), ("faccionA", i64::from(r.a)), ("faccionB", i64::from(r.b)), ("tipo", tipo_codigo(r.tipo))];
        datos.extend(por.map(|f| ("faccion", i64::from(f))));
        self.hecho_de_relacion(t, "diplomacia.relacion_rota", vec![r.a, r.b], datos, vec![]);
    }

    /// Ids del contador común para lo que la regla vaya a crear: se piden mirando y solo se gastan si la regla lo acepta (una orden rechazada no deja rastro).
    fn ids_desde(&self) -> impl FnMut() -> u64 {
        let mut siguiente = self.estado.siguiente_id;
        move || {
            siguiente += 1;
            siguiente - 1
        }
    }

    /// Gasta los ids que usaron las relaciones nuevas (las de `ids_desde`).
    fn gastar_ids_hasta(&mut self, mayor: Option<u64>) {
        if let Some(m) = mayor {
            self.estado.siguiente_id = self.estado.siguiente_id.max(m + 1);
        }
    }

    // ───────────────────────────── La guerra al atacar ─────────────────────────────

    /// El arrastre que vería quien ataca sin estar en guerra (`guerra.confirmar`): las Facciones del bando del objetivo que entrarían por vasallaje.
    pub(super) fn arrastre_de(&self, atacante: u32, objetivo: u32) -> Vec<u32> {
        self.estado.politica.relaciones.arrastre(atacante, objetivo)
    }

    /// Declara la guerra de toda agresión (`atacar` con `declara_guerra`, §5.4): con el arrastre; contra una aliada, la traición de Q2. `comprobar_agresion` ya miró que no haya un
    /// vasallaje entre las dos ni una guerra; las Facciones que no existan como entidad nacen vacías (las del andamiaje).
    pub(super) fn declarar_guerra(&mut self, t: Instante, atacante: u32, contra: u32) {
        self.asegurar_facciones(&[atacante, contra]);
        let rr = self.estado.reglas.economia.comercio.reputacion.clone();
        let mut ids = self.ids_desde();
        let Politica { facciones, relaciones, .. } = &mut self.estado.politica;
        let Declaracion { guerra, alianza_rota, sucesos } = relaciones.declarar_al_atacar(&rr, facciones, atacante, contra, t, &mut ids).expect("comprobar_agresion ya miró el par");
        self.gastar_ids_hasta(guerra.relaciones.iter().max().copied());
        if let Some(id) = alianza_rota {
            self.anotar_ruptura(t, id, Some(atacante));
        }
        self.aplicar_sucesos(t, &sucesos);
        self.anotar_guerra(t, &guerra, alianza_rota.is_some());
    }

    // ───────────────────────────── Lo que pasa al caer una Facción ─────────────────────────────

    /// En el instante en que `f` pierde su última plaza (§5.5, C7), en este orden: el **desarme del señor** (sus vasallajes se rompen, sin reputación ni guerra) y la **vasalla
    /// conquistada** (si era vasalla y la derrotó otra Facción, pasa a ella con el mismo tributo).
    pub(super) fn diplomacia_tras_derrota(&mut self, t: Instante, f: u32) {
        for (id, vasalla) in self.estado.politica.relaciones.liberar_vasallas_de(f) {
            self.tocar_relacion(t, id);
            self.hecho_de_relacion(t, "diplomacia.vasallo_liberado", vec![f, vasalla], vec![("relacion", id as i64), ("faccionA", i64::from(f)), ("faccionB", i64::from(vasalla))], vec![]);
        }
        self.pasar_vasalla_conquistada(t, f);
    }

    /// `pasarVasallasConquistadas` para `vasalla` (al caer su última plaza, o al aceptar una señora nueva sin tener plazas: Diplomacia §12).
    fn pasar_vasalla_conquistada(&mut self, t: Instante, vasalla: u32) {
        if self.plazas_de(vasalla).next().is_some() {
            return;
        }
        let nueva = self.estado.siguiente_id;
        let Politica { facciones, relaciones, .. } = &mut self.estado.politica;
        let Some(c) = relaciones.pasar_vasalla_conquistada(facciones, vasalla, t, nueva) else { return };
        self.gastar_ids_hasta(Some(nueva));
        for id in [c.rota, c.nueva].into_iter().chain(c.acabadas.iter().copied()) {
            self.tocar_relacion(t, id);
        }
        let datos = vec![("relacion", c.nueva as i64), ("faccionA", i64::from(c.conquistadora)), ("faccionB", i64::from(c.vasalla)), ("faccionAnterior", i64::from(c.senora_anterior))];
        self.hecho_de_relacion(t, "diplomacia.vasallo_conquistado", vec![c.conquistadora, c.vasalla, c.senora_anterior], datos, vec![]);
    }

    /// Defender al vasallo (§5.9) tras un combate cuyo defensor es de la Facción `vasalla`: `defensores` son los héroes que defendieron (con Unity, los que participaron).
    pub(super) fn defensa_de_vasallo(&mut self, t: Instante, vasalla: Option<u32>, defensores: &[u64], gano_el_defensor: bool) {
        let Some(v) = vasalla else { return };
        if self.estado.politica.relaciones.senora_de(v).is_none() {
            return;
        }
        let presentes: BTreeSet<u32> = defensores.iter().filter_map(|&h| self.heroe(h).and_then(|x| x.faccion)).collect();
        let sucesos = self.estado.politica.relaciones.defensa_de_vasallo(v, &presentes, gano_el_defensor);
        self.aplicar_sucesos(t, &sucesos);
    }

    // ───────────────────────────── El tributo ─────────────────────────────

    /// El tributo de la plaza `id` en su paso de minuto (Q3): si su Facción es vasalla con un tributo pactado, paga su parte (lo pactado entre sus plazas, en unidades enteras con resto
    /// oculto, sin pasar de su stock) a la capital de la señora; lo que no cabe allí se pierde (5A-2). Sin capital, no paga. Un hecho por plaza que paga algo. No marca nada en el registro:
    /// los almacenes son del interior de cada plaza, que va por su propia secuencia de pasos de minuto.
    pub(super) fn pagar_tributo(&mut self, id: u64, t: Instante) {
        let Some(a) = self.estado.asentamientos.get(&id) else { return };
        let Some(rel) = self.estado.politica.relaciones.vasallaje_de(a.faccion) else { return };
        let (Some(Tributo { recurso, cantidad_por_minuto }), senora, relacion) = (rel.tributo, rel.a, rel.id) else { return };
        if cantidad_por_minuto <= 0 {
            return;
        }
        let Some(capital) = self.capital_de(senora) else { return };
        let plazas = self.espacio.plazas_por_faccion.get(&a.faccion).map_or(1, |s| s.len()) as u32;
        let (vasalla, stock, resto) = (a.faccion, a.economia.almacen.cantidad(recurso) / base::UNIDAD, a.resto_tributo);
        let hueco = self.estado.asentamientos[&capital].economia.almacen.espacio(recurso) / base::UNIDAD;
        let pago = politica::tributo::pago(cantidad_por_minuto, plazas, resto, stock, hueco);
        let a = self.estado.asentamientos.get_mut(&id).expect("existe");
        a.resto_tributo = pago.resto;
        if pago.pagadas == 0 {
            return;
        }
        a.economia.almacen.quitar(recurso, pago.pagadas * base::UNIDAD);
        self.estado.asentamientos.get_mut(&capital).expect("existe").economia.almacen.agregar(recurso, pago.recibidas * base::UNIDAD);
        let datos = vec![
            ("relacion", relacion as i64),
            ("faccionA", i64::from(senora)),
            ("faccionB", i64::from(vasalla)),
            ("plaza", id as i64),
            ("capital", capital as i64),
            ("recurso", i64::from(crate::codec_economia::recurso_a_u8(recurso))),
            ("pagadas", pago.pagadas),
            ("recibidas", pago.recibidas),
        ];
        self.tr.emitir(&self.estado, EventoNucleo { asentamientos: vec![id, capital], facciones: vec![senora.min(vasalla), senora.max(vasalla)], ..EventoNucleo::nuevo("diplomacia.tributo_pagado", t, datos) });
    }

    // ───────────────────────────── La administración ─────────────────────────────

    /// `fijarRelacion` (andamiaje de administración, decisión 9 del plan del sistema 5): rompe la relación activa del par (si la hay) y crea la pedida, con `a` como señora o declarante,
    /// sin propuesta, sin arrastre y sin reputación. Las Facciones que no existen nacen vacías. Sin relación (`None`), solo rompe.
    pub(super) fn fijar_relacion(&mut self, t: Instante, a: u32, b: u32, relacion: Option<Relacion>) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        if a == b {
            return Err(Respuesta::rechazo(rechazos::RELACION_CONSIGO_MISMA, "una Facción no tiene relación consigo misma", v));
        }
        if self.estado.relacion(a, b) == relacion {
            return Err(Respuesta::sin_cambios(v));
        }
        let rs = &self.estado.politica.relaciones;
        if relacion == Some(Relacion::Vasallaje) && rs.senora_de(b).is_some_and(|s| s != a) {
            return Err(Respuesta::rechazo(rechazos::DIPLOMACIA_VASALLA_DE_TERCERA, format!("f-{b} ya es vasalla de otra señora"), v));
        }
        if let Some(id) = rs.activa_entre(a, b).map(|r| r.id) {
            self.estado.politica.relaciones.terminar(id);
            self.anotar_ruptura(t, id, None);
        }
        let Some(r) = relacion else { return Ok(vec![]) };
        self.asegurar_facciones(&[a, b]);
        let tipo = match r {
            Relacion::Alianza => TipoRelacion::Alianza,
            Relacion::Vasallaje => TipoRelacion::Vasallaje,
            Relacion::Guerra => TipoRelacion::Guerra,
        };
        let id = self.estado.siguiente_id;
        let Politica { facciones, relaciones, .. } = &mut self.estado.politica;
        relaciones.crear(facciones, id, tipo, a, b, None, t).expect("par validado");
        self.gastar_ids_hasta(Some(id));
        if tipo == TipoRelacion::Guerra {
            self.anotar_guerra(t, &Guerra { atacante: a, objetivo: b, relaciones: vec![id], arrastradas: vec![] }, false);
        } else {
            self.tocar_relacion(t, id);
            self.hecho_de_relacion(t, "diplomacia.relacion_aceptada", vec![a, b], vec![("relacion", id as i64), ("faccionA", i64::from(a)), ("faccionB", i64::from(b)), ("tipo", tipo_codigo(tipo))], vec![]);
        }
        Ok(vec![])
    }

    // ───────────────────────────── Órdenes ─────────────────────────────

    /// Las órdenes de diplomacia (144-152). Los rechazos y «sin cambios» salen como `Err` sin haber tocado el estado.
    pub(super) fn ejecutar_diplomacia(&mut self, t: Instante, p: &Peticion) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |(c, m): (&'static str, &'static str)| Respuesta::rechazo(c, m, v);
        let rel = |n: &Nucleo, id: u64| n.estado.politica.relaciones.get(id).cloned().ok_or_else(|| rechazo(de_diplomacia(ErrorDiplomacia::RelacionNoExiste)));
        match &p.comando {
            Comando::ProponerRelacion { heroe, a, b, vasallaje, tributo } => {
                self.exigir_autoridad(p, *heroe, *a)?;
                let tributo = match (vasallaje, tributo) {
                    (_, Some((_, n))) if *n < 0 => return Err(rechazo((rechazos::DIPLOMACIA_TRIBUTO_INVALIDO, "el tributo no puede ser negativo"))),
                    (false, Some(_)) => return Err(rechazo((rechazos::DIPLOMACIA_TRIBUTO_INVALIDO, "una alianza no lleva tributo"))),
                    (_, t) => t.map(|(recurso, cantidad_por_minuto)| Tributo { recurso, cantidad_por_minuto }),
                };
                let pacto = if *vasallaje { Pacto::Vasallaje } else { Pacto::Alianza };
                let (r, rr, rep) = (self.estado.reglas.economia.politica.clone(), self.estado.reglas.economia.comercio.reputacion.clone(), self.reputacion(*a));
                let id = self.estado.siguiente_id;
                self.estado.politica.proponer_relacion(&r, &rr, id, pacto, *a, *b, tributo, *heroe, rep, t).map_err(|e| rechazo(de_propuesta(e)))?;
                self.gastar_ids_hasta(Some(id));
                self.tr.tocar_todos(Clase::Faccion, [u64::from(*a), u64::from(*b)]);
                let datos = vec![("propuesta", id as i64), ("faccionA", i64::from(*a)), ("faccionB", i64::from(*b)), ("tipo", *vasallaje as i64), ("heroe", *heroe as i64)];
                self.hecho_de_relacion(t, "diplomacia.relacion_propuesta", vec![*a, *b], datos, vec![]);
                Ok(vec![id])
            }
            Comando::ResponderRelacion { heroe, propuesta, aceptar } => {
                let (a, b, pacto) = self.propuesta_de_relacion(*propuesta, t)?;
                self.exigir_rey(p, *heroe, b)?;
                let rr = self.estado.reglas.economia.comercio.reputacion.clone();
                let (rep, batalla, id) = (self.reputacion(a), self.faccion_en_batalla(a) || self.faccion_en_batalla(b), self.estado.siguiente_id);
                let nueva = self.estado.politica.responder_relacion(&rr, *propuesta, *aceptar, rep, batalla, id, t).map_err(|e| rechazo(de_propuesta(e)))?;
                self.tr.tocar_todos(Clase::Faccion, [u64::from(a), u64::from(b)]);
                let mut datos = vec![("propuesta", *propuesta as i64), ("faccionA", i64::from(a)), ("faccionB", i64::from(b)), ("tipo", (pacto == Pacto::Vasallaje) as i64), ("heroe", *heroe as i64)];
                let Some(id) = nueva else {
                    self.hecho_de_relacion(t, "diplomacia.relacion_rechazada", vec![a, b], datos, vec![]);
                    return Ok(vec![]);
                };
                self.gastar_ids_hasta(Some(id));
                self.tocar_relacion(t, id);
                datos.push(("relacion", id as i64));
                self.hecho_de_relacion(t, "diplomacia.relacion_aceptada", vec![a, b], datos, vec![]);
                if pacto == Pacto::Vasallaje {
                    // Una vasalla sin plazas derrotada por otra pasa a quien la derrotó (Diplomacia §12: `derrotadaPor` no caduca mientras no tenga plaza).
                    self.pasar_vasalla_conquistada(t, b);
                }
                Ok(vec![])
            }
            Comando::RetirarRelacion { heroe, propuesta } => {
                let (a, b, _) = self.propuesta_de_relacion(*propuesta, t)?;
                self.exigir_autoridad(p, *heroe, a)?;
                self.estado.politica.retirar_propuesta(*propuesta, t).map_err(|e| rechazo(de_propuesta(e)))?;
                self.tr.tocar_todos(Clase::Faccion, [u64::from(a), u64::from(b)]);
                self.hecho_de_relacion(t, "diplomacia.relacion_retirada", vec![a, b], vec![("propuesta", *propuesta as i64), ("faccionA", i64::from(a)), ("faccionB", i64::from(b)), ("heroe", *heroe as i64)], vec![]);
                Ok(vec![])
            }
            Comando::DeclararGuerra { heroe, a, b } => {
                self.exigir_autoridad(p, *heroe, *a)?;
                let rr = self.estado.reglas.economia.comercio.reputacion.clone();
                let mut ids = self.ids_desde();
                let Politica { facciones, relaciones, .. } = &mut self.estado.politica;
                let tras_romper = relaciones.guerra_tras_romper(&rr, *a, *b, t);
                let guerra = relaciones.declarar_guerra(facciones, *a, *b, t, &mut ids).map_err(|e| rechazo(de_diplomacia(e)))?;
                self.gastar_ids_hasta(guerra.relaciones.iter().max().copied());
                self.aplicar_sucesos(t, &tras_romper.into_iter().collect::<Vec<_>>());
                self.anotar_guerra(t, &guerra, false);
                Ok(vec![])
            }
            Comando::ProponerPaz { heroe, relacion, faccion } => {
                rel(self, *relacion)?;
                self.exigir_autoridad(p, *heroe, *faccion)?;
                let firmada = self.estado.politica.relaciones.proponer_paz(*relacion, *faccion).map_err(|e| rechazo(de_diplomacia(e)))?;
                self.tocar_relacion(t, *relacion);
                let r = self.estado.politica.relaciones.get(*relacion).expect("existe").clone();
                let datos = vec![("relacion", *relacion as i64), ("faccionA", i64::from(r.a)), ("faccionB", i64::from(r.b)), ("faccion", i64::from(*faccion))];
                self.hecho_de_relacion(t, if firmada { "diplomacia.paz_firmada" } else { "diplomacia.paz_propuesta" }, vec![r.a, r.b], datos, vec![]);
                Ok(vec![])
            }
            Comando::RomperRelacion { heroe, relacion, iniciador } => {
                rel(self, *relacion)?;
                self.exigir_autoridad(p, *heroe, *iniciador)?;
                let sucesos = self.estado.politica.relaciones.romper(*relacion, *iniciador, t).map_err(|e| rechazo(de_diplomacia(e)))?;
                self.anotar_ruptura(t, *relacion, Some(*iniciador));
                self.aplicar_sucesos(t, &sucesos);
                Ok(vec![])
            }
            Comando::RebelionVasallo { heroe, relacion } => {
                let r = rel(self, *relacion)?;
                self.exigir_autoridad(p, *heroe, r.b)?;
                let mut ids = self.ids_desde();
                let Politica { facciones, relaciones, .. } = &mut self.estado.politica;
                let reb = relaciones.rebelion(facciones, *relacion, t, &mut ids).map_err(|e| rechazo(de_diplomacia(e)))?;
                self.gastar_ids_hasta(reb.guerra.relaciones.iter().max().copied());
                self.tocar_relacion(t, *relacion);
                let datos = vec![("relacion", *relacion as i64), ("faccionA", i64::from(reb.senora)), ("faccionB", i64::from(reb.vasalla))];
                self.hecho_de_relacion(t, "diplomacia.rebelion_vasallo", vec![reb.senora, reb.vasalla], datos, vec![]);
                self.expirar_trueques_entre(t, reb.senora, reb.vasalla);
                self.aplicar_sucesos(t, &reb.sucesos);
                self.anotar_guerra(t, &reb.guerra, false);
                Ok(vec![])
            }
            Comando::AdmitirOtrasFacciones { heroe, faccion, admitir } => {
                self.exigir_rey(p, *heroe, *faccion)?;
                let cambia = self.estado.politica.facciones.admitir_otras(*faccion, *admitir).map_err(|_| rechazo(de_diplomacia(ErrorDiplomacia::FaccionNoExiste)))?;
                if !cambia {
                    return Err(Respuesta::sin_cambios(v));
                }
                self.tr.tocar(Clase::Faccion, u64::from(*faccion));
                self.hecho_de_relacion(t, "faccion.admision_en_ataques", vec![*faccion], vec![("faccion", i64::from(*faccion)), ("admite", *admitir as i64), ("heroe", *heroe as i64)], vec![]);
                Ok(vec![])
            }
            Comando::AlternarReabastecerAliados { asentamiento, permitido } => {
                let faccion = self.plaza_o_rechazo(*asentamiento)?.faccion;
                if !self.manda_en(p, faccion) {
                    return Err(Respuesta::rechazo(rechazos::NO_AUTORIZADO, format!("no mandas en as-{asentamiento}"), v));
                }
                let a = self.estado.asentamientos.get_mut(asentamiento).expect("existe");
                if a.permite_reabastecer_aliados == *permitido {
                    return Err(Respuesta::sin_cambios(v));
                }
                a.permite_reabastecer_aliados = *permitido;
                self.tr.tocar(Clase::Asentamiento, *asentamiento);
                self.emitir_propio(t, "logistica.reabastecer_aliados", vec![*asentamiento], vec![("permitido", *permitido as i64)]);
                Ok(vec![])
            }
            _ => unreachable!("solo se llama con órdenes de diplomacia"),
        }
    }

    /// La propuesta de relación `id`, vigente: `(a, b, pacto)`.
    fn propuesta_de_relacion(&self, id: u64, t: Instante) -> Result<(u32, u32, Pacto), Respuesta> {
        let v = self.estado.version;
        let p = self.estado.politica.propuestas.vigente(id, t).map_err(|e| {
            let (c, m) = de_propuesta(e);
            Respuesta::rechazo(c, m, v)
        })?;
        match p.clase {
            ClaseDePropuesta::Relacion { pacto, a, b, .. } => Ok((a, b, pacto)),
            _ => Err(Respuesta::rechazo(rechazos::DIPLOMACIA_PROPUESTA_DESCONOCIDA, "esa propuesta no es de relación", v)),
        }
    }

    /// La rebelión expira, sin penalizar a nadie, los trueques **activos** entre plazas de la señora y de la vasalla (§5.5).
    fn expirar_trueques_entre(&mut self, t: Instante, f: u32, g: u32) {
        let faccion = |p: u32| self.estado.asentamientos.get(&u64::from(p)).map(|a| a.faccion);
        let entre: Vec<(u64, u32)> = self
            .estado
            .comercio
            .acuerdos
            .iter()
            .filter(|x| x.estado == comercio::trueque::EstadoTrueque::Activo)
            .filter_map(|x| {
                let (fa, fb) = (faccion(x.a)?, faccion(x.b)?);
                ((fa, fb) == (f, g) || (fa, fb) == (g, f)).then_some((x.id, x.b))
            })
            .collect();
        for (acuerdo, socia) in entre {
            let x = self.estado.comercio.acuerdos.iter_mut().find(|x| x.id == acuerdo).expect("existe");
            x.cancelar_por_plaza();
            self.tr.tocar(Clase::Acuerdo, acuerdo);
            self.invalidar_comercio(crate::estado_comercio::entidad_trueque(acuerdo));
            self.emitir_comercio(t, comercio::evento::Evento::TruequeCancelado { acuerdo, socia }, vec![u64::from(socia)], &[]);
        }
    }
}
