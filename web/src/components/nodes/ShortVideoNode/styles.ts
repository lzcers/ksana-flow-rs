export const shortVideoNodeStyles = {
  wrapperClass: 'flex flex-col',
  container: 'p-4 flex-1 flex flex-col items-center justify-center min-h-0 bg-zinc-950/50',
  card: 'text-center space-y-2',
  iconWrap: 'w-12 h-12 rounded-xl bg-zinc-900 flex items-center justify-center mx-auto border border-zinc-800 shadow-sm',
  title: 'text-sm font-medium text-zinc-200',
  subtitle: 'text-xs text-zinc-500 mt-1',
  button:
    'px-4 py-1.5 bg-zinc-800/50 hover:bg-zinc-700/80 text-zinc-300 hover:text-zinc-100 text-xs rounded-md transition-colors border border-zinc-700/30 hover:border-zinc-600',
  headerActions: 'flex items-center gap-1',
  headerButton: 'text-zinc-400 hover:text-zinc-200 transition-colors p-1 rounded hover:bg-zinc-800',
} as const;
