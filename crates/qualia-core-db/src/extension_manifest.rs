use serde::{Deserialize, Serialize};

/// The transport mechanism used to communicate with the extension daemon.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TransportProtocol {
    /// Localhost HTTP/REST or WebSocket over a specific port
    LocalHttp { port: u16 },
    /// Local named pipe (Windows) or Unix domain socket (macOS/Linux)
    NamedPipe { pipe_name: String },
    /// Standard Input / Standard Output (for simple one-shot binaries)
    Stdio,
}

/// The sandbox level required by the extension.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SandboxLevel {
    /// Strict sandbox: no network, no filesystem (except via provided zero-copy buffers)
    Strict,
    /// Partial sandbox: allowed specific network domains or directories
    Partial {
        allowed_domains: Vec<String>,
        allowed_dirs: Vec<String>,
    },
    /// Trusted: full host access (requires explicit user Guardian approval to install)
    Trusted,
}

/// Declares an individual capability exposed by the extension.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ExtensionCapability {
    /// The semantic interface this capability fulfills (e.g., "q42:VideoTranscode", "q42:ObjectDetection")
    pub interface: String,
    /// MIME types this capability can accept as input
    pub supported_mimetypes: Vec<String>,
    /// Expected output semantic types or MIME types
    pub outputs: Vec<String>,
    /// Description of what the capability does
    pub description: String,
}

/// Defines the security profile and resource requirements for the extension.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ExtensionSecurity {
    pub sandbox_level: SandboxLevel,
    /// Whether the extension requires GPU access (e.g., for OpenCV, ONNX, CUDA)
    pub requires_gpu: bool,
    /// Whether the extension needs to bind to local ports
    pub requires_network_bind: bool,
}

/// The Capability Manifest schema for a Qualia-DB Extension.
/// This defines an isolated process (like FFmpeg, OpenCV) that plugs into the local engine.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ExtensionManifest {
    pub extension_id: String,
    pub version: String,
    pub display_name: String,
    pub description: String,
    
    pub transport: TransportProtocol,
    pub capabilities: Vec<ExtensionCapability>,
    pub security: ExtensionSecurity,
}

impl ExtensionManifest {
    /// Parses an ExtensionManifest from a JSON file.
    pub fn from_json(json_bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(json_bytes)
    }
}
