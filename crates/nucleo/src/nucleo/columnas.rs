//! Las columnas dentro del núcleo (Movimiento_Ejercitos, Presencia_Y_Convocatoria, fase 4B-2): una columna es un [`Ejercito`] —su posición y su
//! plan— más la composición y el mando de `militar::columna::Columna`. Este módulo tiene lo común: las consultas, la autorización, el consumo
//! y el hambre, el alta y la baja de columnas y los plazos (deserción, formación, peticiones de unión). Salir y entrar en un lugar están en
//! [`salidas`](super::salidas); unir, separar, formar, repostar y las caravanas adjuntas, en [`uniones`](super::uniones); las convocatorias, en
//! [`convocatorias`](super::convocatorias).
//!
//! # Comer sin coste por columna ociosa
//! El consumo de una columna es un **ritmo** sobre sus víveres (`Columna::viveres`) que se vuelve a anotar cada vez que cambia lo que lo decide:
//! que marche o acampe, quién va dentro, cuánta tropa tiene en pie (`reanclar_consumo`). Del ritmo salen solos los dos hechos de juego que
//! tienen instante exacto: «víveres a 0» (`Carga::Umbral`) y el hambre de la tropa. La **moral** de cada escuadra es otro ritmo (sube con
//! ración, baja sin ella, `Escuadra::con_racion`) y se anota cuando los víveres cruzan el 0 (`sincronizar_hambre`); a 0 la escuadra deserta una
//! vez y otra cada minuto mientras siga sin ración, cada una en la fase en que su moral llegó a 0 (`Escuadra::desercion_a_partir_de`). Un único
//! evento por columna, `SucesoMilitar::Desercion`, apunta a la siguiente deserción de cualquiera de sus escuadras; solo existe mientras la columna
//! tiene hambre y soldados, y se descarta solo al subir la generación de la columna (cualquier cambio de ritmo la sube). Una columna que come, o que
//! no tiene soldados, o que está quieta con víveres, no programa nada más que su llegada y sus dos umbrales.
//!
//! # Quién manda
//! La cuenta (el cliente) de una columna es la de su Líder, que es un héroe; la administración manda en todas. Las órdenes de movimiento
//! (`MarcharA`, `Estacionar`) son del Líder y de que vaya a bordo (un héroe dentro de un lugar no dirige su columna, que está dentro con él); las de «dentro de un
//! ejército» (caravanas adjuntas, entregar, materiales) las puede dar cualquier participante.

use super::*;
use crate::estado::{PlanEnPausa, SucesoMilitar, PRIO_LLEGADA, PRIO_SALIDA, PRIO_UMBRAL};
use super::viaje::pausar_plan;
use crate::estado_militar::{entidad_formacion, entidad_peticion_de_union};
use comercio::caravana::TipoCaravana;
use militar::columna::{self, Columna, ErrorColumna, TipoColumna};
use militar::convocatoria::Lugar;
use militar::escuadra::{Contenedor, Escuadra};
use militar::presencia::Ubicacion;
use militar::racion;
use militar::racion::Estado as EstadoMarcha;

/// Los códigos de rechazo de las reglas de una columna.
pub(super) fn rechazo_de_columna(e: ErrorColumna) -> (&'static str, &'static str) {
    use rechazos::*;
    match e {
        ErrorColumna::YaVasEnEsaColumna => (COLUMNA_YA_VAS_EN_ELLA, "ya vas en esa columna"),
        ErrorColumna::NoAdmite => (COLUMNA_NO_ADMITE, "esa columna no admite a nadie más"),
        ErrorColumna::NoHayEjercito => (COLUMNA_NO_ES_EJERCITO, "solo se une a un ejército o a una formación"),
        ErrorColumna::SoloUnaPersonal => (COLUMNA_SOLO_UNA_PERSONAL, "solo se une una columna personal sin formación"),
        ErrorColumna::OtraFaccion => (COLUMNA_OTRA_FACCION, "un ejército lo componen ciudadanos de una sola Facción"),
        ErrorColumna::Lejos => (COLUMNA_LEJOS, "hay que estar junto a la columna"),
        ErrorColumna::NoPregunta => (COLUMNA_NO_PREGUNTA, "la política de esa columna no es preguntar: no hay a quién pedir permiso"),
        ErrorColumna::NoVaDentro => (COLUMNA_NO_VA_DENTRO, "no vas en esa columna"),
        ErrorColumna::DeUnaPersonalNoTeSeparas => (COLUMNA_DE_UNA_PERSONAL_NO_TE_SEPARAS, "de una columna personal no te separas: es tuya"),
        ErrorColumna::ElLiderNoPuede => (COLUMNA_EL_LIDER_NO_PUEDE, "el Líder no se separa: cede antes el mando"),
        ErrorColumna::EsElUltimo => (COLUMNA_ES_EL_ULTIMO, "el último no se separa: la columna no se vacía en campo abierto"),
        ErrorColumna::SoloElLider => (COLUMNA_SOLO_EL_LIDER, "solo el Líder puede hacerlo"),
        ErrorColumna::YaEresElLider => (COLUMNA_YA_ERES_EL_LIDER, "ya eres el Líder"),
        ErrorColumna::SucesorFuera => (COLUMNA_SUCESOR_FUERA, "el sucesor tiene que ir dentro de la columna"),
        ErrorColumna::NoHayPeticion => (COLUMNA_NO_HAY_PETICION, "no hay una petición de ese héroe"),
        ErrorColumna::PeticionCaducada => (COLUMNA_PETICION_CADUCADA, "la petición caducó: el silencio vale como un no"),
        ErrorColumna::YaEsEjercito => (COLUMNA_YA_ES_EJERCITO, "un ejército no organiza otro"),
        ErrorColumna::YaOrganizando => (COLUMNA_YA_ORGANIZANDO, "ya se está organizando un ejército"),
        ErrorColumna::NoEstaSolo => (COLUMNA_NO_ESTA_SOLO, "solo se organiza un ejército yendo solo"),
        ErrorColumna::PoliticaInvalida => (COLUMNA_POLITICA_INVALIDA, "la política de una formación es aceptar o preguntar"),
        ErrorColumna::NoEsUnaFormacion => (COLUMNA_NO_ES_UNA_FORMACION, "esa columna no es una formación"),
    }
}

impl Nucleo {
    // ───────────────────────────── Consultas ─────────────────────────────

    /// Una columna por id.
    pub fn ejercito(&self, id: u64) -> Option<&Ejercito> {
        self.estado.ejercitos.get(&id)
    }

    /// La columna de un héroe: aquella en la que va o con la que entró de visita en un lugar ajeno (dentro, fuera del mapa).
    pub fn columna_del_heroe(&self, heroe: u64) -> Option<&Ejercito> {
        self.estado.ejercitos.get(&self.estado.ejercito_de_heroe(heroe)?)
    }

    /// Las escuadras que lleva la columna, **en el orden en que entran en combate** (el de sus ids).
    pub fn escuadras_de_columna(&self, id: u64) -> Vec<&Escuadra> {
        let Some(e) = self.estado.ejercitos.get(&id) else { return vec![] };
        e.columna.escuadras.iter().filter_map(|&s| self.escuadra(s).map(|(_, esc)| esc)).collect()
    }

    /// Soldados en pie que lleva la columna (los que comen y los que la hacen ver más lejos).
    pub fn soldados_de_columna(&self, id: u64) -> Option<i64> {
        let e = self.estado.ejercitos.get(&id)?;
        Some(racion::soldados(e.columna.escuadras.iter().filter_map(|&s| self.escuadra(s).map(|(_, esc)| esc))))
    }

    /// Trigo por minuto que come la columna ahora (milésimas), derivado de su composición y de si marcha.
    pub fn consumo_de_columna(&self, id: u64) -> Option<Mil> {
        Some(-self.estado.ejercitos.get(&id)?.columna.viveres.por_minuto)
    }

    /// Alcance de vista de la columna: 30 si lleva soldados en pie, 15 si no.
    pub fn alcance_de_vista(&self, id: u64) -> Option<Mil> {
        let e = self.estado.ejercitos.get(&id)?;
        Some(columna::alcance_de_vista(&self.estado.reglas.economia.militar, self.escuadras_de_columna(e.id)))
    }

    /// Velocidad de la columna, en milésimas de unidad por segundo (30 unidades por minuto = 500): la fija de las columnas de administración o, si no, la
    /// derivada de su composición (la de su escuadra con soldados más lenta o la de su caravana adjunta más lenta; un héroe solo, el viajero).
    pub fn velocidad_de_ejercito(&self, id: u64) -> Option<Mil> {
        Some(self.velocidad_de_columna(self.estado.ejercitos.get(&id)?))
    }

    /// Una convocatoria por id.
    pub fn convocatoria(&self, id: u64) -> Option<&militar::convocatoria::Convocatoria> {
        self.estado.militar.convocatorias.get(&id)
    }

    /// Las convocatorias que se preparan en un lugar, por id.
    pub fn convocatorias_del_lugar(&self, lugar: Lugar) -> Vec<&militar::convocatoria::Convocatoria> {
        self.estado.militar.convocatorias.values().filter(|c| c.lugar == lugar).collect()
    }

    // ───────────────────────────── Ayudas ─────────────────────────────

    pub(super) fn nuevo_id(&mut self) -> u64 {
        self.estado.siguiente_id += 1;
        self.estado.siguiente_id - 1
    }

    fn cuenta_de(&self, heroe: u64) -> Option<&str> {
        self.heroe(heroe).map(|h| h.cuenta.as_str())
    }

    /// ¿Manda `p` en la columna? La administración, siempre; si no, la cuenta de su Líder (`solo_lider`) o la de cualquiera de sus participantes.
    pub(super) fn manda_columna(&self, p: &Peticion, e: &Ejercito, solo_lider: bool) -> bool {
        p.admin || if solo_lider { self.cuenta_de(e.columna.lider) == Some(p.actor.as_str()) } else { e.columna.participantes.iter().any(|x| self.cuenta_de(x.heroe) == Some(p.actor.as_str())) }
    }

    /// Quién puede mover la columna (`marcharA`, `estacionarEjercito`): su Líder, que además tiene que ir a bordo (un héroe dentro de un lugar no
    /// dirige su columna, que está dentro con él), y nunca una formación, que no se mueve. La administración puede con todas las demás.
    pub(super) fn autorizar_movimiento(&self, p: &Peticion, e: &Ejercito) -> Result<(), Respuesta> {
        let v = self.estado.version;
        if !self.manda_columna(p, e, true) {
            return Err(Respuesta::rechazo(rechazos::NO_AUTORIZADO, "la columna no es tuya", v));
        }
        if e.columna.es_formacion() {
            return Err(Respuesta::rechazo(rechazos::COLUMNA_EN_FORMACION, "una formación no se mueve: espera a ser un ejército", v));
        }
        if e.en_batalla {
            return Err(Respuesta::rechazo(rechazos::COLUMNA_EN_BATALLA, "la columna está apartada por una batalla: no se la puede tocar", v));
        }
        if !p.admin {
            let lider = self.heroe(e.columna.lider).map(|h| &h.heroe.presencia);
            if lider.is_some_and(|l| l.fuera) {
                return Err(Respuesta::rechazo(rechazos::HEROE_FUERA_DEL_MUNDO, "fuera del mundo solo se puede conectar", v));
            }
            if lider.is_none_or(|l| l.ubicacion != Ubicacion::Columna(e.id)) {
                return Err(Respuesta::rechazo(rechazos::HEROE_DENTRO_DE_UN_LUGAR, "dentro de un lugar no se dirige la columna: hay que salir antes", v));
            }
        }
        Ok(())
    }

    /// Los héroes que van en la columna, por antigüedad de entrada.
    pub(super) fn participantes_de(e: &Ejercito) -> Vec<u64> {
        e.columna.participantes.iter().map(|p| p.heroe).collect()
    }

    /// Las escuadras de un héroe que van en la columna `columna` (en el orden del héroe).
    pub(super) fn escuadras_de_heroe_en(&self, heroe: u64, columna: u64) -> Vec<u64> {
        self.heroe(heroe).map_or(vec![], |h| h.heroe.escuadras.iter().filter(|e| e.contenedor == Contenedor::Ejercito(columna)).map(|e| e.id).collect())
    }

    /// Pone en `contenedor` las escuadras `ids` de los héroes `heroes`, y su moral a lo que toque: la de una columna la decide su hambre
    /// (`sincronizar_hambre`); fuera de una columna queda quieta.
    pub(super) fn poner_escuadras_en(&mut self, heroes: &[u64], ids: &[u64], contenedor: Contenedor, t: Instante) {
        for h in heroes {
            self.tr.tocar(Clase::Heroe, *h);
            let Some(h) = self.estado.militar.heroes.get_mut(h) else { continue };
            for e in h.heroe.escuadras.iter_mut().filter(|e| ids.contains(&e.id)) {
                e.contenedor = contenedor;
                if !matches!(contenedor, Contenedor::Ejercito(_)) {
                    e.moral_quieta(t);
                }
            }
        }
    }

    /// Lo que decide el consumo de una columna ha cambiado (quién va, cuánta tropa en pie): se vuelve a anotar desde `t` sin saltos.
    pub(super) fn reanclar_consumo(&mut self, id: u64, t: Instante) {
        let Estado { ejercitos, militar, reglas, .. } = &mut self.estado;
        let Some(e) = ejercitos.get_mut(&id) else { return };
        let heroes = Self::participantes_de(e);
        e.columna.reanclar_consumo(&reglas.economia.militar, t, &|ids| militar.soldados_de(&heroes, ids));
    }

    /// La columna marcha o acampa desde `t`.
    pub(super) fn poner_marcha(&mut self, id: u64, t: Instante, estado: EstadoMarcha) {
        let Estado { ejercitos, militar, reglas, .. } = &mut self.estado;
        let Some(e) = ejercitos.get_mut(&id) else { return };
        let heroes = Self::participantes_de(e);
        e.columna.poner_estado(&reglas.economia.militar, t, estado, &|ids| militar.soldados_de(&heroes, ids));
    }

    /// Anota en cada escuadra de la columna si come (su moral sube) o pasa hambre (baja), según tenga o no víveres en `t`. No toca nada si ya estaba así.
    pub(super) fn sincronizar_hambre(&mut self, id: u64, t: Instante) {
        let Estado { ejercitos, militar, reglas, .. } = &mut self.estado;
        let Some(e) = ejercitos.get(&id) else { return };
        let come = e.columna.viveres.valor_en(t) > 0;
        for p in &e.columna.participantes {
            let Some(h) = militar.heroes.get_mut(&p.heroe) else { continue };
            self.tr.tocar(Clase::Heroe, p.heroe);
            for esc in h.heroe.escuadras.iter_mut().filter(|s| s.contenedor == Contenedor::Ejercito(id)) {
                esc.con_racion(&reglas.economia.militar, t, come);
            }
        }
    }

    /// Cuándo desertará la próxima escuadra de la columna sin víveres, si alguna lo hará: siempre después de `t` (las citas de `t` ya se atendieron, porque
    /// una orden se aplica con los eventos hasta `t` ya procesados).
    fn proxima_desercion(&self, e: &Ejercito, t: Instante) -> Option<Instante> {
        if e.columna.viveres.valor_en(t) > 0 {
            return None;
        }
        e.columna.participantes.iter().filter_map(|p| self.heroe(p.heroe)).flat_map(|h| h.heroe.escuadras.iter()).filter(|s| s.contenedor == Contenedor::Ejercito(e.id)).filter_map(|s| s.desercion_a_partir_de(t + 1)).min()
    }

    /// Programa el siguiente hambre de la columna (su deserción), con la generación vigente. Lo llama `programar_ejercito`.
    pub(super) fn programar_desercion(&mut self, id: u64, t: Instante) {
        let Some(e) = self.estado.ejercitos.get(&id) else { return };
        if let Some(d) = self.proxima_desercion(e, t) {
            let clave = self.estado.generaciones.clave(id);
            self.estado.cola.programar(d.max(t), PRIO_UMBRAL, clave, Carga::Militar(SucesoMilitar::Desercion));
        }
    }

    /// Un hecho de la columna: lo oye quien oye a la columna y, si se dan, las plazas a las que se atribuye.
    pub(super) fn evento_de_columna(&mut self, codigo: &'static str, t: Instante, id: u64, plazas: Vec<u64>, datos: Vec<(&'static str, i64)>) {
        let punto = self.estado.ejercitos.get(&id).map(|e| e.posicion_en(t));
        self.tr.eventos.push(EventoNucleo { codigo: Cow::Borrowed(codigo), en: t, ejercitos: vec![id], asentamientos: plazas, punto, datos });
    }

    // ───────────────────────────── Alta y baja ─────────────────────────────

    /// Da de alta una columna ya compuesta: la inserta, anota en cada participante en qué columna va, pone sus escuadras dentro y deja programados
    /// sus umbrales de víveres, su hambre y su llegada. Los héroes siguen donde estaban (su ubicación la fija quien llama).
    pub(super) fn alta_de_columna(&mut self, t: Instante, e: Ejercito) {
        let (id, heroes, escuadras) = (e.id, Self::participantes_de(&e), e.columna.escuadras.clone());
        for h in &heroes {
            if let Some(h) = self.estado.militar.heroes.get_mut(h) {
                h.columna = Some(id);
                self.tr.tocar(Clase::Heroe, h.heroe.id);
            }
        }
        self.estado.ejercitos.insert(id, e);
        self.poner_escuadras_en(&heroes, &escuadras, Contenedor::Ejercito(id), t);
        self.tr.tocar(Clase::Ejercito, id);
        // Nace con la generación de su id (sin subirla): aún no hay nada programado que invalidar.
        self.sincronizar_hambre(id, t);
        self.programar_ejercito(id, t);
        self.anunciar_columna(id, Motivo::Plan);
    }

    /// Quita la columna del mundo: sus eventos se descartan, el espacio la olvida y quien la perseguía la da por puesta a cubierto. Los participantes,
    /// sus escuadras y las caravanas son de quien llama.
    pub(super) fn baja_de_columna(&mut self, id: u64) {
        self.fijar_persecucion(id, None);
        self.estado.ejercitos.remove(&id);
        self.estado.generaciones.subir(id);
        self.estado.generaciones.subir(entidad_formacion(id));
        self.tr.tocar(Clase::Ejercito, id);
        self.avisar_plan(id, false);
        self.anunciar_columna(id, Motivo::Plan);
    }

    /// Mete a un héroe en una columna como participante nuevo con sus escuadras. Los héroes siguen donde estaban (la ubicación la fija quien llama).
    /// La columna con alguien a bordo entra en el espacio si no estaba.
    pub(super) fn ubicar_en_columna(&mut self, heroe: u64, columna: u64) {
        if let Some(h) = self.estado.militar.heroes.get_mut(&heroe) {
            self.tr.tocar(Clase::Heroe, heroe);
            h.columna = Some(columna);
            h.heroe.presencia.ubicacion = Ubicacion::Columna(columna);
        }
        self.anunciar_columna(columna, Motivo::Ritmo);
    }

    /// Una columna nueva nacida de repartir otra (separarse, desarmar, salir del mundo): quieta donde está su origen, sin caravanas.
    pub(super) fn columna_suelta(&self, columna: Columna, posicion: Punto) -> Ejercito {
        Ejercito { id: columna.id, columna, posicion, plan: Plan::default(), velocidad_fija: None, caravana_fundacion: false, caravanas_adjuntas: vec![], persiguiendo: None, en_batalla: false, plan_en_pausa: None, version: 0 }
    }

    /// Una columna sin nadie dentro se disuelve (Movimiento_Ejercitos §11): sus caravanas adjuntas, que sí viajan solas, vuelven a su origen
    /// (o se pierden si no hay camino por tierra). No hay tropa que devolver: las escuadras son de sus héroes.
    pub(super) fn disolver_columna(&mut self, t: Instante, id: u64) {
        let Some(e) = self.estado.ejercitos.get(&id) else { return };
        let (posicion, adjuntas) = (e.posicion_en(t), e.caravanas_adjuntas.clone());
        self.baja_de_columna(id);
        for c in adjuntas {
            let Some(caravana) = self.estado.comercio.caravana(c) else { continue };
            let origen = caravana.origen as u64;
            match (caravana.tipo == TipoCaravana::Comercial).then(|| self.plan_de_regreso(c, posicion, t)).flatten() {
                Some((plan, camino)) => {
                    let i = self.estado.comercio.posicion_caravana(c).expect("existe");
                    let k = &mut self.estado.comercio.flota[i];
                    if k.volver_por_su_cuenta(plan.clone()).is_err() {
                        continue;
                    }
                    (k.ruta, k.costes, k.posicion) = (camino.puntos, camino.costes, posicion);
                    self.instalar_plan(c, plan);
                    self.emitir_comercio(t, comercio::evento::Evento::CaravanaVuelve { caravana: c }, vec![origen], &[("sinNadie", 1)]);
                }
                None => self.perder_caravana(t, c),
            }
        }
        self.evento("ejercito.disuelto", t, id, Some(posicion), vec![]);
    }

    // ───────────────────────────── Los plazos de las columnas ─────────────────────────────

    /// Vence un plazo militar. La clave de la cola ya garantiza que la entidad no cambió de ritmo ni de formación.
    pub(super) fn suceso_militar(&mut self, entidad: u64, suceso: SucesoMilitar, t: Instante) {
        match suceso {
            SucesoMilitar::Desercion => self.desercion_de_columna(entidad, t),
            SucesoMilitar::CaducaFormacion => self.caduca_formacion(crate::estado_comercio::id_de_entidad(entidad), t),
            SucesoMilitar::CaducaPeticion { columna } => self.caduca_peticion_de_union(columna, crate::estado_comercio::id_de_entidad(entidad), t),
            SucesoMilitar::CaducaPeticionDeConvocatoria { convocatoria } => self.caduca_peticion_de_convocatoria(convocatoria, crate::estado_comercio::id_de_entidad(entidad), t),
            SucesoMilitar::PresaFueraDeVista { presa, presa_generacion } => self.presa_fuera_de_vista(entidad, presa, presa_generacion, t),
        }
    }

    /// Les toca desertar a las escuadras de la columna cuya fase cae ahora: `ceil(5 %)` de sus soldados cada una, atribuido a la plaza donde se
    /// reclutaron. Cambia el consumo y reprograma la siguiente.
    fn desercion_de_columna(&mut self, id: u64, t: Instante) {
        let mr = self.estado.reglas.economia.militar.clone();
        let Some(e) = self.estado.ejercitos.get(&id) else { return };
        let heroes = Self::participantes_de(e);
        let mut bajas = Vec::new();
        for h in heroes {
            let Some(hn) = self.estado.militar.heroes.get_mut(&h) else { continue };
            self.tr.tocar(Clase::Heroe, h);
            for s in hn.heroe.escuadras.iter_mut().filter(|s| s.contenedor == Contenedor::Ejercito(id) && s.deserta_en(t)) {
                let n = s.desertar(&mr, t);
                if n > 0 {
                    bajas.push((s.plaza_origen.map(u64::from), h, s.id, n));
                }
            }
        }
        self.reanclar_consumo(id, t);
        self.reprogramar_columna(id, t);
        for (plaza, heroe, escuadra, desertores) in bajas {
            self.evento_de_columna("tropas.desercion", t, id, plaza.into_iter().collect(), vec![("heroe", heroe as i64), ("escuadra", escuadra as i64), ("desertores", desertores)]);
        }
    }

    /// Venció el plazo de la formación sin llegar al mínimo: se deshace (cada uno vuelve a su columna personal con lo suyo).
    fn caduca_formacion(&mut self, id: u64, t: Instante) {
        if !self.estado.ejercitos.get(&id).is_some_and(|e| e.columna.formacion_vencida(t)) {
            return;
        }
        self.deshacer_formacion(id, t);
    }

    /// Caduca la petición de unión en campo que `solicitante` le hizo a la columna `columna`: se retira y se avisa a las dos partes.
    fn caduca_peticion_de_union(&mut self, columna: u64, solicitante: u64, t: Instante) {
        let Some(e) = self.estado.ejercitos.get_mut(&columna) else { return };
        let antes = e.columna.peticiones.len();
        e.columna.peticiones.retain(|p| !(p.heroe == solicitante && t >= p.expira_en));
        if e.columna.peticiones.len() == antes {
            return;
        }
        self.tr.tocar(Clase::Ejercito, columna);
        self.evento_de_columna("columna.peticion_caducada", t, columna, vec![], vec![("solicitante", solicitante as i64)]);
    }

    /// Programa la caducidad de una petición de unión en campo a su `expira_en`.
    pub(super) fn programar_caducidad_de_peticion(&mut self, columna: u64, solicitante: u64, expira_en: Instante) {
        let clave = self.estado.generaciones.clave(entidad_peticion_de_union(solicitante));
        self.estado.cola.programar(expira_en, PRIO_LLEGADA, clave, Carga::Militar(SucesoMilitar::CaducaPeticion { columna }));
    }

    // ───────────────────────────── Salir del mundo y volver (Reloj_Y_Eventos §7) ─────────────────────────────

    /// El héroe sale del mundo. Dentro de una plaza o de un campamento no se mueve nada (solo pierde el control). Con su columna en el mapa se lleva
    /// sus escuadras (`fuera`), su parte del carro y sus víveres, y reaparecerá en su punto: si la columna es un ejército con más gente se separa con lo
    /// suyo y el mando pasa al más antiguo de los conectados; si iba solo, la columna deja de existir y sus caravanas adjuntas vuelven solas a su origen.
    pub(super) fn sale_del_mundo(&mut self, id: u64, t: Instante) {
        let mr = self.estado.reglas.economia.militar.clone();
        let residencia = self.heroe(id).and_then(|h| match h.heroe.residencia {
            Some(militar::heroe::Residencia::Asentamiento(a)) => Some(a as u64),
            _ => None,
        });
        let Some(Ubicacion::Columna(c)) = self.heroe(id).map(|h| h.heroe.presencia.ubicacion) else {
            self.heroe_mut(id).heroe.presencia.salir_del_mundo(Punto::default());
            self.emitir_propio(t, "heroe.sale_del_mundo", residencia.into_iter().collect(), vec![("heroe", id as i64)]);
            return;
        };
        let e = &self.estado.ejercitos[&c];
        let (posicion, suyas, antes) = (e.posicion_en(t), self.escuadras_de_heroe_en(id, c), self.velocidad_de_columna(e));
        let solo = e.columna.participantes.len() <= 1;
        let (carro, viveres) = if !solo {
            // El mando pasa al conectado de más antigüedad antes de irse; si no hay ninguno conectado, al de más antigüedad.
            let conectado = |h: u64| self.heroe(h).is_some_and(|x| !x.heroe.presencia.fuera && x.heroe.presencia.desconecta_en.is_none());
            let sucesor = e.columna.sucesor(id, &conectado).or_else(|| e.columna.sucesor(id, &|_| true));
            let Estado { ejercitos, militar, .. } = &mut self.estado;
            let e = ejercitos.get_mut(&c).expect("existe");
            let lider_antes = e.columna.lider;
            if let Some(s) = sucesor.filter(|_| lider_antes == id) {
                e.columna.lider = s;
            }
            let heroes = Self::participantes_de(e);
            let suya = e.columna.desgajar(&mr, id, &suyas, 0, t, &|ids| militar.soldados_de(&heroes, ids)).expect("va dentro");
            let nuevo_lider = e.columna.lider;
            if lider_antes == id {
                self.evento_de_columna("columna.liderazgo_cedido", t, c, vec![], vec![("anterior", id as i64), ("sucesor", nuevo_lider as i64)]);
            }
            (suya.carga, suya.viveres.valor_en(t))
        } else {
            (e.columna.carga.clone(), e.columna.viveres.valor_en(t))
        };
                self.poner_escuadras_en(&[id], &suyas, Contenedor::Fuera, t);
        let cap = mr.logistica.capacidad_viveres;
        let hn = self.heroe_mut(id);
        hn.carro = carro;
        hn.heroe.viveres = viveres.min(cap);
        hn.columna = None;
        hn.heroe.presencia.salir_del_mundo(posicion);
        if solo {
            // La columna se queda sin nadie: nada que devolver (la tropa es del héroe, que se la lleva) y las caravanas vuelven solas.
            self.disolver_columna(t, c);
        } else {
            self.recomponer_columna(c, t, antes);
        }
        self.emitir_propio(t, "heroe.sale_del_mundo", residencia.into_iter().collect(), vec![("heroe", id as i64), ("columna", c as i64)]);
    }

    /// El héroe vuelve al mundo (`conectarse` estando fuera). Dentro de un lugar solo recupera el control; si salió con su columna reaparece en su
    /// punto con una columna personal nueva, quieta, con sus escuadras, su carro y sus víveres. Devuelve el id de la columna nueva, si la hay.
    pub(super) fn volver_al_mundo(&mut self, t: Instante, id: u64) -> Vec<u64> {
        let mr = self.estado.reglas.economia.militar.clone();
        let hn = self.heroe(id).expect("existe");
        let residencia = match hn.heroe.residencia {
            Some(militar::heroe::Residencia::Asentamiento(a)) => Some(a as u64),
            _ => None,
        };
        let Ubicacion::Desconectado(punto) = hn.heroe.presencia.ubicacion else {
            self.emitir_propio(t, "heroe.vuelve_al_mundo", residencia.into_iter().collect(), vec![("heroe", id as i64)]);
            return vec![];
        };
        let escuadras: Vec<u64> = hn.heroe.escuadras.iter().filter(|e| e.contenedor == Contenedor::Fuera).map(|e| e.id).collect();
        let (faccion, carro, viveres) = (hn.faccion, hn.carro.clone(), hn.heroe.viveres);
        let columna_id = self.nuevo_id();
        let heroes = [id];
        let militar = &self.estado.militar;
        let columna = Columna::salir(&mr, columna_id, faccion, id, TipoColumna::Personal, militar::columna::Politica::Rechazar, escuadras, carro, viveres, t, &|ids| militar.soldados_de(&heroes, ids));
        let hn = self.heroe_mut(id);
        hn.carro.clear();
        hn.heroe.viveres = 0;
        let e = self.columna_suelta(columna, punto);
        self.alta_de_columna(t, e);
        self.ubicar_en_columna(id, columna_id);
        self.emitir_propio(t, "heroe.vuelve_al_mundo", residencia.into_iter().collect(), vec![("heroe", id as i64), ("columna", columna_id as i64)]);
        vec![columna_id]
    }

    // ───────────────────────────── Batalla (hook del sistema militar) ─────────────────────────────

    /// La batalla de Unity aparta la columna (Reloj_Y_Eventos §6): se queda donde está, sale del espacio —no tiene encuentros, nadie la persigue ni ella
    /// persigue—, no come y no admite órdenes de movimiento hasta que acabe. El plan que llevaba queda en pausa y lo retoma al salir. `false` si no existe
    /// o ya estaba apartada. Procesa los eventos hasta `t` y cierra la versión. Lo llama el ciclo de la batalla (`Nucleo::abrir_batalla`) y las pruebas;
    /// no deja entrada en el registro de peticiones.
    pub fn columna_entra_en_batalla(&mut self, t: Instante, id: u64) -> bool {
        let t = t.max(self.estado.ahora);
        self.procesar_hasta(t);
        let entra = self.apartar_columna(t, id);
        self.cerrar_version(t, entra);
        entra
    }

    /// La batalla acabó: la columna vuelve al espacio donde está y retoma el plan que llevaba, desplazado lo que duró la batalla (si las bajas le han
    /// cambiado la velocidad, el resto del plan se reescala); quieta si no marchaba. Quien pidió salir del mundo mientras combatía lo reintenta ya. `false`
    /// si no existe o no estaba apartada.
    pub fn columna_sale_de_batalla(&mut self, t: Instante, id: u64) -> bool {
        let t = t.max(self.estado.ahora);
        self.procesar_hasta(t);
        let sale = self.reponer_columna(t, id);
        self.cerrar_version(t, sale);
        sale
    }

    /// `columna_entra_en_batalla` dentro de una transición ya abierta (no procesa eventos ni cierra la versión).
    pub(super) fn apartar_columna(&mut self, t: Instante, id: u64) -> bool {
        let Some(e) = self.estado.ejercitos.get(&id) else { return false };
        if e.en_batalla {
            return false;
        }
        let en_marcha = e.marchando().then(|| (e.plan.clone(), self.velocidad_de_columna(e)));
        if en_marcha.is_some() {
            self.detener(id, t);
        }
        let e = self.estado.ejercitos.get_mut(&id).expect("existe");
        e.en_batalla = true;
        e.plan_en_pausa = en_marcha.map(|(plan, velocidad)| PlanEnPausa { desde: t, velocidad, plan });
        let punto = e.posicion;
        self.poner_marcha(id, t, EstadoMarcha::Apartada);
        self.reprogramar_columna(id, t);
        self.evento("ejercito.en_batalla", t, id, Some(punto), vec![]);
        true
    }

    /// `columna_sale_de_batalla` dentro de una transición ya abierta.
    pub(super) fn reponer_columna(&mut self, t: Instante, id: u64) -> bool {
        let Some(e) = self.estado.ejercitos.get_mut(&id).filter(|e| e.en_batalla) else { return false };
        e.en_batalla = false;
        let (punto, heroes, pausa) = (e.posicion, Self::participantes_de(e), e.plan_en_pausa.take());
        self.tr.tocar(Clase::Ejercito, id);
        match pausa {
            Some(PlanEnPausa { desde, velocidad, plan }) => {
                self.estado.ejercitos.get_mut(&id).expect("existe").plan = pausar_plan(&plan, desde, t, punto);
                self.poner_marcha(id, t, EstadoMarcha::Marchando);
                // Con las bajas la columna puede marchar a otra velocidad: lo que le queda de plan se escala.
                let despues = self.velocidad_de_columna(&self.estado.ejercitos[&id]);
                if !self.reescalar_marcha(id, t, velocidad, despues) {
                    self.reprogramar_columna(id, t);
                }
            }
            None => {
                self.poner_marcha(id, t, EstadoMarcha::Estacionada);
                self.reprogramar_columna(id, t);
            }
        }
        let pendientes: Vec<u64> = heroes.into_iter().filter(|&h| self.heroe(h).is_some_and(|x| x.heroe.presencia.desconecta_en.is_some() && !x.heroe.presencia.fuera)).collect();
        for h in pendientes {
            let clave = self.estado.generaciones.clave(h);
            self.estado.cola.programar(t, PRIO_SALIDA, clave, Carga::SalidaDelMundo);
        }
        self.evento("ejercito.sale_de_batalla", t, id, Some(punto), vec![]);
        true
    }

    // ───────────────────────────── Velocidad y capacidad ─────────────────────────────

    /// La velocidad de la columna en `t`, en milésimas por segundo (§4.1): la fija de una columna de administración o la derivada de su composición;
    /// las caravanas adjuntas entran en el mínimo de las dos.
    pub(super) fn velocidad_de_columna(&self, e: &Ejercito) -> Mil {
        let flota = &self.estado.reglas.economia.comercio.flota;
        let caravanas: Vec<Mil> = e.caravanas_adjuntas.iter().filter_map(|&c| self.estado.comercio.caravana(c)).map(|c| c.velocidad(flota)).filter(|&v| v > 0).collect();
        let por_minuto = match e.velocidad_fija {
            Some(v) => caravanas.into_iter().chain(std::iter::once(v * 60)).min().unwrap_or(0),
            None => columna::velocidad(&self.estado.reglas.economia.militar, self.escuadras_en(e), caravanas, e.columna.participantes.len()),
        };
        por_minuto / 60
    }

    /// Las escuadras que van en la columna (las de sus participantes con ese contenedor).
    pub(super) fn escuadras_en<'a>(&'a self, e: &'a Ejercito) -> impl Iterator<Item = &'a Escuadra> + 'a {
        e.columna.participantes.iter().filter_map(|p| self.heroe(p.heroe)).flat_map(|h| h.heroe.escuadras.iter()).filter(|s| s.contenedor == Contenedor::Ejercito(e.id))
    }

    /// Cambió quién va en la columna o qué tropa tiene en pie: se vuelve a anotar el consumo y, si marcha, se reescala el plan a la velocidad nueva.
    /// `antes` es la velocidad (por segundo) que tenía antes del cambio.
    pub(super) fn recomponer_columna(&mut self, id: u64, t: Instante, antes: Mil) {
        self.reanclar_consumo(id, t);
        let despues = self.estado.ejercitos.get(&id).map_or(0, |e| self.velocidad_de_columna(e));
        if !self.reescalar_marcha(id, t, antes, despues) {
            self.reprogramar_columna(id, t);
        }
    }

    /// ¿Es la columna un ejército de verdad (no una formación)?
    pub(super) fn es_ejercito(e: &Ejercito) -> bool {
        e.columna.tipo == TipoColumna::Ejercito && !e.columna.es_formacion()
    }
}
