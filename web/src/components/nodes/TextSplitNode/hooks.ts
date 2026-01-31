import { useCallback, useEffect, useMemo, useState } from 'react';
import { useNodeConfig } from '../shared/hooks/useNodeConfig';
import type { NodeData } from '../../../model/types';

export function useTextSplitNodeController(id: string, data: NodeData) {
  const { updateConfig } = useNodeConfig(id, data.config);

  // Mode configuration
  const mode = useMemo(() => {
    const modeConfig = data.config?.mode;
    if (modeConfig && typeof modeConfig === 'object') {
      if ('by_rule' in modeConfig) return 'by_rule' as const;
      if ('by_line_count' in modeConfig) return 'by_line_count' as const;
    }
    return 'by_line_count' as const;
  }, [data.config?.mode]);

  const [maxLinesPerPart, setMaxLinesPerPart] = useState(() => {
    const modeConfig = data.config?.mode;
    if (modeConfig && typeof modeConfig === 'object' && 'by_line_count' in modeConfig) {
      return Number(modeConfig.by_line_count?.max_lines_per_part ?? 200);
    }
    return 200;
  });

  // Rule settings
  const [keywords, setKeywords] = useState(() => {
    const modeConfig = data.config?.mode;
    if (modeConfig && typeof modeConfig === 'object' && 'by_rule' in modeConfig) {
      const rule = modeConfig.by_rule?.rule;
      if (rule && typeof rule === 'object' && 'heading_keywords' in rule) {
        const kws = rule.heading_keywords?.keywords;
        if (Array.isArray(kws)) {
          return kws.join(', ');
        }
      }
    }
    return '';
  });

  const [requirePrefix, setRequirePrefix] = useState(() => {
    const modeConfig = data.config?.mode;
    if (modeConfig && typeof modeConfig === 'object' && 'by_rule' in modeConfig) {
      const rule = modeConfig.by_rule?.rule;
      if (rule && typeof rule === 'object' && 'heading_keywords' in rule) {
        return rule.heading_keywords?.require_prefix ?? '';
      }
    }
    return '';
  });

  // General options
  const [removeEmptyLines, setRemoveEmptyLines] = useState(
    Boolean(data.config?.remove_empty_lines ?? false)
  );

  const [lineNumbersEnabled, setLineNumbersEnabled] = useState(() => {
    const lineNums = data.config?.line_numbers;
    if (lineNums && typeof lineNums === 'object') {
      return Boolean(lineNums.enabled ?? false);
    }
    return false;
  });

  const [lineNumbersTemplate, setLineNumbersTemplate] = useState(() => {
    const lineNums = data.config?.line_numbers;
    if (lineNums && typeof lineNums === 'object') {
      return String(lineNums.template ?? '{line}: ');
    }
    return '{line}: ';
  });

  const [ruleOnlyKeepMatched, setRuleOnlyKeepMatched] = useState(
    Boolean(data.config?.rule_only_keep_matched_ranges ?? false)
  );

  // Output
  const [outputText, setOutputText] = useState<string>('');

  // Sync with external config changes
  useEffect(() => {
    const modeConfig = data.config?.mode;
    if (modeConfig && typeof modeConfig === 'object') {
      if ('by_line_count' in modeConfig) {
        setMaxLinesPerPart(Number(modeConfig.by_line_count?.max_lines_per_part ?? 200));
      }
    }
  }, [data.config?.mode]);

  useEffect(() => {
    setRemoveEmptyLines(Boolean(data.config?.remove_empty_lines ?? false));
  }, [data.config?.remove_empty_lines]);

  useEffect(() => {
    setRuleOnlyKeepMatched(Boolean(data.config?.rule_only_keep_matched_ranges ?? false));
  }, [data.config?.rule_only_keep_matched_ranges]);

  useEffect(() => {
    const lineNums = data.config?.line_numbers;
    if (lineNums && typeof lineNums === 'object') {
      setLineNumbersEnabled(Boolean(lineNums.enabled ?? false));
      setLineNumbersTemplate(String(lineNums.template ?? '{line}: '));
    }
  }, [data.config?.line_numbers]);

  // Update output when data changes
  useEffect(() => {
    let nextText: string | null = null;
    if (typeof data.config?.output === 'string') {
      nextText = data.config.output;
    } else if (typeof data.outputs?.output === 'string') {
      nextText = data.outputs.output;
    } else if (data.lastMessage && typeof data.lastMessage === 'string') {
      nextText = data.lastMessage;
    }

    if (nextText !== null) {
      setOutputText(nextText);
    }
  }, [data.config?.output, data.outputs?.output, data.lastMessage]);

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

  const onOutputChange = useCallback((next: string) => {
    setOutputText(next);
  }, []);

  const onOutputBlur = useCallback(() => {
    updateConfig({ output: outputText });
  }, [outputText, updateConfig]);

  return {
    mode,
    maxLinesPerPart,
    keywords,
    requirePrefix,
    removeEmptyLines,
    lineNumbersEnabled,
    lineNumbersTemplate,
    ruleOnlyKeepMatched,
    outputText,
    onModeChange,
    onMaxLinesChange,
    onKeywordsChange,
    onRequirePrefixChange,
    onRemoveEmptyLinesChange,
    onLineNumbersEnabledChange,
    onLineNumbersTemplateChange,
    onRuleOnlyKeepMatchedChange,
    onOutputChange,
    onOutputBlur,
  };
}
