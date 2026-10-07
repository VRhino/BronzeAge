// Panel de control local: arranca/para los servicios de .claude/launch.json y muestra si escuchan.
// Uso: npm run panel  (abre http://localhost:3999)
import http from 'node:http';
import net from 'node:net';
import { spawn, exec } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const RAIZ = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const PUERTO = Number(process.env.PANEL_PUERTO ?? 3999);
const { configurations } = JSON.parse(readFileSync(path.join(RAIZ, '.claude/launch.json'), 'utf8'));

// Cómo le dice cada servicio su puerto (y el de los que consume). P = puertos elegidos ahora mismo.
const VITE = (p) => ['--', '--port', p, '--strictPort'];
const SERVICIOS = {
  'bronze-age-server': { tipo: 'backend', con: (p) => ({ env: { PORT: p } }) },
  'bronze-age-bots': { tipo: 'backend', con: (p, P) => ({ env: { PORT: p, BOTS_SERVIDOR: `http://localhost:${P['bronze-age-server']}` } }) },
  'bronze-age-cliente': { tipo: 'cliente', con: (p, P) => ({ args: VITE(p), env: {
    BACKEND_URL: `http://localhost:${P['bronze-age-server']}`, BOTS_URL: `ws://localhost:${P['bronze-age-bots']}/control` } }) },
  'bronze-age-cliente-jugador': { tipo: 'cliente', con: (p, P) => ({ args: VITE(p), env: { BACKEND_URL: `http://localhost:${P['bronze-age-server']}` } }) },
  'lab-trazado': { tipo: 'cliente', con: (p) => ({ env: { LAB_PORT: p } }) },
};

const puertos = Object.fromEntries(configurations.map((c) => [c.name, c.port]));
const procesos = {}; // nombre -> { hijo, log[] }

function escucha(puerto) {
  return new Promise((ok) => {
    // 'localhost' + autoSelectFamily: Vite escucha solo en ::1, el backend en 0.0.0.0.
    const s = net.connect({ port: puerto, host: 'localhost', autoSelectFamily: true });
    s.once('connect', () => { s.destroy(); ok(true); });
    s.once('error', () => ok(false));
    s.setTimeout(400, () => { s.destroy(); ok(false); });
  });
}

function arrancar(c) {
  if (procesos[c.name]?.hijo) return;
  const P = Object.fromEntries(Object.entries(puertos).map(([k, v]) => [k, String(v)]));
  const { args = [], env = {} } = SERVICIOS[c.name]?.con(P[c.name], P) ?? {};
  const p = { log: [] };
  const hijo = spawn(c.runtimeExecutable, [...c.runtimeArgs, ...args], {
    cwd: RAIZ, shell: true, env: { ...process.env, ...c.env, ...env, FORCE_COLOR: '0' },
  });
  const anotar = (b) => {
    p.log.push(...b.toString().replace(/\x1b\[[0-9;]*m/g, '').split(/\r?\n/).filter(Boolean));
    p.log.splice(0, Math.max(0, p.log.length - 200));
  };
  hijo.stdout.on('data', anotar);
  hijo.stderr.on('data', anotar);
  hijo.on('exit', (code) => { p.hijo = null; p.log.push(`— proceso terminado (código ${code}) —`); });
  p.hijo = hijo;
  procesos[c.name] = p;
}

function parar(nombre) {
  const h = procesos[nombre]?.hijo;
  if (!h) return;
  // shell:true en Windows deja npm/node como nietos: hay que matar el árbol.
  if (process.platform === 'win32') exec(`taskkill /pid ${h.pid} /T /F`);
  else h.kill('SIGTERM');
}

async function estado() {
  return Promise.all(configurations.map(async (c) => {
    const p = procesos[c.name];
    return {
      nombre: c.name, tipo: SERVICIOS[c.name]?.tipo ?? 'backend', puerto: puertos[c.name],
      comando: [c.runtimeExecutable, ...c.runtimeArgs].join(' '),
      escucha: await escucha(puertos[c.name]), lanzado: !!p?.hijo, log: p?.log.slice(-60) ?? [],
    };
  }));
}

const HTML = `<!doctype html><html lang="es"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1"><title>Panel BronzeAge</title>
<style>
:root{--bg:#14110d;--card:#1f1a14;--borde:#3a2f22;--txt:#eadfcc;--sub:#9c8b72;--on:#5fbf6a;--off:#c4553f;--wait:#d9a63c;--acento:#c98a3b;--cli:#1a1d20;--cli-borde:#2c3640;--cli-acento:#6fa8c9}
*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--txt);font:14px/1.4 system-ui,sans-serif;padding:24px 16px}
h1{font-size:18px;margin:0 0 18px;letter-spacing:.5px;color:var(--acento)}
h2{font-size:13px;text-transform:uppercase;letter-spacing:1.5px;color:var(--sub);margin:26px 0 10px;padding-bottom:6px;border-bottom:1px solid var(--borde);max-width:1200px}
.grid{display:grid;gap:14px;grid-template-columns:repeat(auto-fill,minmax(320px,1fr));max-width:1200px}
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
pre{background:#0d0b08;border-radius:6px;padding:8px;height:140px;overflow:auto;font-size:11px;margin:10px 0 0;white-space:pre-wrap;color:var(--sub)}
.est{font-size:12px;color:var(--sub)}
</style></head><body><h1>⚒ Panel de control — BronzeAge</h1>
<h2>Backends</h2><div class="grid" id="backend"></div>
<h2>Clientes</h2><div class="grid" id="clientes"></div>
<script>
async function post(u){await fetch(u,{method:'POST'});setTimeout(pintar,300)}
async function pintar(){
  const datos=await (await fetch('/api/estado')).json();
  for(const d of datos){
    const n=encodeURIComponent(d.nombre);
    let c=document.getElementById(d.nombre);
    if(!c){c=document.createElement('div');c.className='card';c.id=d.nombre;
      document.getElementById(d.tipo==='cliente'?'clientes':'backend').append(c);
      c.innerHTML='<div class="top"><span class="dot"></span><span class="nom"></span><span class="est"></span></div><div class="cmd"></div>'
        +'<div class="fila"><button data-a="arrancar">Arrancar</button><button data-a="parar">Parar</button><button class="abrir">Abrir ↗</button>'
        +'<label>puerto <input type="number" min="1" max="65535"></label></div><pre></pre>';
      c.querySelectorAll('[data-a]').forEach(b=>b.onclick=()=>post('/api/'+b.dataset.a+'/'+n));
      c.querySelector('.abrir').onclick=()=>window.open('http://localhost:'+c.querySelector('input').value,'_blank');
      c.querySelector('input').onchange=e=>post('/api/puerto/'+n+'/'+e.target.value);}
    c.querySelector('.dot').style.background=d.escucha?'var(--on)':d.lanzado?'var(--wait)':'var(--off)';
    c.querySelector('.nom').textContent=d.nombre;
    c.querySelector('.est').textContent=d.escucha?(d.lanzado?'encendido':'encendido (externo)'):d.lanzado?'arrancando…':'apagado';
    c.querySelector('.cmd').textContent=d.comando;
    const inp=c.querySelector('input');
    if(document.activeElement!==inp)inp.value=d.puerto;
    inp.disabled=d.lanzado; // el puerto se fija al arrancar
    c.querySelector('.abrir').classList.toggle('on',d.tipo==='cliente'&&d.escucha);
    c.querySelector('[data-a=arrancar]').disabled=d.lanzado||d.escucha;
    c.querySelector('[data-a=parar]').disabled=!d.lanzado;
    const pre=c.querySelector('pre'),abajo=pre.scrollTop+pre.clientHeight>=pre.scrollHeight-4;
    pre.textContent=d.log.join('\\n');if(abajo)pre.scrollTop=pre.scrollHeight;
  }
}
pintar();setInterval(pintar,2000);
</script></body></html>`;

http.createServer(async (req, res) => {
  const [, api, accion, nombre, valor] = req.url.split('/');
  if (api !== 'api') { res.writeHead(200, { 'content-type': 'text/html; charset=utf-8' }); return res.end(HTML); }
  if (accion === 'estado') { res.writeHead(200, { 'content-type': 'application/json' }); return res.end(JSON.stringify(await estado())); }
  const c = configurations.find((x) => x.name === decodeURIComponent(nombre ?? ''));
  if (req.method !== 'POST' || !c) { res.writeHead(404); return res.end(); }
  if (accion === 'arrancar') arrancar(c);
  else if (accion === 'parar') parar(c.name);
  else if (accion === 'puerto') {
    const p = Number(valor);
    if (!Number.isInteger(p) || p < 1 || p > 65535 || procesos[c.name]?.hijo) { res.writeHead(400); return res.end(); }
    puertos[c.name] = p;
  }
  res.writeHead(204); res.end();
}).listen(PUERTO, '127.0.0.1', () => {
  const url = `http://localhost:${PUERTO}`;
  console.log(`Panel en ${url}`);
  exec(process.platform === 'win32' ? `start "" ${url}` : `open ${url}`);
});

// Al cerrar el panel se llevan por delante los servicios que lanzó.
for (const s of ['SIGINT', 'SIGTERM']) process.on(s, () => { Object.keys(procesos).forEach(parar); setTimeout(() => process.exit(), 500); });
