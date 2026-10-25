//! Los alijos de exploración en el núcleo (sistema 6, tanda 6F; `Generacion_Del_Mundo.md` §12): los trae el artefacto de mundo (posición y oro, en milésimas), cada héroe guarda los que
//! abrió (`HeroeNucleo::alijos_abiertos`, por índice del artefacto; foto `BAN36`) y `AbrirAlijo` (174) carga su oro en el carro de la columna. Las reglas puras están en `crate::alijos`.
//!
//! No tienen tick ni eventos: abrir es una orden y verlos es una consulta sobre una rejilla de 10 u (la de la economía, `economia::Rejilla`), que hará la proyección de 6G.

use super::*;
use crate::alijos::{self as reglas, NoSeAbre};
use crate::economia::Rejilla;
use crate::estado::Dato;
use asentamiento::Recurso;

/// Un alijo del artefacto: su id de cadena (el del contrato), dónde está y su oro en milésimas.
pub struct Alijo {
    pub id: String,
    pub posicion: Punto,
    pub oro: Mil,
}

/// Los alijos del artefacto, en su orden (el índice es el que guarda el héroe), con su rejilla para «a la vista».
pub(crate) struct AlijosDelMundo {
    pub(crate) lista: Vec<Alijo>,
    rejilla: Rejilla,
}

impl AlijosDelMundo {
    pub(crate) fn de(m: &Mundo) -> Self {
        let lista: Vec<Alijo> = m.alijos().iter().map(|a| Alijo { id: a.id.clone(), posicion: a.posicion.as_ref().map(|p| Punto::new(p.x, p.y)).unwrap_or_default(), oro: a.oro }).collect();
        let (ancho, alto) = m.tamano();
        let rejilla = Rejilla::nueva(ancho, alto, lista.iter().enumerate().map(|(i, a)| (i as u32, a.posicion)));
        Self { lista, rejilla }
    }

    /// Los que caen a `radio` o menos de `centro`, por índice.
    fn en_disco(&self, centro: Punto, radio: Mil) -> Vec<u32> {
        self.rejilla.en_disco(centro, radio, |i| self.lista[i as usize].posicion)
    }
}

impl Nucleo {
    fn alijos_del_mundo(&self) -> Option<&AlijosDelMundo> {
        self.geografia.as_ref().map(|g| &g.alijos)
    }

    /// El alijo `i` del artefacto.
    pub fn alijo(&self, i: u32) -> Option<&Alijo> {
        self.alijos_del_mundo()?.lista.get(i as usize)
    }

    /// El índice del alijo con ese id del artefacto (el del contrato). Recorre la lista: son unos cientos y solo lo pide una orden.
    pub fn alijo_por_id(&self, id: &str) -> Option<u32> {
        self.alijos_del_mundo()?.lista.iter().position(|a| a.id == id).map(|i| i as u32)
    }

    /// ¿Puede el héroe buscar alijos? Sin Facción, o de una sin ninguna plaza.
    pub fn puede_buscar_alijos(&self, heroe: u64) -> bool {
        self.heroe(heroe).is_some_and(|h| reglas::puede_buscar(h.faccion, |f| self.plazas_de(f).next().is_some()))
    }

    /// Los alijos que el héroe ve ahora (Niebla_De_Guerra §5, «Alijos»): si puede buscarlos, los que tiene a `VISION.jugadorSolo` de su columna en el mapa y no ha abierto, por índice.
    pub fn alijos_a_la_vista(&self, heroe: u64, t: Instante) -> Vec<u32> {
        let (Some(todos), Some(h), Some(col)) = (self.alijos_del_mundo(), self.heroe(heroe), self.columna_de(heroe).and_then(|c| self.estado.ejercitos.get(&c))) else { return vec![] };
        let puede = self.puede_buscar_alijos(heroe);
        let (pos, vision) = (col.posicion_en(t), self.estado.reglas.economia.militar.vision.jugador_solo);
        todos.en_disco(pos, vision).into_iter().filter(|&i| reglas::a_la_vista(puede, h.alijos_abiertos.contains(&i), Some(todos.lista[i as usize].posicion.dist2(pos)), vision)).collect()
    }

    /// `AbrirAlijo`: el oro del alijo, en unidades enteras, al carro de la columna del héroe (`cargar_botin_en`: lo que no cabe se pierde, decisión 10 del plan del sistema 6), y el alijo queda
    /// abierto para él. Los rechazos no tocan nada.
    pub(super) fn ejecutar_alijo(&mut self, t: Instante, p: &Peticion) -> Result<Vec<u64>, Respuesta> {
        let Comando::AbrirAlijo { heroe, alijo } = p.comando else { unreachable!("solo abrir un alijo") };
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: &str| Respuesta::rechazo(c, m, v);
        self.autorizar_heroe(p, heroe, false)?;
        let (id, posicion, oro) = match self.alijo(alijo) {
            Some(a) => (a.id.clone(), a.posicion, reglas::oro(a.oro)),
            None => return Err(rechazo(rechazos::ALIJO_DESCONOCIDO, "ese alijo no existe")),
        };
        let columna = self.columna_de(heroe).filter(|c| self.estado.ejercitos.contains_key(c));
        let distancia2 = columna.map(|c| self.estado.ejercitos[&c].posicion_en(t).dist2(posicion));
        let abierto = self.heroe(heroe).expect("autorizado").alijos_abiertos.contains(&alijo);
        match reglas::abrir(self.puede_buscar_alijos(heroe), abierto, distancia2, self.radio_puerta()) {
            Ok(()) => {}
            Err(NoSeAbre::NoPuedesBuscar) => return Err(rechazo(rechazos::ALIJO_NO_PUEDES_BUSCAR, "los alijos solo los abre quien no tiene Facción o es de una sin plazas")),
            Err(NoSeAbre::YaAbierto) => return Err(rechazo(rechazos::ALIJO_YA_ABIERTO, "ese alijo ya lo abriste")),
            Err(NoSeAbre::Lejos) => return Err(rechazo(rechazos::ALIJO_LEJOS, "hay que estar en el sitio, con la columna")),
        }
        let columna = columna.expect("en el sitio");
        self.heroe_mut(heroe).alijos_abiertos.insert(alijo);
        let cargado = self.cargar_botin_en(columna, &[(Recurso::Oro, oro)].into_iter().collect()).get(&Recurso::Oro).copied().unwrap_or(0);
        let datos = vec![("heroe", heroe as i64), ("oro", oro), ("cargado", cargado)];
        self.tr.emitir(&self.estado, EventoNucleo { ejercitos: vec![columna], valores: vec![("alijo", Dato::Texto(id))], ..EventoNucleo::nuevo("alijo.abierto", t, datos) });
        Ok(vec![])
    }
}
