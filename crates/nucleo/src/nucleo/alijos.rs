//! Los alijos de exploración en el núcleo (sistema 6, tanda 6F; `Generacion_Del_Mundo.md` §12): los trae el artefacto de mundo (posición y oro, en milésimas), cada héroe guarda los que
//! abrió (`HeroeNucleo::alijos_abiertos`, por índice del artefacto; foto `BAN36`) y `AbrirAlijo` (174) carga su oro en el carro de la columna. Las reglas puras son las del crate
//! `tecnologia` (`tecnologia::alijos`, 6A).
//!
//! No tienen tick ni eventos: abrir es una orden y verlos es una consulta sobre una rejilla de 10 u (la de la economía, `economia::Rejilla`), que hará la proyección de 6G.

use super::*;
use crate::economia::Rejilla;
use crate::estado::Dato;
use asentamiento::Recurso;
use tecnologia::alijos::{self as reglas, ErrorAlijo};

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

    /// ¿Puede el héroe buscar alijos (`buscaAlijos`)? Sin Facción, o de una sin ninguna plaza.
    pub fn puede_buscar_alijos(&self, heroe: u64) -> bool {
        self.heroe(heroe).is_some_and(|h| h.faccion.is_none_or(|f| self.plazas_de(f).next().is_none()))
    }

    /// Dónde está la columna del héroe en el mapa (dentro de un lugar no cuenta).
    fn sitio_de_la_columna(&self, heroe: u64, t: Instante) -> Option<(u64, Punto)> {
        let c = self.columna_de(heroe)?;
        Some((c, self.estado.ejercitos.get(&c)?.posicion_en(t)))
    }

    /// Los alijos que el héroe ve ahora (Niebla_De_Guerra §5, «Alijos»): si puede buscarlos, los que tiene a `VISION.jugadorSolo` de su columna en el mapa y no ha abierto, por índice.
    pub fn alijos_a_la_vista(&self, heroe: u64, t: Instante) -> Vec<u32> {
        let (Some(todos), Some(h), Some((_, pos))) = (self.alijos_del_mundo(), self.heroe(heroe), self.sitio_de_la_columna(heroe, t)) else { return vec![] };
        if !self.puede_buscar_alijos(heroe) {
            return vec![];
        }
        let vision = self.estado.reglas.economia.militar.vision.jugador_solo;
        // La rejilla da los candidatos; la regla, cuáles se ven.
        let cerca = todos.en_disco(pos, vision);
        let sitios: Vec<Punto> = cerca.iter().map(|&i| todos.lista[i as usize].posicion).collect();
        let abiertos: reglas::Abiertos = cerca.iter().enumerate().filter(|(_, i)| h.alijos_abiertos.contains(i)).map(|(k, _)| k as u32).collect();
        reglas::a_la_vista(&sitios, &abiertos, Some(pos), vision).map(|k| cerca[k as usize]).collect()
    }

    /// `AbrirAlijo`: el oro del alijo, en unidades enteras, al carro de la columna del héroe (`cargar_botin_en`: lo que no cabe se pierde, decisión 10 del plan del sistema 6), y el alijo queda
    /// abierto para él. Los rechazos no tocan nada.
    pub(super) fn ejecutar_alijo(&mut self, t: Instante, p: &Peticion) -> Result<Vec<u64>, Respuesta> {
        let Comando::AbrirAlijo { heroe, alijo } = p.comando else { unreachable!("solo abrir un alijo") };
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: &str| Respuesta::rechazo(c, m, v);
        self.autorizar_heroe(p, heroe, false)?;
        let Some(a) = self.alijo(alijo) else { return Err(rechazo(rechazos::ALIJO_DESCONOCIDO, "ese alijo no existe")) };
        let (id, sitio) = (a.id.clone(), (a.posicion, a.oro));
        let columna = self.sitio_de_la_columna(heroe, t);
        let hueco = columna.and_then(|(c, _)| Some(self.capacidad_de_carga(c)? - militar::total(&self.estado.ejercitos[&c].columna.carga))).unwrap_or(0);
        let mut abiertos = self.heroe(heroe).expect("autorizado").alijos_abiertos.clone();
        let apertura = match reglas::abrir(alijo, sitio, &mut abiertos, columna.map(|(_, p)| p), self.radio_puerta(), self.puede_buscar_alijos(heroe), hueco) {
            Ok(x) => x,
            Err(ErrorAlijo::NoPuedesBuscar) => return Err(rechazo(rechazos::ALIJO_NO_PUEDES_BUSCAR, "los alijos solo los abre quien no tiene Facción o es de una sin plazas")),
            Err(ErrorAlijo::YaAbierto) => return Err(rechazo(rechazos::ALIJO_YA_ABIERTO, "ese alijo ya lo abriste")),
            Err(ErrorAlijo::Lejos) => return Err(rechazo(rechazos::ALIJO_LEJOS, "hay que estar en el sitio, con la columna")),
        };
        let (columna, _) = columna.expect("en el sitio");
        self.heroe_mut(heroe).alijos_abiertos = abiertos;
        let cargado = self.cargar_botin_en(columna, &[(Recurso::Oro, apertura.cargado)].into_iter().collect()).get(&Recurso::Oro).copied().unwrap_or(0);
        debug_assert_eq!(cargado, apertura.cargado, "la regla y el carro dicen lo mismo de lo que cabe");
        let datos = vec![("heroe", heroe as i64), ("oro", apertura.oro), ("cargado", cargado)];
        // Solo lo oye quien lo abre (decisión del autor), no los demás de la columna aunque el oro vaya al carro común.
        self.tr.emitir(&self.estado, EventoNucleo { heroes: vec![heroe], valores: vec![("alijo", Dato::Texto(id))], ..EventoNucleo::nuevo("alijo.abierto", t, datos) });
        Ok(vec![])
    }
}
