/* @ts-self-types="./qualia_core_db.d.ts" */
import * as wasm from "./qualia_core_db_bg.wasm";
import { __wbg_set_wasm } from "./qualia_core_db_bg.js";

__wbg_set_wasm(wasm);
wasm.__wbindgen_start();
export {
    QualiaWasmBridge, compile_query_to_json
} from "./qualia_core_db_bg.js";
