// Ficheros dorados: el JSON es la fuente; el .bin es la codificación canónica. `GOLDEN_UPDATE=1` regenera los .bin.
// Rust y C# comprueban los mismos .bin (ver crates/contrato/tests y contratos/cs).
import { expect, test } from "vitest";
import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { fromJson, toBinary, fromBinary, toJson } from "@bufbuild/protobuf";
import * as session from "./gen/bronzeage/v1/session_pb.js";
import * as world from "./gen/bronzeage/v1/world_pb.js";

const dir = new URL("../golden/", import.meta.url);
const esquemas: Record<string, any> = {
  MensajeDeCliente: session.MensajeDeClienteSchema,
  MensajeDeServidor: session.MensajeDeServidorSchema,
  ArtefactoDeMundo: world.ArtefactoDeMundoSchema,
};

for (const f of readdirSync(dir).filter((n) => n.endsWith(".json"))) {
  const nombre = f.replace(/\.json$/, "");
  test(nombre, () => {
    const schema = esquemas[nombre.split("__")[0]];
    const msg = fromJson(schema, JSON.parse(readFileSync(new URL(f, dir), "utf8")));
    const bin = toBinary(schema, msg);
    const binPath = new URL(`${nombre}.bin`, dir);
    if (process.env.GOLDEN_UPDATE) writeFileSync(binPath, bin);
    expect(Buffer.from(bin).equals(readFileSync(binPath))).toBe(true);
    expect(toJson(schema, fromBinary(schema, bin))).toEqual(toJson(schema, msg));
  });
}
