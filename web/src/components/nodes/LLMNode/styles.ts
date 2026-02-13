export const llmNodeStyles = {
    headerActions: "relative flex items-center gap-1",
    headerButton: "text-zinc-400 hover:text-zinc-200 transition-colors p-1 rounded hover:bg-zinc-800",
    configOverlay: "absolute inset-0 z-50 bg-zinc-900/95 backdrop-blur-xl border border-zinc-800 rounded-xl shadow-2xl flex flex-col",
    configHeader: "px-3 py-2 flex items-center justify-between border-b border-zinc-800",
    configTitleRow: "flex items-center gap-2 text-[10px] text-zinc-400",
    configBody: "p-3 grid grid-cols-12 gap-2 flex-1 overflow-auto grid-rows-[auto_1fr] min-auto custom-scrollbar",
    configRow: "col-span-12 flex items-center justify-between",
    configLabel: "text-[10px] text-zinc-500 font-bold",
    select: "min-w-[160px] text-xs bg-zinc-800/50 hover:bg-zinc-800/80 focus:bg-zinc-900 border border-zinc-800 focus:border-zinc-700 rounded px-1 py-1 text-zinc-300 focus:text-zinc-200 focus:outline-none focus:ring-1 focus:ring-blue-500/50 focus:ring-offset-1 focus:ring-offset-zinc-900 transition-colors duration-200",
    checkbox: "rounded border-zinc-800 bg-black text-blue-500 focus:ring-blue-500 cursor-pointer",
    promptLabelRow: "flex items-center justify-between text-[10px] text-zinc-500 font-bold mb-1",
    promptTextarea:
        "flex-1 text-xs nowheel bg-zinc-900/60 hover:bg-zinc-900/70 focus:bg-zinc-900 border border-zinc-800 focus:border-zinc-700 rounded resize-none nodrag focus:outline-none focus:ring-1 focus:ring-blue-500/50 focus:ring-offset-1 focus:ring-offset-zinc-900 text-zinc-300 focus:text-zinc-200 transition-colors duration-200 placeholder-zinc-500 shadow-inner",
    fullscreenBody: "w-full h-full flex p-4 text-zinc-200 text-sm overflow-auto custom-scrollbar bg-black justify-center items-center",
    fullscreenTextarea: "w-full h-full p-4 bg-black resize-none focus:outline-none text-zinc-200 font-mono",
    outputPanel: "flex flex-col flex-1 p-3 bg-zinc-950/50 border-b border-zinc-800 overflow-auto custom-scrollbar",
    outputHeader: "text-[10px] text-zinc-500 font-bold mb-1 flex items-center justify-between",
    outputMode: "text-[10px] opacity-50",
    markdownBox:
        "w-full flex-1 text-xs bg-zinc-900/60 border border-zinc-800 rounded overflow-auto overflow-x-hidden nodrag nowheel text-zinc-200 custom-scrollbar",
    rawTextarea:
        "w-full flex-1 p-2 text-xs bg-zinc-900/60 hover:bg-zinc-900/70 focus:bg-zinc-900 border border-zinc-800 focus:border-zinc-700 rounded resize-none nodrag nowheel focus:outline-none focus:ring-1 focus:ring-blue-500/50 focus:ring-offset-1 focus:ring-offset-zinc-900 text-zinc-300 focus:text-zinc-200 transition-colors duration-200 placeholder-zinc-500 shadow-inner",
    userPromptSection: "px-3 pb-3 space-y-2 pt-2",
    userPromptLabel: "text-[10px] text-zinc-500 font-bold block mb-1",
    userPromptTextarea:
        "w-full flex-1 text-xs bg-zinc-900/60 hover:bg-zinc-900/70 focus:bg-zinc-900 border overflow-x-hidden max-h-[200px] border-zinc-800 focus:border-zinc-700 rounded resize-none nodrag nowheel focus:outline-none focus:ring-1 focus:ring-blue-500/50 focus:ring-offset-1 focus:ring-offset-zinc-900 text-zinc-300 focus:text-zinc-200 transition-colors duration-200 placeholder-zinc-500 shadow-inner",
} as const;
