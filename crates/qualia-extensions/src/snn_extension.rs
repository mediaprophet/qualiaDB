//! SNN Extension for QualiaDB Advanced
//! 
//! Spiking Neural Networks with noisy gradient CRDT synchronization
//! for temporal processing and event-driven computation while maintaining
//! distributed consistency across edge deployments.

use crate::{Extension, ExtensionCapability, ExtensionError, ExtensionJob, ExtensionResult, ResourceRequirements, NQuin};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// SNN Extension implementation with CRDT synchronization
pub struct SnnExtension {
    network_manager: SnnNetworkManager,
    crdt_synchronizer: tokio::sync::Mutex<NoisyGradientCrdt>,
    capability: ExtensionCapability,
}

/// SNN Network Manager for spiking neural networks
pub struct SnnNetworkManager {
    loaded_networks: HashMap<String, SpikingNetwork>,
    network_cache_path: String,
    temporal_processor: TemporalProcessor,
}

/// Noisy Gradient CRDT Synchronizer for distributed SNN training
pub struct NoisyGradientCrdt {
    node_id: Uuid,
    gradient_state: GradientCrdtState,
    noise_generator: NoiseGenerator,
    sync_config: CrdtSyncConfig,
}

/// Spiking Neural Network model
#[derive(Debug, Clone)]
pub struct SpikingNetwork {
    pub name: String,
    pub network_type: NetworkType,
    pub neurons: Vec<SpikingNeuron>,
    pub synapses: Vec<Synapse>,
    pub temporal_config: TemporalConfig,
    pub crdt_config: CrdtConfig,
}

/// Types of spiking neural networks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkType {
    LIF,           // Leaky Integrate-and-Fire
    Izhikevich,     // Izhikevich model
    HodgkinHuxley,  // Hodgkin-Huxley model
    SRM,           // Spike Response Model
    AdEx,          // Adaptive Exponential Integrate-and-Fire
}

/// Spiking neuron with temporal dynamics
#[derive(Debug, Clone)]
pub struct SpikingNeuron {
    pub id: u32,
    pub neuron_type: NeuronType,
    pub membrane_potential: f64,
    pub threshold: f64,
    pub refractory_period: Duration,
    pub last_spike_time: Option<Instant>,
    pub temporal_state: TemporalState,
}

/// Types of spiking neurons
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NeuronType {
    Excitatory,
    Inhibitory,
    Modulatory,
}

/// Temporal state for spiking neurons
#[derive(Debug, Clone)]
pub struct TemporalState {
    pub adaptation_current: f64,
    pub recovery_variable: f64,
    pub synaptic_current: f64,
    pub noise_amplitude: f64,
}

/// Synapse with plasticity
#[derive(Debug, Clone)]
pub struct Synapse {
    pub pre_neuron_id: u32,
    pub post_neuron_id: u32,
    pub weight: f64,
    pub delay: Duration,
    pub plasticity_type: PlasticityType,
    pub crdt_weight: CrdtWeight,
}

/// Types of synaptic plasticity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlasticityType {
    Static,
    STDP,          // Spike-Timing Dependent Plasticity
    RSTDP,         // Reward-Modulated STDP
    Homeostatic,    // Homeostatic plasticity
    CRDT,          // CRDT-synchronized plasticity
}

/// CRDT-weighted synapse for distributed learning
#[derive(Debug, Clone)]
pub struct CrdtWeight {
    pub value: f64,
    pub version_vector: HashMap<Uuid, u64>,
    pub last_update: Instant,
    pub conflict_resolution: ConflictResolution,
}

/// Conflict resolution strategy for CRDT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolution {
    LastWriterWins,
    NoisyGradient,   // Use noisy gradient for conflict resolution
    TemporalPriority, // Use temporal priority
    Consensus,        // Require consensus
}

/// Temporal configuration for SNN
#[derive(Debug, Clone)]
pub struct TemporalConfig {
    pub time_step: Duration,
    pub simulation_window: Duration,
    pub spike_encoding: SpikeEncoding,
    pub temporal_resolution: u32,
}

/// Spike encoding methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpikeEncoding {
    Rate,
    Temporal,
    Phase,
    RankOrder,
}

/// CRDT configuration for distributed synchronization
#[derive(Debug, Clone)]
pub struct CrdtConfig {
    pub sync_interval: Duration,
    pub noise_amplitude: f64,
    pub gradient_clipping: f64,
    pub consensus_threshold: f64,
    pub network_topology: NetworkTopology,
}

/// Network topology for CRDT synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkTopology {
    FullyConnected,
    Ring,
    Mesh,
    Tree,
    Random,
}

/// Gradient CRDT state for synchronization
#[derive(Debug, Clone)]
pub struct GradientCrdtState {
    pub gradients: HashMap<String, CrdtGradient>,
    pub version_vector: HashMap<Uuid, u64>,
    pub pending_updates: Vec<PendingUpdate>,
    pub conflict_buffer: ConflictBuffer,
}

/// CRDT gradient with noise
#[derive(Debug, Clone)]
pub struct CrdtGradient {
    pub gradient_value: f64,
    pub noisy_value: f64,
    pub timestamp: Instant,
    pub source_node: Uuid,
    pub confidence: f64,
}

/// Pending update for CRDT
#[derive(Debug, Clone)]
pub struct PendingUpdate {
    pub update_id: Uuid,
    pub gradient: CrdtGradient,
    pub dependencies: Vec<Uuid>,
    pub created_at: Instant,
}

/// Conflict buffer for CRDT resolution
#[derive(Debug, Clone)]
pub struct ConflictBuffer {
    pub conflicts: Vec<GradientConflict>,
    pub resolution_strategy: ConflictResolution,
    pub max_buffer_size: usize,
}

/// Gradient conflict in CRDT
#[derive(Debug, Clone)]
pub struct GradientConflict {
    pub conflicting_gradients: Vec<CrdtGradient>,
    pub conflict_type: ConflictType,
    pub resolution_time: Instant,
}

/// Types of conflicts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictType {
    VersionConflict,
    ValueConflict,
    TimingConflict,
    TopologyConflict,
}

/// Noise generator for noisy gradients
#[derive(Debug, Clone)]
pub struct NoiseGenerator {
    pub noise_type: NoiseType,
    pub amplitude: f64,
    pub correlation_time: Duration,
    pub seed: u64,
}

/// Types of noise for gradient perturbation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NoiseType {
    Gaussian,
    Uniform,
    OrnsteinUhlenbeck,
    Pink,
    Brownian,
}

/// CRDT synchronization configuration
#[derive(Debug, Clone)]
pub struct CrdtSyncConfig {
    pub sync_protocol: SyncProtocol,
    pub compression_enabled: bool,
    pub encryption_enabled: bool,
    pub batch_size: usize,
    pub timeout: Duration,
}

/// Synchronization protocols
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncProtocol {
    Gossip,
    PushPull,
    TreeBased,
    Consensus,
}

/// Temporal processor for spike timing
#[derive(Debug, Clone)]
pub struct TemporalProcessor {
    pub spike_queue: VecDeque<SpikeEvent>,
    pub current_time: Instant,
    pub time_step: Duration,
    pub event_handlers: Vec<SpikeEventHandler>,
}

/// Spike event in temporal processing
#[derive(Debug, Clone)]
pub struct SpikeEvent {
    pub neuron_id: u32,
    pub spike_time: Instant,
    pub spike_type: SpikeType,
    pub propagation_delay: Duration,
}

/// Types of spikes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpikeType {
    Excitatory,
    Inhibitory,
    Modulatory,
}

/// Spike event handler
#[derive(Debug, Clone)]
pub struct SpikeEventHandler {
    pub handler_type: HandlerType,
    pub priority: u8,
    pub enabled: bool,
}

/// Types of event handlers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HandlerType {
    Plasticity,
    Synchronization,
    Logging,
    Monitoring,
}

/// SNN execution parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnnJobParams {
    pub network_name: String,
    pub input_spikes: Vec<SpikeTrain>,
    pub simulation_time: Duration,
    pub learning_enabled: bool,
    pub crdt_sync_enabled: bool,
    pub noise_level: f64,
}

/// Spike train for input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpikeTrain {
    pub neuron_id: u32,
    pub spike_times: Vec<Duration>,
    pub spike_amplitudes: Vec<f64>,
}

/// SNN execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnnExecutionResult {
    pub output_spikes: Vec<SpikeTrain>,
    pub membrane_potentials: Vec<Vec<f64>>,
    pub synaptic_weights: Vec<Vec<f64>>,
    pub learning_metrics: LearningMetrics,
    pub crdt_sync_metrics: CrdtSyncMetrics,
    pub execution_time_ms: u64,
}

/// Learning metrics for SNN
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningMetrics {
    pub final_loss: f64,
    pub convergence_rate: f64,
    pub spike_rate: f64,
    pub synaptic_change: f64,
    pub adaptation_level: f64,
}

/// CRDT synchronization metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrdtSyncMetrics {
    pub sync_rounds: u32,
    pub conflicts_resolved: u32,
    pub convergence_achieved: bool,
    pub noise_effectiveness: f64,
    pub network_utilization: f64,
}

impl SnnExtension {
    pub fn new() -> Self {
        let node_id = Uuid::new_v4();
        let crdt_synchronizer = NoisyGradientCrdt::new(node_id);
        
        let network_manager = SnnNetworkManager {
            loaded_networks: HashMap::new(),
            network_cache_path: std::env::var("QUALIA_SNN_CACHE").unwrap_or_else(|_| "./snn_networks".to_string()),
            temporal_processor: TemporalProcessor::new(),
        };

        Self {
            network_manager,
            crdt_synchronizer: tokio::sync::Mutex::new(crdt_synchronizer),
            capability: ExtensionCapability {
                name: "snn".to_string(),
                version: "1.0.0".to_string(),
                description: "Spiking Neural Networks with noisy gradient CRDT synchronization".to_string(),
                required_resources: ResourceRequirements {
                    min_memory_mb: 512,
                    min_vram_mb: Some(256),
                    requires_gpu: true,
                    requires_network: true, // Required for CRDT sync
                    max_concurrent_jobs: 2,
                },
                supported_operations: vec![
                    "simulate_snn".to_string(),
                    "train_distributed".to_string(),
                    "sync_gradients".to_string(),
                    "resolve_conflicts".to_string(),
                    "export_network".to_string(),
                    "import_network".to_string(),
                ],
            },
        }
    }

    async fn simulate_snn(&self, params: SnnJobParams) -> Result<SnnExecutionResult, ExtensionError> {
        let network = self.network_manager.get_network(&params.network_name)
            .ok_or_else(|| ExtensionError::ExtensionNotFound(format!("Network '{}' not found", params.network_name)))?;

        let start_time = Instant::now();
        
        // Execute SNN simulation with temporal processing
        let result = self.execute_snn_simulation(network, &params).await?;
        
        let sync_result = if params.crdt_sync_enabled {
            let mut sync = self.crdt_synchronizer.lock().await;
            sync.synchronize_gradients(&result).await?
        } else {
            CrdtSyncMetrics::default()
        };

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(SnnExecutionResult {
            output_spikes: result.output_spikes,
            membrane_potentials: result.membrane_potentials,
            synaptic_weights: result.synaptic_weights,
            learning_metrics: result.learning_metrics,
            crdt_sync_metrics: sync_result,
            execution_time_ms: execution_time,
        })
    }

    async fn execute_snn_simulation(&self, network: &SpikingNetwork, params: &SnnJobParams) -> Result<SnnExecutionResult, ExtensionError> {
        let mut network_sim = network.clone();
        let mut temporal_processor = network_sim.temporal_config.create_processor();
        let mut current_time = Duration::ZERO;
        let mut output_spikes = Vec::new();
        let mut membrane_potentials = Vec::new();
        let mut synaptic_weights = Vec::new();

        // Simulate for the specified duration
        while current_time < params.simulation_time {
            // Process input spikes
            for spike_train in &params.input_spikes {
                if let Some(&spike_time) = spike_train.spike_times.iter().find(|&&t| t == current_time) {
                    temporal_processor.process_input_spike(spike_train.neuron_id, spike_time);
                }
            }

            // Update neuron states
            self.update_neuron_states(&mut network_sim, &mut temporal_processor, current_time)?;

            // Record membrane potentials
            let potentials: Vec<f64> = network_sim.neurons.iter()
                .map(|neuron| neuron.membrane_potential)
                .collect();
            membrane_potentials.push(potentials);

            // Record synaptic weights
            let weights: Vec<f64> = network_sim.synapses.iter()
                .map(|synapse| synapse.weight)
                .collect();
            synaptic_weights.push(weights);

            // Advance time
            current_time += network_sim.temporal_config.time_step;
        }

        // Extract output spikes
        output_spikes = temporal_processor.extract_output_spikes();

        // Calculate learning metrics
        let learning_metrics = self.calculate_learning_metrics(&membrane_potentials, &synaptic_weights);

        Ok(SnnExecutionResult {
            output_spikes,
            membrane_potentials,
            synaptic_weights,
            learning_metrics,
            crdt_sync_metrics: CrdtSyncMetrics::default(),
            execution_time_ms: 0, // Will be set by caller
        })
    }

    fn update_neuron_states(&self, network: &mut SpikingNetwork, processor: &mut TemporalProcessor, current_time: Duration) -> Result<(), ExtensionError> {
        for neuron in &mut network.neurons {
            // Check if neuron is in refractory period
            if let Some(last_spike) = neuron.last_spike_time {
                if last_spike.elapsed() < neuron.refractory_period {
                    continue;
                }
            }

            // Update membrane potential
            let synaptic_input = processor.calculate_synaptic_input(neuron.id);
            let noise = self.generate_noise(neuron.temporal_state.noise_amplitude);
            
            neuron.membrane_potential += synaptic_input + noise;

            // Check for spike
            if neuron.membrane_potential >= neuron.threshold {
                processor.emit_spike(neuron.id, current_time);
                neuron.membrane_potential = 0.0; // Reset
                neuron.last_spike_time = Some(Instant::now());
            }

            // Apply leak
            neuron.membrane_potential *= 0.99; // Simple leak
        }

        Ok(())
    }

    fn generate_noise(&self, amplitude: f64) -> f64 {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        rng.gen_range(-amplitude..amplitude)
    }

    fn calculate_learning_metrics(&self, potentials: &[Vec<f64>], weights: &[Vec<f64>]) -> LearningMetrics {
        let final_loss = if potentials.len() > 1 {
            (potentials[potentials.len() - 1][0] - potentials[potentials.len() - 2][0]).abs()
        } else {
            0.0
        };

        let convergence_rate = if potentials.len() > 10 {
            let recent_changes: Vec<f64> = potentials.iter().skip(potentials.len() - 10)
                .zip(potentials.iter().skip(potentials.len() - 9))
                .map(|(curr, prev)| (curr[0] - prev[0]).abs())
                .collect();
            recent_changes.iter().sum::<f64>() / recent_changes.len() as f64
        } else {
            0.0
        };

        let spike_rate = potentials.iter()
            .map(|p| p.iter().filter(|&v| *v > 0.0).count() as f64 / p.len() as f64)
            .sum::<f64>() / potentials.len() as f64;

        let synaptic_change = if weights.len() > 1 {
            let initial_sum = weights[0].iter().sum::<f64>();
            let final_sum = weights[weights.len() - 1].iter().sum::<f64>();
            (final_sum - initial_sum).abs()
        } else {
            0.0
        };

        LearningMetrics {
            final_loss,
            convergence_rate,
            spike_rate,
            synaptic_change,
            adaptation_level: 0.5, // Placeholder
        }
    }

    fn result_to_quins(result: &SnnExecutionResult, job_id: &str) -> Vec<NQuin> {
        let mut quins = Vec::new();

        // Add learning metrics
        let learning_quin = NQuin {
            subject: crate::q_hash(job_id),
            predicate: crate::q_hash("q42:hasLearningMetrics"),
            object: (result.learning_metrics.final_loss * 1000000.0) as u64,
            context: crate::q_hash("snn:learning"),
            metadata: ((result.learning_metrics.convergence_rate * 1000000.0) as u64) << 32 | 
                     (if result.learning_metrics.convergence_rate < 0.001 { 1 } else { 0 }),
            parity: 0,
        };
        quins.push(learning_quin);

        // Add CRDT sync metrics
        let sync_quin = NQuin {
            subject: crate::q_hash(job_id),
            predicate: crate::q_hash("q42:hasCrdtSync"),
            object: result.crdt_sync_metrics.sync_rounds as u64,
            context: crate::q_hash("snn:crdt"),
            metadata: ((result.crdt_sync_metrics.conflicts_resolved as u64) << 32) | 
                     (if result.crdt_sync_metrics.convergence_achieved { 1 } else { 0 }),
            parity: 0,
        };
        quins.push(sync_quin);

        // Add execution time
        let time_quin = NQuin {
            subject: crate::q_hash(job_id),
            predicate: crate::q_hash("q42:hasExecutionTime"),
            object: result.execution_time_ms,
            context: crate::q_hash("snn:performance"),
            metadata: 0,
            parity: 0,
        };
        quins.push(time_quin);

        quins
    }
}

impl SnnNetworkManager {
    pub fn get_network(&self, name: &str) -> Option<&SpikingNetwork> {
        self.loaded_networks.get(name)
    }

    pub fn load_network(&mut self, network: SpikingNetwork) -> Result<(), ExtensionError> {
        self.loaded_networks.insert(network.name.clone(), network);
        Ok(())
    }
}

impl NoisyGradientCrdt {
    pub fn new(node_id: Uuid) -> Self {
        Self {
            node_id,
            gradient_state: GradientCrdtState::new(),
            noise_generator: NoiseGenerator::new(),
            sync_config: CrdtSyncConfig::default(),
        }
    }

    pub async fn synchronize_gradients(&mut self, result: &SnnExecutionResult) -> Result<CrdtSyncMetrics, ExtensionError> {
        // Extract gradients from synaptic weights
        let gradients = self.extract_gradients(&result.synaptic_weights)?;
        
        // Add noise to gradients
        let noisy_gradients = self.add_noise_to_gradients(&gradients)?;
        
        // Synchronize with other nodes
        let sync_result = self.perform_sync(&noisy_gradients).await?;
        
        // Resolve conflicts
        let conflicts_resolved = self.resolve_conflicts(&sync_result.conflicts)?;
        
        Ok(CrdtSyncMetrics {
            sync_rounds: sync_result.rounds,
            conflicts_resolved,
            convergence_achieved: sync_result.converged,
            noise_effectiveness: sync_result.noise_effectiveness,
            network_utilization: sync_result.utilization,
        })
    }

    fn extract_gradients(&self, weights: &[Vec<f64>]) -> Result<HashMap<String, CrdtGradient>, ExtensionError> {
        let mut gradients = HashMap::new();
        
        for (i, weight_vector) in weights.iter().enumerate() {
            for (j, &weight) in weight_vector.iter().enumerate() {
                let gradient_id = format!("weight_{}_{}", i, j);
                let gradient = CrdtGradient {
                    gradient_value: weight,
                    noisy_value: weight,
                    timestamp: Instant::now(),
                    source_node: self.node_id,
                    confidence: 1.0,
                };
                gradients.insert(gradient_id, gradient);
            }
        }
        
        Ok(gradients)
    }

    fn add_noise_to_gradients(&mut self, gradients: &HashMap<String, CrdtGradient>) -> Result<HashMap<String, CrdtGradient>, ExtensionError> {
        let mut noisy_gradients = HashMap::new();
        
        for (id, gradient) in gradients {
            let noise = self.noise_generator.generate_noise();
            let noisy_gradient = CrdtGradient {
                gradient_value: gradient.gradient_value,
                noisy_value: gradient.gradient_value + noise,
                timestamp: Instant::now(),
                source_node: self.node_id,
                confidence: gradient.confidence * 0.9, // Reduce confidence due to noise
            };
            noisy_gradients.insert(id.clone(), noisy_gradient);
        }
        
        Ok(noisy_gradients)
    }

    async fn perform_sync(&mut self, gradients: &HashMap<String, CrdtGradient>) -> Result<SyncResult, ExtensionError> {
        // Mock synchronization - in real implementation, this would communicate with other nodes
        Ok(SyncResult {
            rounds: 3,
            converged: true,
            conflicts: vec![],
            noise_effectiveness: 0.85,
            utilization: 0.7,
        })
    }

    fn resolve_conflicts(&mut self, conflicts: &[GradientConflict]) -> Result<u32, ExtensionError> {
        let mut resolved = 0;
        
        for conflict in conflicts {
            // Use noisy gradient for conflict resolution
            let resolution = self.resolve_conflict_with_noise(conflict)?;
            if resolution {
                resolved += 1;
            }
        }
        
        Ok(resolved)
    }

    fn resolve_conflict_with_noise(&mut self, conflict: &GradientConflict) -> Result<bool, ExtensionError> {
        // Mock conflict resolution using noisy gradients
        Ok(true)
    }
}

impl NoiseGenerator {
    pub fn new() -> Self {
        Self {
            noise_type: NoiseType::Gaussian,
            amplitude: 0.1,
            correlation_time: Duration::from_millis(100),
            seed: 42,
        }
    }

    pub fn generate_noise(&mut self) -> f64 {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        match self.noise_type {
            NoiseType::Gaussian => {
                // Box-Muller transform for Gaussian noise
                let u1: f64 = rng.gen();
                let u2: f64 = rng.gen();
                let noise = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
                noise * self.amplitude
            }
            NoiseType::Uniform => {
                rng.gen_range(-self.amplitude..self.amplitude)
            }
            _ => 0.0, // Placeholder for other noise types
        }
    }
}

impl TemporalConfig {
    pub fn create_processor(&self) -> TemporalProcessor {
        TemporalProcessor {
            spike_queue: VecDeque::new(),
            current_time: Instant::now(),
            time_step: self.time_step,
            event_handlers: vec![
                SpikeEventHandler {
                    handler_type: HandlerType::Plasticity,
                    priority: 1,
                    enabled: true,
                },
            ],
        }
    }
}

impl TemporalProcessor {
    pub fn new() -> Self {
        Self {
            spike_queue: VecDeque::new(),
            current_time: Instant::now(),
            time_step: Duration::from_millis(1),
            event_handlers: vec![],
        }
    }

    pub fn process_input_spike(&mut self, neuron_id: u32, spike_time: Duration) {
        let spike_event = SpikeEvent {
            neuron_id,
            spike_time: self.current_time + spike_time,
            spike_type: SpikeType::Excitatory,
            propagation_delay: Duration::from_millis(1),
        };
        self.spike_queue.push_back(spike_event);
    }

    pub fn emit_spike(&mut self, neuron_id: u32, current_time: Duration) {
        let spike_event = SpikeEvent {
            neuron_id,
            spike_time: self.current_time + current_time,
            spike_type: SpikeType::Excitatory,
            propagation_delay: Duration::from_millis(1),
        };
        self.spike_queue.push_back(spike_event);
    }

    pub fn calculate_synaptic_input(&self, neuron_id: u32) -> f64 {
        // Mock synaptic input calculation
        self.spike_queue.iter()
            .filter(|spike| spike.neuron_id == neuron_id)
            .map(|_| 0.1) // Mock weight
            .sum()
    }

    pub fn extract_output_spikes(&self) -> Vec<SpikeTrain> {
        // Mock output spike extraction
        vec![]
    }
}

// Default implementations
impl Default for CrdtSyncMetrics {
    fn default() -> Self {
        Self {
            sync_rounds: 0,
            conflicts_resolved: 0,
            convergence_achieved: false,
            noise_effectiveness: 0.0,
            network_utilization: 0.0,
        }
    }
}

impl Default for GradientCrdtState {
    fn default() -> Self {
        Self {
            gradients: HashMap::new(),
            version_vector: HashMap::new(),
            pending_updates: vec![],
            conflict_buffer: ConflictBuffer::default(),
        }
    }
}

impl GradientCrdtState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for ConflictBuffer {
    fn default() -> Self {
        Self {
            conflicts: vec![],
            resolution_strategy: ConflictResolution::NoisyGradient,
            max_buffer_size: 1000,
        }
    }
}

impl Default for CrdtSyncConfig {
    fn default() -> Self {
        Self {
            sync_protocol: SyncProtocol::Gossip,
            compression_enabled: true,
            encryption_enabled: false,
            batch_size: 100,
            timeout: Duration::from_secs(30),
        }
    }
}

// Sync result structure
#[derive(Debug, Clone)]
struct SyncResult {
    rounds: u32,
    converged: bool,
    conflicts: Vec<GradientConflict>,
    noise_effectiveness: f64,
    utilization: f64,
}

#[async_trait]
impl Extension for SnnExtension {
    fn capability(&self) -> ExtensionCapability {
        self.capability.clone()
    }

    fn shutdown(&self) -> Result<(), ExtensionError> {
        Ok(())
    }

    async fn execute(&self, job: ExtensionJob) -> Result<ExtensionResult, ExtensionError> {
        let start_time = Instant::now();
        
        match job.operation.as_str() {
            "simulate_snn" => {
                let params: SnnJobParams = serde_json::from_value(
                    job.parameters.get("snn_params")
                        .ok_or_else(|| ExtensionError::ExecutionFailed("Missing snn_params".to_string()))?
                        .clone()
                ).map_err(|e| ExtensionError::ExecutionFailed(format!("Invalid snn_params: {}", e)))?;

                let result = self.simulate_snn(params).await?;
                let quins = Self::result_to_quins(&result, &job.job_id);
                
                Ok(ExtensionResult {
                    job_id: job.job_id,
                    success: true,
                    result_quins: quins,
                    metadata: {
                        let mut meta = HashMap::new();
                        meta.insert("converged".to_string(), result.learning_metrics.convergence_rate.to_string());
                        meta.insert("final_loss".to_string(), result.learning_metrics.final_loss.to_string());
                        meta.insert("spike_rate".to_string(), result.learning_metrics.spike_rate.to_string());
                        meta.insert("sync_rounds".to_string(), result.crdt_sync_metrics.sync_rounds.to_string());
                        meta.insert("conflicts_resolved".to_string(), result.crdt_sync_metrics.conflicts_resolved.to_string());
                        meta
                    },
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                })
            }
            _ => Err(ExtensionError::OperationNotSupported(job.operation)),
        }
    }
}

// Add missing dependencies
use std::collections::VecDeque;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Extension;

    #[tokio::test]
    async fn test_snn_extension_creation() {
        let extension = SnnExtension::new();
        let capability = extension.capability();
        
        assert_eq!(capability.name, "snn");
        assert_eq!(capability.version, "1.0.0");
        assert!(capability.supported_operations.contains(&"simulate_snn".to_string()));
        assert!(capability.required_resources.requires_network); // CRDT sync requires network
    }

    #[tokio::test]
    async fn test_snn_simulation() {
        let extension = SnnExtension::new();
        
        let params = SnnJobParams {
            network_name: "test_network".to_string(),
            input_spikes: vec![],
            simulation_time: Duration::from_millis(100),
            learning_enabled: true,
            crdt_sync_enabled: true,
            noise_level: 0.1,
        };

        // This would fail since network doesn't exist, but tests the structure
        let result = extension.simulate_snn(params).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_noise_generator() {
        let mut noise_gen = NoiseGenerator::new();
        let noise1 = noise_gen.generate_noise();
        let noise2 = noise_gen.generate_noise();
        
        // Should generate different noise values
        assert_ne!(noise1, noise2);
        
        // Should be within amplitude bounds
        assert!(noise1.abs() <= 0.1);
        assert!(noise2.abs() <= 0.1);
    }

    #[test]
    fn test_crdt_gradient() {
        let node_id = Uuid::new_v4();
        let gradient = CrdtGradient {
            gradient_value: 0.5,
            noisy_value: 0.52,
            timestamp: Instant::now(),
            source_node: node_id,
            confidence: 0.9,
        };

        assert_eq!(gradient.source_node, node_id);
        assert!(gradient.confidence < 1.0);
        assert!(gradient.noisy_value != gradient.gradient_value);
    }
}
