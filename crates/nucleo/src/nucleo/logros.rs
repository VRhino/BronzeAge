//! La tecnología del mundo dentro del núcleo (sistema 6, tanda 6B; Tecnologia_Aedas §1-§3): los contadores de los logros, los logros y las Eras. Las reglas puras son del crate `tecnologia`;
//! aquí están el estado (`Estado::tecnologia`), de dónde sale cada suma y la cita de la Era.
//!
//! # Contar en origen (decisión técnica 3 del plan, D18)
//! Nada se cuenta a partir de los hechos: cada sitio que produce el hecho suma su [`Delta`] al ocurrir, así que un hecho que deja dos rastros (el asedio con números tras su convocatoria emite
//! `combate.asedio_*` y `batalla.aplicada`) cuenta una vez. Las vías:
//!
//! | Contador | Dónde se suma |
//! |---|---|
//! | `extraido.*`, `fabricado.*` | el paso de minuto de la plaza (`con_exterior`): lo que sale de los nodos de la plaza y de sus pueblos (`FuentesMundo::extraer`, D17) y lo que fabrican sus recetas (`Fuentes::fabricado`), en milésimas con un resto oculto |
//! | `batallas.libradas`, `.conHoplitas`, `.campoAbierto` | columna contra columna con números (el `combate.resuelto` y su `combate.encuentro`); toda batalla de Unity aplicada (`aplicar_resultado`, D10) |
//! | `asedios.*`, `conquistas.conMurallaCompleta` | el asedio con números (`resolver_asedio_con_numeros`); el asedio de Unity aplicado. Nunca `batalla.aplicada` |
//! | `caravanas.destruidasOCapturadas` | la caravana interceptada con números (también la de Fundación), el acoso que ganan los bandidos y la batalla de caravana de Unity que gana el atacante |
//! | `bandidos.campamentosDestruidos` | el campamento destruido con números y la batalla de Unity contra un campamento que gana el atacante |
//! | `animales.comprados` | `ComprarAnimalCaravana` |
//! | `reclutados.*` | reclutar (y reponer) en una plaza y en un campamento de mercenarios (Q4); no la guarnición de serie de las avanzadas ni la tropa prestada |
//! | `plazasEnNivel.N` | el récord, al subir el número de plazas con nivel actual ≥ N (el índice derivado `Espacio::plazas_por_nivel`) |
//!
//! # La Era, sin tick
//! Al fijar un logro se mira si la Era vigente tiene ya todos los suyos; y hay **una cita global** (`Carga::Era`, prioridad 10) al fin de su plazo. Se arma como el plazo de los títulos: al
//! pasar algo en el mundo con alguna Facción, si no hay otra (`Estado::era_en`). Una Era por evaluación: si la que empieza ya tiene todos sus logros cumplidos, su cita cae en ese mismo
//! instante y la siguiente empieza en otra evaluación. La última Era (la III, mientras no existan los bloques de la IV y la V) no tiene cita.

use super::*;
use crate::estado::{entidad_era, Dato, PRIO_ERA};
use militar::catalogo::Tropa;
use tecnologia::contador::{Contador, Delta};
use tecnologia::{Conjunto, TecId};

/// Niveles que lleva el índice de plazas por nivel actual (los de más se cuentan en el último).
pub(super) const NIVELES_INDICE: usize = 8;

/// ¿Va algún hoplita ciudadano entre estas tropas? (`batallas.conHoplitas`.)
pub(super) fn con_hoplitas(mut tropas: impl Iterator<Item = Tropa>) -> bool {
    tropas.any(|t| t == Tropa::HoplitasCiudadanos)
}

fn indice_de_nivel(nivel: u32) -> usize {
    (nivel as usize).min(NIVELES_INDICE - 1)
}

impl Nucleo {
    // ───────────────────────────── Consultas ─────────────────────────────

    /// El estado de tecnología del mundo (Era, contadores, logros, descubridoras, tecnologías de cada Facción).
    pub fn tecnologia(&self) -> &tecnologia::EstadoTecnologia {
        &self.estado.tecnologia
    }

    /// ¿Tiene la Facción adoptada esa tecnología? Las de arranque las tiene toda Facción.
    pub fn tiene_tecnologia(&self, faccion: u32, tecnologia: &str) -> bool {
        self.estado.tiene_tecnologia(faccion, tecnologia)
    }

    /// El `orden` de la Era vigente (lo que mira la Maravilla en el `Exterior`).
    pub fn orden_era(&self) -> u32 {
        self.estado.reglas.economia.tecnologia.eras[self.estado.tecnologia.era as usize].orden
    }

    /// Las plazas del mundo con nivel actual ≥ `nivel` (el índice derivado de los récords `plazasEnNivel.N`).
    pub fn plazas_en_nivel(&self, nivel: u32) -> u32 {
        self.espacio.plazas_por_nivel[indice_de_nivel(nivel)..].iter().sum()
    }

    // ───────────────────────────── Sumar ─────────────────────────────

    /// Suma lo que deja un hecho a los contadores y anuncia los logros que se cumplen.
    pub(super) fn contar(&mut self, t: Instante, d: Delta) {
        if d.es_vacio() {
            return;
        }
        let r = self.estado.reglas.economia.tecnologia.clone();
        let fijados = self.estado.tecnologia.aplicar(&r, &d, t);
        self.tras_logros(t, &fijados);
    }

    /// Suma los flujos de un paso de la economía (milésimas por contador: lo extraído y lo fabricado).
    pub(super) fn contar_flujos(&mut self, t: Instante, contado: &[Mil; 24]) {
        let Estado { reglas, tecnologia, .. } = &mut self.estado;
        let mut fijados = Vec::new();
        for (i, &n) in contado.iter().enumerate().filter(|(_, &n)| n > 0) {
            fijados.extend(tecnologia.sumar_milesimas(&reglas.economia.tecnologia, Contador::TODOS[i], n as u64, t));
        }
        self.tras_logros(t, &fijados);
    }

    /// Una plaza pasa del nivel actual `antes` a `ahora` (`None`: no existía o ya no existe). Mantiene el índice y, si sube, los récords `plazasEnNivel.N`.
    pub(super) fn contar_nivel(&mut self, t: Instante, antes: Option<u32>, ahora: Option<u32>) {
        if antes == ahora {
            return;
        }
        let indice = &mut self.espacio.plazas_por_nivel;
        if let Some(n) = antes {
            indice[indice_de_nivel(n)] -= 1;
        }
        if let Some(n) = ahora {
            indice[indice_de_nivel(n)] += 1;
        }
        if ahora.unwrap_or(0) <= antes.unwrap_or(0) {
            return;
        }
        let r = self.estado.reglas.economia.tecnologia.clone();
        let mut fijados = Vec::new();
        for nivel in 2..=3 {
            let Some(c) = Contador::de_plazas_en_nivel(nivel as u8) else { continue };
            fijados.extend(self.estado.tecnologia.subir_record(&r, c, u64::from(self.plazas_en_nivel(nivel)), t));
        }
        self.tras_logros(t, &fijados);
    }

    /// Anuncia los logros recién fijados (`tecnologia.logro { contador, umbral }`, público: dice qué pasó, nunca qué tecnología abre) y mira si con ellos empieza la Era siguiente.
    fn tras_logros(&mut self, t: Instante, fijados: &[TecId]) {
        if fijados.is_empty() {
            return;
        }
        let r = self.estado.reglas.economia.tecnologia.clone();
        for &x in fijados {
            let l = r.def(x).logro.expect("solo se fija lo que tiene logro");
            let valores = vec![("contador", Dato::Texto(l.contador.nombre().to_owned()))];
            self.tr.emitir(&self.estado, EventoNucleo { valores, publico: true, ..EventoNucleo::nuevo("tecnologia.logro", t, vec![("umbral", l.umbral as i64)]) });
        }
        self.mirar_era(t);
    }

    // ───────────────────────────── La Era ─────────────────────────────

    /// TA §3 paso 2: si la Era vigente tiene todos sus logros o se agotó su plazo, empieza la siguiente (`era.comienza { era, porPlazo }`, público) y se arma su cita.
    fn mirar_era(&mut self, t: Instante) {
        let r = self.estado.reglas.economia.tecnologia.clone();
        let Some(cambio) = self.estado.tecnologia.avanzar_era(&r, t) else { return };
        let era = &r.eras[cambio.era as usize];
        let datos = vec![("orden", i64::from(era.orden)), ("porPlazo", cambio.por_plazo as i64)];
        self.tr.emitir(&self.estado, EventoNucleo { valores: vec![("era", Dato::Texto(era.id.clone()))], publico: true, ..EventoNucleo::nuevo("era.comienza", t, datos) });
        // La cita de la Era que acaba ya no vale.
        self.estado.generaciones.subir(entidad_era());
        self.estado.era_en = None;
        self.armar_era(t);
    }

    /// Arma la cita de la Era si no hay una y hay alguna Facción: al fin del plazo de la vigente o, si ya tiene todos sus logros, en este mismo instante (una Era por evaluación).
    pub(super) fn armar_era(&mut self, t: Instante) {
        if self.estado.era_en.is_some() || self.estado.politica.facciones.iter().next().is_none() {
            return;
        }
        let r = &self.estado.reglas.economia.tecnologia;
        let Some(fin) = self.estado.tecnologia.fin_del_plazo(r) else { return };
        // DUDA: el TypeScript evaluaba en cada tick; aquí, si la Era que empieza ya tiene todos sus logros, la siguiente empieza en una cita del mismo instante (otra evaluación).
        let en = if self.estado.tecnologia.todos_los_logros_de_la_era(r) { t } else { fin.max(t) };
        let clave = self.estado.generaciones.clave(entidad_era());
        self.estado.cola.programar(en, PRIO_ERA, clave, Carga::Era);
        self.estado.era_en = Some(en);
    }

    /// Vence la cita de la Era.
    pub(super) fn cita_de_era(&mut self, t: Instante) {
        self.estado.era_en = None;
        self.mirar_era(t);
        self.armar_era(t);
    }

    // ───────────────────────────── Administración ─────────────────────────────

    /// `FijarTecnologias` sobre las reglas reales (decisión técnica 8 del plan): las dadas pasan a aparecidas (con su descubridora si nadie la tenía) y las adoptadas de la Facción son las de
    /// arranque más las dadas, sin tarifa. `Ok(false)`: no cambia nada. `Err` con la que no existe.
    pub(super) fn fijar_tecnologias(&mut self, t: Instante, faccion: u32, tecnologias: &[String]) -> Result<bool, String> {
        let r = self.estado.reglas.economia.tecnologia.clone();
        let mut dadas = Conjunto::VACIO;
        for x in tecnologias {
            dadas.poner(r.buscar(x).ok_or_else(|| x.clone())?);
        }
        let antes = self.estado.tecnologia.de(&r, faccion);
        let adoptadas = r.de_arranque.union(dadas);
        if antes.adoptadas == adoptadas && dadas.es_subconjunto_de(antes.aparecidas) {
            return Ok(false);
        }
        for x in dadas.iter() {
            self.estado.tecnologia.hacer_aparecer(&r, faccion, x, t);
        }
        self.estado.tecnologia.de_mut(&r, faccion).adoptadas = adoptadas;
        Ok(true)
    }
}
