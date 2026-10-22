//! Salir al mundo y entrar en un lugar (Presencia_Y_Convocatoria §2-§5, Movimiento_Ejercitos §3 y §11): `salirAlMundo`, `movilizarEjercito`,
//! `salirDelCampamento`, `entrarEnAsentamiento`, `salirDeAsentamiento`, `guarnecer`, `entrarEnCampamento`, y las dos de Entrar con todo: `fijarAcceso` y
//! `vaciarCarro`. Una salida es siempre la misma
//! operación en dos tiempos: [`PlanDeSalida`] comprueba **todo** sin tocar nada (tropa, Liderazgo, carro, reserva de trigo, víveres) y
//! `aplicar_salida` la ejecuta; las convocatorias simulan la salida de cada integrante con el primer tiempo y parten con el segundo, de modo
//! que «saldría con lo que eligió» y «sale» no pueden discrepar (Presencia §9.2).
//!
//! # Se entra con todo y se sale con todo (decidido el 2026-10-22)
//! Quien entra en un lugar lleva consigo todo lo suyo —tropa, carro, víveres y **todas** sus caravanas adjuntas, sean del origen que sean— y al salir se lo
//! lleva. No hay «columna aparcada a la puerta»: a la puerta se está fuera, en el mapa, y no defiende; **dentro** es haber hecho la interacción de entrar.
//! - **De visita** (otra plaza, o un campamento que no es su residencia) todo queda en su columna, que sigue existiendo pero ya no está en el mapa (ni se la ve,
//!   ni se la persigue, ni se la ataca): la tropa come de **sus** víveres como un ejército acampado y la plaza anfitriona no gasta trigo por ella. Al salir la
//!   columna vuelve al mapa tal cual.
//! - **En su residencia** la tropa vuelve al campamento y los víveres al héroe (`Heroe::viveres`), pero el carro **se queda con él** ([`HeroeNucleo::carro`]):
//!   entrar no lo vacía. `vaciarCarro` lo pasa al almacén personal hasta donde quepa (lo que no cabe se queda en el carro) y al salir se lo lleva. Solo si
//!   entra con caravanas adjuntas su columna sigue existiendo (sin tropa ni carro) para llevarlas.
//!
//! Los víveres son del héroe (Movimiento_Ejercitos §6.1). Dentro de su residencia los lleva él (`Heroe::viveres`); en una columna son un único montón de
//! la columna (`Columna::viveres`, capacidad de la suma de sus héroes).
//!
//! # Quién entra
//! El dueño de la plaza decide por relación quién puede entrar (`fijarAcceso`, [`Acceso`]): la misma Facción y los residentes siempre; los aliados y vasallos,
//! los neutrales y los enemigos según lo que fije (por defecto, hasta el sistema político, todos menos los enemigos). Los pueblos extractores y las avanzadas
//! heredan el acceso de su plaza madre. Un campamento de mercenarios no es de nadie y está abierto a todos.
//!
//! # Lo provisional
//! La puerta completa de una plaza (`fijarPuerta` por grupos, vetos individuales) es del sistema político (5). Cerrar la puerta (`fijarAcceso`) saca al mapa, al instante y con
//! todo lo suyo, a quien ya está dentro y ya no puede entrar (en la plaza y en los pueblos y avanzadas que heredan su acceso: [`dentro`](super::dentro)). Entrar en un pueblo o en
//! una avanzada también vive en `dentro`. `Fundar` todavía no deshace la columna dentro de la plaza nueva.

use super::*;
use crate::estado_militar::Acceso;
use crate::nucleo::mercado::RADIO_PUERTA;
use crate::nucleo::mercado::CAPACIDAD_CARRO;
use asentamiento::almacen::Almacen;
use asentamiento::Recurso;
use comercio::{total, Carga as Mercancia};
use militar::columna::{Columna, Politica, TipoColumna};
use militar::convocatoria::Lugar;
use militar::escuadra::Contenedor;
use militar::heroe::Residencia;
use militar::liderazgo;
use militar::presencia::Ubicacion;
use militar::racion;
use militar::reclutamiento;

/// Lo que una salida lleva y gasta, comprobado antes de tocar nada.
#[derive(Clone, Debug)]
pub(super) struct PlanDeSalida {
    pub heroe: u64,
    pub lugar: Lugar,
    /// Las escuadras que saca, en el orden en que quiere que combatan.
    pub escuadras: Vec<u64>,
    /// Lo que carga en el carro: del almacén de la plaza o del personal del héroe.
    pub carga: Mercancia,
    /// Trigo que sale del almacén de la plaza a sus víveres (`conViveresLlenos`).
    pub trigo: Mil,
    /// La ración gratis del campamento: llena sus víveres y estampa `racion_en`.
    pub racion_gratis: bool,
    /// La columna con la que sale: la que trajo dentro de un lugar ajeno, o la que conserva en su residencia para llevar sus caravanas. Sin ella, nace una.
    pub retoma: Option<u64>,
    /// Liderazgo que ocupan las escuadras que saca (para el hecho).
    pub liderazgo: i64,
}

/// Lo que las salidas simuladas de antes ya habrían gastado en el mismo lugar (una convocatoria simula a todos sus integrantes en orden).
#[derive(Default)]
pub(super) struct Reservas {
    /// El almacén de la plaza tal como lo dejan las salidas anteriores.
    almacen: Option<Almacen>,
    /// Soldados que ya habrían salido del campamento de la plaza.
    soldados: i64,
}

/// Cómo termina una salida: columna nueva o suma a la del Líder (partir una convocatoria).
pub(super) enum Destino {
    Columna { tipo: TipoColumna, politica: Politica },
    Unirse(u64),
}

impl Nucleo {
    /// Dónde está la puerta de un lugar.
    pub(super) fn puerta_de(&self, lugar: Lugar) -> Option<Punto> {
        match lugar {
            Lugar::Asentamiento(a) => self.estado.asentamientos.get(&(a as u64)).map(|a| a.posicion),
            Lugar::Campamento(c) => self.puerta_de_campamento(c),
        }
    }

    // ───────────────────────────── Salir ─────────────────────────────

    /// Comprueba una salida entera sin tocar el estado (Presencia §9.2): `elegidas` son las escuadras del campamento que saca, en orden, y
    /// `carga` lo que carga en el carro. Quien reside en el lugar elige (y se lleva el carro que trajo); quien está de visita sale con todo lo que entró, su
    /// columna (su elección se ignora). `reservas` acumula lo que gastan las salidas simuladas antes de esta en el mismo lugar.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn validar_salida(&mut self, t: Instante, heroe: u64, lugar: Lugar, elegidas: &[u64], carga: &[(Recurso, Mil)], exigir_escuadras: bool, reservas: &mut Reservas) -> Result<PlanDeSalida, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let mr = self.estado.reglas.economia.militar.clone();
        let hn = self.heroe(heroe).expect("el héroe existe");
        let (reside, presente, no_esta) = match lugar {
            Lugar::Asentamiento(a) => (hn.heroe.residencia == Some(Residencia::Asentamiento(a)), hn.heroe.presencia.ubicacion == Ubicacion::Asentamiento(a), rechazos::HEROE_NO_ESTA_EN_LA_PLAZA),
            Lugar::Campamento(c) => (hn.heroe.residencia == Some(Residencia::Campamento(c)), hn.heroe.presencia.ubicacion == Ubicacion::Mercenarios(c), rechazos::HEROE_NO_ESTA_EN_EL_CAMPAMENTO),
        };
        // Quien ya va fuera, en una plaza, no sale otra vez.
        let columna = hn.columna;
        if matches!(lugar, Lugar::Asentamiento(_)) && reside && columna.is_some() && matches!(hn.heroe.presencia.ubicacion, Ubicacion::Columna(_)) {
            return Err(rechazo(rechazos::HEROE_YA_EN_COLUMNA, "ya estás fuera: no se puede salir dos veces".into()));
        }
        if !presente {
            return Err(rechazo(no_esta, format!("el héroe no está dentro de {lugar:?}")));
        }
        if !reside {
            // De visita se sale con todo lo que se entró: la columna tal cual (su elección se ignora). Quien no trajo ninguna sale solo, con sus víveres; no puede sacar
            // tropa ni carro que no tiene aquí (su campamento y su almacén personal están en su residencia).
            if columna.is_none() && (!elegidas.is_empty() || carga.iter().any(|(_, n)| *n != 0)) {
                return Err(rechazo(rechazos::SALIDA_SOLO_DE_TU_RESIDENCIA, "no tienes tropa ni carro en este lugar: están en tu residencia".into()));
            }
            return Ok(PlanDeSalida { heroe, lugar, escuadras: vec![], carga: Mercancia::new(), trigo: 0, racion_gratis: false, retoma: columna, liderazgo: 0 });
        }
        // Los víveres que ya lleva: los suyos y, si conserva su columna, los que quedaron en ella.
        let propios = hn.heroe.viveres + columna.and_then(|c| self.estado.ejercitos.get(&c)).map_or(0, |c| c.columna.viveres.valor_en(t));

        // Las escuadras: existen, son suyas, están en su campamento, con soldados, fuera de la guarnición y sin candado de batalla.
        let mut vistas = std::collections::BTreeSet::new();
        let mut soldados = 0;
        for &id in elegidas {
            let e = hn.heroe.escuadra(id).ok_or_else(|| rechazo(rechazos::ESCUADRA_AJENA, format!("la escuadra {id} no es tuya")))?;
            if !vistas.insert(id) {
                return Err(rechazo(rechazos::ESCUADRA_REPETIDA, format!("la escuadra {id} se repite")));
            }
            if e.contenedor != Contenedor::Campamento {
                return Err(rechazo(rechazos::ESCUADRA_FUERA_DEL_CAMPAMENTO, format!("la escuadra {id} no está en tu campamento")));
            }
            if e.cantidad <= 0 {
                return Err(rechazo(rechazos::ESCUADRA_ANIQUILADA, format!("la escuadra {id} está aniquilada")));
            }
            if e.en_guarnicion {
                return Err(rechazo(rechazos::ESCUADRA_EN_LA_GUARNICION, format!("la escuadra {id} está en la guarnición")));
            }
            if e.reserva_batalla.is_some() {
                return Err(rechazo(rechazos::ESCUADRA_EN_BATALLA, format!("la escuadra {id} está reservada para una batalla")));
            }
            soldados += e.cantidad;
        }
        if elegidas.is_empty() && exigir_escuadras {
            return Err(rechazo(rechazos::SALIDA_SIN_ESCUADRAS, "hay que llevarse al menos una escuadra".into()));
        }
        // Liderazgo: lo que ya tiene fuera del campamento (en una columna o de escolta) más lo que saca, contra el suyo.
        let fuera = hn.heroe.escuadras.iter().filter(|e| e.contenedor != Contenedor::Campamento && !vistas.contains(&e.id));
        let nuevas = hn.heroe.escuadras.iter().filter(|e| vistas.contains(&e.id));
        if !liderazgo::puede_llevar(&mr, hn.heroe.liderazgo_base, fuera.chain(nuevas)) {
            return Err(rechazo(rechazos::LIDERAZGO_SUPERADO, "las escuadras pasan del Liderazgo del héroe".into()));
        }
        let liderazgo = liderazgo::comprometido(&mr, hn.heroe.escuadras.iter().filter(|e| vistas.contains(&e.id)));

        // La carga: no negativa, que quepa en el carro y que haya de sobra en el almacén (el trigo, sin bajar de la reserva).
        let pedido: Mercancia = carga.iter().filter(|(_, n)| *n != 0).fold(Mercancia::new(), |mut m, &(r, n)| {
            *m.entry(r).or_insert(0) += n;
            m
        });
        if pedido.values().any(|&n| n < 0) {
            return Err(rechazo(rechazos::CARGA_NEGATIVA, "no se puede cargar una cantidad negativa".into()));
        }
        // El carro nunca lleva más de su capacidad: cuenta el que trajo a su residencia (y, si conserva su columna, lo que ella lleve).
        let ocupado = total(&hn.carro) + columna.and_then(|c| self.estado.ejercitos.get(&c)).map_or(0, |c| total(&c.columna.carga));
        let hueco = (CAPACIDAD_CARRO - ocupado).max(0);
        if total(&pedido) > hueco {
            return Err(rechazo(rechazos::CARRO_LLENO, format!("el carro admite {hueco} y se piden {}", total(&pedido))));
        }
        match lugar {
            Lugar::Asentamiento(a) => {
                let a = a as u64;
                let consumo = self.con_exterior(a, t, |eco, x| eco.resumen_de_reclutamiento(x.reglas, x).consumo_poblacion).ok_or_else(|| rechazo(rechazos::SIN_MUNDO, "el núcleo no tiene mundo cargado".into()))?;
                let restantes = self.soldados_del_campamento(a) - reservas.soldados - soldados;
                let reserva = reclutamiento::reserva_de_trigo(&mr, consumo, restantes);
                let almacen = reservas.almacen.get_or_insert_with(|| self.estado.asentamientos[&a].economia.almacen.clone());
                for (&r, &n) in &pedido {
                    let de_sobra = almacen.cantidad(r) - if r == Recurso::Trigo { reserva } else { 0 };
                    if n > de_sobra {
                        return Err(rechazo(rechazos::ALMACEN_SIN_EXISTENCIAS, format!("el almacén no tiene {n} de {} de sobra", r.nombre())));
                    }
                }
                pedido.iter().for_each(|(&r, &n)| {
                    almacen.quitar(r, n);
                });
                // Los víveres se llenan después de la carga y con la tropa ya fuera; si la plaza iba justa se sale con menos, no se bloquea.
                let trigo = (mr.logistica.capacidad_viveres - propios).max(0).min((almacen.cantidad(Recurso::Trigo) - reserva).max(0));
                almacen.quitar(Recurso::Trigo, trigo);
                reservas.soldados += soldados;
                Ok(PlanDeSalida { heroe, lugar, escuadras: elegidas.to_vec(), carga: pedido, trigo, racion_gratis: false, retoma: columna, liderazgo })
            }
            Lugar::Campamento(_) => {
                for (&r, &n) in &pedido {
                    if n > comercio::de(&hn.almacen_personal, r) {
                        return Err(rechazo(rechazos::ALMACEN_SIN_EXISTENCIAS, format!("no hay {n} de {} en el almacén personal", r.nombre())));
                    }
                }
                let racion_gratis = hn.heroe.racion_en.is_none_or(|r| t - r >= mr.mercenarios.racion_cada);
                Ok(PlanDeSalida { heroe, lugar, escuadras: elegidas.to_vec(), carga: pedido, trigo: 0, racion_gratis, retoma: columna, liderazgo })
            }
        }
    }

    /// Ejecuta una salida ya comprobada y devuelve la columna en la que va el héroe. Con `Destino::Unirse` el héroe se suma al ejército dado
    /// (los integrantes de una convocatoria que parte) con lo que saca; si no, nace una columna (o se sale con la que trajo).
    pub(super) fn aplicar_salida(&mut self, t: Instante, plan: &PlanDeSalida, destino: Destino) -> u64 {
        let mr = self.estado.reglas.economia.militar.clone();
        let PlanDeSalida { heroe, lugar, escuadras, carga, trigo, racion_gratis, retoma, .. } = plan;
        // 1. Lo que sale del almacén.
        match lugar {
            Lugar::Asentamiento(a) => {
                let almacen = &mut self.estado.asentamientos.get_mut(&(*a as u64)).expect("existe").economia.almacen;
                carga.iter().for_each(|(&r, &n)| {
                    almacen.quitar(r, n);
                });
                almacen.quitar(Recurso::Trigo, *trigo);
                self.tr.tocar(Clase::Asentamiento, *a as u64);
            }
            Lugar::Campamento(_) => {
                let personal = &mut self.heroe_mut(*heroe).almacen_personal;
                carga.iter().for_each(|(&r, &n)| *personal.entry(r).or_insert(0) -= n);
                personal.retain(|_, n| *n > 0);
            }
        }
        // 2. Los víveres que lleva: los suyos, más lo que llenó. La ración gratis los lleva al tope. Y el carro que trajo a su residencia, con lo que saca ahora.
        let hn = self.heroe_mut(*heroe);
        let propios = std::mem::take(&mut hn.heroe.viveres);
        let faccion = hn.faccion;
        let mut carga = carga.clone();
        for (r, n) in std::mem::take(&mut hn.carro) {
            *carga.entry(r).or_insert(0) += n;
        }
        let carga = &carga;
        if *racion_gratis {
            hn.heroe.racion_en = Some(t);
        }
        let comida = if *racion_gratis { mr.logistica.capacidad_viveres } else { propios + trigo };
        let posicion = self.puerta_de(*lugar).expect("el lugar existe");
        let columna = match (retoma, destino) {
            (Some(c0), Destino::Columna { tipo, politica }) => {
                // Sale con la columna que trajo: tal cual (de visita) o con lo que se le suma (si la conservaba en su residencia).
                self.tras_retomar(t, *c0, escuadras, carga, comida, tipo, politica);
                *c0
            }
            (_, Destino::Columna { tipo, politica }) => {
                let id = self.nuevo_id();
                let heroes = [*heroe];
                let militar = &self.estado.militar;
                let columna = Columna::salir(&mr, id, faccion, *heroe, tipo, politica, escuadras.clone(), carga.clone(), comida, t, &|ids| militar.soldados_de(&heroes, ids));
                let e = self.columna_suelta(columna, posicion);
                self.alta_de_columna(t, e);
                id
            }
            (Some(c0), Destino::Unirse(base)) => {
                // Quien parte con el ejército llevando su columna (de visita, o la que conservaba en su residencia): se le suma lo que saca ahora y se funde con la del Líder.
                if !escuadras.is_empty() || !carga.is_empty() || comida > 0 {
                    self.tras_retomar(t, *c0, escuadras, carga, comida, TipoColumna::Personal, Politica::Rechazar);
                }
                self.fusionar_columna(t, base, *c0).expect("la convocatoria comprobó que se puede fundir");
                base
            }
            (None, Destino::Unirse(base)) => {
                let antes = self.velocidad_de_ejercito(base).unwrap_or(0);
                let Estado { ejercitos, militar, .. } = &mut self.estado;
                let e = ejercitos.get_mut(&base).expect("la columna base existe");
                let mut heroes = Self::participantes_de(e);
                heroes.push(*heroe);
                e.columna.unirse_en_plaza(&mr, *heroe, escuadras, carga.clone(), comida, t, &|ids| militar.soldados_de(&heroes, ids)).expect("el ejército admite al que parte con él");
                self.poner_escuadras_en(&[*heroe], escuadras, Contenedor::Ejercito(base), t);
                self.recomponer_columna(base, t, antes);
                base
            }
        };
        self.ubicar_en_columna(*heroe, columna);
        self.tr.tocar(Clase::Ejercito, columna);
        columna
    }

    /// Salir con la columna `c0`: si el héroe la conservaba en su residencia, se le suman las escuadras, la carga y los víveres que saca.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn tras_retomar(&mut self, t: Instante, c0: u64, escuadras: &[u64], carga: &Mercancia, comida: Mil, tipo: TipoColumna, politica: Politica) {
        let heroes = self.estado.ejercitos.get(&c0).map(Self::participantes_de).unwrap_or_default();
        {
            let Estado { ejercitos, .. } = &mut self.estado;
            let c = &mut ejercitos.get_mut(&c0).expect("la columna con la que sale existe").columna;
            c.escuadras.extend_from_slice(escuadras);
            carga.iter().for_each(|(&r, &n)| *c.carga.entry(r).or_insert(0) += n);
            if comida > 0 {
                let ofrecido = comida;
                racion::llenar(&mut c.viveres, t, ofrecido);
            }
            if tipo == TipoColumna::Ejercito && c.tipo == TipoColumna::Personal {
                (c.tipo, c.politica) = (tipo, politica);
            }
        }
        self.poner_escuadras_en(&heroes, escuadras, Contenedor::Ejercito(c0), t);
        self.reanclar_consumo(c0, t);
        self.reprogramar_columna(c0, t);
    }

    /// Las comprobaciones comunes de las tres órdenes de salir de un lugar propio (`salirAlMundo`, `movilizarEjercito`, `salirDelCampamento`).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn salir_de_residencia(
        &mut self,
        t: Instante,
        p: &Peticion,
        heroe: u64,
        lugar: Lugar,
        escuadras: &[u64],
        carga: &[(Recurso, Mil)],
        movilizar: bool,
        politica: Politica,
    ) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        if let Lugar::Asentamiento(a) = lugar {
            self.plaza_o_rechazo(a as u64)?;
        }
        if let Lugar::Campamento(c) = lugar {
            self.puerta_de_campamento(c).ok_or_else(|| Respuesta::rechazo(rechazos::CAMPAMENTO_DESCONOCIDO, ids::campamento(c), v))?;
        }
        self.autorizar_heroe(p, heroe, false)?;
        self.sin_convocatoria(heroe)?;
        if let Lugar::Asentamiento(a) = lugar {
            if self.heroe(heroe).expect("existe").heroe.residencia != Some(Residencia::Asentamiento(a)) {
                return Err(Respuesta::rechazo(rechazos::SALIDA_SOLO_DE_TU_RESIDENCIA, "solo se sale de campaña desde tu residencia: ahí está tu campamento", v));
            }
        }
        let plan = self.validar_salida(t, heroe, lugar, escuadras, carga, movilizar, &mut Reservas::default())?;
        let tipo = if movilizar || politica != Politica::Rechazar { TipoColumna::Ejercito } else { TipoColumna::Personal };
        let ya_estaba = plan.retoma;
        let columna = self.aplicar_salida(t, &plan, Destino::Columna { tipo, politica });
        let (plaza, codigo) = match (lugar, movilizar) {
            (Lugar::Asentamiento(a), true) => (vec![a as u64], "ejercito.movilizado"),
            (Lugar::Asentamiento(a), false) => (vec![a as u64], "jugador.sale_al_mundo"),
            (Lugar::Campamento(_), _) => (vec![], "jugador.sale_de_campamento"),
        };
        let carga_total = self.estado.ejercitos[&columna].columna.carga.values().sum::<Mil>();
        self.evento_de_columna(codigo, t, columna, plaza, vec![("heroe", heroe as i64), ("escuadras", plan.escuadras.len() as i64), ("liderazgoUsado", plan.liderazgo), ("cargaTotal", carga_total)]);
        Ok(if ya_estaba.is_some() { vec![] } else { vec![columna] })
    }

    /// Un héroe que va en una convocatoria no sale por su cuenta (`sinConvocatoria`, Presencia §9.5).
    pub(super) fn sin_convocatoria(&self, heroe: u64) -> Result<(), Respuesta> {
        match self.estado.militar.convocatoria_de(heroe) {
            None => Ok(()),
            Some(_) => Err(Respuesta::rechazo(rechazos::HEROE_EN_CONVOCATORIA, "estás en un ejército en preparación: sepárate o cancélalo antes de salir por tu cuenta", self.estado.version)),
        }
    }

    // ───────────────────────────── Quién entra ─────────────────────────────

    /// El acceso de una plaza: lo que su dueño ha decidido que se permita a cada relación (por defecto, todos menos los enemigos).
    pub fn acceso_de_plaza(&self, plaza: u64) -> Option<Acceso> {
        self.estado.asentamientos.get(&plaza).map(|a| a.acceso)
    }

    /// ¿Le abre la plaza su puerta? Quien reside en ella y los de su Facción, siempre; el resto, según el acceso que fijó su dueño para su relación (aliada o
    /// vasalla, neutral, enemiga). `false` si el héroe o la plaza no existen.
    pub fn puede_entrar_en_plaza(&self, heroe: u64, plaza: u64) -> bool {
        let Some(hn) = self.heroe(heroe).filter(|_| self.estado.asentamientos.contains_key(&plaza)) else { return false };
        hn.heroe.residencia == Some(Residencia::Asentamiento(plaza as u32)) || self.acceso_deja_pasar(heroe, plaza)
    }

    /// ¿Deja pasar el acceso de la plaza, con su dueño de ahora, a la relación del héroe? Es `puede_entrar_en_plaza` sin la excepción de la residencia: la mira quien ha de saber
    /// si seguiría abriéndose la puerta a alguien que deja de residir en ella (la conquista de la madre).
    pub(super) fn acceso_deja_pasar(&self, heroe: u64, plaza: u64) -> bool {
        let (Some(hn), Some(a)) = (self.heroe(heroe), self.estado.asentamientos.get(&plaza)) else { return false };
        a.acceso.permite(self.estado.militar.trato(a.faccion, hn.faccion))
    }

    /// ¿Le abriría su puerta un pueblo extractor? Hereda el acceso de su plaza madre (`None` si el pueblo no existe). Se entra con `EntrarEnPueblo` (`dentro`).
    pub fn puede_entrar_en_pueblo(&self, heroe: u64, pueblo: u64) -> Option<bool> {
        let (madre, _) = self.pueblo(pueblo)?;
        Some(self.puede_entrar_en_plaza(heroe, madre))
    }

    /// ¿Le abriría su puerta una avanzada? Hereda el acceso de su plaza madre (`None` si no existe o es una ruina sin dueño). Se entra con `EntrarEnAvanzada` (`dentro`).
    pub fn puede_entrar_en_avanzada(&self, heroe: u64, avanzada: u64) -> Option<bool> {
        let (madre, _) = self.avanzada(avanzada)?;
        Some(self.puede_entrar_en_plaza(heroe, madre?))
    }

    // ───────────────────────────── Entrar ─────────────────────────────

    /// El residente entra en su residencia con su columna: la tropa vuelve al campamento, el carro se queda con él y, si no lleva caravanas, la columna se
    /// disuelve (los víveres vuelven al héroe). Con caravanas adjuntas la columna sigue —quieta, sin tropa— para llevarlas, y el carro se queda en ella (su capacidad
    /// es la de la columna con sus caravanas). Devuelve lo que carga el carro y si la columna sigue.
    fn recoger_en_residencia(&mut self, t: Instante, heroe: u64, columna: u64, ubicacion: Ubicacion) -> (Mil, bool) {
        let sigue = !self.estado.ejercitos[&columna].caravanas_adjuntas.is_empty();
        if sigue && self.estado.ejercitos[&columna].marchando() {
            self.detener(columna, t);
        }
        let propios = self.escuadras_de_heroe_en(heroe, columna);
        let e = self.estado.ejercitos.get_mut(&columna).expect("existe");
        let en_el_carro = total(&e.columna.carga);
        let carga = if sigue { Mercancia::new() } else { std::mem::take(&mut e.columna.carga) };
        e.columna.escuadras.retain(|s| !propios.contains(s));
        let viveres = e.columna.viveres.valor_en(t);
        self.poner_escuadras_en(&[heroe], &propios, Contenedor::Campamento, t);
        let cap = self.estado.reglas.economia.militar.logistica.capacidad_viveres;
        let hn = self.heroe_mut(heroe);
        for (&r, &n) in &carga {
            *hn.carro.entry(r).or_insert(0) += n;
        }
        hn.heroe.presencia.ubicacion = ubicacion;
        if sigue {
            self.reanclar_consumo(columna, t);
            self.reprogramar_columna(columna, t);
        } else {
            hn.heroe.viveres = (hn.heroe.viveres + viveres).min(cap);
            hn.columna = None;
            self.baja_de_columna(columna);
        }
        (en_el_carro, sigue)
    }

    /// El héroe entra en la plaza con todo lo suyo (Entrar con todo, 2026-10-22): en su residencia la tropa vuelve al campamento, los víveres al héroe y el carro se
    /// queda con él (`vaciarCarro` lo pasa al almacén personal); en otra plaza entra de visita y su columna entera —tropa, carro, víveres y caravanas— queda
    /// dentro, fuera del mapa.
    pub(super) fn entrar_en_plaza(&mut self, t: Instante, heroe: u64, plaza: u64) {
        let columna = self.estado.ejercito_de_heroe(heroe).expect("va en una columna");
        let reside = self.heroe(heroe).expect("existe").heroe.residencia == Some(Residencia::Asentamiento(plaza as u32));
        let dentro = Ubicacion::Asentamiento(plaza as u32);
        if reside {
            let (carro, sigue) = self.recoger_en_residencia(t, heroe, columna, dentro);
            self.tr.tocar(Clase::Asentamiento, plaza);
            self.anunciar_columna(columna, Motivo::Plan);
            self.emitir_propio(t, "jugador.entra_en_asentamiento", vec![plaza], vec![("heroe", heroe as i64), ("ejercito", columna as i64), ("residencia", 1), ("carro", carro), ("columnaSigue", sigue as i64)]);
        } else {
            self.quedar_dentro_de_visita(t, heroe, columna, dentro);
            let caravanas = self.estado.ejercitos[&columna].caravanas_adjuntas.len() as i64;
            self.emitir_propio(t, "jugador.entra_en_asentamiento", vec![plaza], vec![("heroe", heroe as i64), ("ejercito", columna as i64), ("residencia", 0), ("caravanas", caravanas)]);
        }
    }

    /// El héroe entra en el campamento con todo lo suyo (Presencia §5.1). El residente deja la tropa y se queda con el carro, como en una plaza; el visitante entra con
    /// su columna entera, que queda dentro.
    pub(super) fn entrar_en_campamento(&mut self, t: Instante, heroe: u64, campamento: u32) {
        let columna = self.estado.ejercito_de_heroe(heroe).expect("va en una columna");
        let reside = self.heroe(heroe).expect("existe").heroe.residencia == Some(Residencia::Campamento(campamento));
        let dentro = Ubicacion::Mercenarios(campamento);
        let mut carro = 0;
        if reside {
            carro = self.recoger_en_residencia(t, heroe, columna, dentro).0;
        } else {
            if self.estado.ejercitos[&columna].marchando() {
                self.detener(columna, t);
            }
            self.reanclar_consumo(columna, t);
            self.reprogramar_columna(columna, t);
            self.heroe_mut(heroe).heroe.presencia.ubicacion = dentro;
        }
        self.anunciar_columna(columna, Motivo::Plan);
        self.emitir_propio(t, "jugador.entra_en_campamento", vec![], vec![("heroe", heroe as i64), ("campamento", campamento as i64), ("ejercito", columna as i64), ("residencia", reside as i64), ("carro", carro)]);
    }

    /// `entrarEnAsentamiento`, `salirDeAsentamiento`, `guarnecer`, `entrarEnCampamento`, las tres salidas, `fijarAcceso` y `vaciarCarro`.
    pub(super) fn ejecutar_salidas(&mut self, t: Instante, p: &Peticion) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        match &p.comando {
            Comando::SalirAlMundo { asentamiento, heroe, escuadras, carga } => self.salir_de_residencia(t, p, *heroe, Lugar::Asentamiento(*asentamiento as u32), escuadras, carga, false, Politica::Rechazar),
            Comando::MovilizarEjercito { asentamiento, heroe, escuadras, carga, politica } => {
                self.salir_de_residencia(t, p, *heroe, Lugar::Asentamiento(*asentamiento as u32), escuadras, carga, true, *politica)
            }
            Comando::SalirDelCampamento { campamento, heroe, escuadras, carga, politica } => self.salir_de_residencia(t, p, *heroe, Lugar::Campamento(*campamento), escuadras, carga, false, *politica),
            Comando::EntrarEnAsentamiento { asentamiento, heroe } => {
                let plaza = self.plaza_o_rechazo(*asentamiento)?.posicion;
                self.autorizar_heroe(p, *heroe, false)?;
                let (_, e) = self.columna_a_bordo(*heroe)?;
                if e.columna.tipo != TipoColumna::Personal {
                    return Err(rechazo(rechazos::COLUMNA_ES_UN_EJERCITO, "vas en un ejército: un ejército entra con `guarnecer`; para entrar solo, sepárate antes".into()));
                }
                if e.columna.es_formacion() {
                    return Err(rechazo(rechazos::COLUMNA_EN_FORMACION, "cancela antes la formación".into()));
                }
                if !p.admin && e.posicion_en(t).dist(plaza) > RADIO_PUERTA {
                    return Err(rechazo(rechazos::ASENTAMIENTO_FUERA_DE_LA_PUERTA, "la columna no está a la puerta de la plaza".into()));
                }
                if !self.puede_entrar_en_plaza(*heroe, *asentamiento) {
                    return Err(rechazo(rechazos::PUERTA_CERRADA, "la plaza no te abre su puerta".into()));
                }
                self.entrar_en_plaza(t, *heroe, *asentamiento);
                Ok(vec![])
            }
            Comando::SalirDeAsentamiento { asentamiento, heroe } => {
                self.plaza_o_rechazo(*asentamiento)?;
                self.autorizar_heroe(p, *heroe, false)?;
                self.sin_convocatoria(*heroe)?;
                let hn = self.heroe(*heroe).expect("existe");
                if hn.heroe.residencia == Some(Residencia::Asentamiento(*asentamiento as u32)) {
                    return Err(rechazo(rechazos::SALIDA_DESDE_TU_RESIDENCIA, "desde tu residencia se sale al mundo (`salirAlMundo`), eligiendo tropa y carga".into()));
                }
                if hn.heroe.presencia.ubicacion != Ubicacion::Asentamiento(*asentamiento as u32) {
                    return Err(rechazo(rechazos::HEROE_NO_ESTA_EN_LA_PLAZA, format!("el héroe no está dentro de as-{asentamiento}")));
                }
                // Sale con todo lo que entró: su columna vuelve al mapa tal cual. Quien no trajo ninguna (lo colocó la administración) sale solo, con sus víveres.
                let posicion = self.puerta_de(Lugar::Asentamiento(*asentamiento as u32)).expect("la plaza existe");
                let columna = self.salir_al_mapa(t, *heroe, posicion);
                self.emitir_propio(t, "jugador.sale_de_asentamiento", vec![*asentamiento], vec![("heroe", *heroe as i64), ("ejercito", columna as i64)]);
                Ok(vec![])
            }
            Comando::Guarnecer { asentamiento, heroe } => self.guarnecer(t, p, *asentamiento, *heroe),
            Comando::EntrarEnCampamento { campamento, heroe } => self.entrar_en_el_campamento(t, p, *campamento, *heroe),
            Comando::FijarAcceso { asentamiento, aliados, neutrales, enemigos } => {
                let plaza = self.plaza_o_rechazo(*asentamiento)?;
                if !self.manda_en(p, plaza.faccion) {
                    return Err(rechazo(rechazos::NO_AUTORIZADO, "solo quien manda en la plaza decide quién entra".into()));
                }
                let nuevo = Acceso { aliados: *aliados, neutrales: *neutrales, enemigos: *enemigos };
                if plaza.acceso == nuevo {
                    return Err(Respuesta::sin_cambios(v));
                }
                // Un pueblo o una avanzada de la plaza en batalla no recibe órdenes, y echar a quien combate dentro sería una (la administración no lleva candado).
                if !p.admin && self.sitios_en_batalla(*asentamiento) {
                    return Err(rechazo(rechazos::BATALLA_BLOQUEADO, "un pueblo o una avanzada de la plaza está en una batalla: la puerta no se toca hasta que acabe".into()));
                }
                self.estado.asentamientos.get_mut(asentamiento).expect("existe").acceso = nuevo;
                self.tr.tocar(Clase::Asentamiento, *asentamiento);
                // Cerrar la puerta echa al mapa, al instante y con todo lo suyo, a quien ya está dentro y ya no puede entrar.
                self.cerrar_la_puerta(t, *asentamiento);
                self.emitir_propio(t, "asentamiento.acceso", vec![*asentamiento], vec![("aliados", nuevo.aliados as i64), ("neutrales", nuevo.neutrales as i64), ("enemigos", nuevo.enemigos as i64)]);
                Ok(vec![])
            }
            Comando::VaciarCarro { heroe } => self.vaciar_carro(t, p, *heroe),
            _ => unreachable!("solo se llama con órdenes de salir y entrar"),
        }
    }

    /// `vaciarCarro`: dentro de su residencia, el héroe pasa el carro que trajo a su almacén personal (`ALMACEN_PERSONAL.capacidad`), recurso a recurso y por orden,
    /// hasta donde quepa. Lo que no cabe se queda en el carro (en la columna que conserva para llevar sus caravanas, si el carro estaba en ella). Entrar en la
    /// residencia no lo hace solo.
    // DUDA: el almacén personal de quien reside en una plaza tiene la capacidad del de un campamento de mercenarios (hasta que la residencia completa del sistema 5 diga otra).
    fn vaciar_carro(&mut self, t: Instante, p: &Peticion, heroe: u64) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        self.autorizar_heroe(p, heroe, false)?;
        let hn = self.heroe(heroe).expect("existe");
        let (en_residencia, plaza) = match (hn.heroe.residencia, hn.heroe.presencia.ubicacion) {
            (Some(Residencia::Asentamiento(a)), Ubicacion::Asentamiento(b)) => (a == b, vec![a as u64]),
            (Some(Residencia::Campamento(a)), Ubicacion::Mercenarios(b)) => (a == b, vec![]),
            _ => (false, vec![]),
        };
        if !en_residencia {
            return Err(rechazo(rechazos::VACIAR_SOLO_EN_TU_RESIDENCIA, "el carro se vacía dentro de tu residencia, donde está tu almacén personal".into()));
        }
        // El carro está con el héroe y, si conserva una columna para llevar sus caravanas, también en ella.
        let columna = hn.columna.filter(|c| self.estado.ejercitos.contains_key(c));
        if hn.carro.is_empty() && columna.is_none_or(|c| self.estado.ejercitos[&c].columna.carga.is_empty()) {
            return Err(Respuesta::sin_cambios(v));
        }
        let cap = self.estado.reglas.economia.comercio.mercenarios.capacidad_almacen_personal;
        let mut libre = (cap - total(&hn.almacen_personal)).max(0);
        if libre == 0 {
            return Err(rechazo(rechazos::ALMACEN_PERSONAL_LLENO, "tu almacén personal está lleno: no cabe nada del carro".into()));
        }
        let mut origenes = vec![std::mem::take(&mut self.heroe_mut(heroe).carro)];
        if let Some(c) = columna {
            origenes.push(std::mem::take(&mut self.estado.ejercitos.get_mut(&c).expect("existe").columna.carga));
        }
        let (mut movido, mut restos) = (0, Vec::new());
        for origen in origenes {
            let mut resto = Mercancia::new();
            for (r, n) in origen {
                let cabe = n.min(libre);
                if cabe > 0 {
                    *self.heroe_mut(heroe).almacen_personal.entry(r).or_insert(0) += cabe;
                    libre -= cabe;
                    movido += cabe;
                }
                if n > cabe {
                    resto.insert(r, n - cabe);
                }
            }
            restos.push(resto);
        }
        let quedan: Mil = restos.iter().map(total).sum();
        let mut restos = restos.into_iter();
        self.heroe_mut(heroe).carro = restos.next().expect("el del héroe");
        if let (Some(c), Some(resto)) = (columna, restos.next()) {
            self.estado.ejercitos.get_mut(&c).expect("existe").columna.carga = resto;
            self.tr.tocar(Clase::Ejercito, c);
        }
        self.emitir_propio(t, "heroe.vacia_el_carro", plaza, vec![("heroe", heroe as i64), ("movido", movido), ("resto", quedan)]);
        Ok(vec![])
    }

    /// La columna en la que va el héroe, a bordo (`ubicacion = columna`).
    pub(super) fn columna_a_bordo(&self, heroe: u64) -> Result<(u64, &Ejercito), Respuesta> {
        let v = self.estado.version;
        match self.heroe(heroe).map(|h| h.heroe.presencia.ubicacion) {
            Some(Ubicacion::Columna(c)) => match self.estado.ejercitos.get(&c) {
                Some(e) => Ok((c, e)),
                None => Err(Respuesta::rechazo(rechazos::EJERCITO_DESCONOCIDO, ids::ejercito(c), v)),
            },
            _ => Err(Respuesta::rechazo(rechazos::HEROE_SIN_COLUMNA, "el héroe no va en ninguna columna", v)),
        }
    }

    /// `guarnecer` (Presencia §4.3): un ejército, a la puerta de una plaza que le abre su puerta (a todos sus héroes), entra con todo —incluidas sus caravanas
    /// adjuntas, sean de donde sean—: solo lo ordena su Líder; se desarma y cada héroe entra como lo haría solo. Las caravanas se quedan con la columna del Líder,
    /// que sigue siendo un ejército mientras las lleve.
    fn guarnecer(&mut self, t: Instante, p: &Peticion, asentamiento: u64, heroe: u64) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let posicion = self.plaza_o_rechazo(asentamiento)?.posicion;
        self.autorizar_heroe(p, heroe, false)?;
        let (id, e) = self.columna_a_bordo(heroe)?;
        if !Self::es_ejercito(e) {
            return Err(rechazo(rechazos::COLUMNA_NO_ES_EJERCITO, "solo un ejército guarnece: una columna personal entra con `entrarEnAsentamiento`".into()));
        }
        if !p.admin && e.posicion_en(t).dist(posicion) > RADIO_PUERTA {
            return Err(rechazo(rechazos::ASENTAMIENTO_FUERA_DE_LA_PUERTA, "la columna no está a la puerta de la plaza".into()));
        }
        if e.columna.lider != heroe {
            return Err(rechazo(rechazos::COLUMNA_SOLO_EL_LIDER, "solo el Líder hace entrar al ejército".into()));
        }
        let antes = Self::participantes_de(e);
        if let Some(h) = antes.iter().find(|&&h| !self.puede_entrar_en_plaza(h, asentamiento)) {
            return Err(rechazo(rechazos::PUERTA_CERRADA, format!("la plaza no abre su puerta al héroe {h} del ejército")));
        }
        let adjuntas = e.caravanas_adjuntas.len();
        let escuadrones = e.columna.escuadras.len();
        self.desarmar_columna(t, id);
        for h in antes.iter().copied() {
            self.entrar_en_plaza(t, h, asentamiento);
        }
        self.emitir_propio(t, "ejercito.guarnece", vec![asentamiento], vec![("ejercito", id as i64), ("escuadrones", escuadrones as i64), ("caravanas", adjuntas as i64), ("heroes", antes.len() as i64)]);
        Ok(vec![])
    }

    /// `entrarEnCampamento` (Presencia §5.1): quien va solo en una columna personal entra; un ejército (o una personal con compañía) entra entero,
    /// ordenado por su Líder, con todo —caravanas incluidas—, se desarma y cada héroe entra como lo haría solo.
    fn entrar_en_el_campamento(&mut self, t: Instante, p: &Peticion, campamento: u32, heroe: u64) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let puerta = self.puerta_de_campamento(campamento).ok_or_else(|| rechazo(rechazos::CAMPAMENTO_DESCONOCIDO, ids::campamento(campamento)))?;
        self.autorizar_heroe(p, heroe, false)?;
        let (id, e) = self.columna_a_bordo(heroe)?;
        let solo = e.columna.tipo == TipoColumna::Personal && e.columna.participantes.len() == 1;
        if solo && e.columna.es_formacion() {
            return Err(rechazo(rechazos::COLUMNA_EN_FORMACION, "cancela antes la formación".into()));
        }
        if !solo && e.columna.lider != heroe {
            return Err(rechazo(rechazos::COLUMNA_SOLO_EL_LIDER, "solo el Líder hace entrar al ejército".into()));
        }
        if !p.admin && e.posicion_en(t).dist(puerta) > RADIO_PUERTA {
            return Err(rechazo(rechazos::CAMPAMENTO_FUERA_DE_LA_PUERTA, "la columna no está a la puerta del campamento".into()));
        }
        if solo {
            self.entrar_en_campamento(t, heroe, campamento);
        } else {
            let antes = Self::participantes_de(e);
            self.desarmar_columna(t, id);
            for h in antes.iter().copied() {
                self.entrar_en_campamento(t, h, campamento);
            }
            self.emitir_propio(t, "ejercito.entra_en_campamento", vec![], vec![("ejercito", id as i64), ("campamento", campamento as i64), ("heroes", antes.len() as i64)]);
        }
        Ok(vec![])
    }
}
