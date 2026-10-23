//! El trazado urbano dentro del núcleo (`Docs/Mechanics/Trazado_Urbano.md`, T): cada plaza lleva su [`Urbe`] —la
//! [`trazado::Ciudad`] con sus piezas— y coloca sus edificios en ella de verdad. El crate `trazado` es geometría pura; aquí está lo que
//! lo une a la economía:
//!
//! - [`Urbanismo`]: la tabla de tipos → huella y categoría. Las huellas salen de los bloques `EDIFICIO_TAMANO`,
//!   `EDIFICIO_TAMANO_POR_DEFECTO`, `PUESTO_MERCADO_FORMA` y `MERCADO_PUESTOS_POR_NIVEL` del documento (incrustado con `include_str!`,
//!   como las cifras de la economía) y, para la Granja, del `tamano` de cada nivel de `EDIFICIO_CATALOGO`; la categoría y el despacho
//!   de cada tipo son T §6.1. La identidad de las reglas de las fotos incluye estos bloques.
//! - [`Urbe`]: la ciudad de una plaza, con el caché de «¿cabe?» que evita recalcular la red y barrer sitios en cada minuto.
//! - [`SitiosUrbe`]: el adaptador de [`Sitios`] solo con la ciudad (sin yacimientos); lo componen las plazas del núcleo
//!   ([`crate::economia::SitiosPlaza`]) y lo usan las pruebas.
//!
//! # Semilla de cada ciudad
//! `semilla = trazado::hash(semilla_del_núcleo, "ciudad", [id_de_la_plaza])`: determinista, distinta por plaza y por mundo. Se guarda en
//! la foto con la ciudad, así que recuperar no necesita la semilla del núcleo.
//!
//! # Espacios separados
//! La ciudad (espacio local, celdas de 3 u.l.) y la zona de la plaza (mapa) no se mezclan (Docs/Game 1.0a). El radio de la ciudad crece
//! con los edificios completados y el nivel ([`trazado::radio_ciudad`]); la zona del mapa (`Asentamiento.radio`) sigue su propia regla.

use asentamiento::asentamiento::{Gratuito, Sitios, Urbano};
use asentamiento::catalogo::{Tipo, TIPOS};
use asentamiento::datos::bloque_de;
use asentamiento::edificio::Fuente;
use base::Punto;
use std::collections::BTreeMap;
use trazado::{fusionar_celdas, radio_ciudad, Ancla, Categoria, CeldaMuro, ClaseCelda, Ciudad, Colocacion, EntradaFundacion, Huella, OpcionAncla, Parametros, ParametrosMuralla, Pedido, Perfil, Recinto, Rect, RectLocal, Rol, TipoPieza, CELDA_UL};

/// El documento del Trazado urbano: de él salen las huellas.
pub const DOC_TRAZADO: &str = include_str!("../../../Docs/Mechanics/Trazado_Urbano.md");

/// Cómo se coloca un tipo de edificio: su categoría de barrio, su papel respecto a las anclas y si puede girar (T §6.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clase {
    pub categoria: Categoria,
    pub rol: Rol,
    /// La Granja nunca gira (T §6.2); todo lo demás sí.
    pub rota: bool,
}

/// Las huellas y las clases de todos los tipos.
#[derive(Debug)]
pub struct Urbanismo {
    defecto: Huella,
    por_tipo: BTreeMap<Tipo, Huella>,
    forma_puesto: BTreeMap<u8, Huella>,
    puestos_por_nivel: BTreeMap<u8, Vec<u8>>,
    /// Huella de la Granja por nivel interno (índice = nivel − 1).
    granja: Vec<Huella>,
    /// `RADIO_CIUDAD` (T §12): inicial, crecimiento por edificio y tope por nivel, en unidades locales.
    radio: (i32, i32, [i32; 5]),
}

fn huella_de(v: &serde_json::Value) -> Huella {
    let n = |k: &str| v[k].as_i64().unwrap_or_else(|| panic!("falta «{k}» en una huella")) as i32;
    Huella::new(n("ancho"), n("alto"))
}

impl Urbanismo {
    /// Lee los bloques del documento incrustado (parsea JSON: hágase una vez). Un dato que falte es un error de programación.
    pub fn cargar(eco: &asentamiento::Reglas) -> Self {
        let b = |n: &str| bloque_de(DOC_TRAZADO, n).unwrap_or_else(|e| panic!("{e}"));
        let defecto = huella_de(&b("EDIFICIO_TAMANO_POR_DEFECTO"));
        let por_tipo = b("EDIFICIO_TAMANO")
            .as_object()
            .expect("EDIFICIO_TAMANO es un objeto")
            .iter()
            .map(|(k, v)| (Tipo::desde_nombre(k).unwrap_or_else(|| panic!("tipo «{k}» en EDIFICIO_TAMANO")), huella_de(v)))
            .collect();
        let forma_puesto = b("PUESTO_MERCADO_FORMA").as_object().expect("objeto").iter().map(|(k, v)| (k.parse().expect("forma numérica"), huella_de(v))).collect();
        let puestos_por_nivel = b("MERCADO_PUESTOS_POR_NIVEL")
            .as_object()
            .expect("objeto")
            .iter()
            .map(|(k, v)| (k.parse().expect("nivel numérico"), v.as_array().expect("lista").iter().map(|f| f.as_u64().expect("forma") as u8).collect()))
            .collect();
        let granja = eco.catalogo.tipo(Tipo::Granja).niveles.iter().map(|n| n.tamano.map_or(defecto, |(a, h)| Huella::new(a, h))).collect();
        let rc = b("RADIO_CIUDAD");
        let ul = |v: &serde_json::Value| v.as_i64().expect("RADIO_CIUDAD en unidades locales enteras") as i32;
        let topes: Vec<i32> = rc["maximoPorNivel"].as_array().expect("maximoPorNivel es una lista").iter().map(ul).collect();
        let radio = (ul(&rc["inicial"]), ul(&rc["crecimientoPorEdificio"]), topes.try_into().expect("maximoPorNivel: un tope por nivel 1-5"));
        Self { defecto, por_tipo, forma_puesto, puestos_por_nivel, granja, radio }
    }

    /// Huella sin girar de un tipo a ese nivel interno: la Granja por nivel, el puesto del Mercado por su forma, el resto por tipo.
    pub fn huella(&self, tipo: Tipo, nivel_interno: u8) -> Huella {
        match tipo {
            Tipo::Granja if !self.granja.is_empty() => self.granja[(nivel_interno.max(1) as usize - 1).min(self.granja.len() - 1)],
            Tipo::PuestoMercado => self.forma_puesto.get(&nivel_interno).copied().unwrap_or(self.defecto),
            _ => self.por_tipo.get(&tipo).copied().unwrap_or(self.defecto),
        }
    }

    /// Las formas de los puestos que se añaden al llegar el Mercado a ese nivel (T §2.4, acumulativo).
    pub fn puestos(&self, nivel_mercado: u8) -> &[u8] {
        self.puestos_por_nivel.get(&nivel_mercado).map_or(&[], Vec::as_slice)
    }

    /// Categoría, papel y rotación de un tipo (T §6.1). `None` = no participa de la rejilla de la ciudad (ámbito de mapa: la Cantera y las
    /// minas, que solo ocupan su yacimiento).
    pub fn clase(&self, tipo: Tipo) -> Option<Clase> {
        use Categoria::*;
        let (categoria, rol, rota) = match tipo {
            Tipo::CentroUrbano => (Residencial, Rol::Centro, false),
            Tipo::Granja => (Afueras, Rol::Normal, false),
            Tipo::Corral => (Afueras, Rol::Normal, true),
            Tipo::Palacio | Tipo::Almacen | Tipo::Granero | Tipo::Lenera | Tipo::Taberna | Tipo::SalaConsejo => (Generico, Rol::Normal, true),
            Tipo::Vivienda => (Residencial, Rol::Normal, true),
            Tipo::Fundicion | Tipo::Curtiduria | Tipo::Armeria | Tipo::GranFundicion | Tipo::Maravilla => (Industria, Rol::Normal, true),
            Tipo::Barracon | Tipo::GaleriaDeTiro | Tipo::Caballerizas | Tipo::Carpinteria => (Militar, Rol::Normal, true),
            Tipo::Mercado => (Mercado, Rol::AnclaPrimaria, true),
            Tipo::PuestoMercado => (Mercado, Rol::Normal, true),
            Tipo::Plaza | Tipo::Pozo | Tipo::Parque => (Residencial, Rol::AnclaSaturacion, true),
            Tipo::PlazaDeArmas => (Militar, Rol::AnclaSaturacion, true),
            Tipo::PatioDeGremios => (Industria, Rol::AnclaSaturacion, true),
            Tipo::Cantera | Tipo::Mina | Tipo::MinaCobre | Tipo::MinaEstano | Tipo::MinaHierro => return None,
        };
        Some(Clase { categoria, rol, rota })
    }

    /// Los tipos de ancla de saturación de una categoría, en el orden de T §7.1 (el sorteo indexa en él).
    fn opciones_ancla(&self, c: Categoria) -> Vec<OpcionAncla> {
        let tipos: &[Tipo] = match c {
            Categoria::Residencial => &[Tipo::Plaza, Tipo::Pozo, Tipo::Parque],
            Categoria::Militar => &[Tipo::PlazaDeArmas],
            Categoria::Industria => &[Tipo::PatioDeGremios],
            _ => &[],
        };
        tipos.iter().map(|&t| OpcionAncla { tipo: tipo_a_u16(t), huella: self.huella(t, 1) }).collect()
    }

    /// Los parámetros de la geometría con los valores de la economía que los comparten (el gate de las anclas y las Viviendas de la
    /// fundación) y el `RADIO_CIUDAD` del documento; el resto, los de T §12.
    pub fn parametros(&self, eco: &asentamiento::Reglas) -> Parametros {
        let (radio_inicial_ul, crecimiento_por_edificio_ul, maximo_por_nivel_ul) = self.radio;
        Parametros {
            nivel_gate_anclas: eco.construccion.necesidades.nivel_gate_anclas as u8,
            viviendas_iniciales: eco.viviendas_iniciales,
            radio_inicial_ul,
            crecimiento_por_edificio_ul,
            maximo_por_nivel_ul,
            ..Parametros::default()
        }
    }
}

pub(crate) fn tipo_a_u16(t: Tipo) -> u16 {
    t as u8 as u16
}

fn perfil_de(nombre: &str) -> Option<Perfil> {
    Some(match nombre {
        "nucleos" => Perfil::Nucleos,
        "caminera" => Perfil::Caminera,
        "compacta" => Perfil::Compacta,
        "gremial" => Perfil::Gremial,
        _ => return None,
    })
}

/// `semilla` de la ciudad de una plaza (ver la cabecera del módulo).
pub fn semilla_de_ciudad(semilla_nucleo: u64, id_plaza: u64) -> u64 {
    trazado::hash(semilla_nucleo, "ciudad", &[id_plaza as i64])
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Consulta {
    Cabe(Tipo),
    Mudar(u32, u8),
}

/// De lo que depende una respuesta del caché, **solo lo que de verdad le afecta** (para que no se invalide sin motivo): la revisión de la
/// ciudad; el radio, que solo importa si queda por debajo del de las afueras para los campos y siempre para el resto; el id que tendría el
/// próximo edificio y el gate de nivel, que solo cuentan donde el trazado puede crear un ancla.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Clave {
    /// Revisión de la ciudad (cada colocación, mudanza, baja o cambio de recintos la sube).
    rev: u32,
    radio: i32,
    /// 0 donde no se crea ancla.
    siguiente_id: u32,
    /// Con el nivel operativo en el gate o por encima (solo donde se crea ancla).
    gate_abierto: bool,
    perfil: Perfil,
}

/// «¿Hay sitio para esto?» por tipo (o «¿cabe esta mejora?» por edificio), válido mientras no cambie nada de lo que ve el trazado.
/// Es un derivado: nunca entra en el hash ni en las fotos, y no cambia ningún resultado (solo evita repetirlo).
#[derive(Clone, Debug, Default)]
struct Cache {
    filas: Vec<(Consulta, Clave, bool)>,
}

impl Cache {
    fn buscar(&self, c: Consulta, k: Clave) -> Option<bool> {
        self.filas.iter().find(|(q, kq, _)| *q == c && *kq == k).map(|&(_, _, r)| r)
    }
    fn poner(&mut self, c: Consulta, k: Clave, r: bool) {
        match self.filas.iter_mut().find(|(q, ..)| *q == c) {
            Some(f) => *f = (c, k, r),
            None => self.filas.push((c, k, r)),
        }
    }
}

/// La ciudad de una plaza: el estado persistible ([`Ciudad`]) más el caché de consultas.
#[derive(Clone, Debug)]
pub struct Urbe {
    ciudad: Ciudad,
    rev: u32,
    cache: Cache,
}

/// La igualdad es la de la ciudad: el caché y la revisión son derivados.
impl PartialEq for Urbe {
    fn eq(&self, o: &Self) -> bool {
        self.ciudad == o.ciudad
    }
}
impl Eq for Urbe {}

impl Urbe {
    /// La ciudad de una plaza recién fundada: Centro Urbano (id 0), las Viviendas iniciales y la Granja inicial, con los mismos ids que
    /// los edificios iniciales de la economía (`Asentamiento::fundar`).
    ///
    /// DUDA: si la Granja inicial no cupiera, el trazado no la coloca (T §11 la dejaría encima del Centro Urbano) y la economía tendría una
    /// Granja sin pieza: la consulta de la ciudad la ignora y su mejora no la muda. En una ciudad vacía de 288 u.l. no ocurre.
    pub fn fundar(reglas: &crate::ReglasEconomia, semilla_nucleo: u64, id_plaza: u64) -> Self {
        let (eco, urb) = (&reglas.datos, &reglas.urbanismo);
        let semilla = semilla_de_ciudad(semilla_nucleo, id_plaza);
        let pieza = |t: Tipo| TipoPieza { tipo: tipo_a_u16(t), huella: urb.huella(t, 1) };
        let entrada = EntradaFundacion {
            semilla,
            params: urb.parametros(eco),
            perfil: Perfil::por_tradicion(semilla),
            primer_id: 0,
            centro: pieza(Tipo::CentroUrbano),
            vivienda: pieza(Tipo::Vivienda),
            granja: pieza(Tipo::Granja),
        };
        Self::desde_ciudad(Ciudad::fundar(&entrada))
    }

    pub fn desde_ciudad(ciudad: Ciudad) -> Self {
        Self { ciudad, rev: 0, cache: Cache::default() }
    }

    pub fn ciudad(&self) -> &Ciudad {
        &self.ciudad
    }

    /// Reemplaza los recintos de muralla (las celdas de muro bloquean y se recalcula la red entera: es caro, así que solo cuando la proyección cambia). Lo que
    /// recibe es **derivado** de los recintos de la economía de la plaza (`asentamiento::recintos::proyeccion`): la ciudad no los guarda en la foto.
    pub fn set_recintos(&mut self, recintos: Vec<Recinto>) {
        self.ciudad.set_recintos(recintos);
        self.rev += 1;
    }

    /// Vuelve a marcar las torres de un anillo ya trazado para `nivel`, con el interior que le da esta ciudad (Murallas §3). Sin Centro Urbano, o si el anillo lo
    /// pisa, no cambia nada.
    pub fn reclasificar_torres(&self, celdas: &mut [CeldaMuro], nivel: u8, p: &ParametrosMuralla) {
        if let Some(interior) = trazado::interior_de(&self.ciudad, celdas) {
            trazado::marcar_torres(celdas, &interior, p, nivel);
        }
    }

    /// Un edificio sale de la cola sin haberse levantado: su pieza desaparece.
    pub fn quitar(&mut self, id: u32) {
        if self.ciudad.quitar(id) {
            self.rev += 1;
        }
    }

    /// Perfil de colocación: el de la ordenanza activa o, si no hay, el de la tradición de la plaza. Solo afecta a colocaciones nuevas.
    fn perfil(&self, u: &Urbano) -> Perfil {
        u.perfil.and_then(perfil_de).unwrap_or_else(|| self.ciudad.perfil_por_tradicion())
    }

    /// La clave de una consulta sobre un tipo (`None` = sin tipo concreto: la mudanza de una pieza ya colocada).
    fn clave(&self, u: &Urbano, clase: Option<Clase>) -> Clave {
        let prm = self.ciudad.params();
        let mut radio = radio_ciudad(prm, u.completados, u.nivel.min(255) as u8);
        let crea_ancla = clase.is_some_and(|c| c.rol == Rol::Normal && c.categoria.tiene_barrio() && c.categoria != Categoria::Mercado);
        if clase.is_none_or(|c| c.categoria == Categoria::Afueras) {
            // Las afueras se miden desde `max(radioCiudad, afuerasMín + banda)`: mientras la ciudad sea menor, no cuenta.
            radio = radio.max(prm.radio_afueras_min_ul + prm.ancho_banda_afueras_ul);
        }
        Clave {
            rev: self.rev,
            radio,
            siguiente_id: if crea_ancla { u.siguiente_id } else { 0 },
            gate_abierto: crea_ancla && u.nivel_operativo >= prm.nivel_gate_anclas as u32,
            perfil: self.perfil(u),
        }
    }

    fn pedido(urb: &Urbanismo, c: Clase, id: u32, tipo: Tipo, nivel_interno: u8, k: &Clave) -> Pedido {
        Pedido { id, tipo: tipo_a_u16(tipo), categoria: c.categoria, rol: c.rol, huella: urb.huella(tipo, nivel_interno), permitir_rotacion: c.rota, perfil: k.perfil, radio_ul: k.radio }
    }

    /// ¿Hay sitio en la ciudad para este tipo? Como el TypeScript al proponer (`asegurarAnclaPara`, T §7.1): para un tipo con barrio
    /// el trazado marca antes las anclas llenas y, si no queda ninguna con hueco y el gate de nivel está abierto, crea **ya** el ancla de
    /// saturación con el id `*ids` (que avanza). El ancla y las marcas se quedan aunque el proyecto no se llegue a comprometer, y el ancla
    /// vuelve como edificio gratuito para la economía. No coloca el edificio. Los tipos de ámbito de mapa (Cantera, minas) siempre caben.
    pub fn proponer(&mut self, urb: &Urbanismo, tipo: Tipo, u: &Urbano, ids: &mut u32) -> (bool, Vec<Gratuito>) {
        let Some(clase) = urb.clase(tipo) else { return (true, vec![]) };
        if clase.rol == Rol::Centro {
            return (false, vec![]);
        }
        let u = Urbano { siguiente_id: *ids, ..*u };
        let k = self.clave(&u, Some(clase));
        if let Some(r) = self.cache.buscar(Consulta::Cabe(tipo), k) {
            return (r, vec![]);
        }
        let ped = Self::pedido(urb, clase, *ids, tipo, 1, &k);
        let mut anclas = vec![];
        if clase.rol == Rol::Normal && clase.categoria.tiene_barrio() {
            let antes = self.ciudad.piezas().to_vec();
            if let Ancla::Creada(a) = self.ciudad.asegurar_ancla_para(&ped, *ids, u.nivel_operativo.min(255) as u8, &urb.opciones_ancla(clase.categoria)) {
                anclas.push(Gratuito { id: a, tipo: TIPOS[self.ciudad.pieza(a).expect("recién creada").tipo as usize], nivel_interno: 1 });
                *ids += 1;
            }
            if self.ciudad.piezas() != antes.as_slice() {
                self.rev += 1;
            }
        }
        let cabe = self.ciudad.siguiente_sitio(&ped).is_some();
        let k = self.clave(&Urbano { siguiente_id: *ids, ..u }, Some(clase));
        self.cache.poner(Consulta::Cabe(tipo), k, cabe);
        (cabe, anclas)
    }

    /// Coloca el edificio `id` en la ciudad. Antes, si hace falta (otro proyecto de la misma pasada llenó el ancla que [`Urbe::proponer`]
    /// le dio), el trazado crea el ancla de saturación de su barrio (un edificio gratuito más, con su id de `ids`). `None` = ya no hay sitio
    /// y la ciudad queda intacta.
    pub fn tomar(&mut self, urb: &Urbanismo, id: u32, tipo: Tipo, u: &Urbano, ids: &mut u32) -> Option<Vec<Gratuito>> {
        let Some(clase) = urb.clase(tipo) else { return Some(vec![]) };
        let k = self.clave(u, Some(clase));
        let ped = Self::pedido(urb, clase, id, tipo, 1, &k);
        let (mut gratuitos, mut sig) = (vec![], *ids);
        if clase.rol == Rol::Normal && clase.categoria.tiene_barrio() {
            // Sobre una copia: si luego no cabe, ni el ancla ni sus marcas de «llena» se quedan.
            let mut copia = self.ciudad.clone();
            if let Ancla::Creada(a) = copia.asegurar_ancla_para(&ped, sig, u.nivel_operativo.min(255) as u8, &urb.opciones_ancla(clase.categoria)) {
                gratuitos.push(Gratuito { id: a, tipo: TIPOS[copia.pieza(a)?.tipo as usize], nivel_interno: 1 });
                sig += 1;
            }
            copia.colocar_siguiente(&ped)?;
            self.ciudad = copia;
        } else {
            self.ciudad.colocar_siguiente(&ped)?;
        }
        self.rev += 1;
        *ids = sig;
        Some(gratuitos)
    }

    /// Los puestos gratuitos del Mercado para ese nivel, en el orden de `MERCADO_PUESTOS_POR_NIVEL`. A un puesto sin sitio se le salta en
    /// silencio (Economia_Construccion §6.6).
    pub fn puestos(&mut self, urb: &Urbanismo, nivel_mercado: u8, u: &Urbano, ids: &mut u32) -> Vec<Gratuito> {
        let clase = urb.clase(Tipo::PuestoMercado).expect("el puesto del Mercado está en la rejilla");
        let k = self.clave(u, Some(clase));
        let mut puestos = vec![];
        for &forma in urb.puestos(nivel_mercado) {
            let ped = Self::pedido(urb, clase, *ids, Tipo::PuestoMercado, forma, &k);
            if self.ciudad.colocar_siguiente(&ped).is_some() {
                self.rev += 1;
                puestos.push(Gratuito { id: *ids, tipo: Tipo::PuestoMercado, nivel_interno: forma });
                *ids += 1;
            }
        }
        puestos
    }

    /// La huella nueva de un edificio que al mejorar a `nivel` cambia de tamaño (hoy solo la Granja), si su pieza está en la ciudad.
    fn huella_nueva(&self, urb: &Urbanismo, id: u32, tipo: Tipo, nivel: u8) -> Option<Huella> {
        if tipo != Tipo::Granja {
            return None;
        }
        let r = self.ciudad.pieza(id)?.rect;
        let h = urb.huella(tipo, nivel);
        ((r.ancho, r.alto) != (h.ancho, h.alto)).then_some(h)
    }

    /// ¿Hay hueco para la huella nueva? No toca la ciudad. El caché guarda también las negativas: una mejora que espera su hueco
    /// pregunta cada minuto y no debe barrer el suelo cada vez.
    pub fn hueco_al_mejorar(&mut self, urb: &Urbanismo, id: u32, tipo: Tipo, nivel: u8, u: &Urbano) -> bool {
        let Some(h) = self.huella_nueva(urb, id, tipo, nivel) else { return true };
        let k = self.clave(u, None);
        if let Some(r) = self.cache.buscar(Consulta::Mudar(id, nivel), k) {
            return r;
        }
        let cabe = self.ciudad.reubicar_por_tamano(id, h, k.perfil, k.radio).is_some();
        self.cache.poner(Consulta::Mudar(id, nivel), k, cabe);
        cabe
    }

    /// Muda el edificio a un hueco de su huella nueva (T §6.7), conservando su sitio en el orden de construcción. `false` = no hay hueco.
    pub fn reubicar(&mut self, urb: &Urbanismo, id: u32, tipo: Tipo, nivel: u8, u: &Urbano) -> bool {
        let Some(h) = self.huella_nueva(urb, id, tipo, nivel) else { return true };
        let k = self.clave(u, None);
        if self.cache.buscar(Consulta::Mudar(id, nivel), k) == Some(false) {
            return false;
        }
        match self.ciudad.reubicar_por_tamano(id, h, k.perfil, k.radio) {
            Some(sitio) => {
                self.ciudad.reubicar(id, sitio);
                self.rev += 1;
                true
            }
            None => {
                self.cache.poner(Consulta::Mudar(id, nivel), k, false);
                false
            }
        }
    }
}

/// El [`Sitios`] de una plaza solo con su ciudad: ni nodos ni parcelas (los extractores ligados a un yacimiento no tienen sitio aquí).
/// Es la parte urbana de [`crate::economia::SitiosPlaza`] y el adaptador de las pruebas de la ciudad.
pub struct SitiosUrbe<'a> {
    pub urbe: &'a mut Urbe,
    pub urbanismo: &'a Urbanismo,
}

impl Sitios for SitiosUrbe<'_> {
    fn proponer(&mut self, tipo: Tipo, u: &Urbano, ids: &mut u32, anclas: &mut Vec<Gratuito>) -> Option<Option<Fuente>> {
        if tipo.es_extractor() || tipo == Tipo::Lenera {
            return None;
        }
        let (cabe, nuevas) = self.urbe.proponer(self.urbanismo, tipo, u, ids);
        anclas.extend(nuevas);
        cabe.then_some(None)
    }
    fn tomar(&mut self, id: u32, tipo: Tipo, _: Option<Fuente>, u: &Urbano, ids: &mut u32) -> Option<Vec<Gratuito>> {
        self.urbe.tomar(self.urbanismo, id, tipo, u, ids)
    }
    fn puestos_de_mercado(&mut self, nivel: u8, u: &Urbano, ids: &mut u32) -> Vec<Gratuito> {
        self.urbe.puestos(self.urbanismo, nivel, u, ids)
    }
    fn hueco_al_mejorar(&mut self, id: u32, tipo: Tipo, nivel: u8, u: &Urbano) -> bool {
        self.urbe.hueco_al_mejorar(self.urbanismo, id, tipo, nivel, u)
    }
    fn reubicar(&mut self, id: u32, tipo: Tipo, nivel: u8, u: &Urbano) -> bool {
        self.urbe.reubicar(self.urbanismo, id, tipo, nivel, u)
    }
    fn reclasificar_torres(&mut self, celdas: &mut [CeldaMuro], nivel: u8, p: &ParametrosMuralla) {
        self.urbe.reclasificar_torres(celdas, nivel, p);
    }
}

// ───────────────────────────── Para dibujar ─────────────────────────────

/// Una pieza de la ciudad como la ve un cliente: su rectángulo en unidades locales (no ve celdas).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PiezaVista {
    /// El id del edificio de la economía (`economia(id).edificio(id)`).
    pub id: u32,
    pub tipo: Tipo,
    pub categoria: Categoria,
    pub rol: Rol,
    pub rect: RectLocal,
    /// Centro del rectángulo, en milésimas de unidad local.
    pub centro: Punto,
    pub rotado: bool,
}

/// Un recinto de muralla como lo dibuja un cliente (Murallas §9.1, Trazado_Urbano §10): las celdas **en pie** fusionadas por clase en rectángulos y las que aún están por
/// levantar aparte (ocupan suelo igual, pero no defienden). Todo en unidades locales, como el resto de la vista.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecintoVista {
    /// `n` del id de texto `recinto-<plaza>-<n>` del contrato.
    pub id: u32,
    /// El nivel del anillo en pie: 1 empalizada, 2 muro de piedra, 3 muralla con adarve.
    pub nivel: u8,
    /// El nivel al que se está mejorando, si hay una mejora en curso (el anillo en pie sigue siendo el de `nivel`).
    pub mejorando_a: Option<u8>,
    /// Las celdas levantadas, por clase.
    pub muros: Vec<RectLocal>,
    pub torres: Vec<RectLocal>,
    pub puertas: Vec<RectLocal>,
    /// Las celdas del anillo congelado que todavía no se han levantado, de cualquier clase.
    pub planificadas: Vec<RectLocal>,
}

/// Lo que un cliente dibuja de los recintos de una plaza, del más interior al más exterior.
fn vista_de_recintos(recintos: &[asentamiento::recintos::Recinto]) -> Vec<RecintoVista> {
    let celdas = |cs: &[CeldaMuro], clase: ClaseCelda| fusionar_celdas(&cs.iter().filter(|c| c.clase == clase).map(|c| c.celda).collect::<Vec<_>>());
    recintos
        .iter()
        .map(|r| {
            let (en_pie, por_levantar) = r.celdas.split_at(r.levantadas());
            RecintoVista {
                id: r.id,
                nivel: r.nivel,
                mejorando_a: r.mejora.map(|m| m.nivel),
                muros: celdas(en_pie, ClaseCelda::Muro),
                torres: celdas(en_pie, ClaseCelda::Torre),
                puertas: celdas(en_pie, ClaseCelda::Puerta),
                planificadas: fusionar_celdas(&por_levantar.iter().map(|c| c.celda).collect::<Vec<_>>()),
            }
        })
        .collect()
}

/// Todo lo que un cliente necesita para dibujar la ciudad de una plaza (T §10): piezas, calles, caminos y murallas en unidades locales.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VistaCiudad {
    /// `radioCiudad` actual, en unidades locales (el disco urbano).
    pub radio_ul: i32,
    pub piezas: Vec<PiezaVista>,
    pub calles: Vec<RectLocal>,
    pub caminos: Vec<RectLocal>,
    pub murallas: Vec<RecintoVista>,
}

impl Urbe {
    /// La vista de la ciudad; los `recintos` son los de la economía de la plaza (la ciudad solo guarda su proyección, sin niveles ni avance).
    pub fn vista(&self, radio_ul: i32, recintos: &[asentamiento::recintos::Recinto]) -> VistaCiudad {
        let piezas = self
            .ciudad
            .piezas()
            .iter()
            .map(|p| PiezaVista {
                id: p.id,
                tipo: TIPOS[p.tipo as usize],
                categoria: p.categoria,
                rol: p.rol,
                rect: RectLocal { x: p.rect.col * CELDA_UL, y: p.rect.row * CELDA_UL, ancho: p.rect.ancho * CELDA_UL, alto: p.rect.alto * CELDA_UL },
                centro: p.posicion(),
                rotado: p.rotado,
            })
            .collect();
        VistaCiudad { radio_ul, piezas, calles: self.ciudad.calles_rectangulos(), caminos: self.ciudad.caminos_rectangulos(), murallas: vista_de_recintos(recintos) }
    }

    /// `radioCiudad` de la plaza con ese contexto.
    pub fn radio_ul(&self, u: &Urbano) -> i32 {
        radio_ciudad(self.ciudad.params(), u.completados, u.nivel.min(255) as u8)
    }
}

// ───────────────────────────── La planta de un pueblo ─────────────────────────────

/// Radio con el que se empieza a trazar la planta de un pueblo, en unidades locales (la ciudad de una plaza nace con 30; un pueblo es una aldea).
/// Crece de `RADIO_CIUDAD.crecimientoPorEdificio` en `crecimientoPorEdificio` hasta que caben todas sus piezas.
const RADIO_MINIMO_DE_PUEBLO_UL: i32 = 8;

/// El radio del disco centrado en el origen que contiene todas las piezas, en unidades locales (hacia arriba).
fn radio_que_ocupan(piezas: &[PiezaVista]) -> i32 {
    let esquinas = piezas.iter().flat_map(|p| [(p.rect.x, p.rect.y), (p.rect.x + p.rect.ancho, p.rect.y), (p.rect.x, p.rect.y + p.rect.alto), (p.rect.x + p.rect.ancho, p.rect.y + p.rect.alto)]);
    let d2 = esquinas.map(|(x, y)| x as i128 * x as i128 + y as i128 * y as i128).max().unwrap_or(0);
    let r = base::isqrt(d2);
    (if r * r < d2 { r + 1 } else { r }) as i32
}

/// La planta de un pueblo (Campamentos_Mercenarios §7, receta de `layoutCampamento`): el trazado de asentamientos **una sola vez**, con la mina
/// o la cantera (`extractor`) como ancla en el origen, `PUEBLOS_EXTRACTORES.casasDeLaPlanta` casas, el almacén y un corral para el buey, con
/// **el radio que necesiten sus piezas** (Economia_Construccion §14: la planta es un espacio interno; en el mapa el pueblo sigue ocupando 0,2).
/// Función pura de la semilla del núcleo y del id del pueblo: no se guarda y el mismo pueblo da siempre la misma planta. Es lo que se ve dentro
/// y el mapa de su batalla de saqueo. Los ids de las piezas son sus posiciones en la planta (el ancla es la 0), no ids de edificio de ninguna
/// economía.
pub fn planta_de_pueblo(reglas: &crate::ReglasEconomia, semilla_nucleo: u64, id_pueblo: u64, extractor: Tipo) -> VistaCiudad {
    let urb = &reglas.urbanismo;
    let semilla = trazado::hash(semilla_nucleo, "pueblo", &[id_pueblo as i64]);
    let perfil = Perfil::por_tradicion(semilla);
    let base = urb.parametros(&reglas.datos);
    let casas = reglas.datos.pueblos.casas_de_la_planta as usize;
    let resto: Vec<(Tipo, Categoria)> = std::iter::repeat_n((Tipo::Vivienda, Categoria::Residencial), casas).chain([(Tipo::Almacen, Categoria::Generico), (Tipo::Corral, Categoria::Afueras)]).collect();
    let trazar = |radio_ul: i32| {
        let params = Parametros { radio_inicial_ul: radio_ul, radio_afueras_min_ul: radio_ul / 2, ancho_banda_afueras_ul: radio_ul / 2, ..base };
        let pedido = |id: u32, tipo: Tipo, categoria: Categoria, rol: Rol| Pedido { id, tipo: tipo_a_u16(tipo), categoria, rol, huella: urb.huella(tipo, 1), permitir_rotacion: true, perfil, radio_ul };
        let mut ciudad = Ciudad::vacia(semilla, params);
        let ancla = pedido(0, extractor, Categoria::Residencial, Rol::Centro);
        ciudad.colocar(&ancla, Colocacion { rect: Rect::desde_centro(Punto::new(0, 0), ancla.huella), rotado: false }).expect("la planta vacía admite su ancla");
        let todas = resto.iter().enumerate().all(|(id, &(tipo, categoria))| ciudad.colocar_siguiente(&pedido(id as u32 + 1, tipo, categoria, Rol::Normal)).is_some());
        todas.then(|| {
            let mut vista = Urbe::desde_ciudad(ciudad).vista(radio_ul, &[]);
            vista.radio_ul = radio_ul.max(radio_que_ocupan(&vista.piezas));
            vista
        })
    };
    let tope = base.maximo_por_nivel_ul[4];
    (RADIO_MINIMO_DE_PUEBLO_UL..=tope).step_by(base.crecimiento_por_edificio_ul as usize).find_map(trazar).expect("las piezas de un pueblo caben en una ciudad de nivel 5")
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::ReglasEconomia;

    #[test]
    fn las_huellas_salen_del_documento() {
        let r = ReglasEconomia::default();
        let u = &r.urbanismo;
        let h = |t, n| {
            let h = u.huella(t, n);
            (h.ancho, h.alto)
        };
        assert_eq!(h(Tipo::CentroUrbano, 1), (4, 4));
        assert_eq!(h(Tipo::Vivienda, 1), (1, 1), "sin entrada: la huella por defecto");
        assert_eq!(h(Tipo::Carpinteria, 1), (4, 2));
        assert_eq!(h(Tipo::Armeria, 1), (2, 3));
        assert_eq!((h(Tipo::Granja, 1), h(Tipo::Granja, 2), h(Tipo::Granja, 3), h(Tipo::Granja, 4)), ((2, 2), (2, 3), (4, 3), (6, 6)));
        assert_eq!((h(Tipo::PuestoMercado, 1), h(Tipo::PuestoMercado, 2), h(Tipo::PuestoMercado, 3)), ((1, 2), (1, 3), (1, 1)));
        let acumulado: Vec<usize> = (1..=3).scan(1, |piezas, n| { *piezas += u.puestos(n).len(); Some(*piezas) }).collect();
        assert_eq!(acumulado, vec![6, 13, 17], "la zona queda en 6, 13 y 17 piezas contando el Mercado");
    }

    #[test]
    fn la_planta_de_un_pueblo_sale_de_su_id_con_la_mina_de_ancla_y_no_se_guarda() {
        let r = ReglasEconomia::default();
        let a = planta_de_pueblo(&r, 42, 7, Tipo::MinaCobre);
        assert_eq!(a, planta_de_pueblo(&r, 42, 7, Tipo::MinaCobre), "función pura");
        assert_ne!(a, planta_de_pueblo(&r, 42, 8, Tipo::MinaCobre), "cada pueblo tiene la suya");
        assert_eq!((a.piezas[0].id, a.piezas[0].tipo, a.piezas[0].centro), (0, Tipo::MinaCobre, Punto::new(1_500, 1_500)), "el ancla en el origen");
        let de = |t: Tipo| a.piezas.iter().filter(|p| p.tipo == t).count();
        assert_eq!((de(Tipo::Vivienda), de(Tipo::Almacen), de(Tipo::Corral)), (6, 1, 1), "6 casas, el almacén y el corral del buey");
        assert!(!a.calles.is_empty() || !a.caminos.is_empty());
    }

    #[test]
    fn la_planta_de_un_pueblo_toma_el_radio_que_necesitan_sus_piezas() {
        let r = ReglasEconomia::default();
        for id in 0..40 {
            let a = planta_de_pueblo(&r, 42, id, Tipo::MinaHierro);
            assert_eq!(a.piezas.len(), 1 + 6 + 2, "pueblo {id}: caben todas, también el corral de las afueras");
            // El radio de la planta cubre todas sus piezas (sin el tope de 8 u.): ninguna esquina queda fuera del disco.
            assert!(a.radio_ul > RADIO_MINIMO_DE_PUEBLO_UL, "pueblo {id}: 9 piezas con sus calles no caben en 8 u. ({} u.)", a.radio_ul);
            for p in &a.piezas {
                for (x, y) in [(p.rect.x, p.rect.y), (p.rect.x + p.rect.ancho, p.rect.y), (p.rect.x, p.rect.y + p.rect.alto), (p.rect.x + p.rect.ancho, p.rect.y + p.rect.alto)] {
                    assert!((x as i64).pow(2) + (y as i64).pow(2) <= (a.radio_ul as i64).pow(2), "pueblo {id}: {:?} fuera de {} u.", p.rect, a.radio_ul);
                }
            }
        }
    }
}
