//! El sigilo de una Facción (Diplomacia_Facciones §3.8, Doc 2.8.1): ocho piezas de un catálogo cerrado (`CATALOGO_SIGILO`). Se elige al crear y no cambia nunca.
//! Vale si todas las piezas son del catálogo y los dos colores del fondo son distintos; solo se rechaza el duplicado **exacto** (el color de la orla solo cuenta
//! si hay orla). Sin elección, `sigilo_libre` reparte uno estable desde una semilla con FNV-1a de 32 bits, como `engine/sigilo.ts`.

use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Sigilo {
    pub forma: String,
    pub campo: String,
    pub emblema: String,
    pub color_primario: String,
    pub color_secundario: String,
    pub color_emblema: String,
    pub orla: String,
    pub color_orla: String,
}

/// Las piezas del catálogo (`CATALOGO_SIGILO`), en su orden: el orden importa para `sigilo_libre`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Catalogo {
    pub version: u32,
    pub formas: Vec<String>,
    pub campos: Vec<String>,
    pub emblemas: Vec<String>,
    pub orlas: Vec<String>,
    /// Ids de los colores.
    pub colores: Vec<String>,
}

/// Por qué no vale un sigilo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MotivoSigilo {
    /// Alguna pieza no es del catálogo, o el fondo repite color.
    Invalido,
    /// Otra Facción lleva uno idéntico.
    Duplicado,
}

/// La orla que no dibuja nada: con ella, el color de la orla no distingue un sigilo de otro.
pub const SIN_ORLA: &str = "ninguna";

impl Catalogo {
    pub fn de(v: &Value) -> Self {
        let ids = |k: &str| -> Vec<String> { v[k].as_array().unwrap_or_else(|| panic!("CATALOGO_SIGILO.{k}")).iter().map(|x| x.as_str().expect("id").to_string()).collect() };
        Self {
            version: v["version"].as_u64().expect("version") as u32,
            formas: ids("formas"),
            campos: ids("campos"),
            emblemas: ids("emblemas"),
            orlas: ids("orlas"),
            colores: v["colores"].as_array().expect("colores").iter().map(|c| c["id"].as_str().expect("color").to_string()).collect(),
        }
    }

    /// ¿Es del catálogo? Los sigilos reservados del servidor (sin Facción, bandidos, mercenarios) llevan emblemas que no están en él.
    pub fn es_del_catalogo(&self, s: &Sigilo) -> bool {
        let en = |l: &[String], x: &str| l.iter().any(|y| y == x);
        en(&self.formas, &s.forma)
            && en(&self.campos, &s.campo)
            && en(&self.emblemas, &s.emblema)
            && en(&self.orlas, &s.orla)
            && [&s.color_primario, &s.color_secundario, &s.color_emblema, &s.color_orla].iter().all(|c| en(&self.colores, c))
            && s.color_primario != s.color_secundario
    }

    /// `motivoSigiloRechazado`: `None` si vale.
    pub fn motivo_rechazo<'a>(&self, s: &Sigilo, mut ocupados: impl Iterator<Item = &'a Sigilo>) -> Option<MotivoSigilo> {
        if !self.es_del_catalogo(s) {
            return Some(MotivoSigilo::Invalido);
        }
        ocupados.any(|o| mismo_sigilo(o, s)).then_some(MotivoSigilo::Duplicado)
    }

    /// Un sigilo libre a partir de `semilla` (el id de la Facción en el contrato, `f-N`): el mismo id da el mismo mientras esté libre; si no, el
    /// siguiente. El catálogo da cientos de millones de combinaciones, así que la búsqueda termina.
    pub fn sigilo_libre<'a>(&self, semilla: &str, ocupados: impl Iterator<Item = &'a Sigilo> + Clone) -> Sigilo {
        let k = self.colores.len() as u64;
        let combinaciones = [self.formas.len(), self.campos.len(), self.emblemas.len(), self.orlas.len()].iter().map(|&x| x as u64).product::<u64>() * k * (k - 1) * k * k;
        let h = hash32(semilla) as u64;
        for paso in 0..combinaciones {
            let mut n = (h + paso) % combinaciones;
            let mut saca = |modulo: usize| {
                let resto = (n % modulo as u64) as usize;
                n /= modulo as u64;
                resto
            };
            let forma = saca(self.formas.len());
            let campo = saca(self.campos.len());
            let emblema = saca(self.emblemas.len());
            let orla = saca(self.orlas.len());
            let primario = saca(k as usize);
            // El secundario es cualquiera de los otros: se salta el primario.
            let secundario = saca(k as usize - 1);
            let color_emblema = saca(k as usize);
            let color_orla = saca(k as usize);
            let candidato = Sigilo {
                forma: self.formas[forma].clone(),
                campo: self.campos[campo].clone(),
                emblema: self.emblemas[emblema].clone(),
                color_primario: self.colores[primario].clone(),
                color_secundario: self.colores[if secundario >= primario { secundario + 1 } else { secundario }].clone(),
                color_emblema: self.colores[color_emblema].clone(),
                orla: self.orlas[orla].clone(),
                color_orla: self.colores[color_orla].clone(),
            };
            if !ocupados.clone().any(|o| mismo_sigilo(o, &candidato)) {
                return candidato;
            }
        }
        panic!("no queda ningún sigilo libre en el catálogo")
    }
}

/// Iguales en todas las piezas; sin orla, el color de la orla no se ve y no cuenta.
pub fn mismo_sigilo(a: &Sigilo, b: &Sigilo) -> bool {
    a.forma == b.forma
        && a.campo == b.campo
        && a.emblema == b.emblema
        && a.color_primario == b.color_primario
        && a.color_secundario == b.color_secundario
        && a.color_emblema == b.color_emblema
        && a.orla == b.orla
        && (a.orla == SIN_ORLA || a.color_orla == b.color_orla)
}

/// FNV-1a de 32 bits sobre las unidades UTF-16 del texto (`charCodeAt`), como el TypeScript.
pub fn hash32(texto: &str) -> u32 {
    texto.encode_utf16().fold(0x811c_9dc5u32, |h, c| (h ^ c as u32).wrapping_mul(0x0100_0193))
}

#[cfg(test)]
mod pruebas {
    //! Casos de `typescript-final:src/engine/__tests__/sigilo.test.ts`.
    use super::*;
    use crate::datos::Reglas;

    fn valido() -> Sigilo {
        Sigilo {
            forma: "clasico".into(),
            campo: "liso".into(),
            emblema: "toro".into(),
            color_primario: "rojo".into(),
            color_secundario: "oro".into(),
            color_emblema: "blanco".into(),
            orla: "ninguna".into(),
            color_orla: "negro".into(),
        }
    }

    fn con(f: impl FnOnce(&mut Sigilo)) -> Sigilo {
        let mut s = valido();
        f(&mut s);
        s
    }

    fn cat() -> Catalogo {
        Reglas::cargar().sigilo
    }

    #[test]
    fn acepta_uno_del_catalogo_que_nadie_lleva() {
        assert_eq!(cat().motivo_rechazo(&valido(), [].iter()), None);
        assert_eq!(cat().version, 4);
    }

    #[test]
    fn rechaza_ids_fuera_del_catalogo_colores_repetidos_y_los_reservados() {
        let c = cat();
        for s in [
            con(|s| s.campo = "inventado".into()),
            con(|s| s.forma = "inventada".into()),
            con(|s| s.orla = "inventada".into()),
            con(|s| s.color_emblema = "inventado".into()),
            con(|s| s.color_secundario = "rojo".into()),
            // El reservado de los bandidos: emblema «bandidos», que no está en el catálogo.
            con(|s| {
                s.emblema = "bandidos".into();
                s.color_primario = "negro".into();
                s.color_secundario = "rojo".into();
            }),
        ] {
            assert_eq!(c.motivo_rechazo(&s, [].iter()), Some(MotivoSigilo::Invalido), "{s:?}");
        }
    }

    #[test]
    fn rechaza_solo_el_duplicado_exacto() {
        let c = cat();
        let v = valido();
        assert_eq!(c.motivo_rechazo(&v, [v.clone()].iter()), Some(MotivoSigilo::Duplicado));
        assert_eq!(c.motivo_rechazo(&v, [con(|s| s.color_secundario = "blanco".into())].iter()), None);
        assert_eq!(c.motivo_rechazo(&v, [con(|s| s.forma = "aspis".into())].iter()), None);
        assert_eq!(c.motivo_rechazo(&v, [con(|s| s.color_emblema = "negro".into())].iter()), None);
    }

    #[test]
    fn sin_orla_el_color_de_la_orla_no_distingue_con_orla_si() {
        let c = cat();
        assert_eq!(c.motivo_rechazo(&valido(), [con(|s| s.color_orla = "verde".into())].iter()), Some(MotivoSigilo::Duplicado));
        let con_orla = con(|s| s.orla = "greca".into());
        let mut otro = con_orla.clone();
        otro.color_orla = "verde".into();
        assert_eq!(c.motivo_rechazo(&con_orla, [otro].iter()), None);
        assert_eq!(c.motivo_rechazo(&con_orla, [con_orla.clone()].iter()), Some(MotivoSigilo::Duplicado));
    }

    #[test]
    fn sigilo_libre_es_determinista_valido_y_salta_los_ocupados() {
        let c = cat();
        let primero = c.sigilo_libre("faccion-1", [].iter());
        assert_eq!(c.sigilo_libre("faccion-1", [].iter()), primero);
        assert_eq!(c.motivo_rechazo(&primero, [].iter()), None);
        let segundo = c.sigilo_libre("faccion-1", [primero.clone()].iter());
        assert_ne!(segundo, primero);
        assert_eq!(c.motivo_rechazo(&segundo, [primero].iter()), None);
    }

    #[test]
    fn trescientos_ids_dan_trescientos_sigilos_validos_y_distintos() {
        let c = cat();
        let mut ocupados: Vec<Sigilo> = vec![];
        for i in 0..300 {
            let s = c.sigilo_libre(&format!("faccion-{i}"), ocupados.iter());
            ocupados.push(s);
        }
        assert_eq!(ocupados.iter().collect::<std::collections::BTreeSet<_>>().len(), 300);
        assert!(ocupados.iter().all(|s| c.motivo_rechazo(s, [].iter()).is_none()));
    }

    #[test]
    fn fnv1a_y_el_reparto_coinciden_con_el_typescript() {
        // Vectores de FNV-1a de 32 bits y el sigilo que da el algoritmo del TypeScript para «faccion-1» (calculado aparte, en Python).
        assert_eq!(hash32(""), 0x811c_9dc5);
        assert_eq!(hash32("a"), 0xe40c_292c);
        assert_eq!(hash32("faccion-1"), HASH_FACCION_1);
        assert_eq!(cat().sigilo_libre("faccion-1", [].iter()), sigilo_faccion_1());
    }

    const HASH_FACCION_1: u32 = 0x000e_5a5a;
    fn sigilo_faccion_1() -> Sigilo {
        Sigilo {
            forma: "ocho".into(),
            campo: "bordura".into(),
            emblema: "anfora".into(),
            color_primario: "azul".into(),
            color_secundario: "verde".into(),
            color_emblema: "rojo".into(),
            orla: "dentada".into(),
            color_orla: "rojo".into(),
        }
    }
}
