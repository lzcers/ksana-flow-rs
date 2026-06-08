import { defineNodeManifest } from '../nodeManifest';
import { ReduceNode } from './index';
import { reduceNodeMetadata } from './metadata';

export const reduceNodeManifest = defineNodeManifest({
  metadata: reduceNodeMetadata,
  Component: ReduceNode,
});
