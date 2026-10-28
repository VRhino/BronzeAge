//! Las avanzadas dentro del núcleo (Doc 5.17, Economia_Construccion §14.1, Combate §7.4, Reloj_Y_Eventos, fase 4F). Una avanzada es un puesto fortificado de la plaza
//! madre, en el mapa: tres etapas (atalaya → fuerte → ciudadela), levantada como una **obra de mapa** igual que un pueblo extractor pero con su fin exacto en la
//! cola, con visión de 12, almacén, guarnición de serie (IA) y de jugadores, ruinas e invulnerabilidad. Las reglas puras son los crates `asentamiento::avanzadas` (la
//! obra, el cupo, las etapas) y `combate::avanzada` (el ataque, la guarnición de serie, el saqueo del almacén); aquí está lo que las une con el mapa, las columnas, los
//! héroes y la cola.
//!
//! # El ciclo
//! 1. **Colocar** (`ColocarAvanzada`): un héroe con su columna en el sitio, dentro de la zona de su plaza madre, sin pisar su ciudad (cuyo radio es una décima del de su
//!    zona) ni otro elemento del mapa (avanzadas, pueblos, campamentos: cada uno mide 0,2). Deja una obra «pendiente de materiales» que ocupa cupo, no se ve, no ve y no se ataca.
//! 2. **Entregar** (`EntregarMaterialesAAvanzada`): lo que lleva la columna a la puerta. Con todo entregado arranca la obra y su fin tiene **cita propia**:
//!    `SucesoCombate::FinObraAvanzada` (prioridad 1, antes del paso de minuto), en su instante exacto y ligada a la avanzada para poder cancelarla. Al acabar la primera obra
//!    la atalaya entra en la rejilla de encuentros como entidad fija y ve a 12.
//! 3. **Mejorar / reparar / cancelar**: otra obra de mapa; la avanzada sigue funcionando (mejora) o en ruinas (reparación) hasta que acaba. `CancelarObraDeAvanzada` la
//!    abandona sin reembolso (lo entregado se pierde) y descarta su cita; un ataque que derriba la avanzada también pierde la obra.
//! 4. **Guarnición**: la de serie (IA, fuera de cupo) nace completa la primera vez que se la necesita, con la milicia de lanceros del canon en los tres escalones o la tropa que
//!    eligió el dueño para cada uno entre las que la madre puede reclutar (tecnología y edificio), y se repone gratis de soldado en soldado, entera en 6 horas
//!    (`reposicionMinutos`) desde 0 (una cita por soldado, `SucesoCombate::ReponeAvanzada`); la de jugadores son escuadras de héroes residentes de la madre dejadas a la puerta (`Contenedor::Avanzada`), que comen el
//!    trigo de la madre.
//! 5. **Ataque** (`Atacar` con `ObjetivoAtaque::Avanzada`): solo un ejército. Con números se resuelve como un asedio; con servidores de batalla abre siempre
//!    una batalla de Unity (capacidad de asedio, sin convocatoria). Defienden **solo los que están dentro**: las guarniciones y los héroes que han entrado en la
//!    avanzada (`EntrarEnAvanzada`: de la Facción de la madre, aliados o vasallos, con las escuadras de su columna; en Unity entra de oficio solo la guarnición y los
//!    demás se unen), y una columna amiga a su puerta, en el mapa, no defiende (ni con números ni uniéndose a la batalla). Si gana el atacante: se lleva del almacén lo que le quepa en los carros y la atalaya queda
//!    destruida, el fuerte en ruinas con un día de invulnerabilidad (`SucesoCombate::FinInvulnerabilidad`; si pasado ese día lo atacan y gana el atacante, destruido) y la
//!    ciudadela en ruinas, que nunca se destruye. En ruinas no funciona nada: la guarnición de jugadores vuelve a 0 al campamento de su héroe y quien estaba dentro sale al mapa con
//!    lo suyo; los defensores en persona quedan heridos y los que residen en la madre pierden la mitad de su carro, que es del vencedor.
//! 6. **La madre**: si cae por conquista, las avanzadas pasan al conquistador (su guarnición de jugadores se va con el desalojo y la de serie y las elecciones se reinician);
//!    si cae en ruinas, la atalaya y el fuerte desaparecen y la ciudadela queda como ruina sin dueño, que repara quien tenga después su zona.
//!
//! Nada recurre por avanzada ociosa: los únicos plazos son el fin de la obra (en marcha), la reposición (incompleta) y la invulnerabilidad (un día).
//!
//! # Quién manda
//! Son obras de mapa del panel de la plaza madre (Doc 4.2.2, 5E): mejorar, reparar, destruir y cancelar las da el Gobernador o el Maestro de Obras de la madre, residente y dentro; elegir la
//! tropa de serie, el Gobernador (Doc 5.17); colocar, el Gobernador o el Maestro de Obras de la madre, residente, con su columna **en el sitio** en vez de dentro («la coloca el héroe en el sitio»; 5E-1, `Autoriza::CargoEnElSitio`). Guarnecer y retirar son de la tropa
//! de cada uno: la cuenta del héroe (guarnece quien reside en la madre). Las elecciones de tropa se comprueban al elegir (tecnología y edificio de la madre) y no se revisan después: si la Facción pierde la tecnología o la madre
//! el edificio, la elección sigue.

use super::combates::{caidos, DefensoresDeDentro};
use super::dentro::{MOTIVO_DESTRUIDO, MOTIVO_RUINA};
use super::*;
use crate::estado::{SucesoCombate, PRIO_ASENTAMIENTO, PRIO_UMBRAL};
use crate::estado_combate::{entidad_avanzada, entidad_obra_de_avanzada};
use crate::estado_comercio::id_de_entidad;
use ::encuentros::plan_quieto;
use asentamiento::asentamiento::Evento as EventoEco;
use asentamiento::avanzadas::{Avanzada, ErrorAvanzada, Estado as EstadoAvanzada, Etapa};
use asentamiento::Recurso;
use combate::asedio::{heridos_del_asedio, Asedio};
use combate::avanzada as regla;
use combate::botin::CarroDeColumna;
use combate::resolucion::Bando;
use militar::catalogo::Tropa;
use militar::escuadra::{Contenedor, Escuadra};
use militar::heroe::Residencia;
use militar::presencia::Ubicacion;
use militar::{liderazgo, racion, reclutamiento, total, Carga as Mercancia};
use std::collections::BTreeMap;

fn rechazo_de_avanzada(e: ErrorAvanzada) -> (&'static str, &'static str) {
    use rechazos::*;
    match e {
        ErrorAvanzada::NivelInsuficiente => (AVANZADA_NIVEL_INSUFICIENTE, "la plaza madre no tiene el nivel que pide esa etapa"),
        ErrorAvanzada::SinCupo => (AVANZADA_SIN_CUPO, "la plaza madre ya tiene todas las avanzadas que su nivel permite"),
        ErrorAvanzada::NoExiste => (AVANZADA_DESCONOCIDA, "no existe esa avanzada"),
        ErrorAvanzada::NoMejorable => (AVANZADA_NO_MEJORABLE, "solo se mejora una avanzada operativa que no sea ya una ciudadela"),
        ErrorAvanzada::NoReparable => (AVANZADA_NO_REPARABLE, "solo se reparan unas ruinas de fuerte o de ciudadela"),
        ErrorAvanzada::YaHayObra => (AVANZADA_YA_HAY_OBRA, "la avanzada ya tiene una obra en marcha"),
        ErrorAvanzada::NoEstaPendiente => (AVANZADA_NO_ESTA_PENDIENTE, "la obra no está pendiente de materiales"),
        ErrorAvanzada::NadaQueEntregar => (AVANZADA_NADA_QUE_ENTREGAR, "la columna no lleva nada que la obra necesite"),
        ErrorAvanzada::NoSeDestruyeALaMano => (AVANZADA_CIUDADELA_NO_SE_DESTRUYE, "una ciudadela no se destruye a mano"),
        ErrorAvanzada::DentroDelAsentamiento => (AVANZADA_DENTRO_DEL_ASENTAMIENTO, "una avanzada no se coloca sobre la ciudad de la plaza madre"),
        ErrorAvanzada::PisaOtroElemento => (AVANZADA_PISA_OTRO_ELEMENTO, "el sitio pisa otro elemento del mapa (una avanzada, un pueblo o un campamento)"),
        ErrorAvanzada::NadaQueCancelar => (AVANZADA_NADA_QUE_CANCELAR, "no hay una mejora ni una reparación que cancelar (la primera obra se cancela destruyendo la avanzada)"),
    }
}

impl Nucleo {
    // ───────────────────────────── Consultas ─────────────────────────────

    /// Una avanzada por id y la plaza madre a la que pertenece (`None`: unas ruinas de ciudadela sin dueño). Recorre las plazas: es para órdenes y consultas, no para
    /// el camino caliente.
    pub fn avanzada(&self, id: u64) -> Option<(Option<u64>, &Avanzada)> {
        self.estado.asentamientos.values().find_map(|a| a.economia.avanzada(id).map(|v| (Some(a.id), v))).or_else(|| self.estado.combate.huerfanas.get(&id).map(|v| (None, v)))
    }

    /// Las avanzadas de una plaza (en cualquier estado), por id.
    pub fn avanzadas_de(&self, plaza: u64) -> &[Avanzada] {
        self.estado.asentamientos.get(&plaza).map_or(&[], |a| &a.economia.avanzadas)
    }

    /// Las ruinas de ciudadela sin dueño, por id.
    pub fn avanzadas_huerfanas(&self) -> impl Iterator<Item = &Avanzada> {
        self.estado.combate.huerfanas.values()
    }

    /// Dónde está una avanzada.
    pub fn posicion_de_avanzada(&self, id: u64) -> Option<Punto> {
        self.avanzada(id).map(|(_, v)| v.posicion)
    }

    /// La guarnición de serie de una avanzada (vacía mientras no se ha necesitado, y en ruinas).
    pub fn guarnicion_de_serie_de_avanzada(&self, id: u64) -> &[Escuadra] {
        self.estado.combate.avanzadas.get(&id).map_or(&[], |g| &g.escuadras)
    }

    /// Las tropas que el Gobernador eligió para cada escalón de la guarnición de serie.
    pub fn tropas_elegidas_de_avanzada(&self, id: u64) -> BTreeMap<u8, Tropa> {
        self.estado.combate.avanzadas.get(&id).map_or_else(BTreeMap::new, |g| g.elegidas.clone())
    }

    /// Las escuadras de jugadores que guarnecen la avanzada, por héroe y en el orden de cada uno.
    pub fn guarnicion_de_jugadores_de_avanzada(&self, id: u64) -> Vec<&Escuadra> {
        self.heroes().flat_map(|h| h.heroe.escuadras.iter()).filter(|e| e.contenedor == Contenedor::Avanzada(id)).collect()
    }

    /// El cupo de la guarnición de jugadores de una avanzada por héroe residente de la madre, en puntos de Liderazgo.
    pub fn cupo_de_guarnicion_de_avanzada(&self, id: u64) -> Option<i64> {
        self.avanzada(id).map(|(_, v)| regla::cupo_de_guarnicion(&self.estado.reglas.economia.combate, v.etapa))
    }

    // ───────────────────────────── Ayudas ─────────────────────────────

    /// Los centros de lo que ya ocupa el mapa, con su diámetro de `TAMANOS_MAPA` (Doc 4.2.2): las avanzadas en cualquier estado (también unas ruinas sin dueño), los pueblos
    /// extractores sobre su nodo y los campamentos de mercenarios y de bandidos.
    fn elementos_de_mapa(&self) -> Vec<(Punto, Mil)> {
        let (asentamientos, combate) = (&self.estado.asentamientos, &self.estado.combate);
        let t = &self.estado.reglas.economia.datos.tamanos;
        let nodos = self.geografia.as_ref().map_or(&[][..], |g| &g.mapa.nodos[..]);
        let avanzadas = asentamientos.values().flat_map(|a| a.economia.avanzadas.iter()).chain(combate.huerfanas.values()).map(|v| (v.posicion, t.avanzada));
        let pueblos = asentamientos.values().flat_map(|a| a.economia.pueblos.iter()).filter_map(|p| crate::economia::posicion_de_pueblo(nodos, p)).map(|pos| (pos, t.pueblo));
        let mercenarios = self.geografia.iter().flat_map(|g| g.mapa.campamentos.iter().map(|&c| (c, t.campamento_mercenarios)));
        avanzadas.chain(pueblos).chain(mercenarios).chain(combate.bandidos.values().map(|b| (b.posicion, t.campamento_bandidos))).collect()
    }

    /// La avanzada y su plaza madre; las ruinas sin dueño no tienen madre y no reciben órdenes (salvo repararlas).
    fn con_madre(&self, id: u64) -> Result<(u64, &Avanzada), Respuesta> {
        match self.avanzada(id) {
            Some((Some(madre), v)) => Ok((madre, v)),
            _ => Err(Respuesta::rechazo(rechazos::AVANZADA_DESCONOCIDA, ids::avanzada(id), self.estado.version)),
        }
    }

    /// Cuenta un hecho de la avanzada: lo oye la plaza madre, si la hay.
    fn hecho_de_avanzada(&mut self, t: Instante, codigo: &'static str, madre: Option<u64>, punto: Punto, datos: Vec<(&'static str, i64)>) {
        self.tr.emitir(&self.estado, EventoNucleo { codigo: Cow::Borrowed(codigo), en: t, ejercitos: vec![], asentamientos: madre.into_iter().collect(), punto: Some(punto), datos, ..Default::default() });
        self.tr.tocar_todos(Clase::Asentamiento, madre);
    }

    /// Una avanzada terminada es una cosa fija del espacio: ahí la encuentran las columnas (el hecho `avanzada.encuentro` solo salta si además se puede atacar).
    pub(super) fn situar_avanzada(&mut self, id: u64, posicion: Punto) {
        let entidad = entidad_avanzada(id);
        self.espacio.rejilla.apuntar(entidad, self.estado.generaciones.actual(entidad), &plan_quieto(posicion, self.estado.ahora, 0));
    }

    /// Las escuadras de jugadores dejan la avanzada y vuelven al campamento de su héroe, con 0 soldados si la causa es una derrota (Doc 5.15.2b). Devuelve cuántas.
    fn liberar_guarnicion_de_jugadores(&mut self, avanzada: u64, derrota: bool) -> usize {
        let mut liberadas = 0;
        for h in self.estado.militar.heroes.values_mut() {
            let antes = liberadas;
            for e in h.heroe.escuadras.iter_mut().filter(|e| e.contenedor == Contenedor::Avanzada(avanzada)) {
                e.contenedor = Contenedor::Campamento;
                e.en_guarnicion = false;
                if derrota {
                    e.cantidad = 0;
                }
                liberadas += 1;
            }
            if liberadas > antes {
                self.tr.tocar(Clase::Heroe, h.heroe.id);
            }
        }
        liberadas
    }

    // ───────────────────────────── La guarnición de serie ─────────────────────────────

    /// La guarnición de serie nace completa la primera vez que se la necesita (un ataque): con la tropa por defecto o la que eligió el Gobernador en cada escalón.
    fn asegurar_guarnicion(&mut self, id: u64, etapa: Etapa, t: Instante) {
        if self.estado.combate.avanzadas.get(&id).is_some_and(|g| !g.escuadras.is_empty()) {
            return;
        }
        let Estado { combate, siguiente_id, reglas, .. } = &mut self.estado;
        self.tr.tocar(Clase::Avanzada, id);
        let g = combate.avanzadas.entry(id).or_default();
        g.escuadras = regla::guarnicion_de_serie(&reglas.economia.combate, etapa, &g.elegidas, || { *siguiente_id += 1; *siguiente_id - 1 }, t);
    }

    /// Guarda la guarnición de serie tras un combate y, si quedó incompleta y no se estaba reponiendo, cita la vuelta del primer soldado (de uno en uno,
    /// como los mineros de un pueblo; un combate durante la reposición no la reinicia).
    pub(super) fn guardar_guarnicion_de_avanzada(&mut self, id: u64, escuadras: Vec<Escuadra>, t: Instante) {
        let cr = self.estado.reglas.economia.combate.clone();
        let Some(g) = self.estado.combate.avanzadas.get_mut(&id) else { return };
        self.tr.tocar(Clase::Avanzada, id);
        g.escuadras = escuadras;
        if g.repone_en.is_none() && regla::incompleta(&cr, &g.escuadras) {
            let en = regla::reposicion_en(&cr, &g.escuadras, t);
            self.programar_soldado_de_avanzada(id, en);
        }
    }

    /// La cita de la vuelta del siguiente soldado de la guarnición de serie de una avanzada.
    fn programar_soldado_de_avanzada(&mut self, id: u64, en: Instante) {
        self.estado.combate.avanzadas.get_mut(&id).expect("existe").repone_en = Some(en);
        let clave = self.estado.generaciones.clave(entidad_avanzada(id));
        self.estado.cola.programar(en, PRIO_UMBRAL, clave, Carga::Combate(SucesoCombate::ReponeAvanzada));
    }

    /// La guarnición de serie ya nacida se rehace con lo que toca ahora (otra etapa, otra elección) **si está completa**; si está esperando su reposición, lo que
    /// toca ahora llega con ella (cambiar de tropa no cura a nadie).
    fn recomponer_guarnicion_de_serie(&mut self, id: u64, t: Instante) {
        let Some(g) = self.estado.combate.avanzadas.get_mut(&id) else { return };
        if g.escuadras.is_empty() || g.repone_en.is_some() {
            return;
        }
        g.escuadras.clear();
        self.tr.tocar(Clase::Avanzada, id);
        if let Some((_, v)) = self.avanzada(id) {
            let etapa = v.etapa;
            self.asegurar_guarnicion(id, etapa, t);
        }
    }

    /// Vuelve un soldado a la guarnición de serie, con su nivel y su experiencia; si aún falta alguno, se cita el siguiente. Completa, se rehace con las tropas que
    /// toquen si desde entonces cambiaron (otra elección, otra etapa) y lo dice.
    pub(super) fn repone_avanzada(&mut self, entidad: u64, t: Instante) {
        let id = id_de_entidad(entidad);
        let cr = self.estado.reglas.economia.combate.clone();
        let Some(g) = self.estado.combate.avanzadas.get_mut(&id).filter(|g| g.repone_en == Some(t)) else { return };
        g.repone_en = None;
        self.tr.tocar(Clase::Avanzada, id);
        if !regla::reponer(&cr, &mut g.escuadras) {
            let en = regla::reposicion_en(&cr, &g.escuadras, t);
            return self.programar_soldado_de_avanzada(id, en);
        }
        let Some((madre, v)) = self.avanzada(id) else { return };
        let (etapa, punto) = (v.etapa, v.posicion);
        let g = &self.estado.combate.avanzadas[&id];
        if regla::composicion(&cr, etapa, &g.elegidas) != g.escuadras.iter().map(|e| e.tropa).collect::<Vec<_>>() {
            self.estado.combate.avanzadas.get_mut(&id).expect("existe").escuadras.clear();
            self.asegurar_guarnicion(id, etapa, t);
        }
        self.hecho_de_avanzada(t, "avanzada.guarnicion_repuesta", madre, punto, vec![("avanzada", id as i64)]);
    }

    /// Vence el día de invulnerabilidad de un fuerte en ruinas: vuelve a poder atacarse.
    pub(super) fn fin_de_invulnerabilidad(&mut self, entidad: u64, t: Instante) {
        let id = id_de_entidad(entidad);
        let Some(madre) = self.estado.asentamientos.values().find(|a| a.economia.avanzada(id).is_some()).map(|a| a.id) else { return };
        let v = self.estado.asentamientos.get_mut(&madre).and_then(|a| a.economia.avanzadas.iter_mut().find(|v| v.id == id)).expect("existe");
        let EstadoAvanzada::EnRuinas { invulnerable_hasta: Some(hasta) } = v.estado else { return };
        if t < hasta {
            return;
        }
        v.estado = EstadoAvanzada::EnRuinas { invulnerable_hasta: None };
        let punto = v.posicion;
        self.tr.tocar(Clase::Avanzada, id);
        self.hecho_de_avanzada(t, "avanzada.invulnerabilidad_terminada", Some(madre), punto, vec![("avanzada", id as i64)]);
    }

    // ───────────────────────────── Las obras ─────────────────────────────

    /// La obra de una avanzada ha terminado (`AvanzadaCompletada`): la primera obra la pone en el espacio, la mejora cambia su guarnición de serie y la reparación
    /// la saca de sus ruinas.
    pub(super) fn avanzada_completada(&mut self, madre: u64, sucesos: &[EventoEco], t: Instante) {
        for s in sucesos {
            let EventoEco::AvanzadaCompletada { id, tipo, .. } = *s else { continue };
            let Some(posicion) = self.avanzadas_de(madre).iter().find(|v| v.id == id).map(|v| v.posicion) else { continue };
            self.situar_avanzada(id, posicion);
            if tipo == asentamiento::avanzadas::TipoObra::Mejorar {
                self.recomponer_guarnicion_de_serie(id, t);
            }
        }
    }

    // ───────────────────────────── Órdenes ─────────────────────────────

    /// Las órdenes de las avanzadas. `Ok(ids)` = cambió algo y creó esos ids; rechazos y «sin cambios» salen como `Err` sin haber tocado el estado.
    pub(super) fn ejecutar_avanzadas(&mut self, t: Instante, p: &Peticion) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let sin_cambios = || Respuesta::sin_cambios(v);
        let de_avanzada = |e: ErrorAvanzada| {
            let (c, m) = rechazo_de_avanzada(e);
            rechazo(c, m.into())
        };
        let datos = self.estado.reglas.economia.datos.clone();
        match &p.comando {
            Comando::ColocarAvanzada { asentamiento, heroe, posicion } => {
                let plaza = self.plaza_o_rechazo(*asentamiento)?;
                let (centro, radio) = (plaza.posicion, plaza.radio);
                self.autorizar_en_plaza(p, *asentamiento, Some(*heroe))?;
                let (_, e) = self.columna_a_bordo(*heroe)?;
                if !p.admin && e.posicion_en(t).dist(*posicion) > self.radio_puerta() {
                    return Err(rechazo(rechazos::AVANZADA_FUERA_DEL_SITIO, "la columna del héroe no está en el sitio donde se coloca la avanzada".into()));
                }
                if !self.en_zona(*asentamiento, *posicion) {
                    return Err(rechazo(rechazos::AVANZADA_FUERA_DE_LA_ZONA, format!("el sitio cae fuera de la zona de as-{asentamiento}")));
                }
                datos.avanzadas.comprobar_sitio(centro, radio, *posicion, self.elementos_de_mapa()).map_err(de_avanzada)?;
                let (id, mut ev) = (self.estado.siguiente_id, vec![]);
                self.estado.asentamientos.get_mut(asentamiento).expect("existe").economia.iniciar_avanzada(&datos, id, *posicion, &mut ev).map_err(de_avanzada)?;
                self.estado.siguiente_id += 1;
                self.anotar_sucesos(*asentamiento, t, &ev);
                Ok(vec![id])
            }
            Comando::EntregarMaterialesAAvanzada { ejercito, avanzada } => {
                let e = self.estado.ejercitos.get(ejercito).ok_or_else(|| rechazo(rechazos::EJERCITO_DESCONOCIDO, ids::ejercito(*ejercito)))?;
                if !self.manda_columna(p, e, false) {
                    return Err(rechazo(rechazos::NO_AUTORIZADO, "la columna no es tuya".into()));
                }
                let (madre, av) = self.con_madre(*avanzada)?;
                let faccion = self.estado.asentamientos[&madre].faccion;
                if e.columna.faccion != Some(faccion) {
                    return Err(rechazo(rechazos::AVANZADA_FACCION_AJENA, "la obra es de otra Facción".into()));
                }
                if !self.entrega_un_ciudadano(p, e, faccion) {
                    return Err(rechazo(rechazos::AVANZADA_FACCION_AJENA, "solo entregan los ciudadanos de la Facción de la obra".into()));
                }
                if e.posicion_en(t).dist(av.posicion) > self.radio_puerta() {
                    return Err(rechazo(rechazos::AVANZADA_FUERA_DE_LA_PUERTA, "la columna no está a la puerta de la obra".into()));
                }
                let disponible: asentamiento::Costo = self.material_de_columna(*ejercito).into_iter().collect();
                let mut ev = vec![];
                let eco = &mut self.estado.asentamientos.get_mut(&madre).expect("existe").economia;
                let tomado = eco.entregar_a_avanzada(t, &datos, *avanzada, &disponible, &mut ev).map_err(de_avanzada)?;
                self.quitar_de_columna(*ejercito, &tomado);
                let lider = self.estado.ejercitos[ejercito].columna.lider;
                self.avance(t, Dueno::Heroe(lider), "entregas", 1);
                self.tr.tocar(Clase::Ejercito, *ejercito);
                self.anotar_sucesos(madre, t, &ev);
                Ok(vec![])
            }
            Comando::MejorarAvanzada { heroe, avanzada } => {
                let (madre, _) = self.con_madre(*avanzada)?;
                self.autorizar_en_plaza(p, madre, Some(*heroe))?;
                let mut ev = vec![];
                self.estado.asentamientos.get_mut(&madre).expect("existe").economia.mejorar_avanzada(&datos, *avanzada, &mut ev).map_err(de_avanzada)?;
                self.anotar_sucesos(madre, t, &ev);
                Ok(vec![])
            }
            Comando::RepararAvanzada { heroe, avanzada } => self.reparar_avanzada(t, p, *heroe, *avanzada),
            Comando::DestruirAvanzada { heroe, avanzada } => {
                let (madre, av) = self.con_madre(*avanzada)?;
                let posicion = av.posicion;
                self.autorizar_en_plaza(p, madre, Some(*heroe))?;
                let mut ev = vec![];
                self.estado.asentamientos.get_mut(&madre).expect("existe").economia.destruir_avanzada(*avanzada, &mut ev).map_err(de_avanzada)?;
                self.vaciar_lugar(t, Ubicacion::Avanzada(*avanzada), posicion, madre, MOTIVO_DESTRUIDO);
                self.quitar_avanzada(*avanzada);
                self.liberar_guarnicion_de_jugadores(*avanzada, false);
                self.anotar_sucesos(madre, t, &ev);
                Ok(vec![])
            }
            Comando::CancelarObraDeAvanzada { heroe, avanzada } => {
                let (madre, _) = self.con_madre(*avanzada)?;
                self.autorizar_en_plaza(p, madre, Some(*heroe))?;
                let mut ev = vec![];
                self.estado.asentamientos.get_mut(&madre).expect("existe").economia.cancelar_obra_de_avanzada(*avanzada, &mut ev).map_err(de_avanzada)?;
                self.perder_obra(*avanzada);
                self.anotar_sucesos(madre, t, &ev);
                Ok(vec![])
            }
            Comando::ElegirTropaDeAvanzada { heroe, avanzada, tropa } => {
                let (madre, v) = self.con_madre(*avanzada)?;
                let (faccion, punto) = (self.estado.asentamientos[&madre].faccion, v.posicion);
                self.autorizar_en_plaza(p, madre, Some(*heroe))?;
                let cr = self.estado.reglas.economia.combate.clone();
                let escalon = regla::escalon_elegible(&cr, *tropa).ok_or_else(|| rechazo(rechazos::AVANZADA_TROPA_NO_ELEGIBLE, "la guarnición de serie solo usa tropas de los escalones 2, 3 y 4".into()))?;
                // Solo lo que la plaza madre puede reclutar: la tecnología de la Facción y el edificio activo al nivel que pide la tropa.
                let def = cr.militar.tropas.tropa(*tropa);
                if !self.tiene_tecnologia(faccion, &def.tecnologia) {
                    return Err(rechazo(rechazos::AVANZADA_TROPA_SIN_TECNOLOGIA, "la Facción no tiene la tecnología de esa tropa".into()));
                }
                if !reclutamiento::edificio_basta(def, &self.estado.asentamientos[&madre].economia.edificios) {
                    return Err(rechazo(rechazos::AVANZADA_TROPA_SIN_EDIFICIO, "la plaza madre no tiene el edificio de esa tropa, activo y al nivel que pide".into()));
                }
                let vigente = self.estado.combate.avanzadas.get(avanzada).and_then(|g| g.elegidas.get(&escalon)).copied().unwrap_or(cr.avanzada.tropa_por_defecto);
                if vigente == *tropa {
                    return Err(sin_cambios());
                }
                self.estado.combate.avanzadas.entry(*avanzada).or_default().elegidas.insert(escalon, *tropa);
                self.recomponer_guarnicion_de_serie(*avanzada, t);
                self.hecho_de_avanzada(t, "avanzada.tropa_elegida", Some(madre), punto, vec![("avanzada", *avanzada as i64), ("escalon", escalon as i64), ("tropa", crate::codec_militar::tropa_a_u8(*tropa) as i64)]);
                Ok(vec![])
            }
            Comando::GuarnecerAvanzada { heroe, avanzada, escuadras } => self.guarnecer_avanzada(t, p, *heroe, *avanzada, escuadras),
            Comando::RetirarDeAvanzada { heroe, avanzada, escuadras } => self.retirar_de_avanzada(t, p, *heroe, *avanzada, escuadras),
            // Como `repostar` en una plaza, vale para cualquier columna de la Facción cuyo Líder lo pida, personal o ejército (decisión del autor, 2026-10-22).
            Comando::RepostarEnAvanzada { heroe, avanzada } => {
                let (madre, av) = self.con_madre(*avanzada)?;
                let (faccion, posicion) = (self.estado.asentamientos[&madre].faccion, av.posicion);
                self.autorizar_heroe(p, *heroe, false)?;
                let (cid, e) = self.columna_a_bordo(*heroe)?;
                if e.columna.lider != *heroe {
                    return Err(rechazo(rechazos::COLUMNA_SOLO_EL_LIDER, "solo el Líder reposta".into()));
                }
                if e.columna.faccion != Some(faccion) {
                    return Err(rechazo(rechazos::AVANZADA_FACCION_AJENA, "esa avanzada no abre su almacén a esta columna".into()));
                }
                if !p.admin && e.posicion_en(t).dist(posicion) > self.radio_puerta() {
                    return Err(rechazo(rechazos::AVANZADA_FUERA_DE_LA_PUERTA, "la columna no está a la puerta de la avanzada".into()));
                }
                if !av.operativa() {
                    return Err(rechazo(rechazos::AVANZADA_NO_OPERATIVA, "en ruinas o en obra no funciona nada, ni el reabastecimiento".into()));
                }
                if av.almacen.capacidad(Recurso::Trigo) == 0 {
                    return Err(rechazo(rechazos::AVANZADA_SIN_ALMACEN, "una atalaya no tiene almacén".into()));
                }
                let (llevaba, cap) = (e.columna.viveres.valor_en(t), e.columna.viveres.maximo.unwrap_or(0));
                if cap - llevaba < base::UNIDAD {
                    return Err(rechazo(rechazos::COLUMNA_VIVERES_LLENOS, "los víveres ya están llenos".into()));
                }
                if av.almacen.cantidad(Recurso::Trigo) == 0 {
                    return Err(rechazo(rechazos::REPOSTAR_SIN_EXISTENCIAS, "el almacén de la avanzada no tiene trigo".into()));
                }
                let de_sobra = av.almacen.cantidad(Recurso::Trigo);
                let tomado = racion::llenar_en_unidades(&mut self.estado.ejercitos.get_mut(&cid).expect("existe").columna.viveres, t, de_sobra);
                self.estado.asentamientos.get_mut(&madre).and_then(|a| a.economia.avanzadas.iter_mut().find(|v| v.id == *avanzada)).expect("existe").almacen.quitar(Recurso::Trigo, tomado);
                self.reprogramar_columna(cid, t);
                self.hecho_de_avanzada(t, "avanzada.reabastecimiento", Some(madre), posicion, vec![("avanzada", *avanzada as i64), ("heroe", *heroe as i64), ("repuesto", tomado)]);
                self.avance(t, Dueno::Heroe(*heroe), "repostajes", 1);
                Ok(vec![])
            }
            Comando::DepositarEnAvanzada { ejercito, avanzada, recurso, cantidad } => {
                let e = self.estado.ejercitos.get(ejercito).ok_or_else(|| rechazo(rechazos::EJERCITO_DESCONOCIDO, ids::ejercito(*ejercito)))?;
                if !self.manda_columna(p, e, false) {
                    return Err(rechazo(rechazos::NO_AUTORIZADO, "la columna no es tuya".into()));
                }
                let (madre, av) = self.con_madre(*avanzada)?;
                if e.columna.faccion != Some(self.estado.asentamientos[&madre].faccion) {
                    return Err(rechazo(rechazos::AVANZADA_FACCION_AJENA, "esa avanzada es de otra Facción".into()));
                }
                if e.posicion_en(t).dist(av.posicion) > self.radio_puerta() {
                    return Err(rechazo(rechazos::AVANZADA_FUERA_DE_LA_PUERTA, "la columna no está a la puerta de la avanzada".into()));
                }
                if *cantidad <= 0 {
                    return Err(rechazo(rechazos::CANTIDAD_NO_POSITIVA, "hay que depositar una cantidad positiva".into()));
                }
                if !av.operativa() {
                    return Err(rechazo(rechazos::AVANZADA_NO_OPERATIVA, "en ruinas o en obra no funciona nada, ni el almacén".into()));
                }
                if av.almacen.capacidad(*recurso) == 0 {
                    return Err(rechazo(rechazos::AVANZADA_SIN_ALMACEN, "una atalaya no tiene almacén".into()));
                }
                let lleva = self.material_de_columna(*ejercito).get(recurso).copied().unwrap_or(0);
                if lleva == 0 {
                    return Err(rechazo(rechazos::AVANZADA_NADA_QUE_DEPOSITAR, format!("la columna no lleva {}", recurso.nombre())));
                }
                if av.almacen.espacio(*recurso) == 0 {
                    return Err(rechazo(rechazos::AVANZADA_ALMACEN_LLENO, format!("el almacén no tiene sitio para más {}", recurso.nombre())));
                }
                let punto = av.posicion;
                let metido = self.estado.asentamientos.get_mut(&madre).and_then(|a| a.economia.avanzadas.iter_mut().find(|v| v.id == *avanzada)).expect("existe").almacen.agregar(*recurso, (*cantidad).min(lleva));
                self.quitar_de_columna(*ejercito, &vec![(*recurso, metido)]);
                self.tr.tocar(Clase::Ejercito, *ejercito);
                self.hecho_de_avanzada(t, "avanzada.deposito", Some(madre), punto, vec![("avanzada", *avanzada as i64), ("recurso", crate::codec_economia::recurso_a_u8(*recurso) as i64), ("cantidad", metido)]);
                Ok(vec![])
            }
            _ => unreachable!("solo se llama con órdenes de avanzadas"),
        }
    }

    /// Una destrucción (a mano o por un ataque) saca la avanzada del espacio, borra su guarnición de serie y pierde su obra en marcha; los demás plazos pendientes
    /// encuentran el estado cambiado y no hacen nada.
    fn quitar_avanzada(&mut self, id: u64) {
        self.espacio.rejilla.quitar(entidad_avanzada(id));
        self.estado.combate.avanzadas.remove(&id);
        self.perder_obra(id);
    }

    /// La obra de la avanzada se abandona o se pierde con ella: su cita de fin deja de ser vigente (sube la generación de su entidad) y no se cumple.
    fn perder_obra(&mut self, id: u64) {
        self.estado.generaciones.subir(entidad_obra_de_avanzada(id));
    }

    /// Vence la cita de fin de la obra de una avanzada: la plaza madre cumple lo que acaba en ese instante (la primera obra, la mejora o la reparación).
    pub(super) fn fin_de_obra_de_avanzada(&mut self, entidad: u64, t: Instante) {
        self.tr.tocar(Clase::Avanzada, id_de_entidad(entidad));
        if let Some((Some(madre), _)) = self.avanzada(id_de_entidad(entidad)) {
            self.fines_de_plaza(madre, t);
        }
    }

    /// `repararAvanzada`: unas ruinas de fuerte o ciudadela. Una ciudadela sin dueño pasa antes a la plaza de la Facción del héroe que tiene ahora su zona (si cabe en su cupo).
    fn reparar_avanzada(&mut self, t: Instante, p: &Peticion, heroe: u64, avanzada: u64) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let de_avanzada = |e: ErrorAvanzada| {
            let (c, m) = rechazo_de_avanzada(e);
            rechazo(c, m.into())
        };
        let datos = self.estado.reglas.economia.datos.clone();
        let (madre, huerfana) = match self.avanzada(avanzada) {
            Some((Some(madre), _)) => (madre, None),
            Some((None, ruinas)) => {
                self.autorizar_heroe(p, heroe, false)?;
                let faccion = self.heroe(heroe).and_then(|h| h.faccion).ok_or_else(|| rechazo(rechazos::NO_AUTORIZADO, "el héroe no es ciudadano de ninguna Facción".into()))?;
                let posicion = ruinas.posicion;
                let madre = self.estado.asentamientos.values().find(|a| a.faccion == faccion && self.en_zona(a.id, posicion)).map(|a| a.id);
                (madre.ok_or_else(|| rechazo(rechazos::AVANZADA_FUERA_DE_LA_ZONA, "ninguna plaza de tu Facción tiene ahora la zona de esas ruinas".into()))?, Some(ruinas.clone()))
            }
            None => return Err(rechazo(rechazos::AVANZADA_DESCONOCIDA, ids::avanzada(avanzada))),
        };
        self.autorizar_en_plaza(p, madre, Some(heroe))?;
        let mut ev = vec![];
        let eco = &mut self.estado.asentamientos.get_mut(&madre).expect("existe").economia;
        if let Some(ruinas) = huerfana {
            // La adopción comprueba el nivel y el cupo que la reparación pediría después: si pasa, la reparación no puede fallar.
            eco.adoptar_avanzada(&datos, ruinas).map_err(de_avanzada)?;
            self.estado.combate.huerfanas.remove(&avanzada);
            let eco = &mut self.estado.asentamientos.get_mut(&madre).expect("existe").economia;
            eco.reparar_avanzada(&datos, avanzada, &mut ev).expect("la adopción ya comprobó el nivel y el cupo");
            let punto = self.avanzadas_de(madre).iter().find(|v| v.id == avanzada).map(|v| v.posicion).expect("recién adoptada");
            self.hecho_de_avanzada(t, "avanzada.adoptada", Some(madre), punto, vec![("avanzada", avanzada as i64)]);
        } else {
            eco.reparar_avanzada(&datos, avanzada, &mut ev).map_err(de_avanzada)?;
        }
        self.anotar_sucesos(madre, t, &ev);
        Ok(vec![])
    }

    /// `guarnecerAvanzada`: el héroe residente de la madre, con su columna a la puerta, deja escuadras suyas de la columna en la guarnición de jugadores de la avanzada,
    /// dentro de su cupo (14 / 45 / 90 puntos de Liderazgo). Comen del trigo de la madre; la columna recalcula lo que come y su velocidad.
    fn guarnecer_avanzada(&mut self, t: Instante, p: &Peticion, heroe: u64, avanzada: u64, escuadras: &[u64]) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let (madre, av) = self.con_madre(avanzada)?;
        let (posicion, etapa, punto) = (av.posicion, av.etapa, av.posicion);
        self.autorizar_heroe(p, heroe, false)?;
        let (cid, e) = self.columna_a_bordo(heroe)?;
        if !p.admin && e.posicion_en(t).dist(posicion) > self.radio_puerta() {
            return Err(rechazo(rechazos::AVANZADA_FUERA_DE_LA_PUERTA, "la columna del héroe no está a la puerta de la avanzada".into()));
        }
        if !av.operativa() {
            return Err(rechazo(rechazos::AVANZADA_NO_OPERATIVA, "solo se guarnece una avanzada operativa".into()));
        }
        let h = &self.heroe(heroe).expect("existe").heroe;
        if h.residencia != Some(Residencia::Asentamiento(madre as u32)) {
            return Err(rechazo(rechazos::AVANZADA_NO_RESIDES, "solo los héroes residentes de la plaza madre guarnecen sus avanzadas".into()));
        }
        if escuadras.is_empty() {
            return Err(rechazo(rechazos::SALIDA_SIN_ESCUADRAS, "hay que dejar al menos una escuadra".into()));
        }
        let mr = self.estado.reglas.economia.militar.clone();
        let mut vistas = std::collections::BTreeSet::new();
        for &id in escuadras {
            let s = h.escuadra(id).ok_or_else(|| rechazo(rechazos::ESCUADRA_AJENA, format!("la escuadra {id} no es tuya")))?;
            if !vistas.insert(id) {
                return Err(rechazo(rechazos::ESCUADRA_REPETIDA, format!("la escuadra {id} se repite")));
            }
            if s.contenedor != Contenedor::Ejercito(cid) {
                return Err(rechazo(rechazos::AVANZADA_ESCUADRA_FUERA_DE_TU_COLUMNA, format!("la escuadra {id} no va en tu columna")));
            }
            if s.cantidad <= 0 {
                return Err(rechazo(rechazos::ESCUADRA_ANIQUILADA, format!("la escuadra {id} está aniquilada")));
            }
            if s.reserva_batalla.is_some() {
                return Err(rechazo(rechazos::ESCUADRA_EN_BATALLA, format!("la escuadra {id} está reservada para una batalla")));
            }
        }
        let cupo = regla::cupo_de_guarnicion(&self.estado.reglas.economia.combate, etapa);
        let ocupado = liderazgo::comprometido(&mr, h.escuadras.iter().filter(|s| s.contenedor == Contenedor::Avanzada(avanzada) || vistas.contains(&s.id)));
        if ocupado > cupo {
            return Err(rechazo(rechazos::GUARNICION_SUPERA_EL_CUPO, format!("la guarnición pasaría a {ocupado} de un cupo de {cupo}")));
        }
        let antes = self.velocidad_de_ejercito(cid).unwrap_or(0);
        self.estado.ejercitos.get_mut(&cid).expect("existe").columna.escuadras.retain(|s| !vistas.contains(s));
        let ids: Vec<u64> = vistas.iter().copied().collect();
        self.poner_escuadras_en(&[heroe], &ids, Contenedor::Avanzada(avanzada), t);
        self.recomponer_columna(cid, t, antes);
        self.tr.tocar(Clase::Ejercito, cid);
        self.hecho_de_avanzada(t, "avanzada.guarnecida", Some(madre), punto, vec![("avanzada", avanzada as i64), ("heroe", heroe as i64), ("escuadras", ids.len() as i64), ("liderazgo", ocupado)]);
        Ok(vec![])
    }

    /// El héroe, con su columna a la puerta, recoge de la avanzada escuadras suyas. Las escuadras que lleva fuera del campamento (con las que recoge) no pasan de su
    /// Liderazgo.
    fn retirar_de_avanzada(&mut self, t: Instante, p: &Peticion, heroe: u64, avanzada: u64, escuadras: &[u64]) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let (madre, av) = self.con_madre(avanzada)?;
        let posicion = av.posicion;
        self.autorizar_heroe(p, heroe, false)?;
        let (cid, e) = self.columna_a_bordo(heroe)?;
        if !p.admin && e.posicion_en(t).dist(posicion) > self.radio_puerta() {
            return Err(rechazo(rechazos::AVANZADA_FUERA_DE_LA_PUERTA, "la columna del héroe no está a la puerta de la avanzada".into()));
        }
        if escuadras.is_empty() {
            return Err(rechazo(rechazos::SALIDA_SIN_ESCUADRAS, "hay que recoger al menos una escuadra".into()));
        }
        let mr = self.estado.reglas.economia.militar.clone();
        let h = &self.heroe(heroe).expect("existe").heroe;
        let mut vistas = std::collections::BTreeSet::new();
        for &id in escuadras {
            let s = h.escuadra(id).ok_or_else(|| rechazo(rechazos::ESCUADRA_AJENA, format!("la escuadra {id} no es tuya")))?;
            if !vistas.insert(id) {
                return Err(rechazo(rechazos::ESCUADRA_REPETIDA, format!("la escuadra {id} se repite")));
            }
            if s.contenedor != Contenedor::Avanzada(avanzada) {
                return Err(rechazo(rechazos::AVANZADA_ESCUADRA_NO_ESTA, format!("la escuadra {id} no está en esa avanzada")));
            }
            if s.reserva_batalla.is_some() {
                return Err(rechazo(rechazos::ESCUADRA_EN_BATALLA, format!("la escuadra {id} está reservada para una batalla")));
            }
        }
        let fuera = h.escuadras.iter().filter(|s| s.contenedor != Contenedor::Campamento && s.contenedor != Contenedor::Avanzada(avanzada) || vistas.contains(&s.id));
        if !liderazgo::puede_llevar(&mr, h.liderazgo_base, fuera) {
            return Err(rechazo(rechazos::LIDERAZGO_SUPERADO, "las escuadras pasan del Liderazgo del héroe".into()));
        }
        let antes = self.velocidad_de_ejercito(cid).unwrap_or(0);
        let ids: Vec<u64> = vistas.iter().copied().collect();
        self.estado.ejercitos.get_mut(&cid).expect("existe").columna.escuadras.extend(&ids);
        self.poner_escuadras_en(&[heroe], &ids, Contenedor::Ejercito(cid), t);
        self.sincronizar_hambre(cid, t);
        self.recomponer_columna(cid, t, antes);
        self.tr.tocar(Clase::Ejercito, cid);
        self.hecho_de_avanzada(t, "avanzada.guarnicion_retirada", Some(madre), posicion, vec![("avanzada", avanzada as i64), ("heroe", heroe as i64), ("escuadras", ids.len() as i64)]);
        Ok(vec![])
    }

    // ───────────────────────────── Cuando cae la madre ─────────────────────────────

    /// La plaza madre cae en ruinas (`avanzadas` son las suyas, ya sacadas del estado): la guarnición de jugadores vuelve intacta al campamento de su héroe, la
    /// atalaya y el fuerte desaparecen y la ciudadela queda como ruina sin dueño, que repara quien tenga después su zona (Doc 5.17).
    pub(super) fn avanzadas_de_la_madre_arruinada(&mut self, avanzadas: Vec<Avanzada>, t: Instante) {
        for mut av in avanzadas {
            self.liberar_guarnicion_de_jugadores(av.id, false);
            self.quitar_avanzada(av.id);
            self.tr.tocar(Clase::Avanzada, av.id);
            let id = av.id as i64;
            if av.etapa == Etapa::Ciudadela {
                (av.estado, av.obra) = (EstadoAvanzada::EnRuinas { invulnerable_hasta: None }, None);
                self.hecho_de_avanzada(t, "avanzada.sin_dueno", None, av.posicion, vec![("avanzada", id)]);
                self.estado.combate.huerfanas.insert(av.id, av);
            } else {
                self.hecho_de_avanzada(t, "avanzada.destruida", None, av.posicion, vec![("avanzada", id), ("motivo", MOTIVO_RUINA_DE_LA_MADRE)]);
            }
        }
    }

    /// La plaza madre cambia de dueño: sus avanzadas pasan con ella al conquistador. La guarnición de jugadores de los antiguos residentes se va con el desalojo (derrota: a
    /// 0) y la de serie y las elecciones del Gobernador derrotado se reinician.
    pub(super) fn avanzadas_de_la_madre_conquistada(&mut self, madre: u64, t: Instante) {
        let faccion = self.estado.asentamientos[&madre].faccion as i64;
        for av in self.avanzadas_de(madre).iter().map(|v| (v.id, v.posicion)).collect::<Vec<_>>() {
            self.liberar_guarnicion_de_jugadores(av.0, true);
            self.estado.combate.avanzadas.remove(&av.0);
            self.tr.tocar(Clase::Avanzada, av.0);
            self.hecho_de_avanzada(t, "avanzada.traspasada", Some(madre), av.1, vec![("avanzada", av.0 as i64), ("faccion", faccion)]);
        }
    }

    // ───────────────────────────── Atacar ─────────────────────────────

    /// Combate §7.4. Solo un ejército, de otra Facción que la de la madre, a ≤ `radioEncuentro`, contra una avanzada que no sea una obra ni esté en su día de
    /// invulnerabilidad. Se resuelve como un asedio: defienden la guarnición de serie y la de jugadores (dos tiradas); unas ruinas no tienen defensores y caen sin combate
    /// ni tirada. Si gana el atacante, [`Nucleo::cae_avanzada`]; si no, quedan heridos todos los participantes del ejército y la chatarra va al almacén de la madre.
    /// Solo defiende lo que está dentro (decisión del autor, 2026-10-22): una columna amiga a su puerta, en el mapa, no cuenta.
    pub(super) fn atacar_avanzada(&mut self, t: Instante, heroe: u64, id: u64, x: u64, declara_guerra: bool) -> Result<(), Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: &str| Respuesta::rechazo(c, m, v);
        let (madre, av) = self.avanzada(x).ok_or_else(|| rechazo(rechazos::COMBATE_AVANZADA_NO_EXISTE, "no existe esa avanzada"))?;
        let Some(madre) = madre else {
            return Err(rechazo(rechazos::COMBATE_AVANZADA_SIN_DUENO, "unas ruinas de ciudadela sin dueño no tienen Facción con la que estar en guerra ni nada que defender"));
        };
        let (faccion_madre, posicion, etapa, estado, vacia) = (self.estado.asentamientos[&madre].faccion, av.posicion, av.etapa, av.estado, (0..29).all(|i| av.almacen.partes().0[i] == 0));
        let e = &self.estado.ejercitos[&id];
        if !Self::es_ejercito(e) {
            return Err(rechazo(rechazos::COMBATE_SOLO_UN_EJERCITO, "solo un ejército ataca una avanzada: una columna personal no"));
        }
        if e.columna.faccion.is_none() {
            return Err(rechazo(rechazos::COMBATE_SIN_FACCION, "una columna sin Facción no tiene guerra que declarar"));
        }
        match estado {
            EstadoAvanzada::EnConstruccion => return Err(rechazo(rechazos::COMBATE_AVANZADA_EN_OBRA, "la avanzada aún es una obra: se ve en el mapa, pero no se puede atacar")),
            EstadoAvanzada::EnRuinas { invulnerable_hasta: Some(h) } if t < h => return Err(rechazo(rechazos::COMBATE_AVANZADA_INVULNERABLE, "unas ruinas de fuerte son invulnerables un día")),
            _ => {}
        }
        let guerra = self.comprobar_agresion(e, t, posicion, Some(faccion_madre), declara_guerra)?;
        if self.bloqueos().avanzadas.contains(&x) {
            return Err(rechazo(rechazos::BATALLA_BLOQUEADO, "esa avanzada ya está en una batalla"));
        }
        let cr = self.estado.reglas.economia.combate.clone();
        let mut atac = self.tropa_de_columna(id, t, cr.batalla.capacidad_asedio);
        if !atac.escuadras.iter().any(Escuadra::con_soldados) {
            return Err(rechazo(rechazos::COMBATE_SIN_ATACANTES, "sin soldados no hay con quién combatir: un héroe solo no ataca una avanzada"));
        }
        // Unas ruinas de ciudadela ya saqueadas no tienen nada que dar ni nada que derribar: la orden no cambia el mundo.
        if etapa == Etapa::Ciudadela && estado == (EstadoAvanzada::EnRuinas { invulnerable_hasta: None }) && vacia {
            return Err(Respuesta::sin_cambios(v));
        }

        let operativa = estado == EstadoAvanzada::Operativa;
        if operativa {
            self.asegurar_guarnicion(x, etapa, t);
            // Sin nadie en pie esperando la reposición no hay partida que jugar: cae sin combate, como una plaza sin defensores.
            let con_defensa = self.estado.combate.avanzadas[&x].escuadras.iter().any(Escuadra::con_soldados) || self.guarnicion_de_jugadores_de_avanzada(x).iter().any(|s| s.con_soldados());
            if con_defensa && self.estado.reglas.batallas_en_unity {
                let ap = self.apertura_de_avanzada(t, heroe, id, x, madre);
                self.abrir_batalla_de_ataque(t, ap, guerra);
                return Ok(());
            }
        }

        let serie = if operativa { self.estado.combate.avanzadas[&x].escuadras.clone() } else { vec![] };
        let n = serie.len();
        let mut defensa = serie;
        // Defienden además los héroes que están dentro (de la Facción de la madre, aliados o vasallos, con las escuadras de su columna).
        let mut dentro = DefensoresDeDentro::nadie();
        if operativa {
            defensa.extend(self.guarnicion_de_jugadores_de_avanzada(x).into_iter().cloned());
            dentro = self.defensores_de_dentro(Ubicacion::Avanzada(x), faccion_madre, None, cr.batalla.capacidad_asedio, t);
            defensa.extend(dentro.tropa.escuadras.iter().cloned());
        }
        let columnas = [self.columnas_de(&atac.escuadras), self.columnas_de(&dentro.tropa.escuadras)].concat();
        let participantes = Self::participantes_de(&self.estado.ejercitos[&id]);
        let punto = self.estado.ejercitos[&id].posicion_en(t);
        let yo = self.estado.ejercitos[&id].columna.faccion.expect("comprobado");
        if let Some((a, b)) = guerra {
            self.declarar_guerra(t, a, b);
        }
        let resultado = regla::resolver_ataque(&cr, estado, t, Bando { escuadras: &mut atac.escuadras, heroes: &atac.heroes }, Bando { escuadras: &mut defensa, heroes: &dentro.tropa.heroes }, &mut self.estado.rng);
        self.volcar_escuadras(&atac.escuadras);
        self.volcar_escuadras(&defensa[n..]);
        if operativa {
            self.guardar_guarnicion_de_avanzada(x, defensa[..n].to_vec(), t);
        }
        let gana = resultado.conquistada();
        let en_combate = matches!(resultado, Asedio::Combate(_));
        self.defensa_de_vasallo(t, Some(faccion_madre), &dentro.tropa.heroes, !gana);
        let heridos = self.herir_heroes(&heridos_del_asedio(&resultado, &participantes), t);

        let mut datos = vec![("avanzada", x as i64), ("etapa", etapa.indice() as i64), ("faccionAtacante", yo as i64), ("faccionDefensora", faccion_madre as i64)];
        let mut chatarra = Mercancia::new();
        if let Asedio::Combate(i) = &resultado {
            self.experiencia_de_combate(t, i);
            chatarra = i.chatarra.clone();
            datos.extend([("poderAtacante", i.atacante.poder), ("poderDefensor", i.defensor.poder), ("bajasAtacante", caidos(&i.atacante.bajas)), ("bajasDefensor", caidos(&i.defensor.bajas))]);
        }
        // Los residentes de la madre que defendieron en persona y perdieron pierden la mitad del carro (antes de que la avanzada los saque al mapa).
        // Solo si gana el atacante: si gana el defensor no hay derrota que castigar (Combate §7.4).
        let robado = if gana && matches!(resultado, Asedio::Combate(_)) { self.pierden_la_mitad_del_carro(&dentro.tropa.heroes) } else { Mercancia::new() };
        let (codigo, saqueado) = if gana {
            let (nuevo, llevado) = self.cae_avanzada(t, x, &[id]);
            chatarra = self.cargar_botin_en(id, &chatarra);
            self.entregar_lo_robado(id, &robado);
            let codigo = match nuevo {
                EstadoAvanzada::Destruida => "combate.avanzada_destruida",
                _ if nuevo == estado => "combate.avanzada_saqueada",
                _ => "combate.avanzada_en_ruinas",
            };
            (codigo, llevado)
        } else {
            if let Some(a) = self.estado.asentamientos.get_mut(&madre) {
                chatarra.iter().for_each(|(&r, &n)| {
                    a.economia.almacen.agregar(r, n);
                });
            }
            self.tr.tocar(Clase::Asentamiento, madre);
            ("combate.avanzada_resiste", Mercancia::new())
        };
        self.recomponer_tras_bajas(&columnas, t);
        datos.extend([("heridos", heridos as i64), ("chatarra", total(&chatarra)), ("saqueado", total(&saqueado)), ("carroPerdido", total(&robado))]);
        self.hecho_de_combate(codigo, t, vec![id], vec![madre], punto, datos);
        // La participación (Participacion §2), solo con combate: la avanzada no da defensa, solo la batalla ganada.
        if en_combate {
            let ganadores = if gana { participantes.clone() } else { dentro.tropa.heroes.to_vec() };
            self.participacion_de_batalla(t, &[], &[], &ganadores);
        }
        Ok(())
    }

    /// Gana el atacante (con números o por el resultado de Unity): se lleva del almacén lo que le quepa en los carros de `columnas`, y la avanzada pasa a lo que
    /// `combate::avanzada::tras_ataque` dice: la atalaya se destruye; el fuerte queda en ruinas con un día de invulnerabilidad (con su cita) y, si ya estaba en ruinas,
    /// se destruye; la ciudadela queda en ruinas. Al perderse el puesto, la guarnición de jugadores vuelve a 0 a su campamento y la obra en marcha se pierde. Devuelve el estado
    /// nuevo y lo llevado.
    pub(super) fn cae_avanzada(&mut self, t: Instante, x: u64, columnas: &[u64]) -> (EstadoAvanzada, Mercancia) {
        let Some((Some(madre), av)) = self.avanzada(x) else { return (EstadoAvanzada::Destruida, Mercancia::new()) };
        let (etapa, antes, posicion) = (av.etapa, av.estado, av.posicion);
        let cr = self.estado.reglas.economia.combate.clone();
        let columnas: Vec<u64> = columnas.iter().copied().filter(|c| self.estado.ejercitos.contains_key(c)).collect();
        let mut carros: Vec<CarroDeColumna> = columnas.iter().map(|&c| CarroDeColumna { carga: self.estado.ejercitos[&c].columna.carga.clone(), capacidad: self.capacidad_de_carga(c).unwrap_or(0) }).collect();
        let eco = &mut self.estado.asentamientos.get_mut(&madre).expect("existe").economia;
        let llevado = regla::saquear(&mut eco.avanzadas.iter_mut().find(|v| v.id == x).expect("existe").almacen, &mut carros);
        for (&c, carro) in columnas.iter().zip(carros) {
            self.estado.ejercitos.get_mut(&c).expect("existe").columna.carga = carro.carga;
            self.tr.tocar(Clase::Ejercito, c);
        }
        let nuevo = regla::tras_ataque(&cr, etapa, antes, t, true);
        if nuevo != antes {
            // Un puesto destruido o en ruinas no funciona: quien estaba dentro sale al mapa con lo suyo.
            self.vaciar_lugar(t, Ubicacion::Avanzada(x), posicion, madre, if nuevo == EstadoAvanzada::Destruida { MOTIVO_DESTRUIDO } else { MOTIVO_RUINA });
            self.liberar_guarnicion_de_jugadores(x, true);
            match nuevo {
                EstadoAvanzada::Destruida => {
                    self.estado.asentamientos.get_mut(&madre).expect("existe").economia.avanzadas.retain(|v| v.id != x);
                    self.quitar_avanzada(x);
                    self.hecho_de_avanzada(t, "avanzada.destruida", Some(madre), posicion, vec![("avanzada", x as i64), ("motivo", MOTIVO_ATAQUE)]);
                }
                EstadoAvanzada::EnRuinas { invulnerable_hasta } => {
                    let v = self.estado.asentamientos.get_mut(&madre).and_then(|a| a.economia.avanzadas.iter_mut().find(|v| v.id == x)).expect("existe");
                    (v.estado, v.obra) = (nuevo, None);
                    self.perder_obra(x);
                    // Sin guarnición de serie en ruinas: renace completa al repararla. Las tropas elegidas se conservan.
                    if let Some(g) = self.estado.combate.avanzadas.get_mut(&x) {
                        (g.escuadras, g.repone_en) = (vec![], None);
                        if g.elegidas.is_empty() {
                            self.estado.combate.avanzadas.remove(&x);
                        }
                    }
                    if let Some(h) = invulnerable_hasta {
                        let clave = self.estado.generaciones.clave(entidad_avanzada(x));
                        self.estado.cola.programar(h, PRIO_ASENTAMIENTO, clave, Carga::Combate(SucesoCombate::FinInvulnerabilidad));
                    }
                    self.hecho_de_avanzada(t, "avanzada.en_ruinas", Some(madre), posicion, vec![("avanzada", x as i64), ("invulnerableHasta", invulnerable_hasta.unwrap_or(-1))]);
                }
                EstadoAvanzada::EnConstruccion | EstadoAvanzada::Operativa => unreachable!("un ataque ganado solo deja ruinas o nada"),
            }
        }
        self.tr.tocar(Clase::Asentamiento, madre);
        (nuevo, llevado)
    }
}

/// Por qué desapareció una avanzada, como número de un hecho (`motivo`): 1 un ataque, 2 la ruina de la madre (0 sería destruirla a mano).
const MOTIVO_ATAQUE: i64 = 1;
const MOTIVO_RUINA_DE_LA_MADRE: i64 = 2;
