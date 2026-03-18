import { defineNodeManifest } from '../nodeManifest';
import { MapNode } from './index';
import { mapNodeMetadata } from './metadata';

export const mapNodeManifest = defineNodeManifest({
  metadata: mapNodeMetadata,
  Component: MapNode,
});
