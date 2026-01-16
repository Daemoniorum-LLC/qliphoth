/**
 * Sigil WASM Runtime
 *
 * Provides JS implementations of all WASM imports required by Sigil programs.
 * This is the bridge between WASM and browser APIs.
 */

// =============================================================================
// Memory Management
// =============================================================================

let wasmMemory = null;
let wasmExports = null;
let heapPtr = 1024 * 64; // Start heap after 64KB stack

function setWasmExports(exports) {
    wasmExports = exports;
    wasmMemory = exports.memory;
}

function getMemory() {
    return new Uint8Array(wasmMemory.buffer);
}

function readString(ptr, len) {
    const mem = getMemory();
    const bytes = mem.slice(ptr, ptr + len);
    return new TextDecoder().decode(bytes);
}

// Read a length-prefixed string (Sigil's format: 4-byte len + bytes)
function readLengthPrefixedString(ptr) {
    const mem = getMemory();
    const view = new DataView(wasmMemory.buffer);
    const len = view.getUint32(ptr, true); // little-endian
    const bytes = mem.slice(ptr + 4, ptr + 4 + len);
    return new TextDecoder().decode(bytes);
}

function writeString(str) {
    const bytes = new TextEncoder().encode(str);
    const ptr = heapPtr;
    const mem = getMemory();
    mem.set(bytes, ptr);
    heapPtr += bytes.length + 1; // +1 for null terminator
    return { ptr, len: bytes.length };
}

// Write a length-prefixed string (Sigil's format: 4-byte len + bytes)
function writeLengthPrefixedString(str) {
    const bytes = new TextEncoder().encode(str);
    const ptr = heapPtr;
    const view = new DataView(wasmMemory.buffer);
    // Write 4-byte length
    view.setUint32(ptr, bytes.length, true); // little-endian
    // Write string bytes
    const mem = getMemory();
    mem.set(bytes, ptr + 4);
    // Align to 8 bytes
    heapPtr += 4 + bytes.length;
    heapPtr = (heapPtr + 7) & ~7;
    return ptr;
}

// =============================================================================
// Signal System - Fine-grained Reactivity
// =============================================================================

const signals = new Map();          // id -> value
const signalSubscribers = new Map(); // id -> Set of effect runners
let nextSignalId = 1;
let nextEffectId = 1;

// Dependency tracking
let currentEffect = null;           // Currently executing effect (for auto-tracking)
let batchDepth = 0;
const pendingEffects = new Set();

// Effect registry
const effects = new Map();          // effectId -> { run, deps, cleanup }

function signalCreate(initialValue) {
    const id = nextSignalId++;
    signals.set(id, initialValue);
    signalSubscribers.set(id, new Set());
    console.log(`[signal.create] id=${id} value=${initialValue}`);
    return id;
}

function signalGet(id) {
    // Auto-track dependency if we're inside an effect
    if (currentEffect !== null) {
        const subs = signalSubscribers.get(id);
        if (subs && !subs.has(currentEffect)) {
            subs.add(currentEffect);
            currentEffect.deps.add(id);
        }
    }
    const value = signals.get(id) ?? 0n;
    console.log(`[signal.get] id=${id} value=${value}`);
    return value;
}

function signalSet(id, value) {
    const old = signals.get(id);
    console.log(`[signal.set] id=${id} old=${old} new=${value}`);
    if (old !== value) {
        signals.set(id, value);
        // Notify subscribers
        const subs = signalSubscribers.get(id);
        if (subs) {
            if (batchDepth > 0) {
                subs.forEach(effect => pendingEffects.add(effect));
            } else {
                // Run effects immediately, but avoid infinite loops
                const toRun = [...subs];
                toRun.forEach(effect => {
                    if (effect !== currentEffect) {
                        effect.run();
                    }
                });
            }
        }
    }
}

function signalSubscribe(id, callbackPtr) {
    const callback = () => {
        if (wasmExports && wasmExports.__indirect_function_table) {
            wasmExports.__indirect_function_table.get(callbackPtr)();
        }
    };
    const subs = signalSubscribers.get(id) || new Set();
    const handle = { run: callback, deps: new Set() };
    subs.add(handle);
    signalSubscribers.set(id, subs);
    return callbackPtr;
}

function signalUnsubscribe(handle) {
    // Would need to track handles properly in production
}

function signalBatchStart() {
    batchDepth++;
    console.log(`[signal.batch_start] depth=${batchDepth}`);
}

function signalBatchEnd() {
    batchDepth--;
    console.log(`[signal.batch_end] depth=${batchDepth} pending=${pendingEffects.size}`);
    if (batchDepth === 0 && pendingEffects.size > 0) {
        const toRun = [...pendingEffects];
        pendingEffects.clear();
        toRun.forEach(effect => effect.run());
    }
}

function signalComputed(computePtr) {
    // Create a computed signal - derives value from other signals
    const id = nextSignalId++;
    signals.set(id, 0n);
    signalSubscribers.set(id, new Set());

    const compute = () => {
        if (wasmExports && wasmExports.__indirect_function_table) {
            const result = wasmExports.__indirect_function_table.get(computePtr)();
            const old = signals.get(id);
            if (result !== old) {
                signals.set(id, result);
                // Notify our subscribers
                const subs = signalSubscribers.get(id);
                if (subs && subs.size > 0) {
                    subs.forEach(effect => {
                        if (effect !== currentEffect) {
                            effect.run();
                        }
                    });
                }
            }
            return result;
        }
        return 0n;
    };

    // Create an effect that recomputes when dependencies change
    const effectRunner = {
        deps: new Set(),
        run: () => {
            const prev = currentEffect;
            currentEffect = effectRunner;
            try {
                compute();
            } finally {
                currentEffect = prev;
            }
        }
    };

    // Run once to establish dependencies and get initial value
    effectRunner.run();

    console.log(`[signal.computed] id=${id} initial=${signals.get(id)}`);
    return id;
}

function signalEffect(effectPtr) {
    // Create and run a side effect that auto-tracks dependencies
    const effectId = nextEffectId++;

    const effectRunner = {
        id: effectId,
        deps: new Set(),
        cleanup: null,
        run: () => {
            // Clean up previous run
            if (effectRunner.cleanup) {
                effectRunner.cleanup();
                effectRunner.cleanup = null;
            }

            // Clear old subscriptions
            for (const sigId of effectRunner.deps) {
                const subs = signalSubscribers.get(sigId);
                if (subs) {
                    subs.delete(effectRunner);
                }
            }
            effectRunner.deps.clear();

            // Run effect with dependency tracking
            const prev = currentEffect;
            currentEffect = effectRunner;
            try {
                if (wasmExports && wasmExports.__indirect_function_table) {
                    wasmExports.__indirect_function_table.get(effectPtr)();
                }
            } finally {
                currentEffect = prev;
            }
        }
    };

    effects.set(effectId, effectRunner);

    // Run immediately to establish dependencies
    effectRunner.run();

    console.log(`[signal.effect] id=${effectId} deps=${[...effectRunner.deps].join(',')}`);
    return effectId;
}

// =============================================================================
// Console
// =============================================================================

function consoleLogI64(value) {
    console.log('[sigil]', value);
}

function consoleLogF64(value) {
    console.log('[sigil]', value);
}

function consoleLogStr(ptr, len) {
    console.log('[sigil]', readString(ptr, len));
}

function consolePrint(value) {
    console.log(value);
}

// =============================================================================
// String Operations - All strings are length-prefixed (4-byte len + bytes)
// =============================================================================

function stringConcat(ptr1, ptr2) {
    // Read both strings and concatenate
    const str1 = readLengthPrefixedString(ptr1);
    const str2 = readLengthPrefixedString(ptr2);
    const result = str1 + str2;
    console.log('[string.concat]', JSON.stringify(str1), '+', JSON.stringify(str2), '=', JSON.stringify(result));
    return writeLengthPrefixedString(result);
}

function stringLength(ptr) {
    const str = readLengthPrefixedString(ptr);
    return str.length;
}

function stringSlice(ptr, start, end) {
    const str = readLengthPrefixedString(ptr);
    const result = str.slice(Number(start), Number(end));
    return writeLengthPrefixedString(result);
}

function stringEq(ptr1, ptr2) {
    const str1 = readLengthPrefixedString(ptr1);
    const str2 = readLengthPrefixedString(ptr2);
    return str1 === str2 ? 1 : 0;
}

function stringFromInt(value) {
    const str = String(value);
    console.log('[string.from_int]', value, '=>', JSON.stringify(str));
    return writeLengthPrefixedString(str);
}

function stringFromFloat(value) {
    const str = String(value);
    console.log('[string.from_float]', value, '=>', JSON.stringify(str));
    return writeLengthPrefixedString(str);
}

function stringParseInt(ptr) {
    const str = readLengthPrefixedString(ptr);
    return BigInt(parseInt(str, 10) || 0);
}

function stringParseFloat(ptr) {
    const str = readLengthPrefixedString(ptr);
    return parseFloat(str) || 0.0;
}

// =============================================================================
// DOM Operations - All strings are length-prefixed (4-byte len + bytes)
// =============================================================================

const domElements = new Map();
let nextDomId = 1;

function domCreateElement(tagPtr) {
    const tag = readLengthPrefixedString(tagPtr);
    console.log('[dom.create_element]', tag);
    const el = document.createElement(tag);
    const id = nextDomId++;
    domElements.set(id, el);
    return id;
}

function domCreateText(textPtr) {
    const text = readLengthPrefixedString(textPtr);
    console.log('[dom.create_text]', text);
    const node = document.createTextNode(text);
    const id = nextDomId++;
    domElements.set(id, node);
    return id;
}

function domSetAttribute(elId, namePtr, valuePtr) {
    const el = domElements.get(elId);
    if (el) {
        const name = readLengthPrefixedString(namePtr);
        const value = readLengthPrefixedString(valuePtr);
        console.log('[dom.set_attribute]', elId, name, '=', value);
        el.setAttribute(name, value);
    }
}

function domRemoveAttribute(elId, namePtr) {
    const el = domElements.get(elId);
    if (el) {
        const name = readLengthPrefixedString(namePtr);
        console.log('[dom.remove_attribute]', elId, name);
        el.removeAttribute(name);
    }
}

function domSetProperty(elId, namePtr, value) {
    const el = domElements.get(elId);
    if (el) {
        const name = readLengthPrefixedString(namePtr);
        console.log('[dom.set_property]', elId, name, '=', value);
        el[name] = value;
    }
}

function domAppendChild(parentId, childId) {
    const parent = domElements.get(parentId);
    const child = domElements.get(childId);
    if (parent && child) {
        parent.appendChild(child);
    }
}

function domInsertBefore(parentId, newId, refId) {
    const parent = domElements.get(parentId);
    const newNode = domElements.get(newId);
    const ref = domElements.get(refId);
    if (parent && newNode) {
        parent.insertBefore(newNode, ref);
    }
}

function domRemoveChild(parentId, childId) {
    const parent = domElements.get(parentId);
    const child = domElements.get(childId);
    if (parent && child) {
        parent.removeChild(child);
    }
}

function domReplaceChild(parentId, newId, oldId) {
    const parent = domElements.get(parentId);
    const newNode = domElements.get(newId);
    const oldNode = domElements.get(oldId);
    if (parent && newNode && oldNode) {
        parent.replaceChild(newNode, oldNode);
    }
}

function domSetTextContent(elId, textPtr) {
    const el = domElements.get(elId);
    if (el) {
        const text = readLengthPrefixedString(textPtr);
        console.log('[dom.set_text_content]', elId, text);
        el.textContent = text;
    }
}

function domGetElementById(idPtr) {
    const id = readLengthPrefixedString(idPtr);
    console.log('[dom.get_element_by_id]', id);
    const el = document.getElementById(id);
    if (el) {
        const domId = nextDomId++;
        domElements.set(domId, el);
        return domId;
    }
    return 0;
}

function domQuerySelector(selectorPtr) {
    const selector = readLengthPrefixedString(selectorPtr);
    console.log('[dom.query_selector]', selector);
    const el = document.querySelector(selector);
    if (el) {
        const id = nextDomId++;
        domElements.set(id, el);
        return id;
    }
    return 0;
}

function domCloneNode(elId, deep) {
    const el = domElements.get(elId);
    if (el) {
        const clone = el.cloneNode(!!deep);
        const id = nextDomId++;
        domElements.set(id, clone);
        return id;
    }
    return 0;
}

// =============================================================================
// Events
// =============================================================================

const eventListeners = new Map();
let nextListenerId = 1;

function eventsAddListener(elId, typePtr, typeLen, callbackPtr) {
    const el = domElements.get(elId);
    if (!el) return 0;

    const type = readString(typePtr, typeLen);
    const listener = (event) => {
        if (wasmExports && wasmExports.__indirect_function_table) {
            wasmExports.__indirect_function_table.get(callbackPtr)(0);
        }
    };

    el.addEventListener(type, listener);
    const id = nextListenerId++;
    eventListeners.set(id, { el, type, listener });
    return id;
}

function eventsRemoveListener(listenerId) {
    const info = eventListeners.get(listenerId);
    if (info) {
        info.el.removeEventListener(info.type, info.listener);
        eventListeners.delete(listenerId);
    }
}

function eventsPreventDefault(eventId) {
    // Simplified
}

function eventsStopPropagation(eventId) {
    // Simplified
}

function eventsGetTarget(eventId) {
    return 0;
}

function eventsGetValue(eventId, resultPtr) {
    return 0;
}

// =============================================================================
// Timing
// =============================================================================

function timingNow() {
    return performance.now();
}

function timingSetTimeout(callbackPtr, ms) {
    return setTimeout(() => {
        if (wasmExports && wasmExports.__indirect_function_table) {
            wasmExports.__indirect_function_table.get(callbackPtr)();
        }
    }, ms);
}

function timingClearTimeout(id) {
    clearTimeout(id);
}

function timingSetInterval(callbackPtr, ms) {
    return setInterval(() => {
        if (wasmExports && wasmExports.__indirect_function_table) {
            wasmExports.__indirect_function_table.get(callbackPtr)();
        }
    }, ms);
}

function timingClearInterval(id) {
    clearInterval(id);
}

function timingRequestAnimationFrame(callbackPtr) {
    return requestAnimationFrame((time) => {
        if (wasmExports && wasmExports.__indirect_function_table) {
            wasmExports.__indirect_function_table.get(callbackPtr)(time);
        }
    });
}

// =============================================================================
// Fetch
// =============================================================================

const fetchRequests = new Map();
let nextFetchId = 1;

function fetchStart(urlPtr, urlLen, method) {
    const url = readString(urlPtr, urlLen);
    const id = nextFetchId++;

    fetch(url)
        .then(res => {
            fetchRequests.set(id, { status: res.status, body: null, done: false, response: res });
            return res.text();
        })
        .then(body => {
            const req = fetchRequests.get(id);
            if (req) {
                req.body = body;
                req.done = true;
            }
        })
        .catch(err => {
            fetchRequests.set(id, { status: 0, body: null, done: true, error: err });
        });

    fetchRequests.set(id, { status: 0, body: null, done: false });
    return id;
}

function fetchPoll(id) {
    const req = fetchRequests.get(id);
    return req?.done ? 1 : 0;
}

function fetchGetStatus(id) {
    const req = fetchRequests.get(id);
    return req?.status ?? 0;
}

function fetchGetBody(id) {
    const req = fetchRequests.get(id);
    if (req?.body) {
        return writeLengthPrefixedString(req.body);
    }
    return 0;
}

function fetchGetHeaders(id) {
    const req = fetchRequests.get(id);
    if (req?.response) {
        const headers = {};
        req.response.headers.forEach((value, key) => {
            headers[key] = value;
        });
        return writeLengthPrefixedString(JSON.stringify(headers));
    }
    return 0;
}

function fetchAbort(id) {
    fetchRequests.delete(id);
}

// =============================================================================
// Storage
// =============================================================================

function storageLocalGet(keyPtr) {
    const key = readLengthPrefixedString(keyPtr);
    const value = localStorage.getItem(key);
    console.log('[storage.local_get]', key, '->', value || '(not found)');
    if (value) {
        return writeLengthPrefixedString(value);
    }
    return 0;
}

function storageLocalSet(keyPtr, valuePtr) {
    const key = readLengthPrefixedString(keyPtr);
    const value = readLengthPrefixedString(valuePtr);
    localStorage.setItem(key, value);
    console.log('[storage.local_set]', key, '=', value);
}

function storageLocalRemove(keyPtr) {
    const key = readLengthPrefixedString(keyPtr);
    localStorage.removeItem(key);
    console.log('[storage.local_remove]', key);
}

function storageLocalClear() {
    localStorage.clear();
    console.log('[storage.local_clear]');
}

function storageLocalKeys() {
    const keys = Object.keys(localStorage);
    console.log('[storage.local_keys]', keys);
    // Return as morpheme array handle
    const arrId = arrayNew();
    keys.forEach(k => arrayPush(arrId, writeLengthPrefixedString(k)));
    return arrId;
}

// =============================================================================
// Router
// =============================================================================

function routerPushState(urlPtr) {
    const url = readLengthPrefixedString(urlPtr);
    console.log('[router.push_state]', url);
    history.pushState(null, '', url);
}

function routerReplaceState(urlPtr) {
    const url = readLengthPrefixedString(urlPtr);
    console.log('[router.replace_state]', url);
    history.replaceState(null, '', url);
}

function routerGetPathname() {
    console.log('[router.get_pathname]', location.pathname);
    return writeLengthPrefixedString(location.pathname);
}

function routerGo(delta) {
    console.log('[router.go]', delta);
    history.go(delta);
}

function routerBack() {
    console.log('[router.back]');
    history.back();
}

function routerForward() {
    console.log('[router.forward]');
    history.forward();
}

// =============================================================================
// Memory
// =============================================================================

function memoryAlloc(size) {
    const ptr = heapPtr;
    heapPtr += size;
    // Grow memory if needed
    const pages = Math.ceil(heapPtr / 65536);
    if (wasmMemory && wasmMemory.buffer.byteLength < pages * 65536) {
        wasmMemory.grow(pages - wasmMemory.buffer.byteLength / 65536);
    }
    return ptr;
}

function memoryRealloc(ptr, newSize) {
    // Simplified - just allocate new
    return memoryAlloc(newSize);
}

function memoryFree(ptr) {
    // No-op for bump allocator
}

function heapAlloc(size) {
    return BigInt(memoryAlloc(Number(size)));
}

// =============================================================================
// Math
// =============================================================================

const mathImports = {
    sqrt: Math.sqrt,
    sin: Math.sin,
    cos: Math.cos,
    tan: Math.tan,
    pow: Math.pow,
    exp: Math.exp,
    log: Math.log,
    floor: Math.floor,
    ceil: Math.ceil,
    round: Math.round,
    abs: Math.abs,
    random: Math.random,
};

// =============================================================================
// Morpheme (Array) Operations
// =============================================================================

const arrays = new Map();
let nextArrayId = 1;

function arrayNew() {
    const id = nextArrayId++;
    arrays.set(id, []);
    return id;
}

function arrayPush(arrId, value) {
    const arr = arrays.get(arrId);
    if (arr) arr.push(value);
}

function arrayGet(arrId, index) {
    const arr = arrays.get(arrId);
    return arr ? (arr[index] ?? 0n) : 0n;
}

function arraySet(arrId, index, value) {
    const arr = arrays.get(arrId);
    if (arr) arr[index] = value;
}

function arrayLen(arrId) {
    const arr = arrays.get(arrId);
    return arr ? arr.length : 0;
}

function arrayMap(arrId, fnPtr) {
    // Simplified
    return arrId;
}

function arrayFilter(arrId, fnPtr) {
    return arrId;
}

function arrayReduce(arrId, fnPtr, initial) {
    return initial;
}

function arraySort(arrId) {
    const arr = arrays.get(arrId);
    if (arr) arr.sort((a, b) => Number(a - b));
    return arrId;
}

function arrayFirst(arrId) {
    const arr = arrays.get(arrId);
    return arr && arr.length > 0 ? arr[0] : 0n;
}

function arrayLast(arrId) {
    const arr = arrays.get(arrId);
    return arr && arr.length > 0 ? arr[arr.length - 1] : 0n;
}

function arrayNth(arrId, n) {
    const arr = arrays.get(arrId);
    return arr ? (arr[n] ?? 0n) : 0n;
}

function arraySum(arrId) {
    const arr = arrays.get(arrId);
    return arr ? arr.reduce((a, b) => a + b, 0n) : 0n;
}

function arrayProduct(arrId) {
    const arr = arrays.get(arrId);
    return arr && arr.length > 0 ? arr.reduce((a, b) => a * b, 1n) : 0n;
}

function arrayMin(arrId) {
    const arr = arrays.get(arrId);
    return arr && arr.length > 0 ? arr.reduce((a, b) => a < b ? a : b) : 0n;
}

function arrayMax(arrId) {
    const arr = arrays.get(arrId);
    return arr && arr.length > 0 ? arr.reduce((a, b) => a > b ? a : b) : 0n;
}

function arrayAll(arrId) {
    const arr = arrays.get(arrId);
    return arr && arr.every(x => x) ? 1 : 0;
}

function arrayAny(arrId) {
    const arr = arrays.get(arrId);
    return arr && arr.some(x => x) ? 1 : 0;
}

function arrayRandomElement(arrId) {
    const arr = arrays.get(arrId);
    if (arr && arr.length > 0) {
        return arr[Math.floor(Math.random() * arr.length)];
    }
    return 0n;
}

// Parallel morphemes (simplified - just run sequentially)
function arrayParallelMap(arrId, fnPtr) { return arrayMap(arrId, fnPtr); }
function arrayParallelFilter(arrId, fnPtr) { return arrayFilter(arrId, fnPtr); }
function arrayParallelReduce(arrId, fnPtr, initial) { return arrayReduce(arrId, fnPtr, initial); }

// =============================================================================
// VDOM
// =============================================================================

const vnodes = new Map();
let nextVnodeId = 1;

function vdomCreateVnode(tagStrRef) {
    const id = nextVnodeId++;
    vnodes.set(id, { tag: tagStrRef, props: {}, children: [], isText: false });
    return id;
}

function vdomCreateTextVnode(textStrRef) {
    const id = nextVnodeId++;
    vnodes.set(id, { text: textStrRef, isText: true });
    return id;
}

function vdomCreateFragment() {
    const id = nextVnodeId++;
    vnodes.set(id, { isFragment: true, children: [] });
    return id;
}

function vdomSetVnodeProp(vnodeId, nameStrRef, value) {
    const vnode = vnodes.get(vnodeId);
    if (vnode && !vnode.isText) {
        vnode.props[nameStrRef] = value;
    }
}

function vdomSetVnodeStrProp(vnodeId, nameStrRef, valueStrRef) {
    const vnode = vnodes.get(vnodeId);
    if (vnode && !vnode.isText) {
        vnode.props[nameStrRef] = valueStrRef;
    }
}

function vdomAppendVnodeChild(parentId, childId) {
    const parent = vnodes.get(parentId);
    const child = vnodes.get(childId);
    if (parent && child && !parent.isText) {
        parent.children = parent.children || [];
        parent.children.push(childId);
    }
}

function vdomDiffAndPatch(oldId, newId, domId) {
    // Simplified - would do full diff/patch
}

function vdomMountVnode(vnodeId, selectorStrRef) {
    // Mount vnode to DOM
    return 0;
}

function vdomDispose(vnodeId) {
    vnodes.delete(vnodeId);
}

// =============================================================================
// Promise - Async/Await Support
// =============================================================================

const promises = new Map();
let nextPromiseId = 1;

// Promise states
const PROMISE_PENDING = 0, PROMISE_RESOLVED = 1, PROMISE_REJECTED = 2;

function promiseNew() {
    const id = nextPromiseId++;
    promises.set(id, {
        id,
        state: PROMISE_PENDING,
        value: 0n,
        error: null,
        thenCallbacks: [],
        catchCallbacks: [],
    });
    console.log('[promise.new] ->', id);
    return id;
}

function promiseResolve(id, value) {
    const p = promises.get(id);
    if (!p || p.state !== PROMISE_PENDING) return;
    p.state = PROMISE_RESOLVED;
    p.value = value;
    console.log('[promise.resolve]', id, value);
    // Execute then callbacks
    for (const cb of p.thenCallbacks) {
        try {
            if (typeof cb === 'function') cb(value);
        } catch (e) {
            console.error('[promise] then callback error:', e);
        }
    }
}

function promiseReject(id, errorPtr, errorLen) {
    const p = promises.get(id);
    if (!p || p.state !== PROMISE_PENDING) return;
    p.state = PROMISE_REJECTED;
    p.error = errorPtr ? readLengthPrefixedString(errorPtr) : 'Unknown error';
    console.log('[promise.reject]', id, p.error);
    // Execute catch callbacks
    for (const cb of p.catchCallbacks) {
        try {
            if (typeof cb === 'function') cb(p.error);
        } catch (e) {
            console.error('[promise] catch callback error:', e);
        }
    }
}

function promiseThen(id, callbackTableIdx, envPtr) {
    const p = promises.get(id);
    if (!p) return 0;
    const newPromiseId = promiseNew();
    console.log('[promise.then]', id, '-> new promise', newPromiseId);

    const callback = (value) => {
        console.log('[promise] then callback triggered with', value);
        promiseResolve(newPromiseId, value);
    };

    if (p.state === PROMISE_RESOLVED) {
        setTimeout(() => callback(p.value), 0);
    } else if (p.state === PROMISE_PENDING) {
        p.thenCallbacks.push(callback);
    }
    return newPromiseId;
}

function promiseCatch(id, callbackTableIdx) {
    const p = promises.get(id);
    if (!p) return 0;
    const newPromiseId = promiseNew();
    console.log('[promise.catch]', id, '-> new promise', newPromiseId);

    const callback = (error) => {
        console.log('[promise] catch callback triggered:', error);
        promiseResolve(newPromiseId, 0n);
    };

    if (p.state === PROMISE_REJECTED) {
        setTimeout(() => callback(p.error), 0);
    } else if (p.state === PROMISE_PENDING) {
        p.catchCallbacks.push(callback);
    }
    return newPromiseId;
}

function promiseAll(arrayId) {
    const id = promiseNew();
    console.log('[promise.all] -> promise', id);
    // Simplified: resolve immediately (full impl would wait for all)
    setTimeout(() => promiseResolve(id, 0n), 0);
    return id;
}

function promiseRace(arrayId) {
    const id = promiseNew();
    console.log('[promise.race] -> promise', id);
    // Simplified: resolve immediately (full impl would wait for first)
    setTimeout(() => promiseResolve(id, 0n), 0);
    return id;
}

function promiseSpawn(funcTableIdx) {
    const taskId = nextPromiseId++;
    console.log('[promise.spawn] task', taskId);
    return taskId;
}

function promiseYieldNow() {
    console.log('[promise.yield_now]');
}

function promiseAwait(id) {
    const p = promises.get(id);
    if (!p) return 0n;
    console.log('[promise.await]', id, 'state=', p.state);
    if (p.state === PROMISE_RESOLVED) {
        return p.value;
    }
    console.log('[promise] WARNING: await on pending promise');
    return 0n;
}

function promiseContinuation(stateMachinePtr, nextState) {
    const contId = nextPromiseId++;
    console.log('[promise.continuation]', contId, 'state:', nextState);
    return contId;
}

function promiseResume(stateMachinePtr, value) {
    console.log('[promise.resume] ptr:', stateMachinePtr, 'value:', value);
}

// =============================================================================
// Export Runtime
// =============================================================================

export function createImports() {
    return {
        console: {
            log_i64: consoleLogI64,
            log_f64: consoleLogF64,
            log_str: consoleLogStr,
            print: consolePrint,
        },
        string: {
            concat: stringConcat,
            length: stringLength,
            slice: stringSlice,
            eq: stringEq,
            from_int: stringFromInt,
            from_float: stringFromFloat,
            parse_int: stringParseInt,
            parse_float: stringParseFloat,
        },
        dom: {
            create_element: domCreateElement,
            create_text: domCreateText,
            set_attribute: domSetAttribute,
            remove_attribute: domRemoveAttribute,
            set_property: domSetProperty,
            append_child: domAppendChild,
            insert_before: domInsertBefore,
            remove_child: domRemoveChild,
            replace_child: domReplaceChild,
            set_text_content: domSetTextContent,
            get_element_by_id: domGetElementById,
            query_selector: domQuerySelector,
            clone_node: domCloneNode,
        },
        events: {
            add_listener: eventsAddListener,
            remove_listener: eventsRemoveListener,
            prevent_default: eventsPreventDefault,
            stop_propagation: eventsStopPropagation,
            get_target: eventsGetTarget,
            get_value: eventsGetValue,
        },
        timing: {
            now: timingNow,
            set_timeout: timingSetTimeout,
            clear_timeout: timingClearTimeout,
            set_interval: timingSetInterval,
            clear_interval: timingClearInterval,
            request_animation_frame: timingRequestAnimationFrame,
        },
        fetch: {
            start: fetchStart,
            poll: fetchPoll,
            get_status: fetchGetStatus,
            get_body: fetchGetBody,
            get_headers: fetchGetHeaders,
            abort: fetchAbort,
        },
        storage: {
            local_get: storageLocalGet,
            local_set: storageLocalSet,
            local_remove: storageLocalRemove,
            local_clear: storageLocalClear,
            local_keys: storageLocalKeys,
        },
        router: {
            push_state: routerPushState,
            replace_state: routerReplaceState,
            get_pathname: routerGetPathname,
            go: routerGo,
            back: routerBack,
            forward: routerForward,
        },
        memory: {
            alloc: memoryAlloc,
            realloc: memoryRealloc,
            free: memoryFree,
            heap_alloc: heapAlloc,
        },
        math: mathImports,
        morpheme: {
            array_new: arrayNew,
            array_push: arrayPush,
            array_get: arrayGet,
            array_set: arraySet,
            array_len: arrayLen,
            array_map: arrayMap,
            array_filter: arrayFilter,
            array_reduce: arrayReduce,
            array_sort: arraySort,
            array_first: arrayFirst,
            array_last: arrayLast,
            array_nth: arrayNth,
            array_sum: arraySum,
            array_product: arrayProduct,
            array_min: arrayMin,
            array_max: arrayMax,
            array_all: arrayAll,
            array_any: arrayAny,
            array_random_element: arrayRandomElement,
            array_parallel_map: arrayParallelMap,
            array_parallel_filter: arrayParallelFilter,
            array_parallel_reduce: arrayParallelReduce,
        },
        vdom: {
            create_vnode: vdomCreateVnode,
            create_text_vnode: vdomCreateTextVnode,
            create_fragment: vdomCreateFragment,
            set_vnode_prop: vdomSetVnodeProp,
            set_vnode_str_prop: vdomSetVnodeStrProp,
            append_vnode_child: vdomAppendVnodeChild,
            diff_and_patch: vdomDiffAndPatch,
            mount_vnode: vdomMountVnode,
            dispose: vdomDispose,
        },
        signal: {
            create: signalCreate,
            get: signalGet,
            set: signalSet,
            subscribe: signalSubscribe,
            unsubscribe: signalUnsubscribe,
            batch_start: signalBatchStart,
            batch_end: signalBatchEnd,
            computed: signalComputed,
            effect: signalEffect,
        },
        promise: {
            new: promiseNew,
            resolve: promiseResolve,
            reject: promiseReject,
            then: promiseThen,
            catch: promiseCatch,
            all: promiseAll,
            race: promiseRace,
            spawn: promiseSpawn,
            yield_now: promiseYieldNow,
            await: promiseAwait,
            continuation: promiseContinuation,
            resume: promiseResume,
        },
    };
}

export async function loadWasm(wasmPath, additionalImports = {}) {
    const imports = createImports();

    // Merge additional imports
    Object.assign(imports, additionalImports);

    const response = await fetch(wasmPath);
    const bytes = await response.arrayBuffer();
    const { instance } = await WebAssembly.instantiate(bytes, imports);

    setWasmExports(instance.exports);

    return instance;
}

export default { createImports, loadWasm };
