//! Las cifras del combate. Fuente única: los bloques `<!-- parametros:NOMBRE -->` de `Docs/Mechanics/{Combate, Batalla_Unity_Ciclo,
//! Movimiento_Ejercitos, Murallas, Generacion_Del_Mundo}.md`, incrustados en el binario. Como en `militar`, **los bloques son lo vigente**
//! (decisión del autor, 2026-10-19): ninguna cifra se corrige en el código. Decimales → **ppm** (tasas y factores) y **milésimas** (`Mil`).

use asentamiento::datos::{bloque_de, mil, ppm, texto_de_parametros_de, UNO};
use asentamiento::Recurso;
use base::{Mil, MS_POR_MINUTO};
use militar::datos::{bloque, entero, num};
use asentamiento::Costo;
use militar::Tropa;
use serde_json::Value;

pub const COMBATE: &str = militar::datos::COMBATE;
pub const BATALLA_UNITY: &str = include_str!("../../../Docs/Mechanics/Batalla_Unity_Ciclo.md");
const MURALLAS: &str = include_str!("../../../Docs/Mechanics/Murallas.md");
const MUNDO: &str = include_str!("../../../Docs/Mechanics/Generacion_Del_Mundo.md");

/// Los bloques de parámetros que lee este crate, para la identidad de las reglas (hash del núcleo). De Murallas, Movimiento y el mundo solo
/// entran los bloques que se usan (el resto del documento no es de este crate).
pub fn texto_de_parametros() -> String {
    let suelto = |doc: &str, nombre: &str| format!("<!-- parametros:{nombre} -->\n{}\n<!-- /parametros -->\n", bloque(doc, nombre));
    texto_de_parametros_de(COMBATE)
        + &texto_de_parametros_de(BATALLA_UNITY)
        + &suelto(MURALLAS, "OCUPACION")
        + &suelto(militar::datos::MOVIMIENTO, "MOVIMIENTO")
        + &suelto(MUNDO, "CAMPAMENTOS_BANDIDOS")
}

/// Poder, varianza y ventajas (`MILITAR`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglasPoder {
    /// Amplitud de la tirada de cada bando (ppm): ±15 %.
    pub varianza: i64,
    /// Ventaja de todo defensor (ppm).
    pub ventaja_defensor: i64,
    /// Bonus del defensor por cada escuadra de más (ppm).
    pub cohesion: i64,
    /// Lo que suma un héroe sano que combate (milésimas de poder).
    pub poder_por_heroe: Mil,
    /// Defensa de una caravana sin escolta ni ejército (milésimas de poder).
    pub defensa_base_caravana: Mil,
    /// Experiencia de escuadra por combate ganado y perdido (milésimas de punto).
    pub xp_victoria: Mil,
    pub xp_derrota: Mil,
}

/// Fracciones de bajas y demás cifras que antes vivían en el código (`COMBATE_NUMERICO`), todas en ppm.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglasNumericas {
    pub bajas_ganador_base: i64,
    pub bajas_ganador_por_ratio: i64,
    pub bajas_perdedor_base: i64,
    pub bajas_perdedor_por_ratio: i64,
    /// Combate contra una cifra (caravana, campamento): bajas del que gana y del que pierde.
    pub bajas_fijas_ganador: i64,
    pub bajas_fijas_perdedor: i64,
    pub ventaja_aguante: i64,
    pub carga_perdida_aguante: i64,
    pub carros_perdidos: i64,
    pub protegida_pueblo: i64,
}

/// Lo que ocurre con una plaza conquistada (`OCUPACION`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglasOcupacion {
    /// Cuánto es inatacable tras la conquista y cuánto dura la ocupación (ms).
    pub proteccion: i64,
    pub duracion: i64,
    pub saqueo_poblacion: i64,
    pub edificios_danados: i64,
    pub coste_reconstruccion: i64,
    pub factor_recaudacion: i64,
    pub factor_crecimiento: i64,
}

/// El ciclo de una batalla de Unity (`BATALLA`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglasBatalla {
    pub capacidad_asedio: usize,
    pub capacidad_resto: usize,
    /// Duración máxima de la partida (ms).
    pub duracion_asedio: i64,
    pub duracion_resto: i64,
    pub convocatoria: i64,
    pub plazo_asignacion: i64,
    pub plazo_inicio: i64,
    pub margen: i64,
    pub conservar_cerradas: i64,
    /// La defensa de una caravana sin escolta cuando juega Unity: tropa y soldados.
    pub carreteros: (Tropa, i64),
    pub casillas_inventario: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NivelBandidos {
    pub poder: Mil,
    pub unidades: i64,
    pub oro_por_heroe: Mil,
    /// Probabilidad de que un campamento nuevo sea de este nivel (ppm).
    pub peso: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglasBandidos {
    /// Niveles 1, 2 y 3 (índice 0, 1, 2).
    pub niveles: [NivelBandidos; 3],
    /// Rendimientos decrecientes del botín por héroe.
    pub ventana: i64,
    pub completas: i64,
    pub caida_por_cada: i64,
    pub solo_experiencia_desde: i64,
    /// Anillo de campamentos de mercenarios donde aparecen, y cuántos según la demanda.
    pub anillo_min: Mil,
    pub anillo_max: Mil,
    pub residentes_por_bandido: i64,
    pub anillo_minimo: i64,
    pub anillo_maximo: i64,
    pub reaparicion_anillo: i64,
    /// A cuánto de su campamento acosan a una caravana.
    pub radio_ataque_caravana: Mil,
    /// Tras destruir el campamento de una plaza, cuánto tarda en volver (ms).
    pub respawn: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglasPueblos {
    /// Mineros de la guarnición de serie y lo que tarda en reponerse gratis (ms).
    pub mineros: i64,
    pub reposicion: i64,
}

/// El metal del equipo de cada tropa convertido en chatarra por soldado muerto (Doc 4.2.1): para cada tropa, `(recurso de chatarra,
/// numerador)` con `chatarra = numerador × soldados / (1000 × UNO)` milésimas, para no redondear soldado a soldado.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chatarra {
    por_tropa: Vec<Vec<(Recurso, i128)>>,
}

impl Chatarra {
    fn de(r: &militar::Reglas, fraccion: i64, recetas: &asentamiento::Catalogo) -> Self {
        let chatarra_de = |lingote: Recurso| match lingote {
            Recurso::LingoteCobre => Some(Recurso::ChatarraCobre),
            Recurso::LingoteBronce => Some(Recurso::ChatarraBronce),
            Recurso::LingoteHierro => Some(Recurso::ChatarraHierro),
            _ => None,
        };
        // Los lingotes que gasta fabricar cada pieza: lo que piden sus recetas en Armería y Carpintería.
        let lingotes = |equipo: Recurso| -> Costo {
            use asentamiento::Tipo::{Armeria, Carpinteria};
            [Armeria, Carpinteria]
                .iter()
                .flat_map(|&t| recetas.tipo(t).niveles.iter().flat_map(|n| n.recetas.iter()))
                .filter(|re| re.produce == equipo)
                .last()
                .map(|re| re.consume_por_unidad.clone())
                .unwrap_or_default()
        };
        let por_tropa = r
            .tropas
            .iter()
            .map(|def| {
                let mut v: Vec<(Recurso, i128)> = vec![];
                for &(equipo, piezas) in &def.costo_equipo {
                    for (lingote, por_pieza) in lingotes(equipo) {
                        let Some(chatarra) = chatarra_de(lingote) else { continue };
                        let n = piezas as i128 * por_pieza as i128 * fraccion as i128;
                        match v.iter_mut().find(|(rec, _)| *rec == chatarra) {
                            Some((_, x)) => *x += n,
                            None => v.push((chatarra, n)),
                        }
                    }
                }
                v
            })
            .collect();
        Self { por_tropa }
    }

    /// La chatarra de `muertos` soldados de una tropa.
    pub fn de_muertos(&self, tropa: Tropa, muertos: i64) -> Vec<(Recurso, Mil)> {
        if muertos <= 0 {
            return vec![];
        }
        self.por_tropa[tropa as usize]
            .iter()
            .map(|&(rec, n)| (rec, (n * muertos as i128 / (1000 * UNO as i128)) as Mil))
            .filter(|&(_, n)| n > 0)
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reglas {
    pub militar: militar::Reglas,
    pub poder: ReglasPoder,
    pub numericas: ReglasNumericas,
    pub chatarra: Chatarra,
    /// Fracción del carro del derrotado que se pierde (`MOVIMIENTO.fraccionRobada`, ppm).
    pub fraccion_robada: i64,
    pub ocupacion: ReglasOcupacion,
    pub batalla: ReglasBatalla,
    pub bandidos: ReglasBandidos,
    pub pueblos: ReglasPueblos,
    /// Cuánto es invulnerable un fuerte en ruinas (ms).
    pub invulnerabilidad_avanzada: i64,
}

fn nivel_de_bandidos(b: &Value, nivel: &str) -> NivelBandidos {
    let n = &b["niveles"][nivel];
    NivelBandidos { poder: mil(num(n, "poder")), unidades: entero(n, "unidades"), oro_por_heroe: mil(num(n, "oroPorHeroe")), peso: ppm(num(n, "peso")) }
}

impl Reglas {
    pub fn cargar() -> Self {
        let militar = militar::Reglas::cargar();
        let (mi, nu, av) = (bloque(COMBATE, "MILITAR"), bloque(COMBATE, "COMBATE_NUMERICO"), bloque(COMBATE, "AVANZADA"));
        let (ch, mv, oc) = (bloque(COMBATE, "CHATARRA"), bloque(militar::datos::MOVIMIENTO, "MOVIMIENTO"), bloque(MURALLAS, "OCUPACION"));
        let (ba, bd) = (bloque(BATALLA_UNITY, "BATALLA"), bloque_de(MUNDO, "CAMPAMENTOS_BANDIDOS").unwrap_or_else(|e| panic!("{e}")));
        let economia = asentamiento::Reglas::cargar();
        let pueblos = asentamiento::pueblos::ReglasPueblos::de(&asentamiento::datos::bloque("PUEBLOS_EXTRACTORES"));
        let minutos = |v: &Value, k: &str| entero(v, k) * MS_POR_MINUTO;
        let carreteros = (Tropa::desde_nombre(ba["tropaCarreteros"]["tropaId"].as_str().expect("tropaCarreteros.tropaId")).expect("tropa de carreteros"), entero(&ba, "tropaCarreteros.unidades"));
        Reglas {
            poder: ReglasPoder {
                varianza: ppm(num(&mi, "varianzaCombate")),
                ventaja_defensor: ppm(num(&mi, "ventajaDefensor")),
                cohesion: ppm(num(&mi, "bonusCohesionPorEscuadronExtra")),
                poder_por_heroe: mil(num(&mi, "poderPorHeroe")),
                defensa_base_caravana: mil(num(&mi, "defensaBaseCaravana")),
                xp_victoria: mil(num(&mi, "experienciaGanadaPorVictoria")),
                xp_derrota: mil(num(&mi, "experienciaGanadaPorDerrota")),
            },
            numericas: ReglasNumericas {
                bajas_ganador_base: ppm(num(&nu, "bajasGanadorBase")),
                bajas_ganador_por_ratio: ppm(num(&nu, "bajasGanadorPorRatio")),
                bajas_perdedor_base: ppm(num(&nu, "bajasPerdedorBase")),
                bajas_perdedor_por_ratio: ppm(num(&nu, "bajasPerdedorPorRatio")),
                bajas_fijas_ganador: ppm(num(&nu, "bajasFijasGanador")),
                bajas_fijas_perdedor: ppm(num(&nu, "bajasFijasPerdedor")),
                ventaja_aguante: ppm(num(&nu, "ventajaAguante")),
                carga_perdida_aguante: ppm(num(&nu, "fraccionCargaPerdidaAguante")),
                carros_perdidos: ppm(num(&nu, "fraccionCarrosPerdidos")),
                protegida_pueblo: ppm(num(&nu, "fraccionProtegidaPueblo")),
            },
            chatarra: Chatarra::de(&militar, ppm(num(&ch, "fraccion")), &economia.catalogo),
            fraccion_robada: ppm(num(&mv, "fraccionRobada")),
            ocupacion: ReglasOcupacion {
                proteccion: minutos(&oc, "proteccionMinutos"),
                duracion: minutos(&oc, "duracionMinutos"),
                saqueo_poblacion: ppm(num(&oc, "fraccionSaqueoPoblacion")),
                edificios_danados: ppm(num(&oc, "fraccionEdificiosDanados")),
                coste_reconstruccion: ppm(num(&oc, "fraccionCosteReconstruccion")),
                factor_recaudacion: ppm(num(&oc, "factorRecaudacion")),
                factor_crecimiento: ppm(num(&oc, "factorCrecimiento")),
            },
            batalla: ReglasBatalla {
                capacidad_asedio: entero(&ba, "capacidad.asedio") as usize,
                capacidad_resto: entero(&ba, "capacidad.resto") as usize,
                duracion_asedio: minutos(&ba, "duracionMinutos.asedio"),
                duracion_resto: minutos(&ba, "duracionMinutos.resto"),
                convocatoria: entero(&ba, "convocatoriaSegundos") * 1000,
                plazo_asignacion: minutos(&ba, "plazoAsignacionMinutos"),
                plazo_inicio: minutos(&ba, "plazoInicioMinutos"),
                margen: minutos(&ba, "margenMinutos"),
                conservar_cerradas: minutos(&ba, "conservarCerradasMinutos"),
                carreteros,
                casillas_inventario: entero(&ba, "casillasInventario") as u32,
            },
            bandidos: ReglasBandidos {
                niveles: [nivel_de_bandidos(&bd, "1"), nivel_de_bandidos(&bd, "2"), nivel_de_bandidos(&bd, "3")],
                ventana: entero(&bd, "rendimientos.ventanaHoras") * 60 * MS_POR_MINUTO,
                completas: entero(&bd, "rendimientos.completas"),
                caida_por_cada: ppm(num(&bd, "rendimientos.caidaPorCada")),
                solo_experiencia_desde: entero(&bd, "rendimientos.soloExperienciaDesde"),
                anillo_min: mil(num(&bd, "anillo.radioMin")),
                anillo_max: mil(num(&bd, "anillo.radioMax")),
                residentes_por_bandido: entero(&bd, "anillo.residentesPorBandido"),
                anillo_minimo: entero(&bd, "anillo.minimo"),
                anillo_maximo: entero(&bd, "anillo.maximo"),
                reaparicion_anillo: minutos(&bd, "anillo.reaparicionMinutos"),
                radio_ataque_caravana: mil(num(&bd, "radioAtaqueCaravana")),
                respawn: minutos(&bd, "respawnMinutos"),
            },
            pueblos: ReglasPueblos { mineros: pueblos.mineros as i64, reposicion: pueblos.reposicion_mineros_min * MS_POR_MINUTO },
            invulnerabilidad_avanzada: minutos(&av, "invulnerabilidadMinutos"),
            militar,
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn r() -> Reglas {
        Reglas::cargar()
    }

    #[test]
    fn las_cifras_vigentes_son_las_de_los_bloques_sin_correcciones() {
        let r = r();
        assert_eq!((r.poder.varianza, r.poder.ventaja_defensor, r.poder.cohesion), (150_000, 1_050_000, 100_000));
        assert_eq!((r.poder.poder_por_heroe, r.poder.defensa_base_caravana, r.poder.xp_victoria, r.poder.xp_derrota), (15_000, 26_000, 1_000, 500));
        let n = &r.numericas;
        assert_eq!((n.bajas_ganador_base, n.bajas_ganador_por_ratio, n.bajas_perdedor_base, n.bajas_perdedor_por_ratio), (50_000, 150_000, 300_000, 400_000));
        assert_eq!((n.bajas_fijas_ganador, n.bajas_fijas_perdedor), (50_000, 250_000));
        assert_eq!((n.ventaja_aguante, n.carga_perdida_aguante, n.carros_perdidos, n.protegida_pueblo), (1_250_000, 700_000, 500_000, 500_000));
        assert_eq!(r.fraccion_robada, 500_000);
        assert_eq!(r.invulnerabilidad_avanzada, 1440 * MS_POR_MINUTO);
    }

    #[test]
    fn la_ocupacion_y_el_ciclo_de_la_batalla() {
        let r = r();
        assert_eq!((r.ocupacion.proteccion, r.ocupacion.duracion), (1440 * MS_POR_MINUTO, 90 * MS_POR_MINUTO));
        assert_eq!((r.ocupacion.saqueo_poblacion, r.ocupacion.edificios_danados, r.ocupacion.coste_reconstruccion), (250_000, 250_000, 500_000));
        let b = &r.batalla;
        assert_eq!((b.capacidad_asedio, b.capacidad_resto), (15, 5), "15 contra 15 en el asedio (2026-10-13)");
        assert_eq!((b.duracion_asedio, b.duracion_resto), (30 * MS_POR_MINUTO, 15 * MS_POR_MINUTO));
        assert_eq!((b.convocatoria, b.plazo_asignacion, b.plazo_inicio, b.margen), (30_000, 5 * MS_POR_MINUTO, 5 * MS_POR_MINUTO, 5 * MS_POR_MINUTO));
        assert_eq!((b.conservar_cerradas, b.carreteros, b.casillas_inventario), (60 * MS_POR_MINUTO, (Tropa::MiliciaLanceros, 13), 72));
    }

    #[test]
    fn los_bandidos_traen_la_calibracion_de_distancias() {
        let b = r().bandidos;
        assert_eq!((b.radio_ataque_caravana, b.anillo_min, b.anillo_max), (7_000, 30_000, 50_000));
        assert_eq!(b.niveles.clone().map(|n| (n.poder, n.unidades, n.oro_por_heroe, n.peso)), [(20_000, 10, 30_000, 500_000), (60_000, 30, 33_000, 300_000), (120_000, 60, 36_000, 200_000)]);
        assert_eq!((b.ventana, b.completas, b.caida_por_cada, b.solo_experiencia_desde), (24 * 60 * MS_POR_MINUTO, 8, 150_000, 14));
        assert_eq!((b.residentes_por_bandido, b.anillo_minimo, b.anillo_maximo, b.reaparicion_anillo, b.respawn), (2, 1, 6, 10 * MS_POR_MINUTO, 60 * MS_POR_MINUTO));
        assert_eq!(b.niveles.iter().map(|n| n.peso).sum::<i64>(), UNO, "los pesos suman 1");
    }

    #[test]
    fn los_mineros_del_pueblo() {
        assert_eq!(r().pueblos, ReglasPueblos { mineros: 30, reposicion: 360 * MS_POR_MINUTO });
    }

    #[test]
    fn la_chatarra_sale_del_metal_del_equipo() {
        let r = r();
        // Sin equipo de metal (madera, cuero) no deja nada.
        assert!(r.chatarra.de_muertos(Tropa::MiliciaLanceros, 100).is_empty());
        // Los espadachines de bronce llevan arma de bronce y armadura de metal: dejan chatarra de bronce.
        let c = r.chatarra.de_muertos(Tropa::EspadachinesBronce, 10);
        assert!(c.iter().all(|&(rec, n)| n > 0 && matches!(rec, Recurso::ChatarraCobre | Recurso::ChatarraBronce | Recurso::ChatarraHierro)));
        assert!(!c.is_empty());
        assert!(r.chatarra.de_muertos(Tropa::EspadachinesBronce, 0).is_empty());
        assert_eq!(r.chatarra.de_muertos(Tropa::EspadachinesBronce, 20).iter().map(|x| x.1).sum::<Mil>(), 2 * c.iter().map(|x| x.1).sum::<Mil>(), "lineal en los caídos");
    }

    #[test]
    fn la_identidad_son_los_bloques_que_lee_el_crate() {
        let t = texto_de_parametros();
        for nombre in ["COMBATE_NUMERICO", "AVANZADA", "BATALLA", "OCUPACION", "CAMPAMENTOS_BANDIDOS", "MOVIMIENTO", "CHATARRA"] {
            assert!(t.contains(&format!("<!-- parametros:{nombre} -->")), "falta {nombre}");
        }
        assert!(!t.contains("worldgen") && !t.contains("RECURSO_RAREZA"), "no arrastra el resto del documento del mundo");
        assert_eq!(t.matches("<!-- parametros:").count(), t.matches("<!-- /parametros -->").count());
    }
}
