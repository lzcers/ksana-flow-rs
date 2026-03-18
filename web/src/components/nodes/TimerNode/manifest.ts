import { defineNodeManifest } from '../nodeManifest';
import { TimerNode } from './index';
import { timerNodeMetadata } from './metadata';

export const timerNodeManifest = defineNodeManifest({
  metadata: timerNodeMetadata,
  Component: TimerNode,
});
