import { worxide_app_js_path, worxide_glue_url, worxide_glue_url_from_path } from './snippets/worxide-593c3a9d3926efa8/inline0.js';
import * as import1 from "./snippets/worxide-593c3a9d3926efa8/inline1.js"


/**
 * Seed this thread's resolved glue-URL cache.
 *
 * Called by `worker.js` immediately after `initSync`, passing the `glue_url`
 * the worker received over `postMessage`. This makes any *nested* spawn the
 * worker performs reuse the already-resolved URL instead of re-deriving it —
 * crucial because `window` / `globalThis.app_js_path` is not set on workers.
 * @param {string} url
 */
export function __worxide_seed_glue_url(url) {
    const ptr0 = passStringToWasm0(url, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    wasm.__worxide_seed_glue_url(ptr0, len0);
}

/**
 * Worker thread entry point for sync tasks. Returns the result pointer bytes,
 * or `Err` (thrown to JS and caught by worker.js as a per-call error frame) if
 * the payload is malformed — see [`decode_task_ptr`].
 * @param {Uint8Array} ptr_bytes
 * @returns {Uint8Array}
 */
export function __worxide_worker_entry(ptr_bytes) {
    const ptr0 = passArray8ToWasm0(ptr_bytes, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.__worxide_worker_entry(ptr0, len0);
    if (ret[3]) {
        throw takeFromExternrefTable0(ret[2]);
    }
    var v2 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    return v2;
}

/**
 * Worker thread entry point for async tasks. Returns a Promise that
 * resolves with the result pointer bytes once the task's future completes.
 * We deliberately avoid `wasm_bindgen_futures::future_to_promise`
 * `spawn_local` here. With atomics enabled (required for shared memory),
 * wasm-bindgen-futures selects its *multithread* executor, which schedules
 * wakeups through `Atomics.waitAsync` and a coordinator worker. That
 * machinery assumes a persistent thread-pool runtime we don't have — in our
 * one-shot worker it produces an undefined wakeup promise and dies. Instead
 * we drive the future ourselves on the worker's own event loop, rescheduling
 * polls via `queueMicrotask`. JsFuture wakeups (Promise.then callbacks) run
 * on that same event loop, so progress happens without any cross-thread
 * futex coordination.
 * @param {Uint8Array} ptr_bytes
 * @returns {Promise<any>}
 */
export function __worxide_worker_entry_async(ptr_bytes) {
    const ptr0 = passArray8ToWasm0(ptr_bytes, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.__worxide_worker_entry_async(ptr0, len0);
    return ret;
}

/**
 * @returns {Promise<void>}
 */
export function run_app() {
    const ret = wasm.run_app();
    return ret;
}
function __wbg_get_imports(memory) {
    const import0 = {
        __proto__: null,
        __wbg___wbindgen_debug_string_8a447059637473e2: function(arg0, arg1) {
            const ret = debugString(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_is_function_acc5528be2b923f2: function(arg0) {
            const ret = typeof(arg0) === 'function';
            return ret;
        },
        __wbg___wbindgen_is_null_or_undefined_d75296adef6d67e9: function(arg0) {
            const ret = arg0 == null;
            return ret;
        },
        __wbg___wbindgen_is_undefined_721f8decd50c87a3: function(arg0) {
            const ret = arg0 === undefined;
            return ret;
        },
        __wbg___wbindgen_memory_9751d9a3017e7c25: function() {
            const ret = wasm.memory;
            return ret;
        },
        __wbg___wbindgen_module_f86f28f0f85995dd: function() {
            const ret = wasmModule;
            return ret;
        },
        __wbg___wbindgen_rethrow_858623e73c3311dc: function(arg0) {
            throw arg0;
        },
        __wbg___wbindgen_string_get_71bb4348194e31f0: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'string' ? obj : undefined;
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_throw_ea4887a5f8f9a9db: function(arg0, arg1) {
            var v0 = getCachedStringFromWasm0(arg0, arg1);
            throw new Error(v0);
        },
        __wbg__wbg_cb_unref_33c39e13d73b25f6: function(arg0) {
            arg0._wbg_cb_unref();
        },
        __wbg_abort_6e6ea7d259504afc: function(arg0) {
            arg0.abort();
        },
        __wbg_abort_9e39323f373e2585: function(arg0, arg1) {
            arg0.abort(arg1);
        },
        __wbg_addEventListener_9f744a06d0879451: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            arg0.addEventListener(v0, arg3, arg4);
        }, arguments); },
        __wbg_add_af6cf10f53edb446: function() { return handleError(function (arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            arg0.add(v0);
        }, arguments); },
        __wbg_appendChild_acb7691406591783: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.appendChild(arg1);
            return ret;
        }, arguments); },
        __wbg_append_912a8705e9b6a483: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            var v1 = getCachedStringFromWasm0(arg3, arg4);
            arg0.append(v0, v1);
        }, arguments); },
        __wbg_arrayBuffer_ff96d08b7b6be32e: function() { return handleError(function (arg0) {
            const ret = arg0.arrayBuffer();
            return ret;
        }, arguments); },
        __wbg_async_4ec36f08efecafdc: function(arg0) {
            const ret = arg0.async;
            return ret;
        },
        __wbg_body_c94f9310613c153b: function(arg0) {
            const ret = arg0.body;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_buffer_49b4f592d8036785: function(arg0) {
            const ret = arg0.buffer;
            return ret;
        },
        __wbg_call_0e855b388e315e17: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = arg0.call(arg1, arg2, arg3);
            return ret;
        }, arguments); },
        __wbg_call_5575218572ead796: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.call(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_classList_58709a5793901814: function(arg0) {
            const ret = arg0.classList;
            return ret;
        },
        __wbg_clearTimeout_333bba87532ab9d3: function(arg0) {
            const ret = clearTimeout(arg0);
            return ret;
        },
        __wbg_construct_96b88162f762fb4f: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.construct(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_createComment_7e51040356705a12: function(arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            const ret = arg0.createComment(v0);
            return ret;
        },
        __wbg_createElement_9e23ac95e40e302c: function() { return handleError(function (arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            const ret = arg0.createElement(v0);
            return ret;
        }, arguments); },
        __wbg_createObjectURL_d8fd80787743974d: function() { return handleError(function (arg0, arg1) {
            const ret = URL.createObjectURL(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_createTextNode_f1aff30c8f1e7fff: function(arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            const ret = arg0.createTextNode(v0);
            return ret;
        },
        __wbg_data_23c14f7d1102d077: function(arg0) {
            const ret = arg0.data;
            return ret;
        },
        __wbg_data_4a7f1308dbd33a21: function(arg0) {
            const ret = arg0.data;
            return ret;
        },
        __wbg_document_2634180a4c694068: function(arg0) {
            const ret = arg0.document;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_done_b62d4a7d2286852a: function(arg0) {
            const ret = arg0.done;
            return ret;
        },
        __wbg_entries_d3062691e1f88830: function(arg0) {
            const ret = arg0.entries();
            return ret;
        },
        __wbg_fetch_074561c3e313c86f: function(arg0) {
            const ret = fetch(arg0);
            return ret;
        },
        __wbg_fetch_db87be8a748781a2: function(arg0, arg1) {
            const ret = arg0.fetch(arg1);
            return ret;
        },
        __wbg_getPropertyValue_25192bdc8db9053c: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            var v0 = getCachedStringFromWasm0(arg2, arg3);
            const ret = arg1.getPropertyValue(v0);
            const ptr2 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len2 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len2, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr2, true);
        }, arguments); },
        __wbg_get_197a3fe98f169e38: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
        },
        __wbg_get_dddb90ff5d27a080: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_has_4f060fe202ad7e87: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.has(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_headers_d9123c649c85d441: function(arg0) {
            const ret = arg0.headers;
            return ret;
        },
        __wbg_insertBefore_d1c9b9c881ae2c96: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.insertBefore(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_instanceof_HtmlButtonElement_ed9bdefa1c407160: function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLButtonElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_HtmlDivElement_0d08ab894861e59a: function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLDivElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_HtmlHeadingElement_8e77e0c8500cfca6: function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLHeadingElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Response_79948c98d1d2ba75: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Response;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Uint8Array_f080092dc70f5d58: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Uint8Array;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Window_0d356b88a2f77c42: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Window;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_isArray_145a34fd0a38d37b: function(arg0) {
            const ret = Array.isArray(arg0);
            return ret;
        },
        __wbg_length_589238bdcf171f0e: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_length_c6054974c0a6cdb9: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_new_10e2f2ad134f940f: function() { return handleError(function () {
            const ret = new Headers();
            return ret;
        }, arguments); },
        __wbg_new_2e117a478906f062: function() {
            const ret = new Object();
            return ret;
        },
        __wbg_new_36e147a8ced3c6e0: function() {
            const ret = new Array();
            return ret;
        },
        __wbg_new_476e05fb84d8e4f3: function(arg0) {
            const ret = new Int32Array(arg0);
            return ret;
        },
        __wbg_new_51233fa2a760b272: function() { return handleError(function () {
            const ret = new AbortController();
            return ret;
        }, arguments); },
        __wbg_new_81880fb5002cb255: function(arg0) {
            const ret = new Uint8Array(arg0);
            return ret;
        },
        __wbg_new_f85beb941dc6d8aa: function(arg0, arg1) {
            try {
                var state0 = {a: arg0, b: arg1};
                var cb0 = (arg0, arg1) => {
                    const a = state0.a;
                    state0.a = 0;
                    try {
                        return wasm_bindgen_5968a9e4e00efcb7___convert__closures_____invoke___js_sys_e16c673b36a06dba___Function_fn_wasm_bindgen_5968a9e4e00efcb7___JsValue_____wasm_bindgen_5968a9e4e00efcb7___sys__Undefined___js_sys_e16c673b36a06dba___Function_fn_wasm_bindgen_5968a9e4e00efcb7___JsValue_____wasm_bindgen_5968a9e4e00efcb7___sys__Undefined_______true_(a, state0.b, arg0, arg1);
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
        __wbg_new_from_slice_543b875b27789a8f: function(arg0, arg1) {
            const ret = new Uint8Array(getArrayU8FromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_new_typed_00a409eb4ec4f2d9: function(arg0, arg1) {
            try {
                var state0 = {a: arg0, b: arg1};
                var cb0 = (arg0, arg1) => {
                    const a = state0.a;
                    state0.a = 0;
                    try {
                        return wasm_bindgen_5968a9e4e00efcb7___convert__closures_____invoke___js_sys_e16c673b36a06dba___Function_fn_wasm_bindgen_5968a9e4e00efcb7___JsValue_____wasm_bindgen_5968a9e4e00efcb7___sys__Undefined___js_sys_e16c673b36a06dba___Function_fn_wasm_bindgen_5968a9e4e00efcb7___JsValue_____wasm_bindgen_5968a9e4e00efcb7___sys__Undefined_______true_(a, state0.b, arg0, arg1);
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
        __wbg_new_with_options_c94a1e29cc348bc1: function() { return handleError(function (arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg0, arg1);
            const ret = new Worker(v0, arg2);
            return ret;
        }, arguments); },
        __wbg_new_with_str_and_init_5b299538bdeeec64: function() { return handleError(function (arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg0, arg1);
            const ret = new Request(v0, arg2);
            return ret;
        }, arguments); },
        __wbg_new_with_str_sequence_and_options_b67a6653749e1791: function() { return handleError(function (arg0, arg1) {
            const ret = new Blob(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_new_worker_e68fc8188cc230b5: function(arg0, arg1) {
            var v0 = getCachedStringFromWasm0(arg0, arg1);
            const ret = new Worker(v0);
            return ret;
        },
        __wbg_next_0c4066e251d2eff9: function() { return handleError(function (arg0) {
            const ret = arg0.next();
            return ret;
        }, arguments); },
        __wbg_now_0f628e0e435c541b: function(arg0) {
            const ret = arg0.now();
            return ret;
        },
        __wbg_of_62183ea089c00bfa: function(arg0) {
            const ret = Array.of(arg0);
            return ret;
        },
        __wbg_of_8737c43050b5546e: function(arg0, arg1, arg2) {
            const ret = Array.of(arg0, arg1, arg2);
            return ret;
        },
        __wbg_performance_4c23a97261596fec: function(arg0) {
            const ret = arg0.performance;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_postMessage_0162be6e48cf631e: function() { return handleError(function (arg0, arg1) {
            arg0.postMessage(arg1);
        }, arguments); },
        __wbg_postMessage_62e0230ec77661dc: function() { return handleError(function (arg0, arg1) {
            arg0.postMessage(arg1);
        }, arguments); },
        __wbg_prototypesetcall_d721637c7ca66eb8: function(arg0, arg1, arg2) {
            Uint8Array.prototype.set.call(getArrayU8FromWasm0(arg0, arg1), arg2);
        },
        __wbg_push_f724b5db8acf89d2: function(arg0, arg1) {
            const ret = arg0.push(arg1);
            return ret;
        },
        __wbg_queueMicrotask_1c9b3800e321a967: function(arg0) {
            const ret = arg0.queueMicrotask;
            return ret;
        },
        __wbg_queueMicrotask_311744e534a929a3: function(arg0) {
            queueMicrotask(arg0);
        },
        __wbg_random_3182549db57fb083: function() {
            const ret = Math.random();
            return ret;
        },
        __wbg_reject_142c43e36402c65b: function(arg0) {
            const ret = Promise.reject(arg0);
            return ret;
        },
        __wbg_removeChild_8e93fdf43472d328: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.removeChild(arg1);
            return ret;
        }, arguments); },
        __wbg_removeEventListener_c82e19c28fb77da9: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            arg0.removeEventListener(v0, arg3, arg4 !== 0);
        }, arguments); },
        __wbg_removeProperty_2015da6fd4077af0: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            var v0 = getCachedStringFromWasm0(arg2, arg3);
            const ret = arg1.removeProperty(v0);
            const ptr2 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len2 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len2, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr2, true);
        }, arguments); },
        __wbg_remove_c70bc51826fd90df: function() { return handleError(function (arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            arg0.remove(v0);
        }, arguments); },
        __wbg_replaceChild_e280f8abf58e5960: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.replaceChild(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_requestAnimationFrame_b92ccfbc8ca777a8: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.requestAnimationFrame(arg1);
            return ret;
        }, arguments); },
        __wbg_resolve_d82363d90af6928a: function(arg0) {
            const ret = Promise.resolve(arg0);
            return ret;
        },
        __wbg_setProperty_0f6cb60d274ee8d4: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            var v1 = getCachedStringFromWasm0(arg3, arg4);
            var v2 = getCachedStringFromWasm0(arg5, arg6);
            arg0.setProperty(v0, v1, v2);
        }, arguments); },
        __wbg_setTimeout_3a808dd861dd3c12: function(arg0, arg1) {
            const ret = setTimeout(arg0, arg1);
            return ret;
        },
        __wbg_set_4564f7dc44fcb0c9: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = Reflect.set(arg0, arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_set_body_97c25d1c0051cb04: function(arg0, arg1) {
            arg0.body = arg1;
        },
        __wbg_set_cache_47f0e68e0309bb63: function(arg0, arg1) {
            arg0.cache = __wbindgen_enum_RequestCache[arg1];
        },
        __wbg_set_capture_6f2b7cf10f3b3d75: function(arg0, arg1) {
            arg0.capture = arg1 !== 0;
        },
        __wbg_set_credentials_8dece1804391d22f: function(arg0, arg1) {
            arg0.credentials = __wbindgen_enum_RequestCredentials[arg1];
        },
        __wbg_set_data_a53b037521383e4d: function(arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            arg0.data = v0;
        },
        __wbg_set_headers_6751c09a8e579ff7: function(arg0, arg1) {
            arg0.headers = arg1;
        },
        __wbg_set_method_1120482abe0934aa: function(arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            arg0.method = v0;
        },
        __wbg_set_mode_e41f820af904cdaa: function(arg0, arg1) {
            arg0.mode = __wbindgen_enum_RequestMode[arg1];
        },
        __wbg_set_once_0ff9e026e78fe912: function(arg0, arg1) {
            arg0.once = arg1 !== 0;
        },
        __wbg_set_onerror_9919d8845c17500a: function(arg0, arg1) {
            arg0.onerror = arg1;
        },
        __wbg_set_onmessage_913b6742d632bd90: function(arg0, arg1) {
            arg0.onmessage = arg1;
        },
        __wbg_set_onmessage_d05709471e546dca: function(arg0, arg1) {
            arg0.onmessage = arg1;
        },
        __wbg_set_passive_6282b78d17eaaa2c: function(arg0, arg1) {
            arg0.passive = arg1 !== 0;
        },
        __wbg_set_signal_4a69430cb12800f3: function(arg0, arg1) {
            arg0.signal = arg1;
        },
        __wbg_set_type_3194793b7f2a05be: function(arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            arg0.type = v0;
        },
        __wbg_set_type_4a4ade8def855e6e: function(arg0, arg1) {
            arg0.type = __wbindgen_enum_WorkerType[arg1];
        },
        __wbg_signal_4d9d567be73ea52c: function(arg0) {
            const ret = arg0.signal;
            return ret;
        },
        __wbg_static_accessor_GLOBAL_THIS_2fee5048bcca5938: function() {
            const ret = typeof globalThis === 'undefined' ? null : globalThis;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_GLOBAL_ce44e66a4935da8c: function() {
            const ret = typeof global === 'undefined' ? null : global;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_SELF_44f6e0cb5e67cdad: function() {
            const ret = typeof self === 'undefined' ? null : self;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_WINDOW_168f178805d978fe: function() {
            const ret = typeof window === 'undefined' ? null : window;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_status_0053aa6239760447: function(arg0) {
            const ret = arg0.status;
            return ret;
        },
        __wbg_stringify_747a843de2eb6359: function() { return handleError(function (arg0) {
            const ret = JSON.stringify(arg0);
            return ret;
        }, arguments); },
        __wbg_style_f00fbf31e0a68f0e: function(arg0) {
            const ret = arg0.style;
            return ret;
        },
        __wbg_terminate_16a41ab2eaf82418: function(arg0) {
            arg0.terminate();
        },
        __wbg_then_05edfc8a4fea5106: function(arg0, arg1, arg2) {
            const ret = arg0.then(arg1, arg2);
            return ret;
        },
        __wbg_then_591b6b3a75ee817a: function(arg0, arg1) {
            const ret = arg0.then(arg1);
            return ret;
        },
        __wbg_then_c768c7c3e60c20ef: function(arg0, arg1) {
            const ret = arg0.then(arg1);
            return ret;
        },
        __wbg_url_0e0eeabf01fb5519: function(arg0, arg1) {
            const ret = arg1.url;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_value_2f34afb824ffcd9a: function(arg0) {
            const ret = arg0.value;
            return ret;
        },
        __wbg_value_49f783bb59765962: function(arg0) {
            const ret = arg0.value;
            return ret;
        },
        __wbg_waitAsync_134f0b4abc50f1f2: function(arg0, arg1, arg2) {
            const ret = Atomics.waitAsync(arg0, arg1 >>> 0, arg2);
            return ret;
        },
        __wbg_waitAsync_485a8d512901fd53: function() {
            const ret = Atomics.waitAsync;
            return ret;
        },
        __wbg_worxide_app_js_path_a64a67e2ecc92d5b: function(arg0) {
            const ret = worxide_app_js_path();
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_worxide_glue_url_533c15b120cfe7df: function(arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            const ret = worxide_glue_url(v0);
            const ptr2 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len2 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len2, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr2, true);
        },
        __wbg_worxide_glue_url_from_path_5aae197a34a2df14: function(arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            const ret = worxide_glue_url_from_path(v0);
            const ptr2 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len2 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len2, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr2, true);
        },
        __wbindgen_cast_0000000000000001: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [Externref], shim_idx: 12, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_5968a9e4e00efcb7___convert__closures_____invoke___wasm_bindgen_5968a9e4e00efcb7___JsValue______true_);
            return ret;
        },
        __wbindgen_cast_0000000000000002: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [Externref], shim_idx: 50, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_5968a9e4e00efcb7___convert__closures_____invoke___wasm_bindgen_5968a9e4e00efcb7___JsValue__core_ad197ea32fa24677___result__Result_____wasm_bindgen_5968a9e4e00efcb7___JsError___true_);
            return ret;
        },
        __wbindgen_cast_0000000000000003: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("MessageEvent")], shim_idx: 12, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_5968a9e4e00efcb7___convert__closures_____invoke___wasm_bindgen_5968a9e4e00efcb7___JsValue______true__2);
            return ret;
        },
        __wbindgen_cast_0000000000000004: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [Ref(NamedExternref("Event"))], shim_idx: 24, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_5968a9e4e00efcb7___convert__closures________invoke___web_sys_977ed13a694527c3___features__gen_Event__Event______true_);
            return ret;
        },
        __wbindgen_cast_0000000000000005: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [], shim_idx: 62, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_5968a9e4e00efcb7___convert__closures_____invoke_______true_);
            return ret;
        },
        __wbindgen_cast_0000000000000006: function(arg0) {
            // Cast intrinsic for `F64 -> Externref`.
            const ret = arg0;
            return ret;
        },
        __wbindgen_cast_0000000000000007: function(arg0, arg1) {
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
        __wbindgen_link_ec60efd85dcca315: function(arg0) {
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
        memory: memory || new WebAssembly.Memory({initial:20,maximum:16384,shared:true}),
    };
    return {
        __proto__: null,
        "./worxide_example_bg.js": import0,
        "./snippets/worxide-593c3a9d3926efa8/inline1.js": import1,
    };
}

function wasm_bindgen_5968a9e4e00efcb7___convert__closures_____invoke_______true_(arg0, arg1) {
    wasm.wasm_bindgen_5968a9e4e00efcb7___convert__closures_____invoke_______true_(arg0, arg1);
}

function wasm_bindgen_5968a9e4e00efcb7___convert__closures_____invoke___wasm_bindgen_5968a9e4e00efcb7___JsValue______true_(arg0, arg1, arg2) {
    wasm.wasm_bindgen_5968a9e4e00efcb7___convert__closures_____invoke___wasm_bindgen_5968a9e4e00efcb7___JsValue______true_(arg0, arg1, arg2);
}

function wasm_bindgen_5968a9e4e00efcb7___convert__closures_____invoke___wasm_bindgen_5968a9e4e00efcb7___JsValue______true__2(arg0, arg1, arg2) {
    wasm.wasm_bindgen_5968a9e4e00efcb7___convert__closures_____invoke___wasm_bindgen_5968a9e4e00efcb7___JsValue______true__2(arg0, arg1, arg2);
}

function wasm_bindgen_5968a9e4e00efcb7___convert__closures________invoke___web_sys_977ed13a694527c3___features__gen_Event__Event______true_(arg0, arg1, arg2) {
    wasm.wasm_bindgen_5968a9e4e00efcb7___convert__closures________invoke___web_sys_977ed13a694527c3___features__gen_Event__Event______true_(arg0, arg1, arg2);
}

function wasm_bindgen_5968a9e4e00efcb7___convert__closures_____invoke___wasm_bindgen_5968a9e4e00efcb7___JsValue__core_ad197ea32fa24677___result__Result_____wasm_bindgen_5968a9e4e00efcb7___JsError___true_(arg0, arg1, arg2) {
    const ret = wasm.wasm_bindgen_5968a9e4e00efcb7___convert__closures_____invoke___wasm_bindgen_5968a9e4e00efcb7___JsValue__core_ad197ea32fa24677___result__Result_____wasm_bindgen_5968a9e4e00efcb7___JsError___true_(arg0, arg1, arg2);
    if (ret[1]) {
        throw takeFromExternrefTable0(ret[0]);
    }
}

function wasm_bindgen_5968a9e4e00efcb7___convert__closures_____invoke___js_sys_e16c673b36a06dba___Function_fn_wasm_bindgen_5968a9e4e00efcb7___JsValue_____wasm_bindgen_5968a9e4e00efcb7___sys__Undefined___js_sys_e16c673b36a06dba___Function_fn_wasm_bindgen_5968a9e4e00efcb7___JsValue_____wasm_bindgen_5968a9e4e00efcb7___sys__Undefined_______true_(arg0, arg1, arg2, arg3) {
    wasm.wasm_bindgen_5968a9e4e00efcb7___convert__closures_____invoke___js_sys_e16c673b36a06dba___Function_fn_wasm_bindgen_5968a9e4e00efcb7___JsValue_____wasm_bindgen_5968a9e4e00efcb7___sys__Undefined___js_sys_e16c673b36a06dba___Function_fn_wasm_bindgen_5968a9e4e00efcb7___JsValue_____wasm_bindgen_5968a9e4e00efcb7___sys__Undefined_______true_(arg0, arg1, arg2, arg3);
}


const __wbindgen_enum_RequestCache = ["default", "no-store", "reload", "no-cache", "force-cache", "only-if-cached"];


const __wbindgen_enum_RequestCredentials = ["omit", "same-origin", "include"];


const __wbindgen_enum_RequestMode = ["same-origin", "no-cors", "cors", "navigate"];


const __wbindgen_enum_WorkerType = ["classic", "module"];

function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_externrefs.set(idx, obj);
    return idx;
}

const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(state => wasm.__wbindgen_destroy_closure(state.a, state.b));

function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return  `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        } else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        } else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for(let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches && builtInMatches.length > 1) {
        className = builtInMatches[1];
    } else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        } catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

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

function passArray8ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 1, 1) >>> 0;
    getUint8ArrayMemory0().set(arg, ptr / 1);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
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
