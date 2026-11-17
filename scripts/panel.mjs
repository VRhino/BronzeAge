// Panel de arranque local (fase 7): enciende y para los servicios de .claude/launch.json, dice si escuchan y enseña sus logs.
// Uso: `node scripts/panel.mjs` o `panel.cmd` (abre http://localhost:3999). Solo Node, sin dependencias.
//
// Viene del panel del TypeScript (rama typescript-final, scripts/panel.mjs), sin su parte de carga y pruebas (telemetría, serie
// de carga, auditoría e informe): en Rust eso llega por el ámbito ADMIN del contrato 05 y es del admin web (tanda 7H).
//
// Partidas (fase 7, para el admin web): cada partida es una carpeta de datos del servidor en `partidas/<nombre>/` (`mundo/`, `acceso/`, `respaldos/` y
// `partida.json` con su semilla y su artefacto de mundo); la carpeta `datos/` de antes cuenta como la partida «datos». El servidor arranca con la partida
// actual. La API de partidas está más abajo (`/api/partidas`, `/api/mundo`); solo contesta a esta máquina (ver `procedencia`). Desde el 2026-10-27 (decisión del autor) las partidas y el mundo se
// manejan desde la pestaña «Mundo y partidas» del propio panel, que funciona con el servidor apagado; el admin web ya no las lleva.
//
// Cada servicio puede llevar `cwd` (relativo a la raíz del motor): el motor es esta carpeta y los repos hermanos están al lado
// (../BronceAgeBotsBrain, ../BronceAgeAdminClient, ../BronzeAgeClient). Si la carpeta no existe, la tarjeta lo dice y no arranca.
import http from 'node:http';
import net from 'node:net';
import { spawn, exec } from 'node:child_process';
import { cpSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { randomBytes } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import os from 'node:os';

const RAIZ = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const PUERTO = Number(process.env.PANEL_PUERTO ?? 3999);
const { configurations } = JSON.parse(readFileSync(path.join(RAIZ, '.claude/launch.json'), 'utf8'));

// Las batallas con Conquest (2026-11-14, pedido del autor): el botón «Arrancar con Conquest» del lanzador arranca el servidor si está parado y, cuando escucha,
// el lanzador con el servidor de batalla de Conquest (`Builds/PruebaRed/PruebaRed.exe`, que se construye desde Conquest con el menú de `PruebaRedEditor`) en vez
// del de mentira. Conquest está junto al motor (`../Conquest_prototype`) o donde diga `CONQUEST_RAIZ`. Va sin ventana y juega la batalla de verdad
// (`CONQUEST_BATALLA_REAL=1`); `CONQUEST_DURACION` (segundos, opcional) la acorta. Lo que el lanzador pone detrás de cada batalla (`--batalla`, `--lanzador`,
// `--puerto`) no se toca. «Arrancar» sigue arrancando el de mentira de launch.json.
const CONQUEST = path.resolve(RAIZ, process.env.CONQUEST_RAIZ ?? '../Conquest_prototype');
const PRUEBA_RED = path.join(CONQUEST, 'Builds', 'PruebaRed', process.platform === 'win32' ? 'PruebaRed.exe' : 'PruebaRed');
// Qué lleva el lanzador en marcha ('mentira' o 'conquest') y la petición del botón mientras espera al servidor.
const conquest = { modo: null, duracion: null, esperando: false, mensaje: null };
function envConquest(duracion) {
  const registro = path.join(CONQUEST, 'Logs', 'batalla_servidor.log');
  // LANZADOR_ARGUMENTOS se parte por espacios: el registro solo va si su ruta no tiene ninguno (si no, Unity escribe en su Player.log de siempre).
  const args = ['-batchmode', '-nographics', ...(/\s/.test(registro) ? [] : ['-logFile', registro])];
  return { LANZADOR_PROCESO: PRUEBA_RED, LANZADOR_ARGUMENTOS: args.join(' '), CONQUEST_BATALLA_REAL: '1', ...(duracion ? { CONQUEST_DURACION: String(duracion) } : {}) };
}
function avisoConquest() {
  if (!existsSync(PRUEBA_RED)) return `falta ${PRUEBA_RED}: constrúyelo desde Conquest (menú de PruebaRedEditor, o Unity -batchmode -quit -executeMethod ConquestTactics.Net.Editor.PruebaRedEditor.Construir con el editor cerrado), o pon CONQUEST_RAIZ con la carpeta de Conquest`;
  return null;
}

// Jugar desde otro equipo de la red local (2026-11-15, pedido del autor). Con la red abierta, el servidor escucha en 0.0.0.0 (no solo en 127.0.0.1) y el lanzador
// da a los jugadores la IP elegida como dirección de las batallas (`LANZADOR_HOST`). Se guarda en `partidas/red.json` y vale al arrancar: lo que ya esté en
// marcha sigue como arrancó hasta «Aplicar». El cortafuegos de Windows se abre con un botón (pide permiso de administrador): el puerto del servidor en TCP y
// los de las batallas en UDP. El panel sigue atendiendo solo a esta máquina.
const RED = path.join(RAIZ, 'partidas', 'red.json');
const ipsLocales = () => Object.entries(os.networkInterfaces()).flatMap(([nombre, ds]) => (ds ?? [])
  .filter((d) => d.family === 'IPv4' && !d.internal && !d.address.startsWith('169.254.')).map((d) => ({ ip: d.address, nombre })));
const red = { abierta: false, ip: null, ...(() => { try { return JSON.parse(readFileSync(RED, 'utf8')); } catch { return {}; } })() };
if (!ipsLocales().some((x) => x.ip === red.ip)) red.ip = ipsLocales()[0]?.ip ?? null;
const guardarRed = () => { mkdirSync(path.dirname(RED), { recursive: true }); writeFileSync(RED, JSON.stringify({ abierta: red.abierta, ip: red.ip }, null, 2)); };
const redDe = () => (red.abierta ? `0.0.0.0 · ${red.ip}` : 'local'); // con qué red arranca ahora un servicio
const puertosBatalla = () => configurations.find((c) => c.name === 'bronzeage-lanzador')?.env?.LANZADOR_PUERTOS || process.env.LANZADOR_PUERTOS || '7777-7786';
const ejecutar = (cmd) => new Promise((ok) => exec(cmd, { windowsHide: true }, (e, out) => ok({ codigo: e ? e.code ?? 1 : 0, out: String(out) })));
const REGLAS = () => [
  { nombre: 'BronzeAge servidor', protocolo: 'TCP', puertos: String(puertos['bronzeage-servidor']) },
  { nombre: 'BronzeAge batallas', protocolo: 'UDP', puertos: puertosBatalla() },
  { nombre: 'BronzeAge cliente web', protocolo: 'TCP', puertos: String(puertos['bronzeage-cliente-jugador']) },
];
// Si cada regla existe y lleva sus puertos de ahora. Fuera de Windows no se mira (null).
async function cortafuegos() {
  if (process.platform !== 'win32') return null;
  return Promise.all(REGLAS().map(async (r) => {
    const { codigo, out } = await ejecutar(`netsh advfirewall firewall show rule name="${r.nombre}"`);
    return { ...r, ok: codigo === 0 && out.includes(r.puertos) && out.includes(r.protocolo) };
  }));
}
// Borra y vuelve a crear las dos reglas en un PowerShell elevado (Windows pide permiso). Devuelve null si fue bien o por qué no.
async function abrirCortafuegos() {
  if (process.platform !== 'win32') return 'el cortafuegos solo se abre desde aquí en Windows';
  const ordenes = REGLAS().flatMap((r) => [`netsh advfirewall firewall delete rule name="${r.nombre}" | Out-Null`,
    `netsh advfirewall firewall add rule name="${r.nombre}" dir=in action=allow protocol=${r.protocolo} localport=${r.puertos} profile=any`]).join('; ');
  const b64 = Buffer.from(ordenes, 'utf16le').toString('base64');
  const { codigo } = await ejecutar(`powershell -NoProfile -Command "Start-Process powershell -Verb RunAs -Wait -WindowStyle Hidden -ArgumentList '-NoProfile','-EncodedCommand','${b64}'"`);
  return codigo === 0 ? null : 'no se abrió: hace falta aceptar el permiso de administrador';
}

// Cómo le dice el panel a cada servicio su puerto y el del servidor. P = los puertos elegidos ahora mismo.
// El servidor sirve en un solo puerto el WebSocket del contrato 05 y el HTTP del acceso.
const VITE = (p) => ['--', '--port', p, '--strictPort'];
// `aviso`: por qué no se puede arrancar todavía (la tarjeta lo dice y deja el botón apagado).
// `secretos`: variables de entorno secretas que el servicio necesita; `secreto()` las resuelve al arrancarlo (ver más abajo).
const SERVICIOS = {
  // `parar`: el servidor se apaga con su foto al leer esa línea por la entrada estándar (Windows no deja mandarle Ctrl+C).
  'bronzeage-servidor': { tipo: 'backend', parar: 'parar\n', aviso: avisoMundo, secretos: ['CODIGO_REGISTRO_BOTS', 'CODIGO_REGISTRO_SERVIDORES'], con: (p) => ({ env: { BRONZEAGE_DIRECCION: `${red.abierta ? '0.0.0.0' : '127.0.0.1'}:${p}`, ...envPartida() } }) },
  // El lanzador de servidores de batalla (2026-11-11): sin puerto propio (los de juego, 7777-7786, los abre cada batalla). Con la partida en Unity, entra con su cuenta
  // `lanzador-local` (la da de alta solo con el código de servidores, que es secreto del panel como su clave). Qué proceso arranca por batalla lo dice launch.json:
  // `LANZADOR_PROCESO=mentira` es el servidor de batalla de mentira que lleva dentro (dura lo que diga `LANZADOR_ARGUMENTOS`, nadie muere y gana el defensor); para
  // Conquest, la ruta de su ejecutable y sus argumentos.
  'bronzeage-lanzador': { tipo: 'backend', parar: 'parar\n', aviso: avisoLanzador, secretos: ['CODIGO_REGISTRO_SERVIDORES', 'LANZADOR_CLAVE'], con: (p, P) => ({ env: {
    LANZADOR_MOTOR: `127.0.0.1:${P['bronzeage-servidor']}`, LANZADOR_NICK: 'lanzador-local', LANZADOR_HOST: red.abierta ? red.ip : '127.0.0.1' } }) },
  // Una tarea que termina sola: sin puerto. La semilla la elige POST /api/mundo.
  'generar-mundo': { tipo: 'tarea', con: () => ({ args: [String(mundo.semilla)] }) },
  // `BOTS_REGISTROS` (2026-11-03, aviso de los bots): la ruta absoluta de `partidas/`. Los bots guardan sus registros dentro de cada partida
  // (`partidas/<nombre>/bots.json` y `mercs.json`), así que borrar la partida los borra con ella; `servicio-bots.json` y `servicio-mercs.json` van en esa carpeta.
  'bronzeage-bots': { tipo: 'backend', secretos: ['CODIGO_REGISTRO_BOTS', 'BOTS_TOKEN'], con: (p, P) => ({ env: { BOTS_PUERTO: p, BOTS_SERVIDOR: `http://localhost:${P['bronzeage-servidor']}`, BOTS_REGISTROS: PARTIDAS } }) },
  // Los bots mercenarios (2026-11-02): el mismo ejecutable de bots en un segundo proceso, con `BOTS_POBLACION=mercenarios` (en .claude/launch.json),
  // su propio puerto de control y el mismo servidor, token y código de registro que los libres (los secretos los pone `secreto()`, no launch.json). Los dos comparten carpeta y `target` de cargo:
  // si cambia el código de los bots, conviene parar los dos antes de arrancar (en Windows un ejecutable en marcha no se puede reemplazar).
  'bronzeage-bots-mercenarios': { tipo: 'backend', secretos: ['CODIGO_REGISTRO_BOTS', 'BOTS_TOKEN'], con: (p, P) => ({ env: { BOTS_PUERTO: p, BOTS_SERVIDOR: `http://localhost:${P['bronzeage-servidor']}`, BOTS_REGISTROS: PARTIDAS } }) },
  // El admin lee el control de los libres de `BOTS_URL` y el de los mercenarios de `MERCS_URL` (sin ella, ws://localhost:4001/control).
  'bronzeage-admin': { tipo: 'cliente', con: (p, P) => ({ args: VITE(p), env: {
    BACKEND_URL: `http://localhost:${P['bronzeage-servidor']}`, BOTS_URL: `ws://localhost:${P['bronzeage-bots']}/control`,
    MERCS_URL: `ws://localhost:${P['bronzeage-bots-mercenarios']}/control`, PANEL_URL: `http://localhost:${PUERTO}` } }) },
  // Con la red abierta escucha en todas las interfaces, para que otro equipo lo abra por la IP de este (llama al motor en esa misma IP, puerto 7878).
  'bronzeage-cliente-jugador': { tipo: 'cliente', con: (p, P) => ({ args: [...VITE(p), ...(red.abierta ? ['--host', '0.0.0.0'] : [])], env: { BACKEND_URL: `http://localhost:${P['bronzeage-servidor']}` } }) },
};

// El lanzador solo sirve con una partida que juega las batallas en Unity (se elige al crearla).
function avisoLanzador() {
  const n = partidaActual();
  if (!n) return 'no hay partida activa';
  return infoPartida(n).batallas_en_unity ? null : `la partida ${n} resuelve las batallas con números: crea una con «batallas en Unity» para jugarlas con el lanzador`;
}
// El servidor necesita el artefacto de mundo (`BRONZEAGE_MUNDO`: un .bin o una carpeta con un mundo-*.bin; por defecto generador/salida/mundo).
function avisoMundo(c) {
  if (!partidaActual()) return 'no hay partida activa: crea una o activa una de la lista (POST /api/partidas o /api/partidas/<nombre>/activar)';
  const ruta = path.resolve(RAIZ, envPartida().BRONZEAGE_MUNDO ?? c.env?.BRONZEAGE_MUNDO ?? process.env.BRONZEAGE_MUNDO ?? 'generador/salida/mundo');
  if (ruta.endsWith('.bin') ? existsSync(ruta) : existsSync(ruta) && readdirSync(ruta).some((f) => /^mundo-.*\.bin$/.test(f))) return null;
  if (procesos['generar-mundo']?.hijo) return 'generando el artefacto de mundo… (unos 2 min)';
  return `falta el artefacto de mundo en ${ruta}: arranca «generar-mundo» (unos 2 min) o pon BRONZEAGE_MUNDO con la ruta de un mundo-*.bin`;
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
  return { nombre: n, carpeta: carpetaPartida(n), semilla: meta.semilla ?? 42, mundo: meta.mundo ?? null, creada_ms: meta.creada_ms ?? null, batallas_en_unity: meta.batallas_en_unity === true,
    respaldos: respaldosDe(n).length, actual: n === partidaActual(), en_marcha: n === partidaActual() && enMarcha() };
}
// El entorno del servidor para la partida actual: su carpeta, su semilla, si juega las batallas en Unity (una regla de la partida: se fija al crearla) y su artefacto (si lo fijó al crearla).
function envPartida() {
  const n = partidaActual();
  if (!n) return {};
  const i = infoPartida(n);
  return { BRONZEAGE_DATOS: i.carpeta, BRONZEAGE_SEMILLA: String(i.semilla), BRONZEAGE_BATALLAS_EN_UNITY: i.batallas_en_unity ? '1' : '0', ...(i.mundo ? { BRONZEAGE_MUNDO: path.join(MUNDOS, i.mundo) } : {}) };
}

// La regeneración del mundo (la tarea «generar-mundo»): con qué semilla y cómo acabó la última.
const mundo = { semilla: 42, ultima: null };

const puertos = Object.fromEntries(configurations.map((c) => [c.name, c.port]));
const procesos = {}; // nombre -> { hijo, log[] }
const carpeta = (c) => path.resolve(RAIZ, c.cwd ?? '.');

// Un secreto de un servicio: el que traiga en su `env` de launch.json, si no el del entorno del panel (para quien quiera fijarlo) y, si no, uno aleatorio
// que se genera la primera vez y se guarda en `partidas/secretos.json` (fuera de git) para que sea el mismo en los arranques siguientes.
function secreto(c, clave) {
  const fijo = c.env?.[clave] || process.env[clave];
  if (fijo) return fijo;
  const f = path.join(PARTIDAS, 'secretos.json'), guardados = leerJson(f) ?? {};
  if (!guardados[clave]) {
    guardados[clave] = randomBytes(18).toString('base64url');
    mkdirSync(PARTIDAS, { recursive: true });
    writeFileSync(f, JSON.stringify(guardados, null, 2), { mode: 0o600 });
  }
  return guardados[clave];
}

function escucha(puerto, host = 'localhost') {
  return new Promise((ok) => {
    // 'localhost' + autoSelectFamily: Vite escucha solo en ::1, el servidor en 127.0.0.1. Con `host` (la IP de la red), si se le alcanza desde fuera.
    const s = net.connect({ port: puerto, host, autoSelectFamily: true });
    s.once('connect', () => { s.destroy(); ok(true); });
    s.once('error', () => ok(false));
    s.setTimeout(400, () => { s.destroy(); ok(false); });
  });
}

function arrancar(c, extra = {}) {
  if (procesos[c.name]?.hijo || !existsSync(carpeta(c)) || SERVICIOS[c.name]?.aviso?.(c)) return;
  const P = Object.fromEntries(Object.entries(puertos).map(([k, v]) => [k, String(v)]));
  const { args = [], env: base = {} } = SERVICIOS[c.name]?.con(P[c.name], P) ?? {};
  const env = { ...base, ...extra };
  if (c.name === 'bronzeage-lanzador') { conquest.modo = extra.LANZADOR_PROCESO === PRUEBA_RED ? 'conquest' : 'mentira'; conquest.duracion = extra.CONQUEST_DURACION ?? null; }
  for (const k of SERVICIOS[c.name]?.secretos ?? []) env[k] = secreto(c, k);
  const p = { log: [], red: redDe() };
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

// El botón «Arrancar con Conquest»: el servidor si hace falta (y si puede), y el lanzador con Conquest en cuanto el servidor escucha (cargo puede tardar en compilar).
async function arrancarConquest(duracion, mentira = false) {
  const s = configurations.find((x) => x.name === 'bronzeage-servidor'), l = configurations.find((x) => x.name === 'bronzeage-lanzador');
  if (!s || !l || conquest.esperando || procesos[l.name]?.hijo) return;
  const no = (mentira ? null : avisoConquest()) ?? SERVICIOS[l.name].aviso?.(l);
  if (no) { conquest.mensaje = no; return; }
  conquest.esperando = true; conquest.mensaje = null;
  try {
    if (!(await escucha(puertos[s.name]))) {
      const noServidor = SERVICIOS[s.name].aviso?.(s);
      if (noServidor) { conquest.mensaje = `el servidor no puede arrancar: ${noServidor}`; return; }
      arrancar(s);
      const limite = Date.now() + 15 * 60_000;
      while (!(await escucha(puertos[s.name]))) {
        if (!procesos[s.name]?.hijo) { conquest.mensaje = 'el servidor se paró antes de escuchar: mira su registro'; return; }
        if (Date.now() > limite) { conquest.mensaje = 'el servidor no escucha tras 15 min: mira su registro'; return; }
        await new Promise((ok) => setTimeout(ok, 1000));
      }
    }
    arrancar(l, mentira ? {} : envConquest(duracion));
  } finally { conquest.esperando = false; }
}

// Lo que escucha distinto con la red abierta: si arrancó con la red de antes, «Aplicar» y «Jugar» lo reinician.
const CON_RED = ['bronzeage-servidor', 'bronzeage-lanzador', 'bronzeage-cliente-jugador'];
// «Aplicar» de la red: vuelve a arrancar el servidor, el lanzador y el cliente web que estén en marcha, para que tomen la red de ahora (el lanzador, como estaba: con Conquest o de mentira).
async function aplicarRed() {
  const c = 'bronzeage-cliente-jugador';
  if (procesos[c]?.hijo) { const p = procesos[c]; parar(c); await p.fin; arrancar(configurations.find((x) => x.name === c)); }
  const l = 'bronzeage-lanzador', antes = procesos[l]?.hijo ? { modo: conquest.modo, duracion: conquest.duracion } : null;
  if (antes) { const p = procesos[l]; parar(l); await p.fin; }
  if (await pararServidor()) arrancar(servidor());
  if (antes) await arrancarConquest(antes.duracion, antes.modo !== 'conquest');
}

// «Jugar» (pestaña Inicio, 2026-11-15): arranca lo que falte para jugar la partida activa: el servidor y, si la partida juega las batallas en Unity,
// el lanzador con Conquest (con el de mentira si Conquest no está construido). No espera a cargo: el progreso lo enseña el estado. Devuelve por qué no puede, o null.
// `conRed` (si viene) abre o cierra la red local antes; con la red abierta, abre el cortafuegos si hace falta (Windows pide permiso) y reinicia lo que arrancó con la red de antes.
async function jugar(conRed) {
  const n = partidaActual(), s = servidor();
  if (!n) return 'no hay partida activa: elige una o crea una nueva';
  if (!(await escucha(puertos[s.name])) && !procesos[s.name]?.hijo) { const no = SERVICIOS[s.name].aviso(s); if (no) return no; }
  if (conRed !== undefined && conRed !== red.abierta) {
    if (conRed && !red.ip) return 'este equipo no tiene ninguna IP de red local';
    red.abierta = conRed === true;
    guardarRed();
  }
  if (red.abierta && (await cortafuegos())?.some((r) => !r.ok)) { const no = await abrirCortafuegos(); if (no) return no; }
  const seguir = async () => {
    // Con la red abierta, el cliente web también, para que el otro equipo entre por su puerto.
    const cw = configurations.find((x) => x.name === 'bronzeage-cliente-jugador');
    if (red.abierta && cw && !(await escucha(puertos[cw.name]))) arrancar(cw);
    if (!infoPartida(n).batallas_en_unity) { if (!(await escucha(puertos[s.name]))) arrancar(s); return; }
    conquest.mensaje = null;
    arrancarConquest(null, !!avisoConquest()); // arranca también el servidor si hace falta
  };
  if (CON_RED.some((x) => procesos[x]?.hijo && procesos[x].red !== redDe())) aplicarRed().then(seguir);
  else await seguir();
  return null;
}
// «Parar todo»: lo que lanzó el panel, menos una generación de mundo a medias.
const pararTodo = () => Object.keys(procesos).filter((n) => n !== 'generar-mundo').forEach(parar);

async function estado() {
  return Promise.all(configurations.map(async (c) => {
    const p = procesos[c.name];
    return {
      nombre: c.name, tipo: SERVICIOS[c.name]?.tipo ?? 'backend', puerto: puertos[c.name],
      comando: [c.runtimeExecutable, ...c.runtimeArgs].join(' ') + (c.cwd ? `  (en ${c.cwd})` : '')
        + (c.name === 'bronzeage-servidor' ? `  [partida ${partidaActual() ?? 'ninguna'}]` : ''),
      // El token del canal de control de los bots, para pegarlo en el admin web (el código de registro no sale del panel).
      token_control: SERVICIOS[c.name]?.secretos?.includes('BOTS_TOKEN') ? secreto(c, 'BOTS_TOKEN') : null,
      falta: !existsSync(carpeta(c)), aviso: SERVICIOS[c.name]?.aviso?.(c) ?? null,
      escucha: puertos[c.name] ? await escucha(puertos[c.name]) : false, lanzado: !!p?.hijo, log: p?.log.slice(-200) ?? [], red: p?.hijo ? p.red : null,
      ...(c.name === 'bronzeage-lanzador' ? { conquest: { exe: PRUEBA_RED, aviso: avisoConquest(), modo: p?.hijo ? conquest.modo : null, duracion: conquest.duracion,
        esperando: conquest.esperando, mensaje: conquest.mensaje } } : {}),
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
.token{margin:0 0 8px}.token[hidden]{display:none}.token code{font:12px ui-monospace,monospace;user-select:all}
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
#v-inicio .card{max-width:820px}.form input[type=checkbox]{min-width:0;width:auto}.pasos{list-style:none;margin:16px 0;padding:0;display:grid;gap:9px}
.pasos li{display:flex;align-items:center;gap:10px;flex-wrap:wrap}.pasos b{min-width:84px;font-weight:600}.pasos li button{padding:3px 10px;font-size:12px}
.grande{font-size:16px;padding:10px 28px;background:var(--acento);color:#14110d;border-color:var(--acento);font-weight:600}.grande:hover:not(:disabled){filter:brightness(1.1)}
select{background:#0d0b08;color:var(--txt);border:1px solid var(--borde);border-radius:6px;padding:6px;font:inherit}
.reg{margin:0}.reg>summary{font-size:12px;text-transform:none;letter-spacing:0;border:none;margin:10px 0 0;padding:0}
.interruptor{margin:14px 0 0;display:flex;gap:8px;align-items:center;font-size:14px;color:var(--txt);cursor:pointer}.interruptor input{width:auto;min-width:0}
.enred{margin-top:10px;padding:10px 12px;border:1px dashed var(--acento);border-radius:8px;display:grid;gap:8px}.enred[hidden]{display:none}
.enred code{font:13px ui-monospace,monospace;color:var(--txt);user-select:all}.enred button{padding:3px 10px;font-size:12px}
.est{font-size:12px;color:var(--sub)}.aviso{color:var(--wait);font-size:12px;margin:0 0 8px}.aviso:empty{display:none}.nota{color:var(--sub);font-size:12px;margin-top:22px}
</style></head><body><h1>⚒ Panel de arranque — BronzeAge</h1>
<nav class="pestanas"><button data-vista="inicio">Inicio</button><button data-vista="partidas">Mundo y partidas</button><button data-vista="avanzado">Avanzado</button></nav>
<section class="vista" id="v-inicio">
<details open data-seccion="jugar"><summary>Jugar</summary>
<div class="card"><div class="form"><label>Partida<select id="i-partida"></select></label></div>
<label class="interruptor"><input type="checkbox" id="i-red"> Dejar entrar a otros equipos de mi red (wifi o cable)</label>
<div class="enred" id="i-enred" hidden>
<div class="fila"><span class="est">Cliente web, en el navegador del otro equipo</span><code id="i-web"></code><button data-copia="i-web">Copiar</button><span id="i-ips"></span></div>
<div class="fila"><span class="est">El motor, para Conquest</span><code id="i-url"></code><button data-copia="i-url">Copiar</button></div>
<div class="fila"><span class="est">Conquest compilado, en el otro equipo</span><code id="i-cmd"></code><button data-copia="i-cmd">Copiar</button></div>
<div class="est">Con el editor de Unity: esa dirección en el campo «Servidor» de ConexionDeMundo (Inspector). La cuenta se crea con «Registrar».</div></div>
<ul class="pasos" id="i-pasos"></ul>
<div class="fila"><button class="grande" id="i-jugar">▶ Jugar</button><button id="i-parar">■ Parar todo</button></div><div class="msg" id="i-msg"></div></div></details>
<details open data-seccion="abrir"><summary>Abrir</summary><div class="card"><div class="fila" id="i-clientes"></div><div class="est" style="margin-top:8px">Si no está en marcha, se arranca y se abre en cuanto responde.</div></div></details>
</section>
<section class="vista" id="v-partidas">
<details open data-seccion="partidas"><summary>Partidas<span class="cuenta" id="p-cuenta"></span></summary>
<div class="card"><table><thead><tr><th>Partida</th><th>Semilla</th><th>Mundo</th><th>Batallas</th><th>Creada</th><th>Estado</th><th></th></tr></thead><tbody id="p-lista"></tbody></table>
<div class="msg" id="p-msg"></div></div></details>
<details open data-seccion="nueva"><summary>Nueva partida</summary>
<div class="card"><div class="form"><label>Nombre<input id="n-nombre" placeholder="misiones" maxlength="40"></label>
<label>Semilla<input id="n-semilla" type="number" min="0" value="42"></label>
<label>Mundo<select id="n-mundo"></select></label>
<label style="flex-direction:row;align-items:center" title="Las juega el lanzador (bronzeage-lanzador) en vez de resolverse con números. Es una regla de la partida: no se cambia después"><input id="n-unity" type="checkbox" style="width:auto"> batallas en Unity</label>
<label style="flex-direction:row;align-items:center"><input id="n-activar" type="checkbox" checked style="width:auto"> activarla y arrancar el servidor</label>
<button id="n-crear">Crear</button></div><div class="msg" id="n-msg"></div></div></details>
<details open data-seccion="mundo"><summary>Mundo</summary>
<div class="card"><div class="form"><label>Semilla<input id="m-semilla" type="number" min="0" value="42"></label><button id="m-generar">Generar mundo (unos 2 min)</button>
<span class="est" id="m-estado"></span></div>
<table style="margin-top:12px"><thead><tr><th>Artefacto</th><th>Creado</th><th>Partidas</th><th></th></tr></thead><tbody id="m-lista"></tbody></table><div class="msg" id="m-msg"></div></div></details>
</section>
<section class="vista" id="v-avanzado">
<details open data-seccion="red"><summary>Red local (detalle)<span class="cuenta" id="r-cuenta"></span></summary>
<div class="card"><div class="form"><label style="flex-direction:row;align-items:center"><input id="r-abierta" type="checkbox" style="width:auto"> abrir a la red local</label>
<label>IP de este equipo<select id="r-ip"></select></label><button id="r-cortafuegos">Abrir cortafuegos</button><button id="r-aplicar">Aplicar (reinicia servidor y lanzador)</button></div>
<div class="est" id="r-estado" style="margin-top:10px"></div><div class="aviso" id="r-aviso" style="margin-top:8px"></div>
<div class="token fila" id="r-otro" hidden><span class="est">en el otro equipo</span><code></code><button>Copiar</button></div><div class="msg" id="r-msg"></div></div></details>
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
let vista='inicio';try{vista=localStorage.getItem('panel.vista')||vista}catch{}verVista(['inicio','partidas','avanzado'].includes(vista)?vista:'inicio');
// La API de partidas y mundo con JSON; deja el resultado en la línea de mensajes msg.
async function api(metodo,u,cuerpo,msg,espera){
  const el=document.getElementById(msg);el.className='msg';el.textContent=espera??'…';
  try{const r=await fetch(u,{method:metodo,headers:{'content-type':'application/json'},body:cuerpo?JSON.stringify(cuerpo):undefined});
    const v=await r.json().catch(()=>({}));
    if(!r.ok){el.className='msg error';el.textContent=v.mensaje||v.error||('error '+r.status);return null}
    el.className='msg ok';el.textContent=v.aviso?('hecho; el servidor no arranca: '+v.aviso):'hecho';pintarPartidas();pintarRed();setTimeout(pintar,300);return v;
  }catch(e){el.className='msg error';el.textContent=String(e);return null}
}
const fecha=ms=>ms?new Date(ms).toLocaleString('es-ES',{dateStyle:'short',timeStyle:'short'}):'—';
const celda=(tr,t,cl)=>{const td=document.createElement('td');td.textContent=t;if(cl)td.className=cl;tr.append(td);return td};
const boton=(td,t,f,cl)=>{const b=document.createElement('button');b.textContent=t;b.onclick=f;if(cl)b.className=cl;td.querySelector('.fila').append(b);return b};
const abiertos=new Set(); // partidas con la lista de respaldos desplegada
async function pintarPartidas(){
  let d,m;try{[d,m]=await Promise.all([(await fetch('/api/partidas')).json(),(await fetch('/api/mundo')).json()])}catch{return}
  P=d;M=m;
  const lista=document.getElementById('p-lista');lista.textContent='';
  document.getElementById('p-cuenta').textContent='· '+d.partidas.length+(d.actual?' · activa: '+d.actual:' · ninguna activa');
  for(const p of d.partidas){
    const tr=document.createElement('tr'),n=encodeURIComponent(p.nombre);
    celda(tr,p.nombre,p.actual?'activa':'');celda(tr,String(p.semilla));celda(tr,p.mundo?p.mundo.slice(6,18)+'…':'el último');celda(tr,p.batallas_en_unity?'en Unity':'con números');celda(tr,fecha(p.creada_ms));
    celda(tr,p.actual?(p.en_marcha?'activa · servidor en marcha':'activa · servidor parado'):'guardada');
    const acc=celda(tr,'');acc.innerHTML='<div class="fila"></div>';
    if(!p.actual)boton(acc,'Activar',()=>api('POST','/api/partidas/'+n+'/activar',null,'p-msg'));
    else{if(!p.en_marcha)boton(acc,'Arrancar',()=>api('POST','/api/partidas/'+n+'/activar',null,'p-msg'));
      boton(acc,'Apagar',()=>api('POST','/api/partidas/'+n+'/apagar',null,'p-msg'));}
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
    const b=boton(acc,'Borrar',()=>{if(confirm('¿Borrar el mundo '+a.hash.slice(0,12)+'…? No se puede deshacer: habría que generarlo otra vez (unos 2 min).'))
      api('DELETE','/api/mundo/'+encodeURIComponent(a.archivo),null,'m-msg')},'peligro');
    if(usan.length||m.generando){b.disabled=true;b.title=m.generando?'se está generando un mundo':'lo usa'+(usan.length>1?'n las partidas ':' la partida ')+usan.join(', ')+(usan.length>1?': bórralas antes':': bórrala antes')}
    ml.append(tr)}
  if(!m.artefactos.length){const tr=document.createElement('tr');celda(tr,'No hay ningún artefacto de mundo: genera uno.').colSpan=4;ml.append(tr)}
  document.getElementById('m-generar').disabled=m.generando;
  document.getElementById('m-estado').textContent=m.generando?'generando con la semilla '+m.semilla+'…':m.ultima?('último: semilla '+m.ultima.semilla+(m.ultima.codigo===0?', bien':', falló (código '+m.ultima.codigo+'; mira su registro en Avanzado)')):'';
}
document.getElementById('n-crear').onclick=()=>api('POST','/api/partidas',{nombre:document.getElementById('n-nombre').value,semilla:Number(document.getElementById('n-semilla').value),
  mundo:document.getElementById('n-mundo').value||undefined,batallas_en_unity:document.getElementById('n-unity').checked,activar:document.getElementById('n-activar').checked},'n-msg');
document.getElementById('m-generar').onclick=()=>api('POST','/api/mundo',{semilla:Number(document.getElementById('m-semilla').value)},'m-msg');
pintarPartidas();setInterval(pintarPartidas,4000);
// La red local: el interruptor, la IP, el cortafuegos y qué falta para que entre otro equipo.
async function pintarRed(){
  let r;try{r=await (await fetch('/api/red')).json()}catch{return}
  RED=r;
  const ab=document.getElementById('r-abierta'),sel=document.getElementById('r-ip');ab.checked=r.abierta;
  if(document.activeElement!==sel){sel.textContent='';for(const x of r.ips){const o=document.createElement('option');o.value=x.ip;o.textContent=x.ip+' ('+x.nombre+')';sel.append(o)}sel.value=r.ip??''}
  const fw=r.cortafuegos,fwOk=!fw||fw.every(x=>x.ok);
  document.getElementById('r-cortafuegos').disabled=!fw;
  document.getElementById('r-cuenta').textContent=r.abierta?'· abierta en '+r.ip:'· solo este equipo';
  document.getElementById('r-estado').textContent=fw?'Cortafuegos: '+fw.map(x=>x.nombre+' ('+x.protocolo+' '+x.puertos+') '+(x.ok?'abierto':'sin abrir')).join(' · '):'Cortafuegos: ábrelo a mano (no es Windows)';
  const viejos=r.en_marcha.filter(x=>x.red!==r.con);
  document.getElementById('r-aplicar').disabled=!viejos.length;
  document.getElementById('r-aviso').textContent=viejos.length?viejos.map(x=>x.nombre).join(' y ')+' siguen con la red de antes: pulsa «Aplicar»':r.abierta&&!fwOk?'El cortafuegos no deja entrar: pulsa «Abrir cortafuegos» (Windows pedirá permiso de administrador)':'';
  const otro=document.getElementById('r-otro');otro.hidden=!r.abierta;otro.querySelector('code').textContent='Conquest.exe -servidor http://'+r.ip+':'+r.puerto;
}
document.getElementById('r-abierta').onchange=e=>api('POST','/api/red',{abierta:e.target.checked},'r-msg').then(pintarRed);
document.getElementById('r-ip').onchange=e=>api('POST','/api/red',{ip:e.target.value},'r-msg').then(pintarRed);
document.getElementById('r-cortafuegos').onclick=()=>api('POST','/api/red/cortafuegos',null,'r-msg').then(pintarRed);
document.getElementById('r-aplicar').onclick=()=>api('POST','/api/red/aplicar',null,'r-msg').then(pintarRed);
document.querySelector('#r-otro button').onclick=e=>navigator.clipboard.writeText(document.querySelector('#r-otro code').textContent).then(()=>{e.target.textContent='Copiado';setTimeout(()=>e.target.textContent='Copiar',1500)});
pintarRed();setInterval(pintarRed,5000);
// ───── Inicio: el estado de un vistazo y los botones de siempre ─────
const NOMBRES={'bronzeage-servidor':'Servidor del mundo','bronzeage-lanzador':'Lanzador de batallas','generar-mundo':'Generar mundo','bronzeage-bots':'Bots',
  'bronzeage-bots-mercenarios':'Bots mercenarios','bronzeage-admin':'Admin web','bronzeage-cliente-jugador':'Cliente jugador (web)'};
let P=null,M=null,RED=null,ESTADO=[]; // lo último de /api/partidas, /api/mundo, /api/red y /api/estado
function paso(ul,color,titulo,texto,accion){
  const li=document.createElement('li');li.innerHTML='<span class="dot"></span><b></b><span class="est"></span>';
  li.querySelector('.dot').style.background='var(--'+color+')';li.querySelector('b').textContent=titulo;li.querySelector('.est').textContent=texto;
  if(accion){const b=document.createElement('button');b.textContent=accion[0];b.onclick=accion[1];li.append(b)}
  ul.append(li);
}
function pintarInicio(datos){
  ESTADO=datos;if(!P||!M)return;
  const de=n=>datos.find(d=>d.nombre===n),s=de('bronzeage-servidor'),l=de('bronzeage-lanzador'),q=l?.conquest??{},p=P.partidas.find(x=>x.actual);
  const sel=document.getElementById('i-partida');
  if(document.activeElement!==sel){sel.textContent='';if(!p)sel.append(new Option('— ninguna activa —',''));
    for(const x of P.partidas)sel.append(new Option(x.nombre+(x.batallas_en_unity?' · batallas en Unity':' · batallas con números'),x.nombre));
    sel.append(new Option('+ Nueva partida…','+'));sel.value=p?.nombre??''}
  const ul=document.getElementById('i-pasos');ul.textContent='';
  if(!p)paso(ul,'off','Partida','no hay ninguna activa: elige una arriba o crea una nueva',['Nueva partida',()=>{verVista('partidas');document.getElementById('n-nombre').focus()}]);
  else if(M.generando)paso(ul,'wait','Mundo','generándose con la semilla '+M.semilla+'… (unos 2 min)');
  else if(p.mundo?M.artefactos.some(a=>a.archivo===p.mundo):M.artefactos.length)paso(ul,'on','Mundo','listo');
  else if(p.mundo)paso(ul,'off','Mundo','falta el mundo de esta partida ('+p.mundo.slice(6,18)+'…): genéralo con su semilla',['Ir a Mundo',()=>verVista('partidas')]);
  else paso(ul,'off','Mundo','no hay ningún mundo generado',['Generar mundo (2 min)',()=>api('POST','/api/mundo',{semilla:42},'i-msg')]);
  if(s.escucha)paso(ul,'on','Servidor','en marcha en el puerto '+s.puerto+(s.red&&s.red!=='local'?' · abierto a la red':''));
  else if(s.lanzado)paso(ul,'wait','Servidor','arrancando… (si hay que compilar, tarda unos minutos)');
  else paso(ul,'off','Servidor','apagado');
  if(p&&!p.batallas_en_unity)paso(ul,'sub','Batallas','se resuelven con números: no hace falta el lanzador');
  else if(p&&l.lanzado)paso(ul,'on','Batallas',q.modo==='conquest'?'con Conquest':'con el servidor de batalla de mentira'+(q.aviso?' (Conquest no está construido)':''));
  else if(p&&q.esperando)paso(ul,'wait','Batallas','esperando a que arranque el servidor…');
  else if(p)paso(ul,'off','Batallas',q.mensaje||('apagado'+(q.aviso?' · Conquest no está construido: irán con el de mentira':'')));
  const fwMal=!!RED?.abierta&&!!RED.cortafuegos?.some(x=>!x.ok),viejos=!!RED?.en_marcha.some(x=>x.red!==RED.con);
  if(RED&&!RED.abierta)paso(ul,'sub','Red','solo este equipo'+(viejos?' · el servidor sigue abierto a la red: pulsa «Jugar» para cerrarla':''));
  else if(RED&&RED.sordos?.length)paso(ul,'off','Red','abierta, pero '+RED.sordos.map(n=>NOMBRES[n]??n).join(' y ')+' solo escucha en este equipo'+(RED.sordos.some(n=>!ESTADO.find(d=>d.nombre===n)?.lanzado)?': se arrancó fuera del panel; ciérralo y pulsa «Jugar»':': pulsa «Jugar» para reiniciarlo'));
  else if(RED)paso(ul,viejos||fwMal?'wait':'on','Red',viejos?'abierta, pero lo que está en marcha arrancó antes: pulsa «Jugar» para aplicarlo':fwMal?'abierta, pero el cortafuegos aún no deja entrar: se abre al pulsar «Jugar» (Windows pide permiso)':'abierta: el otro equipo puede entrar');
  const rb=document.getElementById('i-red');if(RED&&document.activeElement!==rb)rb.checked=RED.abierta;
  document.getElementById('i-enred').hidden=!RED?.abierta;
  if(RED?.abierta){const url='http://'+RED.ip+':'+RED.puerto;document.getElementById('i-url').textContent=url;document.getElementById('i-cmd').textContent='Conquest.exe -servidor '+url;
    const cw=de('bronzeage-cliente-jugador');document.getElementById('i-web').textContent='http://'+RED.ip+':'+(cw?.puerto??5174);
    const ips=document.getElementById('i-ips');
    if(RED.ips.length>1&&!ips.contains(document.activeElement)){ips.innerHTML='<select></select>';const se=ips.firstChild;
      for(const x of RED.ips)se.append(new Option(x.ip+' ('+x.nombre+')',x.ip));se.value=RED.ip;
      se.onchange=()=>api('POST','/api/red',{ip:se.value},'i-msg').then(()=>{se.blur();if(ESTADO.some(d=>d.lanzado&&d.red&&d.red!=='local'))document.getElementById('i-msg').textContent='IP cambiada: pulsa «Jugar» para aplicarla'})}
    else if(RED.ips.length<=1)ips.textContent='';}
  const listo=p&&s.escucha&&(!p.batallas_en_unity||l.lanzado)&&!viejos&&!fwMal&&!RED?.sordos?.length&&(!RED?.abierta||!!de('bronzeage-cliente-jugador')?.escucha),arrancando=(s.lanzado&&!s.escucha)||q.esperando;
  const jb=document.getElementById('i-jugar');jb.disabled=!p||listo||arrancando||M.generando;jb.textContent=listo?'✓ Todo en marcha':arrancando?'Arrancando…':'▶ Jugar';
  document.getElementById('i-parar').disabled=!datos.some(d=>d.lanzado&&d.tipo!=='tarea');
  const cl=document.getElementById('i-clientes');
  for(const d of datos.filter(d=>d.tipo==='cliente')){let b=cl.querySelector('[data-n="'+d.nombre+'"]');
    if(!b){b=document.createElement('button');b.dataset.n=d.nombre;b.onclick=()=>abrirCliente(d.nombre);cl.append(b)}
    b.textContent=(NOMBRES[d.nombre]??d.nombre)+' ↗';b.disabled=d.falta;b.title=d.escucha?'ya está en marcha':'se arranca y se abre';}
}
// Abre un cliente web; si no está en marcha, lo arranca y lo abre cuando responde (la pestaña se abre ya para que el navegador no la bloquee).
async function abrirCliente(n){
  const w=window.open('about:blank','_blank'),m=document.getElementById('i-msg'),nom=NOMBRES[n]??n;
  const busca=async()=>(await (await fetch('/api/estado')).json()).find(x=>x.nombre===n);
  let d=await busca();
  if(!d.escucha){await fetch('/api/arrancar/'+encodeURIComponent(n),{method:'POST'});m.className='msg';m.textContent='arrancando '+nom+'…';
    for(let i=0;i<180&&!d.escucha;i++){await new Promise(r=>setTimeout(r,1000));d=await busca();
      if(!d.lanzado&&i>2){w?.close();m.className='msg error';m.textContent=nom+' no arrancó: mira su registro en Avanzado';return}}}
  m.textContent='';const url='http://localhost:'+d.puerto;if(w)w.location=url;else window.open(url,'_blank');
}
// «Jugar» manda también cómo está el interruptor de la red; si hay que abrir el cortafuegos, Windows pide permiso y la petición espera a que se conteste.
document.getElementById('i-jugar').onclick=()=>{const on=document.getElementById('i-red').checked,fw=on&&RED?.cortafuegos?.some(x=>!x.ok);
  api('POST','/api/jugar',{red:on},'i-msg',fw?'abriendo el cortafuegos: acepta el permiso de administrador de Windows…':'…').then(pintarRed)};
// El interruptor: con el servidor o el lanzador en marcha lo aplica ya (los reinicia); si no, solo lo guarda para el próximo «Jugar».
document.getElementById('i-red').onchange=e=>{const on=e.target.checked;e.target.blur();
  if(!ESTADO.some(d=>(d.nombre==='bronzeage-servidor'||d.nombre==='bronzeage-lanzador')&&d.lanzado))return api('POST','/api/red',{abierta:on},'i-msg').then(pintarRed);
  if(!confirm((on?'Abrir':'Cerrar')+' la red reinicia el servidor y el lanzador: quien esté jugando se desconecta un momento.'+(on&&RED?.cortafuegos?.some(x=>!x.ok)?' Windows pedirá permiso de administrador para abrir el cortafuegos.':'')+' ¿Seguir?')){e.target.checked=!on;return}
  api('POST','/api/jugar',{red:on},'i-msg',on&&RED?.cortafuegos?.some(x=>!x.ok)?'abriendo el cortafuegos: acepta el permiso de administrador de Windows…':'…').then(pintarRed)};
document.querySelectorAll('[data-copia]').forEach(b=>b.onclick=()=>navigator.clipboard.writeText(document.getElementById(b.dataset.copia).textContent).then(()=>{b.textContent='Copiado';setTimeout(()=>b.textContent='Copiar',1500)}));
document.getElementById('i-parar').onclick=()=>{if(confirm('¿Parar todo? El servidor guarda su foto al apagarse; quien esté jugando se queda sin conexión.'))api('POST','/api/parar-todo',null,'i-msg')};
document.getElementById('i-partida').onchange=e=>{const v=e.target.value;e.target.blur();
  if(v==='+'){verVista('partidas');document.getElementById('n-nombre').focus();return}
  if(v&&confirm('¿Activar la partida «'+v+'»? Si el servidor está en marcha, se para (con su foto) y arranca con ella.'))api('POST','/api/partidas/'+encodeURIComponent(v)+'/activar',null,'i-msg');};

async function pintar(){
  const datos=await (await fetch('/api/estado')).json();
  pintarInicio(datos);
  for(const d of datos){
    const n=encodeURIComponent(d.nombre);
    let c=document.getElementById(d.nombre);
    if(!c){c=document.createElement('div');c.className='card';c.id=d.nombre;
      document.getElementById(d.tipo==='cliente'?'clientes':d.tipo==='tarea'?'tareas':'backend').append(c);
      c.innerHTML='<div class="top"><span class="dot"></span><span class="nom"></span><span class="est"></span></div><div class="cmd"></div><div class="aviso"></div><div class="token fila" hidden><span class="est">token de control</span><code></code><button>Copiar</button></div>'
        +'<div class="fila"><button data-a="arrancar">Arrancar</button><button data-a="parar">Parar</button><button class="abrir">Abrir ↗</button>'
        +'<label>puerto <input type="number" min="1" max="65535"></label></div>'
        +(d.conquest?'<div class="fila conquest"><button class="con">Arrancar con Conquest</button><label style="margin-left:0">duración (s) <input class="dur" type="number" min="10" placeholder="ticket"></label><span class="est cq"></span></div><div class="aviso cqa"></div>':'')
        +'<details class="reg"><summary>Registro</summary><pre></pre></details>';
      if(d.conquest)c.querySelector('.con').onclick=()=>{const v=c.querySelector('.dur').value;post('/api/conquest/'+n+(v?'/'+v:''))};
      c.querySelectorAll('[data-a]').forEach(b=>b.onclick=()=>post('/api/'+b.dataset.a+'/'+n));
      c.querySelector('.token button').onclick=e=>navigator.clipboard.writeText(c.querySelector('.token code').textContent).then(()=>{e.target.textContent='Copiado';setTimeout(()=>e.target.textContent='Copiar',1500)});
      c.querySelector('.abrir').onclick=()=>window.open('http://localhost:'+c.querySelector('input').value,'_blank');
      c.querySelector('input').onchange=e=>post('/api/puerto/'+n+'/'+e.target.value);}
    c.querySelector('.dot').style.background=d.escucha||(d.lanzado&&!d.puerto&&d.tipo!=='tarea')?'var(--on)':d.lanzado?'var(--wait)':'var(--off)';
    c.querySelector('.nom').textContent=NOMBRES[d.nombre]??d.nombre;
    c.querySelector('.est').textContent=d.falta?'sin carpeta':d.tipo==='tarea'?(d.lanzado?'en marcha…':'parada'):d.aviso&&!d.lanzado?'no puede arrancar':d.escucha?(d.lanzado?'encendido':'encendido (externo)'):d.lanzado?(d.puerto?'arrancando…':'en marcha'):'apagado';
    c.querySelector('.cmd').textContent=d.nombre+' · '+d.comando;
    c.querySelector('.aviso').textContent=d.aviso??'';
    c.querySelector('.token').hidden=!d.token_control;c.querySelector('.token code').textContent=d.token_control??'';
    c.querySelector('label').style.display=d.tipo==='tarea'||!d.puerto?'none':'';
    if(d.conquest){const q=d.conquest;
      c.querySelector('.con').disabled=!!q.aviso||!!d.aviso||d.lanzado||q.esperando;c.querySelector('.dur').disabled=d.lanzado||q.esperando;
      c.querySelector('.cq').textContent=q.esperando?'arrancando el servidor y esperando a que escuche…':d.lanzado?(q.modo==='conquest'?'con Conquest'+(q.duracion?' · batallas de '+q.duracion+' s':''):'con el servidor de batalla de mentira'):'';
      c.querySelector('.cqa').textContent=q.aviso??q.mensaje??'';
      c.querySelector('.cmd').textContent=d.nombre+' · '+d.comando+(d.lanzado&&q.modo==='conquest'?'  [Conquest: '+q.exe+']':'');}
    const inp=c.querySelector('label input');
    if(document.activeElement!==inp)inp.value=d.puerto;
    inp.disabled=d.lanzado; // el puerto se fija al arrancar
    c.querySelector('.abrir').classList.toggle('on',d.tipo==='cliente'&&d.escucha);
    c.querySelector('[data-a=arrancar]').disabled=d.falta||!!d.aviso||d.lanzado||d.escucha;
    c.querySelector('[data-a=parar]').disabled=!d.lanzado;
    const pre=c.querySelector('pre'),abajo=pre.scrollTop+pre.clientHeight>=pre.scrollHeight-4;
    pre.textContent=d.log.join('\\n');if(abajo)pre.scrollTop=pre.scrollHeight;
  }
  // En el título de cada sección, cuántos están en marcha: se ve aunque esté plegada.
  for(const g of document.querySelectorAll('#v-avanzado details[data-seccion]:not([data-seccion=red])')){
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
    writeFileSync(path.join(PARTIDAS, n, 'partida.json'), JSON.stringify({ nombre: n, semilla, mundo: elegido.archivo, batallas_en_unity: cuerpo?.batallas_en_unity === true, creada_ms: Date.now() }, null, 2));
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
    rmSync(path.join(MUNDOS, `relieve-${a.hash}.bin`), { force: true }); // su paquete de relieve, si lo tiene
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

// Solo se atiende a páginas y llamadas de esta máquina (contra webs ajenas y DNS rebinding): `Host` y, si viene, `Origin` han de ser de aquí.
// Devuelve las cabeceras CORS para ese origen ({} sin `Origin`) o null si no se atiende.
const LOCAL = '(localhost|127\\.0\\.0\\.1|\\[::1\\])(:\\d+)?';
const HOST_LOCAL = new RegExp(`^${LOCAL}$`, 'i');
const ORIGEN_LOCAL = new RegExp(`^https?://${LOCAL}$`, 'i');
function procedencia(req) {
  if (!HOST_LOCAL.test(req.headers.host ?? '')) return null;
  const origen = req.headers.origin;
  if (origen === undefined) return {};
  if (!ORIGEN_LOCAL.test(origen)) return null;
  return { 'access-control-allow-origin': origen, vary: 'origin', 'access-control-allow-methods': 'GET, POST, PUT, DELETE, OPTIONS', 'access-control-allow-headers': 'content-type, authorization' };
}
const leerCuerpo = (req) => new Promise((ok) => { let b = ''; req.on('data', (d) => { b += d; }); req.on('end', () => { try { ok(b ? JSON.parse(b) : null); } catch { ok(undefined); } }); });

http.createServer(async (req, res) => {
  const cors = procedencia(req);
  if (!cors) { res.writeHead(403); return res.end(); }
  const partes = new URL(req.url, 'http://x').pathname.split('/').filter(Boolean).map(decodeURIComponent);
  if (partes[0] !== 'api') { res.writeHead(200, { 'content-type': 'text/html; charset=utf-8' }); return res.end(HTML); }
  if (req.method === 'OPTIONS') { res.writeHead(204, cors); return res.end(); }
  const json = (estado, v) => { res.writeHead(estado, { ...cors, 'content-type': 'application/json' }); res.end(JSON.stringify(v)); };
  if (partes[1] === 'partidas' || partes[1] === 'mundo') {
    const cuerpo = await leerCuerpo(req);
    if (cuerpo === undefined) return json(400, { error: 'datos_invalidos', mensaje: 'el cuerpo es JSON' });
    try { return json(...(partes[1] === 'mundo' ? await apiMundo(req.method, cuerpo, partes[2]) : await apiPartidas(req.method, partes.slice(1), cuerpo))); }
    catch (e) { return json(500, { error: 'interno', mensaje: String(e?.message ?? e) }); }
  }
  if (partes[1] === 'red') {
    // GET: cómo está la red; POST { abierta, ip }: la cambia (vale al arrancar); POST /api/red/cortafuegos: lo abre; POST /api/red/aplicar: reinicia lo que esté en marcha.
    // `sordos`: lo que escucha en este equipo pero no en la IP de la red (p. ej. arrancado a mano, fuera del panel, sin abrirlo a la red).
    const sordos = red.abierta ? (await Promise.all(['bronzeage-servidor', 'bronzeage-cliente-jugador'].map(async (n) => (await escucha(puertos[n])) && !(await escucha(puertos[n], red.ip)) ? n : null))).filter(Boolean) : [];
    if (req.method === 'GET') return json(200, { ...red, ips: ipsLocales(), puerto: puertos['bronzeage-servidor'], cortafuegos: await cortafuegos(), con: redDe(), sordos,
      en_marcha: CON_RED.filter((n) => procesos[n]?.hijo).map((n) => ({ nombre: n, red: procesos[n].red })) });
    if (req.method !== 'POST') return json(405, { error: 'metodo_no_permitido' });
    if (partes[2] === 'cortafuegos') { const no = await abrirCortafuegos(); return no ? json(409, { error: 'cortafuegos', mensaje: no }) : json(200, { cortafuegos: await cortafuegos() }); }
    if (partes[2] === 'aplicar') { await aplicarRed(); return json(200, { con: redDe() }); }
    const cuerpo = await leerCuerpo(req);
    if (cuerpo?.ip !== undefined) {
      if (!ipsLocales().some((x) => x.ip === cuerpo.ip)) return json(400, { error: 'datos_invalidos', mensaje: 'esa IP no es de este equipo' });
      red.ip = cuerpo.ip;
    }
    if (cuerpo?.abierta !== undefined) {
      if (cuerpo.abierta && !red.ip) return json(409, { error: 'sin_ip', mensaje: 'este equipo no tiene ninguna IP de red local' });
      red.abierta = cuerpo.abierta === true;
    }
    guardarRed();
    return json(200, { ...red, con: redDe() });
  }
  if (req.method === 'POST' && partes[1] === 'jugar') { const no = await jugar((await leerCuerpo(req))?.red); return no ? json(409, { error: 'no_puede', mensaje: no }) : json(200, { ok: true }); }
  if (req.method === 'POST' && partes[1] === 'parar-todo') { pararTodo(); return json(200, { ok: true }); }
  const [, accion, nombre, valor] = partes;
  if (accion === 'estado') return json(200, await estado());
  const c = configurations.find((x) => x.name === (nombre ?? ''));
  if (req.method !== 'POST' || !c) { res.writeHead(404, cors); return res.end(); }
  if (accion === 'arrancar') { if (c.name === 'bronzeage-lanzador') conquest.mensaje = null; arrancar(c); }
  else if (accion === 'conquest' && c.name === 'bronzeage-lanzador') {
    const d = valor === undefined ? null : Number(valor);
    if (d !== null && (!Number.isInteger(d) || d < 10)) { res.writeHead(400, cors); return res.end(); }
    arrancarConquest(d);
  }
  else if (accion === 'parar') parar(c.name);
  else if (accion === 'puerto') {
    const p = Number(valor);
    if (!Number.isInteger(p) || p < 1 || p > 65535 || procesos[c.name]?.hijo) { res.writeHead(400, cors); return res.end(); }
    puertos[c.name] = p;
  }
  res.writeHead(204, cors); res.end();
}).listen(PUERTO, '127.0.0.1', () => {
  const url = `http://localhost:${PUERTO}`;
  console.log(`Panel en ${url}`);
  if (!process.env.PANEL_SIN_NAVEGADOR) exec(process.platform === 'win32' ? `start "" ${url}` : `open ${url}`);
});

// Al cerrar el panel se llevan por delante los servicios que lanzó.
for (const s of ['SIGINT', 'SIGTERM']) process.on(s, () => { Object.keys(procesos).forEach(parar); setTimeout(() => process.exit(), 500); });
