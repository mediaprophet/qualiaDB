// Test-only dedicated worker. All command/results traverse canonical transport.
import { EtalonWorkerTransport } from '/transport.js';
import initialize, * as q from '/qualia.js';
const ready = initialize();
let canvas;
let busy = false;
const transport = new EtalonWorkerTransport(self, async message => {
  const id = message.id;
  try {
    if (busy) throw new Error('One lifecycle scenario at a time');
    busy = true;
    await ready;
    if (message.command === 'attach') {
      if (canvas || !(message.canvas instanceof OffscreenCanvas)) throw new Error('Canvas ownership invalid');
      canvas = message.canvas;
      transport.send({ id, attached: true, shared: transport.shared });
    } else if (message.command === 'run') {
      if (!canvas) throw new Error('No owned canvas');
      if (typeof q.runRenderLifecycleHarness !== 'function') throw new Error('Rebuild with device-harness: missing runRenderLifecycleHarness');
      const result = JSON.parse(await q.runRenderLifecycleHarness(transport.shared, message.action));
      transport.send({ id, shared: transport.shared, result });
    } else if (message.command === 'detach') {
      const returned = canvas;
      canvas = undefined;
      // Actual transfer back permits the next worker to own this same canvas.
      // Canonical transport intentionally admits ArrayBuffer binary payloads only;
      // OffscreenCanvas ownership uses the browser's native structured transfer.
      self.postMessage({ id, canvas: returned, shared: transport.shared }, [returned]);
    } else throw new Error('Unknown lifecycle command');
  } catch (error) { transport.send({ id, error: String(error) }); }
  finally { busy = false; }
});
self.onmessage = event => transport.handle(event.data);
