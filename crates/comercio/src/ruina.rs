//! Cuando una plaza desaparece o cambia de dueño (Comercio_Caravanas §10, `cerrarDependientesDeRuina`). Funciones puras que, dada la plaza,
//! los acuerdos y las caravanas, deciden qué pasa con cada uno; el núcleo las llama en el instante del evento de ruina o de conquista
//! (prioridad 5, antes de cualquier evento de comercio del mismo instante) y aplica el resultado con los métodos de [`Caravana`].
//!
//! - **Ruina de `R`**: sus trueques `activo` o `propuesto` pasan a `expirado` **sin penalizar a nadie**; las caravanas que **salían de `R`**
//!   se pierden; las que **iban hacia `R`** vuelven (la `preparando` a `disponible` con su carga; la `en_transito` regresa por un plan nuevo
//!   o se pierde si no hay camino por tierra ni origen); la **aparcada en `R`** se destruye con todo, sin captura. Las `construccion` y las
//!   `adjunta` siguen su propio camino.
//! - **Conquista de `R`** (decidida el 2026-10-18): la **flota** pasa al conquistador con su zona (no se pierde ninguna), los trueques con la
//!   plaza se anulan igual que en la ruina y las caravanas que iban hacia ella **regresan con su carga**. La **aparcada en `R`** se destruye
//!   igual que en la ruina (decidido el 2026-10-19: el tren de suministros es objetivo, Doc 3.13.7 y 5.13.2).

use crate::caravana::{Caravana, EstadoCaravana, Escuadra, TipoCaravana};
use crate::tipos::{AcuerdoId, AsentamientoId, CaravanaId};
use crate::trueque::AcuerdoTrueque;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Motivo {
    /// La plaza desaparece.
    Ruina,
    /// La plaza sigue pero cambia de dueño.
    Conquista,
}

/// Qué hay que hacer con una caravana.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Accion {
    /// Se elimina (`comercio.caravana_perdida`); su escolta vuelve al campamento.
    Pierde,
    /// Estaba `preparando` hacia la plaza: la carga vuelve al almacén del origen y queda `disponible` (`comercio.caravana_vuelve`):
    /// [`Caravana::cancelar`].
    Vuelve,
    /// En camino hacia la plaza: regresa con su carga por `calcularRuta(posición → origen)` sin atracción: el núcleo construye el plan y
    /// llama a [`Caravana::regresar_a_origen`].
    Regresa,
    /// Aparcada en la plaza que cae (en ruinas o conquistada): se destruye con carros, animales y carga, sin captura.
    Destruye,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccionCaravana {
    pub caravana: CaravanaId,
    pub accion: Accion,
    /// Su escolta (la que se libera si se pierde, vuelve o se destruye).
    pub escolta: Vec<Escuadra>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TruequeCancelado {
    pub acuerdo: AcuerdoId,
    /// La plaza socia, a la que va `comercio.trueque_cancelado`.
    pub socia: AsentamientoId,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CierrePlaza {
    pub trueques: Vec<TruequeCancelado>,
    pub caravanas: Vec<AccionCaravana>,
    /// Solo en una conquista: la flota de la plaza (caravanas con ese origen) que pasa al conquistador junto con su zona.
    pub flota_traspasada: Vec<CaravanaId>,
}

/// Anula los trueques de la plaza (vivos: `propuesto` o `activo`) **sin penalizar a nadie**.
pub fn cancelar_trueques(plaza: AsentamientoId, acuerdos: &mut [AcuerdoTrueque]) -> Vec<TruequeCancelado> {
    let mut v = Vec::new();
    for a in acuerdos.iter_mut().filter(|a| a.a == plaza || a.b == plaza) {
        if a.cancelar_por_plaza() {
            v.push(TruequeCancelado { acuerdo: a.id, socia: if a.a == plaza { a.b } else { a.a } });
        }
    }
    v
}

/// Resuelve la cascada de comercio de la plaza `plaza` que desaparece o cambia de dueño. `origen_existe` dice si el origen de una
/// caravana sigue existiendo (en la ruina, `plaza` ya no cuenta) y `hay_camino_a_origen` si desde su posición actual hay camino por tierra
/// a su origen. Los trueques se cancelan en `acuerdos`; las caravanas **no se tocan**: se devuelven las acciones.
// DUDA: la escolta de la flota traspasada por una conquista (escuadras de héroes desalojados) se deja al sistema militar.
// DUDA: una caravana `EnBatalla` sigue como está; su resultado lo aplica el sistema militar.
pub fn cerrar_dependientes(
    motivo: Motivo,
    plaza: AsentamientoId,
    acuerdos: &mut [AcuerdoTrueque],
    caravanas: &[Caravana],
    origen_existe: &dyn Fn(AsentamientoId) -> bool,
    hay_camino_a_origen: &dyn Fn(&Caravana) -> bool,
) -> CierrePlaza {
    let mut cierre = CierrePlaza { trueques: cancelar_trueques(plaza, acuerdos), ..Default::default() };
    for c in caravanas.iter().filter(|c| c.tipo != TipoCaravana::Construccion && c.estado != EstadoCaravana::Adjunta) {
        let mut accion = |accion| cierre.caravanas.push(AccionCaravana { caravana: c.id, accion, escolta: c.escolta.clone() });
        if c.origen == plaza {
            match motivo {
                Motivo::Ruina => accion(Accion::Pierde),
                Motivo::Conquista => cierre.flota_traspasada.push(c.id),
            }
            continue;
        }
        match (c.estado, c.destino) {
            (EstadoCaravana::Preparando, Some(d)) if d == plaza => accion(Accion::Vuelve),
            (EstadoCaravana::EnTransito, Some(d)) if d == plaza => {
                accion(if origen_existe(c.origen) && hay_camino_a_origen(c) { Accion::Regresa } else { Accion::Pierde });
            }
            (EstadoCaravana::Aparcada, _) if c.anfitriona == Some(plaza) => accion(Accion::Destruye),
            _ => {} // las demás siguen como estaban
        }
    }
    cierre
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::caravana::{Animal, Carro, TipoCarro};
    use crate::datos::Reglas;
    use crate::tipos::Lado;
    use crate::trueque::EstadoTrueque;
    use asentamiento::Recurso;
    use base::Punto;

    fn car(id: u64, origen: u32, estado: EstadoCaravana, destino: Option<u32>) -> Caravana {
        let mut c = Caravana::nueva(id, origen, Punto::default());
        c.carros = vec![Carro { tipo: TipoCarro::Basico, animal: Some(Animal::Buey) }];
        c.estado = estado;
        c.destino = destino;
        c.contenido.insert(Recurso::Madera, 1_000);
        c
    }

    fn trueque(id: u64, a: u32, b: u32, activo: bool) -> AcuerdoTrueque {
        let r = Reglas::cargar();
        let mut t = AcuerdoTrueque::proponer(&r.trueque, id, a, b, &[(Recurso::Madera, 1_000)], &[(Recurso::Piedra, 1_000)], 0, true).unwrap();
        if activo {
            t.aceptar(&r.trueque, 0, true).unwrap();
        }
        t
    }

    fn cerrar(motivo: Motivo, plaza: u32, ac: &mut [AcuerdoTrueque], cs: &[Caravana]) -> CierrePlaza {
        cerrar_dependientes(motivo, plaza, ac, cs, &|o| o != plaza, &|_| true)
    }

    fn accion_de(c: &CierrePlaza, id: u64) -> Option<Accion> {
        c.caravanas.iter().find(|a| a.caravana == id).map(|a| a.accion)
    }

    #[test]
    fn los_trueques_de_la_plaza_se_cancelan_sin_penalizar_y_avisan_a_la_socia() {
        let mut ac = vec![trueque(1, 5, 6, true), trueque(2, 7, 5, false), trueque(3, 8, 9, true), trueque(4, 5, 6, true)];
        ac[3].estado = EstadoTrueque::Cumplido;
        let c = cerrar(Motivo::Ruina, 5, &mut ac, &[]);
        assert_eq!(c.trueques, vec![TruequeCancelado { acuerdo: 1, socia: 6 }, TruequeCancelado { acuerdo: 2, socia: 7 }]);
        assert_eq!((ac[0].estado, ac[1].estado), (EstadoTrueque::Expirado, EstadoTrueque::Expirado));
        assert_eq!(ac[2].estado, EstadoTrueque::Activo, "los de otras plazas no se tocan");
        assert_eq!(ac[3].estado, EstadoTrueque::Cumplido, "lo terminado queda como está");
        assert_eq!(ac[0].aceptado_en, Some(0), "no se toca nada más: ni reputación ni líneas");
    }

    #[test]
    fn la_ruina_pierde_las_que_salian_de_la_plaza_y_hace_volver_las_que_iban() {
        let mut escoltada = car(1, 5, EstadoCaravana::EnTransito, Some(9));
        escoltada.escolta = vec![Escuadra { id: 3, heroe: 4, liderazgo: 10 }];
        let cs = vec![
            escoltada,                                                  // sale de R
            car(2, 5, EstadoCaravana::Disponible, None),                // casco de R
            car(3, 6, EstadoCaravana::Preparando, Some(5)),             // va a R, preparando
            car(4, 6, EstadoCaravana::EnTransito, Some(5)),             // va a R, en camino
            car(5, 6, EstadoCaravana::Retornando, Some(5)),             // ya entregó en R y vuelve a casa
            car(6, 6, EstadoCaravana::EnTransito, Some(9)),             // no tiene que ver
            car(7, 6, EstadoCaravana::Disponible, None),
        ];
        let c = cerrar(Motivo::Ruina, 5, &mut [], &cs);
        assert_eq!(accion_de(&c, 1), Some(Accion::Pierde));
        assert_eq!(c.caravanas[0].escolta.len(), 1, "su escolta vuelve al campamento");
        assert_eq!(accion_de(&c, 2), Some(Accion::Pierde));
        assert_eq!(accion_de(&c, 3), Some(Accion::Vuelve));
        assert_eq!(accion_de(&c, 4), Some(Accion::Regresa));
        for id in [5, 6, 7] {
            assert_eq!(accion_de(&c, id), None, "las demás siguen como estaban ({id})");
        }
        assert!(c.flota_traspasada.is_empty());
    }

    #[test]
    fn sin_camino_de_vuelta_o_sin_origen_la_que_iba_a_la_ruina_se_pierde() {
        let cs = vec![car(1, 6, EstadoCaravana::EnTransito, Some(5)), car(2, 7, EstadoCaravana::EnTransito, Some(5))];
        let sin_camino = cerrar_dependientes(Motivo::Ruina, 5, &mut [], &cs, &|_| true, &|c| c.id != 1);
        assert_eq!((accion_de(&sin_camino, 1), accion_de(&sin_camino, 2)), (Some(Accion::Pierde), Some(Accion::Regresa)));
        let sin_origen = cerrar_dependientes(Motivo::Ruina, 5, &mut [], &cs, &|o| o != 7, &|_| true);
        assert_eq!((accion_de(&sin_origen, 1), accion_de(&sin_origen, 2)), (Some(Accion::Regresa), Some(Accion::Pierde)));
    }

    #[test]
    fn la_aparcada_en_la_plaza_que_cae_se_destruye_sin_captura_en_la_ruina_y_en_la_conquista() {
        let mut ap = car(1, 6, EstadoCaravana::Aparcada, None);
        ap.anfitriona = Some(5);
        let mut ap_otra = car(2, 6, EstadoCaravana::Aparcada, None);
        ap_otra.anfitriona = Some(99);
        for motivo in [Motivo::Ruina, Motivo::Conquista] {
            let c = cerrar(motivo, 5, &mut [], &[ap.clone(), ap_otra.clone()]);
            assert_eq!(accion_de(&c, 1), Some(Accion::Destruye), "{motivo:?}");
            assert_eq!(accion_de(&c, 2), None, "la aparcada en otra plaza no se toca");
        }
    }

    #[test]
    fn las_de_fundacion_y_las_adjuntas_siguen_su_propio_camino() {
        let mut fundacion = car(1, 5, EstadoCaravana::EnTransito, Some(5));
        fundacion.tipo = TipoCaravana::Construccion;
        let adjunta = car(2, 5, EstadoCaravana::Adjunta, None);
        let c = cerrar(Motivo::Ruina, 5, &mut [], &[fundacion, adjunta]);
        assert!(c.caravanas.is_empty());
    }

    #[test]
    fn la_conquista_traspasa_la_flota_y_hace_regresar_a_las_que_iban() {
        let mut ap_ajena = car(5, 6, EstadoCaravana::Aparcada, None);
        ap_ajena.anfitriona = Some(5);
        let cs = vec![
            car(1, 5, EstadoCaravana::EnTransito, Some(9)),
            car(2, 5, EstadoCaravana::Disponible, None),
            car(3, 6, EstadoCaravana::Preparando, Some(5)),
            car(4, 6, EstadoCaravana::EnTransito, Some(5)),
            ap_ajena,
        ];
        let mut ac = vec![trueque(1, 5, 6, true)];
        let c = cerrar(Motivo::Conquista, 5, &mut ac, &cs);
        assert_eq!(c.flota_traspasada, vec![1, 2], "la flota de la plaza no se pierde: pasa al conquistador");
        assert_eq!(accion_de(&c, 1), None);
        assert_eq!(accion_de(&c, 3), Some(Accion::Vuelve));
        assert_eq!(accion_de(&c, 4), Some(Accion::Regresa), "regresan con su carga");
        assert_eq!(accion_de(&c, 5), Some(Accion::Destruye), "la aparcada en la plaza conquistada se destruye (Doc 3.13.7)");
        assert_eq!(c.trueques.len(), 1, "los trueques con la plaza se anulan igual");
        assert_eq!(ac[0].estado, EstadoTrueque::Expirado);
    }

    #[test]
    fn aplicar_las_acciones_deja_a_las_caravanas_en_estados_validos() {
        use crate::viaje::plan_uniforme;
        let r = Reglas::cargar();
        let mut alm = asentamiento::Almacen::nuevo(1_000_000);
        // Vuelve: la preparando cancela y devuelve su carga.
        let mut prep = car(3, 6, EstadoCaravana::Preparando, Some(5));
        prep.cancelar(&mut alm).unwrap();
        assert_eq!((prep.estado, alm.cantidad(Recurso::Madera)), (EstadoCaravana::Disponible, 1_000));
        // Regresa: la en camino queda retornando con su carga y destino = la plaza.
        let mut camino = car(4, 6, EstadoCaravana::EnTransito, Some(5));
        camino.origen_acuerdo = Some((1, Lado::A));
        let plan = plan_uniforme(&[Punto::new(0, 0), Punto::new(10_000, 0)], 0, r.flota.buey.velocidad, 2).unwrap();
        camino.regresar_a_origen(5, plan).unwrap();
        assert_eq!((camino.estado, camino.destino, camino.origen_acuerdo), (EstadoCaravana::Retornando, Some(5), None));
        assert_eq!(camino.contenido[&Recurso::Madera], 1_000, "con su carga");
        // Una disponible no puede regresar: no está en camino.
        assert!(car(1, 6, EstadoCaravana::Disponible, None).regresar_a_origen(5, Default::default()).is_err());
    }
}
