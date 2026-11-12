//! La Armería y los consumibles del héroe dentro del núcleo (Heroe_Y_Tropa §4.6; entradas 70 y 73): `comprar_objeto`, `vender_objeto`, `forjar_objeto` y `usar_objeto`. Las
//! reglas son del crate `objetos` (`tienda::comprar`, `tienda::vender`, `forja::forjar`, `consumible::usar` y `entregar`, que no cambian nada si rechazan); aquí están dónde se
//! puede (D1 y D2), las monedas, los materiales del almacén personal, el efecto de lo usado, los rechazos y los hechos.
//!
//! - **Comprar y vender** (D1): dentro de cualquier plaza con la Armería activa; el metal que se compra lo abre la tecnología de la Facción **de esa plaza**. Pagan y cobran las
//!   monedas del héroe.
//! - **Forjar** (D1): en la Armería de la plaza donde reside, estando dentro. El plano y los crisoles salen de su mochila y los materiales de su almacén personal; la calidad, del
//!   generador del estado, y la pieza lleva `obj-N` del contador de ids del estado, que solo se gasta si la forja se acepta. Todo o nada.
//! - **Usar** (D2): donde se equipa. Una medalla de tropa, sobre una escuadra suya que no sea prestada ni esté en el nivel máximo, sin el bonus de Ejército permanente (lo que sobra
//!   al llegar al máximo se pierde); una del héroe, su porcentaje de lo que pide su nivel para el siguiente; el cofre, una montura que llega como lo que llega sin pedirlo (D17 y
//!   D22: a la mochila, si no cabe al almacén de objetos y si tampoco se pierde).

use super::a_mano::quitar;
use super::equipo::rechazo_de_inventario;
use super::*;
use crate::estado::Dato;
use asentamiento::edificio::Edificios;
use asentamiento::{Recurso, Tipo as Edificio};
use base::UNIDAD;
use comercio::de;
use militar::heroe::{experiencia_para_nivel, Heroe, Residencia};
use militar::presencia::Ubicacion;
use objetos::consumible::{usar, Efecto, ErrorUso};
use objetos::datos::{Consumo, Tipo};
use objetos::forja::{forjar, ErrorForja};
use objetos::tienda::{comprar, vender, ErrorTienda};
use objetos::{entregar, Destino, Pieza};

/// Forjar fuera de la plaza donde reside (D1).
const FUERA_DE_LA_FORJA: (&str, &str) = (rechazos::FORJA_FUERA_DE_TU_RESIDENCIA, "se forja en la Armería de la plaza donde resides, estando dentro");

fn rechazo_de_tienda(e: ErrorTienda) -> (&'static str, &'static str) {
    use rechazos::*;
    match e {
        ErrorTienda::Inventario(e) => rechazo_de_inventario(e, MOCHILA_LLENA),
        ErrorTienda::Desconocido => (OBJETO_DESCONOCIDO, "el objeto no está en el catálogo"),
        ErrorTienda::NoSeVende => (ARMERIA_NO_SE_VENDE, "la Armería no vende ese objeto"),
        ErrorTienda::MetalCerrado => (ARMERIA_SIN_TECNOLOGIA, "la Facción de esta plaza no tiene la tecnología de su metal"),
        ErrorTienda::SinMonedas => (HEROE_SIN_MONEDAS, "no tienes las monedas que cuesta"),
    }
}

fn rechazo_de_forja(e: ErrorForja) -> (&'static str, &'static str) {
    use rechazos::*;
    match e {
        ErrorForja::Inventario(e) => rechazo_de_inventario(e, MOCHILA_LLENA),
        ErrorForja::Desconocido => (OBJETO_DESCONOCIDO, "el objeto no está en el catálogo"),
        ErrorForja::SinReceta => (FORJA_SIN_RECETA, "ese objeto no se forja"),
        ErrorForja::MetalCerrado => (ARMERIA_SIN_TECNOLOGIA, "la Facción de esta plaza no tiene la tecnología de su metal"),
        ErrorForja::SinPlano => (FORJA_SIN_PLANO, "no llevas en la mochila un plano de su tier"),
        ErrorForja::SinCrisoles => (FORJA_SIN_CRISOLES, "no llevas en la mochila los crisoles de su tier"),
    }
}

fn rechazo_de_uso(e: ErrorUso) -> (&'static str, &'static str) {
    use rechazos::*;
    match e {
        ErrorUso::Inventario(e) => rechazo_de_inventario(e, MOCHILA_LLENA),
        ErrorUso::Desconocido => (OBJETO_DESCONOCIDO, "el objeto no está en el catálogo"),
        ErrorUso::NoConsumible => (OBJETO_NO_SE_USA, "solo se usa un consumible"),
    }
}

/// La escuadra del héroe que recibe una medalla de tropa (Catalogo_De_Objetos §3.3): suya, no prestada y por debajo del nivel máximo. Se mira antes de gastar la medalla.
fn escuadra_para_medalla(r: &militar::Reglas, h: &Heroe, escuadra: Option<u64>) -> Result<u64, (&'static str, &'static str)> {
    let e = escuadra.and_then(|id| h.escuadra(id)).ok_or((rechazos::ESCUADRA_AJENA, "una medalla de tropa se usa sobre una escuadra tuya"))?;
    if e.prestada.is_some() {
        return Err((rechazos::ESCUADRA_PRESTADA, "la tropa prestada no es tuya: no gana experiencia"));
    }
    if e.nivel >= r.militar.nivel_maximo {
        return Err((rechazos::ESCUADRA_NIVEL_MAXIMO, "la escuadra ya está en el nivel máximo"));
    }
    Ok(e.id)
}

impl Nucleo {
    pub(super) fn ejecutar_armeria(&mut self, t: Instante, p: &Peticion) -> Result<(), Respuesta> {
        let v = self.estado.version;
        let rechazo = |(c, m): (&'static str, &'static str)| Respuesta::rechazo(c, m, v);
        let mr = self.estado.reglas.economia.militar.clone();
        let cat = &mr.objetos;
        match &p.comando {
            Comando::ComprarObjeto { heroe, objeto } => {
                let plaza = self.armeria_donde_esta(p, *heroe)?;
                let faccion = self.estado.asentamientos[&plaza].faccion;
                let h = &self.heroe(*heroe).expect("existe").heroe;
                let (mut mochila, mut monedas) = (h.mochila.clone(), h.monedas);
                comprar(&mut mochila, cat, objeto, &mut monedas, |tec| self.estado.tiene_tecnologia(faccion, tec)).map_err(|e| rechazo(rechazo_de_tienda(e)))?;
                let precio = h.monedas - monedas;
                let h = &mut self.heroe_mut(*heroe).heroe;
                (h.mochila, h.monedas) = (mochila, monedas);
                self.hecho_de_objeto(t, "heroe.objeto_comprado", vec![("heroe", *heroe as i64), ("plaza", plaza as i64), ("precio", precio)], objeto, vec![]);
            }
            Comando::VenderObjeto { heroe, casilla, cantidad } => {
                let plaza = self.armeria_donde_esta(p, *heroe)?;
                let h = &mut self.heroe_mut(*heroe).heroe;
                let objeto = h.mochila.casillas.get(casilla).map(|c| c.pieza.objeto.clone());
                let cobro = vender(&mut h.mochila, cat, *casilla, *cantidad, &mut h.monedas).map_err(|e| rechazo(rechazo_de_tienda(e)))?;
                let objeto = objeto.expect("se vendió de una casilla con pieza");
                let datos = vec![("heroe", *heroe as i64), ("plaza", plaza as i64), ("cantidad", i64::from(*cantidad)), ("cobro", cobro)];
                self.hecho_de_objeto(t, "heroe.objeto_vendido", datos, &objeto, vec![]);
            }
            Comando::ForjarObjeto { heroe, objeto } => {
                let plaza = match self.exigir_dentro_de_su_residencia(p, *heroe, FUERA_DE_LA_FORJA)? {
                    Residencia::Asentamiento(a) if self.con_armeria(u64::from(a)) => u64::from(a),
                    Residencia::Asentamiento(_) | Residencia::Campamento(_) => return Err(rechazo((rechazos::ARMERIA_SIN_ARMERIA, "no hay una Armería activa donde resides"))),
                };
                let faccion = self.estado.asentamientos[&plaza].faccion;
                let (id, hn) = (self.estado.siguiente_id, self.heroe(*heroe).expect("existe"));
                let (mut mochila, mut rng) = (hn.heroe.mochila.clone(), self.estado.rng.clone());
                let (materiales, calidad) = forjar(&mut mochila, cat, objeto, id, &mut rng, |tec| self.estado.tiene_tecnologia(faccion, tec)).map_err(|e| rechazo(rechazo_de_forja(e)))?;
                // Los materiales de las recetas son recursos del motor (lo comprueba `pruebas::los_materiales_de_las_recetas_son_recursos_del_motor`), en unidades.
                let materiales: Vec<(Recurso, Mil)> = materiales.iter().map(|(r, n)| (Recurso::desde_nombre(r).expect("un material de receta es un recurso del motor"), n * UNIDAD)).collect();
                if materiales.iter().any(|&(r, n)| de(&hn.almacen_personal, r) < n) {
                    return Err(rechazo((rechazos::FORJA_SIN_MATERIALES, "tu almacén personal no tiene los materiales de la receta")));
                }
                (self.estado.siguiente_id, self.estado.rng) = (id + 1, rng);
                let hn = self.heroe_mut(*heroe);
                hn.heroe.mochila = mochila;
                for (r, n) in materiales {
                    quitar(&mut hn.almacen_personal, r, n);
                }
                self.hecho_de_objeto(t, "heroe.objeto_forjado", vec![("heroe", *heroe as i64), ("plaza", plaza as i64), ("pieza", id as i64), ("calidad", calidad)], objeto, vec![]);
            }
            Comando::UsarObjeto { heroe, casilla, escuadra } => {
                self.exigir_lugar_de_equipar(p, *heroe)?;
                let h = &self.heroe(*heroe).expect("existe").heroe;
                let objeto = h.mochila.casillas.get(casilla).map(|c| c.pieza.objeto.clone());
                let de_tropa = matches!(objeto.as_ref().and_then(|o| cat.objetos.get(o)).map(|f| &f.tipo), Some(Tipo::Consumible(Consumo::ExperienciaTropa(_))));
                let objetivo = if de_tropa { Some(escuadra_para_medalla(&mr, h, *escuadra).map_err(rechazo)?) } else { None };
                let nivel = h.nivel;
                self.tr.tocar(Clase::Heroe, *heroe);
                let Estado { militar, rng, .. } = &mut self.estado;
                let h = &mut militar.heroes.get_mut(heroe).expect("existe").heroe;
                let efecto = usar(&mut h.mochila, cat, *casilla, rng).map_err(|e| rechazo(rechazo_de_uso(e)))?;
                let objeto = objeto.expect("se usó de una casilla con pieza");
                let mut datos = vec![("heroe", *heroe as i64)];
                let mut valores = vec![];
                match efecto {
                    Efecto::ExperienciaTropa(x) => {
                        let id = objetivo.expect("la escuadra de la medalla de tropa se comprobó antes de usarla");
                        let mut e = h.escuadra(id).expect("comprobada").clone();
                        self.sumar_experiencia_de_escuadra(t, &mut e, x * UNIDAD);
                        *self.heroe_mut(*heroe).heroe.escuadra_mut(id).expect("comprobada") = e;
                        self.tr.tocar(Clase::Escuadra, id);
                        datos.push(("escuadra", id as i64));
                    }
                    Efecto::ExperienciaHeroePct(pct) => {
                        let pide = experiencia_para_nivel(&mr, nivel + 1) - experiencia_para_nivel(&mr, nivel);
                        self.experiencia_de_heroe(t, *heroe, pide * pct / 100);
                    }
                    Efecto::Montura(montura) => {
                        let destino = entregar(&mut h.mochila, &mut h.almacen_objetos, cat, Pieza::del_catalogo(&montura), 1).expect("las monturas del cofre están en el catálogo (comprobado al cargar)");
                        let destino = match destino {
                            Destino::Mochila => "mochila",
                            Destino::Almacen => "almacen",
                            Destino::Perdido => "perdido",
                        };
                        valores = vec![("montura", Dato::Texto(montura)), ("destino", Dato::Texto(destino.into()))];
                    }
                }
                self.hecho_de_objeto(t, "heroe.objeto_usado", datos, &objeto, valores);
            }
            _ => unreachable!("solo se llama con las órdenes de la Armería y de los consumibles"),
        }
        Ok(())
    }

    /// ¿Tiene la plaza una Armería activa?
    fn con_armeria(&self, plaza: u64) -> bool {
        self.estado.asentamientos.get(&plaza).is_some_and(|a| a.economia.edificios.activos(Edificio::Armeria) > 0)
    }

    /// D1: la tienda de la Armería, dentro de una plaza que la tenga activa. Devuelve la plaza.
    fn armeria_donde_esta(&self, p: &Peticion, heroe: u64) -> Result<u64, Respuesta> {
        self.autorizar_heroe(p, heroe, false)?;
        match self.heroe(heroe).expect("existe").heroe.presencia.ubicacion {
            Ubicacion::Asentamiento(a) if self.con_armeria(u64::from(a)) => Ok(u64::from(a)),
            _ => Err(Respuesta::rechazo(rechazos::ARMERIA_SIN_ARMERIA, "se compra y se vende dentro de una plaza con la Armería activa", self.estado.version)),
        }
    }

    /// Un hecho del héroe con el id del catálogo del objeto (`objeto`, un texto) y lo demás que lleve.
    fn hecho_de_objeto(&mut self, t: Instante, codigo: &'static str, datos: Vec<(&'static str, i64)>, objeto: &str, mut valores: Vec<(&'static str, Dato)>) {
        valores.insert(0, ("objeto", Dato::Texto(objeto.to_owned())));
        self.tr.emitir(&self.estado, EventoNucleo { valores, ..EventoNucleo::nuevo(codigo, t, datos) });
    }
}

#[cfg(test)]
mod pruebas {
    //! Cada orden por su camino bueno y por cada rechazo, sobre un núcleo sin mundo con una plaza a mano (la 50, de la Facción 1, con su Armería activa) y lo que hace falta
    //! metido a mano en la mochila y en el almacén personal: ninguna orden da aún planos, crisoles ni consumibles. Las reglas de cada pieza las prueba el crate `objetos`; las
    //! órdenes de punta a punta, `tests/heroes.rs`.
    use super::*;
    use crate::estado::Asentamiento;
    use crate::estado_militar::HeroeNucleo;
    use crate::{Ajustes, Arranque, Reglas, Resultado};
    use asentamiento::edificio::Estado as EstadoEdificio;
    use contrato::v1::GrafoNavegacion;
    use militar::catalogo::Tropa;
    use militar::escuadra::Escuadra;
    use militar::heroe::Controlador;
    use tecnologia::contador::Contador;
    use objetos::Inventario;
    use rutas::Rutas;
    use std::collections::BTreeSet;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    const HEROE: u64 = 7;
    const PLAZA: u64 = 50;
    /// La Facción de la plaza; el héroe es de la 2 (D1: el metal lo abre la de la plaza).
    const FACCION: u32 = 1;
    const ESCUADRA: u64 = 20;

    /// Un héroe de la Facción 2 que reside dentro de la plaza 50 (Facción 1, con la Armería activa), con esas piezas en su mochila, de la casilla 0 en adelante.
    fn nucleo(semilla: u64, piezas: &[(&str, u32)]) -> Nucleo {
        let reglas = Reglas::default();
        let rutas = Arc::new(Rutas::desde_grafo(&GrafoNavegacion::default()));
        let mut n = Nucleo::nuevo(Arranque { reglas: reglas.clone(), ajustes: Ajustes::default(), semilla, inicio: 0, rutas, mundo_hash: "h".into(), mundo: None });
        let eco = &reglas.economia.datos;
        let mut economia = asentamiento::asentamiento::Asentamiento::fundar(eco, 0, &eco.materiales_iniciales, eco.viviendas_iniciales);
        economia.edificios.push(asentamiento::edificio::Edificio::nuevo(99, Edificio::Armeria, EstadoEdificio::Activo));
        let urbe = crate::urbanismo::Urbe::fundar(&reglas.economia, semilla, PLAZA);
        n.estado.asentamientos.insert(PLAZA, Asentamiento { id: PLAZA, faccion: FACCION, posicion: base::Punto::new(0, 0), fundado_en: 0, radio: 0, parcelas: BTreeSet::new(), version: 0, economia, urbe, protegida_hasta: None, ocupada_hasta: None, puerta: Default::default(), cargos: Default::default(), nombre: String::new(), capital_de: None, de_la_faccion_desde: 0, resto_tributo: 0, permite_reabastecer_aliados: false, almacen_cerrado_a_ciudadanos: false, reclutamiento_cerrado_a_ciudadanos: false, fundadores: Vec::new() });
        let mr = &reglas.economia.militar;
        let mut h = Heroe::nuevo(mr, HEROE, Controlador::Humano, 0, 1, "espada_escudo_mediano").expect("una clase del catálogo");
        (h.residencia, h.presencia.ubicacion) = (Some(Residencia::Asentamiento(PLAZA as u32)), Ubicacion::Asentamiento(PLAZA as u32));
        for (p, cantidad) in piezas {
            h.mochila.meter(&mr.objetos, Pieza::del_catalogo(p), *cantidad).unwrap();
        }
        h.escuadras.push(Escuadra::nueva(mr, ESCUADRA, HEROE, Tropa::MiliciaLanceros, 10, None, 0));
        n.estado.militar.insertar(HeroeNucleo::nuevo(h, "ana", Some(2)));
        n
    }

    fn heroe(n: &mut Nucleo) -> &mut HeroeNucleo {
        n.estado.militar.heroes.get_mut(&HEROE).unwrap()
    }

    fn abrir(n: &mut Nucleo, faccion: u32, tecnologias: &[&str]) {
        n.fijar_tecnologias(0, faccion, &tecnologias.iter().map(|t| t.to_string()).collect::<Vec<_>>()).unwrap();
    }

    /// La orden de ana: `Ok` si se acepta; si se rechaza, el código, y el estado queda como estaba.
    fn orden(n: &mut Nucleo, c: Comando) -> Result<(), &'static str> {
        static ID: AtomicU64 = AtomicU64::new(0);
        let (hash, version) = (n.hash_estado(), n.version());
        let r = n.aplicar_en(0, Peticion::nueva("ana", &format!("o{}", ID.fetch_add(1, Ordering::Relaxed)), c));
        match r.resultado {
            Resultado::Aceptada => Ok(()),
            Resultado::Rechazada { codigo, .. } => {
                assert_eq!((n.hash_estado(), n.version()), (hash, version), "un rechazo no cambia nada ({codigo})");
                Err(codigo)
            }
            otro => panic!("{otro:?}"),
        }
    }

    fn hechos(n: &Nucleo, codigo: &str) -> Vec<EventoNucleo> {
        n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter()).filter(|e| e.codigo == codigo).cloned().collect()
    }

    fn texto<'a>(e: &'a EventoNucleo, clave: &str) -> &'a str {
        match e.valores.iter().find(|(k, _)| *k == clave) {
            Some((_, Dato::Texto(s))) => s,
            otro => panic!("el hecho no lleva el texto «{clave}»: {otro:?}"),
        }
    }

    fn objetos_de(m: &Inventario) -> Vec<(u32, &str, u32)> {
        m.casillas.iter().map(|(n, c)| (*n, c.pieza.objeto.as_str(), c.cantidad)).collect()
    }

    fn comprar(objeto: &str) -> Comando {
        Comando::ComprarObjeto { heroe: HEROE, objeto: objeto.into() }
    }

    fn forjar(objeto: &str) -> Comando {
        Comando::ForjarObjeto { heroe: HEROE, objeto: objeto.into() }
    }

    fn usar(casilla: u32, escuadra: Option<u64>) -> Comando {
        Comando::UsarObjeto { heroe: HEROE, casilla, escuadra }
    }

    /// Los materiales de la receta de `objeto`, en milésimas de recurso.
    fn receta(n: &Nucleo, objeto: &str) -> Vec<(Recurso, Mil)> {
        let (Tipo::Arma { combate, .. } | Tipo::Armadura { combate, .. }) = &n.estado.reglas.economia.militar.objetos.objetos[objeto].tipo else { panic!("arma o armadura") };
        combate.forja.as_ref().unwrap().materiales.iter().map(|(r, x)| (Recurso::desde_nombre(r).unwrap(), x * UNIDAD)).collect()
    }

    /// Lo que pide la forja de `objeto`, a la puerta: el plano y los crisoles en la mochila (los da `nucleo`) y los materiales en el almacén personal.
    fn con_materiales(n: &mut Nucleo, objeto: &str) {
        for (r, x) in receta(n, objeto) {
            comercio::sumar(&mut heroe(n).almacen_personal, r, x);
        }
    }

    #[test]
    fn los_materiales_de_las_recetas_son_recursos_del_motor() {
        let cat = &Reglas::default().economia.militar.objetos;
        let mut recetas = 0;
        for (id, f) in &cat.objetos {
            if let Tipo::Arma { combate, .. } | Tipo::Armadura { combate, .. } = &f.tipo {
                for (r, _) in combate.forja.iter().flat_map(|x| &x.materiales) {
                    assert!(Recurso::desde_nombre(r).is_some(), "{id}: «{r}» no es un recurso del motor");
                    recetas += 1;
                }
            }
        }
        assert!(recetas > 100, "{recetas} materiales");
    }

    #[test]
    fn comprar_en_la_armeria_con_las_monedas_del_heroe_y_el_metal_de_la_faccion_de_la_plaza() {
        let mut n = nucleo(1, &[]);
        assert_eq!(orden(&mut n, comprar("montura_estepa")), Ok(()), "una montura no pide tecnología");
        assert_eq!(orden(&mut n, comprar("espada_de_madera")), Err(rechazos::OBJETO_DESCONOCIDO));
        assert_eq!(orden(&mut n, comprar("crisol")), Err(rechazos::ARMERIA_NO_SE_VENDE));
        assert_eq!(orden(&mut n, comprar("dagas_bronce")), Err(rechazos::ARMERIA_NO_SE_VENDE), "tier III");
        abrir(&mut n, 2, &["metalurgia_cobre"]);
        assert_eq!(orden(&mut n, comprar("dagas_cobre")), Err(rechazos::ARMERIA_SIN_TECNOLOGIA), "la del héroe no cuenta");
        abrir(&mut n, FACCION, &["metalurgia_cobre"]);
        assert_eq!(orden(&mut n, comprar("dagas_cobre")), Ok(()));
        let precio = n.estado.reglas.economia.militar.objetos.objetos["dagas_cobre"].precio;
        let h = &heroe(&mut n).heroe;
        assert_eq!((h.monedas, objetos_de(&h.mochila)), (1_500 - 600 - precio, vec![(0, "montura_estepa", 1), (1, "dagas_cobre", 1)]));
        let e = hechos(&n, "heroe.objeto_comprado");
        assert_eq!(e.iter().map(|e| (texto(e, "objeto"), e.dato("heroe"), e.dato("plaza"), e.dato("precio"))).collect::<Vec<_>>(), vec![("montura_estepa", Some(HEROE as i64), Some(PLAZA as i64), Some(600)), ("dagas_cobre", Some(HEROE as i64), Some(PLAZA as i64), Some(precio))]);
        heroe(&mut n).heroe.monedas = precio - 1;
        assert_eq!(orden(&mut n, comprar("dagas_cobre")), Err(rechazos::HEROE_SIN_MONEDAS));
        heroe(&mut n).heroe.monedas = 10_000;
        heroe(&mut n).heroe.mochila.capacidad = 2;
        assert_eq!(orden(&mut n, comprar("dagas_cobre")), Err(rechazos::MOCHILA_LLENA), "sin cobrar (D17)");
        // Sin Armería activa, o fuera de una plaza, no hay tienda.
        heroe(&mut n).heroe.presencia.ubicacion = Ubicacion::Mercenarios(0);
        assert_eq!(orden(&mut n, comprar("montura_tiro")), Err(rechazos::ARMERIA_SIN_ARMERIA));
        heroe(&mut n).heroe.presencia.ubicacion = Ubicacion::Asentamiento(PLAZA as u32);
        n.estado.asentamientos.get_mut(&PLAZA).unwrap().economia.edificios.last_mut().unwrap().estado = EstadoEdificio::EnConstruccion;
        assert_eq!(orden(&mut n, comprar("montura_tiro")), Err(rechazos::ARMERIA_SIN_ARMERIA));
    }

    #[test]
    fn vender_a_la_armeria_cobra_la_mitad_de_la_ficha() {
        let mut n = nucleo(1, &[("medalla_tropa_muy_pequena", 5), ("torso_ligera_hierro", 1)]);
        assert_eq!(orden(&mut n, Comando::VenderObjeto { heroe: HEROE, casilla: 0, cantidad: 3 }), Ok(()));
        assert_eq!(orden(&mut n, Comando::VenderObjeto { heroe: HEROE, casilla: 1, cantidad: 1 }), Ok(()));
        let h = &heroe(&mut n).heroe;
        assert_eq!((h.monedas, objetos_de(&h.mochila)), (1_500 + 6 + 262, vec![(0, "medalla_tropa_muy_pequena", 2)]), "2 cada medalla (D19) y 525 al 50 %, hacia abajo");
        let e = hechos(&n, "heroe.objeto_vendido");
        assert_eq!(e.iter().map(|e| (texto(e, "objeto"), e.dato("cantidad"), e.dato("cobro"))).collect::<Vec<_>>(), vec![("medalla_tropa_muy_pequena", Some(3), Some(6)), ("torso_ligera_hierro", Some(1), Some(262))]);
        assert_eq!(orden(&mut n, Comando::VenderObjeto { heroe: HEROE, casilla: 1, cantidad: 1 }), Err(rechazos::INVENTARIO_CASILLA_VACIA));
        assert_eq!(orden(&mut n, Comando::VenderObjeto { heroe: HEROE, casilla: 0, cantidad: 3 }), Err(rechazos::INVENTARIO_POCA_CANTIDAD));
        assert_eq!(orden(&mut n, Comando::VenderObjeto { heroe: HEROE, casilla: 0, cantidad: 0 }), Err(rechazos::INVENTARIO_CANTIDAD_INVALIDA));
        heroe(&mut n).heroe.presencia.ubicacion = Ubicacion::Pueblo(3);
        assert_eq!(orden(&mut n, Comando::VenderObjeto { heroe: HEROE, casilla: 0, cantidad: 1 }), Err(rechazos::ARMERIA_SIN_ARMERIA));
    }

    #[test]
    fn forjar_gasta_plano_crisoles_y_materiales_y_la_pieza_lleva_el_siguiente_id() {
        let mut n = nucleo(1, &[("plano_2", 1), ("crisol", 3)]);
        abrir(&mut n, FACCION, &["metalurgia_cobre"]);
        con_materiales(&mut n, "dagas_cobre");
        comercio::sumar(&mut heroe(&mut n).almacen_personal, Recurso::Madera, 7 * UNIDAD);
        let id = n.estado.siguiente_id;
        assert_eq!(orden(&mut n, forjar("dagas_cobre")), Ok(()));
        assert_eq!(n.estado.siguiente_id, id + 1);
        let h = heroe(&mut n);
        assert_eq!(objetos_de(&h.heroe.mochila), vec![(0, "dagas_cobre", 1), (1, "crisol", 1)], "el plano y dos crisoles, gastados; la pieza, en la casilla libre más baja");
        assert_eq!(h.heroe.mochila.casillas[&0].pieza.forjada.as_ref().map(|f| f.id), Some(id));
        assert_eq!(h.almacen_personal.clone().into_iter().collect::<Vec<_>>(), vec![(Recurso::Madera, 7 * UNIDAD)], "los materiales, del almacén personal; lo demás se queda");
        let e = &hechos(&n, "heroe.objeto_forjado")[0];
        assert_eq!((texto(e, "objeto"), e.dato("pieza"), e.dato("plaza")), ("dagas_cobre", Some(id as i64), Some(PLAZA as i64)));
        assert!(e.dato("calidad").is_some_and(|c| (90..=110).contains(&c)), "{e:?}");
    }

    #[test]
    fn forjar_rechaza_cada_falta_sin_gastar_nada_ni_el_id() {
        let mut n = nucleo(1, &[("plano_2", 1), ("crisol", 1)]);
        assert_eq!(orden(&mut n, forjar("dagas_cobre")), Err(rechazos::ARMERIA_SIN_TECNOLOGIA));
        abrir(&mut n, FACCION, &["metalurgia_cobre", "aleacion_bronce"]);
        assert_eq!(orden(&mut n, forjar("espada_de_madera")), Err(rechazos::OBJETO_DESCONOCIDO));
        assert_eq!(orden(&mut n, forjar("dagas_inicio")), Err(rechazos::FORJA_SIN_RECETA));
        assert_eq!(orden(&mut n, forjar("dagas_bronce")), Err(rechazos::FORJA_SIN_PLANO), "el plano es de otro tier");
        assert_eq!(orden(&mut n, forjar("dagas_cobre")), Err(rechazos::FORJA_SIN_CRISOLES));
        let cat = n.estado.reglas.economia.militar.objetos.clone();
        heroe(&mut n).heroe.mochila.meter(&cat, Pieza::del_catalogo("crisol"), 1).unwrap();
        assert_eq!(orden(&mut n, forjar("dagas_cobre")), Err(rechazos::FORJA_SIN_MATERIALES));
        con_materiales(&mut n, "dagas_cobre");
        let (cuero, _) = receta(&n, "dagas_cobre")[0];
        quitar(&mut heroe(&mut n).almacen_personal, cuero, 1);
        assert_eq!(orden(&mut n, forjar("dagas_cobre")), Err(rechazos::FORJA_SIN_MATERIALES), "una milésima de menos");
        con_materiales(&mut n, "dagas_cobre");
        let mochila = &mut heroe(&mut n).heroe.mochila;
        mochila.capacidad = 3;
        mochila.meter(&cat, Pieza::del_catalogo("plano_2"), 1).unwrap();
        mochila.meter(&cat, Pieza::del_catalogo("crisol"), 1).unwrap();
        mochila.meter(&cat, Pieza::del_catalogo("dagas_inicio"), 1).unwrap();
        assert_eq!(orden(&mut n, forjar("dagas_cobre")), Err(rechazos::MOCHILA_LLENA), "quedan un plano y un crisol: la pieza no cabe");
        heroe(&mut n).heroe.mochila.capacidad = 72;
        // Fuera de la plaza donde reside, o residiendo en un campamento, no se forja.
        heroe(&mut n).heroe.residencia = Some(Residencia::Campamento(0));
        assert_eq!(orden(&mut n, forjar("dagas_cobre")), Err(rechazos::FORJA_FUERA_DE_TU_RESIDENCIA));
        heroe(&mut n).heroe.presencia.ubicacion = Ubicacion::Mercenarios(0);
        assert_eq!(orden(&mut n, forjar("dagas_cobre")), Err(rechazos::ARMERIA_SIN_ARMERIA), "en su campamento no hay Armería");
        heroe(&mut n).heroe.residencia = None;
        assert_eq!(orden(&mut n, forjar("dagas_cobre")), Err(rechazos::HEROE_SIN_RESIDENCIA));
        (heroe(&mut n).heroe.residencia, heroe(&mut n).heroe.presencia.ubicacion) = (Some(Residencia::Asentamiento(PLAZA as u32)), Ubicacion::Asentamiento(PLAZA as u32));
        assert_eq!(orden(&mut n, forjar("dagas_cobre")), Ok(()), "con todo, sí");
    }

    #[test]
    fn la_forja_es_igual_con_la_misma_semilla_y_su_calidad_va_del_90_al_110() {
        let forjada = |semilla| {
            let mut n = nucleo(semilla, &[("plano_2", 1), ("crisol", 2)]);
            abrir(&mut n, FACCION, &["metalurgia_cobre"]);
            con_materiales(&mut n, "torso_media_cobre");
            orden(&mut n, forjar("torso_media_cobre")).unwrap();
            (heroe(&mut n).heroe.mochila.casillas[&0].pieza.clone(), hechos(&n, "heroe.objeto_forjado")[0].dato("calidad").unwrap())
        };
        assert_eq!(forjada(3), forjada(3));
        let calidades: BTreeSet<i64> = (0..60).map(|s| forjada(s).1).collect();
        assert!(calidades.len() > 5 && calidades.iter().all(|c| (90..=110).contains(c)), "{calidades:?}");
    }

    #[test]
    fn la_pieza_forjada_y_el_contador_de_ids_van_en_la_foto() {
        let mut n = nucleo(1, &[("plano_2", 1), ("crisol", 2)]);
        abrir(&mut n, FACCION, &["metalurgia_cobre"]);
        con_materiales(&mut n, "dagas_cobre");
        orden(&mut n, forjar("dagas_cobre")).unwrap();
        let vuelta = Estado::deserializar(&n.estado.serializar(), &Reglas::default(), "h").unwrap();
        assert_eq!(vuelta.siguiente_id, n.estado.siguiente_id);
        assert_eq!(vuelta.militar.heroes[&HEROE].heroe.mochila, heroe(&mut n).heroe.mochila);
    }

    #[test]
    fn la_medalla_de_tropa_da_su_experiencia_sin_el_bonus_a_una_escuadra_suya() {
        let mut n = nucleo(1, &[("medalla_tropa_grande", 3), ("medalla_tropa_muy_pequena", 1)]);
        abrir(&mut n, 2, &["ejercito_permanente"]);
        assert_eq!(orden(&mut n, usar(0, Some(ESCUADRA))), Ok(()));
        assert_eq!(heroe(&mut n).heroe.escuadra(ESCUADRA).unwrap().experiencia, 50_000, "sin el 10 % de Ejército permanente");
        let e = &hechos(&n, "heroe.objeto_usado")[0];
        assert_eq!((texto(e, "objeto"), e.dato("escuadra")), ("medalla_tropa_grande", Some(ESCUADRA as i64)));
        // Cruza el nivel 5 (185) y cuenta una veterana, como en combate.
        heroe(&mut n).heroe.escuadra_mut(ESCUADRA).unwrap().experiencia = 180_000;
        assert_eq!(orden(&mut n, usar(1, Some(ESCUADRA))), Ok(()));
        let x = heroe(&mut n).heroe.escuadra(ESCUADRA).unwrap().clone();
        assert_eq!((x.nivel, x.experiencia, n.tecnologia().contador(Contador::EscuadrasVeteranas)), (5, 185_000, 1));
        // Ajena, prestada o en el nivel máximo: la medalla no se gasta.
        assert_eq!(orden(&mut n, usar(0, None)), Err(rechazos::ESCUADRA_AJENA));
        assert_eq!(orden(&mut n, usar(0, Some(99))), Err(rechazos::ESCUADRA_AJENA));
        heroe(&mut n).heroe.escuadra_mut(ESCUADRA).unwrap().prestada = Some(0);
        assert_eq!(orden(&mut n, usar(0, Some(ESCUADRA))), Err(rechazos::ESCUADRA_PRESTADA));
        let e = heroe(&mut n).heroe.escuadra_mut(ESCUADRA).unwrap();
        (e.prestada, e.nivel) = (None, 10);
        assert_eq!(orden(&mut n, usar(0, Some(ESCUADRA))), Err(rechazos::ESCUADRA_NIVEL_MAXIMO));
        // A un paso del máximo, lo que sobra se pierde: llega al 10 y no pasa.
        let e = heroe(&mut n).heroe.escuadra_mut(ESCUADRA).unwrap();
        (e.nivel, e.experiencia) = (9, 1_374_000);
        assert_eq!(orden(&mut n, usar(0, Some(ESCUADRA))), Ok(()));
        assert_eq!(heroe(&mut n).heroe.escuadra(ESCUADRA).unwrap().nivel, 10);
        assert_eq!(heroe(&mut n).heroe.mochila.casillas[&0].cantidad, 1, "tres medallas, dos usadas");
    }

    #[test]
    fn la_medalla_del_heroe_da_su_porcentaje_de_lo_que_pide_su_nivel() {
        let mut n = nucleo(1, &[("medalla_heroe_pequena", 1), ("medalla_heroe_grande", 1)]);
        assert_eq!(orden(&mut n, usar(0, Some(ESCUADRA))), Ok(()), "la escuadra no la mira");
        assert_eq!(heroe(&mut n).heroe.experiencia, 10_000, "el 10 % de los 100 del nivel 1");
        let mr = n.estado.reglas.economia.militar.clone();
        let h = &mut heroe(&mut n).heroe;
        (h.nivel, h.experiencia) = (30, experiencia_para_nivel(&mr, 30));
        assert_eq!(orden(&mut n, usar(1, None)), Ok(()));
        assert_eq!(heroe(&mut n).heroe.experiencia, experiencia_para_nivel(&mr, 30) + 340_000, "desde el 30, el 50 % de 680");
    }

    #[test]
    fn el_cofre_da_una_montura_que_llega_a_la_mochila_al_almacen_o_se_pierde() {
        let mut n = nucleo(1, &[("cofre_monturas", 1), ("cofre_monturas", 2)]);
        let casillas = |n: &mut Nucleo| heroe(n).heroe.mochila.casillas.len();
        let monturas = ["montura_escita", "montura_tesalia", "montura_nisea"];
        heroe(&mut n).heroe.mochila.capacidad = 1;
        assert_eq!(orden(&mut n, usar(0, None)), Ok(()));
        let h = &heroe(&mut n).heroe;
        assert_eq!((h.mochila.casillas[&0].cantidad, h.almacen_objetos.casillas.len()), (2, 1), "con la pila en la mochila llena, al almacén de objetos");
        assert!(monturas.contains(&h.almacen_objetos.casillas[&0].pieza.objeto.as_str()));
        heroe(&mut n).heroe.almacen_objetos.capacidad = 1;
        assert_eq!(orden(&mut n, usar(0, None)), Ok(()));
        assert_eq!((casillas(&mut n), heroe(&mut n).heroe.almacen_objetos.casillas.len()), (1, 1), "sin sitio en ninguno, se pierde");
        assert_eq!(orden(&mut n, usar(0, None)), Ok(()));
        let m = &heroe(&mut n).heroe.mochila.casillas[&0];
        assert!(monturas.contains(&m.pieza.objeto.as_str()) && m.cantidad == 1, "el último cofre deja su casilla a la montura");
        let e = hechos(&n, "heroe.objeto_usado");
        assert_eq!(e.iter().map(|e| (texto(e, "objeto"), texto(e, "destino"))).collect::<Vec<_>>(), vec![("cofre_monturas", "almacen"), ("cofre_monturas", "perdido"), ("cofre_monturas", "mochila")]);
        assert!(e.iter().all(|e| monturas.contains(&texto(e, "montura"))));
    }

    #[test]
    fn usar_solo_un_consumible_y_donde_se_equipa() {
        let mut n = nucleo(1, &[("crisol", 1), ("medalla_heroe_pequena", 1)]);
        assert_eq!(orden(&mut n, usar(0, None)), Err(rechazos::OBJETO_NO_SE_USA));
        assert_eq!(orden(&mut n, usar(5, None)), Err(rechazos::INVENTARIO_CASILLA_VACIA));
        heroe(&mut n).heroe.presencia.ubicacion = Ubicacion::Columna(3);
        assert_eq!(orden(&mut n, usar(1, None)), Err(rechazos::EQUIPO_FUERA_DE_LUGAR));
        heroe(&mut n).heroe.presencia.ubicacion = Ubicacion::Pueblo(3);
        heroe(&mut n).heroe.escuadra_mut(ESCUADRA).unwrap().reserva_batalla = Some(1);
        assert_eq!(orden(&mut n, usar(1, None)), Err(rechazos::EQUIPO_EN_BATALLA));
        heroe(&mut n).heroe.escuadra_mut(ESCUADRA).unwrap().reserva_batalla = None;
        assert_eq!(orden(&mut n, usar(1, None)), Ok(()), "en un pueblo, sí");
    }
}
