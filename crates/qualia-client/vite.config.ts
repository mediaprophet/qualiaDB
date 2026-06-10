import { defineConfig, loadEnv } from 'vite'
import react from '@vitejs/plugin-react'

// https://vite.dev/config/
export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '')
  
  return {
    plugins: [react()],
    define: {
      __TAURI_MODE__: JSON.stringify(mode),
      __WEB_MODE__: JSON.stringify(mode === 'web'),
      __DESKTOP_MODE__: JSON.stringify(mode === 'desktop'),
    },
    server: {
      port: 5173,
      strictPort: true,
      host: mode === 'desktop' ? '127.0.0.1' : '0.0.0.0',
    },
    build: {
      outDir: mode === 'desktop' ? '../qualia-desktop/dist' : 'dist',
      emptyOutDir: true,
      target: 'esnext',
      minify: 'terser',
      sourcemap: true,
    },
    resolve: {
      alias: {
        '@': '/src',
      },
    },
  }
})
