import { memo } from 'react';
import { type NodeProps } from '@xyflow/react';
import type { NodeData } from '../../../model/types';
import { useNodeConfig } from '../shared/hooks/useNodeConfig';
import { useNodeConfigField } from '../shared/hooks/useNodeConfigField';
import { TimerNodeView } from './view';

export const TimerNode = memo((props: NodeProps & { data: NodeData }) => {
  const { id, data } = props;
  const { updateConfig } = useNodeConfig(id, data.config);

  const cronExprField = useNodeConfigField<string>({
    value: String(data.config?.cron_expr ?? ''),
    commitMode: 'change',
    updateValue: (next) => updateConfig({ cron_expr: next }),
  });

  return <TimerNodeView {...props} cronExpr={cronExprField.draft} onCronChange={cronExprField.onChange} />;
});

TimerNode.displayName = 'TimerNode';
