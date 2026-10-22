//! El actor de sesiones: consume los `ComandoSesion` de `servidor`, aplica las órdenes en el núcleo y reparte `Foto`, `Delta`
//! y `ResultadoOrden` por el canal de cada sesión, cumpliendo las obligaciones documentadas en `servidor`:
//! - **`try_send` siempre**: si el canal de una sesión está lleno (cliente lento) o cerrado, la sesión se descarta (se suelta
//!   su `Sender`, y el servidor la cierra con `interno`). Nunca se bloquea el mundo por un cliente.
//! - **Causalidad**: antes del `ResultadoOrden` de una orden se reparten TODAS las `Delta` con `hasta <= version_tras`.
//! - **Conecta**: se contesta el `oneshot` con la `Bienvenida` y a continuación se deposita la `Foto` o la `Delta` de reanudación.
//!
//! # Ámbitos
//! Cada sesión recibe el ámbito `HEROE:<cliente>` con sus columnas. `Delta.desde` es la última versión que esa sesión tiene
//! del ámbito: las versiones que no tocan nada suyo no le cuestan ningún mensaje (cada sesión lleva su propia versión del
//! ámbito, que avanza al recibir una `Delta`). El ámbito `VISTA` (lo ajeno, con niebla) lo servirá `vision`; mientras tanto
//! `CambiarVista` solo se recuerda.
//!
//! # Reloj
//! El actor es quien gobierna el [`RelojCompartido`]: duerme hasta el próximo evento de la cola (convertido a pared con el
//! factor vigente) o, como mucho, 500 ms, y al despertar avanza el núcleo hasta el instante de mundo.

use crate::comando::{rechazos, AccionReloj, Comando, Peticion, Resultado, Respuesta};
use crate::estado::{CambioVersion, Clase, Estado, EventoNucleo};
use crate::nucleo::Nucleo;
use crate::proyeccion::ids::{self, parse_ejercito, parse_escuadra, parse_heroe};
use crate::proyeccion::{ambito_heroe, cambio_ejercito, clave_ambito_heroe, detalle_de_rechazo, entidad_ejercito, evento_dominio, referencia_creada};
use combate::ciclo::{Asignacion, EscuadraDeSalida, Instancia, MotivoSalida, SalidaBatalla, Token};
use combate::resolucion::Lado;
use combate::resultado::{Botin, ObjetoBotin, Razon, Resultado as ResultadoDeBatalla, ResultadoEscuadra, ResultadoHeroe};
use militar::heroe::Monedas;
use contrato::v1 as pb;
use pb::orden::Cuerpo;
use sha2::{Digest, Sha256};
use servidor::{ComandoSesion, Difusion, RelojCompartido, SesionId};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

const ESPERA_MAXIMA_MS: i64 = 500;
/// La versión del esquema del contrato de batalla que habla este servidor (`SCHEMA_VERSION`).
const SCHEMA_VERSION: u32 = 1;

struct Sesion {
    cliente: String,
    admin: bool,
    tx: mpsc::Sender<Difusion>,
    /// Última versión del ámbito `HEROE:<cliente>` que tiene el cliente.
    version: u64,
    #[allow(dead_code)] // ponytail: solo se recuerda; `vision` lo usará para filtrar la VISTA
    vista: Option<pb::Rectangulo>,
}

struct Actor {
    nucleo: Nucleo,
    reloj: RelojCompartido,
    sesiones: HashMap<SesionId, Sesion>,
    por_cliente: HashMap<String, SesionId>,
    /// Última versión cuyos cambios ya se repartieron.
    entregada: u64,
}

/// Arranca el actor del núcleo. Termina (devolviendo el [`Nucleo`], ya apagado) cuando se cierran todos los emisores de `rx`.
/// Conecta el reloj al núcleo: a partir de aquí el mundo avanza al ritmo del reloj.
pub fn lanzar(mut nucleo: Nucleo, reloj: RelojCompartido, rx: mpsc::Receiver<ComandoSesion>) -> JoinHandle<Nucleo> {
    nucleo.conectar_reloj(reloj.clone());
    let entregada = nucleo.version();
    let actor = Actor { nucleo, reloj, sesiones: HashMap::new(), por_cliente: HashMap::new(), entregada };
    tokio::spawn(actor.correr(rx))
}

/// Lo que cambió para un dueño: sus columnas tocadas y los eventos que nombran a alguna, con la versión que los produjo.
type Cubo<'r> = (BTreeSet<u64>, Vec<(u64, &'r EventoNucleo)>);

/// Agrupa por dueño (`solo` = uno solo) las columnas tocadas y los eventos de los registros. Una columna es de las cuentas de todos sus participantes.
fn agrupar<'e, 'r>(estado: &'e Estado, registros: impl Iterator<Item = &'r CambioVersion>, solo: Option<&str>) -> BTreeMap<&'e str, Cubo<'r>> {
    let mut por_dueno: BTreeMap<&'e str, Cubo<'r>> = BTreeMap::new();
    let duenos = |id: &u64| -> BTreeSet<&'e str> {
        let participantes = estado.ejercitos.get(id).into_iter().flat_map(|e| e.columna.participantes.iter());
        participantes.filter_map(|p| estado.militar.heroes.get(&p.heroe)).map(|h| h.cuenta.as_str()).filter(|d| solo.is_none_or(|s| s == *d)).collect()
    };
    for cv in registros {
        for id in cv.tocados.de(Clase::Ejercito) {
            for d in duenos(id) {
                por_dueno.entry(d).or_default().0.insert(*id);
            }
        }
        for ev in &cv.eventos {
            let oyen: BTreeSet<&str> = ev.ejercitos.iter().flat_map(duenos).collect();
            for d in oyen {
                por_dueno.entry(d).or_default().1.push((cv.version, ev));
            }
        }
    }
    por_dueno
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
                let admin = hola.cliente() == pb::TipoCliente::ClienteAdminWeb;
                self.conecta(sesion, identidad.cliente_id, admin, hola, difusion, respuesta);
            }
            ComandoSesion::Orden { sesion, orden } => self.orden(sesion, orden),
            ComandoSesion::Vista { sesion, rect } => {
                if let Some(s) = self.sesiones.get_mut(&sesion) {
                    s.vista = Some(rect);
                }
            }
            ComandoSesion::Batalla { sesion, mensaje } => self.batalla(sesion, mensaje),
            ComandoSesion::Desconecta { sesion } => self.quitar(sesion),
        }
    }

    fn quitar(&mut self, sesion: SesionId) {
        if let Some(s) = self.sesiones.remove(&sesion) {
            if self.por_cliente.get(&s.cliente) == Some(&sesion) {
                self.por_cliente.remove(&s.cliente);
            }
        }
    }

    // ───────────────────────────── Conexión y reanudación ─────────────────────────────

    fn conecta(
        &mut self,
        sesion: SesionId,
        cliente: String,
        admin: bool,
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
        if let Some(vieja) = self.por_cliente.remove(&cliente) {
            self.sesiones.remove(&vieja);
        }
        let conocida = hola.versiones_conocidas.get(&clave_ambito_heroe(&cliente)).copied();
        let (decision, version, msg) = self.reanudar(&cliente, conocida);
        let bienvenida = pb::Bienvenida {
            sesion_id: format!("s{sesion}"),
            heroe_id: Some(cliente.clone()),
            mundo: Some(pb::InfoMundo { hash: self.nucleo.mundo_hash().into(), ..Default::default() }),
            reloj: Some(self.reloj.reloj()),
            ambitos: vec![pb::AmbitoSuscrito {
                ambito: Some(ambito_heroe(&cliente)),
                version_actual: self.nucleo.version(),
                decision: decision as i32,
            }],
            ..Default::default()
        };
        if respuesta.send(Ok(bienvenida)).is_err() {
            return;
        }
        let vivo = msg.is_none_or(|m| depositar(&tx, m));
        if vivo {
            self.por_cliente.insert(cliente.clone(), sesion);
            self.sesiones.insert(sesion, Sesion { cliente, admin, tx, version, vista: None });
        }
    }

    /// Decide cómo reanudar el ámbito del cliente desde la versión `conocida`. Devuelve la decisión, la versión del ámbito
    /// que tendrá el cliente al aplicar el mensaje y el mensaje (foto o delta) si procede.
    fn reanudar(&self, cliente: &str, conocida: Option<u64>) -> (pb::DecisionReanudacion, u64, Option<Difusion>) {
        let v = self.nucleo.version();
        let foto = || {
            let entidades = self.nucleo.estado().ejercitos.values().filter(|e| e.columna.participantes.iter().any(|p| self.nucleo.estado().militar.heroes.get(&p.heroe).is_some_and(|h| h.cuenta == cliente))).map(|e| entidad_ejercito(e, self.nucleo.ahora())).collect();
            (pb::DecisionReanudacion::ReanudaConFoto, v, Some(Difusion::Foto(pb::Foto { ambito: Some(ambito_heroe(cliente)), version: v, entidades, niebla: None })))
        };
        let Some(k) = conocida else { return foto() };
        let Some(registros) = self.nucleo.cambios_desde(k) else { return foto() };
        match self.delta_de(cliente, k, v, registros) {
            Some(d) => (pb::DecisionReanudacion::ReanudaConDeltas, v, Some(Difusion::Delta(d))),
            None => (pb::DecisionReanudacion::ReanudaSinNovedad, k, None),
        }
    }

    /// La `Delta` consolidada `desde → hasta` con lo del cliente, o `None` si nada de lo registrado le toca.
    fn delta_de<'a>(&self, cliente: &str, desde: u64, hasta: u64, registros: impl Iterator<Item = &'a CambioVersion>) -> Option<pb::Delta> {
        let cubo = agrupar(self.nucleo.estado(), registros, Some(cliente)).remove(cliente)?;
        Some(self.armar(cliente, desde, hasta, cubo))
    }

    fn armar(&self, cliente: &str, desde: u64, hasta: u64, (ids, eventos): Cubo<'_>) -> pb::Delta {
        let estado = self.nucleo.estado();
        let t = self.nucleo.ahora();
        let cambios = ids.iter().filter_map(|id| estado.ejercitos.get(id)).map(|e| cambio_ejercito(e, t)).collect();
        let eventos = eventos
            .into_iter()
            .map(|(version, ev)| {
                let heroes = ev.ejercitos.iter().filter_map(|id| estado.ejercitos.get(id)).flat_map(|e| e.columna.participantes.iter().map(|p| ids::heroe(p.heroe))).collect();
                evento_dominio(ev, version, heroes)
            })
            .collect();
        pb::Delta { ambito: Some(ambito_heroe(cliente)), desde, hasta, cambios, eventos, niebla: None }
    }

    // ───────────────────────────── Reparto de cambios ─────────────────────────────

    /// Reparte a cada sesión lo que cambió desde la última entrega, en una sola `Delta` por sesión. Una pasada por los
    /// registros agrupa por dueño (no se recorre el registro una vez por sesión).
    fn distribuir(&mut self) {
        let hasta = self.nucleo.version();
        if hasta == self.entregada {
            return;
        }
        let mut envios: Vec<(SesionId, Difusion)> = Vec::new();
        match self.nucleo.cambios_desde(self.entregada) {
            Some(registros) => {
                for (cliente, cubo) in agrupar(self.nucleo.estado(), registros, None) {
                    let Some(&id) = self.por_cliente.get(cliente) else { continue };
                    envios.push((id, Difusion::Delta(self.armar(cliente, self.sesiones[&id].version, hasta, cubo))));
                }
            }
            None => {
                // Se perdió registro (no debería ocurrir en vivo): foto completa a todos.
                for (&id, s) in &self.sesiones {
                    envios.push((id, self.reanudar(&s.cliente, None).2.expect("la foto siempre existe")));
                }
            }
        }
        self.entregada = hasta;
        for (id, d) in envios {
            let hasta_envio = match &d {
                Difusion::Delta(x) => x.hasta,
                Difusion::Foto(x) => x.version,
                _ => hasta,
            };
            let Some(s) = self.sesiones.get_mut(&id) else { continue };
            s.version = hasta_envio;
            if !depositar(&s.tx, d) {
                self.quitar(id);
            }
        }
    }

    // ───────────────────────────── Órdenes ─────────────────────────────

    fn orden(&mut self, sesion: SesionId, orden: pb::Orden) {
        let Some(s) = self.sesiones.get(&sesion) else { return };
        let (cliente, admin, tx) = (s.cliente.clone(), s.admin, s.tx.clone());
        let id = orden.peticion_id.clone();
        let respuesta = match self.traducir(&orden, &cliente, admin) {
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
            self.quitar(sesion);
        }
        if cambio_de_reloj {
            self.avisar_reloj();
        }
    }

    /// Un mensaje del servidor de batalla (Batalla_Unity_Ciclo §8): se aplica como una orden del núcleo con el actor `batalla-servidor:<id>` (el id de su cliente)
    /// y su respuesta, si es un rechazo, vuelve a esa sesión como un `Aviso` con el código estable (el contrato 05 todavía no tiene un acuse por mensaje: es de la
    /// fase 4H). Repetir un mensaje ya aplicado no cambia nada ni avisa de nada.
    fn batalla(&mut self, sesion: SesionId, mensaje: pb::MensajeDeServidorDeBatalla) {
        let Some(s) = self.sesiones.get(&sesion) else { return };
        let (servidor, tx) = (s.cliente.clone(), s.tx.clone());
        let respuesta = match self.traducir_batalla(mensaje) {
            Ok((peticion_id, comando)) => {
                let r = self.nucleo.aplicar(Peticion::de_servidor_de_batalla(&servidor, &peticion_id, comando));
                self.distribuir();
                r
            }
            Err(r) => r,
        };
        if let Resultado::Rechazada { codigo, mensaje, .. } = respuesta.resultado {
            let aviso = pb::Aviso { codigo: codigo.into(), mensaje, reloj: None };
            if !depositar(&tx, Difusion::Aviso(aviso)) {
                self.quitar(sesion);
            }
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
        let lentas: Vec<SesionId> = self.sesiones.iter().filter(|(_, s)| !depositar(&s.tx, Difusion::Aviso(aviso.clone()))).map(|(&id, _)| id).collect();
        for id in lentas {
            self.quitar(id);
        }
    }

    /// De la orden del contrato a la petición del núcleo. `Err` = rechazo sin pasar por el núcleo (no deja rastro).
    fn traducir(&self, orden: &pb::Orden, cliente: &str, admin: bool) -> Result<Peticion, Respuesta> {
        let v = self.nucleo.version();
        let no = |codigo, m: &str| Err(Respuesta::rechazo(codigo, m, v));
        let comando = match &orden.cuerpo {
            Some(Cuerpo::MarcharA(m)) => {
                use pb::orden_marchar_a::objetivo::Tipo;
                let destino = match m.objetivo.as_ref().and_then(|o| o.tipo.as_ref()) {
                    Some(Tipo::Punto(p)) => base::Punto::new(p.x, p.y),
                    _ => return no(rechazos::OBJETIVO_NO_SOPORTADO, "el núcleo genérico solo marcha a puntos"),
                };
                match parse_heroe(&m.heroe_id).and_then(|h| self.nucleo.estado().ejercito_de_heroe(h)) {
                    Some(ejercito) => Comando::MarcharA { ejercito, destino },
                    None => return no(rechazos::HEROE_DESCONOCIDO, &m.heroe_id),
                }
            }
            Some(Cuerpo::EstacionarEjercito(e)) => match parse_ejercito(&e.ejercito_id) {
                Some(ejercito) => Comando::Estacionar { ejercito },
                None => return no(rechazos::EJERCITO_DESCONOCIDO, &e.ejercito_id),
            },
            Some(Cuerpo::ControlarReloj(c)) => Comando::Reloj(match (c.accion.as_str(), c.factor_milesimas, c.hasta_ms) {
                ("pausar", ..) => AccionReloj::Pausar,
                ("reanudar", ..) => AccionReloj::Reanudar,
                ("fijar_factor", Some(f), _) => match u32::try_from(f) {
                    Ok(f) => AccionReloj::FijarFactor(f),
                    Err(_) => return no(rechazos::RELOJ_INVALIDO, "factor fuera de rango"),
                },
                ("avanzar_hasta", _, Some(h)) => AccionReloj::AvanzarHasta(h),
                _ => return no(rechazos::RELOJ_INVALIDO, "acción de reloj desconocida o incompleta"),
            }),
            Some(_) => return no(rechazos::ORDEN_NO_SOPORTADA, "el núcleo genérico aún no implementa esta orden (fase 6)"),
            None => return no(rechazos::ORDEN_NO_SOPORTADA, "orden vacía"),
        };
        Ok(Peticion { actor: cliente.into(), admin, peticion_id: orden.peticion_id.clone(), version_esperada: orden.version_esperada, comando })
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
