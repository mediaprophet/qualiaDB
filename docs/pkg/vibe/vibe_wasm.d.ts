/* tslint:disable */
/* eslint-disable */

/**
 * Persistent compiled cell for the browser binding.
 *
 * Keeps the compiled [`bytecode::Chunk`] so timed / repeated runs can call
 * [`CompiledCell::run`] without re-decoding VBC1 bytes. Prefer this over
 * [`decode_and_run`] when the job is "run what we already compiled."
 */
export class CompiledCell {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Compile a cell expression (`= expr`) into a handle. Does not run it.
     */
    static compile(src: string): CompiledCell;
    /**
     * Human-readable disassembly of the held chunk (inspect only).
     */
    disassembly(): string;
    /**
     * Decode a VBC1 byte buffer once into a handle. Later [`Self::run`] calls
     * do not decode again.
     */
    static from_bytes(bytes: Uint8Array): CompiledCell;
    /**
     * Run the held chunk on a fresh local VM. Does not decode.
     */
    run(): any;
    readonly code_size: number;
    readonly constants: number;
    readonly functions: number;
    readonly top_locals: number;
}

/**
 * Apply a structural edit to a VibeScript program and project the result.
 *
 * The edit is specified as a JSON object with an `op` field and
 * operation-specific fields. This enables LLMs and browsers to
 * edit program structure without text patching.
 *
 * Supported ops:
 * - `add_field`: { op, name, ty, unit?, support?, representation? }
 * - `add_material`: { op, name, properties: [{name, value}] }
 * - `add_law`: { op, name, condition, consequence }
 * - `remove_item`: { op, index }
 * - `rename_item`: { op, index, new_name }
 * - `set_field_unit`: { op, index, unit? }
 * - `set_field_support`: { op, index, support }
 * - `set_field_representation`: { op, index, representation }
 * - `add_material_property`: { op, index, name, value }
 * - `remove_material_property`: { op, index, name }
 * - `add_prefix`: { op, prefix, iri }
 * - `remove_prefix`: { op, prefix }
 */
export function apply_structural_edit(src: string, edit_json: string): any;

/**
 * Apply multiple structural edits in sequence.
 * `edits_json` is a JSON array of edit objects.
 */
export function apply_structural_edits(src: string, edits_json: string): any;

/**
 * Get the JSON schema for the AST.
 */
export function ast_schema_json(): string;

/**
 * Capability invoke pin — default fail-closed E300 (parity with Host::capability_invoke).
 * Args are accepted as a JSON string for the JS boundary.
 */
export function capability_invoke(id: string, _args_json: string): any;

/**
 * Check a cell expression.
 */
export function check_cell_src(src: string): any;

/**
 * Check a full program.
 */
export function check_program_src(src: string): any;

/**
 * Compile a cell expression to bytecode and return chunk metadata.
 *
 * For repeated execution without re-decode, use [`CompiledCell::compile`] and
 * [`CompiledCell::run`] instead. This export stays for playground inspect /
 * size reporting.
 */
export function compile_cell_bytecode(src: string): any;

/**
 * Decode a binary bytecode chunk and run it once.
 *
 * Compat wrapper: each call decodes again. Prefer [`CompiledCell::from_bytes`]
 * then [`CompiledCell::run`] when the same bytes will run more than once.
 */
export function decode_and_run(bytes: Uint8Array): any;

/**
 * Parse + check a module; collect up to eight diagnostics.
 */
export function diagnose_src(src: string): any;

/**
 * Get the JSON schema for diagnostics.
 */
export function diagnostic_schema_json(): string;

/**
 * Get the EBNF grammar string.
 */
export function ebnf_grammar(): string;

/**
 * Encode a cell's bytecode to a binary Uint8Array.
 */
export function encode_cell_bytecode(src: string): any;

/**
 * Evaluate a cell and return the result as a JSON-compatible JS value.
 * Playground Run uses `eval_program_src` (module + optional `main`), not this.
 */
export function eval_cell_json(src: string): any;

/**
 * Evaluate a cell expression (`= expr`) with the in-process local host.
 */
export function eval_cell_src(src: string): any;

/**
 * Evaluate a full program on LocalHost (workshop dialect).
 *
 * Runs preamble items, then `main` when present — same as `vibe eval FILE main`.
 */
export function eval_program_src(src: string): any;

/**
 * Get the GBNF grammar (for LLM constrained decoding).
 */
export function gbnf_grammar(): string;

/**
 * Frozen host ABI stamp (`vibe-host-0.1`).
 */
export function host_version(): string;

/**
 * Get the VibeScript language version string.
 */
export function language_version(): string;

/**
 * Parse a cell expression (`= expr`).
 * Returns `{ ok: true, ast: ... }` or `{ ok: false, error: ... }`.
 */
export function parse_cell_src(src: string): any;

/**
 * Parse a full VibeScript program (module).
 */
export function parse_program_src(src: string): any;

/**
 * Project a VibeScript program source to canonical form.
 * Parses the source, then re-projects it from the AST.
 * This is the core of projectional authoring: structure → text.
 */
export function project_source(src: string): any;

/**
 * Compile a cell to bytecode, run it on the VM, and return the result.
 */
export function run_cell_bytecode(src: string): any;

/**
 * Compile a program to bytecode, encode it to binary, decode it, and run
 * a named function.  Demonstrates the full bytecode round-trip.
 */
export function run_program_bytecode(src: string, fn_name: string, args: any[]): any;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_compiledcell_free: (a: number, b: number) => void;
    readonly apply_structural_edit: (a: number, b: number, c: number, d: number) => any;
    readonly apply_structural_edits: (a: number, b: number, c: number, d: number) => any;
    readonly ast_schema_json: () => [number, number];
    readonly capability_invoke: (a: number, b: number, c: number, d: number) => any;
    readonly check_cell_src: (a: number, b: number) => any;
    readonly check_program_src: (a: number, b: number) => any;
    readonly compile_cell_bytecode: (a: number, b: number) => any;
    readonly compiledcell_code_size: (a: number) => number;
    readonly compiledcell_compile: (a: number, b: number) => [number, number, number];
    readonly compiledcell_constants: (a: number) => number;
    readonly compiledcell_disassembly: (a: number) => [number, number];
    readonly compiledcell_from_bytes: (a: number, b: number) => [number, number, number];
    readonly compiledcell_functions: (a: number) => number;
    readonly compiledcell_run: (a: number) => any;
    readonly compiledcell_top_locals: (a: number) => number;
    readonly decode_and_run: (a: number, b: number) => any;
    readonly diagnose_src: (a: number, b: number) => any;
    readonly diagnostic_schema_json: () => [number, number];
    readonly ebnf_grammar: () => [number, number];
    readonly encode_cell_bytecode: (a: number, b: number) => any;
    readonly eval_cell_json: (a: number, b: number) => any;
    readonly eval_program_src: (a: number, b: number) => any;
    readonly gbnf_grammar: () => [number, number];
    readonly host_version: () => [number, number];
    readonly language_version: () => [number, number];
    readonly parse_cell_src: (a: number, b: number) => any;
    readonly parse_program_src: (a: number, b: number) => any;
    readonly project_source: (a: number, b: number) => any;
    readonly run_cell_bytecode: (a: number, b: number) => any;
    readonly run_program_bytecode: (a: number, b: number, c: number, d: number, e: number, f: number) => any;
    readonly eval_cell_src: (a: number, b: number) => any;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
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
