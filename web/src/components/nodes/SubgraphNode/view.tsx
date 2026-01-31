import { memo } from 'react';
import { type NodeProps } from '@xyflow/react';
import { ChevronDown, ChevronUp } from 'lucide-react';
import { NodeWrapper } from '../shared/NodeWrapper';
import { subgraphNodeStyles } from './styles';
import type { NodeData } from '../../../model/types';

interface SubgraphNodeViewProps extends NodeProps {
  data: NodeData;
  expanded: boolean;
  onToggle: () => void;
}

export const SubgraphNodeView = memo(({
  id,
  data,
  selected,
  expanded,
  onToggle,
  width,
  height,
  ...props
}: SubgraphNodeViewProps) => {
  const headerActions = (
    <div className={subgraphNodeStyles.headerActions}>
      <button
        onClick={(e) => {
          e.stopPropagation();
          onToggle();
        }}
        className={subgraphNodeStyles.headerButton}
        title={expanded ? 'Collapse' : 'Expand'}
      >
        {expanded ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
      </button>
    </div>
  );

  return (
    <NodeWrapper
      id={id}
      type="SubgraphNode"
      data={data}
      selected={selected}
      className={expanded ? subgraphNodeStyles.expandedContainer : subgraphNodeStyles.collapsedContainer}
      headerActions={headerActions}
      resizable={expanded}
      minWidth={200}
      minHeight={100}
      style={{ width, height }}
      {...props}
    >
      {!expanded && (
        <div className="flex items-center justify-center h-full pt-6">
          <span className={subgraphNodeStyles.collapsedLabel}>Subgraph Group</span>
        </div>
      )}
    </NodeWrapper>
  );
});
