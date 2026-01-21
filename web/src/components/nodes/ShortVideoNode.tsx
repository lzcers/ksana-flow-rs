import { memo, useState, useEffect } from 'react';
import { Position, type NodeProps } from '@xyflow/react';
import { Maximize2, Clapperboard } from 'lucide-react';
import { NodeWrapper } from './NodeWrapper';
import { FullScreenModal } from '../ui/FullScreenModal';
import { ShortVideoCreation } from '../ShortVideoCreation';
import type { ProjectData } from '../ShortVideoCreation/types';
import { useStore } from '../../store';
import { type NodeData } from '../../model/types';
import './index.css';

const SOURCE_HANDLES = [Position.Right];
const TARGET_HANDLES = [Position.Left];

export const ShortVideoNodeComponent = ({ id, data, selected, width, height }: NodeProps & { data: NodeData }) => {
  const { updateNodeData } = useStore();
  const [isFullScreen, setIsFullScreen] = useState(false);
  const [projectData, setProjectData] = useState<ProjectData | null>(data.config?.projectData || null);

  // Handle upstream text input
  // In a real scenario, this text would be sent to a backend to generate the JSON.
  // Here we just observe it.
  const lastMessage = data.lastMessage;

  useEffect(() => {
    if (lastMessage && typeof lastMessage === 'string') {
      // Logic to trigger generation or update script content could go here
      // For now, we assume the user manually triggers generation or it's mocked
    }
  }, [lastMessage]);

  const headerActions = (
    <div className="flex items-center gap-1">
      <button
        onClick={(e) => {
          e.stopPropagation();
          setIsFullScreen(true);
        }}
        className="text-zinc-400 hover:text-zinc-200 transition-colors p-1 rounded hover:bg-zinc-800"
        title="Open Editor"
      >
        <Maximize2 size={12} />
      </button>
    </div>
  );

  return (
    <NodeWrapper
      id={id}
      data={data}
      selected={selected}
      sourceHandles={SOURCE_HANDLES}
      targetHandles={TARGET_HANDLES}
      className="flex flex-col"
      minWidth={240}
      minHeight={120}
      style={{ width: width ?? 240, height: height ?? 120 }}
      headerActions={headerActions}
    >
      <div className="p-4 flex-1 flex flex-col items-center justify-center min-h-0 bg-zinc-950/50">
        <div className="text-center space-y-2">
          <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-indigo-500 to-purple-600 flex items-center justify-center mx-auto shadow-lg shadow-indigo-500/20">
            <Clapperboard className="text-white" size={24} />
          </div>
          <div>
            <h3 className="text-sm font-medium text-zinc-200">AI Short Video Creator</h3>
            <p className="text-xs text-zinc-500 mt-1">
              {projectData ? 'Project ready' : 'Waiting for input...'}
            </p>
          </div>

          <button
            onClick={() => setIsFullScreen(true)}
            className="px-4 py-1.5 bg-zinc-800 hover:bg-zinc-700 text-zinc-200 text-xs rounded-md transition-colors border border-zinc-700"
          >
            Open Studio
          </button>
        </div>

        {isFullScreen && (
          <FullScreenModal
            isOpen={isFullScreen}
            onClose={() => setIsFullScreen(false)}
            title="Short Video Studio"
            className="bg-white dark:bg-zinc-950"
          >
            <ShortVideoCreation
              data={projectData as ProjectData}
              onBack={() => setIsFullScreen(false)}
              onDataChange={(newData) => {
                setProjectData(newData);
                updateNodeData(id, {
                  config: { ...data.config, projectData: newData }
                });
              }}
            />
          </FullScreenModal>
        )}
      </div>
    </NodeWrapper>
  );
};

export const ShortVideoNode = memo(ShortVideoNodeComponent);
