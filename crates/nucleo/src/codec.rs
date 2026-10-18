//! Codificación binaria canónica (little endian) de las entradas del registro y de las fotos. Escrita a mano: el formato
//! tiene que ser estable y su orden de campos, fijo, porque `hash_estado` es el sha256 de estos bytes.

use base::Punto;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorFormato(pub &'static str);

impl fmt::Display for ErrorFormato {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "formato inválido: {}", self.0)
    }
}
impl std::error::Error for ErrorFormato {}

#[derive(Default)]
pub(crate) struct Escritor(pub Vec<u8>);

impl Escritor {
    pub fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    pub fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    pub fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    pub fn i32(&mut self, v: i32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    pub fn u64(&mut self, v: u64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    pub fn i64(&mut self, v: i64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    pub fn bool(&mut self, v: bool) {
        self.u8(v as u8);
    }
    pub fn texto(&mut self, s: &str) {
        self.u32(s.len() as u32);
        self.0.extend_from_slice(s.as_bytes());
    }
    pub fn punto(&mut self, p: Punto) {
        self.i64(p.x);
        self.i64(p.y);
    }
    pub fn opt_i64(&mut self, v: Option<i64>) {
        self.bool(v.is_some());
        self.i64(v.unwrap_or(0));
    }
    pub fn opt_u64(&mut self, v: Option<u64>) {
        self.bool(v.is_some());
        self.u64(v.unwrap_or(0));
    }
}

pub(crate) struct Lector<'a> {
    bytes: &'a [u8],
    i: usize,
}

impl<'a> Lector<'a> {
    pub fn nuevo(bytes: &'a [u8]) -> Self {
        Self { bytes, i: 0 }
    }
    fn tomar(&mut self, n: usize) -> Result<&'a [u8], ErrorFormato> {
        let fin = self.i.checked_add(n).filter(|&f| f <= self.bytes.len()).ok_or(ErrorFormato("datos truncados"))?;
        let s = &self.bytes[self.i..fin];
        self.i = fin;
        Ok(s)
    }
    pub fn u8(&mut self) -> Result<u8, ErrorFormato> {
        Ok(self.tomar(1)?[0])
    }
    pub fn u16(&mut self) -> Result<u16, ErrorFormato> {
        Ok(u16::from_le_bytes(self.tomar(2)?.try_into().unwrap()))
    }
    pub fn u32(&mut self) -> Result<u32, ErrorFormato> {
        Ok(u32::from_le_bytes(self.tomar(4)?.try_into().unwrap()))
    }
    pub fn i32(&mut self) -> Result<i32, ErrorFormato> {
        Ok(i32::from_le_bytes(self.tomar(4)?.try_into().unwrap()))
    }
    pub fn u64(&mut self) -> Result<u64, ErrorFormato> {
        Ok(u64::from_le_bytes(self.tomar(8)?.try_into().unwrap()))
    }
    pub fn i64(&mut self) -> Result<i64, ErrorFormato> {
        Ok(i64::from_le_bytes(self.tomar(8)?.try_into().unwrap()))
    }
    pub fn bool(&mut self) -> Result<bool, ErrorFormato> {
        Ok(self.u8()? != 0)
    }
    pub fn magia(&mut self) -> Result<[u8; 4], ErrorFormato> {
        Ok(self.tomar(4)?.try_into().unwrap())
    }
    pub fn texto(&mut self) -> Result<String, ErrorFormato> {
        let n = self.u32()? as usize;
        String::from_utf8(self.tomar(n)?.to_vec()).map_err(|_| ErrorFormato("texto no UTF-8"))
    }
    pub fn punto(&mut self) -> Result<Punto, ErrorFormato> {
        Ok(Punto::new(self.i64()?, self.i64()?))
    }
    pub fn opt_i64(&mut self) -> Result<Option<i64>, ErrorFormato> {
        let (hay, v) = (self.bool()?, self.i64()?);
        Ok(hay.then_some(v))
    }
    pub fn opt_u64(&mut self) -> Result<Option<u64>, ErrorFormato> {
        let (hay, v) = (self.bool()?, self.u64()?);
        Ok(hay.then_some(v))
    }
    /// Cuenta de elementos, acotada por los bytes que quedan (una cuenta absurda no reserva memoria).
    pub fn cuenta(&mut self) -> Result<usize, ErrorFormato> {
        let n = self.u64()? as usize;
        if n > self.bytes.len() - self.i {
            return Err(ErrorFormato("cuenta mayor que los datos"));
        }
        Ok(n)
    }
    pub fn fin(&self) -> Result<(), ErrorFormato> {
        if self.i == self.bytes.len() {
            Ok(())
        } else {
            Err(ErrorFormato("bytes sobrantes"))
        }
    }
}
