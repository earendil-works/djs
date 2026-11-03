/**
 * prelude-next: pi's codemode prelude (prelude-source.ts, pi d65de9db942d, verbatim there)
 * in the shape pi should move to on djs (D2565, docs/continuation-dumps.md "pi's future
 * shape"), for js/pi/vm.ts's `next` mode. Everything a waiting script has in flight is
 * then engine state (open host calls, suspended activations) or adapter state, never
 * prelude state, so a session persists as a continuation dump: the prelude is run again
 * on resume (it is deterministic, D2262) and the dump names its globals as host globals.
 *
 * Differences from pi's prelude, all of them:
 * - No `lockdown()`: the adapter's `next` mode makes the VM with frozen builtins (D2304),
 *   which do what it did (pi's tests pass the same, js/pi/pi-tests.ts --no-lockdown). Its
 *   accessors were prelude state on the builtins, which a dump cannot recreate; frozen
 *   builtins are part of the VM.
 * - Tools and globals are engine async host calls: `tools.x(args)` serializes its
 *   arguments as before and returns the promise of `call(kind, name, argsJson)`, an async
 *   host function the adapter provides (D1374). No `pending` map, no `nextId`; the adapter
 *   numbers the calls for pi's protocol. The adapter settles a call with the value
 *   `decode(payload)` returns, or rejects it with `failure(payload, propertiesJson)` (or
 *   with what `decode` threw): the same values pi's `settle` made.
 * - The body's promise is the adapter's: `run(fn)` returns it (a host handle) instead of
 *   reporting through `then` reactions; when it settles the adapter calls `encode(value)` or
 *   `describe(error)` and reports `done`. It does that when the job queue has run empty, so
 *   output a job queued before the body settled still counts (pi: `done` first).
 * - `finished`, the output counters and the store's contents and writes live in the
 *   adapter. `bridge("output", ...)`, `bridge("store", key, json)` and `bridge("load",
 *   key)` are sync host calls that answer: undefined, a limit's message, or the stored JSON.
 *   `done(ok, payload)` reports through `bridge("done", ok, payload)`; the adapter adds
 *   the writes. `storeJson` is no longer a parameter.
 * - `stalled()` is the adapter's (no tool call open, the body pending).
 *
 * Evaluates to `(bridge, call, toolsJson, globalsJson) => { run, decode, failure, encode,
 * describe }`.
 */
/**
 * JavaScript evaluated inside the QuickJS VM before the script runs.
 *
 * The VM is its own wasm instance, so nothing here guards a realm boundary.
 * The prelude keeps the host bridge in a closure so the script cannot call it
 * directly, and builds `tools`, `ALL_TOOLS`, the output helpers (`text`, `image`,
 * `exit`, `console`), and globals on top of it. Tool arguments and results cross
 * as JSON strings and are parsed on this side.
 *
 * Output helpers: `text(value)` appends a text item (non-strings are
 * JSON-stringified), `image(urlOrItem)` appends an image from a base64 `data:` URL, an
 * `{ image_url }` object, or an image block `{ type: "image", data, mimeType }`, and `exit()` ends the script
 * successfully. `console.*` appends text items marked as console output.
 *
 * `store(key, value)` and `load(key)` are synchronous: they work on a snapshot of
 * JSON text passed in as `storeJson`, and the keys the script wrote are reported
 * with a successful "done".
 *
 * Evaluates to a function `(bridge, toolsJson, globalsJson, storeJson) => { settle, run, stalled }`.
 * `settle(id, ok, payload, propertiesJson)` settles call `id`; a rejection's `Error` gets the own properties of the
 * `propertiesJson` object, except `name`, `message`, and `stack`.
 * `toolsJson` lists `{ name, jsName, description }`: `tools[jsName]` and `tools[name]` call the
 * tool, and `ALL_TOOLS` lists `{ name: jsName, description }`.
 * `stalled()` reports a script that has not finished while no host call is pending: with no timers
 * or I/O in the VM, nothing can ever resume it.
 * `globalsJson` lists `{ name, spread }`; `a.b` names are grouped into a frozen `a` object.
 * `bridge(kind, a, b, c)` with kind "call" or "global" (id, name, argsJson),
 * "output" ("text" or "console", text) or ("image", data, mimeType), or "done" (ok, valueJsonOrErrorJson, writesJson).
 *
 * (pi:) Before anything else the prelude freezes the built-ins and makes the built-in globals
 * read-only. In this shape the VM's frozen builtins do that.
 */
export const MAX_STORE_VALUE_CHARS = 256 * 1024;
export const MAX_STORE_TOTAL_CHARS = 1024 * 1024;
/**
 * Output one script may produce with `text()`, `image()`, and `console.*`: characters of text and
 * base64 image data, and items. The host keeps all output until the script ends, so without a
 * limit a script that prints in a loop grows the host's memory until it crashes. The item limit
 * covers loops that print empty strings.
 */
export const MAX_OUTPUT_CHARS = 16 * 1024 * 1024;
export const MAX_OUTPUT_ITEMS = 100_000;

const IMAGE_HELPER_EXPECTS =
	'image expects a non-empty image URL string, an object with image_url, or an image block { type: "image", data, mimeType }';

export const PRELUDE_NEXT: string = `(function (bridge, call, toolsJson, globalsJson) {
	"use strict";

	// No lockdown(): the VM has frozen builtins (D2304), which do its job.
	const stringify = JSON.stringify;
	const parse = JSON.parse;
	const ErrorCtor = Error;
	const TypeErrorCtor = TypeError;
	const RangeErrorCtor = RangeError;
	const promiseReject = Promise.reject.bind(Promise);
	// Thrown by exit() to unwind the script after it already reported success.
	const EXIT = Object.freeze({});

	function done(ok, payload) {
		bridge("done", ok, payload);
	}

	function serialize(value) {
		return value === undefined ? undefined : stringify(value);
	}

	// QuickJS stacks list frames only. Prefix "Name: message" like V8 so the
	// text reads the same as a Node error, and drop this prelude's frames.
	function errorText(error) {
		const head = error.message ? error.name + ": " + error.message : String(error.name);
		const frames =
			typeof error.stack === "string"
				? error.stack.split("\\n").filter((line) => line.trim() && !line.includes("codemode-prelude.js"))
				: [];
		return [head, ...frames].join("\\n");
	}

	function format(value) {
		if (typeof value === "string") return value;
		if (value instanceof ErrorCtor) return errorText(value);
		try {
			const json = stringify(value);
			return json === undefined ? String(value) : json;
		} catch {
			return String(value);
		}
	}

	// The host requires string fields. A script can set an error's name or message to anything, so
	// they are coerced, and a value whose coercion throws gets a placeholder.
	function describeError(error) {
		try {
			if (error instanceof ErrorCtor) {
				return stringify({ name: String(error.name), message: String(error.message), stack: errorText(error) });
			}
			return stringify({ message: String(format(error)) });
		} catch {
			return stringify({ message: "The script threw a value that cannot be described" });
		}
	}

	function caller(kind, name, spread) {
		return (...args) => {
			let json;
			try {
				json = serialize(spread ? args : args[0]);
			} catch (error) {
				return promiseReject(error);
			}
			return call(kind, name, json);
		};
	}

	const tools = Object.create(null);
	const allTools = [];
	for (const { name, jsName, description } of parse(toolsJson)) {
		const fn = caller("call", name);
		// The first tool wins when two names normalize to the same identifier.
		if (!(jsName in tools)) {
			tools[jsName] = fn;
			allTools.push(Object.freeze({ name: jsName, description }));
		}
		if (!(name in tools)) tools[name] = fn;
	}
	Object.freeze(tools);
	Object.freeze(allTools);

	// Reading a member that does not exist throws an error that names the close matches, instead of
	// a later "not a function". \`in\` checks still work.
	const comparable = (name) => name.toLowerCase().replace(/[^a-z0-9]/g, "");
	function guard(target, label, names, hint) {
		return new Proxy(target, {
			get(object, property, receiver) {
				if (typeof property !== "string" || property in object || property in Object.prototype || property === "then" || property === "toJSON") {
					return Reflect.get(object, property, receiver);
				}
				const wanted = comparable(property);
				const exact = names.filter((name) => comparable(name) === wanted);
				const close = exact.length > 0 ? exact : names.filter((name) => wanted && (comparable(name).includes(wanted) || wanted.includes(comparable(name))));
				let message = label + "." + property + " does not exist.";
				if (close.length > 0) message += " Did you mean " + close.slice(0, 5).map((name) => label + "." + name).join(", ") + "?";
				else if (names.length <= 20) message += " Available: " + names.join(", ") + ".";
				if (hint) message += " " + hint;
				message += ' Check for a member with "' + property + '" in ' + label + ".";
				throw new TypeErrorCtor(message);
			},
		});
	}
	const toolsProxy = guard(
		tools,
		"tools",
		allTools.map((tool) => tool.name),
		"ALL_TOOLS lists every tool; searchTools(query) finds tools by topic.",
	);

	const namespaces = new Map();
	for (const { name, spread } of parse(globalsJson)) {
		const fn = caller("global", name, spread);
		const dot = name.indexOf(".");
		if (dot === -1) {
			Object.defineProperty(globalThis, name, { value: fn, enumerable: true });
			continue;
		}
		const namespace = name.slice(0, dot);
		if (!namespaces.has(namespace)) namespaces.set(namespace, Object.create(null));
		namespaces.get(namespace)[name.slice(dot + 1)] = fn;
	}
	for (const [namespace, members] of namespaces) {
		Object.freeze(members);
		const value = guard(members, namespace, Object.keys(members));
		Object.defineProperty(globalThis, namespace, { value, enumerable: true });
	}

	const STORE_HINT =
		"store() is for small state such as IDs or summaries. Show images with image(), keep large data in variables, or write it to a file with a tool.";

	function checkKey(name, key) {
		if (typeof key !== "string") throw new TypeError(name + "() key must be a string");
	}

	// The contents, sizes and writes are the adapter's: it answers a limit's message.
	function store(key, value) {
		checkKey("store", key);
		if (value === undefined) {
			bridge("store", key);
			return;
		}
		let json;
		try {
			json = stringify(value);
		} catch (error) {
			throw new TypeError("store(" + stringify(key) + ") value is not JSON-serializable: " + format(error));
		}
		if (json === undefined) {
			throw new TypeError("store(" + stringify(key) + ") value is not JSON-serializable");
		}
		const limit = bridge("store", key, json);
		if (limit !== undefined) throw new RangeError(limit + STORE_HINT);
	}

	function load(key) {
		checkKey("load", key);
		const json = bridge("load", key);
		return json === undefined ? undefined : parse(json);
	}

	Object.defineProperty(globalThis, "store", { value: store, enumerable: true });
	Object.defineProperty(globalThis, "load", { value: load, enumerable: true });

	// Past the output limits the script fails: done() reports the error, so catching it does not
	// resume output, and the host ends the script. The adapter counts, and ignores output
	// after the script is done.
	function output(kind, data, mimeType) {
		const limit = bridge("output", kind, data, mimeType);
		if (limit === undefined) return;
		const error = new RangeErrorCtor(limit);
		done(false, describeError(error));
		throw error;
	}

	// Primitives become their string form, everything else JSON.
	function outputText(value) {
		if (value === undefined || value === null || typeof value !== "object" && typeof value !== "function") {
			return String(value);
		}
		const json = stringify(value);
		return json === undefined ? String(value) : json;
	}

	function text(value) {
		let rendered;
		try {
			rendered = outputText(value);
		} catch (error) {
			throw new TypeErrorCtor(error instanceof ErrorCtor ? error.message : String(error));
		}
		output("text", rendered);
	}

	function imageUrl(value) {
		if (typeof value === "string") return value;
		if (typeof value !== "object" || value === null || Array.isArray(value)) {
			throw new TypeErrorCtor(${JSON.stringify(IMAGE_HELPER_EXPECTS)});
		}
		if (value.image_url !== undefined) {
			if (typeof value.image_url !== "string") throw new TypeErrorCtor(${JSON.stringify(IMAGE_HELPER_EXPECTS)});
			return value.image_url;
		}
		if (typeof value.type !== "string") throw new TypeErrorCtor(${JSON.stringify(IMAGE_HELPER_EXPECTS)});
		if (value.type !== "image") {
			throw new TypeErrorCtor('image only accepts image blocks, got "' + value.type + '"');
		}
		if (typeof value.data !== "string" || value.data === "") throw new TypeErrorCtor("image expected image block data");
		if (value.data.toLowerCase().startsWith("data:")) return value.data;
		return "data:;base64," + value.data;
	}

	// Base64 of the signatures of the formats providers accept inline (PNG, JPEG except
	// JPEG-LS, GIF, "RIFF....WEBP"). Signatures start at byte 0, so their encodings are prefixes.
	const IMAGE_SIGNATURES = [
		["image/png", /^iVBORw0KGg/],
		["image/jpeg", /^[/]9j[/](?!9)/],
		["image/gif", /^R0lGOD[dl]h/],
		["image/webp", /^UklG.{8}RUJQ/],
	];

	function image(value) {
		const url = imageUrl(value);
		if (url === "") throw new TypeErrorCtor(${JSON.stringify(IMAGE_HELPER_EXPECTS)});
		const colon = url.indexOf(":");
		const scheme = colon === -1 ? "" : url.slice(0, colon).toLowerCase();
		if (scheme === "http" || scheme === "https") {
			throw new TypeErrorCtor("remote image URLs are not supported in tool outputs. Pass a base64 data URI instead");
		}
		const comma = url.indexOf(",");
		const header = comma === -1 ? [] : url.slice(colon + 1, comma).split(";");
		if (scheme !== "data" || comma === -1 || header.slice(1).every((part) => part.toLowerCase() !== "base64")) {
			throw new TypeErrorCtor("invalid image output. Pass a base64 data URI instead");
		}
		// Providers reject the whole request on a bad image, and a persisted image block would be
		// resent on every later turn. Line breaks from wrapped base64 are dropped. The declared type
		// is ignored in favor of the detected one, as providers also reject mismatches.
		const data = url.slice(comma + 1).replace(/\\s+/g, "");
		if (data.length % 4 !== 0 || !/^[A-Za-z0-9+/]+={0,2}$/.test(data)) {
			throw new TypeErrorCtor("invalid image output. The image data is not valid base64 (truncated or corrupted?)");
		}
		const head = data.slice(0, 16);
		const signature = IMAGE_SIGNATURES.find(([, pattern]) => pattern.test(head));
		if (!signature) {
			throw new TypeErrorCtor("invalid image output. The image data is not a PNG, JPEG, GIF, or WebP image");
		}
		output("image", data, signature[0]);
	}

	function exit() {
		done(true, undefined);
		throw EXIT;
	}

	const console = {};
	for (const level of ["log", "info", "warn", "error", "debug"]) {
		console[level] = (...args) => {
			output("console", args.map(format).join(" "));
		};
	}
	Object.freeze(console);

	Object.defineProperty(globalThis, "tools", { value: toolsProxy, enumerable: true });
	Object.defineProperty(globalThis, "ALL_TOOLS", { value: allTools, enumerable: true });
	Object.defineProperty(globalThis, "console", { value: console, enumerable: true });
	Object.defineProperty(globalThis, "text", { value: text, enumerable: true });
	Object.defineProperty(globalThis, "image", { value: image, enumerable: true });
	Object.defineProperty(globalThis, "exit", { value: exit, enumerable: true });

	return {
		run(fn) {
			try {
				return fn(toolsProxy, console);
			} catch (error) {
				done(false, describeError(error));
				return undefined;
			}
		},
		decode(payload) {
			return payload === undefined ? undefined : parse(payload);
		},
		failure(payload, propertiesJson) {
			const error = new ErrorCtor(payload);
			if (propertiesJson !== undefined) {
				const properties = parse(propertiesJson);
				for (const key of Object.keys(properties)) {
					if (key === "name" || key === "message" || key === "stack") continue;
					Object.defineProperty(error, key, { value: properties[key], writable: true, enumerable: true, configurable: true });
				}
			}
			return error;
		},
		encode(value) {
			return serialize(value);
		},
		describe(error) {
			return describeError(error);
		},
	};
})`;
