export const nodeWrapperStyles = {
    root: "relative group",
    header: "absolute -top-9 left-0 w-full h-9 flex items-center justify-between transition-all duration-300 z-10",
    headerLeft: "flex items-center gap-2",
    headerDot: "w-2 h-2 rounded-full bg-zinc-400 shadow-[0_0_8px_rgba(161,161,170,0.4)]",
    headerLabel: "text-sm font-bold text-zinc-300 tracking-wide drop-shadow-sm cursor-text",
    headerRight: "flex items-center gap-2",
    labelInput:
        "h-7 px-2 text-sm font-semibold bg-black border border-zinc-800 text-zinc-200 rounded-md focus:outline-none focus:ring-1 focus:ring-blue-500 placeholder:text-zinc-700 shadow-sm w-40",
    runButton:
        "cursor-pointer group/run flex items-center justify-center w-7 h-7 backdrop-blur-xl bg-zinc-800/50 hover:bg-zinc-700/80 text-zinc-400 hover:text-zinc-200 rounded-full border border-zinc-700/30 hover:border-zinc-600 shadow-sm hover:shadow-md transition-all duration-300",
    cardBase: "w-full h-full flex-1 bg-zinc-900 border duration-300 relative rounded-xl",
    cardSelected: "border-blue-500/50 shadow-[0_0_20px_rgba(59,130,246,0.15)] ring-1 ring-blue-500/20",
    cardIdle: "border-zinc-800 hover:border-zinc-700 shadow-lg shadow-black/40",
    resizeControlBase: "bg-transparent! border-none! z-50",
    resizeControlHidden: "opacity-0 group-hover:opacity-100 transition-opacity duration-300",
    resizeHandle: "absolute -bottom-3 -right-3 p-2 cursor-nwse-resize group/resize",
    resizeIcon: "text-zinc-600 group-hover/resize:text-blue-500 transition-colors",
    handleTargetBase:
        "w-3.5! h-3.5! bg-zinc-950! border-[1.5px]! border-zinc-500! hover:border-blue-500! hover:bg-zinc-900! rounded-full! transition-all duration-200 z-50",
    handleSourceBase: "w-3.5! h-3.5! bg-zinc-400! hover:bg-blue-400! border-[1.5px]! border-zinc-950! rounded-full! transition-all duration-200 z-50",
    contentArea: "w-full h-full overflow-hidden",
    errorToast:
        "absolute bottom-3 left-3 right-8 text-[10px] text-red-400 bg-red-950/80 p-1.5 rounded-md border border-red-900/50 truncate backdrop-blur-sm",
} as const;
