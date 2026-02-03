import React, { memo, useCallback, useRef, useState } from 'react';
import { type NodeProps } from '@xyflow/react';
import { useStore } from '@/store';
import { type NodeData } from '@/model/types';
import { useNodeConfig } from '../shared/hooks/useNodeConfig';
import { TextFileNodeView } from './view';

export const TextFileNode = memo((props: NodeProps & { data: NodeData }) => {
  const { id, data } = props;
  const { uploadFile } = useStore();
  const { updateConfig } = useNodeConfig(id, data.config);

  const fileInputRef = useRef<HTMLInputElement>(null);
  const [isUploading, setIsUploading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleUploadClick = useCallback(() => {
    fileInputRef.current?.click();
  }, []);

  const handleFileChange = useCallback(
    async (event: React.ChangeEvent<HTMLInputElement>) => {
      const file = event.target.files?.[0];
      if (!file) return;

      setIsUploading(true);
      setError(null);

      try {
        const result = await uploadFile(file);
        updateConfig({
          file_id: result.id,
          filename: result.filename,
          size: result.size,
        });
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Upload failed');
      } finally {
        setIsUploading(false);
        if (fileInputRef.current) {
          fileInputRef.current.value = '';
        }
      }
    },
    [uploadFile, updateConfig],
  );

  return (
    <TextFileNodeView
      {...props}
      fileInputRef={fileInputRef}
      fileId={data.config?.file_id}
      filename={data.config?.filename}
      size={data.config?.size}
      isUploading={isUploading}
      error={error}
      onUploadClick={handleUploadClick}
      onFileChange={handleFileChange}
    />
  );
});

TextFileNode.displayName = 'TextFileNode';
