//! El carro de acarreo (Comercio_Caravanas, segunda §12; Doc 3.13.8): cada pueblo extractor tiene un carro propio, una caravana de tipo
//! `acarreo` con **un carro básico y un buey** (500 de capacidad, velocidad 24) y una **ruta fija** pueblo ↔ plaza madre. Es una [`Caravana`] más
//! de la flota (la plaza madre es su origen); lo que la distingue lo dice este módulo y la llegada ([`crate::caravana::Llegada::Acarreo`]).
//!
//! - **Solo sale con el carro lleno**: cuando el almacén local del pueblo alcanza la capacidad del carro, o al reponerse el carro con el
//!   almacén ya lleno ([`debe_salir`]). Vuelca en el almacén de la madre y vuelve vacío.
//! - **No cuenta** contra los cupos de caravanas ni de carros, ni exige Mercado, ni entra en el reparto automático ni en el trueque.
//! - **No le afectan** "Carga Ampliada" ni "Rutas Rápidas" (decidido el 2026-10-18): con ×1,5 de capacidad el carro de 750 superaría el
//!   almacén de 500 del pueblo y «sale lleno» no se dispararía nunca. Por eso las funciones de aquí usan siempre factor `UNO`.
//! - **Si se destruye**, se compra otro en la plaza madre solo si el pueblo no tiene ninguno ([`reponer`]), al coste de carro + buey; mientras
//!   no tenga, el pueblo acumula hasta llenar su almacén y la extracción para.

use crate::caravana::{Animal, Carro, Caravana, ErrorCaravana, EstadoCaravana, TipoCarro, TipoCaravana};
use crate::datos::ReglasFlota;
use crate::tipos::{AsentamientoId, CaravanaId, Carga};
use crate::viaje::Peaje;
use asentamiento::datos::UNO;
use asentamiento::{Almacen, Costo};
use base::{Mil, Plan, Punto};

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

/// El carro de un pueblo recién levantado: un carro básico con buey, parado en el pueblo. `madre` es su origen.
pub fn nuevo_carro(id: CaravanaId, madre: AsentamientoId, posicion_pueblo: Punto) -> Caravana {
    let mut c = Caravana::nueva(id, madre, posicion_pueblo);
    c.tipo = TipoCaravana::Acarreo;
    c.carros = vec![Carro { tipo: TipoCarro::Basico, animal: Some(Animal::Buey) }];
    c
}

/// ¿Debe salir ya? Con el carro parado en el pueblo y el almacén local en `stock_local >= capacidad`.
pub fn debe_salir(c: &Caravana, r: &ReglasFlota, stock_local: Mil) -> bool {
    c.estado == EstadoCaravana::Disponible && stock_local >= capacidad(r)
}

/// Sale con el carro lleno: lleva `carga` (la capacidad entera, que quien llama ya sacó del almacén local) hacia la plaza madre (`destino`) por
/// la ruta fija pueblo → madre con su `plan` de ida y los `peajes` que cruza.
#[allow(clippy::too_many_arguments)]
pub fn salir(c: &mut Caravana, destino: AsentamientoId, carga: Carga, ruta: Vec<Punto>, costes: Vec<i64>, peajes: Vec<Peaje>, plan: Plan) -> Result<(), ErrorCaravana> {
    if c.tipo != TipoCaravana::Acarreo {
        return Err(ErrorCaravana::EstadoInvalido);
    }
    if c.estado != EstadoCaravana::Disponible {
        return Err(ErrorCaravana::NoDisponible);
    }
    c.pasar(EstadoCaravana::EnTransito)?;
    c.destino = Some(destino);
    c.contenido = carga;
    c.posicion = ruta.first().copied().unwrap_or(c.posicion);
    c.ruta = ruta;
    c.costes = costes;
    c.peajes = peajes;
    c.plan = Some(plan);
    Ok(())
}

/// Repone el carro de un pueblo que no tiene ninguno, comprándolo en la plaza madre (`madre`, su almacén) al coste de carro + buey. El carro
/// nace parado en el pueblo; si el almacén local ya está lleno, [`debe_salir`] lo dirá al instante. Si la madre no puede pagarlo no se toca nada.
pub fn reponer(r: &ReglasFlota, madre: &mut Almacen, id: CaravanaId, madre_id: AsentamientoId, posicion_pueblo: Punto) -> Result<Caravana, ErrorCaravana> {
    if !madre.pagar(&coste_reposicion(r)) {
        return Err(ErrorCaravana::SinRecursos);
    }
    Ok(nuevo_carro(id, madre_id, posicion_pueblo))
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::caravana::Llegada;
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

    fn carro() -> Caravana {
        nuevo_carro(100, 3, Punto::new(0, 0))
    }

    fn carga(n: Mil) -> Carga {
        [(Recurso::Cobre, n)].into_iter().collect()
    }

    /// El carro sale con 500 de cobre por la ruta de prueba (llega a los 2 minutos).
    fn en_marcha(r: &ReglasFlota) -> Caravana {
        let mut c = carro();
        salir(&mut c, 3, carga(500_000), ruta(), vec![1, 1], vec![], plan_uniforme(&ruta(), 0, velocidad(r), 1).unwrap()).unwrap();
        c
    }

    #[test]
    fn el_carro_es_un_basico_con_buey_de_500_a_velocidad_24_sin_politicas() {
        let r = flota();
        assert_eq!((capacidad(&r), velocidad(&r)), (500_000, 24_000));
        let c = carro();
        assert_eq!((c.tipo, c.estado, c.origen, c.carros.len()), (TipoCaravana::Acarreo, EstadoCaravana::Disponible, 3, 1));
        // «Carga Ampliada» y «Rutas Rápidas» no le afectan: la capacidad de acarreo ignora el factor de política del origen.
        let p = FactoresPolitica { capacidad: 1_500_000, velocidad: 1_500_000, ..FactoresPolitica::NINGUNO };
        assert_eq!(capacidad(&r), 500_000, "aunque la madre tenga {p:?}");
        assert_eq!(c.velocidad_efectiva(&r, p.velocidad), 24_000);
        assert!(c.candidata_de_reparto(&r, 3, UNO).is_none(), "no entra en el reparto automático (no es comercial)");
    }

    #[test]
    fn solo_sale_con_el_carro_lleno() {
        let r = flota();
        let mut c = carro();
        assert!(!debe_salir(&c, &r, 499_999) && debe_salir(&c, &r, 500_000));
        salir(&mut c, 3, carga(500_000), ruta(), vec![1, 1], vec![], plan_uniforme(&ruta(), 0, velocidad(&r), 1).unwrap()).unwrap();
        assert_eq!((c.estado, c.destino, c.contenido[&Recurso::Cobre]), (EstadoCaravana::EnTransito, Some(3), 500_000));
        assert!(!debe_salir(&c, &r, 900_000), "en ruta no sale otra vez");
        assert_eq!(salir(&mut c, 3, carga(1), ruta(), vec![], vec![], Default::default()), Err(ErrorCaravana::NoDisponible));
        let mut otra = Caravana::nueva(1, 3, Punto::default());
        assert_eq!(salir(&mut otra, 3, carga(1), ruta(), vec![], vec![], Default::default()), Err(ErrorCaravana::EstadoInvalido), "solo el carro de acarreo");
    }

    #[test]
    fn llega_a_la_madre_con_su_carga_y_vuelve_vacio_por_la_misma_ruta() {
        let r = flota();
        let mut c = en_marcha(&r);
        let fin = c.llegada_en().unwrap();
        assert_eq!(fin, 2 * 60_000);
        assert_eq!(c.llegar(fin - 1), Err(ErrorCaravana::NoHaLlegado));
        let Llegada::Acarreo { destino, contenido } = c.llegar(fin).unwrap() else { panic!("debía llegar como acarreo") };
        assert_eq!((destino, contenido[&Recurso::Cobre]), (3, 500_000));
        assert_eq!((c.estado, c.contenido.is_empty(), c.ruta[0], c.costes.clone()), (EstadoCaravana::Retornando, true, Punto::new(48_000, 0), vec![1, 1]));
        let vuelta = c.llegada_en().unwrap();
        assert_eq!(vuelta, fin + 2 * 60_000);
        let Llegada::Casa { contenido, .. } = c.llegar(vuelta).unwrap() else { panic!("debía llegar a casa") };
        assert!(contenido.is_empty());
        assert_eq!((c.estado, c.posicion, c.destino), (EstadoCaravana::Disponible, Punto::new(0, 0), None));
        assert!(debe_salir(&c, &r, 500_000), "listo para el próximo viaje");
    }

    #[test]
    fn si_se_repone_se_paga_carro_mas_buey_y_sin_recursos_no_se_toca_nada() {
        let r = flota();
        let mut madre = Almacen::nuevo(1_000_000);
        madre.poner(Recurso::Madera, 100_000);
        madre.poner(Recurso::Oro, 11_999);
        assert_eq!(coste_reposicion(&r), vec![(Recurso::Madera, 20_000), (Recurso::Oro, 12_000)]);
        assert_eq!(reponer(&r, &mut madre, 101, 3, Punto::default()), Err(ErrorCaravana::SinRecursos));
        assert_eq!(madre.cantidad(Recurso::Madera), 100_000, "no cobra si falla");
        madre.poner(Recurso::Oro, 12_000);
        let nuevo = reponer(&r, &mut madre, 101, 3, Punto::default()).unwrap();
        assert_eq!((madre.cantidad(Recurso::Madera), madre.cantidad(Recurso::Oro)), (80_000, 0));
        assert_eq!((nuevo.id, nuevo.tipo, nuevo.origen), (101, TipoCaravana::Acarreo, 3));
        assert!(debe_salir(&nuevo, &r, 500_000), "con el almacén local ya lleno sale al reponerse");
    }

    #[test]
    fn no_cuenta_para_los_cupos_de_la_flota_comercial() {
        use crate::caravana::{caravanas_comerciales_de, carros_en_uso};
        let v = vec![carro(), Caravana::nueva(1, 3, Punto::default())];
        assert_eq!(caravanas_comerciales_de(&v, 3), 1);
        assert_eq!(carros_en_uso(&v, 3), 0);
    }

    #[test]
    fn la_escolta_del_acarreo_se_cede_con_el_cupo_de_la_madre() {
        use crate::caravana::Escuadra;
        let r = flota();
        let mut c = carro();
        let cupo = crate::flota::cupo_escolta_de_acarreo(&r, None);
        c.asignar_escolta(vec![Escuadra { id: 1, heroe: 1, liderazgo: 60 }], cupo).unwrap();
        assert_eq!(c.asignar_escolta(vec![Escuadra { id: 2, heroe: 1, liderazgo: 50 }], cupo), Err(ErrorCaravana::CupoDeEscolta), "100 sin Mercado");
        assert!(c.asignar_escolta(vec![Escuadra { id: 2, heroe: 1, liderazgo: 50 }], crate::flota::cupo_escolta_de_acarreo(&r, Some(2))).is_ok());
    }
}
