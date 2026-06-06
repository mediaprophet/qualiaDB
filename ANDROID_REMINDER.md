# Android Prototype Wiring Reminders
This is a reminder to wire the Android application natively to the QualiaDB engine and eliminate mocked prototype data.

## Remaining Mock Tasks
1. **UI & Ingestion Layers:** Replace mocked CBOR-LD encoders, mocked LLM extraction outputs (`EdgeExtractor.kt`), and mocked credential proofs (`CredentialManager.kt`) with actual calls to the JNI layer.
2. **Device Sensors:** Remove the mock location polling in `SpatialTrackingService.kt` and use actual GPS services. Remove the mock temporal filtering in the photoplethysmography C++ layer and implement real DSP/Hann window logic.
3. **Networking:** Remove hardcoded node IPs and mock Desktop responses in `FederatedNodeManager.kt` and rely on actual local P2P discovery.
