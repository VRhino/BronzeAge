// Panel de arranque local (fase 7): enciende y para los servicios de .claude/launch.json, dice si escuchan y enseña sus logs.
// Uso: `node scripts/panel.mjs` o `panel.cmd` (abre http://localhost:3999). Solo Node, sin dependencias.
//
// Viene del panel del TypeScript (rama typescript-final, scripts/panel.mjs), sin su parte de carga y pruebas (telemetría, serie
// de carga, auditoría e informe): en Rust eso llega por el ámbito ADMIN del contrato 05 y es del admin web (tanda 7H).
//
// Partidas (fase 7, para el admin web): cada partida es una carpeta de datos del servidor en `partidas/<nombre>/` (`mundo/`, `acceso/`, `respaldos/` y
// `partida.json` con su semilla y su artefacto de mundo); la carpeta `datos/` de antes cuenta como la partida «datos». El servidor arranca con la partida
// actual. La API de partidas está más abajo (`/api/partidas`, `/api/mundo`), con CORS abierto. Desde el 2026-10-27 (decisión del autor) las partidas y el mundo se
// manejan desde la pestaña «Mundo y partidas» del propio panel, que funciona con el servidor apagado; el admin web ya no las lleva.
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
  if (!partidaActual()) return 'no hay partida activa: crea una o activa una de la lista (POST /api/partidas o /api/partidas/<nombre>/activar)';
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
// La partida activa: la de `actual.txt`. Ese archivo vacío es «sin partida activa» (tras apagarla o borrarla); sin el archivo, la primera que haya.
function partidaActual() {
  if (!existsSync(ACTUAL)) return nombresDePartida()[0] ?? null;
  const a = readFileSync(ACTUAL, 'utf8').trim();
  return nombresDePartida().includes(a) ? a : null;
}
function ponerActual(n) { mkdirSync(PARTIDAS, { recursive: true }); writeFileSync(ACTUAL, n ?? ''); }
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
    administradores: meta.administradores ?? listaAdmins(servidor().env?.ADMINISTRADORES),
    respaldos: respaldosDe(n).length, actual: n === partidaActual(), en_marcha: n === partidaActual() && enMarcha() };
}
// Los nicks de administrador como lista, desde texto «a, b» o desde una lista.
const listaAdmins = (v) => (Array.isArray(v) ? v : String(v ?? '').split(',')).map((x) => String(x).trim()).filter(Boolean);
// El entorno del servidor para la partida actual: su carpeta, su semilla, su artefacto (si lo fijó al crearla) y sus administradores.
function envPartida() {
  const n = partidaActual();
  if (!n) return {};
  const i = infoPartida(n);
  return { BRONZEAGE_DATOS: i.carpeta, BRONZEAGE_SEMILLA: String(i.semilla), ADMINISTRADORES: i.administradores.join(','), ...(i.mundo ? { BRONZEAGE_MUNDO: path.join(MUNDOS, i.mundo) } : {}) };
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
        + (c.name === 'bronzeage-servidor' ? `  [partida ${partidaActual() ?? 'ninguna'}]` : ''),
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
.pestanas{display:flex;gap:6px;margin:0 0 18px;border-bottom:1px solid var(--borde)}
.pestanas button{border-radius:8px 8px 0 0;border-bottom:none;padding:8px 18px;background:transparent;color:var(--sub)}
.pestanas button.on{background:var(--card);color:var(--txt);border-color:var(--acento)}
.vista{display:none}.vista.on{display:block}
table{width:100%;border-collapse:collapse;font-size:13px}th,td{text-align:left;padding:6px 8px;border-bottom:1px solid var(--borde);vertical-align:top}
th{color:var(--sub);font-weight:500;font-size:12px}td .fila{gap:6px}td button{padding:4px 10px;font-size:12px}
.form{display:flex;gap:10px;flex-wrap:wrap;align-items:end}.form label{margin:0;display:flex;flex-direction:column;gap:4px}
.form input,.form select{width:auto;min-width:90px;background:#0d0b08;color:var(--txt);border:1px solid var(--borde);border-radius:6px;padding:5px 6px;font:inherit}
.msg{font-size:12px;margin-top:10px;min-height:16px}.msg.error{color:var(--off)}.msg.ok{color:var(--on)}
.activa{color:var(--on);font-weight:600}.peligro{border-color:var(--off);color:var(--off)}
.respaldos td{font-size:12px;color:var(--sub)}
.est{font-size:12px;color:var(--sub)}.aviso{color:var(--wait);font-size:12px;margin:0 0 8px}.aviso:empty{display:none}.nota{color:var(--sub);font-size:12px;margin-top:22px}
</style></head><body><h1>⚒ Panel de arranque — BronzeAge</h1>
<nav class="pestanas"><button data-vista="partidas">Mundo y partidas</button><button data-vista="servicios">Servicios</button></nav>
<section class="vista" id="v-partidas">
<details open data-seccion="partidas"><summary>Partidas<span class="cuenta" id="p-cuenta"></span></summary>
<div class="card"><table><thead><tr><th>Partida</th><th>Semilla</th><th>Mundo</th><th>Creada</th><th>Administradores</th><th>Estado</th><th></th></tr></thead><tbody id="p-lista"></tbody></table>
<div class="msg" id="p-msg"></div></div></details>
<details open data-seccion="nueva"><summary>Nueva partida</summary>
<div class="card"><div class="form"><label>Nombre<input id="n-nombre" placeholder="misiones" maxlength="40"></label>
<label>Semilla<input id="n-semilla" type="number" min="0" value="42"></label>
<label>Mundo<select id="n-mundo"></select></label>
<label>Administradores<input id="n-admins" placeholder="tu nick, otro"></label>
<label style="flex-direction:row;align-items:center"><input id="n-activar" type="checkbox" checked style="width:auto"> activarla y arrancar el servidor</label>
<button id="n-crear">Crear</button></div><div class="msg" id="n-msg"></div></div></details>
<details open data-seccion="mundo"><summary>Mundo</summary>
<div class="card"><div class="form"><label>Semilla<input id="m-semilla" type="number" min="0" value="42"></label><button id="m-generar">Generar mundo (unos 15 min)</button>
<span class="est" id="m-estado"></span></div>
<table style="margin-top:12px"><thead><tr><th>Artefacto</th><th>Creado</th><th>Partidas</th><th></th></tr></thead><tbody id="m-lista"></tbody></table><div class="msg" id="m-msg"></div></div></details>
</section>
<section class="vista" id="v-servicios">
<details open data-seccion="backend"><summary>Servicios<span class="cuenta"></span></summary><div class="grid" id="backend"></div></details>
<details open data-seccion="tareas"><summary>Tareas<span class="cuenta"></span></summary><div class="grid" id="tareas"></div></details>
<details open data-seccion="clientes"><summary>Clientes<span class="cuenta"></span></summary><div class="grid" id="clientes"></div></details>
<p class="nota">La carga, la auditoría y las pruebas se miran en el admin web (ámbito ADMIN del contrato 05).</p>
</section>
<script>
// Cada sección se pliega y se acuerda de cómo la dejaste (solo en este navegador).
for(const d of document.querySelectorAll('details[data-seccion]')){
  const k='panel.plegada.'+d.dataset.seccion;
  try{if(localStorage.getItem(k)==='1')d.open=false}catch{}
  d.ontoggle=()=>{try{localStorage.setItem(k,d.open?'0':'1')}catch{}};
}
async function post(u){await fetch(u,{method:'POST'});setTimeout(pintar,300)}
// Las dos pestañas; se acuerda de la última (solo en este navegador).
function verVista(v){for(const b of document.querySelectorAll('.pestanas button'))b.classList.toggle('on',b.dataset.vista===v);
  for(const s of document.querySelectorAll('.vista'))s.classList.toggle('on',s.id==='v-'+v);try{localStorage.setItem('panel.vista',v)}catch{}}
document.querySelectorAll('.pestanas button').forEach(b=>b.onclick=()=>verVista(b.dataset.vista));
let vista='partidas';try{vista=localStorage.getItem('panel.vista')||vista}catch{}verVista(vista);
// La API de partidas y mundo con JSON; deja el resultado en la línea de mensajes msg.
async function api(metodo,u,cuerpo,msg){
  const el=document.getElementById(msg);el.className='msg';el.textContent='…';
  try{const r=await fetch(u,{method:metodo,headers:{'content-type':'application/json'},body:cuerpo?JSON.stringify(cuerpo):undefined});
    const v=await r.json().catch(()=>({}));
    if(!r.ok){el.className='msg error';el.textContent=v.mensaje||v.error||('error '+r.status);return null}
    el.className='msg ok';el.textContent=v.aviso?('hecho; el servidor no arranca: '+v.aviso):'hecho';pintarPartidas();setTimeout(pintar,300);return v;
  }catch(e){el.className='msg error';el.textContent=String(e);return null}
}
const fecha=ms=>ms?new Date(ms).toLocaleString('es-ES',{dateStyle:'short',timeStyle:'short'}):'—';
const celda=(tr,t,cl)=>{const td=document.createElement('td');td.textContent=t;if(cl)td.className=cl;tr.append(td);return td};
const boton=(td,t,f,cl)=>{const b=document.createElement('button');b.textContent=t;b.onclick=f;if(cl)b.className=cl;td.querySelector('.fila').append(b);return b};
const abiertos=new Set(); // partidas con la lista de respaldos desplegada
async function pintarPartidas(){
  let d,m;try{[d,m]=await Promise.all([(await fetch('/api/partidas')).json(),(await fetch('/api/mundo')).json()])}catch{return}
  const lista=document.getElementById('p-lista');lista.textContent='';
  document.getElementById('p-cuenta').textContent='· '+d.partidas.length+(d.actual?' · activa: '+d.actual:' · ninguna activa');
  for(const p of d.partidas){
    const tr=document.createElement('tr'),n=encodeURIComponent(p.nombre);
    celda(tr,p.nombre,p.actual?'activa':'');celda(tr,String(p.semilla));celda(tr,p.mundo?p.mundo.slice(6,18)+'…':'el último');celda(tr,fecha(p.creada_ms));celda(tr,p.administradores.join(', ')||'ninguno');
    celda(tr,p.actual?(p.en_marcha?'activa · servidor en marcha':'activa · servidor parado'):'guardada');
    const acc=celda(tr,'');acc.innerHTML='<div class="fila"></div>';
    if(!p.actual)boton(acc,'Activar',()=>api('POST','/api/partidas/'+n+'/activar',null,'p-msg'));
    else{if(!p.en_marcha)boton(acc,'Arrancar',()=>api('POST','/api/partidas/'+n+'/activar',null,'p-msg'));
      boton(acc,'Apagar',()=>api('POST','/api/partidas/'+n+'/apagar',null,'p-msg'));}
    boton(acc,'Administradores',()=>{const v=prompt('Nicks de administrador, separados por comas (se reinicia el servidor si está en marcha)',p.administradores.join(', '));
      if(v!==null)api('POST','/api/partidas/'+n+'/administradores',{administradores:v},'p-msg')});
    boton(acc,'Respaldar',()=>api('POST','/api/partidas/'+n+'/respaldar',{nota:prompt('Nota del respaldo (opcional)')??''},'p-msg'));
    boton(acc,'Respaldos ('+p.respaldos+')',()=>{abiertos.has(p.nombre)?abiertos.delete(p.nombre):abiertos.add(p.nombre);pintarPartidas()});
    boton(acc,'Borrar',()=>{if(confirm('¿Borrar la partida «'+p.nombre+'» con sus respaldos?'+(p.actual?' Es la activa: se apaga el servidor.':'')+' No se puede deshacer.'))
      api('DELETE','/api/partidas/'+n,{confirmar:true},'p-msg')},'peligro');
    lista.append(tr);
    if(abiertos.has(p.nombre)){
      const fila=document.createElement('tr');fila.className='respaldos';const td=document.createElement('td');td.colSpan=7;fila.append(td);lista.append(fila);
      const rs=(await (await fetch('/api/partidas/'+n+'/respaldos')).json()).respaldos??[];
      if(!rs.length)td.textContent='Sin respaldos.';
      const t=document.createElement('table');td.append(t);
      for(const r of rs){const f=document.createElement('tr');celda(f,fecha(r.creado_ms));celda(f,r.nota||r.id);
        const a=celda(f,'');a.innerHTML='<div class="fila"></div>';
        boton(a,'Restaurar',()=>{if(confirm('¿Restaurar «'+p.nombre+'» al respaldo de '+fecha(r.creado_ms)+'? Lo de ahora se guarda antes como otro respaldo.'))
          api('POST','/api/partidas/'+n+'/restaurar',{respaldo:r.id},'p-msg')});t.append(f)}
    }
  }
  if(!d.partidas.length){const tr=document.createElement('tr');celda(tr,'No hay partidas: crea una abajo.').colSpan=7;lista.append(tr)}
  const sel=document.getElementById('n-mundo'),antes=sel.value;sel.textContent='';
  for(const a of m.artefactos){const o=document.createElement('option');o.value=a.archivo;o.textContent=a.hash.slice(0,12)+'… ('+fecha(a.creado_ms)+')';sel.append(o)}
  if(antes)sel.value=antes;
  document.getElementById('n-crear').disabled=!m.artefactos.length;
  const ml=document.getElementById('m-lista');ml.textContent='';
  for(const [i,a] of m.artefactos.entries()){const tr=document.createElement('tr');celda(tr,a.archivo);celda(tr,fecha(a.creado_ms));
    const usan=d.partidas.filter(p=>p.mundo===a.archivo||(!p.mundo&&i===0)).map(p=>p.nombre);celda(tr,usan.join(', ')||'ninguna');
    const acc=celda(tr,'');acc.innerHTML='<div class="fila"></div>';
    const b=boton(acc,'Borrar',()=>{if(confirm('¿Borrar el mundo '+a.hash.slice(0,12)+'…? No se puede deshacer: habría que generarlo otra vez (unos 15 min).'))
      api('DELETE','/api/mundo/'+encodeURIComponent(a.archivo),null,'m-msg')},'peligro');
    if(usan.length||m.generando){b.disabled=true;b.title=m.generando?'se está generando un mundo':'lo usa'+(usan.length>1?'n las partidas ':' la partida ')+usan.join(', ')+(usan.length>1?': bórralas antes':': bórrala antes')}
    ml.append(tr)}
  if(!m.artefactos.length){const tr=document.createElement('tr');celda(tr,'No hay ningún artefacto de mundo: genera uno.').colSpan=4;ml.append(tr)}
  document.getElementById('m-generar').disabled=m.generando;
  document.getElementById('m-estado').textContent=m.generando?'generando con la semilla '+m.semilla+'…':m.ultima?('último: semilla '+m.ultima.semilla+(m.ultima.codigo===0?', bien':', falló (código '+m.ultima.codigo+'; mira su log en Servicios)')):'';
}
document.getElementById('n-crear').onclick=()=>api('POST','/api/partidas',{nombre:document.getElementById('n-nombre').value,semilla:Number(document.getElementById('n-semilla').value),
  mundo:document.getElementById('n-mundo').value||undefined,activar:document.getElementById('n-activar').checked,
  ...(document.getElementById('n-admins').value.trim()?{administradores:document.getElementById('n-admins').value}:{})},'n-msg');
document.getElementById('m-generar').onclick=()=>api('POST','/api/mundo',{semilla:Number(document.getElementById('m-semilla').value)},'m-msg');
pintarPartidas();setInterval(pintarPartidas,4000);
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
  for(const g of document.querySelectorAll('#v-servicios details[data-seccion]')){
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
    const administradores = cuerpo?.administradores === undefined ? listaAdmins(servidor().env?.ADMINISTRADORES) : listaAdmins(cuerpo.administradores);
    mkdirSync(path.join(PARTIDAS, n), { recursive: true });
    writeFileSync(path.join(PARTIDAS, n, 'partida.json'), JSON.stringify({ nombre: n, semilla, mundo: elegido.archivo, creada_ms: Date.now(), administradores }, null, 2));
    if (cuerpo?.activar !== false) { await pararServidor(); ponerActual(n); arrancar(servidor()); }
    return [201, { ...infoPartida(n), ...arranque() }];
  }
  if (!nombresDePartida().includes(nombre)) return [404, { error: 'no_encontrada', mensaje: `no hay partida ${nombre}` }];
  if (!accion) {
    if (metodo === 'GET') return [200, { ...infoPartida(nombre), lista_respaldos: respaldosDe(nombre) }];
    if (metodo !== 'DELETE') return [405, { error: 'metodo_no_permitido' }];
    // La activa se borra solo con `confirmar: true`: se apaga el servidor (con su foto, que se borra con ella) y queda sin partida activa.
    const activa = nombre === partidaActual();
    if (activa && cuerpo?.confirmar !== true) return [409, { error: 'partida_activa', mensaje: 'es la partida activa: borrarla apaga el servidor; repite con { "confirmar": true }' }];
    if (activa) { await pararServidor(); ponerActual(null); }
    rmSync(carpetaPartida(nombre), { recursive: true, force: true });
    return [200, { borrada: nombre, actual: partidaActual() }];
  }
  if (metodo !== 'POST' && !(metodo === 'GET' && accion === 'respaldos')) return [405, { error: 'metodo_no_permitido' }];
  if (accion === 'respaldos') return [200, { respaldos: respaldosDe(nombre) }];
  if (accion === 'apagar') {
    // Apaga el servidor si sirve esta partida (con su foto) y deja el panel sin partida activa; la partida queda guardada para activarla después.
    if (nombre !== partidaActual()) return [409, { error: 'no_es_la_activa', mensaje: `${nombre} no es la partida activa` }];
    const estaba = await pararServidor();
    ponerActual(null);
    return [200, { apagada: nombre, estaba_en_marcha: estaba, actual: null }];
  }
  if (accion === 'respaldar') {
    // Copia `mundo/` y `acceso/` a `respaldos/<id>`. Si el servidor sirve esta partida, se para (guarda su foto), se copia y se vuelve a arrancar.
    const carpeta = carpetaPartida(nombre);
    const seguia = nombre === partidaActual() && (await pararServidor());
    const id = `panel-${Date.now()}`;
    for (const sub of ['mundo', 'acceso']) if (existsSync(path.join(carpeta, sub))) cpSync(path.join(carpeta, sub), path.join(carpeta, 'respaldos', id, sub), { recursive: true });
    mkdirSync(path.join(carpeta, 'respaldos', id), { recursive: true });
    const r = { id, creado_ms: Date.now(), version: null, instante: null, nota: String(cuerpo?.nota ?? '').slice(0, 200) || 'desde el panel' };
    writeFileSync(path.join(carpeta, 'respaldos', id, 'respaldo.json'), JSON.stringify(r));
    if (seguia) arrancar(servidor());
    return [201, { respaldo: r, ...(seguia ? arranque() : { arrancado: false, aviso: null }) }];
  }
  if (accion === 'administradores') {
    // Cambia los nicks de administrador de la partida (los que pueden abrir el admin web). Si el servidor la sirve, se reinicia para aplicarlo.
    const fichero = path.join(carpetaPartida(nombre), 'partida.json');
    const meta = { ...(leerJson(fichero) ?? { nombre, semilla: infoPartida(nombre).semilla }), administradores: listaAdmins(cuerpo?.administradores) };
    const seguia = nombre === partidaActual() && (await pararServidor());
    writeFileSync(fichero, JSON.stringify(meta, null, 2));
    if (seguia) arrancar(servidor());
    return [200, { ...infoPartida(nombre), ...(seguia ? arranque() : { arrancado: false, aviso: null }) }];
  }
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

// La regeneración del mundo: GET dice si está en marcha y cómo acabó la última; POST { semilla } la lanza; DELETE /api/mundo/<archivo> borra un artefacto.
async function apiMundo(metodo, cuerpo, archivo) {
  const tarea = configurations.find((c) => c.name === 'generar-mundo');
  if (archivo) {
    if (metodo !== 'DELETE') return [405, { error: 'metodo_no_permitido' }];
    const a = artefactos().find((x) => x.archivo === archivo || x.hash === archivo);
    if (!a) return [404, { error: 'no_encontrado', mensaje: 'no hay ese artefacto de mundo' }];
    if (procesos['generar-mundo']?.hijo) return [409, { error: 'ya_generando', mensaje: 'se está generando un mundo: espera a que acabe' }];
    // No se borra el mundo de una partida: el de las que lo fijaron al crearlas y, si es el más nuevo, el de las que usan «el último».
    const usan = nombresDePartida().map(infoPartida).filter((p) => p.mundo === a.archivo || (!p.mundo && artefactos()[0]?.archivo === a.archivo)).map((p) => p.nombre);
    if (usan.length) return [409, { error: 'mundo_en_uso', mensaje: `lo usa${usan.length > 1 ? 'n las partidas' : ' la partida'} ${usan.join(', ')}: ${usan.length > 1 ? 'bórralas' : 'bórrala'} antes`, partidas: usan }];
    rmSync(path.join(MUNDOS, a.archivo), { force: true });
    return [200, { borrado: a.archivo, artefactos: artefactos() }];
  }
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
    try { return json(...(partes[1] === 'mundo' ? await apiMundo(req.method, cuerpo, partes[2]) : await apiPartidas(req.method, partes.slice(1), cuerpo))); }
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
