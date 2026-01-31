export const subgraphNodeStyles = {
  expandedContainer: 'bg-zinc-900/20 backdrop-blur-sm border-2 border-dashed border-zinc-700/50 rounded-xl transition-all duration-300 relative',
  collapsedContainer: 'bg-zinc-900 border border-zinc-700 rounded-xl flex items-center justify-center transition-all duration-300 relative shadow-lg',
  headerActions: 'flex items-center gap-1 mr-2',
  headerButton: 'p-1 hover:bg-zinc-800 rounded text-zinc-400 hover:text-zinc-200 transition-colors cursor-pointer',
  collapsedLabel: 'text-zinc-500 font-medium text-xs uppercase tracking-wider',
} as const;
