// ES modules (0241, D3171): the evaluation of a linked module graph and the traps of
// module namespace objects. Loading and linking are leaf work in Rust (interp/module.rs);
// evaluation calls the module bodies, so it is here.
//
// A module record is an internal Record (interp/module.rs has the layout): slot 1 the
// status (2 linked, 3 evaluating, 4 evaluated, 5 evaluated with the error in slot 5),
// 2 the module body, 3 the requested modules (a Record of records) and 4 their count, 6
// and 7 [[DFSIndex]] and [[DFSAncestorIndex]], 10 the record under it on the evaluation
// stack.
//
// @embed-only: left out of the preludes of builds without `embed` (D3175).

/**
 * Evaluate ( ), 16.2.1.5.3, of a linked module, without top-level await: the promise of
 * the evaluation, settled when it returns. An error is kept by every module on the stack
 * and thrown again by later evaluations.
 */
function ModuleEvaluate(m: Obj): Obj {
  const promise: Obj = PromiseCreate(RealmIntrinsic("PromisePrototype"));
  try {
    EvaluateGraph(m);
  } catch (e) {
    RejectPromise(promise, e);
    return promise;
  }
  FulfillPromise(promise, undefined);
  return promise;
}

/** Evaluate's work: runs the graph of m, or throws the error it ends with. */
function EvaluateGraph(m: Obj): void {
  // [the top of the stack, the next DFS index]
  const state: Obj = NewRecord(2);
  RecordSet(state, 0, null);
  RecordSet(state, 1, 0);
  try {
    InnerModuleEvaluation(m, state);
  } catch (e) {
    let top: Value = RecordGet(state, 0);
    while (top !== null) {
      const t: Obj = AsObj(top);
      RecordSet(t, 1, 5);
      RecordSet(t, 5, e);
      top = RecordGet(t, 10);
      RecordSet(t, 10, null);
    }
    throw e;
  }
}

/**
 * EvaluateImportCall ( specifierExpression, optionsExpression ), 13.3.10.2, in builds with
 * modules: the compiler calls this (through ModuleOp) with the referrer, the key of the
 * module or the name of the script the call is in. The checks reject as in
 * builtins/import.ts; then HostLoadImportedModule is a call of the VM's async host
 * function for imports (`vm::IMPORT_FN_ID`), which the host answers with the key of the
 * graph it loaded and linked (or an error, which rejects), and FinishImport evaluates it
 * (ContinueDynamicImport). A VM whose host does not answer imports rejects as without
 * modules.
 */
function ImportCall(specifier: Value, options: Value, referrer: Value): Obj {
  const load: Value = NsOp(4, undefined, undefined, undefined);
  if (load === undefined) {
    return DynamicImport(specifier, options);
  }
  const promise: Obj = PromiseCreate(RealmIntrinsic("PromisePrototype"));
  try {
    const name: Str = ToString(specifier);
    // The `type` attribute (the only one djs supports), or undefined.
    let type: Value = undefined;
    if (options !== undefined) {
      if (!IsObject(options)) {
        throw NewTypeError("options must be an object");
      }
      const attributes: Value = Get(AsObj(options), "with");
      if (attributes !== undefined) {
        if (!IsObject(attributes)) {
          throw NewTypeError("options.with must be an object");
        }
        const entries: Obj = EnumerableOwnProperties(AsObj(attributes), 2);
        const n: f64 = LengthOfArrayLike(entries);
        let unsupported: Value = undefined;
        for (let i: f64 = 0; i < n; i++) {
          const entry: Obj = AsObj(Get(entries, i));
          const key: Value = Get(entry, 0);
          const value: Value = Get(entry, 1);
          if (!IsString(value)) {
            throw NewTypeError("module attribute values must be strings");
          }
          if (key === "type") {
            type = value;
          } else if (unsupported === undefined) {
            unsupported = key;
          }
        }
        if (unsupported !== undefined) {
          // AllImportAttributesSupported.
          throw NewSyntaxError("unsupported import attribute");
        }
      }
    }
    const loaded: Value = Call(load, undefined, name, referrer, type);
    const finish: Value = NsOp(7, undefined, undefined, undefined);
    PerformPromiseThen(AsObj(loaded), finish, undefined, promise);
  } catch (e) {
    RejectPromise(promise, e);
  }
  return promise;
}

/**
 * ContinueDynamicImport's fulfilled path, 16.2.1.10.1, once the host loaded and linked the
 * graph of key: evaluates it (a module evaluates once; an error is kept and thrown again)
 * and gives its namespace.
 */
function FinishImport(key: Value): Value {
  const m: Obj = AsObj(NsOp(5, key, undefined, undefined));
  EvaluateGraph(m);
  return NsOp(6, m, undefined, undefined);
}

/** InnerModuleEvaluation ( module, stack, index ), 16.2.1.5.3.1, without async modules. */
function InnerModuleEvaluation(m: Obj, state: Obj): void {
  const status: f64 = AsNumber(RecordGet(m, 1));
  if (status === 5) {
    throw RecordGet(m, 5);
  }
  if (status !== 2) {
    // Evaluating (a cycle) or evaluated.
    return;
  }
  const index: f64 = AsNumber(RecordGet(state, 1));
  RecordSet(m, 1, 3);
  RecordSet(m, 6, index);
  RecordSet(m, 7, index);
  RecordSet(state, 1, index + 1);
  RecordSet(m, 10, RecordGet(state, 0));
  RecordSet(state, 0, m);
  const requested: Obj = AsObj(RecordGet(m, 3));
  const n: f64 = AsNumber(RecordGet(m, 4));
  for (let i: f64 = 0; i < n; i++) {
    const r: Obj = AsObj(RecordGet(requested, F64ToI32(i)));
    InnerModuleEvaluation(r, state);
    if (AsNumber(RecordGet(r, 1)) === 3) {
      const ancestor: f64 = AsNumber(RecordGet(r, 7));
      if (ancestor < AsNumber(RecordGet(m, 7))) {
        RecordSet(m, 7, ancestor);
      }
    }
  }
  Call(RecordGet(m, 2), undefined);
  if (AsNumber(RecordGet(m, 7)) === index) {
    // The root of a strongly connected component: it and everything above it are done.
    let done: boolean = false;
    while (!done) {
      const t: Obj = AsObj(RecordGet(state, 0));
      RecordSet(state, 0, RecordGet(t, 10));
      RecordSet(t, 10, null);
      RecordSet(t, 1, 4);
      done = t === m;
    }
  }
}

// A module namespace object (10.4.6) is a Proxy over a non-extensible target with the
// same keys (so that the proxy invariants hold), which also holds the namespace's data in
// internal slots. Its handler's traps get, set, getOwnPropertyDescriptor, defineProperty
// and ownKeys are builtin closures of ModuleOp; the other internal methods (has,
// deleteProperty, getPrototypeOf, ...) are the target's, which answers them as a
// namespace does.

/**
 * The module builtins' entry for Rust and the compiler (one intrinsic: each costs the
 * shipped module table space): `this` undefined is `import(target, key)` from referrer a
 * ([`ImportCall`], the compiler's call); `this` 5 evaluates module record target
 * ([`ModuleEvaluate`]); 6 finishes an import of key target ([`FinishImport`], the
 * reaction); otherwise it is a namespace trap and `this` (the builtin closure's data) the
 * internal method: 0 [[Get]] (10.4.6.8), 1 [[GetOwnProperty]] (10.4.6.5), 2
 * [[DefineOwnProperty]] (10.4.6.6), 3 [[OwnPropertyKeys]] (10.4.6.11), 4 [[Set]] (10.4.6.9:
 * always false).
 * @intrinsic
 */
function ModuleOp(this: Value, target: Value, key: Value, a: Value): Value {
  if (this === undefined) {
    return ImportCall(target, key, a);
  }
  const op: i32 = F64ToI32(AsNumber(this));
  if (op === 5) {
    return ModuleEvaluate(AsObj(target));
  }
  if (op === 6) {
    return FinishImport(target);
  }
  if (op === 4) {
    return false;
  }
  return NsOp(op, AsObj(target), key, a);
}
