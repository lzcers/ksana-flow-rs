import { Position } from '@xyflow/react';
import type { NodeMetadata } from '@/model/nodeRegistry/types';

export const imgGenNodeMetadata: NodeMetadata = {
  type: 'ImgGenNode',
  displayName: 'Image Gen',
  category: 'ai',
  icon: 'image',
  description: 'AI 图像生成节点',
  ports: {
    inputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Left },
      { id: 'prompt', label: 'Prompt', kind: 'data', dataType: 'string', position: Position.Left },
    ],
    outputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Right },
      { id: 'imageUrl', label: 'Image URL', kind: 'data', dataType: 'string', position: Position.Right },
    ],
  },
  defaultConfig: {
    model: 'google/gemini-3.1-flash-image-preview',
    aspect_ratio: '1:1',
    image_size: '1K',
    input_image_file_id: '',
    user_prompt_template: '',
    output: '',
  },
  defaultSize: { width: 400, height: 400 },
};
