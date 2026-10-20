//! Conquista, desalojo, protección y ocupación (Combate §8-§10, Murallas `OCUPACION`, Reloj_Y_Eventos §3.1, fase 4D). Cuando un asedio con números tumba una
//! plaza, [`Nucleo::conquistar_plaza`] ejecuta, en este orden y en el mismo instante: la ciudad cambia de dueño y se saquea (`combate::conquista`: población,
//! edificios dañados, obra de ascenso y decisiones del gobierno perdidas), queda protegida y ocupada, los héroes de dentro y los residentes se desalojan,
//! el comercio se cierra con la plaza (`Motivo::Conquista`: su flota pasa al conquistador, los trueques se anulan, la caravana aparcada se destruye) y, si era la
//! última plaza de su Facción, se anota la derrota.
//!
//! # La protección y la ocupación son eventos
//! La plaza guarda `protegida_hasta` (inatacable un día) y `ocupada_hasta` (hora y media con la recaudación y el crecimiento a la mitad y el mantenimiento
//! congelado). Cada una tiene su evento en la cola, ligado a la generación de [`entidad_conquista`] (una conquista nueva descarta los de la anterior): al vencer
//! se limpia el campo y se cuenta. Mientras la plaza está ocupada, el paso de minuto de su economía pasa los factores de la ocupación al `Exterior`
//! (`felicidad_extra` y `recaudacion_extra`, que existían para esto) y `Asentamiento::ocupado` congela su mantenimiento. Nada recurre por plaza.
//!
//! # Lo provisional
//! Los cargos, los fundadores y las casas compradas quedan vacíos cuando existan (sistema 5: hoy no hay nada que vaciar). La derrota de la Facción solo se
//! cuenta (`faccion.derrotada`); qué hace después quien la gobierna, la reputación y la experiencia de Facción por conquistar son del sistema político. Los
//! recintos de muralla no existen (fase propia): no pierden avance. El traslado de los residentes no mira el cupo de viviendas del destino (Combate §9).

use super::*;
use crate::estado::{SucesoCombate, PRIO_ASENTAMIENTO};
use crate::estado_combate::entidad_conquista;
use combate::conquista::{aplicar_conquista, aplicar_desalojo, faccion_derrotada, planear_desalojo, refugio, residencia_tras_desalojo, Afectado, Salida};
use comercio::ruina::Motivo as MotivoDeCierre;
use militar::columna::{Columna, Politica, TipoColumna};
use militar::heroe::Residencia;
use militar::presencia::Ubicacion;
use std::collections::BTreeSet;

impl Nucleo {
    // ───────────────────────────── Consultas ─────────────────────────────

    /// Hasta cuándo es inatacable la plaza tras su última conquista, si lo es todavía.
    pub fn protegida_hasta(&self, plaza: u64) -> Option<Instante> {
        self.estado.asentamientos.get(&plaza)?.protegida_hasta
    }

    /// Hasta cuándo está ocupada la plaza (recaudación y crecimiento a la mitad), si lo está todavía.
    pub fn ocupada_hasta(&self, plaza: u64) -> Option<Instante> {
        self.estado.asentamientos.get(&plaza)?.ocupada_hasta
    }

    // ───────────────────────────── La conquista ─────────────────────────────

    /// La plaza cae en manos de `conquistador` (la Facción del ejército `ejercito`). `lucharon` son los héroes que la defendieron en persona con las
    /// escuadras con que lo hicieron: si sobreviven, salen con ellos. La llama el asedio con números; la de Unity (4E) llamará a la misma.
    pub(super) fn conquistar_plaza(&mut self, t: Instante, plaza: u64, conquistador: u32, ejercito: u64, lucharon: &[(u64, Vec<u64>)]) {
        let cr = self.estado.reglas.economia.combate.clone();
        let datos = self.estado.reglas.economia.datos.clone();
        let (vieja, posicion) = {
            let a = &self.estado.asentamientos[&plaza];
            (a.faccion, a.posicion)
        };

        // 1. Quién se ve afectado, mirado antes de que nada cambie: los residentes (estén o no dentro) y quien está dentro de visita.
        let dentro = Ubicacion::Asentamiento(plaza as u32);
        let ids: BTreeSet<u64> = self.estado.militar.residentes_de(plaza).map(|h| h.heroe.id).chain(self.heroes().filter(|h| h.heroe.presencia.ubicacion == dentro).map(|h| h.heroe.id)).collect();
        let afectados: Vec<Afectado> = ids
            .into_iter()
            .map(|id| {
                let h = self.heroe(id).expect("existe");
                let sobrevivieron = |ids: &Vec<u64>| ids.iter().copied().filter(|s| h.heroe.escuadra(*s).is_some_and(|e| e.con_soldados())).collect();
                Afectado {
                    heroe: id,
                    residente: h.heroe.residencia == Some(Residencia::Asentamiento(plaza as u32)),
                    dentro: h.heroe.presencia.ubicacion == dentro,
                    columna_aparcada: h.columna,
                    lucharon: lucharon.iter().find(|(l, _)| *l == id).map(|(_, ids)| sobrevivieron(ids)).unwrap_or_default(),
                }
            })
            .collect();

        // 2. La ciudad cambia de dueño y se saquea; queda protegida y ocupada, con sus dos plazos en la cola.
        let c = {
            let a = self.estado.asentamientos.get_mut(&plaza).expect("existe");
            a.faccion = conquistador;
            let c = aplicar_conquista(&cr, &datos, &mut a.economia, t);
            (a.protegida_hasta, a.ocupada_hasta) = (Some(c.protegida_hasta), Some(c.ocupacion_hasta));
            c
        };
        let entidad = entidad_conquista(plaza);
        self.estado.generaciones.subir(entidad);
        let clave = self.estado.generaciones.clave(entidad);
        self.estado.cola.programar(c.protegida_hasta, PRIO_ASENTAMIENTO, clave, Carga::Combate(SucesoCombate::FinProteccion));
        self.estado.cola.programar(c.ocupacion_hasta, PRIO_ASENTAMIENTO, clave, Carga::Combate(SucesoCombate::FinOcupacion));
        self.tr.asentamientos.push(plaza);
        // La plaza cambió de bandera: sus fronteras con otras Facciones se miran de nuevo.
        self.contactos_nuevos(plaza, t);

        // 3. Los residentes y quien estaba dentro (Combate §9).
        let refugio_de = refugio(&self.estado.asentamientos.values().filter(|a| a.faccion == vieja && a.id != plaza).map(|a| (a.id as u32, a.posicion.dist(posicion))).collect::<Vec<_>>());
        // Un artefacto de mundo siempre trae campamentos de mercenarios; sin ellos nadie se queda sin casa porque no hay casa que dar.
        let campamento = self.geografia.as_ref().and_then(|g| g.mapa.campamentos.iter().enumerate().min_by_key(|(_, c)| c.dist2(posicion)).map(|(i, _)| i as u32)).unwrap_or(0);
        let nueva = residencia_tras_desalojo(refugio_de, campamento);
        for d in planear_desalojo(&afectados, nueva) {
            if d.residencia.is_some() {
                self.mudar_de_residencia(d.heroe, d.residencia);
                aplicar_desalojo(&mut self.heroe_mut(d.heroe).heroe, &d);
            }
            match &d.salida {
                None => {}
                Some(Salida::ALaColumna(columna)) => self.ubicar_en_columna(d.heroe, *columna),
                Some(Salida::ColumnaNueva { con_faccion_de_la_plaza, escuadras }) => self.columna_de_salida(t, d.heroe, posicion, con_faccion_de_la_plaza.then_some(vieja), escuadras),
            }
            self.emitir_propio(t, "heroe.desalojado", vec![plaza], vec![("heroe", d.heroe as i64), ("residente", d.residencia.is_some() as i64), ("dentro", d.salida.is_some() as i64)]);
        }

        // 4. El comercio se cierra con la plaza que cambia de dueño.
        self.cerrar_comercio(plaza, MotivoDeCierre::Conquista, t);

        // 5. La Facción que se queda sin plazas queda derrotada.
        let quedan = self.plazas_de(vieja).count();
        if faccion_derrotada(quedan) {
            self.emitir_propio(t, "faccion.derrotada", vec![plaza], vec![("faccion", vieja as i64), ("por", conquistador as i64)]);
        }
        self.emitir_propio(
            t,
            "asentamiento.conquistada",
            vec![plaza],
            vec![
                ("faccionAnterior", vieja as i64),
                ("faccion", conquistador as i64),
                ("ejercito", ejercito as i64),
                ("protegidaHasta", c.protegida_hasta),
                ("ocupadaHasta", c.ocupacion_hasta),
                ("edificiosDanados", c.edificios_danados.len() as i64),
                ("pesantsPerdidos", c.poblacion_perdida.0),
                ("artesanosPerdidos", c.poblacion_perdida.1),
                ("noblezaPerdida", c.poblacion_perdida.2),
            ],
        );
    }

    /// Quien estaba dentro y no tiene columna a la puerta sale a la calle (Combate §9): nace una columna personal quieta en la plaza, con el carro vacío, los
    /// víveres que llevaba el héroe y las escuadras con que defendió y sobrevivieron. Su Facción es la de la plaza si residía; un visitante sale sin bandera.
    fn columna_de_salida(&mut self, t: Instante, heroe: u64, posicion: Punto, faccion: Option<u32>, escuadras: &[u64]) {
        let mr = self.estado.reglas.economia.militar.clone();
        let id = self.nuevo_id();
        let viveres = std::mem::take(&mut self.heroe_mut(heroe).heroe.viveres);
        let heroes = [heroe];
        let militar = &self.estado.militar;
        let columna = Columna::salir(&mr, id, faccion, heroe, TipoColumna::Personal, Politica::Rechazar, escuadras.to_vec(), militar::Carga::new(), viveres, t, &|ids| militar.soldados_de(&heroes, ids));
        let e = self.columna_suelta(columna, posicion);
        self.alta_de_columna(t, e);
        self.ubicar_en_columna(heroe, id);
    }

    // ───────────────────────────── Los plazos ─────────────────────────────

    /// Acaba la protección de una plaza conquistada: se la puede asediar otra vez. (Si una conquista nueva se llevó la plaza antes, este evento ya no existe.)
    pub(super) fn fin_de_proteccion(&mut self, plaza: u64, t: Instante) {
        let Some(a) = self.estado.asentamientos.get_mut(&plaza) else { return };
        a.protegida_hasta = None;
        self.tr.asentamientos.push(plaza);
        self.emitir_propio(t, "asentamiento.proteccion_terminada", vec![plaza], vec![]);
    }

    /// Acaba la ocupación: vuelven la recaudación, el crecimiento y el mantenimiento de la plaza.
    pub(super) fn fin_de_ocupacion(&mut self, plaza: u64, t: Instante) {
        let Some(a) = self.estado.asentamientos.get_mut(&plaza) else { return };
        (a.ocupada_hasta, a.economia.ocupado) = (None, false);
        self.tr.asentamientos.push(plaza);
        self.emitir_propio(t, "asentamiento.ocupacion_terminada", vec![plaza], vec![]);
    }
}
