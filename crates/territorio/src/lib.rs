//! Reglas puras de mundo y territorio (fase 6, Doc 1): zonas de influencia y fronteras, dónde se puede fundar y a qué
//! parcela de bosque tiene derecho una Leñera. Sin estado ni reloj: el núcleo guarda el estado y llama a estas funciones.

mod contorno;
mod fundacion;
mod parcelas;
mod zona;

pub use contorno::contornos;
pub use fundacion::{validar, viabilidad, DistanciasDeFundacion, Motivo, Terreno, Viabilidad};
pub use parcelas::{cupo_por_masa, elegir as elegir_parcela, Parcela, SinParcela};
pub use zona::{en_zona, poligono, se_tocan, Disco};
