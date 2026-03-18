import { defineNodeManifest } from '../nodeManifest';
import { SourceNode } from './index';
import { sourceNodeMetadata } from './metadata';

export const sourceNodeManifest = defineNodeManifest({
  metadata: sourceNodeMetadata,
  Component: SourceNode,
});
