//! La experiencia de combate de las escuadras y de los héroes (Progresion_Tropas_Y_Heroe §2-§4; Eras_IV_Y_V §2 y §3): **un solo punto de entrada** para la escuadra
//! ([`Nucleo::experiencia_de_escuadra`]) y otro para el héroe ([`Nucleo::experiencia_de_heroe`]), por los que pasan el combate con números y la batalla de Unity.
//!
//! - **Con números**, 1 si su bando gana y 0,5 si pierde (`MILITAR.experienciaGanadaPor…`): a cada escuadra que combatió, la que anota su [`Baja`], y a cada héroe sano que combatió con
//!   ellas, una vez por combate ([`Nucleo::experiencia_de_bando`]).
//! - **En Unity**, 4 o 2 según gane su bando (`experienciaUnityPor…`, D7: el motor no lee la de Unity): a cada escuadra que entró y a cada héroe que participó (`aplicar_resultado`). Una
//!   salida anticipada no da nada.
//! - **Ejército permanente** (`bonusExperiencia`): las escuadras de los héroes de una Facción que la tiene ganan un 10 % más. El héroe, no.
//! - **`escuadras.veteranas`**: suma 1 cada vez que una escuadra cruza el nivel 5.
//! - El héroe que sube de nivel emite `heroe.nivel_subio` con su nivel nuevo.

use super::*;
use asentamiento::datos::{por, UNO};
use combate::resolucion::{Baja, Informe, Lado};
use militar::escuadra::Escuadra;
use tecnologia::contador::{Contador, Delta};

/// El nivel que cuenta para `escuadras.veteranas` (Eras_IV_Y_V §3: «una escuadra llega al nivel 5»): es la definición del contador, como los hoplitas de `batallas.conHoplitas`.
const NIVEL_VETERANA: u8 = 5;

impl Nucleo {
    /// El punto de entrada de la experiencia de combate de una escuadra (milésimas), sobre la copia que se volcará al estado: con el bonus de Ejército permanente si su héroe es de una
    /// Facción que lo tiene, y `escuadras.veteranas` si cruza el nivel 5. La tropa prestada no gana (`Escuadra::con_experiencia`).
    pub(super) fn experiencia_de_escuadra(&mut self, t: Instante, e: &mut Escuadra, ganada: Mil) {
        let mr = &self.estado.reglas.economia.militar;
        let con_bonus = self.heroe(e.heroe).and_then(|h| h.faccion).is_some_and(|f| self.estado.tiene_tecnologia(f, &mr.militar.tecnologia_bonus_experiencia));
        let ganada = if con_bonus { por(ganada, UNO + mr.militar.bonus_experiencia) } else { ganada };
        let antes = e.nivel;
        e.con_experiencia(mr, ganada);
        if antes < NIVEL_VETERANA && e.nivel >= NIVEL_VETERANA {
            self.contar(t, Delta::uno(Contador::EscuadrasVeteranas));
        }
    }

    /// El punto de entrada de la experiencia de combate de un héroe (milésimas): sube los niveles que cruce, con sus puntos y su Liderazgo (`Heroe::con_experiencia`), y si sube emite
    /// `heroe.nivel_subio`.
    pub(super) fn experiencia_de_heroe(&mut self, t: Instante, heroe: u64, ganada: Mil) {
        let Some(h) = self.estado.militar.heroes.get_mut(&heroe) else { return };
        self.tr.tocar(Clase::Heroe, heroe);
        if h.heroe.con_experiencia(&self.estado.reglas.economia.militar, ganada) {
            let nivel = i64::from(h.heroe.nivel);
            self.emitir_propio(t, "heroe.nivel_subio", vec![], vec![("heroe", heroe as i64), ("nivel", nivel)]);
        }
    }

    /// Un bando de un combate con números, antes de volcar sus escuadras: a cada una que combatió, la experiencia de su [`Baja`]; a cada héroe que combatió con ellas, la de su bando
    /// según gane o pierda.
    pub(super) fn experiencia_de_bando(&mut self, t: Instante, escuadras: &mut [Escuadra], bajas: &[Baja], heroes: &[u64], victoria: bool) {
        for b in bajas {
            if let Some(e) = escuadras.iter_mut().find(|e| e.id == b.escuadra) {
                self.experiencia_de_escuadra(t, e, b.experiencia);
            }
        }
        let p = &self.estado.reglas.economia.combate.poder;
        let ganada = if victoria { p.xp_victoria } else { p.xp_derrota };
        for &h in heroes {
            self.experiencia_de_heroe(t, h, ganada);
        }
    }

    /// Los dos bandos de un combate con números que acaba en un [`Informe`] (columna contra columna, asedio, saqueo de un pueblo y ataque a una avanzada).
    pub(super) fn experiencia_de_informe(&mut self, t: Instante, i: &Informe, atacante: &mut [Escuadra], defensor: &mut [Escuadra]) {
        self.experiencia_de_bando(t, atacante, &i.atacante.bajas, &i.atacante.heroes, i.ganador == Lado::Atacante);
        self.experiencia_de_bando(t, defensor, &i.defensor.bajas, &i.defensor.heroes, i.ganador == Lado::Defensor);
    }
}

#[cfg(test)]
mod pruebas {
    //! Los dos puntos de entrada sobre un núcleo sin mundo: el combate que los llama lo prueban `tests/combate.rs` (con números) y `tests/batallas.rs` (Unity); aquí, lo que no cabe en
    //! unos pocos combates (cruzar el nivel 5 o el 2 del héroe).
    use super::*;
    use crate::estado_militar::HeroeNucleo;
    use crate::{Ajustes, Arranque, Reglas};
    use contrato::v1::GrafoNavegacion;
    use militar::catalogo::Tropa;
    use militar::heroe::{Controlador, Heroe};
    use rutas::Rutas;
    use std::sync::Arc;

    const HEROE: u64 = 7;
    const FACCION: u32 = 1;

    fn nucleo_con_heroe() -> Nucleo {
        let reglas = Reglas::default();
        let rutas = Arc::new(Rutas::desde_grafo(&GrafoNavegacion::default()));
        let mut n = Nucleo::nuevo(Arranque { reglas: reglas.clone(), ajustes: Ajustes::default(), semilla: 1, inicio: 0, rutas, mundo_hash: "h".into(), mundo: None });
        n.estado.militar.insertar(HeroeNucleo::nuevo(Heroe::nuevo(&reglas.economia.militar, HEROE, Controlador::Humano, 0, 1, "dagas").expect("una clase del catálogo"), "ana", Some(FACCION)));
        n
    }

    fn escuadra(n: &Nucleo) -> Escuadra {
        Escuadra::nueva(&n.estado.reglas.economia.militar, 20, HEROE, Tropa::MiliciaLanceros, 10, None, 0)
    }

    #[test]
    fn el_heroe_que_cruza_un_nivel_gana_su_punto_y_su_liderazgo_y_emite_nivel_subio() {
        let mut n = nucleo_con_heroe();
        n.experiencia_de_heroe(0, HEROE, 99_500);
        let h = &n.heroe(HEROE).unwrap().heroe;
        assert_eq!((h.nivel, h.puntos_de_atributo, h.liderazgo_base), (1, 0, 14));
        assert!(n.tr.eventos.iter().all(|e| e.codigo != "heroe.nivel_subio"), "aún no llega a 100");

        n.experiencia_de_heroe(0, HEROE, 500);
        let h = &n.heroe(HEROE).unwrap().heroe;
        assert_eq!((h.nivel, h.experiencia, h.puntos_de_atributo, h.liderazgo_base), (2, 100_000, 1, 17));
        let subidos: Vec<_> = n.tr.eventos.iter().filter(|e| e.codigo == "heroe.nivel_subio").collect();
        assert_eq!((subidos.len(), subidos[0].dato("heroe"), subidos[0].dato("nivel")), (1, Some(HEROE as i64), Some(2)));
    }

    #[test]
    fn ejercito_permanente_da_un_10_por_ciento_mas_a_las_escuadras_y_nada_al_heroe() {
        let mut n = nucleo_con_heroe();
        let mut e = escuadra(&n);
        n.experiencia_de_escuadra(0, &mut e, 1_000);
        assert_eq!(e.experiencia, 1_000, "sin la tecnología, la de siempre");

        n.fijar_tecnologias(0, FACCION, &["ejercito_permanente".into()]).unwrap();
        n.experiencia_de_escuadra(0, &mut e, 1_000);
        assert_eq!(e.experiencia, 2_100, "con ella, 1,1");
        n.experiencia_de_heroe(0, HEROE, 1_000);
        assert_eq!(n.heroe(HEROE).unwrap().heroe.experiencia, 1_000, "el héroe, no");
    }

    #[test]
    fn la_escuadra_que_cruza_el_nivel_5_cuenta_una_veterana_una_sola_vez() {
        let mut n = nucleo_con_heroe();
        let mut e = escuadra(&n);
        // 10 + 25 + 50 + 100 = 185 para el nivel 5.
        n.experiencia_de_escuadra(0, &mut e, 184_000);
        assert_eq!((e.nivel, n.tecnologia().contador(Contador::EscuadrasVeteranas)), (4, 0));
        n.experiencia_de_escuadra(0, &mut e, 1_000);
        assert_eq!((e.nivel, n.tecnologia().contador(Contador::EscuadrasVeteranas)), (5, 1));
        n.experiencia_de_escuadra(0, &mut e, 200_000);
        assert_eq!((e.nivel, n.tecnologia().contador(Contador::EscuadrasVeteranas)), (6, 1), "ya era veterana");
    }
}
