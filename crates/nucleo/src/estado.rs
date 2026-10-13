//! El estado determinista del mundo: arena de ejércitos, cola de eventos, generaciones, azar, versión y reloj. Es TODO lo
//! que entra en `hash_estado` y en las fotos; lo que no está aquí (sesiones, registro de cambios, métricas) se reconstruye.

use crate::codec::{ErrorFormato, Escritor, Lector};
use base::{Instante, Mil, Plan, Punto, Ritmo, Rng, Vertice};
use cola::{Clave, Cola, Evento, Generaciones};
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, VecDeque};

/// Prioridad de los umbrales de víveres: «consumo de población y de tropa» (Reloj_Y_Eventos §2).
pub const PRIO_UMBRAL: u8 = 3;
/// Prioridad de las llegadas: «llegadas y encuentros de columnas».
pub const PRIO_LLEGADA: u8 = 8;

const MAGIA: &[u8; 4] = b"BAN1";

/// Reglas de juego mínimas del núcleo genérico. Entran en el hash y en las fotos: dos mundos con reglas distintas no se
/// pueden confundir (recuperar con otras reglas es un error).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reglas {
    /// Víveres que gasta una columna en marcha, en milésimas por minuto (≈ 0,83: Movimiento_Ejercitos §6).
    pub consumo_marcha: Mil,
    /// Ídem acampada.
    pub consumo_estacionado: Mil,
    /// Aviso de víveres bajos al caer por debajo de este porcentaje de la capacidad (0 = sin aviso).
    pub viveres_bajos_pct: u32,
    /// Peticiones recordadas para la idempotencia (`INFRA.limiteIdempotencia`).
    pub limite_idempotencia: usize,
}

impl Default for Reglas {
    fn default() -> Self {
        Self { consumo_marcha: 830, consumo_estacionado: 83, viveres_bajos_pct: 25, limite_idempotencia: 500 }
    }
}

/// Una columna mínima: lo justo para demostrar planes (movimiento) y ritmos (víveres).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ejercito {
    pub id: u64,
    /// Héroe que la dirige (`Orden*.heroe_id`).
    pub lider: String,
    /// Cliente (cuenta) al que obedece; solo él (o un administrador) puede darle órdenes.
    pub propietario: String,
    pub faccion: u32,
    /// Milésimas de unidad por segundo, antes del coste del terreno (30 unidades/min = 500).
    pub velocidad: Mil,
    /// Dónde está cuando no tiene plan.
    pub posicion: Punto,
    /// Sin vértices = quieta en `posicion`.
    pub plan: Plan,
    pub viveres: Ritmo,
    pub capacidad_viveres: Mil,
    /// Versión de mundo del último cambio.
    pub version: u64,
}

impl Ejercito {
    pub fn marchando(&self) -> bool {
        !self.plan.vertices.is_empty()
    }
    /// Posición en `t`, evaluada del plan vigente (o la fija si está quieta).
    pub fn posicion_en(&self, t: Instante) -> Punto {
        self.plan.posicion_en(t).unwrap_or(self.posicion)
    }
}

/// Qué ocurre cuando vence un evento de la cola.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Carga {
    /// El plan llegó a su último vértice.
    Llegada,
    /// Los víveres cruzaron este valor (milésimas).
    Umbral(Mil),
    /// Evento de un [`GeneradorDeEventos`](crate::GeneradorDeEventos): `generador` es su posición en la lista.
    Externa { generador: u16, tipo: u16, a: u64, b: u64 },
}

/// Un hecho ocurrido (crónica y avisos), no el cambio de estado: ese viaja en las entidades.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventoNucleo {
    /// `familia.nombre`, como `ejercito.llega`.
    pub codigo: Cow<'static, str>,
    pub en: Instante,
    /// Columnas implicadas: deciden quién lo oye.
    pub ejercitos: Vec<u64>,
    pub punto: Option<Punto>,
    pub datos: Vec<(&'static str, i64)>,
}

/// Registro de cambios de una versión: lo que alimenta `Delta` y `Foto`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CambioVersion {
    pub version: u64,
    pub en: Instante,
    /// Ejércitos cuyo estado cambió (ordenados, sin repetir).
    pub ejercitos: Vec<u64>,
    pub eventos: Vec<EventoNucleo>,
}

pub struct Estado {
    pub reglas: Reglas,
    /// Hash del artefacto de mundo con el que se calcularon las rutas: reproducir sobre otro mundo no tiene sentido.
    pub mundo_hash: String,
    pub version: u64,
    pub ahora: Instante,
    pub factor_milesimas: u32,
    pub pausado: bool,
    pub rng: Rng,
    pub siguiente_id: u64,
    pub ejercitos: BTreeMap<u64, Ejercito>,
    pub generaciones: Generaciones,
    pub cola: Cola<Carga>,
    por_lider: HashMap<String, u64>,
    /// `(actor, peticion_id)` en orden de llegada, con la versión que produjeron.
    vistos: VecDeque<(String, String)>,
    vistos_version: HashMap<(String, String), u64>,
}

impl Estado {
    pub fn nuevo(reglas: Reglas, mundo_hash: String, semilla: u64, ahora: Instante) -> Self {
        Self {
            reglas,
            mundo_hash,
            version: 0,
            ahora,
            factor_milesimas: 1000,
            pausado: false,
            rng: Rng::derivado(semilla, "nucleo"),
            siguiente_id: 1,
            ejercitos: BTreeMap::new(),
            generaciones: Generaciones::nuevas(),
            cola: Cola::nueva(),
            por_lider: HashMap::new(),
            vistos: VecDeque::new(),
            vistos_version: HashMap::new(),
        }
    }

    pub fn ejercito_de_lider(&self, lider: &str) -> Option<u64> {
        self.por_lider.get(lider).copied()
    }

    pub(crate) fn insertar(&mut self, e: Ejercito) {
        self.por_lider.insert(e.lider.clone(), e.id);
        self.ejercitos.insert(e.id, e);
    }

    /// Versión que produjo una petición ya aceptada.
    pub fn visto(&self, actor: &str, peticion_id: &str) -> Option<u64> {
        self.vistos_version.get(&(actor.to_owned(), peticion_id.to_owned())).copied()
    }

    pub(crate) fn recordar(&mut self, actor: &str, peticion_id: &str, version: u64) {
        let clave = (actor.to_owned(), peticion_id.to_owned());
        if self.vistos_version.insert(clave.clone(), version).is_none() {
            self.vistos.push_back(clave);
        }
        while self.vistos.len() > self.reglas.limite_idempotencia {
            if let Some(vieja) = self.vistos.pop_front() {
                self.vistos_version.remove(&vieja);
            }
        }
    }

    /// Estado canónico en bytes: orden fijo (ejércitos por id, cola por `(instante, prioridad, secuencia)`), solo eventos
    /// vigentes (los invalidados que aún no salieron de la cola no son estado: depende de cuándo se descartaron).
    pub fn serializar(&self) -> Vec<u8> {
        let mut w = Escritor::default();
        w.0.extend_from_slice(MAGIA);
        w.texto(&self.mundo_hash);
        let r = &self.reglas;
        w.i64(r.consumo_marcha);
        w.i64(r.consumo_estacionado);
        w.u32(r.viveres_bajos_pct);
        w.u64(r.limite_idempotencia as u64);
        w.u64(self.version);
        w.i64(self.ahora);
        w.u32(self.factor_milesimas);
        w.bool(self.pausado);
        w.u64(self.rng.estado());
        w.u64(self.siguiente_id);

        w.u64(self.ejercitos.len() as u64);
        for e in self.ejercitos.values() {
            w.u64(e.id);
            w.texto(&e.lider);
            w.texto(&e.propietario);
            w.u32(e.faccion);
            w.i64(e.velocidad);
            w.punto(e.posicion);
            w.u32(e.plan.generacion);
            w.u32(e.plan.vertices.len() as u32);
            for v in &e.plan.vertices {
                w.punto(v.p);
                w.i64(v.en);
            }
            let v = &e.viveres;
            w.i64(v.valor_en_t0);
            w.i64(v.t0);
            w.i64(v.por_minuto);
            w.opt_i64(v.maximo);
            w.opt_i64(v.minimo);
            w.i64(e.capacidad_viveres);
            w.u64(e.version);
        }

        let gens = self.generaciones.volcar();
        w.u64(gens.len() as u64);
        for (id, g) in gens {
            w.u64(id);
            w.u32(g);
        }

        let eventos: Vec<_> = self.cola.volcar().into_iter().filter(|e| self.generaciones.es_vigente(e.clave)).collect();
        w.u64(self.cola.siguiente_secuencia());
        w.u64(eventos.len() as u64);
        for e in &eventos {
            w.i64(e.instante);
            w.u8(e.prioridad);
            w.u64(e.secuencia);
            w.u64(e.clave.entidad);
            w.u32(e.clave.generacion);
            match &e.carga {
                Carga::Llegada => w.u8(0),
                Carga::Umbral(v) => {
                    w.u8(1);
                    w.i64(*v);
                }
                Carga::Externa { generador, tipo, a, b } => {
                    w.u8(2);
                    w.u16(*generador);
                    w.u16(*tipo);
                    w.u64(*a);
                    w.u64(*b);
                }
            }
        }

        w.u64(self.vistos.len() as u64);
        for clave in &self.vistos {
            w.texto(&clave.0);
            w.texto(&clave.1);
            w.u64(self.vistos_version[clave]);
        }
        w.0
    }

    pub fn hash(&self) -> [u8; 32] {
        Sha256::digest(self.serializar()).into()
    }

    /// Inversa de [`Estado::serializar`]. Falla si las reglas o el mundo de la foto no son los que se piden.
    pub fn deserializar(bytes: &[u8], reglas: &Reglas, mundo_hash: &str) -> Result<Estado, ErrorFormato> {
        let mut r = Lector::nuevo(bytes);
        if r.magia()? != *MAGIA {
            return Err(ErrorFormato("no es un estado del núcleo"));
        }
        if r.texto()? != mundo_hash {
            return Err(ErrorFormato("la foto es de otro mundo (hash del artefacto distinto)"));
        }
        let guardadas = Reglas {
            consumo_marcha: r.i64()?,
            consumo_estacionado: r.i64()?,
            viveres_bajos_pct: r.u32()?,
            limite_idempotencia: r.u64()? as usize,
        };
        if &guardadas != reglas {
            return Err(ErrorFormato("las reglas de la foto no coinciden con las pedidas"));
        }
        let mut s = Estado::nuevo(guardadas, mundo_hash.to_owned(), 0, 0);
        s.version = r.u64()?;
        s.ahora = r.i64()?;
        s.factor_milesimas = r.u32()?;
        s.pausado = r.bool()?;
        s.rng = Rng::nuevo(r.u64()?);
        s.siguiente_id = r.u64()?;

        for _ in 0..r.cuenta()? {
            let (id, lider, propietario, faccion, velocidad) = (r.u64()?, r.texto()?, r.texto()?, r.u32()?, r.i64()?);
            let posicion = r.punto()?;
            let generacion = r.u32()?;
            let mut vertices = Vec::new();
            for _ in 0..r.u32()? {
                vertices.push(Vertice { p: r.punto()?, en: r.i64()? });
            }
            let viveres = Ritmo {
                valor_en_t0: r.i64()?,
                t0: r.i64()?,
                por_minuto: r.i64()?,
                maximo: r.opt_i64()?,
                minimo: r.opt_i64()?,
            };
            let (capacidad_viveres, version) = (r.i64()?, r.u64()?);
            s.insertar(Ejercito {
                id,
                lider,
                propietario,
                faccion,
                velocidad,
                posicion,
                plan: Plan { generacion, vertices },
                viveres,
                capacidad_viveres,
                version,
            });
        }

        let mut gens = Vec::new();
        for _ in 0..r.cuenta()? {
            gens.push((r.u64()?, r.u32()?));
        }
        s.generaciones = Generaciones::restaurar(gens);

        let siguiente_secuencia = r.u64()?;
        let mut eventos = Vec::new();
        for _ in 0..r.cuenta()? {
            let (instante, prioridad, secuencia) = (r.i64()?, r.u8()?, r.u64()?);
            let clave = Clave { entidad: r.u64()?, generacion: r.u32()? };
            let carga = match r.u8()? {
                0 => Carga::Llegada,
                1 => Carga::Umbral(r.i64()?),
                2 => Carga::Externa { generador: r.u16()?, tipo: r.u16()?, a: r.u64()?, b: r.u64()? },
                _ => return Err(ErrorFormato("tipo de carga desconocido")),
            };
            eventos.push(Evento { instante, prioridad, secuencia, clave, carga });
        }
        s.cola = Cola::restaurar(eventos, siguiente_secuencia);

        for _ in 0..r.cuenta()? {
            let (actor, peticion, version) = (r.texto()?, r.texto()?, r.u64()?);
            s.recordar(&actor, &peticion, version);
        }
        r.fin()?;
        Ok(s)
    }
}
