//! Lo que entra al núcleo y lo que sale de él. Una [`Peticion`] aceptada es lo que se escribe en el registro: reproducidas
//! en orden, en sus instantes, reconstruyen el mismo mundo.

use crate::codec::{ErrorFormato, Escritor, Lector};
use base::{Instante, Mil, Punto};

/// Acción sobre el reloj de mundo (`OrdenControlarReloj`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccionReloj {
    Pausar,
    Reanudar,
    /// Segundos de mundo por segundo real × 1000.
    FijarFactor(u32),
    /// Adelanta el mundo hasta ese instante sin esperar (modo acelerado).
    AvanzarHasta(Instante),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Comando {
    /// Alta de una columna (la hará `salir_al_mundo` en la fase 6). Solo administración.
    CrearEjercito { lider: String, propietario: String, faccion: u32, posicion: Punto, velocidad: Mil, viveres: Mil, capacidad_viveres: Mil },
    /// Marcha desde la posición real al instante de la orden.
    MarcharA { ejercito: u64, destino: Punto },
    /// Cancela la marcha: la columna se queda donde está.
    Estacionar { ejercito: u64 },
    /// Solo administración.
    Reloj(AccionReloj),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Peticion {
    /// Cliente que la manda; clave de la idempotencia junto con `peticion_id`.
    pub actor: String,
    pub admin: bool,
    pub peticion_id: String,
    pub version_esperada: Option<u64>,
    pub comando: Comando,
}

impl Peticion {
    pub fn nueva(actor: &str, peticion_id: &str, comando: Comando) -> Self {
        Self { actor: actor.into(), admin: false, peticion_id: peticion_id.into(), version_esperada: None, comando }
    }
    pub fn de_admin(mut self) -> Self {
        self.admin = true;
        self
    }

    pub(crate) fn codificar(&self) -> Vec<u8> {
        let mut w = Escritor::default();
        w.texto(&self.actor);
        w.bool(self.admin);
        w.texto(&self.peticion_id);
        w.opt_u64(self.version_esperada);
        match &self.comando {
            Comando::CrearEjercito { lider, propietario, faccion, posicion, velocidad, viveres, capacidad_viveres } => {
                w.u8(1);
                w.texto(lider);
                w.texto(propietario);
                w.u32(*faccion);
                w.punto(*posicion);
                w.i64(*velocidad);
                w.i64(*viveres);
                w.i64(*capacidad_viveres);
            }
            Comando::MarcharA { ejercito, destino } => {
                w.u8(2);
                w.u64(*ejercito);
                w.punto(*destino);
            }
            Comando::Estacionar { ejercito } => {
                w.u8(3);
                w.u64(*ejercito);
            }
            Comando::Reloj(a) => {
                w.u8(4);
                match a {
                    AccionReloj::Pausar => w.u8(0),
                    AccionReloj::Reanudar => w.u8(1),
                    AccionReloj::FijarFactor(f) => {
                        w.u8(2);
                        w.u32(*f);
                    }
                    AccionReloj::AvanzarHasta(t) => {
                        w.u8(3);
                        w.i64(*t);
                    }
                }
            }
        }
        w.0
    }

    pub(crate) fn decodificar(bytes: &[u8]) -> Result<Peticion, ErrorFormato> {
        let mut r = Lector::nuevo(bytes);
        let (actor, admin, peticion_id, version_esperada) = (r.texto()?, r.bool()?, r.texto()?, r.opt_u64()?);
        let comando = match r.u8()? {
            1 => Comando::CrearEjercito {
                lider: r.texto()?,
                propietario: r.texto()?,
                faccion: r.u32()?,
                posicion: r.punto()?,
                velocidad: r.i64()?,
                viveres: r.i64()?,
                capacidad_viveres: r.i64()?,
            },
            2 => Comando::MarcharA { ejercito: r.u64()?, destino: r.punto()? },
            3 => Comando::Estacionar { ejercito: r.u64()? },
            4 => Comando::Reloj(match r.u8()? {
                0 => AccionReloj::Pausar,
                1 => AccionReloj::Reanudar,
                2 => AccionReloj::FijarFactor(r.u32()?),
                3 => AccionReloj::AvanzarHasta(r.i64()?),
                _ => return Err(ErrorFormato("acción de reloj desconocida")),
            }),
            _ => return Err(ErrorFormato("comando desconocido")),
        };
        r.fin()?;
        Ok(Peticion { actor, admin, peticion_id, version_esperada, comando })
    }
}

/// Códigos de rechazo estables (`RechazoOrden.codigo`).
pub mod rechazos {
    pub const NO_AUTORIZADO: &str = "no_autorizado";
    pub const VERSION_OBSOLETA: &str = "version.obsoleta";
    pub const EJERCITO_DESCONOCIDO: &str = "ejercito.desconocido";
    pub const HEROE_DESCONOCIDO: &str = "heroe.desconocido";
    pub const RUTA_IMPOSIBLE: &str = "ruta.imposible";
    pub const ORDEN_NO_SOPORTADA: &str = "orden.no_soportada";
    pub const OBJETIVO_NO_SOPORTADO: &str = "objetivo.no_soportado";
    pub const RELOJ_INVALIDO: &str = "reloj.invalido";
    pub const LIDER_REPETIDO: &str = "heroe.ya_tiene_columna";
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resultado {
    Aceptada,
    Rechazada { codigo: &'static str, mensaje: String },
    /// Válida pero no cambió nada: no sube la versión.
    SinCambios,
    /// Misma `peticion_id` ya aceptada: no se ejecuta otra vez.
    Duplicada,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Respuesta {
    pub resultado: Resultado,
    /// Versión de mundo con los efectos ya aplicados.
    pub version_tras: u64,
    /// Ejércitos creados por la orden.
    pub creadas: Vec<u64>,
}

impl Respuesta {
    pub(crate) fn rechazo(codigo: &'static str, mensaje: impl Into<String>, version: u64) -> Self {
        Self { resultado: Resultado::Rechazada { codigo, mensaje: mensaje.into() }, version_tras: version, creadas: vec![] }
    }
    pub fn aceptada(&self) -> bool {
        self.resultado == Resultado::Aceptada
    }
}
