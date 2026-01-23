import React from 'react';
import type { StoryboardShot } from './types';
import { User } from 'lucide-react';

interface StoryboardModuleProps {
  shots: StoryboardShot[];
  isNodeCompleted?: (value: any) => boolean;
}

export const StoryboardModule: React.FC<StoryboardModuleProps> = ({ shots, isNodeCompleted }) => {
  return (
    <div className="w-full h-full overflow-auto bg-zinc-950 custom-scrollbar">
      <table className="w-full text-left border-collapse min-w-[1200px]">
        <thead className="sticky top-0 bg-zinc-950 z-10 shadow-sm shadow-black/20">
          <tr className="border-b border-zinc-800 text-sm font-semibold text-zinc-300">
            <th className="p-4 w-16 text-center">镜号</th>
            <th className="p-4 w-64">画面</th>
            <th className="p-4 w-80">画面描述</th>
            <th className="p-4 w-80">台词</th>
            <th className="p-4 w-24 text-center">主人物</th>
            <th className="p-4 w-24">景别</th>
            <th className="p-4 w-24">摄像机角度</th>
            <th className="p-4 w-24">镜头类型</th>
            <th className="p-4 w-16 text-center">时长</th>
          </tr>
        </thead>
        <tbody className="divide-y divide-zinc-900">
          {shots?.map((shot) => {
            const isComplete = isNodeCompleted ? isNodeCompleted(shot) : true;
            return (
              <tr
                key={shot.id}
                className={`hover:bg-zinc-900/50 transition-colors text-sm text-zinc-400 ${!isComplete ? 'opacity-60 bg-zinc-900/30' : ''}`}
              >
                <td className="p-4 text-center font-medium relative text-zinc-500">
                  {shot.shotNo}
                  {!isComplete && (
                    <span className="absolute left-1 top-1/2 -translate-y-1/2 w-1 h-1 bg-indigo-500/50 rounded-full animate-pulse" />
                  )}
                </td>
                <td className="p-4">
                  <div className="w-full aspect-video rounded-md overflow-hidden bg-zinc-900 border border-zinc-800 relative group">
                    {shot.image ? (
                      <img src={shot.image} alt={`Shot ${shot.shotNo}`} className="w-full h-full object-cover" />
                    ) : (
                      <div className="w-full h-full flex items-center justify-center text-zinc-600 text-xs">No Image</div>
                    )}
                  </div>
                </td>
                <td className="p-4 align-top">
                  <div className="space-y-2 text-xs">
                    {shot.description.background && (
                      <p><span className="font-semibold text-zinc-500">背景：</span>{shot.description.background}</p>
                    )}
                    {shot.description.relation && (
                      <p><span className="font-semibold text-zinc-500">关系：</span>{shot.description.relation}</p>
                    )}
                    {shot.description.composition && (
                      <p><span className="font-semibold text-zinc-500">构图：</span>{shot.description.composition}</p>
                    )}
                  </div>
                </td>
                <td className="p-4 align-top">
                  <div className="space-y-2 text-xs">
                    {shot.lines.narration && (
                      <p><span className="font-semibold text-zinc-500">旁白：</span>{shot.lines.narration}</p>
                    )}
                    {shot.lines.dialogue && (
                      <p><span className="font-semibold text-zinc-500">台词：</span>{shot.lines.dialogue}</p>
                    )}
                  </div>
                </td>
                <td className="p-4 align-top text-center">
                  <div className="flex flex-col items-center gap-2">
                    <div className="w-10 h-10 rounded-full overflow-hidden bg-zinc-900 border border-zinc-800 flex items-center justify-center">
                      <User size={20} className="text-zinc-600" />
                    </div>
                    <span className="text-xs">{shot.mainCharacter}</span>
                  </div>
                </td>
                <td className="p-4 align-top">{shot.shotSize}</td>
                <td className="p-4 align-top">{shot.cameraAngle}</td>
                <td className="p-4 align-top">{shot.lensType}</td>
                <td className="p-4 align-top text-center">{shot.duration}</td>
              </tr>
            );
          })}
          {shots.length === 0 && (
            <tr>
              <td colSpan={9} className="p-8 text-center text-zinc-600">
                No shots generated yet.
              </td>
            </tr>
          )}
        </tbody>
      </table>
    </div>
  );
};
