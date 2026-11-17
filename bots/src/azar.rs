//! El azar de los bots: SplitMix64, determinista por semilla. Sin dependencias: basta para sortear horarios, llegadas y decisiones.

pub struct Azar(u64);

impl Azar {
    pub fn new(semilla: u64) -> Self {
        Self(semilla)
    }

    pub fn siguiente(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// En [0, 1).
    pub fn real(&mut self) -> f64 {
        (self.siguiente() >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// FNV-1a: de un texto (un nick) a una semilla estable.
pub fn hash(texto: &str) -> u64 {
    texto.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3))
}
