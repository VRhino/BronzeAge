//! Las órdenes de mando y mudar a un permanente (Mercenarios_Por_Contrato §6). Tarea T5 del plan.
//!
//! # El ciclo de una orden (§6.3)
//! Cada contrato tiene como mucho una orden **vigente**: la inicial de su trabajo al firmar y, después, la que le dé quien tenga alcance y rango ([`Nucleo::ordenar_mercenario`]).
//! Una orden acaba de tres modos y en los tres vuelve la inicial del contrato si aún vale (`inicial_de`):
//! - **cumplida**: un `atacar` es **un solo ataque** (decisión 6 del plan); se cumple en la primera batalla que resuelve su ataque, gane o pierda ([`Nucleo::tras_batalla`], llamada desde
//!   cada camino de batalla). Si la inicial es esa misma orden (el trabajo «Atacar un objetivo») no vuelve y el mercenario espera donde está hasta que le den otra;
//! - **caducada** por su plazo (`ir_a` y `atacar`, una cita `VenceOrden` en la cola), porque su objetivo desapareció o dejó de ser de la Facción, porque la guerra de un `atacar` acabó o
//!   porque el mercenario ya no va dentro de lo que acompañaba ([`Nucleo::vigilar_ordenes`], al cerrar cada versión);
//! - **sustituida** por otra de igual o más rango.
//!
//! La orden de quien deja el cargo sigue vigente hasta que acaba por uno de esos caminos (§15): lleva el puesto con el que se dio, no a quien lo ocupa hoy. La inicial del contrato no
//! lleva puesto y no tiene rango: la sustituye cualquier cargo que alcance al mercenario.
//!
//! # Quién lo oye
//! Los hechos de una orden (`ordenado`, `orden_cumplida`, `orden_caducada`) y `mudado` los oyen el mercenario y quien la dio, y nadie más (decisión 20 del plan): por eso no llevan claves
//! que nombren Facción, columna o `heroe` (esas las completa [`EventoNucleo`] por su cuenta).

use super::*;
use crate::codec_contratos::tipo_orden_a_u8;
use crate::estado::Dato;
use crate::estado_militar::Relacion;
use ::mercenarios::contrato::orden_inicial;
use ::mercenarios::orden::{admite, puesto_para_ordenar, sustituye, ErrorOrden};
use ::mercenarios::{Contrato, Orden, Puesto};
use militar::heroe::Residencia;
use std::collections::BTreeSet;

/// Cuántos mercenarios lleva como mucho una misma orden (decisión 23 del plan).
const MAXIMO_POR_ORDEN: usize = 16;

/// Lo que una batalla resuelta dice a las órdenes de mando (decisión 19 del plan): todo lo que se atacó en ella (en campo abierto, cada columna defensora) y quiénes lo atacaron.
pub(in crate::nucleo) struct Desenlace<'a> {
    pub atacados: &'a [Objetivo],
    pub atacantes: &'a [u64],
}

/// Por qué caduca una orden (hecho `mercenarios.orden_caducada`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Caducidad {
    /// Venció su plazo (`ir_a` y `atacar`).
    Plazo,
    /// El objetivo desapareció o ya no es de la Facción.
    ObjetivoPerdido,
    /// La Facción ya no está en guerra con la dueña del objetivo de un `atacar`.
    Paz,
    /// Estuvo dentro de lo que acompañaba (`unirse`) y ya no lo está.
    Salida,
}

impl Caducidad {
    /// El motivo tal como lo dice el hecho.
    fn id(self) -> &'static str {
        match self {
            Caducidad::Plazo => "plazo",
            Caducidad::ObjetivoPerdido => "objetivo",
            Caducidad::Paz => "paz",
            Caducidad::Salida => "salida",
        }
    }
}

/// Cómo acaba una orden vigente.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Cierre {
    /// Se hizo lo que pedía: el ataque se libró.
    Cumplida,
    Caducada(Caducidad),
}

/// El rechazo que corresponde a un fallo de las reglas puras de las órdenes: su código y su mensaje.
fn rechazo_de_orden(e: ErrorOrden) -> (&'static str, &'static str) {
    use rechazos::*;
    match e {
        ErrorOrden::ObjetivoInvalido => (MERCENARIO_OBJETIVO_INVALIDO, "el objetivo no es de la clase que pide esa orden o no es de quien debe ser"),
        ErrorOrden::FueraDeContrato => (MERCENARIO_ORDEN_FUERA_DE_CONTRATO, "su trabajo no admite esa orden"),
        ErrorOrden::SinAlcance => (MERCENARIO_SIN_ALCANCE, "ninguno de tus cargos alcanza a ese mercenario con esa orden"),
    }
}

/// La entidad que cambia cuando cambia lo que nombra `o`, para saber si la transición que se cierra lo tocó. Un punto no es una entidad.
fn entidad_de(o: &Objetivo) -> Option<(Clase, u64)> {
    Some(match *o {
        Objetivo::Plaza(id) => (Clase::Asentamiento, id),
        Objetivo::Pueblo(id) => (Clase::Pueblo, id),
        Objetivo::Avanzada(id) => (Clase::Avanzada, id),
        Objetivo::Caravana(id) => (Clase::Caravana, id),
        Objetivo::Bandidos(id) => (Clase::Bandidos, id),
        Objetivo::Ejercito(id) => (Clase::Ejercito, id),
        Objetivo::Convocatoria(id) => (Clase::Convocatoria, id),
        Objetivo::Faccion(id) => (Clase::Faccion, u64::from(id)),
        Objetivo::Punto(_) => return None,
    })
}

impl Nucleo {
    // ───────────────────────────── Consultas ─────────────────────────────

    /// Dónde está lo que nombra `o` en `t`, si es algo con sitio: para medir la distancia de una orden a su contrato (`atacar_en_radio`, §6.4).
    fn posicion_de(&self, o: &Objetivo, t: Instante) -> Option<Punto> {
        match *o {
            Objetivo::Plaza(id) => self.estado.asentamientos.get(&id).map(|a| a.posicion),
            Objetivo::Pueblo(id) => self.posicion_de_pueblo(id),
            Objetivo::Avanzada(id) => self.posicion_de_avanzada(id),
            Objetivo::Caravana(id) => self.posicion_de_caravana(id, t),
            Objetivo::Bandidos(id) => self.campamento_de_bandidos(id).map(|b| b.posicion),
            Objetivo::Ejercito(id) => self.ejercito(id).map(|e| e.posicion_en(t)),
            Objetivo::Punto(p) => Some(p),
            Objetivo::Convocatoria(_) | Objetivo::Faccion(_) => None,
        }
    }

    /// La plaza donde reside `heroe`, si reside en una.
    fn plaza_de_residencia(&self, heroe: u64) -> Option<u64> {
        match self.heroe(heroe)?.heroe.residencia {
            Some(Residencia::Asentamiento(a)) => Some(u64::from(a)),
            _ => None,
        }
    }

    /// ¿Va `heroe` dentro de ese ejército o de esa convocatoria? (`unirse`, §6.1)
    fn va_dentro_de(&self, heroe: u64, objetivo: &Objetivo) -> bool {
        match *objetivo {
            Objetivo::Ejercito(id) => self.ejercito(id).is_some_and(|e| e.columna.participa(heroe)),
            Objetivo::Convocatoria(id) => self.convocatoria(id).is_some_and(|c| c.integrantes.iter().any(|i| i.heroe == heroe)),
            _ => false,
        }
    }

    /// Por qué ya no vale la orden `o` del contrato `c`, si ya no vale (§6.3). El plazo no está aquí: lo cumple su cita.
    fn caducidad_de(&self, c: &Contrato, o: &Orden) -> Option<Caducidad> {
        let Some(s) = self.situacion(c.faccion, &o.objetivo) else { return Some(Caducidad::ObjetivoPerdido) };
        match o.tipo {
            TipoOrden::IrA => None,
            TipoOrden::Guardar => (s.duena != Some(c.faccion)).then_some(Caducidad::ObjetivoPerdido),
            TipoOrden::Atacar => o.contra.filter(|&contra| self.estado.relacion(c.faccion, contra) != Some(Relacion::Guerra)).map(|_| Caducidad::Paz),
            TipoOrden::Unirse => (o.unido && !self.va_dentro_de(c.mercenario, &o.objetivo)).then_some(Caducidad::Salida),
        }
    }

    /// La orden inicial del contrato `c` tal como se daría en `t` (§6.3), si todavía vale: su objetivo existe y no habría caducado nada más darla.
    fn inicial_de(&self, c: &Contrato, t: Instante) -> Option<Orden> {
        let contra = self.situacion(c.faccion, &c.objetivo)?.duena;
        let orden = orden_inicial(&self.estado.reglas.economia.contratos, c, self.plaza_de_residencia(c.mercenario), contra, t)?;
        self.caducidad_de(c, &orden).is_none().then_some(orden)
    }

    /// ¿Tocó la transición que se cierra la Facción del contrato o lo que nombra la orden? Solo entonces puede haber cambiado algo para ella.
    fn tocada(&self, faccion: u32, o: &Orden) -> bool {
        let tocados = &self.tr.tocados;
        tocados.contiene(Clase::Faccion, u64::from(faccion)) || entidad_de(&o.objetivo).is_some_and(|(clase, id)| tocados.contiene(clase, id))
    }

    // ───────────────────────────── Ayudas ─────────────────────────────

    /// Deja `orden` como la vigente de `mercenario` (o ninguna) con su plazo en la cola, y toca lo que lee la proyección: su héroe y su Facción. Una `unirse` nace con `unido` si el
    /// mercenario ya va dentro de lo que acompaña. La cita que quedó vieja se ignora al vencer porque su instante ya no es el guardado.
    fn fijar_orden(&mut self, mercenario: u64, orden: Option<Orden>) {
        let orden = orden.map(|o| Orden { unido: o.unido || (o.tipo == TipoOrden::Unirse && self.va_dentro_de(mercenario, &o.objetivo)), ..o });
        let c = self.estado.contratos.contratos.get_mut(&mercenario).expect("quien recibe o pierde una orden tiene contrato");
        c.orden = orden;
        let faccion = c.faccion;
        if let Some(vence) = orden.and_then(|o| o.vence) {
            self.citar_contrato(mercenario, SucesoContrato::VenceOrden, vence);
        }
        self.tr.tocar(Clase::Heroe, mercenario);
        self.tr.tocar(Clase::Faccion, u64::from(faccion));
    }

    /// Un hecho sobre la orden `o` de `mercenario`: lo oyen él y quien la dio.
    fn hecho_de_orden(&mut self, t: Instante, codigo: &'static str, mercenario: u64, o: &Orden, valores: Vec<(&'static str, Dato)>) {
        let datos = vec![("mercenario", mercenario as i64), ("ordenante", o.dada_por as i64), ("tipo", i64::from(tipo_orden_a_u8(o.tipo)))];
        self.tr.emitir(&self.estado, EventoNucleo { heroes: vec![mercenario, o.dada_por], valores, ..EventoNucleo::nuevo(codigo, t, datos) });
    }

    /// La orden vigente de `mercenario` acaba (§6.3): se lo cuenta a él y a quien la dio, y vuelve la inicial del contrato si aún vale. La inicial no vuelve si es la misma orden que
    /// acaba de cumplirse: un ataque es una sola vez.
    fn cerrar_orden(&mut self, t: Instante, mercenario: u64, cierre: Cierre) {
        let Some((c, cerrada)) = self.estado.contratos.contratos.get(&mercenario).and_then(|c| Some((c, c.orden?))) else { return };
        let misma = |i: &Orden| (i.tipo, i.objetivo) == (cerrada.tipo, cerrada.objetivo);
        let inicial = self.inicial_de(c, t).filter(|i| cierre != Cierre::Cumplida || !misma(i));
        match cierre {
            Cierre::Cumplida => self.hecho_de_orden(t, "mercenarios.orden_cumplida", mercenario, &cerrada, vec![]),
            Cierre::Caducada(motivo) => self.hecho_de_orden(t, "mercenarios.orden_caducada", mercenario, &cerrada, vec![("motivo", Dato::Texto(motivo.id().into()))]),
        }
        self.fijar_orden(mercenario, inicial);
    }

    // ───────────────────────────── Órdenes ─────────────────────────────

    /// `ordenar_mercenario` (§6.1, §6.5; decisión 23 del plan): un cargo da la misma orden a entre 1 y 16 mercenarios de su Facción, a distancia. Se comprueba cada uno con las reglas de
    /// siempre antes de tocar nada: si uno falla no entra ninguna y el rechazo lo nombra. Quien ya tiene esa misma orden (tipo y objetivo) se queda como está; si la tienen todos, no cambia nada.
    pub(super) fn ordenar_mercenario(&mut self, t: Instante, p: &Peticion, heroe: u64, mercenarios: &[u64], tipo: TipoOrden, objetivo: Objetivo) -> Result<Vec<u64>, Respuesta> {
        use rechazos::*;
        self.autorizar_heroe(p, heroe, false)?;
        let v = self.estado.version;
        if !(1..=MAXIMO_POR_ORDEN).contains(&mercenarios.len()) || mercenarios.iter().collect::<BTreeSet<_>>().len() != mercenarios.len() {
            return Err(Respuesta::rechazo(MERCENARIO_LISTA_INVALIDA, format!("una orden lleva de 1 a {MAXIMO_POR_ORDEN} mercenarios, sin repetir"), v));
        }
        let d = self.estado.reglas.economia.contratos.clone();
        let faccion = self.heroe(heroe).and_then(|h| h.faccion);
        let puestos = self.puestos_de(heroe);
        let situacion = faccion.and_then(|f| self.situacion(f, &objetivo));
        let donde = self.posicion_de(&objetivo, t);
        let mut nuevos = Vec::new();
        for &m in mercenarios {
            let nombra = |codigo: &'static str, mensaje: &str| Respuesta::rechazo(codigo, format!("{}: {mensaje}", ids::heroe(m)), v);
            let de_orden = |e: ErrorOrden| {
                let (codigo, mensaje) = rechazo_de_orden(e);
                nombra(codigo, mensaje)
            };
            let c = self.estado.contratos.contratos.get(&m).filter(|c| Some(c.faccion) == faccion).ok_or_else(|| nombra(MERCENARIO_SIN_CONTRATO, "no tiene un contrato vigente con tu Facción"))?;
            let s = situacion.as_ref().ok_or_else(|| nombra(MERCENARIO_OBJETIVO_INVALIDO, "el objetivo no existe"))?;
            let puesto = puesto_para_ordenar(c, self.plaza_de_residencia(m), &puestos, tipo, &objetivo, s, heroe).map_err(de_orden)?;
            let distancia = donde.zip(self.posicion_de(&c.objetivo, t)).map(|(a, b)| a.dist(b));
            admite(&d, c, tipo, &objetivo, distancia).map_err(de_orden)?;
            if c.orden.is_some_and(|o| o.tipo == tipo && o.objetivo == objetivo) {
                continue;
            }
            if !sustituye(c.orden.as_ref(), puesto) {
                return Err(nombra(MERCENARIO_ORDEN_DE_MAS_RANGO, "su orden vigente la dio un cargo de más rango que el tuyo"));
            }
            nuevos.push((m, puesto));
        }
        if nuevos.is_empty() {
            return Err(Respuesta::sin_cambios(v));
        }

        let contra = situacion.and_then(|s| s.duena).filter(|_| tipo == TipoOrden::Atacar);
        let vence = matches!(tipo, TipoOrden::IrA | TipoOrden::Atacar).then_some(t + d.plazo_orden);
        for (m, puesto) in nuevos {
            let orden = Orden { tipo, objetivo, dada_por: heroe, puesto: Some(puesto), dada_en: t, vence, contra, unido: false };
            self.fijar_orden(m, Some(orden));
            self.hecho_de_orden(t, "mercenarios.ordenado", m, &orden, vec![]);
        }
        Ok(vec![])
    }

    /// `mudar_mercenario` (§5, §6.2): el Rey o el Embajador cambian a un permanente de plaza, a distancia y sin cooldown. Su `guardar` de la plaza vieja pasa a ser de la nueva.
    pub(super) fn mudar_mercenario(&mut self, t: Instante, p: &Peticion, heroe: u64, mercenario: u64, plaza: u64) -> Result<Vec<u64>, Respuesta> {
        use rechazos::*;
        self.autorizar_heroe(p, heroe, false)?;
        let v = self.estado.version;
        let rechazo = |codigo: &'static str, mensaje: &str| Respuesta::rechazo(codigo, mensaje, v);
        let faccion = self.heroe(heroe).and_then(|h| h.faccion);
        let c = self.estado.contratos.contratos.get(&mercenario).filter(|c| Some(c.faccion) == faccion).ok_or_else(|| rechazo(MERCENARIO_SIN_CONTRATO, "ese mercenario no tiene un contrato vigente con tu Facción"))?;
        if !self.puestos_de(heroe).iter().any(|x| matches!(x, Puesto::Rey | Puesto::Embajador)) {
            return Err(rechazo(MERCENARIO_SIN_ALCANCE, "solo el Rey o el Embajador mudan a un permanente"));
        }
        if c.trabajo != Trabajo::Permanente {
            return Err(rechazo(MERCENARIO_TIPO_DE_CONTRATO, "solo se muda a un permanente: el temporal sigue en su campamento"));
        }
        if self.estado.asentamientos.get(&plaza).is_none_or(|a| a.faccion != c.faccion) {
            return Err(rechazo(MERCENARIO_OBJETIVO_INVALIDO, "el permanente reside en una plaza de la Facción"));
        }
        let vieja = self.plaza_de_residencia(mercenario);
        if vieja == Some(plaza) {
            return Err(Respuesta::sin_cambios(v));
        }
        let guardaba_la_vieja = c.orden.filter(|o| o.tipo == TipoOrden::Guardar && vieja.is_some_and(|a| o.objetivo == Objetivo::Plaza(a)));

        self.mudar_de_residencia(mercenario, Some(Residencia::Asentamiento(plaza as u32)));
        if let Some(o) = guardaba_la_vieja {
            self.fijar_orden(mercenario, Some(Orden { objetivo: Objetivo::Plaza(plaza), ..o }));
        }
        let datos = vec![("mercenario", mercenario as i64), ("ordenante", heroe as i64), ("plaza", plaza as i64)];
        self.tr.emitir(&self.estado, EventoNucleo { heroes: vec![mercenario, heroe], ..EventoNucleo::nuevo("mercenarios.mudado", t, datos) });
        Ok(vec![])
    }

    // ───────────────────────────── El ciclo de la orden ─────────────────────────────

    /// Tras resolverse una batalla (decisión 19 del plan): cada mercenario que atacó alguno de los objetivos de su orden `atacar` la cumple, gane o pierda. Un ataque es una sola vez (decisión 6).
    pub(in crate::nucleo) fn tras_batalla(&mut self, t: Instante, d: &Desenlace) {
        for &h in d.atacantes {
            let cumple = self.estado.contratos.contratos.get(&h).and_then(|c| c.orden).is_some_and(|o| o.tipo == TipoOrden::Atacar && d.atacados.contains(&o.objetivo));
            if cumple {
                self.cerrar_orden(t, h, Cierre::Cumplida);
            }
        }
    }

    /// Al cerrar cada versión (decisión 18 del plan): las órdenes de las que la transición tocó su objetivo o su Facción pueden haber caducado (§6.3). Una `unirse` cuyo mercenario ya va
    /// dentro lo apunta (`unido`), porque desde entonces salir la hace caducar.
    pub(super) fn vigilar_ordenes(&mut self, t: Instante) {
        let tocadas: Vec<u64> = self.estado.contratos.contratos.values().filter(|c| c.orden.is_some_and(|o| self.tocada(c.faccion, &o))).map(|c| c.mercenario).collect();
        for m in tocadas {
            let c = &self.estado.contratos.contratos[&m];
            let o = c.orden.expect("se recogió con orden y solo esta vuelta la cambia");
            match self.caducidad_de(c, &o) {
                Some(motivo) => self.cerrar_orden(t, m, Cierre::Caducada(motivo)),
                None if o.tipo == TipoOrden::Unirse && !o.unido && self.va_dentro_de(m, &o.objetivo) => self.fijar_orden(m, Some(Orden { unido: true, ..o })),
                None => {}
            }
        }
    }

    /// Una convocatoria parte y el ejército que nace (`ejercito`) es aquello que acompañaban (§6.3): los contratos de acompañarla y las órdenes `unirse` a ella pasan a ser del ejército,
    /// y las órdenes conservan si el mercenario ya estaba dentro. Solo caducan si la convocatoria se cancela.
    pub(in crate::nucleo) fn convocatoria_parte(&mut self, convocatoria: u64, ejercito: u64) {
        let (antes, ahora) = (Objetivo::Convocatoria(convocatoria), Objetivo::Ejercito(ejercito));
        let mut cambiados = Vec::new();
        for c in self.estado.contratos.contratos.values_mut() {
            let mut cambia = false;
            if c.objetivo == antes {
                c.objetivo = ahora;
                cambia = true;
            }
            if let Some(o) = c.orden.as_mut().filter(|o| o.objetivo == antes) {
                o.objetivo = ahora;
                cambia = true;
            }
            if cambia {
                cambiados.push((c.mercenario, c.faccion));
            }
        }
        for (mercenario, faccion) in cambiados {
            self.tr.tocar(Clase::Heroe, mercenario);
            self.tr.tocar(Clase::Faccion, u64::from(faccion));
        }
    }

    /// Vence el plazo de la orden de `mercenario` (`ir_a` y `atacar`): caduca si el instante sigue siendo el guardado; si la sustituyeron, esta cita es la vieja.
    pub(super) fn vence_orden(&mut self, mercenario: u64, t: Instante) {
        let vence = self.estado.contratos.contratos.get(&mercenario).and_then(|c| c.orden).is_some_and(|o| o.vence == Some(t));
        if vence {
            self.cerrar_orden(t, mercenario, Cierre::Caducada(Caducidad::Plazo));
        }
    }
}
