//! Almacén de un asentamiento, de un pueblo o de una avanzada (Economia_Construccion §4): cantidad y capacidad por recurso. Sin reloj: el que
//! lo usa decide cuándo se agrega o se quita; aquí solo están las reglas de cabida.
//!
//! **Solo unidades enteras** (decisión C3b del autor, 2026-10-24): lo guardado es siempre un múltiplo de [`base::UNIDAD`]. Hay dos formas de mover:
//!
//! - **Discreta** ([`Almacen::agregar`], [`Almacen::quitar`], [`Almacen::pagar`], [`Almacen::devolver`], [`Almacen::poner`]): entra o sale un
//!   número entero de unidades. **Lo que se mueve se redondea hacia abajo; lo que se cobra, hacia arriba** (EE-1). Nunca se crea de más.
//! - **Flujo por minuto** ([`Almacen::producir`] y [`Almacen::consumir`]: producción, extracción, recetas, comida, mantenimiento, recaudación):
//!   la fracción va a un **resto oculto** por recurso, en `(−1000, 1000)`, como la población entera: positivo es una unidad a medio entrar,
//!   negativo una unidad a medio consumir. Lo exacto es lo guardado más el resto. Una unidad **entra** cuando el resto la completa y **sale cuando
//!   se completa su consumo** (EE-2), no al empezar a consumirse. Producción y consumo del mismo recurso se compensan en el resto (no se descarta al
//!   cambiar de signo: eso crearía o destruiría fracciones). En N minutos entra o sale exactamente la suma entera.
//!
//! Lo que se **ve** ([`Almacen::visible`]) es lo guardado: la unidad que se está consumiendo sigue ahí hasta acabarse. Lo que se **mueve, comercia
//! y paga** ([`Almacen::cantidad`]) es lo que hay entero de verdad: lo guardado sin esa unidad empezada.

use crate::recurso::{Recurso, TODOS};
use base::{entero, entero_arriba, Mil, UNIDAD};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Almacen {
    /// Unidades guardadas (múltiplo de 1000), con la que se está consumiendo.
    cantidad: [Mil; 29],
    capacidad: [Mil; 29],
    /// Resto oculto de los flujos, en `(−1000, 1000)`; lo exacto (`cantidad + resto`) nunca es negativo ni pasa de la capacidad. Nadie fuera de
    /// aquí lo lee salvo la foto.
    resto: [Mil; 29],
    /// Lo que entra por [`Almacen::producir`] y sale por [`Almacen::consumir`] mientras se registra ([`Almacen::registrar_flujos`]): el paso de minuto de la plaza lo abre y lo
    /// cierra ([`Almacen::tomar_flujos`]), así que fuera del paso siempre es `None` y no cuenta en la igualdad ni en la foto.
    flujos: Option<Box<Flujos>>,
}

/// Lo producido y lo consumido de cada recurso en un tramo (milésimas), por separado: producción y consumo del mismo recurso no se compensan aquí (mecánica 60).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Flujos {
    pub producido: [Mil; 29],
    pub consumido: [Mil; 29],
}

impl Almacen {
    /// Un almacén vacío con `capacidad` para cada uno de los 29 recursos (`ALMACEN.capacidadInicialPorRecurso`).
    pub fn nuevo(capacidad: Mil) -> Self {
        Self { cantidad: [0; 29], capacidad: [capacidad; 29], resto: [0; 29], flujos: None }
    }

    /// Las unidades guardadas, capacidades y restos de los 29 recursos, en orden de [`TODOS`] (para serializar).
    pub fn partes(&self) -> (&[Mil; 29], &[Mil; 29], &[Mil; 29]) {
        (&self.cantidad, &self.capacidad, &self.resto)
    }

    /// Inversa de [`Almacen::partes`] (restaurar de una foto). No recorta: se guardó un almacén válido.
    pub fn desde_partes(cantidad: [Mil; 29], capacidad: [Mil; 29], resto: [Mil; 29]) -> Self {
        Self { cantidad, capacidad, resto, flujos: None }
    }

    /// Empieza a registrar lo que entra y sale por los flujos (lo de antes se descarta).
    pub fn registrar_flujos(&mut self) {
        self.flujos = Some(Box::default());
    }

    /// Deja de registrar y devuelve lo registrado desde [`Almacen::registrar_flujos`] (todo a 0 si no se registraba).
    pub fn tomar_flujos(&mut self) -> Flujos {
        self.flujos.take().map_or_else(Flujos::default, |f| *f)
    }

    /// Unidades enteras que hay de verdad (múltiplo de 1000): lo que se mueve, se comercia, se carga y se paga. Es lo guardado sin la unidad
    /// que se está consumiendo, si la hay.
    pub fn cantidad(&self, r: Recurso) -> Mil {
        let i = r.indice();
        self.cantidad[i] - if self.resto[i] < 0 { UNIDAD } else { 0 }
    }

    /// Lo que se ve (múltiplo de 1000): lo guardado, con la unidad que se está consumiendo hasta que se acabe (EE-2).
    pub fn visible(&self, r: Recurso) -> Mil {
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
        entero((self.capacidad(r) - self.cantidad[r.indice()]).max(0))
    }

    /// Lo que todavía puede entrar por un flujo (milésimas): el hueco hasta la capacidad desde lo exacto.
    pub fn hueco(&self, r: Recurso) -> Mil {
        (self.capacidad(r) - self.disponible(r)).max(0)
    }

    /// Lo exacto nunca pasa de la capacidad (lo que no cabe se pierde, como siempre) ni queda negativo.
    fn ajustar(&mut self, i: usize) {
        self.resto[i] = self.resto[i].min((self.capacidad[i] - self.cantidad[i]).max(0));
        if self.cantidad[i] == 0 {
            self.resto[i] = self.resto[i].max(0);
        }
    }

    /// Fija lo guardado en unidades enteras (entregas iniciales, restaurar un estado); nunca por encima de la capacidad. Olvida un consumo a medias.
    pub fn poner(&mut self, r: Recurso, n: Mil) {
        let i = r.indice();
        self.cantidad[i] = entero(n).clamp(0, entero(self.capacidad[i].max(0)));
        self.resto[i] = self.resto[i].max(0);
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

    /// Quita las unidades enteras de `n`, hasta las que hay de verdad ([`Almacen::cantidad`]), y devuelve lo quitado.
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
        if let Some(f) = &mut self.flujos {
            f.producido[i] += entra;
        }
        self.resto[i] += entra;
        if self.resto[i] >= UNIDAD {
            let unidades = entero(self.resto[i]);
            self.cantidad[i] += unidades;
            self.resto[i] -= unidades;
        }
        entra
    }

    /// Flujo de salida de un minuto (comida, mantenimiento, insumos de una receta): `n` milésimas, hasta lo exacto que haya, salen del resto y
    /// salen del almacén las unidades cuyo consumo se completa (EE-2). Devuelve lo consumido (milésimas).
    pub fn consumir(&mut self, r: Recurso, n: Mil) -> Mil {
        let sale = n.max(0).min(self.disponible(r));
        let i = r.indice();
        if let Some(f) = &mut self.flujos {
            f.consumido[i] += sale;
        }
        self.resto[i] -= sale;
        if self.resto[i] <= -UNIDAD {
            let unidades = entero(-self.resto[i]);
            self.cantidad[i] -= unidades;
            self.resto[i] += unidades;
        }
        sale
    }

    /// ¿Hay para pagar el coste? Cada línea se cobra en unidades enteras, redondeada hacia arriba: como lo que hay ya es un múltiplo de 1000, tenerla
    /// redondeada hacia arriba es lo mismo que tener la línea tal cual (y así no se divide en cada pregunta de la auto-construcción).
    pub fn tiene(&self, costo: &[(Recurso, Mil)]) -> bool {
        costo.iter().all(|&(r, n)| self.cantidad(r) >= n)
    }

    /// Paga el costo entero (cada línea redondeada hacia arriba a unidades, EE-1) o no toca nada.
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
    fn mientras_se_registra_lo_producido_y_lo_consumido_se_suman_por_separado_y_solo_lo_que_entro_o_salio() {
        let mut a = Almacen::nuevo(10_000);
        a.poner(Trigo, 5_000);
        a.producir(Trigo, 1_000);
        assert_eq!(a.tomar_flujos(), Flujos::default(), "sin registrar no se apunta nada");
        a.registrar_flujos();
        a.producir(Trigo, 2_500);
        a.consumir(Trigo, 1_200);
        a.agregar(Trigo, 1_000);
        a.producir(Trigo, 9_000); // solo cabe lo que queda hasta la capacidad
        a.consumir(Madera, 500); // no hay madera: no sale nada
        let f = a.tomar_flujos();
        assert_eq!((f.producido[Trigo.indice()], f.consumido[Trigo.indice()], f.consumido[Madera.indice()]), (2_500 + 1_700, 1_200, 0), "producción y consumo no se compensan; lo discreto no cuenta");
        assert_eq!(a.tomar_flujos(), Flujos::default(), "tomar cierra el registro");
        assert_eq!(a, { let mut b = a.clone(); b.registrar_flujos(); b.tomar_flujos(); b }, "fuera del registro no cuenta en la igualdad");
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
            assert_eq!(b.visible(Madera) % 1000, 0);
            assert!(b.cantidad(Madera) <= b.disponible(Madera) && b.disponible(Madera) <= b.visible(Madera), "lo que se mueve nunca pasa de lo exacto; lo que se ve, la unidad empezada");
            assert!(b.visible(Madera) - b.disponible(Madera) < 1_000);
        }
        assert_eq!((b.cantidad(Madera), b.visible(Madera), b.disponible(Madera)), (350_000, 350_000, 350_000));
    }

    #[test]
    fn la_unidad_sale_al_completarse_su_consumo_como_en_la_poblacion() {
        let mut a = Almacen::nuevo(400_000);
        a.poner(Trigo, 10_000);
        let mut visto = vec![];
        for _ in 0..4 {
            a.consumir(Trigo, 500);
            visto.push((a.visible(Trigo) / 1000, a.cantidad(Trigo) / 1000));
        }
        assert_eq!(visto, vec![(10, 9), (9, 9), (9, 8), (8, 8)], "0,5 por minuto sobre 10: la unidad sale cada dos minutos, al acabarse; mientras tanto se ve pero no se mueve");
        assert_eq!(a.quitar(Trigo, 10_000), 8_000);
    }

    #[test]
    fn el_consumo_no_pasa_de_lo_exacto_y_produccion_y_consumo_se_compensan_en_el_resto() {
        let mut a = Almacen::nuevo(400_000);
        a.poner(Trigo, 1_000);
        a.producir(Trigo, 600);
        assert_eq!((a.cantidad(Trigo), a.disponible(Trigo)), (1_000, 1_600));
        assert_eq!(a.consumir(Trigo, 900), 900);
        assert_eq!((a.visible(Trigo), a.cantidad(Trigo), a.disponible(Trigo)), (1_000, 0, 700), "la unidad empezada sigue (se ve, no se mueve) hasta acabarse");
        assert_eq!(a.quitar(Trigo, 1_000), 0, "no se vende lo que ya se está comiendo");
        assert_eq!(a.consumir(Trigo, 5_000), 700, "no se come lo que no hay");
        assert_eq!((a.visible(Trigo), a.disponible(Trigo)), (0, 0));
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
            let (c, k, r) = (a.visible(Trigo), a.capacidad(Trigo), a.partes().2[Trigo.indice()]);
            assert!(c % 1_000 == 0 && (-999..1_000).contains(&r) && c + r <= k && c + r >= 0 && a.cantidad(Trigo) <= c + r, "{c} {r} {k}");
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
