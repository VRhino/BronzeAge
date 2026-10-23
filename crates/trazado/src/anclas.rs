//! Las anclas (T §7): edificios que dan frente de calle a sus satélites. Todas forman un único árbol que crece desde el
//! Centro Urbano por cinco ranuras equidistantes.

use crate::azar::{cos_sen_1000, sorteo};
use crate::ciudad::{en_ventana, Ciudad};
use crate::geom::{Huella, Rect};
use crate::tipos::*;
use base::isqrt;

/// Número de ranuras de crecimiento de un ancla (T §7.2).
const RANURAS: i32 = 5;

impl Ciudad {
    fn rects_de_anclas(&self) -> Vec<Rect> {
        self.piezas().iter().filter(|a| a.rol.es_ancla()).map(|a| a.rect).collect()
    }

    /// `ranuraOcupada`: hay un ancla entre `radioInicial − 2` y `radioMáximo + 2` celdas y a menos de `0,55 × media ranura`
    /// (19,8°) de la dirección. El coseno se compara con 0,9409 en enteros: sin trigonometría.
    fn ranura_ocupada(&self, semilla: &Rect, dir: (i32, i32), otras: &[Rect]) -> bool {
        let prm = self.params();
        let (r0, r1) = ((prm.separacion_minima_anclas - 2) as i64, (3 * prm.separacion_minima_anclas + 2) as i64);
        let (sx, sy) = ((2 * semilla.col + semilla.ancho) as i64, (2 * semilla.row + semilla.alto) as i64);
        let nu = isqrt((dir.0 as i128).pow(2) + (dir.1 as i128).pow(2)) as i64;
        otras.iter().any(|o| {
            let (vx, vy) = ((2 * o.col + o.ancho) as i64 - sx, (2 * o.row + o.alto) as i64 - sy);
            let d2 = vx * vx + vy * vy;
            if d2 < 4 * r0 * r0 || d2 > 4 * r1 * r1 {
                return false;
            }
            let dot = dir.0 as i64 * vx + dir.1 as i64 * vy;
            dot * 10_000 > 9_409 * nu * isqrt(d2 as i128) as i64
        })
    }

    /// `huecoEnDireccion`: prueba radios de `separacionMinimaAnclas` a `3×` en saltos de `fondoManzana` celdas; el rectángulo
    /// centrado ahí debe estar libre y a `separacionSeguridadAnclas` (piso duro) de todas las anclas. Como T §7.2, sin disco urbano:
    /// el árbol de anclas crece por sus ranuras, no por el `radioCiudad`.
    fn hueco_en_direccion(&self, s: &Rect, dir: (i32, i32), h: Huella, otras: &[Rect]) -> Option<Rect> {
        let prm = self.params();
        let (cx2, cy2) = ((2 * s.col + s.ancho) as i64, (2 * s.row + s.alto) as i64);
        let mut r = prm.separacion_minima_anclas;
        while r <= 3 * prm.separacion_minima_anclas {
            let cx = cx2 * 500 + dir.0 as i64 * r as i64;
            let cy = cy2 * 500 + dir.1 as i64 * r as i64;
            let col = (cx - h.ancho as i64 * 500 + 500).div_euclid(1000) as i32;
            let row = (cy - h.alto as i64 * 500 + 500).div_euclid(1000) as i32;
            let rect = Rect::new(col, row, h.ancho, h.alto);
            if en_ventana(&rect, 1) && self.suelo.rect_libre(&rect) && otras.iter().all(|o| rect.hueco(o) >= prm.separacion_seguridad_anclas)
            {
                return Some(rect);
            }
            r += prm.fondo_manzana;
        }
        None
    }

    /// `crearAnclaNueva` (T §7.2): el sitio de un ancla nueva (con la huella y el id del pedido) y las semillas que se agotaron
    /// por el camino (para marcarlas `semilla_saturada` si el llamador la acepta).
    pub(crate) fn crear_ancla_nueva(&self, p: &Pedido) -> (Option<Colocacion>, Vec<u32>) {
        let prm = self.params();
        let otras = self.rects_de_anclas();
        let mut excluidas: Vec<u32> = Vec::new();
        loop {
            // la ancla real no saturada y no excluida más cercana al origen
            let Some(semilla) = self
                .piezas()
                .iter()
                .filter(|a| a.rol.es_ancla() && !a.semilla_saturada && !excluidas.contains(&a.id))
                .min_by_key(|a| (a.rect.dist2_centro_al_origen(), a.id))
            else {
                return (None, excluidas);
            };
            // 5 ranuras a 72°, la primera hacia el norte (−y) más la rotación del asentamiento (0..71°), barajadas por semilla
            let rot = sorteo(self.semilla(), "rotacion-eje", &[], 72) as i32;
            let mut orden: [i32; RANURAS as usize] = [0, 1, 2, 3, 4];
            for i in (1..RANURAS).rev() {
                let j = sorteo(self.semilla(), "ranura", &[semilla.id as i64, i as i64], i as u64 + 1) as usize;
                orden.swap(i as usize, j);
            }
            let dirs: Vec<(i32, i32)> = orden.iter().map(|&k| cos_sen_1000(-90 + rot + 72 * k)).collect();
            let (libres, usadas): (Vec<_>, Vec<_>) = dirs.into_iter().partition(|&d| !self.ranura_ocupada(&semilla.rect, d, &otras));

            let normal_primero = p.huella.es_cuadrada() || sorteo(self.semilla(), "orientacion-ancla", &[semilla.id as i64, p.id as i64], 2) == 0;
            let orientaciones: Vec<(Huella, bool)> = if p.huella.es_cuadrada() {
                vec![(p.huella, false)]
            } else if normal_primero {
                vec![(p.huella, false), (p.huella.girada(), true)]
            } else {
                vec![(p.huella.girada(), true), (p.huella, false)]
            };
            for dir in libres.into_iter().chain(usadas) {
                for &(h, rotado) in &orientaciones {
                    let Some(rect) = self.hueco_en_direccion(&semilla.rect, dir, h, &otras) else { continue };
                    if self.suelo.tiene_frente(&rect) || self.suelo.corredor(&rect, prm.cap_corredor_urbano).is_some() {
                        return (Some(Colocacion { rect, rotado }), excluidas);
                    }
                }
            }
            excluidas.push(semilla.id);
        }
    }

    /// Primera ancla de la categoría con hueco para el pedido y las que se probaron antes y quedaron llenas. Como T §7.1, un ancla
    /// queda **llena en cuanto no cabe la huella pedida** en su núcleo (el latch es permanente: no vuelve a recibir satélites).
    fn ancla_activa(&self, p: &Pedido, campo: &[u16], afines: &[bool]) -> (Option<u32>, Vec<u32>) {
        let mut llenas = Vec::new();
        for a in self.anclas_activas(p.categoria) {
            if !self.atraccion_dura(&a.rect, p, afines, campo).is_empty() {
                return (Some(a.id), llenas);
            }
            llenas.push(a.id);
        }
        (None, llenas)
    }

    /// `asegurarAnclaPara` (T §7.1): corre **antes** de pedir sitio para un edificio con barrio. Marca las anclas llenas, y si
    /// no queda ninguna con hueco (y el asentamiento está en `nivel_gate_anclas` o más) crea un ancla de saturación sorteada entre
    /// `opciones`, siempre que su núcleo tenga sitio para el pedido (gate de utilidad). El Mercado no se sortea: es su propia ancla.
    pub fn asegurar_ancla_para(&mut self, p: &Pedido, id_ancla: u32, nivel_asentamiento: u8, opciones: &[OpcionAncla]) -> Ancla {
        if !p.categoria.tiene_barrio() || self.centro().is_none() {
            return Ancla::Ninguna;
        }
        let cap = self.params().cap_corredor_urbano;
        let afines = self.mapa_de_afines(p);
        let (activa, llenas) = self.ancla_activa(p, &self.suelo.campo(cap), &afines);
        for id in llenas {
            self.marcar(id, true, false);
        }
        if let Some(id) = activa {
            return Ancla::Existente(id);
        }
        if nivel_asentamiento < self.params().nivel_gate_anclas || p.categoria == Categoria::Mercado || opciones.is_empty() {
            return Ancla::Ninguna;
        }
        // tipo de ancla de saturación: sorteo determinista por el id de la nueva
        let op = opciones[sorteo(self.semilla(), "tipo-ancla", &[id_ancla as i64], opciones.len() as u64) as usize];
        let nueva = Pedido {
            id: id_ancla,
            tipo: op.tipo,
            categoria: p.categoria,
            rol: Rol::AnclaSaturacion,
            huella: op.huella,
            permitir_rotacion: true,
            perfil: p.perfil,
            radio_ul: p.radio_ul,
        };
        let (sitio, saturadas) = self.crear_ancla_nueva(&nueva);
        let Some(sitio) = sitio else { return Ancla::Ninguna };
        for id in saturadas {
            self.marcar(id, false, true); // se conservan aunque el ancla nueva no sirva
        }
        // GATE de utilidad: si el núcleo de la ancla nueva no tiene sitio para el pedido, no se añade y se reintenta luego
        let mut prueba = self.clone();
        if prueba.colocar(&nueva, sitio).is_err() {
            return Ancla::Ninguna;
        }
        let campo = prueba.suelo.campo(cap);
        if prueba.atraccion_dura(&sitio.rect, p, &afines, &campo).is_empty() {
            return Ancla::Ninguna;
        }
        *self = prueba;
        Ancla::Creada(id_ancla)
    }
}
