import { defineConfig } from 'vite'

// Standard Tauri v2 + Vanilla JS (Vite) dev server config.
// Frontend sources live in ./src; Vite serves them on :1420 in dev,
// and builds to ../dist for Tauri to bundle.
export default defineConfig({
  root: 'src',
  base: './',
  // Treat src/assets as Vite's public dir so dog.png / bark.mp3 are served
  // at the site root (/dog.png, /bark.mp3) and copied into dist on build.
  publicDir: 'assets',
  build: {
    outDir: '../dist',
    emptyOutDir: true,
  },
  server: {
    port: 1420,
    strictPort: true,
  },
})
