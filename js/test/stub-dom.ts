// A small DOM for node tests of the docs site's live.js and snapview.js (docs-live.test.ts):
// elements parsed from HTML, the selectors those scripts use (tags, classes, ids,
// attributes, descendants), events that bubble, and the few properties they touch. Not a
// browser: layout, focus and editing commands are no-ops.

type Listener = (ev: StubEvent) => void;

export class StubEvent {
  type: string;
  target: StubElement | null = null;
  defaultPrevented = false;
  bubbles = true;
  constructor(type: string, init: Record<string, unknown> = {}) {
    this.type = type;
    Object.assign(this, init);
  }
  preventDefault(): void {
    this.defaultPrevented = true;
  }
}

const VOID = new Set(["br", "hr", "img", "input", "meta", "link", "wbr"]);

const decode = (s: string) => s.replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&quot;/g, '"').replace(/&#39;/g, "'").replace(/&amp;/g, "&");
const encode = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

export class StubText {
  parent: StubElement | null = null;
  data: string;
  constructor(data: string) {
    this.data = data;
  }
  get textContent(): string {
    return this.data;
  }
}

type Node = StubElement | StubText;

export class StubElement {
  tagName: string;
  attrs = new Map<string, string>();
  children: Node[] = [];
  parent: StubElement | null = null;
  listeners = new Map<string, Listener[]>();
  value = "";
  selectionStart = 0;
  selectionEnd = 0;
  scrollTop = 0;
  scrollLeft = 0;
  [on: `on${string}`]: unknown;

  constructor(tag: string) {
    this.tagName = tag.toUpperCase();
  }

  // ---- attributes
  getAttribute(n: string): string | null {
    return this.attrs.has(n) ? this.attrs.get(n)! : null;
  }
  setAttribute(n: string, v: string): void {
    this.attrs.set(n, String(v));
  }
  removeAttribute(n: string): void {
    this.attrs.delete(n);
  }
  hasAttribute(n: string): boolean {
    return this.attrs.has(n);
  }
  get id(): string {
    return this.getAttribute("id") ?? "";
  }
  set id(v: string) {
    this.setAttribute("id", v);
  }
  get className(): string {
    return this.getAttribute("class") ?? "";
  }
  set className(v: string) {
    this.setAttribute("class", v);
  }
  get hidden(): boolean {
    return this.hasAttribute("hidden");
  }
  set hidden(v: boolean) {
    if (v) this.setAttribute("hidden", "");
    else this.removeAttribute("hidden");
  }
  get open(): boolean {
    return this.hasAttribute("open");
  }
  set open(v: boolean) {
    const was = this.open;
    if (v) this.setAttribute("open", "");
    else this.removeAttribute("open");
    if (was !== v) this.dispatchEvent(new StubEvent("toggle", { bubbles: false }));
  }
  get href(): string {
    return this.getAttribute("href") ?? "";
  }
  set href(v: string) {
    this.setAttribute("href", v);
  }
  set download(v: string) {
    this.setAttribute("download", v);
  }
  get classList() {
    const get = () => new Set(this.className.split(/\s+/).filter(Boolean));
    const put = (s: Set<string>) => (this.className = [...s].join(" "));
    return {
      contains: (c: string) => get().has(c),
      add: (...cs: string[]) => put(new Set([...get(), ...cs])),
      remove: (...cs: string[]) => {
        const s = get();
        for (const c of cs) s.delete(c);
        put(s);
      },
      toggle: (c: string, on?: boolean) => {
        const s = get();
        const want = on ?? !s.has(c);
        if (want) s.add(c);
        else s.delete(c);
        put(s);
        return want;
      },
    };
  }
  get dataset(): Record<string, string> {
    const out: Record<string, string> = {};
    for (const [k, v] of this.attrs) if (k.startsWith("data-")) out[k.slice(5).replace(/-(\w)/g, (_, c) => c.toUpperCase())] = v;
    return out;
  }

  // ---- tree
  get textContent(): string {
    if (this.tagName === "TEXTAREA") return this.value;
    return this.children.map((c) => c.textContent).join("");
  }
  set textContent(s: string) {
    this.children = [];
    this.append(new StubText(String(s)));
  }
  get innerHTML(): string {
    return this.children.map(serialize).join("");
  }
  set innerHTML(html: string) {
    this.children = [];
    for (const c of parse(html)) this.append(c);
  }
  append(c: Node): void {
    c.parent = this;
    this.children.push(c);
  }
  after(n: StubElement): void {
    const p = this.parent!;
    n.parent = p;
    p.children.splice(p.children.indexOf(this) + 1, 0, n);
  }
  get nextElementSibling(): StubElement | null {
    const sibs = this.parent?.children ?? [];
    for (let k = sibs.indexOf(this) + 1; k < sibs.length; k++) if (sibs[k] instanceof StubElement) return sibs[k] as StubElement;
    return null;
  }
  *descendants(): Generator<StubElement> {
    for (const c of this.children) {
      if (c instanceof StubElement) {
        yield c;
        yield* c.descendants();
      }
    }
  }
  querySelectorAll(sel: string): StubElement[] {
    const groups = sel.split(",").map((g) => g.trim().split(/\s+/).map(compound));
    return [...this.descendants()].filter((e) => groups.some((g) => matchesChain(e, g, this)));
  }
  querySelector(sel: string): StubElement | null {
    return this.querySelectorAll(sel)[0] ?? null;
  }
  matches(sel: string): boolean {
    return sel.split(",").some((g) => matchesChain(this, g.trim().split(/\s+/).map(compound), null));
  }
  closest(sel: string): StubElement | null {
    for (let e: StubElement | null = this; e; e = e.parent) if (e.matches(sel)) return e;
    return null;
  }

  // ---- events
  addEventListener(type: string, f: Listener, _opts?: unknown): void {
    this.listeners.set(type, [...(this.listeners.get(type) ?? []), f]);
  }
  dispatchEvent(ev: StubEvent): boolean {
    ev.target ??= this;
    for (let e: StubElement | null = this; e; e = ev.bubbles ? e.parent : null) {
      for (const f of e.listeners.get(ev.type) ?? []) f(ev);
      const on = e[`on${ev.type}`];
      if (typeof on === "function") on.call(e, ev);
    }
    return !ev.defaultPrevented;
  }
  click(): void {
    this.dispatchEvent(new StubEvent("click"));
  }
  focus(): void {}
  select(): void {}
  scrollIntoView(): void {}
  setSelectionRange(a: number, b: number): void {
    this.selectionStart = a;
    this.selectionEnd = b;
  }
}

type Compound = { tag?: string; id?: string; classes: string[]; attrs: [string, string | null][] };

function compound(s: string): Compound {
  const c: Compound = { classes: [], attrs: [] };
  const re = /^[a-zA-Z][\w-]*|\.[\w-]+|#[\w-]+|\[([\w-]+)(?:="([^"]*)")?\]/g;
  for (const m of s.matchAll(re)) {
    const t = m[0];
    if (t[0] === ".") c.classes.push(t.slice(1));
    else if (t[0] === "#") c.id = t.slice(1);
    else if (t[0] === "[") c.attrs.push([m[1], m[2] ?? null]);
    else c.tag = t.toUpperCase();
  }
  return c;
}

function matchOne(e: StubElement, c: Compound): boolean {
  if (c.tag && e.tagName !== c.tag) return false;
  if (c.id && e.id !== c.id) return false;
  const cls = e.className.split(/\s+/);
  if (!c.classes.every((x) => cls.includes(x))) return false;
  return c.attrs.every(([n, v]) => (v === null ? e.hasAttribute(n) : e.getAttribute(n) === v));
}

/** `e` matches the last compound, and its ancestors (below `root`) the others in order. */
function matchesChain(e: StubElement, chain: Compound[], root: StubElement | null): boolean {
  if (!matchOne(e, chain.at(-1)!)) return false;
  let k = chain.length - 2;
  for (let a = e.parent; a && a !== root && k >= 0; a = a.parent) if (matchOne(a, chain[k])) k--;
  return k < 0;
}

function serialize(n: Node): string {
  if (n instanceof StubText) return encode(n.data);
  const attrs = [...n.attrs].map(([k, v]) => (v === "" ? ` ${k}` : ` ${k}="${v.replace(/"/g, "&quot;")}"`)).join("");
  const tag = n.tagName.toLowerCase();
  return VOID.has(tag) ? `<${tag}${attrs}>` : `<${tag}${attrs}>${n.children.map(serialize).join("")}</${tag}>`;
}

/** HTML as nodes: well-formed markup as the site writes it. */
export function parse(html: string): Node[] {
  const top = new StubElement("#root");
  let cur = top;
  const re = /<!--[\s\S]*?-->|<\/([a-zA-Z][\w-]*)\s*>|<([a-zA-Z][\w-]*)((?:\s+[\w:-]+(?:\s*=\s*"[^"]*")?)*)\s*\/?>|[^<]+|</g;
  for (const m of html.matchAll(re)) {
    const [all, close, open, attrs] = m;
    if (all.startsWith("<!--")) continue;
    if (close) {
      for (let e: StubElement | null = cur; e && e !== top; e = e.parent) {
        if (e.tagName === close.toUpperCase()) {
          cur = e.parent!;
          break;
        }
      }
    } else if (open) {
      const e = new StubElement(open);
      for (const a of (attrs ?? "").matchAll(/([\w:-]+)(?:\s*=\s*"([^"]*)")?/g)) e.attrs.set(a[1], decode(a[2] ?? ""));
      if (e.tagName === "TEXTAREA") e.value = "";
      cur.append(e);
      if (!VOID.has(open.toLowerCase()) && !all.endsWith("/>")) cur = e;
    } else {
      if (cur.tagName === "TEXTAREA") cur.value += decode(all);
      else cur.append(new StubText(decode(all)));
    }
  }
  const out = top.children;
  for (const c of out) c.parent = null;
  return out;
}

/** A document whose body is `html`. */
export function stubDocument(html: string) {
  const body = new StubElement("body");
  body.innerHTML = html;
  return {
    body,
    querySelector: (s: string) => body.querySelector(s),
    querySelectorAll: (s: string) => body.querySelectorAll(s),
    createElement: (t: string) => new StubElement(t),
    execCommand: () => false,
  };
}
