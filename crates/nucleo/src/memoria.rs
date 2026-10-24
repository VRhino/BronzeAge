//! La memoria de la niebla (Niebla_De_Guerra §2, §4 y §6, 4H T3d): lo que cada Facción ha explorado, la última ficha de cada sitio ajeno que vio, lo que explora un héroe sin bandera, hasta
//! dónde tiene grabado cada columna lo que ve, y la intel de las tabernas (las Miradas compradas y el último Informe de cada plaza). Es estado de juego (lo explorado decide qué se proyecta;
//! las fichas y las Miradas, qué se conoce y qué se puede comprar): entra en el hash y en la foto (`BAN29`).
//!
//! Lo graba el núcleo al cerrar cada versión (`nucleo/exploracion.rs`), nunca un muestreo: una columna barre su recorrido exacto desde su último [`Vigia`] cuando algo la toca, y lo que
//! lleva andado desde entonces se deriva de su plan (la proyección lo suma). Así la memoria es la misma con sesiones y sin ellas y no depende de cada cuánto se mire.
//!
//! Codificación (little endian, la de `codec`): `rejilla{hay bool · columnas u64 · filas u64 · tamaño i64} · exploración{n · n × (facción u32, bytes)} · personal{n · n × (héroe u64, bytes)} ·
//! fichas{n · n × (facción u32 · m · m × (sitio, datos, conocido_en i64))} · vigías{n · n × (columna u64, desde i64, radio i64)} · sitios{n · n × (sitio, desde i64)}`, con
//! `sitio = (clase u8: 0 plaza, 1 pueblo, 2 avanzada; id u64)` y `datos = (facción u32, posición, nivel u32, radio i64, estado u8: 0 activo, 1 en obra, 2 en ruinas)`; detrás, `miradas{n · n × (id u64, facción u32, origen, centro, radio i64,
//! comprada_en i64, expira_en i64, libre_en i64)} · informes{n · n × (facción u32 · m · m × (plaza u64, nivel u32, posición, conocido_en i64, edificios{k · k × (tipo u8, en_obra bool, nivel u8,
//! hay bool, sitio)}, defensa))}`, con `origen = (0 plaza u64 | 1 campamento u32)`, el sitio de un edificio en milésimas de unidad local y la defensa como el contenido de un informe de inspección.

use crate::codec::{ErrorFormato, Escritor, Lector};
use crate::codec_economia::{tipo_a_u8, tipo_de_u8};
use crate::codec_militar::{contenido_escribir, contenido_leer};
use asentamiento::catalogo::Tipo;
use base::{Instante, Mil, Punto};
use militar::inspeccion::Contenido;
use std::collections::BTreeMap;
use vision::Mascara;

/// Un sitio del mapa que se recuerda «como una ciudad» (§4.1): una plaza, un pueblo extractor o una avanzada.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Sitio {
    Plaza(u64),
    Pueblo(u64),
    Avanzada(u64),
}

/// Cómo estaba un sitio cuando se vio (Niebla_De_Guerra §4.1, decisión del autor del 2026-10-24): en uso, en obra (un pueblo o una avanzada que se levanta, o una avanzada que se mejora o se
/// repara) o en ruinas (una avanzada arruinada).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EstadoSitio {
    Activo,
    EnObra,
    EnRuinas,
}

/// Lo que se ve de un sitio desde fuera y se guarda en su ficha: de quién es (0: de nadie, las ruinas de una ciudadela sin dueño), dónde está, su nivel (la etapa de una avanzada; 0 en un pueblo),
/// su radio (el de la zona de una plaza; 0 en los demás) y su estado.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DatosSitio {
    pub faccion: u32,
    pub posicion: Punto,
    pub nivel: u32,
    pub radio: Mil,
    pub estado: EstadoSitio,
}

/// La última foto de un sitio ajeno (`FichaConocida`): sus datos de cuando se vio por última vez y ese instante.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ficha {
    pub datos: DatosSitio,
    pub conocido_en: Instante,
}

/// Hasta dónde tiene grabado una columna lo que ve: lo que anduvo antes de `desde` ya está en la memoria; lo de después se barre con `radio` (su alcance de entonces) la próxima vez que algo la toque.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Vigia {
    pub desde: Instante,
    pub radio: Mil,
}

/// Dónde se compra la intel: la taberna de una plaza o la de un campamento de mercenarios (Niebla_De_Guerra §6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrigenIntel {
    Plaza(u64),
    Campamento(u32),
}

/// Una Mirada comprada en una taberna (§6.1): un ojo prestado de `radio` sobre `centro` mientras `ahora < expira_en`. No explora ni deja memoria; su zona no se puede volver a mirar hasta `libre_en`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mirada {
    pub id: u64,
    pub faccion: u32,
    pub origen: OrigenIntel,
    pub centro: Punto,
    pub radio: Mil,
    pub comprada_en: Instante,
    pub expira_en: Instante,
    pub libre_en: Instante,
}

impl Mirada {
    /// ¿Sigue abierto el ojo?
    pub fn activa(&self, t: Instante) -> bool {
        self.expira_en > t
    }
}

/// Un edificio de una plaza ajena tal como lo trae un Informe: su tipo, si está levantado o en obra, su nivel interno y su sitio en la ciudad (milésimas de unidad local, el centro de su huella).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EdificioVisto {
    pub tipo: Tipo,
    pub en_obra: bool,
    pub nivel_interno: u8,
    pub sitio: Option<Punto>,
}

/// El último Informe de plaza que compró una Facción (§6.2): la foto, fechada, del trazado y la defensa de una plaza ajena. Nunca su almacén, sus colas, sus cargos ni sus políticas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InformeDePlaza {
    pub nivel: u32,
    pub posicion: Punto,
    pub conocido_en: Instante,
    /// Los edificios levantados y en obra (no los de la cola).
    pub edificios: Vec<EdificioVisto>,
    /// La defensa: su Facción, quiénes estaban dentro, la guarnición y los recintos de muralla (lo mismo que ve `inspeccionar`, `Contenido::Plaza`).
    pub defensa: Contenido,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Memoria {
    /// La rejilla de lo explorado: columnas, filas y lado de la celda (de los límites del mapa y `EXPLORACION.tamanoCelda`). `None` sin artefacto de mundo: no se graba nada.
    pub rejilla: Option<(usize, usize, Mil)>,
    /// Lo explorado por cada Facción (solo crece). Una ausente no ha visto nada.
    pub exploracion: BTreeMap<u32, Mascara>,
    /// Lo explorado por un héroe sin bandera (`exploracionPersonal`, §4.2): se funde con su Facción al entrar en una.
    pub personal: BTreeMap<u64, Mascara>,
    /// La última ficha de cada sitio ajeno que vio cada Facción (§4.1). Los propios no tienen ficha.
    pub fichas: BTreeMap<u32, BTreeMap<Sitio, Ficha>>,
    /// Hasta dónde tiene grabado cada columna lo que ve.
    pub vigias: BTreeMap<u64, Vigia>,
    /// Desde cuándo vale lo que se ve de cada sitio (sus [`DatosSitio`]): lo que una columna vio antes de ese instante era otra ficha, o nada, y ya está grabado.
    pub sitios: BTreeMap<Sitio, Instante>,
    /// Las Miradas de todas las Facciones que aún enfriaban su zona en la última compra (`libre_en` posterior a ella), en el orden en que se compraron.
    pub miradas: Vec<Mirada>,
    /// El último Informe de cada plaza ajena que compró cada Facción, por plaza.
    pub informes: BTreeMap<u32, BTreeMap<u64, InformeDePlaza>>,
}

impl Memoria {
    /// Una máscara vacía de la rejilla (`None` sin rejilla).
    pub fn vacia(&self) -> Option<Mascara> {
        self.rejilla.map(|(c, f, t)| Mascara::nueva(c, f, t))
    }
}

fn sitio_escribir(w: &mut Escritor, s: Sitio) {
    let (clase, id) = match s {
        Sitio::Plaza(id) => (0, id),
        Sitio::Pueblo(id) => (1, id),
        Sitio::Avanzada(id) => (2, id),
    };
    w.u8(clase);
    w.u64(id);
}

fn sitio_leer(r: &mut Lector) -> Result<Sitio, ErrorFormato> {
    let (clase, id) = (r.u8()?, r.u64()?);
    Ok(match clase {
        0 => Sitio::Plaza(id),
        1 => Sitio::Pueblo(id),
        2 => Sitio::Avanzada(id),
        _ => return Err(ErrorFormato("sitio de una ficha desconocido")),
    })
}

pub(crate) fn escribir(w: &mut Escritor, m: &Memoria) {
    let Memoria { rejilla, exploracion, personal, fichas, vigias, sitios, miradas, informes } = m;
    w.bool(rejilla.is_some());
    let (c, f, t) = rejilla.unwrap_or_default();
    w.u64(c as u64);
    w.u64(f as u64);
    w.i64(t);
    w.u64(exploracion.len() as u64);
    for (&faccion, mascara) in exploracion {
        w.u32(faccion);
        w.bytes(mascara.bytes());
    }
    w.u64(personal.len() as u64);
    for (&heroe, mascara) in personal {
        w.u64(heroe);
        w.bytes(mascara.bytes());
    }
    w.u64(fichas.len() as u64);
    for (&faccion, de) in fichas {
        w.u32(faccion);
        w.u64(de.len() as u64);
        for (&s, ficha) in de {
            sitio_escribir(w, s);
            let Ficha { datos: DatosSitio { faccion, posicion, nivel, radio, estado }, conocido_en } = *ficha;
            w.u32(faccion);
            w.punto(posicion);
            w.u32(nivel);
            w.i64(radio);
            w.u8(estado as u8);
            w.i64(conocido_en);
        }
    }
    w.u64(vigias.len() as u64);
    for (&columna, v) in vigias {
        w.u64(columna);
        w.i64(v.desde);
        w.i64(v.radio);
    }
    w.u64(sitios.len() as u64);
    for (&s, &desde) in sitios {
        sitio_escribir(w, s);
        w.i64(desde);
    }
    w.u64(miradas.len() as u64);
    for &Mirada { id, faccion, origen, centro, radio, comprada_en, expira_en, libre_en } in miradas {
        w.u64(id);
        w.u32(faccion);
        match origen {
            OrigenIntel::Plaza(a) => {
                w.u8(0);
                w.u64(a);
            }
            OrigenIntel::Campamento(c) => {
                w.u8(1);
                w.u32(c);
            }
        }
        w.punto(centro);
        [radio, comprada_en, expira_en, libre_en].into_iter().for_each(|x| w.i64(x));
    }
    w.u64(informes.len() as u64);
    for (&faccion, de) in informes {
        w.u32(faccion);
        w.u64(de.len() as u64);
        for (&plaza, InformeDePlaza { nivel, posicion, conocido_en, edificios, defensa }) in de {
            w.u64(plaza);
            w.u32(*nivel);
            w.punto(*posicion);
            w.i64(*conocido_en);
            w.u64(edificios.len() as u64);
            for &EdificioVisto { tipo, en_obra, nivel_interno, sitio } in edificios {
                w.u8(tipo_a_u8(tipo));
                w.bool(en_obra);
                w.u8(nivel_interno);
                w.bool(sitio.is_some());
                w.punto(sitio.unwrap_or(Punto::new(0, 0)));
            }
            contenido_escribir(w, defensa);
        }
    }
}

pub(crate) fn leer(r: &mut Lector) -> Result<Memoria, ErrorFormato> {
    let hay = r.bool()?;
    let (c, f, t) = (r.u64()? as usize, r.u64()? as usize, r.i64()?);
    if hay && t <= 0 {
        return Err(ErrorFormato("rejilla de la niebla con celdas de lado no positivo"));
    }
    let rejilla = hay.then_some((c, f, t));
    let mascara = |r: &mut Lector| -> Result<Mascara, ErrorFormato> {
        let bytes = r.bytes()?;
        Ok(Mascara::desde_bytes(c, f, t.max(1), bytes))
    };
    let mut m = Memoria { rejilla, ..Memoria::default() };
    for _ in 0..r.cuenta()? {
        let faccion = r.u32()?;
        m.exploracion.insert(faccion, mascara(r)?);
    }
    for _ in 0..r.cuenta()? {
        let heroe = r.u64()?;
        m.personal.insert(heroe, mascara(r)?);
    }
    for _ in 0..r.cuenta()? {
        let faccion = r.u32()?;
        let de = m.fichas.entry(faccion).or_default();
        for _ in 0..r.cuenta()? {
            let s = sitio_leer(r)?;
            let (faccion, posicion, nivel, radio) = (r.u32()?, r.punto()?, r.u32()?, r.i64()?);
            let estado = match r.u8()? {
                0 => EstadoSitio::Activo,
                1 => EstadoSitio::EnObra,
                2 => EstadoSitio::EnRuinas,
                _ => return Err(ErrorFormato("estado de una ficha desconocido")),
            };
            let datos = DatosSitio { faccion, posicion, nivel, radio, estado };
            de.insert(s, Ficha { datos, conocido_en: r.i64()? });
        }
    }
    for _ in 0..r.cuenta()? {
        let columna = r.u64()?;
        m.vigias.insert(columna, Vigia { desde: r.i64()?, radio: r.i64()? });
    }
    for _ in 0..r.cuenta()? {
        let s = sitio_leer(r)?;
        m.sitios.insert(s, r.i64()?);
    }
    for _ in 0..r.cuenta()? {
        let (id, faccion) = (r.u64()?, r.u32()?);
        let origen = match r.u8()? {
            0 => OrigenIntel::Plaza(r.u64()?),
            1 => OrigenIntel::Campamento(r.u32()?),
            _ => return Err(ErrorFormato("origen de una Mirada desconocido")),
        };
        m.miradas.push(Mirada { id, faccion, origen, centro: r.punto()?, radio: r.i64()?, comprada_en: r.i64()?, expira_en: r.i64()?, libre_en: r.i64()? });
    }
    for _ in 0..r.cuenta()? {
        let faccion = r.u32()?;
        let de = m.informes.entry(faccion).or_default();
        for _ in 0..r.cuenta()? {
            let (plaza, nivel, posicion, conocido_en) = (r.u64()?, r.u32()?, r.punto()?, r.i64()?);
            let mut edificios = Vec::new();
            for _ in 0..r.cuenta()? {
                let (tipo, en_obra, nivel_interno, hay) = (tipo_de_u8(r.u8()?)?, r.bool()?, r.u8()?, r.bool()?);
                let sitio = r.punto()?;
                edificios.push(EdificioVisto { tipo, en_obra, nivel_interno, sitio: hay.then_some(sitio) });
            }
            de.insert(plaza, InformeDePlaza { nivel, posicion, conocido_en, edificios, defensa: contenido_leer(r)? });
        }
    }
    Ok(m)
}
