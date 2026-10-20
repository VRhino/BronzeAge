//! Atacar (Combate §2-§7, fase 4D): `atacar` con sus seis objetivos y la resolución con números —bot contra bot, sin batalla de Unity, que es de la fase
//! 4E— con todas sus consecuencias: bajas permanentes y experiencia de escuadra, chatarra, heridos, la mitad del carro que pierde el derrotado y el botín
//! que se lleva el vencedor. Las reglas puras son el crate `combate`; aquí se mira el estado, se comprueba **todo** antes de tocar nada o de gastar una
//! tirada (una orden rechazada no deja rastro) y se vuelcan los resultados.
//!
//! # Qué se ataca
//! - **Una columna** (`atacar_columna`): de su misma clase, enemiga, en el mapa y con algún héroe sano. Gana quien dice la resolución por poder (dos tiradas).
//! - **Una caravana** (`atacar_caravana`): la que viaja suelta (no la adjunta a un ejército: se ataca al ejército). El servidor decide el tipo por la zona en
//!   que está **en el instante del ataque** ([`Nucleo::en_zona`]): aguante (B) en la zona del dueño o de un aliado, disputa de carros (A) en el resto. Una tirada.
//! - **Un campamento de bandidos** (`atacar_campamento`): PvE, sin guerra. Una tirada contra el poder de su nivel.
//! - **Una plaza** (`atacar_plaza`): solo un ejército. Se asedia con la guarnición y los defensores en persona; si cae, [`conquistas`](super::conquistas).
//! - **Un pueblo extractor** (`atacar_pueblo`): un saqueo, nunca una conquista. Defienden los 30 mineros de serie.
//! - **Una avanzada**: todavía no existen (fase 4F); la orden se rechaza.
//!
//! # Lo que se mira antes (en este orden)
//! El héroe (sano, a bordo, columna fuera de batalla), el objetivo (existe, está en el mapa), el alcance (`radioEncuentro`), la protección de los campamentos
//! de mercenarios (`radioProteccion`: nadie inicia un combate a esa distancia, ni desde allí ni contra lo que esté allí) y la guerra. Entre Facciones
//! distintas hace falta guerra; `declara_guerra` la declara si falta, pero solo cuando todo lo demás ya vale (las relaciones son un andamiaje de
//! administración hasta el sistema político, sin arrastre). Una columna sin Facción ni exige ni declara guerra: no hay a quién.

use super::persecuciones::Fin;
use super::*;
use crate::comando::ObjetivoAtaque;
use crate::estado::{SucesoCombate, PRIO_UMBRAL};
use crate::estado_combate::entidad_guarnicion;
use crate::estado_militar::Relacion;
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

/// Quién defiende una plaza con números (Combate §3.2).
pub(super) struct DefensaDePlaza {
    pub tropa: Tropa,
    /// Cada héroe que defiende en persona con las escuadras con las que lucha: si la plaza cae, salen con él.
    pub lucharon: Vec<(u64, Vec<u64>)>,
}

/// Lo que cambia con unas bajas: las columnas cuyas escuadras combatieron y la velocidad que llevaban (para recomponer su consumo y su marcha).
type Columnas = Vec<(u64, Mil)>;

/// Las escuadras de su loadout activo que el héroe tiene en el campamento, con soldados y fuera de la guarnición: las que defienden en persona.
fn loadout_en_el_campamento(h: &HeroeNucleo) -> Vec<&Escuadra> {
    let activas = h.heroe.escuadras_del_loadout_activo();
    h.heroe.escuadras.iter().filter(|s| activas.contains(&s.id) && s.contenedor == Contenedor::Campamento && !s.en_guarnicion && s.con_soldados()).collect()
}

/// Las escuadras con soldados del héroe que van en una columna (la que dejó aparcada a la puerta, si es un visitante): con las que defiende de visita.
fn de_su_columna(h: &HeroeNucleo) -> Vec<&Escuadra> {
    h.heroe.escuadras.iter().filter(|s| matches!(s.contenedor, Contenedor::Ejercito(_)) && s.con_soldados()).collect()
}

fn caidos(bajas: &[Baja]) -> i64 {
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
        let e = &self.estado.ejercitos[&id];
        let cr = &self.estado.reglas.economia.combate;
        let candidatos: Vec<Candidato> = e
            .columna
            .participantes
            .iter()
            .filter_map(|p| self.heroe(p.heroe))
            .filter(|h| !h.heroe.esta_herido(t))
            .map(|h| {
                let suyas: Vec<&Escuadra> = h.heroe.escuadras.iter().filter(|s| s.contenedor == Contenedor::Ejercito(id)).collect();
                Candidato::de(cr, &h.heroe, &suyas)
            })
            .collect();
        let entran = seleccionar(&candidatos, tope).entran;
        let escuadras = e.columna.escuadras.iter().filter_map(|&s| self.escuadra(s).map(|(_, esc)| esc)).filter(|s| entran.contains(&s.heroe)).cloned().collect();
        Tropa { escuadras, heroes: entran }
    }

    /// Las columnas de las escuadras dadas y la velocidad que llevan ahora.
    fn columnas_de(&self, escuadras: &[Escuadra]) -> Columnas {
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
            }
        }
    }

    /// Tras unas bajas, cada columna vuelve a anotar lo que come y a qué velocidad marcha.
    fn recomponer_tras_bajas(&mut self, columnas: &Columnas, t: Instante) {
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
        self.tr.tocados.push(columna);
        cogido
    }

    /// El derrotado pierde la mitad de su carro (`trasDerrota`): devuelve lo robado. Una columna personal que pierde suelta además su presa.
    fn perder_la_mitad_del_carro(&mut self, columna: u64) -> Mercancia {
        let cr = self.estado.reglas.economia.combate.clone();
        let Some(e) = self.estado.ejercitos.get_mut(&columna) else { return Mercancia::new() };
        let robado = tras_derrota(&cr, &mut e.columna.carga);
        let personal = e.columna.tipo == militar::columna::TipoColumna::Personal;
        self.tr.tocados.push(columna);
        if personal {
            self.fijar_persecucion(columna, None);
        }
        robado
    }

    fn hecho_de_combate(&mut self, codigo: &'static str, t: Instante, columnas: Vec<u64>, plazas: Vec<u64>, punto: Punto, datos: Vec<(&'static str, i64)>) {
        self.tr.eventos.push(EventoNucleo { codigo: Cow::Borrowed(codigo), en: t, ejercitos: columnas, asentamientos: plazas, punto: Some(punto), datos });
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
            ObjetivoAtaque::Ejercito(x) => self.atacar_columna(t, id, x, declara_guerra)?,
            ObjetivoAtaque::Caravana(x) => self.atacar_caravana(t, id, x, declara_guerra)?,
            ObjetivoAtaque::Campamento(x) => self.atacar_campamento(t, id, x)?,
            ObjetivoAtaque::Asentamiento(x) => self.atacar_plaza(t, id, x, declara_guerra)?,
            ObjetivoAtaque::Pueblo(x) => self.atacar_pueblo(t, id, x, declara_guerra)?,
            ObjetivoAtaque::Avanzada(_) => return Err(rechazo(rechazos::COMBATE_AVANZADA_NO_EXISTE, "las avanzadas son de la fase 4F: todavía no existe ninguna")),
        }
        Ok(vec![])
    }

    /// Lo que toda agresión comprueba: el alcance (`radioEncuentro`), la protección de los campamentos y la guerra. Devuelve la guerra que hay que declarar
    /// (entre las dos Facciones) si falta una y la orden la pide; no declara nada: eso se hace cuando todo lo demás ya vale.
    fn comprobar_agresion(&self, e: &Ejercito, t: Instante, donde: Punto, contra: Option<u32>, declara_guerra: bool) -> Result<Option<(u32, u32)>, Respuesta> {
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
            _ => Err(rechazo(rechazos::COMBATE_SIN_GUERRA, format!("la Facción {yo} no está en guerra con la {otra}: atacar con `declara_guerra` la declara (y la deja en guerra con ella, sin arrastre de aliados ni vasallos)"))),
        }
    }

    /// Declara la guerra entre dos Facciones (`declara_guerra` de `atacar`): la relación pasa a ser de guerra y se cuenta.
    fn declarar_guerra(&mut self, t: Instante, atacante: u32, contra: u32) {
        self.estado.militar.relaciones.insert((atacante.min(contra), atacante.max(contra)), Relacion::Guerra);
        self.emitir_propio(t, "faccion.guerra_declarada", vec![], vec![("atacante", atacante as i64), ("contra", contra as i64)]);
    }

    // ───────────────────────────── Columna contra columna ─────────────────────────────

    /// Combate §5. Las dos columnas entran con sus héroes sanos (como mucho 5 por bando) y gana quien dice el poder; el perdedor pierde la mitad de su carro y
    /// todos sus héroes quedan heridos; el ganador carga lo robado y la chatarra hasta donde le quepa.
    fn atacar_columna(&mut self, t: Instante, id: u64, x: u64, declara_guerra: bool) -> Result<(), Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: &str| Respuesta::rechazo(c, m, v);
        let e = &self.estado.ejercitos[&id];
        let o = self.estado.ejercitos.get(&x).ok_or_else(|| Respuesta::rechazo(rechazos::EJERCITO_DESCONOCIDO, format!("ej-{x}"), v))?;
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

        let cr = self.estado.reglas.economia.combate.clone();
        let (mut atac, mut def) = (self.tropa_de_columna(id, t, cr.batalla.capacidad_resto), self.tropa_de_columna(x, t, cr.batalla.capacidad_resto));
        let (columnas, punto) = ([self.columnas_de(&atac.escuadras), self.columnas_de(&def.escuadras)].concat(), self.estado.ejercitos[&id].posicion_en(t));
        let (participantes_a, participantes_d) = (Self::participantes_de(&self.estado.ejercitos[&id]), Self::participantes_de(&self.estado.ejercitos[&x]));
        if let Some((a, b)) = guerra {
            self.declarar_guerra(t, a, b);
        }
        let informe = resolver_combate(&cr, Bando { escuadras: &mut atac.escuadras, heroes: &atac.heroes }, Bando { escuadras: &mut def.escuadras, heroes: &def.heroes }, asedio::SIN_MURALLA, &mut self.estado.rng)
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
        Ok(())
    }

    // ───────────────────────────── Interceptar una caravana ─────────────────────────────

    /// Combate §7.1. Ver el módulo: tipo A o B por la zona, una tirada contra la defensa. Gana el atacante: el dueño pierde la mitad de los carros (y en B el
    /// 70 % de la carga), el atacante carga lo perdido hasta donde le quepa y la escolta cedida queda a 0 y vuelve al campamento de su héroe. Gana la caravana:
    /// el atacante pierde el 25 %, la escolta el 5 % y los héroes de la columna quedan heridos.
    fn atacar_caravana(&mut self, t: Instante, id: u64, x: u64, declara_guerra: bool) -> Result<(), Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: &str| Respuesta::rechazo(c, m, v);
        let e = &self.estado.ejercitos[&id];
        let k = self.estado.comercio.caravana(x).ok_or_else(|| Respuesta::rechazo(rechazos::CARAVANA_DESCONOCIDA, format!("caravana-{x}"), v))?;
        match k.estado {
            EstadoCaravana::Adjunta => return Err(rechazo(rechazos::COMBATE_CARAVANA_ADJUNTA, "esa caravana va adjunta a un ejército: se ataca al ejército")),
            EstadoCaravana::EnTransito | EstadoCaravana::Retornando => {}
            _ => return Err(rechazo(rechazos::COMBATE_A_CUBIERTO, "esa caravana no está en el mapa")),
        }
        let (origen, dueno) = (k.origen as u64, self.estado.asentamientos.get(&(k.origen as u64)).map(|a| a.faccion));
        let donde = self.posicion_de_caravana(x, t).unwrap_or(k.posicion);
        let guerra = self.comprobar_agresion(e, t, donde, dueno, declara_guerra)?;

        let modalidad = asalto::modalidad_de(self.zona_en(donde, dueno));
        let cr = self.estado.reglas.economia.combate.clone();
        let flota = self.estado.reglas.economia.comercio.flota.clone();
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

        let mut perdida = None;
        if r.gana_atacante {
            // La caravana cae: sus carros, su carga y su escolta (a 0 y de vuelta al campamento de su héroe).
            let i = self.estado.comercio.posicion_caravana(x).expect("existe");
            let c = &mut self.estado.comercio.flota[i];
            let p = asalto::aplicar_al_dueno(&cr, &flota, c, modalidad, None).expect("los carros que se pierden son los últimos de la lista");
            let cedidas = c.liberar_escolta();
            self.devolver_escolta(&cedidas);
            perdida = Some(p);
        }
        let (carros, cogido, destruida) = match &perdida {
            Some(p) => (p.carros_perdidos as i64, self.cargar_botin_en(id, &p.carga_perdida), p.destruida),
            None => (0, Mercancia::new(), false),
        };
        if let Some(p) = &perdida {
            if destruida {
                // Un aguante que se queda sin carros salva igual el 30 %: llega a casa con los carreteros.
                // DUDA: Combate §7.1 no dice a dónde; se entrega al almacén de la plaza de origen, hasta donde quepa.
                if let Some(a) = self.estado.asentamientos.get_mut(&origen) {
                    p.carga_salvada.iter().for_each(|(&r, &n)| {
                        a.economia.almacen.agregar(r, n);
                    });
                }
                self.perder_caravana(t, x);
            } else {
                self.tr.asentamientos.push(origen);
            }
        }
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

    // ───────────────────────────── Un campamento de bandidos ─────────────────────────────

    /// Combate §7.2. No exige guerra. Una tirada contra el poder de su nivel: si gana, el campamento cae (oro por cada héroe que combatió, a su carro) y se
    /// agenda su reaparición; si no, todos los héroes de la columna quedan heridos y la columna pierde la mitad del carro (nadie se la queda).
    fn atacar_campamento(&mut self, t: Instante, id: u64, x: u64) -> Result<(), Respuesta> {
        let v = self.estado.version;
        let b = self.estado.combate.bandidos.get(&x).ok_or_else(|| Respuesta::rechazo(rechazos::CAMPAMENTO_DE_BANDIDOS_DESCONOCIDO, format!("bandidos-{x}"), v))?;
        let (nivel, donde) = (b.nivel, b.posicion);
        self.comprobar_agresion(&self.estado.ejercitos[&id], t, donde, None, false)?;

        let cr = self.estado.reglas.economia.combate.clone();
        let mut atac = self.tropa_de_columna(id, t, cr.batalla.capacidad_resto);
        let columnas = self.columnas_de(&atac.escuadras);
        let participantes = Self::participantes_de(&self.estado.ejercitos[&id]);
        let punto = self.estado.ejercitos[&id].posicion_en(t);
        let r = combate::bandidos::resolver_campamento(&cr, Bando { escuadras: &mut atac.escuadras, heroes: &atac.heroes }, nivel, &mut self.estado.rng).expect("la columna lleva al menos un héroe sano");
        self.volcar_escuadras(&atac.escuadras);

        let (mut oro, mut heridos, mut perdido) = (Mercancia::new(), 0, Mercancia::new());
        if r.destruido {
            let ventana = self.estado.combate.destruidos.clone();
            for &h in &atac.heroes {
                let mut recientes = ventana.get(&h).cloned().unwrap_or_default();
                let ganado = combate::bandidos::botin_de_bandidos(&cr, &mut recientes, nivel, t);
                self.estado.combate.destruidos.insert(h, recientes);
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
    /// una aliada, con las escuadras de la columna que dejaron aparcada a la puerta.
    pub(super) fn defensa_de_plaza(&self, plaza: u64, t: Instante) -> DefensaDePlaza {
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

        let amiga = |f: Option<u32>| f.is_some_and(|f| f == a.faccion || matches!(self.estado.militar.relacion(f, a.faccion), Some(Relacion::Alianza | Relacion::Vasallaje)));
        let cupo = tope.saturating_sub(residentes_que_entran.len());
        let visitantes: Vec<Candidato> = if cupo == 0 {
            vec![]
        } else {
            self.heroes().filter(|h| h.heroe.residencia != Some(militar::heroe::Residencia::Asentamiento(plaza as u32)) && h.heroe.presencia.ubicacion == dentro && sana_y_conectada(h) && amiga(h.faccion)).map(|h| Candidato::de(cr, &h.heroe, &de_su_columna(h))).collect()
        };
        let visitantes_que_entran = seleccionar(&visitantes, cupo).entran;
        for &h in &visitantes_que_entran {
            let suyas = de_su_columna(self.heroe(h).expect("existe"));
            lucharon.push((h, suyas.iter().map(|s| s.id).collect()));
            escuadras.extend(suyas.into_iter().cloned());
        }
        DefensaDePlaza { tropa: Tropa { escuadras, heroes: [residentes_que_entran, visitantes_que_entran].concat() }, lucharon }
    }

    /// Combate §6. Solo un ejército; la plaza, de otra Facción, a ≤ `radioEncuentro` y no protegida por una conquista reciente. Sin defensores con soldados
    /// cae sin combate y sin tirada. Si cae: heridos sus defensores en persona, chatarra al carro, [`Nucleo::conquistar_plaza`]. Si resiste: heridos todos los
    /// participantes del ejército y la chatarra al almacén de la plaza.
    fn atacar_plaza(&mut self, t: Instante, id: u64, x: u64, declara_guerra: bool) -> Result<(), Respuesta> {
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
            return Err(rechazo(rechazos::COMBATE_PLAZA_PROTEGIDA, "esa plaza está protegida tras su conquista: todavía no se la puede asediar"));
        }
        let cr = self.estado.reglas.economia.combate.clone();
        let mut atac = self.tropa_de_columna(id, t, cr.batalla.capacidad_asedio);
        if !atac.escuadras.iter().any(Escuadra::con_soldados) {
            return Err(rechazo(rechazos::COMBATE_SIN_ATACANTES, "sin soldados no hay con quién combatir: un héroe solo no abre un asedio"));
        }

        let mut defensa = self.defensa_de_plaza(x, t);
        let columnas = [self.columnas_de(&atac.escuadras), self.columnas_de(&defensa.tropa.escuadras)].concat();
        let participantes = Self::participantes_de(&self.estado.ejercitos[&id]);
        let punto = self.estado.ejercitos[&id].posicion_en(t);
        if let Some((a, b)) = guerra {
            self.declarar_guerra(t, a, b);
        }
        let resultado = asedio::resolver_asedio(&cr, false, Bando { escuadras: &mut atac.escuadras, heroes: &atac.heroes }, Bando { escuadras: &mut defensa.tropa.escuadras, heroes: &defensa.tropa.heroes }, &mut self.estado.rng);
        self.volcar_escuadras(&atac.escuadras);
        self.volcar_escuadras(&defensa.tropa.escuadras);
        let conquistada = resultado.conquistada();
        let heridos = self.herir_heroes(&asedio::heridos_del_asedio(&resultado, &participantes), t);

        let mut datos = vec![("faccionAtacante", yo as i64), ("faccionDefensora", faccion_vieja as i64)];
        let mut chatarra = Mercancia::new();
        if let Asedio::Combate(i) = &resultado {
            chatarra = i.chatarra.clone();
            datos.extend([("poderAtacante", i.atacante.poder), ("poderDefensor", i.defensor.poder), ("bajasAtacante", caidos(&i.atacante.bajas)), ("bajasDefensor", caidos(&i.defensor.bajas))]);
        }
        let llevada = if conquistada {
            self.cargar_botin_en(id, &chatarra)
        } else {
            let a = &mut self.estado.asentamientos.get_mut(&x).expect("existe").economia.almacen;
            chatarra.iter().for_each(|(&r, &n)| {
                a.agregar(r, n);
            });
            self.tr.asentamientos.push(x);
            Mercancia::new()
        };
        self.recomponer_tras_bajas(&columnas, t);
        datos.extend([("heridos", heridos as i64), ("chatarra", total(&chatarra)), ("botin", total(&llevada))]);
        self.hecho_de_combate(if conquistada { "combate.asedio_conquista" } else { "combate.asedio_resistido" }, t, vec![id], if conquistada { vec![] } else { vec![x] }, punto, datos);
        if conquistada {
            self.conquistar_plaza(t, x, yo, id, &defensa.lucharon);
        }
        Ok(())
    }

    // ───────────────────────────── Un pueblo extractor ─────────────────────────────

    /// Combate §7.3. Solo un ejército, de otra Facción que la de la plaza madre. Defienden los 30 mineros de serie (sin dueño, fuera de cupo) con la ventaja del
    /// aguante. Si gana el atacante se lleva, hasta donde le quepa, lo que el almacén del pueblo guarda por encima de la mitad de su capacidad; si no, nada. Es un
    /// saqueo: el pueblo no cambia de dueño ni se destruye. La guarnición que queda por debajo de su número de serie se repone gratis a las 6 horas.
    // DUDA: «si cae, se repone» se lee aquí como «si queda por debajo de 30»: el reloj de 6 h empieza en el primer combate que la deja incompleta y no se
    // reinicia mientras corre. Con la guarnición a 0 esperando su reposición el pueblo cae sin combate (como una plaza sin defensores).
    // DUDA: quien «defienda el pueblo» además de los mineros no existe (nadie puede estar en un pueblo); la chatarra, si resiste, va al almacén de la plaza madre.
    fn atacar_pueblo(&mut self, t: Instante, id: u64, x: u64, declara_guerra: bool) -> Result<(), Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: &str| Respuesta::rechazo(c, m, v);
        let (madre, pueblo) = self.pueblo(x).ok_or_else(|| Respuesta::rechazo(rechazos::PUEBLO_DESCONOCIDO, format!("pueblo-{x}"), v))?;
        let e = &self.estado.ejercitos[&id];
        if !Self::es_ejercito(e) {
            return Err(rechazo(rechazos::COMBATE_SOLO_UN_EJERCITO, "solo un ejército saquea un pueblo"));
        }
        if !pueblo.activo() {
            return Err(rechazo(rechazos::COMBATE_PUEBLO_INACTIVO, "el pueblo aún es una obra: no tiene almacén ni guarnición"));
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
        }
        let mut mineros = vec![self.estado.combate.guarniciones[&x].escuadra.clone()];
        let columnas = self.columnas_de(&atac.escuadras);
        let participantes = Self::participantes_de(&self.estado.ejercitos[&id]);
        let punto = self.estado.ejercitos[&id].posicion_en(t);
        if let Some((a, b)) = guerra {
            self.declarar_guerra(t, a, b);
        }
        let (gana, informe): (bool, Option<Informe>) = if mineros[0].con_soldados() {
            let i = reglas_pueblo::resolver_saqueo(&cr, Bando { escuadras: &mut atac.escuadras, heroes: &atac.heroes }, Bando { escuadras: &mut mineros, heroes: &[] }, &mut self.estado.rng).expect("los dos bandos tienen quién combata");
            (i.ganador == Lado::Atacante, Some(i))
        } else {
            (true, None)
        };
        self.volcar_escuadras(&atac.escuadras);
        self.guardar_guarnicion(x, mineros.remove(0), t);

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
            self.tr.tocados.push(id);
            let chatarra_cogida = self.cargar_botin_en(id, &chatarra);
            chatarra = chatarra_cogida;
        } else {
            let a = &mut self.estado.asentamientos.get_mut(&madre).expect("existe").economia.almacen;
            chatarra.iter().for_each(|(&r, &n)| {
                a.agregar(r, n);
            });
        }
        self.tr.asentamientos.push(madre);
        let heridos = if gana { 0 } else { self.herir_heroes(&participantes, t) };
        self.recomponer_tras_bajas(&columnas, t);
        let mut datos = vec![("pueblo", x as i64), ("saqueado", total(&llevado)), ("chatarra", total(&chatarra)), ("heridos", heridos as i64)];
        if let Some(i) = &informe {
            datos.extend([("poderAtacante", i.atacante.poder), ("poderDefensor", i.defensor.poder), ("bajasAtacante", caidos(&i.atacante.bajas)), ("bajasDefensor", caidos(&i.defensor.bajas))]);
        }
        self.hecho_de_combate(if gana { "combate.pueblo_saqueado" } else { "combate.pueblo_resiste" }, t, vec![id], vec![madre], punto, datos);
        Ok(())
    }

    /// Guarda la guarnición de serie de un pueblo tras un combate y, si quedó incompleta y no tenía ya fecha, programa su reposición a las 6 horas.
    fn guardar_guarnicion(&mut self, pueblo: u64, escuadra: Escuadra, t: Instante) {
        let cr = self.estado.reglas.economia.combate.clone();
        let incompleta = escuadra.cantidad < cr.pueblos.mineros;
        let g = self.estado.combate.guarniciones.get_mut(&pueblo).expect("existe");
        g.escuadra = escuadra;
        if incompleta && g.repone_en.is_none() {
            let en = reglas_pueblo::reposicion_en(&cr, t);
            g.repone_en = Some(en);
            let clave = self.estado.generaciones.clave(entidad_guarnicion(pueblo));
            self.estado.cola.programar(en, PRIO_UMBRAL, clave, Carga::Combate(SucesoCombate::ReponeGuarnicion));
        }
    }

    /// Vence la reposición de la guarnición de serie de un pueblo: de nuevo completa, con su nivel y su experiencia.
    pub(super) fn repone_guarnicion(&mut self, entidad: u64, t: Instante) {
        let pueblo = crate::estado_comercio::id_de_entidad(entidad);
        let cr = self.estado.reglas.economia.combate.clone();
        let Some(g) = self.estado.combate.guarniciones.get_mut(&pueblo) else { return };
        reglas_pueblo::reponer(&cr, &mut g.escuadra);
        g.repone_en = None;
        if let Some((madre, _)) = self.pueblo(pueblo) {
            self.tr.asentamientos.push(madre);
            self.emitir_propio(t, "pueblo.guarnicion_repuesta", vec![madre], vec![("pueblo", pueblo as i64)]);
        }
    }

    // ───────────────────────────── La orden de administración ─────────────────────────────

    /// `darEscuadra` (escenarios y pruebas): una escuadra nueva para el héroe, en su columna si va a bordo y, si no, en su campamento. Sin coste ni Liderazgo.
    pub(super) fn dar_escuadra(&mut self, t: Instante, p: &Peticion, heroe: u64, tropa: militar::catalogo::Tropa, cantidad: i64) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        if !p.admin {
            return Err(Respuesta::rechazo(rechazos::NO_AUTORIZADO, "solo administración da escuadras", v));
        }
        let hn = self.heroe(heroe).ok_or_else(|| Respuesta::rechazo(rechazos::HEROE_DESCONOCIDO, format!("heroe-{heroe}"), v))?;
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
