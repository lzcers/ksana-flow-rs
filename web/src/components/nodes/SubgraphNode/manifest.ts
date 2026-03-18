import { defineNodeManifest } from '../nodeManifest';
import { SubgraphNode } from './index';
import { subgraphNodeMetadata } from './metadata';

export const subgraphNodeManifest = defineNodeManifest({
  metadata: subgraphNodeMetadata,
  Component: SubgraphNode,
});
