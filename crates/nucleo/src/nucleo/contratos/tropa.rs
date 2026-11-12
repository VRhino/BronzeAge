//! El rango y la tropa del mercenario: apuntar renombre y medallas en su ficha, la leva propia y la escuadra de premio (Mercenarios_Por_Contrato §8.1 a §8.3). La licencia de reclutar y la
//! reposición de la leva viven en `nucleo/mercenarios.rs`, donde se recluta. Tarea T8 del plan.
//!
//! # Lo que ve cada uno
//! Los hechos de la ficha (`rango_subido`, `medalla`, `premio_reclamado`, `leva_recibida`) los oye solo el mercenario: su ficha es pública en su héroe, pero lo que la hizo crecer no (§10).

use super::*;
use crate::nucleo::botin_de_objetos::Fuente;
use crate::codec_militar::tropa_a_u8;
use crate::estado::Dato;
use ::mercenarios::{Ficha, Medalla};
use militar::escuadra::{Contenedor, Escuadra};

impl Nucleo {
    // ───────────────────────────── Ayudas ─────────────────────────────

    /// La ficha de `heroe`, que se va a cambiar: la ven él y el tablón donde esté ofrecido. Nace con el primer ofrecimiento, y nadie gana renombre ni reclama nada sin haberse ofrecido.
    pub(super) fn ficha_mut(&mut self, heroe: u64) -> &mut Ficha {
        self.tr.tocar(Clase::Heroe, heroe);
        if let Some(oferta) = self.estado.contratos.ofertas.get(&heroe) {
            self.tr.tocar(Clase::Campamento, u64::from(oferta.campamento));
        }
        self.estado.contratos.fichas.get_mut(&heroe).expect("la ficha nace con el primer ofrecimiento")
    }

    /// Suma renombre a la ficha de `heroe` (§8.3). Por cada rango nuevo la ficha suma su escuadra de premio y se cuenta `mercenarios.rango_subido`; el 4 y el 5 dan además su
    /// tabla de objetos (`BOTIN_OBJETOS.rangoMercenario`, entrada 68), a su mochila.
    pub(super) fn apuntar_renombre(&mut self, t: Instante, heroe: u64, n: u32) {
        let d = self.estado.reglas.economia.contratos.clone();
        for rango in self.ficha_mut(heroe).sumar_renombre(&d, n) {
            self.emitir_nombrando(t, "mercenarios.rango_subido", vec![], vec![heroe], vec![("heroe", heroe as i64), ("rango", i64::from(rango))]);
            self.botin_de_objetos(t, heroe, Fuente::Mercenario(rango));
        }
    }

    /// Cuenta `n` para la medalla `m` de `heroe` (§9). Si cruza un umbral, se cuenta `mercenarios.medalla` y se apunta el renombre de ese grado (§8.2).
    pub(super) fn apuntar_medalla(&mut self, t: Instante, heroe: u64, m: Medalla, n: u32) {
        let d = self.estado.reglas.economia.contratos.clone();
        let Some(grado) = self.ficha_mut(heroe).contar(&d, m, n) else { return };
        let valores = vec![("medalla", Dato::Texto(m.id().to_owned()))];
        self.tr.emitir(&self.estado, EventoNucleo { heroes: vec![heroe], valores, ..EventoNucleo::nuevo("mercenarios.medalla", t, vec![("heroe", heroe as i64), ("grado", i64::from(grado))]) });
        self.apuntar_renombre(t, heroe, d.renombre.grado_medalla[usize::from(grado) - 1]);
    }

    /// Suma `soldados` a la escuadra de `tropa` de `heroe`, sin pasar de los que lleva una completa: se la suma donde esté (una escuadra por tropa en toda la partida,
    /// Campamentos_Mercenarios §5.3) o, si no tiene, nace una nueva en su campamento (sin plaza de origen). Lo que sobra se pierde. Devuelve su id, si es nueva y cuántos soldados se
    /// sumaron de verdad. No usa `dar_escuadra`, que mete la nueva en la columna del héroe sin mirar el Liderazgo: el premio y la recompensa se reclaman también con la columna a la
    /// puerta, y unirla a ella es cosa suya, con su Liderazgo.
    pub(in crate::nucleo) fn sumar_soldados(&mut self, t: Instante, heroe: u64, tropa: Tropa, soldados: i64) -> (u64, bool, i64) {
        let mr = self.estado.reglas.economia.militar.clone();
        let completa = mr.tropas.tropa(tropa).unidades;
        let Some((id, contenedor, cantidad)) = self.heroe(heroe).expect("el héroe existe").heroe.escuadra_de(tropa).map(|e| (e.id, e.contenedor, e.cantidad)) else {
            let id = self.nuevo_id();
            let sumados = soldados.min(completa);
            self.heroe_mut(heroe).heroe.escuadras.push(Escuadra::nueva(&mr, id, heroe, tropa, sumados, None, t));
            return (id, true, sumados);
        };
        let columna = if let Contenedor::Ejercito(c) = contenedor { Some(c) } else { None };
        let velocidad_antes = columna.and_then(|c| self.velocidad_de_ejercito(c)).unwrap_or(0);
        let sumados = soldados.min(completa - cantidad);
        self.heroe_mut(heroe).heroe.escuadra_mut(id).expect("la acabamos de leer").cantidad += sumados;
        // La tropa sumada que va en una columna come más y marcha distinto.
        if let Some(c) = columna {
            self.recomponer_columna(c, t, velocidad_antes);
        }
        (id, false, sumados)
    }

    // ───────────────────────────── Órdenes ─────────────────────────────

    /// La leva propia del rango 1 (§8.3): la primera vez que `heroe` se ofrece recibe una escuadra completa y suya de cada tropa de `levaPropia`, en su campamento. No es prestada: gana
    /// experiencia, no se pierde al mudarse y la repone pagando (`tropas_que_ofrece_a`). Devuelve las escuadras nuevas (la que ya tenía se repone). Las veces siguientes no hace nada.
    pub(super) fn dar_leva_propia(&mut self, t: Instante, heroe: u64) -> Vec<u64> {
        let d = self.estado.reglas.economia.contratos.clone();
        let ficha = self.estado.contratos.fichas.get_mut(&heroe).expect("la ficha nace con el primer ofrecimiento");
        if std::mem::replace(&mut ficha.leva_recibida, true) {
            return Vec::new();
        }
        let mr = self.estado.reglas.economia.militar.clone();
        let nuevas: Vec<u64> = d.leva_propia.iter().map(|&tropa| self.sumar_soldados(t, heroe, tropa, mr.tropas.tropa(tropa).unidades)).filter_map(|(id, nueva, _)| nueva.then_some(id)).collect();
        self.emitir_nombrando(t, "mercenarios.leva_recibida", vec![], vec![heroe], vec![("heroe", heroe as i64), ("escuadras", nuevas.len() as i64)]);
        nuevas
    }

    /// `reclamar_premio_de_rango` (§8.3): `heroe` cambia una escuadra de premio por una completa y suya de `tropa`, que ha de ser de su licencia, en su campamento base, dentro o con la
    /// columna a la puerta. Si ya tiene escuadra de esa tropa se la reponen entera; si no, nace una nueva en el campamento, y la devuelve.
    pub(super) fn reclamar_premio_de_rango(&mut self, t: Instante, p: &Peticion, heroe: u64, tropa: Tropa) -> Result<Vec<u64>, Respuesta> {
        use rechazos::*;
        self.autorizar_heroe(p, heroe, false)?;
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: &str| Respuesta::rechazo(c, m, v);
        if self.estado.contratos.fichas.get(&heroe).is_none_or(|f| f.premios == 0) {
            return Err(rechazo(MERCENARIO_SIN_PREMIO, "no tienes ninguna escuadra de premio por reclamar"));
        }
        // Su campamento base: el de su contrato o, sin contrato, aquel donde reside.
        let base = self.estado.contratos.contratos.get(&heroe).map(|c| c.campamento_base).or_else(|| self.campamento_de_residencia(heroe));
        let campamento = base.ok_or_else(|| rechazo(MERCENARIOS_NO_RESIDES, "el premio se reclama en tu campamento base: el de tu contrato o, sin contrato, aquel donde resides"))?;
        if !self.esta_en_el_campamento(heroe, campamento, t) {
            return Err(rechazo(CAMPAMENTO_FUERA_DE_LA_PUERTA, "el premio se reclama dentro de tu campamento base o con tu columna a su puerta"));
        }
        if !self.tropas_de_licencia(heroe, campamento).contains(&tropa) {
            return Err(rechazo(MERCENARIO_FUERA_DE_LICENCIA, "esa tropa no es de tu licencia: tu edificio y tu rango la abren, y alguien ha de haber descubierto su tecnología"));
        }
        let completa = self.estado.reglas.economia.militar.tropas.tropa(tropa).unidades;
        if self.heroe(heroe).expect("autorizado").heroe.escuadra_de(tropa).is_some_and(|e| e.cantidad >= completa) {
            return Err(rechazo(MERCENARIOS_AL_TOPE, "esa escuadra ya está completa: el premio no le añadiría nada"));
        }

        let (escuadra, nueva, _) = self.sumar_soldados(t, heroe, tropa, completa);
        self.ficha_mut(heroe).premios -= 1;
        let datos = vec![("heroe", heroe as i64), ("campamento", i64::from(campamento)), ("tropa", i64::from(tropa_a_u8(tropa))), ("escuadra", escuadra as i64), ("nueva", i64::from(nueva))];
        self.emitir_nombrando(t, "mercenarios.premio_reclamado", vec![], vec![heroe], datos);
        Ok(if nueva { vec![escuadra] } else { vec![] })
    }
}

#[cfg(test)]
mod pruebas {
    //! `apuntar_renombre` y `apuntar_medalla` no tienen comando que los dé hasta que T9 enganche los hechos que ganan renombre: se prueban aquí, sobre un núcleo sin mundo con un bot que ya tiene ficha.
    use super::*;
    use crate::estado_militar::HeroeNucleo;
    use crate::{Ajustes, Arranque, Reglas};
    use contrato::v1::GrafoNavegacion;
    use militar::heroe::{Controlador, Heroe};
    use rutas::Rutas;
    use std::sync::Arc;

    const BOT: u64 = 7;

    fn nucleo_con_bot() -> Nucleo {
        let reglas = Reglas::default();
        let rutas = Arc::new(Rutas::desde_grafo(&GrafoNavegacion::default()));
        let mut n = Nucleo::nuevo(Arranque { reglas: reglas.clone(), ajustes: Ajustes::default(), semilla: 1, inicio: 0, rutas, mundo_hash: "h".into(), mundo: None });
        n.estado.militar.insertar(HeroeNucleo::nuevo(Heroe::nuevo(&reglas.economia.militar, BOT, Controlador::Bot, 0, 1, "dagas").expect("una clase del catálogo"), "bot", None));
        n.estado.contratos.fichas.insert(BOT, Ficha::nueva());
        n
    }

    /// Los hechos de ese código que ha dado la transición en curso, en orden.
    fn hechos<'a>(n: &'a Nucleo, codigo: &str) -> Vec<&'a EventoNucleo> {
        n.tr.eventos.iter().filter(|e| e.codigo == codigo).collect()
    }

    #[test]
    fn el_renombre_que_cruza_un_rango_da_su_premio_y_su_hecho_y_si_salta_dos_da_los_dos_en_orden() {
        let mut n = nucleo_con_bot();
        let d = n.estado.reglas.economia.contratos.clone();
        let lanza = d.rangos[1].renombre;

        n.apuntar_renombre(0, BOT, lanza - 1);
        let ficha = &n.estado.contratos.fichas[&BOT];
        assert_eq!((ficha.renombre, ficha.rango, ficha.premios), (lanza - 1, 1, 0));
        assert!(hechos(&n, "mercenarios.rango_subido").is_empty());

        n.apuntar_renombre(0, BOT, 1);
        let ficha = &n.estado.contratos.fichas[&BOT];
        assert_eq!((ficha.renombre, ficha.rango, ficha.premios), (lanza, 2, 1));
        let subidos = hechos(&n, "mercenarios.rango_subido");
        assert_eq!((subidos.len(), subidos[0].dato("heroe"), subidos[0].dato("rango")), (1, Some(BOT as i64), Some(2)));
        assert_eq!((&subidos[0].heroes, &subidos[0].facciones), (&vec![BOT], &Vec::<u32>::new()), "lo oye el mercenario");

        // Salta el rango 3 y el 4 de golpe: un premio por cada uno y un hecho por cada uno, en orden.
        let veterano = d.rangos[3].renombre;
        n.apuntar_renombre(0, BOT, veterano - lanza);
        let ficha = &n.estado.contratos.fichas[&BOT];
        assert_eq!((ficha.rango, ficha.premios), (4, 3));
        let rangos: Vec<_> = hechos(&n, "mercenarios.rango_subido").iter().map(|h| h.dato("rango")).collect();
        assert_eq!(rangos, [Some(2), Some(3), Some(4)]);
    }

    #[test]
    fn el_grado_nuevo_de_una_medalla_da_su_hecho_y_el_renombre_de_ese_grado_y_solo_ese() {
        let mut n = nucleo_con_bot();
        let d = n.estado.reglas.economia.contratos.clone();
        let [bronce, plata, _] = d.umbrales(Medalla::Muralla);

        n.apuntar_medalla(0, BOT, Medalla::Muralla, bronce - 1);
        let ficha = &n.estado.contratos.fichas[&BOT];
        assert_eq!((ficha.contadores[&Medalla::Muralla], ficha.renombre), (bronce - 1, 0), "sin cruzar el umbral no hay grado ni renombre");
        assert!(hechos(&n, "mercenarios.medalla").is_empty());

        n.apuntar_medalla(0, BOT, Medalla::Muralla, 1);
        let ficha = &n.estado.contratos.fichas[&BOT];
        assert_eq!((ficha.grados[&Medalla::Muralla], ficha.renombre), (1, d.renombre.grado_medalla[0]));
        let medallas = hechos(&n, "mercenarios.medalla");
        assert_eq!((medallas.len(), medallas[0].dato("heroe"), medallas[0].dato("grado")), (1, Some(BOT as i64), Some(1)));
        assert_eq!(medallas[0].valores, vec![("medalla", Dato::Texto("muralla".into()))]);
        assert_eq!((&medallas[0].heroes, &medallas[0].facciones), (&vec![BOT], &Vec::<u32>::new()), "lo oye el mercenario");

        // Seguir contando dentro del mismo grado no repite ni el hecho ni el renombre; el siguiente grado da el suyo.
        n.apuntar_medalla(0, BOT, Medalla::Muralla, 1);
        assert_eq!(hechos(&n, "mercenarios.medalla").len(), 1);
        n.apuntar_medalla(0, BOT, Medalla::Muralla, plata);
        let ficha = &n.estado.contratos.fichas[&BOT];
        assert_eq!((ficha.grados[&Medalla::Muralla], ficha.renombre), (2, d.renombre.grado_medalla[0] + d.renombre.grado_medalla[1]));
        assert_eq!(hechos(&n, "mercenarios.medalla").len(), 2);
    }

    #[test]
    fn el_renombre_de_una_medalla_puede_subir_de_rango() {
        let mut n = nucleo_con_bot();
        let d = n.estado.reglas.economia.contratos.clone();
        n.apuntar_renombre(0, BOT, d.rangos[1].renombre - 1);
        assert_eq!(n.estado.contratos.fichas[&BOT].rango, 1);

        n.apuntar_medalla(0, BOT, Medalla::PalabraCumplida, 1);
        let ficha = &n.estado.contratos.fichas[&BOT];
        assert_eq!((ficha.rango, ficha.premios), (2, 1), "el renombre del bronce lo lleva al rango 2");
        assert_eq!(hechos(&n, "mercenarios.rango_subido").len(), 1);
    }

    #[test]
    fn cambiar_la_ficha_marca_al_heroe_y_el_tablon_donde_esta_ofrecido() {
        let mut n = nucleo_con_bot();
        n.apuntar_renombre(0, BOT, 1);
        assert!(n.tr.tocados.contiene(Clase::Heroe, BOT) && n.tr.tocados.de(Clase::Campamento).is_empty());

        n.estado.contratos.ofertas.insert(BOT, ::mercenarios::Oferta { campamento: 3, desde: 0 });
        n.apuntar_medalla(0, BOT, Medalla::Leal, 1);
        assert!(n.tr.tocados.contiene(Clase::Campamento, 3));
    }
}
