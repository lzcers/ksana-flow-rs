import { defineNodeManifest } from '../nodeManifest';
import { TextMergeNode } from './index';
import { textMergeNodeMetadata } from './metadata';

export const textMergeNodeManifest = defineNodeManifest({
  metadata: textMergeNodeMetadata,
  Component: TextMergeNode,
});
