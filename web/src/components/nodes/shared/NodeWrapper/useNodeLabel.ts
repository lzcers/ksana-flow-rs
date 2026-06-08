import * as React from 'react';

export function useNodeLabel({
  id,
  label,
  updateNodeData,
}: {
  id: string;
  label?: string;
  updateNodeData: (id: string, patch: Record<string, unknown>) => void;
}) {
  const [editingLabel, setEditingLabel] = React.useState(false);
  const [labelDraft, setLabelDraft] = React.useState<string>(label || '');
  const inputRef = React.useRef<HTMLInputElement>(null);

  React.useEffect(() => {
    if (!editingLabel) setLabelDraft(label || '');
  }, [label, editingLabel]);

  React.useEffect(() => {
    if (editingLabel && inputRef.current) {
      inputRef.current.focus();
      inputRef.current.select();
    }
  }, [editingLabel]);

  const commitLabel = React.useCallback(() => {
    const next = labelDraft.trim();
    if (next && next !== label) {
      updateNodeData(id, { label: next });
    }
    setEditingLabel(false);
  }, [id, label, labelDraft, updateNodeData]);

  const cancelLabel = React.useCallback(() => {
    setEditingLabel(false);
    setLabelDraft(label || '');
  }, [label]);

  return {
    editingLabel,
    setEditingLabel,
    labelDraft,
    setLabelDraft,
    inputRef,
    commitLabel,
    cancelLabel,
  };
}
