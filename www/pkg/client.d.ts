/* tslint:disable */
/* eslint-disable */

export class GameState {
    free(): void;
    [Symbol.dispose](): void;
    get_map_data(): any;
    /**
     * Snapshot consumed by Three.js. Positions include previous/current pairs for interpolation.
     */
    get_render_data(): any;
    /**
     * Movement is continuous; action is an optional one-shot command.
     */
    handle_input(move_x: number, move_z: number, action: string | null | undefined, aim_x: number, aim_z: number): void;
    constructor();
    reset_save(): void;
    save_now(): void;
    send_chat(text: string): void;
    /**
     * JS calls this once per animation frame. Rust advances only at a fixed 20 Hz.
     */
    tick(dt: number): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_gamestate_free: (a: number, b: number) => void;
    readonly gamestate_get_map_data: (a: number) => any;
    readonly gamestate_get_render_data: (a: number) => any;
    readonly gamestate_handle_input: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
    readonly gamestate_new: () => number;
    readonly gamestate_reset_save: (a: number) => void;
    readonly gamestate_save_now: (a: number) => void;
    readonly gamestate_send_chat: (a: number, b: number, c: number) => void;
    readonly gamestate_tick: (a: number, b: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
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
