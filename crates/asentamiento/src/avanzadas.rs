//! Las avanzadas (Doc 5.17, Economia_Construccion §14.1): puestos fortificados de la plaza madre, en el mapa. Tres etapas, que son sus tres niveles
//! (atalaya → fuerte → ciudadela); siempre se empieza por la atalaya y se sube de una en una. Es la otra **obra de mapa** además del pueblo
//! extractor (`pueblos`): no pasa por la cola de construcción, queda **pendiente de materiales** (ocupa cupo desde que se marca), los materiales se
//! entregan en el sitio y, con todo entregado, arranca la obra, que acaba en su instante exacto (el núcleo le da una cita propia, ligada a la avanzada
//! para poder cancelarla: no espera al paso de minuto). Una avanzada operativa cuenta como un edificio activo de la madre en el mantenimiento.
//!
//! Aquí están el estado de la avanzada (etapa, obra, almacén, ruinas), el cupo, los costes, dónde cabe una nueva y las órdenes sobre la obra (marcarla, entregarla,
//! cancelarla). Lo que tiene que ver con el combate (guarnición de serie y de jugadores, el ataque y qué queda tras él) es del núcleo y del crate `combate`: la
//! avanzada solo dice si está operativa, en obra o en ruinas.

use crate::almacen::Almacen;
use crate::asentamiento::{Asentamiento, Evento};
use crate::catalogo::costo;
use crate::datos::{mil, por, ppm, Reglas};
use crate::pueblos::{falta_de, tomar_materiales};
use crate::recurso::{Costo, Recurso};
use base::{Instante, Mil, Punto, MS_POR_MINUTO};
use serde_json::Value;

/// Las tres etapas de una avanzada, de menor a mayor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Etapa {
    Atalaya,
    Fuerte,
    Ciudadela,
}

impl Etapa {
    pub const TODAS: [Etapa; 3] = [Etapa::Atalaya, Etapa::Fuerte, Etapa::Ciudadela];

    pub fn indice(self) -> usize {
        self as usize
    }

    /// La etapa a la que se mejora desde esta; la ciudadela no tiene siguiente.
    pub fn siguiente(self) -> Option<Etapa> {
        Self::TODAS.get(self.indice() + 1).copied()
    }

    /// El nombre del documento (`atalaya`, `fuerte`, `ciudadela`).
    pub fn nombre(self) -> &'static str {
        ["atalaya", "fuerte", "ciudadela"][self.indice()]
    }
}

/// En qué está una avanzada.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Estado {
    /// Su primera obra aún no acabó: ocupa cupo, pero no es un puesto todavía (no se ve, no ve, no se ataca).
    EnConstruccion,
    Operativa,
    /// Sin visión, sin guarnición y sin reabastecimiento. Un fuerte recién arruinado es invulnerable hasta `invulnerable_hasta`; una ciudadela
    /// arruinada no lo es (no hay nada más que perder).
    EnRuinas { invulnerable_hasta: Option<Instante> },
    /// Derribada: el resultado de un ataque o de destruirla a mano. Una avanzada destruida sale del estado: nunca se guarda con este valor.
    Destruida,
}

/// Qué hace una obra de mapa sobre una avanzada.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TipoObra {
    /// Levantar la atalaya.
    Construir,
    /// Pasar a la etapa siguiente.
    Mejorar,
    /// Reparar unas ruinas.
    Reparar,
}

/// Una obra de mapa «pendiente de materiales» o ya en marcha.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Obra {
    pub tipo: TipoObra,
    /// Lo que pide la obra y lo que ya se entregó (los mismos recursos, en el mismo orden).
    pub requerido: Costo,
    pub entregado: Costo,
    /// Cuándo acaba, una vez entregado todo; `None` mientras espera materiales.
    pub completa_en: Option<Instante>,
}

impl Obra {
    fn nueva(tipo: TipoObra, requerido: Costo) -> Self {
        let entregado = requerido.iter().map(|&(rec, _)| (rec, 0)).collect();
        Self { tipo, requerido, entregado, completa_en: None }
    }

    /// Lo que todavía falta entregar (vacío si ya está todo).
    pub fn falta(&self) -> Costo {
        falta_de(&self.requerido, &self.entregado)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Avanzada {
    /// Id de mundo (el mismo contador que ejércitos, plazas y pueblos).
    pub id: u64,
    pub posicion: Punto,
    /// La etapa que tiene (la atalaya mientras se levanta; la que tenía mientras se mejora o se repara).
    pub etapa: Etapa,
    pub estado: Estado,
    pub obra: Option<Obra>,
    /// Lo que guardan los héroes en ella (los carros llenan; un ejército reposta; un atacante que gana se lleva lo que le quepa). La atalaya no tiene.
    pub almacen: Almacen,
}

impl Avanzada {
    pub fn operativa(&self) -> bool {
        self.estado == Estado::Operativa
    }

    /// Las ruinas de una avanzada que no es de nadie: la madre desapareció y es una ciudadela.
    pub fn en_ruinas(&self) -> bool {
        matches!(self.estado, Estado::EnRuinas { .. })
    }
}

/// `AVANZADAS`: las cifras de las obras de mapa de las avanzadas.
#[derive(Clone, Debug)]
pub struct ReglasAvanzadas {
    /// Nivel operativo mínimo de la madre para tener cada etapa.
    pub nivel_madre: [u32; 3],
    /// Avanzadas que admite la madre por nivel (2 en el nivel 2 … 5 en el 5).
    pub cupo_por_nivel: u32,
    /// Cabida por recurso del almacén de cada etapa.
    pub capacidad_almacen: [Mil; 3],
    /// Lo que mide la avanzada en el mapa (`TAMANOS_MAPA.avanzada`): el diámetro del disco que ocupa.
    pub tamano: Mil,
    /// Lo que mide la ciudad de la plaza madre en el mapa, como fracción del radio de su zona (ppm): «la ciudad nunca pasa de una décima de su provincia».
    pub ciudad_de_la_zona: i64,
    /// Parte del coste de la etapa que cuesta reparar unas ruinas (ppm).
    pub fraccion_reparacion: i64,
    /// Coste y duración (ms) de cada paso: construir la atalaya, mejorarla a fuerte, mejorar el fuerte a ciudadela.
    obras: [(Costo, i64); 3],
}

impl ReglasAvanzadas {
    pub fn de(b: &Value, tamanos: &crate::datos::TamanosMapa) -> Self {
        let por_etapa = |bloque: &Value| -> [Value; 3] { Etapa::TODAS.map(|e| bloque[e.nombre()].clone()) };
        let numero = |v: &Value| v.as_f64().unwrap_or_else(|| panic!("falta un número en AVANZADAS: {v}"));
        let nivel = por_etapa(&b["nivelMadre"]).map(|v| numero(&v) as u32);
        let capacidad = por_etapa(&b["capacidadAlmacen"]).map(|v| mil(numero(&v)));
        let obras = Etapa::TODAS.map(|e| {
            let o = &b["obras"][e.nombre()];
            (costo(o.get("costo")), numero(&o["minutos"]) as i64 * MS_POR_MINUTO)
        });
        Self {
            nivel_madre: nivel,
            cupo_por_nivel: numero(&b["cupoPorNivelDeLaMadre"]) as u32,
            capacidad_almacen: capacidad,
            tamano: tamanos.avanzada,
            ciudad_de_la_zona: ppm(numero(&b["fraccionCiudadDeLaZona"])),
            fraccion_reparacion: ppm(numero(&b["fraccionReparacion"])),
            obras,
        }
    }

    /// Lo que cuesta el paso que lleva a esa etapa.
    pub fn costo(&self, etapa: Etapa) -> &Costo {
        &self.obras[etapa.indice()].0
    }

    /// Lo que dura ese paso (ms).
    pub fn duracion(&self, etapa: Etapa) -> i64 {
        self.obras[etapa.indice()].1
    }

    /// Lo que mide la ciudad de una plaza cuya zona tiene ese radio.
    pub fn radio_de_ciudad(&self, radio_de_zona: Mil) -> Mil {
        por(radio_de_zona, self.ciudad_de_la_zona)
    }

    /// Dónde cabe una avanzada nueva (Doc 5.17, 4.2.2): sin pisar la ciudad de su madre (la distancia a su centro es la de su radio más la mitad del tamaño de la
    /// avanzada) ni ningún otro elemento del mapa (`otros` son los centros de los demás, con su diámetro: avanzadas, pueblos, campamentos; dos se pisan si sus centros distan
    /// menos de la media de sus diámetros). Que esté en la zona y que el héroe esté en el sitio lo mira el núcleo.
    pub fn comprobar_sitio(&self, plaza: Punto, radio_de_zona: Mil, sitio: Punto, otros: impl IntoIterator<Item = (Punto, Mil)>) -> Result<(), ErrorAvanzada> {
        if sitio.dist(plaza) < self.radio_de_ciudad(radio_de_zona) + self.tamano / 2 {
            return Err(ErrorAvanzada::DentroDelAsentamiento);
        }
        if otros.into_iter().any(|(o, d)| sitio.dist(o) < (self.tamano + d) / 2) {
            return Err(ErrorAvanzada::PisaOtroElemento);
        }
        Ok(())
    }

    /// Lo que cuesta reparar unas ruinas de esa etapa: la fracción del coste de construirla.
    pub fn costo_de_reparar(&self, etapa: Etapa) -> Costo {
        self.costo(etapa).iter().map(|&(rec, n)| (rec, base::entero_arriba(por(n, self.fraccion_reparacion)))).collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorAvanzada {
    /// La madre no tiene el nivel operativo que pide la etapa.
    NivelInsuficiente,
    SinCupo,
    NoExiste,
    /// No se puede mejorar: no está operativa o ya es una ciudadela.
    NoMejorable,
    /// No se puede reparar: no está en ruinas (una atalaya no tiene ruinas: se destruye).
    NoReparable,
    /// Ya tiene una obra en marcha.
    YaHayObra,
    /// La obra no espera materiales.
    NoEstaPendiente,
    /// Lo que se ofrece no sirve a lo que falta.
    NadaQueEntregar,
    /// La ciudadela no se destruye a mano.
    NoSeDestruyeALaMano,
    /// El sitio cae sobre la ciudad de la plaza madre.
    DentroDelAsentamiento,
    /// El sitio pisa otro elemento del mapa.
    PisaOtroElemento,
    /// No hay una mejora ni una reparación en marcha que cancelar.
    NadaQueCancelar,
}

impl Asentamiento {
    pub fn avanzada(&self, id: u64) -> Option<&Avanzada> {
        self.avanzadas.iter().find(|a| a.id == id)
    }

    /// Cuántas avanzadas admite ahora la plaza (pendientes, operativas y en ruinas): una por nivel operativo; 0 por debajo del nivel que pide la
    /// atalaya.
    pub fn cupo_de_avanzadas(&self, r: &Reglas) -> u32 {
        let nivel = self.nivel_actual();
        if nivel < r.avanzadas.nivel_madre[Etapa::Atalaya.indice()] {
            return 0;
        }
        nivel * r.avanzadas.cupo_por_nivel
    }

    /// Edificios de mapa activos de la plaza: los pueblos y las avanzadas operativas cuentan como un edificio activo más al pagar el mantenimiento.
    pub(crate) fn elementos_de_mapa_activos(&self) -> usize {
        self.pueblos_activos() + self.avanzadas.iter().filter(|a| a.operativa()).count()
    }

    /// Marca la atalaya de una avanzada nueva en `posicion` (que el núcleo ya comprobó dentro de la zona y fuera del asentamiento): queda pendiente
    /// de materiales y ocupa cupo.
    pub fn iniciar_avanzada(&mut self, r: &Reglas, id: u64, posicion: Punto, ev: &mut Vec<Evento>) -> Result<(), ErrorAvanzada> {
        let av = &r.avanzadas;
        if self.nivel_actual() < av.nivel_madre[Etapa::Atalaya.indice()] {
            return Err(ErrorAvanzada::NivelInsuficiente);
        }
        if self.avanzadas.len() as u32 >= self.cupo_de_avanzadas(r) {
            return Err(ErrorAvanzada::SinCupo);
        }
        let obra = Obra::nueva(TipoObra::Construir, av.costo(Etapa::Atalaya).clone());
        let almacen = Almacen::nuevo(av.capacidad_almacen[Etapa::Atalaya.indice()]);
        self.avanzadas.push(Avanzada { id, posicion, etapa: Etapa::Atalaya, estado: Estado::EnConstruccion, obra: Some(obra), almacen });
        ev.push(Evento::AvanzadaObraMarcada { id, tipo: TipoObra::Construir, posicion });
        Ok(())
    }

    /// Una ciudadela en ruinas sin dueño pasa a esta plaza (la que tiene después su zona) para repararla: ocupa un puesto de su cupo.
    pub fn adoptar_avanzada(&mut self, r: &Reglas, avanzada: Avanzada) -> Result<(), ErrorAvanzada> {
        if self.nivel_actual() < r.avanzadas.nivel_madre[avanzada.etapa.indice()] {
            return Err(ErrorAvanzada::NivelInsuficiente);
        }
        if self.avanzadas.len() as u32 >= self.cupo_de_avanzadas(r) {
            return Err(ErrorAvanzada::SinCupo);
        }
        let sitio = self.avanzadas.partition_point(|a| a.id < avanzada.id);
        self.avanzadas.insert(sitio, avanzada);
        Ok(())
    }

    /// Marca la mejora de una avanzada operativa a la etapa siguiente: sigue funcionando mientras tanto, y la obra espera sus materiales.
    pub fn mejorar_avanzada(&mut self, r: &Reglas, id: u64, ev: &mut Vec<Evento>) -> Result<(), ErrorAvanzada> {
        let nivel = self.nivel_actual();
        let a = self.avanzadas.iter_mut().find(|a| a.id == id).ok_or(ErrorAvanzada::NoExiste)?;
        let siguiente = a.etapa.siguiente().filter(|_| a.operativa()).ok_or(ErrorAvanzada::NoMejorable)?;
        if a.obra.is_some() {
            return Err(ErrorAvanzada::YaHayObra);
        }
        if nivel < r.avanzadas.nivel_madre[siguiente.indice()] {
            return Err(ErrorAvanzada::NivelInsuficiente);
        }
        a.obra = Some(Obra::nueva(TipoObra::Mejorar, r.avanzadas.costo(siguiente).clone()));
        ev.push(Evento::AvanzadaObraMarcada { id, tipo: TipoObra::Mejorar, posicion: a.posicion });
        Ok(())
    }

    /// Marca la reparación de unas ruinas de fuerte o ciudadela: la mitad del coste de la etapa. Siguen en ruinas hasta que acabe la obra.
    pub fn reparar_avanzada(&mut self, r: &Reglas, id: u64, ev: &mut Vec<Evento>) -> Result<(), ErrorAvanzada> {
        let nivel = self.nivel_actual();
        let a = self.avanzadas.iter_mut().find(|a| a.id == id).ok_or(ErrorAvanzada::NoExiste)?;
        if !a.en_ruinas() || a.etapa == Etapa::Atalaya {
            return Err(ErrorAvanzada::NoReparable);
        }
        if a.obra.is_some() {
            return Err(ErrorAvanzada::YaHayObra);
        }
        if nivel < r.avanzadas.nivel_madre[a.etapa.indice()] {
            return Err(ErrorAvanzada::NivelInsuficiente);
        }
        a.obra = Some(Obra::nueva(TipoObra::Reparar, r.avanzadas.costo_de_reparar(a.etapa)));
        ev.push(Evento::AvanzadaObraMarcada { id, tipo: TipoObra::Reparar, posicion: a.posicion });
        Ok(())
    }

    /// Entrega materiales a la obra pendiente: toma de `disponible` lo que falta, recurso a recurso. Con todo entregado arranca la obra (la duración de
    /// la etapa a la que lleva). Devuelve lo tomado, para que quien entrega lo quite de donde lo llevaba.
    pub fn entregar_a_avanzada(&mut self, t: Instante, r: &Reglas, id: u64, disponible: &[(Recurso, Mil)], ev: &mut Vec<Evento>) -> Result<Costo, ErrorAvanzada> {
        let a = self.avanzadas.iter_mut().find(|a| a.id == id).ok_or(ErrorAvanzada::NoExiste)?;
        let etapa = a.etapa;
        let Some(obra @ Obra { completa_en: None, .. }) = &mut a.obra else { return Err(ErrorAvanzada::NoEstaPendiente) };
        let tomado = tomar_materiales(&obra.requerido, &mut obra.entregado, disponible);
        if tomado.is_empty() {
            return Err(ErrorAvanzada::NadaQueEntregar);
        }
        ev.push(Evento::AvanzadaEntrega { id, cantidad: tomado.iter().map(|&(_, n)| n).sum() });
        if obra.falta().is_empty() {
            let destino = if obra.tipo == TipoObra::Mejorar { etapa.siguiente().expect("una mejora tiene etapa siguiente") } else { etapa };
            let completa_en = t + r.avanzadas.duracion(destino);
            obra.completa_en = Some(completa_en);
            ev.push(Evento::AvanzadaObraIniciada { id, tipo: obra.tipo, completa_en });
        }
        Ok(tomado)
    }

    /// Cancela la mejora o la reparación en marcha (o pendiente de materiales) de una avanzada: sin reembolso, los materiales entregados se pierden, y la avanzada sigue como
    /// estaba (operativa en su etapa, o en ruinas). La primera obra no se cancela así: se destruye la avanzada.
    pub fn cancelar_obra_de_avanzada(&mut self, id: u64, ev: &mut Vec<Evento>) -> Result<(), ErrorAvanzada> {
        let a = self.avanzadas.iter_mut().find(|a| a.id == id).ok_or(ErrorAvanzada::NoExiste)?;
        let Some(Obra { tipo, .. }) = a.obra.as_ref().filter(|o| o.tipo != TipoObra::Construir) else { return Err(ErrorAvanzada::NadaQueCancelar) };
        ev.push(Evento::AvanzadaObraCancelada { id, tipo: *tipo });
        a.obra = None;
        Ok(())
    }

    /// Destruye la avanzada a mano (o cancela su primera obra): libera el cupo y no recupera nada. La ciudadela no se destruye a mano. Devuelve la
    /// avanzada para que el núcleo libere lo suyo (guarnición, rejilla).
    pub fn destruir_avanzada(&mut self, id: u64, ev: &mut Vec<Evento>) -> Result<Avanzada, ErrorAvanzada> {
        let i = self.avanzadas.iter().position(|a| a.id == id).ok_or(ErrorAvanzada::NoExiste)?;
        if self.avanzadas[i].etapa == Etapa::Ciudadela {
            return Err(ErrorAvanzada::NoSeDestruyeALaMano);
        }
        ev.push(Evento::AvanzadaDestruida { id });
        Ok(self.avanzadas.remove(i))
    }

    /// Las obras de avanzada cuyo instante llegó acaban: la primera deja la atalaya operativa, la mejora sube de etapa (y de almacén) y la reparación
    /// saca la avanzada de sus ruinas. Idempotente: una cita que llega con la avanzada ya cambiada no encuentra nada.
    pub(crate) fn cumplir_avanzadas(&mut self, t: Instante, r: &Reglas, ev: &mut Vec<Evento>) {
        for a in &mut self.avanzadas {
            let Some(Obra { tipo, completa_en: Some(fin), .. }) = &a.obra else { continue };
            if t < *fin {
                continue;
            }
            let tipo = *tipo;
            match tipo {
                TipoObra::Construir | TipoObra::Reparar => a.estado = Estado::Operativa,
                TipoObra::Mejorar => {
                    let antes = r.avanzadas.capacidad_almacen[a.etapa.indice()];
                    a.etapa = a.etapa.siguiente().expect("una mejora tiene etapa siguiente");
                    a.almacen.ampliar_todos(r.avanzadas.capacidad_almacen[a.etapa.indice()] - antes);
                }
            }
            a.obra = None;
            ev.push(Evento::AvanzadaCompletada { id: a.id, tipo, etapa: a.etapa });
        }
    }

    /// Cuándo acaban las obras de avanzada en marcha.
    pub(crate) fn fines_de_avanzadas(&self) -> impl Iterator<Item = Instante> + '_ {
        self.avanzadas.iter().filter_map(|a| a.obra.as_ref()?.completa_en)
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::recurso::Recurso::*;

    const MIN: i64 = MS_POR_MINUTO;

    fn madre(r: &Reglas, nivel: u32) -> Asentamiento {
        let mut a = Asentamiento::fundar(r, 0, &[], 3);
        a.nivel = nivel;
        a.medidor.nivel_actual = nivel;
        a
    }

    fn marcar(a: &mut Asentamiento, r: &Reglas, id: u64) -> Result<(), ErrorAvanzada> {
        a.iniciar_avanzada(r, id, Punto::new(id as i64 * 1_000, 0), &mut vec![])
    }

    /// Entrega todo lo que pide la obra de la avanzada `id` en `t`.
    fn entregar_todo(a: &mut Asentamiento, r: &Reglas, t: Instante, id: u64) -> Costo {
        let falta = a.avanzada(id).unwrap().obra.as_ref().unwrap().falta();
        a.entregar_a_avanzada(t, r, id, &falta, &mut vec![]).unwrap()
    }

    #[test]
    fn las_cifras_salen_del_documento() {
        let r = Reglas::cargar().avanzadas;
        assert_eq!((r.nivel_madre, r.cupo_por_nivel, r.capacidad_almacen), ([2, 2, 3], 1, [0, 2_000_000, 10_000_000]));
        assert_eq!((r.tamano, r.ciudad_de_la_zona, r.fraccion_reparacion), (200, 100_000, 500_000), "elemento de 0,2; ciudad de una décima de la zona; reparar cuesta la mitad");
        assert_eq!(r.costo(Etapa::Atalaya), &vec![(Madera, 200_000), (Piedra, 100_000)], "200 madera + 100 piedra, obra de 2 h");
        assert_eq!(r.duracion(Etapa::Atalaya), 120 * MIN);
        assert_eq!(r.costo(Etapa::Fuerte), &vec![(Madera, 600_000), (Oro, 50_000), (Piedra, 400_000)]);
        assert_eq!(r.duracion(Etapa::Fuerte), 12 * 60 * MIN);
        assert_eq!(r.costo(Etapa::Ciudadela), &vec![(Madera, 1_500_000), (Oro, 300_000), (Piedra, 2_000_000)]);
        assert_eq!(r.duracion(Etapa::Ciudadela), 2 * 24 * 60 * MIN);
        assert_eq!(r.costo_de_reparar(Etapa::Fuerte), vec![(Madera, 300_000), (Oro, 25_000), (Piedra, 200_000)], "reparar cuesta el 50 % de construir la etapa");
    }

    #[test]
    fn exige_nivel_2_y_el_cupo_es_uno_por_nivel_contando_pendientes() {
        let r = Reglas::cargar();
        let mut uno = madre(&r, 1);
        assert_eq!((marcar(&mut uno, &r, 1), uno.cupo_de_avanzadas(&r)), (Err(ErrorAvanzada::NivelInsuficiente), 0));
        let mut a = madre(&r, 2);
        assert_eq!(a.cupo_de_avanzadas(&r), 2);
        assert_eq!((marcar(&mut a, &r, 1), marcar(&mut a, &r, 2)), (Ok(()), Ok(())));
        assert_eq!(marcar(&mut a, &r, 3), Err(ErrorAvanzada::SinCupo), "2 avanzadas en el nivel 2, contando las pendientes");
        for (nivel, cupo) in [(3, 3), (4, 4), (5, 5)] {
            (a.nivel, a.medidor.nivel_actual) = (nivel, nivel);
            assert_eq!(a.cupo_de_avanzadas(&r), cupo);
        }
    }

    #[test]
    fn la_obra_arranca_con_todo_entregado_y_acaba_en_su_instante_exacto() {
        let r = Reglas::cargar();
        let mut a = madre(&r, 2);
        marcar(&mut a, &r, 1).unwrap();
        let av = a.avanzada(1).unwrap();
        assert_eq!((av.etapa, av.estado), (Etapa::Atalaya, Estado::EnConstruccion));
        assert_eq!(av.obra.as_ref().unwrap().falta(), vec![(Madera, 200_000), (Piedra, 100_000)]);
        let mut ev = vec![];
        // Parcial: toma lo que sirve y no arranca nada; con lo que no sirve no hay entrega.
        let t = a.entregar_a_avanzada(1_000, &r, 1, &[(Madera, 150_000), (Oro, 9)], &mut ev).unwrap();
        assert_eq!(t, vec![(Madera, 150_000)]);
        assert_eq!(a.entregar_a_avanzada(1_000, &r, 1, &[(Oro, 5)], &mut ev), Err(ErrorAvanzada::NadaQueEntregar));
        let t = a.entregar_a_avanzada(5_000, &r, 1, &[(Madera, 999_000), (Piedra, 100_000)], &mut ev).unwrap();
        assert_eq!(t, vec![(Madera, 50_000), (Piedra, 100_000)]);
        let fin = 5_000 + 120 * MIN;
        assert!(ev.contains(&Evento::AvanzadaObraIniciada { id: 1, tipo: TipoObra::Construir, completa_en: fin }));
        assert_eq!(a.entregar_a_avanzada(6_000, &r, 1, &[(Madera, 1)], &mut ev), Err(ErrorAvanzada::NoEstaPendiente), "ya no espera materiales");
        // Ni un instante antes; en el exacto, operativa.
        a.cumplir_avanzadas(fin - 1, &r, &mut ev);
        assert_eq!(a.avanzada(1).unwrap().estado, Estado::EnConstruccion);
        a.cumplir_avanzadas(fin, &r, &mut ev);
        let av = a.avanzada(1).unwrap();
        assert_eq!((av.estado, av.obra.is_none(), av.almacen.capacidad(Trigo)), (Estado::Operativa, true, 0));
        assert_eq!(ev.last(), Some(&Evento::AvanzadaCompletada { id: 1, tipo: TipoObra::Construir, etapa: Etapa::Atalaya }));
        a.cumplir_avanzadas(fin + MIN, &r, &mut ev);
        assert_eq!(ev.iter().filter(|e| matches!(e, Evento::AvanzadaCompletada { .. })).count(), 1, "idempotente");
    }

    #[test]
    fn mejorar_sube_de_etapa_con_su_almacen_y_pide_el_nivel_de_la_madre_mientras_sigue_funcionando() {
        let r = Reglas::cargar();
        let mut a = madre(&r, 2);
        marcar(&mut a, &r, 1).unwrap();
        assert_eq!(a.mejorar_avanzada(&r, 1, &mut vec![]), Err(ErrorAvanzada::NoMejorable), "en obra no se mejora");
        entregar_todo(&mut a, &r, 0, 1);
        a.cumplir_avanzadas(120 * MIN, &r, &mut vec![]);
        assert_eq!(a.mejorar_avanzada(&r, 9, &mut vec![]), Err(ErrorAvanzada::NoExiste));
        a.mejorar_avanzada(&r, 1, &mut vec![]).unwrap();
        assert!(a.avanzada(1).unwrap().operativa(), "mientras se mejora sigue funcionando");
        assert_eq!(a.mejorar_avanzada(&r, 1, &mut vec![]), Err(ErrorAvanzada::YaHayObra));
        let t = 200 * MIN;
        entregar_todo(&mut a, &r, t, 1);
        a.cumplir_avanzadas(t + 12 * 60 * MIN, &r, &mut vec![]);
        let av = a.avanzada(1).unwrap();
        assert_eq!((av.etapa, av.operativa(), av.almacen.capacidad(Trigo)), (Etapa::Fuerte, true, 2_000_000));
        // La ciudadela pide el nivel 3 de la madre.
        assert_eq!(a.mejorar_avanzada(&r, 1, &mut vec![]), Err(ErrorAvanzada::NivelInsuficiente));
        (a.nivel, a.medidor.nivel_actual) = (3, 3);
        a.mejorar_avanzada(&r, 1, &mut vec![]).unwrap();
        entregar_todo(&mut a, &r, 2_000 * MIN, 1);
        a.cumplir_avanzadas(2_000 * MIN + 2 * 24 * 60 * MIN, &r, &mut vec![]);
        assert_eq!((a.avanzada(1).unwrap().etapa, a.avanzada(1).unwrap().almacen.capacidad(Madera)), (Etapa::Ciudadela, 10_000_000));
        assert_eq!(a.mejorar_avanzada(&r, 1, &mut vec![]), Err(ErrorAvanzada::NoMejorable), "la ciudadela no tiene siguiente");
    }

    #[test]
    fn reparar_cuesta_la_mitad_deja_la_avanzada_en_ruinas_hasta_que_acaba_y_no_vale_para_una_atalaya() {
        let r = Reglas::cargar();
        let mut a = madre(&r, 3);
        marcar(&mut a, &r, 1).unwrap();
        assert_eq!(a.reparar_avanzada(&r, 1, &mut vec![]), Err(ErrorAvanzada::NoReparable), "operativa no se repara");
        entregar_todo(&mut a, &r, 0, 1);
        a.cumplir_avanzadas(120 * MIN, &r, &mut vec![]);
        a.avanzadas[0].etapa = Etapa::Fuerte;
        a.avanzadas[0].estado = Estado::EnRuinas { invulnerable_hasta: None };
        a.reparar_avanzada(&r, 1, &mut vec![]).unwrap();
        let obra = a.avanzada(1).unwrap().obra.clone().unwrap();
        assert_eq!((obra.tipo, obra.requerido), (TipoObra::Reparar, vec![(Madera, 300_000), (Oro, 25_000), (Piedra, 200_000)]));
        assert!(a.avanzada(1).unwrap().en_ruinas(), "sigue en ruinas mientras se repara");
        let t = 500 * MIN;
        entregar_todo(&mut a, &r, t, 1);
        assert_eq!(a.fines_de_avanzadas().collect::<Vec<_>>(), vec![t + 12 * 60 * MIN], "la reparación dura lo que la etapa");
        a.cumplir_avanzadas(t + 12 * 60 * MIN, &r, &mut vec![]);
        assert!(a.avanzada(1).unwrap().operativa());
        let mut atalaya = a.avanzadas[0].clone();
        atalaya.etapa = Etapa::Atalaya;
        atalaya.estado = Estado::EnRuinas { invulnerable_hasta: None };
        a.avanzadas[0] = atalaya;
        assert_eq!(a.reparar_avanzada(&r, 1, &mut vec![]), Err(ErrorAvanzada::NoReparable));
    }

    #[test]
    fn destruir_a_mano_libera_el_cupo_sin_recuperar_nada_y_la_ciudadela_no_se_destruye() {
        let r = Reglas::cargar();
        let mut a = madre(&r, 2);
        marcar(&mut a, &r, 1).unwrap();
        marcar(&mut a, &r, 2).unwrap();
        a.entregar_a_avanzada(0, &r, 1, &[(Madera, 50_000)], &mut vec![]).unwrap();
        let almacen = a.almacen.clone();
        assert!(a.destruir_avanzada(1, &mut vec![]).is_ok() && a.destruir_avanzada(1, &mut vec![]) == Err(ErrorAvanzada::NoExiste));
        assert_eq!(a.almacen, almacen, "lo entregado no vuelve");
        assert_eq!(marcar(&mut a, &r, 3), Ok(()), "el cupo quedó libre");
        a.avanzadas[0].etapa = Etapa::Ciudadela;
        assert_eq!(a.destruir_avanzada(2, &mut vec![]), Err(ErrorAvanzada::NoSeDestruyeALaMano));
    }

    #[test]
    fn el_sitio_no_pisa_la_ciudad_de_la_madre_ni_otro_elemento_del_mapa() {
        let r = Reglas::cargar().avanzadas;
        let plaza = Punto::new(100_000, 100_000);
        let a = |dx: i64, dy: i64| Punto::new(plaza.x + dx, plaza.y + dy);
        // La ciudad mide una décima de la zona (zona 15 → 1,5) y la avanzada pide además la mitad de su tamaño (0,1): 1,6 de centro a centro.
        assert_eq!((r.radio_de_ciudad(6_000), r.radio_de_ciudad(15_000)), (600, 1_500));
        assert_eq!(r.comprobar_sitio(plaza, 15_000, a(1_599, 0), []), Err(ErrorAvanzada::DentroDelAsentamiento));
        assert_eq!(r.comprobar_sitio(plaza, 15_000, a(0, -1_599), []), Err(ErrorAvanzada::DentroDelAsentamiento));
        assert_eq!(r.comprobar_sitio(plaza, 15_000, a(1_600, 0), []), Ok(()), "justo fuera, sin pisarla");
        // Con una zona mayor la ciudad es mayor: lo que valía deja de valer.
        assert_eq!(r.comprobar_sitio(plaza, 30_000, a(2_000, 0), []), Err(ErrorAvanzada::DentroDelAsentamiento));
        assert_eq!(r.comprobar_sitio(plaza, 30_000, a(3_100, 0), []), Ok(()));
        // Otro elemento (una avanzada, un pueblo, un campamento) a menos de la media de los dos diámetros (0,2 y 0,2) se pisa; a 0,2 justo, no.
        let sitio = a(8_000, 0);
        let otros = [(a(8_199, 0), 200), (a(20_000, 0), 200)];
        assert_eq!(r.comprobar_sitio(plaza, 15_000, sitio, otros), Err(ErrorAvanzada::PisaOtroElemento));
        assert_eq!(r.comprobar_sitio(plaza, 15_000, sitio, [(a(8_000, 200), 200), (a(7_800, 0), 200)]), Ok(()));
        assert_eq!(r.comprobar_sitio(plaza, 15_000, sitio, [(sitio, 200)]), Err(ErrorAvanzada::PisaOtroElemento), "ni encima");
        // Uno más grande pisa desde más lejos: con 1 de diámetro, la media es 0,6.
        assert_eq!(r.comprobar_sitio(plaza, 15_000, sitio, [(a(8_599, 0), 1_000)]), Err(ErrorAvanzada::PisaOtroElemento));
        assert_eq!(r.comprobar_sitio(plaza, 15_000, sitio, [(a(8_600, 0), 1_000)]), Ok(()));
        // La ciudad se mira antes que los demás elementos.
        assert_eq!(r.comprobar_sitio(plaza, 15_000, a(500, 0), [(a(500, 0), 200)]), Err(ErrorAvanzada::DentroDelAsentamiento));
    }

    #[test]
    fn cancelar_una_mejora_o_una_reparacion_pierde_lo_entregado_y_deja_la_avanzada_como_estaba() {
        let r = Reglas::cargar();
        let mut a = madre(&r, 3);
        marcar(&mut a, &r, 1).unwrap();
        let mut ev = vec![];
        // Sin obra o con la primera obra no hay nada que cancelar (la primera se destruye).
        assert_eq!(a.cancelar_obra_de_avanzada(9, &mut ev), Err(ErrorAvanzada::NoExiste));
        assert_eq!(a.cancelar_obra_de_avanzada(1, &mut ev), Err(ErrorAvanzada::NadaQueCancelar), "la primera obra se cancela destruyendo la avanzada");
        entregar_todo(&mut a, &r, 0, 1);
        a.cumplir_avanzadas(120 * MIN, &r, &mut vec![]);
        assert_eq!(a.cancelar_obra_de_avanzada(1, &mut ev), Err(ErrorAvanzada::NadaQueCancelar), "operativa y sin obra");
        assert!(ev.is_empty());
        // Una mejora a medio entregar: se cancela, el almacén de la plaza no recibe nada y la avanzada sigue operativa en su etapa.
        a.mejorar_avanzada(&r, 1, &mut vec![]).unwrap();
        a.entregar_a_avanzada(130 * MIN, &r, 1, &[(Madera, 100_000)], &mut vec![]).unwrap();
        let almacen = a.almacen.clone();
        a.cancelar_obra_de_avanzada(1, &mut ev).unwrap();
        assert_eq!(ev, vec![Evento::AvanzadaObraCancelada { id: 1, tipo: TipoObra::Mejorar }]);
        let av = a.avanzada(1).unwrap();
        assert_eq!((av.obra.is_none(), av.operativa(), av.etapa, &a.almacen), (true, true, Etapa::Atalaya, &almacen), "no hay reembolso");
        assert_eq!(a.cancelar_obra_de_avanzada(1, &mut ev), Err(ErrorAvanzada::NadaQueCancelar), "ya no queda nada");
        // Una mejora con todo entregado y en marcha también se cancela, y su fin deja de contar.
        a.mejorar_avanzada(&r, 1, &mut vec![]).unwrap();
        entregar_todo(&mut a, &r, 200 * MIN, 1);
        assert_eq!(a.fines_de_avanzadas().count(), 1);
        a.cancelar_obra_de_avanzada(1, &mut ev).unwrap();
        assert_eq!(a.fines_de_avanzadas().count(), 0);
        a.cumplir_avanzadas(10_000 * MIN, &r, &mut vec![]);
        assert_eq!(a.avanzada(1).unwrap().etapa, Etapa::Atalaya, "la mejora cancelada no acaba");
        // Una reparación: las ruinas siguen siéndolo.
        a.avanzadas[0].etapa = Etapa::Fuerte;
        a.avanzadas[0].estado = Estado::EnRuinas { invulnerable_hasta: None };
        a.reparar_avanzada(&r, 1, &mut vec![]).unwrap();
        a.cancelar_obra_de_avanzada(1, &mut ev).unwrap();
        assert_eq!(ev.last(), Some(&Evento::AvanzadaObraCancelada { id: 1, tipo: TipoObra::Reparar }));
        assert!(a.avanzada(1).unwrap().en_ruinas() && a.avanzada(1).unwrap().obra.is_none());
    }

    #[test]
    fn solo_las_operativas_cuentan_como_edificio_activo_de_la_madre() {
        let r = Reglas::cargar();
        let mut a = madre(&r, 2);
        marcar(&mut a, &r, 1).unwrap();
        marcar(&mut a, &r, 2).unwrap();
        assert_eq!(a.elementos_de_mapa_activos(), 0, "pendientes: nada");
        a.avanzadas[0].estado = Estado::Operativa;
        a.avanzadas[1].estado = Estado::EnRuinas { invulnerable_hasta: None };
        assert_eq!(a.elementos_de_mapa_activos(), 1, "una en ruinas no funciona: no cuenta");
        let sin = madre(&r, 2).coste_mantenimiento(&r, 0);
        assert_ne!(a.coste_mantenimiento(&r, 0), sin, "la operativa sube el mantenimiento");
    }

    #[test]
    fn una_ciudadela_huerfana_se_adopta_si_cabe_en_el_cupo_y_el_nivel() {
        let r = Reglas::cargar();
        let huerfana = Avanzada { id: 5, posicion: Punto::new(1, 1), etapa: Etapa::Ciudadela, estado: Estado::EnRuinas { invulnerable_hasta: None }, obra: None, almacen: Almacen::nuevo(10_000_000) };
        let mut a = madre(&r, 2);
        assert_eq!(a.adoptar_avanzada(&r, huerfana.clone()), Err(ErrorAvanzada::NivelInsuficiente), "la ciudadela pide el nivel 3");
        let mut a = madre(&r, 3);
        marcar(&mut a, &r, 9).unwrap();
        a.adoptar_avanzada(&r, huerfana.clone()).unwrap();
        assert_eq!(a.avanzadas.iter().map(|x| x.id).collect::<Vec<_>>(), vec![5, 9], "por id ascendente");
        let mut llena = madre(&r, 3);
        (1..=3).for_each(|i| marcar(&mut llena, &r, i).unwrap());
        assert_eq!(llena.adoptar_avanzada(&r, huerfana), Err(ErrorAvanzada::SinCupo));
    }
}
