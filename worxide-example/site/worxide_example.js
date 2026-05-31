import { worxide_glue_url } from './snippets/worxide-b41acc353c4bdba2/inline1.js';
import * as import1 from "./snippets/worxide-b41acc353c4bdba2/inline0.js"


/**
 * Worker thread entry point for sync tasks. Returns the result pointer.
 * @param {number} task_ptr
 * @returns {number}
 */
export function __worxide_worker_entry(task_ptr) {
    const ret = wasm.__worxide_worker_entry(task_ptr);
    return ret >>> 0;
}

/**
 * Worker thread entry point for async tasks. Returns a Promise that
 * resolves with the result pointer once the task's future completes.
 *
 * We deliberately avoid `wasm_bindgen_futures::future_to_promise` /
 * `spawn_local` here. With atomics enabled (required for shared memory),
 * wasm-bindgen-futures selects its *multithread* executor, which schedules
 * wakeups through `Atomics.waitAsync` and a coordinator worker. That
 * machinery assumes a persistent thread-pool runtime we don't have — in our
 * one-shot worker it produces an undefined wakeup promise and dies. Instead
 * we drive the future ourselves on the worker's own event loop, rescheduling
 * polls via `queueMicrotask`. JsFuture wakeups (Promise.then callbacks) run
 * on that same event loop, so progress happens without any cross-thread
 * futex coordination.
 * @param {number} task_ptr
 * @returns {Promise<any>}
 */
export function __worxide_worker_entry_async(task_ptr) {
    const ret = wasm.__worxide_worker_entry_async(task_ptr);
    return ret;
}

/**
 * Main-thread entry point. Called explicitly from index.html via init+run.
 * NOT a `#[wasm_bindgen(start)]` — that would run on every wasm instantiation
 * including each worker, where there is no DOM (and dominator's globals would
 * panic and poison shared state). Workers never call this; they only run
 * worxide's worker entry points.
 */
export function run() {
    wasm.run();
}
function __wbg_get_imports(memory) {
    const import0 = {
        __proto__: null,
        __wbg___wbindgen_is_function_754e9f305ff6029e: function(arg0) {
            const ret = typeof(arg0) === 'function';
            return ret;
        },
        __wbg___wbindgen_is_undefined_67b456be8673d3d7: function(arg0) {
            const ret = arg0 === undefined;
            return ret;
        },
        __wbg___wbindgen_memory_fbc4c3e30b409f08: function() {
            const ret = wasm.memory;
            return ret;
        },
        __wbg___wbindgen_module_5dcc25d553a4424f: function() {
            const ret = wasmModule;
            return ret;
        },
        __wbg___wbindgen_number_get_9bb1761122181af2: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'number' ? obj : undefined;
            getDataViewMemory0().setFloat64(arg0 + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
        },
        __wbg___wbindgen_rethrow_c4d99b4b53265290: function(arg0) {
            throw arg0;
        },
        __wbg___wbindgen_string_get_72bdf95d3ae505b1: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'string' ? obj : undefined;
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_throw_1506f2235d1bdba0: function(arg0, arg1) {
            var v0 = getCachedStringFromWasm0(arg0, arg1);
            throw new Error(v0);
        },
        __wbg__wbg_cb_unref_61db23ac97f16c31: function(arg0) {
            arg0._wbg_cb_unref();
        },
        __wbg_add_7230e441ab238a9d: function() { return handleError(function (arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            arg0.add(v0);
        }, arguments); },
        __wbg_appendChild_364435158a70bd03: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.appendChild(arg1);
            return ret;
        }, arguments); },
        __wbg_async_ed0edf9269e8f04a: function(arg0) {
            const ret = arg0.async;
            return ret;
        },
        __wbg_body_7d5b1a2ac7f2c821: function(arg0) {
            const ret = arg0.body;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_buffer_a1f116eb4fdb1531: function(arg0) {
            const ret = arg0.buffer;
            return ret;
        },
        __wbg_call_40e4174f169eaca7: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = arg0.call(arg1, arg2, arg3);
            return ret;
        }, arguments); },
        __wbg_call_8a89609d89f6608a: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.call(arg1);
            return ret;
        }, arguments); },
        __wbg_call_9c758de292015997: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.call(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_classList_0b8ee0b50be29141: function(arg0) {
            const ret = arg0.classList;
            return ret;
        },
        __wbg_construct_e0f002bb615dbba1: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.construct(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_createComment_fee3099b63c574fb: function(arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            const ret = arg0.createComment(v0);
            return ret;
        },
        __wbg_createElement_c3c16a9aa7f5cc74: function() { return handleError(function (arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            const ret = arg0.createElement(v0);
            return ret;
        }, arguments); },
        __wbg_createObjectURL_395ba916655726cd: function() { return handleError(function (arg0, arg1) {
            const ret = URL.createObjectURL(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_createTextNode_f78a04409331196b: function(arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            const ret = arg0.createTextNode(v0);
            return ret;
        },
        __wbg_cssRules_e26519a35793cd05: function() { return handleError(function (arg0) {
            const ret = arg0.cssRules;
            return ret;
        }, arguments); },
        __wbg_data_93740e25a9d5b212: function(arg0) {
            const ret = arg0.data;
            return ret;
        },
        __wbg_data_bd354b70c783c66e: function(arg0) {
            const ret = arg0.data;
            return ret;
        },
        __wbg_document_aceb08cd6489baf5: function(arg0) {
            const ret = arg0.document;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_error_78ff5b3a29b770e0: function(arg0) {
            console.error(arg0);
        },
        __wbg_getPropertyValue_dbbb77f232017e4d: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            var v0 = getCachedStringFromWasm0(arg2, arg3);
            const ret = arg1.getPropertyValue(v0);
            const ptr2 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len2 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len2, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr2, true);
        }, arguments); },
        __wbg_get_2b48c7d0d006a781: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
        },
        __wbg_get_8baf556ec6b77a8a: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_get_de6a0f7d4d18a304: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_head_79cb26ea82b99acd: function(arg0) {
            const ret = arg0.head;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_insertBefore_41123fdf1b69b43d: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.insertBefore(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_insertRule_df7eafadd190c6e4: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            const ret = arg0.insertRule(v0, arg3 >>> 0);
            return ret;
        }, arguments); },
        __wbg_instanceof_HtmlElement_9d326f7a42217802: function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Window_e093be59ee9a8e14: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Window;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_isArray_67c2c9c4313f4448: function(arg0) {
            const ret = Array.isArray(arg0);
            return ret;
        },
        __wbg_length_66f1a4b2e9026940: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_length_6b04cfc608031423: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_log_cf2e968649f3384e: function(arg0) {
            console.log(arg0);
        },
        __wbg_new_b682b81e8eaaf027: function(arg0, arg1) {
            try {
                var state0 = {a: arg0, b: arg1};
                var cb0 = (arg0, arg1) => {
                    const a = state0.a;
                    state0.a = 0;
                    try {
                        return wasm_bindgen_9162eb1f66f461d0___convert__closures_____invoke___js_sys_7b0dd281e21c96aa___Function_fn_wasm_bindgen_9162eb1f66f461d0___JsValue_____wasm_bindgen_9162eb1f66f461d0___sys__Undefined___js_sys_7b0dd281e21c96aa___Function_fn_wasm_bindgen_9162eb1f66f461d0___JsValue_____wasm_bindgen_9162eb1f66f461d0___sys__Undefined_______true_(a, state0.b, arg0, arg1);
                    } finally {
                        state0.a = a;
                    }
                };
                const ret = new Promise(cb0);
                return ret;
            } finally {
                state0.a = 0;
            }
        },
        __wbg_new_ce1ab61c1c2b300d: function() {
            const ret = new Object();
            return ret;
        },
        __wbg_new_d90091b82fdf5b91: function() {
            const ret = new Array();
            return ret;
        },
        __wbg_new_dc32d91df76232c8: function(arg0) {
            const ret = new Int32Array(arg0);
            return ret;
        },
        __wbg_new_with_options_5c98ca2e0eb88040: function() { return handleError(function (arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg0, arg1);
            const ret = new Worker(v0, arg2);
            return ret;
        }, arguments); },
        __wbg_new_with_str_sequence_and_options_21cfcd771283a47d: function() { return handleError(function (arg0, arg1) {
            const ret = new Blob(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_new_worker_227309bcfae51cd3: function(arg0, arg1) {
            var v0 = getCachedStringFromWasm0(arg0, arg1);
            const ret = new Worker(v0);
            return ret;
        },
        __wbg_of_57145fdec12d159f: function(arg0) {
            const ret = Array.of(arg0);
            return ret;
        },
        __wbg_of_5d9c1c77975668d1: function(arg0, arg1, arg2) {
            const ret = Array.of(arg0, arg1, arg2);
            return ret;
        },
        __wbg_postMessage_c28ba544836193c8: function() { return handleError(function (arg0, arg1) {
            arg0.postMessage(arg1);
        }, arguments); },
        __wbg_postMessage_cf975f9c13498b76: function() { return handleError(function (arg0, arg1) {
            arg0.postMessage(arg1);
        }, arguments); },
        __wbg_push_a6822215aa43e71c: function(arg0, arg1) {
            const ret = arg0.push(arg1);
            return ret;
        },
        __wbg_queueMicrotask_35c611f4a14830b2: function(arg0) {
            queueMicrotask(arg0);
        },
        __wbg_queueMicrotask_404ed0a58e0b63cc: function(arg0) {
            const ret = arg0.queueMicrotask;
            return ret;
        },
        __wbg_random_33cfffca5c784d5e: function() {
            const ret = Math.random();
            return ret;
        },
        __wbg_removeChild_542662c726ba0cbf: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.removeChild(arg1);
            return ret;
        }, arguments); },
        __wbg_removeProperty_d208a45736ad36a4: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            var v0 = getCachedStringFromWasm0(arg2, arg3);
            const ret = arg1.removeProperty(v0);
            const ptr2 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len2 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len2, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr2, true);
        }, arguments); },
        __wbg_remove_cb24381a80bd06e4: function() { return handleError(function (arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            arg0.remove(v0);
        }, arguments); },
        __wbg_replaceChild_d525cded560578a9: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.replaceChild(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_requestAnimationFrame_72bbc2f340fc7a29: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.requestAnimationFrame(arg1);
            return ret;
        }, arguments); },
        __wbg_resolve_25a7e548d5881dca: function(arg0) {
            const ret = Promise.resolve(arg0);
            return ret;
        },
        __wbg_setProperty_b21c8c7e36721268: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            var v1 = getCachedStringFromWasm0(arg3, arg4);
            var v2 = getCachedStringFromWasm0(arg5, arg6);
            arg0.setProperty(v0, v1, v2);
        }, arguments); },
        __wbg_setTimeout_b5f25e402b6e8ff9: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.setTimeout(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_set_6e30c9374c26414c: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = Reflect.set(arg0, arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_set_data_52bc8ae1eb8bd677: function(arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            arg0.data = v0;
        },
        __wbg_set_onerror_ca63827b3797eaa6: function(arg0, arg1) {
            arg0.onerror = arg1;
        },
        __wbg_set_onmessage_4a3fd7b90b968fc4: function(arg0, arg1) {
            arg0.onmessage = arg1;
        },
        __wbg_set_onmessage_ad00166b07fad0be: function(arg0, arg1) {
            arg0.onmessage = arg1;
        },
        __wbg_set_textContent_2a822316d8a2310b: function(arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            arg0.textContent = v0;
        },
        __wbg_set_type_a700d088c461a553: function(arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            arg0.type = v0;
        },
        __wbg_set_type_efd1d9519b590c34: function(arg0, arg1) {
            arg0.type = __wbindgen_enum_WorkerType[arg1];
        },
        __wbg_set_type_f2ba381718ed1039: function(arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            arg0.type = v0;
        },
        __wbg_sheet_fd15cff74ff3b5d2: function(arg0) {
            const ret = arg0.sheet;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_GLOBAL_9d53f2689e622ca1: function() {
            const ret = typeof global === 'undefined' ? null : global;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_GLOBAL_THIS_a1a35cec07001a8a: function() {
            const ret = typeof globalThis === 'undefined' ? null : globalThis;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_SELF_4c59f6c7ea29a144: function() {
            const ret = typeof self === 'undefined' ? null : self;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_WINDOW_e70ae9f2eb052253: function() {
            const ret = typeof window === 'undefined' ? null : window;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_stringify_8286df6dcc591521: function() { return handleError(function (arg0) {
            const ret = JSON.stringify(arg0);
            return ret;
        }, arguments); },
        __wbg_style_4291b02b3de4faea: function(arg0) {
            const ret = arg0.style;
            return ret;
        },
        __wbg_style_ad0f3eb1fd1aa2bc: function(arg0) {
            const ret = arg0.style;
            return ret;
        },
        __wbg_terminate_854183e37c3fd8fd: function(arg0) {
            arg0.terminate();
        },
        __wbg_then_18f476d590e58992: function(arg0, arg1, arg2) {
            const ret = arg0.then(arg1, arg2);
            return ret;
        },
        __wbg_then_47213a40b6aeb86c: function(arg0, arg1) {
            const ret = arg0.then(arg1);
            return ret;
        },
        __wbg_then_ac7b025999b52837: function(arg0, arg1) {
            const ret = arg0.then(arg1);
            return ret;
        },
        __wbg_value_2b11d753e2be3e57: function(arg0) {
            const ret = arg0.value;
            return ret;
        },
        __wbg_waitAsync_bfb213899274180a: function(arg0, arg1, arg2) {
            const ret = Atomics.waitAsync(arg0, arg1 >>> 0, arg2);
            return ret;
        },
        __wbg_waitAsync_fbc667ccb52b6fbf: function() {
            const ret = Atomics.waitAsync;
            return ret;
        },
        __wbg_worxide_glue_url_9e4d5679748dd5e6: function(arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            const ret = worxide_glue_url(v0);
            const ptr2 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len2 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len2, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr2, true);
        },
        __wbindgen_cast_0000000000000001: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [Externref], shim_idx: 11, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_9162eb1f66f461d0___convert__closures_____invoke___wasm_bindgen_9162eb1f66f461d0___JsValue______true_);
            return ret;
        },
        __wbindgen_cast_0000000000000002: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [Externref], shim_idx: 43, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_9162eb1f66f461d0___convert__closures_____invoke___wasm_bindgen_9162eb1f66f461d0___JsValue__core_2157250efb9f8096___result__Result_____wasm_bindgen_9162eb1f66f461d0___JsError___true_);
            return ret;
        },
        __wbindgen_cast_0000000000000003: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("MessageEvent")], shim_idx: 11, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_9162eb1f66f461d0___convert__closures_____invoke___wasm_bindgen_9162eb1f66f461d0___JsValue______true__2);
            return ret;
        },
        __wbindgen_cast_0000000000000004: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [], shim_idx: 16, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_9162eb1f66f461d0___convert__closures_____invoke_______true_);
            return ret;
        },
        __wbindgen_cast_0000000000000005: function(arg0) {
            // Cast intrinsic for `F64 -> Externref`.
            const ret = arg0;
            return ret;
        },
        __wbindgen_cast_0000000000000006: function(arg0, arg1) {
            var v0 = getCachedStringFromWasm0(arg0, arg1);
            // Cast intrinsic for `Ref(CachedString) -> Externref`.
            const ret = v0;
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
        __wbindgen_link_e2b5a1199ad11c6b: function(arg0) {
            const val = `onmessage = function (ev) {
                let [ia, index, value] = ev.data;
                ia = new Int32Array(ia.buffer);
                let result = Atomics.wait(ia, index, value);
                postMessage(result);
            };
            `;
            const ret = typeof URL.createObjectURL === 'undefined' ? "data:application/javascript," + encodeURIComponent(val) : URL.createObjectURL(new Blob([val], { type: "text/javascript" }));
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        memory: memory || new WebAssembly.Memory({initial:18,maximum:16384,shared:true}),
    };
    return {
        __proto__: null,
        "./worxide_example_bg.js": import0,
        "./snippets/worxide-b41acc353c4bdba2/inline0.js": import1,
    };
}

function wasm_bindgen_9162eb1f66f461d0___convert__closures_____invoke_______true_(arg0, arg1) {
    wasm.wasm_bindgen_9162eb1f66f461d0___convert__closures_____invoke_______true_(arg0, arg1);
}

function wasm_bindgen_9162eb1f66f461d0___convert__closures_____invoke___wasm_bindgen_9162eb1f66f461d0___JsValue______true_(arg0, arg1, arg2) {
    wasm.wasm_bindgen_9162eb1f66f461d0___convert__closures_____invoke___wasm_bindgen_9162eb1f66f461d0___JsValue______true_(arg0, arg1, arg2);
}

function wasm_bindgen_9162eb1f66f461d0___convert__closures_____invoke___wasm_bindgen_9162eb1f66f461d0___JsValue______true__2(arg0, arg1, arg2) {
    wasm.wasm_bindgen_9162eb1f66f461d0___convert__closures_____invoke___wasm_bindgen_9162eb1f66f461d0___JsValue______true__2(arg0, arg1, arg2);
}

function wasm_bindgen_9162eb1f66f461d0___convert__closures_____invoke___wasm_bindgen_9162eb1f66f461d0___JsValue__core_2157250efb9f8096___result__Result_____wasm_bindgen_9162eb1f66f461d0___JsError___true_(arg0, arg1, arg2) {
    const ret = wasm.wasm_bindgen_9162eb1f66f461d0___convert__closures_____invoke___wasm_bindgen_9162eb1f66f461d0___JsValue__core_2157250efb9f8096___result__Result_____wasm_bindgen_9162eb1f66f461d0___JsError___true_(arg0, arg1, arg2);
    if (ret[1]) {
        throw takeFromExternrefTable0(ret[0]);
    }
}

function wasm_bindgen_9162eb1f66f461d0___convert__closures_____invoke___js_sys_7b0dd281e21c96aa___Function_fn_wasm_bindgen_9162eb1f66f461d0___JsValue_____wasm_bindgen_9162eb1f66f461d0___sys__Undefined___js_sys_7b0dd281e21c96aa___Function_fn_wasm_bindgen_9162eb1f66f461d0___JsValue_____wasm_bindgen_9162eb1f66f461d0___sys__Undefined_______true_(arg0, arg1, arg2, arg3) {
    wasm.wasm_bindgen_9162eb1f66f461d0___convert__closures_____invoke___js_sys_7b0dd281e21c96aa___Function_fn_wasm_bindgen_9162eb1f66f461d0___JsValue_____wasm_bindgen_9162eb1f66f461d0___sys__Undefined___js_sys_7b0dd281e21c96aa___Function_fn_wasm_bindgen_9162eb1f66f461d0___JsValue_____wasm_bindgen_9162eb1f66f461d0___sys__Undefined_______true_(arg0, arg1, arg2, arg3);
}


const __wbindgen_enum_WorkerType = ["classic", "module"];

function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_externrefs.set(idx, obj);
    return idx;
}

const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(state => wasm.__wbindgen_destroy_closure(state.a, state.b));

function getCachedStringFromWasm0(ptr, len) {
    if (ptr === 0) {
        return getFromExternrefTable0(len);
    } else {
        return getStringFromWasm0(ptr, len);
    }
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer !== wasm.memory.buffer) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

function getFromExternrefTable0(idx) { return wasm.__wbindgen_externrefs.get(idx); }

function getStringFromWasm0(ptr, len) {
    return decodeText(ptr >>> 0, len);
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.buffer !== wasm.memory.buffer) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        const idx = addToExternrefTable0(e);
        wasm.__wbindgen_exn_store(idx);
    }
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

function makeMutClosure(arg0, arg1, f) {
    const state = { a: arg0, b: arg1, cnt: 1 };
    const real = (...args) => {

        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        } finally {
            state.a = a;
            real._wbg_cb_unref();
        }
    };
    real._wbg_cb_unref = () => {
        if (--state.cnt === 0) {
            wasm.__wbindgen_destroy_closure(state.a, state.b);
            state.a = 0;
            CLOSURE_DTORS.unregister(state);
        }
    };
    CLOSURE_DTORS.register(real, state, state);
    return real;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = (typeof TextDecoder !== 'undefined' ? new TextDecoder('utf-8', { ignoreBOM: true, fatal: true }) : undefined);
if (cachedTextDecoder) cachedTextDecoder.decode();

const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().slice(ptr, ptr + len));
}

const cachedTextEncoder = (typeof TextEncoder !== 'undefined' ? new TextEncoder() : undefined);

if (cachedTextEncoder) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasmInstance, wasm;
function __wbg_finalize_init(instance, module, thread_stack_size) {
    wasmInstance = instance;
    wasm = instance.exports;
    wasmModule = module;
    cachedDataViewMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    if (typeof thread_stack_size !== 'undefined' && (typeof thread_stack_size !== 'number' || thread_stack_size === 0 || thread_stack_size % 65536 !== 0)) {
        throw new Error('invalid stack size');
    }

    wasm.__wbindgen_start(thread_stack_size);
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = module.ok && expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module, memory) {
    if (wasm !== undefined) return wasm;

    let thread_stack_size
    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module, memory, thread_stack_size} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports(memory);
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module, thread_stack_size);
}

async function __wbg_init(module_or_path, memory) {
    if (wasm !== undefined) return wasm;

    let thread_stack_size
    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path, memory, thread_stack_size} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('worxide_example_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports(memory);

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module, thread_stack_size);
}

export { initSync, __wbg_init as default };
