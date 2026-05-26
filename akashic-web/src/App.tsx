import React from 'react';
import { useGameUIStore } from './store/gameStore';
import LobbyPage from './pages/LobbyPage';
import ArchiveListPage from './pages/ArchiveListPage';
import CreationPage from './pages/CreationPage';
import GeneratingPage from './pages/GeneratingPage';
import GameplayPage from './pages/GameplayPage';

function App() {
  const gameState = useGameUIStore((state) => state.gameState);

  return (
    <div className="relative h-dvh w-full overflow-hidden bg-background">
      <div className="pointer-events-none absolute inset-0 z-0">
        <div className="absolute -left-24 top-12 h-72 w-72 rounded-full bg-sky-500/10 blur-3xl" />
        <div className="absolute bottom-10 right-[-4rem] h-80 w-80 rounded-full bg-indigo-500/10 blur-3xl" />
        <div className="absolute inset-y-0 left-[8%] w-px bg-white/5" />
        <div className="absolute inset-y-0 right-[8%] w-px bg-white/5" />
      </div>

      <main className="akashic-scroll relative z-10 h-full w-full overflow-y-auto overflow-x-hidden">
        {gameState === 'lobby' && <LobbyPage />}
        {gameState === 'archive_list' && <ArchiveListPage />}
        {gameState === 'creation' && <CreationPage />}
        {gameState === 'generating' && <GeneratingPage />}
        {gameState === 'playing' && <GameplayPage />}
      </main>
    </div>
  );
}

export default App;
