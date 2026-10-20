//! Salir al mundo y entrar en un lugar (Presencia_Y_Convocatoria §2-§5, Movimiento_Ejercitos §3 y §11): `salirAlMundo`, `movilizarEjercito`,
//! `salirDelCampamento`, `entrarEnAsentamiento`, `salirDeAsentamiento`, `guarnecer` y `entrarEnCampamento`. Una salida es siempre la misma
//! operación en dos tiempos: [`PlanDeSalida`] comprueba **todo** sin tocar nada (tropa, Liderazgo, carro, reserva de trigo, víveres) y
//! `aplicar_salida` la ejecuta; las convocatorias simulan la salida de cada integrante con el primer tiempo y parten con el segundo, de modo
//! que «saldría con lo que eligió» y «sale» no pueden discrepar (Presencia §9.2).
//!
//! # Dónde están los víveres
//! Los víveres son del héroe (Movimiento_Ejercitos §6.1). Dentro de un lugar los lleva él (`Heroe::viveres`); en una columna son un único montón de
//! la columna (`Columna::viveres`, capacidad de la suma de sus héroes). Al salir pasan a la columna y al entrar en su residencia —donde la columna
//! se deshace— vuelven al héroe; una columna que queda aparcada a la puerta (de visita) los conserva y sigue comiendo a ritmo de acampada.
//!
//! # Lo provisional
//! La puerta de una plaza (`fijarPuerta`, vetos, grupos) es del sistema político: hasta que exista, entra en una plaza quien reside en ella o es
//! ciudadano de su Facción (los aliados, que dependen de las relaciones, no entran). `Fundar` todavía no deshace la columna dentro de la plaza nueva.

use super::*;
use crate::estado_militar::HeroeNucleo;
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
    /// La columna que retoma: la que dejó aparcada a la puerta de un lugar ajeno, o el resto que dejó al entrar en su campamento.
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
    /// `carga` lo que carga en el carro. Quien reside en el lugar elige; quien está de visita retoma la columna que dejó aparcada (su elección se
    /// ignora). `reservas` acumula lo que gastan las salidas simuladas antes de esta en el mismo lugar.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn validar_salida(&mut self, t: Instante, heroe: u64, lugar: Lugar, elegidas: &[u64], carga: &[(Recurso, Mil)], exigir_escuadras: bool, reservas: &mut Reservas) -> Result<PlanDeSalida, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let mr = self.estado.reglas.economia.militar.clone();
        let hn = self.heroe(heroe).expect("el héroe existe");
        let propios = hn.heroe.viveres;
        let (reside, presente, no_esta) = match lugar {
            Lugar::Asentamiento(a) => (hn.heroe.residencia == Some(Residencia::Asentamiento(a)), hn.heroe.presencia.ubicacion == Ubicacion::Asentamiento(a), rechazos::HEROE_NO_ESTA_EN_LA_PLAZA),
            Lugar::Campamento(c) => (hn.heroe.residencia == Some(Residencia::Campamento(c)), hn.heroe.presencia.ubicacion == Ubicacion::Mercenarios(c), rechazos::HEROE_NO_ESTA_EN_EL_CAMPAMENTO),
        };
        // Quien ya va fuera, en una plaza, no sale otra vez; en un campamento, lo que tiene fuera es el resto que dejó al entrar (se retoma).
        let columna = hn.columna.filter(|_| !matches!(lugar, Lugar::Asentamiento(_)) || !reside);
        if hn.columna.is_some() && columna.is_none() {
            return Err(rechazo(rechazos::HEROE_YA_EN_COLUMNA, "ya estás fuera: no se puede salir dos veces".into()));
        }
        if !presente {
            return Err(rechazo(no_esta, format!("el héroe no está dentro de {lugar:?}")));
        }
        if !reside {
            let retoma = columna.ok_or_else(|| rechazo(rechazos::COLUMNA_NO_APARCADA, "tu columna ya no está a la puerta".into()))?;
            return Ok(PlanDeSalida { heroe, lugar, escuadras: vec![], carga: Mercancia::new(), trigo: 0, racion_gratis: false, retoma: Some(retoma), liderazgo: 0 });
        }

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
        let ocupado = columna.and_then(|c| self.estado.ejercitos.get(&c)).map_or(0, |c| total(&c.columna.carga));
        if total(&pedido) > CAPACIDAD_CARRO - ocupado {
            return Err(rechazo(rechazos::CARRO_LLENO, format!("el carro admite {} y se piden {}", CAPACIDAD_CARRO - ocupado, total(&pedido))));
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
    /// (los integrantes de una convocatoria que parte) con lo que saca; si no, nace una columna (o se retoma la aparcada).
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
                self.tr.asentamientos.push(*a as u64);
            }
            Lugar::Campamento(_) => {
                let personal = &mut self.heroe_mut(*heroe).almacen_personal;
                carga.iter().for_each(|(&r, &n)| *personal.entry(r).or_insert(0) -= n);
                personal.retain(|_, n| *n > 0);
            }
        }
        // 2. Los víveres que lleva: los suyos, más lo que llenó. La ración gratis los lleva al tope.
        let hn = self.heroe_mut(*heroe);
        let propios = std::mem::take(&mut hn.heroe.viveres);
        let faccion = hn.faccion;
        if *racion_gratis {
            hn.heroe.racion_en = Some(t);
        }
        let comida = if *racion_gratis { mr.logistica.capacidad_viveres } else { propios + trigo };
        let posicion = self.puerta_de(*lugar).expect("el lugar existe");
        let columna = match (retoma, destino) {
            (Some(c0), Destino::Columna { tipo, politica }) => {
                // Retomar: la aparcada tal cual (de visita) o el resto con lo que se le suma (en su campamento).
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
                // Un visitante que parte con el ejército: su columna aparcada se funde con la del Líder.
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
        self.tr.tocados.push(columna);
        columna
    }

    /// Retomar la columna `c0`: si el héroe deja en ella un resto (reside en el campamento), se le suman las escuadras, la carga y los víveres.
    #[allow(clippy::too_many_arguments)]
    fn tras_retomar(&mut self, t: Instante, c0: u64, escuadras: &[u64], carga: &Mercancia, comida: Mil, tipo: TipoColumna, politica: Politica) {
        let heroes = self.estado.ejercitos.get(&c0).map(Self::participantes_de).unwrap_or_default();
        {
            let Estado { ejercitos, .. } = &mut self.estado;
            let c = &mut ejercitos.get_mut(&c0).expect("la columna aparcada existe").columna;
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
            self.puerta_de_campamento(c).ok_or_else(|| Respuesta::rechazo(rechazos::CAMPAMENTO_DESCONOCIDO, format!("campamento-{c}"), v))?;
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

    // ───────────────────────────── Entrar ─────────────────────────────

    /// ¿Entra el héroe en esa plaza? Un residente, siempre; si no, quien es ciudadano de su Facción (provisional: véase el módulo).
    fn permiso_de_entrada(&self, hn: &HeroeNucleo, plaza: u64) -> bool {
        hn.heroe.residencia == Some(Residencia::Asentamiento(plaza as u32)) || hn.faccion.is_some_and(|f| f == self.estado.asentamientos[&plaza].faccion)
    }

    /// El héroe entra en la plaza «como lo haría solo» (Presencia §4.1): en su residencia su columna se deshace dentro (la tropa vuelve al
    /// campamento, el carro al almacén hasta donde quepa y los víveres quedan con él); en otra plaza entra de visita y su columna queda aparcada
    /// a la puerta, parada.
    pub(super) fn entrar_en_plaza(&mut self, t: Instante, heroe: u64, plaza: u64) {
        let columna = self.estado.ejercito_de_heroe(heroe).expect("va en una columna");
        let reside = self.heroe(heroe).expect("existe").heroe.residencia == Some(Residencia::Asentamiento(plaza as u32));
        if reside {
            let e = self.estado.ejercitos.get(&columna).expect("existe");
            let (propios, viveres) = (self.escuadras_de_heroe_en(heroe, columna), e.columna.viveres.valor_en(t));
            let carga = e.columna.carga.clone();
            let almacen = &mut self.estado.asentamientos.get_mut(&plaza).expect("existe").economia.almacen;
            let admitido: Mil = carga.iter().map(|(&r, &n)| almacen.agregar(r, n)).sum();
            self.tr.asentamientos.push(plaza);
            self.poner_escuadras_en(&[heroe], &propios, Contenedor::Campamento, t);
            let cap = self.estado.reglas.economia.militar.logistica.capacidad_viveres;
            let hn = self.heroe_mut(heroe);
            hn.heroe.viveres = (hn.heroe.viveres + viveres).min(cap);
            hn.columna = None;
            hn.heroe.presencia.ubicacion = Ubicacion::Asentamiento(plaza as u32);
            self.baja_de_columna(columna);
            self.emitir_propio(t, "jugador.entra_en_asentamiento", vec![plaza], vec![("heroe", heroe as i64), ("ejercito", columna as i64), ("residencia", 1), ("perdido", total(&carga) - admitido)]);
        } else {
            if self.estado.ejercitos[&columna].marchando() {
                self.detener(columna, t);
            }
            self.heroe_mut(heroe).heroe.presencia.ubicacion = Ubicacion::Asentamiento(plaza as u32);
            self.anunciar_columna(columna, Motivo::Plan);
            self.tr.tocados.push(columna);
            self.emitir_propio(t, "jugador.entra_en_asentamiento", vec![plaza], vec![("heroe", heroe as i64), ("ejercito", columna as i64), ("residencia", 0)]);
        }
    }

    /// El héroe entra en el campamento (Presencia §5.1). El residente deja la tropa y vuelca el carro en su almacén personal hasta la capacidad: lo
    /// que no cabe se queda en el carro, y la columna sigue aparcada a la puerta con ese resto, sin escuadras. El visitante entra con su columna
    /// intacta a la puerta.
    pub(super) fn entrar_en_campamento(&mut self, t: Instante, heroe: u64, campamento: u32) {
        let columna = self.estado.ejercito_de_heroe(heroe).expect("va en una columna");
        let reside = self.heroe(heroe).expect("existe").heroe.residencia == Some(Residencia::Campamento(campamento));
        let cap_personal = self.estado.reglas.economia.comercio.mercenarios.capacidad_almacen_personal;
        let mut resto = Mercancia::new();
        if reside {
            let propios = self.escuadras_de_heroe_en(heroe, columna);
            let carga = self.estado.ejercitos[&columna].columna.carga.clone();
            let hn = self.heroe_mut(heroe);
            let mut libre = (cap_personal - total(&hn.almacen_personal)).max(0);
            for (&r, &n) in &carga {
                let cabe = n.min(libre);
                if cabe > 0 {
                    *hn.almacen_personal.entry(r).or_insert(0) += cabe;
                }
                if n - cabe > 0 {
                    resto.insert(r, n - cabe);
                }
                libre -= cabe;
            }
            self.poner_escuadras_en(&[heroe], &propios, Contenedor::Campamento, t);
            let e = self.estado.ejercitos.get_mut(&columna).expect("existe");
            e.columna.carga = resto.clone();
            e.columna.escuadras.retain(|s| !propios.contains(s));
        }
        let e = &self.estado.ejercitos[&columna];
        let (marchando, queda) = (e.marchando(), !reside || !resto.is_empty());
        if marchando {
            self.detener(columna, t);
        }
        if !queda {
            // Sin resto, la columna se deshace dentro y los víveres quedan con el héroe.
            let viveres = self.estado.ejercitos[&columna].columna.viveres.valor_en(t);
            let cap = self.estado.reglas.economia.militar.logistica.capacidad_viveres;
            let hn = self.heroe_mut(heroe);
            hn.heroe.viveres = (hn.heroe.viveres + viveres).min(cap);
            hn.columna = None;
            self.baja_de_columna(columna);
        } else {
            self.reanclar_consumo(columna, t);
            self.reprogramar_columna(columna, t);
        }
        self.heroe_mut(heroe).heroe.presencia.ubicacion = Ubicacion::Mercenarios(campamento);
        self.anunciar_columna(columna, Motivo::Plan);
        self.emitir_propio(t, "jugador.entra_en_campamento", vec![], vec![("heroe", heroe as i64), ("campamento", campamento as i64), ("ejercito", columna as i64), ("residencia", reside as i64), ("resto", total(&resto))]);
    }

    /// `entrarEnAsentamiento`, `salirDeAsentamiento`, `guarnecer`, `entrarEnCampamento` y las tres salidas.
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
                let (columna, e) = self.columna_a_bordo(*heroe)?;
                if e.columna.tipo != TipoColumna::Personal {
                    return Err(rechazo(rechazos::COLUMNA_ES_UN_EJERCITO, "vas en un ejército: un ejército entra con `guarnecer`; para entrar solo, sepárate antes".into()));
                }
                if e.columna.es_formacion() {
                    return Err(rechazo(rechazos::COLUMNA_EN_FORMACION, "cancela antes la formación".into()));
                }
                if !p.admin && e.posicion_en(t).dist(plaza) > RADIO_PUERTA {
                    return Err(rechazo(rechazos::ASENTAMIENTO_FUERA_DE_LA_PUERTA, "la columna no está a la puerta de la plaza".into()));
                }
                if !self.permiso_de_entrada(self.heroe(*heroe).expect("existe"), *asentamiento) {
                    return Err(rechazo(rechazos::PUERTA_CERRADA, "la plaza no te abre su puerta".into()));
                }
                let _ = columna;
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
                let columna = hn.columna.ok_or_else(|| rechazo(rechazos::COLUMNA_NO_APARCADA, "tu columna ya no está a la puerta".into()))?;
                self.ubicar_en_columna(*heroe, columna);
                self.tr.tocados.push(columna);
                self.emitir_propio(t, "jugador.sale_de_asentamiento", vec![*asentamiento], vec![("heroe", *heroe as i64), ("ejercito", columna as i64)]);
                Ok(vec![])
            }
            Comando::Guarnecer { asentamiento, heroe } => self.guarnecer(t, p, *asentamiento, *heroe),
            Comando::EntrarEnCampamento { campamento, heroe } => self.entrar_en_el_campamento(t, p, *campamento, *heroe),
            _ => unreachable!("solo se llama con órdenes de salir y entrar"),
        }
    }

    /// La columna en la que va el héroe, a bordo (`ubicacion = columna`).
    pub(super) fn columna_a_bordo(&self, heroe: u64) -> Result<(u64, &Ejercito), Respuesta> {
        let v = self.estado.version;
        match self.heroe(heroe).map(|h| h.heroe.presencia.ubicacion) {
            Some(Ubicacion::Columna(c)) => match self.estado.ejercitos.get(&c) {
                Some(e) => Ok((c, e)),
                None => Err(Respuesta::rechazo(rechazos::EJERCITO_DESCONOCIDO, format!("ej-{c}"), v)),
            },
            _ => Err(Respuesta::rechazo(rechazos::HEROE_SIN_COLUMNA, "el héroe no va en ninguna columna", v)),
        }
    }

    /// `guarnecer` (Presencia §4.3): un ejército de la Facción de la plaza, a su puerta, entra: solo lo ordena su Líder, solo si todas sus caravanas
    /// adjuntas son de esa plaza (quedan aparcadas en ella), se desarma y cada héroe entra como lo haría solo.
    fn guarnecer(&mut self, t: Instante, p: &Peticion, asentamiento: u64, heroe: u64) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let plaza = self.plaza_o_rechazo(asentamiento)?;
        let (faccion_plaza, posicion) = (plaza.faccion, plaza.posicion);
        self.autorizar_heroe(p, heroe, false)?;
        let (id, e) = self.columna_a_bordo(heroe)?;
        if !Self::es_ejercito(e) {
            return Err(rechazo(rechazos::COLUMNA_NO_ES_EJERCITO, "solo un ejército guarnece: una columna personal entra con `entrarEnAsentamiento`".into()));
        }
        if e.columna.faccion != Some(faccion_plaza) {
            return Err(rechazo(rechazos::COLUMNA_OTRA_FACCION, "solo se guarnece una plaza de tu propia Facción".into()));
        }
        if !p.admin && e.posicion_en(t).dist(posicion) > RADIO_PUERTA {
            return Err(rechazo(rechazos::ASENTAMIENTO_FUERA_DE_LA_PUERTA, "la columna no está a la puerta de la plaza".into()));
        }
        if e.columna.lider != heroe {
            return Err(rechazo(rechazos::COLUMNA_SOLO_EL_LIDER, "solo el Líder hace entrar al ejército".into()));
        }
        self.caravanas_del_lugar(e, asentamiento)?;
        let adjuntas = e.caravanas_adjuntas.clone();
        let antes = Self::participantes_de(e);
        let escuadrones = e.columna.escuadras.len();
        self.aparcar_todas(t, id, asentamiento, posicion);
        self.desarmar_columna(t, id);
        for h in antes.iter().copied() {
            self.entrar_en_plaza(t, h, asentamiento);
        }
        self.emitir_propio(t, "ejercito.guarnece", vec![asentamiento], vec![("ejercito", id as i64), ("escuadrones", escuadrones as i64), ("caravanasAparcadas", adjuntas.len() as i64), ("heroes", antes.len() as i64)]);
        Ok(vec![])
    }

    /// `entrarEnCampamento` (Presencia §5.1): quien va solo en una columna personal entra; un ejército (o una personal con compañía) entra entero,
    /// ordenado por su Líder, se desarma y cada héroe entra como lo haría solo.
    fn entrar_en_el_campamento(&mut self, t: Instante, p: &Peticion, campamento: u32, heroe: u64) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let puerta = self.puerta_de_campamento(campamento).ok_or_else(|| rechazo(rechazos::CAMPAMENTO_DESCONOCIDO, format!("campamento-{campamento}")))?;
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
        // Las caravanas comerciales son de una plaza, nunca de un campamento: con alguna enganchada no se entra (la Caravana de Fundación,
        // que sí podría, todavía no es una caravana de la flota).
        if let Some(&c) = e.caravanas_adjuntas.first() {
            return Err(rechazo(rechazos::CARAVANA_AJENA_AL_LUGAR, format!("la caravana {c} no es de este campamento: hay que soltarla o llevarla a su origen antes de entrar")));
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
