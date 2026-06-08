import { defineNodeManifest } from '../nodeManifest';
import { TextNode } from './index';
import { textNodeMetadata } from './metadata';

export const textNodeManifest = defineNodeManifest({
  metadata: textNodeMetadata,
  Component: TextNode,
});
