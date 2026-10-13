//! Niebla: lo explorado por cada Facción como máscara de bits sobre el raster del mundo (Niebla_De_Guerra §2).
//!
//! Misma rejilla y mismo orden de bits que la máscara `transitable` del mundo: celda `i = fila · columnas + columna`
//! → byte `i >> 3`, bit `i & 7`, LSB primero; fila 0 al norte. Una máscara tiene `⌈columnas · filas / 8⌉` bytes.

use crate::{FaccionId, Observador};
use base::{isqrt, Instante, Mil, Punto};
use contrato::v1::NieblaProyectada;
use mundo::{Mundo, TAMANO_CELDA};
use rayon::prelude::*;

/// Máscara de bits sobre la rejilla.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mascara {
    columnas: usize,
    filas: usize,
    tamano: i64,
    bits: Vec<u8>,
}

impl Mascara {
    pub fn nueva(columnas: usize, filas: usize, tamano: i64) -> Self {
        Self { columnas, filas, tamano, bits: vec![0; (columnas * filas).div_ceil(8)] }
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bits
    }

    pub fn get(&self, columna: usize, fila: usize) -> bool {
        columna < self.columnas && fila < self.filas && {
            let i = fila * self.columnas + columna;
            self.bits[i >> 3] >> (i & 7) & 1 == 1
        }
    }

    /// Marca la celda; `true` si el bit era nuevo.
    fn poner(&mut self, columna: usize, fila: usize) -> bool {
        let i = fila * self.columnas + columna;
        let (b, m) = (i >> 3, 1u8 << (i & 7));
        let nuevo = self.bits[b] & m == 0;
        self.bits[b] |= m;
        nuevo
    }

    /// `marcarVisto` (§2.2): marca la celda del ojo siempre y las celdas cuyo **centro** está a `≤ radio`.
    /// Devuelve `true` si hubo algún bit nuevo.
    pub fn marcar_vision(&mut self, centro: Punto, radio: Mil) -> bool {
        let t = self.tamano;
        let mut nuevo = false;
        let (cc, cf) = (centro.x.div_euclid(t), centro.y.div_euclid(t));
        if cc >= 0 && cf >= 0 && (cc as usize) < self.columnas && (cf as usize) < self.filas {
            nuevo |= self.poner(cc as usize, cf as usize);
        }
        if radio < 0 {
            return nuevo;
        }
        let f0 = (centro.y - radio).div_euclid(t).max(0);
        let f1 = (centro.y + radio).div_euclid(t).min(self.filas as i64 - 1);
        let r2 = radio as i128 * radio as i128;
        for fila in f0..=f1 {
            // Por fila: las columnas con centro en [x - semi, x + semi], con semi = ⌊√(r² − dy²)⌋ (exacto con enteros).
            let dy = (fila * t + t / 2 - centro.y) as i128;
            if dy * dy > r2 {
                continue;
            }
            let semi = isqrt(r2 - dy * dy) as i64;
            let c0 = (centro.x - semi - t / 2 + t - 1).div_euclid(t).max(0); // ⌈(x − semi − t/2) / t⌉
            let c1 = (centro.x + semi - t / 2).div_euclid(t).min(self.columnas as i64 - 1);
            for col in c0..=c1 {
                nuevo |= self.poner(col as usize, fila as usize);
            }
        }
        nuevo
    }

    /// OR bit a bit (`fundirExploraciones`). Con rejillas distintas no hace nada.
    pub fn fundir(&mut self, otra: &Mascara) {
        if self.bits.len() == otra.bits.len() {
            self.bits.iter_mut().zip(&otra.bits).for_each(|(a, b)| *a |= b);
        }
    }

    /// Número de celdas marcadas.
    pub fn contar(&self) -> usize {
        self.bits.iter().map(|b| b.count_ones() as usize).sum()
    }
}

/// Lo explorado por cada Facción.
#[derive(Clone, Debug)]
pub struct Niebla {
    columnas: usize,
    filas: usize,
    tamano: i64,
    /// Ordenado por Facción. Una ausente no ha visto nada.
    facciones: Vec<(FaccionId, Mascara)>,
}

impl Niebla {
    pub fn nueva(columnas: usize, filas: usize) -> Self {
        Self { columnas, filas, tamano: TAMANO_CELDA as i64, facciones: Vec::new() }
    }

    /// Sobre la rejilla de un mundo cargado.
    pub fn para_mundo(m: &Mundo) -> Self {
        let (c, f) = m.rejilla();
        Self::nueva(c, f)
    }

    pub fn rejilla(&self) -> (usize, usize) {
        (self.columnas, self.filas)
    }

    pub fn facciones(&self) -> impl Iterator<Item = FaccionId> + '_ {
        self.facciones.iter().map(|(f, _)| *f)
    }

    fn mascara(&self, f: FaccionId) -> Option<&Mascara> {
        self.facciones.binary_search_by_key(&f, |(g, _)| *g).ok().map(|i| &self.facciones[i].1)
    }

    fn mascara_mut(&mut self, f: FaccionId) -> &mut Mascara {
        let i = match self.facciones.binary_search_by_key(&f, |(g, _)| *g) {
            Ok(i) => i,
            Err(i) => {
                self.facciones.insert(i, (f, Mascara::nueva(self.columnas, self.filas, self.tamano)));
                i
            }
        };
        &mut self.facciones[i].1
    }

    /// Un ojo propio de `faccion` mira desde `centro` con `radio` (§2.2). `true` si descubrió algo nuevo.
    pub fn descubrir(&mut self, faccion: FaccionId, centro: Punto, radio: Mil) -> bool {
        self.mascara_mut(faccion).marcar_vision(centro, radio)
    }

    /// ¿Ha explorado `faccion` la celda `(columna, fila)`?
    pub fn explorada(&self, faccion: FaccionId, columna: usize, fila: usize) -> bool {
        self.mascara(faccion).is_some_and(|m| m.get(columna, fila))
    }

    /// ¿Ha explorado `faccion` el punto?
    pub fn explorado_en(&self, faccion: FaccionId, p: Punto) -> bool {
        let (c, f) = (p.x.div_euclid(self.tamano), p.y.div_euclid(self.tamano));
        c >= 0 && f >= 0 && self.explorada(faccion, c as usize, f as usize)
    }

    /// Muestreo de exploración (§4.1): cada observador con `graba` marca lo que ve en su Facción, en paralelo por
    /// Facción (cada máscara es de un solo hilo). Los aliados y las Miradas (`graba == false`) no exploran.
    pub fn grabar(&mut self, t: Instante, observadores: &[Observador]) {
        let mut ojos: Vec<(FaccionId, Punto, Mil)> =
            observadores.iter().filter(|o| o.graba).filter_map(|o| Some((o.faccion, o.donde.en(t)?, o.radio))).collect();
        ojos.sort_by_key(|o| o.0); // estable
        for (f, _, _) in &ojos {
            self.mascara_mut(*f);
        }
        self.facciones.par_iter_mut().for_each(|(f, m)| {
            let a = ojos.partition_point(|o| o.0 < *f);
            for &(_, p, r) in ojos[a..].iter().take_while(|o| o.0 == *f) {
                m.marcar_vision(p, r);
            }
        });
    }

    /// Une lo explorado por `origen` a `destino` y borra `origen` (héroe sin bandera que entra en una Facción, o fusión
    /// de Facciones; §4.3).
    pub fn fundir(&mut self, origen: FaccionId, destino: FaccionId) {
        if origen == destino {
            return;
        }
        if let Ok(i) = self.facciones.binary_search_by_key(&origen, |(g, _)| *g) {
            let (_, m) = self.facciones.remove(i);
            self.mascara_mut(destino).fundir(&m);
        }
    }

    /// Bytes de la máscara explorada (vacío si la Facción no ha visto nada), para persistir en las fotos.
    pub fn volcar(&self, faccion: FaccionId) -> Vec<u8> {
        self.mascara(faccion).map(|m| m.bits.clone()).unwrap_or_default()
    }

    /// Restaura la máscara de `faccion`: más corta se completa con ceros, más larga se trunca, y los bits sobrantes del
    /// último byte se limpian (§2.1).
    pub fn restaurar(&mut self, faccion: FaccionId, bytes: &[u8]) {
        let m = self.mascara_mut(faccion);
        let n = m.bits.len();
        m.bits.fill(0);
        let k = n.min(bytes.len());
        m.bits[..k].copy_from_slice(&bytes[..k]);
        let total = m.columnas * m.filas;
        if !total.is_multiple_of(8) && n > 0 {
            m.bits[n - 1] &= (1u8 << (total % 8)) - 1;
        }
    }

    /// `NieblaProyectada` (§2.3, §5.3): `celdas` = explorado ∪ lo que ven ahora los ojos `propios`; `visibles` = propios
    /// ∪ `prestados` (aliados y Miradas). Lo prestado no se explora: no entra en `celdas` ni se guarda.
    pub fn proyectar(&self, faccion: FaccionId, propios: &[(Punto, Mil)], prestados: &[(Punto, Mil)]) -> NieblaProyectada {
        let vacia = Mascara::nueva(self.columnas, self.filas, self.tamano);
        let mut celdas = self.mascara(faccion).unwrap_or(&vacia).clone();
        let mut visibles = vacia;
        for &(p, r) in propios {
            celdas.marcar_vision(p, r);
            visibles.marcar_vision(p, r);
        }
        for &(p, r) in prestados {
            visibles.marcar_vision(p, r);
        }
        NieblaProyectada {
            tamano_celda: self.tamano as u32,
            columnas: self.columnas as u32,
            filas: self.filas as u32,
            celdas: celdas.bits,
            visibles: visibles.bits,
        }
    }
}
