import { memo } from 'react';
import { type NodeProps } from '@xyflow/react';
import type { NodeData } from '@/model/workflow/types';
import { useStringNodeConfigField } from '../shared/hooks/useNodeConfigValueField';
import { TimerNodeView } from './view';

export const TimerNode = memo((props: NodeProps & { data: NodeData }) => {
  const { id, data } = props;
  const cronExprField = useStringNodeConfigField({ id, config: data.config, configKey: 'cron_expr' });

  return <TimerNodeView {...props} cronExpr={cronExprField.draft} onCronChange={cronExprField.onChange} />;
});

TimerNode.displayName = 'TimerNode';
