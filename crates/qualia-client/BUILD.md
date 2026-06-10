# Qualia Client Build Instructions

The Qualia Client supports both web browser and desktop (Tauri) deployment modes.

## ⚠️ Important Setup Note

**You must run `npm install` before development or building.** The `node_modules` directory was excluded during migration to keep the repository lightweight, so TypeScript type definitions and other dependencies are not currently available.

```bash
npm install
```

After installing dependencies, consider restoring the TypeScript type definitions in:
- `tsconfig.app.json`: Change `"types": []` to `"types": ["vite/client"]`
- `tsconfig.node.json`: Change `"types": []` to `"types": ["node"]`

## Prerequisites

- Node.js 18+ 
- npm or yarn
- For desktop builds: Rust toolchain and Tauri CLI

## Development

### Web Mode (Default)
```bash
npm install
npm run dev
```
This runs the client in web-only mode with mocked Tauri APIs.

### Desktop Mode
```bash
npm install
npm run dev:desktop
```
This runs the client with full Tauri integration for desktop features.

## Production Builds

### Web Build
```bash
npm run build
```
Output: `dist/` directory - can be deployed to any web server.

### Desktop Build
```bash
# First build the frontend for desktop
npm run build:desktop

# Then build the desktop app from the qualia-desktop directory
cd ../qualia-desktop
cargo tauri build
```

This creates platform-specific installers in `qualia-desktop/src-tauri/target/release/bundle/`.

## Architecture

### Dual Mode Support
The client uses a compatibility layer (`src/lib/tauri-compat.ts`) that:
- Detects the runtime environment (web vs desktop)
- Provides mock implementations for Tauri APIs in web mode
- Uses real Tauri APIs in desktop mode
- Ensures seamless development and deployment across platforms

### Logging System
The client includes a comprehensive logging system (`src/lib/logger.ts`):
- Automatic log persistence (localStorage in web, file system in desktop)
- Multiple log levels (DEBUG, INFO, WARN, ERROR, CRITICAL)
- Built-in log viewer component with filtering and export
- Configurable retention policies

### System Tray Integration (Desktop Only)
The desktop app includes an enhanced system tray menu with:
- Show/Hide window controls
- Quick access to Settings
- Direct access to System Logs
- Daemon status monitoring
- Application quit control

## Project Structure

```
qualia-client/
├── src/
│   ├── components/       # Reusable UI components
│   │   └── LogViewer.tsx # Log viewing interface
│   ├── lib/              # Core libraries
│   │   ├── tauri-compat.ts  # Tauri compatibility layer
│   │   └── logger.ts        # Logging system
│   ├── pages/            # Application pages
│   └── main.tsx          # Application entry point
├── public/               # Static assets
├── package.json          # Dependencies and scripts
├── vite.config.ts        # Build configuration
└── tailwind.config.js    # Styling configuration
```

## Environment Variables

The build process uses the following environment variables:
- `__TAURI_MODE__`: 'web' or 'desktop'
- `__WEB_MODE__`: true in web mode
- `__DESKTOP_MODE__`: true in desktop mode

## Troubleshooting

### Desktop Build Issues
If you encounter issues with desktop builds:
1. Ensure Rust and Tauri CLI are properly installed
2. Check that the desktop build output directory exists
3. Verify that qualia-desktop can access the built frontend

### Web Mode Limitations
In web mode, the following features are mocked/unavailable:
- File system access
- System tray
- Native window controls
- Hardware information
- Daemon management

These features automatically use mock implementations for development purposes.

## Development Notes

- Always test in both web and desktop modes before deployment
- The compatibility layer logs mocked API calls in development
- Use `isTauri()` from `tauri-compat.ts` for feature detection
- Logging works in both modes with appropriate storage backends