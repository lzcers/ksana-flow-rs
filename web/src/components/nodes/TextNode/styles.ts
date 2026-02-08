export const textNodeStyles = {
  wrapperClass: 'flex flex-col',
  container: 'p-2 h-full flex flex-col',
  headerRow: 'text-xs text-zinc-500 mb-1 flex items-center justify-between',
  modeHint: 'text-[10px] opacity-50',
  previewBox:
    'w-full flex-1 text-xs bg-zinc-900/60 border border-zinc-800 rounded shadow-inner overflow-y-auto overflow-x-hidden nodrag nowheel text-zinc-200 custom-scrollbar',
  editTextarea:
    'w-full flex-1 p-2 text-xs bg-zinc-900/60 hover:bg-zinc-900/70 focus:bg-zinc-900 border border-zinc-800 focus:border-zinc-700 rounded resize-none nodrag nowheel focus:outline-none focus:ring-1 focus:ring-blue-500/50 focus:ring-offset-1 focus:ring-offset-zinc-900 text-zinc-300 focus:text-zinc-200 transition-colors duration-200 placeholder-zinc-500 shadow-inner',
  fullscreenContainer:
    'w-full h-full flex p-4 text-zinc-200 text-sm overflow-auto custom-scrollbar bg-zinc-900/70 justify-center items-center',
  fullscreenTextarea: 'w-full h-full p-4 bg-black resize-none focus:outline-none text-zinc-200 font-mono',
  headerActions: 'flex items-center gap-1',
  headerButton: 'text-zinc-400 hover:text-zinc-200 transition-colors p-1 rounded hover:bg-zinc-800',
} as const;
