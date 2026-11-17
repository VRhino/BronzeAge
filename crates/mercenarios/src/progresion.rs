//! La progresión del mercenario sobre su [`Ficha`](crate::Ficha) (diseño §8.2, §8.3 y §9): renombre, rangos y premios, medallas, licencia de reclutar y valoración.

use crate::datos::Datos;
use crate::tipos::{Ficha, Medalla, Objetivo};

impl Ficha {
    /// Suma renombre y devuelve, en orden, los rangos nuevos que alcanza (§8.3). Cada uno da una escuadra de premio: la ficha nace en el rango 1, así que todo rango nuevo
    /// es desde el 2, y saltar varios de golpe da un premio por cada uno. El rango alcanzado no baja, así que perder renombre y recuperarlo no repite el premio.
    pub fn sumar_renombre(&mut self, d: &Datos, n: u32) -> Vec<u8> {
        self.renombre = self.renombre.saturating_add(n);
        let mut nuevos = Vec::new();
        // El rango N es `rangos[N - 1]`: los que aún no tiene empiezan en el índice `rango`.
        for (rango, fila) in ((self.rango + 1)..).zip(d.rangos.iter().skip(usize::from(self.rango))) {
            if fila.renombre > self.renombre {
                break;
            }
            self.rango = rango;
            self.premios += 1;
            nuevos.push(rango);
        }
        nuevos
    }

    /// Resta renombre sin bajar de 0 (duda 2: una mala valoración quita lo apuntado). El rango alcanzado no baja.
    pub fn restar_renombre(&mut self, n: u32) {
        self.renombre = self.renombre.saturating_sub(n);
    }

    /// Suma `n` al contador de la medalla y devuelve el grado nuevo (1 bronce, 2 plata, 3 oro) si cruza un umbral (§9). Si cruza varios de golpe, solo el más alto. El grado
    /// nunca baja, aunque el contador baje, y no se repite: un grado ya ganado no vuelve a devolverse.
    pub fn contar(&mut self, d: &Datos, m: Medalla, n: u32) -> Option<u8> {
        let cuenta = self.contadores.entry(m).or_default();
        *cuenta = cuenta.saturating_add(n);
        let cuenta = *cuenta;
        let alcanzado = (1..=3).zip(d.umbrales(m)).take_while(|&(_, umbral)| umbral <= cuenta).map(|(grado, _)| grado).last()?;
        if alcanzado <= self.grados.get(&m).copied().unwrap_or(0) {
            return None;
        }
        self.grados.insert(m, alcanzado);
        Some(alcanzado)
    }

    /// Escalón máximo de su licencia de reclutar, que sigue al rango alcanzado (§8.3). 0: sin licencia.
    pub fn licencia(&self, d: &Datos) -> u8 {
        usize::from(self.rango).checked_sub(1).and_then(|i| d.rangos.get(i)).map_or(0, |fila| fila.escalon)
    }

    /// Renombre que le falta para el siguiente rango al alcanzado (§8.3); `None` en el último.
    pub fn falta_para_subir(&self, d: &Datos) -> Option<u32> {
        d.rangos.get(usize::from(self.rango)).map(|siguiente| siguiente.renombre.saturating_sub(self.renombre))
    }
}

/// Renombre de cumplir una orden `atacar` ganando el ataque (§8.2): bandidos, por el nivel del campamento (`nivel_bandidos`; sin nivel no dan nada); el resto de lo que se
/// puede atacar, la cifra fija de su clase. Lo que no se ataca (convocatoria, Facción, punto) no da nada.
pub fn renombre_de_ataque(d: &Datos, objetivo: &Objetivo, nivel_bandidos: Option<u8>) -> u32 {
    let cifras = &d.renombre.atacar;
    match objetivo {
        Objetivo::Bandidos(_) => cifras.bandidos_por_nivel * u32::from(nivel_bandidos.unwrap_or(0)),
        Objetivo::Ejercito(_) => cifras.ejercito,
        Objetivo::Caravana(_) => cifras.caravana,
        Objetivo::Pueblo(_) => cifras.pueblo,
        Objetivo::Avanzada(_) => cifras.avanzada,
        Objetivo::Plaza(_) => cifras.plaza,
        Objetivo::Convocatoria(_) | Objetivo::Faccion(_) | Objetivo::Punto(_) => 0,
    }
}

/// Lo que suma una buena valoración sobre lo apuntado del contrato: `buenaPct` menos el 100 % que ya tiene, redondeado hacia abajo (§8.2, duda 2). Nunca resta.
pub fn extra_por_buena(d: &Datos, apuntado: u32) -> u32 {
    apuntado * d.buena_pct.saturating_sub(100) / 100
}

/// Lo que quita una mala valoración del renombre de la ficha: lo apuntado del contrato menos lo que `malaPct` deja valer, redondeado hacia abajo (§8.2, duda 2). Con `malaPct` 0, todo lo
/// apuntado. Nunca suma.
pub fn quita_por_mala(d: &Datos, apuntado: u32) -> u32 {
    apuntado * 100u32.saturating_sub(d.mala_pct) / 100
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::datos::{RenombreAtaque, ReglasRenombre};
    use base::Punto;

    /// El renombre con el que se alcanza el rango `n` (el 2 en adelante).
    fn umbral_de_rango(d: &Datos, n: u8) -> u32 {
        d.rangos[usize::from(n) - 1].renombre
    }

    #[test]
    fn la_ficha_sube_de_rango_justo_al_llegar_a_su_renombre() {
        let d = Datos::cargar();
        let mut f = Ficha::nueva();
        let lanza = umbral_de_rango(&d, 2);
        assert_eq!(f.sumar_renombre(&d, lanza - 1), Vec::<u8>::new());
        assert_eq!((f.rango, f.premios), (1, 0));
        assert_eq!(f.sumar_renombre(&d, 1), [2]);
        assert_eq!((f.renombre, f.rango, f.premios), (lanza, 2, 1));
        assert_eq!(f.sumar_renombre(&d, 1), Vec::<u8>::new());
    }

    #[test]
    fn saltar_dos_rangos_de_golpe_da_los_dos_en_orden_y_dos_premios() {
        let d = Datos::cargar();
        let mut f = Ficha::nueva();
        assert_eq!(f.sumar_renombre(&d, umbral_de_rango(&d, 3)), [2, 3]);
        assert_eq!((f.rango, f.premios), (3, 2));
    }

    #[test]
    fn el_ultimo_rango_no_sube_mas_aunque_siga_ganando() {
        let d = Datos::cargar();
        let ultimo = u8::try_from(d.rangos.len()).unwrap();
        let mut f = Ficha::nueva();
        assert_eq!(f.sumar_renombre(&d, umbral_de_rango(&d, ultimo)), (2..=ultimo).collect::<Vec<_>>());
        assert_eq!((f.rango, f.premios), (ultimo, ultimo - 1));
        assert_eq!(f.sumar_renombre(&d, 10_000), Vec::<u8>::new());
        assert_eq!((f.rango, f.premios), (ultimo, ultimo - 1));
    }

    #[test]
    fn restar_no_baja_de_cero_ni_el_rango_y_recuperarlo_no_repite_el_premio() {
        let d = Datos::cargar();
        let lanza = umbral_de_rango(&d, 2);
        let mut f = Ficha::nueva();
        f.sumar_renombre(&d, lanza + 5);
        f.restar_renombre(10);
        assert_eq!((f.renombre, f.rango, f.premios), (lanza - 5, 2, 1));
        f.restar_renombre(u32::MAX);
        assert_eq!((f.renombre, f.rango, f.premios), (0, 2, 1));
        assert_eq!(f.sumar_renombre(&d, lanza), Vec::<u8>::new());
        assert_eq!((f.rango, f.premios), (2, 1));
    }

    #[test]
    fn cada_medalla_cruza_sus_tres_umbrales_justo_al_llegar() {
        let d = Datos::cargar();
        for m in Medalla::TODAS {
            let [bronce, plata, oro] = d.umbrales(m);
            let mut f = Ficha::nueva();
            assert_eq!(f.contar(&d, m, bronce - 1), None, "{m:?}");
            assert_eq!(f.contar(&d, m, 1), Some(1), "{m:?}");
            assert_eq!(f.contar(&d, m, plata - bronce - 1), None, "{m:?}");
            assert_eq!(f.contar(&d, m, 1), Some(2), "{m:?}");
            assert_eq!(f.contar(&d, m, oro - plata - 1), None, "{m:?}");
            assert_eq!(f.contar(&d, m, 1), Some(3), "{m:?}");
            assert_eq!((f.contadores[&m], f.grados[&m]), (oro, 3), "{m:?}");
        }
    }

    #[test]
    fn cruzar_varios_umbrales_de_golpe_devuelve_solo_el_grado_mas_alto() {
        let d = Datos::cargar();
        let [bronce, _, oro] = d.umbrales(Medalla::Muralla);
        let mut f = Ficha::nueva();
        assert_eq!(f.contar(&d, Medalla::Muralla, bronce + 1), Some(1));
        assert_eq!(f.contar(&d, Medalla::Muralla, oro), Some(3));
        assert_eq!(f.grados[&Medalla::Muralla], 3);
    }

    #[test]
    fn una_medalla_no_repite_grado_y_el_grado_no_baja_con_el_contador() {
        let d = Datos::cargar();
        let [bronce, plata, oro] = d.umbrales(Medalla::PalabraCumplida);
        let mut f = Ficha::nueva();
        assert_eq!(f.contar(&d, Medalla::PalabraCumplida, plata), Some(2));
        assert_eq!(f.contar(&d, Medalla::PalabraCumplida, 1), None);
        // Una mala valoración le quita cuenta a Palabra cumplida (T9): el grado ganado se queda.
        f.contadores.insert(Medalla::PalabraCumplida, 0);
        assert_eq!(f.contar(&d, Medalla::PalabraCumplida, bronce), None);
        assert_eq!(f.grados[&Medalla::PalabraCumplida], 2);
        assert_eq!(f.contar(&d, Medalla::PalabraCumplida, oro), Some(3));
    }

    #[test]
    fn la_licencia_sigue_al_rango_alcanzado_aunque_baje_el_renombre() {
        let d = Datos::cargar();
        assert_eq!(Ficha::default().licencia(&d), 0);
        let mut f = Ficha::nueva();
        assert_eq!(f.licencia(&d), 0);
        f.sumar_renombre(&d, umbral_de_rango(&d, 2));
        assert_eq!(f.licencia(&d), d.rangos[1].escalon);
        assert!(f.licencia(&d) > 0);
        f.restar_renombre(u32::MAX);
        assert_eq!(f.licencia(&d), d.rangos[1].escalon);
        f.sumar_renombre(&d, u32::MAX);
        assert_eq!(f.licencia(&d), d.rangos.last().unwrap().escalon);
    }

    #[test]
    fn falta_para_subir_cuenta_hasta_el_siguiente_del_alcanzado_y_no_hay_tras_el_ultimo() {
        let d = Datos::cargar();
        let mut f = Ficha::nueva();
        assert_eq!(f.falta_para_subir(&d), Some(umbral_de_rango(&d, 2)));
        f.sumar_renombre(&d, 30);
        assert_eq!(f.falta_para_subir(&d), Some(umbral_de_rango(&d, 2) - 30));
        // Con el rango 2 recién alcanzado, lo que falta es para el 3.
        f.sumar_renombre(&d, umbral_de_rango(&d, 2));
        assert_eq!(f.falta_para_subir(&d), Some(umbral_de_rango(&d, 3) - umbral_de_rango(&d, 2) - 30));
        // Con el rango 2 ya alcanzado, perder renombre no cambia el rango que viene.
        f.restar_renombre(u32::MAX);
        assert_eq!(f.falta_para_subir(&d), Some(umbral_de_rango(&d, 3)));
        f.sumar_renombre(&d, u32::MAX);
        assert_eq!(f.falta_para_subir(&d), None);
    }

    #[test]
    fn cada_clase_de_ataque_da_su_cifra_y_los_bandidos_la_multiplican_por_nivel() {
        // Cifras distintas entre sí para que confundir dos clases se note.
        let atacar = RenombreAtaque { bandidos_por_nivel: 2, ejercito: 3, caravana: 5, pueblo: 7, avanzada: 11, plaza: 13 };
        let d = Datos { renombre: ReglasRenombre { atacar, ..Datos::cargar().renombre }, ..Datos::cargar() };
        assert_eq!(renombre_de_ataque(&d, &Objetivo::Bandidos(1), Some(1)), 2);
        assert_eq!(renombre_de_ataque(&d, &Objetivo::Bandidos(1), Some(4)), 8);
        assert_eq!(renombre_de_ataque(&d, &Objetivo::Bandidos(1), None), 0);
        assert_eq!(renombre_de_ataque(&d, &Objetivo::Ejercito(1), None), 3);
        assert_eq!(renombre_de_ataque(&d, &Objetivo::Caravana(1), None), 5);
        assert_eq!(renombre_de_ataque(&d, &Objetivo::Pueblo(1), None), 7);
        assert_eq!(renombre_de_ataque(&d, &Objetivo::Avanzada(1), None), 11);
        assert_eq!(renombre_de_ataque(&d, &Objetivo::Plaza(1), None), 13);
        // El nivel de unos bandidos no cuenta para nada que no sean bandidos.
        assert_eq!(renombre_de_ataque(&d, &Objetivo::Plaza(1), Some(9)), 13);
    }

    #[test]
    fn lo_que_no_se_ataca_no_da_renombre_de_ataque() {
        let d = Datos::cargar();
        assert_eq!(renombre_de_ataque(&d, &Objetivo::Convocatoria(1), None), 0);
        assert_eq!(renombre_de_ataque(&d, &Objetivo::Faccion(1), None), 0);
        assert_eq!(renombre_de_ataque(&d, &Objetivo::Punto(Punto { x: 0, y: 0 }), None), 0);
    }

    #[test]
    fn la_buena_valoracion_suma_el_porcentaje_de_mas_redondeando_hacia_abajo() {
        let d = Datos { buena_pct: 175, ..Datos::cargar() };
        assert_eq!(extra_por_buena(&d, 100), 75);
        assert_eq!(extra_por_buena(&d, 25), 18);
        assert_eq!(extra_por_buena(&d, 0), 0);
        // Una «buena» nunca resta, ni siquiera con un porcentaje por debajo del 100 %.
        let d = Datos { buena_pct: 80, ..Datos::cargar() };
        assert_eq!(extra_por_buena(&d, 100), 0);
    }

    #[test]
    fn la_mala_valoracion_quita_lo_que_el_contrato_deja_de_valer_redondeando_hacia_abajo() {
        let con = |mala_pct| Datos { mala_pct, ..Datos::cargar() };
        assert_eq!(quita_por_mala(&con(0), 25), 25, "con malaPct 0 se pierde todo lo apuntado");
        assert_eq!(quita_por_mala(&con(40), 25), 15);
        assert_eq!(quita_por_mala(&con(30), 25), 17, "25 × 70 % = 17,5 hacia abajo");
        assert_eq!(quita_por_mala(&con(100), 25), 0);
        // Una «mala» nunca suma, ni siquiera con un porcentaje por encima del 100 %.
        assert_eq!(quita_por_mala(&con(150), 25), 0);
        assert_eq!(quita_por_mala(&con(0), 0), 0);
    }
}
