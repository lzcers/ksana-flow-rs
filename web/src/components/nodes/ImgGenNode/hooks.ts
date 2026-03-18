import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useStore } from "@/store";
import { useNodeConfig } from "../shared/hooks/useNodeConfig";
import { useNodeConfigField } from "../shared/hooks/useNodeConfigField";
import type { NodeData } from "@/model/workflow/types";

const IMG_GEN_MIN_WIDTH = 400;
const IMG_GEN_MIN_HEIGHT = 400;

type ImgGenOutput = {
    media_id?: string;
    data_url?: string;
    mime_type?: string;
    space_id?: string;
};

function parseAspectRatio(value: unknown): number | undefined {
    if (typeof value !== "string") return undefined;
    const s = value.trim();
    if (!s) return undefined;

    const sep = s.includes(":") ? ":" : s.includes("/") ? "/" : null;
    if (!sep) return undefined;
    const [wRaw, hRaw] = s.split(sep).map(p => p.trim());
    const w = Number(wRaw);
    const h = Number(hRaw);
    if (!Number.isFinite(w) || !Number.isFinite(h) || w <= 0 || h <= 0) return undefined;
    return w / h;
}

function fitToAspectRatio(width: number, height: number, ratio: number, minWidth: number, minHeight: number): { width: number; height: number } {
    const baseW = Math.max(width, minWidth);
    const baseH = Math.max(height, minHeight);

    const cand1 = (() => {
        let h = baseH;
        let w = h * ratio;
        if (w < minWidth) {
            w = minWidth;
            h = w / ratio;
        }
        if (h < minHeight) {
            h = minHeight;
            w = h * ratio;
        }
        return { width: w, height: h };
    })();

    const cand2 = (() => {
        let w = baseW;
        let h = w / ratio;
        if (h < minHeight) {
            h = minHeight;
            w = h * ratio;
        }
        if (w < minWidth) {
            w = minWidth;
            h = w / ratio;
        }
        return { width: w, height: h };
    })();

    const d1 = Math.hypot(cand1.width - baseW, cand1.height - baseH);
    const d2 = Math.hypot(cand2.width - baseW, cand2.height - baseH);
    const picked = d1 <= d2 ? cand1 : cand2;

    const nextW = Math.round(picked.width);
    const nextH = Math.round(picked.height);
    return { width: nextW, height: nextH };
}

function parseImgGenOutput(value: unknown): { imageSrc?: string; mediaId?: string } {
    if (typeof value === "string") {
        const s = value.trim();
        if (s.startsWith("data:image/")) return { imageSrc: s };
        try {
            const obj = JSON.parse(s) as ImgGenOutput;
            if (typeof obj?.data_url === "string" && obj.data_url.startsWith("data:image/")) {
                return { imageSrc: obj.data_url, mediaId: obj.media_id };
            }
            if (typeof obj?.media_id === "string" && obj.media_id.length > 0) {
                return { mediaId: obj.media_id };
            }
        } catch {
            return {};
        }
        return {};
    }

    if (value && typeof value === "object") {
        const obj = value as ImgGenOutput;
        if (typeof obj?.data_url === "string" && obj.data_url.startsWith("data:image/")) {
            return { imageSrc: obj.data_url, mediaId: obj.media_id };
        }
        if (typeof obj?.media_id === "string" && obj.media_id.length > 0) {
            return { mediaId: obj.media_id };
        }
    }

    return {};
}

export function useImgGenNodeController({
    id,
    data,
    selected,
    width,
    height,
}: {
    id: string;
    data: NodeData;
    selected: boolean;
    width?: number | null;
    height?: number | null;
}) {
    const updateNodeDimensions = useStore(s => s.updateNodeDimensions);
    const currentSpaceId = useStore(s => s.currentSpaceId);
    const { updateConfig } = useNodeConfig(id, data.config);

    const [isConfigOpen, setIsConfigOpen] = useState(false);
    const [isPreviewOpen, setIsPreviewOpen] = useState(false);

    const promptRef = useRef<HTMLTextAreaElement>(null);

    const promptField = useNodeConfigField<string>({
        value: String(data.config?.user_prompt_template ?? ""),
        commitMode: "change",
        composition: true,
        isFocused: () => document.activeElement === promptRef.current,
        updateValue: next => updateConfig({ user_prompt_template: next }),
    });

    const [model, setModel] = useState<string | undefined>(
        typeof data.config?.model === "string" ? data.config.model : undefined,
    );
    const [aspectRatio, setAspectRatio] = useState<string>(String(data.config?.aspect_ratio ?? "1:1"));
    const [imageSize, setImageSize] = useState<string>(String(data.config?.image_size ?? "1K"));
    const [inputImageFileId, setInputImageFileId] = useState<string>(String(data.config?.input_image_file_id ?? ""));
    const [outputRaw, setOutputRaw] = useState<string>(String(data.config?.output ?? ""));

    const initialParsed = useMemo(() => {
        const configOutput = data.config?.output;
        if (typeof configOutput === "string") return parseImgGenOutput(configOutput);
        if (typeof data.lastMessage === "string") return parseImgGenOutput(data.lastMessage);
        return {};
    }, [data.config?.output, data.lastMessage]);

    const [imageSrc, setImageSrc] = useState<string | undefined>(initialParsed.imageSrc);
    const [mediaId, setMediaId] = useState<string | undefined>(initialParsed.mediaId);
    const objectUrlRef = useRef<string | null>(null);

    const apiBase = useMemo(() => {
        return import.meta.env.PROD ? "/api" : "http://localhost:3000/api";
    }, []);

    const parsedAspectRatio = useMemo(() => parseAspectRatio(aspectRatio), [aspectRatio]);
    const lastAppliedAspectRatioRef = useRef<string | null>(null);

    useEffect(() => {
        if (!selected) return;
        if (!parsedAspectRatio) return;
        if (lastAppliedAspectRatioRef.current === aspectRatio) return;
        lastAppliedAspectRatioRef.current = aspectRatio;

        const baseW = typeof width === "number" ? width : IMG_GEN_MIN_WIDTH;
        const baseH = typeof height === "number" ? height : IMG_GEN_MIN_HEIGHT;
        const next = fitToAspectRatio(baseW, baseH, parsedAspectRatio, IMG_GEN_MIN_WIDTH, IMG_GEN_MIN_HEIGHT);

        if (Math.abs(next.width - baseW) < 1 && Math.abs(next.height - baseH) < 1) return;
        updateNodeDimensions(id, next.width, next.height);
    }, [aspectRatio, id, parsedAspectRatio, selected, updateNodeDimensions, height, width]);

    useEffect(() => {
        return () => {
            if (objectUrlRef.current) {
                URL.revokeObjectURL(objectUrlRef.current);
                objectUrlRef.current = null;
            }
        };
    }, []);

    useEffect(() => {
        setModel(typeof data.config?.model === "string" ? data.config.model : undefined);
        setAspectRatio(String(data.config?.aspect_ratio ?? "1:1"));
        setImageSize(String(data.config?.image_size ?? "1K"));
        setInputImageFileId(String(data.config?.input_image_file_id ?? ""));
        setOutputRaw(String(data.config?.output ?? ""));
    }, [data.config?.model, data.config?.aspect_ratio, data.config?.image_size, data.config?.input_image_file_id, data.config?.output]);

    useEffect(() => {
        if (!isConfigOpen) return;
        const onKey = (e: KeyboardEvent) => {
            if (e.key === "Escape") setIsConfigOpen(false);
        };
        window.addEventListener("keydown", onKey);
        return () => window.removeEventListener("keydown", onKey);
    }, [isConfigOpen]);

    useEffect(() => {
        if (data.status !== "running") return;
        setImageSrc(undefined);
        setMediaId(undefined);
        setOutputRaw("");
    }, [data.status]);

    useEffect(() => {
        const value = data.outputs && "output" in data.outputs ? data.outputs.output : data.lastMessage;
        if (value == null) return;
        const nextRaw = typeof value === "string" ? value : JSON.stringify(value);
        setOutputRaw(nextRaw);
        const parsed = parseImgGenOutput(nextRaw);
        setImageSrc(parsed.imageSrc);
        setMediaId(parsed.mediaId);
    }, [data.lastMessage, data.outputs]);

    useEffect(() => {
        const run = async () => {
            if (imageSrc) return;
            if (!mediaId) return;
            if (!currentSpaceId) return;

            try {
                const res = await fetch(`${apiBase}/ai_media/${mediaId}?space_id=${currentSpaceId}`);
                if (!res.ok) return;
                const blob = await res.blob();
                const nextUrl = URL.createObjectURL(blob);
                if (objectUrlRef.current) URL.revokeObjectURL(objectUrlRef.current);
                objectUrlRef.current = nextUrl;
                setImageSrc(nextUrl);
            } catch {
                return;
            }
        };
        run();
    }, [apiBase, currentSpaceId, imageSrc, mediaId]);

    const onModelChange = useCallback(
        (v: string) => {
            setModel(v);
            updateConfig({ model: v });
        },
        [updateConfig],
    );

    const onAspectRatioChange = useCallback(
        (v: string) => {
            setAspectRatio(v);
            updateConfig({ aspect_ratio: v });
        },
        [updateConfig],
    );

    const onImageSizeChange = useCallback(
        (v: string) => {
            setImageSize(v);
            updateConfig({ image_size: v });
        },
        [updateConfig],
    );

    const onInputImageFileIdChange = useCallback(
        (v: string) => {
            setInputImageFileId(v);
            updateConfig({ input_image_file_id: v });
        },
        [updateConfig],
    );

    const onOpenPreview = useCallback(
        (e: React.MouseEvent) => {
            const target = e.target as unknown;
            if (
                target instanceof HTMLTextAreaElement ||
                target instanceof HTMLInputElement ||
                target instanceof HTMLSelectElement ||
                target instanceof HTMLButtonElement
            ) {
                return;
            }
            if (!imageSrc) return;
            e.preventDefault();
            e.stopPropagation();
            setIsPreviewOpen(true);
        },
        [imageSrc],
    );

    return {
        minWidth: IMG_GEN_MIN_WIDTH,
        minHeight: IMG_GEN_MIN_HEIGHT,
        parsedAspectRatio,
        isConfigOpen,
        setIsConfigOpen,
        isPreviewOpen,
        setIsPreviewOpen,
        promptRef,
        prompt: promptField.draft,
        onPromptChange: promptField.onChange,
        onPromptCompositionStart: promptField.onCompositionStart,
        onPromptCompositionEnd: promptField.onCompositionEnd,
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
    };
}
