import { defineConfig, loadEnv } from 'vite'
import vue from '@vitejs/plugin-vue'
import { fileURLToPath, URL } from 'node:url'
import { version } from './package.json'

export default defineConfig(({ mode }) => {
  // 走 loadEnv 而不是 process.env：Vite 不会把 .env 文件注入 process.env，
  // 直接读 process.env 会让 .env 里的 VITE_API_PROXY_TARGET 静默失效。
  // 第三个参数传 '' 表示不按前缀过滤，读取该模式下全部变量。
  const env = loadEnv(mode, process.cwd(), '')
  const apiProxyTarget = env.VITE_API_PROXY_TARGET || 'http://localhost:8080'

  return {
    plugins: [vue()],
    // 页面展示的版本号直接取 package.json，避免两处漂移
    define: {
      APP_VERSION: JSON.stringify(`v${version}`),
    },
    resolve: {
      alias: {
        '@': fileURLToPath(new URL('./src', import.meta.url)),
      },
    },
    server: {
      port: 5173,
      open: true,
      proxy: {
        '/api': {
          target: apiProxyTarget,
          changeOrigin: true,
        },
      },
    },
  }
})
