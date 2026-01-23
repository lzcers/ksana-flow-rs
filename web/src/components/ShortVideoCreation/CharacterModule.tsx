import React from 'react';
import type { CharacterData } from './types';
import { User, Tag } from 'lucide-react';

interface CharacterModuleProps {
  characters: CharacterData[];
  isNodeCompleted?: (value: any) => boolean;
}

export const CharacterModule: React.FC<CharacterModuleProps> = ({ characters, isNodeCompleted }) => {
  return (
    <div className="w-full h-full overflow-auto bg-zinc-950 custom-scrollbar">
      <table className="w-full text-left border-collapse">
        <thead className="sticky top-0 bg-zinc-950 z-10 shadow-sm shadow-black/20">
          <tr className="border-b border-zinc-800 text-sm font-medium text-zinc-300">
            <th className="p-4 w-20">ID</th>
            <th className="p-4 w-32">头像</th>
            <th className="p-4 w-40">名称</th>
            <th className="p-4">描述</th>
            <th className="p-4 w-48">标签</th>
          </tr>
        </thead>
        <tbody className="divide-y divide-zinc-900">
          {characters?.map((char) => {
            const isComplete = isNodeCompleted ? isNodeCompleted(char) : true;
            return (
              <tr
                key={char.id}
                className={`hover:bg-zinc-900/50 transition-colors text-sm text-zinc-400 ${!isComplete ? 'opacity-60 bg-zinc-900/30' : ''}`}
              >
                <td className="p-4 text-zinc-500 relative">
                  {char.id}
                  {!isComplete && (
                    <span className="absolute left-1 top-1/2 -translate-y-1/2 w-1 h-1 bg-indigo-500/50 rounded-full animate-pulse" />
                  )}
                </td>
                <td className="p-4">
                  <div className="w-12 h-12 rounded-full overflow-hidden bg-zinc-900 flex items-center justify-center border border-zinc-800">
                    {char.avatar ? (
                      <img src={char.avatar} alt={char.name} className="w-full h-full object-cover" />
                    ) : (
                      <User className="text-zinc-600" size={24} />
                    )}
                  </div>
                </td>
                <td className="p-4 font-medium text-zinc-200">{char.name}</td>
                <td className="p-4">
                  <p className="line-clamp-3">{char.description}</p>
                </td>
                <td className="p-4">
                  <div className="flex flex-wrap gap-1">
                    {char.tags?.map((tag, idx) => (
                      <span key={idx} className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs bg-zinc-900 text-zinc-500 border border-zinc-800">
                        <Tag size={10} />
                        {tag}
                      </span>
                    ))}
                  </div>
                </td>
              </tr>
            );
          })}
          {characters.length === 0 && (
            <tr>
              <td colSpan={5} className="p-8 text-center text-zinc-600">
                No characters generated yet.
              </td>
            </tr>
          )}
        </tbody>
      </table>
    </div>
  );
};
