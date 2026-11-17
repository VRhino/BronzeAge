//! Las épicas de los Aedas residentes (TA §7.2): empezar, abandonar, los hechos inspiradores **tipados** (decisión técnica 3: el núcleo los anota en la
//! transición donde ocurren, no se reparsean los hechos de la versión) y su avance con claves irrepetibles y enfriamiento. El núcleo llama a
//! [`Residentes::avanzar_epicas`] una vez por versión con todos los hechos anotados, antes de cerrarla (6E): a lo sumo un hecho por Aeda y versión.

use crate::datos::{HechoEpico, Reglas};
use crate::estado::EstadoTecnologia;
use crate::residentes::Residentes;
use crate::{AsentamientoId, FaccionId, TecId};
use asentamiento::Tipo;
use base::{Instante, Mil};
use std::collections::VecDeque;

/// La clave irrepetible de un hecho (las cadenas del TS, tipadas). Las batallas de Unity van por su id: se cantan una vez aunque toquen varias plazas.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Clave {
    /// `defensa:<plaza>:<momento>`.
    Defensa { plaza: AsentamientoId, en: Instante },
    /// `conquista:<plaza>:<momento>`.
    Conquista { plaza: AsentamientoId, en: Instante },
    /// `batalla:<A>:<B>:<momento>` (las columnas del encuentro).
    Batalla { a: u64, b: u64, en: Instante },
    /// `defensa:<battleId>`, `conquista:<battleId>`, `batalla:<battleId>`: una batalla de Unity.
    Unity { tipo: HechoEpico, batalla: u64 },
    /// `obra:<edificioId>:<nivelNuevo ?? 0>` (con la plaza: el id de edificio es de la plaza).
    Obra { plaza: AsentamientoId, edificio: u32, nivel: u8 },
    /// `ascenso:<plaza>:<nivel>`.
    Ascenso { plaza: AsentamientoId, nivel: u8 },
    /// `adopcion:<facción>:<tecnología>`.
    Adopcion { faccion: FaccionId, tec: TecId },
    /// `caravana:<caravanaId>`.
    Caravana(u64),
}

/// Un hecho inspirador (`Hecho` de `hechosDeEventos`): de qué tipo, de quién, dónde y con qué clave. Un hecho **de plaza** solo inspira al Aeda de esa
/// plaza; uno **de Facción** (sin plaza), a los de todas sus plazas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hecho {
    pub tipo: HechoEpico,
    pub faccion: FaccionId,
    pub plaza: Option<AsentamientoId>,
    pub edificio: Option<Tipo>,
    pub clave: Clave,
}

/// El contexto de una batalla de Unity aplicada, para su hecho.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextoUnity {
    CampoAbierto,
    Asedio(AsentamientoId),
    /// Caravana o campamento de bandidos: no inspiran.
    Otro,
}

impl Hecho {
    /// `combate.asedio_resistido` con `enCombate`: la defensa de la plaza (lo trivial, un rebote sin combate, no cuenta).
    pub fn defensa(en_combate: bool, plaza: AsentamientoId, defensora: FaccionId, en: Instante) -> Option<Hecho> {
        en_combate.then_some(Hecho { tipo: HechoEpico::Defensa, faccion: defensora, plaza: Some(plaza), edificio: None, clave: Clave::Defensa { plaza, en } })
    }

    /// `combate.asedio_conquista` con `enCombate`: de la Facción atacante.
    pub fn conquista(en_combate: bool, plaza: AsentamientoId, atacante: FaccionId, en: Instante) -> Option<Hecho> {
        en_combate.then_some(Hecho { tipo: HechoEpico::Conquista, faccion: atacante, plaza: None, edificio: None, clave: Clave::Conquista { plaza, en } })
    }

    /// `combate.encuentro` entre Facciones **distintas**: de la ganadora. `a` y `b` son (columna, Facción).
    pub fn batalla(a: (u64, FaccionId), b: (u64, FaccionId), gana_a: bool, en: Instante) -> Option<Hecho> {
        (a.1 != b.1).then_some(Hecho { tipo: HechoEpico::Batalla, faccion: if gana_a { a.1 } else { b.1 }, plaza: None, edificio: None, clave: Clave::Batalla { a: a.0, b: b.0, en } })
    }

    /// Una batalla de Unity aplicada con ganador: campo abierto entre Facciones distintas (batalla de la ganadora), asedio resistido (defensa de la plaza)
    /// o conquistado (conquista de la atacante). Sin ganador o sin Facción en el bando que cuenta (bandidos), nada.
    pub fn de_unity(batalla: u64, contexto: ContextoUnity, atacante: Option<FaccionId>, defensora: Option<FaccionId>, gana_el_atacante: Option<bool>) -> Option<Hecho> {
        let gana = gana_el_atacante?;
        let h = |tipo, faccion, plaza| Some(Hecho { tipo, faccion, plaza, edificio: None, clave: Clave::Unity { tipo, batalla } });
        match contexto {
            ContextoUnity::CampoAbierto => match (atacante, defensora) {
                (Some(a), Some(d)) if a != d => h(HechoEpico::Batalla, if gana { a } else { d }, None),
                _ => None,
            },
            ContextoUnity::Asedio(plaza) if !gana => h(HechoEpico::Defensa, defensora?, Some(plaza)),
            ContextoUnity::Asedio(_) => h(HechoEpico::Conquista, atacante?, None),
            ContextoUnity::Otro => None,
        }
    }

    /// `construccion.edificio_completado` (`nivel_nuevo = None`) o `construccion.mejora_completada`: la obra de la plaza.
    pub fn obra(plaza: AsentamientoId, faccion: FaccionId, edificio: u32, tipo: Tipo, nivel_nuevo: Option<u8>) -> Hecho {
        Hecho { tipo: HechoEpico::Obra, faccion, plaza: Some(plaza), edificio: Some(tipo), clave: Clave::Obra { plaza, edificio, nivel: nivel_nuevo.unwrap_or(0) } }
    }

    /// `asentamiento.nivel_subio`.
    pub fn ascenso(plaza: AsentamientoId, faccion: FaccionId, nivel: u8) -> Hecho {
        Hecho { tipo: HechoEpico::Ascenso, faccion, plaza: Some(plaza), edificio: None, clave: Clave::Ascenso { plaza, nivel } }
    }

    /// `tecnologia.adoptada`: de la Facción.
    pub fn adopcion(faccion: FaccionId, tec: TecId) -> Hecho {
        Hecho { tipo: HechoEpico::Adopcion, faccion, plaza: None, edificio: None, clave: Clave::Adopcion { faccion, tec } }
    }

    /// `comercio.caravana_llega` de una caravana **comercial** con carga total (milésimas) ≥ `cargaMinimaCaravana`: de su plaza de origen (si sigue
    /// siendo de alguien).
    pub fn caravana(r: &Reglas, caravana: u64, comercial: bool, origen: AsentamientoId, faccion_origen: Option<FaccionId>, carga: Mil) -> Option<Hecho> {
        let faccion = faccion_origen?;
        (comercial && carga >= r.aedas.carga_minima_caravana).then_some(Hecho { tipo: HechoEpico::Caravana, faccion, plaza: Some(origen), edificio: None, clave: Clave::Caravana(caravana) })
    }
}

/// `EpicaEnCurso`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Epica {
    pub tec: TecId,
    /// El capítulo en curso, desde 0.
    pub capitulo: u8,
    /// Hechos contados del capítulo en curso.
    pub hechos: u32,
    pub ultimo_hecho_en: Option<Instante>,
    /// Las últimas `clavesRecordadas` claves contadas.
    pub claves: VecDeque<Clave>,
}

impl Epica {
    pub fn nueva(tec: TecId) -> Self {
        Epica { tec, capitulo: 0, hechos: 0, ultimo_hecho_en: None, claves: VecDeque::new() }
    }
}

/// Por qué no se empieza o abandona una épica (`epica.*`, decisión técnica 9: los ocho motivos de TA §7.2 y el de abandonar).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorEpica {
    /// `epica.no_vive_aqui`: ese Aeda no vive en esta plaza.
    NoViveAqui,
    /// `epica.sin_epica`: la tecnología no tiene épica (las de arranque).
    SinEpica,
    /// `epica.aeda_ajeno`: el Aeda aún no se ha asentado con el dueño actual de la plaza.
    AedaAjeno,
    /// `epica.ya_canta`: ya canta otra; hay que abandonarla antes.
    YaCanta,
    /// `epica.sin_logro`: el logro del servidor no se ha cumplido.
    SinLogro,
    /// `epica.era_cerrada`.
    EraCerrada,
    /// `epica.ya_aparecida`.
    YaAparecida,
    /// `epica.otro_la_canta`: otro Aeda de la Facción ya canta la épica de esa tecnología.
    OtroLaCanta,
    /// `epica.no_canta` (abandonar): no canta ninguna.
    NoCanta,
}

/// Lo que pasó con una épica al contar un hecho.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Avance {
    /// Contó un hecho sin cerrar el capítulo (sin evento).
    Hecho { aeda: u32 },
    /// Cerró un capítulo y queda otro (`aedas.epica_capitulo { aeda, faccion, tecnologia, capitulo, capitulos }`, privado de la plaza). `capitulo` es el
    /// que empieza (1 = el segundo), como el payload del TS.
    Capitulo { aeda: u32, plaza: AsentamientoId, faccion: FaccionId, tec: TecId, capitulo: u8, capitulos: u8 },
    /// Cerró el último: la tecnología aparece sin hito (`aedas.epica_tecnologia`, privado de la plaza, y `aedas.epica_cumplida`, **público**, sin nombrarla).
    /// `primera`: la Facción queda como descubridora.
    Cumplida { aeda: u32, plaza: AsentamientoId, faccion: FaccionId, tec: TecId, capitulos: u8, primera: bool },
}

/// `inspira`: mismo tipo que el capítulo, mismo edificio si lo pide, y el hecho es de la plaza del Aeda o, sin plaza, de su Facción.
fn inspira(h: &Hecho, plaza: AsentamientoId, faccion: FaccionId, cap: &crate::datos::Capitulo) -> bool {
    h.tipo == cap.hecho && cap.edificio.is_none_or(|e| h.edificio == Some(e)) && h.plaza.map_or(h.faccion == faccion, |p| p == plaza)
}

impl Residentes {
    /// `empezarEpica`, con las comprobaciones en el orden del TS. `duena` es la Facción dueña actual de la plaza. No cuesta oro; puede empezar aunque la
    /// tecnología solo esté revelada, o ni eso. Devuelve los capítulos que tiene (`aedas.epica_empieza { …, capitulo: 0, capitulos }`).
    pub fn empezar_epica(&mut self, r: &Reglas, t: &EstadoTecnologia, plaza: AsentamientoId, duena: FaccionId, aeda: u32, tec: TecId) -> Result<u8, ErrorEpica> {
        let a = self.aedas.iter().find(|a| a.id == aeda && a.plaza == plaza).ok_or(ErrorEpica::NoViveAqui)?;
        let def = r.def(tec).epica.as_ref().ok_or(ErrorEpica::SinEpica)?;
        if a.faccion != duena {
            return Err(ErrorEpica::AedaAjeno);
        }
        if a.epica.is_some() {
            return Err(ErrorEpica::YaCanta);
        }
        if !t.logro_cumplido(tec) {
            return Err(ErrorEpica::SinLogro);
        }
        if !t.era_abierta(r, tec) {
            return Err(ErrorEpica::EraCerrada);
        }
        if t.de(r, duena).aparecidas.contiene(tec) {
            return Err(ErrorEpica::YaAparecida);
        }
        if self.aedas.iter().any(|o| o.faccion == duena && o.epica.as_ref().is_some_and(|e| e.tec == tec)) {
            return Err(ErrorEpica::OtroLaCanta);
        }
        self.aedas.iter_mut().find(|a| a.id == aeda).expect("existe").epica = Some(Epica::nueva(tec));
        Ok(def.capitulos.len() as u8)
    }

    /// `abandonarEpica`: borra la épica en curso y todo lo avanzado (`aedas.epica_abandonada { motivo: orden }`). Devuelve su tecnología.
    pub fn abandonar_epica(&mut self, plaza: AsentamientoId, aeda: u32) -> Result<TecId, ErrorEpica> {
        let a = self.aedas.iter_mut().find(|a| a.id == aeda && a.plaza == plaza).ok_or(ErrorEpica::NoViveAqui)?;
        a.epica.take().map(|e| e.tec).ok_or(ErrorEpica::NoCanta)
    }

    /// `avanzarEpicas`: para cada Aeda con épica cuya plaza sigue siendo de su Facción (`faccion_de`), fuera del enfriamiento, el **primer** hecho de la
    /// lista que no ha contado ya y que inspira su capítulo cuenta (a lo sumo uno por Aeda y llamada). Al cerrar el último capítulo la tecnología aparece
    /// sin hito (`hacer_aparecer`) y la Facción suma una épica cumplida.
    pub fn avanzar_epicas(&mut self, r: &Reglas, t: &mut EstadoTecnologia, hechos: &[Hecho], faccion_de: impl Fn(AsentamientoId) -> Option<FaccionId>, ahora: Instante) -> Vec<Avance> {
        let mut avances = Vec::new();
        if hechos.is_empty() {
            return avances;
        }
        for a in self.aedas.iter_mut() {
            let Some(e) = a.epica.as_mut() else { continue };
            if faccion_de(a.plaza) != Some(a.faccion) || e.ultimo_hecho_en.is_some_and(|u| ahora - u < r.aedas.enfriamiento_epica) {
                continue;
            }
            let def = r.def(e.tec).epica.as_ref().expect("épica");
            let cap = &def.capitulos[e.capitulo as usize];
            let Some(h) = hechos.iter().find(|h| !e.claves.contains(&h.clave) && inspira(h, a.plaza, a.faccion, cap)) else { continue };
            e.claves.push_back(h.clave);
            while e.claves.len() > r.aedas.claves_recordadas {
                e.claves.pop_front();
            }
            e.hechos += 1;
            e.ultimo_hecho_en = Some(ahora);
            let capitulos = def.capitulos.len() as u8;
            if e.hechos < cap.cantidad {
                avances.push(Avance::Hecho { aeda: a.id });
            } else if e.capitulo + 1 < capitulos {
                e.capitulo += 1;
                e.hechos = 0;
                avances.push(Avance::Capitulo { aeda: a.id, plaza: a.plaza, faccion: a.faccion, tec: e.tec, capitulo: e.capitulo, capitulos });
            } else {
                let tec = e.tec;
                a.epica = None;
                *self.cumplidas.entry(a.faccion).or_insert(0) += 1;
                let primera = t.hacer_aparecer(r, a.faccion, tec, ahora).unwrap_or(false);
                avances.push(Avance::Cumplida { aeda: a.id, plaza: a.plaza, faccion: a.faccion, tec, capitulos, primera });
            }
        }
        avances
    }
}

#[cfg(test)]
mod pruebas {
    //! Casos de `typescript-final:src/engine/__tests__/aedasResidentes.test.ts` («empezar y abandonar una épica», «hechos», «avance de una épica») y la
    //! parte de reglas de `typescript-final:src/session/__tests__/comandosTecnologia.test.ts` («épica de un Aeda residente»).
    use super::*;
    use crate::estado::pruebas::t as tec;
    use crate::estado::Primero;
    use crate::residentes::pruebas::{con_residente, r};
    use crate::residentes::AedaResidente;

    /// El logro de `metalurgia_cobre` cumplido (`tecnologia` del TS).
    fn tecnologia() -> EstadoTecnologia {
        let r = r();
        let mut e = EstadoTecnologia::nuevo(&r, 0);
        e.logros[tec(&r, "metalurgia_cobre") as usize] = Some(0);
        e
    }

    fn de_10(p: AsentamientoId) -> Option<FaccionId> {
        (p == 10).then_some(1)
    }

    #[test]
    fn empieza_con_el_logro_cumplido_y_la_tecnologia_sin_aparecer_y_se_abandona_perdiendo_lo_avanzado() {
        let r = r();
        let cobre = tec(&r, "metalurgia_cobre");
        let mut s = con_residente(None);
        assert_eq!(s.empezar_epica(&r, &tecnologia(), 10, 1, 1, cobre), Ok(3));
        assert_eq!(s.aedas[0].epica, Some(Epica::nueva(cobre)));
        assert_eq!(s.abandonar_epica(10, 1), Ok(cobre));
        assert!(s.aedas[0].epica.is_none());
        assert_eq!(s.abandonar_epica(10, 1), Err(ErrorEpica::NoCanta));
    }

    #[test]
    fn rechazo_el_aeda_aun_no_ha_pasado_al_nuevo_dueno() {
        let r = r();
        let mut s = con_residente(None);
        assert_eq!(s.empezar_epica(&r, &tecnologia(), 10, 2, 1, tec(&r, "metalurgia_cobre")), Err(ErrorEpica::AedaAjeno));
    }

    #[test]
    fn rechazos_sin_logro_era_cerrada_ya_aparecida_sin_epica_aeda_ajeno_ya_con_epica_o_repetida_por_otro() {
        let r = r();
        let cobre = tec(&r, "metalurgia_cobre");
        let t = tecnologia();
        let mut s = con_residente(None);
        assert_eq!(s.empezar_epica(&r, &t, 10, 1, 1, tec(&r, "aleacion_bronce")), Err(ErrorEpica::SinLogro));
        let mut cerrada = t.clone();
        let civicas = tec(&r, "instituciones_civicas");
        cerrada.logros[civicas as usize] = Some(0);
        assert_eq!(s.empezar_epica(&r, &cerrada, 10, 1, 1, civicas), Err(ErrorEpica::EraCerrada));
        let mut aparecida = t.clone();
        aparecida.hacer_aparecer(&r, 1, cobre, 0);
        assert_eq!(s.empezar_epica(&r, &aparecida, 10, 1, 1, cobre), Err(ErrorEpica::YaAparecida));
        assert_eq!(s.empezar_epica(&r, &t, 10, 1, 1, tec(&r, "leva_comunal")), Err(ErrorEpica::SinEpica));
        assert_eq!(s.empezar_epica(&r, &t, 10, 1, 9, cobre), Err(ErrorEpica::NoViveAqui));
        assert_eq!(s.empezar_epica(&r, &t, 11, 1, 1, cobre), Err(ErrorEpica::NoViveAqui), "vive en otra plaza");
        s.empezar_epica(&r, &t, 10, 1, 1, cobre).unwrap();
        assert_eq!(s.empezar_epica(&r, &t, 10, 1, 1, cobre), Err(ErrorEpica::YaCanta));
        s.aedas.push(AedaResidente { id: 2, plaza: 10, faccion: 1, llegada_en: 0, epica: None });
        assert_eq!(s.empezar_epica(&r, &t, 10, 1, 2, cobre), Err(ErrorEpica::OtroLaCanta));
    }

    #[test]
    fn lo_que_cuenta_defensa_y_conquista_en_combate_batalla_entre_distintas_obra_ascenso_adopcion_y_caravana_con_carga() {
        let r = r();
        assert_eq!(Hecho::defensa(true, 10, 1, 5).map(|h| (h.tipo, h.plaza)), Some((HechoEpico::Defensa, Some(10))));
        assert_eq!(Hecho::conquista(true, 10, 2, 5).map(|h| (h.tipo, h.faccion, h.plaza)), Some((HechoEpico::Conquista, 2, None)));
        assert_eq!(Hecho::batalla((7, 1), (8, 2), false, 5).map(|h| (h.tipo, h.faccion)), Some((HechoEpico::Batalla, 2)));
        let obra = Hecho::obra(10, 1, 3, Tipo::Fundicion, None);
        assert_eq!((obra.tipo, obra.edificio, obra.plaza, obra.clave), (HechoEpico::Obra, Some(Tipo::Fundicion), Some(10), Clave::Obra { plaza: 10, edificio: 3, nivel: 0 }));
        assert_eq!(Hecho::ascenso(10, 1, 3).tipo, HechoEpico::Ascenso);
        assert_eq!(Hecho::adopcion(1, 2).tipo, HechoEpico::Adopcion);
        assert_eq!(Hecho::caravana(&r, 1, true, 10, Some(1), 100_000).map(|h| h.tipo), Some(HechoEpico::Caravana));
    }

    #[test]
    fn las_batallas_de_unity_cuentan_por_su_id_campo_abierto_asedio_resistido_y_conquistado() {
        let campo = Hecho::de_unity(1, ContextoUnity::CampoAbierto, Some(2), Some(1), Some(false)).unwrap();
        assert_eq!((campo.tipo, campo.faccion, campo.clave), (HechoEpico::Batalla, 1, Clave::Unity { tipo: HechoEpico::Batalla, batalla: 1 }));
        let defensa = Hecho::de_unity(1, ContextoUnity::Asedio(10), Some(2), Some(1), Some(false)).unwrap();
        assert_eq!((defensa.tipo, defensa.faccion, defensa.plaza), (HechoEpico::Defensa, 1, Some(10)));
        let conquista = Hecho::de_unity(1, ContextoUnity::Asedio(10), Some(2), Some(1), Some(true)).unwrap();
        assert_eq!((conquista.tipo, conquista.faccion), (HechoEpico::Conquista, 2));
        assert!(Hecho::de_unity(1, ContextoUnity::Otro, Some(2), Some(1), Some(true)).is_none(), "caravana o campamento de bandidos");
        assert!(Hecho::de_unity(1, ContextoUnity::CampoAbierto, None, Some(1), Some(false)).is_none(), "sin Facción en un bando (bandidos)");
        assert!(Hecho::de_unity(1, ContextoUnity::CampoAbierto, Some(2), Some(1), None).is_none(), "sin ganador");
    }

    #[test]
    fn lo_que_no_asedio_sin_combate_batalla_contra_la_propia_faccion_caravana_ligera() {
        let r = r();
        assert!(Hecho::defensa(false, 10, 1, 5).is_none() && Hecho::conquista(false, 10, 2, 5).is_none());
        assert!(Hecho::batalla((7, 1), (8, 1), true, 5).is_none());
        assert!(Hecho::caravana(&r, 1, true, 10, Some(1), 1_000).is_none());
        assert!(Hecho::caravana(&r, 1, false, 10, Some(1), 500_000).is_none(), "solo las comerciales");
        assert!(Hecho::caravana(&r, 1, true, 10, None, 500_000).is_none(), "origen sin dueño");
    }

    fn caravana(id: u64) -> Hecho {
        Hecho::caravana(&r(), id, true, 10, Some(1), 500_000).unwrap()
    }

    #[test]
    fn un_hecho_del_tipo_del_capitulo_lo_cierra_y_pasa_al_siguiente_uno_de_otro_tipo_o_de_otra_plaza_no_cuenta() {
        let r = r();
        let cobre = tec(&r, "metalurgia_cobre");
        let mut t = tecnologia();
        let e0 = con_residente(Some(Epica::nueva(cobre)));
        let mut s = e0.clone();
        assert!(s.avanzar_epicas(&r, &mut t, &[Hecho::obra(10, 1, 1, Tipo::Armeria, None)], de_10, 10).is_empty());
        assert!(s.avanzar_epicas(&r, &mut t, &[Hecho::obra(11, 1, 1, Tipo::Fundicion, None)], de_10, 10).is_empty());
        assert_eq!(s, e0);
        let av = s.avanzar_epicas(&r, &mut t, &[Hecho::obra(10, 1, 1, Tipo::Fundicion, None)], de_10, 10);
        assert_eq!(av, vec![Avance::Capitulo { aeda: 1, plaza: 10, faccion: 1, tec: cobre, capitulo: 1, capitulos: 3 }]);
        let e = s.aedas[0].epica.as_ref().unwrap();
        assert_eq!((e.capitulo, e.hechos, e.ultimo_hecho_en), (1, 0, Some(10)));
    }

    #[test]
    fn el_mismo_hecho_no_cuenta_dos_veces_y_entre_hechos_hay_que_esperar_el_enfriamiento() {
        let r = r();
        let cobre = tec(&r, "metalurgia_cobre");
        let enfr = r.aedas.enfriamiento_epica;
        let mut t = tecnologia();
        let mut s = con_residente(Some(Epica { capitulo: 1, ..Epica::nueva(cobre) }));
        assert_eq!(s.avanzar_epicas(&r, &mut t, &[caravana(1)], de_10, 10), vec![Avance::Hecho { aeda: 1 }]);
        let antes = s.clone();
        assert!(s.avanzar_epicas(&r, &mut t, &[caravana(2)], de_10, 10 + enfr - 1).is_empty(), "aún en enfriamiento");
        assert!(s.avanzar_epicas(&r, &mut t, &[caravana(1)], de_10, 10 + enfr).is_empty(), "misma clave");
        assert_eq!(s, antes);
        s.avanzar_epicas(&r, &mut t, &[caravana(2)], de_10, 10 + enfr);
        let e = s.aedas[0].epica.as_ref().unwrap();
        assert_eq!((e.capitulo, e.hechos), (2, 0));
    }

    #[test]
    fn no_acredita_hechos_a_la_faccion_vieja_mientras_la_plaza_ha_cambiado_de_dueno() {
        let r = r();
        let mut t = tecnologia();
        let mut s = con_residente(Some(Epica::nueva(tec(&r, "metalurgia_cobre"))));
        let conquistada = |p| (p == 10).then_some(2);
        assert!(s.avanzar_epicas(&r, &mut t, &[Hecho::obra(10, 1, 1, Tipo::Fundicion, None)], conquistada, 10).is_empty());
    }

    #[test]
    fn al_cerrar_el_ultimo_capitulo_la_tecnologia_aparece_sin_hito_y_la_faccion_suma_una_epica() {
        let r = r();
        let cobre = tec(&r, "metalurgia_cobre");
        let mut t = tecnologia();
        let mut s = con_residente(Some(Epica { capitulo: 2, ..Epica::nueva(cobre) }));
        let av = s.avanzar_epicas(&r, &mut t, &[Hecho::ascenso(10, 1, 3)], de_10, 10);
        assert_eq!(av, vec![Avance::Cumplida { aeda: 1, plaza: 10, faccion: 1, tec: cobre, capitulos: 3, primera: true }]);
        assert!(s.aedas[0].epica.is_none());
        assert_eq!(s.cumplidas_de(1), 1);
        assert!(t.de(&r, 1).aparecidas.contiene(cobre) && !t.de(&r, 1).adoptadas.contiene(cobre));
        assert_eq!(t.primeros[cobre as usize], Some(Primero { faccion: 1, en: 10 }), "un descubrimiento es un descubrimiento");
    }

    #[test]
    fn un_hecho_de_faccion_inspira_al_aeda_de_cualquiera_de_sus_plazas() {
        // La adopción del cobre cierra el último capítulo de la cantería (la prueba de `comandosTecnologia` por la ruta real, en reglas).
        let r = r();
        let canteria = tec(&r, "canteria");
        let mut t = tecnologia();
        t.logros[canteria as usize] = Some(0);
        let mut s = con_residente(Some(Epica { capitulo: 2, ..Epica::nueva(canteria) }));
        let av = s.avanzar_epicas(&r, &mut t, &[Hecho::adopcion(2, tec(&r, "metalurgia_cobre")), Hecho::adopcion(1, tec(&r, "metalurgia_cobre"))], de_10, 0);
        assert!(matches!(av[..], [Avance::Cumplida { tec, faccion: 1, .. }] if tec == canteria));
        assert!(t.de(&r, 1).aparecidas.contiene(canteria));
    }

    #[test]
    fn las_claves_recordadas_son_las_ultimas() {
        let r = r();
        let cobre = tec(&r, "metalurgia_cobre");
        let mut t = tecnologia();
        // Capítulo de caravanas de 2: se cuentan 2 y pasa al siguiente; se fuerza a seguir contando volviendo a él para llenar la memoria.
        let mut s = con_residente(Some(Epica { capitulo: 1, ..Epica::nueva(cobre) }));
        for i in 0..(r.aedas.claves_recordadas as u64 + 5) {
            let e = s.aedas[0].epica.as_mut().unwrap();
            e.capitulo = 1;
            e.hechos = 0;
            s.avanzar_epicas(&r, &mut t, &[caravana(i)], de_10, i as i64 * r.aedas.enfriamiento_epica);
        }
        let e = s.aedas[0].epica.as_ref().unwrap();
        assert_eq!(e.claves.len(), r.aedas.claves_recordadas);
        assert_eq!(e.claves.front(), Some(&Clave::Caravana(5)));
    }
}
