//! La Facción como entidad, la ciudadanía y la residencia (sistema 5, tanda 5B; Diplomacia_Facciones §3.1-§3.4 y §4.1-§4.2, Doc 2.2 y 2.5): crear una Facción, pedir el ingreso y
//! responderlo, dejarla, el Rey y el Embajador, y las tres órdenes de residencia con su cooldown. Las reglas puras son el crate `politica`; aquí están el estado (`Estado::politica`),
//! lo que cada cambio arrastra en el resto del mundo y el traslado a hechos y versiones.
//!
//! # La ciudadanía tiene una sola fuente
//! La Facción guarda sus ciudadanos en orden de ingreso (`politica::Facciones`, con su índice héroe → Facción); `HeroeNucleo::faccion` es su copia para quien la lee en caliente (encuentros,
//! visión, autorización...) y solo la escribe [`Nucleo::tras_cambiar_de_faccion`], que además da la Facción a la columna de quien va solo (§3.1-§3.3) y rehace la demanda de bandidos del
//! campamento donde resida (un residente sin plazas los pide). Lo que anduvo sin bandera se funde con la memoria de la Facción al cerrar la versión (`exploracion`, §4.3), porque el héroe queda tocado.
//!
//! # La administración opera sobre las mismas reglas (decisión 9 del plan del sistema 5)
//! `FijarFaccionDeHeroe` quita o da la ciudadanía con la sucesión del trono, sin cooldown, sin Q7 y sin tocar la residencia; si la Facción que nombra no existe, nace vacía («Facción N», con su
//! sigilo libre) y quien entra es su Rey (5A-8). `CrearEjercito` hace ciudadano a su héroe de campo por la misma vía. `FijarResidencia` es la reubicación forzosa (`mercenarios`): sin cooldown.
//!
//! # Quien deja su casa estando dentro (5B-1, decisión del autor del 2026-10-24)
//! Su campamento (las escuadras) y su almacén personal van con la residencia, así que pasan sin viaje a la nueva (Doc 2.5 y 5.15.5): no hay nada que mover. El héroe **sigue dentro como
//! visitante** si la puerta le admite (un campamento de mercenarios admite a todos), con su carro y sus víveres; si dejó la Facción y la puerta ya no le admite, sale expulsado al mapa
//! **solo** (sin tropa: la tropa es de la residencia nueva), con su carro (`heroe.expulsado`, motivo `puerta_cerrada`).
//!
//! # No se muda nadie de una plaza en batalla (5B-10)
//! Las tres órdenes de residencia y dejar la Facción se rechazan (`batalla.bloqueado`) si la plaza donde reside está en una batalla activa, como ya se rechazaba mudarse a una.

use super::*;
use crate::estado::Dato;
use militar::columna::TipoColumna;
use militar::convocatoria::Lugar;
use militar::heroe::Residencia;
use militar::presencia::Ubicacion;
use politica::autorizacion::{Actor, Vista};
use politica::faccion::ErrorFaccion;
use politica::residencia::{self, ErrorResidencia};
use politica::Cargos;

/// La autorización de la política mira las plazas a través del núcleo. Los cargos locales llegan en 5E: hasta entonces ninguna plaza tiene titulares.
impl Vista for Nucleo {
    fn duena(&self, a: u32) -> Option<u32> {
        self.estado.asentamientos.get(&u64::from(a)).map(|p| p.faccion)
    }
    fn residente(&self, h: u64, a: u32) -> bool {
        self.heroe(h).is_some_and(|x| x.heroe.residencia == Some(Residencia::Asentamiento(a)))
    }
    fn presente(&self, h: u64, a: u32) -> bool {
        self.heroe(h).is_some_and(|x| x.heroe.presencia.ubicacion == Ubicacion::Asentamiento(a))
    }
    fn cargos(&self, _a: u32) -> Cargos {
        Cargos::default()
    }
}

fn rechazo_de_faccion(e: ErrorFaccion) -> (&'static str, String) {
    use rechazos::*;
    match e {
        ErrorFaccion::NoExiste => (FACCION_NO_EXISTE, "la Facción no existe".into()),
        ErrorFaccion::NombreVacio => (FACCION_NOMBRE_VACIO, "el nombre queda vacío".into()),
        ErrorFaccion::NombreDuplicado => (FACCION_NOMBRE_DUPLICADO, "otra Facción ya se llama así".into()),
        ErrorFaccion::SigiloInvalido => (FACCION_SIGILO_INVALIDO, "el sigilo lleva piezas que no son del catálogo o repite el color del fondo".into()),
        ErrorFaccion::SigiloDuplicado => (FACCION_SIGILO_DUPLICADO, "otra Facción lleva ese mismo sigilo".into()),
        ErrorFaccion::YaPertenece => (FACCION_YA_PERTENECE, "ya es ciudadano de una Facción: 1 héroe, 1 Facción".into()),
        ErrorFaccion::CooldownCreacion { libre_en } => (FACCION_COOLDOWN_CREACION, format!("dejó una Facción hace poco: puede crear otra desde {libre_en}")),
        ErrorFaccion::NoHaPedido => (FACCION_NO_HA_PEDIDO, "ese héroe no ha pedido entrar".into()),
        ErrorFaccion::NoPertenece => (FACCION_NO_PERTENECE, "no es ciudadano de ninguna Facción".into()),
        ErrorFaccion::EnEjercito => (FACCION_EN_EJERCITO, "va en un ejército o en un ejército en preparación: primero se separa (y si es el Líder, cede antes el mando)".into()),
        ErrorFaccion::NoCiudadano => (FACCION_NO_CIUDADANO, "el designado no es ciudadano de la Facción".into()),
        ErrorFaccion::SinRey => (FACCION_SIN_REY, "la Facción no tiene Rey".into()),
    }
}

fn rechazo_de_residencia(e: ErrorResidencia) -> (&'static str, String) {
    use rechazos::*;
    match e {
        ErrorResidencia::Cooldown { libre_en } => (RESIDENCIA_COOLDOWN, format!("cambió de residencia hace poco: puede volver a hacerlo desde {libre_en}")),
        ErrorResidencia::PlazaAjena => (RESIDENCIA_PLAZA_AJENA, "solo se reside en una plaza de la propia Facción".into()),
        ErrorResidencia::YaReside => (RESIDENCIA_YA_RESIDE, "ya reside ahí".into()),
        ErrorResidencia::Vetado => (RESIDENCIA_VETADO, "la plaza no le admite como residente".into()),
    }
}

impl Nucleo {
    // ───────────────────────────── Consultas ─────────────────────────────

    /// Las Facciones como entidad, por id.
    pub fn facciones(&self) -> impl Iterator<Item = &politica::faccion::Faccion> {
        self.estado.politica.facciones.iter()
    }

    pub fn faccion(&self, id: u32) -> Option<&politica::faccion::Faccion> {
        self.estado.politica.facciones.get(id)
    }

    // ───────────────────────────── Ayudas ─────────────────────────────

    /// El id de una Facción nueva: el siguiente a la mayor que se nombra en el mundo (las que nombró la administración sin crearlas también cuentan) o que desapareció en una anexión o una
    /// fusión (5F: un id no se vuelve a dar).
    pub(super) fn siguiente_faccion(&self) -> u32 {
        let mayor = crate::proyeccion::audiencia::facciones(&self.estado).last().copied().unwrap_or(0);
        mayor.max(self.estado.politica.mayor_desaparecida) + 1
    }

    /// Las Facciones a las que `heroe` ha pedido entrar: ganar una ciudadanía borra sus solicitudes en todas, y esas Facciones cambian.
    fn pedidas_por(&self, heroe: u64) -> Vec<u64> {
        self.facciones().filter(|f| f.solicitudes.contains(&heroe)).map(|f| u64::from(f.id)).collect()
    }

    /// La administración da la ciudadanía de `faccion` a `heroe`, que no tiene ninguna (decisión 9): si la Facción no existe, nace vacía y él es su Rey. Solo la Facción: lo demás es de
    /// [`Nucleo::tras_cambiar_de_faccion`].
    pub(super) fn otorgar_de_administracion(&mut self, heroe: u64, faccion: u32) {
        let r = self.estado.reglas.economia.politica.clone();
        let pidio = self.pedidas_por(heroe);
        let fs = &mut self.estado.politica.facciones;
        fs.crear_vacia(&r, faccion);
        fs.otorgar(faccion, heroe).expect("el héroe no es ciudadano de otra");
        self.tr.tocar_todos(Clase::Faccion, pidio.into_iter().chain([u64::from(faccion)]));
    }

    /// Lo que arrastra que `heroe` cambie de Facción (la Facción ya lo sabe; `antes` es la de antes): su copia en el héroe, su columna si va solo (§3.1-§3.3; la de un ejército no cambia), la
    /// demanda de bandidos del campamento donde reside y las dos Facciones en el registro de cambios.
    pub(super) fn tras_cambiar_de_faccion(&mut self, t: Instante, heroe: u64, antes: Option<u32>) {
        let ahora = self.estado.politica.facciones.faccion_de(heroe);
        self.tr.tocar_todos(Clase::Faccion, antes.into_iter().chain(ahora).map(u64::from));
        let h = self.heroe_mut(heroe);
        h.faccion = ahora;
        let (columna, residencia) = (h.columna, h.heroe.residencia);
        if let Some(e) = columna.and_then(|c| self.estado.ejercitos.get_mut(&c)).filter(|e| e.columna.participantes.len() == 1) {
            e.columna.faccion = ahora;
            self.tr.tocar(Clase::Ejercito, e.id);
        }
        if let Some(Residencia::Campamento(c)) = residencia {
            self.reprogramar_anillo(c, t);
        }
    }

    /// ¿Va en un ejército (aunque sea solo) o en uno en preparación? Q7 y 5A-4: no deja la Facción. DUDA: la regla dice «una convocatoria con otros héroes»; se rechaza también la de uno
    /// solo (es de su Facción y, si se fuera, la convocaría un extranjero).
    fn en_ejercito(&self, heroe: u64) -> bool {
        let columna = self.estado.ejercito_de_heroe(heroe).and_then(|c| self.estado.ejercitos.get(&c));
        columna.is_some_and(|e| e.columna.tipo == TipoColumna::Ejercito) || self.estado.militar.convocatoria_de(heroe).is_some()
    }

    /// El campamento de mercenarios más cercano a `p` (empate: el de menor índice).
    fn campamento_mas_cercano(&self, p: Punto) -> Option<u32> {
        self.puertas_de_campamentos().min_by_key(|&(i, c)| (c.dist2(p), i)).map(|(i, _)| i)
    }

    /// 5B-10 (2026-10-24): nadie se muda de una plaza en batalla (su campamento se iría con él), como nadie se muda a una.
    fn sin_batalla_en_su_plaza(&self, heroe: u64) -> Result<(), Respuesta> {
        match self.heroe(heroe).and_then(|h| h.heroe.residencia) {
            Some(Residencia::Asentamiento(a)) if self.batallas_activas().next().is_some() && self.bloqueos().asentamientos.contains(&a) => {
                Err(Respuesta::rechazo(rechazos::BATALLA_BLOQUEADO, "tu plaza está en una batalla: nadie se muda de ella hasta que acabe", self.estado.version))
            }
            _ => Ok(()),
        }
    }

    /// El héroe pasa a residir en `nueva` por una orden suya: comprueba el cooldown, que no esté en un ejército en preparación y que su plaza no esté en batalla, se muda y lo estampa.
    // DUDA: el cooldown y el ejército en preparación son de la regla; que no se cambie de residencia en una convocatoria no lo dice nadie, pero su tropa se iría con él a otra casa.
    fn mudarse_por_orden(&mut self, t: Instante, heroe: u64, nueva: Option<Residencia>) -> Result<(), Respuesta> {
        self.sin_convocatoria(heroe)?;
        self.sin_batalla_en_su_plaza(heroe)?;
        self.mudar_de_residencia(heroe, nueva);
        self.heroe_mut(heroe).residencia_cambiada_en = Some(t);
        Ok(())
    }

    // ───────────────────────────── Órdenes ─────────────────────────────

    /// Las órdenes de la Facción, la ciudadanía y la residencia, y la ciudadanía de la administración. `Ok(ids)` = cambió algo y creó esos ids (la Facción nueva); los rechazos y «sin cambios»
    /// salen como `Err` sin haber tocado el estado.
    pub(super) fn ejecutar_politica(&mut self, t: Instante, p: &Peticion) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let sin_cambios = || Respuesta::sin_cambios(v);
        let de_faccion = |e: ErrorFaccion| {
            let (c, m) = rechazo_de_faccion(e);
            rechazo(c, m)
        };
        let de_residencia = |e: ErrorResidencia| {
            let (c, m) = rechazo_de_residencia(e);
            rechazo(c, m)
        };
        let r = self.estado.reglas.economia.politica.clone();
        let existe = |n: &Nucleo, f: u32| if n.estado.politica.facciones.existe(f) { Ok(()) } else { Err(de_faccion(ErrorFaccion::NoExiste)) };
        match &p.comando {
            Comando::CrearFaccion { heroe, nombre, sigilo } => {
                self.autorizar_heroe(p, *heroe, false)?;
                let (id, pidio) = (self.siguiente_faccion(), self.pedidas_por(*heroe));
                self.estado.politica.facciones.crear(&r, id, *heroe, nombre, sigilo.clone(), t).map_err(de_faccion)?;
                self.tr.tocar_todos(Clase::Faccion, pidio);
                self.tras_cambiar_de_faccion(t, *heroe, None);
                let nombre = self.estado.politica.facciones.get(id).expect("acaba de nacer").nombre.clone();
                self.tr.emitir(&self.estado, EventoNucleo { valores: vec![("nombre", Dato::Texto(nombre))], ..EventoNucleo::nuevo("faccion.creada", t, vec![("faccion", i64::from(id)), ("heroe", *heroe as i64)]) });
                Ok(vec![u64::from(id)])
            }
            Comando::SolicitarIngreso { heroe, faccion } => {
                self.autorizar_heroe(p, *heroe, false)?;
                let en_ejercito = self.en_ejercito(*heroe);
                if !self.estado.politica.facciones.solicitar(*faccion, *heroe, en_ejercito).map_err(de_faccion)? {
                    return Err(sin_cambios());
                }
                self.tr.tocar(Clase::Faccion, u64::from(*faccion));
                self.emitir_propio(t, "faccion.solicitud_ingreso", vec![], vec![("faccion", i64::from(*faccion)), ("heroe", *heroe as i64)]);
                Ok(vec![])
            }
            Comando::ResponderSolicitud { heroe, faccion, solicitante, aceptar } => {
                self.autorizar_heroe(p, *heroe, false)?;
                existe(self, *faccion)?;
                if !(Actor { yo: *heroe, politica: &self.estado.politica, vista: self }).responde_solicitudes(*faccion) {
                    return Err(rechazo(rechazos::NO_AUTORIZADO, "solo el Rey de la Facción responde a las solicitudes".into()));
                }
                let (pidio, en_ejercito) = (self.pedidas_por(*solicitante), self.en_ejercito(*solicitante));
                self.estado.politica.facciones.responder(*faccion, *solicitante, *aceptar, en_ejercito).map_err(de_faccion)?;
                self.tr.tocar_todos(Clase::Faccion, pidio);
                let codigo = if *aceptar {
                    self.tras_cambiar_de_faccion(t, *solicitante, None);
                    "faccion.ciudadania_union"
                } else {
                    "faccion.solicitud_denegada"
                };
                self.emitir_propio(t, codigo, vec![], vec![("faccion", i64::from(*faccion)), ("heroe", *solicitante as i64)]);
                Ok(vec![])
            }
            Comando::DejarFaccion { heroe } => {
                self.autorizar_heroe(p, *heroe, false)?;
                let en_ejercito = self.en_ejercito(*heroe);
                // DUDA: 5B-10 habla de mudarse; dejar la Facción también muda a quien reside en una plaza, así que se bloquea igual (después de los rechazos de la regla).
                if self.estado.politica.facciones.faccion_de(*heroe).is_some() && !en_ejercito {
                    self.sin_batalla_en_su_plaza(*heroe)?;
                }
                let salida = self.estado.politica.facciones.dejar(*heroe, t, en_ejercito).map_err(de_faccion)?;
                self.tras_cambiar_de_faccion(t, *heroe, Some(salida.faccion));
                // Sin ciudadanía no se reside en una plaza: al campamento de mercenarios más cercano a la que deja (§3.3, decisión 10 y 5B-4; en uno de mercenarios, sigue allí). No cuenta
                // para el cooldown de residencia. Si estaba dentro, sigue de visita si la puerta le admite (5B-1).
                let mut datos = vec![("faccion", i64::from(salida.faccion)), ("heroe", *heroe as i64)];
                if let Some(Residencia::Asentamiento(a)) = self.heroe(*heroe).expect("existe").heroe.residencia {
                    let campamento = self.puerta_de(Lugar::Asentamiento(a)).and_then(|p| self.campamento_mas_cercano(p));
                    self.mudar_de_residencia(*heroe, campamento.map(Residencia::Campamento));
                    datos.push(("asentamiento", i64::from(a)));
                    datos.extend(campamento.map(|c| ("campamento", i64::from(c))));
                }
                // Quien está dentro de un lugar cuya puerta ya no le admite sin la Facción sale expulsado al mapa, solo y con su carro (5B-1).
                datos.extend(self.echar_si_ya_no_entra(t, *heroe).map(|c| ("ejercito", c as i64)));
                self.emitir_propio(t, "faccion.abandonada", vec![], datos);
                Ok(vec![])
            }
            Comando::AsignarRey { heroe, faccion, designado } => {
                self.autorizar_heroe(p, *heroe, false)?;
                existe(self, *faccion)?;
                if !(Actor { yo: *heroe, politica: &self.estado.politica, vista: self }).asigna_rey(*faccion) {
                    return Err(rechazo(rechazos::NO_AUTORIZADO, "nombra al Rey un ciudadano si no lo hay; si lo hay, solo él traspasa el trono".into()));
                }
                if self.estado.politica.facciones.get(*faccion).is_some_and(|f| f.rey == Some(*designado)) {
                    return Err(sin_cambios());
                }
                self.estado.politica.facciones.asignar_rey(*faccion, *designado).map_err(de_faccion)?;
                self.tr.tocar(Clase::Faccion, u64::from(*faccion));
                self.emitir_propio(t, "cargo.rey_asignado", vec![], vec![("faccion", i64::from(*faccion)), ("heroe", *designado as i64)]);
                Ok(vec![])
            }
            Comando::AsignarEmbajador { heroe, faccion, designado } => {
                self.autorizar_heroe(p, *heroe, false)?;
                existe(self, *faccion)?;
                if !(Actor { yo: *heroe, politica: &self.estado.politica, vista: self }).rey_de_faccion(*faccion) {
                    return Err(rechazo(rechazos::NO_AUTORIZADO, "solo el Rey nombra al Embajador".into()));
                }
                if self.estado.politica.facciones.get(*faccion).is_some_and(|f| f.embajador == Some(*designado)) {
                    return Err(sin_cambios());
                }
                self.estado.politica.facciones.asignar_embajador(*faccion, *designado).map_err(de_faccion)?;
                self.tr.tocar(Clase::Faccion, u64::from(*faccion));
                self.emitir_propio(t, "cargo.embajador_asignado", vec![], vec![("faccion", i64::from(*faccion)), ("heroe", *designado as i64)]);
                Ok(vec![])
            }
            Comando::CambiarResidencia { heroe, asentamiento } => {
                self.autorizar_heroe(p, *heroe, false)?;
                let duena = self.plaza_o_rechazo(*asentamiento)?.faccion;
                let h = self.heroe(*heroe).expect("existe");
                let (origen, destino) = (h.heroe.residencia, Residencia::Asentamiento(*asentamiento as u32));
                // DUDA: los vetos llegan en 5E; hasta entonces ninguna plaza veta.
                residencia::puede_mudarse(&r, h.residencia_cambiada_en, t, h.faccion, duena, origen == Some(destino), false).map_err(de_residencia)?;
                self.mudarse_por_orden(t, *heroe, Some(destino))?;
                let mut datos = vec![("heroe", *heroe as i64), ("asentamiento", *asentamiento as i64)];
                if let Some(Residencia::Asentamiento(o)) = origen {
                    datos.push(("origen", i64::from(o)));
                }
                self.emitir_propio(t, "ciudadania.residencia_cambiada", vec![*asentamiento], datos);
                Ok(vec![])
            }
            Comando::ResidirEnCampamento { heroe, campamento } => {
                self.autorizar_heroe(p, *heroe, false)?;
                self.puerta_de_campamento(*campamento).ok_or_else(|| rechazo(rechazos::CAMPAMENTO_DESCONOCIDO, ids::campamento(*campamento)))?;
                let h = self.heroe(*heroe).expect("existe");
                residencia::sin_cooldown(&r, h.residencia_cambiada_en, t).map_err(de_residencia)?;
                if h.heroe.residencia == Some(Residencia::Campamento(*campamento)) {
                    return Err(de_residencia(ErrorResidencia::YaReside));
                }
                self.mudarse_por_orden(t, *heroe, Some(Residencia::Campamento(*campamento)))?;
                let datos = vec![("heroe", *heroe as i64), ("campamento", i64::from(*campamento))];
                self.emitir_propio(t, "ciudadania.residencia_en_campamento", vec![], datos);
                Ok(vec![])
            }
            Comando::DejarResidencia { heroe } => {
                self.autorizar_heroe(p, *heroe, false)?;
                let h = self.heroe(*heroe).expect("existe");
                let Some(Residencia::Asentamiento(a)) = h.heroe.residencia else {
                    return Err(rechazo(rechazos::RESIDENCIA_SIN_PLAZA, "no reside en ninguna plaza".into()));
                };
                residencia::sin_cooldown(&r, h.residencia_cambiada_en, t).map_err(de_residencia)?;
                // Al campamento de mercenarios más cercano a la plaza que deja (§4.2, 5B-4, 2026-10-24).
                let campamento = self.puerta_de(Lugar::Asentamiento(a)).and_then(|p| self.campamento_mas_cercano(p));
                self.mudarse_por_orden(t, *heroe, campamento.map(Residencia::Campamento))?;
                let mut datos = vec![("heroe", *heroe as i64), ("asentamiento", i64::from(a))];
                datos.extend(campamento.map(|c| ("campamento", i64::from(c))));
                self.emitir_propio(t, "ciudadania.residencia_dejada", vec![u64::from(a)], datos);
                Ok(vec![])
            }
            Comando::FijarFaccionDeHeroe { heroe, faccion } => {
                self.autorizar_heroe(p, *heroe, true)?;
                let antes = self.heroe(*heroe).expect("existe").faccion;
                if antes == *faccion {
                    return Err(sin_cambios());
                }
                self.estado.politica.facciones.quitar(*heroe);
                if let Some(f) = faccion {
                    self.otorgar_de_administracion(*heroe, *f);
                }
                self.tras_cambiar_de_faccion(t, *heroe, antes);
                Ok(vec![])
            }
            _ => unreachable!("solo se llama con órdenes de política"),
        }
    }
}
