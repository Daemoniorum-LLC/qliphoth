import { readFile } from 'fs/promises';

const wasmPath = process.argv[2];
const bytes = await readFile(wasmPath);

console.log(`Loading ${wasmPath} (${bytes.length} bytes)`);

// Shared memory reference for reading strings
let wasmMemory = null;

// Helper to read length-prefixed string from WASM memory
const readStr = (ptr) => {
    if (!wasmMemory) return `<ptr:${ptr}>`;
    const view = new DataView(wasmMemory.buffer);
    const len = view.getUint32(ptr, true);
    const bytes = new Uint8Array(wasmMemory.buffer, ptr + 4, len);
    return new TextDecoder().decode(bytes);
};

// Heap allocator for dynamic strings
let heapPtr = 1024 * 64;
const writeStr = (str) => {
    if (!wasmMemory) return 0;
    const bytes = new TextEncoder().encode(str);
    const ptr = heapPtr;
    const view = new DataView(wasmMemory.buffer);
    view.setUint32(ptr, bytes.length, true);
    new Uint8Array(wasmMemory.buffer).set(bytes, ptr + 4);
    heapPtr += 4 + bytes.length;
    heapPtr = (heapPtr + 7) & ~7;
    return ptr;
};

// Create minimal imports
const imports = {
    console: {
        log_i64: (v) => console.log('[log_i64]', v),
        log_f64: (v) => console.log('[log_f64]', v),
        log_str: (ptr) => {
            const str = readStr(ptr);
            console.log('[log_str]', str);
        },
        print: (v) => console.log('[print]', v),
    },
    string: {
        concat: (a, b) => {
            const s1 = readStr(a);
            const s2 = readStr(b);
            const result = s1 + s2;
            console.log('[string.concat]', JSON.stringify(s1), '+', JSON.stringify(s2), '=', JSON.stringify(result));
            return writeStr(result);
        },
        length: (ptr) => readStr(ptr).length,
        slice: (ptr, start, end) => writeStr(readStr(ptr).slice(Number(start), Number(end))),
        eq: (a, b) => readStr(a) === readStr(b) ? 1 : 0,
        from_int: (v) => {
            const str = String(v);
            console.log('[string.from_int]', v, '=>', JSON.stringify(str));
            return writeStr(str);
        },
        from_float: (v) => {
            const str = String(v);
            console.log('[string.from_float]', v, '=>', JSON.stringify(str));
            return writeStr(str);
        },
        parse_int: (ptr) => BigInt(parseInt(readStr(ptr), 10) || 0),
        parse_float: (ptr) => parseFloat(readStr(ptr)) || 0.0,
    },
    dom: (() => {
        const elements = new Map();
        let nextId = 1;
        return {
            create_element: (tagPtr) => {
                const tag = readStr(tagPtr);
                console.log('[dom.create_element]', tag);
                const id = nextId++;
                elements.set(id, { type: 'element', tag, attrs: {}, text: '' });
                return id;
            },
            create_text: (textPtr) => {
                const text = readStr(textPtr);
                console.log('[dom.create_text]', text);
                const id = nextId++;
                elements.set(id, { type: 'text', text });
                return id;
            },
            set_attribute: (elId, namePtr, valuePtr) => {
                const name = readStr(namePtr);
                const value = readStr(valuePtr);
                console.log('[dom.set_attribute]', elId, name, '=', value);
                const el = elements.get(elId);
                if (el) el.attrs[name] = value;
            },
            remove_attribute: (elId, namePtr) => {
                const name = readStr(namePtr);
                console.log('[dom.remove_attribute]', elId, name);
            },
            set_property: (elId, namePtr, value) => {
                const name = readStr(namePtr);
                console.log('[dom.set_property]', elId, name, '=', value);
            },
            append_child: (parentId, childId) => {
                console.log('[dom.append_child]', parentId, childId);
            },
            insert_before: () => {},
            remove_child: () => {},
            replace_child: () => {},
            set_text_content: (elId, textPtr) => {
                const text = readStr(textPtr);
                console.log('[dom.set_text_content]', elId, text);
                const el = elements.get(elId);
                if (el) el.text = text;
            },
            get_element_by_id: (idPtr) => {
                const id = readStr(idPtr);
                console.log('[dom.get_element_by_id]', id);
                return 0;
            },
            query_selector: (selectorPtr) => {
                const selector = readStr(selectorPtr);
                console.log('[dom.query_selector]', selector);
                return 0;
            },
            clone_node: () => 0,
        };
    })(),
    events: (() => {
        // Event management state
        const listeners = new Map(); // listenerId -> { elId, eventType, callbackFnPtr }
        const activeEvents = new Map(); // eventId -> event data
        let nextListenerId = 1;
        let nextEventId = 1;
        let wasmInstance = null;

        const setWasm = (inst) => { wasmInstance = inst; };

        // Helper to dispatch callback via function table
        // Handles both simple closures (small table index) and capturing closures (heap pointer)
        const callCallback = (callbackValue, eventId) => {
            if (!wasmInstance || !wasmInstance.exports.__indirect_function_table) {
                console.warn('[events] no function table available');
                return;
            }

            const memory = wasmInstance.exports.memory;
            const view = new DataView(memory.buffer);
            const table = wasmInstance.exports.__indirect_function_table;

            // Detect if this is a simple table index or a closure object pointer
            // Simple closures: small table index (0, 1, 2, ...)
            // Capturing closures: heap pointer (typically >= 1024)
            const isClosureObject = callbackValue >= 1000;

            try {
                if (isClosureObject) {
                    // Check if we need to grow memory
                    const memoryPages = memory.buffer.byteLength / 65536;
                    const neededPages = Math.ceil((callbackValue + 16) / 65536);
                    if (neededPages > memoryPages) {
                        console.log('[events] growing memory from', memoryPages, 'to', neededPages, 'pages');
                        memory.grow(neededPages - memoryPages);
                    }

                    // Re-create view after potential memory growth
                    const currentView = new DataView(memory.buffer);

                    // Debug: dump memory around the closure object
                    console.log('[events] memory dump at', callbackValue, ':');
                    const bytes = new Uint8Array(memory.buffer, callbackValue, 24);
                    console.log('  raw bytes:', [...bytes].map(b => b.toString(16).padStart(2, '0')).join(' '));

                    // Read closure object: [table_idx (8 bytes), env_ptr (8 bytes)]
                    const tableIdx = Number(currentView.getBigInt64(callbackValue, true));
                    const envPtr = currentView.getBigInt64(callbackValue + 8, true);

                    console.log('[events] calling capturing closure', { tableIdx, envPtr, eventId, memSize: memory.buffer.byteLength });

                    const fn = table.get(tableIdx);
                    if (fn) {
                        // Call with env_ptr as first argument (the closure expects it)
                        fn(envPtr);
                    }
                } else {
                    // Simple closure - direct table index, no environment
                    console.log('[events] calling simple callback', callbackValue, 'with eventId', eventId);
                    const fn = table.get(callbackValue);
                    if (fn) {
                        fn(eventId);
                    }
                }
            } catch (e) {
                console.error('[events] callback error:', e);
            }
        };

        return {
            _setWasm: setWasm,

            // Register event listener: (elId, eventTypePtr, callbackFnPtr, flags) -> listenerId
            add_listener: (elId, eventTypePtr, callbackFnPtr, flags) => {
                const eventType = readStr(eventTypePtr);
                const id = nextListenerId++;
                listeners.set(id, { elId, eventType, callbackFnPtr, flags });
                console.log('[events.add_listener]', { id, elId, eventType, callbackFnPtr, flags });
                return id;
            },

            // Remove listener by ID
            remove_listener: (listenerId) => {
                console.log('[events.remove_listener]', listenerId);
                listeners.delete(listenerId);
            },

            // Prevent default behavior
            prevent_default: (eventId) => {
                console.log('[events.prevent_default]', eventId);
                const evt = activeEvents.get(eventId);
                if (evt) evt.defaultPrevented = true;
            },

            // Stop propagation
            stop_propagation: (eventId) => {
                console.log('[events.stop_propagation]', eventId);
                const evt = activeEvents.get(eventId);
                if (evt) evt.propagationStopped = true;
            },

            // Get event target element ID
            get_target: (eventId) => {
                const evt = activeEvents.get(eventId);
                console.log('[events.get_target]', eventId, '=>', evt?.targetElId ?? 0);
                return evt?.targetElId ?? 0;
            },

            // Get event property value
            get_value: (eventId, propPtr) => {
                const prop = readStr(propPtr);
                const evt = activeEvents.get(eventId);
                console.log('[events.get_value]', eventId, prop);
                // For testing, return mock values
                if (prop === 'value' && evt?.value) {
                    return writeStr(evt.value);
                }
                return 0;
            },

            // Test helper: dispatch an event to an element
            _dispatchEvent: (elId, eventType, eventData = {}) => {
                const eventId = nextEventId++;
                activeEvents.set(eventId, {
                    targetElId: elId,
                    type: eventType,
                    ...eventData,
                    defaultPrevented: false,
                    propagationStopped: false,
                });

                console.log('[events._dispatchEvent]', { eventId, elId, eventType });

                // Find matching listeners and call them
                for (const [listenerId, listener] of listeners) {
                    if (listener.elId === elId && listener.eventType === eventType) {
                        callCallback(listener.callbackFnPtr, eventId);
                        if (activeEvents.get(eventId)?.propagationStopped) break;
                    }
                }

                // Cleanup event data after dispatch
                const evt = activeEvents.get(eventId);
                activeEvents.delete(eventId);
                return evt;
            },
        };
    })(),
    timing: {
        now: () => Date.now(),
        set_timeout: () => 0,
        clear_timeout: () => {},
        set_interval: () => 0,
        clear_interval: () => {},
        request_animation_frame: () => 0,
    },
    fetch: (() => {
        // Simulated fetch for Node.js testing
        const requests = new Map();
        let nextHandle = 1;

        // Status constants
        const PENDING = 0, COMPLETE = 1, ERROR = 2;

        return {
            start: (urlPtr, methodPtr, bodyPtr) => {
                const url = readStr(urlPtr);
                const method = methodPtr ? readStr(methodPtr) : 'GET';
                const body = bodyPtr ? readStr(bodyPtr) : null;

                const handle = nextHandle++;
                const request = {
                    url,
                    method,
                    body,
                    status: PENDING,
                    httpStatus: 0,
                    responseBody: '',
                    headers: {},
                };
                requests.set(handle, request);

                console.log('[fetch.start]', method, url, '-> handle', handle);

                // Simulate async completion (in real browser, use actual fetch)
                setTimeout(() => {
                    // Mock responses based on URL patterns
                    if (url.includes('/api/')) {
                        request.status = COMPLETE;
                        request.httpStatus = 200;
                        request.responseBody = JSON.stringify({ success: true, url });
                        request.headers = { 'content-type': 'application/json' };
                    } else if (url.includes('/error')) {
                        request.status = ERROR;
                        request.httpStatus = 500;
                        request.responseBody = 'Internal Server Error';
                    } else {
                        request.status = COMPLETE;
                        request.httpStatus = 200;
                        request.responseBody = `Response from ${url}`;
                        request.headers = { 'content-type': 'text/plain' };
                    }
                    console.log('[fetch] completed:', handle, request.httpStatus);
                }, 10);

                return handle;
            },
            poll: (handle) => {
                const req = requests.get(handle);
                if (!req) return ERROR;
                console.log('[fetch.poll]', handle, '-> status', req.status);
                return req.status;
            },
            get_status: (handle) => {
                const req = requests.get(handle);
                if (!req) return 0;
                console.log('[fetch.get_status]', handle, '->', req.httpStatus);
                return req.httpStatus;
            },
            get_body: (handle) => {
                const req = requests.get(handle);
                if (!req) return 0;
                console.log('[fetch.get_body]', handle, '->', req.responseBody.substring(0, 50));
                return writeStr(req.responseBody);
            },
            get_headers: (handle) => {
                const req = requests.get(handle);
                if (!req) return 0;
                const json = JSON.stringify(req.headers);
                console.log('[fetch.get_headers]', handle, '->', json);
                return writeStr(json);
            },
            abort: (handle) => {
                const req = requests.get(handle);
                if (req && req.status === PENDING) {
                    req.status = ERROR;
                    console.log('[fetch.abort]', handle);
                }
            },
            // Test helper: manually complete a request
            _complete: (handle, status, body) => {
                const req = requests.get(handle);
                if (req) {
                    req.status = COMPLETE;
                    req.httpStatus = status;
                    req.responseBody = body;
                }
            },
        };
    })(),
    storage: (() => {
        // Simulated localStorage for Node.js testing
        const store = new Map();

        return {
            local_get: (keyPtr) => {
                const key = readStr(keyPtr);
                const value = store.get(key);
                console.log('[storage.local_get]', key, '->', value || '(not found)');
                if (value === undefined) return 0;
                return writeStr(value);
            },
            local_set: (keyPtr, valuePtr) => {
                const key = readStr(keyPtr);
                const value = readStr(valuePtr);
                store.set(key, value);
                console.log('[storage.local_set]', key, '=', value);
            },
            local_remove: (keyPtr) => {
                const key = readStr(keyPtr);
                store.delete(key);
                console.log('[storage.local_remove]', key);
            },
            local_clear: () => {
                store.clear();
                console.log('[storage.local_clear]');
            },
            local_keys: () => {
                const keys = Array.from(store.keys());
                console.log('[storage.local_keys]', keys);
                // Return as morpheme array - need to integrate with morpheme system
                // For now, return a pointer to array data
                return 0; // TODO: integrate with morpheme arrays
            },
            // Test helpers
            _dump: () => Object.fromEntries(store),
            _size: () => store.size,
        };
    })(),
    router: (() => {
        // Simulated browser history for Node.js testing
        const history = ['/'];
        let currentIndex = 0;
        const listeners = new Map();
        let listenerId = 0;

        const notifyListeners = () => {
            const pathname = history[currentIndex];
            console.log('[router] pathname changed to:', pathname);
            for (const callback of listeners.values()) {
                if (typeof callback === 'function') {
                    callback(pathname);
                }
            }
        };

        return {
            push_state: (urlPtr) => {
                const url = readStr(urlPtr);
                // Remove forward history when pushing new state
                history.length = currentIndex + 1;
                history.push(url);
                currentIndex = history.length - 1;
                console.log('[router.push_state]', url, '| history:', history);
                notifyListeners();
            },
            replace_state: (urlPtr) => {
                const url = readStr(urlPtr);
                history[currentIndex] = url;
                console.log('[router.replace_state]', url, '| history:', history);
                notifyListeners();
            },
            get_pathname: () => {
                const pathname = history[currentIndex];
                console.log('[router.get_pathname]', pathname);
                return writeStr(pathname);
            },
            // Extension: navigation methods for testing
            go: (delta) => {
                const newIndex = currentIndex + delta;
                if (newIndex >= 0 && newIndex < history.length) {
                    currentIndex = newIndex;
                    console.log('[router.go]', delta, '| now at:', history[currentIndex]);
                    notifyListeners();
                }
            },
            back: () => {
                if (currentIndex > 0) {
                    currentIndex--;
                    console.log('[router.back] now at:', history[currentIndex]);
                    notifyListeners();
                }
            },
            forward: () => {
                if (currentIndex < history.length - 1) {
                    currentIndex++;
                    console.log('[router.forward] now at:', history[currentIndex]);
                    notifyListeners();
                }
            },
            // Test helper: get full history
            _getHistory: () => ({ history: [...history], currentIndex }),
            // Test helper: simulate popstate
            _dispatchPopstate: () => notifyListeners(),
        };
    })(),
    memory: (() => {
        // Simple bump allocator starting at 256KB (well after data section)
        // WASM data section can extend to ~64KB, so we use a higher base
        let heapPointer = 262144; // 256KB
        return {
            alloc: (size) => {
                const ptr = heapPointer;
                heapPointer += size;
                // Align to 8 bytes
                heapPointer = (heapPointer + 7) & ~7;
                console.log('[memory.alloc]', size, '=>', ptr);
                return ptr;
            },
            realloc: (ptr, newSize) => {
                // Simple realloc - just allocate new
                const newPtr = heapPointer;
                heapPointer += newSize;
                heapPointer = (heapPointer + 7) & ~7;
                return newPtr;
            },
            free: () => {},
            heap_alloc: (size) => {
                const s = Number(size);
                const ptr = heapPointer;
                heapPointer += s;
                // Align to 8 bytes
                heapPointer = (heapPointer + 7) & ~7;
                console.log('[memory.heap_alloc]', s, '=>', ptr);
                return BigInt(ptr);
            },
        };
    })(),
    math: {
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
    },
    morpheme: {
        array_new: () => 0,
        array_push: () => {},
        array_get: () => 0n,
        array_set: () => {},
        array_len: () => 0,
        array_map: (a) => a,
        array_filter: (a) => a,
        array_reduce: (a, f, i) => i,
        array_sort: (a) => a,
        array_first: () => 0n,
        array_last: () => 0n,
        array_nth: () => 0n,
        array_sum: () => 0n,
        array_product: () => 0n,
        array_min: () => 0n,
        array_max: () => 0n,
        array_all: () => 0,
        array_any: () => 0,
        array_random_element: () => 0n,
        array_parallel_map: (a) => a,
        array_parallel_filter: (a) => a,
        array_parallel_reduce: (a, f, i) => i,
    },
    vdom: (() => {
        // VNode types
        const ELEMENT = 1;
        const TEXT = 2;
        const FRAGMENT = 3;

        // VNode storage
        const vnodes = new Map();
        let nextVnodeId = 1;

        // DOM element storage (simulated for Node.js)
        const domElements = new Map();
        let nextDomId = 1;

        // Track vnode -> dom mapping
        const vnodeToDom = new Map();

        // Helper to create a new vnode
        const createVNode = (type, data) => {
            const id = nextVnodeId++;
            vnodes.set(id, {
                id,
                type,
                ...data,
                props: {},
                children: [],
            });
            return id;
        };

        // Helper to create simulated DOM element
        const createDomElement = (tag) => {
            const id = nextDomId++;
            domElements.set(id, {
                id,
                tag,
                attrs: {},
                children: [],
                textContent: '',
            });
            return id;
        };

        // Render vnode to DOM (recursive)
        const renderToDom = (vnodeId) => {
            const vnode = vnodes.get(vnodeId);
            if (!vnode) return 0;

            let domId;
            switch (vnode.type) {
                case ELEMENT: {
                    domId = createDomElement(vnode.tag);
                    const dom = domElements.get(domId);
                    // Copy props as attrs
                    Object.assign(dom.attrs, vnode.props);
                    // Render children
                    for (const childVnodeId of vnode.children) {
                        const childDomId = renderToDom(childVnodeId);
                        if (childDomId) {
                            dom.children.push(childDomId);
                        }
                    }
                    break;
                }
                case TEXT: {
                    domId = createDomElement('#text');
                    const dom = domElements.get(domId);
                    dom.textContent = vnode.text;
                    break;
                }
                case FRAGMENT: {
                    // Fragment renders children directly
                    domId = createDomElement('#fragment');
                    const dom = domElements.get(domId);
                    for (const childVnodeId of vnode.children) {
                        const childDomId = renderToDom(childVnodeId);
                        if (childDomId) {
                            dom.children.push(childDomId);
                        }
                    }
                    break;
                }
            }

            vnodeToDom.set(vnodeId, domId);
            return domId;
        };

        // Diff two vnodes and apply patches to DOM
        const diffAndPatchImpl = (oldVnodeId, newVnodeId, containerDomId) => {
            const oldVnode = vnodes.get(oldVnodeId);
            const newVnode = vnodes.get(newVnodeId);
            const container = domElements.get(containerDomId);

            if (!newVnode) {
                // Remove old
                console.log('[vdom.diff] remove', oldVnodeId);
                return;
            }

            if (!oldVnode) {
                // Mount new
                console.log('[vdom.diff] mount new', newVnodeId);
                const domId = renderToDom(newVnodeId);
                if (container) container.children.push(domId);
                return;
            }

            // Different types - replace
            if (oldVnode.type !== newVnode.type) {
                console.log('[vdom.diff] replace', oldVnodeId, 'with', newVnodeId);
                const domId = renderToDom(newVnodeId);
                // In real impl, would replace in parent
                return;
            }

            // Same type - patch
            if (oldVnode.type === TEXT) {
                if (oldVnode.text !== newVnode.text) {
                    console.log('[vdom.diff] update text', JSON.stringify(oldVnode.text), '->', JSON.stringify(newVnode.text));
                    const domId = vnodeToDom.get(oldVnodeId);
                    if (domId) {
                        const dom = domElements.get(domId);
                        if (dom) dom.textContent = newVnode.text;
                    }
                }
            } else if (oldVnode.type === ELEMENT) {
                if (oldVnode.tag !== newVnode.tag) {
                    console.log('[vdom.diff] replace element', oldVnode.tag, '->', newVnode.tag);
                    return;
                }

                // Diff props
                const allProps = new Set([...Object.keys(oldVnode.props), ...Object.keys(newVnode.props)]);
                for (const key of allProps) {
                    if (oldVnode.props[key] !== newVnode.props[key]) {
                        console.log('[vdom.diff] update prop', key, oldVnode.props[key], '->', newVnode.props[key]);
                    }
                }

                // Diff children (simple: by index)
                const maxChildren = Math.max(oldVnode.children.length, newVnode.children.length);
                for (let i = 0; i < maxChildren; i++) {
                    const oldChildId = oldVnode.children[i];
                    const newChildId = newVnode.children[i];
                    const domId = vnodeToDom.get(oldVnodeId);
                    diffAndPatchImpl(oldChildId, newChildId, domId || 0);
                }
            }
        };

        return {
            // Create element vnode: (tagStrRef) -> vnodeId
            create_vnode: (tagPtr) => {
                const tag = readStr(Number(tagPtr));
                const id = createVNode(ELEMENT, { tag });
                console.log('[vdom.create_vnode]', tag, '=>', id);
                return id;
            },

            // Create text vnode: (textStrRef) -> vnodeId
            create_text_vnode: (textPtr) => {
                const text = readStr(Number(textPtr));
                const id = createVNode(TEXT, { text });
                console.log('[vdom.create_text_vnode]', JSON.stringify(text), '=>', id);
                return id;
            },

            // Create fragment: () -> vnodeId
            create_fragment: () => {
                const id = createVNode(FRAGMENT, {});
                console.log('[vdom.create_fragment] =>', id);
                return id;
            },

            // Set numeric/boolean prop: (vnodeId, nameStrRef, value)
            set_vnode_prop: (vnodeId, namePtr, value) => {
                const name = readStr(Number(namePtr));
                const vnode = vnodes.get(vnodeId);
                if (vnode) {
                    vnode.props[name] = value;
                    console.log('[vdom.set_vnode_prop]', vnodeId, name, '=', value);
                }
            },

            // Set string prop: (vnodeId, nameStrRef, valueStrRef)
            set_vnode_str_prop: (vnodeId, namePtr, valuePtr) => {
                const name = readStr(Number(namePtr));
                const value = readStr(Number(valuePtr));
                const vnode = vnodes.get(vnodeId);
                if (vnode) {
                    vnode.props[name] = value;
                    console.log('[vdom.set_vnode_str_prop]', vnodeId, name, '=', JSON.stringify(value));
                }
            },

            // Append child: (parentId, childId)
            append_vnode_child: (parentId, childId) => {
                const parent = vnodes.get(parentId);
                if (parent) {
                    parent.children.push(childId);
                    console.log('[vdom.append_vnode_child]', parentId, '<-', childId);
                }
            },

            // Diff and patch: (oldVnodeId, newVnodeId, containerDomId)
            diff_and_patch: (oldVnodeId, newVnodeId, containerDomId) => {
                console.log('[vdom.diff_and_patch]', oldVnodeId, '->', newVnodeId, 'in container', containerDomId);
                diffAndPatchImpl(oldVnodeId, newVnodeId, containerDomId);
            },

            // Mount vnode to DOM: (vnodeId, selectorStrRef) -> domId
            mount_vnode: (vnodeId, selectorPtr) => {
                const selector = readStr(Number(selectorPtr));
                console.log('[vdom.mount_vnode]', vnodeId, 'to', JSON.stringify(selector));
                const domId = renderToDom(vnodeId);
                console.log('[vdom.mount_vnode] rendered to domId', domId);
                return domId;
            },

            // Dispose vnode: (vnodeId)
            dispose: (vnodeId) => {
                console.log('[vdom.dispose]', vnodeId);
                vnodes.delete(vnodeId);
                vnodeToDom.delete(vnodeId);
            },

            // Test helper: get vnode for inspection
            _getVnode: (id) => vnodes.get(id),
            _getDom: (id) => domElements.get(id),
        };
    })(),
    signal: (() => {
        const signals = new Map();
        const subscribers = new Map();
        let nextId = 1;
        let currentEffect = null;
        let wasmInstance = null;

        const setWasm = (inst) => { wasmInstance = inst; };

        return {
            _setWasm: setWasm,
            create: (v) => {
                const id = nextId++;
                signals.set(id, v);
                subscribers.set(id, new Set());
                console.log('[signal.create]', v);
                return id;
            },
            get: (id) => {
                // Track dependency if inside an effect
                if (currentEffect !== null) {
                    const subs = subscribers.get(id);
                    if (subs) subs.add(currentEffect);
                }
                const v = signals.get(id) ?? 0n;
                console.log('[signal.get]', id, '=>', v);
                return v;
            },
            set: (id, v) => {
                console.log('[signal.set]', id, v);
                signals.set(id, v);
                // Notify subscribers
                const subs = subscribers.get(id);
                if (subs) {
                    subs.forEach(eff => {
                        if (eff !== currentEffect && eff.run) {
                            eff.run();
                        }
                    });
                }
            },
            subscribe: () => 0,
            unsubscribe: () => {},
            batch_start: () => console.log('[signal.batch_start]'),
            batch_end: () => console.log('[signal.batch_end]'),
            computed: (fnPtr) => {
                console.log('[signal.computed]', fnPtr);
                return 0;
            },
            effect: (fnPtr) => {
                console.log('[signal.effect] registering', fnPtr);
                const effectObj = {
                    fnPtr,
                    run: () => {
                        if (wasmInstance && wasmInstance.exports.__indirect_function_table) {
                            const prev = currentEffect;
                            currentEffect = effectObj;
                            try {
                                console.log('[signal.effect] running', fnPtr);
                                wasmInstance.exports.__indirect_function_table.get(fnPtr)();
                            } catch (e) {
                                console.error('[signal.effect] error:', e);
                            } finally {
                                currentEffect = prev;
                            }
                        }
                    }
                };
                // Run immediately to establish dependencies
                effectObj.run();
                return fnPtr;
            },
        };
    })(),
    promise: (() => {
        // Promise runtime for async/await support
        const promises = new Map();
        let nextId = 1;

        // Promise states
        const PENDING = 0, RESOLVED = 1, REJECTED = 2;

        const createPromise = () => {
            const id = nextId++;
            const promise = {
                id,
                state: PENDING,
                value: 0n,
                error: null,
                thenCallbacks: [],
                catchCallbacks: [],
            };
            promises.set(id, promise);
            return id;
        };

        const resolvePromise = (id, value) => {
            const p = promises.get(id);
            if (!p || p.state !== PENDING) return;
            p.state = RESOLVED;
            p.value = value;
            console.log('[promise] id', id, 'resolved with', value);
            // Execute then callbacks
            for (const cb of p.thenCallbacks) {
                try {
                    if (typeof cb === 'function') cb(value);
                } catch (e) {
                    console.error('[promise] then callback error:', e);
                }
            }
        };

        const rejectPromise = (id, errorPtr) => {
            const p = promises.get(id);
            if (!p || p.state !== PENDING) return;
            p.state = REJECTED;
            p.error = errorPtr ? readStr(errorPtr) : 'Unknown error';
            console.log('[promise] id', id, 'rejected:', p.error);
            // Execute catch callbacks
            for (const cb of p.catchCallbacks) {
                try {
                    if (typeof cb === 'function') cb(p.error);
                } catch (e) {
                    console.error('[promise] catch callback error:', e);
                }
            }
        };

        return {
            new: () => {
                const id = createPromise();
                console.log('[promise.new] ->', id);
                return id;
            },
            resolve: (id, value) => {
                console.log('[promise.resolve]', id, value);
                resolvePromise(id, value);
            },
            reject: (id, errorPtr, errorLen) => {
                console.log('[promise.reject]', id);
                rejectPromise(id, errorPtr);
            },
            then: (id, callbackTableIdx, envPtr) => {
                const p = promises.get(id);
                if (!p) return 0;
                const newPromiseId = createPromise();
                console.log('[promise.then]', id, '-> new promise', newPromiseId);

                const callback = (value) => {
                    console.log('[promise] then callback triggered with', value);
                    resolvePromise(newPromiseId, value);
                };

                if (p.state === RESOLVED) {
                    setTimeout(() => callback(p.value), 0);
                } else if (p.state === PENDING) {
                    p.thenCallbacks.push(callback);
                }
                return newPromiseId;
            },
            catch: (id, callbackTableIdx) => {
                const p = promises.get(id);
                if (!p) return 0;
                const newPromiseId = createPromise();
                console.log('[promise.catch]', id, '-> new promise', newPromiseId);

                const callback = (error) => {
                    console.log('[promise] catch callback triggered:', error);
                    resolvePromise(newPromiseId, 0n);
                };

                if (p.state === REJECTED) {
                    setTimeout(() => callback(p.error), 0);
                } else if (p.state === PENDING) {
                    p.catchCallbacks.push(callback);
                }
                return newPromiseId;
            },
            all: (arrayHandle) => {
                const id = createPromise();
                console.log('[promise.all] -> promise', id);
                setTimeout(() => resolvePromise(id, 0n), 0);
                return id;
            },
            race: (arrayHandle) => {
                const id = createPromise();
                console.log('[promise.race] -> promise', id);
                setTimeout(() => resolvePromise(id, 0n), 0);
                return id;
            },
            spawn: (funcTableIdx) => {
                const taskId = nextId++;
                console.log('[promise.spawn] task', taskId);
                return taskId;
            },
            yield_now: () => {
                console.log('[promise.yield_now]');
            },
            await: (id) => {
                const p = promises.get(id);
                if (!p) return 0n;
                console.log('[promise.await]', id, 'state=', p.state);
                if (p.state === RESOLVED) {
                    return p.value;
                }
                console.log('[promise] WARNING: await on pending promise');
                return 0n;
            },
            continuation: (stateMachinePtr, nextState) => {
                const contId = nextId++;
                console.log('[promise.continuation]', contId, 'state:', nextState);
                return contId;
            },
            resume: (stateMachinePtr, value) => {
                console.log('[promise.resume] ptr:', stateMachinePtr, 'value:', value);
            },
            // Test helpers
            _getPromise: (id) => promises.get(id),
            _resolve: (id, value) => resolvePromise(id, BigInt(value)),
            _reject: (id, error) => rejectPromise(id, null),
        };
    })(),
};

try {
    const { instance } = await WebAssembly.instantiate(bytes, imports);
    // Set shared memory reference
    if (instance.exports.memory) {
        wasmMemory = instance.exports.memory;
    }
    // Give signal module access to WASM exports for effects
    if (imports.signal._setWasm) {
        imports.signal._setWasm(instance);
    }
    // Give events module access to WASM exports for callbacks
    if (imports.events._setWasm) {
        imports.events._setWasm(instance);
    }
    console.log('✓ WASM instantiated successfully!');
    console.log('Exports:', Object.keys(instance.exports));

    if (instance.exports.main) {
        console.log('Calling main()...');
        const result = instance.exports.main();
        console.log('main() returned:', result);
    }

    // Test event dispatch if enabled via command line arg
    if (process.argv.includes('--dispatch-events')) {
        console.log('\n--- Testing Event Dispatch ---');
        // Dispatch a click event to element 1 (the button)
        const evtResult = imports.events._dispatchEvent(1, 'click');
        console.log('Event dispatch result:', evtResult);
    }
} catch (e) {
    console.error('✗ WASM error:', e.message);
    process.exit(1);
}
