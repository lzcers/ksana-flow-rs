import { useCallback, useEffect, useMemo, useState } from 'react';
import { useNodeConfig } from '../shared/hooks/useNodeConfig';
import type { NodeData } from '@/model/workflow/types';

type LineCountMode = { max_lines_per_part?: number };
type HeadingKeywordsRule = { keywords?: string[]; require_prefix?: string | null };
type RuleMode = { rule?: { heading_keywords?: HeadingKeywordsRule } };
type LineNumbersConfig = { enabled?: boolean; template?: string };

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function getLineCountMode(mode: unknown): LineCountMode | undefined {
  if (!isRecord(mode) || !('by_line_count' in mode) || !isRecord(mode.by_line_count)) {
    return undefined;
  }

  return mode.by_line_count as LineCountMode;
}

function getRuleMode(mode: unknown): RuleMode | undefined {
  if (!isRecord(mode) || !('by_rule' in mode) || !isRecord(mode.by_rule)) {
    return undefined;
  }

  return mode.by_rule as RuleMode;
}

function getHeadingKeywordsRule(mode: unknown): HeadingKeywordsRule | undefined {
  const ruleMode = getRuleMode(mode);
  if (!ruleMode?.rule || !isRecord(ruleMode.rule.heading_keywords)) {
    return undefined;
  }

  return ruleMode.rule.heading_keywords as HeadingKeywordsRule;
}

function getLineNumbersConfig(value: unknown): LineNumbersConfig | undefined {
  if (!isRecord(value)) {
    return undefined;
  }

  return value as LineNumbersConfig;
}

export function useTextSplitNodeController(id: string, data: NodeData) {
  const { updateConfig } = useNodeConfig(id, data.config);

  // Mode configuration
  const mode = useMemo(() => {
    const modeConfig = data.config?.mode;
    if (getRuleMode(modeConfig)) return 'by_rule' as const;
    if (getLineCountMode(modeConfig)) return 'by_line_count' as const;
    return 'by_line_count' as const;
  }, [data.config?.mode]);

  const [maxLinesPerPart, setMaxLinesPerPart] = useState(() => {
    const lineCountMode = getLineCountMode(data.config?.mode);
    if (lineCountMode) {
      return Number(lineCountMode.max_lines_per_part ?? 200);
    }
    return 200;
  });

  // Rule settings
  const [keywords, setKeywords] = useState(() => {
    const rule = getHeadingKeywordsRule(data.config?.mode);
    if (rule) {
      const keywordsValue = rule.keywords;
      if (Array.isArray(keywordsValue)) {
        return keywordsValue.join(', ');
      }
    }
    return '';
  });

  const [requirePrefix, setRequirePrefix] = useState(() => {
    const rule = getHeadingKeywordsRule(data.config?.mode);
    if (rule) {
      return rule.require_prefix ?? '';
    }
    return '';
  });

  // General options
  const [removeEmptyLines, setRemoveEmptyLines] = useState(
    Boolean(data.config?.remove_empty_lines ?? false)
  );

  const [lineNumbersEnabled, setLineNumbersEnabled] = useState(() => {
    const lineNums = getLineNumbersConfig(data.config?.line_numbers);
    if (lineNums) {
      return Boolean(lineNums.enabled ?? false);
    }
    return false;
  });

  const [lineNumbersTemplate, setLineNumbersTemplate] = useState(() => {
    const lineNums = getLineNumbersConfig(data.config?.line_numbers);
    if (lineNums) {
      return String(lineNums.template ?? '{line}: ');
    }
    return '{line}: ';
  });

  const [ruleOnlyKeepMatched, setRuleOnlyKeepMatched] = useState(
    Boolean(data.config?.rule_only_keep_matched_ranges ?? false)
  );


  // Sync with external config changes
  useEffect(() => {
    const lineCountMode = getLineCountMode(data.config?.mode);
    if (lineCountMode) {
      setMaxLinesPerPart(Number(lineCountMode.max_lines_per_part ?? 200));
    }
  }, [data.config?.mode]);

  useEffect(() => {
    setRemoveEmptyLines(Boolean(data.config?.remove_empty_lines ?? false));
  }, [data.config?.remove_empty_lines]);

  useEffect(() => {
    setRuleOnlyKeepMatched(Boolean(data.config?.rule_only_keep_matched_ranges ?? false));
  }, [data.config?.rule_only_keep_matched_ranges]);

  useEffect(() => {
    const lineNums = getLineNumbersConfig(data.config?.line_numbers);
    if (lineNums) {
      setLineNumbersEnabled(Boolean(lineNums.enabled ?? false));
      setLineNumbersTemplate(String(lineNums.template ?? '{line}: '));
    }
  }, [data.config?.line_numbers]);


  const onModeChange = useCallback(
    (next: 'by_line_count' | 'by_rule') => {
      if (next === 'by_line_count') {
        updateConfig({
          mode: {
            by_line_count: {
              max_lines_per_part: maxLinesPerPart,
            },
          },
        });
      } else {
        updateConfig({
          mode: {
            by_rule: {
              rule: {
                heading_keywords: {
                  keywords: keywords
                    .split(',')
                    .map((k) => k.trim())
                    .filter((k) => k),
                  require_prefix: requirePrefix || null,
                },
              },
            },
          },
        });
      }
    },
    [updateConfig, maxLinesPerPart, keywords, requirePrefix]
  );

  const onMaxLinesChange = useCallback(
    (next: number) => {
      const clamped = Math.max(1, Math.min(10000, next));
      setMaxLinesPerPart(clamped);
      if (mode === 'by_line_count') {
        updateConfig({
          mode: {
            by_line_count: {
              max_lines_per_part: clamped,
            },
          },
        });
      }
    },
    [updateConfig, mode]
  );

  const onKeywordsChange = useCallback(
    (next: string) => {
      setKeywords(next);
      if (mode === 'by_rule') {
        updateConfig({
          mode: {
            by_rule: {
              rule: {
                heading_keywords: {
                  keywords: next
                    .split(',')
                    .map((k) => k.trim())
                    .filter((k) => k),
                  require_prefix: requirePrefix || null,
                },
              },
            },
          },
        });
      }
    },
    [updateConfig, mode, requirePrefix]
  );

  const onRequirePrefixChange = useCallback(
    (next: string) => {
      setRequirePrefix(next);
      if (mode === 'by_rule') {
        updateConfig({
          mode: {
            by_rule: {
              rule: {
                heading_keywords: {
                  keywords: keywords
                    .split(',')
                    .map((k) => k.trim())
                    .filter((k) => k),
                  require_prefix: next || null,
                },
              },
            },
          },
        });
      }
    },
    [updateConfig, mode, keywords]
  );

  const onRemoveEmptyLinesChange = useCallback(
    (next: boolean) => {
      setRemoveEmptyLines(next);
      updateConfig({ remove_empty_lines: next });
    },
    [updateConfig]
  );

  const onLineNumbersEnabledChange = useCallback(
    (next: boolean) => {
      setLineNumbersEnabled(next);
      updateConfig({
        line_numbers: {
          enabled: next,
          template: lineNumbersTemplate,
          pad_width: null,
          pad_char: '0',
        },
      });
    },
    [updateConfig, lineNumbersTemplate]
  );

  const onLineNumbersTemplateChange = useCallback(
    (next: string) => {
      setLineNumbersTemplate(next);
      updateConfig({
        line_numbers: {
          enabled: lineNumbersEnabled,
          template: next,
          pad_width: null,
          pad_char: '0',
        },
      });
    },
    [updateConfig, lineNumbersEnabled]
  );

  const onRuleOnlyKeepMatchedChange = useCallback(
    (next: boolean) => {
      setRuleOnlyKeepMatched(next);
      updateConfig({ rule_only_keep_matched_ranges: next });
    },
    [updateConfig]
  );


  return {
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
  };
}
