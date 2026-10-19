//! La caravana comercial (Comercio_Caravanas §2.3, §5, §6, §7.3): una entidad propia —no un `Ejercito`— con el mismo `base::Plan.
//! Es un contenedor de **carros** (cada uno con a lo sumo un animal) y de **escolta cedida**; de lo que se le monta derivan su
//! capacidad y su velocidad. Este módulo es su máquina de estados: cada transición válida es una función pura sobre la estructura; el
//! núcleo programa los eventos (salida, peaje, llegada) a los instantes que aquí se calculan.
//!
//! Estados: `Disponible` (parada en su origen, sin asignar) · `Preparando` (lanzada a mano, esperando en el origen) · `EnTransito` (hacia
//! el destino) · `Retornando` (de vuelta) · `Aparcada` (dejada en una plaza de su Facción) · `Adjunta` (enganchada a un ejército) ·
//! `EnBatalla` (la resuelve el sistema militar: aquí solo se guarda el estado previo y se aplica el resultado).

use crate::datos::{DefAnimal, DefCarro, ReglasFlota};
use crate::reparto::Candidata;
use crate::tipos::{total, AcuerdoId, AsentamientoId, CaravanaId, Carga, EscuadraId, HeroeId, Lado};
use crate::viaje::{instante_de_progreso, invertir_plan, invertir_ruta, progreso_en, CobroPeaje, Peaje};
use asentamiento::datos::{por, UNO};
use asentamiento::{Almacen, Recurso};
use base::{Instante, Mil, Plan, Punto};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TipoCaravana {
    /// La flota propia (Doc 3.12).
    Comercial,
    /// La Caravana de Fundación (Fundacion_Y_Refundacion): solo datos aquí.
    Construccion,
    Militar,
    Contrabando,
    /// El carro de un pueblo extractor (módulo `acarreo`).
    Acarreo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TipoCarro {
    Basico,
    Reforzado,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Animal {
    Buey,
    Caballo,
    Camello,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Carro {
    pub tipo: TipoCarro,
    pub animal: Option<Animal>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EstadoCaravana {
    Disponible,
    Preparando,
    EnTransito,
    Retornando,
    Aparcada,
    Adjunta,
    EnBatalla,
}

/// Una escuadra cedida como escolta: su id, el héroe que la cedió (la retira solo él) y su coste de Liderazgo por escalón.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Escuadra {
    pub id: EscuadraId,
    pub heroe: HeroeId,
    pub liderazgo: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorCaravana {
    NoEsComercial,
    /// La operación exige la caravana `disponible` (parada en su origen).
    NoDisponible,
    /// La operación exige la caravana `preparando`.
    NoPreparando,
    /// La operación exige la caravana `aparcada`.
    NoAparcada,
    /// La transición pedida no existe en la máquina de estados.
    EstadoInvalido,
    SinMercado,
    /// No han pasado `CARAVANA_COOLDOWN` minutos desde la última caravana creada (comercial o de Fundación).
    EnCooldown,
    CupoDeCaravanas,
    CupoDeCarros,
    /// El carro reforzado exige una Carpintería activa en el origen.
    SinCarpinteria,
    /// El almacén no tiene el coste.
    SinRecursos,
    CarroInexistente,
    CarroYaTieneAnimal,
    /// Las dos caravanas no son distintas o no son del mismo origen.
    OrigenDistinto,
    /// Capacidad 0: ningún carro con animal.
    SinCapacidad,
    /// Carga vacía, negativa o por encima de la capacidad.
    CargaInvalida,
    /// El almacén de origen no tiene lo que se quiere cargar.
    SinStock,
    CupoDeEscolta,
    EscoltaInvalida,
    /// No hay ruta por tierra (el agua es infranqueable).
    SinRuta,
    /// `salirEn` fuera de `(ahora, ahora + maxProgramacionDias]`.
    SalidaInvalida,
    /// Aún no ha llegado (el plan no ha acabado).
    NoHaLlegado,
    /// Sin plan que seguir.
    SinPlan,
    /// Cantidad a mover ≤ 0 (cargar/descargar en una caravana aparcada).
    NadaQueMover,
}

/// Cómo acabó una llegada.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Llegada {
    /// `EnTransito` comercial: hay que **entregar** `contenido` en `destino` (§4.4). La caravana ya queda `Retornando`, vacía y con la
    /// ruta invertida; ha perdido las marcas del trueque.
    Entrega { destino: AsentamientoId, contenido: Carga, origen_acuerdo: Option<(AcuerdoId, Lado)> },
    /// `Retornando`: llegada a casa. Hay que volcar `contenido` (solo si vino de `enviarCaravanaAlOrigen`) en el origen; la caravana
    /// queda `Disponible` y limpia, y la escolta vuelve al campamento.
    Casa { contenido: Carga, escolta: Vec<Escuadra> },
    /// Una caravana no comercial llegó por este camino: se pierde al entregar (la de Fundación y la de acarreo tienen su propio subsistema).
    Perdida { contenido: Carga, escolta: Vec<Escuadra> },
}

/// Lo que ocurrió al aplicar la pérdida de carros que declara el sistema militar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PerdidaCarros {
    pub carros_perdidos: usize,
    pub carga_perdida: Carga,
    /// Se perdieron todos los carros: la caravana se elimina (su escolta vuelve al campamento).
    pub destruida: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Caravana {
    pub id: CaravanaId,
    pub tipo: TipoCaravana,
    /// Su casa para siempre; nunca cambia, tampoco al viajar.
    pub origen: AsentamientoId,
    /// El destino de la entrega. Se conserva durante el regreso. Cuando es una `aparcada`/`adjunta` suelta, `None`.
    pub destino: Option<AsentamientoId>,
    pub contenido: Carga,
    pub carros: Vec<Carro>,
    pub estado: EstadoCaravana,
    pub posicion: Punto,
    /// La polilínea calculada al lanzar (la de ida); el regreso la desanda.
    pub ruta: Vec<Punto>,
    /// Coste de terreno de cada tramo de `ruta` (milésimas de espaciado, como `Camino::costes` de `crates/rutas`): con él se rehace el plan al
    /// salir de la preparación. Vacío si no hay ruta.
    pub costes: Vec<i64>,
    pub plan: Option<Plan>,
    /// Pasos forzados por ciudades ajenas que aún no ha cruzado (solo a la ida).
    pub peajes: Vec<Peaje>,
    /// Solo en el reparto automático: a qué trueque y a qué lado sirve.
    pub origen_acuerdo: Option<(AcuerdoId, Lado)>,
    /// Solo en `Preparando`: el instante en que sale.
    pub prepara_hasta: Option<Instante>,
    pub escolta: Vec<Escuadra>,
    /// `true` ⇒ fuera del reparto automático, sea cual sea su tamaño.
    pub reservada_manual: bool,
    /// La plaza que la hospeda mientras está `Aparcada`.
    pub anfitriona: Option<AsentamientoId>,
    /// Lo que estaba haciendo al entrar en batalla.
    pub antes_de_batalla: Option<EstadoCaravana>,
    /// Instante en que quedó apartada por la batalla (su plan queda en pausa; al salir se desplaza lo que haya durado).
    pub pausada_desde: Option<Instante>,
    /// Nacida en un campamento: no cuenta para `carrosEnUso`.
    pub de_campamento: bool,
}

impl ReglasFlota {
    pub fn carro(&self, t: TipoCarro) -> &DefCarro {
        match t {
            TipoCarro::Basico => &self.basico,
            TipoCarro::Reforzado => &self.reforzado,
        }
    }
    pub fn animal(&self, a: Animal) -> &DefAnimal {
        match a {
            Animal::Buey => &self.buey,
            Animal::Caballo => &self.caballo,
            Animal::Camello => &self.camello,
        }
    }
}

/// ¿Existe la transición `desde → hasta` en la máquina de estados? `EnBatalla` solo se entra desde un estado en el mapa y solo se sale
/// a lo que se estaba haciendo (`EnTransito`, `Retornando` o `Adjunta`).
pub fn transicion_valida(desde: EstadoCaravana, hasta: EstadoCaravana) -> bool {
    use EstadoCaravana::*;
    matches!(
        (desde, hasta),
        (Disponible, Preparando | EnTransito | Adjunta)
            | (Preparando, EnTransito | Disponible)
            | (EnTransito, Retornando | EnBatalla)
            | (Retornando, Disponible | EnBatalla)
            | (Adjunta, Aparcada | Retornando | EnBatalla)
            | (Aparcada, Adjunta | Disponible | Retornando)
            | (EnBatalla, EnTransito | Retornando | Adjunta)
    )
}

/// Lo que hace falta saber para crear una caravana (§5.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContextoCrear {
    pub mercado_activo: bool,
    /// `Asentamiento.ultimaCaravanaCreadaEn`: la marca es compartida con la Caravana de Fundación.
    pub ultima_creada: Option<Instante>,
    /// Caravanas `comercial` con este origen en **cualquier estado**.
    pub existentes: usize,
    pub cupo_caravanas: u32,
}

/// Lo que hace falta saber para fabricar un carro (§5.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContextoCarro {
    pub carpinteria_activa: bool,
    /// `carrosEnUso`: Σ carros de las comerciales de este origen (sin las nacidas en un campamento).
    pub carros_en_uso: usize,
    pub cupo_carros: u32,
}

/// Parámetros de un lanzamiento a mano (§5.6).
#[derive(Clone, Debug)]
pub struct Lanzamiento {
    pub destino: AsentamientoId,
    pub carga: Carga,
    /// Escuadras que se ceden ahora (se suman a las ya cedidas).
    pub escolta_nueva: Vec<Escuadra>,
    pub cupo_escolta: i64,
    /// Hora de salida programada (opcional).
    pub salir_en: Option<Instante>,
    /// `factorCapacidadCaravana` de las políticas del origen (ppm; `UNO` si no hay).
    pub factor_capacidad: i64,
    /// La polilínea trazada (`trazarRutaComercial`), o `None` si no hay camino por tierra.
    pub ruta: Option<Vec<Punto>>,
    pub peajes: Vec<Peaje>,
}

/// Lo que devuelve un lanzamiento: el estado en que queda y cuándo sale (para agendar el evento de salida o el plan de ida).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Preparacion {
    pub estado: EstadoCaravana,
    pub sale_en: Instante,
}

/// Escuadra candidata a escolta con lo que el núcleo sabe de ella (§5.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CandidataEscolta {
    pub escuadra: Escuadra,
    pub es_del_heroe: bool,
    pub en_campamento: bool,
    pub en_guarnicion: bool,
    pub aniquilada: bool,
}

/// `seleccionarEscoltaCaravana`: el héroe reside en el origen y cada escuadra es **suya**, está en su campamento, **no** en la
/// guarnición y no está aniquilada; ≥ 1 y sin repetidos.
pub fn seleccionar_escolta(heroe_reside_en_origen: bool, candidatas: &[CandidataEscolta]) -> Result<Vec<Escuadra>, ErrorCaravana> {
    if !heroe_reside_en_origen || candidatas.is_empty() {
        return Err(ErrorCaravana::EscoltaInvalida);
    }
    let mut v: Vec<Escuadra> = Vec::new();
    for c in candidatas {
        if !c.es_del_heroe || !c.en_campamento || c.en_guarnicion || c.aniquilada || v.iter().any(|e| e.id == c.escuadra.id) {
            return Err(ErrorCaravana::EscoltaInvalida);
        }
        v.push(c.escuadra);
    }
    Ok(v)
}

/// Liderazgo comprometido por unas escuadras (cada una suma su coste por escalón, entera).
pub fn liderazgo_comprometido(escuadras: &[Escuadra]) -> i64 {
    escuadras.iter().map(|e| e.liderazgo).sum()
}

/// `puedeCrearCaravana`: han pasado `cooldownMinutos` desde la última (sin marca previa, sí).
pub fn puede_crear(r: &ReglasFlota, ultima_creada: Option<Instante>, ahora: Instante) -> bool {
    ultima_creada.is_none_or(|u| ahora - u >= r.cooldown_creacion)
}

impl Caravana {
    /// Un casco **vacío y gratis**, `disponible`, en la posición del asentamiento.
    pub fn nueva(id: CaravanaId, origen: AsentamientoId, posicion: Punto) -> Self {
        Self {
            id,
            tipo: TipoCaravana::Comercial,
            origen,
            destino: None,
            contenido: Carga::new(),
            carros: Vec::new(),
            estado: EstadoCaravana::Disponible,
            posicion,
            ruta: Vec::new(),
            costes: Vec::new(),
            plan: None,
            peajes: Vec::new(),
            origen_acuerdo: None,
            prepara_hasta: None,
            escolta: Vec::new(),
            reservada_manual: false,
            anfitriona: None,
            antes_de_batalla: None,
            pausada_desde: None,
            de_campamento: false,
        }
    }

    /// `crearCaravana` (§5.2), en este orden: Mercado activo, cooldown, cupo de caravanas. El núcleo guarda `ultimaCaravanaCreadaEn = ahora`.
    pub fn crear(r: &ReglasFlota, c: &ContextoCrear, ahora: Instante, id: CaravanaId, origen: AsentamientoId, posicion: Punto) -> Result<Self, ErrorCaravana> {
        if !c.mercado_activo {
            return Err(ErrorCaravana::SinMercado);
        }
        if !puede_crear(r, c.ultima_creada, ahora) {
            return Err(ErrorCaravana::EnCooldown);
        }
        if c.existentes >= c.cupo_caravanas as usize {
            return Err(ErrorCaravana::CupoDeCaravanas);
        }
        Ok(Self::nueva(id, origen, posicion))
    }

    // ---------- Derivados (§5.4) ----------

    pub fn carros_con_traccion(&self) -> impl Iterator<Item = (&Carro, Animal)> {
        self.carros.iter().filter_map(|c| c.animal.map(|a| (c, a)))
    }

    /// `capacidadCaravana`: Σ `capacidadBase × factorCarga` de los carros con animal; 0 sin carros con animal. Las categorías sin flota
    /// propia usan `CARAVANA_CATALOGO`.
    pub fn capacidad(&self, r: &ReglasFlota) -> Mil {
        match self.tipo {
            TipoCaravana::Militar => r.militar.capacidad,
            TipoCaravana::Construccion => r.construccion.capacidad,
            TipoCaravana::Contrabando => r.contrabando.capacidad,
            _ => self.carros_con_traccion().map(|(c, a)| por(r.carro(c.tipo).capacidad_base, r.animal(a).factor_carga)).sum(),
        }
    }

    /// `capacidadEfectiva`: la capacidad × `factorCapacidadCaravana` de las políticas del **origen** («Carga Ampliada» ×1,5). Vale para
    /// toda caravana propia (reparto, `prepararCaravana`, adjunta, aparcada). El acarreo no la recibe: quien llama le pasa `UNO`.
    pub fn capacidad_efectiva(&self, r: &ReglasFlota, factor_capacidad: i64) -> Mil {
        por(self.capacidad(r), factor_capacidad)
    }

    /// `velocidadCaravana`: la del animal más lento (unidades por minuto en milésimas, ya con la calibración ×1,5); 0 sin carros con animal.
    pub fn velocidad(&self, r: &ReglasFlota) -> Mil {
        match self.tipo {
            TipoCaravana::Militar => r.militar.velocidad,
            TipoCaravana::Construccion => r.construccion.velocidad,
            TipoCaravana::Contrabando => r.contrabando.velocidad,
            _ => self.carros_con_traccion().map(|(_, a)| r.animal(a).velocidad).min().unwrap_or(0),
        }
    }

    /// Velocidad con «Rutas Rápidas» (`factorVelocidadCaravana` del origen) para una comercial; las demás, sin factor.
    pub fn velocidad_efectiva(&self, r: &ReglasFlota, factor_velocidad: i64) -> Mil {
        if self.tipo == TipoCaravana::Comercial {
            por(self.velocidad(r), factor_velocidad)
        } else {
            self.velocidad(r)
        }
    }

    /// `esCaravanaGrande`: comercial con `≥ VISION.caravanaGrande.carros` carros con animal (para ser vista).
    pub fn es_grande(&self, r: &ReglasFlota) -> bool {
        self.tipo == TipoCaravana::Comercial && self.carros_con_traccion().count() >= r.carros_caravana_grande
    }

    /// Lo que lleva, en total.
    pub fn carga_total(&self) -> Mil {
        total(&self.contenido)
    }

    /// ¿La mira el reparto automático? Comercial, `disponible`, no reservada a mano, de este origen y con capacidad > 0 (las que no
    /// tienen ningún carro con animal no cuentan ni ocupan el turno de las que sí).
    pub fn candidata_de_reparto(&self, r: &ReglasFlota, origen: AsentamientoId, factor_capacidad: i64) -> Option<Candidata> {
        let capacidad = self.capacidad_efectiva(r, factor_capacidad);
        (self.tipo == TipoCaravana::Comercial && self.estado == EstadoCaravana::Disponible && !self.reservada_manual && self.origen == origen && capacidad > 0)
            .then_some(Candidata { caravana: self.id, capacidad })
    }

    // ---------- Composición (§5.3) ----------

    fn exigir_disponible_comercial(&self) -> Result<(), ErrorCaravana> {
        if self.tipo != TipoCaravana::Comercial {
            return Err(ErrorCaravana::NoEsComercial);
        }
        if self.estado != EstadoCaravana::Disponible {
            return Err(ErrorCaravana::NoDisponible);
        }
        Ok(())
    }

    /// La escolta cedida solo se asigna o retira con la caravana parada en su origen; la comercial y el carro de acarreo la admiten.
    fn exigir_disponible_escoltable(&self) -> Result<(), ErrorCaravana> {
        if !matches!(self.tipo, TipoCaravana::Comercial | TipoCaravana::Acarreo) {
            return Err(ErrorCaravana::NoEsComercial);
        }
        if self.estado != EstadoCaravana::Disponible {
            return Err(ErrorCaravana::NoDisponible);
        }
        Ok(())
    }

    /// `agregarCarroCaravana`: caravana comercial `disponible`; la flota no está en su `cupoCarros`; el reforzado exige Carpintería
    /// activa; el almacén tiene el coste (se descuenta). El carro nace **sin animal**.
    pub fn agregar_carro(&mut self, r: &ReglasFlota, tipo: TipoCarro, c: &ContextoCarro, almacen: &mut Almacen) -> Result<(), ErrorCaravana> {
        self.exigir_disponible_comercial()?;
        if c.carros_en_uso >= c.cupo_carros as usize {
            return Err(ErrorCaravana::CupoDeCarros);
        }
        let def = r.carro(tipo);
        if def.exige_carpinteria && !c.carpinteria_activa {
            return Err(ErrorCaravana::SinCarpinteria);
        }
        if !almacen.pagar(&def.costo) {
            return Err(ErrorCaravana::SinRecursos);
        }
        self.carros.push(Carro { tipo, animal: None });
        Ok(())
    }

    /// `comprarAnimalCaravana`: caravana `disponible`; el carro existe y no tiene animal; el almacén tiene el coste (oro).
    pub fn comprar_animal(&mut self, r: &ReglasFlota, indice: usize, animal: Animal, almacen: &mut Almacen) -> Result<(), ErrorCaravana> {
        self.exigir_disponible_comercial()?;
        let carro = self.carros.get(indice).ok_or(ErrorCaravana::CarroInexistente)?;
        if carro.animal.is_some() {
            return Err(ErrorCaravana::CarroYaTieneAnimal);
        }
        if !almacen.pagar(&r.animal(animal).costo) {
            return Err(ErrorCaravana::SinRecursos);
        }
        self.carros[indice].animal = Some(animal);
        Ok(())
    }

    /// `reservarCaravana`: pone o quita `reservadaManual`. No comprueba estado.
    pub fn reservar(&mut self, reservada: bool) {
        self.reservada_manual = reservada;
    }

    // ---------- Escolta cedida (§5.5) ----------

    /// `asignarEscolta`: caravana comercial `disponible`, ≥ 1 escuadra, sin repetidos y dentro del cupo de Liderazgo de la caravana.
    pub fn asignar_escolta(&mut self, nuevas: Vec<Escuadra>, cupo: i64) -> Result<(), ErrorCaravana> {
        self.exigir_disponible_escoltable()?;
        if nuevas.is_empty() || nuevas.iter().enumerate().any(|(i, e)| nuevas[..i].iter().any(|x| x.id == e.id) || self.escolta.iter().any(|x| x.id == e.id)) {
            return Err(ErrorCaravana::EscoltaInvalida);
        }
        if liderazgo_comprometido(&self.escolta) + liderazgo_comprometido(&nuevas) > cupo {
            return Err(ErrorCaravana::CupoDeEscolta);
        }
        self.escolta.extend(nuevas);
        Ok(())
    }

    /// `quitarEscolta`: solo con la caravana `disponible`; cada héroe retira únicamente las suyas (por defecto, todas las suyas). Devuelve
    /// las que vuelven al campamento.
    pub fn quitar_escolta(&mut self, heroe: HeroeId, ids: Option<&[EscuadraId]>) -> Result<Vec<Escuadra>, ErrorCaravana> {
        self.exigir_disponible_escoltable()?;
        if let Some(ids) = ids {
            if ids.iter().any(|i| !self.escolta.iter().any(|e| e.id == *i && e.heroe == heroe)) {
                return Err(ErrorCaravana::EscoltaInvalida);
            }
        }
        let (quitadas, quedan): (Vec<_>, Vec<_>) = std::mem::take(&mut self.escolta).into_iter().partition(|e| e.heroe == heroe && ids.is_none_or(|ids| ids.contains(&e.id)));
        self.escolta = quedan;
        Ok(quitadas)
    }

    // ---------- Lanzar a mano y cancelar (§5.6) ----------

    /// `prepararCaravana`. Validaciones, en este orden (si falla una no se cambia nada): comercial, `disponible`, de este origen;
    /// capacidad > 0; carga > 0, ≤ capacidad y con stock; cupo de escolta; ruta por tierra; `salirEn` en `(ahora, ahora + máx]`. No se
    /// comprueba nada del destino. Efecto: la carga se reserva **ya** del almacén; `saleEn = max(ahora + kPorCarro × (carros − 1), salirEn)`;
    /// con `saleEn > ahora` queda `Preparando` (el evento a `prepara_hasta` la pasa a `EnTransito` con [`Caravana::salir`]); si no,
    /// `EnTransito` y el núcleo le pone el plan con [`Caravana::poner_plan`] (sin evento `caravana_sale`).
    pub fn preparar(&mut self, r: &ReglasFlota, p: Lanzamiento, almacen: &mut Almacen, ahora: Instante) -> Result<Preparacion, ErrorCaravana> {
        self.exigir_disponible_comercial()?;
        let capacidad = self.capacidad_efectiva(r, p.factor_capacidad);
        if capacidad <= 0 {
            return Err(ErrorCaravana::SinCapacidad);
        }
        let suma = total(&p.carga);
        if p.carga.values().any(|&n| n < 0) || suma <= 0 || suma > capacidad {
            return Err(ErrorCaravana::CargaInvalida);
        }
        if p.carga.iter().any(|(&rec, &n)| almacen.cantidad(rec) < n) {
            return Err(ErrorCaravana::SinStock);
        }
        let ya = liderazgo_comprometido(&self.escolta);
        if ya + liderazgo_comprometido(&p.escolta_nueva) > p.cupo_escolta {
            return Err(ErrorCaravana::CupoDeEscolta);
        }
        if p.escolta_nueva.iter().enumerate().any(|(i, e)| p.escolta_nueva[..i].iter().any(|x| x.id == e.id) || self.escolta.iter().any(|x| x.id == e.id)) {
            return Err(ErrorCaravana::EscoltaInvalida);
        }
        let Some(ruta) = p.ruta else { return Err(ErrorCaravana::SinRuta) };
        if let Some(s) = p.salir_en {
            if s <= ahora || s > ahora + r.max_programacion {
                return Err(ErrorCaravana::SalidaInvalida);
            }
        }
        let prep = r.k_preparacion_por_carro * (self.carros.len() as i64 - 1).max(0);
        let sale_en = (ahora + prep).max(p.salir_en.unwrap_or(0));
        let estado = if sale_en > ahora { EstadoCaravana::Preparando } else { EstadoCaravana::EnTransito };
        for (&rec, &n) in &p.carga {
            if n > 0 {
                almacen.quitar(rec, n);
                self.contenido.insert(rec, n);
            }
        }
        self.destino = Some(p.destino);
        self.origen_acuerdo = None;
        self.posicion = ruta.first().copied().unwrap_or(self.posicion);
        self.ruta = ruta;
        self.peajes = p.peajes;
        self.plan = None;
        self.prepara_hasta = (estado == EstadoCaravana::Preparando).then_some(sale_en);
        self.escolta.extend(p.escolta_nueva);
        self.estado = estado;
        Ok(Preparacion { estado, sale_en })
    }

    /// `cancelarCaravana` (solo `Preparando`): la carga vuelve al almacén (topada por su capacidad: lo que no cabe se pierde), la
    /// caravana pasa a `Disponible` sin destino, contenido, ruta ni escolta, y se devuelve la escolta. La ruta ya registrada en la red
    /// **no se retira** (eso es del núcleo).
    pub fn cancelar(&mut self, almacen: &mut Almacen) -> Result<Vec<Escuadra>, ErrorCaravana> {
        if self.estado != EstadoCaravana::Preparando {
            return Err(ErrorCaravana::NoPreparando);
        }
        for (rec, n) in std::mem::take(&mut self.contenido) {
            almacen.agregar(rec, n);
        }
        self.limpiar_viaje();
        self.estado = EstadoCaravana::Disponible;
        Ok(std::mem::take(&mut self.escolta))
    }

    fn limpiar_viaje(&mut self) {
        self.destino = None;
        self.origen_acuerdo = None;
        self.prepara_hasta = None;
        self.ruta.clear();
        self.costes.clear();
        self.peajes.clear();
        self.plan = None;
        self.anfitriona = None;
    }

    pub(crate) fn pasar(&mut self, a: EstadoCaravana) -> Result<(), ErrorCaravana> {
        if !transicion_valida(self.estado, a) {
            return Err(ErrorCaravana::EstadoInvalido);
        }
        self.estado = a;
        Ok(())
    }

    /// El evento de salida de una `Preparando` (a `prepara_hasta`): pasa a `EnTransito` con el plan que construye el núcleo desde ese
    /// instante. Emite `comercio.caravana_sale`.
    pub fn salir(&mut self, ahora: Instante, plan: Plan) -> Result<(), ErrorCaravana> {
        if self.estado != EstadoCaravana::Preparando {
            return Err(ErrorCaravana::NoPreparando);
        }
        if self.prepara_hasta.is_none_or(|t| ahora < t) {
            return Err(ErrorCaravana::NoHaLlegado);
        }
        self.pasar(EstadoCaravana::EnTransito)?;
        self.prepara_hasta = None;
        self.plan = Some(plan);
        Ok(())
    }

    /// El reparto automático la manda (§4.3): una `Disponible` pasa a `EnTransito` con la carga ya sacada del almacén por
    /// [`crate::reparto::asignar_origen`], el trueque y lado a que sirve, la ruta trazada y su plan.
    #[allow(clippy::too_many_arguments)]
    pub fn salir_por_reparto(&mut self, destino: AsentamientoId, carga: Carga, acuerdo: AcuerdoId, lado: Lado, ruta: Vec<Punto>, peajes: Vec<Peaje>, plan: Plan) -> Result<(), ErrorCaravana> {
        self.exigir_disponible_comercial()?;
        self.pasar(EstadoCaravana::EnTransito)?;
        self.destino = Some(destino);
        self.contenido = carga;
        self.origen_acuerdo = Some((acuerdo, lado));
        self.posicion = ruta.first().copied().unwrap_or(self.posicion);
        self.ruta = ruta;
        self.peajes = peajes;
        self.plan = Some(plan);
        Ok(())
    }

    /// Le pone el plan a una `EnTransito` lanzada a mano con salida inmediata (o lo rehace si cambia la velocidad, se construye un
    /// camino en el trayecto o se engancha).
    pub fn poner_plan(&mut self, plan: Plan) -> Result<(), ErrorCaravana> {
        if !matches!(self.estado, EstadoCaravana::EnTransito | EstadoCaravana::Retornando) {
            return Err(ErrorCaravana::EstadoInvalido);
        }
        self.plan = Some(plan);
        Ok(())
    }

    // ---------- El viaje ----------

    /// Posición en `t`: sobre el plan si lo hay, y si no la última conocida.
    pub fn posicion_en(&self, t: Instante) -> Punto {
        self.plan.as_ref().and_then(|p| p.posicion_en(t)).unwrap_or(self.posicion)
    }

    /// Instante del evento de llegada (el último vértice del plan), si va en viaje.
    pub fn llegada_en(&self) -> Option<Instante> {
        matches!(self.estado, EstadoCaravana::EnTransito | EstadoCaravana::Retornando).then(|| self.plan.as_ref().and_then(Plan::fin)).flatten()
    }

    /// Instante del próximo peaje por cruzar (solo a la ida).
    pub fn proximo_peaje(&self) -> Option<Instante> {
        if self.estado != EstadoCaravana::EnTransito {
            return None;
        }
        let plan = self.plan.as_ref()?;
        self.peajes.first().and_then(|p| instante_de_progreso(plan, p.progreso))
    }

    /// Cobra los peajes alcanzados en `ahora` (§7.3, solo a la ida): para cada uno, `PEAJE_PASO.tasa` del contenido **de ese momento**
    /// (ya reducido por los anteriores), solo lo que sea `> 0`; se resta del contenido y se devuelve para que el núcleo lo sume al
    /// almacén de la ciudad (topado). Los peajes alcanzados se descartan se haya cobrado o no. La caravana sigue de largo.
    pub fn cobrar_peajes(&mut self, ahora: Instante, tasa: i64) -> Vec<CobroPeaje> {
        if self.estado != EstadoCaravana::EnTransito {
            return Vec::new();
        }
        let Some(plan) = &self.plan else { return Vec::new() };
        let avance = progreso_en(plan, ahora);
        let (alcanzados, pendientes): (Vec<Peaje>, Vec<Peaje>) = self.peajes.iter().partition(|p| p.progreso <= avance);
        self.peajes = pendientes;
        let mut cobros = Vec::new();
        for pj in alcanzados {
            let mut cobrado = Carga::new();
            for (&rec, n) in self.contenido.iter_mut() {
                let c = (*n as i128 * tasa as i128 / UNO as i128) as Mil;
                if c > 0 {
                    cobrado.insert(rec, c);
                    *n -= c;
                }
            }
            if !cobrado.is_empty() {
                cobros.push(CobroPeaje { ciudad: pj.ciudad, cobrado });
            }
        }
        self.contenido.retain(|_, n| *n > 0);
        cobros
    }

    /// El evento de llegada (el último vértice del plan), en `ahora >= fin del plan`.
    ///
    /// - `EnTransito` comercial ⇒ [`Llegada::Entrega`]: la caravana pasa a `Retornando`, vacía, `progreso 0` en el destino, con la
    ///   **misma ruta invertida** y sin marcas de trueque.
    /// - `Retornando` ⇒ [`Llegada::Casa`]: queda `Disponible`, limpia, en la posición de su origen.
    /// - `EnTransito` de otro tipo ⇒ [`Llegada::Perdida`].
    pub fn llegar(&mut self, ahora: Instante) -> Result<Llegada, ErrorCaravana> {
        if !matches!(self.estado, EstadoCaravana::EnTransito | EstadoCaravana::Retornando) {
            return Err(ErrorCaravana::EstadoInvalido);
        }
        let plan = self.plan.as_ref().ok_or(ErrorCaravana::SinPlan)?;
        let fin = plan.fin().ok_or(ErrorCaravana::SinPlan)?;
        if ahora < fin {
            return Err(ErrorCaravana::NoHaLlegado);
        }
        let aqui = plan.vertices.last().map(|v| v.p).unwrap_or(self.posicion);
        if self.estado == EstadoCaravana::Retornando {
            let contenido = std::mem::take(&mut self.contenido);
            let escolta = std::mem::take(&mut self.escolta);
            self.pasar(EstadoCaravana::Disponible)?;
            self.limpiar_viaje();
            self.posicion = aqui;
            return Ok(Llegada::Casa { contenido, escolta });
        }
        if self.tipo != TipoCaravana::Comercial {
            return Ok(Llegada::Perdida { contenido: std::mem::take(&mut self.contenido), escolta: std::mem::take(&mut self.escolta) });
        }
        let destino = self.destino.ok_or(ErrorCaravana::SinPlan)?;
        let vuelta = invertir_plan(plan, ahora, plan.generacion + 1);
        let contenido = std::mem::take(&mut self.contenido);
        let origen_acuerdo = self.origen_acuerdo.take();
        self.pasar(EstadoCaravana::Retornando)?;
        self.posicion = aqui;
        self.plan = Some(vuelta);
        self.ruta = invertir_ruta(&self.ruta);
        self.costes.reverse();
        self.peajes.clear();
        Ok(Llegada::Entrega { destino, contenido, origen_acuerdo })
    }

    /// §6.3: ¿desaparece esta caravana? Sin destino (Fundación, `disponible`, `adjunta`, `aparcada`) se deja pasar; con destino, si su
    /// origen no existe, o si el destino no existe y no está `retornando`. Sin evento; su escolta vuelve al campamento.
    pub fn desaparece(&self, origen_existe: bool, destino_existe: bool) -> bool {
        self.destino.is_some() && (!origen_existe || (!destino_existe && self.estado != EstadoCaravana::Retornando))
    }

    /// La plaza `ruina` (destruida o conquistada) era el destino: la caravana en camino regresa a su origen **con su carga** por el plan
    /// `calcularRuta(posición actual → origen)` (sin atracción de la red) que construye el núcleo; `destino = ruina` y, al llegar, vuelca
    /// el contenido en el origen (§10).
    pub fn regresar_a_origen(&mut self, ruina: AsentamientoId, plan: Plan) -> Result<(), ErrorCaravana> {
        self.pasar(EstadoCaravana::Retornando)?;
        self.destino = Some(ruina);
        self.origen_acuerdo = None;
        self.peajes.clear();
        self.prepara_hasta = None;
        self.plan = Some(plan);
        Ok(())
    }

    // ---------- Aparcada y adjunta (§5.7) ----------

    /// Se engancha a un ejército (desde `Disponible` o `Aparcada`).
    pub fn adjuntar(&mut self) -> Result<(), ErrorCaravana> {
        self.pasar(EstadoCaravana::Adjunta)?;
        self.anfitriona = None;
        Ok(())
    }

    /// El ejército entra en una plaza de su Facción: la caravana queda aparcada allí, exactamente en su posición.
    pub fn aparcar(&mut self, anfitriona: AsentamientoId, posicion: Punto) -> Result<(), ErrorCaravana> {
        self.pasar(EstadoCaravana::Aparcada)?;
        self.anfitriona = Some(anfitriona);
        self.posicion = posicion;
        Ok(())
    }

    /// El ejército se deshizo sin que nadie lo venciera: la caravana vuelve por su cuenta al origen con el plan dado.
    pub fn volver_por_su_cuenta(&mut self, plan: Plan) -> Result<(), ErrorCaravana> {
        self.pasar(EstadoCaravana::Retornando)?;
        self.plan = Some(plan);
        Ok(())
    }

    /// `moverCargaCaravanaAparcada`, cargar: `min(pedido, capacidadEfectiva − ya cargado, disponible en el almacén)`; ≤ 0 se rechaza.
    /// (Estar presente, ser residente del origen y la posición exacta los comprueba el núcleo.)
    pub fn cargar_aparcada(&mut self, r: &ReglasFlota, factor_capacidad: i64, almacen: &mut Almacen, recurso: Recurso, pedido: Mil) -> Result<Mil, ErrorCaravana> {
        if self.estado != EstadoCaravana::Aparcada {
            return Err(ErrorCaravana::NoAparcada);
        }
        let n = pedido.min(self.capacidad_efectiva(r, factor_capacidad) - self.carga_total()).min(almacen.cantidad(recurso));
        if n <= 0 {
            return Err(ErrorCaravana::NadaQueMover);
        }
        almacen.quitar(recurso, n);
        *self.contenido.entry(recurso).or_insert(0) += n;
        Ok(n)
    }

    /// `moverCargaCaravanaAparcada`, descargar: `min(pedido, lo que lleva, hueco del almacén de ese recurso)`.
    pub fn descargar_aparcada(&mut self, almacen: &mut Almacen, recurso: Recurso, pedido: Mil) -> Result<Mil, ErrorCaravana> {
        if self.estado != EstadoCaravana::Aparcada {
            return Err(ErrorCaravana::NoAparcada);
        }
        let lleva = self.contenido.get(&recurso).copied().unwrap_or(0);
        let n = pedido.min(lleva).min(almacen.espacio(recurso));
        if n <= 0 {
            return Err(ErrorCaravana::NadaQueMover);
        }
        almacen.agregar(recurso, n);
        if n == lleva {
            self.contenido.remove(&recurso);
        } else {
            self.contenido.insert(recurso, lleva - n);
        }
        Ok(n)
    }

    /// `enviarCaravanaAlOrigen`: **vacía** ⇒ `Disponible` al instante en su origen (sin viaje). **Con carga** ⇒ `Retornando` con
    /// `destino` = la anfitriona y el plan dado (`calcularRuta(anfitriona → origen)` sin atracción de la red ni peajes); sin plan (sin ruta
    /// por tierra) ⇒ error.
    pub fn enviar_al_origen(&mut self, posicion_origen: Punto, plan: Option<Plan>) -> Result<(), ErrorCaravana> {
        if self.estado != EstadoCaravana::Aparcada {
            return Err(ErrorCaravana::NoAparcada);
        }
        if self.contenido.is_empty() {
            self.pasar(EstadoCaravana::Disponible)?;
            self.limpiar_viaje();
            self.posicion = posicion_origen;
            return Ok(());
        }
        let plan = plan.ok_or(ErrorCaravana::SinRuta)?;
        self.pasar(EstadoCaravana::Retornando)?;
        self.destino = self.anfitriona.take();
        self.peajes.clear();
        self.plan = Some(plan);
        Ok(())
    }

    // ---------- Batalla (sistema militar) ----------

    /// Entra en batalla (interceptada o atacada): deja el estado previo anotado y espera el resultado del sistema militar.
    pub fn entrar_en_batalla(&mut self) -> Result<(), ErrorCaravana> {
        let antes = self.estado;
        self.pasar(EstadoCaravana::EnBatalla)?;
        self.antes_de_batalla = Some(antes);
        Ok(())
    }

    /// Sale de la batalla al estado que tenía.
    pub fn salir_de_batalla(&mut self) -> Result<(), ErrorCaravana> {
        let antes = self.antes_de_batalla.ok_or(ErrorCaravana::EstadoInvalido)?;
        self.pasar(antes)?;
        self.antes_de_batalla = None;
        Ok(())
    }

    /// Hook del sistema militar (Doc 3.10, «los carros no cambian de manos en el mundo: solo la carga»): quita los carros `perdidos`
    /// (índices; con su animal) y la parte de la carga que viajaba en ellos, **proporcional a su capacidad** sobre la de todos los carros
    /// con animal. Sin carros restantes, la caravana se destruye con toda su carga.
    // DUDA: el documento dice «la carga perdida es la de los carros perdidos» sin fijar cómo se reparte la carga entre carros; se reparte
    // proporcionalmente a su capacidad (floor), y el tope de lo que se lleva el atacante (lo que le quepa) lo aplica el sistema militar.
    pub fn aplicar_perdida_de_carros(&mut self, r: &ReglasFlota, perdidos: &[usize]) -> Result<PerdidaCarros, ErrorCaravana> {
        if perdidos.iter().enumerate().any(|(i, &p)| p >= self.carros.len() || perdidos[..i].contains(&p)) {
            return Err(ErrorCaravana::CarroInexistente);
        }
        let capacidad_de = |c: &Carro| c.animal.map_or(0, |a| por(r.carro(c.tipo).capacidad_base, r.animal(a).factor_carga));
        let cap_total: Mil = self.carros.iter().map(capacidad_de).sum();
        let cap_perdida: Mil = perdidos.iter().map(|&i| capacidad_de(&self.carros[i])).sum();
        let mut perdido = Carga::new();
        let mut indices: Vec<usize> = perdidos.to_vec();
        indices.sort_unstable_by(|a, b| b.cmp(a));
        for i in indices {
            self.carros.remove(i);
        }
        let destruida = self.carros.is_empty();
        if destruida {
            perdido = std::mem::take(&mut self.contenido);
        } else if cap_total > 0 {
            for (&rec, n) in self.contenido.iter_mut() {
                let l = (*n as i128 * cap_perdida as i128 / cap_total as i128) as Mil;
                if l > 0 {
                    perdido.insert(rec, l);
                    *n -= l;
                }
            }
            self.contenido.retain(|_, n| *n > 0);
        }
        Ok(PerdidaCarros { carros_perdidos: perdidos.len(), carga_perdida: perdido, destruida })
    }

    /// Hook del sistema militar para el tipo B (Aguante): el dueño pierde `fraccion` (ppm) de la carga, recurso a recurso, hacia abajo.
    /// Devuelve lo perdido.
    pub fn perder_fraccion_de_carga(&mut self, fraccion: i64) -> Carga {
        let mut perdido = Carga::new();
        for (&rec, n) in self.contenido.iter_mut() {
            let l = (*n as i128 * fraccion.clamp(0, UNO) as i128 / UNO as i128) as Mil;
            if l > 0 {
                perdido.insert(rec, l);
                *n -= l;
            }
        }
        self.contenido.retain(|_, n| *n > 0);
        perdido
    }

    /// Libera la escolta (vuelve al campamento de su héroe).
    pub fn liberar_escolta(&mut self) -> Vec<Escuadra> {
        std::mem::take(&mut self.escolta)
    }
}

/// `carrosEnUso(asentamiento)`: Σ carros de las comerciales de ese origen, sin las nacidas en un campamento.
pub fn carros_en_uso(caravanas: &[Caravana], origen: AsentamientoId) -> usize {
    caravanas.iter().filter(|c| c.tipo == TipoCaravana::Comercial && c.origen == origen && !c.de_campamento).map(|c| c.carros.len()).sum()
}

/// Caravanas `comercial` con ese origen en **cualquier estado** (lo que cuenta contra `cupoCaravanas`).
pub fn caravanas_comerciales_de(caravanas: &[Caravana], origen: AsentamientoId) -> usize {
    caravanas.iter().filter(|c| c.tipo == TipoCaravana::Comercial && c.origen == origen).count()
}

/// `moverCarroCaravana`: las dos comerciales `disponibles`, distintas y **del mismo origen**; el carro (con su animal) pasa de una a otra.
/// Sin coste, tiempo ni comprobación de cupo.
pub fn mover_carro(de: &mut Caravana, a: &mut Caravana, indice: usize) -> Result<(), ErrorCaravana> {
    de.exigir_disponible_comercial()?;
    a.exigir_disponible_comercial()?;
    if de.id == a.id || de.origen != a.origen {
        return Err(ErrorCaravana::OrigenDistinto);
    }
    if indice >= de.carros.len() {
        return Err(ErrorCaravana::CarroInexistente);
    }
    a.carros.push(de.carros.remove(indice));
    Ok(())
}

/// Candidatas del reparto de un origen, en el orden del array de caravanas.
pub fn candidatas_de_reparto(caravanas: &[Caravana], r: &ReglasFlota, origen: AsentamientoId, factor_capacidad: i64) -> Vec<Candidata> {
    caravanas.iter().filter_map(|c| c.candidata_de_reparto(r, origen, factor_capacidad)).collect()
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::datos::Reglas;
    use crate::viaje::{peajes_de_paradas, plan_uniforme};
    use EstadoCaravana::*;

    const MIN: i64 = 60_000;

    fn r() -> ReglasFlota {
        Reglas::cargar().flota
    }

    fn almacen(items: &[(Recurso, Mil)]) -> Almacen {
        let mut a = Almacen::nuevo(10_000_000);
        for &(rec, n) in items {
            a.poner(rec, n);
        }
        a
    }

    fn con(carros: &[(TipoCarro, Option<Animal>)]) -> Caravana {
        let mut c = Caravana::nueva(1, 10, Punto::new(0, 0));
        c.carros = carros.iter().map(|&(tipo, animal)| Carro { tipo, animal }).collect();
        c
    }

    fn buey() -> Caravana {
        con(&[(TipoCarro::Basico, Some(Animal::Buey))])
    }

    fn ruta() -> Vec<Punto> {
        vec![Punto::new(0, 0), Punto::new(60_000, 0), Punto::new(60_000, 80_000)] // 140 unidades
    }

    fn lanz(destino: u32, carga: &[(Recurso, Mil)]) -> Lanzamiento {
        Lanzamiento {
            destino,
            carga: carga.iter().copied().collect(),
            escolta_nueva: vec![],
            cupo_escolta: 100,
            salir_en: None,
            factor_capacidad: UNO,
            ruta: Some(ruta()),
            peajes: vec![],
        }
    }

    // ---- derivados ----

    #[test]
    fn capacidad_y_velocidad_de_un_carro_basico_con_buey() {
        let r = r();
        let c = buey();
        assert_eq!(c.capacidad(&r), 500_000);
        assert_eq!(c.velocidad(&r), 24_000);
        assert_eq!(c.capacidad_efectiva(&r, 1_500_000), 750_000, "Carga Ampliada ×1,5");
        assert_eq!(c.velocidad_efectiva(&r, 1_500_000), 36_000, "Rutas Rápidas ×1,5");
    }

    #[test]
    fn la_capacidad_suma_por_carro_y_la_velocidad_es_la_del_animal_mas_lento() {
        let r = r();
        let c = con(&[(TipoCarro::Basico, Some(Animal::Buey)), (TipoCarro::Reforzado, Some(Animal::Caballo)), (TipoCarro::Basico, Some(Animal::Camello))]);
        assert_eq!(c.capacidad(&r), 500_000 + 400_000 + 375_000);
        assert_eq!(c.velocidad(&r), 24_000, "el buey frena al caballo (36) y al camello (28,5)");
        let rapida = con(&[(TipoCarro::Basico, Some(Animal::Caballo))]);
        assert_eq!((rapida.capacidad(&r), rapida.velocidad(&r)), (250_000, 36_000));
        let camello = con(&[(TipoCarro::Basico, Some(Animal::Camello))]);
        assert_eq!(camello.velocidad(&r), 28_500);
    }

    #[test]
    fn los_carros_sin_animal_no_cuentan() {
        let r = r();
        let c = con(&[(TipoCarro::Basico, None), (TipoCarro::Reforzado, None)]);
        assert_eq!((c.capacidad(&r), c.velocidad(&r)), (0, 0));
        let c = con(&[(TipoCarro::Basico, Some(Animal::Buey)), (TipoCarro::Reforzado, None)]);
        assert_eq!((c.capacidad(&r), c.velocidad(&r)), (500_000, 24_000), "el carro sin tracción no suma ni frena");
        assert_eq!(Caravana::nueva(1, 1, Punto::default()).capacidad(&r), 0, "el casco vacío no carga nada");
    }

    #[test]
    fn es_grande_con_tres_carros_con_animal() {
        let r = r();
        let tres = con(&[(TipoCarro::Basico, Some(Animal::Buey)); 3]);
        assert!(tres.es_grande(&r));
        let dos_y_uno_sin = con(&[(TipoCarro::Basico, Some(Animal::Buey)), (TipoCarro::Basico, Some(Animal::Buey)), (TipoCarro::Basico, None)]);
        assert!(!dos_y_uno_sin.es_grande(&r), "el que no tiene tracción no cuenta");
    }

    // ---- crear y componer ----

    #[test]
    fn crear_exige_mercado_cooldown_y_cupo_en_ese_orden() {
        let r = r();
        let base = ContextoCrear { mercado_activo: true, ultima_creada: None, existentes: 0, cupo_caravanas: 2 };
        let crear = |c: ContextoCrear, t| Caravana::crear(&r, &c, t, 5, 10, Punto::new(1, 2));
        let mut sin = base;
        sin.mercado_activo = false;
        sin.ultima_creada = Some(0);
        sin.existentes = 9;
        assert_eq!(crear(sin, 1).unwrap_err(), ErrorCaravana::SinMercado);
        let mut cd = base;
        cd.ultima_creada = Some(100);
        cd.existentes = 9;
        assert_eq!(crear(cd, 100 + 10 * MIN - 1).unwrap_err(), ErrorCaravana::EnCooldown);
        let mut lleno = base;
        lleno.existentes = 2;
        assert_eq!(crear(lleno, 0).unwrap_err(), ErrorCaravana::CupoDeCaravanas);
        let c = crear(cd, 100 + 10 * MIN).unwrap_err();
        assert_eq!(c, ErrorCaravana::CupoDeCaravanas, "pasado el cooldown ya solo manda el cupo");
        let ok = crear(base, 0).unwrap();
        assert_eq!((ok.estado, ok.carros.len(), ok.posicion, ok.reservada_manual), (Disponible, 0, Punto::new(1, 2), false));
        assert!(ok.contenido.is_empty(), "casco vacío y gratis");
    }

    #[test]
    fn el_cooldown_es_de_diez_minutos_y_sin_marca_previa_se_puede() {
        let r = r();
        assert!(puede_crear(&r, None, 0));
        assert!(!puede_crear(&r, Some(0), 10 * MIN - 1));
        assert!(puede_crear(&r, Some(0), 10 * MIN));
    }

    #[test]
    fn agregar_carro_valida_en_orden_y_cobra() {
        let r = r();
        let mut c = Caravana::nueva(1, 10, Punto::default());
        let ctx = ContextoCarro { carpinteria_activa: false, carros_en_uso: 0, cupo_carros: 3 };
        let mut alm = almacen(&[(Recurso::Madera, 100_000)]);
        c.agregar_carro(&r, TipoCarro::Basico, &ctx, &mut alm).unwrap();
        assert_eq!((c.carros.len(), alm.cantidad(Recurso::Madera)), (1, 80_000), "20 de madera");
        assert_eq!(c.carros[0].animal, None, "nace sin animal");
        assert_eq!(c.agregar_carro(&r, TipoCarro::Reforzado, &ctx, &mut alm), Err(ErrorCaravana::SinCarpinteria));
        let con_carp = ContextoCarro { carpinteria_activa: true, ..ctx };
        c.agregar_carro(&r, TipoCarro::Reforzado, &con_carp, &mut alm).unwrap();
        assert_eq!(alm.cantidad(Recurso::Madera), 40_000);
        let lleno = ContextoCarro { carros_en_uso: 3, ..con_carp };
        assert_eq!(c.agregar_carro(&r, TipoCarro::Basico, &lleno, &mut alm), Err(ErrorCaravana::CupoDeCarros));
        let mut pobre = almacen(&[(Recurso::Madera, 19_999)]);
        assert_eq!(c.agregar_carro(&r, TipoCarro::Basico, &ctx, &mut pobre), Err(ErrorCaravana::SinRecursos));
        assert_eq!(pobre.cantidad(Recurso::Madera), 19_999, "no cobra si falla");
        c.estado = EnTransito;
        assert_eq!(c.agregar_carro(&r, TipoCarro::Basico, &ctx, &mut alm), Err(ErrorCaravana::NoDisponible));
        let mut militar = Caravana::nueva(2, 10, Punto::default());
        militar.tipo = TipoCaravana::Militar;
        assert_eq!(militar.agregar_carro(&r, TipoCarro::Basico, &ctx, &mut alm), Err(ErrorCaravana::NoEsComercial));
    }

    #[test]
    fn comprar_animal_paga_en_oro_y_solo_a_un_carro_libre() {
        let r = r();
        let mut c = con(&[(TipoCarro::Basico, None)]);
        let mut alm = almacen(&[(Recurso::Oro, 100_000)]);
        assert_eq!(c.comprar_animal(&r, 5, Animal::Buey, &mut alm), Err(ErrorCaravana::CarroInexistente));
        c.comprar_animal(&r, 0, Animal::Caballo, &mut alm).unwrap();
        assert_eq!((c.carros[0].animal, alm.cantidad(Recurso::Oro)), (Some(Animal::Caballo), 40_000), "60 de oro");
        assert_eq!(c.comprar_animal(&r, 0, Animal::Buey, &mut alm), Err(ErrorCaravana::CarroYaTieneAnimal));
        let mut c2 = con(&[(TipoCarro::Basico, None)]);
        assert_eq!(c2.comprar_animal(&r, 0, Animal::Caballo, &mut alm), Err(ErrorCaravana::SinRecursos), "quedan 40 y el caballo cuesta 60");
        c2.comprar_animal(&r, 0, Animal::Buey, &mut alm).unwrap();
        assert_eq!(alm.cantidad(Recurso::Oro), 28_000);
    }

    #[test]
    fn mover_carros_entre_caravanas_del_mismo_origen_disponibles() {
        let mut a = con(&[(TipoCarro::Basico, Some(Animal::Buey)), (TipoCarro::Reforzado, None)]);
        let mut b = Caravana { id: 2, ..Caravana::nueva(2, 10, Punto::default()) };
        mover_carro(&mut a, &mut b, 0).unwrap();
        assert_eq!((a.carros.len(), b.carros.len()), (1, 1));
        assert_eq!(b.carros[0].animal, Some(Animal::Buey), "el carro se lleva su animal");
        assert_eq!(mover_carro(&mut a, &mut b, 7), Err(ErrorCaravana::CarroInexistente));
        let mut otro = Caravana { id: 3, ..Caravana::nueva(3, 99, Punto::default()) };
        assert_eq!(mover_carro(&mut a, &mut otro, 0), Err(ErrorCaravana::OrigenDistinto));
        b.estado = Preparando;
        assert_eq!(mover_carro(&mut a, &mut b, 0), Err(ErrorCaravana::NoDisponible));
        let mut gemela = a.clone();
        assert_eq!(mover_carro(&mut a, &mut gemela, 0), Err(ErrorCaravana::OrigenDistinto), "distintas: mismo id no vale");
    }

    #[test]
    fn los_cupos_cuentan_flota_y_carros_de_las_comerciales_de_ese_origen() {
        let mut v = vec![buey(), con(&[(TipoCarro::Basico, None), (TipoCarro::Basico, None)]), Caravana::nueva(3, 10, Punto::default()), Caravana::nueva(4, 77, Punto::default())];
        v[2].estado = EnTransito;
        v[3].carros = vec![Carro { tipo: TipoCarro::Basico, animal: None }];
        let mut campamento = buey();
        campamento.id = 9;
        campamento.de_campamento = true;
        v.push(campamento);
        let mut fundacion = buey();
        fundacion.id = 10;
        fundacion.tipo = TipoCaravana::Construccion;
        v.push(fundacion);
        assert_eq!(caravanas_comerciales_de(&v, 10), 4, "en cualquier estado, también la nacida en un campamento");
        assert_eq!(carros_en_uso(&v, 10), 3, "sin las nacidas en un campamento ni las no comerciales");
    }

    #[test]
    fn el_reparto_no_ve_las_reservadas_ni_las_sin_animal_ni_las_de_otro_origen() {
        let r = r();
        let mut a = buey();
        let sin_animal = Caravana { id: 2, ..con(&[(TipoCarro::Basico, None)]) };
        let mut reservada = Caravana { id: 3, ..buey() };
        reservada.reservar(true);
        let mut ocupada = Caravana { id: 4, ..buey() };
        ocupada.estado = EnTransito;
        let ajena = Caravana { id: 5, origen: 99, ..buey() };
        let buena = Caravana { id: 6, ..buey() };
        a.id = 1;
        let v = vec![a, sin_animal, reservada, ocupada, ajena, buena];
        let c = candidatas_de_reparto(&v, &r, 10, 1_500_000);
        assert_eq!(c.iter().map(|x| x.caravana).collect::<Vec<_>>(), vec![1, 6]);
        assert_eq!(c[0].capacidad, 750_000, "con la política del origen");
    }

    // ---- escolta ----

    fn esc(id: u64, heroe: u64, lid: i64) -> Escuadra {
        Escuadra { id, heroe, liderazgo: lid }
    }

    #[test]
    fn la_escolta_respeta_el_cupo_de_liderazgo_de_la_caravana() {
        let mut c = buey();
        c.asignar_escolta(vec![esc(1, 7, 45), esc(2, 7, 45)], 100).unwrap();
        assert_eq!(c.asignar_escolta(vec![esc(3, 8, 20)], 100), Err(ErrorCaravana::CupoDeEscolta), "90 + 20 > 100");
        c.asignar_escolta(vec![esc(3, 8, 10)], 100).unwrap();
        assert_eq!(c.asignar_escolta(vec![esc(3, 8, 1)], 300), Err(ErrorCaravana::EscoltaInvalida), "repetida");
        assert_eq!(c.asignar_escolta(vec![], 300), Err(ErrorCaravana::EscoltaInvalida));
        assert_eq!(liderazgo_comprometido(&c.escolta), 100);
    }

    #[test]
    fn cada_heroe_retira_solo_las_suyas() {
        let mut c = buey();
        c.asignar_escolta(vec![esc(1, 7, 10), esc(2, 7, 10), esc(3, 8, 10)], 100).unwrap();
        assert_eq!(c.quitar_escolta(7, Some(&[3])), Err(ErrorCaravana::EscoltaInvalida), "la 3 es del héroe 8");
        let quitadas = c.quitar_escolta(7, Some(&[1])).unwrap();
        assert_eq!(quitadas, vec![esc(1, 7, 10)]);
        let resto = c.quitar_escolta(7, None).unwrap();
        assert_eq!(resto, vec![esc(2, 7, 10)], "por defecto, todas las suyas");
        assert_eq!(c.escolta, vec![esc(3, 8, 10)]);
        c.estado = Preparando;
        assert_eq!(c.quitar_escolta(8, None), Err(ErrorCaravana::NoDisponible), "solo con la caravana parada");
    }

    #[test]
    fn seleccionar_escolta_exige_residencia_y_escuadras_validas() {
        let ok = CandidataEscolta { escuadra: esc(1, 7, 10), es_del_heroe: true, en_campamento: true, en_guarnicion: false, aniquilada: false };
        assert_eq!(seleccionar_escolta(true, &[ok]).unwrap(), vec![esc(1, 7, 10)]);
        assert!(seleccionar_escolta(false, &[ok]).is_err(), "no reside en el origen");
        assert!(seleccionar_escolta(true, &[]).is_err());
        for malo in [
            CandidataEscolta { es_del_heroe: false, ..ok },
            CandidataEscolta { en_campamento: false, ..ok },
            CandidataEscolta { en_guarnicion: true, ..ok },
            CandidataEscolta { aniquilada: true, ..ok },
        ] {
            assert!(seleccionar_escolta(true, &[malo]).is_err());
        }
        assert!(seleccionar_escolta(true, &[ok, ok]).is_err(), "sin repetidos");
    }

    // ---- lanzar a mano ----

    #[test]
    fn lanzar_una_caravana_de_un_carro_sale_al_instante_y_reserva_la_carga() {
        let r = r();
        let mut c = buey();
        let mut alm = almacen(&[(Recurso::Madera, 300_000)]);
        let p = c.preparar(&r, lanz(20, &[(Recurso::Madera, 200_000)]), &mut alm, 1_000).unwrap();
        assert_eq!(p, Preparacion { estado: EnTransito, sale_en: 1_000 });
        assert_eq!((c.estado, c.destino, c.prepara_hasta), (EnTransito, Some(20), None));
        assert_eq!((alm.cantidad(Recurso::Madera), c.contenido[&Recurso::Madera]), (100_000, 200_000), "la carga se reserva ya");
        assert_eq!(c.ruta.len(), 3);
        assert!(c.plan.is_none(), "el núcleo le pone el plan desde `sale_en`");
        assert!(c.origen_acuerdo.is_none(), "el lanzamiento manual no se vincula a un trueque");
    }

    #[test]
    fn una_caravana_grande_se_prepara_dos_minutos_por_carro_menos_uno() {
        let r = r();
        let mut c = con(&[(TipoCarro::Basico, Some(Animal::Buey)); 4]);
        let mut alm = almacen(&[(Recurso::Piedra, 900_000)]);
        let p = c.preparar(&r, lanz(20, &[(Recurso::Piedra, 900_000)]), &mut alm, 5_000).unwrap();
        assert_eq!(p, Preparacion { estado: Preparando, sale_en: 5_000 + 6 * MIN }, "2 min × (4 − 1)");
        assert_eq!(c.prepara_hasta, Some(5_000 + 6 * MIN));
        // El carro sin animal también cuenta para el tiempo de preparación.
        let mut d = con(&[(TipoCarro::Basico, Some(Animal::Buey)), (TipoCarro::Basico, None)]);
        let mut alm = almacen(&[(Recurso::Piedra, 900_000)]);
        assert_eq!(d.preparar(&r, lanz(20, &[(Recurso::Piedra, 10_000)]), &mut alm, 0).unwrap().sale_en, 2 * MIN);
    }

    #[test]
    fn la_salida_programada_manda_si_tarda_mas_que_la_preparacion() {
        let r = r();
        let mut c = buey();
        let mut alm = almacen(&[(Recurso::Madera, 300_000)]);
        let mut p = lanz(20, &[(Recurso::Madera, 100_000)]);
        p.salir_en = Some(3 * MIN);
        let pr = c.preparar(&r, p, &mut alm, 0).unwrap();
        assert_eq!((pr.estado, pr.sale_en), (Preparando, 3 * MIN));
        // Y nunca sale antes de estar lista: preparación de 6 min con salida pedida a los 3.
        let mut grande = con(&[(TipoCarro::Basico, Some(Animal::Buey)); 4]);
        let mut p = lanz(20, &[(Recurso::Madera, 100_000)]);
        p.salir_en = Some(3 * MIN);
        assert_eq!(grande.preparar(&r, p, &mut alm, 0).unwrap().sale_en, 6 * MIN);
    }

    #[test]
    fn lanzar_rechaza_en_orden_sin_tocar_nada() {
        let r = r();
        let mut alm = almacen(&[(Recurso::Madera, 300_000)]);
        let antes = alm.clone();
        let mut prueba = |mut c: Caravana, p: Lanzamiento| {
            let copia = c.clone();
            let e = c.preparar(&r, p, &mut alm, 100).unwrap_err();
            assert_eq!(c, copia, "una caravana rechazada no cambia");
            e
        };
        assert_eq!(prueba(Caravana { estado: EnTransito, ..buey() }, lanz(2, &[(Recurso::Madera, 1)])), ErrorCaravana::NoDisponible);
        assert_eq!(prueba(con(&[(TipoCarro::Basico, None)]), lanz(2, &[(Recurso::Madera, 1)])), ErrorCaravana::SinCapacidad);
        assert_eq!(prueba(buey(), lanz(2, &[])), ErrorCaravana::CargaInvalida);
        assert_eq!(prueba(buey(), lanz(2, &[(Recurso::Madera, 500_001)])), ErrorCaravana::CargaInvalida);
        assert_eq!(prueba(buey(), lanz(2, &[(Recurso::Madera, 300_001)])), ErrorCaravana::SinStock);
        let mut esc_mucha = lanz(2, &[(Recurso::Madera, 1)]);
        esc_mucha.escolta_nueva = vec![esc(1, 1, 101)];
        assert_eq!(prueba(buey(), esc_mucha), ErrorCaravana::CupoDeEscolta);
        let mut sin_ruta = lanz(2, &[(Recurso::Madera, 1)]);
        sin_ruta.ruta = None;
        assert_eq!(prueba(buey(), sin_ruta), ErrorCaravana::SinRuta);
        for s in [100, 50, 100 + 3 * 24 * 60 * MIN + 1] {
            let mut p = lanz(2, &[(Recurso::Madera, 1)]);
            p.salir_en = Some(s);
            assert_eq!(prueba(buey(), p), ErrorCaravana::SalidaInvalida, "salirEn {s}");
        }
        assert_eq!(alm, antes);
        // En el límite superior (3 días) sí vale.
        let mut p = lanz(2, &[(Recurso::Madera, 1)]);
        p.salir_en = Some(100 + 3 * 24 * 60 * MIN);
        assert!(buey().preparar(&r, p, &mut alm, 100).is_ok());
    }

    #[test]
    fn carga_ampliada_deja_cargar_mas_de_la_capacidad_base() {
        let r = r();
        let mut alm = almacen(&[(Recurso::Madera, 900_000)]);
        let mut p = lanz(2, &[(Recurso::Madera, 750_000)]);
        assert_eq!(buey().preparar(&r, p.clone(), &mut alm, 0), Err(ErrorCaravana::CargaInvalida));
        p.factor_capacidad = 1_500_000;
        assert!(buey().preparar(&r, p, &mut alm, 0).is_ok());
    }

    #[test]
    fn el_destino_puede_ser_cualquiera_incluso_el_propio_origen() {
        let r = r();
        let mut alm = almacen(&[(Recurso::Madera, 300_000)]);
        let mut c = buey();
        assert!(c.preparar(&r, lanz(10, &[(Recurso::Madera, 1_000)]), &mut alm, 0).is_ok(), "destino == origen: no se comprueba nada del destino");
    }

    #[test]
    fn la_escolta_nueva_se_suma_a_la_ya_cedida() {
        let r = r();
        let mut c = buey();
        c.asignar_escolta(vec![esc(1, 7, 60)], 100).unwrap();
        let mut alm = almacen(&[(Recurso::Madera, 300_000)]);
        let mut p = lanz(2, &[(Recurso::Madera, 1_000)]);
        p.escolta_nueva = vec![esc(2, 7, 50)];
        assert_eq!(c.preparar(&r, p, &mut alm, 0), Err(ErrorCaravana::CupoDeEscolta), "60 + 50 > 100");
        let mut p = lanz(2, &[(Recurso::Madera, 1_000)]);
        p.escolta_nueva = vec![esc(2, 7, 40)];
        c.preparar(&r, p, &mut alm, 0).unwrap();
        assert_eq!(c.escolta.len(), 2);
    }

    #[test]
    fn cancelar_devuelve_la_carga_topada_y_la_escolta() {
        let r = r();
        let mut c = con(&[(TipoCarro::Basico, Some(Animal::Buey)); 2]);
        let mut alm = almacen(&[(Recurso::Madera, 300_000)]);
        let mut p = lanz(2, &[(Recurso::Madera, 200_000)]);
        p.escolta_nueva = vec![esc(1, 7, 30)];
        c.preparar(&r, p, &mut alm, 0).unwrap();
        assert_eq!(c.estado, Preparando);
        // Mientras esperaba, el almacén se llenó: lo que no cabe se pierde.
        alm.poner(Recurso::Madera, 9_900_000);
        let escolta = c.cancelar(&mut alm).unwrap();
        assert_eq!(escolta, vec![esc(1, 7, 30)]);
        assert_eq!(alm.cantidad(Recurso::Madera), 10_000_000, "topado por la capacidad");
        assert_eq!((c.estado, c.destino, c.ruta.len(), c.contenido.is_empty(), c.escolta.is_empty(), c.prepara_hasta), (Disponible, None, 0, true, true, None));
        assert_eq!(c.cancelar(&mut alm), Err(ErrorCaravana::NoPreparando));
    }

    // ---- viaje ----

    fn en_marcha(r: &ReglasFlota, contenido: &[(Recurso, Mil)], peajes: Vec<Peaje>) -> Caravana {
        let mut c = buey();
        let mut alm = almacen(contenido);
        let mut p = lanz(20, contenido);
        p.peajes = peajes;
        c.preparar(r, p, &mut alm, 0).unwrap();
        c.poner_plan(plan_uniforme(&ruta(), 0, c.velocidad(r), 1).unwrap()).unwrap();
        c
    }

    #[test]
    fn la_salida_diferida_pasa_a_en_transito_en_su_instante() {
        let r = r();
        let mut c = con(&[(TipoCarro::Basico, Some(Animal::Buey)); 3]);
        let mut alm = almacen(&[(Recurso::Madera, 100_000)]);
        c.preparar(&r, lanz(20, &[(Recurso::Madera, 100_000)]), &mut alm, 0).unwrap();
        let plan = plan_uniforme(&ruta(), 4 * MIN, 24_000, 1).unwrap();
        assert_eq!(c.salir(4 * MIN - 1, plan.clone()), Err(ErrorCaravana::NoHaLlegado));
        c.salir(4 * MIN, plan).unwrap();
        assert_eq!((c.estado, c.prepara_hasta), (EnTransito, None));
        assert_eq!(c.llegada_en(), Some(4 * MIN + 350_000), "140 unidades a 24 = 5,833 min");
    }

    #[test]
    fn llega_entrega_y_regresa_por_la_misma_ruta_invertida() {
        let r = r();
        let mut c = en_marcha(&r, &[(Recurso::Madera, 200_000)], vec![]);
        let fin = c.llegada_en().unwrap();
        assert_eq!(c.llegar(fin - 1), Err(ErrorCaravana::NoHaLlegado));
        c.origen_acuerdo = Some((9, Lado::A));
        match c.llegar(fin).unwrap() {
            Llegada::Entrega { destino, contenido, origen_acuerdo } => {
                assert_eq!((destino, contenido[&Recurso::Madera], origen_acuerdo), (20, 200_000, Some((9, Lado::A))));
            }
            l => panic!("{l:?}"),
        }
        assert_eq!(c.estado, Retornando);
        assert!(c.contenido.is_empty() && c.origen_acuerdo.is_none(), "pierde las marcas del trueque al entregar");
        assert_eq!(c.posicion, Punto::new(60_000, 80_000), "en el destino");
        assert_eq!(c.destino, Some(20), "el destino ya entregado sigue siendo `destino`");
        assert_eq!(c.ruta[0], Punto::new(60_000, 80_000));
        let plan = c.plan.as_ref().unwrap();
        assert_eq!((plan.inicio(), plan.fin()), (Some(fin), Some(fin + fin)), "mismas duraciones en la vuelta");
        // No se reasigna mientras vuelve: no es candidata hasta llegar a casa.
        assert!(c.candidata_de_reparto(&r, 10, UNO).is_none());
        let vuelta = c.llegada_en().unwrap();
        match c.llegar(vuelta).unwrap() {
            Llegada::Casa { contenido, escolta } => assert!(contenido.is_empty() && escolta.is_empty()),
            l => panic!("{l:?}"),
        }
        assert_eq!((c.estado, c.destino, c.posicion, c.ruta.len()), (Disponible, None, Punto::new(0, 0), 0));
        assert!(c.candidata_de_reparto(&r, 10, UNO).is_some(), "vuelve a ser candidata al llegar a casa");
    }

    #[test]
    fn la_escolta_vuelve_al_campamento_al_terminar_el_regreso() {
        let r = r();
        let mut c = en_marcha(&r, &[(Recurso::Madera, 1_000)], vec![]);
        c.escolta = vec![esc(1, 7, 30)];
        let fin = c.llegada_en().unwrap();
        c.llegar(fin).unwrap();
        assert_eq!(c.escolta.len(), 1, "durante la vuelta sigue cedida");
        match c.llegar(c.llegada_en().unwrap()).unwrap() {
            Llegada::Casa { escolta, .. } => assert_eq!(escolta, vec![esc(1, 7, 30)]),
            l => panic!("{l:?}"),
        }
        assert!(c.escolta.is_empty());
    }

    #[test]
    fn una_caravana_no_comercial_que_llega_se_pierde_al_entregar() {
        let r = r();
        let mut c = en_marcha(&r, &[(Recurso::Madera, 1_000)], vec![]);
        c.tipo = TipoCaravana::Construccion;
        let fin = c.llegada_en().unwrap();
        assert!(matches!(c.llegar(fin).unwrap(), Llegada::Perdida { .. }));
    }

    #[test]
    fn peaje_del_dos_por_ciento_solo_a_la_ida_y_sobre_lo_que_queda() {
        let r = r();
        let tasa = Reglas::cargar().peaje;
        let pj = peajes_de_paradas(&[(7, 60_000), (8, 100_000)], 140_000);
        let mut c = en_marcha(&r, &[(Recurso::Madera, 200_000)], pj);
        let t1 = c.proximo_peaje().unwrap();
        assert_eq!(c.cobrar_peajes(t1 - 1, tasa), vec![], "aún no ha cruzado");
        let cobros = c.cobrar_peajes(t1, tasa);
        assert_eq!(cobros.len(), 1);
        assert_eq!((cobros[0].ciudad, cobros[0].cobrado[&Recurso::Madera]), (7, 4_000), "2 % de 200");
        assert_eq!(c.contenido[&Recurso::Madera], 196_000);
        assert_eq!(c.peajes.len(), 1);
        let t2 = c.proximo_peaje().unwrap();
        let cobros = c.cobrar_peajes(t2, tasa);
        assert_eq!(cobros[0].cobrado[&Recurso::Madera], 3_920, "el segundo peaje es el 2 % de lo que queda, no de lo original");
        assert_eq!(c.contenido[&Recurso::Madera], 192_080);
        assert!(c.peajes.is_empty() && c.proximo_peaje().is_none());
        // Al retornar no hay peajes ni nada que cobrar.
        c.llegar(c.llegada_en().unwrap()).unwrap();
        assert!(c.cobrar_peajes(c.llegada_en().unwrap(), tasa).is_empty());
    }

    #[test]
    fn dos_peajes_alcanzados_a_la_vez_se_cobran_en_orden_y_la_carga_pequena_no_paga() {
        let r = r();
        let tasa = Reglas::cargar().peaje;
        let mut c = en_marcha(&r, &[(Recurso::Madera, 40)], vec![Peaje { ciudad: 1, progreso: 100_000 }, Peaje { ciudad: 2, progreso: 100_000 }]);
        // 2 % de 0,040 = 0,0008 → 0 milésimas: nada que cobrar, pero el peaje se descarta igualmente.
        let t = c.proximo_peaje().unwrap();
        assert!(c.cobrar_peajes(t, tasa).is_empty());
        assert!(c.peajes.is_empty(), "se descartan los alcanzados, se haya cobrado o no");
        assert_eq!(c.contenido[&Recurso::Madera], 40);
        let mut d = en_marcha(&r, &[(Recurso::Madera, 200_000)], vec![Peaje { ciudad: 1, progreso: 100_000 }, Peaje { ciudad: 2, progreso: 100_000 }]);
        let cobros = d.cobrar_peajes(t, tasa);
        assert_eq!(cobros.iter().map(|c| c.ciudad).collect::<Vec<_>>(), vec![1, 2]);
        assert_eq!(d.contenido[&Recurso::Madera], 200_000 - 4_000 - 3_920);
    }

    #[test]
    fn desaparece_solo_con_destino_y_origen_o_destino_inexistentes() {
        let r = r();
        let mut c = en_marcha(&r, &[(Recurso::Madera, 1_000)], vec![]);
        assert!(!c.desaparece(true, true));
        assert!(c.desaparece(false, true));
        assert!(c.desaparece(true, false));
        c.estado = Retornando;
        assert!(!c.desaparece(true, false), "retornando no necesita el destino");
        assert!(c.desaparece(false, true), "pero sí su origen");
        let libre = buey();
        assert!(!libre.desaparece(false, false), "sin destino se deja pasar");
    }

    // ---- aparcada ----

    fn aparcada() -> Caravana {
        let mut c = buey();
        c.adjuntar().unwrap();
        c.aparcar(33, Punto::new(5, 5)).unwrap();
        c
    }

    #[test]
    fn cargar_y_descargar_contra_el_almacen_de_la_anfitriona() {
        let r = r();
        let mut c = aparcada();
        assert_eq!((c.estado, c.anfitriona), (Aparcada, Some(33)));
        let mut alm = almacen(&[(Recurso::Madera, 800_000)]);
        assert_eq!(c.cargar_aparcada(&r, UNO, &mut alm, Recurso::Madera, 600_000), Ok(500_000), "tope: capacidad 500");
        assert_eq!(c.cargar_aparcada(&r, UNO, &mut alm, Recurso::Madera, 10), Err(ErrorCaravana::NadaQueMover), "ya está llena");
        assert_eq!(c.cargar_aparcada(&r, 1_500_000, &mut alm, Recurso::Madera, 600_000), Ok(250_000), "Carga Ampliada vale también aparcada");
        assert_eq!(alm.cantidad(Recurso::Madera), 50_000);
        alm.poner(Recurso::Madera, 9_900_000);
        assert_eq!(c.descargar_aparcada(&mut alm, Recurso::Madera, 750_000), Ok(100_000), "tope: el hueco del almacén");
        assert_eq!(c.contenido[&Recurso::Madera], 650_000);
        assert_eq!(c.descargar_aparcada(&mut alm, Recurso::Piedra, 10), Err(ErrorCaravana::NadaQueMover));
        let mut libre = buey();
        assert_eq!(libre.cargar_aparcada(&r, UNO, &mut alm, Recurso::Madera, 1), Err(ErrorCaravana::NoAparcada));
    }

    #[test]
    fn enviar_al_origen_vacia_es_instantaneo_y_con_carga_viaja() {
        let mut vacia = aparcada();
        vacia.enviar_al_origen(Punto::new(0, 0), None).unwrap();
        assert_eq!((vacia.estado, vacia.posicion, vacia.anfitriona), (Disponible, Punto::new(0, 0), None));
        let mut con_carga = aparcada();
        con_carga.contenido.insert(Recurso::Piedra, 1_000);
        assert_eq!(con_carga.enviar_al_origen(Punto::new(0, 0), None), Err(ErrorCaravana::SinRuta));
        assert_eq!(con_carga.estado, Aparcada, "un error no cambia nada");
        let plan = plan_uniforme(&ruta(), 0, 24_000, 1).unwrap();
        con_carga.enviar_al_origen(Punto::new(0, 0), Some(plan)).unwrap();
        assert_eq!((con_carga.estado, con_carga.destino), (Retornando, Some(33)), "destino = la anfitriona");
        match con_carga.llegar(con_carga.llegada_en().unwrap()).unwrap() {
            Llegada::Casa { contenido, .. } => assert_eq!(contenido[&Recurso::Piedra], 1_000, "vuelca el contenido en el origen"),
            l => panic!("{l:?}"),
        }
    }

    // ---- batalla y pérdidas ----

    #[test]
    fn entrar_y_salir_de_batalla_restaura_el_estado() {
        let r = r();
        let mut c = en_marcha(&r, &[(Recurso::Madera, 1_000)], vec![]);
        c.entrar_en_batalla().unwrap();
        assert_eq!((c.estado, c.antes_de_batalla), (EnBatalla, Some(EnTransito)));
        assert_eq!(c.entrar_en_batalla(), Err(ErrorCaravana::EstadoInvalido));
        c.salir_de_batalla().unwrap();
        assert_eq!((c.estado, c.antes_de_batalla), (EnTransito, None));
        let mut quieta = buey();
        assert_eq!(quieta.entrar_en_batalla(), Err(ErrorCaravana::EstadoInvalido), "una `preparando` o `disponible` está en su ciudad: no es interceptable");
    }

    #[test]
    fn perdida_de_carros_quita_carros_y_su_parte_de_la_carga() {
        let r = r();
        let mut c = con(&[(TipoCarro::Basico, Some(Animal::Buey)), (TipoCarro::Basico, Some(Animal::Buey)), (TipoCarro::Reforzado, None)]);
        c.contenido.insert(Recurso::Madera, 1_000_000);
        let p = c.aplicar_perdida_de_carros(&r, &[1]).unwrap();
        assert_eq!((p.carros_perdidos, p.destruida, p.carga_perdida[&Recurso::Madera]), (1, false, 500_000));
        assert_eq!((c.carros.len(), c.contenido[&Recurso::Madera]), (2, 500_000));
        assert_eq!(c.aplicar_perdida_de_carros(&r, &[5]), Err(ErrorCaravana::CarroInexistente));
        let p = c.aplicar_perdida_de_carros(&r, &[0, 1]).unwrap();
        assert!(p.destruida, "sin carros la caravana se elimina");
        assert_eq!(p.carga_perdida[&Recurso::Madera], 500_000, "con toda su carga");
        assert!(c.contenido.is_empty());
    }

    #[test]
    fn perder_una_fraccion_de_la_carga_recurso_a_recurso() {
        let mut c = buey();
        c.contenido.insert(Recurso::Madera, 1_000_000);
        c.contenido.insert(Recurso::Piedra, 3);
        let perdido = c.perder_fraccion_de_carga(700_000);
        assert_eq!((perdido[&Recurso::Madera], c.contenido[&Recurso::Madera]), (700_000, 300_000), "pierde el 70 %, se salva el 30 %");
        assert_eq!((perdido[&Recurso::Piedra], c.contenido[&Recurso::Piedra]), (2, 1), "hacia abajo: de 2,1 milésimas se pierden 2");
    }

    // ---- la máquina de estados ----

    #[test]
    fn la_tabla_de_transiciones_es_la_del_documento() {
        let todos = [Disponible, Preparando, EnTransito, Retornando, Aparcada, Adjunta, EnBatalla];
        let validas: Vec<_> = todos.iter().flat_map(|&d| todos.iter().filter(move |&&h| transicion_valida(d, h)).map(move |&h| (d, h))).collect();
        assert_eq!(validas.len(), 18);
        // El ciclo completo del documento.
        for (d, h) in [(Disponible, Preparando), (Preparando, EnTransito), (EnTransito, Retornando), (Retornando, Disponible)] {
            assert!(transicion_valida(d, h));
        }
        // Una caravana nunca se teletransporta: de en ruta no se pasa a `Disponible` ni `Aparcada` sin pasar por el regreso.
        assert!(!transicion_valida(EnTransito, Disponible));
        assert!(!transicion_valida(EnTransito, Aparcada));
        assert!(!transicion_valida(Preparando, Retornando));
        assert!(!transicion_valida(Disponible, EnBatalla));
        for e in todos {
            assert!(!transicion_valida(e, e), "no hay transiciones a sí mismo");
        }
    }

    #[test]
    fn el_ejercito_deshecho_hace_volver_a_la_caravana_por_su_cuenta() {
        let mut c = buey();
        c.adjuntar().unwrap();
        c.volver_por_su_cuenta(plan_uniforme(&ruta(), 0, 24_000, 1).unwrap()).unwrap();
        assert_eq!(c.estado, Retornando);
        let mut d = buey();
        assert_eq!(d.volver_por_su_cuenta(plan_uniforme(&ruta(), 0, 24_000, 1).unwrap()), Err(ErrorCaravana::EstadoInvalido));
    }

    #[test]
    fn el_reparto_envia_con_marcas_y_ruta() {
        let r = r();
        let mut c = buey();
        let carga: Carga = [(Recurso::Madera, 100_000)].into_iter().collect();
        let plan = plan_uniforme(&ruta(), 0, c.velocidad(&r), 1).unwrap();
        c.salir_por_reparto(20, carga, 9, Lado::B, ruta(), vec![], plan).unwrap();
        assert_eq!((c.estado, c.destino, c.origen_acuerdo), (EnTransito, Some(20), Some((9, Lado::B))));
        let plan = plan_uniforme(&ruta(), 0, 1, 1).unwrap();
        assert_eq!(c.salir_por_reparto(20, Carga::new(), 9, Lado::B, ruta(), vec![], plan), Err(ErrorCaravana::NoDisponible));
    }
}
