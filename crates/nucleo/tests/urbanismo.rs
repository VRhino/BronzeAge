//! La ciudad de cada plaza (Docs/Mechanics/Trazado_Urbano.md) dentro del núcleo: los edificios de la economía se colocan de verdad en
//! su ciudad, con anclas gratuitas y su gate de nivel, puestos del Mercado, reubicación de la Granja al mejorar, y todo ello guardado en
//! las fotos. Las pruebas de la economía con una ciudad usan [`SitiosUrbe`] (sin mundo); las de la plaza viva usan el artefacto de mundo
//! real (`generador/salida/mundo/mundo-*.bin`) y se omiten si no está.

use asentamiento::asentamiento::{Asentamiento, Evento, Exterior, Urbano};
use asentamiento::catalogo::{Tipo, TIPOS};
use asentamiento::datos::UNO;
use asentamiento::edificio::{Edificios, Estado as EstadoEdificio};
use asentamiento::obras::ErrorObra;
use asentamiento::produccion::Fuentes;
use asentamiento::recurso::TODOS;
use base::{Instante, Mil, Punto, MS_POR_MINUTO};
use mundo::{Mundo, OpcionesValidacion};
use nucleo::urbanismo::{SitiosUrbe, Urbe};
use nucleo::{Ajustes, Arranque, Comando, Estado, Nucleo, Peticion, Reglas, ReglasEconomia};
use rutas::Rutas;
use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, OnceLock};
use trazado::{Categoria, Celda, Ciudad, Recinto, Rol, CELDA_UL};

const MIN: i64 = MS_POR_MINUTO;
const HORA: i64 = 60 * MIN;

// ───────────────────────────── La economía sobre una ciudad (sin mundo) ─────────────────────────────

struct SinMundo;
impl Fuentes for SinMundo {
    fn densidad_parcela(&self, _: u32) -> i64 {
        0
    }
    fn extraer(&mut self, _: u32, _: Mil) -> Mil {
        0
    }
}

/// Una plaza rica y quieta (sin auto-construcción) con su ciudad recién fundada: solo se construye lo que se pide.
fn plaza_rica(re: &ReglasEconomia) -> (Asentamiento, Urbe) {
    let r = &re.datos;
    let mut a = Asentamiento::fundar(r, 0, &[], r.viviendas_iniciales);
    for &rec in TODOS.iter() {
        a.almacen.ampliar(rec, 10_000_000);
        a.almacen.poner(rec, 5_000_000);
    }
    a.auto_pausada = true;
    (a, Urbe::fundar(re, 7, 1))
}

/// Con el `Exterior` de una plaza que solo tiene ciudad (sin yacimientos), ejecuta `f`.
fn con_ciudad<R>(re: &ReglasEconomia, urbe: &mut Urbe, f: impl FnOnce(&mut Exterior) -> R) -> R {
    let mut s = SitiosUrbe { urbe, urbanismo: &re.urbanismo };
    let mut mundo = SinMundo;
    let mut x = Exterior {
        reglas: &re.datos,
        fuentes: &mut mundo,
        sitios: &mut s,
        fertilidad: UNO,
        tiene_tecnologia: &|_| false,
        factor_tecnologia: &|_| UNO,
        nivel_faccion: 1,
        orden_era: 1,
        es_capital: true,
        faccion_tiene_gran_fundicion: false,
        ciudadanos: 1,
        distancia_capital: 0,
        recinto_nivel: 0,
        racion_guarnicion: 0,
        felicidad_extra: UNO,
        nobleza_extra: UNO,
        recaudacion_extra: UNO,
    };
    f(&mut x)
}

/// Avanza los minutos `desde+1 ..= hasta` y devuelve los sucesos.
fn avanzar(re: &ReglasEconomia, a: &mut Asentamiento, urbe: &mut Urbe, desde: i64, hasta: i64) -> Vec<Evento> {
    let mut ev = vec![];
    for minuto in desde + 1..=hasta {
        ev.extend(con_ciudad(re, urbe, |x| a.avanzar_hasta(minuto * MIN, x)));
    }
    ev
}

fn urbano(nivel: u32) -> Urbano<'static> {
    Urbano { nivel, nivel_operativo: nivel, completados: 0, perfil: None, siguiente_id: 100 }
}

/// Cada pieza de la ciudad tiene su edificio en la economía (mismo id y tipo) y cada edificio con sitio en la ciudad tiene su pieza.
fn ciudad_y_economia_cuadran(re: &ReglasEconomia, a: &Asentamiento, c: &Ciudad) {
    for p in c.piezas() {
        let e = a.edificio(p.id).unwrap_or_else(|| panic!("la pieza {} ({:?}) no tiene edificio", p.id, TIPOS[p.tipo as usize]));
        assert_eq!(e.tipo, TIPOS[p.tipo as usize], "el edificio {} y su pieza son de tipos distintos", p.id);
    }
    for e in &a.edificios {
        let en_rejilla = re.urbanismo.clase(e.tipo).is_some();
        assert_eq!(c.pieza(e.id).is_some(), en_rejilla, "el edificio {} ({:?}) {} pieza", e.id, e.tipo, if en_rejilla { "no tiene" } else { "tiene" });
    }
}

#[test]
fn la_fundacion_pone_en_la_ciudad_los_mismos_edificios_iniciales_que_en_la_economia() {
    let re = ReglasEconomia::default();
    let (a, urbe) = plaza_rica(&re);
    let c = urbe.ciudad();
    let pares: Vec<(u32, Tipo)> = c.piezas().iter().map(|p| (p.id, TIPOS[p.tipo as usize])).collect();
    assert_eq!(pares, vec![(0, Tipo::CentroUrbano), (1, Tipo::Vivienda), (2, Tipo::Vivienda), (3, Tipo::Vivienda), (4, Tipo::Granja)]);
    assert_eq!(a.edificios.iter().map(|e| (e.id, e.tipo)).collect::<Vec<_>>(), pares, "mismos ids y tipos que Asentamiento::fundar");
    ciudad_y_economia_cuadran(&re, &a, c);
    // Dos plazas, dos ciudades; la misma plaza en el mismo mundo, la misma ciudad.
    assert_ne!(Urbe::fundar(&re, 7, 1).ciudad().semilla(), Urbe::fundar(&re, 7, 2).ciudad().semilla());
    assert_ne!(Urbe::fundar(&re, 7, 1).ciudad().semilla(), Urbe::fundar(&re, 8, 1).ciudad().semilla());
    assert_eq!(Urbe::fundar(&re, 7, 1), Urbe::fundar(&re, 7, 1));
}

#[test]
fn el_gate_de_anclas_sin_ancla_de_industria_no_hay_sitio_en_nivel_1_y_en_el_nivel_2_se_crea() {
    let re = ReglasEconomia::default();
    let mut urbe = Urbe::fundar(&re, 7, 1);
    for t in [Tipo::Fundicion, Tipo::Armeria, Tipo::Curtiduria, Tipo::Barracon, Tipo::Carpinteria] {
        assert!(!urbe.cabe(&re.urbanismo, t, &urbano(1)), "{t:?}: sin ancla de su barrio no hay sitio en el nivel 1");
    }
    // El Almacén y la Vivienda no tienen ancla que esperar (genérico y residencial: el Centro Urbano es su ancla).
    assert!(urbe.cabe(&re.urbanismo, Tipo::Almacen, &urbano(1)) && urbe.cabe(&re.urbanismo, Tipo::Vivienda, &urbano(1)));
    // En el nivel 2 cabe porque el trazado crearía antes el ancla (y lo dice sin tocar la ciudad).
    let antes = urbe.ciudad().piezas().len();
    for t in [Tipo::Fundicion, Tipo::Armeria] {
        assert!(urbe.cabe(&re.urbanismo, t, &urbano(2)), "{t:?}: con el gate abierto cabe");
    }
    assert_eq!(urbe.ciudad().piezas().len(), antes, "preguntar no coloca nada");
    // Y al comprometerlo, el ancla nace como edificio gratuito con el id siguiente.
    let mut ids = 51;
    let gratuitos = urbe.tomar(&re.urbanismo, 50, Tipo::Fundicion, &urbano(2), &mut ids).expect("hay sitio");
    assert_eq!(gratuitos.len(), 1);
    assert_eq!((gratuitos[0].id, gratuitos[0].tipo, ids), (51, Tipo::PatioDeGremios, 52));
    let c = urbe.ciudad();
    let ancla = c.pieza(51).unwrap();
    assert_eq!((ancla.rol, ancla.categoria), (Rol::AnclaSaturacion, Categoria::Industria));
    assert_eq!(c.pieza(50).unwrap().categoria, Categoria::Industria);
    // Con el ancla puesta, la Armería ya cabe también en el nivel 1 (el gate solo vale para crear anclas).
    assert!(urbe.cabe(&re.urbanismo, Tipo::Armeria, &urbano(1)));
}

#[test]
fn una_obra_de_industria_en_nivel_2_crea_su_ancla_como_edificio_gratuito_y_activo() {
    let re = ReglasEconomia::default();
    let (mut a, mut urbe) = plaza_rica(&re);
    // En el nivel 1 la economía ni lo intenta (nivel insuficiente); en el 2, sí.
    a.nivel = 2;
    a.medidor.nivel_actual = 2;
    let mut ev = vec![];
    let id = con_ciudad(&re, &mut urbe, |x| a.anadir_manual(0, &re.datos, x, Tipo::Fundicion, &mut ev)).expect("cabe");
    assert_eq!(id, 5, "el id del proyecto sale antes que el del ancla");
    let patio = a.edificios.iter().find(|e| e.tipo == Tipo::PatioDeGremios).expect("el trazado creó el ancla");
    assert_eq!((patio.id, patio.estado, patio.nivel_interno), (6, EstadoEdificio::Activo, 1));
    assert_eq!(a.siguiente_edificio, 7);
    assert!(ev.contains(&Evento::EdificioGratuito { id: 6, tipo: Tipo::PatioDeGremios, nivel: 1 }), "{ev:?}");
    assert_eq!(a.edificio(id).unwrap().estado, EstadoEdificio::EnConstruccion, "hay cuadrilla libre: arranca en el instante de la orden");
    ciudad_y_economia_cuadran(&re, &a, urbe.ciudad());
    // Las anclas son piezas gratuitas: cuentan a la mitad en el mantenimiento y no agrandan la ciudad.
    assert!(re.datos.mantenimiento.piezas_gratuitas.contains(&Tipo::PatioDeGremios));
    assert_eq!(a.urbano(&re.datos).completados, 0, "ni los edificios de la fundación ni los gratuitos agrandan la ciudad");
    assert_eq!(a.tomar_completados(), 0, "ni la zona del mapa");
}

#[test]
fn quitar_un_proyecto_de_la_cola_libera_su_sitio_en_la_ciudad() {
    let re = ReglasEconomia::default();
    let (mut a, mut urbe) = plaza_rica(&re);
    // Las dos cuadrillas ocupadas: lo siguiente espera en la cola (una obra en curso no se quita).
    for _ in 0..2 {
        con_ciudad(&re, &mut urbe, |x| a.anadir_manual(0, &re.datos, x, Tipo::Vivienda, &mut vec![])).unwrap();
    }
    let id = con_ciudad(&re, &mut urbe, |x| a.anadir_manual(0, &re.datos, x, Tipo::Vivienda, &mut vec![])).unwrap();
    assert_eq!(a.edificio(id).unwrap().estado, EstadoEdificio::EnCola);
    let con = urbe.ciudad().clone();
    assert!(con.pieza(id).is_some());
    // La baja es del núcleo (economía y ciudad a la vez): aquí, lo que hace su orden.
    a.quitar_de_cola(&re.datos, id).unwrap();
    urbe.quitar(id);
    assert!(urbe.ciudad().pieza(id).is_none());
    assert_eq!(urbe.ciudad().piezas().len(), con.piezas().len() - 1);
    ciudad_y_economia_cuadran(&re, &a, urbe.ciudad());
}

#[test]
fn los_puestos_del_mercado_nacen_gratis_y_activos_al_completarse_y_al_mejorar() {
    let re = ReglasEconomia::default();
    let (mut a, mut urbe) = plaza_rica(&re);
    let mut ev = vec![];
    let mercado = con_ciudad(&re, &mut urbe, |x| a.anadir_manual(0, &re.datos, x, Tipo::Mercado, &mut ev)).expect("el Mercado cabe");
    let m = urbe.ciudad().pieza(mercado).unwrap();
    assert_eq!((m.rol, m.categoria), (Rol::AnclaPrimaria, Categoria::Mercado), "el Mercado es su propia ancla");
    let puestos = |a: &Asentamiento| a.edificios.iter().filter(|e| e.tipo == Tipo::PuestoMercado).map(|e| (e.nivel_interno, e.estado)).collect::<Vec<_>>();
    assert!(puestos(&a).is_empty(), "todavía en obras: sin puestos");
    // Al completarse (480 min de obra) nacen los del nivel 1, con la forma de cada uno en `nivel_interno`.
    let ev = avanzar(&re, &mut a, &mut urbe, 0, 481);
    assert!(ev.contains(&Evento::EdificioCompletado { id: mercado, tipo: Tipo::Mercado }), "se completó");
    let nuevos: Vec<_> = ev.iter().filter(|e| matches!(e, Evento::EdificioGratuito { tipo: Tipo::PuestoMercado, .. })).collect();
    assert_eq!(nuevos.len(), 5, "cinco puestos de nivel 1 (1·1·2·2·3): {ev:?}");
    let formas: Vec<u8> = puestos(&a).into_iter().map(|(forma, estado)| {
        assert_eq!(estado, EstadoEdificio::Activo, "nacen activos, sin pasar por la cola");
        forma
    }).collect();
    assert_eq!(formas, vec![1, 1, 2, 2, 3]);
    // Y cada puesto tiene su pieza con la huella de su forma (alto 2, 2, 3, 3, 1 o girada).
    for e in a.edificios.iter().filter(|e| e.tipo == Tipo::PuestoMercado) {
        let r = urbe.ciudad().pieza(e.id).expect("el puesto está en la ciudad").rect;
        let lados = (r.ancho.min(r.alto), r.ancho.max(r.alto));
        assert_eq!(lados, match e.nivel_interno { 1 => (1, 2), 2 => (1, 3), _ => (1, 1) }, "forma {}", e.nivel_interno);
        assert_eq!(urbe.ciudad().pieza(e.id).unwrap().categoria, Categoria::Mercado);
    }
    ciudad_y_economia_cuadran(&re, &a, urbe.ciudad());
    assert_eq!(a.edificios.iter().filter(|e| e.tipo == Tipo::Mercado).count(), 1);
    assert!(a.edificios.iter().filter(|e| e.tipo == Tipo::PuestoMercado).all(|e| !e.ocupa_cuadrilla()), "los puestos no ocupan cuadrilla");

    // Mejorar el Mercado al nivel 2 (pide el nivel 2 de asentamiento): al terminar, siete puestos más (1·1·2·2·3·3·3).
    a.nivel = 2;
    a.medidor.nivel_actual = 2;
    con_ciudad(&re, &mut urbe, |x| a.mejorar_ahora(482 * MIN, &re.datos, x, mercado, &mut vec![])).expect("se puede mejorar");
    let termina = a.edificio(mercado).unwrap().mejora.unwrap().completa_en / MIN;
    let ev = avanzar(&re, &mut a, &mut urbe, 481, termina + 1);
    assert!(ev.contains(&Evento::MejoraCompletada { id: mercado, nivel: 2 }));
    assert_eq!(ev.iter().filter(|e| matches!(e, Evento::EdificioGratuito { tipo: Tipo::PuestoMercado, .. })).count(), 7, "{ev:?}");
    assert_eq!(puestos(&a).len(), 12);
    ciudad_y_economia_cuadran(&re, &a, urbe.ciudad());
}

#[test]
fn un_puesto_sin_sitio_se_salta_en_silencio() {
    let re = ReglasEconomia::default();
    let (mut a, mut urbe) = plaza_rica(&re);
    let mercado = con_ciudad(&re, &mut urbe, |x| a.anadir_manual(0, &re.datos, x, Tipo::Mercado, &mut vec![])).unwrap();
    // Todo el suelo del anillo del Mercado lo ocupa un muro: ningún puesto cabe, y la plaza sigue.
    let c = urbe.ciudad();
    let m = c.pieza(mercado).unwrap().rect;
    let mut muros = vec![];
    for col in m.col - 6..m.fin_col() + 6 {
        for row in m.row - 6..m.fin_row() + 6 {
            let cel = Celda::new(col, row);
            if !m.contiene(cel) && !c.esta_ocupada(cel) {
                muros.push(cel);
            }
        }
    }
    urbe.set_recintos(vec![Recinto { muros, puertas: vec![], completo: false }]);
    let antes = a.siguiente_edificio;
    let ev = avanzar(&re, &mut a, &mut urbe, 0, 481);
    assert!(ev.contains(&Evento::EdificioCompletado { id: mercado, tipo: Tipo::Mercado }));
    assert_eq!(a.edificios.iter().filter(|e| e.tipo == Tipo::PuestoMercado).count(), 0);
    assert_eq!(a.siguiente_edificio, antes, "un puesto sin sitio no gasta ids");
}

/// Un recinto que ocupa toda la ventana salvo la ciudad (a menos de 55 u.l. del centro) y las celdas de la pieza `id`.
fn muros_menos(c: &Ciudad, id: u32) -> Vec<Celda> {
    let propia = c.pieza(id).unwrap().rect;
    let mut muros = vec![];
    for row in -trazado::MITAD..=trazado::MITAD {
        for col in -trazado::MITAD..=trazado::MITAD {
            let cel = Celda::new(col, row);
            let (x, y) = ((col * CELDA_UL) as i64, (row * CELDA_UL) as i64);
            if !propia.contiene(cel) && x * x + y * y >= 55 * 55 {
                muros.push(cel);
            }
        }
    }
    muros
}

#[test]
fn la_mejora_de_la_granja_la_muda_a_un_hueco_de_su_huella_nueva() {
    let re = ReglasEconomia::default();
    let (mut a, mut urbe) = plaza_rica(&re);
    let granja = 4;
    let antes = urbe.ciudad().pieza(granja).unwrap().clone();
    assert_eq!((antes.rect.ancho, antes.rect.alto), (2, 2));
    let posicion = urbe.ciudad().piezas().iter().position(|p| p.id == granja).unwrap();
    con_ciudad(&re, &mut urbe, |x| a.mejorar_ahora(0, &re.datos, x, granja, &mut vec![])).expect("hay hueco");
    let termina = a.edificio(granja).unwrap().mejora.unwrap().completa_en / MIN;
    avanzar(&re, &mut a, &mut urbe, 0, termina - 1);
    assert_eq!(urbe.ciudad().pieza(granja).unwrap().rect, antes.rect, "mientras se mejora sigue donde estaba");
    let ev = avanzar(&re, &mut a, &mut urbe, termina - 1, termina);
    assert!(ev.contains(&Evento::MejoraCompletada { id: granja, nivel: 2 }), "{ev:?}");
    let despues = urbe.ciudad().pieza(granja).unwrap();
    assert_eq!((despues.rect.ancho, despues.rect.alto), (2, 3), "la huella del nivel 2");
    assert_eq!(urbe.ciudad().piezas().iter().position(|p| p.id == granja), Some(posicion), "conserva su sitio en el orden de construcción");
    assert_eq!(a.edificio(granja).unwrap().nivel_interno, 2);
    ciudad_y_economia_cuadran(&re, &a, urbe.ciudad());
}

#[test]
fn la_mejora_de_la_granja_espera_su_hueco_y_la_manual_lo_exige() {
    let re = ReglasEconomia::default();
    let (mut a, mut urbe) = plaza_rica(&re);
    let granja = 4;

    // Manual sin hueco: se rechaza antes de cobrar nada.
    urbe.set_recintos(vec![Recinto { muros: muros_menos(urbe.ciudad(), granja), puertas: vec![], completo: false }]);
    let madera = a.almacen.cantidad(asentamiento::Recurso::Madera);
    assert_eq!(con_ciudad(&re, &mut urbe, |x| a.mejorar_ahora(0, &re.datos, x, granja, &mut vec![])), Err(ErrorObra::SinSitio));
    assert_eq!((a.almacen.cantidad(asentamiento::Recurso::Madera), a.edificio(granja).unwrap().mejora), (madera, None), "ni cobra ni arranca");

    // Con hueco arranca; si el hueco desaparece antes de acabar, la mejora espera conservando su cuadrilla.
    urbe.set_recintos(vec![]);
    con_ciudad(&re, &mut urbe, |x| a.mejorar_ahora(0, &re.datos, x, granja, &mut vec![])).expect("ahora sí hay hueco");
    let completa_en = a.edificio(granja).unwrap().mejora.unwrap().completa_en;
    let termina = completa_en / MIN;
    urbe.set_recintos(vec![Recinto { muros: muros_menos(urbe.ciudad(), granja), puertas: vec![], completo: false }]);
    let rect = urbe.ciudad().pieza(granja).unwrap().rect;
    let ev = avanzar(&re, &mut a, &mut urbe, 0, termina + 30);
    assert!(!ev.iter().any(|e| matches!(e, Evento::MejoraCompletada { .. })), "sin hueco no se completa");
    let e = a.edificio(granja).unwrap();
    assert_eq!((e.nivel_interno, e.mejora.map(|m| m.completa_en)), (1, Some(completa_en)), "conserva la mejora");
    assert_eq!(a.edificios.cuadrillas_ocupadas(), 1, "y su cuadrilla");
    assert_eq!(urbe.ciudad().pieza(granja).unwrap().rect, rect, "la granja no se movió");

    // En cuanto aparece el hueco, el minuto siguiente la completa.
    urbe.set_recintos(vec![]);
    let ev = avanzar(&re, &mut a, &mut urbe, termina + 30, termina + 31);
    assert!(ev.contains(&Evento::MejoraCompletada { id: granja, nivel: 2 }), "{ev:?}");
    let g = urbe.ciudad().pieza(granja).unwrap();
    assert_eq!((g.rect.ancho, g.rect.alto), (2, 3));
    assert_eq!(a.edificio(granja).unwrap().nivel_interno, 2);
    ciudad_y_economia_cuadran(&re, &a, urbe.ciudad());
}

#[test]
fn el_perfil_de_la_ordenanza_manda_sobre_la_tradicion_y_solo_afecta_a_lo_nuevo() {
    let re = ReglasEconomia::default();
    let mut urbe = Urbe::fundar(&re, 7, 1);
    let con_perfil = |p: Option<&'static str>| Urbano { perfil: p, ..urbano(1) };
    let colocar = |urbe: &mut Urbe, p: Option<&'static str>, n: u32| {
        let mut ids = n + 1;
        for k in 0..12 {
            urbe.tomar(&re.urbanismo, 20 + n * 100 + k, Tipo::Vivienda, &con_perfil(p), &mut ids).expect("cabe");
        }
    };
    colocar(&mut urbe, Some("compacta"), 0);
    let piezas = urbe.ciudad().piezas().to_vec();
    colocar(&mut urbe, Some("gremial"), 1);
    assert_eq!(&urbe.ciudad().piezas()[..piezas.len()], &piezas[..], "cambiar de perfil no recoloca lo ya colocado");
    // Un nombre desconocido cae a la tradición de la plaza.
    let mut a = Urbe::fundar(&re, 7, 1);
    let mut b = Urbe::fundar(&re, 7, 1);
    let tradicion = a.ciudad().perfil_por_tradicion();
    let nombre = match tradicion {
        trazado::Perfil::Nucleos => "nucleos",
        trazado::Perfil::Caminera => "caminera",
        trazado::Perfil::Compacta => "compacta",
        trazado::Perfil::Gremial => "gremial",
    };
    colocar(&mut a, None, 0);
    colocar(&mut b, Some(nombre), 0);
    colocar(&mut b, Some("no_existe"), 1);
    colocar(&mut a, None, 1);
    assert_eq!(a, b, "sin ordenanza (o con una desconocida) manda el perfil de la tradición");
}

// ───────────────────────────── Serialización ─────────────────────────────

#[test]
fn la_ciudad_se_guarda_en_la_foto_con_su_version_de_geometria_y_se_rechaza_otra() {
    let reglas = Reglas::default();
    let mut s = Estado::nuevo(reglas.clone(), "h".into(), 1, 0);
    let eco = &s.reglas.economia.datos;
    let mut economia = Asentamiento::fundar(eco, 0, &eco.materiales_iniciales, eco.viviendas_iniciales);
    economia.registrar_acarreo(5_000_000, asentamiento::Recurso::Piedra, 7_000);
    let mut urbe = Urbe::fundar(&s.reglas.economia, 7, 1);
    let mut ids = 5;
    for k in 0..10 {
        urbe.tomar(&s.reglas.economia.urbanismo, 5 + k, Tipo::Vivienda, &urbano(1), &mut ids).unwrap();
    }
    urbe.set_recintos(vec![Recinto { muros: vec![Celda::new(40, 40), Celda::new(41, 40)], puertas: vec![Celda::new(42, 40)], completo: true }]);
    s.asentamientos.insert(1, nucleo::Asentamiento { id: 1, faccion: 1, posicion: Punto::new(1000, 2000), fundado_en: 0, radio: 6_000, parcelas: Default::default(), version: 0, economia, urbe, protegida_hasta: None, ocupada_hasta: None, acceso: Default::default() });
    let bytes = s.serializar();
    assert_eq!(&bytes[..5], b"BAN18", "el formato de foto es BAN18 (BAN17 no traía los héroes dentro de pueblos ni avanzadas; BAN16 no traía el acceso de las plazas; BAN14 no traía las avanzadas; BAN13 tenía la población continua y no traía las citas de fin de obra)");
    let vuelta = Estado::deserializar(&bytes, &reglas, "h").unwrap();
    assert_eq!(vuelta.serializar(), bytes, "ida y vuelta idéntica");
    assert_eq!(vuelta.asentamientos, s.asentamientos);
    assert_eq!(vuelta.asentamientos[&1].economia.ingresos_acarreo.len(), 1, "la ventana del acarreo vuelve de la foto");
    let mut otro_ingreso = Estado::deserializar(&bytes, &reglas, "h").unwrap();
    otro_ingreso.asentamientos.get_mut(&1).unwrap().economia.registrar_acarreo(5_000_000, asentamiento::Recurso::Piedra, 1);
    assert_ne!(otro_ingreso.hash(), s.hash(), "lo ingresado por acarreo entra en el hash");
    let (a, b) = (s.asentamientos[&1].urbe.ciudad(), vuelta.asentamientos[&1].urbe.ciudad());
    assert_eq!((a.calles(), a.caminos(), a.recintos()), (b.calles(), b.caminos(), b.recintos()), "el suelo y la red se derivan igual");
    assert_eq!(a.piezas(), b.piezas());

    // Colocar una pieza cambia el hash; quitar una, también.
    let mut mas = Estado::deserializar(&bytes, &reglas, "h").unwrap();
    let rec = mas.reglas.economia.clone();
    mas.asentamientos.get_mut(&1).unwrap().urbe.tomar(&rec.urbanismo, 99, Tipo::Almacen, &urbano(1), &mut 100).unwrap();
    assert_ne!(mas.hash(), s.hash());
    let mut menos = Estado::deserializar(&bytes, &reglas, "h").unwrap();
    menos.asentamientos.get_mut(&1).unwrap().urbe.quitar(7);
    assert_ne!(menos.hash(), s.hash());
    assert_ne!(menos.hash(), mas.hash());

    // Una foto de otra revisión de la geometría se rechaza; también el formato anterior.
    let semilla = a.semilla().to_le_bytes();
    let pos = bytes.windows(12).position(|w| w[4..] == semilla).expect("la ciudad empieza con su versión y su semilla");
    let mut otra = bytes.clone();
    otra[pos..pos + 4].copy_from_slice(&(trazado::LAYOUT_VERSION - 1).to_le_bytes());
    let Err(e) = Estado::deserializar(&otra, &reglas, "h") else { panic!("debería rechazarse") };
    assert!(e.to_string().contains("LAYOUT_VERSION"), "{e}");
    let mut vieja = bytes.clone();
    vieja[..4].copy_from_slice(b"BAN3");
    assert!(Estado::deserializar(&vieja, &reglas, "h").is_err());
}

// ───────────────────────────── Una plaza viva sobre el mundo real ─────────────────────────────

type M = (Arc<Mundo>, Arc<Rutas>);

fn mundo() -> Option<&'static M> {
    static M: OnceLock<Option<M>> = OnceLock::new();
    M.get_or_init(|| {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../generador/salida/mundo");
        let f = std::fs::read_dir(dir).ok()?.flatten().find(|e| e.file_name().to_string_lossy().starts_with("mundo-"))?;
        let m = Mundo::cargar(&std::fs::read(f.path()).ok()?, &OpcionesValidacion::default()).expect("artefacto válido");
        let r = Arc::new(Rutas::desde_mundo(&m));
        Some((Arc::new(m), r))
    })
    .as_ref()
}

macro_rules! con_mundo {
    () => {
        match mundo() {
            Some(m) => m,
            None => {
                eprintln!("sin artefacto de mundo real: se omite");
                return;
            }
        }
    };
}

fn arranque(m: &M, semilla: u64) -> Arranque {
    Arranque { reglas: Reglas::default(), ajustes: Ajustes::default(), semilla, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) }
}

fn admin(id: &str, c: Comando) -> Peticion {
    Peticion::nueva("admin", id, c).de_admin()
}

fn ana(id: &str, c: Comando) -> Peticion {
    Peticion::nueva("ana", id, c)
}

fn sitio_verde(n: &Nucleo, m: &Mundo, i: usize, faccion: u32) -> Punto {
    let p = m.campamentos()[i].posicion.unwrap();
    for r in (0..8).map(|k| 50_000 + k * 10_000) {
        for g in 0..360 {
            let a = (g as f64).to_radians();
            let q = Punto::new(p.x + (r as f64 * a.cos()) as i64, p.y + (r as f64 * a.sin()) as i64);
            if n.viabilidad(q, faccion).is_some_and(|v| v.motivo.is_none() && v.color == territorio::Viabilidad::Verde && v.parcelas_en_techo >= 6) {
                return q;
            }
        }
    }
    panic!("sin sitio rico cerca del campamento {i}");
}

fn fundar_en(n: &mut Nucleo, t: Instante, nombre: &str, faccion: u32, p: Punto) -> u64 {
    let e = n.crear_ejercito(t, "ana", faccion, p, 500, 100_000).unwrap();
    assert!(n.aplicar_en(t, admin(&format!("c-{nombre}"), Comando::DarCaravanaFundacion { ejercito: e })).aceptada());
    let r = n.aplicar_en(t, ana(&format!("f-{nombre}"), Comando::Fundar { ejercito: e }));
    assert!(r.aceptada(), "{r:?}");
    r.creadas[0]
}

fn conexa(c: &Ciudad) -> bool {
    let red: HashSet<Celda> = c.calles().into_iter().chain(c.caminos()).collect();
    let Some(&inicio) = red.iter().min() else { return true };
    let (mut vistas, mut cola) = (HashSet::from([inicio]), VecDeque::from([inicio]));
    while let Some(Celda { col, row }) = cola.pop_front() {
        for (dc, dr) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
            let n = Celda::new(col + dc, row + dr);
            if red.contains(&n) && vistas.insert(n) {
                cola.push_back(n);
            }
        }
    }
    vistas.len() == red.len()
}

/// Los invariantes de T §13 sobre la ciudad de una plaza viva.
fn verificar(n: &Nucleo, id: u64) {
    let c = n.ciudad(id).expect("la plaza tiene ciudad");
    let radio_ul = n.vista_ciudad(id).unwrap().radio_ul as i64;
    let prm = c.params();
    let piezas = c.piezas();
    for (k, a) in piezas.iter().enumerate() {
        for b in &piezas[k + 1..] {
            assert!(!a.rect.solapa(&b.rect), "{} y {} se solapan", a.id, b.id);
        }
        for cel in a.rect.celdas() {
            assert!(!c.es_calle(cel) && !c.es_camino(cel), "la pieza {} pisa la red en {cel:?}", a.id);
        }
    }
    assert!(conexa(c), "la red tiene islas");
    for p in piezas.iter().filter(|p| p.rol != Rol::Centro) {
        assert!(p.rect.adyacentes().into_iter().any(|cel| c.es_calle(cel) || c.es_camino(cel)), "la pieza {} ({:?}) no tiene salida a la red", p.id, TIPOS[p.tipo as usize]);
    }
    let anclas: Vec<_> = piezas.iter().filter(|p| p.rol.es_ancla()).collect();
    for (k, a) in anclas.iter().enumerate() {
        for b in &anclas[k + 1..] {
            assert!(a.rect.hueco(&b.rect) >= prm.separacion_seguridad_anclas, "anclas {} y {} demasiado juntas", a.id, b.id);
        }
    }
    let banda = prm.radio_afueras_min_ul as i64 * 1000;
    for p in piezas {
        if p.categoria == Categoria::Afueras {
            assert!(p.rect.dist2_borde_al_origen() >= banda * banda, "la granja o el corral {} está dentro de las afueras mínimas", p.id);
        } else {
            let (c0, r) = (p.rect.centro(), radio_ul * 1000);
            assert!(c0.x * c0.x + c0.y * c0.y <= r * r, "la pieza {} ({:?}) se sale del disco de {radio_ul} u.l.", p.id, TIPOS[p.tipo as usize]);
        }
    }
    // Un barrio con satélites tiene su ancla: el gate de nivel 2 lo respeta.
    for cat in [Categoria::Industria, Categoria::Militar] {
        if piezas.iter().any(|p| p.categoria == cat && p.rol == Rol::Normal) {
            assert!(piezas.iter().any(|p| p.categoria == cat && p.rol == Rol::AnclaSaturacion), "hay satélites {cat:?} sin ancla");
        }
    }
}

#[test]
fn una_plaza_vive_diez_dias_y_su_ciudad_cumple_los_invariantes() {
    let m = con_mundo!();
    let mut n = Nucleo::nuevo(arranque(m, 7));
    let p = sitio_verde(&n, &m.0, 0, 1);
    let id = fundar_en(&mut n, 90_123, "a", 1, p);
    verificar(&n, id);
    for hora in 2..=10 * 24 {
        let t = hora * HORA;
        n.avanzar_hasta(t);
        n.aplicar_en(t, ana(&format!("asc{id}-{hora}"), Comando::SolicitarAscenso { asentamiento: id }));
        if hora % 24 == 0 {
            verificar(&n, id);
        }
    }
    let eco = n.economia(id).expect("la plaza sigue viva");
    let c = n.ciudad(id).unwrap();
    let re = &n.estado().reglas.economia;
    ciudad_y_economia_cuadran(re, eco, c);
    let cuenta = |t: Tipo| eco.edificios.iter().filter(|e| e.tipo == t).count();
    let anclas: Vec<Tipo> = c.piezas().iter().filter(|p| p.rol == Rol::AnclaSaturacion).map(|p| TIPOS[p.tipo as usize]).collect();
    println!("{} piezas; {} viviendas, {} leñeras, {} canteras, {} almacenes, {} granjas, nivel {}; anclas {anclas:?}; radio {} u.l.", c.piezas().len(), cuenta(Tipo::Vivienda), cuenta(Tipo::Lenera), cuenta(Tipo::Cantera), cuenta(Tipo::Almacen), cuenta(Tipo::Granja), eco.nivel, n.vista_ciudad(id).unwrap().radio_ul);
    assert!(cuenta(Tipo::Lenera) >= 1 && cuenta(Tipo::Cantera) >= 1, "construye su Leñera y su Cantera sola");
    assert!(cuenta(Tipo::Vivienda) > 3, "y más Viviendas");
    // La Leñera es una pieza de la ciudad (genérico, 1×1); la Cantera solo ocupa su yacimiento (ámbito de mapa).
    for e in eco.edificios.iter().filter(|e| e.tipo == Tipo::Lenera) {
        assert_eq!(c.pieza(e.id).unwrap().categoria, Categoria::Generico);
    }
    assert!(eco.edificios.iter().filter(|e| e.tipo == Tipo::Cantera).all(|e| c.pieza(e.id).is_none()));
    verificar(&n, id);

    // La vista para pintar: piezas con su rectángulo en unidades locales, calles y caminos.
    let v = n.vista_ciudad(id).unwrap();
    assert_eq!(v.piezas.len(), c.piezas().len());
    assert!(!v.calles.is_empty() && !v.caminos.is_empty());
    let cu = v.piezas.iter().find(|p| p.tipo == Tipo::CentroUrbano).unwrap();
    assert_eq!((cu.rect.x, cu.rect.y, cu.rect.ancho, cu.rect.alto, cu.centro), (-6, -6, 12, 12, Punto::new(0, 0)));

    // Foto y vuelta: la misma ciudad, el mismo hash.
    let e2 = Estado::deserializar(&n.estado().serializar(), &Reglas::default(), m.0.hash()).unwrap();
    assert_eq!(e2.hash(), n.hash_estado());
    assert_eq!(e2.asentamientos[&id].urbe, n.estado().asentamientos[&id].urbe);
}

#[test]
fn encolar_y_quitar_un_edificio_en_la_plaza_viva_coloca_y_libera_su_pieza() {
    let m = con_mundo!();
    let mut n = Nucleo::nuevo(arranque(m, 7));
    let p = sitio_verde(&n, &m.0, 0, 1);
    let id = fundar_en(&mut n, 0, "a", 1, p);
    assert!(n.aplicar_en(0, admin("pausa", Comando::AlternarAutoConstruccion { asentamiento: id, pausada: true })).aceptada());
    for (recurso, cantidad) in [(asentamiento::Recurso::Madera, 300_000), (asentamiento::Recurso::Piedra, 300_000)] {
        assert!(n.aplicar_en(0, admin(&format!("dar-{recurso:?}"), Comando::DarRecursos { asentamiento: id, recurso, cantidad })).aceptada());
    }
    // Las dos cuadrillas ocupadas: lo siguiente espera en la cola (una obra en curso no se quita).
    for k in 0..2 {
        assert!(n.aplicar_en(0, ana(&format!("t{k}"), Comando::AnadirEdificio { asentamiento: id, tipo: Tipo::Vivienda })).aceptada());
    }
    let (hash0, piezas0) = (n.hash_estado(), n.ciudad(id).unwrap().piezas().len());
    let r = n.aplicar_en(0, ana("v1", Comando::AnadirEdificio { asentamiento: id, tipo: Tipo::Vivienda }));
    assert!(r.aceptada(), "{r:?}");
    let v = r.creadas[0] as u32;
    assert_eq!(n.ciudad(id).unwrap().piezas().len(), piezas0 + 1);
    assert_eq!(TIPOS[n.ciudad(id).unwrap().pieza(v).unwrap().tipo as usize], Tipo::Vivienda);
    assert_ne!(n.hash_estado(), hash0);
    let q = n.aplicar_en(0, ana("q1", Comando::QuitarDeCola { asentamiento: id, edificio: v }));
    assert!(q.aceptada(), "{q:?}");
    assert!(n.ciudad(id).unwrap().pieza(v).is_none());
    assert_eq!(n.ciudad(id).unwrap().piezas().len(), piezas0);
    verificar(&n, id);
    // Un edificio de ámbito de mapa (la Cantera) no ocupa la ciudad.
    let c = n.aplicar_en(0, ana("c1", Comando::AnadirEdificio { asentamiento: id, tipo: Tipo::Cantera }));
    assert!(c.aceptada(), "{c:?}");
    assert_eq!(n.ciudad(id).unwrap().piezas().len(), piezas0);
}
