//! Dentro de un pueblo extractor o de una avanzada, y echar a quien ya no puede estar dentro (Entrar con todo, decisiones del autor del 2026-10-22; Presencia_Y_Convocatoria
//! §4-§5, Combate §3.2 y §7): `entrarEnPueblo`, `salirDePueblo`, `entrarEnAvanzada` y `salirDeAvanzada`, y las salidas forzosas que cierran la puerta o el lugar.
//!
//! # Entrar y salir
//! Se entra como de visita en una plaza ajena (`salidas`): con todo lo suyo —tropa, carro, víveres y todas sus caravanas— y la columna entera queda dentro, fuera del mapa, comiendo de
//! sus víveres. No hay residencia que valga aquí: nadie reside en un pueblo ni en una avanzada, así que entra igual el residente de la plaza madre que el de otra Facción
//! y se lleva todo al salir. El acceso se hereda de la plaza madre ([`Nucleo::puede_entrar_en_pueblo`], [`Nucleo::puede_entrar_en_avanzada`]): el de otra Facción entra si el dueño
//! lo permite a su relación. Un pueblo ha de estar activo y una avanzada operativa (en obra o en ruinas no funciona nada). Entra solo quien va en una columna personal y un
//! ejército entero lo hace por orden de su Líder, que lo desarma y hace entrar a cada héroe como lo haría solo (como `guarnecer`).
//!
//! # Salir a la fuerza
//! Quien está dentro sale al mapa **al instante y con todo lo suyo** —su columna tal cual, a la puerta; o una nueva con su carro y sus víveres si no trajo ninguna— cuando:
//! - el dueño cierra la puerta a su relación (`fijarAcceso`: en la plaza y en los pueblos y avanzadas que la heredan; la misma Facción y los residentes no se expulsan);
//! - el lugar desaparece (un pueblo destruido a mano, una avanzada destruida o en ruinas, la plaza madre en ruinas);
//! - el lugar cambia de dueño (la madre conquistada): salen solo quienes el acceso heredado ya no deja entrar con el dueño nuevo, sus enemigos y los que cierra; los demás se quedan.
//!
//! Quien defendió y perdió queda sujeto a lo que le pase en la defensa (heridas, bajas y la mitad del carro, en `combates`); aquí solo se le saca.

use super::*;
use militar::columna::TipoColumna;
use militar::presencia::Ubicacion;

/// Por qué sale alguien a la fuerza, como número del hecho `heroe.expulsado` (`motivo`).
pub(super) const MOTIVO_PUERTA_CERRADA: i64 = 0;
/// El lugar queda en ruinas (una avanzada) o cae en ruinas su plaza madre.
pub(super) const MOTIVO_RUINA: i64 = 1;
/// El lugar cambia de dueño: su plaza madre fue conquistada.
pub(super) const MOTIVO_CAMBIA_DE_DUENO: i64 = 2;
/// El lugar se destruye: lo destruye su dueño o un ataque.
pub(super) const MOTIVO_DESTRUIDO: i64 = 3;

/// Un pueblo o una avanzada, visto como un sitio en el que se entra.
struct Sitio {
    lugar: Ubicacion,
    id: u64,
    /// La plaza madre, de la que hereda el acceso.
    madre: u64,
    puerta: Punto,
}

impl Sitio {
    fn es_pueblo(&self) -> bool {
        matches!(self.lugar, Ubicacion::Pueblo(_))
    }
}

/// El lugar como número del hecho `heroe.expulsado` (`lugar`) y su id: 0 una plaza, 1 un pueblo, 2 una avanzada.
fn clase_e_id(lugar: Ubicacion) -> (i64, u64) {
    match lugar {
        Ubicacion::Asentamiento(a) => (0, a as u64),
        Ubicacion::Pueblo(x) => (1, x),
        Ubicacion::Avanzada(x) => (2, x),
        Ubicacion::Columna(_) | Ubicacion::Mercenarios(_) | Ubicacion::Desconectado(_) => unreachable!("solo se echa de una plaza, un pueblo o una avanzada"),
    }
}

impl Nucleo {
    // ───────────────────────────── Los sitios ─────────────────────────────

    /// El pueblo como sitio. Para entrar ha de estar activo (una obra no tiene puerta).
    fn sitio_de_pueblo(&self, pueblo: u64, para_entrar: bool) -> Result<Sitio, Respuesta> {
        let v = self.estado.version;
        let (madre, pu) = self.pueblo(pueblo).ok_or_else(|| Respuesta::rechazo(rechazos::PUEBLO_DESCONOCIDO, ids::pueblo(pueblo), v))?;
        if para_entrar && !pu.activo() {
            return Err(Respuesta::rechazo(rechazos::PUEBLO_NO_ESTA_ACTIVO, "el pueblo aún es una obra: no tiene puerta", v));
        }
        let puerta = self.posicion_de_pueblo(pueblo).ok_or_else(|| Respuesta::rechazo(rechazos::SIN_MUNDO, "el núcleo no tiene mundo cargado", v))?;
        Ok(Sitio { lugar: Ubicacion::Pueblo(pueblo), id: pueblo, madre, puerta })
    }

    /// La avanzada como sitio. Para entrar ha de ser operativa (en obra o en ruinas no funciona nada).
    fn sitio_de_avanzada(&self, avanzada: u64, para_entrar: bool) -> Result<Sitio, Respuesta> {
        let v = self.estado.version;
        let (Some(madre), av) = self.avanzada(avanzada).ok_or_else(|| Respuesta::rechazo(rechazos::AVANZADA_DESCONOCIDA, ids::avanzada(avanzada), v))? else {
            return Err(Respuesta::rechazo(rechazos::AVANZADA_DESCONOCIDA, ids::avanzada(avanzada), v));
        };
        if para_entrar && !av.operativa() {
            return Err(Respuesta::rechazo(rechazos::AVANZADA_NO_OPERATIVA, "en obra o en ruinas no funciona nada, ni la puerta", v));
        }
        Ok(Sitio { lugar: Ubicacion::Avanzada(avanzada), id: avanzada, madre, puerta: av.posicion })
    }

    // ───────────────────────────── Las órdenes ─────────────────────────────

    /// `entrarEnPueblo`, `salirDePueblo`, `entrarEnAvanzada` y `salirDeAvanzada`.
    pub(super) fn ejecutar_dentro(&mut self, t: Instante, p: &Peticion) -> Result<Vec<u64>, Respuesta> {
        match &p.comando {
            Comando::EntrarEnPueblo { pueblo, heroe } => {
                let sitio = self.sitio_de_pueblo(*pueblo, true)?;
                self.entrar_en_sitio(t, p, &sitio, *heroe)
            }
            Comando::SalirDePueblo { pueblo, heroe } => {
                let sitio = self.sitio_de_pueblo(*pueblo, false)?;
                self.salir_de_sitio(t, p, &sitio, *heroe)
            }
            Comando::EntrarEnAvanzada { avanzada, heroe } => {
                let sitio = self.sitio_de_avanzada(*avanzada, true)?;
                self.entrar_en_sitio(t, p, &sitio, *heroe)
            }
            Comando::SalirDeAvanzada { avanzada, heroe } => {
                let sitio = self.sitio_de_avanzada(*avanzada, false)?;
                self.salir_de_sitio(t, p, &sitio, *heroe)
            }
            _ => unreachable!("solo se llama con órdenes de entrar y salir de un pueblo o una avanzada"),
        }
    }

    /// Entra solo quien va en una columna personal; un ejército (o una personal con compañía) entra entero, ordenado por su Líder, desarmándose.
    fn entrar_en_sitio(&mut self, t: Instante, p: &Peticion, sitio: &Sitio, heroe: u64) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        self.autorizar_heroe(p, heroe, false)?;
        let (id, e) = self.columna_a_bordo(heroe)?;
        let solo = e.columna.tipo == TipoColumna::Personal && e.columna.participantes.len() == 1;
        if solo && e.columna.es_formacion() {
            return Err(rechazo(rechazos::COLUMNA_EN_FORMACION, "cancela antes la formación".into()));
        }
        if !solo && e.columna.lider != heroe {
            return Err(rechazo(rechazos::COLUMNA_SOLO_EL_LIDER, "solo el Líder hace entrar al ejército".into()));
        }
        if !p.admin && e.posicion_en(t).dist(sitio.puerta) > self.radio_puerta() {
            let codigo = if sitio.es_pueblo() { rechazos::PUEBLO_FUERA_DE_LA_PUERTA } else { rechazos::AVANZADA_FUERA_DE_LA_PUERTA };
            return Err(rechazo(codigo, "la columna no está a la puerta".into()));
        }
        let antes = Self::participantes_de(e);
        if let Some(h) = antes.iter().find(|&&h| !self.puede_entrar_en_plaza(h, sitio.madre)) {
            return Err(rechazo(rechazos::PUERTA_CERRADA, format!("la plaza madre no abre su puerta al héroe {h}")));
        }
        let caravanas = e.caravanas_adjuntas.len() as i64;
        if !solo {
            self.desarmar_columna(t, id);
        }
        for &h in &antes {
            let columna = self.estado.ejercito_de_heroe(h).expect("va en una columna");
            self.quedar_dentro_de_visita(t, h, columna, sitio.lugar);
            let codigo = if sitio.es_pueblo() { "jugador.entra_en_pueblo" } else { "jugador.entra_en_avanzada" };
            let clave = if sitio.es_pueblo() { "pueblo" } else { "avanzada" };
            self.emitir_propio(t, codigo, vec![sitio.madre], vec![("heroe", h as i64), (clave, sitio.id as i64), ("ejercito", columna as i64), ("caravanas", self.estado.ejercitos[&columna].caravanas_adjuntas.len() as i64)]);
        }
        if !solo {
            let codigo = if sitio.es_pueblo() { "ejercito.entra_en_pueblo" } else { "ejercito.entra_en_avanzada" };
            let clave = if sitio.es_pueblo() { "pueblo" } else { "avanzada" };
            self.emitir_nombrando(t, codigo, vec![sitio.madre], antes.clone(), vec![("ejercito", id as i64), (clave, sitio.id as i64), ("heroes", antes.len() as i64), ("caravanas", caravanas)]);
        }
        Ok(vec![])
    }

    /// Sale con todo lo que entró: su columna vuelve al mapa tal cual, a la puerta.
    fn salir_de_sitio(&mut self, t: Instante, p: &Peticion, sitio: &Sitio, heroe: u64) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        self.autorizar_heroe(p, heroe, false)?;
        if self.heroe(heroe).expect("existe").heroe.presencia.ubicacion != sitio.lugar {
            let (codigo, nombre) = if sitio.es_pueblo() { (rechazos::HEROE_NO_ESTA_EN_EL_PUEBLO, "pueblo") } else { (rechazos::HEROE_NO_ESTA_EN_LA_AVANZADA, "avanzada") };
            return Err(Respuesta::rechazo(codigo, format!("el héroe no está dentro de {nombre}-{}", sitio.id), v));
        }
        let columna = self.salir_al_mapa(t, heroe, sitio.puerta);
        let (codigo, clave) = if sitio.es_pueblo() { ("jugador.sale_de_pueblo", "pueblo") } else { ("jugador.sale_de_avanzada", "avanzada") };
        self.emitir_propio(t, codigo, vec![sitio.madre], vec![("heroe", heroe as i64), (clave, sitio.id as i64), ("ejercito", columna as i64)]);
        Ok(vec![])
    }

    // ───────────────────────────── Quedarse dentro y salir al mapa ─────────────────────────────

    /// El héroe queda dentro de `dentro` de visita, con su columna entera: se detiene donde está, deja de estar en el mapa (nadie la ve ni la ataca) y sigue comiendo de sus víveres.
    pub(super) fn quedar_dentro_de_visita(&mut self, t: Instante, heroe: u64, columna: u64, dentro: Ubicacion) {
        if self.estado.ejercitos[&columna].marchando() {
            self.detener(columna, t);
        }
        self.heroe_mut(heroe).heroe.presencia.ubicacion = dentro;
        self.anunciar_columna(columna, Motivo::Plan);
        self.tr.tocar(Clase::Ejercito, columna);
    }

    /// El héroe que está dentro de un lugar sale al mapa con todo lo suyo: su columna (la que trajo) vuelve al mapa tal cual; si no trajo ninguna nace una columna personal quieta en
    /// `puerta`, con su carro y sus víveres. Devuelve su id.
    pub(super) fn salir_al_mapa(&mut self, t: Instante, heroe: u64, puerta: Punto) -> u64 {
        let hn = self.heroe(heroe).expect("existe");
        let (columna, faccion) = (hn.columna, hn.faccion);
        match columna {
            Some(c) => {
                self.ubicar_en_columna(heroe, c);
                self.tr.tocar(Clase::Ejercito, c);
                c
            }
            None => self.columna_de_salida(t, heroe, puerta, faccion, &[]),
        }
    }

    // ───────────────────────────── Salir a la fuerza ─────────────────────────────

    /// Los héroes que están dentro de `lugar`, por id.
    fn dentro_de(&self, lugar: Ubicacion) -> Vec<u64> {
        self.heroes().filter(|h| h.heroe.presencia.ubicacion == lugar).map(|h| h.heroe.id).collect()
    }

    /// Saca al mapa, con todo lo suyo, a `heroes`, que están dentro de `lugar` (de la plaza `plaza` o de uno de sus pueblos o avanzadas).
    fn expulsar(&mut self, t: Instante, heroes: Vec<u64>, lugar: Ubicacion, puerta: Punto, plaza: u64, motivo: i64) {
        let (clase, id) = clase_e_id(lugar);
        // La plaza madre que cae en ruinas ya no está en el estado: el hecho no la nombra.
        let plazas = if self.estado.asentamientos.contains_key(&plaza) { vec![plaza] } else { vec![] };
        for h in heroes {
            let columna = self.salir_al_mapa(t, h, puerta);
            self.emitir_propio(t, "heroe.expulsado", plazas.clone(), vec![("heroe", h as i64), ("lugar", clase), ("sitio", id as i64), ("ejercito", columna as i64), ("motivo", motivo)]);
        }
    }

    /// Quien acaba de dejar su Facción y está dentro de una plaza, un pueblo o una avanzada cuya puerta ya no le admite sale expulsado al mapa con lo suyo: sin columna, solo y con su carro
    /// (5B-1, 2026-10-24). Devuelve la columna con la que salió. Regla (decisión del autor, 2026-10-24, 5C-10): vale para
    /// cualquier lugar donde esté, no solo la plaza que deja (la regla de la puerta es «sale quien ya no puede entrar»).
    pub(super) fn echar_si_ya_no_entra(&mut self, t: Instante, heroe: u64, motivo: i64) -> Option<u64> {
        let lugar = self.heroe(heroe)?.heroe.presencia.ubicacion;
        let (plaza, puerta) = match lugar {
            Ubicacion::Asentamiento(a) => (u64::from(a), self.estado.asentamientos.get(&u64::from(a))?.posicion),
            Ubicacion::Pueblo(x) => (self.pueblo(x)?.0, self.posicion_de_pueblo(x)?),
            Ubicacion::Avanzada(x) => (self.avanzada(x)?.0?, self.posicion_de_avanzada(x)?),
            Ubicacion::Columna(_) | Ubicacion::Mercenarios(_) | Ubicacion::Desconectado(_) => return None,
        };
        if self.puede_entrar_en_plaza(heroe, plaza) {
            return None;
        }
        self.expulsar(t, vec![heroe], lugar, puerta, plaza, motivo);
        self.heroe(heroe)?.columna
    }

    /// El lugar desaparece o cambia de dueño: todos los que están dentro salen al mapa con lo suyo.
    pub(super) fn vaciar_lugar(&mut self, t: Instante, lugar: Ubicacion, puerta: Punto, plaza: u64, motivo: i64) {
        let dentro = self.dentro_de(lugar);
        self.expulsar(t, dentro, lugar, puerta, plaza, motivo);
    }

    /// La plaza madre desaparece (cae en ruinas): se vacían sus pueblos y sus avanzadas (`(id, puerta)` de cada uno) y todos los de dentro salen al mapa con lo suyo.
    pub(super) fn vaciar_pueblos_y_avanzadas(&mut self, t: Instante, plaza: u64, pueblos: &[(u64, Punto)], avanzadas: &[(u64, Punto)]) {
        for &(x, puerta) in pueblos {
            self.vaciar_lugar(t, Ubicacion::Pueblo(x), puerta, plaza, MOTIVO_RUINA);
        }
        for &(x, puerta) in avanzadas {
            self.vaciar_lugar(t, Ubicacion::Avanzada(x), puerta, plaza, MOTIVO_RUINA);
        }
    }

    /// La plaza madre acaba de cambiar de dueño: sus pueblos y sus avanzadas pasan al conquistador con el acceso heredado de la plaza. Salen al mapa, con todo lo suyo, solo quienes ese
    /// acceso ya no deja entrar con el dueño nuevo (sus enemigos y los que cierra); los demás se quedan dentro (2026-10-22, tercera ronda). Se mira sin la excepción de la residencia: quien
    /// residía en la madre la pierde con la conquista.
    pub(super) fn vaciar_tras_conquista(&mut self, t: Instante, plaza: u64) {
        let (pueblos, avanzadas) = self.sitios_de(plaza);
        for (lugar, puerta) in Self::ubicaciones(pueblos, avanzadas) {
            let fuera: Vec<u64> = self.dentro_de(lugar).into_iter().filter(|&h| !self.acceso_deja_pasar(h, plaza)).collect();
            self.expulsar(t, fuera, lugar, puerta, plaza, MOTIVO_CAMBIA_DE_DUENO);
        }
    }

    /// Los pueblos y las avanzadas de `sitios_de` como lugares con su puerta.
    fn ubicaciones(pueblos: Vec<(u64, Punto)>, avanzadas: Vec<(u64, Punto)>) -> Vec<(Ubicacion, Punto)> {
        pueblos.into_iter().map(|(x, p)| (Ubicacion::Pueblo(x), p)).chain(avanzadas.into_iter().map(|(x, p)| (Ubicacion::Avanzada(x), p))).collect()
    }

    /// ¿Está en una batalla activa algún pueblo o alguna avanzada de la plaza?
    pub(super) fn sitios_en_batalla(&self, plaza: u64) -> bool {
        let b = self.bloqueos();
        self.pueblos_de(plaza).iter().any(|p| b.pueblos.contains(&p.id)) || self.avanzadas_de(plaza).iter().any(|a| b.avanzadas.contains(&a.id))
    }

    /// Los pueblos y las avanzadas de una plaza con su puerta (los pueblos sin mundo no tienen sitio).
    pub(super) fn sitios_de(&self, plaza: u64) -> (Vec<(u64, Punto)>, Vec<(u64, Punto)>) {
        let pueblos = self.pueblos_de(plaza).iter().filter_map(|p| self.posicion_de_pueblo(p.id).map(|pos| (p.id, pos))).collect();
        let avanzadas = self.avanzadas_de(plaza).iter().map(|a| (a.id, a.posicion)).collect();
        (pueblos, avanzadas)
    }

    /// El dueño ha cerrado la puerta (`fijarAcceso`): sale al mapa, con todo lo suyo, quien está dentro de la plaza, de sus pueblos o de sus avanzadas y la puerta, con el acceso
    /// nuevo, ya no le abre. La misma Facción y los residentes pasan siempre (`puede_entrar_en_plaza`), así que nunca salen.
    pub(super) fn cerrar_la_puerta(&mut self, t: Instante, plaza: u64) {
        let (pueblos, avanzadas) = self.sitios_de(plaza);
        let puerta = self.estado.asentamientos[&plaza].posicion;
        let sitios = [(Ubicacion::Asentamiento(plaza as u32), puerta)].into_iter().chain(Self::ubicaciones(pueblos, avanzadas));
        for (lugar, puerta) in sitios.collect::<Vec<_>>() {
            let fuera: Vec<u64> = self.dentro_de(lugar).into_iter().filter(|&h| !self.puede_entrar_en_plaza(h, plaza)).collect();
            self.expulsar(t, fuera, lugar, puerta, plaza, MOTIVO_PUERTA_CERRADA);
        }
    }
}
