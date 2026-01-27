import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

type CommitMode = 'change' | 'blur' | 'manual';

export function useNodeConfigField<T>({
  value,
  commitMode = 'change',
  updateValue,
  isFocused,
  composition = false,
}: {
  value: T;
  commitMode?: CommitMode;
  updateValue: (next: T) => void;
  isFocused?: () => boolean;
  composition?: boolean;
}) {
  const [draft, setDraft] = useState<T>(value);
  const isComposingRef = useRef(false);

  useEffect(() => {
    if (isFocused?.()) return;
    if (composition && isComposingRef.current) return;
    setDraft(value);
  }, [value, isFocused, composition]);

  const commit = useCallback(
    (next: T) => {
      updateValue(next);
    },
    [updateValue],
  );

  const onChange = useCallback(
    (next: T) => {
      setDraft(next);
      if (commitMode === 'change') {
        if (composition && isComposingRef.current) return;
        commit(next);
      }
    },
    [commitMode, commit, composition],
  );

  const onBlur = useCallback(() => {
    if (commitMode === 'blur') commit(draft);
  }, [commitMode, commit, draft]);

  const onCompositionStart = useMemo(() => {
    if (!composition) return undefined;
    return () => {
      isComposingRef.current = true;
    };
  }, [composition]);

  const onCompositionEnd = useMemo(() => {
    if (!composition) return undefined;
    return (next: T) => {
      isComposingRef.current = false;
      setDraft(next);
      if (commitMode === 'change') commit(next);
    };
  }, [composition, commitMode, commit]);

  const onManualCommit = useCallback(() => commit(draft), [commit, draft]);

  return { draft, setDraft, onChange, onBlur, onCompositionStart, onCompositionEnd, onManualCommit };
}
