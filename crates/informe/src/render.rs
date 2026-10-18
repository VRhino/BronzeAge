//! Página HTML del informe: cabecera, resumen, métricas, gráficos, mapa, anomalías y decisiones.

use std::fmt::Write;

use crate::{esc, graficos, num, semaforo, Anomalia, Causa, Decision, Informe, Metrica, Semaforo};

const CSS: &str = r#"
:root{--fondo:#f6f7f9;--tarjeta:#fff;--texto:#1c2430;--suave:#5b6675;--borde:#d9dee6;
--verde:#14703a;--verde-f:#dff3e6;--ambar:#8a5a00;--ambar-f:#fdf0cc;--rojo:#b3261e;--rojo-f:#fbe0de;
--gris:#4d5766;--gris-f:#e7eaee;--acento:#1f5fbf;--mundo:#eef1f5;
--s0:#2563eb;--s1:#d97706;--s2:#059669;--s3:#9333ea;--s4:#dc2626;--s5:#0891b2}
@media (prefers-color-scheme:dark){:root{--fondo:#12161c;--tarjeta:#1b212a;--texto:#e6e9ee;--suave:#9aa5b5;--borde:#323a47;
--verde:#6fd69a;--verde-f:#173a28;--ambar:#f1c46a;--ambar-f:#42330f;--rojo:#ff8a82;--rojo-f:#4a1d1a;
--gris:#b4bdca;--gris-f:#2a313c;--acento:#7fb0ff;--mundo:#232a34;
--s0:#6ea0ff;--s1:#f0a24a;--s2:#4fd1a1;--s3:#c08cf5;--s4:#ff7b7b;--s5:#4fc3dc}}
*{box-sizing:border-box}
body{margin:0;background:var(--fondo);color:var(--texto);font:16px/1.5 system-ui,-apple-system,"Segoe UI",Roboto,sans-serif}
main{max-width:960px;margin:0 auto;padding:16px}
h1{font-size:1.5rem;margin:0 0 4px}
h2{font-size:1.2rem;margin:32px 0 12px;padding-bottom:6px;border-bottom:1px solid var(--borde)}
h3{font-size:1rem;margin:0 0 8px}
.meta{color:var(--suave);font-size:.9rem;display:flex;flex-wrap:wrap;gap:4px 16px}
.chips{display:flex;flex-wrap:wrap;gap:8px;margin:16px 0}
.chip,.sem{display:inline-block;border-radius:999px;padding:2px 10px;font-weight:600;font-size:.85rem;white-space:nowrap}
.chip{font-size:1rem;padding:4px 14px}
.verde{background:var(--verde-f);color:var(--verde)}
.ambar{background:var(--ambar-f);color:var(--ambar)}
.rojo{background:var(--rojo-f);color:var(--rojo)}
.gris{background:var(--gris-f);color:var(--gris)}
.resumen{background:var(--tarjeta);border:1px solid var(--borde);border-radius:8px;padding:12px 16px;margin:0}
.tabla{overflow-x:auto;background:var(--tarjeta);border:1px solid var(--borde);border-radius:8px}
table{border-collapse:collapse;width:100%;min-width:640px;font-size:.92rem}
th,td{text-align:left;padding:8px 10px;border-bottom:1px solid var(--borde);vertical-align:top}
th{color:var(--suave);font-weight:600;font-size:.8rem;text-transform:uppercase;letter-spacing:.03em}
tr:last-child td{border-bottom:0}
td.n{white-space:nowrap}
td:first-child{min-width:170px}
.nota{color:var(--suave)}
.graficos{display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,420px),1fr));gap:16px}
.grafico,.tarjeta{background:var(--tarjeta);border:1px solid var(--borde);border-radius:8px;padding:12px}
svg{display:block;width:100%;height:auto}
svg:not(.mapa){max-width:560px;margin:0 auto}
svg text{fill:var(--suave);font:14px system-ui,sans-serif}
svg .tt{fill:var(--texto);font-size:14px}
.rej{stroke:var(--borde);stroke-width:1}
.eje{stroke:var(--suave);stroke-width:1}
.l{fill:none;stroke-width:2.2;stroke-linejoin:round}
.s0{stroke:var(--s0)}.s1{stroke:var(--s1)}.s2{stroke:var(--s2)}.s3{stroke:var(--s3)}.s4{stroke:var(--s4)}.s5{stroke:var(--s5)}
.f0{fill:var(--s0)}.f1{fill:var(--s1)}.f2{fill:var(--s2)}.f3{fill:var(--s3)}.f4{fill:var(--s4)}.f5{fill:var(--s5)}
.muestra{box-shadow:inset 0 0 0 1px var(--borde);display:inline-block;width:12px;height:12px;border-radius:3px;margin-right:6px;vertical-align:-1px}
span.f0{background:var(--s0)}span.f1{background:var(--s1)}span.f2{background:var(--s2)}span.f3{background:var(--s3)}span.f4{background:var(--s4)}span.f5{background:var(--s5)}
.leyenda{list-style:none;margin:8px 0 0;padding:0;display:flex;flex-wrap:wrap;gap:4px 16px;font-size:.88rem}
.vacio{color:var(--suave);margin:0}
.mundo{fill:var(--mundo);stroke:var(--suave);stroke-width:2;vector-effect:non-scaling-stroke}
.zona{fill-opacity:.15;stroke-width:1.5;vector-effect:non-scaling-stroke}
.pie{color:var(--suave);font-size:.85rem;margin:8px 0 0}
.tarjeta{margin-bottom:12px}
.tarjeta h3{display:flex;flex-wrap:wrap;gap:4px 10px;align-items:baseline}
.tarjeta p{margin:4px 0}
.etiqueta{font-size:.78rem;font-weight:600;border:1px solid var(--borde);border-radius:4px;padding:1px 6px;color:var(--suave);text-transform:uppercase}
.id{color:var(--acento);font-weight:700}
ol.opciones{margin:8px 0 0;padding-left:22px}
ol.opciones li{margin:6px 0}
.rec{font-weight:600}
.efecto{display:block;color:var(--suave);font-size:.9rem}
.estrella{color:var(--verde);font-weight:700}
dl.params{display:grid;grid-template-columns:minmax(0,1fr) minmax(0,1fr);gap:6px 16px;margin:0}
dl.params dt{color:var(--suave)}dl.params dd{margin:0;font-weight:600;overflow-wrap:anywhere}
"#;

/// Símbolo y texto del semáforo, para no depender solo del color.
fn etiqueta(s: Semaforo) -> (&'static str, &'static str) {
    match s {
        Semaforo::Verde => ("verde", "✔ Cumple"),
        Semaforo::Ambar => ("ambar", "▲ Cerca"),
        Semaforo::Rojo => ("rojo", "✖ Falla"),
        Semaforo::Informativa => ("gris", "● Informativa"),
        Semaforo::SinDato => ("gris", "○ Sin dato"),
    }
}

fn insignia(s: Semaforo) -> String {
    let (clase, texto) = etiqueta(s);
    format!("<span class=\"sem {clase}\">{texto}</span>")
}

/// Valor con su unidad ("31,2 u").
fn con_unidad(v: f64, unidad: &str) -> String {
    if unidad.is_empty() {
        num(v)
    } else {
        format!("{} {}", num(v), unidad)
    }
}

/// Rango esperado legible: «≥ 30 u», «5–15 u», «≤ 3 u».
fn esperado(m: &Metrica) -> String {
    let texto = match (m.esperado_min, m.esperado_max) {
        (Some(a), Some(b)) if a == b => format!("= {}", num(a)),
        (Some(a), Some(b)) => format!("{}–{}", num(a), num(b)),
        (Some(a), None) => format!("≥ {}", num(a)),
        (None, Some(b)) => format!("≤ {}", num(b)),
        (None, None) => return "—".to_string(),
    };
    if m.unidad.is_empty() {
        texto
    } else {
        format!("{texto} {}", m.unidad)
    }
}

/// Orden de aparición en la tabla: lo que falla primero.
fn prioridad(s: Semaforo) -> u8 {
    match s {
        Semaforo::Rojo => 0,
        Semaforo::Ambar => 1,
        Semaforo::SinDato => 2,
        Semaforo::Verde => 3,
        Semaforo::Informativa => 4,
    }
}

fn cabecera(i: &Informe) -> String {
    let mut o = format!("<header><h1>Informe de batch: {}</h1><div class=\"meta\">", esc(&i.sistema));
    let _ = write!(o, "<span>Fecha: {}</span>", esc(&i.fecha));
    if !i.mundo_hash.is_empty() {
        let corto: String = i.mundo_hash.chars().take(8).collect();
        let _ = write!(o, "<span>Mundo: <code>{}</code></span>", esc(&corto));
    }
    let _ = write!(o, "<span>Semilla: {}</span><span>Días simulados: {}</span></div></header>", i.semilla, num(i.dias_simulados));
    o
}

fn resumen(i: &Informe) -> String {
    let cuenta = |s: Semaforo| i.metricas.iter().filter(|m| semaforo(m) == s).count();
    let mut o = String::from("<section><h2>Resumen</h2><div class=\"chips\">");
    for (s, nombre) in [(Semaforo::Verde, "cumplen"), (Semaforo::Ambar, "cerca del límite"), (Semaforo::Rojo, "fallan")] {
        let (clase, simbolo) = etiqueta(s);
        let _ = write!(
            o,
            "<span class=\"chip {clase}\">{} {} {nombre}</span>",
            simbolo.split(' ').next().unwrap_or(""),
            cuenta(s)
        );
    }
    let otras = cuenta(Semaforo::Informativa) + cuenta(Semaforo::SinDato);
    if otras > 0 {
        let _ = write!(o, "<span class=\"chip gris\">● {otras} informativas o sin dato</span>");
    }
    o.push_str("</div>");
    if !i.resumen.is_empty() {
        let _ = write!(o, "<p class=\"resumen\">{}</p>", esc(&i.resumen));
    }
    o + "</section>"
}

fn tabla_metricas(i: &Informe) -> String {
    let mut filas: Vec<(Semaforo, &Metrica)> = i.metricas.iter().map(|m| (semaforo(m), m)).collect();
    filas.sort_by_key(|(s, _)| prioridad(*s)); // estable: conserva el orden original dentro de cada color
    let mut o = String::from(
        "<section><h2>Métricas</h2><div class=\"tabla\"><table><thead><tr><th>Métrica</th><th>Esperado</th><th>Medido</th><th>Estado</th><th>Origen</th><th>Nota</th></tr></thead><tbody>",
    );
    for (s, m) in filas {
        let medido = m.medido.map_or("sin dato".to_string(), |v| con_unidad(v, &m.unidad));
        let _ = write!(
            o,
            "<tr><td>{}</td><td class=\"n\">{}</td><td class=\"n\"><strong>{}</strong></td><td>{}</td><td>{}</td><td class=\"nota\">{}</td></tr>",
            esc(&m.nombre),
            esc(&esperado(m)),
            esc(&medido),
            insignia(s),
            esc(&m.origen),
            esc(m.nota.as_deref().unwrap_or(""))
        );
    }
    if i.metricas.is_empty() {
        o.push_str("<tr><td colspan=\"6\" class=\"nota\">No hay métricas en este informe.</td></tr>");
    }
    o + "</tbody></table></div></section>"
}

fn seccion_graficos(i: &Informe) -> String {
    if i.series.is_empty() {
        return String::new();
    }
    let cuerpo: String = i.series.iter().map(graficos::grafico).collect();
    format!("<section><h2>Gráficos</h2><div class=\"graficos\">{cuerpo}</div></section>")
}

fn seccion_mapa(i: &Informe) -> String {
    let Some(m) = i.mapa.as_ref() else { return String::new() };
    let Some(svg) = graficos::mapa(m) else { return String::new() };
    format!(
        "<section><h2>Mapa</h2><div class=\"tarjeta\">{svg}<p class=\"pie\">Mundo de {} × {} u; el norte está arriba.</p></div></section>",
        num(m.ancho),
        num(m.alto)
    )
}

fn nombre_causa(c: Causa) -> &'static str {
    match c {
        Causa::Codigo => "código",
        Causa::Diseno => "diseño",
        Causa::Medicion => "medición",
    }
}

fn anomalia(a: &Anomalia) -> String {
    let causa = a
        .causa
        .map_or(String::new(), |c| format!("<span class=\"etiqueta\">{}</span>", nombre_causa(c)));
    let detalle = if a.detalle.is_empty() { String::new() } else { format!("<p>{}</p>", esc(&a.detalle)) };
    format!("<div class=\"tarjeta\"><h3>{}{causa}</h3>{detalle}</div>", esc(&a.titulo))
}

fn seccion_anomalias(i: &Informe) -> String {
    let cuerpo = if i.anomalias.is_empty() {
        "<p class=\"vacio\">No se detectaron anomalías.</p>".to_string()
    } else {
        i.anomalias.iter().map(anomalia).collect()
    };
    format!("<section><h2>Anomalías</h2>{cuerpo}</section>")
}

fn decision(d: &Decision) -> String {
    let mut o = format!("<div class=\"tarjeta\"><h3><span class=\"id\">{}</span> {}</h3>", esc(&d.id), esc(&d.pregunta));
    if !d.contexto.is_empty() {
        let _ = write!(o, "<p>{}</p>", esc(&d.contexto));
    }
    let mut opciones: Vec<_> = d.opciones.iter().collect();
    opciones.sort_by_key(|op| !op.recomendada); // estable: la recomendada va primera
    o.push_str("<ol class=\"opciones\">");
    for op in opciones {
        let (clase, marca) = if op.recomendada {
            (" class=\"rec\"", "<span class=\"estrella\">★ Recomendada: </span>")
        } else {
            ("", "")
        };
        let efecto = op.efecto.as_deref().map_or(String::new(), |e| format!("<span class=\"efecto\">Efecto: {}</span>", esc(e)));
        let _ = write!(o, "<li{clase}>{marca}{}{efecto}</li>", esc(&op.texto));
    }
    o + "</ol></div>"
}

fn seccion_decisiones(i: &Informe) -> String {
    let cuerpo = if i.decisiones.is_empty() {
        "<p class=\"vacio\">No hay decisiones pendientes.</p>".to_string()
    } else {
        i.decisiones.iter().map(decision).collect()
    };
    format!("<section><h2>Decisiones para ti</h2>{cuerpo}</section>")
}

fn seccion_parametros(i: &Informe) -> String {
    if i.parametros.is_empty() {
        return String::new();
    }
    let mut o = String::from("<section><h2>Parámetros usados</h2><div class=\"tarjeta\"><dl class=\"params\">");
    for p in &i.parametros {
        let _ = write!(o, "<dt>{}</dt><dd>{}</dd>", esc(&p.nombre), esc(&p.valor));
    }
    o + "</dl></div></section>"
}

/// Página HTML completa y autónoma (CSS y SVG en línea, sin scripts).
pub fn renderizar(i: &Informe) -> String {
    format!(
        "<!doctype html><html lang=\"es\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Informe: {}</title><style>{CSS}</style></head><body><main>{}{}{}{}{}{}{}{}</main></body></html>",
        esc(&i.sistema),
        cabecera(i),
        resumen(i),
        tabla_metricas(i),
        seccion_graficos(i),
        seccion_mapa(i),
        seccion_anomalias(i),
        seccion_decisiones(i),
        seccion_parametros(i),
    )
}
