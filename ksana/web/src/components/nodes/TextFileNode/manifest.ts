import { defineNodeManifest } from '../nodeManifest';
import { TextFileNode } from './index';
import { textFileNodeMetadata } from './metadata';

export const textFileNodeManifest = defineNodeManifest({
  metadata: textFileNodeMetadata,
  Component: TextFileNode,
});
