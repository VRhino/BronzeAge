//! Mundo y territorio sobre el núcleo (Doc 1): fundar, zonas de influencia que crecen y chocan, y parcelas de bosque.
//! Las reglas puras viven en el crate `territorio`; aquí está el estado, los eventos y las órdenes.
//!
//! La zona de una plaza es un disco cuyo radio **solo crece con los edificios completados** (`AgrandarZona`: 1 unidad cada
//! uno, hasta el techo de su nivel); no crece con el tiempo (Economia_Construccion §3.1). Subir de nivel solo levanta el
//! techo. Por eso no hay eventos programados: el contacto entre zonas de Facciones distintas se comprueba en el momento en que
//! un radio cambia (fundar, agrandar) y se anuncia una sola vez por pareja. Desde entonces la frontera dura recorta ambas
//! zonas; el recorte es una consulta ([`Nucleo::en_zona`]), no un estado.

use super::*;
use crate::economia::Mapa;
use crate::estado::Asentamiento;
use base::Instante;
use mundo::Mundo;
use std::collections::BTreeSet;
use territorio::{Disco, DistanciasDeFundacion, Motivo, SinParcela, Terreno, Viabilidad};

/// Lo que el núcleo necesita del artefacto de mundo, ya preparado: no es estado de la partida.
pub(crate) struct Geografia {
    mundo: Arc<Mundo>,
    /// Parcelas, nodos y campamentos con su rejilla de búsqueda (los comparte la economía de las plazas).
    pub(super) mapa: Arc<Mapa>,
}

impl Geografia {
    pub(crate) fn de(mundo: Arc<Mundo>) -> Self {
        Self { mapa: Mapa::de(&mundo), mundo }
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
    /// Parcelas de bosque libres a menos del techo de la zona del nivel 1 (10 u) del punto. Con 0 la plaza **caerá en ruinas**: sin
    /// Leñera no hay madera para el mantenimiento y no llega a crecer lo bastante para alcanzar otro bosque (decidido el 2026-10-18).
    pub parcelas_en_techo: usize,
    /// Madera por minuto (milésimas) que daría ese bosque si se explotase entero: base de la Leñera × Σ densidades de las parcelas libres.
    pub madera_potencial: Mil,
    /// La plaza caerá en ruinas: sin parcela a 10 u o con menos madera potencial que `economia::MADERA_MINIMA_POTENCIAL`.
    pub condenada: bool,
}

impl Nucleo {
    // ───────────────────────────── Consultas ─────────────────────────────

    fn disco_de(&self, a: &Asentamiento) -> Disco {
        Disco { centro: a.posicion, radio: a.radio, faccion: a.faccion }
    }

    /// Los discos de todas las plazas en `t`, por id.
    pub(super) fn discos(&self) -> Vec<(u64, Disco)> {
        self.estado.asentamientos.values().map(|a| (a.id, self.disco_de(a))).collect()
    }

    /// Radio de la zona de una plaza ahora, en milésimas.
    pub fn radio_zona(&self, id: u64) -> Option<Mil> {
        self.estado.asentamientos.get(&id).map(|a| a.radio)
    }

    /// ¿Cae `p` en la zona real de la plaza (disco recortado por las fronteras con otras Facciones)?
    pub fn en_zona(&self, id: u64, p: Punto) -> bool {
        let Some(a) = self.estado.asentamientos.get(&id) else { return false };
        let yo = self.disco_de(a);
        let vecinos: Vec<Disco> = self.discos().into_iter().filter(|&(o, _)| o != id).map(|(_, d)| d).collect();
        territorio::en_zona(p, &yo, &vecinos)
    }

    /// Polígono de la zona real (presentación: `Asentamiento.zona`). 48 lados, como `ZONA_INFLUENCIA.segmentosPoligono`.
    pub fn poligono_zona(&self, id: u64) -> Vec<Punto> {
        let Some(a) = self.estado.asentamientos.get(&id) else { return vec![] };
        let vecinos: Vec<Disco> = self.discos().into_iter().filter(|&(o, _)| o != id).map(|(_, d)| d).collect();
        territorio::poligono(&self.disco_de(a), &vecinos, 48)
    }

    /// La ciudad de una plaza (solo lectura): sus piezas con el rectángulo de cada edificio, las calles y los caminos. Para pintar, véase
    /// también [`Nucleo::vista_ciudad`].
    pub fn ciudad(&self, id: u64) -> Option<&trazado::Ciudad> {
        self.estado.asentamientos.get(&id).map(|a| a.urbe.ciudad())
    }

    /// Todo lo que un cliente necesita para dibujar la ciudad de una plaza (T §10), en unidades locales.
    pub fn vista_ciudad(&self, id: u64) -> Option<crate::urbanismo::VistaCiudad> {
        let a = self.estado.asentamientos.get(&id)?;
        let u = a.economia.urbano(&self.estado.reglas.economia.datos);
        Some(a.urbe.vista(a.urbe.radio_ul(&u)))
    }

    /// Plazas vigentes de una Facción.
    pub fn plazas_de(&self, faccion: u32) -> impl Iterator<Item = &Asentamiento> {
        self.estado.asentamientos.values().filter(move |a| a.faccion == faccion)
    }

    /// El aviso de viabilidad de fundar en `p` para esa Facción. `None` si el núcleo no tiene mundo cargado.
    pub fn viabilidad(&self, p: Punto, faccion: u32) -> Option<InformeViabilidad> {
        let geo = self.geografia.as_ref()?;
        let motivo = self.validar_fundacion(geo, p).err();
        let nuevo = Disco { centro: p, radio: self.estado.reglas.territorio.radio_inicial, faccion };
        // La zona con la que nacería: el círculo inicial recortado contra las plazas de otras Facciones.
        let vecinos: Vec<Disco> = self.discos().into_iter().map(|(_, d)| d).collect();
        let tocan: Vec<Disco> = vecinos.into_iter().filter(|d| territorio::se_tocan(&nuevo, d)).collect();
        let en_zona = |c: Punto| territorio::en_zona(c, &nuevo, &tocan);
        let parcelas_libres = geo.mapa.parcelas.iter().enumerate().filter(|(i, pa)| !self.estado.parcelas_ocupadas.contains_key(&(*i as u32)) && en_zona(pa.centro)).count();
        let piedra = geo.mapa.nodos.iter().filter(|n| n.tipo == asentamiento::Tipo::Cantera && en_zona(n.pos)).count();
        // El mismo cálculo con el disco que alcanzará la zona en el nivel 1.
        let techo = Disco { radio: self.estado.reglas.territorio.techo(1), ..nuevo };
        let en_techo = |c: Punto| territorio::en_zona(c, &techo, &tocan);
        let libres_en_techo: Vec<&territorio::Parcela> = geo.mapa.parcelas.iter().enumerate().filter(|(i, pa)| !self.estado.parcelas_ocupadas.contains_key(&(*i as u32)) && en_techo(pa.centro)).map(|(_, pa)| pa).collect();
        let parcelas_en_techo = libres_en_techo.len();
        let base_lenera = self.estado.reglas.economia.datos.catalogo.tipo(asentamiento::Tipo::Lenera).produccion.map_or(0, |(_, b)| b);
        let madera_potencial = base_lenera * libres_en_techo.iter().map(|pa| pa.densidad as Mil).sum::<Mil>() / 1000;
        let condenada = motivo.is_none() && (parcelas_en_techo == 0 || madera_potencial < crate::economia::MADERA_MINIMA_POTENCIAL);
        Some(InformeViabilidad { color: territorio::viabilidad(motivo.is_none(), parcelas_libres, piedra), motivo, parcelas_libres, piedra, parcelas_en_techo, madera_potencial, condenada })
    }

    fn validar_fundacion(&self, geo: &Geografia, p: Punto) -> Result<(), Motivo> {
        let r = &self.estado.reglas.territorio;
        let plazas: Vec<Disco> = self.discos().into_iter().map(|(_, d)| d).collect();
        let dist = DistanciasDeFundacion { separacion_plazas: r.separacion_plazas, exclusion_campamento: r.exclusion_campamento };
        territorio::validar(p, geo.terreno(p), &plazas, geo.mapa.campamentos.iter().copied(), &dist)
    }

    // ───────────────────────────── Fronteras ─────────────────────────────

    /// Tras cambiar el radio de `id` (o nacer), anuncia las parejas con una plaza de otra Facción cuyas zonas ya se tocan y
    /// no se habían tocado antes. Las zonas nunca se encogen, así que un contacto no se deshace.
    fn contactos_nuevos(&mut self, id: u64, t: Instante) {
        let a = &self.estado.asentamientos[&id];
        let (yo, faccion, posicion) = (self.disco_de(a), a.faccion, a.posicion);
        let nuevos: Vec<u64> = self
            .estado
            .asentamientos
            .values()
            .filter(|o| o.faccion != faccion && territorio::se_tocan(&yo, &self.disco_de(o)) && !self.estado.contactos.contains(&(id.min(o.id), id.max(o.id))))
            .map(|o| o.id)
            .collect();
        for otra in nuevos {
            let b = &self.estado.asentamientos[&otra];
            let (fa, fb) = (faccion as i64, b.faccion as i64);
            let punto = Punto::new((posicion.x + b.posicion.x) / 2, (posicion.y + b.posicion.y) / 2);
            self.estado.contactos.insert((id.min(otra), id.max(otra)));
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
    }

    /// La zona de `id` crece `edificios` unidades de `crecimiento_por_edificio` (uno por edificio completado), sin pasar del techo del
    /// nivel alcanzado. Anuncia los contactos nuevos. Devuelve si el radio cambió. Es el único sitio donde crece una zona.
    pub(super) fn agrandar_zona(&mut self, id: u64, edificios: i64, t: Instante) -> bool {
        let r = &self.estado.reglas.territorio;
        let a = self.estado.asentamientos.get_mut(&id).expect("existe");
        let nuevo = (a.radio + edificios * r.crecimiento_por_edificio).min(r.techo(a.nivel()));
        if nuevo <= a.radio {
            return false;
        }
        a.radio = nuevo;
        self.tr.asentamientos.push(id);
        self.contactos_nuevos(id, t);
        true
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
                let (actual, max_nivel) = (self.estado.asentamientos[asentamiento].nivel(), self.estado.reglas.territorio.techos.len() as u32);
                if *nivel < actual || *nivel > max_nivel {
                    return Err(rechazo(rechazos::NIVEL_INVALIDO, format!("el nivel va de {actual} a {max_nivel}, no a {nivel}")));
                }
                if *nivel == actual {
                    return Err(sin_cambios());
                }
                // Solo sube el techo: el radio no salta (Doc 1.2). Fija el nivel alcanzado y el operativo de la economía.
                let eco = &mut self.estado.asentamientos.get_mut(asentamiento).expect("existe").economia;
                eco.nivel = *nivel;
                eco.medidor.nivel_actual = *nivel;
                // DUDA: una obra de ascenso hacia un nivel que el atajo ya concede no tiene sentido (al acabar bajaría el nivel): se descarta.
                if eco.ascenso.is_some_and(|s| s.nivel_objetivo <= *nivel) {
                    eco.ascenso = None;
                }
                self.tr.asentamientos.push(*asentamiento);
                self.tr.eventos.push(EventoNucleo { codigo: "asentamiento.nivel".into(), en: t, ejercitos: vec![], asentamientos: vec![*asentamiento], punto: None, datos: vec![("nivel", *nivel as i64)] });
                Ok(None)
            }
            Comando::AgrandarZona { asentamiento } => {
                if let Some(r) = solo_admin() {
                    return Err(r);
                }
                existe(self, *asentamiento)?;
                if !self.agrandar_zona(*asentamiento, 1, t) {
                    return Err(sin_cambios());
                }
                Ok(None)
            }
            Comando::ReclamarParcela { asentamiento } => {
                if let Some(r) = solo_admin() {
                    return Err(r);
                }
                let Some(geo) = &self.geografia else { return Err(rechazo(rechazos::SIN_MUNDO, "el núcleo no tiene mundo cargado".into())) };
                existe(self, *asentamiento)?;
                let a = &self.estado.asentamientos[asentamiento];
                let yo = self.disco_de(a);
                // Solo importan las vecinas que recortan esta zona; el resto no cambia la respuesta.
                let tocan: Vec<Disco> = self.discos().into_iter().filter(|&(o, d)| o != *asentamiento && territorio::se_tocan(&yo, &d)).map(|(_, d)| d).collect();
                let elegida = territorio::elegir_parcela(
                    &geo.mapa.parcelas,
                    &geo.mapa.total_por_masa,
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
                let centro = geo.mapa.parcelas[parcela as usize].centro;
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
        if !self.manda_columna(p, e, true) {
            return Err(Respuesta::rechazo(rechazos::NO_AUTORIZADO, "la columna no es tuya", v));
        }
        if !e.caravana_fundacion {
            return Err(Respuesta::rechazo(rechazos::SIN_CARAVANA, "la columna no lleva una Caravana de Fundación", v));
        }
        let punto = e.posicion_en(t);
        let faccion = e.columna.faccion.ok_or_else(|| Respuesta::rechazo(rechazos::FUNDACION_SIN_FACCION, "una columna sin Facción no funda", v))?;
        let r = &self.estado.reglas.territorio;
        let nivel = self.estado.niveles_faccion.get(&faccion).copied().unwrap_or(1);
        let cap = territorio::cap_de_fundacion(nivel, &r.cap_por_nivel) as usize;
        let tiene = self.plazas_de(faccion).count();
        if tiene >= cap {
            return Err(Respuesta::rechazo(rechazos::CAP_ALCANZADO, format!("la Facción tiene {tiene} plazas y su cap es {cap}"), v));
        }
        if let Err(m) = self.validar_fundacion(geo, punto) {
            return Err(Respuesta::rechazo(m.codigo(), format!("no se puede fundar en ({}, {})", punto.x, punto.y), v));
        }
        let radio = r.radio_inicial;
        let id = self.estado.siguiente_id;
        self.estado.siguiente_id += 1;
        let eco = &self.estado.reglas.economia.datos;
        let economia = asentamiento::asentamiento::Asentamiento::fundar(eco, t, &eco.materiales_iniciales, eco.viviendas_iniciales);
        // Su ciudad nace con los mismos edificios iniciales que la economía (ids 0..=viviendas+1).
        let urbe = crate::urbanismo::Urbe::fundar(&self.estado.reglas.economia, self.semilla, id);
        self.estado.asentamientos.insert(id, Asentamiento { id, faccion, posicion: punto, fundado_en: t, radio, parcelas: BTreeSet::new(), version: 0, economia, urbe });
        self.programar_minuto(id, t + base::MS_POR_MINUTO);
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
        self.contactos_nuevos(id, t);
        Ok(Some(id))
    }
}
