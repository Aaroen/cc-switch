import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import tailwindcss from '@tailwindcss/vite'
import { VitePWA } from 'vite-plugin-pwa'

export default defineConfig({
  base: '/admin/',
  plugins: [
    vue(),
    tailwindcss(),
    VitePWA({
      // workaround: workbox terser bundling hangs in production mode on current toolchain
      // development
      mode: 'production',
      base: '/admin/',
      registerType: 'autoUpdate',
      includeAssets: ['icons/pwa.svg'],
      manifest: {
        name: '透明代理监控面板',
        short_name: '代理监控面板',
        start_url: '/admin/',
        scope: '/admin/',
        display: 'standalone',
        background_color: '#0f172a',
        theme_color: '#0ea5e9',
        icons: [
          {
            src: '/admin/icons/pwa.svg',
            sizes: 'any',
            type: 'image/svg+xml',
            purpose: 'any maskable',
          },
        ],
      },
    }),
  ],
  resolve: {
    alias: {
      '@': '/src',
    },
  },
  // server: {
  //   proxy: {
  //     '/api': {
  //       target: 'http://localhost:8087',
  //       changeOrigin: true,
  //     },
  //   },
  // },
  build: {
    outDir: '../static',
    emptyOutDir: true,
  },
})
