# Flutter Prototype Wiring Reminders
This is a reminder to wire the Flutter application natively to the QualiaDB engine.

## Remaining Stub Tasks
1. **Dart UI Layer:** Replace hardcoded UI values in `chat_screen.dart`, `settings_screen.dart`, etc., with reactive data streams from the native layer.
2. **Rust API Bridge (`qualia-flutter/rust/src/api/qualia_api.rs`):** Un-stub the Rust/Flutter bridge to call actual `qualia-desktop::commands` instead of returning hardcoded responses.
