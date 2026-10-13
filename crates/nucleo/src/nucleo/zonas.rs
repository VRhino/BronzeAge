//! Mundo y territorio sobre el núcleo (Doc 1): fundar, zonas de influencia que crecen y chocan, y parcelas de bosque.
//! Las reglas puras viven en el crate `territorio`; aquí está el estado, los eventos y las órdenes.
//!
//! La zona de una plaza es un disco cuyo radio es un [`Ritmo`] (crece cada minuto hasta el techo de su nivel). No hay
//! barrido: dos cosas se programan en la cola con la generación de la plaza, y se rehacen cuando el ritmo cambia
//! (fundar, subir de nivel, agrandar):
//! - `ZonaTecho`: el instante en que el radio llega al techo.
//! - `ZonaContacto`: para cada vecina de otra Facción, el instante exacto en que sus zonas se tocan (suma de radios = distancia).
//!   Desde ese instante la frontera dura recorta ambas zonas; el recorte es una consulta ([`Nucleo::en_zona`]), no un estado.

use super::*;
use crate::estado::{Asentamiento, PRIO_ZONA};
use base::Instante;
use cola::Clave;
use mundo::Mundo;
use std::collections::BTreeSet;
use territorio::{Disco, DistanciasDeFundacion, Motivo, Parcela, SinParcela, Terreno, Viabilidad};

/// Lo que el núcleo necesita del artefacto de mundo, ya preparado: no es estado de la partida.
pub(crate) struct Geografia {
    mundo: Arc<Mundo>,
    parcelas: Vec<Parcela>,
    total_por_masa: Vec<usize>,
    campamentos: Vec<Punto>,
    piedra: Vec<Punto>,
}

impl Geografia {
    pub(crate) fn de(mundo: Arc<Mundo>) -> Self {
        let masas: std::collections::HashMap<&str, usize> = mundo.bosques().iter().enumerate().map(|(i, b)| (b.id.as_str(), i)).collect();
        let punto = |p: &Option<contrato::v1::Punto>| p.map(|p| Punto::new(p.x, p.y)).unwrap_or_default();
        let parcelas: Vec<Parcela> = mundo
            .parcelas()
            .iter()
            .map(|p| Parcela { bosque: masas[p.bosque_id.as_str()] as u32, centro: punto(&p.centro), densidad: p.densidad_milesimas })
            .collect();
        let mut total_por_masa = vec![0; masas.len()];
        parcelas.iter().for_each(|p| total_por_masa[p.bosque as usize] += 1);
        let campamentos = mundo.campamentos().iter().map(|c| punto(&c.posicion)).collect();
        let piedra = mundo.nodos().iter().filter(|n| n.tipo == "piedra").map(|n| punto(&n.posicion)).collect();
        Self { mundo, parcelas, total_por_masa, campamentos, piedra }
    }

    fn terreno(&self, p: Punto) -> Terreno {
        use contrato::v1::ClaseTerreno as C;
        match self.mundo.terreno_en(p.x, p.y) {
            None => Terreno::FueraDelMapa,
            Some(C::TerrenoAgua) => Terreno::Agua,
            Some(C::TerrenoCima) => Terreno::Cima,
            Some(_) if !self.mundo.transitable_en(p.x, p.y) => Terreno::NoTransitable,
            Some(_) => Terreno::Firme,
        }
    }
}

/// Distancia entera redondeada hacia arriba: dos discos se tocan cuando la suma de radios llega a este valor.
fn distancia_techo(a: Punto, b: Punto) -> Mil {
    let d2 = a.dist2(b);
    let d = base::isqrt(d2);
    (if d * d < d2 { d + 1 } else { d }) as Mil
}

/// Lo que se sabe de un sitio antes de fundar (Doc 1.2: el aviso de viabilidad, que no impide fundar).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InformeViabilidad {
    pub color: Viabilidad,
    /// Por qué no se puede fundar ahí, si no se puede.
    pub motivo: Option<Motivo>,
    /// Parcelas de bosque libres cuyo centro caería en la zona con la que nacería.
    pub parcelas_libres: usize,
    /// Nodos de piedra en esa zona.
    pub piedra: usize,
}

impl Nucleo {
    // ───────────────────────────── Consultas ─────────────────────────────

    fn disco_de(&self, a: &Asentamiento, t: Instante) -> Disco {
        Disco { centro: a.posicion, radio: a.radio.valor_en(t), faccion: a.faccion }
    }

    /// Los discos de todas las plazas en `t`, por id.
    fn discos(&self, t: Instante) -> Vec<(u64, Disco)> {
        self.estado.asentamientos.values().map(|a| (a.id, self.disco_de(a, t))).collect()
    }

    /// Radio de la zona de una plaza ahora, en milésimas.
    pub fn radio_zona(&self, id: u64) -> Option<Mil> {
        self.estado.asentamientos.get(&id).map(|a| a.radio.valor_en(self.estado.ahora))
    }

    /// ¿Cae `p` en la zona real de la plaza (disco recortado por las fronteras con otras Facciones)?
    pub fn en_zona(&self, id: u64, p: Punto) -> bool {
        let t = self.estado.ahora;
        let Some(a) = self.estado.asentamientos.get(&id) else { return false };
        let yo = self.disco_de(a, t);
        let vecinos: Vec<Disco> = self.discos(t).into_iter().filter(|&(o, _)| o != id).map(|(_, d)| d).collect();
        territorio::en_zona(p, &yo, &vecinos)
    }

    /// Polígono de la zona real (presentación: `Asentamiento.zona`). 48 lados, como `ZONA_INFLUENCIA.segmentosPoligono`.
    pub fn poligono_zona(&self, id: u64) -> Vec<Punto> {
        let t = self.estado.ahora;
        let Some(a) = self.estado.asentamientos.get(&id) else { return vec![] };
        let vecinos: Vec<Disco> = self.discos(t).into_iter().filter(|&(o, _)| o != id).map(|(_, d)| d).collect();
        territorio::poligono(&self.disco_de(a, t), &vecinos, 48)
    }

    /// Plazas vigentes de una Facción.
    pub fn plazas_de(&self, faccion: u32) -> impl Iterator<Item = &Asentamiento> {
        self.estado.asentamientos.values().filter(move |a| a.faccion == faccion)
    }

    /// El aviso de viabilidad de fundar en `p` para esa Facción. `None` si el núcleo no tiene mundo cargado.
    pub fn viabilidad(&self, p: Punto, faccion: u32) -> Option<InformeViabilidad> {
        let geo = self.geografia.as_ref()?;
        let t = self.estado.ahora;
        let motivo = self.validar_fundacion(geo, p, t).err();
        let nuevo = Disco { centro: p, radio: self.estado.reglas.territorio.radio_inicial, faccion };
        // La zona con la que nacería: el círculo inicial recortado contra las plazas de otras Facciones.
        let vecinos: Vec<Disco> = self.discos(t).into_iter().map(|(_, d)| d).collect();
        let tocan: Vec<Disco> = vecinos.into_iter().filter(|d| territorio::se_tocan(&nuevo, d)).collect();
        let en_zona = |c: Punto| territorio::en_zona(c, &nuevo, &tocan);
        let parcelas_libres = geo.parcelas.iter().enumerate().filter(|(i, pa)| !self.estado.parcelas_ocupadas.contains_key(&(*i as u32)) && en_zona(pa.centro)).count();
        let piedra = geo.piedra.iter().filter(|&&c| en_zona(c)).count();
        Some(InformeViabilidad { color: territorio::viabilidad(motivo.is_none(), parcelas_libres, piedra), motivo, parcelas_libres, piedra })
    }

    fn validar_fundacion(&self, geo: &Geografia, p: Punto, t: Instante) -> Result<(), Motivo> {
        let r = &self.estado.reglas.territorio;
        let plazas: Vec<Disco> = self.discos(t).into_iter().map(|(_, d)| d).collect();
        let dist = DistanciasDeFundacion { separacion_plazas: r.separacion_plazas, exclusion_campamento: r.exclusion_campamento };
        territorio::validar(p, geo.terreno(p), &plazas, geo.campamentos.iter().copied(), &dist)
    }

    // ───────────────────────────── Eventos de la zona ─────────────────────────────

    /// Rehace los eventos de la plaza tras cambiar su ritmo: sube la generación (los viejos se descartan solos) y programa el
    /// techo y el contacto con cada vecina de otra Facción que aún no se ha tocado.
    fn reprogramar_zona(&mut self, id: u64, t: Instante) {
        let generacion = self.estado.generaciones.subir(id);
        let clave = Clave { entidad: id, generacion };
        let a = self.estado.asentamientos[&id].clone();
        if let Some(max) = a.radio.maximo.filter(|&m| a.radio.valor_en(t) < m) {
            if let Some(ti) = a.radio.instante_de_valor(max, t) {
                self.estado.cola.programar(ti, PRIO_ZONA, clave, Carga::ZonaTecho);
            }
        }
        let alcance_a = a.radio.maximo.unwrap_or(0);
        let mut contactos = Vec::new();
        for o in self.estado.asentamientos.values().filter(|o| o.id != id && o.faccion != a.faccion) {
            if self.estado.contactos.contains(&(id.min(o.id), id.max(o.id))) {
                continue;
            }
            let d = distancia_techo(a.posicion, o.posicion);
            if d > alcance_a + o.radio.maximo.unwrap_or(0) {
                continue; // con los techos de hoy no se tocarán; subir de nivel rehará esto
            }
            if let Some(tc) = territorio::instante_de_contacto(&a.radio, &o.radio, d, t) {
                contactos.push((tc, o.id));
            }
        }
        for (tc, otra) in contactos {
            let otra_generacion = self.estado.generaciones.actual(otra);
            self.estado.cola.programar(tc, PRIO_ZONA, clave, Carga::ZonaContacto { otra, otra_generacion });
        }
    }

    pub(super) fn zona_en_techo(&mut self, id: u64, t: Instante) {
        let Some(a) = self.estado.asentamientos.get(&id) else { return };
        if a.radio.maximo.is_none_or(|m| a.radio.valor_en(t) < m) {
            return; // defensivo: el evento no corresponde al ritmo vigente
        }
        let (posicion, radio) = (a.posicion, a.radio.valor_en(t));
        self.tr.asentamientos.push(id);
        self.tr.eventos.push(EventoNucleo { codigo: "zona.techo".into(), en: t, ejercitos: vec![], asentamientos: vec![id], punto: Some(posicion), datos: vec![("radio", radio)] });
    }

    pub(super) fn zonas_se_tocan(&mut self, id: u64, otra: u64, otra_generacion: u32, t: Instante) {
        if self.estado.generaciones.actual(otra) != otra_generacion {
            return; // la otra cambió de ritmo después de calcular el cruce: su reprogramación lo recalculó
        }
        let (Some(a), Some(b)) = (self.estado.asentamientos.get(&id), self.estado.asentamientos.get(&otra)) else { return };
        if !self.estado.contactos.insert((id.min(otra), id.max(otra))) {
            return;
        }
        let (fa, fb) = (a.faccion as i64, b.faccion as i64);
        let punto = Punto::new((a.posicion.x + b.posicion.x) / 2, (a.posicion.y + b.posicion.y) / 2);
        self.tr.asentamientos.extend([id, otra]);
        self.tr.eventos.push(EventoNucleo {
            codigo: "zona.frontera".into(),
            en: t,
            ejercitos: vec![],
            asentamientos: vec![id.min(otra), id.max(otra)],
            punto: Some(punto),
            datos: vec![("faccionA", fa), ("faccionB", fb)],
        });
    }

    // ───────────────────────────── Órdenes ─────────────────────────────

    /// Ejecuta las órdenes de territorio. `Ok(Some(n))` = cambió algo y creó `n` (una plaza o una parcela); `Ok(None)` =
    /// cambió algo sin crear nada. Rechazos y «sin cambios» salen como `Err` sin haber tocado el estado.
    pub(super) fn ejecutar_territorio(&mut self, t: Instante, p: &Peticion) -> Result<Option<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let sin_cambios = || Respuesta { resultado: Resultado::SinCambios, version_tras: v, creadas: vec![] };
        let solo_admin = || (!p.admin).then(|| rechazo(rechazos::NO_AUTORIZADO, "solo administración".into()));
        let existe = |s: &Self, id: u64| s.estado.asentamientos.contains_key(&id).then_some(()).ok_or_else(|| rechazo(rechazos::ASENTAMIENTO_DESCONOCIDO, format!("as-{id}")));
        match &p.comando {
            Comando::FijarNivelFaccion { faccion, nivel } => {
                if let Some(r) = solo_admin() {
                    return Err(r);
                }
                if *nivel == 0 {
                    return Err(rechazo(rechazos::NIVEL_INVALIDO, "el nivel de Facción empieza en 1".into()));
                }
                if self.estado.niveles_faccion.get(faccion).copied().unwrap_or(1) == *nivel {
                    return Err(sin_cambios());
                }
                self.estado.niveles_faccion.insert(*faccion, *nivel);
                self.tr.eventos.push(EventoNucleo { codigo: "faccion.nivel".into(), en: t, ejercitos: vec![], asentamientos: vec![], punto: None, datos: vec![("faccion", *faccion as i64), ("nivel", *nivel as i64)] });
                Ok(None)
            }
            Comando::DarCaravanaFundacion { ejercito } => {
                if let Some(r) = solo_admin() {
                    return Err(r);
                }
                let Some(e) = self.estado.ejercitos.get_mut(ejercito) else { return Err(rechazo(rechazos::EJERCITO_DESCONOCIDO, format!("ej-{ejercito}"))) };
                if e.caravana_fundacion {
                    return Err(sin_cambios());
                }
                e.caravana_fundacion = true;
                self.tr.tocados.push(*ejercito);
                Ok(None)
            }
            Comando::Fundar { ejercito } => self.fundar(t, p, *ejercito),
            Comando::FijarNivelAsentamiento { asentamiento, nivel } => {
                if let Some(r) = solo_admin() {
                    return Err(r);
                }
                existe(self, *asentamiento)?;
                let (actual, max_nivel) = (self.estado.asentamientos[asentamiento].nivel, self.estado.reglas.territorio.techos.len() as u32);
                if *nivel < actual || *nivel > max_nivel {
                    return Err(rechazo(rechazos::NIVEL_INVALIDO, format!("el nivel va de {actual} a {max_nivel}, no a {nivel}")));
                }
                if *nivel == actual {
                    return Err(sin_cambios());
                }
                let techo = self.estado.reglas.territorio.techo(*nivel);
                let a = self.estado.asentamientos.get_mut(asentamiento).expect("existe");
                a.nivel = *nivel;
                a.radio = Ritmo { maximo: Some(techo), ..a.radio.reanclado(t, a.radio.por_minuto) };
                self.tr.asentamientos.push(*asentamiento);
                self.tr.eventos.push(EventoNucleo { codigo: "asentamiento.nivel".into(), en: t, ejercitos: vec![], asentamientos: vec![*asentamiento], punto: None, datos: vec![("nivel", *nivel as i64)] });
                self.reprogramar_zona(*asentamiento, t);
                Ok(None)
            }
            Comando::AgrandarZona { asentamiento, milesimas } => {
                if let Some(r) = solo_admin() {
                    return Err(r);
                }
                existe(self, *asentamiento)?;
                if *milesimas <= 0 {
                    return Err(rechazo(rechazos::ZONA_INVALIDA, "agrandar es sumar radio".into()));
                }
                let a = self.estado.asentamientos.get_mut(asentamiento).expect("existe");
                let (actual, techo) = (a.radio.valor_en(t), a.radio.maximo.unwrap_or(Mil::MAX));
                let nuevo = (actual + milesimas).min(techo);
                if nuevo == actual {
                    return Err(sin_cambios());
                }
                a.radio = Ritmo { valor_en_t0: nuevo, t0: t, ..a.radio };
                self.tr.asentamientos.push(*asentamiento);
                self.reprogramar_zona(*asentamiento, t);
                Ok(None)
            }
            Comando::ReclamarParcela { asentamiento } => {
                if let Some(r) = solo_admin() {
                    return Err(r);
                }
                let Some(geo) = &self.geografia else { return Err(rechazo(rechazos::SIN_MUNDO, "el núcleo no tiene mundo cargado".into())) };
                existe(self, *asentamiento)?;
                let a = &self.estado.asentamientos[asentamiento];
                let yo = self.disco_de(a, t);
                // Solo importan las vecinas que recortan esta zona; el resto no cambia la respuesta.
                let tocan: Vec<Disco> = self.discos(t).into_iter().filter(|&(o, d)| o != *asentamiento && territorio::se_tocan(&yo, &d)).map(|(_, d)| d).collect();
                let elegida = territorio::elegir_parcela(
                    &geo.parcelas,
                    &geo.total_por_masa,
                    self.estado.reglas.territorio.cuota_parcelas_pct,
                    a.posicion,
                    &a.parcelas,
                    |i| !self.estado.parcelas_ocupadas.contains_key(&i),
                    |c| c.dist2(yo.centro) <= (yo.radio as i128).pow(2) && territorio::en_zona(c, &yo, &tocan),
                );
                let parcela = match elegida {
                    Ok(i) => i,
                    Err(SinParcela::NoHayLibres) => return Err(rechazo(rechazos::PARCELA_SIN_LIBRES, "no hay parcelas libres en la zona".into())),
                    Err(SinParcela::CupoAgotado) => return Err(rechazo(rechazos::PARCELA_CUPO_AGOTADO, "ya explota todo lo que le toca de los bosques que alcanza".into())),
                };
                let centro = geo.parcelas[parcela as usize].centro;
                self.estado.parcelas_ocupadas.insert(parcela, *asentamiento);
                self.estado.asentamientos.get_mut(asentamiento).expect("existe").parcelas.insert(parcela);
                self.tr.asentamientos.push(*asentamiento);
                self.tr.eventos.push(EventoNucleo { codigo: "parcela.reclamada".into(), en: t, ejercitos: vec![], asentamientos: vec![*asentamiento], punto: Some(centro), datos: vec![("parcela", parcela as i64)] });
                Ok(Some(parcela as u64))
            }
            Comando::SoltarParcela { asentamiento, parcela } => {
                if let Some(r) = solo_admin() {
                    return Err(r);
                }
                existe(self, *asentamiento)?;
                if !self.estado.asentamientos[asentamiento].parcelas.contains(parcela) {
                    return Err(rechazo(rechazos::PARCELA_AJENA, format!("la parcela {parcela} no la explota as-{asentamiento}")));
                }
                self.estado.parcelas_ocupadas.remove(parcela);
                self.estado.asentamientos.get_mut(asentamiento).expect("existe").parcelas.remove(parcela);
                self.tr.asentamientos.push(*asentamiento);
                self.tr.eventos.push(EventoNucleo { codigo: "parcela.liberada".into(), en: t, ejercitos: vec![], asentamientos: vec![*asentamiento], punto: None, datos: vec![("parcela", *parcela as i64)] });
                Ok(None)
            }
            _ => unreachable!("solo se llama con órdenes de territorio"),
        }
    }

    /// `fundar` (Doc 1.8): el titular, con la caravana enganchada, funda donde está la columna.
    fn fundar(&mut self, t: Instante, p: &Peticion, ejercito: u64) -> Result<Option<u64>, Respuesta> {
        let v = self.estado.version;
        let Some(geo) = &self.geografia else { return Err(Respuesta::rechazo(rechazos::SIN_MUNDO, "el núcleo no tiene mundo cargado", v)) };
        let Some(e) = self.estado.ejercitos.get(&ejercito) else { return Err(Respuesta::rechazo(rechazos::EJERCITO_DESCONOCIDO, format!("ej-{ejercito}"), v)) };
        if !p.admin && e.propietario != p.actor {
            return Err(Respuesta::rechazo(rechazos::NO_AUTORIZADO, "la columna no es tuya", v));
        }
        if !e.caravana_fundacion {
            return Err(Respuesta::rechazo(rechazos::SIN_CARAVANA, "la columna no lleva una Caravana de Fundación", v));
        }
        let (punto, faccion) = (e.posicion_en(t), e.faccion);
        let r = &self.estado.reglas.territorio;
        let nivel = self.estado.niveles_faccion.get(&faccion).copied().unwrap_or(1);
        let cap = territorio::cap_de_fundacion(nivel, &r.cap_por_nivel) as usize;
        let tiene = self.plazas_de(faccion).count();
        if tiene >= cap {
            return Err(Respuesta::rechazo(rechazos::CAP_ALCANZADO, format!("la Facción tiene {tiene} plazas y su cap es {cap}"), v));
        }
        if let Err(m) = self.validar_fundacion(geo, punto, t) {
            return Err(Respuesta::rechazo(m.codigo(), format!("no se puede fundar en ({}, {})", punto.x, punto.y), v));
        }
        let techo = r.techo(1);
        let radio = Ritmo { valor_en_t0: r.radio_inicial, t0: t, por_minuto: r.crecimiento_por_minuto, maximo: Some(techo), minimo: None };
        let id = self.estado.siguiente_id;
        self.estado.siguiente_id += 1;
        self.estado.asentamientos.insert(id, Asentamiento { id, faccion, posicion: punto, nivel: 1, fundado_en: t, radio, parcelas: BTreeSet::new(), version: 0 });
        self.estado.ejercitos.get_mut(&ejercito).expect("existe").caravana_fundacion = false;
        self.tr.tocados.push(ejercito);
        self.tr.asentamientos.push(id);
        self.tr.eventos.push(EventoNucleo {
            codigo: "fundacion.asentamiento_fundado".into(),
            en: t,
            ejercitos: vec![ejercito],
            asentamientos: vec![id],
            punto: Some(punto),
            datos: vec![("faccion", faccion as i64)],
        });
        self.reprogramar_zona(id, t);
        Ok(Some(id))
    }
}
