import { defineNodeManifest } from '../nodeManifest';
import { LLMNode } from './index';
import { llmNodeMetadata } from './metadata';

export const llmNodeManifest = defineNodeManifest({
  metadata: llmNodeMetadata,
  Component: LLMNode,
});
