// The messages of pi's codemode VM contract, structurally the same as pi-private's
// packages/codemode/src/runtime/protocol.ts (pi d65de9db942d), so this adapter needs no
// import from pi. Keep them in sync with pi when its protocol changes.

export interface OutputItemText {
  type: "text";
  text: string;
  console?: boolean;
}

export interface OutputItemImage {
  type: "image";
  data: string;
  mimeType: string;
}

export type OutputItem = OutputItemText | OutputItemImage;

/** Everything a VM needs to run one script (pi's `VmStartData`). */
export interface VmStartData {
  code: string;
  tools: { name: string; jsName: string; description: string }[];
  globals: { name: string; spread: boolean }[];
  memoryLimitBytes: number | undefined;
  /** Snapshot for `load()`: key to JSON text. */
  store: Record<string, string>;
  deterministic?: { startedAt: number; seed: number };
}

export type WorkerToHostMessage =
  | { type: "call"; id: number; target: "tool" | "global"; name: string; args: string | undefined }
  | { type: "output"; item: OutputItem }
  | { type: "done"; ok: true; value: string | undefined; writes: string; during?: { seq: number; issued: number } }
  | { type: "done"; ok: false; error: string; during?: { seq: number; issued: number } }
  | { type: "crash"; message: string }
  | { type: "started"; fingerprint: string }
  | { type: "idle"; seq: number; issued: number };

export interface HostToWorkerMessage {
  type: "result";
  id: number;
  ok: boolean;
  payload: string | undefined;
  seq?: number;
  now?: number;
  properties?: string;
}
