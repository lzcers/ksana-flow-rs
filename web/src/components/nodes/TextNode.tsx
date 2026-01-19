import React, { memo, useCallback, useEffect, useState } from 'react';
import { Handle, Position, type NodeProps, useNodeConnections } from '@xyflow/react';
import { IncremarkContent, ThemeProvider } from '@incremark/react';
import { Eye, Pencil } from 'lucide-react';
import { NodeWrapper } from './NodeWrapper';
import { useStore } from '../../store';
import { type NodeData } from '../../model/types';
import '@incremark/theme/styles.css';
import './index.css';

export const TextNodeComponent = ({ id, data, selected, width, height }: NodeProps & { data: NodeData }) => {
  const { updateNodeData } = useStore();
  const [text, setText] = useState(data.config?.text || '');
  const [isMarkdown, setIsMarkdown] = useState(false);

  const connections = useNodeConnections({
    handleType: 'target',
  });

  const isStreaming = useStore(useCallback((state) => {
    return connections.some(conn => {
      const node = state.nodes.find(n => n.id === conn.source);
      return node?.data?.isOutputStream;
    });
  }, [connections]));

  const [wasStreaming, setWasStreaming] = useState(false);
  useEffect(() => {
    if (!wasStreaming && isStreaming) {
      setText('');
      updateNodeData(id, { config: { ...data.config, text: '' } });
    }
    setWasStreaming(isStreaming);
  }, [isStreaming, wasStreaming, id, updateNodeData]); // Intentionally omitting data.config to avoid loop

  useEffect(() => {
    if (!isStreaming) {
      setText(data.config?.text || '');
    }
  }, [data.config?.text, isStreaming]);

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


  useEffect(() => {
    if (data.lastMessage !== undefined && typeof data.lastMessage === 'string') {
      if (connections.length > 0) {
        if (isStreaming) {
          setText((prev: string) => {
            const next = prev + data.lastMessage;
            updateNodeData(id, {
              config: { ...data.config, text: next }
            });
            return next;
          });
        } else if (data.config?.text !== data.lastMessage) {
          updateNodeData(id, {
            config: { ...data.config, text: data.lastMessage }
          });
        }
      }
    }
  }, [data.lastMessage, id, updateNodeData, connections.length, isStreaming]);

  const headerActions = (
    <button
      onClick={(e) => {
        e.stopPropagation();
        setIsMarkdown(!isMarkdown);
      }}
      className="text-zinc-400 hover:text-zinc-200 transition-colors p-1 rounded hover:bg-zinc-800"
      title={isMarkdown ? "Switch to Edit Mode" : "Switch to Markdown Preview"}
    >
      {isMarkdown ? <Pencil size={12} /> : <Eye size={12} />}
    </button>
  );

  return (
    <NodeWrapper
      id={id}
      data={data}
      selected={selected}
      className="flex flex-col"
      minWidth={200}
      minHeight={150}
      style={{ width: width ?? 240, height: height ?? 160 }}
      headerActions={headerActions}
    >
      <div className="p-2 flex-1 flex flex-col min-h-0">
        <div className="text-xs text-zinc-500 mb-1 flex items-center justify-between">
          <span>Text Content</span>
          <span className="text-[10px] opacity-50">{isMarkdown ? 'Markdown' : 'Raw'}</span>
        </div>

        {isMarkdown ? (
          <div
            className="w-full flex-1 text-xs bg-zinc-950 border border-zinc-800 rounded overflow-auto overflow-x-hidden nodrag text-zinc-200 custom-scrollbar"
            onKeyDown={(e) => e.stopPropagation()}
            onMouseDown={(e) => e.stopPropagation()}
            onWheel={(e) => {
              e.stopPropagation();
            }}
          >
            <ThemeProvider theme="dark">
              <IncremarkContent content={text} isFinished={true} incremarkOptions={{
                math: { tex: true }
              }} />
            </ThemeProvider>
          </div>
        ) : (
          <textarea
            className="w-full flex-1 p-2 text-xs bg-zinc-950 border border-zinc-800 rounded resize-none nodrag focus:outline-none focus:ring-1 focus:ring-blue-500 text-zinc-200"
            value={text}
            onChange={onChange}
            onBlur={onBlur}
            placeholder="Enter text here..."
            onKeyDown={(e) => e.stopPropagation()}
            onMouseDown={(e) => e.stopPropagation()}
          />
        )}
      </div>

      {/* Inputs */}
      <Handle
        type="target"
        position={Position.Left}
        className="!bg-slate-500 !w-3 !h-3"
        style={{ left: -6 }}
      />

      {/* Outputs */}
      <Handle
        type="source"
        position={Position.Right}
        className="!bg-slate-500 !w-3 !h-3"
        style={{ right: -6 }}
      />
    </NodeWrapper>
  );
};

export const TextNode = memo(TextNodeComponent);
