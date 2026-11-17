import { expect, test } from "vitest";
import { create, toBinary, fromBinary } from "@bufbuild/protobuf";
import { MensajeDeClienteSchema } from "./gen/bronzeage/v1/session_pb.js";

test("ping ida y vuelta", () => {
  const m = create(MensajeDeClienteSchema, { cuerpo: { case: "ping", value: { clienteEnvioMs: 42n } } });
  expect(fromBinary(MensajeDeClienteSchema, toBinary(MensajeDeClienteSchema, m))).toEqual(m);
});
