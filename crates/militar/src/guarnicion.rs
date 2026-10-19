//! La guarnición de una plaza (Doc 5.15.3, Heroe_Y_Tropa §7): las escuadras que sus residentes han entregado a la IA. Defienden siempre, aun
//! con su dueño herido o desconectado. Cada héroe residente tiene un **cupo** propio, en la escala del coste de Liderazgo, que decide la
//! plaza y que **no gasta** su Liderazgo.

use crate::datos::Reglas;
use crate::escuadra::Contenedor;
use crate::heroe::{ErrorHeroe, Heroe, Residencia};
use crate::liderazgo;
use crate::tipos::{AsentamientoId, EscuadraId};
use asentamiento::edificio::Edificio;
use asentamiento::Tipo;

/// Cupo de guarnición **por héroe residente** de la plaza, en puntos de Liderazgo:
/// el **primer** Barracón y la **primera** Galería de tiro activos (por su nivel interno) + las Caballerizas (la primera activa) + el mejor
/// recinto de muralla completo (`nivel_recinto`, 0 si no hay) + la política «Levas de guarnición» (`extra`). Sin Barracón, Galería ni
/// Caballerizas de nivel 2 no hay guarnición, aunque haya muralla.
pub fn cupo(r: &Reglas, edificios: &[Edificio], nivel_recinto: u32, extra: i64) -> i64 {
    let primero = |t: Tipo| edificios.iter().find(|e| e.tipo == t && e.activo());
    let tabla = |tabla: &[i64], e: Option<&Edificio>| e.map_or(0, |e| tabla.get(e.nivel_interno as usize - 1).copied().unwrap_or(0));
    let g = &r.guarnicion;
    let por_edificios = tabla(&g.por_nivel_edificio, primero(Tipo::Barracon))
        + tabla(&g.por_nivel_edificio, primero(Tipo::GaleriaDeTiro))
        + tabla(&g.por_nivel_caballerizas, primero(Tipo::Caballerizas));
    if por_edificios == 0 {
        return 0;
    }
    let muralla = if nivel_recinto > 0 { g.recinto_completo_por_nivel.get(nivel_recinto as usize - 1).copied().unwrap_or(0) } else { 0 };
    por_edificios + muralla + extra
}

/// Cupo que ya ocupa un héroe: el Liderazgo de sus escuadras en guarnición.
pub fn ocupada(r: &Reglas, heroe: &Heroe) -> i64 {
    liderazgo::comprometido(r, heroe.escuadras.iter().filter(|e| e.en_guarnicion))
}

/// Entrega una escuadra de su campamento a la guarnición de la plaza `plaza` (donde reside), dentro de su `cupo` (el de [`cupo`]). Si el cupo
/// baja después, lo asignado se queda: solo impide asignar más.
pub fn asignar(r: &Reglas, heroe: &mut Heroe, plaza: AsentamientoId, cupo: i64, escuadra: EscuadraId) -> Result<(), ErrorHeroe> {
    if heroe.residencia != Some(Residencia::Asentamiento(plaza)) {
        return Err(ErrorHeroe::SinResidencia);
    }
    let e = heroe.escuadra(escuadra).ok_or(ErrorHeroe::EscuadraAjena)?;
    if e.reserva_batalla.is_some() {
        return Err(ErrorHeroe::EnBatalla);
    }
    if e.contenedor != Contenedor::Campamento {
        return Err(ErrorHeroe::NoEnCampamento);
    }
    if e.en_guarnicion {
        return Err(ErrorHeroe::YaEnGuarnicion);
    }
    let ocupado = ocupada(r, heroe) + liderazgo::coste(r, e.tropa);
    if ocupado > cupo {
        return Err(ErrorHeroe::SuperaElCupo { ocupado, cupo });
    }
    heroe.escuadra_mut(escuadra).expect("existe").en_guarnicion = true;
    Ok(())
}

/// Devuelve la escuadra a su campamento. Una reservada para una batalla no entra ni sale mientras dura.
pub fn retirar(heroe: &mut Heroe, escuadra: EscuadraId) -> Result<(), ErrorHeroe> {
    let e = heroe.escuadra_mut(escuadra).filter(|e| e.en_guarnicion).ok_or(ErrorHeroe::NoEnGuarnicion)?;
    if e.reserva_batalla.is_some() {
        return Err(ErrorHeroe::EnBatalla);
    }
    e.en_guarnicion = false;
    Ok(())
}

/// Suelta la guarnición de un héroe que deja de residir donde la tenía: solo se guarnece la residencia.
pub fn soltar(heroe: &mut Heroe) {
    heroe.escuadras.iter_mut().for_each(|e| e.en_guarnicion = false);
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::catalogo::Tropa;
    use crate::escuadra::Escuadra;
    use crate::heroe::Controlador;
    use asentamiento::edificio::Estado;

    fn r() -> Reglas {
        Reglas::cargar()
    }

    fn edificio(id: u32, tipo: Tipo, nivel: u8, estado: Estado) -> Edificio {
        Edificio { nivel_interno: nivel, ..Edificio::nuevo(id, tipo, estado) }
    }

    fn activo(id: u32, tipo: Tipo, nivel: u8) -> Edificio {
        edificio(id, tipo, nivel, Estado::Activo)
    }

    fn heroe_en(plaza: u32, tropas: &[Tropa]) -> Heroe {
        let r = r();
        let mut h = Heroe::nuevo(&r, 1, Controlador::Humano, 1, 1);
        h.residencia = Some(Residencia::Asentamiento(plaza));
        for (i, &t) in tropas.iter().enumerate() {
            h.escuadras.push(Escuadra::nueva(&r, 10 + i as u64, 1, t, 1, Some(plaza), 0));
        }
        h
    }

    #[test]
    fn el_cupo_suma_edificios_recinto_y_politica() {
        let r = r();
        let e = [activo(1, Tipo::Barracon, 2), activo(2, Tipo::GaleriaDeTiro, 1), activo(3, Tipo::Caballerizas, 2)];
        assert_eq!(cupo(&r, &e, 0, 0), 14 + 7 + 14);
        assert_eq!(cupo(&r, &e, 3, 14), 14 + 7 + 14 + 20 + 14, "adarve (20) y Levas de guarnición (+14)");
        assert_eq!(cupo(&r, &e, 1, 0), 35 + 10);
    }

    #[test]
    fn las_caballerizas_de_nivel_uno_no_dan_nada_y_sin_barracon_ni_galeria_no_hay_guarnicion() {
        let r = r();
        assert_eq!(cupo(&r, &[activo(1, Tipo::Caballerizas, 1)], 0, 0), 0);
        assert_eq!(cupo(&r, &[activo(1, Tipo::Caballerizas, 3)], 0, 0), 32);
        // Sin Barracón ni Galería, aunque haya muralla y política, no hay guarnición.
        assert_eq!(cupo(&r, &[activo(1, Tipo::CentroUrbano, 1), activo(2, Tipo::Vivienda, 1)], 3, 14), 0);
        // La muralla y la política suman solo cuando ya hay algún edificio que dé cupo.
        assert_eq!(cupo(&r, &[activo(1, Tipo::Barracon, 1)], 2, 0), 7 + 15);
    }

    #[test]
    fn solo_cuenta_el_primer_edificio_activo_de_cada_tipo() {
        let r = r();
        let e = [edificio(1, Tipo::Barracon, 3, Estado::EnConstruccion), activo(2, Tipo::Barracon, 1), activo(3, Tipo::Barracon, 3), edificio(4, Tipo::GaleriaDeTiro, 2, Estado::EnCola)];
        assert_eq!(cupo(&r, &e, 0, 0), 7, "ni el que está en obra ni el segundo activo suman");
    }

    #[test]
    fn asignar_respeta_el_cupo_y_no_gasta_liderazgo() {
        let r = r();
        let mut h = heroe_en(5, &[Tropa::Hequetai, Tropa::Honderos, Tropa::Arqueros]); // 45, 14, 22
        assert_eq!(asignar(&r, &mut h, 5, 40, 10), Err(ErrorHeroe::SuperaElCupo { ocupado: 45, cupo: 40 }));
        assert_eq!(asignar(&r, &mut h, 5, 40, 11), Ok(()));
        assert_eq!(ocupada(&r, &h), 14);
        assert_eq!(asignar(&r, &mut h, 5, 40, 12), Ok(()), "14 + 22 = 36");
        assert_eq!(ocupada(&r, &h), 36);
        assert_eq!(asignar(&r, &mut h, 5, 40, 11), Err(ErrorHeroe::YaEnGuarnicion));
        assert_eq!(liderazgo::disponible(&r, h.liderazgo_base, h.escuadras.iter().filter(|e| !e.en_guarnicion)), 55, "lo entregado a la IA no se descuenta del Liderazgo del héroe");
    }

    #[test]
    fn rechazos_de_asignar_en_su_orden() {
        let r = r();
        let mut h = heroe_en(5, &[Tropa::Honderos, Tropa::Honderos]);
        h.escuadras[1].tropa = Tropa::Arqueros;
        assert_eq!(asignar(&r, &mut h, 6, 99, 10), Err(ErrorHeroe::SinResidencia), "reside en la 5, no en la 6");
        assert_eq!(asignar(&r, &mut h, 5, 99, 77), Err(ErrorHeroe::EscuadraAjena));
        h.escuadras[0].reserva_batalla = Some(3);
        assert_eq!(asignar(&r, &mut h, 5, 99, 10), Err(ErrorHeroe::EnBatalla));
        h.escuadras[0].reserva_batalla = None;
        h.escuadras[0].contenedor = Contenedor::Ejercito(2);
        assert_eq!(asignar(&r, &mut h, 5, 99, 10), Err(ErrorHeroe::NoEnCampamento));
        h.residencia = Some(Residencia::Campamento(1));
        assert_eq!(asignar(&r, &mut h, 5, 99, 11), Err(ErrorHeroe::SinResidencia), "un residente de un campamento de mercenarios no tiene guarnición");
        h.residencia = None;
        assert_eq!(asignar(&r, &mut h, 5, 99, 11), Err(ErrorHeroe::SinResidencia));
    }

    #[test]
    fn si_el_cupo_baja_lo_asignado_se_queda_y_solo_se_impide_asignar_mas() {
        let r = r();
        let mut h = heroe_en(5, &[Tropa::Arqueros, Tropa::Honderos]);
        asignar(&r, &mut h, 5, 40, 10).unwrap();
        asignar(&r, &mut h, 5, 40, 11).unwrap();
        assert_eq!(ocupada(&r, &h), 36);
        // El cupo cae a 10 (expira la política, se daña un edificio): nada se retira.
        assert!(h.escuadras.iter().all(|e| e.en_guarnicion));
        let mut otra = heroe_en(5, &[Tropa::Lenadores]);
        otra.id = 2;
        assert_eq!(asignar(&r, &mut otra, 5, 6, 10), Err(ErrorHeroe::SuperaElCupo { ocupado: 7, cupo: 6 }));
    }

    #[test]
    fn retirar_la_devuelve_pero_una_reservada_no_entra_ni_sale() {
        let r = r();
        let mut h = heroe_en(5, &[Tropa::Arqueros]);
        assert_eq!(retirar(&mut h, 10), Err(ErrorHeroe::NoEnGuarnicion));
        assert_eq!(retirar(&mut h, 99), Err(ErrorHeroe::NoEnGuarnicion));
        asignar(&r, &mut h, 5, 40, 10).unwrap();
        h.escuadras[0].reserva_batalla = Some(8);
        assert_eq!(retirar(&mut h, 10), Err(ErrorHeroe::EnBatalla));
        assert!(h.escuadras[0].en_guarnicion);
        h.escuadras[0].reserva_batalla = None;
        assert_eq!(retirar(&mut h, 10), Ok(()));
        assert!(!h.escuadras[0].en_guarnicion);
    }

    #[test]
    fn dejar_de_residir_suelta_la_guarnicion() {
        let r = r();
        let mut h = heroe_en(5, &[Tropa::Arqueros, Tropa::Honderos]);
        asignar(&r, &mut h, 5, 99, 10).unwrap();
        asignar(&r, &mut h, 5, 99, 11).unwrap();
        soltar(&mut h);
        assert_eq!(ocupada(&r, &h), 0);
        assert!(h.escuadras.iter().all(|e| !e.en_guarnicion));
    }
}
