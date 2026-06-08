import { type NodeProps } from '@xyflow/react';
import { type NodeData } from '@/model/workflow/types';
import { FormNodeView } from '../shared/FormNodeView';

export function TextSplitNodeView({
  id,
  type,
  data,
  selected,
  width,
  height,
  mode,
  maxLinesPerPart,
  keywords,
  requirePrefix,
  removeEmptyLines,
  lineNumbersEnabled,
  lineNumbersTemplate,
  ruleOnlyKeepMatched,
  onModeChange,
  onMaxLinesChange,
  onKeywordsChange,
  onRequirePrefixChange,
  onRemoveEmptyLinesChange,
  onLineNumbersEnabledChange,
  onLineNumbersTemplateChange,
  onRuleOnlyKeepMatchedChange,
}: NodeProps & {
  data: NodeData;
} & {
  mode: 'by_line_count' | 'by_rule';
  maxLinesPerPart: number;
  keywords: string;
  requirePrefix: string;
  removeEmptyLines: boolean;
  lineNumbersEnabled: boolean;
  lineNumbersTemplate: string;
  ruleOnlyKeepMatched: boolean;
  onModeChange: (next: 'by_line_count' | 'by_rule') => void;
  onMaxLinesChange: (next: number) => void;
  onKeywordsChange: (next: string) => void;
  onRequirePrefixChange: (next: string) => void;
  onRemoveEmptyLinesChange: (next: boolean) => void;
  onLineNumbersEnabledChange: (next: boolean) => void;
  onLineNumbersTemplateChange: (next: string) => void;
  onRuleOnlyKeepMatchedChange: (next: boolean) => void;
}) {
  return (
    <FormNodeView
      id={id}
      type={type}
      data={data}
      selected={selected}
      width={width}
      height={height}
      minWidth={320}
      minHeight={450}
      contentClassName="space-y-3"
      groups={[
        {
          fields: [
            {
              kind: 'select',
              label: 'Split Mode',
              value: mode,
              onChange: next => onModeChange(next as 'by_line_count' | 'by_rule'),
              options: [
                { value: 'by_line_count', label: 'By Line Count' },
                { value: 'by_rule', label: 'By Rule (Keywords)' },
              ],
            },
          ],
        },
        ...(mode === 'by_line_count'
          ? [
              {
                title: 'Line Count Settings',
                fields: [
                  {
                    kind: 'input' as const,
                    label: 'Max Lines Per Part',
                    value: String(maxLinesPerPart),
                    onChange: (next: string) => onMaxLinesChange(parseInt(next, 10) || 1),
                    inputType: 'number' as const,
                    min: 1,
                    max: 10000,
                    controlClassName: 'w-20 text-center',
                  },
                ],
              },
            ]
          : [
              {
                title: 'Rule Settings',
                fields: [
                  {
                    kind: 'input' as const,
                    label: 'Keywords (comma-separated)',
                    value: keywords,
                    onChange: onKeywordsChange,
                    placeholder: 'e.g., Chapter, Section, Part',
                  },
                  {
                    kind: 'input' as const,
                    label: 'Require Prefix',
                    value: requirePrefix,
                    onChange: onRequirePrefixChange,
                    placeholder: 'Optional prefix',
                  },
                  {
                    kind: 'checkbox' as const,
                    label: 'Only Keep Matched Ranges',
                    checked: ruleOnlyKeepMatched,
                    onChange: onRuleOnlyKeepMatchedChange,
                  },
                ],
              },
            ]),
        {
          title: 'Options',
          fields: [
            {
              kind: 'checkbox',
              label: 'Remove Empty Lines',
              checked: removeEmptyLines,
              onChange: onRemoveEmptyLinesChange,
            },
            {
              kind: 'checkbox',
              label: 'Add Line Numbers',
              checked: lineNumbersEnabled,
              onChange: onLineNumbersEnabledChange,
            },
            ...(lineNumbersEnabled
              ? [
                  {
                    kind: 'input' as const,
                    label: 'Template',
                    value: lineNumbersTemplate,
                    onChange: onLineNumbersTemplateChange,
                    placeholder: '{line}: ',
                  },
                ]
              : []),
          ],
        },
      ]}
    />
  );
}
