import { Position } from '@xyflow/react';
import type { NodeMetadata } from '@/model/nodeRegistry/types';

export const llmNodeMetadata: NodeMetadata = {
  type: 'LLMNode',
  displayName: 'LLM',
  category: 'ai',
  icon: 'bot',
  description: '大语言模型节点',
  ports: {
    inputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Left },
      { id: 'system', label: 'System', kind: 'data', dataType: 'string', position: Position.Left },
      { id: 'user', label: 'User', kind: 'data', dataType: 'string', position: Position.Left },
      { id: 'context', label: 'Context', kind: 'data', dataType: 'json', position: Position.Left, multiple: true },
    ],
    outputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Right },
      { id: 'output', label: 'Output', kind: 'data', dataType: 'string', position: Position.Right },
      { id: 'usage', label: 'Usage', kind: 'data', dataType: 'json', position: Position.Right },
    ],
  },
  defaultConfig: {
    model: 'deepseek-chat',
    stream: true,
    system_prompt: '',
    user_prompt_template: '',
    output: '',
  },
  defaultSize: { width: 300, height: 400 },
};
