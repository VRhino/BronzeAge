//! El vigía del batch de comercio: lee el registro de cambios del núcleo cada pocos minutos de mundo y lo reduce a lo que el informe necesita
//! (trueques con sus tiempos, viajes de caravana, peajes, comisiones, compras de mercenarios, pueblos y acarreos). No guarda los hechos: el
//! registro del núcleo retiene un número limitado de versiones y una corrida de 30 días lo desborda.

use base::Instante;
use nucleo::Nucleo;
use std::collections::BTreeMap;

const DIA: i64 = 24 * 60 * base::MS_POR_MINUTO;

/// Un trueque desde que se propone hasta que se salda.
#[derive(Clone, Debug, Default)]
pub struct DatosTrueque {
    pub propuesto: Instante,
    pub aceptado: Option<Instante>,
    pub cumplido: Option<Instante>,
    pub entre_facciones: bool,
}

/// Un viaje de ida de una caravana: de la salida a la entrega.
#[derive(Clone, Copy, Debug)]
pub struct Viaje {
    pub origen: u64,
    pub destino: u64,
    pub dura_ms: i64,
    pub carga: i64,
    /// Ciudades ajenas cuyo peaje pagó en este viaje.
    pub peajes: u32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Compra {
    pub veces: u64,
    pub unidades: i64,
    pub oro: i64,
}

#[derive(Default)]
pub struct Vigia {
    visto: u64,
    /// Versiones del registro que se perdieron antes de leerlas (debe ser 0).
    pub huecos: u64,
    /// Cuántas veces salió cada código de hecho.
    pub hechos: BTreeMap<String, u64>,
    pub trueques: BTreeMap<u64, DatosTrueque>,
    /// Trueques cumplidos en los que se movió cobre, estaño o hierro.
    pub trueques_de_minerales: u64,
    /// Tiempo (ms) de la aceptación de un trueque a cada salida de caravana que lo sirve.
    pub repartos_ms: Vec<i64>,
    /// Trueques vencidos con lo pactado sin entregar (penalizan la reputación).
    pub incumplidos: u64,
    pub viajes: Vec<Viaje>,
    /// Caravana → (salida, origen, destino) mientras viaja.
    en_ruta: BTreeMap<u64, (Instante, u64, u64, u32)>,
    /// Peajes por Facción que paga: (cobros, unidades).
    pub peajes_pagados: BTreeMap<u32, (u64, i64)>,
    pub peajes_cobrados: BTreeMap<u32, (u64, i64)>,
    pub comision: i64,
    pub recargo: i64,
    pub carga_entregada: i64,
    pub perdido_en_entrega: i64,
    /// Lo que las caravanas volcaron al volver a casa y lo que no cupo.
    pub volcado_en_casa: i64,
    pub perdido_en_casa: i64,
    /// Compras de mercenarios por recurso (índice del enum).
    pub compras: BTreeMap<usize, Compra>,
    pub compras_residente: u64,
    pub oro_de_compras: i64,
    /// Acarreo: lo que sacan los carros de los pueblos, lo que ingresa en la madre y lo que no cupo.
    pub acarreo_sacado: i64,
    pub acarreo_carga: i64,
    pub acarreo_perdido: i64,
    /// Instante en que se marcó y se completó cada pueblo.
    pub pueblos_marcados: BTreeMap<u64, Instante>,
    pub pueblos_completados: BTreeMap<u64, Instante>,
    /// Plazas arruinadas (id → instante).
    pub ruinas: BTreeMap<u64, Instante>,
    /// Lo que compra a los mercenarios cada columna residente en un día, por recurso: lo que el cupo diario debe acotar.
    pub compras_de_residentes: BTreeMap<(u64, i64, usize), i64>,
    pub animales_comprados: u64,
}

fn dato(datos: &[(&'static str, i64)], clave: &str) -> i64 {
    datos.iter().find(|(k, _)| *k == clave).map_or(0, |&(_, v)| v)
}

impl Vigia {
    /// Cuántas veces salió un hecho.
    pub fn n(&self, codigo: &str) -> u64 {
        self.hechos.get(codigo).copied().unwrap_or(0)
    }

    /// Lee los hechos nuevos del núcleo. `faccion_de` da la Facción de una plaza (también las que ya no existen).
    pub fn leer(&mut self, n: &Nucleo, faccion_de: &dyn Fn(u64) -> Option<u32>) {
        let Some(cambios) = n.cambios_desde(self.visto) else {
            self.huecos += n.version() - self.visto;
            self.visto = n.version();
            return;
        };
        for c in cambios {
            for e in &c.eventos {
                self.hecho(&e.codigo, e.en, &e.asentamientos, &e.datos, faccion_de);
            }
            self.visto = c.version;
        }
    }

    fn hecho(&mut self, codigo: &str, t: Instante, plazas: &[u64], datos: &[(&'static str, i64)], faccion_de: &dyn Fn(u64) -> Option<u32>) {
        *self.hechos.entry(codigo.to_string()).or_insert(0) += 1;
        let acuerdo = dato(datos, "acuerdo") as u64;
        let caravana = dato(datos, "caravana") as u64;
        match codigo {
            "comercio.trueque_propuesto" => {
                self.trueques.insert(acuerdo, DatosTrueque { propuesto: t, ..DatosTrueque::default() });
            }
            "comercio.trueque_aceptado" => {
                if let Some(x) = self.trueques.get_mut(&acuerdo) {
                    x.aceptado = Some(t);
                }
            }
            "comercio.trueque_cumplido" => {
                if let Some(x) = self.trueques.get_mut(&acuerdo) {
                    x.cumplido = Some(t);
                    x.entre_facciones = dato(datos, "entreFacciones") == 1;
                }
                self.trueques_de_minerales += datos.iter().any(|(k, _)| ["cobre", "estano", "hierro"].contains(k)) as u64;
            }
            "comercio.trueque_expirado" => self.incumplidos += (dato(datos, "incumplido") == 1) as u64,
            "comercio.caravana_sale" => {
                self.en_ruta.insert(caravana, (t, plazas[0], plazas.get(1).copied().unwrap_or(plazas[0]), 0));
                if let Some(aceptado) = self.trueques.get(&acuerdo).and_then(|x| x.aceptado).filter(|_| datos.iter().any(|(k, _)| *k == "acuerdo")) {
                    self.repartos_ms.push(t - aceptado);
                }
            }
            "comercio.caravana_llega" => {
                self.comision += dato(datos, "comision");
                self.recargo += dato(datos, "recargo");
                self.carga_entregada += dato(datos, "carga");
                self.perdido_en_entrega += dato(datos, "perdido");
                if let Some((sale, origen, destino, peajes)) = self.en_ruta.remove(&caravana) {
                    self.viajes.push(Viaje { origen, destino, dura_ms: t - sale, carga: dato(datos, "carga"), peajes });
                }
            }
            "comercio.caravana_en_casa" => {
                self.volcado_en_casa += dato(datos, "volcado");
                self.perdido_en_casa += dato(datos, "perdido");
            }
            "comercio.caravana_perdida" => drop(self.en_ruta.remove(&caravana)),
            "comercio.caravana_animal_comprado" => self.animales_comprados += 1,
            "comercio.peaje_paso" => {
                let unidades: i64 = datos.iter().filter(|(k, _)| !["caravana", "ciudad"].contains(k)).map(|&(_, v)| v).sum();
                let (origen, ciudad) = (self.en_ruta.get(&caravana).map(|x| x.1), plazas.first().copied());
                if let Some(x) = self.en_ruta.get_mut(&caravana) {
                    x.3 += 1;
                }
                if let (Some(p), Some(c)) = (origen.and_then(faccion_de), ciudad.and_then(faccion_de)) {
                    for (mapa, f) in [(&mut self.peajes_pagados, p), (&mut self.peajes_cobrados, c)] {
                        let e = mapa.entry(f).or_insert((0, 0));
                        *e = (e.0 + 1, e.1 + unidades);
                    }
                }
            }
            "mercenarios.comprado" => {
                let c = self.compras.entry(dato(datos, "recurso") as usize).or_default();
                *c = Compra { veces: c.veces + 1, unidades: c.unidades + dato(datos, "cantidad"), oro: c.oro + dato(datos, "oro") };
                if dato(datos, "residente") == 1 {
                    self.compras_residente += 1;
                    *self.compras_de_residentes.entry((dato(datos, "heroe") as u64, t.div_euclid(DIA), dato(datos, "recurso") as usize)).or_insert(0) += dato(datos, "cantidad");
                }
                self.oro_de_compras += dato(datos, "oro");
            }
            "pueblo.marcado" => drop(self.pueblos_marcados.insert(dato(datos, "pueblo") as u64, t)),
            "pueblo.completado" => drop(self.pueblos_completados.insert(dato(datos, "pueblo") as u64, t)),
            "acarreo.sale" => {
                self.acarreo_sacado += dato(datos, "carga");
                // El carro de acarreo paga peajes como una caravana más: su Facción es la de la madre.
                self.en_ruta.insert(caravana, (t, plazas[0], plazas[0], 0));
            }
            "acarreo.llega" => {
                self.acarreo_carga += dato(datos, "carga");
                self.acarreo_perdido += dato(datos, "perdido");
                self.en_ruta.remove(&caravana);
            }
            "asentamiento.ruinas" => drop(self.ruinas.insert(plazas[0], t)),
            _ => {}
        }
    }
}
