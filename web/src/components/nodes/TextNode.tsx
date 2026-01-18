import React, { useCallback, useEffect, useState } from 'react';
import { Handle, Position, type NodeProps, useNodeConnections } from '@xyflow/react';
import { NodeWrapper } from './NodeWrapper';
import { useWorkflowContext } from '../../contexts/WorkflowContext';
import { type WorkflowNodeData } from '../../model/types';

export const TextNode = ({ id, data, selected, width, height }: NodeProps & { data: WorkflowNodeData }) => {
  const { updateNodeData } = useWorkflowContext();
  const [text, setText] = useState(data.config?.text || '');

  const connections = useNodeConnections({
    handleType: 'target',
  });

  useEffect(() => {
    setText(data.config?.text || '');
  }, [data.config?.text]);

  const onChange = useCallback((evt: React.ChangeEvent<HTMLTextAreaElement>) => {
    setText(evt.target.value);
  }, []);

  const onBlur = useCallback(() => {
    if (text !== data.config?.text) {
      updateNodeData(id, {
        config: { ...data.config, text }
      });
    }
  }, [id, data.config, text, updateNodeData]);


  // If we receive a message from upstream, show it.
  useEffect(() => {
    if (data.lastMessage !== undefined && typeof data.lastMessage === 'string') {
      // Only update if we have upstream connections (meaning we are acting as a display node)
      if (connections.length > 0 && data.config?.text !== data.lastMessage) {
        updateNodeData(id, {
          config: { ...data.config, text: data.lastMessage }
        });
      }
    }
  }, [data.lastMessage, id, updateNodeData, data.config, connections.length]);

  return (
    <NodeWrapper
      id={id}
      data={data}
      selected={selected}
      className="flex flex-col"
      showSourceHandle={false}
      showTargetHandle={false}
      minWidth={200}
      minHeight={150}
      style={{ width: width ?? 240, height: height ?? 160 }}
    >
      <div className="p-2 flex-1 flex flex-col min-h-0">
        <div className="text-xs text-zinc-500 mb-1">Text Content</div>
        <textarea
          className="w-full flex-1 p-2 text-xs bg-zinc-950 border border-zinc-800 rounded resize-none nodrag focus:outline-none focus:ring-1 focus:ring-blue-500 text-zinc-200"
          value={text}
          onChange={onChange}
          onBlur={onBlur}
          placeholder="Enter text here..."
          onKeyDown={(e) => e.stopPropagation()}
          onMouseDown={(e) => e.stopPropagation()}
        />
      </div>

      {/* Inputs */}
      <Handle
        type="target"
        position={Position.Left}
        className="!bg-slate-500 !w-3 !h-3"
      />

      {/* Outputs */}
      <Handle
        type="source"
        position={Position.Right}
        className="!bg-slate-500 !w-3 !h-3"
      />
    </NodeWrapper>
  );
};
