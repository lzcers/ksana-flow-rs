import { Routes, Route } from 'react-router-dom'
import Layout from './components/Layout'
import HomePage from './pages/HomePage'
import FeedbackPage from './pages/FeedbackPage'
import ChangelogPage from './pages/ChangelogPage'

function App() {
  return (
    <Routes>
      <Route path="/" element={<Layout />}>
        <Route index element={<HomePage />} />
        <Route path="feedback" element={<FeedbackPage />} />
        <Route path="changelog" element={<ChangelogPage />} />
      </Route>
    </Routes>
  )
}

export default App
