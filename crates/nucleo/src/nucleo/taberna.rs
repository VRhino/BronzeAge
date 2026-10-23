//! La intel de las tabernas (Niebla_De_Guerra §6, Doc 5.12.10; 4H T3d): comprar una **Mirada** (un ojo prestado sobre un punto del mapa durante un rato, que ve y no explora) o el **Informe** de una
//! plaza ajena (la foto, fechada, de sus edificios, sus recintos y su defensa). Los dos son exactos en el instante de la compra y quedan en la memoria de la Facción (`memoria::Memoria`), que la `VISTA`
//! enseña: la Mirada mientras dura suma a lo que se ve (de la Facción y de sus aliados); el Informe se recuerda con su fecha, como una ficha.
//!
//! Dónde se compra y con qué oro: en la taberna de una plaza de la Facción, con el oro de su almacén; en la de un campamento de mercenarios, con el del héroe (el de su almacén personal si reside
//! allí; si no, el del carro de su columna). Quien compra tiene que estar dentro del lugar (o con su columna a la puerta).
//!
//! DUDA: en una plaza, Niebla_De_Guerra §6.1 reserva la compra al Rey, el Embajador o el Gobernador presentes; los cargos son del sistema 5 y, como las demás órdenes de plaza, hoy la puede dar
//! cualquier ciudadano presente.

use super::*;
use crate::memoria::{EdificioVisto, InformeDePlaza, Mirada, OrigenIntel, Sitio};
use asentamiento::catalogo::Tipo;
use asentamiento::edificio::Estado as EstadoEdificio;
use asentamiento::Recurso;
use comercio::de;
use militar::heroe::Residencia;
use militar::presencia::Ubicacion;

/// De dónde sale el oro de una compra.
enum Bolsa {
    Plaza(u64),
    AlmacenPersonal(u64),
    Carro(u64),
}

/// Quién compra y dónde: la Facción, la taberna (su sitio y cuántas Miradas admite) y de dónde se paga.
struct Compra {
    faccion: u32,
    origen: OrigenIntel,
    posicion: Punto,
    cupo: u32,
    bolsa: Bolsa,
}

/// Precio en oro (milésimas) redondeado hacia arriba a oro entero, como el `Math.ceil` del documento.
fn oro_entero(mil: Mil) -> Mil {
    (mil.max(0) + 999) / 1_000 * 1_000
}

impl Nucleo {
    /// Las Miradas que ve la Facción `f` en `t` (las suyas abiertas; las de sus aliados las suma quien proyecta).
    pub fn miradas_activas(&self, f: u32, t: Instante) -> impl Iterator<Item = &Mirada> {
        self.estado.memoria.miradas.iter().filter(move |m| m.faccion == f && m.activa(t))
    }

    /// ¿Conoce la Facción `f` la plaza `id`? La tiene en su memoria, la vio una columna suya que aún no lo grabó, o cae en una Mirada suya abierta (§6.2).
    fn conoce_plaza(&self, f: u32, id: u64, t: Instante) -> bool {
        let Some(a) = self.estado.asentamientos.get(&id) else { return false };
        let s = Sitio::Plaza(id);
        self.estado.memoria.fichas.get(&f).is_some_and(|de| de.contains_key(&s))
            || self.miradas_activas(f, t).any(|m| exploracion::cerca(m.centro, a.posicion, m.radio))
            || self.estado.ejercitos.values().filter(|e| e.columna.faccion == Some(f)).filter_map(|e| self.recorrido_pendiente(e, t)).any(|(r, radio)| self.avistados_pendientes(&r, radio).iter().any(|(x, _, _)| *x == s))
    }

    /// Quién compra en la taberna de `origen`: el héroe, ciudadano de una Facción, dentro del lugar (o con su columna a la puerta).
    fn resolver_compra(&self, t: Instante, p: &Peticion, heroe: u64, origen: OrigenIntel) -> Result<Compra, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        self.autorizar_heroe(p, heroe, false)?;
        let h = self.heroe(heroe).expect("autorizado");
        let faccion = h.faccion.ok_or_else(|| rechazo(rechazos::INTEL_SIN_FACCION, "la intel se compra para una Facción: hay que pertenecer a una".into()))?;
        let a_la_puerta = |puerta: Punto| self.estado.ejercito_de_heroe(heroe).and_then(|c| self.estado.ejercitos.get(&c)).is_some_and(|e| e.posicion_en(t).dist(puerta) <= self.radio_puerta());
        match origen {
            OrigenIntel::Plaza(x) => {
                let a = self.plaza_o_rechazo(x)?;
                if a.faccion != faccion {
                    return Err(rechazo(rechazos::INTEL_PLAZA_AJENA, "la taberna es de una plaza de otra Facción".into()));
                }
                if h.heroe.presencia.ubicacion != Ubicacion::Asentamiento(x as u32) && !a_la_puerta(a.posicion) {
                    return Err(rechazo(rechazos::INTEL_NO_PRESENTE, "hay que estar en la plaza para comprar en su taberna".into()));
                }
                let catalogo = &self.estado.reglas.economia.datos.catalogo;
                let taberna = a.economia.edificios.iter().find(|b| b.tipo == Tipo::Taberna && b.estado == EstadoEdificio::Activo);
                let taberna = taberna.ok_or_else(|| rechazo(rechazos::INTEL_SIN_TABERNA, "esta plaza no tiene Taberna".into()))?;
                let cupo = catalogo.tipo(Tipo::Taberna).nivel(taberna.nivel_interno).map_or(0, |n| n.cupo_miradas);
                Ok(Compra { faccion, origen, posicion: a.posicion, cupo, bolsa: Bolsa::Plaza(x) })
            }
            OrigenIntel::Campamento(c) => {
                let puerta = self.puerta_de_campamento(c).ok_or_else(|| rechazo(rechazos::CAMPAMENTO_DESCONOCIDO, ids::campamento(c)))?;
                if !self.esta_en_el_campamento(heroe, c, t) {
                    return Err(rechazo(rechazos::INTEL_NO_PRESENTE, "hay que estar en el campamento para comprar en su taberna".into()));
                }
                let bolsa = if h.heroe.residencia == Some(Residencia::Campamento(c)) {
                    Bolsa::AlmacenPersonal(heroe)
                } else {
                    Bolsa::Carro(self.estado.ejercito_de_heroe(heroe).ok_or_else(|| rechazo(rechazos::HEROE_SIN_COLUMNA, "de visita se paga con el carro de la columna: el héroe no va en ninguna".into()))?)
                };
                Ok(Compra { faccion, origen, posicion: puerta, cupo: self.estado.reglas.economia.militar.intel.cupo_campamento, bolsa })
            }
        }
    }

    /// Cobra `precio` (milésimas de oro) de la bolsa de la compra, o rechaza sin tocar nada si no llega.
    fn cobrar(&mut self, bolsa: &Bolsa, precio: Mil) -> Result<(), Respuesta> {
        let v = self.estado.version;
        let falta = |hay: Mil| Respuesta::rechazo(rechazos::INTEL_SIN_ORO, format!("cuesta {} de oro y hay {}", precio / 1_000, hay / 1_000), v);
        match *bolsa {
            Bolsa::Plaza(x) => {
                let almacen = &mut self.estado.asentamientos.get_mut(&x).expect("existe").economia.almacen;
                let hay = almacen.cantidad(Recurso::Oro);
                if hay < precio {
                    return Err(falta(hay));
                }
                almacen.quitar(Recurso::Oro, precio);
                self.tr.tocar(Clase::Asentamiento, x);
            }
            Bolsa::AlmacenPersonal(h) | Bolsa::Carro(h) => {
                let carga = match *bolsa {
                    Bolsa::Carro(c) => {
                        self.tr.tocar(Clase::Ejercito, c);
                        &mut self.estado.ejercitos.get_mut(&c).expect("existe").columna.carga
                    }
                    _ => &mut self.estado.militar.heroes.get_mut(&h).expect("existe").almacen_personal,
                };
                let hay = de(carga, Recurso::Oro);
                if hay < precio {
                    return Err(falta(hay));
                }
                *carga.entry(Recurso::Oro).or_insert(0) -= precio;
                carga.retain(|_, n| *n > 0);
            }
        }
        Ok(())
    }

    /// `ComprarMirada` y `ComprarInformePlaza`. `Ok(Some(id))` = la Mirada comprada; `Ok(None)` = el Informe; los rechazos no tocan nada.
    pub(super) fn ejecutar_taberna(&mut self, t: Instante, p: &Peticion) -> Result<Option<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: &str| Respuesta::rechazo(c, m, v);
        let ri = self.estado.reglas.economia.militar.intel.clone();
        match p.comando {
            Comando::ComprarMirada { heroe, origen, centro } => {
                let compra = self.resolver_compra(t, p, heroe, origen)?;
                let (ancho, alto) = self.geografia.as_ref().map_or((0, 0), |g| g.tamano());
                if centro.x < 0 || centro.y < 0 || centro.x > ancho || centro.y > alto {
                    return Err(rechazo(rechazos::INTEL_FUERA_DEL_MAPA, "esa zona queda fuera del mapa"));
                }
                if compra.cupo == 0 {
                    return Err(rechazo(rechazos::INTEL_SIN_CUPO, "esta taberna no admite Miradas"));
                }
                let propias: Vec<&Mirada> = self.estado.memoria.miradas.iter().filter(|m| m.faccion == compra.faccion && m.libre_en > t).collect();
                if propias.iter().filter(|m| m.origen == compra.origen && m.activa(t)).count() >= compra.cupo as usize {
                    return Err(rechazo(rechazos::INTEL_CUPO_LLENO, "esta taberna ya tiene abiertas todas las Miradas que admite"));
                }
                if propias.iter().any(|m| m.centro.dist(centro) < ri.radio_mirada) {
                    return Err(rechazo(rechazos::INTEL_ZONA_RECIENTE, "esa zona se ha mirado hace poco: hay que esperar a que se enfríe"));
                }
                // Mirar lejos cuesta más: la distancia a los ojos propios más cercanos (la taberna, las plazas y las columnas de la Facción).
                let mut ojos = vec![compra.posicion];
                ojos.extend(self.plazas_de(compra.faccion).map(|a| a.posicion));
                ojos.extend(self.estado.ejercitos.values().filter(|e| e.columna.faccion == Some(compra.faccion)).map(|e| e.posicion_en(t)));
                let lejos = ojos.iter().map(|o| o.dist(centro)).min().unwrap_or(0);
                let precio = oro_entero(ri.oro_base + ((lejos as i128 * ri.oro_por_unidad as i128 + 999) / 1_000) as Mil);
                self.cobrar(&compra.bolsa, precio)?;
                let id = self.nuevo_id();
                let expira_en = t + ri.duracion_mirada;
                let mirada = Mirada { id, faccion: compra.faccion, origen: compra.origen, centro, radio: ri.radio_mirada, comprada_en: t, expira_en, libre_en: expira_en + ri.enfriamiento_mirada };
                let miradas = &mut self.estado.memoria.miradas;
                miradas.retain(|m| m.libre_en > t);
                // No avisa a nadie (§6.1).
                miradas.push(mirada);
                Ok(Some(id))
            }
            Comando::ComprarInformePlaza { heroe, origen, asentamiento } => {
                let compra = self.resolver_compra(t, p, heroe, origen)?;
                let objetivo = self.plaza_o_rechazo(asentamiento)?;
                if objetivo.faccion == compra.faccion {
                    return Err(rechazo(rechazos::INTEL_PLAZA_PROPIA, "esa plaza es de tu Facción"));
                }
                if !self.conoce_plaza(compra.faccion, asentamiento, t) {
                    return Err(rechazo(rechazos::INTEL_PLAZA_DESCONOCIDA, "no conoces esa plaza: hay que haberla visto antes"));
                }
                let previo = self.estado.memoria.informes.get(&compra.faccion).and_then(|de| de.get(&asentamiento));
                if previo.is_some_and(|i| i.conocido_en + ri.enfriamiento_informe > t) {
                    return Err(rechazo(rechazos::INTEL_INFORME_RECIENTE, "ya tienes un informe reciente de esa plaza"));
                }
                let precio = ri.oro_por_nivel * i64::from(objetivo.nivel());
                let informe = self.levantar_informe(asentamiento, t);
                self.cobrar(&compra.bolsa, precio)?;
                self.estado.memoria.informes.entry(compra.faccion).or_default().insert(asentamiento, informe);
                // A la plaza espiada: el aviso es para su Facción, sin decir quién.
                self.emitir_propio(t, "asentamiento.informe_pedido", vec![asentamiento], vec![]);
                Ok(None)
            }
            _ => unreachable!("solo las compras de la taberna"),
        }
    }

    /// La foto de una plaza ajena para un Informe: sus edificios levantados y en obra (con su sitio en la ciudad) y su defensa (la misma que ve `inspeccionar`).
    fn levantar_informe(&self, id: u64, t: Instante) -> InformeDePlaza {
        let a = &self.estado.asentamientos[&id];
        let ciudad = self.vista_ciudad(id);
        let edificios = a
            .economia
            .edificios
            .iter()
            .filter(|b| b.estado != EstadoEdificio::EnCola)
            .map(|b| EdificioVisto { tipo: b.tipo, en_obra: b.estado == EstadoEdificio::EnConstruccion, nivel_interno: b.nivel_interno, sitio: ciudad.as_ref().and_then(|c| c.piezas.iter().find(|x| x.id == b.id)).map(|x| x.centro) })
            .collect();
        InformeDePlaza { nivel: a.nivel(), posicion: a.posicion, conocido_en: t, edificios, defensa: self.defensa_vista(id) }
    }
}
