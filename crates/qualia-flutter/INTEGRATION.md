# Integrating Flutter with Rust Resource Catalog

This document explains how to connect the Flutter UI (`llm_hub_screen` and `ontology_hub_screen`) with the Rust Resource Catalog layer.

## Current State

- Rust API is defined in `rust/src/api.rs` using `#[frb]` attributes.
- Flutter screens are prepared to consume the generated API.
- We are using `flutter_rust_bridge` for FFI communication.

## Step-by-step Integration

### 1. Install flutter_rust_bridge_codegen (if not already installed)

```bash
cargo install flutter_rust_bridge_codegen
```

### 2. Generate Dart bindings

From inside `crates/qualia-flutter/`:

```bash
flutter_rust_bridge_codegen generate
```

This will generate `lib/src/rust/api.dart` (or similar) containing the `RustApi` class.

### 3. Add dependencies to pubspec.yaml (if not present)

```yaml
dependencies:
  flutter_rust_bridge: ^2.0.0   # or latest compatible version
```

### 4. Initialize the bridge in main.dart or early in the app

```dart
import 'src/rust/frb_generated.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  runApp(const MyApp());
}
```

### 5. Use the generated API in screens

Example in `llm_hub_screen.dart`:

```dart
final resources = await RustApi.loadLlmResources();
```

## Next Steps

- Implement real `download_llm(id)` and `import_ontology(id)` logic in Rust.
- Connect downloads to the existing QualiaDB download/persistence system.
- Add proper error handling and progress reporting from Rust to Flutter.

## Notes

- Keep the Resource Catalog data in `resources/*.yaml` as the single source of truth.
- The Rust layer should eventually read those YAML files directly.