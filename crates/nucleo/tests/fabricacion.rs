//! La fabricación de bronce dentro del núcleo (sistema 6, duda 4 de 6B, decidida por el autor: va en 6H). Una plaza del mundo real con un taller de bronce de verdad (su Fundición
//! mejorada al nivel que pide, sus artesanos y sus lingotes) fabrica en su paso de minuto, y lo fabricado suma a `fabricado.equipoBronce` y `fabricado.armaduraBronce` **exactamente una vez**:
//! lo que el contador sube (en milésimas, con su resto) es lo que entra en el almacén, se avance de un salto o minuto a minuto. Se omite sin el artefacto de mundo.

mod comun;

use asentamiento::catalogo::Tipo;
use asentamiento::Recurso;
use base::{Instante, MS_POR_MINUTO};
use comun::real::{self, M};
use nucleo::{Ajustes, Arranque, Comando, Nucleo, Peticion, Reglas};
use tecnologia::Contador;

const MIN: Instante = MS_POR_MINUTO;
const HORA: Instante = 60 * MIN;
const DIA: Instante = 24 * HORA;

/// Lo que fabrican las recetas de bronce (los cuatro de `fabricado.equipoBronce`).
const BRONCE: [Recurso; 4] = [Recurso::ArmaBronce, Recurso::ArmaBronceCalidad, Recurso::ArmaduraBronce, Recurso::ArmaduraBronceCalidad];

fn reglas() -> Reglas {
    let mut r = comun::reglas();
    let b = &mut std::sync::Arc::make_mut(&mut r.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    std::sync::Arc::make_mut(&mut r.economia.datos).capacidad_inicial = 100_000_000;
    r
}

fn arranque(m: &M) -> Arranque {
    Arranque { reglas: reglas(), ajustes: Ajustes::default(), semilla: 7, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) }
}

struct Taller {
    n: Nucleo,
    plaza: u64,
    k: u32,
}

impl Taller {
    fn ord(&mut self, c: Comando) -> nucleo::Respuesta {
        self.k += 1;
        self.n.aplicar_en(self.n.ahora(), Peticion::nueva("admin", &format!("o{}", self.k), c).de_admin())
    }

    fn ok(&mut self, c: Comando) {
        let r = self.ord(c);
        assert!(r.aceptada(), "{r:?}");
    }

    /// De todo lo que consumen las obras y las recetas, salvo lo que fabrican las de bronce (así el almacén de esas piezas solo crece por lo fabricado).
    fn llenar(&mut self) {
        for r in asentamiento::recurso::TODOS.into_iter().filter(|r| !BRONCE.contains(r)) {
            let p = self.plaza;
            // Un almacén ya lleno de ese recurso deja la orden sin cambios.
            self.ord(Comando::DarRecursos { asentamiento: p, recurso: r, cantidad: 20_000_000 });
        }
    }

    fn edificio(&self, tipo: Tipo) -> Option<(u32, u8, bool)> {
        self.n.economia(self.plaza).unwrap().edificios.iter().filter(|e| e.tipo == tipo).map(|e| (e.id, e.nivel_interno, e.activo() && e.mejora.is_none())).max_by_key(|e| e.1)
    }

    /// Levanta `tipo` y lo mejora hasta `nivel`, esperando cada obra.
    fn levantar(&mut self, tipo: Tipo, nivel: u8) {
        let p = self.plaza;
        loop {
            self.llenar();
            match self.edificio(tipo) {
                None => self.ok(Comando::AnadirEdificio { asentamiento: p, tipo }),
                Some((_, n, true)) if n >= nivel => return,
                Some((id, _, true)) => self.ok(Comando::MejorarEdificio { asentamiento: p, edificio: id }),
                Some(_) => {}
            }
            let fin = self.n.ahora() + 3 * DIA;
            while !self.edificio(tipo).is_some_and(|e| e.2) {
                assert!(self.n.ahora() < fin, "{tipo:?} no acabó su obra");
                let t = self.n.ahora() + 10 * MIN;
                self.n.avanzar_hasta(t);
            }
        }
    }

    /// (lo contado en `equipoBronce` y en `armaduraBronce`, en milésimas con el resto; lo que hay de las cuatro piezas y de la armadura en el almacén).
    fn medida(&self) -> (u64, u64, i64, i64) {
        let e = self.n.tecnologia();
        let mil = |c: Contador| e.contador(c) * 1000 + e.restos[c.indice()];
        let a = &self.n.economia(self.plaza).unwrap().almacen;
        (mil(Contador::FabricadoEquipoBronce), mil(Contador::FabricadoArmaduraBronce), BRONCE.iter().map(|&r| a.cantidad(r)).sum(), a.cantidad(Recurso::ArmaduraBronce))
    }
}

#[test]
fn un_taller_de_bronce_suma_a_su_contador_exactamente_lo_que_fabrica() {
    let Some(m) = real::mundo() else {
        eprintln!("sin artefacto de mundo real: se omite");
        return;
    };
    let mut n = Nucleo::nuevo(arranque(m));
    let p = real::sitio_rico(&n, &m.0, 0, 1);
    let plaza = real::fundar(&mut n, "ana", 1, p);
    let mut x = Taller { n, plaza, k: 0 };
    x.ok(Comando::AlternarAutoConstruccion { asentamiento: plaza, pausada: true });
    x.ok(Comando::FijarNivelAsentamiento { asentamiento: plaza, nivel: 3 });
    // Las recetas de bronce piden la aleación (las del armamento de calidad, el bronce de calidad militar): por administración, como en las pruebas de 6C.
    x.ok(Comando::FijarTecnologias { faccion: 1, tecnologias: ["metalurgia_cobre", "aleacion_bronce", "bronce_calidad_militar"].map(String::from).to_vec() });
    // El primer taller del catálogo con una receta de armas de bronce, en el nivel interno que la tiene, con el edificio que pide.
    let cat = x.n.estado().reglas.economia.datos.catalogo.clone();
    let (tipo, nivel, requiere) = asentamiento::catalogo::TIPOS
        .iter()
        .flat_map(|&t| cat.tipo(t).niveles.iter().enumerate().map(move |(i, n)| (t, i as u8 + 1, n)))
        .find(|(_, _, n)| n.recetas.iter().any(|r| r.produce == Recurso::ArmaBronce))
        .map(|(t, i, n)| (t, i, n.requiere_edificio))
        .expect("hay una receta de armas de bronce");
    if let Some((otro, nv)) = requiere {
        x.levantar(otro, nv);
    }
    x.levantar(tipo, nivel);
    // Hasta que el taller fabrica de verdad (artesanos y lingotes).
    let fin = x.n.ahora() + 7 * DIA;
    while x.medida().0 == 0 {
        assert!(x.n.ahora() < fin, "el taller no fabricó: {:?}", x.n.economia(plaza).unwrap().poblacion);
        x.llenar();
        let t = x.n.ahora() + HORA;
        x.n.avanzar_hasta(t);
    }
    // Un tramo de un salto y otro minuto a minuto: lo que sube cada contador es lo que entra en el almacén, ni más (contado dos veces) ni menos.
    for paso in [6 * HORA, MIN] {
        x.llenar();
        let antes = x.medida();
        let fin = x.n.ahora() + 6 * HORA;
        while x.n.ahora() < fin {
            let t = x.n.ahora() + paso;
            x.n.avanzar_hasta(t);
        }
        let despues = x.medida();
        let (equipo, armadura) = (despues.0 - antes.0, despues.1 - antes.1);
        assert!(equipo > 0, "el tramo fabricó algo (paso {paso} ms)");
        assert_eq!(equipo as i64, despues.2 - antes.2, "`fabricado.equipoBronce` sube lo que entra en el almacén de las cuatro piezas (paso {paso} ms)");
        assert_eq!(armadura as i64, despues.3 - antes.3, "`fabricado.armaduraBronce` sube lo que entra de armadura (paso {paso} ms)");
    }
}
