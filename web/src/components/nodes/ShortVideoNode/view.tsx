import { Clapperboard, Maximize2 } from 'lucide-react';
import { Position, type NodeProps } from '@xyflow/react';
import { FullScreenModal } from '../../ui/FullScreenModal';
import { ShortVideoCreation } from '../../ShortVideoCreation';
import type { ProjectData } from '../../ShortVideoCreation/types';
import type { NodeData } from '../../../model/types';
import { NodeWrapper } from '../shared/NodeWrapper';
import { shortVideoNodeStyles } from './styles';

const SOURCE_HANDLES = [Position.Right];
const TARGET_HANDLES = [Position.Left];

export function ShortVideoNodeView({
  id,
  type,
  data,
  selected,
  isFullScreen,
  setIsFullScreen,
  projectData,
  isNodeCompleted,
  onProjectDataChange,
}: NodeProps & {
  data: NodeData;
} & {
  isFullScreen: boolean;
  setIsFullScreen: (next: boolean) => void;
  projectData: ProjectData | null;
  isNodeCompleted: (value: any) => boolean;
  onProjectDataChange: (next: ProjectData) => void;
}) {
  const headerActions = (
    <div className={shortVideoNodeStyles.headerActions}>
      <button
        onClick={(e) => {
          e.stopPropagation();
          setIsFullScreen(true);
        }}
        className={shortVideoNodeStyles.headerButton}
        title="Open Editor"
      >
        <Maximize2 size={12} />
      </button>
    </div>
  );

  return (
    <NodeWrapper
      id={id}
      type={type}
      data={data}
      selected={selected}
      sourceHandles={SOURCE_HANDLES}
      targetHandles={TARGET_HANDLES}
      className={shortVideoNodeStyles.wrapperClass}
      minWidth={240}
      minHeight={160}
      resizable={false}
      headerActions={headerActions}
    >
      <div className={shortVideoNodeStyles.container}>
        <div className={shortVideoNodeStyles.card}>
          <div className={shortVideoNodeStyles.iconWrap}>
            <Clapperboard className="text-zinc-500" size={24} />
          </div>
          <div>
            <h3 className={shortVideoNodeStyles.title}>AI Short Video Creator</h3>
            <p className={shortVideoNodeStyles.subtitle}>{projectData ? 'Project ready' : 'Waiting for input...'}</p>
          </div>

          <button onClick={() => setIsFullScreen(true)} className={shortVideoNodeStyles.button}>
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
              isNodeCompleted={isNodeCompleted}
              onDataChange={onProjectDataChange}
            />
          </FullScreenModal>
        )}
      </div>
    </NodeWrapper>
  );
}

