//! Los invariantes del combate que `batch militar` comprueba **orden a orden**: antes de cada `Atacar` se fotografía lo que el combate puede tocar (cada escuadra
//! del mundo, los heridos, los recursos de todos los contenedores, la carga del atacante, la caravana atacada, los recintos de la plaza atacada y la guarnición de
//! serie) y, con la orden aplicada, se compara con lo que dicen los hechos de esa versión. Una orden se aplica con los eventos hasta su instante ya procesados, así
//! que todo lo que cambia entre las dos fotos es de la orden.

use asentamiento::datos::UNO;
use asentamiento::recintos::Recinto;
use asentamiento::recurso::TODOS;
use asentamiento::Recurso;
use base::{Instante, Mil};
use comercio::caravana::EstadoCaravana;
use militar::catalogo::Tropa;
use militar::escuadra::Contenedor;
use militar::heroe::Residencia;
use militar::presencia::Ubicacion;
use nucleo::{Nucleo, ObjetivoAtaque};
use std::collections::BTreeMap;

/// Lo que importa de una escuadra (de un héroe o de una guarnición de serie).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Esc {
    pub tropa: Tropa,
    pub cantidad: i64,
    pub experiencia: Mil,
    pub nivel: u8,
    pub contenedor: Contenedor,
}

/// Toda escuadra del mundo: las de los héroes y las guarniciones de serie de pueblos y avanzadas.
pub fn escuadras(n: &Nucleo) -> BTreeMap<u64, Esc> {
    let de = |s: &militar::escuadra::Escuadra| (s.id, Esc { tropa: s.tropa, cantidad: s.cantidad, experiencia: s.experiencia, nivel: s.nivel, contenedor: s.contenedor });
    let c = &n.estado().combate;
    n.heroes()
        .flat_map(|h| h.heroe.escuadras.iter().map(de))
        .chain(c.guarniciones.values().map(|g| de(&g.escuadra)))
        .chain(c.avanzadas.values().flat_map(|g| g.escuadras.iter().map(de)))
        .collect()
}

/// Lo que hay de cada recurso en todos los contenedores del mundo: almacenes de plazas, pueblos y avanzadas (también las huérfanas), carros de columnas, caravanas, almacenes
/// personales y carros de los héroes.
pub fn recursos(n: &Nucleo) -> [i64; 29] {
    let mut v = [0i64; 29];
    let e = n.estado();
    let mut sumar = |r: Recurso, q: i64| v[r.indice()] += q;
    for a in e.asentamientos.values() {
        for &r in TODOS.iter() {
            sumar(r, a.economia.almacen.disponible(r));
            a.economia.pueblos.iter().for_each(|p| sumar(r, p.almacen.disponible(r)));
            a.economia.avanzadas.iter().for_each(|x| sumar(r, x.almacen.disponible(r)));
        }
    }
    for x in e.combate.huerfanas.values() {
        TODOS.iter().for_each(|&r| sumar(r, x.almacen.disponible(r)));
    }
    e.ejercitos.values().flat_map(|c| c.columna.carga.iter()).for_each(|(&r, &q)| sumar(r, q));
    e.comercio.flota.iter().flat_map(|c| c.contenido.iter()).for_each(|(&r, &q)| sumar(r, q));
    n.heroes().flat_map(|h| h.almacen_personal.iter().chain(h.carro.iter())).for_each(|(&r, &q)| sumar(r, q));
    v
}

pub const CHATARRA: [Recurso; 3] = [Recurso::ChatarraCobre, Recurso::ChatarraBronce, Recurso::ChatarraHierro];

/// La guarnición de serie del pueblo o de la avanzada: sus soldados y si está completa. `None` si aún no ha nacido (nace completa la primera vez que se la necesita) o si
/// la avanzada la perdió al caer.
pub fn serie(n: &Nucleo, objetivo: ObjetivoAtaque) -> Option<(i64, bool)> {
    let e = n.estado();
    let (c, cr) = (&e.combate, &e.reglas.economia.combate);
    match objetivo {
        ObjetivoAtaque::Pueblo(x) => c.guarniciones.get(&x).map(|g| (g.escuadra.cantidad, g.escuadra.cantidad >= cr.pueblos.mineros)),
        ObjetivoAtaque::Avanzada(x) => c.avanzadas.get(&x).filter(|g| !g.escuadras.is_empty()).map(|g| (g.escuadras.iter().map(|s| s.cantidad).sum(), !combate::avanzada::incompleta(cr, &g.escuadras))),
        _ => None,
    }
}

/// Los soldados con los que nacería ahora la guarnición de serie del pueblo o de la avanzada, si aún no existe: los que la orden puede crear.
fn nacen(n: &Nucleo, objetivo: ObjetivoAtaque) -> i64 {
    let e = n.estado();
    let cr = &e.reglas.economia.combate;
    if serie(n, objetivo).is_some() {
        return 0;
    }
    match objetivo {
        ObjetivoAtaque::Pueblo(x) => n.pueblo(x).filter(|(_, p)| p.activo()).map_or(0, |_| cr.pueblos.mineros),
        ObjetivoAtaque::Avanzada(x) => n.avanzada(x).filter(|(_, a)| a.operativa()).map_or(0, |(_, a)| {
            combate::avanzada::composicion(cr, a.etapa, &n.tropas_elegidas_de_avanzada(x)).iter().map(|&t| e.reglas.economia.militar.tropas.tropa(t).unidades).sum()
        }),
        _ => 0,
    }
}

/// La foto de antes de una orden de ataque.
pub struct Antes {
    version: u64,
    escuadras: BTreeMap<u64, Esc>,
    /// Héroe → hasta cuándo está herido.
    heridos: BTreeMap<u64, Option<Instante>>,
    recursos: [i64; 29],
    /// La caravana atacada: carros y carga total.
    caravana: Option<(usize, i64)>,
    recintos: Vec<Recinto>,
    serie: Option<(i64, bool)>,
    nacen: i64,
    carga_atacante: i64,
}

impl Antes {
    pub fn de(n: &Nucleo, columna: u64, objetivo: ObjetivoAtaque) -> Antes {
        let e = n.estado();
        let caravana = match objetivo {
            ObjetivoAtaque::Caravana(k) => e.comercio.caravana(k).map(|c| (c.carros.len(), c.contenido.values().sum())),
            _ => None,
        };
        let recintos = match objetivo {
            ObjetivoAtaque::Asentamiento(p) => e.asentamientos.get(&p).map(|a| a.economia.recintos.clone()).unwrap_or_default(),
            _ => vec![],
        };
        Antes {
            version: n.version(),
            escuadras: escuadras(n),
            heridos: n.heroes().map(|h| (h.heroe.id, h.heroe.herido_hasta)).collect(),
            recursos: recursos(n),
            caravana,
            recintos,
            serie: serie(n, objetivo),
            nacen: nacen(n, objetivo),
            carga_atacante: e.ejercitos.get(&columna).map_or(0, |c| c.columna.carga.values().sum()),
        }
    }
}

/// Un hecho de la orden: su código y sus datos enteros.
#[derive(Clone, Debug)]
pub struct Hecho {
    pub codigo: String,
    pub datos: Vec<(&'static str, i64)>,
}

impl Hecho {
    pub fn dato(&self, clave: &str) -> i64 {
        self.datos.iter().find(|(k, _)| *k == clave).map_or(0, |&(_, v)| v)
    }
}

/// Lo que una orden de ataque rompió (todo debe quedar a 0).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ViolacionesCombate {
    /// Las bajas de un bando no son los soldados que perdieron sus escuadras.
    pub bajas: u64,
    /// En un combate entre columnas cambió una escuadra que no era de ninguna de las dos.
    pub ajenas: u64,
    /// Una escuadra que perdió experiencia o nivel.
    pub experiencia: u64,
    /// Un herido cuyas escuadras de columna perdieron soldados, o una herida que no dura `heridoMinutos` o que se alargó.
    pub heridos: u64,
    /// Algún recurso (salvo la chatarra y el oro de los bandidos) que creció, o chatarra u oro por encima de lo que dicen los hechos.
    pub conservacion: u64,
    /// Los carros o la carga de la caravana atacada no siguen la regla de su modalidad.
    pub carros: u64,
    /// El botín no es lo que entró en el carro, o el carro queda por encima de su capacidad.
    pub botin: u64,
    /// Un héroe solo (sin soldados) que destruyó un campamento de bandidos.
    pub heroe_solo: u64,
    /// La conquista no dejó la plaza como dicen las cascadas de comercio y de murallas.
    pub cascada: u64,
}

impl ViolacionesCombate {
    pub fn sumar(&mut self, o: &ViolacionesCombate) {
        self.bajas += o.bajas;
        self.ajenas += o.ajenas;
        self.experiencia += o.experiencia;
        self.heridos += o.heridos;
        self.conservacion += o.conservacion;
        self.carros += o.carros;
        self.botin += o.botin;
        self.heroe_solo += o.heroe_solo;
        self.cascada += o.cascada;
    }
}

/// Lo que se mide de una orden de ataque además de sus invariantes.
#[derive(Debug, Default)]
pub struct Efecto {
    pub v: ViolacionesCombate,
    /// Los hechos de la orden.
    pub hechos: Vec<Hecho>,
    /// Soldados perdidos por tropa (bajas y lo que queda a 0 por regla).
    pub bajas_por_tropa: BTreeMap<Tropa, i64>,
    /// Héroes que quedaron heridos con esta orden.
    pub heridos: u64,
    /// La guarnición de serie atacada antes y después (soldados y si está completa).
    pub serie: (Option<(i64, bool)>, Option<(i64, bool)>),
}

impl Efecto {
    /// El hecho del combate.
    pub fn combate(&self) -> Option<&Hecho> {
        del_combate(&self.hechos)
    }
}

/// El hecho del combate entre los de una orden: el primero `combate.*` que no es el aviso del cruce.
fn del_combate(hechos: &[Hecho]) -> Option<&Hecho> {
    hechos.iter().find(|h| h.codigo.starts_with("combate.") && h.codigo != "combate.encuentro")
}

fn perdida(antes: &BTreeMap<u64, Esc>, despues: &BTreeMap<u64, Esc>, ids: &[u64]) -> i64 {
    ids.iter().map(|id| antes[id].cantidad - despues.get(id).map_or(0, |e| e.cantidad)).sum()
}

/// Compara la foto de antes con el estado de después de la orden de ataque de la columna `columna` contra `objetivo`, en `t`.
pub fn comparar(n: &Nucleo, a: &Antes, t: Instante, columna: u64, objetivo: ObjetivoAtaque) -> Efecto {
    let hechos: Vec<Hecho> = n.cambios_desde(a.version).map(|c| c.flat_map(|c| c.eventos.iter()).map(|e| Hecho { codigo: e.codigo.to_string(), datos: e.datos.clone() }).collect()).unwrap_or_default();
    let despues = escuadras(n);
    let e = n.estado();
    let combate = del_combate(&hechos);
    let adjuntas_perdidas = hechos.iter().any(|x| x.codigo == "ejercito.caravanas_perdidas");
    let (mut v, mut bajas_por_tropa, mut heridos) = (ViolacionesCombate::default(), BTreeMap::new(), 0);

    // Las bajas, bando a bando. El atacante son las escuadras de su columna (las apartadas no pierden nada); el resto del mundo, el defensor y lo que las reglas dejan a 0:
    // la escolta de una caravana que cae, el campamento de los residentes de una plaza conquistada, la guarnición de jugadores de una avanzada perdida y la escolta de las
    // adjuntas de una columna derrotada.
    let atacantes: Vec<u64> = a.escuadras.iter().filter(|(_, x)| x.contenedor == Contenedor::Ejercito(columna)).map(|(&id, _)| id).collect();
    let resto: Vec<u64> = a.escuadras.keys().copied().filter(|id| !atacantes.contains(id)).collect();
    // La guarnición de serie que nace en la orden cuenta con los soldados con que nació.
    let nacidas: i64 = despues.iter().filter(|(id, _)| !a.escuadras.contains_key(id)).map(|(_, x)| x.cantidad).sum();
    let (perdida_a, perdida_d) = (perdida(&a.escuadras, &despues, &atacantes), perdida(&a.escuadras, &despues, &resto) + a.nacen - nacidas);
    match combate {
        Some(h) => {
            let bajas_d = h.dato("bajasDefensor") + h.dato("bajasEscolta");
            let a_cero = adjuntas_perdidas || matches!(h.codigo.as_str(), "combate.caravana_interceptada" | "combate.asedio_conquista" | "combate.avanzada_destruida" | "combate.avanzada_en_ruinas" | "combate.avanzada_saqueada");
            v.bajas += (perdida_a != h.dato("bajasAtacante")) as u64;
            v.bajas += if a_cero { perdida_d < bajas_d } else { perdida_d != bajas_d } as u64;
        }
        None => v.bajas += (perdida_a != 0 || perdida_d != 0) as u64,
    }
    if let (ObjetivoAtaque::Ejercito(x), false) = (objetivo, adjuntas_perdidas) {
        let de_un_bando = |c: Contenedor| c == Contenedor::Ejercito(columna) || c == Contenedor::Ejercito(x);
        v.ajenas += a.escuadras.iter().filter(|(id, s)| !de_un_bando(s.contenedor) && despues.get(id) != Some(s)).count() as u64;
    }
    for (id, s) in &a.escuadras {
        let d = despues.get(id);
        v.experiencia += d.is_some_and(|d| d.experiencia < s.experiencia || d.nivel < s.nivel) as u64;
        let perdidos = s.cantidad - d.map_or(0, |d| d.cantidad);
        if perdidos > 0 {
            *bajas_por_tropa.entry(s.tropa).or_insert(0) += perdidos;
        }
    }

    // Un herido no combate: sus escuadras de columna no pierden soldados. Quien queda herido lo está `heridoMinutos`; a quien ya lo estaba no se le alarga.
    let herido = e.reglas.economia.militar.heroe.herido;
    for h in n.heroes() {
        let antes = a.heridos.get(&h.heroe.id).copied().flatten();
        if antes.is_some_and(|x| x > t) {
            v.heridos += (h.heroe.herido_hasta != antes) as u64;
            let perdio = |s: &&militar::escuadra::Escuadra| a.escuadras.get(&s.id).is_some_and(|x| matches!(x.contenedor, Contenedor::Ejercito(_)) && s.cantidad < x.cantidad);
            v.heridos += h.heroe.escuadras.iter().filter(perdio).count() as u64;
        } else if let Some(d) = h.heroe.herido_hasta.filter(|&d| d > t) {
            heridos += 1;
            v.heridos += (d != t + herido) as u64;
        }
    }

    // Nada se crea: ningún recurso crece, salvo la chatarra de las bajas y el oro de los bandidos, que no pasan de lo que dicen los hechos.
    let r = recursos(n);
    let suma = |c: &str| hechos.iter().map(|h| h.dato(c)).sum::<i64>();
    let crece = |rc: Recurso| r[rc.indice()] - a.recursos[rc.indice()];
    let oro_de_bandidos = if combate.is_some_and(|h| h.codigo == "combate.campamento_destruido") { suma("oro") } else { 0 };
    v.conservacion += TODOS.iter().filter(|&&rc| !CHATARRA.contains(&rc) && crece(rc) > if rc == Recurso::Oro { oro_de_bandidos } else { 0 }).count() as u64;
    v.conservacion += (CHATARRA.iter().map(|&c| crece(c)).sum::<i64>() > suma("chatarra")) as u64;

    // El botín es lo que entró en el carro del atacante cuando gana una columna, una caravana o un campamento, y el carro no pasa de su capacidad.
    if let Some(c) = e.ejercitos.get(&columna) {
        let carga: i64 = c.columna.carga.values().sum();
        v.botin += n.capacidad_de_carga(columna).is_some_and(|cap| carga > cap) as u64;
        let cobrado = match combate {
            Some(h) if h.codigo == "combate.resuelto" && h.dato("ganaElAtacante") == 1 => Some(h.dato("botin")),
            Some(h) if h.codigo == "combate.caravana_interceptada" => Some(h.dato("botin")),
            Some(h) if h.codigo == "combate.campamento_destruido" => Some(h.dato("oro")),
            _ => None,
        };
        v.botin += cobrado.is_some_and(|b| carga - a.carga_atacante != b) as u64;
    }

    // La caravana atacada: disputa, la mitad de los carros hacia arriba (sin carros, desaparece); aguante, los mismos pero nunca el último, y se salva el 30 % de la carga
    // (recurso a recurso: una milésima de redondeo por recurso como mucho). Si resiste, no pierde nada.
    if let (ObjetivoAtaque::Caravana(k), Some((carros, carga)), Some(h)) = (objetivo, a.caravana, combate) {
        let ahora = e.comercio.caravana(k).map(|c| (c.carros.len(), c.contenido.values().sum::<i64>()));
        let aguante = h.dato("aguante") == 1;
        let ok = if h.codigo == "combate.caravana_interceptada" {
            let perdidos = carros.div_ceil(2).min(if aguante { carros - 1 } else { carros });
            let carga_ok = |c: i64| if aguante { (c - carga * 3 / 10).abs() <= TODOS.len() as i64 } else { c <= carga };
            h.dato("carros") == perdidos as i64 && h.dato("botin") <= carga && ahora.map_or(!aguante && perdidos == carros, |(q, c)| q == carros - perdidos && carga_ok(c))
        } else {
            ahora == Some((carros, carga))
        };
        v.carros += (!ok) as u64;
    }

    // Un héroe solo no destruye ningún campamento.
    if let (ObjetivoAtaque::Campamento(_), Some(h)) = (objetivo, combate) {
        let soldados: i64 = atacantes.iter().map(|id| a.escuadras[id].cantidad).sum();
        v.heroe_solo += (soldados == 0 && h.codigo == "combate.campamento_destruido") as u64;
    }

    // La conquista: la plaza es del vencedor, protegida y ocupada, sin nadie dentro ni residiendo, sin caravanas aparcadas ni trueques vivos, y cada recinto completo y sin
    // mejora pierde la fracción de sus celdas (la reparación puede haber levantado ya la primera, en el mismo instante).
    if let (ObjetivoAtaque::Asentamiento(p), Some(h)) = (objetivo, combate.filter(|h| h.codigo == "combate.asedio_conquista")) {
        let oc = &e.reglas.economia.combate.ocupacion;
        let (dentro, reside) = (Ubicacion::Asentamiento(p as u32), Some(Residencia::Asentamiento(p as u32)));
        let mut ok = e.asentamientos.get(&p).is_some_and(|x| x.faccion as i64 == h.dato("faccionAtacante") && x.protegida_hasta == Some(t + oc.proteccion) && x.ocupada_hasta == Some(t + oc.duracion));
        ok &= !n.heroes().any(|x| x.heroe.presencia.ubicacion == dentro || x.heroe.residencia == reside);
        ok &= !e.comercio.flota.iter().any(|c| c.estado == EstadoCaravana::Aparcada && c.anfitriona == Some(p as u32));
        ok &= !e.comercio.acuerdos.iter().any(|x| !x.estado.terminado() && (x.a as u64 == p || x.b as u64 == p));
        if let Some(x) = e.asentamientos.get(&p) {
            for (antes, ahora) in a.recintos.iter().zip(&x.economia.recintos) {
                let dano = (antes.celdas.len() as i64 * oc.dano_muralla / UNO) as i32;
                let esperado = if antes.completo() && antes.mejora.is_none() { (antes.avance - dano).max(-1) } else { antes.avance };
                ok &= ahora.avance == esperado || ahora.avance == esperado + 1;
            }
        }
        v.cascada += (!ok) as u64;
    }
    Efecto { v, bajas_por_tropa, heridos, serie: (a.serie, serie(n, objetivo)), hechos }
}
