//! El reparto automático de caravanas a los lados pendientes de un trueque (Comercio_Caravanas §4.3). La lógica pura solo responde a
//! «¿qué asigno ahora?»: el núcleo la invoca cuando se acepta un trueque, cuando queda libre una caravana y con un pulso configurable
//! (el reparto ya no corre cada tick). El trazado de la ruta (A\*, red de caminos) es de `crates/rutas`: entra como una función
//! inyectada que devuelve el trazado o `None` si no hay camino por tierra.

use crate::datos::ReglasAsignacion;
use crate::tipos::{AcuerdoId, AsentamientoId, Carga, CaravanaId, Lado};
use crate::trueque::{AcuerdoTrueque, EstadoTrueque};
use asentamiento::datos::{por, razon, UNO};
use asentamiento::{Almacen, Recurso};
use base::{Instante, Mil};

/// Un cargamento que todavía debe salir: lo que falta de un lado de un trueque `activo` vivo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LadoPendiente {
    pub acuerdo: AcuerdoId,
    pub lado: Lado,
    pub origen: AsentamientoId,
    pub destino: AsentamientoId,
    /// Líneas con faltante, en el orden del acuerdo.
    pub lineas: Vec<(Recurso, Mil)>,
    /// Puntuación 0..100 en ppm (100 puntos = 100 × `UNO`).
    pub score: i64,
}

/// Una caravana que podría salir: id y capacidad **efectiva** (`capacidadCaravana × factorCapacidadCaravana(origen)`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Candidata {
    pub caravana: CaravanaId,
    pub capacidad: Mil,
}

/// Lo que el reparto necesita saber del mundo, inyectado.
pub struct EntornoReparto<'a> {
    pub ahora: Instante,
    /// Distancia en línea recta entre dos asentamientos.
    pub distancia: &'a dyn Fn(AsentamientoId, AsentamientoId) -> Mil,
    /// ¿Existe el asentamiento? (un lado hacia un destino que ya no existe no entra).
    pub existe: &'a dyn Fn(AsentamientoId) -> bool,
    /// ¿Hay ya una caravana `en_transito` con `origenAcuerdoId == acuerdo` y ese `ladoAcuerdo`? (un solo cargamento en ruta por lado).
    pub en_ruta: &'a dyn Fn(AcuerdoId, Lado) -> bool,
}

/// `scoreAsignacion` (§4.3), en ppm de punto (0..100 × `UNO`): `0,5·urgenciaExpiración + 0,3·urgenciaVolumen + 0,2·cercanía`.
///
/// - `urgenciaExpiracion = 100 × (1 − max(0, expiraEn − ahora) / (expiraEn − aceptadoEn))`: la urgencia se cuenta **desde la
///   aceptación** (decidido el 2026-10-18), no desde la propuesta; 100 si el plazo total es 0.
/// - `urgenciaVolumen = 100 × min(1, (total − entregado) / total)`, sumando todo el lado.
/// - `cercania = 100 × (1 − min(1, distancia / distanciaReferencia))`.
pub fn score_asignacion(r: &ReglasAsignacion, a: &AcuerdoTrueque, lado: Lado, ahora: Instante, distancia: Mil) -> i64 {
    let inicio = a.aceptado_en.unwrap_or(a.creado_en);
    let plazo = a.expira_en - inicio;
    let urg_exp = if plazo <= 0 { 100 * UNO } else { 100 * (UNO - razon((a.expira_en - ahora).clamp(0, plazo), plazo)) };
    let (total, entregado) = a.lineas(lado).iter().fold((0, 0), |(t, e), l| (t + l.cantidad_total, e + l.cantidad_entregada));
    let urg_vol = if total <= 0 { 0 } else { 100 * razon((total - entregado).clamp(0, total), total) };
    let cercania = 100 * (UNO - razon(distancia.max(0), r.distancia_referencia).min(UNO));
    por(urg_exp, r.peso_urgencia_expiracion) + por(urg_vol, r.peso_urgencia_volumen) + por(cercania, r.peso_cercania)
}

/// Los lados pendientes de todos los acuerdos, en el orden del array: solo `activo` vivo (`ahora < expira_en`; el vencimiento lo cierra
/// `AcuerdoTrueque::vencer` antes), con algo por entregar, sin cargamento ya en ruta y con destino existente.
pub fn lados_pendientes(r: &ReglasAsignacion, acuerdos: &[AcuerdoTrueque], e: &EntornoReparto) -> Vec<LadoPendiente> {
    let mut v = Vec::new();
    for a in acuerdos.iter().filter(|a| a.estado == EstadoTrueque::Activo && e.ahora < a.expira_en) {
        for lado in [Lado::A, Lado::B] {
            let lineas = a.pendiente(lado);
            let (origen, destino) = (a.origen_de(lado), a.destino_de(lado));
            if lineas.is_empty() || (e.en_ruta)(a.id, lado) || !(e.existe)(destino) {
                continue;
            }
            let score = score_asignacion(r, a, lado, e.ahora, (e.distancia)(origen, destino));
            v.push(LadoPendiente { acuerdo: a.id, lado, origen, destino, lineas, score });
        }
    }
    v
}

/// Los asentamientos de origen con algo pendiente, **en el orden en que aparecen** (sin repetir).
pub fn origenes(pendientes: &[LadoPendiente]) -> Vec<AsentamientoId> {
    let mut o = Vec::new();
    for p in pendientes {
        if !o.contains(&p.origen) {
            o.push(p.origen);
        }
    }
    o
}

/// Una caravana asignada a un cargamento. La carga ya salió del almacén de origen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asignacion<T> {
    pub caravana: CaravanaId,
    pub acuerdo: AcuerdoId,
    pub lado: Lado,
    pub origen: AsentamientoId,
    pub destino: AsentamientoId,
    pub carga: Carga,
    /// Lo que devolvió el trazador para esta ruta (el núcleo lo usa para crear el plan y registrar la ruta en la red).
    pub trazado: T,
}

/// Reparte las caravanas candidatas de **un** asentamiento de origen entre sus lados pendientes (§4.3), por score descendente (orden
/// estable). Por cada lado, la caravana `idx` carga las líneas en el orden del acuerdo hasta llenarse o agotar el stock de cada una:
///
/// - sin carga ⇒ no gasta la caravana (se reintenta con el siguiente lado);
/// - sin ruta por tierra (`trazar` da `None`) ⇒ no sale y **no gasta** la caravana;
/// - con carga y ruta ⇒ la carga **sale ya del almacén** y la caravana se gasta.
///
/// `candidatas` va en el orden del array de caravanas del núcleo y ya filtrada (comercial, `disponible`, no reservada a mano, de este
/// origen y con capacidad > 0: [`crate::caravana::Caravana::es_candidata_de_reparto`]).
pub fn asignar_origen<T>(
    origen: AsentamientoId,
    pendientes: &[LadoPendiente],
    candidatas: &[Candidata],
    almacen: &mut Almacen,
    trazar: &mut dyn FnMut(AsentamientoId, AsentamientoId) -> Option<T>,
) -> Vec<Asignacion<T>> {
    let mut lados: Vec<&LadoPendiente> = pendientes.iter().filter(|p| p.origen == origen).collect();
    lados.sort_by_key(|p| std::cmp::Reverse(p.score)); // estable
    let mut salidas = Vec::new();
    let mut idx = 0;
    for p in lados {
        let Some(c) = candidatas.get(idx) else { break };
        let mut libre = c.capacidad;
        let mut carga = Carga::new();
        for &(r, faltante) in &p.lineas {
            let n = almacen.cantidad(r).min(faltante).min(libre);
            if n > 0 {
                carga.insert(r, n);
                libre -= n;
            }
        }
        if carga.is_empty() {
            continue;
        }
        let Some(trazado) = trazar(origen, p.destino) else { continue };
        idx += 1;
        for (&r, &n) in &carga {
            almacen.quitar(r, n);
        }
        salidas.push(Asignacion { caravana: c.caravana, acuerdo: p.acuerdo, lado: p.lado, origen, destino: p.destino, carga, trazado });
    }
    salidas
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::datos::Reglas;
    use Recurso::*;

    const MIN: i64 = 60_000;

    fn acuerdo(r: &Reglas, id: u64, a: u32, b: u32, aceptado: Instante) -> AcuerdoTrueque {
        let mut x = AcuerdoTrueque::proponer(&r.trueque, id, a, b, &[(Madera, 100_000), (Piedra, 50_000)], &[(Cobre, 30_000)], 0, true).unwrap();
        x.aceptar(&r.trueque, aceptado, true).unwrap();
        x
    }

    fn entorno<'a>(ahora: Instante, d: &'a dyn Fn(u32, u32) -> Mil, ex: &'a dyn Fn(u32) -> bool, er: &'a dyn Fn(u64, Lado) -> bool) -> EntornoReparto<'a> {
        EntornoReparto { ahora, distancia: d, existe: ex, en_ruta: er }
    }

    #[test]
    fn el_score_pondera_expiracion_volumen_y_cercania() {
        let r = Reglas::cargar();
        let a = acuerdo(&r, 1, 1, 2, 0);
        // Recién aceptado, nada entregado, a distancia 0: 0 de urgencia por plazo, 100 de volumen, 100 de cercanía → 0,3·100 + 0,2·100 = 50.
        assert_eq!(score_asignacion(&r.asignacion, &a, Lado::A, 0, 0), 50 * UNO);
        // A mitad de plazo, a 60 unidades (la mitad de la referencia): 0,5·50 + 0,3·100 + 0,2·50 = 65.
        assert_eq!(score_asignacion(&r.asignacion, &a, Lado::A, 720 * MIN, 60_000), 65 * UNO);
        // Con el plazo agotado y lejos: 0,5·100 + 0,3·100 + 0 = 80.
        assert_eq!(score_asignacion(&r.asignacion, &a, Lado::A, 2_000 * MIN, 999_000), 80 * UNO);
    }

    #[test]
    fn la_urgencia_se_cuenta_desde_la_aceptacion_no_desde_la_propuesta() {
        let r = Reglas::cargar();
        // Propuesta en 0 y aceptada 1000 min después: el plazo corre desde el sí.
        let a = acuerdo(&r, 1, 1, 2, 1_000 * MIN);
        assert_eq!(a.creado_en, 0);
        assert_eq!(score_asignacion(&r.asignacion, &a, Lado::A, 1_000 * MIN, 120_000), 30 * UNO, "0 de urgencia por plazo: acaba de empezar");
        // Si se contara desde la creación (plazo = 2440 min) la urgencia ya sería > 40: no lo es.
        assert_eq!(score_asignacion(&r.asignacion, &a, Lado::A, 1_000 * MIN + 720 * MIN, 120_000), 55 * UNO);
    }

    #[test]
    fn el_volumen_pesa_lo_que_falta_de_todo_el_lado() {
        let r = Reglas::cargar();
        let mut a = acuerdo(&r, 1, 1, 2, 0);
        let mut c = Carga::new();
        c.insert(Madera, 100_000);
        a.abonar(Lado::A, &c, (1, 2));
        // Falta 50 de 150 → 33,3 puntos de volumen.
        assert_eq!(score_asignacion(&r.asignacion, &a, Lado::A, 0, 120_000), por(100 * razon(50_000, 150_000), 300_000));
    }

    #[test]
    fn plazo_total_cero_cuenta_como_urgencia_maxima() {
        let r = Reglas::cargar();
        let mut a = acuerdo(&r, 1, 1, 2, 0);
        a.expira_en = 0;
        assert_eq!(score_asignacion(&r.asignacion, &a, Lado::A, 0, 120_000), 50 * UNO + 30 * UNO);
    }

    #[test]
    fn los_lados_pendientes_solo_de_activos_vivos_sin_cargamento_en_ruta_y_con_destino() {
        let r = Reglas::cargar();
        let mut v = vec![acuerdo(&r, 1, 1, 2, 0), acuerdo(&r, 2, 1, 3, 0), acuerdo(&r, 3, 1, 4, 0), acuerdo(&r, 4, 1, 5, 0)];
        v[1].estado = EstadoTrueque::Propuesto;
        let d = |_, _| 10_000;
        let ex = |x| x != 4;
        let er = |id, lado| id == 4 && lado == Lado::A;
        let p = lados_pendientes(&r.asignacion, &v, &entorno(10, &d, &ex, &er));
        let claves: Vec<_> = p.iter().map(|x| (x.acuerdo, x.lado)).collect();
        assert_eq!(claves, vec![(1, Lado::A), (1, Lado::B), (3, Lado::B), (4, Lado::B)], "el 2 no está activo, el 3-A va a la plaza inexistente 4 (el 3-B sale de ella hacia 1 y sí entra), el 4-A ya está en ruta");
        assert_eq!((p[1].origen, p[1].destino), (2, 1));
        assert_eq!(p[0].lineas, vec![(Madera, 100_000), (Piedra, 50_000)]);
        // Vencido: no entra.
        let p = lados_pendientes(&r.asignacion, &v, &entorno(v[0].expira_en, &d, &ex, &er));
        assert!(p.is_empty());
    }

    #[test]
    fn los_origenes_salen_en_orden_de_aparicion() {
        let r = Reglas::cargar();
        let v = vec![acuerdo(&r, 1, 7, 2, 0), acuerdo(&r, 2, 2, 7, 0)];
        let (d, ex, er) = (|_, _| 0, |_| true, |_, _| false);
        let p = lados_pendientes(&r.asignacion, &v, &entorno(0, &d, &ex, &er));
        assert_eq!(origenes(&p), vec![7, 2]);
    }

    fn almacen(items: &[(Recurso, Mil)]) -> Almacen {
        let mut a = Almacen::nuevo(10_000_000);
        for &(r, n) in items {
            a.poner(r, n);
        }
        a
    }

    fn un_lado(acuerdo: u64, score: i64, destino: u32, lineas: Vec<(Recurso, Mil)>) -> LadoPendiente {
        LadoPendiente { acuerdo, lado: Lado::A, origen: 1, destino, lineas, score }
    }

    #[test]
    fn carga_las_lineas_en_orden_hasta_llenar_la_caravana() {
        let mut alm = almacen(&[(Madera, 900_000), (Piedra, 900_000)]);
        let p = [un_lado(1, 10, 2, vec![(Madera, 100_000), (Piedra, 50_000)])];
        let c = [Candidata { caravana: 7, capacidad: 120_000 }];
        let s = asignar_origen(1, &p, &c, &mut alm, &mut |_, _| Some(()));
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].carga[&Madera], 100_000);
        assert_eq!(s[0].carga[&Piedra], 20_000, "la caravana se llena: lo que no cabe va en el siguiente viaje");
        assert_eq!((alm.cantidad(Madera), alm.cantidad(Piedra)), (800_000, 880_000), "la carga sale ya del almacén");
    }

    #[test]
    fn sin_stock_de_una_linea_sale_lo_de_las_demas() {
        let mut alm = almacen(&[(Piedra, 30_000)]);
        let p = [un_lado(1, 10, 2, vec![(Madera, 100_000), (Piedra, 50_000)])];
        let c = [Candidata { caravana: 7, capacidad: 500_000 }];
        let s = asignar_origen(1, &p, &c, &mut alm, &mut |_, _| Some(()));
        assert_eq!(s[0].carga.len(), 1);
        assert_eq!(s[0].carga[&Piedra], 30_000);
    }

    #[test]
    fn lado_sin_carga_no_gasta_la_caravana() {
        let mut alm = almacen(&[(Cobre, 10_000)]);
        let p = [un_lado(1, 90, 2, vec![(Madera, 100_000)]), un_lado(2, 50, 3, vec![(Cobre, 10_000)])];
        let c = [Candidata { caravana: 7, capacidad: 500_000 }];
        let s = asignar_origen(1, &p, &c, &mut alm, &mut |_, _| Some(()));
        assert_eq!(s.len(), 1);
        assert_eq!((s[0].acuerdo, s[0].caravana), (2, 7), "el primer lado no tenía stock: la caravana se reintenta con el siguiente");
    }

    #[test]
    fn sin_ruta_por_tierra_no_sale_ni_gasta_la_caravana() {
        let mut alm = almacen(&[(Madera, 900_000)]);
        let p = [un_lado(1, 90, 2, vec![(Madera, 100_000)]), un_lado(2, 50, 3, vec![(Madera, 100_000)])];
        let c = [Candidata { caravana: 7, capacidad: 500_000 }];
        let s = asignar_origen(1, &p, &c, &mut alm, &mut |_, d| (d == 3).then_some("ruta"));
        assert_eq!(s.len(), 1);
        assert_eq!((s[0].acuerdo, s[0].trazado), (2, "ruta"));
        assert_eq!(alm.cantidad(Madera), 800_000, "solo salió una carga");
    }

    #[test]
    fn menos_caravanas_que_lados_gana_el_mayor_score_y_el_empate_respeta_el_orden() {
        let mut alm = almacen(&[(Madera, 900_000)]);
        let p = [un_lado(1, 10, 2, vec![(Madera, 100_000)]), un_lado(2, 80, 3, vec![(Madera, 100_000)]), un_lado(3, 80, 4, vec![(Madera, 100_000)])];
        let c = [Candidata { caravana: 7, capacidad: 500_000 }, Candidata { caravana: 8, capacidad: 500_000 }];
        let s = asignar_origen(1, &p, &c, &mut alm, &mut |_, _| Some(()));
        assert_eq!(s.iter().map(|x| (x.acuerdo, x.caravana)).collect::<Vec<_>>(), vec![(2, 7), (3, 8)], "orden descendente, estable en el empate");
    }

    #[test]
    fn solo_atiende_los_lados_de_su_origen_y_sin_candidatas_no_hace_nada() {
        let mut alm = almacen(&[(Madera, 900_000)]);
        let mut ajeno = un_lado(2, 99, 3, vec![(Madera, 1_000)]);
        ajeno.origen = 9;
        let p = [ajeno, un_lado(1, 10, 2, vec![(Madera, 100_000)])];
        assert!(asignar_origen(1, &p, &[], &mut alm, &mut |_, _| Some(())).is_empty());
        let c = [Candidata { caravana: 7, capacidad: 500_000 }];
        let s = asignar_origen(1, &p, &c, &mut alm, &mut |_, _| Some(()));
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].acuerdo, 1);
    }

    #[test]
    fn el_reparto_conserva_los_recursos_y_es_determinista() {
        let hacer = || {
            let mut alm = almacen(&[(Madera, 250_000), (Piedra, 70_000)]);
            let p = [un_lado(1, 5, 2, vec![(Madera, 200_000), (Piedra, 100_000)]), un_lado(2, 7, 3, vec![(Madera, 150_000)])];
            let c = [Candidata { caravana: 1, capacidad: 180_000 }, Candidata { caravana: 2, capacidad: 180_000 }];
            let s = asignar_origen(1, &p, &c, &mut alm, &mut |_, _| Some(()));
            let sacado: i64 = s.iter().flat_map(|x| x.carga.values()).sum();
            assert_eq!(sacado + alm.cantidad(Madera) + alm.cantidad(Piedra), 320_000, "lo que sale + lo que queda = lo que había");
            s
        };
        assert_eq!(hacer(), hacer());
    }
}
