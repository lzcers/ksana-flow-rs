import type { ComponentType } from 'react';
import type { NodeProps } from '@xyflow/react';
import type { NodeData } from '@/model/workflow/types';
import type { NodeMetadata } from '@/model/nodeRegistry/types';

export type NodeComponent = ComponentType<NodeProps & { data: NodeData }>;

export interface NodeManifest {
  metadata: NodeMetadata;
  Component: NodeComponent;
  color?: string;
}

export function defineNodeManifest<T extends NodeManifest>(manifest: T): T {
  return manifest;
}
