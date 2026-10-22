//! La convocatoria (Doc 5.14.5, Presencia_Y_Convocatoria §9): un ejército **en preparación dentro** de una plaza o de un campamento, antes de
//! salir. Nadie ha salido todavía: las escuadras siguen en su campamento y el almacén intacto, así que cancelar o separarse no devuelve nada.
//! Espera **sin caducidad** hasta que el Líder parte o cancela. Partir es **todo o nada**.
//!
//! Aquí solo las reglas de quién convoca, quién se une y quién parte. Comprobar que cada uno «saldría con lo que eligió» (Liderazgo, almacén,
//! reserva de trigo…) y sacar de verdad a la gente lo hace el núcleo, que simula la salida real; llega a estas funciones como una función
//! que dice si puede salir.

use crate::columna::Politica;
use crate::datos::Reglas;
use crate::presencia::Ubicacion;
use crate::tipos::{AsentamientoId, CampamentoId, Carga, EscuadraId, FaccionId, HeroeId};
use base::Instante;

/// El lugar donde se prepara: una plaza o un campamento de mercenarios.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lugar {
    Asentamiento(AsentamientoId),
    Campamento(CampamentoId),
}

/// Dónde está dentro un héroe, si está dentro de una plaza o un campamento. Una columna en el mapa, fuera del mundo o dentro de un pueblo o una avanzada (donde no se
/// convoca) no es un lugar.
pub fn lugar_de(ubicacion: Ubicacion) -> Option<Lugar> {
    match ubicacion {
        Ubicacion::Asentamiento(id) => Some(Lugar::Asentamiento(id)),
        Ubicacion::Mercenarios(id) => Some(Lugar::Campamento(id)),
        Ubicacion::Columna(_) | Ubicacion::Pueblo(_) | Ubicacion::Avanzada(_) | Ubicacion::Desconectado(_) => None,
    }
}

/// Con qué saldría un integrante: sus escuadras **en el orden en que quiere que entren en combate** y su carga.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Seleccion {
    pub escuadras: Vec<EscuadraId>,
    pub carga: Carga,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Integrante {
    pub heroe: HeroeId,
    /// Decide la sucesión del mando.
    pub unido_en: Instante,
    pub seleccion: Seleccion,
}

/// Una petición de unión al Líder (política `Preguntar`): vive `vidaPeticionUnion` y el silencio vale como «no».
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Peticion {
    pub heroe: HeroeId,
    pub pedido_en: Instante,
    pub expira_en: Instante,
    pub seleccion: Seleccion,
}

/// Quién quiere unirse y qué se sabe de él: lo reúne quien llama.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Aspirante {
    pub heroe: HeroeId,
    pub lugar: Option<Lugar>,
    pub faccion: Option<FaccionId>,
    /// Ya va en alguna convocatoria (un héroe va como mucho en una).
    pub ya_en_convocatoria: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorConvocatoria {
    /// Se convoca desde dentro de una plaza o de un campamento.
    FueraDeUnLugar,
    /// Sin Facción no hay compañeros con quienes formar un ejército.
    SinFaccion,
    YaEnConvocatoria,
    /// La política de una convocatoria es `Aceptar` o `Preguntar`, nunca `Rechazar`.
    PoliticaInvalida,
    /// Solo se une quien está dentro del mismo lugar.
    OtroLugar,
    /// Un ejército lo componen ciudadanos de una sola Facción.
    OtraFaccion,
    NoEstas,
    /// El Líder no se separa de su ejército: lo cancela.
    ElLiderCancela,
    SoloElLider,
    NoHayPeticion,
    PeticionCaducada,
}

/// Qué pasó al pedir unirse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Union {
    /// Política `Aceptar`: ya está dentro.
    Unido,
    /// Política `Preguntar`: queda una petición que caduca en `expira_en`.
    Pedida { expira_en: Instante },
}

/// Por qué no se parte. Es todo o nada: si falla, nadie sale y el mensaje dice quién y por qué.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorPartir<E> {
    SoloElLider,
    /// El Líder tiene que ser el primero de los que siguen dentro.
    LiderFuera,
    NoPuedeSalir { heroe: HeroeId, motivo: E },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Convocatoria {
    pub id: u64,
    pub faccion: FaccionId,
    pub lugar: Lugar,
    pub lider: HeroeId,
    /// `Aceptar` o `Preguntar`.
    pub politica: Politica,
    /// El Líder primero.
    pub integrantes: Vec<Integrante>,
    pub peticiones: Vec<Peticion>,
    pub creada_en: Instante,
}

/// La convocatoria en la que ya va este héroe, si va en alguna.
pub fn convocatoria_de(convocatorias: &[Convocatoria], heroe: HeroeId) -> Option<&Convocatoria> {
    convocatorias.iter().find(|c| c.integrantes.iter().any(|i| i.heroe == heroe))
}

impl Convocatoria {
    /// `convocarEjercito`: un héroe dentro de un lugar, con Facción y sin otra convocatoria, convoca con él de Líder e integrante único.
    #[allow(clippy::too_many_arguments)]
    pub fn convocar(convocatorias: &[Convocatoria], id: u64, heroe: HeroeId, lugar: Option<Lugar>, faccion: Option<FaccionId>, politica: Politica, seleccion: Seleccion, ahora: Instante) -> Result<Self, ErrorConvocatoria> {
        let lugar = lugar.ok_or(ErrorConvocatoria::FueraDeUnLugar)?;
        let faccion = faccion.ok_or(ErrorConvocatoria::SinFaccion)?;
        if convocatoria_de(convocatorias, heroe).is_some() {
            return Err(ErrorConvocatoria::YaEnConvocatoria);
        }
        if politica == Politica::Rechazar {
            return Err(ErrorConvocatoria::PoliticaInvalida);
        }
        Ok(Self { id, faccion, lugar, lider: heroe, politica, integrantes: vec![Integrante { heroe, unido_en: ahora, seleccion }], peticiones: vec![], creada_en: ahora })
    }

    fn exigir_puede_unirse(&self, a: &Aspirante) -> Result<(), ErrorConvocatoria> {
        if a.lugar != Some(self.lugar) {
            return Err(ErrorConvocatoria::OtroLugar);
        }
        if a.faccion != Some(self.faccion) {
            return Err(ErrorConvocatoria::OtraFaccion);
        }
        if a.ya_en_convocatoria {
            return Err(ErrorConvocatoria::YaEnConvocatoria);
        }
        Ok(())
    }

    /// `unirseAConvocatoria`: con `Aceptar` entra ya (y quita su petición, si la había); con `Preguntar` deja una petición al Líder que vive
    /// `vidaPeticionUnion`, barriendo de paso las caducadas y la suya anterior. Que pueda salir con `seleccion` lo ha comprobado quien llama.
    pub fn unirse(&mut self, r: &Reglas, a: &Aspirante, seleccion: Seleccion, ahora: Instante) -> Result<Union, ErrorConvocatoria> {
        self.exigir_puede_unirse(a)?;
        if self.politica == Politica::Aceptar {
            self.integrar(a.heroe, seleccion, ahora);
            return Ok(Union::Unido);
        }
        let expira_en = ahora + r.movimiento.vida_peticion_union;
        self.peticiones.retain(|p| ahora < p.expira_en && p.heroe != a.heroe);
        self.peticiones.push(Peticion { heroe: a.heroe, pedido_en: ahora, expira_en, seleccion });
        Ok(Union::Pedida { expira_en })
    }

    /// Retira la petición que el Líder contesta: solo el Líder, con la petición viva («el silencio cuenta como un no»). Para aceptar, quien
    /// llama **revalida todo** con la selección de la petición (sigue dentro, misma Facción, puede salir) y llama a [`Convocatoria::aceptar`].
    pub fn retirar_peticion(&mut self, lider: HeroeId, solicitante: HeroeId, ahora: Instante) -> Result<Peticion, ErrorConvocatoria> {
        if self.lider != lider {
            return Err(ErrorConvocatoria::SoloElLider);
        }
        let i = self.peticiones.iter().position(|p| p.heroe == solicitante).ok_or(ErrorConvocatoria::NoHayPeticion)?;
        if ahora >= self.peticiones[i].expira_en {
            return Err(ErrorConvocatoria::PeticionCaducada);
        }
        Ok(self.peticiones.remove(i))
    }

    /// Une al que el Líder aceptó (ya revalidado).
    pub fn aceptar(&mut self, a: &Aspirante, seleccion: Seleccion, ahora: Instante) -> Result<(), ErrorConvocatoria> {
        self.exigir_puede_unirse(a)?;
        self.integrar(a.heroe, seleccion, ahora);
        Ok(())
    }

    fn integrar(&mut self, heroe: HeroeId, seleccion: Seleccion, ahora: Instante) {
        self.peticiones.retain(|p| p.heroe != heroe);
        self.integrantes.push(Integrante { heroe, unido_en: ahora, seleccion });
    }

    /// Cada integrante, también el Líder, cambia **solo la suya**: el orden de sus escuadras es el orden con el que su columna sale y combate.
    pub fn cambiar_seleccion(&mut self, heroe: HeroeId, seleccion: Seleccion) -> Result<(), ErrorConvocatoria> {
        let i = self.integrantes.iter_mut().find(|i| i.heroe == heroe).ok_or(ErrorConvocatoria::NoEstas)?;
        i.seleccion = seleccion;
        Ok(())
    }

    /// Un integrante se va mientras se prepara: sigue dentro del lugar, no hay nada que devolver. El Líder cancela.
    pub fn separarse(&mut self, heroe: HeroeId) -> Result<(), ErrorConvocatoria> {
        if !self.integrantes.iter().any(|i| i.heroe == heroe) {
            return Err(ErrorConvocatoria::NoEstas);
        }
        if self.lider == heroe {
            return Err(ErrorConvocatoria::ElLiderCancela);
        }
        self.integrantes.retain(|i| i.heroe != heroe);
        Ok(())
    }

    /// `cancelarConvocatoria`: solo el Líder; no mueve nada. Devuelve a quién avisar: los integrantes y también los que tenían una petición.
    pub fn cancelar(&self, lider: HeroeId) -> Result<Vec<HeroeId>, ErrorConvocatoria> {
        if self.lider != lider {
            return Err(ErrorConvocatoria::SoloElLider);
        }
        Ok(self.integrantes.iter().map(|i| i.heroe).chain(self.peticiones.iter().map(|p| p.heroe)).collect())
    }

    /// Los integrantes que siguen dentro del lugar (`dentro`): quien se fue o se desconectó no sale con los demás.
    pub fn integrantes_dentro(&self, dentro: &dyn Fn(HeroeId) -> bool) -> Vec<&Integrante> {
        self.integrantes.iter().filter(|i| dentro(i.heroe)).collect()
    }

    /// `partirConvocatoria`: solo el Líder, que debe ser el **primero** de los que siguen dentro. Cada integrante de los de dentro, en orden,
    /// tiene que poder salir con lo que eligió (`puede_salir`); si **alguien** no puede, no sale nadie y se dice quién y por qué. Si todo va
    /// bien devuelve los que salen, el Líder primero. Los que ya no estaban dentro se quedan fuera de la salida.
    pub fn partir<E>(&self, quien: HeroeId, dentro: &dyn Fn(HeroeId) -> bool, puede_salir: &mut dyn FnMut(&Integrante) -> Result<(), E>) -> Result<Vec<&Integrante>, ErrorPartir<E>> {
        if self.lider != quien {
            return Err(ErrorPartir::SoloElLider);
        }
        let salen = self.integrantes_dentro(dentro);
        if salen.first().map(|i| i.heroe) != Some(self.lider) {
            return Err(ErrorPartir::LiderFuera);
        }
        for i in &salen {
            puede_salir(i).map_err(|motivo| ErrorPartir::NoPuedeSalir { heroe: i.heroe, motivo })?;
        }
        Ok(salen)
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use base::{Punto, MS_POR_MINUTO as MIN};

    fn r() -> Reglas {
        Reglas::cargar()
    }

    const PLAZA: Lugar = Lugar::Asentamiento(4);

    fn sel(escuadras: &[EscuadraId]) -> Seleccion {
        Seleccion { escuadras: escuadras.to_vec(), carga: Carga::new() }
    }

    fn aspirante(heroe: HeroeId) -> Aspirante {
        Aspirante { heroe, lugar: Some(PLAZA), faccion: Some(1), ya_en_convocatoria: false }
    }

    fn convocada(politica: Politica) -> Convocatoria {
        Convocatoria::convocar(&[], 1, 7, Some(PLAZA), Some(1), politica, sel(&[1, 2]), 0).unwrap()
    }

    #[test]
    fn el_lugar_de_un_heroe_es_la_plaza_o_el_campamento_donde_esta_dentro() {
        assert_eq!(lugar_de(Ubicacion::Asentamiento(4)), Some(PLAZA));
        assert_eq!(lugar_de(Ubicacion::Mercenarios(2)), Some(Lugar::Campamento(2)));
        assert_eq!(lugar_de(Ubicacion::Columna(9)), None, "una columna en el mapa no es un lugar");
        assert_eq!(lugar_de(Ubicacion::Desconectado(Punto::default())), None);
        assert_eq!((lugar_de(Ubicacion::Pueblo(7)), lugar_de(Ubicacion::Avanzada(8))), (None, None), "en un pueblo o una avanzada no se convoca");
    }

    #[test]
    fn convocar_crea_la_convocatoria_con_el_lider_como_unico_integrante() {
        let c = convocada(Politica::Preguntar);
        assert_eq!((c.lider, c.lugar, c.faccion, c.politica), (7, PLAZA, 1, Politica::Preguntar));
        assert_eq!(c.integrantes, vec![Integrante { heroe: 7, unido_en: 0, seleccion: sel(&[1, 2]) }]);
        assert!(c.peticiones.is_empty());
    }

    #[test]
    fn las_condiciones_de_convocar() {
        let otras = [convocada(Politica::Aceptar)];
        let conv = |lugar, faccion, politica, heroe| Convocatoria::convocar(&otras, 2, heroe, lugar, faccion, politica, Seleccion::default(), 0);
        assert_eq!(conv(None, Some(1), Politica::Aceptar, 8), Err(ErrorConvocatoria::FueraDeUnLugar));
        assert_eq!(conv(Some(PLAZA), None, Politica::Aceptar, 8), Err(ErrorConvocatoria::SinFaccion));
        assert_eq!(conv(Some(PLAZA), Some(1), Politica::Aceptar, 7), Err(ErrorConvocatoria::YaEnConvocatoria), "como mucho en una");
        assert_eq!(conv(Some(PLAZA), Some(1), Politica::Rechazar, 8), Err(ErrorConvocatoria::PoliticaInvalida));
        assert!(conv(Some(Lugar::Campamento(3)), Some(1), Politica::Preguntar, 8).is_ok());
        assert_eq!(convocatoria_de(&otras, 7).map(|c| c.id), Some(1));
        assert!(convocatoria_de(&otras, 8).is_none());
    }

    #[test]
    fn con_aceptar_se_une_ya_con_su_seleccion() {
        let r = r();
        let mut c = convocada(Politica::Aceptar);
        assert_eq!(c.unirse(&r, &aspirante(8), sel(&[3]), 2 * MIN), Ok(Union::Unido));
        assert_eq!(c.integrantes.iter().map(|i| (i.heroe, i.unido_en)).collect::<Vec<_>>(), vec![(7, 0), (8, 2 * MIN)]);
        assert_eq!(c.integrantes[1].seleccion, sel(&[3]));
    }

    #[test]
    fn solo_se_une_quien_esta_dentro_del_mismo_lugar_y_es_de_la_misma_faccion() {
        let r = r();
        let mut c = convocada(Politica::Aceptar);
        assert_eq!(c.unirse(&r, &Aspirante { lugar: Some(Lugar::Asentamiento(5)), ..aspirante(8) }, sel(&[]), 0), Err(ErrorConvocatoria::OtroLugar));
        assert_eq!(c.unirse(&r, &Aspirante { lugar: Some(Lugar::Campamento(4)), ..aspirante(8) }, sel(&[]), 0), Err(ErrorConvocatoria::OtroLugar), "un campamento no es una plaza");
        assert_eq!(c.unirse(&r, &Aspirante { lugar: None, ..aspirante(8) }, sel(&[]), 0), Err(ErrorConvocatoria::OtroLugar), "en una columna no se une");
        assert_eq!(c.unirse(&r, &Aspirante { faccion: Some(2), ..aspirante(8) }, sel(&[]), 0), Err(ErrorConvocatoria::OtraFaccion), "ni un aliado");
        assert_eq!(c.unirse(&r, &Aspirante { faccion: None, ..aspirante(8) }, sel(&[]), 0), Err(ErrorConvocatoria::OtraFaccion));
        assert_eq!(c.unirse(&r, &Aspirante { ya_en_convocatoria: true, ..aspirante(8) }, sel(&[]), 0), Err(ErrorConvocatoria::YaEnConvocatoria));
        assert_eq!(c.integrantes.len(), 1, "los rechazos no dejan rastro");
    }

    #[test]
    fn con_preguntar_queda_una_peticion_de_diez_segundos_que_barre_las_caducadas() {
        let r = r();
        let mut c = convocada(Politica::Preguntar);
        assert_eq!(c.unirse(&r, &aspirante(8), sel(&[3]), 0), Ok(Union::Pedida { expira_en: 10_000 }));
        assert_eq!(c.unirse(&r, &aspirante(9), sel(&[4]), 4_000), Ok(Union::Pedida { expira_en: 14_000 }));
        assert_eq!(c.integrantes.len(), 1, "preguntar no une todavía");
        // La del 8 caducó; la del 9 se renueva; el 10 entra en la lista.
        c.unirse(&r, &aspirante(9), sel(&[5]), 12_000).unwrap();
        c.unirse(&r, &aspirante(10), sel(&[6]), 12_500).unwrap();
        assert_eq!(c.peticiones.iter().map(|p| (p.heroe, p.pedido_en, p.seleccion.clone())).collect::<Vec<_>>(), vec![(9, 12_000, sel(&[5])), (10, 12_500, sel(&[6]))]);
    }

    #[test]
    fn el_lider_contesta_con_la_peticion_viva_y_aceptar_la_convierte_en_integrante() {
        let r = r();
        let mut c = convocada(Politica::Preguntar);
        c.unirse(&r, &aspirante(8), sel(&[3]), 0).unwrap();
        assert_eq!(c.retirar_peticion(9, 8, 1_000), Err(ErrorConvocatoria::SoloElLider));
        assert_eq!(c.retirar_peticion(7, 99, 1_000), Err(ErrorConvocatoria::NoHayPeticion));
        assert_eq!(c.retirar_peticion(7, 8, 10_000), Err(ErrorConvocatoria::PeticionCaducada), "el silencio es un no");
        let p = c.retirar_peticion(7, 8, 9_999).unwrap();
        assert_eq!(p.seleccion, sel(&[3]));
        assert!(c.peticiones.is_empty());
        assert_eq!(c.aceptar(&aspirante(8), p.seleccion, 9_999), Ok(()));
        assert_eq!(c.integrantes.len(), 2);
        // Aceptar revalida: si ya no está dentro, no entra.
        assert_eq!(c.aceptar(&Aspirante { lugar: None, ..aspirante(11) }, sel(&[]), 0), Err(ErrorConvocatoria::OtroLugar));
    }

    #[test]
    fn rechazar_una_peticion_la_quita_sin_unir_a_nadie() {
        let r = r();
        let mut c = convocada(Politica::Preguntar);
        c.unirse(&r, &aspirante(8), sel(&[3]), 0).unwrap();
        c.retirar_peticion(7, 8, 1_000).unwrap();
        assert_eq!((c.integrantes.len(), c.peticiones.len()), (1, 0));
    }

    #[test]
    fn cada_integrante_cambia_solo_su_seleccion_en_el_orden_que_quiere() {
        let r = r();
        let mut c = convocada(Politica::Aceptar);
        c.unirse(&r, &aspirante(8), sel(&[3, 4]), 0).unwrap();
        c.cambiar_seleccion(8, sel(&[4, 3])).unwrap();
        c.cambiar_seleccion(7, sel(&[2, 1])).unwrap();
        assert_eq!((c.integrantes[0].seleccion.clone(), c.integrantes[1].seleccion.clone()), (sel(&[2, 1]), sel(&[4, 3])));
        assert_eq!(c.cambiar_seleccion(99, sel(&[])), Err(ErrorConvocatoria::NoEstas));
    }

    #[test]
    fn un_integrante_se_separa_el_lider_cancela() {
        let r = r();
        let mut c = convocada(Politica::Aceptar);
        c.unirse(&r, &aspirante(8), sel(&[3]), 0).unwrap();
        assert_eq!(c.separarse(7), Err(ErrorConvocatoria::ElLiderCancela));
        assert_eq!(c.separarse(99), Err(ErrorConvocatoria::NoEstas));
        assert_eq!(c.separarse(8), Ok(()));
        assert_eq!(c.integrantes.len(), 1);
        assert_eq!(c.separarse(8), Err(ErrorConvocatoria::NoEstas));
    }

    #[test]
    fn cancelar_es_del_lider_y_avisa_tambien_a_quienes_tenian_una_peticion() {
        let r = r();
        let mut c = convocada(Politica::Preguntar);
        c.unirse(&r, &aspirante(8), sel(&[]), 0).unwrap();
        c.aceptar(&aspirante(9), sel(&[]), 0).unwrap();
        assert_eq!(c.cancelar(9), Err(ErrorConvocatoria::SoloElLider));
        assert_eq!(c.cancelar(7), Ok(vec![7, 9, 8]), "integrantes y, después, los de las peticiones");
    }

    #[test]
    fn partir_saca_a_los_que_siguen_dentro_con_el_lider_primero() {
        let r = r();
        let mut c = convocada(Politica::Aceptar);
        for h in [8, 9, 10] {
            c.unirse(&r, &aspirante(h), sel(&[h]), 0).unwrap();
        }
        let fuera_el_9 = |h: HeroeId| h != 9;
        let salen = c.partir::<()>(7, &fuera_el_9, &mut |_| Ok(())).unwrap();
        assert_eq!(salen.iter().map(|i| i.heroe).collect::<Vec<_>>(), vec![7, 8, 10], "quien ya no está dentro se queda fuera de la salida");
        assert_eq!(c.partir::<()>(8, &|_| true, &mut |_| Ok(())), Err(ErrorPartir::SoloElLider));
        assert_eq!(c.partir::<()>(7, &|h| h != 7, &mut |_| Ok(())), Err(ErrorPartir::LiderFuera), "el Líder tiene que seguir dentro");
    }

    #[test]
    fn partir_es_todo_o_nada_y_dice_quien_no_puede_y_por_que() {
        let r = r();
        let mut c = convocada(Politica::Aceptar);
        for h in [8, 9] {
            c.unirse(&r, &aspirante(h), sel(&[h]), 0).unwrap();
        }
        let mut probados = vec![];
        let res = c.partir(7, &|_| true, &mut |i| {
            probados.push(i.heroe);
            if i.heroe == 8 { Err("su Liderazgo no llega") } else { Ok(()) }
        });
        assert_eq!(res, Err(ErrorPartir::NoPuedeSalir { heroe: 8, motivo: "su Liderazgo no llega" }));
        assert_eq!(probados, vec![7, 8], "se prueba en orden y se para en el primero que falla");
        assert_eq!(c.integrantes.len(), 3, "la convocatoria no cambia: nadie ha salido");
    }
}
