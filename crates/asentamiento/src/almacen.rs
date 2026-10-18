//! Almacén de un asentamiento (Economia_Construccion §4): cantidad y capacidad por recurso, en milésimas. Sin reloj: el que lo
//! usa decide cuándo se agrega o se quita; aquí solo están las reglas de cabida.

use crate::recurso::{Recurso, TODOS};
use base::Mil;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Almacen {
    cantidad: [Mil; 29],
    capacidad: [Mil; 29],
}

impl Almacen {
    /// Un almacén vacío con `capacidad` para cada uno de los 29 recursos (`ALMACEN.capacidadInicialPorRecurso`).
    pub fn nuevo(capacidad: Mil) -> Self {
        Self { cantidad: [0; 29], capacidad: [capacidad; 29] }
    }

    pub fn cantidad(&self, r: Recurso) -> Mil {
        self.cantidad[r.indice()]
    }

    pub fn capacidad(&self, r: Recurso) -> Mil {
        self.capacidad[r.indice()]
    }

    /// Lo que todavía cabe.
    pub fn espacio(&self, r: Recurso) -> Mil {
        (self.capacidad(r) - self.cantidad(r)).max(0)
    }

    /// Fija la cantidad (entregas iniciales, restaurar un estado); nunca por encima de la capacidad.
    pub fn poner(&mut self, r: Recurso, n: Mil) {
        self.cantidad[r.indice()] = n.clamp(0, self.capacidad(r));
    }

    /// Agrega `n` (≥ 0) hasta donde quepa y devuelve lo que entró; el sobrante se pierde.
    pub fn agregar(&mut self, r: Recurso, n: Mil) -> Mil {
        let entra = n.max(0).min(self.espacio(r));
        self.cantidad[r.indice()] += entra;
        entra
    }

    /// Quita hasta `n` y devuelve lo quitado (nunca deja la cantidad negativa).
    pub fn quitar(&mut self, r: Recurso, n: Mil) -> Mil {
        let sale = n.max(0).min(self.cantidad(r));
        self.cantidad[r.indice()] -= sale;
        sale
    }

    pub fn tiene(&self, costo: &[(Recurso, Mil)]) -> bool {
        costo.iter().all(|&(r, n)| self.cantidad(r) >= n)
    }

    /// Paga el costo entero o no toca nada.
    pub fn pagar(&mut self, costo: &[(Recurso, Mil)]) -> bool {
        if !self.tiene(costo) {
            return false;
        }
        costo.iter().for_each(|&(r, n)| self.cantidad[r.indice()] -= n);
        true
    }

    /// Devuelve un costo ya pagado (reembolso); lo que no quepa se pierde.
    pub fn devolver(&mut self, costo: &[(Recurso, Mil)]) {
        costo.iter().for_each(|&(r, n)| {
            self.agregar(r, n);
        });
    }

    /// `capacidad = max(0, capacidad + extra)`; lo que ya no cabe se pierde (`ampliarCapacidad`).
    pub fn ampliar(&mut self, r: Recurso, extra: Mil) {
        let i = r.indice();
        self.capacidad[i] = (self.capacidad[i] + extra).max(0);
        self.cantidad[i] = self.cantidad[i].min(self.capacidad[i]);
    }

    /// Lo mismo para todos los recursos (el Almacén suma 300 a cada uno).
    pub fn ampliar_todos(&mut self, extra: Mil) {
        TODOS.iter().for_each(|&r| self.ampliar(r, extra));
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use Recurso::*;

    #[test]
    fn agregar_respeta_la_capacidad_y_devuelve_lo_que_cupo() {
        let mut a = Almacen::nuevo(400_000);
        assert_eq!(a.agregar(Madera, 300_000), 300_000);
        assert_eq!(a.agregar(Madera, 300_000), 100_000, "solo cabían 100");
        assert_eq!(a.cantidad(Madera), 400_000);
        assert_eq!(a.espacio(Madera), 0);
        assert_eq!(a.agregar(Madera, -5), 0);
    }

    #[test]
    fn pagar_es_todo_o_nada() {
        let mut a = Almacen::nuevo(400_000);
        a.poner(Madera, 50_000);
        a.poner(Piedra, 10_000);
        let costo = [(Madera, 40_000), (Piedra, 20_000)];
        assert!(!a.pagar(&costo));
        assert_eq!((a.cantidad(Madera), a.cantidad(Piedra)), (50_000, 10_000), "no se toca nada si falta algo");
        a.poner(Piedra, 20_000);
        assert!(a.pagar(&costo));
        assert_eq!((a.cantidad(Madera), a.cantidad(Piedra)), (10_000, 0));
        a.devolver(&costo);
        assert_eq!((a.cantidad(Madera), a.cantidad(Piedra)), (50_000, 20_000));
    }

    #[test]
    fn quitar_nunca_deja_negativo() {
        let mut a = Almacen::nuevo(400_000);
        a.poner(Trigo, 5_000);
        assert_eq!(a.quitar(Trigo, 9_000), 5_000);
        assert_eq!(a.cantidad(Trigo), 0);
    }

    #[test]
    fn ampliar_y_reducir_capacidad() {
        let mut a = Almacen::nuevo(400_000);
        a.poner(Trigo, 400_000);
        a.ampliar(Trigo, 2_000_000);
        assert_eq!(a.capacidad(Trigo), 2_400_000);
        a.ampliar_todos(300_000);
        assert_eq!((a.capacidad(Madera), a.capacidad(Trigo)), (700_000, 2_700_000));
        a.ampliar(Trigo, -9_000_000);
        assert_eq!((a.capacidad(Trigo), a.cantidad(Trigo)), (0, 0), "la capacidad no baja de 0 y lo que no cabe se pierde");
    }
}
