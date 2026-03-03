import { type NodeProps } from '@xyflow/react';
import type { NodeData } from '@/model/workflow/types';
import { NodeWrapper } from '../shared/NodeWrapper';
import { emailNotifyNodeStyles } from './styles';

export function EmailNotifyNodeView({
  id,
  type,
  data,
  selected,
  width,
  height,
  subject,
  body,
  onSubjectChange,
  onBodyChange,
  onBodyCompositionStart,
  onBodyCompositionEnd,
}: NodeProps & {
  data: NodeData;
} & {
  subject: string;
  body: string;
  onSubjectChange: (next: string) => void;
  onBodyChange: (next: string) => void;
  onBodyCompositionStart?: () => void;
  onBodyCompositionEnd?: (next: string) => void;
}) {
  return (
    <NodeWrapper
      id={id}
      type={type}
      data={data}
      selected={selected}
      minWidth={300}
      minHeight={180}
      style={{ width, height }}
    >
      <div className={emailNotifyNodeStyles.section}>
        <div>
          <label className={emailNotifyNodeStyles.label}>Subject</label>
          <input
            className={emailNotifyNodeStyles.subjectInput}
            value={subject}
            onChange={(e) => onSubjectChange(e.target.value)}
            onKeyDown={(e) => e.stopPropagation()}
            placeholder="Email subject..."
          />
        </div>
        <div>
          <label className={emailNotifyNodeStyles.label}>Body</label>
          <textarea
            className={emailNotifyNodeStyles.bodyTextarea}
            rows={4}
            value={body}
            onChange={(e) => onBodyChange(e.target.value)}
            onCompositionStart={() => onBodyCompositionStart?.()}
            onCompositionEnd={(e) => onBodyCompositionEnd?.(e.currentTarget.value)}
            onKeyDown={(e) => e.stopPropagation()}
            placeholder="Email body..."
          />
        </div>
      </div>
    </NodeWrapper>
  );
}
