import { nodeFormStyles } from '../shared/formStyles';

export const timerNodeStyles = {
  section: nodeFormStyles.section,
  label: nodeFormStyles.label,
  input: `${nodeFormStyles.input} font-mono`,
  hint: 'text-[9px] text-zinc-600 mt-1',
} as const;
