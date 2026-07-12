use super::*;

impl PhysicsSimulationLibrary {
    /// Create new physics simulation library
    pub fn new() -> Self {
        Self {
            simulation_engine: SimulationEngine::new(),
            physics_solver: PhysicsSolver::new(),
            mesh_coordinator: MeshCoordinator::new(),
            data_manager: PhysicsDataManager::new(),
            performance_monitor: PhysicsPerformanceMonitor::new(),
        }
    }
    /// Initialize the library
    pub fn initialize(&mut self) -> Result<(), PhysicsError> {
        // Initialize simulation engine
        self.simulation_engine.initialize()?;

        // Initialize physics solver
        self.physics_solver.initialize()?;

        // Initialize mesh coordinator
        self.mesh_coordinator.initialize()?;

        // Initialize data manager
        self.data_manager.initialize()?;

        Ok(())
    }
    /// Create a new simulation
    pub fn create_simulation(
        &mut self,
        config: SimulationConfig,
    ) -> Result<Simulation, PhysicsError> {
        // Validate configuration
        self.validate_config(&config)?;

        // Create simulation
        let simulation = Simulation {
            config: config.clone(),
            current_time: 0.0,
            current_step: 0,
            fields: HashMap::new(),
            mesh: None,
            status: SimulationStatus::Created,
        };

        Ok(simulation)
    }
    /// Get performance statistics
    pub fn get_performance_stats(&self) -> PhysicsPerformanceMetrics {
        self.performance_monitor.get_metrics()
    }

    // Internal methods
    fn validate_config(&self, config: &SimulationConfig) -> Result<(), PhysicsError> {
        if config.time_step <= 0.0 {
            return Err(PhysicsError::InvalidConfiguration(
                "Time step must be positive".to_string(),
            ));
        }
        if config.total_time <= 0.0 {
            return Err(PhysicsError::InvalidConfiguration(
                "Total time must be positive".to_string(),
            ));
        }
        if config.spatial_resolution.nx == 0 {
            return Err(PhysicsError::InvalidConfiguration(
                "Spatial resolution must be positive".to_string(),
            ));
        }
        Ok(())
    }
}
