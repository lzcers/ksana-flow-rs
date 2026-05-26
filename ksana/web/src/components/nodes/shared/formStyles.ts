export const nodeFormStyles = {
    container: "px-3 pb-3 pt-2",
    section: "px-3 pb-3 space-y-2 border-t border-zinc-800 pt-2",
    stack: "flex flex-col gap-2",
    field: "flex flex-col gap-1",
    inlineField: "flex items-center justify-between gap-2",
    grid2: "grid grid-cols-2 gap-2",
    label: "text-[10px] text-zinc-500 font-bold block mb-1",
    inlineLabel: "text-[10px] text-zinc-500",
    title: "text-xs text-zinc-500",
    hint: "text-[10px] text-zinc-500",
    input: "w-full text-[10px] p-1.5 bg-zinc-800/50 hover:bg-zinc-800/80 focus:bg-black border border-transparent focus:border-zinc-800 rounded focus:ring-1 focus:ring-purple-500/50 outline-none nodrag text-zinc-300 focus:text-zinc-200 transition-colors duration-200 placeholder-zinc-600",
    inputPlain:
        "w-full text-[10px] p-1.5 bg-black border border-zinc-800 rounded focus:ring-1 focus:ring-purple-500/50 outline-none  text-zinc-300 placeholder:text-zinc-700",
    select: "w-full text-[10px] p-1.5 bg-zinc-800/50 hover:bg-zinc-800/80 focus:bg-black border border-transparent focus:border-zinc-800 rounded focus:ring-1 focus:ring-purple-500/50 outline-none nodrag text-zinc-300 focus:text-zinc-200 transition-colors duration-200 placeholder-zinc-600",
    selectPlain:
        "w-full text-[10px] p-1.5 bg-black border border-zinc-800 rounded focus:ring-1 focus:ring-purple-500/50 outline-none text-zinc-300",
    checkbox: "rounded border-zinc-800 bg-black text-blue-500 focus:ring-blue-500 cursor-pointer w-4 h-4 shrink-0",
    textarea:
        "w-full text-[10px] p-1.5 bg-zinc-800/50 hover:bg-zinc-800/80 focus:bg-black border border-transparent focus:border-zinc-800 rounded focus:ring-1 focus:ring-purple-500/50 resize-none outline-none nodrag text-zinc-300 focus:text-zinc-200 transition-colors duration-200 placeholder-zinc-600",
} as const;
