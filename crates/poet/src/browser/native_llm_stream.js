// Authenticated SSE over fetch: EventSource cannot carry x-qualia-token.
// This stream is owned by one UI session and never retries a failed authority.
export async function readNativeLlmStream(url, token, signal, onData) {
    const response = await fetch(url, {
        headers: { "x-qualia-token": token, "Accept": "text/event-stream" },
        signal,
        redirect: "error",
        cache: "no-store",
    });
    if (!response.ok) {
        throw new Error(`Daemon returned status ${response.status}: ${await response.text()}`);
    }
    if (!response.body) throw new Error("Daemon stream body unavailable");
    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    let pending = "";
    let data = [];
    let frameBytes = 0;
    try {
        while (true) {
            const { done, value } = await reader.read();
            if (done) break;
            pending += decoder.decode(value, { stream: true });
            let newline;
            while ((newline = pending.indexOf("\n")) !== -1) {
                let line = pending.slice(0, newline);
                pending = pending.slice(newline + 1);
                if (line.endsWith("\r")) line = line.slice(0, -1);
                frameBytes += line.length;
                if (frameBytes > 1024 * 1024) throw new Error("Daemon event exceeds 1 MiB budget");
                if (line === "") {
                    if (data.length) {
                        const text = data.join("\n");
                        // Fail malformed events through the same UI error route.
                        const event = JSON.parse(text);
                        onData(text);
                        if (["done", "error", "cancelled"].includes(event.kind)) return;
                    }
                    data = [];
                    frameBytes = 0;
                } else if (line.startsWith("data:")) {
                    data.push(line.slice(5).replace(/^ /, ""));
                }
            }
            if (pending.length > 1024 * 1024) throw new Error("Daemon event line exceeds 1 MiB budget");
        }
        if (!signal.aborted) throw new Error("Daemon stream disconnected before a terminal event");
    } finally {
        await reader.cancel().catch(() => {});
        reader.releaseLock();
    }
}
