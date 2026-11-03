// Binary buffers in flight: a decoded 1 MB blob, a Float64Array of samples and a view.
const b64 = (await tools.shell({ cmd: "blob" })).stdout;
const bytes = new Uint8Array(1 << 20);
for (let i = 0; i < bytes.length; i++) bytes[i] = b64.charCodeAt(i % b64.length) ^ (i & 0xff);
const samples = new Float64Array(100000).map((_, i) => Math.sin(i / 100));
const view = new DataView(bytes.buffer, 1024, 4096);
const status = await tools.status({});
let sum = 0;
for (let i = 0; i < view.byteLength; i++) sum += view.getUint8(i);
return { state: status.state, sum, mean: samples.reduce((a, b) => a + b, 0) / samples.length };
