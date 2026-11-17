//! Relaciones entre Facciones (Diplomacia_Facciones §2.3, §5; Doc 2.3-2.4.1). Una relación es de un **par**: entre dos Facciones hay como mucho **una activa**
//! de cualquier clase (la guerra cuenta), y las rotas se conservan (la ventana de «guerra tras romper» y la proyección las miran). El almacén indexa la activa
//! de cada par (decisión 5 del plan), así que las consultas de encuentros, defensa, puerta y visión son O(log n).
//!
//! En un vasallaje **A es la señora y B la vasalla**; en una guerra, **A declaró**; la alianza es simétrica. La reputación no vive aquí: las reglas devuelven
//! los [`Suceso`]s de `comercio::reputacion` que la mueven y el núcleo los aplica. Los trueques que una rebelión expira también son del núcleo.

use crate::faccion::{Derrota, Facciones};
use crate::tipos::{FaccionId, RelacionId};
use asentamiento::Recurso;
use base::{Instante, Mil};
use comercio::datos::ReglasReputacion;
use comercio::reputacion::Suceso;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TipoRelacion {
    Vasallaje,
    Alianza,
    Guerra,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EstadoRelacion {
    Activa,
    Rota,
}

/// El tributo de un vasallaje: `cantidad_por_minuto` en milésimas de unidad (contrato `Tributo`), repartido entre las plazas de la vasalla (`tributo`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tributo {
    pub recurso: Recurso,
    pub cantidad_por_minuto: Mil,
}

impl Tributo {
    /// Sin tributo pactado: trigo a 0 por minuto (`proponerRelacion` sin `tributo`).
    pub const POR_DEFECTO: Tributo = Tributo { recurso: Recurso::Trigo, cantidad_por_minuto: 0 };
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelacionPolitica {
    pub id: RelacionId,
    pub tipo: TipoRelacion,
    pub a: FaccionId,
    pub b: FaccionId,
    /// Solo en un vasallaje.
    pub tributo: Option<Tributo>,
    pub creada_en: Instante,
    pub estado: EstadoRelacion,
    /// Solo en una alianza que alguien rompió con `romperRelacion` (o con la traición, Q2).
    pub rota_por: Option<FaccionId>,
    pub rota_en: Option<Instante>,
    /// En una guerra, la primera Facción que ofreció la paz.
    pub paz_propuesta_por: Option<FaccionId>,
}

impl RelacionPolitica {
    pub fn activa(&self) -> bool {
        self.estado == EstadoRelacion::Activa
    }

    pub fn incluye(&self, f: FaccionId) -> bool {
        self.a == f || self.b == f
    }

    /// La otra Facción del par.
    pub fn otra(&self, f: FaccionId) -> FaccionId {
        if self.a == f {
            self.b
        } else {
            self.a
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorDiplomacia {
    /// Una Facción no se relaciona consigo misma.
    MismaFaccion,
    /// Alguna de las dos Facciones no existe.
    FaccionNoExiste,
    /// Ya hay una relación activa entre ellas (para declarar la guerra a una aliada, una señora o una vasalla hay que romper antes).
    YaHayRelacion,
    RelacionNoExiste,
    /// La relación ya está rota.
    NoActiva,
    /// La paz es de una guerra.
    NoEsGuerra,
    /// La rebelión es de un vasallaje.
    NoEsVasallaje,
    /// Esa Facción no está en esa relación.
    NoEsParte,
    /// Esa Facción ya ofreció la paz; falta la otra.
    YaOfrecioPaz,
    /// Un vasallaje con una vasalla que ya lo es de otra (una Facción tiene como mucho una señora).
    YaEsVasalla,
    /// Una guerra no se rompe: se acaba con la paz.
    GuerraNoSeRompe,
    /// Un vasallaje lo libera la señora; la vasalla solo puede rebelarse.
    SoloLaSenora,
}

/// Una guerra recién declarada: una relación por Facción alcanzada (la primera, la del objetivo directo), y las arrastradas por vasallaje.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Guerra {
    pub atacante: FaccionId,
    pub objetivo: FaccionId,
    pub relaciones: Vec<RelacionId>,
    /// Las Facciones del bando del objetivo que entran en guerra por vasallaje (`GuerraDeclarada.arrastradas_ids`), sin el objetivo.
    pub arrastradas: Vec<FaccionId>,
}

/// Lo que pasa al declarar la guerra al atacar (§5.4 y Q2): la guerra, la alianza que se rompió si era una traición, y la reputación.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Declaracion {
    pub guerra: Guerra,
    /// Q2: la alianza con el objetivo que la traición rompió.
    pub alianza_rota: Option<RelacionId>,
    pub sucesos: Vec<(FaccionId, Suceso)>,
}

/// Lo que deja una rebelión (§5.5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rebelion {
    pub senora: FaccionId,
    pub vasalla: FaccionId,
    pub guerra: Guerra,
    pub sucesos: Vec<(FaccionId, Suceso)>,
}

/// Lo que deja una vasalla que un tercero conquistó entera (§5.5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VasallaConquistada {
    pub rota: RelacionId,
    pub nueva: RelacionId,
    pub senora_anterior: FaccionId,
    pub vasalla: FaccionId,
    pub conquistadora: FaccionId,
    /// Las relaciones activas entre la vasalla y la conquistadora que se acaban (las guerras).
    pub acabadas: Vec<RelacionId>,
}

fn par(a: FaccionId, b: FaccionId) -> (FaccionId, FaccionId) {
    (a.min(b), a.max(b))
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Relaciones {
    por_id: BTreeMap<RelacionId, RelacionPolitica>,
    activa: BTreeMap<(FaccionId, FaccionId), RelacionId>,
    /// Vasalla → su vasallaje activo: `senora_de` en O(log n) (lo pregunta el paso de minuto de cada plaza por el tributo, 5D).
    senora: BTreeMap<FaccionId, RelacionId>,
}

impl Relaciones {
    /// Reconstruye el almacén y su índice desde una foto.
    pub fn desde(relaciones: impl IntoIterator<Item = RelacionPolitica>) -> Self {
        let mut r = Self::default();
        relaciones.into_iter().for_each(|x| r.insertar(x));
        r
    }

    fn insertar(&mut self, r: RelacionPolitica) {
        if r.activa() {
            self.activa.insert(par(r.a, r.b), r.id);
            if r.tipo == TipoRelacion::Vasallaje {
                self.senora.insert(r.b, r.id);
            }
        }
        self.por_id.insert(r.id, r);
    }

    pub fn get(&self, id: RelacionId) -> Option<&RelacionPolitica> {
        self.por_id.get(&id)
    }

    /// Todas, en orden de id (las rotas también).
    pub fn iter(&self) -> impl Iterator<Item = &RelacionPolitica> {
        self.por_id.values()
    }

    /// Las activas, en orden de par.
    pub fn activas(&self) -> impl Iterator<Item = &RelacionPolitica> {
        self.activa.values().map(|id| &self.por_id[id])
    }

    pub fn activa_entre(&self, a: FaccionId, b: FaccionId) -> Option<&RelacionPolitica> {
        self.activa.get(&par(a, b)).map(|id| &self.por_id[id])
    }

    fn tipo_entre(&self, a: FaccionId, b: FaccionId) -> Option<TipoRelacion> {
        self.activa_entre(a, b).map(|r| r.tipo)
    }

    /// `estanAliadas`: alianza activa (repostar, «los aliados no se cruzan en combate»).
    pub fn estan_aliadas(&self, a: FaccionId, b: FaccionId) -> bool {
        self.tipo_entre(a, b) == Some(TipoRelacion::Alianza)
    }

    /// `estanEnGuerra`.
    pub fn en_guerra(&self, a: FaccionId, b: FaccionId) -> bool {
        self.tipo_entre(a, b) == Some(TipoRelacion::Guerra)
    }

    /// `compartenVision`: alianza **o** vasallaje, en cualquier sentido (también el grupo `aliados` de la puerta).
    pub fn comparten_vision(&self, a: FaccionId, b: FaccionId) -> bool {
        matches!(self.tipo_entre(a, b), Some(TipoRelacion::Alianza | TipoRelacion::Vasallaje))
    }

    /// La señora de `v`, si es vasalla de alguien.
    pub fn senora_de(&self, v: FaccionId) -> Option<FaccionId> {
        self.vasallaje_de(v).map(|r| r.a)
    }

    /// El vasallaje activo de la vasalla `v` (con su tributo).
    pub fn vasallaje_de(&self, v: FaccionId) -> Option<&RelacionPolitica> {
        self.senora.get(&v).map(|id| &self.por_id[id])
    }

    /// Las vasallas de `s`, por id.
    pub fn vasallas_de(&self, s: FaccionId) -> Vec<FaccionId> {
        let mut v: Vec<_> = self.activas().filter(|r| r.tipo == TipoRelacion::Vasallaje && r.a == s).map(|r| r.b).collect();
        v.sort();
        v
    }

    /// Alianzas activas de una Facción: lo que suma el ritmo de su reputación (`Reputacion::con_alianzas`).
    pub fn alianzas_de(&self, f: FaccionId) -> u32 {
        self.activas().filter(|r| r.tipo == TipoRelacion::Alianza && r.incluye(f)).count() as u32
    }

    /// `senorAjeno`: la señora de `f` si es vasalla de alguien que no es `excepto` (su señora propia puede absorberla o fusionarse con ella).
    pub fn senora_ajena(&self, f: FaccionId, excepto: FaccionId) -> Option<FaccionId> {
        self.senora_de(f).filter(|&s| s != excepto)
    }

    /// `validarPar` (§5.2): distintas, las dos existen y sin relación activa de ninguna clase entre ellas.
    pub fn validar_par(&self, facciones: &Facciones, a: FaccionId, b: FaccionId) -> Result<(), ErrorDiplomacia> {
        if a == b {
            return Err(ErrorDiplomacia::MismaFaccion);
        }
        if !facciones.existe(a) || !facciones.existe(b) {
            return Err(ErrorDiplomacia::FaccionNoExiste);
        }
        if self.activa_entre(a, b).is_some() {
            return Err(ErrorDiplomacia::YaHayRelacion);
        }
        Ok(())
    }

    /// Una relación activa nueva, tras [`Relaciones::validar_par`]. La usan aceptar una propuesta y la administración (`FijarRelacion`, decisión 9:
    /// `a` es la señora o quien declara, sin propuesta ni reputación).
    #[allow(clippy::too_many_arguments)]
    pub fn crear(&mut self, facciones: &Facciones, id: RelacionId, tipo: TipoRelacion, a: FaccionId, b: FaccionId, tributo: Option<Tributo>, ahora: Instante) -> Result<RelacionId, ErrorDiplomacia> {
        self.validar_par(facciones, a, b)?;
        if tipo == TipoRelacion::Vasallaje && self.senora.contains_key(&b) {
            return Err(ErrorDiplomacia::YaEsVasalla);
        }
        let tributo = (tipo == TipoRelacion::Vasallaje).then(|| tributo.unwrap_or(Tributo::POR_DEFECTO));
        self.insertar(RelacionPolitica { id, tipo, a, b, tributo, creada_en: ahora, estado: EstadoRelacion::Activa, rota_por: None, rota_en: None, paz_propuesta_por: None });
        Ok(id)
    }

    /// Pasa una relación a `rota` sin reputación (la anexión, la fusión, el desarme, la administración). Devuelve si estaba activa.
    pub fn terminar(&mut self, id: RelacionId) -> bool {
        let Some(r) = self.por_id.get_mut(&id).filter(|r| r.activa()) else { return false };
        r.estado = EstadoRelacion::Rota;
        self.activa.remove(&par(r.a, r.b));
        if r.tipo == TipoRelacion::Vasallaje {
            self.senora.remove(&r.b);
        }
        true
    }

    /// Las Facciones del bando del objetivo que una guerra de `atacante` arrastraría por vasallaje (§5.4), sin el objetivo y sin mirar si se puede declarar: la señora del objetivo
    /// (o el objetivo, si no es vasalla) y sus demás vasallas, salvo el propio atacante y las que ya tienen una relación activa con él. Es lo que `guerra.confirmar` enseña.
    pub fn arrastre(&self, atacante: FaccionId, objetivo: FaccionId) -> Vec<FaccionId> {
        let senora = self.senora_de(objetivo).unwrap_or(objetivo);
        let mut bando = vec![senora];
        bando.extend(self.vasallas_de(senora));
        let mut vistos = BTreeSet::new();
        bando.retain(|&f| vistos.insert(f) && f != atacante && f != objetivo && self.activa_entre(atacante, f).is_none());
        bando
    }

    /// `declararGuerra` con el **arrastre por vasallaje** (§5.4): `validarPar(atacante, objetivo)`; el bando es la señora del objetivo (o el objetivo si no es
    /// vasalla), el objetivo y las demás vasallas de esa señora; nace una guerra `atacante → id` con cada uno salvo el propio atacante y los que ya tienen una
    /// relación activa con él (el objetivo, si la tuviera, ya hizo fallar `validarPar`). Ids de `ids`, en el orden del bando.
    pub fn declarar_guerra(&mut self, facciones: &Facciones, atacante: FaccionId, objetivo: FaccionId, ahora: Instante, ids: &mut impl FnMut() -> RelacionId) -> Result<Guerra, ErrorDiplomacia> {
        self.validar_par(facciones, atacante, objetivo)?;
        // El orden de siempre: la señora (si el objetivo es vasalla y se arrastra), el objetivo y las demás vasallas.
        let arrastradas = self.arrastre(atacante, objetivo);
        let senora = self.senora_de(objetivo).filter(|s| arrastradas.contains(s));
        let bando: Vec<FaccionId> = senora.into_iter().chain([objetivo]).chain(arrastradas.iter().copied().filter(|&f| Some(f) != senora)).collect();
        let relaciones = bando.iter().map(|&f| self.crear(facciones, ids(), TipoRelacion::Guerra, atacante, f, None, ahora).expect("par validado")).collect();
        Ok(Guerra { atacante, objetivo, relaciones, arrastradas })
    }

    /// `ajustesPorGuerraTrasRomper`: declarar la guerra a una Facción con la que el atacante **rompió** una alianza hace menos de la ventana le cuesta reputación.
    pub fn guerra_tras_romper(&self, rr: &ReglasReputacion, atacante: FaccionId, objetivo: FaccionId, ahora: Instante) -> Option<(FaccionId, Suceso)> {
        self.iter()
            .any(|r| {
                r.tipo == TipoRelacion::Alianza
                    && !r.activa()
                    && r.rota_por == Some(atacante)
                    && par(r.a, r.b) == par(atacante, objetivo)
                    && r.rota_en.is_some_and(|t| ahora < t + rr.ventana_guerra_tras_romper_alianza)
            })
            .then_some((atacante, Suceso::GuerraTrasRomperAlianza))
    }

    /// La guerra que declara **toda agresión** (`atacar` con `declaraGuerra`, §5.4) y la orden `declararGuerra`. Con una alianza activa con el objetivo es la
    /// **traición** de la decisión **Q2** (2026-10-24): rompe la alianza en el acto (`rota_por` = el atacante), cuesta `penalizacionAtacarAliado` (−25) en vez de
    /// la de romper (−12) y declara la guerra con su arrastre. Sin alianza, la guerra normal, con la penalización de la guerra tras romper si toca. Con un
    /// vasallaje entre las dos, se rechaza como siempre (`YaHayRelacion`: la vasalla se rebela, la señora libera).
    /// La traición cuesta **solo** −25 (5A-1, 2026-10-24): no suma además `penalizacionGuerraTrasRomperAlianza` (−15) por la alianza que rompe en ese instante.
    /// Sí deja `rota_por`/`rota_en`, así que volver a declararle la guerra dentro de la ventana (tras una paz) la cobraría.
    pub fn declarar_al_atacar(&mut self, rr: &ReglasReputacion, facciones: &Facciones, atacante: FaccionId, objetivo: FaccionId, ahora: Instante, ids: &mut impl FnMut() -> RelacionId) -> Result<Declaracion, ErrorDiplomacia> {
        let alianza = self.activa_entre(atacante, objetivo).filter(|r| r.tipo == TipoRelacion::Alianza).map(|r| r.id);
        let Some(alianza) = alianza else {
            let sucesos = self.guerra_tras_romper(rr, atacante, objetivo, ahora).into_iter().collect();
            return Ok(Declaracion { guerra: self.declarar_guerra(facciones, atacante, objetivo, ahora, ids)?, alianza_rota: None, sucesos });
        };
        if !facciones.existe(atacante) || !facciones.existe(objetivo) {
            return Err(ErrorDiplomacia::FaccionNoExiste);
        }
        self.romper_alianza(alianza, atacante, ahora);
        let guerra = self.declarar_guerra(facciones, atacante, objetivo, ahora, ids).expect("la alianza era la única relación del par");
        Ok(Declaracion { guerra, alianza_rota: Some(alianza), sucesos: vec![(atacante, Suceso::AtacarAliado)] })
    }

    fn romper_alianza(&mut self, id: RelacionId, por: FaccionId, ahora: Instante) {
        self.terminar(id);
        let r = self.por_id.get_mut(&id).expect("existe");
        r.rota_por = Some(por);
        r.rota_en = Some(ahora);
    }

    fn activa_mut(&mut self, id: RelacionId) -> Result<&mut RelacionPolitica, ErrorDiplomacia> {
        let r = self.por_id.get_mut(&id).ok_or(ErrorDiplomacia::RelacionNoExiste)?;
        if r.activa() {
            Ok(r)
        } else {
            Err(ErrorDiplomacia::NoActiva)
        }
    }

    /// `proponerPaz` (§5.4): por cada guerra; la primera Facción que la ofrece queda en `paz_propuesta_por`; cuando la ofrece la otra, la guerra pasa a `rota`
    /// (firmada). Devuelve si se firmó.
    pub fn proponer_paz(&mut self, id: RelacionId, faccion: FaccionId) -> Result<bool, ErrorDiplomacia> {
        let g = self.activa_mut(id)?;
        if g.tipo != TipoRelacion::Guerra {
            return Err(ErrorDiplomacia::NoEsGuerra);
        }
        if !g.incluye(faccion) {
            return Err(ErrorDiplomacia::NoEsParte);
        }
        match g.paz_propuesta_por {
            Some(f) if f == faccion => Err(ErrorDiplomacia::YaOfrecioPaz),
            Some(_) => Ok(self.terminar(id)),
            None => {
                g.paz_propuesta_por = Some(faccion);
                Ok(false)
            }
        }
    }

    /// `romperRelacion` (§5.5): una guerra no; una alianza la rompe cualquiera de las dos (`penalizacionRomperAlianza` a quien rompe, y queda `rota_por`/`rota_en`);
    /// un vasallaje, solo la señora (`bonusLiberarVasalloVoluntario` para ella).
    /// Romper o rebelarse sobre una relación ya rota se rechaza con `NoActiva` (5A-6, 2026-10-24; el TypeScript no lo miraba y volvía a cobrar la reputación).
    pub fn romper(&mut self, id: RelacionId, iniciador: FaccionId, ahora: Instante) -> Result<Vec<(FaccionId, Suceso)>, ErrorDiplomacia> {
        let r = self.activa_mut(id)?;
        if r.tipo == TipoRelacion::Guerra {
            return Err(ErrorDiplomacia::GuerraNoSeRompe);
        }
        if !r.incluye(iniciador) {
            return Err(ErrorDiplomacia::NoEsParte);
        }
        match r.tipo {
            TipoRelacion::Vasallaje if iniciador != r.a => Err(ErrorDiplomacia::SoloLaSenora),
            TipoRelacion::Vasallaje => {
                let senora = r.a;
                self.terminar(id);
                Ok(vec![(senora, Suceso::LiberarVasalloVoluntario)])
            }
            _ => {
                self.romper_alianza(id, iniciador, ahora);
                Ok(vec![(iniciador, Suceso::RomperAlianza)])
            }
        }
    }

    /// `rebelionVasallo` (§5.5): el vasallaje pasa a `rota`, `penalizacionRebelionParaSenora` a la señora y la vasalla **declara la guerra** a la señora, con el
    /// arrastre (a sus demás vasallas también). Los trueques activos entre plazas de las dos pasan a expirados sin penalización: eso lo hace el núcleo.
    pub fn rebelion(&mut self, facciones: &Facciones, id: RelacionId, ahora: Instante, ids: &mut impl FnMut() -> RelacionId) -> Result<Rebelion, ErrorDiplomacia> {
        let r = self.activa_mut(id)?;
        if r.tipo != TipoRelacion::Vasallaje {
            return Err(ErrorDiplomacia::NoEsVasallaje);
        }
        let (senora, vasalla) = (r.a, r.b);
        self.terminar(id);
        let guerra = self.declarar_guerra(facciones, vasalla, senora, ahora, ids)?;
        Ok(Rebelion { senora, vasalla, guerra, sucesos: vec![(senora, Suceso::RebelionParaSenora)] })
    }

    /// **Señor desarmado** (`liberarVasallosDeSenoresDesarmados`): al perder `senora` su última plaza, todos sus vasallajes activos pasan a `rota`, sin
    /// reputación ni guerra (evento `diplomacia.vasallo_liberado` por cada uno). Devuelve `(relación, vasalla)`.
    pub fn liberar_vasallas_de(&mut self, senora: FaccionId) -> Vec<(RelacionId, FaccionId)> {
        let liberadas: Vec<_> = self.activas().filter(|r| r.tipo == TipoRelacion::Vasallaje && r.a == senora).map(|r| (r.id, r.b)).collect();
        liberadas.iter().for_each(|&(id, _)| {
            self.terminar(id);
        });
        liberadas
    }

    /// **Vasalla conquistada por un tercero** (`pasarVasallasConquistadas`, §5.5): si `vasalla` no tiene plazas (lo sabe el núcleo) y su derrota es de una
    /// Facción que existe y no es su señora, su vasallaje pasa a `rota` y nace uno de la conquistadora con el **mismo tributo** (id `nueva`); las relaciones
    /// activas entre la vasalla y la conquistadora se acaban. Sin reputación. Va justo después del desarme del señor, en el instante de la caída (C7); el
    /// núcleo lo llama también cuando una vasalla sin plazas acepta otra señora (Diplomacia §12: `derrotadaPor` no caduca mientras no tenga plaza).
    /// Se acaba **cualquier** relación activa del par, no solo las guerras como en el TypeScript (5A-3, 2026-10-24): nunca quedan dos activas en el mismo par.
    pub fn pasar_vasalla_conquistada(&mut self, facciones: &Facciones, vasalla: FaccionId, ahora: Instante, nueva: RelacionId) -> Option<VasallaConquistada> {
        let Some(Derrota::Por(conquistadora)) = facciones.get(vasalla)?.derrota else { return None };
        let v = self.activas().find(|r| r.tipo == TipoRelacion::Vasallaje && r.b == vasalla && r.a != conquistadora)?.clone();
        if !facciones.existe(conquistadora) {
            return None;
        }
        self.terminar(v.id);
        let acabadas: Vec<_> = self.activa_entre(vasalla, conquistadora).map(|r| r.id).into_iter().collect();
        acabadas.iter().for_each(|&id| {
            self.terminar(id);
        });
        self.crear(facciones, nueva, TipoRelacion::Vasallaje, conquistadora, vasalla, v.tributo, ahora).expect("par libre");
        Some(VasallaConquistada { rota: v.id, nueva, senora_anterior: v.a, vasalla, conquistadora, acabadas })
    }

    /// **Defender al vasallo** (`ajustesPorDefensaDeVasallo`, §5.9): por cada señora de `vasalla`, `DefenderVasallo` si algún héroe suyo defendió (`presentes`)
    /// y ganó el defensor; `NoDefenderVasallo` si no estuvo y ganó el atacante; nada en los otros casos.
    pub fn defensa_de_vasallo(&self, vasalla: FaccionId, presentes: &BTreeSet<FaccionId>, gano_el_defensor: bool) -> Vec<(FaccionId, Suceso)> {
        let mut senoras: Vec<_> = self.activas().filter(|r| r.tipo == TipoRelacion::Vasallaje && r.b == vasalla).map(|r| r.a).collect();
        senoras.sort();
        senoras
            .into_iter()
            .filter_map(|s| match (presentes.contains(&s), gano_el_defensor) {
                (true, true) => Some((s, Suceso::DefenderVasallo)),
                (false, false) => Some((s, Suceso::NoDefenderVasallo)),
                _ => None,
            })
            .collect()
    }

    /// El vasallaje cuya señora es `origen` pasa a `destino` (el traslado). Mantiene el id y el tributo.
    pub(crate) fn cambiar_senora(&mut self, id: RelacionId, destino: FaccionId) {
        let r = self.por_id.get_mut(&id).expect("existe");
        self.activa.remove(&par(r.a, r.b));
        r.a = destino;
        self.activa.insert(par(r.a, r.b), id);
    }
}

#[cfg(test)]
pub(crate) mod pruebas {
    //! Casos de `typescript-final:src/engine/__tests__/{vasallaConquistada,reputacionVasallos}.test.ts` y de las reglas de `engine/diplomacia.ts` que prueban
    //! `comandosDiplomacia.test.ts` y `eventosDominioDiplomacia.test.ts`; más Q2.
    use super::*;
    use crate::datos::DIA;
    use crate::faccion::pruebas::con_faccion;
    use comercio::datos::Reglas as ReglasComercio;

    /// Facciones 1..=n, cada una con su Rey `10 × id`.
    pub(crate) fn facciones(n: FaccionId) -> Facciones {
        let mut fs = Facciones::default();
        for id in 1..=n {
            con_faccion(&mut fs, id, &format!("F{id}"), 10 * id as u64, &[]);
        }
        fs
    }

    pub(crate) fn contador() -> impl FnMut() -> RelacionId {
        let mut n = 100;
        move || {
            n += 1;
            n
        }
    }

    fn rr() -> ReglasReputacion {
        ReglasComercio::cargar().reputacion
    }

    #[test]
    fn validar_par_mira_distintas_existentes_y_sin_relacion_activa() {
        let fs = facciones(3);
        let mut rs = Relaciones::default();
        assert_eq!(rs.validar_par(&fs, 1, 1), Err(ErrorDiplomacia::MismaFaccion));
        assert_eq!(rs.validar_par(&fs, 1, 9), Err(ErrorDiplomacia::FaccionNoExiste));
        rs.crear(&fs, 1, TipoRelacion::Alianza, 1, 2, None, 0).unwrap();
        assert_eq!(rs.validar_par(&fs, 2, 1), Err(ErrorDiplomacia::YaHayRelacion), "en cualquier sentido");
        assert!(rs.terminar(1));
        assert!(!rs.terminar(1));
        assert_eq!(rs.validar_par(&fs, 2, 1), Ok(()), "una rota ya no cuenta");
        assert_eq!(rs.get(1).unwrap().estado, EstadoRelacion::Rota, "y se conserva");
    }

    #[test]
    fn los_predicados_miran_solo_la_activa_del_par() {
        let fs = facciones(4);
        let mut rs = Relaciones::default();
        rs.crear(&fs, 1, TipoRelacion::Alianza, 1, 2, None, 0).unwrap();
        rs.crear(&fs, 2, TipoRelacion::Vasallaje, 3, 1, None, 0).unwrap();
        rs.crear(&fs, 3, TipoRelacion::Guerra, 4, 1, None, 0).unwrap();
        assert!(rs.estan_aliadas(2, 1) && !rs.estan_aliadas(1, 3));
        assert!(rs.comparten_vision(1, 2) && rs.comparten_vision(1, 3) && !rs.comparten_vision(1, 4));
        assert!(rs.en_guerra(1, 4) && !rs.en_guerra(1, 2));
        assert_eq!((rs.senora_de(1), rs.vasallas_de(3), rs.alianzas_de(1)), (Some(3), vec![1], 1));
        assert_eq!(rs.get(2).unwrap().tributo, Some(Tributo::POR_DEFECTO), "vasallaje sin tributo: trigo a 0");
        assert_eq!(rs.get(1).unwrap().tributo, None);
    }

    #[test]
    fn la_guerra_se_arrastra_por_vasallaje_salvo_a_quien_ya_tiene_relacion() {
        // 1 señora de 2 y 3; 4 aliada de 3. 4 declara a 2: entra 1 (señora) y 2; 3 no porque ya es aliada de 4.
        let fs = facciones(4);
        let mut rs = Relaciones::default();
        rs.crear(&fs, 1, TipoRelacion::Vasallaje, 1, 2, None, 0).unwrap();
        rs.crear(&fs, 2, TipoRelacion::Vasallaje, 1, 3, None, 0).unwrap();
        rs.crear(&fs, 3, TipoRelacion::Alianza, 4, 3, None, 0).unwrap();
        let g = rs.declarar_guerra(&fs, 4, 2, 5, &mut contador()).unwrap();
        assert_eq!(g, Guerra { atacante: 4, objetivo: 2, relaciones: vec![101, 102], arrastradas: vec![1] });
        assert!(rs.en_guerra(4, 1) && rs.en_guerra(4, 2) && !rs.en_guerra(4, 3));
        assert_eq!(rs.get(101).unwrap().a, 4, "A es quien declara");
        // A la señora: todas sus vasallas.
        let fs = facciones(4);
        let mut rs = Relaciones::default();
        rs.crear(&fs, 1, TipoRelacion::Vasallaje, 1, 2, None, 0).unwrap();
        rs.crear(&fs, 2, TipoRelacion::Vasallaje, 1, 3, None, 0).unwrap();
        let g = rs.declarar_guerra(&fs, 4, 1, 5, &mut contador()).unwrap();
        assert_eq!(g.arrastradas, vec![2, 3]);
        // Con relación activa con el objetivo directo, falla entero.
        assert_eq!(rs.declarar_guerra(&fs, 4, 2, 5, &mut contador()), Err(ErrorDiplomacia::YaHayRelacion));
        assert_eq!(rs.declarar_guerra(&fs, 1, 2, 5, &mut contador()), Err(ErrorDiplomacia::YaHayRelacion), "D4: un vasallaje no se sustituye por la guerra");
    }

    #[test]
    fn la_paz_es_mutua() {
        let fs = facciones(2);
        let mut rs = Relaciones::default();
        let g = rs.declarar_guerra(&fs, 1, 2, 0, &mut contador()).unwrap().relaciones[0];
        assert_eq!(rs.proponer_paz(g, 3), Err(ErrorDiplomacia::NoEsParte));
        assert_eq!(rs.proponer_paz(g, 1), Ok(false));
        assert_eq!(rs.proponer_paz(g, 1), Err(ErrorDiplomacia::YaOfrecioPaz));
        assert_eq!(rs.proponer_paz(g, 2), Ok(true), "la ofrece la otra: firmada");
        assert!(!rs.en_guerra(1, 2));
        assert_eq!(rs.proponer_paz(g, 2), Err(ErrorDiplomacia::NoActiva));
        assert_eq!(rs.romper(g, 1, 0), Err(ErrorDiplomacia::NoActiva));
    }

    #[test]
    fn romper_una_alianza_cuesta_a_quien_rompe_y_un_vasallaje_solo_lo_libera_la_senora() {
        let fs = facciones(3);
        let mut rs = Relaciones::default();
        rs.crear(&fs, 1, TipoRelacion::Alianza, 1, 2, None, 0).unwrap();
        rs.crear(&fs, 2, TipoRelacion::Vasallaje, 1, 3, None, 0).unwrap();
        let g = rs.declarar_guerra(&fs, 2, 3, 0, &mut contador()).unwrap().relaciones[0];
        assert_eq!(rs.romper(g, 2, 0), Err(ErrorDiplomacia::GuerraNoSeRompe));
        assert_eq!(rs.romper(1, 3, 0), Err(ErrorDiplomacia::NoEsParte));
        assert_eq!(rs.romper(1, 2, 7), Ok(vec![(2, Suceso::RomperAlianza)]));
        assert_eq!((rs.get(1).unwrap().rota_por, rs.get(1).unwrap().rota_en), (Some(2), Some(7)));
        assert_eq!(rs.romper(2, 3, 0), Err(ErrorDiplomacia::SoloLaSenora));
        assert_eq!(rs.romper(2, 1, 0), Ok(vec![(1, Suceso::LiberarVasalloVoluntario)]));
        assert_eq!(rs.get(2).unwrap().rota_por, None, "solo las alianzas anotan quién rompió");
        assert_eq!(rs.romper(99, 1, 0), Err(ErrorDiplomacia::RelacionNoExiste));
    }

    #[test]
    fn la_guerra_tras_romper_una_alianza_cuesta_dentro_de_la_ventana_y_solo_a_quien_rompio() {
        let (fs, rr) = (facciones(2), rr());
        let mut rs = Relaciones::default();
        rs.crear(&fs, 1, TipoRelacion::Alianza, 1, 2, None, 0).unwrap();
        rs.romper(1, 1, 0).unwrap();
        assert_eq!(rs.guerra_tras_romper(&rr, 1, 2, 3 * DIA - 1), Some((1, Suceso::GuerraTrasRomperAlianza)));
        assert_eq!(rs.guerra_tras_romper(&rr, 1, 2, 3 * DIA), None, "fuera de la ventana");
        assert_eq!(rs.guerra_tras_romper(&rr, 2, 1, DIA), None, "no rompió ella");
        let d = rs.declarar_al_atacar(&rr, &fs, 1, 2, DIA, &mut contador()).unwrap();
        assert_eq!((d.alianza_rota, d.sucesos), (None, vec![(1, Suceso::GuerraTrasRomperAlianza)]));
    }

    #[test]
    fn q2_atacar_a_un_aliado_es_una_traicion() {
        // 1 aliada de 2; 2 señora de 3. 1 ataca a 2: rompe la alianza, −25 y guerra a 2 y a su vasalla 3.
        let (fs, rr) = (facciones(3), rr());
        let mut rs = Relaciones::default();
        rs.crear(&fs, 1, TipoRelacion::Alianza, 1, 2, None, 0).unwrap();
        rs.crear(&fs, 2, TipoRelacion::Vasallaje, 2, 3, None, 0).unwrap();
        let d = rs.declarar_al_atacar(&rr, &fs, 1, 2, 9, &mut contador()).unwrap();
        assert_eq!(d.alianza_rota, Some(1));
        assert_eq!(d.sucesos, vec![(1, Suceso::AtacarAliado)]);
        assert_eq!(Suceso::AtacarAliado.delta(&rr), -25 * comercio::datos::PUNTO);
        assert_eq!(d.guerra.arrastradas, vec![3]);
        assert!(rs.en_guerra(1, 2) && rs.en_guerra(1, 3) && !rs.estan_aliadas(1, 2));
        assert_eq!((rs.get(1).unwrap().rota_por, rs.get(1).unwrap().rota_en), (Some(1), Some(9)));
        // Atacar a la propia vasalla o a la señora sigue rechazado.
        assert_eq!(rs.declarar_al_atacar(&rr, &fs, 2, 3, 9, &mut contador()), Err(ErrorDiplomacia::YaHayRelacion));
    }

    #[test]
    fn la_rebelion_rompe_el_vasallaje_penaliza_a_la_senora_y_declara_la_guerra_con_arrastre() {
        let fs = facciones(3);
        let mut rs = Relaciones::default();
        rs.crear(&fs, 1, TipoRelacion::Vasallaje, 1, 2, None, 0).unwrap();
        rs.crear(&fs, 2, TipoRelacion::Vasallaje, 1, 3, None, 0).unwrap();
        let a = rs.crear(&fs, 3, TipoRelacion::Alianza, 2, 3, None, 0);
        assert!(a.is_ok());
        let rb = rs.rebelion(&fs, 1, 5, &mut contador()).unwrap();
        assert_eq!((rb.senora, rb.vasalla, rb.sucesos.clone()), (1, 2, vec![(1, Suceso::RebelionParaSenora)]));
        assert!(rs.en_guerra(2, 1));
        assert_eq!(rb.guerra.arrastradas, Vec::<FaccionId>::new(), "la otra vasalla es su aliada: no se arrastra");
        assert_eq!(rs.rebelion(&fs, 1, 5, &mut contador()), Err(ErrorDiplomacia::NoActiva));
        assert_eq!(rs.rebelion(&fs, 3, 5, &mut contador()), Err(ErrorDiplomacia::NoEsVasallaje));
    }

    #[test]
    fn el_senor_desarmado_libera_a_sus_vasallas_sin_mas() {
        let fs = facciones(3);
        let mut rs = Relaciones::default();
        rs.crear(&fs, 1, TipoRelacion::Vasallaje, 1, 2, None, 0).unwrap();
        rs.crear(&fs, 2, TipoRelacion::Vasallaje, 1, 3, None, 0).unwrap();
        assert_eq!(rs.liberar_vasallas_de(1), vec![(1, 2), (2, 3)]);
        assert!(rs.activas().next().is_none());
        assert!(rs.liberar_vasallas_de(1).is_empty());
    }

    fn escenario_conquista(derrota: Option<FaccionId>) -> (Facciones, Relaciones) {
        // senora 1, vasalla 2, tercero 3 en guerra con la vasalla.
        let mut fs = facciones(3);
        if let Some(d) = derrota {
            fs.registrar_derrota(2, Some(d));
        }
        let mut rs = Relaciones::default();
        rs.crear(&fs, 1, TipoRelacion::Vasallaje, 1, 2, Some(Tributo { recurso: Recurso::Madera, cantidad_por_minuto: 4_000 }), 0).unwrap();
        rs.crear(&fs, 2, TipoRelacion::Guerra, 3, 2, None, 0).unwrap();
        (fs, rs)
    }

    #[test]
    fn la_vasalla_conquistada_por_un_tercero_pasa_a_el_con_el_mismo_tributo() {
        let (fs, mut rs) = escenario_conquista(Some(3));
        let c = rs.pasar_vasalla_conquistada(&fs, 2, 50, 77).unwrap();
        assert_eq!(c, VasallaConquistada { rota: 1, nueva: 77, senora_anterior: 1, vasalla: 2, conquistadora: 3, acabadas: vec![2] });
        let nueva = rs.get(77).unwrap();
        assert_eq!((nueva.a, nueva.b, nueva.tributo, nueva.creada_en), (3, 2, Some(Tributo { recurso: Recurso::Madera, cantidad_por_minuto: 4_000 }), 50));
        assert!(!rs.en_guerra(2, 3) && rs.senora_de(2) == Some(3));
    }

    #[test]
    fn sin_efecto_si_cayo_por_ruina_la_conquisto_su_senora_o_ya_es_del_conquistador() {
        let (fs, mut rs) = escenario_conquista(None);
        assert_eq!(rs.pasar_vasalla_conquistada(&fs, 2, 50, 77), None, "sin derrota");
        let mut fs = facciones(3);
        fs.registrar_derrota(2, None);
        assert_eq!(rs.pasar_vasalla_conquistada(&fs, 2, 50, 77), None, "colapso");
        let (fs, mut rs) = escenario_conquista(Some(1));
        assert_eq!(rs.pasar_vasalla_conquistada(&fs, 2, 50, 77), None, "la conquistó su señora");
        assert_eq!(rs.senora_de(2), Some(1));
    }

    #[test]
    fn defender_al_vasallo() {
        let fs = facciones(3);
        let mut rs = Relaciones::default();
        rs.crear(&fs, 1, TipoRelacion::Vasallaje, 1, 2, None, 0).unwrap();
        let presente = BTreeSet::from([1]);
        let nadie = BTreeSet::new();
        assert_eq!(rs.defensa_de_vasallo(2, &presente, true), vec![(1, Suceso::DefenderVasallo)]);
        assert_eq!(rs.defensa_de_vasallo(2, &nadie, false), vec![(1, Suceso::NoDefenderVasallo)]);
        assert!(rs.defensa_de_vasallo(2, &presente, false).is_empty(), "estuvo y perdió: nada");
        assert!(rs.defensa_de_vasallo(2, &nadie, true).is_empty(), "no estuvo y ganó: nada");
        assert!(rs.defensa_de_vasallo(3, &nadie, false).is_empty(), "sin señora");
    }

    #[test]
    fn una_vasalla_tiene_una_sola_senora_y_el_arrastre_se_ve_sin_declarar() {
        // 1 señora de 2 y 3; 4 aliada de 3.
        let fs = facciones(5);
        let mut rs = Relaciones::default();
        rs.crear(&fs, 1, TipoRelacion::Vasallaje, 1, 2, None, 0).unwrap();
        rs.crear(&fs, 2, TipoRelacion::Vasallaje, 1, 3, None, 0).unwrap();
        assert_eq!(rs.crear(&fs, 3, TipoRelacion::Vasallaje, 5, 2, None, 0), Err(ErrorDiplomacia::YaEsVasalla));
        rs.crear(&fs, 4, TipoRelacion::Alianza, 4, 3, None, 0).unwrap();
        assert_eq!((rs.vasallaje_de(2).map(|r| r.id), rs.senora_de(3)), (Some(1), Some(1)));
        assert_eq!(rs.arrastre(4, 2), vec![1], "la señora entra; 3 no, porque ya es aliada de 4");
        assert_eq!(rs.arrastre(5, 1), vec![2, 3], "contra la señora, todas sus vasallas");
        assert!(rs.terminar(1));
        assert_eq!(rs.senora_de(2), None, "el índice olvida el vasallaje roto");
        assert_eq!(Relaciones::desde(rs.iter().cloned().collect::<Vec<_>>()), rs);
    }

    #[test]
    fn el_indice_se_reconstruye_desde_la_foto() {
        let fs = facciones(3);
        let mut rs = Relaciones::default();
        rs.crear(&fs, 1, TipoRelacion::Alianza, 1, 2, None, 0).unwrap();
        rs.romper(1, 1, 0).unwrap();
        rs.crear(&fs, 2, TipoRelacion::Alianza, 2, 1, None, 0).unwrap();
        let copia = Relaciones::desde(rs.iter().cloned().collect::<Vec<_>>());
        assert_eq!(copia, rs);
        assert_eq!(copia.activa_entre(1, 2).unwrap().id, 2);
    }
}
