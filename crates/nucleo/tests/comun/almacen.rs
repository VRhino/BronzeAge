//! Lo que un cliente guarda de sus ámbitos, como lo haría Unity con su almacén normalizado (contrato 05 §5 y §7): cada `Foto` sustituye al ámbito, cada `Delta` se aplica sobre él (la plaza
//! parcial con su máscara) y tiene que empezar donde acabó la anterior, cada `Revocado` lo descarta; y los hechos que le llegaron. Con él las pruebas comparan «foto + deltas» con una foto
//! nueva y reanudan con lo que el cliente conserva (`Hola.versiones_conocidas` y `versiones_interiores_conocidas`).

use contrato::v1::{self as pb, entidad::Cuerpo, mensaje_de_servidor as ms, TipoCambio, TipoCliente, TipoEntidad};
use nucleo::proyeccion::{interior, vista};
use std::collections::BTreeMap;

/// La identidad de una entidad en el almacén: su clase del contrato y su id.
pub type Clave = (i32, String);

/// Un ámbito tal como lo tiene el cliente.
#[derive(Clone, Default, Debug, PartialEq)]
pub struct Ambito {
    pub version: u64,
    pub entidades: BTreeMap<Clave, Cuerpo>,
    pub niebla: Option<pb::NieblaProyectada>,
}

/// Todos los ámbitos del cliente, por su clave `TIPO:id`, y los hechos que le llegaron (con el ámbito por el que llegaron).
#[derive(Clone, Default, Debug)]
pub struct Almacen {
    pub ambitos: BTreeMap<String, Ambito>,
    pub hechos: Vec<(String, pb::EventoDominio)>,
}

/// La clave `TIPO:id` de un ámbito del contrato.
pub fn clave_de_ambito(a: &Option<pb::Ambito>) -> String {
    let a = a.as_ref().expect("un ámbito");
    format!("{}:{}", pb::AmbitoTipo::try_from(a.tipo).unwrap().as_str_name().trim_start_matches("AMBITO_"), a.id)
}

/// La clave de una entidad en el almacén.
pub fn clave(c: &Cuerpo) -> Clave {
    let (t, id) = match c {
        Cuerpo::Ejercito(x) => (TipoEntidad::Ejercito, x.id.clone()),
        Cuerpo::Asentamiento(x) => (TipoEntidad::Asentamiento, x.id.clone()),
        Cuerpo::Faccion(x) => (TipoEntidad::Faccion, x.id.clone()),
        Cuerpo::Tecnologias(x) => (TipoEntidad::TecnologiaFaccion, x.faccion_id.clone()),
        Cuerpo::Heroe(x) => (TipoEntidad::Heroe, x.id.clone()),
        Cuerpo::InformeInspeccion(x) => (TipoEntidad::InformeInspeccion, x.id.clone()),
        Cuerpo::Escuadron(x) => (TipoEntidad::Escuadron, x.id.clone()),
        Cuerpo::Caravana(x) => (TipoEntidad::Caravana, x.id.clone()),
        Cuerpo::Convocatoria(x) => (TipoEntidad::Convocatoria, x.id.clone()),
        Cuerpo::AcuerdoTrueque(x) => (TipoEntidad::AcuerdoTrueque, x.id.clone()),
        Cuerpo::OrdenMercado(x) => (TipoEntidad::OrdenMercado, x.id.clone()),
        Cuerpo::Batalla(x) => (TipoEntidad::Batalla, x.id.clone()),
        Cuerpo::Relacion(x) => (TipoEntidad::RelacionPolitica, x.id.clone()),
        Cuerpo::Pueblo(x) => (TipoEntidad::Pueblo, x.id.clone()),
        Cuerpo::Obra(x) => (TipoEntidad::ObraDeMapa, x.id.clone()),
        Cuerpo::Avanzada(x) => (TipoEntidad::Avanzada, x.id.clone()),
        Cuerpo::CampamentoMercenarios(x) => (TipoEntidad::CampamentoMercenarios, x.id.clone()),
        Cuerpo::CampamentoBandidos(x) => (TipoEntidad::CampamentoBandidos, x.id.clone()),
        Cuerpo::FichaConocida(x) => (TipoEntidad::FichaConocida, x.asentamiento_id.clone()),
        Cuerpo::ZonaFaccion(x) => (TipoEntidad::ZonaFaccion, x.faccion_id.clone()),
        Cuerpo::RedCaminos(_) => (TipoEntidad::RedCaminos, "red".into()),
        Cuerpo::Mirada(x) => (TipoEntidad::MiradaIntel, x.id.clone()),
        Cuerpo::Informe(x) => (TipoEntidad::InformePlaza, x.asentamiento_id.clone()),
        otro => panic!("entidad sin clave en el almacén de prueba: {otro:?}"),
    };
    (t as i32, id)
}

impl Almacen {
    /// Aplica un mensaje del servidor (lo que no es de un ámbito no cambia nada).
    pub fn aplicar(&mut self, m: &ms::Cuerpo) {
        match m {
            ms::Cuerpo::Foto(f) => {
                let entidades = f.entidades.iter().filter_map(|e| e.cuerpo.clone()).map(|c| (clave(&c), c)).collect();
                self.ambitos.insert(clave_de_ambito(&f.ambito), Ambito { version: f.version, entidades, niebla: f.niebla.clone() });
            }
            ms::Cuerpo::Delta(d) => {
                let k = clave_de_ambito(&d.ambito);
                let a = self.ambitos.get_mut(&k).unwrap_or_else(|| panic!("una Delta de {k}, que el cliente no tiene"));
                assert_eq!(d.desde, a.version, "la Delta de {k} no empieza donde acabó la anterior (hueco)");
                assert!(d.hasta >= d.desde, "la Delta de {k} va hacia atrás");
                for c in &d.cambios {
                    let e = (c.entidad, c.id.clone());
                    let datos = c.datos.clone().and_then(|x| x.cuerpo);
                    if c.tipo == TipoCambio::CambioQuita as i32 {
                        // Lo que nació y desapareció dentro del tramo (o del hueco de quien reanuda con algo viejo) se quita sin que el cliente lo llegara a tener: no es un error.
                        a.entidades.remove(&e);
                    } else if c.reemplaza {
                        a.entidades.insert(e, datos.expect("ACTUALIZA con datos"));
                    } else {
                        let (Some(Cuerpo::Asentamiento(destino)), Some(Cuerpo::Asentamiento(parcial))) = (a.entidades.get_mut(&e), datos) else { panic!("en {k} solo la plaza viaja parcial, y sobre una que el cliente tiene: {e:?}") };
                        interior::aplicar(destino, &parcial, &c.campos);
                    }
                }
                if let Some(x) = &d.niebla {
                    vista::aplicar_niebla(&mut a.niebla, x);
                }
                a.version = d.hasta;
                self.hechos.extend(d.eventos.iter().map(|ev| (k.clone(), ev.clone())));
            }
            ms::Cuerpo::Revocado(r) => {
                let k = clave_de_ambito(&r.ambito);
                assert!(self.ambitos.remove(&k).is_some(), "Revocado de {k}, que el cliente no tenía");
            }
            _ => {}
        }
    }

    pub fn aplicar_todo<'a>(&mut self, mensajes: impl IntoIterator<Item = &'a ms::Cuerpo>) {
        for m in mensajes {
            self.aplicar(m);
        }
    }

    /// El `Hola` de quien reanuda con lo que conserva: la versión de cada ámbito (salvo `VISTA`, que no se reanuda) y la secuencia interior de cada plaza que tiene con su interior (la de su
    /// `ASENTAMIENTO` o, en `ADMIN`, todas), con la clave `ASENTAMIENTO:as-N`.
    pub fn hola(&self, token: &str, tipo: TipoCliente) -> pb::Hola {
        let mut h = servidor::prueba::hola(token, tipo);
        for (k, a) in self.ambitos.iter().filter(|(k, _)| !k.starts_with("VISTA:")) {
            h.versiones_conocidas.insert(k.clone(), a.version);
            if k.starts_with("ASENTAMIENTO:") || k == "ADMIN:" {
                for c in a.entidades.values() {
                    if let Cuerpo::Asentamiento(p) = c {
                        h.versiones_interiores_conocidas.insert(format!("ASENTAMIENTO:{}", p.id), p.version_interior);
                    }
                }
            }
        }
        h
    }

    /// Lo que tiene, para compararlo con una foto nueva: por ámbito, sus entidades y su niebla (la versión de una foto es la de hoy y la del almacén la del último cambio del ámbito). Dos cosas
    /// dependen de cuándo se mandaron y no de lo que hay: la secuencia interior de una plaza (avanza sin mensaje cuando un paso de minuto no cambia nada visible) y, en `VISTA`, el tramo y el sitio
    /// de lo ajeno que anda (el cliente sigue con el tramo que se le mandó mientras le dure).
    pub fn contenido(&self) -> BTreeMap<String, (BTreeMap<Clave, Cuerpo>, Option<pb::NieblaProyectada>)> {
        self.ambitos
            .iter()
            .map(|(k, a)| {
                let vista = k.starts_with("VISTA:");
                let entidades = a.entidades.iter().map(|(c, e)| (c.clone(), normalizada(e.clone(), vista))).collect();
                (k.clone(), (entidades, a.niebla.clone()))
            })
            .collect()
    }
}

fn normalizada(mut e: Cuerpo, vista: bool) -> Cuerpo {
    match &mut e {
        Cuerpo::Asentamiento(x) => x.version_interior = 0,
        Cuerpo::Pueblo(x) => x.version_interior = 0,
        Cuerpo::Ejercito(x) if vista && x.lider_id.is_empty() => (x.plan, x.posicion) = (None, None),
        Cuerpo::Caravana(x) if vista && x.contenido.is_none() => (x.plan, x.posicion) = (None, None),
        _ => {}
    }
    e
}
