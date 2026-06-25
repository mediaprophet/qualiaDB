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

// ── Real LIF/STDP dynamics constants + helpers ──────────────────────────────────

/// STDP causal potentiation increment (pre-before-post).
const STDP_A_PLUS: f64 = 0.01;
/// STDP acausal depression decrement (post-without-pre).
const STDP_A_MINUS: f64 = 0.005;
/// STDP weight clamp ceiling.
const STDP_W_MAX: f64 = 1.0;

/// LIF membrane leak factor per step = `exp(−dt/τ_m)`, with a 20 ms membrane time
/// constant. (Replaces the old fixed `*= 0.99`.)
fn leak_factor(dt: Duration) -> f64 {
    const TAU_M_S: f64 = 0.020;
    (-dt.as_secs_f64() / TAU_M_S).exp()
}

/// Group `(neuron_id, spike_time)` firing events into per-neuron spike trains (sorted).
fn group_into_spike_trains(events: &[(u32, Duration)]) -> Vec<SpikeTrain> {
    let mut by_neuron: HashMap<u32, Vec<Duration>> = HashMap::new();
    for &(nid, t) in events {
        by_neuron.entry(nid).or_default().push(t);
    }
    by_neuron
        .into_iter()
        .map(|(neuron_id, mut spike_times)| {
            spike_times.sort();
            SpikeTrain {
                neuron_id,
                spike_amplitudes: vec![1.0; spike_times.len()],
                spike_times,
            }
        })
        .collect()
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
        let dt = network_sim.temporal_config.time_step;
        let mut current_time = Duration::ZERO;
        let mut membrane_potentials = Vec::new();
        let mut synaptic_weights = Vec::new();

        // Real event-driven spike state carried across steps.
        let mut prev_fired: std::collections::HashSet<u32> = std::collections::HashSet::new();
        let mut last_spike_sim: HashMap<u32, Duration> = HashMap::new();
        let mut output_events: Vec<(u32, Duration)> = Vec::new();

        // Synaptic polarity from each pre-neuron's type (excitatory +1 / inhibitory −1 / modulatory +½).
        let neuron_sign: HashMap<u32, f64> = network_sim
            .neurons
            .iter()
            .map(|n| {
                let s = match n.neuron_type {
                    NeuronType::Excitatory => 1.0,
                    NeuronType::Inhibitory => -1.0,
                    NeuronType::Modulatory => 0.5,
                };
                (n.id, s)
            })
            .collect();

        while current_time < params.simulation_time {
            // External input spikes scheduled within this step act as pre-synaptic drivers.
            let mut external_fired: std::collections::HashSet<u32> = std::collections::HashSet::new();
            for spike_train in &params.input_spikes {
                if spike_train
                    .spike_times
                    .iter()
                    .any(|&t| t <= current_time && current_time.saturating_sub(t) < dt)
                {
                    external_fired.insert(spike_train.neuron_id);
                }
            }

            let fired = self.step_neurons(
                &mut network_sim,
                current_time,
                dt,
                &prev_fired,
                &external_fired,
                &neuron_sign,
                &mut last_spike_sim,
            );
            for &nid in &fired {
                output_events.push((nid, current_time));
            }

            membrane_potentials
                .push(network_sim.neurons.iter().map(|n| n.membrane_potential).collect());
            synaptic_weights.push(network_sim.synapses.iter().map(|s| s.weight).collect());

            // Next step's pre-synaptic drivers = the neurons (+ external inputs) that fired this step.
            prev_fired = fired.into_iter().chain(external_fired).collect();
            current_time += dt;
        }

        let output_spikes = group_into_spike_trains(&output_events);
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

    /// One real LIF integrate-and-fire step: synapse-weighted input from the pre-neurons
    /// that fired last step, exponential membrane leak, threshold spike + reset, a
    /// **simulation-time** refractory period (the old code compared wall-clock `Instant`s
    /// against sim-time — a bug), and STDP weight updates. Returns the ids that fired.
    fn step_neurons(
        &self,
        network: &mut SpikingNetwork,
        current_time: Duration,
        dt: Duration,
        prev_fired: &std::collections::HashSet<u32>,
        external_fired: &std::collections::HashSet<u32>,
        neuron_sign: &HashMap<u32, f64>,
        last_spike_sim: &mut HashMap<u32, Duration>,
    ) -> Vec<u32> {
        // 1. Real synaptic current: Σ over incoming synapses whose pre-neuron fired, signed
        //    by the pre-neuron's excitatory/inhibitory type and scaled by the synapse weight.
        let mut input: HashMap<u32, f64> = HashMap::new();
        for syn in &network.synapses {
            let pre_fired = prev_fired.contains(&syn.pre_neuron_id)
                || external_fired.contains(&syn.pre_neuron_id);
            if pre_fired {
                let sign = neuron_sign.get(&syn.pre_neuron_id).copied().unwrap_or(1.0);
                *input.entry(syn.post_neuron_id).or_insert(0.0) += sign * syn.weight;
            }
        }

        // 2. LIF integrate + threshold, honoring a sim-time refractory period.
        let mut fired = Vec::new();
        for neuron in &mut network.neurons {
            if let Some(&last) = last_spike_sim.get(&neuron.id) {
                if current_time.saturating_sub(last) < neuron.refractory_period {
                    neuron.membrane_potential = 0.0;
                    continue;
                }
            }
            let syn_in = input.get(&neuron.id).copied().unwrap_or(0.0);
            let noise = self.generate_noise(neuron.temporal_state.noise_amplitude);
            neuron.membrane_potential = neuron.membrane_potential * leak_factor(dt) + syn_in + noise;
            if neuron.membrane_potential >= neuron.threshold {
                fired.push(neuron.id);
                neuron.membrane_potential = 0.0;
                neuron.last_spike_time = Some(Instant::now());
                last_spike_sim.insert(neuron.id, current_time);
            }
        }

        // 3. Real STDP on plastic synapses: pre-before-post potentiates, post-without-pre depresses.
        let fired_set: std::collections::HashSet<u32> = fired.iter().copied().collect();
        for syn in &mut network.synapses {
            if !matches!(
                syn.plasticity_type,
                PlasticityType::STDP | PlasticityType::RSTDP | PlasticityType::CRDT
            ) {
                continue;
            }
            let pre_fired = prev_fired.contains(&syn.pre_neuron_id)
                || external_fired.contains(&syn.pre_neuron_id);
            let post_fired = fired_set.contains(&syn.post_neuron_id);
            if pre_fired && post_fired {
                syn.weight = (syn.weight + STDP_A_PLUS).clamp(0.0, STDP_W_MAX);
            } else if post_fired && !pre_fired {
                syn.weight = (syn.weight - STDP_A_MINUS).clamp(0.0, STDP_W_MAX);
            }
        }

        fired
    }

    fn generate_noise(&self, amplitude: f64) -> f64 {
        // Guard against a zero/negative amplitude — `random_range(-0.0..0.0)` is an empty
        // range and panics. Zero amplitude means deterministic (no noise).
        if amplitude <= 0.0 {
            return 0.0;
        }
        use rand::RngExt;
        let mut rng = rand::rng();
        rng.random_range(-amplitude..amplitude)
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
        // Real CRDT merge of the (noisy) local gradients into the persistent gradient
        // state. There is one in-process node, so there is no network round-trip — but the
        // MERGE semantics are real: advance this node's logical clock, merge each gradient,
        // and detect genuine value conflicts against prior state from another source node
        // (resolving by keeping the higher-confidence "noisy gradient"). Metrics are derived
        // from the actual data, not hard-coded.
        const VALUE_CONFLICT_EPS: f64 = 1e-6;
        *self
            .gradient_state
            .version_vector
            .entry(self.node_id)
            .or_insert(0) += 1;

        let mut conflicts = Vec::new();
        let mut confidence_sum = 0.0;
        for (id, incoming) in gradients {
            confidence_sum += incoming.confidence;
            match self.gradient_state.gradients.get(id).cloned() {
                Some(existing)
                    if existing.source_node != incoming.source_node
                        && (existing.gradient_value - incoming.gradient_value).abs()
                            > VALUE_CONFLICT_EPS =>
                {
                    conflicts.push(GradientConflict {
                        conflicting_gradients: vec![existing.clone(), incoming.clone()],
                        conflict_type: ConflictType::ValueConflict,
                        resolution_time: Instant::now(),
                    });
                    // CRDT resolution: keep the higher-confidence value.
                    if incoming.confidence >= existing.confidence {
                        self.gradient_state
                            .gradients
                            .insert(id.clone(), incoming.clone());
                    }
                }
                _ => {
                    self.gradient_state
                        .gradients
                        .insert(id.clone(), incoming.clone());
                }
            }
        }

        let n = gradients.len().max(1) as f64;
        Ok(SyncResult {
            rounds: 1,
            converged: conflicts.is_empty(),
            conflicts,
            noise_effectiveness: confidence_sum / n,
            utilization: (self.gradient_state.gradients.len() as f64 / n).min(1.0),
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
        // Noisy-gradient resolution: among the conflicting candidates choose the one with
        // the highest confidence; a fresh node-noise draw breaks exact ties on the noisy
        // value so symmetric cross-node conflicts converge instead of deadlocking. Returns
        // whether the conflict had any candidate to resolve to.
        let tie_break = self.noise_generator.generate_noise();
        let winner = conflict.conflicting_gradients.iter().max_by(|a, b| {
            a.confidence
                .partial_cmp(&b.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(
                    (a.noisy_value + tie_break)
                        .partial_cmp(&b.noisy_value)
                        .unwrap_or(std::cmp::Ordering::Equal),
                )
        });
        Ok(winner.is_some())
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
        use rand::RngExt;
        let mut rng = rand::rng();
        
        match self.noise_type {
            NoiseType::Gaussian => {
                // Box-Muller transform for Gaussian noise
                let u1: f64 = rng.random();
                let u2: f64 = rng.random();
                let noise = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
                noise * self.amplitude
            }
            NoiseType::Uniform => {
                rng.random_range(-self.amplitude..self.amplitude)
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

    // (Removed the mock `calculate_synaptic_input` / `extract_output_spikes`. The real
    // weighted synaptic integration is `SnnExtension::step_neurons`; the real output
    // spike-train extraction is `group_into_spike_trains` in `execute_snn_simulation`.)
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

        // Gaussian noise is UNBOUNDED (amplitude scales the std-dev, it is not a hard
        // cap) — the old `abs() <= amplitude` assertion was wrong and flaky. Assert the
        // draws are finite and of a sane scale (well within ~50 sigma).
        assert!(noise1.is_finite() && noise2.is_finite());
        assert!(noise1.abs() < 5.0 && noise2.abs() < 5.0);
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

    #[test]
    fn leak_and_grouping_helpers() {
        // Membrane leak = exp(−1ms / 20ms) ≈ 0.951.
        let lf = leak_factor(Duration::from_millis(1));
        assert!((lf - (-0.05f64).exp()).abs() < 1e-9);
        // Grouping fired events into per-neuron spike trains (with unit amplitudes).
        let trains = group_into_spike_trains(&[
            (1, Duration::from_millis(2)),
            (1, Duration::from_millis(5)),
            (2, Duration::from_millis(3)),
        ]);
        assert_eq!(trains.len(), 2);
        let n1 = trains.iter().find(|t| t.neuron_id == 1).unwrap();
        assert_eq!(n1.spike_times.len(), 2);
        assert_eq!(n1.spike_amplitudes.len(), 2);
    }

    #[test]
    fn real_lif_fires_on_synaptic_drive_and_stdp_potentiates() {
        use std::collections::HashSet;
        let mk_neuron = |id: u32, threshold: f64| SpikingNeuron {
            id,
            neuron_type: NeuronType::Excitatory,
            membrane_potential: 0.0,
            threshold,
            refractory_period: Duration::from_millis(2),
            last_spike_time: None,
            temporal_state: TemporalState {
                adaptation_current: 0.0,
                recovery_variable: 0.0,
                synaptic_current: 0.0,
                noise_amplitude: 0.0, // deterministic
            },
        };
        // Weight 0.5 (below STDP_W_MAX) so potentiation is observable; neuron-2 threshold
        // 0.4 so the 0.5 synaptic drive crosses it.
        let synapse = Synapse {
            pre_neuron_id: 1,
            post_neuron_id: 2,
            weight: 0.5,
            delay: Duration::from_millis(1),
            plasticity_type: PlasticityType::STDP,
            crdt_weight: CrdtWeight {
                value: 0.5,
                version_vector: HashMap::new(),
                last_update: Instant::now(),
                conflict_resolution: ConflictResolution::LastWriterWins,
            },
        };
        let mut net = SpikingNetwork {
            name: "t".to_string(),
            network_type: NetworkType::LIF,
            neurons: vec![mk_neuron(1, 10.0), mk_neuron(2, 0.4)],
            synapses: vec![synapse],
            temporal_config: TemporalConfig {
                time_step: Duration::from_millis(1),
                simulation_window: Duration::from_millis(10),
                spike_encoding: SpikeEncoding::Temporal,
                temporal_resolution: 1000,
            },
            crdt_config: CrdtConfig {
                sync_interval: Duration::from_millis(100),
                noise_amplitude: 0.0,
                gradient_clipping: 1.0,
                consensus_threshold: 0.5,
                network_topology: NetworkTopology::FullyConnected,
            },
        };
        let ext = SnnExtension::new();
        let sign: HashMap<u32, f64> = [(1u32, 1.0), (2u32, 1.0)].into_iter().collect();
        let mut last: HashMap<u32, Duration> = HashMap::new();
        let w0 = net.synapses[0].weight;
        let prev: HashSet<u32> = HashSet::new();
        let external: HashSet<u32> = [1u32].into_iter().collect();

        // Neuron 1 fires externally → the 1→2 synapse delivers weight 1.0 to neuron 2
        // (threshold 0.5) → neuron 2's membrane crosses threshold and it fires.
        let fired = ext.step_neurons(
            &mut net,
            Duration::from_millis(1),
            Duration::from_millis(1),
            &prev,
            &external,
            &sign,
            &mut last,
        );
        assert!(fired.contains(&2), "neuron 2 should fire from the real synaptic drive");
        // STDP: pre (1) before post (2) → causal potentiation of the synapse.
        assert!(net.synapses[0].weight > w0, "STDP should potentiate the 1→2 synapse");
    }
}
