//! El actor de sesiones: consume los `ComandoSesion` de `servidor`, aplica las órdenes en el núcleo y reparte `Foto`, `Delta`, `Revocado`
//! y `ResultadoOrden` por el canal de cada sesión, cumpliendo las obligaciones documentadas en `servidor`:
//! - **`try_send` siempre**: si el canal de una sesión está lleno (cliente lento) o cerrado, la sesión se descarta (se suelta
//!   su `Sender`, y el servidor la cierra con `interno`). Nunca se bloquea el mundo por un cliente.
//! - **Causalidad**: antes del `ResultadoOrden` de una orden se reparten TODAS las `Delta` con `hasta <= version_tras`.
//! - **Conecta**: se contesta el `oneshot` con la `Bienvenida` y a continuación se depositan la `Foto` o la `Delta` de reanudación de cada ámbito.
//!
//! # Ámbitos
//! Cada sesión recibe los ámbitos del contrato 05 §3 a los que tiene derecho (`ambitos`): `HEROE:<héroe>`, `FACCION:<facción>`, `ASENTAMIENTO:<plaza>` y `CAMPAMENTO:<campamento>` mientras está
//! dentro, `VISTA:<facción>` desde que fija su rectángulo con `CambiarVista`, y `ADMIN:` en una sesión de administración. El servidor decide: cuando el estado cambia el derecho (el héroe sale de la
//! plaza, cambia de Facción...) manda `Revocado` del ámbito que pierde y la `Foto` del que gana. `Delta.desde` es la última versión que esa sesión tiene del ámbito: las versiones que no tocan nada
//! suyo no le cuestan ningún mensaje (cada sesión lleva su propia versión de cada ámbito, que avanza al recibir una `Delta` o una `Foto`). Al reconectar, `Hola.versiones_conocidas` decide por
//! ámbito: sin novedad, deltas si el registro de cambios alcanza esa versión, foto si no.
//!
//! # Órdenes
//! `ordenes::traducir` lleva cada orden del jugador del contrato (`orders.proto`, 145 órdenes) a su `Comando` del núcleo; el núcleo aplica la autorización. Las que el núcleo aún no
//! tiene se rechazan con `orden.no_disponible`; `OrdenDeAdministracion` solo vale en una sesión de administración.
//!
//! # Reloj
//! El actor es quien gobierna el [`RelojCompartido`]: duerme hasta el próximo evento de la cola (convertido a pared con el
//! factor vigente) o, como mucho, 500 ms, y al despertar avanza el núcleo hasta el instante de mundo.

mod ambitos;
mod ordenes;

use crate::comando::{rechazos, Comando, Peticion, Resultado, Respuesta};
use crate::estado::CambioVersion;
use crate::nucleo::Nucleo;
use crate::proyeccion::ambito::Ambito;
use crate::proyeccion::ids::{self, parse_escuadra, parse_heroe};
use crate::proyeccion::{detalle_de_rechazo, referencia_creada};
use ambitos::{cambia_derechos, delta, derechos, es_jugador, foto, reanudable, Mesa, Sesion};
use base::Punto;
use combate::ciclo::{Asignacion, EscuadraDeSalida, Instancia, MotivoSalida, SalidaBatalla, Token};
use combate::resolucion::Lado;
use combate::resultado::{Botin, ObjetoBotin, Razon, Resultado as ResultadoDeBatalla, ResultadoEscuadra, ResultadoHeroe};
use militar::heroe::Monedas;
use contrato::v1 as pb;
use pb::orden::Cuerpo;
use sha2::{Digest, Sha256};
use servidor::{ComandoSesion, Difusion, RelojCompartido, SesionId};
use std::collections::BTreeMap;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

const ESPERA_MAXIMA_MS: i64 = 500;
/// La versión del esquema del contrato de batalla que habla este servidor (`SCHEMA_VERSION`).
const SCHEMA_VERSION: u32 = 1;

struct Actor {
    nucleo: Nucleo,
    reloj: RelojCompartido,
    mesa: Mesa,
    /// Última versión cuyos cambios ya se repartieron.
    entregada: u64,
}

/// Arranca el actor del núcleo. Termina (devolviendo el [`Nucleo`], ya apagado) cuando se cierran todos los emisores de `rx`.
/// Conecta el reloj al núcleo: a partir de aquí el mundo avanza al ritmo del reloj.
pub fn lanzar(mut nucleo: Nucleo, reloj: RelojCompartido, rx: mpsc::Receiver<ComandoSesion>) -> JoinHandle<Nucleo> {
    nucleo.conectar_reloj(reloj.clone());
    let entregada = nucleo.version();
    let actor = Actor { nucleo, reloj, mesa: Mesa::default(), entregada };
    tokio::spawn(actor.correr(rx))
}

fn depositar(tx: &mpsc::Sender<Difusion>, d: Difusion) -> bool {
    tx.try_send(d).is_ok()
}

impl Actor {
    async fn correr(mut self, mut rx: mpsc::Receiver<ComandoSesion>) -> Nucleo {
        loop {
            let espera = Duration::from_millis(self.espera_ms() as u64);
            tokio::select! {
                c = rx.recv() => match c {
                    Some(c) => self.manejar(c),
                    None => break,
                },
                _ = tokio::time::sleep(espera) => {}
            }
            self.avanzar();
        }
        self.avanzar();
        if let Err(e) = self.nucleo.apagar() {
            eprintln!("nucleo: error al apagar: {e}");
        }
        self.nucleo
    }

    /// Milisegundos de pared hasta el próximo evento de la cola (tope 500; al menos 1 si ya vencía).
    fn espera_ms(&self) -> i64 {
        let Some(t) = self.nucleo.proximo_instante() else { return ESPERA_MAXIMA_MS };
        let ms = self.reloj.con(|r, pared| {
            if r.pausado() || r.factor_milesimas() == 0 {
                None
            } else {
                Some(r.pared_de(t).map_or(0, |w| w - pared))
            }
        });
        ms.map_or(ESPERA_MAXIMA_MS, |m| m.clamp(1, ESPERA_MAXIMA_MS))
    }

    fn avanzar(&mut self) {
        self.nucleo.pulso();
        self.distribuir();
    }

    fn manejar(&mut self, c: ComandoSesion) {
        match c {
            ComandoSesion::Conecta { sesion, identidad, hola, difusion, respuesta } => {
                let rol = hola.cliente();
                self.conecta(sesion, identidad.cliente_id, rol, hola, difusion, respuesta);
            }
            ComandoSesion::Orden { sesion, orden } => self.orden(sesion, orden),
            ComandoSesion::Vista { sesion, rect } => self.vista(sesion, rect),
            ComandoSesion::Batalla { sesion, mensaje } => self.batalla(sesion, mensaje),
            ComandoSesion::Desconecta { sesion } => {
                self.mesa.quitar(sesion);
            }
        }
    }

    /// Un `Aviso` suelto a una sesión (un rechazo que no es de una orden).
    fn avisar(&mut self, sesion: SesionId, codigo: &str, mensaje: String) {
        let Some(s) = self.mesa.get(sesion) else { return };
        if !depositar(&s.tx, Difusion::Aviso(pb::Aviso { codigo: codigo.into(), mensaje, reloj: None })) {
            self.mesa.quitar(sesion);
        }
    }

    // ───────────────────────────── Conexión y reanudación ─────────────────────────────

    fn conecta(
        &mut self,
        sesion: SesionId,
        cliente: String,
        rol: pb::TipoCliente,
        hola: pb::Hola,
        tx: mpsc::Sender<Difusion>,
        respuesta: oneshot::Sender<Result<pb::Bienvenida, pb::ErrorSesion>>,
    ) {
        if hola.mundo_hash_conocido.as_deref().is_some_and(|h| h != self.nucleo.mundo_hash()) {
            let e = pb::ErrorSesion { codigo: "mundo_distinto".into(), mensaje: "el hash del mundo cambió".into(), fatal: true };
            let _ = respuesta.send(Err(e));
            return;
        }
        self.avanzar();
        if let Some(vieja) = self.mesa.de_cliente(&cliente) {
            self.mesa.quitar(vieja);
        }
        let v = self.nucleo.version();
        let mut s = Sesion::nueva(&self.nucleo, cliente, rol, tx.clone());
        // Por cada ámbito al que tiene derecho: reanuda si el cliente conserva una versión que el registro alcanza; si no, foto.
        let derechos = derechos(&self.nucleo, &s);
        let mut reanudan: BTreeMap<Ambito, u64> = BTreeMap::new();
        for &a in &derechos {
            let k = hola.versiones_conocidas.get(&a.clave()).copied().filter(|&k| reanudable(&self.nucleo, &s, a, k));
            s.abiertos.insert(a, k.unwrap_or(v));
            reanudan.extend(k.map(|k| (a, k)));
        }
        let heroe = s.heroe.and_then(|h| self.nucleo.heroe(h));
        let (heroe_id, faccion_id) = (heroe.map(|h| ids::heroe(h.heroe.id)), heroe.and_then(|h| h.faccion).map(ids::faccion));
        let campamentos = if heroe.is_none() && es_jugador(rol) { self.campamentos_para_elegir() } else { Vec::new() };
        self.mesa.insertar(sesion, s);
        // El tramo que le falta a lo que reanuda, de la versión más vieja que conserva.
        let mut cubos = match reanudan.values().min() {
            Some(&k) => self.mesa.repartir(&self.nucleo, self.nucleo.cambios_desde(k).into_iter().flatten(), Some(sesion)),
            None => BTreeMap::new(),
        };
        let (mut ambitos, mut mensajes, mut avances) = (Vec::new(), Vec::new(), Vec::new());
        for a in derechos {
            let s = self.mesa.get(sesion).expect("recién insertada");
            let (decision, mensaje) = match (reanudan.get(&a), cubos.remove(&(sesion, a))) {
                (None, _) => (pb::DecisionReanudacion::ReanudaConFoto, Some(Difusion::Foto(foto(&self.nucleo, s, a, v)))),
                (Some(&k), Some(cubo)) => (pb::DecisionReanudacion::ReanudaConDeltas, Some(Difusion::Delta(delta(&self.nucleo, s, a, k, v, cubo)))),
                (Some(_), None) => (pb::DecisionReanudacion::ReanudaSinNovedad, None),
            };
            if decision == pb::DecisionReanudacion::ReanudaConDeltas {
                avances.push(a);
            }
            ambitos.push(pb::AmbitoSuscrito { ambito: Some(a.a_contrato()), version_actual: v, decision: decision as i32 });
            mensajes.extend(mensaje);
        }
        for a in avances {
            self.mesa.avanzar(sesion, a, v);
        }
        let bienvenida = pb::Bienvenida {
            sesion_id: format!("s{sesion}"),
            heroe_id,
            faccion_id,
            mundo: Some(pb::InfoMundo { hash: self.nucleo.mundo_hash().into(), ..Default::default() }),
            reloj: Some(self.reloj.reloj()),
            ambitos,
            campamentos_para_elegir: campamentos,
            ..Default::default()
        };
        if respuesta.send(Ok(bienvenida)).is_err() || mensajes.into_iter().any(|m| !depositar(&tx, m)) {
            self.mesa.quitar(sesion);
        }
    }

    /// Lo que ve un jugador sin héroe: de cada campamento de mercenarios, dónde está y cuántos viven en él (`Bienvenida.campamentos_para_elegir`). DUDA: `eligieron_como_inicial` va a 0 (no se guarda aún).
    fn campamentos_para_elegir(&self) -> Vec<pb::CampamentoParaElegir> {
        let elegibles = self.nucleo.campamentos_para_elegir();
        elegibles
            .into_iter()
            .map(|(c, puerta, origen, residentes)| pb::CampamentoParaElegir {
                campamento_id: ids::campamento(c),
                posicion: Some(pb::Punto { x: puerta.x, y: puerta.y }),
                origen,
                eligieron_como_inicial: 0,
                residentes: residentes as u32,
            })
            .collect()
    }

    // ───────────────────────────── El interés en el mapa ─────────────────────────────

    /// `CambiarVista`: fija el rectángulo y manda la `Foto` nueva de `VISTA` (otra vez si ya estaba abierta: el cliente descarta lo que tenía). Sin Facción, o en una sesión que no es
    /// de jugador, no hay `VISTA`: `Aviso{vista.sin_derecho}`; un rectángulo al revés o incompleto: `Aviso{vista.invalida}`.
    /// DUDA: `VISTA` no viaja en la `Bienvenida` ni se reanuda (el rectángulo es de la sesión) y el cliente no puede cerrarla; una columna que sigue en marcha y sale del rectángulo no se retira hasta que
    /// se toque (T3d renueva el tramo de lo visible).
    fn vista(&mut self, sesion: SesionId, rect: pb::Rectangulo) {
        let esquinas = match (rect.min, rect.max) {
            (Some(a), Some(b)) if a.x <= b.x && a.y <= b.y => (Punto::new(a.x, a.y), Punto::new(b.x, b.y)),
            _ => return self.avisar(sesion, "vista.invalida", "el rectángulo necesita sus dos esquinas, la mínima antes que la máxima".into()),
        };
        let Some(s) = self.mesa.get_mut(sesion) else { return };
        s.vista = Some(esquinas);
        let abiertas: Vec<Ambito> = s.abiertos.keys().filter(|a| matches!(a, Ambito::Vista(_))).copied().collect();
        for a in abiertas {
            self.mesa.cerrar(sesion, a);
        }
        let mut envios = Vec::new();
        self.mesa.sincronizar(&self.nucleo, sesion, false, &mut envios);
        let abierta = self.mesa.get(sesion).is_some_and(|s| s.abiertos.keys().any(|a| matches!(a, Ambito::Vista(_))));
        self.mesa.entregar(envios);
        if !abierta {
            self.avisar(sesion, "vista.sin_derecho", "la vista del mundo es de la Facción: hace falta un héroe con Facción".into());
        }
    }

    // ───────────────────────────── Reparto de cambios ─────────────────────────────

    /// Reparte a cada sesión lo que cambió desde la última entrega. Primero los derechos (`Revocado` y `Foto` de los ámbitos que cambian) y después, de cada ámbito abierto, una sola `Delta`
    /// con todo lo que le toca: una pasada por los registros agrupa por ámbito (no se recorre el registro una vez por sesión).
    fn distribuir(&mut self) {
        let hasta = self.nucleo.version();
        if hasta == self.entregada {
            return;
        }
        let mut envios: Vec<(SesionId, Difusion)> = Vec::new();
        match self.nucleo.cambios_desde(self.entregada) {
            Some(registros) => {
                let registros: Vec<&CambioVersion> = registros.collect();
                self.mesa.reasignar_heroes(&self.nucleo, registros.iter().copied());
                if registros.iter().any(|cv| cambia_derechos(cv)) {
                    for id in self.mesa.ids() {
                        self.mesa.sincronizar(&self.nucleo, id, false, &mut envios);
                    }
                }
                let mut avances = Vec::new();
                for ((id, a), cubo) in self.mesa.repartir(&self.nucleo, registros.iter().copied(), None) {
                    let Some(s) = self.mesa.get(id) else { continue };
                    envios.push((id, Difusion::Delta(delta(&self.nucleo, s, a, s.abiertos[&a], hasta, cubo))));
                    avances.push((id, a));
                }
                for (id, a) in avances {
                    self.mesa.avanzar(id, a, hasta);
                }
            }
            None => {
                // Se perdió registro (no debería ocurrir en vivo): foto completa de todos los ámbitos a todos.
                for id in self.mesa.ids() {
                    self.mesa.sincronizar(&self.nucleo, id, true, &mut envios);
                }
            }
        }
        self.entregada = hasta;
        self.mesa.entregar(envios);
    }

    // ───────────────────────────── Órdenes ─────────────────────────────

    fn orden(&mut self, sesion: SesionId, orden: pb::Orden) {
        let Some(s) = self.mesa.get(sesion) else { return };
        let (cliente, admin, tx) = (s.cliente.clone(), s.es_admin(), s.tx.clone());
        let id = orden.peticion_id.clone();
        let respuesta = match ordenes::traducir(&self.nucleo, &orden, &cliente, admin) {
            Ok(p) => {
                let r = self.nucleo.aplicar(p);
                // Causalidad: primero TODAS las Delta (las de eventos que vencieron justo antes y las de esta orden)...
                self.distribuir();
                r
            }
            Err(r) => r,
        };
        let cambio_de_reloj = respuesta.aceptada() && matches!(orden.cuerpo, Some(Cuerpo::ControlarReloj(_)));
        // ...y después el resultado.
        if !depositar(&tx, Difusion::Resultado(resultado_orden(&id, &respuesta, &|b| self.nucleo.uuid_de_batalla(b)))) {
            self.mesa.quitar(sesion);
        }
        if cambio_de_reloj {
            self.avisar_reloj();
        }
    }

    /// Un mensaje del servidor de batalla (Batalla_Unity_Ciclo §8): se aplica como una orden del núcleo con el actor `batalla-servidor:<id>` (el id de su cliente)
    /// y su respuesta, si es un rechazo, vuelve a esa sesión como un `Aviso` con el código estable (el contrato 05 todavía no tiene un acuse por mensaje: es de la
    /// fase 4H). Repetir un mensaje ya aplicado no cambia nada ni avisa de nada.
    fn batalla(&mut self, sesion: SesionId, mensaje: pb::MensajeDeServidorDeBatalla) {
        let Some(s) = self.mesa.get(sesion) else { return };
        let servidor = s.cliente.clone();
        let respuesta = match self.traducir_batalla(mensaje) {
            Ok((peticion_id, comando)) => {
                let r = self.nucleo.aplicar(Peticion::de_servidor_de_batalla(&servidor, &peticion_id, comando));
                self.distribuir();
                r
            }
            Err(r) => r,
        };
        if let Resultado::Rechazada { codigo, mensaje, .. } = respuesta.resultado {
            self.avisar(sesion, codigo, mensaje);
        }
    }

    /// Del mensaje del contrato al comando del núcleo, con el id de petición con que se recuerda (un reintento idéntico responde como duplicado).
    fn traducir_batalla(&self, m: pb::MensajeDeServidorDeBatalla) -> Result<(String, Comando), Respuesta> {
        use pb::mensaje_de_servidor_de_batalla::Cuerpo;
        let v = self.nucleo.version();
        let invalido = |texto: &str| Respuesta::rechazo(rechazos::BATALLA_INVALIDA, texto, v);
        let batalla = |uuid: &str| self.nucleo.id_de_batalla(uuid).ok_or_else(|| Respuesta::rechazo(rechazos::BATALLA_NO_EXISTE, uuid, v));
        let heroe = |id: &str| parse_heroe(id).ok_or_else(|| invalido(&format!("héroe desconocido: {id}")));
        let escuadra = |id: &str| parse_escuadra(id).ok_or_else(|| invalido(&format!("escuadra desconocida: {id}")));
        let lado = |l: i32| match pb::LadoId::try_from(l) {
            Ok(pb::LadoId::Atacante) => Ok(Lado::Atacante),
            Ok(pb::LadoId::Defensor) => Ok(Lado::Defensor),
            _ => Err(invalido("lado desconocido")),
        };
        let entero = |n: u64| i64::try_from(n).map_err(|_| invalido("cifra fuera de rango"));
        let tokens = |ts: Vec<pb::TokenParticipante>| ts.into_iter().map(|t| Ok(Token { heroe: heroe(&t.heroe_id)?, token: t.token, expira_en: t.expira_en_ms })).collect::<Result<Vec<_>, Respuesta>>();
        let version_de_esquema = |n: u32| if n == SCHEMA_VERSION { Ok(()) } else { Err(invalido(&format!("schemaVersion {n}: se esperaba {SCHEMA_VERSION}"))) };
        Ok(match m.cuerpo.ok_or_else(|| invalido("mensaje de batalla vacío"))? {
            Cuerpo::Asignacion(a) => {
                version_de_esquema(a.schema_version)?;
                let instancia = a.instancia.ok_or_else(|| invalido("la asignación no trae instancia"))?;
                let instancia = Instancia { host: instancia.host, puerto: u16::try_from(instancia.puerto).map_err(|_| invalido("puerto fuera de rango"))?, protocolo: instancia.protocolo };
                let id = batalla(&a.battle_id)?;
                let peticion = format!("asignacion:{}:{}", a.battle_id, a.intento_asignacion_id);
                // El servidor lo pone el núcleo con la credencial del actor.
                let asignacion = Asignacion { intento: a.intento_asignacion_id, servidor: String::new(), revision: a.ticket_revision, instancia, tokens: tokens(a.tokens_participante)? };
                (peticion, Comando::RegistrarAsignacion { batalla: id, asignacion })
            }
            Cuerpo::Tokens(t) => {
                version_de_esquema(t.schema_version)?;
                // La petición se recuerda por su contenido: unos tokens nuevos del mismo intento no son un reintento del mensaje anterior.
                let huella = t.tokens_participante.iter().fold(Sha256::new(), |h, k| h.chain_update(k.heroe_id.as_bytes()).chain_update(k.token.as_bytes()).chain_update(k.expira_en_ms.to_le_bytes()));
                let peticion = format!("tokens:{}:{}:{}", t.battle_id, t.intento_asignacion_id, crate::hex(&huella.finalize().into()));
                (peticion, Comando::RegistrarTokens { batalla: batalla(&t.battle_id)?, intento: t.intento_asignacion_id, tokens: tokens(t.tokens_participante)? })
            }
            Cuerpo::Inicio(i) => {
                version_de_esquema(i.schema_version)?;
                (format!("inicio:{}:{}:{}", i.battle_id, i.intento_asignacion_id, i.ticket_revision), Comando::ConfirmarInicio { batalla: batalla(&i.battle_id)?, intento: i.intento_asignacion_id, revision: i.ticket_revision })
            }
            Cuerpo::Salida(s) => {
                version_de_esquema(s.schema_version)?;
                let motivo = match pb::MotivoSalida::try_from(s.motivo) {
                    Ok(pb::MotivoSalida::SalidaNoConectado) => MotivoSalida::NoConectado,
                    Ok(pb::MotivoSalida::SalidaEliminado) => MotivoSalida::Eliminado,
                    Ok(pb::MotivoSalida::SalidaAbandono) => MotivoSalida::Abandono,
                    Ok(pb::MotivoSalida::SalidaDesconexion) => MotivoSalida::Desconexion,
                    _ => return Err(invalido("motivo de salida desconocido")),
                };
                let escuadras = s.escuadras.into_iter().map(|e| Ok(EscuadraDeSalida { escuadra: escuadra(&e.squad_id)?, supervivientes: i64::from(e.supervivientes), muertos: i64::from(e.muertos) })).collect::<Result<Vec<_>, Respuesta>>()?;
                let peticion = format!("salida:{}:{}", s.battle_id, s.heroe_id);
                let salida = SalidaBatalla { intento: s.intento_asignacion_id, revision: s.ticket_revision, heroe: heroe(&s.heroe_id)?, motivo, derrotado: s.derrotado, escuadras };
                (peticion, Comando::RegistrarSalida { batalla: batalla(&s.battle_id)?, salida })
            }
            Cuerpo::Resultado(r) => {
                version_de_esquema(r.schema_version)?;
                let razon = match pb::RazonVictoria::try_from(r.razon) {
                    Ok(pb::RazonVictoria::ObjetivosCapturados) => Razon::ObjetivosCapturados,
                    Ok(pb::RazonVictoria::Aniquilacion) => Razon::Aniquilacion,
                    Ok(pb::RazonVictoria::TiempoAgotado) => Razon::TiempoAgotado,
                    _ => return Err(invalido("razón de victoria desconocida")),
                };
                let objetivos = r.objetivos.into_iter().map(|o| Ok((o.objetivo_id, o.capturado_por.map(lado).transpose()?))).collect::<Result<Vec<_>, Respuesta>>()?;
                let por_escuadra = r
                    .por_escuadra
                    .into_iter()
                    .map(|e| Ok(ResultadoEscuadra { escuadra: escuadra(&e.squad_id)?, desplegados: i64::from(e.desplegados), supervivientes_al_cierre: i64::from(e.supervivientes_al_cierre), muertos: i64::from(e.muertos), xp_ganada: e.xp_ganada }))
                    .collect::<Result<Vec<_>, Respuesta>>()?;
                let por_heroe = r
                    .por_heroe
                    .into_iter()
                    .map(|h| {
                        let botin = h
                            .botin
                            .map(|b| {
                                let m = b.monedas.unwrap_or_default();
                                let objetos = b.objetos.into_iter().map(|o| ObjetoBotin { definicion: o.item_definition_id, cantidad: i64::from(o.cantidad), instancia: o.item_instance_id }).collect();
                                Ok(Botin { objetos, monedas: Monedas { bronce: entero(m.bronce)?, plata: entero(m.plata)?, oro: entero(m.oro)? } })
                            })
                            .transpose()?;
                        Ok(ResultadoHeroe { heroe: heroe(&h.heroe_id)?, participo: h.participo, sobrevivio_al_cierre: h.sobrevivio_al_cierre, xp_ganada: h.xp_ganada, derrotado: h.derrotado, botin })
                    })
                    .collect::<Result<Vec<_>, Respuesta>>()?;
                let peticion = format!("resultado:{}:{}", r.battle_id, r.result_id);
                let resultado = ResultadoDeBatalla {
                    resultado_id: r.result_id,
                    revision: r.ticket_revision,
                    intento: r.intento_asignacion_id,
                    inicio: r.inicio_ms,
                    fin: r.fin_ms,
                    ganador: lado(r.ganador)?,
                    razon,
                    objetivos,
                    por_escuadra,
                    por_heroe,
                    version_servidor: r.version_servidor,
                    carros_perdidos: r.carros_perdidos,
                };
                (peticion, Comando::AplicarResultado { batalla: batalla(&r.battle_id)?, resultado })
            }
        })
    }

    fn avisar_reloj(&mut self) {
        let aviso = pb::Aviso { codigo: "reloj.cambiado".into(), mensaje: "el reloj de mundo cambió".into(), reloj: Some(self.reloj.reloj()) };
        let lentas: Vec<SesionId> = self.mesa.sesiones().filter(|(_, s)| !depositar(&s.tx, Difusion::Aviso(aviso.clone()))).map(|(id, _)| id).collect();
        for id in lentas {
            self.mesa.quitar(id);
        }
    }
}

pub fn resultado_orden(peticion_id: &str, r: &Respuesta, uuid_de_batalla: &dyn Fn(u64) -> String) -> pb::ResultadoOrden {
    let (resultado, rechazo) = match &r.resultado {
        Resultado::Aceptada => (pb::Resultado::OrdenAceptada, None),
        Resultado::SinCambios => (pb::Resultado::OrdenSinCambios, None),
        Resultado::Duplicada => (pb::Resultado::OrdenDuplicada, None),
        Resultado::Rechazada { codigo, mensaje, detalle } => {
            (pb::Resultado::OrdenRechazada, Some(pb::RechazoOrden { codigo: (*codigo).into(), mensaje: mensaje.clone(), detalle: detalle.as_ref().map(detalle_de_rechazo) }))
        }
    };
    pb::ResultadoOrden {
        peticion_id: peticion_id.into(),
        resultado: resultado as i32,
        version_tras: r.version_tras,
        rechazo,
        creadas: r.tipo_creadas.map_or_else(Vec::new, |tipo| r.creadas.iter().filter_map(|&id| referencia_creada(tipo, id, uuid_de_batalla)).collect()),
        datos: None,
    }
}
