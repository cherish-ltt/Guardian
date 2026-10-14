import { createApp } from 'vue'
import App from './App.vue'
import router from './router'
import { installPermissionDirective } from './directives/permission'
import { installHoverLift, installPressFeedback, installRipple } from './composables/motion'

// 设计令牌 → 基础 → 材质 → 控件 → 外壳 → 页面
import './styles/tokens.css'
import './styles/base.css'
import './styles/materials.css'
import './styles/components.css'
import './styles/instruments.css'
import './styles/layout.css'
import './styles/pages.css'

// 外观设置在挂载前写入 data-theme / data-density，避免首屏闪色
import './composables/appearance'

const app = createApp(App)
installPermissionDirective(app)
app.use(router).mount('#app')

installPressFeedback()
installHoverLift()
installRipple()
