//! Los bandidos en el espacio (Doc 1.9, Generacion_Del_Mundo §13, Combate §7.2, fase 4D). Un campamento de bandidos es una entidad **fija** de la rejilla de
//! encuentros, igual que una plaza: no tiene plan, pero cuenta para el cruce con las caravanas.
//!
//! # Cuándo aparecen
//! Todo es un evento de prioridad 7 (`PRIO_BANDIDOS`: los bandidos ven a las columnas tal como estaban antes de moverse), uno por cosa que espera y ninguno por
//! cosa ociosa:
//! - **Uno por plaza, siempre** (`SucesoCombate::BandidosDeLaPlaza`): al fundarla, y al vencer el plazo de reaparición (`respawnMinutos`) del que se destruyó.
//!   En el bosque no reclamado más cercano a ella, sin campamento encima y con el centro fuera de toda zona. Sin bosque que valga no gasta tirada ni reintenta
//!   (DUDA: el TypeScript volvía a mirar cada tick).
//! - **En el anillo de cada campamento de mercenarios** (`SucesoCombate::BandidosDelAnillo`): `clamp(ceil(residentes sin plaza / 2), 1, 6)` campamentos, uno cada
//!   `reaparicionMinutos` mientras falten. La cita arranca con el mundo, se renueva al crear uno o al destruirse uno del anillo, y se detiene cuando no falta ninguno
//!   (DUDA: un residente que llega a un campamento no adelanta la cita; el TypeScript miraba la demanda cada tick). Sin sitio válido tras `INTENTOS_ANILLO` intentos
//!   (dos tiradas cada uno) reintenta a los `reaparicionMinutos`.
//! - **Los del mundo**: el artefacto trae los campamentos de las vetas de estaño y oro (`bandidos_iniciales`); nacen con la partida, no reaparecen.
//! - **Los de administración** (`ColocarBandidos`): para escenarios y pruebas.
//!
//! # Cómo acosan
//! A una caravana que pasa a ≤ `radioAtaqueCaravana` de un campamento: el cruce es un evento exacto del mismo tipo que los encuentros
//! (`Carga::Encuentro`), pero de prioridad 7. Se programa cuando la caravana estrena plan o cuando el campamento aparece, y al vencer se mira con el estado de ese
//! instante: una caravana parada en una plaza, dentro de cualquier zona o en la protección de un campamento de mercenarios no es presa. Una tirada: gana el
//! bandido si `poder × tirada > defensa` (la escolta cedida, o los carreteros); la caravana desaparece con su carga y su escolta queda a 0 y vuelve al
//! campamento de su héroe. Una caravana que se salva no vuelve a ser acosada por ese campamento hasta que cambie de plan. Las caravanas adjuntas a un ejército
//! se defienden con el poder de la columna, que no sufre bajas: el cruce lo programa la columna en marcha que las lleva.
//!
//! # Con servidores de batalla (quinta ronda de Entrar con todo, 2026-10-22)
//! Los bandidos no son bots ni jugadores. Si el acoso es a una columna que lleva caravanas adjuntas y **algún héroe sano** y el proceso tiene servidores de batalla, no se tira: se abre una
//! batalla de caravana ([`Nucleo::apertura_de_acoso`]) con **todas** las caravanas adjuntas que son presa (decisión del autor, 2026-10-22), en la que atacan los bandidos y defienden los héroes de la columna, y Unity decide (haya humanos o no). Una caravana sin héroes
//! —suelta, con escolta de tropa cedida o sin ella, o adjunta a una columna sin héroes sanos— se resuelve **siempre con números**, haya servidores o no. Un campamento no vuelve a
//! acosar a la misma columna mientras se conserve su batalla ([`Nucleo::acoso_ya_jugado`]).

use super::*;
use crate::estado::{SucesoCombate, PRIO_BANDIDOS};
use crate::estado_combate::{entidad_anillo, entidad_bandidos, es_entidad_bandidos, CampamentoBandidos, OrigenBandidos};
use crate::estado_comercio::es_entidad_caravana;
use ::encuentros::{plan_quieto, Entradas};
use combate::bandidos::{aparece_junto_a_plaza, campamentos_que_tocan, nivel_al_azar, proxima_aparicion_en_el_anillo, punto_del_anillo, puede_acosar, reaparece_en, resolver_acoso};
use combate::caravana::{Defensa, Zona};
use militar::escuadra::Escuadra;

/// Cuánto más lejos del borde de la zona que el propio centro del bosque (mil. = 10 u): el sitio de una plaza no pega a una frontera.
// DUDA: Generacion_Del_Mundo §13 dice «el centro del bosque fuera de toda zona» y la nota de la 4A-2 añade «a ≥ 10 del borde de la zona»; se aplica la
// segunda, que contiene a la primera (el disco de la zona contiene a su polígono).
const MARGEN_DE_ZONA: Mil = 10_000;

impl Nucleo {
    // ───────────────────────────── Consultas ─────────────────────────────

    /// Un campamento de bandidos por id.
    pub fn campamento_de_bandidos(&self, id: u64) -> Option<&CampamentoBandidos> {
        self.estado.combate.bandidos.get(&id)
    }

    /// Todos los campamentos de bandidos, por id.
    pub fn campamentos_de_bandidos(&self) -> impl Iterator<Item = &CampamentoBandidos> {
        self.estado.combate.bandidos.values()
    }

    // ───────────────────────────── Alta y baja ─────────────────────────────

    /// La partida nace: los campamentos que trae el artefacto de mundo (junto a las vetas) y la primera cita de cada anillo de mercenarios. Solo en un estado
    /// nuevo; una foto ya lo trae todo.
    pub(super) fn sembrar_el_mundo(&mut self) {
        let Some(geo) = &self.geografia else { return };
        let (t, n) = (self.estado.ahora, geo.mapa.campamentos.len() as u32);
        let del_artefacto: Vec<(Punto, u8)> = geo.bandidos_del_artefacto();
        for (i, (p, nivel)) in del_artefacto.into_iter().enumerate() {
            self.poner_bandidos(p, nivel, OrigenBandidos::Veta(i as u32), None, t);
        }
        for c in 0..n {
            self.programar_anillo(c, t);
        }
    }

    /// Pone un campamento de bandidos en el mapa: entra en la rejilla y programa el acoso a las caravanas que ya pasan por su radio.
    fn poner_bandidos(&mut self, posicion: Punto, nivel: u8, origen: OrigenBandidos, bosque: Option<u32>, t: Instante) -> u64 {
        let id = self.estado.combate.siguiente_bandidos;
        self.estado.combate.siguiente_bandidos += 1;
        self.estado.combate.bandidos.insert(id, CampamentoBandidos { id, posicion, nivel, origen, bosque });
        let entidad = entidad_bandidos(id);
        let plan = plan_quieto(posicion, t, 0);
        self.espacio.rejilla.apuntar(entidad, self.estado.generaciones.actual(entidad), &plan);
        self.programar_acosos(entidad, &plan, Entradas::Todas);
        id
    }

    /// Destruye un campamento: sale del mapa y del espacio, sus citas pendientes mueren con su generación y, según de dónde saliera, se agenda el siguiente.
    pub(super) fn destruir_bandidos(&mut self, id: u64, t: Instante) {
        let Some(b) = self.estado.combate.bandidos.remove(&id) else { return };
        let entidad = entidad_bandidos(id);
        self.estado.generaciones.subir(entidad);
        self.espacio.rejilla.quitar(entidad);
        match b.origen {
            OrigenBandidos::Plaza(plaza) => {
                let cr = self.estado.reglas.economia.combate.clone();
                let en = reaparece_en(&cr, t);
                self.estado.combate.reaparecen.insert(plaza, en);
                self.programar_bandidos_de_plaza(plaza, en);
            }
            OrigenBandidos::Anillo(c) => {
                let desde = self.estado.combate.anillos.get(&c).copied().unwrap_or(t).max(t);
                self.programar_anillo(c, desde);
            }
            OrigenBandidos::Veta(_) | OrigenBandidos::Admin => {}
        }
    }

    /// La plaza cae en ruinas: su campamento de bandidos desaparece con ella (sin reaparecer) y su plazo de reaparición también.
    pub(super) fn bandidos_de_plaza_arruinada(&mut self, plaza: u64) {
        let suyos: Vec<u64> = self.estado.combate.bandidos.values().filter(|b| b.origen == OrigenBandidos::Plaza(plaza)).map(|b| b.id).collect();
        for id in suyos {
            self.estado.combate.bandidos.remove(&id);
            self.estado.generaciones.subir(entidad_bandidos(id));
            self.espacio.rejilla.quitar(entidad_bandidos(id));
        }
        self.estado.combate.reaparecen.remove(&plaza);
    }

    // ───────────────────────────── Las citas ─────────────────────────────

    /// Programa la aparición del campamento de una plaza en `en`.
    pub(super) fn programar_bandidos_de_plaza(&mut self, plaza: u64, en: Instante) {
        let clave = self.estado.generaciones.clave(plaza);
        self.estado.cola.programar(en, PRIO_BANDIDOS, clave, Carga::Combate(SucesoCombate::BandidosDeLaPlaza));
    }

    /// Programa la cita del anillo del campamento de mercenarios `c` en `en`.
    fn programar_anillo(&mut self, c: u32, en: Instante) {
        let clave = self.estado.generaciones.clave(entidad_anillo(c));
        self.estado.cola.programar(en, PRIO_BANDIDOS, clave, Carga::Combate(SucesoCombate::BandidosDelAnillo));
    }

    /// El bosque no reclamado más cercano a `desde`: sin campamento de bandidos encima y con el centro fuera de la zona de toda plaza (más el margen). Devuelve su
    /// índice y su centro. No gasta tiradas.
    fn bosque_libre_mas_cercano(&self, desde: Punto) -> Option<(u32, Punto)> {
        let geo = self.geografia.as_ref()?;
        let mut candidatos: Vec<(i128, u32, Punto)> = geo.bosques().iter().enumerate().filter_map(|(i, b)| b.centro.as_ref().map(|c| (Punto::new(c.x, c.y).dist2(desde), i as u32, Punto::new(c.x, c.y)))).collect();
        candidatos.sort_by_key(|&(d, i, _)| (d, i));
        let ocupado = |i: u32| self.estado.combate.bandidos.values().any(|b| b.bosque == Some(i));
        candidatos.into_iter().find(|&(_, i, centro)| !ocupado(i) && !self.estado.asentamientos.values().any(|a| a.posicion.dist(centro) < a.radio + MARGEN_DE_ZONA)).map(|(_, i, c)| (i, c))
    }

    /// Vence la cita de aparición del campamento de la plaza.
    pub(super) fn bandidos_de_la_plaza(&mut self, plaza: u64, t: Instante) {
        let Some(a) = self.estado.asentamientos.get(&plaza) else { return };
        let (tiene, reaparece) = (self.estado.combate.bandidos.values().any(|b| b.origen == OrigenBandidos::Plaza(plaza)), self.estado.combate.reaparecen.get(&plaza).copied());
        if !aparece_junto_a_plaza(tiene, reaparece, t) {
            return;
        }
        let Some((bosque, centro)) = self.bosque_libre_mas_cercano(a.posicion) else { return };
        let cr = self.estado.reglas.economia.combate.clone();
        let nivel = nivel_al_azar(&cr, &mut self.estado.rng);
        self.estado.combate.reaparecen.remove(&plaza);
        let id = self.poner_bandidos(centro, nivel, OrigenBandidos::Plaza(plaza), Some(bosque), t);
        self.aparecio(id, t, vec![plaza]);
    }

    /// Vence la cita del anillo del campamento de mercenarios `c`.
    pub(super) fn bandidos_del_anillo(&mut self, c: u32, t: Instante) {
        let Some(centro) = self.geografia.as_ref().and_then(|g| g.mapa.campamentos.get(c as usize).copied()) else { return };
        if self.estado.combate.anillos.get(&c).is_some_and(|&desde| t < desde) {
            return;
        }
        let cr = self.estado.reglas.economia.combate.clone();
        // Cuentan los residentes cuya Facción no tiene plazas o que no tienen Facción: son los que necesitan bandidos que los acosen.
        let sin_plaza = self.heroes().filter(|h| h.heroe.residencia == Some(militar::heroe::Residencia::Campamento(c)) && h.faccion.is_none_or(|f| self.plazas_de(f).next().is_none())).count() as i64;
        let existentes = self.estado.combate.bandidos.values().filter(|b| b.origen == OrigenBandidos::Anillo(c)).count() as i64;
        if existentes >= campamentos_que_tocan(&cr, sin_plaza) {
            return;
        }
        let valido = |s: &Self, p: Punto| s.geografia.as_ref().is_some_and(|g| g.es_tierra_firme(p)) && s.zona_en(p, None) == Zona::Ninguna && !s.en_proteccion_de_campamento(p);
        // El sorteo del punto presta el generador de la partida: se saca del estado mientras se mira el mapa y se devuelve con lo que gastó.
        let mut rng = std::mem::replace(&mut self.estado.rng, base::Rng::nuevo(0));
        let punto = punto_del_anillo(&cr, centro, &mut rng, |p| valido(self, p));
        self.estado.rng = rng;
        let Some(punto) = punto else {
            // Sin sitio: las tiradas de los intentos se gastaron igual (el TypeScript también las gastaba) y se reintenta más tarde.
            self.programar_anillo(c, proxima_aparicion_en_el_anillo(&cr, t));
            return;
        };
        let nivel = nivel_al_azar(&cr, &mut self.estado.rng);
        let id = self.poner_bandidos(punto, nivel, OrigenBandidos::Anillo(c), None, t);
        let siguiente = proxima_aparicion_en_el_anillo(&cr, t);
        self.estado.combate.anillos.insert(c, siguiente);
        self.programar_anillo(c, siguiente);
        self.aparecio(id, t, vec![]);
    }

    fn aparecio(&mut self, id: u64, t: Instante, plazas: Vec<u64>) {
        let b = &self.estado.combate.bandidos[&id];
        let (punto, nivel) = (b.posicion, b.nivel);
        self.tr.eventos.push(EventoNucleo { codigo: Cow::Borrowed("bandidos.campamento_aparece"), en: t, ejercitos: vec![], asentamientos: plazas, punto: Some(punto), datos: vec![("campamento", id as i64), ("nivel", nivel as i64)] });
    }

    // ───────────────────────────── El acoso ─────────────────────────────

    /// Programa los acosos de `entidad` (un campamento de bandidos o una caravana) con su `plan`: cada cruce a ≤ `radioAtaqueCaravana` con una caravana (si
    /// es un campamento) o con un campamento (si es una caravana) es un evento exacto de prioridad 7.
    pub(super) fn programar_acosos(&mut self, entidad: u64, plan: &Plan, entradas: Entradas) {
        let (t, radio) = (self.estado.ahora, self.estado.reglas.economia.combate.bandidos.radio_ataque_caravana);
        let es_campamento = es_entidad_bandidos(entidad);
        let clave = cola::Clave { entidad, generacion: self.estado.generaciones.actual(entidad) };
        for e in self.espacio.rejilla.encuentros_de(entidad, plan, radio, entradas) {
            let pareja = if es_campamento { self.atrae_bandidos(e.otra) } else { es_entidad_bandidos(e.otra) };
            if pareja {
                self.estado.cola.programar(e.instante.max(t), PRIO_BANDIDOS, clave, Carga::Encuentro { otra: e.otra, otra_generacion: e.otra_generacion });
            }
        }
    }

    /// ¿Atrae a los bandidos? Una caravana que viaja suelta y una columna que lleva caravanas adjuntas.
    pub(super) fn atrae_bandidos(&self, entidad: u64) -> bool {
        match self.cosa(entidad) {
            Some(Cosa::Caravana(k)) => matches!(k.estado, EstadoCaravana::EnTransito | EstadoCaravana::Retornando),
            Some(Cosa::Columna(e)) => !e.caravanas_adjuntas.is_empty(),
            _ => false,
        }
    }

    /// Vence el cruce de un campamento con una caravana suelta o con una columna que lleva caravanas (`entidad` y `otra` son los dos, en cualquier orden).
    pub(super) fn acoso_vence(&mut self, entidad: u64, otra: u64, otra_generacion: u32, t: Instante) {
        if self.estado.generaciones.actual(otra) != otra_generacion {
            return;
        }
        let (camp, presa) = if es_entidad_bandidos(entidad) { (entidad, otra) } else { (otra, entidad) };
        let idc = crate::estado_comercio::id_de_entidad(camp);
        if !es_entidad_bandidos(camp) || !self.estado.combate.bandidos.contains_key(&idc) {
            return;
        }
        if es_entidad_caravana(presa) {
            let idk = crate::estado_comercio::id_de_entidad(presa);
            self.acosar(idc, idk, None, t);
        } else if self.estado.ejercitos.get(&presa).is_some_and(|e| self.columna_en_el_mapa(e) && !self.acoso_ya_jugado(idc, e.id)) {
            self.acosar_columna(idc, presa, t);
        }
    }

    /// ¿Es presa de los bandidos una caravana en ese estado y en ese punto? No si está parada en una plaza (`disponible`, `preparando`, `aparcada`), ni dentro de cualquier zona, ni en la
    /// protección de un campamento de mercenarios. La distancia ya la garantiza el evento (exacto); la de las posiciones redondeadas podría pasarse de una milésima.
    pub(super) fn es_presa_de_bandidos(&self, estado: EstadoCaravana, donde: Punto) -> bool {
        puede_acosar(&self.estado.reglas.economia.combate, estado, 0, self.zona_en(donde, None) != Zona::Ninguna, self.en_proteccion_de_campamento(donde))
    }

    /// El campamento `idc` acosa a la columna `columna`, que lleva caravanas adjuntas. Con servidores de batalla y algún héroe sano a bordo, los bandidos y la columna se juegan **todas** las
    /// caravanas adjuntas en una sola batalla de Unity (quinta ronda de Entrar con todo, 2026-10-22; la columna queda apartada y ahí acaba el cruce). Si no, cada caravana adjunta tiene su
    /// propia tirada con números, en orden de id, y se defiende con el poder de la columna. Los bandidos no son bots ni jugadores: sin héroes, siempre con números.
    fn acosar_columna(&mut self, idc: u64, columna: u64, t: Instante) {
        if self.estado.reglas.batallas_en_unity {
            let donde = self.estado.ejercitos[&columna].posicion_en(t);
            if let Some(ap) = self.apertura_de_acoso(t, idc, columna, donde) {
                self.abrir_batalla(t, ap);
                return;
            }
        }
        for idk in self.estado.ejercitos[&columna].caravanas_adjuntas.clone() {
            self.acosar(idc, idk, Some(columna), t);
        }
    }

    /// El campamento `idc` acosa con una tirada con números a la caravana `idk`, que se defiende sola (con su escolta cedida o con los carreteros) o, si va adjunta a la columna `ejercito`,
    /// con el poder de esa columna (que no sufre bajas).
    fn acosar(&mut self, idc: u64, idk: u64, ejercito: Option<u64>, t: Instante) {
        let (Some(b), Some(k)) = (self.estado.combate.bandidos.get(&idc), self.estado.comercio.caravana(idk)) else { return };
        let (nivel, origen, estado) = (b.nivel, k.origen as u64, k.estado);
        let donde = match ejercito {
            Some(c) => self.estado.ejercitos[&c].posicion_en(t),
            None => self.posicion_de_caravana(idk, t).unwrap_or(k.posicion),
        };
        if !self.es_presa_de_bandidos(estado, donde) {
            return;
        }
        let cr = self.estado.reglas.economia.combate.clone();
        let mut escolta: Vec<Escuadra> = k.escolta.iter().filter_map(|c| self.escuadra(c.id).map(|(_, s)| s.clone())).collect();
        let de_la_columna: Vec<Escuadra> = ejercito.map(|c| self.escuadras_de_columna(c).into_iter().cloned().collect()).unwrap_or_default();
        let defensa = match ejercito {
            Some(_) => Defensa::Ejercito(&de_la_columna),
            None if escolta.iter().any(Escuadra::con_soldados) => Defensa::Escolta(&mut escolta),
            None => Defensa::Base,
        };
        let acoso = resolver_acoso(&cr, nivel, defensa, &mut self.estado.rng);
        self.volcar_escuadras(&escolta);
        let plazas = vec![origen].into_iter().filter(|o| self.estado.asentamientos.contains_key(o)).collect();
        let datos = vec![("campamento", idc as i64), ("caravana", idk as i64), ("poderBandido", acoso.poder_bandido), ("defensa", acoso.defensa), ("bajasEscolta", acoso.bajas_escolta.iter().map(|b| b.antes - b.despues).sum())];
        self.tr.eventos.push(EventoNucleo { codigo: Cow::Borrowed(if acoso.gana_el_bandido { "bandidos.caravana_interceptada" } else { "bandidos.caravana_escapa" }), en: t, ejercitos: ejercito.into_iter().collect(), asentamientos: plazas, punto: Some(donde), datos });
        if acoso.gana_el_bandido {
            // La caravana desaparece con su carga; su escolta (a 0) vuelve al campamento de su héroe al perderse.
            self.perder_caravana(t, idk);
        }
    }

    // ───────────────────────────── La orden de administración ─────────────────────────────

    /// `colocarBandidos` (escenarios y pruebas): un campamento de ese nivel, fuera de toda zona y de la protección de un campamento de mercenarios, donde no
    /// aparecen bandidos de verdad.
    pub(super) fn colocar_bandidos(&mut self, t: Instante, p: &Peticion, posicion: Punto, nivel: u8) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        if !p.admin {
            return Err(Respuesta::rechazo(rechazos::NO_AUTORIZADO, "solo administración coloca campamentos de bandidos", v));
        }
        if !(1..=3).contains(&nivel) {
            return Err(Respuesta::rechazo(rechazos::BANDIDOS_NIVEL_INVALIDO, "el nivel de un campamento de bandidos va de 1 a 3", v));
        }
        if self.zona_en(posicion, None) != Zona::Ninguna || self.en_proteccion_de_campamento(posicion) {
            return Err(Respuesta::rechazo(rechazos::BANDIDOS_SOBRE_UNA_ZONA, "los bandidos no actúan dentro de una zona de influencia ni junto a un campamento de mercenarios", v));
        }
        let id = self.poner_bandidos(posicion, nivel, OrigenBandidos::Admin, None, t);
        self.aparecio(id, t, vec![]);
        Ok(vec![id])
    }
}
