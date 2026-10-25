//! La aparición, la adopción y las citas de cada tecnología descubierta (sistema 6, tanda 6C; Tecnologia_Aedas §3-§6.1). Las reglas puras son del crate `tecnologia`; aquí están la vista del
//! hito sobre las plazas de la Facción, los disparadores, la orden `AdoptarTecnologia` y las dos citas de cada descubrimiento.
//!
//! # La aparición, sin tick (plan del sistema 6, §5.1)
//! Una tecnología solo puede aparecer cuando cambia algo que mira su hito: quien lo cambia marca a la Facción (`aparicion_pendiente`) y al cerrar la versión se evalúan las candidatas de cada
//! Facción marcada que siga viva, con el estado ya cambiado (así el `tecnologia.aparece` entra en la misma versión que su causa). Los disparadores:
//!
//! | Ocurre | Se marca |
//! |---|---|
//! | se fija un logro; empieza una Era | todas las vivas (`tras_logros`, `mirar_era`) |
//! | un edificio se completa o se mejora; la plaza sube de nivel | su Facción (`anotar_sucesos`; el nivel actual, en `contar_nivel`) |
//! | la zona de una plaza crece (solo por obra) | su Facción (`agrandar_zona`) |
//! | una plaza cambia de dueño, se funda o desaparece; se designa la capital | las Facciones de antes y de ahora (`situar_plaza`, `cambiar_plaza_de_faccion`, `DesignarCapital`) |
//! | se adopta una tecnología (también por administración) | la Facción |
//! | el paso de minuto de una capital, si una candidata pide `recursoEnCapital` y su hito ya se cumple | su Facción (`con_exterior`; solo mira `Reglas::piden_capital`, hoy una) |
//!
//! Perder algo no se mira: aparecer es para siempre. La marca no se persiste: se vacía en cada cierre de versión, y una orden que no se acepta la descarta.
//!
//! # Las citas de cada descubrimiento
//! Al registrarse la descubridora de una tecnología (`primeros`, por cualquier vía: hito, administración y, con 6D y 6E, compra y épica) se programan sus dos citas (`Carga::Tecnologia`,
//! prioridad 10, una entidad por tecnología, `entidad_tecnologia`): la de **conocimiento** a `primeros.en + retrasoConocimiento`, en la que los Aedas la cantan (`aedas.canta_descubrimiento`, público)
//! y, con 6D, la revelan a las plazas con un Aeda parado; y la del **campamento** a `primeros.en + horasTrasLaPrimera`, en la que los campamentos de mercenarios la desbloquean. Como mucho dos por
//! tecnología en toda la partida; nada más programa eventos. Los Aedas también cantan al fijarse un logro o empezar una Era (una tecnología puede tener descubridora antes que logro, por
//! administración, o ser de una Era aún cerrada).

use super::*;
use crate::estado::{entidad_tecnologia, Asentamiento, Dato, SucesoTecnologia, PRIO_TECNOLOGIA};
use asentamiento::catalogo::Tipo;
use asentamiento::edificio::Edificios;
use asentamiento::Recurso;
use politica::autorizacion::{Actor, Vista};
use std::cell::OnceCell;
use tecnologia::adopcion::ErrorAdopcion;
use tecnologia::hito::VistaFaccion;
use tecnologia::{Conjunto, TecId};

/// Lo que el hito mira de una Facción, sobre sus plazas (el índice de `Espacio`) y su capital efectiva (buscada una vez, si alguna condición la pide).
struct VistaDeFaccion<'a> {
    n: &'a Nucleo,
    faccion: u32,
    capital: OnceCell<Option<&'a Asentamiento>>,
}

impl<'a> VistaDeFaccion<'a> {
    fn de(n: &'a Nucleo, faccion: u32) -> Self {
        VistaDeFaccion { n, faccion, capital: OnceCell::new() }
    }

    fn capital(&self) -> Option<&'a Asentamiento> {
        *self.capital.get_or_init(|| self.n.capital_de(self.faccion).and_then(|c| self.n.estado.asentamientos.get(&c)))
    }
}

impl VistaFaccion for VistaDeFaccion<'_> {
    fn edificio_activo(&self, tipo: Tipo, nivel: u8) -> bool {
        self.n.plazas_de(self.faccion).any(|a| a.economia.edificios.hay_activo_con_nivel(tipo, nivel))
    }

    fn recurso_en_capital(&self, r: Recurso) -> bool {
        self.capital().is_some_and(|a| a.economia.almacen.cantidad(r) > 0)
    }

    fn capital_en_nivel(&self, nivel: u8, con: Tipo) -> bool {
        self.capital().is_some_and(|a| a.nivel_actual() >= u32::from(nivel) && a.economia.edificios.activos(con) > 0)
    }

    fn yacimiento_en_territorio(&self, r: Recurso) -> bool {
        let Some(g) = self.n.geografia.as_ref() else { return false };
        let (mapa, catalogo) = (&g.mapa, &self.n.estado.reglas.economia.datos.catalogo);
        let de_ese_recurso = |&i: &u32| catalogo.tipo(mapa.nodos[i as usize].tipo).produccion.is_some_and(|(x, _)| x == r);
        self.n.plazas_de(self.faccion).any(|a| {
            let cerca = [mapa.piedra_en(a.posicion, a.radio), mapa.ganado_en(a.posicion, a.radio), mapa.minerales_en(a.posicion, a.radio)];
            cerca.iter().flatten().filter(|i| de_ese_recurso(i)).any(|&i| self.n.en_zona(a.id, mapa.nodos[i as usize].pos))
        })
    }
}

fn rechazo_de_adopcion(e: ErrorAdopcion) -> (&'static str, &'static str) {
    use rechazos::*;
    match e {
        ErrorAdopcion::SinCapital => (TECNOLOGIA_SIN_CAPITAL, "la Facción no tiene capital: no le queda ninguna plaza"),
        ErrorAdopcion::ReyFueraDeLaCapital => (TECNOLOGIA_REY_FUERA_DE_LA_CAPITAL, "el Rey tiene que estar dentro de la capital"),
        ErrorAdopcion::YaAdoptada => (TECNOLOGIA_YA_ADOPTADA, "la Facción ya la tiene adoptada"),
        ErrorAdopcion::NoAparecida => (TECNOLOGIA_NO_APARECIDA, "a la Facción aún no le ha aparecido"),
        ErrorAdopcion::SinTarifa => (TECNOLOGIA_SIN_TARIFA, "el almacén de la capital no tiene la tarifa de su Era"),
    }
}

impl Nucleo {
    // ───────────────────────────── Los disparadores ─────────────────────────────

    /// Algo que mira el hito cambió para la Facción: se evalúa al cerrar la versión.
    pub(super) fn mirar_aparicion(&mut self, faccion: u32) {
        self.aparicion_pendiente.insert(faccion);
    }

    /// Se fijó un logro o empezó una Era: se evalúan todas las vivas.
    pub(super) fn mirar_aparicion_de_todas(&mut self) {
        self.aparicion_pendiente.extend(self.espacio.plazas_por_faccion.keys().copied());
    }

    /// El paso de minuto de la capital de `faccion` (plan §5.1): solo si una candidata pide `recursoEnCapital` y ya cumple su hito entero se marca la Facción. O(candidatas que lo piden).
    pub(super) fn mirar_recurso_en_capital(&mut self, faccion: u32) {
        let (r, e) = (&self.estado.reglas.economia.tecnologia, &self.estado.tecnologia);
        // Lo barato primero (es el paso de minuto): sin el logro de ninguna de las que piden la capital no hay nada que mirar, ni siquiera la Facción.
        if r.piden_capital.iter().any(|t| e.logro_cumplido(t)) && !e.listas_por_hito(r, faccion, &VistaDeFaccion::de(self, faccion), r.piden_capital).is_empty() {
            self.mirar_aparicion(faccion);
        }
    }

    /// Al cerrar la versión: a cada Facción marcada que siga viva le aparecen las candidatas cuyo hito se cumple (`tecnologia.aparece { faccion, primera; tecnologia }`, privado, atribuido a
    /// su capital).
    // DUDA: se evalúa con todo lo que cambió la transición, al cerrarla (el TypeScript lo hacía en el tick siguiente); así lo que aparece va en la versión de su causa.
    pub(super) fn apariciones(&mut self, t: Instante) {
        if self.aparicion_pendiente.is_empty() {
            return;
        }
        let r = self.estado.reglas.economia.tecnologia.clone();
        for f in std::mem::take(&mut self.aparicion_pendiente) {
            if !self.espacio.plazas_por_faccion.contains_key(&f) {
                continue;
            }
            let listas = self.estado.tecnologia.listas_por_hito(&r, f, &VistaDeFaccion::de(self, f), r.todas());
            if listas.is_empty() {
                continue;
            }
            let capital = self.capital_de(f);
            let punto = capital.and_then(|c| self.estado.asentamientos.get(&c)).map(|a| a.posicion);
            for x in listas {
                let Some(primera) = self.registrar_aparicion(f, x, t) else { continue };
                let datos = vec![("faccion", i64::from(f)), ("primera", primera as i64)];
                let valores = vec![("tecnologia", Dato::Texto(r.def(x).id.clone()))];
                self.tr.emitir(&self.estado, EventoNucleo { asentamientos: capital.into_iter().collect(), punto, valores, ..EventoNucleo::nuevo("tecnologia.aparece", t, datos) });
            }
            self.tr.tocar(Clase::Faccion, u64::from(f));
        }
    }

    /// `hacerAparecer` por cualquier vía: si la Facción es la descubridora, programa las dos citas de la tecnología. `None` si ya le había aparecido; si no, si es la primera del mundo.
    pub(super) fn registrar_aparicion(&mut self, faccion: u32, x: TecId, t: Instante) -> Option<bool> {
        let r = self.estado.reglas.economia.tecnologia.clone();
        let primera = self.estado.tecnologia.hacer_aparecer(&r, faccion, x, t)?;
        if primera {
            let clave = self.estado.generaciones.clave(entidad_tecnologia(x));
            self.estado.cola.programar(t + r.aedas.retraso_conocimiento, PRIO_TECNOLOGIA, clave, Carga::Tecnologia(SucesoTecnologia::Conocimiento));
            self.estado.cola.programar(t + r.campamento.tras_la_primera, PRIO_TECNOLOGIA, clave, Carga::Tecnologia(SucesoTecnologia::Campamento));
        }
        Some(primera)
    }

    // ───────────────────────────── Las citas ─────────────────────────────

    /// Vence una cita de una tecnología descubierta.
    pub(super) fn cita_de_tecnologia(&mut self, suceso: SucesoTecnologia, t: Instante) {
        match suceso {
            // 6D: aquí, además, la revelación a las plazas con un Aeda itinerante parado (Tecnologia_Aedas §6.3).
            SucesoTecnologia::Conocimiento => self.cantar(t),
            // Lo que ofrecen los campamentos depende de sus tecnologías: no se guarda, se marca para quien lo mira.
            SucesoTecnologia::Campamento => self.tocar_todos_los_campamentos(),
        }
    }

    /// TA §3 paso 4: los Aedas cantan los descubrimientos que ya conocen y aún no cantaron (`aedas.canta_descubrimiento { faccion: la descubridora; tecnologia }`, público, a la crónica).
    // DUDA: además de en la cita de conocimiento, al fijarse un logro o empezar una Era (una descubridora sin logro, por administración, o de una Era cerrada); el TS lo hacía en el tick siguiente.
    pub(super) fn cantar(&mut self, t: Instante) {
        let r = self.estado.reglas.economia.tecnologia.clone();
        for (x, f) in self.estado.tecnologia.cantar(&r, t) {
            let valores = vec![("tecnologia", Dato::Texto(r.def(x).id.clone()))];
            self.tr.emitir(&self.estado, EventoNucleo { valores, publico: true, ..EventoNucleo::nuevo("aedas.canta_descubrimiento", t, vec![("faccion", i64::from(f))]) });
        }
    }

    // ───────────────────────────── La adopción ─────────────────────────────

    /// `AdoptarTecnologia` (TA §4): el Rey, presente en la capital efectiva, adopta una tecnología aparecida y la paga el almacén de la capital con la tarifa de su Era (unidades enteras), al
    /// instante (`tecnologia.adoptada { faccion, capital; tecnologia }`, privado, atribuido a la capital). Los rechazos no tocan nada.
    pub(super) fn ejecutar_adopcion(&mut self, t: Instante, p: &Peticion) -> Result<Vec<u64>, Respuesta> {
        let Comando::AdoptarTecnologia { heroe, faccion, tecnologia: x } = p.comando else { unreachable!("solo adoptar una tecnología") };
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: &str| Respuesta::rechazo(c, m, v);
        self.autorizar_heroe(p, heroe, false)?;
        if !self.estado.politica.facciones.existe(faccion) {
            return Err(rechazo(rechazos::FACCION_NO_EXISTE, "la Facción no existe"));
        }
        if !(Actor { yo: heroe, politica: &self.estado.politica, vista: self }).rey_de_faccion(faccion) {
            return Err(rechazo(rechazos::NO_AUTORIZADO, "solo el Rey adopta una tecnología"));
        }
        let r = self.estado.reglas.economia.tecnologia.clone();
        if usize::from(x) >= r.catalogo.len() {
            return Err(rechazo(rechazos::TECNOLOGIA_DESCONOCIDA, "esa tecnología no existe"));
        }
        let capital = self.capital_de(faccion);
        // DUDA: «el Rey está en la capital» (`estaEnAsentamiento`) es estar dentro de ella; con su columna a la puerta, no.
        let en_capital = capital.is_some_and(|c| self.presente(heroe, c as u32));
        let Estado { tecnologia, asentamientos, .. } = &mut self.estado;
        let almacen = capital.map(|c| &mut asentamientos.get_mut(&c).expect("la capital existe").economia.almacen);
        tecnologia.adoptar(&r, faccion, x, almacen, en_capital).map_err(|e| {
            let (c, m) = rechazo_de_adopcion(e);
            rechazo(c, m)
        })?;
        let capital = capital.expect("adoptada en la capital");
        let punto = self.estado.asentamientos.get(&capital).map(|a| a.posicion);
        let datos = vec![("faccion", i64::from(faccion)), ("capital", capital as i64)];
        let valores = vec![("tecnologia", Dato::Texto(r.def(x).id.clone()))];
        self.tr.emitir(&self.estado, EventoNucleo { asentamientos: vec![capital], punto, valores, ..EventoNucleo::nuevo("tecnologia.adoptada", t, datos) });
        self.tr.tocar(Clase::Asentamiento, capital);
        // Una adopción puede dejar a la mitad de las vivas con ella (los campamentos) y cumplir una condición `tecnologia` de otro hito.
        self.tocar_todos_los_campamentos();
        self.mirar_aparicion(faccion);
        // El hecho de épica `adopcion` (TA §7.2): lo recogerán los Aedas residentes de 6E al cerrar la versión.
        self.hechos_epicos.push(tecnologia::epicas::Hecho::adopcion(faccion, x));
        Ok(vec![])
    }

    // ───────────────────────────── Los campamentos ─────────────────────────────

    /// Las tecnologías que desbloquean ahora los campamentos de mercenarios (TA §5): las de arranque; las demás, con descubridora y pasado su plazo, o adoptadas por la mitad de las vivas.
    pub(super) fn tecnologias_de_los_campamentos(&self) -> Conjunto {
        let vivas: Vec<u32> = self.espacio.plazas_por_faccion.keys().copied().collect();
        self.estado.tecnologia.tecnologias_del_campamento(&self.estado.reglas.economia.tecnologia, &vivas, self.estado.ahora)
    }
}
