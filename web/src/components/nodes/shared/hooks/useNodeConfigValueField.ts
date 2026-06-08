import { useCallback } from 'react';
import { useNodeConfig } from './useNodeConfig';
import { type CommitMode, useNodeConfigField } from './useNodeConfigField';

export const SKIP_NODE_CONFIG_UPDATE = Symbol('SKIP_NODE_CONFIG_UPDATE');

type ReadConfigValue<TDraft> = (value: unknown, defaultValue: TDraft) => TDraft;
type WriteConfigValue<TDraft> = (next: TDraft) => unknown | typeof SKIP_NODE_CONFIG_UPDATE;

export function useNodeConfigValueField<TDraft>({
  id,
  config,
  configKey,
  defaultValue,
  commitMode = 'change',
  composition = false,
  isFocused,
  read,
  write,
}: {
  id: string;
  config?: Record<string, unknown>;
  configKey: string;
  defaultValue: TDraft;
  commitMode?: CommitMode;
  composition?: boolean;
  isFocused?: () => boolean;
  read: ReadConfigValue<TDraft>;
  write: WriteConfigValue<TDraft>;
}) {
  const { updateConfig } = useNodeConfig(id, config);
  const value = read(config?.[configKey], defaultValue);

  const updateValue = useCallback(
    (next: TDraft) => {
      const storedValue = write(next);
      if (storedValue === SKIP_NODE_CONFIG_UPDATE) return;
      updateConfig({ [configKey]: storedValue });
    },
    [configKey, updateConfig, write],
  );

  return useNodeConfigField<TDraft>({
    value,
    commitMode,
    composition,
    isFocused,
    updateValue,
  });
}

export function useStringNodeConfigField({
  id,
  config,
  configKey,
  defaultValue = '',
  commitMode = 'change',
  composition = false,
  isFocused,
}: {
  id: string;
  config?: Record<string, unknown>;
  configKey: string;
  defaultValue?: string;
  commitMode?: CommitMode;
  composition?: boolean;
  isFocused?: () => boolean;
}) {
  return useNodeConfigValueField<string>({
    id,
    config,
    configKey,
    defaultValue,
    commitMode,
    composition,
    isFocused,
    read: (value, fallback) => {
      if (typeof value === 'string') return value;
      if (value == null) return fallback;
      return String(value);
    },
    write: next => next,
  });
}

export function useNumericStringNodeConfigField({
  id,
  config,
  configKey,
  defaultValue,
  commitMode = 'change',
  parse,
}: {
  id: string;
  config?: Record<string, unknown>;
  configKey: string;
  defaultValue: string;
  commitMode?: CommitMode;
  parse: (next: string) => number | undefined;
}) {
  return useNodeConfigValueField<string>({
    id,
    config,
    configKey,
    defaultValue,
    commitMode,
    read: (value, fallback) => String(value ?? fallback),
    write: next => {
      const parsed = parse(next);
      return parsed === undefined ? SKIP_NODE_CONFIG_UPDATE : parsed;
    },
  });
}

export function useBooleanNodeConfigField({
  id,
  config,
  configKey,
  defaultValue = false,
  commitMode = 'change',
}: {
  id: string;
  config?: Record<string, unknown>;
  configKey: string;
  defaultValue?: boolean;
  commitMode?: CommitMode;
}) {
  return useNodeConfigValueField<boolean>({
    id,
    config,
    configKey,
    defaultValue,
    commitMode,
    read: (value, fallback) => {
      if (typeof value === 'boolean') return value;
      if (value == null) return fallback;
      return Boolean(value);
    },
    write: next => next,
  });
}
