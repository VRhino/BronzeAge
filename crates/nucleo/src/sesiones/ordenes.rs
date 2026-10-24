//! Las órdenes del jugador del contrato 05 (`orders.proto`) como comandos del núcleo, con sus ids de texto (`<prefijo>-<n>`, `proyeccion::ids`), y la orden genérica de administración.
//!
//! - La **autorización** no es de aquí: la aplica el núcleo con el actor de la petición (el cliente) y su `admin`. Lo único que esta traducción decide por su cuenta es que la
//!   orden de administración (`OrdenDeAdministracion`) solo la da una sesión de administración (`no_autorizado` en cualquier otra).
//! - Lo que el contrato deja implícito se resuelve preguntando al mundo ([`Consulta`]): el héroe de la cuenta (`fundar`, `guardar_loadout`, `pasar_a_viveres`…), su columna, la plaza
//!   a la que se marcha, la plaza o avanzada donde repostar (la más cercana a la puerta si no viene `lugar_id`), el campamento donde está, el pueblo o la avanzada de una obra y la
//!   batalla de un UUID.
//! - Un id que no tiene la forma de su entidad se rechaza con el código de «desconocida» de esa entidad (`heroe.desconocido`…); un valor fuera de un catálogo cerrado (recurso, tropa,
//!   cargo, política, tipo de carro…), con `orden.forma_invalida`. Lo demás (que la entidad exista, las cantidades) lo valida el núcleo.
//! - Las órdenes del contrato que el núcleo aún no tiene (la política salvo la Facción, la ciudadanía y la residencia de 5B, tecnología, Aedas, refundación, caravanas a mano y almacén personal) se rechazan con
//!   `orden.no_disponible`; ninguna se descarta en silencio. El `match` sobre `Cuerpo` no tiene comodín: una orden nueva del contrato no compila hasta clasificarla.
//!
//! La tabla orden ↔ comando, con lo que cada una ignora, está en `Docs/Mechanics/Comandos_Parametros.md`.

use crate::comando::{rechazos, AccionReloj, Comando, ObjetivoAtaque, OrigenIntel, PagarCon, Peticion, Respuesta};
use crate::estado_militar::Relacion;
use crate::nucleo::Nucleo;
use crate::proyeccion::ids;
use asentamiento::{Cargo, Recurso, Tipo};
use base::{Mil, Punto};
use combate::resolucion::Lado;
use comercio::caravana::{Animal, TipoCarro};
use contrato::v1 as pb;
use militar::catalogo::Tropa;
use militar::columna::Politica;
use militar::heroe::{Controlador, Residencia};
use militar::inspeccion::Objetivo;
use militar::presencia::Ubicacion;
use pb::orden::Cuerpo;
use prost_types::{value::Kind, Struct};

/// Lo que es una obra de mapa del contrato (`obra-N`): el pueblo o la avanzada que se levanta, se mejora o se repara.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Obra {
    Pueblo,
    Avanzada,
}

/// Lo que la traducción pregunta al mundo para resolver lo que la orden deja implícito. Lo implementa [`Nucleo`]; las pruebas, un mundo de mentira.
pub trait Consulta {
    /// La versión del mundo, para el rechazo.
    fn version(&self) -> u64;
    /// El héroe de una cuenta (uno por cuenta y partida; no cuenta el de campo de la administración).
    fn heroe_de_cuenta(&self, cuenta: &str) -> Option<u64>;
    /// La columna del héroe.
    fn columna_de(&self, heroe: u64) -> Option<u64>;
    /// Dónde está la plaza: la posición donde termina una marcha «a un asentamiento» (queda dentro de la puerta).
    fn posicion_de_plaza(&self, plaza: u64) -> Option<Punto>;
    /// La plaza o la avanzada operativa más cercana a la puerta de la columna del héroe (`Ubicacion::Asentamiento` o `Ubicacion::Avanzada`).
    fn lugar_de_repostar(&self, heroe: u64) -> Option<Ubicacion>;
    /// El campamento de mercenarios donde está el héroe, dentro o a su puerta.
    fn campamento_del_heroe(&self, heroe: u64) -> Option<u32>;
    /// Si `id` es un pueblo, una avanzada o ninguno.
    fn obra(&self, id: u64) -> Option<Obra>;
    /// El id del núcleo de una batalla por su UUID.
    fn batalla(&self, uuid: &str) -> Option<u64>;
}

impl Consulta for Nucleo {
    fn version(&self) -> u64 {
        Nucleo::version(self)
    }
    fn heroe_de_cuenta(&self, cuenta: &str) -> Option<u64> {
        Nucleo::heroe_de_cuenta(self, cuenta).map(|h| h.heroe.id)
    }
    fn columna_de(&self, heroe: u64) -> Option<u64> {
        self.estado().ejercito_de_heroe(heroe)
    }
    fn posicion_de_plaza(&self, plaza: u64) -> Option<Punto> {
        self.estado().asentamientos.get(&plaza).map(|a| a.posicion)
    }
    fn lugar_de_repostar(&self, heroe: u64) -> Option<Ubicacion> {
        Nucleo::lugar_de_repostar(self, heroe)
    }
    fn campamento_del_heroe(&self, heroe: u64) -> Option<u32> {
        Nucleo::campamento_del_heroe(self, heroe)
    }
    fn obra(&self, id: u64) -> Option<Obra> {
        self.pueblo(id).map(|_| Obra::Pueblo).or_else(|| self.avanzada(id).map(|_| Obra::Avanzada))
    }
    fn batalla(&self, uuid: &str) -> Option<u64> {
        self.id_de_batalla(uuid)
    }
}

/// El sigilo de `crear_faccion`, pieza a pieza: que cada una sea del catálogo lo mira el núcleo (`faccion.sigilo_invalido`).
fn sigilo(s: &pb::orden_crear_faccion::Sigilo) -> politica::sigilo::Sigilo {
    politica::sigilo::Sigilo {
        forma: s.forma_id.clone(),
        campo: s.campo_id.clone(),
        emblema: s.emblema_id.clone(),
        color_primario: s.color_primario_id.clone(),
        color_secundario: s.color_secundario_id.clone(),
        color_emblema: s.color_emblema_id.clone(),
        orla: s.orla_id.clone(),
        color_orla: s.color_orla_id.clone(),
    }
}

/// De la orden del contrato a la petición del núcleo. `Err` = rechazo sin pasar por el núcleo (no deja rastro).
pub fn traducir(mundo: &dyn Consulta, orden: &pb::Orden, cliente: &str, admin: bool) -> Result<Peticion, Respuesta> {
    let x = Ctx { mundo, cliente };
    let Some(cuerpo) = &orden.cuerpo else { return Err(x.no(rechazos::ORDEN_NO_SOPORTADA, "orden vacía")) };
    let comando = x.comando(cuerpo, admin)?;
    Ok(Peticion { actor: cliente.into(), admin, peticion_id: orden.peticion_id.clone(), version_esperada: orden.version_esperada, comando })
}

type R<T> = Result<T, Respuesta>;

struct Ctx<'a> {
    mundo: &'a dyn Consulta,
    cliente: &'a str,
}

impl Ctx<'_> {
    fn no(&self, codigo: &'static str, texto: impl Into<String>) -> Respuesta {
        Respuesta::rechazo(codigo, texto, self.mundo.version())
    }

    fn forma<T>(&self, texto: impl Into<String>) -> R<T> {
        Err(self.no(rechazos::ORDEN_FORMA_INVALIDA, texto))
    }

    fn no_disponible<T>(&self, que: &str) -> R<T> {
        Err(self.no(rechazos::ORDEN_NO_DISPONIBLE, format!("esta orden aún no existe en el servidor: {que}")))
    }

    fn id<T>(&self, parse: fn(&str) -> Option<T>, codigo: &'static str, texto: &str) -> R<T> {
        parse(texto).ok_or_else(|| self.no(codigo, texto))
    }

    // ───────────────────────────── Entidades ─────────────────────────────

    fn heroe(&self, s: &str) -> R<u64> {
        self.id(ids::parse_heroe, rechazos::HEROE_DESCONOCIDO, s)
    }
    fn ejercito(&self, s: &str) -> R<u64> {
        self.id(ids::parse_ejercito, rechazos::EJERCITO_DESCONOCIDO, s)
    }
    fn plaza(&self, s: &str) -> R<u64> {
        self.id(ids::parse_asentamiento, rechazos::ASENTAMIENTO_DESCONOCIDO, s)
    }
    fn caravana(&self, s: &str) -> R<u64> {
        self.id(ids::parse_caravana, rechazos::CARAVANA_DESCONOCIDA, s)
    }
    fn escuadra(&self, s: &str) -> R<u64> {
        self.id(ids::parse_escuadra, rechazos::ORDEN_FORMA_INVALIDA, s)
    }
    fn escuadras(&self, v: &[String]) -> R<Vec<u64>> {
        v.iter().map(|s| self.escuadra(s)).collect()
    }
    fn acuerdo(&self, s: &str) -> R<u64> {
        self.id(ids::parse_acuerdo, rechazos::TRUEQUE_DESCONOCIDO, s)
    }
    fn orden_de_mercado(&self, s: &str) -> R<u64> {
        self.id(ids::parse_orden_de_mercado, rechazos::ORDEN_DESCONOCIDA, s)
    }
    fn pueblo(&self, s: &str) -> R<u64> {
        self.id(ids::parse_pueblo, rechazos::PUEBLO_DESCONOCIDO, s)
    }
    fn avanzada(&self, s: &str) -> R<u64> {
        self.id(ids::parse_avanzada, rechazos::AVANZADA_DESCONOCIDA, s)
    }
    fn convocatoria(&self, s: &str) -> R<u64> {
        self.id(ids::parse_convocatoria, rechazos::CONVOCATORIA_DESCONOCIDA, s)
    }
    fn loadout(&self, s: &str) -> R<u64> {
        self.id(ids::parse_loadout, rechazos::LOADOUT_DESCONOCIDO, s)
    }
    fn campamento(&self, s: &str) -> R<u32> {
        self.id(ids::parse_campamento, rechazos::CAMPAMENTO_DESCONOCIDO, s)
    }
    fn bandidos(&self, s: &str) -> R<u64> {
        self.id(ids::parse_bandidos, rechazos::CAMPAMENTO_DE_BANDIDOS_DESCONOCIDO, s)
    }
    fn faccion(&self, s: &str) -> R<u32> {
        self.id(ids::parse_faccion, rechazos::ORDEN_FORMA_INVALIDA, s)
    }
    fn nodo(&self, s: &str) -> R<u32> {
        self.id(ids::parse_nodo, rechazos::PUEBLO_NODO_DESCONOCIDO, s)
    }
    fn batalla(&self, uuid: &str) -> R<u64> {
        self.mundo.batalla(uuid).ok_or_else(|| self.no(rechazos::BATALLA_NO_EXISTE, uuid))
    }

    /// `ed-<plaza>-<n>`: el edificio `n` de esa plaza.
    fn edificio(&self, plaza: u64, s: &str) -> R<u32> {
        match ids::parse_edificio(s) {
            Some((p, n)) if p == plaza => Ok(n),
            _ => Err(self.no(rechazos::EDIFICIO_DESCONOCIDO, s)),
        }
    }

    /// `recinto-<plaza>-<n>`: el recinto `n` de esa plaza.
    fn recinto(&self, plaza: u64, s: &str) -> R<u32> {
        match ids::parse_recinto(s) {
            Some((p, n)) if p == plaza => Ok(n),
            _ => Err(self.no(rechazos::MURALLAS_RECINTO_DESCONOCIDO, s)),
        }
    }

    /// `obra-N`: el pueblo o la avanzada de esa obra.
    fn obra(&self, s: &str) -> R<(Obra, u64)> {
        let id = self.id(ids::parse_obra, rechazos::OBRA_DE_MAPA_DESCONOCIDA, s)?;
        self.mundo.obra(id).map(|o| (o, id)).ok_or_else(|| self.no(rechazos::OBRA_DE_MAPA_DESCONOCIDA, s))
    }

    /// El héroe de la cuenta que da la orden: la del contrato no lo nombra porque solo hay uno.
    fn mi_heroe(&self) -> R<u64> {
        self.mundo.heroe_de_cuenta(self.cliente).ok_or_else(|| self.no(rechazos::HEROE_DESCONOCIDO, "la cuenta no tiene héroe"))
    }

    fn columna(&self, heroe: u64) -> R<u64> {
        self.mundo.columna_de(heroe).ok_or_else(|| self.no(rechazos::HEROE_SIN_COLUMNA, "el héroe no va en ninguna columna"))
    }

    // ───────────────────────────── Catálogos y cantidades ─────────────────────────────

    fn recurso(&self, s: &str) -> R<Recurso> {
        Recurso::desde_nombre(s).map_or_else(|| self.forma(format!("recurso desconocido: {s}")), Ok)
    }

    fn tropa(&self, s: &str) -> R<Tropa> {
        Tropa::desde_nombre(s).map_or_else(|| self.forma(format!("tropa desconocida: {s}")), Ok)
    }

    fn cargo(&self, s: &str) -> R<Cargo> {
        Cargo::desde_clave(s).map_or_else(|| self.forma(format!("cargo desconocido: {s}")), Ok)
    }

    fn politica(&self, s: &str) -> R<Politica> {
        match s {
            "rechazar" => Ok(Politica::Rechazar),
            "aceptar" => Ok(Politica::Aceptar),
            "preguntar" => Ok(Politica::Preguntar),
            _ => self.forma(format!("política de unión desconocida: {s}")),
        }
    }

    /// La política opcional de una salida: sin ella, nadie se une.
    fn politica_o_rechazar(&self, s: Option<&str>) -> R<Politica> {
        s.map_or(Ok(Politica::Rechazar), |s| self.politica(s))
    }

    fn lado(&self, s: &str) -> R<Lado> {
        match s {
            "atacante" => Ok(Lado::Atacante),
            "defensor" => Ok(Lado::Defensor),
            _ => self.forma(format!("lado desconocido: {s}")),
        }
    }

    fn cantidades(&self, c: Option<&pb::Cantidades>) -> R<Vec<(Recurso, Mil)>> {
        c.map_or(Ok(vec![]), |c| c.por_id.iter().map(|(r, n)| Ok((self.recurso(r)?, *n))).collect())
    }

    fn entero_u32(&self, n: i64, que: &str) -> R<u32> {
        u32::try_from(n).map_or_else(|_| self.forma(format!("{que} fuera de rango: {n}")), Ok)
    }

    fn punto(&self, p: Option<&pb::Punto>) -> R<Punto> {
        p.map_or_else(|| self.forma("falta la posición"), |p| Ok(Punto::new(p.x, p.y)))
    }

    /// La taberna de una compra de intel: `{tipo: asentamiento | campamento, id}`.
    fn origen_de_intel(&self, o: Option<(&str, &str)>) -> R<OrigenIntel> {
        match o {
            Some(("asentamiento", id)) => Ok(OrigenIntel::Plaza(self.plaza(id)?)),
            Some(("campamento", id)) => Ok(OrigenIntel::Campamento(self.campamento(id)?)),
            Some((otro, _)) => self.forma(format!("taberna desconocida: {otro}")),
            None => self.forma("falta la taberna (origen)"),
        }
    }

    // ───────────────────────────── El comando de cada orden ─────────────────────────────

    fn comando(&self, cuerpo: &Cuerpo, admin: bool) -> R<Comando> {
        Ok(match cuerpo {
            // ── Presencia y héroe ──
            Cuerpo::CrearHeroe(o) => {
                // DUDA: nombre, clase, género y avatar se ignoran hasta que Conquest publique su catálogo (decisión 4 de 4H): el núcleo no los guarda.
                Comando::CrearHeroe { cuenta: self.cliente.into(), campamento: self.campamento(&o.campamento_id)?, controlador: Controlador::Humano }
            }
            Cuerpo::ReclutarTropa(o) => Comando::ReclutarTropa { asentamiento: self.plaza(&o.asentamiento_id)?, heroe: self.heroe(&o.heroe_id)?, tropa: self.tropa(&o.tropa_id)? },
            Cuerpo::ReclutarEnCampamento(o) => {
                let pagar_con = match o.pagar_con.as_deref() {
                    None | Some("almacenPersonal") => PagarCon::AlmacenPersonal,
                    Some("carro") => PagarCon::Carro,
                    Some(otro) => return self.forma(format!("forma de pago desconocida: {otro}")),
                };
                Comando::ReclutarEnCampamento { heroe: self.mi_heroe()?, tropa: self.tropa(&o.tropa_id)?, pagar_con }
            }
            Cuerpo::PedirPrestamo(o) => Comando::PedirPrestamo { heroe: self.mi_heroe()?, tropas: o.tropa_ids.iter().map(|t| self.tropa(t)).collect::<R<_>>()? },
            Cuerpo::ReponerPrestamo(_) => Comando::ReponerPrestamo { heroe: self.mi_heroe()? },
            Cuerpo::GuardarLoadout(o) => Comando::GuardarLoadout {
                heroe: self.mi_heroe()?,
                loadout: o.loadout_id.as_deref().map(|s| self.loadout(s)).transpose()?,
                nombre: o.display_name.clone(),
                escuadras: self.escuadras(&o.squad_ids)?,
                perks: o.perks_seleccionados.iter().map(|&p| self.entero_u32(p, "perk")).collect::<R<_>>()?,
                activo: o.activo,
            },
            Cuerpo::BorrarLoadout(o) => Comando::BorrarLoadout { heroe: self.mi_heroe()?, loadout: self.loadout(&o.loadout_id)? },
            Cuerpo::RepartirPuntos(o) => {
                let a = o.atributos.unwrap_or_default();
                Comando::RepartirPuntos { heroe: self.mi_heroe()?, fuerza: a.fuerza.unwrap_or(0), destreza: a.destreza.unwrap_or(0), armadura: a.armadura.unwrap_or(0), vitalidad: a.vitalidad.unwrap_or(0) }
            }
            Cuerpo::OrdenarEscuadras(o) => Comando::OrdenarEscuadras { heroe: self.mi_heroe()?, escuadras: self.escuadras(&o.escuadron_ids)? },
            Cuerpo::AsignarGuarnicion(o) => Comando::AsignarGuarnicion { heroe: self.mi_heroe()?, escuadra: self.escuadra(&o.squad_id)? },
            Cuerpo::RetirarGuarnicion(o) => Comando::RetirarGuarnicion { heroe: self.mi_heroe()?, escuadra: self.escuadra(&o.squad_id)? },
            Cuerpo::PasarAViveres(o) => Comando::PasarAViveres { heroe: self.mi_heroe()?, cantidad: o.cantidad },
            Cuerpo::AsignarEscolta(o) => Comando::AsignarEscolta { caravana: self.caravana(&o.caravana_id)?, heroe: self.heroe(&o.heroe_id)?, escuadras: self.escuadras(&o.escuadron_ids)? },
            Cuerpo::QuitarEscolta(o) => {
                // Sin escuadrones, retira todas las que cedió.
                let escuadras = if o.escuadron_ids.is_empty() { None } else { Some(self.escuadras(&o.escuadron_ids)?) };
                Comando::QuitarEscolta { caravana: self.caravana(&o.caravana_id)?, heroe: self.heroe(&o.heroe_id)?, escuadras }
            }
            Cuerpo::Desconectarse(o) => Comando::Desconectarse { heroe: self.heroe(&o.heroe_id)? },
            Cuerpo::Conectarse(o) => Comando::Conectarse { heroe: self.heroe(&o.heroe_id)? },
            Cuerpo::ComprarEnCampamento(o) => {
                let heroe = self.mi_heroe()?;
                let campamento = self.campamento_o_el_del(heroe, o.campamento_id.as_deref())?;
                Comando::ComprarEnCampamento { heroe, campamento, recurso: self.recurso(&o.recurso)?, cantidad: o.cantidad }
            }
            Cuerpo::RepostarEnCampamento(o) => {
                let heroe = self.mi_heroe()?;
                Comando::RepostarEnCampamento { heroe, campamento: self.campamento_o_el_del(heroe, o.campamento_id.as_deref())? }
            }

            // ── Columnas y convocatorias ──
            Cuerpo::MarcharA(o) => {
                use pb::orden_marchar_a::objetivo::Tipo;
                let heroe = self.heroe(&o.heroe_id)?;
                // Marchar a un asentamiento es marchar a su plaza: queda dentro del `radioPuerta`.
                let destino = match o.objetivo.as_ref().and_then(|x| x.tipo.as_ref()) {
                    Some(Tipo::Punto(p)) => Punto::new(p.x, p.y),
                    Some(Tipo::AsentamientoId(s)) => self.mundo.posicion_de_plaza(self.plaza(s)?).ok_or_else(|| self.no(rechazos::ASENTAMIENTO_DESCONOCIDO, s.as_str()))?,
                    None => return self.forma("la marcha no trae objetivo"),
                };
                Comando::MarcharA { ejercito: self.columna(heroe)?, destino }
            }
            Cuerpo::EstacionarEjercito(o) => Comando::Estacionar { ejercito: self.ejercito(&o.ejercito_id)? },
            Cuerpo::ControlarReloj(c) => Comando::Reloj(match (c.accion.as_str(), c.factor_milesimas, c.hasta_ms) {
                ("pausar", ..) => AccionReloj::Pausar,
                ("reanudar", ..) => AccionReloj::Reanudar,
                ("fijar_factor", Some(f), _) => match u32::try_from(f) {
                    Ok(f) => AccionReloj::FijarFactor(f),
                    Err(_) => return Err(self.no(rechazos::RELOJ_INVALIDO, "factor fuera de rango")),
                },
                ("avanzar_hasta", _, Some(h)) => AccionReloj::AvanzarHasta(h),
                _ => return Err(self.no(rechazos::RELOJ_INVALIDO, "acción de reloj desconocida o incompleta")),
            }),
            Cuerpo::Fundar(_) => Comando::Fundar { ejercito: self.columna(self.mi_heroe()?)? },
            Cuerpo::SalirAlMundo(o) => Comando::SalirAlMundo { asentamiento: self.plaza(&o.asentamiento_id)?, heroe: self.heroe(&o.heroe_id)?, escuadras: self.escuadras(&o.escuadron_ids)?, carga: self.cantidades(o.carga.as_ref())? },
            Cuerpo::MovilizarEjercito(o) => {
                // El ejército sale sin destino (Presencia_Y_Convocatoria): el `objetivo` de la orden se rechaza en vez de ignorarse; después, `marchar_a`.
                if o.objetivo.is_some() {
                    return Err(self.no(rechazos::OBJETIVO_NO_SOPORTADO, "el ejército sale sin destino: después, marchar_a"));
                }
                Comando::MovilizarEjercito {
                    asentamiento: self.plaza(&o.asentamiento_id)?,
                    heroe: self.heroe(&o.heroe_id)?,
                    escuadras: self.escuadras(&o.escuadron_ids)?,
                    carga: self.cantidades(o.carga.as_ref())?,
                    politica: self.politica_o_rechazar(o.politica_de_union.as_deref())?,
                }
            }
            Cuerpo::SalirDelCampamento(o) => {
                if o.objetivo.is_some() {
                    return Err(self.no(rechazos::OBJETIVO_NO_SOPORTADO, "la columna sale sin destino: después, marchar_a"));
                }
                Comando::SalirDelCampamento {
                    campamento: self.campamento(&o.campamento_id)?,
                    heroe: self.heroe(&o.heroe_id)?,
                    escuadras: self.escuadras(&o.escuadron_ids)?,
                    carga: self.cantidades(o.carga.as_ref())?,
                    politica: self.politica_o_rechazar(o.politica_de_union.as_deref())?,
                }
            }
            Cuerpo::EntrarEnAsentamiento(o) => Comando::EntrarEnAsentamiento { asentamiento: self.plaza(&o.asentamiento_id)?, heroe: self.heroe(&o.heroe_id)? },
            Cuerpo::SalirDeAsentamiento(o) => Comando::SalirDeAsentamiento { asentamiento: self.plaza(&o.asentamiento_id)?, heroe: self.heroe(&o.heroe_id)? },
            Cuerpo::Guarnecer(o) => Comando::Guarnecer { asentamiento: self.plaza(&o.asentamiento_id)?, heroe: self.heroe(&o.heroe_id)? },
            Cuerpo::EntrarEnCampamento(o) => Comando::EntrarEnCampamento { campamento: self.campamento(&o.campamento_id)?, heroe: self.heroe(&o.heroe_id)? },
            // `OrdenUnirseAEjercito` no trae carga (la del TypeScript tampoco): se une sin sacarla de su plaza.
            Cuerpo::UnirseAEjercito(o) => {
                Comando::UnirseAEjercito { asentamiento: self.plaza(&o.asentamiento_id)?, heroe: self.heroe(&o.heroe_id)?, ejercito: self.ejercito(&o.ejercito_id)?, escuadras: self.escuadras(&o.escuadron_ids)?, carga: vec![] }
            }
            Cuerpo::UnirseEnCampo(o) => Comando::UnirseEnCampo { heroe: self.heroe(&o.heroe_id)?, ejercito: self.ejercito(&o.ejercito_id)? },
            Cuerpo::ResponderPeticionDeUnion(o) => {
                Comando::ResponderPeticionDeUnion { heroe: self.heroe(&o.heroe_id)?, ejercito: self.ejercito(&o.ejercito_id)?, solicitante: self.heroe(&o.solicitante_id)?, aceptar: o.aceptar }
            }
            Cuerpo::SepararseDelEjercito(o) => Comando::SepararseDelEjercito { heroe: self.heroe(&o.heroe_id)? },
            Cuerpo::CederLiderazgo(o) => Comando::CederLiderazgo { heroe: self.heroe(&o.heroe_id)?, sucesor: self.heroe(&o.sucesor_id)? },
            Cuerpo::OrganizarEjercito(o) => Comando::OrganizarEjercito { heroe: self.heroe(&o.heroe_id)?, politica: self.politica(&o.politica_de_union)? },
            Cuerpo::CancelarFormacion(o) => Comando::CancelarFormacion { heroe: self.heroe(&o.heroe_id)? },
            Cuerpo::Repostar(o) => {
                let heroe = self.heroe(&o.heroe_id)?;
                match o.lugar_id.as_deref() {
                    Some(s) if ids::parse_avanzada(s).is_some() => Comando::RepostarEnAvanzada { heroe, avanzada: self.avanzada(s)? },
                    Some(s) => Comando::Repostar { heroe, asentamiento: self.plaza(s)? },
                    None => match self.mundo.lugar_de_repostar(heroe) {
                        Some(Ubicacion::Asentamiento(a)) => Comando::Repostar { heroe, asentamiento: u64::from(a) },
                        Some(Ubicacion::Avanzada(avanzada)) => Comando::RepostarEnAvanzada { heroe, avanzada },
                        _ => return Err(self.no(rechazos::REPOSTAR_SIN_LUGAR, "no hay ninguna plaza ni avanzada operativa a la puerta de la columna")),
                    },
                }
            }
            Cuerpo::ConvocarEjercito(o) => {
                Comando::ConvocarEjercito { heroe: self.heroe(&o.heroe_id)?, escuadras: self.escuadras(&o.escuadron_ids)?, carga: self.cantidades(o.carga.as_ref())?, politica: self.politica(&o.politica_de_union)? }
            }
            Cuerpo::UnirseAConvocatoria(o) => {
                Comando::UnirseAConvocatoria { heroe: self.heroe(&o.heroe_id)?, convocatoria: self.convocatoria(&o.convocatoria_id)?, escuadras: self.escuadras(&o.escuadron_ids)?, carga: self.cantidades(o.carga.as_ref())? }
            }
            Cuerpo::ResponderPeticionDeConvocatoria(o) => Comando::ResponderPeticionDeConvocatoria {
                heroe: self.heroe(&o.heroe_id)?,
                convocatoria: self.convocatoria(&o.convocatoria_id)?,
                solicitante: self.heroe(&o.solicitante_id)?,
                aceptar: o.aceptar,
            },
            Cuerpo::CambiarSeleccionDeConvocatoria(o) => Comando::CambiarSeleccionDeConvocatoria { heroe: self.heroe(&o.heroe_id)?, escuadras: self.escuadras(&o.escuadron_ids)?, carga: self.cantidades(o.carga.as_ref())? },
            Cuerpo::SepararseDeConvocatoria(o) => Comando::SepararseDeConvocatoria { heroe: self.heroe(&o.heroe_id)? },
            Cuerpo::CancelarConvocatoria(o) => Comando::CancelarConvocatoria { heroe: self.heroe(&o.heroe_id)? },
            Cuerpo::PartirConvocatoria(o) => Comando::PartirConvocatoria { heroe: self.heroe(&o.heroe_id)? },

            // ── Encuentros, combate y batalla ──
            Cuerpo::Perseguir(o) => {
                use pb::orden_perseguir::objetivo::Tipo;
                let objetivo = match o.objetivo.as_ref().and_then(|x| x.tipo.as_ref()) {
                    Some(Tipo::EjercitoId(s)) => Objetivo::Ejercito(self.ejercito(s)?),
                    Some(Tipo::CaravanaId(s)) => Objetivo::Caravana(self.caravana(s)?),
                    None => return self.forma("la persecución no trae objetivo"),
                };
                Comando::Perseguir { heroe: self.heroe(&o.heroe_id)?, objetivo }
            }
            Cuerpo::DejarDePerseguir(o) => Comando::DejarDePerseguir { heroe: self.heroe(&o.heroe_id)? },
            Cuerpo::Inspeccionar(o) => {
                use pb::orden_inspeccionar::objetivo::Tipo;
                let objetivo = match o.objetivo.as_ref().and_then(|x| x.tipo.as_ref()) {
                    Some(Tipo::EjercitoId(s)) => Objetivo::Ejercito(self.ejercito(s)?),
                    Some(Tipo::CaravanaId(s)) => Objetivo::Caravana(self.caravana(s)?),
                    Some(Tipo::AsentamientoId(s)) => Objetivo::Asentamiento(self.plaza(s)?),
                    None => return self.forma("la inspección no trae objetivo"),
                };
                Comando::Inspeccionar { heroe: self.heroe(&o.heroe_id)?, objetivo }
            }
            Cuerpo::Atacar(o) => {
                use pb::orden_atacar::objetivo::Tipo;
                let objetivo = match o.objetivo.as_ref().and_then(|x| x.tipo.as_ref()) {
                    Some(Tipo::EjercitoId(s)) => ObjetivoAtaque::Ejercito(self.ejercito(s)?),
                    Some(Tipo::CaravanaId(s)) => ObjetivoAtaque::Caravana(self.caravana(s)?),
                    Some(Tipo::CampamentoId(s)) => ObjetivoAtaque::Campamento(self.bandidos(s)?),
                    Some(Tipo::AsentamientoId(s)) => ObjetivoAtaque::Asentamiento(self.plaza(s)?),
                    Some(Tipo::PuebloId(s)) => ObjetivoAtaque::Pueblo(self.pueblo(s)?),
                    Some(Tipo::AvanzadaId(s)) => ObjetivoAtaque::Avanzada(self.avanzada(s)?),
                    None => return self.forma("el ataque no trae objetivo"),
                };
                Comando::Atacar { heroe: self.heroe(&o.heroe_id)?, objetivo, declara_guerra: o.declara_guerra.unwrap_or(false) }
            }
            Cuerpo::UnirseABatalla(o) => Comando::UnirseABatalla { heroe: self.heroe(&o.heroe_id)?, batalla: self.batalla(&o.battle_id)?, lado: o.lado.as_deref().map(|l| self.lado(l)).transpose()? },
            Cuerpo::CancelarBatalla(o) => Comando::CancelarBatalla { heroe: self.mi_heroe()?, batalla: self.batalla(&o.battle_id)? },

            // ── Avanzadas ──
            Cuerpo::ColocarAvanzada(o) => Comando::ColocarAvanzada { asentamiento: self.plaza(&o.asentamiento_id)?, heroe: self.heroe(&o.heroe_id)?, posicion: self.punto(o.posicion.as_ref())? },
            Cuerpo::EntregarEnObra(o) => {
                // `materiales` se ignora: el núcleo entrega lo que falta, hasta lo que lleve la columna y sus caravanas adjuntas.
                let ejercito = self.columna(self.heroe(&o.heroe_id)?)?;
                match self.obra(&o.obra_id)? {
                    (Obra::Pueblo, pueblo) => Comando::EntregarMateriales { ejercito, pueblo },
                    (Obra::Avanzada, avanzada) => Comando::EntregarMaterialesAAvanzada { ejercito, avanzada },
                }
            }
            Cuerpo::CancelarObra(o) => {
                let heroe = self.heroe(&o.heroe_id)?;
                match self.obra(&o.obra_id)? {
                    (Obra::Avanzada, avanzada) => Comando::CancelarObraDeAvanzada { heroe, avanzada },
                    // La obra de un pueblo ya tiene su orden (`destruir_pueblo`, que además lo derriba una vez levantado): aquí se rechaza en vez de duplicarla.
                    (Obra::Pueblo, _) => return Err(self.no(rechazos::OBJETIVO_NO_SOPORTADO, "la obra de un pueblo se cancela con destruir_pueblo")),
                }
            }
            Cuerpo::MejorarAvanzada(o) => Comando::MejorarAvanzada { heroe: self.heroe(&o.heroe_id)?, avanzada: self.avanzada(&o.avanzada_id)? },
            Cuerpo::RepararAvanzada(o) => Comando::RepararAvanzada { heroe: self.heroe(&o.heroe_id)?, avanzada: self.avanzada(&o.avanzada_id)? },
            Cuerpo::DestruirAvanzada(o) => Comando::DestruirAvanzada { heroe: self.heroe(&o.heroe_id)?, avanzada: self.avanzada(&o.avanzada_id)? },
            Cuerpo::ElegirTropaDeAvanzada(o) => Comando::ElegirTropaDeAvanzada { heroe: self.heroe(&o.heroe_id)?, avanzada: self.avanzada(&o.avanzada_id)?, tropa: self.tropa(&o.tropa_id)? },
            Cuerpo::GuarnecerAvanzada(o) => Comando::GuarnecerAvanzada { heroe: self.heroe(&o.heroe_id)?, avanzada: self.avanzada(&o.avanzada_id)?, escuadras: self.escuadras(&o.escuadron_ids)? },
            Cuerpo::RetirarDeAvanzada(o) => Comando::RetirarDeAvanzada { heroe: self.heroe(&o.heroe_id)?, avanzada: self.avanzada(&o.avanzada_id)?, escuadras: self.escuadras(&o.escuadron_ids)? },
            Cuerpo::DepositarEnAvanzada(o) => {
                Comando::DepositarEnAvanzada { ejercito: self.ejercito(&o.ejercito_id)?, avanzada: self.avanzada(&o.avanzada_id)?, recurso: self.recurso(&o.recurso)?, cantidad: o.cantidad }
            }
            Cuerpo::EntrarEnAvanzada(o) => Comando::EntrarEnAvanzada { avanzada: self.avanzada(&o.avanzada_id)?, heroe: self.heroe(&o.heroe_id)? },
            Cuerpo::SalirDeAvanzada(o) => Comando::SalirDeAvanzada { avanzada: self.avanzada(&o.avanzada_id)?, heroe: self.heroe(&o.heroe_id)? },

            // ── Acceso y murallas ──
            Cuerpo::FijarPuerta(o) => {
                // `cerrada_a` son los grupos a `false`; los vasallos van con los aliados y las Aedas no existen aún en el núcleo (cerrarles la puerta no cierra nada).
                if let Some(otro) = o.cerrada_a.iter().find(|g| !["neutrales", "aliados", "enemigos", "aedas"].contains(&g.as_str())) {
                    return self.forma(format!("grupo de la puerta desconocido: {otro}"));
                }
                let abierta = |grupo: &str| !o.cerrada_a.iter().any(|g| g == grupo);
                Comando::FijarAcceso { asentamiento: self.plaza(&o.asentamiento_id)?, aliados: abierta("aliados"), neutrales: abierta("neutrales"), enemigos: abierta("enemigos") }
            }
            Cuerpo::ComprometerRecinto(o) => {
                let nivel = o.nivel.parse::<u8>().map_or_else(|_| self.forma(format!("nivel de recinto desconocido: {}", o.nivel)), Ok)?;
                Comando::ComprometerRecinto { asentamiento: self.plaza(&o.asentamiento_id)?, cargo: self.cargo(&o.cargo)?, nivel }
            }
            Cuerpo::MejorarRecinto(o) => {
                let plaza = self.plaza(&o.asentamiento_id)?;
                Comando::MejorarRecinto { asentamiento: plaza, cargo: self.cargo(&o.cargo)?, recinto: self.recinto(plaza, &o.recinto_id)? }
            }
            Cuerpo::AbandonarRecinto(o) => {
                let plaza = self.plaza(&o.asentamiento_id)?;
                Comando::AbandonarRecinto { asentamiento: plaza, recinto: self.recinto(plaza, &o.recinto_id)? }
            }

            // ── Economía y construcción de una plaza ──
            // El `cargo` de las órdenes de obras (Gobernador o Maestro de Obras) se ignora: el núcleo autoriza con `manda_en`, y los cargos son del sistema 5.
            Cuerpo::AnadirEdificioManualmente(o) => {
                let tipo = Tipo::desde_nombre(&o.tipo).map_or_else(|| self.forma(format!("tipo de edificio desconocido: {}", o.tipo)), Ok)?;
                Comando::AnadirEdificio { asentamiento: self.plaza(&o.asentamiento_id)?, tipo }
            }
            Cuerpo::QuitarDeCola(o) => {
                let plaza = self.plaza(&o.asentamiento_id)?;
                Comando::QuitarDeCola { asentamiento: plaza, edificio: self.edificio(plaza, &o.edificio_id)? }
            }
            Cuerpo::MoverEnCola(o) => {
                let plaza = self.plaza(&o.asentamiento_id)?;
                let arriba = match o.direccion.as_str() {
                    "arriba" => true,
                    "abajo" => false,
                    otra => return self.forma(format!("dirección de la cola desconocida: {otra}")),
                };
                Comando::MoverEnCola { asentamiento: plaza, edificio: self.edificio(plaza, &o.edificio_id)?, arriba }
            }
            Cuerpo::MejorarEdificioAhora(o) => {
                let plaza = self.plaza(&o.asentamiento_id)?;
                Comando::MejorarEdificio { asentamiento: plaza, edificio: self.edificio(plaza, &o.edificio_id)? }
            }
            Cuerpo::AlternarAutoConstruccion(o) => Comando::AlternarAutoConstruccion { asentamiento: self.plaza(&o.asentamiento_id)?, pausada: o.pausada },
            Cuerpo::AlternarReceta(o) => Comando::AlternarReceta { asentamiento: self.plaza(&o.asentamiento_id)?, recurso: self.recurso(&o.recurso)?, pausada: o.pausada },
            Cuerpo::CalibrarReservaManual(o) => Comando::CalibrarReserva { asentamiento: self.plaza(&o.asentamiento_id)?, recurso: self.recurso(&o.recurso)?, valor: o.valor },
            Cuerpo::SolicitarAscenso(o) => Comando::SolicitarAscenso { asentamiento: self.plaza(&o.asentamiento_id)? },
            Cuerpo::ActivarPolitica(o) => Comando::ActivarPolitica { asentamiento: self.plaza(&o.asentamiento_id)?, politica: o.politica_id.clone(), cargo: self.cargo(&o.cargo)? },

            // ── Comercio ──
            Cuerpo::ProponerTrueque(o) => {
                let lineas_a = o.lineas_a.iter().map(|l| Ok((self.recurso(&l.recurso)?, l.cantidad))).collect::<R<_>>()?;
                let lineas_b = o.lineas_b.iter().map(|l| Ok((self.recurso(&l.recurso)?, l.cantidad))).collect::<R<_>>()?;
                Comando::ProponerTrueque { a: self.plaza(&o.asentamiento_a_id)?, b: self.plaza(&o.asentamiento_b_id)?, lineas_a, lineas_b }
            }
            Cuerpo::AceptarTrueque(o) => Comando::AceptarTrueque { acuerdo: self.acuerdo(&o.acuerdo_id)? },
            Cuerpo::RechazarTrueque(o) => Comando::RechazarTrueque { acuerdo: self.acuerdo(&o.acuerdo_id)? },
            Cuerpo::ColocarOrdenMercado(o) => {
                let venta = match o.tipo.as_str() {
                    "venta" => true,
                    "compra" => false,
                    otro => return self.forma(format!("tipo de orden de mercado desconocido: {otro}")),
                };
                Comando::ColocarOrdenMercado { asentamiento: self.plaza(&o.asentamiento_id)?, venta, recurso: self.recurso(&o.recurso)?, cantidad: o.cantidad, precio: o.precio }
            }
            Cuerpo::CancelarOrdenMercado(o) => Comando::CancelarOrdenMercado { orden: self.orden_de_mercado(&o.orden_id)? },
            Cuerpo::ComerciarEnPlaza(o) => Comando::ComerciarEnPlaza { ejercito: self.columna(self.heroe(&o.heroe_id)?)?, orden: self.orden_de_mercado(&o.orden_id)?, cantidad: o.cantidad },
            Cuerpo::CrearCaravana(o) => Comando::CrearCaravana { asentamiento: self.plaza(&o.asentamiento_id)? },
            Cuerpo::AgregarCarroCaravana(o) => {
                let tipo = match o.tipo_carro.as_str() {
                    "basico" => TipoCarro::Basico,
                    "reforzado" => TipoCarro::Reforzado,
                    otro => return self.forma(format!("tipo de carro desconocido: {otro}")),
                };
                Comando::AnadirCarro { caravana: self.caravana(&o.caravana_id)?, tipo }
            }
            Cuerpo::ComprarAnimalCaravana(o) => {
                let animal = match o.tipo_animal.as_str() {
                    "buey" => Animal::Buey,
                    "caballo" => Animal::Caballo,
                    "camello" => Animal::Camello,
                    otro => return self.forma(format!("animal desconocido: {otro}")),
                };
                Comando::AnadirAnimal { caravana: self.caravana(&o.caravana_id)?, carro: self.entero_u32(o.carro_indice, "índice de carro")?, animal }
            }
            Cuerpo::MoverCarroCaravana(o) => {
                Comando::MoverCarro { de: self.caravana(&o.desde_caravana_id)?, a: self.caravana(&o.hacia_caravana_id)?, carro: self.entero_u32(o.carro_indice, "índice de carro")? }
            }
            Cuerpo::ReservarCaravana(o) => Comando::ReservarCaravana { caravana: self.caravana(&o.caravana_id)?, reservada: o.reservada },
            Cuerpo::PrepararCaravana(o) => {
                // La escolta se cede antes (`asignar_escolta`): ceder en el mismo lanzamiento no existe (Heroe_Y_Tropa).
                if !o.escolta_escuadron_ids.is_empty() {
                    return Err(self.no(rechazos::CARAVANA_ESCOLTA_INVALIDA, "la escolta se cede antes, con asignar_escolta"));
                }
                Comando::PrepararCaravana { caravana: self.caravana(&o.caravana_id)?, destino: self.plaza(&o.destino_asentamiento_id)?, carga: self.cantidades(o.carga.as_ref())?, salir_en: o.salir_en }
            }
            Cuerpo::CancelarCaravana(o) => Comando::CancelarCaravana { caravana: self.caravana(&o.caravana_id)? },
            Cuerpo::MoverCargaCaravanaAparcada(o) => {
                let cargar = match o.sentido.as_str() {
                    "cargar" => true,
                    "descargar" => false,
                    otro => return self.forma(format!("sentido desconocido: {otro}")),
                };
                Comando::MoverCargaAparcada { caravana: self.caravana(&o.caravana_id)?, recurso: self.recurso(&o.recurso)?, cantidad: o.cantidad, cargar }
            }
            Cuerpo::EnviarCaravanaAlOrigen(o) => Comando::EnviarCaravanaAlOrigen { caravana: self.caravana(&o.caravana_id)? },
            Cuerpo::EntregarDeCaravana(o) => Comando::EntregarDeCaravana { ejercito: self.ejercito(&o.ejercito_id)?, caravana: self.caravana(&o.caravana_id)?, acuerdo: self.acuerdo(&o.acuerdo_id)? },
            Cuerpo::AdjuntarCaravana(o) => Comando::AdjuntarCaravana { caravana: self.caravana(&o.caravana_id)?, ejercito: self.ejercito(&o.ejercito_id)? },
            Cuerpo::AparcarCaravana(o) => Comando::AparcarCaravana { caravana: self.caravana(&o.caravana_id)?, asentamiento: self.plaza(&o.asentamiento_id)? },

            // ── Pueblos extractores ──
            Cuerpo::MarcarPueblo(o) => Comando::MarcarPueblo { asentamiento: self.plaza(&o.asentamiento_id)?, nodo: self.nodo(&o.nodo_id)? },
            Cuerpo::DestruirPueblo(o) => Comando::DestruirPueblo { pueblo: self.pueblo(&o.pueblo_id)? },
            Cuerpo::ReponerCarroDeAcarreo(o) => Comando::ReponerCarro { pueblo: self.pueblo(&o.pueblo_id)? },
            Cuerpo::EntrarEnPueblo(o) => Comando::EntrarEnPueblo { pueblo: self.pueblo(&o.pueblo_id)?, heroe: self.heroe(&o.heroe_id)? },
            Cuerpo::SalirDePueblo(o) => Comando::SalirDePueblo { pueblo: self.pueblo(&o.pueblo_id)?, heroe: self.heroe(&o.heroe_id)? },
            Cuerpo::VaciarCarro(o) => Comando::VaciarCarro { heroe: self.heroe(&o.heroe_id)? },

            // ── Administración ──
            Cuerpo::Administracion(o) => {
                if !admin {
                    return Err(self.no(rechazos::NO_AUTORIZADO, "solo una sesión de administración da órdenes de administración"));
                }
                self.administracion(&o.codigo, o.parametros.as_ref().unwrap_or(&Struct::default()))?
            }

            // ── La Facción, la ciudadanía y la residencia (sistema 5, 5B) ──
            Cuerpo::CrearFaccion(o) => Comando::CrearFaccion { heroe: self.mi_heroe()?, nombre: o.nombre.clone(), sigilo: o.sigilo.as_ref().map(sigilo) },
            Cuerpo::SolicitarIngreso(o) => Comando::SolicitarIngreso { heroe: self.mi_heroe()?, faccion: self.faccion(&o.faccion_id)? },
            Cuerpo::DejarFaccion(_) => Comando::DejarFaccion { heroe: self.mi_heroe()? },
            Cuerpo::ResponderSolicitud(o) => Comando::ResponderSolicitud { heroe: self.mi_heroe()?, faccion: self.faccion(&o.faccion_id)?, solicitante: self.heroe(&o.heroe_id)?, aceptar: o.aceptar },
            Cuerpo::AsignarRey(o) => Comando::AsignarRey { heroe: self.mi_heroe()?, faccion: self.faccion(&o.faccion_id)?, designado: self.heroe(&o.heroe_id)? },
            Cuerpo::AsignarEmbajador(o) => Comando::AsignarEmbajador { heroe: self.mi_heroe()?, faccion: self.faccion(&o.faccion_id)?, designado: self.heroe(&o.heroe_id)? },
            Cuerpo::CambiarResidencia(o) => Comando::CambiarResidencia { heroe: self.heroe(&o.heroe_id)?, asentamiento: self.plaza(&o.destino_id)? },
            Cuerpo::ResidirEnCampamento(o) => Comando::ResidirEnCampamento { heroe: self.heroe(&o.heroe_id)?, campamento: self.campamento(&o.campamento_id)? },
            Cuerpo::DejarResidencia(o) => Comando::DejarResidencia { heroe: self.heroe(&o.heroe_id)? },

            // ── Lo que el núcleo aún no tiene ──
            Cuerpo::AsignarCargoLocal(_)
            | Cuerpo::DesignarCapital(_)
            | Cuerpo::AdmitirOtrasFacciones(_)
            | Cuerpo::ProponerRelacion(_)
            | Cuerpo::ResponderRelacion(_)
            | Cuerpo::RetirarRelacion(_)
            | Cuerpo::ProponerAnexion(_)
            | Cuerpo::ResponderAnexion(_)
            | Cuerpo::RetirarAnexion(_)
            | Cuerpo::ProponerFusion(_)
            | Cuerpo::ResponderFusion(_)
            | Cuerpo::RetirarFusion(_)
            | Cuerpo::ProponerPaz(_)
            | Cuerpo::RomperRelacion(_)
            | Cuerpo::RebelionVasallo(_)
            | Cuerpo::DeclararGuerra(_)
            | Cuerpo::VetarJugador(_)
            | Cuerpo::RenombrarAsentamiento(_)
            | Cuerpo::AlternarReabastecerAliados(_) => return self.no_disponible("política y Facciones (sistema 5)"),
            Cuerpo::AdoptarTecnologia(_)
            | Cuerpo::EmpezarEpica(_)
            | Cuerpo::AbandonarEpica(_)
            | Cuerpo::ComprarTecnologiaAeda(_)
            | Cuerpo::AbrirAlijo(_) => return self.no_disponible("tecnología, Aedas y alijos (sistema 6)"),
            Cuerpo::ComprarMirada(o) => Comando::ComprarMirada { heroe: self.mi_heroe()?, origen: self.origen_de_intel(o.origen.as_ref().map(|x| (x.tipo.as_str(), x.id.as_str())))?, centro: self.punto(o.centro.as_ref())? },
            Cuerpo::ComprarInformePlaza(o) => {
                Comando::ComprarInformePlaza { heroe: self.mi_heroe()?, origen: self.origen_de_intel(o.origen.as_ref().map(|x| (x.tipo.as_str(), x.id.as_str())))?, asentamiento: self.plaza(&o.asentamiento_id)? }
            }
            Cuerpo::AportarARefundacion(_)
            | Cuerpo::RetirarDeRefundacion(_)
            | Cuerpo::ComprarCaravanaDeRefundacion(_)
            | Cuerpo::LanzarCaravanaFundacion(_)
            | Cuerpo::DesarmarCaravanaFundacion(_) => return self.no_disponible("refundación y Caravana de Fundación"),

            // ── Almacén personal y caravanas a mano (5G-a) ──
            Cuerpo::GuardarEnAlmacenPersonal(o) => Comando::GuardarEnAlmacenPersonal { heroe: self.mi_heroe()?, recurso: self.recurso(&o.recurso)?, cantidad: o.cantidad },
            Cuerpo::SacarDelAlmacenPersonal(o) => Comando::SacarDelAlmacenPersonal { heroe: self.mi_heroe()?, recurso: self.recurso(&o.recurso)?, cantidad: o.cantidad },
            Cuerpo::CargarCaravana(o) => Comando::CargarCaravana {
                ejercito: self.ejercito(&o.ejercito_id)?,
                caravana: self.caravana(&o.caravana_id)?,
                asentamiento: self.plaza(&o.asentamiento_id)?,
                recurso: self.recurso(&o.recurso)?,
                cantidad: o.cantidad,
            },
            Cuerpo::ReclamarCaravana(o) => {
                Comando::ReclamarCaravana { heroe: self.heroe(&o.heroe_id)?, caravana: self.caravana(&o.caravana_id)?, politica: self.politica_o_rechazar(o.politica_de_union.as_deref())? }
            }
            Cuerpo::SoltarCaravana(o) => Comando::SoltarCaravana { ejercito: self.ejercito(&o.ejercito_id)?, caravana: self.caravana(&o.caravana_id)?, heroe: self.heroe(&o.heroe_id)? },
        })
    }

    /// El campamento de la orden o, si no lo dice, aquel en el que está el héroe.
    fn campamento_o_el_del(&self, heroe: u64, id: Option<&str>) -> R<u32> {
        match id {
            Some(s) => self.campamento(s),
            None => self.mundo.campamento_del_heroe(heroe).ok_or_else(|| self.no(rechazos::HEROE_NO_ESTA_EN_EL_CAMPAMENTO, "el héroe no está en ningún campamento")),
        }
    }

    // ───────────────────────────── Administración ─────────────────────────────

    fn campo<'p>(&self, p: &'p Struct, k: &str) -> R<&'p Kind> {
        match p.fields.get(k).and_then(|v| v.kind.as_ref()) {
            Some(Kind::NullValue(_)) | None => self.forma(format!("falta el parámetro «{k}»")),
            Some(v) => Ok(v),
        }
    }

    /// El parámetro opcional: ausente o nulo es `None`.
    fn campo_opcional<'p>(&self, p: &'p Struct, k: &str) -> Option<&'p Kind> {
        p.fields.get(k).and_then(|v| v.kind.as_ref()).filter(|v| !matches!(v, Kind::NullValue(_)))
    }

    fn texto<'p>(&self, p: &'p Struct, k: &str) -> R<&'p str> {
        match self.campo(p, k)? {
            Kind::StringValue(s) => Ok(s),
            _ => self.forma(format!("«{k}» debe ser un texto")),
        }
    }

    fn texto_opcional<'p>(&self, p: &'p Struct, k: &str) -> R<Option<&'p str>> {
        match self.campo_opcional(p, k) {
            None => Ok(None),
            Some(Kind::StringValue(s)) => Ok(Some(s)),
            Some(_) => self.forma(format!("«{k}» debe ser un texto")),
        }
    }

    /// Un número entero (los `Struct` solo llevan `double`: exacto hasta 2⁵³).
    fn entero(&self, p: &Struct, k: &str) -> R<i64> {
        match self.campo(p, k)? {
            Kind::NumberValue(n) if n.fract() == 0.0 && n.abs() <= 9_007_199_254_740_992.0 => Ok(*n as i64),
            _ => self.forma(format!("«{k}» debe ser un entero")),
        }
    }

    fn punto_de(&self, p: &Struct, k: &str) -> R<Punto> {
        match self.campo(p, k)? {
            Kind::StructValue(s) => Ok(Punto::new(self.entero(s, "x")?, self.entero(s, "y")?)),
            _ => self.forma(format!("«{k}» debe ser un punto {{x, y}}")),
        }
    }

    fn lista_de_textos(&self, p: &Struct, k: &str) -> R<Vec<String>> {
        match self.campo(p, k)? {
            Kind::ListValue(l) => l.values.iter().map(|v| if let Some(Kind::StringValue(s)) = &v.kind { Ok(s.clone()) } else { self.forma(format!("«{k}» debe ser una lista de textos")) }).collect(),
            _ => self.forma(format!("«{k}» debe ser una lista de textos")),
        }
    }

    /// La plaza, el campamento o la columna que nombra un id de texto, como sitio de un héroe.
    fn sitio(&self, s: &str) -> R<Ubicacion> {
        if let Some(a) = ids::parse_asentamiento(s) {
            return u32::try_from(a).map_or_else(|_| self.forma(format!("sitio fuera de rango: {s}")), |a| Ok(Ubicacion::Asentamiento(a)));
        }
        if let Some(c) = ids::parse_campamento(s) {
            return Ok(Ubicacion::Mercenarios(c));
        }
        ids::parse_ejercito(s).map_or_else(|| self.forma(format!("sitio desconocido: {s}")), |e| Ok(Ubicacion::Columna(e)))
    }

    /// Las órdenes de administración (`OrdenDeAdministracion.codigo`): una por comando de andamiaje del núcleo, con los ids del contrato y las cantidades en milésimas.
    /// `Docs/Mechanics/Comandos_Parametros.md` lista los parámetros de cada una.
    fn administracion(&self, codigo: &str, p: &Struct) -> R<Comando> {
        let (t, n) = (|k| self.texto(p, k), |k| self.entero(p, k));
        let heroe = || self.heroe(t("heroe_id")?);
        let plaza = || self.plaza(t("asentamiento_id")?);
        let ejercito = || self.ejercito(t("ejercito_id")?);
        let recurso = || self.recurso(t("recurso")?);
        Ok(match codigo {
            "crear_ejercito" => Comando::CrearEjercito { cuenta: t("cuenta")?.into(), faccion: self.faccion(t("faccion_id")?)?, posicion: self.punto_de(p, "posicion")?, velocidad: n("velocidad")?, viveres: n("viveres")? },
            "fijar_nivel_faccion" => Comando::FijarNivelFaccion { faccion: self.faccion(t("faccion_id")?)?, nivel: self.entero_u32(n("nivel")?, "nivel")? },
            "dar_caravana_fundacion" => Comando::DarCaravanaFundacion { ejercito: ejercito()? },
            "fijar_nivel_asentamiento" => Comando::FijarNivelAsentamiento { asentamiento: plaza()?, nivel: self.entero_u32(n("nivel")?, "nivel")? },
            "agrandar_zona" => Comando::AgrandarZona { asentamiento: plaza()? },
            "reclamar_parcela" => Comando::ReclamarParcela { asentamiento: plaza()? },
            "soltar_parcela" => Comando::SoltarParcela { asentamiento: plaza()?, parcela: self.entero_u32(n("parcela")?, "parcela")? },
            "dar_recursos" => Comando::DarRecursos { asentamiento: plaza()?, recurso: recurso()?, cantidad: n("cantidad")? },
            "dar_suministro" => Comando::DarSuministro { ejercito: ejercito()?, recurso: recurso()?, cantidad: n("cantidad")? },
            "dar_almacen_personal" => Comando::DarAlmacenPersonal { heroe: heroe()?, recurso: recurso()?, cantidad: n("cantidad")? },
            "fijar_residencia" => {
                let residencia = match self.sitio_opcional(p, "residencia")? {
                    None => None,
                    Some(Ubicacion::Asentamiento(a)) => Some(Residencia::Asentamiento(a)),
                    Some(Ubicacion::Mercenarios(c)) => Some(Residencia::Campamento(c)),
                    Some(_) => return self.forma("la residencia es una plaza o un campamento"),
                };
                Comando::FijarResidencia { heroe: heroe()?, residencia }
            }
            "fijar_tecnologias" => Comando::FijarTecnologias { faccion: self.faccion(t("faccion_id")?)?, tecnologias: self.lista_de_textos(p, "tecnologias")? },
            "fijar_faccion_de_heroe" => Comando::FijarFaccionDeHeroe { heroe: heroe()?, faccion: self.texto_opcional(p, "faccion_id")?.map(|f| self.faccion(f)).transpose()? },
            "colocar_heroe" => Comando::ColocarHeroe { heroe: heroe()?, ubicacion: self.sitio(t("ubicacion")?)? },
            "herir_heroe" => Comando::HerirHeroe { heroe: heroe()? },
            "fijar_relacion" => {
                let relacion = match self.texto_opcional(p, "relacion")? {
                    None => None,
                    Some("alianza") => Some(Relacion::Alianza),
                    Some("vasallaje") => Some(Relacion::Vasallaje),
                    Some("guerra") => Some(Relacion::Guerra),
                    Some(otra) => return self.forma(format!("relación desconocida: {otra}")),
                };
                Comando::FijarRelacion { a: self.faccion(t("faccion_a_id")?)?, b: self.faccion(t("faccion_b_id")?)?, relacion }
            }
            "colocar_bandidos" => Comando::ColocarBandidos { posicion: self.punto_de(p, "posicion")?, nivel: u8::try_from(n("nivel")?).map_or_else(|_| self.forma("nivel fuera de rango"), Ok)? },
            "dar_escuadra" => Comando::DarEscuadra { heroe: heroe()?, tropa: self.tropa(t("tropa_id")?)?, cantidad: n("cantidad")? },
            "fijar_controlador" => {
                let controlador = match t("controlador")? {
                    "humano" => Controlador::Humano,
                    "bot" => Controlador::Bot,
                    otro => return self.forma(format!("controlador desconocido: {otro}")),
                };
                Comando::FijarControlador { heroe: heroe()?, controlador }
            }
            otro => return self.forma(format!("orden de administración desconocida: {otro}")),
        })
    }

    fn sitio_opcional(&self, p: &Struct, k: &str) -> R<Option<Ubicacion>> {
        self.texto_opcional(p, k)?.map(|s| self.sitio(s)).transpose()
    }
}

#[cfg(test)]
mod pruebas;
