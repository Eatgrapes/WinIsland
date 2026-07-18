import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import '@sohumsuthar/liquid-glass/css/liquid-glass-core.css'
import '@sohumsuthar/liquid-glass/css/liquid-glass-effects.css'
import './styles.css'
import App from './App'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
