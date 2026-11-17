//! Las variables de balance del libro de agravios: viven aquí, en el servicio de bots, y no en el motor (decisión del autor, 2026-10-31). Son valores iniciales, a calibrar midiendo partidas.

/// Cómo pesa cada apunte en la relación, en puntos de −100 a 100.
pub const PESO_PLAZA_CONQUISTADA: f64 = -60.0;
pub const PESO_ALIANZA_ROTA: f64 = -50.0;
pub const PESO_GUERRA_DECLARADA: f64 = -20.0;
pub const PESO_PUEBLO_SAQUEADO: f64 = -25.0;
pub const PESO_CARAVANA_ASALTADA: f64 = -15.0;
pub const PESO_PLAZA_SITIADA: f64 = -15.0;
pub const PESO_TRUEQUE_INCUMPLIDO: f64 = -10.0;
pub const PESO_TRUEQUE_CUMPLIDO: f64 = 5.0;
pub const PESO_ENEMIGO_COMUN: f64 = 10.0;

/// El roce de tener plazas vecinas, por hora de mundo: con el olvido de 7 días se estabiliza cerca de −48.
pub const ROCE_FRONTERA_POR_HORA: f64 = -0.2;
/// Hasta dónde dos plazas se disputan la comarca, en milésimas.
pub const DISTANCIA_FRONTERA: i64 = 80_000;
/// Lo que puede pesar como mucho una clase de apunte.
pub const TOPE_DE_ASIENTO: f64 = 100.0;

/// Los umbrales de la escalera.
pub const UMBRAL_ALIANZA: f64 = 50.0;
pub const UMBRAL_AMISTAD: f64 = 20.0;
pub const UMBRAL_RECELO: f64 = -20.0;
pub const UMBRAL_HOSTILIDAD: f64 = -50.0;
pub const UMBRAL_ODIO: f64 = -75.0;

/// El olvido: un apunte pierde la mitad de su peso cada tanto tiempo de mundo.
pub const VIDA_MEDIA_MS: i64 = 7 * 86_400_000;

/// Facción nueva: hasta qué nivel y cuánto tiempo desde que un bot la vio por primera vez. Ha de cumplir las dos y no tener nada anotado.
pub const NUEVA_NIVEL_MAX: u32 = 1;
pub const NUEVA_TIEMPO_MS: i64 = 7 * 86_400_000;

/// Cuántas Facciones de bots pueden tener por enemigo jurado a una misma.
pub const FACCIONES_CONTRA_UNA: usize = 3;

/// Cuánto dura lo que marca una cicatriz, en ms de mundo.
pub const CICATRIZ_CARAVANAS_MS: i64 = 7 * 86_400_000;
pub const CICATRIZ_SAQUEO_MS: i64 = 7 * 86_400_000;
pub const CICATRIZ_DERROTA_MS: i64 = 5 * 86_400_000;
pub const CICATRIZ_ASEDIO_MS: i64 = 7 * 86_400_000;
/// Cuánto margen se quita quien resistió un asedio contra quien lo sitió.
pub const FACTOR_CONFIANZA: f64 = 0.8;
/// Cuánta relación de más pide a las demás una Facción traicionada para aliarse.
pub const RECARGO_TRAS_TRAICION: f64 = 20.0;

/// El presupuesto de intel de un bot: compras al día por Facción.
pub const INTEL_POR_DIA: u32 = 2;
/// A qué distancia de casa mira una Mirada a ciegas, en milésimas: los vecinos están cerca.
pub const MIRADA_A_CIEGAS: i64 = 60_000;

/// El fundador no sale hasta que su grupo suma tantas veces el poder del bandido más fuerte del anillo: la caravana se defiende con el poder de la columna que la lleva.
pub const MARGEN_PARA_FUNDAR: f64 = 1.2;

/// Cuánto más allá del anillo de bandidos busca sitio para fundar, en milésimas, cuántos sitios compara y a qué distancia acosa un bandido a quien pasa.
pub const FUNDAR_MAS_ALLA: i64 = 15_000;
pub const SITIOS_A_COMPARAR: u32 = 6;
pub const RADIO_ACOSO: i64 = 7_000;
/// Cuánto recuerda una Facción dónde hay un campamento de bandidos: no se mueven, y el camino de una caravana se juzga con lo que recuerda.
pub const RECUERDO_BANDIDO_MS: i64 = 7 * 24 * 3_600_000;
/// Cuánto vale que un bot sin plaza diga «este campamento es mío»: los demás buscan otra presa. Si no vuelve a tiempo, queda libre.
pub const CAZA_RESERVADA_MS: i64 = 2 * 3_600_000;
/// Lo más que espera en la puerta, con la Caravana de Fundación, a que el grupo pueda con lo que hay en el camino: con horario, los compañeros no están todos a la vez.
pub const ESPERA_PARA_FUNDAR_MS: i64 = 12 * 3_600_000;
/// Lo que vale una tarea del tablero de la Facción si quien la lleva no la renueva (se desconecta, desaparece): después, otro la puede tomar.
pub const TAREA_VALE_MS: i64 = 24 * 3_600_000;
/// Lo más que espera en la puerta un grupo de caza o de campaña a juntar la fuerza que necesita; después se deshace.
pub const ESPERA_GRUPO_TOPE_MS: i64 = 2 * 3_600_000;
/// Lo que la Facción deja en paz un objetivo (un campamento de bandidos, una plaza) después de que un grupo no juntara fuerza para él: si no, volvería a esperar en la puerta.
pub const VETO_TRAS_DESHACER_MS: i64 = 6 * 3_600_000;
/// El lado de las casillas de la rejilla con que la Facción sabe qué ha visto de su región (en milésimas).
pub const CASILLA: i64 = 20_000;
/// Lo que vale «esta casilla ya se vio»: después vuelve a estar por ver (los campamentos de bandidos reaparecen, las plazas cambian).
pub const CASILLA_VALE_MS: i64 = 24 * 3_600_000;
/// Lo que un explorador aparta la casilla a la que va.
pub const CASILLA_APARTADA_MS: i64 = 6 * 3_600_000;
/// Lo lejos que un explorador elige su casilla de las que ya llevan otros de su Facción: cada uno abre su propio frente.
pub const SEPARACION_EXPLORADORES: i64 = 40_000;
/// Cuántas veces sale mal fundar (la espera se acaba sin fuerza, o ningún sitio admite la plaza) antes de que una Facción sin plaza se rinda: sus héroes la dejan y piden
/// entrar en una Facción de bots que ya tenga plaza, como haría un jugador.
pub const FRACASOS_PARA_RENDIRSE: u32 = 3;
/// A cuánto de la línea recta se mira si hay bandidos antes de lanzar una caravana: el camino real no va recto.
pub const ANCHO_DEL_CAMINO: i64 = 20_000;
/// Hasta dónde caza una plaza los campamentos de su alrededor, en milésimas: los que le cortan las caravanas.
pub const RADIO_DE_CAZA: i64 = 30_000;
/// Una plaza sale de caza como mucho cada tanto: al ritmo con que reaparecen los campamentos de bandidos (una oleada cada 2 h).
pub const ENTRE_CAZAS_MS: i64 = 2 * 3_600_000;
/// Cada plaza sale de batida por la rejilla como mucho cada tanto (la militarista, el doble de a menudo).
pub const ENTRE_BATIDAS_MS: i64 = 6 * 3_600_000;

/// Cuánto espera un bot para volver a saquear un mismo pueblo.
pub const ENTRE_SAQUEOS_MS: i64 = 24 * 3_600_000;

/// Cuánto espera una Facción para lanzar otra Caravana de Fundación desde una plaza: una perdida no se repone al momento.
pub const ENTRE_CARAVANAS_DE_FUNDACION_MS: i64 = 12 * 3_600_000;

/// Una Facción joven se queda en su región: nada de exploradores, búsquedas ni campañas a más de `REGION` de sus plazas (en milésimas). Es estable, y mira más allá, cuando una plaza
/// llega a `ESTABLE_NIVEL` con `ESTABLE_PUEBLOS` pueblos terminados.
pub const REGION: i64 = 120_000;
pub const ESTABLE_NIVEL: u32 = 2;
pub const ESTABLE_PUEBLOS: usize = 2;

/// El estaño: desde qué nivel de plaza lo busca la Facción, cuánto pide por trueque (en milésimas), cada cuánto lo pide y cuánto enfría cada día una disputa por él.
pub const ESTANO_NIVEL_PLAZA: u32 = 2;
pub const ESTANO_POR_TRUEQUE: i64 = 200_000;
pub const ENTRE_PETICIONES_ESTANO_MS: i64 = 24 * 3_600_000;
pub const PESO_DISPUTA_ESTANO: f64 = -30.0;
/// Cómo se llega a un yacimiento lejano: hasta aquí se funda a su lado; más lejos, se avanza de plaza en plaza con este paso. En milésimas.
pub const ESTANO_A_MANO: i64 = 70_000;
pub const ESTANO_PASO: i64 = 70_000;
/// Hasta qué distancia de casa se va a por el estaño fundando: si el camino no es seguro, espera antes que gastar el cupo de plazas en otra parte. Más lejos no llegaría con ese cupo.
pub const ESTANO_ALCANCE: i64 = 280_000;
/// Cuántos tramos hace una búsqueda: la del buscador de estaño (cada tramo, una casilla de la rejilla) y la del mercenario que no ve a su objetivo (en espiral: a qué
/// distancia va el primero y cuánto se aleja en cada uno).
pub const TRAMOS_DE_BUSQUEDA: u32 = 14;
pub const BUSQUEDA_RADIO_INICIAL: i64 = 40_000;
pub const BUSQUEDA_RADIO_POR_TRAMO: i64 = 20_000;
/// Cuánto gira en cada tramo, en grados, y cuánto tarda otro en relevarlo si se pierde o vuelve sin nada.
pub const BUSQUEDA_GIRO: u32 = 60;
pub const ENTRE_BUSQUEDAS_MS: i64 = 12 * 3_600_000;
/// El buscador de estaño de una Facción estable busca por anillos alrededor de su plaza: hasta `ESTANO_ANILLO`; cuando ya vio todo ahí, hasta el doble, y así hasta
/// `ESTANO_ANILLOS` anillos. El estaño no se mueve: una casilla ya vista no se vuelve a buscar. Una Facción joven no sale de su región.
pub const ESTANO_ANILLO: i64 = 200_000;
pub const ESTANO_ANILLOS: i64 = 10;

/// La ronda del mercenario sin contrato: hasta dónde caza, en milésimas, cuánto más fuerte ha de ser que el campamento y cada cuánto sale.
pub const RADIO_DE_RONDA: i64 = 25_000;
pub const MARGEN_DE_RONDA: f64 = 1.2;
pub const ENTRE_RONDAS_MS: i64 = 12 * 3_600_000;
pub const TRAMOS_DE_RONDA: u32 = 4;
/// Hasta dónde se considera de una plaza lo que tiene alrededor, en milésimas: la zona máxima de nivel 3.
pub const ZONA_DE_UNA_PLAZA: i64 = 20_000;

/// Cuánto vive un objetivo de varios días antes de replantearse.
pub const OBJETIVO_MS: i64 = 14 * 86_400_000;

/// El mando de antagonismo: domina quien tiene tantas veces las plazas de la media (y al menos tantas), y el roce que eso causa por hora.
pub const DOMINIO_VECES: f64 = 2.0;
pub const DOMINIO_PLAZAS_MIN: usize = 4;
pub const ROCE_DOMINIO_POR_HORA: f64 = -0.25;
/// Lo que une a quienes temen al mismo, por hora.
pub const UNION_CONTRA_DOMINIO_POR_HORA: f64 = 0.125;
/// Lo que vale «estoy dentro del campamento y listo para salir» en la pizarra de un bot sin plaza: lo renueva cada vez que piensa, así que con más se fían de compañeros
/// que ya se fueron, y los bandidos reaparecen cada 2 h.
pub const LISTO_MS: i64 = 10 * 60_000;
/// Al elegir dónde fundar, cuentan los yacimientos a esta distancia del sitio: la zona de una plaza de nivel 2, donde ya puede marcar su pueblo.
pub const NODO_EN_ZONA: i64 = 15_000;
/// Lo que vale para fundar al lado de cada tipo de yacimiento que la Facción conoce (su base de yacimientos), por lo que necesita para sus metas: el estaño y el cobre del
/// bronce (sin él no hay nivel 3), la piedra de las obras, el hierro de la Era II y el oro. Cada tipo cuenta una vez por sitio, dividido entre 1 + los pueblos que la
/// Facción ya saca de él.
pub const PESO_NODO_PARA_FUNDAR: &[(&str, f64)] = &[("estano", 5.0), ("cobre", 4.0), ("piedra", 2.0), ("hierro", 1.0), ("oro", 1.0)];
/// Con una escuadra propia ya, un bot no recluta más mientras su plaza no tenga los pesants que pide su subida siguiente (reclutar gasta población: M9). Con `false`,
/// completa sus tropas aunque retrase el ascenso.
pub const LEVA_ESPERA_AL_ASCENSO: bool = true;
/// Lo que dura una necesidad en el tablero de la Facción si ninguna plaza la vuelve a apuntar.
pub const NECESIDAD_VALE_MS: i64 = 30 * 60_000;
/// El rumor de las vetas de estaño de la taberna: lo que cuesta (el motor, `INTEL.rumores.vetasEstano.oro`; si cambia, rechaza con `intel.sin_oro`), el oro que la plaza se guarda
/// además, y cada cuánto lo vuelve a intentar cada plaza.
pub const RUMOR_ESTANO_ORO: i64 = 80;
pub const RUMOR_ESTANO_RESERVA: i64 = 100;
pub const ENTRE_RUMORES_MS: i64 = 7 * 24 * 3_600_000;
/// Hasta qué nivel sube sus pueblos (aldeas, Cantera y minas; el motor, 2026-11-10): nunca por encima del de la plaza madre.
pub const NIVEL_MAXIMO_PUEBLO: u32 = 3;
/// Los logros de una Facción de bots y sus puntos (por clase: guerra, obras, exploración, comercio). Quien lo logra se lleva los puntos enteros; cada uno de los demás que
/// estuvieron, esta parte. Los puntos no se olvidan (decisión de Víctor, 2026-11-11).
pub const PARTE_DE_PARTICIPANTE: f64 = 0.5;
/// Cuántos logros guarda la pizarra para el panel (los puntos cuentan todos).
pub const LOGROS_GUARDADOS: usize = 500;
/// Los cargos van por puntos: cada cuánto se revisan en cada plaza, y cuánto más tiene que tener el aspirante que el titular para quitarle el cargo.
pub const REVISION_CARGOS_MS: i64 = 24 * 3_600_000;
pub const MARGEN_PARA_EL_CARGO: f64 = 1.25;

/// Los puntos de cada logro.
pub mod puntos {
    pub const CAMPAMENTO_DESTRUIDO: u64 = 10;
    pub const PLAZA_CONQUISTADA: u64 = 50;
    pub const ASEDIO_RESISTIDO: u64 = 30;
    pub const PUEBLO_SAQUEADO: u64 = 5;
    pub const PLAZA_FUNDADA: u64 = 40;
    pub const ESTANO_DESCUBIERTO: u64 = 20;
    /// Por cada yacimiento de cobre, hierro, oro o piedra que la Facción no conocía.
    pub const COBRE_DESCUBIERTO: u64 = 5;
    pub const HIERRO_DESCUBIERTO: u64 = 5;
    pub const ORO_DESCUBIERTO: u64 = 5;
    pub const PIEDRA_DESCUBIERTA: u64 = 1;
    pub const MATERIALES_ENTREGADOS: u64 = 5;
    pub const OBRA_TERMINADA: u64 = 10;
    pub const TRUEQUE_CUMPLIDO: u64 = 5;
    pub const CARAVANA_EN_CASA: u64 = 1;
    /// Cuando a la Facción le aparece una tecnología.
    pub const TECNOLOGIA_APARECIDA: u64 = 10;
    /// Cuando la Facción es la primera del mundo en tener una tecnología.
    pub const TECNOLOGIA_DESCUBIERTA: u64 = 30;
}
/// La clase de cada bien, para el tesorero: lo que se come cada día, la comida de lujo (hace crecer a artesanos y nobleza, y se puede dejar de comer para venderla), el
/// material de obra, el mineral, los lingotes, las armas y el oro. Lo que no está aquí es materia prima.
pub const CLASES_DE_BIENES: &[(&str, &str)] = &[
    ("trigo", "comida"), ("pan", "comida"), ("carne", "comida"), ("leche", "comida"),
    ("queso", "lujo"), ("vino", "lujo"), ("cerveza", "lujo"), ("aceite", "lujo"),
    ("madera", "obra"), ("piedra", "obra"),
    ("cobre", "mineral"), ("estano", "mineral"), ("hierro", "mineral"),
    ("lingoteCobre", "lingote"), ("lingoteEstano", "lingote"), ("lingoteBronce", "lingote"), ("lingoteHierro", "lingote"),
    ("oro", "oro"),
];

/// La clase de un bien ([`CLASES_DE_BIENES`]); las armas y armaduras, por su nombre.
pub fn clase_de_bien(bien: &str) -> &'static str {
    CLASES_DE_BIENES.iter().find(|(b, _)| *b == bien).map(|(_, c)| *c).unwrap_or(if bien.starts_with("arma") { "armas" } else { "materia" })
}

/// La meta del bronce de una plaza de nivel 2 o más (en unidades): los 100 lingotes que pide el nivel 3 y lo que hace falta para fundirlos (1,6 de lingote de cobre y 0,4
/// de estaño por lingote; 2 de cobre por lingote de cobre y 5 de estaño por lingote de estaño). No se vende y se compra antes que nada.
pub const META_BRONCE: &[(&str, i64)] = &[("lingoteBronce", 100), ("lingoteCobre", 160), ("lingoteEstano", 40), ("cobre", 320), ("estano", 200)];
/// Las horas de lo que come la plaza que guarda como meta de cada comida (más que la reserva de supervivencia).
pub const META_HORAS_DE_COMIDA: i64 = 24;
/// Un trueque se acepta si lo que recibe vale al menos esta parte de lo que entrega (a precio base).
pub const TRUEQUE_VALOR_MINIMO: f64 = 0.9;
/// La comida de lujo deja de comerse (para venderla) cuando a la plaza le falta oro (por debajo de esta parte de su capacidad) y le sobra de ella (por encima de la otra).
pub const LUJO_PAUSA_ORO: f64 = 0.2;
pub const LUJO_PAUSA_SOBRA: f64 = 0.5;
/// La prioridad de cada tecnología para el tablero de necesidades y para el Gobernador que levanta su hito (1 primero): el bronce antes que nada (sin él no hay nivel 3),
/// luego lo que abre tropas y la cantería. Las que no están, 3.
pub const PRIORIDAD_TECNOLOGIA: &[(&str, u8)] = &[
    ("aleacion_bronce", 1), ("metalurgia_cobre", 1),
    ("escudos_ligeros", 2), ("armamento_palacial", 2), ("arqueria_palacial", 2), ("cria_caballar", 2), ("canteria", 2),
];
pub fn prioridad_tecnologia(id: &str) -> u8 {
    PRIORIDAD_TECNOLOGIA.iter().find(|(t, _)| *t == id).map_or(3, |(_, p)| *p)
}
/// Por debajo de esta parte de su cupo, la guarnición de una plaza es una necesidad.
pub const GUARNICION_MINIMA: f64 = 0.5;
/// Lo que guarda la capital de un recurso que pide el hito de una tecnología (el estaño de la aleación), en unidades.
pub const RESERVA_DE_HITO: i64 = 10;
/// Un bot sin plaza vuelve a dar la vuelta al anillo de su campamento en busca de bandidos nuevos cuando la última vuelta (de cualquiera) acabó hace más que esto: siempre
/// hay uno mirando, para fundar cuanto antes.
pub const EXPLORAR_ANILLO_CADA_MS: i64 = 5 * 60_000;
