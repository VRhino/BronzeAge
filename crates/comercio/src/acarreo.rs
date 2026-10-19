//! El carro de acarreo (Comercio_Caravanas, segunda §12; Doc 3.13.8): cada pueblo extractor tiene un carro propio, una caravana de tipo
//! `acarreo` ligada al pueblo, con **un carro básico y un buey** (500 de capacidad, velocidad 24) y una **ruta fija** pueblo ↔ plaza madre.
//!
//! - **Solo sale con el carro lleno**: el evento de salida se programa cuando el almacén local del pueblo alcanza la capacidad del carro
//!   (o al reponerse el carro con el almacén ya lleno). Vuelca en el almacén de la madre y vuelve vacío.
//! - **No cuenta** contra los cupos de caravanas ni de carros, ni exige Mercado, ni entra en el reparto automático ni en el trueque.
//! - **No le afectan** "Carga Ampliada" ni "Rutas Rápidas" (decidido el 2026-10-18): con ×1,5 de capacidad el carro de 750 superaría el
//!   almacén de 500 del pueblo y «sale lleno» no se dispararía nunca. Por eso las funciones de aquí usan siempre factor `UNO`.
//! - **Si se destruye**, se compra otro en la plaza madre solo si el pueblo no tiene ninguno, al coste de carro + buey; mientras no tenga,
//!   el pueblo acumula hasta llenar su almacén y la extracción para.

use crate::caravana::{Animal, Carro, Caravana, ErrorCaravana, EstadoCaravana, TipoCarro, TipoCaravana};
use crate::datos::ReglasFlota;
use crate::tipos::{AsentamientoId, CaravanaId, Carga};
use crate::viaje::{invertir_plan, invertir_ruta};
use asentamiento::datos::UNO;
use asentamiento::{Almacen, Costo};
use base::{Instante, Mil, Plan, Punto, Ritmo};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorAcarreo {
    /// El pueblo no tiene carro (destruido).
    SinCarro,
    /// El carro no está parado en el pueblo.
    NoEstaEnElPueblo,
    /// El almacén local aún no llena el carro.
    NoEstaLleno,
    /// El carro no está en el estado que pide la operación.
    EstadoInvalido,
    /// Aún no ha llegado.
    NoHaLlegado,
    /// El pueblo ya tiene carro: no se repone.
    YaTieneCarro,
    /// La plaza madre no tiene el coste de carro + buey.
    SinRecursos,
}

impl From<ErrorCaravana> for ErrorAcarreo {
    fn from(_: ErrorCaravana) -> Self {
        ErrorAcarreo::EstadoInvalido
    }
}

/// Lo que ingresó el viaje en el almacén de la madre.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ingreso {
    pub ingresado: Carga,
    /// Lo que no cupo (lo que se pierde).
    pub perdido: Carga,
}

/// El carro de acarreo de un pueblo extractor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PuebloAcarreo {
    pub pueblo: u32,
    pub madre: AsentamientoId,
    /// `None` mientras el pueblo no tiene carro.
    pub carro: Option<Caravana>,
}

/// Capacidad del carro de acarreo: un carro básico con buey, sin factores de política.
pub fn capacidad(r: &ReglasFlota) -> Mil {
    nuevo_carro(0, 0, Punto::default()).capacidad_efectiva(r, UNO)
}

/// Velocidad del carro de acarreo: la del buey, sin «Rutas Rápidas».
pub fn velocidad(r: &ReglasFlota) -> Mil {
    nuevo_carro(0, 0, Punto::default()).velocidad(r)
}

/// Lo que cuesta comprar otro carro: carro básico + buey (20 madera y 12 de oro con las cifras actuales).
pub fn coste_reposicion(r: &ReglasFlota) -> Costo {
    let mut c: Costo = r.basico.costo.iter().chain(&r.buey.costo).copied().collect();
    c.sort();
    c
}

fn nuevo_carro(id: CaravanaId, madre: AsentamientoId, posicion: Punto) -> Caravana {
    let mut c = Caravana::nueva(id, madre, posicion);
    c.tipo = TipoCaravana::Acarreo;
    c.carros = vec![Carro { tipo: TipoCarro::Basico, animal: Some(Animal::Buey) }];
    c
}

impl PuebloAcarreo {
    /// Un pueblo recién fundado: con su carro (básico + buey) parado en el pueblo. `id` es el de la caravana.
    pub fn nuevo(pueblo: u32, madre: AsentamientoId, id: CaravanaId, posicion_pueblo: Punto) -> Self {
        Self { pueblo, madre, carro: Some(nuevo_carro(id, madre, posicion_pueblo)) }
    }

    /// ¿Debe salir ya? Con el carro parado en el pueblo y el almacén local en `stock_local >= capacidad`.
    pub fn debe_salir(&self, r: &ReglasFlota, stock_local: Mil) -> bool {
        self.carro.as_ref().is_some_and(|c| c.estado == EstadoCaravana::Disponible) && stock_local >= capacidad(r)
    }

    /// El instante en que el almacén local llegará a llenar el carro, para agendar la salida: lo da el ritmo de extracción
    /// (`Ritmo::instante_de_valor`). `None` si no hay carro parado o el ritmo no llega nunca.
    pub fn instante_de_salida(&self, r: &ReglasFlota, stock_local: &Ritmo, desde: Instante) -> Option<Instante> {
        self.carro.as_ref().filter(|c| c.estado == EstadoCaravana::Disponible)?;
        stock_local.instante_de_valor(capacidad(r), desde)
    }

    /// Sale con el carro lleno: carga exactamente `capacidad` del almacén local (en el orden de los recursos) y emprende la ruta fija
    /// (`ruta` y su `plan` de ida, calculados una vez como cualquier plan). Devuelve lo cargado.
    pub fn salir(&mut self, r: &ReglasFlota, local: &mut Carga, ruta: Vec<Punto>, plan: Plan) -> Result<Carga, ErrorAcarreo> {
        let cap = capacidad(r);
        let c = self.carro.as_mut().ok_or(ErrorAcarreo::SinCarro)?;
        if c.estado != EstadoCaravana::Disponible {
            return Err(ErrorAcarreo::NoEstaEnElPueblo);
        }
        if local.values().sum::<Mil>() < cap {
            return Err(ErrorAcarreo::NoEstaLleno);
        }
        let mut falta = cap;
        let mut carga = Carga::new();
        for (&rec, n) in local.iter_mut() {
            let toma = (*n).min(falta);
            if toma > 0 {
                carga.insert(rec, toma);
                *n -= toma;
                falta -= toma;
            }
        }
        local.retain(|_, n| *n > 0);
        c.pasar(EstadoCaravana::EnTransito)?;
        c.destino = Some(self.madre);
        c.contenido = carga.clone();
        c.posicion = ruta.first().copied().unwrap_or(c.posicion);
        c.ruta = ruta;
        c.plan = Some(plan);
        Ok(carga)
    }

    /// Instante de la llegada del viaje en curso (a la madre o de vuelta al pueblo).
    pub fn llegada_en(&self) -> Option<Instante> {
        self.carro.as_ref().and_then(Caravana::llegada_en)
    }

    /// Llega a la plaza madre: vuelca en su almacén (topado: lo que no cabe se pierde) y vuelve **vacío** por la misma ruta.
    pub fn llegar_a_la_madre(&mut self, ahora: Instante, madre: &mut Almacen) -> Result<Ingreso, ErrorAcarreo> {
        let c = self.carro.as_mut().ok_or(ErrorAcarreo::SinCarro)?;
        if c.estado != EstadoCaravana::EnTransito {
            return Err(ErrorAcarreo::EstadoInvalido);
        }
        let plan = c.plan.as_ref().ok_or(ErrorAcarreo::EstadoInvalido)?;
        if plan.fin().is_none_or(|f| ahora < f) {
            return Err(ErrorAcarreo::NoHaLlegado);
        }
        let vuelta = invertir_plan(plan, ahora, plan.generacion + 1);
        let (mut ingresado, mut perdido) = (Carga::new(), Carga::new());
        for (rec, n) in std::mem::take(&mut c.contenido) {
            let entra = madre.agregar(rec, n);
            if entra > 0 {
                ingresado.insert(rec, entra);
            }
            if entra < n {
                perdido.insert(rec, n - entra);
            }
        }
        c.pasar(EstadoCaravana::Retornando)?;
        c.plan = Some(vuelta);
        c.ruta = invertir_ruta(&c.ruta);
        Ok(Ingreso { ingresado, perdido })
    }

    /// Llega de vuelta al pueblo: queda parado y listo para el próximo viaje.
    pub fn volver_al_pueblo(&mut self, ahora: Instante) -> Result<(), ErrorAcarreo> {
        let c = self.carro.as_mut().ok_or(ErrorAcarreo::SinCarro)?;
        if c.estado != EstadoCaravana::Retornando {
            return Err(ErrorAcarreo::EstadoInvalido);
        }
        let fin = c.plan.as_ref().and_then(Plan::fin).ok_or(ErrorAcarreo::EstadoInvalido)?;
        if ahora < fin {
            return Err(ErrorAcarreo::NoHaLlegado);
        }
        let pos = c.plan.as_ref().and_then(|p| p.vertices.last()).map(|v| v.p).unwrap_or(c.posicion);
        c.pasar(EstadoCaravana::Disponible)?;
        c.destino = None;
        c.ruta.clear();
        c.plan = None;
        c.posicion = pos;
        Ok(())
    }

    /// El carro se destruye (interceptado y perdido): el pueblo se queda sin carro y sin lo que llevaba. Devuelve la caravana destruida
    /// (con su escolta, que vuelve al campamento).
    pub fn destruir(&mut self) -> Option<Caravana> {
        self.carro.take()
    }

    /// Repone el carro **solo si el pueblo no tiene ninguno**, comprándolo en la plaza madre al coste de carro + buey. El carro nace parado
    /// en el pueblo; si el almacén local ya está lleno, [`PuebloAcarreo::debe_salir`] lo dirá al instante.
    pub fn reponer(&mut self, r: &ReglasFlota, madre: &mut Almacen, id: CaravanaId, posicion_pueblo: Punto) -> Result<(), ErrorAcarreo> {
        if self.carro.is_some() {
            return Err(ErrorAcarreo::YaTieneCarro);
        }
        if !madre.pagar(&coste_reposicion(r)) {
            return Err(ErrorAcarreo::SinRecursos);
        }
        self.carro = Some(nuevo_carro(id, self.madre, posicion_pueblo));
        Ok(())
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::datos::Reglas;
    use crate::flota::FactoresPolitica;
    use crate::viaje::plan_uniforme;
    use asentamiento::Recurso;

    fn flota() -> ReglasFlota {
        Reglas::cargar().flota
    }

    fn ruta() -> Vec<Punto> {
        vec![Punto::new(0, 0), Punto::new(48_000, 0)] // 48 unidades: 2 min a 24
    }

    fn pueblo() -> PuebloAcarreo {
        PuebloAcarreo::nuevo(7, 3, 100, Punto::new(0, 0))
    }

    fn local(n: Mil) -> Carga {
        [(Recurso::Cobre, n)].into_iter().collect()
    }

    #[test]
    fn el_carro_es_un_basico_con_buey_de_500_a_velocidad_24_sin_politicas() {
        let r = flota();
        assert_eq!((capacidad(&r), velocidad(&r)), (500_000, 24_000));
        let c = pueblo().carro.unwrap();
        assert_eq!((c.tipo, c.estado, c.origen, c.carros.len()), (TipoCaravana::Acarreo, EstadoCaravana::Disponible, 3, 1));
        // «Carga Ampliada» y «Rutas Rápidas» no le afectan: la capacidad de acarreo ignora el factor de política del origen.
        let p = FactoresPolitica { capacidad: 1_500_000, velocidad: 1_500_000, ..FactoresPolitica::NINGUNO };
        assert_eq!(capacidad(&r), 500_000, "aunque la madre tenga {p:?}");
        assert!(c.candidata_de_reparto(&r, 3, UNO).is_none(), "no entra en el reparto automático (no es comercial)");
    }

    #[test]
    fn solo_sale_con_el_carro_lleno() {
        let r = flota();
        let mut p = pueblo();
        assert!(!p.debe_salir(&r, 499_999));
        assert!(p.debe_salir(&r, 500_000));
        let plan = plan_uniforme(&ruta(), 0, velocidad(&r), 1).unwrap();
        let mut l = local(499_999);
        assert_eq!(p.salir(&r, &mut l, ruta(), plan.clone()), Err(ErrorAcarreo::NoEstaLleno));
        assert_eq!(l[&Recurso::Cobre], 499_999, "no se toca nada si no sale");
        let mut l = local(650_000);
        let carga = p.salir(&r, &mut l, ruta(), plan).unwrap();
        assert_eq!(carga[&Recurso::Cobre], 500_000);
        assert_eq!(l[&Recurso::Cobre], 150_000, "queda lo que sobraba");
        let c = p.carro.as_ref().unwrap();
        assert_eq!((c.estado, c.destino), (EstadoCaravana::EnTransito, Some(3)));
        assert!(!p.debe_salir(&r, 900_000), "en ruta no sale otra vez");
    }

    #[test]
    fn el_instante_de_salida_lo_da_el_ritmo_de_extraccion() {
        let r = flota();
        let p = pueblo();
        // El pueblo produce 10 por minuto: tarda 50 minutos en llenar los 500.
        let ritmo = Ritmo { valor_en_t0: 0, t0: 0, por_minuto: 10_000, maximo: Some(500_000), minimo: None };
        assert_eq!(p.instante_de_salida(&r, &ritmo, 0), Some(50 * 60_000));
        let parado = Ritmo { por_minuto: 0, ..ritmo };
        assert_eq!(p.instante_de_salida(&r, &parado, 0), None);
        let mut sin = pueblo();
        sin.destruir();
        assert_eq!(sin.instante_de_salida(&r, &ritmo, 0), None, "sin carro no se agenda nada");
    }

    #[test]
    fn el_viaje_vuelca_en_la_madre_y_vuelve_vacio_por_la_misma_ruta() {
        let r = flota();
        let mut p = pueblo();
        let plan = plan_uniforme(&ruta(), 0, velocidad(&r), 1).unwrap();
        p.salir(&r, &mut local(500_000), ruta(), plan).unwrap();
        let fin = p.llegada_en().unwrap();
        assert_eq!(fin, 2 * 60_000);
        let mut madre = Almacen::nuevo(1_000_000);
        assert_eq!(p.llegar_a_la_madre(fin - 1, &mut madre), Err(ErrorAcarreo::NoHaLlegado));
        let ing = p.llegar_a_la_madre(fin, &mut madre).unwrap();
        assert_eq!(ing.ingresado[&Recurso::Cobre], 500_000);
        assert!(ing.perdido.is_empty());
        assert_eq!(madre.cantidad(Recurso::Cobre), 500_000);
        let c = p.carro.as_ref().unwrap();
        assert_eq!((c.estado, c.contenido.is_empty(), c.ruta[0]), (EstadoCaravana::Retornando, true, Punto::new(48_000, 0)));
        let vuelta = p.llegada_en().unwrap();
        assert_eq!(vuelta, fin + 2 * 60_000);
        assert_eq!(p.volver_al_pueblo(vuelta - 1), Err(ErrorAcarreo::NoHaLlegado));
        p.volver_al_pueblo(vuelta).unwrap();
        let c = p.carro.as_ref().unwrap();
        assert_eq!((c.estado, c.posicion, c.destino), (EstadoCaravana::Disponible, Punto::new(0, 0), None));
        assert!(p.debe_salir(&r, 500_000), "listo para el próximo viaje");
    }

    #[test]
    fn lo_que_no_cabe_en_la_madre_se_pierde() {
        let r = flota();
        let mut p = pueblo();
        p.salir(&r, &mut local(500_000), ruta(), plan_uniforme(&ruta(), 0, velocidad(&r), 1).unwrap()).unwrap();
        let mut madre = Almacen::nuevo(1_000_000);
        madre.poner(Recurso::Cobre, 800_000);
        let ing = p.llegar_a_la_madre(p.llegada_en().unwrap(), &mut madre).unwrap();
        assert_eq!((ing.ingresado[&Recurso::Cobre], ing.perdido[&Recurso::Cobre]), (200_000, 300_000));
    }

    #[test]
    fn si_se_destruye_se_repone_solo_si_no_tiene_ninguno_pagando_carro_mas_buey() {
        let r = flota();
        let mut p = pueblo();
        let mut madre = Almacen::nuevo(1_000_000);
        madre.poner(Recurso::Madera, 100_000);
        madre.poner(Recurso::Oro, 11_999);
        assert_eq!(p.reponer(&r, &mut madre, 101, Punto::default()), Err(ErrorAcarreo::YaTieneCarro));
        assert!(p.destruir().is_some());
        assert!(p.destruir().is_none());
        assert_eq!(p.salir(&r, &mut local(500_000), ruta(), Default::default()), Err(ErrorAcarreo::SinCarro), "mientras no tenga, el pueblo acumula");
        assert_eq!(coste_reposicion(&r), vec![(Recurso::Madera, 20_000), (Recurso::Oro, 12_000)]);
        assert_eq!(p.reponer(&r, &mut madre, 101, Punto::default()), Err(ErrorAcarreo::SinRecursos));
        assert_eq!(madre.cantidad(Recurso::Madera), 100_000, "no cobra si falla");
        madre.poner(Recurso::Oro, 12_000);
        p.reponer(&r, &mut madre, 101, Punto::default()).unwrap();
        assert_eq!((madre.cantidad(Recurso::Madera), madre.cantidad(Recurso::Oro)), (80_000, 0));
        assert_eq!(p.carro.as_ref().unwrap().id, 101);
        assert!(p.debe_salir(&r, 500_000), "con el almacén local ya lleno sale al reponerse");
    }

    #[test]
    fn no_cuenta_para_los_cupos_de_la_flota_comercial() {
        use crate::caravana::{caravanas_comerciales_de, carros_en_uso};
        let v = vec![pueblo().carro.unwrap(), Caravana::nueva(1, 3, Punto::default())];
        assert_eq!(caravanas_comerciales_de(&v, 3), 1);
        assert_eq!(carros_en_uso(&v, 3), 0);
    }

    #[test]
    fn la_escolta_del_acarreo_se_cede_con_el_cupo_de_la_madre() {
        use crate::caravana::Escuadra;
        let r = flota();
        let mut p = pueblo();
        let c = p.carro.as_mut().unwrap();
        let cupo = crate::flota::cupo_escolta_de_acarreo(&r, None);
        c.asignar_escolta(vec![Escuadra { id: 1, heroe: 1, liderazgo: 60 }], cupo).unwrap();
        assert_eq!(c.asignar_escolta(vec![Escuadra { id: 2, heroe: 1, liderazgo: 50 }], cupo), Err(ErrorCaravana::CupoDeEscolta), "100 sin Mercado");
        assert!(c.asignar_escolta(vec![Escuadra { id: 2, heroe: 1, liderazgo: 50 }], crate::flota::cupo_escolta_de_acarreo(&r, Some(2))).is_ok());
    }
}
