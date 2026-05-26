import { useEffect, useRef, useState } from "react";
import { useIncremark } from "@incremark/react";
import { useNodeConfig } from "../shared/hooks/useNodeConfig";
import type { NodeData } from "@/model/workflow/types";

function adjustTextareaHeight(el: HTMLTextAreaElement) {
    el.style.height = "auto";
    el.style.height = `${el.scrollHeight}px`;
    el.style.overflowY = el.scrollHeight > el.clientHeight ? "auto" : "hidden";
}

export function useLLMNodeController(id: string, data: NodeData) {
    const { updateConfig } = useNodeConfig(id, data.config);

    const systemInputRef = useRef<HTMLTextAreaElement>(null);
    const userInputRef = useRef<HTMLTextAreaElement>(null);

    const [systemPrompt, setSystemPrompt] = useState(String(data.config?.system_prompt ?? ""));
    const [userPrompt, setUserPrompt] = useState(String(data.config?.user_prompt_template ?? ""));
    const [model, setModel] = useState(String(data.config?.model ?? "deepseek-chat"));
    const [stream, setStream] = useState(Boolean(data.config?.stream ?? true));
    const [isConfigOpen, setIsConfigOpen] = useState(false);
    const [outputText, setOutputText] = useState<string>("");
    const [isMarkdown, setIsMarkdown] = useState(true);
    const [isFullScreen, setIsFullScreen] = useState(false);
    const [isSystemFullScreen, setIsSystemFullScreen] = useState(false);

    const incremark = useIncremark({
        math: { tex: true },
        gfm: true,
    });

    const isComposingSystem = useRef(false);
    const isComposingUser = useRef(false);

    useEffect(() => {
        if (document.activeElement !== systemInputRef.current) {
            setSystemPrompt(String(data.config?.system_prompt ?? ""));
            if (systemInputRef.current) adjustTextareaHeight(systemInputRef.current);
        }
    }, [data.config?.system_prompt]);

    useEffect(() => {
        if (document.activeElement !== userInputRef.current) {
            setUserPrompt(String(data.config?.user_prompt_template ?? ""));
            if (userInputRef.current) adjustTextareaHeight(userInputRef.current);
        }
    }, [data.config?.user_prompt_template]);

    useEffect(() => {
        setModel(String(data.config?.model ?? "deepseek-chat"));
        setStream(Boolean(data.config?.stream ?? true));
    }, [data.config?.model, data.config?.stream]);

    // 直接渲染 lastMessage，因为 instance.ts 已在事件处理时合并了流式消息
    useEffect(() => {
        const text = typeof data.lastMessage === "string" ? data.lastMessage : "";
        setOutputText(text);
        if (isMarkdown) {
            incremark.render(text);
        }
    }, [data.lastMessage, isMarkdown]);

    useEffect(() => {
        if (!isConfigOpen) return;
        const onKey = (e: KeyboardEvent) => {
            if (e.key === "Escape") setIsConfigOpen(false);
        };
        window.addEventListener("keydown", onKey);
        return () => window.removeEventListener("keydown", onKey);
    }, [isConfigOpen]);

    useEffect(() => {
        if (data.isOutputStream) return;
        const value = data?.outputs?.output;
        if (typeof value !== "string") return;
        setOutputText(value);
        if (isMarkdown) {
            incremark.render(value);
        }
    }, [data?.outputs, data.isOutputStream, isMarkdown]);

    const onModelChange = (next: string) => {
        setModel(next);
        updateConfig({ model: next });
    };

    const onStreamChange = (next: boolean) => {
        setStream(next);
        updateConfig({ stream: next });
    };

    const onSystemPromptChange = (next: string, el?: HTMLTextAreaElement) => {
        setSystemPrompt(next);
        if (el) adjustTextareaHeight(el);
        if (!isComposingSystem.current) {
            updateConfig({ system_prompt: next });
        }
    };

    const onSystemCompositionStart = () => {
        isComposingSystem.current = true;
    };

    const onSystemCompositionEnd = (next: string) => {
        isComposingSystem.current = false;
        updateConfig({ system_prompt: next });
    };

    const onUserPromptChange = (next: string, el?: HTMLTextAreaElement) => {
        setUserPrompt(next);
        if (el) adjustTextareaHeight(el);
        if (!isComposingUser.current) {
            updateConfig({ user_prompt_template: next });
        }
    };

    const onUserCompositionStart = () => {
        isComposingUser.current = true;
    };

    const onUserCompositionEnd = (next: string) => {
        isComposingUser.current = false;
        updateConfig({ user_prompt_template: next });
    };

    const onOutputChange = (next: string) => {
        setOutputText(next);
    };

    const onOutputBlur = () => {
        updateConfig({ output: outputText });
        if (isMarkdown) {
            incremark.render(outputText);
        }
    };

    return {
        systemInputRef,
        userInputRef,
        systemPrompt,
        userPrompt,
        model,
        stream,
        isConfigOpen,
        isMarkdown,
        isFullScreen,
        isSystemFullScreen,
        outputText,
        incremark,
        setIsConfigOpen,
        setIsMarkdown,
        setIsFullScreen,
        setIsSystemFullScreen,
        onModelChange,
        onStreamChange,
        onSystemPromptChange,
        onSystemCompositionStart,
        onSystemCompositionEnd,
        onUserPromptChange,
        onUserCompositionStart,
        onUserCompositionEnd,
        onOutputChange,
        onOutputBlur,
    };
}
