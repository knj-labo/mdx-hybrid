import { useState } from 'react'
import { MDXProvider } from '@mdx-js/react'
import Welcome from './content/welcome.mdx'
import Features from './content/features.mdx'
import Performance from './content/performance.mdx'
import './App.css'

// Custom components for MDX
const components = {
  h1: (props) => <h1 style={{ color: '#333', borderBottom: '2px solid #0969da' }} {...props} />,
  h2: (props) => <h2 style={{ color: '#555' }} {...props} />,
  code: (props) => <code style={{ backgroundColor: '#f6f8fa', padding: '2px 4px', borderRadius: '3px' }} {...props} />,
  Button: ({ children, onClick }) => (
    <button 
      onClick={onClick}
      style={{
        padding: '8px 16px',
        backgroundColor: '#0969da',
        color: 'white',
        border: 'none',
        borderRadius: '6px',
        cursor: 'pointer'
      }}
    >
      {children}
    </button>
  ),
  Alert: ({ type = 'info', children }) => (
    <div style={{
      padding: '12px',
      marginBottom: '16px',
      borderRadius: '6px',
      backgroundColor: type === 'success' ? '#d1f5d3' : '#d0e8ff',
      border: `1px solid ${type === 'success' ? '#2da44e' : '#0969da'}`
    }}>
      {children}
    </div>
  )
}

function App() {
  const [activeTab, setActiveTab] = useState('welcome')
  
  const tabs = [
    { id: 'welcome', label: 'Welcome' },
    { id: 'features', label: 'Features' },
    { id: 'performance', label: 'Performance' }
  ]

  return (
    <MDXProvider components={components}>
      <div className="app">
        <header>
          <h1>⚡ MDX Hybrid Demo</h1>
          <p>Fast MDX compilation with Vite + React</p>
        </header>
        
        <nav className="tabs">
          {tabs.map(tab => (
            <button
              key={tab.id}
              className={`tab ${activeTab === tab.id ? 'active' : ''}`}
              onClick={() => setActiveTab(tab.id)}
            >
              {tab.label}
            </button>
          ))}
        </nav>
        
        <main className="content">
          {activeTab === 'welcome' && <Welcome />}
          {activeTab === 'features' && <Features />}
          {activeTab === 'performance' && <Performance />}
        </main>
        
        <footer>
          <p>Built with MDX Hybrid, Vite, and React</p>
        </footer>
      </div>
    </MDXProvider>
  )
}

export default App