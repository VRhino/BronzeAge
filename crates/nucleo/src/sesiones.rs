//! El actor de sesiones: consume los `ComandoSesion` de `servidor`, aplica las órdenes en el núcleo y reparte `Foto`, `Delta`, `Revocado`
//! y `ResultadoOrden` por el canal de cada sesión, cumpliendo las obligaciones documentadas en `servidor`:
//! - **`try_send` siempre**: si el canal de una sesión está lleno (cliente lento) o cerrado, la sesión se descarta (se suelta
//!   su `Sender`, y el servidor la cierra con `interno`). Nunca se bloquea el mundo por un cliente.
//! - **Causalidad**: antes del `ResultadoOrden` de una orden se reparten TODAS las `Delta` con `hasta <= version_tras`.
//! - **Conecta**: se contesta el `oneshot` con la `Bienvenida` y a continuación se depositan la `Foto` o la `Delta` de reanudación de cada ámbito.
//!
//! # Ámbitos
//! Cada sesión recibe los ámbitos del contrato 05 §3 a los que tiene derecho (`ambitos`): `HEROE:<héroe>`, `FACCION:<facción>`, `BATALLA:<uuid>` de cada batalla en que combate, `ASENTAMIENTO:<plaza>` y `CAMPAMENTO:<campamento>` mientras está
//! dentro, `VISTA:<facción>` desde que fija su rectángulo con `CambiarVista`, y `ADMIN:` en una sesión de administración. El servidor decide: cuando el estado cambia el derecho (el héroe sale de la
//! plaza, cambia de Facción...) manda `Revocado` del ámbito que pierde y la `Foto` del que gana. `Delta.desde` es la última versión que esa sesión tiene del ámbito: las versiones que no tocan nada
//! suyo no le cuestan ningún mensaje (cada sesión lleva su propia versión de cada ámbito, que avanza al recibir una `Delta` o una `Foto`). Al reconectar, `Hola.versiones_conocidas` decide por
//! ámbito: sin novedad, deltas si el registro de cambios alcanza esa versión, foto si no.
//! Lo que viaja a `HEROE` y `FACCION` lo dice el registro de cambios (`CambioVersion.pertenencias`, `proyeccion::audiencia`): a qué ámbitos pertenece cada entidad tocada y de cuáles salió (`QUITA`).
//! A `ADMIN` viaja todo lo que toca cada versión, sin redactar y sin tokens de batalla (T3e).
//!
//! # Órdenes
//! `ordenes::traducir` lleva cada orden del jugador del contrato (`orders.proto`, 145 órdenes) a su `Comando` del núcleo; el núcleo aplica la autorización. Las que el núcleo aún no
//! tiene se rechazan con `orden.no_disponible`; `OrdenDeAdministracion` solo vale en una sesión de administración.
//!
//! # Reloj
//! El actor es quien gobierna el [`RelojCompartido`]: duerme hasta el próximo evento de la cola (convertido a pared con el
//! factor vigente) o, como mucho, 500 ms, y al despertar avanza el núcleo hasta el instante de mundo. Con alguna `VISTA` abierta despierta
//! además en cada muestreo de visión (cada [`MUESTREO_VISTA_MS`] de mundo) para poner al día lo que se ve: no cambia el estado del núcleo ni su versión.

mod ambitos;
mod ordenes;

use crate::comando::{rechazos, Comando, Peticion, Resultado, Respuesta};
use crate::estado::CambioVersion;
use crate::nucleo::Nucleo;
use crate::proyeccion::ambito::Ambito;
use crate::proyeccion::batalla::{incorporacion, ticket, SCHEMA_VERSION};
use crate::proyeccion::ids::{self, parse_escuadra, parse_heroe};
use crate::proyeccion::{detalle_de_rechazo, referencia_creada};
use ambitos::{cambia_derechos, delta, derechos, es_jugador, foto, plazas_de, reanudable, Entrega, Mesa, Reanuda, Sesion};
use base::Punto;
use combate::ciclo::{Asignacion, Batalla, EscuadraDeSalida, EstadoBatalla, Instancia, MotivoSalida, SalidaBatalla, Token};
use combate::resolucion::Lado;
use combate::resultado::{Botin, ObjetoBotin, Razon, Resultado as ResultadoDeBatalla, ResultadoEscuadra, ResultadoHeroe};
use militar::heroe::Monedas;
use contrato::v1 as pb;
use pb::orden::Cuerpo;
use sha2::{Digest, Sha256};
use servidor::{ComandoSesion, Difusion, EstadoNucleo, PeticionAdmin, RelojCompartido, SesionId};
use std::collections::BTreeMap;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

const ESPERA_MAXIMA_MS: i64 = 500;

/// Cada cuánto (ms de mundo) se pone al día lo que ve cada `VISTA` abierta (Niebla_De_Guerra §4.1; decisión 6 del autor: este muestreo renueva el tramo ajeno).
pub const MUESTREO_VISTA_MS: i64 = 250;

/// Lo que se mandó a los servidores de batalla de una batalla: la revisión de su ticket y la última incorporación (su secuencia).
struct Publicada {
    revision: u32,
    incorporacion: u32,
}

struct Actor {
    nucleo: Nucleo,
    reloj: RelojCompartido,
    mesa: Mesa,
    /// Lo que ya se mandó a los servidores de batalla de cada batalla activa.
    publicadas: BTreeMap<u64, Publicada>,
    /// Última versión cuyos cambios ya se repartieron.
    entregada: u64,
    /// El instante de mundo del próximo muestreo de las vistas.
    proxima_vista: base::Instante,
}

/// El primer instante de muestreo de las vistas después de `t`.
fn siguiente_muestreo(t: base::Instante) -> base::Instante {
    (t.div_euclid(MUESTREO_VISTA_MS) + 1) * MUESTREO_VISTA_MS
}

/// Arranca el actor del núcleo. Termina (devolviendo el [`Nucleo`], ya apagado) cuando se cierran todos los emisores de `rx`.
/// Conecta el reloj al núcleo: a partir de aquí el mundo avanza al ritmo del reloj.
pub fn lanzar(mut nucleo: Nucleo, reloj: RelojCompartido, rx: mpsc::Receiver<ComandoSesion>) -> JoinHandle<Nucleo> {
    nucleo.conectar_reloj(reloj.clone());
    nucleo.seguir_pertenencias();
    let (entregada, proxima_vista) = (nucleo.version(), siguiente_muestreo(nucleo.ahora()));
    let actor = Actor { nucleo, reloj, mesa: Mesa::default(), publicadas: BTreeMap::new(), entregada, proxima_vista };
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

    /// Milisegundos de pared hasta el próximo evento de la cola o, con alguna vista abierta, hasta el próximo muestreo de las vistas (tope 500; al menos 1 si ya vencía).
    fn espera_ms(&self) -> i64 {
        let muestreo = self.mesa.hay_vistas().then_some(self.proxima_vista);
        let Some(t) = self.nucleo.proximo_instante().into_iter().chain(muestreo).min() else { return ESPERA_MAXIMA_MS };
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
        self.distribuir(None);
    }

    fn manejar(&mut self, c: ComandoSesion) {
        match c {
            ComandoSesion::Conecta { sesion, identidad, hola, difusion, respuesta } => {
                let rol = hola.cliente();
                let moderador = identidad.rol == servidor::RolTecnico::Moderador;
                self.conecta(sesion, identidad.cliente_id, rol, moderador, hola, difusion, respuesta);
            }
            ComandoSesion::Orden { sesion, orden } => self.orden(sesion, orden),
            ComandoSesion::Vista { sesion, rect } => self.vista(sesion, rect),
            ComandoSesion::Batalla { sesion, mensaje } => self.batalla(sesion, mensaje),
            ComandoSesion::Cronica { sesion, pedir } => {
                // La página la lee el hilo de la persistencia, no el actor (Sistema 6 §5.3); la respuesta va directa al canal de la sesión.
                let Some(tx) = self.mesa.get(sesion).map(|s| s.tx.clone()) else { return };
                self.nucleo.pedir_cronica(pedir, Box::new(move |c| {
                    let _ = tx.try_send(Difusion::Cronica(c));
                }));
            }
            ComandoSesion::Desconecta { sesion } => {
                self.mesa.quitar(sesion);
            }
            ComandoSesion::Admin(PeticionAdmin::Estado { respuesta }) => {
                let _ = respuesta.send(EstadoNucleo { version: self.nucleo.version(), instante: self.nucleo.ahora() });
            }
            ComandoSesion::Admin(PeticionAdmin::Respaldo { destino, respuesta }) => {
                let r = self.nucleo.respaldar(&destino).map(|()| EstadoNucleo { version: self.nucleo.version(), instante: self.nucleo.ahora() });
                let _ = respuesta.send(r);
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
        moderador: bool,
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
        let servidor_de_batalla = (rol == pb::TipoCliente::ClienteServidorBatalla).then(|| cliente.clone());
        let mut s = Sesion::nueva(&self.nucleo, cliente, rol, tx.clone());
        s.moderador = moderador;
        // Por cada ámbito al que tiene derecho: reanuda si el cliente conserva una versión que el registro alcanza; si no, foto.
        let derechos = derechos(&self.nucleo, &s);
        let mut reanudan: BTreeMap<Ambito, u64> = BTreeMap::new();
        for &a in &derechos {
            let clave = a.clave(&|b| self.nucleo.uuid_de_batalla(b));
            let ki = hola.versiones_interiores_conocidas.get(&clave).copied().unwrap_or(0);
            let k = hola.versiones_conocidas.get(&clave).copied().filter(|&k| reanudable(&self.nucleo, a, k, ki));
            s.abiertos.insert(a, k.unwrap_or(v));
            reanudan.extend(k.map(|k| (a, k)));
        }
        let heroe = s.heroe.and_then(|h| self.nucleo.heroe(h));
        let (heroe_id, faccion_id) = (heroe.map(|h| ids::heroe(h.heroe.id)), heroe.and_then(|h| h.faccion).map(ids::faccion));
        let campamentos = if heroe.is_none() && es_jugador(rol) { self.campamentos_para_elegir() } else { Vec::new() };
        self.mesa.insertar(sesion, s);
        // El tramo que le falta a lo que reanuda, de la versión más vieja que conserva; de los hechos, solo los posteriores a la mayor que declara (los anteriores ya los oyó).
        let oido = hola.versiones_conocidas.values().copied().max().unwrap_or(0);
        let mut cubos = match reanudan.values().min() {
            Some(&k) => self.mesa.repartir(&self.nucleo, self.nucleo.cambios_desde(k).into_iter().flatten(), Some(Reanuda { sesion, oido })),
            None => BTreeMap::new(),
        };
        // La plaza cuyo interior avanzó desde la secuencia que trae el cliente (el paso de minuto no está en el registro del mundo) se le manda entera; en `ADMIN`, cada plaza con la secuencia que el
        // cliente da para la clave de su ámbito (`ASENTAMIENTO:as-N`). Una secuencia mayor que la de hoy solo puede llegar aquí en `ADMIN` (en `ASENTAMIENTO` pide foto): esa plaza también va entera.
        for &a in reanudan.keys() {
            let plazas: Vec<u64> = match a {
                Ambito::Asentamiento(p) => vec![p],
                Ambito::Admin => self.nucleo.estado().asentamientos.keys().copied().collect(),
                _ => continue,
            };
            for p in plazas {
                let ki = hola.versiones_interiores_conocidas.get(&Ambito::Asentamiento(p).clave(&|b| self.nucleo.uuid_de_batalla(b))).copied().unwrap_or(0);
                if self.nucleo.version_interior(p).is_some_and(|actual| ki != actual) {
                    cubos.entry((sesion, a)).or_default().interior_de(&self.nucleo, p);
                }
            }
        }
        let (mut ambitos, mut mensajes, mut avances) = (Vec::new(), Vec::new(), Vec::new());
        for a in derechos {
            let s = self.mesa.get(sesion).expect("recién insertada");
            let (decision, mensaje) = match (reanudan.get(&a), cubos.remove(&(sesion, a))) {
                (None, _) => {
                    let f = foto(&self.nucleo, s, a, v);
                    self.mesa.recordar_foto(&self.nucleo, sesion, a);
                    (pb::DecisionReanudacion::ReanudaConFoto, Some(Difusion::Foto(f)))
                }
                (Some(&k), cubo) => match cubo.and_then(|cubo| delta(&self.nucleo, s, a, k, v, cubo).delta) {
                    // Lo que no pasa por el registro (6G: los alijos de `HEROE`; en `ADMIN`, la tecnología con sus contadores y los Aedas itinerantes) va entero en la misma `Delta`.
                    Some(mut d) => {
                        d.cambios.extend(self.mesa.fuera_del_registro(&self.nucleo, sesion, a));
                        (pb::DecisionReanudacion::ReanudaConDeltas, Some(Difusion::Delta(d)))
                    }
                    // Lo tocado no cambió nada que el cliente vea (la ficha de la plaza sin cambios): nada que mandar. Sin novedad, el mundo no se movió: lo de fuera del registro se da por sabido.
                    None => {
                        self.mesa.recordar_foto(&self.nucleo, sesion, a);
                        (pb::DecisionReanudacion::ReanudaSinNovedad, None)
                    }
                },
            };
            if decision == pb::DecisionReanudacion::ReanudaConDeltas {
                avances.push(a);
            }
            let version_interior_actual = if let Ambito::Asentamiento(p) = a { self.nucleo.version_interior(p).unwrap_or(0) } else { 0 };
            ambitos.push(pb::AmbitoSuscrito { ambito: Some(a.a_contrato(&|b| self.nucleo.uuid_de_batalla(b))), version_actual: v, decision: decision as i32, version_interior_actual });
            mensajes.extend(mensaje);
        }
        for a in avances {
            self.mesa.avanzar(sesion, a, v, Vec::new());
        }
        // Al terminar de reanudar el cliente tiene cada plaza de sus ámbitos como está ahora (la de la `Foto`, la entera de la `Delta` o la que ya tenía, igual a la de hoy): contra ellas se calculan los
        // cambios que vienen.
        if let Some(s) = self.mesa.get(sesion) {
            let plazas = s.abiertos.keys().flat_map(|&a| plazas_de(&self.nucleo, a)).collect();
            self.mesa.get_mut(sesion).expect("existe").plazas = plazas;
        }
        // Un servidor de batalla que llega recibe las batallas que esperan servidor y las que ya tenía asignadas.
        if let Some(servidor) = servidor_de_batalla {
            mensajes.extend(self.batallas_de(&servidor).into_iter().map(Difusion::Batalla));
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

    /// Lo que ve un jugador sin héroe: de cada campamento de mercenarios, dónde está y cuántos viven en él (`Bienvenida.campamentos_para_elegir`), con cuántos lo eligieron para nacer.
    fn campamentos_para_elegir(&self) -> Vec<pb::CampamentoParaElegir> {
        let elegibles = self.nucleo.campamentos_para_elegir();
        elegibles
            .into_iter()
            .map(|(c, puerta, origen, residentes, eligieron)| pb::CampamentoParaElegir {
                campamento_id: ids::campamento(c),
                posicion: Some(pb::Punto { x: puerta.x, y: puerta.y }),
                origen,
                eligieron_como_inicial: eligieron,
                residentes: residentes as u32,
            })
            .collect()
    }

    // ───────────────────────────── El interés en el mapa ─────────────────────────────

    /// `CambiarVista`: fija el rectángulo y manda la `Foto` nueva de `VISTA` (otra vez si ya estaba abierta: el cliente descarta lo que tenía). Sin Facción, o en una sesión que no es
    /// de jugador, no hay `VISTA`: `Aviso{vista.sin_derecho}`; un rectángulo al revés o incompleto: `Aviso{vista.invalida}`.
    /// `VISTA` no viaja en la `Bienvenida` ni se reanuda (el rectángulo es de la sesión: al reconectar, el cliente la vuelve a pedir) y el cliente no puede cerrarla. Lo que se ve se reconcilia tras cada
    /// reparto y en el muestreo de [`MUESTREO_VISTA_MS`]: lo que sale del rectángulo o deja de verse, aunque nada lo toque, sale con `QUITA` en el muestreo siguiente.
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
    /// con todo lo que le toca: una pasada por los registros agrupa por ámbito (no se recorre el registro una vez por sesión). `causante` es la sesión cuya orden se acaba de aplicar.
    fn distribuir(&mut self, causante: Option<SesionId>) {
        let hasta = self.nucleo.version();
        let mut envios: Vec<(SesionId, Difusion)> = Vec::new();
        // El mundo puede no haber avanzado y el interior de una plaza sí (el paso de minuto no sube la versión): los registros que faltan son los de `entregada` a `hasta`, quizá ninguno.
        let registros = if hasta == self.entregada { Some(Vec::new()) } else { self.nucleo.cambios_desde(self.entregada).map(|r| r.collect::<Vec<&CambioVersion>>()) };
        match registros {
            Some(registros) => {
                self.mesa.reasignar_heroes(&self.nucleo, registros.iter().copied());
                if registros.iter().any(|cv| cambia_derechos(cv)) {
                    for id in self.mesa.ids() {
                        self.mesa.sincronizar(&self.nucleo, id, false, &mut envios);
                    }
                }
                let mut cubos = self.mesa.repartir(&self.nucleo, registros.iter().copied(), None);
                self.mesa.repartir_interior(&self.nucleo, &mut cubos);
                let mut avances = Vec::new();
                for ((id, a), cubo) in cubos {
                    let Some(s) = self.mesa.get(id) else { continue };
                    let k = s.abiertos[&a];
                    let Entrega { delta: d, plazas } = delta(&self.nucleo, s, a, k, hasta, cubo);
                    // Sin `Delta` el cliente sigue en la versión que tenía; la plaza que se recuerda sí avanza (su secuencia interior), para no compararla otra vez hasta el paso siguiente.
                    let version = if d.is_some() { hasta } else { k };
                    envios.extend(d.map(|d| (id, Difusion::Delta(d))));
                    avances.push((id, a, version, plazas));
                }
                for (id, a, version, plazas) in avances {
                    self.mesa.avanzar(id, a, version, plazas);
                }
                // Lo que `ADMIN` ve y no pasa por el registro (6G): los contadores de la tecnología y los Aedas itinerantes.
                self.mesa.admin_al_dia(&self.nucleo, None, &mut envios);
                // Y lo que cobra cada campamento a quien está dentro: los precios siguen al stock de las plazas y a la reputación, sin subir la versión.
                self.mesa.campamentos_al_dia(&self.nucleo, None, &mut envios);
            }
            None => {
                // Se perdió registro (no debería ocurrir en vivo): foto completa de todos los ámbitos a todos.
                for id in self.mesa.ids() {
                    self.mesa.sincronizar(&self.nucleo, id, true, &mut envios);
                }
            }
        }
        // Lo que se ve (05 §3, decisión del autor del 2026-10-24): en cada muestreo, todas las vistas; tras una orden, al momento solo la de quien la dio (la causalidad es suya) y las demás
        // en el muestreo siguiente; tras un cambio que no viene de una orden, o con el reloj parado (no hay muestreo que esperar), todas al momento.
        let ahora = self.nucleo.ahora();
        let parado = self.reloj.con(|r, _| r.pausado() || r.factor_milesimas() == 0);
        // Los alijos a la vista del `HEROE` (6G) se renuevan igual, con la columna que anda.
        if ahora >= self.proxima_vista || (hasta != self.entregada && (causante.is_none() || parado)) {
            self.mesa.reconciliar_vistas(&self.nucleo, None, &mut envios);
            self.mesa.reconciliar_alijos(&self.nucleo, None, &mut envios);
            self.proxima_vista = siguiente_muestreo(ahora);
        } else if hasta != self.entregada {
            self.mesa.reconciliar_vistas(&self.nucleo, causante, &mut envios);
            self.mesa.reconciliar_alijos(&self.nucleo, causante, &mut envios);
        }
        self.entregada = hasta;
        self.mesa.entregar(envios);
        self.publicar_batallas();
    }

    // ───────────────────────────── Órdenes ─────────────────────────────

    fn orden(&mut self, sesion: SesionId, orden: pb::Orden) {
        let Some(s) = self.mesa.get(sesion) else { return };
        let (cliente, admin, moderador, tx) = (s.cliente.clone(), s.es_admin(), s.moderador, s.tx.clone());
        let id = orden.peticion_id.clone();
        // El moderador ve la partida (el ámbito `ADMIN`) y no da órdenes (Proyecciones §2.2, 5E).
        let traducida = match moderador {
            true => Err(Respuesta::rechazo(rechazos::NO_AUTORIZADO, "el moderador ve la partida y no da órdenes", self.nucleo.version())),
            false => ordenes::traducir(&self.nucleo, &orden, &cliente, admin),
        };
        let respuesta = match traducida {
            Ok(p) => {
                let r = self.nucleo.aplicar(p);
                // Causalidad: primero TODAS las Delta (las de eventos que vencieron justo antes y las de esta orden)...
                self.distribuir(Some(sesion));
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
        let (servidor, tx) = (s.cliente.clone(), s.tx.clone());
        let (nombre, battle_id) = identificar(&mensaje);
        let respuesta = match self.traducir_batalla(mensaje) {
            Ok((peticion_id, comando)) => {
                let r = self.nucleo.aplicar(Peticion::de_servidor_de_batalla(&servidor, &peticion_id, comando));
                // Lo que el mensaje cambió (una revisión nueva del ticket, por ejemplo) sale antes del acuse.
                self.distribuir(Some(sesion));
                r
            }
            Err(r) => r,
        };
        let (resultado, rechazo) = resultado_y_rechazo(&respuesta);
        let acuse = pb::AcuseBatalla { mensaje: nombre.into(), battle_id, resultado: resultado as i32, rechazo };
        if !depositar(&tx, Difusion::Batalla(pb::MensajeParaServidorDeBatalla { cuerpo: Some(pb::mensaje_para_servidor_de_batalla::Cuerpo::Acuse(acuse)) })) {
            self.mesa.quitar(sesion);
        }
    }

    // ───────────────────────────── Tickets para los servidores de batalla ─────────────────────────────

    /// El ticket vigente de una batalla y sus incorporaciones vigentes con secuencia mayor que `desde`.
    fn mensajes_de(&self, b: &Batalla, con_ticket: bool, desde: u32) -> Vec<pb::MensajeParaServidorDeBatalla> {
        use pb::mensaje_para_servidor_de_batalla::Cuerpo;
        let mensaje = |c| pb::MensajeParaServidorDeBatalla { cuerpo: Some(c) };
        let nuevas = self.nucleo.incorporaciones_vigentes(b.id).into_iter().filter(|i| i.secuencia > desde);
        con_ticket.then(|| mensaje(Cuerpo::Ticket(ticket(&self.nucleo, b)))).into_iter().chain(nuevas.map(|i| mensaje(Cuerpo::Incorporacion(incorporacion(&self.nucleo, b, i))))).collect()
    }

    /// Lo que un servidor de batalla que acaba de conectar necesita: las batallas que esperan servidor y las que ya tenía asignadas, cada una con su ticket y sus incorporaciones.
    fn batallas_de(&self, servidor: &str) -> Vec<pb::MensajeParaServidorDeBatalla> {
        let t = self.nucleo.ahora();
        let suyas = |b: &&Batalla| b.pendiente_de_asignar(t) || (b.activa(t) && b.asignacion.as_ref().is_some_and(|a| a.servidor == servidor));
        self.nucleo.batallas().filter(suyas).flat_map(|b| self.mensajes_de(b, true, 0)).collect()
    }

    /// Publica a los servidores de batalla conectados lo que cambió en las batallas activas (decisión 7 del autor: el ticket va a todos y gana la primera asignación). El ticket sale cuando la
    /// batalla queda pendiente de asignar con una revisión que aún no se mandó (al abrirse, al acabar la convocatoria de un asedio y al reabrirse tras una salida previa al inicio); las
    /// incorporaciones, a todos mientras nadie la tiene y solo al servidor de la asignación después. Sin ningún servidor que deba oírlo no se proyecta nada (4H T6): se anota como publicado y el que
    /// llegue lo recoge entero (`batallas_de`).
    fn publicar_batallas(&mut self) {
        let t = self.nucleo.ahora();
        self.publicadas.retain(|id, _| self.nucleo.batalla(*id).is_some_and(|b| b.activa(t)));
        let servidores: Vec<(SesionId, String)> = self.mesa.servidores().map(|(id, s)| (id, s.cliente.clone())).collect();
        let mut envios: Vec<(SesionId, Difusion)> = Vec::new();
        for b in self.nucleo.batallas_activas() {
            let publicada = self.publicadas.get(&b.id);
            let ticket_nuevo = b.pendiente_de_asignar(t) && publicada.is_none_or(|p| p.revision != b.ticket.revision);
            if publicada.is_none() && !ticket_nuevo {
                continue;
            }
            let desde = if ticket_nuevo { 0 } else { publicada.map_or(0, |p| p.incorporacion) };
            let hasta = self.nucleo.incorporaciones_vigentes(b.id).iter().map(|i| i.secuencia).max().unwrap_or(0).max(desde);
            // Con la batalla asignada, solo el servidor que la tiene oye sus incorporaciones.
            let dueno = b.asignacion.as_ref().filter(|_| b.estado != EstadoBatalla::Convocando).map(|a| a.servidor.as_str());
            let destinos: Vec<SesionId> = servidores.iter().filter(|(_, c)| dueno.is_none_or(|d| c == d)).map(|&(id, _)| id).collect();
            if !destinos.is_empty() {
                let mensajes = self.mensajes_de(b, ticket_nuevo, desde);
                for sesion in destinos {
                    envios.extend(mensajes.iter().map(|m| (sesion, Difusion::Batalla(m.clone()))));
                }
            }
            self.publicadas.insert(b.id, Publicada { revision: b.ticket.revision, incorporacion: hasta });
        }
        self.mesa.entregar(envios);
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

/// El resultado de una orden (o de un mensaje del servidor de batalla) en el contrato, y su rechazo si lo hubo.
fn resultado_y_rechazo(r: &Respuesta) -> (pb::Resultado, Option<pb::RechazoOrden>) {
    match &r.resultado {
        Resultado::Aceptada => (pb::Resultado::OrdenAceptada, None),
        Resultado::SinCambios => (pb::Resultado::OrdenSinCambios, None),
        Resultado::Duplicada => (pb::Resultado::OrdenDuplicada, None),
        Resultado::Rechazada { codigo, mensaje, detalle } => {
            (pb::Resultado::OrdenRechazada, Some(pb::RechazoOrden { codigo: (*codigo).into(), mensaje: mensaje.clone(), detalle: detalle.as_ref().map(detalle_de_rechazo) }))
        }
    }
}

/// Qué mensaje es y de qué batalla (para el acuse, aunque el mensaje no se pueda traducir).
fn identificar(m: &pb::MensajeDeServidorDeBatalla) -> (&'static str, String) {
    use pb::mensaje_de_servidor_de_batalla::Cuerpo;
    match &m.cuerpo {
        Some(Cuerpo::Asignacion(a)) => ("asignacion", a.battle_id.clone()),
        Some(Cuerpo::Inicio(i)) => ("inicio", i.battle_id.clone()),
        Some(Cuerpo::Tokens(t)) => ("tokens", t.battle_id.clone()),
        Some(Cuerpo::Salida(s)) => ("salida", s.battle_id.clone()),
        Some(Cuerpo::Resultado(r)) => ("resultado", r.battle_id.clone()),
        None => ("", String::new()),
    }
}

pub fn resultado_orden(peticion_id: &str, r: &Respuesta, uuid_de_batalla: &dyn Fn(u64) -> String) -> pb::ResultadoOrden {
    let (resultado, rechazo) = resultado_y_rechazo(r);
    pb::ResultadoOrden {
        peticion_id: peticion_id.into(),
        resultado: resultado as i32,
        version_tras: r.version_tras,
        rechazo,
        creadas: r.tipo_creadas.map_or_else(Vec::new, |tipo| r.creadas.iter().filter_map(|&id| referencia_creada(tipo, id, uuid_de_batalla)).collect()),
        datos: None,
    }
}
