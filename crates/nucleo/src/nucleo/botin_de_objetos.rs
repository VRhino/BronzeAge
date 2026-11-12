//! El botín de objetos del héroe (entradas 67, 68 y 70; Progresion_Tropas_Y_Heroe §5 y Catalogo_De_Objetos §6b, D5 a D7, D17 y D22): de dónde llega, con su tabla de
//! `BOTIN_OBJETOS` tirada con el rng del estado, y adónde va: a la mochila; si no cabe, al almacén de objetos, aunque el héroe esté fuera; si tampoco cabe allí, se pierde.
//! Las reglas (tirar una tabla y entregar) son del crate `objetos` (`Tabla::tirar` y `objetos::entregar`); aquí, qué tabla toca y el hecho `heroe.botin`, que dice de dónde
//! llegó y qué fue a la mochila, qué al almacén y qué se perdió. Lo oye solo el héroe.
//!
//! Las fuentes: una batalla de Unity (cada héroe que participó), un campamento de bandidos destruido (dentro de las completas de la ventana del héroe), una diaria
//! individual del tablón, un alijo, el rango 4 y el 5 de un mercenario y el objeto de la recompensa de una misión personal (este, sin tabla).

use super::*;
use crate::estado::Dato;
use objetos::{entregar, Destino, Pieza};

/// De dónde llega un botín con tabla, con lo que la elige.
#[derive(Clone, Copy)]
pub(super) enum Fuente {
    /// Una batalla de Unity, a cada héroe que participó: lo de `soloSiGana`, solo si ganó su bando (D6 y D7).
    Unity { gana: bool },
    /// Un campamento de bandidos de ese nivel destruido, dentro de las destrucciones completas de la ventana de 24 h del héroe.
    Bandidos(u8),
    /// Una diaria individual del tablón, además de su recompensa (D5).
    Diaria,
    /// Un alijo abierto, a quien lo abre.
    Alijo,
    /// El rango que alcanza un mercenario: solo el 4 y el 5 tienen tabla (Mercenarios_Por_Contrato §8.3).
    Mercenario(u8),
}

impl Fuente {
    /// El `origen` del hecho.
    fn origen(self) -> &'static str {
        match self {
            Fuente::Unity { .. } => "unity",
            Fuente::Bandidos(_) => "bandidos",
            Fuente::Diaria => "diaria",
            Fuente::Alijo => "alijo",
            Fuente::Mercenario(_) => "mercenario",
        }
    }
}

impl Nucleo {
    /// Tira la tabla de `fuente` con el rng del estado y entrega lo que sale a `heroe` ([`Nucleo::entregar_objetos`]). Sin tabla (un nivel o un rango que no la tiene) o
    /// sin héroe, no tira nada.
    pub(super) fn botin_de_objetos(&mut self, t: Instante, heroe: u64, fuente: Fuente) {
        let mr = self.estado.reglas.economia.militar.clone();
        let b = &mr.objetos.botin;
        let (tabla, gana) = match fuente {
            Fuente::Unity { gana } => (Some(&b.unity), gana),
            Fuente::Bandidos(nivel) => (b.bandidos.get(&nivel), false),
            Fuente::Diaria => (Some(&b.mision_diaria), false),
            Fuente::Alijo => (Some(&b.alijo), false),
            Fuente::Mercenario(rango) => (b.rango_mercenario.get(&rango), false),
        };
        let Some(tabla) = tabla.filter(|_| self.heroe(heroe).is_some()) else { return };
        let sale = tabla.tirar(gana, &mut self.estado.rng);
        self.entregar_objetos(t, heroe, fuente.origen(), sale);
    }

    /// Entrega a `heroe` cada objeto de `objetos` (id del catálogo y cantidad; D17 y D22): a la mochila, si no cabe al almacén de objetos y si tampoco, se pierde. Emite
    /// `heroe.botin` con `origen` y las tres listas (`[objeto, cantidad]`), si llega algo.
    pub(super) fn entregar_objetos(&mut self, t: Instante, heroe: u64, origen: &'static str, objetos: Vec<(String, u32)>) {
        if objetos.is_empty() || self.heroe(heroe).is_none() {
            return;
        }
        let mr = self.estado.reglas.economia.militar.clone();
        let h = &mut self.heroe_mut(heroe).heroe;
        let (mut mochila, mut almacen, mut perdido) = (Vec::new(), Vec::new(), Vec::new());
        for (objeto, cantidad) in objetos {
            // Las fichas y las cantidades se validaron al cargar las tablas y las misiones: solo una pila que pasaría de u32::MAX no entra, y se pierde.
            let destino = entregar(&mut h.mochila, &mut h.almacen_objetos, &mr.objetos, Pieza::del_catalogo(&objeto), cantidad).unwrap_or(Destino::Perdido);
            let lista = match destino {
                Destino::Mochila => &mut mochila,
                Destino::Almacen => &mut almacen,
                Destino::Perdido => &mut perdido,
            };
            lista.push(Dato::Lista(vec![Dato::Texto(objeto), Dato::Numero(i64::from(cantidad))]));
        }
        let valores = vec![("origen", Dato::Texto(origen.into())), ("mochila", Dato::Lista(mochila)), ("almacen", Dato::Lista(almacen)), ("perdido", Dato::Lista(perdido))];
        self.tr.emitir(&self.estado, EventoNucleo { heroes: vec![heroe], valores, ..EventoNucleo::nuevo("heroe.botin", t, vec![("heroe", heroe as i64)]) });
    }
}

#[cfg(test)]
mod pruebas {
    //! Cada fuente con su tabla y el rng del estado, lo de `soloSiGana` y adónde va lo que llega (D17 y D22), sobre un núcleo sin mundo con un héroe a mano. Cada fuente de punta a
    //! punta (la batalla, el campamento, la diaria, el alijo, el rango y la misión) la prueba su `tests/`.
    use super::*;
    use crate::estado_militar::HeroeNucleo;
    use crate::{Ajustes, Arranque, Reglas};
    use base::Rng;
    use contrato::v1::GrafoNavegacion;
    use militar::heroe::{Controlador, Heroe};
    use objetos::Inventario;
    use rutas::Rutas;
    use std::sync::Arc;

    const HEROE: u64 = 7;

    fn nucleo() -> Nucleo {
        let reglas = Reglas::default();
        let rutas = Arc::new(Rutas::desde_grafo(&GrafoNavegacion::default()));
        let mut n = Nucleo::nuevo(Arranque { reglas: reglas.clone(), ajustes: Ajustes::default(), semilla: 1, inicio: 0, rutas, mundo_hash: "h".into(), mundo: None });
        let h = Heroe::nuevo(&reglas.economia.militar, HEROE, Controlador::Humano, 0, 1, "espada_escudo_mediano").expect("una clase del catálogo");
        n.estado.militar.insertar(HeroeNucleo::nuevo(h, "ana", None));
        n
    }

    fn heroe(n: &mut Nucleo) -> &mut Heroe {
        &mut n.estado.militar.heroes.get_mut(&HEROE).unwrap().heroe
    }

    /// Los `heroe.botin` de la transición en curso (las llamadas directas no la cierran): (origen, mochila, almacén, perdido), cada lista con sus `(objeto, cantidad)`.
    type Hecho = (String, Vec<(String, i64)>, Vec<(String, i64)>, Vec<(String, i64)>);

    fn botines(n: &Nucleo) -> Vec<Hecho> {
        let lista = |d: &Dato| match d {
            Dato::Lista(l) => l
                .iter()
                .map(|x| match x {
                    Dato::Lista(par) => match par.as_slice() {
                        [Dato::Texto(o), Dato::Numero(c)] => (o.clone(), *c),
                        otro => panic!("{otro:?}"),
                    },
                    otro => panic!("{otro:?}"),
                })
                .collect(),
            otro => panic!("{otro:?}"),
        };
        n.tr.eventos
            .iter()
            .filter(|e| e.codigo == "heroe.botin")
            .map(|e| {
                assert_eq!((e.dato("heroe"), e.heroes.as_slice()), (Some(HEROE as i64), &[HEROE][..]), "lo oye solo el héroe");
                let v = |k: &str| &e.valores.iter().find(|(c, _)| *c == k).unwrap().1;
                let Dato::Texto(origen) = v("origen") else { panic!() };
                (origen.clone(), lista(v("mochila")), lista(v("almacen")), lista(v("perdido")))
            })
            .collect()
    }

    #[test]
    fn cada_fuente_tira_su_tabla_con_el_rng_del_estado_y_lo_da_a_la_mochila() {
        let mut n = nucleo();
        let b = n.estado.reglas.economia.militar.objetos.botin.clone();
        let fuentes = [
            (Fuente::Unity { gana: true }, Some(&b.unity), "unity"),
            (Fuente::Bandidos(3), Some(&b.bandidos[&3]), "bandidos"),
            (Fuente::Diaria, Some(&b.mision_diaria), "diaria"),
            (Fuente::Alijo, Some(&b.alijo), "alijo"),
            (Fuente::Mercenario(5), Some(&b.rango_mercenario[&5]), "mercenario"),
            (Fuente::Mercenario(3), None, "mercenario"),
            (Fuente::Bandidos(9), None, "bandidos"),
        ];
        let mut esperado = Vec::new();
        for (fuente, tabla, origen) in fuentes {
            let mut rng = Rng::nuevo(n.estado.rng.estado());
            if let Some(sale) = tabla.map(|t| t.tirar(matches!(fuente, Fuente::Unity { gana: true }), &mut rng)).filter(|s| !s.is_empty()) {
                esperado.push((origen.to_owned(), sale.into_iter().map(|(o, c)| (o, i64::from(c))).collect(), vec![], vec![]));
            }
            n.botin_de_objetos(0, HEROE, fuente);
            assert_eq!(n.estado.rng.estado(), rng.estado(), "{origen}: una tirada por entrada de su tabla; sin tabla, ninguna");
        }
        assert_eq!(botines(&n), esperado);
        let piezas: u32 = heroe(&mut n).mochila.casillas.values().map(|c| c.cantidad).sum();
        assert_eq!(i64::from(piezas), esperado.iter().flat_map(|h| &h.1).map(|(_, c)| c).sum::<i64>(), "todo en la mochila");
        // Un héroe que no existe no gasta ninguna tirada.
        let antes = n.estado.rng.estado();
        n.botin_de_objetos(0, 99, Fuente::Alijo);
        assert_eq!(n.estado.rng.estado(), antes);
    }

    #[test]
    fn en_unity_lo_de_solo_si_gana_solo_va_al_que_gana() {
        let mut n = nucleo();
        for _ in 0..50 {
            n.botin_de_objetos(0, HEROE, Fuente::Unity { gana: false });
        }
        let crisoles = |n: &mut Nucleo| heroe(n).mochila.casillas.values().filter(|c| c.pieza.objeto == "crisol").map(|c| c.cantidad).sum::<u32>();
        assert_eq!(crisoles(&mut n), 50, "un crisol por batalla perdida");
        assert!(!heroe(&mut n).mochila.casillas.values().any(|c| c.pieza.objeto.starts_with("medalla") || c.pieza.objeto == "cofre_monturas"), "ni medallas ni cofre");
        n.botin_de_objetos(0, HEROE, Fuente::Unity { gana: true });
        assert_eq!(crisoles(&mut n), 52, "dos si gana");
    }

    #[test]
    fn lo_que_no_cabe_en_la_mochila_va_al_almacen_de_objetos_y_si_tampoco_se_pierde() {
        // El alijo da siempre tres cosas: 3 crisoles, un plano (III o IV) y una medalla grande de tropa.
        let mut n = nucleo();
        let cat = n.estado.reglas.economia.militar.objetos.clone();
        let h = heroe(&mut n);
        (h.mochila, h.almacen_objetos) = (Inventario::vacio(1), Inventario::vacio(1));
        h.mochila.meter(&cat, Pieza::del_catalogo("crisol"), 1).unwrap();
        n.botin_de_objetos(0, HEROE, Fuente::Alijo);
        let [(origen, mochila, almacen, perdido)] = botines(&n).try_into().unwrap();
        assert_eq!((origen.as_str(), mochila), ("alijo", vec![("crisol".into(), 3)]), "a su pila, aunque la mochila esté llena");
        assert_eq!((almacen.len(), perdido.len()), (1, 1), "el plano, al almacén; la medalla ya no cabe en ninguno");
        assert!(almacen[0].0.starts_with("plano_") && perdido[0] == ("medalla_tropa_grande".into(), 1));
        let h = heroe(&mut n);
        assert_eq!((h.mochila.casillas[&0].cantidad, h.almacen_objetos.casillas[&0].pieza.objeto.clone()), (4, almacen[0].0.clone()));
    }
}
