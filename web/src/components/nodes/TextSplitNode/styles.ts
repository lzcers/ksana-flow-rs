export const textSplitNodeStyles = {
  container: 'p-3 flex flex-col h-full gap-3',
  header: 'text-[10px] text-zinc-500 font-bold uppercase tracking-wider',
  section: 'flex flex-col gap-2',
  sectionTitle: 'text-[10px] text-zinc-500 font-bold',
  configRow: 'flex items-center justify-between gap-2',
  configLabel: 'text-[10px] text-zinc-500',
  select:
    'min-w-[140px] text-xs bg-zinc-800/50 hover:bg-zinc-800/80 focus:bg-black border border-transparent focus:border-zinc-800 rounded px-2 py-1 text-zinc-300 focus:text-zinc-200 focus:outline-none focus:ring-1 focus:ring-blue-500/50 transition-colors duration-200',
  input:
    'flex-1 min-w-0 p-1.5 text-xs bg-zinc-800/50 hover:bg-zinc-800/80 focus:bg-black border border-transparent focus:border-zinc-800 rounded nodrag focus:outline-none focus:ring-1 focus:ring-blue-500/50 text-zinc-300 focus:text-zinc-200 transition-colors duration-200 placeholder-zinc-600',
  numberInput:
    'w-20 p-1.5 text-xs bg-zinc-800/50 hover:bg-zinc-800/80 focus:bg-black border border-transparent focus:border-zinc-800 rounded nodrag focus:outline-none focus:ring-1 focus:ring-blue-500/50 text-zinc-300 focus:text-zinc-200 transition-colors duration-200 text-center',
  checkbox: 'rounded border-zinc-800 bg-black text-blue-500 focus:ring-blue-500 cursor-pointer w-4 h-4',
  tagInput:
    'w-full p-1.5 text-xs bg-zinc-800/50 hover:bg-zinc-800/80 focus:bg-black border border-transparent focus:border-zinc-800 rounded nodrag focus:outline-none focus:ring-1 focus:ring-blue-500/50 text-zinc-300 focus:text-zinc-200 transition-colors duration-200 placeholder-zinc-600',
  hint: 'text-[10px] text-zinc-600 italic',
  outputSection: 'flex flex-col gap-1 flex-1',
  outputLabel: 'text-[10px] text-zinc-500 font-bold flex items-center justify-between',
  outputTextarea:
    'w-full flex-1 text-xs bg-zinc-900/60 hover:bg-zinc-900/70 focus:bg-zinc-900 border border-zinc-800 focus:border-zinc-700 rounded resize-none nodrag focus:outline-none focus:ring-1 focus:ring-blue-500/50 focus:ring-offset-1 focus:ring-offset-zinc-900 text-zinc-300 focus:text-zinc-200 transition-colors duration-200 placeholder-zinc-500 shadow-inner min-h-[120px]',
} as const;
