import React from "react";
import { Position, type NodeProps } from "@xyflow/react";
import { Settings, X } from "lucide-react";
import type { NodeData } from "@/model/workflow/types";
import { NodeWrapper } from "../shared/NodeWrapper";
import { FullScreenModal } from "../../ui/FullScreenModal";
import { imgGenNodeStyles } from "./styles";

const TARGET_HANDLES = [Position.Left, Position.Top, Position.Bottom];
const SOURCE_HANDLES = [Position.Right, Position.Top, Position.Bottom];

export function ImgGenNodeView({
    id,
    type,
    data,
    selected,
    width,
    height,
    minWidth,
    minHeight,
    parsedAspectRatio,
    isConfigOpen,
    setIsConfigOpen,
    isPreviewOpen,
    setIsPreviewOpen,
    promptRef,
    prompt,
    onPromptChange,
    onPromptCompositionStart,
    onPromptCompositionEnd,
    model,
    aspectRatio,
    imageSize,
    inputImageFileId,
    outputRaw,
    imageSrc,
    onModelChange,
    onAspectRatioChange,
    onImageSizeChange,
    onInputImageFileIdChange,
    onOpenPreview,
}: NodeProps & {
    data: NodeData;
} & {
    minWidth: number;
    minHeight: number;
    parsedAspectRatio?: number;
    isConfigOpen: boolean;
    setIsConfigOpen: React.Dispatch<React.SetStateAction<boolean>>;
    isPreviewOpen: boolean;
    setIsPreviewOpen: React.Dispatch<React.SetStateAction<boolean>>;
    promptRef: React.RefObject<HTMLTextAreaElement | null>;
    prompt: string;
    onPromptChange: (next: string) => void;
    onPromptCompositionStart?: () => void;
    onPromptCompositionEnd?: (next: string) => void;
    model?: string;
    aspectRatio: string;
    imageSize: string;
    inputImageFileId: string;
    outputRaw: string;
    imageSrc?: string;
    onModelChange: (next: string) => void;
    onAspectRatioChange: (next: string) => void;
    onImageSizeChange: (next: string) => void;
    onInputImageFileIdChange: (next: string) => void;
    onOpenPreview: (e: React.MouseEvent) => void;
}) {
    const headerActions = (
        <div className={imgGenNodeStyles.headerActions}>
            <button
                onClick={e => {
                    e.stopPropagation();
                    setIsConfigOpen(v => !v);
                }}
                className={imgGenNodeStyles.headerButton}
                title="设置"
            >
                <Settings size={12} />
            </button>
        </div>
    );

    return (
        <>
            <FullScreenModal isOpen={isPreviewOpen} onClose={() => setIsPreviewOpen(false)} title="图像预览" className="bg-black">
                {imageSrc && (
                    <div className="absolute inset-0 p-6">
                        <img src={imageSrc} alt="preview" className="w-full h-full object-contain select-none" draggable={false} />
                    </div>
                )}
            </FullScreenModal>

            <NodeWrapper
                id={id}
                type={type}
                data={data}
                selected={selected}
                minWidth={minWidth}
                minHeight={minHeight}
                keepAspectRatio={Boolean(parsedAspectRatio)}
                style={{ width, height }}
                targetHandles={TARGET_HANDLES}
                sourceHandles={SOURCE_HANDLES}
                headerActions={headerActions}
            >
                <div className={imgGenNodeStyles.canvas}>
                    {isConfigOpen && (
                        <div className={imgGenNodeStyles.configOverlay}>
                            <div className={imgGenNodeStyles.configHeader}>
                                <div className={imgGenNodeStyles.configTitle}>
                                    <span>图像生成设置</span>
                                </div>
                                <button
                                    onClick={e => {
                                        e.stopPropagation();
                                        setIsConfigOpen(false);
                                    }}
                                    className={imgGenNodeStyles.headerButton}
                                    title="关闭"
                                >
                                    <X size={12} />
                                </button>
                            </div>
                            <div className={imgGenNodeStyles.configBody}>
                                <div className={imgGenNodeStyles.configRow}>
                                    <div className={imgGenNodeStyles.configInline}>
                                        <span className={imgGenNodeStyles.configLabel}>Model</span>
                                        <select className={imgGenNodeStyles.select} value={model} onChange={e => onModelChange(e.target.value)}>
                                            <option value="black-forest-labs/flux.2-klein-4b">black-forest-labs/flux.2-klein-4b</option>
                                            <option value="google/gemini-3-pro-image-preview">Gemini 3 Pro Image Preview</option>
                                        </select>
                                    </div>
                                </div>
                                <div className="flex flex-col gap-2">
                                    <div className={imgGenNodeStyles.configInline}>
                                        <span className={imgGenNodeStyles.configLabel}>Aspect</span>
                                        <select
                                            className={imgGenNodeStyles.selectSmall}
                                            value={aspectRatio}
                                            onChange={e => onAspectRatioChange(e.target.value)}
                                        >
                                            <option value="1:1">1:1</option>
                                            <option value="16:9">16:9</option>
                                            <option value="9:16">9:16</option>
                                            <option value="4:3">4:3</option>
                                            <option value="3:4">3:4</option>
                                        </select>
                                    </div>
                                    <div className={imgGenNodeStyles.configInline}>
                                        <span className={imgGenNodeStyles.configLabel}>Size</span>
                                        <select
                                            className={imgGenNodeStyles.selectTiny}
                                            value={imageSize}
                                            onChange={e => onImageSizeChange(e.target.value)}
                                        >
                                            <option value="1K">1K</option>
                                            <option value="2K">2K</option>
                                            <option value="4K">4K</option>
                                        </select>
                                    </div>
                                </div>
                                <div className="flex flex-col gap-1">
                                    <span className="text-[10px] text-zinc-500 font-bold">Input Image File ID (optional)</span>
                                    <input
                                        value={inputImageFileId}
                                        onChange={e => onInputImageFileIdChange(e.target.value)}
                                        onKeyDown={e => e.stopPropagation()}
                                        className={imgGenNodeStyles.input}
                                        placeholder="uploaded_files.id"
                                    />
                                </div>
                            </div>
                        </div>
                    )}

                    <div
                        className={`${imgGenNodeStyles.imageAreaBase} ${imageSrc ? "cursor-zoom-in" : "cursor-default"}`}
                        onDoubleClick={onOpenPreview}
                        title={imageSrc ? "双击全屏预览" : undefined}
                    >
                        {imageSrc && data.status === "completed" ? (
                            <img src={imageSrc} alt="generated" className={imgGenNodeStyles.image} draggable={false} />
                        ) : (
                            <div className={imgGenNodeStyles.emptyCenter}>
                                <div className={imgGenNodeStyles.emptyText}>{data.status === "running" ? "正在生成图像…" : "暂无图像输出"}</div>
                            </div>
                        )}

                        <div className={imgGenNodeStyles.topGradient} />
                        <div className={imgGenNodeStyles.topBar}>
                            <div className={`${imgGenNodeStyles.pill} text-zinc-200/90`}>图像输出</div>
                            <div className={imgGenNodeStyles.pillMuted}>
                                {data.status === "running" ? "Generating" : imageSrc ? "Completed" : "Empty"}
                            </div>
                        </div>

                        <div className={imgGenNodeStyles.bottomGradient} />
                        <div className={imgGenNodeStyles.bottomPanelWrap}>
                            <div className={imgGenNodeStyles.bottomPanel}>
                                <div className={imgGenNodeStyles.bottomHeader}>
                                    <label className={imgGenNodeStyles.bottomLabel}>Prompt</label>
                                    <span className={imgGenNodeStyles.bottomRight}>{aspectRatio}</span>
                                </div>
                                <textarea
                                    ref={promptRef}
                                    className={imgGenNodeStyles.prompt}
                                    rows={3}
                                    value={prompt}
                                    onChange={e => onPromptChange(e.target.value)}
                                    onCompositionStart={() => onPromptCompositionStart?.()}
                                    onCompositionEnd={e => onPromptCompositionEnd?.(e.currentTarget.value)}
                                    onKeyDown={e => e.stopPropagation()}
                                    placeholder="输入你想生成的图像描述…"
                                    spellCheck={false}
                                />
                                <input type="hidden" value={outputRaw} readOnly />
                            </div>
                        </div>
                    </div>
                </div>
            </NodeWrapper>
        </>
    );
}
