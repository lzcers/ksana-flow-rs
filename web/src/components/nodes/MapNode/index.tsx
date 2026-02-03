import { memo, useCallback } from 'react';
import { type NodeProps } from '@xyflow/react';
import type { NodeData } from '@/model/types';
import { useNodeConfig } from '../shared/hooks/useNodeConfig';
import { useNodeConfigField } from '../shared/hooks/useNodeConfigField';
import { useStore } from '@/store';
import { useMapNodeStream } from './hooks';
import { MapNodeView } from './view';

export const MapNode = memo((props: NodeProps & { data: NodeData }) => {
  const { id, data } = props;
  const { updateConfig } = useNodeConfig(id, data.config);

  const toggleSubgraph = useStore((s) => s.toggleSubgraph);
  const onToggle = useCallback(() => {
    if (toggleSubgraph) toggleSubgraph(id);
  }, [id, toggleSubgraph]);

  const maxConcurrencyField = useNodeConfigField<string>({
    value: String(data.config?.max_concurrency ?? 10),
    commitMode: 'change',
    updateValue: (next) => {
      const n = parseInt(next, 10);
      if (Number.isFinite(n) && n >= 0) updateConfig({ max_concurrency: n });
    },
  });

  const streaming = Boolean(data.config?.streaming ?? false);
  const onStreamingToggle = useCallback(() => {
    updateConfig({ streaming: !streaming });
  }, [streaming, updateConfig]);

  const streamState = useMapNodeStream(id);

  return (
    <MapNodeView
      {...props}
      expanded={data.expanded !== false}
      onToggle={onToggle}
      maxConcurrency={maxConcurrencyField.draft}
      streaming={streaming}
      onMaxConcurrencyChange={maxConcurrencyField.onChange}
      onStreamingToggle={onStreamingToggle}
      streamState={streamState}
    />
  );
});

MapNode.displayName = 'MapNode';

