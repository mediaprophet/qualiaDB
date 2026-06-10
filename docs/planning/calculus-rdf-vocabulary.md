# Calculus Modality RDF Vocabulary

## Overview

This document defines the RDF vocabulary and ontology mappings for the calculus modality, enabling dynamic definition of integration boundaries, target shaders, and computational parameters through the semantic graph.

## Namespace

```
@prefix calc: <http://qualiaDB.org/calculus#> .
@prefix q42:  <http://qualiaDB.org/q42#> .
@prefix sh:   <http://www.w3.org/ns/shacl#> .
@prefix xsd:  <http://www.w3.org/2001/XMLSchema#> .
```

## Core Classes

### calc:IntegrationOperation

Represents a numerical integration operation to be performed on continuous data.

```turtle
calc:IntegrationOperation a rdfs:Class ;
    rdfs:label "Integration Operation" ;
    rdfs:comment "A numerical integration operation on continuous data grids" ;
    rdfs:subClassOf sh:NodeShape .
```

### calc:ODESystem

Represents a system of coupled ordinary differential equations.

```turtle
calc:ODESystem a rdfs:Class ;
    rdfs:label "ODE System" ;
    rdfs:comment "A system of coupled ordinary differential equations" ;
    rdfs:subClassOf sh:NodeShape .
```

### calc:DataGrid

Represents a continuous data grid (e.g., radial grid from astrophysics simulation).

```turtle
calc:DataGrid a rdfs:Class ;
    rdfs:label "Data Grid" ;
    rdfs:comment "A continuous data grid for numerical computation" ;
    rdfs:subClassOf q42:Resource .
```

### calc:ComputeTarget

Represents the target compute backend (CPU, GPU DirectStorage, WebGPU).

```turtle
calc:ComputeTarget a rdfs:Class ;
    rdfs:label "Compute Target" ;
    rdfs:comment "Target compute backend for the operation" .
```

## Properties

### calc:integrationMethod

Specifies the integration method to use.

```turtle
calc:integrationMethod a rdf:Property ;
    rdfs:label "Integration Method" ;
    rdfs:domain calc:IntegrationOperation ;
    rdfs:range calc:IntegrationMethod .

calc:IntegrationMethod a rdfs:Class ;
    rdfs:label "Integration Method Enum" ;
    rdfs:comment "Enumeration of available integration methods" .

calc:SimpsonsRule a calc:IntegrationMethod ;
    rdfs:label "Simpson's Rule" .

calc:TrapezoidalRule a calc:IntegrationMethod ;
    rdfs:label "Trapezoidal Rule" .

calc:AdaptiveStep a calc:IntegrationMethod ;
    rdfs:label "Adaptive Step Size" .
```

### calc:stepSize

Specifies the integration step size (h).

```turtle
calc:stepSize a rdf:Property ;
    rdfs:label "Step Size" ;
    rdfs:domain calc:IntegrationOperation ;
    rdfs:range xsd:float .
```

### calc:tolerance

Specifies the numerical tolerance for adaptive methods.

```turtle
calc:tolerance a rdf:Property ;
    rdfs:label "Tolerance" ;
    rdfs:domain calc:IntegrationOperation ;
    rdfs:range xsd:float .
```

### calc:dataSource

Specifies the data grid to operate on.

```turtle
calc:dataSource a rdf:Property ;
    rdfs:label "Data Source" ;
    rdfs:domain calc:IntegrationOperation ;
    rdfs:range calc:DataGrid .
```

### calc:computeTarget

Specifies the target compute backend.

```turtle
calc:computeTarget a rdf:Property ;
    rdfs:label "Compute Target" ;
    rdfs:domain calc:IntegrationOperation ;
    rdfs:range calc:ComputeTarget .

calc:CPU a calc:ComputeTarget ;
    rdfs:label "CPU" ;
    calc:usesSIMD true .

calc:DirectStorage a calc:ComputeTarget ;
    rdfs:label "DirectStorage (Windows)" ;
    calc:dmaBypass true .

calc:GPUDirect a calc:ComputeTarget ;
    rdfs:label "GPUDirect (Linux)" ;
    calc:dmaBypass true .

calc:WebGPU a calc:ComputeTarget ;
    rdfs:label "WebGPU (Cross-platform)" ;
    calc:dmaBypass false .
```

### calc:shaderEntryPoint

Specifies the WGSL shader entry point for GPU compute.

```turtle
calc:shaderEntryPoint a rdf:Property ;
    rdfs:label "Shader Entry Point" ;
    rdfs:domain calc:IntegrationOperation ;
    rdfs:range xsd:string .
```

### calc:workgroupSize

Specifies the WGSL workgroup size (x, y, z).

```turtle
calc:workgroupSize a rdf:Property ;
    rdfs:label "Workgroup Size" ;
    rdfs:domain calc:IntegrationOperation ;
    rdfs:range xsd:string .  # Format: "64,1,1"
```

### calc:precisionMode

Specifies the precision mode (f32 or f64).

```turtle
calc:precisionMode a rdf:Property ;
    rdfs:label "Precision Mode" ;
    rdfs:domain calc:IntegrationOperation ;
    rdfs:range calc:PrecisionMode .

calc:PrecisionMode a rdfs:Class ;
    rdfs:label "Precision Mode Enum" .

calc:F32 a calc:PrecisionMode ;
    rdfs:label "32-bit Float" .

calc:F64 a calc:PrecisionMode ;
    rdfs:label "64-bit Float" .
```

### calc:kahanCompensation

Enables Kahan summation for precision.

```turtle
calc:kahanCompensation a rdf:Property ;
    rdfs:label "Kahan Compensation" ;
    rdfs:domain calc:IntegrationOperation ;
    rdfs:range xsd:boolean .
```

## Integration Boundary Definition

### calc:IntegrationBoundary

Defines the spatial/temporal boundaries of integration.

```turtle
calc:IntegrationBoundary a rdfs:Class ;
    rdfs:label "Integration Boundary" ;
    rdfs:comment "Defines the integration domain boundaries" .

calc:lowerBound a rdf:Property ;
    rdfs:label "Lower Bound" ;
    rdfs:domain calc:IntegrationBoundary ;
    rdfs:range xsd:float .

calc:upperBound a rdf:Property ;
    rdfs:label "Upper Bound" ;
    rdfs:domain calc:IntegrationBoundary ;
    rdfs:range xsd:float .

calc:boundaryType a rdf:Property ;
    rdfs:label "Boundary Type" ;
    rdfs:domain calc:IntegrationBoundary ;
    rdfs:range calc:BoundaryType .

calc:BoundaryType a rdfs:Class ;
    rdfs:label "Boundary Type Enum" .

calc:Radial a calc:BoundaryType ;
    rdfs:label "Radial Boundary" .

calc:Cartesian a calc:BoundaryType ;
    rdfs:label "Cartesian Boundary" .

calc:Temporal a calc:BoundaryType ;
    rdfs:label "Temporal Boundary" .
```

## ODE System Definition

### calc:coupledEquations

Specifies the number of coupled equations in the system.

```turtle
calc:coupledEquations a rdf:Property ;
    rdfs:label "Coupled Equations" ;
    rdfs:domain calc:ODESystem ;
    rdfs:range xsd:integer .
```

### calc:equationType

Specifies the type of differential equation.

```turtle
calc:equationType a rdf:Property ;
    rdfs:label "Equation Type" ;
    rdfs:domain calc:ODESystem ;
    rdfs:range calc:EquationType .

calc:EquationType a rdfs:Class ;
    rdfs:label "Equation Type Enum" .

calc:Boltzmann a calc:EquationType ;
    rdfs:label "Boltzmann Equation" .

calc:NavierStokes a calc:EquationType ;
    rdfs:label "Navier-Stokes" .

calc:ReactionDiffusion a calc:EquationType ;
    rdfs:label "Reaction-Diffusion" .
```

## Example: Astrophysics Integration

```turtle
@prefix ex: <http://example.org/astrophysics#> .

ex:RadialGrid a calc:DataGrid ;
    q42:did "did:q42:z3Q...radial_grid_5000" ;
    calc:gridSize 5000 ;
    calc:gridType calc:Radial .

ex:PhaseSpaceIntegration a calc:IntegrationOperation ;
    calc:integrationMethod calc:SimpsonsRule ;
    calc:stepSize 0.001 ;
    calc:dataSource ex:RadialGrid ;
    calc:computeTarget calc:DirectStorage ;
    calc:shaderEntryPoint "simpsons_integration" ;
    calc:workgroupSize "64,1,1" ;
    calc:precisionMode calc:F64 ;
    calc:kahanCompensation true ;
    calc:integrationBoundary ex:RadialBoundary .

ex:RadialBoundary a calc:IntegrationBoundary ;
    calc:lowerBound 0.0 ;
    calc:upperBound 100.0 ;
    calc:boundaryType calc:Radial .
```

## Example: Chemical Kinetics ODE

```turtle
@prefix chem: <http://example.org/chemistry#> .

chem:CamelinaKinetics a calc:ODESystem ;
    calc:coupledEquations 12 ;
    calc:equationType calc:ReactionDiffusion ;
    calc:computeTarget calc:WebGPU ;
    calc:shaderEntryPoint "rk4_step" ;
    calc:workgroupSize "128,1,1" ;
    calc:precisionMode calc:F64 ;
    calc:kahanCompensation true ;
    calc:timeStep 0.01 ;
    calc:maxIterations 10000 .
```

## SHACL Constraint Mapping

The SHACL compiler will map these RDF properties to `SlgOpcode` sequences:

```turtle
calc:IntegrationOperation sh:targetClass calc:IntegrationOperation ;
    sh:property [
        sh:path calc:integrationMethod ;
        sh:in (calc:SimpsonsRule calc:TrapezoidalRule) ;
        sh:severity sh:Violation ;
    ] ;
    sh:property [
        sh:path calc:stepSize ;
        sh:datatype xsd:float ;
        sh:minInclusive 0.0 ;
    ] ;
    sh:property [
        sh:path calc:computeTarget ;
        sh:in (calc:CPU calc:DirectStorage calc:GPUDirect calc:WebGPU) ;
    ] .
```

## Compiler Integration

When the SHACL compiler encounters a `calc:IntegrationOperation`, it:

1. **Validates** the operation against SHACL constraints
2. **Selects** the appropriate compute target based on `calc:computeTarget`
3. **Generates** the corresponding `SlgOpcode`:
   - `OP_SIMPSONS_INTEGRATION` (0x50) for CPU/GPU
   - `OP_GPU_INTEGRATION` (0x54) for GPU-specific dispatch
4. **Packs** parameters into the Quin:
   - `subject`: Job ID (q_hash of operation IRI)
   - `predicate`: Opcode
   - `object`: Data grid byte offset
   - `context`: Step size (f32) + Kahan compensation (f32)
   - `metadata`: Running accumulator (f64)
5. **Routes** to the appropriate backend:
   - CPU → `calculus::integrate_simpsons_chunked()`
   - DirectStorage → `directml_bridge::DirectStorageManager::async_read_to_gpu()`
   - WebGPU → `calculus_gpu::WebGpuIntegrator::integrate_simpsons_gpu()`
