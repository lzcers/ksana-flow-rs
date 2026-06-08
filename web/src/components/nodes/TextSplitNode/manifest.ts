import { defineNodeManifest } from '../nodeManifest';
import { TextSplitNode } from './index';
import { textSplitNodeMetadata } from './metadata';

export const textSplitNodeManifest = defineNodeManifest({
  metadata: textSplitNodeMetadata,
  Component: TextSplitNode,
});
