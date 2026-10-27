//! El batch tecnológico (sistema 6, tanda 6H): la población del batch político ([`super::pol`]) con **bots de tecnología** mínimos y deterministas por semilla que solo dan órdenes del
//! contrato, y los invariantes del sistema 6 comprobados **orden a orden**. Comprueba reglas, no cifras (Q3 del autor): las X de los logros, la velocidad y la estancia de los Aedas son las
//! del TypeScript hasta la fase 7. `--escala-logros D` divide cada umbral por `D` (experimento para ejercitar las reglas en pocos días; el bloque no cambia).
//!
//! # Los bots de tecnología
//! - **El Rey** adopta en su capital lo que le aparece (`Sim::adoptar`, la vía de juego de todos los batches desde 6H).
//! - **El Gobernador de la capital** construye hacia los hitos: cada `OBRAS_CADA_H` horas, el primer edificio (o el nivel interno) que pide el hito de una tecnología de una Era abierta que
//!   a su Facción aún no le apareció y que ninguna plaza suya tiene (`AnadirEdificio`, `MejorarEdificio`).
//! - **El Gobernador de cada plaza**, con un Aeda itinerante parado en ella, le compra lo que le reveló (`ComprarTecnologiaAeda`) si el almacén tiene el oro; con el `PUERTA_AEDAS_PCT` %
//!   les cierra la puerta (`FijarPuerta` con el grupo `aedas`) y la vuelve a abrir a las `PUERTA_AEDAS_H` horas; a cada Aeda residente sin épica le encarga la primera de las épicas posibles de
//!   su Facción (`EmpezarEpica`) y, con el `ABANDONO_POR_MIL` ‰ de sus turnos, abandona una en curso (`AbandonarEpica`).
//! - **Los buscadores** (`BUSCADORES` héroes sin Facción, cada uno de un campamento) salen de su campamento, abren hasta `ALIJOS_POR_SALIDA` alijos cerca de él (`AbrirAlijo`; el
//!   `REINTENTO_PCT` % de las veces lo intentan otra vez, y tiene que rechazarse), vuelven, vacían el carro y reclutan en su campamento con el oro (`ReclutarEnCampamento`, Q4).
//!
//! # Invariantes (todos a 0)
//! Tras cada orden: el logro es fijo y nunca antes de su umbral ni después (sin fijar con el contador cruzado); ningún contador baja; `adoptadas ⊆ aparecidas` y nada revelado está
//! aparecido; nada aparecido sin su logro ni de una Era cerrada; la Era pasa de una en una, nunca antes de su mínimo ni después de su plazo (ni sigue pasados su mínimo y su último logro);
//! una épica por tecnología y Facción y ninguna de una ya aparecida; los itinerantes nunca bajan y ninguno está parado en una plaza que les cierra la puerta. En cada hecho: lo que aparece
//! por hito, compra o épica tiene su logro y su Era; un héroe abre un alijo una vez. En cada observación: los contadores discretos suben exactamente lo que dicen los hechos (un hecho no
//! cuenta dos veces, también el asedio con números, D18). Al final: la crónica guardada es exactamente la emitida. Y los del político y el militar, la reproducción y la foto ida y vuelta (en el
//! mismo directorio de la partida, con la crónica).

use super::*;
use asentamiento::edificio::Edificios;
use contrato::v1 as pb;
use nucleo::{rechazos, Dato, PagarCon};
use politica::tipos::{Grupo, Grupos};
use tecnologia::contador::{self as cont, Contador, Delta};
use tecnologia::datos::Condicion;
use tecnologia::TecId;

// ── Supuestos del batch tecnológico (hipótesis sintéticas, se anotan en `parametros` del informe) ──
pub const BUSCADORES: usize = 6;
/// Un buscador abre alijos a esta distancia de donde está como mucho (milésimas), hasta `ALIJOS_POR_SALIDA` por salida, y descansa en su campamento entre salidas.
pub const RADIO_ALIJOS: i64 = 200_000;
pub const ALIJOS_POR_SALIDA: u32 = 3;
pub const DESCANSO_BUSCADOR_H: i64 = 6;
pub const REINTENTO_PCT: u64 = 30;
pub const PUERTA_AEDAS_PCT: u64 = 3;
pub const PUERTA_AEDAS_H: i64 = 12;
pub const ABANDONO_POR_MIL: u64 = 1;
pub const OBRAS_CADA_H: i64 = 2;
/// La carga de trigo (milésimas) de las caravanas del Intendente en este batch: por encima de `cargaMinimaCaravana` (100), para que inspiren el capítulo `caravana` (en el militar, 30).
pub const CARGA_DE_CARAVANA: i64 = 120_000;

/// Una épica sin un hecho en estos días se abandona por atascada.
pub const ATASCADA_DIAS: i64 = 4;

/// Los contadores discretos que salen enteros de los hechos (lo extraído y lo fabricado vienen del paso de minuto, sin hecho; los récords, del índice de plazas).
const DE_HECHOS: [Contador; 14] = [
    Contador::CaravanasDestruidasOCapturadas,
    Contador::BandidosCampamentosDestruidos,
    Contador::AnimalesComprados,
    Contador::BatallasLibradas,
    Contador::BatallasCampoAbierto,
    Contador::AsediosResistidosEnCombate,
    Contador::AsediosContraMurallaCompleta,
    Contador::ConquistasConMurallaCompleta,
    Contador::ReclutadosEscuadrones,
    Contador::ReclutadosArqueros,
    Contador::ReclutadosCaballeria,
    Contador::ReclutadosEscaramuzadores,
    Contador::ReclutadosArquerosCompuesto,
    Contador::ReclutadosJinetesAsirios,
];

/// Lo que se puede variar del escenario tecnológico sin tocar ninguna regla.
#[derive(Clone, Copy, Debug)]
pub struct Ajustes {
    /// 1 = las X del TypeScript (Q3). Mayor: cada umbral dividido por ella (experimento).
    pub escala_logros: u64,
}

impl Default for Ajustes {
    fn default() -> Self {
        Self { escala_logros: 1 }
    }
}

/// Divide cada umbral de logro por `escala` (como mínimo 1) y rehace lo derivado del catálogo.
pub fn escalar_logros(r: &mut tecnologia::Reglas, escala: u64) {
    for d in r.catalogo.iter_mut() {
        if let Some(l) = &mut d.logro {
            l.umbral = (l.umbral / escala).max(1);
        }
    }
    r.indexar();
}

/// La persistencia del batch: sin `fsync` por lote (la corrida no sobrevive a un corte de corriente, ni le hace falta).
pub fn config() -> persistencia::Config {
    persistencia::Config { sincronia: persistencia::Sincronia::Nunca, ..persistencia::Config::default() }
}

/// Los invariantes del sistema 6 (todos deben quedar a 0).
#[derive(Clone, Copy, Debug, Default)]
pub struct ViolacionesTec {
    /// Un logro que cambia de instante o se borra, que se fija sin que su contador cruce el umbral en ese tramo, o que no se fija con el contador cruzado.
    pub logro: u64,
    /// Un contador que baja.
    pub contador: u64,
    /// Una adoptada que no está aparecida, o una revelada que ya lo está.
    pub adoptadas: u64,
    /// Algo aparecido de una Era que no ha empezado (en su hecho o en el estado).
    pub era_cerrada: u64,
    /// Una Era que pasa antes de su mínimo o después de su plazo, que salta una, o que sigue pasados su plazo o su mínimo con todos sus logros.
    pub era: u64,
    /// Algo aparecido (por hito, compra o épica) sin su logro, o comprado sin que los Aedas lo conocieran.
    pub sin_logro: u64,
    /// Dos épicas de la misma tecnología en una Facción, o una de una tecnología que ya le apareció.
    pub epica: u64,
    /// Un tramo entre observaciones en que un contador discreto no sube exactamente lo que dicen sus hechos (o `conHoplitas` / `conResidentes` pasan del suyo).
    pub doble: u64,
    /// Los itinerantes bajan.
    pub itinerantes: u64,
    /// Un Aeda parado en una plaza que no existe o que les cierra la puerta.
    pub puerta: u64,
    /// Un héroe que abre dos veces el mismo alijo (o un segundo intento aceptado), o que carga más oro del que tenía el alijo.
    pub alijo: u64,
    /// Entradas de la crónica guardada distintas de las emitidas (o de más o de menos).
    pub cronica: u64,
}

/// Lo que se cuenta de la tecnología a lo largo de la corrida.
#[derive(Clone, Debug, Default)]
pub struct MedidasTec {
    pub adopciones: u64,
    /// Lo aparecido por hito, por compra a un Aeda y por épica (hechos).
    pub aparecidas_por: [u64; 3],
    pub compras: u64,
    pub compras_oro: i64,
    pub compra_sin_oro: u64,
    pub revelaciones: u64,
    pub cantos: u64,
    pub residentes_llegan: u64,
    pub residentes_se_van: u64,
    pub epicas_empezadas: u64,
    pub epicas_abandonadas: u64,
    pub epicas_atascadas: u64,
    pub capitulos: u64,
    pub epicas_cumplidas: u64,
    /// `aedas.epica_abandonada` por motivo.
    pub epicas_perdidas: BTreeMap<String, u64>,
    pub puertas_cerradas: u64,
    pub aedas_expulsados: u64,
    pub obras_hito: u64,
    /// Salidas del Rey hacia su capital para adoptar, y llegadas.
    pub reyes_salen: u64,
    pub reyes_llegan: u64,
    pub obras_epica: u64,
    pub minas: u64,
    /// El oro que la administración da a cada buscador al nacer (milésimas) y el precio del recluta más barato de su campamento.
    pub oro_andamiaje: i64,
    pub precio_recluta: Vec<i64>,
    pub alijos_abiertos: u64,
    pub alijos_oro: i64,
    pub alijos_cargado: i64,
    pub alijos_inalcanzables: u64,
    pub reintentos: u64,
    pub reclutas_campamento: u64,
    pub reclutados_campamento: u64,
    /// (instante, orden de la Era que empieza, por plazo).
    pub eras: Vec<(Instante, u32, bool)>,
    /// Entradas de crónica emitidas y guardadas.
    pub cronica: (usize, usize),
}

/// Una foto diaria de la tecnología.
#[derive(Clone, Debug, Default)]
pub struct MuestraTec {
    pub dia: u32,
    pub era: u8,
    pub logros: usize,
    pub aparecidas: usize,
    pub adoptadas: usize,
    pub reveladas: usize,
    pub itinerantes: usize,
    pub residentes: usize,
    pub epicas: usize,
}

/// Un buscador de alijos: un héroe sin Facción de un campamento.
#[derive(Clone, Debug)]
pub struct Buscador {
    pub heroe: u64,
    pub cuenta: String,
    pub campamento: u32,
    /// El alijo al que va (`None`: vuelve a su campamento).
    destino: Option<u32>,
    en_salida: u32,
    /// Los que no alcanza (su columna no llega a `radioPuerta`): no vuelve a por ellos.
    saltados: BTreeSet<u32>,
    cita: u32,
}

/// El estado de la tecnología tras la orden anterior.
#[derive(Clone, Debug)]
struct Previo {
    t: Instante,
    contadores: [u64; 24],
    logros: Vec<Option<Instante>>,
    era: u8,
    era_desde: Instante,
    itinerantes: usize,
}

pub struct Tec {
    pub ajustes: Ajustes,
    pub v: ViolacionesTec,
    pub m: MedidasTec,
    pub muestras: Vec<MuestraTec>,
    pub buscadores: Vec<Buscador>,
    previo: Option<Previo>,
    /// Lo que los hechos leídos dicen que subió cada contador discreto, en total y en la observación anterior; y los contadores reales de entonces.
    esperado: [u64; 24],
    esperado_antes: [u64; 24],
    real_antes: [u64; 24],
    /// La Era según los hechos leídos (índice de `Reglas::eras`).
    era_hechos: u8,
    /// (héroe, alijo) abiertos según los hechos.
    abiertos: BTreeSet<(u64, String)>,
    /// La crónica tal como salió en los cambios de versión.
    pub(super) emitida: Vec<pb::EventoDominio>,
    /// Plaza → cuándo vuelve a abrir la puerta a los Aedas el bot que se la cerró.
    reabrir: BTreeMap<u64, Instante>,
    /// (plaza, edificio) → hasta cuándo no se vuelve a pedir tras un `obra.sin_sitio`.
    sin_sitio: BTreeMap<(u64, Tipo), Instante>,
    /// Facción → la columna en que su Rey va a la capital.
    reyes: BTreeMap<u32, u64>,
    /// Aeda residente → cuándo empezó su épica; (plaza, tecnología) de las épicas que se atascaron allí.
    epicas_desde: BTreeMap<u32, Instante>,
    descartadas: BTreeSet<(u64, TecId)>,
}

impl Tec {
    pub fn nuevo(ajustes: Ajustes) -> Self {
        Tec {
            ajustes,
            v: ViolacionesTec::default(),
            m: MedidasTec::default(),
            muestras: Vec::new(),
            buscadores: Vec::new(),
            previo: None,
            esperado: [0; 24],
            esperado_antes: [0; 24],
            real_antes: [0; 24],
            era_hechos: 0,
            abiertos: BTreeSet::new(),
            emitida: Vec::new(),
            reabrir: BTreeMap::new(),
            sin_sitio: BTreeMap::new(),
            reyes: BTreeMap::new(),
            epicas_desde: BTreeMap::new(),
            descartadas: BTreeSet::new(),
        }
    }
}

impl Sim<'_> {
    fn t_(&mut self) -> &mut Tec {
        self.tec.as_mut().expect("batch tecnológico")
    }

    pub(super) fn tec_arrancar(&mut self, t0: Instante) {
        self.tec_vigilar(t0);
        for k in 0..BUSCADORES {
            let campamento = (k % self.geo.campamentos.len().max(1)) as u32;
            let cuenta = format!("buscador-{k}");
            let r = self.segura(t0, &cuenta, "CrearHeroe", Comando::CrearHeroe { cuenta: cuenta.clone(), campamento, controlador: Controlador::Humano, nombre: None });
            let Some(&heroe) = r.creadas.first().filter(|_| r.aceptada()) else { continue };
            // Andamiaje declarado: el oro de los alijos (unas pocas unidades cada uno) no paga un recluta de campamento, así que la administración le da al nacer el de dos reclutas de la
            // tropa más barata de su campamento, para ejercitar la vía de Q4 (`reclutados.*` también en el campamento).
            let mr = self.n.estado().reglas.economia.militar.clone();
            let precio = self.n.tropas_del_campamento(campamento).unwrap_or_default().into_iter().map(|x| militar::mercenarios::precio(&mr, mr.tropas.tropa(x), mr.tropas.tropa(x).unidades, asentamiento::datos::UNO, false)).min();
            if let Some(precio) = precio {
                self.admin(t0, "DarAlmacenPersonal", Comando::DarAlmacenPersonal { heroe, recurso: Recurso::Oro, cantidad: 2 * precio });
                let m = &mut self.t_().m;
                m.oro_andamiaje += 2 * precio;
                m.precio_recluta.push(precio);
            }
            let x = self.t_();
            x.buscadores.push(Buscador { heroe, cuenta, campamento, destino: None, en_salida: 0, saltados: BTreeSet::new(), cita: 0 });
            let k = x.buscadores.len() - 1;
            self.programar(t0 + (k as i64 + 1) * 7 * MIN, Accion::Buscador { k, cita: 0 });
        }
    }

    // ───────────────────────────── Los Gobernadores ─────────────────────────────

    /// El turno de gobierno (cada hora) de una plaza: la puerta a los Aedas, la compra, las épicas y, en la capital, las obras hacia los hitos.
    pub(super) fn tec_plaza(&mut self, t: Instante, i: usize, p: u64) {
        let f = self.n.estado().asentamientos[&p].faccion;
        self.tec_puerta(t, i, p);
        self.tec_comprar(t, i, p, f);
        let obras = t % (OBRAS_CADA_H * HORA) < HORA;
        self.tec_epicas(t, i, p, f, obras);
        if obras && self.n.capital_de(f) == Some(p) {
            self.tec_obras(t, i, p, f);
        }
        if t % (6 * HORA) < HORA {
            self.tec_minas(t, i, p);
        }
    }

    /// Hacia los logros de lo extraído: con el nivel 2, la plaza marca un pueblo minero (cobre, estaño o hierro) en un nodo de su zona si a ese contador le queda un logro pendiente de una
    /// Era abierta. Uno cada 6 h; el cupo y los materiales son del juego (el Intendente lleva los materiales a la obra).
    fn tec_minas(&mut self, t: Instante, i: usize, p: u64) {
        if self.n.estado().asentamientos[&p].economia.nivel_actual() < 2 {
            return;
        }
        let r = self.n.estado().reglas.economia.tecnologia.clone();
        let e = self.n.tecnologia();
        let pendiente = |c: Contador| r.por_contador[c.indice()].iter().any(|&(_, x)| !e.logro_cumplido(x) && e.era_abierta(&r, x));
        let mina = self.n.candidatos_de_pueblo(p).into_iter().find(|&(_, tipo)| match tipo {
            Tipo::MinaCobre => pendiente(Contador::ExtraidoCobre),
            Tipo::MinaEstano => pendiente(Contador::ExtraidoEstano),
            Tipo::MinaHierro => pendiente(Contador::ExtraidoHierro),
            _ => false,
        });
        if let Some((nodo, _)) = mina {
            if self.suya(t, i, "MarcarMina", Comando::MarcarPueblo { asentamiento: p, nodo }).aceptada() {
                self.t_().m.minas += 1;
            }
        }
    }

    fn aedas_en(&self, p: u64) -> usize {
        self.n.aedas().iter().filter(|a| a.en == Some(p as u32)).count()
    }

    fn tec_puerta(&mut self, t: Instante, i: usize, p: u64) {
        let heroe = self.bots[i].heroe;
        let actual = self.n.puerta_cerrada_a(p).unwrap_or_default();
        if let Some(&hasta) = self.t_().reabrir.get(&p) {
            if t < hasta {
                return;
            }
            self.t_().reabrir.remove(&p);
            // La puerta pudo cambiarla otro (el bot político la fija cada 12 h sin los Aedas): solo se abre si sigue cerrada.
            if actual.contiene(Grupo::Aedas) {
                let cerrada_a = Grupos::de(actual.iter().filter(|&g| g != Grupo::Aedas));
                self.esperada(t, i, "AbrirPuertaAedas", Comando::FijarPuerta { heroe, asentamiento: p, cerrada_a });
            }
            return;
        }
        let dentro = self.aedas_en(p) as u64;
        if dentro == 0 || actual.contiene(Grupo::Aedas) || self.rng.por_debajo(100) >= PUERTA_AEDAS_PCT {
            return;
        }
        let cerrada_a = Grupos::de(actual.iter().chain([Grupo::Aedas]));
        if self.esperada(t, i, "CerrarPuertaAedas", Comando::FijarPuerta { heroe, asentamiento: p, cerrada_a }).aceptada() {
            let quedan = self.aedas_en(p) as u64;
            let x = self.t_();
            x.m.puertas_cerradas += 1;
            x.m.aedas_expulsados += dentro.saturating_sub(quedan);
            x.reabrir.insert(p, t + PUERTA_AEDAS_H * HORA);
        }
    }

    /// Con un Aeda parado en la plaza, se le compra la primera revelada que el almacén puede pagar.
    fn tec_comprar(&mut self, t: Instante, i: usize, p: u64, f: u32) {
        if self.aedas_en(p) == 0 {
            return;
        }
        let r = self.n.estado().reglas.economia.tecnologia.clone();
        let reveladas = self.n.tecnologia().de(&r, f).reveladas;
        let almacen = &self.n.estado().asentamientos[&p].economia.almacen;
        let pagable = reveladas.iter().find(|&x| r.orden_era(x) <= r.aedas.orden_era_maximo_venta && almacen.tiene(&[(Recurso::Oro, tecnologia::adopcion::precio_de_venta(&r, x))]));
        let Some(x) = pagable else {
            if !reveladas.is_empty() {
                self.t_().m.compra_sin_oro += 1;
            }
            return;
        };
        let heroe = self.bots[i].heroe;
        if self.esperada(t, i, "ComprarTecnologiaAeda", Comando::ComprarTecnologiaAeda { heroe, asentamiento: p, tecnologia: x }).aceptada() {
            // Comprar se salta el hito, nunca el logro (TA §6.4): el logro estaba cumplido y los Aedas la conocían.
            let e = self.n.tecnologia();
            let bien = e.logro_cumplido(x) && e.conocida_desde(&r, x).is_some_and(|d| d <= t);
            let tt = self.t_();
            tt.m.compras += 1;
            tt.v.sin_logro += (!bien) as u64;
        }
    }

    /// A cada Aeda residente sin épica se le encarga la primera posible; de vez en cuando se abandona una en curso. Con `obras`, si el capítulo en curso de una pide una obra, la plaza
    /// la levanta (o mejora el edificio que ya tiene: la mejora también cuenta).
    fn tec_epicas(&mut self, t: Instante, i: usize, p: u64, f: u32, obras: bool) {
        let heroe = self.bots[i].heroe;
        let r = self.n.estado().reglas.economia.tecnologia.clone();
        let capitulo = |e: &tecnologia::Epica| r.def(e.tec).epica.as_ref().and_then(|d| d.capitulos.get(e.capitulo as usize)).filter(|c| c.hecho == tecnologia::datos::HechoEpico::Obra).and_then(|c| c.edificio);
        let aedas: Vec<(u32, bool, Option<Tipo>)> = self.n.residentes().de_plaza(p as u32).iter().filter(|a| a.faccion == f).map(|a| (a.id, a.epica.is_some(), a.epica.as_ref().and_then(capitulo))).collect();
        for (aeda, canta, obra) in aedas {
            if canta {
                // Atascada: sin un hecho en `ATASCADA_DIAS` desde que empezó o desde el último, se abandona y no se vuelve a empezar en esta plaza; y alguna vez, al azar.
                let ultimo = self.n.residentes().get(aeda).and_then(|a| a.epica.as_ref()).map(|e| (e.tec, e.ultimo_hecho_en));
                let desde = self.t_().epicas_desde.get(&aeda).copied().unwrap_or(t).max(ultimo.and_then(|u| u.1).unwrap_or(Instante::MIN));
                let atascada = t - desde > ATASCADA_DIAS * DIA;
                if (atascada || self.rng.por_debajo(1000) < ABANDONO_POR_MIL) && self.esperada(t, i, "AbandonarEpica", Comando::AbandonarEpica { heroe, asentamiento: p, aeda }).aceptada() {
                    let x = self.t_();
                    x.m.epicas_abandonadas += 1;
                    x.epicas_desde.remove(&aeda);
                    if let (true, Some((tec, _))) = (atascada, ultimo) {
                        x.descartadas.insert((p, tec));
                        x.m.epicas_atascadas += 1;
                    }
                } else if let Some(tipo) = obra.filter(|_| obras) {
                    self.obra_de_epica(t, i, p, tipo);
                }
                continue;
            }
            // La posible que la plaza tiene más a mano (`dificultad`; a igualdad, la primera del catálogo), salvo las que ya se le atascaron aquí.
            let descartadas = &self.tec.as_ref().expect("batch tecnológico").descartadas;
            let elegida = self.n.epicas_posibles(f).iter().filter(|&x| !descartadas.contains(&(p, x))).min_by_key(|&x| (self.dificultad(t, p, &r, x), x));
            let Some(x) = elegida else { continue };
            if self.esperada(t, i, "EmpezarEpica", Comando::EmpezarEpica { heroe, asentamiento: p, aeda, tecnologia: x }).aceptada() {
                let tt = self.t_();
                tt.m.epicas_empezadas += 1;
                tt.epicas_desde.insert(aeda, t);
            }
        }
    }

    /// Lo difícil que le resulta a esta plaza cada hecho de los capítulos de la épica de `x` (hipótesis del batch para elegir): una obra que la plaza puede pedir ahora, 0; la adopción, la
    /// batalla en campo y la caravana, que llegan solas con el tiempo, 1; el ascenso (del nivel 2 al 3), la defensa, la conquista y una obra que no puede pedir, 3.
    fn dificultad(&self, t: Instante, p: u64, r: &tecnologia::Reglas, x: TecId) -> u32 {
        use tecnologia::datos::HechoEpico::*;
        let obra = |tipo: Option<Tipo>| tipo.is_some_and(|tipo| self.comando_de_obra(t, p, tipo, self.siguiente_nivel(p, tipo)).is_some());
        r.def(x).epica.as_ref().map_or(u32::MAX, |e| {
            e.capitulos
                .iter()
                .map(|c| {
                    c.cantidad
                        * match c.hecho {
                            Obra if obra(c.edificio) => 0,
                            Adopcion | Batalla | Caravana => 1,
                            Obra | Ascenso | Defensa | Conquista => 3,
                        }
                })
                .sum()
        })
    }

    /// El nivel interno que daría una obra de `tipo` en la plaza: 1 si no lo hay; si lo hay, el siguiente del más alto (la mejora también es obra).
    fn siguiente_nivel(&self, p: u64, tipo: Tipo) -> u8 {
        self.n.estado().asentamientos[&p].economia.edificios.iter().filter(|x| x.tipo == tipo).map(|x| x.nivel_interno + 1).max().unwrap_or(1)
    }

    /// La obra que pide el capítulo de una épica: el edificio nuevo o, si ya lo hay activo, su mejora (si no está ya en obra o mejorándose).
    fn obra_de_epica(&mut self, t: Instante, i: usize, p: u64, tipo: Tipo) {
        let objetivo = self.siguiente_nivel(p, tipo);
        if self.obra(t, i, p, tipo, objetivo, "ObraDeEpica") {
            self.t_().m.obras_epica += 1;
        }
    }

    /// Lleva el edificio `tipo` de la plaza hasta el nivel interno `objetivo`: lo añade si no lo hay, o mejora el que hay (activo, sin mejora en curso y por debajo del objetivo). No pide
    /// lo que el catálogo no deja (el nivel de la plaza, la tecnología adoptada, un nivel interno que no existe) ni, durante un día, lo que se rechazó por falta de sitio en la ciudad.
    /// Devuelve si la orden se aceptó; el resto de rechazos (materiales, cuadrillas, cola) son del juego.
    fn obra(&mut self, t: Instante, i: usize, p: u64, tipo: Tipo, objetivo: u8, etiqueta: &'static str) -> bool {
        let Some(c) = self.comando_de_obra(t, p, tipo, objetivo) else { return false };
        let r = self.suya(t, i, etiqueta, c);
        if matches!(r.resultado, Resultado::Rechazada { codigo, .. } if codigo == rechazos::OBRA_SIN_SITIO) {
            self.t_().sin_sitio.insert((p, tipo), t + DIA);
        }
        r.aceptada()
    }

    /// La orden que pediría [`Sim::obra`], si la puede pedir.
    fn comando_de_obra(&self, t: Instante, p: u64, tipo: Tipo, objetivo: u8) -> Option<Comando> {
        if self.tec.as_ref()?.sin_sitio.get(&(p, tipo)).is_some_and(|&hasta| t < hasta) {
            return None;
        }
        let a = &self.n.estado().asentamientos[&p];
        let (f, nivel_plaza) = (a.faccion, a.economia.nivel_actual());
        let def = self.n.estado().reglas.economia.datos.catalogo.tipo(tipo);
        let adoptada = |x: &Option<String>| x.as_deref().is_none_or(|x| self.n.tiene_tecnologia(f, x));
        let ya = a.economia.edificios.iter().filter(|x| x.tipo == tipo).max_by_key(|x| x.nivel_interno).map(|x| (x.id, x.nivel_interno, x.activo() && x.mejora.is_none()));
        match ya {
            Some((edificio, nivel, true)) if nivel < objetivo => match def.nivel(nivel + 1).filter(|_| nivel < def.max_nivel_interno()) {
                Some(n) if n.requisito_nivel_asentamiento <= nivel_plaza && adoptada(&n.tecnologia) => Some(Comando::MejorarEdificio { asentamiento: p, edificio }),
                _ => None,
            },
            Some(_) => None,
            None if def.nivel_construccion <= nivel_plaza && adoptada(&def.tecnologia) && objetivo <= def.max_nivel_interno() => Some(Comando::AnadirEdificio { asentamiento: p, tipo }),
            None => None,
        }
    }

    /// El Gobernador de la capital levanta (o mejora) el primer edificio que pide el hito de una tecnología de una Era abierta que a su Facción no le ha aparecido y que ninguna plaza
    /// suya tiene; uno por turno. Los rechazos (nivel, materiales, cupo) son del juego.
    fn tec_obras(&mut self, t: Instante, i: usize, p: u64, f: u32) {
        let r = self.n.estado().reglas.economia.tecnologia.clone();
        let e = self.n.tecnologia();
        let propias = e.de(&r, f);
        let mut pedidos: Vec<(Tipo, u8)> = Vec::new();
        for x in 0..r.catalogo.len() as TecId {
            if propias.aparecidas.contiene(x) || !e.era_abierta(&r, x) {
                continue;
            }
            for c in &r.def(x).hito {
                match *c {
                    Condicion::Edificio { tipo, nivel } => pedidos.push((tipo, nivel)),
                    Condicion::CapitalEnNivel { con, .. } => pedidos.push((con, 1)),
                    _ => {}
                }
            }
        }
        for (tipo, nivel) in pedidos {
            if self.n.plazas_de(f).any(|a| a.economia.edificios.hay_activo_con_nivel(tipo, nivel)) {
                continue;
            }
            // El primero que falta y se puede pedir; uno por turno.
            if self.obra(t, i, p, tipo, nivel, "ObraDeHito") {
                self.t_().m.obras_hito += 1;
                return;
            }
        }
    }

    // ───────────────────────────── El Rey va a su capital ─────────────────────────────

    /// El Rey (un héroe que no es ningún bot militar: los bots tienen su papel) está dentro de otra plaza de su Facción y en la capital hay algo que adoptar: sale y marcha a ella.
    pub(super) fn tec_rey_a_la_capital(&mut self, t: Instante, f: u32, rey: u64, capital: u64) {
        if self.t_().reyes.contains_key(&f) || self.bots.iter().any(|b| b.heroe == rey) {
            return;
        }
        let Some(Ubicacion::Asentamiento(q)) = self.n.heroe(rey).map(|h| h.heroe.presencia.ubicacion) else { return };
        if self.faccion_de_plaza(u64::from(q)) != Some(f) {
            return;
        }
        let cuenta = self.n.heroe(rey).map(|h| h.cuenta.clone()).unwrap_or_default();
        // De su residencia se sale al mundo; de otra plaza, como visitante.
        let c = if self.n.heroe(rey).is_some_and(|h| h.heroe.residencia == Some(Residencia::Asentamiento(q))) {
            Comando::SalirAlMundo { asentamiento: u64::from(q), heroe: rey, escuadras: vec![], carga: vec![] }
        } else {
            Comando::SalirDeAsentamiento { asentamiento: u64::from(q), heroe: rey }
        };
        self.segura(t, &cuenta, "ReySaleHaciaLaCapital", c);
        let Some(c) = self.n.heroe(rey).and_then(|h| h.columna) else { return };
        let destino = self.n.estado().asentamientos[&capital].posicion;
        if !self.orden(t, &cuenta, "ReyMarchaALaCapital", Comando::MarcharA { ejercito: c, destino }).aceptada() {
            return;
        }
        let llega = self.n.ejercito(c).and_then(|e| e.plan.fin()).unwrap_or(t).max(t + 1);
        let x = self.t_();
        x.reyes.insert(f, rey);
        x.m.reyes_salen += 1;
        self.programar(llega, Accion::Rey { faccion: f });
    }

    /// El Rey llega a la puerta de la capital (la de ahora): entra; si la capital cambió por el camino, sigue hacia la nueva.
    pub(super) fn tec_rey_llega(&mut self, t: Instante, f: u32) {
        let Some(rey) = self.t_().reyes.remove(&f) else { return };
        let (Some(capital), Some(c)) = (self.n.capital_de(f), self.n.heroe(rey).and_then(|h| h.columna)) else { return };
        let Some(e) = self.n.ejercito(c) else { return };
        if let Some(fin) = e.plan.fin().filter(|&x| x > t) {
            self.t_().reyes.insert(f, rey);
            return self.programar(fin, Accion::Rey { faccion: f });
        }
        let destino = self.n.estado().asentamientos[&capital].posicion;
        let cuenta = self.n.heroe(rey).map(|h| h.cuenta.clone()).unwrap_or_default();
        let radio = self.n.estado().reglas.economia.militar.movimiento.radio_puerta;
        if e.posicion_en(t).dist(destino) > radio {
            if self.orden(t, &cuenta, "ReyMarchaALaCapital", Comando::MarcharA { ejercito: c, destino }).aceptada() {
                let llega = self.n.ejercito(c).and_then(|e| e.plan.fin()).unwrap_or(t).max(t + 1);
                self.t_().reyes.insert(f, rey);
                self.programar(llega, Accion::Rey { faccion: f });
            }
            return;
        }
        if self.orden(t, &cuenta, "ReyEntraEnLaCapital", Comando::EntrarEnAsentamiento { asentamiento: capital, heroe: rey }).aceptada() {
            self.t_().m.reyes_llegan += 1;
            self.adoptar(t, capital);
        }
    }

    // ───────────────────────────── Los buscadores de alijos ─────────────────────────────

    fn cita_buscador(&mut self, t: Instante, k: usize) {
        let x = &mut self.t_().buscadores[k];
        x.cita += 1;
        let cita = x.cita;
        self.programar(t, Accion::Buscador { k, cita });
    }

    pub(super) fn tec_buscador(&mut self, t: Instante, k: usize, cita: u32) {
        if self.t_().buscadores[k].cita != cita {
            return;
        }
        let heroe = self.t_().buscadores[k].heroe;
        match self.n.heroe(heroe).map(|h| h.heroe.presencia.ubicacion) {
            Some(Ubicacion::Mercenarios(_)) => self.buscador_en_campamento(t, k),
            Some(Ubicacion::Columna(c)) => self.buscador_en_columna(t, k, c),
            _ => self.cita_buscador(t + HORA, k),
        }
    }

    /// El alijo más cercano a `desde` que el buscador no ha abierto ni descartado, a `RADIO_ALIJOS` como mucho.
    fn alijo_cercano(&self, k: usize, desde: Punto) -> Option<u32> {
        let b = &self.tec.as_ref()?.buscadores[k];
        let abiertos = &self.n.heroe(b.heroe)?.alijos_abiertos;
        (0..self.n.alijos_del_artefacto())
            .filter(|i| !abiertos.contains(i) && !b.saltados.contains(i))
            .filter_map(|i| Some((self.n.alijo(i)?.posicion.dist(desde), i)))
            .filter(|&(d, _)| d <= RADIO_ALIJOS)
            .min()
            .map(|(_, i)| i)
    }

    fn buscador_en_campamento(&mut self, t: Instante, k: usize) {
        let b = self.t_().buscadores[k].clone();
        let heroe = b.heroe;
        // Lo que trae en el carro, a su almacén personal; con ese oro recluta en su campamento (cuenta para los logros `reclutados.*`, Q4).
        if self.n.heroe(heroe).is_some_and(|h| !h.carro.is_empty()) {
            self.orden(t, &b.cuenta, "VaciarCarro", Comando::VaciarCarro { heroe });
        }
        if self.n.heroe(heroe).is_some_and(|h| h.almacen_personal.get(&Recurso::Oro).is_some_and(|&o| o > 0)) {
            // La primera que el campamento ofrece; sin el oro, espera a traer más.
            for tropa in self.n.tropas_del_campamento(b.campamento).unwrap_or_default() {
                let r = self.orden(t, &b.cuenta, "ReclutarEnCampamento", Comando::ReclutarEnCampamento { heroe, tropa, pagar_con: PagarCon::AlmacenPersonal });
                if r.aceptada() {
                    self.t_().m.reclutas_campamento += 1;
                }
                if !matches!(r.resultado, Resultado::Rechazada { codigo, .. } if codigo != rechazos::MERCENARIOS_SIN_ORO) {
                    break;
                }
            }
        }
        let Some(puerta) = self.n.puerta_de_campamento(b.campamento) else { return };
        let Some(alijo) = self.alijo_cercano(k, puerta) else { return self.cita_buscador(t + DIA, k) };
        let r = self.segura(t, &b.cuenta, "SalirDelCampamento", Comando::SalirDelCampamento { campamento: b.campamento, heroe, escuadras: vec![], carga: vec![], politica: Politica::Rechazar, caravana_fundacion: None });
        let Some(&c) = r.creadas.first().filter(|_| r.aceptada()) else { return self.cita_buscador(t + DESCANSO_BUSCADOR_H * HORA, k) };
        self.t_().buscadores[k].en_salida = 0;
        self.buscador_ir(t, k, c, Some(alijo));
    }

    /// Marcha hacia el alijo (o, con `None`, a la puerta de su campamento). Si no hay camino al alijo, lo descarta y vuelve.
    fn buscador_ir(&mut self, t: Instante, k: usize, c: u64, destino: Option<u32>) {
        let b = self.t_().buscadores[k].clone();
        let Some(punto) = (match destino {
            Some(a) => self.n.alijo(a).map(|x| x.posicion),
            None => self.n.puerta_de_campamento(b.campamento),
        }) else {
            return;
        };
        let r = self.orden(t, &b.cuenta, "MarcharA", Comando::MarcharA { ejercito: c, destino: punto });
        let llega = match r.resultado {
            Resultado::Aceptada => self.n.ejercito(c).and_then(|e| e.plan.fin()).unwrap_or(t),
            Resultado::SinCambios => t,
            _ => {
                return match destino {
                    Some(a) => {
                        self.t_().buscadores[k].saltados.insert(a);
                        self.t_().m.alijos_inalcanzables += 1;
                        self.buscador_ir(t, k, c, None)
                    }
                    None => self.cita_buscador(t + DIA, k),
                };
            }
        };
        self.t_().buscadores[k].destino = destino;
        self.cita_buscador(llega.max(t + 1), k);
    }

    fn buscador_en_columna(&mut self, t: Instante, k: usize, c: u64) {
        let b = self.t_().buscadores[k].clone();
        if let Some(fin) = self.n.ejercito(c).and_then(|e| e.plan.fin()).filter(|&f| f > t) {
            return self.cita_buscador(fin, k);
        }
        let aqui = self.n.ejercito(c).map_or_else(Punto::default, |e| e.posicion_en(t));
        let Some(a) = b.destino else {
            // En la puerta de su campamento: entra y descansa.
            self.segura(t, &b.cuenta, "EntrarEnCampamento", Comando::EntrarEnCampamento { campamento: b.campamento, heroe: b.heroe });
            return self.cita_buscador(t + DESCANSO_BUSCADOR_H * HORA, k);
        };
        let radio = self.n.estado().reglas.economia.militar.movimiento.radio_puerta;
        if self.n.alijo(a).is_some_and(|x| x.posicion.dist(aqui) <= radio) {
            let r = self.segura(t, &b.cuenta, "AbrirAlijo", Comando::AbrirAlijo { heroe: b.heroe, alijo: a });
            if r.aceptada() && self.rng.por_debajo(100) < REINTENTO_PCT {
                // Un alijo se abre una vez por héroe: el segundo intento tiene que rechazarse con `alijo.ya_abierto` y sin rastro.
                let v = self.n.version();
                let r = self.orden(t, &b.cuenta, "AbrirAlijoOtraVez", Comando::AbrirAlijo { heroe: b.heroe, alijo: a });
                let bien = matches!(r.resultado, Resultado::Rechazada { codigo, .. } if codigo == rechazos::ALIJO_YA_ABIERTO) && self.n.version() == v;
                let x = self.t_();
                x.m.reintentos += 1;
                x.v.alijo += (!bien) as u64;
            }
        } else {
            self.t_().buscadores[k].saltados.insert(a);
            self.t_().m.alijos_inalcanzables += 1;
        }
        let sigue = {
            let x = &mut self.t_().buscadores[k];
            x.en_salida += 1;
            x.en_salida < ALIJOS_POR_SALIDA
        };
        let otro = if sigue { self.alijo_cercano(k, aqui) } else { None };
        self.buscador_ir(t, k, c, otro);
    }

    // ───────────────────────────── Los invariantes ─────────────────────────────

    /// Tras cada orden (y al arrancar): los invariantes del estado de tecnología, de los itinerantes y de las épicas.
    pub(super) fn tec_vigilar(&mut self, t: Instante) {
        let (n, Some(tec)) = (&self.n, self.tec.as_mut()) else { return };
        let r = &n.estado().reglas.economia.tecnologia;
        let e = n.tecnologia();
        let v = &mut tec.v;
        if let Some(a) = &tec.previo {
            for c in Contador::TODOS {
                v.contador += (e.contador(c) < a.contadores[c.indice()]) as u64;
            }
            for (x, d) in r.catalogo.iter().enumerate() {
                let Some(l) = d.logro else { continue };
                let (valor, antes) = (e.contador(l.contador), a.contadores[l.contador.indice()]);
                v.logro += match (a.logros[x], e.logros[x]) {
                    (Some(x0), Some(x1)) => x0 != x1,
                    (Some(_), None) => true,
                    (None, Some(en)) => en < a.t || en > t || valor < l.umbral || antes >= l.umbral,
                    (None, None) => valor >= l.umbral,
                } as u64;
            }
            if e.era != a.era {
                let era = &r.eras[a.era as usize];
                let bien = e.era == a.era + 1 && e.era_desde >= a.era_desde + era.minimo && e.era_desde <= a.era_desde + era.plazo && (a.t..=t).contains(&e.era_desde);
                v.era += (!bien) as u64;
            }
            v.itinerantes += (n.aedas().len() < a.itinerantes) as u64;
        }
        // La vigente no sigue pasado su plazo, ni pasados su mínimo y su último logro si ya los tiene todos (la última Era no tiene plazo).
        if (e.era as usize) + 1 < r.eras.len() {
            let era = &r.eras[e.era as usize];
            v.era += (t > e.era_desde + era.plazo) as u64;
            let mut ultimo = Some(e.era_desde);
            for (x, _) in r.catalogo.iter().enumerate().filter(|(_, d)| d.era == e.era && d.logro.is_some()) {
                ultimo = ultimo.zip(e.logros[x]).map(|(u, l)| u.max(l));
            }
            if let Some(u) = ultimo {
                v.era += (t > u.max(e.era_desde + era.minimo)) as u64;
            }
        }
        for tf in e.por_faccion.values() {
            v.adoptadas += (tf.adoptadas.iter().any(|x| !tf.aparecidas.contiene(x)) || tf.reveladas.iter().any(|x| tf.aparecidas.contiene(x))) as u64;
            for x in tf.aparecidas.iter().filter(|&x| !r.def(x).de_arranque) {
                v.sin_logro += (!e.logro_cumplido(x)) as u64;
                v.era_cerrada += (!e.era_abierta(r, x)) as u64;
            }
        }
        let pr = &n.estado().reglas.economia.politica;
        for a in n.aedas() {
            if let Some(p) = a.en {
                v.puerta += (!n.estado().asentamientos.get(&u64::from(p)).is_some_and(|x| x.puerta.admite_aedas(pr))) as u64;
            }
        }
        let mut cantadas = BTreeSet::new();
        for a in &n.residentes().aedas {
            if let Some(ep) = &a.epica {
                v.epica += (!cantadas.insert((a.faccion, ep.tec)) || e.de(r, a.faccion).aparecidas.contiene(ep.tec)) as u64;
            }
        }
        tec.previo = Some(Previo { t, contadores: e.contadores, logros: e.logros.clone(), era: e.era, era_desde: e.era_desde, itinerantes: n.aedas().len() });
    }

    /// Cada hecho leído: lo que dice de los contadores discretos, la Era, lo que aparece y los alijos.
    pub(super) fn tec_hecho(&mut self, codigo: &str, t: Instante, datos: &[(&'static str, i64)], valores: &[(&'static str, Dato)]) {
        let (n, Some(x)) = (&self.n, self.tec.as_mut()) else { return };
        let dato = |k: &str| datos.iter().find(|(c, _)| *c == k).map(|&(_, v)| v);
        let texto = |k: &str| valores.iter().find_map(|(c, d)| match d {
            Dato::Texto(s) if *c == k => Some(s.clone()),
            _ => None,
        });
        let r = &n.estado().reglas.economia.tecnologia;
        let d = match codigo {
            "combate.resuelto" => Delta::uno(Contador::BatallasLibradas),
            "combate.encuentro" => Delta::uno(Contador::BatallasCampoAbierto),
            "combate.caravana_interceptada" | "bandidos.caravana_interceptada" => Delta::uno(Contador::CaravanasDestruidasOCapturadas),
            "combate.campamento_destruido" => Delta::uno(Contador::BandidosCampamentosDestruidos),
            // Solo el hecho del asedio, nunca su `batalla.aplicada` (D18). `conResidentes` no viaja en el hecho: se acota abajo.
            "combate.asedio_conquista" | "combate.asedio_resistido" => cont::asedio(codigo == "combate.asedio_conquista", dato("poderAtacante").is_some(), false, dato("murallaCompleta") == Some(1)),
            "comercio.caravana_animal_comprado" => Delta::uno(Contador::AnimalesComprados),
            "tropas.reclutadas" | "mercenarios.reclutado" => match dato("tropa").and_then(|i| TROPAS.get(i as usize)) {
                Some(&tropa) => {
                    let soldados = dato(if codigo == "tropas.reclutadas" { "reclutados" } else { "cantidad" }).unwrap_or(0).max(0) as u64;
                    if codigo == "mercenarios.reclutado" {
                        x.m.reclutados_campamento += soldados;
                    }
                    cont::reclutamiento(tropa.nombre(), n.estado().reglas.economia.militar.tropas.tropa(tropa).edificio, soldados, dato("nueva") == Some(1))
                }
                None => Delta::default(),
            },
            _ => Delta::default(),
        };
        for (c, k) in d.iter() {
            x.esperado[c.indice()] += k;
        }
        match codigo {
            "era.comienza" => {
                let orden = dato("orden").unwrap_or(0) as u32;
                if let Some(k) = r.eras.iter().position(|e| e.orden == orden) {
                    x.era_hechos = k as u8;
                }
                x.m.eras.push((t, orden, dato("porPlazo") == Some(1)));
            }
            "tecnologia.aparece" | "aedas.venta" | "aedas.epica_tecnologia" => {
                if let Some(tt) = texto("tecnologia").and_then(|s| r.buscar(&s)) {
                    x.m.aparecidas_por[match codigo {
                        "tecnologia.aparece" => 0,
                        "aedas.venta" => 1,
                        _ => 2,
                    }] += 1;
                    x.v.era_cerrada += (r.orden_era(tt) > r.eras[x.era_hechos as usize].orden) as u64;
                    x.v.sin_logro += (!n.tecnologia().logros[tt as usize].is_some_and(|en| en <= t)) as u64;
                }
                if codigo == "aedas.venta" {
                    x.m.compras_oro += dato("precio").unwrap_or(0);
                }
            }
            "alijo.abierto" => {
                let par = (dato("heroe").unwrap_or(0) as u64, texto("alijo").unwrap_or_default());
                x.v.alijo += (!x.abiertos.insert(par) || dato("cargado") > dato("oro")) as u64;
                x.m.alijos_abiertos += 1;
                x.m.alijos_oro += dato("oro").unwrap_or(0);
                x.m.alijos_cargado += dato("cargado").unwrap_or(0);
            }
            "tecnologia.adoptada" => x.m.adopciones += 1,
            "aedas.revela" => x.m.revelaciones += 1,
            "aedas.canta_descubrimiento" => x.m.cantos += 1,
            "aedas.residente_llega" => x.m.residentes_llegan += 1,
            "aedas.residente_se_va" => x.m.residentes_se_van += 1,
            "aedas.epica_capitulo" => x.m.capitulos += 1,
            "aedas.epica_cumplida" => x.m.epicas_cumplidas += 1,
            "aedas.epica_abandonada" => *x.m.epicas_perdidas.entry(texto("motivo").unwrap_or_default()).or_insert(0) += 1,
            _ => {}
        }
    }

    /// Cada observación (tras leer los hechos): los contadores discretos subieron exactamente lo que dicen sus hechos desde la anterior; y la muestra del día.
    pub(super) fn tec_observar(&mut self, t: Instante) {
        let (n, Some(x)) = (&self.n, self.tec.as_mut()) else { return };
        let e = n.tecnologia();
        for c in DE_HECHOS {
            let i = c.indice();
            x.v.doble += (e.contadores[i] - x.real_antes[i] != x.esperado[i] - x.esperado_antes[i]) as u64;
        }
        let tramo = |c: Contador| e.contadores[c.indice()] - x.real_antes[c.indice()];
        x.v.doble += (tramo(Contador::BatallasConHoplitas) > tramo(Contador::BatallasLibradas)) as u64;
        x.v.doble += (tramo(Contador::AsediosResistidosConResidentes) > tramo(Contador::AsediosResistidosEnCombate)) as u64;
        (x.real_antes, x.esperado_antes) = (e.contadores, x.esperado);
        if t % DIA < OBSERVAR_MIN * MIN || t >= self.fin {
            let r = &n.estado().reglas.economia.tecnologia;
            let suma = |f: &dyn Fn(&tecnologia::TecnologiasFaccion) -> usize| n.facciones().map(|g| f(&e.de(r, g.id))).sum::<usize>();
            x.muestras.push(MuestraTec {
                dia: (t / DIA) as u32,
                era: e.era,
                logros: e.logros.iter().flatten().count(),
                aparecidas: suma(&|tf| tf.aparecidas.iter().filter(|&y| !r.def(y).de_arranque).count()),
                adoptadas: suma(&|tf| tf.adoptadas.iter().filter(|&y| !r.def(y).de_arranque).count()),
                reveladas: suma(&|tf| tf.reveladas.len()),
                itinerantes: n.aedas().len(),
                residentes: n.residentes().aedas.len(),
                epicas: n.residentes().aedas.iter().filter(|a| a.epica.is_some()).count(),
            });
        }
    }

    /// Al final: la crónica guardada en el diario de la partida, leída entera por páginas, es exactamente la emitida.
    pub(super) fn tec_final(&mut self) {
        let mut guardada: Vec<pb::EventoDominio> = Vec::new();
        let mut desde = 0;
        loop {
            let (tx, rx) = std::sync::mpsc::channel();
            self.n.pedir_cronica(pb::PedirCronica { peticion_id: "batch".into(), desde_version: Some(desde), hasta_version: None, limite: 1_000 }, Box::new(move |c| drop(tx.send(c))));
            let Ok(p) = rx.recv() else { break };
            guardada.extend(p.entradas.iter().cloned());
            match p.entradas.last() {
                Some(u) if p.mas_recientes => desde = u.version + 1,
                _ => break,
            }
        }
        let x = self.t_();
        let distintas = guardada.iter().zip(&x.emitida).filter(|(a, b)| a != b).count() + guardada.len().abs_diff(x.emitida.len());
        x.v.cronica += distintas as u64;
        x.m.cronica = (x.emitida.len(), guardada.len());
    }

    /// La foto ida y vuelta del batch tecnológico: en el **mismo** directorio de la partida (así la crónica sigue en su diario), se apaga, se hace la foto y se abre otro núcleo desde
    /// ella; el hash ha de ser el mismo y la crónica no pierde ni repite nada.
    pub(super) fn tec_ida_y_vuelta(&mut self) {
        let antes = self.n.hash_estado();
        let Some(dir) = self.dir_foto.clone() else { return };
        let apagado = self.n.apagar();
        self.n.foto_ahora();
        self.n.esperar_fotos();
        let rehecho = apagado.and_then(|()| Nucleo::con_persistencia(self.arranque(), &dir, config()));
        let Ok(n) = rehecho else {
            self.militares.foto += 1;
            return;
        };
        let despues = n.hash_estado();
        self.militares.foto += (antes != despues) as u64;
        self.foto = Some((antes, despues));
        self.visto = n.version();
        self.n = n;
    }
}
