import { evaluarElevacion, evaluarTerreno, gradienteElevacion } from '../src/elevacion';
import { evaluarForestal } from '../src/forestacionRegional';
import { evaluarSuelo } from '../src/suelosRegional';
import { areaRegionalEn } from '../src/regional';
import type { generarRegionalVisor } from './visor-worldgen.regional';

export type MundoRegional = ReturnType<typeof generarRegionalVisor>;
export interface BloqueUnity { id:string; columna:number; fila:number; x:number; z:number; ancho:number; alto:number; resolucion:number; resolucionCapas:number; alturas:string; capas:string }
export const CANALES_UNITY=['bosque','fertilidad','pastos','agua','region','pendiente','tipoForestal'] as const;
export function definirBloquesUnity(ancho:number,alto:number,lado=4000,resolucion=129,resolucionCapas=64):BloqueUnity[]{
 if(!Number.isFinite(lado)||lado<=0||ancho%lado||alto%lado)throw new Error('El lado de bloque debe dividir ancho y alto');
 if(![33,65,129,257,513,1025].includes(resolucion))throw new Error('Resolución de altura no admitida');
 if(![32,64,128,256,512].includes(resolucionCapas))throw new Error('Resolución de capas no admitida');
 const bloques:BloqueUnity[]=[];
 for(let fila=0;fila<alto/lado;fila++)for(let columna=0;columna<ancho/lado;columna++){
  const id=`tile-${columna}-${fila}`;bloques.push({id,columna,fila,x:columna*lado,z:fila*lado,ancho:lado,alto:lado,resolucion,resolucionCapas,alturas:`${id}.r16`,capas:`${id}.layers`});
 }
 return bloques;
}
export function muestrearBloqueUnity(b:BloqueUnity,altura:(x:number,z:number)=>number){
 const raw=Buffer.alloc(b.resolucion*b.resolucion*2);let errorMaximo=0;
 for(let y=0;y<b.resolucion;y++)for(let x=0;x<b.resolucion;x++){
  const h=altura(b.x+x*b.ancho/(b.resolucion-1),b.z+y*b.alto/(b.resolucion-1));
  if(!Number.isFinite(h)||h<0||h>1000)throw new Error('Altura fuera de [0,1000] m');
  raw.writeUInt16LE(Math.round(h/1000*65535),(y*b.resolucion+x)*2);
 }
 for(let y=0;y<b.resolucion-1;y++)for(let x=0;x<b.resolucion-1;x++){
  const i=y*b.resolucion+x,media=[i,i+1,i+b.resolucion,i+b.resolucion+1].reduce((s,j)=>s+raw.readUInt16LE(j*2)/65535*1000,0)/4;
  errorMaximo=Math.max(errorMaximo,Math.abs(media-altura(b.x+(x+.5)*b.ancho/(b.resolucion-1),b.z+(y+.5)*b.alto/(b.resolucion-1))));
 }
 return {raw,errorMaximoCentrosMetros:errorMaximo};
}
export function muestrearCapasUnity(m:MundoRegional,b:BloqueUnity){
 const raw=Buffer.alloc(b.resolucionCapas*b.resolucionCapas*CANALES_UNITY.length);
 for(let y=0;y<b.resolucionCapas;y++)for(let x=0;x<b.resolucionCapas;x++){
  const p={x:b.x+(x+.5)*b.ancho/b.resolucionCapas,y:b.z+(y+.5)*b.alto/b.resolucionCapas};
  const terreno=evaluarTerreno(m.elevacion,p),agua=terreno==='agua',g=gradienteElevacion(m.elevacion,p,2),pendiente=Math.hypot(g.dx,g.dy)*1000;
  const f=evaluarForestal(m.forestacion,m.elevacion,p),s=evaluarSuelo(m.suelos,m.elevacion,p,f.densidad);
  const valores=[f.densidad*255,s.fertilidad*255,s.pastos*255,agua?255:0,agua?0:areaRegionalEn(m.elevacion.regional!,p).id+1,
   Math.min(1,pendiente)*255,agua?0:['abierto','denso','ribera','montano'].indexOf(f.tipo)+1];
  valores.forEach((v,c)=>{raw[(y*b.resolucionCapas+x)*CANALES_UNITY.length+c]=Math.round(v);});
 }
 return raw;
}
export function crearDatosUnity(m:MundoRegional){
 const altura=(p:{x:number;y:number})=>evaluarElevacion(m.elevacion,p)*1000;
 return {
  costas:crearCostasUnity(m.elevacion),
  recursos:m.recursos.map(r=>({...r,x:r.posicion.x,z:r.posicion.y,alturaY:altura(r.posicion)})),
  rios:m.rios.map(r=>({...r,puntos:r.puntos.map(p=>({x:p.x,z:p.y,alturaY:altura(p)}))})),
  lagos:m.lagos.map(l=>({...l,centro:{x:l.centro.x,z:l.centro.y},alturaY:l.elevacionCentro*1000,contorno:l.contorno?.map(p=>({x:p.x,z:p.y}))})),
  regiones:m.areas, pasos:m.pasos, 
  camposCanonicos:{elevacion:m.elevacion,forestacion:m.forestacion,suelos:m.suelos},
  influenciaPropuesta:{radioInicial:90,radioMaximoPorNivel:{1:180,2:270,3:360,4:450,5:540}},
  nota:'Geografía experimental. Sin cantidadesIniciales calibradas, stock ni estado de partida. Las reglas autoritativas siguen en el motor; capas y alturas son para visualización.'
 };
}

/** Cliff faces come from canonical points and elevations, never Unity random placement. */
export function crearCostasUnity(elevacion:import('../src/types').CampoElevacion){
 const field=elevacion.costas;if(!field)return undefined;
 const point=(v:import('../src/costasRegional').VerticeCostero,d:number)=>{
  const p={x:v.x+v.nx*d,y:v.y+v.ny*d};return {x:p.x,z:p.y,alturaY:evaluarElevacion(elevacion,p)*1000+.8};
 };
 return {revision:field.revision,seed:field.seed,segmentos:field.tramos.filter(s=>s.tipo!=='playa').map(s=>({
  id:s.id,tipo:s.tipo,intensidad:(s.a.intensidad+s.b.intensidad)/2,
  puntos:[point(s.a,-8),point(s.b,-8),point(s.a,s.a.anchoFrente*.4),point(s.b,s.b.anchoFrente*.4),point(s.a,s.a.anchoFrente),point(s.b,s.b.anchoFrente)]
 }))};
}
