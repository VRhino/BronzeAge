//! El ciclo de una batalla de Unity dentro del núcleo (Batalla_Unity_Ciclo, Reloj_Y_Eventos §6, fase 4E). **Unity decide todo lo de dentro de la partida**;
//! BronzeAge solo la abre, congela el ticket, custodia los tokens, bloquea lo que combate, valida el resultado entero y aplica las consecuencias. La máquina de
//! estados es pura y vive en `combate::{ciclo, resultado, union}`; aquí está lo que toca el mundo: sacar los bandos del estado, reservar las escuadras, apartar
//! las columnas y las caravanas, programar los plazos y volcar el resultado.
//!
//! # Cuándo se abre
//! La abre `atacar` (no hay otra orden) siempre que el proceso tiene servidores de batalla ([`Reglas::batallas_en_unity`](crate::estado::Reglas)): **toda** batalla va a Unity, también
//! bot contra bot (decisión del autor, cuarta ronda, 2026-10-22), y Unity la resuelve como pueda; sin servidores se resuelve con números, salvo el asedio, que abre la misma
//! batalla con su convocatoria de 30 s y, al acabar, se resuelve con números con todas las columnas que se unieron (M8, 2026-10-24). Se abre después de las mismas validaciones del
//! combate con números y, si procede, de declarar la guerra. Una plaza o un pueblo que ya están en una batalla no se atacan otra vez.
//!
//! # Quién defiende y quién ataca (2026-10-22, tercera ronda)
//! Al abrirse entra de oficio solo la guarnición (la de serie, la de jugadores y los mineros); **nadie más está en el ticket**, ni los residentes. Quien está **dentro** del
//! asedio, del pueblo o de la avanzada y es de la Facción dueña, aliado o vasallo puede unirse a la defensa con `UnirseABatalla`, con las escuadras que lleva encima (el loadout activo si
//! reside en su plaza, las de su columna si no), durante la convocatoria o después; quien no se une no combate. Desde **fuera**, a ≤ `radioEncuentro`, se une al ataque la Facción
//! atacante y los neutrales y enemigos del dueño, no sus amigos; que el elemento está en asedio lo dice [`Nucleo::batalla_sobre`].
//!
//! # Lo que bloquea
//! Mientras una batalla está activa (`convocando`, `asignada` o `en_curso` y dentro de plazo) sus columnas y caravanas quedan **apartadas** (no se mueven, no
//! comen, no son perseguidas ni atacadas, `apartar_columna`), sus escuadras con dueño llevan el candado `reserva_batalla`, y **ninguna orden** (salvo las de §5.1)
//! puede tocar a sus héroes, a la plaza asediada, al pueblo o campamento atacado, ni a nada que esté apartado: `Comando::implicados` dice qué toca cada orden y
//! [`Nucleo::rechazo_por_batalla`] la rechaza con `batalla.bloqueado`. La administración no lleva candado (sus órdenes son andamiaje de escenarios). **Quien está dentro del lugar disputado y no se ha unido a la defensa
//! solo puede salir** (cuarta ronda): las órdenes de salir del lugar al mapa (`Comando::es_salida_de_lugar`) no chocan con el lugar ni con estar dentro, y sí con combatir (quien se
//! unió a la defensa no sale hasta que acaba).
//!
//! # Los plazos
//! Tres citas de prioridad 1 (`Carga::Batalla`), todas de la entidad de la batalla: el fin de la convocatoria de un asedio (`FinConvocatoria`), el vencimiento del
//! estado en que está (`Vence`, a `expira_en`: pasa a `fallida` sin que nadie pierda nada) y la retirada de la cerrada (`Retira`, a `conservarCerradasMinutos`).
//! Cada cambio de estado sube la generación de la entidad y reprograma lo que siga haciendo falta, así que a una batalla le quedan en la cola, como mucho, dos.
//! Como `Vence` es de prioridad 1 y toda orden se aplica con los eventos hasta su instante ya procesados, una batalla vencida está siempre cerrada antes de que
//! nada pueda mirarla: no hace falta soltar a mano los candados de las vencidas (`sinCandadosVencidos` del TypeScript).
//!
//! # Los mensajes del servidor de batalla
//! `RegistrarAsignacion`, `ConfirmarInicio`, `RegistrarTokens`, `RegistrarSalida` y `AplicarResultado` son órdenes del núcleo con el actor `batalla-servidor:<id>`.
//! Repetir una ya aplicada no cambia nada (`SinCambios`: ni versión, ni hecho, ni id). Aplicar el resultado es una sola mutación: se valida entero (el checklist
//! de §8.5) antes de tocar nada.

use super::combates::{de_su_columna, loadout_en_el_campamento};
use super::*;
use crate::comando::Implicados;
use crate::estado::{Dato, SucesoBatalla, PRIO_BATALLA, PRIO_SALIDA, SIN_BANDERA};
use crate::estado_batallas::entidad_batalla;
use crate::estado_militar::Relacion;
use combate::botin::{repartir_botin, CarroDeColumna};
use combate::caravana::Modalidad;
use combate::ciclo::{bloqueos_de, uuid_de_batalla, Asignacion, BandoDeTicket, Batalla, Bloqueo, Bloqueos, CaravanaEnBatalla, Contexto, EdificioCongelado, Efecto, EscuadraEnBatalla, ErrorBatalla, Participante, PlazaCongelada, RecintoCongelado, SalidaBatalla, Ticket, Token};
use combate::resolucion::Lado;
use combate::resultado::{Efectos, Resultado as ResultadoDeBatalla};
use combate::union::{lado_para_unirse, Donde, ErrorUnion, Reparto};
use militar::columna::TipoColumna;
use militar::escuadra::{Contenedor, Escuadra};
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use militar::{sumar, Carga as Mercancia};
use std::collections::BTreeSet;

/// El id de una escuadra que aún no lo tiene (carreteros, bandidos): al abrir la batalla recibe uno de la partida.
const SIN_ID: u64 = 0;

/// El «héroe» que inicia una batalla que nadie ha pedido (los bandidos que acosan a una caravana): los ids de la partida empiezan en 1, así que no es de nadie y nadie la cancela.
const SIN_HEROE: u64 = 0;

/// Qué se cuenta de una batalla (Batalla_Unity_Ciclo §10). Contra un campamento de bandidos el prefijo es `evento_pve.` en lugar de `batalla.`.
#[derive(Clone, Copy)]
enum Hecho {
    Abierta,
    Refuerzos,
    Asignada,
    EnCurso,
    Salida,
    Revisada,
    Aplicada,
    Cancelada,
    Fallida,
    /// No está en la lista de §10: acaba la convocatoria de un asedio y el ticket ya puede publicarse (es la cita que el contrato necesita).
    ConvocatoriaCerrada,
}

impl Hecho {
    fn codigo(self, pve: bool) -> &'static str {
        match (self, pve) {
            (Hecho::Abierta, false) => "batalla.abierta",
            (Hecho::Abierta, true) => "evento_pve.abierta",
            (Hecho::Refuerzos, false) => "batalla.refuerzos",
            (Hecho::Refuerzos, true) => "evento_pve.refuerzos",
            (Hecho::Asignada, false) => "batalla.asignada",
            (Hecho::Asignada, true) => "evento_pve.asignada",
            (Hecho::EnCurso, false) => "batalla.en_curso",
            (Hecho::EnCurso, true) => "evento_pve.en_curso",
            (Hecho::Salida, false) => "batalla.salida",
            (Hecho::Salida, true) => "evento_pve.salida",
            (Hecho::Revisada, false) => "batalla.revisada",
            (Hecho::Revisada, true) => "evento_pve.revisada",
            (Hecho::Aplicada, false) => "batalla.aplicada",
            (Hecho::Aplicada, true) => "evento_pve.aplicada",
            (Hecho::Cancelada, false) => "batalla.cancelada",
            (Hecho::Cancelada, true) => "evento_pve.cancelada",
            (Hecho::Fallida, false) => "batalla.fallida",
            (Hecho::Fallida, true) => "evento_pve.fallida",
            (Hecho::ConvocatoriaCerrada, _) => "batalla.convocatoria_cerrada",
        }
    }
}

/// Qué es la batalla, como número de un hecho (`contexto`): 0 asedio, 1 campo abierto, 2 caravana, 3 campamento de bandidos, 4 saqueo de pueblo, 5 avanzada.
fn numero_de_contexto(c: &Contexto) -> i64 {
    match c {
        Contexto::Asedio { .. } => 0,
        Contexto::CampoAbierto { .. } => 1,
        Contexto::Caravana { .. } => 2,
        Contexto::CampamentoBandidos { .. } => 3,
        Contexto::SaqueoPueblo { .. } => 4,
        Contexto::Avanzada { .. } => 5,
    }
}

/// Contra qué se juega, como datos del hecho de apertura (con las claves que el payload traduce a ids del contrato): la plaza del asedio, el pueblo, la avanzada (y su etapa), el
/// campamento de bandidos, si son solitarios en campo abierto y la modalidad de una batalla de caravana (0 disputa, 1 aguante; las caravanas van en `caravanasIds`).
fn objetivo_de_contexto(c: &Contexto) -> Vec<(&'static str, i64)> {
    match c {
        Contexto::Asedio { asentamiento } => vec![("plaza", i64::from(*asentamiento))],
        Contexto::CampoAbierto { solitarios, .. } => vec![("solitarios", i64::from(*solitarios))],
        Contexto::Caravana { modalidad, .. } => vec![("modalidad", i64::from(*modalidad == Modalidad::Aguante))],
        Contexto::CampamentoBandidos { campamento, .. } => vec![("campamento", i64::from(*campamento))],
        Contexto::SaqueoPueblo { pueblo, .. } => vec![("pueblo", *pueblo as i64)],
        Contexto::Avanzada { avanzada, etapa, .. } => vec![("avanzada", *avanzada as i64), ("etapa", etapa.indice() as i64)],
    }
}

/// Lo que hace falta para abrir una batalla: qué se disputa, los dos bandos y lo que se queda quieto (`Apertura` del TypeScript).
pub(super) struct Apertura {
    contexto: Contexto,
    punto: Punto,
    iniciada_por: u64,
    atacante: BandoDeTicket,
    defensor: BandoDeTicket,
    bloqueo: Bloqueo,
    /// Solo un asedio: la plaza como está al abrir.
    plaza: Option<PlazaCongelada>,
}

/// Lo que ve quien pide su token (`GET /jugador/partidas/:g/batallas/:id/asignacion`, §8.7). BronzeAge nunca verifica el token: solo lo custodia y controla
/// quién lo lee.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenDeJugador {
    /// La batalla no está activa (404).
    NoExiste,
    /// El héroe no combate en ella (403).
    NoCombate,
    /// Todavía no hay asignación o token para él (404).
    SinToken,
    Token { instancia: combate::ciclo::Instancia, token: String, expira_en: i64 },
}

fn rechazo_de_batalla(e: ErrorBatalla, v: u64) -> Respuesta {
    use rechazos::*;
    let (codigo, mensaje) = match e {
        ErrorBatalla::NoActiva => (BATALLA_NO_EXISTE, "la batalla no existe o ya no está activa".to_string()),
        ErrorBatalla::YaAsignada => (BATALLA_YA_ASIGNADA, "otro servidor ya tiene esta batalla".into()),
        ErrorBatalla::EnConvocatoria => (BATALLA_INVALIDA, "un asedio en convocatoria no se publica ni se asigna".into()),
        ErrorBatalla::RevisionVieja => (BATALLA_INVALIDA, "la revisión del ticket no es la vigente".into()),
        ErrorBatalla::EstadoInvalido => (BATALLA_INVALIDA, "el estado de la batalla no admite esta operación".into()),
        ErrorBatalla::ServidorAjeno => (BATALLA_INVALIDA, "no es el servidor de la asignación activa".into()),
        ErrorBatalla::TokenInvalido(h) => (BATALLA_INVALIDA, format!("el héroe {h} no es un humano de esta batalla: no lleva token")),
        ErrorBatalla::NoCombate(h) => (BATALLA_INVALIDA, format!("el héroe {h} no combate en esta batalla")),
        ErrorBatalla::LadoLleno => (BATALLA_INVALIDA, "ese bando está lleno".into()),
        ErrorBatalla::YaSalio(h) => (BATALLA_INVALIDA, format!("el héroe {h} ya salió de esta batalla y no vuelve a entrar")),
        ErrorBatalla::YaCombate(h) => (BATALLA_INVALIDA, format!("el héroe {h} ya combate en esta batalla")),
        ErrorBatalla::NoPuedeCancelar => (BATALLA_INVALIDA, "solo quien la inició puede cancelarla, y antes de que empiece la partida".into()),
        ErrorBatalla::SalidaInvalida => (BATALLA_INVALIDA, "la salida trae una escuadra que no es del héroe, repetida o que no cierra sobre lo autorizado".into()),
        ErrorBatalla::SalidaIncompleta => (BATALLA_SALIDA_INCOMPLETA, "la salida no nombra todas las escuadras autorizadas del héroe".into()),
        ErrorBatalla::Resultado(e) => (BATALLA_INVALIDA, format!("resultado inválido: {e:?}")),
        ErrorBatalla::OtroResultado => (BATALLA_INVALIDA, "la batalla ya se aplicó con otro resultado".into()),
    };
    Respuesta::rechazo(codigo, mensaje, v)
}

fn rechazo_de_union(e: ErrorUnion, v: u64) -> Respuesta {
    let mensaje = match e {
        ErrorUnion::NadieDeFuera => "en una batalla campal entre ejércitos no entra nadie de fuera".to_string(),
        ErrorUnion::SoloColumnasPersonales => "una persecución solo admite columnas personales".into(),
        ErrorUnion::LadoObligatorio => "en una persecución hay que elegir bando".into(),
        ErrorUnion::NadieAyudaALosBandidos => "nadie ayuda a los bandidos".into(),
        ErrorUnion::SinFaccion => "hace falta Facción para unirse a esta batalla".into(),
        ErrorUnion::NoCombateAqui => "tu Facción no combate aquí".into(),
        ErrorUnion::LadoDistinto(l) => format!("a ti te toca el bando {l:?}"),
        ErrorUnion::HayQueEstarDentro => "para defender hay que estar dentro (haber entrado): a la puerta, en el mapa, solo se ataca".into(),
        ErrorUnion::HayQueEstarFuera => "para atacar hay que estar fuera, en el mapa: desde dentro de un lugar no se ataca".into(),
    };
    Respuesta::rechazo(rechazos::BATALLA_INVALIDA, mensaje, v)
}

fn sin_cambios(v: u64) -> Respuesta {
    Respuesta::sin_cambios(v)
}

/// Dónde está quien quiere unirse a una batalla respecto de lo que se disputa: en el mapa con su columna, dentro de la plaza, el pueblo o la avanzada que se disputa, o dentro de
/// cualquier otro lugar (Entrar con todo, 2026-10-22).
fn donde_esta(ubicacion: Ubicacion, contexto: &Contexto) -> Donde {
    match (ubicacion, contexto) {
        (Ubicacion::Columna(_), _) => Donde::Fuera,
        (Ubicacion::Asentamiento(a), Contexto::Asedio { asentamiento }) if a == *asentamiento => Donde::Dentro,
        (Ubicacion::Pueblo(x), Contexto::SaqueoPueblo { pueblo, .. }) if x == *pueblo => Donde::Dentro,
        (Ubicacion::Avanzada(x), Contexto::Avanzada { avanzada, .. }) if x == *avanzada => Donde::Dentro,
        _ => Donde::EnOtroLugar,
    }
}

/// Una escuadra como entra en el ticket: con su nivel, su experiencia y su moral de `t`, el instante de abrir.
fn escuadra_en_batalla(s: &Escuadra, t: Instante) -> EscuadraEnBatalla {
    EscuadraEnBatalla { id: s.id, tropa: s.tropa, dueno: Some(s.heroe), efectivos: s.cantidad, nivel: s.nivel, experiencia: s.experiencia, moral: s.moral_en(t) }
}

impl Nucleo {
    // ───────────────────────────── Consultas ─────────────────────────────

    /// Una batalla por id, abierta o cerrada reciente.
    pub fn batalla(&self, id: u64) -> Option<&Batalla> {
        self.estado.batallas.batallas.get(&id)
    }

    /// Todas las batallas que conserva el estado, por id.
    pub fn batallas(&self) -> impl Iterator<Item = &Batalla> {
        self.estado.batallas.batallas.values()
    }

    /// Las batallas activas ahora (`convocando`, `asignada` o `en_curso` y dentro de plazo).
    pub fn batallas_activas(&self) -> impl Iterator<Item = &Batalla> {
        let t = self.estado.ahora;
        self.batallas().filter(move |b| b.activa(t))
    }

    /// Lo que lista `GET /batallas/pendientes`: las `convocando`, activas y fuera de convocatoria. Idempotente y sin efectos.
    pub fn batallas_pendientes(&self) -> Vec<u64> {
        let t = self.estado.ahora;
        self.batallas().filter(|b| b.pendiente_de_asignar(t)).map(|b| b.id).collect()
    }

    /// `GET /batallas/:id/ticket`: el ticket vigente, recuperable mientras la batalla siga en el estado (también una cerrada, durante la retención). Mientras
    /// dura la convocatoria de un asedio no se publica (`None`).
    pub fn ticket_publicado(&self, id: u64) -> Option<&Ticket> {
        self.batalla(id).filter(|b| !b.en_convocatoria(self.estado.ahora)).map(|b| &b.ticket)
    }

    /// `GET /batallas/:id/incorporaciones`: las incorporaciones vigentes, en orden de secuencia (sin las de quien salió).
    pub fn incorporaciones_vigentes(&self, id: u64) -> Vec<&combate::ciclo::Incorporacion> {
        self.batalla(id).map(|b| b.incorporaciones.iter().filter(|i| !b.salio(i.participante.heroe)).collect()).unwrap_or_default()
    }

    /// Lo que se bloquea ahora por las batallas activas (`bloqueosDe`).
    pub fn bloqueos(&self) -> Bloqueos {
        bloqueos_de(self.batallas(), self.estado.ahora)
    }

    /// La batalla activa que disputa una plaza, un pueblo o una avanzada (`Ubicacion::Asentamiento`, `Pueblo` o `Avanzada`), si la hay: es lo que el mapa enseña como «en asedio», para
    /// que desde fuera se vea y los neutrales y enemigos del dueño puedan unirse al ataque (`UnirseABatalla`). Su proyección al contrato es de la fase 4H.
    pub fn batalla_sobre(&self, lugar: Ubicacion) -> Option<&Batalla> {
        self.batallas_activas().find(|b| donde_esta(lugar, &b.ticket.contexto) == Donde::Dentro)
    }

    /// ¿Combate el héroe en alguna batalla activa? (Quien combate no sale del mundo hasta que acaba.)
    pub fn heroe_en_batalla(&self, heroe: u64) -> bool {
        self.batallas_activas().any(|b| b.heroes_que_combaten().any(|h| h == heroe))
    }

    /// El `battleId` del contrato de una batalla: un UUID determinista del id de la partida ([`Nucleo::id_de_partida`]) y del id de la batalla.
    pub fn uuid_de_batalla(&self, id: u64) -> String {
        uuid_de_batalla(&self.id_de_partida(), id)
    }

    /// El `gameId` del contrato de batalla: el que fijó quien sirve la partida (paridad P4: el servicio lo guarda en `<datos>/partida.id`) o, sin él, el hash del mundo y la semilla
    /// (que no distinguen dos partidas con el mismo mundo y la misma semilla).
    pub fn id_de_partida(&self) -> String {
        self.id_partida.clone().unwrap_or_else(|| format!("{}:{}", self.mundo_hash, self.semilla))
    }

    /// Fija el `gameId` de la partida (antes de servirla: cambia el `battleId` de las batallas).
    pub fn fijar_id_de_partida(&mut self, id: impl Into<String>) {
        self.id_partida = Some(id.into());
    }

    /// Inversa de [`Nucleo::uuid_de_batalla`]: el id de la partida de un `battleId`, si es de esta partida.
    pub fn id_de_batalla(&self, uuid: &str) -> Option<u64> {
        let hex = uuid.rsplit('-').take(2).collect::<Vec<_>>();
        let id = u64::from_str_radix(&format!("{}{}", hex.get(1)?, hex.first()?), 16).ok()?;
        (self.uuid_de_batalla(id) == uuid).then_some(id)
    }

    /// El token del propio jugador (§8.7). `NoExiste` si la batalla no está activa, `NoCombate` si el héroe no combate en ella, `SinToken` mientras no haya
    /// asignación o token para él.
    pub fn token_de_jugador(&self, heroe: u64, batalla: u64) -> TokenDeJugador {
        let Some(b) = self.batalla(batalla).filter(|b| b.activa(self.estado.ahora)) else { return TokenDeJugador::NoExiste };
        if b.participante(heroe).is_none() {
            return TokenDeJugador::NoCombate;
        }
        match b.asignacion.as_ref().and_then(|a| a.tokens.iter().find(|t| t.heroe == heroe).map(|t| (a, t))) {
            Some((a, t)) => TokenDeJugador::Token { instancia: a.instancia.clone(), token: t.token.clone(), expira_en: t.expira_en },
            None => TokenDeJugador::SinToken,
        }
    }

    /// Las batallas que salieron del estado al cumplir su retención (§3.3), en un núcleo sin persistencia (con ella van al histórico, [`HISTORICO_DE_BATALLAS`]). Se vacía al
    /// leer; no es estado: lo no recogido al cerrar el proceso se pierde.
    pub fn tomar_batallas_retiradas(&mut self) -> Vec<Batalla> {
        std::mem::take(&mut self.retiradas)
    }

    // ───────────────────────────── El candado de las órdenes ─────────────────────────────

    /// `tocaLoBloqueado` (§5): ¿choca la orden con algo que está en una batalla activa? Un héroe que combate no hace nada más, ni quien está dentro de una plaza
    /// asediada, salvo para salir (cuarta ronda); nadie apunta a una columna, caravana, plaza, pueblo o campamento bloqueados. Marchar hacia una plaza asediada sí se puede. La administración y
    /// las órdenes de §5.1 no lo llevan. Una orden rechazada no deja rastro.
    pub(super) fn rechazo_por_batalla(&self, t: Instante, p: &Peticion) -> Option<Respuesta> {
        if p.admin || !p.comando.lleva_candado() || self.batallas_activas().next().is_none() {
            return None;
        }
        let bloqueos = bloqueos_de(self.batallas(), t);
        let Implicados { heroes, ejercitos, caravanas, mut asentamientos, mut pueblos, mut avanzadas, campamentos } = p.comando.implicados();
        // Quien está dentro del lugar disputado y no se ha unido a la defensa solo puede salir (cuarta ronda): la orden de salir no toca el lugar ni cuenta el estar dentro; solo la detiene
        // combatir (quien se unió a la defensa no sale hasta que acaba).
        let salida = p.comando.es_salida_de_lugar();
        if salida {
            (asentamientos, pueblos, avanzadas) = Default::default();
        }
        // Un héroe dentro de la plaza asediada, del pueblo saqueado o de la avanzada atacada tampoco da otras órdenes.
        let heroe_bloqueado = |id: u64| {
            let dentro = self.heroe(id).map(|h| h.heroe.presencia.ubicacion);
            let plaza = match dentro {
                Some(Ubicacion::Asentamiento(a)) => Some(a),
                _ => None,
            };
            bloqueos.heroes.contains(&id)
                || (!salida
                    && (bloqueos.heroe_bloqueado(id, plaza) || matches!(dentro, Some(Ubicacion::Pueblo(x)) if bloqueos.pueblos.contains(&x)) || matches!(dentro, Some(Ubicacion::Avanzada(x)) if bloqueos.avanzadas.contains(&x))))
        };
        let actor = self.heroe_de_cuenta(&p.actor).map(|h| h.heroe.id);
        let choca = actor.into_iter().chain(heroes).any(heroe_bloqueado)
            || asentamientos.iter().any(|a| bloqueos.asentamientos.contains(&(*a as u32)))
            || ejercitos.iter().any(|e| bloqueos.ejercitos.contains(e))
            || caravanas.iter().any(|c| bloqueos.caravanas.contains(c))
            || pueblos.iter().any(|x| bloqueos.pueblos.contains(x))
            || avanzadas.iter().any(|x| bloqueos.avanzadas.contains(x))
            || campamentos.iter().any(|c| bloqueos.campamentos.contains(&(*c as u32)));
        choca.then(|| Respuesta::rechazo(rechazos::BATALLA_BLOQUEADO, "lo que tocas está en una batalla: ni se mueve ni recibe órdenes hasta que acabe", self.estado.version))
    }

    // ───────────────────────────── Abrir ─────────────────────────────

    /// Los héroes sanos de una columna con las escuadras suyas que lleva en ella y que tienen efectivos (Doc 5.15.1, 5.16.4).
    fn heroes_de_columna(&self, columna: u64, t: Instante) -> Vec<Participante> {
        let e = &self.estado.ejercitos[&columna];
        e.columna
            .participantes
            .iter()
            .filter_map(|p| self.heroe(p.heroe))
            .filter(|h| !h.heroe.esta_herido(t))
            .map(|h| {
                let suyas = h.heroe.escuadras.iter().filter(|s| s.contenedor == Contenedor::Ejercito(columna) && s.con_soldados());
                self.participante(h, suyas.map(|s| escuadra_en_batalla(s, t)).collect())
            })
            .collect()
    }

    fn participante(&self, h: &HeroeNucleo, escuadras: Vec<EscuadraEnBatalla>) -> Participante {
        // Los héroes todavía no tienen inventario: caben tantos objetos como casillas tiene la mochila (el botín de Unity no tiene dónde caer, CQ-004).
        Participante { heroe: h.heroe.id, faccion: h.faccion, controlador: h.heroe.controlador, casillas_libres: self.estado.reglas.economia.combate.batalla.casillas_inventario, escuadras }
    }

    fn bando_de_columna(&self, columna: u64, t: Instante) -> BandoDeTicket {
        BandoDeTicket::nuevo(self.estado.ejercitos[&columna].columna.faccion, self.heroes_de_columna(columna, t), vec![])
    }

    /// Una batalla campal (entre ejércitos) o una persecución (entre columnas personales).
    pub(super) fn apertura_de_columna(&self, t: Instante, heroe: u64, id: u64, x: u64) -> Apertura {
        let punto = self.estado.ejercitos[&x].posicion_en(t);
        let solitarios = self.estado.ejercitos[&id].columna.tipo == TipoColumna::Personal;
        Apertura {
            contexto: Contexto::CampoAbierto { punto, solitarios },
            punto,
            iniciada_por: heroe,
            atacante: self.bando_de_columna(id, t),
            defensor: self.bando_de_columna(x, t),
            bloqueo: Bloqueo { ejercitos: vec![id, x], ..Default::default() },
            plaza: None,
        }
    }

    /// Contra una caravana defiende su escolta cedida y, sin escolta, sus carreteros, que no son de nadie (Doc 3.10, 5.15.4).
    pub(super) fn apertura_de_caravana(&self, t: Instante, heroe: u64, id: u64, x: u64, punto: Punto, modalidad: Modalidad) -> Apertura {
        let k = self.estado.comercio.caravana(x).expect("existe");
        let escolta: Vec<EscuadraEnBatalla> = k.escolta.iter().filter_map(|c| self.escuadra(c.id).map(|(_, s)| s)).filter(|s| s.con_soldados()).map(|s| escuadra_en_batalla(s, t)).collect();
        let sin_heroe = if escolta.is_empty() {
            let (tropa, efectivos) = self.estado.reglas.economia.combate.batalla.carreteros;
            vec![EscuadraEnBatalla::sin_dueno(SIN_ID, tropa, efectivos)]
        } else {
            escolta
        };
        let faccion = self.estado.asentamientos.get(&(k.origen as u64)).map(|a| a.faccion);
        Apertura {
            contexto: Contexto::Caravana { caravanas: vec![CaravanaEnBatalla { id: x, carros: k.carros.len() }], punto, modalidad },
            punto,
            iniciada_por: heroe,
            atacante: self.bando_de_columna(id, t),
            defensor: BandoDeTicket::nuevo(faccion, vec![], sin_heroe),
            bloqueo: Bloqueo { ejercitos: vec![id], caravanas: vec![x], ..Default::default() },
            plaza: None,
        }
    }

    /// La Caravana de Fundación que vuelve sola (decisión del autor del 2026-10-24): una batalla de caravana como la de una comercial, con un solo carro —la caravana entera,
    /// `car-N:0`, con lo que costó— y los carreteros de defensa (no lleva escolta). Lo que pierde si gana el atacante lo decide `Nucleo::caer_fundacion`.
    pub(super) fn apertura_de_fundacion(&self, t: Instante, heroe: u64, id: u64, x: u64, punto: Punto, modalidad: Modalidad) -> Apertura {
        let (tropa, efectivos) = self.estado.reglas.economia.combate.batalla.carreteros;
        Apertura {
            contexto: Contexto::Caravana { caravanas: vec![CaravanaEnBatalla { id: x, carros: 1 }], punto, modalidad },
            punto,
            iniciada_por: heroe,
            atacante: self.bando_de_columna(id, t),
            defensor: BandoDeTicket::nuevo(self.estado.fundacion.caravanas[&x].faccion, vec![], vec![EscuadraEnBatalla::sin_dueno(SIN_ID, tropa, efectivos)]),
            bloqueo: Bloqueo { ejercitos: vec![id], caravanas: vec![x], ..Default::default() },
            plaza: None,
        }
    }

    /// La escuadra de milicia con las unidades del nivel de un campamento de bandidos, sin dueño ni Facción.
    fn bandidos_en_batalla(&self, campamento: u64) -> EscuadraEnBatalla {
        let nivel = self.estado.combate.bandidos[&campamento].nivel;
        let unidades = combate::bandidos::nivel_de(&self.estado.reglas.economia.combate, nivel).unidades;
        EscuadraEnBatalla::sin_dueno(SIN_ID, militar::Tropa::MiliciaLanceros, unidades)
    }

    /// Contra un campamento de bandidos defiende una escuadra de milicia con las unidades de su nivel, sin dueño ni Facción.
    pub(super) fn apertura_de_campamento(&self, t: Instante, heroe: u64, id: u64, x: u64) -> Apertura {
        let c = &self.estado.combate.bandidos[&x];
        Apertura {
            contexto: Contexto::CampamentoBandidos { campamento: x as u32, punto: c.posicion },
            punto: c.posicion,
            iniciada_por: heroe,
            atacante: self.bando_de_columna(id, t),
            defensor: BandoDeTicket::nuevo(None, vec![], vec![self.bandidos_en_batalla(x)]),
            bloqueo: Bloqueo { ejercitos: vec![id], campamento: Some(x as u32), ..Default::default() },
            plaza: None,
        }
    }

    /// ¿Se jugó ya (y se conserva) en Unity el acoso de este campamento a esta columna? Al volver al espacio tras la batalla la columna sigue en el radio del campamento y, por el
    /// redondeo de su posición, puede contar como una entrada nueva: sin esto los bandidos la acosarían otra vez, y otra, cada vez que acabara una batalla.
    pub(super) fn acoso_ya_jugado(&self, campamento: u64, columna: u64) -> bool {
        self.batallas().any(|b| b.iniciada_por == SIN_HEROE && b.bloqueo.campamento == Some(campamento as u32) && b.bloqueo.ejercitos.contains(&columna))
    }

    /// Los bandidos del campamento `campamento` acosan a la columna `columna` (quinta ronda, 2026-10-22): una batalla de caravana en la que los bandidos atacan (una escuadra de milicia
    /// sin dueño, sin héroes ni tokens) y defienden los héroes sanos de la columna con lo que llevan. **Todas** las caravanas adjuntas que son presa están en juego (decisión del autor,
    /// 2026-10-22): son parte del combate, y Unity nombra los carros perdidos de cada una. `None` si la columna no lleva ningún héroe sano o ninguna de sus caravanas es presa: sin héroes el
    /// acoso se resuelve siempre con números. Nadie inicia la batalla (`SIN_HEROE`): ningún jugador puede cancelarla.
    pub(super) fn apertura_de_acoso(&self, t: Instante, campamento: u64, columna: u64, punto: Punto) -> Option<Apertura> {
        let defensor = self.bando_de_columna(columna, t);
        if defensor.participantes.is_empty() {
            return None;
        }
        let caravanas: Vec<CaravanaEnBatalla> =
            self.estado.ejercitos[&columna].caravanas_adjuntas.iter().filter_map(|&k| self.estado.comercio.caravana(k)).filter(|k| self.es_presa_de_bandidos(k.estado, punto)).map(|k| CaravanaEnBatalla { id: k.id, carros: k.carros.len() }).collect();
        if caravanas.is_empty() {
            return None;
        }
        let bloqueadas = caravanas.iter().map(|k| k.id).collect();
        Some(Apertura {
            // El acoso no ocurre dentro de una zona de influencia, así que es siempre una disputa de carros.
            contexto: Contexto::Caravana { caravanas, punto, modalidad: Modalidad::Disputa },
            punto,
            iniciada_por: SIN_HEROE,
            atacante: BandoDeTicket::nuevo(None, vec![], vec![self.bandidos_en_batalla(campamento)]),
            defensor,
            bloqueo: Bloqueo { ejercitos: vec![columna], caravanas: bloqueadas, campamento: Some(campamento as u32), ..Default::default() },
            plaza: None,
        })
    }

    /// En un asedio defiende de oficio solo la guarnición sin héroe (Doc 5.15.1, 5.15.3). Los residentes y los amigos que están **dentro** (la Facción dueña, aliados, vasallos) no
    /// entran de oficio: se unen ellos con `UnirseABatalla`, durante la convocatoria o después (2026-10-22, tercera ronda). Sin nadie más, se juega igual.
    pub(super) fn apertura_de_asedio(&self, t: Instante, heroe: u64, id: u64, plaza: u64) -> Apertura {
        let a = &self.estado.asentamientos[&plaza];
        let residentes = self.estado.militar.residentes_de(plaza);
        let guarnicion = residentes.flat_map(|h| h.heroe.escuadras.iter()).filter(|s| s.contenedor == Contenedor::Campamento && s.en_guarnicion && s.con_soldados());
        Apertura {
            contexto: Contexto::Asedio { asentamiento: plaza as u32 },
            punto: a.posicion,
            iniciada_por: heroe,
            atacante: self.bando_de_columna(id, t),
            defensor: BandoDeTicket::nuevo(Some(a.faccion), vec![], guarnicion.map(|s| escuadra_en_batalla(s, t)).collect()),
            bloqueo: Bloqueo { ejercitos: vec![id], asentamiento: Some(plaza as u32), ..Default::default() },
            plaza: Some(self.plaza_congelada(plaza)),
        }
    }

    /// El mapa de un asedio (`SettlementBattleSnapshot`, Batalla_Unity_Ciclo §3.3): los edificios internos de la plaza y los recintos con solo las celdas en pie, tal como están al abrir. La
    /// posición de cada edificio es el centro de su huella en milésimas de unidad local, y su huella, en celdas con la rotación ya aplicada.
    fn plaza_congelada(&self, plaza: u64) -> PlazaCongelada {
        let a = &self.estado.asentamientos[&plaza];
        let celda = trazado::CELDA_UL;
        let edificios = self.vista_ciudad(plaza).into_iter().flat_map(|v| v.piezas).filter_map(|p| {
            let e = a.economia.edificio(p.id)?;
            Some(EdificioCongelado { id: p.id, tipo: p.tipo.nombre().to_owned(), centro: p.centro, ancho: (p.rect.ancho / celda) as u32, alto: (p.rect.alto / celda) as u32, nivel_interno: e.nivel_interno, estado: e.estado, danado: e.danado })
        });
        let recintos = self.recintos_en_pie(plaza).into_iter().map(|r| RecintoCongelado { id: r.id, nivel: r.nivel, celdas: r.celdas });
        PlazaCongelada { layout_version: trazado::LAYOUT_VERSION, unidades_por_celda: celda as u32, edificios: edificios.collect(), recintos: recintos.collect() }
    }

    /// Quien se une a defender desde dentro, con lo que lleva encima: el residente en su plaza, las escuadras de su loadout activo que tiene en el campamento; el resto (un visitante,
    /// o un residente dentro de un pueblo o una avanzada), las de la columna con la que entró. Su liderazgo ya dice cuántas lleva.
    fn defensor_de_dentro(&self, t: Instante, h: &HeroeNucleo, dentro: Ubicacion) -> Participante {
        let en_su_plaza = matches!(dentro, Ubicacion::Asentamiento(a) if h.heroe.residencia == Some(Residencia::Asentamiento(a)));
        let escuadras = if en_su_plaza { loadout_en_el_campamento(h) } else { de_su_columna(h) };
        self.participante(h, escuadras.into_iter().map(|s| escuadra_en_batalla(s, t)).collect())
    }

    /// El saqueo de un pueblo: defienden de oficio la guarnición de serie de mineros (sin dueño) y la Facción de la plaza madre; los de dentro se unen ellos (`UnirseABatalla`). Su
    /// escuadra ya existe (nace al primer ataque); si está a 0 esperando su reposición y nadie se une, el pueblo no tiene defensores.
    pub(super) fn apertura_de_pueblo(&self, t: Instante, heroe: u64, id: u64, pueblo: u64, madre: u64, punto: Punto) -> Apertura {
        let g = &self.estado.combate.guarniciones[&pueblo].escuadra;
        let mineros = Some(g).filter(|s| s.con_soldados()).map(|s| EscuadraEnBatalla { dueno: None, ..escuadra_en_batalla(s, t) });
        let faccion = self.estado.asentamientos[&madre].faccion;
        Apertura {
            contexto: Contexto::SaqueoPueblo { pueblo, punto },
            punto,
            iniciada_por: heroe,
            atacante: self.bando_de_columna(id, t),
            defensor: BandoDeTicket::nuevo(Some(faccion), vec![], mineros.into_iter().collect()),
            bloqueo: Bloqueo { ejercitos: vec![id], pueblo: Some(pueblo), ..Default::default() },
            plaza: None,
        }
    }

    /// El ataque a una avanzada se juega como un asedio, sin convocatoria: defienden de oficio la guarnición de serie (sin dueño), la de jugadores (escuadras de los héroes residentes
    /// de la madre, sin su héroe: la maneja la IA) y la Facción de la plaza madre; los de dentro se unen ellos (`UnirseABatalla`). La guarnición de serie ya existe (nace al primer
    /// ataque).
    pub(super) fn apertura_de_avanzada(&self, t: Instante, heroe: u64, id: u64, avanzada: u64, madre: u64) -> Apertura {
        let (_, v) = self.avanzada(avanzada).expect("existe");
        let serie = self.guarnicion_de_serie_de_avanzada(avanzada).iter().filter(|s| s.con_soldados()).map(|s| EscuadraEnBatalla { dueno: None, ..escuadra_en_batalla(s, t) });
        let jugadores = self.guarnicion_de_jugadores_de_avanzada(avanzada).into_iter().filter(|s| s.con_soldados()).map(|s| escuadra_en_batalla(s, t));
        let faccion = self.estado.asentamientos[&madre].faccion;
        Apertura {
            contexto: Contexto::Avanzada { avanzada, etapa: v.etapa, punto: v.posicion },
            punto: v.posicion,
            iniciada_por: heroe,
            atacante: self.bando_de_columna(id, t),
            defensor: BandoDeTicket::nuevo(Some(faccion), vec![], serie.chain(jugadores).collect()),
            bloqueo: Bloqueo { ejercitos: vec![id], avanzada: Some(avanzada), ..Default::default() },
            plaza: None,
        }
    }

    /// Abre la batalla en `convocando` con el ticket congelado: reserva las escuadras con dueño, aparta las columnas y caravanas, programa sus plazos y la
    /// cuenta. Devuelve su id. La orden que la abre ya comprobó todo y declaró la guerra, si hacía falta.
    pub(super) fn abrir_batalla(&mut self, t: Instante, mut ap: Apertura) -> u64 {
        let cr = self.estado.reglas.economia.combate.clone();
        let id = self.nuevo_id();
        // Lo que no es de nadie (carreteros, bandidos) no está en ningún héroe, pero el resultado tiene que cubrirlo: lleva un id de la partida.
        for e in ap.atacante.sin_heroe.iter_mut().chain(ap.defensor.sin_heroe.iter_mut()).filter(|e| e.id == SIN_ID) {
            e.id = self.nuevo_id();
        }
        let ticket = Ticket { plaza: ap.plaza, ..Ticket::nuevo(&cr, ap.contexto, ap.atacante, ap.defensor) };
        let b = Batalla::abrir(&cr, id, ap.iniciada_por, ap.punto, ticket, ap.bloqueo, t);
        self.reservar(&b.escuadras_con_candado(), Some(id));
        for c in &b.bloqueo.ejercitos {
            self.apartar_columna(t, *c);
        }
        for k in &b.bloqueo.caravanas {
            self.apartar_caravana(t, *k);
        }
        self.estado.batallas.batallas.insert(id, b);
        self.tr.tocar(Clase::Batalla, id);
        self.tocar_lo_disputado(id);
        self.programar_batalla(id);
        self.hecho_de_batalla(t, id, Hecho::Abierta, None);
        id
    }

    /// La orden de atacar abre la batalla en lugar de resolver el combate: declara la guerra, si hacía falta y todo lo demás ya valía, y la abre.
    pub(super) fn abrir_batalla_de_ataque(&mut self, t: Instante, ap: Apertura, guerra: Option<(u32, u32)>) {
        if let Some((a, b)) = guerra {
            self.declarar_guerra(t, a, b);
        }
        self.abrir_batalla(t, ap);
    }

    // ───────────────────────────── Ayudas ─────────────────────────────

    /// Pone (`Some`) o quita (`None`) el candado `reservaBatalla` a las escuadras de los héroes indicadas.
    fn reservar(&mut self, escuadras: &[u64], batalla: Option<u64>) {
        for h in self.estado.militar.heroes.values_mut() {
            if h.heroe.escuadras.iter().any(|s| escuadras.contains(&s.id)) {
                self.tr.tocar(Clase::Heroe, h.heroe.id);
            }
            for s in h.heroe.escuadras.iter_mut().filter(|s| escuadras.contains(&s.id)) {
                s.reserva_batalla = batalla;
            }
        }
    }

    fn escuadra_mut(&mut self, id: u64) -> Option<&mut Escuadra> {
        let h = self.estado.militar.heroes.values_mut().find(|h| h.heroe.escuadras.iter().any(|e| e.id == id))?;
        self.tr.tocar(Clase::Heroe, h.heroe.id);
        h.heroe.escuadra_mut(id)
    }

    /// Reprograma los plazos de la batalla con una generación nueva: el fin de la convocatoria (si aún no pasó), el vencimiento de su estado y, cerrada, su
    /// retirada.
    fn programar_batalla(&mut self, id: u64) {
        let t = self.estado.ahora;
        let Some(b) = self.estado.batallas.batallas.get(&id) else { return };
        let (cerrada, expira_en, convocatoria, retirada) = (b.cerrada(), b.expira_en, b.convocatoria_hasta, b.cerrada_en.unwrap_or(t) + self.estado.reglas.economia.combate.batalla.conservar_cerradas);
        let entidad = entidad_batalla(id);
        self.estado.generaciones.subir(entidad);
        let clave = self.estado.generaciones.clave(entidad);
        if cerrada {
            self.estado.cola.programar(retirada, PRIO_BATALLA, clave, Carga::Batalla(SucesoBatalla::Retira));
            self.tocar_lo_disputado(id);
            return;
        }
        if let Some(h) = convocatoria.filter(|&h| h > t) {
            self.estado.cola.programar(h, PRIO_BATALLA, clave, Carga::Batalla(SucesoBatalla::FinConvocatoria));
        }
        self.estado.cola.programar(expira_en.max(t), PRIO_BATALLA, clave, Carga::Batalla(SucesoBatalla::Vence));
    }

    /// Marca lo que la batalla disputa (la plaza asediada, el pueblo saqueado o la avanzada atacada): su `batalla_id` cambia al abrirse la batalla y al cerrarse.
    fn tocar_lo_disputado(&mut self, id: u64) {
        let Some(b) = self.estado.batallas.batallas.get(&id) else { return };
        let (plaza, pueblo, avanzada) = (b.bloqueo.asentamiento, b.bloqueo.pueblo, b.bloqueo.avanzada);
        if let Some(plaza) = plaza {
            self.tr.tocar(Clase::Asentamiento, u64::from(plaza));
        }
        if let Some(pueblo) = pueblo {
            self.tr.tocar(Clase::Pueblo, pueblo);
        }
        if let Some(avanzada) = avanzada {
            self.tr.tocar(Clase::Avanzada, avanzada);
        }
    }

    /// Cuenta la batalla: lo oyen las columnas que combaten y las plazas implicadas (la asediada, la madre del pueblo y el origen de cada caravana).
    fn hecho_de_batalla(&mut self, t: Instante, id: u64, hecho: Hecho, ganador: Option<Lado>) {
        let b = &self.estado.batallas.batallas[&id];
        let pve = matches!(b.ticket.contexto, Contexto::CampamentoBandidos { .. });
        let mut plazas: Vec<u64> = b.bloqueo.asentamiento.map(u64::from).into_iter().collect();
        plazas.extend(b.bloqueo.caravanas.iter().filter_map(|&k| self.estado.comercio.caravana(k)).map(|k| k.origen as u64));
        plazas.extend(b.bloqueo.caravanas.iter().filter_map(|k| self.estado.fundacion.caravanas.get(k)).filter_map(|k| if let crate::estado_fundacion::Origen::Plaza(a) = k.origen { Some(a) } else { None }));
        plazas.extend(b.bloqueo.pueblo.and_then(|p| self.pueblo(p)).map(|(madre, _)| madre));
        plazas.extend(b.bloqueo.avanzada.and_then(|a| self.avanzada(a)).and_then(|(madre, _)| madre));
        plazas.retain(|p| self.estado.asentamientos.contains_key(p));
        plazas.sort_unstable();
        plazas.dedup();
        let bandera = |f: Option<u32>| f.unwrap_or(SIN_BANDERA) as i64;
        let heroes = |l: Lado| b.heroes_que_combaten().filter(|&h| b.lado_de(h) == Some(l)).count() as i64;
        let mut datos = vec![
            ("batalla", id as i64),
            ("contexto", numero_de_contexto(&b.ticket.contexto)),
            ("faccionAtacante", bandera(b.ticket.atacante.faccion)),
            ("faccionDefensora", bandera(b.ticket.defensor.faccion)),
            ("heroesAtacante", heroes(Lado::Atacante)),
            ("heroesDefensor", heroes(Lado::Defensor)),
            ("capacidad", b.ticket.atacante.capacidad as i64),
        ];
        if let Some(l) = ganador {
            datos.push(("ganaElAtacante", (l == Lado::Atacante) as i64));
        }
        let mut valores = Vec::new();
        if let Hecho::Abierta = hecho {
            // Lo que el tipado `BatallaAbierta` dice, en el propio hecho: así se tipa igual aunque la batalla ya no esté en el estado cuando se proyecte (una reanudación tardía).
            datos.extend(objetivo_de_contexto(&b.ticket.contexto));
            if b.iniciada_por != SIN_HEROE {
                datos.push(("iniciador", b.iniciada_por as i64));
            }
            datos.extend(b.convocatoria_hasta.map(|h| ("convocatoriaHasta", h)));
            if let Contexto::Caravana { caravanas, .. } = &b.ticket.contexto {
                valores.push(("caravanasIds", Dato::Lista(caravanas.iter().map(|k| Dato::Texto(ids::caravana(k.id))).collect())));
            }
        }
        let ejercitos = b.bloqueo.ejercitos.iter().copied().filter(|e| self.estado.ejercitos.contains_key(e)).collect();
        // Lo oye quien combate o combatió en ella aunque no tenga plaza ni siga en una columna del bloqueo (el cursor del TypeScript, §14; paridad P4).
        let mut heroes: Vec<u64> = b.heroes_que_combaten().chain(b.salidas.iter().map(|s| s.heroe)).filter(|&h| h != SIN_HEROE).collect();
        heroes.sort_unstable();
        heroes.dedup();
        let codigo = hecho.codigo(pve);
        self.tr.emitir(&self.estado, EventoNucleo { codigo: Cow::Borrowed(codigo), en: t, ejercitos, asentamientos: plazas, punto: Some(b.punto), datos, valores, heroes, ..Default::default() });
    }

    /// Las columnas de un bando que siguen en la batalla, en el orden del bloqueo: la primera es la que abrió o recibió el combate.
    fn columnas_del_bando(&self, b: &Batalla, lado: Lado) -> Vec<u64> {
        b.bloqueo.ejercitos.iter().copied().filter(|c| self.estado.ejercitos.get(c).is_some_and(|e| e.columna.participantes.iter().any(|p| b.lado_de(p.heroe) == Some(lado)))).collect()
    }

    /// El reparto por rondas del botín entre las columnas, hasta donde quepa en cada carro. Devuelve lo que se cargó (lo que no cabe en ninguna se pierde).
    pub(super) fn repartir_en_columnas(&mut self, columnas: &[u64], botin: &Mercancia) -> Mercancia {
        if columnas.is_empty() || botin.is_empty() {
            return Mercancia::new();
        }
        let mut carros: Vec<CarroDeColumna> = columnas.iter().map(|&c| CarroDeColumna { carga: self.estado.ejercitos[&c].columna.carga.clone(), capacidad: self.capacidad_de_carga(c).unwrap_or(0) }).collect();
        let perdido = repartir_botin(&mut carros, botin);
        for (&c, carro) in columnas.iter().zip(carros) {
            self.estado.ejercitos.get_mut(&c).expect("existe").columna.carga = carro.carga;
            self.tr.tocar(Clase::Ejercito, c);
        }
        let mut cogido = Mercancia::new();
        for (&r, &n) in botin {
            sumar(&mut cogido, r, n - perdido.get(&r).copied().unwrap_or(0));
        }
        cogido.retain(|_, n| *n > 0);
        cogido
    }

    /// La batalla se cierra sin resultado (cancelada o fallida): se sueltan los candados y vuelve al mundo lo que estaba apartado. Reprograma los plazos
    /// (queda la retirada) y la cuenta.
    fn cerrar_batalla(&mut self, t: Instante, id: u64, liberar: &[u64], hecho: Hecho) {
        self.reservar(liberar, None);
        self.devolver_al_mundo(t, id);
        self.programar_batalla(id);
        self.hecho_de_batalla(t, id, hecho, None);
    }

    /// Lo que la batalla tenía apartado vuelve al espacio y quien pidió salir del mundo mientras combatía lo reintenta ya.
    fn devolver_al_mundo(&mut self, t: Instante, id: u64) {
        let b = &self.estado.batallas.batallas[&id];
        let (columnas, caravanas) = (b.bloqueo.ejercitos.clone(), b.bloqueo.caravanas.clone());
        let heroes: Vec<u64> = b.ticket.atacante.participantes.iter().chain(&b.ticket.defensor.participantes).chain(b.incorporaciones.iter().map(|i| &i.participante)).map(|p| p.heroe).collect();
        for c in columnas {
            self.reponer_columna(t, c);
        }
        for k in caravanas {
            self.reponer_caravana(t, k);
        }
        // Los que combatían dentro de una plaza no tienen columna que lo haga por ellos.
        for h in heroes {
            let pendiente = self.heroe(h).is_some_and(|x| x.heroe.presencia.desconecta_en.is_some() && !x.heroe.presencia.fuera && !matches!(x.heroe.presencia.ubicacion, Ubicacion::Columna(_)));
            if pendiente && !self.heroe_en_batalla(h) {
                let clave = self.estado.generaciones.clave(h);
                self.estado.cola.programar(t, PRIO_SALIDA, clave, Carga::SalidaDelMundo);
            }
        }
    }

    // ───────────────────────────── Los plazos ─────────────────────────────

    /// Vence un plazo de una batalla (la clave de la cola ya garantiza que no cambió de estado desde que se programó).
    pub(super) fn suceso_de_batalla(&mut self, id: u64, suceso: SucesoBatalla, t: Instante) {
        match suceso {
            SucesoBatalla::FinConvocatoria if !self.estado.reglas.batallas_en_unity => self.asedio_con_numeros(t, id),
            SucesoBatalla::FinConvocatoria => {
                if self.estado.batallas.batallas.get(&id).is_some_and(|b| b.activa(t)) {
                    self.hecho_de_batalla(t, id, Hecho::ConvocatoriaCerrada, None);
                }
            }
            SucesoBatalla::Vence => {
                let Some(liberar) = self.estado.batallas.batallas.get_mut(&id).and_then(|b| b.vencer(t)) else { return };
                self.tr.tocar(Clase::Batalla, id);
                self.cerrar_batalla(t, id, &liberar, Hecho::Fallida);
            }
            SucesoBatalla::Retira => {
                let cr = self.estado.reglas.economia.combate.clone();
                if self.estado.batallas.batallas.get(&id).is_some_and(|b| b.retirable(&cr, t)) {
                    let b = self.estado.batallas.batallas.remove(&id).expect("existe");
                    self.tr.tocar(Clase::Batalla, id);
                    // Sin esto la retirada cambiaría el estado sin dejar versión. Lo oyen las Facciones de los dos bandos, que la tenían.
                    let facciones = [b.ticket.atacante.faccion, b.ticket.defensor.faccion].into_iter().flatten().collect();
                    self.archivar_batalla(b);
                    self.tr.emitir(&self.estado, EventoNucleo { facciones, ..EventoNucleo::nuevo("batalla.retirada", t, vec![("batalla", id as i64)]) });
                }
            }
        }
    }

    /// Sin servidores de batalla, el asedio acaba su convocatoria y se resuelve con números con todas las columnas que se unieron al ataque (calibración del sistema 4, M8): la
    /// batalla queda `aplicada` sin resultado de Unity, suelta sus candados y devuelve al mundo lo apartado, y el combate es el de siempre ([`Nucleo::resolver_asedio_con_numeros`]).
    fn asedio_con_numeros(&mut self, t: Instante, id: u64) {
        let Some(liberar) = self.estado.batallas.batallas.get_mut(&id).and_then(|b| b.resolver_con_numeros(t)) else { return };
        self.tr.tocar(Clase::Batalla, id);
        let b = self.estado.batallas.batallas[&id].clone();
        let Contexto::Asedio { asentamiento } = b.ticket.contexto else { unreachable!("solo un asedio tiene convocatoria") };
        let atacantes = self.columnas_del_bando(&b, Lado::Atacante);
        self.reservar(&liberar, None);
        self.devolver_al_mundo(t, id);
        self.programar_batalla(id);
        let cae = self.resolver_asedio_con_numeros(t, u64::from(asentamiento), &atacantes);
        self.hecho_de_batalla(t, id, Hecho::Aplicada, Some(if cae { Lado::Atacante } else { Lado::Defensor }));
    }

    // ───────────────────────────── Las órdenes ─────────────────────────────

    /// Las órdenes del ciclo: unirse y cancelar (de un jugador) y los cinco mensajes del servidor de batalla.
    pub(super) fn ejecutar_batallas(&mut self, t: Instante, p: &Peticion) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        match &p.comando {
            Comando::FijarControlador { heroe, controlador } => self.fijar_controlador(*heroe, *controlador),
            Comando::UnirseABatalla { heroe, batalla, lado } => self.unirse_a_batalla(t, p, *heroe, *batalla, *lado),
            Comando::CancelarBatalla { heroe, batalla } => self.cancelar_batalla(t, p, *heroe, *batalla),
            Comando::RegistrarAsignacion { batalla, asignacion } => {
                let servidor = self.servidor_de(p)?;
                let a = Asignacion { servidor, ..asignacion.clone() };
                let cr = self.estado.reglas.economia.combate.clone();
                let b = self.batalla_mut(*batalla)?;
                match b.registrar_asignacion(&cr, t, a).map_err(|e| rechazo_de_batalla(e, v))? {
                    Efecto::SinCambios => Err(sin_cambios(v)),
                    Efecto::Aplicado(()) => {
                        self.programar_batalla(*batalla);
                        self.hecho_de_batalla(t, *batalla, Hecho::Asignada, None);
                        Ok(vec![])
                    }
                }
            }
            Comando::ConfirmarInicio { batalla, intento, revision } => {
                let servidor = self.servidor_de(p)?;
                let cr = self.estado.reglas.economia.combate.clone();
                let b = self.batalla_mut(*batalla)?;
                match b.confirmar_inicio(&cr, t, intento, &servidor, *revision).map_err(|e| rechazo_de_batalla(e, v))? {
                    Efecto::SinCambios => Err(sin_cambios(v)),
                    Efecto::Aplicado(()) => {
                        self.programar_batalla(*batalla);
                        self.hecho_de_batalla(t, *batalla, Hecho::EnCurso, None);
                        Ok(vec![])
                    }
                }
            }
            Comando::RegistrarTokens { batalla, intento, tokens } => {
                let servidor = self.servidor_de(p)?;
                let b = self.batalla_mut(*batalla)?;
                let antes = ordenados(b.asignacion.as_ref().map(|a| a.tokens.clone()));
                b.registrar_tokens(t, intento, &servidor, tokens.clone()).map_err(|e| rechazo_de_batalla(e, v))?;
                // Sin evento: que alguien tenga ya su token no es noticia para nadie más. Repetir los mismos no cambia nada.
                if ordenados(b.asignacion.as_ref().map(|a| a.tokens.clone())) == antes {
                    return Err(sin_cambios(v));
                }
                Ok(vec![])
            }
            Comando::RegistrarSalida { batalla, salida } => {
                let servidor = self.servidor_de(p)?;
                self.registrar_salida(t, *batalla, &servidor, salida.clone())
            }
            Comando::AplicarResultado { batalla, resultado } => {
                let servidor = self.servidor_de(p)?;
                self.aplicar_resultado(t, *batalla, &servidor, resultado.clone())
            }
            otra => unreachable!("no es una orden de batalla: {otra:?}"),
        }
    }

    /// Quién manda el mensaje: el servidor de batalla cuya credencial lleva el actor (`batalla-servidor:<id>`), nunca la cuenta de un jugador.
    fn servidor_de(&self, p: &Peticion) -> Result<String, Respuesta> {
        p.actor.strip_prefix(crate::comando::ACTOR_SERVIDOR_DE_BATALLA).filter(|s| !s.is_empty()).map(str::to_owned).ok_or_else(|| {
            Respuesta::rechazo(rechazos::BATALLA_SIN_CREDENCIAL, "los mensajes del servidor de batalla llevan la credencial del servidor, no la de un jugador", self.estado.version)
        })
    }

    /// Una batalla **activa**; si no existe, está cerrada o su plazo venció, `batalla.no_existe` (404).
    fn batalla_mut(&mut self, id: u64) -> Result<&mut Batalla, Respuesta> {
        let (v, t) = (self.estado.version, self.estado.ahora);
        self.tr.tocar(Clase::Batalla, id);
        self.estado.batallas.batallas.get_mut(&id).filter(|b| b.activa(t)).ok_or_else(|| Respuesta::rechazo(rechazos::BATALLA_NO_EXISTE, format!("batalla-{id}"), v))
    }

    fn fijar_controlador(&mut self, heroe: u64, controlador: Controlador) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let h = self.heroe(heroe).ok_or_else(|| Respuesta::rechazo(rechazos::HEROE_DESCONOCIDO, ids::heroe(heroe), v))?;
        if h.heroe.controlador == controlador {
            return Err(sin_cambios(v));
        }
        self.heroe_mut(heroe).heroe.controlador = controlador;
        Ok(vec![])
    }

    /// `unirseABatalla` (§7). Se defiende un asedio, un pueblo o una avanzada desde **dentro** (quien ha entrado y es de la Facción dueña, aliado o vasallo: residentes incluidos, que ya no
    /// entran de oficio) con lo que lleva encima, y se ataca desde **fuera**, en el mapa, con la columna entera, a ≤ `radioEncuentro` de la batalla: allí se admite a la Facción atacante y
    /// a los neutrales y enemigos del dueño, no a sus amigos (2026-10-22, tercera ronda). Quien está dentro de otro lugar no se une a ninguno de los dos bandos.
    fn unirse_a_batalla(&mut self, t: Instante, p: &Peticion, heroe: u64, id: u64, pedido: Option<Lado>) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        self.autorizar_heroe(p, heroe, false)?;
        let b = self.batalla_mut(id)?.clone();
        let (lado, donde, columna) = self.elegibilidad(t, heroe, &b, pedido)?;
        let hn = self.heroe(heroe).expect("autorizado");
        let entran = match donde {
            Donde::Dentro => vec![self.defensor_de_dentro(t, hn, hn.heroe.presencia.ubicacion)],
            _ => self.heroes_de_columna(columna.expect("comprobado"), t),
        };
        let reservadas: Vec<u64> = entran.iter().flat_map(|p| p.escuadras.iter().map(|e| e.id)).collect();
        let secuencias = self.batalla_mut(id)?.unirse(t, lado, entran, columna).map_err(|e| rechazo_de_batalla(e, v))?;
        self.reservar(&reservadas, Some(id));
        if let Some(c) = columna {
            self.apartar_columna(t, c);
        }
        self.hecho_de_batalla(t, id, Hecho::Refuerzos, None);
        Ok(secuencias.into_iter().map(u64::from).collect())
    }

    /// Las condiciones de §7 para que `heroe` se una a `b`, en su orden: sano, sin estar ya en una batalla, a su alcance (con su columna si está fuera; dentro de lo que se disputa no hace falta
    /// ni columna ni distancia), con su columna libre y con un bando que le toque. Devuelve el bando, dónde está respecto de lo que se disputa y su columna. No toca nada, así que también sirve para
    /// saber qué podría hacer ([`Nucleo::lado_para_unirse`]).
    fn elegibilidad(&self, t: Instante, heroe: u64, b: &Batalla, pedido: Option<Lado>) -> Result<(Lado, Donde, Option<u64>), Respuesta> {
        let v = self.estado.version;
        let invalida = |m: &str| Respuesta::rechazo(rechazos::BATALLA_INVALIDA, m, v);
        let hn = self.heroe(heroe).ok_or_else(|| Respuesta::rechazo(rechazos::HEROE_DESCONOCIDO, ids::heroe(heroe), v))?;
        if self.heroe_herido(heroe) {
            return Err(invalida("estás herido: no puedes entrar en batalla"));
        }
        let bloqueos = self.bloqueos();
        if bloqueos.heroes.contains(&heroe) {
            return Err(invalida("ya estás en una batalla"));
        }
        let donde = donde_esta(hn.heroe.presencia.ubicacion, &b.ticket.contexto);
        let columna = self.estado.ejercito_de_heroe(heroe);
        // Quien está fuera llega con su columna y a su alcance; quien está dentro está donde se disputa y no necesita ni columna ni distancia.
        if donde != Donde::Dentro {
            let Some(c) = columna else { return Err(invalida("hay que llegar con tu columna")) };
            let radio = self.estado.reglas.economia.militar.logistica.radio_encuentro;
            if self.estado.ejercitos[&c].posicion_en(t).dist(b.punto) > radio {
                return Err(invalida(&format!("hay que estar a {radio} o menos de la batalla")));
            }
        }
        if columna.is_some_and(|c| bloqueos.ejercitos.contains(&c)) {
            return Err(invalida("tu columna ya está en una batalla"));
        }
        let aliadas = |a: u32, b: u32| matches!(self.estado.relacion(a, b), Some(Relacion::Alianza | Relacion::Vasallaje));
        let personal = columna.is_some_and(|c| self.estado.ejercitos[&c].columna.tipo == TipoColumna::Personal);
        // «El Rey admite otras Facciones en sus ataques» (`admiteOtrasEnAtaques`, 5D): solo cuenta ya para las caravanas (asedios, pueblos y avanzadas admiten a los no amigos del dueño).
        let rey_admite_otras = b.ticket.atacante.faccion.and_then(|f| self.estado.politica.facciones.get(f)).is_some_and(|f| f.admite_otras_en_ataques);
        let reparto = Reparto { contexto: b.ticket.contexto.clone(), atacante: b.ticket.atacante.faccion, defensor: b.ticket.defensor.faccion, aliadas_en_batalla: &aliadas, rey_admite_otras, donde };
        let lado = lado_para_unirse(&reparto, hn.faccion, personal, pedido).map_err(|e| rechazo_de_union(e, v))?;
        Ok((lado, donde, columna))
    }

    /// El bando al que `heroe` podría unirse ahora a la batalla `id` (`Batalla.lado_para_unirse`): el de las mismas condiciones que `UnirseABatalla`, sin elegir bando. `None` si no puede: la batalla no
    /// está activa, ya combate (en ella o en otra), salió de ella, no llega o no es de ninguno. No mira si el bando está lleno (`capacidadMaxima`): lo rechaza la
    /// orden al intentarlo (Batalla_Unity_Ciclo §7).
    pub fn lado_para_unirse(&self, heroe: u64, id: u64) -> Option<Lado> {
        let t = self.estado.ahora;
        let b = self.batalla(id).filter(|b| b.activa(t) && !b.salio(heroe))?;
        self.elegibilidad(t, heroe, b, None).ok().map(|(lado, _, _)| lado)
    }

    /// `cancelarBatalla`: quien la inició se echa atrás antes de que empiece la partida; nadie pierde nada.
    fn cancelar_batalla(&mut self, t: Instante, p: &Peticion, heroe: u64, id: u64) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        self.autorizar_heroe(p, heroe, false)?;
        let liberar = self.batalla_mut(id)?.cancelar(t, heroe).map_err(|e| rechazo_de_batalla(e, v))?;
        self.cerrar_batalla(t, id, &liberar, Hecho::Cancelada);
        Ok(vec![])
    }

    /// `registrarSalida` (§8.4, CQ-011): un héroe sale de la batalla. Antes de empezar (`no_conectado`) sin castigo y se reabre la convocatoria o se cancela; con la
    /// partida en curso se aplican las bajas de sus escuadras y, si lo dan por derrotado, queda herido.
    fn registrar_salida(&mut self, t: Instante, id: u64, servidor: &str, s: SalidaBatalla) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let cr = self.estado.reglas.economia.combate.clone();
        let heroe = s.heroe;
        let aplicada = match self.batalla_mut(id)?.registrar_salida(&cr, t, servidor, s).map_err(|e| rechazo_de_batalla(e, v))? {
            Efecto::SinCambios => return Err(sin_cambios(v)),
            Efecto::Aplicado(a) => a,
        };
        // Las bajas de sus escuadras (la columna vuelve a anotar lo que come y a qué velocidad marcha cuando acabe la batalla).
        for &(escuadra, supervivientes) in &aplicada.bajas {
            if let Some(e) = self.escuadra_mut(escuadra) {
                e.cantidad = supervivientes;
            }
        }
        self.reservar(&aplicada.liberar, None);
        if aplicada.herido {
            self.herir_heroes(&[heroe], t);
        }
        if aplicada.cancelada {
            self.cerrar_batalla(t, id, &[], Hecho::Cancelada);
            return Ok(vec![]);
        }
        // Su columna deja de estar quieta si no queda nadie más de ella en la batalla.
        if let Some(columna) = self.estado.ejercito_de_heroe(heroe) {
            self.tr.tocar(Clase::Batalla, id);
            let b = self.estado.batallas.batallas.get_mut(&id).expect("existe");
            let sigue = self.estado.ejercitos.get(&columna).is_some_and(|e| e.columna.participantes.iter().any(|p| b.lado_de(p.heroe).is_some()));
            if !sigue && b.bloqueo.ejercitos.contains(&columna) {
                b.bloqueo.ejercitos.retain(|&c| c != columna);
                self.reponer_columna(t, columna);
            }
        }
        if aplicada.reabierta {
            self.programar_batalla(id);
        }
        self.hecho_de_batalla(t, id, if aplicada.reabierta { Hecho::Revisada } else { Hecho::Salida }, None);
        Ok(vec![])
    }

    // ───────────────────────────── El resultado ─────────────────────────────

    /// `aplicarResultado` (§8.5, §8.6, §9): el checklist entero o nada y, en una sola mutación, las consecuencias. Repetir el mismo resultado sobre una
    /// `aplicada` responde bien sin cambiar nada; otro distinto se rechaza.
    fn aplicar_resultado(&mut self, t: Instante, id: u64, servidor: &str, res: ResultadoDeBatalla) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        // Los `itemInstanceId` ya recibidos en otras batallas que se conservan: los héroes aún no tienen inventario donde mirar (CQ-004).
        let existentes: BTreeSet<String> = self.batallas().filter(|b| b.id != id).filter_map(|b| b.resultado.as_ref()).flat_map(|r| r.por_heroe.iter()).filter_map(|h| h.botin.as_ref()).flat_map(|b| b.objetos.iter()).filter_map(|o| o.instancia.clone()).collect();
        self.tr.tocar(Clase::Batalla, id);
        let b = self.estado.batallas.batallas.get_mut(&id).ok_or_else(|| Respuesta::rechazo(rechazos::BATALLA_NO_EXISTE, format!("batalla-{id}"), v))?;
        let efectos = match b.aplicar_resultado(t, servidor, res.clone(), &|i| existentes.contains(i)).map_err(|e| rechazo_de_batalla(e, v))? {
            Efecto::SinCambios => return Err(sin_cambios(v)),
            Efecto::Aplicado(e) => e,
        };
        let b = self.estado.batallas.batallas[&id].clone();
        let mr = self.estado.reglas.economia.militar.clone();

        // 1-2. Lo que Unity dice de cada uno: supervivientes y experiencia de cada escuadra (las de los héroes, las de la guarnición de un pueblo), experiencia y
        // monedas de cada héroe, y los candados fuera. El nivel del héroe no sube (CQ-001) y los objetos del botín no tienen dónde caer (CQ-004).
        for e in &efectos.escuadras {
            if let Some(s) = self.escuadra_mut(e.escuadra) {
                s.cantidad = e.cantidad;
                s.con_experiencia(&mr, e.xp);
            } else if let Some((&pueblo, g)) = self.estado.combate.guarniciones.iter().find(|(_, g)| g.escuadra.id == e.escuadra) {
                // La guarnición de serie de un pueblo no es de ningún héroe: queda por debajo de su número, empieza su reposición gratis.
                let mut mineros = g.escuadra.clone();
                mineros.cantidad = e.cantidad;
                mineros.con_experiencia(&mr, e.xp);
                self.guardar_guarnicion(pueblo, mineros, t);
            } else if let Some((&avanzada, g)) = self.estado.combate.avanzadas.iter().find(|(_, g)| g.escuadras.iter().any(|s| s.id == e.escuadra)) {
                // Lo mismo con la guarnición de serie de una avanzada.
                let mut escuadras = g.escuadras.clone();
                let s = escuadras.iter_mut().find(|s| s.id == e.escuadra).expect("está");
                s.cantidad = e.cantidad;
                s.con_experiencia(&mr, e.xp);
                self.guardar_guarnicion_de_avanzada(avanzada, escuadras, t);
            }
        }
        for e in &efectos.heroes {
            let Some(h) = self.estado.militar.heroes.get_mut(&e.heroe) else { continue };
            self.tr.tocar(Clase::Heroe, e.heroe);
            h.heroe.experiencia += e.xp;
            if let Some(botin) = &e.botin {
                h.heroe.monedas.bronce += botin.monedas.bronce;
                h.heroe.monedas.plata += botin.monedas.plata;
                h.heroe.monedas.oro += botin.monedas.oro;
            }
        }
        self.reservar(&efectos.soltar, None);
        // 1. Heridos.
        self.herir_heroes(&efectos.heridos, t);
        // 4-5. Consecuencias según el contexto y chatarra de los caídos.
        // Defender al vasallo (§5.9), antes de las consecuencias (una conquista cambiaría la señora de la vasalla): los héroes del bando defensor que participaron.
        let defendieron: Vec<u64> = self.participantes_del_lado(&b, Lado::Defensor).iter().map(|q| q.heroe).filter(|&h| res.por_heroe.iter().any(|x| x.heroe == h && x.participo)).collect();
        self.defensa_de_vasallo(t, b.ticket.defensor.faccion, &defendieron, res.ganador == Lado::Defensor);
        // Los logros de la batalla (D10), con la plaza de un asedio tal como estaba antes de su conquista (que daña la muralla); nunca desde `batalla.aplicada` (D18).
        let logros = self.contadores_de_batalla(&b, res.ganador);
        // El hecho de épica de la batalla (6E), con las Facciones del ticket y por su id: se canta una vez aunque toque varias plazas.
        let contexto = match b.ticket.contexto {
            Contexto::CampoAbierto { .. } => tecnologia::epicas::ContextoUnity::CampoAbierto,
            Contexto::Asedio { asentamiento } => tecnologia::epicas::ContextoUnity::Asedio(asentamiento),
            _ => tecnologia::epicas::ContextoUnity::Otro,
        };
        self.hecho_epico(tecnologia::epicas::Hecho::de_unity(id, contexto, b.ticket.atacante.faccion, b.ticket.defensor.faccion, Some(res.ganador == Lado::Atacante)));
        self.consecuencias_de_batalla(t, &b, &res, &efectos);
        self.contar(t, logros);
        // Las misiones (Misiones §3): cada héroe que participó libró una batalla; los atacantes que la ganan a un campamento lo destruyen; un asedio se resiste o se conquista.
        let heroes: Vec<u64> = [Lado::Atacante, Lado::Defensor].into_iter().flat_map(|l| self.participantes_del_lado(&b, l)).map(|q| q.heroe).filter(|&h| res.por_heroe.iter().any(|x| x.heroe == h && x.participo)).collect();
        self.avance_de_heroes(t, &heroes, "batallas", 1);
        match b.ticket.contexto {
            Contexto::CampamentoBandidos { .. } if res.ganador == Lado::Atacante => {
                let atacantes: Vec<u64> = self.participantes_del_lado(&b, Lado::Atacante).iter().map(|q| q.heroe).collect();
                self.avance_de_heroes(t, &atacantes, "bandidos", 1);
                if let Some(f) = b.ticket.atacante.faccion {
                    self.avance(t, Dueno::Faccion(f), "bandidos", 1);
                }
            }
            Contexto::Asedio { asentamiento } => {
                let plaza = asentamiento as u64;
                match b.ticket.atacante.faccion {
                    Some(f) if res.ganador == Lado::Atacante && self.estado.asentamientos.get(&plaza).is_some_and(|a| a.faccion == f) => self.avance(t, Dueno::Faccion(f), "conquistas", 1),
                    _ if res.ganador == Lado::Defensor => self.avance(t, Dueno::Asentamiento(plaza), "asedios", 1),
                    _ => {}
                }
            }
            _ => {}
        }
        // La experiencia de Facción: por cada héroe que participó, a la Facción con la que entró (la del ticket), sin filtro de dignidad (5C). Contra un campamento de bandidos, solo la
        // de bandidos al destruirlo (5C-4, decisión del autor del 2026-10-24): el mismo premio con Unity que con números.
        let participo = |h: u64| res.por_heroe.iter().any(|x| x.heroe == h && x.participo);
        let mut por_faccion = std::collections::BTreeMap::new();
        if !matches!(b.ticket.contexto, Contexto::CampamentoBandidos { .. }) {
            for p in [Lado::Atacante, Lado::Defensor].into_iter().flat_map(|l| self.participantes_del_lado(&b, l)) {
                if let Some(f) = p.faccion.filter(|_| participo(p.heroe)) {
                    *por_faccion.entry(f).or_insert(0u32) += 1;
                }
            }
        }
        self.experiencia_de_batalla(t, por_faccion);
        // Vuelve al mundo lo que estaba apartado (las columnas retoman su plan con la velocidad que les dejan las bajas) y se programa la retirada.
        self.devolver_al_mundo(t, id);
        self.programar_batalla(id);
        self.hecho_de_batalla(t, id, Hecho::Aplicada, Some(res.ganador));
        Ok(vec![])
    }

    /// Lo que una batalla de Unity suma a los logros del servidor (`contadoresDeBatalla` del TypeScript; Tecnologia_Aedas §2.1): lo mismo que un combate con números. La de una avanzada o
    /// un saqueo de pueblo (contextos que el TypeScript no tenía) cuenta como batalla librada y nada más.
    fn contadores_de_batalla(&self, b: &Batalla, ganador: Lado) -> tecnologia::contador::Delta {
        use tecnologia::contador::{batalla_unity, combate_resuelto, ContextoBatalla};
        let lados = [Lado::Atacante, Lado::Defensor];
        let escuadras = lados.iter().flat_map(|&l| self.participantes_del_lado(b, l).into_iter().flat_map(|p| p.escuadras.iter()).chain(&b.ticket.bando(l).sin_heroe).map(|e| e.tropa).collect::<Vec<_>>());
        let hoplitas = super::logros::con_hoplitas(escuadras);
        let contexto = match b.ticket.contexto {
            Contexto::CampoAbierto { .. } => ContextoBatalla::CampoAbierto,
            Contexto::Caravana { .. } => ContextoBatalla::Caravana,
            Contexto::CampamentoBandidos { .. } => ContextoBatalla::CampamentoBandidos,
            Contexto::Asedio { asentamiento } => ContextoBatalla::Asedio {
                muralla_completa: self.estado.asentamientos.get(&u64::from(asentamiento)).is_some_and(|a| asentamiento::recintos::muralla_completa(&a.economia.recintos)),
                con_heroe_defensor: !self.participantes_del_lado(b, Lado::Defensor).is_empty(),
            },
            Contexto::Avanzada { .. } | Contexto::SaqueoPueblo { .. } => return combate_resuelto(hoplitas),
        };
        batalla_unity(contexto, ganador == Lado::Atacante, hoplitas)
    }

    /// Lo que decide BronzeAge según dónde se combatió (§9, pasos 4 y 5). **Sin** la reputación por defender a un vasallo (5D) ni los contadores de logros (6B, `contadores_de_batalla`); la
    /// experiencia de Facción la da `aplicar_resultado` (5C), y la de bandidos, la captura del campamento.
    fn consecuencias_de_batalla(&mut self, t: Instante, b: &Batalla, res: &ResultadoDeBatalla, efectos: &Efectos) {
        let (ganador, perdedor) = (res.ganador, res.ganador.otro());
        match &b.ticket.contexto {
            // Un asedio ganado por el atacante es una conquista: la plaza pasa a su Facción con el saqueo de siempre (sin Facción, no se conquista nada). Unity no
            // reporta daño: lo pone el saqueo. Ganado por el defensor, nada más (no hay pérdida de carro en un asedio).
            &Contexto::Asedio { asentamiento } => {
                let plaza = asentamiento as u64;
                if let Some(f) = b.ticket.atacante.faccion.filter(|_| ganador == Lado::Atacante) {
                    if self.estado.asentamientos.get(&plaza).is_some_and(|a| a.faccion != f) {
                        let lucharon: Vec<(u64, Vec<u64>)> = self.participantes_del_lado(b, Lado::Defensor).iter().map(|p| (p.heroe, p.escuadras.iter().map(|e| e.id).collect())).collect();
                        let atacantes = self.columnas_del_bando(b, Lado::Atacante);
                        let robado = self.conquistar_plaza(t, plaza, f, atacantes.first().copied().unwrap_or(0), &lucharon);
                        // La mitad del carro de todos los que defendieron y perdieron es del vencedor, repartida entre sus columnas hasta donde quepa.
                        self.repartir_en_columnas(&atacantes, &robado);
                    }
                }
            }
            // Una avanzada atacada y ganada por el atacante cae como con números: se lleva el almacén que le quepa y queda destruida o en ruinas. Si ya no existe (su madre
            // cayó en ruinas durante la batalla) no queda nada que aplicar.
            &Contexto::Avanzada { avanzada, .. } if ganador == Lado::Atacante => {
                if matches!(self.avanzada(avanzada), Some((Some(_), _))) {
                    self.castigo_a_los_defensores(b);
                }
                let atacantes = self.columnas_del_bando(b, Lado::Atacante);
                self.cae_avanzada(t, avanzada, &atacantes);
            }
            Contexto::Avanzada { .. } => {}
            // Mundo abierto: el bando que pierde entrega la mitad de su carro, que se reparte entre las columnas del que gana, lo que quepa (contra una caravana o un
            // campamento esa mitad no se la lleva nadie).
            Contexto::CampoAbierto { .. } | Contexto::Caravana { .. } | Contexto::CampamentoBandidos { .. } => {
                let mut robado = Mercancia::new();
                for c in self.columnas_del_bando(b, perdedor) {
                    for (r, n) in self.perder_la_mitad_del_carro(c) {
                        sumar(&mut robado, r, n);
                    }
                }
                let ganadoras = self.columnas_del_bando(b, ganador);
                self.repartir_en_columnas(&ganadoras, &robado);
                if ganador == Lado::Atacante {
                    self.captura_de_batalla(t, b, efectos);
                }
            }
            &Contexto::SaqueoPueblo { pueblo, .. } if ganador == Lado::Atacante => {
                if let Some((madre, _)) = self.pueblo(pueblo) {
                    let cr = self.estado.reglas.economia.combate.clone();
                    let columnas = self.columnas_del_bando(b, Lado::Atacante);
                    let mut carros: Vec<CarroDeColumna> = columnas.iter().map(|&c| CarroDeColumna { carga: self.estado.ejercitos[&c].columna.carga.clone(), capacidad: self.capacidad_de_carga(c).unwrap_or(0) }).collect();
                    let almacen = &mut self.estado.asentamientos.get_mut(&madre).expect("existe").economia.pueblos.iter_mut().find(|p| p.id == pueblo).expect("existe").almacen;
                    combate::pueblo::saquear(&cr, almacen, &mut carros);
                    self.saquear_rebano(madre, pueblo, &mut carros);
                    for (&c, carro) in columnas.iter().zip(carros) {
                        self.estado.ejercitos.get_mut(&c).expect("existe").columna.carga = carro.carga;
                        self.tr.tocar(Clase::Ejercito, c);
                    }
                    self.tr.tocar(Clase::Asentamiento, madre);
                    self.castigo_a_los_defensores(b);
                }
            }
            Contexto::SaqueoPueblo { .. } => {}
        }
        self.recoger_chatarra(b, res);
    }

    /// Los defensores de un pueblo o de una avanzada han perdido sin que haya conquista: todos los que defendieron, residentes o visitantes amigos, pierden la mitad de su carro, que se
    /// reparte entre las columnas atacantes hasta donde quepa (Entrar con todo, 2026-10-22, tercera ronda).
    fn castigo_a_los_defensores(&mut self, b: &Batalla) {
        let defensores: Vec<u64> = self.participantes_del_lado(b, Lado::Defensor).iter().map(|p| p.heroe).collect();
        let robado = self.pierden_la_mitad_del_carro(&defensores);
        let atacantes = self.columnas_del_bando(b, Lado::Atacante);
        self.repartir_en_columnas(&atacantes, &robado);
    }

    /// El atacante gana en mundo abierto: la caravana cae (sus carros, su carga y su escolta) o el campamento de bandidos desaparece y paga su oro.
    fn captura_de_batalla(&mut self, t: Instante, b: &Batalla, efectos: &Efectos) {
        let atacantes = self.columnas_del_bando(b, Lado::Atacante);
        match &b.ticket.contexto {
            Contexto::Caravana { caravanas, modalidad, .. } => {
                // Cada caravana en juego cae por su cuenta: pierde los carros que dice Unity (y desaparece si se queda sin ninguno) y su escolta cedida queda a 0.
                let mut perdida = Mercancia::new();
                for caravana in caravanas.iter().map(|k| k.id) {
                    if self.estado.fundacion.caravanas.contains_key(&caravana) {
                        // La Caravana de Fundación (2026-10-24): sin carros, cae según el tipo (Combate §7.1).
                        let (perdido, _) = self.caer_fundacion(t, caravana, *modalidad);
                        perdido.into_iter().for_each(|(r, n)| sumar(&mut perdida, r, n));
                        continue;
                    }
                    let Some(k) = self.estado.comercio.caravana(caravana) else { continue };
                    // La escolta cae con ella: queda a 0 (Doc 5.15.5) y `caer_caravana` la devuelve al campamento de su héroe.
                    let escolta: Vec<u64> = k.escolta.iter().map(|c| c.id).collect();
                    for id in escolta {
                        if let Some(s) = self.escuadra_mut(id) {
                            s.cantidad = 0;
                        }
                    }
                    let carros: Vec<usize> = efectos.carros_perdidos.iter().filter(|&&(k, _)| k == caravana).map(|&(_, i)| i).collect();
                    for (r, n) in self.caer_caravana(t, caravana, *modalidad, Some(&carros)).carga_perdida {
                        sumar(&mut perdida, r, n);
                    }
                }
                self.repartir_en_columnas(&atacantes, &perdida);
            }
            &Contexto::CampamentoBandidos { campamento, .. } => {
                let campamento = u64::from(campamento);
                let Some((nivel, origen)) = self.estado.combate.bandidos.get(&campamento).map(|c| (c.nivel, c.origen)) else { return };
                let mut oro = Mercancia::new();
                for heroe in self.participantes_del_lado(b, Lado::Atacante).iter().map(|p| p.heroe) {
                    let ganado = self.oro_de_bandidos_para(heroe, origen, nivel, t);
                    sumar(&mut oro, asentamiento::Recurso::Oro, ganado);
                }
                self.repartir_en_columnas(&atacantes, &oro);
                self.destruir_bandidos(campamento, t);
                self.experiencia_de_bandidos(t, b.ticket.atacante.faccion);
            }
            _ => {}
        }
    }

    /// Los participantes de un bando que no han salido (los del ticket y las incorporaciones).
    fn participantes_del_lado<'a>(&self, b: &'a Batalla, lado: Lado) -> Vec<&'a Participante> {
        let t = &b.ticket;
        t.bando(lado).participantes.iter().chain(b.incorporaciones.iter().filter(|i| i.lado == lado).map(|i| &i.participante)).filter(|p| !b.salio(p.heroe)).collect()
    }

    /// La chatarra de las bajas (Doc 4.2.1): el equipo de **todos** los caídos, de los dos bandos, se lo queda quien gana, a los carros de sus columnas (lo que
    /// quepa); si defiende una plaza y gana, a su almacén. Sin columna que la lleve (caravana o campamento vencedores) se pierde.
    fn recoger_chatarra(&mut self, b: &Batalla, res: &ResultadoDeBatalla) {
        let cr = self.estado.reglas.economia.combate.clone();
        let mut chatarra = Mercancia::new();
        for e in &res.por_escuadra {
            let Some(s) = b.escuadras_de().find(|s| s.id == e.escuadra) else { continue };
            for (r, n) in cr.chatarra.de_muertos(s.tropa, e.muertos) {
                sumar(&mut chatarra, r, n);
            }
        }
        // La chatarra se suma exacta y queda en unidades enteras, hacia abajo (C3b): la fracción no la coge nadie.
        chatarra.values_mut().for_each(|n| *n = base::entero(*n));
        chatarra.retain(|_, n| *n > 0);
        if chatarra.is_empty() {
            return;
        }
        // Si defiende una plaza o una avanzada y gana, la chatarra va al almacén de la plaza (la madre, en una avanzada).
        let plaza = match (&b.ticket.contexto, res.ganador) {
            (&Contexto::Asedio { asentamiento }, Lado::Defensor) => Some(asentamiento as u64),
            (&Contexto::Avanzada { avanzada, .. }, Lado::Defensor) => self.avanzada(avanzada).and_then(|(madre, _)| madre),
            _ => None,
        };
        if let Some(plaza) = plaza {
            if let Some(a) = self.estado.asentamientos.get_mut(&plaza) {
                chatarra.iter().for_each(|(&r, &n)| {
                    a.economia.almacen.agregar(r, n);
                });
                self.tr.tocar(Clase::Asentamiento, plaza);
            }
            return;
        }
        let ganadoras = self.columnas_del_bando(b, res.ganador);
        self.repartir_en_columnas(&ganadoras, &chatarra);
    }
}

/// Los tokens de una asignación en un orden que no depende de cuándo llegó cada uno (para saber si un mensaje cambió algo).
fn ordenados(tokens: Option<Vec<Token>>) -> Vec<Token> {
    let mut v = tokens.unwrap_or_default();
    v.sort_by_key(|t| t.heroe);
    v
}

/// El histórico de batallas de la partida (§3.3), en la carpeta del núcleo persistente: un JSON por línea, `{ gameId, archivadaMs, batalla }`.
pub const HISTORICO_DE_BATALLAS: &str = "batallas.jsonl";

fn lado_json(l: Lado) -> &'static str {
    match l {
        Lado::Atacante => "atacante",
        Lado::Defensor => "defensor",
    }
}

impl Nucleo {
    /// Una batalla retirada: con persistencia se anexa al histórico (paridad P4, `<gameId>.batallas.jsonl` del TypeScript); sin ella se guarda para `tomar_batallas_retiradas`. La repetición
    /// del registro al recuperar puede anexar otra vez la misma: quien lo lee descarta las repetidas por `batalla.id`, como el TypeScript.
    fn archivar_batalla(&mut self, b: Batalla) {
        let Some(dir) = self.persist.as_ref().map(|p| p.dir.clone()) else {
            self.retiradas.push(b);
            return;
        };
        let linea = format!("{}\n", self.historico_de_batalla(&b));
        let anexar = || -> std::io::Result<()> {
            use std::io::Write;
            std::fs::OpenOptions::new().create(true).append(true).open(dir.join(HISTORICO_DE_BATALLAS))?.write_all(linea.as_bytes())
        };
        if let Err(e) = anexar() {
            eprintln!("nucleo: no se pudo anexar la batalla {} al histórico ({e})", b.id);
        }
    }

    /// La línea del histórico: la batalla entera, legible (ids del contrato, instantes de mundo en ms).
    pub fn historico_de_batalla(&self, b: &Batalla) -> serde_json::Value {
        use serde_json::json;
        let escuadras = |v: &[combate::ciclo::EscuadraEnBatalla]| v.iter().map(|s| json!({ "id": s.id, "tropa": format!("{:?}", s.tropa), "efectivos": s.efectivos, "dueno": s.dueno, "nivel": s.nivel })).collect::<Vec<_>>();
        let participante = |p: &combate::ciclo::Participante| json!({ "heroe": p.heroe, "faccion": p.faccion, "controlador": format!("{:?}", p.controlador), "escuadras": escuadras(&p.escuadras) });
        let bando = |x: &combate::ciclo::BandoDeTicket| json!({ "faccion": x.faccion, "capacidad": x.capacidad, "participantes": x.participantes.iter().map(participante).collect::<Vec<_>>(), "sinHeroe": escuadras(&x.sin_heroe) });
        let resultado = b.resultado.as_ref().map(|r| {
            json!({
                "resultadoId": r.resultado_id, "revision": r.revision, "intento": r.intento, "inicio": r.inicio, "fin": r.fin, "ganador": lado_json(r.ganador), "razon": format!("{:?}", r.razon),
                "objetivos": r.objetivos.iter().map(|(o, l)| json!({ "objetivo": o, "lado": l.map(lado_json) })).collect::<Vec<_>>(),
                "porEscuadra": r.por_escuadra.iter().map(|s| json!({ "escuadra": s.escuadra, "desplegados": s.desplegados, "supervivientes": s.supervivientes_al_cierre, "muertos": s.muertos, "xp": s.xp_ganada })).collect::<Vec<_>>(),
                "porHeroe": r.por_heroe.iter().map(|h| json!({ "heroe": h.heroe, "participo": h.participo, "sobrevivio": h.sobrevivio_al_cierre, "xp": h.xp_ganada, "derrotado": h.derrotado })).collect::<Vec<_>>(),
                "versionServidor": r.version_servidor, "carrosPerdidos": r.carros_perdidos,
            })
        });
        let contexto: serde_json::Map<String, serde_json::Value> = objetivo_de_contexto(&b.ticket.contexto).into_iter().map(|(k, v)| (k.to_string(), json!(v))).chain([("tipo".to_string(), json!(numero_de_contexto(&b.ticket.contexto)))]).collect();
        json!({
            "gameId": self.id_de_partida(),
            "archivadaMs": acceso_ahora_ms(),
            "batalla": {
                "id": b.id, "battleId": self.uuid_de_batalla(b.id), "estado": format!("{:?}", b.estado), "iniciadaPor": b.iniciada_por, "punto": { "x": b.punto.x, "y": b.punto.y },
                "contexto": contexto, "revision": b.ticket.revision, "duracionMaxima": b.ticket.duracion_maxima,
                "atacante": bando(&b.ticket.atacante), "defensor": bando(&b.ticket.defensor),
                "incorporaciones": b.incorporaciones.iter().map(|i| json!({ "secuencia": i.secuencia, "lado": lado_json(i.lado), "participante": participante(&i.participante) })).collect::<Vec<_>>(),
                "salidas": b.salidas.iter().map(|s| json!({ "heroe": s.heroe, "motivo": format!("{:?}", s.motivo), "derrotado": s.derrotado, "en": s.en })).collect::<Vec<_>>(),
                "expiraEn": b.expira_en, "convocatoriaHasta": b.convocatoria_hasta, "iniciadaEn": b.iniciada_en, "aplicadaEn": b.aplicada_en, "cerradaEn": b.cerrada_en,
                "asignacion": b.asignacion.as_ref().map(|a| json!({ "intento": a.intento, "instancia": { "host": a.instancia.host, "puerto": a.instancia.puerto, "protocolo": a.instancia.protocolo } })),
                "resultado": resultado,
            }
        })
    }
}

/// El reloj de pared (ms desde 1970): solo para fechar el archivo, nunca entra en el estado.
fn acceso_ahora_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}
