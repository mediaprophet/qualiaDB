// sync_io_worker.js
// This Web Worker handles synchronous IO operations to bypass the main UI thread's
// async limitations, satisfying the zero-allocation requirements of the Qualia Sentinel VM.

let directoryHandle = null;
let fileHandles = new Map();
let accessHandles = new Map();

self.onmessage = async (e) => {
    const { type, payload, id } = e.data;

    try {
        switch (type) {
            case 'INIT_VAULT':
                directoryHandle = payload.handle;
                self.postMessage({ id, status: 'success' });
                break;

            case 'OPEN_SUPERBLOCK':
                const { filename } = payload;
                const fileHandle = await directoryHandle.getFileHandle(filename, { create: true });
                const accessHandle = await fileHandle.createSyncAccessHandle();
                fileHandles.set(filename, fileHandle);
                accessHandles.set(filename, accessHandle);
                self.postMessage({ id, status: 'success' });
                break;

            case 'READ_BYTES':
                if (!accessHandles.has(payload.filename)) {
                    throw new Error(`File not open: ${payload.filename}`);
                }
                const readHandle = accessHandles.get(payload.filename);
                // Create a buffer for synchronous reading
                const readBuffer = new Uint8Array(payload.length);
                const bytesRead = readHandle.read(readBuffer, { at: payload.offset });
                
                self.postMessage({
                    id,
                    status: 'success',
                    data: readBuffer.buffer,
                    bytesRead
                }, [readBuffer.buffer]);
                break;

            case 'WRITE_BYTES':
                if (!accessHandles.has(payload.filename)) {
                    throw new Error(`File not open: ${payload.filename}`);
                }
                const writeHandle = accessHandles.get(payload.filename);
                
                // Perform synchronous write (zero async allocation block)
                const writeBuffer = new Uint8Array(payload.data);
                const bytesWritten = writeHandle.write(writeBuffer, { at: payload.offset });
                
                // Flush immediately to ensure WAL persistence (important for Android edge nodes)
                writeHandle.flush();

                self.postMessage({ id, status: 'success', bytesWritten });
                break;

            case 'CLOSE_SUPERBLOCK':
                if (accessHandles.has(payload.filename)) {
                    accessHandles.get(payload.filename).close();
                    accessHandles.delete(payload.filename);
                }
                self.postMessage({ id, status: 'success' });
                break;

            default:
                throw new Error(`Unknown operation: ${type}`);
        }
    } catch (err) {
        self.postMessage({ id, status: 'error', error: err.message });
    }
};
