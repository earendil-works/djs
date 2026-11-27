// ES modules (0241, D3171): the evaluation of a linked module graph and the traps of
// module namespace objects. Loading and linking are leaf work in Rust (interp/module.rs);
// evaluation calls the module bodies, so it is here.
//
// A module record is a ModuleRecord (below; interp/module.rs makes them).
//
// @embed-only: left out of the preludes of builds without `embed` (D3175).

/**
 * A module record: its key (a string), its status, the module closure (its template is
 * the body, its captures the environment; a synthetic module's is a SyntheticBody), the
 * requested module records (a RequestedModules, once loaded) and their count, the
 * evaluation error of an errored module, [[DFSIndex]] and [[DFSAncestorIndex]] while it
 * evaluates, the namespace object once made, the metadata (compiler/module_meta.rs, an
 * internal U32Array), the record under it on the evaluation stack (or null), a Box
 * holding the namespace (what `import * as` and `export * as` bind) once made, its
 * flags, [[CycleRoot]] (once its strongly connected component is done, or null),
 * [[AsyncEvaluationOrder]] (0 unset, -1 done, else the order), [[PendingAsyncDependencies]],
 * [[AsyncParentModules]] (an Array of records, or null), [[TopLevelCapability]] (the
 * promise of an Evaluate of it, or undefined).
 * @record
 */
enum ModuleRecord {
  key,
  /** @enum ModuleStatus */
  status,
  body,
  requested,
  count,
  error,
  dfs,
  ancestor,
  namespace,
  meta,
  stack,
  nsBox,
  /** @flags ModuleFlags */
  flags,
  cycleRoot,
  asyncOrder,
  pending,
  parents,
  capability,
}

/** [[Status]] of a module record. @constants */
enum ModuleStatus {
  new = 0,
  linking = 1,
  linked = 2,
  evaluating = 3,
  evaluated = 4,
  /** Evaluated with an error (in `error`). */
  errored = 5,
  /** evaluating-async: its SCC is done, an async module in it (or below) is not. */
  evaluatingAsync = 6,
}

/** A module record's flags. @flags */
enum ModuleFlags {
  /** A JSON module (a synthetic module with the parsed text as its default export). */
  json = 1,
  /** [[HasTLA]]: the body is an async function (top-level await). */
  tla = 2,
}

/** A module's requested module records, by request index (interp/module.rs). @record @rest modules */
enum RequestedModules {
}

/**
 * A synthetic module's body: the Boxes of its exports, in export order
 * (interp/module.rs).
 * @record
 * @rest boxes
 */
enum SyntheticBody {
}

/** The realm's module registry (`Intrinsic::Modules`, interp/module.rs). @record */
enum ModuleRegistry {
  /** A null-prototype object from key atom to module record. */
  registry,
  /** The handler of every namespace proxy. */
  traps,
  /**
   * The async host function `import()` calls (fn id `vm::IMPORT_FN_ID`), or undefined:
   * then `import()` rejects as without modules (builtins/import.ts).
   */
  import,
  /** The last [[AsyncEvaluationOrder]] given (IncrementModuleAsyncEvaluationCount). */
  asyncCount,
}

/** The state of an Evaluate: the top of the stack, the next DFS index. @record */
enum EvaluationState {
  top,
  index,
}

/** What a ModuleReaction does (ModuleOp). @constants */
enum ModuleReactions {
  /** An async module's body fulfilled. */
  asyncFulfilled = 10,
  /** An async module's body rejected. */
  asyncRejected = 11,
  /** An import's namespace. */
  importNamespace = 12,
  /** An import's graph loaded ([`FinishImport`]). */
  importLoaded = 13,
  /** An import rejected. */
  importRejected = 14,
}

/** A reaction of module evaluation (ModuleClosure's data). @record */
enum ModuleReaction {
  /** @enum ModuleReactions */
  op,
  operand,
}

/**
 * Evaluate ( ), 16.2.1.5.3, of a linked module: the promise of the evaluation of its cycle
 * root, made once ([[TopLevelCapability]]). Sync modules settle it before it returns; a
 * graph with top-level await settles it when its last async module is done. An error is
 * kept by every module it reached and thrown again by later evaluations.
 */
function ModuleEvaluate(m: Obj): Obj {
  let module: Obj = m;
  const status: f64 = AsNumber(RecordGet(module, ModuleRecord.status));
  if (status === ModuleStatus.evaluated || status === ModuleStatus.errored || status === ModuleStatus.evaluatingAsync) {
    const root: Value = RecordGet(module, ModuleRecord.cycleRoot);
    if (root !== null) {
      module = AsObj(root);
    }
  }
  const existing: Value = RecordGet(module, ModuleRecord.capability);
  if (existing !== undefined) {
    return AsObj(existing);
  }
  const promise: Obj = PromiseCreate(RealmIntrinsic("PromisePrototype"));
  RecordSet(module, ModuleRecord.capability, promise);
  const state: Obj = NewRecord(EvaluationState);
  RecordSet(state, EvaluationState.top, null);
  RecordSet(state, EvaluationState.index, 0);
  try {
    InnerModuleEvaluation(module, state);
  } catch (e) {
    let top: Value = RecordGet(state, EvaluationState.top);
    while (top !== null) {
      const t: Obj = AsObj(top);
      RecordSet(t, ModuleRecord.status, ModuleStatus.errored);
      RecordSet(t, ModuleRecord.error, e);
      top = RecordGet(t, ModuleRecord.stack);
      RecordSet(t, ModuleRecord.stack, null);
    }
    RejectPromise(promise, e);
    return promise;
  }
  // Not async, or async and done already (an evaluated module evaluated again).
  if (AsNumber(RecordGet(module, ModuleRecord.asyncOrder)) <= 0) {
    FulfillPromise(promise, undefined);
  }
  return promise;
}

/** [[HasTLA]] of record m. */
function HasTLA(m: Obj): boolean {
  return (F64ToI32(AsNumber(RecordGet(m, ModuleRecord.flags))) & ModuleFlags.tla) !== 0;
}

/** A builtin closure of ModuleOp with op and its operand (module.ts's reactions). */
function ModuleClosure(op: i32, operand: Value, length: i32): Value {
  const data: Obj = NewRecord(ModuleReaction);
  RecordSet(data, ModuleReaction.op, op);
  RecordSet(data, ModuleReaction.operand, operand);
  return NsOp(8, data, length, undefined);
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
    PerformPromiseThen(
      AsObj(loaded),
      ModuleClosure(ModuleReactions.importLoaded, promise, 1),
      ModuleClosure(ModuleReactions.importRejected, promise, 1),
      undefined,
    );
  } catch (e) {
    RejectPromise(promise, e);
  }
  return promise;
}

/**
 * ContinueDynamicImport's fulfilled path, 16.2.1.10.1, once the host loaded and linked the
 * graph of key: evaluates it and resolves promise with its namespace (a thenable namespace
 * is adopted), or rejects it.
 */
function FinishImport(promise: Obj, key: Value): void {
  try {
    const m: Obj = AsObj(NsOp(5, key, undefined, undefined));
    const evaluated: Obj = ModuleEvaluate(m);
    PerformPromiseThen(evaluated, ModuleClosure(ModuleReactions.importNamespace, m, 0), undefined, promise);
  } catch (e) {
    RejectPromise(promise, e);
  }
}

/** InnerModuleEvaluation ( module, stack, index ), 16.2.1.5.3.1. */
function InnerModuleEvaluation(m: Obj, state: Obj): void {
  const status: f64 = AsNumber(RecordGet(m, ModuleRecord.status));
  if (status === ModuleStatus.errored) {
    throw RecordGet(m, ModuleRecord.error);
  }
  if (status !== ModuleStatus.linked) {
    // Evaluating (a cycle), evaluating-async or evaluated (synthetic modules too).
    return;
  }
  const index: f64 = AsNumber(RecordGet(state, EvaluationState.index));
  RecordSet(m, ModuleRecord.status, ModuleStatus.evaluating);
  RecordSet(m, ModuleRecord.dfs, index);
  RecordSet(m, ModuleRecord.ancestor, index);
  RecordSet(m, ModuleRecord.pending, 0);
  RecordSet(state, EvaluationState.index, index + 1);
  RecordSet(m, ModuleRecord.stack, RecordGet(state, EvaluationState.top));
  RecordSet(state, EvaluationState.top, m);
  const requested: Obj = AsObj(RecordGet(m, ModuleRecord.requested));
  const n: f64 = AsNumber(RecordGet(m, ModuleRecord.count));
  for (let i: f64 = 0; i < n; i++) {
    let r: Obj = AsObj(RecordGet(requested, F64ToI32(i)));
    InnerModuleEvaluation(r, state);
    if (AsNumber(RecordGet(r, ModuleRecord.status)) === ModuleStatus.evaluating) {
      const ancestor: f64 = AsNumber(RecordGet(r, ModuleRecord.ancestor));
      if (ancestor < AsNumber(RecordGet(m, ModuleRecord.ancestor))) {
        RecordSet(m, ModuleRecord.ancestor, ancestor);
      }
    } else {
      const root: Value = RecordGet(r, ModuleRecord.cycleRoot);
      if (root === null) {
        // A synthetic module: evaluated, never async.
        continue;
      }
      r = AsObj(root);
      if (AsNumber(RecordGet(r, ModuleRecord.status)) === ModuleStatus.errored) {
        throw RecordGet(r, ModuleRecord.error);
      }
    }
    if (AsNumber(RecordGet(r, ModuleRecord.asyncOrder)) > 0) {
      RecordSet(m, ModuleRecord.pending, AsNumber(RecordGet(m, ModuleRecord.pending)) + 1);
      let parents: Value = RecordGet(r, ModuleRecord.parents);
      if (parents === null) {
        parents = ArrayCreate(0);
        RecordSet(r, ModuleRecord.parents, parents);
      }
      ArrayPushDense(AsObj(parents), m);
    }
  }
  const pending: f64 = AsNumber(RecordGet(m, ModuleRecord.pending));
  if (pending > 0 || HasTLA(m)) {
    RecordSet(m, ModuleRecord.asyncOrder, NsOp(7, undefined, undefined, undefined));
    if (pending === 0) {
      ExecuteAsyncModule(m);
    }
  } else {
    Call(RecordGet(m, ModuleRecord.body), undefined);
  }
  if (AsNumber(RecordGet(m, ModuleRecord.ancestor)) === index) {
    // The root of a strongly connected component: it and everything above it are done.
    let done: boolean = false;
    while (!done) {
      const t: Obj = AsObj(RecordGet(state, EvaluationState.top));
      RecordSet(state, EvaluationState.top, RecordGet(t, ModuleRecord.stack));
      RecordSet(t, ModuleRecord.stack, null);
      RecordSet(t, ModuleRecord.status, AsNumber(RecordGet(t, ModuleRecord.asyncOrder)) === 0 ? ModuleStatus.evaluated : ModuleStatus.evaluatingAsync);
      RecordSet(t, ModuleRecord.cycleRoot, m);
      done = t === m;
    }
  }
}

/** ExecuteAsyncModule ( module ), 16.2.1.5.3.2: runs the async body of m. */
function ExecuteAsyncModule(m: Obj): void {
  const p: Value = Call(RecordGet(m, ModuleRecord.body), undefined);
  PerformPromiseThen(AsObj(p), ModuleClosure(ModuleReactions.asyncFulfilled, m, 0), ModuleClosure(ModuleReactions.asyncRejected, m, 1), undefined);
}

/**
 * GatherAvailableAncestors ( module, execList ), 16.2.1.5.3.3: the async parents of m that
 * have no pending dependency left, into list.
 */
function GatherAvailableAncestors(m: Obj, list: Obj): void {
  const parents: Value = RecordGet(m, ModuleRecord.parents);
  if (parents === null) {
    return;
  }
  const n: f64 = LengthOfArrayLike(AsObj(parents));
  for (let i: f64 = 0; i < n; i++) {
    const p: Obj = AsObj(Get(AsObj(parents), i));
    if (ListContains(list, p)) {
      continue;
    }
    if (AsNumber(RecordGet(AsObj(RecordGet(p, ModuleRecord.cycleRoot)), ModuleRecord.status)) === ModuleStatus.errored) {
      continue;
    }
    const pending: f64 = AsNumber(RecordGet(p, ModuleRecord.pending)) - 1;
    RecordSet(p, ModuleRecord.pending, pending);
    if (pending === 0) {
      ArrayPushDense(list, p);
      if (!HasTLA(p)) {
        GatherAvailableAncestors(p, list);
      }
    }
  }
}

/** Whether array list holds v. */
function ListContains(list: Obj, v: Value): boolean {
  const n: f64 = LengthOfArrayLike(list);
  for (let i: f64 = 0; i < n; i++) {
    if (Get(list, i) === v) {
      return true;
    }
  }
  return false;
}

/** AsyncModuleExecutionFulfilled ( module ), 16.2.1.5.3.4. */
function AsyncModuleExecutionFulfilled(m: Obj): void {
  if (AsNumber(RecordGet(m, ModuleRecord.status)) === ModuleStatus.errored) {
    return;
  }
  RecordSet(m, ModuleRecord.asyncOrder, -1);
  RecordSet(m, ModuleRecord.status, ModuleStatus.evaluated);
  const capability: Value = RecordGet(m, ModuleRecord.capability);
  if (capability !== undefined) {
    FulfillPromise(AsObj(capability), undefined);
  }
  const list: Obj = ArrayCreate(0);
  GatherAvailableAncestors(m, list);
  // Sorted by [[AsyncEvaluationOrder]] (insertion sort: lists are short).
  const n: f64 = LengthOfArrayLike(list);
  for (let i: f64 = 1; i < n; i++) {
    const x: Value = Get(list, i);
    const order: f64 = AsNumber(RecordGet(AsObj(x), ModuleRecord.asyncOrder));
    let j: f64 = i - 1;
    while (j >= 0 && AsNumber(RecordGet(AsObj(Get(list, j)), ModuleRecord.asyncOrder)) > order) {
      CreateDataPropertyOrThrow(list, j + 1, Get(list, j));
      j = j - 1;
    }
    CreateDataPropertyOrThrow(list, j + 1, x);
  }
  for (let i: f64 = 0; i < n; i++) {
    const x: Obj = AsObj(Get(list, i));
    const status: f64 = AsNumber(RecordGet(x, ModuleRecord.status));
    if (status === ModuleStatus.evaluated || status === ModuleStatus.errored) {
      continue;
    }
    if (HasTLA(x)) {
      ExecuteAsyncModule(x);
      continue;
    }
    try {
      Call(RecordGet(x, ModuleRecord.body), undefined);
    } catch (e) {
      AsyncModuleExecutionRejected(x, e);
      continue;
    }
    RecordSet(x, ModuleRecord.asyncOrder, -1);
    RecordSet(x, ModuleRecord.status, ModuleStatus.evaluated);
    const c: Value = RecordGet(x, ModuleRecord.capability);
    if (c !== undefined) {
      FulfillPromise(AsObj(c), undefined);
    }
  }
}

/** AsyncModuleExecutionRejected ( module, error ), 16.2.1.5.3.5. */
function AsyncModuleExecutionRejected(m: Obj, error: Value): void {
  if (AsNumber(RecordGet(m, ModuleRecord.status)) === ModuleStatus.errored) {
    return;
  }
  RecordSet(m, ModuleRecord.error, error);
  RecordSet(m, ModuleRecord.status, ModuleStatus.errored);
  RecordSet(m, ModuleRecord.asyncOrder, -1);
  // The module's own evaluation rejects before its parents' (the order test262's
  // rejection-order.js checks).
  const capability: Value = RecordGet(m, ModuleRecord.capability);
  if (capability !== undefined) {
    RejectPromise(AsObj(capability), error);
  }
  const parents: Value = RecordGet(m, ModuleRecord.parents);
  if (parents !== null) {
    const n: f64 = LengthOfArrayLike(AsObj(parents));
    for (let i: f64 = 0; i < n; i++) {
      AsyncModuleExecutionRejected(AsObj(Get(AsObj(parents), i)), error);
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
 * ([`ModuleEvaluate`]); a ModuleReaction is a reaction (ModuleClosure); otherwise it is a
 * namespace trap and
 * `this` (the builtin closure's data) the
 * internal method: 0 [[Get]] (10.4.6.8), 1 [[GetOwnProperty]] (10.4.6.5), 2
 * [[DefineOwnProperty]] (10.4.6.6), 3 [[OwnPropertyKeys]] (10.4.6.11), 4 [[Set]] (10.4.6.9:
 * always false).
 * @intrinsic
 */
function ModuleOp(this: Value, target: Value, key: Value, a: Value): Value {
  if (this === undefined) {
    return ImportCall(target, key, a);
  }
  if (IsObject(this)) {
    // A ModuleReaction.
    const r: Obj = AsObj(this);
    const op: f64 = AsNumber(RecordGet(r, ModuleReaction.op));
    const operand: Value = RecordGet(r, ModuleReaction.operand);
    if (op === ModuleReactions.asyncFulfilled) {
      AsyncModuleExecutionFulfilled(AsObj(operand));
    } else if (op === ModuleReactions.asyncRejected) {
      AsyncModuleExecutionRejected(AsObj(operand), target);
    } else if (op === ModuleReactions.importNamespace) {
      return NsOp(6, operand, undefined, undefined);
    } else if (op === ModuleReactions.importLoaded) {
      FinishImport(AsObj(operand), target);
    } else {
      RejectPromise(AsObj(operand), target);
    }
    return undefined;
  }
  const op: i32 = F64ToI32(AsNumber(this));
  if (op === 5) {
    return ModuleEvaluate(AsObj(target));
  }
  if (op === 4) {
    return false;
  }
  return NsOp(op, AsObj(target), key, a);
}
