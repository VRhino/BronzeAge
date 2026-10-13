/** Desired regional identities for the experimental campaign.
 * These are audit targets, not production bonuses, resource caps or manual coordinates. */
export type ValorRegional='forestal'|'minero'|'agricola'|'ganadero'|'conexion'|'defensa';
export interface PerfilEstrategicoRegional {area:number;principal:ValorRegional;secundario:ValorRegional;disputa:string;}
export const PERFILES_ESTRATEGICOS_REGIONALES:readonly PerfilEstrategicoRegional[]=[
 {area:0,principal:'forestal',secundario:'conexion',disputa:'Accesos de la costa al interior'},
 {area:1,principal:'minero',secundario:'defensa',disputa:'Valles habitables próximos a zonas mineras'},
 {area:2,principal:'minero',secundario:'conexion',disputa:'Posiciones abastecidas junto a corredores'},
 {area:3,principal:'conexion',secundario:'minero',disputa:'Cruces entre regiones y acceso a fuentes locales'},
 {area:4,principal:'agricola',secundario:'conexion',disputa:'Suelo productivo próximo a varias conexiones'},
 {area:5,principal:'agricola',secundario:'ganadero',disputa:'Accesos entre áreas productivas; cruces fluviales pendientes de mecánica'},
 {area:6,principal:'conexion',secundario:'minero',disputa:'Salidas de valles hacia la llanura'},
 {area:7,principal:'forestal',secundario:'minero',disputa:'Valles abastecidos de madera y piedra'},
 {area:8,principal:'ganadero',secundario:'conexion',disputa:'Cruces entre costa e interior'},
 {area:9,principal:'agricola',secundario:'conexion',disputa:'Entradas y salidas del valle'},
 {area:10,principal:'ganadero',secundario:'conexion',disputa:'Conexiones entre zonas productivas separadas'},
 {area:11,principal:'defensa',secundario:'minero',disputa:'Acceso terrestre y posiciones resguardadas de la península'},
 {area:12,principal:'conexion',secundario:'minero',disputa:'Pasos principales y alternativas'},
 {area:13,principal:'conexion',secundario:'defensa',disputa:'Accesos del istmo a territorios vecinos'},
 {area:14,principal:'agricola',secundario:'defensa',disputa:'Terreno seco útil del delta; cruces fluviales pendientes de mecánica'},
 {area:15,principal:'minero',secundario:'conexion',disputa:'Pasos entre bolsas de riqueza de distintos valles'},
];
export const CRITERIOS_AUDITORIA_REGIONALES={radioInicial:90,radioMaximo:540,minimoArranques:4,
 densidadMadera:.15,separacionInicial:180,fraccionBosqueDestacada:.30,fraccionAgricolaDestacada:.12,
 fraccionPastosDestacada:.25,minimoMetalesEnFoco:2,factorRodeoRelevante:1.2,minimoParesDePaso:3};
