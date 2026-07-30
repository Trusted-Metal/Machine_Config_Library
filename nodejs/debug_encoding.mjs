import * as h5wasm from "h5wasm/node";
await h5wasm.ready;
const f = new h5wasm.File("../fixtures/reference_config.h5", "r");
const trainGrp = f.get("Machine/Optical_Trains/Optical_Train_01");
const a = trainGrp.attrs;
const attr = a["Beam_Waist_Major_unit"];
const v = attr.value;
console.log("type:", typeof v);
console.log("JSON.stringify:", JSON.stringify(v));
if (typeof v === "string") {
  const codes = [...v].map(c => c.codePointAt(0).toString(16).padStart(4, "0")).join(" ");
  console.log("code points:", codes);
  // Try re-encoding as UTF-8 bytes from Latin-1 interpretation
  const bytes = Uint8Array.from(v, c => c.charCodeAt(0));
  const decoded = new TextDecoder("utf-8", { fatal: false }).decode(bytes);
  console.log("utf8-decoded:", JSON.stringify(decoded));
}
f.close();
