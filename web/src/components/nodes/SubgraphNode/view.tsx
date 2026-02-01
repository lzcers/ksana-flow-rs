import { memo, useEffect } from 'react';
import { type NodeProps, useReactFlow, type Node, Position, useUpdateNodeInternals } from '@xyflow/react';
import { ChevronDown, ChevronUp, Network } from 'lucide-react';
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
  const { getNodes } = useReactFlow();
  const updateNodeInternals = useUpdateNodeInternals();

  // Get child nodes count
  const childNodes = getNodes().filter((n: Node) => n.parentId === id);
  const childCount = childNodes.length;

  useEffect(() => {
    updateNodeInternals(id);
  }, [id, expanded, width, height, updateNodeInternals]);

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
      data={data}
      selected={selected}
      className={expanded ? subgraphNodeStyles.expandedContainer : subgraphNodeStyles.collapsedContainer}
      headerActions={headerActions}
      resizable={expanded}
      minWidth={expanded ? 200 : 260}
      minHeight={expanded ? 200 : 200}
      targetHandles={[Position.Left, Position.Right, Position.Top, Position.Bottom]}
      sourceHandles={[Position.Left, Position.Right, Position.Top, Position.Bottom]}
      style={{ width, height }}
      {...props}
    >
      {!expanded && (
        <div className="flex flex-col items-center justify-center h-full pt-4">
          <div className={subgraphNodeStyles.collapsedIcon}>
            <Network size={20} className="text-zinc-400" />
          </div>
          <span className={subgraphNodeStyles.collapsedLabel}>Subgraph</span>
          {childCount > 0 && (
            <span className={subgraphNodeStyles.collapsedCount}>{childCount} nodes</span>
          )}
        </div>
      )}
    </NodeWrapper>
  );
});
