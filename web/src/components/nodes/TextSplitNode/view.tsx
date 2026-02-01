import { Position, type NodeProps } from '@xyflow/react';
import { NodeWrapper } from '../NodeWrapper';
import { type NodeData } from '../../../model/types';
import { textSplitNodeStyles } from './styles';

const SOURCE_HANDLES = [Position.Right, Position.Top, Position.Bottom];
const TARGET_HANDLES = [Position.Left, Position.Top, Position.Bottom];

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
    <NodeWrapper
      id={id}
      type={type}
      data={data}
      selected={selected}
      sourceHandles={SOURCE_HANDLES}
      targetHandles={TARGET_HANDLES}
      className="flex flex-col"
      minWidth={320}
      minHeight={450}
      style={{ width, height }}
    >
      <div className={textSplitNodeStyles.container}>
        {/* Mode Selection */}
        <div className={textSplitNodeStyles.section}>
          <div className={textSplitNodeStyles.configRow}>
            <span className={textSplitNodeStyles.configLabel}>Split Mode</span>
            <select
              className={textSplitNodeStyles.select}
              value={mode}
              onChange={(e) => onModeChange(e.target.value as 'by_line_count' | 'by_rule')}
            >
              <option value="by_line_count">By Line Count</option>
              <option value="by_rule">By Rule (Keywords)</option>
            </select>
          </div>
        </div>

        {/* Mode-specific settings */}
        {mode === 'by_line_count' ? (
          <div className={textSplitNodeStyles.section}>
            <span className={textSplitNodeStyles.sectionTitle}>Line Count Settings</span>
            <div className={textSplitNodeStyles.configRow}>
              <span className={textSplitNodeStyles.configLabel}>Max Lines Per Part</span>
              <input
                type="number"
                className={textSplitNodeStyles.numberInput}
                value={maxLinesPerPart}
                onChange={(e) => onMaxLinesChange(parseInt(e.target.value) || 1)}
                min={1}
                max={10000}
              />
            </div>
          </div>
        ) : (
          <div className={textSplitNodeStyles.section}>
            <span className={textSplitNodeStyles.sectionTitle}>Rule Settings</span>
            <div className={textSplitNodeStyles.configRow}>
              <span className={textSplitNodeStyles.configLabel}>Keywords (comma-separated)</span>
            </div>
            <input
              type="text"
              className={textSplitNodeStyles.tagInput}
              value={keywords}
              onChange={(e) => onKeywordsChange(e.target.value)}
              placeholder="e.g., Chapter, Section, Part"
              onKeyDown={(e) => e.stopPropagation()}
            />
            <div className={textSplitNodeStyles.configRow}>
              <span className={textSplitNodeStyles.configLabel}>Require Prefix</span>
              <input
                type="text"
                className={textSplitNodeStyles.input}
                value={requirePrefix}
                onChange={(e) => onRequirePrefixChange(e.target.value)}
                placeholder="Optional prefix"
                onKeyDown={(e) => e.stopPropagation()}
              />
            </div>
            <div className={textSplitNodeStyles.configRow}>
              <span className={textSplitNodeStyles.configLabel}>Only Keep Matched Ranges</span>
              <input
                type="checkbox"
                className={textSplitNodeStyles.checkbox}
                checked={ruleOnlyKeepMatched}
                onChange={(e) => onRuleOnlyKeepMatchedChange(e.target.checked)}
              />
            </div>
          </div>
        )}

        {/* General Options */}
        <div className={textSplitNodeStyles.section}>
          <span className={textSplitNodeStyles.sectionTitle}>Options</span>
          <div className={textSplitNodeStyles.configRow}>
            <span className={textSplitNodeStyles.configLabel}>Remove Empty Lines</span>
            <input
              type="checkbox"
              className={textSplitNodeStyles.checkbox}
              checked={removeEmptyLines}
              onChange={(e) => onRemoveEmptyLinesChange(e.target.checked)}
            />
          </div>
          <div className={textSplitNodeStyles.configRow}>
            <span className={textSplitNodeStyles.configLabel}>Add Line Numbers</span>
            <input
              type="checkbox"
              className={textSplitNodeStyles.checkbox}
              checked={lineNumbersEnabled}
              onChange={(e) => onLineNumbersEnabledChange(e.target.checked)}
            />
          </div>
          {lineNumbersEnabled && (
            <div className={textSplitNodeStyles.configRow}>
              <span className={textSplitNodeStyles.configLabel}>Template</span>
              <input
                type="text"
                className={textSplitNodeStyles.input}
                value={lineNumbersTemplate}
                onChange={(e) => onLineNumbersTemplateChange(e.target.value)}
                placeholder="{line}: "
                onKeyDown={(e) => e.stopPropagation()}
              />
            </div>
          )}
        </div>
      </div>
    </NodeWrapper>
  );
}
