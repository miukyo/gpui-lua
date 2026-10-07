/* tslint:disable */
/* eslint-disable */

/**
 * Structured result of a [`run`] call: the program's captured `print` output
 * and, *separately*, any error text (an empty string when the run succeeded).
 *
 * Keeping the two apart — rather than concatenating them into one string the
 * caller then has to guess apart — is what lets the playground classify a run
 * correctly. With a single combined string the only signal available to
 * JavaScript was a content heuristic, which both *false-positived* (legitimate
 * output containing the word "error" — e.g. iterating `_G`, which has a global
 * literally named `error` — was painted as a failure) and *false-negatived* (a
 * compile error whose text lacked the magic words was reported as success).
 * `error` non-empty ⇔ the run failed; no scanning of `output` required.
 */
export class RunResult {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Error text, or the empty string when the run succeeded. In the browser
     * build this is a compile/load error message: a genuine *runtime* error
     * traps the WebAssembly instance (`panic = "abort"` on
     * `wasm32-unknown-unknown`) and is surfaced by the caller's trap handler,
     * so it never reaches here.
     */
    readonly error: string;
    /**
     * The script's captured `print` output (tab-separated arguments, one line
     * per `print`, each terminated by a newline).
     */
    readonly output: string;
}

/**
 * Type-check `source` with the analyzer (old solver) and return the
 * newline-joined `line: message` diagnostics, or [`NO_ERRORS`] when clean.
 */
export function check(source: string): string;

/**
 * Compile and execute `source` on a fresh sandboxed Luau VM, returning the
 * program's captured `print` output and any error text as separate fields of a
 * [`RunResult`].
 *
 * This is the browser counterpart of the crate's
 * [`execute_script`](crate::functions::execute_script::execute_script):
 * 两者共用 [`run_in_sandbox`] 骨架，唯一差别是本入口在沙箱冻结全局表之前把
 * `print` 覆写成捕获版（cpp `Web.cpp:64-69` 的 `setupState` 没有这一步）。
 */
export function run(source: string): RunResult;

/**
 * Module start hook. A Lua runtime error reaches this panic hook as a
 * `lua_exception` payload (`lua_d_throw` -> `panic_any`). We recover its message
 * (`what()` reads the error object off the still-intact stack — `panic=abort`
 * does not unwind) and hand it to JS *before* the abort traps the instance, so
 * runtime errors surface with their text instead of an opaque `unreachable`
 * trap. Any other panic keeps the `console.error` diagnostic.
 *
 * ## Hook ordering — why this is more than a single `set_hook`
 *
 * The VM installs its OWN process-wide hook ([`install_lua_exception_panic_hook`],
 * fired lazily on the first `lua_d_rawrunprotected` during state setup) that
 * *silently swallows* every `lua_exception` payload: those panics are its
 * `longjmp` emulation, not crashes, and the CLI must not print "thread panicked"
 * for a normal `error()`. That same swallowing, however, also hides the error
 * *message* — the VM hook `take_hook()`s whatever we install and then `return`s
 * early for `lua_exception`, so a naive hook here would never be reached.
 *
 * So we deliberately build the chain with OUR hook outermost (it runs first):
 *
 * ```text
 *   ours (lua_exception -> JS bridge)  ->  VM hook  ->  console_error_panic_hook
 * ```
 *
 * Force `console_error_panic_hook` as the base, force the VM hook to install on
 * top of it *now* (so its captured `previous` is the console hook, not ours),
 * then wrap that with our hook. A `lua_exception` is intercepted here and its
 * message forwarded to JS; any real Rust bug falls through to the VM hook, which
 * delegates to `console_error_panic_hook` unchanged.
 */
export function wasm_start(): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_runresult_free: (a: number, b: number) => void;
    readonly check: (a: number, b: number) => [number, number];
    readonly run: (a: number, b: number) => number;
    readonly runresult_error: (a: number) => [number, number];
    readonly runresult_output: (a: number) => [number, number];
    readonly wasm_start: () => void;
    readonly sysconf: (a: number) => number;
    readonly mprotect: (a: number, b: number, c: number) => number;
    readonly munmap: (a: number, b: number) => number;
    readonly mmap: (a: number, b: number, c: number, d: number, e: number, f: bigint) => number;
    readonly free: (a: number) => void;
    readonly malloc: (a: number) => number;
    readonly realloc: (a: number, b: number) => number;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
