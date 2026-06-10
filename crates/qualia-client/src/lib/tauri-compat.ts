/**
 * Tauri Compatibility Layer
 * Provides unified API interface for both Tauri desktop and web browser environments
 */

// Environment detection
export const isTauri = () => {
  return typeof window !== 'undefined' && 
         '__TAURI__' in window &&
         window.__TAURI__ !== undefined;
};

// Mock Tauri APIs for web mode
const mockTauriAPIs = {
  invoke: async (cmd: string, args?: any) => {
    console.log(`[Web Mock] Tauri invoke: ${cmd}`, args);
    
    // Mock responses for common commands
    switch(cmd) {
      case 'get_config':
        return {
          storage_path: '/mock/qualia/data',
          storage_quota_gb: 50,
          base_connectivity_cost_ilp: 5000
        };
      case 'is_first_run':
        return false;
      case 'get_hardware_status':
        return {
          cpu_usage: 45.2,
          memory_used_gb: 8.3,
          memory_total_gb: 16.0,
          disk_usage_gb: 120.5,
          disk_total_gb: 500.0
        };
      case 'daemon_status':
        return 'running';
      case 'get_wallet_status':
        return {
          is_configured: false,
          network_count: 0,
          balance_usd: 0
        };
      default:
        console.warn(`[Web Mock] Unhandled command: ${cmd}`);
        return null;
    }
  },
  
  listen: async (event: string, handler: any) => {
    console.log(`[Web Mock] Tauri listen: ${event}`);
    // Return mock unlisten function
    return () => console.log(`[Web Mock] Unlisten: ${event}`);
  },
  
  event: {
    emit: async (event: string, payload?: any) => {
      console.log(`[Web Mock] Tauri emit: ${event}`, payload);
    }
  }
};

// Export unified API
export const tauri = isTauri() 
  ? (await import('@tauri-apps/api/tauri')).default
  : mockTauriAPIs;

export const invoke = isTauri()
  ? (await import('@tauri-apps/api/tauri')).invoke
  : mockTauriAPIs.invoke;

export const listen = isTauri()
  ? (await import('@tauri-apps/api/event')).listen
  : mockTauriAPIs.listen;

// Export event module
export const event = isTauri()
  ? await import('@tauri-apps/api/event')
  : mockTauriAPIs.event;

// Window APIs (desktop only)
export const window = isTauri()
  ? await import('@tauri-apps/api/window')
  : {
      getCurrent: () => ({
        minimize: () => console.log('[Web Mock] Window minimize'),
        maximize: () => console.log('[Web Mock] Window maximize'),
        close: () => console.log('[Web Mock] Window close'),
        hide: () => console.log('[Web Mock] Window hide'),
        show: () => console.log('[Web Mock] Window show'),
        setFocus: () => console.log('[Web Mock] Window setFocus'),
      }),
      getAll: () => [],
      appWindow: {
        minimize: () => console.log('[Web Mock] AppWindow minimize'),
        maximize: () => console.log('[Web Mock] AppWindow maximize'),
        close: () => console.log('[Web Mock] AppWindow close'),
        hide: () => console.log('[Web Mock] AppWindow hide'),
        show: () => console.log('[Web Mock] AppWindow show'),
      }
    };

// Shell APIs (desktop only)
export const shell = isTauri()
  ? await import('@tauri-apps/api/shell')
  : {
      open: (url: string) => {
        console.log(`[Web Mock] Shell open: ${url}`);
        window.open(url, '_blank');
      }
    };

// Dialog APIs (desktop only)
export const dialog = isTauri()
  ? await import('@tauri-apps/api/dialog')
  : {
      message: (message: string, title?: string) => {
        console.log(`[Web Mock] Dialog message: ${title} - ${message}`);
        alert(`${title ? title + ': ' : ''}${message}`);
      },
      confirm: async (message: string, title?: string) => {
        console.log(`[Web Mock] Dialog confirm: ${title} - ${message}`);
        return window.confirm(`${title ? title + ': ' : ''}${message}`);
      },
      save: async () => {
        console.log('[Web Mock] Dialog save');
        return null;
      },
      open: async () => {
        console.log('[Web Mock] Dialog open');
        return null;
      }
    };

// FS APIs (desktop only)
export const fs = isTauri()
  ? await import('@tauri-apps/api/fs')
  : {
      readTextFile: async (path: string) => {
        console.log(`[Web Mock] FS readTextFile: ${path}`);
        return '';
      },
      writeTextFile: async (path: string, contents: string) => {
        console.log(`[Web Mock] FS writeTextFile: ${path}`);
      },
      exists: async (path: string) => {
        console.log(`[Web Mock] FS exists: ${path}`);
        return false;
      },
      createDir: async (path: string) => {
        console.log(`[Web Mock] FS createDir: ${path}`);
      }
    };

// Path APIs (desktop only)
export const path = isTauri()
  ? await import('@tauri-apps/api/path')
  : {
      join: (...parts: string[]) => parts.join('/'),
      basename: (path: string) => path.split('/').pop() || path,
      dirname: (path: string) => path.split('/').slice(0, -1).join('/'),
      resolve: (...parts: string[]) => parts.join('/'),
      appDir: async () => '/mock/app/dir',
      appDataDir: async () => '/mock/app/data',
      downloadDir: async () => '/mock/downloads',
      documentDir: async () => '/mock/documents',
    };

// OS APIs (desktop only)
export const os = isTauri()
  ? await import('@tauri-apps/api/os')
  : {
      platform: async () => 'web',
      version: async () => '1.0.0',
      type: async () => 'Web Browser',
      arch: async () => 'unknown',
    };

// HTTP APIs (use fetch in web mode)
export const http = isTauri()
  ? await import('@tauri-apps/api/http')
  : {
      fetch: async (url: string, options?: any) => {
        console.log(`[Web Mock] HTTP fetch: ${url}`, options);
        const response = await fetch(url, options);
        return {
          data: await response.json(),
          status: response.status,
          headers: Object.fromEntries(response.headers.entries()),
        };
      }
    };

export default {
  isTauri,
  invoke,
  listen,
  event,
  window,
  shell,
  dialog,
  fs,
  path,
  os,
  http,
};