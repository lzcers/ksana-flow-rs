import { nodeFormStyles } from '../shared/formStyles';

export const sourceNodeStyles = {
  section: nodeFormStyles.section,
  label: nodeFormStyles.label,
  input: nodeFormStyles.input,
  select: nodeFormStyles.select,
  inputPlain: nodeFormStyles.inputPlain,
  grid2: 'grid grid-cols-2 gap-2',
} as const;
