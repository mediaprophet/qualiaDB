/**
 * Qualia Network Typescript SDK
 * 
 * This provides the core connection interface to the `qualia-core-db` Rust Daemon.
 * We rely on standard JSON/HTTP and WebSockets to maintain complete framework agnosticism.
 */

const API_BASE = "http://127.0.0.1:4242";
const WS_BASE  = "ws://127.0.0.1:4242";

export interface QualiaQuin {
    subject: number;
    predicate: number;
    object: number;
    context: number;
    metadata: number;
    parity: number;
}

export interface SimulationResult {
    mean: number;
    value_at_risk: number;
}

export class QualiaClient {
    private ws: WebSocket | null = null;

    /**
     * Executes a CBOR-LD/N3 logical query against the local node.
     */
    async queryGraph(query: string): Promise<QualiaQuin[]> {
        console.log(`[QualiaClient] Executing Query: ${query}`);
        try {
            const res = await fetch(`${API_BASE}/query`, {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({ query })
            });
            if (!res.ok) throw new Error("Graph query failed");
            return await res.json();
        } catch (e) {
            console.error(e);
            return [];
        }
    }

    /**
     * Invokes the advanced scientific compute engine in the backend daemon.
     */
    async runSimulation(initialPrice: number, drift: number, volatility: number, time: number, steps: number): Promise<SimulationResult | null> {
        console.log(`[QualiaClient] Dispatching simulation task...`);
        try {
            const res = await fetch(`${API_BASE}/simulate`, {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({ initial_price: initialPrice, drift, volatility, time_horizon: time, steps })
            });
            if (!res.ok) throw new Error("Simulation failed");
            return await res.json();
        } catch (e) {
            console.error(e);
            return null;
        }
    }

    /**
     * Connects to the Web Civics P2P mesh event stream via WebSocket.
     */
    connectBridge(onMessage: (msg: any) => void) {
        if (this.ws) this.ws.close();
        
        this.ws = new WebSocket(`${WS_BASE}/qualia-bridge`);
        
        this.ws.onopen = () => {
            console.log("[QualiaClient] Bridge Connected.");
        };

        this.ws.onmessage = (event) => {
            try {
                const data = JSON.parse(event.data);
                onMessage(data);
            } catch (e) {
                console.warn("Received non-JSON bridge message:", event.data);
                onMessage({ raw: event.data });
            }
        };

        this.ws.onclose = () => {
            console.log("[QualiaClient] Bridge Disconnected. Retrying in 5s...");
            setTimeout(() => this.connectBridge(onMessage), 5000);
        };
    }
}
