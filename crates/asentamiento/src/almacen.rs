//! Almacén de un asentamiento, de un pueblo o de una avanzada (Economia_Construccion §4): cantidad y capacidad por recurso. Sin reloj: el que
//! lo usa decide cuándo se agrega o se quita; aquí solo están las reglas de cabida.
//!
//! **Solo unidades enteras** (decisión C3b del autor, 2026-10-24): `cantidad` es siempre un múltiplo de [`base::UNIDAD`]. Hay dos formas de mover:
//!
//! - **Discreta** ([`Almacen::agregar`], [`Almacen::quitar`], [`Almacen::pagar`], [`Almacen::devolver`], [`Almacen::poner`]): entra o sale un
//!   número entero de unidades. Lo que entra o sale se **redondea hacia abajo**; un coste se cobra **redondeado hacia arriba**. Nunca se crea de más.
//! - **Flujo por minuto** ([`Almacen::producir`] y [`Almacen::consumir`]: producción, extracción, recetas, comida, mantenimiento, recaudación):
//!   la fracción va a un **resto oculto** por recurso, siempre en `[0, 1000)`, como la población entera. Lo exacto es `cantidad + resto`; lo que
//!   se ve es la parte entera. Una unidad entra cuando el resto la completa y sale en cuanto el flujo empieza a comérsela, así que lo visible nunca
//!   pasa de lo exacto. En N minutos entra exactamente la suma entera de lo producido.

use crate::recurso::{Recurso, TODOS};
use base::{entero, entero_arriba, Mil};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Almacen {
    cantidad: [Mil; 29],
    capacidad: [Mil; 29],
    /// Resto oculto de los flujos, en `[0, 1000)` y nunca por encima del hueco que queda. Nadie fuera de aquí lo lee salvo la foto.
    resto: [Mil; 29],
}

impl Almacen {
    /// Un almacén vacío con `capacidad` para cada uno de los 29 recursos (`ALMACEN.capacidadInicialPorRecurso`).
    pub fn nuevo(capacidad: Mil) -> Self {
        Self { cantidad: [0; 29], capacidad: [capacidad; 29], resto: [0; 29] }
    }

    /// Las cantidades, capacidades y restos de los 29 recursos, en orden de [`TODOS`] (para serializar).
    pub fn partes(&self) -> (&[Mil; 29], &[Mil; 29], &[Mil; 29]) {
        (&self.cantidad, &self.capacidad, &self.resto)
    }

    /// Inversa de [`Almacen::partes`] (restaurar de una foto). No recorta: se guardó un almacén válido.
    pub fn desde_partes(cantidad: [Mil; 29], capacidad: [Mil; 29], resto: [Mil; 29]) -> Self {
        Self { cantidad, capacidad, resto }
    }

    /// Unidades enteras que hay (múltiplo de 1000): lo que se ve, se comercia, se carga y se paga.
    pub fn cantidad(&self, r: Recurso) -> Mil {
        self.cantidad[r.indice()]
    }

    /// Lo exacto, con el resto oculto: lo que un flujo puede consumir y la base de la tendencia «+N/min».
    pub fn disponible(&self, r: Recurso) -> Mil {
        self.cantidad[r.indice()] + self.resto[r.indice()]
    }

    pub fn capacidad(&self, r: Recurso) -> Mil {
        self.capacidad[r.indice()]
    }

    /// Unidades enteras que todavía caben.
    pub fn espacio(&self, r: Recurso) -> Mil {
        entero((self.capacidad(r) - self.cantidad(r)).max(0))
    }

    /// Lo que todavía puede entrar por un flujo (milésimas): el hueco hasta la capacidad sin el resto ya acumulado.
    pub fn hueco(&self, r: Recurso) -> Mil {
        (self.capacidad(r) - self.disponible(r)).max(0)
    }

    /// El resto nunca deja lo exacto por encima de la capacidad (lo que no cabe se pierde, como siempre).
    fn ajustar(&mut self, i: usize) {
        self.resto[i] = self.resto[i].min((self.capacidad[i] - self.cantidad[i]).max(0));
    }

    /// Fija la cantidad en unidades enteras (entregas iniciales, restaurar un estado); nunca por encima de la capacidad.
    pub fn poner(&mut self, r: Recurso, n: Mil) {
        let i = r.indice();
        self.cantidad[i] = entero(n).clamp(0, entero(self.capacidad[i].max(0)));
        self.ajustar(i);
    }

    /// Agrega las unidades enteras de `n` (≥ 0) hasta donde quepan y devuelve lo que entró; la fracción y el sobrante se pierden.
    pub fn agregar(&mut self, r: Recurso, n: Mil) -> Mil {
        let entra = entero(n.max(0)).min(self.espacio(r));
        let i = r.indice();
        self.cantidad[i] += entra;
        self.ajustar(i);
        entra
    }

    /// Quita las unidades enteras de `n`, hasta las que haya, y devuelve lo quitado (nunca deja la cantidad negativa).
    pub fn quitar(&mut self, r: Recurso, n: Mil) -> Mil {
        let sale = entero(n.max(0)).min(self.cantidad(r));
        self.cantidad[r.indice()] -= sale;
        sale
    }

    /// Flujo de entrada de un minuto (producción, extracción, receta, recaudación): `n` milésimas, hasta el hueco, van al resto y entran las
    /// unidades que se completan. Devuelve lo que entró del flujo (milésimas); si es menos que `n`, el almacén está lleno.
    pub fn producir(&mut self, r: Recurso, n: Mil) -> Mil {
        let entra = n.max(0).min(self.hueco(r));
        let i = r.indice();
        self.resto[i] += entra;
        if self.resto[i] >= base::UNIDAD {
            let unidades = entero(self.resto[i]);
            self.cantidad[i] += unidades;
            self.resto[i] -= unidades;
        }
        entra
    }

    /// Flujo de salida de un minuto (comida, mantenimiento, insumos de una receta): `n` milésimas, hasta lo exacto que haya. Sale primero el
    /// resto y, si no basta, las unidades enteras que hagan falta; lo que sobra de la última vuelve al resto. Devuelve lo consumido (milésimas).
    pub fn consumir(&mut self, r: Recurso, n: Mil) -> Mil {
        let sale = n.max(0).min(self.disponible(r));
        let i = r.indice();
        if sale <= self.resto[i] {
            self.resto[i] -= sale;
        } else {
            let falta = sale - self.resto[i];
            let unidades = entero_arriba(falta);
            self.cantidad[i] -= unidades;
            self.resto[i] = unidades - falta;
        }
        sale
    }

    /// ¿Hay para pagar el coste? Cada línea se cobra en unidades enteras, redondeada hacia arriba: como la cantidad ya es un múltiplo de 1000, tenerla
    /// redondeada hacia arriba es lo mismo que tener la línea tal cual (y así no se divide en cada pregunta de la auto-construcción).
    pub fn tiene(&self, costo: &[(Recurso, Mil)]) -> bool {
        costo.iter().all(|&(r, n)| self.cantidad(r) >= n)
    }

    /// Paga el costo entero (cada línea redondeada hacia arriba a unidades) o no toca nada.
    pub fn pagar(&mut self, costo: &[(Recurso, Mil)]) -> bool {
        if !self.tiene(costo) {
            return false;
        }
        costo.iter().for_each(|&(r, n)| self.cantidad[r.indice()] -= entero_arriba(n.max(0)));
        true
    }

    /// Devuelve un costo ya pagado (reembolso), en unidades enteras hacia abajo; lo que no quepa se pierde.
    pub fn devolver(&mut self, costo: &[(Recurso, Mil)]) {
        costo.iter().for_each(|&(r, n)| {
            self.agregar(r, n);
        });
    }

    /// `capacidad = max(0, capacidad + extra)`; lo que ya no cabe se pierde (`ampliarCapacidad`).
    pub fn ampliar(&mut self, r: Recurso, extra: Mil) {
        let i = r.indice();
        self.capacidad[i] = (self.capacidad[i] + extra).max(0);
        self.cantidad[i] = self.cantidad[i].min(entero(self.capacidad[i]));
        self.ajustar(i);
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
    fn lo_discreto_va_en_unidades_enteras_hacia_abajo() {
        let mut a = Almacen::nuevo(400_000);
        assert_eq!(a.agregar(Oro, 11_250), 11_000, "una comisión de 11,25 acuña 11");
        assert_eq!(a.quitar(Oro, 2_999), 2_000);
        a.poner(Cobre, 7_600);
        assert_eq!((a.cantidad(Oro), a.cantidad(Cobre)), (9_000, 7_000));
        // Un coste de 1,65 se cobra como 2.
        assert!(a.pagar(&[(Oro, 1_650)]));
        assert_eq!(a.cantidad(Oro), 7_000);
        assert!(!a.tiene(&[(Oro, 7_001)]));
    }

    #[test]
    fn un_flujo_da_en_n_minutos_exactamente_la_suma_entera() {
        let mut a = Almacen::nuevo(400_000);
        let mut visto = vec![];
        for _ in 0..10 {
            a.producir(Trigo, 370);
            visto.push(a.cantidad(Trigo) / 1000);
        }
        assert_eq!(visto, vec![0, 0, 1, 1, 1, 2, 2, 2, 3, 3], "0,37 por minuto: entra una unidad cada vez que el resto la completa");
        assert_eq!(a.disponible(Trigo), 3_700, "nada se pierde ni se crea: lo exacto es la suma");
        // Mil minutos de 1,65 de consumo sobre 2.000 de madera dejan exactamente 350.
        let mut b = Almacen::nuevo(4_000_000);
        b.poner(Madera, 2_000_000);
        for _ in 0..1000 {
            assert_eq!(b.consumir(Madera, 1_650), 1_650);
            assert_eq!(b.cantidad(Madera) % 1000, 0);
            assert!(b.cantidad(Madera) <= b.disponible(Madera), "lo visible nunca pasa de lo exacto");
        }
        assert_eq!((b.cantidad(Madera), b.disponible(Madera)), (350_000, 350_000));
    }

    #[test]
    fn el_consumo_no_pasa_de_lo_exacto_y_produccion_y_consumo_se_compensan_en_el_resto() {
        let mut a = Almacen::nuevo(400_000);
        a.poner(Trigo, 1_000);
        a.producir(Trigo, 600);
        assert_eq!((a.cantidad(Trigo), a.disponible(Trigo)), (1_000, 1_600));
        assert_eq!(a.consumir(Trigo, 900), 900);
        assert_eq!((a.cantidad(Trigo), a.disponible(Trigo)), (0, 700), "sale la unidad empezada; el resto guarda los 0,7");
        assert_eq!(a.consumir(Trigo, 5_000), 700, "no se come lo que no hay");
        assert_eq!(a.disponible(Trigo), 0);
    }

    #[test]
    fn el_flujo_se_para_en_la_capacidad() {
        let mut a = Almacen::nuevo(2_000);
        a.poner(Piedra, 1_000);
        assert_eq!(a.producir(Piedra, 700), 700);
        assert_eq!(a.producir(Piedra, 700), 300, "solo cabía hasta 2");
        assert_eq!((a.cantidad(Piedra), a.disponible(Piedra), a.hueco(Piedra)), (2_000, 2_000, 0));
        // Al llenarse por una entrada discreta, el resto que ya no cabe se pierde.
        let mut b = Almacen::nuevo(2_000);
        b.producir(Piedra, 1_500);
        assert_eq!(b.agregar(Piedra, 1_000), 1_000);
        assert_eq!(b.disponible(Piedra), 2_000);
    }

    #[test]
    fn ninguna_operacion_crea_recursos_y_todo_queda_en_unidades_enteras() {
        // Diez mil operaciones al azar: lo exacto nunca sube más de lo que la operación dice que entró (solo baja si lo que ya no cabe se pierde), lo que entra nunca
        // pasa de lo pedido y lo visible es siempre entero.
        let mut rng = base::Rng::nuevo(42);
        let mut a = Almacen::nuevo(50_000);
        for _ in 0..10_000 {
            let n = rng.por_debajo(9_000) as Mil;
            let antes = a.disponible(Trigo);
            let delta = match rng.por_debajo(5) {
                0 => a.agregar(Trigo, n),
                1 => -a.quitar(Trigo, n),
                2 => a.producir(Trigo, n),
                3 => -a.consumir(Trigo, n),
                _ => {
                    if a.pagar(&[(Trigo, n)]) {
                        -entero_arriba(n)
                    } else {
                        0
                    }
                }
            };
            assert!(delta <= n, "nunca entra más de lo pedido");
            assert!(a.disponible(Trigo) <= antes + delta, "nada se crea: {antes} {delta} {}", a.disponible(Trigo));
            let (c, k, r) = (a.cantidad(Trigo), a.capacidad(Trigo), a.partes().2[Trigo.indice()]);
            assert!(c % 1_000 == 0 && (0..1_000).contains(&r) && c + r <= k && c >= 0, "{c} {r} {k}");
        }
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
