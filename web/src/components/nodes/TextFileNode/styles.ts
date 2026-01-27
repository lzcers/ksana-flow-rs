export const textFileNodeStyles = {
  container: 'h-full p-4 flex flex-col items-center justify-center gap-3',
  fileInfo: 'flex flex-col items-center gap-1 w-full',
  fileRow: 'flex items-center gap-2 text-zinc-200',
  fileName: 'text-sm truncate max-w-[150px]',
  fileSize: 'text-xs text-zinc-500',
  changeButton: 'mt-2 text-xs text-zinc-400 hover:text-zinc-200 underline',
  uploadButton:
    'flex items-center gap-2 px-3 py-2 bg-zinc-800/50 hover:bg-zinc-700/80 rounded text-zinc-300 hover:text-zinc-100 text-sm transition-all border border-zinc-700/30 hover:border-zinc-600 shadow-sm',
  error: 'text-xs text-red-400 mt-1',
} as const;
