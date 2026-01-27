import { nodeFormStyles } from '../shared/formStyles';

export const backtesterNodeStyles = {
  section: nodeFormStyles.section,
  label: nodeFormStyles.label,
  initMoneyInput: nodeFormStyles.inputPlain,
  feeRateInput: nodeFormStyles.input,
} as const;
