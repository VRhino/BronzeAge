//! La experiencia y el nivel de la Facción, su capital, su derrota y el nombre de sus plazas (sistema 5, tanda 5C; Diplomacia_Facciones §3.5, §3.7 y §8, Fundacion_Y_Refundacion §8,
//! Economia_Construccion §10.1). Las reglas puras son de `politica` (`nivel`, `capital`, `nombres`); aquí, cuándo ocurre cada hecho y lo que arrastra.
//!
//! # La experiencia se da al ocurrir el hecho
//! [`Nucleo::dar_experiencia`] suma la cuantía de `NIVEL_FACCION.xp` (en milésimas) en el instante del hecho y, si cruza un umbral, emite `faccion.nivel_subio` en ese mismo instante; nada
//! espera en la cola. Los hechos: un combate con números digno (columna contra columna y asedio, como el TypeScript), por héroe que combatió, a la Facción de cada héroe; una batalla de Unity,
//! por héroe que participó y sin filtro; la conquista de una plaza; un campamento de bandidos destruido (por campamento, dividido por el nivel, nada desde `nivelSinXpBandidos`); un edificio
//! completado; el ascenso de una plaza (30 × el nivel alcanzado); una plaza fundada que no es la primera; y un trueque cumplido entre Facciones distintas (a cada lado).
//!
//! # El nivel se deriva
//! No se guarda: [`Nucleo::nivel_de_faccion`] lo saca de la experiencia cada vez (cap de fundación, cupo de los niveles 2 y 3, huecos de políticas del Gobernador, la Gran Fundición).
//! `FijarNivelFaccion` (administración, decisión 9) fija la experiencia al umbral del nivel que pide.
//!
//! # La capital se calcula al paso
//! La designada vigente (la marca de `DesignarCapital` con un Palacio activo) o, si no, la plaza viva más antigua de la Facción. El paso de minuto de cada plaza la busca entre las plazas de su
//! Facción (el índice `Espacio::plazas_por_faccion`: como mucho las del cap) y mete en su `Exterior` si es la capital, la distancia a ella (que sube su mantenimiento) y, en la capital, si la
//! Facción ya tiene Gran Fundición.

use super::*;
use crate::estado::Dato;
use asentamiento::edificio::Edificios;
use asentamiento::Tipo;
use politica::autorizacion::Actor;
use politica::capital::{self, ErrorCapital, PlazaDeCapital};
use politica::nivel::{self as reglas_nivel, Hecho};

/// Por qué gana la Facción experiencia: el `motivo` del hecho `faccion.experiencia`.
pub(super) mod motivo {
    pub const TRUEQUE: i64 = 1;
    pub const COMBATE: i64 = 2;
    pub const BATALLA: i64 = 3;
    pub const EDIFICIO: i64 = 4;
    pub const CONQUISTA: i64 = 5;
    pub const ASCENSO: i64 = 6;
    pub const FUNDACION: i64 = 7;
    pub const BANDIDOS: i64 = 8;
}

fn rechazo_de_capital(e: ErrorCapital) -> (&'static str, String) {
    use rechazos::*;
    match e {
        ErrorCapital::PlazaAjena => (CAPITAL_PLAZA_AJENA, "la plaza no es de la Facción".into()),
        ErrorCapital::SinPalacio => (CAPITAL_SIN_PALACIO, "la plaza no tiene un Palacio activo".into()),
        ErrorCapital::YaEsLaCapital => (CAPITAL_YA_ES_LA_CAPITAL, "la plaza ya es la capital".into()),
        ErrorCapital::Cooldown { libre_en } => (CAPITAL_COOLDOWN, format!("la capital designada sigue en pie: se podrá trasladar desde {libre_en}")),
    }
}

impl Nucleo {
    // ───────────────────────────── Nivel y experiencia ─────────────────────────────

    /// El nivel de la Facción, derivado de su experiencia (1 si no existe como entidad).
    pub fn nivel_de_faccion(&self, faccion: u32) -> u32 {
        self.estado.politica.facciones.get(faccion).map_or(1, |f| f.nivel(&self.estado.reglas.economia.politica))
    }

    /// La Facción gana `delta` milésimas de experiencia por `motivo` (lo ≤ 0, o una Facción que no existe como entidad, no da nada): `faccion.experiencia` y, si cruza un umbral,
    /// `faccion.nivel_subio` en el mismo instante.
    pub(super) fn dar_experiencia(&mut self, t: Instante, faccion: u32, delta: Mil, motivo: i64) {
        if delta <= 0 || !self.estado.politica.facciones.existe(faccion) {
            return;
        }
        let r = self.estado.reglas.economia.politica.clone();
        let sube = self.estado.politica.facciones.aplicar_experiencia(&r, faccion, delta);
        self.tr.tocar(Clase::Faccion, u64::from(faccion));
        let total = self.estado.politica.facciones.get(faccion).map_or(0, |f| f.experiencia);
        self.emitir_propio(t, "faccion.experiencia", vec![], vec![("faccion", i64::from(faccion)), ("motivo", motivo), ("cantidad", delta), ("experiencia", total)]);
        if let Some(nivel) = sube {
            self.emitir_propio(t, "faccion.nivel_subio", vec![], vec![("faccion", i64::from(faccion)), ("nivel", i64::from(nivel))]);
        }
    }

    /// La experiencia de un hecho con su cuantía de `NIVEL_FACCION.xp`.
    pub(super) fn dar_experiencia_por(&mut self, t: Instante, faccion: u32, hecho: Hecho, motivo: i64) {
        let delta = reglas_nivel::xp_de(&self.estado.reglas.economia.politica.nivel, hecho);
        self.dar_experiencia(t, faccion, delta, motivo);
    }

    /// Un combate con números, **sea cual sea el objetivo** (5C-3, decisión del autor del 2026-10-24: columna contra columna, asedio, saqueo de un pueblo y ataque a una avanzada, que
    /// resuelven con un `Informe`): si fue digno, a la Facción de cada héroe que combatió, `combate` por héroe (§3.5; el TypeScript daba a la del bando, que con números es la misma).
    pub(super) fn experiencia_de_combate(&mut self, t: Instante, informe: &combate::resolucion::Informe) {
        let ratio = self.estado.reglas.economia.politica.nivel.ratio_combate_digno;
        let heroes = informe.atacante.heroes.iter().chain(&informe.defensor.heroes).copied().collect();
        self.experiencia_si_digno(t, informe.digno(ratio), heroes);
    }

    /// El asalto a una caravana con números (5C-3): digno si el poder del más débil llega a `ratioCombateDigno` del del más fuerte (`politica::nivel::combate_digno`, el poder del atacante contra
    /// la defensa de la caravana). La caravana no lleva héroes: solo ganan los del atacante.
    pub(super) fn experiencia_de_asalto(&mut self, t: Instante, heroes: &[u64], poder_atacante: Mil, defensa: Mil) {
        let digno = reglas_nivel::combate_digno(&self.estado.reglas.economia.politica.nivel, poder_atacante, defensa);
        self.experiencia_si_digno(t, digno, heroes.to_vec());
    }

    fn experiencia_si_digno(&mut self, t: Instante, digno: bool, heroes: Vec<u64>) {
        if digno {
            self.experiencia_por_heroes(t, heroes, |heroes| Hecho::Combate { heroes, digno: true }, motivo::COMBATE);
        }
    }

    /// Una batalla de Unity: a la Facción de cada héroe que participó (la del ticket), `combate` por héroe, sin filtro de dignidad (§3.5, `resultadoBatalla.ts`).
    pub(super) fn experiencia_de_batalla(&mut self, t: Instante, por_faccion: std::collections::BTreeMap<u32, u32>) {
        for (f, n) in por_faccion {
            self.dar_experiencia_por(t, f, Hecho::BatallaUnity { heroes: n }, motivo::BATALLA);
        }
    }

    fn experiencia_por_heroes(&mut self, t: Instante, heroes: Vec<u64>, hecho: impl Fn(u32) -> Hecho, motivo: i64) {
        let mut por_faccion: std::collections::BTreeMap<u32, u32> = std::collections::BTreeMap::new();
        for f in heroes.into_iter().filter_map(|h| self.heroe(h).and_then(|x| x.faccion)) {
            *por_faccion.entry(f).or_insert(0) += 1;
        }
        for (f, n) in por_faccion {
            self.dar_experiencia_por(t, f, hecho(n), motivo);
        }
    }

    /// Un campamento de bandidos destruido por la Facción `faccion`: por campamento, `bandidos ÷ nivel` y nada desde `nivelSinXpBandidos` (§3.5).
    pub(super) fn experiencia_de_bandidos(&mut self, t: Instante, faccion: Option<u32>) {
        let Some(f) = faccion else { return };
        let nivel_faccion = self.nivel_de_faccion(f);
        self.dar_experiencia_por(t, f, Hecho::Bandidos { nivel_faccion }, motivo::BANDIDOS);
    }

    // ───────────────────────────── La derrota ─────────────────────────────

    /// `registrarDerrota` (§8): la última plaza de `perdedora` acaba de caer, conquistada por `ganadora` o en ruinas (`None`).
    pub(super) fn registrar_derrota(&mut self, perdedora: u32, ganadora: Option<u32>) {
        self.estado.politica.facciones.registrar_derrota(perdedora, ganadora);
        self.tr.tocar(Clase::Faccion, u64::from(perdedora));
    }

    // ───────────────────────────── La capital ─────────────────────────────

    /// Lo que la regla de la capital mira de las plazas de la Facción, sin reservar memoria. El Palacio solo se mira donde hace falta: en la plaza con la marca y, si `todas`, en
    /// todas (designar).
    fn plazas_de_capital(&self, faccion: u32, todas: bool) -> impl Iterator<Item = PlazaDeCapital> + Clone + '_ {
        self.plazas_de(faccion).map(move |a| PlazaDeCapital {
            id: a.id as u32,
            faccion,
            desde: a.de_la_faccion_desde,
            palacio_activo: (todas || a.capital_de == Some(faccion)) && a.economia.edificios.activos(Tipo::Palacio) > 0,
            marca: a.capital_de,
        })
    }

    /// La capital efectiva de la Facción (`encontrarCapital`): la designada vigente o la plaza viva más antigua. `None` si no le queda ninguna.
    pub fn capital_de(&self, faccion: u32) -> Option<u64> {
        capital::efectiva(faccion, self.plazas_de_capital(faccion, false)).map(u64::from)
    }

    /// Lo que el paso de la plaza `id` mira de su capital: si es la capital, la distancia a ella en milésimas (0 si lo es) y si la Facción tiene ya Gran Fundición en alguna plaza, en
    /// cualquier estado (`faccionYaTieneGranFundicion`: «ya tiene, o está levantando»). Esto último solo se busca donde cuenta: en la capital de una Facción con el nivel que la pide
    /// (fuera de ahí no se puede construir de todas formas). Una Facción de una sola plaza no busca su capital: es ella. Es el coste de 5C en el paso de minuto.
    pub(super) fn exterior_de_capital(&self, id: u64, nivel_faccion: u32) -> (bool, Mil, bool) {
        let a = &self.estado.asentamientos[&id];
        let sola = self.espacio.plazas_por_faccion.get(&a.faccion).is_none_or(|s| s.len() <= 1);
        let capital = if sola { Some(id) } else { self.capital_de(a.faccion) };
        match capital {
            Some(c) if c != id => (false, a.posicion.dist(self.estado.asentamientos[&c].posicion), false),
            _ => {
                let minimo = self.estado.reglas.economia.datos.catalogo.tipo(Tipo::GranFundicion).nivel_faccion_minimo;
                let cuenta = minimo.is_none_or(|m| nivel_faccion >= m);
                (true, 0, cuenta && self.plazas_de(a.faccion).any(|p| p.economia.edificios.en_cualquier_estado(Tipo::GranFundicion) > 0))
            }
        }
    }

    // ───────────────────────────── El nombre ─────────────────────────────

    /// El nombre de una plaza nueva (`nombreDeCiudadLibre`): uno de la lista que ninguna plaza lleve ya, por su id (`as-N`). Vacío si se agotaron.
    pub(super) fn nombre_de_ciudad_libre(&self, id: u64) -> String {
        politica::nombres::ciudad_libre(self.estado.asentamientos.values().map(|a| a.nombre.as_str()), &ids::asentamiento(id)).unwrap_or_default().to_owned()
    }

    // ───────────────────────────── Órdenes ─────────────────────────────

    /// `DesignarCapital` y `RenombrarAsentamiento`. Los rechazos y «sin cambios» salen como `Err` sin haber tocado el estado.
    pub(super) fn ejecutar_capital_y_nombre(&mut self, t: Instante, p: &Peticion) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        match &p.comando {
            Comando::DesignarCapital { heroe, faccion, asentamiento } => {
                self.autorizar_heroe(p, *heroe, false)?;
                if !self.estado.politica.facciones.existe(*faccion) {
                    return Err(rechazo(rechazos::FACCION_NO_EXISTE, "la Facción no existe".into()));
                }
                if !(Actor { yo: *heroe, politica: &self.estado.politica, vista: self }).rey_de_faccion(*faccion) {
                    return Err(rechazo(rechazos::NO_AUTORIZADO, "solo el Rey designa la capital".into()));
                }
                self.plaza_o_rechazo(*asentamiento)?;
                let (r, plazas) = (self.estado.reglas.economia.politica.clone(), self.plazas_de_capital(*faccion, true).collect::<Vec<_>>());
                let quitar = self.estado.politica.facciones.designar_capital(&r, *faccion, &plazas, *asentamiento as u32, t).expect("la Facción existe").map_err(|e| {
                    let (c, m) = rechazo_de_capital(e);
                    rechazo(c, m)
                })?;
                for q in quitar.into_iter().map(u64::from) {
                    self.estado.asentamientos.get_mut(&q).expect("es de la Facción").capital_de = None;
                    self.tr.tocar(Clase::Asentamiento, q);
                }
                self.estado.asentamientos.get_mut(asentamiento).expect("existe").capital_de = Some(*faccion);
                self.tr.tocar(Clase::Asentamiento, *asentamiento);
                // Otra capital: `recursoEnCapital` y `capitalEnNivel` miran otra plaza (6C).
                self.mirar_aparicion(*faccion);
                self.tr.tocar(Clase::Faccion, u64::from(*faccion));
                self.emitir_propio(t, "faccion.capital_designada", vec![*asentamiento], vec![("faccion", i64::from(*faccion)), ("asentamiento", *asentamiento as i64), ("heroe", *heroe as i64)]);
                Ok(vec![])
            }
            Comando::RenombrarAsentamiento { heroe, asentamiento, nombre } => {
                self.autorizar_heroe(p, *heroe, false)?;
                self.plaza_o_rechazo(*asentamiento)?;
                if !(Actor { yo: *heroe, politica: &self.estado.politica, vista: self }).residente(*asentamiento as u32) {
                    return Err(rechazo(rechazos::NO_AUTORIZADO, "solo quien reside en la plaza le cambia el nombre".into()));
                }
                // 5C-6 (decisión del autor del 2026-10-24): recortado, como mucho `NOMBRE_ASENTAMIENTO.largoMaximo` caracteres y único entre las plazas vivas (sin tildes ni mayúsculas).
                let limpio = nombre.trim().to_owned();
                if self.estado.asentamientos[asentamiento].nombre == limpio {
                    return Err(Respuesta::sin_cambios(v));
                }
                let largo = self.estado.reglas.economia.datos.largo_maximo_nombre;
                if limpio.chars().count() > largo {
                    return Err(rechazo(rechazos::ASENTAMIENTO_NOMBRE_LARGO, format!("el nombre de una plaza tiene como mucho {largo} caracteres")));
                }
                let normal = politica::nombres::normalizar(&limpio);
                if !limpio.is_empty() && self.estado.asentamientos.values().any(|o| o.id != *asentamiento && politica::nombres::normalizar(&o.nombre) == normal) {
                    return Err(rechazo(rechazos::ASENTAMIENTO_NOMBRE_DUPLICADO, "otra plaza ya se llama así".into()));
                }
                let a = self.estado.asentamientos.get_mut(asentamiento).expect("existe");
                a.nombre = limpio.clone();
                self.tr.tocar(Clase::Asentamiento, *asentamiento);
                let punto = Some(a.posicion);
                let datos = vec![("asentamiento", *asentamiento as i64), ("heroe", *heroe as i64)];
                self.tr.emitir(&self.estado, EventoNucleo { asentamientos: vec![*asentamiento], punto, valores: vec![("nombre", Dato::Texto(limpio))], ..EventoNucleo::nuevo("asentamiento.renombrado", t, datos) });
                Ok(vec![])
            }
            _ => unreachable!("solo se llama con DesignarCapital y RenombrarAsentamiento"),
        }
    }
}
