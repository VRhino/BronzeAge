//! Mantenimiento y nivel operativo (Doc 4.5, Economia_Construccion §10): el coste escala con los edificios activos y con la
//! distancia a la capital; el medidor baja con el déficit; sin pagar, el asentamiento pierde un nivel operativo y en el
//! nivel 1 cae en ruinas. El trigo no entra: tiene su propio medidor (nutrición).

use crate::catalogo::Tipo;
use crate::datos::{mil, por, ppm, razon, UNO};
use crate::edificio::Edificio;
use crate::recurso::{Costo, Recurso};
use base::Mil;
use serde_json::Value;

#[derive(Clone, Debug)]
pub struct ReglasMantenimiento {
    pub medidor_inicial: Mil,
    pub costo_base_madera: Mil,
    pub nivel_para_piedra: u32,
    pub piedra_base: Mil,
    pub nivel_para_oro: u32,
    pub oro_base: Mil,
    pub edificios_referencia: i64,
    pub peso_vivienda: i64,
    pub peso_pieza_gratuita: i64,
    pub piezas_gratuitas: Vec<Tipo>,
    /// Distancia a la capital (en milésimas de unidad de mapa) que duplica el coste. **80 unidades**, no las 400 del bloque de
    /// TypeScript (Economia_Construccion §15, calibración de distancias).
    pub escala_distancia: Mil,
    pub degradacion_por_deficit_total: Mil,
    pub regeneracion_si_pago_completo: Mil,
    pub gracia_minutos: i64,
    pub minutos_sanos_para_recuperar_nivel: u32,
}

impl ReglasMantenimiento {
    pub fn de(b: &Value) -> Self {
        let g = |k: &str| b.get(k).and_then(Value::as_f64).unwrap_or_else(|| panic!("falta «{k}»"));
        ReglasMantenimiento {
            medidor_inicial: mil(g("medidorInicial")),
            costo_base_madera: mil(b["costoBase"]["madera"].as_f64().expect("costoBase.madera")),
            nivel_para_piedra: g("nivelParaPiedra") as u32,
            piedra_base: mil(g("piedraBase")),
            nivel_para_oro: g("nivelParaOro") as u32,
            oro_base: mil(g("oroBase")),
            edificios_referencia: g("edificiosReferencia") as i64,
            peso_vivienda: ppm(g("pesoVivienda")),
            peso_pieza_gratuita: ppm(g("pesoPiezaGratuita")),
            piezas_gratuitas: b["piezasGratuitas"].as_array().expect("piezasGratuitas").iter().filter_map(|x| Tipo::desde_nombre(x.as_str()?)).collect(),
            // Vigente (Economia_Construccion §15): el bloque dice 400, la calibración de distancias dice 80.
            escala_distancia: 80_000,
            degradacion_por_deficit_total: mil(g("degradacionPorDeficitTotal")),
            regeneracion_si_pago_completo: mil(g("regeneracionSiPagoCompleto")),
            gracia_minutos: g("graciaMinutos") as i64,
            minutos_sanos_para_recuperar_nivel: g("minutosSanosParaRecuperarNivel") as u32,
        }
    }
}

/// El medidor de un asentamiento y su nivel operativo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Medidor {
    /// 0..100 en milésimas.
    pub valor: Mil,
    /// Minutos sanos seguidos con el nivel operativo por debajo del alcanzado.
    pub racha: u32,
    /// Nivel operativo: sube y baja con la salud, nunca por encima del alcanzado.
    pub nivel_actual: u32,
}

impl Medidor {
    pub fn nuevo(r: &ReglasMantenimiento) -> Self {
        Self { valor: r.medidor_inicial, racha: 0, nivel_actual: 1 }
    }
}

/// Lo que pasó en un minuto de mantenimiento.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resultado {
    /// En gracia u ocupado: ni cobra ni regenera.
    Nada,
    Sano,
    Recuperado,
    Deficit,
    /// El medidor llegó a 0 y baja un nivel operativo.
    Colapsado,
    /// En el nivel 1 con el medidor a 0: el asentamiento desaparece. Lleva los recursos que no pudo pagar.
    Ruinas { faltantes: Vec<Recurso> },
}

/// `calcularCostoMantenimiento`: madera siempre, piedra desde el nivel 2, oro desde el 3; todo × (1 + edificios/ref) × (1 + dist/escala).
/// `upkeep_recintos` se suma sin escala (Murallas §9.4). Cada uno de los `pueblos` extractores activos cuenta como un edificio activo más
/// (Doc 4.2.2).
pub fn coste(r: &ReglasMantenimiento, edificios: &[Edificio], pueblos: usize, nivel_actual: u32, distancia_capital: Mil, upkeep_recintos: &Costo) -> Costo {
    let suma = pueblos as i64 * UNO + edificios
        .iter()
        .filter(|e| e.activo())
        .map(|e| {
            if e.tipo == Tipo::Vivienda {
                r.peso_vivienda
            } else if r.piezas_gratuitas.contains(&e.tipo) {
                r.peso_pieza_gratuita
            } else {
                UNO
            }
        })
        .sum::<i64>();
    // `suma` está en ppm de edificio: 1 edificio = UNO.
    let escala = por(UNO + razon(suma, r.edificios_referencia * UNO), UNO + razon(distancia_capital.max(0), r.escala_distancia));
    let mut c: Costo = vec![(Recurso::Madera, por(r.costo_base_madera, escala))];
    if nivel_actual >= r.nivel_para_piedra {
        c.push((Recurso::Piedra, por(r.piedra_base, escala)));
    }
    if nivel_actual >= r.nivel_para_oro {
        c.push((Recurso::Oro, por(r.oro_base, escala)));
    }
    for &(rec, n) in upkeep_recintos {
        match c.iter_mut().find(|(x, _)| *x == rec) {
            Some(f) => f.1 += n,
            None => c.push((rec, n)),
        }
    }
    c
}

/// Un minuto de pago y medidor (`avanzarMantenimiento`). `almacen` es el del asentamiento; descuenta lo que paga.
/// `alcanzado` es el nivel alcanzado (el techo del operativo). `minuto_de_vida` son los minutos desde la fundación.
pub fn avanzar_minuto(
    r: &ReglasMantenimiento,
    m: &mut Medidor,
    alcanzado: u32,
    coste: &Costo,
    almacen: &mut crate::almacen::Almacen,
    minutos_de_vida: i64,
    ocupado: bool,
) -> Resultado {
    if minutos_de_vida < r.gracia_minutos || ocupado {
        return Resultado::Nada;
    }
    let cubierta = if coste.is_empty() {
        UNO
    } else {
        coste.iter().map(|&(rec, n)| if n <= 0 { UNO } else { razon(almacen.cantidad(rec), n).min(UNO) }).sum::<i64>() / coste.len() as i64
    };
    if cubierta >= UNO {
        coste.iter().for_each(|&(rec, n)| {
            almacen.quitar(rec, n);
        });
        m.racha += 1;
        m.valor = (m.valor + r.regeneracion_si_pago_completo).min(100_000);
        if m.racha >= r.minutos_sanos_para_recuperar_nivel && m.nivel_actual < alcanzado {
            m.nivel_actual += 1;
            m.racha = 0;
            return Resultado::Recuperado;
        }
        return Resultado::Sano;
    }
    // Se paga todo lo que hay.
    let mut faltantes = vec![];
    for &(rec, n) in coste {
        if almacen.cantidad(rec) < n {
            faltantes.push(rec);
        }
        almacen.quitar(rec, n);
    }
    m.racha = 0;
    m.valor = (m.valor - por(r.degradacion_por_deficit_total, UNO - cubierta)).max(0);
    if m.valor <= 0 {
        if m.nivel_actual > 1 {
            m.nivel_actual -= 1;
            m.valor = r.medidor_inicial;
            return Resultado::Colapsado;
        }
        return Resultado::Ruinas { faltantes };
    }
    Resultado::Deficit
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::almacen::Almacen;
    use crate::datos::Reglas;
    use crate::edificio::Estado;

    fn reglas() -> ReglasMantenimiento {
        Reglas::cargar().mantenimiento
    }

    fn activos(tipos: &[Tipo]) -> Vec<Edificio> {
        tipos.iter().enumerate().map(|(i, &t)| Edificio::nuevo(i as u32, t, Estado::Activo)).collect()
    }

    #[test]
    fn el_coste_escala_con_edificios_y_distancia_y_la_vivienda_cuenta_la_mitad() {
        let r = reglas();
        // 4 normales + 2 viviendas (=1) = 5 edificios; 5/50 = 0,1 → ×1,1. Nivel 1: solo madera 1,5 × 1,1 = 1,65.
        let e = activos(&[Tipo::CentroUrbano, Tipo::Granja, Tipo::Lenera, Tipo::Cantera, Tipo::Vivienda, Tipo::Vivienda]);
        assert_eq!(coste(&r, &e, 0, 1, 0, &vec![]), vec![(Recurso::Madera, 1_650)]);
        // A 80 unidades de la capital el coste se duplica.
        assert_eq!(coste(&r, &e, 0, 1, 80_000, &vec![]), vec![(Recurso::Madera, 3_300)]);
        // Piedra desde el nivel 2 (3 base) y oro desde el 3 (2 base).
        let c3 = coste(&r, &e, 0, 3, 0, &vec![]);
        assert_eq!(c3, vec![(Recurso::Madera, 1_650), (Recurso::Piedra, 3_300), (Recurso::Oro, 2_200)]);
        // Un edificio en cola o en obra no paga mantenimiento.
        let mut e2 = e.clone();
        e2.push(Edificio::nuevo(9, Tipo::Granja, Estado::EnCola));
        assert_eq!(coste(&r, &e2, 0, 1, 0, &vec![]), vec![(Recurso::Madera, 1_650)]);
        // El upkeep de recintos se suma sin escala.
        assert_eq!(coste(&r, &e, 0, 1, 80_000, &vec![(Recurso::Madera, 500)])[0], (Recurso::Madera, 3_800));
    }

    #[test]
    fn cada_pueblo_activo_cuenta_como_un_edificio_activo_de_la_madre() {
        let r = reglas();
        let e = activos(&[Tipo::CentroUrbano, Tipo::Granja, Tipo::Lenera, Tipo::Cantera, Tipo::Vivienda, Tipo::Vivienda]);
        // 5 edificios + 2 pueblos = 7; 7/50 = 0,14 → 1,5 × 1,14 = 1,71.
        assert_eq!(coste(&r, &e, 2, 1, 0, &vec![]), vec![(Recurso::Madera, 1_710)]);
    }

    #[test]
    fn las_piezas_gratuitas_cuentan_la_mitad() {
        let r = reglas();
        let e = activos(&[Tipo::CentroUrbano, Tipo::Plaza, Tipo::Pozo, Tipo::PuestoMercado]);
        // 1 + 0,5 × 3 = 2,5 edificios → 2,5/50 = 0,05 → 1,5 × 1,05 = 1,575.
        assert_eq!(coste(&r, &e, 0, 1, 0, &vec![]), vec![(Recurso::Madera, 1_575)]);
    }

    #[test]
    fn la_escala_de_distancia_vigente_es_80_no_400() {
        assert_eq!(reglas().escala_distancia, 80_000);
    }

    #[test]
    fn la_gracia_no_cobra_ni_regenera() {
        let r = reglas();
        let (mut m, mut a) = (Medidor::nuevo(&r), Almacen::nuevo(400_000));
        a.poner(Recurso::Madera, 100_000);
        let c = vec![(Recurso::Madera, 1_650)];
        assert_eq!(avanzar_minuto(&r, &mut m, 1, &c, &mut a, 1439, false), Resultado::Nada);
        assert_eq!(a.cantidad(Recurso::Madera), 100_000);
        assert_eq!(avanzar_minuto(&r, &mut m, 1, &c, &mut a, 1440, true), Resultado::Nada, "ocupado: tampoco");
        assert_eq!(avanzar_minuto(&r, &mut m, 1, &c, &mut a, 1440, false), Resultado::Sano);
        assert_eq!(a.cantidad(Recurso::Madera), 98_350);
    }

    #[test]
    fn el_deficit_baja_el_medidor_proporcional_y_el_pago_completo_lo_regenera() {
        let r = reglas();
        let (mut m, mut a) = (Medidor::nuevo(&r), Almacen::nuevo(400_000));
        m.valor = 50_000;
        // Solo hay la mitad de la madera: cubierta 0,5 → medidor −10 × 0,5 = −5.
        a.poner(Recurso::Madera, 1_000);
        let c = vec![(Recurso::Madera, 2_000)];
        assert_eq!(avanzar_minuto(&r, &mut m, 1, &c, &mut a, 5_000, false), Resultado::Deficit);
        assert_eq!((m.valor, a.cantidad(Recurso::Madera)), (45_000, 0), "paga todo lo que hay");
        a.poner(Recurso::Madera, 10_000);
        assert_eq!(avanzar_minuto(&r, &mut m, 1, &c, &mut a, 5_001, false), Resultado::Sano);
        assert_eq!(m.valor, 50_000, "+5 con el pago completo");
    }

    #[test]
    fn sin_pagar_nada_el_medidor_cae_en_diez_minutos_luego_baja_un_nivel_y_en_el_uno_hay_ruinas() {
        let r = reglas();
        let (mut m, mut a) = (Medidor::nuevo(&r), Almacen::nuevo(400_000));
        m.nivel_actual = 2;
        let c = vec![(Recurso::Madera, 2_000), (Recurso::Piedra, 3_000)];
        let (mut resultados, mut tras_colapso) = (vec![], (0, 0));
        for i in 0..40 {
            resultados.push(avanzar_minuto(&r, &mut m, 2, &c, &mut a, 5_000 + i, false));
            if i == 9 {
                tras_colapso = (m.nivel_actual, m.valor);
            }
        }
        assert_eq!(resultados[8], Resultado::Deficit);
        assert_eq!(resultados[9], Resultado::Colapsado, "100 − 10 × 10 minutos = 0");
        assert_eq!(tras_colapso, (1, 100_000), "baja un nivel y el medidor vuelve a empezar");
        assert!(matches!(resultados[19], Resultado::Ruinas { ref faltantes } if faltantes.contains(&Recurso::Madera)), "{:?}", resultados[19]);
    }

    #[test]
    fn tras_treinta_minutos_sanos_recupera_un_nivel() {
        let r = reglas();
        let (mut m, mut a) = (Medidor::nuevo(&r), Almacen::nuevo(400_000));
        a.poner(Recurso::Madera, 400_000);
        let c = vec![(Recurso::Madera, 1_000)];
        let mut fin = Resultado::Nada;
        for i in 0..30 {
            fin = avanzar_minuto(&r, &mut m, 3, &c, &mut a, 5_000 + i, false);
        }
        assert_eq!((fin, m.nivel_actual, m.racha), (Resultado::Recuperado, 2, 0));
    }
}
