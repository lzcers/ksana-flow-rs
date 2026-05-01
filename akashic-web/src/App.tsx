import React from 'react';
import { useGameStore } from './store/gameStore';
import LobbyPage from './pages/LobbyPage';
import CreationPage from './pages/CreationPage';
import GameplayPage from './pages/GameplayPage';
import EndingPage from './pages/EndingPage';

function App() {
  const gameState = useGameStore((state) => state.gameState);

  return (
    <div className="relative h-screen h-dvh w-full overflow-hidden bg-background">
      {/* Global animated background can be placed here */}
      <div className="absolute inset-0 z-0 opacity-20 pointer-events-none">
        <div className="absolute inset-0 bg-gradient-to-br from-indigo-900/30 via-zinc-900 to-black"></div>
      </div>
      
      <main className="relative z-10 h-full w-full">
        {gameState === 'lobby' && <LobbyPage />}
        {gameState === 'creation' && <CreationPage />}
        {gameState === 'playing' && <GameplayPage />}
        {gameState === 'ending' && <EndingPage />}
      </main>
    </div>
  );
}

export default App;
