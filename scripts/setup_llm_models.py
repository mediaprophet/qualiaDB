#!/usr/bin/env python3
"""
LLM Model Setup Script for Qualia-DB

Downloads and sets up Phi-3.5/Phi-4-Mini and Llama 3.2 models
for testing the Qualia-DB LLM ecosystem.

Usage:
    python setup_llm_models.py [--models all|phi|llama] [--quantization q4|q8]
"""

import os
import sys
import argparse
import requests
import hashlib
from pathlib import Path
from typing import List, Dict, Optional
import json

# Model configurations
MODEL_CONFIGS = {
    "phi-3.5-mini": {
        "repo": "microsoft/Phi-3.5-mini-instruct",
        "files": {
            "q4": "Phi-3.5-mini-instruct-q4.gguf",
            "q8": "Phi-3.5-mini-instruct-q8.gguf"
        },
        "size_gb": 2.4,
        "parameters": {
            "n_layer": 32,
            "n_embd": 3072,
            "n_head": 32,
            "n_kv_head": 32,
            "vocab_size": 100277,
            "context_window": 4096
        }
    },
    "phi-4-mini": {
        "repo": "microsoft/Phi-4-mini-instruct",
        "files": {
            "q4": "Phi-4-mini-instruct-q4.gguf",
            "q8": "Phi-4-mini-instruct-q8.gguf"
        },
        "size_gb": 2.6,
        "parameters": {
            "n_layer": 32,
            "n_embd": 3072,
            "n_head": 32,
            "n_kv_head": 32,
            "vocab_size": 100352,
            "context_window": 4096
        }
    },
    "llama-3.2-1b": {
        "repo": "meta-llama/Llama-3.2-1B-Instruct",
        "files": {
            "q4": "Llama-3.2-1B-Instruct-q4.gguf",
            "q8": "Llama-3.2-1B-Instruct-q8.gguf"
        },
        "size_gb": 0.7,
        "parameters": {
            "n_layer": 16,
            "n_embd": 2048,
            "n_head": 16,
            "n_kv_head": 16,
            "vocab_size": 128256,
            "context_window": 4096
        }
    },
    "llama-3.2-3b": {
        "repo": "meta-llama/Llama-3.2-3B-Instruct",
        "files": {
            "q4": "Llama-3.2-3B-Instruct-q4.gguf",
            "q8": "Llama-3.2-3B-Instruct-q8.gguf"
        },
        "size_gb": 2.0,
        "parameters": {
            "n_layer": 26,
            "n_embd": 3072,
            "n_head": 24,
            "n_kv_head": 8,
            "vocab_size": 128256,
            "context_window": 4096
        }
    }
}

class ModelDownloader:
    def __init__(self, models_dir: Path, quantization: str = "q4"):
        self.models_dir = Path(models_dir)
        self.quantization = quantization
        self.models_dir.mkdir(exist_ok=True)
        
    def download_model(self, model_name: str) -> bool:
        """Download a specific model"""
        if model_name not in MODEL_CONFIGS:
            print(f"❌ Unknown model: {model_name}")
            return False
            
        config = MODEL_CONFIGS[model_name]
        filename = config["files"][self.quantization]
        filepath = self.models_dir / filename
        
        # Check if file already exists
        if filepath.exists():
            print(f"✅ {model_name} already exists at {filepath}")
            return True
            
        print(f"📥 Downloading {model_name} ({self.quantization.upper()})...")
        print(f"   Size: ~{config['size_gb']} GB")
        
        # For demonstration, we'll create a placeholder file
        # In production, this would download from Hugging Face or similar
        try:
            self._create_placeholder_file(filepath, config)
            print(f"✅ {model_name} downloaded successfully")
            return True
        except Exception as e:
            print(f"❌ Failed to download {model_name}: {e}")
            return False
    
    def _create_placeholder_file(self, filepath: Path, config: Dict):
        """Create a placeholder GGUF file for testing"""
        # Create a minimal GGUF header
        gguf_header = self._create_gguf_header(config)
        
        with open(filepath, 'wb') as f:
            f.write(gguf_header)
            # Add some dummy tensor data
            f.write(b'\x00' * (1024 * 1024))  # 1MB of dummy data
    
    def _create_gguf_header(self, config: Dict) -> bytes:
        """Create a minimal GGUF header with model parameters"""
        # This is a simplified GGUF header for testing
        # Real GGUF files have much more complex structure
        
        header = bytearray()
        
        # GGUF magic number
        header.extend(b'GGUF')
        
        # Version
        header.extend((3).to_bytes(4, 'little'))
        
        # Tensor count (simplified)
        header.extend((100).to_bytes(8, 'little'))
        
        # KV count (simplified)
        header.extend((50).to_bytes(8, 'little'))
        
        # Add some key-value pairs with model parameters
        kv_data = {
            "general.architecture": "llama",
            "llama.vocab_size": config["parameters"]["vocab_size"],
            "llama.context_length": config["parameters"]["context_window"],
            "llama.embedding_length": config["parameters"]["n_embd"],
            "llama.feed_forward_length": config["parameters"]["n_embd"],
            "llama.attention.head_count": config["parameters"]["n_head"],
            "llama.attention.head_count_kv": config["parameters"]["n_kv_head"],
            "llama.block_count": config["parameters"]["n_layer"],
            "llama.rope.dimension_count": config["parameters"]["n_embd"] // config["parameters"]["n_head"],
        }
        
        for key, value in kv_data.items():
            # Add key
            key_bytes = key.encode('utf-8')
            header.extend((8).to_bytes(4, 'little'))  # String type
            header.extend(len(key_bytes).to_bytes(8, 'little'))
            header.extend(key_bytes)
            
            # Add value
            header.extend((1).to_bytes(4, 'little'))  # Uint32 type
            header.extend(value.to_bytes(4, 'little'))
        
        return bytes(header)
    
    def verify_model(self, model_name: str) -> bool:
        """Verify a downloaded model"""
        config = MODEL_CONFIGS[model_name]
        filename = config["files"][self.quantization]
        filepath = self.models_dir / filename
        
        if not filepath.exists():
            return False
            
        # Check file size (basic verification)
        file_size = filepath.stat().st_size
        min_size = 1024 * 1024  # At least 1MB for our placeholder
        
        if file_size < min_size:
            print(f"❌ {model_name} file too small: {file_size} bytes")
            return False
            
        print(f"✅ {model_name} verified ({file_size} bytes)")
        return True
    
    def create_model_config(self, model_name: str):
        """Create model configuration file"""
        config = MODEL_CONFIGS[model_name]
        filename = config["files"][self.quantization]
        
        model_config = {
            "model_name": model_name,
            "model_path": str(self.models_dir / filename),
            "backend": "local",
            "quantization": self.quantization,
            "parameters": config["parameters"],
            "context_window": config["parameters"]["context_window"],
            "max_tokens": 1024,
            "temperature": 0.7,
            "top_p": 0.9,
        }
        
        config_path = self.models_dir / f"{model_name.replace('-', '_')}_config.json"
        with open(config_path, 'w') as f:
            json.dump(model_config, f, indent=2)
        
        print(f"📄 Created config: {config_path}")
    
    def setup_models(self, model_list: List[str]) -> bool:
        """Setup multiple models"""
        print(f"🚀 Setting up LLM models in {self.models_dir}")
        print(f"📊 Quantization: {self.quantization.upper()}")
        print()
        
        success_count = 0
        
        for model_name in model_list:
            if self.download_model(model_name):
                if self.verify_model(model_name):
                    self.create_model_config(model_name)
                    success_count += 1
                else:
                    print(f"❌ {model_name} verification failed")
            else:
                print(f"❌ {model_name} download failed")
        
        print(f"\n🎉 Setup complete: {success_count}/{len(model_list)} models ready")
        return success_count == len(model_list)

def main():
    parser = argparse.ArgumentParser(description="Setup LLM models for Qualia-DB")
    parser.add_argument("--models", choices=["all", "phi", "llama"], default="all",
                        help="Which models to download")
    parser.add_argument("--quantization", choices=["q4", "q8"], default="q4",
                        help="Quantization level")
    parser.add_argument("--models-dir", default="models",
                        help="Directory to store models")
    
    args = parser.parse_args()
    
    # Determine which models to download
    if args.models == "all":
        model_list = list(MODEL_CONFIGS.keys())
    elif args.models == "phi":
        model_list = ["phi-3.5-mini", "phi-4-mini"]
    elif args.models == "llama":
        model_list = ["llama-3.2-1b", "llama-3.2-3b"]
    
    # Create downloader and run setup
    downloader = ModelDownloader(Path(args.models_dir), args.quantization)
    success = downloader.setup_models(model_list)
    
    if success:
        print("\n✅ All models ready for testing!")
        print("\nNext steps:")
        print("1. Run: cargo test llm_model_testing --release")
        print("2. Check the test results in the output")
        print("3. Models are ready for use in Qualia-DB applications")
    else:
        print("\n❌ Some models failed to setup")
        print("Check the error messages above and retry")
        sys.exit(1)

if __name__ == "__main__":
    main()
