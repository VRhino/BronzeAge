//! La comida que el jugador de los batch de población, comercio y militar encarga a mano (tanda 69): la auto-construcción no propone los talleres de la
//! comida (Economia_Construccion §5.3) ni sube las aldeas (§14.2). Aquí se decide qué toca; cada batch da la orden a su manera (al instante en el de población,
//! con la orden del contrato en los del núcleo). Y se mide lo que quedó al final.

use crate::armar::{metrica, r3, INFO};
use crate::medidas::pct;
use asentamiento::asentamiento::Asentamiento;
use asentamiento::catalogo::Tipo;
use asentamiento::edificio::Edificios;
use asentamiento::pueblos::Pueblo;
use asentamiento::Reglas;
use informe::Metrica;

/// Los talleres de la comida que se encargan, por orden: harina y pan del trigo, y queso de la leche. La bodega y la prensa piden los niveles 3 y 4 de la plaza,
/// a los que estos batch no llegan, y aldeas de vid y de olivo.
const TALLERES: [Tipo; 3] = [Tipo::Molino, Tipo::Horno, Tipo::Quesera];

/// El taller de la comida que toca encargar: con la aldea de trigo activa (de ella salen el trigo de más y la leche), el primero de [`TALLERES`] que la plaza
/// no tiene y que su nivel ya permite.
pub fn taller_que_falta(a: &Asentamiento, r: &Reglas) -> Option<Tipo> {
    if !a.pueblos.iter().any(|p| p.es_aldea() && p.activo()) {
        return None;
    }
    TALLERES.into_iter().find(|&t| a.edificios.en_cualquier_estado(t) == 0 && r.catalogo.tipo(t).nivel_construccion <= a.nivel_actual())
}

/// El molino al que aún no se le ha comprado su buey (`AsignarBuey`).
pub fn molino_sin_buey(a: &Asentamiento) -> Option<u32> {
    a.edificios.iter().find(|e| e.tipo == Tipo::Molino && !e.buey).map(|e| e.id)
}

/// La aldea que toca subir de nivel: la activa, sin otra subida en curso y por debajo del nivel máximo.
pub fn aldea_que_sube<'a>(a: &'a Asentamiento, r: &Reglas) -> Option<&'a Pueblo> {
    a.pueblos.iter().find(|p| p.es_aldea() && p.activo() && p.mejora.is_none() && p.nivel < r.pueblos.aldea.nivel_maximo())
}

/// Las aldeas y los talleres de la comida de las plazas al final de la corrida.
pub fn metricas<'a>(plazas: impl Iterator<Item = &'a Asentamiento>) -> Vec<Metrica> {
    let (mut total, mut por_nivel, mut con_pan, mut queseras, mut bueyes) = (0, [0usize; 3], 0, 0, 0);
    for a in plazas {
        total += 1;
        if let Some(n) = a.pueblos.iter().filter(|p| p.es_aldea() && p.activo()).map(|p| p.nivel).max() {
            por_nivel[(n as usize).clamp(1, 3) - 1] += 1;
        }
        con_pan += usize::from(a.edificios.activos(Tipo::Molino) > 0 && a.edificios.activos(Tipo::Horno) > 0);
        queseras += usize::from(a.edificios.activos(Tipo::Quesera) > 0);
        bueyes += usize::from(a.edificios.iter().any(|e| e.tipo == Tipo::Molino && e.buey));
    }
    let con_aldea: usize = por_nivel.iter().sum();
    let nivel_medio = (con_aldea > 0).then(|| r3((por_nivel[0] + 2 * por_nivel[1] + 3 * por_nivel[2]) as f64 / con_aldea as f64));
    vec![
        metrica(
            "aldea_activa_pct",
            "Plazas con su aldea de trigo activa al final",
            "%",
            INFO,
            pct(con_aldea, total),
            "Economia_Construccion §14.2",
            &format!("De {total} plazas. Por el nivel de su aldea: {} en el 1, {} en el 2 y {} en el 3. El jugador la sube en cuanto puede.", por_nivel[0], por_nivel[1], por_nivel[2]),
        ),
        metrica("aldea_nivel_medio", "Nivel medio de la aldea de trigo al final", "nivel", INFO, nivel_medio, "Economia_Construccion §14.2", "Solo las plazas que tienen aldea activa."),
        metrica(
            "pan_pct",
            "Plazas con molino y horno activos al final",
            "%",
            INFO,
            pct(con_pan, total),
            "Economia_Construccion §5.3 y §9.2 (el pan da ×2 al crecimiento de los pesants)",
            &format!("Se encargan a mano en cuanto la aldea está activa. Con quesera activa: {queseras}; molinos con buey: {bueyes}."),
        ),
    ]
}
