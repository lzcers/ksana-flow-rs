import React, { useState } from 'react';
import { Play, Save, Activity, Box, Database, FileText, Plus, FolderOpen, Save as SaveIcon, Trash2, Edit2, X, Check, Loader2 } from 'lucide-react';
import { cn } from '../../utils/cn';
import type { NodeMetadata } from '../../api';

interface SidebarProps {
  nodeTypes: NodeMetadata[];
  workflows: { id: number; name: string }[];
  currentWorkflowId: number | null;
  isRunning: boolean;
  onAddNode: (type: string) => void;
  onRun: () => void;
  onLoadWorkflow: (id: number) => void;
  onSaveWorkflow: (name?: string) => void;
  onDeleteWorkflow: (id: number) => void;
  onRenameWorkflow: (id: number, newName: string) => void;
  onCreateNew: () => void;
}

const getIconForCategory = (category: string) => {
  switch (category.toLowerCase()) {
    case 'source': return Database;
    case 'strategy': return Activity;
    case 'sink': return Box;
    default: return Box;
  }
};

const getColorForCategory = (category: string) => {
  switch (category.toLowerCase()) {
    case 'source': return 'bg-blue-100 text-blue-600';
    case 'strategy': return 'bg-purple-100 text-purple-600';
    case 'sink': return 'bg-orange-100 text-orange-600';
    default: return 'bg-slate-100 text-slate-600';
  }
};

export const Sidebar: React.FC<SidebarProps> = ({
  nodeTypes,
  workflows,
  currentWorkflowId,
  isRunning,
  onAddNode,
  onRun,
  onLoadWorkflow,
  onSaveWorkflow,
  onDeleteWorkflow,
  onRenameWorkflow,
  onCreateNew
}) => {
  const [showSaveDialog, setShowSaveDialog] = useState(false);
  const [newWorkflowName, setNewWorkflowName] = useState('');
  const [editingId, setEditingId] = useState<number | null>(null);
  const [editingName, setEditingName] = useState('');

  const handleSave = () => {
    if (currentWorkflowId) {
      onSaveWorkflow();
    } else {
      setShowSaveDialog(true);
    }
  };

  const confirmSave = () => {
    onSaveWorkflow(newWorkflowName);
    setShowSaveDialog(false);
    setNewWorkflowName('');
  };

  const startEditing = (id: number, name: string, e: React.MouseEvent) => {
    e.stopPropagation();
    setEditingId(id);
    setEditingName(name);
  };

  const cancelEditing = (e?: React.MouseEvent) => {
    e?.stopPropagation();
    setEditingId(null);
    setEditingName('');
  };

  const saveEditing = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (editingId && editingName.trim()) {
      onRenameWorkflow(editingId, editingName.trim());
      setEditingId(null);
      setEditingName('');
    }
  };

  return (
    <aside className="w-64 border-r border-slate-100 bg-white p-6 z-10 flex flex-col h-full overflow-hidden">
      <div className="mb-6 flex-shrink-0">
        <h1 className="text-lg font-bold tracking-tight text-slate-900 flex items-center gap-2 mb-6">
          <div className="w-6 h-6 bg-slate-900 rounded flex items-center justify-center text-white">
            <Play size={12} fill="currentColor" />
          </div>
          Ksana Flow
        </h1>

        <div className="space-y-2">
          <button
            onClick={onCreateNew}
            className="w-full flex items-center gap-2 px-3 py-2 text-sm font-medium text-slate-600 hover:bg-slate-50 rounded-md transition-colors border border-slate-200"
          >
            <Plus size={16} />
            New Workflow
          </button>
          <div className="relative group">
            <button
              onClick={handleSave}
              className="w-full flex items-center gap-2 px-3 py-2 text-sm font-medium text-slate-600 hover:bg-slate-50 rounded-md transition-colors border border-slate-200"
            >
              <SaveIcon size={16} />
              {currentWorkflowId ? 'Save Workflow' : 'Save As...'}
            </button>
          </div>
        </div>

        {showSaveDialog && (
          <div className="mt-2 p-3 bg-slate-50 rounded-lg border border-slate-200">
            <input
              type="text"
              value={newWorkflowName}
              onChange={(e) => setNewWorkflowName(e.target.value)}
              placeholder="Workflow Name"
              className="w-full px-2 py-1 text-sm border border-slate-300 rounded mb-2"
              autoFocus
            />
            <div className="flex gap-2 justify-end">
              <button onClick={() => setShowSaveDialog(false)} className="text-xs text-slate-500 hover:text-slate-700">Cancel</button>
              <button onClick={confirmSave} className="text-xs bg-slate-900 text-white px-2 py-1 rounded hover:bg-slate-800">Save</button>
            </div>
          </div>
        )}
      </div>

      <div className="flex-1 overflow-y-auto min-h-0 space-y-6 pr-2">
        <div>
          <h2 className="text-[11px] font-bold text-slate-400 uppercase tracking-[0.1em] mb-2 sticky top-0 bg-white py-1">Workflows</h2>
          <div className="space-y-1">
            {workflows.map(wf => (
              <div
                key={wf.id}
                className={cn(
                  "group relative w-full flex items-center gap-2 px-3 py-2 text-sm rounded-md transition-colors cursor-pointer",
                  currentWorkflowId === wf.id
                    ? "bg-blue-50 text-blue-700 font-medium"
                    : "text-slate-600 hover:bg-slate-50"
                )}
                onClick={() => {
                  if (editingId !== wf.id) onLoadWorkflow(wf.id);
                }}
              >
                <FileText size={14} className={editingId === wf.id ? "text-slate-400" : ""} />

                {editingId === wf.id ? (
                  <div className="flex-1 flex items-center gap-1 min-w-0" onClick={e => e.stopPropagation()}>
                    <input
                      type="text"
                      value={editingName}
                      onChange={e => setEditingName(e.target.value)}
                      className="flex-1 min-w-0 px-1 py-0.5 text-xs border border-blue-300 rounded focus:outline-none focus:border-blue-500 bg-white text-slate-900"
                      autoFocus
                      onKeyDown={e => {
                        if (e.key === 'Enter') saveEditing(e as any);
                        if (e.key === 'Escape') cancelEditing(e as any);
                      }}
                    />
                    <button onClick={saveEditing} className="p-0.5 text-green-600 hover:bg-green-50 rounded"><Check size={12} /></button>
                    <button onClick={cancelEditing} className="p-0.5 text-red-500 hover:bg-red-50 rounded"><X size={12} /></button>
                  </div>
                ) : (
                  <>
                    <span className="truncate flex-1" onDoubleClick={(e) => startEditing(wf.id, wf.name, e)}>
                      {wf.name}
                    </span>
                    <div className="flex items-center opacity-0 group-hover:opacity-100 transition-opacity gap-1">
                      <button
                        onClick={(e) => startEditing(wf.id, wf.name, e)}
                        className="p-1 hover:bg-slate-200 text-slate-500 rounded transition-all"
                        title="Rename Workflow"
                      >
                        <Edit2 size={12} />
                      </button>
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          if (confirm('Are you sure you want to delete this workflow?')) {
                            onDeleteWorkflow(wf.id);
                          }
                        }}
                        className="p-1 hover:bg-red-100 hover:text-red-600 rounded transition-all"
                        title="Delete Workflow"
                      >
                        <Trash2 size={12} />
                      </button>
                    </div>
                  </>
                )}
              </div>
            ))}
            {workflows.length === 0 && (
              <div className="text-xs text-slate-400 italic px-3 py-2">No saved workflows</div>
            )}
          </div>
        </div>

        <div>
          <h2 className="text-[11px] font-bold text-slate-400 uppercase tracking-[0.1em] mb-2 sticky top-0 bg-white py-1">Components</h2>
          <div className="space-y-1.5">
            {nodeTypes.map(nodeType => {
              const Icon = getIconForCategory(nodeType.category);
              const colorClass = getColorForCategory(nodeType.category);
              return (
                <button
                  key={nodeType.name}
                  onClick={() => onAddNode(nodeType.name)}
                  draggable
                  onDragStart={(e) => {
                    e.dataTransfer.setData('application/reactflow', nodeType.name);
                    e.dataTransfer.effectAllowed = 'move';
                  }}
                  className="w-full flex items-center gap-3 p-2 rounded-lg hover:bg-slate-50 transition-colors text-left group cursor-grab active:cursor-grabbing"
                >
                  <div className={cn("p-1.5 rounded-md transition-colors", colorClass)}>
                    <Icon size={16} />
                  </div>
                  <div className="flex flex-col">
                    <span className="text-sm font-medium text-slate-600 group-hover:text-slate-900">
                      {nodeType.name}
                    </span>
                    <span className="text-[10px] text-slate-400">{nodeType.category}</span>
                  </div>
                </button>
              );
            })}
          </div>
        </div>
      </div>

      <div className="mt-4 pt-4 border-t border-slate-100 flex-shrink-0">
        <button
          onClick={onRun}
          disabled={isRunning}
          className={cn(
            "w-full flex items-center justify-center gap-2 bg-slate-900 text-white py-2.5 rounded-lg text-sm font-medium transition-all active:scale-[0.98]",
            isRunning ? "opacity-70 cursor-not-allowed" : "hover:bg-slate-800"
          )}
        >
          {isRunning ? <Loader2 size={16} className="animate-spin" /> : <Play size={16} />}
          {isRunning ? 'Running...' : 'Run Workflow'}
        </button>
      </div>
    </aside>
  );
};
