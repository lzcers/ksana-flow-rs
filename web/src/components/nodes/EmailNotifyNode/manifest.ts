import { defineNodeManifest } from '../nodeManifest';
import { EmailNotifyNode } from './index';
import { emailNotifyNodeMetadata } from './metadata';

export const emailNotifyNodeManifest = defineNodeManifest({
  metadata: emailNotifyNodeMetadata,
  Component: EmailNotifyNode,
});
