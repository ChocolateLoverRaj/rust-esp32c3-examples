import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import unwrap from 'throw-empty'

import { App } from './App'

const root = createRoot(unwrap(document.getElementById('app')))

root.render(
  <StrictMode>
    <App name='StackBlitz' />
  </StrictMode>
)
