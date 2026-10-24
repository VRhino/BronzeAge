//! Atacar (Combate §2-§7, fase 4D): `atacar` con sus seis objetivos y la resolución con números —sin servidores de batalla; la batalla de Unity es de la fase
//! 4E— con todas sus consecuencias: bajas permanentes y experiencia de escuadra, chatarra, heridos, la mitad del carro que pierde el derrotado y el botín
//! que se lleva el vencedor. Las reglas puras son el crate `combate`; aquí se mira el estado, se comprueba **todo** antes de tocar nada o de gastar una
//! tirada (una orden rechazada no deja rastro) y se vuelcan los resultados.
//!
//! # Qué se ataca
//! - **Una columna** (`atacar_columna`): de su misma clase, enemiga, en el mapa y con algún héroe sano. Gana quien dice la resolución por poder (dos tiradas).
//! - **Una caravana** (`atacar_caravana`): la que viaja suelta (no la adjunta a un ejército: se ataca al ejército). El servidor decide el tipo por la zona en
//!   que está **en el instante del ataque** ([`Nucleo::en_zona`]): aguante (B) en la zona del dueño o de un aliado, disputa de carros (A) en el resto. Una tirada.
//! - **Un campamento de bandidos** (`atacar_campamento`): PvE, sin guerra. Una tirada contra el poder de su nivel.
//! - **Una plaza** (`atacar_plaza`): solo un ejército. Abre la batalla del asedio con su convocatoria de 30 s, también sin servidores de batalla (M8): al acabar,
//!   [`Nucleo::resolver_asedio_con_numeros`] lo resuelve con todas las columnas que se unieron. Se asedia con la guarnición y los defensores en persona —los que están **dentro**: una columna amiga a la puerta, en el
//!   mapa, no defiende (2026-10-22)—; el defensor lleva el multiplicador de su muralla ([`Nucleo::multiplicador_de_plaza`]: el recinto de referencia, `1` sin recinto; la columna,
//!   el pueblo, la caravana y la avanzada no lo tienen); si cae, [`conquistas`](super::conquistas).
//! - **Un pueblo extractor** (`atacar_pueblo`): un saqueo, nunca una conquista. Defienden los 30 mineros de serie y los héroes que han entrado en él.
//! - **Una avanzada** (`atacar_avanzada`, en [`avanzadas`](super::avanzadas)): solo un ejército, como un asedio contra su guarnición de serie y la de jugadores y los héroes que han
//!   entrado en ella; unas ruinas caen sin combate. Si gana el atacante, se lleva el almacén que le quepa y la avanzada se destruye o queda en ruinas.
//!
//! # Quien defiende y pierde (Entrar con todo, 2026-10-22)
//! Los defensores en persona quedan heridos y **todos** pierden la mitad del carro, residentes o visitantes amigos que defendieron desde dentro (tercera ronda, 2026-10-22); es del
//! vencedor ([`Nucleo::pierden_la_mitad_del_carro`]). A un bando solo se une quien está en su sitio: dentro para defender, fuera para atacar.
//!
//! # Lo que se mira antes (en este orden)
//! El héroe (sano, a bordo, columna fuera de batalla), el objetivo (existe, está en el mapa), el alcance (`radioEncuentro`), la protección de los campamentos
//! de mercenarios (`radioProteccion`: nadie inicia un combate a esa distancia, ni desde allí ni contra lo que esté allí) y la guerra. Entre Facciones
//! distintas hace falta guerra; `declara_guerra` la declara si falta, pero solo cuando todo lo demás ya vale (las relaciones son un andamiaje de
//! administración hasta el sistema político, sin arrastre). Una columna sin Facción ni exige ni declara guerra: no hay a quién.

use super::persecuciones::Fin;
use super::*;
use crate::comando::{Detalle, ObjetivoAtaque};
use crate::estado::{SucesoCombate, PRIO_UMBRAL};
use crate::estado_combate::entidad_guarnicion;
use crate::estado_militar::Relacion;
use asentamiento::recintos;
use combate::asedio::{self, Asedio};
use combate::botin::{cargar_botin, tras_derrota, CarroDeColumna};
use combate::caravana::{self as asalto, Defensa, Zona};
use combate::conquista::esta_protegida;
use combate::pueblo as reglas_pueblo;
use combate::resolucion::{resolver_combate, Baja, Bando, Informe, Lado, SIN_DUENO};
use combate::seleccion::{seleccionar, Candidato};
use militar::escuadra::{Contenedor, Escuadra};
use asentamiento::Recurso;
use militar::presencia::Ubicacion;
use militar::{sumar, total, Carga as Mercancia};

/// Un bando antes de combatir: copias de sus escuadras (el combate las modifica en el sitio y luego se vuelcan al estado, [`Nucleo::volcar_escuadras`]) y
/// los héroes que combaten con ellas.
pub(super) struct Tropa {
    pub escuadras: Vec<Escuadra>,
    pub heroes: Vec<u64>,
}

/// Quién defiende con números lo que hay dentro de una plaza, un pueblo o una avanzada (Combate §3.2): los héroes de dentro y las escuadras con las que luchan.
pub(super) struct DefensoresDeDentro {
    pub tropa: Tropa,
    /// Cada héroe que defiende en persona con las escuadras con las que lucha: si la plaza cae, salen con él.
    pub lucharon: Vec<(u64, Vec<u64>)>,
}

impl DefensoresDeDentro {
    pub(super) fn nadie() -> Self {
        Self { tropa: Tropa { escuadras: vec![], heroes: vec![] }, lucharon: vec![] }
    }
}

/// Lo que cambia con unas bajas: las columnas cuyas escuadras combatieron y la velocidad que llevaban (para recomponer su consumo y su marcha).
type Columnas = Vec<(u64, Mil)>;

/// Las escuadras de su loadout activo que el héroe tiene en el campamento, con soldados y fuera de la guarnición: las que defienden en persona.
pub(super) fn loadout_en_el_campamento(h: &HeroeNucleo) -> Vec<&Escuadra> {
    let activas = h.heroe.escuadras_del_loadout_activo();
    h.heroe.escuadras.iter().filter(|s| activas.contains(&s.id) && s.contenedor == Contenedor::Campamento && !s.en_guarnicion && s.con_soldados()).collect()
}

/// Las escuadras con soldados del héroe que van en una columna (la que trajo dentro, si es un visitante): con las que defiende de visita.
pub(super) fn de_su_columna(h: &HeroeNucleo) -> Vec<&Escuadra> {
    h.heroe.escuadras.iter().filter(|s| matches!(s.contenedor, Contenedor::Ejercito(_)) && s.con_soldados()).collect()
}

pub(super) fn caidos(bajas: &[Baja]) -> i64 {
    bajas.iter().map(|b| b.antes - b.despues).sum()
}

impl Nucleo {
    // ───────────────────────────── Consultas ─────────────────────────────

    /// ¿Está `p` a `radioProteccion` o menos de algún campamento de mercenarios? Ahí nadie inicia un combate.
    pub fn en_proteccion_de_campamento(&self, p: Punto) -> bool {
        let radio = self.estado.reglas.economia.militar.mercenarios.radio_proteccion;
        self.geografia.as_ref().is_some_and(|g| g.mapa.campamentos.iter().any(|c| c.dist(p) <= radio))
    }

    /// ¿Está la plaza protegida por una conquista reciente (inatacable)?
    pub fn plaza_protegida(&self, plaza: u64) -> bool {
        self.estado.asentamientos.get(&plaza).is_some_and(|a| esta_protegida(a.protegida_hasta, self.estado.ahora))
    }

    /// Dónde está una caravana respecto a las zonas de influencia, en `t`, mirando desde el dueño (Combate §7.1): en la zona de una plaza suya o de una de su
    /// alianza o vasallaje (aguante), en la de cualquier otra (disputa) o fuera de todas.
    pub fn zona_de_caravana(&self, caravana: u64, t: Instante) -> Option<Zona> {
        let c = self.estado.comercio.caravana(caravana)?;
        let dueno = self.estado.asentamientos.get(&(c.origen as u64)).map(|a| a.faccion);
        Some(self.zona_en(self.posicion_de_caravana(caravana, t).unwrap_or(c.posicion), dueno))
    }

    pub(super) fn zona_en(&self, p: Punto, dueno: Option<u32>) -> Zona {
        let mut zona = Zona::Ninguna;
        // Solo las plazas cuyo disco contiene el punto pueden tenerlo en su zona real (el polígono recortado cabe en el disco).
        for a in self.estado.asentamientos.values().filter(|a| a.posicion.dist(p) <= a.radio) {
            if !self.en_zona(a.id, p) {
                continue;
            }
            let amiga = dueno.is_some_and(|d| d == a.faccion || matches!(self.estado.militar.relacion(d, a.faccion), Some(Relacion::Alianza | Relacion::Vasallaje)));
            if amiga {
                return Zona::DelDuenoOAliado;
            }
            zona = Zona::Ajena;
        }
        zona
    }

    // ───────────────────────────── Ayudas ─────────────────────────────

    /// Los héroes sanos de la columna que entran en la batalla (como mucho `tope`, los de más nivel y escuadras más fuertes) con sus escuadras, en el orden en
    /// que la columna las lleva. Las de un herido no combaten y las de los apartados vuelven intactas (no están en la copia).
    pub(super) fn tropa_de_columna(&self, id: u64, t: Instante, tope: usize) -> Tropa {
        self.tropa_de_columnas(&[id], t, tope)
    }

    /// Lo mismo con varias columnas que combaten juntas (las de un asedio con convocatoria): el tope es del bando entero.
    pub(super) fn tropa_de_columnas(&self, columnas: &[u64], t: Instante, tope: usize) -> Tropa {
        let cr = &self.estado.reglas.economia.combate;
        let candidatos: Vec<Candidato> = columnas
            .iter()
            .flat_map(|&id| self.estado.ejercitos[&id].columna.participantes.iter().map(move |p| (id, p.heroe)))
            .filter_map(|(id, h)| self.heroe(h).map(|h| (id, h)))
            .filter(|(_, h)| !h.heroe.esta_herido(t))
            .map(|(id, h)| {
                let suyas: Vec<&Escuadra> = h.heroe.escuadras.iter().filter(|s| s.contenedor == Contenedor::Ejercito(id)).collect();
                Candidato::de(cr, &h.heroe, &suyas)
            })
            .collect();
        let entran = seleccionar(&candidatos, tope).entran;
        let escuadras = columnas.iter().flat_map(|id| &self.estado.ejercitos[id].columna.escuadras).filter_map(|&s| self.escuadra(s).map(|(_, esc)| esc)).filter(|s| entran.contains(&s.heroe)).cloned().collect();
        Tropa { escuadras, heroes: entran }
    }

    /// Las columnas de las escuadras dadas y la velocidad que llevan ahora.
    pub(super) fn columnas_de(&self, escuadras: &[Escuadra]) -> Columnas {
        let mut v: Columnas = escuadras.iter().filter_map(|s| if let Contenedor::Ejercito(c) = s.contenedor { Some(c) } else { None }).collect::<std::collections::BTreeSet<_>>().into_iter().map(|c| (c, 0)).collect();
        for (c, vel) in v.iter_mut() {
            *vel = self.velocidad_de_ejercito(*c).unwrap_or(0);
        }
        v
    }

    /// Devuelve al estado las escuadras que combatieron (bajas, experiencia y nivel). Las que no son de ningún héroe no están en él.
    pub(super) fn volcar_escuadras(&mut self, escuadras: &[Escuadra]) {
        for s in escuadras.iter().filter(|s| s.heroe != SIN_DUENO) {
            if let Some(slot) = self.estado.militar.heroes.get_mut(&s.heroe).and_then(|h| h.heroe.escuadra_mut(s.id)) {
                *slot = s.clone();
                self.tr.tocar(Clase::Heroe, s.heroe);
            }
        }
    }

    /// Tras unas bajas, cada columna vuelve a anotar lo que come y a qué velocidad marcha.
    pub(super) fn recomponer_tras_bajas(&mut self, columnas: &Columnas, t: Instante) {
        for &(c, antes) in columnas {
            if self.estado.ejercitos.contains_key(&c) {
                self.recomponer_columna(c, t, antes);
            }
        }
    }

    /// Hiere a los héroes (`heridoMinutos`; a quien ya lo está no se le alarga). Una columna que se queda sin ningún héroe sano no persigue ni es perseguida
    /// (Combate §11): su persecución y las que iban tras ella terminan.
    pub(super) fn herir_heroes(&mut self, heroes: &[u64], t: Instante) -> usize {
        let mr = self.estado.reglas.economia.militar.clone();
        let mut heridos = 0;
        let mut columnas = std::collections::BTreeSet::new();
        for &h in heroes {
            let Some(hn) = self.estado.militar.heroes.get_mut(&h) else { continue };
            self.tr.tocar(Clase::Heroe, h);
            heridos += hn.heroe.herir(&mr, t) as usize;
            columnas.extend(hn.columna);
        }
        for c in columnas {
            let Some(e) = self.estado.ejercitos.get(&c) else { continue };
            if !self.tiene_heroe_sano(e, t) {
                self.fijar_persecucion(c, None);
                self.soltar_perseguidores(c, Fin::ACubierto);
            }
        }
        heridos
    }

    /// El vencedor carga el botín en el carro de su columna hasta donde le quepa; lo que no cabe se pierde. Devuelve lo que cogió.
    pub(super) fn cargar_botin_en(&mut self, columna: u64, botin: &Mercancia) -> Mercancia {
        let capacidad = self.capacidad_de_carga(columna).unwrap_or(0);
        let Some(e) = self.estado.ejercitos.get_mut(&columna) else { return Mercancia::new() };
        let cogido = cargar_botin(&mut e.columna.carga, botin, capacidad);
        self.tr.tocar(Clase::Ejercito, columna);
        cogido
    }

    /// El derrotado pierde la mitad de su carro (`trasDerrota`): devuelve lo robado. Una columna personal que pierde suelta además su presa.
    pub(super) fn perder_la_mitad_del_carro(&mut self, columna: u64) -> Mercancia {
        let cr = self.estado.reglas.economia.combate.clone();
        let Some(e) = self.estado.ejercitos.get_mut(&columna) else { return Mercancia::new() };
        let robado = tras_derrota(&cr, &mut e.columna.carga);
        let personal = e.columna.tipo == militar::columna::TipoColumna::Personal;
        self.tr.tocar(Clase::Ejercito, columna);
        if personal {
            self.fijar_persecucion(columna, None);
        }
        robado
    }

    /// Todo defensor en persona que pierde, sea residente o visitante amigo que defendió desde dentro, pierde la mitad de su carro (Entrar con todo, 2026-10-22, tercera ronda): lo
    /// que llevaba consigo —el carro que no vació y, si conserva una columna, el de esa columna— queda a la mitad, hacia abajo. Devuelve lo robado, para el botín del vencedor.
    pub(super) fn pierden_la_mitad_del_carro(&mut self, defensores: &[u64]) -> Mercancia {
        let cr = self.estado.reglas.economia.combate.clone();
        let (mut robado, mut columnas) = (Mercancia::new(), std::collections::BTreeSet::new());
        for &h in defensores {
            let Some(hn) = self.estado.militar.heroes.get_mut(&h) else { continue };
            self.tr.tocar(Clase::Heroe, h);
            for (r, n) in tras_derrota(&cr, &mut hn.carro) {
                sumar(&mut robado, r, n);
            }
            columnas.extend(hn.columna);
        }
        for c in columnas {
            let Some(e) = self.estado.ejercitos.get_mut(&c) else { continue };
            for (r, n) in tras_derrota(&cr, &mut e.columna.carga) {
                sumar(&mut robado, r, n);
            }
            self.tr.tocar(Clase::Ejercito, c);
        }
        robado
    }

    /// El vencedor carga en el carro de su columna, hasta donde le quepa, lo que el derrotado perdió (nada si no hubo nada).
    pub(super) fn entregar_lo_robado(&mut self, columna: u64, robado: &Mercancia) {
        if !robado.is_empty() {
            self.cargar_botin_en(columna, robado);
        }
    }

    pub(super) fn hecho_de_combate(&mut self, codigo: &'static str, t: Instante, columnas: Vec<u64>, plazas: Vec<u64>, punto: Punto, datos: Vec<(&'static str, i64)>) {
        self.tr.emitir(&self.estado, EventoNucleo { codigo: Cow::Borrowed(codigo), en: t, ejercitos: columnas, asentamientos: plazas, punto: Some(punto), datos, ..Default::default() });
    }

    // ───────────────────────────── Los plazos ─────────────────────────────

    /// Vence un plazo del combate. La clave de la cola ya garantiza que la entidad no cambió de generación.
    pub(super) fn suceso_de_combate(&mut self, entidad: u64, suceso: SucesoCombate, t: Instante) {
        let id = crate::estado_comercio::id_de_entidad(entidad);
        match suceso {
            SucesoCombate::BandidosDeLaPlaza => self.bandidos_de_la_plaza(entidad, t),
            SucesoCombate::BandidosDelAnillo => self.bandidos_del_anillo(id as u32, t),
            SucesoCombate::FinProteccion => self.fin_de_proteccion(id, t),
            SucesoCombate::FinOcupacion => self.fin_de_ocupacion(id, t),
            SucesoCombate::ReponeGuarnicion => self.repone_guarnicion(entidad, t),
            SucesoCombate::FinInvulnerabilidad => self.fin_de_invulnerabilidad(entidad, t),
            SucesoCombate::ReponeAvanzada => self.repone_avanzada(entidad, t),
            SucesoCombate::FinObraAvanzada => self.fin_de_obra_de_avanzada(entidad, t),
        }
    }

    // ───────────────────────────── La orden ─────────────────────────────

    /// `atacar`. Todo se comprueba antes de tocar nada.
    pub(super) fn atacar(&mut self, t: Instante, p: &Peticion, heroe: u64, objetivo: ObjetivoAtaque, declara_guerra: bool) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: &str| Respuesta::rechazo(c, m, v);
        self.autorizar_heroe(p, heroe, false)?;
        let (id, e) = self.columna_a_bordo(heroe)?;
        if e.en_batalla {
            return Err(rechazo(rechazos::COLUMNA_EN_BATALLA, "la columna está apartada por una batalla"));
        }
        if e.columna.es_formacion() {
            return Err(rechazo(rechazos::COLUMNA_EN_FORMACION, "una formación no se mueve ni combate: espera a ser un ejército"));
        }
        if self.heroe(heroe).is_some_and(|h| h.heroe.esta_herido(t)) {
            return Err(rechazo(rechazos::HEROE_HERIDO, "estás herido: no puedes entrar en batalla"));
        }
        match objetivo {
            ObjetivoAtaque::Ejercito(x) => self.atacar_columna(t, heroe, id, x, declara_guerra)?,
            ObjetivoAtaque::Caravana(x) => self.atacar_caravana(t, heroe, id, x, declara_guerra)?,
            ObjetivoAtaque::Campamento(x) => self.atacar_campamento(t, heroe, id, x)?,
            ObjetivoAtaque::Asentamiento(x) => self.atacar_plaza(t, heroe, id, x, declara_guerra)?,
            ObjetivoAtaque::Pueblo(x) => self.atacar_pueblo(t, heroe, id, x, declara_guerra)?,
            ObjetivoAtaque::Avanzada(x) => self.atacar_avanzada(t, heroe, id, x, declara_guerra)?,
        }
        Ok(vec![])
    }

    /// Lo que toda agresión comprueba: el alcance (`radioEncuentro`), la protección de los campamentos y la guerra. Devuelve la guerra que hay que declarar
    /// (entre las dos Facciones) si falta una y la orden la pide; no declara nada: eso se hace cuando todo lo demás ya vale.
    pub(super) fn comprobar_agresion(&self, e: &Ejercito, t: Instante, donde: Punto, contra: Option<u32>, declara_guerra: bool) -> Result<Option<(u32, u32)>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let aqui = e.posicion_en(t);
        let radio = self.estado.reglas.economia.militar.logistica.radio_encuentro;
        if aqui.dist(donde) > radio {
            return Err(rechazo(rechazos::COMBATE_LEJOS, format!("hay que estar a {radio} o menos para atacar")));
        }
        if self.en_proteccion_de_campamento(aqui) || self.en_proteccion_de_campamento(donde) {
            return Err(rechazo(rechazos::COMBATE_PROTECCION_DE_CAMPAMENTO, "junto a un campamento de mercenarios nadie inicia un combate".into()));
        }
        let (Some(yo), Some(otra)) = (e.columna.faccion, contra) else { return Ok(None) };
        if yo == otra {
            return Err(rechazo(rechazos::COMBATE_PROPIA, "no se ataca a los tuyos".into()));
        }
        match self.estado.militar.relacion(yo, otra) {
            Some(Relacion::Alianza) => Err(rechazo(rechazos::COMBATE_ALIADA, "no se ataca a un aliado".into())),
            Some(Relacion::Guerra) => Ok(None),
            _ if declara_guerra => Ok(Some((yo, otra))),
            _ => Err(Respuesta::rechazo_con(
                rechazos::GUERRA_CONFIRMAR,
                format!("la Facción {yo} no está en guerra con la {otra}: atacar con `declara_guerra` la declara (y la deja en guerra con ella, sin arrastre de aliados ni vasallos)"),
                v,
                Some(Detalle::Guerra { objetivo: otra, arrastre: vec![] }),
            )),
        }
    }

    /// Declara la guerra entre dos Facciones (`declara_guerra` de `atacar`): la relación pasa a ser de guerra y se cuenta.
    pub(super) fn declarar_guerra(&mut self, t: Instante, atacante: u32, contra: u32) {
        self.estado.militar.relaciones.insert((atacante.min(contra), atacante.max(contra)), Relacion::Guerra);
        self.tr.tocar_todos(Clase::Faccion, [u64::from(atacante), u64::from(contra)]);
        self.tr.tocar(Clase::Relacion, Clase::par(atacante, contra));
        let datos = vec![("atacante", atacante as i64), ("contra", contra as i64)];
        self.tr.emitir(&self.estado, EventoNucleo { facciones: vec![atacante, contra], ..EventoNucleo::nuevo("faccion.guerra_declarada", t, datos) });
    }

    // ───────────────────────────── Columna contra columna ─────────────────────────────

    /// Combate §5. Las dos columnas entran con sus héroes sanos (como mucho 5 por bando) y gana quien dice el poder; el perdedor pierde la mitad de su carro y
    /// todos sus héroes quedan heridos; el ganador carga lo robado y la chatarra hasta donde le quepa.
    fn atacar_columna(&mut self, t: Instante, heroe: u64, id: u64, x: u64, declara_guerra: bool) -> Result<(), Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: &str| Respuesta::rechazo(c, m, v);
        let e = &self.estado.ejercitos[&id];
        let o = self.estado.ejercitos.get(&x).ok_or_else(|| Respuesta::rechazo(rechazos::EJERCITO_DESCONOCIDO, ids::ejercito(x), v))?;
        if x == id {
            return Err(rechazo(rechazos::COMBATE_PROPIA, "esa columna es la tuya"));
        }
        if !self.columna_en_el_mapa(o) {
            return Err(rechazo(rechazos::COMBATE_A_CUBIERTO, "esa columna no está en el mapa"));
        }
        if e.columna.tipo != o.columna.tipo {
            return Err(rechazo(rechazos::COMBATE_CLASE_DISTINTA, "un ejército y una columna personal no combaten entre sí"));
        }
        if !self.tiene_heroe_sano(o, t) {
            return Err(rechazo(rechazos::COMBATE_SIN_HEROE_SANO, "esa columna solo lleva héroes heridos: no se la puede tocar"));
        }
        let guerra = self.comprobar_agresion(e, t, o.posicion_en(t), o.columna.faccion, declara_guerra)?;
        if self.estado.reglas.batallas_en_unity {
            let ap = self.apertura_de_columna(t, heroe, id, x);
            self.abrir_batalla_de_ataque(t, ap, guerra);
            return Ok(());
        }

        let cr = self.estado.reglas.economia.combate.clone();
        let (mut atac, mut def) = (self.tropa_de_columna(id, t, cr.batalla.capacidad_resto), self.tropa_de_columna(x, t, cr.batalla.capacidad_resto));
        let (columnas, punto) = ([self.columnas_de(&atac.escuadras), self.columnas_de(&def.escuadras)].concat(), self.estado.ejercitos[&id].posicion_en(t));
        let (participantes_a, participantes_d) = (Self::participantes_de(&self.estado.ejercitos[&id]), Self::participantes_de(&self.estado.ejercitos[&x]));
        if let Some((a, b)) = guerra {
            self.declarar_guerra(t, a, b);
        }
        let informe = resolver_combate(&cr, Bando { escuadras: &mut atac.escuadras, heroes: &atac.heroes }, Bando { escuadras: &mut def.escuadras, heroes: &def.heroes }, asedio::SIN_RECINTO, &mut self.estado.rng)
            .expect("los dos bandos llevan al menos un héroe sano");
        self.volcar_escuadras(&atac.escuadras);
        self.volcar_escuadras(&def.escuadras);

        let (ganador, perdedor, vencidos) = match informe.ganador {
            Lado::Atacante => (id, x, participantes_d),
            Lado::Defensor => (x, id, participantes_a),
        };
        // El atacante que gana ya no persigue a quien acaba de atacar.
        if self.estado.ejercitos[&id].persiguiendo == Some(x) {
            self.fijar_persecucion(id, None);
        }
        let robado = self.perder_la_mitad_del_carro(perdedor);
        let mut botin = robado.clone();
        informe.chatarra.iter().for_each(|(&r, &n)| sumar(&mut botin, r, n));
        self.adjuntas_de_la_derrotada(t, perdedor).iter().for_each(|(&r, &n)| sumar(&mut botin, r, n));
        let cogido = self.cargar_botin_en(ganador, &botin);
        let heridos = self.herir_heroes(&vencidos, t);
        self.recomponer_tras_bajas(&columnas, t);
        self.hecho_de_combate(
            "combate.resuelto",
            t,
            vec![id, x],
            vec![],
            punto,
            vec![
                ("ganaElAtacante", (informe.ganador == Lado::Atacante) as i64),
                ("poderAtacante", informe.atacante.poder),
                ("poderDefensor", informe.defensor.poder),
                ("bajasAtacante", caidos(&informe.atacante.bajas)),
                ("bajasDefensor", caidos(&informe.defensor.bajas)),
                ("heridos", heridos as i64),
                ("robado", total(&robado)),
                ("chatarra", total(&informe.chatarra)),
                ("botin", total(&cogido)),
            ],
        );
        // El cruce y quién ganó, para los dos hogares (Combate §5, paso 5).
        let banderas = (self.estado.ejercitos[&id].bandera() as i64, self.estado.ejercitos[&x].bandera() as i64);
        let datos = vec![("ejercitoA", id as i64), ("ejercitoB", x as i64), ("faccionA", banderas.0), ("faccionB", banderas.1), ("ganador", ganador as i64)];
        self.hecho_de_combate("combate.encuentro", t, vec![id, x], vec![], punto, datos);
        Ok(())
    }

    /// La columna derrotada con números lleva caravanas adjuntas: cada una pierde la mitad de sus carros, hacia arriba, con su carga (la que se queda sin carros
    /// desaparece), como en una disputa y como con Unity (decisión del autor, 2026-10-24; Combate §5). Avisa con `ejercito.caravanas_perdidas` y devuelve lo perdido,
    /// que es botín del vencedor.
    fn adjuntas_de_la_derrotada(&mut self, t: Instante, columna: u64) -> Mercancia {
        let adjuntas = self.estado.ejercitos[&columna].caravanas_adjuntas.clone();
        let mut perdida = Mercancia::new();
        if adjuntas.is_empty() {
            return perdida;
        }
        let mut carros = 0;
        for &k in &adjuntas {
            let p = self.caer_caravana(t, k, asalto::Modalidad::Disputa, None);
            carros += p.carros_perdidos as i64;
            p.carga_perdida.iter().for_each(|(&r, &n)| sumar(&mut perdida, r, n));
        }
        let datos = vec![("caravanas", adjuntas.len() as i64), ("carros", carros), ("carga", total(&perdida))];
        self.evento_de_columna("ejercito.caravanas_perdidas", t, columna, vec![], datos);
        perdida
    }

    // ───────────────────────────── Interceptar una caravana ─────────────────────────────

    /// Combate §7.1. Ver el módulo: tipo A o B por la zona, una tirada contra la defensa. Gana el atacante: el dueño pierde la mitad de los carros (y en B el
    /// 70 % de la carga), el atacante carga lo perdido hasta donde le quepa y la escolta cedida queda a 0 y vuelve al campamento de su héroe. Gana la caravana:
    /// el atacante pierde el 25 %, la escolta el 5 % y los héroes de la columna quedan heridos.
    fn atacar_caravana(&mut self, t: Instante, heroe: u64, id: u64, x: u64, declara_guerra: bool) -> Result<(), Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: &str| Respuesta::rechazo(c, m, v);
        let e = &self.estado.ejercitos[&id];
        let k = self.estado.comercio.caravana(x).ok_or_else(|| Respuesta::rechazo(rechazos::CARAVANA_DESCONOCIDA, ids::caravana(x), v))?;
        match k.estado {
            EstadoCaravana::Adjunta => return Err(rechazo(rechazos::COMBATE_CARAVANA_ADJUNTA, "esa caravana va adjunta a un ejército: se ataca al ejército")),
            EstadoCaravana::EnTransito | EstadoCaravana::Retornando => {}
            _ => return Err(rechazo(rechazos::COMBATE_A_CUBIERTO, "esa caravana no está en el mapa")),
        }
        let (origen, dueno) = (k.origen as u64, self.estado.asentamientos.get(&(k.origen as u64)).map(|a| a.faccion));
        let donde = self.posicion_de_caravana(x, t).unwrap_or(k.posicion);
        let guerra = self.comprobar_agresion(e, t, donde, dueno, declara_guerra)?;

        let modalidad = asalto::modalidad_de(self.zona_en(donde, dueno));
        if self.estado.reglas.batallas_en_unity {
            let ap = self.apertura_de_caravana(t, heroe, id, x, donde, modalidad);
            self.abrir_batalla_de_ataque(t, ap, guerra);
            return Ok(());
        }
        let cr = self.estado.reglas.economia.combate.clone();
        let mut atac = self.tropa_de_columna(id, t, cr.batalla.capacidad_resto);
        let columnas = self.columnas_de(&atac.escuadras);
        let participantes = Self::participantes_de(&self.estado.ejercitos[&id]);
        let punto = self.estado.ejercitos[&id].posicion_en(t);
        // La escolta cedida defiende con su poder si queda alguien en pie; si no, es como no tener escolta y defienden los carreteros.
        let mut escolta: Vec<Escuadra> = self.estado.comercio.caravana(x).expect("existe").escolta.iter().filter_map(|c| self.escuadra(c.id).map(|(_, s)| s.clone())).collect();
        let con_escolta = escolta.iter().any(|s| s.con_soldados());
        if let Some((a, b)) = guerra {
            self.declarar_guerra(t, a, b);
        }
        let defensa = if con_escolta { Defensa::Escolta(&mut escolta) } else { Defensa::Base };
        let r = asalto::resolver_asalto(&cr, Bando { escuadras: &mut atac.escuadras, heroes: &atac.heroes }, defensa, modalidad, &mut self.estado.rng).expect("la columna lleva al menos un héroe sano");
        self.volcar_escuadras(&atac.escuadras);
        self.volcar_escuadras(&escolta);

        let perdida = r.gana_atacante.then(|| self.caer_caravana(t, x, modalidad, None));
        let (carros, cogido, destruida) = match &perdida {
            Some(p) => (p.carros_perdidos as i64, self.cargar_botin_en(id, &p.carga_perdida), p.destruida),
            None => (0, Mercancia::new(), false),
        };
        let heridos = if r.gana_atacante { 0 } else { self.herir_heroes(&participantes, t) };
        self.recomponer_tras_bajas(&columnas, t);
        self.hecho_de_combate(
            if r.gana_atacante { "combate.caravana_interceptada" } else { "combate.caravana_resiste" },
            t,
            vec![id],
            vec![origen].into_iter().filter(|o| self.estado.asentamientos.contains_key(o)).collect(),
            punto,
            vec![
                ("caravana", x as i64),
                ("aguante", (modalidad == asalto::Modalidad::Aguante) as i64),
                ("poderAtacante", r.poder_atacante),
                ("defensa", r.defensa),
                ("bajasAtacante", caidos(&r.bajas_atacante)),
                ("bajasEscolta", caidos(&r.bajas_escolta)),
                ("carros", carros),
                ("botin", total(&cogido)),
                ("destruida", destruida as i64),
                ("heridos", heridos as i64),
            ],
        );
        Ok(())
    }

    /// La caravana cae en manos del atacante (Combate §7.1, Batalla_Unity_Ciclo §9): según el tipo pierde la mitad de sus carros con su animal o el 70 % de su
    /// carga (sin perder nunca el último carro), su escolta cedida queda a 0 y vuelve al campamento de su héroe y, si se queda sin carros, desaparece. `carros` son los que Unity da por perdidos;
    /// con números se pierden los últimos de la lista. Devuelve lo perdido: la carga se la lleva quien llama.
    pub(super) fn caer_caravana(&mut self, t: Instante, x: u64, modalidad: asalto::Modalidad, carros: Option<&[usize]>) -> asalto::Perdida {
        let cr = self.estado.reglas.economia.combate.clone();
        let flota = self.estado.reglas.economia.comercio.flota.clone();
        let i = self.estado.comercio.posicion_caravana(x).expect("existe");
        let c = &mut self.estado.comercio.flota[i];
        let (origen, antes) = (c.origen as u64, c.velocidad(&flota));
        let mut p = asalto::aplicar_al_dueno(&cr, &flota, c, modalidad, carros).expect("los carros que se pierden existen");
        let cedidas = c.liberar_escolta();
        self.devolver_escolta(&cedidas);
        // La carga sigue a los carros (M10): lo que la columna de una adjunta ya no puede llevar sin los carros perdidos iba en ellos y se pierde con ellos.
        let de_la_columna = if p.destruida {
            self.perder_caravana(t, x)
        } else {
            self.reajustar_plan_en_pausa(x, antes);
            self.tr.tocar(Clase::Caravana, x);
            self.tr.tocar(Clase::Asentamiento, origen);
            let columna = self.estado.ejercitos.iter().find(|(_, e)| e.caravanas_adjuntas.contains(&x)).map(|(&id, _)| id);
            columna.map(|c| self.sacar_lo_que_no_cabe(c)).unwrap_or_default()
        };
        for (r, n) in de_la_columna {
            sumar(&mut p.carga_perdida, r, n);
        }
        p
    }

    // ───────────────────────────── Un campamento de bandidos ─────────────────────────────

    /// Combate §7.2. No exige guerra. Una tirada contra el poder de su nivel: si gana, el campamento cae (oro por cada héroe que combatió, a su carro) y se
    /// agenda su reaparición; si no, todos los héroes de la columna quedan heridos y la columna pierde la mitad del carro (nadie se la queda).
    fn atacar_campamento(&mut self, t: Instante, heroe: u64, id: u64, x: u64) -> Result<(), Respuesta> {
        let v = self.estado.version;
        let b = self.estado.combate.bandidos.get(&x).ok_or_else(|| Respuesta::rechazo(rechazos::CAMPAMENTO_DE_BANDIDOS_DESCONOCIDO, format!("bandidos-{x}"), v))?;
        let (nivel, donde, origen) = (b.nivel, b.posicion, b.origen);
        self.comprobar_agresion(&self.estado.ejercitos[&id], t, donde, None, false)?;
        if self.estado.reglas.batallas_en_unity {
            let ap = self.apertura_de_campamento(t, heroe, id, x);
            self.abrir_batalla_de_ataque(t, ap, None);
            return Ok(());
        }

        let cr = self.estado.reglas.economia.combate.clone();
        let mut atac = self.tropa_de_columna(id, t, cr.batalla.capacidad_resto);
        let columnas = self.columnas_de(&atac.escuadras);
        let participantes = Self::participantes_de(&self.estado.ejercitos[&id]);
        let punto = self.estado.ejercitos[&id].posicion_en(t);
        let r = combate::bandidos::resolver_campamento(&cr, Bando { escuadras: &mut atac.escuadras, heroes: &atac.heroes }, nivel, &mut self.estado.rng).expect("la columna lleva al menos un héroe sano");
        self.volcar_escuadras(&atac.escuadras);

        let (mut oro, mut heridos, mut perdido) = (Mercancia::new(), 0, Mercancia::new());
        if r.destruido {
            for &h in &atac.heroes {
                let ganado = self.oro_de_bandidos_para(h, origen, nivel, t);
                sumar(&mut oro, Recurso::Oro, ganado);
            }
            let cogido = self.cargar_botin_en(id, &oro);
            oro = cogido;
            self.destruir_bandidos(x, t);
        } else {
            heridos = self.herir_heroes(&participantes, t);
            perdido = self.perder_la_mitad_del_carro(id);
        }
        self.recomponer_tras_bajas(&columnas, t);
        self.hecho_de_combate(
            if r.destruido { "combate.campamento_destruido" } else { "combate.ataque_campamento_fallido" },
            t,
            vec![id],
            vec![],
            punto,
            vec![("campamento", x as i64), ("nivel", nivel as i64), ("poderAtacante", r.poder_atacante), ("poderCampamento", r.poder_campamento), ("bajasAtacante", caidos(&r.bajas)), ("oro", total(&oro)), ("heridos", heridos as i64), ("carroPerdido", total(&perdido))],
        );
        Ok(())
    }

    // ───────────────────────────── Asedio ─────────────────────────────

    /// Quién defiende la plaza con números (Combate §3.2): su guarnición (siempre, aunque su dueño esté herido), los residentes que están dentro, sanos y
    /// conectados con las escuadras de su loadout activo que tengan en el campamento (como mucho 15) y, en el cupo que dejan, los visitantes de la propia Facción o de
    /// una aliada, con las escuadras de la columna con la que entraron (no está en el mapa: es lo que trajeron). **Solo defiende lo que está dentro** (2026-10-22):
    /// una columna amiga en el mapa, a la puerta, no cuenta; y un neutral o un enemigo dentro, tampoco.
    pub(super) fn defensa_de_plaza(&self, plaza: u64, t: Instante) -> DefensoresDeDentro {
        let cr = &self.estado.reglas.economia.combate;
        let a = &self.estado.asentamientos[&plaza];
        let tope = cr.batalla.capacidad_asedio;
        let en_pie = |s: &&Escuadra| s.con_soldados();
        let (dentro, sana_y_conectada) = (Ubicacion::Asentamiento(plaza as u32), |h: &HeroeNucleo| !h.heroe.esta_herido(t) && !h.heroe.presencia.fuera);

        let residentes: Vec<&HeroeNucleo> = self.estado.militar.residentes_de(plaza).collect();
        let mut escuadras: Vec<Escuadra> = residentes.iter().flat_map(|h| h.heroe.escuadras.iter()).filter(|s| s.contenedor == Contenedor::Campamento && s.en_guarnicion).filter(en_pie).cloned().collect();
        let candidatos: Vec<Candidato> = residentes.iter().filter(|h| h.heroe.presencia.ubicacion == dentro && sana_y_conectada(h)).map(|h| Candidato::de(cr, &h.heroe, &loadout_en_el_campamento(h))).collect();
        let residentes_que_entran = seleccionar(&candidatos, tope).entran;
        let mut lucharon: Vec<(u64, Vec<u64>)> = vec![];
        for &h in &residentes_que_entran {
            let suyas = loadout_en_el_campamento(self.heroe(h).expect("existe"));
            lucharon.push((h, suyas.iter().map(|s| s.id).collect()));
            escuadras.extend(suyas.into_iter().cloned());
        }

        // Defienden los de la misma Facción, los aliados y los vasallos, y solo si están dentro: neutrales y enemigos nunca, aunque la puerta les haya abierto (Entrar con todo).
        let cupo = tope.saturating_sub(residentes_que_entran.len());
        let visitantes = self.defensores_de_dentro(dentro, a.faccion, Some(plaza as u32), cupo, t);
        escuadras.extend(visitantes.tropa.escuadras);
        lucharon.extend(visitantes.lucharon);
        DefensoresDeDentro { tropa: Tropa { escuadras, heroes: [residentes_que_entran, visitantes.tropa.heroes].concat() }, lucharon }
    }

    /// Los héroes que están dentro de `dentro` (una plaza, un pueblo o una avanzada) y lo defienden con números: sanos y conectados, de la Facción `faccion` del lugar, aliados o
    /// vasallos —neutrales y enemigos nunca, aunque la puerta les haya abierto—, hasta `cupo` y por nivel y poder, con las escuadras de la columna con la que entraron (no está
    /// en el mapa: es lo que trajeron y cuenta como dentro). `residentes_de` son los de esa plaza, que la defensa de la plaza cuenta aparte.
    pub(super) fn defensores_de_dentro(&self, dentro: Ubicacion, faccion: u32, residentes_de: Option<u32>, cupo: usize, t: Instante) -> DefensoresDeDentro {
        let cr = &self.estado.reglas.economia.combate;
        let candidatos: Vec<Candidato> = if cupo == 0 {
            vec![]
        } else {
            let propio = |h: &HeroeNucleo| residentes_de.is_some_and(|p| h.heroe.residencia == Some(militar::heroe::Residencia::Asentamiento(p)));
            let sano_y_conectado = |h: &HeroeNucleo| !h.heroe.esta_herido(t) && !h.heroe.presencia.fuera;
            self.heroes().filter(|h| !propio(h) && h.heroe.presencia.ubicacion == dentro && sano_y_conectado(h) && self.estado.militar.defiende_a(faccion, h.faccion)).map(|h| Candidato::de(cr, &h.heroe, &de_su_columna(h))).collect()
        };
        let (mut escuadras, mut lucharon) = (vec![], vec![]);
        let heroes = seleccionar(&candidatos, cupo).entran;
        for &h in &heroes {
            let suyas = de_su_columna(self.heroe(h).expect("existe"));
            lucharon.push((h, suyas.iter().map(|s| s.id).collect()));
            escuadras.extend(suyas.into_iter().cloned());
        }
        DefensoresDeDentro { tropa: Tropa { escuadras, heroes }, lucharon }
    }

    /// Combate §6. Solo un ejército; la plaza, de otra Facción, a ≤ `radioEncuentro` y no protegida (recién fundada o conquistada). Abre siempre la batalla del asedio, con su
    /// convocatoria de 30 s (Batalla_Unity_Ciclo §4): con servidores de batalla la juega Unity; sin ellos, al acabar la convocatoria se resuelve con números con todas las columnas
    /// que se unieron ([`Nucleo::resolver_asedio_con_numeros`]; calibración del sistema 4, M8).
    fn atacar_plaza(&mut self, t: Instante, heroe: u64, id: u64, x: u64, declara_guerra: bool) -> Result<(), Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: &str| Respuesta::rechazo(c, m, v);
        let plaza = self.plaza_o_rechazo(x)?;
        let e = &self.estado.ejercitos[&id];
        if !Self::es_ejercito(e) {
            return Err(rechazo(rechazos::COMBATE_SOLO_UN_EJERCITO, "solo un ejército abre un asedio: una columna personal no asedia"));
        }
        let Some(yo) = e.columna.faccion else {
            return Err(rechazo(rechazos::COMBATE_SIN_FACCION, "una columna sin Facción no conquista: no tendría a quién dar la plaza"));
        };
        if plaza.faccion == yo {
            return Err(rechazo(rechazos::COMBATE_PLAZA_PROPIA, "no se asedia una plaza de tu Facción"));
        }
        let (faccion_vieja, posicion) = (plaza.faccion, plaza.posicion);
        let guerra = self.comprobar_agresion(e, t, posicion, Some(faccion_vieja), declara_guerra)?;
        if esta_protegida(plaza.protegida_hasta, t) {
            return Err(rechazo(rechazos::COMBATE_PLAZA_PROTEGIDA, "esa plaza está protegida (recién fundada o conquistada): todavía no se la puede asediar"));
        }
        if self.bloqueos().asentamientos.contains(&(x as u32)) {
            return Err(rechazo(rechazos::BATALLA_BLOQUEADO, "esa plaza ya está en una batalla"));
        }
        let cr = self.estado.reglas.economia.combate.clone();
        let atac = self.tropa_de_columna(id, t, cr.batalla.capacidad_asedio);
        if !atac.escuadras.iter().any(Escuadra::con_soldados) {
            return Err(rechazo(rechazos::COMBATE_SIN_ATACANTES, "sin soldados no hay con quién combatir: un héroe solo no abre un asedio"));
        }
        let ap = self.apertura_de_asedio(t, heroe, id, x);
        self.abrir_batalla_de_ataque(t, ap, guerra);
        Ok(())
    }

    /// El asedio con números (sin servidores de batalla), al acabar su convocatoria: atacan juntas las columnas que se unieron (`atacantes`, la primera la que lo abrió), hasta
    /// `BATALLA.capacidad.asedio` héroes por nivel y poder, y defienden la guarnición y los amigos de dentro (Combate §3.2). Sin defensores con soldados cae sin combate y sin
    /// tirada. Si cae: heridos sus defensores en persona, la chatarra a los carros atacantes, [`Nucleo::conquistar_plaza`]. Si resiste: heridos todos los participantes de las
    /// columnas atacantes y la chatarra al almacén de la plaza. Devuelve si cayó.
    pub(super) fn resolver_asedio_con_numeros(&mut self, t: Instante, x: u64, atacantes: &[u64]) -> bool {
        let (Some(&id), Some(plaza)) = (atacantes.first(), self.estado.asentamientos.get(&x)) else { return false };
        let Some(yo) = self.estado.ejercitos[&id].columna.faccion.filter(|&f| f != plaza.faccion) else { return false };
        let faccion_vieja = plaza.faccion;
        let cr = self.estado.reglas.economia.combate.clone();
        let mut atac = self.tropa_de_columnas(atacantes, t, cr.batalla.capacidad_asedio);
        let mut defensa = self.defensa_de_plaza(x, t);
        let columnas = [self.columnas_de(&atac.escuadras), self.columnas_de(&defensa.tropa.escuadras)].concat();
        let participantes: Vec<u64> = atacantes.iter().flat_map(|c| Self::participantes_de(&self.estado.ejercitos[c])).collect();
        let punto = self.estado.ejercitos[&id].posicion_en(t);
        // Solo el asedio a una plaza multiplica al defensor con su recinto de referencia (Murallas §9.1); el equipo de asedio no lo reduce (Q7). No consume tirada.
        let multiplicador = self.multiplicador_de_plaza(x);
        let muralla_completa = recintos::muralla_completa(&self.estado.asentamientos[&x].economia.recintos);
        let resultado = asedio::resolver_asedio(&cr, false, Bando { escuadras: &mut atac.escuadras, heroes: &atac.heroes }, Bando { escuadras: &mut defensa.tropa.escuadras, heroes: &defensa.tropa.heroes }, multiplicador, &mut self.estado.rng);
        self.volcar_escuadras(&atac.escuadras);
        self.volcar_escuadras(&defensa.tropa.escuadras);
        let conquistada = resultado.conquistada();
        let heridos = self.herir_heroes(&asedio::heridos_del_asedio(&resultado, &participantes), t);

        let mut datos = vec![("faccionAtacante", yo as i64), ("faccionDefensora", faccion_vieja as i64), ("multiplicadorMuralla", multiplicador), ("murallaCompleta", muralla_completa as i64)];
        let mut chatarra = Mercancia::new();
        if let Asedio::Combate(i) = &resultado {
            chatarra = i.chatarra.clone();
            datos.extend([("poderAtacante", i.atacante.poder), ("poderDefensor", i.defensor.poder), ("bajasAtacante", caidos(&i.atacante.bajas)), ("bajasDefensor", caidos(&i.defensor.bajas))]);
        }
        let llevada = if conquistada {
            self.repartir_en_columnas(atacantes, &chatarra)
        } else {
            let a = &mut self.estado.asentamientos.get_mut(&x).expect("existe").economia.almacen;
            chatarra.iter().for_each(|(&r, &n)| {
                a.agregar(r, n);
            });
            self.tr.tocar(Clase::Asentamiento, x);
            Mercancia::new()
        };
        self.recomponer_tras_bajas(&columnas, t);
        datos.extend([("heridos", heridos as i64), ("chatarra", total(&chatarra)), ("botin", total(&llevada))]);
        datos.push(("columnas", atacantes.len() as i64));
        self.hecho_de_combate(if conquistada { "combate.asedio_conquista" } else { "combate.asedio_resistido" }, t, atacantes.to_vec(), if conquistada { vec![] } else { vec![x] }, punto, datos);
        if conquistada {
            // Los residentes que defendieron pierden la mitad del carro: es del vencedor, repartida entre sus columnas hasta donde quepa.
            let robado = self.conquistar_plaza(t, x, yo, id, &defensa.lucharon);
            self.repartir_en_columnas(atacantes, &robado);
        }
        conquistada
    }

    // ───────────────────────────── Un pueblo extractor ─────────────────────────────

    /// Combate §7.3. Solo un ejército, de otra Facción que la de la plaza madre. Defienden los 30 mineros de serie (sin dueño, fuera de cupo) y los héroes que están
    /// **dentro** del pueblo —de la Facción de la plaza madre, aliados o vasallos, hasta 5, con las escuadras de su columna—, con la ventaja del aguante. Si gana el atacante
    /// se lleva, hasta donde le quepa, lo que el almacén del pueblo guarda por encima de la mitad de su capacidad; si no, nada; los defensores en persona quedan heridos y los
    /// que residen en la plaza madre pierden la mitad de su carro (Entrar con todo, 2026-10-22). Es un saqueo: el pueblo no cambia de dueño ni se destruye. La guarnición que
    /// queda por debajo de su número de serie se repone gratis, un minero cada 12 minutos desde la primera baja ([`Nucleo::guardar_guarnicion`]); con la guarnición a 0 el
    /// pueblo cae sin combate (como una plaza sin defensores). Solo defiende lo que está dentro (decisión del autor, 2026-10-22): una columna amiga a su puerta, en el mapa,
    /// no cuenta. La chatarra, si resiste, va al almacén de la plaza madre (Combate §7.3).
    fn atacar_pueblo(&mut self, t: Instante, heroe: u64, id: u64, x: u64, declara_guerra: bool) -> Result<(), Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: &str| Respuesta::rechazo(c, m, v);
        let (madre, pueblo) = self.pueblo(x).ok_or_else(|| Respuesta::rechazo(rechazos::PUEBLO_DESCONOCIDO, ids::pueblo(x), v))?;
        let e = &self.estado.ejercitos[&id];
        if !Self::es_ejercito(e) {
            return Err(rechazo(rechazos::COMBATE_SOLO_UN_EJERCITO, "solo un ejército saquea un pueblo"));
        }
        if !pueblo.activo() {
            return Err(rechazo(rechazos::COMBATE_PUEBLO_INACTIVO, "el pueblo aún es una obra: no tiene almacén ni guarnición"));
        }
        if self.bloqueos().pueblos.contains(&x) {
            return Err(rechazo(rechazos::BATALLA_BLOQUEADO, "ese pueblo ya está en una batalla"));
        }
        let donde = self.posicion_de_pueblo(x).ok_or_else(|| rechazo(rechazos::SIN_MUNDO, "el núcleo no tiene mundo cargado"))?;
        let faccion_madre = self.estado.asentamientos[&madre].faccion;
        let guerra = self.comprobar_agresion(e, t, donde, Some(faccion_madre), declara_guerra)?;
        let cr = self.estado.reglas.economia.combate.clone();
        let mut atac = self.tropa_de_columna(id, t, cr.batalla.capacidad_resto);
        if !atac.escuadras.iter().any(Escuadra::con_soldados) {
            return Err(rechazo(rechazos::COMBATE_SIN_ATACANTES, "sin soldados no hay con quién combatir: un héroe solo no saquea un pueblo"));
        }

        // La guarnición de serie nace completa la primera vez que se la necesita.
        if !self.estado.combate.guarniciones.contains_key(&x) {
            let sid = self.nuevo_id();
            let escuadra = reglas_pueblo::guarnicion_de_serie(&cr, sid, t);
            self.estado.combate.guarniciones.insert(x, crate::estado_combate::GuarnicionDePueblo { escuadra, repone_en: None });
            self.tr.tocar(Clase::Pueblo, x);
        }
        if self.estado.reglas.batallas_en_unity {
            let ap = self.apertura_de_pueblo(t, heroe, id, x, madre, donde);
            self.abrir_batalla_de_ataque(t, ap, guerra);
            return Ok(());
        }
        let dentro = self.defensores_de_dentro(Ubicacion::Pueblo(x), faccion_madre, None, cr.batalla.capacidad_resto, t);
        let mut defensa = vec![self.estado.combate.guarniciones[&x].escuadra.clone()];
        defensa.extend(dentro.tropa.escuadras.iter().cloned());
        let columnas = [self.columnas_de(&atac.escuadras), self.columnas_de(&dentro.tropa.escuadras)].concat();
        let participantes = Self::participantes_de(&self.estado.ejercitos[&id]);
        let punto = self.estado.ejercitos[&id].posicion_en(t);
        if let Some((a, b)) = guerra {
            self.declarar_guerra(t, a, b);
        }
        let (gana, informe): (bool, Option<Informe>) = if defensa.iter().any(Escuadra::con_soldados) {
            let i = reglas_pueblo::resolver_saqueo(&cr, Bando { escuadras: &mut atac.escuadras, heroes: &atac.heroes }, Bando { escuadras: &mut defensa, heroes: &dentro.tropa.heroes }, &mut self.estado.rng)
                .expect("los dos bandos tienen quién combata");
            (i.ganador == Lado::Atacante, Some(i))
        } else {
            (true, None)
        };
        self.volcar_escuadras(&atac.escuadras);
        self.volcar_escuadras(&defensa);
        self.guardar_guarnicion(x, defensa.remove(0), t);

        let (mut llevado, mut chatarra) = (Mercancia::new(), Mercancia::new());
        if let Some(i) = &informe {
            chatarra = i.chatarra.clone();
        }
        if gana {
            let capacidad = self.capacidad_de_carga(id).unwrap_or(0);
            let carga = self.estado.ejercitos[&id].columna.carga.clone();
            let mut carros = [CarroDeColumna { carga, capacidad }];
            let almacen = &mut self.estado.asentamientos.get_mut(&madre).expect("existe").economia.pueblos.iter_mut().find(|p| p.id == x).expect("existe").almacen;
            llevado = reglas_pueblo::saquear(&cr, almacen, &mut carros);
            let [CarroDeColumna { carga, .. }] = carros;
            self.estado.ejercitos.get_mut(&id).expect("existe").columna.carga = carga;
            self.tr.tocar(Clase::Ejercito, id);
            let chatarra_cogida = self.cargar_botin_en(id, &chatarra);
            chatarra = chatarra_cogida;
        } else {
            let a = &mut self.estado.asentamientos.get_mut(&madre).expect("existe").economia.almacen;
            chatarra.iter().for_each(|(&r, &n)| {
                a.agregar(r, n);
            });
        }
        self.tr.tocar(Clase::Asentamiento, madre);
        // Los defensores en persona quedan heridos si el pueblo cae en el saqueo (sin combate, nadie); si no, todo el ejército. El residente pierde la mitad del carro.
        let (heridos, carro_perdido) = match (gana, &informe) {
            (false, _) => (self.herir_heroes(&participantes, t), Mercancia::new()),
            (true, Some(_)) => (self.herir_heroes(&dentro.tropa.heroes, t), self.pierden_la_mitad_del_carro(&dentro.tropa.heroes)),
            (true, None) => (0, Mercancia::new()),
        };
        self.entregar_lo_robado(id, &carro_perdido);
        self.recomponer_tras_bajas(&columnas, t);
        let mut datos = vec![("pueblo", x as i64), ("saqueado", total(&llevado)), ("chatarra", total(&chatarra)), ("heridos", heridos as i64), ("carroPerdido", total(&carro_perdido))];
        if let Some(i) = &informe {
            datos.extend([("poderAtacante", i.atacante.poder), ("poderDefensor", i.defensor.poder), ("bajasAtacante", caidos(&i.atacante.bajas)), ("bajasDefensor", caidos(&i.defensor.bajas))]);
        }
        self.hecho_de_combate(if gana { "combate.pueblo_saqueado" } else { "combate.pueblo_resiste" }, t, vec![id], vec![madre], punto, datos);
        Ok(())
    }

    /// Guarda la guarnición de serie de un pueblo tras un combate y, si quedó incompleta y no se estaba reponiendo, programa la vuelta del primer minero (Combate
    /// §7.3: uno cada 12 minutos desde la primera baja; un combate durante la reposición no la reinicia).
    pub(super) fn guardar_guarnicion(&mut self, pueblo: u64, escuadra: Escuadra, t: Instante) {
        let cr = self.estado.reglas.economia.combate.clone();
        let incompleta = escuadra.cantidad < cr.pueblos.mineros;
        let g = self.estado.combate.guarniciones.get_mut(&pueblo).expect("existe");
        self.tr.tocar(Clase::Pueblo, pueblo);
        g.escuadra = escuadra;
        if incompleta && g.repone_en.is_none() {
            self.programar_minero(pueblo, reglas_pueblo::reposicion_en(&cr, t));
        }
    }

    /// La cita de la vuelta del siguiente minero de la guarnición de serie de un pueblo.
    fn programar_minero(&mut self, pueblo: u64, en: Instante) {
        self.estado.combate.guarniciones.get_mut(&pueblo).expect("existe").repone_en = Some(en);
        let clave = self.estado.generaciones.clave(entidad_guarnicion(pueblo));
        self.estado.cola.programar(en, PRIO_UMBRAL, clave, Carga::Combate(SucesoCombate::ReponeGuarnicion));
    }

    /// Vuelve un minero a la guarnición de serie de un pueblo, con su nivel y su experiencia; si aún falta alguno, se cita el siguiente. Completa, lo dice.
    pub(super) fn repone_guarnicion(&mut self, entidad: u64, t: Instante) {
        let pueblo = crate::estado_comercio::id_de_entidad(entidad);
        let cr = self.estado.reglas.economia.combate.clone();
        let Some(g) = self.estado.combate.guarniciones.get_mut(&pueblo) else { return };
        self.tr.tocar(Clase::Pueblo, pueblo);
        g.repone_en = None;
        if !reglas_pueblo::reponer(&cr, &mut g.escuadra) {
            return self.programar_minero(pueblo, reglas_pueblo::reposicion_en(&cr, t));
        }
        if let Some((madre, _)) = self.pueblo(pueblo) {
            self.tr.tocar(Clase::Asentamiento, madre);
            self.emitir_propio(t, "pueblo.guarnicion_repuesta", vec![madre], vec![("pueblo", pueblo as i64)]);
        }
    }

    // ───────────────────────────── La orden de administración ─────────────────────────────

    /// `darEscuadra` (escenarios y pruebas): una escuadra nueva para el héroe, en su columna si va a bordo y, si no, en su campamento. Sin coste ni Liderazgo.
    pub(super) fn dar_escuadra(&mut self, t: Instante, heroe: u64, tropa: militar::catalogo::Tropa, cantidad: i64) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let hn = self.heroe(heroe).ok_or_else(|| Respuesta::rechazo(rechazos::HEROE_DESCONOCIDO, ids::heroe(heroe), v))?;
        let mr = self.estado.reglas.economia.militar.clone();
        if cantidad < 1 || cantidad > mr.tropas.tropa(tropa).unidades {
            return Err(Respuesta::rechazo(rechazos::ESCUADRA_CANTIDAD_INVALIDA, format!("una escuadra de esa tropa lleva de 1 a {} soldados", mr.tropas.tropa(tropa).unidades), v));
        }
        let a_bordo = match hn.heroe.presencia.ubicacion {
            Ubicacion::Columna(c) if self.estado.ejercitos.contains_key(&c) => Some(c),
            _ => None,
        };
        let antes = a_bordo.and_then(|c| self.velocidad_de_ejercito(c)).unwrap_or(0);
        let id = self.nuevo_id();
        let mut escuadra = Escuadra::nueva(&mr, id, heroe, tropa, cantidad, None, t);
        if let Some(c) = a_bordo {
            escuadra.contenedor = Contenedor::Ejercito(c);
            self.estado.ejercitos.get_mut(&c).expect("existe").columna.escuadras.push(id);
        }
        self.heroe_mut(heroe).heroe.escuadras.push(escuadra);
        if let Some(c) = a_bordo {
            self.recomponer_columna(c, t, antes);
        }
        Ok(vec![id])
    }
}
