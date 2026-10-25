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

    /// Ancho y alto del mapa, en milésimas.
    pub(super) fn tamano(&self) -> (i64, i64) {
        self.mundo.tamano()
    }

    /// Las masas de bosque del artefacto, en su orden (el índice es el que guarda un campamento de bandidos).
    pub(super) fn bosques(&self) -> &[contrato::v1::ZonaBosque] {
        self.mundo.bosques()
    }

    /// Los campamentos de bandidos que trae el artefacto (junto a las vetas de estaño y oro): posición y nivel (1 a 3), en su orden.
    pub(super) fn bandidos_del_artefacto(&self) -> Vec<(Punto, u8)> {
        self.mundo.bandidos_iniciales().iter().filter_map(|b| b.posicion.as_ref().map(|p| (Punto::new(p.x, p.y), b.nivel.clamp(1, 3) as u8))).collect()
    }

    /// Los campamentos de mercenarios del artefacto: su puerta y su origen, en su orden (el índice es el del campamento).
    pub(super) fn campamentos_del_artefacto(&self) -> Vec<(Punto, u32)> {
        self.mundo.campamentos().iter().filter_map(|c| c.posicion.as_ref().map(|p| (Punto::new(p.x, p.y), c.origen))).collect()
    }

    /// ¿Se puede pisar `p`? Tierra firme dentro del mapa y transitable (ni agua ni cima ni río sin vado).
    pub(super) fn es_tierra_firme(&self, p: Punto) -> bool {
        self.terreno(p) == Terreno::Firme
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
    /// Parcelas de bosque libres en la zona del techo del nivel 1 (10 u) del punto, recortada por las plazas de otras Facciones que tocan ese
    /// disco aunque no toquen el inicial (decidido el 2026-10-24). Con 0 la plaza **caerá en ruinas**: sin
    /// Leñera no hay madera para el mantenimiento y no llega a crecer lo bastante para alcanzar otro bosque (decidido el 2026-10-18).
    pub parcelas_en_techo: usize,
    /// Madera por minuto (milésimas) que daría ese bosque si se explotase entero: base de la Leñera × Σ densidades de las parcelas libres.
    pub madera_potencial: Mil,
    /// La plaza caerá en ruinas: sin parcela a 10 u o con menos madera potencial que `economia::MADERA_MINIMA_POTENCIAL`.
    pub condenada: bool,
}

/// Lo que cuenta el aviso de viabilidad: parcelas libres en la zona `nuevo`, piedra en esa zona y las parcelas libres en la zona
/// `techo`. Cada zona la recortan las plazas de otras Facciones que tocan **su** disco: el techo, también las que no tocan el disco
/// inicial (decidido el 2026-10-24). Solo mira lo que la rejilla del mapa da dentro de los discos (`en_zona` exige estar en el
/// disco), así que cuenta lo mismo que recorrer el mapa entero.
fn contar_sitio<'m>(mapa: &'m Mapa, nuevo: &Disco, techo: &Disco, plazas: impl IntoIterator<Item = Disco>, libre: impl Fn(u32) -> bool) -> (usize, usize, Vec<&'m territorio::Parcela>) {
    // Las que tocan el disco mayor; `en_zona` descarta para cada disco las que no lo tocan.
    let mayor = if techo.radio >= nuevo.radio { techo } else { nuevo };
    let tocan: Vec<Disco> = plazas.into_iter().filter(|d| territorio::se_tocan(mayor, d)).collect();
    let tocan = &tocan[..];
    let libres: Vec<&territorio::Parcela> = mapa.parcelas_en(nuevo.centro, nuevo.radio.max(techo.radio)).into_iter().filter(|&i| libre(i)).map(|i| &mapa.parcelas[i as usize]).collect();
    let parcelas_libres = libres.iter().filter(|pa| territorio::en_zona(pa.centro, nuevo, tocan)).count();
    let piedra = mapa.piedra_en(nuevo.centro, nuevo.radio).into_iter().filter(|&n| territorio::en_zona(mapa.nodos[n as usize].pos, nuevo, tocan)).count();
    let en_techo = libres.into_iter().filter(|pa| territorio::en_zona(pa.centro, techo, tocan)).collect();
    (parcelas_libres, piedra, en_techo)
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

    /// De quién es el suelo en cada punto: la Facción de la zona real que lo contiene (las de Facciones distintas no se solapan) o `None` en tierra de nadie (`territorioPorEjercito`,
    /// Niebla_De_Guerra §5.4). Las zonas se leen una vez para todos los puntos.
    pub fn faccion_del_suelo(&self, puntos: &[Punto]) -> Vec<Option<u32>> {
        let discos = self.discos();
        // `en_zona` mira primero el disco (enteros, sin raíz) y solo a los vecinos si el punto cae dentro.
        let suelo = |p: Punto| discos.iter().find(|(id, d)| territorio::en_zona(p, d, discos.iter().filter(|(o, _)| o != id).map(|(_, x)| x))).map(|(_, d)| d.faccion);
        puntos.iter().map(|&p| suelo(p)).collect()
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
        Some(a.urbe.vista(a.urbe.radio_ul(&u), &a.economia.recintos))
    }

    /// Plazas vigentes de una Facción, por id (del índice de `Espacio`: no recorre todas las plazas).
    pub fn plazas_de(&self, faccion: u32) -> impl Iterator<Item = &Asentamiento> + Clone {
        self.espacio.plazas_por_faccion.get(&faccion).into_iter().flatten().filter_map(|id| self.estado.asentamientos.get(id))
    }

    /// El aviso de viabilidad de fundar en `p` para esa Facción. `None` si el núcleo no tiene mundo cargado.
    pub fn viabilidad(&self, p: Punto, faccion: u32) -> Option<InformeViabilidad> {
        let geo = self.geografia.as_ref()?;
        let motivo = self.validar_fundacion(geo, p).err();
        let nuevo = Disco { centro: p, radio: self.estado.reglas.territorio.radio_inicial, faccion };
        // El mismo cálculo con el disco que alcanzará la zona en el nivel 1.
        let techo = Disco { radio: self.estado.reglas.territorio.techo(1), ..nuevo };
        // La zona con la que nacería y la del techo, cada una recortada contra las plazas de otras Facciones que la tocan.
        let plazas = self.estado.asentamientos.values().map(|a| self.disco_de(a));
        let (parcelas_libres, piedra, libres_en_techo) = contar_sitio(&geo.mapa, &nuevo, &techo, plazas, |i| !self.estado.parcelas_ocupadas.contains_key(&i));
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
    pub(super) fn contactos_nuevos(&mut self, id: u64, t: Instante) {
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
            self.tr.tocar_todos(Clase::Asentamiento, [id, otra]);
            self.tr.emitir(&self.estado, EventoNucleo {
                codigo: "zona.frontera".into(),
                en: t,
                ejercitos: vec![],
                asentamientos: vec![id.min(otra), id.max(otra)],
                punto: Some(punto),
                datos: vec![("faccionA", fa), ("faccionB", fb)],
                ..Default::default()
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
        self.tr.tocar(Clase::Asentamiento, id);
        // Las plazas con las que se toca recortan su zona contra esta: su polígono cambia con el radio.
        let vecinas: Vec<u64> = self.estado.contactos.iter().filter_map(|&(x, y)| if x == id { Some(y) } else { (y == id).then_some(x) }).collect();
        self.tr.tocar_todos(Clase::Asentamiento, vecinas);
        self.contactos_nuevos(id, t);
        true
    }

    // ───────────────────────────── Órdenes ─────────────────────────────

    /// Ejecuta las órdenes de territorio. `Ok(Some(n))` = cambió algo y creó `n` (una plaza o una parcela); `Ok(None)` =
    /// cambió algo sin crear nada. Rechazos y «sin cambios» salen como `Err` sin haber tocado el estado.
    pub(super) fn ejecutar_territorio(&mut self, t: Instante, p: &Peticion) -> Result<Option<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let sin_cambios = || Respuesta::sin_cambios(v);
        let existe = |s: &Self, id: u64| s.estado.asentamientos.contains_key(&id).then_some(()).ok_or_else(|| rechazo(rechazos::ASENTAMIENTO_DESCONOCIDO, ids::asentamiento(id)));
        match &p.comando {
            // Andamiaje sobre la regla real (decisión 9 del plan del sistema 5): fija la experiencia al umbral del nivel. La Facción que no existe nace vacía.
            Comando::FijarNivelFaccion { faccion, nivel } => {
                if *nivel == 0 {
                    return Err(rechazo(rechazos::NIVEL_INVALIDO, "el nivel de Facción empieza en 1".into()));
                }
                if self.nivel_de_faccion(*faccion) == *nivel {
                    return Err(sin_cambios());
                }
                let r = self.estado.reglas.economia.politica.clone();
                self.estado.politica.facciones.crear_vacia(&r, *faccion);
                self.estado.politica.facciones.fijar_experiencia(*faccion, politica::nivel::experiencia_minima(&r.nivel, *nivel));
                self.tr.tocar(Clase::Faccion, u64::from(*faccion));
                self.tr.emitir(&self.estado, EventoNucleo { codigo: "faccion.nivel".into(), en: t, ejercitos: vec![], asentamientos: vec![], punto: None, datos: vec![("faccion", *faccion as i64), ("nivel", *nivel as i64)], ..Default::default() });
                Ok(None)
            }
            Comando::DarCaravanaFundacion { ejercito } => {
                if !self.estado.ejercitos.contains_key(ejercito) {
                    return Err(rechazo(rechazos::EJERCITO_DESCONOCIDO, ids::ejercito(*ejercito)));
                }
                if !self.dar_fundacion(t, *ejercito) {
                    return Err(sin_cambios());
                }
                Ok(None)
            }
            Comando::Fundar { ejercito } => self.fundar(t, p, *ejercito),
            Comando::FijarNivelAsentamiento { asentamiento, nivel } => {
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
                let nivel_antes = eco.nivel_actual();
                eco.nivel = *nivel;
                eco.medidor.nivel_actual = *nivel;
                // Una obra de ascenso hacia un nivel que el atajo ya concede se descarta: al acabar bajaría el nivel (Economia_Construccion §11).
                if eco.ascenso.is_some_and(|s| s.nivel_objetivo <= *nivel) {
                    eco.ascenso = None;
                }
                self.tr.tocar(Clase::Asentamiento, *asentamiento);
                self.tr.emitir(&self.estado, EventoNucleo { codigo: "asentamiento.nivel".into(), en: t, ejercitos: vec![], asentamientos: vec![*asentamiento], punto: None, datos: vec![("nivel", *nivel as i64)], ..Default::default() });
                self.contar_nivel(t, Some(nivel_antes), Some(*nivel));
                Ok(None)
            }
            Comando::AgrandarZona { asentamiento } => {
                existe(self, *asentamiento)?;
                if !self.agrandar_zona(*asentamiento, 1, t) {
                    return Err(sin_cambios());
                }
                Ok(None)
            }
            Comando::ReclamarParcela { asentamiento } => {
                let Some(geo) = &self.geografia else { return Err(rechazo(rechazos::SIN_MUNDO, "el núcleo no tiene mundo cargado".into())) };
                existe(self, *asentamiento)?;
                let a = &self.estado.asentamientos[asentamiento];
                let yo = self.disco_de(a);
                // Solo importan las vecinas que recortan esta zona; el resto no cambia la respuesta.
                let tocan: Vec<Disco> = self.discos().into_iter().filter(|&(o, d)| o != *asentamiento && territorio::se_tocan(&yo, &d)).map(|(_, d)| d).collect();
                let elegida = geo.mapa.parcela(
                    a.posicion,
                    yo.radio,
                    &a.parcelas,
                    self.estado.reglas.territorio.cuota_parcelas_pct,
                    &|i| !self.estado.parcelas_ocupadas.contains_key(&i),
                    &|c| territorio::en_zona(c, &yo, &tocan),
                );
                let parcela = match elegida {
                    Ok(i) => i,
                    Err(SinParcela::NoHayLibres) => return Err(rechazo(rechazos::PARCELA_SIN_LIBRES, "no hay parcelas libres en la zona".into())),
                    Err(SinParcela::CupoAgotado) => return Err(rechazo(rechazos::PARCELA_CUPO_AGOTADO, "ya explota todo lo que le toca de los bosques que alcanza".into())),
                };
                let centro = geo.mapa.parcelas[parcela as usize].centro;
                self.estado.parcelas_ocupadas.insert(parcela, *asentamiento);
                self.estado.asentamientos.get_mut(asentamiento).expect("existe").parcelas.insert(parcela);
                self.tr.tocar(Clase::Asentamiento, *asentamiento);
                self.tr.emitir(&self.estado, EventoNucleo { codigo: "parcela.reclamada".into(), en: t, ejercitos: vec![], asentamientos: vec![*asentamiento], punto: Some(centro), datos: vec![("parcela", parcela as i64)], ..Default::default() });
                Ok(Some(parcela as u64))
            }
            Comando::SoltarParcela { asentamiento, parcela } => {
                existe(self, *asentamiento)?;
                if !self.estado.asentamientos[asentamiento].parcelas.contains(parcela) {
                    return Err(rechazo(rechazos::PARCELA_AJENA, format!("la parcela {parcela} no la explota as-{asentamiento}")));
                }
                self.estado.parcelas_ocupadas.remove(parcela);
                self.estado.asentamientos.get_mut(asentamiento).expect("existe").parcelas.remove(parcela);
                self.tr.tocar(Clase::Asentamiento, *asentamiento);
                self.tr.emitir(&self.estado, EventoNucleo { codigo: "parcela.liberada".into(), en: t, ejercitos: vec![], asentamientos: vec![*asentamiento], punto: None, datos: vec![("parcela", *parcela as i64)], ..Default::default() });
                Ok(None)
            }
            _ => unreachable!("solo se llama con órdenes de territorio"),
        }
    }

    /// `fundar` (Doc 1.8): el titular, con la caravana enganchada, funda donde está la columna.
    fn fundar(&mut self, t: Instante, p: &Peticion, ejercito: u64) -> Result<Option<u64>, Respuesta> {
        let v = self.estado.version;
        let Some(geo) = &self.geografia else { return Err(Respuesta::rechazo(rechazos::SIN_MUNDO, "el núcleo no tiene mundo cargado", v)) };
        let Some(e) = self.estado.ejercitos.get(&ejercito) else { return Err(Respuesta::rechazo(rechazos::EJERCITO_DESCONOCIDO, ids::ejercito(ejercito), v)) };
        if !self.manda_columna(p, e, true) {
            return Err(Respuesta::rechazo(rechazos::NO_AUTORIZADO, "la columna no es tuya", v));
        }
        let Some(caravana) = self.estado.fundacion.de_columna(ejercito) else {
            return Err(Respuesta::rechazo(rechazos::SIN_CARAVANA, "la columna no lleva una Caravana de Fundación", v));
        };
        let (punto, lider) = (e.posicion_en(t), e.columna.lider);
        let faccion = e.columna.faccion.ok_or_else(|| Respuesta::rechazo(rechazos::FUNDACION_SIN_FACCION, "una columna sin Facción no funda", v))?;
        // Los fundadores (Fundacion_Y_Refundacion §5.2): el titular y los demás de la columna que son ciudadanos de su Facción, en su orden y hasta `maxJugadoresFundacionGrupal`.
        let fundadores: Vec<u64> = Self::participantes_de(e)
            .into_iter()
            .filter(|h| self.estado.militar.heroes.get(h).is_some_and(|x| x.faccion == Some(faccion)))
            .take(self.estado.reglas.economia.datos.max_fundadores)
            .collect();
        if fundadores.is_empty() {
            return Err(Respuesta::rechazo(rechazos::FUNDACION_SIN_FUNDADORES, "ningún héroe de la columna es ciudadano de su Facción", v));
        }
        let cap = politica::nivel::cap_fundacion(&self.estado.reglas.economia.politica.nivel, self.nivel_de_faccion(faccion)) as usize;
        let r = &self.estado.reglas.territorio;
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
        let nombre = self.nombre_de_ciudad_libre(id);
        self.estado.asentamientos.insert(id, Asentamiento { id, faccion, posicion: punto, fundado_en: t, radio, parcelas: BTreeSet::new(), version: 0, economia, urbe, protegida_hasta: None, ocupada_hasta: None, puerta: Default::default(), cargos: Default::default(), nombre: nombre.clone(), capital_de: None, de_la_faccion_desde: t, resto_tributo: 0, permite_reabastecer_aliados: false, fundadores: Vec::new() });
        self.situar_plaza(id, punto, faccion);
        self.proteger_al_fundar(id, t);
        self.programar_minuto(id, t + base::MS_POR_MINUTO);
        self.programar_bandidos_de_plaza(id, t);
        self.gastar_fundacion(caravana);
        self.tr.tocar(Clase::Ejercito, ejercito);
        // Pasan a residir en ella (dejan la plaza o el campamento donde vivieran; su campamento se traslada con ellos). No es una orden suya: no cuenta para el cooldown de residencia.
        // La columna sigue en el mapa, a la puerta: que se deshaga dentro (§5.5) es deuda de la Caravana de Fundación (5G).
        for &h in &fundadores {
            self.mudar_de_residencia(h, Some(militar::heroe::Residencia::Asentamiento(id as u32)));
        }
        // 5E-3 (decisión del autor, 2026-10-25): quien funda, el Líder de la columna, es su Gobernador de oficio (Doc 2.2); el Rey puede cambiarlo después. Solo si es uno de los fundadores:
        // el cargo pide residir en la plaza (Q4). DUDA: un Líder que no es ciudadano de la Facción de la caravana no reside en ella y la plaza nace sin Gobernador, como antes de 5E-3.
        let gobernador = fundadores.contains(&lider);
        let plaza = self.estado.asentamientos.get_mut(&id).expect("recién fundada");
        if gobernador {
            plaza.cargos.fijar(asentamiento::Cargo::Gobernador, Some(lider));
        }
        // 5H (5B-9): quiénes la fundaron, en su orden; dejan de serlo al mudarse (`mudar_de_residencia`).
        plaza.fundadores = fundadores;

        self.tr.tocar(Clase::Asentamiento, id);
        self.tr.emitir(&self.estado, EventoNucleo {
            codigo: "fundacion.asentamiento_fundado".into(),
            en: t,
            ejercitos: vec![ejercito],
            asentamientos: vec![id],
            punto: Some(punto),
            datos: vec![("faccion", faccion as i64), ("caravana", caravana as i64)],
            valores: if nombre.is_empty() { vec![] } else { vec![("nombre", crate::estado::Dato::Texto(nombre))] },
            ..Default::default()
        });
        if gobernador {
            let (datos, valores) = (vec![("asentamiento", id as i64), ("faccion", i64::from(faccion)), ("heroe", lider as i64)], vec![("cargo", crate::estado::Dato::Texto(asentamiento::Cargo::Gobernador.clave().to_owned()))]);
            self.tr.emitir(&self.estado, EventoNucleo { valores, asentamientos: vec![id], ..EventoNucleo::nuevo("cargo.local_asignado", t, datos) });
        }
        // La derrota se borra al fundar (§8) y la fundación da experiencia salvo la primera plaza de la Facción (§3.5). La capital puede cambiar: la Facción cambia.
        self.estado.politica.facciones.anular_derrota(faccion);
        self.tr.tocar(Clase::Faccion, u64::from(faccion));
        self.dar_experiencia_por(t, faccion, politica::nivel::Hecho::Fundacion { primera: tiene == 0 }, super::experiencia::motivo::FUNDACION);
        self.contactos_nuevos(id, t);
        Ok(Some(id))
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::economia::NodoMapa;
    use asentamiento::Tipo;
    use base::Rng;
    use territorio::Parcela;

    /// La rejilla cuenta lo mismo que recorrer el mapa entero (como hacía `viabilidad` hasta el 2026-10-24), en mundos al azar.
    #[test]
    fn la_rejilla_cuenta_lo_mismo_que_el_recorrido_lineal() {
        let mut rng = Rng::nuevo(2026_10_24);
        for _ in 0..15 {
            let (ancho, alto) = (100_000 + rng.por_debajo(400_000) as i64, 100_000 + rng.por_debajo(400_000) as i64);
            let punto = |rng: &mut Rng| Punto::new(rng.por_debajo(ancho as u64 + 40_000) as i64 - 20_000, rng.por_debajo(alto as u64 + 40_000) as i64 - 20_000);
            let parcelas: Vec<Parcela> = (0..2000).map(|_| Parcela { bosque: rng.por_debajo(3) as u32, centro: punto(&mut rng), densidad: rng.por_debajo(1001) as u32 }).collect();
            let tipos = [Tipo::Cantera, Tipo::Corral, Tipo::Mina];
            let nodos: Vec<NodoMapa> = (0..600).map(|_| NodoMapa { pos: punto(&mut rng), tipo: tipos[rng.por_debajo(3) as usize], inicial: 1000 }).collect();
            let mapa = Mapa::nuevo(nodos, parcelas, vec![0; 3], vec![], ancho, alto);
            let ocupadas: BTreeSet<u32> = (0..500).map(|_| rng.por_debajo(2000) as u32).collect();
            let plazas: Vec<Disco> = (0..60).map(|_| Disco { centro: punto(&mut rng), radio: 6_000 + rng.por_debajo(24_001) as i64, faccion: 1 + rng.por_debajo(4) as u32 }).collect();
            for _ in 0..300 {
                let nuevo = Disco { centro: punto(&mut rng), radio: 6_000, faccion: 1 + rng.por_debajo(4) as u32 };
                let techo = Disco { radio: 10_000, ..nuevo };
                let libre = |i: u32| !ocupadas.contains(&i);
                let lineal_libres = mapa.parcelas.iter().enumerate().filter(|(i, pa)| libre(*i as u32) && territorio::en_zona(pa.centro, &nuevo, &plazas)).count();
                let lineal_piedra = mapa.nodos.iter().filter(|n| n.tipo == Tipo::Cantera && territorio::en_zona(n.pos, &nuevo, &plazas)).count();
                let lineal_techo: Vec<&Parcela> = mapa.parcelas.iter().enumerate().filter(|(i, pa)| libre(*i as u32) && territorio::en_zona(pa.centro, &techo, &plazas)).map(|(_, pa)| pa).collect();
                let (libres, piedra, en_techo) = contar_sitio(&mapa, &nuevo, &techo, plazas.iter().copied(), libre);
                assert_eq!((libres, piedra), (lineal_libres, lineal_piedra));
                assert_eq!(en_techo, lineal_techo, "mismas parcelas y en el mismo orden");
            }
        }
    }

    /// El techo del nivel 1 lo recortan también las plazas de otras Facciones que lo tocan sin tocar el disco inicial (2026-10-24).
    #[test]
    fn el_techo_lo_recortan_las_plazas_que_solo_tocan_el_techo() {
        // Rival de r 6 a 14 u: no toca el disco inicial (6 + 6 < 14) pero sí el techo (10 + 6 ≥ 14). La frontera del techo queda a
        // 14 · 10 / 16 = 8,75 u, así que la parcela a 9 u en su dirección no es de la zona; la de 9 u al otro lado, sí.
        let parcela = |x: i64| Parcela { bosque: 0, centro: Punto::new(x, 100_000), densidad: 500 };
        let mapa = Mapa::nuevo(vec![], vec![parcela(109_000), parcela(91_000)], vec![2], vec![], 200_000, 200_000);
        let nuevo = Disco { centro: Punto::new(100_000, 100_000), radio: 6_000, faccion: 1 };
        let techo = Disco { radio: 10_000, ..nuevo };
        let rival = Disco { centro: Punto::new(114_000, 100_000), radio: 6_000, faccion: 2 };
        assert!(!territorio::se_tocan(&nuevo, &rival) && territorio::se_tocan(&techo, &rival));
        let (_, _, en_techo) = contar_sitio(&mapa, &nuevo, &techo, [rival], |_| true);
        assert_eq!(en_techo, vec![&mapa.parcelas[1]]);
        // De la misma Facción no recorta.
        let (_, _, en_techo) = contar_sitio(&mapa, &nuevo, &techo, [Disco { faccion: 1, ..rival }], |_| true);
        assert_eq!(en_techo.len(), 2);
    }

    /// `ReclamarParcela` con la rejilla elige lo mismo, y falla con el mismo motivo, que `elegir_parcela` sobre el mapa entero.
    #[test]
    fn reclamar_parcela_con_la_rejilla_es_lo_mismo_que_el_recorrido_lineal() {
        let mut rng = Rng::nuevo(2026_10_24_2);
        let mut vistos = [0usize; 3];
        for _ in 0..15 {
            let (ancho, alto) = (100_000 + rng.por_debajo(300_000) as i64, 100_000 + rng.por_debajo(300_000) as i64);
            let punto = |rng: &mut Rng| Punto::new(rng.por_debajo(ancho as u64 + 40_000) as i64 - 20_000, rng.por_debajo(alto as u64 + 40_000) as i64 - 20_000);
            let parcelas: Vec<Parcela> = (0..2000).map(|_| Parcela { bosque: rng.por_debajo(3) as u32, centro: punto(&mut rng), densidad: rng.por_debajo(4) as u32 * 250 }).collect();
            let mut total = vec![0; 3];
            parcelas.iter().for_each(|p| total[p.bosque as usize] += 1);
            let mapa = Mapa::nuevo(vec![], parcelas, total, vec![], ancho, alto);
            let ocupadas: BTreeSet<u32> = (0..800).map(|_| rng.por_debajo(2000) as u32).collect();
            let plazas: Vec<Disco> = (0..60).map(|_| Disco { centro: punto(&mut rng), radio: 6_000 + rng.por_debajo(24_001) as i64, faccion: 1 + rng.por_debajo(4) as u32 }).collect();
            for _ in 0..300 {
                let yo = Disco { centro: punto(&mut rng), radio: 6_000 + rng.por_debajo(24_001) as i64, faccion: 1 + rng.por_debajo(4) as u32 };
                let tocan: Vec<Disco> = plazas.iter().copied().filter(|d| territorio::se_tocan(&yo, d)).collect();
                // Propias: ocupadas por esta plaza; con cuota 0 el cupo es 1 por masa y se agota en cuanto tiene una.
                let propias: BTreeSet<u32> = ocupadas.iter().copied().filter(|_| rng.por_debajo(400) == 0).collect();
                let cuota = [0, 1, 50][rng.por_debajo(3) as usize];
                let libre = |i: u32| !ocupadas.contains(&i);
                let r2 = (yo.radio as i128).pow(2);
                let lineal = territorio::elegir_parcela(&mapa.parcelas, &mapa.total_por_masa, cuota, yo.centro, &propias, libre, |c| c.dist2(yo.centro) <= r2 && territorio::en_zona(c, &yo, &tocan));
                let rejilla = mapa.parcela(yo.centro, yo.radio, &propias, cuota, &libre, &|c| territorio::en_zona(c, &yo, &tocan));
                assert_eq!(rejilla, lineal);
                vistos[match lineal { Ok(_) => 0, Err(SinParcela::NoHayLibres) => 1, Err(SinParcela::CupoAgotado) => 2 }] += 1;
            }
        }
        assert!(vistos.iter().all(|&k| k > 0), "se prueban los tres desenlaces: {vistos:?}");
    }
}
