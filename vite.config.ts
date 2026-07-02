import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import { readFileSync } from 'fs'

const pkg = JSON.parse(readFileSync('package.json', 'utf-8'));

export default defineConfig({
  define: {
    __APP_VERSION__: JSON.stringify(pkg.version),
  },
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    target: 'esnext',
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    chunkSizeWarningLimit: 5000,
    rolldownOptions: {
      output: {
        codeSplitting: {
          groups: [
            {
              name: 'mermaid',
              test: /[\\/]node_modules[\\/]mermaid[\\/]/,
            },
            {
              name: 'excalidraw',
              test: /[\\/]node_modules[\\/]@excalidraw[\\/]/,
            },
            {
              name: 'blocknote',
              test: /[\\/]node_modules[\\/]@blocknote[\\/]/,
            },
            {
              name: 'katex',
              test: /[\\/]node_modules[\\/]katex[\\/]/,
            },
            {
              name: 'd3-graph',
              test: /[\\/]node_modules[\\/](react-force-graph|d3-)[\\/]/,
            },
          ],
        },
      },
    },
  },
})
