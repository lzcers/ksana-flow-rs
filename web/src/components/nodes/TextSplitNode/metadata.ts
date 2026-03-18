import { Position } from '@xyflow/react';
import type { NodeMetadata } from '@/model/nodeRegistry/types';

export const textSplitNodeMetadata: NodeMetadata = {
  type: 'TextSplitNode',
  displayName: 'Text Split',
  category: 'transform',
  icon: 'split',
  description: '分割文本为多个部分',
  ports: {
    inputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Left },
      { id: 'text', label: 'Text', kind: 'data', dataType: 'string', position: Position.Left },
    ],
    outputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Right },
      { id: 'parts', label: 'Parts', kind: 'data', dataType: 'json', position: Position.Right },
    ],
  },
  defaultConfig: {
    mode: {
      by_line_count: {
        max_lines_per_part: 200,
      },
    },
    remove_empty_lines: false,
    line_numbers: {
      enabled: false,
      template: '{line}: ',
      pad_width: null,
      pad_char: '0',
    },
    rule_only_keep_matched_ranges: false,
  },
  defaultSize: { width: 320, height: 450 },
};
