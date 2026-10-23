//! Colocación: ¿dónde va un edificio nuevo? (T §6). Barrido de candidatos, afueras, genéricos, satélites con atracción dura
//! a un ancla, perfiles de orden y preferencia intramuros.

use crate::azar::hash;
use crate::ciudad::{celda_de, en_ventana, idx, vecinos_i, Ciudad, MITAD};
use crate::geom::{media_diagonal_mil, Huella, Rect, CELDA_MIL};
use crate::tipos::*;
use std::cmp::Ordering;

/// Un candidato con todas las claves de orden (T §6.2, §6.5).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Cand {
    pub rect: Rect,
    pub rotado: bool,
    /// 0 continúa una fila · 1 da a calle · 2 pared con pared con algo · 3 suelto (habrá que estirar la red).
    pub nivel: u8,
    pub sem: u64,
    pub hueco: i32,
    pub borde: i32,
    pub afin: i32,
    pub centro2: i64,
}

impl Cand {
    fn colocacion(&self) -> Colocacion {
        Colocacion { rect: self.rect, rotado: self.rotado }
    }
}

type Filtro<'a> = &'a dyn Fn(&Rect) -> bool;

/// Rango de esquinas mínimas que cubre todo rectángulo de `huella` cuyo centro cae a ≤ `radio_mil` del origen.
fn ventana_del_radio(radio_mil: i64, h: Huella) -> ((i32, i32), (i32, i32)) {
    let m = ((radio_mil + CELDA_MIL - 1) / CELDA_MIL) as i32 + 1;
    ((-m - h.ancho, m), (-m - h.alto, m))
}

pub(crate) fn en_disco(r: &Rect, radio_ul: i32) -> bool {
    let radio = radio_ul as i64 * 1000;
    r.dist2_centro_al_origen() <= radio * radio
}

impl Ciudad {
    pub(crate) fn cap_de(&self, cat: Categoria) -> u16 {
        if cat == Categoria::Afueras {
            self.params().cap_corredor_afueras
        } else {
            self.params().cap_corredor_urbano
        }
    }

    /// Huellas a barrer: la normal y, si procede, la girada (T §6.2).
    fn orientaciones(p: &Pedido) -> Vec<(Huella, bool)> {
        let mut v = vec![(p.huella, false)];
        if p.permitir_rotacion && !p.huella.es_cuadrada() {
            v.push((p.huella.girada(), true));
        }
        v
    }

    /// Todos los sitios posibles para un pedido, **el mejor primero**. Vacía = no cabe (T §6.1). No toca nada.
    ///
    /// El resultado depende solo de (semilla, piezas, recintos, `radio_ul`, categoría, huella, perfil).
    pub fn sitios_para(&self, p: &Pedido) -> Vec<Colocacion> {
        if self.centro().is_none() {
            return Vec::new();
        }
        match (p.rol, p.categoria) {
            (Rol::Centro, _) => Vec::new(),
            (Rol::AnclaPrimaria | Rol::AnclaSaturacion, _) => self.crear_ancla_nueva(p).0.into_iter().collect(),
            (_, Categoria::Afueras) => self.sitios_afueras(p),
            (_, Categoria::Generico) => self.sitios_genericos(p),
            (_, _) => self.sitios_satelite(p),
        }
    }

    /// El mejor sitio **con salida a la red**: el primero de [`Ciudad::sitios_para`] que tiene frente de calle o al que se
    /// puede abrir un corredor dentro del cap. `None` = no cabe.
    pub fn siguiente_sitio(&self, p: &Pedido) -> Option<Colocacion> {
        let cap = self.cap_de(p.categoria);
        self.sitios_para(p).into_iter().find(|s| self.suelo.tiene_frente(&s.rect) || self.suelo.corredor(&s.rect, cap).is_some())
    }

    // ---- barrido (T §6.2) ----

    fn coste_desde_la_red(&self, r: &Rect, campo: &[u16]) -> Option<u16> {
        let mut mejor: Option<u16> = None;
        for c in r.adyacentes() {
            let i = idx(c)?;
            if self.suelo.red[i] != 0 {
                return Some(0);
            }
            let d = campo[i];
            if d > 0 && mejor.is_none_or(|m| d < m) {
                mejor = Some(d);
            }
        }
        mejor
    }

    /// Alguna celda de red pegada al rectángulo tiene, entre sus otras vecinas, una ocupada: otro edificio da a la misma calle.
    fn continua_fila(&self, r: &Rect) -> bool {
        r.adyacentes().into_iter().filter_map(idx).filter(|&i| self.suelo.red[i] != 0).any(|i| {
            vecinos_i(i).into_iter().flatten().any(|n| self.suelo.ocup[n] && !r.contiene(celda_de(n)))
        })
    }

    fn barrido(&self, h: Huella, rotado: bool, cols: (i32, i32), rows: (i32, i32), campo: &[u16], filtro: Filtro) -> Vec<Cand> {
        let mut v = Vec::new();
        for row in rows.0.max(-MITAD)..=rows.1.min(MITAD) {
            for col in cols.0.max(-MITAD)..=cols.1.min(MITAD) {
                let rect = Rect::new(col, row, h.ancho, h.alto);
                if !en_ventana(&rect, 1) || !filtro(&rect) || !self.suelo.rect_libre(&rect) {
                    continue;
                }
                // GATE DURO: un sitio sin salida a la calle no es un sitio.
                let Some(coste) = self.coste_desde_la_red(&rect, campo) else { continue };
                let nivel = if coste == 0 {
                    if self.continua_fila(&rect) {
                        0
                    } else {
                        1
                    }
                } else if rect.adyacentes().into_iter().filter_map(idx).any(|i| !self.suelo.libre_i(i)) {
                    2
                } else {
                    3
                };
                let sem = hash(self.semilla(), "candidato", &[col as i64, row as i64, rotado as i64]);
                v.push(Cand { rect, rotado, nivel, sem, hueco: 0, borde: 0, afin: 0, centro2: rect.dist2_centro_al_origen() });
            }
        }
        v
    }

    // ---- afueras (T §6.3) ----

    fn sitios_afueras(&self, p: &Pedido) -> Vec<Colocacion> {
        let prm = self.params();
        let base_ul = p.radio_ul.max(prm.radio_afueras_min_ul + prm.ancho_banda_afueras_ul);
        let veto = prm.radio_afueras_min_ul as i64 * 1000;
        let campo = self.suelo.campo(prm.cap_corredor_afueras);
        let mut cands = Vec::new();
        for (h, rotado) in Self::orientaciones(p) {
            let radio_max = base_ul as i64 * 1000 + media_diagonal_mil(h);
            let (cols, rows) = ventana_del_radio(radio_max, h);
            let filtro = |r: &Rect| r.dist2_centro_al_origen() <= radio_max * radio_max && r.dist2_borde_al_origen() >= veto * veto;
            cands.extend(self.barrido(h, rotado, cols, rows, &campo, &filtro));
        }
        // el más lejano primero: las granjas acompañan el borde de la ciudad
        cands.sort_by(|a, b| b.centro2.cmp(&a.centro2).then(a.sem.cmp(&b.sem)));
        cands.iter().map(Cand::colocacion).collect()
    }

    // ---- genéricos (T §6.4) ----

    fn sitios_genericos(&self, p: &Pedido) -> Vec<Colocacion> {
        let campo = self.suelo.campo(self.params().cap_corredor_urbano);
        let mut cands = Vec::new();
        for (h, rotado) in Self::orientaciones(p) {
            let (cols, rows) = ventana_del_radio(p.radio_ul as i64 * 1000, h);
            cands.extend(self.barrido(h, rotado, cols, rows, &campo, &|r| en_disco(r, p.radio_ul)));
        }
        cands.sort_by(|a, b| a.centro2.cmp(&b.centro2).then(a.sem.cmp(&b.sem)));
        self.preferir_intramuros(&mut cands);
        cands.iter().map(Cand::colocacion).collect()
    }

    // ---- satélites (T §6.5) ----

    /// Anclas de la categoría que no están llenas, de más cerca a más lejos del origen.
    pub(crate) fn anclas_activas(&self, cat: Categoria) -> Vec<&Pieza> {
        let mut v: Vec<&Pieza> = self.piezas().iter().filter(|a| a.rol.es_ancla() && a.categoria == cat && !a.ancla_llena).collect();
        v.sort_by_key(|a| (a.rect.dist2_centro_al_origen(), a.id));
        v
    }

    /// Celdas de las piezas afines al pedido (mismo tipo o misma categoría), excluidas las anclas.
    pub(crate) fn mapa_de_afines(&self, p: &Pedido) -> Vec<bool> {
        let mut m = vec![false; crate::ciudad::CELDAS];
        for a in self.piezas().iter().filter(|a| !a.rol.es_ancla() && (a.tipo == p.tipo || a.categoria == p.categoria)) {
            for c in a.rect.celdas() {
                if let Some(i) = idx(c) {
                    m[i] = true;
                }
            }
        }
        m
    }

    fn sitios_satelite(&self, p: &Pedido) -> Vec<Colocacion> {
        let campo = self.suelo.campo(self.params().cap_corredor_urbano);
        let afines = self.mapa_de_afines(p);
        // T §6.5 toma «la ancla más cercana no llena». Tras `asegurar_ancla_para`, que marca llenas las que no admiten esta huella, esa
        // es la primera con sitio; recorrerlas todas solo cambia algo si se pregunta sin asegurar antes.
        for a in self.anclas_activas(p.categoria) {
            let mut cands = self.atraccion_dura(&a.rect, p, &afines, &campo);
            if !cands.is_empty() {
                self.preferir_intramuros(&mut cands);
                return cands.iter().map(Cand::colocacion).collect();
            }
        }
        Vec::new()
    }

    /// `sitiosPorAtraccionDura`: candidatos del núcleo del ancla (hueco ≤ fondo de manzana), ordenados por el perfil. Como T §6.5, sin
    /// disco urbano y dentro del círculo de barrido del TypeScript (`candidatosLibres(rectAncla, margen·T)`): centro a centro con el
    /// ancla desnuda, a ≤ `fondo + lado mayor del ancla + lado mayor del pedido` celdas.
    pub(crate) fn atraccion_dura(&self, ancla: &Rect, p: &Pedido, afines: &[bool], campo: &[u16]) -> Vec<Cand> {
        let prm = self.params();
        let rca = ancla.ensanchado(prm.ancho_calle);
        let fondo = prm.fondo_manzana;
        let margen = (fondo + ancla.ancho.max(ancla.alto) + p.huella.ancho.max(p.huella.alto)) as i64 * CELDA_MIL;
        let ca = ancla.centro();
        let en_circulo = |r: &Rect| {
            let c = r.centro();
            (c.x - ca.x).pow(2) + (c.y - ca.y).pow(2) <= margen * margen
        };
        let mut cands = Vec::new();
        for (h, rotado) in Self::orientaciones(p) {
            let cols = (rca.col - h.ancho - fondo, rca.fin_col() + fondo);
            let rows = (rca.row - h.alto - fondo, rca.fin_row() + fondo);
            let filtro = |r: &Rect| r.hueco(&rca) <= fondo && en_circulo(r);
            cands.extend(self.barrido(h, rotado, cols, rows, campo, &filtro));
        }
        for c in &mut cands {
            c.hueco = c.rect.hueco(&rca);
            c.borde = c.rect.borde_compartido(&rca);
            c.afin = c.rect.adyacentes().into_iter().filter_map(idx).filter(|&i| afines[i]).count() as i32;
        }
        let orden = p.perfil.orden();
        cands.sort_by(|a, b| comparar(a, b, &orden));
        cands
    }

    // ---- preferencia intramuros (T §6.6) ----

    /// Partición estable: con un recinto exterior completo, los candidatos cuya huella queda entera dentro van antes.
    fn preferir_intramuros(&self, cands: &mut Vec<Cand>) {
        let Some(dentro) = self.interior_de_recinto() else { return };
        let (mut dentro_v, fuera_v): (Vec<Cand>, Vec<Cand>) =
            cands.iter().copied().partition(|c| c.rect.celdas().all(|cel| idx(cel).is_some_and(|i| dentro[i])));
        dentro_v.extend(fuera_v);
        *cands = dentro_v;
    }

    // ---- reubicación (T §6.7) ----

    /// Sitio para la huella nueva de una pieza que crece (hoy, la granja al mejorar), con el suelo sin la propia pieza.
    /// `None` = no cabe en ningún sitio (la mejora no se aplica).
    ///
    /// Elige entre los sitios **con salida a la red** (T §6.7): primero los que no acercan su borde al centro, y de ellos el más cercano
    /// a la posición actual (a igual distancia, el primero de [`Ciudad::sitios_para`]); si ninguno, el más cercano de todos. La salida a
    /// la red cuesta un BFS por sitio, así que se prueba en orden de cercanía y se para en el primero que vale: da exactamente lo mismo
    /// que filtrar todos y quedarse con el mínimo, sin pagar miles de búsquedas.
    pub fn reubicar_por_tamano(&self, id: u32, nueva: Huella, perfil: Perfil, radio_ul: i32) -> Option<Colocacion> {
        let pieza = self.pieza(id)?;
        let sin = self.sin_pieza(id);
        let ped = Pedido {
            id,
            tipo: pieza.tipo,
            categoria: pieza.categoria,
            rol: pieza.rol,
            huella: nueva,
            permitir_rotacion: false,
            perfil,
            radio_ul,
        };
        let cap = sin.cap_de(pieza.categoria);
        let sitios = sin.sitios_para(&ped);
        let actual = pieza.rect;
        let (cx, cy) = (actual.centro().x, actual.centro().y);
        let dist = |s: &Colocacion| {
            let c = s.rect.centro();
            (c.x - cx) * (c.x - cx) + (c.y - cy) * (c.y - cy)
        };
        let borde_actual = actual.dist2_borde_al_origen();
        let con_salida = |s: &Colocacion| sin.suelo.tiene_frente(&s.rect) || sin.suelo.corredor(&s.rect, cap).is_some();
        let mejor = |sin_acercar: bool| {
            let mut orden: Vec<(i64, usize)> =
                sitios.iter().enumerate().filter(|(_, s)| !sin_acercar || s.rect.dist2_borde_al_origen() >= borde_actual).map(|(i, s)| (dist(s), i)).collect();
            orden.sort_unstable();
            orden.into_iter().map(|(_, i)| &sitios[i]).find(|s| con_salida(s)).copied()
        };
        mejor(true).or_else(|| mejor(false))
    }

    /// Aplica una reubicación: la pieza cambia de rectángulo conservando su sitio en el orden de construcción.
    pub fn reubicar(&mut self, id: u32, sitio: Colocacion) {
        self.mover(id, sitio);
    }
}

fn comparar(a: &Cand, b: &Cand, orden: &[crate::tipos::Clave; 5]) -> Ordering {
    use crate::tipos::Clave::*;
    for k in orden {
        // menor es mejor; `borde` y `bordeAfin` ganan al ser mayores
        let o = match k {
            Hueco => a.hueco.cmp(&b.hueco),
            Nivel => a.nivel.cmp(&b.nivel),
            Centro => a.centro2.cmp(&b.centro2),
            Borde => b.borde.cmp(&a.borde),
            BordeAfin => b.afin.cmp(&a.afin),
        };
        if o != Ordering::Equal {
            return o;
        }
    }
    a.sem.cmp(&b.sem)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    /// La definición literal de T §6.7: filtrar todos los sitios con salida y quedarse con el mínimo (la que había antes de hacerlo perezoso).
    fn por_tamano_literal(c: &Ciudad, id: u32, nueva: Huella, perfil: Perfil, radio_ul: i32) -> Option<Colocacion> {
        let pieza = c.pieza(id)?;
        let sin = c.sin_pieza(id);
        let ped = Pedido { id, tipo: pieza.tipo, categoria: pieza.categoria, rol: pieza.rol, huella: nueva, permitir_rotacion: false, perfil, radio_ul };
        let cap = sin.cap_de(pieza.categoria);
        let validos: Vec<Colocacion> =
            sin.sitios_para(&ped).into_iter().filter(|s| sin.suelo.tiene_frente(&s.rect) || sin.suelo.corredor(&s.rect, cap).is_some()).collect();
        let actual = pieza.rect;
        let (cx, cy) = (actual.centro().x, actual.centro().y);
        let dist = |s: &Colocacion| {
            let c = s.rect.centro();
            (c.x - cx) * (c.x - cx) + (c.y - cy) * (c.y - cy)
        };
        let borde_actual = actual.dist2_borde_al_origen();
        validos.iter().filter(|s| s.rect.dist2_borde_al_origen() >= borde_actual).min_by_key(|s| dist(s)).or_else(|| validos.iter().min_by_key(|s| dist(s))).copied()
    }

    /// Una ciudad recién fundada con unas granjas y casas más (el suelo de las pruebas de reubicación).
    fn ciudad_con_granjas(semilla: u64) -> (Ciudad, u32) {
        let tp = |tipo, a, h| crate::fundacion::TipoPieza { tipo, huella: Huella::new(a, h) };
        let mut c = Ciudad::fundar(&crate::fundacion::EntradaFundacion {
            semilla,
            params: Parametros::default(),
            perfil: Perfil::por_tradicion(semilla),
            primer_id: 0,
            centro: tp(1, 4, 4),
            vivienda: tp(2, 1, 1),
            granja: tp(3, 2, 2),
        });
        let granja = c.piezas().iter().find(|p| p.categoria == Categoria::Afueras).map(|p| p.id).expect("la fundación pone la granja");
        for (k, (cat, h)) in [(Categoria::Afueras, (2, 2)), (Categoria::Residencial, (1, 1)), (Categoria::Afueras, (4, 3)), (Categoria::Generico, (2, 1))].into_iter().cycle().take(24).enumerate() {
            let ped = Pedido { id: 100 + k as u32, tipo: 9, categoria: cat, rol: Rol::Normal, huella: Huella::new(h.0, h.1), permitir_rotacion: cat != Categoria::Afueras || h != (2, 2), perfil: Perfil::Nucleos, radio_ul: 60 };
            c.colocar_siguiente(&ped);
        }
        (c, granja)
    }

    #[test]
    fn la_granja_que_crece_busca_sitio_sin_su_suelo_pero_con_su_camino() {
        for semilla in [3u64, 8, 21] {
            let (c, granja) = ciudad_con_granjas(semilla);
            let sin = c.sin_pieza(granja);
            assert!(!c.caminos().is_empty(), "las granjas llegan a la red por un camino");
            assert_eq!((sin.calles(), sin.caminos()), (c.calles(), c.caminos()), "semilla {semilla}: la red es la que da con la granja (T §6.7)");
            assert!(sin.suelo.rect_libre(&c.pieza(granja).unwrap().rect), "semilla {semilla}: su suelo queda libre");
            assert!(sin.pieza(granja).is_none());
        }
    }

    #[test]
    fn reubicar_perezoso_da_lo_mismo_que_la_definicion_literal() {
        for semilla in [3u64, 8, 21] {
            let params = Parametros::default();
            let tp = |tipo, a, h| crate::fundacion::TipoPieza { tipo, huella: Huella::new(a, h) };
            let mut c = Ciudad::fundar(&crate::fundacion::EntradaFundacion {
                semilla,
                params,
                perfil: Perfil::por_tradicion(semilla),
                primer_id: 0,
                centro: tp(1, 4, 4),
                vivienda: tp(2, 1, 1),
                granja: tp(3, 2, 2),
            });
            let granja = c.piezas().iter().find(|p| p.categoria == Categoria::Afueras).map(|p| p.id).expect("la fundación pone la granja");
            // Un par de granjas más y casas para que el suelo no esté vacío.
            for (k, (cat, h)) in [(Categoria::Afueras, (2, 2)), (Categoria::Residencial, (1, 1)), (Categoria::Afueras, (4, 3)), (Categoria::Generico, (2, 1))].into_iter().cycle().take(24).enumerate() {
                let ped = Pedido { id: 100 + k as u32, tipo: 9, categoria: cat, rol: Rol::Normal, huella: Huella::new(h.0, h.1), permitir_rotacion: cat != Categoria::Afueras || h != (2, 2), perfil: Perfil::Nucleos, radio_ul: 60 };
                c.colocar_siguiente(&ped);
            }
            for h in [(2, 3), (4, 3), (6, 6), (9, 9)] {
                let (nueva, radio) = (Huella::new(h.0, h.1), 60);
                assert_eq!(c.reubicar_por_tamano(granja, nueva, Perfil::Nucleos, radio), por_tamano_literal(&c, granja, nueva, Perfil::Nucleos, radio), "semilla {semilla}, huella {h:?}");
            }
        }
    }
}
