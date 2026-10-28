//! El asentamiento: el estado de una plaza y su paso de **un minuto de mundo** (Economia_Construccion §1, con la decisión del
//! 2026-10-18: cada asentamiento avanza un minuto por evento recurrente, se mire o no, en enteros y sin tick global).
//!
//! El orden dentro de un minuto es el de las reglas de Mechanics: pueblos, arranque de la cola, producción, recetas, mejoras
//! automáticas, auto-construcción; luego comida y hambre, crecimiento, recaudación y mantenimiento. **Lo que termina** (obra, mejora,
//! ascenso) **y las políticas que caducan no esperan al paso**: tienen su cita en su instante exacto ([`Asentamiento::cumplir_fines`] y
//! [`Asentamiento::caducar_politicas`], que programa quien lleva la cola). Las órdenes de los jugadores (cola, mejoras, ascenso,
//! políticas) están en `obras`, `ascenso` y aquí; la geometría (dónde cae cada edificio) y los yacimientos del mapa entran por los
//! traits [`Sitios`] y [`Fuentes`].

use crate::almacen::Almacen;
use crate::avanzadas::{Avanzada, TipoObra};
use crate::catalogo::Tipo;
use crate::datos::{por, Reglas};
use crate::edificio::{Edificio, Edificios, Estado, Fuente};
use crate::mantenimiento::{self, Medidor};
use crate::poblacion::{self, Entorno, Poblacion};
use crate::politicas::Politicas;
use crate::produccion::{self, Fuentes};
use crate::recintos::{self, Recinto};
use crate::pueblos::{IngresoAcarreo, Pueblo};
use crate::recurso::{Costo, Recurso};
use base::{Instante, Mil, Punto, MS_POR_MINUTO};
use trazado::{CeldaMuro, ParametrosMuralla};

/// Una obra de ascenso en curso (Doc 4.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ascenso {
    pub nivel_objetivo: u32,
    pub iniciado_en: Instante,
    pub completa_en: Instante,
}

/// Lo que el trazado urbano necesita saber de la plaza en cada pregunta (cambia dentro del minuto: una obra que acaba agranda la
/// ciudad). Lo calcula [`Asentamiento::urbano`]; el adaptador no ve la economía.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Urbano<'r> {
    /// Nivel alcanzado: fija el techo del radio de la ciudad.
    pub nivel: u32,
    /// Nivel operativo: el que abre el gate de las anclas.
    pub nivel_operativo: u32,
    /// Edificios construidos por la cola (activos, sin los de la fundación ni los gratuitos): agrandan la ciudad.
    pub completados: u32,
    /// Perfil de trazado de la ordenanza activa (`nucleos`, `caminera`, `compacta`, `gremial`), si la hay.
    pub perfil: Option<&'r str>,
    /// El id que recibiría el próximo edificio (el de un proyecto que se comprometa ahora).
    pub siguiente_id: u32,
}

/// Un edificio gratuito y ya activo que el trazado crea por su cuenta (ancla de saturación, puesto del Mercado): nunca pasa por la
/// cola ni cuesta nada, pero es un edificio de la economía (cuenta a la mitad en el mantenimiento).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gratuito {
    pub id: u32,
    pub tipo: Tipo,
    /// En los puestos del Mercado es su **forma**, no un nivel (Economia_Construccion §6.6).
    pub nivel_interno: u8,
}

/// Dónde puede ir un edificio nuevo: la geometría de la ciudad y los nodos o parcelas libres del mapa. Lo implementa el núcleo
/// (con el trazado urbano y el mundo); las pruebas, con un doble.
pub trait Sitios {
    /// ¿Hay sitio para este tipo? `Some(fuente)` si lo hay (la fuente libre más cercana en la zona si el tipo ligue a una), `None` si no.
    /// No toma ninguna fuente ni coloca el edificio. Si para tener sitio el barrio necesita un ancla de saturación, el trazado la crea
    /// **ya** (Trazado_Urbano §7.1, al proponer): su id sale de `ids` y vuelve en `anclas`, y se queda aunque el proyecto no se comprometa.
    fn proponer(&mut self, tipo: Tipo, u: &Urbano, ids: &mut u32, anclas: &mut Vec<Gratuito>) -> Option<Option<Fuente>>;
    /// El proyecto se comprometió con el id `id`: la fuente queda tomada y el edificio, colocado en la ciudad. Si hay que crear antes un
    /// ancla (edificio gratuito), su id sale de `ids` (el contador de ids de la plaza, ya por delante de `id`) y vuelve en el resultado.
    /// `None` = ya no hay sitio (otro proyecto de la misma pasada se lo quedó): no se toca nada y el proyecto no se compromete.
    fn tomar(&mut self, id: u32, tipo: Tipo, fuente: Option<Fuente>, u: &Urbano, ids: &mut u32) -> Option<Vec<Gratuito>>;
    /// Un Mercado acaba de completarse (`nivel` 1) o de subir a `nivel`: los puestos gratuitos de ese nivel que caben, con sus ids de
    /// `ids`. Si a un puesto no le queda sitio se salta en silencio.
    fn puestos_de_mercado(&mut self, _nivel: u8, _u: &Urbano, _ids: &mut u32) -> Vec<Gratuito> {
        vec![]
    }
    /// Al mejorar a `nivel`, ¿cabe la huella nueva del edificio? Solo cambia de huella la Granja. No toca nada.
    fn hueco_al_mejorar(&mut self, _id: u32, _tipo: Tipo, _nivel: u8, _u: &Urbano) -> bool {
        true
    }
    /// La mejora terminó: muda el edificio a un hueco de su huella nueva. `false` = no hay hueco y la mejora espera.
    fn reubicar(&mut self, _id: u32, _tipo: Tipo, _nivel: u8, _u: &Urbano) -> bool {
        true
    }
    /// Una mejora de recinto terminó: vuelve a marcar las torres del anillo (sobre sus celdas de muro, sin juntar las del nivel anterior) para `nivel`, con
    /// el interior que le da la ciudad. Sin ciudad no cambia nada.
    fn reclasificar_torres(&mut self, _celdas: &mut [CeldaMuro], _nivel: u8, _p: &ParametrosMuralla) {}
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
    /// Trigo por minuto de la ración de la guarnición (otro sistema); entra en el déficit y en la reserva de comida.
    pub racion_guarnicion: Mil,
    /// Multiplicadores de otros sistemas (ocupación, Aedas), ppm; `UNO` si no hay.
    pub felicidad_extra: i64,
    pub nobleza_extra: i64,
    pub recaudacion_extra: i64,
}

/// Lo que el reclutamiento de tropa mira de una plaza (ver [`Asentamiento::resumen_de_reclutamiento`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResumenReclutamiento {
    /// Pesants y artesanos que no hacen falta para producir y toda la nobleza, en enteros y en ese orden.
    pub disponibles: [i64; 3],
    /// `factorCostoReclutamiento` de las políticas (ppm; `UNO` si no hay «Leva Forzosa»).
    pub factor_costo: i64,
    /// Trigo por minuto que come la población.
    pub consumo_poblacion: Mil,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Evento {
    EdificioCompletado { id: u32, tipo: Tipo, nivel: u8 },
    /// Nació un edificio gratuito y activo (ancla de saturación, puesto del Mercado). `nivel` es su forma si es un puesto.
    EdificioGratuito { id: u32, tipo: Tipo, nivel: u8 },
    ObraIniciada { id: u32, tipo: Tipo, completa_en: Instante },
    MejoraIniciada { id: u32, nivel: u8, completa_en: Instante },
    MejoraCompletada { id: u32, nivel: u8 },
    /// La auto-construcción comprometió un proyecto.
    NecesidadDetectada { id: u32, tipo: Tipo, prioridad: i64 },
    /// Un jugador añadió, quitó o reordenó la cola.
    Encolada { id: u32, tipo: Tipo },
    Desencolada { id: u32 },
    ColaReordenada,
    PrimerosArtesanos,
    PrimerosNobles,
    /// Personas que murieron de hambre en el minuto, por clase.
    Hambruna { pesants: i64, artesanos: i64, nobleza: i64 },
    PoliticaExpirada { id: String },
    MantenimientoDeficit,
    MantenimientoColapsado { nivel: u32 },
    MantenimientoRecuperado { nivel: u32 },
    Ruinas { faltantes: Vec<Recurso> },
    AscensoIniciado { objetivo: u32, completa_en: Instante },
    NivelSubio { nivel: u32 },
    /// Se marcó la obra de un pueblo extractor sobre un nodo (queda pendiente de materiales).
    PuebloMarcado { id: u64, tipo: Tipo, nodo: u32 },
    /// Se entregaron materiales a la obra de un pueblo (`cantidad` en total).
    PuebloEntrega { id: u64, cantidad: Mil },
    /// Con todos los materiales entregados, arrancó la obra del pueblo.
    PuebloObraIniciada { id: u64, completa_en: Instante },
    PuebloCompletado { id: u64 },
    /// El almacén local del pueblo se llenó: la extracción se para hasta que el carro lo vacíe.
    PuebloLleno { id: u64 },
    /// El pueblo (o su obra) se destruyó a mano.
    PuebloDestruido { id: u64 },
    /// Se marcó la obra de una aldea agrícola en un punto (queda pendiente de materiales).
    AldeaMarcada { id: u64, pos: Punto },
    /// Se marcó la subida de una aldea a `nivel` (otra obra de mapa pendiente de materiales).
    AldeaMejoraMarcada { id: u64, nivel: u32 },
    /// Acabó la obra de la subida: la aldea ya es de `nivel`.
    AldeaSubio { id: u64, nivel: u32 },
    /// Se marcó la obra de una avanzada (levantar la atalaya, mejorarla o repararla): queda pendiente de materiales.
    AvanzadaObraMarcada { id: u64, tipo: TipoObra, posicion: Punto },
    /// Se entregaron materiales a la obra de una avanzada (`cantidad` en total).
    AvanzadaEntrega { id: u64, cantidad: Mil },
    /// Con todos los materiales entregados, arrancó la obra de la avanzada.
    AvanzadaObraIniciada { id: u64, tipo: TipoObra, completa_en: Instante },
    /// Acabó la obra de la avanzada: ya es operativa (primera obra o reparación) o ha subido de etapa (mejora).
    AvanzadaCompletada { id: u64, tipo: TipoObra, etapa: crate::avanzadas::Etapa },
    /// La avanzada (o su primera obra) se destruyó a mano.
    AvanzadaDestruida { id: u64 },
    /// Se canceló a mano la mejora o la reparación de la avanzada: los materiales entregados se perdieron.
    AvanzadaObraCancelada { id: u64, tipo: TipoObra },
    /// Se comprometió un recinto de muralla (gratis): el anillo queda trazado y sus celdas ocupan suelo; la obra empieza con la primera celda.
    RecintoComprometido { recinto: u32, nivel: u8, celdas: usize, puertas: usize },
    /// Empezó la mejora de un recinto completo al `nivel` (destino).
    RecintoMejoraIniciada { recinto: u32, nivel: u8 },
    /// Se abandonó un recinto que nunca se había cerrado.
    RecintoAbandonado { recinto: u32 },
    /// Se levantó una celda del recinto (de su anillo, de su mejora o de su reparación): `levantadas` celdas hechas y la cita de la siguiente, si queda alguna.
    RecintoCelda { recinto: u32, levantadas: usize, siguiente_en: Option<Instante> },
    /// El anillo se cerró por primera vez.
    RecintoCompletado { recinto: u32, nivel: u8, celdas: usize, puertas: usize },
    /// Terminó la mejora: el recinto está al `nivel` nuevo.
    RecintoMejorado { recinto: u32, nivel: u8, celdas: usize, puertas: usize },
}

/// La tendencia «+N/min» de una plaza (4H T3c, decisión 3 del autor): el neto **real** del último paso de minuto —producción, consumo, recaudación, mantenimiento y lo que come la guarnición; no el gasto
/// de las órdenes—, por recurso del almacén (milésimas) y por clase de la población (personas enteras: pesants, artesanos, nobleza). Vale 0 hasta el primer paso. Lo escribe quien lleva el reloj de la plaza
/// (el núcleo, que cierra el paso entero); el crate no lo toca, así que el batch no paga nada por ella.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tendencia {
    pub almacen: [Mil; 29],
    pub poblacion: [i64; 3],
    /// Lo que entró y lo que salió de cada recurso por los flujos del paso, por separado (mecánica 60, 2026-10-28): la Granja y la comida del trigo, por ejemplo, no se
    /// compensan. `almacen` sigue siendo el neto.
    pub producido: [Mil; 29],
    pub consumido: [Mil; 29],
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
    /// Los pueblos extractores de la plaza, en cualquier estado, por id ascendente. No son edificios de la ciudad (Doc 4.2.2).
    pub pueblos: Vec<Pueblo>,
    /// Las avanzadas de la plaza, en cualquier estado, por id ascendente (Doc 5.17). Tampoco son edificios de la ciudad.
    pub avanzadas: Vec<Avanzada>,
    /// Los recintos de muralla de la plaza, del más interior al más exterior (Murallas.md). Ver [`crate::recintos`].
    pub recintos: Vec<Recinto>,
    /// Lo que el acarreo ingresó de verdad en el almacén de la plaza en las últimas horas (ventana de 24 h, por hora y recurso): es lo que
    /// cuenta como acarreo en la solvencia del ascenso. Solo se escribe cuando llega un carro; un pueblo ocioso no la toca.
    pub ingresos_acarreo: Vec<IngresoAcarreo>,
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
    /// Lo que cambió el último paso de minuto (la tendencia «+N/min» del interior).
    pub tendencia: Tendencia,
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
            pueblos: vec![],
            avanzadas: vec![],
            recintos: vec![],
            ingresos_acarreo: vec![],
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
            tendencia: Tendencia::default(),
        }
    }

    pub fn nivel_actual(&self) -> u32 {
        self.medidor.nivel_actual
    }

    /// Lo que el trazado urbano pregunta a la plaza (ver [`Urbano`]). Los edificios de la fundación son los de id menor que
    /// `viviendas iniciales + 2` (Centro Urbano, Viviendas y Granja).
    ///
    /// El `radioCiudad` crece con los edificios que pasaron por la cola, no con los de la fundación ni con los gratuitos, y topa por el
    /// nivel alcanzado; el gate de las anclas usa el operativo (Trazado_Urbano §2.1).
    pub fn urbano<'r>(&self, r: &'r Reglas) -> Urbano<'r> {
        let fundacion = r.viviendas_iniciales + 2;
        let gratuitas = &r.mantenimiento.piezas_gratuitas;
        let completados = self.edificios.iter().filter(|e| e.activo() && e.id >= fundacion && !gratuitas.contains(&e.tipo)).count() as u32;
        Urbano { nivel: self.nivel, nivel_operativo: self.nivel_actual(), completados, perfil: self.politicas.perfil_trazado(&r.politicas), siguiente_id: self.siguiente_edificio }
    }

    /// Pregunta a `x.sitios` si hay sitio para `tipo` ([`Sitios::proponer`]): las anclas que el trazado cree para ello se dan de alta ya en la
    /// economía, como edificios gratuitos, aunque el proyecto no llegue a comprometerse (así toda pieza de la ciudad tiene su edificio). El id del
    /// ancla sale del contador de la plaza, no de `u.siguiente_id`.
    pub(crate) fn proponer(&mut self, x: &mut Exterior, tipo: Tipo, u: &Urbano, ev: &mut Vec<Evento>) -> Option<Option<Fuente>> {
        let mut anclas = vec![];
        let fuente = x.sitios.proponer(tipo, u, &mut self.siguiente_edificio, &mut anclas);
        self.alta_gratuitos(anclas, ev);
        fuente
    }

    /// Da de alta edificios gratuitos ya activos (con su suceso). No cuentan como «completados»: no agrandan la zona.
    pub(crate) fn alta_gratuitos(&mut self, gratuitos: Vec<Gratuito>, ev: &mut Vec<Evento>) {
        for g in gratuitos {
            let mut e = Edificio::nuevo(g.id, g.tipo, Estado::Activo);
            e.nivel_interno = g.nivel_interno;
            self.edificios.push(e);
            ev.push(Evento::EdificioGratuito { id: g.id, tipo: g.tipo, nivel: g.nivel_interno });
        }
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

    /// Coste de mantenimiento ahora, con el de los recintos (las celdas en pie, sin escala).
    pub fn coste_mantenimiento(&self, r: &Reglas, distancia_capital: Mil) -> Costo {
        mantenimiento::coste(&r.mantenimiento, &self.edificios, self.elementos_de_mapa_activos(), self.nivel_actual(), distancia_capital, &recintos::upkeep(&r.muralla, &self.recintos))
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

    /// Lo que reclutar mira de la plaza (Heroe_Y_Tropa §11): el pool de cada clase, el factor de «Leva Forzosa» y lo que come la población.
    pub fn resumen_de_reclutamiento(&self, r: &Reglas, x: &Exterior) -> ResumenReclutamiento {
        let (pesants, artesanos, nobleza) = self.poblacion.enteros();
        let mano = produccion::mano_de_obra(&r.catalogo, &self.edificios, pesants, artesanos, self.trabajadores_de_pueblos(r));
        let (pesants_libres, artesanos_libres) = produccion::disponibles_para_reclutar(&mano, pesants, artesanos);
        ResumenReclutamiento {
            disponibles: [pesants_libres, artesanos_libres, nobleza],
            factor_costo: self.politicas.factor(&r.politicas, "factorCostoReclutamiento"),
            consumo_poblacion: self.poblacion.consumo_por_minuto(&r.poblacion, &self.entorno_poblacion(r, x)),
        }
    }

    /// Un minuto de mundo. `t` es el instante del fin del minuto. Devuelve lo que pasó, en orden.
    pub fn avanzar_minuto(&mut self, t: Instante, x: &mut Exterior) -> Vec<Evento> {
        let mut ev = vec![];
        if self.destruido {
            return ev;
        }
        let r = x.reglas;

        // 1. Construcción. Lo que acaba ya acabó en su instante (`cumplir_fines`); aquí arrancan las obras que esperaban (las que la
        //    auto-construcción comprometió en el minuto anterior, las reconstrucciones) y se reintentan las mejoras que esperaban hueco.
        self.completar_pueblos(t, r, &mut ev);
        self.terminar_mejoras(t, r, x, &mut ev);
        self.arrancar_obras(t, r, &mut ev);

        let (pesants, artesanos, _) = self.poblacion.enteros();
        let mano = produccion::mano_de_obra(&r.catalogo, &self.edificios, pesants, artesanos, self.trabajadores_de_pueblos(r));
        let reserva_auto = self.reserva(r, x, true);
        let pausadas = self.recetas_pausadas.clone();
        let ctx = produccion::Contexto { fertilidad: x.fertilidad, factor_trigo: self.politicas.factor(&r.politicas, "factorProduccionTrigo"), factor_talleres: self.politicas.factor(&r.politicas, "factorProduccionTalleres"), factor_tecnologia: x.factor_tecnologia, tiene_tecnologia: x.tiene_tecnologia, recetas_pausadas: &pausadas };
        produccion::extraer_y_cultivar(&r.catalogo, &mut self.edificios, &mut self.almacen, &mano, &ctx, x.fuentes);
        self.extraer_pueblos(r, &mano, &ctx, x.fuentes, &mut ev);
        for (recurso, n) in produccion::recetas(&r.catalogo, &mut self.edificios, &mut self.almacen, &mano, &ctx, &reserva_auto) {
            x.fuentes.fabricado(recurso, n);
        }
        self.iniciar_mejoras_automaticas(t, r, x, &reserva_auto, &mut ev);
        if !self.auto_pausada {
            self.evaluar_necesidades(t, x, &mano, &reserva_auto, &mut ev);
        }
        // La obra de los recintos, con la misma reserva: reintenta la celda que esperaba materiales (las que tienen cita las levanta `cumplir_fines`).
        if self.recintos.iter().any(|r| r.vence(t)) {
            self.avanzar_recintos(t, x, &reserva_auto, &mut ev);
        }

        // 2. Comida y hambre, crecimiento y recaudación.
        let entorno = self.entorno_poblacion(r, x);
        let mut sucesos = vec![];
        let comido = self.poblacion.nutrir(&r.poblacion, &entorno, self.almacen.disponible(Recurso::Trigo), &mut sucesos);
        self.almacen.consumir(Recurso::Trigo, comido);
        self.poblacion.crecer(&r.poblacion, &entorno, &mut sucesos);
        self.almacen.producir(Recurso::Oro, self.poblacion.recaudacion_por_minuto(&r.poblacion, &entorno));
        for s in sucesos {
            ev.push(match s {
                poblacion::Suceso::PrimerosArtesanos => Evento::PrimerosArtesanos,
                poblacion::Suceso::PrimerosNobles => Evento::PrimerosNobles,
                poblacion::Suceso::Hambruna { pesants, artesanos, nobleza } => Evento::Hambruna { pesants, artesanos, nobleza },
            });
        }

        // 3. Mantenimiento.
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

    /// Lo que vence en el instante `t`: las obras que acaban (el edificio pasa a activo), las mejoras, el ascenso. Si acabó una obra o una mejora
    /// se libera su cuadrilla y la cola arranca **en ese mismo instante** lo que esperaba. Es idempotente: una cita que llega con la plaza ya
    /// cambiada (una conquista descartó el ascenso, por ejemplo) no encuentra nada que hacer. Quien lleva la cola programa una cita en el
    /// `completa_en` de cada `ObraIniciada`, `MejoraIniciada` y `AscensoIniciado`.
    pub fn cumplir_fines(&mut self, t: Instante, x: &mut Exterior) -> Vec<Evento> {
        let mut ev = vec![];
        if self.destruido {
            return ev;
        }
        let r = x.reglas;
        self.completar_obras(t, r, x, &mut ev);
        self.terminar_mejoras(t, r, x, &mut ev);
        self.terminar_ascenso(t, &mut ev);
        self.cumplir_avanzadas(t, r, &mut ev);
        self.levantar_recintos(t, x, &mut ev);
        if ev.iter().any(|e| matches!(e, Evento::EdificioCompletado { .. } | Evento::MejoraCompletada { .. })) {
            self.arrancar_obras(t, r, &mut ev);
        }
        ev
    }

    /// Las políticas que vencen en `t` caducan (cita propia, después del paso de minuto de la plaza: el último minuto de una política todavía
    /// la aplica).
    pub fn caducar_politicas(&mut self, t: Instante) -> Vec<Evento> {
        self.politicas.caducar(t).into_iter().map(|p| Evento::PoliticaExpirada { id: p.id }).collect()
    }

    /// Para quien no lleva una cola de eventos (pruebas y batch): cumple en su instante exacto los fines que caen en `(ultimo_minuto, t]`, da
    /// el paso de minuto de `t` y caduca las políticas que vencen en `t`. Es el orden de las citas del núcleo con un paso por minuto.
    pub fn avanzar_hasta(&mut self, t: Instante, x: &mut Exterior) -> Vec<Evento> {
        let desde = self.ultimo_minuto;
        let obras = self.edificios.iter().filter(|e| e.estado == Estado::EnConstruccion).filter_map(|e| e.completa_en);
        let mut instantes: Vec<Instante> = obras.chain(self.edificios.iter().filter_map(|e| e.mejora.map(|m| m.completa_en))).chain(self.ascenso.map(|a| a.completa_en)).chain(self.fines_de_avanzadas()).chain(self.fines_de_recintos()).filter(|&f| f > desde && f <= t).collect();
        instantes.sort_unstable();
        instantes.dedup();
        let mut ev: Vec<Evento> = instantes.into_iter().flat_map(|f| self.cumplir_fines(f, x)).collect();
        ev.extend(self.avanzar_minuto(t, x));
        ev.extend(self.caducar_politicas(t));
        ev
    }

    /// Cuántos edificios se completaron desde la última vez que se preguntó (el núcleo agranda la zona uno por cada uno).
    pub fn tomar_completados(&mut self) -> u32 {
        std::mem::take(&mut self.completados_pendientes)
    }

    /// Para pruebas y proyecciones: ¿cubre la producción de trigo el consumo?
    pub fn trigo_neto_por_minuto(&self, r: &Reglas, x: &Exterior) -> Mil {
        let (p, a, _) = self.poblacion.enteros();
        let mano = produccion::mano_de_obra(&r.catalogo, &self.edificios, p, a, self.trabajadores_de_pueblos(r));
        let factor = self.politicas.factor(&r.politicas, "factorProduccionTrigo");
        produccion::produccion_trigo_por_minuto(&r.catalogo, &self.edificios, &mano, x.fertilidad, factor)
            - self.poblacion.consumo_por_minuto(&r.poblacion, &self.entorno_poblacion(r, x))
            - x.racion_guarnicion
    }
}

/// Sitio inagotable para pruebas y baterías: siempre hay sitio y ninguna fuente.
pub struct SitiosLibres;

impl Sitios for SitiosLibres {
    fn proponer(&mut self, tipo: Tipo, _: &Urbano, _: &mut u32, _: &mut Vec<Gratuito>) -> Option<Option<Fuente>> {
        (!tipo.es_extractor() && tipo != Tipo::Lenera).then_some(None)
    }
    fn tomar(&mut self, _: u32, _: Tipo, _: Option<Fuente>, _: &Urbano, _: &mut u32) -> Option<Vec<Gratuito>> {
        Some(vec![])
    }
}
