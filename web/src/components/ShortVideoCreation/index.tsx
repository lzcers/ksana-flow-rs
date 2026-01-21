import React, { useState } from 'react';
import { ArrowLeft, Book, Users, Clapperboard } from 'lucide-react';
import { ScriptModule } from './ScriptModule';
import { CharacterModule } from './CharacterModule';
import { StoryboardModule } from './StoryboardModule';
import type { ModuleType, ShortVideoCreationProps } from './types';



export const ShortVideoCreation: React.FC<ShortVideoCreationProps> = ({ data, onBack, isNodeCompleted }) => {
  const [currentModule, setCurrentModule] = useState<ModuleType>('storyboard'); // Default to storyboard as per image

  // Use passed data or fallback to mock data if empty
  const displayData = {
    storyboard: data?.storyboard || [],
    script: data?.script || { content: '' },
    characters: data?.characters || [],
  };

  return (
    <div className="w-full h-full flex flex-col bg-white dark:bg-zinc-950 text-zinc-900 dark:text-zinc-100 overflow-hidden font-sans">
      {/* Top Toolbar */}
      <header className="h-14 border-b border-zinc-200 dark:border-zinc-800 flex items-center justify-between px-4 shrink-0 bg-white dark:bg-zinc-950">
        <div className="flex items-center gap-4">
          <button
            onClick={onBack}
            className="p-2 -ml-2 rounded-full hover:bg-zinc-100 dark:hover:bg-zinc-800 text-zinc-600 dark:text-zinc-400 transition-colors"
          >
            <ArrowLeft size={20} />
          </button>
          <h1 className="text-base font-semibold text-zinc-900 dark:text-zinc-100">
          </h1>
        </div>

        {/* Module Switcher */}
        <div className="flex items-center gap-1 bg-zinc-100 dark:bg-zinc-900 p-1 rounded-lg">
          <button
            onClick={() => setCurrentModule('script')}
            className={`px-3 py-1.5 rounded-md text-sm font-medium flex items-center gap-2 transition-colors ${currentModule === 'script'
              ? 'bg-white dark:bg-zinc-800 text-zinc-900 dark:text-zinc-100 shadow-sm'
              : 'text-zinc-500 dark:text-zinc-400 hover:text-zinc-700 dark:hover:text-zinc-300'
              }`}
          >
            <Book size={16} />
            剧本
          </button>
          <button
            onClick={() => setCurrentModule('character')}
            className={`px-3 py-1.5 rounded-md text-sm font-medium flex items-center gap-2 transition-colors ${currentModule === 'character'
              ? 'bg-white dark:bg-zinc-800 text-zinc-900 dark:text-zinc-100 shadow-sm'
              : 'text-zinc-500 dark:text-zinc-400 hover:text-zinc-700 dark:hover:text-zinc-300'
              }`}
          >
            <Users size={16} />
            角色
          </button>
          <button
            onClick={() => setCurrentModule('storyboard')}
            className={`px-3 py-1.5 rounded-md text-sm font-medium flex items-center gap-2 transition-colors ${currentModule === 'storyboard'
              ? 'bg-white dark:bg-zinc-800 text-zinc-900 dark:text-zinc-100 shadow-sm'
              : 'text-zinc-500 dark:text-zinc-400 hover:text-zinc-700 dark:hover:text-zinc-300'
              }`}
          >
            <Clapperboard size={16} />
            分镜表
          </button>
        </div>

        {/* Right Actions */}
        <div className="flex items-center gap-2">

        </div>
      </header>

      {/* Main Content */}
      <main className="flex-1 overflow-hidden relative bg-zinc-50 dark:bg-zinc-900">
        {currentModule === 'script' && (
          <ScriptModule
            content={displayData.script?.content || ''}
            readOnly={true}
            onChange={(content) => {
              // In a real app, propagate this change up
              console.log('Script changed:', content);
            }}
          />
        )}
        {currentModule === 'character' && (
          <CharacterModule characters={displayData.characters || []} isNodeCompleted={isNodeCompleted} />
        )}
        {currentModule === 'storyboard' && (
          <StoryboardModule shots={displayData.storyboard || []} isNodeCompleted={isNodeCompleted} />
        )}
      </main>
    </div>
  );
};
