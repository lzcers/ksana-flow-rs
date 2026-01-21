import React, { useState } from 'react';
import { ArrowLeft, Book, Users, Clapperboard, ChevronDown } from 'lucide-react';
import { ScriptModule } from './ScriptModule';
import { CharacterModule } from './CharacterModule';
import { StoryboardModule } from './StoryboardModule';
import type { ProjectData, ModuleType } from './types';

interface ShortVideoCreationProps {
  data: ProjectData;
  onBack?: () => void;
  onDataChange?: (data: ProjectData) => void;
}

// Mock data based on the OCR/Image description
const MOCK_DATA: ProjectData = {
  script: {
    content: `# 虚拟KOL解说短视频

**场号 1**
背景：热闹的朱雀桥现场，河道中漂浮着刻满甲骨文的青铜龙鳞...

**场号 2**
背景：河道中有青铜龙鳞...
`
  },
  characters: [
    {
      id: '1',
      name: '柳夜熙',
      description: '虚拟美妆达人，擅长国风',
      tags: ['虚拟人', '国风', '美妆'],
      avatar: 'https://coresg-normal.trae.ai/api/ide/v1/text_to_image?prompt=portrait%20of%20Liu%20Yexi%2C%20chinese%20virtual%20idol%2C%20traditional%20makeup&image_size=square'
    },
    {
      id: '2',
      name: '敖灵',
      description: '龙族后裔，神秘少女',
      tags: ['龙族', '神秘', '少女'],
      avatar: 'https://coresg-normal.trae.ai/api/ide/v1/text_to_image?prompt=portrait%20of%20Ao%20Ling%2C%20chinese%20dragon%20girl%2C%20mysterious&image_size=square'
    }
  ],
  storyboard: [
    {
      id: '1',
      shotNo: 1,
      image: 'https://coresg-normal.trae.ai/api/ide/v1/text_to_image?prompt=bust%20shot%20of%20a%20woman%20in%20red%20dress%20by%20the%20river%2C%20ancient%20chinese%20city%20background%2C%20lively%20scene&image_size=landscape_16_9',
      description: {
        background: '热闹的龙舟彩排现场,河道中漂浮着刻满甲骨文的青铜龙鳞',
        relation: '柳夜熙: 正在直播',
        composition: '正左画面'
      },
      lines: {
        narration: '端午前夕,柳夜熙正在直播龙舟彩排,镜头给到热闹现场,随后聚焦在河道中漂浮的刻满甲骨文的青铜龙鳞上',
        dialogue: '柳夜熙: ...'
      },
      mainCharacter: '柳夜熙',
      shotSize: '特写',
      cameraAngle: '视平',
      lensType: '单人镜头',
      duration: 1
    },
    {
      id: '2',
      shotNo: 2,
      image: 'https://coresg-normal.trae.ai/api/ide/v1/text_to_image?prompt=close%20up%20of%20woman%20looking%20at%20floating%20dragon%20scales%20in%20river&image_size=landscape_16_9',
      description: {
        background: '河道中有青铜龙鳞',
        relation: '柳夜熙: 好奇,触碰青铜龙鳞',
        composition: ''
      },
      lines: {
        narration: '柳夜熙好奇地触碰青铜龙鳞,瞬间时空扭曲,光芒闪耀',
        dialogue: '柳夜熙: ...'
      },
      mainCharacter: '柳夜熙',
      shotSize: '近景',
      cameraAngle: '视平',
      lensType: '单人镜头',
      duration: 1
    },
    {
      id: '3',
      shotNo: 3,
      image: 'https://coresg-normal.trae.ai/api/ide/v1/text_to_image?prompt=girl%20in%20white%20dress%20falling%20from%20mist%20onto%20ground&image_size=landscape_16_9',
      description: {
        background: '有一片水雾',
        relation: '从水雾中跌落,重重摔在地上',
        composition: ''
      },
      lines: {
        narration: '身着碧色曲裾的少女敖灵从水雾中跌落,重重摔在地上',
        dialogue: '敖灵: ...'
      },
      mainCharacter: '敖灵',
      shotSize: '中景',
      cameraAngle: '视平',
      lensType: '单人镜头',
      duration: 1
    }
  ]
};

export const ShortVideoCreation: React.FC<ShortVideoCreationProps> = ({ data, onBack, onDataChange }) => {
  const [currentModule, setCurrentModule] = useState<ModuleType>('storyboard'); // Default to storyboard as per image

  // Use passed data or fallback to mock data if empty
  const displayData = (data && data.storyboard && data.storyboard.length > 0) ? data : MOCK_DATA;

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
            虚拟KOL解说短视频
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
          <button className="flex items-center gap-1 px-3 py-1.5 text-sm font-medium text-zinc-600 dark:text-zinc-300 hover:bg-zinc-100 dark:hover:bg-zinc-800 rounded-md transition-colors">
            分镜表
            <ChevronDown size={16} />
          </button>
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
          <CharacterModule characters={displayData.characters || []} />
        )}
        {currentModule === 'storyboard' && (
          <StoryboardModule shots={displayData.storyboard || []} />
        )}
      </main>
    </div>
  );
};
