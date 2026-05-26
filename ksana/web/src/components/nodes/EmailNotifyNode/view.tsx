import { type NodeProps } from '@xyflow/react';
import type { NodeData } from '@/model/workflow/types';
import { FormNodeView } from '../shared/FormNodeView';

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
    <FormNodeView
      id={id}
      type={type}
      data={data}
      selected={selected}
      width={width}
      height={height}
      minWidth={300}
      minHeight={180}
      groups={[
        {
          fields: [
            {
              kind: 'input',
              label: 'Subject',
              value: subject,
              onChange: onSubjectChange,
              placeholder: 'Email subject...',
              controlVariant: 'plain',
            },
            {
              kind: 'textarea',
              label: 'Body',
              value: body,
              onChange: onBodyChange,
              rows: 4,
              placeholder: 'Email body...',
              onCompositionStart: onBodyCompositionStart,
              onCompositionEnd: onBodyCompositionEnd,
            },
          ],
        },
      ]}
    />
  );
}
