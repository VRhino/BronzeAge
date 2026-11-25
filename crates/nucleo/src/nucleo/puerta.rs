//! La puerta de cada lugar del mapa es su contorno (Tamaños y distancias del mapa, bloque C; decisión del autor del 2026-11-23). Cada plaza, pueblo, aldea,
//! avanzada y campamento ocupa un disco con el radio de `TAMANOS_MAPA` (la plaza, el de su nivel y su muralla; la aldea, el de su nivel; la avanzada, el de
//! su etapa). **Estar a la puerta** es estar a `MOVIMIENTO.radioPuerta` (1 u) o menos de ese borde, por cualquier punto del contorno: a `radio + radioPuerta`
//! o menos del centro. Marchar a un lugar acaba en el contorno, por donde llega la ruta; al salir, la columna nace en el borde; y atacar se mide desde el borde.
//! Hasta entonces la puerta era el centro, a 2 u.

use super::*;
use asentamiento::avanzadas::Avanzada;
use asentamiento::pueblos::{Pueblo, Sitio};
use militar::convocatoria::Lugar;
use crate::estado_comercio::Trazado;

/// Direcciones que se prueban, en orden, para el punto de salida: del sur en sentido horario, de 22,5° en 22,5° (en milésimas del radio).
const DIRECCIONES: [(i64, i64); 16] = [
    (0, -1000), (-383, -924), (-707, -707), (-924, -383), (-1000, 0), (-924, 383), (-707, 707), (-383, 924),
    (0, 1000), (383, 924), (707, 707), (924, 383), (1000, 0), (924, -383), (707, -707), (383, -924),
];

impl Nucleo {
    /// Lo que mide una plaza en el mapa: el radio de su nivel, con o sin muralla.
    pub fn radio_de_plaza(&self, a: &crate::estado::Asentamiento) -> Mil {
        self.estado.reglas.economia.datos.tamanos.plaza(a.nivel(), !a.economia.recintos.is_empty())
    }

    /// Lo que mide un pueblo extractor, o una aldea con sus campos en su nivel.
    pub(crate) fn radio_de_pueblo(&self, p: &Pueblo) -> Mil {
        let t = &self.estado.reglas.economia.datos.tamanos;
        match p.sitio {
            Sitio::Nodo(_) => t.pueblo,
            Sitio::Punto { .. } => t.aldea(p.nivel),
        }
    }

    /// Lo que mide una avanzada en su etapa.
    pub(crate) fn radio_de_avanzada(&self, v: &Avanzada) -> Mil {
        self.estado.reglas.economia.datos.tamanos.avanzada(v.etapa.indice() as u32 + 1)
    }

    /// Lo que mide un campamento de mercenarios.
    pub(crate) fn radio_de_campamento(&self) -> Mil {
        self.estado.reglas.economia.datos.tamanos.campamento_mercenarios
    }

    /// El centro y el radio de una plaza o de un campamento.
    pub(crate) fn huella_de(&self, lugar: Lugar) -> Option<(Punto, Mil)> {
        match lugar {
            Lugar::Asentamiento(a) => self.estado.asentamientos.get(&(a as u64)).map(|a| (a.posicion, self.radio_de_plaza(a))),
            Lugar::Campamento(c) => self.puerta_de_campamento(c).map(|p| (p, self.radio_de_campamento())),
        }
    }

    /// ¿Está `pos` a la puerta del disco de `centro` y `radio`? A `radioPuerta` o menos de su borde.
    pub(crate) fn a_la_puerta(&self, centro: Punto, radio: Mil, pos: Punto) -> bool {
        pos.dist(centro) <= radio + self.radio_puerta()
    }

    /// ¿Está `pos` a la puerta de esa plaza o de ese campamento? `false` si no existe.
    pub(crate) fn a_la_puerta_de(&self, lugar: Lugar, pos: Punto) -> bool {
        self.huella_de(lugar).is_some_and(|(c, r)| self.a_la_puerta(c, r, pos))
    }

    /// ¿Está `pos` a la puerta de la plaza `a`?
    pub(crate) fn en_la_puerta_de_plaza(&self, a: &crate::estado::Asentamiento, pos: Punto) -> bool {
        self.a_la_puerta(a.posicion, self.radio_de_plaza(a), pos)
    }

    /// Lo que está fuera de `centro`, medido desde el borde de lo que ocupa: la distancia al centro menos su radio, nunca negativa.
    pub(crate) fn desde_el_borde(centro: Punto, radio: Mil, pos: Punto) -> Mil {
        (pos.dist(centro) - radio).max(0)
    }

    /// El radio de lo que el mapa tiene **exactamente** en `centro` (una plaza, un pueblo, una aldea, una avanzada o un campamento de mercenarios), o 0 si
    /// no hay nada: para las comprobaciones que solo conocen el punto (el origen de una Caravana de Fundación, el punto de una batalla).
    pub fn radio_en(&self, centro: Punto) -> Mil {
        self.huellas_del_mapa().find(|&(c, _)| c == centro).map_or(0, |(_, r)| r)
    }

    /// Lo que ocupa el mapa, con su radio de hoy: las plazas, los pueblos y las aldeas, las avanzadas (también las ruinas sin dueño) y los campamentos de mercenarios.
    fn huellas_del_mapa(&self) -> impl Iterator<Item = (Punto, Mil)> + '_ {
        let nodos = self.geografia.as_ref().map_or(&[][..], |g| &g.mapa.nodos[..]);
        let plazas = self.estado.asentamientos.values().map(|a| (a.posicion, self.radio_de_plaza(a)));
        let pueblos = self.estado.asentamientos.values().flat_map(|a| a.economia.pueblos.iter()).filter_map(move |p| crate::economia::posicion_de_pueblo(nodos, p).map(|pos| (pos, self.radio_de_pueblo(p))));
        let avanzadas = self.estado.asentamientos.values().flat_map(|a| a.economia.avanzadas.iter()).chain(self.estado.combate.huerfanas.values()).map(|v| (v.posicion, self.radio_de_avanzada(v)));
        let campamentos = self.geografia.iter().flat_map(|g| g.mapa.campamentos.iter().map(|&c| (c, self.radio_de_campamento())));
        plazas.chain(pueblos).chain(avanzadas).chain(campamentos)
    }

    /// La huella que tiene `p` dentro (a su radio o menos del centro), la de centro más cercano si hay varias.
    pub(crate) fn huella_que_contiene(&self, p: Punto) -> Option<(Punto, Mil)> {
        self.huellas_del_mapa().filter(|&(c, r)| p.dist2(c) <= (r as i128) * (r as i128)).min_by_key(|&(c, _)| (p.dist2(c), c.x, c.y))
    }

    /// Adónde va de verdad una marcha a `destino`: si cae dentro de lo que ocupa un lugar, al punto de su contorno más cercano (o al centro, si es el centro:
    /// entonces la corta [`Nucleo::cortar_en_el_contorno`] por donde llegue la ruta). Fuera de todo lugar, el mismo `destino`.
    pub(crate) fn destino_de_marcha(&self, destino: Punto) -> Punto {
        match self.huella_que_contiene(destino) {
            Some((c, r)) if c != destino => {
                let d = destino.dist(c).max(1);
                Punto::new(c.x + (destino.x - c.x) * r / d, c.y + (destino.y - c.y) * r / d)
            }
            _ => destino,
        }
    }

    /// El camino de una marcha a `destino`, cortado donde entra en lo que ocupa el lugar al que va (si va a uno): la columna espera fuera, en su contorno.
    pub(crate) fn cortar_en_el_contorno(&self, camino: rutas::Camino, destino: Punto) -> rutas::Camino {
        match self.huella_que_contiene(destino) {
            Some((c, r)) => camino.hasta_el_contorno(c, r),
            None => camino,
        }
    }

    /// Dónde nace la columna que sale de una plaza o de un campamento: en su contorno (ver [`Nucleo::salida_en`]).
    pub(crate) fn salida_de(&self, lugar: Lugar) -> Option<Punto> {
        self.huella_de(lugar).map(|(c, r)| self.salida_en(c, r))
    }

    /// El punto del contorno de `centro` y `radio` por donde sale una columna: el primero transitable del sur en sentido horario (la salida no trae destino:
    /// lo da después `marchar_a`). Sin ninguno transitable, el del sur; con radio 0, el centro.
    pub(crate) fn salida_en(&self, c: Punto, r: Mil) -> Punto {
        let en = |&(dx, dy): &(i64, i64)| Punto::new(c.x + dx * r / 1000, c.y + dy * r / 1000);
        DIRECCIONES.iter().map(en).find(|&p| self.geografia.as_ref().is_none_or(|g| g.transitable(p))).unwrap_or_else(|| en(&DIRECCIONES[0]))
    }

    /// El borde de lo que el mapa tiene en `centro` (ver [`Nucleo::radio_en`]); el mismo punto si no hay nada (unas ruinas que ya no están).
    pub fn al_borde(&self, centro: Punto) -> Punto {
        self.salida_en(centro, self.radio_en(centro))
    }

    /// Las opciones de la ruta de una columna (marchar, perseguir): suavizada y rodeando las plazas (Tamaños y distancias del mapa, bloque E,
    /// 2026-11-23). Cada plaza es un disco de su radio de hoy; un tramo que se mete en uno cuesta `MOVIMIENTO.factorPasoPorPlaza` veces lo suyo, así
    /// que la ruta solo la cruza si no hay otro camino. Pueblos, aldeas, avanzadas y campamentos no estorban.
    pub(crate) fn opciones_de_columna(&self) -> rutas::Opciones {
        let factor = self.estado.reglas.economia.militar.movimiento.factor_paso_por_plaza;
        let discos = self.estado.asentamientos.values().map(|a| (a.posicion, self.radio_de_plaza(a))).filter(|&(_, r)| r > 0).collect();
        rutas::Opciones { estorbos: Some(std::sync::Arc::new(rutas::Estorbos { discos, factor })), ..rutas::Opciones::de_columna() }
    }

    /// El trazado entre el contorno de `inicio` y el de `fin` (cada uno un disco `(centro, radio)`): se quita lo que va dentro de cada uno, con los costes en
    /// proporción, y las paradas de peaje que quedan fuera se descartan o se corren. Lo usa el carro de acarreo, que sale del borde de su pueblo y llega al de
    /// su plaza (bloque E, 2026-11-23).
    pub(crate) fn trazado_entre_contornos(tz: &Trazado, inicio: (Punto, Mil), fin: (Punto, Mil)) -> Trazado {
        let espaciado = 1;
        let camino = rutas::Camino { puntos: tz.puntos.clone(), costes: tz.costes.clone(), nodos: Vec::new(), espaciado };
        let hasta_fin = camino.hasta_el_contorno(fin.0, fin.1);
        // El principio, al revés: se corta desde el final del camino invertido.
        let mut al_reves = rutas::Camino { puntos: hasta_fin.puntos.iter().rev().copied().collect(), costes: hasta_fin.costes.iter().rev().copied().collect(), nodos: Vec::new(), espaciado };
        al_reves = al_reves.hasta_el_contorno(inicio.0, inicio.1);
        let (puntos, costes): (Vec<Punto>, Vec<i64>) = (al_reves.puntos.into_iter().rev().collect(), al_reves.costes.into_iter().rev().collect());
        if puntos.len() < 2 {
            // Los dos contornos se tocan o el camino era nada: el de siempre.
            return tz.clone();
        }
        let quitado = comercio::viaje::longitud_ruta(&hasta_fin.puntos) - comercio::viaje::longitud_ruta(&puntos);
        let largo = comercio::viaje::longitud_ruta(&puntos);
        let paradas = tz.paradas.iter().map(|&(c, d)| (c, d - quitado)).filter(|&(_, d)| (0..=largo).contains(&d)).collect();
        Trazado { puntos, costes, paradas }
    }

    /// Lo que dura la estancia de una caravana dentro de una plaza en la que descarga sin encargo (el carro de acarreo) o que cruza (la escala): `tiempoDescarga`
    /// por carro con animal, nunca menos de `ENCARGO.paradaMinimaMinutos` (bloque E, 2026-11-23).
    pub(crate) fn estancia_en_plaza(&self, carros_con_animal: usize) -> i64 {
        let e = &self.estado.reglas.economia.comercio.encargo;
        (e.tiempo_descarga * carros_con_animal as i64).max(e.parada_minima)
    }
}
