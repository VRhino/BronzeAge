//! Las convocatorias dentro del núcleo (Presencia_Y_Convocatoria §9): un ejército **en preparación dentro** de una plaza o de un campamento, antes de
//! salir. Nadie ha salido todavía —las escuadras siguen en su campamento y el almacén intacto—, así que cancelar o separarse no devuelve nada, y la
//! convocatoria espera **sin caducidad** a que su Líder parta o cancele. Las reglas de quién convoca, quién se une y quién parte son de
//! `militar::convocatoria`; aquí se comprueba «saldría con lo que eligió» simulando la salida real de cada integrante ([`PlanDeSalida`]) y se
//! parte de verdad con esos mismos planes.
//!
//! # Partir es todo o nada
//! `partir` simula a todos los que siguen dentro, en orden, acumulando lo que gastan (`Reservas`): el almacén de la plaza lo comparten, y lo que saca el
//! primero no lo tiene el segundo. Si alguien no puede salir no sale nadie y se dice quién y por qué. Si todos pueden, el Líder sale como un ejército
//! quieto en la puerta con la política de la convocatoria y los demás se suman a él con lo que eligieron (un integrante de visita se suma con la
//! columna que dejó aparcada, que ha de poder fundirse con la del Líder: personal, de la misma Facción y sin formación).
//!
//! # Los plazos
//! Lo único con instante es la petición de unión de un integrante potencial (10 s con la política `Preguntar`, el silencio vale como un no): un evento
//! `CaducaPeticionDeConvocatoria` por petición, que se descarta al contestarla. La convocatoria en sí no tiene ninguno.

use super::*;
use crate::estado::{SucesoMilitar, PRIO_LLEGADA};
use crate::estado_militar::entidad_peticion_de_convocatoria;
use crate::nucleo::salidas::{Destino, Reservas};
use asentamiento::Recurso;
use militar::columna::TipoColumna;
use militar::convocatoria::{lugar_de, Aspirante, Convocatoria, ErrorConvocatoria, ErrorPartir, Lugar, Seleccion, Union};

fn rechazo_de_convocatoria(e: ErrorConvocatoria) -> (&'static str, &'static str) {
    use rechazos::*;
    match e {
        ErrorConvocatoria::FueraDeUnLugar => (CONVOCATORIA_FUERA_DE_UN_LUGAR, "se convoca desde dentro de una plaza o de un campamento"),
        ErrorConvocatoria::SinFaccion => (CONVOCATORIA_SIN_FACCION, "sin Facción no hay compañeros con quienes formar un ejército"),
        ErrorConvocatoria::YaEnConvocatoria => (CONVOCATORIA_YA_EN_ELLA, "ya vas en una convocatoria"),
        ErrorConvocatoria::PoliticaInvalida => (CONVOCATORIA_POLITICA_INVALIDA, "la política de una convocatoria es aceptar o preguntar"),
        ErrorConvocatoria::OtroLugar => (CONVOCATORIA_OTRO_LUGAR, "solo se une quien está dentro del mismo lugar"),
        ErrorConvocatoria::OtraFaccion => (CONVOCATORIA_OTRA_FACCION, "un ejército lo componen ciudadanos de una sola Facción"),
        ErrorConvocatoria::NoEstas => (CONVOCATORIA_NO_ESTAS, "no vas en esa convocatoria"),
        ErrorConvocatoria::ElLiderCancela => (CONVOCATORIA_EL_LIDER_CANCELA, "el Líder no se separa de su ejército: lo cancela"),
        ErrorConvocatoria::SoloElLider => (CONVOCATORIA_SOLO_EL_LIDER, "solo el Líder puede hacerlo"),
        ErrorConvocatoria::NoHayPeticion => (CONVOCATORIA_NO_HAY_PETICION, "no hay una petición de ese héroe"),
        ErrorConvocatoria::PeticionCaducada => (CONVOCATORIA_PETICION_CADUCADA, "la petición caducó: el silencio vale como un no"),
    }
}

impl Nucleo {
    /// Las líneas `(recurso, cantidad)` de una carga, en el orden del mapa.
    fn lineas_de(carga: &comercio::Carga) -> Vec<(Recurso, Mil)> {
        carga.iter().map(|(&r, &n)| (r, n)).collect()
    }

    /// Las plazas a las que se atribuye un hecho de una convocatoria: la del lugar si es una plaza, ninguna si es un campamento.
    fn plazas_del_lugar(lugar: Lugar) -> Vec<u64> {
        match lugar {
            Lugar::Asentamiento(a) => vec![a as u64],
            Lugar::Campamento(_) => vec![],
        }
    }

    fn lugar_del_heroe(&self, heroe: u64) -> Option<Lugar> {
        lugar_de(self.heroe(heroe)?.heroe.presencia.ubicacion)
    }

    /// La convocatoria en la que va el héroe, o el rechazo de que no va en ninguna.
    fn convocatoria_del_heroe(&self, heroe: u64) -> Result<&Convocatoria, Respuesta> {
        self.estado.militar.convocatoria_de(heroe).ok_or_else(|| Respuesta::rechazo(rechazos::CONVOCATORIA_NO_ESTAS, "no vas en ninguna convocatoria", self.estado.version))
    }

    /// Simula que el héroe saldría de su lugar con esta selección. No toca nada.
    fn comprobar_seleccion(&mut self, t: Instante, heroe: u64, lugar: Lugar, seleccion: &Seleccion) -> Result<(), Respuesta> {
        self.validar_salida(t, heroe, lugar, &seleccion.escuadras, &Self::lineas_de(&seleccion.carga), false, &mut Reservas::default()).map(|_| ())
    }

    fn lineas_a_seleccion(escuadras: &[u64], carga: &[(Recurso, Mil)]) -> Seleccion {
        Seleccion { escuadras: escuadras.to_vec(), carga: carga.iter().filter(|(_, n)| *n != 0).fold(comercio::Carga::new(), |mut m, &(r, n)| {
            *m.entry(r).or_insert(0) += n;
            m
        }) }
    }

    /// Las órdenes de convocatoria. `Ok(ids)` = cambió algo y creó esos ids; los rechazos salen como `Err` sin haber tocado el estado.
    pub(super) fn ejecutar_convocatorias(&mut self, t: Instante, p: &Peticion) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let de_convocatoria = |e: ErrorConvocatoria| respuesta_de(v, e);
        let mr = self.estado.reglas.economia.militar.clone();
        match &p.comando {
            Comando::ConvocarEjercito { heroe, escuadras, carga, politica } => {
                self.autorizar_heroe(p, *heroe, false)?;
                let hn = self.heroe(*heroe).expect("existe");
                let (lugar, faccion) = (self.lugar_del_heroe(*heroe), hn.faccion);
                let todas: Vec<Convocatoria> = self.estado.militar.convocatorias.values().cloned().collect();
                let seleccion = Self::lineas_a_seleccion(escuadras, carga);
                let id = self.estado.siguiente_id;
                let c = Convocatoria::convocar(&todas, id, *heroe, lugar, faccion, *politica, seleccion.clone(), t).map_err(de_convocatoria)?;
                self.comprobar_seleccion(t, *heroe, c.lugar, &seleccion)?;
                self.estado.siguiente_id += 1;
                self.emitir_propio(t, "convocatoria.convocada", Self::plazas_del_lugar(c.lugar), vec![("convocatoria", id as i64), ("lider", *heroe as i64), ("politica", crate::codec_columna::politica_a_u8(*politica) as i64)]);
                self.estado.militar.convocatorias.insert(id, c);
                Ok(vec![id])
            }
            Comando::UnirseAConvocatoria { heroe, convocatoria, escuadras, carga } => {
                self.autorizar_heroe(p, *heroe, false)?;
                let mut c = self.estado.militar.convocatorias.get(convocatoria).cloned().ok_or_else(|| Respuesta::rechazo(rechazos::CONVOCATORIA_DESCONOCIDA, format!("convocatoria-{convocatoria}"), v))?;
                let hn = self.heroe(*heroe).expect("existe");
                let aspirante = Aspirante { heroe: *heroe, lugar: self.lugar_del_heroe(*heroe), faccion: hn.faccion, ya_en_convocatoria: self.estado.militar.convocatoria_de(*heroe).is_some() };
                let seleccion = Self::lineas_a_seleccion(escuadras, carga);
                let union = c.unirse(&mr, &aspirante, seleccion.clone(), t).map_err(de_convocatoria)?;
                self.comprobar_seleccion(t, *heroe, c.lugar, &seleccion)?;
                let plazas = Self::plazas_del_lugar(c.lugar);
                self.estado.militar.convocatorias.insert(*convocatoria, c);
                match union {
                    Union::Unido => self.emitir_propio(t, "convocatoria.unido", plazas, vec![("convocatoria", *convocatoria as i64), ("heroe", *heroe as i64)]),
                    Union::Pedida { expira_en } => {
                        self.estado.generaciones.subir(entidad_peticion_de_convocatoria(*heroe));
                        let clave = self.estado.generaciones.clave(entidad_peticion_de_convocatoria(*heroe));
                        self.estado.cola.programar(expira_en, PRIO_LLEGADA, clave, Carga::Militar(SucesoMilitar::CaducaPeticionDeConvocatoria { convocatoria: *convocatoria }));
                        self.emitir_propio(t, "convocatoria.union_pedida", plazas, vec![("convocatoria", *convocatoria as i64), ("heroe", *heroe as i64), ("expiraEn", expira_en)]);
                    }
                }
                Ok(vec![])
            }
            Comando::ResponderPeticionDeConvocatoria { heroe, convocatoria, solicitante, aceptar } => {
                self.autorizar_heroe(p, *heroe, false)?;
                let mut c = self.estado.militar.convocatorias.get(convocatoria).cloned().ok_or_else(|| Respuesta::rechazo(rechazos::CONVOCATORIA_DESCONOCIDA, format!("convocatoria-{convocatoria}"), v))?;
                let peticion = c.retirar_peticion(*heroe, *solicitante, t).map_err(de_convocatoria)?;
                let plazas = Self::plazas_del_lugar(c.lugar);
                if *aceptar {
                    // Se revalida todo con la selección de la petición: sigue dentro del lugar, es de la misma Facción y puede salir.
                    let hn = self.heroe(*solicitante).ok_or_else(|| Respuesta::rechazo(rechazos::HEROE_DESCONOCIDO, format!("heroe-{solicitante}"), v))?;
                    let aspirante = Aspirante { heroe: *solicitante, lugar: self.lugar_del_heroe(*solicitante), faccion: hn.faccion, ya_en_convocatoria: self.estado.militar.convocatoria_de(*solicitante).is_some() };
                    c.aceptar(&aspirante, peticion.seleccion.clone(), t).map_err(de_convocatoria)?;
                    self.comprobar_seleccion(t, *solicitante, c.lugar, &peticion.seleccion)?;
                }
                self.estado.militar.convocatorias.insert(*convocatoria, c);
                self.estado.generaciones.subir(entidad_peticion_de_convocatoria(*solicitante));
                let codigo = if *aceptar { "convocatoria.unido" } else { "convocatoria.union_rechazada" };
                self.emitir_propio(t, codigo, plazas, vec![("convocatoria", *convocatoria as i64), ("heroe", *solicitante as i64)]);
                Ok(vec![])
            }
            Comando::CambiarSeleccionDeConvocatoria { heroe, escuadras, carga } => {
                self.autorizar_heroe(p, *heroe, false)?;
                let mut c = self.convocatoria_del_heroe(*heroe)?.clone();
                let seleccion = Self::lineas_a_seleccion(escuadras, carga);
                c.cambiar_seleccion(*heroe, seleccion.clone()).map_err(de_convocatoria)?;
                self.comprobar_seleccion(t, *heroe, c.lugar, &seleccion)?;
                let (id, plazas) = (c.id, Self::plazas_del_lugar(c.lugar));
                self.estado.militar.convocatorias.insert(id, c);
                self.emitir_propio(t, "convocatoria.seleccion_cambiada", plazas, vec![("convocatoria", id as i64), ("heroe", *heroe as i64)]);
                Ok(vec![])
            }
            Comando::SepararseDeConvocatoria { heroe } => {
                self.autorizar_heroe(p, *heroe, false)?;
                let mut c = self.convocatoria_del_heroe(*heroe)?.clone();
                c.separarse(*heroe).map_err(de_convocatoria)?;
                let (id, plazas) = (c.id, Self::plazas_del_lugar(c.lugar));
                self.estado.militar.convocatorias.insert(id, c);
                self.emitir_propio(t, "convocatoria.separado", plazas, vec![("convocatoria", id as i64), ("heroe", *heroe as i64)]);
                Ok(vec![])
            }
            Comando::CancelarConvocatoria { heroe } => {
                self.autorizar_heroe(p, *heroe, true)?;
                let c = self.convocatoria_del_heroe(*heroe)?.clone();
                let avisados = c.cancelar(*heroe).map_err(de_convocatoria)?;
                self.estado.militar.convocatorias.remove(&c.id);
                c.peticiones.iter().for_each(|x| {
                    self.estado.generaciones.subir(entidad_peticion_de_convocatoria(x.heroe));
                });
                self.emitir_propio(t, "convocatoria.cancelada", Self::plazas_del_lugar(c.lugar), vec![("convocatoria", c.id as i64), ("lider", *heroe as i64), ("avisados", avisados.len() as i64)]);
                Ok(vec![])
            }
            Comando::PartirConvocatoria { heroe } => self.partir_convocatoria(t, p, *heroe),
            _ => unreachable!("solo se llama con órdenes de convocatoria"),
        }
    }

    /// `partirConvocatoria` (§9.4): todo o nada.
    fn partir_convocatoria(&mut self, t: Instante, p: &Peticion, heroe: u64) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        self.autorizar_heroe(p, heroe, false)?;
        let c = self.convocatoria_del_heroe(heroe)?.clone();
        let dentro: Vec<u64> = c.integrantes.iter().map(|i| i.heroe).filter(|&h| self.heroe(h).is_some_and(|hn| !hn.heroe.presencia.fuera && lugar_de(hn.heroe.presencia.ubicacion) == Some(c.lugar))).collect();
        let (mut planes, mut reservas) = (Vec::new(), Reservas::default());
        let resultado = c.partir(heroe, &|h| dentro.contains(&h), &mut |i| {
            let plan = self.validar_salida(t, i.heroe, c.lugar, &i.seleccion.escuadras, &Self::lineas_de(&i.seleccion.carga), false, &mut reservas)?;
            // Un integrante de visita se suma con la columna que dejó aparcada: ha de poder fundirse con la del Líder.
            if let Some(retomada) = plan.retoma.and_then(|r| self.estado.ejercitos.get(&r)).filter(|_| i.heroe != c.lider) {
                let cc = &retomada.columna;
                let error = if cc.tipo != TipoColumna::Personal || cc.es_formacion() {
                    Some(rechazos::COLUMNA_SOLO_UNA_PERSONAL)
                } else if cc.faccion != Some(c.faccion) {
                    Some(rechazos::COLUMNA_OTRA_FACCION)
                } else {
                    None
                };
                if let Some(codigo) = error {
                    return Err(Respuesta::rechazo(codigo, "su columna aparcada no se puede fundir con el ejército", v));
                }
            }
            planes.push(plan);
            Ok(())
        });
        match resultado {
            Err(ErrorPartir::SoloElLider) => Err(respuesta_de(v, ErrorConvocatoria::SoloElLider)),
            Err(ErrorPartir::LiderFuera) => Err(Respuesta::rechazo(rechazos::CONVOCATORIA_LIDER_FUERA, "el Líder ya no está dentro del lugar", v)),
            Err(ErrorPartir::NoPuedeSalir { heroe: h, motivo: Respuesta { resultado: Resultado::Rechazada { codigo, mensaje }, .. } }) => {
                Err(Respuesta::rechazo(rechazos::CONVOCATORIA_NO_PUEDE_SALIR, format!("el héroe {h} no puede salir con lo elegido ({codigo}): {mensaje}"), v))
            }
            Err(ErrorPartir::NoPuedeSalir { .. }) => unreachable!("la simulación solo rechaza"),
            Ok(_) => {
                let (lider, otros) = planes.split_first().expect("el Líder sale el primero");
                let base = self.aplicar_salida(t, lider, Destino::Columna { tipo: TipoColumna::Ejercito, politica: c.politica });
                for plan in otros {
                    self.aplicar_salida(t, plan, Destino::Unirse(base));
                }
                self.estado.militar.convocatorias.remove(&c.id);
                c.peticiones.iter().for_each(|x| {
                    self.estado.generaciones.subir(entidad_peticion_de_convocatoria(x.heroe));
                });
                self.evento_de_columna("convocatoria.partio", t, base, Self::plazas_del_lugar(c.lugar), vec![("convocatoria", c.id as i64), ("heroes", planes.len() as i64)]);
                Ok(vec![base])
            }
        }
    }

    /// Caduca la petición de unión a una convocatoria que `solicitante` le hizo: se retira y se avisa.
    pub(super) fn caduca_peticion_de_convocatoria(&mut self, convocatoria: u64, solicitante: u64, t: Instante) {
        let Some(c) = self.estado.militar.convocatorias.get_mut(&convocatoria) else { return };
        let antes = c.peticiones.len();
        c.peticiones.retain(|x| !(x.heroe == solicitante && t >= x.expira_en));
        if c.peticiones.len() == antes {
            return;
        }
        let plazas = Self::plazas_del_lugar(c.lugar);
        self.emitir_propio(t, "convocatoria.peticion_caducada", plazas, vec![("convocatoria", convocatoria as i64), ("heroe", solicitante as i64)]);
    }
}

fn respuesta_de(version: u64, e: ErrorConvocatoria) -> Respuesta {
    let (c, m) = rechazo_de_convocatoria(e);
    Respuesta::rechazo(c, m, version)
}
