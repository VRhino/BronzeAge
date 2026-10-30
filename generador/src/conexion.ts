import type { Point } from './tipos';
import type { CampoElevacion } from './elevacion';
import { esTransitable } from './costeMovimiento';

/** Conexión terrestre del relieve final, incluyendo lagos: comprueba aristas, no solo centros.
 * Las islas secundarias no son objetivos de conectividad. */
export function comprobarConexion(campo: CampoElevacion) {
  const regional = campo.regional;
  const { ancho, alto } = regional.limites;
  const n = 160, dx = ancho / n, dy = alto / n;
  const punto = (i: number): Point => ({ x: (i % n + 0.5) * dx, y: (Math.floor(i / n) + 0.5) * dy });
  const valido = new Uint8Array(n * n), visitado = new Uint8Array(n * n);
  for (let i = 0; i < valido.length; i++) valido[i] = Number(esTransitable(campo, punto(i)));
  // Acceso representativo próximo al centro de cada zona, sobre terreno válido.
  const accesos = regional.areas.map((area) => {
    let mejor = -1, distancia = Infinity;
    for (let i = 0; i < valido.length; i++) if (valido[i]) {
      const p = punto(i), d = Math.hypot(p.x - area.centro.x, p.y - area.centro.y);
      if (d < distancia) { distancia = d; mejor = i; }
    }
    return { id: area.id, indice: mejor, distancia };
  });
  const primero = accesos[0]?.indice ?? -1;
  const cola = primero < 0 ? [] : [primero];
  if (primero >= 0) visitado[primero] = 1;
  for (let k = 0; k < cola.length; k++) {
    const i = cola[k]!, x = i % n, y = Math.floor(i / n);
    const vecinos = [x > 0 ? i - 1 : -1, x < n - 1 ? i + 1 : -1, y > 0 ? i - n : -1, y < n - 1 ? i + n : -1];
    for (const j of vecinos) {
      if (j < 0 || !valido[j] || visitado[j]) continue;
      const a = punto(i), b = punto(j);
      const pasos = Math.ceil(Math.hypot(a.x - b.x, a.y - b.y) / 30);
      let libre = true;
      for (let paso = 1; paso < pasos; paso++) {
        const t = paso / pasos;
        if (!esTransitable(campo, { x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t })) { libre = false; break; }
      }
      if (libre) { visitado[j] = 1; cola.push(j); }
    }
  }
  // Acceso al continente medido: la pendiente local sola también acepta islas secundarias y repisas aisladas.
  const tieneAcceso = (p: Point): boolean => {
    const x=Math.floor(p.x/dx),y=Math.floor(p.y/dy);
    for(let sy=Math.max(0,y-2);sy<=Math.min(n-1,y+2);sy++)
      for(let sx=Math.max(0,x-2);sx<=Math.min(n-1,x+2);sx++){
        const i=sy*n+sx;if(!visitado[i])continue;
        const q=punto(i),pasos=Math.max(1,Math.ceil(Math.hypot(q.x-p.x,q.y-p.y)/30));
        let libre=true;
        for(let k=0;k<=pasos;k++)if(!esTransitable(campo,{
          x:p.x+(q.x-p.x)*k/pasos,y:p.y+(q.y-p.y)*k/pasos})){libre=false;break;}
        if(libre)return true;
      }
    return false;
  };
  const conectadas = accesos.filter((a) => a.indice >= 0 && a.distancia <= Math.min(ancho, alto) * 0.025 && visitado[a.indice]).map((a) => a.id);
  return { tieneAcceso, zonas: accesos.length, conectadas: conectadas.length, ids: conectadas,
    pasoMalla: Math.max(dx, dy), pasoArista: 30, superficieConectadaKm2: cola.length * dx * dy / 1e6 };
}
