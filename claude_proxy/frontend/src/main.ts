import './style.css'
import { createApp } from 'vue'
import { createPinia } from 'pinia'
import { registerSW } from 'virtual:pwa-register'
import router from './router'
import App from './App.vue'

// 创建 Vue 应用实例
const app = createApp(App)

// 创建 Pinia 实例
const pinia = createPinia()

// 使用插件
app.use(pinia)
app.use(router)

// 挂载应用
app.mount('#app')

// 仅在生产环境注册 Service Worker
if (import.meta.env.PROD) {
  registerSW({
    immediate: true,
  })
}
