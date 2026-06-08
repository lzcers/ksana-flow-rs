import { defineNodeManifest } from '../nodeManifest';
import { BacktesterNode } from './index';
import { backtesterNodeMetadata } from './metadata';

export const backtesterNodeManifest = defineNodeManifest({
  metadata: backtesterNodeMetadata,
  Component: BacktesterNode,
});
