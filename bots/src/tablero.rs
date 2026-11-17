//! El tablero de tareas de una Facción, en su pizarra: qué va a hacer cada bot, para que dos no salgan por separado a lo mismo, y qué grupo se reúne en qué puerta, para que
//! los que coinciden se le unan y salgan como un solo ejército.
//!
//! Una tarea se nombra por lo que hace y a qué (`cazar:band-3`, `campana:as-12`, `saquear:pu-4`, `explorador`). Quien la toma la lleva hasta que la suelta o caduca: un bot
//! que se desconecta o desaparece no la retiene para siempre.

use std::collections::BTreeMap;

/// Lo que alguien de la Facción va a hacer.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tarea {
    /// Quién la lleva.
    pub lider: String,
    /// Hasta cuándo vale si nadie la renueva.
    pub hasta: i64,
    /// Mientras reúne a su grupo en la puerta.
    pub reunion: Option<Reunion>,
}

/// Un grupo que espera en la puerta de una plaza o un campamento a que se le unan.
#[derive(Clone, Debug, PartialEq)]
pub struct Reunion {
    /// La plaza o el campamento.
    pub puerta: String,
    /// La columna que espera.
    pub columna: String,
    /// A qué va (para quien se une): una plaza, un campamento de bandidos o `fundar`.
    pub objetivo: String,
    /// Hasta cuándo se le puede unir uno.
    pub hasta: i64,
    /// Lo último que midió el líder: cuántos van en la columna y con qué fuerza.
    pub unidos: usize,
    pub fuerza: u32,
}

/// Lo que la Facción necesita para sobrevivir (la comida, la defensa de sus plazas y pueblos): lo apunta cada plaza al pensar, con su prioridad (1, lo primero) y quién lo
/// hace (un cargo, `libre` o `residentes`), y dura mientras se vuelva a apuntar.
#[derive(Clone, Debug, PartialEq)]
pub struct Necesidad {
    pub texto: String,
    pub prioridad: u8,
    /// La plaza que la apunta (o vacío, si es de toda la Facción).
    pub plaza: String,
    pub hasta: i64,
    /// Quién la hace: `gobernador`, `tesorero`, `rey`, `libre` (cualquiera sin cargo que esté en casa) o `residentes` (cada uno, lo suyo).
    pub quien: String,
}

/// Un paso de una meta, que hace alguien de la Facción: la tarea que toma quien lo hace (vacía si nadie puede, como lo que pide el mundo), qué es y quién lo hace (como en
/// [`Necesidad::quien`], o `mundo`: llega solo).
#[derive(Clone, Debug, PartialEq)]
pub struct PasoDeMeta {
    pub clave: String,
    pub texto: String,
    pub quien: String,
}

/// Lo que la Facción persigue para avanzar (una tecnología, el bronce, sus obras, explorar, mejor tropa), con su prioridad (1, lo primero) y los pasos con que se llega,
/// cada uno de alguien. Como las necesidades, dura mientras se vuelva a apuntar.
#[derive(Clone, Debug, PartialEq)]
pub struct Meta {
    pub texto: String,
    pub prioridad: u8,
    pub plaza: String,
    pub hasta: i64,
    pub pasos: Vec<PasoDeMeta>,
}

#[derive(Debug, Default)]
pub struct Tablero {
    tareas: BTreeMap<String, Tarea>,
    necesidades: BTreeMap<String, Necesidad>,
    metas: BTreeMap<String, Meta>,
}

impl Tablero {
    /// ¿Puede `yo` hacer `clave`? Nadie más la lleva ahora.
    pub fn libre(&self, clave: &str, yo: &str, t: i64) -> bool {
        self.tareas.get(clave).is_none_or(|x| x.lider == yo || x.hasta <= t)
    }

    /// La toma, o la renueva, hasta `hasta`. ¿La lleva? Si otro la lleva vigente, no.
    pub fn tomar(&mut self, clave: &str, yo: &str, t: i64, hasta: i64) -> bool {
        if !self.libre(clave, yo, t) {
            return false;
        }
        let x = self.tareas.entry(clave.into()).or_default();
        if x.lider != yo || x.hasta <= t {
            *x = Tarea { lider: yo.into(), ..Default::default() };
        }
        x.hasta = x.hasta.max(hasta);
        true
    }

    /// La suelta: hecha, fallida o abandonada. Solo quien la lleva.
    pub fn soltar(&mut self, clave: &str, yo: &str) {
        if self.tareas.get(clave).is_some_and(|x| x.lider == yo) {
            self.tareas.remove(clave);
        }
    }

    /// Abre, o alarga, la reunión de su grupo: toma la tarea y espera en `puerta` con `columna` hasta `hasta_reunion`. ¿La abre? No, si otro lleva la tarea o ya reúne
    /// a otro grupo en esa puerta.
    #[allow(clippy::too_many_arguments)]
    pub fn reunir(&mut self, clave: &str, yo: &str, puerta: &str, columna: &str, objetivo: &str, t: i64, hasta_reunion: i64, hasta: i64) -> bool {
        if self.reunida_en(puerta, t).is_some_and(|(c, _)| c != clave) || !self.tomar(clave, yo, t, hasta) {
            return false;
        }
        let antes = self.tareas.get(clave).and_then(|x| x.reunion.as_ref()).map_or((0, 0), |r| (r.unidos, r.fuerza));
        let r = Reunion { puerta: puerta.into(), columna: columna.into(), objetivo: objetivo.into(), hasta: hasta_reunion, unidos: antes.0, fuerza: antes.1 };
        self.tareas.get_mut(clave).unwrap().reunion = Some(r);
        true
    }

    /// El grupo que se reúne ahora en `puerta`, si hay uno: su tarea y quién lo lleva.
    pub fn reunida_en(&self, puerta: &str, t: i64) -> Option<(&str, &Tarea)> {
        self.tareas.iter().find(|(_, x)| x.hasta > t && x.reunion.as_ref().is_some_and(|r| r.puerta == puerta && r.hasta > t)).map(|(c, x)| (c.as_str(), x))
    }

    /// ¿Sigue `yo` reuniendo a su grupo en `puerta`?
    pub fn reuniendo(&self, puerta: &str, yo: &str, t: i64) -> bool {
        self.reunida_en(puerta, t).is_some_and(|(_, x)| x.lider == yo)
    }

    /// La reunión de `puerta` a la que se puede unir `yo` (no la suya): la columna y a qué va.
    pub fn a_unirse(&self, puerta: &str, yo: &str, t: i64) -> Option<(String, String)> {
        self.reunida_en(puerta, t).filter(|(_, x)| x.lider != yo).and_then(|(_, x)| x.reunion.as_ref()).map(|r| (r.columna.clone(), r.objetivo.clone()))
    }

    /// El grupo de `yo` sale (o se deshace): cierra su reunión en `puerta`. La tarea sigue tomada.
    pub fn cerrar_reunion(&mut self, puerta: &str, yo: &str) {
        for x in self.tareas.values_mut().filter(|x| x.lider == yo) {
            if x.reunion.as_ref().is_some_and(|r| r.puerta == puerta) {
                x.reunion = None;
            }
        }
    }

    /// Todas, para el panel.
    pub fn tareas(&self) -> &BTreeMap<String, Tarea> {
        &self.tareas
    }

    /// Lo que mide el líder de su grupo en `puerta`: cuántos van y con qué fuerza.
    pub fn anotar_fuerza(&mut self, puerta: &str, yo: &str, unidos: usize, fuerza: u32) {
        for r in self.tareas.values_mut().filter(|x| x.lider == yo).filter_map(|x| x.reunion.as_mut()).filter(|r| r.puerta == puerta) {
            (r.unidos, r.fuerza) = (unidos, fuerza);
        }
    }

    /// Las tareas vigentes que empiezan por `prefijo` y lleva otro que `yo`.
    pub fn de_otros(&self, prefijo: &str, yo: &str, t: i64) -> Vec<&str> {
        self.tareas.range(prefijo.to_string()..).take_while(|(k, _)| k.starts_with(prefijo)).filter(|(_, x)| x.lider != yo && x.hasta > t).map(|(k, _)| k.as_str()).collect()
    }

    /// Suelta todas las suyas que empiezan por `prefijo`.
    pub fn soltar_de(&mut self, prefijo: &str, yo: &str) {
        self.tareas.retain(|k, x| !(k.starts_with(prefijo) && x.lider == yo));
    }

    /// Suelta todo lo que lleva `yo`: ha vuelto a casa sin nada en curso o deja la Facción.
    pub fn soltar_todas(&mut self, yo: &str) {
        self.tareas.retain(|_, x| x.lider != yo);
    }

    /// Apunta (o renueva) una necesidad de la Facción. La clave es la de la tarea que la cubre: quien la toma, la cubre.
    pub fn apuntar_necesidad(&mut self, clave: &str, n: Necesidad) {
        self.necesidades.insert(clave.into(), n);
    }

    /// Las necesidades vigentes, con quién las cubre si alguien lleva su tarea.
    pub fn necesidades(&self, t: i64) -> Vec<(&str, &Necesidad, Option<&str>)> {
        let mut v: Vec<_> = self
            .necesidades
            .iter()
            .filter(|(_, n)| n.hasta > t)
            .map(|(k, n)| (k.as_str(), n, self.tareas.get(k).filter(|x| x.hasta > t).map(|x| x.lider.as_str())))
            .collect();
        v.sort_by_key(|(k, n, _)| (n.prioridad, *k));
        v
    }

    /// Apunta (o renueva) una meta de la Facción.
    pub fn apuntar_meta(&mut self, clave: &str, m: Meta) {
        self.metas.insert(clave.into(), m);
    }

    /// Quién lleva ahora la tarea `clave`.
    fn quien_lleva(&self, clave: &str, t: i64) -> Option<&str> {
        self.tareas.get(clave).filter(|x| x.hasta > t).map(|x| x.lider.as_str())
    }

    /// Las metas vigentes, por prioridad, con quién lleva cada paso.
    pub fn metas(&self, t: i64) -> Vec<(&str, &Meta, Vec<Option<&str>>)> {
        let mut v: Vec<_> = self.metas.iter().filter(|(_, m)| m.hasta > t).map(|(k, m)| (k.as_str(), m, m.pasos.iter().map(|p| self.quien_lleva(&p.clave, t)).collect())).collect();
        v.sort_by_key(|(k, m, _)| (m.prioridad, *k));
        v
    }

    /// Lo que puede hacer uno libre (sin cargo, en casa) de la plaza `plaza`, por prioridad: las necesidades `libre` sin cubrir y los pasos `libre` sin tomar de las metas,
    /// de ella o de toda la Facción. A igual prioridad, las necesidades antes. Cada tarea, una vez.
    pub fn para_libres(&self, plaza: &str, yo: &str, t: i64) -> Vec<String> {
        let de_aqui = |p: &str| p.is_empty() || p == plaza;
        let mut v: Vec<(u8, u8, String)> = self
            .necesidades
            .iter()
            .filter(|(k, n)| n.hasta > t && n.quien == "libre" && de_aqui(&n.plaza) && self.libre(k, yo, t))
            .map(|(k, n)| (n.prioridad, 0, k.clone()))
            .collect();
        for m in self.metas.values().filter(|m| m.hasta > t && de_aqui(&m.plaza)) {
            v.extend(m.pasos.iter().filter(|p| p.quien == "libre" && !p.clave.is_empty() && self.libre(&p.clave, yo, t)).map(|p| (m.prioridad, 1, p.clave.clone())));
        }
        v.sort();
        let mut claves: Vec<String> = Vec::new();
        for (_, _, k) in v {
            if !claves.contains(&k) {
                claves.push(k);
            }
        }
        claves
    }

    /// Olvida lo caducado.
    pub fn limpiar(&mut self, t: i64) {
        self.tareas.retain(|_, x| x.hasta > t);
        self.necesidades.retain(|_, n| n.hasta > t);
        self.metas.retain(|_, m| m.hasta > t);
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn una_tarea_la_lleva_uno_hasta_que_la_suelta_o_caduca() {
        let mut tb = Tablero::default();
        assert!(tb.tomar("cazar:b1", "h1", 0, 100));
        assert!(!tb.libre("cazar:b1", "h2", 50) && !tb.tomar("cazar:b1", "h2", 50, 200), "otro no la toma");
        assert!(tb.tomar("cazar:b1", "h1", 50, 200), "quien la lleva la renueva");
        assert!(tb.libre("cazar:b1", "h2", 200), "caducada, queda libre");
        tb.soltar("cazar:b1", "h2");
        assert!(!tb.libre("cazar:b1", "h2", 100), "solo la suelta quien la lleva");
        tb.soltar("cazar:b1", "h1");
        assert!(tb.libre("cazar:b1", "h2", 100));
    }

    #[test]
    fn una_puerta_reune_un_grupo_a_la_vez_y_los_demas_se_le_unen() {
        let mut tb = Tablero::default();
        assert!(tb.reunir("campana:as-9", "h1", "as-1", "ej-1", "as-9", 0, 10, 1000));
        assert_eq!(tb.a_unirse("as-1", "h2", 5), Some(("ej-1".into(), "as-9".into())));
        assert_eq!(tb.a_unirse("as-1", "h1", 5), None, "a la suya no se une");
        assert!(!tb.reunir("cazar:b1", "h2", "as-1", "ej-2", "b1", 5, 15, 1000), "la puerta está ocupada");
        assert!(tb.reuniendo("as-1", "h1", 5) && !tb.reuniendo("as-1", "h1", 10), "la reunión acaba a su hora");
        assert!(tb.reunir("campana:as-9", "h1", "as-1", "ej-1", "as-9", 10, 20, 1000), "y el líder la alarga");
        tb.cerrar_reunion("as-1", "h1");
        assert_eq!(tb.a_unirse("as-1", "h2", 12), None, "salió: ya no se une nadie");
        assert!(!tb.libre("campana:as-9", "h2", 12), "pero la tarea sigue tomada");
        assert!(tb.reunir("cazar:b1", "h2", "as-1", "ej-2", "b1", 12, 30, 1000), "y la puerta queda libre");
    }

    #[test]
    fn una_necesidad_dice_quien_la_cubre_y_caduca_si_nadie_la_renueva() {
        let mut tb = Tablero::default();
        let n = |texto: &str, prioridad| Necesidad { texto: texto.into(), prioridad, plaza: "as-1".into(), hasta: 100, quien: "libre".into() };
        tb.apuntar_necesidad("guarnecer:pu-1", n("guarnecer", 3));
        tb.apuntar_necesidad("cazar:b1", n("cazar", 2));
        tb.tomar("cazar:b1", "h1", 0, 50);
        let v = tb.necesidades(10);
        assert_eq!(v.iter().map(|(k, _, q)| (*k, *q)).collect::<Vec<_>>(), vec![("cazar:b1", Some("h1")), ("guarnecer:pu-1", None)], "por prioridad, con quién la cubre");
        tb.limpiar(100);
        assert!(tb.necesidades(100).is_empty());
    }

    #[test]
    fn cada_explorador_ve_las_casillas_de_los_demas_y_suelta_las_suyas() {
        let mut tb = Tablero::default();
        tb.tomar("casilla:1,2", "h1", 0, 100);
        tb.tomar("casilla:3,4", "h1", 0, 100);
        tb.tomar("casilla:5,6", "h2", 0, 100);
        tb.tomar("cazar:b1", "h2", 0, 100);
        assert_eq!(tb.de_otros("casilla:", "h2", 50), vec!["casilla:1,2", "casilla:3,4"]);
        assert!(tb.de_otros("casilla:", "h2", 100).is_empty(), "las caducadas no cuentan");
        tb.soltar_de("casilla:", "h1");
        tb.tomar("explorador", "h3", 0, 100);
        tb.soltar_todas("h3");
        assert!(tb.libre("explorador", "h1", 50) && !tb.libre("cazar:b1", "h1", 50), "solo suelta las suyas");
        assert_eq!(tb.de_otros("casilla:", "h3", 50), vec!["casilla:5,6"]);
    }

    #[test]
    fn uno_libre_toma_por_prioridad_lo_libre_de_necesidades_y_metas_sin_tomar() {
        let mut tb = Tablero::default();
        let n = |prioridad, quien: &str| Necesidad { texto: String::new(), prioridad, plaza: "as-1".into(), hasta: 100, quien: quien.into() };
        let paso = |clave: &str, quien: &str| PasoDeMeta { clave: clave.into(), texto: String::new(), quien: quien.into() };
        tb.apuntar_necesidad("cazar:b1", n(2, "libre"));
        tb.apuntar_necesidad("comprar:trigo", n(1, "tesorero"));
        tb.apuntar_meta("meta:obras", Meta { texto: String::new(), prioridad: 2, plaza: "as-1".into(), hasta: 100, pasos: vec![paso("obra:o1", "libre"), paso("comprar:madera", "tesorero")] });
        tb.apuntar_meta("meta:bronce", Meta { texto: String::new(), prioridad: 1, plaza: String::new(), hasta: 100, pasos: vec![paso("buscador", "libre"), paso("", "mundo")] });
        tb.apuntar_meta("meta:otra", Meta { texto: String::new(), prioridad: 1, plaza: "as-2".into(), hasta: 100, pasos: vec![paso("batida:as-2", "libre")] });
        assert_eq!(tb.para_libres("as-1", "h1", 10), vec!["buscador", "cazar:b1", "obra:o1"], "la meta de prioridad 1 antes; a igual prioridad, la necesidad; nada de otra plaza ni de un cargo");
        tb.tomar("buscador", "h2", 10, 50);
        assert_eq!(tb.para_libres("as-1", "h1", 10), vec!["cazar:b1", "obra:o1"], "lo que lleva otro, no");
        let metas = tb.metas(10);
        assert_eq!(metas[0].0, "meta:bronce");
        assert_eq!(metas[0].2, vec![Some("h2"), None], "quién lleva cada paso");
        tb.limpiar(100);
        assert!(tb.metas(10).is_empty());
    }
}
