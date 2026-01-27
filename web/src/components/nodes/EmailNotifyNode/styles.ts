import { nodeFormStyles } from '../shared/formStyles';

export const emailNotifyNodeStyles = {
  section: nodeFormStyles.section,
  label: nodeFormStyles.label,
  subjectInput: nodeFormStyles.inputPlain,
  bodyTextarea:
    'w-full text-[10px] p-1.5 bg-zinc-800/50 hover:bg-zinc-800/80 focus:bg-black border border-transparent focus:border-zinc-800 rounded focus:ring-1 focus:ring-purple-500/50 resize-none outline-none nodrag text-zinc-300 focus:text-zinc-200 transition-colors duration-200 placeholder-zinc-600',
} as const;
