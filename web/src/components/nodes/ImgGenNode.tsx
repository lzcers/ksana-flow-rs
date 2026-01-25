import { memo, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Position, type NodeProps } from '@xyflow/react';
import { Settings, X } from 'lucide-react';
import type { NodeData } from '../../model/types';
import { NodeWrapper } from './NodeWrapper';
import { useStore } from '../../store';
import './index.css';

const TARGET_HANDLES = [Position.Left, Position.Top, Position.Bottom];
const SOURCE_HANDLES = [Position.Right, Position.Top, Position.Bottom];

type ImgGenOutput = {
  media_id?: string;
  data_url?: string;
  mime_type?: string;
  space_id?: string;
};

function parseImgGenOutput(value: unknown): { imageSrc?: string; mediaId?: string } {
  if (typeof value === 'string') {
    const s = value.trim();
    if (s.startsWith('data:image/')) return { imageSrc: s };
    try {
      const obj = JSON.parse(s) as ImgGenOutput;
      if (typeof obj?.data_url === 'string' && obj.data_url.startsWith('data:image/')) {
        return { imageSrc: obj.data_url, mediaId: obj.media_id };
      }
      if (typeof obj?.media_id === 'string' && obj.media_id.length > 0) {
        return { mediaId: obj.media_id };
      }
    } catch {
      return {};
    }
    return {};
  }

  if (value && typeof value === 'object') {
    const obj = value as ImgGenOutput;
    if (typeof obj?.data_url === 'string' && obj.data_url.startsWith('data:image/')) {
      return { imageSrc: obj.data_url, mediaId: obj.media_id };
    }
    if (typeof obj?.media_id === 'string' && obj.media_id.length > 0) {
      return { mediaId: obj.media_id };
    }
  }

  return {};
}

export const ImgGenNode = memo(({ id, type, data, selected, width, height }: NodeProps & { data: NodeData }) => {
  const { updateNodeData, events$, currentRunId, currentSpaceId } = useStore();

  const [isConfigOpen, setIsConfigOpen] = useState(false);
  const [promptText, setPromptText] = useState<string>(data.config?.user_prompt_template || '');
  const [model, setModel] = useState<string>(data.config?.model);
  const [aspectRatio, setAspectRatio] = useState<string>(data.config?.aspect_ratio || '1:1');
  const [imageSize, setImageSize] = useState<string>(data.config?.image_size || '1K');
  const [inputImageFileId, setInputImageFileId] = useState<string>(data.config?.input_image_file_id || '');
  const [outputRaw, setOutputRaw] = useState<string>(data.config?.output || '');

  const initialParsed = useMemo(() => {
    if (typeof data.config?.output === 'string') return parseImgGenOutput(data.config.output);
    if (typeof data.lastMessage === 'string') return parseImgGenOutput(data.lastMessage);
    return {};
  }, []);

  const [imageSrc, setImageSrc] = useState<string | undefined>(initialParsed.imageSrc);
  const [mediaId, setMediaId] = useState<string | undefined>(initialParsed.mediaId);
  const objectUrlRef = useRef<string | null>(null);

  const isComposing = useRef(false);
  const promptRef = useRef<HTMLTextAreaElement>(null);

  const apiBase = useMemo(() => {
    return import.meta.env.PROD ? '/api' : 'http://localhost:3000/api';
  }, []);

  useEffect(() => {
    return () => {
      if (objectUrlRef.current) {
        URL.revokeObjectURL(objectUrlRef.current);
        objectUrlRef.current = null;
      }
    };
  }, []);

  useEffect(() => {
    if (document.activeElement !== promptRef.current) {
      setPromptText(data.config?.user_prompt_template || '');
    }
  }, [data.config?.user_prompt_template]);

  useEffect(() => {
    setModel(data.config?.model);
    setAspectRatio(data.config?.aspect_ratio || '1:1');
    setImageSize(data.config?.image_size || '1K');
    setInputImageFileId(data.config?.input_image_file_id || '');
  }, [data.config?.model, data.config?.aspect_ratio, data.config?.image_size, data.config?.input_image_file_id]);

  useEffect(() => {
    if (!isConfigOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setIsConfigOpen(false);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [isConfigOpen]);

  useEffect(() => {
    if (!events$) return;
    const subscription = events$.subscribe((wrapper: any) => {
      const { event, runId } = wrapper;
      if (currentRunId && runId !== currentRunId) return;

      if (event.NodeStarted) {
        if (event.NodeStarted === id) {
          setImageSrc(undefined);
          setMediaId(undefined);
          setOutputRaw('');
          updateNodeData(id, { config: { ...data.config, output: '' } });
        }
      } else if (event.NodeOutMessage) {
        const [nodeId, value] = event.NodeOutMessage;
        if (nodeId === id) {
          const nextRaw = typeof value === 'string' ? value : JSON.stringify(value);
          setOutputRaw(nextRaw);
          updateNodeData(id, { config: { ...data.config, output: nextRaw } });
          const parsed = parseImgGenOutput(nextRaw);
          setImageSrc(parsed.imageSrc);
          setMediaId(parsed.mediaId);
        }
      }
    });
    return () => subscription.unsubscribe();
  }, [events$, currentRunId, id, updateNodeData, data.config]);

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

  const onPromptChange = useCallback((e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const next = e.target.value;
    setPromptText(next);
    if (!isComposing.current) {
      updateNodeData(id, { config: { ...data.config, user_prompt_template: next } });
    }
  }, [id, data.config, updateNodeData]);

  const onPromptCompositionStart = useCallback(() => {
    isComposing.current = true;
  }, []);

  const onPromptCompositionEnd = useCallback((e: React.CompositionEvent<HTMLTextAreaElement>) => {
    isComposing.current = false;
    const next = e.currentTarget.value;
    updateNodeData(id, { config: { ...data.config, user_prompt_template: next } });
  }, [id, data.config, updateNodeData]);

  const onModelChange = useCallback((e: React.ChangeEvent<HTMLSelectElement>) => {
    const v = e.target.value;
    setModel(v);
    updateNodeData(id, { config: { ...data.config, model: v } });
  }, [id, data.config, updateNodeData]);

  const onAspectRatioChange = useCallback((e: React.ChangeEvent<HTMLSelectElement>) => {
    const v = e.target.value;
    setAspectRatio(v);
    updateNodeData(id, { config: { ...data.config, aspect_ratio: v } });
  }, [id, data.config, updateNodeData]);

  const onImageSizeChange = useCallback((e: React.ChangeEvent<HTMLSelectElement>) => {
    const v = e.target.value;
    setImageSize(v);
    updateNodeData(id, { config: { ...data.config, image_size: v } });
  }, [id, data.config, updateNodeData]);

  const onInputImageFileIdChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const v = e.target.value;
    setInputImageFileId(v);
    updateNodeData(id, { config: { ...data.config, input_image_file_id: v } });
  }, [id, data.config, updateNodeData]);

  const headerActions = (
    <div className="relative flex items-center gap-1">
      <button
        onClick={(e) => {
          e.stopPropagation();
          setIsConfigOpen(v => !v);
        }}
        className="text-zinc-400 hover:text-zinc-200 transition-colors p-1 rounded hover:bg-zinc-800"
        title="设置"
      >
        <Settings size={12} />
      </button>
    </div>
  );

  return (
    <NodeWrapper
      id={id}
      type={type}
      data={data}
      selected={selected}
      minWidth={300}
      minHeight={360}
      style={{ width, height }}
      targetHandles={TARGET_HANDLES}
      sourceHandles={SOURCE_HANDLES}
      headerActions={headerActions}
    >
      <div className="flex flex-col h-full relative">
        {isConfigOpen && (
          <div className="absolute inset-0 z-50 bg-zinc-900/95 backdrop-blur-xl border border-zinc-800 rounded-xl shadow-2xl flex flex-col">
            <div className="px-3 py-2 flex items-center justify-between border-b border-zinc-800">
              <div className="flex items-center gap-2 text-[10px] text-zinc-400">
                <span>图像生成设置</span>
              </div>
              <button
                onClick={(e) => { e.stopPropagation(); setIsConfigOpen(false); }}
                className="text-zinc-400 hover:text-zinc-200 p-1 rounded hover:bg-zinc-800"
                title="关闭"
              >
                <X size={12} />
              </button>
            </div>
            <div className="p-3 flex flex-col gap-2 flex-1 overflow-auto custom-scrollbar">
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2">
                  <span className="text-[10px] w-10 text-zinc-500 font-bold">Model</span>
                  <select
                    className="min-w-[200px] text-xs bg-zinc-800/50 hover:bg-zinc-800/80 focus:bg-zinc-900 border border-zinc-800 focus:border-zinc-700 rounded px-1 py-1 text-zinc-300 focus:text-zinc-200 focus:outline-none focus:ring-1 focus:ring-blue-500/50 focus:ring-offset-1 focus:ring-offset-zinc-900 transition-colors duration-200"
                    value={model}
                    onChange={onModelChange}
                  >
                    <option value="google/gemini-3-pro-image-preview">Gemini 3 Pro Image Preview</option>
                  </select>
                </div>
              </div>
              <div className="flex flex-col gap-2">
                <div className="flex items-center gap-2">
                  <span className="text-[10px] w-10 text-zinc-500 font-bold">Aspect</span>
                  <select
                    className="min-w-[120px] text-xs bg-zinc-800/50 hover:bg-zinc-800/80 focus:bg-zinc-900 border border-zinc-800 focus:border-zinc-700 rounded px-1 py-1 text-zinc-300 focus:text-zinc-200 focus:outline-none focus:ring-1 focus:ring-blue-500/50 focus:ring-offset-1 focus:ring-offset-zinc-900 transition-colors duration-200"
                    value={aspectRatio}
                    onChange={onAspectRatioChange}
                  >
                    <option value="1:1">1:1</option>
                    <option value="16:9">16:9</option>
                    <option value="9:16">9:16</option>
                    <option value="4:3">4:3</option>
                    <option value="3:4">3:4</option>
                  </select>
                </div>
                <div className="flex items-center gap-2">
                  <span className="text-[10px] w-10 text-zinc-500 font-bold">Size</span>
                  <select
                    className="min-w-[90px] text-xs bg-zinc-800/50 hover:bg-zinc-800/80 focus:bg-zinc-900 border border-zinc-800 focus:border-zinc-700 rounded px-1 py-1 text-zinc-300 focus:text-zinc-200 focus:outline-none focus:ring-1 focus:ring-blue-500/50 focus:ring-offset-1 focus:ring-offset-zinc-900 transition-colors duration-200"
                    value={imageSize}
                    onChange={onImageSizeChange}
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
                  onChange={onInputImageFileIdChange}
                  onKeyDown={(e) => e.stopPropagation()}
                  className="w-full text-xs bg-zinc-900/60 hover:bg-zinc-900/70 focus:bg-zinc-900 border border-zinc-800 focus:border-zinc-700 rounded px-2 py-1 text-zinc-300 focus:text-zinc-200 focus:outline-none focus:ring-1 focus:ring-blue-500/50 focus:ring-offset-1 focus:ring-offset-zinc-900 transition-colors duration-200 placeholder-zinc-500 shadow-inner"
                  placeholder="uploaded_files.id"
                />
              </div>
            </div>
          </div>
        )}

        <div className="flex flex-col flex-1 p-3 bg-zinc-950/50 border-b border-zinc-800 overflow-hidden">
          <div className="text-[10px] text-zinc-500 font-bold mb-1 flex items-center justify-between">
            <span>图像输出</span>
            <span className="text-[10px] opacity-50">{data.status === 'running' ? 'Generating' : imageSrc ? 'Ready' : 'Empty'}</span>
          </div>
          <div className="flex-1 bg-black/40 border border-zinc-800 rounded-md overflow-hidden flex items-center justify-center">
            {imageSrc ? (
              <img
                src={imageSrc}
                alt="generated"
                className="w-full h-full object-contain"
                draggable={false}
              />
            ) : (
              <div className="text-xs text-zinc-500 px-3 text-center">
                {data.status === 'running' ? '正在生成图像…' : '暂无图像输出'}
              </div>
            )}
          </div>
        </div>

        <div className="p-2 space-y-2">
          <div className='mb-0'>
            <label className="text-[10px] text-zinc-500 font-bold block mb-1">Prompt</label>
            <textarea
              ref={promptRef}
              className="w-full flex-1 text-xs bg-zinc-900/60 hover:bg-zinc-900/70 focus:bg-zinc-900 border border-zinc-800 focus:border-zinc-700 rounded resize-none nodrag focus:outline-none focus:ring-1 focus:ring-blue-500/50 focus:ring-offset-1 focus:ring-offset-zinc-900 text-zinc-300 focus:text-zinc-200 transition-colors duration-200 placeholder-zinc-500 shadow-inner"
              rows={5}
              value={promptText}
              onChange={onPromptChange}
              onCompositionStart={onPromptCompositionStart}
              onCompositionEnd={onPromptCompositionEnd}
              onKeyDown={(e) => e.stopPropagation()}
              placeholder="输入你想生成的图像描述…"
              spellCheck={false}
            />
          </div>
          <input type="hidden" value={outputRaw} readOnly />
        </div>
      </div>
    </NodeWrapper>
  );
});

ImgGenNode.displayName = 'ImgGenNode';
