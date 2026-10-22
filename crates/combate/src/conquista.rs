//! Cuando cae una plaza (Combate §8-§10, Doc 5.15.5): el saqueo de la ciudad, el desalojo de quien estaba dentro y la derrota de la Facción. Lo
//! comparten el combate con números y el resultado de Unity: las consecuencias son las mismas por los dos caminos.
//!
//! Es **puro y determinista** (no usa el generador). Lo que toca a otros sistemas se deja anotado, no se hace aquí:
//! - cargos, fundadores y casas compradas quedan vacíos (sistema 5, política);
//! - la flota, los trueques y las caravanas aparcadas se cierran con `comercio::ruina::cerrar_dependientes(Motivo::Conquista, …)` (la aparcada se
//!   destruye, decidido el 2026-10-19);
//! - los pueblos extractores y las avanzadas de la plaza pasan al conquistador con su zona (la ruina de la madre los hace desaparecer);
//! - los recintos de muralla pierden `fraccionDanoMuralla` de avance cuando existan (fase de Murallas; hoy no hay recintos).

use crate::datos::Reglas;
use asentamiento::asentamiento::Asentamiento;
use asentamiento::catalogo::Tipo;
use asentamiento::datos::UNO;
use asentamiento::edificio::Estado;
use base::{Instante, Mil};
use militar::escuadra::Contenedor;
use militar::heroe::{Heroe, Residencia};
use militar::{AsentamientoId, CampamentoId, ColumnaId, EscuadraId, HeroeId};

/// ¿Sigue inatacable una plaza conquistada? (`ahora < protegidaHasta`.)
pub fn esta_protegida(protegida_hasta: Option<Instante>, ahora: Instante) -> bool {
    protegida_hasta.is_some_and(|h| ahora < h)
}

/// Lo que el núcleo debe anotar de una plaza conquistada.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conquistada {
    /// Inatacable hasta aquí (un día).
    pub protegida_hasta: Instante,
    /// La ocupación dura hasta aquí: recaudación y crecimiento al `factorRecaudacion` y `factorCrecimiento`, mantenimiento congelado.
    pub ocupacion_hasta: Instante,
    /// Los edificios que pasaron a `en_cola` dañados, por id.
    pub edificios_danados: Vec<u32>,
    /// Población que se perdió: pesants, artesanos y nobleza (personas).
    pub poblacion_perdida: (i64, i64, i64),
}

/// `aplicarConquista`: el saqueo determinista de la ciudad.
/// - **Población**: pesants, artesanos y nobleza pierden `fraccionSaqueoPoblacion`; lo que queda se redondea hacia abajo a personas y los
///   restos ocultos de crecimiento y de muertes se borran.
/// - **Edificios**: de los `activo`, por id, exentos el Centro Urbano y la **primera Granja y la primera Leñera** activas, los primeros
///   `ceil(fraccionEdificiosDanados × n)` pasan a `en_cola` marcados `danado` (se reconstruyen por `fraccionCosteReconstruccion`). Un Almacén o
///   Granero dañado deja de aportar capacidad y lo guardado que ya no cabe se pierde.
/// - La **obra de ascenso** en curso se pierde sin devolución y las **decisiones del gobierno derrotado** caen: políticas, reserva manual,
///   pausa de la auto-construcción y recetas paradas.
/// - `medidorMantenimiento` vuelve a su valor inicial (100) y la plaza queda `ocupada`.
pub fn aplicar_conquista(r: &Reglas, economia: &asentamiento::Reglas, a: &mut Asentamiento, ahora: Instante) -> Conquistada {
    let queda = UNO - r.ocupacion.saqueo_poblacion;
    let tras = |n: i64| (n as i128 * queda as i128 / UNO as i128) as i64;
    let antes = (a.poblacion.pesants, a.poblacion.artesanos, a.poblacion.nobleza);
    (a.poblacion.pesants, a.poblacion.artesanos, a.poblacion.nobleza) = (tras(antes.0), tras(antes.1), tras(antes.2));
    a.poblacion.restos = [0; 3];

    let mut activos: Vec<usize> = (0..a.edificios.len()).filter(|&i| a.edificios[i].estado == Estado::Activo).collect();
    activos.sort_by_key(|&i| a.edificios[i].id);
    let primero = |tipo| activos.iter().copied().find(|&i| a.edificios[i].tipo == tipo);
    let exentos = [primero(Tipo::Granja), primero(Tipo::Lenera)];
    let danables: Vec<usize> = activos.iter().copied().filter(|&i| a.edificios[i].tipo != Tipo::CentroUrbano && !exentos.contains(&Some(i))).collect();
    let cuantos = (danables.len() as i128 * r.ocupacion.edificios_danados as i128 + UNO as i128 - 1) / UNO as i128;
    let mut edificios_danados = vec![];
    for &i in danables.iter().take(cuantos as usize) {
        let (id, tipo, nivel) = (a.edificios[i].id, a.edificios[i].tipo, a.edificios[i].nivel_interno);
        let e = &mut a.edificios[i];
        e.estado = Estado::EnCola;
        e.danado = true;
        e.completa_en = None;
        e.mejora = None;
        a.aplicar_capacidad(economia, tipo, nivel, -1);
        edificios_danados.push(id);
    }

    a.ascenso = None;
    a.politicas = Default::default();
    a.reserva_manual = [0; 29];
    a.auto_pausada = false;
    a.recetas_pausadas.clear();
    a.medidor.valor = economia.mantenimiento.medidor_inicial;
    a.ocupado = true;
    Conquistada {
        protegida_hasta: ahora + r.ocupacion.proteccion,
        ocupacion_hasta: ahora + r.ocupacion.duracion,
        edificios_danados,
        poblacion_perdida: (antes.0 - a.poblacion.pesants, antes.1 - a.poblacion.artesanos, antes.2 - a.poblacion.nobleza),
    }
}

/// Un héroe al que afecta la caída: un residente (esté o no dentro) o un visitante que estaba dentro.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Afectado {
    pub heroe: HeroeId,
    pub residente: bool,
    pub dentro: bool,
    /// La columna con la que entró (tropa, carro, víveres y caravanas), si es un visitante: sale con ella al mapa, sin perder nada.
    pub columna: Option<ColumnaId>,
    /// Las escuadras con las que defendió y sobrevivieron (con soldados). Un residente herido no defendió: vacío.
    pub lucharon: Vec<EscuadraId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Salida {
    /// Vuelve al mapa con la columna con la que entró. Un residente que conservaba su columna para llevar sus caravanas (Entrar con todo, 2026-10-22) suma a ella las escuadras con
    /// las que defendió y sobrevivieron, que estaban en su campamento; en un visitante van vacías (las suyas ya van en la columna).
    ALaColumna { columna: ColumnaId, escuadras: Vec<EscuadraId> },
    /// Nace una **columna personal** estacionada en la plaza, con el carro vacío y esas escuadras. Su Facción es la de la plaza si era residente,
    /// ninguna si era visitante.
    ColumnaNueva { con_faccion_de_la_plaza: bool, escuadras: Vec<EscuadraId> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Desalojo {
    pub heroe: HeroeId,
    /// Solo quien estaba dentro: nadie se queda dentro de una plaza enemiga.
    pub salida: Option<Salida>,
    /// Solo los residentes: a dónde pasan a residir.
    pub residencia: Option<Residencia>,
}

/// El refugio de los residentes: el asentamiento **más cercano de su Facción** (empate por id), sin contar el conquistado. `candidatas` son
/// `(id, distancia a la plaza perdida)` de esos asentamientos.
pub fn refugio(candidatas: &[(AsentamientoId, Mil)]) -> Option<AsentamientoId> {
    candidatas.iter().min_by_key(|&&(id, d)| (d, id)).map(|&(id, _)| id)
}

/// Dónde pasan a residir: en el refugio, y si la Facción no tiene otro asentamiento, en el **campamento de mercenarios más cercano** a la plaza
/// perdida (nadie queda sin casa, Doc 2.5).
pub fn residencia_tras_desalojo(refugio: Option<AsentamientoId>, campamento_mas_cercano: CampamentoId) -> Residencia {
    refugio.map_or(Residencia::Campamento(campamento_mas_cercano), Residencia::Asentamiento)
}

/// `desalojarResidentes`: qué le pasa a cada afectado. Quien estaba dentro sale (con la columna con la que entró, o a una columna nueva con lo que le
/// sobrevivió); todos los residentes pasan a residir en `residencia_nueva`.
pub fn planear_desalojo(afectados: &[Afectado], residencia_nueva: Residencia) -> Vec<Desalojo> {
    afectados
        .iter()
        .map(|a| Desalojo {
            heroe: a.heroe,
            salida: a.dentro.then(|| match a.columna {
                Some(columna) => Salida::ALaColumna { columna, escuadras: if a.residente { a.lucharon.clone() } else { vec![] } },
                None => Salida::ColumnaNueva { con_faccion_de_la_plaza: a.residente, escuadras: a.lucharon.clone() },
            }),
            residencia: a.residente.then_some(residencia_nueva),
        })
        .collect()
}

/// Aplica el desalojo a un héroe residente: pasa a residir donde dice el plan y **el resto de su campamento** (guarnición incluida: tropa
/// prestada que ya no puede seguir donde está, Doc 5.15.2b) queda a 0 y se va con él, suyo, con su nivel y su experiencia. Lo que llevaba fuera
/// (en su columna o de escolta) no se toca, ni las escuadras con las que sale a la calle (`Salida::ColumnaNueva` o `Salida::ALaColumna`): esas las mueve el núcleo.
pub fn aplicar_desalojo(heroe: &mut Heroe, d: &Desalojo) {
    let Some(residencia) = d.residencia else { return };
    let salen: &[EscuadraId] = match &d.salida {
        Some(Salida::ColumnaNueva { escuadras, .. } | Salida::ALaColumna { escuadras, .. }) => escuadras,
        None => &[],
    };
    heroe.residencia = Some(residencia);
    for e in heroe.escuadras.iter_mut().filter(|e| e.contenedor == Contenedor::Campamento && !salen.contains(&e.id)) {
        e.cantidad = 0;
        e.en_guarnicion = false;
    }
}

/// `registrarDerrota`: la Facción que no conserva ningún asentamiento tras la caída queda derrotada.
pub fn faccion_derrotada(asentamientos_que_le_quedan: usize) -> bool {
    asentamientos_que_le_quedan == 0
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::resolucion::apoyo::*;
    use asentamiento::edificio::{Edificio, Mejora};
    use base::MS_POR_MINUTO;
    use militar::heroe::Controlador;
    use militar::Tropa;

    fn economia() -> &'static asentamiento::Reglas {
        static E: std::sync::OnceLock<asentamiento::Reglas> = std::sync::OnceLock::new();
        E.get_or_init(asentamiento::Reglas::cargar)
    }

    fn plaza(tipos: &[Tipo]) -> Asentamiento {
        let mut a = Asentamiento::fundar(economia(), 0, &[], 3);
        a.edificios = tipos.iter().enumerate().map(|(i, &t)| Edificio::nuevo(i as u32, t, Estado::Activo)).collect();
        a
    }

    #[test]
    fn la_poblacion_pierde_el_25_por_ciento_y_se_redondea_a_personas_hacia_abajo() {
        let r = r();
        let mut a = plaza(&[Tipo::CentroUrbano]);
        (a.poblacion.pesants, a.poblacion.artesanos, a.poblacion.nobleza) = (10, 100, 3);
        a.poblacion.restos = [500, 0, -300];
        let c = aplicar_conquista(r, economia(), &mut a, 0);
        assert_eq!((a.poblacion.pesants, a.poblacion.artesanos, a.poblacion.nobleza, a.poblacion.restos), (7, 75, 2, [0; 3]), "7,5 → 7; 75; 2,25 → 2, y sin restos");
        assert_eq!(c.poblacion_perdida, (3, 25, 1));
    }

    #[test]
    fn se_danan_el_25_por_ciento_de_los_edificios_activos_salvo_el_centro_y_la_primera_granja_y_lenera() {
        let r = r();
        use Tipo::*;
        // ids:        0            1         2        3       4       5        6        7        8       9         10        11
        let mut a = plaza(&[CentroUrbano, Vivienda, Granja, Granja, Lenera, Lenera, Vivienda, Cantera, Vivienda, Vivienda, Vivienda, Vivienda]);
        a.edificios[8].estado = Estado::EnConstruccion; // un edificio sin terminar no se daña
        let c = aplicar_conquista(r, economia(), &mut a, 0);
        // Activos: 11; exentos: 0 (Centro), 2 (1.ª Granja), 4 (1.ª Leñera) ⇒ 8 dañables; ceil(8 × 0,25) = 2: los de id menor.
        assert_eq!(c.edificios_danados, vec![1, 3]);
        for e in &a.edificios {
            let danado = c.edificios_danados.contains(&e.id);
            assert_eq!((e.danado, e.estado == Estado::EnCola), (danado, danado), "edificio {}", e.id);
        }
        assert_eq!(a.edificios[8].estado, Estado::EnConstruccion);
    }

    #[test]
    fn el_redondeo_de_edificios_es_hacia_arriba() {
        let r = r();
        use Tipo::*;
        let mut a = plaza(&[CentroUrbano, Vivienda, Vivienda, Vivienda, Vivienda, Vivienda]);
        assert_eq!(aplicar_conquista(r, economia(), &mut a, 0).edificios_danados.len(), 2, "ceil(5 × 0,25) = 2");
        let mut b = plaza(&[CentroUrbano, Vivienda]);
        assert_eq!(aplicar_conquista(r, economia(), &mut b, 0).edificios_danados, vec![1], "ceil(0,25) = 1");
        let mut c = plaza(&[CentroUrbano, Granja]);
        assert!(aplicar_conquista(r, economia(), &mut c, 0).edificios_danados.is_empty(), "sin dañables no se daña nada");
    }

    #[test]
    fn un_almacen_danado_deja_de_dar_capacidad_y_lo_que_no_cabe_se_pierde() {
        let r = r();
        let mut a = plaza(&[Tipo::CentroUrbano, Tipo::Almacen]);
        a.almacen.ampliar_todos(economia().catalogo.tipo(Tipo::Almacen).capacidad_por_recurso);
        let cap = a.almacen.capacidad(asentamiento::Recurso::Madera);
        a.almacen.poner(asentamiento::Recurso::Madera, cap);
        aplicar_conquista(r, economia(), &mut a, 0);
        let cap_tras = a.almacen.capacidad(asentamiento::Recurso::Madera);
        assert!(cap_tras < cap);
        assert_eq!(a.almacen.cantidad(asentamiento::Recurso::Madera), cap_tras, "lo guardado que ya no cabe se pierde");
    }

    #[test]
    fn un_edificio_danado_pierde_su_mejora_en_curso() {
        let r = r();
        let mut a = plaza(&[Tipo::CentroUrbano, Tipo::Vivienda]);
        a.edificios[1].mejora = Some(Mejora { nivel_objetivo: 2, completa_en: 99 });
        aplicar_conquista(r, economia(), &mut a, 0);
        assert!(a.edificios[1].mejora.is_none() && a.edificios[1].completa_en.is_none());
    }

    #[test]
    fn caen_el_ascenso_y_las_decisiones_del_gobierno_derrotado_y_empieza_la_ocupacion() {
        let r = r();
        let mut a = plaza(&[Tipo::CentroUrbano]);
        a.ascenso = Some(asentamiento::asentamiento::Ascenso { nivel_objetivo: 3, iniciado_en: 0, completa_en: 10 });
        a.reserva_manual = [5; 29];
        a.auto_pausada = true;
        a.recetas_pausadas = vec![asentamiento::Recurso::LingoteCobre];
        a.medidor.valor = 12_000;
        let c = aplicar_conquista(r, economia(), &mut a, 3 * MS_POR_MINUTO);
        assert!(a.ascenso.is_none() && !a.auto_pausada && a.recetas_pausadas.is_empty());
        assert_eq!(a.reserva_manual, [0; 29]);
        assert_eq!(a.politicas, Default::default());
        assert_eq!(a.medidor.valor, economia().mantenimiento.medidor_inicial, "medidor a 100");
        assert!(a.ocupado);
        assert_eq!(c.protegida_hasta, 3 * MS_POR_MINUTO + 1440 * MS_POR_MINUTO);
        assert_eq!(c.ocupacion_hasta, 3 * MS_POR_MINUTO + 90 * MS_POR_MINUTO);
    }

    #[test]
    fn la_proteccion_dura_hasta_su_instante() {
        assert!(esta_protegida(Some(100), 99));
        assert!(!esta_protegida(Some(100), 100));
        assert!(!esta_protegida(None, 0));
    }

    #[test]
    fn es_determinista_y_no_aplica_dos_veces_el_mismo_dano_a_lo_ya_danado() {
        let r = r();
        let mut a = plaza(&[Tipo::CentroUrbano, Tipo::Vivienda, Tipo::Vivienda, Tipo::Vivienda, Tipo::Vivienda]);
        let mut b = a.clone();
        assert_eq!(aplicar_conquista(r, economia(), &mut a, 5), aplicar_conquista(r, economia(), &mut b, 5));
        assert_eq!(a, b);
        let segunda = aplicar_conquista(r, economia(), &mut a, 6);
        assert!(segunda.edificios_danados.iter().all(|id| !b.edificios[*id as usize].danado), "los ya dañados están en cola, no activos");
    }

    #[test]
    fn el_refugio_es_el_mas_cercano_con_desempate_por_id() {
        assert_eq!(refugio(&[(7, 100), (3, 50), (2, 50)]), Some(2));
        assert_eq!(refugio(&[]), None);
        assert_eq!(residencia_tras_desalojo(Some(3), 9), Residencia::Asentamiento(3));
        assert_eq!(residencia_tras_desalojo(None, 9), Residencia::Campamento(9), "sin plaza propia, al campamento de mercenarios más cercano");
    }

    #[test]
    fn nadie_se_queda_dentro_y_los_residentes_se_mudan() {
        let nueva = Residencia::Asentamiento(4);
        let afectados = [
            Afectado { heroe: 1, residente: true, dentro: true, columna: None, lucharon: vec![10, 11] },
            Afectado { heroe: 2, residente: true, dentro: true, columna: None, lucharon: vec![] },
            Afectado { heroe: 3, residente: true, dentro: false, columna: None, lucharon: vec![] },
            Afectado { heroe: 4, residente: false, dentro: true, columna: Some(77), lucharon: vec![] },
            Afectado { heroe: 5, residente: false, dentro: true, columna: None, lucharon: vec![] },
        ];
        let plan = planear_desalojo(&afectados, nueva);
        assert_eq!(plan[0], Desalojo { heroe: 1, salida: Some(Salida::ColumnaNueva { con_faccion_de_la_plaza: true, escuadras: vec![10, 11] }), residencia: Some(nueva) });
        assert_eq!(plan[1].salida, Some(Salida::ColumnaNueva { con_faccion_de_la_plaza: true, escuadras: vec![] }), "un herido sale solo");
        assert_eq!(plan[2], Desalojo { heroe: 3, salida: None, residencia: Some(nueva) }, "un residente fuera solo se muda");
        assert_eq!(plan[3], Desalojo { heroe: 4, salida: Some(Salida::ALaColumna { columna: 77, escuadras: vec![] }), residencia: None }, "el visitante vuelve a su columna y no cambia de casa");
        assert_eq!(plan[4].salida, Some(Salida::ColumnaNueva { con_faccion_de_la_plaza: false, escuadras: vec![] }), "un visitante sin columna sale sin bandera");
        assert!(afectados.iter().zip(&plan).all(|(a, p)| (p.salida.is_some()) == a.dentro), "sale quien estaba dentro, y solo ellos");
    }

    #[test]
    fn el_resto_del_campamento_queda_a_cero_con_su_nivel_y_experiencia_menos_lo_que_sale_y_lo_que_va_fuera() {
        let r = r();
        let mut h = Heroe::nuevo(&r.militar, 1, Controlador::Humano, 3, 1);
        h.residencia = Some(Residencia::Asentamiento(9));
        let mut en_guarnicion = escuadra(11, 1, Tropa::Arqueros);
        en_guarnicion.en_guarnicion = true;
        en_guarnicion.nivel = 4;
        en_guarnicion.experiencia = 12_000;
        let mut fuera = escuadra(12, 1, Tropa::Hequetai);
        fuera.contenedor = Contenedor::Ejercito(55);
        let mut escolta = escuadra(13, 1, Tropa::Shardana);
        escolta.contenedor = Contenedor::Escolta(8);
        h.escuadras = vec![escuadra(10, 1, Tropa::MiliciaLanceros), en_guarnicion, fuera, escolta, escuadra(14, 1, Tropa::Peltastas)];
        let d = Desalojo { heroe: 1, salida: Some(Salida::ColumnaNueva { con_faccion_de_la_plaza: true, escuadras: vec![14] }), residencia: Some(Residencia::Campamento(2)) };
        aplicar_desalojo(&mut h, &d);
        assert_eq!(h.residencia, Some(Residencia::Campamento(2)));
        let cantidades: Vec<i64> = h.escuadras.iter().map(|e| e.cantidad).collect();
        let completa = |t: Tropa| r.militar.tropas.tropa(t).unidades;
        assert_eq!(cantidades, vec![0, 0, completa(Tropa::Hequetai), completa(Tropa::Shardana), completa(Tropa::Peltastas)], "campamento y guarnición a 0; la columna, la escolta y la que sale, intactas");
        assert!(!h.escuadras[1].en_guarnicion);
        assert_eq!((h.escuadras[1].nivel, h.escuadras[1].experiencia), (4, 12_000), "siguen siendo suyas, con su nivel y experiencia");
    }

    #[test]
    fn un_visitante_no_cambia_de_casa_ni_pierde_tropa() {
        let r = r();
        let mut h = Heroe::nuevo(&r.militar, 4, Controlador::Bot, 3, 1);
        h.escuadras = vec![escuadra(1, 4, Tropa::Arqueros)];
        let antes = h.clone();
        aplicar_desalojo(&mut h, &Desalojo { heroe: 4, salida: Some(Salida::ALaColumna { columna: 7, escuadras: vec![] }), residencia: None });
        assert_eq!(h, antes);
    }

    #[test]
    fn un_residente_con_su_columna_para_las_caravanas_sale_con_ella_y_con_las_escuadras_que_defendieron() {
        let r = r();
        let nueva = Residencia::Asentamiento(4);
        let mut h = Heroe::nuevo(&r.militar, 1, Controlador::Humano, 3, 1);
        h.residencia = Some(Residencia::Asentamiento(9));
        h.escuadras = vec![escuadra(10, 1, Tropa::MiliciaLanceros), escuadra(11, 1, Tropa::Arqueros)];
        let residente = Afectado { heroe: 1, residente: true, dentro: true, columna: Some(77), lucharon: vec![10] };
        let plan = planear_desalojo(&[residente], nueva);
        assert_eq!(plan[0], Desalojo { heroe: 1, salida: Some(Salida::ALaColumna { columna: 77, escuadras: vec![10] }), residencia: Some(nueva) });
        aplicar_desalojo(&mut h, &plan[0]);
        let cantidades: Vec<i64> = h.escuadras.iter().map(|e| e.cantidad).collect();
        assert_eq!(cantidades, vec![r.militar.tropas.tropa(Tropa::MiliciaLanceros).unidades, 0], "la que defendió sale con él; el resto del campamento, a 0");
    }

    #[test]
    fn el_comercio_de_la_plaza_conquistada_se_cierra_con_la_cascada_de_comercio() {
        use base::Punto;
        use comercio::caravana::{Caravana, EstadoCaravana};
        use comercio::ruina::{cerrar_dependientes, Accion, Motivo};
        // La aparcada de otra Facción en la plaza 9 se destruye (decidido el 2026-10-19); la flota de la plaza pasa al conquistador.
        let mut aparcada = Caravana::nueva(1, 5, Punto::default());
        aparcada.estado = EstadoCaravana::Aparcada;
        aparcada.anfitriona = Some(9);
        let flota = Caravana::nueva(2, 9, Punto::default());
        let cierre = cerrar_dependientes(Motivo::Conquista, 9, &mut [], &[aparcada, flota], &|_| true, &|_| true);
        assert_eq!(cierre.caravanas.iter().map(|c| (c.caravana, c.accion)).collect::<Vec<_>>(), vec![(1, Accion::Destruye)]);
        assert_eq!(cierre.flota_traspasada, vec![2]);
    }

    #[test]
    fn una_faccion_sin_asentamientos_queda_derrotada() {
        assert!(faccion_derrotada(0));
        assert!(!faccion_derrotada(1));
    }
}
