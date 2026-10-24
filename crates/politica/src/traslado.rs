//! `trasladarFaccion(origen → destino)` (Diplomacia_Facciones §6.3), la ruta única de la anexión y la fusión, como un **plan de cambios puros** sobre el
//! estado político ([`Plan`]) que se aplica de una vez ([`aplicar`]). Lo que no es político lo hace el núcleo en la misma transición (5F), en el mismo orden:
//! las plazas (con cargos locales y guarnición, sin la marca de capital del origen), las columnas, caravanas y flota, rutas, pueblos, avanzadas, convocatorias,
//! Aedas residentes y épicas cumplidas, Miradas e Informes, la memoria (se funde), la tecnología (se borra la del origen; `primeros` apunta al destino) y la
//! reputación (la del origen se pierde; en la fusión, la mayor de las dos).

use crate::faccion::{Derrota, Faccion};
use crate::propuestas::Union;
use crate::relaciones::TipoRelacion;
use crate::tipos::{FaccionId, HeroeId, PropuestaId, RelacionId};
use crate::{Politica, Reglas};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cambio {
    /// Un vasallaje cuya señora era el origen pasa al destino (misma vasalla, mismo tributo, mismo id).
    Heredado { relacion: RelacionId, vasalla: FaccionId },
    /// Cualquier otra relación activa del origen pasa a `rota`, sin reputación.
    Rota { relacion: RelacionId },
    /// Las propuestas que nombraban al origen se descartan.
    Descartada { propuesta: PropuestaId },
    /// Los ciudadanos del origen se suman al final de los del destino, sin repetir.
    Ciudadanos { nuevos: Vec<HeroeId> },
    /// `derrotadaPor` que apuntaba al origen apunta al destino.
    Derrota { faccion: FaccionId },
    /// El origen desaparece (con sus solicitudes, sus cargos de Facción, su sigilo y su experiencia).
    Desaparece,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub origen: FaccionId,
    pub destino: FaccionId,
    pub cambios: Vec<Cambio>,
}

/// El plan del traslado: un vasallaje con el origen de señora pasa al destino **salvo** que la vasalla sea el destino o el destino ya tenga relación activa con
/// ella (entonces se rompe); las demás relaciones activas del origen se rompen.
pub fn planificar(p: &Politica, origen: FaccionId, destino: FaccionId) -> Plan {
    let mut cambios = vec![];
    for r in p.relaciones.activas().filter(|r| r.incluye(origen)) {
        let hereda = r.tipo == TipoRelacion::Vasallaje && r.a == origen && r.b != destino && p.relaciones.activa_entre(destino, r.b).is_none();
        cambios.push(if hereda { Cambio::Heredado { relacion: r.id, vasalla: r.b } } else { Cambio::Rota { relacion: r.id } });
    }
    cambios.extend(p.propuestas.iter().filter(|x| x.clase.nombra(origen)).map(|x| Cambio::Descartada { propuesta: x.id }));
    if let (Some(o), Some(d)) = (p.facciones.get(origen), p.facciones.get(destino)) {
        let nuevos: Vec<_> = o.ciudadanos.iter().copied().filter(|h| !d.ciudadanos.contains(h)).collect();
        if !nuevos.is_empty() {
            cambios.push(Cambio::Ciudadanos { nuevos });
        }
    }
    cambios.extend(p.facciones.iter().filter(|f| f.derrota == Some(Derrota::Por(origen))).map(|f| Cambio::Derrota { faccion: f.id }));
    cambios.push(Cambio::Desaparece);
    Plan { origen, destino, cambios }
}

/// Aplica un plan sobre el estado político, en el orden del plan (primero las relaciones que se rompen y luego las que se heredan, para no chocar en el índice).
pub fn aplicar(p: &mut Politica, plan: &Plan) {
    for c in &plan.cambios {
        if let Cambio::Rota { relacion } = c {
            p.relaciones.terminar(*relacion);
        }
    }
    for c in &plan.cambios {
        match c {
            Cambio::Heredado { relacion, .. } => p.relaciones.cambiar_senora(*relacion, plan.destino),
            Cambio::Descartada { propuesta } => {
                p.propuestas.quitar(*propuesta);
            }
            Cambio::Derrota { faccion } => {
                if let Some(f) = p.facciones.get_mut(*faccion) {
                    f.derrota = Some(Derrota::Por(plan.destino));
                }
            }
            Cambio::Desaparece => {
                p.facciones.eliminar(plan.origen);
            }
            Cambio::Rota { .. } | Cambio::Ciudadanos { .. } => {}
        }
    }
    // Los ciudadanos, ya sin el origen en el índice.
    if let Some(Cambio::Ciudadanos { nuevos }) = plan.cambios.iter().find(|c| matches!(c, Cambio::Ciudadanos { .. })) {
        p.facciones.unir_ciudadanos(plan.destino, nuevos);
    }
}

/// `anexionar` (§6.1): A absorbe a B. A conserva nombre, Rey, Embajador, sigilo, experiencia (y reputación y tecnología, en el núcleo).
pub fn anexionar(p: &mut Politica, absorbente: FaccionId, absorbida: FaccionId) -> Plan {
    let plan = planificar(p, absorbida, absorbente);
    aplicar(p, &plan);
    plan
}

/// `fusionar` (§6.2): nace C (`id`) con el nombre y el Rey de la propuesta, el sigilo de A, la experiencia mayor, sin Embajador ni `capital_designada_en`; luego
/// `A → C` y `B → C`. El Rey que no lo es de C queda de ciudadano. La reputación mayor, la unión de tecnologías y la marca de capital (solo la que había designado
/// el Rey electo pasa a C: [`marca_tras_fusion`]) son del núcleo. Devuelve los dos planes.
pub fn fusionar(p: &mut Politica, r: &Reglas, id: FaccionId, union: &Union) -> Option<[Plan; 2]> {
    let Union::Fusion { a, b, nombre, rey } = union else { return None };
    let (fa, fb) = (p.facciones.get(*a)?.clone(), p.facciones.get(*b)?.clone());
    let c = Faccion {
        id,
        nombre: nombre.clone(),
        sigilo: fa.sigilo.clone(),
        rey: Some(*rey),
        embajador: None,
        capital_designada_en: None,
        experiencia: fa.experiencia.max(fb.experiencia),
        ciudadanos: vec![],
        solicitudes: vec![],
        derrota: None,
        admite_otras_en_ataques: false,
    };
    debug_assert!(c.nivel(r) >= fa.nivel(r).max(fb.nivel(r)));
    p.facciones.insertar(c);
    let pa = planificar(p, *a, id);
    aplicar(p, &pa);
    let pb = planificar(p, *b, id);
    aplicar(p, &pb);
    // El Rey electo es ciudadano de C (los dos lo son tras los traslados): el trono es suyo.
    p.facciones.asignar_rey(id, *rey).ok();
    Some([pa, pb])
}

/// La marca de capital de una plaza tras la fusión de `a` y `b` en `c`, siendo `de_rey_electo` la Facción cuyo Rey reina en C: solo la designada por el Rey
/// electo pasa a C; la de la otra se borra; las demás no cambian.
// DUDA: el TypeScript no comprueba aquí que la marca siga siendo vigente (Palacio activo, plaza de esa Facción); se copia tal cual y `capital::designada`
// ya la ignora si no vale.
pub fn marca_tras_fusion(marca: Option<FaccionId>, a: FaccionId, b: FaccionId, de_rey_electo: FaccionId, c: FaccionId) -> Option<FaccionId> {
    match marca {
        Some(m) if m == a || m == b => (m == de_rey_electo).then_some(c),
        otra => otra,
    }
}

#[cfg(test)]
mod pruebas {
    //! Casos de `typescript-final:src/engine/__tests__/diplomaciaDecisiones.test.ts` (la marca de capital en la anexión, en `marca_tras_fusion`) y de las
    //! reglas de `trasladoDeFaccion.ts` que prueban `comandosAnexion.test.ts` y `comandosFusion.test.ts`.
    use super::*;
    use crate::propuestas::{Pacto, Union};
    use crate::relaciones::pruebas::facciones;
    use crate::relaciones::Tributo;
    use asentamiento::Recurso;

    fn mundo(n: FaccionId) -> Politica {
        Politica { facciones: facciones(n), ..Default::default() }
    }

    #[test]
    fn la_anexion_hereda_vasallas_rompe_lo_demas_y_une_ciudadanos() {
        // 2 (absorbida) señora de 3 y 4, aliada de 5, en guerra con 6; 1 (absorbente) ya aliada de 4. 7 derrotada por 2.
        let mut m = mundo(7);
        m.facciones.otorgar(2, 21).unwrap();
        let t = Tributo { recurso: Recurso::Madera, cantidad_por_minuto: 3_000 };
        let fs = m.facciones.clone();
        m.relaciones.crear(&fs, 1, TipoRelacion::Vasallaje, 2, 3, Some(t), 0).unwrap();
        m.relaciones.crear(&fs, 2, TipoRelacion::Vasallaje, 2, 4, None, 0).unwrap();
        m.relaciones.crear(&fs, 3, TipoRelacion::Alianza, 2, 5, None, 0).unwrap();
        m.relaciones.crear(&fs, 4, TipoRelacion::Guerra, 6, 2, None, 0).unwrap();
        m.relaciones.crear(&fs, 5, TipoRelacion::Alianza, 1, 4, None, 0).unwrap();
        m.facciones.registrar_derrota(7, Some(2));
        let r = Reglas::default();
        let rr = comercio::datos::Reglas::cargar().reputacion;
        m.proponer_anexion(&r, 30, 2, 7, 20, 0).unwrap();
        m.proponer_relacion(&r, &rr, 31, Pacto::Alianza, 6, 1, None, 60, 0, 0).unwrap();

        let plan = anexionar(&mut m, 1, 2);
        assert_eq!(
            plan.cambios,
            vec![
                Cambio::Heredado { relacion: 1, vasalla: 3 },
                Cambio::Rota { relacion: 2 },
                Cambio::Rota { relacion: 3 },
                Cambio::Rota { relacion: 4 },
                Cambio::Descartada { propuesta: 30 },
                Cambio::Ciudadanos { nuevos: vec![20, 21] },
                Cambio::Derrota { faccion: 7 },
                Cambio::Desaparece,
            ]
        );
        assert_eq!(m.relaciones.senora_de(3), Some(1));
        assert_eq!(m.relaciones.get(1).unwrap().tributo, Some(t), "mismo tributo");
        assert_eq!(m.relaciones.senora_de(4), None, "1 ya era aliada de 4: queda libre");
        assert!(m.relaciones.estan_aliadas(1, 4) && !m.relaciones.en_guerra(6, 2));
        assert!(m.facciones.get(2).is_none());
        assert_eq!(m.facciones.get(1).unwrap().ciudadanos, vec![10, 20, 21]);
        assert_eq!((m.facciones.faccion_de(20), m.facciones.get(1).unwrap().rey), (Some(1), Some(10)), "A conserva su Rey");
        assert_eq!(m.facciones.get(7).unwrap().derrota, Some(Derrota::Por(1)));
        assert!(m.propuestas.get(30).is_none() && m.propuestas.get(31).is_some(), "lo que otros ofrecen al destino sigue");
    }

    #[test]
    fn la_vasalla_absorbida_por_su_senora_termina_el_vasallaje() {
        let mut m = mundo(2);
        let fs = m.facciones.clone();
        m.relaciones.crear(&fs, 1, TipoRelacion::Vasallaje, 1, 2, None, 0).unwrap();
        let plan = anexionar(&mut m, 1, 2);
        assert_eq!(plan.cambios[0], Cambio::Rota { relacion: 1 });
        assert!(m.relaciones.activas().next().is_none());
    }

    #[test]
    fn la_fusion_hace_nacer_c_con_lo_de_a_y_la_experiencia_mayor() {
        let r = Reglas::default();
        let mut m = mundo(3);
        m.facciones.get_mut(2).unwrap().experiencia = 400_000;
        m.facciones.asignar_embajador(1, 10).unwrap();
        let fs = m.facciones.clone();
        m.relaciones.crear(&fs, 1, TipoRelacion::Alianza, 1, 2, None, 0).unwrap();
        m.relaciones.crear(&fs, 2, TipoRelacion::Vasallaje, 2, 3, None, 0).unwrap();
        let sigilo_a = m.facciones.get(1).unwrap().sigilo.clone();
        let planes = fusionar(&mut m, &r, 9, &Union::Fusion { a: 1, b: 2, nombre: "Delta".into(), rey: 20 }).unwrap();
        assert_eq!(planes[0].cambios.last(), Some(&Cambio::Desaparece));
        let c = m.facciones.get(9).unwrap();
        assert_eq!((c.nombre.as_str(), c.rey, c.embajador, c.experiencia, c.nivel(&r)), ("Delta", Some(20), None, 400_000, 4));
        assert_eq!(c.sigilo, sigilo_a);
        assert_eq!(c.ciudadanos, vec![10, 20], "primero los de A, luego los de B; el otro Rey queda de ciudadano");
        assert!(m.facciones.get(1).is_none() && m.facciones.get(2).is_none());
        assert_eq!(m.relaciones.senora_de(3), Some(9), "las vasallas pasan a C");
        assert!(!m.relaciones.estan_aliadas(1, 2));
    }

    #[test]
    fn solo_la_marca_del_rey_electo_pasa_a_c() {
        // Anexión de `diplomaciaDecisiones.test.ts`: la marca de B no sobrevive (el núcleo la borra: no es del destino).
        assert_eq!(marca_tras_fusion(Some(1), 1, 2, 1, 9), Some(9));
        assert_eq!(marca_tras_fusion(Some(2), 1, 2, 1, 9), None);
        assert_eq!(marca_tras_fusion(Some(5), 1, 2, 1, 9), Some(5));
        assert_eq!(marca_tras_fusion(None, 1, 2, 1, 9), None);
    }
}
