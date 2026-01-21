import React, { memo, useCallback, useState, useEffect } from 'react';
import { Position, type NodeProps } from '@xyflow/react';
import { NodeWrapper } from './NodeWrapper';
import { useStore } from '../../store';
import { type NodeData } from '../../model/types';

const SOURCE_HANDLES = [Position.Right, Position.Top, Position.Bottom];
const TARGET_HANDLES = [Position.Left];

export const TextMergeNodeComponent = ({ id, data, selected, width, height }: NodeProps & { data: NodeData }) => {
  const { updateNodeData } = useStore();
  const [separator, setSeparator] = useState(data.config?.separator ?? '\n');

  useEffect(() => {
    setSeparator(data.config?.separator ?? '\n');
  }, [data.config?.separator]);

  const onChange = useCallback((evt: React.ChangeEvent<HTMLInputElement>) => {
    setSeparator(evt.target.value);
  }, []);

  const onBlur = useCallback(() => {
    if (separator !== data.config?.separator) {
      updateNodeData(id, {
        config: { ...data.config, separator }
      });
    }
  }, [id, data.config, separator, updateNodeData]);

  return (
    <NodeWrapper
      id={id}
      data={data}
      selected={selected}
      sourceHandles={SOURCE_HANDLES}
      targetHandles={TARGET_HANDLES}
      className="flex flex-col"
      minWidth={180}
      minHeight={150}
      style={{ width: width ?? 180, height: height ?? 100 }}
    >
      <div className="p-2 flex-1 flex flex-col min-h-0">
        <div className="text-xs text-zinc-500 mb-1">
          Separator
        </div>
        <input
          className="w-full p-2 text-xs bg-zinc-950 border border-zinc-800 rounded nodrag focus:outline-none focus:ring-1 focus:ring-blue-500 text-zinc-200"
          value={separator}
          onChange={onChange}
          onBlur={onBlur}
          placeholder="Separator"
          onKeyDown={(e) => e.stopPropagation()}
          onMouseDown={(e) => e.stopPropagation()}
        />
        <div className="mt-2 text-[10px] text-zinc-500">
          Inputs are merged in alphabetical order of source node IDs.
        </div>
      </div>
    </NodeWrapper>
  );
};

export const TextMergeNode = memo(TextMergeNodeComponent);
