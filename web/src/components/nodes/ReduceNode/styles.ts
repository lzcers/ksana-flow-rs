export const reduceNodeStyles = {
  container: 'w-full h-full bg-zinc-900 text-zinc-100 p-4 flex flex-col gap-3',
  title: 'text-xs font-semibold tracking-wide text-zinc-200',
  fieldLabel: 'text-[10px] uppercase tracking-wide text-zinc-500',
  select:
    'w-full bg-black border border-zinc-800 text-zinc-200 rounded-md px-2 py-2 text-xs focus:outline-none focus:ring-1 focus:ring-blue-500',
  input:
    'w-full bg-black border border-zinc-800 text-zinc-200 rounded-md px-2 py-2 text-xs focus:outline-none focus:ring-1 focus:ring-blue-500',
  hint: 'text-[10px] text-zinc-500 leading-snug',
  preview: 'text-[11px] text-zinc-400 truncate',
} as const;

