import { Position } from '@xyflow/react';
import type { NodeMetadata } from '@/model/nodeRegistry/types';

export const emailNotifyNodeMetadata: NodeMetadata = {
  type: 'EmailNotifyNode',
  displayName: 'Email Notify',
  category: 'output',
  icon: 'mail',
  description: '邮件通知节点',
  ports: {
    inputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Left },
      { id: 'subject', label: 'Subject', kind: 'data', dataType: 'string', position: Position.Left },
      { id: 'body', label: 'Body', kind: 'data', dataType: 'string', position: Position.Left },
    ],
    outputs: [{ id: 'ctrl', label: '', kind: 'control', position: Position.Right }],
  },
  defaultConfig: {
    to: '',
    subject: '',
    body: '',
  },
  defaultSize: { width: 300, height: 180 },
};
