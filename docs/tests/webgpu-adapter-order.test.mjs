import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import vm from 'node:vm';

const shimSource = await readFile(new URL('../js/webgpu-limits-shim.js', import.meta.url), 'utf8');

function installShim(userAgent, requestAdapter) {
    class GPU {
        requestAdapter(options) {
            return requestAdapter(options);
        }
    }
    class GPUAdapter {
        requestDevice(descriptor) {
            return Promise.resolve(descriptor);
        }
    }
    const gpu = new GPU();
    const context = {
        console: { debug() {} },
        GPU,
        GPUAdapter,
        navigator: { gpu, userAgent },
    };
    vm.runInNewContext(shimSource, context, { filename: 'webgpu-limits-shim.js' });
    return gpu;
}

{
    const calls = [];
    const compatibilityAdapter = { kind: 'gles-compatibility' };
    const gpu = installShim('Mozilla/5.0 (Linux; Android 16)', async (options) => {
        calls.push(options ?? null);
        return options?.featureLevel === 'compatibility' ? compatibilityAdapter : null;
    });

    const adapter = await gpu.requestAdapter({ powerPreference: 'high-performance' });
    assert.equal(adapter, compatibilityAdapter);
    assert.equal(JSON.stringify(calls), JSON.stringify([{ featureLevel: 'compatibility' }]));
}

{
    const calls = [];
    const defaultAdapter = { kind: 'desktop-default' };
    const gpu = installShim('Mozilla/5.0 (Windows NT 10.0; Win64; x64)', async (options) => {
        calls.push(options ?? null);
        return options == null ? defaultAdapter : null;
    });

    const adapter = await gpu.requestAdapter({ powerPreference: 'high-performance' });
    assert.equal(adapter, defaultAdapter);
    assert.equal(
        JSON.stringify(calls),
        JSON.stringify([{ powerPreference: 'high-performance' }, null]),
    );
}

console.log('webgpu adapter ordering: ok');
