export const mapNodeStyles = {
  expandedContainer:
    'bg-zinc-900/20 backdrop-blur-sm border-2 border-dashed border-zinc-700/50 rounded-xl transition-all duration-300 relative',
  collapsedContainer:
    'bg-zinc-800/80 border-2 border-zinc-600 rounded-xl transition-all duration-300 relative shadow-lg backdrop-blur-sm',
  headerActions: 'flex items-center gap-1 mr-2',
  headerButton: 'p-1 hover:bg-zinc-800 rounded text-zinc-400 hover:text-zinc-200 transition-colors cursor-pointer',
  collapsedIcon: 'w-8 h-8 rounded-lg bg-zinc-700/50 flex items-center justify-center mb-2',
  collapsedLabel: 'text-zinc-200 font-medium text-xs uppercase tracking-wider',
  collapsedCount: 'text-[10px] text-zinc-500 mt-1',
  panel: 'mt-2 w-full px-3 pb-3',
  fieldRow: 'flex items-center justify-between gap-3',
  fieldLabel: 'text-[10px] text-zinc-500 uppercase tracking-wide',
  numberInput:
    'w-20 bg-black border border-zinc-800 text-zinc-200 rounded-md px-2 py-1 text-xs focus:outline-none focus:ring-1 focus:ring-blue-500',
  toggleButton:
    'px-2 py-1 rounded-md text-xs border border-zinc-700 bg-zinc-900/60 text-zinc-200 hover:bg-zinc-900 transition-colors cursor-pointer',
  toggleOn: 'border-blue-500/60 text-blue-300',
  statusLine: 'mt-2 text-[11px] text-zinc-400 truncate',
} as const;

