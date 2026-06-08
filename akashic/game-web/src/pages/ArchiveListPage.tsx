import React, { useMemo, useRef, useState } from 'react';
import { ArrowLeft, Download, FolderOpen, Trash2, TriangleAlert, Upload } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import {
  PageTitle,
  PrimaryButton,
  ScreenShell,
  SecondaryButton,
  SectionCard,
  StoryFrame,
  StatusPill,
} from '../components/AkashicUI';
import {
  readStoredSaveArchive,
  readStoredSaveSlots,
  removeStoredSaveSlot,
  upsertStoredSaveSlot,
  writeStoredSaveArchive,
  type StoredSaveSlot,
} from '../lib/saveSlots';
import { appRoutes } from '../lib/appRoutes';
import { useGameUIStore } from '../store/gameUIStore';

function formatTimeLabel(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  return new Intl.DateTimeFormat('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  }).format(date);
}

interface SharedArchiveFile {
  sessionId: string;
  title: string;
  compressedArchive: string;
}

function isSharedArchiveFile(value: unknown): value is SharedArchiveFile {
  if (!value || typeof value !== 'object') {
    return false;
  }

  const archiveFile = value as Record<string, unknown>;
  return typeof archiveFile.sessionId === 'string'
    && typeof archiveFile.title === 'string'
    && typeof archiveFile.compressedArchive === 'string';
}

function buildArchiveFileName(slot: StoredSaveSlot) {
  const baseName = (slot.title || slot.slotId)
    .trim()
    .replace(/[\\/:*?"<>|]+/g, '-')
    .replace(/\s+/g, '-')
    .toLowerCase();
  return `${baseName || slot.slotId}.json`;
}

function createSlotId() {
  const cryptoApi = globalThis.crypto;
  if (typeof cryptoApi?.randomUUID === 'function') {
    return `slot-${cryptoApi.randomUUID().replace(/-/g, '')}`;
  }

  if (typeof cryptoApi?.getRandomValues === 'function') {
    const randomBytes = new Uint8Array(16);
    cryptoApi.getRandomValues(randomBytes);
    const randomToken = Array.from(randomBytes, (byte) => byte.toString(16).padStart(2, '0')).join('');
    return `slot-${randomToken}`;
  }

  return `slot-${Date.now().toString(16)}${Math.random().toString(16).slice(2)}`;
}

const ArchiveListPage: React.FC = () => {
  const navigate = useNavigate();
  const loadSave = useGameUIStore((state) => state.loadSave);
  const isLoading = useGameUIStore((state) => state.isLoading);
  const error = useGameUIStore((state) => state.error);
  const [slots, setSlots] = useState<StoredSaveSlot[]>(() => readStoredSaveSlots());
  const [feedback, setFeedback] = useState<{ type: 'success' | 'error'; message: string } | null>(null);
  const importInputRef = useRef<HTMLInputElement | null>(null);

  const refreshSlots = () => {
    setSlots(readStoredSaveSlots());
  };

  const hasSlots = useMemo(() => slots.length > 0, [slots]);

  const handleLoad = async (slotId: string) => {
    setFeedback(null);
    try {
      await loadSave(slotId);
    } catch {
      // Store already exposes the failure reason.
    }
  };

  const handleDelete = (slot: StoredSaveSlot) => {
    setFeedback(null);
    const confirmed = window.confirm(`确认删除本地存档“${slot.title || slot.slotId}”吗？`);
    if (!confirmed) {
      return;
    }

    removeStoredSaveSlot(slot.slotId);
    refreshSlots();
  };

  const handleExport = (slot: StoredSaveSlot) => {
    setFeedback(null);
    const archive = readStoredSaveArchive(slot.slotId);
    if (!archive) {
      setFeedback({
        type: 'error',
        message: `未找到存档“${slot.title || slot.slotId}”的本地数据。`,
      });
      return;
    }

    const sharedArchiveFile: SharedArchiveFile = {
      sessionId: slot.sessionId,
      title: slot.title,
      compressedArchive: archive,
    };
    const blob = new Blob([JSON.stringify(sharedArchiveFile, null, 2)], {
      type: 'application/json;charset=utf-8',
    });
    const url = window.URL.createObjectURL(blob);
    const anchor = document.createElement('a');
    anchor.href = url;
    anchor.download = buildArchiveFileName(slot);
    anchor.click();
    window.URL.revokeObjectURL(url);
    setFeedback({
      type: 'success',
      message: `已导出“${slot.title || slot.slotId}”存档。`,
    });
  };

  const handleImportButtonClick = () => {
    setFeedback(null);
    importInputRef.current?.click();
  };

  const handleImportFile = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    event.target.value = '';
    if (!file) {
      return;
    }

    try {
      const rawText = await file.text();
      const parsed = JSON.parse(rawText) as unknown;
      if (!isSharedArchiveFile(parsed)) {
        throw new Error('该文件不是可用的存档文件。');
      }

      const slotId = createSlotId();
      const archivedAt = new Date().toISOString();
      writeStoredSaveArchive(slotId, parsed.compressedArchive);
      upsertStoredSaveSlot({
        slotId,
        sessionId: parsed.sessionId,
        title: parsed.title || file.name.replace(/\.json$/i, ''),
        createdAt: archivedAt,
        updatedAt: archivedAt,
      });
      refreshSlots();
      setFeedback({
        type: 'success',
        message: `已导入存档“${file.name}”。`,
      });
    } catch (importError) {
      setFeedback({
        type: 'error',
        message: importError instanceof Error ? importError.message : '导入存档失败。',
      });
    }
  };

  return (
    <ScreenShell>
      <StoryFrame className="overflow-hidden p-6 md:p-8">
        <div
          className="absolute inset-0 bg-cover bg-center bg-no-repeat opacity-20"
        />
        <div className="relative z-10 space-y-6">
          <input
            ref={importInputRef}
            type="file"
            accept="application/json,.json"
            className="hidden"
            onChange={handleImportFile}
          />
          <div className="flex flex-wrap items-center justify-between gap-3">
            <SecondaryButton onClick={() => navigate(appRoutes.lobby)} disabled={isLoading}>
              <ArrowLeft className="h-4 w-4" />
              返回大厅
            </SecondaryButton>
            <PrimaryButton type="button" onClick={handleImportButtonClick} disabled={isLoading}>
              <Upload className="h-4 w-4" />
              导入存档
            </PrimaryButton>
          </div>

          <PageTitle
            title="存档列表"
            subtitle="这里保存着你在当前设备上的旅程记录，可随时导入、导出或继续。"
          />

          {error ? (
            <StatusPill
              icon={TriangleAlert}
              className="border-[#7f3b3b]/50 bg-[#2a1216]/85 text-[#ffd7d7]"
              iconClassName="text-[#ff9b9b]"
            >
              {error}
            </StatusPill>
          ) : null}

          {feedback ? (
            <StatusPill
              icon={feedback.type === 'error' ? TriangleAlert : null}
              className={
                feedback.type === 'error'
                  ? 'border-[#7f3b3b]/50 bg-[#2a1216]/85 text-[#ffd7d7]'
                  : 'border-[#36593c]/50 bg-[#15251a]/85 text-[#d9ffe0]'
              }
              iconClassName={feedback.type === 'error' ? 'text-[#ff9b9b]' : undefined}
            >
              {feedback.message}
            </StatusPill>
          ) : null}

          {!hasSlots ? (
            <SectionCard className="space-y-3">
              <p className="text-base text-[#e9edf7]">当前设备上还没有留下任何存档。</p>
              <p className="text-sm leading-7 text-[#98a3ba]">
                先开启一段旅程并保存，或直接导入一份已有存档。
              </p>
            </SectionCard>
          ) : (
            <div className="space-y-4">
              {slots.map((slot) => (
                <SectionCard key={slot.slotId} className="space-y-4">
                  <div className="space-y-2">
                    <div className="flex flex-wrap items-start justify-between gap-3">
                      <div className="space-y-1">
                        <h3 className="text-lg text-[#f2eadf]">{slot.title || '未命名存档'}</h3>
                        <p className="text-xs tracking-[0.12em] text-[#8f98ab]">
                          存档编号：{slot.slotId}
                        </p>
                      </div>
                      <StatusPill icon={null}>最近更新 {formatTimeLabel(slot.updatedAt)}</StatusPill>
                    </div>
                    <p className="text-sm text-[#b6c0d6]">旅程编号：{slot.sessionId}</p>
                    <p className="text-sm text-[#8f98ab]">
                      创建于 {formatTimeLabel(slot.createdAt)}
                    </p>
                  </div>
                  <div className="flex flex-col gap-3 sm:flex-row">
                    <PrimaryButton
                      onClick={() => handleLoad(slot.slotId)}
                      disabled={isLoading}
                      className="flex-1"
                    >
                      <FolderOpen className="h-4 w-4" />
                      {isLoading ? '读取中...' : '继续旅程'}
                    </PrimaryButton>
                    <SecondaryButton
                      type="button"
                      onClick={() => handleExport(slot)}
                      disabled={isLoading}
                      className="flex-1"
                    >
                      <Download className="h-4 w-4" />
                      导出存档
                    </SecondaryButton>
                    <SecondaryButton
                      type="button"
                      onClick={() => handleDelete(slot)}
                      disabled={isLoading}
                      className="flex-1 text-[#ffb6b6] hover:border-[#7f3b3b]/60 hover:bg-[#2a1216]/85 hover:text-[#ffd7d7]"
                    >
                      <Trash2 className="h-4 w-4" />
                      删除存档
                    </SecondaryButton>
                  </div>
                </SectionCard>
              ))}
            </div>
          )}
        </div>
      </StoryFrame>
    </ScreenShell>
  );
};

export default ArchiveListPage;
