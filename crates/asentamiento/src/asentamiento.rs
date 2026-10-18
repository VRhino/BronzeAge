//! El asentamiento: el estado de una plaza y su paso de **un minuto de mundo** (Economia_Construccion §1, con la decisión del
//! 2026-10-18: cada asentamiento avanza un minuto por evento recurrente, se mire o no, en enteros y sin tick global).
//!
//! El orden dentro de un minuto es el de las reglas de Mechanics: obras (completar, arrancar), producción, recetas, mejoras,
//! auto-construcción; luego políticas que caducan, ascenso, comida y hambre, crecimiento, recaudación y mantenimiento. Las órdenes
//! de los jugadores (cola, mejoras, ascenso, políticas) están en `obras`, `ascenso` y aquí; la geometría (dónde cae cada edificio) y los
//! yacimientos del mapa entran por los traits [`Sitios`] y [`Fuentes`].

use crate::almacen::Almacen;
use crate::catalogo::Tipo;
use crate::datos::{por, Reglas};
use crate::edificio::{Edificio, Edificios, Estado, Fuente};
use crate::mantenimiento::{self, Medidor};
use crate::poblacion::{self, Entorno, Poblacion};
use crate::politicas::Politicas;
use crate::produccion::{self, Fuentes};
use crate::recurso::{Costo, Recurso};
use base::{Instante, Mil, MS_POR_MINUTO};

/// Una obra de ascenso en curso (Doc 4.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ascenso {
    pub nivel_objetivo: u32,
    pub iniciado_en: Instante,
    pub completa_en: Instante,
}

/// Dónde puede ir un edificio nuevo: la geometría de la ciudad y los nodos o parcelas libres del mapa. Lo implementa el núcleo
/// (con el trazado urbano y el mundo); las pruebas, con un doble.
pub trait Sitios {
    /// ¿Hay sitio para este tipo? `Some(fuente)` si lo hay (la fuente libre más cercana en la zona si el tipo ligue a una), `None` si no.
    /// No toma nada: solo mira.
    fn proponer(&mut self, tipo: Tipo) -> Option<Option<Fuente>>;
    /// El proyecto se comprometió: la fuente queda tomada.
    fn tomar(&mut self, tipo: Tipo, fuente: Option<Fuente>);
}

/// Lo que el asentamiento necesita del resto del mundo en cada minuto.
pub struct Exterior<'a> {
    pub reglas: &'a Reglas,
    pub fuentes: &'a mut dyn Fuentes,
    pub sitios: &'a mut dyn Sitios,
    /// `fertilidadZona` (ppm).
    pub fertilidad: i64,
    pub tiene_tecnologia: &'a dyn Fn(&str) -> bool,
    pub factor_tecnologia: &'a dyn Fn(Recurso) -> i64,
    pub nivel_faccion: u32,
    /// Orden de la Era que ha alcanzado el servidor (la Maravilla pide la V).
    pub orden_era: u32,
    /// Este asentamiento es la capital de su Facción, y la Facción ya tiene una Gran Fundición.
    pub es_capital: bool,
    pub faccion_tiene_gran_fundicion: bool,
    /// Ciudadanos (héroes con casa) en el asentamiento.
    pub ciudadanos: u32,
    /// Distancia a la capital de la Facción (milésimas de unidad de mapa); 0 si es la capital o no hay.
    pub distancia_capital: Mil,
    /// Nivel del mejor recinto completo de muralla (0 = ninguno, 2 = de piedra).
    pub recinto_nivel: u32,
    /// Trigo por minuto de la ración de la guarnición (otro sistema); entra en el déficit y en la reserva de comida.
    pub racion_guarnicion: Mil,
    /// Multiplicadores de otros sistemas (ocupación, Aedas), ppm; `UNO` si no hay.
    pub felicidad_extra: i64,
    pub nobleza_extra: i64,
    pub recaudacion_extra: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Evento {
    EdificioCompletado { id: u32, tipo: Tipo },
    ObraIniciada { id: u32, tipo: Tipo, completa_en: Instante },
    MejoraIniciada { id: u32, nivel: u8 },
    MejoraCompletada { id: u32, nivel: u8 },
    /// La auto-construcción comprometió un proyecto.
    NecesidadDetectada { id: u32, tipo: Tipo, prioridad: i64 },
    /// Un jugador añadió, quitó o reordenó la cola.
    Encolada { id: u32, tipo: Tipo },
    Desencolada { id: u32 },
    ColaReordenada,
    PrimerosArtesanos,
    PrimerosNobles,
    Hambruna { pesants: Mil, artesanos: Mil, nobleza: Mil },
    PoliticaExpirada { id: String },
    MantenimientoDeficit,
    MantenimientoColapsado { nivel: u32 },
    MantenimientoRecuperado { nivel: u32 },
    Ruinas { faltantes: Vec<Recurso> },
    AscensoIniciado { objetivo: u32 },
    NivelSubio { nivel: u32 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asentamiento {
    pub fundado_en: Instante,
    /// Nivel alcanzado (monótono): fija el techo de población y de zona.
    pub nivel: u32,
    pub medidor: Medidor,
    pub poblacion: Poblacion,
    pub almacen: Almacen,
    pub politicas: Politicas,
    /// El orden es el historial de crecimiento y nunca se reordena (la cola reordena `prioridad`).
    pub edificios: Vec<Edificio>,
    pub siguiente_edificio: u32,
    pub ascenso: Option<Ascenso>,
    pub recetas_pausadas: Vec<Recurso>,
    pub auto_pausada: bool,
    /// Reserva manual del Tesorero por recurso (milésimas); solo cuenta en el camino automático.
    pub reserva_manual: [Mil; 29],
    /// Tras una conquista el mantenimiento se suspende.
    pub ocupado: bool,
    /// Minutos seguidos que un extractor propuesto no pudo comprometerse (desempate por urgencia).
    pub extractor_minutos_sin_cupo: Vec<(Tipo, i64)>,
    /// Edificios completados desde la última lectura: el núcleo agranda la zona uno por cada uno.
    pub completados_pendientes: u32,
    pub destruido: bool,
    /// El último minuto integrado.
    pub ultimo_minuto: Instante,
}

impl Asentamiento {
    /// Estado inicial de una fundación (Economia_Construccion §3): Centro Urbano, tres Viviendas y una Granja ya activos, 20
    /// pesants, los materiales iniciales en el almacén y todos los recursos con la capacidad de partida.
    pub fn fundar(r: &Reglas, fundado_en: Instante, materiales: &[(Recurso, Mil)], viviendas_iniciales: u32) -> Self {
        let mut almacen = Almacen::nuevo(r.capacidad_inicial);
        materiales.iter().for_each(|&(rec, n)| almacen.poner(rec, n));
        let mut edificios = vec![Edificio::nuevo(0, Tipo::CentroUrbano, Estado::Activo)];
        let mut id = 1;
        for _ in 0..viviendas_iniciales {
            edificios.push(Edificio::nuevo(id, Tipo::Vivienda, Estado::Activo));
            id += 1;
        }
        edificios.push(Edificio::nuevo(id, Tipo::Granja, Estado::Activo));
        Asentamiento {
            fundado_en,
            nivel: 1,
            medidor: Medidor::nuevo(&r.mantenimiento),
            poblacion: Poblacion::nueva(&r.poblacion),
            almacen,
            politicas: Politicas::default(),
            edificios,
            siguiente_edificio: id + 1,
            ascenso: None,
            recetas_pausadas: vec![],
            auto_pausada: false,
            reserva_manual: [0; 29],
            ocupado: false,
            extractor_minutos_sin_cupo: vec![],
            completados_pendientes: 0,
            destruido: false,
            ultimo_minuto: fundado_en,
        }
    }

    pub fn nivel_actual(&self) -> u32 {
        self.medidor.nivel_actual
    }

    pub fn edificio(&self, id: u32) -> Option<&Edificio> {
        self.edificios.iter().find(|e| e.id == id)
    }

    /// Cupos de habitantes que dan los edificios activos (Viviendas y Palacio), en milésimas.
    pub fn capacidades(&self, r: &Reglas) -> (Mil, Mil, Mil) {
        let (mut p, mut a, mut n) = (0, 0, 0);
        for e in self.edificios.iter().filter(|e| e.activo()) {
            let def = r.catalogo.tipo(e.tipo);
            p += def.capacidad_pesants;
            a += def.capacidad_artesanos;
            if e.tipo == Tipo::Palacio {
                n += def.nivel(e.nivel_interno).map_or(0, |l| l.capacidad_nobles);
            }
        }
        (p, a, n)
    }

    /// Coste de mantenimiento ahora (sin recintos).
    pub fn coste_mantenimiento(&self, r: &Reglas, distancia_capital: Mil) -> Costo {
        mantenimiento::coste(&r.mantenimiento, &self.edificios, self.nivel_actual(), distancia_capital, &vec![])
    }

    pub(crate) fn entorno_poblacion(&self, r: &Reglas, x: &Exterior) -> Entorno {
        let (cap_p, cap_a, cap_n) = self.capacidades(r);
        let pol = &r.politicas;
        Entorno {
            capacidad_pesants: cap_p,
            capacidad_artesanos: cap_a,
            capacidad_nobles: cap_n,
            techo: r.nivel.techo(self.nivel),
            hay_transformacion: [Tipo::Fundicion, Tipo::Curtiduria, Tipo::Armeria, Tipo::Carpinteria].iter().any(|&t| self.edificios.activos(t) > 0),
            hay_palacio: self.edificios.activos(Tipo::Palacio) > 0,
            ciudadanos: x.ciudadanos,
            factor_consumo: self.politicas.factor(pol, "factorConsumoComida"),
            felicidad: por(self.politicas.factor(pol, "factorCrecimientoPoblacion"), x.felicidad_extra),
            factor_nobleza: por(self.politicas.factor(pol, "factorCrecimientoNobleza"), x.nobleza_extra),
            factor_recaudacion: por(self.politicas.factor(pol, "factorRecaudacion"), x.recaudacion_extra),
        }
    }

    /// Reserva mínima de construcción (§6.3): lo que el mantenimiento y la comida van a cobrar en los próximos minutos. Con
    /// `con_manual`, más la reserva manual del Tesorero (solo el camino automático).
    pub fn reserva(&self, r: &Reglas, x: &Exterior, con_manual: bool) -> [Mil; 29] {
        let entorno = self.entorno_poblacion(r, x);
        let comida = self.poblacion.consumo_por_minuto(&r.poblacion, &entorno) + x.racion_guarnicion;
        let c = &r.construccion;
        let mut res = produccion::reserva_dinamica(&self.coste_mantenimiento(r, x.distancia_capital), comida, c.horizonte_mantenimiento_min, c.horizonte_comida_min);
        if con_manual {
            res.iter_mut().zip(self.reserva_manual).for_each(|(a, m)| *a += m);
        }
        res
    }

    /// Un minuto de mundo. `t` es el instante del fin del minuto. Devuelve lo que pasó, en orden.
    pub fn avanzar_minuto(&mut self, t: Instante, x: &mut Exterior) -> Vec<Evento> {
        let mut ev = vec![];
        if self.destruido {
            return ev;
        }
        let r = x.reglas;

        // 1. Construcción. Las cuadrillas se miden con el estado de principio de minuto: una obra que acaba ahora no libera hueco
        //    hasta el siguiente.
        let ocupadas_al_empezar = self.edificios.cuadrillas_ocupadas();
        self.completar_obras(t, r, &mut ev);
        self.arrancar_obras(t, r, ocupadas_al_empezar, &mut ev);

        let (pesants, artesanos, _) = self.poblacion.enteros();
        let mano = produccion::mano_de_obra(&r.catalogo, &self.edificios, pesants, artesanos);
        let reserva_auto = self.reserva(r, x, true);
        let pausadas = self.recetas_pausadas.clone();
        let ctx = produccion::Contexto { fertilidad: x.fertilidad, factor_trigo: self.politicas.factor(&r.politicas, "factorProduccionTrigo"), factor_talleres: self.politicas.factor(&r.politicas, "factorProduccionTalleres"), factor_tecnologia: x.factor_tecnologia, tiene_tecnologia: x.tiene_tecnologia, recetas_pausadas: &pausadas };
        produccion::extraer_y_cultivar(&r.catalogo, &mut self.edificios, &mut self.almacen, &mano, &ctx, x.fuentes);
        produccion::recetas(&r.catalogo, &mut self.edificios, &mut self.almacen, &mano, &ctx, &reserva_auto);
        self.avanzar_mejoras(t, r, x, &reserva_auto, &mut ev);
        if !self.auto_pausada {
            self.evaluar_necesidades(t, x, &mano, &reserva_auto, &mut ev);
        }

        // 2. Políticas que caducan (después de la construcción: el último minuto de una política todavía la aplica).
        for p in self.politicas.caducar(t) {
            ev.push(Evento::PoliticaExpirada { id: p.id });
        }
        // 3. Ascenso.
        self.terminar_ascenso(t, &mut ev);

        // 4. Comida y hambre, crecimiento y recaudación.
        let entorno = self.entorno_poblacion(r, x);
        let mut sucesos = vec![];
        let comido = self.poblacion.nutrir(&r.poblacion, &entorno, self.almacen.cantidad(Recurso::Trigo), &mut sucesos);
        self.almacen.quitar(Recurso::Trigo, comido);
        self.poblacion.crecer(&r.poblacion, &entorno, &mut sucesos);
        self.almacen.agregar(Recurso::Oro, self.poblacion.recaudacion_por_minuto(&r.poblacion, &entorno));
        for s in sucesos {
            ev.push(match s {
                poblacion::Suceso::PrimerosArtesanos => Evento::PrimerosArtesanos,
                poblacion::Suceso::PrimerosNobles => Evento::PrimerosNobles,
                poblacion::Suceso::Hambruna { pesants, artesanos, nobleza } => Evento::Hambruna { pesants, artesanos, nobleza },
            });
        }

        // 5. Mantenimiento.
        let coste = self.coste_mantenimiento(r, x.distancia_capital);
        let vida = (t - self.fundado_en) / MS_POR_MINUTO;
        let antes = self.medidor.valor;
        match mantenimiento::avanzar_minuto(&r.mantenimiento, &mut self.medidor, self.nivel, &coste, &mut self.almacen, vida, self.ocupado) {
            mantenimiento::Resultado::Deficit if self.medidor.valor < antes => ev.push(Evento::MantenimientoDeficit),
            mantenimiento::Resultado::Colapsado => ev.push(Evento::MantenimientoColapsado { nivel: self.medidor.nivel_actual }),
            mantenimiento::Resultado::Recuperado => ev.push(Evento::MantenimientoRecuperado { nivel: self.medidor.nivel_actual }),
            mantenimiento::Resultado::Ruinas { faltantes } => {
                self.destruido = true;
                ev.push(Evento::Ruinas { faltantes });
            }
            _ => {}
        }
        self.ultimo_minuto = t;
        ev
    }

    /// Cuántos edificios se completaron desde la última vez que se preguntó (el núcleo agranda la zona uno por cada uno).
    pub fn tomar_completados(&mut self) -> u32 {
        std::mem::take(&mut self.completados_pendientes)
    }

    /// Para pruebas y proyecciones: ¿cubre la producción de trigo el consumo?
    pub fn trigo_neto_por_minuto(&self, r: &Reglas, x: &Exterior) -> Mil {
        let (p, a, _) = self.poblacion.enteros();
        let mano = produccion::mano_de_obra(&r.catalogo, &self.edificios, p, a);
        let factor = self.politicas.factor(&r.politicas, "factorProduccionTrigo");
        produccion::produccion_trigo_por_minuto(&r.catalogo, &self.edificios, &mano, x.fertilidad, factor)
            - self.poblacion.consumo_por_minuto(&r.poblacion, &self.entorno_poblacion(r, x))
            - x.racion_guarnicion
    }
}

/// Sitio inagotable para pruebas y baterías: siempre hay sitio y ninguna fuente.
pub struct SitiosLibres;

impl Sitios for SitiosLibres {
    fn proponer(&mut self, tipo: Tipo) -> Option<Option<Fuente>> {
        (!tipo.es_extractor() && tipo != Tipo::Lenera).then_some(None)
    }
    fn tomar(&mut self, _: Tipo, _: Option<Fuente>) {}
}

