export const subgraphNodeStyles = {
  expandedContainer: 'bg-zinc-900/20 backdrop-blur-sm border-2 border-dashed border-zinc-700/50 rounded-xl transition-all duration-300 relative',
  collapsedContainer: 'bg-zinc-800/80 border-2 border-zinc-600 rounded-xl flex items-center justify-center transition-all duration-300 relative shadow-lg backdrop-blur-sm',
  headerActions: 'flex items-center gap-1 mr-2',
  headerButton: 'p-1 hover:bg-zinc-800 rounded text-zinc-400 hover:text-zinc-200 transition-colors cursor-pointer',
  collapsedLabel: 'text-zinc-400 font-medium text-xs uppercase tracking-wider',
  collapsedIcon: 'w-8 h-8 rounded-lg bg-zinc-700/50 flex items-center justify-center mb-2',
  collapsedCount: 'text-[10px] text-zinc-500 mt-1',
} as const;
