//! La Facción como entidad (Diplomacia_Facciones §2.1, §3.1-§3.4; Doc 2.2 y 2.5): crearla, pedir el ingreso y responderlo, dejarla con la
//! **sucesión del trono por orden de ingreso**, designar Rey y Embajador, el ajuste de admitir a otras en los ataques, la experiencia y la derrota.
//!
//! «1 héroe, 1 Facción»: [`Facciones`] lleva un índice héroe → Facción que mantienen sus métodos (se reconstruye con [`Facciones::desde`] al leer una
//! foto). Quién puede dar cada orden es la autorización (`autorizacion`); aquí solo qué transición es válida.

use crate::datos::Reglas;
use crate::nivel;
use crate::sigilo::{MotivoSigilo, Sigilo};
use crate::tipos::{FaccionId, HeroeId};
use base::{Instante, Mil};
use std::collections::BTreeMap;

/// Quién derrotó a una Facción que se quedó sin plazas (§8). Solo existe mientras no tiene ninguna.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Derrota {
    Por(FaccionId),
    /// Su última plaza colapsó sola (ruina).
    Colapso,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Faccion {
    pub id: FaccionId,
    pub nombre: String,
    pub sigilo: Sigilo,
    /// `None` solo si no queda ningún ciudadano.
    pub rey: Option<HeroeId>,
    pub embajador: Option<HeroeId>,
    /// Cuándo designó el Rey su capital (cooldown, §3.7).
    pub capital_designada_en: Option<Instante>,
    /// Monótona, en milésimas.
    pub experiencia: Mil,
    /// En **orden de ingreso** (el fundador primero): de aquí sale la sucesión del trono.
    pub ciudadanos: Vec<HeroeId>,
    /// Héroes que han pedido entrar, en el orden en que lo pidieron. No caducan.
    pub solicitudes: Vec<HeroeId>,
    pub derrota: Option<Derrota>,
    pub admite_otras_en_ataques: bool,
}

impl Faccion {
    pub fn nivel(&self, r: &Reglas) -> u32 {
        nivel::nivel(&r.nivel, self.experiencia)
    }

    pub fn es_ciudadano(&self, h: HeroeId) -> bool {
        self.ciudadanos.contains(&h)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorFaccion {
    NoExiste,
    /// El nombre queda vacío tras recortarlo.
    NombreVacio,
    /// Otra Facción ya se llama así (sin mirar mayúsculas).
    NombreDuplicado,
    SigiloInvalido,
    SigiloDuplicado,
    /// Ya es ciudadano de una Facción: 1 héroe, 1 Facción.
    YaPertenece,
    /// Dejó una Facción hace menos de `cooldownCreacionFaccionDias`.
    CooldownCreacion { libre_en: Instante },
    /// No ha pedido entrar en esa Facción.
    NoHaPedido,
    /// No es ciudadano de ninguna Facción (o no de esa).
    NoPertenece,
    /// Q7: va en un ejército (aunque sea solo) o en una convocatoria con otros héroes; primero se separa o lo deshace (y si es el Líder, cede antes el mando).
    EnEjercito,
    /// El designado no es ciudadano de la Facción.
    NoCiudadano,
    /// La Facción no tiene Rey (para designar Embajador).
    SinRey,
}

/// Lo que cambió al salir un héroe de su Facción.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Salida {
    pub faccion: FaccionId,
    /// El Rey después de la salida, si el que se fue era el Rey (`None` si no quedó nadie).
    pub sucesion: Option<Option<HeroeId>>,
    /// La embajada quedó vacante (se fue el Embajador, o el heredero del trono lo era).
    pub embajador_vaciado: bool,
}

/// Las Facciones de la partida, el índice de ciudadanía y cuándo dejó cada héroe su última Facción (`salidasFaccionPorHeroe`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Facciones {
    por_id: BTreeMap<FaccionId, Faccion>,
    de_heroe: BTreeMap<HeroeId, FaccionId>,
    salidas: BTreeMap<HeroeId, Instante>,
}

/// El nombre tal como se compara: recortado y sin mayúsculas.
fn clave_de_nombre(n: &str) -> String {
    n.trim().to_lowercase()
}

impl Facciones {
    /// Reconstruye el almacén (y su índice) desde lo guardado en una foto.
    pub fn desde(facciones: impl IntoIterator<Item = Faccion>, salidas: BTreeMap<HeroeId, Instante>) -> Self {
        let por_id: BTreeMap<_, _> = facciones.into_iter().map(|f| (f.id, f)).collect();
        let de_heroe = por_id.values().flat_map(|f| f.ciudadanos.iter().map(move |&h| (h, f.id))).collect();
        Self { por_id, de_heroe, salidas }
    }

    pub fn get(&self, id: FaccionId) -> Option<&Faccion> {
        self.por_id.get(&id)
    }

    pub(crate) fn get_mut(&mut self, id: FaccionId) -> Option<&mut Faccion> {
        self.por_id.get_mut(&id)
    }

    pub fn existe(&self, id: FaccionId) -> bool {
        self.por_id.contains_key(&id)
    }

    /// En orden de id.
    pub fn iter(&self) -> impl Iterator<Item = &Faccion> {
        self.por_id.values()
    }

    pub fn salidas(&self) -> &BTreeMap<HeroeId, Instante> {
        &self.salidas
    }

    /// La Facción de la que `h` es ciudadano.
    pub fn faccion_de(&self, h: HeroeId) -> Option<FaccionId> {
        self.de_heroe.get(&h).copied()
    }

    /// ¿Está libre ese nombre? `excepto` no cuentan (en la fusión, las dos que se disuelven).
    pub fn nombre_libre(&self, nombre: &str, excepto: &[FaccionId]) -> bool {
        let clave = clave_de_nombre(nombre);
        !self.por_id.values().any(|f| !excepto.contains(&f.id) && clave_de_nombre(&f.nombre) == clave)
    }

    /// `crearFaccion` (§3.1). Validaciones en este orden: nombre no vacío y libre, sigilo del catálogo y no duplicado (si lo trae), el actor no es
    /// ciudadano de ninguna, cooldown de creación. La Facción nace con el actor como primer ciudadano y Rey, nivel 1, sin Embajador, y sin
    /// solicitar en ninguna otra; sin sigilo recibe [`crate::sigilo::Catalogo::sigilo_libre`] con la semilla `f-{id}` (el id del contrato).
    /// La memoria del héroe sin bandera, su columna y su reputación (0) son del núcleo.
    pub fn crear(&mut self, r: &Reglas, id: FaccionId, actor: HeroeId, nombre: &str, sigilo: Option<Sigilo>, ahora: Instante) -> Result<&Faccion, ErrorFaccion> {
        let nombre = nombre.trim();
        if nombre.is_empty() {
            return Err(ErrorFaccion::NombreVacio);
        }
        if !self.nombre_libre(nombre, &[]) {
            return Err(ErrorFaccion::NombreDuplicado);
        }
        if let Some(s) = &sigilo {
            match r.sigilo.motivo_rechazo(s, self.por_id.values().map(|f| &f.sigilo)) {
                Some(MotivoSigilo::Invalido) => return Err(ErrorFaccion::SigiloInvalido),
                Some(MotivoSigilo::Duplicado) => return Err(ErrorFaccion::SigiloDuplicado),
                None => {}
            }
        }
        if self.de_heroe.contains_key(&actor) {
            return Err(ErrorFaccion::YaPertenece);
        }
        if let Some(&salio) = self.salidas.get(&actor) {
            if ahora < salio + r.cooldown_creacion {
                return Err(ErrorFaccion::CooldownCreacion { libre_en: salio + r.cooldown_creacion });
            }
        }
        let sigilo = sigilo.unwrap_or_else(|| r.sigilo.sigilo_libre(&format!("f-{id}"), self.por_id.values().map(|f| &f.sigilo)));
        self.sin_solicitudes_de(actor);
        self.por_id.insert(
            id,
            Faccion {
                id,
                nombre: nombre.to_string(),
                sigilo,
                rey: Some(actor),
                embajador: None,
                capital_designada_en: None,
                experiencia: 0,
                ciudadanos: vec![actor],
                solicitudes: vec![],
                derrota: None,
                admite_otras_en_ataques: false,
            },
        );
        self.de_heroe.insert(actor, id);
        Ok(&self.por_id[&id])
    }

    /// Una Facción sin ciudadanos que nace porque la administración la nombra (`FijarFaccionDeHeroe`, `CrearEjercito`; decisión 9 del plan del sistema 5): «Facción N», con el
    /// sigilo libre de su id, nivel 1 y sin Rey. Quien reciba después la ciudadanía (`otorgar`) será su Rey (5A-8). No mira el nombre: la administración no compite por él.
    pub fn crear_vacia(&mut self, r: &Reglas, id: FaccionId) -> &Faccion {
        if self.por_id.contains_key(&id) {
            return &self.por_id[&id];
        }
        let sigilo = r.sigilo.sigilo_libre(&format!("f-{id}"), self.por_id.values().map(|f| &f.sigilo));
        let f = Faccion {
            id,
            nombre: format!("Facción {id}"),
            sigilo,
            rey: None,
            embajador: None,
            capital_designada_en: None,
            experiencia: 0,
            ciudadanos: vec![],
            solicitudes: vec![],
            derrota: None,
            admite_otras_en_ataques: false,
        };
        self.por_id.entry(id).or_insert(f)
    }

    /// `solicitarIngreso` (§3.2): entra en la lista de esa Facción; pedir dos veces es pedir lo mismo (`Ok(false)`). Se puede pedir a varias a la vez.
    pub fn solicitar(&mut self, faccion: FaccionId, h: HeroeId) -> Result<bool, ErrorFaccion> {
        if self.de_heroe.contains_key(&h) {
            return Err(ErrorFaccion::YaPertenece);
        }
        let f = self.por_id.get_mut(&faccion).ok_or(ErrorFaccion::NoExiste)?;
        if f.solicitudes.contains(&h) {
            return Ok(false);
        }
        f.solicitudes.push(h);
        Ok(true)
    }

    /// `responderSolicitud` (§3.2): aceptar lo hace ciudadano (al final de la lista) y lo quita de **todas** las listas; denegar solo lo quita de esta.
    /// Aceptar no da residencia (el núcleo funde su memoria y su columna toma la Facción).
    pub fn responder(&mut self, faccion: FaccionId, h: HeroeId, aceptar: bool) -> Result<(), ErrorFaccion> {
        let f = self.por_id.get_mut(&faccion).ok_or(ErrorFaccion::NoExiste)?;
        if !f.solicitudes.contains(&h) {
            return Err(ErrorFaccion::NoHaPedido);
        }
        if !aceptar {
            f.solicitudes.retain(|&x| x != h);
            return Ok(());
        }
        self.otorgar(faccion, h)
    }

    /// La ciudadanía por cualquier vía (aceptar una solicitud, y la administración: `FijarFaccionDeHeroe`, decisión 9): al final de `ciudadanos` y sin
    /// solicitudes en ninguna. Quien ya es ciudadano de otra no la recibe; de esta misma, no cambia nada.
    pub fn otorgar(&mut self, faccion: FaccionId, h: HeroeId) -> Result<(), ErrorFaccion> {
        match self.de_heroe.get(&h) {
            Some(&f) if f == faccion => return Ok(()),
            Some(_) => return Err(ErrorFaccion::YaPertenece),
            None => {}
        }
        if !self.por_id.contains_key(&faccion) {
            return Err(ErrorFaccion::NoExiste);
        }
        self.sin_solicitudes_de(h);
        let f = self.por_id.get_mut(&faccion).expect("existe");
        f.ciudadanos.push(h);
        // 5A-8 (2026-10-24): quien recibe la ciudadanía de una Facción que se quedó sin nadie pasa a ser su Rey.
        f.rey.get_or_insert(h);
        self.de_heroe.insert(h, faccion);
        Ok(())
    }

    fn sin_solicitudes_de(&mut self, h: HeroeId) {
        self.por_id.values_mut().for_each(|f| f.solicitudes.retain(|&x| x != h));
    }

    /// `quitarCiudadania` (§3.3) con la **sucesión**: si se va el Rey, el trono pasa al primero de los que quedan (orden de ingreso); el Embajador se vacía si
    /// era el que se iba o si es el nuevo Rey. No estampa el cooldown (la administración y el traslado lo usan así); la orden de jugador es [`Facciones::dejar`].
    pub fn quitar(&mut self, h: HeroeId) -> Option<Salida> {
        let id = self.de_heroe.remove(&h)?;
        let f = self.por_id.get_mut(&id).expect("índice coherente");
        f.ciudadanos.retain(|&x| x != h);
        let sucesion = (f.rey == Some(h)).then(|| f.ciudadanos.first().copied());
        if let Some(nuevo) = sucesion {
            f.rey = nuevo;
        }
        let embajador_vaciado = f.embajador.is_some() && (f.embajador == Some(h) || f.embajador == f.rey);
        if embajador_vaciado {
            f.embajador = None;
        }
        Some(Salida { faccion: id, sucesion, embajador_vaciado })
    }

    /// `dejarFaccion` (§3.3): solo la propia. Q7 (y 5A-4, 2026-10-24): se rechaza **siempre** que vaya en un ejército, aunque sea de un solo héroe, y en una
    /// convocatoria con otros héroes (`en_ejercito`: lo sabe el núcleo). Estampa `salidasFaccionPorHeroe` (el cooldown de creación). Dejar la residencia, los
    /// cargos locales y la guarnición, y pasar al campamento más cercano (decisión 10), es del núcleo.
    pub fn dejar(&mut self, h: HeroeId, ahora: Instante, en_ejercito: bool) -> Result<Salida, ErrorFaccion> {
        if !self.de_heroe.contains_key(&h) {
            return Err(ErrorFaccion::NoPertenece);
        }
        if en_ejercito {
            return Err(ErrorFaccion::EnEjercito);
        }
        let salida = self.quitar(h).expect("es ciudadano");
        self.salidas.insert(h, ahora);
        Ok(salida)
    }

    /// `asignarRey` (§3.4): el designado ha de ser ciudadano. Con Rey, es un traspaso: el antiguo sigue de ciudadano y el Embajador no se toca. Quién puede
    /// hacerlo (cualquier ciudadano sin Rey; con Rey, solo él) es de la autorización.
    pub fn asignar_rey(&mut self, faccion: FaccionId, h: HeroeId) -> Result<(), ErrorFaccion> {
        let f = self.por_id.get_mut(&faccion).ok_or(ErrorFaccion::NoExiste)?;
        if !f.es_ciudadano(h) {
            return Err(ErrorFaccion::NoCiudadano);
        }
        f.rey = Some(h);
        Ok(())
    }

    /// `asignarEmbajador` (§3.4): la Facción tiene Rey y el designado es ciudadano (puede ser el propio Rey).
    pub fn asignar_embajador(&mut self, faccion: FaccionId, h: HeroeId) -> Result<(), ErrorFaccion> {
        let f = self.por_id.get_mut(&faccion).ok_or(ErrorFaccion::NoExiste)?;
        if f.rey.is_none() {
            return Err(ErrorFaccion::SinRey);
        }
        if !f.es_ciudadano(h) {
            return Err(ErrorFaccion::NoCiudadano);
        }
        f.embajador = Some(h);
        Ok(())
    }

    /// `admitirOtrasFacciones`: el ajuste permanente del Rey. Devuelve si cambió.
    pub fn admitir_otras(&mut self, faccion: FaccionId, admitir: bool) -> Result<bool, ErrorFaccion> {
        let f = self.por_id.get_mut(&faccion).ok_or(ErrorFaccion::NoExiste)?;
        Ok(std::mem::replace(&mut f.admite_otras_en_ataques, admitir) != admitir)
    }

    /// `aplicarExperiencia`: suma la experiencia (lo ≤ 0 se ignora: es monótona) y devuelve el nivel nuevo **si sube** (el núcleo emite `faccion.nivel_subio`
    /// en el mismo instante).
    pub fn aplicar_experiencia(&mut self, r: &Reglas, faccion: FaccionId, delta: Mil) -> Option<u32> {
        let f = self.por_id.get_mut(&faccion).filter(|_| delta > 0)?;
        let antes = f.nivel(r);
        f.experiencia = f.experiencia.saturating_add(delta);
        let despues = f.nivel(r);
        (despues > antes).then_some(despues)
    }

    /// `registrarDerrota` (§8): se llama al caer la **última** plaza de `perdedora` (el núcleo sabe si le queda alguna). `ganadora = None`: colapsó sola.
    pub fn registrar_derrota(&mut self, perdedora: FaccionId, ganadora: Option<FaccionId>) {
        if let Some(f) = self.por_id.get_mut(&perdedora) {
            f.derrota = Some(ganadora.map_or(Derrota::Colapso, Derrota::Por));
        }
    }

    /// Al fundar, la derrota se borra.
    pub fn anular_derrota(&mut self, faccion: FaccionId) {
        if let Some(f) = self.por_id.get_mut(&faccion) {
            f.derrota = None;
        }
    }

    /// Una Facción que nace fuera de `crear` (la de una fusión); sus ciudadanos entran en el índice.
    pub fn insertar(&mut self, f: Faccion) {
        f.ciudadanos.iter().for_each(|&h| {
            self.de_heroe.insert(h, f.id);
        });
        self.por_id.insert(f.id, f);
    }

    /// Una Facción que desaparece (traslado); sus ciudadanos salen del índice.
    pub(crate) fn eliminar(&mut self, id: FaccionId) -> Option<Faccion> {
        let f = self.por_id.remove(&id)?;
        f.ciudadanos.iter().for_each(|h| {
            self.de_heroe.remove(h);
        });
        Some(f)
    }

    /// Añade ciudadanos al final, sin repetir (el traslado); quien era ciudadano de otra se mueve a esta.
    pub(crate) fn unir_ciudadanos(&mut self, faccion: FaccionId, nuevos: &[HeroeId]) {
        for &h in nuevos {
            if let Some(otra) = self.de_heroe.insert(h, faccion).filter(|&o| o != faccion) {
                if let Some(o) = self.por_id.get_mut(&otra) {
                    o.ciudadanos.retain(|&x| x != h);
                }
            }
            let f = self.por_id.get_mut(&faccion).expect("existe");
            if !f.ciudadanos.contains(&h) {
                f.ciudadanos.push(h);
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod pruebas {
    //! Casos de `typescript-final:src/engine/__tests__/{faccion,cargos,eventosDominioFaccion}.test.ts` y de las reglas de `session/comandos/crearFaccion.ts`
    //! que prueba `comandosMembresiaFaccion.test.ts`.
    use super::*;
    use crate::datos::DIA;

    pub(crate) fn r() -> Reglas {
        Reglas::default()
    }

    /// Una Facción `id` creada por `rey` con estos ciudadanos más, en orden.
    pub(crate) fn con_faccion(fs: &mut Facciones, id: FaccionId, nombre: &str, rey: HeroeId, mas: &[HeroeId]) {
        fs.crear(&r(), id, rey, nombre, None, 0).unwrap();
        for &h in mas {
            fs.otorgar(id, h).unwrap();
        }
    }

    #[test]
    fn crear_hace_al_actor_primer_ciudadano_y_rey() {
        let mut fs = Facciones::default();
        let f = fs.crear(&r(), 1, 10, "  Micenas ", None, 0).unwrap();
        assert_eq!((f.nombre.as_str(), f.rey, f.embajador, f.ciudadanos.clone(), f.experiencia), ("Micenas", Some(10), None, vec![10], 0));
        assert_eq!(f.nivel(&r()), 1);
        assert_eq!(f.sigilo, r().sigilo.sigilo_libre("f-1", [].iter()));
        assert_eq!(fs.faccion_de(10), Some(1));
    }

    #[test]
    fn crear_valida_nombre_sigilo_pertenencia_y_cooldown_en_orden() {
        let mut fs = Facciones::default();
        let r = r();
        assert_eq!(fs.crear(&r, 1, 10, "   ", None, 0).unwrap_err(), ErrorFaccion::NombreVacio);
        fs.crear(&r, 1, 10, "Micenas", None, 0).unwrap();
        assert_eq!(fs.crear(&r, 2, 11, " MICENAS", None, 0).unwrap_err(), ErrorFaccion::NombreDuplicado, "sin mirar mayúsculas");
        let ocupado = fs.get(1).unwrap().sigilo.clone();
        assert_eq!(fs.crear(&r, 2, 11, "Troya", Some(ocupado.clone()), 0).unwrap_err(), ErrorFaccion::SigiloDuplicado);
        let mut malo = ocupado;
        malo.forma = "inventada".into();
        assert_eq!(fs.crear(&r, 2, 11, "Troya", Some(malo), 0).unwrap_err(), ErrorFaccion::SigiloInvalido);
        assert_eq!(fs.crear(&r, 2, 10, "Troya", None, 0).unwrap_err(), ErrorFaccion::YaPertenece, "1 héroe, 1 Facción");
        // Dejarla estampa el cooldown de creación (7 días); pedir el ingreso en otra sigue libre.
        fs.crear(&r, 2, 11, "Troya", None, 0).unwrap();
        fs.dejar(10, 5 * DIA, false).unwrap();
        assert_eq!(fs.crear(&r, 3, 10, "Tebas", None, 6 * DIA).unwrap_err(), ErrorFaccion::CooldownCreacion { libre_en: 12 * DIA });
        assert_eq!(fs.solicitar(2, 10), Ok(true));
        assert!(fs.crear(&r, 3, 10, "Tebas", None, 12 * DIA).is_ok());
        assert_eq!(fs.get(2).unwrap().solicitudes, Vec::<HeroeId>::new(), "crear borra sus solicitudes en otras");
    }

    #[test]
    fn pedir_entra_en_la_lista_una_vez_y_un_ciudadano_no_pide() {
        let mut fs = Facciones::default();
        con_faccion(&mut fs, 1, "Micenas", 10, &[11]);
        assert_eq!(fs.solicitar(1, 20), Ok(true));
        assert_eq!(fs.solicitar(1, 20), Ok(false), "pedir dos veces es pedir lo mismo");
        assert_eq!(fs.get(1).unwrap().solicitudes, vec![20]);
        assert_eq!(fs.solicitar(1, 11), Err(ErrorFaccion::YaPertenece));
        assert_eq!(fs.solicitar(9, 21), Err(ErrorFaccion::NoExiste));
    }

    #[test]
    fn aceptada_da_ciudadania_y_quita_todas_las_solicitudes_denegada_solo_la_quita() {
        let mut fs = Facciones::default();
        con_faccion(&mut fs, 1, "Micenas", 10, &[]);
        con_faccion(&mut fs, 2, "Troya", 30, &[]);
        fs.solicitar(1, 20).unwrap();
        fs.solicitar(2, 20).unwrap();
        fs.solicitar(1, 21).unwrap();
        fs.responder(1, 21, false).unwrap();
        assert_eq!(fs.get(1).unwrap().solicitudes, vec![20]);
        assert_eq!(fs.faccion_de(21), None);
        fs.responder(1, 20, true).unwrap();
        assert_eq!(fs.get(1).unwrap().ciudadanos, vec![10, 20]);
        assert!(fs.get(1).unwrap().solicitudes.is_empty() && fs.get(2).unwrap().solicitudes.is_empty());
        assert_eq!(fs.responder(2, 20, true), Err(ErrorFaccion::NoHaPedido));
    }

    #[test]
    fn la_sucesion_va_por_orden_de_ingreso() {
        let mut fs = Facciones::default();
        con_faccion(&mut fs, 1, "Micenas", 10, &[11, 12]);
        fs.asignar_embajador(1, 11).unwrap();
        // Se va el Rey: hereda el primero de los que quedan (11), que era el Embajador: la embajada queda vacante.
        let s = fs.dejar(10, 0, false).unwrap();
        assert_eq!(s, Salida { faccion: 1, sucesion: Some(Some(11)), embajador_vaciado: true });
        let f = fs.get(1).unwrap();
        assert_eq!((f.rey, f.embajador, f.ciudadanos.clone()), (Some(11), None, vec![11, 12]));
        // Se va un ciudadano cualquiera: nada de trono.
        assert_eq!(fs.dejar(12, 0, false).unwrap().sucesion, None);
        // Se va el último: sin Rey.
        assert_eq!(fs.dejar(11, 0, false).unwrap().sucesion, Some(None));
        assert_eq!(fs.get(1).unwrap().rey, None);
        assert_eq!(fs.dejar(11, 0, false), Err(ErrorFaccion::NoPertenece));
    }

    #[test]
    fn el_embajador_que_se_va_libera_su_cargo() {
        let mut fs = Facciones::default();
        con_faccion(&mut fs, 1, "Micenas", 10, &[11, 12]);
        fs.asignar_embajador(1, 12).unwrap();
        assert!(fs.dejar(12, 0, false).unwrap().embajador_vaciado);
        assert_eq!(fs.get(1).unwrap().rey, Some(10));
    }

    #[test]
    fn q7_no_se_deja_la_faccion_dentro_de_un_ejercito_ni_siquiera_solo() {
        let mut fs = Facciones::default();
        con_faccion(&mut fs, 1, "Micenas", 10, &[11]);
        // 5A-4: el núcleo pasa `true` por ir en cualquier ejército, también uno en el que solo va él.
        assert_eq!(fs.dejar(11, 0, true), Err(ErrorFaccion::EnEjercito));
        assert_eq!(fs.dejar(10, 0, true), Err(ErrorFaccion::EnEjercito), "el Rey, solo en su ejército, tampoco");
        assert_eq!(fs.faccion_de(11), Some(1), "sigue dentro");
        assert!(fs.salidas().is_empty());
    }

    #[test]
    fn rey_y_embajador_solo_entre_ciudadanos() {
        let mut fs = Facciones::default();
        con_faccion(&mut fs, 1, "Micenas", 10, &[11]);
        assert_eq!(fs.asignar_rey(1, 99), Err(ErrorFaccion::NoCiudadano));
        assert_eq!(fs.asignar_embajador(1, 99), Err(ErrorFaccion::NoCiudadano));
        fs.asignar_embajador(1, 10).unwrap();
        assert_eq!(fs.get(1).unwrap().embajador, Some(10), "el Embajador puede ser el propio Rey");
        fs.asignar_rey(1, 11).unwrap();
        let f = fs.get(1).unwrap();
        assert_eq!((f.rey, f.embajador), (Some(11), Some(10)), "traspaso: el Embajador no se toca");
        // Embajador sin Rey (no se llega por órdenes desde que una Facción nace con Rey; la guarda sigue).
        fs.dejar(11, 0, false).unwrap();
        fs.dejar(10, 0, false).unwrap();
        assert_eq!(fs.asignar_embajador(1, 10), Err(ErrorFaccion::SinRey));
    }

    #[test]
    fn la_experiencia_es_monotona_y_sube_el_nivel_en_el_momento() {
        let r = r();
        let mut fs = Facciones::default();
        con_faccion(&mut fs, 1, "Micenas", 10, &[]);
        fs.get_mut(1).unwrap().experiencia = 95_000;
        assert_eq!(fs.aplicar_experiencia(&r, 1, 1_000), None, "sin cruzar el umbral");
        assert_eq!(fs.aplicar_experiencia(&r, 1, -50_000), None);
        assert_eq!(fs.get(1).unwrap().experiencia, 96_000, "lo negativo se ignora");
        assert_eq!(fs.aplicar_experiencia(&r, 1, 10_000), Some(2), "cruzando: nivel 2 en el acto");
        assert_eq!(fs.aplicar_experiencia(&r, 9, 10_000), None);
    }

    #[test]
    fn la_derrota_se_anota_y_se_anula_al_fundar() {
        let mut fs = Facciones::default();
        con_faccion(&mut fs, 1, "Micenas", 10, &[]);
        fs.registrar_derrota(1, Some(2));
        assert_eq!(fs.get(1).unwrap().derrota, Some(Derrota::Por(2)));
        fs.registrar_derrota(1, None);
        assert_eq!(fs.get(1).unwrap().derrota, Some(Derrota::Colapso));
        fs.anular_derrota(1);
        assert_eq!(fs.get(1).unwrap().derrota, None);
    }

    #[test]
    fn una_faccion_vacia_vuelve_a_tener_rey_y_el_indice_se_reconstruye() {
        let mut fs = Facciones::default();
        con_faccion(&mut fs, 1, "Micenas", 10, &[]);
        fs.dejar(10, 0, false).unwrap();
        fs.otorgar(1, 20).unwrap();
        assert_eq!(fs.get(1).unwrap().rey, Some(20));
        let copia = Facciones::desde(fs.iter().cloned().collect::<Vec<_>>(), fs.salidas().clone());
        assert_eq!(copia, fs);
        assert_eq!(fs.admitir_otras(1, true), Ok(true));
        assert_eq!(fs.admitir_otras(1, true), Ok(false));
    }

    #[test]
    fn la_que_nombra_la_administracion_nace_vacia_y_su_primer_ciudadano_es_rey() {
        let mut fs = Facciones::default();
        let f = fs.crear_vacia(&r(), 7);
        assert_eq!((f.nombre.as_str(), f.rey, f.ciudadanos.len()), ("Facción 7", None, 0));
        fs.otorgar(7, 30).unwrap();
        fs.otorgar(7, 31).unwrap();
        assert_eq!((fs.get(7).unwrap().rey, fs.faccion_de(31)), (Some(30), Some(7)));
        assert_eq!(fs.crear_vacia(&r(), 7).ciudadanos, vec![30, 31], "si ya existe, no la toca");
    }
}
