//! Cuándo llegan los bots y cuándo juegan (12_NPC_Fuera_Del_Motor.md del motor, §3, §11 y §12; D55, D56 y D57), como en el runner TypeScript (`typescript-final`,
//! `src/bots/llegadas.ts` y `runner.ts`), con el tiempo en ms de mundo en vez de ticks.

use serde::{Deserialize, Serialize};

use crate::azar::Azar;

/// Cómo llega y con quién juega (D57). Cambia las prioridades del cerebro, no las reglas.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "tipo", rename_all = "lowercase")]
pub enum Perfil {
    /// Llegan cinco juntos; el `lider` (su héroe) crea la Facción y los demás le piden entrar.
    Amigos { lider: String },
    /// Crea la suya.
    Solitario,
    /// Llega en la segunda mitad y pide entrar en una Facción que ya exista.
    Tardio,
    /// Llega porque un bot de `faccion` fundó una plaza (§16: cada fundación de un bot trae 5 héroes nuevos) y pide entrar en ella.
    Refuerzo { faccion: String },
    /// De la población de mercenarios (`mercenario.rs`), que tiene su propio plan de llegadas.
    Mercenario,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TipoPerfil {
    Amigos,
    Solitario,
    Tardio,
    Refuerzo(String),
    Mercenario,
}

/// Una llegada: a los `en_ms` de mundo desde que el proceso empezó en la partida, los `cuantos` del grupo `grupo`.
#[derive(Clone, Debug, PartialEq)]
pub struct Llegada {
    pub en_ms: i64,
    pub grupo: u32,
    pub perfil: TipoPerfil,
    pub cuantos: u32,
}

pub const AMIGOS_POR_GRUPO: u32 = 5;
/// Reparto por GRUPO (por bots: 56 % amigos, 6 % solitarios, 39 % tardíos). ponytail: placeholder hasta medir con batch, como en TypeScript.
const REPARTO_AMIGOS: f64 = 0.2;
const REPARTO_SOLITARIO: f64 = 0.1;

/// `total` bots repartidos en `dias` días de mundo, por orden de llegada. Misma semilla, mismas llegadas.
pub fn plan_de_llegadas(semilla: u64, total: u32, dias: f64) -> Vec<Llegada> {
    let mut azar = Azar::new(semilla ^ 0x11e9);
    let ms = ((dias * 86_400_000.0).round() as i64).max(1);
    let mut llegadas = Vec::new();
    let mut quedan = total;
    let mut grupo = 0;
    while quedan > 0 {
        let r = azar.real();
        let sorteo = if r < REPARTO_AMIGOS { TipoPerfil::Amigos } else if r < REPARTO_AMIGOS + REPARTO_SOLITARIO { TipoPerfil::Solitario } else { TipoPerfil::Tardio };
        // El primer grupo funda siempre, para que los tardíos tengan a quién pedir entrar; unos amigos que ya no caben, a tardíos.
        let perfil = match (grupo, sorteo.clone()) {
            (0, _) if quedan >= AMIGOS_POR_GRUPO => TipoPerfil::Amigos,
            (0, _) => TipoPerfil::Solitario,
            (_, TipoPerfil::Amigos) if quedan < AMIGOS_POR_GRUPO => TipoPerfil::Tardio,
            (_, p) => p,
        };
        let desde = if perfil == TipoPerfil::Tardio { ms / 2 } else { 0 };
        let en_ms = desde + (azar.real() * (ms - desde) as f64) as i64;
        let cuantos = if perfil == TipoPerfil::Amigos { AMIGOS_POR_GRUPO } else { 1 };
        llegadas.push(Llegada { en_ms, grupo, perfil, cuantos });
        quedan -= cuantos;
        grupo += 1;
    }
    llegadas.sort_by_key(|l| (l.en_ms, l.grupo));
    llegadas
}

const MINUTOS_DIA: i64 = 24 * 60;

/// Los bloques del día en que juega, en minutos `[inicio, fin)` (`fin` puede pasar de 1440): uno o dos que suman de 2 a 6 horas (D55). ponytail: placeholder a calibrar con batch.
pub fn horario(azar: &mut Azar) -> Vec<(i64, i64)> {
    let total = ((2.0 + azar.real() * 4.0) * 60.0).round() as i64;
    let bloques = if azar.real() < 0.5 { 1 } else { 2 };
    let duracion = total / bloques;
    let primero = (azar.real() * MINUTOS_DIA as f64) as i64;
    // El segundo, al menos 4 horas después de que acabe el primero.
    let segundo = (primero + duracion + 240 + (azar.real() * (MINUTOS_DIA - 2 * duracion - 240) as f64) as i64) % MINUTOS_DIA;
    let inicios = if bloques == 1 { vec![primero] } else { vec![primero, segundo] };
    inicios.into_iter().map(|i| (i, i + duracion)).collect()
}

pub fn en_sesion(sesiones: &[(i64, i64)], mundo_ms: i64) -> bool {
    let minuto = (mundo_ms / 60_000).rem_euclid(MINUTOS_DIA);
    sesiones.iter().any(|&(i, f)| (minuto >= i && minuto < f) || minuto + MINUTOS_DIA < f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llegadas_deterministas_y_completas() {
        let plan = plan_de_llegadas(7, 30, 3.0);
        assert_eq!(plan, plan_de_llegadas(7, 30, 3.0));
        assert_eq!(plan.iter().map(|l| l.cuantos).sum::<u32>(), 30);
        assert!(plan.iter().any(|l| l.grupo == 0 && l.perfil == TipoPerfil::Amigos));
        assert!(plan.windows(2).all(|w| w[0].en_ms <= w[1].en_ms));
        assert!(plan.iter().all(|l| l.en_ms < 3 * 86_400_000 && (l.perfil != TipoPerfil::Tardio || l.en_ms >= 3 * 43_200_000)));
    }

    #[test]
    fn horario_de_2_a_6_horas() {
        for s in 0..200 {
            let h = horario(&mut Azar::new(s));
            let total: i64 = h.iter().map(|(i, f)| f - i).sum();
            assert!((119..=360).contains(&total), "{h:?}");
            assert!(en_sesion(&h, h[0].0 * 60_000), "{h:?}");
        }
        // Un bloque que cruza la medianoche.
        assert!(en_sesion(&[(1400, 1500)], 30 * 60_000) && !en_sesion(&[(1400, 1500)], 61 * 60_000));
    }
}
