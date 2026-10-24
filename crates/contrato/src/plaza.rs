//! El cambio parcial de una `Asentamiento` (contrato 05 §5, decisión 2 de 4H): la plaza vista desde dentro viaja con **máscara de campos**, solo lo que cambió. [`mascara`] es lo que calcula
//! el servidor; [`aplicar`], lo que hace el cliente con el `Cambio` (`campos` y `datos`); [`todos_los_campos`], la máscara de quien reanuda sin saber qué tiene el cliente.

use crate::v1 as pb;

/// Define a la vez [`mascara`], [`aplicar`] y [`todos_los_campos`] sobre la lista de campos de `Asentamiento` (sin `id`, `version` ni `version_interior`, que siempre viajan). El literal de `mascara` nombra todos los
/// campos sin `..`: un campo nuevo del contrato no compila hasta entrar en la lista.
macro_rules! campos_de_la_plaza {
    ($($c:ident),* $(,)?) => {
        /// Del cambio de la plaza de `antes` a `ahora`, los campos que cambiaron (con `version` y `version_interior` delante) y la plaza con solo esos campos; `None` si nada cambió (las dos versiones solas no son un cambio).
        pub fn mascara(antes: &pb::Asentamiento, ahora: &pb::Asentamiento) -> Option<(Vec<String>, pb::Asentamiento)> {
            let mut campos = Vec::new();
            let datos = pb::Asentamiento {
                id: ahora.id.clone(),
                version: ahora.version,
                version_interior: ahora.version_interior,
                $($c: if antes.$c != ahora.$c { campos.push(stringify!($c).to_owned()); ahora.$c.clone() } else { Default::default() },)*
            };
            (!campos.is_empty()).then(|| {
                campos.splice(0..0, ["version".to_owned(), "version_interior".to_owned()]);
                (campos, datos)
            })
        }

        /// Lo que hace el cliente con un cambio parcial: copia de `datos` los campos de la máscara (un campo con su valor por defecto se pone a él, un `optional` ausente se borra).
        pub fn aplicar(destino: &mut pb::Asentamiento, datos: &pb::Asentamiento, campos: &[String]) {
            for c in campos {
                match c.as_str() {
                    "version" => destino.version = datos.version,
                    "version_interior" => destino.version_interior = datos.version_interior,
                    $(stringify!($c) => destino.$c = datos.$c.clone(),)*
                    _ => {}
                }
            }
        }

        /// La máscara que lo cambia todo: la de quien no sabe qué tiene el cliente (la reanudación por deltas).
        pub fn todos_los_campos() -> Vec<String> {
            vec!["version".to_owned(), "version_interior".to_owned(), $(stringify!($c).to_owned()),*]
        }
    };
}

campos_de_la_plaza!(
    nombre,
    faccion_id,
    fundadores_ids,
    posicion,
    nivel,
    nivel_actual,
    racha_mantenimiento_sano,
    fundado_en_ms,
    radio_potencial,
    zona,
    poblacion,
    almacen,
    edificios,
    recintos,
    cargos,
    casas_compradas,
    puerta_cerrada_a,
    capital_de_faccion_id,
    vetados_ids,
    politicas_activas,
    ocupacion_hasta_ms,
    bandidos_reaparecen_en_ms,
    protegida_hasta_ms,
    ascenso,
    medidor_mantenimiento,
    nutricion,
    auto_construccion_pausada,
    recetas_pausadas,
    permite_reabastecer_aliados,
    reserva_manual,
    ultima_caravana_creada_en_ms,
    pueblos_ids,
    avanzadas_ids,
    trazado,
    batalla_id,
    guarnicion_ids,
    cupo_guarnicion,
    presentes_ids,
    caravanas_aparcadas_ids,
    flota_ids,
);
