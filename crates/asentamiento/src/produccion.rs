//! Producción de un asentamiento (Economia_Construccion §5): mano de obra, granjas, Leñeras, extractores y recetas de los
//! talleres. Un paso es **un minuto**: cada edificio activo produce en el orden de la lista (importa si el almacén se llena a mitad
//! del recorrido), siempre con tope de capacidad. Con el almacén lleno **se pausa**: un extractor no gasta el yacimiento por lo que
//! no cabe y un taller no consume insumos por lo que no puede guardar.

use crate::almacen::Almacen;
use crate::catalogo::{Catalogo, Tipo};
use crate::datos::{por, razon, UNO};
use crate::edificio::{Edificio, Edificios, Fuente};
use crate::recurso::{Costo, Recurso};
use base::Mil;

/// El mundo que ve la producción: los yacimientos finitos y las parcelas de bosque.
pub trait Fuentes {
    /// Densidad de la parcela de bosque de una Leñera, en ppm (1,0 = `UNO`). 0 si no existe.
    fn densidad_parcela(&self, parcela: u32) -> i64;
    /// Saca hasta `pedido` del nodo y devuelve lo que salió (`min(pedido, stock)`); solo se gasta lo que sale.
    fn extraer(&mut self, nodo: u32, pedido: Mil) -> Mil;
    /// Lo que fabricaron las recetas de los talleres este minuto, por recurso (milésimas): el mundo lo cuenta para los logros `fabricado.*` (Tecnologia_Aedas §2.1, sistema 6). Por
    /// defecto no se cuenta.
    fn fabricado(&mut self, _recurso: Recurso, _cantidad: Mil) {}
}

/// Lo que condiciona la producción y viene de fuera (zona, políticas, tecnologías).
pub struct Contexto<'a> {
    /// `fertilidadZona`: la mejor fertilidad del terreno de la zona, en ppm. Todas las Granjas comparten este valor.
    pub fertilidad: i64,
    /// `factorProduccionTrigo` de las políticas (Edicto de Cosecha ×1,5), ppm.
    pub factor_trigo: i64,
    /// `factorProduccionTalleres` (Barrios Gremiales ×1,1), ppm.
    pub factor_talleres: i64,
    /// Producto de los `bonusProduccion.factor` de las tecnologías adoptadas para ese recurso (ppm; `canteria` ×1,5 la piedra).
    pub factor_tecnologia: &'a dyn Fn(Recurso) -> i64,
    /// ¿La Facción tiene adoptada esa tecnología?
    pub tiene_tecnologia: &'a dyn Fn(&str) -> bool,
    /// Recetas paradas por el Gobernador o el Maestro de Obras (por el recurso que producen).
    pub recetas_pausadas: &'a [Recurso],
}

/// Cuánto de lo que piden los edificios cubren los habitantes, en ppm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mano {
    pub pesants: i64,
    pub artesanos: i64,
    /// Trabajadores que piden los edificios de pesants y los talleres (milésimas), para el pool de reclutamiento.
    pub requeridos_pesants: Mil,
    pub requeridos_artesanos: Mil,
}

/// `ratioManoObra`: `min(1, pesants / trabajadoresRequeridosTotal)` (1 si no se pide nadie), y lo mismo con los artesanos.
/// No hay asignación por edificio: es una demanda agregada. Un extractor con el nodo agotado sigue contando. `pueblos` son los pesants que
/// piden los pueblos extractores activos de la plaza (no tienen población propia: trabajan los de la madre).
pub fn mano_de_obra(cat: &Catalogo, edificios: &[Edificio], pesants_enteros: i64, artesanos_enteros: i64, pueblos: Mil) -> Mano {
    let activos = edificios.iter().filter(|e| e.activo());
    let requeridos_pesants: Mil = activos.clone().filter(|e| e.tipo.usa_pesants()).map(|e| cat.tipo(e.tipo).trabajadores).sum::<Mil>() + pueblos;
    let requeridos_artesanos: Mil =
        activos.filter(|e| e.tipo.es_taller()).map(|e| cat.tipo(e.tipo).nivel(e.nivel_interno).map_or(0, |n| n.trabajadores)).sum();
    let ratio = |habitantes: i64, requeridos: Mil| if requeridos <= 0 { UNO } else { razon(habitantes * 1000, requeridos).min(UNO) };
    Mano { pesants: ratio(pesants_enteros, requeridos_pesants), artesanos: ratio(artesanos_enteros, requeridos_artesanos), requeridos_pesants, requeridos_artesanos }
}

/// Reserva mínima: lo que el mantenimiento y la comida van a cobrar en los próximos minutos (§6.3). Una obra, un taller o una
/// mejora automáticos no pueden bajar un recurso de ahí. La reserva manual del Tesorero se suma aparte, solo en el camino automático.
pub fn reserva_dinamica(coste_mantenimiento: &Costo, consumo_comida: Mil, horizonte_mantenimiento: i64, horizonte_comida: i64) -> [Mil; 29] {
    let mut r = [0; 29];
    for &(rec, n) in coste_mantenimiento {
        r[rec.indice()] += n * horizonte_mantenimiento;
    }
    r[Recurso::Trigo.indice()] += consumo_comida * horizonte_comida;
    r
}

/// Una pasada de extractores, granjas y Leñeras (un minuto). Devuelve lo producido por recurso (milésimas), para los logros.
pub fn extraer_y_cultivar(cat: &Catalogo, edificios: &mut [Edificio], almacen: &mut Almacen, mano: &Mano, ctx: &Contexto, fuentes: &mut dyn Fuentes) -> Vec<(Recurso, Mil)> {
    let mut producido: Vec<(Recurso, Mil)> = vec![];
    let mut sumar = |r: Recurso, n: Mil| match producido.iter_mut().find(|(x, _)| *x == r) {
        Some(p) => p.1 += n,
        None => producido.push((r, n)),
    };
    for e in edificios.iter_mut().filter(|e| e.activo() && e.tipo.usa_pesants()) {
        let def = cat.tipo(e.tipo);
        match e.tipo {
            Tipo::Granja => {
                let base = def.nivel(e.nivel_interno).map_or(0, |n| n.produccion_base_trigo);
                let deseado = por(por(por(base, ctx.fertilidad), mano.pesants), ctx.factor_trigo);
                let entra = almacen.producir(Recurso::Trigo, deseado);
                e.pausado_por_almacen = entra < deseado;
                sumar(Recurso::Trigo, entra);
            }
            Tipo::Lenera => {
                let Some((_, base)) = def.produccion else { continue };
                let densidad = match e.fuente {
                    Some(Fuente::Parcela(p)) => fuentes.densidad_parcela(p),
                    _ => 0,
                };
                let deseado = por(por(base, densidad), mano.pesants);
                let entra = almacen.producir(Recurso::Madera, deseado);
                e.pausado_por_almacen = entra < deseado;
                sumar(Recurso::Madera, entra);
            }
            _ => {
                let (Some((recurso, base)), Some(Fuente::Nodo(nodo))) = (def.produccion, e.fuente) else { continue };
                let deseado = por(por(base, mano.pesants), (ctx.factor_tecnologia)(recurso));
                let pedido = deseado.min(almacen.hueco(recurso));
                e.pausado_por_almacen = pedido < deseado;
                let salido = if pedido > 0 { fuentes.extraer(nodo, pedido) } else { 0 };
                sumar(recurso, almacen.producir(recurso, salido));
            }
        }
    }
    producido
}

/// Una pasada de recetas de los talleres (un minuto), en el orden de la lista y del catálogo: una receta puede consumir lo que otra
/// produjo este mismo minuto. `reserva` es la misma de la auto-construcción más la manual.
pub fn recetas(cat: &Catalogo, edificios: &mut [Edificio], almacen: &mut Almacen, mano: &Mano, ctx: &Contexto, reserva: &[Mil; 29]) -> Vec<(Recurso, Mil)> {
    let mut hecho: Vec<(Recurso, Mil)> = vec![];
    // `requiereEdificio` se mide con el estado al empezar la pasada (un taller recién activo no se cuenta a sí mismo dos veces).
    for i in 0..edificios.len() {
        let e = &edificios[i];
        if !e.activo() || !e.tipo.es_taller() {
            continue;
        }
        let Some(nivel) = cat.tipo(e.tipo).nivel(e.nivel_interno) else { continue };
        let mut limitado = false;
        for rc in &nivel.recetas {
            if ctx.recetas_pausadas.contains(&rc.produce) || rc.tecnologia.as_deref().is_some_and(|t| !(ctx.tiene_tecnologia)(t)) {
                continue;
            }
            let mut cantidad = por(por(rc.produccion_base, mano.artesanos), ctx.factor_talleres);
            for &(insumo, por_unidad) in &rc.consume_por_unidad {
                let disponible = (almacen.disponible(insumo) - reserva[insumo.indice()]).max(0);
                cantidad = cantidad.min(disponible * 1000 / por_unidad.max(1));
            }
            let espacio = almacen.hueco(rc.produce);
            if cantidad > espacio {
                cantidad = espacio;
                limitado = true;
            }
            if cantidad <= 0 {
                continue;
            }
            for &(insumo, por_unidad) in &rc.consume_por_unidad {
                almacen.consumir(insumo, (por_unidad * cantidad + 999) / 1000); // hacia arriba: una receta nunca sale más barata
            }
            almacen.producir(rc.produce, cantidad);
            match hecho.iter_mut().find(|(x, _)| *x == rc.produce) {
                Some(p) => p.1 += cantidad,
                None => hecho.push((rc.produce, cantidad)),
            }
        }
        edificios[i].pausado_por_almacen = limitado;
    }
    hecho
}

/// Trigo por minuto que producen ahora las Granjas activas (para el déficit de la auto-construcción).
pub fn produccion_trigo_por_minuto(cat: &Catalogo, edificios: &[Edificio], mano: &Mano, fertilidad: i64, factor_trigo: i64) -> Mil {
    edificios
        .iter()
        .filter(|e| e.activo() && e.tipo == Tipo::Granja)
        .map(|e| por(por(por(cat.tipo(e.tipo).nivel(e.nivel_interno).map_or(0, |n| n.produccion_base_trigo), fertilidad), mano.pesants), factor_trigo))
        .sum()
}

/// Pesants y artesanos que no hacen falta para producir: el pool de reclutamiento (`poblacionDisponibleParaReclutar`).
pub fn disponibles_para_reclutar(mano: &Mano, pesants_enteros: i64, artesanos_enteros: i64) -> (i64, i64) {
    (((pesants_enteros * 1000 - mano.requeridos_pesants) / 1000).max(0), ((artesanos_enteros * 1000 - mano.requeridos_artesanos) / 1000).max(0))
}

/// Cuántos extractores activos de un tipo hay (útil para las pruebas y el reemplazo de fuentes).
pub fn activos_de(edificios: &[Edificio], t: Tipo) -> usize {
    edificios.activos(t)
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::datos::Reglas;
    use crate::edificio::Estado;
    use std::collections::HashMap;

    #[derive(Default)]
    struct Mundo {
        stock: HashMap<u32, Mil>,
        parcelas: HashMap<u32, i64>,
    }

    impl Fuentes for Mundo {
        fn densidad_parcela(&self, p: u32) -> i64 {
            self.parcelas.get(&p).copied().unwrap_or(0)
        }
        fn extraer(&mut self, n: u32, pedido: Mil) -> Mil {
            let s = self.stock.entry(n).or_insert(0);
            let sale = pedido.min(*s);
            *s -= sale;
            sale
        }
    }

    fn ctx<'a>(pausadas: &'a [Recurso], tecnologias: &'a [&'a str]) -> Contexto<'a> {
        Contexto {
            fertilidad: UNO,
            factor_trigo: UNO,
            factor_talleres: UNO,
            factor_tecnologia: &|_| UNO,
            tiene_tecnologia: Box::leak(Box::new(move |t: &str| tecnologias.contains(&t))),
            recetas_pausadas: pausadas,
        }
    }

    fn activo(id: u32, t: Tipo) -> Edificio {
        Edificio::nuevo(id, t, Estado::Activo)
    }

    #[test]
    fn la_mano_de_obra_es_una_demanda_agregada() {
        let cat = Reglas::cargar().catalogo;
        // Granja (4) + Leñera (4) + Cantera (4) piden 12 pesants; con 6 el ratio es 0,5.
        let e = vec![activo(1, Tipo::Granja), activo(2, Tipo::Lenera), activo(3, Tipo::Cantera), activo(4, Tipo::Vivienda)];
        let m = mano_de_obra(&cat, &e, 6, 0, 0);
        assert_eq!((m.pesants, m.artesanos, m.requeridos_pesants), (500_000, UNO, 12_000));
        assert_eq!(mano_de_obra(&cat, &e, 20, 0, 0).pesants, UNO, "con pesants de sobra el ratio no pasa de 1");
        assert_eq!(disponibles_para_reclutar(&mano_de_obra(&cat, &e, 20, 0, 0), 20, 0), (8, 0), "los que sobran son el pool de reclutamiento");
        // Un taller pide artesanos según su nivel interno.
        let t = vec![activo(1, Tipo::Fundicion)];
        assert_eq!(mano_de_obra(&cat, &t, 0, 2, 0).artesanos, 500_000, "2 de los 4 que pide la Fundición de nivel 1");
        assert_eq!(mano_de_obra(&cat, &[], 0, 0, 0).pesants, UNO);
    }

    #[test]
    fn la_granja_produce_60_de_trigo_a_plena_mano_de_obra_y_el_edicto_la_sube() {
        let cat = Reglas::cargar().catalogo;
        let (mut a, mut e, mut w) = (Almacen::nuevo(400_000), vec![activo(1, Tipo::Granja)], Mundo::default());
        let m = mano_de_obra(&cat, &e, 20, 0, 0);
        let p = extraer_y_cultivar(&cat, &mut e, &mut a, &m, &ctx(&[], &[]), &mut w);
        assert_eq!((a.cantidad(Recurso::Trigo), p), (60_000, vec![(Recurso::Trigo, 60_000)]));
        let c = Contexto { factor_trigo: 1_500_000, fertilidad: 500_000, ..ctx(&[], &[]) };
        extraer_y_cultivar(&cat, &mut e, &mut a, &m, &c, &mut w);
        assert_eq!(a.cantidad(Recurso::Trigo), 60_000 + 45_000, "60 × fertilidad 0,5 × Edicto 1,5");
    }

    #[test]
    fn con_el_almacen_lleno_se_pausa_y_el_yacimiento_no_se_gasta() {
        let cat = Reglas::cargar().catalogo;
        let (mut a, mut w) = (Almacen::nuevo(400_000), Mundo::default());
        w.stock.insert(7, 100_000);
        let mut cantera = activo(1, Tipo::Cantera);
        cantera.fuente = Some(Fuente::Nodo(7));
        let mut e = vec![cantera];
        let m = mano_de_obra(&cat, &e, 10, 0, 0);
        a.poner(Recurso::Piedra, 395_000);
        extraer_y_cultivar(&cat, &mut e, &mut a, &m, &ctx(&[], &[]), &mut w);
        assert_eq!(a.cantidad(Recurso::Piedra), 400_000);
        assert_eq!(w.stock[&7], 95_000, "solo se gastan los 5 que cabían, no los 10 que sacaría");
        assert!(e[0].pausado_por_almacen);
        extraer_y_cultivar(&cat, &mut e, &mut a, &m, &ctx(&[], &[]), &mut w);
        assert_eq!(w.stock[&7], 95_000, "lleno del todo: no extrae nada");
        a.quitar(Recurso::Piedra, 400_000);
        extraer_y_cultivar(&cat, &mut e, &mut a, &m, &ctx(&[], &[]), &mut w);
        assert!(!e[0].pausado_por_almacen);
        assert_eq!(a.cantidad(Recurso::Piedra), 10_000);
    }

    #[test]
    fn la_cantera_se_limita_al_stock_del_nodo_y_la_tecnologia_la_multiplica() {
        let cat = Reglas::cargar().catalogo;
        let (mut a, mut w) = (Almacen::nuevo(400_000), Mundo::default());
        w.stock.insert(1, 12_000);
        let mut cantera = activo(1, Tipo::Cantera);
        cantera.fuente = Some(Fuente::Nodo(1));
        let mut e = vec![cantera];
        let m = mano_de_obra(&cat, &e, 10, 0, 0);
        let c = Contexto { factor_tecnologia: &|r| if r == Recurso::Piedra { 1_500_000 } else { UNO }, ..ctx(&[], &[]) };
        extraer_y_cultivar(&cat, &mut e, &mut a, &m, &c, &mut w);
        assert_eq!(a.cantidad(Recurso::Piedra), 12_000, "pide 15 con canteria pero el nodo solo tiene 12");
        extraer_y_cultivar(&cat, &mut e, &mut a, &m, &c, &mut w);
        assert_eq!(a.cantidad(Recurso::Piedra), 12_000, "nodo agotado: nada hasta que se regenere");
    }

    #[test]
    fn la_lenera_depende_de_la_densidad_de_su_parcela() {
        let cat = Reglas::cargar().catalogo;
        let (mut a, mut w) = (Almacen::nuevo(400_000), Mundo::default());
        w.parcelas.insert(3, 800_000);
        let mut l = activo(1, Tipo::Lenera);
        l.fuente = Some(Fuente::Parcela(3));
        let mut e = vec![l, activo(2, Tipo::Lenera)];
        e[1].fuente = Some(Fuente::Parcela(99));
        let m = mano_de_obra(&cat, &e, 8, 0, 0);
        extraer_y_cultivar(&cat, &mut e, &mut a, &m, &ctx(&[], &[]), &mut w);
        assert_eq!(a.cantidad(Recurso::Madera), 4_000, "5 × 0,8 de la primera; la segunda no tiene parcela");
    }

    #[test]
    fn la_fundicion_convierte_cobre_en_lingotes_hasta_donde_llega_el_cobre_y_respeta_la_reserva() {
        let cat = Reglas::cargar().catalogo;
        let mut a = Almacen::nuevo(400_000);
        let mut e = vec![activo(1, Tipo::Fundicion)];
        let m = mano_de_obra(&cat, &e, 0, 4, 0);
        a.poner(Recurso::Cobre, 6_000);
        let c = ctx(&[], &[]);
        let h = recetas(&cat, &mut e, &mut a, &m, &c, &[0; 29]);
        assert_eq!((a.cantidad(Recurso::Cobre), a.cantidad(Recurso::LingoteCobre)), (0, 3_000), "6 cobre → 3 lingotes (2 por 1), no los 5 del techo");
        assert_eq!(h, vec![(Recurso::LingoteCobre, 3_000)]);
        // Con 20 de cobre sale el techo de 5 lingotes (10 cobre).
        a.poner(Recurso::Cobre, 20_000);
        recetas(&cat, &mut e, &mut a, &m, &c, &[0; 29]);
        assert_eq!((a.cantidad(Recurso::Cobre), a.cantidad(Recurso::LingoteCobre)), (10_000, 8_000));
        // Una reserva de 18 deja solo 2 de cobre utilizables → 1 lingote.
        let mut reserva = [0; 29];
        reserva[Recurso::Cobre.indice()] = 18_000;
        a.poner(Recurso::Cobre, 20_000);
        let antes = a.cantidad(Recurso::LingoteCobre);
        recetas(&cat, &mut e, &mut a, &m, &c, &reserva);
        assert_eq!((a.cantidad(Recurso::Cobre), a.cantidad(Recurso::LingoteCobre) - antes), (18_000, 1_000));
    }

    #[test]
    fn faltando_artesanos_se_produce_proporcionalmente_menos_y_una_receta_pausada_no_corre() {
        let cat = Reglas::cargar().catalogo;
        let mut a = Almacen::nuevo(400_000);
        a.poner(Recurso::Cobre, 100_000);
        let mut e = vec![activo(1, Tipo::Fundicion)];
        let m = mano_de_obra(&cat, &e, 0, 2, 0);
        recetas(&cat, &mut e, &mut a, &m, &ctx(&[], &[]), &[0; 29]);
        assert_eq!((a.cantidad(Recurso::LingoteCobre), a.disponible(Recurso::LingoteCobre)), (2_000, 2_500), "ratio 0,5 de los 5 del techo: 2 enteros y medio en el resto");
        let antes = a.cantidad(Recurso::Cobre);
        recetas(&cat, &mut e, &mut a, &m, &ctx(&[Recurso::LingoteCobre], &[]), &[0; 29]);
        assert_eq!(a.cantidad(Recurso::Cobre), antes, "pausada por el Gobernador");
    }

    #[test]
    fn la_tecnologia_abre_recetas_y_el_almacen_lleno_pausa_el_taller() {
        let cat = Reglas::cargar().catalogo;
        let mut a = Almacen::nuevo(400_000);
        let mut f = activo(1, Tipo::Fundicion);
        f.nivel_interno = 2;
        let mut e = vec![f];
        let m = mano_de_obra(&cat, &e, 0, 8, 0);
        a.poner(Recurso::Estano, 100_000);
        recetas(&cat, &mut e, &mut a, &m, &ctx(&[], &[]), &[0; 29]);
        assert_eq!(a.cantidad(Recurso::LingoteEstano), 0, "sin aleacion_bronce no funde estaño");
        recetas(&cat, &mut e, &mut a, &m, &ctx(&[], &["aleacion_bronce"]), &[0; 29]);
        assert_eq!(a.cantidad(Recurso::LingoteEstano), 3_000);
        // Con el producto lleno, no consume insumos y queda marcado.
        a.poner(Recurso::LingoteEstano, 400_000);
        let antes = a.cantidad(Recurso::Estano);
        recetas(&cat, &mut e, &mut a, &m, &ctx(&[], &["aleacion_bronce"]), &[0; 29]);
        assert_eq!(a.cantidad(Recurso::Estano), antes);
        assert!(e[0].pausado_por_almacen);
    }

    #[test]
    fn la_reserva_cubre_ocho_minutos_de_mantenimiento_y_de_comida() {
        let r = reserva_dinamica(&vec![(Recurso::Madera, 1_650), (Recurso::Piedra, 3_300)], 2_000, 8, 8);
        assert_eq!((r[Recurso::Madera.indice()], r[Recurso::Piedra.indice()], r[Recurso::Trigo.indice()], r[Recurso::Oro.indice()]), (13_200, 26_400, 16_000, 0));
    }

    #[test]
    fn la_produccion_de_trigo_suma_las_granjas_activas() {
        let cat = Reglas::cargar().catalogo;
        let mut g2 = activo(2, Tipo::Granja);
        g2.nivel_interno = 2;
        let e = vec![activo(1, Tipo::Granja), g2, Edificio::nuevo(3, Tipo::Granja, Estado::EnCola)];
        let m = mano_de_obra(&cat, &e, 20, 0, 0);
        assert_eq!(produccion_trigo_por_minuto(&cat, &e, &m, UNO, UNO), 150_000, "60 + 90; la que está en cola no cuenta");
    }
}
