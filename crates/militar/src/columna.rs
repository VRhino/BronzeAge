//! La columna por dentro (Movimiento_Ejercitos §2, §5, §10 y §11; Doc 5.12.1, 5.14): quién va en ella, quién la manda, qué lleva (escuadras,
//! carro y víveres) y cómo se junta y se separa. Es **la misma entidad** para una columna personal y para un ejército; lo que las separa es
//! cómo salió (`tipo`), que **no cambia nunca**.
//!
//! No tiene posición ni plan: eso es del núcleo. Aquí solo la composición y el mando, más el estado (`marchando` o `estacionada`) del que
//! depende la ración. Las escuadras viven en sus héroes; la columna guarda sus ids, y quien llama dice cuántos soldados tiene un conjunto de
//! ids (`soldados_de`) para que el ritmo de los víveres quede siempre bien anotado tras cada cambio.

use crate::datos::Reglas;
use crate::escuadra::Escuadra;
use crate::racion::{self, Estado};
use crate::tipos::{total, Carga, ColumnaId, EscuadraId, FaccionId, HeroeId};
use base::{Instante, Mil, Ritmo};

/// Cuántos soldados hay en estas escuadras (ids). Lo pone quien conoce a los héroes.
pub type Soldados<'a> = &'a dyn Fn(&[EscuadraId]) -> i64;

/// Una columna **personal** la dirige su héroe; un **ejército** lo dirige su Líder. Se fija al salir y no cambia (un ejército reducido a un
/// héroe sigue siendo un ejército).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TipoColumna {
    Personal,
    Ejercito,
}

/// Quién puede sumarse a un ejército. La fija el Líder al salir y no cambia.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Politica {
    Rechazar,
    Aceptar,
    Preguntar,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Participante {
    pub heroe: HeroeId,
    /// Decide la sucesión del mando: el más antiguo manda si el Líder se va.
    pub unido_en: Instante,
}

/// Una petición de unión al Líder (política `Preguntar`): vive 10 s y el silencio vale como «no».
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Peticion {
    pub heroe: HeroeId,
    pub pedido_en: Instante,
    pub expira_en: Instante,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorColumna {
    /// Ya vas en esa columna.
    YaVasEnEsaColumna,
    /// La política `Rechazar` cierra la puerta (también desde la plaza, salvo reforzar el ejército en el que ya vas).
    NoAdmite,
    /// Unirse a un ejército (o formación) pide que lo sea; dos viajeros que se cruzan no forman uno.
    NoHayEjercito,
    /// Solo una columna personal sin formación se une en campo.
    SoloUnaPersonal,
    /// Un ejército lo componen ciudadanos de una sola Facción (y sin Facción no hay compañeros).
    OtraFaccion,
    /// Hay que estar a ≤ `radioEncuentro` para unirse en campo.
    Lejos,
    /// La política de la columna no es `Preguntar`: no hay a quién pedirle permiso.
    NoPregunta,
    NoVaDentro,
    /// De una columna personal no te separas: es tuya.
    DeUnaPersonalNoTeSeparas,
    /// El Líder no se separa: cede antes el mando (salvo en una formación).
    ElLiderNoPuede,
    /// El último no se separa: la columna no se vacía en campo abierto.
    EsElUltimo,
    SoloElLider,
    YaEresElLider,
    /// El sucesor tiene que ir dentro.
    SucesorFuera,
    NoHayPeticion,
    PeticionCaducada,
    /// Un ejército no organiza otro, o ya se está organizando uno.
    YaEsEjercito,
    YaOrganizando,
    /// Solo se organiza un ejército yendo solo.
    NoEstaSolo,
    /// La política de una formación es `Aceptar` o `Preguntar`, nunca `Rechazar`.
    PoliticaInvalida,
    NoEsUnaFormacion,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Columna {
    pub id: ColumnaId,
    /// `None`: una columna sin bandera.
    pub faccion: Option<FaccionId>,
    pub tipo: TipoColumna,
    pub lider: HeroeId,
    pub politica: Politica,
    pub estado: Estado,
    pub participantes: Vec<Participante>,
    pub peticiones: Vec<Peticion>,
    /// Una formación (ejército por formar) caduca a este instante si no llega al mínimo.
    pub formacion_expira_en: Option<Instante>,
    pub escuadras: Vec<EscuadraId>,
    /// El carro: botín y materiales. No es lo que se come.
    pub carga: Carga,
    /// Los víveres del ejército (en una columna personal, los del héroe). Es lo que se come.
    pub viveres: Ritmo,
}

/// La velocidad de una columna: la de su escuadra con soldados más lenta o la de su caravana adjunta más lenta; sin ninguna de las dos, la de
/// un héroe solo si va alguien. Una escuadra aniquilada no frena a nadie. Una escuadra pesada frena a una fuerza ligera, y engancharse una
/// caravana baja la velocidad al mínimo de las dos, así que no se puede escoltar y depredar a la vez.
pub fn velocidad<'a>(r: &Reglas, escuadras: impl IntoIterator<Item = &'a Escuadra>, caravanas: impl IntoIterator<Item = Mil>, participantes: usize) -> Mil {
    let tropas = escuadras.into_iter().filter(|e| e.con_soldados()).map(|e| r.tropas.tropa(e.tropa).velocidad);
    tropas.chain(caravanas).min().unwrap_or(if participantes > 0 { r.movimiento.velocidad_jugador } else { 0 })
}

/// Alcance de vista de una columna: 30 si lleva soldados en pie, 15 si no (un héroe solo, o con escuadras aniquiladas).
pub fn alcance_de_vista<'a>(r: &Reglas, escuadras: impl IntoIterator<Item = &'a Escuadra>) -> Mil {
    if escuadras.into_iter().any(Escuadra::con_soldados) {
        r.vision.ejercito
    } else {
        r.vision.jugador_solo
    }
}

impl Columna {
    /// Sale una columna (`movilizarEjercito` o `salirAlMundo`): un participante, quieta, con las escuadras, el carro y los víveres con que
    /// sale. Una columna personal tiene siempre la política `Rechazar`. Los víveres se topan a la capacidad de un héroe.
    #[allow(clippy::too_many_arguments)]
    pub fn salir(
        r: &Reglas,
        id: ColumnaId,
        faccion: Option<FaccionId>,
        lider: HeroeId,
        tipo: TipoColumna,
        politica: Politica,
        escuadras: Vec<EscuadraId>,
        carga: Carga,
        viveres: Mil,
        ahora: Instante,
        soldados_de: Soldados,
    ) -> Self {
        let mut c = Self {
            id,
            faccion,
            tipo,
            lider,
            politica: if tipo == TipoColumna::Personal { Politica::Rechazar } else { politica },
            estado: Estado::Estacionada,
            participantes: vec![Participante { heroe: lider, unido_en: ahora }],
            peticiones: vec![],
            formacion_expira_en: None,
            escuadras,
            carga,
            viveres: racion::viveres(r, 1, viveres, ahora),
        };
        c.reanclar_consumo(r, ahora, soldados_de);
        c
    }

    pub fn participa(&self, heroe: HeroeId) -> bool {
        self.participantes.iter().any(|p| p.heroe == heroe)
    }

    /// Una columna sin nadie dentro se disuelve; sin soldados no (los héroes siguen yendo a pie).
    pub fn sin_nadie(&self) -> bool {
        self.participantes.is_empty()
    }

    pub fn es_formacion(&self) -> bool {
        self.formacion_expira_en.is_some()
    }

    /// Capacidad de carga: un carro por héroe más la de las caravanas adjuntas (`caravanas`, ya con «Carga Ampliada»).
    pub fn capacidad_de_carga(&self, r: &Reglas, caravanas: Mil) -> Mil {
        self.participantes.len() as i64 * r.logistica.capacidad_carro + caravanas
    }

    /// Anota el consumo de víveres según lo que hay ahora dentro: soldados, héroes y si marcha o está acampada.
    pub fn reanclar_consumo(&mut self, r: &Reglas, ahora: Instante, soldados_de: Soldados) {
        let consumo = racion::racion_columna(r, soldados_de(&self.escuadras), self.participantes.len(), self.estado);
        racion::anotar_consumo(&mut self.viveres, ahora, consumo);
    }

    /// Pasa de marchar a acampar o al revés (`marcharA`, llegar, `estacionarEjercito`) y recalcula el consumo.
    pub fn poner_estado(&mut self, r: &Reglas, ahora: Instante, estado: Estado, soldados_de: Soldados) {
        self.estado = estado;
        self.reanclar_consumo(r, ahora, soldados_de);
    }

    /// Fija los víveres tras un cambio de composición: nueva capacidad (la suma de sus héroes), el `valor` dado y el consumo de ahora.
    fn fijar_viveres(&mut self, r: &Reglas, ahora: Instante, valor: Mil, soldados_de: Soldados) {
        self.viveres = racion::viveres(r, self.participantes.len(), valor, ahora);
        self.reanclar_consumo(r, ahora, soldados_de);
    }

    // ───────────────────────────── Unirse ─────────────────────────────

    /// Las condiciones de unirse en campo (Doc 5.14.1), aparte porque también las comprueba quien solo pide unirse: la columna no admite si
    /// rechaza; es un ejército o una formación; la que se une es personal y sin formación; las dos de la misma Facción (no vacía); y a
    /// ≤ `radioEncuentro` (`distancia`).
    pub fn validar_union_en_campo(&self, r: &Reglas, otra: &Columna, distancia: Mil) -> Result<(), ErrorColumna> {
        if self.id == otra.id {
            return Err(ErrorColumna::YaVasEnEsaColumna);
        }
        if self.politica == Politica::Rechazar {
            return Err(ErrorColumna::NoAdmite);
        }
        if self.tipo != TipoColumna::Ejercito && !self.es_formacion() {
            return Err(ErrorColumna::NoHayEjercito);
        }
        if otra.tipo != TipoColumna::Personal || otra.es_formacion() {
            return Err(ErrorColumna::SoloUnaPersonal);
        }
        if self.faccion.is_none() || self.faccion != otra.faccion {
            return Err(ErrorColumna::OtraFaccion);
        }
        if distancia > r.logistica.radio_encuentro {
            return Err(ErrorColumna::Lejos);
        }
        Ok(())
    }

    /// Unirse en campo: la columna `otra` entra con lo que ya lleva (escuadras, carro y víveres); sus héroes pasan a ser participantes con
    /// `unido_en = ahora` y **adoptan el rumbo del ejército** (eso lo hace el núcleo). Retira las peticiones atendidas. Si es una formación
    /// y llega al mínimo, pasa a ser un ejército (`true`).
    pub fn unir_en_campo(&mut self, r: &Reglas, otra: Columna, distancia: Mil, ahora: Instante, soldados_de: Soldados) -> Result<bool, ErrorColumna> {
        self.validar_union_en_campo(r, &otra, distancia)?;
        let viveres = self.viveres.valor_en(ahora) + otra.viveres.valor_en(ahora);
        self.peticiones.retain(|p| !otra.participa(p.heroe));
        self.participantes.extend(otra.participantes.iter().map(|p| Participante { unido_en: ahora, ..*p }));
        self.escuadras.extend(otra.escuadras);
        sumar_cargas(&mut self.carga, otra.carga);
        self.fijar_viveres(r, ahora, viveres, soldados_de);
        Ok(self.completar_formacion(r))
    }

    /// Unirse pasando el ejército por tu plaza (`unirseAEjercito`, §10.1): el héroe aporta las escuadras que saca de su campamento, su carga
    /// y los víveres con que llena su cupo. Solo a un ejército; la política `Rechazar` cierra la puerta, salvo para reforzar el ejército en
    /// el que ya vas (sumar tropa no te hace dos participantes). La distancia a la puerta y el Liderazgo los comprueba quien llama.
    #[allow(clippy::too_many_arguments)]
    pub fn unirse_en_plaza(&mut self, r: &Reglas, heroe: HeroeId, escuadras: &[EscuadraId], carga: Carga, viveres: Mil, ahora: Instante, soldados_de: Soldados) -> Result<(), ErrorColumna> {
        if self.tipo != TipoColumna::Ejercito || self.es_formacion() {
            return Err(ErrorColumna::NoHayEjercito);
        }
        let ya_dentro = self.participa(heroe);
        if self.politica == Politica::Rechazar && !ya_dentro {
            return Err(ErrorColumna::NoAdmite);
        }
        let viveres = self.viveres.valor_en(ahora) + viveres;
        if !ya_dentro {
            self.participantes.push(Participante { heroe, unido_en: ahora });
        }
        self.escuadras.extend_from_slice(escuadras);
        sumar_cargas(&mut self.carga, carga);
        self.fijar_viveres(r, ahora, viveres, soldados_de);
        Ok(())
    }

    /// Mete a un héroe en la columna sin ninguna condición (andamiaje de administración: `colocarHeroe`): participante nuevo con sus `viveres` y su
    /// capacidad, sin escuadras ni carro. Si ya iba, no cambia nada.
    pub fn admitir(&mut self, r: &Reglas, heroe: HeroeId, viveres: Mil, ahora: Instante, soldados_de: Soldados) {
        if self.participa(heroe) {
            return;
        }
        let viveres = self.viveres.valor_en(ahora) + viveres;
        self.participantes.push(Participante { heroe, unido_en: ahora });
        self.fijar_viveres(r, ahora, viveres, soldados_de);
    }

    /// Anota la petición de unión de la columna `otra` (política `Preguntar`) y devuelve cuándo caduca (`ahora + vidaPeticionUnion`).
    /// Valida la unión **antes** de anotar (no hacerle perder al Líder los diez segundos con algo que se va a rechazar) y barre de paso las
    /// caducadas y la anterior del mismo solicitante.
    pub fn pedir_union(&mut self, r: &Reglas, otra: &Columna, distancia: Mil, ahora: Instante) -> Result<Instante, ErrorColumna> {
        self.validar_union_en_campo(r, otra, distancia)?;
        if self.politica != Politica::Preguntar {
            return Err(ErrorColumna::NoPregunta);
        }
        let expira_en = ahora + r.movimiento.vida_peticion_union;
        self.peticiones.retain(|p| ahora < p.expira_en && p.heroe != otra.lider);
        self.peticiones.push(Peticion { heroe: otra.lider, pedido_en: ahora, expira_en });
        Ok(expira_en)
    }

    /// Retira la petición que el Líder contesta (aceptar o rechazar): solo el Líder, con la petición viva. La caducidad se mira aquí, al
    /// leer: nada la barrió al vencer.
    pub fn retirar_peticion(&mut self, lider: HeroeId, solicitante: HeroeId, ahora: Instante) -> Result<Peticion, ErrorColumna> {
        if self.lider != lider {
            return Err(ErrorColumna::SoloElLider);
        }
        let i = self.peticiones.iter().position(|p| p.heroe == solicitante).ok_or(ErrorColumna::NoHayPeticion)?;
        if ahora >= self.peticiones[i].expira_en {
            return Err(ErrorColumna::PeticionCaducada);
        }
        Ok(self.peticiones.remove(i))
    }

    // ───────────────────────────── Separarse y el mando ─────────────────────────────

    /// Quién manda tras irse (o desconectarse) `saliente`: el participante más antiguo que no sea él y que esté conectado (`conectado`); si ese
    /// también está fuera, el siguiente. Nunca se queda sin candidato mientras quede alguien conectado.
    pub fn sucesor(&self, saliente: HeroeId, conectado: &dyn Fn(HeroeId) -> bool) -> Option<HeroeId> {
        self.participantes.iter().filter(|p| p.heroe != saliente && conectado(p.heroe)).min_by_key(|p| (p.unido_en, p.heroe)).map(|p| p.heroe)
    }

    /// Eleva a otro participante a Líder: el único camino para que el Líder pueda irse.
    pub fn ceder_liderazgo(&mut self, actual: HeroeId, sucesor: HeroeId) -> Result<(), ErrorColumna> {
        if self.lider != actual {
            return Err(ErrorColumna::SoloElLider);
        }
        if sucesor == actual {
            return Err(ErrorColumna::YaEresElLider);
        }
        if !self.participa(sucesor) {
            return Err(ErrorColumna::SucesorFuera);
        }
        self.lider = sucesor;
        Ok(())
    }

    /// `separarseDelEjercito`: sales con lo tuyo y naces como columna personal donde estabas. De una columna personal no te separas; el Líder no
    /// puede (cede antes) salvo en una formación, donde el mando pasa al más antiguo; el último tampoco. `suyas` son tus escuadras en la columna.
    pub fn separarse(&mut self, r: &Reglas, heroe: HeroeId, suyas: &[EscuadraId], id: ColumnaId, ahora: Instante, soldados_de: Soldados) -> Result<Columna, ErrorColumna> {
        if !self.participa(heroe) {
            return Err(ErrorColumna::NoVaDentro);
        }
        if self.tipo != TipoColumna::Ejercito && !self.es_formacion() {
            return Err(ErrorColumna::DeUnaPersonalNoTeSeparas);
        }
        if self.lider == heroe && !self.es_formacion() {
            return Err(ErrorColumna::ElLiderNoPuede);
        }
        if self.participantes.len() <= 1 {
            return Err(ErrorColumna::EsElUltimo);
        }
        let era_lider = self.lider == heroe;
        let nueva = self.desgajar(r, heroe, suyas, id, ahora, soldados_de)?;
        if era_lider {
            self.lider = self.participantes.iter().min_by_key(|p| (p.unido_en, p.heroe)).map(|p| p.heroe).expect("queda alguien");
        }
        Ok(nueva)
    }

    /// Saca a un héroe de la columna con lo suyo y, como mucho, un carro, como columna personal quieta donde estaba. Sin más validación que
    /// ir dentro: `separarse`, desarmar un ejército y la salida de quien se desconecta ponen cada uno sus condiciones. **No hay casa a la que
    /// volver**: la tropa que va con su héroe se queda con su héroe.
    ///
    /// - **Escuadras**: las `suyas`.
    /// - **Carro**: `min(un carro, total / participantes)`, a prorrata por recurso (el resto de milésimas se queda en el ejército).
    /// - **Víveres**: su parte igual: `víveres × su capacidad / la capacidad total`.
    pub fn desgajar(&mut self, r: &Reglas, heroe: HeroeId, suyas: &[EscuadraId], id: ColumnaId, ahora: Instante, soldados_de: Soldados) -> Result<Columna, ErrorColumna> {
        let dentro = *self.participantes.iter().find(|p| p.heroe == heroe).ok_or(ErrorColumna::NoVaDentro)?;
        let n = self.participantes.len() as i64;
        let (en_total, mut carga) = (total(&self.carga), Carga::new());
        if en_total > 0 {
            let se_lleva = r.logistica.capacidad_carro.min(en_total / n);
            for (recurso, cantidad) in self.carga.iter_mut() {
                let parte = (*cantidad as i128 * se_lleva as i128 / en_total as i128) as Mil;
                *cantidad -= parte;
                if parte > 0 {
                    carga.insert(*recurso, parte);
                }
            }
            self.carga.retain(|_, c| *c > 0);
        }
        let viveres = self.viveres.valor_en(ahora);
        let parte = (viveres as i128 * racion::capacidad_viveres(r, 1) as i128 / racion::capacidad_viveres(r, n as usize) as i128) as Mil;
        self.participantes.retain(|p| p.heroe != heroe);
        self.escuadras.retain(|e| !suyas.contains(e));
        self.peticiones.retain(|p| p.heroe != heroe);
        self.fijar_viveres(r, ahora, viveres - parte, soldados_de);
        let mut nueva = Columna {
            id,
            faccion: self.faccion,
            tipo: TipoColumna::Personal,
            lider: heroe,
            politica: Politica::Rechazar,
            estado: Estado::Estacionada,
            participantes: vec![dentro],
            peticiones: vec![],
            formacion_expira_en: None,
            escuadras: suyas.to_vec(),
            carga,
            viveres: racion::viveres(r, 1, parte, ahora),
        };
        nueva.reanclar_consumo(r, ahora, soldados_de);
        Ok(nueva)
    }

    // ───────────────────────────── Formar un ejército en campo (Doc 5.14.4) ─────────────────────────────

    /// `organizarEjercito`: una columna personal sin formación y con **un** participante queda quieta, como Líder fija la política (`Aceptar` o
    /// `Preguntar`) y espera `plazoMinutos` a ser tres. Devuelve cuándo caduca la formación (el núcleo programa el evento).
    pub fn organizar(&mut self, r: &Reglas, politica: Politica, ahora: Instante, soldados_de: Soldados) -> Result<Instante, ErrorColumna> {
        if self.tipo != TipoColumna::Personal {
            return Err(ErrorColumna::YaEsEjercito);
        }
        if self.es_formacion() {
            return Err(ErrorColumna::YaOrganizando);
        }
        if self.participantes.len() != 1 {
            return Err(ErrorColumna::NoEstaSolo);
        }
        if politica == Politica::Rechazar {
            return Err(ErrorColumna::PoliticaInvalida);
        }
        let expira_en = ahora + r.formacion.plazo;
        self.politica = politica;
        self.formacion_expira_en = Some(expira_en);
        self.poner_estado(r, ahora, Estado::Estacionada, soldados_de);
        Ok(expira_en)
    }

    /// Con `formacion.minimo` o más participantes la formación pasa a ser un ejército (quieto hasta que su Líder lo dirija). Si luego baja de
    /// ahí sigue siendo ejército: el mínimo cuenta solo para formarlo. Devuelve si se completó.
    pub fn completar_formacion(&mut self, r: &Reglas) -> bool {
        if self.es_formacion() && self.participantes.len() >= r.formacion.minimo {
            self.formacion_expira_en = None;
            self.tipo = TipoColumna::Ejercito;
            return true;
        }
        false
    }

    /// ¿Venció el plazo de la formación sin llegar al mínimo?
    pub fn formacion_vencida(&self, ahora: Instante) -> bool {
        self.formacion_expira_en.is_some_and(|e| ahora >= e)
    }

    /// Desarma la columna (una formación, o un ejército que va a entrar en un lugar): el Líder se queda con la columna, que vuelve a ser
    /// personal (política `Rechazar`, sin peticiones ni formación), y cada otro nace como columna personal donde estaba, con `desgajar`. Devuelve
    /// esas columnas nuevas. `suyas_de` dice qué escuadras de la columna son de cada héroe; los ids de las nuevas salen de `ids`.
    pub fn desarmar(&mut self, r: &Reglas, ahora: Instante, ids: &mut dyn FnMut() -> ColumnaId, suyas_de: &dyn Fn(HeroeId) -> Vec<EscuadraId>, soldados_de: Soldados) -> Vec<Columna> {
        let otros: Vec<HeroeId> = self.participantes.iter().map(|p| p.heroe).filter(|&h| h != self.lider).collect();
        let sueltas = otros.into_iter().map(|h| self.desgajar(r, h, &suyas_de(h), ids(), ahora, soldados_de).expect("va dentro")).collect();
        self.tipo = TipoColumna::Personal;
        self.politica = Politica::Rechazar;
        self.peticiones.clear();
        self.formacion_expira_en = None;
        sueltas
    }

    /// `cancelarFormacion`: solo el Líder de una formación; la desarma.
    pub fn cancelar_formacion(&mut self, r: &Reglas, lider: HeroeId, ahora: Instante, ids: &mut dyn FnMut() -> ColumnaId, suyas_de: &dyn Fn(HeroeId) -> Vec<EscuadraId>, soldados_de: Soldados) -> Result<Vec<Columna>, ErrorColumna> {
        if !self.es_formacion() {
            return Err(ErrorColumna::NoEsUnaFormacion);
        }
        if self.lider != lider {
            return Err(ErrorColumna::SoloElLider);
        }
        Ok(self.desarmar(r, ahora, ids, suyas_de, soldados_de))
    }
}

fn sumar_cargas(destino: &mut Carga, origen: Carga) {
    for (recurso, cantidad) in origen {
        crate::tipos::sumar(destino, recurso, cantidad);
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::catalogo::Tropa;
    use crate::tipos::sumar;
    use asentamiento::Recurso;
    use base::MS_POR_MINUTO as MIN;
    use std::collections::BTreeMap;

    fn r() -> Reglas {
        Reglas::cargar()
    }

    /// Un mundo de juguete: qué soldados tiene cada escuadra.
    fn soldados(m: &'static [(u64, i64)]) -> impl Fn(&[EscuadraId]) -> i64 {
        move |ids| ids.iter().map(|i| m.iter().find(|(k, _)| k == i).map_or(0, |x| x.1)).sum()
    }

    const TROPA: &[(u64, i64)] = &[(1, 25), (2, 20), (3, 18), (4, 12)];

    fn carga(trigo: Mil, madera: Mil) -> Carga {
        let mut c = Carga::new();
        sumar(&mut c, Recurso::Trigo, trigo);
        sumar(&mut c, Recurso::Madera, madera);
        c
    }
    fn ejercito(lider: HeroeId, escuadras: &[EscuadraId], viveres: Mil, politica: Politica) -> Columna {
        let r = r();
        Columna::salir(&r, 100, Some(1), lider, TipoColumna::Ejercito, politica, escuadras.to_vec(), carga(60_000, 30_000), viveres, 0, &soldados(TROPA))
    }

    fn personal(id: ColumnaId, heroe: HeroeId, escuadras: &[EscuadraId], viveres: Mil) -> Columna {
        let r = r();
        Columna::salir(&r, id, Some(1), heroe, TipoColumna::Personal, Politica::Aceptar, escuadras.to_vec(), carga(0, 0), viveres, 0, &soldados(TROPA))
    }

    // ───────────── salir, ración y capacidades ─────────────

    #[test]
    fn sale_quieta_con_un_participante_y_su_consumo_de_acampada() {
        let c = ejercito(7, &[1, 2], 350_000, Politica::Aceptar);
        assert_eq!((c.tipo, c.lider, c.estado), (TipoColumna::Ejercito, 7, Estado::Estacionada));
        assert_eq!(c.participantes, vec![Participante { heroe: 7, unido_en: 0 }]);
        // (45 soldados × 0,15 + 0,5) × 0,25 × 0,1 = 0,18125 → 181 milésimas por minuto.
        assert_eq!(c.viveres.por_minuto, -181);
        assert_eq!(c.viveres.valor_en(0), 350_000);
        assert_eq!(c.viveres.maximo, Some(350_000));
    }

    #[test]
    fn una_columna_personal_es_siempre_rechazar_aunque_se_pida_otra_politica() {
        let c = personal(5, 9, &[], 100_000);
        assert_eq!(c.politica, Politica::Rechazar);
        assert_eq!(c.capacidad_de_carga(&r(), 0), 500_000);
        assert_eq!(c.capacidad_de_carga(&r(), 500_000), 1_000_000, "un carro más de una caravana adjunta");
    }

    #[test]
    fn marchar_cambia_el_consumo_sin_saltos_en_los_viveres() {
        let r = r();
        let mut c = ejercito(7, &[1, 2], 350_000, Politica::Aceptar);
        c.poner_estado(&r, 10 * MIN, Estado::Marchando, &soldados(TROPA));
        assert_eq!(c.viveres.valor_en(10 * MIN), 350_000 - 1_810, "diez minutos acampada");
        assert_eq!(c.viveres.por_minuto, -1_813, "en marcha es diez veces más: ≈ 1,8 por minuto");
        let fin = racion::se_acaban_en(&c.viveres, 10 * MIN).unwrap();
        assert_eq!(c.viveres.valor_en(fin), 0);
        assert!(c.viveres.valor_en(fin - 1) > 0, "se acaba en el primer instante con 0");
    }

    #[test]
    fn la_velocidad_es_la_de_la_escuadra_mas_lenta_con_soldados() {
        let r = r();
        let e = |t, c| Escuadra { cantidad: c, ..Escuadra::nueva(&r, 1, 1, t, 1, None, 0) };
        let ligera = e(Tropa::Honderos, 20); // 30
        let pesada = e(Tropa::LancerosPesados, 15); // 18
        let montada = e(Tropa::ExploradoresCaballo, 20); // 42
        assert_eq!(velocidad(&r, [&ligera, &montada], [], 1), 30_000);
        assert_eq!(velocidad(&r, [&ligera, &pesada, &montada], [], 2), 18_000, "una pesada frena a la fuerza ligera");
        let aniquilada = Escuadra { cantidad: 0, ..pesada.clone() };
        assert_eq!(velocidad(&r, [&ligera, &aniquilada], [], 1), 30_000, "una aniquilada no frena a nadie");
    }

    #[test]
    fn engancharse_una_caravana_baja_la_velocidad_al_minimo_de_las_dos() {
        let r = r();
        let ligera = Escuadra::nueva(&r, 1, 1, Tropa::Honderos, 20, None, 0);
        assert_eq!(velocidad(&r, [&ligera], [24_000], 1), 24_000, "la ligera (30) con una comercial de buey (24): no se puede escoltar y cazar a la vez");
        let montada = Escuadra::nueva(&r, 2, 1, Tropa::JinetesEscitas, 18, None, 0);
        assert!(velocidad(&r, [&montada], [], 1) > 36_000, "solo la montada (42) alcanza a la de caballo (36)");
        assert!(velocidad(&r, [&ligera], [], 1) < 36_000);
    }

    #[test]
    fn sin_tropa_ni_caravana_va_a_la_velocidad_del_viajero_y_sin_nadie_no_se_mueve() {
        let r = r();
        assert_eq!(velocidad(&r, [], [], 1), 33_000, "más rápido que cualquier tropa porque no arrastra impedimenta");
        assert!(33_000 < r.tropas.tropa(Tropa::ExploradoresCaballo).velocidad && 33_000 > r.tropas.tropa(Tropa::Arqueros).velocidad);
        assert_eq!(velocidad(&r, [], [], 0), 0);
        let vacia = Escuadra { cantidad: 0, ..Escuadra::nueva(&r, 1, 1, Tropa::Arqueros, 1, None, 0) };
        assert_eq!(velocidad(&r, [&vacia], [], 1), 33_000, "perder todos los soldados no basta: los héroes viajan a pie");
    }

    #[test]
    fn el_alcance_de_vista_depende_de_los_soldados_en_pie() {
        let r = r();
        let con = Escuadra::nueva(&r, 1, 1, Tropa::Arqueros, 18, None, 0);
        let sin = Escuadra { cantidad: 0, ..con.clone() };
        assert_eq!(alcance_de_vista(&r, [&con]), 30_000);
        assert_eq!(alcance_de_vista(&r, [&sin]), 15_000, "una escuadra con cantidad 0 no ve más que un hombre solo");
        assert_eq!(alcance_de_vista(&r, []), 15_000);
    }

    // ───────────── unirse ─────────────

    #[test]
    fn unirse_en_campo_aporta_escuadras_carro_y_viveres_y_fecha_la_entrada() {
        let r = r();
        let mut e = ejercito(7, &[1], 200_000, Politica::Aceptar);
        let otra = personal(8, 8, &[2, 3], 300_000);
        assert_eq!(e.unir_en_campo(&r, otra, 3_000, 5 * MIN, &soldados(TROPA)), Ok(false));
        assert_eq!(e.participantes, vec![Participante { heroe: 7, unido_en: 0 }, Participante { heroe: 8, unido_en: 5 * MIN }]);
        assert_eq!(e.escuadras, vec![1, 2, 3]);
        assert_eq!(e.carga[&Recurso::Trigo], 60_000, "la columna que se une no traía carga");
        // Víveres: 200 − 5 min de acampada del ejército + 300, con la capacidad de dos héroes; el consumo, el de 63 soldados y 2 héroes.
        assert_eq!(e.viveres.maximo, Some(700_000));
        assert!(e.viveres.valor_en(5 * MIN) > 490_000 && e.viveres.valor_en(5 * MIN) < 500_000);
        assert_eq!(e.viveres.por_minuto, -racion::racion_columna(&r, 63, 2, Estado::Estacionada));
        assert_eq!(e.tipo, TipoColumna::Ejercito, "un ejército sigue siéndolo");
    }

    #[test]
    fn las_condiciones_de_unirse_en_campo() {
        let r = r();
        let otra = || personal(8, 8, &[2], 100_000);
        let mut rechaza = ejercito(7, &[1], 0, Politica::Rechazar);
        assert_eq!(rechaza.unir_en_campo(&r, otra(), 0, 0, &soldados(TROPA)), Err(ErrorColumna::NoAdmite));
        let mut e = ejercito(7, &[1], 0, Politica::Aceptar);
        assert_eq!(e.validar_union_en_campo(&r, &e.clone(), 0), Err(ErrorColumna::YaVasEnEsaColumna));
        assert_eq!(e.unir_en_campo(&r, otra(), 3_001, 0, &soldados(TROPA)), Err(ErrorColumna::Lejos), "a más de 3");
        assert!(e.validar_union_en_campo(&r, &otra(), 3_000).is_ok(), "justo a 3 sí");
        let mut ajena = otra();
        ajena.faccion = Some(2);
        assert_eq!(e.validar_union_en_campo(&r, &ajena, 0), Err(ErrorColumna::OtraFaccion), "ni siquiera un aliado");
        let mut sin_bandera = otra();
        sin_bandera.faccion = None;
        e.faccion = None;
        assert_eq!(e.validar_union_en_campo(&r, &sin_bandera, 0), Err(ErrorColumna::OtraFaccion), "sin Facción no hay compañeros");
        e.faccion = Some(1);
        let mut otro_ejercito = otra();
        otro_ejercito.tipo = TipoColumna::Ejercito;
        assert_eq!(e.validar_union_en_campo(&r, &otro_ejercito, 0), Err(ErrorColumna::SoloUnaPersonal));
        let mut viajero = personal(9, 9, &[], 0);
        viajero.politica = Politica::Aceptar;
        assert_eq!(viajero.validar_union_en_campo(&r, &otra(), 0), Err(ErrorColumna::NoHayEjercito), "dos viajeros que se cruzan no forman un ejército");
    }

    #[test]
    fn unirse_en_la_plaza_suma_un_participante_o_refuerza_al_que_ya_va() {
        let r = r();
        let mut e = ejercito(7, &[1], 100_000, Politica::Aceptar);
        assert_eq!(e.unirse_en_plaza(&r, 8, &[2], carga(0, 20_000), 350_000, 2 * MIN, &soldados(TROPA)), Ok(()));
        assert_eq!(e.participantes.len(), 2);
        assert_eq!((e.escuadras.clone(), e.carga[&Recurso::Madera]), (vec![1, 2], 50_000));
        assert_eq!(e.viveres.maximo, Some(700_000));
        // Reforzar: el mismo héroe suma tropa sin ser dos participantes.
        assert_eq!(e.unirse_en_plaza(&r, 8, &[3], Carga::new(), 0, 3 * MIN, &soldados(TROPA)), Ok(()));
        assert_eq!((e.participantes.len(), e.escuadras.clone()), (2, vec![1, 2, 3]));
    }

    #[test]
    fn rechazar_cierra_tambien_la_puerta_de_la_plaza_pero_no_a_quien_ya_va() {
        let r = r();
        let mut e = ejercito(7, &[1], 0, Politica::Rechazar);
        assert_eq!(e.unirse_en_plaza(&r, 8, &[2], Carga::new(), 0, 0, &soldados(TROPA)), Err(ErrorColumna::NoAdmite));
        assert_eq!(e.unirse_en_plaza(&r, 7, &[2], Carga::new(), 0, 0, &soldados(TROPA)), Ok(()), "el Líder puede reforzar su propio ejército");
        let mut p = personal(9, 9, &[], 0);
        assert_eq!(p.unirse_en_plaza(&r, 8, &[], Carga::new(), 0, 0, &soldados(TROPA)), Err(ErrorColumna::NoHayEjercito), "una personal no recoge tropas");
    }

    #[test]
    fn la_peticion_vive_diez_segundos_y_el_silencio_es_un_no() {
        let r = r();
        let mut e = ejercito(7, &[1], 0, Politica::Preguntar);
        let sol = personal(8, 8, &[2], 0);
        let expira = e.pedir_union(&r, &sol, 1_000, 100_000).unwrap();
        assert_eq!(expira, 110_000);
        assert_eq!(e.peticiones, vec![Peticion { heroe: 8, pedido_en: 100_000, expira_en: 110_000 }]);
        assert_eq!(e.retirar_peticion(9, 8, 105_000), Err(ErrorColumna::SoloElLider));
        assert_eq!(e.retirar_peticion(7, 99, 105_000), Err(ErrorColumna::NoHayPeticion));
        assert_eq!(e.retirar_peticion(7, 8, 110_000), Err(ErrorColumna::PeticionCaducada), "vence en el instante exacto");
        assert_eq!(e.peticiones.len(), 1, "un rechazo no la retira: la barre quien anote la siguiente");
        assert_eq!(e.retirar_peticion(7, 8, 109_999).map(|p| p.heroe), Ok(8));
        assert!(e.peticiones.is_empty());
    }

    #[test]
    fn pedir_barre_las_caducadas_y_la_anterior_del_mismo_solicitante_y_valida_antes_de_anotar() {
        let r = r();
        let mut e = ejercito(7, &[1], 0, Politica::Preguntar);
        e.pedir_union(&r, &personal(8, 8, &[], 0), 0, 0).unwrap();
        e.pedir_union(&r, &personal(9, 9, &[], 0), 0, 5_000).unwrap();
        e.pedir_union(&r, &personal(10, 10, &[], 0), 0, 12_000).unwrap(); // la del 8 caducó a los 10 s
        assert_eq!(e.peticiones.iter().map(|p| p.heroe).collect::<Vec<_>>(), vec![9, 10]);
        e.pedir_union(&r, &personal(19, 9, &[], 0), 0, 13_000).unwrap(); // la del 9 se renueva
        assert_eq!(e.peticiones.iter().map(|p| (p.heroe, p.pedido_en)).collect::<Vec<_>>(), vec![(10, 12_000), (9, 13_000)]);
        assert_eq!(e.pedir_union(&r, &personal(11, 11, &[], 0), 9_000, 14_000), Err(ErrorColumna::Lejos));
        assert_eq!(e.peticiones.len(), 2, "lo rechazado no se anota");
        let mut acepta = ejercito(7, &[1], 0, Politica::Aceptar);
        assert_eq!(acepta.pedir_union(&r, &personal(8, 8, &[], 0), 0, 0), Err(ErrorColumna::NoPregunta), "con aceptar no hay a quién preguntar");
    }

    #[test]
    fn aceptar_la_peticion_retira_la_de_quien_se_une() {
        let r = r();
        let mut e = ejercito(7, &[1], 0, Politica::Preguntar);
        let sol = personal(8, 8, &[2], 0);
        e.pedir_union(&r, &sol, 0, 0).unwrap();
        e.retirar_peticion(7, 8, 1_000).unwrap();
        e.unir_en_campo(&r, sol, 0, 2_000, &soldados(TROPA)).unwrap();
        assert!(e.peticiones.is_empty() && e.participa(8));
    }

    // ───────────── separarse ─────────────

    fn con_tres(r: &Reglas) -> Columna {
        // 7 (Líder, desde 0), 8 (desde 5), 9 (desde 9); escuadras 1 (de 7), 2 (de 8), 3 y 4 (de 9); 90 trigo y 30 madera; 600 de víveres.
        let mut e = Columna::salir(r, 100, Some(1), 7, TipoColumna::Ejercito, Politica::Aceptar, vec![1], carga(90_000, 30_000), 350_000, 0, &soldados(TROPA));
        e.unirse_en_plaza(r, 8, &[2], Carga::new(), 350_000, 5 * MIN, &soldados(TROPA)).unwrap();
        e.unirse_en_plaza(r, 9, &[3, 4], Carga::new(), 350_000, 9 * MIN, &soldados(TROPA)).unwrap();
        e
    }

    #[test]
    fn separarse_te_lleva_tus_escuadras_un_carro_a_prorrata_y_tu_parte_igual_de_los_viveres() {
        let r = r();
        let mut e = con_tres(&r);
        let antes = e.viveres.valor_en(10 * MIN);
        let nueva = e.separarse(&r, 9, &[3, 4], 200, 10 * MIN, &soldados(TROPA)).unwrap();
        assert_eq!((nueva.tipo, nueva.lider, nueva.politica, nueva.estado), (TipoColumna::Personal, 9, Politica::Rechazar, Estado::Estacionada));
        assert_eq!(nueva.participantes, vec![Participante { heroe: 9, unido_en: 9 * MIN }]);
        assert_eq!((nueva.escuadras.clone(), e.escuadras.clone()), (vec![3, 4], vec![1, 2]));
        // Carro: min(500, 120 / 3 = 40) = 40 de 120, a prorrata: 30 de trigo y 10 de madera.
        assert_eq!((nueva.carga[&Recurso::Trigo], nueva.carga[&Recurso::Madera]), (30_000, 10_000));
        assert_eq!((e.carga[&Recurso::Trigo], e.carga[&Recurso::Madera]), (60_000, 20_000));
        // Víveres: un tercio, con la capacidad de un héroe; el resto se queda con la capacidad de los dos que quedan.
        assert_eq!(nueva.viveres.valor_en(10 * MIN), antes / 3);
        assert_eq!(e.viveres.valor_en(10 * MIN), antes - antes / 3);
        assert_eq!((nueva.viveres.maximo, e.viveres.maximo), (Some(350_000), Some(700_000)));
        // Los consumos reflejan la nueva composición.
        assert_eq!(nueva.viveres.por_minuto, -racion::racion_columna(&r, 30, 1, Estado::Estacionada));
        assert_eq!(e.viveres.por_minuto, -racion::racion_columna(&r, 45, 2, Estado::Estacionada));
    }

    #[test]
    fn el_carro_que_se_lleva_no_pasa_de_uno_aunque_haya_mucho() {
        let r = r();
        let mut e = Columna::salir(&r, 1, Some(1), 7, TipoColumna::Ejercito, Politica::Aceptar, vec![], carga(3_000_000, 0), 0, 0, &soldados(TROPA));
        e.unirse_en_plaza(&r, 8, &[], Carga::new(), 0, 0, &soldados(TROPA)).unwrap();
        let nueva = e.separarse(&r, 8, &[], 2, 0, &soldados(TROPA)).unwrap();
        assert_eq!(nueva.carga[&Recurso::Trigo], 500_000, "un carro como mucho: min(500, 3.000 / 2)");
        assert_eq!(e.carga[&Recurso::Trigo], 2_500_000);
    }

    #[test]
    fn las_condiciones_de_separarse() {
        let r = r();
        let mut e = con_tres(&r);
        assert_eq!(e.separarse(&r, 99, &[], 200, 0, &soldados(TROPA)), Err(ErrorColumna::NoVaDentro));
        assert_eq!(e.separarse(&r, 7, &[1], 200, 0, &soldados(TROPA)), Err(ErrorColumna::ElLiderNoPuede), "el Líder cede antes");
        let mut p = personal(5, 9, &[], 0);
        assert_eq!(p.separarse(&r, 9, &[], 200, 0, &soldados(TROPA)), Err(ErrorColumna::DeUnaPersonalNoTeSeparas));
        let mut solo = ejercito(7, &[1], 0, Politica::Aceptar);
        assert_eq!(solo.separarse(&r, 7, &[1], 200, 0, &soldados(TROPA)), Err(ErrorColumna::ElLiderNoPuede));
        solo.lider = 99;
        assert_eq!(solo.separarse(&r, 7, &[1], 200, 0, &soldados(TROPA)), Err(ErrorColumna::EsElUltimo), "el último no se separa");
        assert_eq!(e.participantes.len(), 3, "los rechazos no dejan rastro");
    }

    #[test]
    fn ceder_el_mando_es_del_lider_a_alguien_que_va_dentro_y_luego_ya_puede_irse() {
        let r = r();
        let mut e = con_tres(&r);
        assert_eq!(e.ceder_liderazgo(8, 9), Err(ErrorColumna::SoloElLider));
        assert_eq!(e.ceder_liderazgo(7, 7), Err(ErrorColumna::YaEresElLider));
        assert_eq!(e.ceder_liderazgo(7, 99), Err(ErrorColumna::SucesorFuera));
        assert_eq!(e.ceder_liderazgo(7, 8), Ok(()));
        assert_eq!(e.lider, 8);
        assert!(e.separarse(&r, 7, &[1], 201, 20 * MIN, &soldados(TROPA)).is_ok(), "ya no es el Líder");
        assert_eq!(e.tipo, TipoColumna::Ejercito, "un ejército al que se separan miembros sigue siéndolo");
    }

    #[test]
    fn el_mando_pasa_al_mas_antiguo_que_este_conectado() {
        let r = r();
        let e = con_tres(&r);
        assert_eq!(e.sucesor(7, &|_| true), Some(8), "el más antiguo tras el Líder");
        assert_eq!(e.sucesor(7, &|h| h != 8), Some(9), "si ese también está fuera, el siguiente");
        assert_eq!(e.sucesor(7, &|_| false), None);
        assert_eq!(e.sucesor(8, &|_| true), Some(7), "el más antiguo de todos, sin contar al que sale");
    }

    #[test]
    fn dos_participantes_con_la_misma_antiguedad_se_deciden_por_id() {
        let r = r();
        let mut e = Columna::salir(&r, 1, Some(1), 7, TipoColumna::Ejercito, Politica::Aceptar, vec![], Carga::new(), 0, 0, &soldados(TROPA));
        e.unirse_en_plaza(&r, 20, &[], Carga::new(), 0, MIN, &soldados(TROPA)).unwrap();
        e.unirse_en_plaza(&r, 11, &[], Carga::new(), 0, MIN, &soldados(TROPA)).unwrap();
        assert_eq!(e.sucesor(7, &|_| true), Some(11));
    }

    // ───────────── formar un ejército en campo ─────────────

    #[test]
    fn organizar_deja_la_columna_quieta_con_un_plazo_de_diez_minutos() {
        let r = r();
        let mut c = personal(5, 9, &[1], 100_000);
        c.poner_estado(&r, 0, Estado::Marchando, &soldados(TROPA));
        assert_eq!(c.organizar(&r, Politica::Preguntar, 3 * MIN, &soldados(TROPA)), Ok(13 * MIN));
        assert_eq!((c.politica, c.estado, c.es_formacion(), c.tipo), (Politica::Preguntar, Estado::Estacionada, true, TipoColumna::Personal));
        assert_eq!(c.viveres.por_minuto, -racion::racion_columna(&r, 25, 1, Estado::Estacionada));
        assert!(!c.formacion_vencida(13 * MIN - 1) && c.formacion_vencida(13 * MIN));
        assert_eq!(c.organizar(&r, Politica::Aceptar, 0, &soldados(TROPA)), Err(ErrorColumna::YaOrganizando));
    }

    #[test]
    fn solo_organiza_una_personal_sola_con_politica_aceptar_o_preguntar() {
        let r = r();
        let mut e = ejercito(7, &[1], 0, Politica::Aceptar);
        assert_eq!(e.organizar(&r, Politica::Aceptar, 0, &soldados(TROPA)), Err(ErrorColumna::YaEsEjercito));
        let mut p = personal(5, 9, &[], 0);
        assert_eq!(p.organizar(&r, Politica::Rechazar, 0, &soldados(TROPA)), Err(ErrorColumna::PoliticaInvalida));
        p.participantes.push(Participante { heroe: 10, unido_en: 0 });
        assert_eq!(p.organizar(&r, Politica::Aceptar, 0, &soldados(TROPA)), Err(ErrorColumna::NoEstaSolo));
        assert!(!p.es_formacion(), "los rechazos no dejan rastro");
    }

    #[test]
    fn al_llegar_a_tres_la_formacion_pasa_a_ser_un_ejercito() {
        let r = r();
        let mut f = personal(5, 9, &[1], 100_000);
        f.organizar(&r, Politica::Aceptar, 0, &soldados(TROPA)).unwrap();
        assert_eq!(f.unir_en_campo(&r, personal(6, 10, &[2], 0), 0, MIN, &soldados(TROPA)), Ok(false));
        assert!(f.es_formacion() && f.tipo == TipoColumna::Personal, "con menos de tres, para el combate sigue siendo personal");
        assert_eq!(f.unir_en_campo(&r, personal(7, 11, &[3], 0), 0, 2 * MIN, &soldados(TROPA)), Ok(true));
        assert_eq!((f.tipo, f.es_formacion(), f.participantes.len()), (TipoColumna::Ejercito, false, 3));
        // Si luego baja de tres sigue siendo ejército.
        let mut f2 = f.clone();
        f2.ceder_liderazgo(9, 10).unwrap();
        f2.separarse(&r, 9, &[1], 300, 3 * MIN, &soldados(TROPA)).unwrap();
        assert_eq!((f2.tipo, f2.participantes.len()), (TipoColumna::Ejercito, 2));
    }

    #[test]
    fn el_lider_de_una_formacion_si_puede_irse_y_el_mando_pasa_al_mas_antiguo() {
        let r = r();
        let mut f = personal(5, 9, &[1], 100_000);
        f.organizar(&r, Politica::Aceptar, 0, &soldados(TROPA)).unwrap();
        f.unir_en_campo(&r, personal(6, 10, &[2], 0), 0, MIN, &soldados(TROPA)).unwrap();
        assert_eq!(f.separarse(&r, 9, &[1], 301, 2 * MIN, &soldados(TROPA)).map(|c| c.lider), Ok(9));
        assert_eq!(f.lider, 10, "el que queda es el más antiguo");
        // La formación sigue (se deshará al vencer), pero de una formación de uno no se sale.
        assert_eq!(f.separarse(&r, 10, &[2], 302, 3 * MIN, &soldados(TROPA)), Err(ErrorColumna::EsElUltimo));
    }

    #[test]
    fn desarmar_deja_al_lider_con_la_columna_personal_y_a_cada_otro_con_lo_suyo() {
        let r = r();
        let mut e = con_tres(&r);
        e.peticiones.push(Peticion { heroe: 50, pedido_en: 0, expira_en: 99 });
        let mut siguiente = 400;
        let suyas: BTreeMap<HeroeId, Vec<EscuadraId>> = [(7, vec![1]), (8, vec![2]), (9, vec![3, 4])].into();
        let nuevas = e.desarmar(&r, 10 * MIN, &mut || { siguiente += 1; siguiente }, &|h| suyas[&h].clone(), &soldados(TROPA));
        assert_eq!(nuevas.iter().map(|c| (c.id, c.lider, c.escuadras.clone())).collect::<Vec<_>>(), vec![(401, 8, vec![2]), (402, 9, vec![3, 4])]);
        assert!(nuevas.iter().all(|c| c.tipo == TipoColumna::Personal && c.politica == Politica::Rechazar && c.estado == Estado::Estacionada));
        assert_eq!((e.tipo, e.politica, e.lider, e.escuadras.clone(), e.participantes.len()), (TipoColumna::Personal, Politica::Rechazar, 7, vec![1], 1));
        assert!(e.peticiones.is_empty() && !e.es_formacion());
        // Reparto: 120 de carro: 8 se lleva min(500, 120/3 = 40); 9 se lleva min(500, 80/2 = 40); el Líder se queda con 40.
        assert_eq!(nuevas.iter().map(|c| total(&c.carga)).collect::<Vec<_>>(), vec![40_000, 40_000]);
        assert_eq!(total(&e.carga), 40_000);
        // Víveres: una parte igual cada uno (a los tres, un tercio del montón; al segundo, la mitad de lo que quedaba).
        let (a, b, l) = (nuevas[0].viveres.valor_en(10 * MIN), nuevas[1].viveres.valor_en(10 * MIN), e.viveres.valor_en(10 * MIN));
        assert!((a - b).abs() <= 1 && (a - l).abs() <= 1, "{a} {b} {l}");
    }

    #[test]
    fn cancelar_la_formacion_es_del_lider_y_de_una_formacion() {
        let r = r();
        let mut f = personal(5, 9, &[1], 0);
        f.organizar(&r, Politica::Aceptar, 0, &soldados(TROPA)).unwrap();
        f.unir_en_campo(&r, personal(6, 10, &[2], 0), 0, MIN, &soldados(TROPA)).unwrap();
        let suyas = |h: HeroeId| if h == 10 { vec![2] } else { vec![1] };
        assert_eq!(f.cancelar_formacion(&r, 10, 2 * MIN, &mut || 500, &suyas, &soldados(TROPA)), Err(ErrorColumna::SoloElLider));
        let sueltas = f.cancelar_formacion(&r, 9, 2 * MIN, &mut || 500, &suyas, &soldados(TROPA)).unwrap();
        assert_eq!((sueltas.len(), sueltas[0].lider, f.es_formacion(), f.participantes.len()), (1, 10, false, 1));
        assert_eq!(f.cancelar_formacion(&r, 9, 3 * MIN, &mut || 501, &suyas, &soldados(TROPA)), Err(ErrorColumna::NoEsUnaFormacion));
    }

    #[test]
    fn una_columna_sin_nadie_se_disuelve_pero_sin_soldados_no() {
        let r = r();
        let mut c = personal(5, 9, &[4], 0);
        assert!(!c.sin_nadie());
        c.escuadras.clear();
        assert!(!c.sin_nadie(), "perder la tropa no basta: los jugadores siguen dentro");
        c.participantes.clear();
        assert!(c.sin_nadie());
        assert_eq!(c.capacidad_de_carga(&r, 0), 0);
    }
}
