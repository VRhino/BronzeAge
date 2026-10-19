//! Lo que entra al núcleo y lo que sale de él. Una [`Peticion`] aceptada es lo que se escribe en el registro: reproducidas
//! en orden, en sus instantes, reconstruyen el mismo mundo.

use crate::codec::{ErrorFormato, Escritor, Lector};
use crate::codec_economia::{cargo_a_u8, cargo_de_u8, recurso_a_u8, recurso_de_u8, tipo_a_u8, tipo_de_u8};
use crate::codec_comercio::{animal_a_u8, animal_de_u8, tipo_carro_a_u8, tipo_carro_de_u8};
use asentamiento::{Cargo, Recurso, Tipo};
use base::{Instante, Mil, Punto};
use comercio::caravana::{Animal, TipoCarro};

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
    /// Nivel de una Facción (fija su cap de fundación). Solo administración; lo sustituirá la experiencia de Facción.
    FijarNivelFaccion { faccion: u32, nivel: u32 },
    /// Engancha una Caravana de Fundación a la columna. Solo administración; lo sustituirán la compra en un campamento y el
    /// lanzamiento desde una plaza.
    DarCaravanaFundacion { ejercito: u64 },
    /// Funda una plaza donde está la columna, gastando su caravana (Doc 1.8).
    Fundar { ejercito: u64 },
    /// Sube el nivel de una plaza (y con él el techo de su zona). Solo administración; lo sustituirá el ascenso.
    FijarNivelAsentamiento { asentamiento: u64, nivel: u32 },
    /// Un edificio completado agranda la zona de su plaza (`crecimiento_por_edificio`, hasta el techo del nivel). Solo
    /// administración; lo llamará la construcción.
    AgrandarZona { asentamiento: u64 },
    /// Una Leñera pide su parcela de bosque (Doc 1.9c). Solo administración; lo llamará la construcción.
    ReclamarParcela { asentamiento: u64 },
    /// Una Leñera destruida devuelve su parcela. Solo administración.
    SoltarParcela { asentamiento: u64, parcela: u32 },
    // ── Economía y construcción de una plaza (Economia_Construccion §13). Las puede dar la administración o quien sea propietario de
    //    alguna columna de la Facción de la plaza; los cargos (Gobernador, Maestro de Obras, Tesorero) los traerá el sistema de residentes. ──
    /// Encola un edificio (paga el coste completo al encolar). `creadas` lleva el id del edificio nuevo.
    AnadirEdificio { asentamiento: u64, tipo: Tipo },
    /// Saca de la cola un edificio que aún no empezó y devuelve su coste.
    QuitarDeCola { asentamiento: u64, edificio: u32 },
    /// Sube o baja un puesto en la cola de obras.
    MoverEnCola { asentamiento: u64, edificio: u32, arriba: bool },
    /// Mejora un edificio activo al siguiente nivel interno.
    MejorarEdificio { asentamiento: u64, edificio: u32 },
    /// Pausa o reanuda la auto-construcción (solo la detección de necesidades nuevas).
    AlternarAutoConstruccion { asentamiento: u64, pausada: bool },
    /// Para o reanuda una receta por el recurso que produce, en todos los talleres.
    AlternarReceta { asentamiento: u64, recurso: Recurso, pausada: bool },
    /// Reserva manual del Tesorero para un recurso (milésimas; se recorta al máximo).
    CalibrarReserva { asentamiento: u64, recurso: Recurso, valor: Mil },
    /// Pide la subida al nivel siguiente (obra de ascenso).
    SolicitarAscenso { asentamiento: u64 },
    /// Activa una política con la duración por defecto, en el cargo indicado.
    ActivarPolitica { asentamiento: u64, politica: String, cargo: Cargo },
    // ── Comercio (Comercio_Caravanas §4-§5, §8). Las puede dar la administración o el propietario de alguna columna de la Facción de la plaza
    //    (en el trueque, la del lado al que le toca); los cargos, la residencia y la presencia vendrán del sistema de héroes y residentes. ──
    /// Propone un trueque de `a` a `b`: `lineas_a` salen de A, `lineas_b` de B. `creadas` lleva el id del acuerdo.
    ProponerTrueque { a: u64, b: u64, lineas_a: Vec<(Recurso, Mil)>, lineas_b: Vec<(Recurso, Mil)> },
    /// Acepta una propuesta (la plaza B): el plazo para cumplir se cuenta desde el sí.
    AceptarTrueque { acuerdo: u64 },
    /// Rechaza una propuesta (la plaza B).
    RechazarTrueque { acuerdo: u64 },
    /// Coloca una orden de compra o de venta en el mercado de la plaza. `precio` `None` = el de referencia. `creadas` lleva el id de la orden.
    ColocarOrdenMercado { asentamiento: u64, venta: bool, recurso: Recurso, cantidad: Mil, precio: Option<i64> },
    /// Retira una orden activa antes de su plazo (no figura en el documento de mecánicas: lo pidió el autor al encargar la fase 3B-1).
    CancelarOrdenMercado { orden: u64 },
    /// El Líder de la columna toma una orden en el mostrador de la plaza, con su columna en la puerta.
    ComerciarEnPlaza { ejercito: u64, orden: u64, cantidad: Mil },
    /// Crea un casco de caravana comercial en la plaza (Mercado activo, cooldown y cupo). `creadas` lleva el id de la caravana.
    CrearCaravana { asentamiento: u64 },
    /// Fabrica un carro (sin animal) y lo monta en la caravana, pagando del almacén de su origen.
    AnadirCarro { caravana: u64, tipo: TipoCarro },
    /// Compra un animal (oro del almacén del origen) para el carro `carro` de la caravana.
    AnadirAnimal { caravana: u64, carro: u32, animal: Animal },
    /// Pasa el carro `carro` (con su animal) de la caravana `de` a la caravana `a`, del mismo origen.
    MoverCarro { de: u64, a: u64, carro: u32 },
    /// Pone o quita `reservadaManual` (fuera del reparto automático).
    ReservarCaravana { caravana: u64, reservada: bool },
    /// Lanza a mano una caravana: reserva la carga del almacén y queda `preparando`. Aún no sale (fase 3B-2).
    PrepararCaravana { caravana: u64, destino: u64, carga: Vec<(Recurso, Mil)>, salir_en: Option<Instante> },
    /// Cancela una caravana `preparando`: la carga vuelve al almacén de su origen (topada por su capacidad).
    CancelarCaravana { caravana: u64 },
    /// Carga o descarga una caravana aparcada contra el almacén de la plaza que la hospeda.
    MoverCargaAparcada { caravana: u64, recurso: Recurso, cantidad: Mil, cargar: bool },
    /// Manda una caravana aparcada vacía a su origen (al instante). Con carga hace falta un viaje (fase 3B-2).
    EnviarCaravanaAlOrigen { caravana: u64 },
    /// Entrega manual de una caravana adjunta a la columna, a la puerta de la plaza que recibe (§4.5).
    EntregarDeCaravana { ejercito: u64, caravana: u64, acuerdo: u64 },
    // ── Andamiaje de administración, provisional hasta que existan los sistemas que lo sustituyen. ──
    /// Añade recursos al almacén de una plaza (fuera del diseño: no es una fuente de juego). Solo administración.
    DarRecursos { asentamiento: u64, recurso: Recurso, cantidad: Mil },
    /// Añade al carro de una columna (`suministro`). Solo administración; lo sustituirán el sistema de héroes y el botín.
    DarSuministro { ejercito: u64, recurso: Recurso, cantidad: Mil },
    /// Engancha una caravana `disponible` o `aparcada` a una columna. Solo administración; lo hará el sistema de ejércitos.
    AdjuntarCaravana { caravana: u64, ejercito: u64 },
    /// Deja la caravana adjunta aparcada en una plaza de su Facción (la columna entra en ella). Solo administración; lo hará el sistema de ejércitos.
    AparcarCaravana { caravana: u64, asentamiento: u64 },
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

/// Una lista de `(recurso, cantidad)`: `n u64 · n × (recurso u8, cantidad i64)`.
fn lineas(w: &mut Escritor, v: &[(Recurso, Mil)]) {
    w.u64(v.len() as u64);
    for &(r, n) in v {
        w.u8(recurso_a_u8(r));
        w.i64(n);
    }
}

fn leer_lineas(r: &mut Lector) -> Result<Vec<(Recurso, Mil)>, ErrorFormato> {
    let mut v = Vec::new();
    for _ in 0..r.cuenta()? {
        v.push((recurso_de_u8(r.u8()?)?, r.i64()?));
    }
    Ok(v)
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
            Comando::FijarNivelFaccion { faccion, nivel } => {
                w.u8(5);
                w.u32(*faccion);
                w.u32(*nivel);
            }
            Comando::DarCaravanaFundacion { ejercito } => {
                w.u8(6);
                w.u64(*ejercito);
            }
            Comando::Fundar { ejercito } => {
                w.u8(7);
                w.u64(*ejercito);
            }
            Comando::FijarNivelAsentamiento { asentamiento, nivel } => {
                w.u8(8);
                w.u64(*asentamiento);
                w.u32(*nivel);
            }
            Comando::AgrandarZona { asentamiento } => {
                w.u8(9);
                w.u64(*asentamiento);
            }
            Comando::ReclamarParcela { asentamiento } => {
                w.u8(10);
                w.u64(*asentamiento);
            }
            Comando::SoltarParcela { asentamiento, parcela } => {
                w.u8(11);
                w.u64(*asentamiento);
                w.u32(*parcela);
            }
            Comando::AnadirEdificio { asentamiento, tipo } => {
                w.u8(12);
                w.u64(*asentamiento);
                w.u8(tipo_a_u8(*tipo));
            }
            Comando::QuitarDeCola { asentamiento, edificio } => {
                w.u8(13);
                w.u64(*asentamiento);
                w.u32(*edificio);
            }
            Comando::MoverEnCola { asentamiento, edificio, arriba } => {
                w.u8(14);
                w.u64(*asentamiento);
                w.u32(*edificio);
                w.bool(*arriba);
            }
            Comando::MejorarEdificio { asentamiento, edificio } => {
                w.u8(15);
                w.u64(*asentamiento);
                w.u32(*edificio);
            }
            Comando::AlternarAutoConstruccion { asentamiento, pausada } => {
                w.u8(16);
                w.u64(*asentamiento);
                w.bool(*pausada);
            }
            Comando::AlternarReceta { asentamiento, recurso, pausada } => {
                w.u8(17);
                w.u64(*asentamiento);
                w.u8(recurso_a_u8(*recurso));
                w.bool(*pausada);
            }
            Comando::CalibrarReserva { asentamiento, recurso, valor } => {
                w.u8(18);
                w.u64(*asentamiento);
                w.u8(recurso_a_u8(*recurso));
                w.i64(*valor);
            }
            Comando::SolicitarAscenso { asentamiento } => {
                w.u8(19);
                w.u64(*asentamiento);
            }
            Comando::ActivarPolitica { asentamiento, politica, cargo } => {
                w.u8(20);
                w.u64(*asentamiento);
                w.texto(politica);
                w.u8(cargo_a_u8(*cargo));
            }
            Comando::ProponerTrueque { a, b, lineas_a, lineas_b } => {
                w.u8(21);
                w.u64(*a);
                w.u64(*b);
                lineas(&mut w, lineas_a);
                lineas(&mut w, lineas_b);
            }
            Comando::AceptarTrueque { acuerdo } => {
                w.u8(22);
                w.u64(*acuerdo);
            }
            Comando::RechazarTrueque { acuerdo } => {
                w.u8(23);
                w.u64(*acuerdo);
            }
            Comando::ColocarOrdenMercado { asentamiento, venta, recurso, cantidad, precio } => {
                w.u8(24);
                w.u64(*asentamiento);
                w.bool(*venta);
                w.u8(recurso_a_u8(*recurso));
                w.i64(*cantidad);
                w.opt_i64(*precio);
            }
            Comando::CancelarOrdenMercado { orden } => {
                w.u8(25);
                w.u64(*orden);
            }
            Comando::ComerciarEnPlaza { ejercito, orden, cantidad } => {
                w.u8(26);
                w.u64(*ejercito);
                w.u64(*orden);
                w.i64(*cantidad);
            }
            Comando::CrearCaravana { asentamiento } => {
                w.u8(27);
                w.u64(*asentamiento);
            }
            Comando::AnadirCarro { caravana, tipo } => {
                w.u8(28);
                w.u64(*caravana);
                w.u8(tipo_carro_a_u8(*tipo));
            }
            Comando::AnadirAnimal { caravana, carro, animal } => {
                w.u8(29);
                w.u64(*caravana);
                w.u32(*carro);
                w.u8(animal_a_u8(*animal));
            }
            Comando::MoverCarro { de, a, carro } => {
                w.u8(30);
                w.u64(*de);
                w.u64(*a);
                w.u32(*carro);
            }
            Comando::ReservarCaravana { caravana, reservada } => {
                w.u8(31);
                w.u64(*caravana);
                w.bool(*reservada);
            }
            Comando::PrepararCaravana { caravana, destino, carga, salir_en } => {
                w.u8(32);
                w.u64(*caravana);
                w.u64(*destino);
                lineas(&mut w, carga);
                w.opt_i64(*salir_en);
            }
            Comando::CancelarCaravana { caravana } => {
                w.u8(33);
                w.u64(*caravana);
            }
            Comando::MoverCargaAparcada { caravana, recurso, cantidad, cargar } => {
                w.u8(34);
                w.u64(*caravana);
                w.u8(recurso_a_u8(*recurso));
                w.i64(*cantidad);
                w.bool(*cargar);
            }
            Comando::EnviarCaravanaAlOrigen { caravana } => {
                w.u8(35);
                w.u64(*caravana);
            }
            Comando::EntregarDeCaravana { ejercito, caravana, acuerdo } => {
                w.u8(36);
                w.u64(*ejercito);
                w.u64(*caravana);
                w.u64(*acuerdo);
            }
            Comando::DarRecursos { asentamiento, recurso, cantidad } => {
                w.u8(37);
                w.u64(*asentamiento);
                w.u8(recurso_a_u8(*recurso));
                w.i64(*cantidad);
            }
            Comando::DarSuministro { ejercito, recurso, cantidad } => {
                w.u8(38);
                w.u64(*ejercito);
                w.u8(recurso_a_u8(*recurso));
                w.i64(*cantidad);
            }
            Comando::AdjuntarCaravana { caravana, ejercito } => {
                w.u8(39);
                w.u64(*caravana);
                w.u64(*ejercito);
            }
            Comando::AparcarCaravana { caravana, asentamiento } => {
                w.u8(40);
                w.u64(*caravana);
                w.u64(*asentamiento);
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
            5 => Comando::FijarNivelFaccion { faccion: r.u32()?, nivel: r.u32()? },
            6 => Comando::DarCaravanaFundacion { ejercito: r.u64()? },
            7 => Comando::Fundar { ejercito: r.u64()? },
            8 => Comando::FijarNivelAsentamiento { asentamiento: r.u64()?, nivel: r.u32()? },
            9 => Comando::AgrandarZona { asentamiento: r.u64()? },
            10 => Comando::ReclamarParcela { asentamiento: r.u64()? },
            11 => Comando::SoltarParcela { asentamiento: r.u64()?, parcela: r.u32()? },
            12 => Comando::AnadirEdificio { asentamiento: r.u64()?, tipo: tipo_de_u8(r.u8()?)? },
            13 => Comando::QuitarDeCola { asentamiento: r.u64()?, edificio: r.u32()? },
            14 => Comando::MoverEnCola { asentamiento: r.u64()?, edificio: r.u32()?, arriba: r.bool()? },
            15 => Comando::MejorarEdificio { asentamiento: r.u64()?, edificio: r.u32()? },
            16 => Comando::AlternarAutoConstruccion { asentamiento: r.u64()?, pausada: r.bool()? },
            17 => Comando::AlternarReceta { asentamiento: r.u64()?, recurso: recurso_de_u8(r.u8()?)?, pausada: r.bool()? },
            18 => Comando::CalibrarReserva { asentamiento: r.u64()?, recurso: recurso_de_u8(r.u8()?)?, valor: r.i64()? },
            19 => Comando::SolicitarAscenso { asentamiento: r.u64()? },
            20 => Comando::ActivarPolitica { asentamiento: r.u64()?, politica: r.texto()?, cargo: cargo_de_u8(r.u8()?)? },
            21 => Comando::ProponerTrueque { a: r.u64()?, b: r.u64()?, lineas_a: leer_lineas(&mut r)?, lineas_b: leer_lineas(&mut r)? },
            22 => Comando::AceptarTrueque { acuerdo: r.u64()? },
            23 => Comando::RechazarTrueque { acuerdo: r.u64()? },
            24 => Comando::ColocarOrdenMercado { asentamiento: r.u64()?, venta: r.bool()?, recurso: recurso_de_u8(r.u8()?)?, cantidad: r.i64()?, precio: r.opt_i64()? },
            25 => Comando::CancelarOrdenMercado { orden: r.u64()? },
            26 => Comando::ComerciarEnPlaza { ejercito: r.u64()?, orden: r.u64()?, cantidad: r.i64()? },
            27 => Comando::CrearCaravana { asentamiento: r.u64()? },
            28 => Comando::AnadirCarro { caravana: r.u64()?, tipo: tipo_carro_de_u8(r.u8()?)? },
            29 => Comando::AnadirAnimal { caravana: r.u64()?, carro: r.u32()?, animal: animal_de_u8(r.u8()?)? },
            30 => Comando::MoverCarro { de: r.u64()?, a: r.u64()?, carro: r.u32()? },
            31 => Comando::ReservarCaravana { caravana: r.u64()?, reservada: r.bool()? },
            32 => Comando::PrepararCaravana { caravana: r.u64()?, destino: r.u64()?, carga: leer_lineas(&mut r)?, salir_en: r.opt_i64()? },
            33 => Comando::CancelarCaravana { caravana: r.u64()? },
            34 => Comando::MoverCargaAparcada { caravana: r.u64()?, recurso: recurso_de_u8(r.u8()?)?, cantidad: r.i64()?, cargar: r.bool()? },
            35 => Comando::EnviarCaravanaAlOrigen { caravana: r.u64()? },
            36 => Comando::EntregarDeCaravana { ejercito: r.u64()?, caravana: r.u64()?, acuerdo: r.u64()? },
            37 => Comando::DarRecursos { asentamiento: r.u64()?, recurso: recurso_de_u8(r.u8()?)?, cantidad: r.i64()? },
            38 => Comando::DarSuministro { ejercito: r.u64()?, recurso: recurso_de_u8(r.u8()?)?, cantidad: r.i64()? },
            39 => Comando::AdjuntarCaravana { caravana: r.u64()?, ejercito: r.u64()? },
            40 => Comando::AparcarCaravana { caravana: r.u64()?, asentamiento: r.u64()? },
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
    pub const SIN_MUNDO: &str = "mundo.no_cargado";
    pub const ASENTAMIENTO_DESCONOCIDO: &str = "asentamiento.desconocido";
    pub const SIN_CARAVANA: &str = "fundacion.sin_caravana";
    pub const CAP_ALCANZADO: &str = "fundacion.cap_alcanzado";
    pub const NIVEL_INVALIDO: &str = "nivel.invalido";
    pub const PARCELA_SIN_LIBRES: &str = "parcela.sin_libres";
    pub const PARCELA_CUPO_AGOTADO: &str = "parcela.cupo_agotado";
    pub const PARCELA_AJENA: &str = "parcela.no_es_tuya";
    // Obras (`ErrorObra` del crate `asentamiento`).
    pub const OBRA_TECNOLOGIA_FALTANTE: &str = "obra.tecnologia_faltante";
    pub const OBRA_ERA_INSUFICIENTE: &str = "obra.era_insuficiente";
    pub const OBRA_TIPO_NO_MANUAL: &str = "obra.tipo_no_manual";
    pub const OBRA_UNICO: &str = "obra.unico";
    pub const OBRA_TOPE_ALMACENES: &str = "obra.tope_almacenes";
    pub const OBRA_TOPE_VIVIENDAS: &str = "obra.tope_viviendas";
    pub const OBRA_NIVEL_INSUFICIENTE: &str = "obra.nivel_insuficiente";
    pub const OBRA_TOPE_TRANSFORMACION: &str = "obra.tope_transformacion";
    pub const OBRA_GRAN_FUNDICION: &str = "obra.gran_fundicion";
    pub const OBRA_COLA_LLENA: &str = "obra.cola_llena";
    pub const OBRA_SIN_SITIO: &str = "obra.sin_sitio";
    pub const OBRA_RECURSOS_INSUFICIENTES: &str = "obra.recursos_insuficientes";
    pub const EDIFICIO_DESCONOCIDO: &str = "edificio.desconocido";
    pub const OBRA_NO_ESTA_EN_COLA: &str = "obra.no_esta_en_cola";
    pub const OBRA_NO_MEJORABLE: &str = "obra.no_mejorable";
    pub const OBRA_SIN_CUADRILLA: &str = "obra.sin_cuadrilla";
    // Ascenso (`Bloqueo`): el rechazo lleva el código del primero y todos en el mensaje.
    pub const ASCENSO_NIVEL_MAXIMO: &str = "ascenso.nivel_maximo";
    pub const ASCENSO_EN_CURSO: &str = "ascenso.en_curso";
    pub const ASCENSO_FALTA_POBLACION: &str = "ascenso.falta_poblacion";
    pub const ASCENSO_FALTAN_EDIFICIOS: &str = "ascenso.faltan_edificios";
    pub const ASCENSO_SIN_CUPO_DE_FACCION: &str = "ascenso.sin_cupo_de_faccion";
    pub const ASCENSO_RECURSOS_INSUFICIENTES: &str = "ascenso.recursos_insuficientes";
    pub const ASCENSO_INSOLVENTE: &str = "ascenso.insolvente";
    // Políticas (`ErrorPolitica`).
    pub const POLITICA_DESCONOCIDA: &str = "politica.desconocida";
    pub const POLITICA_FUERA_DE_POOL: &str = "politica.fuera_de_pool";
    pub const POLITICA_CARGO_VACIO: &str = "politica.cargo_vacio";
    pub const POLITICA_YA_ACTIVA: &str = "politica.ya_activa";
    pub const POLITICA_SIN_SLOT: &str = "politica.sin_slot";
    // Trueque (`ErrorTrueque`).
    pub const TRUEQUE_DESCONOCIDO: &str = "trueque.desconocido";
    pub const TRUEQUE_MISMO_ASENTAMIENTO: &str = "trueque.mismo_asentamiento";
    pub const TRUEQUE_LADO_VACIO: &str = "trueque.lado_vacio";
    pub const TRUEQUE_CANTIDAD_NO_POSITIVA: &str = "trueque.cantidad_no_positiva";
    pub const TRUEQUE_RECURSO_REPETIDO: &str = "trueque.recurso_repetido";
    pub const TRUEQUE_NO_ESTA_PROPUESTO: &str = "trueque.no_esta_propuesto";
    pub const TRUEQUE_PROPUESTA_VENCIDA: &str = "trueque.propuesta_vencida";
    pub const TRUEQUE_NO_ACTIVO: &str = "trueque.no_activo";
    // Mercado de la plaza (`ErrorOrden`) y mostrador (`ErrorMostrador`).
    pub const MERCADO_SIN_MERCADO: &str = "mercado.sin_mercado";
    pub const ORDEN_DESCONOCIDA: &str = "orden.desconocida";
    pub const ORDEN_NO_ACTIVA: &str = "orden.no_activa";
    pub const ORDEN_CANTIDAD_NO_POSITIVA: &str = "orden.cantidad_no_positiva";
    pub const ORDEN_SIN_STOCK: &str = "orden.sin_stock";
    pub const MOSTRADOR_NO_ES_LIDER: &str = "mostrador.no_es_lider";
    pub const MOSTRADOR_ORDEN_AJENA: &str = "mostrador.orden_ajena";
    pub const MOSTRADOR_ORDEN_CADUCADA: &str = "mostrador.orden_caducada";
    pub const MOSTRADOR_FUERA_DE_LA_PUERTA: &str = "mostrador.fuera_de_la_puerta";
    pub const MOSTRADOR_CANTIDAD_NO_POSITIVA: &str = "mostrador.cantidad_no_positiva";
    pub const MOSTRADOR_PRECIO_NO_POSITIVO: &str = "mostrador.precio_no_positivo";
    pub const MOSTRADOR_RECURSO_ORO: &str = "mostrador.recurso_oro";
    pub const MOSTRADOR_NADA_QUE_SERVIR: &str = "mostrador.nada_que_servir";
    // Caravanas (`ErrorCaravana`).
    pub const CARAVANA_DESCONOCIDA: &str = "caravana.desconocida";
    pub const CARAVANA_NO_ES_COMERCIAL: &str = "caravana.no_es_comercial";
    pub const CARAVANA_NO_DISPONIBLE: &str = "caravana.no_disponible";
    pub const CARAVANA_NO_PREPARANDO: &str = "caravana.no_preparando";
    pub const CARAVANA_NO_APARCADA: &str = "caravana.no_aparcada";
    pub const CARAVANA_ESTADO_INVALIDO: &str = "caravana.estado_invalido";
    pub const CARAVANA_SIN_MERCADO: &str = "caravana.sin_mercado";
    pub const CARAVANA_EN_COOLDOWN: &str = "caravana.en_cooldown";
    pub const CARAVANA_CUPO_DE_CARAVANAS: &str = "caravana.cupo_de_caravanas";
    pub const CARAVANA_CUPO_DE_CARROS: &str = "caravana.cupo_de_carros";
    pub const CARAVANA_SIN_CARPINTERIA: &str = "caravana.sin_carpinteria";
    pub const CARAVANA_SIN_RECURSOS: &str = "caravana.sin_recursos";
    pub const CARAVANA_CARRO_INEXISTENTE: &str = "caravana.carro_inexistente";
    pub const CARAVANA_CARRO_YA_TIENE_ANIMAL: &str = "caravana.carro_ya_tiene_animal";
    pub const CARAVANA_ORIGEN_DISTINTO: &str = "caravana.origen_distinto";
    pub const CARAVANA_SIN_CAPACIDAD: &str = "caravana.sin_capacidad";
    pub const CARAVANA_CARGA_INVALIDA: &str = "caravana.carga_invalida";
    pub const CARAVANA_SIN_STOCK: &str = "caravana.sin_stock";
    pub const CARAVANA_CUPO_DE_ESCOLTA: &str = "caravana.cupo_de_escolta";
    pub const CARAVANA_ESCOLTA_INVALIDA: &str = "caravana.escolta_invalida";
    pub const CARAVANA_SIN_RUTA: &str = "caravana.sin_ruta";
    pub const CARAVANA_SALIDA_INVALIDA: &str = "caravana.salida_invalida";
    pub const CARAVANA_NO_HA_LLEGADO: &str = "caravana.no_ha_llegado";
    pub const CARAVANA_SIN_PLAN: &str = "caravana.sin_plan";
    pub const CARAVANA_NADA_QUE_MOVER: &str = "caravana.nada_que_mover";
    /// 3B-1: una salida inmediata (un solo carro y sin `salir_en`) necesita el movimiento de la fase 3B-2.
    pub const CARAVANA_SALIDA_INMEDIATA: &str = "caravana.salida_inmediata";
    /// 3B-1: mandar al origen una aparcada con carga necesita un viaje (fase 3B-2).
    pub const CARAVANA_REQUIERE_VIAJE: &str = "caravana.requiere_viaje";
    pub const CARAVANA_PLAZA_AJENA: &str = "caravana.plaza_ajena";
    pub const CARAVANA_NO_ESTA_ADJUNTA: &str = "caravana.no_esta_adjunta";
    // Entrega manual (§4.5).
    pub const ENTREGA_INVALIDA: &str = "entrega.invalida";
    pub const ENTREGA_NO_DEBE: &str = "entrega.no_debe";
    pub const ENTREGA_FUERA_DE_LA_PUERTA: &str = "entrega.fuera_de_la_puerta";
    pub const CANTIDAD_NO_POSITIVA: &str = "cantidad.no_positiva";
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
    /// Ejércitos o plazas creados por la orden.
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

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn las_ordenes_de_economia_dan_la_vuelta_por_el_registro() {
        let comandos = vec![
            Comando::AnadirEdificio { asentamiento: 3, tipo: Tipo::Lenera },
            Comando::QuitarDeCola { asentamiento: 3, edificio: 17 },
            Comando::MoverEnCola { asentamiento: 3, edificio: 17, arriba: true },
            Comando::MejorarEdificio { asentamiento: 3, edificio: 4 },
            Comando::AlternarAutoConstruccion { asentamiento: 3, pausada: true },
            Comando::AlternarReceta { asentamiento: 3, recurso: Recurso::LingoteBronce, pausada: false },
            Comando::CalibrarReserva { asentamiento: 3, recurso: Recurso::Madera, valor: 12_345 },
            Comando::SolicitarAscenso { asentamiento: 3 },
            Comando::ActivarPolitica { asentamiento: 3, politica: "racionamiento".into(), cargo: Cargo::Sacerdote },
        ];
        for c in comandos {
            let p = Peticion { actor: "ana".into(), admin: false, peticion_id: "x".into(), version_esperada: Some(9), comando: c };
            assert_eq!(Peticion::decodificar(&p.codificar()).unwrap(), p);
        }
        // Un tipo o un recurso fuera de rango no se decodifica.
        let mut b = Peticion::nueva("a", "i", Comando::AnadirEdificio { asentamiento: 1, tipo: Tipo::Granja }).codificar();
        *b.last_mut().unwrap() = 200;
        assert!(Peticion::decodificar(&b).is_err());
    }

    #[test]
    fn las_ordenes_de_comercio_dan_la_vuelta_por_el_registro() {
        let comandos = vec![
            Comando::ProponerTrueque { a: 3, b: 4, lineas_a: vec![(Recurso::Madera, 100_000), (Recurso::Oro, 5)], lineas_b: vec![(Recurso::Cobre, 30_000)] },
            Comando::AceptarTrueque { acuerdo: 7 },
            Comando::RechazarTrueque { acuerdo: 7 },
            Comando::ColocarOrdenMercado { asentamiento: 3, venta: true, recurso: Recurso::Piedra, cantidad: 9, precio: Some(-4) },
            Comando::ColocarOrdenMercado { asentamiento: 3, venta: false, recurso: Recurso::Piedra, cantidad: 9, precio: None },
            Comando::CancelarOrdenMercado { orden: 2 },
            Comando::ComerciarEnPlaza { ejercito: 1, orden: 2, cantidad: 3_000 },
            Comando::CrearCaravana { asentamiento: 3 },
            Comando::AnadirCarro { caravana: 1, tipo: TipoCarro::Reforzado },
            Comando::AnadirAnimal { caravana: 1, carro: 0, animal: Animal::Camello },
            Comando::MoverCarro { de: 1, a: 2, carro: 1 },
            Comando::ReservarCaravana { caravana: 1, reservada: true },
            Comando::PrepararCaravana { caravana: 1, destino: 4, carga: vec![(Recurso::Madera, 1_000)], salir_en: Some(77) },
            Comando::PrepararCaravana { caravana: 1, destino: 4, carga: vec![], salir_en: None },
            Comando::CancelarCaravana { caravana: 1 },
            Comando::MoverCargaAparcada { caravana: 1, recurso: Recurso::Trigo, cantidad: 5, cargar: false },
            Comando::EnviarCaravanaAlOrigen { caravana: 1 },
            Comando::EntregarDeCaravana { ejercito: 2, caravana: 1, acuerdo: 3 },
            Comando::DarRecursos { asentamiento: 3, recurso: Recurso::Oro, cantidad: 1_000 },
            Comando::DarSuministro { ejercito: 2, recurso: Recurso::Oro, cantidad: 1_000 },
            Comando::AdjuntarCaravana { caravana: 1, ejercito: 2 },
            Comando::AparcarCaravana { caravana: 1, asentamiento: 3 },
        ];
        for c in comandos {
            let p = Peticion { actor: "ana".into(), admin: true, peticion_id: "y".into(), version_esperada: None, comando: c };
            assert_eq!(Peticion::decodificar(&p.codificar()).unwrap(), p);
        }
        // Un tipo de carro fuera de rango no se decodifica.
        let mut b = Peticion::nueva("a", "i", Comando::AnadirCarro { caravana: 1, tipo: TipoCarro::Basico }).codificar();
        *b.last_mut().unwrap() = 9;
        assert!(Peticion::decodificar(&b).is_err());
    }
}
