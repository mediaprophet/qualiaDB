import { useState, useMemo, useRef } from 'react';
import { Box, Activity, Sliders, Waves, Layers } from 'lucide-react';
import { Canvas, useFrame } from '@react-three/fiber';
import { OrbitControls, Wireframe } from '@react-three/drei';
import * as THREE from 'three';

function SuperBlockMesh({ temperature, pressure, timeDilation }: { temperature: number, pressure: number, timeDilation: number }) {
  const meshRef = useRef<THREE.Mesh>(null);
  
  // MOCK: Zero-Copy WebGL Architecture
  // In the real implementation, this Float32Array points directly to `wasm.memory.buffer`
  // so the Rust engine can update vertices without JavaScript Garbage Collection pauses.
  const [positions, indices] = useMemo(() => {
    const scale = 1 + (pressure / 100) * 0.5;
    
    // A simplified simulated buffer containing vertex coordinates
    const verts = new Float32Array([
      0, scale, 0,
      scale, 0, scale,
      scale, 0, -scale,
      -scale, 0, -scale,
      -scale, 0, scale,
      0, -scale, 0
    ]);
    
    const idxs = new Uint16Array([
      0, 1, 2,  0, 2, 3,  0, 3, 4,  0, 4, 1,
      5, 2, 1,  5, 3, 2,  5, 4, 3,  5, 1, 4
    ]);
    
    return [verts, idxs];
  }, [pressure]);

  useFrame((state, delta) => {
    if (meshRef.current) {
      // Simulate Rust writing to the buffer to warp the mesh via Thermodynamics
      const warp = Math.sin(state.clock.elapsedTime * timeDilation) * (temperature / 100) * 0.2;
      meshRef.current.scale.set(1 + warp, 1 + warp, 1 + warp);
      meshRef.current.rotation.x += delta * timeDilation;
      meshRef.current.rotation.y += delta * (timeDilation * 1.5);
    }
  });
  
  return (
    <mesh ref={meshRef}>
      <bufferGeometry>
        <bufferAttribute 
          attach="attributes-position"
          count={positions.length / 3}
          array={positions}
          itemSize={3}
        />
        <bufferAttribute 
          attach="index"
          count={indices.length}
          array={indices}
          itemSize={1}
        />
      </bufferGeometry>
      <meshStandardMaterial color={temperature > 70 ? "#ff4444" : "#00f0ff"} wireframe={false} transparent opacity={0.8} />
      <Wireframe thickness={0.05} stroke="#b026ff" />
    </mesh>
  );
}

export default function SpatialPhysics() {
  const [temperature, setTemperature] = useState(50);
  const [pressure, setPressure] = useState(50);
  const [timeDilation, setTimeDilation] = useState(1.0);

  return (
    <div className="flex flex-col gap-6 h-full">
      <div className="flex gap-4">
        <div className="glass-panel flex-1 flex items-center gap-4 py-3">
          <Layers className="text-[#00f0ff]" />
          <div>
            <div className="text-[10px] text-gray-500 font-mono uppercase tracking-widest">Active Payload</div>
            <div className="text-xl font-bold text-white font-mono">.q42_SuperBlock_A1</div>
          </div>
        </div>
        <div className="glass-panel flex-1 flex items-center gap-4 py-3 border-[#ff4444]/30">
          <Activity className="text-[#ff4444]" />
          <div>
            <div className="text-[10px] text-gray-500 font-mono uppercase tracking-widest">Thermodynamic State</div>
            <div className="text-xl font-bold text-white font-mono">{temperature > 70 ? 'CRITICAL' : 'STABLE'}</div>
          </div>
        </div>
      </div>

      <div className="flex gap-6 flex-1 min-h-0">
        {/* Left Viewport: Thermodynamic Controls */}
        <div className="flex-[1] flex flex-col gap-6">
          <div className="glass-panel flex-1 flex flex-col">
            <h2 className="text-sm font-bold border-b border-white/10 pb-2 mb-6 flex items-center gap-2 text-white font-mono">
              <Sliders className="text-[#00f0ff] w-4 h-4" /> Physics Engine Variables
            </h2>
            
            <div className="flex flex-col gap-8 flex-1">
              {/* Temperature Slider */}
              <div>
                <div className="flex justify-between mb-2">
                  <label className="text-xs text-gray-400 font-mono">Ambient Temperature (K)</label>
                  <span className="text-[#ff4444] font-mono text-xs font-bold">{temperature * 10}K</span>
                </div>
                <input 
                  type="range" min="0" max="100" 
                  value={temperature} 
                  onChange={(e) => setTemperature(Number(e.target.value))} 
                  className="w-full accent-[#ff4444]" 
                />
              </div>

              {/* Pressure Slider */}
              <div>
                <div className="flex justify-between mb-2">
                  <label className="text-xs text-gray-400 font-mono">Manifold Pressure (hPa)</label>
                  <span className="text-[#b026ff] font-mono text-xs font-bold">{pressure * 20} hPa</span>
                </div>
                <input 
                  type="range" min="0" max="100" 
                  value={pressure} 
                  onChange={(e) => setPressure(Number(e.target.value))} 
                  className="w-full accent-[#b026ff]" 
                />
              </div>

              {/* Temporal Dilation Slider */}
              <div>
                <div className="flex justify-between mb-2">
                  <label className="text-xs text-gray-400 font-mono">Temporal Velocity (dt)</label>
                  <span className="text-[#00ff88] font-mono text-xs font-bold">x{timeDilation.toFixed(2)}</span>
                </div>
                <input 
                  type="range" min="0" max="5" step="0.1"
                  value={timeDilation} 
                  onChange={(e) => setTimeDilation(Number(e.target.value))} 
                  className="w-full accent-[#00ff88]" 
                />
              </div>
            </div>
            
            <div className="mt-auto p-4 bg-black/60 rounded border border-white/10 text-[10px] font-mono text-gray-500">
              <Waves className="w-3 h-3 mb-2 text-[#00f0ff]" />
              <p>Hardware-accelerated rendering active. Vector bindings updating natively via SPSC Ring Buffers.</p>
            </div>
          </div>
        </div>

        {/* Right Viewport: 3D Render Canvas */}
        <div className="glass-panel flex-[2] flex flex-col relative overflow-hidden">
          <h2 className="text-sm font-bold border-b border-white/10 pb-2 mb-4 flex items-center gap-2 text-white font-mono absolute top-4 left-4 z-10 w-[calc(100%-2rem)]">
            <Box className="text-[#b026ff] w-4 h-4" /> Spatial WebGL Mesh
          </h2>
          
          <div className="flex-1 bg-black/80 rounded-xl border border-white/5 relative overflow-hidden">
            <Canvas camera={{ position: [0, 0, 5] }}>
              <ambientLight intensity={0.5} />
              <pointLight position={[10, 10, 10]} intensity={2} color="#00f0ff" />
              <pointLight position={[-10, -10, -10]} intensity={1} color="#b026ff" />
              <SuperBlockMesh temperature={temperature} pressure={pressure} timeDilation={timeDilation} />
              <OrbitControls autoRotate autoRotateSpeed={timeDilation} enableZoom={false} />
            </Canvas>
          </div>
        </div>
      </div>
    </div>
  );
}
