import type { NodeMetadata } from '@/model/nodeRegistry/types';
import { backtesterNodeMetadata } from './BacktesterNode/metadata';
import { emailNotifyNodeMetadata } from './EmailNotifyNode/metadata';
import { imgGenNodeMetadata } from './ImgGenNode/metadata';
import { llmNodeMetadata } from './LLMNode/metadata';
import { mapNodeMetadata } from './MapNode/metadata';
import { reduceNodeMetadata } from './ReduceNode/metadata';
import { sourceNodeMetadata } from './SourceNode/metadata';
import { subgraphNodeMetadata } from './SubgraphNode/metadata';
import { textFileNodeMetadata } from './TextFileNode/metadata';
import { textMergeNodeMetadata } from './TextMergeNode/metadata';
import { textNodeMetadata } from './TextNode/metadata';
import { textSplitNodeMetadata } from './TextSplitNode/metadata';
import { timerNodeMetadata } from './TimerNode/metadata';
import { volMfiNodeMetadata } from './VolMfiNode/metadata';

export const BUILTIN_NODE_METADATA: NodeMetadata[] = [
  textNodeMetadata,
  llmNodeMetadata,
  textMergeNodeMetadata,
  textSplitNodeMetadata,
  mapNodeMetadata,
  reduceNodeMetadata,
  subgraphNodeMetadata,
  timerNodeMetadata,
  sourceNodeMetadata,
  volMfiNodeMetadata,
  backtesterNodeMetadata,
  emailNotifyNodeMetadata,
  textFileNodeMetadata,
  imgGenNodeMetadata,
];
