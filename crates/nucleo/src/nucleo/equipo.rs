//! El equipo, la mochila y el almacén de objetos del héroe dentro del núcleo (Heroe_Y_Tropa §4.5; entradas 70 y 71): `equipar_objeto`, `desequipar_objeto`,
//! `guardar_objeto` y `sacar_objeto`. Las reglas son del crate `objetos` (`Equipo::equipar`, `Equipo::quitar` e `Inventario::pasar`, que no cambian nada si
//! rechazan); aquí están dónde se puede (D2 y D22), los rechazos, el hecho y el registro de cambios.
//!
//! - **Equipar y quitar** (D2): dentro de una plaza, un pueblo o un campamento; nunca en una columna, en una avanzada ni fuera del mundo, ni con una escuadra del
//!   héroe reservada para una batalla.
//! - **Guardar y sacar** (D22): con las reglas de acceso del almacén personal, el almacén de objetos está en su residencia y solo se usa estando dentro de ella.

use super::*;
use militar::heroe::Residencia;
use militar::presencia::Ubicacion;
use objetos::equipo::ErrorEquipo;
use objetos::inventario::ErrorInventario;

/// El rechazo de una regla de la mochila o del almacén de objetos; `lleno` es el del inventario al que iba la pieza.
pub(super) fn rechazo_de_inventario(e: ErrorInventario, lleno: &'static str) -> (&'static str, &'static str) {
    use rechazos::*;
    match e {
        ErrorInventario::Cantidad => (INVENTARIO_CANTIDAD_INVALIDA, "la cantidad es 0 o la pila pasaría del máximo"),
        ErrorInventario::Desconocido => (OBJETO_DESCONOCIDO, "la pieza no está en el catálogo"),
        ErrorInventario::NoCabe => (lleno, "no cabe: no cambia nada"),
        ErrorInventario::SinCasilla => (INVENTARIO_CASILLA_VACIA, "esa casilla está vacía"),
        ErrorInventario::PocaCantidad => (INVENTARIO_POCA_CANTIDAD, "la casilla tiene menos de lo que pides"),
    }
}

fn rechazo_de_equipo(e: ErrorEquipo) -> (&'static str, &'static str) {
    use rechazos::*;
    match e {
        ErrorEquipo::Inventario(e) => rechazo_de_inventario(e, MOCHILA_LLENA),
        ErrorEquipo::ClaseDesconocida => (HEROE_CLASE_DESCONOCIDA, "esa clase no existe"),
        ErrorEquipo::Desconocido => (OBJETO_DESCONOCIDO, "la pieza no está en el catálogo"),
        ErrorEquipo::NoEquipable => (EQUIPO_NO_EQUIPABLE, "un material o un consumible no se equipa"),
        ErrorEquipo::Incompatible => (EQUIPO_INCOMPATIBLE, "la clase de tu arma no admite el peso de esa armadura"),
        ErrorEquipo::ArmaFija => (EQUIPO_ARMA_FIJA, "el arma no se quita: se cambia por otra"),
        ErrorEquipo::HuecoVacio => (EQUIPO_HUECO_VACIO, "no llevas nada en ese hueco"),
    }
}

impl Nucleo {
    pub(super) fn ejecutar_equipo(&mut self, t: Instante, p: &Peticion) -> Result<(), Respuesta> {
        let v = self.estado.version;
        let rechazo = |(c, m): (&'static str, &'static str)| Respuesta::rechazo(c, m, v);
        let mr = self.estado.reglas.economia.militar.clone();
        match &p.comando {
            Comando::EquiparObjeto { heroe, casilla } => {
                self.exigir_lugar_de_equipar(p, *heroe)?;
                let h = &mut self.heroe_mut(*heroe).heroe;
                h.equipo.equipar(&mut h.mochila, &mr.objetos, *casilla).map_err(|e| rechazo(rechazo_de_equipo(e)))?;
                self.emitir_propio(t, "heroe.equipo_cambiado", vec![], vec![("heroe", *heroe as i64)]);
            }
            Comando::DesequiparObjeto { heroe, hueco } => {
                self.exigir_lugar_de_equipar(p, *heroe)?;
                let h = &mut self.heroe_mut(*heroe).heroe;
                h.equipo.quitar(&mut h.mochila, &mr.objetos, *hueco).map_err(|e| rechazo(rechazo_de_equipo(e)))?;
                self.emitir_propio(t, "heroe.equipo_cambiado", vec![], vec![("heroe", *heroe as i64)]);
            }
            Comando::GuardarObjeto { heroe, casilla, cantidad } => {
                self.exigir_dentro_de_su_residencia(p, *heroe)?;
                let h = &mut self.heroe_mut(*heroe).heroe;
                h.mochila.pasar(&mut h.almacen_objetos, &mr.objetos, *casilla, *cantidad).map_err(|e| rechazo(rechazo_de_inventario(e, rechazos::ALMACEN_OBJETOS_LLENO)))?;
            }
            Comando::SacarObjeto { heroe, casilla, cantidad } => {
                self.exigir_dentro_de_su_residencia(p, *heroe)?;
                let h = &mut self.heroe_mut(*heroe).heroe;
                h.almacen_objetos.pasar(&mut h.mochila, &mr.objetos, *casilla, *cantidad).map_err(|e| rechazo(rechazo_de_inventario(e, rechazos::MOCHILA_LLENA)))?;
            }
            _ => unreachable!("solo se llama con las órdenes del equipo, la mochila y el almacén de objetos"),
        }
        Ok(())
    }

    /// D2: dentro de una plaza, un pueblo o un campamento, y sin ninguna escuadra reservada para una batalla.
    pub(super) fn exigir_lugar_de_equipar(&self, p: &Peticion, heroe: u64) -> Result<(), Respuesta> {
        self.autorizar_heroe(p, heroe, false)?;
        let (v, h) = (self.estado.version, &self.heroe(heroe).expect("existe").heroe);
        match h.presencia.ubicacion {
            Ubicacion::Asentamiento(_) | Ubicacion::Pueblo(_) | Ubicacion::Mercenarios(_) => {}
            Ubicacion::Columna(_) | Ubicacion::Avanzada(_) | Ubicacion::Desconectado(_) => {
                return Err(Respuesta::rechazo(rechazos::EQUIPO_FUERA_DE_LUGAR, "el equipo se cambia dentro de una plaza, un pueblo o un campamento", v));
            }
        }
        if h.escuadras.iter().any(|e| e.reserva_batalla.is_some()) {
            return Err(Respuesta::rechazo(rechazos::EQUIPO_EN_BATALLA, "tienes una escuadra reservada para una batalla", v));
        }
        Ok(())
    }

    /// D22: el almacén de objetos está en su residencia, y solo se usa estando dentro de ella (la plaza o el campamento donde reside).
    fn exigir_dentro_de_su_residencia(&self, p: &Peticion, heroe: u64) -> Result<(), Respuesta> {
        self.autorizar_heroe(p, heroe, false)?;
        let (v, h) = (self.estado.version, &self.heroe(heroe).expect("existe").heroe);
        let dentro = match h.residencia {
            Some(Residencia::Asentamiento(a)) => h.presencia.ubicacion == Ubicacion::Asentamiento(a),
            Some(Residencia::Campamento(c)) => h.presencia.ubicacion == Ubicacion::Mercenarios(c),
            None => return Err(Respuesta::rechazo(rechazos::HEROE_SIN_RESIDENCIA, "sin residencia no hay almacén de objetos", v)),
        };
        if !dentro {
            return Err(Respuesta::rechazo(rechazos::ALMACEN_OBJETOS_FUERA_DE_TU_RESIDENCIA, "el almacén de objetos está en tu residencia: hay que estar dentro", v));
        }
        Ok(())
    }
}

#[cfg(test)]
mod pruebas {
    //! Dónde se puede (D2 y D22) y cada rechazo, sobre un núcleo sin mundo: las piezas que aún no da ninguna orden (tienda, forja, botín) se meten a mano en la mochila. Las reglas
    //! de cada pieza las prueba el crate `objetos`; la orden de punta a punta, `tests/heroes.rs`.
    use super::*;
    use crate::estado_militar::HeroeNucleo;
    use crate::{Ajustes, Arranque, Reglas, Resultado};
    use contrato::v1::GrafoNavegacion;
    use militar::escuadra::Escuadra;
    use militar::heroe::{Controlador, Heroe};
    use objetos::datos::Hueco;
    use objetos::{Inventario, Pieza};
    use rutas::Rutas;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    const HEROE: u64 = 7;

    /// Un héroe de la espada (armaduras media y pesada) dentro del campamento 0, donde reside, con esas piezas en su mochila, de la casilla 0 en adelante.
    fn nucleo(piezas: &[&str]) -> Nucleo {
        let reglas = Reglas::default();
        let rutas = Arc::new(Rutas::desde_grafo(&GrafoNavegacion::default()));
        let mut n = Nucleo::nuevo(Arranque { reglas: reglas.clone(), ajustes: Ajustes::default(), semilla: 1, inicio: 0, rutas, mundo_hash: "h".into(), mundo: None });
        let mr = &reglas.economia.militar;
        let mut h = Heroe::nuevo(mr, HEROE, Controlador::Humano, 0, 1, "espada_escudo_mediano").expect("una clase del catálogo");
        for p in piezas {
            h.mochila.meter(&mr.objetos, Pieza::del_catalogo(p), 1).unwrap();
        }
        n.estado.militar.insertar(HeroeNucleo::nuevo(h, "ana", None));
        n
    }

    fn heroe(n: &mut Nucleo) -> &mut Heroe {
        &mut n.estado.militar.heroes.get_mut(&HEROE).unwrap().heroe
    }

    /// La orden de ana: `Ok` si se acepta, el código si se rechaza.
    fn orden(n: &mut Nucleo, c: Comando) -> Result<(), &'static str> {
        static ID: AtomicU64 = AtomicU64::new(0);
        let r = n.aplicar_en(0, Peticion::nueva("ana", &format!("o{}", ID.fetch_add(1, Ordering::Relaxed)), c));
        match r.resultado {
            Resultado::Aceptada => Ok(()),
            Resultado::Rechazada { codigo, .. } => Err(codigo),
            otro => panic!("{otro:?}"),
        }
    }

    fn equipar(casilla: u32) -> Comando {
        Comando::EquiparObjeto { heroe: HEROE, casilla }
    }

    fn quitar(hueco: Hueco) -> Comando {
        Comando::DesequiparObjeto { heroe: HEROE, hueco }
    }

    #[test]
    fn equipar_y_quitar_siguen_las_reglas_de_la_clase_del_arma() {
        let mut n = nucleo(&["dagas_cobre", "casco_pesada_cobre", "torso_ligera_cobre", "crisol"]);
        let mr = n.estado.reglas.economia.militar.clone();
        assert_eq!(orden(&mut n, quitar(Hueco::Arma)), Err(rechazos::EQUIPO_ARMA_FIJA));
        assert_eq!(orden(&mut n, quitar(Hueco::Casco)), Err(rechazos::EQUIPO_HUECO_VACIO));
        assert_eq!(orden(&mut n, equipar(2)), Err(rechazos::EQUIPO_INCOMPATIBLE), "la espada no admite la ligera");
        assert_eq!(orden(&mut n, equipar(3)), Err(rechazos::EQUIPO_NO_EQUIPABLE));
        assert_eq!(orden(&mut n, equipar(9)), Err(rechazos::INVENTARIO_CASILLA_VACIA));
        assert_eq!(orden(&mut n, equipar(1)), Ok(()), "la pesada, sí");
        assert_eq!(heroe(&mut n).equipo.casco, Some(Pieza::del_catalogo("casco_pesada_cobre")));
        // Cambiar de arma es cambiar de clase: las dagas solo admiten la ligera, y el casco pesado y las cuatro piezas medias van a la mochila con la espada.
        assert_eq!(orden(&mut n, equipar(0)), Ok(()));
        let h = heroe(&mut n);
        assert_eq!((h.clase(&mr), h.equipo.casco.is_none(), h.equipo.torso.is_none(), h.equipo.montura.is_some()), ("dagas", true, true, true));
        assert_eq!(h.mochila.casillas.len(), 3 + 5, "el torso ligero y el crisol, más la espada, el casco y tres piezas medias (la cuarta ocupa la casilla 1)");
        assert_eq!(h.atributos(&mr), mr.objetos.clases["dagas"].atributos_base, "los atributos, los de la base nueva");
        assert_eq!(orden(&mut n, equipar(2)), Ok(()), "ahora la ligera, sí");
        assert_eq!(n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter()).filter(|e| e.codigo == "heroe.equipo_cambiado").count(), 3, "un hecho por cada orden aceptada");
        // Con la mochila llena, lo que se quitaría se queda puesto (D17).
        heroe(&mut n).mochila = Inventario::vacio(0);
        assert_eq!(orden(&mut n, quitar(Hueco::Montura)), Err(rechazos::MOCHILA_LLENA));
        assert!(heroe(&mut n).equipo.montura.is_some());
    }

    #[test]
    fn se_equipa_dentro_de_una_plaza_un_pueblo_o_un_campamento_y_sin_escuadras_reservadas() {
        let mut n = nucleo(&[]);
        for (ubicacion, codigo) in [
            (Ubicacion::Columna(3), Some(rechazos::EQUIPO_FUERA_DE_LUGAR)),
            (Ubicacion::Avanzada(4), Some(rechazos::EQUIPO_FUERA_DE_LUGAR)),
            (Ubicacion::Asentamiento(5), None),
            (Ubicacion::Pueblo(6), None),
            (Ubicacion::Mercenarios(0), None),
        ] {
            heroe(&mut n).presencia.ubicacion = ubicacion;
            assert_eq!(orden(&mut n, quitar(Hueco::Montura)).err(), codigo, "quitar en {ubicacion:?}");
            if codigo.is_none() {
                assert_eq!(orden(&mut n, equipar(0)), Ok(()), "y ponérsela otra vez en {ubicacion:?}");
            }
        }
        let mr = n.estado.reglas.economia.militar.clone();
        let mut e = Escuadra::nueva(&mr, 20, HEROE, militar::catalogo::Tropa::MiliciaLanceros, 10, None, 0);
        e.reserva_batalla = Some(1);
        heroe(&mut n).escuadras.push(e);
        assert_eq!(orden(&mut n, quitar(Hueco::Montura)), Err(rechazos::EQUIPO_EN_BATALLA));
    }

    #[test]
    fn guardar_y_sacar_solo_dentro_de_su_residencia() {
        let mut n = nucleo(&["crisol", "dagas_cobre"]);
        let mr = n.estado.reglas.economia.militar.clone();
        heroe(&mut n).mochila.meter(&mr.objetos, Pieza::del_catalogo("crisol"), 4).unwrap();
        let guardar = |casilla, cantidad| Comando::GuardarObjeto { heroe: HEROE, casilla, cantidad };
        let sacar = |casilla, cantidad| Comando::SacarObjeto { heroe: HEROE, casilla, cantidad };
        assert_eq!(orden(&mut n, guardar(0, 3)), Ok(()), "dentro del campamento donde reside");
        assert_eq!(orden(&mut n, guardar(1, 1)), Ok(()));
        let h = heroe(&mut n);
        assert_eq!((h.mochila.casillas[&0].cantidad, h.mochila.casillas.len(), h.almacen_objetos.casillas[&0].cantidad, h.almacen_objetos.casillas[&1].pieza.objeto.as_str()), (2, 1, 3, "dagas_cobre"));
        assert_eq!(orden(&mut n, sacar(0, 4)), Err(rechazos::INVENTARIO_POCA_CANTIDAD));
        assert_eq!(orden(&mut n, sacar(0, 0)), Err(rechazos::INVENTARIO_CANTIDAD_INVALIDA));
        assert_eq!(orden(&mut n, sacar(5, 1)), Err(rechazos::INVENTARIO_CASILLA_VACIA));
        assert_eq!(orden(&mut n, sacar(1, 1)), Ok(()));
        assert_eq!(heroe(&mut n).mochila.casillas[&1].pieza.objeto, "dagas_cobre", "vuelve a la casilla libre más baja");
        // Llenos: lo que no cabe se rechaza sin mover nada.
        heroe(&mut n).almacen_objetos = Inventario::vacio(1);
        heroe(&mut n).almacen_objetos.meter(&mr.objetos, Pieza::del_catalogo("plano_2"), 1).unwrap();
        assert_eq!(orden(&mut n, guardar(1, 1)), Err(rechazos::ALMACEN_OBJETOS_LLENO));
        heroe(&mut n).mochila = Inventario::vacio(0);
        assert_eq!(orden(&mut n, sacar(0, 1)), Err(rechazos::MOCHILA_LLENA));
        // Fuera de su residencia, o sin ella, no hay almacén de objetos.
        heroe(&mut n).presencia.ubicacion = Ubicacion::Asentamiento(5);
        assert_eq!(orden(&mut n, sacar(0, 1)), Err(rechazos::ALMACEN_OBJETOS_FUERA_DE_TU_RESIDENCIA), "en una plaza donde no reside");
        heroe(&mut n).residencia = Some(Residencia::Asentamiento(5));
        heroe(&mut n).mochila = Inventario::vacio(72);
        assert_eq!(orden(&mut n, sacar(0, 1)), Ok(()), "ya reside en ella");
        heroe(&mut n).residencia = None;
        assert_eq!(orden(&mut n, guardar(0, 1)), Err(rechazos::HEROE_SIN_RESIDENCIA));
    }
}
