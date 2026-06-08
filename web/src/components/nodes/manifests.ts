import type { NodeManifest } from './nodeManifest';
import { backtesterNodeManifest } from './BacktesterNode/manifest';
import { emailNotifyNodeManifest } from './EmailNotifyNode/manifest';
import { imgGenNodeManifest } from './ImgGenNode/manifest';
import { llmNodeManifest } from './LLMNode/manifest';
import { mapNodeManifest } from './MapNode/manifest';
import { reduceNodeManifest } from './ReduceNode/manifest';
import { sourceNodeManifest } from './SourceNode/manifest';
import { subgraphNodeManifest } from './SubgraphNode/manifest';
import { textFileNodeManifest } from './TextFileNode/manifest';
import { textMergeNodeManifest } from './TextMergeNode/manifest';
import { textNodeManifest } from './TextNode/manifest';
import { textSplitNodeManifest } from './TextSplitNode/manifest';
import { timerNodeManifest } from './TimerNode/manifest';
import { volMfiNodeManifest } from './VolMfiNode/manifest';

export const BUILTIN_NODE_MANIFESTS: NodeManifest[] = [
  textNodeManifest,
  llmNodeManifest,
  textMergeNodeManifest,
  textSplitNodeManifest,
  mapNodeManifest,
  reduceNodeManifest,
  subgraphNodeManifest,
  timerNodeManifest,
  sourceNodeManifest,
  volMfiNodeManifest,
  backtesterNodeManifest,
  emailNotifyNodeManifest,
  textFileNodeManifest,
  imgGenNodeManifest,
];
