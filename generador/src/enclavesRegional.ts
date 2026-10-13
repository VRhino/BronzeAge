import type { Point } from './tipos';
import type { CampoElevacion } from './types';
import type { CampoForestal } from './forestacionRegional';
import type { RecursoRegional } from './suelosRegional';
import { evaluarForestal } from './forestacionRegional';
import { evaluarTerreno, gradienteElevacion } from './elevacion';
import { areaRegionalEn } from './regional';


export interface EnclaveInicial { centro: Point; madera: Point; piedraId: string; radio: number }
const distancia=(a:Point,b:Point)=>Math.hypot(a.x-b.x,a.y-b.y);

/** Testigos geográficos para productores; no simula la economía ni asigna leñeras. */
export function prepararEnclavesRegional(recursos:RecursoRegional[],elevacion:CampoElevacion,bosque:CampoForestal):RecursoRegional[]{
 const salida=recursos.map(r=>({...r})),limites=elevacion.regional!.limites;
 const valido=(p:Point,pendienteMax=.12)=>{
  if(p.x<0||p.y<0||p.x>limites.ancho||p.y>limites.alto||['agua','cima'].includes(evaluarTerreno(elevacion,p)))return false;
  const g=gradienteElevacion(elevacion,p,2);return Math.hypot(g.dx,g.dy)*1000<=pendienteMax;
 };
 const candidatos:{p:Point;area:number}[]=[];
 for(let y=150;y<limites.alto;y+=300)for(let x=150;x<limites.ancho;x+=300){const p={x,y};
  if(valido(p)&&evaluarForestal(bosque,elevacion,p).densidad>=.15)candidatos.push({p,area:areaRegionalEn(elevacion.regional!,p).id});
 }
 const metales=salida.filter(r=>r.tipo==='estano');
 for(const nodo of metales){
  const locales:Point[]=[];
  for(let y=-60;y<=60;y+=30)for(let x=-60;x<=60;x+=30){const p={x:nodo.posicion.x+x,y:nodo.posicion.y+y};
   if(valido(p)&&evaluarForestal(bosque,elevacion,p).densidad>=.15&&areaRegionalEn(elevacion.regional!,p).id===nodo.area)locales.push(p);
  }
  const opciones=[...locales,...candidatos.filter(c=>c.area===nodo.area).map(c=>c.p)]
   .sort((a,b)=>distancia(a,nodo.posicion)-distancia(b,nodo.posicion));
  for(const centro of opciones){
   if(metales.some(r=>r.id!==nodo.id&&distancia(r.posicion,centro)<1100))continue;
   if(salida.some(r=>r.id!==nodo.id&&distancia(r.posicion,centro)<45))continue;
   let piedra=salida.find(r=>r.tipo==='piedra'&&distancia(r.posicion,centro)<80);
   if(!piedra){
    let posicion:Point|undefined;
    for(let i=0;i<24;i++){
     const a=i*Math.PI/12,p={x:centro.x+65*Math.cos(a),y:centro.y+65*Math.sin(a)};
     if(valido(p,.35)&&salida.every(r=>r.id===nodo.id||distancia(r.posicion,p)>=45)){posicion=p;break;}
    }
    if(!posicion)continue;
    piedra={id:`piedra-enclave-${nodo.id}`,tipo:'piedra',posicion,area:areaRegionalEn(elevacion.regional!,posicion).id,calidad:.4};
    salida.push(piedra);
   }
   nodo.posicion=centro;
   nodo.enclaveInicial={centro,madera:centro,piedraId:piedra.id,radio:90};
   break;
  }
 }
 return salida;
}

