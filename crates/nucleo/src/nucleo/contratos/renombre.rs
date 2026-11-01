//! Los hechos que dan renombre y cuentan medallas, la valoración y la medalla Leal (Mercenarios_Por_Contrato §8.2 y §9). Tarea T9 del plan.
//!
//! # Cuándo se apunta (duda 2 del plan)
//! - **Temporal**: lo que gana bajo contrato se guarda en el `renombre` de su contrato ([`Nucleo::ganar`]) y se apunta a la ficha al acabar ([`Nucleo::apuntar_al_acabar`]); con despido se
//!   pierde. Una valoración posterior ([`Nucleo::valorar_mercenario`]) suma la mitad otra vez si es buena o quita lo apuntado si es mala.
//! - **Permanente**: solo acaba por despido y entonces nunca sumaría, así que apunta en el momento.
//! - **Medallas**: su renombre (el del grado que cruzan) se apunta siempre en el momento, por [`Nucleo::apuntar_medalla`].
//!
//! # Quién gana qué
//! Solo un marcado, y por lo que hace con la orden que tiene. Al resolverse una batalla ([`Nucleo::renombre_de_batalla`], que llama `tras_batalla` antes de cerrar las órdenes):
//! quien ataca con su orden `atacar` y gana el ataque, el renombre de lo atacado; quien destruye unos bandidos, Cazador de bandidos (con o sin esa orden); quien defiende con su orden
//! `guardar` lo atacado y lo defiende, `defensaGanada` y Muralla. Al llegar una caravana a su destino ([`Nucleo::escolta_llega`]), quien la escoltaba con su orden `guardar`: `escoltaLlegada` y
//! Buen camino. Quien acompañaba a un ejército o a una convocatoria con su orden `unirse` y siguió hasta que se deshizo o se canceló ([`Nucleo::campana`]), Campaña. Y el permanente cuenta
//! los días que lleva firmado ([`Nucleo::cita_leal`]), con una cita en la cola por cada umbral de Leal.
//!
//! Los hechos de rango y de medalla los cuentan `apuntar_renombre` y `apuntar_medalla`; el único hecho propio es `mercenarios.valorado`.

use super::*;
use ::mercenarios::contrato::duracion;
use ::mercenarios::progresion::{extra_por_buena, quita_por_mala, renombre_de_ataque};
use ::mercenarios::{Contrato, Fin, Medalla};
use militar::escuadra::Escuadra;

/// Cuánto después de su firma cumple `dias` días un permanente: la distancia a la que se cita cada umbral de Leal.
fn leal_a_los(dias: u32) -> Instante {
    Instante::from(dias) * duracion(1)
}

impl Nucleo {
    // ───────────────────────────── Ayudas ─────────────────────────────

    /// Renombre ganado bajo contrato (§8.2, duda 2): el temporal lo guarda en su contrato, que lo apunta al acabar; el permanente lo apunta en el momento. Quien no tiene contrato no gana nada.
    fn ganar(&mut self, t: Instante, heroe: u64, n: u32) {
        let Some(c) = self.estado.contratos.contratos.get_mut(&heroe) else { return };
        if c.trabajo == Trabajo::Permanente {
            self.apuntar_renombre(t, heroe, n);
        } else {
            c.renombre += n;
            let faccion = c.faccion;
            self.tr.tocar(Clase::Heroe, heroe);
            self.tr.tocar(Clase::Faccion, u64::from(faccion));
        }
    }

    /// ¿Tiene `heroe` vigente la orden `tipo` sobre `objetivo`?
    fn ordenado(&self, heroe: u64, tipo: TipoOrden, objetivo: &Objetivo) -> bool {
        self.orden_de(heroe).is_some_and(|o| o.tipo == tipo && o.objetivo == *objetivo)
    }

    // ───────────────────────────── Los hechos ─────────────────────────────

    /// Lo que da una batalla resuelta a los mercenarios que estuvieron en ella (§8.2 y §9), antes de que [`Nucleo::tras_batalla`] cierre sus órdenes. Solo se mira a los héroes del
    /// desenlace y su contrato.
    /// - Si ganó el atacante: quien atacaba con `atacar` de lo atacado gana el renombre de ese ataque, y Asaltante si era un pueblo, una avanzada o una plaza; quien atacaba unos
    ///   bandidos, con o sin esa orden, cuenta Cazador de bandidos.
    /// - Si no ganó: quien defendía con `guardar` de lo atacado gana `defensaGanada` y cuenta Muralla.
    pub(super) fn renombre_de_batalla(&mut self, t: Instante, d: &Desenlace) {
        let defensa_ganada = self.estado.reglas.economia.contratos.renombre.defensa_ganada;
        if d.gana_atacante {
            let bandidos = d.atacados.iter().any(|o| matches!(o, Objetivo::Bandidos(_)));
            for &h in d.atacantes {
                if !self.es_marcado(h) {
                    continue;
                }
                if let Some(objetivo) = d.atacados.iter().find(|o| self.ordenado(h, TipoOrden::Atacar, o)) {
                    let n = renombre_de_ataque(&self.estado.reglas.economia.contratos, objetivo, d.nivel_bandidos);
                    self.ganar(t, h, n);
                    if matches!(objetivo, Objetivo::Pueblo(_) | Objetivo::Avanzada(_) | Objetivo::Plaza(_)) {
                        self.apuntar_medalla(t, h, Medalla::Asaltante, 1);
                    }
                }
                if bandidos {
                    self.apuntar_medalla(t, h, Medalla::CazadorDeBandidos, 1);
                }
            }
        } else {
            for &h in d.defensores {
                if d.atacados.iter().any(|o| self.ordenado(h, TipoOrden::Guardar, o)) {
                    self.ganar(t, h, defensa_ganada);
                    self.apuntar_medalla(t, h, Medalla::Muralla, 1);
                }
            }
        }
    }

    /// La caravana `caravana` llega a su destino (§8.2 y §9): quienes la escoltan con su orden `guardar` de ella ganan `escoltaLlegada` y cuentan Buen camino. Escoltan los héroes de las
    /// escuadras que le han cedido; la que va adjunta a una columna no viaja sola y no llega.
    pub(in crate::nucleo) fn escolta_llega(&mut self, t: Instante, caravana: u64) {
        let Some(k) = self.estado.comercio.caravana(caravana) else { return };
        let cedidas: Vec<Escuadra> = k.escolta.iter().filter_map(|c| self.escuadra(c.id).map(|(_, s)| s.clone())).collect();
        let escolta = self.heroes_de_escolta(&cedidas);
        let llegada = self.estado.reglas.economia.contratos.renombre.escolta_llegada;
        for h in escolta {
            if self.ordenado(h, TipoOrden::Guardar, &Objetivo::Caravana(caravana)) {
                self.ganar(t, h, llegada);
                self.apuntar_medalla(t, h, Medalla::BuenCamino, 1);
            }
        }
    }

    /// Un ejército se deshace (se desarma a la puerta de un lugar o se queda sin nadie) o una convocatoria se cancela (`objetivo`): cada uno de `siguieron` que lo acompañaba con su orden
    /// `unirse` y estuvo dentro cuenta una Campaña (§9). Se llama antes de que la orden se cierre: si hubiera salido antes, ya habría caducado (`unido` es que estuvo dentro).
    pub(in crate::nucleo) fn campana(&mut self, t: Instante, siguieron: &[u64], objetivo: Objetivo) {
        for &h in siguieron {
            if self.orden_de(h).is_some_and(|o| o.tipo == TipoOrden::Unirse && o.objetivo == objetivo && o.unido) {
                self.apuntar_medalla(t, h, Medalla::Campana, 1);
            }
        }
    }

    // ───────────────────────────── El fin del contrato ─────────────────────────────

    /// Lo que el contrato `c`, que acaba por `fin`, apunta a la ficha (§8.2, duda 2): devuelve el renombre apuntado y si contó Palabra cumplida. El temporal que acaba sin despido apunta el
    /// renombre de su contrato (que ya trae el de sus plazos) y, si acaba por su plazo, cuenta Palabra cumplida. Con despido no apunta nada, y el permanente tampoco: ya lo apuntó al ganarlo.
    pub(super) fn apuntar_al_acabar(&mut self, t: Instante, c: &Contrato, fin: Fin) -> (u32, bool) {
        if c.trabajo == Trabajo::Permanente || fin == Fin::Despido {
            return (0, false);
        }
        self.apuntar_renombre(t, c.mercenario, c.renombre);
        let palabra = fin == Fin::Plazo;
        if palabra {
            self.apuntar_medalla(t, c.mercenario, Medalla::PalabraCumplida, 1);
        }
        (c.renombre, palabra)
    }

    // ───────────────────────────── La valoración ─────────────────────────────

    /// `valorar_mercenario` (§8.2, duda 2): quien firmó un contrato o el Rey de su Facción lo valora mientras queda abierto, una sola vez. Una buena suma lo que añade `buenaPct` a lo apuntado
    /// y cuenta Bien valorado; una mala le quita a la ficha lo que el contrato dejó de valer y la Palabra cumplida que contó, si la contó (el grado ganado se queda). Un despido apuntó
    /// 0: valorarlo no cambia el renombre.
    pub(super) fn valorar_mercenario(&mut self, t: Instante, p: &Peticion, heroe: u64, mercenario: u64, buena: bool) -> Result<Vec<u64>, Respuesta> {
        use rechazos::*;
        self.autorizar_heroe(p, heroe, false)?;
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: &str| Respuesta::rechazo(c, m, v);
        let faccion = self.heroe(heroe).and_then(|h| h.faccion);
        let abierto = faccion.and_then(|f| Some((f, *self.estado.contratos.terminados.get(&(mercenario, f))?))).filter(|(_, x)| !x.valorado && t < x.valorable_hasta);
        let (faccion, terminado) = abierto.ok_or_else(|| rechazo(MERCENARIO_SIN_VALORACION, "no hay un contrato de ese mercenario con tu Facción que valorar: no ha acabado, ya lo valoraron o se cerró el plazo"))?;
        if terminado.firmado_por != heroe && self.faccion(faccion).is_none_or(|f| f.rey != Some(heroe)) {
            return Err(rechazo(MERCENARIO_SIN_ALCANCE, "solo valora quien firmó el contrato o el Rey de la Facción"));
        }

        let d = &self.estado.reglas.economia.contratos;
        let (extra, quita) = (extra_por_buena(d, terminado.renombre), quita_por_mala(d, terminado.renombre));
        self.estado.contratos.terminados.get_mut(&(mercenario, faccion)).expect("lo acabamos de leer").valorado = true;
        let renombre = if buena {
            self.ficha_mut(mercenario).historial.buenas += 1;
            self.apuntar_renombre(t, mercenario, extra);
            self.apuntar_medalla(t, mercenario, Medalla::BienValorado, 1);
            extra
        } else {
            let ficha = self.ficha_mut(mercenario);
            let antes = ficha.renombre;
            ficha.historial.malas += 1;
            ficha.restar_renombre(quita);
            if let Some(cuenta) = ficha.contadores.get_mut(&Medalla::PalabraCumplida).filter(|_| terminado.palabra) {
                *cuenta = cuenta.saturating_sub(1);
            }
            antes - ficha.renombre
        };
        let datos = vec![("faccion", i64::from(faccion)), ("mercenario", mercenario as i64), ("valorador", heroe as i64), ("buena", i64::from(buena)), ("renombre", i64::from(renombre))];
        self.tr.emitir(&self.estado, EventoNucleo { heroes: vec![mercenario], facciones: vec![faccion], ..EventoNucleo::nuevo("mercenarios.valorado", t, datos) });
        Ok(vec![])
    }

    // ───────────────────────────── Leal ─────────────────────────────

    /// Al firmar un permanente en `firmado_en` se citan los días de Leal: una cita por cada umbral de la medalla, a ese número de días de la firma (§9).
    pub(super) fn citar_leal(&mut self, mercenario: u64, firmado_en: Instante) {
        for dias in self.estado.reglas.economia.contratos.umbrales(Medalla::Leal) {
            self.citar_contrato(mercenario, SucesoContrato::Leal, firmado_en + leal_a_los(dias));
        }
    }

    /// Llega una cita de Leal (`SucesoContrato::Leal`): si el contrato sigue siendo un permanente y es el mismo que la citó (su firma está a justo un umbral de días), cuenta los días que
    /// falten hasta ese umbral. El contador guarda la racha más larga y nunca baja: una racha anterior ya contada no se suma otra vez, y una cita vieja de un contrato que acabó (o que
    /// otro firmó después, a otra distancia) no encuentra su umbral y no cuenta nada (§9).
    pub(super) fn cita_leal(&mut self, mercenario: u64, t: Instante) {
        let Some(c) = self.estado.contratos.contratos.get(&mercenario).filter(|c| c.trabajo == Trabajo::Permanente) else { return };
        let firmado = t - c.firmado_en;
        let Some(dias) = self.estado.reglas.economia.contratos.umbrales(Medalla::Leal).into_iter().find(|&dias| leal_a_los(dias) == firmado) else { return };
        let cuenta = self.estado.contratos.fichas.get(&mercenario).and_then(|f| f.contadores.get(&Medalla::Leal)).copied().unwrap_or(0);
        if dias > cuenta {
            self.apuntar_medalla(t, mercenario, Medalla::Leal, dias - cuenta);
        }
    }
}
