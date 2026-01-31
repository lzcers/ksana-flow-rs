import React, { useState, useRef, useEffect } from 'react';
import { Plus, Save, X, Trash2, FileText, LayoutGrid, Loader2, Upload, Download } from 'lucide-react';
import { cn } from '../../utils/cn';
import type { WorkflowStatus } from '../../hooks/useWorkflow';

interface Tab {
  id: number | null;
  name: string;
}

interface WorkflowHeaderProps {
  workflows: { id: number; name: string }[];
  currentWorkflowId: number | null;
  workflowStatuses?: Record<number, WorkflowStatus>;
  onLoadWorkflow: (id: number) => void;
  onSaveWorkflow: (name?: string) => void;
  onDeleteWorkflow: (id: number) => void;
  onRenameWorkflow: (id: number, newName: string) => void;
  onCreateNew: () => void;
  onExportWorkflow: () => void;
  onImportWorkflow: (file: File) => void;
  tabs: Tab[];
  onCloseTab: (id: number | null) => void;
  // Note: Grouping is now done via SelectionToolbar in Canvas
}

export const WorkflowHeader: React.FC<WorkflowHeaderProps> = ({
  workflows,
  currentWorkflowId,
  workflowStatuses = {},
  onLoadWorkflow,
  onSaveWorkflow,
  onDeleteWorkflow,
  onRenameWorkflow,
  onCreateNew,
  onExportWorkflow,
  onImportWorkflow,
  tabs,
  onCloseTab,
}) => {
  const [isDropdownOpen, setIsDropdownOpen] = useState(false);
  const [isEditing, setIsEditing] = useState(false);
  const [editingName, setEditingName] = useState('');
  const [showSaveDialog, setShowSaveDialog] = useState(false);
  const [newWorkflowName, setNewWorkflowName] = useState('');
  const dropdownRef = useRef<HTMLDivElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const currentTab = tabs.find(t => t.id === currentWorkflowId);
  const displayName = currentTab?.name || 'New Workflow';

  useEffect(() => {
    setIsEditing(false);
  }, [currentWorkflowId]);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setIsDropdownOpen(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const handleSave = () => {
    if (currentWorkflowId) {
      onSaveWorkflow();
    } else {
      setShowSaveDialog(true);
    }
  };

  const confirmSave = () => {
    if (newWorkflowName.trim()) {
      onSaveWorkflow(newWorkflowName);
      setShowSaveDialog(false);
      setNewWorkflowName('');
    }
  };

  const startEditing = () => {
    if (!currentWorkflowId) return; // Don't allow renaming unsaved workflow in-place yet
    setEditingName(displayName);
    setIsEditing(true);
  };

  const saveEditing = () => {
    const trimmedName = editingName.trim();
    if (trimmedName && currentWorkflowId) {
      onRenameWorkflow(currentWorkflowId, trimmedName);
      setIsEditing(false);
    } else {
      cancelEditing();
    }
  };

  const cancelEditing = () => {
    setIsEditing(false);
    setEditingName('');
  };

  const handleImportClick = () => {
    fileInputRef.current?.click();
  };

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      onImportWorkflow(file);
    }
    // Reset value so same file can be selected again
    if (fileInputRef.current) fileInputRef.current.value = '';
    setIsDropdownOpen(false);
  };

  return (
    <div className="flex items-center gap-2 h-full w-full">
      {/* Workflow Selector (Menu) */}
      <div className="flex-none flex items-center" ref={dropdownRef}>
        <div className="relative">
          <button
            onClick={() => setIsDropdownOpen(!isDropdownOpen)}
            className={cn(
              "flex items-center justify-center w-7 h-7 rounded-lg transition-all duration-200",
              isDropdownOpen
                ? "bg-blue-500 text-white shadow-[0_0_15px_rgba(59,130,246,0.5)]"
                : "text-zinc-400 hover:text-zinc-100 hover:bg-white/5 hover:scale-105 active:scale-95"
            )}
            title="All Workflows"
          >
            <LayoutGrid size={16} />
          </button>

          {isDropdownOpen && (
            <div className="absolute top-full left-0 mt-2 w-56 bg-zinc-900/90 backdrop-blur-xl border border-white/10 rounded-xl shadow-2xl overflow-hidden py-1 max-h-[320px] overflow-y-auto z-50 animate-in fade-in slide-in-from-top-1 duration-200">
              <button
                onClick={() => {
                  onCreateNew();
                  setIsDropdownOpen(false);
                }}
                className="w-full flex items-center gap-2.5 px-3 py-2 text-[11px] font-medium text-zinc-300 hover:bg-white/10 hover:text-white transition-colors border-b border-white/5"
              >
                <div className="w-5 h-5 rounded bg-blue-500/20 flex items-center justify-center text-blue-400">
                  <Plus size={12} />
                </div>
                New Workflow
              </button>

              <button
                onClick={handleImportClick}
                className="w-full flex items-center gap-2.5 px-3 py-2 text-[11px] font-medium text-zinc-300 hover:bg-white/10 hover:text-white transition-colors border-b border-white/5"
              >
                <div className="w-5 h-5 rounded bg-purple-500/20 flex items-center justify-center text-purple-400">
                  <Upload size={12} />
                </div>
                Import Workflow
              </button>
              <input
                type="file"
                ref={fileInputRef}
                onChange={handleFileChange}
                className="hidden"
                accept=".json"
              />

              <button
                onClick={() => {
                  onExportWorkflow();
                  setIsDropdownOpen(false);
                }}
                className="w-full flex items-center gap-2.5 px-3 py-2 text-[11px] font-medium text-zinc-300 hover:bg-white/10 hover:text-white transition-colors border-b border-white/5"
              >
                <div className="w-5 h-5 rounded bg-green-500/20 flex items-center justify-center text-green-400">
                  <Download size={12} />
                </div>
                Export Workflow
              </button>

              <div className="px-1.5 py-1.5">
                <div className="px-1.5 py-1 text-[9px] font-bold text-zinc-500 uppercase tracking-wider">Recent Workflows</div>
                {workflows.map(wf => (
                  <div
                    key={wf.id}
                    className="group flex items-center justify-between px-2 py-1.5 rounded-lg hover:bg-white/5 cursor-pointer transition-colors"
                    onClick={() => {
                      onLoadWorkflow(wf.id);
                      setIsDropdownOpen(false);
                    }}
                  >
                    <span className={cn("text-[11px] truncate pl-0.5", currentWorkflowId === wf.id ? "text-blue-400 font-medium" : "text-zinc-400 group-hover:text-zinc-200")}>
                      {wf.name}
                    </span>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        if (confirm('Are you sure you want to delete this workflow?')) {
                          onDeleteWorkflow(wf.id);
                        }
                      }}
                      className="opacity-0 group-hover:opacity-100 p-1 rounded hover:bg-red-500/20 text-zinc-500 hover:text-red-400 transition-all"
                    >
                      <Trash2 size={10} />
                    </button>
                  </div>
                ))}
                {workflows.length === 0 && (
                  <div className="px-2 py-1.5 text-[10px] text-zinc-600 italic text-center">No workflows found</div>
                )}
              </div>
            </div>
          )}
        </div>
      </div>

      <div className="w-px h-6 bg-white/10 mx-2"></div>

      {/* Tabs List */}
      <div className="flex-1 flex items-center h-full overflow-x-auto no-scrollbar gap-1 px-1">
        {tabs.map(tab => {
          const isActive = currentWorkflowId === tab.id;
          const status = tab.id ? (workflowStatuses[tab.id] || 'idle') : 'idle';
          const isRunning = status === 'running';

          return (
            <div
              key={tab.id ?? 'new'}
              className={cn(
                "group relative flex items-center gap-2 px-2.5 h-7 rounded-lg transition-all cursor-pointer min-w-[120px] max-w-[180px] select-none border",
                isActive
                  ? "bg-zinc-800/80 border-zinc-700 text-zinc-200 shadow-sm"
                  : "bg-transparent border-transparent text-zinc-500 hover:bg-zinc-800/50 hover:text-zinc-300"
              )}
              onClick={() => {
                if (!isActive) {
                  if (tab.id === null) onCreateNew();
                  else onLoadWorkflow(tab.id);
                }
              }}
              onDoubleClick={() => isActive && startEditing()}
            >
              {isRunning ? (
                <div className="relative">
                  <Loader2 size={12} className="shrink-0 text-zinc-400 animate-spin" />
                  <div className="absolute inset-0 bg-zinc-500/20 blur-sm rounded-full animate-pulse"></div>
                </div>
              ) : (
                <FileText size={12} className={cn("shrink-0 transition-colors", isActive ? "text-zinc-400" : "opacity-50 group-hover:opacity-70")} />
              )}

              {isEditing && isActive ? (
                <input
                  type="text"
                  value={editingName}
                  onChange={(e) => setEditingName(e.target.value)}
                  className="w-full h-full px-1 text-[11px] bg-transparent border-b border-zinc-600 focus:outline-none text-zinc-200 placeholder-zinc-600"
                  autoFocus
                  onBlur={saveEditing}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') saveEditing();
                    if (e.key === 'Escape') cancelEditing();
                  }}
                  onClick={(e) => e.stopPropagation()}
                />
              ) : (
                <span className="truncate text-[11px] font-medium flex-1" title={tab.name}>
                  {tab.name}
                </span>
              )}

              <button
                onClick={(e) => {
                  e.stopPropagation();
                  onCloseTab(tab.id);
                }}
                className={cn(
                  "p-0.5 rounded-full hover:bg-zinc-700/50 text-zinc-500 hover:text-zinc-200 transition-all",
                  isActive ? "opacity-100" : "opacity-0 group-hover:opacity-100"
                )}
              >
                <X size={10} />
              </button>
            </div>
          );
        })}
      </div>

      {/* Global Actions */}
      <div className="flex-none flex items-center px-2 border-l border-zinc-800 ml-1">
        <button
          onClick={handleSave}
          className="group flex items-center justify-center w-7 h-7 rounded-lg text-zinc-500 hover:text-zinc-200 hover:bg-zinc-800 transition-all duration-300"
          title="Save (Ctrl+S)"
        >
          <Save size={16} className="group-hover:scale-105 transition-transform" />
        </button>
      </div>

      {showSaveDialog && (
        <>
          <div className="fixed inset-0 bg-black/20 backdrop-blur-[1px] z-40" onClick={() => setShowSaveDialog(false)}></div>
          <div className="absolute top-16 right-4 p-4 bg-zinc-900/95 backdrop-blur-xl rounded-2xl border border-white/10 shadow-2xl w-72 z-50 animate-in fade-in zoom-in-95 duration-200">
            <h3 className="text-xs font-bold text-zinc-400 uppercase mb-3 tracking-wider">Save New Workflow</h3>
            <input
              type="text"
              value={newWorkflowName}
              onChange={(e) => setNewWorkflowName(e.target.value)}
              placeholder="Enter workflow name..."
              className="w-full px-3 py-2 text-sm bg-black/40 border border-white/10 text-zinc-100 rounded-xl mb-4 focus:outline-none focus:border-blue-500/50 focus:ring-2 focus:ring-blue-500/20 transition-all"
              autoFocus
              onKeyDown={(e) => {
                if (e.key === 'Enter') confirmSave();
                if (e.key === 'Escape') setShowSaveDialog(false);
              }}
            />
            <div className="flex gap-2 justify-end">
              <button
                onClick={() => setShowSaveDialog(false)}
                className="text-xs px-3 py-1.5 rounded-lg text-zinc-400 hover:text-zinc-200 hover:bg-white/5 transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={confirmSave}
                className="text-xs px-3 py-1.5 rounded-lg bg-blue-500 text-white font-medium hover:bg-blue-600 shadow-lg shadow-blue-500/20 transition-all"
              >
                Save Workflow
              </button>
            </div>
          </div>
        </>
      )}
    </div>
  );
};
