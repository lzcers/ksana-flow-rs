export const nodeFormStyles = {
  section: 'px-3 pb-3 space-y-2 border-t border-zinc-800 pt-2',
  label: 'text-[10px] text-zinc-500 font-bold block mb-1',
  input:
    'w-full text-[10px] p-1.5 bg-zinc-800/50 hover:bg-zinc-800/80 focus:bg-black border border-transparent focus:border-zinc-800 rounded focus:ring-1 focus:ring-purple-500/50 outline-none nodrag text-zinc-300 focus:text-zinc-200 transition-colors duration-200 placeholder-zinc-600',
  inputPlain:
    'w-full text-[10px] p-1.5 bg-black border border-zinc-800 rounded focus:ring-1 focus:ring-purple-500/50 outline-none nodrag text-zinc-300 placeholder:text-zinc-700',
  select:
    'w-full text-[10px] p-1.5 bg-zinc-800/50 hover:bg-zinc-800/80 focus:bg-black border border-transparent focus:border-zinc-800 rounded focus:ring-1 focus:ring-purple-500/50 outline-none nodrag text-zinc-300 focus:text-zinc-200 transition-colors duration-200 placeholder-zinc-600',
  textarea:
    'w-full text-[10px] p-1.5 bg-zinc-800/50 hover:bg-zinc-800/80 focus:bg-black border border-transparent focus:border-zinc-800 rounded focus:ring-1 focus:ring-purple-500/50 resize-none outline-none nodrag text-zinc-300 focus:text-zinc-200 transition-colors duration-200 placeholder-zinc-600',
} as const;
