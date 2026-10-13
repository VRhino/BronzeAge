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
use crate::estado::{CambioVersion, Estado, EventoNucleo};
use crate::nucleo::Nucleo;
use crate::proyeccion::{ambito_heroe, cambio_ejercito, clave_ambito_heroe, entidad_ejercito, evento_dominio, parse_id_ejercito};
use contrato::v1 as pb;
use pb::orden::Cuerpo;
use servidor::{ComandoSesion, Difusion, RelojCompartido, SesionId};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

const ESPERA_MAXIMA_MS: i64 = 500;

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

/// Agrupa por dueño (`solo` = uno solo) las columnas tocadas y los eventos de los registros.
fn agrupar<'e, 'r>(estado: &'e Estado, registros: impl Iterator<Item = &'r CambioVersion>, solo: Option<&str>) -> BTreeMap<&'e str, Cubo<'r>> {
    let mut por_dueno: BTreeMap<&'e str, Cubo<'r>> = BTreeMap::new();
    let dueno = |id: &u64| estado.ejercitos.get(id).map(|e| e.propietario.as_str()).filter(|d| solo.is_none_or(|s| s == *d));
    for cv in registros {
        for id in &cv.ejercitos {
            if let Some(d) = dueno(id) {
                por_dueno.entry(d).or_default().0.insert(*id);
            }
        }
        for ev in &cv.eventos {
            let duenos: BTreeSet<&str> = ev.ejercitos.iter().filter_map(dueno).collect();
            for d in duenos {
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
            ComandoSesion::Batalla { .. } => {}
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
            let entidades = self.nucleo.estado().ejercitos.values().filter(|e| e.propietario == cliente).map(|e| entidad_ejercito(e, self.nucleo.ahora())).collect();
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
                let heroes = ev.ejercitos.iter().filter_map(|id| estado.ejercitos.get(id)).map(|e| e.lider.clone()).collect();
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
        if !depositar(&tx, Difusion::Resultado(resultado_orden(&id, &respuesta))) {
            self.quitar(sesion);
        }
        if cambio_de_reloj {
            self.avisar_reloj();
        }
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
                match self.nucleo.estado().ejercito_de_lider(&m.heroe_id) {
                    Some(ejercito) => Comando::MarcharA { ejercito, destino },
                    None => return no(rechazos::HEROE_DESCONOCIDO, &m.heroe_id),
                }
            }
            Some(Cuerpo::EstacionarEjercito(e)) => match parse_id_ejercito(&e.ejercito_id) {
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

pub fn resultado_orden(peticion_id: &str, r: &Respuesta) -> pb::ResultadoOrden {
    let (resultado, rechazo) = match &r.resultado {
        Resultado::Aceptada => (pb::Resultado::OrdenAceptada, None),
        Resultado::SinCambios => (pb::Resultado::OrdenSinCambios, None),
        Resultado::Duplicada => (pb::Resultado::OrdenDuplicada, None),
        Resultado::Rechazada { codigo, mensaje } => {
            (pb::Resultado::OrdenRechazada, Some(pb::RechazoOrden { codigo: (*codigo).into(), mensaje: mensaje.clone(), detalle: None }))
        }
    };
    pb::ResultadoOrden {
        peticion_id: peticion_id.into(),
        resultado: resultado as i32,
        version_tras: r.version_tras,
        rechazo,
        creadas: r.creadas.iter().map(|&id| pb::ReferenciaCreada { entidad: pb::TipoEntidad::Ejercito as i32, id: crate::proyeccion::id_ejercito(id) }).collect(),
        datos: None,
    }
}
