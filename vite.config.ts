import { defineConfig } from 'vite'
import { resolve } from 'path'

export default defineConfig({
  root: 'harness',
  server: {
    port: 5180,
  },
  build: {
    outDir: '../dist',
    emptyOutDir: true,
  },
  resolve: {
    alias: {
      '@': resolve(__dirname, 'harness/src'),
    },
  },
})
