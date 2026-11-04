// scripts/dump-costs/cases/binary.js: a 1 MB buffer, a Float64Array and a view in flight.
const b64 = await tools.blob({});
const bytes = new Uint8Array(1 << 20);
for (let i = 0; i < bytes.length; i++) bytes[i] = b64.charCodeAt(i % b64.length) ^ (i & 0xff);
const samples = new Float64Array(100000).map((_, i) => Math.sin(i / 100));
const view = new DataView(bytes.buffer, 1024, 4096);
const status = await tools.status({});
let sum = 0;
for (let i = 0; i < view.byteLength; i++) sum += view.getUint8(i);
return { state: status.state, sum, mean: samples.reduce((a, b) => a + b, 0) / samples.length };
