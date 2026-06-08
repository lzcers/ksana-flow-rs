import React from 'react';
import { type NodeProps } from '@xyflow/react';
import { FileText, Upload } from 'lucide-react';
import type { NodeData } from '@/model/workflow/types';
import { NodeWrapper } from '../shared/NodeWrapper';
import { textFileNodeStyles } from './styles';

export function TextFileNodeView({
  id,
  type,
  data,
  selected,
  width,
  height,
  fileInputRef,
  fileId,
  filename,
  size,
  isUploading,
  error,
  onUploadClick,
  onFileChange,
}: NodeProps & {
  data: NodeData;
} & {
  fileInputRef: React.RefObject<HTMLInputElement | null>;
  fileId?: string;
  filename?: string;
  size?: number;
  isUploading: boolean;
  error: string | null;
  onUploadClick: () => void;
  onFileChange: (event: React.ChangeEvent<HTMLInputElement>) => void;
}) {
  return (
    <NodeWrapper
      id={id}
      type={type}
      data={data}
      selected={selected}
      className="flex flex-col"
      minWidth={280}
      minHeight={120}
      style={{ width, height }}
    >
      <div className={textFileNodeStyles.container}>
        <input type="file" ref={fileInputRef} onChange={onFileChange} accept=".txt" className="hidden" />

        {fileId ? (
          <div className={textFileNodeStyles.fileInfo}>
            <div className={textFileNodeStyles.fileRow}>
              <FileText size={24} className="text-zinc-400" />
              <span className={textFileNodeStyles.fileName} title={filename}>
                {filename}
              </span>
            </div>
            {typeof size === 'number' && <div className={textFileNodeStyles.fileSize}>{(size / 1024).toFixed(2)} KB</div>}
            <button onClick={onUploadClick} className={textFileNodeStyles.changeButton} disabled={isUploading}>
              {isUploading ? 'Uploading...' : 'Change File'}
            </button>
          </div>
        ) : (
          <button onClick={onUploadClick} disabled={isUploading} className={textFileNodeStyles.uploadButton}>
            <Upload size={16} />
            {isUploading ? 'Uploading...' : 'Upload Text File'}
          </button>
        )}

        {error && <div className={textFileNodeStyles.error}>{error}</div>}
      </div>
    </NodeWrapper>
  );
}
