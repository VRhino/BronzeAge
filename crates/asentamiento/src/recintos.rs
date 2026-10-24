//! Los recintos de muralla de una plaza (`Docs/Mechanics/Murallas.md`): su modelo, lo que cuestan, su obra celda a celda, cómo se mejoran, se
//! amplían y se abandonan, y lo que valen (multiplicador defensivo, cupo de guarnición, requisito del nivel 4, mantenimiento y daño de una
//! conquista). Reglas puras en enteros: el **trazo** (dónde cae el anillo) es geometría y vive en `trazado::muralla`; el núcleo lo calcula con la
//! ciudad de la plaza y se lo pasa a [`Asentamiento::comprometer_recinto`]. Los recintos se guardan en la plaza ([`Asentamiento::recintos`]).
//!
//! # Q1: defiende lo que está en pie
//! `nivel` y `avance` son siempre los del **anillo en pie**; una mejora lleva su progreso aparte ([`Mejora`]), así que mejorar o ampliar no quita
//! multiplicador, cupo de guarnición ni requisito del nivel 4 al anillo que ya estaba, y el mantenimiento cuenta sus celdas en pie.
//!
//! # El reloj
//! Cada celda es una cita en [`Recinto::siguiente_celda_en`] (la programa el núcleo a partir de [`Evento::RecintoCelda`]); si al llegar faltan
//! materiales, o se bajaría de la reserva, no se programa otra y el paso de minuto de la plaza reintenta la obra.

use crate::asentamiento::{Asentamiento, Evento, Exterior};
use crate::datos::{mil, por, ppm, UNO};
use crate::recurso::{Costo, Recurso};
use base::{Instante, Mil, MS_POR_MINUTO};
use serde_json::Value;
use trazado::{CeldaDeRed, CeldaMuro, ClaseCelda, ParametrosMuralla};

/// Niveles de recinto: empalizada, muro de piedra, muralla con adarve.
const NIVELES: usize = 3;

// ───────────────────────────── Las cifras (bloque `MURALLA`) ─────────────────────────────

/// El bloque `MURALLA` de `Murallas.md` §11, en enteros: `ppm` para el bono, `Mil` para los costes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglasMuralla {
    /// Nivel operativo de la plaza que exige cada nivel de recinto, para comprometerlo y para mejorarlo a él (índice `nivel − 1`).
    pub nivel_de_plaza: [u32; NIVELES],
    pub nivel_maximo: u8,
    pub franja_de_ronda: i32,
    /// Celdas entre torres (índice `nivel − 1`); 0 = ese nivel no tiene torres.
    pub paso_torres: [i32; NIVELES],
    /// Tope de puertas de un anillo (Cal-6): si la red lo cruza por más sitios, se cortan los que menos red de fuera sirven.
    pub puertas_maximas: usize,
    /// Lo normal: entre cuántas puertas suele quedar un anillo (mínimo y máximo); es la referencia del batch, no una regla del trazo.
    pub puertas_normales: [usize; 2],
    /// Edificios extramuros que exige ampliar.
    pub arrabal_minimo: usize,
    pub minutos_por_celda: [i64; NIVELES],
    pub factor_puerta: i64,
    pub factor_torre: i64,
    /// Coste de una celda llana construida directamente en ese nivel: la tarifa **acumulada** (Q4; mejorar cuesta la diferencia con el nivel anterior).
    pub tarifa_por_celda: [Costo; NIVELES],
    /// Mantenimiento por celda en pie y minuto, en milésimas.
    pub upkeep_por_celda: [Costo; NIVELES],
    /// Multiplicador del defensor con el anillo entero y una sola puerta (ppm).
    pub bono_defensa: [i64; NIVELES],
}

fn por_nivel<T: Default>(v: &Value, f: impl Fn(&Value) -> T) -> [T; NIVELES] {
    std::array::from_fn(|i| v.get((i + 1).to_string()).map_or_else(T::default, &f))
}

fn costo_de(v: &Value) -> Costo {
    let mut c: Costo = v.as_object().expect("un coste es un objeto").iter().map(|(k, n)| (Recurso::desde_nombre(k).unwrap_or_else(|| panic!("recurso «{k}» de MURALLA")), mil(n.as_f64().expect("cantidad")))).collect();
    c.sort_by_key(|&(r, _)| r.indice());
    c
}

fn entero(v: &Value, k: &str) -> i64 {
    v.get(k).and_then(Value::as_f64).unwrap_or_else(|| panic!("falta «{k}» en MURALLA")) as i64
}

fn sumar(total: &mut Costo, r: Recurso, n: Mil) {
    match total.iter_mut().find(|(x, _)| *x == r) {
        Some(f) => f.1 += n,
        None => total.push((r, n)),
    }
}

impl ReglasMuralla {
    pub fn de(b: &Value) -> Self {
        let numero = |v: &Value| v.as_f64().expect("número") as i64;
        Self {
            nivel_de_plaza: por_nivel(&b["nivelAsentamientoPorNivelDeRecinto"], |v| numero(v) as u32),
            nivel_maximo: entero(b, "nivelMaximo") as u8,
            franja_de_ronda: entero(b, "franjaDeRonda") as i32,
            paso_torres: por_nivel(&b["pasoTorres"], |v| numero(v) as i32),
            puertas_maximas: entero(b, "puertasMaximas") as usize,
            puertas_normales: [0, 1].map(|i| b["puertasNormales"][i].as_u64().expect("puertasNormales: [mínimo, máximo]") as usize),
            arrabal_minimo: entero(b, "arrabalMinimo") as usize,
            minutos_por_celda: por_nivel(&b["minutosPorCelda"], numero),
            factor_puerta: entero(b, "factorPuerta"),
            factor_torre: entero(b, "factorTorre"),
            tarifa_por_celda: por_nivel(&b["tarifaPorCelda"], costo_de),
            upkeep_por_celda: por_nivel(&b["upkeepPorCelda"], costo_de),
            bono_defensa: por_nivel(&b["bonoDefensaPorNivel"], |v| ppm(v.as_f64().expect("bono"))),
        }
    }

    /// Lo que el trazo lee del bloque (franja de ronda y separación de las torres).
    pub fn parametros_de_trazo(&self) -> ParametrosMuralla {
        ParametrosMuralla { franja_de_ronda: self.franja_de_ronda, paso_torres: self.paso_torres, puertas_maximas: self.puertas_maximas }
    }

    /// ¿Es un nivel de recinto que existe?
    pub fn existe(&self, nivel: u8) -> bool {
        (1..=self.nivel_maximo.min(NIVELES as u8)).contains(&nivel)
    }

    fn factor(&self, clase: ClaseCelda) -> i64 {
        match clase {
            ClaseCelda::Muro => 1,
            ClaseCelda::Puerta => self.factor_puerta,
            ClaseCelda::Torre => self.factor_torre,
        }
    }

    fn escalar(tarifa: &Costo, veces: i64) -> Costo {
        tarifa.iter().filter(|&&(_, n)| n * veces != 0).map(|&(r, n)| (r, n * veces)).collect()
    }

    /// Lo que cuesta levantar una celda de esa clase en un recinto de `nivel` (la tarifa acumulada × el factor de la clase).
    pub fn coste_de_celda(&self, clase: ClaseCelda, nivel: u8) -> Costo {
        Self::escalar(&self.tarifa_por_celda[nivel as usize - 1], self.factor(clase))
    }

    /// Lo que cuesta mejorar una celda de esa clase al nivel `destino`: la diferencia de tarifas acumuladas con el nivel anterior (nunca negativa).
    pub fn coste_de_mejora_de_celda(&self, clase: ClaseCelda, destino: u8) -> Costo {
        let (nueva, vieja) = (&self.tarifa_por_celda[destino as usize - 1], &self.tarifa_por_celda[destino as usize - 2]);
        let diferencia = |r: Recurso| {
            let de = |t: &Costo| t.iter().find(|&&(x, _)| x == r).map_or(0, |&(_, n)| n);
            (de(nueva) - de(vieja)).max(0)
        };
        let mut recursos: Vec<Recurso> = nueva.iter().chain(vieja).map(|&(r, _)| r).collect();
        recursos.sort_by_key(|r| r.indice());
        recursos.dedup();
        let tarifa: Costo = recursos.into_iter().map(|r| (r, diferencia(r))).collect();
        Self::escalar(&tarifa, self.factor(clase))
    }

    /// El coste de levantar un trazo entero en `nivel`: la suma de sus celdas (el presupuesto que se enseña antes de comprometer).
    pub fn coste_de_trazo(&self, celdas: &[CeldaMuro], nivel: u8) -> Costo {
        let veces: i64 = celdas.iter().map(|c| self.factor(c.clase)).sum();
        Self::escalar(&self.tarifa_por_celda[nivel as usize - 1], veces)
    }

    /// ¿Hay arrabal para ampliar? (`extramuros` edificios urbanos fuera del recinto exterior).
    pub fn arrabal_suficiente(&self, extramuros: usize) -> Result<(), ErrorRecinto> {
        if extramuros >= self.arrabal_minimo {
            Ok(())
        } else {
            Err(ErrorRecinto::Arrabal { extramuros, minimo: self.arrabal_minimo })
        }
    }

    /// Lo que tarda en levantarse una celda de un recinto de `nivel`, con el factor de tiempo de las políticas (ppm), en ms de mundo.
    fn duracion_de_celda(&self, nivel: u8, factor: i64) -> i64 {
        por(self.minutos_por_celda[nivel as usize - 1] * MS_POR_MINUTO, factor).max(1)
    }
}

// ───────────────────────────── El recinto ─────────────────────────────

/// La mejora de un recinto en curso: su progreso va aparte del anillo en pie (Q1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mejora {
    /// El nivel al que se mejora (el del anillo en pie más uno).
    pub nivel: u8,
    /// Índice de la última celda ya mejorada; −1 = ninguna.
    pub avance: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recinto {
    /// `n` = nº de recintos que había + 1; en el contrato de texto, `recinto-<plaza>-<n>`. Solo se abandona el último, así que no se repite.
    pub id: u32,
    /// El nivel del anillo **en pie**: 1 empalizada, 2 muro de piedra, 3 muralla con adarve.
    pub nivel: u8,
    /// El anillo completo en orden de recorrido desde la puerta principal (el orden en que se levanta). Congelado al comprometer: solo cambia la
    /// clase de las celdas de muro a torre y viceversa al terminar una mejora.
    pub celdas: Vec<CeldaMuro>,
    /// La red que el recinto conserva en sus puertas y la pieza tras la que entra al rehacer la red (`trazado::Recinto`, `LAYOUT_VERSION` 5, Murallas §3). Congeladas
    /// con el trazo.
    pub red_de_puertas: Vec<CeldaDeRed>,
    pub tras_pieza: u32,
    /// Índice de la última celda levantada; −1 = ninguna. Completo ⇔ `avance = celdas.len() − 1`.
    pub avance: i32,
    pub mejora: Option<Mejora>,
    /// Antes de este instante no se levanta otra celda; `None` mientras no haya una obra en marcha.
    pub siguiente_celda_en: Option<Instante>,
    pub comprometido_en: Instante,
    /// Cuándo se cerró **por primera vez**; no se borra nunca (distingue un recinto dañado por una conquista de uno que nunca se cerró).
    pub completado_en: Option<Instante>,
}

impl Recinto {
    /// Sus puertas: las aberturas del anillo, tiradas contiguas de celdas de puerta (Cal-6, `trazado::aberturas`).
    pub fn puertas(&self) -> usize {
        trazado::aberturas(&self.celdas).len()
    }

    /// Celdas del anillo en pie levantadas.
    pub fn levantadas(&self) -> usize {
        (self.avance + 1).max(0) as usize
    }

    pub fn completo(&self) -> bool {
        !self.celdas.is_empty() && self.levantadas() == self.celdas.len()
    }

    /// Se cerró alguna vez pero una conquista le quitó celdas: se repara sola con la obra normal.
    pub fn danado(&self) -> bool {
        self.completado_en.is_some() && !self.completo()
    }

    /// `(celdas levantadas × 1,0) / celdas`, en ppm (0 si no hay celdas).
    pub fn integridad(&self) -> i64 {
        if self.celdas.is_empty() {
            0
        } else {
            self.levantadas() as i64 * UNO / self.celdas.len() as i64
        }
    }

    /// Lo que toca levantar ahora: el nivel que se paga, la clase de la celda y si es una mejora. `None` si no hay obra (completo y sin mejora).
    fn siguiente(&self) -> Option<(u8, ClaseCelda, bool)> {
        match self.mejora {
            Some(m) => Some((m.nivel, self.celdas[(m.avance + 1) as usize].clase, true)),
            None if !self.completo() => Some((self.nivel, self.celdas[(self.avance + 1) as usize].clase, false)),
            None => None,
        }
    }

    /// ¿Hay una celda por levantar que ya toque en `t`?
    pub fn vence(&self, t: Instante) -> bool {
        self.siguiente().is_some() && self.siguiente_celda_en.map_or(true, |s| t >= s)
    }
}

/// La proyección que la ciudad necesita de los recintos (`trazado::Ciudad::set_recintos`): los muros ocupan suelo desde el compromiso y solo un
/// recinto completo activa la preferencia intramuros. La ciudad de una plaza se deriva siempre de esto: no se guarda por separado.
pub fn proyeccion(recintos: &[Recinto]) -> Vec<trazado::Recinto> {
    recintos.iter().map(|r| trazado::Recinto::desde_celdas(&r.celdas, &r.red_de_puertas, r.tras_pieza, r.completo())).collect()
}

// ───────────────────────────── Lo que vale un recinto ─────────────────────────────

/// El recinto que cuenta para el multiplicador: el más exterior de los que han llegado a cerrarse alguna vez (Q1: mientras se mejora o se amplía
/// defiende el anillo anterior, y una conquista lo deja débil sin quitarle el puesto); si ninguno se ha cerrado, el más exterior que haya. Los recintos van
/// del más interior al más exterior.
pub fn de_referencia(recintos: &[Recinto]) -> Option<&Recinto> {
    recintos.iter().rev().find(|r| r.completado_en.is_some()).or(recintos.last())
}

/// `1 + (bono − 1) × integridad / nº de puertas` del recinto de referencia, en ppm; 1,0 si no hay recinto o no tiene puertas. Con el anillo
/// entero: ×2,5 con una puerta, ×1,375 con cuatro, ×1,1875 con ocho (nivel 3). Se calcula con una sola división entera hacia abajo
/// (`(bono − 1) × levantadas / (celdas × puertas)`), así que no hay dos redondeos encadenados. Nunca baja de 1,0.
pub fn multiplicador_defensivo(r: &ReglasMuralla, recintos: &[Recinto]) -> i64 {
    let Some(rec) = de_referencia(recintos) else { return UNO };
    let puertas = rec.puertas() as i128;
    if puertas == 0 || !r.existe(rec.nivel) {
        return UNO;
    }
    let extra = (r.bono_defensa[rec.nivel as usize - 1] - UNO).max(0) as i128;
    UNO + (extra * rec.levantadas() as i128 / (rec.celdas.len() as i128 * puertas)) as i64
}

/// El nivel del mejor recinto con integridad ≥ 1 (el de mayor nivel entre los completos; 0 si no hay): lo que suma el cupo de guarnición
/// (`recintoCompletoPorNivel`, sin acumular) y lo que pide el nivel 4 (`recintoCompletoNivelMinimo`). Un recinto en mejora cuenta con su nivel
/// de antes; uno dañado por una conquista no cuenta hasta repararse.
pub fn nivel_del_mejor_completo(recintos: &[Recinto]) -> u32 {
    recintos.iter().filter(|r| r.completo()).map(|r| r.nivel as u32).max().unwrap_or(0)
}

/// ¿Tiene la plaza una **muralla completa**? Al menos un recinto con integridad ≥ 1 (el predicado de las tecnologías de Carpintería, `asedios.contraMurallaCompleta` y
/// `conquistas.conMurallaCompleta`). Una mejora en curso no lo quita (el anillo en pie sigue entero); un recinto dañado, sí. Se mira **antes** del daño de la conquista.
pub fn muralla_completa(recintos: &[Recinto]) -> bool {
    recintos.iter().any(Recinto::completo)
}

/// El mantenimiento de los recintos por minuto, sin escala de población ni de distancia: `upkeepPorCelda[nivel] × celdas en pie` de cada anillo, con su
/// nivel y su avance (una mejora en curso no lo cambia).
pub fn upkeep(r: &ReglasMuralla, recintos: &[Recinto]) -> Costo {
    let mut total = Costo::new();
    for rec in recintos.iter().filter(|rec| rec.levantadas() > 0 && r.existe(rec.nivel)) {
        for &(rc, n) in &r.upkeep_por_celda[rec.nivel as usize - 1] {
            sumar(&mut total, rc, n * rec.levantadas() as i64);
        }
    }
    total
}

/// El daño de una conquista: cada recinto **completo y sin mejora en curso** pierde `floor(fraccion × celdas)` de avance (mínimo −1; `fraccion` en ppm,
/// `OCUPACION.fraccionDanoMuralla`). Un recinto en obra o con mejora no recibe daño y lo sigue el nuevo dueño. Conserva `completado_en` (Q2) y deja el recinto
/// sin cita, para que quien lo llame programe la reparación. Devuelve si algo cambió.
pub fn danar_por_conquista(recintos: &mut [Recinto], fraccion: i64) -> bool {
    let mut cambio = false;
    for rec in recintos.iter_mut().filter(|r| r.completo() && r.mejora.is_none()) {
        let perdidas = (rec.celdas.len() as i128 * fraccion as i128 / UNO as i128) as i32;
        if perdidas > 0 {
            rec.avance = (rec.avance - perdidas).max(-1);
            rec.siguiente_celda_en = None;
            cambio = true;
        }
    }
    cambio
}

// ───────────────────────────── Órdenes y obra ─────────────────────────────

/// Por qué no se puede encargar, mejorar o abandonar un recinto (la geometría —sin trazo, no envuelve— la rechaza quien traza).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorRecinto {
    /// No existe ese nivel de recinto.
    NivelInexistente,
    /// La plaza no tiene el nivel operativo que pide ese nivel de recinto.
    NivelDePlaza { necesario: u32, actual: u32 },
    /// Hay un recinto sin cerrar (en obra o dañado por una conquista): hay que terminarlo o abandonarlo antes.
    EnObra,
    /// Ampliar exige `minimo` edificios extramuros.
    Arrabal { extramuros: usize, minimo: usize },
    NoExiste,
    /// No se mejora un recinto sin cerrar: primero se termina.
    Incompleto,
    YaMejorando,
    EnElMaximo,
    /// Un recinto que alguna vez estuvo completo no se abandona (ni dañado por una conquista).
    YaCerrado,
}

impl Asentamiento {
    /// Las comprobaciones de comprometer que no necesitan geometría, en su orden: el nivel existe, la plaza tiene el nivel operativo que pide, y no hay
    /// ningún recinto sin cerrar (uno dañado por una conquista también bloquea hasta repararse). Devuelve el recinto exterior, si hay: si lo hay, es una
    /// ampliación y quien traza debe comprobar además el arrabal y que el anillo nuevo lo envuelva.
    pub fn comprobar_comprometer(&self, r: &crate::datos::Reglas, nivel: u8) -> Result<Option<&Recinto>, ErrorRecinto> {
        let m = &r.muralla;
        if !m.existe(nivel) {
            return Err(ErrorRecinto::NivelInexistente);
        }
        let necesario = m.nivel_de_plaza[nivel as usize - 1];
        if self.nivel_actual() < necesario {
            return Err(ErrorRecinto::NivelDePlaza { necesario, actual: self.nivel_actual() });
        }
        if self.recintos.iter().any(|r| !r.completo()) {
            return Err(ErrorRecinto::EnObra);
        }
        Ok(self.recintos.last())
    }

    /// Compromete un recinto: congela el trazo en la plaza, sin avance. **Comprometer es gratis**; lo que cuesta es levantarlo, celda a celda. `celdas` es
    /// el trazo en orden de recorrido, que el llamador ya comprobó, con la red que conserva en sus puertas. Devuelve el id del recinto.
    pub fn comprometer_recinto(&mut self, t: Instante, nivel: u8, celdas: Vec<CeldaMuro>, red_de_puertas: Vec<CeldaDeRed>, tras_pieza: u32, ev: &mut Vec<Evento>) -> u32 {
        let id = self.recintos.len() as u32 + 1;
        let (n, puertas) = (celdas.len(), trazado::aberturas(&celdas).len());
        self.recintos.push(Recinto { id, nivel, celdas, red_de_puertas, tras_pieza, avance: -1, mejora: None, siguiente_celda_en: None, comprometido_en: t, completado_en: None });
        ev.push(Evento::RecintoComprometido { recinto: id, nivel, celdas: n, puertas });
        id
    }

    /// Las comprobaciones de mejorar: el recinto existe, no está ya en el máximo, la plaza tiene el nivel operativo del nivel destino, está completo (uno
    /// dañado primero se repara) y no se está mejorando ya.
    pub fn comprobar_mejora(&self, r: &crate::datos::Reglas, recinto: u32) -> Result<(), ErrorRecinto> {
        let rec = self.recintos.iter().find(|x| x.id == recinto).ok_or(ErrorRecinto::NoExiste)?;
        let m = &r.muralla;
        if rec.nivel >= m.nivel_maximo {
            return Err(ErrorRecinto::EnElMaximo);
        }
        let necesario = m.nivel_de_plaza[rec.nivel as usize];
        if self.nivel_actual() < necesario {
            return Err(ErrorRecinto::NivelDePlaza { necesario, actual: self.nivel_actual() });
        }
        if !rec.completo() {
            return Err(ErrorRecinto::Incompleto);
        }
        if rec.mejora.is_some() {
            return Err(ErrorRecinto::YaMejorando);
        }
        Ok(())
    }

    /// Empieza la mejora (gratis, como comprometer): el progreso va aparte y el anillo en pie sigue como estaba. Ya se comprobó con [`Self::comprobar_mejora`].
    pub fn iniciar_mejora(&mut self, recinto: u32, ev: &mut Vec<Evento>) {
        let rec = self.recintos.iter_mut().find(|x| x.id == recinto).expect("se comprobó que existe");
        rec.mejora = Some(Mejora { nivel: rec.nivel + 1, avance: -1 });
        rec.siguiente_celda_en = None;
        ev.push(Evento::RecintoMejoraIniciada { recinto, nivel: rec.nivel + 1 });
    }

    /// Abandona un recinto que **nunca se ha cerrado** (Q2): se borra entero, celdas levantadas incluidas, sin devolución, y su suelo queda libre.
    pub fn abandonar_recinto(&mut self, recinto: u32, ev: &mut Vec<Evento>) -> Result<(), ErrorRecinto> {
        let i = self.recintos.iter().position(|x| x.id == recinto).ok_or(ErrorRecinto::NoExiste)?;
        if self.recintos[i].completado_en.is_some() {
            return Err(ErrorRecinto::YaCerrado);
        }
        self.recintos.remove(i);
        ev.push(Evento::RecintoAbandonado { recinto });
        Ok(())
    }

    /// Los instantes en que espera una celda (para quien no lleva cola de eventos).
    pub(crate) fn fines_de_recintos(&self) -> impl Iterator<Item = Instante> + '_ {
        self.recintos.iter().filter_map(|r| r.siguiente_celda_en)
    }

    /// La obra de los recintos en `t` (cita de una celda, orden recién dada o paso de minuto): si algún recinto tiene una celda por levantar que ya
    /// toca, calcula la reserva de mantenimiento y comida y la levanta. Sin recintos en obra no hace nada.
    pub fn levantar_recintos(&mut self, t: Instante, x: &mut Exterior, ev: &mut Vec<Evento>) {
        if self.recintos.iter().any(|r| r.vence(t)) {
            let reserva = self.reserva(x.reglas, x, true);
            self.avanzar_recintos(t, x, &reserva, ev);
        }
    }

    /// Una pasada de obra (Murallas §5): de cada recinto en obra, dañado o con mejora que ya pueda, **una celda** en el orden del recorrido, pagándola al
    /// levantarla. Si no hay recursos, o pagarlos dejaría un recurso por debajo de la reserva, no hace nada (no falla, no se cancela, no hay deuda).
    /// No ocupa cuadrilla ni entra en la cola de edificios. Cada celda deja su cita en `siguiente_celda_en` y su [`Evento::RecintoCelda`].
    pub fn avanzar_recintos(&mut self, t: Instante, x: &mut Exterior, reserva: &[Mil; 29], ev: &mut Vec<Evento>) {
        let r = x.reglas;
        let factor = self.politicas.factor(&r.politicas, "factorTiempoMuralla");
        for i in 0..self.recintos.len() {
            let rec = &self.recintos[i];
            let Some((pagado, clase, mejorando)) = rec.siguiente().filter(|_| rec.vence(t)) else { continue };
            let coste = if mejorando { r.muralla.coste_de_mejora_de_celda(clase, pagado) } else { r.muralla.coste_de_celda(clase, pagado) };
            let cabe = coste.iter().all(|&(rc, n)| reserva[rc.indice()] <= 0 || self.almacen.cantidad(rc) - n >= reserva[rc.indice()]);
            if !cabe || !self.almacen.pagar(&coste) {
                continue;
            }
            let rec = &mut self.recintos[i];
            let ultima = rec.celdas.len() as i32 - 1;
            let levantadas = match &mut rec.mejora {
                Some(m) => {
                    m.avance += 1;
                    m.avance
                }
                None => {
                    rec.avance += 1;
                    rec.avance
                }
            };
            if levantadas < ultima {
                let siguiente = t + r.muralla.duracion_de_celda(pagado, factor);
                rec.siguiente_celda_en = Some(siguiente);
                ev.push(Evento::RecintoCelda { recinto: rec.id, levantadas: levantadas as usize + 1, siguiente_en: Some(siguiente) });
                continue;
            }
            // La última celda: el anillo se cierra (por primera vez, o tras repararse) o la mejora termina.
            rec.siguiente_celda_en = None;
            ev.push(Evento::RecintoCelda { recinto: rec.id, levantadas: levantadas as usize + 1, siguiente_en: None });
            if let Some(m) = rec.mejora.take() {
                rec.nivel = m.nivel;
                x.sitios.reclasificar_torres(&mut rec.celdas, m.nivel, &r.muralla.parametros_de_trazo());
                ev.push(Evento::RecintoMejorado { recinto: rec.id, nivel: rec.nivel, celdas: rec.celdas.len(), puertas: rec.puertas() });
            } else if rec.completado_en.is_none() {
                rec.completado_en = Some(t);
                ev.push(Evento::RecintoCompletado { recinto: rec.id, nivel: rec.nivel, celdas: rec.celdas.len(), puertas: rec.puertas() });
            }
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::asentamiento::{Asentamiento, SitiosLibres};
    use crate::catalogo::Tipo;
    use crate::datos::Reglas;
    use crate::politicas::{Activa, Politicas};
    use trazado::Celda;

    fn reglas() -> Reglas {
        Reglas::cargar()
    }

    /// Un anillo sintético de `largo` celdas en una fila, con las puertas en las posiciones dadas y torres cada `torre` celdas (0 = ninguna).
    fn anillo(largo: i32, puertas: &[i32], torre: i32) -> Vec<CeldaMuro> {
        (0..largo)
            .map(|k| {
                let clase = if puertas.contains(&k) {
                    ClaseCelda::Puerta
                } else if torre > 0 && k % torre == torre - 1 {
                    ClaseCelda::Torre
                } else {
                    ClaseCelda::Muro
                };
                CeldaMuro { celda: Celda::new(k, 0), clase }
            })
            .collect()
    }

    fn recinto(id: u32, nivel: u8, celdas: Vec<CeldaMuro>, avance: i32, cerrado: bool) -> Recinto {
        Recinto { id, nivel, celdas, red_de_puertas: vec![], tras_pieza: 0, avance, mejora: None, siguiente_celda_en: None, comprometido_en: 0, completado_en: cerrado.then_some(1) }
    }

    /// Una plaza rica con el nivel operativo pedido, sin reserva (el mantenimiento de una plaza nueva está en gracia).
    fn plaza(r: &Reglas, nivel: u32) -> Asentamiento {
        let mut a = Asentamiento::fundar(r, 0, &[], r.viviendas_iniciales);
        for rc in crate::recurso::TODOS {
            a.almacen.ampliar(rc, 100_000_000);
            a.almacen.poner(rc, 50_000_000);
        }
        a.nivel = nivel;
        a.medidor.nivel_actual = nivel;
        a
    }

    fn exterior<'a>(r: &'a Reglas, f: &'a mut dyn crate::produccion::Fuentes, s: &'a mut dyn crate::asentamiento::Sitios) -> Exterior<'a> {
        Exterior {
            reglas: r,
            fuentes: f,
            sitios: s,
            fertilidad: UNO,
            tiene_tecnologia: &|_| false,
            factor_tecnologia: &|_| UNO,
            nivel_faccion: 1,
            orden_era: 1,
            es_capital: true,
            faccion_tiene_gran_fundicion: false,
            ciudadanos: 1,
            distancia_capital: 0,
            racion_guarnicion: 0,
            felicidad_extra: UNO,
            nobleza_extra: UNO,
            recaudacion_extra: UNO,
        }
    }

    struct SinMundo;
    impl crate::produccion::Fuentes for SinMundo {
        fn densidad_parcela(&self, _: u32) -> i64 {
            0
        }
        fn extraer(&mut self, _: u32, _: Mil) -> Mil {
            0
        }
    }

    /// Una pasada de obra de la plaza con una reserva dada.
    fn pasada(r: &Reglas, a: &mut Asentamiento, t: Instante, reserva: &[Mil; 29]) -> Vec<Evento> {
        let (mut f, mut s) = (SinMundo, SitiosLibres);
        let mut x = exterior(r, &mut f, &mut s);
        let mut ev = vec![];
        a.avanzar_recintos(t, &mut x, reserva, &mut ev);
        ev
    }

    const MIN: i64 = MS_POR_MINUTO;

    #[test]
    fn el_bloque_se_lee_del_documento() {
        let m = reglas().muralla;
        assert_eq!((m.nivel_maximo, m.franja_de_ronda, m.arrabal_minimo, m.factor_puerta, m.factor_torre), (3, 1, 6, 4, 3));
        assert_eq!((m.puertas_maximas, m.puertas_normales), (4, [2, 3]));
        assert_eq!((m.nivel_de_plaza, m.paso_torres, m.minutos_por_celda, m.bono_defensa), ([2, 3, 4], [0, 8, 5], [15, 30, 60], [1_300_000, 1_800_000, 2_500_000]));
        assert_eq!(m.tarifa_por_celda[0], vec![(Recurso::Madera, 20_000), (Recurso::Piedra, 2_000)]);
        assert_eq!(m.upkeep_por_celda[2], vec![(Recurso::Piedra, 40)]);
        assert_eq!(m.parametros_de_trazo(), ParametrosMuralla::default());
    }

    #[test]
    fn el_coste_es_la_tarifa_acumulada_y_mejorar_cuesta_la_diferencia() {
        let m = reglas().muralla;
        let total = |c: &Costo| c.iter().map(|&(_, n)| n).sum::<i64>();
        // Una celda llana construida directamente: 22, 52 y 82 en total (Q4).
        assert_eq!([1, 2, 3].map(|n| total(&m.coste_de_celda(ClaseCelda::Muro, n))), [22_000, 52_000, 82_000]);
        // Mejorar cuesta la diferencia con el nivel anterior: 1→2 son 5 de madera y 25 de piedra; 2→3, 10 y 20.
        assert_eq!(m.coste_de_mejora_de_celda(ClaseCelda::Muro, 2), vec![(Recurso::Madera, 5_000), (Recurso::Piedra, 25_000)]);
        assert_eq!(m.coste_de_mejora_de_celda(ClaseCelda::Muro, 3), vec![(Recurso::Madera, 10_000), (Recurso::Piedra, 20_000)]);
        // Y subir nivel a nivel cuesta lo mismo que construir directamente, por clase y por recurso.
        for clase in [ClaseCelda::Muro, ClaseCelda::Puerta, ClaseCelda::Torre] {
            let mut subiendo = m.coste_de_celda(clase, 1);
            for destino in [2, 3] {
                for (rc, n) in m.coste_de_mejora_de_celda(clase, destino) {
                    sumar(&mut subiendo, rc, n);
                }
            }
            assert_eq!(subiendo, m.coste_de_celda(clase, 3), "{clase:?}");
        }
        // Una puerta cuesta ×4 y una torre ×3.
        assert_eq!(total(&m.coste_de_celda(ClaseCelda::Puerta, 1)), 88_000);
        assert_eq!(total(&m.coste_de_celda(ClaseCelda::Torre, 3)), 246_000);
        // El trazo: Σ por clase (10 celdas de las cuales 2 puertas y 1 torre = 7 + 8 + 3 = 18 unidades).
        let trazo = anillo(10, &[0, 5], 10);
        assert_eq!(trazo.iter().filter(|c| c.clase == ClaseCelda::Torre).count(), 1);
        assert_eq!(m.coste_de_trazo(&trazo, 2), vec![(Recurso::Madera, 25_000 * 18), (Recurso::Piedra, 27_000 * 18)]);
    }

    #[test]
    fn comprometer_exige_nivel_de_plaza_y_nada_en_obra_y_es_gratis() {
        let r = reglas();
        let mut a = plaza(&r, 1);
        assert_eq!(a.comprobar_comprometer(&r, 0), Err(ErrorRecinto::NivelInexistente));
        assert_eq!(a.comprobar_comprometer(&r, 4), Err(ErrorRecinto::NivelInexistente));
        assert_eq!(a.comprobar_comprometer(&r, 1), Err(ErrorRecinto::NivelDePlaza { necesario: 2, actual: 1 }));
        a = plaza(&r, 2);
        assert_eq!(a.comprobar_comprometer(&r, 2), Err(ErrorRecinto::NivelDePlaza { necesario: 3, actual: 2 }));
        assert_eq!(a.comprobar_comprometer(&r, 1), Ok(None));
        let mut ev = vec![];
        assert_eq!(a.comprometer_recinto(7, 1, anillo(20, &[3], 0), vec![], 0, &mut ev), 1);
        assert_eq!(ev, vec![Evento::RecintoComprometido { recinto: 1, nivel: 1, celdas: 20, puertas: 1 }]);
        assert_eq!(a.almacen, plaza(&r, 2).almacen, "comprometer no cobra nada");
        // Con uno en obra no cabe otro; el recinto exterior es el que devuelve la comprobación una vez cerrado.
        assert_eq!(a.comprobar_comprometer(&r, 1), Err(ErrorRecinto::EnObra));
        a.recintos[0].avance = 19;
        assert_eq!(a.comprobar_comprometer(&r, 1).map(|o| o.map(|x| x.id)), Ok(Some(1)));
        // Un recinto dañado por una conquista también bloquea hasta repararse.
        a.recintos[0].completado_en = Some(5);
        a.recintos[0].avance = 13;
        assert_eq!(a.comprobar_comprometer(&r, 1), Err(ErrorRecinto::EnObra));
        // El arrabal: 5 no basta, 6 sí.
        assert_eq!(r.muralla.arrabal_suficiente(5), Err(ErrorRecinto::Arrabal { extramuros: 5, minimo: 6 }));
        assert_eq!(r.muralla.arrabal_suficiente(6), Ok(()));
    }

    #[test]
    fn la_obra_levanta_una_celda_por_cita_en_orden_y_cobra_cada_una() {
        let r = reglas();
        let mut a = plaza(&r, 2);
        let celdas = anillo(4, &[0], 0);
        a.comprometer_recinto(0, 1, celdas, vec![], 0, &mut vec![]);
        let reserva = [0; 29];
        let madera = a.almacen.cantidad(Recurso::Madera);
        // La primera celda es la puerta (×4): 80 de madera y 8 de piedra.
        let ev = pasada(&r, &mut a, 0, &reserva);
        assert_eq!(ev, vec![Evento::RecintoCelda { recinto: 1, levantadas: 1, siguiente_en: Some(15 * MIN) }]);
        assert_eq!(madera - a.almacen.cantidad(Recurso::Madera), 80_000);
        assert_eq!(a.recintos[0].avance, 0);
        // Antes de la cita no se levanta otra, aunque se pida; en la cita, sí, y el instante de la siguiente cuenta desde el de esta.
        assert!(pasada(&r, &mut a, 15 * MIN - 1, &reserva).is_empty());
        let ev = pasada(&r, &mut a, 15 * MIN + 5, &reserva);
        assert_eq!(ev, vec![Evento::RecintoCelda { recinto: 1, levantadas: 2, siguiente_en: Some(30 * MIN + 5) }]);
        pasada(&r, &mut a, 30 * MIN + 5, &reserva);
        // La última celda cierra el anillo: completado_en se fija una vez y ya no hay cita.
        let ev = pasada(&r, &mut a, 45 * MIN + 5, &reserva);
        assert_eq!(
            ev,
            vec![
                Evento::RecintoCelda { recinto: 1, levantadas: 4, siguiente_en: None },
                Evento::RecintoCompletado { recinto: 1, nivel: 1, celdas: 4, puertas: 1 }
            ]
        );
        let rec = &a.recintos[0];
        assert!(rec.completo() && rec.siguiente_celda_en.is_none());
        assert_eq!(rec.completado_en, Some(45 * MIN + 5));
        assert!(pasada(&r, &mut a, 99 * MIN, &reserva).is_empty(), "un recinto completo y sin mejora no tiene obra");
    }

    #[test]
    fn sin_materiales_o_bajo_la_reserva_no_avanza_ni_se_cancela() {
        let r = reglas();
        let mut a = plaza(&r, 2);
        a.comprometer_recinto(0, 1, anillo(3, &[], 0), vec![], 0, &mut vec![]);
        // Falta piedra: ni avanza ni cobra madera.
        a.almacen.poner(Recurso::Piedra, 1_000);
        let madera = a.almacen.cantidad(Recurso::Madera);
        assert!(pasada(&r, &mut a, 0, &[0; 29]).is_empty());
        assert_eq!((a.almacen.cantidad(Recurso::Madera), a.recintos[0].avance, a.recintos[0].siguiente_celda_en), (madera, -1, None));
        // Hay madera de sobra pero pagarla dejaría la madera por debajo de la reserva.
        a.almacen.poner(Recurso::Piedra, 50_000_000);
        let mut reserva = [0; 29];
        reserva[Recurso::Madera.indice()] = madera - 19_999;
        assert!(pasada(&r, &mut a, 0, &reserva).is_empty());
        // Con la reserva justa sí se paga, y el almacén nunca baja de ella.
        reserva[Recurso::Madera.indice()] = madera - 20_000;
        assert_eq!(pasada(&r, &mut a, 0, &reserva).len(), 1);
        assert_eq!(a.almacen.cantidad(Recurso::Madera), reserva[Recurso::Madera.indice()]);
        // Cuando la cita llega y no hay con qué, se queda sin cita (el paso de minuto reintenta): sigue vencida.
        a.almacen.poner(Recurso::Madera, 0);
        assert!(pasada(&r, &mut a, 15 * MIN, &[0; 29]).is_empty());
        assert!(a.recintos[0].vence(16 * MIN), "sigue esperando su celda");
    }

    #[test]
    fn el_factor_de_tiempo_de_la_postura_defensiva_acorta_cada_celda() {
        let r = reglas();
        let mut a = plaza(&r, 2);
        a.politicas = Politicas::desde_activas(vec![Activa { id: "postura_defensiva".into(), cargo: crate::politicas::Cargo::MaestroObras, expira_en: i64::MAX }]);
        assert_eq!(a.politicas.factor(&r.politicas, "factorTiempoMuralla"), 750_000, "la política existe en el catálogo");
        a.comprometer_recinto(0, 1, anillo(3, &[], 0), vec![], 0, &mut vec![]);
        let ev = pasada(&r, &mut a, 0, &[0; 29]);
        assert_eq!(ev, vec![Evento::RecintoCelda { recinto: 1, levantadas: 1, siguiente_en: Some(675_000) }], "15 min × 0,75 = 11,25 min exactos");
    }

    #[test]
    fn mejorar_lleva_su_progreso_aparte_y_cobra_la_diferencia() {
        let r = reglas();
        let mut a = plaza(&r, 3);
        a.comprometer_recinto(0, 1, anillo(3, &[0], 0), vec![], 0, &mut vec![]);
        assert_eq!(a.comprobar_mejora(&r, 9), Err(ErrorRecinto::NoExiste));
        assert_eq!(a.comprobar_mejora(&r, 1), Err(ErrorRecinto::Incompleto));
        for k in 0..3 {
            pasada(&r, &mut a, k * 15 * MIN, &[0; 29]);
        }
        assert!(a.recintos[0].completo());
        assert_eq!(a.comprobar_mejora(&r, 1), Ok(()));
        let mut ev = vec![];
        a.iniciar_mejora(1, &mut ev);
        assert_eq!(ev, vec![Evento::RecintoMejoraIniciada { recinto: 1, nivel: 2 }]);
        assert_eq!(a.comprobar_mejora(&r, 1), Err(ErrorRecinto::YaMejorando));
        // El anillo en pie no se toca: sigue completo, a su nivel y con todo su avance (Q1).
        let rec = &a.recintos[0];
        assert_eq!((rec.nivel, rec.avance, rec.completo(), rec.mejora), (1, 2, true, Some(Mejora { nivel: 2, avance: -1 })));
        assert_eq!(nivel_del_mejor_completo(&a.recintos), 1);
        let madera = a.almacen.cantidad(Recurso::Madera);
        // La primera celda a mejorar es la puerta: la diferencia (5 de madera) × 4; tarda lo del nivel destino (30 min).
        let ev = pasada(&r, &mut a, 100 * MIN, &[0; 29]);
        assert_eq!(ev, vec![Evento::RecintoCelda { recinto: 1, levantadas: 1, siguiente_en: Some(130 * MIN) }]);
        assert_eq!(madera - a.almacen.cantidad(Recurso::Madera), 20_000);
        pasada(&r, &mut a, 130 * MIN, &[0; 29]);
        let ev = pasada(&r, &mut a, 160 * MIN, &[0; 29]);
        assert_eq!(ev.last(), Some(&Evento::RecintoMejorado { recinto: 1, nivel: 2, celdas: 3, puertas: 1 }));
        let rec = &a.recintos[0];
        assert_eq!((rec.nivel, rec.mejora, rec.siguiente_celda_en), (2, None, None));
        assert_eq!(a.comprobar_mejora(&r, 1), Err(ErrorRecinto::NivelDePlaza { necesario: 4, actual: 3 }), "adarve desde el nivel 4");
        a.nivel = 4;
        a.medidor.nivel_actual = 4;
        a.recintos[0].nivel = 3;
        assert_eq!(a.comprobar_mejora(&r, 1), Err(ErrorRecinto::EnElMaximo));
    }

    #[test]
    fn el_mantenimiento_cuenta_las_celdas_en_pie_y_no_cambia_al_mejorar() {
        let r = reglas();
        let m = &r.muralla;
        assert!(upkeep(m, &[]).is_empty());
        assert!(upkeep(m, &[recinto(1, 1, anillo(10, &[], 0), -1, false)]).is_empty(), "lo que no se ha levantado no cuesta");
        // 0,02 de madera por celda en pie del nivel 1; 0,02 de piedra del nivel 2.
        let medio = recinto(1, 1, anillo(10, &[], 0), 3, false);
        assert_eq!(upkeep(m, &[medio.clone()]), vec![(Recurso::Madera, 80)]);
        let mut piedra = recinto(2, 2, anillo(30, &[1], 0), 29, true);
        assert_eq!(upkeep(m, &[medio.clone(), piedra.clone()]), vec![(Recurso::Madera, 80), (Recurso::Piedra, 600)]);
        // Mejorar a adarve: el anillo en pie sigue pagando como piedra hasta cerrarse la mejora (K6).
        piedra.mejora = Some(Mejora { nivel: 3, avance: 14 });
        assert_eq!(upkeep(m, &[piedra.clone()]), vec![(Recurso::Piedra, 600)]);
        piedra.mejora = None;
        piedra.nivel = 3;
        assert_eq!(upkeep(m, &[piedra]), vec![(Recurso::Piedra, 1_200)]);
        // Un recinto dañado paga las celdas que le quedan.
        assert_eq!(upkeep(m, &[recinto(1, 2, anillo(30, &[], 0), 19, true)]), vec![(Recurso::Piedra, 400)]);
    }

    #[test]
    fn el_multiplicador_reparte_el_bono_entre_las_puertas_y_mide_la_integridad() {
        let r = reglas();
        let m = &r.muralla;
        assert_eq!(multiplicador_defensivo(m, &[]), UNO);
        let con = |puertas: &[i32], avance: i32| recinto(1, 3, anillo(40, puertas, 0), avance, true);
        // Los ejemplos del documento (nivel 3, anillo entero): ×2,5 con 1 puerta, ×1,375 con 4, ×1,1875 con 8.
        assert_eq!(multiplicador_defensivo(m, &[con(&[0], 39)]), 2_500_000);
        assert_eq!(multiplicador_defensivo(m, &[con(&[0, 5, 10, 15], 39)]), 1_375_000);
        assert_eq!(multiplicador_defensivo(m, &[con(&[0, 5, 10, 15, 20, 25, 30, 35], 39)]), 1_187_500);
        // La integridad es lineal: medio anillo da la mitad del extra, y sin celdas no pasa de 1.
        assert_eq!(multiplicador_defensivo(m, &[con(&[0], 19)]), 1_750_000);
        assert_eq!(multiplicador_defensivo(m, &[con(&[0], -1)]), UNO);
        // Monótono: más puertas, menos; más celdas, más; nunca por debajo de 1.
        let mut anterior = i64::MAX;
        for p in 1..=10 {
            let puertas: Vec<i32> = (0..p).map(|k| k * 3).collect();
            let v = multiplicador_defensivo(m, &[con(&puertas, 39)]);
            assert!(v <= anterior && v >= UNO, "{p} puertas: {v}");
            anterior = v;
        }
        // Sin puertas no hay a quién repartir: neutro.
        assert_eq!(multiplicador_defensivo(m, &[con(&[], 39)]), UNO);
    }

    #[test]
    fn q1_el_recinto_de_referencia_es_el_mas_exterior_de_los_que_se_cerraron() {
        let r = reglas();
        let m = &r.muralla;
        let interior = recinto(1, 2, anillo(40, &[0], 0), 39, true);
        // Una ampliación a medio hacer, o sin empezar, no quita el multiplicador del anillo interior completo.
        for avance in [-1, 10] {
            let ampliacion = recinto(2, 3, anillo(80, &[0], 0), avance, false);
            assert_eq!(de_referencia(&[interior.clone(), ampliacion.clone()]).map(|x| x.id), Some(1));
            assert_eq!(multiplicador_defensivo(m, &[interior.clone(), ampliacion.clone()]), 1_800_000);
            assert_eq!(nivel_del_mejor_completo(&[interior.clone(), ampliacion]), 2, "cupo y requisito del nivel 4 también");
        }
        // Cerrada, la ampliación pasa a ser la referencia (aunque tenga menos nivel que el interior).
        let exterior = recinto(2, 1, anillo(80, &[0], 0), 79, true);
        assert_eq!(de_referencia(&[interior.clone(), exterior.clone()]).map(|x| x.id), Some(2));
        assert_eq!(multiplicador_defensivo(m, &[interior.clone(), exterior.clone()]), 1_300_000);
        assert_eq!(nivel_del_mejor_completo(&[interior.clone(), exterior]), 2, "el cupo y el requisito miran el de mayor nivel entre los completos");
        // Mejorar el único recinto no le quita nada: sigue con su nivel de antes.
        let mut mejorando = interior.clone();
        mejorando.mejora = Some(Mejora { nivel: 3, avance: 4 });
        assert_eq!(multiplicador_defensivo(m, &[mejorando.clone()]), 1_800_000);
        assert_eq!(nivel_del_mejor_completo(&[mejorando]), 2);
        // Si ninguno se ha cerrado, el más exterior que haya.
        let a = recinto(1, 1, anillo(40, &[0], 0), 9, false);
        assert_eq!(de_referencia(&[a]).map(|x| x.id), Some(1));
        // Uno dañado por una conquista sigue siendo la referencia (con poca integridad) y no cuenta como completo; el interior completo sí.
        let danado = recinto(2, 3, anillo(80, &[0], 0), 55, true);
        assert!(danado.danado());
        assert_eq!(multiplicador_defensivo(m, &[interior.clone(), danado.clone()]), UNO + 1_500_000 * 56 / 80);
        assert_eq!(nivel_del_mejor_completo(&[interior, danado]), 2);
    }

    #[test]
    fn la_conquista_daña_un_30_por_ciento_a_los_completos_sin_mejora_y_conserva_completado_en() {
        let mut rs = vec![
            recinto(1, 2, anillo(40, &[0], 0), 39, true),
            recinto(2, 2, anillo(40, &[0], 0), 39, true),
            recinto(3, 2, anillo(40, &[0], 0), 12, false),
        ];
        rs[1].mejora = Some(Mejora { nivel: 3, avance: 2 });
        rs[0].siguiente_celda_en = Some(99);
        assert!(danar_por_conquista(&mut rs, 300_000));
        // Un completo pierde floor(0,3 × 40) = 12 celdas; uno en mejora y uno en obra no reciben daño.
        assert_eq!((rs[0].avance, rs[1].avance, rs[2].avance), (27, 39, 12));
        assert_eq!(rs[0].siguiente_celda_en, None, "queda sin cita: la reparación la programa quien aplica el daño");
        assert!(rs[0].danado() && rs[0].completado_en.is_some() && !rs[0].completo());
        assert_eq!(nivel_del_mejor_completo(&rs[..1]), 0, "dañado no cuenta");
        // Dañado ya no está completo: otra conquista antes de repararlo no lo hunde más (como en el TypeScript).
        assert!(!danar_por_conquista(&mut rs[..1], 300_000));
        assert_eq!(rs[0].avance, 27);
        // Reparado vuelve a recibir daño, y nunca baja de −1 (con una fracción del 100 % queda sin celdas).
        rs[0].avance = 39;
        assert!(danar_por_conquista(&mut rs[..1], UNO));
        assert_eq!(rs[0].avance, -1);
        assert!(!danar_por_conquista(&mut rs[..1], UNO), "sin celdas no hay nada que dañar");
        // Un recinto pequeño: floor(0,3 × 3) = 0, no pierde nada.
        let mut chico = vec![recinto(1, 1, anillo(3, &[], 0), 2, true)];
        assert!(!danar_por_conquista(&mut chico, 300_000));
    }

    #[test]
    fn muralla_completa_es_tener_un_recinto_con_integridad_uno_aunque_se_mejore_o_se_amplie() {
        assert!(!muralla_completa(&[]));
        let mut piedra = recinto(1, 2, anillo(20, &[0], 0), 19, true);
        assert!(muralla_completa(&[piedra.clone()]));
        piedra.mejora = Some(Mejora { nivel: 3, avance: 2 });
        assert!(muralla_completa(&[piedra.clone()]), "una mejora en curso no quita el anillo en pie");
        assert!(muralla_completa(&[piedra.clone(), recinto(2, 1, anillo(40, &[0], 0), 5, false)]), "ni una ampliación a medio hacer");
        assert!(!muralla_completa(&[recinto(1, 1, anillo(20, &[0], 0), 18, false)]), "a falta de una celda no está completa");
        piedra.mejora = None;
        piedra.avance = 13;
        assert!(!muralla_completa(&[piedra]), "uno dañado por una conquista deja de contar hasta repararse");
    }

    #[test]
    fn abandonar_solo_vale_para_lo_que_nunca_se_cerro() {
        let r = reglas();
        let mut a = plaza(&r, 2);
        let mut ev = vec![];
        a.comprometer_recinto(0, 1, anillo(5, &[], 0), vec![], 0, &mut ev);
        assert_eq!(a.abandonar_recinto(9, &mut ev), Err(ErrorRecinto::NoExiste));
        // A medio levantar se puede: se borra entero y el id queda libre.
        a.recintos[0].avance = 2;
        ev.clear();
        assert_eq!(a.abandonar_recinto(1, &mut ev), Ok(()));
        assert_eq!((a.recintos.len(), ev), (0, vec![Evento::RecintoAbandonado { recinto: 1 }]));
        assert_eq!(a.comprometer_recinto(0, 1, anillo(5, &[], 0), vec![], 0, &mut vec![]), 1);
        // Uno que alguna vez estuvo completo no, ni siquiera dañado por una conquista.
        a.recintos[0].avance = 4;
        a.recintos[0].completado_en = Some(9);
        assert_eq!(a.abandonar_recinto(1, &mut vec![]), Err(ErrorRecinto::YaCerrado));
        a.recintos[0].avance = 0;
        assert_eq!(a.abandonar_recinto(1, &mut vec![]), Err(ErrorRecinto::YaCerrado));
        assert_eq!(a.recintos.len(), 1);
    }

    #[test]
    fn la_reparacion_de_un_recinto_danado_es_la_obra_normal_y_no_vuelve_a_anunciar_el_cierre() {
        let r = reglas();
        let mut a = plaza(&r, 2);
        a.comprometer_recinto(0, 1, anillo(4, &[], 0), vec![], 0, &mut vec![]);
        a.recintos[0].avance = 3;
        a.recintos[0].completado_en = Some(5);
        danar_por_conquista(&mut a.recintos, 500_000);
        assert_eq!(a.recintos[0].avance, 1);
        // Repara a la tarifa del nivel del recinto, desde la primera celda sin levantar.
        let ev = pasada(&r, &mut a, 10, &[0; 29]);
        assert_eq!(ev, vec![Evento::RecintoCelda { recinto: 1, levantadas: 3, siguiente_en: Some(10 + 15 * MIN) }]);
        let ev = pasada(&r, &mut a, 10 + 15 * MIN, &[0; 29]);
        assert_eq!(ev, vec![Evento::RecintoCelda { recinto: 1, levantadas: 4, siguiente_en: None }], "sin evento propio de cierre");
        assert!(a.recintos[0].completo());
        assert_eq!(a.recintos[0].completado_en, Some(5), "completado_en no se toca");
    }

    #[test]
    fn el_requisito_del_nivel_4_lee_el_mejor_completo_y_una_mejora_o_una_ampliacion_no_lo_quitan() {
        use crate::edificio::{Edificio, Estado as EstadoEdificio};
        use crate::nivel::Falta;
        let r = reglas();
        let mut sala = Edificio::nuevo(0, Tipo::SalaConsejo, EstadoEdificio::Activo);
        sala.nivel_interno = 1;
        let gate = |rs: &[Recinto]| r.nivel.gates(4, &[sala.clone()], 1_000, 800, nivel_del_mejor_completo(rs)).unwrap();
        assert_eq!(gate(&[]), vec![Falta::Edificios]);
        assert_eq!(gate(&[recinto(1, 1, anillo(20, &[0], 0), 19, true)]), vec![Falta::Edificios], "la empalizada no basta");
        let mut piedra = recinto(1, 2, anillo(20, &[0], 0), 19, true);
        assert_eq!(gate(&[piedra.clone()]), vec![]);
        piedra.mejora = Some(Mejora { nivel: 3, avance: 2 });
        assert_eq!(gate(&[piedra.clone()]), vec![], "una mejora en curso cuenta con el nivel de antes");
        assert_eq!(gate(&[piedra.clone(), recinto(2, 1, anillo(40, &[0], 0), 5, false)]), vec![], "y una ampliación en obra, también");
        piedra.mejora = None;
        piedra.avance = 13;
        assert_eq!(gate(&[piedra]), vec![Falta::Edificios], "uno dañado por una conquista no vale hasta repararse");
    }

    #[test]
    fn sin_cola_de_eventos_avanzar_hasta_levanta_cada_celda_en_su_instante_y_cierra_el_anillo() {
        let r = reglas();
        let mut a = plaza(&r, 2);
        a.comprometer_recinto(0, 1, anillo(5, &[], 0), vec![], 0, &mut vec![]);
        let (mut f, mut s) = (SinMundo, SitiosLibres);
        let mut x = exterior(&r, &mut f, &mut s);
        let mut ev = vec![];
        a.levantar_recintos(0, &mut x, &mut ev);
        assert_eq!(a.recintos[0].avance, 0, "la primera celda sale en el instante de la orden");
        // Una celda cada 15 minutos, aunque el paso de minuto caiga en el mismo instante: sale la cita y el paso no levanta otra.
        let mut cerrada_en = None;
        for minuto in 1..=90 {
            for e in a.avanzar_hasta(minuto * MIN, &mut x) {
                if let Evento::RecintoCompletado { .. } = e {
                    cerrada_en = Some(minuto * MIN);
                }
            }
            assert_eq!(a.recintos[0].avance, (minuto / 15).min(4) as i32, "minuto {minuto}");
        }
        assert_eq!((cerrada_en, a.recintos[0].completado_en), (Some(60 * MIN), Some(60 * MIN)));
        assert!(a.recintos[0].siguiente_celda_en.is_none() && a.fines_de_recintos().next().is_none());
    }

    #[test]
    fn la_proyeccion_para_la_ciudad_separa_muros_y_puertas_y_marca_los_completos() {
        let r = anillo(6, &[1], 3);
        let ya = recinto(1, 2, r.clone(), 5, true);
        let a = proyeccion(&[ya, recinto(2, 1, r, 2, false)]);
        assert_eq!(a.len(), 2);
        assert_eq!(a[0].puertas, vec![Celda::new(1, 0)]);
        assert_eq!(a[0].muros.len(), 5, "muro y torre ocupan suelo");
        assert_eq!((a[0].completo, a[1].completo), (true, false));
        assert_eq!(a[1].muros.len(), 5, "las celdas sin levantar también ocupan suelo");
    }
}
