import { defineNodeManifest } from '../nodeManifest';
import { VolMfiNode } from './index';
import { volMfiNodeMetadata } from './metadata';

export const volMfiNodeManifest = defineNodeManifest({
  metadata: volMfiNodeMetadata,
  Component: VolMfiNode,
});
