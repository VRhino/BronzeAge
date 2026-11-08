//! El prestigio de la Facción (tanda 69, Diplomacia_Facciones §3.4 `PRESTIGIO`): se acumula y no se gasta ni se pierde. Cada hecho suma su cuantía en el
//! instante en que ocurre ([`Nucleo::dar_prestigio`]); lo que lo usa son umbrales: las alianzas activas a la vez (`politica::propuestas`), la Maravilla y las estatuas de la capital (`asentamiento::obras`,
//! que reciben del núcleo lo que el prestigio deja hacer, [`Nucleo::prestigio_para_obras`]).
//!
//! Se gana por: victoria contra héroes de otra Facción (los mismos combates que las monedas de combate, una vez por Facción ganadora y combate), conquistar una plaza, fundar, subir una plaza de nivel
//! (× el nivel nuevo) y comerciar con otra Facción (un trueque cumplido, o una orden de mercado tomada).

use super::*;
use std::collections::BTreeSet;
use combate::resolucion::Informe;

/// Por qué gana la Facción prestigio: el `motivo` del hecho `faccion.prestigio`.
pub(super) mod motivo {
    pub const VICTORIA: i64 = 1;
    pub const CONQUISTA: i64 = 2;
    pub const FUNDACION: i64 = 3;
    pub const ASCENSO: i64 = 4;
    pub const COMERCIO: i64 = 5;
}

impl Nucleo {
    /// La Facción gana `ganado` de prestigio por `motivo` (lo ≤ 0, o una Facción que no existe como entidad, no da nada): `faccion.prestigio` con lo ganado y el total.
    pub(super) fn dar_prestigio(&mut self, t: Instante, faccion: u32, ganado: i64, motivo: i64) {
        let Some(total) = self.estado.politica.facciones.sumar_prestigio(faccion, ganado) else { return };
        self.tr.tocar(Clase::Faccion, u64::from(faccion));
        self.emitir_propio(t, "faccion.prestigio", vec![], vec![("faccion", i64::from(faccion)), ("ganado", ganado), ("total", total), ("motivo", motivo)]);
    }

    /// Lo que el prestigio de la Facción deja construir: si alcanza el umbral de la Maravilla y cuántas estatuas (tantas como umbrales de `PRESTIGIO.estatuas` alcanzados).
    pub(super) fn prestigio_para_obras(&self, faccion: u32) -> (bool, u32) {
        let prestigio = self.estado.politica.facciones.get(faccion).map_or(0, |f| f.prestigio);
        let r = &self.estado.reglas.economia.politica.prestigio;
        (prestigio >= r.maravilla, r.estatuas.iter().filter(|&&umbral| prestigio >= umbral).count() as u32)
    }

    /// El prestigio de la victoria de un combate con números: los héroes de cada bando con su Facción, como en las monedas de combate.
    pub(super) fn prestigio_de_informe(&mut self, t: Instante, informe: &Informe) {
        let (ganadores, perdedores) = self.bandos_con_faccion(informe);
        self.prestigio_por_victoria(t, &ganadores, &perdedores);
    }

    /// El prestigio de la victoria: `victoria` a cada Facción ganadora que venció a héroes de otra Facción, **una vez por combate** (no por héroe). `ganadores` y `perdedores` son (héroe, Facción con la que
    /// combatió); contra bandidos (sin héroes con Facción) o entre héroes de la misma Facción no se da nada.
    pub(super) fn prestigio_por_victoria(&mut self, t: Instante, ganadores: &[(u64, u32)], perdedores: &[(u64, u32)]) {
        let victoria = self.estado.reglas.economia.politica.prestigio.victoria;
        for faccion in facciones_vencedoras(ganadores, perdedores) {
            self.dar_prestigio(t, faccion, victoria, motivo::VICTORIA);
        }
    }
}

/// Las Facciones ganadoras que tuvieron enfrente a algún héroe de otra Facción, cada una una vez.
fn facciones_vencedoras(ganadores: &[(u64, u32)], perdedores: &[(u64, u32)]) -> BTreeSet<u32> {
    ganadores.iter().map(|&(_, f)| f).filter(|&f| perdedores.iter().any(|&(_, p)| p != f)).collect()
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn solo_gana_prestigio_la_faccion_que_vence_a_otra_y_una_sola_vez_aunque_ganen_varios_heroes() {
        // Dos héroes de la Facción 3 contra uno de la 1 y otro de la 3: la 3 gana una vez.
        assert_eq!(facciones_vencedoras(&[(1, 3), (2, 3)], &[(3, 1), (4, 3)]), BTreeSet::from([3]));
        // Entre la misma Facción, nada; sin perdedores con Facción (bandidos), nada.
        assert!(facciones_vencedoras(&[(1, 2), (3, 2)], &[(4, 2)]).is_empty());
        assert!(facciones_vencedoras(&[(1, 2)], &[]).is_empty());
        // Ganadores de dos Facciones contra una tercera: cada una suma.
        assert_eq!(facciones_vencedoras(&[(1, 5), (2, 7)], &[(3, 2)]), BTreeSet::from([5, 7]));
    }
}
