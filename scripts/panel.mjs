// Panel de arranque local (fase 7): enciende y para los servicios de .claude/launch.json, dice si escuchan y enseña sus logs.
// Uso: `node scripts/panel.mjs` o `panel.cmd` (abre http://localhost:3999). Solo Node, sin dependencias.
//
// Viene del panel del TypeScript (rama typescript-final, scripts/panel.mjs), sin su parte de carga y pruebas (telemetría, serie
// de carga, auditoría e informe): en Rust eso llega por el ámbito ADMIN del contrato 05 y es del admin web (tanda 7H).
//
// Partidas (fase 7, para el admin web): cada partida es una carpeta de datos del servidor en `partidas/<nombre>/` (`mundo/`, `acceso/`, `respaldos/` y
// `partida.json` con su semilla y su artefacto de mundo); la carpeta `datos/` de antes cuenta como la partida «datos». El servidor arranca con la partida
// actual. La API de partidas está más abajo (`/api/partidas`, `/api/mundo`), con CORS abierto: el admin la recibe como PANEL_URL.
//
// Cada servicio puede llevar `cwd` (relativo a la raíz del motor): el motor es esta carpeta y los repos hermanos están al lado
// (../BronceAgeBotsBrain, ../BronceAgeAdminClient, ../BronzeAgeClient). Si la carpeta no existe, la tarjeta lo dice y no arranca.
import http from 'node:http';
import net from 'node:net';
import { spawn, exec } from 'node:child_process';
import { cpSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const RAIZ = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const PUERTO = Number(process.env.PANEL_PUERTO ?? 3999);
const { configurations } = JSON.parse(readFileSync(path.join(RAIZ, '.claude/launch.json'), 'utf8'));

// Cómo le dice el panel a cada servicio su puerto y el del servidor. P = los puertos elegidos ahora mismo.
// El servidor sirve en un solo puerto el WebSocket del contrato 05 y el HTTP del acceso.
const VITE = (p) => ['--', '--port', p, '--strictPort'];
// `aviso`: por qué no se puede arrancar todavía (la tarjeta lo dice y deja el botón apagado).
const SERVICIOS = {
  // `parar`: el servidor se apaga con su foto al leer esa línea por la entrada estándar (Windows no deja mandarle Ctrl+C).
  'bronzeage-servidor': { tipo: 'backend', parar: 'parar\n', aviso: avisoMundo, con: (p) => ({ env: { BRONZEAGE_DIRECCION: `127.0.0.1:${p}`, ...envPartida() } }) },
  // Una tarea que termina sola: sin puerto. La semilla la elige POST /api/mundo.
  'generar-mundo': { tipo: 'tarea', con: () => ({ args: [String(mundo.semilla)] }) },
  'bronzeage-bots': { tipo: 'backend', con: (p, P) => ({ env: { BOTS_PUERTO: p, BOTS_SERVIDOR: `http://localhost:${P['bronzeage-servidor']}` } }) },
  'bronzeage-admin': { tipo: 'cliente', con: (p, P) => ({ args: VITE(p), env: {
    BACKEND_URL: `http://localhost:${P['bronzeage-servidor']}`, BOTS_URL: `ws://localhost:${P['bronzeage-bots']}/control`, PANEL_URL: `http://localhost:${PUERTO}` } }) },
  'bronzeage-cliente-jugador': { tipo: 'cliente', con: (p, P) => ({ args: VITE(p), env: { BACKEND_URL: `http://localhost:${P['bronzeage-servidor']}` } }) },
};

// El servidor necesita el artefacto de mundo (`BRONZEAGE_MUNDO`: un .bin o una carpeta con un mundo-*.bin; por defecto generador/salida/mundo).
function avisoMundo(c) {
  const ruta = path.resolve(RAIZ, envPartida().BRONZEAGE_MUNDO ?? c.env?.BRONZEAGE_MUNDO ?? process.env.BRONZEAGE_MUNDO ?? 'generador/salida/mundo');
  if (ruta.endsWith('.bin') ? existsSync(ruta) : existsSync(ruta) && readdirSync(ruta).some((f) => /^mundo-.*\.bin$/.test(f))) return null;
  if (procesos['generar-mundo']?.hijo) return 'generando el artefacto de mundo… (unos 15 min)';
  return `falta el artefacto de mundo en ${ruta}: arranca «generar-mundo» (unos 15 min) o pon BRONZEAGE_MUNDO con la ruta de un mundo-*.bin`;
}

// ───────────────────────────── Partidas ─────────────────────────────

const PARTIDAS = path.join(RAIZ, 'partidas');
const MUNDOS = path.join(RAIZ, 'generador/salida/mundo');
const ACTUAL = path.join(PARTIDAS, 'actual.txt');
const NOMBRE = /^[a-z0-9][a-z0-9_-]{0,39}$/;
const leerJson = (f) => { try { return JSON.parse(readFileSync(f, 'utf8')); } catch { return null; } };
// La carpeta `datos/` de antes de las partidas es la partida «datos» mientras no haya una `partidas/datos`.
const carpetaPartida = (n) => (n === 'datos' && !existsSync(path.join(PARTIDAS, 'datos')) ? path.join(RAIZ, 'datos') : path.join(PARTIDAS, n));
const nombresDePartida = () => {
  const n = existsSync(PARTIDAS) ? readdirSync(PARTIDAS).filter((x) => NOMBRE.test(x) && statSync(path.join(PARTIDAS, x)).isDirectory()) : [];
  if (existsSync(path.join(RAIZ, 'datos')) && !n.includes('datos')) n.unshift('datos');
  return n;
};
function partidaActual() {
  const a = existsSync(ACTUAL) ? readFileSync(ACTUAL, 'utf8').trim() : '';
  return nombresDePartida().includes(a) ? a : nombresDePartida()[0] ?? null;
}
function ponerActual(n) { mkdirSync(PARTIDAS, { recursive: true }); writeFileSync(ACTUAL, n); }
// Los artefactos generados, del más nuevo al más viejo.
const artefactos = () => (existsSync(MUNDOS) ? readdirSync(MUNDOS).filter((f) => /^mundo-.*\.bin$/.test(f)) : [])
  .map((f) => ({ archivo: f, hash: f.slice(6, -4), creado_ms: statSync(path.join(MUNDOS, f)).mtimeMs }))
  .sort((a, b) => b.creado_ms - a.creado_ms);
function respaldosDe(n) {
  const dir = path.join(carpetaPartida(n), 'respaldos');
  return (existsSync(dir) ? readdirSync(dir) : []).map((r) => leerJson(path.join(dir, r, 'respaldo.json'))).filter(Boolean)
    .sort((a, b) => b.creado_ms - a.creado_ms);
}
function infoPartida(n) {
  const meta = leerJson(path.join(carpetaPartida(n), 'partida.json')) ?? {};
  return { nombre: n, carpeta: carpetaPartida(n), semilla: meta.semilla ?? 42, mundo: meta.mundo ?? null, creada_ms: meta.creada_ms ?? null,
    respaldos: respaldosDe(n).length, actual: n === partidaActual() };
}
// El entorno del servidor para la partida actual: su carpeta, su semilla y su artefacto (si lo fijó al crearla).
function envPartida() {
  const n = partidaActual();
  if (!n) return { BRONZEAGE_DATOS: path.join(PARTIDAS, 'principal') };
  const i = infoPartida(n);
  return { BRONZEAGE_DATOS: i.carpeta, BRONZEAGE_SEMILLA: String(i.semilla), ...(i.mundo ? { BRONZEAGE_MUNDO: path.join(MUNDOS, i.mundo) } : {}) };
}

// La regeneración del mundo (la tarea «generar-mundo»): con qué semilla y cómo acabó la última.
const mundo = { semilla: 42, ultima: null };

const puertos = Object.fromEntries(configurations.map((c) => [c.name, c.port]));
const procesos = {}; // nombre -> { hijo, log[] }
const carpeta = (c) => path.resolve(RAIZ, c.cwd ?? '.');

function escucha(puerto) {
  return new Promise((ok) => {
    // 'localhost' + autoSelectFamily: Vite escucha solo en ::1, el servidor en 127.0.0.1.
    const s = net.connect({ port: puerto, host: 'localhost', autoSelectFamily: true });
    s.once('connect', () => { s.destroy(); ok(true); });
    s.once('error', () => ok(false));
    s.setTimeout(400, () => { s.destroy(); ok(false); });
  });
}

function arrancar(c) {
  if (procesos[c.name]?.hijo || !existsSync(carpeta(c)) || SERVICIOS[c.name]?.aviso?.(c)) return;
  const P = Object.fromEntries(Object.entries(puertos).map(([k, v]) => [k, String(v)]));
  const { args = [], env = {} } = SERVICIOS[c.name]?.con(P[c.name], P) ?? {};
  const p = { log: [] };
  // detached fuera de Windows: el hijo encabeza su grupo de procesos y parar() lo corta entero (shell:true deja un sh en medio).
  const hijo = spawn(c.runtimeExecutable, [...c.runtimeArgs, ...args], {
    cwd: carpeta(c), shell: true, detached: process.platform !== 'win32', env: { ...process.env, ...c.env, ...env, FORCE_COLOR: '0' },
  });
  const anotar = (b) => {
    p.log.push(...b.toString().replace(/\x1b\[[0-9;]*m/g, '').split(/\r?\n/).filter(Boolean));
    p.log.splice(0, Math.max(0, p.log.length - 200));
  };
  hijo.stdout.on('data', anotar);
  hijo.stderr.on('data', anotar);
  p.fin = new Promise((ok) => hijo.on('exit', (code) => {
    p.hijo = null; p.log.push(`— proceso terminado (código ${code}) —`);
    if (c.name === 'generar-mundo') mundo.ultima = { semilla: mundo.semilla, codigo: code, fin_ms: Date.now(), artefacto: artefactos()[0] ?? null };
    ok(code);
  }));
  p.hijo = hijo;
  procesos[c.name] = p;
}

// Corta el árbol entero: en Windows con taskkill (shell:true deja cargo/npm como nietos); fuera, al grupo de procesos.
function matar(h, senal = 'SIGINT') {
  if (process.platform === 'win32') exec(`taskkill /pid ${h.pid} /T /F`);
  else try { process.kill(-h.pid, senal); } catch { /* ya terminó */ }
}

function parar(nombre) {
  const h = procesos[nombre]?.hijo;
  if (!h) return;
  const linea = SERVICIOS[nombre]?.parar;
  if (!linea) return matar(h);
  // Primero por las buenas (foto final); si en 20 s no terminó, a la fuerza.
  h.stdin.write(linea);
  setTimeout(() => { if (procesos[nombre]?.hijo === h) matar(h, 'SIGKILL'); }, 20000);
}

async function estado() {
  return Promise.all(configurations.map(async (c) => {
    const p = procesos[c.name];
    return {
      nombre: c.name, tipo: SERVICIOS[c.name]?.tipo ?? 'backend', puerto: puertos[c.name],
      comando: [c.runtimeExecutable, ...c.runtimeArgs].join(' ') + (c.cwd ? `  (en ${c.cwd})` : '')
        + (c.name === 'bronzeage-servidor' ? `  [partida ${partidaActual() ?? 'principal (nueva)'}]` : ''),
      falta: !existsSync(carpeta(c)), aviso: SERVICIOS[c.name]?.aviso?.(c) ?? null,
      escucha: puertos[c.name] ? await escucha(puertos[c.name]) : false, lanzado: !!p?.hijo, log: p?.log.slice(-200) ?? [],
    };
  }));
}

const HTML = `<!doctype html><html lang="es"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1"><title>Panel BronzeAge</title>
<style>
:root{--bg:#14110d;--card:#1f1a14;--borde:#3a2f22;--txt:#eadfcc;--sub:#9c8b72;--on:#5fbf6a;--off:#c4553f;--wait:#d9a63c;--acento:#c98a3b;--cli:#1a1d20;--cli-borde:#2c3640;--cli-acento:#6fa8c9}
*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--txt);font:14px/1.4 system-ui,sans-serif;padding:24px 16px}
h1{font-size:18px;margin:0 0 18px;letter-spacing:.5px;color:var(--acento)}
details{margin:22px 0 0}
summary{font-size:13px;text-transform:uppercase;letter-spacing:1.5px;color:var(--sub);padding-bottom:6px;margin-bottom:10px;border-bottom:1px solid var(--borde);cursor:pointer;user-select:none;list-style:none}
summary::-webkit-details-marker{display:none}summary::before{content:"▸";display:inline-block;width:16px;transition:transform .15s;transform-origin:4px 55%}details[open]>summary::before{transform:rotate(90deg)}
summary:hover{color:var(--txt)}.cuenta{margin-left:8px;letter-spacing:0;text-transform:none;opacity:.7}
.grid{display:grid;gap:14px;grid-template-columns:repeat(auto-fit,minmax(min(100%,460px),1fr))}
.card{background:var(--card);border:1px solid var(--borde);border-left:4px solid var(--acento);border-radius:10px;padding:14px}
#clientes .card{background:var(--cli);border-color:var(--cli-borde);border-left-color:var(--cli-acento)}
.top{display:flex;align-items:center;gap:10px}.dot{width:12px;height:12px;border-radius:50%;flex:none}
.nom{font-weight:600;flex:1}.cmd{color:var(--sub);font:12px ui-monospace,monospace;margin:6px 0 10px}
.fila{display:flex;gap:8px;align-items:center;flex-wrap:wrap}
button{background:#2b241b;color:var(--txt);border:1px solid var(--borde);border-radius:6px;padding:6px 14px;cursor:pointer}
button:hover:not(:disabled){border-color:var(--acento)}button:disabled{opacity:.4;cursor:default}
.abrir{border-color:var(--cli-acento);color:var(--cli-acento)}.abrir:not(.on){display:none}
label{color:var(--sub);font-size:12px;margin-left:auto}
input{width:76px;background:#0d0b08;color:var(--txt);border:1px solid var(--borde);border-radius:6px;padding:5px 6px;font:inherit}
input:disabled{opacity:.5}
pre{background:#0d0b08;border-radius:6px;padding:8px;height:clamp(220px,42vh,720px);min-height:120px;resize:vertical;overflow:auto;font-size:11px;margin:10px 0 0;white-space:pre-wrap;color:var(--sub)}
.est{font-size:12px;color:var(--sub)}.aviso{color:var(--wait);font-size:12px;margin:0 0 8px}.aviso:empty{display:none}.nota{color:var(--sub);font-size:12px;margin-top:22px}
</style></head><body><h1>⚒ Panel de arranque — BronzeAge</h1>
<details open data-seccion="backend"><summary>Servicios<span class="cuenta"></span></summary><div class="grid" id="backend"></div></details>
<details open data-seccion="tareas"><summary>Tareas<span class="cuenta"></span></summary><div class="grid" id="tareas"></div></details>
<details open data-seccion="clientes"><summary>Clientes<span class="cuenta"></span></summary><div class="grid" id="clientes"></div></details>
<p class="nota">La carga, la auditoría y las pruebas se miran en el admin web (ámbito ADMIN del contrato 05).</p>
<script>
// Cada sección se pliega y se acuerda de cómo la dejaste (solo en este navegador).
for(const d of document.querySelectorAll('details[data-seccion]')){
  const k='panel.plegada.'+d.dataset.seccion;
  try{if(localStorage.getItem(k)==='1')d.open=false}catch{}
  d.ontoggle=()=>{try{localStorage.setItem(k,d.open?'0':'1')}catch{}};
}
async function post(u){await fetch(u,{method:'POST'});setTimeout(pintar,300)}
async function pintar(){
  const datos=await (await fetch('/api/estado')).json();
  for(const d of datos){
    const n=encodeURIComponent(d.nombre);
    let c=document.getElementById(d.nombre);
    if(!c){c=document.createElement('div');c.className='card';c.id=d.nombre;
      document.getElementById(d.tipo==='cliente'?'clientes':d.tipo==='tarea'?'tareas':'backend').append(c);
      c.innerHTML='<div class="top"><span class="dot"></span><span class="nom"></span><span class="est"></span></div><div class="cmd"></div><div class="aviso"></div>'
        +'<div class="fila"><button data-a="arrancar">Arrancar</button><button data-a="parar">Parar</button><button class="abrir">Abrir ↗</button>'
        +'<label>puerto <input type="number" min="1" max="65535"></label></div><pre></pre>';
      c.querySelectorAll('[data-a]').forEach(b=>b.onclick=()=>post('/api/'+b.dataset.a+'/'+n));
      c.querySelector('.abrir').onclick=()=>window.open('http://localhost:'+c.querySelector('input').value,'_blank');
      c.querySelector('input').onchange=e=>post('/api/puerto/'+n+'/'+e.target.value);}
    c.querySelector('.dot').style.background=d.escucha?'var(--on)':d.lanzado?'var(--wait)':'var(--off)';
    c.querySelector('.nom').textContent=d.nombre;
    c.querySelector('.est').textContent=d.falta?'sin carpeta':d.tipo==='tarea'?(d.lanzado?'en marcha…':'parada'):d.aviso&&!d.lanzado?'no puede arrancar':d.escucha?(d.lanzado?'encendido':'encendido (externo)'):d.lanzado?'arrancando…':'apagado';
    c.querySelector('.cmd').textContent=d.comando;
    c.querySelector('.aviso').textContent=d.aviso??'';
    c.querySelector('label').style.display=d.tipo==='tarea'?'none':'';
    const inp=c.querySelector('input');
    if(document.activeElement!==inp)inp.value=d.puerto;
    inp.disabled=d.lanzado; // el puerto se fija al arrancar
    c.querySelector('.abrir').classList.toggle('on',d.tipo==='cliente'&&d.escucha);
    c.querySelector('[data-a=arrancar]').disabled=d.falta||!!d.aviso||d.lanzado||d.escucha;
    c.querySelector('[data-a=parar]').disabled=!d.lanzado;
    const pre=c.querySelector('pre'),abajo=pre.scrollTop+pre.clientHeight>=pre.scrollHeight-4;
    pre.textContent=d.log.join('\\n');if(abajo)pre.scrollTop=pre.scrollHeight;
  }
  // En el título de cada sección, cuántos están en marcha: se ve aunque esté plegada.
  for(const g of document.querySelectorAll('details[data-seccion]')){
    const de=datos.filter(d=>(d.tipo==='cliente'?'clientes':d.tipo==='tarea'?'tareas':'backend')===g.dataset.seccion);
    g.querySelector('.cuenta').textContent=de.length?'· '+de.filter(d=>d.escucha||d.lanzado).length+'/'+de.length+' en marcha':'';
  }
}
pintar();setInterval(pintar,2000);
</script></body></html>`;

// Para el servidor si está en marcha y espera a que termine. ¿Estaba en marcha?
async function pararServidor() {
  const p = procesos['bronzeage-servidor'];
  if (!p?.hijo) return false;
  parar('bronzeage-servidor');
  await p.fin;
  return true;
}
// Si el servidor se lanzó y, si no pudo, por qué (falta el artefacto de mundo, por ejemplo).
const arranque = () => ({ arrancado: enMarcha(), aviso: SERVICIOS['bronzeage-servidor'].aviso(servidor()) });
const servidor = () => configurations.find((c) => c.name === 'bronzeage-servidor');
const enMarcha = () => !!procesos['bronzeage-servidor']?.hijo;

// La API de partidas. Devuelve [estado, cuerpo].
async function apiPartidas(metodo, partes, cuerpo) {
  const [, nombre, accion] = partes; // partidas / <nombre> / <accion>
  if (!nombre) {
    if (metodo === 'GET') return [200, { actual: partidaActual(), servidor: { lanzado: enMarcha(), escucha: await escucha(puertos['bronzeage-servidor']) },
      partidas: nombresDePartida().map(infoPartida), artefactos: artefactos() }];
    if (metodo !== 'POST') return [405, { error: 'metodo_no_permitido' }];
    const n = String(cuerpo?.nombre ?? '').trim().toLowerCase();
    if (!NOMBRE.test(n) || n === 'datos') return [400, { error: 'datos_invalidos', mensaje: 'nombre: de 1 a 40 letras minúsculas, cifras, «_» y «-»' }];
    if (nombresDePartida().includes(n)) return [409, { error: 'partida_repetida', mensaje: `ya hay una partida ${n}` }];
    const elegido = cuerpo?.mundo ? artefactos().find((a) => a.archivo === cuerpo.mundo || a.hash === cuerpo.mundo) : artefactos()[0];
    if (!elegido) return [409, { error: 'sin_mundo', mensaje: 'no hay ese artefacto de mundo: genera uno con POST /api/mundo' }];
    const semilla = Number(cuerpo?.semilla ?? 42);
    if (!Number.isSafeInteger(semilla) || semilla < 0) return [400, { error: 'datos_invalidos', mensaje: 'la semilla es un entero ≥ 0' }];
    mkdirSync(path.join(PARTIDAS, n), { recursive: true });
    writeFileSync(path.join(PARTIDAS, n, 'partida.json'), JSON.stringify({ nombre: n, semilla, mundo: elegido.archivo, creada_ms: Date.now() }, null, 2));
    if (cuerpo?.activar !== false) { await pararServidor(); ponerActual(n); arrancar(servidor()); }
    return [201, { ...infoPartida(n), ...arranque() }];
  }
  if (!nombresDePartida().includes(nombre)) return [404, { error: 'no_encontrada', mensaje: `no hay partida ${nombre}` }];
  if (!accion) {
    if (metodo === 'GET') return [200, { ...infoPartida(nombre), lista_respaldos: respaldosDe(nombre) }];
    if (metodo !== 'DELETE') return [405, { error: 'metodo_no_permitido' }];
    if (nombre === partidaActual() && enMarcha()) return [409, { error: 'partida_en_marcha', mensaje: 'para el servidor o cambia de partida antes de borrarla' }];
    rmSync(carpetaPartida(nombre), { recursive: true, force: true });
    return [200, { borrada: nombre, actual: partidaActual() }];
  }
  if (metodo !== 'POST' && !(metodo === 'GET' && accion === 'respaldos')) return [405, { error: 'metodo_no_permitido' }];
  if (accion === 'respaldos') return [200, { respaldos: respaldosDe(nombre) }];
  if (accion === 'activar') {
    await pararServidor(); ponerActual(nombre); arrancar(servidor());
    return [200, { ...infoPartida(nombre), ...arranque() }];
  }
  if (accion === 'restaurar') {
    const r = respaldosDe(nombre).find((x) => x.id === cuerpo?.respaldo);
    if (!r) return [404, { error: 'no_encontrado', mensaje: 'no hay ese respaldo' }];
    const carpeta = carpetaPartida(nombre), copia = path.join(carpeta, 'respaldos', r.id);
    const seguia = nombre === partidaActual() && (await pararServidor());
    // Antes de pisarla, lo de ahora se guarda como un respaldo más (sin la versión: no la sabe sin el servidor).
    const antes = `antes-de-restaurar-${Date.now()}`;
    for (const sub of ['mundo', 'acceso']) if (existsSync(path.join(carpeta, sub))) cpSync(path.join(carpeta, sub), path.join(carpeta, 'respaldos', antes, sub), { recursive: true });
    writeFileSync(path.join(carpeta, 'respaldos', antes, 'respaldo.json'), JSON.stringify({ id: antes, creado_ms: Date.now(), version: null, instante: null, nota: `lo que había antes de restaurar ${r.id}` }));
    for (const sub of ['mundo', 'acceso']) {
      rmSync(path.join(carpeta, sub), { recursive: true, force: true });
      if (existsSync(path.join(copia, sub))) cpSync(path.join(copia, sub), path.join(carpeta, sub), { recursive: true });
    }
    if (seguia) arrancar(servidor());
    return [200, { restaurado: r, antes, ...(seguia ? arranque() : { arrancado: false, aviso: null }) }];
  }
  return [404, { error: 'no_encontrado', mensaje: 'acción desconocida' }];
}

// La regeneración del mundo: GET dice si está en marcha y cómo acabó la última; POST { semilla } la lanza.
async function apiMundo(metodo, cuerpo) {
  const tarea = configurations.find((c) => c.name === 'generar-mundo');
  if (metodo === 'GET') return [200, { generando: !!procesos['generar-mundo']?.hijo, semilla: mundo.semilla, ultima: mundo.ultima, artefactos: artefactos() }];
  if (metodo !== 'POST') return [405, { error: 'metodo_no_permitido' }];
  if (procesos['generar-mundo']?.hijo) return [409, { error: 'ya_generando', mensaje: 'ya se está generando un mundo' }];
  const semilla = Number(cuerpo?.semilla ?? 42);
  if (!Number.isSafeInteger(semilla) || semilla < 0) return [400, { error: 'datos_invalidos', mensaje: 'la semilla es un entero ≥ 0' }];
  mundo.semilla = semilla;
  arrancar(tarea);
  return [202, { generando: true, semilla }];
}

const CORS = { 'access-control-allow-origin': '*', 'access-control-allow-methods': 'GET, POST, PUT, DELETE, OPTIONS', 'access-control-allow-headers': 'content-type, authorization' };
const leerCuerpo = (req) => new Promise((ok) => { let b = ''; req.on('data', (d) => { b += d; }); req.on('end', () => { try { ok(b ? JSON.parse(b) : null); } catch { ok(undefined); } }); });

http.createServer(async (req, res) => {
  const partes = new URL(req.url, 'http://x').pathname.split('/').filter(Boolean).map(decodeURIComponent);
  if (partes[0] !== 'api') { res.writeHead(200, { 'content-type': 'text/html; charset=utf-8' }); return res.end(HTML); }
  if (req.method === 'OPTIONS') { res.writeHead(204, CORS); return res.end(); }
  const json = (estado, v) => { res.writeHead(estado, { ...CORS, 'content-type': 'application/json' }); res.end(JSON.stringify(v)); };
  if (partes[1] === 'partidas' || partes[1] === 'mundo') {
    const cuerpo = await leerCuerpo(req);
    if (cuerpo === undefined) return json(400, { error: 'datos_invalidos', mensaje: 'el cuerpo es JSON' });
    try { return json(...(partes[1] === 'mundo' ? await apiMundo(req.method, cuerpo) : await apiPartidas(req.method, partes.slice(1), cuerpo))); }
    catch (e) { return json(500, { error: 'interno', mensaje: String(e?.message ?? e) }); }
  }
  const [, accion, nombre, valor] = partes;
  if (accion === 'estado') return json(200, await estado());
  const c = configurations.find((x) => x.name === (nombre ?? ''));
  if (req.method !== 'POST' || !c) { res.writeHead(404, CORS); return res.end(); }
  if (accion === 'arrancar') arrancar(c);
  else if (accion === 'parar') parar(c.name);
  else if (accion === 'puerto') {
    const p = Number(valor);
    if (!Number.isInteger(p) || p < 1 || p > 65535 || procesos[c.name]?.hijo) { res.writeHead(400, CORS); return res.end(); }
    puertos[c.name] = p;
  }
  res.writeHead(204, CORS); res.end();
}).listen(PUERTO, '127.0.0.1', () => {
  const url = `http://localhost:${PUERTO}`;
  console.log(`Panel en ${url}`);
  if (!process.env.PANEL_SIN_NAVEGADOR) exec(process.platform === 'win32' ? `start "" ${url}` : `open ${url}`);
});

// Al cerrar el panel se llevan por delante los servicios que lanzó.
for (const s of ['SIGINT', 'SIGTERM']) process.on(s, () => { Object.keys(procesos).forEach(parar); setTimeout(() => process.exit(), 500); });
