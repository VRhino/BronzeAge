//! Los Aedas residentes y sus épicas (sistema 6, tanda 6E; Tecnologia_Aedas §7). Las reglas puras son del crate `tecnologia` (`residentes`, `epicas`, 6A); aquí están el cupo de cada plaza
//! mirado cuando cambia lo que lo decide, la cita de llegada, las dos órdenes, los hechos de épica de la transición y lo que arrastran la conquista, la ruina y el traslado de una Facción.
//!
//! # El cupo, sin tick (decisión técnica 2 del plan)
//! `cupo = (Palacio activo ∧ nobleza > 0) ? cupoPorNivel[nivel actual] : 0`. Se revisa la plaza (`revisar_residentes`) solo cuando puede cambiar: el paso de minuto si la nobleza pasa de 0 a más
//! o al revés, o si cambia el nivel actual (comparado antes y después del paso, sin buscar el Palacio); al completarse un Palacio; al fijar el nivel por administración; en la conquista (que daña
//! edificios y cambia la dueña y su reputación); y al cambiar la reputación de la dueña con un suceso. Si sobran, se van los últimos en llegar (`aedas.residente_se_va`). Mientras queda cupo libre la
//! plaza **espera** y tiene **una** cita en la cola (`Carga::Residentes`, prioridad 10, entidad `entidad_residentes`): a `espera + llegada`; si entonces la reputación de la dueña está en el umbral
//! bajo o por debajo, en el primero de `espera + llegada × factor` y el cruce hacia arriba del umbral (decisión técnica 6, `tecnologia::residentes::llegada`). Al llegar uno
//! (`aedas.residente_llega`, `aeda-rN`) se olvida la espera y, si aún cabe otro, empieza otra. Sin cupo libre no hay cita: una plaza sin cupo no tiene nada en la cola.
//!
//! # Las épicas
//! `EmpezarEpica` (177) y `AbandonarEpica` (178), con `dirigeAedas` (el Rey de la dueña, el Gobernador o el Sacerdote de la plaza, dentro). Los hechos inspiradores se anotan **tipados en la
//! transición** donde ocurren (`Nucleo::hechos_epicos`: la defensa y la conquista del asedio con números y de Unity, el encuentro y el campo abierto de Unity, la obra y el ascenso de la plaza, la
//! adopción y la caravana comercial que llega con carga ≥ `cargaMinimaCaravana`) y se procesan al cerrar la versión (`avanzar_epicas`), antes de la aparición por hito: a lo sumo uno por Aeda y
//! versión, con el enfriamiento y las claves del crate. Al cerrar el último capítulo la tecnología aparece sin hito (con las citas de su descubrimiento si es la primera del mundo), la Facción suma
//! una épica cumplida y salen `aedas.epica_tecnologia` (privado) y `aedas.epica_cumplida` (público, a la crónica). La épica perdida avisa (`aedas.epica_abandonada { motivo }`, decisión técnica
//! 11): `conquista` (la plaza cambió de dueño), `aparecida` (la tecnología le apareció a la Facción por otra vía), `unificada` (tras una anexión o fusión otro Aeda de la Facción la cantaba
//! más avanzada) u `orden`.
//!
//! # La ruina y el traslado
//! La ruina se lleva a los residentes de la plaza con su espera y su cita, sin hecho (como el TypeScript). La anexión y la fusión (decisión técnica 13) pasan los residentes de la que desaparece a la
//! que recoge, con su épica, y le suman sus épicas cumplidas; la épica de una tecnología que la que recoge ya tiene se pierde (`aparecida`), y dos de la misma tecnología se unifican (sigue la más
//! avanzada; la otra, `unificada`).

use super::*;
use crate::estado::{entidad_residentes, Dato, PRIO_RESIDENTES};
use asentamiento::catalogo::Tipo;
use asentamiento::edificio::Edificios;
use tecnologia::epicas::{Avance, ErrorEpica, Hecho};
use tecnologia::residentes::{self as reglas_residentes, Llegada};
use tecnologia::{Residentes, TecId};

/// Cuántos residentes hay en cada plaza (el índice derivado de `Espacio`).
pub(super) fn por_plaza(r: &Residentes) -> BTreeMap<u64, u32> {
    let mut m = BTreeMap::new();
    for a in &r.aedas {
        *m.entry(u64::from(a.plaza)).or_insert(0) += 1;
    }
    m
}

fn rechazo_de_epica(e: ErrorEpica) -> (&'static str, &'static str) {
    use rechazos::*;
    match e {
        ErrorEpica::NoViveAqui => (EPICA_NO_VIVE_AQUI, "ese Aeda no vive en esta plaza"),
        ErrorEpica::SinEpica => (EPICA_SIN_EPICA, "esa tecnología no tiene épica"),
        ErrorEpica::AedaAjeno => (EPICA_AEDA_AJENO, "el Aeda aún no se ha asentado con el dueño actual de la plaza"),
        ErrorEpica::YaCanta => (EPICA_YA_CANTA, "el Aeda ya canta una épica: hay que abandonarla antes"),
        ErrorEpica::SinLogro => (EPICA_SIN_LOGRO, "el logro del servidor de esa tecnología aún no se ha cumplido"),
        ErrorEpica::EraCerrada => (EPICA_ERA_CERRADA, "la Era de esa tecnología aún no ha empezado"),
        ErrorEpica::YaAparecida => (EPICA_YA_APARECIDA, "esa tecnología ya le ha aparecido a la Facción"),
        ErrorEpica::OtroLaCanta => (EPICA_OTRO_LA_CANTA, "otro Aeda de la Facción ya canta la épica de esa tecnología"),
        ErrorEpica::NoCanta => (EPICA_NO_CANTA, "el Aeda no canta ninguna épica"),
    }
}

impl Nucleo {
    // ───────────────────────────── Consultas ─────────────────────────────

    /// Los Aedas residentes del mundo, sus esperas y las épicas cumplidas por Facción.
    pub fn residentes(&self) -> &Residentes {
        &self.estado.residentes
    }

    /// Cuántos Aedas residentes viven en la plaza (el índice derivado).
    pub fn residentes_en(&self, plaza: u64) -> u32 {
        self.espacio.residentes_por_plaza.get(&plaza).copied().unwrap_or(0)
    }

    /// Los residentes de las plazas de la Facción: con sus épicas cumplidas, el valor de «Mecenas de los Aedas» (`cumplidas × 10 + residentes`, Doc 2.9).
    pub(super) fn mecenazgo(&self, faccion: u32) -> i64 {
        let residentes: u32 = self.plazas_de(faccion).map(|a| self.residentes_en(a.id)).sum();
        i64::from(self.estado.residentes.cumplidas_de(faccion)) * 10 + i64::from(residentes)
    }

    /// El cupo de residentes de la plaza ahora (TA §7.1, nivel actual; decisión técnica 5).
    fn cupo_de_residentes(&self, plaza: u64) -> u32 {
        let Some(a) = self.estado.asentamientos.get(&plaza) else { return 0 };
        let eco = &a.economia;
        reglas_residentes::cupo(&self.estado.reglas.economia.tecnologia, eco.edificios.activos(Tipo::Palacio) > 0, eco.poblacion.nobleza > 0, a.nivel_actual().min(u8::MAX as u32) as u8)
    }

    // ───────────────────────────── El ciclo de vida ─────────────────────────────

    /// TA §7.1 para una plaza: se ajusta a su cupo (los que sobran se van), llega uno si ya le toca y se deja en la cola la cita que corresponda (o ninguna, sin cupo libre). Idempotente: si nada
    /// cambió, no toca nada.
    pub(super) fn revisar_residentes(&mut self, plaza: u64, t: Instante) {
        loop {
            let Some(a) = self.estado.asentamientos.get(&plaza) else { return };
            let faccion = a.faccion;
            let cupo = self.cupo_de_residentes(plaza);
            let ajuste = self.estado.residentes.ajustar(plaza as u32, cupo, t);
            for &aeda in &ajuste.salen {
                self.contar_residente(plaza, -1);
                self.tocar_residente(aeda);
                self.emitir_propio(t, "aedas.residente_se_va", vec![plaza], vec![("aedaResidente", i64::from(aeda)), ("faccion", i64::from(faccion)), ("cupo", i64::from(cupo))]);
            }
            if !ajuste.salen.is_empty() {
                self.tocar_residentes(faccion, plaza);
            }
            let Some(espera) = ajuste.espera else {
                return self.programar_llegada(plaza, None);
            };
            let r = self.estado.reglas.economia.tecnologia.clone();
            let umbral = r.aedas.umbral_reputacion_baja;
            // Una Facción sin reputación guardada la tiene en 0 (`Reputacion::nueva`).
            let rep = self.estado.comercio.reputacion.get(&faccion);
            let baja = rep.map_or(0, |x| x.valor_en(t)) <= umbral;
            let sube = rep.filter(|_| baja).and_then(|x| x.cruce_de(umbral + 1, t));
            match reglas_residentes::llegada(&r, espera, t, baja, sube) {
                // Defensa: nunca una cita en el mismo instante (no volvería a cambiar nada).
                Llegada::MirarEn(en) => return self.programar_llegada(plaza, Some(en.max(t + 1))),
                Llegada::Llega => {
                    let aeda = self.estado.residentes.llega(plaza as u32, faccion, t);
                    self.contar_residente(plaza, 1);
                    self.tocar_residente(aeda);
                    self.tocar_residentes(faccion, plaza);
                    self.emitir_propio(t, "aedas.residente_llega", vec![plaza], vec![("aedaResidente", i64::from(aeda)), ("faccion", i64::from(faccion))]);
                    // Si aún cabe otro, la espera vuelve a empezar ahora.
                }
            }
        }
    }

    /// Deja en la cola la cita de llegada de la plaza en `en` (o ninguna): solo si cambia, con una generación nueva que descarta la anterior.
    fn programar_llegada(&mut self, plaza: u64, en: Option<Instante>) {
        let actual = self.estado.residentes_citas.get(&plaza).copied();
        if actual == en {
            return;
        }
        let entidad = entidad_residentes(plaza);
        if actual.is_some() {
            self.estado.generaciones.subir(entidad);
        }
        match en {
            Some(en) => {
                self.estado.residentes_citas.insert(plaza, en);
                let clave = self.estado.generaciones.clave(entidad);
                self.estado.cola.programar(en, PRIO_RESIDENTES, clave, Carga::Residentes);
            }
            None => {
                self.estado.residentes_citas.remove(&plaza);
            }
        }
    }

    /// Vence la cita de llegada de una plaza (la entidad del evento).
    pub(super) fn cita_de_residentes(&mut self, entidad: u64, t: Instante) {
        let plaza = crate::estado_comercio::id_de_entidad(entidad);
        self.estado.residentes_citas.remove(&plaza);
        self.revisar_residentes(plaza, t);
    }

    /// Cambió la reputación de la Facción con un suceso: las plazas suyas que esperan vuelven a mirar su cita (la llegada lenta depende de ella).
    pub(super) fn reputacion_de_residentes(&mut self, faccion: u32, t: Instante) {
        if self.estado.residentes_citas.is_empty() {
            return;
        }
        let esperan: Vec<u64> = self.plazas_de(faccion).map(|a| a.id).filter(|p| self.estado.residentes_citas.contains_key(p)).collect();
        for p in esperan {
            self.revisar_residentes(p, t);
        }
    }

    fn contar_residente(&mut self, plaza: u64, d: i32) {
        let n = self.espacio.residentes_por_plaza.entry(plaza).or_insert(0);
        *n = n.saturating_add_signed(d);
        if *n == 0 {
            self.espacio.residentes_por_plaza.remove(&plaza);
        }
    }

    /// Un residente cambió (llegó, se fue, cambió de dueña o su épica avanzó): viaja como `AedaResidente` a `FACCION` de su dueña (6G).
    fn tocar_residente(&mut self, aeda: u32) {
        self.tr.tocar(Clase::Residente, u64::from(aeda));
    }

    /// Los residentes y sus épicas cambian la Facción (sus épicas posibles, Mecenas) y la felicidad de la plaza.
    fn tocar_residentes(&mut self, faccion: u32, plaza: u64) {
        self.tr.tocar(Clase::Faccion, u64::from(faccion));
        self.tr.tocar(Clase::Asentamiento, plaza);
    }

    /// La plaza cayó en ruinas (ya no está en el estado): sus residentes desaparecen con su espera y su cita, sin hecho (como el TypeScript).
    pub(super) fn residentes_tras_la_ruina(&mut self, plaza: u64) {
        let fuera = self.estado.residentes.quitar_plaza(plaza as u32);
        self.tr.tocar_todos(Clase::Residente, fuera.iter().map(|&a| u64::from(a)));
        if !fuera.is_empty() {
            self.espacio.residentes_por_plaza.remove(&plaza);
        }
        self.programar_llegada(plaza, None);
    }

    /// La plaza cambió de dueño en una conquista: sus residentes pasan a la conquistadora y **pierden la épica** (`aedas.epica_abandonada { motivo: conquista }`, a la Facción de antes); la plaza se
    /// revisa (la conquista daña edificios y cambia la reputación que mira la llegada).
    pub(super) fn residentes_tras_la_conquista(&mut self, t: Instante, plaza: u64, vieja: u32, nueva: u32) {
        let pasan: Vec<u64> = self.estado.residentes.de_plaza(plaza as u32).iter().map(|a| u64::from(a.id)).collect();
        self.tr.tocar_todos(Clase::Residente, pasan);
        for (aeda, tec) in self.estado.residentes.cambiar_de_dueno(plaza as u32, nueva) {
            self.epica_perdida(t, aeda, vieja, tec, Some(plaza), "conquista");
        }
        self.revisar_residentes(plaza, t);
    }

    /// Anexión y fusión (decisión técnica 13): los residentes de `origen` pasan a `destino` con su épica, y sus épicas cumplidas se suman. La épica de una tecnología que `destino` ya tiene se pierde
    /// (`aparecida`); dos de la misma tecnología se unifican y sigue la más avanzada (`unificada`, decisión del autor). Las plazas movidas se revisan (la reputación de la dueña cambió).
    pub(super) fn trasladar_residentes(&mut self, t: Instante, origen: u32, destino: u32, plazas: &[u64]) {
        let res = &mut self.estado.residentes;
        let pasan: Vec<u64> = res.aedas.iter().filter(|a| a.faccion == origen).map(|a| u64::from(a.id)).collect();
        self.tr.tocar_todos(Clase::Residente, pasan);
        let res = &mut self.estado.residentes;
        res.aedas.iter_mut().filter(|a| a.faccion == origen).for_each(|a| a.faccion = destino);
        if let Some(n) = res.cumplidas.remove(&origen) {
            *res.cumplidas.entry(destino).or_insert(0) += n;
        }
        let r = self.estado.reglas.economia.tecnologia.clone();
        let aparecidas = self.estado.tecnologia.de(&r, destino).aparecidas;
        let sobran: Vec<TecId> = self.estado.residentes.aedas.iter().filter(|a| a.faccion == destino).filter_map(|a| a.epica.as_ref().map(|e| e.tec)).filter(|&x| aparecidas.contiene(x)).collect();
        for x in sobran {
            self.epicas_sobrantes(t, destino, x);
        }
        for (aeda, x) in self.estado.residentes.unificar(destino) {
            let plaza = self.estado.residentes.get(aeda).map(|a| u64::from(a.plaza));
            self.epica_perdida(t, aeda, destino, x, plaza, "unificada");
        }
        for &p in plazas {
            self.revisar_residentes(p, t);
        }
    }

    /// La tecnología `x` le apareció a la Facción: las épicas suyas de esa tecnología sobran (`aedas.epica_abandonada { motivo: aparecida }`).
    pub(super) fn epicas_sobrantes(&mut self, t: Instante, faccion: u32, x: TecId) {
        let sobran = self.estado.residentes.al_aparecer(faccion, x);
        if sobran.is_empty() {
            return;
        }
        for aeda in sobran {
            let plaza = self.estado.residentes.get(aeda).map(|a| u64::from(a.plaza));
            self.epica_perdida(t, aeda, faccion, x, plaza, "aparecida");
        }
    }

    /// `aedas.epica_abandonada { aedaResidente, faccion, plaza; tecnologia, motivo }`, privado de la Facción que la cantaba.
    fn epica_perdida(&mut self, t: Instante, aeda: u32, faccion: u32, x: TecId, plaza: Option<u64>, motivo: &'static str) {
        let mut datos = vec![("aedaResidente", i64::from(aeda)), ("faccion", i64::from(faccion))];
        datos.extend(plaza.map(|p| ("plaza", p as i64)));
        let tecnologia = self.estado.reglas.economia.tecnologia.def(x).id.clone();
        let valores = vec![("tecnologia", Dato::Texto(tecnologia)), ("motivo", Dato::Texto(motivo.into()))];
        self.tr.emitir(&self.estado, EventoNucleo { facciones: vec![faccion], valores, ..EventoNucleo::nuevo("aedas.epica_abandonada", t, datos) });
        self.tr.tocar(Clase::Faccion, u64::from(faccion));
        self.tocar_residente(aeda);
    }

    // ───────────────────────────── Las órdenes ─────────────────────────────

    /// `EmpezarEpica` y `AbandonarEpica` (TA §7.2): el héroe de la cuenta dirige a los Aedas de la plaza (`dirigeAedas`). Los rechazos no tocan nada.
    pub(super) fn ejecutar_epica(&mut self, t: Instante, p: &Peticion) -> Result<Vec<u64>, Respuesta> {
        let (heroe, plaza, aeda, tecnologia) = match p.comando {
            Comando::EmpezarEpica { heroe, asentamiento, aeda, tecnologia } => (heroe, asentamiento, aeda, Some(tecnologia)),
            Comando::AbandonarEpica { heroe, asentamiento, aeda } => (heroe, asentamiento, aeda, None),
            _ => unreachable!("solo las órdenes de las épicas"),
        };
        let v = self.estado.version;
        let rechazo = |(c, m): (&'static str, &str)| Respuesta::rechazo(c, m, v);
        let duena = self.plaza_o_rechazo(plaza)?.faccion;
        self.autorizar_en_plaza(p, plaza, Some(heroe))?;
        let r = self.estado.reglas.economia.tecnologia.clone();
        let (codigo, x, mut datos, motivo) = match tecnologia {
            Some(x) => {
                if usize::from(x) >= r.catalogo.len() {
                    return Err(rechazo((rechazos::TECNOLOGIA_DESCONOCIDA, "esa tecnología no existe")));
                }
                let capitulos = self.estado.residentes.empezar_epica(&r, &self.estado.tecnologia, plaza as u32, duena, aeda, x).map_err(|e| rechazo(rechazo_de_epica(e)))?;
                ("aedas.epica_empieza", x, vec![("capitulo", 0), ("capitulos", i64::from(capitulos))], None)
            }
            None => {
                let x = self.estado.residentes.abandonar_epica(plaza as u32, aeda).map_err(|e| rechazo(rechazo_de_epica(e)))?;
                ("aedas.epica_abandonada", x, vec![], Some("orden"))
            }
        };
        datos.extend([("aedaResidente", i64::from(aeda)), ("faccion", i64::from(duena))]);
        let mut valores = vec![("tecnologia", Dato::Texto(r.def(x).id.clone()))];
        valores.extend(motivo.map(|m| ("motivo", Dato::Texto(m.into()))));
        let punto = self.estado.asentamientos.get(&plaza).map(|a| a.posicion);
        self.tr.emitir(&self.estado, EventoNucleo { asentamientos: vec![plaza], punto, valores, ..EventoNucleo::nuevo(codigo, t, datos) });
        self.tr.tocar(Clase::Faccion, u64::from(duena));
        self.tocar_residente(aeda);
        Ok(vec![])
    }

    // ───────────────────────────── Los hechos de épica ─────────────────────────────

    /// Anota un hecho de épica de la transición en curso (se procesa al cerrar la versión).
    pub(super) fn hecho_epico(&mut self, h: Option<Hecho>) {
        if let Some(h) = h {
            self.hechos_epicos.push(h);
        }
    }

    /// `comercio.caravana_llega` (D9): el hecho de épica `caravana` de una caravana comercial que llega con carga total ≥ `cargaMinimaCaravana` (milésimas), de su plaza de origen si sigue siendo de
    /// alguien.
    pub(super) fn hecho_de_caravana(&mut self, caravana: u64, comercial: bool, origen: u64, carga: Mil) {
        let faccion = self.estado.asentamientos.get(&origen).map(|a| a.faccion);
        let h = Hecho::caravana(&self.estado.reglas.economia.tecnologia, caravana, comercial, origen as u32, faccion, carga);
        self.hecho_epico(h);
    }

    /// Al cerrar la versión (antes de la aparición por hito): los hechos de la transición avanzan las épicas, a lo sumo uno por Aeda (TA §7.2).
    pub(super) fn avanzar_epicas(&mut self, t: Instante) {
        if self.hechos_epicos.is_empty() {
            return;
        }
        let hechos = std::mem::take(&mut self.hechos_epicos);
        if !self.estado.residentes.aedas.iter().any(|a| a.epica.is_some()) {
            return;
        }
        let r = self.estado.reglas.economia.tecnologia.clone();
        let Estado { residentes, tecnologia, asentamientos, .. } = &mut self.estado;
        let avances = residentes.avanzar_epicas(&r, tecnologia, &hechos, |p| asentamientos.get(&u64::from(p)).map(|a| a.faccion), t);
        for av in avances {
            match av {
                Avance::Hecho { aeda } => {
                    if let Some(a) = self.estado.residentes.get(aeda) {
                        let f = a.faccion;
                        self.tr.tocar(Clase::Faccion, u64::from(f));
                    }
                    self.tocar_residente(aeda);
                }
                Avance::Capitulo { aeda, plaza, faccion, tec, capitulo, capitulos } => {
                    self.hecho_de_epica(t, "aedas.epica_capitulo", aeda, plaza, faccion, tec, capitulo, capitulos, &r);
                }
                Avance::Cumplida { aeda, plaza, faccion, tec, capitulos, primera } => {
                    // La tecnología ya apareció (lo hace el crate): las citas de su descubrimiento, si es la primera del mundo, y lo que mire su hito.
                    if primera {
                        self.citas_del_descubrimiento(tec, t);
                    }
                    self.epicas_sobrantes(t, faccion, tec);
                    self.mirar_aparicion(faccion);
                    self.hecho_de_epica(t, "aedas.epica_tecnologia", aeda, plaza, faccion, tec, capitulos, capitulos, &r);
                    // Pública y sin nombrar la tecnología: la épica se canta, no su premio (Doc 6.3). A la crónica.
                    self.tr.emitir(&self.estado, EventoNucleo { publico: true, ..EventoNucleo::nuevo("aedas.epica_cumplida", t, vec![("aedaResidente", i64::from(aeda)), ("faccion", i64::from(faccion))]) });
                }
            }
        }
    }

    /// `aedas.epica_capitulo` o `aedas.epica_tecnologia` `{ aedaResidente, faccion, capitulo, capitulos; tecnologia }`, privado de la plaza (`capitulo` es el que empieza, como el payload del TS).
    #[allow(clippy::too_many_arguments)]
    fn hecho_de_epica(&mut self, t: Instante, codigo: &'static str, aeda: u32, plaza: u32, faccion: u32, x: TecId, capitulo: u8, capitulos: u8, r: &tecnologia::Reglas) {
        let plaza = u64::from(plaza);
        let datos = vec![("aedaResidente", i64::from(aeda)), ("faccion", i64::from(faccion)), ("capitulo", i64::from(capitulo)), ("capitulos", i64::from(capitulos))];
        let valores = vec![("tecnologia", Dato::Texto(r.def(x).id.clone()))];
        let punto = self.estado.asentamientos.get(&plaza).map(|a| a.posicion);
        self.tr.emitir(&self.estado, EventoNucleo { asentamientos: vec![plaza], punto, valores, ..EventoNucleo::nuevo(codigo, t, datos) });
        self.tr.tocar(Clase::Faccion, u64::from(faccion));
        self.tocar_residente(aeda);
    }
}

#[cfg(test)]
mod pruebas {
    use super::super::construccion::factores_extra;
    use asentamiento::datos::UNO;

    /// D11: la felicidad de una plaza ocupada con Aedas es el producto de la ocupación y de `1 + bono × residentes`; sin ocupación ni residentes, 1 (lo de antes de 6E); la nobleza solo mira a
    /// los residentes (`aedasResidentes.test`, «cada residente suma felicidad»).
    #[test]
    fn la_felicidad_es_el_producto_de_la_ocupacion_y_los_residentes_y_la_nobleza_solo_los_residentes() {
        let r = tecnologia::Reglas::default();
        let (bf, bn) = (r.aedas.bono_felicidad, r.aedas.bono_nobleza);
        assert_eq!(factores_extra(&r, None, 0), (UNO, UNO));
        assert_eq!(factores_extra(&r, None, 2), (UNO + 2 * bf, UNO + 2 * bn));
        assert_eq!(factores_extra(&r, Some(UNO / 2), 0), (UNO / 2, UNO), "sin residentes, la ocupación de siempre");
        assert_eq!(factores_extra(&r, Some(UNO / 2), 4), ((UNO + 4 * bf) / 2, UNO + 4 * bn), "con los dos, el producto");
    }
}
