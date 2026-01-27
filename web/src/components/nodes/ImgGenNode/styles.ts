export const imgGenNodeStyles = {
  headerActions: 'relative flex items-center gap-1',
  headerButton: 'text-zinc-400 hover:text-zinc-200 transition-colors p-1 rounded hover:bg-zinc-800',
  configOverlay:
    'absolute inset-0 z-50 bg-zinc-900/95 backdrop-blur-xl border border-zinc-800 rounded-xl shadow-2xl flex flex-col',
  configHeader: 'px-3 py-2 flex items-center justify-between border-b border-zinc-800',
  configTitle: 'flex items-center gap-2 text-[10px] text-zinc-400',
  configBody: 'p-3 flex flex-col gap-2 flex-1 overflow-auto custom-scrollbar',
  configRow: 'flex items-center justify-between gap-2',
  configInline: 'flex items-center gap-2',
  configLabel: 'text-[10px] w-10 text-zinc-500 font-bold',
  select:
    'min-w-[200px] text-xs bg-zinc-800/50 hover:bg-zinc-800/80 focus:bg-zinc-900 border border-zinc-800 focus:border-zinc-700 rounded px-1 py-1 text-zinc-300 focus:text-zinc-200 focus:outline-none focus:ring-1 focus:ring-blue-500/50 focus:ring-offset-1 focus:ring-offset-zinc-900 transition-colors duration-200',
  selectSmall:
    'min-w-[120px] text-xs bg-zinc-800/50 hover:bg-zinc-800/80 focus:bg-zinc-900 border border-zinc-800 focus:border-zinc-700 rounded px-1 py-1 text-zinc-300 focus:text-zinc-200 focus:outline-none focus:ring-1 focus:ring-blue-500/50 focus:ring-offset-1 focus:ring-offset-zinc-900 transition-colors duration-200',
  selectTiny:
    'min-w-[90px] text-xs bg-zinc-800/50 hover:bg-zinc-800/80 focus:bg-zinc-900 border border-zinc-800 focus:border-zinc-700 rounded px-1 py-1 text-zinc-300 focus:text-zinc-200 focus:outline-none focus:ring-1 focus:ring-blue-500/50 focus:ring-offset-1 focus:ring-offset-zinc-900 transition-colors duration-200',
  input:
    'w-full text-xs bg-zinc-900/60 hover:bg-zinc-900/70 focus:bg-zinc-900 border border-zinc-800 focus:border-zinc-700 rounded px-2 py-1 text-zinc-300 focus:text-zinc-200 focus:outline-none focus:ring-1 focus:ring-blue-500/50 focus:ring-offset-1 focus:ring-offset-zinc-900 transition-colors duration-200 placeholder-zinc-500 shadow-inner',
  canvas: 'flex flex-col h-full relative group/imggen',
  imageAreaBase: 'relative flex-1 overflow-hidden bg-black/40 nodrag',
  image: 'absolute inset-0 w-full h-full object-cover select-none',
  emptyCenter: 'absolute inset-0 flex items-center justify-center',
  emptyText: 'text-xs text-zinc-400 px-3 text-center',
  topGradient: 'absolute inset-x-0 top-0 h-16 bg-gradient-to-b from-black/70 via-black/30 to-transparent pointer-events-none',
  topBar: 'absolute top-2 left-2 right-2 flex items-center justify-between gap-2 pointer-events-none',
  pill: 'px-2 py-1 rounded-md bg-black/40 backdrop-blur border border-zinc-800/60 text-[10px] font-bold',
  pillMuted: 'px-2 py-1 rounded-md bg-black/40 backdrop-blur border border-zinc-800/60 text-[10px] text-zinc-300/80',
  bottomGradient:
    'absolute inset-x-0 bottom-0 h-40 bg-gradient-to-t from-black/80 via-black/40 to-transparent opacity-0 group-hover/imggen:opacity-100 group-focus-within/imggen:opacity-100 transition-opacity pointer-events-none',
  bottomPanelWrap:
    'absolute left-2 right-2 bottom-2 opacity-0 group-hover/imggen:opacity-100 group-focus-within/imggen:opacity-100 transition-opacity pointer-events-none group-hover/imggen:pointer-events-auto group-focus-within/imggen:pointer-events-auto',
  bottomPanel: 'p-2 rounded-lg bg-black/60 backdrop-blur-md border border-zinc-800/70 shadow-lg shadow-black/40 cursor-auto',
  bottomHeader: 'flex items-center justify-between gap-2 mb-1',
  bottomLabel: 'text-[10px] text-zinc-400 font-bold',
  bottomRight: 'text-[10px] text-zinc-500 font-bold',
  prompt:
    'w-full text-xs nodrag nowheel bg-zinc-950/60 hover:bg-zinc-950/70 focus:bg-zinc-950 border border-zinc-800 focus:border-zinc-700 rounded resize-none nodrag focus:outline-none focus:ring-1 focus:ring-blue-500/50 focus:ring-offset-1 focus:ring-offset-black text-zinc-200 transition-colors duration-200 placeholder-zinc-500 shadow-inner',
} as const;
