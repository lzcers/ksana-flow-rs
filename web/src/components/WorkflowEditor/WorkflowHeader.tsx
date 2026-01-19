import React, { useState, useRef, useEffect } from 'react';
import { ChevronDown, Plus, Save, Edit2, Check, X, Trash2, FileText, LayoutGrid, Loader2 } from 'lucide-react';
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
  tabs: Tab[];
  onCloseTab: (id: number | null) => void;
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
  tabs,
  onCloseTab
}) => {
  const [isDropdownOpen, setIsDropdownOpen] = useState(false);
  const [isEditing, setIsEditing] = useState(false);
  const [editingName, setEditingName] = useState('');
  const [showSaveDialog, setShowSaveDialog] = useState(false);
  const [newWorkflowName, setNewWorkflowName] = useState('');
  const dropdownRef = useRef<HTMLDivElement>(null);

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

  return (
    <div className="flex items-center gap-2 h-full w-full">
      {/* Workflow Selector (Menu) */}
      <div className="flex-none flex items-center" ref={dropdownRef}>
        <div className="relative">
          <button
            onClick={() => setIsDropdownOpen(!isDropdownOpen)}
            className={cn(
              "flex items-center justify-center w-8 h-8 rounded hover:bg-zinc-800 text-zinc-400 hover:text-zinc-100 transition-colors",
              isDropdownOpen && "bg-zinc-800 text-zinc-100"
            )}
            title="All Workflows"
          >
            <LayoutGrid size={16} />
          </button>

          {isDropdownOpen && (
            <div className="absolute top-full left-0 mt-1 w-64 bg-zinc-900 border border-zinc-800 rounded-lg shadow-xl overflow-hidden py-1 max-h-[400px] overflow-y-auto z-50">
              <button
                onClick={() => {
                  onCreateNew();
                  setIsDropdownOpen(false);
                }}
                className="w-full flex items-center gap-2 px-3 py-2 text-xs text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100 transition-colors border-b border-zinc-800/50"
              >
                <Plus size={12} />
                New Workflow
              </button>
              {workflows.map(wf => (
                <div
                  key={wf.id}
                  className="group flex items-center justify-between px-3 py-2 hover:bg-zinc-800 cursor-pointer"
                  onClick={() => {
                    onLoadWorkflow(wf.id);
                    setIsDropdownOpen(false);
                  }}
                >
                  <span className={cn("text-xs truncate", currentWorkflowId === wf.id ? "text-blue-400" : "text-zinc-300")}>
                    {wf.name}
                  </span>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      if (confirm('Are you sure you want to delete this workflow?')) {
                        onDeleteWorkflow(wf.id);
                      }
                    }}
                    className="opacity-0 group-hover:opacity-100 p-1 text-zinc-500 hover:text-red-400 transition-opacity"
                  >
                    <Trash2 size={12} />
                  </button>
                </div>
              ))}
              {workflows.length === 0 && (
                <div className="px-3 py-2 text-xs text-zinc-500 italic">No workflows found</div>
              )}
            </div>
          )}
        </div>
      </div>

      <div className="w-px h-4 bg-zinc-800 mx-1"></div>

      {/* Tabs List */}
      <div className="flex-1 flex items-end h-full overflow-x-auto no-scrollbar gap-1 px-1">
        {tabs.map(tab => {
          const isActive = currentWorkflowId === tab.id;
          // Use global status map if available, fallback to current status if active, otherwise idle
          const status = tab.id ? (workflowStatuses[tab.id] || 'idle') : 'idle';
          const isRunning = status === 'running';

          return (
            <div
              key={tab.id ?? 'new'}
              className={cn(
                "group relative flex items-center gap-2 px-3 h-8 rounded-t-md border-t border-x border-transparent transition-all cursor-pointer min-w-[120px] max-w-[200px] select-none",
                isActive
                  ? "bg-zinc-800 border-zinc-700/50 text-zinc-100"
                  : "text-zinc-500 hover:bg-zinc-800/30 hover:text-zinc-300"
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
                <Loader2 size={12} className="shrink-0 text-blue-400 animate-spin" />
              ) : (
                <FileText size={12} className={cn("shrink-0", isActive ? "text-blue-400" : "opacity-50")} />
              )}

              {isEditing && isActive ? (
                <input
                  type="text"
                  value={editingName}
                  onChange={(e) => setEditingName(e.target.value)}
                  className="w-full h-5 px-1 text-xs bg-zinc-950 border border-blue-500/50 rounded focus:outline-none text-zinc-100"
                  autoFocus
                  onBlur={saveEditing}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') saveEditing();
                    if (e.key === 'Escape') cancelEditing();
                  }}
                  onClick={(e) => e.stopPropagation()}
                />
              ) : (
                <span className="truncate text-xs font-medium flex-1" title={tab.name}>
                  {tab.name}
                </span>
              )}

              <button
                onClick={(e) => {
                  e.stopPropagation();
                  onCloseTab(tab.id);
                }}
                className={cn(
                  "p-0.5 rounded hover:bg-zinc-700 text-zinc-500 hover:text-zinc-200 transition-opacity",
                  isActive ? "opacity-100" : "opacity-0 group-hover:opacity-100"
                )}
              >
                <X size={12} />
              </button>

              {/* Active Indicator Line */}
              {isActive && (
                <div className={cn("absolute bottom-0 left-0 right-0 h-0.5", isRunning ? "bg-green-500 animate-pulse" : "bg-blue-500")}></div>
              )}
            </div>
          );
        })}
      </div>

      {/* Global Actions */}
      <div className="flex-none flex items-center px-2 border-l border-zinc-800 ml-1">
        <button
          onClick={handleSave}
          className="p-1.5 text-zinc-400 hover:text-zinc-100 hover:bg-zinc-800 rounded transition-colors"
          title="Save (Ctrl+S)"
        >
          <Save size={14} />
        </button>
      </div>

      {showSaveDialog && (
        <div className="absolute top-10 right-4 mt-1 p-3 bg-zinc-800 rounded-lg border border-zinc-700 shadow-xl w-64 z-50">
          <h3 className="text-xs font-bold text-zinc-400 uppercase mb-2">Save New Workflow</h3>
          <input
            type="text"
            value={newWorkflowName}
            onChange={(e) => setNewWorkflowName(e.target.value)}
            placeholder="Workflow Name"
            className="w-full px-2 py-1 text-sm bg-zinc-900 border border-zinc-700 text-zinc-100 rounded mb-2 focus:outline-none focus:border-blue-500"
            autoFocus
            onKeyDown={(e) => {
              if (e.key === 'Enter') confirmSave();
              if (e.key === 'Escape') setShowSaveDialog(false);
            }}
          />
          <div className="flex gap-2 justify-end">
            <button onClick={() => setShowSaveDialog(false)} className="text-xs text-zinc-500 hover:text-zinc-300">Cancel</button>
            <button onClick={confirmSave} className="text-xs bg-blue-600 text-white px-2 py-1 rounded hover:bg-blue-500">Save</button>
          </div>
        </div>
      )}
    </div>
  );
};
