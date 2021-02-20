/* tslint:disable */
/* eslint-disable */
/**
* Starts the client.
* @param {string} chain_spec
* @param {string} log_level
* @returns {any}
*/
export function start_client(chain_spec: string, log_level: string): any;
/**
* A running client.
*/
export class Client {
  free(): void;
/**
* Allows starting an RPC request. Returns a `Promise` containing the result of that request.
* @param {string} rpc
* @returns {Promise<any>}
*/
  rpcSend(rpc: string): Promise<any>;
/**
* Subscribes to an RPC pubsub endpoint.
* @param {string} rpc
* @param {Function} callback
*/
  rpcSubscribe(rpc: string, callback: Function): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly start_client: (a: number, b: number, c: number, d: number) => number;
  readonly __wbg_client_free: (a: number) => void;
  readonly client_rpcSend: (a: number, b: number, c: number) => number;
  readonly client_rpcSubscribe: (a: number, b: number, c: number, d: number) => void;
  readonly hash_test: (a: number, b: number) => number;
  readonly __wbindgen_malloc: (a: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number) => number;
  readonly __wbindgen_export_2: WebAssembly.Table;
  readonly _dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h06fdd597dff436cf: (a: number, b: number) => void;
  readonly _dyn_core__ops__function__FnMut___A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h39815ccbd0e19cec: (a: number, b: number, c: number) => void;
  readonly _dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h6fc708920200cf3b: (a: number, b: number) => void;
  readonly _dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h3a6a55f703195a5d: (a: number, b: number) => void;
  readonly _dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__hc70f757e1829f06a: (a: number, b: number, c: number) => void;
  readonly __wbindgen_free: (a: number, b: number) => void;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly wasm_bindgen__convert__closures__invoke2_mut__hb1291065d37f2c5a: (a: number, b: number, c: number, d: number) => void;
}

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {InitInput | Promise<InitInput>} module_or_path
*
* @returns {Promise<InitOutput>}
*/
export default function init (module_or_path?: InitInput | Promise<InitInput>): Promise<InitOutput>;
        