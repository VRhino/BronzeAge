//! El resultado que manda Unity y su checklist (Batalla_Unity_Ciclo §8.5, §8.6 y §9): cualquier fallo rechaza el resultado **entero**, y lo que se
//! aplica sale de él en una sola pasada. Los tipos son la forma del `BattleResult` del contrato (`battle.proto`), sin los campos que el servidor
//! solo guarda y no interpreta.

use crate::ciclo::{Batalla, CaravanaEnBatalla, Contexto, EscuadraEnBatalla};
use crate::resolucion::Lado;
use base::Mil;
use militar::heroe::Monedas;
use militar::{CaravanaId, EscuadraId, HeroeId};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Razon {
    ObjetivosCapturados,
    Aniquilacion,
    TiempoAgotado,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResultadoEscuadra {
    pub escuadra: EscuadraId,
    pub desplegados: i64,
    pub supervivientes_al_cierre: i64,
    pub muertos: i64,
    /// Puntos de experiencia que dice Unity.
    pub xp_ganada: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjetoBotin {
    pub definicion: String,
    pub cantidad: i64,
    /// Un objeto con identidad propia: tiene que ser nuevo en la partida.
    pub instancia: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Botin {
    pub objetos: Vec<ObjetoBotin>,
    pub monedas: Monedas,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResultadoHeroe {
    pub heroe: HeroeId,
    pub participo: bool,
    pub sobrevivio_al_cierre: bool,
    pub xp_ganada: i64,
    /// Ausente = se decide por el bando perdedor; `Some(false)` protege al perdedor; `Some(true)` hiere aunque su bando ganara.
    pub derrotado: Option<bool>,
    pub botin: Option<Botin>,
}

/// El `BattleResult`. Se guarda **entero** en la batalla: repetir uno idéntico es idempotente, otro distinto se rechaza.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resultado {
    pub resultado_id: String,
    pub revision: u32,
    pub intento: String,
    /// Instantes reales (ms), no de mundo.
    pub inicio: i64,
    pub fin: i64,
    pub ganador: Lado,
    pub razon: Razon,
    pub objetivos: Vec<(String, Option<Lado>)>,
    pub por_escuadra: Vec<ResultadoEscuadra>,
    pub por_heroe: Vec<ResultadoHeroe>,
    pub version_servidor: String,
    /// Asalto de caravana: los carros que Unity da por perdidos, cada uno como `"<caravana>:<posición>"` (el servidor los destruye con su animal y calcula la carga).
    pub carros_perdidos: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorResultado {
    /// Una cifra negativa o una `cantidad` de objeto menor que 1 (lo que el esquema rechaza por forma).
    Forma,
    /// No es el servidor de la asignación activa, o es de otra revisión del ticket.
    AsignacionNoVigente,
    IdRepetido,
    /// `porEscuadra` no cubre exactamente las escuadras de la batalla.
    EscuadrasIncompletas,
    /// `porHeroe` no cubre exactamente a los héroes que siguen combatiendo.
    HeroesIncompletos,
    /// `supervivientes + muertos` no cierra sobre lo autorizado, o `desplegados` lo supera.
    EscuadraNoCierra(EscuadraId),
    /// `tiempo_agotado` y no gana el defensor.
    TiempoAgotadoSinDefensor,
    /// `fin` no es posterior a `inicio` o la partida duró más que `duracionMaximaSegundos`.
    Duracion,
    /// Botín de quien no participó.
    BotinSinParticipar(HeroeId),
    /// Un `itemInstanceId` repetido en el resultado o que ya existe en la partida.
    ObjetoRepetido(String),
    /// Más objetos que casillas libres tenía el héroe en el ticket.
    SinCasillas(HeroeId),
    /// Un carro perdido que la caravana no tiene (o repetido), de una caravana que no está en la batalla, mal escrito, o carros perdidos en una batalla que no es de caravana.
    CarroInexistente(String),
}

/// Lo que el núcleo aplica de un resultado ya validado (Batalla_Unity_Ciclo §9, pasos 1-2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EfectoEscuadra {
    pub escuadra: EscuadraId,
    /// `cantidad = supervivientesAlCierre`.
    pub cantidad: i64,
    /// Experiencia en milésimas de punto (la tropa prestada no gana: lo decide `Escuadra::con_experiencia`).
    pub xp: Mil,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EfectoHeroe {
    pub heroe: HeroeId,
    /// Puntos de experiencia hacia el siguiente nivel; el nivel no sube hasta que Conquest publique su curva (CQ-001).
    pub xp: i64,
    pub botin: Option<Botin>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Efectos {
    pub ganador: Lado,
    /// Heridos: los de `porHeroe` con `derrotado: true`; sin ese campo, los del bando perdedor (Doc 5.16.4).
    pub heridos: Vec<HeroeId>,
    pub escuadras: Vec<EfectoEscuadra>,
    pub heroes: Vec<EfectoHeroe>,
    /// Los carros perdidos: la caravana y la posición en su lista de carros (Unity los nombra por su posición).
    pub carros_perdidos: Vec<(CaravanaId, usize)>,
    /// Todos los candados de la batalla, que se sueltan.
    pub soltar: Vec<EscuadraId>,
}

fn repetidos<T: Ord>(v: impl Iterator<Item = T>) -> bool {
    let mut vistos = BTreeSet::new();
    v.into_iter().any(|x| !vistos.insert(x))
}

fn cerrados<T: Ord + Clone>(a: impl Iterator<Item = T>, b: impl Iterator<Item = T>) -> bool {
    a.collect::<BTreeSet<_>>() == b.collect::<BTreeSet<_>>()
}

impl Batalla {
    /// El checklist: cualquier fallo rechaza el resultado entero. `objeto_ya_existe` dice si un `itemInstanceId` ya está en inventarios y equipos.
    pub fn validar_resultado(&self, servidor: &str, res: &Resultado, objeto_ya_existe: &dyn Fn(&str) -> bool) -> Result<(), ErrorResultado> {
        use ErrorResultado::*;
        // Lo que el esquema valida por forma: enteros >= 0 y objetos con cantidad >= 1.
        let m = |b: &Botin| b.monedas.bronce < 0 || b.monedas.plata < 0 || b.monedas.oro < 0 || b.objetos.iter().any(|o| o.cantidad < 1);
        if res.por_escuadra.iter().any(|e| e.desplegados < 0 || e.supervivientes_al_cierre < 0 || e.muertos < 0 || e.xp_ganada < 0)
            || res.por_heroe.iter().any(|h| h.xp_ganada < 0 || h.botin.as_ref().is_some_and(m))
        {
            return Err(Forma);
        }
        // 1. La asignación activa y la revisión vigente.
        let a = self.asignacion.as_ref();
        if a.is_none_or(|a| a.intento != res.intento || a.servidor != servidor) || res.revision != self.ticket.revision {
            return Err(AsignacionNoVigente);
        }
        // 2. Sin ids repetidos.
        if repetidos(res.por_escuadra.iter().map(|e| e.escuadra)) || repetidos(res.por_heroe.iter().map(|h| h.heroe)) {
            return Err(IdRepetido);
        }
        // 3. Completitud: las escuadras de la batalla (también las nunca desplegadas) y los héroes que siguen combatiendo.
        let escuadras: Vec<&EscuadraEnBatalla> = self.escuadras_de().collect();
        if !cerrados(res.por_escuadra.iter().map(|e| e.escuadra), escuadras.iter().map(|e| e.id)) {
            return Err(EscuadrasIncompletas);
        }
        if !cerrados(res.por_heroe.iter().map(|h| h.heroe), self.heroes_que_combaten()) {
            return Err(HeroesIncompletos);
        }
        // 4. Por escuadra: cierra sobre lo autorizado y no despliega más.
        for e in &res.por_escuadra {
            let autorizados = escuadras.iter().find(|x| x.id == e.escuadra).map_or(0, |x| x.efectivos);
            if e.supervivientes_al_cierre + e.muertos != autorizados || e.desplegados > autorizados {
                return Err(EscuadraNoCierra(e.escuadra));
            }
        }
        // 5. Se agotó el tiempo ⇒ gana el defensor.
        if res.razon == Razon::TiempoAgotado && res.ganador != Lado::Defensor {
            return Err(TiempoAgotadoSinDefensor);
        }
        // 6. Duración.
        if res.fin <= res.inicio || res.fin - res.inicio > self.ticket.duracion_maxima {
            return Err(Duracion);
        }
        // 7. Botín: solo con `participo`, objetos nuevos y que quepan.
        let mut instancias = BTreeSet::new();
        for h in &res.por_heroe {
            let Some(b) = &h.botin else { continue };
            if !h.participo {
                return Err(BotinSinParticipar(h.heroe));
            }
            for o in &b.objetos {
                if let Some(i) = &o.instancia {
                    if !instancias.insert(i.clone()) || objeto_ya_existe(i) {
                        return Err(ObjetoRepetido(i.clone()));
                    }
                }
            }
            let libres = self.participante(h.heroe).map_or(0, |p| p.casillas_libres) as usize;
            if b.objetos.len() > libres {
                return Err(SinCasillas(h.heroe));
            }
        }
        // 8. Carros perdidos (solo en un asalto de caravana): posiciones de la lista, sin repetir.
        self.carros_perdidos(res).map(|_| ())
    }

    /// Los carros que Unity da por perdidos, como (caravana, posición en su lista). Solo valen en una batalla de caravana y han de ser de una caravana en juego, existir y no repetirse.
    fn carros_perdidos(&self, res: &Resultado) -> Result<Vec<(CaravanaId, usize)>, ErrorResultado> {
        let caravanas: &[CaravanaEnBatalla] = match &self.ticket.contexto {
            Contexto::Caravana { caravanas, .. } => caravanas,
            _ => &[],
        };
        let mut perdidos: Vec<(CaravanaId, usize)> = vec![];
        for c in &res.carros_perdidos {
            let carro = c.split_once(':').and_then(|(k, i)| Some((k.parse::<CaravanaId>().ok()?, i.parse::<usize>().ok()?)));
            match carro {
                Some((k, i)) if caravanas.iter().any(|x| x.id == k && i < x.carros) && !perdidos.contains(&(k, i)) => perdidos.push((k, i)),
                _ => return Err(ErrorResultado::CarroInexistente(c.clone())),
            }
        }
        Ok(perdidos)
    }

    /// Lo que hay que aplicar de un resultado **ya validado**.
    pub fn efectos_del_resultado(&self, res: &Resultado) -> Efectos {
        let perdedor = res.ganador.otro();
        let mut heridos: Vec<HeroeId> = res
            .por_heroe
            .iter()
            .filter(|h| h.derrotado.unwrap_or_else(|| self.lado_de(h.heroe) == Some(perdedor)))
            .map(|h| h.heroe)
            .collect();
        heridos.sort_unstable();
        Efectos {
            ganador: res.ganador,
            heridos,
            escuadras: res.por_escuadra.iter().map(|e| EfectoEscuadra { escuadra: e.escuadra, cantidad: e.supervivientes_al_cierre, xp: e.xp_ganada * 1000 }).collect(),
            heroes: res.por_heroe.iter().map(|h| EfectoHeroe { heroe: h.heroe, xp: h.xp_ganada, botin: h.botin.clone() }).collect(),
            carros_perdidos: self.carros_perdidos(res).expect("el resultado ya está validado"),
            soltar: self.escuadras_con_candado(),
        }
    }
}
