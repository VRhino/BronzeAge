import {describe,it,expect} from 'vitest';
import {definirBloquesUnity,muestrearBloqueUnity} from '../../scripts/unity-regional.datos';
describe('contrato de alturas Unity regional',()=>{
 it('cubre el rectángulo con bloques completos y rechaza resoluciones incompatibles',()=>{
  const b=definirBloquesUnity(32000,24000);expect(b).toHaveLength(48);
  expect(b.at(-1)).toMatchObject({x:28000,z:20000,ancho:4000,alto:4000});
  expect(()=>definirBloquesUnity(32000,24000,3000)).toThrow();
  expect(()=>definirBloquesUnity(32000,24000,4000,128)).toThrow();
 });
 it('preserva bordes compartidos, orientación y cuantización de 16 bits',()=>{
  const b=definirBloquesUnity(8000,8000,4000,33),altura=(x:number,z:number)=>100+x*.02+z*.04;
  const a=muestrearBloqueUnity(b[0]!,altura),derecha=muestrearBloqueUnity(b[1]!,altura),sur=muestrearBloqueUnity(b[2]!,altura);
  for(let i=0;i<33;i++){
   expect(a.raw.readUInt16LE((i*33+32)*2)).toBe(derecha.raw.readUInt16LE(i*33*2));
   expect(a.raw.readUInt16LE((32*33+i)*2)).toBe(sur.raw.readUInt16LE(i*2));
  }
  expect(a.raw.readUInt16LE(0)).toBe(Math.round(100/1000*65535));
  expect(a.raw.readUInt16LE(32*2)).toBeLessThan(a.raw.readUInt16LE(32*33*2));
  expect(a.errorMaximoCentrosMetros).toBeLessThanOrEqual(1000/65535/2+1e-8);
  expect(()=>muestrearBloqueUnity(b[0]!,()=>NaN)).toThrow();
 });
});
