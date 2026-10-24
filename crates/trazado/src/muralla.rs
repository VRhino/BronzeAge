//! Trazo de los recintos de muralla (Murallas.md §3, §4, §9.5): geometría pura sobre la rejilla de la [`Ciudad`].
//!
//! Dada una ciudad devuelve **dónde caería** su muralla: el anillo cerrado de celdas alrededor del casco urbano, en orden de
//! recorrido, con sus puertas y torres. No cobra, no persiste y no sabe de niveles de plaza: el coste (tarifas por clase), la
//! obra celda a celda y los efectos defensivos viven en `asentamiento` y `nucleo`.
//!
//! Todo se calcula en la rejilla de la ventana de ±[`MITAD`] celdas con vectores de `bool` indexados por celda: el orden de las
//! celdas es siempre el de índice (fila y columna) y nunca se itera un conjunto hash para decidir. Si el anillo no cabe en la
//! ventana el trazo se **rechaza** ([`ErrorMuralla::FueraDeVentana`]); nunca se recorta.
//!
//! El TypeScript (`typescript-final`) usa otro hash y otra red de calles, así que no hay paridad celda a celda: se vigilan las
//! mismas invariantes (cierra, no pisa, una puerta por cruce, como mucho cuatro puertas, torres separadas, determinismo).

use crate::ciudad::{celda_de, idx, inundar, vecinos_i, Ciudad, CALLE, CAMINO, CELDAS, LADO};
use crate::geom::Celda;
use crate::tipos::{Categoria, CeldaDeRed, Recinto};

/// Vueltas máximas del bucle que retira de la semilla lo que el muro pisaría (cada vuelta solo quita celdas).
const MAX_ITERACIONES_CONFLICTO: u32 = 24;

/// Clase de una celda del anillo. La torre es una celda de muro marcada, sin huella propia; la puerta no ocupa suelo.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ClaseCelda {
    Muro,
    Puerta,
    Torre,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CeldaMuro {
    pub celda: Celda,
    pub clase: ClaseCelda,
}

/// Lo que el trazo lee del bloque `MURALLA` de Murallas.md §11 (`franjaDeRonda`, `pasoTorres`, `puertasMaximas`, `anchoMaximoPuerta`); lo construye quien parsea el bloque.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParametrosMuralla {
    /// Pasos de dilatación del casco urbano para el anillo (vecindad 4).
    pub franja_de_ronda: i32,
    /// Celdas de separación entre torres según el nivel del recinto (índice `nivel - 1`); 0 = ese nivel no tiene torres.
    pub paso_torres: [i32; 3],
    /// Tope de puertas de un anillo (`puertasMaximas`, Cal-6).
    pub puertas_maximas: usize,
    /// Celdas de ancho que mide como mucho una puerta (`anchoMaximoPuerta`, Cal2-1).
    pub ancho_maximo_puerta: usize,
}

impl Default for ParametrosMuralla {
    fn default() -> Self {
        Self { franja_de_ronda: 1, paso_torres: [0, 8, 5], puertas_maximas: 4, ancho_maximo_puerta: 2 }
    }
}

impl ParametrosMuralla {
    fn paso_de(&self, nivel: u8) -> i32 {
        (nivel as usize).checked_sub(1).and_then(|k| self.paso_torres.get(k)).copied().unwrap_or(0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorMuralla {
    /// La ciudad no tiene Centro Urbano.
    SinCentroUrbano,
    /// «Un recinto que no se puede cerrar no es un recinto»: el casco queda vacío, el bucle de conflictos no converge o el anillo
    /// no aísla el Centro Urbano.
    NoSePuedeCerrar,
    /// El anillo (la región más una celda) no cabe en la ventana de ±[`crate::MITAD`] celdas.
    FueraDeVentana,
}

/// El anillo trazado. `celdas` va en orden de recorrido (el orden en que se levantará) y es lo que se congela al comprometer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trazo {
    pub celdas: Vec<CeldaMuro>,
    /// La red que el recinto conserva en sus puertas ([`Recinto::red_de_puertas`]), en orden de celda: se congela con el anillo.
    pub red_de_puertas: Vec<CeldaDeRed>,
    /// La pieza de mayor id de la ciudad al trazarlo ([`Recinto::tras_pieza`]).
    pub tras_pieza: u32,
    /// Celdas de suelo que el recinto encierra (incluye la franja de ronda).
    pub area_encerrada: usize,
    /// Ids de las piezas que quedarían dentro, en el orden de la ciudad.
    pub dentro: Vec<u32>,
    /// Ids de las piezas urbanas (no granjas ni corrales) que quedarían fuera: el arrabal del día 1.
    pub fuera: Vec<u32>,
    /// Granjas y corrales rodeados por la ciudad que el relleno de agujeros se traga (Murallas.md §3). Normalmente vacío.
    pub afueras_dentro: Vec<u32>,
}

impl Trazo {
    fn contar(&self, clase: ClaseCelda) -> usize {
        self.celdas.iter().filter(|c| c.clase == clase).count()
    }
    /// Las puertas del anillo: sus [`aberturas`] (Cal-6).
    pub fn puertas(&self) -> usize {
        aberturas(&self.celdas).len()
    }
    pub fn torres(&self) -> usize {
        self.contar(ClaseCelda::Torre)
    }
}

/// El interior de un recinto: las celdas que el anillo deja dentro.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Interior(Vec<bool>);

impl Interior {
    /// Fuera de la ventana no hay interior.
    pub fn contiene(&self, c: Celda) -> bool {
        idx(c).is_some_and(|i| self.0[i])
    }
    pub fn area(&self) -> usize {
        self.0.iter().filter(|&&d| d).count()
    }
}

impl Recinto {
    /// La proyección que el trazado necesita de un recinto (`muros` = muro y torre; `puertas` aparte), en orden de celda, con la red que conserva en sus puertas.
    pub fn desde_celdas(celdas: &[CeldaMuro], red_de_puertas: &[CeldaDeRed], tras_pieza: u32, completo: bool) -> Recinto {
        let de = |clase: fn(ClaseCelda) -> bool| celdas.iter().filter(|c| clase(c.clase)).map(|c| c.celda).collect::<Vec<_>>();
        let (mut muros, mut puertas) = (de(|k| k != ClaseCelda::Puerta), de(|k| k == ClaseCelda::Puerta));
        muros.sort();
        puertas.sort();
        Recinto { muros, puertas, red_de_puertas: red_de_puertas.to_vec(), tras_pieza, completo }
    }
}

// ---- rejilla ----

fn vacio() -> Vec<bool> {
    vec![false; CELDAS]
}

fn cuenta(m: &[bool]) -> usize {
    m.iter().filter(|&&b| b).count()
}

/// Celdas del borde de la ventana (todo lo de fuera es exterior).
fn borde() -> impl Iterator<Item = usize> {
    let l = LADO as usize;
    (0..CELDAS).filter(move |&i| i < l || i >= CELDAS - l || i % l == 0 || i % l == l - 1)
}

/// Dilatación con vecindad 4: sigue la forma real de la ciudad y achaflana las esquinas.
fn dilatar(conj: &[bool], pasos: i32) -> Vec<bool> {
    let mut actual = conj.to_vec();
    for _ in 0..pasos {
        let mut siguiente = actual.clone();
        for i in (0..CELDAS).filter(|&i| actual[i]) {
            for n in vecinos_i(i).into_iter().flatten() {
                siguiente[n] = true;
            }
        }
        actual = siguiente;
    }
    actual
}

/// Toda celda fuera de la región y sin salida al exterior pasa a la región (si no, un patio cerrado daría un anillo interior).
fn rellenar_agujeros(region: &[bool]) -> Vec<bool> {
    let exterior = inundar(borde(), |i| !region[i]);
    exterior.iter().map(|&e| !e).collect()
}

/// El muro: la capa 4-adyacente exterior a la región (impermeable con una sola celda de grosor). `None` si la región toca el
/// borde de la ventana y el anillo se saldría.
fn anillo(region: &[bool]) -> Option<Vec<bool>> {
    let mut muro = vacio();
    for i in (0..CELDAS).filter(|&i| region[i]) {
        for n in vecinos_i(i) {
            let n = n?;
            if !region[n] {
                muro[n] = true;
            }
        }
    }
    Some(muro)
}

fn celdas_de_pieza(r: &crate::geom::Rect) -> Vec<usize> {
    r.celdas().filter_map(idx).collect()
}

/// Desde fuera, sin cruzar `muro`, ¿se llega a alguna de `objetivo`? Las puertas se cruzan (no están en `muro`).
fn se_llega_desde_fuera(muro: &[bool], objetivo: &[usize]) -> bool {
    let alcanzadas = inundar(borde(), |i| !muro[i]);
    objetivo.iter().any(|&i| alcanzadas[i])
}

/// Trazo del recinto de `nivel` para la ciudad (Murallas.md §3). Determinista. La red de calles es la de la ciudad **sin recintos**:
/// el trazo se calcula sobre la ciudad, no sobre sí mismo (y una ampliación no ve el anillo viejo como muro).
pub fn trazar_recinto(ciudad: &Ciudad, p: &ParametrosMuralla, nivel: u8) -> Result<Trazo, ErrorMuralla> {
    let centro: Vec<usize> = celdas_de_pieza(&ciudad.centro().ok_or(ErrorMuralla::SinCentroUrbano)?.rect);
    let sin_recintos;
    let base = if ciudad.recintos().is_empty() {
        ciudad
    } else {
        sin_recintos = {
            let mut c = ciudad.clone();
            c.set_recintos(Vec::new());
            c
        };
        &sin_recintos
    };
    let piezas = ciudad.piezas();
    let por_pieza: Vec<Vec<usize>> = piezas.iter().map(|q| celdas_de_pieza(&q.rect)).collect();
    let afueras = |k: usize| piezas[k].categoria == Categoria::Afueras;

    // 1. vetadas: granjas y corrales con un margen de una celda.
    let mut vetadas = vacio();
    for k in (0..piezas.len()).filter(|&k| afueras(k)) {
        por_pieza[k].iter().for_each(|&i| vetadas[i] = true);
    }
    let vetadas = dilatar(&vetadas, 1);

    // 2. núcleo: edificios urbanos y CALLES (nunca caminos: arrastrarían la región hasta las granjas) que cuelgan del Centro Urbano.
    let mut bruto = vacio();
    for k in (0..piezas.len()).filter(|&k| !afueras(k)) {
        por_pieza[k].iter().filter(|&&i| !vetadas[i]).for_each(|&i| bruto[i] = true);
    }
    for i in (0..CELDAS).filter(|&i| base.suelo.red[i] == CALLE && !vetadas[i]) {
        bruto[i] = true;
    }
    let mut semilla = inundar(centro.iter().copied(), |i| bruto[i]);
    if cuenta(&semilla) == 0 {
        return Err(ErrorMuralla::NoSePuedeCerrar);
    }

    // 3-5. región y anillo, retirando de la semilla lo que el muro pisaría.
    let mut resuelto = None;
    for _ in 0..MAX_ITERACIONES_CONFLICTO {
        let dilatada = dilatar(&semilla, p.franja_de_ronda);
        let conexa = inundar(centro.iter().copied(), |i| dilatada[i] && !vetadas[i]);
        if cuenta(&conexa) == 0 {
            return Err(ErrorMuralla::NoSePuedeCerrar);
        }
        let region = rellenar_agujeros(&conexa);
        let muro = anillo(&region).ok_or(ErrorMuralla::FueraDeVentana)?;
        let mut a_quitar = vacio();
        let mut hay_conflicto = false;
        for cs in por_pieza.iter().filter(|cs| cs.iter().any(|&i| muro[i])) {
            hay_conflicto = true;
            cs.iter().for_each(|&i| a_quitar[i] = true);
        }
        if !hay_conflicto {
            resuelto = Some((region, muro));
            break;
        }
        let zona = dilatar(&a_quitar, p.franja_de_ronda + 1);
        let recortada: Vec<bool> = (0..CELDAS).map(|i| semilla[i] && !zona[i]).collect();
        if cuenta(&recortada) == cuenta(&semilla) {
            return Err(ErrorMuralla::NoSePuedeCerrar);
        }
        semilla = inundar(centro.iter().copied(), |i| recortada[i]);
        if cuenta(&semilla) == 0 {
            return Err(ErrorMuralla::NoSePuedeCerrar);
        }
    }
    let (region, muro) = resuelto.ok_or(ErrorMuralla::NoSePuedeCerrar)?;

    // 6. el anillo tiene que aislar de verdad.
    if se_llega_desde_fuera(&muro, &centro) {
        return Err(ErrorMuralla::NoSePuedeCerrar);
    }

    // 7-8. puertas (donde el muro pisa la red) → orden de recorrido → una puerta por cruce → torres.
    let en_red: Vec<bool> = base.suelo.red.iter().map(|&r| r != 0).collect();
    let por_celda = (0..CELDAS)
        .filter(|&i| muro[i])
        .map(|i| CeldaMuro { celda: celda_de(i), clase: if en_red[i] { ClaseCelda::Puerta } else { ClaseCelda::Muro } });
    let mut celdas = ordenar_anillo(por_celda.collect());
    colapsar_puertas(&mut celdas, &region, &muro, &en_red);
    reabrir_cortes(&mut celdas, &en_red, base);
    let piezas_hasta = piezas.iter().map(|q| q.id).max().unwrap_or(0);
    let valida = |celdas: &[CeldaMuro], cortados: &[usize]| sigue_conectada(base, celdas, &red_de_puertas(base, celdas, cortados, &region, &muro), piezas_hasta);
    let estrechadas = estrechar_puertas(&mut celdas, &region, &muro, &en_red, p.ancho_maximo_puerta);
    if !estrechadas.is_empty() && !valida(&celdas, &estrechadas) {
        return Err(ErrorMuralla::NoSePuedeCerrar);
    }
    let cortados = limitar_puertas(&mut celdas, &base.suelo.red, &region, &muro, p.puertas_maximas, estrechadas, valida).ok_or(ErrorMuralla::NoSePuedeCerrar)?;
    let red_de_puertas = red_de_puertas(base, &celdas, &cortados, &region, &muro);
    let interior = Interior(region);
    marcar_torres(&mut celdas, &interior, p, nivel);

    let (mut dentro, mut fuera, mut afueras_dentro) = (Vec::new(), Vec::new(), Vec::new());
    for (k, q) in piezas.iter().enumerate() {
        let encerrada = por_pieza[k].iter().all(|&i| interior.0[i]);
        match (afueras(k), encerrada) {
            (false, true) => dentro.push(q.id),
            (false, false) => fuera.push(q.id),
            (true, true) => afueras_dentro.push(q.id),
            (true, false) => {}
        }
    }
    let tras_pieza = piezas_hasta;
    Ok(Trazo { area_encerrada: interior.area(), celdas, red_de_puertas, tras_pieza, dentro, fuera, afueras_dentro })
}

/// La red que el recinto conserva en cada puerta (decisión del autor del 2026-10-24, calibración del sistema 4, Murallas M1): la puerta y, a cada lado, el tramo de la
/// red de antes del muro que la toca sin cruzar el anillo (fuera, el camino de la Granja o el arrabal; dentro, la calle que llega). Al rehacer la red con el muro se
/// siembra **conectada** ([`Ciudad`] la une a la red con un corredor si hace falta): nunca queda una isla (lo que vio el TypeScript al sembrar solo las puertas). Sin
/// esto, el corredor de una pieza de fuera que pasaba por donde ahora está el muro podía no hallar otro dentro del cap, y fuera del anillo no quedaba red a la que
/// pegarse.
/// Un cruce **cortado** (Cal-6, [`limitar_puertas`]) conserva igual su tramo a cada lado, pero no la celda, que es muro: al sembrarlo, la red de fuera rodea el muro
/// hasta la red más cercana (`Ciudad::sembrar_antes_de`), que es la de la puerta más cercana.
fn red_de_puertas(base: &Ciudad, celdas: &[CeldaMuro], cortados: &[usize], region: &[bool], muro: &[bool]) -> Vec<CeldaDeRed> {
    let red = &base.suelo.red;
    let mut conserva = vacio();
    let puertas = celdas.iter().filter(|c| c.clase == ClaseCelda::Puerta).filter_map(|c| idx(c.celda));
    for (d, abierta) in puertas.map(|d| (d, true)).chain(cortados.iter().map(|&d| (d, false))) {
        conserva[d] = abierta;
        // A cada lado, la red de antes que toca la puerta sin cruzar el anillo: fuera, el camino o la calle que sale; dentro, la que llega.
        for lado in [false, true] {
            let tramo = inundar(vecinos_i(d).into_iter().flatten(), |i| red[i] != 0 && region[i] == lado && !muro[i]);
            (0..CELDAS).filter(|&i| tramo[i]).for_each(|i| conserva[i] = true);
        }
    }
    (0..CELDAS).filter(|&i| conserva[i]).map(|i| CeldaDeRed { celda: celda_de(i), camino: red[i] == CAMINO }).collect()
}

/// El anillo en orden de recorrido desde la puerta de menor `(fila, columna)` (la «principal»; sin puertas, la celda de menor
/// `(fila, columna)`), por vecindad 8 probando `dr` y `dc` de −1 a 1. En un punto grueso (una esquina de la capa 4-adyacente) el
/// paseo puede quedar sin salida dejando celdas atrás: entonces sigue desde la celda sin visitar más cercana (Chebyshev, y a
/// igual distancia la de menor `(fila, columna)`), de modo que no se pierde ninguna y el desorden se queda en un salto local.
/// Recibe las celdas ya en orden de índice. El TypeScript añadía al final lo que faltara por `(fila, columna)`; el orden del Rust es el que
/// vale (decisión del autor, 2026-10-23, Murallas §3).
fn ordenar_anillo(celdas: Vec<CeldaMuro>) -> Vec<CeldaMuro> {
    if celdas.is_empty() {
        return celdas;
    }
    let mut pos = vec![usize::MAX; CELDAS];
    for (k, c) in celdas.iter().enumerate() {
        pos[idx(c.celda).expect("el anillo está en la ventana")] = k;
    }
    let mut visitada = vec![false; celdas.len()];
    let mut orden = Vec::with_capacity(celdas.len());
    let mut actual = celdas.iter().position(|c| c.clase == ClaseCelda::Puerta).unwrap_or(0);
    loop {
        visitada[actual] = true;
        orden.push(celdas[actual]);
        let Celda { col, row } = celdas[actual].celda;
        let vecina = (-1..=1)
            .flat_map(|dr| (-1..=1).map(move |dc| (dc, dr)))
            .filter(|&d| d != (0, 0))
            .filter_map(|(dc, dr)| idx(Celda::new(col + dc, row + dr)).map(|i| pos[i]))
            .find(|&q| q != usize::MAX && !visitada[q]);
        let distancia = |k: usize| (celdas[k].celda.col - col).abs().max((celdas[k].celda.row - row).abs());
        match vecina.or_else(|| (0..celdas.len()).filter(|&k| !visitada[k]).min_by_key(|&k| distancia(k))) {
            Some(k) => actual = k,
            None => return orden,
        }
    }
}

/// Una puerta por cruce: de cada tirada contigua (recorrida circularmente) de puertas se conserva una sola —la primera que
/// «atraviesa» (red dentro de la región a un lado y red fuera del muro al otro) o, si ninguna, la del medio— y el resto vuelve a
/// ser muro. El nº de puertas es el divisor del bono defensivo: un camino que rozara el muro cinco celdas dividiría la defensa por cinco.
fn colapsar_puertas(anillo: &mut [CeldaMuro], region: &[bool], muro: &[bool], en_red: &[bool]) {
    let n = anillo.len();
    let atraviesa = |c: &CeldaMuro| atraviesa(c.celda, region, muro, en_red);
    let mut visto = vec![false; n];
    for i in 0..n {
        if anillo[i].clase != ClaseCelda::Puerta || visto[i] {
            continue;
        }
        // retroceder hasta el principio real de la tirada (puede envolver el final)
        let mut inicio = i;
        loop {
            let previo = (inicio + n - 1) % n;
            if anillo[previo].clase == ClaseCelda::Puerta && previo != i {
                inicio = previo;
            } else {
                break;
            }
        }
        let mut tirada = Vec::new();
        let mut j = inicio;
        while anillo[j].clase == ClaseCelda::Puerta && tirada.len() < n && !visto[j] {
            visto[j] = true;
            tirada.push(j);
            j = (j + 1) % n;
        }
        if tirada.len() <= 1 {
            continue;
        }
        let elegida = tirada.iter().copied().find(|&k| atraviesa(&anillo[k])).unwrap_or(tirada[tirada.len() / 2]);
        for k in tirada.into_iter().filter(|&k| k != elegida) {
            anillo[k].clase = ClaseCelda::Muro;
        }
    }
}

/// ¿«Atraviesa» la celda del anillo? Tiene red dentro de la región a un lado y red fuera del muro al otro.
fn atraviesa(c: Celda, region: &[bool], muro: &[bool], en_red: &[bool]) -> bool {
    let (mut dentro, mut fuera) = (false, false);
    for v in vecinos_i(idx(c).expect("el anillo está en la ventana")).into_iter().flatten().filter(|&v| en_red[v]) {
        dentro |= region[v];
        fuera |= !region[v] && !muro[v];
    }
    dentro && fuera
}

/// Una puerta mide como mucho `ancho` celdas (Cal2-1, decisión del autor del 2026-10-24; `anchoMaximoPuerta`, 2): más ancha deja de ser un cuello de botella. De
/// una abertura más ancha —el paso que reabre [`reabrir_cortes`] por una calle pegada al muro— quedan como puerta las `ancho` celdas centradas en el cruce (la
/// primera que atraviesa o, si ninguna, la del medio, y sus vecinas de la abertura más cercanas al medio) y el resto vuelve a ser muro. Devuelve las celdas cerradas
/// (índices de la rejilla): su red a cada lado se conserva y rodea el muro hasta la puerta, como la de un cruce cortado ([`red_de_puertas`]).
fn estrechar_puertas(anillo: &mut [CeldaMuro], region: &[bool], muro: &[bool], en_red: &[bool], ancho: usize) -> Vec<usize> {
    let mut cerradas = Vec::new();
    for grupo in aberturas(anillo).into_iter().filter(|g| g.len() > ancho) {
        let medio = grupo.len() / 2;
        let cruce = grupo.iter().position(|&k| atraviesa(anillo[k].celda, region, muro, en_red)).unwrap_or(medio);
        // `ancho` celdas seguidas de la abertura (en el orden del anillo) que contienen al cruce, las más cercanas al medio.
        let desde = medio.saturating_sub(ancho / 2).clamp((cruce + 1).saturating_sub(ancho), cruce).min(grupo.len() - ancho);
        for &k in grupo.iter().enumerate().filter(|&(j, _)| j < desde || j >= desde + ancho).map(|(_, k)| k) {
            anillo[k].clase = ClaseCelda::Muro;
            cerradas.push(idx(anillo[k].celda).expect("el anillo está en la ventana"));
        }
    }
    cerradas
}

/// Una puerta por cruce no puede partir la red (`LAYOUT_VERSION` 5, Murallas M1): cuando una calle o un camino corre pegado al anillo, las celdas de la tirada que se
/// cerraron pueden ser el único paso entre la red que llega al Centro Urbano y otra parte de ella (un barrio de dentro, el camino de la Granja): sin ellas la ciudad
/// se queda sin salida. Se reabre como puerta el paso más corto por esas celdas (búsqueda en anchura desde las que tocan la red que llega, en el orden del anillo)
/// hasta la red que no llega, de uno en uno, hasta que la red de antes, con el muro, vuelva a ser una pieza desde el anillo de calle del Centro Urbano.
fn reabrir_cortes(anillo: &mut [CeldaMuro], en_red: &[bool], base: &Ciudad) {
    let Some(cu) = base.centro() else { return };
    let desde: Vec<usize> = cu.rect.anillo(base.params().ancho_calle).into_iter().filter_map(idx).filter(|&i| en_red[i]).collect();
    let mut pos = vec![usize::MAX; CELDAS];
    for (k, c) in anillo.iter().enumerate() {
        pos[idx(c.celda).expect("el anillo está en la ventana")] = k;
    }
    loop {
        let cerrada = |i: usize| pos[i] != usize::MAX && anillo[pos[i]].clase != ClaseCelda::Puerta && en_red[i];
        let llega = inundar(desde.iter().copied(), |i| en_red[i] && !cerrada(i));
        let mut previa = vec![usize::MAX; CELDAS];
        let mut cola: std::collections::VecDeque<usize> = anillo
            .iter()
            .filter_map(|c| idx(c.celda))
            .filter(|&i| cerrada(i) && vecinos_i(i).into_iter().flatten().any(|v| llega[v]))
            .inspect(|&i| previa[i] = i)
            .collect();
        let mut paso = None;
        while let Some(i) = cola.pop_front() {
            if vecinos_i(i).into_iter().flatten().any(|v| en_red[v] && !cerrada(v) && !llega[v]) {
                paso = Some(i);
                break;
            }
            for n in vecinos_i(i).into_iter().flatten() {
                if cerrada(n) && previa[n] == usize::MAX {
                    previa[n] = i;
                    cola.push_back(n);
                }
            }
        }
        let Some(mut i) = paso else { break };
        loop {
            anillo[pos[i]].clase = ClaseCelda::Puerta;
            if previa[i] == i {
                break;
            }
            i = previa[i];
        }
    }
    // Ni a una pieza (`LAYOUT_VERSION` 6): la que tenía salida y solo la tenía por celdas de red que el anillo cerró —encajada en un entrante del muro— recupera la
    // primera de ellas como puerta (suele ser vecina de la puerta del cruce, y entonces es la misma abertura).
    for q in base.piezas().iter().filter(|q| q.rol != crate::tipos::Rol::Centro) {
        let frente: Vec<usize> = q.rect.adyacentes().into_iter().filter_map(idx).filter(|&i| en_red[i]).collect();
        let cerrada = |i: usize| pos[i] != usize::MAX && anillo[pos[i]].clase != ClaseCelda::Puerta;
        if !frente.is_empty() && frente.iter().all(|&i| cerrada(i)) {
            anillo[pos[frente[0]]].clase = ClaseCelda::Puerta;
        }
    }
}

/// Las **puertas** de un anillo (Cal-6): cada abertura es una tirada 4-conexa de celdas de puerta, porque el paso que reabre [`reabrir_cortes`] por una calle pegada
/// al anillo es una sola puerta de varias celdas. Índices en `celdas`, cada abertura en orden de recorrido y las aberturas por su primera celda. Es lo que cuentan el
/// tope (`puertasMaximas`) y el divisor del bono defensivo; el coste se sigue pagando por celda.
pub fn aberturas(celdas: &[CeldaMuro]) -> Vec<Vec<usize>> {
    let mut pos = std::collections::HashMap::new();
    for (k, c) in celdas.iter().enumerate().filter(|(_, c)| c.clase == ClaseCelda::Puerta) {
        pos.insert(c.celda, k);
    }
    let mut vista = vec![false; celdas.len()];
    let mut grupos = Vec::new();
    for k in (0..celdas.len()).filter(|&k| celdas[k].clase == ClaseCelda::Puerta) {
        if std::mem::replace(&mut vista[k], true) {
            continue;
        }
        let (mut grupo, mut pila) = (vec![], vec![k]);
        while let Some(j) = pila.pop() {
            grupo.push(j);
            let Celda { col, row } = celdas[j].celda;
            for v in [(0, -1), (1, 0), (0, 1), (-1, 0)].into_iter().filter_map(|(dc, dr)| pos.get(&Celda::new(col + dc, row + dr)).copied()) {
                if !std::mem::replace(&mut vista[v], true) {
                    pila.push(v);
                }
            }
        }
        grupo.sort_unstable();
        grupos.push(grupo);
    }
    grupos
}

/// Ningún anillo tiene más de `maximas` puertas (Cal-6, decisión del autor del 2026-10-24; `LAYOUT_VERSION` 6). Si la red lo cruza por más sitios, las
/// [`aberturas`] se ordenan por la red de fuera que **más sirven**, contando solo la que no sirve ya otra mejor (la red de antes, fuera del anillo, que toca la
/// abertura sin cruzarlo): primero las que sirven algún camino —la salida de las Granjas—, después las de más celdas de red y, a igualdad, la primera del anillo.
/// Se cortan, enteras y de la que menos sirve a la que más, mientras sobren: cada corte vuelve muro sus celdas y su red de fuera rodea el muro hasta la puerta más
/// cercana ([`red_de_puertas`]); un corte que partiría la red o dejaría sin salida a una pieza que la tenía (`sigue_conectada`) no se hace. Devuelve las celdas
/// cerradas (las `ya_cerradas` al estrechar las puertas más las cortadas), o `None` si ni así se baja del tope (ese anillo no se puede cerrar con la regla).
fn limitar_puertas(anillo: &mut [CeldaMuro], red: &[u8], region: &[bool], muro: &[bool], maximas: usize, ya_cerradas: Vec<usize>, valida: impl Fn(&[CeldaMuro], &[usize]) -> bool) -> Option<Vec<usize>> {
    let puertas = aberturas(anillo);
    if puertas.len() <= maximas {
        return Some(ya_cerradas);
    }
    let indices: Vec<usize> = anillo.iter().map(|c| idx(c.celda).expect("el anillo está en la ventana")).collect();
    let en = |k: usize| indices[k];
    let servida: Vec<Vec<usize>> = puertas
        .iter()
        .map(|grupo| {
            let fuera = inundar(grupo.iter().flat_map(|&k| vecinos_i(en(k))).flatten(), |i| red[i] != 0 && !region[i] && !muro[i]);
            (0..CELDAS).filter(|&i| fuera[i]).collect()
        })
        .collect();
    // El orden de las aberturas, de la que más red nueva sirve a la que menos.
    let (mut cubierta, mut libres, mut orden) = (vacio(), (0..puertas.len()).collect::<Vec<_>>(), Vec::new());
    while !libres.is_empty() {
        let valor = |j: usize| {
            let nuevas = || servida[j].iter().filter(|&&i| !cubierta[i]);
            (nuevas().any(|&i| red[i] == CAMINO), nuevas().count(), std::cmp::Reverse(j))
        };
        let mejor = libres.iter().copied().max_by_key(|&j| valor(j)).expect("quedan");
        servida[mejor].iter().for_each(|&i| cubierta[i] = true);
        libres.retain(|&j| j != mejor);
        orden.push(mejor);
    }
    let (mut cortadas, mut quedan) = (ya_cerradas, puertas.len());
    for &j in orden.iter().rev() {
        if quedan <= maximas {
            break;
        }
        puertas[j].iter().for_each(|&k| anillo[k].clase = ClaseCelda::Muro);
        let con: Vec<usize> = cortadas.iter().copied().chain(puertas[j].iter().map(|&k| en(k))).collect();
        if valida(anillo, &con) {
            cortadas = con;
            quedan -= 1;
        } else {
            puertas[j].iter().for_each(|&k| anillo[k].clase = ClaseCelda::Puerta);
        }
    }
    (quedan <= maximas).then_some(cortadas)
}

/// ¿Sigue la red de la ciudad, con este anillo, siendo una pieza, y toda pieza que tenía salida la conserva? Se rehace la red con el recinto, como al comprometerlo.
fn sigue_conectada(base: &Ciudad, celdas: &[CeldaMuro], red_de_puertas: &[CeldaDeRed], tras_pieza: u32) -> bool {
    let mut c = base.clone();
    c.set_recintos(vec![Recinto::desde_celdas(celdas, red_de_puertas, tras_pieza, false)]);
    let red = &c.suelo.red;
    let total = red.iter().filter(|&&r| r != 0).count();
    let una_pieza = red.iter().position(|&r| r != 0).is_none_or(|i| cuenta(&inundar([i], |n| red[n] != 0)) == total);
    una_pieza && base.piezas().iter().all(|q| q.rol == crate::tipos::Rol::Centro || !base.suelo.tiene_frente(&q.rect) || c.suelo.tiene_frente(&q.rect))
}

/// Marca las torres de un anillo ya ordenado (solo niveles con `paso_torres`; la empalizada no tiene). **Borra antes las que
/// hubiera**, de modo que al mejorar de nivel se recalculan sobre las celdas de muro y no se juntan. Recorre el anillo con un
/// contador que empieza en `paso`: una puerta solo cuenta; un muro se hace torre si lleva `paso` desde la última y es esquina
/// convexa del `interior` (lo tiene dentro en su vertical y en su horizontal), o si lleva `2 × paso` sin ninguna.
pub fn marcar_torres(celdas: &mut [CeldaMuro], interior: &Interior, p: &ParametrosMuralla, nivel: u8) {
    for c in celdas.iter_mut().filter(|c| c.clase == ClaseCelda::Torre) {
        c.clase = ClaseCelda::Muro;
    }
    let paso = p.paso_de(nivel);
    if paso <= 0 {
        return;
    }
    let dentro = |col, row| interior.contiene(Celda::new(col, row));
    let mut desde_la_ultima = paso;
    for c in celdas.iter_mut() {
        desde_la_ultima += 1;
        if c.clase == ClaseCelda::Puerta {
            continue;
        }
        let Celda { col, row } = c.celda;
        let esquina_convexa = (dentro(col, row - 1) || dentro(col, row + 1)) && (dentro(col - 1, row) || dentro(col + 1, row));
        if desde_la_ultima >= paso && (esquina_convexa || desde_la_ultima >= 2 * paso) {
            desde_la_ultima = 0;
            c.clase = ClaseCelda::Torre;
        }
    }
}

/// El interior de un anillo ya congelado: inundación 4-conexa desde el Centro Urbano bloqueada por **todas** las celdas del anillo,
/// puertas incluidas (la puerta es un hueco para quien camina, no un agujero en la contabilidad). Da en cualquier instante el mismo
/// resultado que la región del trazo. `None` si no hay Centro Urbano o el anillo lo pisa.
pub fn interior_de(ciudad: &Ciudad, celdas: &[CeldaMuro]) -> Option<Interior> {
    let cu = ciudad.centro()?.rect;
    let mut anillo = vacio();
    celdas.iter().filter_map(|c| idx(c.celda)).for_each(|i| anillo[i] = true);
    let inicio = idx(Celda::new(cu.col, cu.row))?;
    (!anillo[inicio]).then(|| Interior(inundar([inicio], |i| !anillo[i])))
}

/// ¿El anillo `nuevo` envuelve **estrictamente** al `anterior`? Ninguna celda coincide y, desde fuera del nuevo sin cruzarlo, no se
/// llega a ninguna del viejo (el área encerrada nunca decrece; calcar el anillo no es ampliar).
pub fn contiene_a(nuevo: &[CeldaMuro], anterior: &[CeldaMuro]) -> bool {
    let mut muro = vacio();
    nuevo.iter().filter_map(|c| idx(c.celda)).for_each(|i| muro[i] = true);
    let objetivo: Vec<usize> = anterior.iter().filter_map(|c| idx(c.celda)).collect();
    !objetivo.iter().any(|&i| muro[i]) && !se_llega_desde_fuera(&muro, &objetivo)
}

/// Edificios urbanos (no granjas ni corrales) con alguna celda fuera del `interior`: el «extramuros» del gate de ampliación.
pub fn edificios_extramuros(ciudad: &Ciudad, interior: &Interior) -> usize {
    ciudad.piezas().iter().filter(|q| q.categoria != Categoria::Afueras && !q.rect.celdas().all(|c| interior.contiene(c))).count()
}
