//! El fin del contrato: despido, plazo y lo que lo acaba desde el mundo (Mercenarios_Por_Contrato §7). Tarea T6 del plan.
//!
//! # Cuándo acaba
//! Por despido ([`Nucleo::despedir_mercenario`]), por plazo (`SucesoContrato::Vence`, [`Nucleo::vence_contrato`]) o por lo que le pasa al mundo mientras dura, que se mira al cerrar cada
//! versión ([`Nucleo::vigilar_fines`]): la plaza que firmó se pierde, no queda en la Facción ningún ciudadano sin marca, o el mercenario ya no es ciudadano de ella (la administración, una
//! anexión o una fusión). Todos pasan por [`Nucleo::terminar_contrato`], que o lo aplaza hasta que acabe la batalla del mercenario o lo cumple.
//!
//! # La salida de la Facción
//! Se separa de su ejército o de su convocatoria donde esté, sale de la Facción con la sucesión de siempre (`politica::quitar`: nunca fue Rey, así que no hay sucesión que hacer) y deja lo que
//! la ciudadanía le daba ([`Nucleo::tras_cambiar_de_faccion`]: su guarnición en pueblos). Lo que no es de la ciudadanía sino del contrato lo hace este fichero: el permanente vuelve a residir en
//! su campamento base sin cooldown y quien queda dentro de un lugar que ya no le admite sale al mapa. Si la Facción se queda sin nadie, es el caso de siempre («Facción sin nadie»): el
//! primero que reciba la ciudadanía será su Rey. Nunca se devuelve oro.

use super::*;
use crate::codec_contratos::trabajo_a_u8;
use crate::estado::Dato;
use crate::nucleo::dentro::MOTIVO_PUERTA_CERRADA;
use ::mercenarios::contrato::puesto_para_firmar;
use ::mercenarios::{Ambito, Contrato, Fin, Terminado};
use militar::columna::TipoColumna;
use militar::heroe::Residencia;

/// El motivo que lleva el hecho `mercenarios.terminado` (§7).
fn motivo(fin: Fin) -> &'static str {
    match fin {
        Fin::Plazo => "plazo",
        Fin::Despido => "despido",
        Fin::PlazaPerdida => "plaza_perdida",
        Fin::SinCiudadanos => "sin_ciudadanos",
        Fin::FaccionPerdida => "faccion_perdida",
    }
}

impl Nucleo {
    /// `despedir_mercenario` (§7): quien podría firmar hoy ese contrato lo da por acabado, a distancia y sin devolver nada. Despedir a quien combate se acepta y el fin espera a que acabe su
    /// batalla; despedir a quien ya tiene un fin aplazado no cambia nada.
    pub(super) fn despedir_mercenario(&mut self, t: Instante, p: &Peticion, heroe: u64, mercenario: u64) -> Result<Vec<u64>, Respuesta> {
        use rechazos::*;
        self.autorizar_heroe(p, heroe, false)?;
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: &str| Respuesta::rechazo(c, m, v);
        let faccion = self.heroe(heroe).and_then(|h| h.faccion);
        let c = self.estado.contratos.contratos.get(&mercenario).filter(|c| Some(c.faccion) == faccion);
        let c = c.ok_or_else(|| rechazo(MERCENARIO_SIN_CONTRATO, "ese mercenario no tiene un contrato vigente con tu Facción"))?;
        // Un objetivo que ya no existe no tiene nada que averiguar: entonces solo el Rey y el Embajador alcanzan el contrato.
        let s = self.situacion(c.faccion, &c.objetivo).unwrap_or_default();
        puesto_para_firmar(c.trabajo, &c.objetivo, &s, &self.puestos_de(heroe), heroe).map_err(|_| rechazo(MERCENARIO_SIN_ALCANCE, "ninguno de tus cargos podría firmar hoy ese contrato"))?;
        if c.fin_aplazado.is_some() {
            return Err(Respuesta::sin_cambios(v));
        }
        self.terminar_contrato(t, mercenario, Fin::Despido);
        Ok(vec![])
    }

    /// Vence el plazo de `mercenario` (`SucesoContrato::Vence`): si su contrato sigue venciendo en `t`, acaba por plazo; si se prorrogó o ya acabó, esta cita quedó vieja.
    pub(super) fn vence_contrato(&mut self, mercenario: u64, t: Instante) {
        if self.estado.contratos.contratos.get(&mercenario).is_some_and(|c| c.vence == Some(t)) {
            self.terminar_contrato(t, mercenario, Fin::Plazo);
        }
    }

    /// Los contratos que el mundo acaba (§7, decisión 18 del plan), al cerrar cada versión. Un contrato solo cambia de fin por lo que toca a una Facción (su ciudadanía, la residencia de un
    /// permanente, que viaja con ella), a una plaza (cambia de dueño o cae) o a una batalla (se cierra o alguien sale de ella): sin nada de eso tocado, no se mira ninguno. Cada uno se mira
    /// con consultas de coste logarítmico, sin recorrer héroes ni plazas.
    pub(super) fn vigilar_fines(&mut self, t: Instante) {
        let tocados = &self.tr.tocados;
        if [Clase::Faccion, Clase::Asentamiento, Clase::Batalla].into_iter().all(|c| tocados.de(c).is_empty()) {
            return;
        }
        let (mut acaban, mut a_casa) = (Vec::new(), Vec::new());
        for c in self.estado.contratos.contratos.values() {
            match self.fin_del_mundo(c) {
                Some(fin) => acaban.push((c.mercenario, fin)),
                None if c.trabajo == Trabajo::Permanente && !self.reside_en_plaza_de(c.mercenario, c.faccion) => a_casa.push((c.mercenario, c.campamento_base)),
                None => {}
            }
        }
        for (mercenario, fin) in acaban {
            self.terminar_contrato(t, mercenario, fin);
        }
        // Sin plaza de su Facción donde residir (la perdió, la Facción se quedó sin plazas, la administración lo mudó) vuelve a su campamento base sin perder el contrato (§7). Si la Facción tiene
        // otras plazas, la reubicación de siempre (`reubicar_residentes`) ya lo dejó en una y no llega aquí.
        for (mercenario, base) in a_casa {
            self.mudar_de_residencia(mercenario, Some(Residencia::Campamento(base)));
        }
    }

    /// Acaba el contrato de `mercenario` por `fin`, o lo aplaza si combate (§7, decisión C del diseño): el primer fin que llega es el que se cumple al acabar su batalla, y de eso se ocupa
    /// [`Nucleo::vigilar_fines`]. El contrato ha de existir.
    pub(super) fn terminar_contrato(&mut self, t: Instante, mercenario: u64, fin: Fin) {
        if self.heroe_en_batalla(mercenario) {
            let c = self.estado.contratos.contratos.get_mut(&mercenario).expect("quien acaba tiene contrato");
            c.fin_aplazado.get_or_insert(fin);
            let faccion = c.faccion;
            self.tr.tocar(Clase::Heroe, mercenario);
            self.tr.tocar(Clase::Faccion, u64::from(faccion));
            return;
        }
        let c = self.estado.contratos.contratos.remove(&mercenario).expect("quien acaba tiene contrato");
        self.separar_de_su_columna(t, mercenario);
        // Sale de la Facción en el mismo instante. Si ya no es ciudadano marcado de ella (la administración, una anexión o una fusión) no hay de qué sacarlo, pero su copia en el héroe sí
        // se pone al día: el traslado de una Facción no la toca en los marcados.
        if self.estado.politica.facciones.marca_de(mercenario).is_some() {
            self.estado.politica.facciones.quitar(mercenario);
        }
        self.tras_cambiar_de_faccion(t, mercenario, Some(c.faccion));
        // Sin ciudadanía no se reside en una plaza: el permanente vuelve a su campamento base, sin cooldown; el temporal sigue donde reside, que es un campamento.
        if self.heroe(mercenario).is_some_and(|h| matches!(h.heroe.residencia, Some(Residencia::Asentamiento(_)))) {
            self.mudar_de_residencia(mercenario, Some(Residencia::Campamento(c.campamento_base)));
        }
        // Y quien está dentro de una plaza, un pueblo o una avanzada que ya no le admite sale al mapa solo, con su carro (5B-1).
        self.echar_si_ya_no_entra(t, mercenario, MOTIVO_PUERTA_CERRADA);
        self.apuntar_fin(t, &c, fin);
    }

    /// El fin que el mundo le ha puesto ya a `c`, si alguno: el aplazado cuya batalla acabó o, si no tiene, el primero de los que se dan ahora. De más grave a menos: el mercenario ya no es
    /// ciudadano marcado de la Facción del contrato (o ella ya no existe), la plaza que lo firmó se perdió y no queda en la Facción ningún ciudadano sin marca.
    fn fin_del_mundo(&self, c: &Contrato) -> Option<Fin> {
        let mercenario = c.mercenario;
        if let Some(fin) = c.fin_aplazado {
            return (!self.heroe_en_batalla(mercenario)).then_some(fin);
        }
        let fs = &self.estado.politica.facciones;
        if fs.faccion_de(mercenario) != Some(c.faccion) || fs.marca_de(mercenario).is_none() {
            return Some(Fin::FaccionPerdida);
        }
        if let Ambito::Plaza(plaza) = c.ambito {
            if self.estado.asentamientos.get(&plaza).is_none_or(|a| a.faccion != c.faccion) {
                return Some(Fin::PlazaPerdida);
            }
        }
        fs.get(c.faccion).is_some_and(|f| f.sin_marca().next().is_none()).then_some(Fin::SinCiudadanos)
    }

    /// El campamento base de `heroe` si tiene un contrato permanente vigente. Lo pregunta el veto de una plaza (Mercenarios_Por_Contrato §11): vetar a un permanente que reside en ella le
    /// quita esa residencia y lo manda allí, sin perder el contrato.
    pub(in crate::nucleo) fn base_del_permanente(&self, heroe: u64) -> Option<u32> {
        self.estado.contratos.contratos.get(&heroe).filter(|c| c.trabajo == Trabajo::Permanente).map(|c| c.campamento_base)
    }

    /// ¿Reside `heroe` en una plaza de la Facción `faccion`?
    fn reside_en_plaza_de(&self, heroe: u64, faccion: u32) -> bool {
        match self.heroe(heroe).and_then(|h| h.heroe.residencia) {
            Some(Residencia::Asentamiento(a)) => self.estado.asentamientos.get(&u64::from(a)).is_some_and(|p| p.faccion == faccion),
            Some(Residencia::Campamento(_)) | None => false,
        }
    }

    /// Se separa de su convocatoria y de su ejército o formación, con lo suyo y donde esté (§7). Una columna personal o un ejército de uno solo no tienen de qué separarse.
    fn separar_de_su_columna(&mut self, t: Instante, mercenario: u64) {
        if self.estado.militar.convocatoria_de(mercenario).is_some() {
            self.separar_de_convocatoria(t, mercenario).expect("un marcado no convoca: no es el Líder de ninguna convocatoria y puede separarse");
        }
        let Ok((id, e)) = self.columna_a_bordo(mercenario) else { return };
        let c = &e.columna;
        if c.participantes.len() < 2 || (c.tipo != TipoColumna::Ejercito && !c.es_formacion()) {
            return;
        }
        // El Líder no se separa: cede antes el mando al más antiguo sin marca; si solo quedan marcados, al más antiguo (decisión 14 del plan).
        if c.lider == mercenario {
            let sin_marca = |h: u64| self.estado.politica.facciones.marca_de(h).is_none();
            let sucesor = c.sucesor(mercenario, &sin_marca).or_else(|| c.sucesor(mercenario, &|_| true)).expect("queda otro participante");
            self.ceder_el_mando(t, id, mercenario, sucesor).expect("el sucesor va en la columna y no es el Líder");
        }
        self.separar_del_ejercito(t, mercenario).expect("va en un ejército con más gente y ya no es su Líder");
    }

    /// Lo que deja el contrato acabado: la cuenta de la ficha, la espera de recontrato si fue un despido, lo que queda por valorar y el hecho `mercenarios.terminado`, que oyen la Facción y
    /// el mercenario. `esperas` y `terminados` se podan de vencidos al insertar, no al leer.
    fn apuntar_fin(&mut self, t: Instante, c: &Contrato, fin: Fin) {
        let d = &self.estado.reglas.economia.contratos;
        let (espera, valoracion) = (d.espera_recontrato, d.valoracion);
        let estado = &mut self.estado.contratos;
        let ficha = estado.fichas.get_mut(&c.mercenario).expect("la ficha nace con el primer ofrecimiento");
        if fin == Fin::Despido {
            ficha.historial.despidos += 1;
            estado.esperas.retain(|_, hasta| *hasta > t);
            estado.esperas.insert((c.mercenario, c.faccion), t + espera);
        } else {
            ficha.historial.terminados += 1;
        }
        estado.terminados.retain(|_, x| x.valorable_hasta > t);
        estado.terminados.insert((c.mercenario, c.faccion), Terminado { firmado_por: c.firmado_por, renombre: c.renombre, valorable_hasta: t + valoracion, valorado: false });
        self.tr.tocar(Clase::Heroe, c.mercenario);
        let datos = vec![("faccion", i64::from(c.faccion)), ("mercenario", c.mercenario as i64), ("trabajo", i64::from(trabajo_a_u8(c.trabajo)))];
        let valores = vec![("motivo", Dato::Texto(motivo(fin).into()))];
        self.tr.emitir(&self.estado, EventoNucleo { heroes: vec![c.mercenario], facciones: vec![c.faccion], valores, ..EventoNucleo::nuevo("mercenarios.terminado", t, datos) });
    }
}
