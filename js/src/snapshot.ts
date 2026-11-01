// The snapshot container (D2260): a VM image, collected and compressed with zstd against a
// reference image both sides can produce (the base snapshot in the module, or pi's image
// taken after its prelude), behind a header that names the engine build and the
// reference. The Sandbox (`bytes()`/`fromBytes()`) and the pi adapter
// (`snapshot()`/`resumeVm()`) write and read it. docs/host.md "Snapshots" has the layout.
//
// Images load only into the build that wrote them, so the header is checked before
// anything else: a snapshot of another build is refused with the build hashes in the
// message, and so is one whose reference this host cannot produce.

/** "DJSN", little-endian. */
export const SNAPSHOT_MAGIC = 0x4e534a44;
export const SNAPSHOT_FORMAT = 1;
const HEADER = 40;

/** Who wrote a snapshot: what its metadata means. */
export const APP_SANDBOX = 1;
export const APP_PI_SESSION = 2;

/** Reference images. */
export const REFERENCE_NONE = 0;
/** The base snapshot of the module (`djs_base_ptr`, `djs_base_len`). */
export const REFERENCE_BASE = 1;
/** pi's prelude image: the session's VM right after pi's prelude, collected. */
export const REFERENCE_PI_PRELUDE = 2;

export const COMPRESSION_NONE = 0;
export const COMPRESSION_ZSTD = 1;

/** zstd level of `encodeSnapshot` (D2261): fast enough to snapshot while a script waits. */
export const DEFAULT_ZSTD_LEVEL = 3;

/** A reference image and its FNV-1a 64 hash. */
export interface Reference {
  kind: number;
  bytes: Uint8Array;
  /** `fnv1a64(bytes)` as 16 hex digits. */
  hash: string;
}

/**
 * zstd with a raw-content dictionary. node's `zlib` has one (`nodeZstd()`); a browser can
 * bring its own (a zstd build in wasm), or snapshots stay uncompressed.
 */
export interface SnapshotCodec {
  compress(data: Uint8Array, dictionary: Uint8Array, level: number): Uint8Array;
  /** `size` is the uncompressed length the header promises. */
  decompress(data: Uint8Array, dictionary: Uint8Array, size: number): Uint8Array;
}

/** Why a snapshot cannot be restored. */
export class SnapshotError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "SnapshotError";
  }
}

export interface SnapshotHeader {
  format: number;
  app: number;
  compression: number;
  build: number;
  referenceKind: number;
  /** 16 hex digits; "0000000000000000" without a reference. */
  referenceHash: string;
  referenceLength: number;
  imageLength: number;
  meta: unknown;
  payload: Uint8Array;
}

interface ZlibLike {
  zstdCompressSync?(data: Uint8Array, options: object): Uint8Array;
  zstdDecompressSync?(data: Uint8Array, options: object): Uint8Array;
  constants: Record<string, number>;
}

let nodeCodec: SnapshotCodec | null | undefined;

/**
 * node's built-in zstd (node 22.15 and 23.8 on) as a codec, found without importing
 * anything, so this module also loads in a browser; `null` where there is none.
 */
export function nodeZstd(): SnapshotCodec | null {
  if (nodeCodec !== undefined) return nodeCodec;
  const process = (globalThis as { process?: { getBuiltinModule?(id: string): unknown } }).process;
  const zlib = process?.getBuiltinModule?.("node:zlib") as ZlibLike | undefined;
  if (zlib?.zstdCompressSync === undefined || zlib.zstdDecompressSync === undefined) return (nodeCodec = null);
  const { zstdCompressSync, zstdDecompressSync, constants: c } = zlib as Required<ZlibLike>;
  nodeCodec = {
    compress: (data, dictionary, level) =>
      zstdCompressSync(data, {
        dictionary,
        params: {
          [c.ZSTD_c_compressionLevel]: level,
          // The reference is history the image matches against: the window must reach
          // across it and the image (D2261).
          [c.ZSTD_c_windowLog]: windowLog(dictionary.length + data.length),
          [c.ZSTD_c_checksumFlag]: 1,
        },
      }),
    decompress: (data, dictionary, size) =>
      zstdDecompressSync(data, { dictionary, maxOutputLength: Math.max(size, 1) }),
  };
  return nodeCodec;
}

/** The smallest window log (zstd allows 10 to 31) that covers `n` bytes. */
function windowLog(n: number): number {
  let log = 10;
  while (log < 31 && 2 ** log < n) log++;
  return log;
}

/** FNV-1a 64 of `bytes` as 16 hex digits (the hash of `Vm::canonical_hash`). */
export function fnv1a64(bytes: Uint8Array): string {
  // 64-bit arithmetic in two 32-bit halves: the prime is 2^40 + 435.
  let hi = 0xcbf29ce4;
  let lo = 0x84222325;
  for (let i = 0; i < bytes.length; i++) {
    lo = (lo ^ bytes[i]) >>> 0;
    const low = lo * 435;
    const carry = Math.floor(low / 0x1_0000_0000);
    hi = (Math.imul(hi, 435) + ((lo << 8) >>> 0) + carry) >>> 0;
    lo = low >>> 0;
  }
  return hi.toString(16).padStart(8, "0") + lo.toString(16).padStart(8, "0");
}

const references = new WeakMap<object, Reference>();

/**
 * The base snapshot of the module that `exports` belong to, as a reference (a copy, hashed
 * once per module). Undefined for a module without `djs_base_ptr` (older than the
 * container): its snapshots are written uncompressed.
 */
export function baseReference(
  module: object,
  exports: { memory: WebAssembly.Memory; djs_base_ptr?(): number; djs_base_len?(): number },
): Reference | undefined {
  let ref = references.get(module);
  if (ref === undefined) {
    if (exports.djs_base_ptr === undefined || exports.djs_base_len === undefined) return undefined;
    const bytes = new Uint8Array(exports.memory.buffer, exports.djs_base_ptr(), exports.djs_base_len()).slice();
    ref = { kind: REFERENCE_BASE, bytes, hash: fnv1a64(bytes) };
    references.set(module, ref);
  }
  return ref;
}

export interface EncodeOptions {
  app: number;
  build: number;
  meta: unknown;
  /** The collected image. */
  image: Uint8Array;
  /** Compress against it. Without one, or without a codec, the image is stored as is. */
  reference?: Reference;
  /** `nodeZstd()` by default; `null` stores the image uncompressed. */
  codec?: SnapshotCodec | null;
  level?: number;
}

/** A snapshot container (D2260). */
export function encodeSnapshot(o: EncodeOptions): Uint8Array {
  const codec = o.codec === undefined ? nodeZstd() : o.codec;
  const ref = codec === null ? undefined : o.reference;
  const payload = ref === undefined ? o.image : codec!.compress(o.image, ref.bytes, o.level ?? DEFAULT_ZSTD_LEVEL);
  const meta = new TextEncoder().encode(JSON.stringify(o.meta));
  const out = new Uint8Array(HEADER + meta.length + payload.length);
  const view = new DataView(out.buffer);
  view.setUint32(0, SNAPSHOT_MAGIC, true);
  view.setUint16(4, SNAPSHOT_FORMAT, true);
  view.setUint8(6, o.app);
  view.setUint8(7, ref === undefined ? COMPRESSION_NONE : COMPRESSION_ZSTD);
  view.setUint32(8, o.build >>> 0, true);
  view.setUint8(12, ref?.kind ?? REFERENCE_NONE);
  const hash = ref?.hash ?? "0".repeat(16);
  view.setUint32(16, parseInt(hash.slice(8), 16), true);
  view.setUint32(20, parseInt(hash.slice(0, 8), 16), true);
  view.setUint32(24, ref?.bytes.length ?? 0, true);
  view.setUint32(28, o.image.length, true);
  view.setUint32(32, meta.length, true);
  view.setUint32(36, payload.length, true);
  out.set(meta, HEADER);
  out.set(payload, HEADER + meta.length);
  return out;
}

/** The header of a container, or undefined if `bytes` is not one (an older format). */
export function readSnapshotHeader(bytes: Uint8Array): SnapshotHeader | undefined {
  if (bytes.length < 8) return undefined;
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  if (view.getUint32(0, true) !== SNAPSHOT_MAGIC) return undefined;
  const format = view.getUint16(4, true);
  if (format !== SNAPSHOT_FORMAT) {
    throw new SnapshotError(`snapshot container format ${format}; this host reads format ${SNAPSHOT_FORMAT}`);
  }
  if (bytes.length < HEADER) throw new SnapshotError("truncated snapshot");
  const metaLength = view.getUint32(32, true);
  const payloadLength = view.getUint32(36, true);
  if (bytes.length !== HEADER + metaLength + payloadLength) throw new SnapshotError("truncated snapshot");
  const hex = (n: number) => n.toString(16).padStart(8, "0");
  let meta: unknown;
  try {
    meta = JSON.parse(new TextDecoder().decode(bytes.subarray(HEADER, HEADER + metaLength)));
  } catch {
    throw new SnapshotError("corrupt snapshot: its metadata is not JSON");
  }
  return {
    format,
    app: view.getUint8(6),
    compression: view.getUint8(7),
    build: view.getUint32(8, true),
    referenceKind: view.getUint8(12),
    referenceHash: hex(view.getUint32(20, true)) + hex(view.getUint32(16, true)),
    referenceLength: view.getUint32(24, true),
    imageLength: view.getUint32(28, true),
    meta,
    payload: bytes.subarray(HEADER + metaLength),
  };
}

const APP_NAMES: Record<number, string> = { [APP_SANDBOX]: "a sandbox snapshot", [APP_PI_SESSION]: "a pi session snapshot" };
const REFERENCE_NAMES: Record<number, string> = {
  [REFERENCE_BASE]: "the base image",
  [REFERENCE_PI_PRELUDE]: "pi's prelude image",
};

export const buildHex = (build: number): string => (build >>> 0).toString(16).padStart(8, "0");

/**
 * Checks a container against this host before anything is decompressed or restored: the
 * app, then the engine build (`build`, from `djs_build_hash()`).
 */
export function checkSnapshot(header: SnapshotHeader, app: number, build: number): void {
  if (header.app !== app) {
    throw new SnapshotError(`this is ${APP_NAMES[header.app] ?? `a snapshot of app ${header.app}`}, not ${APP_NAMES[app]}`);
  }
  if (header.build >>> 0 !== build >>> 0) {
    throw new SnapshotError(
      `snapshot from djs build ${buildHex(header.build)}; this module is build ${buildHex(build)}: ` +
        "an image restores only into the engine build that wrote it",
    );
  }
}

/**
 * The image of a checked container. `reference` is the reference the header names, if this
 * host has it (the caller looks it up by kind and hash).
 */
export function snapshotImage(
  header: SnapshotHeader,
  reference: Reference | undefined,
  codec: SnapshotCodec | null | undefined,
): Uint8Array {
  if (header.compression === COMPRESSION_NONE) {
    if (header.payload.length !== header.imageLength) throw new SnapshotError("truncated snapshot");
    return header.payload;
  }
  if (header.compression !== COMPRESSION_ZSTD) throw new SnapshotError(`unknown snapshot compression ${header.compression}`);
  const kind = REFERENCE_NAMES[header.referenceKind] ?? `reference ${header.referenceKind}`;
  if (reference === undefined || reference.kind !== header.referenceKind) {
    throw new SnapshotError(`the snapshot is compressed against ${kind} ${header.referenceHash}, which this host does not have`);
  }
  if (reference.hash !== header.referenceHash || reference.bytes.length !== header.referenceLength) {
    throw new SnapshotError(
      `the snapshot is compressed against ${kind} ${header.referenceHash}; this host's is ${reference.hash}`,
    );
  }
  const c = codec === undefined ? nodeZstd() : codec;
  if (c === null) {
    throw new SnapshotError("the snapshot is zstd-compressed, and this host has no zstd (node:zlib) or snapshot codec");
  }
  let image: Uint8Array;
  try {
    image = c.decompress(header.payload, reference.bytes, header.imageLength);
  } catch (e) {
    throw new SnapshotError(`corrupt snapshot: ${(e as Error).message}`);
  }
  if (image.length !== header.imageLength) throw new SnapshotError("corrupt snapshot: wrong image length");
  return image;
}
