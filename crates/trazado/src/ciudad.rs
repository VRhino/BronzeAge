//! El estado de un asentamiento y su red de calles (T §1, §4, §5).
//!
//! `Ciudad` guarda lo único persistente (semilla, parámetros, piezas en orden de construcción, recintos). El suelo ocupado y
//! la red de calles son **derivados**: se reconstruyen siempre desde las piezas y los recintos ([`Ciudad::desde_partes`]).
//! Como colocar una pieza solo añade al final, la red se actualiza de forma incremental y da exactamente lo mismo que una
//! reconstrucción completa (lo vigila una prueba).

use crate::azar::sorteo;
use crate::geom::{es_borde_de_manzana, Celda, Rect};
use crate::tipos::*;
use std::collections::{HashSet, VecDeque};
use std::hash::{BuildHasherDefault, Hasher};

/// Semiancho de la ventana de trabajo, en celdas: la ciudad vive en `[-MITAD, MITAD]²` (±288 u.l.). El mayor
/// `radioCiudad` (180 u.l.) más afueras y corredores cabe con holgura; lo que salga de la ventana cuenta como ocupado.
pub const MITAD: i32 = 96;
pub(crate) const LADO: i32 = 2 * MITAD + 1;
pub(crate) const CELDAS: usize = (LADO * LADO) as usize;

pub(crate) const CALLE: u8 = 1;
pub(crate) const CAMINO: u8 = 2;

/// Hasher rápido y determinista (FxHash) para conjuntos pequeños de índices; nunca se itera un conjunto para decidir.
#[derive(Default, Clone, Copy)]
pub(crate) struct Fx(u64);

impl Hasher for Fx {
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0.rotate_left(5) ^ b as u64).wrapping_mul(0x517cc1b727220a95);
        }
    }
    fn write_u32(&mut self, n: u32) {
        self.0 = (self.0.rotate_left(5) ^ n as u64).wrapping_mul(0x517cc1b727220a95);
    }
    fn write_usize(&mut self, n: usize) {
        self.write_u32(n as u32);
    }
    fn finish(&self) -> u64 {
        self.0
    }
}
pub(crate) type Conj = HashSet<usize, BuildHasherDefault<Fx>>;

pub(crate) fn idx(c: Celda) -> Option<usize> {
    if c.col.abs() > MITAD || c.row.abs() > MITAD {
        None
    } else {
        Some(((c.row + MITAD) * LADO + c.col + MITAD) as usize)
    }
}

pub(crate) fn celda_de(i: usize) -> Celda {
    Celda { col: i as i32 % LADO - MITAD, row: i as i32 / LADO - MITAD }
}

pub(crate) fn vecinos_i(i: usize) -> [Option<usize>; 4] {
    let (col, row) = (i as i32 % LADO, i as i32 / LADO);
    [
        (row > 0).then(|| i - LADO as usize),
        (col < LADO - 1).then(|| i + 1),
        (row < LADO - 1).then(|| i + LADO as usize),
        (col > 0).then(|| i - 1),
    ]
}

/// ¿Cabe el rectángulo, con un margen de celdas alrededor, en la ventana?
pub(crate) fn en_ventana(r: &Rect, margen: i32) -> bool {
    r.col - margen >= -MITAD && r.row - margen >= -MITAD && r.fin_col() - 1 + margen <= MITAD && r.fin_row() - 1 + margen <= MITAD
}

/// Suelo derivado: ocupación por edificios y muros, y la red de calles (1) y caminos (2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Suelo {
    pub ocup: Vec<bool>,
    pub red: Vec<u8>,
    pub red_celdas: Vec<u32>,
}

impl Suelo {
    fn nuevo() -> Self {
        Self { ocup: vec![false; CELDAS], red: vec![0; CELDAS], red_celdas: Vec::new() }
    }
    /// Una celda está libre si está en la ventana, sin edificio ni muro y sin calle ni camino (T §4).
    pub fn libre_i(&self, i: usize) -> bool {
        !self.ocup[i] && self.red[i] == 0
    }
    pub fn libre(&self, c: Celda) -> bool {
        idx(c).is_some_and(|i| self.libre_i(i))
    }
    pub fn rect_libre(&self, r: &Rect) -> bool {
        en_ventana(r, 0) && r.celdas().all(|c| self.libre_i(idx(c).unwrap()))
    }
    pub fn es_red(&self, c: Celda) -> bool {
        idx(c).is_some_and(|i| self.red[i] != 0)
    }
    pub fn tiene_frente(&self, r: &Rect) -> bool {
        r.adyacentes().into_iter().any(|c| self.es_red(c))
    }
    fn vecina_red_i(&self, i: usize) -> bool {
        vecinos_i(i).into_iter().flatten().any(|n| self.red[n] != 0)
    }
    fn poner(&mut self, i: usize, clase: u8) {
        self.red[i] = clase;
        self.red_celdas.push(i as u32);
    }

    /// `corredorHastaLaRed` (T §5.1): BFS 4-conexo sobre celdas libres desde las adyacentes al rectángulo; la meta es una celda
    /// con vecina ortogonal en la red. Devuelve el camino de menor longitud, de la meta hacia atrás, o `None` si no hay en `cap`.
    pub fn corredor(&self, r: &Rect, cap: u16) -> Option<Vec<Celda>> {
        let mut prev = vec![-2i32; CELDAS];
        let mut dist = vec![0u16; CELDAS];
        let mut cola: Vec<usize> = Vec::new();
        for c in r.adyacentes() {
            if let Some(i) = idx(c) {
                if self.libre_i(i) && prev[i] == -2 {
                    prev[i] = -1;
                    dist[i] = 1;
                    cola.push(i);
                }
            }
        }
        let mut cabeza = 0;
        while cabeza < cola.len() {
            let i = cola[cabeza];
            cabeza += 1;
            if self.vecina_red_i(i) {
                let mut camino = Vec::new();
                let mut k = i as i32;
                while k >= 0 {
                    camino.push(celda_de(k as usize));
                    k = prev[k as usize];
                }
                return Some(camino);
            }
            if dist[i] < cap {
                for n in vecinos_i(i).into_iter().flatten() {
                    if prev[n] == -2 && self.libre_i(n) && !r.contiene(celda_de(n)) {
                        prev[n] = i as i32;
                        dist[n] = dist[i] + 1;
                        cola.push(n);
                    }
                }
            }
        }
        None
    }

    /// Campo de distancias a la red (T §6.2): para cada celda libre, cuántas celdas libres hay que abrir para tocar la red
    /// (1 si ella misma toca la red), propagado hasta `cap`. 0 = inalcanzable.
    pub fn campo(&self, cap: u16) -> Vec<u16> {
        let mut dist = vec![0u16; CELDAS];
        let mut cola: VecDeque<usize> = VecDeque::new();
        for &r in &self.red_celdas {
            for n in vecinos_i(r as usize).into_iter().flatten() {
                if dist[n] == 0 && self.libre_i(n) {
                    dist[n] = 1;
                    cola.push_back(n);
                }
            }
        }
        while let Some(i) = cola.pop_front() {
            if dist[i] < cap {
                for n in vecinos_i(i).into_iter().flatten() {
                    if dist[n] == 0 && self.libre_i(n) {
                        dist[n] = dist[i] + 1;
                        cola.push_back(n);
                    }
                }
            }
        }
        dist
    }
}

/// Estado de un asentamiento.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ciudad {
    semilla: u64,
    params: Parametros,
    piezas: Vec<Pieza>,
    recintos: Vec<Recinto>,
    pub(crate) suelo: Suelo,
    max_fila: i32,
    desfase: i32,
}

impl Ciudad {
    /// Ciudad sin piezas. `semilla` es el hash estable del id del asentamiento: toda variación sale de ella (T §3).
    pub fn vacia(semilla: u64, params: Parametros) -> Self {
        let rango = (params.largo_fila_max - params.largo_fila_min + 1) as u64;
        let max_fila = params.largo_fila_min + sorteo(semilla, "largo-fila", &[], rango) as i32;
        let desfase = sorteo(semilla, "desfase-manzana", &[], max_fila as u64) as i32;
        Self { semilla, params, piezas: Vec::new(), recintos: Vec::new(), suelo: Suelo::nuevo(), max_fila, desfase }
    }

    /// Reconstruye una ciudad desde lo persistido. Rechaza una revisión de geometría distinta (T §2.1) y piezas que se pisen.
    pub fn desde_partes(
        semilla: u64,
        params: Parametros,
        layout_version: u32,
        piezas: Vec<Pieza>,
        recintos: Vec<Recinto>,
    ) -> Result<Self, ErrorTrazado> {
        if layout_version != LAYOUT_VERSION {
            return Err(ErrorTrazado::VersionDeGeometria(layout_version));
        }
        let mut c = Self::vacia(semilla, params);
        c.recintos = recintos;
        for (k, p) in piezas.iter().enumerate() {
            if piezas[..k].iter().any(|q| q.id == p.id) {
                return Err(ErrorTrazado::IdRepetido(p.id));
            }
            if !en_ventana(&p.rect, 1) {
                return Err(ErrorTrazado::FueraDeVentana);
            }
            if let Some(o) = piezas[..k].iter().find(|q| q.rect.solapa(&p.rect)) {
                return Err(ErrorTrazado::Ocupada(Celda::new(o.rect.col.max(p.rect.col), o.rect.row.max(p.rect.row))));
            }
        }
        c.piezas = piezas;
        c.reconstruir();
        Ok(c)
    }

    pub fn semilla(&self) -> u64 {
        self.semilla
    }
    pub fn params(&self) -> &Parametros {
        &self.params
    }
    pub fn piezas(&self) -> &[Pieza] {
        &self.piezas
    }
    pub fn pieza(&self, id: u32) -> Option<&Pieza> {
        self.piezas.iter().find(|p| p.id == id)
    }
    pub fn recintos(&self) -> &[Recinto] {
        &self.recintos
    }
    pub fn perfil_por_tradicion(&self) -> Perfil {
        Perfil::por_tradicion(self.semilla)
    }
    /// Reemplaza los recintos (las celdas de muro bloquean desde que el recinto se compromete) y recalcula la red.
    pub fn set_recintos(&mut self, recintos: Vec<Recinto>) {
        self.recintos = recintos;
        self.reconstruir();
    }
    /// Recalcula la red desde cero. Solo hace falta tras tocar piezas por fuera de [`Ciudad::colocar`].
    pub fn recalcular_red(&mut self) {
        self.reconstruir();
    }

    pub fn es_calle(&self, c: Celda) -> bool {
        idx(c).is_some_and(|i| self.suelo.red[i] == CALLE)
    }
    pub fn es_camino(&self, c: Celda) -> bool {
        idx(c).is_some_and(|i| self.suelo.red[i] == CAMINO)
    }
    /// Edificio, muro, calle o camino (fuera de la ventana cuenta como ocupado).
    pub fn esta_ocupada(&self, c: Celda) -> bool {
        idx(c).is_none_or(|i| !self.suelo.libre_i(i))
    }
    /// Celdas de calle (urbanas), en orden de fila y columna.
    pub fn calles(&self) -> Vec<Celda> {
        self.celdas_de_red(CALLE)
    }
    /// Celdas de camino (rurales: solo conectan granjas y corrales), en orden de fila y columna.
    pub fn caminos(&self) -> Vec<Celda> {
        self.celdas_de_red(CAMINO)
    }
    fn celdas_de_red(&self, clase: u8) -> Vec<Celda> {
        self.suelo.red.iter().enumerate().filter(|(_, &c)| c == clase).map(|(i, _)| celda_de(i)).collect()
    }

    /// Añade una pieza al final y actualiza la red. No decide el sitio: ver [`Ciudad::siguiente_sitio`].
    pub fn colocar(&mut self, pedido: &Pedido, sitio: Colocacion) -> Result<(), ErrorTrazado> {
        if self.piezas.iter().any(|p| p.id == pedido.id) {
            return Err(ErrorTrazado::IdRepetido(pedido.id));
        }
        if !en_ventana(&sitio.rect, 1) {
            return Err(ErrorTrazado::FueraDeVentana);
        }
        if let Some(c) = sitio.rect.celdas().find(|&c| !self.suelo.libre(c)) {
            return Err(ErrorTrazado::Ocupada(c));
        }
        self.piezas.push(Pieza {
            id: pedido.id,
            tipo: pedido.tipo,
            categoria: pedido.categoria,
            rol: pedido.rol,
            rect: sitio.rect,
            rotado: sitio.rotado,
            ancla_llena: false,
            semilla_saturada: false,
        });
        for c in sitio.rect.celdas() {
            self.suelo.ocup[idx(c).unwrap()] = true;
        }
        if pedido.rol == Rol::Centro {
            self.reconstruir();
        } else if self.centro().is_some() {
            self.procesar(sitio.rect, pedido.categoria == Categoria::Afueras, pedido.rol.es_ancla());
        }
        Ok(())
    }

    /// Pone la siguiente pieza en el mejor sitio que dé el pedido. `None` = no cabe (y no se toca nada).
    pub fn colocar_siguiente(&mut self, pedido: &Pedido) -> Option<Colocacion> {
        let sitio = self.siguiente_sitio(pedido)?;
        self.colocar(pedido, sitio).ok()?;
        Some(sitio)
    }

    /// Marca el latch `ancla_llena` / `semilla_saturada` de una pieza (los latches son permanentes).
    pub(crate) fn marcar(&mut self, id: u32, llena: bool, saturada: bool) {
        if let Some(p) = self.piezas.iter_mut().find(|p| p.id == id) {
            p.ancla_llena |= llena;
            p.semilla_saturada |= saturada;
        }
    }

    pub(crate) fn centro(&self) -> Option<&Pieza> {
        self.piezas.iter().find(|p| p.rol == Rol::Centro)
    }

    /// Cambia el rectángulo de una pieza ya colocada (reubicación por crecimiento, T §6.7) y recalcula la red.
    pub(crate) fn mover(&mut self, id: u32, sitio: Colocacion) {
        if let Some(p) = self.piezas.iter_mut().find(|p| p.id == id) {
            p.rect = sitio.rect;
            p.rotado = sitio.rotado;
        }
        self.reconstruir();
    }

    /// Quita una pieza (un edificio que sale de la cola de obras sin haberse levantado) y recalcula la red desde las demás, en su
    /// orden de construcción. Los latches de las anclas se conservan. `false` si no existía.
    pub fn quitar(&mut self, id: u32) -> bool {
        let Some(k) = self.piezas.iter().position(|p| p.id == id) else { return false };
        self.piezas.remove(k);
        self.reconstruir();
        true
    }

    /// La ciudad sin la pieza `id`, para buscarle sitio nuevo (T §6.7, `sueloOcupado(…, excluirId)`): solo se libera su suelo; la red
    /// se queda la que da **con** ella, así que su camino sigue sirviendo de frente.
    pub(crate) fn sin_pieza(&self, id: u32) -> Ciudad {
        let mut c = self.clone();
        c.piezas.retain(|p| p.id != id);
        if let Some(p) = self.pieza(id) {
            for i in p.rect.celdas().filter_map(idx) {
                c.suelo.ocup[i] = false;
            }
        }
        c
    }

    // ---- red de calles (T §5.1) ----

    fn reconstruir(&mut self) {
        self.suelo = Suelo::nuevo();
        for r in &self.recintos {
            for c in &r.muros {
                if let Some(i) = idx(*c) {
                    self.suelo.ocup[i] = true;
                }
            }
        }
        for p in &self.piezas {
            for c in p.rect.celdas() {
                if let Some(i) = idx(c) {
                    self.suelo.ocup[i] = true;
                }
            }
        }
        let Some(cu) = self.centro().map(|p| p.rect) else { return };
        self.anadir_conectadas(cu.anillo(self.params.ancho_calle), CALLE);
        let pendientes: Vec<(u32, Rect, bool, bool)> = self
            .piezas
            .iter()
            .filter(|p| p.rol != Rol::Centro)
            .map(|p| (p.id, p.rect, p.categoria == Categoria::Afueras, p.rol.es_ancla()))
            .collect();
        // La red que cada recinto conserva en sus puertas entra en el punto de la historia en que se trazó: tras las piezas que ya había (las de id hasta
        // `tras_pieza`) y antes de la primera que vino después (LAYOUT_VERSION 5; Murallas M1). Así una pieza nueva da la misma red colocada que reconstruida.
        let mut siembras: Vec<(u32, Vec<(Celda, u8)>)> = self
            .recintos
            .iter()
            .map(|r| (r.tras_pieza, r.red_de_puertas.iter().map(|c| (c.celda, if c.camino { CAMINO } else { CALLE })).collect()))
            .collect();
        for (id, rect, afueras, ancla) in pendientes {
            self.sembrar_antes_de(&mut siembras, id);
            self.procesar(rect, afueras, ancla);
        }
        self.sembrar_antes_de(&mut siembras, u32::MAX);
    }

    /// Siembra, conectada a la red, la de las puertas de los recintos trazados antes de la pieza `id`: si ningún tramo toca ya la red (el muro cortó la calle que
    /// los unía), cada puerta se une a ella con el corredor más corto por suelo libre, como una pieza sin frente (sin cap: dentro de la ventana).
    fn sembrar_antes_de(&mut self, siembras: &mut [(u32, Vec<(Celda, u8)>)], id: u32) {
        for (tras, celdas) in siembras.iter_mut().filter(|(tras, _)| id > *tras) {
            *tras = u32::MAX;
            let mut celdas = std::mem::take(celdas);
            let puertas: Vec<Celda> = self.recintos.iter().flat_map(|r| r.puertas.iter().copied()).collect();
            for &(p, clase) in celdas.clone().iter().filter(|(c, _)| puertas.contains(c)) {
                if let Some(corredor) = self.suelo.corredor(&Rect::new(p.col, p.row, 1, 1), u16::MAX) {
                    celdas.extend(corredor.into_iter().map(|c| (c, clase)));
                }
            }
            self.anadir_conectadas_de(celdas);
        }
    }

    /// Lo que la red añade por una pieza (T §5.1): su corredor si no tiene frente, el anillo si es ancla y la franja del retículo.
    fn procesar(&mut self, rect: Rect, afueras: bool, ancla: bool) {
        let p = self.params;
        let destino = if afueras { CAMINO } else { CALLE };
        if !self.suelo.tiene_frente(&rect) {
            let cap = if afueras { p.cap_corredor_afueras } else { p.cap_corredor_urbano };
            if let Some(camino) = self.suelo.corredor(&rect, cap) {
                for c in camino {
                    if let Some(i) = idx(c) {
                        self.suelo.poner(i, destino);
                    }
                }
            }
        }
        if ancla {
            self.anadir_conectadas(rect.anillo(p.ancho_calle), CALLE);
        }
        if afueras {
            return; // un camino rural no forma manzanas
        }
        let (paso_col, paso_fila) = (self.max_fila + p.ancho_calle, p.fondo_manzana + p.ancho_calle);
        let mut franja = Vec::new();
        for g in 0..p.ancho_calle {
            let (izq, der, arr, aba) = (rect.col - 1 - g, rect.fin_col() + g, rect.row - 1 - g, rect.fin_row() + g);
            if es_borde_de_manzana(izq, paso_col, self.desfase) {
                franja.extend((rect.row..rect.fin_row()).map(|row| Celda::new(izq, row)));
            }
            if es_borde_de_manzana(der, paso_col, self.desfase) {
                franja.extend((rect.row..rect.fin_row()).map(|row| Celda::new(der, row)));
            }
            if es_borde_de_manzana(arr, paso_fila, self.desfase) {
                franja.extend((rect.col..rect.fin_col()).map(|col| Celda::new(col, arr)));
            }
            if es_borde_de_manzana(aba, paso_fila, self.desfase) {
                franja.extend((rect.col..rect.fin_col()).map(|col| Celda::new(col, aba)));
            }
        }
        self.anadir_conectadas(franja, CALLE);
    }

    /// `añadirConectadas` (T §5.1): de las candidatas, las libres que se conectan a la red (directamente o por flood fill 4-conexo
    /// dentro del grupo). Con la red vacía todas son semilla. Un tramo huérfano no se añade: no hay islas.
    fn anadir_conectadas(&mut self, candidatas: Vec<Celda>, clase: u8) {
        self.anadir_conectadas_de(candidatas.into_iter().map(|c| (c, clase)).collect());
    }

    /// Lo mismo con la clase (calle o camino) de cada candidata.
    fn anadir_conectadas_de(&mut self, candidatas: Vec<(Celda, u8)>) {
        let mut libres: std::collections::HashMap<usize, u8, BuildHasherDefault<Fx>> = Default::default();
        let mut orden = Vec::new();
        for (c, clase) in candidatas {
            if let Some(i) = idx(c) {
                if self.suelo.libre_i(i) && !libres.contains_key(&i) {
                    libres.insert(i, clase);
                    orden.push(i);
                }
            }
        }
        let vacia = self.suelo.red_celdas.is_empty();
        let mut vistas = Conj::default();
        let mut pila: Vec<usize> = Vec::new();
        for &i in &orden {
            if (vacia || self.suelo.vecina_red_i(i)) && vistas.insert(i) {
                pila.push(i);
            }
        }
        let mut aceptadas = Vec::new();
        while let Some(i) = pila.pop() {
            aceptadas.push(i);
            for n in vecinos_i(i).into_iter().flatten() {
                if libres.contains_key(&n) && vistas.insert(n) {
                    pila.push(n);
                }
            }
        }
        for i in aceptadas {
            self.suelo.poner(i, libres[&i]);
        }
    }

    // ---- recintos (T §6.6) ----

    /// Interior del último recinto exterior completo: inundación 4-conexa desde una celda del Centro Urbano que no cruza ninguna
    /// celda del anillo (puertas incluidas). `None` si no hay recinto completo.
    pub(crate) fn interior_de_recinto(&self) -> Option<Vec<bool>> {
        let recinto = self.recintos.iter().rev().find(|r| r.completo)?;
        let cu = self.centro()?.rect;
        let mut anillo = vec![false; CELDAS];
        for c in recinto.muros.iter().chain(&recinto.puertas) {
            if let Some(i) = idx(*c) {
                anillo[i] = true;
            }
        }
        let inicio = idx(Celda::new(cu.col, cu.row))?;
        (!anillo[inicio]).then(|| inundar([inicio], |i| !anillo[i]))
    }
}

/// Inundación 4-conexa en la rejilla de la ventana: las celdas alcanzables desde `semillas` pasando solo por celdas con `pasa`
/// (una semilla que no pasa no cuenta). Ningún orden decide nada.
pub(crate) fn inundar(semillas: impl IntoIterator<Item = usize>, pasa: impl Fn(usize) -> bool) -> Vec<bool> {
    let mut vistas = vec![false; CELDAS];
    let mut pila = Vec::new();
    for s in semillas {
        if pasa(s) && !vistas[s] {
            vistas[s] = true;
            pila.push(s);
        }
    }
    while let Some(i) = pila.pop() {
        for n in vecinos_i(i).into_iter().flatten() {
            if !vistas[n] && pasa(n) {
                vistas[n] = true;
                pila.push(n);
            }
        }
    }
    vistas
}
