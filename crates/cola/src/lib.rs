//! Cola de eventos del núcleo (Reloj_Y_Eventos §2).
//!
//! Orden de salida: `(instante, prioridad de tipo, secuencia)`. La secuencia la asigna la cola, creciente, así que
//! con la misma entrada el orden es siempre el mismo. **Invalidación perezosa**: al cambiar de plan o de ritmo una
//! entidad sube su generación ([`Generaciones`]) y los eventos de generación anterior se descartan al salir.

use base::Instante;
use std::cmp::{Ordering, Reverse};
use std::collections::{BinaryHeap, HashMap};

pub const PRIORIDAD_MIN: u8 = 1;
pub const PRIORIDAD_MAX: u8 = 10;

/// A qué entidad y en qué generación de su plan/ritmo pertenece un evento.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Clave {
    pub entidad: u64,
    pub generacion: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Evento<E> {
    pub instante: Instante,
    /// Prioridad de tipo `1..=10` (menor sale antes con el mismo instante).
    pub prioridad: u8,
    pub secuencia: u64,
    pub clave: Clave,
    pub carga: E,
}

impl<E> Evento<E> {
    fn orden(&self) -> (Instante, u8, u64) {
        (self.instante, self.prioridad, self.secuencia)
    }
}

/// Envoltorio para el heap: ordena solo por `(instante, prioridad, secuencia)` (la secuencia es única: orden total).
struct Entrada<E>(Evento<E>);

impl<E> PartialEq for Entrada<E> {
    fn eq(&self, o: &Self) -> bool {
        self.0.orden() == o.0.orden()
    }
}
impl<E> Eq for Entrada<E> {}
impl<E> PartialOrd for Entrada<E> {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl<E> Ord for Entrada<E> {
    fn cmp(&self, o: &Self) -> Ordering {
        self.0.orden().cmp(&o.0.orden())
    }
}

pub struct Cola<E> {
    heap: BinaryHeap<Reverse<Entrada<E>>>,
    siguiente_secuencia: u64,
    descartados: u64,
}

impl<E> Default for Cola<E> {
    fn default() -> Self {
        Self::nueva()
    }
}

impl<E> Cola<E> {
    pub fn nueva() -> Self {
        Self { heap: BinaryHeap::new(), siguiente_secuencia: 0, descartados: 0 }
    }

    /// Encola un evento y devuelve su número de secuencia. `prioridad` debe estar en `1..=10`.
    pub fn programar(&mut self, instante: Instante, prioridad: u8, clave: Clave, carga: E) -> u64 {
        assert!((PRIORIDAD_MIN..=PRIORIDAD_MAX).contains(&prioridad), "prioridad fuera de 1..=10: {prioridad}");
        let secuencia = self.siguiente_secuencia;
        self.siguiente_secuencia += 1;
        self.heap.push(Reverse(Entrada(Evento { instante, prioridad, secuencia, clave, carga })));
        secuencia
    }

    /// Instante del primer evento, **sin** descartar los no vigentes (puede ser de un evento que saldrá descartado).
    pub fn proximo_instante(&self) -> Option<Instante> {
        self.heap.peek().map(|Reverse(e)| e.0.instante)
    }

    /// Saca el siguiente evento vigente con `instante <= limite`, descartando por el camino los no vigentes.
    pub fn siguiente_hasta(&mut self, limite: Instante, vigente: &impl Fn(Clave) -> bool) -> Option<Evento<E>> {
        while let Some(Reverse(e)) = self.heap.peek() {
            if e.0.instante > limite {
                return None;
            }
            let Reverse(Entrada(ev)) = self.heap.pop().expect("peek devolvió Some");
            if vigente(ev.clave) {
                return Some(ev);
            }
            self.descartados += 1;
        }
        None
    }

    /// Eventos en la cola (incluye los aún no descartados por invalidación).
    pub fn len(&self) -> usize {
        self.heap.len()
    }
    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }
    /// Cuántos eventos no vigentes se han descartado desde que se creó la cola.
    pub fn descartados(&self) -> u64 {
        self.descartados
    }
    /// Secuencia que recibirá el próximo evento (va en la foto junto al volcado).
    pub fn siguiente_secuencia(&self) -> u64 {
        self.siguiente_secuencia
    }

    /// Reconstruye la cola de una foto: mismas secuencias, mismo orden, mismo siguiente número de secuencia.
    /// El contador de descartados empieza en 0 (es diagnóstico, no estado de juego).
    pub fn restaurar(eventos: Vec<Evento<E>>, siguiente_secuencia: u64) -> Self {
        debug_assert!(eventos.iter().all(|e| e.secuencia < siguiente_secuencia));
        let heap = eventos.into_iter().map(|e| Reverse(Entrada(e))).collect();
        Self { heap, siguiente_secuencia, descartados: 0 }
    }
}

impl<E: Clone> Cola<E> {
    /// Instantánea: todos los eventos en orden de salida. Con [`Cola::siguiente_secuencia`] basta para [`Cola::restaurar`].
    pub fn volcar(&self) -> Vec<Evento<E>> {
        let mut v: Vec<Evento<E>> = self.heap.iter().map(|Reverse(e)| e.0.clone()).collect();
        v.sort_unstable_by_key(Evento::orden);
        v
    }
}

/// Generación vigente de cada entidad (0 si nunca se ha subido).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Generaciones(HashMap<u64, u32>);

impl Generaciones {
    pub fn nuevas() -> Self {
        Self::default()
    }
    pub fn actual(&self, entidad: u64) -> u32 {
        self.0.get(&entidad).copied().unwrap_or(0)
    }
    /// Invalida los eventos programados hasta ahora para la entidad; devuelve la generación nueva.
    pub fn subir(&mut self, entidad: u64) -> u32 {
        let g = self.0.entry(entidad).or_insert(0);
        *g += 1;
        *g
    }
    pub fn es_vigente(&self, clave: Clave) -> bool {
        self.actual(clave.entidad) == clave.generacion
    }
    /// Clave con la generación actual de la entidad: la que se usa al programar.
    pub fn clave(&self, entidad: u64) -> Clave {
        Clave { entidad, generacion: self.actual(entidad) }
    }
    /// Para las fotos: pares `(entidad, generación)` ordenados.
    pub fn volcar(&self) -> Vec<(u64, u32)> {
        let mut v: Vec<_> = self.0.iter().map(|(&e, &g)| (e, g)).collect();
        v.sort_unstable();
        v
    }
    pub fn restaurar(pares: impl IntoIterator<Item = (u64, u32)>) -> Self {
        Self(pares.into_iter().collect())
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use base::Rng;

    fn k(e: u64, g: u32) -> Clave {
        Clave { entidad: e, generacion: g }
    }
    fn todo(_: Clave) -> bool {
        true
    }
    fn vaciar<E>(c: &mut Cola<E>, v: &impl Fn(Clave) -> bool) -> Vec<Evento<E>> {
        std::iter::from_fn(|| c.siguiente_hasta(Instante::MAX, v)).collect()
    }

    #[test]
    fn orden_por_instante_prioridad_secuencia() {
        let mut c = Cola::nueva();
        c.programar(20, 1, k(1, 0), "tarde");
        c.programar(10, 5, k(1, 0), "p5 primero");
        c.programar(10, 5, k(2, 0), "p5 segundo");
        c.programar(10, 2, k(3, 0), "p2");
        c.programar(5, 10, k(4, 0), "temprano");
        let cargas: Vec<_> = vaciar(&mut c, &todo).iter().map(|e| e.carga).collect();
        assert_eq!(cargas, ["temprano", "p2", "p5 primero", "p5 segundo", "tarde"]);
    }

    #[test]
    fn secuencias_crecientes_y_empate_por_orden_de_llegada() {
        let mut c = Cola::nueva();
        let s: Vec<u64> = (0..5).map(|i| c.programar(7, 3, k(i, 0), i)).collect();
        assert_eq!(s, [0, 1, 2, 3, 4]);
        assert_eq!(c.siguiente_secuencia(), 5);
        let cargas: Vec<_> = vaciar(&mut c, &todo).iter().map(|e| e.carga).collect();
        assert_eq!(cargas, [0, 1, 2, 3, 4]);
    }

    #[test]
    fn limite_y_proximo_instante() {
        let mut c = Cola::nueva();
        assert_eq!(c.proximo_instante(), None);
        c.programar(100, 1, k(1, 0), ());
        c.programar(50, 1, k(2, 0), ());
        assert_eq!(c.proximo_instante(), Some(50));
        assert!(c.siguiente_hasta(49, &todo).is_none());
        assert_eq!(c.len(), 2);
        assert_eq!(c.siguiente_hasta(50, &todo).unwrap().instante, 50); // límite inclusivo
        assert!(c.siguiente_hasta(99, &todo).is_none());
        assert_eq!(c.siguiente_hasta(100, &todo).unwrap().instante, 100);
        assert!(c.is_empty());
    }

    #[test]
    fn subir_generacion_descarta() {
        let mut c = Cola::nueva();
        let mut g = Generaciones::nuevas();
        c.programar(10, 1, g.clave(1), "viejo a");
        c.programar(20, 1, g.clave(2), "otra entidad");
        c.programar(30, 1, g.clave(1), "viejo b");
        assert_eq!(g.subir(1), 1);
        c.programar(40, 1, g.clave(1), "nuevo");
        let v = |cl| g.es_vigente(cl);
        // proximo_instante no descarta: ve el evento viejo
        assert_eq!(c.proximo_instante(), Some(10));
        assert_eq!(c.descartados(), 0);
        let cargas: Vec<_> = vaciar(&mut c, &v).iter().map(|e| e.carga).collect();
        assert_eq!(cargas, ["otra entidad", "nuevo"]);
        assert_eq!(c.descartados(), 2);
    }

    #[test]
    fn descarte_respeta_el_limite() {
        let mut c = Cola::nueva();
        let mut g = Generaciones::nuevas();
        c.programar(100, 1, g.clave(1), ());
        g.subir(1);
        assert!(c.siguiente_hasta(50, &|cl| g.es_vigente(cl)).is_none());
        assert_eq!((c.len(), c.descartados()), (1, 0)); // aún no ha tocado
    }

    #[test]
    fn generaciones_basicas() {
        let mut g = Generaciones::nuevas();
        assert_eq!(g.actual(9), 0);
        assert!(g.es_vigente(k(9, 0)));
        g.subir(9);
        g.subir(9);
        assert_eq!(g.actual(9), 2);
        assert!(!g.es_vigente(k(9, 1)));
        assert_eq!(Generaciones::restaurar(g.volcar()), g);
    }

    #[test]
    #[should_panic]
    fn prioridad_invalida() {
        Cola::nueva().programar(0, 11, k(0, 0), ());
    }

    /// Operaciones aleatorias deterministas sobre una cola; devuelve la traza de salida.
    fn simular(semilla: u64, pasos: usize) -> Vec<(Instante, u8, u64, u64)> {
        let mut r = Rng::nuevo(semilla);
        let mut c = Cola::nueva();
        let mut g = Generaciones::nuevas();
        let mut ahora: Instante = 0;
        let mut traza = Vec::new();
        for _ in 0..pasos {
            match r.por_debajo(10) {
                0..=5 => {
                    let e = r.por_debajo(20);
                    let t = ahora + r.por_debajo(50) as Instante;
                    c.programar(t, 1 + r.por_debajo(10) as u8, g.clave(e), e);
                }
                6 => {
                    g.subir(r.por_debajo(20));
                }
                _ => {
                    ahora += r.por_debajo(30) as Instante;
                    while let Some(ev) = c.siguiente_hasta(ahora, &|cl| g.es_vigente(cl)) {
                        assert!(ev.instante <= ahora && g.es_vigente(ev.clave));
                        traza.push((ev.instante, ev.prioridad, ev.secuencia, ev.carga));
                    }
                }
            }
        }
        traza
    }

    #[test]
    fn determinismo_con_azar() {
        let a = simular(42, 20_000);
        assert!(a.len() > 1_000);
        assert_eq!(a, simular(42, 20_000));
        assert_ne!(a, simular(43, 20_000));
    }

    #[test]
    fn salida_ordenada_en_un_solo_vaciado() {
        let mut r = Rng::nuevo(7);
        let mut c = Cola::nueva();
        for i in 0..5_000u64 {
            c.programar(r.por_debajo(300) as Instante, 1 + r.por_debajo(10) as u8, k(i % 50, 0), i);
        }
        let v = vaciar(&mut c, &todo);
        assert_eq!(v.len(), 5_000);
        assert!(v.windows(2).all(|w| w[0].orden() < w[1].orden()));
    }

    #[test]
    fn volcar_restaurar_identico() {
        let mut r = Rng::nuevo(99);
        let mut c = Cola::nueva();
        let mut g = Generaciones::nuevas();
        for _ in 0..3_000 {
            let e = r.por_debajo(30);
            c.programar(r.por_debajo(200) as Instante, 1 + r.por_debajo(10) as u8, g.clave(e), e);
            if r.por_debajo(8) == 0 {
                g.subir(r.por_debajo(30));
            }
        }
        // Saca una parte antes de la foto, para que la secuencia siguiente no coincida con len().
        for _ in 0..500 {
            c.siguiente_hasta(Instante::MAX, &|cl| g.es_vigente(cl));
        }
        let volcado = c.volcar();
        assert_eq!(volcado.len(), c.len());
        let mut copia = Cola::restaurar(volcado.clone(), c.siguiente_secuencia());
        let mut gcopia = Generaciones::restaurar(g.volcar());
        assert_eq!(copia.siguiente_secuencia(), c.siguiente_secuencia());
        assert_eq!(copia.volcar(), volcado);
        // Mismo futuro: nuevas altas reciben las mismas secuencias y todo sale igual.
        for i in 0..100u64 {
            let s1 = c.programar(150, 4, g.clave(i), i);
            let s2 = copia.programar(150, 4, gcopia.clave(i), i);
            assert_eq!(s1, s2);
        }
        gcopia.subir(3);
        g.subir(3);
        let a = vaciar(&mut c, &|cl| g.es_vigente(cl));
        let b = vaciar(&mut copia, &|cl| gcopia.es_vigente(cl));
        assert_eq!(a, b);
    }
}
