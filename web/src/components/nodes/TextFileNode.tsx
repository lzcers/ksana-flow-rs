import React, { memo, useRef, useState } from 'react';
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Upload, FileText } from 'lucide-react';
import { NodeWrapper } from './NodeWrapper';
import { useStore } from '../../store';
import { type NodeData } from '../../model/types';
import { uploadFile } from '../../api';

export const TextFileNodeComponent = ({ id, data, selected }: NodeProps & { data: NodeData }) => {
    const { updateNodeData } = useStore();
    const fileInputRef = useRef<HTMLInputElement>(null);
    const [isUploading, setIsUploading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const fileId = data.config?.file_id;
    const filename = data.config?.filename;
    const size = data.config?.size;

    const handleUploadClick = () => {
        fileInputRef.current?.click();
    };

    const handleFileChange = async (event: React.ChangeEvent<HTMLInputElement>) => {
        const file = event.target.files?.[0];
        if (!file) return;

        setIsUploading(true);
        setError(null);

        try {
            const result = await uploadFile(file);

            updateNodeData(id, {
                config: {
                    ...data.config,
                    file_id: result.id,
                    filename: result.filename,
                    size: result.size,
                }
            });
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Upload failed');
        } finally {
            setIsUploading(false);
            // Reset input so same file can be selected again if needed
            if (fileInputRef.current) {
                fileInputRef.current.value = '';
            }
        }
    };

    return (
        <NodeWrapper
            id={id}
            data={data}
            selected={selected}
            className="flex flex-col"
            minWidth={200}
            showSourceHandle={false}
            showTargetHandle={false}
        >
            <div className="p-4 flex flex-col items-center justify-center gap-3">
                <input
                    type="file"
                    ref={fileInputRef}
                    onChange={handleFileChange}
                    accept=".txt"
                    className="hidden"
                />

                {fileId ? (
                    <div className="flex flex-col items-center gap-1 w-full">
                        <div className="flex items-center gap-2 text-zinc-200">
                            <FileText size={24} className="text-blue-500" />
                            <span className="text-sm truncate max-w-[150px]" title={filename}>
                                {filename}
                            </span>
                        </div>
                        <div className="text-xs text-zinc-500">
                            {(size / 1024).toFixed(2)} KB
                        </div>
                        <button
                            onClick={handleUploadClick}
                            className="mt-2 text-xs text-blue-400 hover:text-blue-300 underline"
                            disabled={isUploading}
                        >
                            {isUploading ? 'Uploading...' : 'Change File'}
                        </button>
                    </div>
                ) : (
                    <button
                        onClick={handleUploadClick}
                        disabled={isUploading}
                        className="flex items-center gap-2 px-3 py-2 bg-zinc-800 hover:bg-zinc-700 rounded text-zinc-200 text-sm transition-colors border border-zinc-700"
                    >
                        <Upload size={16} />
                        {isUploading ? 'Uploading...' : 'Upload Text File'}
                    </button>
                )}

                {error && (
                    <div className="text-xs text-red-400 mt-1">
                        {error}
                    </div>
                )}
            </div>

            <Handle
                type="source"
                position={Position.Right}
                className="!bg-slate-500 !w-3 !h-3"
                style={{ right: -6 }}
            />
        </NodeWrapper>
    );
};

export const TextFileNode = memo(TextFileNodeComponent);
