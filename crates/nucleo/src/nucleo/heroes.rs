//! Los héroes y su tropa dentro del núcleo (Heroe_Y_Tropa, Presencia_Y_Convocatoria §7, Reloj_Y_Eventos §7, fase 4B-1): crear al héroe, reclutar
//! en una plaza, loadouts, orden de las escuadras, puntos, guarnición con su ración, escolta de caravanas, conectarse y desconectarse, y el
//! andamiaje de administración que sustituye al sistema político hasta que llegue (Facción, tecnologías, colocar y herir). Las
//! reglas puras son el crate `militar`; aquí están el estado, el reloj, la autorización provisional y el traslado a eventos y versiones. Reclutar
//! en un campamento, la tropa prestada y la residencia están en [`mercenarios`](super::mercenarios).
//!
//! # La ración de la guarnición sin coste por plaza ociosa
//! El «campamento» de una plaza son las escuadras en `Contenedor::Campamento` de los héroes que residen en ella, más las que dejaron en la guarnición de sus avanzadas
//! (`Contenedor::Avanzada`: comen el trigo de la madre, Doc 5.17). Come de su almacén en el paso de
//! un minuto de la plaza (que ya existe), justo después de la población: no hay evento nuevo por plaza ni por héroe. Una plaza sin residentes no
//! hace nada (una consulta al índice de residentes). La ración entra además en el `Exterior` de la economía, de modo que la reserva de la
//! auto-construcción y el déficit de trigo cuentan con ella. Con trigo para toda la ración la moral sube; con menos, baja en proporción a lo que
//! faltó (`Escuadra::paso_de_hambre`), y a moral 0 cada minuto deserta `ceil(5 %)` de la escuadra (`tropas.desercion`, atribuido a la plaza donde se
//! reclutó). Es la regla del TypeScript (`avanzarRacion`), minuto a minuto.
//!
//! # Lo provisional
//! Quien manda en un héroe es su cuenta (o la administración). La Facción del héroe, sus tecnologías, dónde está y cuándo se hiere los fija la
//! administración hasta que existan el sistema político (ciudadanía, cargos, residencia completa), el de tecnología y el combate. Desconectarse con la
//! columna en el mapa tiene su regla en [`columnas`](super::columnas) (`sale_del_mundo`).

use super::*;
use crate::estado::PRIO_SALIDA;
use crate::estado_militar::HeroeNucleo;
use crate::nucleo::flota::rechazo_de_caravana;
use asentamiento::datos::UNO;
use asentamiento::Recurso;
use comercio::caravana::{CandidataEscolta, Escuadra as EscuadraCedida};
use comercio::evento::Evento as EventoComercio;
use militar::catalogo::Tropa;
use militar::escuadra::Contenedor;
use militar::guarnicion;
use militar::heroe::{Atributos, Controlador, DatosLoadout, ErrorHeroe, Heroe, Residencia};
use militar::liderazgo;
use militar::presencia::{Conexion, Salida, Ubicacion};
use militar::racion;
use militar::reclutamiento::{self, ErrorReclutamiento, Pedido, Plaza};

fn rechazo_de_heroe(e: ErrorHeroe) -> (&'static str, String) {
    use rechazos::*;
    match e {
        ErrorHeroe::PuntosNegativos => (HEROE_PUNTOS_NEGATIVOS, "los puntos a repartir no pueden ser negativos".into()),
        ErrorHeroe::NingunPunto => (HEROE_NINGUN_PUNTO, "el reparto suma 0".into()),
        ErrorHeroe::PuntosInsuficientes { pedidos, sin_gastar } => (HEROE_PUNTOS_INSUFICIENTES, format!("se piden {pedidos} puntos y solo hay {sin_gastar} sin gastar")),
        ErrorHeroe::TopeDeAtributo => (HEROE_TOPE_DE_ATRIBUTO, "un atributo pasaría del tope".into()),
        ErrorHeroe::LoadoutSinNombre => (LOADOUT_SIN_NOMBRE, "el loadout necesita un nombre".into()),
        ErrorHeroe::LoadoutDesconocido => (LOADOUT_DESCONOCIDO, "el héroe no tiene ese loadout".into()),
        ErrorHeroe::EscuadraRepetida => (ESCUADRA_REPETIDA, "una escuadra no puede repetirse".into()),
        ErrorHeroe::EscuadraAjena => (ESCUADRA_AJENA, "la escuadra no es del héroe".into()),
        ErrorHeroe::SuperaLiderazgo => (LIDERAZGO_SUPERADO, "las escuadras pasan del Liderazgo del héroe".into()),
        ErrorHeroe::PerkNoDesbloqueado => (LOADOUT_PERK_NO_DESBLOQUEADO, "el perk no está desbloqueado".into()),
        ErrorHeroe::SinResidencia => (GUARNICION_SIN_RESIDENCIA, "solo se guarnece la plaza donde se reside".into()),
        ErrorHeroe::EnBatalla => (ESCUADRA_EN_BATALLA, "la escuadra está reservada para una batalla".into()),
        ErrorHeroe::NoEnCampamento => (GUARNICION_NO_EN_CAMPAMENTO, "la escuadra no está en el campamento".into()),
        ErrorHeroe::YaEnGuarnicion => (GUARNICION_YA_ASIGNADA, "la escuadra ya está en la guarnición".into()),
        ErrorHeroe::SuperaElCupo { ocupado, cupo } => (GUARNICION_SUPERA_EL_CUPO, format!("la guarnición pasaría a {ocupado} de un cupo de {cupo}")),
        ErrorHeroe::NoEnGuarnicion => (GUARNICION_NO_ASIGNADA, "la escuadra no está en la guarnición".into()),
    }
}

fn rechazo_de_reclutamiento(e: ErrorReclutamiento) -> (&'static str, String) {
    use rechazos::*;
    match e {
        ErrorReclutamiento::NoPuedes => (RECLUTAMIENTO_NO_PUEDES, "ni resides en la plaza ni eres ciudadano de su Facción".into()),
        ErrorReclutamiento::SinTecnologia => (RECLUTAMIENTO_SIN_TECNOLOGIA, "la Facción de la plaza no ha adoptado la tecnología de esa tropa".into()),
        ErrorReclutamiento::Reservada => (RECLUTAMIENTO_RESERVADA, "la escuadra está reservada para una batalla".into()),
        ErrorReclutamiento::YaLaTienesFuera(c) => (RECLUTAMIENTO_YA_LA_TIENES_FUERA, format!("ya tienes esa escuadra y no está aquí ({c:?}): solo se repone donde está")),
        ErrorReclutamiento::SoloReponer => (RECLUTAMIENTO_SOLO_REPONER, "sin residir aquí solo se repone una escuadra que ya tienes en la plaza".into()),
        ErrorReclutamiento::AlTope => (RECLUTAMIENTO_AL_TOPE, "la escuadra ya está completa".into()),
        ErrorReclutamiento::SinPoblacion { hacen_falta, disponibles, .. } => (RECLUTAMIENTO_SIN_POBLACION, format!("hacen falta {hacen_falta} y solo hay {disponibles} que no se necesiten para producir")),
        ErrorReclutamiento::SinEdificio => (RECLUTAMIENTO_SIN_EDIFICIO, "falta el edificio activo de ese nivel".into()),
        ErrorReclutamiento::SinRecursos => (RECLUTAMIENTO_SIN_RECURSOS, "el almacén no cubre el coste".into()),
        ErrorReclutamiento::SinReservaDeTrigo { hacen_falta, hay } => (RECLUTAMIENTO_SIN_RESERVA_DE_TRIGO, format!("el trigo debe cubrir la reserva de {hacen_falta} y hay {hay}")),
    }
}

/// ¿Come esta escuadra del almacén de la plaza donde reside su héroe? La del campamento y la que guarnece una avanzada de la plaza.
fn come_de_la_plaza(e: &militar::escuadra::Escuadra) -> bool {
    matches!(e.contenedor, Contenedor::Campamento | Contenedor::Avanzada(_))
}

impl Nucleo {
    // ───────────────────────────── Consultas ─────────────────────────────

    /// Un héroe por id.
    pub fn heroe(&self, id: u64) -> Option<&HeroeNucleo> {
        self.estado.militar.heroes.get(&id)
    }

    /// Todos los héroes, por id.
    pub fn heroes(&self) -> impl Iterator<Item = &HeroeNucleo> {
        self.estado.militar.heroes.values()
    }

    /// El héroe de una cuenta (uno por cuenta y partida).
    pub fn heroe_de_cuenta(&self, cuenta: &str) -> Option<&HeroeNucleo> {
        self.estado.militar.heroe_de_cuenta(cuenta)
    }

    /// ¿Está herido ahora? Se comprueba al leer: nada se dispara al vencer.
    pub fn heroe_herido(&self, id: u64) -> bool {
        self.heroe(id).is_some_and(|h| h.heroe.esta_herido(self.estado.ahora))
    }

    /// Una escuadra por id, con el héroe que la posee.
    pub fn escuadra(&self, id: u64) -> Option<(&HeroeNucleo, &militar::escuadra::Escuadra)> {
        self.heroes().find_map(|h| h.heroe.escuadra(id).map(|e| (h, e)))
    }

    /// ¿Tiene la Facción adoptada esa tecnología? (las de arranque y las que dio la administración).
    pub fn tiene_tecnologia(&self, faccion: u32, tecnologia: &str) -> bool {
        self.estado.militar.tiene_tecnologia(&self.estado.reglas.economia.militar, faccion, tecnologia)
    }

    /// Cupo de guarnición **por héroe residente** de la plaza, en puntos de Liderazgo (Heroe_Y_Tropa §7). `None` si la plaza no existe.
    pub fn cupo_de_guarnicion(&self, plaza: u64) -> Option<i64> {
        let eco = &self.estado.asentamientos.get(&plaza)?.economia;
        let extra = eco.politicas.suma(&self.estado.reglas.economia.datos.politicas, "cupoGuarnicionExtra");
        Some(guarnicion::cupo(&self.estado.reglas.economia.militar, &eco.edificios, asentamiento::recintos::nivel_del_mejor_completo(&eco.recintos), extra))
    }

    /// Soldados en pie del campamento de la plaza: las escuadras en el campamento de los héroes que residen en ella y las que dejaron en la guarnición de sus avanzadas
    /// (lo que come de su almacén).
    pub fn soldados_del_campamento(&self, plaza: u64) -> i64 {
        self.estado.militar.residentes_de(plaza).flat_map(|h| h.heroe.escuadras.iter()).filter(|e| come_de_la_plaza(e)).map(|e| e.cantidad.max(0)).sum()
    }

    /// Trigo por minuto que come la guarnición de la plaza (ración entera, sin participantes).
    pub(super) fn racion_de_guarnicion(&self, plaza: u64) -> Mil {
        if !self.estado.militar.tiene_residentes(plaza) {
            return 0;
        }
        racion::racion_guarnicion(&self.estado.reglas.economia.militar, self.soldados_del_campamento(plaza))
    }

    // ───────────────────────────── Ayudas ─────────────────────────────

    /// La orden de un héroe: el héroe existe, es de quien la da (o de la administración) y está en el mundo.
    pub(super) fn autorizar_heroe(&self, p: &Peticion, id: u64, aunque_este_fuera: bool) -> Result<(), Respuesta> {
        let v = self.estado.version;
        let h = self.heroe(id).ok_or_else(|| Respuesta::rechazo(rechazos::HEROE_DESCONOCIDO, ids::heroe(id), v))?;
        if !p.admin && h.cuenta != p.actor {
            return Err(Respuesta::rechazo(rechazos::NO_AUTORIZADO, "el héroe no es tuyo", v));
        }
        if h.heroe.presencia.fuera && !aunque_este_fuera {
            return Err(Respuesta::rechazo(rechazos::HEROE_FUERA_DEL_MUNDO, "fuera del mundo solo se puede conectar", v));
        }
        Ok(())
    }

    /// El héroe de una orden ya autorizada.
    pub(super) fn heroe_mut(&mut self, id: u64) -> &mut HeroeNucleo {
        self.tr.tocar(Clase::Heroe, id);
        self.estado.militar.heroes.get_mut(&id).expect("el héroe existe")
    }

    /// La columna en la que va el héroe, si va en una.
    pub(super) fn columna_de(&self, heroe: u64) -> Option<u64> {
        match self.heroe(heroe)?.heroe.presencia.ubicacion {
            Ubicacion::Columna(c) => Some(c),
            _ => None,
        }
    }

    /// Devuelve al campamento las escuadras que dejan de ser escolta de una caravana.
    pub(super) fn devolver_escolta(&mut self, escolta: &[EscuadraCedida]) {
        for EscuadraCedida { id, heroe, .. } in escolta {
            if let Some(e) = self.estado.militar.heroes.get_mut(heroe).and_then(|h| h.heroe.escuadra_mut(*id)) {
                if matches!(e.contenedor, Contenedor::Escolta(_)) {
                    self.tr.tocar(Clase::Heroe, *heroe);
                    e.contenedor = Contenedor::Campamento;
                }
            }
        }
    }

    // ───────────────────────────── Órdenes ─────────────────────────────

    /// Las órdenes de los héroes, su tropa y la escolta. `Ok(ids)` = cambió algo y creó esos ids; rechazos y «sin cambios» salen como `Err` sin haber
    /// tocado el estado.
    pub(super) fn ejecutar_heroes(&mut self, t: Instante, p: &Peticion) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let sin_cambios = || Respuesta::sin_cambios(v);
        let solo_admin = || (!p.admin).then(|| rechazo(rechazos::NO_AUTORIZADO, "solo administración".into()));
        let mr = self.estado.reglas.economia.militar.clone();
        let de_heroe = |e: ErrorHeroe| {
            let (c, m) = rechazo_de_heroe(e);
            rechazo(c, m)
        };
        match &p.comando {
            Comando::CrearHeroe { cuenta, campamento, controlador } => {
                if !p.admin && (cuenta != &p.actor || *controlador == Controlador::Bot) {
                    return Err(rechazo(rechazos::NO_AUTORIZADO, "solo la administración crea héroes ajenos o bots".into()));
                }
                if self.estado.militar.heroe_de_cuenta(cuenta).is_some() {
                    return Err(rechazo(rechazos::HEROE_YA_EXISTE, format!("{cuenta} ya tiene su héroe")));
                }
                if self.puerta_de_campamento(*campamento).is_none() {
                    return Err(rechazo(rechazos::HEROE_CAMPAMENTO_DESCONOCIDO, ids::campamento(*campamento)));
                }
                let (id, loadout) = (self.estado.siguiente_id, self.estado.siguiente_id + 1);
                self.estado.siguiente_id += 2;
                let heroe = Heroe::nuevo(&mr, id, *controlador, *campamento, loadout);
                self.estado.militar.insertar(HeroeNucleo::nuevo(heroe, cuenta, None));
                // Un residente nuevo, sin Facción: pide bandidos en el anillo de su campamento.
                self.reprogramar_anillo(*campamento, t);
                self.emitir_propio(t, "heroe.creado", vec![], vec![("heroe", id as i64), ("campamento", *campamento as i64)]);
                Ok(vec![id])
            }
            Comando::ReclutarTropa { asentamiento, heroe, tropa } => self.reclutar_en_plaza(t, p, *asentamiento, *heroe, *tropa),
            Comando::GuardarLoadout { heroe, loadout, nombre, escuadras, perks, activo } => {
                self.autorizar_heroe(p, *heroe, false)?;
                let datos = DatosLoadout { id: *loadout, nombre: nombre.clone(), escuadras: escuadras.clone(), perks: perks.clone(), activo: *activo };
                let Estado { militar, siguiente_id, .. } = &mut self.estado;
                let h = &mut militar.heroes.get_mut(heroe).expect("existe").heroe;
                let mut nuevo = None;
                h.guardar_loadout(&mr, datos, || {
                    nuevo = Some(*siguiente_id);
                    *siguiente_id += 1;
                    *siguiente_id - 1
                })
                .map_err(de_heroe)?;
                Ok(nuevo.into_iter().collect())
            }
            Comando::BorrarLoadout { heroe, loadout } => {
                self.autorizar_heroe(p, *heroe, false)?;
                self.heroe_mut(*heroe).heroe.borrar_loadout(*loadout).map_err(de_heroe)?;
                Ok(vec![])
            }
            Comando::RepartirPuntos { heroe, fuerza, destreza, armadura, vitalidad } => {
                self.autorizar_heroe(p, *heroe, false)?;
                let reparto = Atributos { fuerza: *fuerza, destreza: *destreza, armadura: *armadura, vitalidad: *vitalidad };
                self.heroe_mut(*heroe).heroe.repartir_atributos(&mr, reparto).map_err(de_heroe)?;
                self.emitir_propio(t, "heroe.puntos_repartidos", vec![], vec![("heroe", *heroe as i64), ("fuerza", *fuerza), ("destreza", *destreza), ("armadura", *armadura), ("vitalidad", *vitalidad)]);
                Ok(vec![])
            }
            Comando::OrdenarEscuadras { heroe, escuadras } => {
                self.autorizar_heroe(p, *heroe, false)?;
                // Si va en una columna, sus escuadras ocupan en ella los mismos huecos, en el orden nuevo: es el orden en que entran en combate.
                let columna = self.estado.ejercito_de_heroe(*heroe);
                let Estado { militar, ejercitos, .. } = &mut self.estado;
                let h = &mut militar.heroes.get_mut(heroe).expect("existe").heroe;
                let antes: Vec<u64> = h.escuadras.iter().map(|e| e.id).collect();
                let ids_columna = columna.and_then(|c| ejercitos.get_mut(&c)).map(|e| &mut e.columna.escuadras);
                let en_columna_antes = ids_columna.as_ref().map(|v| (*v).clone());
                h.ordenar_escuadras(escuadras, ids_columna).map_err(de_heroe)?;
                let columna_igual = columna.and_then(|c| ejercitos.get(&c)).map(|e| &e.columna.escuadras) == en_columna_antes.as_ref();
                if h.escuadras.iter().map(|e| e.id).eq(antes) && columna_igual {
                    return Err(sin_cambios());
                }
                if let Some(c) = columna {
                    self.tr.tocar(Clase::Ejercito, c);
                }
                Ok(vec![])
            }
            Comando::AsignarGuarnicion { heroe, escuadra } => {
                self.autorizar_heroe(p, *heroe, false)?;
                let Some(Residencia::Asentamiento(plaza)) = self.heroe(*heroe).expect("existe").heroe.residencia else { return Err(de_heroe(ErrorHeroe::SinResidencia)) };
                let cupo = self.cupo_de_guarnicion(plaza as u64).ok_or_else(|| de_heroe(ErrorHeroe::SinResidencia))?;
                guarnicion::asignar(&mr, &mut self.heroe_mut(*heroe).heroe, plaza, cupo, *escuadra).map_err(de_heroe)?;
                Ok(vec![])
            }
            Comando::RetirarGuarnicion { heroe, escuadra } => {
                self.autorizar_heroe(p, *heroe, false)?;
                guarnicion::retirar(&mut self.heroe_mut(*heroe).heroe, *escuadra).map_err(de_heroe)?;
                Ok(vec![])
            }
            Comando::PasarAViveres { heroe, cantidad } => {
                self.autorizar_heroe(p, *heroe, false)?;
                let columna = self.columna_de(*heroe).ok_or_else(|| rechazo(rechazos::HEROE_SIN_COLUMNA, "los víveres son los de la columna: el héroe no va en ninguna".into()))?;
                if *cantidad <= 0 {
                    return Err(rechazo(rechazos::CANTIDAD_NO_POSITIVA, "la cantidad debe ser mayor que cero".into()));
                }
                let e = self.estado.ejercitos.get_mut(&columna).ok_or_else(|| rechazo(rechazos::EJERCITO_DESCONOCIDO, ids::ejercito(columna)))?;
                if e.columna.lider != *heroe {
                    return Err(rechazo(rechazos::COLUMNA_SOLO_EL_LIDER, "solo el Líder pasa trigo del carro a los víveres".into()));
                }
                let movido = racion::pasar_a_viveres(&mut e.columna.carga, &mut e.columna.viveres, t, *cantidad);
                if movido == 0 {
                    return Err(sin_cambios());
                }
                self.reprogramar_columna(columna, t);
                self.evento("heroe.viveres", t, columna, None, vec![("heroe", *heroe as i64), ("cantidad", movido)]);
                Ok(vec![])
            }
            Comando::AsignarEscolta { caravana, heroe, escuadras } => self.asignar_escolta(t, p, *caravana, *heroe, escuadras),
            Comando::QuitarEscolta { caravana, heroe, escuadras } => self.quitar_escolta(t, p, *caravana, *heroe, escuadras.as_deref()),
            Comando::Desconectarse { heroe } => {
                self.autorizar_heroe(p, *heroe, false)?;
                let Some(sale_en) = self.heroe_mut(*heroe).heroe.presencia.desconectarse(&mr, t) else { return Err(sin_cambios()) };
                let clave = self.estado.generaciones.clave(*heroe);
                self.estado.cola.programar(sale_en, PRIO_SALIDA, clave, Carga::SalidaDelMundo);
                self.emitir_propio(t, "heroe.se_desconecta", vec![], vec![("heroe", *heroe as i64), ("saleEn", sale_en)]);
                Ok(vec![])
            }
            Comando::Conectarse { heroe } => {
                self.autorizar_heroe(p, *heroe, true)?;
                match self.heroe_mut(*heroe).heroe.presencia.conectarse() {
                    Conexion::SinCambios => return Err(sin_cambios()),
                    Conexion::CancelaLaSalida => {
                        self.estado.generaciones.subir(*heroe);
                    }
                    Conexion::Vuelve => return Ok(self.volver_al_mundo(t, *heroe)),
                }
                Ok(vec![])
            }
            Comando::FijarTecnologias { faccion, tecnologias } => {
                if let Some(r) = solo_admin() {
                    return Err(r);
                }
                if let Some(t) = tecnologias.iter().find(|t| !mr.tecnologias.todas.contains(t)) {
                    return Err(rechazo(rechazos::TECNOLOGIA_DESCONOCIDA, format!("«{t}» no es una tecnología")));
                }
                let nuevas: std::collections::BTreeSet<String> = tecnologias.iter().cloned().collect();
                let actuales = self.estado.militar.tecnologias.get(faccion).cloned().unwrap_or_default();
                if nuevas == actuales {
                    return Err(sin_cambios());
                }
                if nuevas.is_empty() {
                    self.estado.militar.tecnologias.remove(faccion);
                } else {
                    self.estado.militar.tecnologias.insert(*faccion, nuevas);
                }
                // Lo que un campamento ofrece depende de las tecnologías de las Facciones vivas.
                self.tocar_todos_los_campamentos();
                self.emitir_propio(t, "faccion.tecnologias", vec![], vec![("faccion", *faccion as i64), ("tecnologias", tecnologias.len() as i64)]);
                Ok(vec![])
            }
            Comando::FijarFaccionDeHeroe { heroe, faccion } => {
                if let Some(r) = solo_admin() {
                    return Err(r);
                }
                self.autorizar_heroe(p, *heroe, true)?;
                let h = self.heroe_mut(*heroe);
                if h.faccion == *faccion {
                    return Err(sin_cambios());
                }
                let anterior = std::mem::replace(&mut h.faccion, *faccion);
                self.tr.tocar_todos(Clase::Faccion, anterior.map(u64::from));
                Ok(vec![])
            }
            Comando::ColocarHeroe { heroe, ubicacion } => {
                if let Some(r) = solo_admin() {
                    return Err(r);
                }
                self.autorizar_heroe(p, *heroe, false)?;
                match ubicacion {
                    Ubicacion::Asentamiento(a) => {
                        self.plaza_o_rechazo(*a as u64)?;
                    }
                    Ubicacion::Columna(c) => {
                        if !self.estado.ejercitos.contains_key(c) {
                            return Err(rechazo(rechazos::EJERCITO_DESCONOCIDO, ids::ejercito(*c)));
                        }
                        let hn = self.heroe(*heroe).expect("existe");
                        if hn.columna.is_some_and(|otra| otra != *c) {
                            return Err(rechazo(rechazos::HEROE_YA_EN_COLUMNA, "un héroe va como mucho en una columna".into()));
                        }
                        if hn.heroe.presencia.desconecta_en.is_some() {
                            return Err(rechazo(rechazos::HEROE_DESCONECTANDOSE, "un héroe que va a salir del mundo no sale a campo".into()));
                        }
                        if hn.columna == Some(*c) && hn.heroe.presencia.ubicacion == *ubicacion {
                            return Err(sin_cambios());
                        }
                        // Se suma como participante (con sus víveres) a una columna que no tiene por qué admitirlo; si ya la tenía, la retoma.
                        let antes = self.velocidad_de_ejercito(*c).unwrap_or(0);
                        let viveres = std::mem::take(&mut self.heroe_mut(*heroe).heroe.viveres);
                        let Estado { ejercitos, militar, .. } = &mut self.estado;
                        let e = ejercitos.get_mut(c).expect("existe");
                        let mut heroes = Self::participantes_de(e);
                        heroes.push(*heroe);
                        e.columna.admitir(&mr, *heroe, viveres, t, &|ids| militar.soldados_de(&heroes, ids));
                        self.ubicar_en_columna(*heroe, *c);
                        self.recomponer_columna(*c, t, antes);
                        return Ok(vec![]);
                    }
                    Ubicacion::Mercenarios(c) => {
                        if self.puerta_de_campamento(*c).is_none() {
                            return Err(rechazo(rechazos::CAMPAMENTO_DESCONOCIDO, ids::campamento(*c)));
                        }
                    }
                    Ubicacion::Desconectado(_) => return Err(rechazo(rechazos::HEROE_UBICACION_INVALIDA, "se sale del mundo desconectándose".into())),
                    Ubicacion::Pueblo(_) | Ubicacion::Avanzada(_) => return Err(rechazo(rechazos::HEROE_UBICACION_INVALIDA, "en un pueblo o una avanzada se entra con `entrarEnPueblo` o `entrarEnAvanzada`".into())),
                }
                let h = self.heroe_mut(*heroe);
                if h.heroe.presencia.ubicacion == *ubicacion {
                    return Err(sin_cambios());
                }
                if matches!(h.heroe.presencia.ubicacion, Ubicacion::Columna(_)) {
                    return Err(rechazo(rechazos::HEROE_EN_COLUMNA, "el héroe va en una columna: entra en un lugar con `entrarEn…` o sepárate antes".into()));
                }
                h.heroe.presencia.ubicacion = *ubicacion;
                Ok(vec![])
            }
            Comando::HerirHeroe { heroe } => {
                if let Some(r) = solo_admin() {
                    return Err(r);
                }
                self.autorizar_heroe(p, *heroe, true)?;
                if !self.heroe_mut(*heroe).heroe.herir(&mr, t) {
                    return Err(sin_cambios());
                }
                let hasta = self.heroe(*heroe).and_then(|h| h.heroe.herido_hasta).unwrap_or(t);
                self.emitir_propio(t, "heroe.herido", vec![], vec![("heroe", *heroe as i64), ("hasta", hasta)]);
                Ok(vec![])
            }
            _ => unreachable!("solo se llama con órdenes de los héroes"),
        }
    }

    /// `reclutarTropa` (Heroe_Y_Tropa §11): la cadena de condiciones de `militar::reclutamiento` contra la plaza real. El resumen de la plaza (pool
    /// por clase, «Leva Forzosa», lo que come la población) sale de su economía; los soldados del campamento, de sus residentes.
    fn reclutar_en_plaza(&mut self, t: Instante, p: &Peticion, asentamiento: u64, heroe: u64, tropa: Tropa) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let mr = self.estado.reglas.economia.militar.clone();
        let (faccion, plaza32) = (self.plaza_o_rechazo(asentamiento)?.faccion, asentamiento as u32);
        self.autorizar_heroe(p, heroe, false)?;
        let h = self.heroe(heroe).expect("existe");
        let reside = h.heroe.residencia == Some(Residencia::Asentamiento(plaza32));
        let permiso = reclutamiento::permiso(reside, h.faccion == Some(faccion), false);
        if permiso == reclutamiento::Permiso::No {
            let (c, m) = rechazo_de_reclutamiento(ErrorReclutamiento::NoPuedes);
            return Err(Respuesta::rechazo(c, m, v));
        }
        if h.heroe.presencia.ubicacion != Ubicacion::Asentamiento(plaza32) {
            return Err(Respuesta::rechazo(rechazos::HEROE_NO_ESTA_EN_LA_PLAZA, format!("el héroe no está dentro de as-{asentamiento}"), v));
        }
        let tecnologia_adoptada = self.tiene_tecnologia(faccion, &mr.tropas.tropa(tropa).tecnologia);
        let soldados = self.soldados_del_campamento(asentamiento);
        let resumen = self
            .con_exterior(asentamiento, t, |eco, x| eco.resumen_de_reclutamiento(x.reglas, x))
            .ok_or_else(|| Respuesta::rechazo(rechazos::SIN_MUNDO, "el núcleo no tiene mundo cargado", v))?;
        let Estado { militar, asentamientos, ejercitos, siguiente_id, .. } = &mut self.estado;
        let plaza = asentamientos.get_mut(&asentamiento).expect("existe");
        let (posicion, eco) = (plaza.posicion, &mut plaza.economia);
        let distancia = |c: u64| ejercitos.get(&c).map(|e| e.posicion_en(t).dist(posicion));
        let sitio = Plaza {
            id: plaza32,
            edificios: &eco.edificios,
            almacen: &mut eco.almacen,
            poblacion: &mut eco.poblacion,
            disponibles: resumen.disponibles,
            factor_costo: resumen.factor_costo,
            consumo_poblacion: resumen.consumo_poblacion,
            soldados_en_el_campamento: soldados,
        };
        let pedido = Pedido { heroe: &mut militar.heroes.get_mut(&heroe).expect("existe").heroe, tropa, permiso, reside, tecnologia_adoptada, distancia_de_columna: &distancia };
        let hecho = reclutamiento::reclutar(
            &mr,
            sitio,
            pedido,
            || {
                *siguiente_id += 1;
                *siguiente_id - 1
            },
            t,
        )
        .map_err(|e| {
            let (c, m) = rechazo_de_reclutamiento(e);
            Respuesta::rechazo(c, m, v)
        })?;
        self.tr.tocar(Clase::Asentamiento, asentamiento);
        self.emitir_propio(
            t,
            "tropas.reclutadas",
            vec![asentamiento],
            vec![("heroe", heroe as i64), ("tropa", crate::codec_militar::tropa_a_u8(tropa) as i64), ("origen", hecho.origen as i64), ("reclutados", hecho.cantidad), ("escuadra", hecho.escuadra as i64), ("nueva", hecho.nueva as i64)],
        );
        Ok(if hecho.nueva { vec![hecho.escuadra] } else { vec![] })
    }

    // ───────────────────────────── Escolta de caravanas (Comercio_Caravanas §5.5) ─────────────────────────────

    /// `asignarEscolta`: el héroe reside en el origen de la caravana y cede escuadras suyas del campamento, fuera de la guarnición y con soldados;
    /// la caravana, parada en su origen, las admite dentro de su cupo de Liderazgo (que no gasta el del héroe).
    fn asignar_escolta(&mut self, t: Instante, p: &Peticion, caravana: u64, heroe: u64, escuadras: &[u64]) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let de_caravana = |e| {
            let (c, m) = rechazo_de_caravana(e);
            Respuesta::rechazo(c, m, v)
        };
        let ci = self.estado.comercio.posicion_caravana(caravana).ok_or_else(|| Respuesta::rechazo(rechazos::CARAVANA_DESCONOCIDA, ids::caravana(caravana), v))?;
        self.autorizar_heroe(p, heroe, false)?;
        let origen = self.estado.comercio.flota[ci].origen as u64;
        let cupo = self.cupos_de(self.plaza_o_rechazo(origen)?).escolta;
        let mr = self.estado.reglas.economia.militar.clone();
        let h = &self.heroe(heroe).expect("existe").heroe;
        let candidatas: Vec<CandidataEscolta> = escuadras
            .iter()
            .map(|&id| match h.escuadra(id) {
                Some(e) => CandidataEscolta {
                    escuadra: EscuadraCedida { id, heroe, liderazgo: liderazgo::coste(&mr, e.tropa) },
                    es_del_heroe: true,
                    en_campamento: e.contenedor == Contenedor::Campamento,
                    en_guarnicion: e.en_guarnicion,
                    aniquilada: e.cantidad <= 0,
                },
                None => CandidataEscolta { escuadra: EscuadraCedida { id, heroe, liderazgo: 0 }, es_del_heroe: false, en_campamento: false, en_guarnicion: false, aniquilada: false },
            })
            .collect();
        let reside = h.residencia == Some(Residencia::Asentamiento(origen as u32));
        let nuevas = comercio::caravana::seleccionar_escolta(reside, &candidatas).map_err(de_caravana)?;
        self.estado.comercio.flota[ci].asignar_escolta(nuevas.clone(), cupo).map_err(de_caravana)?;
        for e in &nuevas {
            self.heroe_mut(heroe).heroe.escuadra_mut(e.id).expect("es suya").contenedor = Contenedor::Escolta(caravana);
        }
        self.emitir_comercio(t, EventoComercio::CaravanaEscoltaCedida { caravana }, vec![origen], &[("heroe", heroe as i64), ("escuadras", nuevas.len() as i64)]);
        Ok(vec![])
    }

    /// `quitarEscolta`: solo con la caravana parada en su origen; cada héroe retira únicamente las suyas (todas las suyas si no se dice cuáles).
    fn quitar_escolta(&mut self, t: Instante, p: &Peticion, caravana: u64, heroe: u64, escuadras: Option<&[u64]>) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let ci = self.estado.comercio.posicion_caravana(caravana).ok_or_else(|| Respuesta::rechazo(rechazos::CARAVANA_DESCONOCIDA, ids::caravana(caravana), v))?;
        self.autorizar_heroe(p, heroe, false)?;
        let quitadas = self.estado.comercio.flota[ci].quitar_escolta(heroe, escuadras).map_err(|e| {
            let (c, m) = rechazo_de_caravana(e);
            Respuesta::rechazo(c, m, v)
        })?;
        if quitadas.is_empty() {
            return Err(Respuesta::sin_cambios(v));
        }
        self.devolver_escolta(&quitadas);
        let origen = self.estado.comercio.flota[ci].origen as u64;
        self.emitir_comercio(t, EventoComercio::CaravanaEscoltaRetirada { caravana }, vec![origen], &[("heroe", heroe as i64), ("escuadras", quitadas.len() as i64)]);
        Ok(vec![])
    }

    // ───────────────────────────── Eventos ─────────────────────────────

    /// Vence la salida del mundo de un héroe que se desconectó (Reloj_Y_Eventos §7). Si combate (su columna o él mismo) espera al fin de la batalla (la reintenta
    /// `columna_sale_de_batalla`); si la persiguen, se aplaza hasta el tope: quien huye desconectándose no gana nada.
    pub(super) fn salida_del_mundo(&mut self, id: u64, t: Instante) {
        let mr = self.estado.reglas.economia.militar.clone();
        let (combate, perseguido) = self.columna_de(id).map_or((false, false), |c| (self.estado.ejercitos.get(&c).is_some_and(|e| e.en_batalla), self.espacio.perseguidores.contains_key(&c)));
        // Quien combate dentro de una plaza asediada tampoco sale (no va en ninguna columna).
        let combate = combate || self.heroe_en_batalla(id);
        let Some(h) = self.estado.militar.heroes.get_mut(&id) else { return };
        self.tr.tocar(Clase::Heroe, id);
        match h.heroe.presencia.al_vencer_la_salida(&mr, t, combate, perseguido) {
            None | Some(Salida::EsperaAlFinDeLaBatalla) => {}
            Some(Salida::AplazadaHasta(hasta)) => {
                let clave = self.estado.generaciones.clave(id);
                self.estado.cola.programar(hasta, PRIO_SALIDA, clave, Carga::SalidaDelMundo);
            }
            Some(Salida::Sale) => self.sale_del_mundo(id, t),
        }
    }

    // ───────────────────────────── La ración de la guarnición ─────────────────────────────

    /// La guarnición de la plaza come en el paso de su minuto, justo después de la población (Reloj_Y_Eventos §2: la población come antes que las
    /// tropas): toma del almacén `min(trigo, ración)`; con la ración entera la moral de cada escuadra sube y, sin ella, baja; con la moral a 0
    /// desertan `ceil(5 %)` de los soldados. Una plaza sin residentes con tropa no hace nada. No programa ningún evento.
    pub(super) fn alimentar_guarnicion(&mut self, plaza: u64, t: Instante) {
        if !self.estado.militar.tiene_residentes(plaza) {
            return;
        }
        let racion = self.racion_de_guarnicion(plaza);
        if racion == 0 {
            return;
        }
        let mr = self.estado.reglas.economia.militar.clone();
        let Estado { militar, asentamientos, .. } = &mut self.estado;
        let Some(a) = asentamientos.get_mut(&plaza) else { return };
        let almacen = &mut a.economia.almacen;
        let consumido = racion.min(almacen.cantidad(Recurso::Trigo));
        almacen.quitar(Recurso::Trigo, consumido);
        // Lo que come la guarnición es parte del paso de minuto: cuenta en la tendencia «+N/min» del trigo.
        a.economia.tendencia.almacen[Recurso::Trigo.indice()] -= consumido;
        // La parte de la ración que se pudo dar (ppm): con ella entera la moral sube y con menos baja en proporción a lo que faltó.
        let suministro = consumido * UNO / racion;
        let ids: Vec<u64> = militar.residentes_de(plaza).map(|h| h.heroe.id).collect();
        let mut deserciones = Vec::new();
        for id in ids {
            self.tr.tocar(Clase::Heroe, id);
            for e in militar.heroes.get_mut(&id).expect("existe").heroe.escuadras.iter_mut().filter(|e| come_de_la_plaza(e) && e.cantidad > 0) {
                e.paso_de_hambre(&mr, t, suministro);
                let desertores = e.desertar(&mr, t);
                if desertores > 0 {
                    deserciones.push((e.plaza_origen.map_or(plaza, u64::from), id, e.id, desertores));
                }
            }
        }
        if !deserciones.is_empty() {
            self.tr.tocar(Clase::Asentamiento, plaza);
        }
        for (origen, heroe, escuadra, desertores) in deserciones {
            self.emitir_propio(t, "tropas.desercion", vec![origen], vec![("heroe", heroe as i64), ("escuadra", escuadra as i64), ("desertores", desertores)]);
        }
    }

    // ───────────────────────────── Ruina de la plaza ─────────────────────────────

    /// Una plaza cae en ruinas (Diplomacia_Facciones §4.2): sus residentes pasan a la plaza de su Facción más cercana a la ruina si queda alguna y, si
    /// no, al campamento de mercenarios más cercano; sueltan la guarnición. Quien estaba dentro, residente o de visita, **sale al mapa con todo lo suyo** (Entrar con
    /// todo, 2026-10-22): con la columna con la que entró o, si no trajo ninguna, con una columna nueva con su carro y sus víveres; nadie pierde nada.
    pub(super) fn reubicar_residentes(&mut self, plaza: u64, posicion: Punto, t: Instante) {
        let dentro: Vec<u64> = self.heroes().filter(|h| h.heroe.residencia == Some(Residencia::Asentamiento(plaza as u32)) || h.heroe.presencia.ubicacion == Ubicacion::Asentamiento(plaza as u32)).map(|h| h.heroe.id).collect();
        if dentro.is_empty() {
            return;
        }
        let campamento = self.geografia.as_ref().and_then(|g| g.mapa.campamentos.iter().enumerate().min_by_key(|(_, c)| c.dist2(posicion)).map(|(i, _)| i as u32));
        for id in dentro {
            let h = self.heroe(id).expect("existe");
            let (residia, faccion) = (h.heroe.residencia == Some(Residencia::Asentamiento(plaza as u32)), h.faccion);
            let hermana = faccion.and_then(|f| self.plazas_de(f).min_by_key(|a| (a.posicion.dist2(posicion), a.id)).map(|a| a.id as u32));
            let nueva = hermana.map(Residencia::Asentamiento).or(campamento.map(Residencia::Campamento));
            if residia {
                self.mudar_de_residencia(id, nueva);
            }
            let estaba_dentro = self.heroe(id).expect("existe").heroe.presencia.ubicacion == Ubicacion::Asentamiento(plaza as u32);
            if estaba_dentro {
                // Las ruinas no se habitan: sale al mapa con su columna (la plaza ya no existe, pero la columna guarda dónde estaba su puerta) o con una nueva.
                match self.heroe(id).expect("existe").columna {
                    Some(c) => {
                        self.ubicar_en_columna(id, c);
                        self.tr.tocar(Clase::Ejercito, c);
                    }
                    None => {
                        self.columna_de_salida(t, id, posicion, residia.then_some(faccion).flatten(), &[]);
                    }
                }
            }
            self.emitir_propio(t, "heroe.reubicado", vec![], vec![("heroe", id as i64), ("ruina", plaza as i64), ("alMapa", estaba_dentro as i64)]);
        }
    }
}
