// Canonical etalon-wasm/js/worker_transport.js; deployment copies are refreshed verbatim.
const MAGIC = 0x45545352;
const HEADER = 192;
const CAPACITY = 32;
const SLOT_BYTES = 8192;
const MESSAGE_BUDGET = 64 * 1024;
const DEFAULT_BINARY_BUDGET = 1024 * 1024 * 1024;
const encoder = new TextEncoder();
const decoder = new TextDecoder('utf-8', { fatal: true });

function packet(value) {
    const json = JSON.stringify(value);
    if (json.length > MESSAGE_BUDGET) throw new Error('Etalon worker message budget exceeded');
    const payload = encoder.encode(json);
    if (payload.length > MESSAGE_BUDGET) throw new Error('Etalon worker message budget exceeded');
    const bytes = new Uint8Array(16 + payload.length);
    const header = new DataView(bytes.buffer);
    header.setBigUint64(0, BigInt(value.id ?? 0), true);
    header.setUint32(8, 1, true);
    header.setUint32(12, payload.length, true);
    bytes.set(payload, 16);
    return bytes;
}

function unpack(bytes) {
    if (bytes.length < 16 || bytes.length > MESSAGE_BUDGET + 16) throw new Error('Invalid Etalon packet size');
    const header = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
    if (header.getUint32(8, true) !== 1 || header.getUint32(12, true) !== bytes.length - 16) {
        throw new Error('Invalid Etalon packet header');
    }
    // Browser TextDecoder rejects views backed by SharedArrayBuffer. Copy only
    // the bounded JSON payload before decoding; the ring slot remains owned
    // by this reader until pop advances its tail.
    const value = JSON.parse(decoder.decode(bytes.slice(16)));
    if (BigInt(value.id ?? 0) !== header.getBigUint64(0, true)) throw new Error('Etalon packet identity mismatch');
    return value;
}

function sharedAvailable() {
    return globalThis.crossOriginIsolated === true && typeof SharedArrayBuffer === 'function';
}

class Ring {
    constructor(buffer, initialize = false) {
        if (!sharedAvailable() || !(buffer instanceof SharedArrayBuffer) ||
            buffer.byteLength !== HEADER + CAPACITY * SLOT_BYTES) {
            throw new Error('Invalid Etalon shared worker ring');
        }
        this.buffer = buffer;
        this.words = new Int32Array(buffer, 0, HEADER / 4);
        this.bytes = new Uint8Array(buffer);
        if (initialize) {
            this.words[0] = MAGIC;
            this.words[1] = (SLOT_BYTES << 16) | 1;
            this.words[2] = CAPACITY;
        }
        if (this.words[0] !== MAGIC || this.words[1] !== ((SLOT_BYTES << 16) | 1) ||
            this.words[2] !== CAPACITY) throw new Error('Unsupported Etalon ring ABI');
    }

    push(bytes) {
        if (bytes.length > SLOT_BYTES - 4) return false;
        const head = Atomics.load(this.words, 16) >>> 0;
        const tail = Atomics.load(this.words, 32) >>> 0;
        if (((head - tail) >>> 0) >= CAPACITY) return false;
        const offset = HEADER + (head % CAPACITY) * SLOT_BYTES;
        new DataView(this.buffer).setUint32(offset, bytes.length, true);
        this.bytes.set(bytes, offset + 4);
        Atomics.store(this.words, 16, (head + 1) | 0);
        return true;
    }

    pop() {
        const tail = Atomics.load(this.words, 32) >>> 0;
        const head = Atomics.load(this.words, 16) >>> 0;
        const available = (head - tail) >>> 0;
        if (available === 0) return null;
        if (available > CAPACITY) throw new Error('Corrupt Etalon ring counters');
        const offset = HEADER + (tail % CAPACITY) * SLOT_BYTES;
        const size = new DataView(this.buffer).getUint32(offset, true);
        if (size > SLOT_BYTES - 4) throw new Error('Corrupt Etalon ring payload');
        const value = unpack(this.bytes.subarray(offset + 4, offset + 4 + size));
        Atomics.store(this.words, 32, (tail + 1) | 0);
        return value;
    }
}

export class EtalonWorkerTransport {
    constructor(port, receive, initiate = false, binaryByteBudget = DEFAULT_BINARY_BUDGET) {
        if (!Number.isSafeInteger(binaryByteBudget) || binaryByteBudget < 0) throw new Error('Invalid worker binary budget');
        this.binaryByteBudget = binaryByteBudget;
        this.port = port;
        this.receive = receive;
        this.incoming = null;
        this.outgoing = null;
        this.configured = initiate;
        if (initiate) port.postMessage({ etalonTransport: 'configure', binaryByteBudget });
        if (initiate && sharedAvailable()) {
            const tx = new Ring(new SharedArrayBuffer(HEADER + CAPACITY * SLOT_BYTES), true);
            const rx = new Ring(new SharedArrayBuffer(HEADER + CAPACITY * SLOT_BYTES), true);
            // SAB setup is structured-cloned; it must never be included in a transfer list.
            try {
                port.postMessage({ etalonTransport: 'attach', incoming: tx.buffer, outgoing: rx.buffer });
                this.outgoing = tx;
                this.incoming = rx;
            } catch {
                // Hosts that deny SAB structured cloning retain transferable message transport.
            }
        }
    }

    get shared() { return this.incoming !== null && this.outgoing !== null; }

    send(value, transfer = []) {
        // Large model ownership already has an explicit ArrayBuffer transfer contract.
        if (transfer.length) {
            let bytes = 0;
            for (const buffer of transfer) {
                if (!(buffer instanceof ArrayBuffer)) throw new Error('Expected transferable ArrayBuffer');
                bytes += buffer.byteLength;
                if (bytes > this.binaryByteBudget) throw new Error('Etalon binary transfer budget exceeded');
            }
            this.port.postMessage(value, transfer);
            return;
        }
        const bytes = packet(value);
        if (this.outgoing?.push(bytes)) {
            this.port.postMessage({ etalonTransport: 'wake' });
        } else {
            const buffer = bytes.buffer;
            this.port.postMessage({ etalonTransport: 'message', buffer }, [buffer]);
        }
    }

    handle(data) {
        if (data?.etalonTransport === 'configure') {
            if (this.configured || !Number.isSafeInteger(data.binaryByteBudget) || data.binaryByteBudget < 0) {
                throw new Error('Invalid Etalon worker budget configuration');
            }
            this.binaryByteBudget = data.binaryByteBudget;
            this.configured = true;
            return;
        }
        if (data?.etalonTransport === 'attach') {
            if (this.shared) throw new Error('Etalon worker transport already attached');
            this.incoming = new Ring(data.incoming);
            this.outgoing = new Ring(data.outgoing);
            return;
        }
        if (data?.etalonTransport === 'wake') {
            if (!this.incoming) throw new Error('Etalon worker ring not attached');
            // One wake consumes one slot, preserving order with transferable overflow messages.
            const value = this.incoming.pop();
            if (value !== null) this.receive(value);
            return;
        }
        if (data?.etalonTransport === 'message') {
            if (!(data.buffer instanceof ArrayBuffer) || data.buffer.byteLength > MESSAGE_BUDGET + 16) {
                throw new Error('Invalid Etalon transferable worker message');
            }
            this.receive(unpack(new Uint8Array(data.buffer)));
            return;
        }
        this.receive(data);
    }
}
